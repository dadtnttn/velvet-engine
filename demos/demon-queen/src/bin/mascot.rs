//! Chiquita over the screen. Her model (`characters/chiquita.glb`, made from her AccuRIG FBX by
//! `tools/chiquita_blender.py`: her body, her hair, her clips) is drawn by a small renderer of her
//! own into a transparent picture the size of the screen, which the overlay lays over everything
//! else: so she can stand by the scenes, turn her back on them, step in front of them and hit the
//! glass.
//!
//! Skinned on the GPU from the pose of one clip or two crossing over, her hair then swung by its own
//! springs (`mascot_hair`); lit as a cel-shaded figure (a warm key, a lilac shadow, a rim of light,
//! a glint on the hair) with an ink line round her (her hull pushed out along its normals, drawn
//! inside out); multisampled. Her eyes and mouth are painted on her texture, which is cut in pieces
//! all over its atlas: so they are found by where they are on her face (her rest pose's
//! positions), and the shader closes lids over them to blink and parts her lips as she talks.
//! The camera looks down −Z from the origin; she stands where the game says her feet are on the
//! screen, as tall as it says, six units away.
use velvet_math::{Mat4, Vec3};
use velvet_render::GltfModel;

use super::mascot_hair::HairSim;
use super::*;

/// Her clips, in the order the game names them (`MascotView::clip`).
pub(crate) const CLIPS: [&str; 13] = [
    "Chq_Idle",
    "Chq_Talk",
    "Chq_Jealous",
    "Chq_Away",
    "Chq_Flustered",
    "Chq_Cheer",
    "Chq_Splat",
    "Chq_Punch",
    "Chq_Enter",
    "Chq_Scold",
    "Chq_Held",
    "Chq_Cover",
    "Chq_Dizzy",
];

/// The screen she is drawn for (the overlay's logical size).
const W: u32 = 1920;
const H: u32 = 1080;
const SAMPLES: u32 = 4;
const FOV: f32 = 0.454; // 26°
const DEPTH: f32 = 6.0;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// Where and how she is this frame.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct MascotView {
    /// The clip and how far into it (ms); the clip before it, still fading out under it.
    pub clip: u8,
    pub time_ms: u32,
    pub prev_clip: u8,
    pub prev_time_ms: u32,
    /// How much the current clip has taken over, 0..=1000.
    pub blend: u16,
    /// Her feet on the screen (px) and how tall she stands there (px).
    pub feet: [i32; 2],
    pub height: u16,
    /// Turned about her height (to her left, +), tipped forward (+) and rolled (tenths of a degree).
    pub yaw: i16,
    pub pitch: i16,
    pub roll: i16,
    /// Her width, height and depth squashed or stretched (per mille).
    pub squash: [u16; 3],
    /// How much of her shows, 0..=1000.
    pub alpha: u16,
    /// The glass she broke.
    pub crack: Option<super::glass_fx::Crack>,
    /// The game's clock (ms): her hair's springs run on it.
    pub clock_ms: u32,
    /// Her eyes closed (0 open, 1000 shut) and her mouth open (0..=1000).
    pub blink: u16,
    pub mouth: u16,
    /// Hypnotised: spirals turning in her eyes (0..=1000).
    pub dazed: u16,
}

impl MascotView {
    /// The top of her head on the screen (px): where her words hang from.
    pub fn head(&self) -> [f32; 2] {
        let h = self.height as f32 * self.squash[1] as f32 / 1000.0;
        [self.feet[0] as f32, self.feet[1] as f32 - h * 0.98]
    }

