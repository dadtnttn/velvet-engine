//! Skinning on the GPU.
//!
//! A skinned mesh keeps its bind pose (positions and normals) and its influences (four joints and
//! four weights a vertex) in storage buffers; each frame the game hands over its joint matrices
//! ([`crate::Wgpu3DRenderer::set_skin_joints`]) and a compute pass writes the deformed positions and
//! normals straight into the mesh's own vertex buffer. Everything that draws meshes (shading,
//! shadows, ink lines, culling) sees an ordinary mesh: nothing else needs to know.
//!
//! On the CPU only the pose is evaluated (a few dozen joints): [`Pose3D`] samples clips, blends
//! two of them for a cross-fade and turns into joint matrices; [`Animator3D`] plays clips with
//! fades. A hundred characters cost the CPU about what one cost when it deformed every vertex.

use bytemuck::{Pod, Zeroable};
use velvet_math::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use crate::mesh3d::{Mesh3D, MeshHandle, MeshVertex3D};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BindVertex {
    position: [f32; 4],
    normal: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Influence {
    /// Four joint indices, two a word.
    joints: [u32; 2],
    _pad: [u32; 2],
    weights: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SkinParams {
    vertices: u32,
    joints: u32,
    _pad: [u32; 2],
}

pub(crate) struct GpuSkin {
    joints: wgpu::Buffer,
    group: wgpu::BindGroup,
    vertex_count: u32,
    pub(crate) joint_count: usize,
    /// Per joint, the bind-space bounds of the vertices it moves (for culling the posed mesh).
    joint_bounds: Vec<Option<[[f32; 3]; 2]>>,
    dirty: bool,
}

pub(crate) struct Skinner3D {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    /// Skins by index; a freed one leaves a gap the next skin takes.
    pub(crate) skins: Vec<Option<GpuSkin>>,
}

const WORKGROUP: u32 = 64;

impl Skinner3D {
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
            label: Some("velvet-skin-layout"),
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
                entry(3, storage(true)),
                entry(4, storage(false)),
            ],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-skin"),
            source: wgpu::ShaderSource::Wgsl(SKIN_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-skin"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("velvet-skin"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("cs_skin"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self {
            layout,
            pipeline,
            skins: Vec::new(),
        }
    }

    /// Registers a skinned mesh whose vertex buffer (`vertex_buffer`, storage-capable) the pass
    /// will write. Returns the skin's index.
    pub(crate) fn add(
        &mut self,
        device: &wgpu::Device,
        mesh: MeshHandle,
        source: &Mesh3D,
        vertex_buffer: &wgpu::Buffer,
    ) -> Option<usize> {
        let skinning = source.skinning.as_ref()?;
        let joint_count = skinning
            .joints
            .iter()
            .flat_map(|j| j.iter())
            .map(|&j| j as usize + 1)
            .max()
            .unwrap_or(1)
            .max(1);
        let bind: Vec<BindVertex> = source
            .vertices
            .iter()
            .map(|v| BindVertex {
                position: [v.position[0], v.position[1], v.position[2], 1.0],
                normal: [v.normal[0], v.normal[1], v.normal[2], 0.0],
            })
            .collect();
        let influences: Vec<Influence> = (0..source.vertices.len())
            .map(|i| {
                let j = skinning.joints.get(i).copied().unwrap_or([0; 4]);
                let w = skinning.weights.get(i).copied().unwrap_or([0.0; 4]);
                Influence {
                    joints: [
                        j[0] as u32 | (j[1] as u32) << 16,
                        j[2] as u32 | (j[3] as u32) << 16,
                    ],
                    _pad: [0; 2],
                    weights: w,
                }
            })
            .collect();
        let mut joint_bounds = vec![None::<[[f32; 3]; 2]>; joint_count];
        for (i, v) in source.vertices.iter().enumerate() {
            let (Some(j), Some(w)) = (skinning.joints.get(i), skinning.weights.get(i)) else {
                continue;
            };
            for slot in 0..4 {
                if w[slot] <= 0.05 {
                    continue;
                }
                let b = joint_bounds[j[slot] as usize].get_or_insert([v.position, v.position]);
                let [low, high] = b;
                for ((low, high), coordinate) in low.iter_mut().zip(high.iter_mut()).zip(v.position)
                {
                    *low = low.min(coordinate);
                    *high = high.max(coordinate);
                }
            }
        }
        let params = SkinParams {
            vertices: bind.len() as u32,
            joints: joint_count as u32,
            _pad: [0; 2],
        };
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-skin-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-skin-bind"),
            contents: bytemuck::cast_slice(&bind),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let influence_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-skin-influences"),
            contents: bytemuck::cast_slice(&influences),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let identity: Vec<[f32; 16]> = vec![Mat4::IDENTITY.to_cols_array(); joint_count];
        let joints = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-skin-joints"),
            contents: bytemuck::cast_slice(&identity),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-skin-group"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bind_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: influence_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: joints.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: vertex_buffer.as_entire_binding(),
                },
            ],
        });
        let _ = mesh;
        let skin = GpuSkin {
            joints,
            group,
            vertex_count: bind.len() as u32,
            joint_count,
            joint_bounds,
            dirty: false,
        };
        match self.skins.iter().position(Option::is_none) {
            Some(free) => {
                self.skins[free] = Some(skin);
                Some(free)
            }
            None => {
                self.skins.push(Some(skin));
                Some(self.skins.len() - 1)
            }
        }
    }

    /// Rebuilds skin `index` for a new mesh (same handle, new buffers); returns the index.
    pub(crate) fn replace(
        &mut self,
        device: &wgpu::Device,
        index: usize,
        mesh: MeshHandle,
        source: &Mesh3D,
        vertex_buffer: &wgpu::Buffer,
    ) -> Option<usize> {
        self.remove(index);
        self.add(device, mesh, source, vertex_buffer)
    }

    /// Frees a skin (its mesh no longer needs one).
    pub(crate) fn remove(&mut self, index: usize) {
        if let Some(slot) = self.skins.get_mut(index) {
            *slot = None;
        }
    }

    /// Skins alive (for tests and diagnostics).
    #[allow(dead_code)]
    pub(crate) fn live(&self) -> usize {
        self.skins.iter().flatten().count()
    }

    /// New joint matrices for a skin; returns the posed mesh's bounds.
    pub(crate) fn set_joints(
        &mut self,
        queue: &wgpu::Queue,
        skin: usize,
        joints: &[Mat4],
    ) -> Option<[[f32; 3]; 2]> {
        let s = self.skins.get_mut(skin)?.as_mut()?;
        let n = joints.len().min(s.joint_count);
        let data: Vec<[f32; 16]> = joints[..n].iter().map(|m| m.to_cols_array()).collect();
        queue.write_buffer(&s.joints, 0, bytemuck::cast_slice(&data));
        s.dirty = true;
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for (j, bounds) in s.joint_bounds.iter().enumerate() {
            let (Some(b), Some(m)) = (bounds, joints.get(j)) else {
                continue;
            };
            for corner in 0..8 {
                let p = Vec3::new(
                    b[corner & 1][0],
                    b[(corner >> 1) & 1][1],
                    b[(corner >> 2) & 1][2],
                );
                let q = m.transform_point3(p);
                for (a, v) in [q.x, q.y, q.z].into_iter().enumerate() {
                    lo[a] = lo[a].min(v);
                    hi[a] = hi[a].max(v);
                }
            }
        }
        (lo[0] <= hi[0]).then_some([lo, hi])
    }

    /// Deforms every skin whose joints changed since the last frame.
    pub(crate) fn dispatch(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.skins.iter().flatten().any(|s| s.dirty) {
            return;
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("velvet-skin"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        for s in self.skins.iter_mut().flatten().filter(|s| s.dirty) {
            pass.set_bind_group(0, &s.group, &[]);
            pass.dispatch_workgroups(s.vertex_count.div_ceil(WORKGROUP), 1, 1);
            s.dirty = false;
        }
    }
}

/// The vertex buffer layout the compute pass writes: [`MeshVertex3D`] as plain floats.
const _: () = assert!(std::mem::size_of::<MeshVertex3D>() == 48);

pub(crate) const SKIN_SHADER: &str = r#"
struct Params {
    vertices: u32,
    joints: u32,
    _pad0: u32,
    _pad1: u32,
};
struct BindVertex {
    position: vec4<f32>,
    normal: vec4<f32>,
};
struct Influence {
    joints: vec2<u32>,
    _pad: vec2<u32>,
    weights: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> bind_pose: array<BindVertex>;
@group(0) @binding(2) var<storage, read> influences: array<Influence>;
@group(0) @binding(3) var<storage, read> joints: array<mat4x4<f32>>;
// The mesh's vertex buffer: 12 floats a vertex (position, normal, uv, colour).
@group(0) @binding(4) var<storage, read_write> out_vertices: array<f32>;

@compute @workgroup_size(64)
fn cs_skin(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.vertices) {
        return;
    }
    let v = bind_pose[i];
    let inf = influences[i];
    let js = array<u32, 4>(inf.joints.x & 0xffffu, inf.joints.x >> 16u, inf.joints.y & 0xffffu, inf.joints.y >> 16u);
    let ws = array<f32, 4>(inf.weights.x, inf.weights.y, inf.weights.z, inf.weights.w);
    var position = vec3<f32>(0.0);
    var normal = vec3<f32>(0.0);
    var total = 0.0;
    for (var k = 0u; k < 4u; k = k + 1u) {
        let w = ws[k];
        if (w <= 0.0 || js[k] >= params.joints) {
            continue;
        }
        let m = joints[js[k]];
        position = position + (m * v.position).xyz * w;
        normal = normal + (m * v.normal).xyz * w;
        total = total + w;
    }
    if (total <= 0.00001) {
        position = v.position.xyz;
        normal = v.normal.xyz;
    }
    let n = normalize(normal + vec3<f32>(0.0, 0.0, 1e-8));
    let base = i * 12u;
    out_vertices[base + 0u] = position.x;
    out_vertices[base + 1u] = position.y;
    out_vertices[base + 2u] = position.z;
    out_vertices[base + 3u] = n.x;
    out_vertices[base + 4u] = n.y;
    out_vertices[base + 5u] = n.z;
}
"#;

// ------------------------------------------------------------------------------------- poses

/// A skeleton's pose: every node's local translation, rotation and scale.
#[derive(Debug, Clone)]
pub struct Pose3D {
    pub(crate) nodes: Vec<crate::mesh3d::NodePose3D>,
}

impl Pose3D {
    /// This pose blended toward `other` by `t` (0 this, 1 the other): translations and scales
    /// interpolated, rotations along the short arc.
    pub fn blend(&self, other: &Pose3D, t: f32) -> Pose3D {
        let t = t.clamp(0.0, 1.0);
        let nodes = self
            .nodes
            .iter()
            .zip(&other.nodes)
            .map(|(a, b)| {
                let mut n = *a;
                n.translation = a.translation.lerp(b.translation, t);
                n.scale = a.scale.lerp(b.scale, t);
                n.rotation = nlerp(a.rotation, b.rotation, t);
                if a.matrix.is_some() || b.matrix.is_some() {
                    n.matrix = if t < 0.5 { a.matrix } else { b.matrix };
                }
                n
            })
            .collect();
        Pose3D { nodes }
    }
}

fn nlerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    if a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3] < 0.0 {
        b = b.map(|v| -v);
    }
    let q = [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t);
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
        .sqrt()
        .max(1e-6);
    q.map(|v| v / len)
}

