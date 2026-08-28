//! GPU 3D primitives for Velvet Engine.
//!
//! This module is deliberately small and data-driven: game code owns scene
//! state, while [`Wgpu3DRenderer`] owns GPU buffers, depth testing, and the
//! lit mesh pipeline.  It is the foundation for replacing exploration
//! billboards with real meshes without coupling the story layer to `wgpu`.

use std::num::NonZeroU64;
use std::path::Path;

use bytemuck::{Pod, Zeroable};
use thiserror::Error;
use velvet_math::{Mat4, Vec3};
use wgpu::util::DeviceExt;

/// A vertex consumed by the built-in lit mesh pipeline.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MeshVertex3D {
    /// Object-space position.
    pub position: [f32; 3],
    /// Object-space normal.
    pub normal: [f32; 3],
    /// UV coordinates reserved for textured materials.
    pub uv: [f32; 2],
    /// Per-vertex color multiplier.
    pub color: [f32; 4],
}

impl MeshVertex3D {
    /// Create a vertex with a white color multiplier.
    pub fn new(position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            uv,
            color: [1.0; 4],
        }
    }
}

/// CPU-side indexed mesh.
#[derive(Debug, Clone)]
pub struct Mesh3D {
    /// Interleaved vertex data.
    pub vertices: Vec<MeshVertex3D>,
    /// Triangle-list indices.
    pub indices: Vec<u32>,
    /// Default material used when the mesh is drawn.
    pub material: Material3D,
    /// Optional index into [`GltfModel::textures`] for imported meshes.
    pub texture_index: Option<usize>,
    /// Optional skin weights for CPU skeletal sampling.
    pub skinning: Option<SkinWeights3D>,
}

impl Mesh3D {
    /// Build a horizontal, Y-up plane centered on the origin.
    pub fn plane(size: f32, material: Material3D) -> Self {
        let h = size.max(0.01) * 0.5;
        Self {
            vertices: vec![
                MeshVertex3D::new([-h, 0.0, -h], [0.0, 1.0, 0.0], [0.0, 0.0]),
                MeshVertex3D::new([h, 0.0, -h], [0.0, 1.0, 0.0], [1.0, 0.0]),
                MeshVertex3D::new([h, 0.0, h], [0.0, 1.0, 0.0], [1.0, 1.0]),
                MeshVertex3D::new([-h, 0.0, h], [0.0, 1.0, 0.0], [0.0, 1.0]),
            ],
            indices: vec![0, 2, 1, 0, 3, 2],
            material,
            texture_index: None,
            skinning: None,
        }
    }

    /// Build a flat ring on the XZ plane. Useful as a player/selection marker.
    pub fn ring(
        outer_radius: f32,
        inner_radius: f32,
        segments: usize,
        material: Material3D,
    ) -> Self {
        let outer_radius = outer_radius.max(0.01);
        let inner_radius = inner_radius.clamp(0.0, outer_radius * 0.98);
        let segments = segments.max(3);
        let mut vertices = Vec::with_capacity(segments * 2);
        let mut indices = Vec::with_capacity(segments * 6);
        let tau = std::f32::consts::TAU;
        for segment in 0..segments {
            let angle = segment as f32 / segments as f32 * tau;
            let (sin, cos) = angle.sin_cos();
            vertices.push(MeshVertex3D::new(
                [outer_radius * cos, 0.0, outer_radius * sin],
                [0.0, 1.0, 0.0],
                [1.0, 0.0],
            ));
            vertices.push(MeshVertex3D::new(
                [inner_radius * cos, 0.0, inner_radius * sin],
                [0.0, 1.0, 0.0],
                [0.0, 0.0],
            ));
        }
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let outer = (segment * 2) as u32;
            let inner = outer + 1;
            let next_outer = (next * 2) as u32;
            let next_inner = next_outer + 1;
            indices.extend_from_slice(&[outer, inner, next_inner, outer, next_inner, next_outer]);
        }
        Self {
            vertices,
            indices,
            material,
            texture_index: None,
            skinning: None,
        }
    }

    /// Build a unit cube centered on the origin.
    pub fn cube(size: f32, material: Material3D) -> Self {
        let h = size.max(0.01) * 0.5;
        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        let faces = [
            (
                [0.0, 1.0, 0.0],
                [[-h, h, -h], [h, h, -h], [h, h, h], [-h, h, h]],
            ),
            (
                [0.0, -1.0, 0.0],
                [[-h, -h, h], [h, -h, h], [h, -h, -h], [-h, -h, -h]],
            ),
            (
                [0.0, 0.0, 1.0],
                [[-h, -h, h], [-h, h, h], [h, h, h], [h, -h, h]],
            ),
            (
                [0.0, 0.0, -1.0],
                [[h, -h, -h], [h, h, -h], [-h, h, -h], [-h, -h, -h]],
            ),
            (
                [1.0, 0.0, 0.0],
                [[h, -h, h], [h, h, h], [h, h, -h], [h, -h, -h]],
            ),
            (
                [-1.0, 0.0, 0.0],
                [[-h, -h, -h], [-h, h, -h], [-h, h, h], [-h, -h, h]],
            ),
        ];
        for (normal, corners) in faces {
            let base = vertices.len() as u32;
            vertices.extend(corners.into_iter().enumerate().map(|(i, position)| {
                MeshVertex3D::new(
                    position,
                    normal,
                    [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]][i],
                )
            }));
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self {
            vertices,
            indices,
            material,
            texture_index: None,
            skinning: None,
        }
    }
}

/// Simple material consumed by the built-in lit shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Material3D {
    /// Base RGBA color.
    pub base_color: [f32; 4],
    /// GPU texture override. `None` uses a white texture.
    pub texture: Option<TextureHandle>,
}

