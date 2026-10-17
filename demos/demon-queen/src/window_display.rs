//! One native surface for the entire game. Both modes author at 1920×1080;
//! only this final pass scales/letterboxes, including fades and the 3D map.

use anyhow::{Context, Result};
use std::sync::Arc;
use velvet_render::SoftwareViewport;
use winit::window::Window;

/// A canvas's pixels (0xAARRGGBB) and its size in them.
pub type CanvasPixels<'a> = (&'a [u32], (u32, u32));

pub const WIDTH: u32 = 1920;
pub const HEIGHT: u32 = 1080;
// The world menu sits inside the minimap header. Keeping it in the shared
// compositor means the texture, click target and ESC shortcut stay identical
// to the VN menu while the 3D scene gets one coherent control cluster.
pub const MENU_RECT: (i32, i32, i32, i32) = (1358, 28, 204, 58);

#[derive(Clone)]
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pub msaa_samples: u32,
}

pub struct WindowDisplay {
    surface: wgpu::Surface<'static>,
    pub gpu: GpuContext,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    canvas: wgpu::Texture,
    canvas_view: wgpu::TextureView,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    /// A picture laid over the scene with its own transparency (premultiplied): Chiquita alive in
    /// the visual novel.
    over_pipeline: wgpu::RenderPipeline,
    /// How much of that picture shows (its own opacity times the scene's).
    over_fade: wgpu::Buffer,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    fade: wgpu::Buffer,
    menu_view: Option<wgpu::TextureView>,
    pub presented_frames: u64,
}

const SHADER: &str = r#"
struct Fade { opacity: vec4<f32> };
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var filtering: sampler;
@group(0) @binding(2) var<uniform> fade: Fade;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var v: Vertex;
    v.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    v.uv = uv;
    return v;
}
@fragment fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(source, filtering, v.uv).rgb * fade.opacity.x, 1.0);
}
@fragment fn fs_over(v: Vertex) -> @location(0) vec4<f32> {
    return textureSample(source, filtering, v.uv) * fade.opacity.x;
}
"#;

