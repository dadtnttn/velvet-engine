//! Light that bounces: screen-space global illumination for the 3D renderer.
//!
//! Before a frame is shaded, a compute pass reads the frame before it — its lit linear colour and
//! its depth — and for every pixel, at half resolution, gathers the light that the surfaces round
//! it send its way: a dozen samples in a disc a few metres wide, each weighted by how squarely the
//! two surfaces face each other and how far apart they are (the form factor of a small disc), with
//! nothing tested for in between. That is blended into the result before it, moved to where the
//! camera has gone since, so the noise of so few samples settles over a few frames; a pixel whose
//! surface was not there before starts afresh.
//!
//! The frame's surfaces then read the result back by their world position and light their albedo
//! with it, as they do the sky's ambient light: a torch-lit floor warms the walls round it, a red
//! carpet tints the stone beside it, a sunlit square lifts the shaded side of a house. The light it
//! reads already holds the bounce of the frame before, so a second bounce builds up over frames.
//! What the camera cannot see (the far side of a wall, what is off screen) sends nothing: it only
//! ever adds light where the eye could find where it came from.

use bytemuck::{Pod, Zeroable};
use velvet_math::{Mat4, Vec3};

/// Samples gathered per pixel and frame.
const SAMPLES: u32 = 12;
/// How much of each new gather goes into the result (the rest is the history).
const BLEND: f32 = 0.12;
/// The format of the result: rgb the gathered light, a the pixel's depth along the view.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GiUniform {
    /// world → clip of the frame read, and back
    view_projection: [f32; 16],
    inverse_view_projection: [f32; 16],
    /// world → clip of the frame the history was gathered for
    history_view_projection: [f32; 16],
    /// the frame read: camera position, gather radius (m)
    camera: [f32; 4],
    /// its view axis, 1 when the history can be used
    forward: [f32; 4],
    /// its right axis, samples per pixel
    right: [f32; 4],
    /// the history's camera position, the weight of the new gather
    history_camera: [f32; 4],
    /// the history's view axis, the frame number (the noise turns with it)
    history_forward: [f32; 4],
    /// full width, height; half width, height
    size: [f32; 4],
}

/// The camera a frame was shaded with: what its depth and colour mean in the world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GiCamera {
    pub view_projection: Mat4,
    pub position: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub size: (u32, u32),
}

impl GiCamera {
    pub(crate) fn of(camera: &crate::mesh3d::Camera3D, size: (u32, u32)) -> Self {
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let forward = (camera.target - camera.position).normalize_or_zero();
        let right = forward.cross(camera.up).normalize_or_zero();
        Self {
            view_projection: camera.view_projection(aspect),
            position: camera.position,
            forward,
            right,
            size,
        }
    }
}

pub(crate) struct ScreenGi {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    /// Two half-resolution results: the latest (`front`) and the one before it (its history).
    targets: Vec<(wgpu::Texture, wgpu::TextureView)>,
    half: (u32, u32),
    front: usize,
    frame: u32,
    /// The camera of the frame last shaded: its depth and colour are what the next gather reads.
    shaded: Option<GiCamera>,
    /// The camera the latest result was gathered for (none: there is no result).
    gathered: Option<GiCamera>,
    /// Black, for the frames with no result.
    blank: wgpu::TextureView,
}

fn result_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