impl Material3D {
    /// Create an opaque material from RGB values in `0..=1`.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self {
            base_color: [r, g, b, 1.0],
            texture: None,
        }
    }
}

impl Default for Material3D {
    fn default() -> Self {
        Self::rgb(0.75, 0.70, 0.82)
    }
}

/// Position, Euler rotation (XYZ), and scale for a 3D node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3D {
    /// World position.
    pub position: Vec3,
    /// Rotation in radians around X, Y, and Z.
    pub rotation: Vec3,
    /// Non-uniform scale.
    pub scale: Vec3,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform3D {
    /// Convert TRS to a column-major matrix.
    pub fn matrix(self) -> Mat4 {
        Mat4::from_translation(self.position)
            * Mat4::from_rotation_z(self.rotation.z)
            * Mat4::from_rotation_y(self.rotation.y)
            * Mat4::from_rotation_x(self.rotation.x)
            * Mat4::from_scale(self.scale)
    }
}

/// Perspective or orthographic camera for Y-up 3D scenes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    /// Camera position in world space.
    pub position: Vec3,
    /// Point the camera looks at.
    pub target: Vec3,
    /// Up direction.
    pub up: Vec3,
    /// Vertical field of view in radians.
    pub fov_y_radians: f32,
    /// Orthographic vertical span in world units. `None` uses perspective.
    pub orthographic_height: Option<f32>,
    /// Near clip plane.
    pub near: f32,
    /// Far clip plane.
    pub far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: Vec3::new(4.5, 3.2, 5.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            up: Vec3::Y,
            fov_y_radians: 45.0_f32.to_radians(),
            orthographic_height: None,
            near: 0.05,
            far: 200.0,
        }
    }
}

impl Camera3D {
    /// View matrix.
    pub fn view_matrix(self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Projection matrix for a viewport aspect ratio.
    pub fn projection_matrix(self, aspect: f32) -> Mat4 {
        if let Some(height) = self.orthographic_height {
            let half_height = height.max(0.01) * 0.5;
            let half_width = half_height * aspect.max(1e-6);
            Mat4::orthographic_rh_zo(
                -half_width,
                half_width,
                -half_height,
                half_height,
                self.near,
                self.far,
            )
        } else {
            Mat4::perspective_rh(self.fov_y_radians, aspect, self.near, self.far)
        }
    }

    /// Combined view-projection matrix.
    pub fn view_projection(self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// Set an orbit camera around a target using azimuth, elevation, and distance.
    pub fn orbit(&mut self, target: Vec3, azimuth: f32, elevation: f32, distance: f32) {
        let elevation = elevation.clamp(-1.45, 1.45);
        let distance = distance.max(0.1);
        let (az_s, az_c) = azimuth.sin_cos();
        let (el_s, el_c) = elevation.sin_cos();
        self.target = target;
        self.position = target + Vec3::new(az_s * el_c, el_s, az_c * el_c) * distance;
    }
}

/// Directional key light plus ambient and ground-bounce colors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lighting3D {
    /// Normalized world-space direction from the surface toward the key light.
    pub direction_to_light: Vec3,
    /// Linear RGB intensity of the directional light.
    pub key_color: [f32; 3],
    /// Cool hemispherical ambient light.
    pub ambient_color: [f32; 3],
    /// Indirect color reflected upward from the ground.
    pub bounce_color: [f32; 3],
    /// Half-width of the directional shadow camera in world units.
    pub shadow_extent: f32,
}

impl Default for Lighting3D {
    fn default() -> Self {
        Self {
            direction_to_light: Vec3::new(-0.48, 0.82, 0.31),
            key_color: [1.05, 0.82, 0.62],
            ambient_color: [0.16, 0.12, 0.28],
            bounce_color: [0.15, 0.04, 0.20],
            shadow_extent: 10.0,
        }
    }
}

/// A mesh uploaded to the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(u32);

impl MeshHandle {
    /// Numeric handle useful for diagnostics.
    pub const fn id(self) -> u32 {
        self.0
    }
}

/// One mesh draw submitted for a frame.
#[derive(Debug, Clone, Copy)]
pub struct MeshDraw3D {
    /// Uploaded mesh.
    pub mesh: MeshHandle,
    /// Object transform.
    pub transform: Mat4,
    /// Material override.
    pub material: Material3D,
    /// Optional albedo texture.
    pub texture: Option<TextureHandle>,
}

/// RGBA8 image data associated with an imported model.
#[derive(Debug, Clone)]
pub struct TextureData3D {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA8 pixels.
    pub rgba: Vec<u8>,
}

/// A texture uploaded to the 3D renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(u32);

impl TextureHandle {
    /// Numeric handle useful for diagnostics.
    pub const fn id(self) -> u32 {
        self.0
    }
}

/// Joint indices and normalized weights for a skinned mesh.
#[derive(Debug, Clone)]
pub struct SkinWeights3D {
    /// Four joint indices per vertex.
    pub joints: Vec<[u16; 4]>,
    /// Four weights per vertex.
    pub weights: Vec<[f32; 4]>,
    /// Index into [`GltfModel::skins`].
    pub skin_index: usize,
    /// Node that owns the skinned mesh, if the source document provided one.
    pub mesh_node: Option<usize>,
}

#[derive(Debug)]
struct GpuMesh3D {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    vertex_count: usize,
    index_count: u32,
}

struct GpuTexture3D {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_projection: [f32; 16],
    light_view_projection: [f32; 16],
    camera_position: [f32; 4],
    light_direction: [f32; 4],
    light_color: [f32; 4],
    ambient_color: [f32; 4],
    bounce_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectUniform {
    model: [f32; 16],
    base_color: [f32; 4],
}

const OBJECT_UNIFORM_CAPACITY: usize = 256;
const SHADOW_MAP_SIZE: u32 = 2048;

/// Errors returned by the 3D asset and renderer layer.
#[derive(Debug, Error)]
pub enum Render3DError {
    /// Mesh contains no triangles.
    #[error("3D mesh is empty")]
    EmptyMesh,
    /// Mesh index is outside the vertex buffer.
    #[error("3D mesh index {index} exceeds vertex count {vertices}")]
    InvalidIndex {
        /// Invalid index value.
        index: u32,
        /// Number of vertices available.
        vertices: usize,
    },
    /// glTF/GLB import failed.
    #[error("glTF import failed: {0}")]
    Gltf(String),
    /// GPU operation failed.
    #[error("3D GPU error: {0}")]
    Gpu(String),
    /// Updated vertex slice does not match uploaded mesh.
    #[error("3D vertex count mismatch: got {got}, expected {expected}")]
    VertexCountMismatch {
        /// Number of updated vertices.
        got: usize,
        /// Number of vertices uploaded initially.
        expected: usize,
    },
}

/// Static glTF/GLB model data ready for GPU upload.
#[derive(Debug, Default)]
pub struct GltfModel {
    /// Mesh primitives in document order.
    pub meshes: Vec<Mesh3D>,
    /// Animation clip names advertised by the file.
    pub animations: Vec<String>,
    /// Parsed animation clips, including keyframes for CPU sampling.
    pub animation_clips: Vec<AnimationClip3D>,
    /// Embedded/external texture data in document order.
    pub textures: Vec<TextureData3D>,
    /// Bind-pose transform of each primitive's mesh node.
    ///
    /// glTF files exported from FBX tools often keep a unit-conversion scale
    /// on the armature root. The renderer must apply this transform outside
    /// CPU skinning or the mesh appears hundreds of times too large.
    mesh_transforms: Vec<Mat4>,
    nodes: Vec<NodePose3D>,
    skins: Vec<Skin3D>,
}

/// Parsed animation clip from a glTF model.
#[derive(Debug, Clone)]
pub struct AnimationClip3D {
    /// Stable clip name.
    pub name: String,
    /// Duration in seconds.
    pub duration: f32,
    channels: Vec<AnimationChannel3D>,
}

#[derive(Debug, Clone)]
enum AnimationChannel3D {
    Translation {
        node: usize,
        keys: Vec<(f32, Vec3)>,
    },
    Rotation {
        node: usize,
        keys: Vec<(f32, [f32; 4])>,
    },
    Scale {
        node: usize,
        keys: Vec<(f32, Vec3)>,
    },
}

#[derive(Debug, Clone, Copy)]
struct NodePose3D {
    parent: Option<usize>,
    translation: Vec3,
    rotation: [f32; 4],
    scale: Vec3,
    matrix: Option<Mat4>,
}

impl NodePose3D {
    fn local_matrix(self) -> Mat4 {
        self.matrix.unwrap_or_else(|| {
            Mat4::from_translation(self.translation)
                * Mat4::from_quaternion(self.rotation)
                * Mat4::from_scale(self.scale)
        })
    }
}

#[derive(Debug, Clone)]
struct Skin3D {
    joints: Vec<usize>,
    inverse_bind_matrices: Vec<Mat4>,
}

impl GltfModel {
    /// Load a `.glb` or `.gltf` file using the bundled glTF importer.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Render3DError> {
        let (document, buffers, _images) =
            gltf::import(path.as_ref()).map_err(|error| Render3DError::Gltf(error.to_string()))?;
        parse_gltf(document, buffers, _images)
    }

