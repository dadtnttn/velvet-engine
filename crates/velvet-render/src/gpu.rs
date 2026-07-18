//! wgpu device, surface, and sprite pipeline.

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tracing::info;
use velvet_math::Vec2;

use crate::batch::SpriteBatch;
use crate::camera::Camera2D;
use crate::letterbox::{compute_letterbox, letterbox_viewport, ScalingMode};
use crate::profile::RenderProfile;
use crate::stats::RenderStats;
use crate::texture::{TextureId, TextureInfo, TextureStore};
use crate::ClearColor;

/// GPU-related errors.
#[derive(Debug, Error)]
pub enum GpuError {
    /// Request adapter/device failed.
    #[error("gpu request failed: {0}")]
    Request(String),
    /// Surface error.
    #[error("surface: {0}")]
    Surface(String),
    /// Validation / internal.
    #[error("gpu: {0}")]
    Message(String),
}

/// Frame acquired from a surface.
pub struct SurfaceFrame {
    /// Texture to render into.
    pub texture: wgpu::SurfaceTexture,
    /// View of the texture.
    pub view: wgpu::TextureView,
}

/// Optional window surface wrapper.
pub struct RenderSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

impl RenderSurface {
    /// Current size.
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Resize surface.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }

    /// Acquire next frame.
    pub fn acquire(&self) -> Result<SurfaceFrame, GpuError> {
        let texture = self
            .surface
            .get_current_texture()
            .map_err(|e| GpuError::Surface(e.to_string()))?;
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok(SurfaceFrame { texture, view })
    }
}

struct GpuTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
}

/// Core GPU context for 2D rendering.
pub struct GpuContext {
    /// Device.
    pub device: wgpu::Device,
    /// Queue.
    pub queue: wgpu::Queue,
    /// Adapter info string.
    pub adapter_info: String,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler_nearest: wgpu::Sampler,
    sampler_linear: wgpu::Sampler,
    instance_buffer: wgpu::Buffer,
    instance_capacity: u64,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    camera_bind_layout: wgpu::BindGroupLayout,
    textures: HashMap<TextureId, GpuTexture>,
    /// CPU texture metadata.
    pub texture_store: TextureStore,
    /// White 1x1 texture for untextured quads.
    pub white_texture: TextureId,
    /// Active profile.
    pub profile: RenderProfile,
    /// Scaling mode (may override profile).
    pub scaling: ScalingMode,
    /// Virtual resolution.
    pub virtual_size: Vec2,
    /// Last frame stats.
    pub stats: RenderStats,
    /// Clear color.
    pub clear: ClearColor,
}

