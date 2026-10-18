//! The 3D viewport: the engine's own renderer drawing the edited `Scene3D` into a texture that
//! egui shows in its tab; the orbit camera; projection, picking rays and triangle picking.

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use velvet_math::{Mat4, Vec3};
use velvet_render::{
    material_from_doc, Atmosphere3D, Camera3D, Celestial3D, EnvironmentFrame3D, GltfModel,
    Lighting3D, Material3D, Mesh3D, MeshDraw3D, MeshHandle, MeshVertex3D, PointLight3D,
    TextureHandle, Wgpu3DRenderer,
};
use velvet_scene3d::{flatten_scene, FlatNode3D, LightDoc, MaterialDoc, MeshKind3D, Scene3D};

/// The viewport texture's format (egui samples it, the 3D pass renders into it).
pub const VIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

// ------------------------------------------------------------------ boxes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn empty() -> Self {
        Self {
            min: Vec3::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vec3::new(f32::MIN, f32::MIN, f32::MIN),
        }
    }
    pub fn cube(half: f32) -> Self {
        Self {
            min: Vec3::new(-half, -half, -half),
            max: Vec3::new(half, half, half),
        }
    }
    pub fn add(&mut self, p: Vec3) {
        self.min = Vec3::new(
            self.min.x.min(p.x),
            self.min.y.min(p.y),
            self.min.z.min(p.z),
        );
        self.max = Vec3::new(
            self.max.x.max(p.x),
            self.max.y.max(p.y),
            self.max.z.max(p.z),
        );
    }
    pub fn merge(&mut self, other: &Aabb) {
        if other.is_valid() {
            self.add(other.min);
            self.add(other.max);
        }
    }
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && self.min.y <= self.max.y && self.min.z <= self.max.z
    }
    pub fn corners(&self) -> [Vec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            Vec3::new(a.x, a.y, a.z),
            Vec3::new(b.x, a.y, a.z),
            Vec3::new(b.x, b.y, a.z),
            Vec3::new(a.x, b.y, a.z),
            Vec3::new(a.x, a.y, b.z),
            Vec3::new(b.x, a.y, b.z),
            Vec3::new(b.x, b.y, b.z),
            Vec3::new(a.x, b.y, b.z),
        ]
    }
    /// The box around this one moved by `m`.
    pub fn transformed(&self, m: &Mat4) -> Aabb {
        let mut out = Aabb::empty();
        if self.is_valid() {
            for c in self.corners() {
                out.add(m.transform_point3(c));
            }
        }
        out
    }
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    pub fn radius(&self) -> f32 {
        (self.max - self.min).length() * 0.5
    }
    /// Distance along the ray to where it enters the box (0 when it starts inside).
    pub fn ray_hit(&self, origin: Vec3, dir: Vec3) -> Option<f32> {
        let (mut t0, mut t1) = (0.0f32, f32::MAX);
        for (o, d, lo, hi) in [
            (origin.x, dir.x, self.min.x, self.max.x),
            (origin.y, dir.y, self.min.y, self.max.y),
            (origin.z, dir.z, self.min.z, self.max.z),
        ] {
            if d.abs() < 1e-9 {
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let (a, b) = ((lo - o) / d, (hi - o) / d);
                let (a, b) = if a < b { (a, b) } else { (b, a) };
                t0 = t0.max(a);
                t1 = t1.min(b);
                if t0 > t1 {
                    return None;
                }
            }
        }
        Some(t0)
    }
}

/// The 12 edges of a box as corner index pairs.
pub const BOX_EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

/// Ray against triangle (Möller–Trumbore), both faces; the distance along the ray.
fn ray_triangle(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-5).then_some(t)
}

// ------------------------------------------------------------------ cameras
#[derive(Debug, Clone, Copy)]
pub struct OrbitCamera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub fov_deg: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 0.8, 0.0),
            yaw: 0.75,
            pitch: 0.42,
            distance: 10.0,
            fov_deg: 50.0,
        }
    }
}