    /// Load from in-memory glTF/GLB bytes.
    pub fn load_bytes(bytes: &[u8]) -> Result<Self, Render3DError> {
        let (document, buffers, _images) =
            gltf::import_slice(bytes).map_err(|error| Render3DError::Gltf(error.to_string()))?;
        parse_gltf(document, buffers, _images)
    }

    /// Find an animation clip by exact name, case-insensitive fallback.
    pub fn animation_index(&self, name: &str) -> Option<usize> {
        self.animation_clips
            .iter()
            .position(|clip| clip.name == name)
            .or_else(|| {
                let needle = name.to_ascii_lowercase();
                self.animation_clips
                    .iter()
                    .position(|clip| clip.name.to_ascii_lowercase().contains(&needle))
            })
    }

    /// Bind-pose local-to-world transform for one imported mesh primitive.
    pub fn mesh_transform(&self, mesh_index: usize) -> Mat4 {
        self.mesh_transforms
            .get(mesh_index)
            .copied()
            .unwrap_or(Mat4::IDENTITY)
    }

    /// Sample a skinned mesh at `time` and return updated vertices.
    ///
    /// Sampling is CPU-side for this first runtime milestone. It keeps the
    /// public renderer independent from a fixed maximum joint count while
    /// still using the authored glTF animation clips exactly.
    pub fn sample_animation(
        &self,
        mesh_index: usize,
        clip_index: usize,
        time: f32,
    ) -> Option<Vec<MeshVertex3D>> {
        let mesh = self.meshes.get(mesh_index)?;
        let skinning = mesh.skinning.as_ref()?;
        let skin = self.skins.get(skinning.skin_index)?;
        let clip = self.animation_clips.get(clip_index)?;
        let mut nodes = self.nodes.clone();
        let sample_time = if clip.duration > 0.0 {
            time.rem_euclid(clip.duration)
        } else {
            0.0
        };
        for channel in &clip.channels {
            match channel {
                AnimationChannel3D::Translation { node, keys } => {
                    if let Some(pose) = nodes.get_mut(*node) {
                        pose.translation = sample_vec3(keys, sample_time);
                        pose.matrix = None;
                    }
                }
                AnimationChannel3D::Rotation { node, keys } => {
                    if let Some(pose) = nodes.get_mut(*node) {
                        pose.rotation = sample_quaternion(keys, sample_time);
                        pose.matrix = None;
                    }
                }
                AnimationChannel3D::Scale { node, keys } => {
                    if let Some(pose) = nodes.get_mut(*node) {
                        pose.scale = sample_vec3(keys, sample_time);
                        pose.matrix = None;
                    }
                }
            }
        }
        let mut globals = vec![None; nodes.len()];
        let mesh_inverse = skinning
            .mesh_node
            .and_then(|node| resolve_global(node, &nodes, &mut globals).inverse())
            .unwrap_or(Mat4::IDENTITY);
        let joint_matrices: Vec<Mat4> = skin
            .joints
            .iter()
            .enumerate()
            .map(|(index, joint)| {
                let global = resolve_global(*joint, &nodes, &mut globals);
                let inverse_bind = skin
                    .inverse_bind_matrices
                    .get(index)
                    .copied()
                    .unwrap_or(Mat4::IDENTITY);
                mesh_inverse * global * inverse_bind
            })
            .collect();
        let mut output = mesh.vertices.clone();
        for (index, vertex) in output.iter_mut().enumerate() {
            let Some(joints) = skinning.joints.get(index) else {
                continue;
            };
            let weights = skinning.weights.get(index).copied().unwrap_or([0.0; 4]);
            let bind_position =
                Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2]);
            let bind_normal = Vec3::new(vertex.normal[0], vertex.normal[1], vertex.normal[2]);
            let mut position = Vec3::ZERO;
            let mut normal = Vec3::ZERO;
            let mut total_weight = 0.0;
            for slot in 0..4 {
                let weight = weights[slot];
                let Some(joint) = joint_matrices.get(joints[slot] as usize).copied() else {
                    continue;
                };
                if weight <= 0.0 {
                    continue;
                }
                position += joint.transform_point3(bind_position) * weight;
                normal += joint.transform_vector3(bind_normal) * weight;
                total_weight += weight;
            }
            if total_weight > 1e-5 {
                vertex.position = position.to_array();
                vertex.normal = normal.normalize_or_zero().to_array();
            }
        }
        Some(output)
    }
}

