//! Water drawn the way the rest of the world is painted: each body of water has a look
//! ([`WaterLook3D`]) designed for its place — a stream carries dashes of light and foams at its
//! banks, a hot spring is milky with bubbles breaking, a still pond is a dark mirror for the sky
//! and the lamps, a well holds a patch of sky, a tub sloshes light. The surface is flat at the
//! water's level and every motion on it is drawn from closed-form patterns, so nothing in it can
//! grow out of hand.
//!
//! What touches the water (a step, a drop, a bubble, a paddle) leaves rings that spread and fade
//! ([`Water3D::touch`], at most [`RINGS`] at once). Splashes are drops drawn along their own arcs
//! (thrown, falling under gravity, gone with a small ring where they meet the water): nothing in
//! the water is simulated.
//!
//! Lights round the water shine in it as broken columns of glitter under them (where their mirror
//! image falls for the camera); the moon lays a path on still water at night.

use bytemuck::{Pod, Zeroable};
use velvet_math::Vec3;

/// Splash particles alive at most at once.
const PARTICLES: usize = 512;
/// Splashes and touches the height fields take per frame.
const SPLASHES: usize = 256;
/// The water step's fixed time step.
const STEP: f32 = 1.0 / 120.0;
/// Rings on the water at most at once (drawn, not simulated).
const RINGS: usize = 32;
/// How long a ring lives (seconds); kept in step with the shader's `ring_lines`.
const RING_LIFE: f32 = 2.6;

/// What kind of water a body is: each is drawn in its own way (toon, designed, no simulation
/// that could break).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaterLook3D {
    /// A running stream: dashes of light carried down it, foam at its banks.
    Brook,
    /// A hot spring: milky, bubbles breaking, a caustic net in the shallows.
    Spring,
    /// A still pool outdoors: a dark mirror for the sky and the lights round it.
    #[default]
    Pond,
    /// Down a well: dark, a patch of sky caught on it.
    Well,
    /// A tub indoors: clear, a slow slosh of light.
    Tub,
}

/// How a body of water looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterStyle3D {
    /// Its kind (how it moves and what it reflects).
    pub look: WaterLook3D,
    /// Linear colour of shallow water (over the bed).
    pub shallow: [f32; 3],
    /// Linear colour of deep water.
    pub deep: [f32; 3],
    /// Depth (metres) at which the water reaches its deep colour.
    pub deep_at: f32,
    /// How much the water lets the bed show through at its shallowest (0..1).
    pub clarity: f32,
    /// How readily it foams (0 never).
    pub foam: f32,
}

impl Default for WaterStyle3D {
    fn default() -> Self {
        Self {
            look: WaterLook3D::Pond,
            shallow: [0.10, 0.32, 0.30],
            deep: [0.012, 0.06, 0.075],
            deep_at: 0.9,
            clarity: 0.45,
            foam: 1.0,
        }
    }
}

/// A body of water to simulate: a grid over its surface.
#[derive(Debug, Clone)]
pub struct WaterBody3D {
    /// World x and z of the grid's first cell's corner.
    pub origin: [f32; 2],
    /// Cell size in metres.
    pub cell: f32,
    /// Cells across (x) and along (z).
    pub size: (u32, u32),
    /// World height of the still surface.
    pub level: f32,
    /// Per cell, 1 for water, 0 for bank.
    pub mask: Vec<f32>,
    /// Per cell, how deep the water is (metres).
    pub depth: Vec<f32>,
    /// Per cell, the velocity (m/s, x and z) the stream is driven towards; `None` for standing
    /// water.
    pub flow: Option<Vec<[f32; 2]>>,
    /// Speed of its ripples (m/s): sqrt(g·depth) for real shallow water.
    pub wave_speed: f32,
    /// How fast ripples die away (per second).
    pub damping: f32,
    /// Its look.
    pub style: WaterStyle3D,
}

impl WaterBody3D {
    /// A body covering the given world triangles (their x and z; the surface lies at their
    /// mean height), on a grid of `cell` metres. Its depth grows from the banks inward up to
    /// `max_depth`.
    pub fn from_triangles(triangles: &[[Vec3; 3]], cell: f32, max_depth: f32) -> Option<Self> {
        if triangles.is_empty() || cell <= 0.0 {
            return None;
        }
        let (mut x0, mut z0, mut x1, mut z1, mut y) = (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
            0.0,
        );
        for t in triangles {
            for p in t {
                x0 = x0.min(p.x);
                z0 = z0.min(p.z);
                x1 = x1.max(p.x);
                z1 = z1.max(p.z);
                y += p.y;
            }
        }
        let level = y / (triangles.len() * 3) as f32;
        let margin = cell * 2.0;
        let origin = [x0 - margin, z0 - margin];
        let w = (((x1 - x0) + 2.0 * margin) / cell).ceil() as u32 + 1;
        let h = (((z1 - z0) + 2.0 * margin) / cell).ceil() as u32 + 1;
        let (w, h) = (w.max(4), h.max(4));
        let mut mask = vec![0.0f32; (w * h) as usize];
        let edge = |a: [f32; 2], b: [f32; 2], p: [f32; 2]| {
            (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        };
        for t in triangles {
            let v = t.map(|p| [p.x, p.z]);
            let area = edge(v[0], v[1], v[2]);
            if area.abs() < 1e-9 {
                continue;
            }
            let lo_x = v.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let hi_x = v.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
            let lo_z = v.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let hi_z = v.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
            let i0 = (((lo_x - origin[0]) / cell).floor().max(0.0)) as u32;
            let i1 = (((hi_x - origin[0]) / cell).ceil() as u32).min(w - 1);
            let j0 = (((lo_z - origin[1]) / cell).floor().max(0.0)) as u32;
            let j1 = (((hi_z - origin[1]) / cell).ceil() as u32).min(h - 1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let p = [
                        origin[0] + (i as f32 + 0.5) * cell,
                        origin[1] + (j as f32 + 0.5) * cell,
                    ];
                    let (a, b, c) = (
                        edge(v[0], v[1], p),
                        edge(v[1], v[2], p),
                        edge(v[2], v[0], p),
                    );
                    let inside = if area > 0.0 {
                        a >= 0.0 && b >= 0.0 && c >= 0.0
                    } else {
                        a <= 0.0 && b <= 0.0 && c <= 0.0
                    };
                    if inside {
                        mask[(j * w + i) as usize] = 1.0;
                    }
                }
            }
        }
        let (wu, hu) = (w as usize, h as usize);
        let hard = mask.clone();
        for _ in 0..2 {
            let src = mask.clone();
            for j in 0..hu {
                for i in 0..wu {
                    let mut sum = 0.0;
                    for dj in -1i32..=1 {
                        for di in -1i32..=1 {
                            let (x, y) = (
                                (i as i32 + di).clamp(0, wu as i32 - 1) as usize,
                                (j as i32 + dj).clamp(0, hu as i32 - 1) as usize,
                            );
                            sum += src[y * wu + x];
                        }
                    }
                    mask[j * wu + i] = sum / 9.0;
                }
            }
        }
        let mask_hard = hard;
        // Depth from the distance to the bank (a two-pass chamfer distance in cells).
        let mut dist: Vec<f32> = mask_hard
            .iter()
            .map(|m| if *m > 0.5 { 1e6 } else { 0.0 })
            .collect();
        let diag = std::f32::consts::SQRT_2;
        for j in 0..hu {
            for i in 0..wu {
                let k = j * wu + i;
                let mut d = dist[k];
                if i > 0 {
                    d = d.min(dist[k - 1] + 1.0);
                }
                if j > 0 {
                    d = d.min(dist[k - wu] + 1.0);
                }
                if i > 0 && j > 0 {
                    d = d.min(dist[k - wu - 1] + diag);
                }
                if i + 1 < wu && j > 0 {
                    d = d.min(dist[k - wu + 1] + diag);
                }
                dist[k] = d;
            }
        }
        for j in (0..hu).rev() {
            for i in (0..wu).rev() {
                let k = j * wu + i;
                let mut d = dist[k];
                if i + 1 < wu {
                    d = d.min(dist[k + 1] + 1.0);
                }
                if j + 1 < hu {
                    d = d.min(dist[k + wu] + 1.0);
                }
                if i + 1 < wu && j + 1 < hu {
                    d = d.min(dist[k + wu + 1] + diag);
                }
                if i > 0 && j + 1 < hu {
                    d = d.min(dist[k + wu - 1] + diag);
                }
                dist[k] = d;
            }
        }
        let widest = dist.iter().copied().fold(0.0f32, f32::max).max(1.0);
        let depth = dist
            .iter()
            .map(|d| {
                let t = (d / widest).clamp(0.0, 1.0);
                max_depth * (t * (2.0 - t)).sqrt().max(0.0)
            })
            .collect();
        let deepest = max_depth.max(0.05);
        Some(Self {
            origin,
            cell,
            size: (w, h),
            level,
            mask,
            depth,
            flow: None,
            wave_speed: (9.81 * deepest * 0.5).sqrt().clamp(0.5, 1.6),
            damping: 0.9,
            style: WaterStyle3D::default(),
        })
    }