    pub fn shown(&self) -> bool {
        self.alpha > 0 && self.height > 0
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    joints: [u32; 4],
    weights: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [f32; 16],
    model: [f32; 16],
    /// Toward the key light (camera space), and the ink line's width (her units).
    light: [f32; 4],
    ink: [f32; 4],
    /// Shadow tint (rgb) and rim strength.
    look: [f32; 4],
    /// This part has her face (1), her eyes shut (0..1), her mouth open (0..1), its glint.
    face: [f32; 4],
    /// The spirals in her eyes (0..1) and the clock (s) they turn with.
    fx: [f32; 4],
}

struct Part {
    mesh: usize,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    uniforms: wgpu::Buffer,
    joints: wgpu::Buffer,
    group: wgpu::BindGroup,
    texture: wgpu::BindGroup,
    /// Her body (the face is on it) or her hair (it glints).
    body: bool,
}

pub(crate) struct Mascot {
    model: GltfModel,
    clips: Vec<Option<usize>>,
    parts: Vec<Part>,
    hair: Option<HairSim>,
    pipeline: wgpu::RenderPipeline,
    outline: wgpu::RenderPipeline,
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
    _picture: wgpu::TextureView,
    /// The picture's texture (read back for her portraits).
    picture_texture: wgpu::Texture,
    /// Her picture as the overlay binds it (its picture layout).
    pub(super) picture_group: wgpu::BindGroup,
    /// What was drawn last (the picture is kept while she has not changed).
    last: Option<MascotView>,
}

const SHADER: &str = r#"
struct U {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
    light: vec4<f32>,
    ink: vec4<f32>,
    look: vec4<f32>,
    face: vec4<f32>,
    fx: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var<storage, read> joints: array<mat4x4<f32>>;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct VIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) j: vec4<u32>,
    @location(4) w: vec4<f32>,
};
struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) n: vec3<f32>,
    @location(2) p: vec3<f32>,
    @location(3) rest: vec3<f32>,
};

fn skin(v: VIn) -> mat4x4<f32> {
    return joints[v.j.x] * v.w.x + joints[v.j.y] * v.w.y + joints[v.j.z] * v.w.z + joints[v.j.w] * v.w.w;
}

@vertex
fn vs_main(v: VIn) -> VOut {
    let m = skin(v);
    let p = m * vec4<f32>(v.position, 1.0);
    let n = (m * vec4<f32>(v.normal, 0.0)).xyz;
    var o: VOut;
    o.clip = u.mvp * p;
    o.uv = v.uv;
    o.n = (u.model * vec4<f32>(n, 0.0)).xyz;
    o.p = (u.model * p).xyz;
    o.rest = v.position;
    return o;
}

@vertex
fn vs_outline(v: VIn) -> @builtin(position) vec4<f32> {
    let m = skin(v);
    let p = (m * vec4<f32>(v.position, 1.0)).xyz;
    let n = normalize((m * vec4<f32>(v.normal, 0.0)).xyz + vec3<f32>(0.0, 0.0, 1e-6));
    return u.mvp * vec4<f32>(p + n * u.light.w, 1.0);
}

// Her eyes and mouth as they are painted on her face, in her rest pose (x across, y up, metres
// of her one-unit height): each eye's middle (halfway between its corners), the turn of the line
// through its corners (the outer corner up), half the distance between them, how high the upper
// lid arches over that line and how low the lower lid sags under it; the line between her lips.
const EYE_L = vec2<f32>(0.0716, 0.7040);
const EYE_R = vec2<f32>(-0.0716, 0.7040);
const EYE_TURN = 0.335;
const EYE_HALF = 0.0385;
const EYE_TOP = 0.0290;
const EYE_LOW = 0.0118;
const MOUTH = vec2<f32>(0.0, 0.6445);
// The lid's skin (her eyeshadow) and the lash line (linear colours).
const LID = vec3<f32>(0.45, 0.30, 0.63);
const LASH = vec3<f32>(0.02, 0.008, 0.03);

// A lid over one eye, closing as `shut` goes to 1: the upper lid comes down over the painted eye
// to a line just above the lower lid, which rises the last bit to meet it; a lash line runs along
// the upper lid's edge, thickening as it closes. `px`: how much of her a screen pixel spans.
fn lid(base: vec3<f32>, r: vec3<f32>, centre: vec2<f32>, turn: f32, shut: f32, px: f32) -> vec3<f32> {
    if (shut < 0.01) {
        return base;
    }
    let d = r.xy - centre;
    let c = cos(turn);
    let s = sin(turn);
    let x = d.x * c + d.y * s;
    let y = -d.x * s + d.y * c;
    let a = x / EYE_HALF;
    if (abs(a) > 1.25 || y > EYE_TOP * 1.6 || y < -EYE_LOW * 2.5) {
        return base;
    }
    let k = max(1.0 - a * a / 1.1, 0.0);
    let top = EYE_TOP * pow(k, 0.6);
    let low = -EYE_LOW * pow(k, 0.8) - 0.0015;
    let closed = low * 0.55;
    let edge = mix(top + 0.0015, closed, shut);
    let floor_line = mix(low - 0.0015, closed, smoothstep(0.6, 1.0, shut));
    let aa = px * 1.2 + 0.0002;
    let across = 1.0 - smoothstep(1.04, 1.1 + aa / EYE_HALF, abs(a));
    let upper = smoothstep(edge - aa, edge + aa, y) * (1.0 - smoothstep(top + 0.002, top + 0.002 + aa, y));
    let lower = (1.0 - smoothstep(floor_line - aa, floor_line + aa, y)) * smoothstep(low - 0.002 - aa, low - 0.002, y);
    // The lid darkens toward its edge, in the shadow of the lashes.
    let shade = mix(0.8, 1.0, smoothstep(edge, edge + 0.008, y));
    var col = mix(base, LID * shade, clamp(upper + lower, 0.0, 1.0) * across);
    let w = mix(0.0009, 0.0024, shut);
    let lash = (1.0 - smoothstep(w, w + aa, abs(y - edge))) * smoothstep(0.1, 0.35, shut) * across;
    return mix(col, LASH, lash);
}