fn resolve_global(index: usize, nodes: &[NodePose3D], globals: &mut [Option<Mat4>]) -> Mat4 {
    if let Some(global) = globals.get(index).and_then(|global| *global) {
        return global;
    }
    let Some(node) = nodes.get(index).copied() else {
        return Mat4::IDENTITY;
    };
    let local = node.local_matrix();
    let global = node
        .parent
        .map(|parent| resolve_global(parent, nodes, globals) * local)
        .unwrap_or(local);
    if let Some(slot) = globals.get_mut(index) {
        *slot = Some(global);
    }
    global
}

fn sample_vec3(keys: &[(f32, Vec3)], time: f32) -> Vec3 {
    if keys.is_empty() {
        return Vec3::ZERO;
    }
    if keys.len() == 1 || time <= keys[0].0 {
        return keys[0].1;
    }
    for window in keys.windows(2) {
        let (a_time, a) = window[0];
        let (b_time, b) = window[1];
        if time <= b_time {
            let t = ((time - a_time) / (b_time - a_time).max(1e-6)).clamp(0.0, 1.0);
            return a.lerp(b, t);
        }
    }
    keys.last().map(|(_, value)| *value).unwrap_or(Vec3::ZERO)
}

fn sample_quaternion(keys: &[(f32, [f32; 4])], time: f32) -> [f32; 4] {
    if keys.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let value = if keys.len() == 1 || time <= keys[0].0 {
        keys[0].1
    } else {
        let mut result = keys
            .last()
            .map(|(_, value)| *value)
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        for window in keys.windows(2) {
            let (a_time, a) = window[0];
            let (b_time, mut b) = window[1];
            if time <= b_time {
                let t = ((time - a_time) / (b_time - a_time).max(1e-6)).clamp(0.0, 1.0);
                // q and -q represent the same rotation.  Interpolating
                // across opposite signs would take the long arc and can
                // produce a full-body 180° flip between walk frames.
                let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
                if dot < 0.0 {
                    b = [-b[0], -b[1], -b[2], -b[3]];
                }
                result = [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                    a[3] + (b[3] - a[3]) * t,
                ];
                break;
            }
        }
        result
    };
    let length =
        (value[0] * value[0] + value[1] * value[1] + value[2] * value[2] + value[3] * value[3])
            .sqrt()
            .max(1e-6);
    [
        value[0] / length,
        value[1] / length,
        value[2] / length,
        value[3] / length,
    ]
}