impl OrbitCamera {
    /// An orbit camera at `position` looking at `target`.
    pub fn looking(position: Vec3, target: Vec3, fov_deg: f32) -> Self {
        let off = position - target;
        let distance = off.length().max(0.3);
        Self {
            target,
            yaw: off.x.atan2(off.z),
            pitch: (off.y / distance)
                .clamp(-1.0, 1.0)
                .asin()
                .clamp(-1.52, 1.52),
            distance,
            fov_deg,
        }
    }
    pub fn position(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + Vec3::new(cp * sy, sp, cp * cy) * self.distance
    }
    pub fn camera(&self) -> Camera3D {
        Camera3D {
            position: self.position(),
            target: self.target,
            up: Vec3::Y,
            fov_y_radians: self.fov_deg.to_radians(),
            orthographic_height: None,
            near: (self.distance * 0.004).clamp(0.02, 0.5),
            far: (self.distance * 60.0).clamp(400.0, 4000.0),
        }
    }
    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.52, 1.52);
    }
    pub fn pan(&mut self, dx: f32, dy: f32, height_px: f32) {
        let cam = self.camera();
        let (_, right, up) = basis(&cam);
        let wpp = world_per_pixel(&cam, self.target, height_px);
        self.target = self.target + right * (-dx * wpp) + up * (dy * wpp);
    }
    pub fn zoom(&mut self, scroll: f32) {
        let k = (1.0 - scroll * 0.0018).clamp(0.6, 1.6);
        self.distance = (self.distance * k).clamp(0.3, 1500.0);
    }
    /// Moves the camera (and what it looks at) in its own frame: forward, right, up.
    pub fn fly(&mut self, forward: f32, right: f32, up: f32) {
        let cam = self.camera();
        let (f, r, _) = basis(&cam);
        self.target = self.target + f * forward + r * right + Vec3::Y * up;
    }
    /// Frames a box: the target at its centre, far enough to see it whole.
    pub fn frame(&mut self, b: &Aabb) {
        if !b.is_valid() {
            return;
        }
        self.target = b.center();
        let r = b.radius().max(0.3);
        self.distance = (r / (self.fov_deg.to_radians() * 0.5).tan() * 1.15).clamp(1.0, 1200.0);
    }
}

/// Forward, right and up of a camera.
pub fn basis(cam: &Camera3D) -> (Vec3, Vec3, Vec3) {
    let forward = (cam.target - cam.position).normalize_or_zero();
    let right = forward.cross(cam.up).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    (forward, right, up)
}

/// World units per screen pixel at the depth of `at`.
pub fn world_per_pixel(cam: &Camera3D, at: Vec3, height_px: f32) -> f32 {
    if let Some(h) = cam.orthographic_height {
        return h / height_px.max(1.0);
    }
    let (f, _, _) = basis(cam);
    let d = (at - cam.position).dot(f).max(0.01);
    2.0 * d * (cam.fov_y_radians * 0.5).tan() / height_px.max(1.0)
}

/// Where `p` lands on the screen rect (None behind the camera).
pub fn project(vp: &Mat4, p: Vec3, rect: egui::Rect) -> Option<egui::Pos2> {
    let x = vp.x_axis[0] * p.x + vp.y_axis[0] * p.y + vp.z_axis[0] * p.z + vp.w_axis[0];
    let y = vp.x_axis[1] * p.x + vp.y_axis[1] * p.y + vp.z_axis[1] * p.z + vp.w_axis[1];
    let w = vp.x_axis[3] * p.x + vp.y_axis[3] * p.y + vp.z_axis[3] * p.z + vp.w_axis[3];
    if w <= 1e-4 {
        return None;
    }
    let (nx, ny) = (x / w, y / w);
    Some(egui::pos2(
        rect.left() + (nx * 0.5 + 0.5) * rect.width(),
        rect.top() + (0.5 - ny * 0.5) * rect.height(),
    ))
}

