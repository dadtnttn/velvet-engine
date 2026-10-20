//! The window: winit for the window and its events, wgpu to show the canvas letterboxed.

use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use velvet_canvas::{Canvas, Point};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::input::{Button, Input, InputEvent};
use crate::viewport::Viewport;
use crate::{Game2D, HostConfig};

/// Opens the window and runs the game until it quits or the window is closed.
pub fn run<G: Game2D + 'static>(config: HostConfig, game: G) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let canvas = Canvas::new(config.logical_width, config.logical_height);
    // QA: `VELVET_CAPTURE_AFTER=120:out.png` saves the canvas after that many frames and closes the window.
    let capture_after = std::env::var("VELVET_CAPTURE_AFTER").ok().and_then(|v| {
        let (frames, path) = v.split_once(':')?;
        Some((frames.trim().parse().ok()?, std::path::PathBuf::from(path.trim())))
    });
    let mut app = App { config, game, canvas, input: Input::new(), running: None, last: None, behind: 0.0, error: None, frames: 0, capture_after };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e.into()),
        None => Ok(()),
    }
}

struct App<G: Game2D> {
    config: HostConfig,
    game: G,
    canvas: Canvas,
    input: Input,
    running: Option<Running>,
    last: Option<Instant>,
    /// Game time not yet stepped.
    behind: f32,
    error: Option<String>,
    /// Frames shown so far.
    frames: u64,
    /// After how many frames to save the canvas and close (QA).
    capture_after: Option<(u64, std::path::PathBuf)>,
}

struct Running {
    window: Arc<Window>,
    presenter: Presenter,
    viewport: Option<Viewport>,
    /// Where the pointer is, in window pixels.
    cursor: Option<(f64, f64)>,
    /// True while the pointer is on the picture.
    on_picture: bool,
}

impl<G: Game2D> App<G> {
    /// The window has a new size: a new letterbox, and the canvas follows the picture's pixels.
    fn resized(&mut self, width: u32, height: u32) {
        let Some(running) = self.running.as_mut() else { return };
        running.presenter.resize(width, height);
        running.viewport = Viewport::letterbox(width, height, self.config.logical_width, self.config.logical_height);
        if let Some(v) = running.viewport {
            let (pw, ph) = self.config.canvas_pixels(v.width, v.height);
            if pw != self.canvas.pixel_width() || ph != self.canvas.pixel_height() {
                self.canvas.resize_pixels(pw, ph);
                self.game.pixels_changed(&self.canvas);
            }
        }
    }

    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = self.last.map_or(0.0, |last| now.duration_since(last).as_secs_f32());
        self.last = Some(now);
        // A long stall (a dragged window, a breakpoint) does not make the game race to catch up.
        self.behind += dt.min(0.25);
        let mut steps = 0;
        while self.behind >= self.config.step && steps < 8 {
            self.game.update(&self.input, self.config.step);
            self.input.end_step(self.config.step);
            self.behind -= self.config.step;
            steps += 1;
        }
        if self.game.quit() {
            event_loop.exit();
            return;
        }
        let Some(running) = self.running.as_mut() else { return };
        // A minimised window shows nothing: nothing is drawn nor sent to the graphics card.
        let Some(viewport) = running.viewport else { return };
        self.game.draw(&mut self.canvas);
        if let Err(e) = running.presenter.present(&self.canvas, &viewport) {
            eprintln!("[velvet-host2d] frame not shown: {e}");
        }
        self.frames += 1;
        if let Some((after, path)) = &self.capture_after {
            if self.frames >= *after {
                match self.canvas.save_png(path) {
                    Ok(()) => eprintln!("[velvet-host2d] {} after {} frames ({}×{} pixels)", path.display(), self.frames, self.canvas.pixel_width(), self.canvas.pixel_height()),
                    Err(e) => eprintln!("[velvet-host2d] the capture failed: {e}"),
                }
                event_loop.exit();
            }
        }
    }

    fn pointer_to(&mut self, x: f64, y: f64) {
        let Some(running) = self.running.as_mut() else { return };
        running.cursor = Some((x, y));
        match running.viewport.and_then(|v| v.to_logical(x, y)) {
            Some((lx, ly)) => {
                running.on_picture = true;
                self.input.push(InputEvent::PointerMoved(Point::new(lx, ly)));
            }
            None if running.on_picture => {
                running.on_picture = false;
                self.input.push(InputEvent::PointerLeft);
            }
            None => {}
        }
    }

    fn button(&mut self, state: ElementState, button: MouseButton) {
        let button = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
            _ => return,
        };
        let Some(running) = self.running.as_ref() else { return };
        let (Some(viewport), Some((x, y))) = (running.viewport, running.cursor) else { return };
        match state {
            // The bars take no input.
            ElementState::Pressed => {
                if let Some((lx, ly)) = viewport.to_logical(x, y) {
                    self.input.push(InputEvent::PointerDown { button, at: Point::new(lx, ly) });
                }
            }
            // A release always arrives, so a drag that left the picture ends.
            ElementState::Released => {
                if self.input.button_down(button) {
                    let (lx, ly) = viewport.to_logical_clamped(x, y);
                    self.input.push(InputEvent::PointerUp { button, at: Point::new(lx, ly) });
                }
            }
        }
    }

    fn screenshot(&self) {
        let Some(dir) = &self.config.captures_dir else { return };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        let path = dir.join(format!("captura-{stamp}.png"));
        match self.canvas.save_png(&path) {
            Ok(()) => eprintln!("[velvet-host2d] {}", path.display()),
            Err(e) => eprintln!("[velvet-host2d] the screenshot failed: {e}"),
        }
    }
}