fn parse_gltf(
    document: gltf::Document,
    buffers: Vec<gltf::buffer::Data>,
    images: Vec<gltf::image::Data>,
) -> Result<GltfModel, Render3DError> {
    let mut nodes: Vec<NodePose3D> = document
        .nodes()
        .map(|node| {
            let (translation, rotation, scale) = node.transform().decomposed();
            NodePose3D {
                parent: None,
                translation: Vec3::new(translation[0], translation[1], translation[2]),
                rotation,
                scale: Vec3::new(scale[0], scale[1], scale[2]),
                matrix: None,
            }
        })
        .collect();
    for node in document.nodes() {
        for child in node.children() {
            if let Some(pose) = nodes.get_mut(child.index()) {
                pose.parent = Some(node.index());
            }
        }
    }
    let mesh_count = document.meshes().count();
    let mut mesh_skin_nodes = vec![None; mesh_count];
    let mut mesh_skin_indices = vec![None; mesh_count];
    for node in document.nodes() {
        if let Some(mesh) = node.mesh() {
            mesh_skin_nodes[mesh.index()] = Some(node.index());
            mesh_skin_indices[mesh.index()] = node.skin().map(|skin| skin.index());
        }
    }

    let skins: Vec<Skin3D> = document
        .skins()
        .map(|skin| {
            let joints = skin.joints().map(|joint| joint.index()).collect::<Vec<_>>();
            let inverse_bind_matrices = skin
                .reader(|buffer| Some(&buffers[buffer.index()].0))
                .read_inverse_bind_matrices()
                .map(|matrices| {
                    matrices
                        .map(|matrix| Mat4::from_cols(matrix[0], matrix[1], matrix[2], matrix[3]))
                        .collect()
                })
                .unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);
            Skin3D {
                joints,
                inverse_bind_matrices,
            }
        })
        .collect();

    let mut bind_globals = vec![None; nodes.len()];
    let mesh_node_transforms: Vec<Mat4> = mesh_skin_nodes
        .iter()
        .map(|node| {
            node.map(|node| resolve_global(node, &nodes, &mut bind_globals))
                .unwrap_or(Mat4::IDENTITY)
        })
        .collect();

    let mut meshes = Vec::new();
    let mut mesh_transforms = Vec::new();
    for mesh in document.meshes() {
        let skin_index = mesh_skin_indices.get(mesh.index()).copied().flatten();
        let mesh_node = mesh_skin_nodes.get(mesh.index()).copied().flatten();
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()].0));
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .ok_or_else(|| Render3DError::Gltf("primitive has no positions".into()))?
                .collect();
            let normals: Vec<[f32; 3]> = reader
                .read_normals()
                .map(|values| values.collect())
                .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);
            let uvs: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|values| values.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
            let colors: Vec<[f32; 4]> = reader
                .read_colors(0)
                .map(|values| values.into_rgba_f32().collect())
                .unwrap_or_else(|| vec![[1.0; 4]; positions.len()]);
            let joints: Vec<[u16; 4]> = reader
                .read_joints(0)
                .map(|values| values.into_u16().collect())
                .unwrap_or_else(|| vec![[0; 4]; positions.len()]);
            let weights: Vec<[f32; 4]> = reader
                .read_weights(0)
                .map(|values| values.into_f32().collect())
                .unwrap_or_else(|| vec![[0.0; 4]; positions.len()]);
            let indices: Vec<u32> = reader
                .read_indices()
                .map(|values| values.into_u32().collect())
                .unwrap_or_else(|| (0..positions.len() as u32).collect());
            let pbr = primitive.material().pbr_metallic_roughness();
            let base_color = pbr.base_color_factor();
            let texture_index = pbr
                .base_color_texture()
                .map(|texture| texture.texture().source().index());
            let vertices = positions
                .into_iter()
                .enumerate()
                .map(|(index, position)| MeshVertex3D {
                    position,
                    normal: normals.get(index).copied().unwrap_or([0.0, 1.0, 0.0]),
                    uv: uvs.get(index).copied().unwrap_or([0.0, 0.0]),
                    color: colors.get(index).copied().unwrap_or([1.0; 4]),
                })
                .collect();
            let skinning = skin_index.map(|skin_index| SkinWeights3D {
                joints: joints.clone(),
                weights: weights.clone(),
                skin_index,
                mesh_node,
            });
            meshes.push(Mesh3D {
                vertices,
                indices,
                material: Material3D {
                    base_color,
                    texture: None,
                },
                texture_index,
                skinning,
            });
            mesh_transforms.push(
                mesh_node_transforms
                    .get(mesh.index())
                    .copied()
                    .unwrap_or(Mat4::IDENTITY),
            );
        }
    }

    let mut animation_clips = Vec::new();
    for animation in document.animations() {
        let mut duration: f32 = 0.0;
        let mut channels = Vec::new();
        for channel in animation.channels() {
            let target_node = channel.target().node().index();
            let reader = channel.reader(|buffer| Some(&buffers[buffer.index()].0));
            let inputs: Vec<f32> = reader
                .read_inputs()
                .map(|values| values.collect())
                .unwrap_or_default();
            duration = duration.max(inputs.iter().copied().fold(0.0, f32::max));
            let Some(outputs) = reader.read_outputs() else {
                continue;
            };
            match outputs {
                gltf::animation::util::ReadOutputs::Translations(values) => {
                    let values: Vec<Vec3> = values
                        .map(|value| Vec3::new(value[0], value[1], value[2]))
                        .collect();
                    channels.push(AnimationChannel3D::Translation {
                        node: target_node,
                        keys: zip_keys(&inputs, &values),
                    });
                }
                gltf::animation::util::ReadOutputs::Rotations(values) => {
                    let values: Vec<[f32; 4]> = values.into_f32().collect();
                    channels.push(AnimationChannel3D::Rotation {
                        node: target_node,
                        keys: zip_keys(&inputs, &values),
                    });
                }
                gltf::animation::util::ReadOutputs::Scales(values) => {
                    let values: Vec<Vec3> = values
                        .map(|value| Vec3::new(value[0], value[1], value[2]))
                        .collect();
                    channels.push(AnimationChannel3D::Scale {
                        node: target_node,
                        keys: zip_keys(&inputs, &values),
                    });
                }
                gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => {}
            }
        }
        animation_clips.push(AnimationClip3D {
            name: animation.name().unwrap_or("unnamed").to_owned(),
            duration,
            channels,
        });
    }
    let animations = animation_clips
        .iter()
        .map(|clip| clip.name.clone())
        .collect();
    let textures = images.into_iter().map(texture_data).collect();
    Ok(GltfModel {
        meshes,
        animations,
        animation_clips,
        textures,
        mesh_transforms,
        nodes,
        skins,
    })
}

