//! The 3D renderer's back end: the frame is shaded into a linear HDR buffer, a bloom is built
//! from what shines brighter than white (lamps, flames, glowing fungi, the sun on wet stone), and
//! a full-screen pass tone maps, grades and writes the result into the caller's target.
//!
//! The HDR buffer is cleared to transparent black and every surface blends into it with
//! premultiplied "over", so its alpha is the coverage. The composite unpremultiplies, tone maps
//! the surface colour and lays it over the caller's clear colour, which keeps clear colours
//! exactly as they were written before (they are never tone mapped) and keeps a transparent clear
//! transparent for callers that draw the 3D view over something else.
//!
//! Bloom: a 13-tap downsample chain (the first step thresholded with a soft knee and weighted
//! against single-pixel fireflies), then a tent-filtered upsample that adds each level into the
//! one above. Six levels from half resolution give halos from a few pixels to a fifth of the
//! screen.

const BLOOM_LEVELS: usize = 6;
/// The linear HDR format the frame is shaded into.
pub(crate) const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PostUniform {
    /// threshold, soft knee, bloom strength, 1 = this is the first (thresholded) downsample
    bloom: [f32; 4],
    /// saturation, contrast, exposure, bloom on (1) / off (0)
    grade: [f32; 4],
    /// the caller's clear colour (linear, as written into the target)
    clear: [f32; 4],
}

/// The tone mapping and bloom settings of one frame.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PostSettings {
    pub bloom_strength: f32,
    pub bloom_threshold: f32,
    pub bloom_knee: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub exposure: f32,
    /// How many of the bloom's levels are used (all six for the full halo; fewer: a lighter,
    /// cheaper one).
    pub bloom_levels: usize,
}

struct Level {
    view: wgpu::TextureView,
}

pub(crate) struct Post3D {
    sample_count: u32,
    size: (u32, u32),
    target_format: wgpu::TextureFormat,
    msaa_view: Option<wgpu::TextureView>,
    resolve_view: wgpu::TextureView,
    levels: Vec<Level>,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    up: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    /// One uniform per pass (the first downsample is thresholded, the rest are not).
    first_uniform: wgpu::Buffer,
    uniform: wgpu::Buffer,
    /// The passes' bind groups, rebuilt with the textures.
    groups: Option<Groups>,
}

struct Groups {
    first_down: wgpu::BindGroup,
    downs: Vec<wgpu::BindGroup>,
    ups: Vec<wgpu::BindGroup>,
    composite: wgpu::BindGroup,
}