/// A picking ray through the screen point `pos`.
pub fn ray(cam: &Camera3D, pos: egui::Pos2, rect: egui::Rect) -> (Vec3, Vec3) {
    let nx = (pos.x - rect.left()) / rect.width().max(1.0) * 2.0 - 1.0;
    let ny = 1.0 - (pos.y - rect.top()) / rect.height().max(1.0) * 2.0;
    let aspect = rect.width() / rect.height().max(1.0);
    let (f, r, u) = basis(cam);
    if let Some(h) = cam.orthographic_height {
        let origin = cam.position + r * (nx * h * 0.5 * aspect) + u * (ny * h * 0.5);
        return (origin, f);
    }
    let t = (cam.fov_y_radians * 0.5).tan();
    let dir = (f + r * (nx * t * aspect) + u * (ny * t)).normalize_or_zero();
    (cam.position, dir)
}

pub fn camera_from_doc(scene: &Scene3D) -> Camera3D {
    let c = &scene.camera;
    Camera3D {
        position: Vec3::new(c.position[0], c.position[1], c.position[2]),
        target: Vec3::new(c.target[0], c.target[1], c.target[2]),
        up: Vec3::Y,
        fov_y_radians: c.fov_deg.clamp(5.0, 170.0).to_radians(),
        orthographic_height: c.orthographic_height,
        near: c.near.max(0.001),
        far: c.far.max(c.near + 0.01),
    }
}

// ------------------------------------------------------------------ grid
const GRID_HALF: i32 = 25;