    /// Makes it a stream running along `course` (world x, z; from source to mouth) at `speed`
    /// m/s, slower by the banks.
    pub fn with_flow_along(mut self, course: &[[f32; 2]], speed: f32) -> Self {
        if course.len() < 2 {
            return self;
        }
        let (w, h) = (self.size.0 as usize, self.size.1 as usize);
        let deepest = self.depth.iter().copied().fold(0.0f32, f32::max).max(1e-3);
        let mut flow = vec![[0.0f32; 2]; w * h];
        for j in 0..h {
            for i in 0..w {
                let k = j * w + i;
                if self.mask[k] < 0.5 {
                    continue;
                }
                let p = [
                    self.origin[0] + (i as f32 + 0.5) * self.cell,
                    self.origin[1] + (j as f32 + 0.5) * self.cell,
                ];
                let mut best = (f32::INFINITY, [0.0f32; 2]);
                for s in course.windows(2) {
                    let (a, b) = (s[0], s[1]);
                    let d = [b[0] - a[0], b[1] - a[1]];
                    let len2 = (d[0] * d[0] + d[1] * d[1]).max(1e-9);
                    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / len2).clamp(0.0, 1.0);
                    let q = [a[0] + d[0] * t - p[0], a[1] + d[1] * t - p[1]];
                    let dist = q[0] * q[0] + q[1] * q[1];
                    if dist < best.0 {
                        let len = len2.sqrt();
                        best = (dist, [d[0] / len, d[1] / len]);
                    }
                }
                let bank = (self.depth[k] / deepest).clamp(0.0, 1.0);
                let v = speed * (0.35 + 0.65 * bank);
                flow[k] = [best.1[0] * v, best.1[1] * v];
            }
        }
        self.flow = Some(flow);
        self.damping = 0.6;
        self
    }

    /// The world point's cell, if it lies on the grid.
    fn cell_of(&self, x: f32, z: f32) -> Option<(u32, u32)> {
        let i = ((x - self.origin[0]) / self.cell).floor();
        let j = ((z - self.origin[1]) / self.cell).floor();
        (i >= 0.0 && j >= 0.0 && (i as u32) < self.size.0 && (j as u32) < self.size.1)
            .then_some((i as u32, j as u32))
    }

    /// The nearest water to (x, z) within `radius`, if any.
    pub fn nearest_water(&self, x: f32, z: f32, radius: f32) -> Option<[f32; 2]> {
        let r = (radius / self.cell).ceil() as i32;
        let (ci, cj) = (
            ((x - self.origin[0]) / self.cell).floor() as i32,
            ((z - self.origin[1]) / self.cell).floor() as i32,
        );
        let mut best: Option<(f32, [f32; 2])> = None;
        for dj in -r..=r {
            for di in -r..=r {
                let (i, j) = (ci + di, cj + dj);
                if i < 0 || j < 0 || i >= self.size.0 as i32 || j >= self.size.1 as i32 {
                    continue;
                }
                if self.mask[(j as u32 * self.size.0 + i as u32) as usize] < 0.5 {
                    continue;
                }
                let p = [
                    self.origin[0] + (i as f32 + 0.5) * self.cell,
                    self.origin[1] + (j as f32 + 0.5) * self.cell,
                ];
                let d = (p[0] - x).hypot(p[1] - z);
                if d <= radius && best.map_or(true, |b| d < b.0) {
                    best = Some((d, p));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// Whether (x, z) is over water.
    pub fn contains(&self, x: f32, z: f32) -> bool {
        self.cell_of(x, z)
            .is_some_and(|(i, j)| self.mask[(j * self.size.0 + i) as usize] > 0.5)
    }
}

/// A body of water registered with the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WaterBodyHandle(pub(crate) usize);

/// Running or falling water registered with the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WaterStreamHandle(pub(crate) usize);

/// Water that runs and falls (channels, cascades, waterfalls) as a ribbon of world-space
/// vertices: `uv` is (across 0..1, along in metres downstream) and the colour carries, per
/// vertex, how steeply it falls (r, 0..1), how much it froths (g, 0..1) and its width / 4 (b).
#[derive(Debug, Clone)]
pub struct WaterStream3D {
    /// The ribbon's vertices, in the world.
    pub vertices: Vec<crate::MeshVertex3D>,
    /// Its triangles.
    pub indices: Vec<u32>,
    /// Its colours (shallow down its sides, deep in its middle) and how readily it foams.
    pub style: WaterStyle3D,
    /// How fast it runs (m/s, roughly).
    pub speed: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct StreamUniform {
    /// shallow rgb, speed
    shallow: [f32; 4],
    /// deep rgb, foam
    deep: [f32; 4],
    /// time, unused ×3
    misc: [f32; 4],
}

struct GpuStream {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
    style: WaterStyle3D,
    speed: f32,
    visible: bool,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BodyUniform {
    /// origin x, origin z, cell, level
    grid: [f32; 4],
    /// width, height, wave speed, damping
    dims: [f32; 4],
    /// flowing (1/0), flow pull per second, time, step dt
    flow: [f32; 4],
    /// shallow rgb, deep_at
    shallow: [f32; 4],
    /// deep rgb, clarity
    deep: [f32; 4],
    /// foam, splash share per step, unused, unused
    misc: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Particle {
    /// xyz, life (seconds; dead at 0)
    pos: [f32; 4],
    /// xyz, density
    vel: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SphUniform {
    /// particles, dt, gravity, bodies
    counts: [f32; 4],
    /// smoothing radius, particle mass, rest density, stiffness
    kernel: [f32; 4],
    /// viscosity, restitution, max speed, unused
    visc: [f32; 4],
    /// per body: [x0, z0, x1, z1] then [level, 0, 0, 0]
    bodies: [[f32; 4]; 16],
}

struct GpuBody {
    desc: WaterBody3D,
    uniform: wgpu::Buffer,
    render: wgpu::BindGroup,
    visible: bool,
}

/// The renderer's water: bodies, droplets, their solvers and their drawing.
pub(crate) struct Water3D {
    bodies: Vec<GpuBody>,
    streams: Vec<GpuStream>,
    render_layout: wgpu::BindGroupLayout,
    stream_layout: wgpu::BindGroupLayout,
    stream_pipeline: wgpu::RenderPipeline,
    /// The scene mirrored in the water, bound at group 2 while drawing it.
    reflection: Option<std::sync::Arc<wgpu::BindGroup>>,
    sph_density: wgpu::ComputePipeline,
    sph_forces: wgpu::ComputePipeline,
    surface: wgpu::RenderPipeline,
    droplets: wgpu::RenderPipeline,
    splashes: wgpu::Buffer,
    /// The rings on the water: (x, z, born at, strength 0..1).
    rings: wgpu::Buffer,
    ring_list: Vec<[f32; 4]>,
    sph_uniform: wgpu::Buffer,
    particles: [wgpu::Buffer; 2],
    sph_groups: [wgpu::BindGroup; 2],
    droplet_groups: [wgpu::BindGroup; 2],
    particle_parity: usize,
    #[allow(dead_code)]
    next_particle: usize,
    /// Touches waiting for the next step: (x, z, radius, strength).
    pending: Vec<[f32; 4]>,
    /// Droplets waiting to be born (unused: drops follow `arcs`).
    #[allow(dead_code)]
    births: Vec<(usize, Particle)>,
    /// The drops in the air, each on its own arc.
    arcs: Vec<Arc>,
    time: f32,
    dt: f32,
    any_alive_until: f32,
}

/// A drop thrown from `from` at `velocity` at time `born`, for `life` seconds at most.
#[derive(Clone, Copy)]
struct Arc {
    from: Vec3,
    velocity: Vec3,
    born: f32,
    life: f32,
}

fn storage_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl Water3D {
    pub(crate) fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        reflection_layout: &wgpu::BindGroupLayout,
        hdr_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> Self {
        let cs = wgpu::ShaderStages::COMPUTE;
        let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let render_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-water-render-layout"),
            entries: &[
                uniform_entry(0, vf),
                storage_entry(2, vf, true),
                uniform_entry(3, vf),
            ],
        });
        let sph_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-water-sph-layout"),
            entries: &[
                uniform_entry(0, cs),
                storage_entry(1, cs, true),
                storage_entry(2, cs, false),
                storage_entry(3, cs, false),
            ],
        });
        let droplet_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-water-droplet-layout"),
            entries: &[storage_entry(4, vf, true)],
        });
        let stream_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-water-stream-layout"),
            entries: &[uniform_entry(3, vf), uniform_entry(5, vf)],
        });
        let sph = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-water-sph"),
            source: wgpu::ShaderSource::Wgsl(SPH_SHADER.into()),
        });
        let draw = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-water-draw"),
            source: wgpu::ShaderSource::Wgsl(format!("{CAMERA_WGSL}{DRAW_SHADER}").into()),
        });
        let compute = |layout: &wgpu::BindGroupLayout, module: &wgpu::ShaderModule, entry: &str| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(entry),
                bind_group_layouts: &[layout],
                push_constant_ranges: &[],
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let render = |layout: &wgpu::BindGroupLayout,
                      vs: &str,
                      fs: &str,
                      write_depth: bool,
                      buffers: &[wgpu::VertexBufferLayout<'_>]| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(vs),
                bind_group_layouts: &[camera_layout, layout, reflection_layout],
                push_constant_ranges: &[],
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(fs),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &draw,
                    entry_point: Some(vs),
                    buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &draw,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: hdr_format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    // The surface writes depth: the ink then outlines the water's edge, not
                    // whatever lies under it.
                    depth_write_enabled: write_depth,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        };
        let rings = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-water-rings"),
            size: (16 + RINGS * 16) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let splashes = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-water-splashes"),
            size: (16 + SPLASHES * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sph_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-water-sph-uniform"),
            size: std::mem::size_of::<SphUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let particle_buffer = |label| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (PARTICLES * std::mem::size_of::<Particle>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let particles = [
            particle_buffer("velvet-water-particles-a"),
            particle_buffer("velvet-water-particles-b"),
        ];
        let sph_groups = [0usize, 1].map(|k| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("velvet-water-sph-group"),
                layout: &sph_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sph_uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particles[k].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particles[1 - k].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: splashes.as_entire_binding(),
                    },
                ],
            })
        });
        let droplet_groups = [0usize, 1].map(|k| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("velvet-water-droplet-group"),
                layout: &droplet_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 4,
                    resource: particles[k].as_entire_binding(),
                }],
            })
        });
        Self {
            bodies: Vec::new(),
            sph_density: compute(&sph_layout, &sph, "cs_density"),
            sph_forces: compute(&sph_layout, &sph, "cs_forces"),
            surface: render(&render_layout, "vs_surface", "fs_surface", true, &[]),
            droplets: render(&droplet_layout, "vs_droplet", "fs_droplet", false, &[]),
            stream_pipeline: render(
                &stream_layout,
                "vs_stream",
                "fs_stream",
                true,
                &[stream_vertex_layout()],
            ),
            streams: Vec::new(),
            reflection: None,
            stream_layout,
            render_layout,
            splashes,
            rings,
            ring_list: Vec::new(),
            sph_uniform,
            particles,
            sph_groups,
            droplet_groups,
            particle_parity: 0,
            next_particle: 0,
            pending: Vec::new(),
            births: Vec::new(),
            arcs: Vec::new(),
            time: 0.0,
            dt: 0.0,
            any_alive_until: 0.0,
        }
    }

    pub(crate) fn add_body(&mut self, device: &wgpu::Device, desc: WaterBody3D) -> WaterBodyHandle {
        use wgpu::util::DeviceExt;
        let cells = (desc.size.0 * desc.size.1) as usize;
        let statics: Vec<[f32; 4]> = (0..cells)
            .map(|k| {
                let f = desc.flow.as_ref().map(|f| f[k]).unwrap_or([0.0, 0.0]);
                [desc.mask[k], desc.depth[k], f[0], f[1]]
            })
            .collect();
        let statics = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-water-statics"),
            contents: bytemuck::cast_slice(&statics),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-water-body"),
            size: std::mem::size_of::<BodyUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let render = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-water-render-group"),
            layout: &self.render_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: statics.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.rings.as_entire_binding(),
                },
            ],
        });
        self.bodies.push(GpuBody {
            desc,
            uniform,
            render,
            visible: false,
        });
        WaterBodyHandle(self.bodies.len() - 1)
    }

    pub(crate) fn add_stream(
        &mut self,
        device: &wgpu::Device,
        desc: WaterStream3D,
    ) -> WaterStreamHandle {
        use wgpu::util::DeviceExt;
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-water-stream-vertices"),
            contents: bytemuck::cast_slice(&desc.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-water-stream-indices"),
            contents: bytemuck::cast_slice(&desc.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-water-stream"),
            size: std::mem::size_of::<StreamUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-water-stream-group"),
            layout: &self.stream_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: self.rings.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        self.streams.push(GpuStream {
            vertices,
            indices,
            count: desc.indices.len() as u32,
            uniform,
            group,
            style: desc.style,
            speed: desc.speed,
            visible: false,
        });
        WaterStreamHandle(self.streams.len() - 1)
    }

    pub(crate) fn set_stream_visible(&mut self, handle: WaterStreamHandle, visible: bool) {
        if let Some(s) = self.streams.get_mut(handle.0) {
            s.visible = visible;
        }
    }

    pub(crate) fn set_visible(&mut self, handle: WaterBodyHandle, visible: bool) {
        if let Some(b) = self.bodies.get_mut(handle.0) {
            b.visible = visible;
        }
    }

    pub(crate) fn body(&self, handle: WaterBodyHandle) -> Option<&WaterBody3D> {
        self.bodies.get(handle.0).map(|b| &b.desc)
    }

    pub(crate) fn any_visible(&self) -> bool {
        self.bodies.iter().any(|b| b.visible) || self.streams.iter().any(|s| s.visible)
    }

    pub(crate) fn touch(&mut self, x: f32, z: f32, radius: f32, strength: f32) {
        let _ = radius;
        let now = self.time;
        self.ring_list.retain(|r| now - r[2] < RING_LIFE);
        // A ring already starting there takes this touch.
        if self
            .ring_list
            .iter()
            .any(|r| (r[0] - x).hypot(r[1] - z) < 0.35 && now - r[2] < 0.3)
        {
            return;
        }
        if self.ring_list.len() >= RINGS {
            self.ring_list.remove(0);
        }
        self.ring_list
            .push([x, z, now, (strength * 140.0).clamp(0.25, 1.0)]);
    }

    /// Throws a drop: drawn along its own arc (no simulation), gone where it meets the water,
    /// leaving a small ring there.
    pub(crate) fn emit(&mut self, position: Vec3, velocity: Vec3, life: f32) {
        if self.arcs.len() >= PARTICLES {
            self.arcs.remove(0);
        }
        self.arcs.push(Arc {
            from: position,
            velocity,
            born: self.time,
            life: life.max(0.05),
        });
        self.any_alive_until = self.any_alive_until.max(self.time + life + 0.1);
    }

    /// The level of the visible water under (x, z), if any.
    fn level_under(&self, x: f32, z: f32) -> Option<f32> {
        self.bodies.iter().filter(|b| b.visible).find_map(|b| {
            let d = &b.desc;
            let inside = x >= d.origin[0]
                && z >= d.origin[1]
                && x <= d.origin[0] + d.size.0 as f32 * d.cell
                && z <= d.origin[1] + d.size.1 as f32 * d.cell;
            inside.then_some(d.level)
        })
    }

    pub(crate) fn advance(&mut self, dt: f32) {
        self.dt += dt.clamp(0.0, 0.1);
    }

    /// Runs the solvers for the time advanced since the last step.
    pub(crate) fn step(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder) {
        let dt = std::mem::take(&mut self.dt).min(1.0 / 20.0);
        let steps = ((dt / STEP).round() as u32).clamp(0, 6);
        if !self.any_visible() {
            self.pending.clear();
            self.births.clear();
            self.arcs.clear();
            self.ring_list.clear();
            return;
        }
        // The rings, for the surfaces to draw.
        {
            let now = self.time + dt;
            self.ring_list.retain(|r| now - r[2] < RING_LIFE);
            let mut data = vec![[0.0f32; 4]; RINGS + 1];
            data[0] = [self.ring_list.len() as f32, now, 0.0, 0.0];
            for (i, r) in self.ring_list.iter().enumerate() {
                data[i + 1] = *r;
            }
            queue.write_buffer(&self.rings, 0, bytemuck::cast_slice(&data));
        }
        let step_dt = if steps > 0 { dt / steps as f32 } else { 0.0 };
        self.time += dt;
        // The drops along their arcs (p = p0 + v t + g t²/2): those back at the water are gone, a
        // little ring where each fell.
        if !self.arcs.is_empty() || self.time < self.any_alive_until {
            let now = self.time;
            let mut landed = Vec::new();
            let mut kept = Vec::with_capacity(self.arcs.len());
            let mut data = vec![
                Particle {
                    pos: [0.0; 4],
                    vel: [0.0; 4]
                };
                PARTICLES
            ];
            for a in std::mem::take(&mut self.arcs) {
                let t = now - a.born;
                let p = a.from + a.velocity * t + Vec3::new(0.0, -4.9 * t * t, 0.0);
                let falling = a.velocity.y - 9.81 * t < 0.0;
                let floor = self.level_under(p.x, p.z).unwrap_or(a.from.y - 0.05);
                if t >= a.life || (falling && p.y <= floor) {
                    if falling && p.y <= floor + 0.05 {
                        landed.push((p.x, p.z));
                    }
                    continue;
                }
                let k = kept.len();
                if k < PARTICLES {
                    data[k] = Particle {
                        pos: [p.x, p.y, p.z, a.life - t],
                        vel: [a.velocity.x, a.velocity.y - 9.81 * t, a.velocity.z, 0.0],
                    };
                }
                kept.push(a);
            }
            self.arcs = kept;
            queue.write_buffer(
                &self.particles[self.particle_parity],
                0,
                bytemuck::cast_slice(&data),
            );
            for (x, z) in landed {
                self.touch(x, z, 0.05, 0.002);
            }
        }
        // The touches, then the droplets that land append theirs.
        let mut head = [0u32; 4];
        head[0] = self.pending.len() as u32;
        queue.write_buffer(&self.splashes, 0, bytemuck::cast_slice(&head));
        if !self.pending.is_empty() {
            queue.write_buffer(&self.splashes, 16, bytemuck::cast_slice(&self.pending));
        }
        self.pending.clear();
        // The bodies the droplets can fall into.
        let mut bodies = [[0.0f32; 4]; 16];
        let mut count = 0usize;
        for b in self.bodies.iter().filter(|b| b.visible).take(8) {
            let d = &b.desc;
            bodies[count * 2] = [
                d.origin[0],
                d.origin[1],
                d.origin[0] + d.size.0 as f32 * d.cell,
                d.origin[1] + d.size.1 as f32 * d.cell,
            ];
            bodies[count * 2 + 1] = [d.level, 0.0, 0.0, 0.0];
            count += 1;
        }
        // (The droplets are drawn on their arcs above: no solver runs.)
        let sph_steps = 0u32;
        let sph_dt = if sph_steps > 0 {
            dt / sph_steps as f32
        } else {
            0.0
        };
        let sph = SphUniform {
            counts: [PARTICLES as f32, sph_dt, -9.81, count as f32],
            // Droplets 2 cm across: a 6 cm kernel, water's density.
            kernel: [0.06, 0.0072, 1000.0, 3.0],
            visc: [0.35, 0.25, 9.0, 0.0],
            bodies,
        };
        queue.write_buffer(&self.sph_uniform, 0, bytemuck::bytes_of(&sph));
        for b in &self.bodies {
            let d = &b.desc;
            let s = &d.style;
            let u = BodyUniform {
                grid: [d.origin[0], d.origin[1], d.cell, d.level],
                dims: [d.size.0 as f32, d.size.1 as f32, d.wave_speed, d.damping],
                flow: [
                    if d.flow.is_some() { 1.0 } else { 0.0 },
                    2.5,
                    self.time,
                    step_dt,
                ],
                shallow: [s.shallow[0], s.shallow[1], s.shallow[2], s.deep_at],
                deep: [s.deep[0], s.deep[1], s.deep[2], s.clarity],
                misc: [s.foam, 1.0 / steps.max(1) as f32, s.look as u32 as f32, 0.0],
            };
            queue.write_buffer(&b.uniform, 0, bytemuck::bytes_of(&u));
        }
        for st in self.streams.iter().filter(|s| s.visible) {
            let s = &st.style;
            let u = StreamUniform {
                shallow: [s.shallow[0], s.shallow[1], s.shallow[2], st.speed],
                deep: [s.deep[0], s.deep[1], s.deep[2], s.foam],
                misc: [self.time, 0.0, 0.0, 0.0],
            };
            queue.write_buffer(&st.uniform, 0, bytemuck::bytes_of(&u));
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("velvet-water-step"),
            timestamp_writes: None,
        });
        // Droplets first (those that land add their splash to this frame's list).
        for _ in 0..sph_steps {
            let groups = PARTICLES.div_ceil(64) as u32;
            pass.set_pipeline(&self.sph_density);
            pass.set_bind_group(0, &self.sph_groups[self.particle_parity], &[]);
            // Density into the other buffer (with everything else copied through), then the
            // forces and the step back into this one.
            pass.dispatch_workgroups(groups, 1, 1);
            self.particle_parity = 1 - self.particle_parity;
            pass.set_pipeline(&self.sph_forces);
            pass.set_bind_group(0, &self.sph_groups[self.particle_parity], &[]);
            pass.dispatch_workgroups(groups, 1, 1);
            self.particle_parity = 1 - self.particle_parity;
        }
        drop(pass);
    }

    /// Draws the visible surfaces and the droplets (in the scene pass, after the solids).
    pub(crate) fn set_reflection(&mut self, group: Option<std::sync::Arc<wgpu::BindGroup>>) {
        self.reflection = group;
    }

    /// The level of the visible water nearest `focus` (the surface the frame mirrors).
    /// Whether any shown body's surface may be on screen (`outside` says a box is not).
    pub(crate) fn any_body_on_screen(&self, outside: impl Fn([[f32; 3]; 2]) -> bool) -> bool {
        self.bodies.iter().filter(|b| b.visible).any(|b| {
            let d = &b.desc;
            let x1 = d.origin[0] + d.size.0 as f32 * d.cell;
            let z1 = d.origin[1] + d.size.1 as f32 * d.cell;
            // A little above and below the still level (rings, wobble).
            !outside([
                [d.origin[0], d.level - 0.5, d.origin[1]],
                [x1, d.level + 0.5, z1],
            ])
        })
    }

    /// The boxes of the shown bodies at `level` (those that show the frame's reflection).
    pub(crate) fn bodies_at_level(&self, level: f32) -> Vec<[[f32; 3]; 2]> {
        self.bodies
            .iter()
            .filter(|b| b.visible && (b.desc.level - level).abs() < 0.05)
            .map(|b| {
                let d = &b.desc;
                let x1 = d.origin[0] + d.size.0 as f32 * d.cell;
                let z1 = d.origin[1] + d.size.1 as f32 * d.cell;
                [
                    [d.origin[0], d.level - 0.5, d.origin[1]],
                    [x1, d.level + 0.5, z1],
                ]
            })
            .collect()
    }

    pub(crate) fn reflection_level(&self, focus: Vec3) -> Option<f32> {
        self.bodies
            .iter()
            .filter(|b| b.visible)
            .map(|b| {
                let d = &b.desc;
                let cx = d.origin[0] + d.size.0 as f32 * d.cell * 0.5;
                let cz = d.origin[1] + d.size.1 as f32 * d.cell * 0.5;
                let half = (d.size.0.max(d.size.1) as f32 * d.cell * 0.5).max(0.1);
                (
                    ((cx - focus.x).hypot(cz - focus.z) - half).max(0.0),
                    d.level,
                )
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .filter(|(d, _)| *d < 30.0)
            .map(|(_, level)| level)
    }

    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(group) = &self.reflection {
            pass.set_bind_group(2, group.as_ref(), &[]);
        }
        if self.streams.iter().any(|s| s.visible) {
            pass.set_pipeline(&self.stream_pipeline);
            for st in self.streams.iter().filter(|s| s.visible) {
                pass.set_bind_group(1, &st.group, &[]);
                pass.set_vertex_buffer(0, st.vertices.slice(..));
                pass.set_index_buffer(st.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..st.count, 0, 0..1);
            }
        }
        pass.set_pipeline(&self.surface);
        for b in self.bodies.iter().filter(|b| b.visible) {
            let quads = (b.desc.size.0 - 1) * (b.desc.size.1 - 1);
            pass.set_bind_group(1, &b.render, &[]);
            pass.draw(0..quads * 6, 0..1);
        }
        if self.time < self.any_alive_until {
            pass.set_pipeline(&self.droplets);
            pass.set_bind_group(1, &self.droplet_groups[self.particle_parity], &[]);
            pass.draw(0..(PARTICLES * 6) as u32, 0..1);
        }
    }
}

/// A stream's vertices are the meshes' own ([`crate::MeshVertex3D`]).
fn stream_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
    const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4];
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<crate::MeshVertex3D>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