fn hdr_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    samples: u32,
    label: &str,
    sampled: bool,
) -> wgpu::TextureView {
    let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    if sampled {
        usage |= wgpu::TextureUsages::TEXTURE_BINDING;
    }
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: size.0.max(1),
                height: size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: HDR_FORMAT,
            usage,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

impl Post3D {
    pub(crate) fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        size: (u32, u32),
        sample_count: u32,
    ) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-post-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-3d-post-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-post-shader"),
            source: wgpu::ShaderSource::Wgsl(POST_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-3d-post-pipeline-layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline =
            |entry: &str, format: wgpu::TextureFormat, blend: Option<wgpu::BlendState>| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(entry),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_post"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some(entry),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                })
            };
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let down = pipeline("fs_down", HDR_FORMAT, None);
        let up = pipeline("fs_up", HDR_FORMAT, Some(additive));
        let composite = pipeline("fs_composite", target_format, None);
        let buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: std::mem::size_of::<PostUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let first_uniform = buffer("velvet-3d-post-uniform-first");
        let uniform = buffer("velvet-3d-post-uniform");
        let mut post = Self {
            sample_count,
            size: (0, 0),
            target_format,
            msaa_view: None,
            resolve_view: hdr_texture(device, (1, 1), 1, "velvet-3d-hdr", true),
            levels: Vec::new(),
            layout,
            sampler,
            down,
            up,
            composite,
            first_uniform,
            uniform,
            groups: None,
        };
        post.resize(device, size);
        post
    }

    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.size == size {
            return;
        }
        self.size = size;
        self.msaa_view = (self.sample_count > 1)
            .then(|| hdr_texture(device, size, self.sample_count, "velvet-3d-hdr-msaa", false));
        self.resolve_view = hdr_texture(device, size, 1, "velvet-3d-hdr", true);
        self.levels = (0..BLOOM_LEVELS)
            .map(|k| {
                let s = ((size.0 >> (k + 1)).max(1), (size.1 >> (k + 1)).max(1));
                Level {
                    view: hdr_texture(device, s, 1, "velvet-3d-bloom", true),
                }
            })
            .collect();
        let group = |uniform: &wgpu::Buffer, src: &wgpu::TextureView, extra: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("velvet-3d-post-group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(extra),
                    },
                ],
            })
        };
        let groups = Groups {
            first_down: group(&self.first_uniform, &self.resolve_view, &self.resolve_view),
            downs: (1..BLOOM_LEVELS)
                .map(|k| group(&self.uniform, &self.levels[k - 1].view, &self.resolve_view))
                .collect(),
            ups: (1..BLOOM_LEVELS)
                .map(|k| group(&self.uniform, &self.levels[k].view, &self.resolve_view))
                .collect(),
            composite: group(&self.uniform, &self.resolve_view, &self.levels[0].view),
        };
        self.groups = Some(groups);
    }

    /// Where the scene pass draws (the multisampled buffer, or the HDR buffer itself) and what
    /// it resolves into.
    pub(crate) fn scene_attachment(&self) -> (&wgpu::TextureView, Option<&wgpu::TextureView>) {
        match &self.msaa_view {
            Some(msaa) => (msaa, Some(&self.resolve_view)),
            None => (&self.resolve_view, None),
        }
    }

    /// The frame's linear colour, resolved (what the bounced light reads of the frame before).
    pub(crate) fn scene_color(&self) -> &wgpu::TextureView {
        &self.resolve_view
    }

    #[allow(dead_code)]
    pub(crate) fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }

    /// Bloom (when on), then tone map and grade into `target`.
    pub(crate) fn finish(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        settings: PostSettings,
        clear: wgpu::Color,
    ) {
        let Some(groups) = &self.groups else { return };
        let levels = settings.bloom_levels.clamp(1, BLOOM_LEVELS);
        let bloom_on = settings.bloom_strength > 0.0 && settings.bloom_levels > 0;
        let make = |first: bool| PostUniform {
            bloom: [
                settings.bloom_threshold,
                settings.bloom_knee.max(1e-4),
                settings.bloom_strength,
                if first { 1.0 } else { 0.0 },
            ],
            grade: [
                settings.saturation,
                settings.contrast,
                settings.exposure,
                if bloom_on { 1.0 } else { 0.0 },
            ],
            clear: [
                clear.r as f32,
                clear.g as f32,
                clear.b as f32,
                clear.a as f32,
            ],
        };
        queue.write_buffer(&self.first_uniform, 0, bytemuck::bytes_of(&make(true)));
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&make(false)));
        let pass = |encoder: &mut wgpu::CommandEncoder,
                    view: &wgpu::TextureView,
                    load: wgpu::LoadOp<wgpu::Color>,
                    pipeline: &wgpu::RenderPipeline,
                    group: &wgpu::BindGroup| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-post-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        if bloom_on {
            let clear_black = wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT);
            pass(
                encoder,
                &self.levels[0].view,
                clear_black,
                &self.down,
                &groups.first_down,
            );
            for k in 1..levels {
                pass(
                    encoder,
                    &self.levels[k].view,
                    clear_black,
                    &self.down,
                    &groups.downs[k - 1],
                );
            }
            for k in (1..levels).rev() {
                pass(
                    encoder,
                    &self.levels[k - 1].view,
                    wgpu::LoadOp::Load,
                    &self.up,
                    &groups.ups[k - 1],
                );
            }
        }
        pass(
            encoder,
            target,
            wgpu::LoadOp::Clear(clear),
            &self.composite,
            &groups.composite,
        );
    }
}

const POST_SHADER: &str = r#"
struct Post {
    bloom: vec4<f32>,
    grade: vec4<f32>,
    clear: vec4<f32>,
};
@group(0) @binding(0) var<uniform> post: Post;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var bloom_tex: texture_2d<f32>;