/// Plays a model's clips with cross-fades: `play` a clip and the last one fades out under it.
#[derive(Debug, Clone)]
pub struct Animator3D {
    clip: usize,
    time: f32,
    speed: f32,
    previous: Option<(usize, f32)>,
    fade: f32,
    fade_length: f32,
}

impl Animator3D {
    /// Starts on `clip` at `time` seconds into it.
    pub fn new(clip: usize, time: f32) -> Self {
        Self {
            clip,
            time,
            speed: 1.0,
            previous: None,
            fade: 1.0,
            fade_length: 0.0,
        }
    }

    /// The clip playing (the one faded into).
    pub fn clip(&self) -> usize {
        self.clip
    }

    /// Seconds into the current clip.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Playback speed (1 as authored).
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    /// Switches to `clip` over `fade` seconds (nothing happens if it is already playing).
    pub fn play(&mut self, clip: usize, fade: f32) {
        if clip == self.clip {
            return;
        }
        self.previous = Some((self.clip, self.time));
        self.clip = clip;
        self.time = 0.0;
        self.fade_length = fade.max(0.0);
        self.fade = if fade > 0.0 { 0.0 } else { 1.0 };
    }

    /// Moves time on.
    pub fn advance(&mut self, dt: f32) {
        let dt = dt * self.speed;
        self.time += dt;
        if let Some((_, t)) = &mut self.previous {
            *t += dt;
        }
        if self.fade_length > 0.0 {
            self.fade = (self.fade + dt / self.fade_length).min(1.0);
        } else {
            self.fade = 1.0;
        }
        if self.fade >= 1.0 {
            self.previous = None;
        }
    }