impl WindowDisplay {
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let mut instance_descriptor = wgpu::InstanceDescriptor::default();
        // Vulkan on the development RTX 5080 loses the device during animated
        // movement. DX12 completes the same movement workload on Windows.
        // Keep WGPU_BACKEND available for explicit diagnostics/other drivers.
        if cfg!(target_os = "windows") {
            instance_descriptor.backends = wgpu::Backends::DX12;
        }
        let instance = wgpu::Instance::new(&instance_descriptor.with_env());
        let surface = instance
            .create_surface(window.clone())
            .context("game window surface")?;
        // QA: `VELVET_ADAPTER=intel` plays on the graphics card whose name holds that (to measure
        // a weaker one on a machine with two).
        let wanted = std::env::var("VELVET_ADAPTER")
            .ok()
            .map(|name| name.to_lowercase());
        let chosen = wanted.and_then(|name| {
            instance
                .enumerate_adapters(wgpu::Backends::all())
                .into_iter()
                .find(|a| {
                    a.get_info().name.to_lowercase().contains(&name)
                        && a.is_surface_supported(&surface)
                })
        });
        let adapter = match chosen {
            Some(adapter) => adapter,
            None => instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                })
                .await
                .context("game graphics adapter")?,
        };
        eprintln!("[game] graphics adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("demon-queen-unified-device"),
                    // Block-compressed textures (BC7): a quarter of the video memory.
                    required_features: adapter.features() & wgpu::Features::TEXTURE_COMPRESSION_BC,
                    required_limits: wgpu::Limits::default(),
                    // Smaller allocation blocks: the laptop GPU is often shared
                    // with other programs (a local AI model), so be frugal.
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .context("game graphics device")?;
        device.set_device_lost_callback(|reason, message| {
            eprintln!("[game] graphics device lost: {reason:?}: {message}");
        });
        // A GPU error (most often running out of video memory while another
        // program holds it) costs a frame, not the game: log and carry on.
        // wgpu's default handler panics and closes the window.
        let reported = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        device.on_uncaptured_error(Box::new(move |error| {
            let count = reported.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count < 8 || count % 600 == 0 {
                eprintln!("[game] graphics error #{count} (frame skipped): {error}");
            }
        }));
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let features = adapter.get_texture_format_features(format);
        // Dense 3D scenes can submit more than a thousand mesh parts. Keep
        // the compositor conservative on Windows GPUs: 1x avoids a driver
        // reset (`Surface::present: Parent device is lost`) while the 1080p
        // native canvas still gives the UI its authored sharpness.
        let msaa_samples = [1, 2, 4]
            .into_iter()
            .find(|&samples| {
                features.flags.sample_count_supported(samples)
                    && (samples == 1
                        || features
                            .flags
                            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE))
            })
            .unwrap_or(1);
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let canvas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vn-native-1080p"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Canvas u32s are 0xAARRGGBB; little-endian bytes are BGRA.
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let canvas_view = canvas.create_view(&Default::default());
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("game-compositor-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("game-compositor"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("game-compositor"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("game-compositor"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let over_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("game-compositor-over"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_over"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("game-bilinear-presentation"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let nearest = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let fade = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("game-scene-opacity"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let over_fade = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("game-over-opacity"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        eprintln!("[game] one window, one device: {}", adapter.get_info().name);
        Ok(Self {
            surface,
            gpu: GpuContext {
                device,
                queue,
                format,
                msaa_samples,
            },
            config,
            window,
            canvas,
            canvas_view,
            layout,
            pipeline,
            over_pipeline,
            over_fade,
            linear,
            nearest,
            fade,
            menu_view: None,
            presented_frames: 0,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
    }

    /// The window shows nothing (minimised, or sized to nothing): there is no frame to present.
    pub fn is_hidden(&self) -> bool {
        let size = self.window.inner_size();
        size.width == 0 || size.height == 0 || self.window.is_minimized() == Some(true)
    }

    /// The canvas texture at `w`×`h` (made again when the canvas's own pixels change size).
    fn ensure_canvas(&mut self, w: u32, h: u32) {
        let size = self.canvas.size();
        if (size.width, size.height) == (w, h) {
            return;
        }
        self.canvas = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vn-canvas"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Canvas u32s are 0xAARRGGBB; little-endian bytes are BGRA.
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.canvas_view = self.canvas.create_view(&Default::default());
    }

    /// The canvas's pixels into its texture.
    fn upload_canvas(&mut self, canvas: CanvasPixels) -> Result<()> {
        let (pixels, (w, h)) = canvas;
        anyhow::ensure!(
            pixels.len() == (w * h) as usize,
            "canvas pixels do not match its size"
        );
        self.ensure_canvas(w, h);
        self.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.canvas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(pixels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        Ok(())
    }

    pub fn present_canvas(&mut self, canvas: CanvasPixels, opacity: f32) -> Result<bool> {
        // A minimised window presents nothing, so the canvas is not written either: each write
        // takes a staging copy that only a submission frees, and the caller asks again at
        // once (it piled up gigabytes a minute while the game sat minimised on a VN screen).
        if self.is_hidden() {
            return Ok(false);
        }
        self.upload_canvas(canvas)?;
        let view = self.canvas_view.clone();
        // Canvas uploads use a temporary wgpu staging allocation.  Drain it
        // before returning; title/VN frames can be seconds apart and otherwise
        // each upload remains resident until another frame is submitted.
        self.present_final(&view, opacity, false, true, None)
    }

    /// The canvas with a picture laid over it (premultiplied by its own transparency): Chiquita
    /// alive over the visual novel. `canvas`: a new canvas, or `None` to show the last one again
    /// (only she has moved).
    pub fn present_canvas_over(
        &mut self,
        canvas: Option<CanvasPixels>,
        opacity: f32,
        over: Option<(&wgpu::TextureView, f32)>,
    ) -> Result<bool> {
        if self.is_hidden() {
            return Ok(false);
        }
        if let Some(canvas) = canvas {
            self.upload_canvas(canvas)?;
        }
        let view = self.canvas_view.clone();
        self.present_final(&view, opacity, false, canvas.is_some(), over)
    }

    pub fn present_texture(&mut self, source: &wgpu::TextureView, opacity: f32) -> Result<bool> {
        self.present_final(source, opacity, false, false, None)
    }

    pub fn set_menu_button(&mut self, rgba: &[u8], width: u32, height: u32) {
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shared-menu-button"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.gpu.queue.write_texture(
            texture.as_image_copy(),
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            texture.size(),
        );
        self.menu_view = Some(texture.create_view(&Default::default()));
    }

    pub fn present_world(
        &mut self,
        source: &wgpu::TextureView,
        opacity: f32,
        show_menu: bool,
    ) -> Result<bool> {
        self.present_final(source, opacity, show_menu, false, None)
    }

    /// No frame this time: whatever was written for it (the canvas) is still sent to the GPU and
    /// waited for, so its staging copy is freed instead of waiting in the queue.
    fn flush_uploads(&self, drain_uploads: bool) {
        if drain_uploads {
            self.gpu.queue.submit(std::iter::empty());
            let _ = self.gpu.device.poll(wgpu::Maintain::Wait);
        }
    }

    fn present_final(
        &mut self,
        source: &wgpu::TextureView,
        opacity: f32,
        show_menu: bool,
        drain_uploads: bool,
        over: Option<(&wgpu::TextureView, f32)>,
    ) -> Result<bool> {
        let size = self.window.inner_size();
        let Some(viewport) = SoftwareViewport::letterbox(size.width, size.height, WIDTH, HEIGHT)
        else {
            self.flush_uploads(drain_uploads);
            return Ok(false);
        };
        if self.config.width != size.width || self.config.height != size.height {
            self.resize(size.width, size.height);
        }
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.resize(size.width, size.height);
                self.flush_uploads(drain_uploads);
                return Ok(false);
            }
            Err(wgpu::SurfaceError::Timeout) => {
                self.flush_uploads(drain_uploads);
                return Ok(false);
            }
            Err(error) => {
                self.flush_uploads(drain_uploads);
                return Err(error.into());
            }
        };
        self.gpu.queue.write_buffer(
            &self.fade,
            0,
            bytemuck::cast_slice(&[opacity.clamp(0.0, 1.0), 0.0, 0.0, 0.0]),
        );
        let sampler = if viewport.width == WIDTH && viewport.height == HEIGHT {
            &self.nearest
        } else {
            &self.linear
        };
        let bindings = self
            .gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("game-scene"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.fade.as_entire_binding(),
                    },
                ],
            });
        let target = frame.texture.create_view(&Default::default());
        let over_bindings = over.map(|(view, over_opacity)| {
            self.gpu.queue.write_buffer(
                &self.over_fade,
                0,
                bytemuck::cast_slice(&[(opacity * over_opacity).clamp(0.0, 1.0), 0.0, 0.0, 0.0]),
            );
            self.gpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("game-scene-over"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.over_fade.as_entire_binding(),
                        },
                    ],
                })
        });
        let menu_bindings = self.menu_view.as_ref().filter(|_| show_menu).map(|view| {
            self.gpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("menu-trigger"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.fade.as_entire_binding(),
                        },
                    ],
                })
        });
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("game-final-letterbox"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_viewport(
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
                0.0,
                1.0,
            );
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bindings, &[]);
            pass.draw(0..3, 0..1);
            if let Some(bindings) = &over_bindings {
                pass.set_pipeline(&self.over_pipeline);
                pass.set_bind_group(0, bindings, &[]);
                pass.draw(0..3, 0..1);
                pass.set_pipeline(&self.pipeline);
            }
            if let Some(bindings) = &menu_bindings {
                let (x, y, w, h) = MENU_RECT;
                let sx = viewport.width as f32 / WIDTH as f32;
                let sy = viewport.height as f32 / HEIGHT as f32;
                pass.set_viewport(
                    viewport.x as f32 + x as f32 * sx,
                    viewport.y as f32 + y as f32 * sy,
                    w as f32 * sx,
                    h as f32 * sy,
                    0.0,
                    1.0,
                );
                pass.set_pipeline(&self.over_pipeline);
                pass.set_bind_group(0, bindings, &[]);
                pass.draw(0..3, 0..1);
                pass.set_pipeline(&self.pipeline);
            }
        }
        self.gpu.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        frame.present();
        // The 3D path is continuously paced and can use a non-blocking poll.
        // Canvas uploads use an 8 MB temporary staging allocation; wait for
        // those copies to complete before returning so sparse VN/title frames
        // do not retain one allocation per presentation.
        let maintenance = if drain_uploads {
            wgpu::Maintain::Wait
        } else {
            wgpu::Maintain::Poll
        };
        let _ = self.gpu.device.poll(maintenance);
        self.presented_frames += 1;
        frame_log();
        Ok(true)
    }
}

