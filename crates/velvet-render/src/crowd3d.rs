//! Crowds on the GPU: thousands of soldiers of one model drawn as one mesh.
//!
//! A crowd mesh is a rigid model built from parts (legs, arms, wings, tail, cape…), each vertex
//! tagged with its part and the joint it turns round. Each frame the game hands over one instance a
//! soldier (where, facing, size, tint, and eight animation numbers: walk phase, how fast, the
//! swing/thrust/shot phase, the fall; stance, hit flash, knock-back, flight), and a compute pass
//! writes every soldier's animated vertices, already in the world, straight into the mesh's own
//! vertex buffer. Everything that draws meshes (shading, shadows, ink lines) sees one ordinary
//! mesh whose drawn length follows the soldiers in it.
//!
//! The animation is the war prototype's vertex patch (`demon-queen-war-threejs/src/battle/crowd.js`)
//! as it was.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::mesh3d::MeshVertex3D;

/// Body parts a crowd vertex can belong to.
pub mod part {
    /// The trunk and head (it does not move on its own).
    pub const BODY: f32 = 0.0;
    /// The left leg.
    pub const LEG_L: f32 = 1.0;
    /// The right leg.
    pub const LEG_R: f32 = 2.0;
    /// The weapon arm (swings).
    pub const ARM_W: f32 = 3.0;
    /// The shield arm.
    pub const ARM_S: f32 = 4.0;
    /// The left wing.
    pub const WING_L: f32 = 5.0;
    /// The right wing.
    pub const WING_R: f32 = 6.0;
    /// A snake's or lizard's tail (waves along its length).
    pub const TAIL: f32 = 7.0;
    /// A four-legged beast's front left leg.
    pub const QLEG_FL: f32 = 8.0;
    /// Front right leg.
    pub const QLEG_FR: f32 = 9.0;
    /// Back left leg.
    pub const QLEG_BL: f32 = 10.0;
    /// Back right leg.
    pub const QLEG_BR: f32 = 11.0;
    /// A spear pushed forward.
    pub const THRUST: f32 = 12.0;
    /// A bow or crossbow arm (raised to shoot).
    pub const BOW: f32 = 13.0;
    /// A beast's head (bites).
    pub const QHEAD: f32 = 14.0;
    /// A beast's tail (wags).
    pub const QTAIL: f32 = 15.0;
    /// A banner's cloth (waves).
    pub const CAPE: f32 = 16.0;
    /// The arm holding a spear (follows the thrust).
    pub const SPEAR_ARM: f32 = 17.0;
    /// A cape hanging from the shoulders.
    pub const CAPE_V: f32 = 18.0;

    /// The part a model's object is named after (`ARM_W__orc` → ARM_W).
    pub fn by_name(name: &str) -> f32 {
        match name {
            "LEG_L" => LEG_L,
            "LEG_R" => LEG_R,
            "ARM_W" => ARM_W,
            "ARM_S" => ARM_S,
            "WING_L" => WING_L,
            "WING_R" => WING_R,
            "TAIL" => TAIL,
            "QLEG_FL" => QLEG_FL,
            "QLEG_FR" => QLEG_FR,
            "QLEG_BL" => QLEG_BL,
            "QLEG_BR" => QLEG_BR,
            "THRUST" => THRUST,
            "BOW" => BOW,
            "QHEAD" => QHEAD,
            "QTAIL" => QTAIL,
            "CAPE" => CAPE,
            "SPEAR_ARM" => SPEAR_ARM,
            "CAPE_V" => CAPE_V,
            _ => BODY,
        }
    }
}

/// One vertex of a crowd model, in its own space (feet on y = 0, facing +z, metres).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CrowdVertex3D {
    /// Where it is in the model.
    pub position: [f32; 3],
    /// Which part it belongs to (see [`part`]).
    pub part: f32,
    /// Its normal in the model.
    pub normal: [f32; 3],
    /// Padding (keep 0).
    pub _pad: f32,
    /// Its colour (linear RGB, alpha).
    pub color: [f32; 4],
    /// The joint its part turns round.
    pub pivot: [f32; 4],
}

/// A crowd model: its vertices and triangles, and whether it walks on four legs.
#[derive(Debug, Clone, Default)]
pub struct CrowdMesh3D {
    /// Its vertices.
    pub vertices: Vec<CrowdVertex3D>,
    /// Its triangles.
    pub indices: Vec<u32>,
    /// Whether it walks on four legs (its body pitches and falls on its side).
    pub quadruped: bool,
}

