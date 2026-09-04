//! `Scene3D` document types (JSON-serializable, renderer-agnostic).

use serde::{Deserialize, Serialize};

/// Position / rotation / scale for one node.
///
/// Positions and scales are world units; rotations are Euler degrees in
/// XYZ order (author-friendly — the flattener converts to radians).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformDoc {
    /// World position `[x, y, z]` (Y-up).
    #[serde(default = "default_position")]
    pub position: [f32; 3],
    /// Euler rotation degrees `[rx, ry, rz]` (XYZ order).
    #[serde(default)]
    pub rotation_deg: [f32; 3],
    /// Non-uniform scale.
    #[serde(default = "default_scale")]
    pub scale: [f32; 3],
}

fn default_position() -> [f32; 3] {
    [0.0, 0.0, 0.0]
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

impl Default for TransformDoc {
    fn default() -> Self {
        Self {
            position: default_position(),
            rotation_deg: [0.0, 0.0, 0.0],
            scale: default_scale(),
        }
    }
}

/// What a scene node draws.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MeshKind3D {
    /// Unit cube centered at the origin (scaled by transform).
    Cube,
    /// UV sphere of radius 1.
    Sphere {
        /// Width segments (clamped to 3..=64 at flatten time).
        #[serde(default = "default_sphere_segments")]
        segments: u32,
    },
    /// 1x1 plane on XZ facing +Y.
    Plane,
    /// External model (`.glb` / `.gltf`, project-relative path).
    Gltf {
        /// Asset path.
        path: String,
    },
    /// Empty transform node (grouping / spawn points / targets).
    Empty,
}

fn default_sphere_segments() -> u32 {
    16
}

/// Surface material override for one node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialDoc {
    /// Linear RGBA base color.
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 4],
    /// Contact/cavity occlusion strength 0..=1 (clamped at flatten time).
    #[serde(default)]
    pub ambient_occlusion: f32,
}

fn default_base_color() -> [f32; 4] {
    [0.75, 0.70, 0.82, 1.0]
}

impl Default for MaterialDoc {
    fn default() -> Self {
        Self {
            base_color: default_base_color(),
            ambient_occlusion: 0.0,
        }
    }
}

/// One node in the scene tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode3D {
    /// Stable id (unique across the scene).
    pub id: String,
    /// What to draw.
    #[serde(default = "default_mesh_kind")]
    pub mesh: MeshKind3D,
    /// Local transform.
    #[serde(default)]
    pub transform: TransformDoc,
    /// Material override.
    #[serde(default)]
    pub material: MaterialDoc,
    /// Casts directional shadows.
    #[serde(default = "default_true")]
    pub cast_shadow: bool,
    /// Contributes a static collider when baked (false for dynamic bodies
    /// like the hero/enemy, which still draw from this node).
    #[serde(default = "default_true")]
    pub collider: bool,
    /// Node is skipped by the flattener (kept in the document).
    #[serde(default)]
    pub disabled: bool,
    /// Nested children (parent world matrix applies).
    #[serde(default)]
    pub children: Vec<SceneNode3D>,
}

fn default_mesh_kind() -> MeshKind3D {
    MeshKind3D::Empty
}

fn default_true() -> bool {
    true
}

/// Perspective or orthographic camera.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraDoc {
    /// Camera position.
    #[serde(default = "default_camera_position")]
    pub position: [f32; 3],
    /// Look-at target.
    #[serde(default = "default_camera_target")]
    pub target: [f32; 3],
    /// Vertical field of view in degrees.
    #[serde(default = "default_fov_deg")]
    pub fov_deg: f32,
    /// Orthographic vertical span (None = perspective).
    #[serde(default)]
    pub orthographic_height: Option<f32>,
    /// Near clip plane.
    #[serde(default = "default_near")]
    pub near: f32,
    /// Far clip plane.
    #[serde(default = "default_far")]
    pub far: f32,
}

fn default_camera_position() -> [f32; 3] {
    [4.5, 3.2, 5.0]
}

fn default_camera_target() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}

fn default_fov_deg() -> f32 {
    45.0
}

fn default_near() -> f32 {
    0.05
}

fn default_far() -> f32 {
    200.0
}

impl Default for CameraDoc {
    fn default() -> Self {
        Self {
            position: default_camera_position(),
            target: default_camera_target(),
            fov_deg: default_fov_deg(),
            orthographic_height: None,
            near: default_near(),
            far: default_far(),
        }
    }
}

/// One scene light.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LightDoc {
    /// Directional key light + ambient + bounce.
    Directional {
        /// Direction from the surface toward the light.
        #[serde(default = "default_light_dir")]
        direction: [f32; 3],
        /// Linear RGB key intensity.
        #[serde(default = "default_key_color")]
        key_color: [f32; 3],
        /// Cool hemispherical ambient.
        #[serde(default = "default_ambient_color")]
        ambient_color: [f32; 3],
        /// Half-width of the shadow camera in world units.
        #[serde(default = "default_shadow_extent")]
        shadow_extent: f32,
    },
    /// Local point light.
    Point {
        /// World-space centre.
        #[serde(default)]
        position: [f32; 3],
        /// Linear RGB colour.
        #[serde(default = "default_point_color")]
        color: [f32; 3],
        /// Intensity multiplier.
        #[serde(default = "default_point_intensity")]
        intensity: f32,
        /// Smooth cutoff radius in world units.
        #[serde(default = "default_point_radius")]
        radius: f32,
    },
}

fn default_light_dir() -> [f32; 3] {
    [-0.48, 0.82, 0.31]
}

fn default_key_color() -> [f32; 3] {
    [1.05, 0.82, 0.62]
}

fn default_ambient_color() -> [f32; 3] {
    [0.16, 0.12, 0.28]
}

fn default_shadow_extent() -> f32 {
    10.0
}

fn default_point_color() -> [f32; 3] {
    [1.0, 0.8, 0.6]
}

fn default_point_intensity() -> f32 {
    1.0
}

fn default_point_radius() -> f32 {
    5.0
}

/// Fog / atmosphere for one scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentDoc {
    /// Linear RGB fog color.
    #[serde(default)]
    pub fog_color: [f32; 3],
    /// Distance where haze starts.
    #[serde(default = "default_fog_start")]
    pub fog_start: f32,
    /// Distance at full haze.
    #[serde(default = "default_fog_end")]
    pub fog_end: f32,
    /// Exponential haze contribution.
    #[serde(default)]
    pub fog_density: f32,
}

fn default_fog_start() -> f32 {
    100_000.0
}

fn default_fog_end() -> f32 {
    100_001.0
}

impl Default for EnvironmentDoc {
    fn default() -> Self {
        Self {
            fog_color: [0.0, 0.0, 0.0],
            fog_start: default_fog_start(),
            fog_end: default_fog_end(),
            fog_density: 0.0,
        }
    }
}

/// A declarative 3D scene document (one `.scene3d.json` file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene3D {
    /// Scene name.
    #[serde(default = "default_scene_name")]
    pub name: String,
    /// Root nodes.
    #[serde(default)]
    pub nodes: Vec<SceneNode3D>,
    /// Camera (defaults to a 45° perspective orbit framing the origin).
    #[serde(default)]
    pub camera: CameraDoc,
    /// Lights (empty = renderer default key light).
    #[serde(default)]
    pub lights: Vec<LightDoc>,
    /// Environment.
    #[serde(default)]
    pub environment: EnvironmentDoc,
}

fn default_scene_name() -> String {
    "scene".into()
}