/// The renderer's camera block (kept in step with the mesh shader's).
const CAMERA_WGSL: &str = r#"
struct Camera {
    view_projection: mat4x4<f32>,
    light_view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    bounce_color: vec4<f32>,
    moon_direction: vec4<f32>,
    moon_color: vec4<f32>,
    reflection_color: vec4<f32>,
    horizon_tint: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    focus_position: vec4<f32>,
    viewport_size: vec4<f32>,
    local_light_meta: vec4<f32>,
    local_light_positions: array<vec4<f32>, 32>,
    local_light_colors: array<vec4<f32>, 32>,
    toon_params: vec4<f32>,
    toon_shadow: vec4<f32>,
    grade: vec4<f32>,
    clip_plane: vec4<f32>,
};
@group(2) @binding(0) var reflection_texture: texture_2d<f32>;
@group(2) @binding(1) var reflection_sampler: sampler;

/// What the water mirrors at this pixel (`offset` in pixels wobbles it), and whether there is a
/// reflection this frame (0 none).
fn mirrored(clip: vec4<f32>, offset: vec2<f32>) -> vec4<f32> {
    let uv = (clip.xy + offset) / camera.viewport_size.xy;
    let c = textureSampleLevel(reflection_texture, reflection_sampler, clamp(uv, vec2<f32>(0.001), vec2<f32>(0.999)), 0.0);
    // Premultiplied: its alpha is how much of the pixel something covers.
    return vec4<f32>(c.rgb / max(c.a, 0.001), c.a);
}
@group(0) @binding(0) var<uniform> camera: Camera;
"#;