fn zip_keys<T: Copy>(times: &[f32], values: &[T]) -> Vec<(f32, T)> {
    times.iter().copied().zip(values.iter().copied()).collect()
}

fn texture_data(image: gltf::image::Data) -> TextureData3D {
    let mut rgba = Vec::with_capacity((image.width * image.height * 4) as usize);
    match image.format {
        gltf::image::Format::R8 => {
            for value in image.pixels {
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        gltf::image::Format::R8G8 => {
            for pair in image.pixels.chunks_exact(2) {
                rgba.extend_from_slice(&[pair[0], pair[1], 0, 255]);
            }
        }
        gltf::image::Format::R8G8B8 => {
            for rgb in image.pixels.chunks_exact(3) {
                rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        gltf::image::Format::R8G8B8A8 => rgba = image.pixels,
        _ => rgba.resize((image.width * image.height * 4) as usize, 255),
    }
    TextureData3D {
        width: image.width.max(1),
        height: image.height.max(1),
        rgba,
    }
}

/// Minimal forward renderer for solid-color, lit 3D meshes.
pub struct Wgpu3DRenderer {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    object_buffer: wgpu::Buffer,
    object_bind_group: wgpu::BindGroup,
    object_bind_group_layout: wgpu::BindGroupLayout,
    object_stride: u64,
    object_capacity: usize,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    meshes: Vec<GpuMesh3D>,
    textures: Vec<GpuTexture3D>,
    white_texture: TextureHandle,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    depth_size: (u32, u32),
    #[allow(dead_code)]
    shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    shadow_bind_group: wgpu::BindGroup,
}

impl Wgpu3DRenderer {
    /// Create a renderer targeting `format` and a depth buffer of `size`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-camera-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let object_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-object-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<ObjectUniform>() as u64),
                },
                count: None,
            }],
        });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("velvet-3d-texture-layout"),
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
        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("velvet-3d-shadow-texture-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-3d-linear-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-3d-pipeline-layout"),
            bind_group_layouts: &[
                &camera_layout,
                &object_layout,
                &texture_bind_group_layout,
                &shadow_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("velvet-3d-shadow-pipeline-layout"),
                bind_group_layouts: &[&camera_layout, &object_layout],
                push_constant_ranges: &[],
            });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-shader"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-shadow-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("velvet-3d-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex3D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("velvet-3d-shadow-pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_shadow"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex3D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x3,
                        1 => Float32x3,
                        2 => Float32x2,
                        3 => Float32x4,
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-camera-buffer"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-camera-bind-group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let object_size = std::mem::size_of::<ObjectUniform>() as u64;
        let alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
        let object_stride = align_uniform_offset(object_size, alignment);
        let object_capacity = OBJECT_UNIFORM_CAPACITY;
        let object_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-object-buffer"),
            size: object_stride * object_capacity as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let object_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-object-bind-group"),
            layout: &object_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &object_buffer,
                    offset: 0,
                    size: NonZeroU64::new(object_size),
                }),
            }],
        });
        let white_texture = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![255, 255, 255, 255],
        };
        let white_gpu = create_gpu_texture(
            device,
            queue,
            &texture_bind_group_layout,
            &sampler,
            &white_texture,
        );
        let (depth_texture, depth_view) = create_depth_texture(device, size);
        let shadow_texture = create_shadow_texture(device);
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-3d-shadow-comparison-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-shadow-texture-bind-group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&shadow_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });
        Self {
            pipeline,
            shadow_pipeline,
            camera_buffer,
            camera_bind_group,
            object_buffer,
            object_bind_group,
            object_bind_group_layout: object_layout,
            object_stride,
            object_capacity,
            texture_bind_group_layout,
            sampler,
            meshes: Vec::new(),
            textures: vec![white_gpu],
            white_texture: TextureHandle(0),
            depth_texture,
            depth_view,
            depth_size: (size.0.max(1), size.1.max(1)),
            shadow_texture,
            shadow_view,
            shadow_bind_group,
        }
    }

    /// Resize depth storage to match the target surface.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.depth_size == size {
            return;
        }
        let (texture, view) = create_depth_texture(device, size);
        self.depth_texture = texture;
        self.depth_view = view;
        self.depth_size = size;
    }

    fn ensure_object_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.object_capacity {
            return;
        }
        let mut capacity = self.object_capacity.max(1);
        while capacity < required {
            capacity = capacity.saturating_mul(2);
        }
        let object_size = std::mem::size_of::<ObjectUniform>() as u64;
        self.object_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-object-buffer-grown"),
            size: self.object_stride * capacity as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.object_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-object-bind-group-grown"),
            layout: &self.object_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.object_buffer,
                    offset: 0,
                    size: NonZeroU64::new(object_size),
                }),
            }],
        });
        self.object_capacity = capacity;
    }

    /// Upload a CPU mesh and return its stable handle.
    pub fn upload_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh: &Mesh3D,
    ) -> Result<MeshHandle, Render3DError> {
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return Err(Render3DError::EmptyMesh);
        }
        for &index in &mesh.indices {
            if index as usize >= mesh.vertices.len() {
                return Err(Render3DError::InvalidIndex {
                    index,
                    vertices: mesh.vertices.len(),
                });
            }
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let handle = MeshHandle(self.meshes.len() as u32);
        self.meshes.push(GpuMesh3D {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len() as u32,
        });
        Ok(handle)
    }

    /// Replace vertices of an uploaded mesh, used by CPU skinning samplers.
    pub fn update_mesh_vertices(
        &self,
        queue: &wgpu::Queue,
        handle: MeshHandle,
        vertices: &[MeshVertex3D],
    ) -> Result<(), Render3DError> {
        let mesh = self
            .meshes
            .get(handle.0 as usize)
            .ok_or_else(|| Render3DError::Gpu(format!("unknown mesh handle {}", handle.0)))?;
        if vertices.len() != mesh.vertex_count {
            return Err(Render3DError::VertexCountMismatch {
                got: vertices.len(),
                expected: mesh.vertex_count,
            });
        }
        if vertices.is_empty() {
            return Err(Render3DError::EmptyMesh);
        }
        let bytes = bytemuck::cast_slice(vertices);
        queue.write_buffer(&mesh.vertex_buffer, 0, bytes);
        Ok(())
    }

    /// Upload an RGBA8 albedo texture and return its stable handle.
    pub fn upload_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &TextureData3D,
    ) -> Result<TextureHandle, Render3DError> {
        let expected = texture.width.max(1) as usize * texture.height.max(1) as usize * 4;
        if texture.rgba.len() < expected {
            return Err(Render3DError::Gpu(format!(
                "texture buffer too small: {} < {expected}",
                texture.rgba.len()
            )));
        }
        let gpu = create_gpu_texture(
            device,
            queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            texture,
        );
        let handle = TextureHandle(self.textures.len() as u32);
        self.textures.push(gpu);
        Ok(handle)
    }

    /// Render one 3D pass into a target view.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
        lighting: Lighting3D,
        draws: &[MeshDraw3D],
        clear: wgpu::Color,
    ) {
        self.resize(device, size);
        self.ensure_object_capacity(device, draws.len());
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let mut light_direction = lighting.direction_to_light.normalize_or_zero();
        if light_direction.length_squared() <= 1e-6 {
            light_direction = Lighting3D::default().direction_to_light.normalize_or_zero();
        }
        let light_target = Vec3::new(camera.target.x, 0.5, camera.target.z);
        let light_position = light_target + light_direction * 18.0;
        let shadow_extent = lighting.shadow_extent.max(1.0);
        let light_view_projection = Mat4::orthographic_rh_zo(
            -shadow_extent,
            shadow_extent,
            -shadow_extent,
            shadow_extent,
            0.1,
            45.0,
        ) * Mat4::look_at_rh(light_position, light_target, Vec3::Y);
        let camera_uniform = CameraUniform {
            view_projection: camera.view_projection(aspect).to_cols_array(),
            light_view_projection: light_view_projection.to_cols_array(),
            camera_position: [camera.position.x, camera.position.y, camera.position.z, 1.0],
            light_direction: [light_direction.x, light_direction.y, light_direction.z, 0.0],
            light_color: [
                lighting.key_color[0],
                lighting.key_color[1],
                lighting.key_color[2],
                1.0,
            ],
            ambient_color: [
                lighting.ambient_color[0],
                lighting.ambient_color[1],
                lighting.ambient_color[2],
                1.0,
            ],
            bounce_color: [
                lighting.bounce_color[0],
                lighting.bounce_color[1],
                lighting.bounce_color[2],
                1.0,
            ],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        for (draw_index, draw) in draws.iter().enumerate() {
            let object = ObjectUniform {
                model: draw.transform.to_cols_array(),
                base_color: draw.material.base_color,
            };
            let offset = self.object_stride * draw_index as u64;
            queue.write_buffer(&self.object_buffer, offset, bytemuck::bytes_of(&object));
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("velvet-3d-frame"),
        });
        {
            let mut shadow_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-shadow-pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            shadow_pass.set_pipeline(&self.shadow_pipeline);
            shadow_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            for (draw_index, draw) in draws.iter().enumerate() {
                let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                    continue;
                };
                let offset = self.object_stride * draw_index as u64;
                shadow_pass.set_bind_group(1, &self.object_bind_group, &[offset as u32]);
                shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                shadow_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(3, &self.shadow_bind_group, &[]);
            for (draw_index, draw) in draws.iter().enumerate() {
                let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                    continue;
                };
                let offset = self.object_stride * draw_index as u64;
                pass.set_bind_group(1, &self.object_bind_group, &[offset as u32]);
                let texture = draw
                    .texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture);
                if let Some(texture) = self.textures.get(texture.0 as usize) {
                    pass.set_bind_group(2, &texture.bind_group, &[]);
                }
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    size: (u32, u32),
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_shadow_texture(device: &wgpu::Device) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-directional-shadow-map"),
        size: wgpu::Extent3d {
            width: SHADOW_MAP_SIZE,
            height: SHADOW_MAP_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn align_uniform_offset(size: u64, alignment: u64) -> u64 {
    let alignment = alignment.max(1);
    size.div_ceil(alignment) * alignment
}

fn create_gpu_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    data: &TextureData3D,
) -> GpuTexture3D {
    let width = data.width.max(1);
    let height = data.height.max(1);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-albedo"),
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
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &data.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("velvet-3d-albedo-bind-group"),
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
    });
    GpuTexture3D {
        texture,
        bind_group,
    }
}