// Spirals turning in her eyes when she is hypnotised (`dazed` 0..1): two arms winding into the
// middle of each painted iris, violet and pink, a glint at the heart; only between her lids.
fn spiral(base: vec3<f32>, r: vec3<f32>, centre: vec2<f32>, turn: f32, dazed: f32, t: f32, px: f32) -> vec3<f32> {
    if (dazed < 0.01) {
        return base;
    }
    let d = r.xy - centre;
    let c = cos(turn);
    let s = sin(turn);
    let x = d.x * c + d.y * s;
    let y = -d.x * s + d.y * c;
    let a = x / EYE_HALF;
    if (abs(a) > 1.15 || y > EYE_TOP * 1.3 || y < -EYE_LOW * 1.8) {
        return base;
    }
    let k = max(1.0 - a * a / 1.1, 0.0);
    let top = EYE_TOP * pow(k, 0.6);
    let low = -EYE_LOW * pow(k, 0.8);
    let aa = px * 1.2 + 0.0002;
    let open = smoothstep(low - aa, low + aa, y) * (1.0 - smoothstep(top - aa, top + aa, y));
    let q = vec2<f32>(x * sign(turn + 1e-6), y - 0.0075);
    let rad = length(q) / 0.0235;
    let ang = atan2(q.y, q.x) / 6.2831853;
    let wave = sin(6.2831853 * (ang * 2.0 + rad * 2.4 - t * 1.3));
    let stripe = smoothstep(-0.18, 0.18, wave);
    let iris = 1.0 - smoothstep(0.92, 1.08, rad);
    let col = mix(vec3<f32>(0.10, 0.01, 0.20), vec3<f32>(1.0, 0.55, 0.95), stripe);
    let core = 1.0 - smoothstep(0.0, 0.2, rad);
    let swirl = mix(col, vec3<f32>(1.0, 0.95, 1.0), core * 0.85);
    return mix(base, swirl, dazed * iris * open);
}

// The mouth parting as she talks: the jaw drops, so it opens mostly downward from the line
// between her lips; dark inside, her tongue at its foot.
fn mouth(base: vec3<f32>, r: vec3<f32>, open: f32, px: f32) -> vec3<f32> {
    if (open < 0.02) {
        return base;
    }
    let hw = 0.0112 + 0.0035 * open;
    let hh = 0.0088 * open + 0.0008;
    let d = r.xy - vec2<f32>(MOUTH.x, MOUTH.y - 0.4 * hh);
    let q = vec2<f32>(d.x / hw, d.y / hh);
    let e = length(q);
    if (e > 1.6) {
        return base;
    }
    let aa = px / hh * 1.2 + 0.03;
    let inside = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, e);
    let tongue = (1.0 - smoothstep(-0.55, -0.1, q.y)) * (1.0 - smoothstep(0.3, 0.85, abs(q.x)));
    let col = mix(vec3<f32>(0.16, 0.02, 0.06), vec3<f32>(0.62, 0.18, 0.26), tongue);
    return mix(base, col, inside);
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    // Taken in uniform control flow: her texture, and how much of her a pixel spans.
    var base = textureSample(tex, samp, i.uv).rgb;
    let px = max(fwidth(i.rest.x), fwidth(i.rest.y));
    if (u.face.x > 0.5 && i.rest.z > 0.06 && i.rest.y > 0.6 && i.rest.y < 0.76) {
        base = spiral(base, i.rest, EYE_R, -EYE_TURN, u.fx.x, u.fx.y, px);
        base = spiral(base, i.rest, EYE_L, EYE_TURN, u.fx.x, u.fx.y, px);
        base = lid(base, i.rest, EYE_R, -EYE_TURN, u.face.y, px);
        base = lid(base, i.rest, EYE_L, EYE_TURN, u.face.y, px);
        base = mouth(base, i.rest, u.face.z, px);
    }
    let n = normalize(i.n);
    let l = normalize(u.light.xyz);
    let view = normalize(-i.p);
    let lit = smoothstep(-0.06, 0.16, dot(n, l));
    let key = vec3<f32>(1.06, 1.0, 0.96);
    let shade = mix(u.look.rgb, key, lit);
    // A soft fill from below so her shadow side keeps its colour.
    let fill = max(dot(n, vec3<f32>(0.2, -0.6, 0.5)), 0.0) * vec3<f32>(0.10, 0.06, 0.16);
    let rim = pow(1.0 - max(dot(n, view), 0.0), 3.0) * u.look.a;
    // A glint on the hair, cut sharp as painted hair has it.
    let glint = smoothstep(0.88, 0.93, dot(n, normalize(l + view))) * u.face.w;
    let rgb = base * (shade + fill) + vec3<f32>(0.95, 0.6, 1.0) * rim + vec3<f32>(1.0, 0.97, 1.0) * glint;
    return vec4<f32>(rgb, 1.0);
}