impl ScreenGi {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue, sample_count: u32) -> Self {
        let multisampled = sample_count > 1;
        let entry = |binding: u32, ty: wgpu::BindingType| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-gi-layout"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled,
                    },
                ),
                entry(
                    2,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    3,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    4,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                entry(
                    5,
                    wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                ),
            ],
        });
        let source = GI_SHADER.replace(
            "__DEPTH__",
            if multisampled {
                "texture_depth_multisampled_2d"
            } else {
                "texture_depth_2d"
            },
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-gi-shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-3d-gi-pipeline-layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("velvet-3d-gi-pipeline"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("gather"),
            compilation_options: Default::default(),
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-gi-uniform"),
            size: std::mem::size_of::<GiUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-3d-gi-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let blank_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velvet-3d-gi-blank"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        // (Half floats of 0: no light; its depth -1 matches no surface.)
        let minus_one = 0xBC00u16;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &blank_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&[0u16, 0, 0, minus_one]),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(8),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let blank = blank_texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            layout,
            pipeline,
            uniform,
            sampler,
            targets: Vec::new(),
            half: (0, 0),
            front: 0,
            frame: 0,
            shaded: None,
            gathered: None,
            blank,
        }
    }

    /// Forgets what was gathered (a new place: nothing of the old one should glow in it).
    pub(crate) fn reset(&mut self) {
        self.shaded = None;
        self.gathered = None;
    }

    /// The frame just shaded, with `camera`: the next gather reads it.
    pub(crate) fn shaded(&mut self, camera: GiCamera) {
        self.shaded = Some(camera);
    }

    /// Gathers the bounced light of the frame last shaded (its `depth` and `color` still hold it),
    /// unless there is none or it was another size. `radius`: how far light travels, in metres.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn gather(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        color: &wgpu::TextureView,
        radius: f32,
        depth_size: (u32, u32),
    ) {
        // (A resize remade the depth: what it held is gone.)
        let Some(read) = self.shaded.take().filter(|r| r.size == depth_size) else {
            self.gathered = None;
            return;
        };
        let half = (
            read.size.0.div_ceil(2).max(1),
            read.size.1.div_ceil(2).max(1),
        );
        if self.half != half || self.targets.is_empty() {
            self.targets = (0..2)
                .map(|i| {
                    result_texture(
                        device,
                        half,
                        if i == 0 {
                            "velvet-3d-gi-a"
                        } else {
                            "velvet-3d-gi-b"
                        },
                    )
                })
                .collect();
            self.half = half;
            self.gathered = None;
        }
        let history = self.gathered.filter(|h| h.size == read.size);
        let back = 1 - self.front;
        self.frame = self.frame.wrapping_add(1);
        let h = history.unwrap_or(read);
        let uniform = GiUniform {
            view_projection: read.view_projection.to_cols_array(),
            inverse_view_projection: read
                .view_projection
                .inverse()
                .unwrap_or(Mat4::IDENTITY)
                .to_cols_array(),
            history_view_projection: h.view_projection.to_cols_array(),
            camera: [
                read.position.x,
                read.position.y,
                read.position.z,
                radius.max(0.1),
            ],
            forward: [
                read.forward.x,
                read.forward.y,
                read.forward.z,
                if history.is_some() { 1.0 } else { 0.0 },
            ],
            right: [read.right.x, read.right.y, read.right.z, SAMPLES as f32],
            history_camera: [h.position.x, h.position.y, h.position.z, BLEND],
            history_forward: [
                h.forward.x,
                h.forward.y,
                h.forward.z,
                (self.frame % 1024) as f32,
            ],
            size: [
                read.size.0 as f32,
                read.size.1 as f32,
                half.0 as f32,
                half.1 as f32,
            ],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&uniform));
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-gi-group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(color),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.targets[self.front].1),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&self.targets[back].1),
                },
            ],
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("velvet-3d-gi-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(half.0.div_ceil(8), half.1.div_ceil(8), 1);
        }
        self.front = back;
        self.gathered = Some(read);
    }

    /// What the frame's surfaces read: the latest result and the camera it was gathered for (or
    /// the black stand-in and none).
    pub(crate) fn result(&self) -> (&wgpu::TextureView, Option<GiCamera>) {
        match self.gathered {
            Some(camera) if !self.targets.is_empty() => (&self.targets[self.front].1, Some(camera)),
            _ => (&self.blank, None),
        }
    }

    pub(crate) fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// The latest result read back (width, height, rgba per pixel), to look at it.
    pub(crate) fn read(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(u32, u32, Vec<[f32; 4]>)> {
        self.gathered?;
        let (texture, _) = self.targets.get(self.front)?;
        let (w, h) = self.half;
        let row = (w * 8).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-gi-readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("velvet-3d-gi-readback"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = sender.send(r);
        });
        let _ = device.poll(wgpu::Maintain::Wait);
        receiver.recv().ok()?.ok()?;
        let data = buffer.slice(..).get_mapped_range();
        let half = |b: &[u8]| half_to_f32(u16::from_le_bytes([b[0], b[1]]));
        let mut out = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let o = (y * row + x * 8) as usize;
                out.push([
                    half(&data[o..]),
                    half(&data[o + 2..]),
                    half(&data[o + 4..]),
                    half(&data[o + 6..]),
                ]);
            }
        }
        Some((w, h, out))
    }
}