/// Smoothed Particle Hydrodynamics for the droplets (Müller, Charypar & Gross 2003).
const SPH_SHADER: &str = r#"
struct Sph {
    counts: vec4<f32>,
    kernel: vec4<f32>,
    visc: vec4<f32>,
    bodies: array<vec4<f32>, 16>,
};
struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
};
struct Splashes {
    count: atomic<u32>,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    items: array<vec4<f32>, 256>,
};
@group(0) @binding(0) var<uniform> sph: Sph;
@group(0) @binding(1) var<storage, read> p_in: array<Particle>;
@group(0) @binding(2) var<storage, read_write> p_out: array<Particle>;
@group(0) @binding(3) var<storage, read_write> splashes: Splashes;

const PI: f32 = 3.14159265;

// Every droplet against every other, a tile of 64 at a time through workgroup memory (each
// droplet is read from the buffer once per workgroup, not once per droplet).
var<workgroup> tile_pos: array<vec4<f32>, 64>;
var<workgroup> tile_vel: array<vec4<f32>, 64>;

// Density at every live droplet (poly6 kernel); copied through to the other buffer.
@compute @workgroup_size(64)
fn cs_density(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) li: u32) {
    let n = u32(sph.counts.x);
    let i = id.x;
    var me = p_in[min(i, n - 1u)];
    let h = sph.kernel.x;
    let h2 = h * h;
    let poly6 = 315.0 / (64.0 * PI * pow(h, 9.0));
    var rho = 0.0;
    for (var base = 0u; base < n; base = base + 64u) {
        let src = p_in[min(base + li, n - 1u)];
        tile_pos[li] = src.pos;
        workgroupBarrier();
        if (me.pos.w > 0.0) {
            for (var j = 0u; j < 64u; j = j + 1u) {
                let o = tile_pos[j];
                if (o.w <= 0.0 || base + j >= n) { continue; }
                let r = me.pos.xyz - o.xyz;
                let r2 = dot(r, r);
                if (r2 < h2) {
                    let d = h2 - r2;
                    rho = rho + sph.kernel.y * poly6 * d * d * d;
                }
            }
        }
        workgroupBarrier();
    }
    if (i >= n) { return; }
    if (me.pos.w > 0.0) {
        me.vel.w = max(rho, sph.kernel.z * 0.05);
    }
    p_out[i] = me;
}