@fragment
fn fs_ink() -> @location(0) vec4<f32> {
    return vec4<f32>(u.ink.rgb, 1.0);
}
"#;

/// A texture's mip chain (a box filter, in the texture's own encoding).
fn mips(width: u32, height: u32, rgba: &[u8]) -> Vec<(u32, u32, Vec<u8>)> {
    let mut out = vec![(width, height, rgba.to_vec())];
    while let Some((w, h, data)) = out.last() {
        if *w <= 1 && *h <= 1 {
            break;
        }
        let (nw, nh) = ((*w / 2).max(1), (*h / 2).max(1));
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(*w - 1);
                        let sy = (y * 2 + dy).min(*h - 1);
                        sum += data[((sy * *w + sx) * 4 + c) as usize] as u32;
                    }
                    next[((y * nw + x) * 4 + c) as usize] = (sum / 4) as u8;
                }
            }
        }
        out.push((nw, nh, next));
    }
    out
}

fn target(
    device: &wgpu::Device,
    label: &str,
    format: wgpu::TextureFormat,
    samples: u32,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

/// One texture of hers, uploaded with its mips, as the parts bind it.
fn texture_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    image: Option<&velvet_render::TextureData3D>,
) -> wgpu::BindGroup {
    let (tw, th, pixels) = match image {
        Some(t) => (t.width.max(1), t.height.max(1), t.rgba.clone()),
        None => (1, 1, vec![255, 255, 255, 255]),
    };
    let chain = mips(tw, th, &pixels);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mascot-texture"),
        size: wgpu::Extent3d {
            width: tw,
            height: th,
            depth_or_array_layers: 1,
        },
        mip_level_count: chain.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (w, h, data)) in chain.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(*h),
            },
            wgpu::Extent3d {
                width: *w,
                height: *h,
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mascot-texture-group"),
        layout,
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
    })
}