impl<G: Game2D> ApplicationHandler for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.running.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(PhysicalSize::new(self.config.window_width, self.config.window_height))
            .with_min_inner_size(PhysicalSize::new(self.config.min_width, self.config.min_height));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.error = Some(format!("the window could not be opened: {e}"));
                event_loop.exit();
                return;
            }
        };
        let presenter = match pollster::block_on(Presenter::new(window.clone())) {
            Ok(p) => p,
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        self.running = Some(Running { window, presenter, viewport: None, cursor: None, on_picture: false });
        self.resized(size.width, size.height);
        self.last = Some(Instant::now());
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.resized(size.width, size.height),
            WindowEvent::CursorMoved { position, .. } => self.pointer_to(position.x, position.y),
            WindowEvent::CursorLeft { .. } => {
                if let Some(running) = self.running.as_mut() {
                    running.cursor = None;
                    if running.on_picture {
                        running.on_picture = false;
                        self.input.push(InputEvent::PointerLeft);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => self.button(state, button),
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 40.0,
                };
                if lines != 0.0 {
                    self.input.push(InputEvent::Wheel(lines));
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            if code == KeyCode::F12 && !event.repeat {
                                self.screenshot();
                            }
                            self.input.push(InputEvent::KeyDown(code));
                        }
                        ElementState::Released => self.input.push(InputEvent::KeyUp(code)),
                    }
                }
                if event.state == ElementState::Pressed {
                    if let Some(text) = event.text.as_ref().filter(|t| t.chars().all(|c| !c.is_control())) {
                        self.input.push(InputEvent::Text(text.to_string()));
                    }
                }
            }
            WindowEvent::RedrawRequested => self.frame(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // A frame at every turn of the loop; the presentation waits for the screen (vsync).
        if let Some(running) = &self.running {
            running.window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

const SHADER: &str = r#"
@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var filtering: sampler;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) index: u32) -> Vertex {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var v: Vertex;
    v.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    v.uv = uv;
    return v;
}
@fragment fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(source, filtering, v.uv).rgb, 1.0);
}
"#;

/// Shows a canvas in the window.
struct Presenter {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    source: Option<Source>,
}

/// The canvas's texture, and its bindings with each sampler.
struct Source {
    texture: wgpu::Texture,
    smooth: wgpu::BindGroup,
    exact: wgpu::BindGroup,
    size: (u32, u32),
}

impl Presenter {
    async fn new(window: Arc<Window>) -> Result<Presenter, String> {
        let mut descriptor = wgpu::InstanceDescriptor::default();
        // As in Demon Queen: DX12 on Windows (Vulkan lost the device on the development machine).
        if cfg!(target_os = "windows") {
            descriptor.backends = wgpu::Backends::DX12;
        }
        let instance = wgpu::Instance::new(&descriptor.with_env());
        let surface = instance.create_surface(window.clone()).map_err(|e| format!("the window's surface: {e}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or("no graphics adapter for the window")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("velvet-host2d"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .map_err(|e| format!("the graphics device: {e}"))?;
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities.formats.iter().copied().find(wgpu::TextureFormat::is_srgb).unwrap_or(capabilities.formats[0]);
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
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-host2d"),
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
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("velvet-host2d"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("velvet-host2d"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("velvet-host2d"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("velvet-host2d"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        let linear = sampler(wgpu::FilterMode::Linear);
        let nearest = sampler(wgpu::FilterMode::Nearest);
        Ok(Presenter { surface, device, queue, config, layout, pipeline, linear, nearest, source: None })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// The texture for a canvas of a size, made again when the size changes.
    fn source(&mut self, size: (u32, u32)) -> &Source {
        if self.source.as_ref().map(|s| s.size) != Some(size) {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("velvet-host2d-canvas"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind = |sampler: &wgpu::Sampler| {
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("velvet-host2d"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
                    ],
                })
            };
            let smooth = bind(&self.linear);
            let exact = bind(&self.nearest);
            self.source = Some(Source { texture, smooth, exact, size });
        }
        self.source.as_ref().expect("made above")
    }

    fn present(&mut self, canvas: &Canvas, viewport: &Viewport) -> Result<(), String> {
        let size = (canvas.pixel_width(), canvas.pixel_height());
        self.source(size);
        let source = self.source.as_ref().expect("made above");
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &source.texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            canvas.pixels(),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(size.0 * 4), rows_per_image: Some(size.1) },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        );
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(e) => return Err(e.to_string()),
        };
        let view = frame.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("velvet-host2d") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-host2d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // A canvas the picture's own size is copied as it is; another size is scaled smoothly.
            let same = size == (viewport.width, viewport.height);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, if same { &source.exact } else { &source.smooth }, &[]);
            pass.set_viewport(viewport.x as f32, viewport.y as f32, viewport.width as f32, viewport.height as f32, 0.0, 1.0);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        // The upload's staging memory is freed now rather than when the next frame comes.
        let _ = self.device.poll(wgpu::Maintain::Poll);
        Ok(())
    }
}