/// The floor grid as thin quads on y = 0 (both faces), one metre apart, every fifth line
/// brighter, the X axis red and the Z axis blue.
fn grid_mesh() -> Mesh3D {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let y = 0.004;
    let mut line = |a: [f32; 2], b: [f32; 2], w: f32, color: [f32; 4]| {
        let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dz * dz).sqrt().max(1e-6);
        let (nx, nz) = (-dz / len * w * 0.5, dx / len * w * 0.5);
        let base = vertices.len() as u32;
        for (x, z) in [
            (a[0] + nx, a[1] + nz),
            (a[0] - nx, a[1] - nz),
            (b[0] - nx, b[1] - nz),
            (b[0] + nx, b[1] + nz),
        ] {
            let mut v = MeshVertex3D::new([x, y, z], [0.0, 1.0, 0.0], [0.0, 0.0]);
            v.color = color;
            vertices.push(v);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        indices.extend_from_slice(&[base, base + 2, base + 1, base, base + 3, base + 2]);
    };
    let h = GRID_HALF as f32;
    let minor = [0.19, 0.20, 0.23, 1.0];
    let major = [0.30, 0.31, 0.36, 1.0];
    for i in -GRID_HALF..=GRID_HALF {
        let f = i as f32;
        if i == 0 {
            continue;
        }
        let (w, c) = if i % 5 == 0 {
            (0.03, major)
        } else {
            (0.016, minor)
        };
        line([f, -h], [f, h], w, c);
        line([-h, f], [h, f], w, c);
    }
    line([-h, 0.0], [h, 0.0], 0.045, [0.80, 0.22, 0.28, 1.0]);
    line([0.0, -h], [0.0, h], 0.045, [0.24, 0.46, 0.88, 1.0]);
    Mesh3D {
        vertices,
        indices,
        material: Material3D::rgb(1.0, 1.0, 1.0),
        metallic_factor: 0.0,
        roughness_factor: 1.0,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

// ------------------------------------------------------------------ GPU side
struct Part {
    mesh: MeshHandle,
    texture: Option<TextureHandle>,
    local: Mat4,
    material: Material3D,
    /// The model's mesh this part draws (for re-posing a skinned one).
    mesh_index: usize,
    skinned: bool,
}

/// A skinned model's skeleton and clips, kept to pose it (rest pose = its first idle frame).
struct Rig {
    gltf: GltfModel,
    clips: Vec<String>,
    durations: Vec<f32>,
    /// The clip shown: `None` is the bind pose.
    current: Option<usize>,
    /// What the GPU skins hold now: (clip, time).
    applied: Option<(Option<usize>, f32)>,
}

/// The clip a character stands in: an idle one, else the first frame of a walk, else the first.
fn default_clip(names: &[String]) -> Option<usize> {
    let find = |word: &str| names.iter().position(|n| n.to_lowercase().contains(word));
    find("idle")
        .or_else(|| find("stand"))
        .or_else(|| find("walk"))
        .or(if names.is_empty() { None } else { Some(0) })
}

/// A model's triangles kept on the CPU (model space) for picking.
struct CpuPart {
    bounds: Aabb,
    positions: Vec<Vec3>,
    indices: Vec<u32>,
}

struct Model {
    parts: Vec<Part>,
    cpu: Vec<CpuPart>,
    bounds: Aabb,
    triangles: usize,
    rig: Option<Rig>,
}

/// What a loaded model reports to the log.
pub struct ModelInfo {
    pub path: String,
    pub parts: usize,
    pub triangles: usize,
    pub textures: usize,
    pub size: [f32; 3],
    pub millis: u128,
}

/// The viewport's GPU state: its texture, the 3D renderer and what it has uploaded.
pub struct Viewport {
    renderer: Wgpu3DRenderer,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    pub size: (u32, u32),
    pub tex_id: Option<egui::TextureId>,
    prims: HashMap<String, (MeshHandle, Aabb, usize)>,
    models: HashMap<String, Result<Model, String>>,
    grid: Option<MeshHandle>,
    /// Last frame: the flattened nodes and each one's world box (for picking and overlays).
    pub flat: Vec<FlatNode3D>,
    pub bounds: HashMap<String, Aabb>,
    pub triangles: usize,
    pub draws: usize,
    pub frame_ms: f32,
    /// Animation preview: playing, and the clock it plays on.
    pub playing: bool,
    clock: f32,
    last_render: Option<Instant>,
}

fn create_target(device: &wgpu::Device, size: (u32, u32)) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-editor-viewport"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: VIEW_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

impl Viewport {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, msaa: u32) -> Self {
        let size = (960, 540);
        let (texture, view) = create_target(device, size);
        let renderer = Wgpu3DRenderer::new_with_msaa(device, queue, VIEW_FORMAT, size, msaa);
        Self {
            renderer,
            texture,
            view,
            size,
            tex_id: None,
            prims: HashMap::new(),
            models: HashMap::new(),
            grid: None,
            flat: Vec::new(),
            bounds: HashMap::new(),
            triangles: 0,
            draws: 0,
            frame_ms: 0.0,
            playing: false,
            clock: 0.0,
            last_render: None,
        }
    }

    /// Matches the texture to the viewport tab's size in pixels and registers it with egui.
    pub fn ensure_size(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        size: (u32, u32),
    ) {
        let size = (size.0.clamp(16, 8192), size.1.clamp(16, 8192));
        if size != self.size {
            let (texture, view) = create_target(device, size);
            self.texture = texture;
            self.view = view;
            self.size = size;
            self.renderer.resize(device, size);
            if let Some(id) = self.tex_id {
                egui_renderer.update_egui_texture_from_wgpu_texture(
                    device,
                    &self.view,
                    wgpu::FilterMode::Linear,
                    id,
                );
            }
        }
        if self.tex_id.is_none() {
            self.tex_id = Some(egui_renderer.register_native_texture(
                device,
                &self.view,
                wgpu::FilterMode::Linear,
            ));
        }
    }

    fn prim(
        &mut self,
        device: &wgpu::Device,
        key: &str,
        make: impl FnOnce() -> Mesh3D,
    ) -> Option<(MeshHandle, Aabb, usize)> {
        if let Some(p) = self.prims.get(key) {
            return Some(*p);
        }
        let mesh = make();
        let mut b = Aabb::empty();
        for v in &mesh.vertices {
            b.add(Vec3::new(v.position[0], v.position[1], v.position[2]));
        }
        let handle = self.renderer.upload_mesh(device, &mesh).ok()?;
        let tris = mesh.indices.len() / 3;
        self.prims.insert(key.to_string(), (handle, b, tris));
        Some((handle, b, tris))
    }

    fn load_model(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        root: &Path,
        path: &str,
    ) -> Result<(Model, usize), String> {
        let mut model = GltfModel::load(root.join(path)).map_err(|e| e.to_string())?;
        let textures: Vec<Option<TextureHandle>> = model
            .textures
            .iter()
            .map(|t| self.renderer.upload_texture(device, queue, t).ok())
            .collect();
        let clip = default_clip(&model.animations);
        let rest = clip
            .and_then(|c| model.pose(c, 0.0))
            .unwrap_or_else(|| model.rest_pose());
        let mut parts = Vec::new();
        let mut cpu = Vec::new();
        let mut bounds = Aabb::empty();
        let mut triangles = 0;
        for (i, mesh) in model.meshes.iter().enumerate() {
            // Skinned characters get a GPU skin, posed like the game stands them (the first frame
            // of their idle); the rest upload as they are.
            let handle = self
                .renderer
                .upload_skinned_mesh(device, mesh)
                .map_err(|e| e.to_string())?;
            let joints = model.joint_matrices(i, &rest);
            if let Some(joints) = &joints {
                let _ = self.renderer.set_skin_joints(queue, handle, joints);
            }
            let local = model.mesh_transform(i);
            let texture = mesh
                .texture_index
                .and_then(|t| textures.get(t).copied().flatten());
            let mut material = mesh.material;
            material.texture = texture;
            // The CPU copy (for picking and the box) skinned into the same pose.
            let positions: Vec<Vec3> = mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(vi, v)| {
                    let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
                    let posed = match (&joints, &mesh.skinning) {
                        (Some(j), Some(s)) => {
                            let mut acc = Vec3::ZERO;
                            let mut total = 0.0;
                            if let (Some(ids), Some(ws)) = (s.joints.get(vi), s.weights.get(vi)) {
                                for k in 0..4 {
                                    if let (true, Some(m)) = (ws[k] > 0.0, j.get(ids[k] as usize)) {
                                        acc += m.transform_point3(p) * ws[k];
                                        total += ws[k];
                                    }
                                }
                            }
                            if total > 0.0 {
                                acc * (1.0 / total)
                            } else {
                                p
                            }
                        }
                        _ => p,
                    };
                    local.transform_point3(posed)
                })
                .collect();
            let mut part_bounds = Aabb::empty();
            for p in &positions {
                part_bounds.add(*p);
            }
            bounds.merge(&part_bounds);
            triangles += mesh.indices.len() / 3;
            cpu.push(CpuPart {
                bounds: part_bounds,
                positions,
                indices: mesh.indices.clone(),
            });
            parts.push(Part {
                mesh: handle,
                texture,
                local,
                material,
                mesh_index: i,
                skinned: mesh.skinning.is_some(),
            });
        }
        if parts.is_empty() {
            return Err("el modelo no tiene mallas".into());
        }
        let texture_count = textures.iter().flatten().count();
        let rig = if parts.iter().any(|p| p.skinned) && !model.animations.is_empty() {
            // The pixels are on the GPU now; keep only the skeleton and the clips.
            model.textures.clear();
            Some(Rig {
                clips: model.animations.clone(),
                durations: model
                    .animation_clips
                    .iter()
                    .map(|c| c.duration.max(0.01))
                    .collect(),
                current: clip,
                applied: Some((clip, 0.0)),
                gltf: model,
            })
        } else {
            None
        };
        Ok((
            Model {
                parts,
                cpu,
                bounds,
                triangles,
                rig,
            },
            texture_count,
        ))
    }

    /// Forgets the uploaded models so they load again from disk.
    pub fn reload_models(&mut self) {
        self.models.clear();
    }

    /// A model's animation clips and the one shown (`None` = bind pose), if it has a skeleton.
    pub fn clips(&self, path: &str) -> Option<(Vec<String>, Option<usize>)> {
        let rig = self.models.get(path)?.as_ref().ok()?.rig.as_ref()?;
        Some((rig.clips.clone(), rig.current))
    }

    /// Shows clip `clip` on every node that uses the model at `path`.
    pub fn set_clip(&mut self, path: &str, clip: Option<usize>) {
        if let Some(Ok(Model { rig: Some(rig), .. })) = self.models.get_mut(path) {
            rig.current = clip;
        }
    }

    /// Whether any loaded model can be animated.
    pub fn has_rigs(&self) -> bool {
        self.models
            .values()
            .any(|m| matches!(m, Ok(Model { rig: Some(_), .. })))
    }

    /// Poses the skinned models: their chosen clip at the preview clock (frame 0 when paused).
    fn pose_rigs(&mut self, queue: &wgpu::Queue) {
        let now = Instant::now();
        let dt = self
            .last_render
            .map_or(0.0, |t| now.duration_since(t).as_secs_f32().min(0.1));
        self.last_render = Some(now);
        if self.playing {
            self.clock += dt;
        } else {
            self.clock = 0.0;
        }
        for model in self.models.values_mut().flatten() {
            let Some(rig) = model.rig.as_mut() else {
                continue;
            };
            let t = match rig.current {
                Some(c) => self.clock % rig.durations.get(c).copied().unwrap_or(1.0),
                None => 0.0,
            };
            if rig.applied == Some((rig.current, t)) {
                continue;
            }
            let pose = match rig.current {
                Some(c) => rig.gltf.pose(c, t).unwrap_or_else(|| rig.gltf.rest_pose()),
                None => rig.gltf.rest_pose(),
            };
            for part in model.parts.iter().filter(|p| p.skinned) {
                if let Some(joints) = rig.gltf.joint_matrices(part.mesh_index, &pose) {
                    let _ = self.renderer.set_skin_joints(queue, part.mesh, &joints);
                }
            }
            rig.applied = Some((rig.current, t));
        }
    }

    /// Draws `scene` from `camera` into the viewport texture. Returns the problems loading
    /// models (once each); what loaded goes to `loaded`, for the log.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        root: &Path,
        scene: &Scene3D,
        camera: Camera3D,
        show_grid: bool,
        loaded: &mut Vec<ModelInfo>,
    ) -> Vec<String> {
        let started = Instant::now();
        self.pose_rigs(queue);
        let mut problems = Vec::new();
        let flat = flatten_scene(scene);
        let mut draws = Vec::with_capacity(flat.len() + 1);
        let mut bounds = HashMap::with_capacity(flat.len());
        let mut triangles = 0usize;
        let neutral = material_from_doc(&MaterialDoc::default());
        for node in &flat {
            let material = material_from_doc(&node.material);
            let prim = match &node.mesh {
                MeshKind3D::Cube => self.prim(device, "cube", || Mesh3D::cube(1.0, neutral)),
                MeshKind3D::Plane => self.prim(device, "plane", || Mesh3D::plane(1.0, neutral)),
                MeshKind3D::Sphere { segments } => {
                    let s = (*segments as usize).clamp(3, 64);
                    self.prim(device, &format!("sphere:{s}"), || {
                        Mesh3D::uv_sphere(1.0, s, (s / 2).max(3), neutral)
                    })
                }
                MeshKind3D::Heightfield {
                    resolution,
                    size,
                    heights,
                } => {
                    let sum: f32 = heights
                        .iter()
                        .enumerate()
                        .map(|(i, h)| h * (1.0 + i as f32 * 0.001))
                        .sum();
                    let key = format!("hf:{resolution}:{size}:{}:{}", heights.len(), sum.to_bits());
                    self.prim(device, &key, || {
                        Mesh3D::heightfield(*resolution as usize, *size, heights, neutral)
                    })
                }
                MeshKind3D::Empty => {
                    bounds.insert(node.id.clone(), Aabb::cube(0.15).transformed(&node.world));
                    None
                }
                MeshKind3D::Gltf { path } => {
                    if !self.models.contains_key(path) {
                        let t0 = Instant::now();
                        let entry = match self.load_model(device, queue, root, path) {
                            Ok((model, textures)) => {
                                let s = model.bounds.max - model.bounds.min;
                                loaded.push(ModelInfo {
                                    path: path.clone(),
                                    parts: model.parts.len(),
                                    triangles: model.triangles,
                                    textures,
                                    size: [s.x, s.y, s.z],
                                    millis: t0.elapsed().as_millis(),
                                });
                                Ok(model)
                            }
                            Err(e) => {
                                problems.push(format!("No se pudo cargar «{path}»: {e}"));
                                Err(e)
                            }
                        };
                        self.models.insert(path.clone(), entry);
                    }
                    if let Some(Ok(model)) = self.models.get(path) {
                        for part in &model.parts {
                            draws.push(MeshDraw3D {
                                mesh: part.mesh,
                                transform: node.world * part.local,
                                material: Material3D {
                                    base_color: mul4(
                                        part.material.base_color,
                                        node.material.base_color,
                                    ),
                                    ..part.material
                                },
                                texture: part.texture,
                            });
                        }
                        triangles += model.triangles;
                        bounds.insert(node.id.clone(), model.bounds.transformed(&node.world));
                    } else {
                        bounds.insert(node.id.clone(), Aabb::cube(0.5).transformed(&node.world));
                    }
                    None
                }
            };
            if let Some((mesh, local_bounds, tris)) = prim {
                triangles += tris;
                draws.push(MeshDraw3D {
                    mesh,
                    transform: node.world,
                    material,
                    texture: None,
                });
                bounds.insert(node.id.clone(), local_bounds.transformed(&node.world));
            }
        }
        if show_grid {
            if self.grid.is_none() {
                self.grid = self.renderer.upload_mesh(device, &grid_mesh()).ok();
            }
            if let Some(grid) = self.grid {
                draws.push(MeshDraw3D {
                    mesh: grid,
                    transform: Mat4::IDENTITY,
                    material: Material3D::rgb(1.0, 1.0, 1.0),
                    texture: None,
                });
            }
        }

        let mut lighting = Lighting3D::default();
        let mut points = Vec::new();
        let mut has_key = false;
        for light in &scene.lights {
            match light {
                LightDoc::Directional {
                    direction,
                    key_color,
                    ambient_color,
                    shadow_extent,
                } if !has_key => {
                    has_key = true;
                    lighting.direction_to_light =
                        Vec3::new(direction[0], direction[1], direction[2]).normalize_or_zero();
                    lighting.key_color = *key_color;
                    lighting.ambient_color = *ambient_color;
                    lighting.shadow_extent = *shadow_extent;
                }
                LightDoc::Point {
                    position,
                    color,
                    intensity,
                    radius,
                } => points.push(PointLight3D::new(
                    Vec3::new(position[0], position[1], position[2]),
                    *color,
                    *intensity,
                    *radius,
                )),
                _ => {}
            }
        }
        let env = &scene.environment;
        let frame = EnvironmentFrame3D {
            lighting,
            atmosphere: Atmosphere3D {
                fog_color: env.fog_color,
                fog_start: env.fog_start,
                fog_end: env.fog_end,
                fog_density: env.fog_density,
                ..Atmosphere3D::default()
            },
            clear_color: [0.105, 0.115, 0.14, 1.0],
            celestial: Celestial3D::default(),
        };
        self.renderer.render_with_environment_and_lights(
            device, queue, &self.view, self.size, camera, frame, &points, &draws,
        );
        self.draws = draws.len();
        self.triangles = triangles;
        self.flat = flat;
        self.bounds = bounds;
        self.frame_ms = started.elapsed().as_secs_f32() * 1000.0;
        problems
    }

    /// The nearest node under a ray: exact triangles for models, boxes for the primitives.
    pub fn pick(&self, origin: Vec3, dir: Vec3) -> Option<(String, f32)> {
        let mut best: Option<(String, f32)> = None;
        for node in &self.flat {
            let Some(b) = self.bounds.get(&node.id) else {
                continue;
            };
            let Some(t_box) = b.ray_hit(origin, dir) else {
                continue;
            };
            if best.as_ref().is_some_and(|(_, bt)| t_box > *bt) {
                continue;
            }
            let t = match &node.mesh {
                MeshKind3D::Gltf { path } => match self.models.get(path) {
                    Some(Ok(model)) => match ray_model(model, &node.world, origin, dir) {
                        Some(t) => t,
                        None => continue,
                    },
                    _ => t_box,
                },
                _ => t_box,
            };
            if best.as_ref().is_none_or(|(_, bt)| t < *bt) {
                best = Some((node.id.clone(), t));
            }
        }
        best
    }

    /// Where a ray first meets the scene's models (or the ground plane): to drop things on.
    pub fn surface_point(&self, origin: Vec3, dir: Vec3) -> Option<Vec3> {
        if let Some((_, t)) = self.pick(origin, dir) {
            return Some(origin + dir * t);
        }
        if dir.y.abs() > 1e-5 {
            let t = -origin.y / dir.y;
            if t > 0.0 {
                return Some(origin + dir * t);
            }
        }
        None
    }

    /// The box around everything drawn.
    pub fn scene_bounds(&self) -> Aabb {
        let mut b = Aabb::empty();
        for v in self.bounds.values() {
            b.merge(v);
        }
        b
    }

    /// The viewport texture (for captures).
    #[allow(dead_code)]
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