impl Mascot {
    /// Her model and her renderer (`None` when the model cannot be read).
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        picture_layout: &wgpu::BindGroupLayout,
    ) -> Option<Self> {
        let path = crate::exploration3d_state::data_dir().join("assets/3d/characters/chiquita.glb");
        let model = match GltfModel::load(&path) {
            Ok(model) => model,
            Err(error) => {
                eprintln!("[mascot] {}: {error}", path.display());
                return None;
            }
        };
        let clips: Vec<Option<usize>> = CLIPS.iter().map(|c| model.animation_index(c)).collect();
        let hair = HairSim::new(&model);
        eprintln!(
            "[mascot] chiquita: {} meshes, clips {:?}, hair {}",
            model.meshes.len(),
            clips,
            if hair.is_some() { "rigged" } else { "none" }
        );

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mascot-uniform-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mascot-texture-layout"),
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
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mascot-sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 4,
            ..Default::default()
        });

        // Her skinned meshes, each with its own texture.
        let mut parts = Vec::new();
        for (index, mesh) in model.meshes.iter().enumerate() {
            let Some(skin) = mesh.skinning.as_ref() else {
                continue;
            };
            let vertices: Vec<Vertex> = mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(k, v)| {
                    let j = skin.joints.get(k).copied().unwrap_or([0; 4]);
                    let w = skin.weights.get(k).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
                    let sum = (w[0] + w[1] + w[2] + w[3]).max(1e-6);
                    Vertex {
                        position: v.position,
                        normal: v.normal,
                        uv: v.uv,
                        joints: [j[0] as u32, j[1] as u32, j[2] as u32, j[3] as u32],
                        weights: [w[0] / sum, w[1] / sum, w[2] / sum, w[3] / sum],
                    }
                })
                .collect();
            let joint_count = model
                .joint_matrices(index, &model.rest_pose())
                .map_or(1, |j| j.len().max(1));
            let vbuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mascot-vertices"),
                size: (vertices.len() * std::mem::size_of::<Vertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vbuf, 0, bytemuck::cast_slice(&vertices));
            let ibuf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mascot-indices"),
                size: (mesh.indices.len() * 4).max(4) as u64,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&ibuf, 0, bytemuck::cast_slice(&mesh.indices));
            let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mascot-uniforms"),
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let joints = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mascot-joints"),
                size: (joint_count * 64) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mascot-group"),
                layout: &uniform_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniforms.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: joints.as_entire_binding(),
                    },
                ],
            });
            let image = mesh.texture_index.and_then(|i| model.textures.get(i));
            let texture = texture_group(device, queue, &texture_layout, &sampler, image);
            let name = model
                .mesh_names
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            parts.push(Part {
                mesh: index,
                vertices: vbuf,
                indices: ibuf,
                count: mesh.indices.len() as u32,
                uniforms,
                joints,
                group,
                texture,
                body: !name.contains("Hair"),
            });
        }
        if parts.is_empty() {
            eprintln!("[mascot] chiquita.glb has no skinned mesh");
            return None;
        }

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mascot-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mascot-layout"),
            bind_group_layouts: &[&uniform_layout, &texture_layout],
            push_constant_ranges: &[],
        });
        let buffers = [wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x2,
                3 => Uint32x4,
                4 => Float32x4
            ],
        }];
        let pipeline_for = |label: &str, vs: &str, fs: &str, cull: wgpu::Face| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    buffers: &buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(cull),
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: SAMPLES,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        };
        let pipeline = pipeline_for("mascot-pipeline", "vs_main", "fs_main", wgpu::Face::Back);
        let outline = pipeline_for("mascot-ink", "vs_outline", "fs_ink", wgpu::Face::Front);

        let msaa = target(
            device,
            "mascot-msaa",
            FORMAT,
            SAMPLES,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let depth = target(
            device,
            "mascot-depth",
            wgpu::TextureFormat::Depth32Float,
            SAMPLES,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
        .create_view(&wgpu::TextureViewDescriptor::default());
        let picture_texture = target(
            device,
            "mascot-picture",
            FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        );
        let picture = picture_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let picture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mascot-picture-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let picture_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mascot-picture-group"),
            layout: picture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&picture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&picture_sampler),
                },
            ],
        });
        Some(Self {
            model,
            clips,
            parts,
            hair,
            pipeline,
            outline,
            msaa,
            depth,
            _picture: picture,
            picture_texture,
            picture_group,
            last: None,
        })
    }

    /// Her last picture read back (sRGB bytes, premultiplied by her coverage): her portraits are
    /// cut from it.
    pub(crate) fn snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let rgba = velvet_render::read_texture_rgba(device, queue, &self.picture_texture).ok()?;
        Some((W, H, rgba))
    }

    /// Her picture (premultiplied by her coverage): what is laid over the screen.
    pub(crate) fn picture(&self) -> &wgpu::TextureView {
        &self._picture
    }

    /// Her picture for `view` (redrawn only when she has changed).
    pub(crate) fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, view: &MascotView) {
        // The glass she broke is the overlay's to draw: only her pose and place count here.
        let key = MascotView {
            crack: None,
            ..view.clone()
        };
        if self.last.as_ref() == Some(&key) {
            return;
        }
        self.last = Some(key);
        let clip = |k: u8| {
            self.clips
                .get(k as usize)
                .copied()
                .flatten()
                .or(self.clips[0])
        };
        let Some(current) =
            clip(view.clip).and_then(|c| self.model.pose(c, view.time_ms as f32 / 1000.0))
        else {
            return;
        };
        let blend = view.blend.min(1000) as f32 / 1000.0;
        let mut pose = if blend < 0.999 {
            match clip(view.prev_clip)
                .and_then(|c| self.model.pose(c, view.prev_time_ms as f32 / 1000.0))
            {
                Some(previous) => previous.blend(&current, blend),
                None => current,
            }
        } else {
            current
        };
        let deg = |t: i16| (t as f32 / 10.0).to_radians();
        // Her hair, swung after the clip.
        if let Some(hair) = self.hair.as_mut() {
            let place = (
                [view.feet[0] as f32, view.feet[1] as f32],
                view.height as f32 * view.squash[1].max(1) as f32 / 1000.0,
                deg(view.yaw),
            );
            hair.step(&self.model, &mut pose, view.clock_ms, place);
        }
        // Where she stands, in the camera's space.
        let aspect = W as f32 / H as f32;
        let tan = (FOV * 0.5).tan();
        let ndc_x = view.feet[0] as f32 / W as f32 * 2.0 - 1.0;
        let ndc_y = 1.0 - view.feet[1] as f32 / H as f32 * 2.0;
        let x = ndc_x * DEPTH * tan * aspect;
        let y = ndc_y * DEPTH * tan;
        let tall = view.height as f32 / H as f32 * 2.0 * DEPTH * tan;
        let sq = |k: usize| view.squash[k].max(1) as f32 / 1000.0;
        let place = Mat4::from_translation(Vec3::new(x, y, -DEPTH))
            * Mat4::from_rotation_y(deg(view.yaw))
            * Mat4::from_rotation_x(deg(view.pitch))
            * Mat4::from_rotation_z(deg(view.roll))
            * Mat4::from_scale(Vec3::new(tall * sq(0), tall * sq(1), tall * sq(2)));
        let projection = Mat4::perspective_rh(FOV, aspect, 0.1, 60.0);
        let ink_px = 2.3;
        let light = Vec3::new(-0.45, 0.72, 0.55).normalize_or_zero();
        let blink = view.blink.min(1000) as f32 / 1000.0;
        let mouth = view.mouth.min(1000) as f32 / 1000.0;
        let fx = [
            view.dazed.min(1000) as f32 / 1000.0,
            view.clock_ms as f32 / 1000.0,
            0.0,
            0.0,
        ];
        for part in &self.parts {
            let mesh_transform = self.model.mesh_transform(part.mesh);
            let model = place * mesh_transform;
            let uniforms = Uniforms {
                mvp: (projection * model).to_cols_array(),
                model: model.to_cols_array(),
                light: [
                    light.x,
                    light.y,
                    light.z,
                    ink_px / view.height.max(1) as f32,
                ],
                ink: [0.07, 0.025, 0.11, 1.0],
                look: [0.52, 0.42, 0.72, if part.body { 0.42 } else { 0.5 }],
                face: if part.body {
                    [1.0, blink, mouth, 0.0]
                } else {
                    [0.0, 0.0, 0.0, 0.22]
                },
                fx,
            };
            queue.write_buffer(&part.uniforms, 0, bytemuck::bytes_of(&uniforms));
            if let Some(joints) = self.model.joint_matrices(part.mesh, &pose) {
                let flat: Vec<[f32; 16]> = joints.iter().map(|m| m.to_cols_array()).collect();
                queue.write_buffer(&part.joints, 0, bytemuck::cast_slice(&flat));
            }
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mascot-encoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mascot-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa,
                    resolve_target: Some(&self._picture),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for part in &self.parts {
                pass.set_bind_group(0, &part.group, &[]);
                pass.set_bind_group(1, &part.texture, &[]);
                pass.set_vertex_buffer(0, part.vertices.slice(..));
                pass.set_index_buffer(part.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_pipeline(&self.outline);
                pass.draw_indexed(0..part.count, 0, 0..1);
                pass.set_pipeline(&self.pipeline);
                pass.draw_indexed(0..part.count, 0, 0..1);
            }
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

/// The quad that lays her picture over the screen, 1:1 (shader mode 14 unpremultiplies it).
pub(super) fn quad(v: &mut Vec<MapVertex>, width: f32, height: f32, alpha: f32) {
    let corner = |x: f32, y: f32| MapVertex {
        position: [x * width, y * height],
        uv: [x, y],
        color: [1.0, 1.0, 1.0, alpha],
        text: 14.0,
        ..MapVertex::BLANK
    };
    let (a, b, c, d) = (
        corner(0.0, 0.0),
        corner(1.0, 0.0),
        corner(1.0, 1.0),
        corner(0.0, 1.0),
    );
    v.extend_from_slice(&[a, b, c, a, c, d]);
}