// Pressure (spiky gradient) and viscosity (its Laplacian) between droplets, gravity, then the
// step; a droplet that falls back into a body leaves its splash there.
@compute @workgroup_size(64)
fn cs_forces(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) li: u32) {
    let n = u32(sph.counts.x);
    let i = id.x;
    var me = p_in[min(i, n - 1u)];
    let dt = sph.counts.y;
    let h = sph.kernel.x;
    let mass = sph.kernel.y;
    let rest = sph.kernel.z;
    let stiff = sph.kernel.w;
    let spiky = -45.0 / (PI * pow(h, 6.0));
    let visc_lap = 45.0 / (PI * pow(h, 6.0));
    let p_i = stiff * (me.vel.w - rest);
    var force = vec3<f32>(0.0);
    for (var base = 0u; base < n; base = base + 64u) {
        let src = p_in[min(base + li, n - 1u)];
        tile_pos[li] = src.pos;
        tile_vel[li] = src.vel;
        workgroupBarrier();
        if (me.pos.w > 0.0) {
            for (var j = 0u; j < 64u; j = j + 1u) {
                let o_pos = tile_pos[j];
                let o_vel = tile_vel[j];
                if (o_pos.w <= 0.0 || base + j >= n || base + j == i) { continue; }
                let r = me.pos.xyz - o_pos.xyz;
                let len = length(r);
                if (len < h && len > 0.00001) {
                    let p_j = stiff * (o_vel.w - rest);
                    let w = h - len;
                    force = force - (r / len) * mass * (p_i + p_j) / (2.0 * o_vel.w) * spiky * w * w;
                    force = force + sph.visc.x * mass * (o_vel.xyz - me.vel.xyz) / o_vel.w * visc_lap * w;
                }
            }
        }
        workgroupBarrier();
    }
    if (i >= n) { return; }
    if (me.pos.w <= 0.0) {
        p_out[i] = me;
        return;
    }
    var v = me.vel.xyz + dt * (force / me.vel.w + vec3<f32>(0.0, sph.counts.z, 0.0));
    let speed = length(v);
    if (speed > sph.visc.z) { v = v * (sph.visc.z / speed); }
    var p = me.pos.xyz + dt * v;
    var life = me.pos.w - dt;
    // Into the water?
    let bodies = u32(sph.counts.w);
    for (var b = 0u; b < bodies; b = b + 1u) {
        let box = sph.bodies[b * 2u];
        let level = sph.bodies[b * 2u + 1u].x;
        if (p.x > box.x && p.x < box.z && p.z > box.y && p.z < box.w && p.y < level && v.y < 0.0) {
            let slot = atomicAdd(&splashes.count, 1u);
            if (slot < 256u) {
                splashes.items[slot] = vec4<f32>(p.x, p.z, 0.07, clamp(-v.y * 0.0012, 0.0, 0.006));
            }
            life = 0.0;
        }
    }
    if (p.y < -50.0) { life = 0.0; }
    me.pos = vec4<f32>(p, life);
    me.vel = vec4<f32>(v, me.vel.w);
    p_out[i] = me;
}
"#;

/// The surface and the droplets.
const DRAW_SHADER: &str = r#"
struct Body {
    grid: vec4<f32>,
    dims: vec4<f32>,
    flow: vec4<f32>,
    shallow: vec4<f32>,
    deep: vec4<f32>,
    misc: vec4<f32>,
};
struct Rings {
    info: vec4<f32>,
    items: array<vec4<f32>, 32>,
};
@group(1) @binding(0) var<uniform> body: Body;
@group(1) @binding(2) var<storage, read> statics: array<vec4<f32>>;
@group(1) @binding(3) var<uniform> rings: Rings;