struct V {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_post(@builtin(vertex_index) i: u32) -> V {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: V;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

fn tap(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(src, samp, uv, 0.0).rgb;
}

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// What of a colour shines past the threshold, with a soft knee.
fn bright(c: vec3<f32>) -> vec3<f32> {
    let threshold = post.bloom.x;
    let knee = post.bloom.y;
    let b = max(c.r, max(c.g, c.b));
    var soft = clamp(b - threshold + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee + 0.0001);
    let w = max(soft, b - threshold) / max(b, 0.0001);
    return c * w;
}

// Karis weight: one blinding pixel cannot flood the whole bloom.
fn karis(c: vec3<f32>) -> f32 {
    return 1.0 / (1.0 + luma(c));
}

@fragment
fn fs_down(in: V) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    let uv = in.uv;
    let a = tap(uv + t * vec2<f32>(-2.0, -2.0));
    let b = tap(uv + t * vec2<f32>(0.0, -2.0));
    let c = tap(uv + t * vec2<f32>(2.0, -2.0));
    let d = tap(uv + t * vec2<f32>(-2.0, 0.0));
    let e = tap(uv);
    let f = tap(uv + t * vec2<f32>(2.0, 0.0));
    let g = tap(uv + t * vec2<f32>(-2.0, 2.0));
    let h = tap(uv + t * vec2<f32>(0.0, 2.0));
    let i = tap(uv + t * vec2<f32>(2.0, 2.0));
    let j = tap(uv + t * vec2<f32>(-1.0, -1.0));
    let k = tap(uv + t * vec2<f32>(1.0, -1.0));
    let l = tap(uv + t * vec2<f32>(-1.0, 1.0));
    let m = tap(uv + t * vec2<f32>(1.0, 1.0));
    if (post.bloom.w > 0.5) {
        // The first step: five overlapping boxes, each thresholded and Karis weighted.
        let b0 = bright((j + k + l + m) * 0.25);
        let b1 = bright((a + b + d + e) * 0.25);
        let b2 = bright((b + c + e + f) * 0.25);
        let b3 = bright((d + e + g + h) * 0.25);
        let b4 = bright((e + f + h + i) * 0.25);
        let w0 = karis(b0) * 0.5;
        let w1 = karis(b1) * 0.125;
        let w2 = karis(b2) * 0.125;
        let w3 = karis(b3) * 0.125;
        let w4 = karis(b4) * 0.125;
        let sum = b0 * w0 + b1 * w1 + b2 * w2 + b3 * w3 + b4 * w4;
        return vec4<f32>(sum / max(w0 + w1 + w2 + w3 + w4, 0.0001), 1.0);
    }
    let sum = e * 0.125 + (a + c + g + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
    return vec4<f32>(sum, 1.0);
}

@fragment
fn fs_up(in: V) -> @location(0) vec4<f32> {
    let t = 1.0 / vec2<f32>(textureDimensions(src));
    let uv = in.uv;
    var s = tap(uv) * 4.0;
    s = s + (tap(uv + t * vec2<f32>(-1.0, 0.0)) + tap(uv + t * vec2<f32>(1.0, 0.0)) + tap(uv + t * vec2<f32>(0.0, -1.0)) + tap(uv + t * vec2<f32>(0.0, 1.0))) * 2.0;
    s = s + tap(uv + t * vec2<f32>(-1.0, -1.0)) + tap(uv + t * vec2<f32>(1.0, -1.0)) + tap(uv + t * vec2<f32>(-1.0, 1.0)) + tap(uv + t * vec2<f32>(1.0, 1.0));
    return vec4<f32>(s / 16.0, 0.0);
}

fn aces_film(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn apply_grade(color: vec3<f32>) -> vec3<f32> {
    let grey = luma(color);
    let saturated = mix(vec3<f32>(grey), color, post.grade.x);
    return clamp((saturated - vec3<f32>(0.5)) * post.grade.y + vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_composite(in: V) -> @location(0) vec4<f32> {
    let hdr = textureSampleLevel(src, samp, in.uv, 0.0);
    let a = clamp(hdr.a, 0.0, 1.0);
    var light = vec3<f32>(0.0);
    if (post.grade.w > 0.5) {
        light = textureSampleLevel(bloom_tex, samp, in.uv, 0.0).rgb * post.bloom.z;
    }
    if (a <= 0.0001) {
        // Nothing drawn: the clear colour as written, and whatever bloom spills over it.
        let spill = aces_film(light * post.grade.z * 1.12);
        return vec4<f32>(post.clear.rgb + spill * (1.0 - post.clear.rgb), post.clear.a);
    }
    let surface = hdr.rgb / a + light;
    let mapped = apply_grade(aces_film(max(surface * post.grade.z * 1.12, vec3<f32>(0.0))));
    let spill = aces_film(light * post.grade.z * 1.12);
    let behind = post.clear.rgb + spill * (1.0 - post.clear.rgb);
    return vec4<f32>(mapped * a + behind * (1.0 - a), a + post.clear.a * (1.0 - a));
}
"#;