/// QA: `VELVET_FRAMELOG=<file>` writes when each frame was shown (ms since the first, and since
/// the one before), to find the hitches a player feels.
type FrameLog = Option<(std::io::BufWriter<std::fs::File>, std::time::Instant, f64)>;
static FRAME_LOG: std::sync::Mutex<FrameLog> = std::sync::Mutex::new(None);

fn frame_log_open() -> Option<std::sync::MutexGuard<'static, FrameLog>> {
    static PATH: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    let path = PATH
        .get_or_init(|| std::env::var("VELVET_FRAMELOG").ok())
        .as_ref()?;
    let mut log = FRAME_LOG.lock().unwrap();
    if log.is_none() {
        let file = std::fs::File::create(path).ok()?;
        *log = Some((
            std::io::BufWriter::new(file),
            std::time::Instant::now(),
            0.0,
        ));
    }
    Some(log)
}

fn frame_log() {
    use std::io::Write;
    let Some(mut log) = frame_log_open() else {
        return;
    };
    let (out, start, last) = log.as_mut().unwrap();
    let now = start.elapsed().as_secs_f64() * 1000.0;
    let _ = writeln!(out, "{now:.2} {:.2}", now - *last);
    *last = now;
    let _ = out.flush();
}

/// QA: a note in the frame log (what took long between two frames), when it is on.
pub(crate) fn frame_note(text: &str) {
    use std::io::Write;
    let Some(mut log) = frame_log_open() else {
        return;
    };
    let (out, start, _) = log.as_mut().unwrap();
    let now = start.elapsed().as_secs_f64() * 1000.0;
    let _ = writeln!(out, "# {now:.2} {text}");
    let _ = out.flush();
}