/// One soldier this frame.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default, PartialEq)]
pub struct CrowdInstance3D {
    /// Where its feet are.
    pub position: [f32; 3],
    /// Which way it faces (radians, 0 = +z).
    pub yaw: f32,
    /// Its size (1 as modelled).
    pub scale: f32,
    /// How see-through it is (0 as modelled, 1 gone).
    pub fade: f32,
    /// Padding (keep 0).
    pub _pad: [f32; 2],
    /// Walk phase, how fast (0..1.15), swing/thrust/shot phase (0 none), the fall (0..1).
    pub anim_a: [f32; 4],
    /// Stance (0 marching .. 1 fighting), hit flash, knock-back, flight (0..1).
    pub anim_b: [f32; 4],
    /// Its colours multiplied by this (linear RGB; 1 as modelled): brighter when chosen, a ring
    /// in a faction's colour. The fourth is unused.
    pub color: [f32; 4],
}

impl CrowdInstance3D {
    /// One standing at `position`, facing `yaw`, `scale` big, as modelled.
    pub fn at(position: [f32; 3], yaw: f32, scale: f32) -> Self {
        Self {
            position,
            yaw,
            scale,
            color: [1.0; 4],
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CrowdParams {
    vertices: u32,
    count: u32,
    quad: f32,
    time: f32,
}

pub(crate) struct GpuCrowd {
    params: wgpu::Buffer,
    instances: wgpu::Buffer,
    group: wgpu::BindGroup,
    pub(crate) vertex_count: u32,
    pub(crate) index_count: u32,
    pub(crate) capacity: u32,
    count: u32,
    quad: bool,
    dirty: bool,
}

pub(crate) struct Crowds3D {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    pub(crate) crowds: Vec<GpuCrowd>,
}

const WORKGROUP: u32 = 64;

impl Crowds3D {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let cs = wgpu::ShaderStages::COMPUTE;
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: cs,
            ty,
            count: None,
        };
        let storage = |read_only| wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-crowd-layout"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
                entry(1, storage(true)),
                entry(2, storage(true)),
                entry(3, storage(false)),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-crowd"),
            source: wgpu::ShaderSource::Wgsl(CROWD_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-crowd"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("velvet-crowd"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cs_crowd"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            layout,
            pipeline,
            crowds: Vec::new(),
        }
    }

    /// Registers a crowd of up to `capacity` soldiers whose vertices the pass writes into
    /// `vertex_buffer` (storage-capable, `capacity` × the model's vertices). Returns its index.
    pub(crate) fn add(
        &mut self,
        device: &wgpu::Device,
        source: &CrowdMesh3D,
        capacity: u32,
        vertex_buffer: &wgpu::Buffer,
    ) -> usize {
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-crowd-params"),
            contents: bytemuck::bytes_of(&CrowdParams {
                vertices: source.vertices.len() as u32,
                count: 0,
                quad: if source.quadruped { 1.0 } else { 0.0 },
                time: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let model = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-crowd-model"),
            contents: bytemuck::cast_slice(&source.vertices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-crowd-instances"),
            size: (capacity.max(1) as u64) * std::mem::size_of::<CrowdInstance3D>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-crowd-group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: model.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: instances.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: vertex_buffer.as_entire_binding(),
                },
            ],
        });
        self.crowds.push(GpuCrowd {
            params,
            instances,
            group,
            vertex_count: source.vertices.len() as u32,
            index_count: source.indices.len() as u32,
            capacity,
            count: 0,
            quad: source.quadruped,
            dirty: false,
        });
        self.crowds.len() - 1
    }

    /// This frame's soldiers of crowd `index` (at most its capacity); returns how many are in.
    pub(crate) fn set(
        &mut self,
        queue: &wgpu::Queue,
        index: usize,
        instances: &[CrowdInstance3D],
        time: f32,
    ) -> u32 {
        let Some(c) = self.crowds.get_mut(index) else {
            return 0;
        };
        let n = (instances.len() as u32).min(c.capacity);
        if n > 0 {
            queue.write_buffer(
                &c.instances,
                0,
                bytemuck::cast_slice(&instances[..n as usize]),
            );
        }
        queue.write_buffer(
            &c.params,
            0,
            bytemuck::bytes_of(&CrowdParams {
                vertices: c.vertex_count,
                count: n,
                quad: if c.quad { 1.0 } else { 0.0 },
                time,
            }),
        );
        c.count = n;
        c.dirty = n > 0;
        n
    }