// The looks (misc.z).
const BROOK: i32 = 0;
const SPRING: i32 = 1;
const POND: i32 = 2;
const WELL: i32 = 3;
const TUB: i32 = 4;

fn width() -> i32 { return i32(body.dims.x); }
fn height() -> i32 { return i32(body.dims.y); }
fn index(i: i32, j: i32) -> i32 {
    return clamp(j, 0, height() - 1) * width() + clamp(i, 0, width() - 1);
}

struct SurfaceOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    // The stream's velocity here (x, z), the depth, the bank's mask.
    @location(1) flow_depth: vec4<f32>,
    @location(2) grid: vec2<f32>,
};

fn view_direction(world: vec3<f32>) -> vec3<f32> {
    if (camera.viewport_size.z > 0.5) {
        return normalize(camera.camera_position.xyz - camera.focus_position.xyz);
    }
    return normalize(camera.camera_position.xyz - world);
}

// A flat surface at the water's level: nothing in it can spike. Everything that moves is drawn.
@vertex
fn vs_surface(@builtin(vertex_index) vi: u32) -> SurfaceOut {
    let quads_x = u32(width() - 1);
    let quad = vi / 6u;
    let corner = vi % 6u;
    var offsets = array<vec2<u32>, 6>(vec2<u32>(0u, 0u), vec2<u32>(1u, 0u), vec2<u32>(1u, 1u), vec2<u32>(0u, 0u), vec2<u32>(1u, 1u), vec2<u32>(0u, 1u));
    let cell = vec2<u32>(quad % quads_x, quad / quads_x) + offsets[corner];
    let g = vec2<f32>(f32(cell.x) + 0.5, f32(cell.y) + 0.5);
    let ci = i32(cell.x);
    let cj = i32(cell.y);
    let st = statics[index(ci, cj)];
    let dx = body.grid.z;
    let world = vec3<f32>(body.grid.x + g.x * dx, body.grid.w, body.grid.y + g.y * dx);
    var out: SurfaceOut;
    // Running water is nudged towards the camera along the line of sight (the picture does not
    // move): a coarse bank poking just above it no longer notches its edge.
    out.clip = camera.view_projection * vec4<f32>(world + view_direction(world) * select(0.0, 0.3, body.flow.x > 0.5), 1.0);
    var near = 0.0;
    for (var dj = -2; dj <= 2; dj = dj + 2) {
        for (var di = -2; di <= 2; di = di + 2) {
            near = max(near, statics[index(ci + di, cj + dj)].x);
        }
    }
    if (near < 0.01) {
        out.clip = vec4<f32>(0.0, 0.0, 2.0, 1.0);
    }
    out.world = world;
    out.flow_depth = vec4<f32>(st.z, st.w, st.y, st.x);
    out.grid = g;
    return out;
}

fn hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}
fn noise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2<f32>(1.0, 0.0)), u.x), mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn fogged(color: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let d = length(camera.camera_position.xyz - world);
    let span = max(camera.fog_params.y - camera.fog_params.x, 0.001);
    let f = clamp((d - camera.fog_params.x) / span, 0.0, 1.0);
    return mix(color, camera.fog_color.rgb, f);
}

// The bank's mask, bilinear per fragment.
fn mask_at(p: vec2<f32>) -> f32 {
    let q = p - vec2<f32>(0.5);
    let i0 = i32(floor(q.x));
    let j0 = i32(floor(q.y));
    let f = q - vec2<f32>(f32(i0), f32(j0));
    let a = statics[index(i0, j0)].x;
    let b = statics[index(i0 + 1, j0)].x;
    let c = statics[index(i0, j0 + 1)].x;
    let d = statics[index(i0 + 1, j0 + 1)].x;
    return mix(mix(a, b, f.x), mix(c, d, f.x), f.y);
}

// Rings spreading from whatever touched the water (a step, a drop, a bubble): drawn, not
// simulated. Each is two thin crests that run out and fade. Returns the line's strength.
fn ring_lines(p: vec2<f32>) -> f32 {
    let t = rings.info.y;
    let n = u32(rings.info.x);
    var line = 0.0;
    for (var i = 0u; i < 32u; i = i + 1u) {
        if (i >= n) { break; }
        let r = rings.items[i];
        let age = t - r.z;
        if (age < 0.0 || age > 2.6) { continue; }
        let life = 1.0 - age / 2.6;
        let d = length(p - r.xy);
        let radius = 0.1 + age * 0.75 * (0.6 + 0.4 * r.w);
        for (var k = 0; k < 2; k = k + 1) {
            let rk = radius - f32(k) * 0.28;
            if (rk <= 0.02) { continue; }
            let w = 0.03 + age * 0.012;
            let crest = exp(-pow((d - rk) / w, 2.0));
            line = line + crest * life * life * r.w * select(1.0, 0.55, k == 1);
        }
    }
    return line;
}

// Thin parallel shine lines (the sky caught on the water, as painted water shows it), drifting.
fn shine(p: vec2<f32>, t: f32, angle: f32, spacing: f32, thin: f32) -> f32 {
    let dir = vec2<f32>(cos(angle), sin(angle));
    let q = dot(p, dir) / spacing + noise2(p * 0.6 + vec2<f32>(t * 0.05, 0.0)) * 1.6 - t * 0.04;
    let along = dot(p, vec2<f32>(-dir.y, dir.x));
    let broken = smoothstep(0.35, 0.6, noise2(vec2<f32>(along * 0.7, floor(q)) + vec2<f32>(t * 0.08, 0.0)));
    let band = 1.0 - smoothstep(0.0, thin, abs(fract(q) - 0.5));
    return band * broken;
}

// A light's reflection: a column of broken glitter on the water, along the line of sight, under
// the light (where its mirror image falls for this camera).
fn light_reflection(p: vec2<f32>, light: vec3<f32>, v: vec3<f32>, t: f32, length_k: f32) -> f32 {
    let level = body.grid.w;
    let up = max(v.y, 0.15);
    let s = max(light.y - level, 0.0) / up;
    let centre = light.xz + v.xz * s;
    let along_dir = normalize(v.xz + vec2<f32>(1e-4, 0.0));
    let rel = p - centre;
    let along = dot(rel, along_dir);
    let across = dot(rel, vec2<f32>(-along_dir.y, along_dir.x));
    let reach = (0.6 + s * 0.35) * length_k;
    let shape = exp(-pow(across / 0.22, 2.0)) * exp(-pow(along / reach, 2.0));
    // Broken into short horizontal dashes that shimmer.
    let dash = smoothstep(0.45, 0.62, noise2(vec2<f32>(across * 9.0 + t * 0.7, along * 2.2 - t * 1.1)));
    return shape * (0.35 + 0.65 * dash);
}