    /// The pose now.
    pub fn pose(&self, model: &crate::mesh3d::GltfModel) -> Option<Pose3D> {
        let now = model.pose(self.clip, self.time)?;
        match self.previous {
            Some((clip, time)) if self.fade < 1.0 => {
                let before = model.pose(clip, time)?;
                // Eased, so the change of weight has no corner.
                let f = self.fade * self.fade * (3.0 - 2.0 * self.fade);
                Some(before.blend(&now, f))
            }
            _ => Some(now),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skin_shader_is_valid() {
        let module = naga::front::wgsl::parse_str(SKIN_SHADER).expect("parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("valid");
    }

    #[test]
    fn a_blend_takes_the_short_arc_and_ends_on_each_pose() {
        let q = [0.0, 0.0, 0.0, 1.0];
        let r = [
            0.0,
            -std::f32::consts::FRAC_1_SQRT_2,
            0.0,
            -std::f32::consts::FRAC_1_SQRT_2,
        ]; // a quarter turn, written with the far sign
        let half = nlerp(q, r, 0.5);
        assert!(half[3].abs() > 0.9, "the short way round: {half:?}");
        assert_eq!(nlerp(q, r, 0.0), q);
        let end = nlerp(q, r, 1.0);
        assert!(
            (end[1] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3
                && (end[3] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3
        );
    }

    /// What the compute pass does, on the CPU: the pose's joint matrices must deform the mesh
    /// exactly as the old CPU sampler did.
    #[test]
    fn joint_matrices_deform_like_the_cpu_sampler() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../DemonQueen/juego/nucleo/data/assets/3d/baths/spa_girl.glb");
        if !path.exists() {
            return;
        }
        let model = crate::mesh3d::GltfModel::load(path).expect("spa girl loads");
        let clip = model.animation_index("Spa_Swim").expect("swim clip");
        let cpu = model.sample_animation(0, clip, 0.7).expect("cpu sample");
        let pose = model.pose(clip, 0.7).expect("pose");
        let joints = model.joint_matrices(0, &pose).expect("joints");
        let mesh = &model.meshes[0];
        let skin = mesh.skinning.as_ref().unwrap();
        let mut worst = 0.0f32;
        for (i, v) in mesh.vertices.iter().enumerate() {
            let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
            let mut out = Vec3::ZERO;
            for k in 0..4 {
                if skin.weights[i][k] > 0.0 {
                    out +=
                        joints[skin.joints[i][k] as usize].transform_point3(p) * skin.weights[i][k];
                }
            }
            let c = cpu[i].position;
            worst = worst.max((out - Vec3::new(c[0], c[1], c[2])).length());
        }
        assert!(worst < 1e-4, "same deformation: off by {worst}");
        // A cross-fade halfway sits between the two clips.
        let sit = model.animation_index("Spa_Sit").unwrap();
        let a = model.pose(sit, 0.0).unwrap();
        let mixed = a.blend(&pose, 0.5);
        assert_eq!(mixed.nodes.len(), pose.nodes.len());
    }

    /// CPU cost of one character a frame: the old way (deform every vertex) against the new one
    /// (pose the joints; the GPU deforms). `cargo test -p velvet-render --release skin_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn skin_cost() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../DemonQueen/juego/nucleo/data/assets/3d/characters/queen_chibi_v2_animated.glb");
        let model = crate::mesh3d::GltfModel::load(path).expect("queen loads");
        let clip = model.animation_index("Queen_Walk").unwrap();
        let verts: usize = model.meshes.iter().map(|m| m.vertices.len()).sum();
        let n = 200;
        let t0 = std::time::Instant::now();
        for k in 0..n {
            for i in 0..model.meshes.len() {
                std::hint::black_box(model.sample_animation(i, clip, k as f32 * 0.016));
            }
        }
        let cpu = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
        let t0 = std::time::Instant::now();
        for k in 0..n {
            let pose = model.pose(clip, k as f32 * 0.016).unwrap();
            for i in 0..model.meshes.len() {
                std::hint::black_box(model.joint_matrices(i, &pose));
            }
        }
        let gpu = t0.elapsed().as_secs_f64() * 1000.0 / n as f64;
        println!("SKIN queen: {verts} vertices; CPU skinning {cpu:.3} ms/frame, pose only {gpu:.3} ms/frame ({:.0}x)", cpu / gpu);
    }

    #[test]
    fn the_animator_fades_and_lets_go_of_the_old_clip() {
        let mut a = Animator3D::new(0, 0.0);
        a.play(1, 0.5);
        assert_eq!(a.clip(), 1);
        a.advance(0.25);
        assert!(a.previous.is_some() && (a.fade - 0.5).abs() < 1e-5);
        a.advance(0.3);
        assert!(a.previous.is_none());
        a.play(1, 0.5);
        assert!(
            a.previous.is_none(),
            "playing the same clip again changes nothing"
        );
    }
}