const MESH_SHADER: &str = r#"
struct Camera {
    view_projection: mat4x4<f32>,
    light_view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    bounce_color: vec4<f32>,
};
struct Object { model: mat4x4<f32>, base_color: vec4<f32>, };
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> object: Object;
@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(3) @binding(0) var shadow_texture: texture_depth_2d;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) light_clip: vec4<f32>,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    let world = object.model * vec4<f32>(input.position, 1.0);
    var out: VertexOut;
    out.clip = camera.view_projection * world;
    out.normal = normalize((object.model * vec4<f32>(input.normal, 0.0)).xyz);
    out.color = input.color;
    out.uv = input.uv;
    out.world_position = world.xyz;
    out.light_clip = camera.light_view_projection * world;
    return out;
}

fn shadow_visibility(light_clip: vec4<f32>, normal: vec3<f32>) -> f32 {
    let ndc = light_clip.xyz / light_clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5));
    if (uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0 || ndc.z <= 0.0 || ndc.z >= 1.0) {
        return 1.0;
    }
    let light = normalize(camera.light_direction.xyz);
    let bias = max(0.0012 * (1.0 - max(dot(normal, light), 0.0)), 0.00028);
    let texel = 1.0 / vec2<f32>(textureDimensions(shadow_texture));
    var visibility = 0.0;
    for (var y: i32 = -1; y <= 1; y = y + 1) {
        for (var x: i32 = -1; x <= 1; x = x + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            visibility = visibility + textureSampleCompare(
                shadow_texture,
                shadow_sampler,
                uv + offset,
                ndc.z - bias,
            );
        }
    }
    return visibility / 9.0;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light_dir = normalize(camera.light_direction.xyz);
    let view_dir = normalize(camera.camera_position.xyz - input.world_position);
    let diffuse = max(dot(normal, light_dir), 0.0);
    let visibility = shadow_visibility(input.light_clip, normal);
    let hemisphere = clamp(normal.y * 0.5 + 0.5, 0.0, 1.0);
    let sky = camera.ambient_color.rgb * (0.52 + hemisphere * 0.48);
    let bounce = camera.bounce_color.rgb * (1.0 - hemisphere) * 0.62;
    let direct = camera.light_color.rgb * diffuse * mix(0.13, 1.0, visibility);
    let halfway = normalize(light_dir + view_dir);
    let specular = pow(max(dot(normal, halfway), 0.0), 48.0) * 0.18 * visibility;
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0) * 0.10;
    let albedo = textureSample(albedo_texture, albedo_sampler, input.uv);
    let base = object.base_color.rgb * input.color.rgb * albedo.rgb;
    let color = base * (sky + bounce + direct) + camera.light_color.rgb * specular + camera.ambient_color.rgb * rim;
    return vec4<f32>(color, object.base_color.a * input.color.a * albedo.a);
}
"#;