@fragment
fn fs_surface(in: SurfaceOut) -> @location(0) vec4<f32> {
    let look = i32(body.misc.z + 0.5);
    var mask = mask_at(in.grid);
    // A stream is built from stretches: where two meet, the bank's mask dips across it. Looking a
    // couple of cells up and down the stream bridges the seam (a real bank runs along the flow).
    if (look == BROOK) {
        let fv = in.flow_depth.xy;
        let dir = select(vec2<f32>(1.0, 0.0), fv / max(length(fv), 0.0001), length(fv) > 0.02);
        mask = max(mask, min(mask_at(in.grid + dir * 2.5), mask_at(in.grid - dir * 2.5)));
    }
    if (mask < 0.42) {
        discard;
    }
    let t = body.flow.z;
    let wp = in.world.xz;
    let vel = in.flow_depth.xy;
    let depth = in.flow_depth.z;
    let speed = length(vel);
    let sun = camera.light_color.rgb;
    let v = view_direction(in.world);
    let night = clamp(camera.moon_direction.w, 0.0, 1.0);
    let light = camera.ambient_color.rgb * 1.8 + camera.bounce_color.rgb * 0.5 + sun * 0.45;
    // Depth in painted bands, their edges wobbling a little.
    let wobble = (noise2(wp * 1.4 + vec2<f32>(t * 0.12, -t * 0.07)) - 0.5) * 0.16;
    let d = clamp(depth / max(body.shallow.w, 0.05) + wobble, 0.0, 1.0);
    let mid = mix(body.shallow.rgb, body.deep.rgb, 0.5);
    var water = body.shallow.rgb;
    water = mix(water, mid, smoothstep(0.24, 0.3, d));
    water = mix(water, body.deep.rgb, smoothstep(0.6, 0.66, d));
    var color = water * light;
    let white = camera.ambient_color.rgb * 2.6 + sun * 0.75 + camera.bounce_color.rgb * 0.5;
    var foam = 0.0;
    var alpha = mix(1.0 - body.deep.w, 0.95, smoothstep(0.24, 0.66, d));
    // The shore: how near the bank (0 at the water's edge, 1 a few cells in).
    let shore = 1.0 - smoothstep(0.42, 0.75, mask);

    if (look == BROOK) {
        // Running water: the stream's own frame, dashes of light carried down it, white water at
        // the banks, a lace of foam along them, broken glints where it is quick.
        let dir = select(vec2<f32>(1.0, 0.0), vel / max(speed, 0.0001), speed > 0.02);
        let along = dot(wp, dir);
        let across = dot(wp, vec2<f32>(-dir.y, dir.x));
        let run = t * (0.6 + speed * 1.4);
        let streak = smoothstep(0.74, 0.8, noise2(vec2<f32>(along * 1.1 - run, across * 5.5)));
        let fine = smoothstep(0.8, 0.86, noise2(vec2<f32>(along * 2.6 - run * 1.6, across * 11.0 + 3.0)));
        color = mix(color, white * 0.9, streak * 0.32 + fine * 0.18);
        // Darker troughs between, so the surface reads as moving ridges.
        let ridge = noise2(vec2<f32>(along * 0.8 - run * 0.8, across * 3.0 + 7.0));
        color = color * (0.9 + 0.2 * smoothstep(0.3, 0.7, ridge));
        let lace = step(0.5, shore + (noise2(vec2<f32>(along * 3.0 - run * 2.0, across * 3.0)) - 0.5) * 0.7);
        foam = max(foam, lace * body.misc.x);
        let sparkle = smoothstep(0.86, 0.9, noise2(vec2<f32>(along * 4.0 - run * 2.2, across * 14.0))) * smoothstep(0.15, 0.5, speed);
        color = color + sun * sparkle * 0.9;
    } else if (look == SPRING) {
        // A hot spring: milky turquoise, lighter where the heat rises, bubbles breaking in little
        // rings, a soft caustic net in the shallows, a gentle shine.
        let heat = smoothstep(0.45, 0.95, d);
        color = mix(color, color * 1.15 + vec3<f32>(0.02, 0.05, 0.05) * light, heat * 0.5);
        let cellp = wp * 1.4;
        let id = floor(cellp);
        let f = fract(cellp) - 0.5;
        let h = hash2(id);
        let cycle = fract(t * (0.35 + h * 0.3) + h * 7.0);
        let spot = vec2<f32>(hash2(id + 3.1), hash2(id + 7.7)) - 0.5;
        let r = length(f - spot * 0.6);
        let bubble = (1.0 - smoothstep(0.0, 0.025, abs(r - cycle * 0.2))) * (1.0 - cycle) * step(0.82, h) * smoothstep(0.3, 0.6, d);
        color = mix(color, white, bubble * 0.45);
        let net = 1.0 - smoothstep(0.02, 0.05, abs(noise2(wp * 1.7 + t * 0.1) - noise2(wp * 2.6 - t * 0.08)));
        color = color + sun * net * (1.0 - smoothstep(0.2, 0.5, d)) * 0.12;
        color = mix(color, white, shine(wp, t, 0.6, 1.3, 0.035) * 0.16);
        let lace = step(0.55, shore + (noise2(wp * 4.0 + t * 0.3) - 0.5) * 0.5);
        foam = max(foam, lace * body.misc.x * 0.8);
    } else if (look == POND) {
        // A still forest pool: a dark mirror. The sky lies on it in a pale band, gusts of wind
        // ruffle patches of it, and every lamp and glowing thing near it shines in it.
        let gust = smoothstep(0.55, 0.75, noise2(wp * 0.35 + vec2<f32>(t * 0.18, t * 0.07)));
        let ruffle = shine(wp, t * 2.0, 1.9, 0.25, 0.08) * gust;
        color = mix(color, color * 1.6 + camera.ambient_color.rgb * 0.4, ruffle * 0.6);
        color = mix(color, camera.fog_color.rgb * 0.6 + camera.ambient_color.rgb * 1.6, shine(wp, t, 0.4, 1.6, 0.08) * 0.22 * (1.0 - night * 0.5));
        let lace = step(0.62, shore + (noise2(wp * 3.0) - 0.5) * 0.4);
        foam = max(foam, lace * body.misc.x * 0.35);
    } else if (look == WELL) {
        // Down a well: dark, still, a round patch of sky caught on it, now and then a drip.
        color = color * 0.8;
        let c = vec2<f32>(body.grid.x + body.dims.x * body.grid.z * 0.42, body.grid.y + body.dims.y * body.grid.z * 0.4);
        let disc = 1.0 - smoothstep(0.18, 0.22, length(wp - c));
        color = mix(color, camera.fog_color.rgb * 0.5 + camera.ambient_color.rgb * 2.0, disc * 0.35);
    } else {
        // A tub indoors: clear, the wooden bed showing through, a slow slosh of shine lines.
        color = mix(color, white, shine(wp, t * 1.4, 0.3, 0.45, 0.06) * 0.3);
        alpha = alpha * 0.92;
    }

    // Rings from whatever touched it.
    let ring = ring_lines(wp);
    color = mix(color, white, smoothstep(0.25, 0.5, ring) * 0.85);
    // The lights round it shine in it (lamps, flames, fungi); stronger on still dark water.
    let reflective = select(0.55, 1.0, look == POND || look == WELL);
    let lights = u32(clamp(camera.local_light_meta.x, 0.0, 32.0));
    for (var q = 0u; q < 32u; q = q + 1u) {
        if (q >= lights) { break; }
        let lp = camera.local_light_positions[q];
        let lc = camera.local_light_colors[q];
        if (length(lp.xz - wp) > lp.w * 2.2 + 4.0) { continue; }
        let r = light_reflection(wp, lp.xyz, v, t, select(1.0, 0.6, look == BROOK));
        color = color + lc.rgb * min(lc.w, 6.0) * 0.06 * reflective * smoothstep(0.25, 0.55, r);
    }
    // The moon on still water at night: a path of glitter.
    if (look == POND || look == SPRING || look == WELL) {
        let moon = normalize(camera.moon_direction.xyz);
        let m = light_reflection(wp, vec3<f32>(wp.x + moon.x * 40.0, body.grid.w + moon.y * 40.0, wp.y + moon.z * 40.0), v, t, 1.0);
        color = color + camera.moon_color.rgb * night * smoothstep(0.4, 0.6, m) * 0.6;
    }
    // The scene mirrored in it (the frame's reflection is of the water nearest the camera: only
    // water at that level shows it), wobbled by the wind and the rings, stronger at a glance.
    if (camera.grade.z > 0.5 && abs(body.grid.w - camera.grade.w) < 0.05) {
        let wob = vec2<f32>(noise2(wp * 2.3 + vec2<f32>(t * 0.4, 0.0)) - 0.5, noise2(wp * 2.3 + vec2<f32>(3.1, t * 0.35)) - 0.5) * 7.0 + vec2<f32>(0.0, ring * 9.0);
        let r = mirrored(in.clip, wob);
        var strength = 0.55;
        if (look == POND) { strength = 0.78; }
        if (look == WELL) { strength = 0.6; }
        if (look == BROOK) { strength = 0.3; }
        if (look == TUB) { strength = 0.4; }
        let glance = 0.7 + 0.3 * pow(1.0 - clamp(abs(v.y), 0.0, 1.0), 2.0);
        // Seen through the water's own colour, a little darker than the thing itself.
        let tinted = r.rgb * mix(vec3<f32>(0.9), body.shallow.rgb * 1.4 + vec3<f32>(0.3), 0.4);
        color = mix(color, tinted, strength * glance * r.a * (1.0 - foam));
        if (camera.grade.z > 1.5) {
            return vec4<f32>(mix(vec3<f32>(0.0), r.rgb, r.a), 1.0);
        }
    }
    color = mix(color, white, foam * 0.9);
    alpha = max(alpha, foam * 0.95) * smoothstep(0.42, 0.5, mask);
    return vec4<f32>(fogged(color, in.world), alpha);
}