impl GpuContext {
    /// Create a headless GPU context (no surface) for tests / offline work.
    pub fn headless() -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: true,
        }))
        .ok_or_else(|| GpuError::Request("no adapter (headless)".into()))?;

        let info = adapter.get_info();
        let adapter_info = format!("{:?} {:?} {:?}", info.backend, info.name, info.device_type);
        info!(%adapter_info, "wgpu adapter (headless)");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("velvet-headless"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| GpuError::Request(e.to_string()))?;

        Self::from_device_queue(device, queue, adapter_info)
    }

    /// Create GPU context and a surface for a raw window handle pair via winit Window.
    #[cfg(feature = "window")]
    pub fn with_window(
        window: Arc<winit::window::Window>,
    ) -> Result<(Self, RenderSurface), GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| GpuError::Surface(e.to_string()))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| GpuError::Request("no adapter".into()))?;

        let info = adapter.get_info();
        let adapter_info = format!("{:?} {:?} {:?}", info.backend, info.name, info.device_type);
        info!(%adapter_info, "wgpu adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("velvet-windowed"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .map_err(|e| GpuError::Request(e.to_string()))?;

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let render_surface = RenderSurface { surface, config };

        let ctx = Self::from_device_queue(device, queue, adapter_info)?;
        Ok((ctx, render_surface))
    }

    fn from_device_queue(
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter_info: String,
    ) -> Result<Self, GpuError> {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite-shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite-texture-layout"),
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

        let camera_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite-pipeline-layout"),
            bind_group_layouts: &[&camera_bind_layout, &bind_group_layout],
            push_constant_ranges: &[],
        });

        // Use a common surface format for headless pipeline; window path recreates if needed.
        // For simplicity we use Bgra8UnormSrgb as default pipeline target.
        let target_format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<crate::sprite::SpriteInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x4,
                        1 => Float32x4,
                        2 => Float32x4,
                        3 => Float32x4,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nearest"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera-ubo"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bg"),
            layout: &camera_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let instance_capacity = 1024u64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: instance_capacity * std::mem::size_of::<crate::sprite::SpriteInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut ctx = Self {
            device,
            queue,
            adapter_info,
            pipeline,
            bind_group_layout,
            sampler_nearest,
            sampler_linear,
            instance_buffer,
            instance_capacity,
            camera_buffer,
            camera_bind_group,
            camera_bind_layout,
            textures: HashMap::new(),
            texture_store: TextureStore::new(),
            white_texture: TextureId::NONE,
            profile: RenderProfile::Default,
            scaling: ScalingMode::Letterbox,
            virtual_size: Vec2::new(1920.0, 1080.0),
            stats: RenderStats::default(),
            clear: ClearColor::default(),
        };

        // 1x1 white texture
        let white = ctx.create_texture_rgba8("white", 1, 1, &[255, 255, 255, 255])?;
        ctx.white_texture = white;
        Ok(ctx)
    }

    /// Upload RGBA8 texture.
    pub fn create_texture_rgba8(
        &mut self,
        label: &str,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<TextureId, GpuError> {
        if data.len() < (width * height * 4) as usize {
            return Err(GpuError::Message("rgba8 buffer too small".into()));
        }
        let id = TextureId::allocate();
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = if self.profile.nearest_textures() {
            &self.sampler_nearest
        } else {
            &self.sampler_linear
        };
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        self.textures.insert(
            id,
            GpuTexture {
                texture,
                view,
                bind_group,
                width,
                height,
            },
        );
        self.texture_store.register(TextureInfo {
            id,
            width,
            height,
            label: label.into(),
        });
        Ok(id)
    }

    /// Load PNG/JPEG bytes via `image` crate.
    pub fn load_image_bytes(&mut self, label: &str, bytes: &[u8]) -> Result<TextureId, GpuError> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| GpuError::Message(e.to_string()))?
            .to_rgba8();
        let (w, h) = img.dimensions();
        self.create_texture_rgba8(label, w, h, &img)
    }

    /// Apply render profile defaults.
    pub fn set_profile(&mut self, profile: RenderProfile) {
        self.profile = profile;
        self.scaling = profile.scaling_mode();
    }

    fn ensure_instance_capacity(&mut self, count: u64) {
        if count <= self.instance_capacity {
            return;
        }
        let mut cap = self.instance_capacity;
        while cap < count {
            cap *= 2;
        }
        self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: cap * std::mem::size_of::<crate::sprite::SpriteInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = cap;
    }

    fn write_camera(&self, camera: &Camera2D) {
        let vp = camera.view_projection();
        // Pack mat3 as mat4-like 4x4 with pad for alignment (std140-ish simplified):
        // We send 12 floats (3 columns of vec4 with w=0/1).
        let cols = vp.to_cols_array();
        // column-major mat3 in mat4:
        let mut ubo = [0.0f32; 16];
        ubo[0] = cols[0];
        ubo[1] = cols[1];
        ubo[2] = cols[2];
        ubo[3] = 0.0;
        ubo[4] = cols[3];
        ubo[5] = cols[4];
        ubo[6] = cols[5];
        ubo[7] = 0.0;
        ubo[8] = cols[6];
        ubo[9] = cols[7];
        ubo[10] = cols[8];
        ubo[11] = 0.0;
        ubo[15] = 1.0;
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&ubo));
    }

    /// Render a sorted sprite batch into a texture view (full physical size).
    pub fn render_batch(
        &mut self,
        target: &wgpu::TextureView,
        physical: (u32, u32),
        camera: &Camera2D,
        batch: &mut SpriteBatch,
    ) {
        let start = std::time::Instant::now();
        batch.sort();
        self.stats = batch.stats.clone();

        let lb = compute_letterbox(
            physical.0 as f32,
            physical.1 as f32,
            self.virtual_size.x,
            self.virtual_size.y,
            self.scaling,
        );
        let (vx, vy, vw, vh) = letterbox_viewport(lb);

        self.write_camera(camera);

        let instances: Vec<_> = batch.commands().iter().map(|c| c.instance).collect();
        self.ensure_instance_capacity(instances.len() as u64);
        if !instances.is_empty() {
            self.queue
                .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velvet-frame"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_viewport(vx, vy, vw, vh, 0.0, 1.0);
            pass.set_scissor_rect(vx as u32, vy as u32, vw as u32, vh as u32);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            let mut draw_calls = 0u32;
            let mut texture_binds = 0u32;
            let mut base = 0u32;
            for tex_batch in batch.batches() {
                let gpu_tex = match self.textures.get(&tex_batch.texture) {
                    Some(t) => t,
                    None => match self.textures.get(&self.white_texture) {
                        Some(t) => t,
                        None => continue,
                    },
                };
                pass.set_bind_group(1, &gpu_tex.bind_group, &[]);
                texture_binds += 1;
                let count = tex_batch.instances.len() as u32;
                pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                // 6 vertices for quad via vertex_index in shader
                pass.draw(0..6, base..base + count);
                base += count;
                draw_calls += 1;
            }
            self.stats.finish_draw_calls(draw_calls);
            self.stats.texture_binds = texture_binds;
            self.stats.cameras = 1;
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.stats.cpu_encode_us = start.elapsed().as_micros() as u64;
    }

    /// Render clear-only (no sprites) — useful for empty window criterion.
    pub fn clear_view(&mut self, target: &wgpu::TextureView) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear.to_wgpu()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

const SPRITE_SHADER: &str = r#"
struct Camera {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;

struct Instance {
    @location(0) i0: vec4<f32>,
    @location(1) i1: vec4<f32>,
    @location(2) i2: vec4<f32>,
    @location(3) i3: vec4<f32>,
};

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32, inst: Instance) -> VsOut {
    // Unit quad corners
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
    );
    let local = positions[vi];
    let translation = inst.i0.xy;
    let rotation = inst.i0.z;
    let size = inst.i1.xy;
    let anchor = inst.i1.zw;
    let uv_min = inst.i2.xy;
    let uv_max = inst.i2.zw;
    let tint = inst.i3;

    let pivoted = (local - anchor) * size;
    let c = cos(rotation);
    let s = sin(rotation);
    let rotated = vec2<f32>(pivoted.x * c - pivoted.y * s, pivoted.x * s + pivoted.y * c);
    let world = rotated + translation;
    let clip = camera.view_proj * vec4<f32>(world, 0.0, 1.0);

    var out: VsOut;
    out.clip = clip;
    out.uv = mix(uv_min, uv_max, local);
    out.tint = tint;
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let tex = textureSample(t_diffuse, s_diffuse, in.uv);
    return tex * in.tint;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_gpu_or_skip() {
        match GpuContext::headless() {
            Ok(ctx) => {
                assert!(!ctx.adapter_info.is_empty());
                assert!(!ctx.white_texture.is_none());
            }
            Err(e) => {
                // CI / machines without GPU: acceptable with recorded reason.
                eprintln!("headless gpu unavailable: {e}");
            }
        }
    }

    #[test]
    fn create_solid_texture_if_gpu() {
        let Ok(mut ctx) = GpuContext::headless() else {
            return;
        };
        let id = ctx
            .create_texture_rgba8(
                "red",
                2,
                2,
                &[
                    255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
                ],
            )
            .unwrap();
        assert!(ctx.texture_store.get(id).is_some());
    }
}