fn half_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = ((h >> 10) & 0x1f) as i32;
    let mantissa = (h & 0x3ff) as f32;
    sign * match exponent {
        0 => mantissa / 1024.0 * 2f32.powi(-14),
        31 => f32::INFINITY,
        e => (1.0 + mantissa / 1024.0) * 2f32.powi(e - 15),
    }
}

const GI_SHADER: &str = r#"
struct Gi {
    view_projection: mat4x4<f32>,
    inverse_view_projection: mat4x4<f32>,
    history_view_projection: mat4x4<f32>,
    camera: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    history_camera: vec4<f32>,
    history_forward: vec4<f32>,
    size: vec4<f32>,
};
@group(0) @binding(0) var<uniform> gi: Gi;
@group(0) @binding(1) var depth_texture: __DEPTH__;
@group(0) @binding(2) var color_texture: texture_2d<f32>;
@group(0) @binding(3) var history_texture: texture_2d<f32>;
@group(0) @binding(4) var history_sampler: sampler;
@group(0) @binding(5) var result: texture_storage_2d<rgba16float, write>;

const PI: f32 = 3.14159265;

fn depth_at(p: vec2<i32>) -> f32 {
    let q = clamp(p, vec2<i32>(0), vec2<i32>(gi.size.xy) - vec2<i32>(1));
    return textureLoad(depth_texture, q, 0);
}

fn world_at(p: vec2<i32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(p) + 0.5) / gi.size.xy;
    let clip = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, depth, 1.0);
    let world = gi.inverse_view_projection * clip;
    return world.xyz / world.w;
}

// The surface's normal from the depth round it: on each axis the neighbour nearer in depth
// (so an edge does not bend it), turned to face the camera.
fn normal_at(p: vec2<i32>, position: vec3<f32>, depth: f32, both_sides: bool) -> vec3<f32> {
    var dx = world_at(p + vec2<i32>(1, 0), depth_at(p + vec2<i32>(1, 0))) - position;
    var dy = world_at(p + vec2<i32>(0, 1), depth_at(p + vec2<i32>(0, 1))) - position;
    if (both_sides) {
        let left = depth_at(p - vec2<i32>(1, 0));
        if (abs(left - depth) < abs(depth_at(p + vec2<i32>(1, 0)) - depth)) {
            dx = position - world_at(p - vec2<i32>(1, 0), left);
        }
        let up = depth_at(p - vec2<i32>(0, 1));
        if (abs(up - depth) < abs(depth_at(p + vec2<i32>(0, 1)) - depth)) {
            dy = position - world_at(p - vec2<i32>(0, 1), up);
        }
    }
    var n = normalize(cross(dy, dx));
    if (dot(n, -gi.forward.xyz) < 0.0) {
        n = -n;
    }
    return n;
}

// Two numbers in 0..1 for a pixel and a frame (a hash: neighbouring pixels share no pattern).
fn noise(p: vec2<i32>, frame: u32) -> vec2<f32> {
    var v = u32(p.x) * 1973u + u32(p.y) * 9277u + frame * 26699u;
    v = (v ^ 61u) ^ (v >> 16u);
    v = v * 9u;
    v = v ^ (v >> 4u);
    v = v * 0x27d4eb2du;
    v = v ^ (v >> 15u);
    return vec2<f32>(f32(v & 0xffffu), f32(v >> 16u)) / 65535.0;
}