const SHADOW_SHADER: &str = r#"
struct Camera {
    view_projection: mat4x4<f32>,
    light_view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
    ambient_color: vec4<f32>,
    bounce_color: vec4<f32>,
};
struct Object { model: mat4x4<f32>, base_color: vec4<f32>, };
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> object: Object;

struct VertexIn {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_shadow(input: VertexIn) -> @builtin(position) vec4<f32> {
    return camera.light_view_projection * object.model * vec4<f32>(input.position, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_projection_is_finite() {
        let camera = Camera3D::default();
        let values = camera.view_projection(16.0 / 9.0).to_cols_array();
        assert!(values.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn cube_has_indexed_triangles() {
        let mesh = Mesh3D::cube(1.0, Material3D::default());
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
        assert!(mesh
            .indices
            .iter()
            .all(|index| (*index as usize) < mesh.vertices.len()));
    }

    #[test]
    fn authored_queen_glb_exposes_walk_skin_and_texture() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_animated.glb");
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("authored Queen GLB should load");
        assert!(!model.meshes.is_empty());
        assert!(!model.textures.is_empty());
        let walk = model.animation_index("Queen_Walk").expect("walk clip");
        assert!(model.meshes[0].skinning.is_some());
        let sampled = model
            .sample_animation(0, walk, 0.15)
            .expect("skinned mesh sample");
        assert_eq!(sampled.len(), model.meshes[0].vertices.len());
    }

    #[test]
    fn mixamo_walk_glb_exposes_runtime_clip() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_mixamo_walk.glb",
        );
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("Mixamo walk GLB should load");
        let walk = model
            .animation_index("Queen_Walk")
            .expect("Mixamo walk clip");
        assert!(model.meshes[0].skinning.is_some());
        let first = model
            .sample_animation(0, walk, 0.0)
            .expect("first walk sample");
        let later = model
            .sample_animation(0, walk, 0.35)
            .expect("later walk sample");
        assert_eq!(first.len(), later.len());
        assert!(first.iter().zip(&later).any(|(a, b)| {
            a.position
                .iter()
                .zip(b.position.iter())
                .any(|(left, right)| (left - right).abs() > 1e-4)
        }));
    }

    #[test]
    fn native_mixamo_queen_keeps_texture_and_authored_walk() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_mixamo_native.glb",
        );
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("native Mixamo Queen GLB should load");
        assert_eq!(model.textures.len(), 1);
        let walk = model
            .animation_index("Queen_Walk")
            .expect("native Mixamo walk clip");
        assert!(model.meshes[0].skinning.is_some());
        let first = model
            .sample_animation(0, walk, 0.0)
            .expect("first native walk sample");
        let later = model
            .sample_animation(0, walk, 0.35)
            .expect("later native walk sample");
        assert_eq!(first.len(), later.len());
        assert!(first.iter().zip(&later).any(|(a, b)| {
            a.position
                .iter()
                .zip(b.position.iter())
                .any(|(left, right)| (left - right).abs() > 1e-4)
        }));
    }

    #[test]
    fn retargeted_queen_glb_keeps_texture_and_walk_clip() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_mixamo_retargeted.glb",
        );
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("retargeted Queen GLB should load");
        assert!(!model.textures.is_empty());
        let walk = model
            .animation_index("Queen_Walk")
            .expect("retargeted walk clip");
        assert!(model.meshes[0].skinning.is_some());
        let first = model
            .sample_animation(0, walk, 0.0)
            .expect("first retargeted sample");
        let later = model
            .sample_animation(0, walk, 0.35)
            .expect("later retargeted sample");
        assert_eq!(first.len(), later.len());
        assert!(first.iter().zip(&later).any(|(a, b)| {
            a.position
                .iter()
                .zip(b.position.iter())
                .any(|(left, right)| (left - right).abs() > 1e-4)
        }));
    }
}