fn glint(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, sharp: f32) -> f32 {
    let hv = normalize(l + v);
    let s = pow(max(dot(n, hv), 0.0), sharp);
    return smoothstep(0.45, 0.6, s) * 0.8 + s * 0.4;
}

// ------------------------------------------------------------------- running and falling water
struct Stream {
    shallow: vec4<f32>,
    deep: vec4<f32>,
    misc: vec4<f32>,
};
@group(1) @binding(5) var<uniform> stream: Stream;

struct StreamIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};
struct StreamOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) flow: vec3<f32>,
};

@vertex
fn vs_stream(v: StreamIn) -> StreamOut {
    var out: StreamOut;
    out.clip = camera.view_projection * vec4<f32>(v.position, 1.0);
    out.world = v.position;
    out.uv = v.uv;
    out.flow = v.color.rgb;
    return out;
}

// Toon: where it runs, the pools' colour deeper down the middle with ridges and dashes of light
// carried downstream and a lace of foam at the banks; where it falls, bright strands pouring down;
// below each fall a froth that thins out downstream. Rings from whatever touches it.
@fragment
fn fs_stream(in: StreamOut) -> @location(0) vec4<f32> {
    let steep = clamp(in.flow.x, 0.0, 1.0);
    let froth = clamp(in.flow.y, 0.0, 1.0);
    let width = max(in.flow.z * 4.0, 0.2);
    let across = abs(in.uv.x - 0.5) * 2.0;
    let ax = in.uv.x * width;
    let v = in.uv.y;
    let t = stream.misc.x;
    let run = t * stream.shallow.w * (1.0 + steep * 1.8);
    let sun = camera.light_color.rgb;
    let light = camera.ambient_color.rgb * 1.8 + camera.bounce_color.rgb * 0.5 + sun * 0.45;
    let white = camera.ambient_color.rgb * 2.6 + sun * 0.75 + camera.bounce_color.rgb * 0.5;
    var color = mix(stream.shallow.rgb, stream.deep.rgb, (1.0 - across) * 0.55) * light;
    let ridge = noise2(vec2<f32>(ax * 3.0 + 7.0, v * 0.9 - run * 0.8));
    color = color * (0.9 + 0.2 * smoothstep(0.3, 0.7, ridge));
    let streak = smoothstep(0.72, 0.78, noise2(vec2<f32>(ax * 5.0, v * 1.3 - run)));
    let fine = smoothstep(0.8, 0.86, noise2(vec2<f32>(ax * 10.0 + 3.0, v * 2.6 - run * 1.6)));
    color = mix(color, white, streak * 0.35 + fine * 0.2);
    let strand = noise2(vec2<f32>(ax * 7.0, v * 0.35 - run * 0.5));
    let falling = mix(mix(stream.shallow.rgb * light, white, 0.35), white, smoothstep(0.42, 0.68, strand) * 0.8);
    color = mix(color, falling, smoothstep(0.25, 0.6, steep));
    let lace = step(0.6, across + (noise2(vec2<f32>(v * 3.0 - run * 2.0, in.uv.x * 3.0)) - 0.5) * 0.5) * (1.0 - steep) * stream.deep.w;
    let bubbles = smoothstep(0.35, 0.6, noise2(vec2<f32>(ax * 4.0 + t * 0.3, v * 3.0 - run)) + froth * 0.35) * froth;
    color = mix(color, white, max(lace * 0.8, bubbles));
    color = mix(color, white, smoothstep(0.25, 0.5, ring_lines(in.world.xz)) * 0.8 * (1.0 - steep));
    // Soft sides where it lies flat (it meets its banks or spreads on a pool).
    let alpha = mix(0.9, 0.97, steep) * (1.0 - smoothstep(0.55, 1.0, across) * (1.0 - steep) * 0.85);
    return vec4<f32>(fogged(color, in.world), alpha);
}

struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
};
@group(1) @binding(4) var<storage, read> particles: array<Particle>;

struct DropOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world: vec3<f32>,
    @location(2) fade: f32,
};

@vertex
fn vs_droplet(@builtin(vertex_index) vi: u32) -> DropOut {
    let p = particles[vi / 6u];
    var corners = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
    let c = corners[vi % 6u];
    var out: DropOut;
    out.uv = c;
    out.fade = clamp(p.pos.w * 4.0, 0.0, 1.0);
    if (p.pos.w <= 0.0) {
        out.clip = vec4<f32>(0.0, 0.0, 2.0, 1.0);
        out.world = vec3<f32>(0.0);
        return out;
    }
    let forward = normalize(camera.focus_position.xyz - camera.camera_position.xyz);
    let right = normalize(cross(forward, vec3<f32>(0.0, 1.0, 0.0)));
    let up = cross(right, forward);
    // Stretched a little along their motion, as falling water looks.
    let size = 0.022;
    let world = p.pos.xyz + (right * c.x + up * c.y) * size;
    out.clip = camera.view_projection * vec4<f32>(world, 1.0);
    out.world = world;
    return out;
}

@fragment
fn fs_droplet(in: DropOut) -> @location(0) vec4<f32> {
    let r2 = dot(in.uv, in.uv);
    if (r2 > 1.0) {
        discard;
    }
    let forward = normalize(camera.focus_position.xyz - camera.camera_position.xyz);
    let right = normalize(cross(forward, vec3<f32>(0.0, 1.0, 0.0)));
    let up = cross(right, forward);
    let n = normalize(right * in.uv.x + up * in.uv.y - forward * sqrt(1.0 - r2));
    let v = -forward;
    let l = normalize(camera.light_direction.xyz);
    let fresnel = 0.02 + 0.98 * pow(1.0 - max(dot(n, v), 0.0), 5.0);
    var color = camera.ambient_color.rgb * 2.0 + camera.light_color.rgb * 0.25;
    color = color * (0.35 + fresnel);
    color = color + camera.light_color.rgb * glint(n, v, l, 80.0) * 1.5;
    let alpha = (0.45 + 0.5 * fresnel) * in.fade;
    return vec4<f32>(fogged(color, in.world), alpha);
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
    fn the_water_shaders_are_valid() {
        validate(SPH_SHADER);
        validate(&format!("{CAMERA_WGSL}{DRAW_SHADER}"));
    }

    fn square(size: f32) -> Vec<[Vec3; 3]> {
        let (a, b, c, d) = (
            Vec3::new(0.0, 0.5, 0.0),
            Vec3::new(size, 0.5, 0.0),
            Vec3::new(size, 0.5, size),
            Vec3::new(0.0, 0.5, size),
        );
        vec![[a, b, c], [a, c, d]]
    }

    #[test]
    fn a_body_covers_its_triangles_and_is_deepest_in_the_middle() {
        let body = WaterBody3D::from_triangles(&square(2.0), 0.1, 0.8).expect("a body");
        assert!((body.level - 0.5).abs() < 1e-5);
        assert!(body.contains(1.0, 1.0));
        assert!(!body.contains(-0.15, 1.0));
        let at = |x: f32, z: f32| {
            let i = ((x - body.origin[0]) / body.cell) as u32;
            let j = ((z - body.origin[1]) / body.cell) as u32;
            body.depth[(j * body.size.0 + i) as usize]
        };
        assert!(at(1.0, 1.0) > at(0.1, 1.0));
        assert!(at(1.0, 1.0) <= 0.8 + 1e-4);
    }

    #[test]
    fn a_stream_runs_along_its_course() {
        let body = WaterBody3D::from_triangles(&square(4.0), 0.1, 0.5)
            .expect("a body")
            .with_flow_along(&[[0.0, 2.0], [4.0, 2.0]], 0.6);
        let flow = body.flow.as_ref().expect("a flow");
        let k = ((2.0 - body.origin[1]) / body.cell) as usize * body.size.0 as usize
            + ((2.0 - body.origin[0]) / body.cell) as usize;
        assert!(flow[k][0] > 0.3 && flow[k][1].abs() < 1e-3, "{:?}", flow[k]);
    }

    #[test]
    fn the_nearest_water_is_found_from_the_shore() {
        let body = WaterBody3D::from_triangles(&square(2.0), 0.1, 0.5).expect("a body");
        let p = body.nearest_water(-0.3, 1.0, 0.6).expect("water near");
        assert!(p[0] > 0.0 && p[0] < 0.2);
        assert!(body.nearest_water(-2.0, 1.0, 0.5).is_none());
    }
}