@compute @workgroup_size(8, 8)
fn gather(@builtin(global_invocation_id) id: vec3<u32>) {
    let half = vec2<i32>(gi.size.zw);
    let g = vec2<i32>(id.xy);
    if (g.x >= half.x || g.y >= half.y) {
        return;
    }
    let frame = u32(gi.history_forward.w);
    // Which of its four full pixels a half pixel reads turns over the frames.
    let jitter = vec2<i32>(i32(frame & 1u), i32((frame >> 1u) & 1u));
    let p = min(g * 2 + jitter, vec2<i32>(gi.size.xy) - vec2<i32>(1));
    let depth = depth_at(p);
    if (depth >= 1.0) {
        textureStore(result, g, vec4<f32>(0.0, 0.0, 0.0, -1.0));
        return;
    }
    let position = world_at(p, depth);
    let normal = normal_at(p, position, depth, true);
    // The disc on the screen: the radius in metres, measured across the view at this point.
    let radius = gi.camera.w;
    let c0 = gi.view_projection * vec4<f32>(position, 1.0);
    let c1 = gi.view_projection * vec4<f32>(position + gi.right.xyz * radius, 1.0);
    let pixels = max(length((c1.xy / c1.w - c0.xy / c0.w) * 0.5 * gi.size.xy), 1.0);
    let metres = radius / pixels;
    let count = i32(gi.right.w);
    let turn = noise(p, frame);
    let share = PI * pixels * pixels / f32(count) * metres * metres;
    var gathered = vec3<f32>(0.0);
    for (var i = 0; i < count; i = i + 1) {
        let k = (f32(i) + turn.y) / f32(count);
        let angle = (f32(i) + turn.x) * 2.39996323;
        let offset = vec2<f32>(cos(angle), sin(angle)) * sqrt(k) * pixels;
        let q = p + vec2<i32>(round(offset));
        if (any(q < vec2<i32>(0)) || any(q >= vec2<i32>(gi.size.xy))) {
            continue;
        }
        let dq = depth_at(q);
        if (dq >= 1.0) {
            continue;
        }
        let sender = world_at(q, dq);
        let to = sender - position;
        let d2 = dot(to, to);
        if (d2 < 0.0004) {
            continue;
        }
        let dir = to * inverseSqrt(d2);
        let facing = dot(normal, dir);
        if (facing <= 0.02) {
            continue;
        }
        let sender_normal = normal_at(q, sender, dq, false);
        let sending = dot(sender_normal, -dir);
        if (sending <= 0.02) {
            continue;
        }
        // The colour there is the light it sends; kept from blinding (a flame is a few pixels).
        let lit = textureLoad(color_texture, q, 0);
        var light = lit.rgb;
        let bright = max(max(light.r, light.g), light.b);
        light = light * min(1.0, 3.0 / max(bright, 0.0001));
        // The surface a sample stands for: its share of the disc, slanted away from the camera.
        let area = share / max(abs(dot(sender_normal, gi.forward.xyz)), 0.25);
        gathered = gathered + light * facing * sending * area / (PI * d2 + area);
    }
    // Along the view: what the history keeps for each pixel, to know it is the same surface.
    let along = dot(position - gi.camera.xyz, gi.forward.xyz);
    var light = gathered;
    if (gi.forward.w > 0.5) {
        let clip = gi.history_view_projection * vec4<f32>(position, 1.0);
        let ndc = clip.xyz / clip.w;
        let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
            let before = textureSampleLevel(history_texture, history_sampler, uv, 0.0);
            let expected = dot(position - gi.history_camera.xyz, gi.history_forward.xyz);
            if (before.a > 0.0 && abs(before.a - expected) < 0.35 + expected * 0.01) {
                light = mix(before.rgb, gathered, gi.history_camera.w);
            }
        }
    }
    textureStore(result, g, vec4<f32>(light, along));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(source: &str) {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}"));
    }

    #[test]
    fn the_gather_shader_is_valid_with_and_without_multisampling() {
        validate(&GI_SHADER.replace("__DEPTH__", "texture_depth_2d"));
        validate(&GI_SHADER.replace("__DEPTH__", "texture_depth_multisampled_2d"));
    }

    #[test]
    fn the_mesh_shader_reads_the_bounced_light() {
        let source = crate::mesh3d::mesh_shader_source();
        assert!(source.contains("bounced_light(input.world_position)"));
        validate(source);
    }
}