    /// Writes every crowd that changed since the last frame.
    pub(crate) fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.crowds.iter().any(|c| c.dirty) {
            return;
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("velvet-crowd"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        for c in self.crowds.iter_mut().filter(|c| c.dirty) {
            pass.set_bind_group(0, &c.group, &[]);
            let threads = c.count * c.vertex_count;
            // One row of up to 65 535 workgroups, more rows when a big crowd needs them.
            let groups = threads.div_ceil(WORKGROUP);
            let x = groups.min(65_535);
            let y = groups.div_ceil(65_535).max(1);
            pass.dispatch_workgroups(x, y, 1);
            c.dirty = false;
        }
    }
}

/// The vertex buffer layout the compute pass writes: [`MeshVertex3D`] as plain floats.
const _: () = assert!(std::mem::size_of::<MeshVertex3D>() == 48);
const _: () = assert!(std::mem::size_of::<CrowdVertex3D>() == 64);
const _: () = assert!(std::mem::size_of::<CrowdInstance3D>() == 80);

pub(crate) const CROWD_SHADER: &str = r#"
struct Params {
    vertices: u32,
    count: u32,
    quad: f32,
    time: f32,
};
struct ModelVertex {
    position: vec4<f32>,
    normal: vec4<f32>,
    color: vec4<f32>,
    pivot: vec4<f32>,
};
struct Instance {
    position_yaw: vec4<f32>,
    scale_fade: vec4<f32>,
    anim_a: vec4<f32>,
    anim_b: vec4<f32>,
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> model: array<ModelVertex>;
@group(0) @binding(2) var<storage, read> instances: array<Instance>;
// The mesh's vertex buffer: 12 floats a vertex (position, normal, uv, colour).
@group(0) @binding(3) var<storage, read_write> out_vertices: array<f32>;

fn r_x(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, c, s), vec3<f32>(0.0, -s, c));
}
fn r_y(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(vec3<f32>(c, 0.0, -s), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(s, 0.0, c));
}
fn r_z(a: f32) -> mat3x3<f32> {
    let c = cos(a);
    let s = sin(a);
    return mat3x3<f32>(vec3<f32>(c, s, 0.0), vec3<f32>(-s, c, 0.0), vec3<f32>(0.0, 0.0, 1.0));
}
fn swing_curve(t: f32) -> f32 {
    if (t < 0.38) {
        let k = t / 0.38;
        return mix(-0.3, -3.3, 1.0 - (1.0 - k) * (1.0 - k));
    }
    if (t < 0.56) {
        let k = (t - 0.38) / 0.18;
        return mix(-3.3, -0.55, k * k);
    }
    return mix(-0.55, -0.3, (t - 0.56) / 0.44);
}
fn thrust_curve(t: f32) -> f32 {
    if (t <= 0.0) {
        return 0.0;
    }
    if (t < 0.35) {
        return -0.25 * (t / 0.35);
    }
    if (t < 0.55) {
        return mix(-0.25, 0.8, (t - 0.35) / 0.2);
    }
    return mix(0.8, 0.0, (t - 0.55) / 0.45);
}