fn mul4(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]]
}

fn ray_model(model: &Model, world: &Mat4, origin: Vec3, dir: Vec3) -> Option<f32> {
    // The same affine map on both ends keeps the ray parameter: a local hit at t is a world hit at t.
    let inv = world.inverse()?;
    let o = inv.transform_point3(origin);
    let d = inv.transform_vector3(dir);
    let mut best = f32::MAX;
    for part in &model.cpu {
        match part.bounds.ray_hit(o, d) {
            Some(t) if t < best => {}
            _ => continue,
        }
        for tri in part.indices.as_chunks::<3>().0 {
            let (Some(a), Some(b), Some(c)) = (
                part.positions.get(tri[0] as usize),
                part.positions.get(tri[1] as usize),
                part.positions.get(tri[2] as usize),
            ) else {
                continue;
            };
            if let Some(t) = ray_triangle(o, d, *a, *b, *c) {
                best = best.min(t);
            }
        }
    }
    (best < f32::MAX).then_some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rays_hit_boxes_and_triangles() {
        let b = Aabb::cube(1.0);
        let t = b
            .ray_hit(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
            .unwrap();
        assert!((t - 4.0).abs() < 1e-4);
        assert!(b
            .ray_hit(Vec3::new(3.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0))
            .is_none());
        let t = ray_triangle(
            Vec3::new(0.2, 0.2, 3.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
        .unwrap();
        assert!((t - 3.0).abs() < 1e-4);
    }

    #[test]
    fn orbit_camera_round_trips_a_view() {
        let cam = OrbitCamera::looking(Vec3::new(4.0, 3.0, 5.0), Vec3::new(0.0, 1.0, 0.0), 45.0);
        let p = cam.position();
        assert!((p - Vec3::new(4.0, 3.0, 5.0)).length() < 1e-3, "{p:?}");
    }
}