@compute @workgroup_size(64)
fn cs_crowd(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x + id.y * 65535u * 64u;
    if (index >= params.count * params.vertices) {
        return;
    }
    let soldier = index / params.vertices;
    let v = model[index % params.vertices];
    let inst = instances[soldier];
    let ph = inst.anim_a.x;
    let mv = inst.anim_a.y;
    let at = inst.anim_a.z;
    let dead = inst.anim_a.w;
    let stance = inst.anim_b.x;
    let knock = inst.anim_b.z;
    let fly = inst.anim_b.w;
    let part = i32(v.position.w + 0.5);
    let pivot = v.pivot.xyz;
    var p = v.position.xyz;
    var rot = mat3x3<f32>(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
    var tr = vec3<f32>(0.0);
    let t = params.time;
    let idle = sin(t * 1.7 + ph * 0.37) * 0.5 + 0.5;
    if (part == 1 || part == 2) {
        let s = select(-1.0, 1.0, part == 1);
        rot = r_x(sin(ph) * 0.72 * mv * s * (1.0 - fly) + 0.55 * fly);
    } else if (part == 3) {
        var a = -0.3 - 0.4 * stance + sin(ph + 3.14) * 0.32 * mv - idle * 0.05;
        if (at > 0.0) {
            a = swing_curve(at);
        }
        rot = r_x(a);
    } else if (part == 4) {
        rot = r_x(-0.42 - 0.38 * stance + sin(ph) * 0.22 * mv);
    } else if (part == 5 || part == 6) {
        let s = select(-1.0, 1.0, part == 5);
        let beat = sin(ph * 1.7);
        var hit = 0.0;
        if (at > 0.0) {
            hit = sin(at * 6.28) * 0.5;
        }
        let a = fly * (0.2 + beat * 0.9) + (1.0 - fly) * (-0.75 + beat * 0.12 + hit);
        rot = r_z(s * a);
    } else if (part == 7) {
        let d = length(p.xz - pivot.xz);
        p.x = p.x + sin(ph * 0.9 - d * 1.9) * 0.16 * min(d, 2.2) * (0.35 + 0.65 * mv) + sin(t * 1.3 - d) * 0.03 * d;
    } else if (part >= 8 && part <= 11) {
        var off = 3.84;
        if (part == 8) { off = 0.0; }
        if (part == 9) { off = 0.7; }
        if (part == 10) { off = 3.14; }
        var lift = 0.0;
        if (at > 0.0 && part < 10) {
            lift = sin(at * 3.14) * 0.5;
        }
        rot = r_x(sin(ph + off) * (0.3 + 0.5 * mv) * min(mv * 3.0, 1.0) - lift);
    } else if (part == 12) {
        rot = r_x(1.42 * stance);
        tr.z = tr.z + thrust_curve(at) * 0.55;
    } else if (part == 17) {
        tr.z = tr.z + thrust_curve(at) * 0.55 + stance * 0.12;
        tr.y = tr.y + stance * 0.1;
    } else if (part == 13) {
        var draw = 0.0;
        if (at > 0.0) {
            draw = -sin(at * 3.14) * 0.18;
        }
        rot = r_x(-1.6 * stance + draw + sin(ph + 3.14) * 0.25 * mv * (1.0 - stance));
    } else if (part == 14) {
        var bite = 0.0;
        if (at > 0.0) {
            bite = sin(at * 3.14) * 0.4;
        }
        rot = r_x(sin(ph * 2.0) * 0.1 * mv - bite);
    } else if (part == 15) {
        rot = r_y(sin(ph * 2.0 + t * 1.5) * 0.35);
    } else if (part == 18) {
        // a cape hanging from the shoulders: it streams back when she moves, and ripples
        let drop = max(pivot.y - p.y, 0.0);
        p.z = p.z - drop * (0.05 + 0.24 * mv);
        p.y = p.y + drop * 0.05 * mv;
        p.x = p.x + sin(t * 2.2 + drop * 2.5) * 0.035 * drop;
        p.z = p.z + sin(t * 3.1 + drop * 3.0 + p.x * 4.0) * 0.03 * drop;
    } else if (part == 16) {
        let d = max(p.x - pivot.x, 0.0);
        let drop = max(pivot.y - p.y, 0.0);
        p.z = p.z + sin(t * 3.2 + d * 3.2 + ph) * 0.16 * d;
        p.x = p.x - drop * 0.08 * (1.0 + mv);
    }
    p = rot * (p - pivot) + pivot + tr;
    if (params.quad > 0.5) {
        let pitch = sin(ph * 2.0) * 0.05 * mv - knock * 0.45;
        let b = r_x(pitch);
        p = b * (p - vec3<f32>(0.0, 1.0, 0.0)) + vec3<f32>(0.0, 1.0, 0.0);
        rot = b * rot;
        p.y = p.y + abs(sin(ph)) * 0.09 * mv;
    } else {
        p.y = p.y + abs(sin(ph)) * 0.07 * mv * (1.0 - fly) + idle * 0.012;
        let b = r_x(-0.5 * knock);
        p = b * p;
        rot = b * rot;
    }
    if (dead > 0.0) {
        var fall = r_x(-1.5 * dead);
        if (params.quad > 0.5) {
            fall = r_z(1.5 * dead);
        }
        p = fall * p;
        rot = fall * rot;
        p.y = p.y - dead * 0.05;
    }
    // into the world: turned, sized, placed
    let yaw = r_y(inst.position_yaw.w);
    let scale = inst.scale_fade.x;
    let world = inst.position_yaw.xyz + yaw * (p * scale);
    let n = normalize(yaw * (rot * v.normal.xyz) + vec3<f32>(0.0, 0.0, 1e-8));
    let colour = mix(v.color.rgb * inst.color.rgb, vec3<f32>(1.0, 0.86, 0.62), inst.anim_b.y * 0.85);
    let base = index * 12u;
    out_vertices[base + 0u] = world.x;
    out_vertices[base + 1u] = world.y;
    out_vertices[base + 2u] = world.z;
    out_vertices[base + 3u] = n.x;
    out_vertices[base + 4u] = n.y;
    out_vertices[base + 5u] = n.z;
    out_vertices[base + 6u] = 0.0;
    out_vertices[base + 7u] = 0.0;
    out_vertices[base + 8u] = colour.r;
    out_vertices[base + 9u] = colour.g;
    out_vertices[base + 10u] = colour.b;
    out_vertices[base + 11u] = v.color.a * (1.0 - clamp(inst.scale_fade.y, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crowd_shader_is_valid() {
        let module = naga::front::wgsl::parse_str(CROWD_SHADER).expect("parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("valid");
    }

    #[test]
    fn parts_are_named_like_the_models() {
        assert_eq!(part::by_name("ARM_W"), part::ARM_W);
        assert_eq!(part::by_name("QLEG_BR"), part::QLEG_BR);
        assert_eq!(part::by_name("whatever"), part::BODY);
    }
}
