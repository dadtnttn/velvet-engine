//! GPU 3D primitives for Velvet Engine.
//!
//! This module is deliberately small and data-driven: game code owns scene
//! state, while [`Wgpu3DRenderer`] owns GPU buffers, depth testing, and the
//! lit mesh pipeline.  It is the foundation for replacing exploration
//! billboards with real meshes without coupling the story layer to `wgpu`.

use std::{collections::BTreeMap, num::NonZeroU64, path::Path};

use crate::{ShadingStyle3D, ToonStyle3D};
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

/// Rotation keyframes grouped by animation target node.
pub type RotationTracks3D = Vec<(String, Vec<(f32, [f32; 4])>)>;

/// CPU-side indexed mesh.
#[derive(Debug, Clone)]
pub struct Mesh3D {
    /// Interleaved vertex data.
    pub vertices: Vec<MeshVertex3D>,
    /// Triangle-list indices.
    pub indices: Vec<u32>,
    /// Default material used when the mesh is drawn.
    pub material: Material3D,
    /// glTF metallic factor retained for lightweight preview renderers.
    pub metallic_factor: f32,
    /// glTF roughness factor retained for lightweight preview renderers.
    pub roughness_factor: f32,
    /// Optional index into [`GltfModel::textures`] for imported meshes.
    pub texture_index: Option<usize>,
    /// Optional tangent-space normal map paired with the albedo texture.
    ///
    /// The index uses the same compact texture table as [`Self::texture_index`]
    /// so uploaders can preserve the authored glTF normal detail without
    /// changing the lightweight draw API.
    pub normal_texture_index: Option<usize>,
    /// Optional skin weights for CPU skeletal sampling.
    pub skinning: Option<SkinWeights3D>,
}

impl Mesh3D {
    /// The mesh's triangles placed in the world by `transform` (each as its three corners).
    pub fn world_triangles(&self, transform: Mat4) -> Vec<[Vec3; 3]> {
        let p = |i: u32| {
            let v = self.vertices[i as usize].position;
            transform.transform_point3(Vec3::new(v[0], v[1], v[2]))
        };
        self.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| [p(t[0]), p(t[1]), p(t[2])])
            .collect()
    }

    /// Split a large mesh into pieces by the horizontal `cell` its triangles'
    /// centres fall in, so frustum culling can skip the parts out of view.
    /// Pieces keep the material and the triangles' original order, and come
    /// out in order of their first triangle. A small or skinned mesh stays
    /// one piece.
    pub fn split_into_cells(&self, cell: f32) -> Vec<Mesh3D> {
        if cell <= 0.0 || self.indices.len() < 3 || self.skinning.is_some() {
            return vec![self.clone()];
        }
        let mut order: Vec<(i32, i32)> = Vec::new();
        let mut groups: std::collections::HashMap<(i32, i32), Vec<u32>> =
            std::collections::HashMap::new();
        for tri in self.indices.as_chunks::<3>().0 {
            let mut centre = [0.0f32; 3];
            for &i in tri {
                let p = self.vertices[i as usize].position;
                for axis in 0..3 {
                    centre[axis] += p[axis] / 3.0;
                }
            }
            let key = (
                (centre[0] / cell).floor() as i32,
                (centre[2] / cell).floor() as i32,
            );
            groups
                .entry(key)
                .or_insert_with(|| {
                    order.push(key);
                    Vec::new()
                })
                .extend_from_slice(tri);
        }
        if order.len() <= 1 {
            return vec![self.clone()];
        }
        order
            .into_iter()
            .map(|key| {
                let source = &groups[&key];
                let mut remap: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut vertices = Vec::new();
                let indices = source
                    .iter()
                    .map(|&i| {
                        *remap.entry(i).or_insert_with(|| {
                            vertices.push(self.vertices[i as usize]);
                            vertices.len() as u32 - 1
                        })
                    })
                    .collect();
                Mesh3D {
                    vertices,
                    indices,
                    ..self.clone_without_geometry()
                }
            })
            .collect()
    }

    fn clone_without_geometry(&self) -> Mesh3D {
        Mesh3D {
            vertices: Vec::new(),
            indices: Vec::new(),
            material: self.material,
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            texture_index: self.texture_index,
            normal_texture_index: self.normal_texture_index,
            skinning: None,
        }
    }

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
            metallic_factor: 0.0,
            roughness_factor: 0.7,
            texture_index: None,
            normal_texture_index: None,
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
            metallic_factor: 0.0,
            roughness_factor: 0.7,
            texture_index: None,
            normal_texture_index: None,
            skinning: None,
        }
    }

    /// Build a Y-up heightfield grid centered on the origin.
    ///
    /// `heights` holds `resolution²` row-major samples spanning
    /// `-size/2..=size/2`; short inputs read as `0.0` (the scene validator
    /// rejects malformed grids before upload reaches here).
    pub fn heightfield(
        resolution: usize,
        size: f32,
        heights: &[f32],
        material: Material3D,
    ) -> Self {
        let res = resolution.max(2);
        let half = size.max(0.01) * 0.5;
        let at = |i: usize, j: usize| heights.get(j * res + i).copied().unwrap_or(0.0);
        let mut vertices = Vec::with_capacity(res * res);
        for j in 0..res {
            for i in 0..res {
                let x = (i as f32 / (res - 1) as f32 - 0.5) * half * 2.0;
                let z = (j as f32 / (res - 1) as f32 - 0.5) * half * 2.0;
                let step = half * 2.0 / (res - 1) as f32;
                let dx = at((i + 1).min(res - 1), j) - at(i.saturating_sub(1), j);
                let dz = at(i, (j + 1).min(res - 1)) - at(i, j.saturating_sub(1));
                let (nx, ny, nz) = (-dx / (2.0 * step), 1.0, -dz / (2.0 * step));
                let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
                vertices.push(MeshVertex3D::new(
                    [x, at(i, j), z],
                    [nx / len, ny / len, nz / len],
                    [i as f32 / (res - 1) as f32, j as f32 / (res - 1) as f32],
                ));
            }
        }
        let mut indices = Vec::with_capacity((res - 1) * (res - 1) * 6);
        for j in 0..res - 1 {
            for i in 0..res - 1 {
                let a = (j * res + i) as u32;
                let b = a + 1;
                let c = a + res as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, c, b, b, c, d]);
            }
        }
        Self {
            vertices,
            indices,
            material,
            metallic_factor: 0.0,
            roughness_factor: 0.7,
            texture_index: None,
            normal_texture_index: None,
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
            metallic_factor: 0.0,
            roughness_factor: 0.7,
            texture_index: None,
            normal_texture_index: None,
            skinning: None,
        }
    }

    /// Build a low-poly UV sphere centered on the origin.
    ///
    /// The primitive is intentionally modest (latitudinal rings and a
    /// longitudinal seam) so it is suitable for distant sun/moon discs and
    /// other lightweight atmospheric props.
    pub fn uv_sphere(radius: f32, segments: usize, rings: usize, material: Material3D) -> Self {
        let radius = radius.max(0.01);
        let segments = segments.max(8);
        let rings = rings.max(4);
        let mut vertices = Vec::with_capacity((segments + 1) * (rings + 1));
        let mut indices = Vec::with_capacity(segments * rings * 6);
        for ring in 0..=rings {
            let v = ring as f32 / rings as f32;
            let theta = v * std::f32::consts::PI;
            let (sin_theta, cos_theta) = theta.sin_cos();
            for segment in 0..=segments {
                let u = segment as f32 / segments as f32;
                let phi = u * std::f32::consts::TAU;
                let (sin_phi, cos_phi) = phi.sin_cos();
                let normal = [sin_theta * cos_phi, cos_theta, sin_theta * sin_phi];
                vertices.push(MeshVertex3D::new(
                    [normal[0] * radius, normal[1] * radius, normal[2] * radius],
                    normal,
                    [u, v],
                ));
            }
        }
        let row = segments + 1;
        for ring in 0..rings {
            for segment in 0..segments {
                let a = (ring * row + segment) as u32;
                let b = a + 1;
                let c = ((ring + 1) * row + segment) as u32;
                let d = c + 1;
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        Self {
            vertices,
            indices,
            material,
            metallic_factor: 0.0,
            roughness_factor: 0.7,
            texture_index: None,
            normal_texture_index: None,
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
    /// Optional local screen-space occlusion window. `x` remains the floor
    /// receiver sentinel when negative, or emission strength when positive.
    /// `y` carries authored ambient-occlusion strength,
    /// `z` is the soft reveal radius in NDC, and `w` is the transparency
    /// strength at its centre.
    pub occlusion_window: [f32; 4],
}

impl Material3D {
    /// Create an opaque material from RGB values in `0..=1`.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self {
            base_color: [r, g, b, 1.0],
            texture: None,
            occlusion_window: [0.0; 4],
        }
    }

    /// Set the material's contact/cavity occlusion strength.
    ///
    /// The value is deliberately stored in the existing window payload so
    /// adding AO does not change the public draw/uniform layout. `0` keeps a
    /// material at the renderer's neutral analytic AO; `1` gives props the
    /// strongest grounded contact.
    pub fn with_ambient_occlusion(mut self, strength: f32) -> Self {
        self.occlusion_window[1] = strength.clamp(0.0, 1.0);
        self
    }

    /// Light added to whatever lies behind (lamplight pooled on the ground, sun flecks, glowing
    /// motes): the textured base colour times its alpha, faded by the haze, not lit, casting no
    /// shadow and writing no depth. The base colour may exceed 1 (it is HDR, and blooms).
    pub fn additive(base_color: [f32; 4]) -> Self {
        Self {
            base_color,
            texture: None,
            occlusion_window: [ADDITIVE_FLAG, 0.0, 0.0, 0.0],
        }
    }

    /// Light added as [`Self::additive`] but never hazed: a sky dome far out round the camera
    /// (everything nearer covers it; where nothing does, its colour shows over the clear).
    pub fn sky(base_color: [f32; 4]) -> Self {
        Self {
            base_color,
            texture: None,
            occlusion_window: [ADDITIVE_FLAG, 0.0, 0.0, -1.0],
        }
    }

    /// Whether this material adds light rather than covering what lies behind.
    pub fn is_additive(&self) -> bool {
        let x = self.occlusion_window[0];
        x <= ADDITIVE_FLAG + 0.5 && x > SHADOW_ONLY_FLAG + 0.5
    }

    /// Seen only by the sun: it casts its (texture-cut) shadow and is never drawn. A canopy of
    /// leaves above the frame, so sunlight reaches the ground dappled through real gaps.
    pub fn shadow_only() -> Self {
        Self {
            base_color: [1.0; 4],
            texture: None,
            occlusion_window: [SHADOW_ONLY_FLAG, 0.0, 0.0, 0.0],
        }
    }

    /// Whether this material only casts a shadow.
    pub fn is_shadow_only(&self) -> bool {
        self.occlusion_window[0] <= SHADOW_ONLY_FLAG + 0.5
    }

    /// Add radiance in the material's textured base color before tonemapping.
    /// Uses the positive range of the existing sentinel, preserving the public
    /// uniform layout. It does not create a light; supply one separately.
    pub fn with_emission(mut self, strength: f32) -> Self {
        self.occlusion_window[0] = if strength.is_finite() {
            strength.clamp(0.0, 16.0)
        } else {
            0.0
        };
        self
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

/// Compact local light used by the forward 3D pass.
///
/// The renderer keeps a small fixed GPU budget and ignores lights beyond the
/// nearest scene-authored set supplied by the caller. This makes candles,
/// torches, and magical props illuminate dense rooms without requiring a
/// deferred G-buffer or hardware ray tracing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLight3D {
    /// World-space centre of the light.
    pub position: Vec3,
    /// Linear RGB light colour.
    pub color: [f32; 3],
    /// Radiant intensity multiplier.
    pub intensity: f32,
    /// Smooth cutoff radius in world units.
    pub radius: f32,
}

impl PointLight3D {
    /// Create a local light with sanitized intensity and radius.
    pub fn new(position: Vec3, color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            position,
            color,
            intensity: intensity.max(0.0),
            radius: radius.max(0.05),
        }
    }
}

/// Celestial lighting and horizon colour resolved for one environment frame.
///
/// The values are deliberately independent from the mesh list. A game can
/// reuse the same frame for a forest, a castle, or a completely different
/// world while the renderer supplies soft moon light, lunar reflections, and
/// twilight colour grading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Celestial3D {
    /// Normalized direction from a surface toward the sun.
    pub sun_direction: Vec3,
    /// Visibility of the sun above the horizon (`0..=1`).
    pub sun_visibility: f32,
    /// Normalized direction from a surface toward the moon.
    pub moon_direction: Vec3,
    /// Visibility of the moon in the sky (`0..=1`).
    pub moon_visibility: f32,
    /// Normalized lunar phase (`0.0` new moon, `0.5` full moon).
    pub moon_phase: f32,
    /// Linear RGB colour of the moon light.
    pub moon_color: [f32; 3],
    /// Intensity of the moon light after phase and horizon attenuation.
    pub moon_intensity: f32,
    /// Linear RGB tint reflected by cool surfaces at night.
    pub reflection_color: [f32; 3],
    /// Strength of the lunar reflection contribution.
    pub reflection_strength: f32,
    /// Warm/cool tint used around sunrise and sunset.
    pub horizon_tint: [f32; 3],
    /// Strength of the horizon tint (`0..=1`).
    pub horizon_strength: f32,
}

impl Default for Celestial3D {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(-0.42, 0.96, 0.24),
            sun_visibility: 1.0,
            moon_direction: Vec3::new(0.24, 0.55, -0.62),
            moon_visibility: 0.0,
            moon_phase: 0.5,
            moon_color: [0.32, 0.42, 0.70],
            moon_intensity: 0.0,
            reflection_color: [0.12, 0.20, 0.42],
            reflection_strength: 0.0,
            horizon_tint: [1.0, 0.32, 0.08],
            horizon_strength: 0.0,
        }
    }
}

/// Distance-based atmospheric haze for a 3D pass.
///
/// The default keeps the renderer backwards-compatible: fog starts beyond the
/// practical camera range and density is zero.  Callers that want a visible
/// atmosphere can use [`Wgpu3DRenderer::render_with_atmosphere`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Atmosphere3D {
    /// Linear RGB color that distant geometry fades toward.
    pub fog_color: [f32; 3],
    /// Distance where the linear haze ramp begins.
    pub fog_start: f32,
    /// Distance where the linear haze ramp reaches full strength.
    pub fog_end: f32,
    /// Exponential distance haze contribution.
    pub fog_density: f32,
    /// Extra haze applied to tall silhouettes, useful for treelines/mist.
    pub height_falloff: f32,
}

impl Default for Atmosphere3D {
    fn default() -> Self {
        Self {
            fog_color: [0.0, 0.0, 0.0],
            fog_start: 100_000.0,
            fog_end: 100_001.0,
            fog_density: 0.0,
            height_falloff: 0.0,
        }
    }
}

/// Reusable visual mood for a 3D scene.
///
/// Presets only describe lighting, fog, the clear colour, and celestial
/// timing. They do not select meshes or materials, so a game can swap the
/// atmosphere without rebuilding its world assets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvironmentPreset3D {
    /// Lush, misty daylight used by the living nature scene.
    #[default]
    Lived,
    /// Purple/red nightfall suited to demonic rooms and battlefields.
    Demonic,
}

impl EnvironmentPreset3D {
    /// Human-readable name suitable for a debug title or settings menu.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lived => "Vivida",
            Self::Demonic => "Demoníaca",
        }
    }

    /// Sample this preset at a point in the day/night cycle.
    ///
    /// `phase` wraps at one: `0.0` is midnight, `0.5` is noon.  Values outside
    /// that range are accepted, which makes callers free to feed an unwrapped
    /// clock directly into the sampler.
    pub fn sample(self, phase: f32) -> EnvironmentFrame3D {
        self.sample_with_moon_phase(phase, 0.5)
    }

    /// Sample this preset with an explicit lunar phase.
    ///
    /// `moon_phase` wraps at one: `0.0` is a new moon and `0.5` is a full
    /// moon. Keeping this input separate from the daily phase lets a game run
    /// a long lunar calendar while its day/night clock loops every few
    /// minutes.
    pub fn sample_with_moon_phase(self, phase: f32, moon_phase: f32) -> EnvironmentFrame3D {
        let phase = if phase.is_finite() {
            phase.rem_euclid(1.0)
        } else {
            0.5
        };
        let moon_phase = if moon_phase.is_finite() {
            moon_phase.rem_euclid(1.0)
        } else {
            0.5
        };
        let daylight = daylight_factor(phase);
        let mut frame = match self {
            Self::Lived => environment_frame(
                Lighting3D {
                    direction_to_light: Vec3::new(-0.42, 0.96, 0.24),
                    key_color: [1.30, 1.18, 0.88],
                    ambient_color: [0.35, 0.45, 0.32],
                    bounce_color: [0.12, 0.22, 0.10],
                    shadow_extent: 28.0,
                },
                Lighting3D {
                    direction_to_light: Vec3::new(0.24, 0.55, -0.62),
                    key_color: [0.26, 0.30, 0.36],
                    ambient_color: [0.14, 0.17, 0.19],
                    bounce_color: [0.10, 0.12, 0.13],
                    shadow_extent: 24.0,
                },
                Atmosphere3D {
                    fog_color: [0.20, 0.31, 0.25],
                    fog_start: 19.0,
                    fog_end: 54.0,
                    fog_density: 0.0028,
                    height_falloff: 0.016,
                },
                Atmosphere3D {
                    fog_color: [0.10, 0.12, 0.15],
                    fog_start: 12.0,
                    fog_end: 42.0,
                    fog_density: 0.004,
                    height_falloff: 0.024,
                },
                [0.035, 0.080, 0.095, 1.0],
                [0.012, 0.017, 0.026, 1.0],
                daylight,
            ),
            Self::Demonic => environment_frame(
                Lighting3D {
                    direction_to_light: Vec3::new(-0.56, 0.86, 0.18),
                    key_color: [1.22, 0.70, 0.78],
                    ambient_color: [0.30, 0.12, 0.38],
                    bounce_color: [0.24, 0.035, 0.16],
                    shadow_extent: 28.0,
                },
                Lighting3D {
                    direction_to_light: Vec3::new(0.24, 0.48, -0.60),
                    key_color: [0.32, 0.07, 0.18],
                    ambient_color: [0.11, 0.03, 0.20],
                    bounce_color: [0.17, 0.015, 0.08],
                    shadow_extent: 24.0,
                },
                Atmosphere3D {
                    fog_color: [0.24, 0.07, 0.19],
                    fog_start: 15.0,
                    fog_end: 48.0,
                    fog_density: 0.004,
                    height_falloff: 0.022,
                },
                Atmosphere3D {
                    fog_color: [0.08, 0.008, 0.075],
                    fog_start: 9.0,
                    fog_end: 38.0,
                    fog_density: 0.006,
                    height_falloff: 0.032,
                },
                [0.12, 0.018, 0.12, 1.0],
                [0.018, 0.002, 0.035, 1.0],
                daylight,
            ),
        };
        frame.celestial = celestial_for_phase(self, phase, moon_phase);
        // Rotate the directional shadow caster with the sun through the same
        // eased visibility curve used by the sky. At night the preset's
        // authored key direction remains as a quiet fill light; the blend
        // prevents a one-frame shadow snap at either horizon.
        let shadow_mix = frame.celestial.sun_visibility * 0.95;
        frame.lighting.direction_to_light = frame
            .lighting
            .direction_to_light
            .lerp(frame.celestial.sun_direction, shadow_mix)
            .normalize_or_zero();
        apply_twilight(&mut frame);
        frame
    }
}

/// Lighting and atmospheric values resolved for one instant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentFrame3D {
    /// Directional, ambient, and bounce lighting for the frame.
    pub lighting: Lighting3D,
    /// Distance haze for the frame.
    pub atmosphere: Atmosphere3D,
    /// RGBA clear colour in linear-ish 0..=1 components.
    pub clear_color: [f32; 4],
    /// Sun/moon positions and transient horizon lighting for the frame.
    pub celestial: Celestial3D,
}

impl Default for EnvironmentFrame3D {
    fn default() -> Self {
        EnvironmentPreset3D::Lived.sample(0.5)
    }
}

/// The finish of a toon scene, laid over the frame: ink lines found in the depth buffer
/// (silhouettes where the depth jumps, creases where two planes meet, fading into the distance)
/// and a colour grade after tone mapping. The default draws no lines and leaves colour alone, so
/// scenes that never set one look exactly as before.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Finish3D {
    /// How dark the ink lines are: 0 draws none, 1 is full ink.
    pub ink: f32,
    /// Linear RGB of the ink, mixed into the darkened surface colour.
    pub ink_color: [f32; 3],
    /// How far apart the depth samples are, in pixels of the target (the lines' weight).
    pub ink_thickness: f32,
    /// Distance from the camera where the lines start to fade, and where they are gone.
    pub ink_fade: [f32; 2],
    /// Colour saturation after tone mapping (1 leaves it).
    pub saturation: f32,
    /// Contrast about mid grey after tone mapping (1 leaves it).
    pub contrast: f32,
    /// How strongly what shines past `bloom_threshold` glows round itself (0: no bloom).
    pub bloom: f32,
    /// The linear brightness (before tone mapping) where the glow starts.
    pub bloom_threshold: f32,
    /// Exposure before tone mapping (1 leaves it).
    pub exposure: f32,
    /// Contact shadow where surfaces meet (screen-space ambient occlusion from the depth, for an
    /// orthographic camera): how dark at most (0: none).
    pub occlusion: f32,
    /// How far the contact shadow reaches, in metres.
    pub occlusion_radius: f32,
    /// Light bounced off what is on screen (screen-space global illumination): how much of it
    /// lights the surfaces round (0: none, 1: as gathered).
    pub gi: f32,
    /// How far the bounced light travels, in metres.
    pub gi_radius: f32,
    /// How much of the flat ambient light (sky and ground fill) stays while the bounced light is
    /// on: the bounce takes over part of what the flat fill only stood for, so rooms are lit
    /// from what is lit in them (1 keeps it all).
    pub gi_ambient: f32,
}

impl Default for Finish3D {
    fn default() -> Self {
        Self {
            ink: 0.0,
            ink_color: [0.023, 0.010, 0.004],
            ink_thickness: 2.0,
            ink_fade: [60.0, 90.0],
            saturation: 1.0,
            contrast: 1.0,
            bloom: 0.0,
            bloom_threshold: 1.0,
            exposure: 1.0,
            occlusion: 0.0,
            occlusion_radius: 0.5,
            gi: 0.0,
            gi_radius: 3.0,
            gi_ambient: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OutlineUniform {
    /// strength, sample offset (px), fade start, fade end
    params: [f32; 4],
    /// ink rgb, 1 for an orthographic camera
    ink: [f32; 4],
    /// near, far, target width, target height
    clip: [f32; 4],
    /// silhouette lo, hi; crease lo, hi
    edges: [f32; 4],
    /// world size of a target pixel (orthographic), unused
    scale: [f32; 4],
}

/// Small reusable day/night clock that resolves an [`EnvironmentPreset3D`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayNightCycle3D {
    preset: EnvironmentPreset3D,
    phase: f32,
    cycle_seconds: f32,
    lunar_phase: f32,
    lunar_cycle_days: f32,
}

impl Default for DayNightCycle3D {
    fn default() -> Self {
        Self::new(EnvironmentPreset3D::Lived, 180.0)
    }
}

impl DayNightCycle3D {
    /// Create a cycle beginning at noon.
    ///
    /// A cycle shorter than one second is clamped to one second to keep an
    /// accidental zero duration from producing infinities.
    pub fn new(preset: EnvironmentPreset3D, cycle_seconds: f32) -> Self {
        Self {
            preset,
            phase: 0.5,
            cycle_seconds: sanitize_cycle_seconds(cycle_seconds),
            // Start at full moon so the first night is legible in a preview.
            lunar_phase: 0.5,
            lunar_cycle_days: 28.0,
        }
    }

    /// Preset currently resolved by this cycle.
    pub const fn preset(self) -> EnvironmentPreset3D {
        self.preset
    }

    /// Select a different visual preset without changing time of day.
    pub fn set_preset(&mut self, preset: EnvironmentPreset3D) {
        self.preset = preset;
    }

    /// Normalized clock phase (`0.0` midnight, `0.5` noon).
    pub const fn phase(self) -> f32 {
        self.phase
    }

    /// Set the normalized clock phase. Non-finite values are ignored.
    pub fn set_phase(&mut self, phase: f32) {
        if phase.is_finite() {
            self.phase = phase.rem_euclid(1.0);
        }
    }

    /// Duration of one complete day/night cycle in seconds.
    pub const fn cycle_seconds(self) -> f32 {
        self.cycle_seconds
    }

    /// Set the cycle duration, clamped to at least one second.
    pub fn set_cycle_seconds(&mut self, cycle_seconds: f32) {
        if cycle_seconds.is_finite() {
            self.cycle_seconds = cycle_seconds.max(1.0);
        }
    }

    /// Normalized lunar phase (`0.0` new moon, `0.5` full moon).
    pub const fn lunar_phase(self) -> f32 {
        self.lunar_phase
    }

    /// Set the normalized lunar phase. Non-finite values are ignored.
    pub fn set_lunar_phase(&mut self, lunar_phase: f32) {
        if lunar_phase.is_finite() {
            self.lunar_phase = lunar_phase.rem_euclid(1.0);
        }
    }

    /// Number of day/night loops in one complete lunar cycle.
    pub const fn lunar_cycle_days(self) -> f32 {
        self.lunar_cycle_days
    }

    /// Set the lunar cycle length, clamped to one day.
    pub fn set_lunar_cycle_days(&mut self, lunar_cycle_days: f32) {
        if lunar_cycle_days.is_finite() {
            self.lunar_cycle_days = lunar_cycle_days.max(1.0);
        }
    }

    /// Advance the clock by elapsed seconds and wrap at one day.
    pub fn advance(&mut self, delta_seconds: f32) {
        if delta_seconds.is_finite() {
            self.phase = (self.phase + delta_seconds / self.cycle_seconds).rem_euclid(1.0);
            self.lunar_phase = (self.lunar_phase
                + delta_seconds / (self.cycle_seconds * self.lunar_cycle_days))
                .rem_euclid(1.0);
        }
    }

    /// Resolve the current preset and phase into render-ready values.
    pub fn sample(self) -> EnvironmentFrame3D {
        self.preset
            .sample_with_moon_phase(self.phase, self.lunar_phase)
    }
}

fn sanitize_cycle_seconds(seconds: f32) -> f32 {
    if seconds.is_finite() {
        seconds.max(1.0)
    } else {
        180.0
    }
}

fn daylight_factor(phase: f32) -> f32 {
    let sun_height = (phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin();
    // Blend over a broad elevation interval. The sun is still below the
    // horizon while the world is in blue hour, so using the full interval
    // avoids the old day/night palette feeling like a binary toggle.
    smoothstep01((sun_height + 0.68) / 1.18)
}

fn smoothstep01(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn celestial_for_phase(preset: EnvironmentPreset3D, phase: f32, moon_phase: f32) -> Celestial3D {
    let solar_angle = phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    let (solar_sin, solar_cos) = solar_angle.sin_cos();
    let sun_height = solar_sin;
    let sun_direction =
        Vec3::new(-solar_cos * 0.72, sun_height * 0.82, solar_sin * 0.72).normalize_or_zero();
    let moon_direction =
        Vec3::new(solar_cos * 0.62, (-sun_height).max(0.05), -solar_sin * 0.62).normalize_or_zero();
    // Bodies ease through the horizon rather than popping on/off at a single
    // elevation. The wider band also gives the fog and shadow blend time to
    // settle as the camera watches the sky.
    let sun_visibility = smoothstep01((sun_height + 0.10) / 0.72);
    let moon_visibility = smoothstep01((-sun_height + 0.10) / 0.72);
    // Golden/blue hour spans both sides of the horizon. Two eased edges keep
    // sunset colour present while the sun sinks, and likewise during the
    // first minutes before sunrise.
    let twilight_in = smoothstep01((sun_height + 0.75) / 0.45);
    let twilight_out = 1.0 - smoothstep01((sun_height - 0.20) / 0.45);
    let twilight_strength = (twilight_in * twilight_out).clamp(0.0, 1.0);
    let illumination = 0.5 - 0.5 * (moon_phase * std::f32::consts::TAU).cos();
    let night_illumination = moon_visibility * illumination;
    let (moon_color, reflection_color, horizon_tint) = match preset {
        EnvironmentPreset3D::Lived => ([0.50, 0.58, 0.72], [0.20, 0.28, 0.40], [1.00, 0.34, 0.10]),
        EnvironmentPreset3D::Demonic => {
            ([0.58, 0.16, 0.58], [0.42, 0.035, 0.26], [0.96, 0.08, 0.24])
        }
    };
    Celestial3D {
        sun_direction,
        sun_visibility,
        moon_direction,
        moon_visibility,
        moon_phase,
        moon_color,
        moon_intensity: 0.26 * night_illumination,
        reflection_color,
        reflection_strength: 0.38 * night_illumination,
        horizon_tint,
        horizon_strength: 0.42 * twilight_strength,
    }
}

fn apply_twilight(frame: &mut EnvironmentFrame3D) {
    let strength = frame.celestial.horizon_strength;
    if strength <= f32::EPSILON {
        return;
    }
    let tint = frame.celestial.horizon_tint;
    let warm_key = [
        1.06 + tint[0] * 0.20,
        0.18 + tint[1] * 0.72,
        0.05 + tint[2] * 0.58,
    ];
    let warm_ambient = [
        0.10 + tint[0] * 0.12,
        0.03 + tint[1] * 0.20,
        0.04 + tint[2] * 0.18,
    ];
    frame.lighting.key_color = mix_color3(warm_key, frame.lighting.key_color, strength * 0.32);
    frame.lighting.ambient_color =
        mix_color3(warm_ambient, frame.lighting.ambient_color, strength * 0.24);
    let warm_fog = [
        0.025 + tint[0] * 0.11,
        0.010 + tint[1] * 0.048,
        0.018 + tint[2] * 0.050,
    ];
    frame.atmosphere.fog_color = mix_color3(warm_fog, frame.atmosphere.fog_color, strength * 0.18);
    let horizon_clear = [
        0.018 + tint[0] * 0.105,
        0.006 + tint[1] * 0.038,
        0.012 + tint[2] * 0.040,
    ];
    for (channel, target) in frame.clear_color[..3].iter_mut().zip(horizon_clear) {
        *channel = mix_f32(target, *channel, strength * 0.28);
    }
}

fn mix_f32(day: f32, night: f32, daylight: f32) -> f32 {
    night + (day - night) * daylight
}

fn mix_color3(day: [f32; 3], night: [f32; 3], daylight: f32) -> [f32; 3] {
    [
        mix_f32(day[0], night[0], daylight),
        mix_f32(day[1], night[1], daylight),
        mix_f32(day[2], night[2], daylight),
    ]
}

fn mix_vec3(day: Vec3, night: Vec3, daylight: f32) -> Vec3 {
    day.lerp(night, 1.0 - daylight).normalize_or_zero()
}

fn environment_frame(
    day_lighting: Lighting3D,
    night_lighting: Lighting3D,
    day_atmosphere: Atmosphere3D,
    night_atmosphere: Atmosphere3D,
    day_clear: [f32; 4],
    night_clear: [f32; 4],
    daylight: f32,
) -> EnvironmentFrame3D {
    EnvironmentFrame3D {
        lighting: Lighting3D {
            direction_to_light: mix_vec3(
                day_lighting.direction_to_light,
                night_lighting.direction_to_light,
                daylight,
            ),
            key_color: mix_color3(day_lighting.key_color, night_lighting.key_color, daylight),
            ambient_color: mix_color3(
                day_lighting.ambient_color,
                night_lighting.ambient_color,
                daylight,
            ),
            bounce_color: mix_color3(
                day_lighting.bounce_color,
                night_lighting.bounce_color,
                daylight,
            ),
            shadow_extent: mix_f32(
                day_lighting.shadow_extent,
                night_lighting.shadow_extent,
                daylight,
            ),
        },
        atmosphere: Atmosphere3D {
            fog_color: mix_color3(
                day_atmosphere.fog_color,
                night_atmosphere.fog_color,
                daylight,
            ),
            fog_start: mix_f32(
                day_atmosphere.fog_start,
                night_atmosphere.fog_start,
                daylight,
            ),
            fog_end: mix_f32(day_atmosphere.fog_end, night_atmosphere.fog_end, daylight),
            fog_density: mix_f32(
                day_atmosphere.fog_density,
                night_atmosphere.fog_density,
                daylight,
            ),
            height_falloff: mix_f32(
                day_atmosphere.height_falloff,
                night_atmosphere.height_falloff,
                daylight,
            ),
        },
        clear_color: [
            mix_f32(day_clear[0], night_clear[0], daylight),
            mix_f32(day_clear[1], night_clear[1], daylight),
            mix_f32(day_clear[2], night_clear[2], daylight),
            mix_f32(day_clear[3], night_clear[3], daylight),
        ],
        celestial: Celestial3D::default(),
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
    /// Index into the source glTF model's skin list.
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
    /// Local-space bounds `[min, max]` of the current vertices, for frustum
    /// culling. A cell because CPU-skinned meshes update through `&self`.
    bounds: std::cell::Cell<[[f32; 3]; 2]>,
    /// The frame its vertices were last replaced (animated meshes cast moving shadows).
    updated: std::cell::Cell<u64>,
    /// Its skin, when the GPU deforms it (see [`crate::skin3d`]).
    skin: Option<usize>,
    /// Simpler versions of it (about half and a quarter of its triangles, the same vertices),
    /// drawn instead when it is small on screen and the quality asks for it.
    lods: Vec<(wgpu::Buffer, u32)>,
}

/// Triangles a mesh needs before simpler versions of it are made.
const LOD_MIN_TRIANGLES: usize = 1200;

/// Index lists of about half and a quarter of `mesh`'s triangles (meshoptimizer's simplifier,
/// its borders kept), uploaded; none for small meshes.
fn build_lods(device: &wgpu::Device, mesh: &Mesh3D) -> Vec<(wgpu::Buffer, u32)> {
    if mesh.indices.len() / 3 < LOD_MIN_TRIANGLES {
        return Vec::new();
    }
    let bytes: &[u8] = bytemuck::cast_slice(&mesh.vertices);
    let Ok(adapter) =
        meshopt::VertexDataAdapter::new(bytes, std::mem::size_of::<MeshVertex3D>(), 0)
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut previous = mesh.indices.len();
    for (share, error) in [(0.5f32, 0.01f32), (0.25, 0.03)] {
        let target = ((mesh.indices.len() as f32 * share) as usize / 3) * 3;
        let simplified = meshopt::simplify(
            &mesh.indices,
            &adapter,
            target,
            error,
            meshopt::SimplifyOptions::LockBorder,
            None,
        );
        // Not simpler enough to be worth a level.
        if simplified.is_empty() || simplified.len() * 10 > previous * 8 {
            break;
        }
        previous = simplified.len();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-lod-indices"),
            contents: bytemuck::cast_slice(&simplified),
            usage: wgpu::BufferUsages::INDEX,
        });
        out.push((buffer, simplified.len() as u32));
    }
    out
}

impl GpuMesh3D {
    /// The index buffer and count of level `lod` (0: the full mesh; past its levels, its last).
    fn level(&self, lod: u8) -> (&wgpu::Buffer, u32) {
        match lod
            .checked_sub(1)
            .and_then(|l| self.lods.get(l as usize).or(self.lods.last()))
        {
            Some((buffer, count)) if lod > 0 => (buffer, *count),
            _ => (&self.index_buffer, self.index_count),
        }
    }
}

/// Axis-aligned bounds of a vertex set.
fn vertex_bounds(vertices: &[MeshVertex3D]) -> [[f32; 3]; 2] {
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for v in vertices {
        for axis in 0..3 {
            lo[axis] = lo[axis].min(v.position[axis]);
            hi[axis] = hi[axis].max(v.position[axis]);
        }
    }
    [lo, hi]
}

/// True when the box `bounds`, placed by `clip` (projection * view * model),
/// lies entirely outside one plane of the view volume (depth range 0..1).
/// Conservative: anything touching the view is kept.
fn outside_view(clip: &[f32; 16], bounds: [[f32; 3]; 2]) -> bool {
    let [lo, hi] = bounds;
    if !(lo.iter().chain(&hi).all(|v| v.is_finite())) {
        return false;
    }
    let mut outside = [true; 6];
    for corner in 0..8 {
        let p = [
            if corner & 1 == 0 { lo[0] } else { hi[0] },
            if corner & 2 == 0 { lo[1] } else { hi[1] },
            if corner & 4 == 0 { lo[2] } else { hi[2] },
        ];
        let row =
            |r: usize| clip[r] * p[0] + clip[4 + r] * p[1] + clip[8 + r] * p[2] + clip[12 + r];
        let (x, y, z, w) = (row(0), row(1), row(2), row(3));
        outside[0] &= x < -w;
        outside[1] &= x > w;
        outside[2] &= y < -w;
        outside[3] &= y > w;
        outside[4] &= z < 0.0;
        outside[5] &= z > w;
    }
    outside.iter().any(|&o| o)
}

struct GpuTexture3D {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    /// Optional tangent-space normal map paired with the albedo.  Keeping it
    /// in the same bind group means existing [`MeshDraw3D`] callers keep their
    /// lightweight API while textured meshes can opt into normal detail.
    #[allow(dead_code)]
    normal_texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    /// Some texels are transparent enough to be cut out (`discard`). Draws
    /// with fully solid textures use the opaque pipeline, which keeps the
    /// GPU's early depth rejection.
    cutout: bool,
    /// Carries a real normal map (not the flat default).
    normal_map: bool,
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
    moon_direction: [f32; 4],
    moon_color: [f32; 4],
    reflection_color: [f32; 4],
    horizon_tint: [f32; 4],
    fog_color: [f32; 4],
    fog_params: [f32; 4],
    /// World-space point the camera is framing.  The final component carries
    /// the viewport aspect so the shader can keep the reveal circular.
    focus_position: [f32; 4],
    viewport_size: [f32; 4],
    /// `x` stores the active local-light count.
    local_light_meta: [f32; 4],
    /// World position in xyz and cutoff radius in w.
    local_light_positions: [[f32; 4]; MAX_POINT_LIGHTS_3D],
    /// Linear RGB in xyz and intensity in w.
    local_light_colors: [[f32; 4]; MAX_POINT_LIGHTS_3D],
    toon_params: [f32; 4],
    toon_shadow: [f32; 4],
    /// The finish's grade: saturation, contrast; `z` 1 when this frame has a water reflection,
    /// `w` the water level it mirrors.
    grade: [f32; 4],
    /// Fragments with `dot(xyz, world) + w < 0` are not drawn (the reflection pass keeps what is
    /// above the water). (0, 0, 0, 1) draws everything.
    clip_plane: [f32; 4],
    /// The bounced light's result: world → clip of the frame it was gathered for, that camera's
    /// position (w: strength, 0 off) and view axis.
    gi_view_projection: [f32; 16],
    gi_camera: [f32; 4],
    gi_forward: [f32; 4],
    /// x: the sun shadow's samples per pixel (see [`Quality3D`]).
    quality: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectUniform {
    model: [f32; 16],
    base_color: [f32; 4],
    occlusion_window: [f32; 4],
    /// Normal matrix (inverse transpose up to scale, mirror-corrected),
    /// computed once per draw instead of once per vertex.
    normal_matrix: [[f32; 4]; 3],
    /// x: the point lights whose reach touches this draw's box (bit i: light i). A light adds
    /// exactly nothing past its radius, so the shader walks only these: the same picture for
    /// a fraction of the work where many lamps are about.
    light_mask: [u32; 4],
}

/// The lights of `positions` (xyz, radius in w; the first `count`) that reach the box `bounds`
/// placed by `transform`, as bits.
fn light_mask(transform: Mat4, bounds: [[f32; 3]; 2], positions: &[[f32; 4]], count: usize) -> u32 {
    if count == 0 {
        return 0;
    }
    let count = count.min(32);
    let [lo, hi] = bounds;
    let center = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let half = [
        (hi[0] - lo[0]) * 0.5,
        (hi[1] - lo[1]) * 0.5,
        (hi[2] - lo[2]) * 0.5,
    ];
    // Column-major: m[c * 4 + r].
    let m = transform.to_cols_array();
    // The placed box: its centre, and its half size along each world axis (the absolute matrix
    // on the half size), a centimetre more for the rounding of the fragments' positions.
    let mut world = [0.0f32; 3];
    let mut extent = [0.0f32; 3];
    for r in 0..3 {
        world[r] = m[r] * center[0] + m[4 + r] * center[1] + m[8 + r] * center[2] + m[12 + r];
        extent[r] =
            m[r].abs() * half[0] + m[4 + r].abs() * half[1] + m[8 + r].abs() * half[2] + 0.01;
    }
    if !world.iter().chain(extent.iter()).all(|v| v.is_finite()) {
        return u32::MAX >> (32 - count);
    }
    let mut mask = 0u32;
    for (index, light) in positions.iter().take(count).enumerate() {
        let mut d2 = 0.0f32;
        for r in 0..3 {
            let outside = ((light[r] - world[r]).abs() - extent[r]).max(0.0);
            d2 += outside * outside;
        }
        let radius = light[3].max(0.05);
        if d2 < radius * radius {
            mask |= 1 << index;
        }
    }
    mask
}

/// Cofactor matrix of the model's upper 3x3 (the inverse transpose up to a
/// positive scale), with the determinant's sign folded in so mirrored
/// transforms keep outward normals. Columns padded to WGSL's `mat3x3`.
fn normal_matrix(model: Mat4) -> [[f32; 4]; 3] {
    let m = model.to_cols_array();
    let a = Vec3::new(m[0], m[1], m[2]);
    let b = Vec3::new(m[4], m[5], m[6]);
    let c = Vec3::new(m[8], m[9], m[10]);
    let sign = if a.dot(b.cross(c)) >= 0.0 { 1.0 } else { -1.0 };
    [b.cross(c), c.cross(a), a.cross(b)].map(|v| [v.x * sign, v.y * sign, v.z * sign, 0.0])
}

const OBJECT_UNIFORM_CAPACITY: usize = 256;
const SHADOW_MAP_SIZE: u32 = 4096;
const MAX_POINT_LIGHTS_3D: usize = 32;
/// The occlusion window's x for additive materials (see [`Material3D::additive`]).
const ADDITIVE_FLAG: f32 = -2.0;
/// The occlusion window's x for shadow-only casters (see [`Material3D::shadow_only`]).
const SHADOW_ONLY_FLAG: f32 = -4.0;
/// How far back along the sun its shadow camera stands, and how deep it sees (metres).
const SHADOW_DISTANCE: f32 = 80.0;
const SHADOW_DEPTH: f32 = 160.0;
/// Casters lower than this (metres) are left out of the shadow map.
const SHADOW_MIN_HEIGHT: f32 = 0.3;

/// How much work the 3D frame does: the game's graphics settings. `Default` is the full look
/// (what the game has always drawn).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quality3D {
    /// Samples per pixel softening the sun's shadow: 16 (full), 8, 4; 0 draws no sun shadows.
    pub shadow_samples: u32,
    /// The light bounced off nearby surfaces (screen-space GI).
    pub bounced_light: bool,
    /// The scene mirrored in still water.
    pub water_reflections: bool,
    /// Bloom levels: 6 (full halo), 3 (lighter), 0 (none).
    pub bloom_levels: u32,
    /// Simpler versions of things small on screen: 0 never (full detail), 1 balanced, 2 for
    /// speed.
    pub lod: u32,
    /// The opaque surfaces' depth drawn first, so hidden pixels are never shaded. Faster; where
    /// two surfaces lie exactly on each other the other one may show.
    pub depth_prepass: bool,
}

impl Default for Quality3D {
    fn default() -> Self {
        Self {
            shadow_samples: 16,
            bounced_light: true,
            water_reflections: true,
            bloom_levels: 6,
            lod: 0,
            depth_prepass: false,
        }
    }
}

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
    /// glTF mesh names, repeated once per imported primitive in `meshes` order.
    ///
    /// This lets game code address authored sub-parts (for example a skeleton's
    /// arms or a chest lid) without depending on fragile primitive indices.
    pub mesh_names: Vec<String>,
    /// Animation clip names advertised by the file.
    pub animations: Vec<String>,
    /// Parsed animation clips, including keyframes for CPU sampling.
    pub animation_clips: Vec<AnimationClip3D>,
    /// Embedded/external texture data in document order.
    pub textures: Vec<TextureData3D>,
    /// The file's image each of `textures` was made from (its index among the file's images).
    pub texture_sources: Vec<usize>,
    /// Bind-pose transform of each primitive's mesh node.
    ///
    /// glTF files exported from FBX tools often keep a unit-conversion scale
    /// on the armature root. The renderer must apply this transform outside
    /// CPU skinning or the mesh appears hundreds of times too large.
    mesh_transforms: Vec<Mat4>,
    /// Node owning each imported primitive, used to animate rigid attachments.
    mesh_nodes: Vec<Option<usize>>,
    /// Source names for animation nodes. Kept alongside the compact pose
    /// representation so game code can apply a small authored correction to
    /// a named joint without exposing the importer internals.
    node_names: Vec<String>,
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

/// One evaluated primitive from an animated glTF model.
#[derive(Debug, Clone)]
pub struct AnimatedMesh3D {
    /// Deformed vertex data for this primitive.
    pub vertices: Vec<MeshVertex3D>,
    /// Current node transform for rigid parts, bind transform for skinned parts.
    pub transform: Mat4,
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
pub(crate) struct NodePose3D {
    pub(crate) parent: Option<usize>,
    pub(crate) translation: Vec3,
    pub(crate) rotation: [f32; 4],
    pub(crate) scale: Vec3,
    pub(crate) matrix: Option<Mat4>,
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

/// A picture to use instead of one a file holds: the file, the picture's index in it and its
/// size, and the replacement (same size), if there is one. Set once by the host (a game showing
/// signs in the player's language).
pub type ImageOverride = fn(&Path, usize, u32, u32) -> Option<image::RgbaImage>;

static IMAGE_OVERRIDE: std::sync::OnceLock<ImageOverride> = std::sync::OnceLock::new();

/// The pictures an override replaced, as the file had them (by file and image index), so the
/// host can put them back (a player going back to the language the files are written in).
type Originals = std::collections::HashMap<(std::path::PathBuf, usize), image::RgbaImage>;

fn originals() -> &'static std::sync::Mutex<Originals> {
    static ORIGINALS: std::sync::OnceLock<std::sync::Mutex<Originals>> = std::sync::OnceLock::new();
    ORIGINALS.get_or_init(Default::default)
}

/// The picture image `index` of file `path` had before an override replaced it (`None` when
/// none did).
pub fn image_original(path: &Path, index: usize) -> Option<image::RgbaImage> {
    originals()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&(path.to_path_buf(), index))
        .cloned()
}

/// Sets how pictures inside model files can be replaced as they load.
pub fn set_image_override(hook: ImageOverride) {
    let _ = IMAGE_OVERRIDE.set(hook);
}

fn apply_image_overrides(path: &Path, images: &mut [gltf::image::Data]) {
    let Some(hook) = IMAGE_OVERRIDE.get() else {
        return;
    };
    for (index, data) in images.iter_mut().enumerate() {
        let Some(rgba) = hook(path, index, data.width, data.height) else {
            continue;
        };
        if rgba.dimensions() != (data.width, data.height) {
            continue;
        }
        let original = match data.format {
            gltf::image::Format::R8G8B8A8 => {
                image::RgbaImage::from_raw(data.width, data.height, data.pixels.clone())
            }
            gltf::image::Format::R8G8B8 => image::RgbaImage::from_raw(
                data.width,
                data.height,
                data.pixels
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .flat_map(|p| [p[0], p[1], p[2], 255])
                    .collect(),
            ),
            _ => None,
        };
        if let Some(original) = original {
            originals()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert((path.to_path_buf(), index), original);
        }
        match data.format {
            gltf::image::Format::R8G8B8A8 => data.pixels = rgba.into_raw(),
            gltf::image::Format::R8G8B8 => {
                data.pixels = rgba.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
            }
            _ => {}
        }
    }
}

/// A glTF file's document, buffers and decoded images, as `gltf::import` gives them, except that
/// an image stored in the file as AVIF (as the release packs them) is decoded too (to RGBA). Any
/// other image goes through the glTF crate's own decoding, unchanged.
fn import_gltf(
    gltf: gltf::Gltf,
    base: Option<&Path>,
) -> Result<
    (
        gltf::Document,
        Vec<gltf::buffer::Data>,
        Vec<gltf::image::Data>,
    ),
    Render3DError,
> {
    let gltf::Gltf { document, blob } = gltf;
    let error = |e: gltf::Error| Render3DError::Gltf(e.to_string());
    let buffers = gltf::import_buffers(&document, base, blob).map_err(error)?;
    let mut images = Vec::new();
    for image in document.images() {
        let file;
        let bytes = match image.source() {
            gltf::image::Source::View { view, .. } => {
                let start = view.offset();
                Some(&buffers[view.buffer().index()].0[start..start + view.length()])
            }
            // A picture beside a `.gltf` (its name as written, spaces as %20).
            gltf::image::Source::Uri { uri, .. } if !uri.starts_with("data:") => {
                file = base.and_then(|base| std::fs::read(base.join(uri.replace("%20", " "))).ok());
                file.as_deref()
            }
            _ => None,
        };
        if let Some(bytes) = bytes {
            if velvet_avif::is_avif(bytes) {
                let rgba = velvet_avif::decode_avif(bytes)
                    .map_err(|e| Render3DError::Gltf(e.to_string()))?;
                let (width, height) = rgba.dimensions();
                images.push(gltf::image::Data {
                    pixels: rgba.into_raw(),
                    format: gltf::image::Format::R8G8B8A8,
                    width,
                    height,
                });
                continue;
            }
        }
        images.push(gltf::image::Data::from_source(image.source(), base, &buffers).map_err(error)?);
    }
    Ok((document, buffers, images))
}

/// The head of a model file packed whole (tools/pack_media.py): `VGLZ`, the format's version
/// (1, little-endian u32), the unpacked length (u64) and then the file as one zlib stream.
const PACKED_MODEL: &[u8; 4] = b"VGLZ";

/// A model file's bytes as glTF: unpacked when the release packed them, else as they are.
pub fn unpack_model_bytes(bytes: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>, Render3DError> {
    if bytes.len() < 16 || &bytes[..4] != PACKED_MODEL {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().expect("4"));
    let length = u64::from_le_bytes(bytes[8..16].try_into().expect("8")) as usize;
    if version != 1 {
        return Err(Render3DError::Gltf(format!(
            "packed model of version {version}"
        )));
    }
    let unpacked = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&bytes[16..], length)
        .map_err(|error| Render3DError::Gltf(format!("packed model: {error:?}")))?;
    if unpacked.len() != length {
        return Err(Render3DError::Gltf("packed model cut short".into()));
    }
    Ok(std::borrow::Cow::Owned(unpacked))
}

impl GltfModel {
    /// Load a `.glb` or `.gltf` file using the bundled glTF importer (or one the release packed).
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Render3DError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|error| Render3DError::Gltf(error.to_string()))?;
        let bytes = unpack_model_bytes(&bytes)?;
        let gltf = gltf::Gltf::from_slice(&bytes)
            .map_err(|error| Render3DError::Gltf(error.to_string()))?;
        let (document, buffers, mut images) =
            import_gltf(gltf, Some(path.parent().unwrap_or(Path::new("./"))))?;
        apply_image_overrides(path, &mut images);
        parse_gltf(document, buffers, images)
    }

    /// Load from in-memory glTF/GLB bytes (or bytes the release packed).
    pub fn load_bytes(bytes: &[u8]) -> Result<Self, Render3DError> {
        let bytes = unpack_model_bytes(bytes)?;
        let gltf = gltf::Gltf::from_slice(&bytes)
            .map_err(|error| Render3DError::Gltf(error.to_string()))?;
        let (document, buffers, images) = import_gltf(gltf, None)?;
        parse_gltf(document, buffers, images)
    }

    /// Rest pose of every node: (name, parent index, translation, rotation
    /// quaternion `[x, y, z, w]`). Procedural animation solves from it.
    pub fn rest_nodes(&self) -> Vec<(String, Option<usize>, Vec3, [f32; 4])> {
        self.nodes
            .iter()
            .zip(&self.node_names)
            .map(|(node, name)| (name.clone(), node.parent, node.translation, node.rotation))
            .collect()
    }

    /// Register a clip built at runtime: rotation tracks and translation
    /// tracks keyed by node name. Returns the clip index; a clip with the
    /// same name is replaced.
    pub fn add_animation_clip(
        &mut self,
        name: &str,
        duration: f32,
        rotations: RotationTracks3D,
        translations: Vec<(String, Vec<(f32, Vec3)>)>,
    ) -> Result<usize, Render3DError> {
        let node = |wanted: &str| {
            self.node_names
                .iter()
                .position(|candidate| candidate == wanted)
                .ok_or_else(|| {
                    Render3DError::Gltf(format!(
                        "clip '{name}' targets node '{wanted}' absent from the skeleton"
                    ))
                })
        };
        let mut channels = Vec::with_capacity(rotations.len() + translations.len());
        for (target, keys) in rotations {
            channels.push(AnimationChannel3D::Rotation {
                node: node(&target)?,
                keys,
            });
        }
        for (target, keys) in translations {
            channels.push(AnimationChannel3D::Translation {
                node: node(&target)?,
                keys,
            });
        }
        let clip = AnimationClip3D {
            name: name.to_string(),
            duration,
            channels,
        };
        if let Some(index) = self
            .animation_clips
            .iter()
            .position(|existing| existing.name == name)
        {
            self.animation_clips[index] = clip;
            return Ok(index);
        }
        self.animations.push(name.to_string());
        self.animation_clips.push(clip);
        Ok(self.animation_clips.len() - 1)
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

    /// Append clips from another model whose skeleton uses the same node names.
    ///
    /// Animation-only glTF files are common in character asset packs. Their
    /// node indices are local to that file, so channels are remapped by exact
    /// node name before they are attached to this model. Existing clip names
    /// are retained, which lets a second animation set omit a duplicate bind
    /// pose without changing clip indices already in use.
    pub fn append_animations_from(&mut self, source: &GltfModel) -> Result<usize, Render3DError> {
        let node_map = source
            .node_names
            .iter()
            .map(|name| {
                self.node_names
                    .iter()
                    .position(|candidate| candidate == name)
            })
            .collect::<Vec<_>>();
        let mut appended = 0;
        for source_clip in &source.animation_clips {
            if self
                .animation_clips
                .iter()
                .any(|clip| clip.name == source_clip.name)
            {
                continue;
            }
            let mut channels = Vec::with_capacity(source_clip.channels.len());
            for channel in &source_clip.channels {
                let source_node = match channel {
                    AnimationChannel3D::Translation { node, .. }
                    | AnimationChannel3D::Rotation { node, .. }
                    | AnimationChannel3D::Scale { node, .. } => *node,
                };
                let Some(node) = node_map.get(source_node).copied().flatten() else {
                    let name = source
                        .node_names
                        .get(source_node)
                        .map(String::as_str)
                        .unwrap_or("<unknown>");
                    return Err(Render3DError::Gltf(format!(
                        "animation '{}' targets node '{}' absent from destination skeleton",
                        source_clip.name, name
                    )));
                };
                channels.push(match channel {
                    AnimationChannel3D::Translation { keys, .. } => {
                        AnimationChannel3D::Translation {
                            node,
                            keys: keys.clone(),
                        }
                    }
                    AnimationChannel3D::Rotation { keys, .. } => AnimationChannel3D::Rotation {
                        node,
                        keys: keys.clone(),
                    },
                    AnimationChannel3D::Scale { keys, .. } => AnimationChannel3D::Scale {
                        node,
                        keys: keys.clone(),
                    },
                });
            }
            self.animation_clips.push(AnimationClip3D {
                name: source_clip.name.clone(),
                duration: source_clip.duration,
                channels,
            });
            self.animations.push(source_clip.name.clone());
            appended += 1;
        }
        Ok(appended)
    }

    /// Appends another file's clips to a skeleton with the same bones but its own proportions (a
    /// realistic body on a stylised rig's bones): only the rotations carry over. Translations are
    /// kept only on the `moving` nodes (a root, the hips), scaled by how much farther that node
    /// rests from its parent here than in the clips' skeleton, so the body keeps its own lengths.
    pub fn append_rotations_from(
        &mut self,
        source: &GltfModel,
        moving: &[&str],
    ) -> Result<usize, Render3DError> {
        let before = self.animation_clips.len();
        let appended = self.append_animations_from(source)?;
        let factors: Vec<Option<f32>> = self
            .node_names
            .iter()
            .enumerate()
            .map(|(node, name)| {
                if !moving.contains(&name.as_str()) {
                    return None;
                }
                let here = self.nodes.get(node)?.translation.length();
                let there = source
                    .node_names
                    .iter()
                    .position(|candidate| candidate == name)
                    .and_then(|index| source.nodes.get(index))
                    .map(|pose| pose.translation.length())
                    .unwrap_or(0.0);
                Some(if there > 1.0e-6 { here / there } else { 1.0 })
            })
            .collect();
        for clip in &mut self.animation_clips[before..] {
            clip.channels.retain_mut(|channel| match channel {
                AnimationChannel3D::Translation { node, keys } => {
                    match factors.get(*node).copied().flatten() {
                        Some(factor) => {
                            for (_, value) in keys.iter_mut() {
                                *value = *value * factor;
                            }
                            true
                        }
                        None => false,
                    }
                }
                _ => true,
            });
        }
        Ok(appended)
    }

    /// Bind-pose local-to-world transform for one imported mesh primitive.
    pub fn mesh_transform(&self, mesh_index: usize) -> Mat4 {
        self.mesh_transforms
            .get(mesh_index)
            .copied()
            .unwrap_or(Mat4::IDENTITY)
    }

    /// Bind-pose global transform for a named node, such as an authored hand
    /// attachment socket.
    pub fn node_transform(&self, node_name: &str) -> Option<Mat4> {
        self.node_transforms(&[node_name])?.into_iter().next()
    }

    /// Bind-pose global transforms for several named attachment nodes.
    pub fn node_transforms(&self, node_names: &[&str]) -> Option<Vec<Mat4>> {
        let nodes = node_names
            .iter()
            .map(|wanted| self.node_names.iter().position(|name| name == wanted))
            .collect::<Option<Vec<_>>>()?;
        let mut globals = vec![None; self.nodes.len()];
        Some(
            nodes
                .into_iter()
                .map(|node| resolve_global(node, &self.nodes, &mut globals))
                .collect(),
        )
    }

    /// Sample a named node's global transform from an animation clip.
    ///
    /// This lets gameplay attach props to animated sockets without estimating
    /// hand positions from the character's body bounds.
    pub fn sample_animation_node_transform(
        &self,
        clip_index: usize,
        time: f32,
        node_name: &str,
    ) -> Option<Mat4> {
        self.sample_animation_node_transforms(clip_index, time, &[node_name])?
            .into_iter()
            .next()
    }

    /// Sample several named nodes in one animation evaluation.
    pub fn sample_animation_node_transforms(
        &self,
        clip_index: usize,
        time: f32,
        node_names: &[&str],
    ) -> Option<Vec<Mat4>> {
        let nodes = node_names
            .iter()
            .map(|wanted| self.node_names.iter().position(|name| name == wanted))
            .collect::<Option<Vec<_>>>()?;
        let animated = self.sampled_nodes(clip_index, time)?;
        let mut globals = vec![None; animated.len()];
        Some(
            nodes
                .into_iter()
                .map(|node| resolve_global(node, &animated, &mut globals))
                .collect(),
        )
    }

    /// The skeleton's rest pose.
    pub fn rest_pose(&self) -> crate::skin3d::Pose3D {
        crate::skin3d::Pose3D {
            nodes: self.nodes.clone(),
        }
    }

    /// The pose of clip `clip_index` at `time` seconds (looping).
    pub fn pose(&self, clip_index: usize, time: f32) -> Option<crate::skin3d::Pose3D> {
        Some(crate::skin3d::Pose3D {
            nodes: self.sampled_nodes(clip_index, time)?,
        })
    }

    /// Moves a named node of `pose` by `offset` in its parent's frame (small fit corrections
    /// after the clip, such as arms held a little away from the body).
    pub fn offset_pose_node(
        &self,
        pose: &mut crate::skin3d::Pose3D,
        node_name: &str,
        offset: Vec3,
    ) {
        if let Some(index) = self.node_names.iter().position(|n| n == node_name) {
            if let Some(node) = pose.nodes.get_mut(index) {
                node.translation += offset;
                node.matrix = None;
            }
        }
    }

    /// Sets a named node's rotation in `pose` (in its parent's frame, `[x, y, z, w]`): for bones a
    /// game moves itself after the clip, such as hair swung by its own physics.
    pub fn set_pose_node_rotation(
        &self,
        pose: &mut crate::skin3d::Pose3D,
        node_name: &str,
        rotation: [f32; 4],
    ) {
        if let Some(index) = self.node_names.iter().position(|n| n == node_name) {
            if let Some(node) = pose.nodes.get_mut(index) {
                node.rotation = rotation;
                node.matrix = None;
            }
        }
    }

    /// A named node's global transform in `pose` (sockets for props in a hand).
    pub fn pose_node_transform(
        &self,
        pose: &crate::skin3d::Pose3D,
        node_name: &str,
    ) -> Option<Mat4> {
        let index = self.node_names.iter().position(|n| n == node_name)?;
        let mut globals = vec![None; pose.nodes.len()];
        Some(resolve_global(index, &pose.nodes, &mut globals))
    }

    /// The joint matrices that deform mesh `mesh_index` into `pose` (for
    /// [`Wgpu3DRenderer::set_skin_joints`]); `None` for a mesh without a skin.
    pub fn joint_matrices(
        &self,
        mesh_index: usize,
        pose: &crate::skin3d::Pose3D,
    ) -> Option<Vec<Mat4>> {
        let skinning = self.meshes.get(mesh_index)?.skinning.as_ref()?;
        let skin = self.skins.get(skinning.skin_index)?;
        let mut globals = vec![None; pose.nodes.len()];
        let mesh_inverse = skinning
            .mesh_node
            .and_then(|node| resolve_global(node, &pose.nodes, &mut globals).inverse())
            .unwrap_or(Mat4::IDENTITY);
        Some(
            skin.joints
                .iter()
                .enumerate()
                .map(|(index, joint)| {
                    let global = resolve_global(*joint, &pose.nodes, &mut globals);
                    let inverse_bind = skin
                        .inverse_bind_matrices
                        .get(index)
                        .copied()
                        .unwrap_or(Mat4::IDENTITY);
                    mesh_inverse * global * inverse_bind
                })
                .collect(),
        )
    }

    /// Each primitive's transform in `pose`: rigid parts (hats, weapons) follow their animated
    /// node, skinned parts keep their bind transform (the skin moves them). The same as
    /// [`Self::sample_animation_meshes`]'s transforms, without deforming anything.
    pub fn pose_mesh_transforms(&self, pose: &crate::skin3d::Pose3D) -> Vec<Mat4> {
        let mut globals = vec![None; pose.nodes.len()];
        (0..self.meshes.len())
            .map(|i| {
                if self.meshes[i].skinning.is_some() {
                    self.mesh_transform(i)
                } else {
                    self.mesh_nodes
                        .get(i)
                        .copied()
                        .flatten()
                        .map(|node| resolve_global(node, &pose.nodes, &mut globals))
                        .unwrap_or_else(|| self.mesh_transform(i))
                }
            })
            .collect()
    }

    fn sampled_nodes(&self, clip_index: usize, time: f32) -> Option<Vec<NodePose3D>> {
        let clip = self.animation_clips.get(clip_index)?;
        let mut nodes = self.nodes.clone();
        let sample_time = if clip.duration > 0.0 && time.is_finite() {
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
        Some(nodes)
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
        self.sample_animation_with_node_offsets(mesh_index, clip_index, time, &[])
    }

    /// Evaluate every primitive for one animation sample.
    ///
    /// Skinned parts receive CPU-deformed vertices. Rigid pieces such as hats,
    /// capes, and armor receive their animated node transform, so accessories
    /// remain attached instead of floating in bind pose. Joint matrices are
    /// shared across primitives with the same skin and mesh node.
    pub fn sample_animation_meshes(
        &self,
        clip_index: usize,
        time: f32,
    ) -> Option<Vec<AnimatedMesh3D>> {
        let nodes = self.sampled_nodes(clip_index, time)?;

        let mut globals = vec![None; nodes.len()];
        let mut joint_cache = BTreeMap::<(usize, Option<usize>), Vec<Mat4>>::new();
        let mut output = Vec::with_capacity(self.meshes.len());
        for (mesh_index, mesh) in self.meshes.iter().enumerate() {
            if let Some(skinning) = &mesh.skinning {
                let key = (skinning.skin_index, skinning.mesh_node);
                if let std::collections::btree_map::Entry::Vacant(entry) = joint_cache.entry(key) {
                    let skin = self.skins.get(skinning.skin_index)?;
                    let mesh_inverse = skinning
                        .mesh_node
                        .and_then(|node| resolve_global(node, &nodes, &mut globals).inverse())
                        .unwrap_or(Mat4::IDENTITY);
                    let joint_matrices = skin
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
                    entry.insert(joint_matrices);
                }
                let joint_matrices = joint_cache.get(&key)?;
                let mut vertices = mesh.vertices.clone();
                for (index, vertex) in vertices.iter_mut().enumerate() {
                    let Some(joints) = skinning.joints.get(index) else {
                        continue;
                    };
                    let weights = skinning.weights.get(index).copied().unwrap_or([0.0; 4]);
                    let bind_position =
                        Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2]);
                    let bind_normal =
                        Vec3::new(vertex.normal[0], vertex.normal[1], vertex.normal[2]);
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
                output.push(AnimatedMesh3D {
                    vertices,
                    transform: self.mesh_transform(mesh_index),
                });
            } else {
                let transform = self
                    .mesh_nodes
                    .get(mesh_index)
                    .copied()
                    .flatten()
                    .map(|node| resolve_global(node, &nodes, &mut globals))
                    .unwrap_or_else(|| self.mesh_transform(mesh_index));
                output.push(AnimatedMesh3D {
                    vertices: mesh.vertices.clone(),
                    transform,
                });
            }
        }
        Some(output)
    }

    /// Sample a skinned mesh while applying constant local translation offsets
    /// to named joints after the animation channels are evaluated. This is
    /// useful for small, deliberate fit corrections on imported characters
    /// (for example, moving both upper arms a few centimeters away from the
    /// torso) while preserving the authored animation itself.
    pub fn sample_animation_with_node_offsets(
        &self,
        mesh_index: usize,
        clip_index: usize,
        time: f32,
        offsets: &[(&str, Vec3)],
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
        for (name, offset) in offsets {
            let Some(node_index) = self
                .node_names
                .iter()
                .position(|node_name| node_name == name)
            else {
                continue;
            };
            if let Some(pose) = nodes.get_mut(node_index) {
                pose.translation += *offset;
                pose.matrix = None;
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
    let node_names = document
        .nodes()
        .map(|node| node.name().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
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
    let mut mesh_names = Vec::new();
    let mut mesh_transforms = Vec::new();
    let mut mesh_nodes = Vec::new();
    for mesh in document.meshes() {
        let mesh_name = mesh.name().unwrap_or("unnamed").to_owned();
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
            let metallic_factor = pbr.metallic_factor();
            let roughness_factor = pbr.roughness_factor();
            let texture_index = pbr
                .base_color_texture()
                .map(|texture| texture.texture().source().index());
            let normal_texture_index = primitive
                .material()
                .normal_texture()
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
                    occlusion_window: [0.0; 4],
                },
                metallic_factor,
                roughness_factor,
                texture_index,
                normal_texture_index,
                skinning,
            });
            mesh_names.push(mesh_name.clone());
            mesh_transforms.push(
                mesh_node_transforms
                    .get(mesh.index())
                    .copied()
                    .unwrap_or(Mat4::IDENTITY),
            );
            mesh_nodes.push(mesh_node);
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
    // Keep only images that are actually used by a mesh.  Albedo and normal
    // maps share one compact table so the preview can upload each authored
    // material as a texture pair without retaining unrelated ORM/detail maps.
    let mut image_remap = vec![None; images.len()];
    let mut used_images = Vec::new();
    for mesh in &meshes {
        for source_index in [mesh.texture_index, mesh.normal_texture_index]
            .into_iter()
            .flatten()
        {
            if source_index < image_remap.len() && image_remap[source_index].is_none() {
                image_remap[source_index] = Some(used_images.len());
                used_images.push(source_index);
            }
        }
    }
    for mesh in &mut meshes {
        mesh.texture_index = mesh
            .texture_index
            .and_then(|source_index| image_remap.get(source_index).copied().flatten());
        mesh.normal_texture_index = mesh
            .normal_texture_index
            .and_then(|source_index| image_remap.get(source_index).copied().flatten());
    }
    let mut images = images.into_iter().map(Some).collect::<Vec<_>>();
    let texture_sources = used_images.clone();
    let textures = used_images
        .into_iter()
        .filter_map(|source_index| images.get_mut(source_index)?.take())
        .map(texture_data)
        .collect();
    Ok(GltfModel {
        meshes,
        mesh_names,
        animations,
        animation_clips,
        textures,
        texture_sources,
        mesh_transforms,
        mesh_nodes,
        node_names,
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
            for pair in image.pixels.as_chunks::<2>().0 {
                rgba.extend_from_slice(&[pair[0], pair[1], 0, 255]);
            }
        }
        gltf::image::Format::R8G8B8 => {
            for rgb in image.pixels.as_chunks::<3>().0 {
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
    shading_style: ShadingStyle3D,
    directional_shadows: bool,
    toon_style: ToonStyle3D,
    /// Shading pipelines by `[cutout][normal_map]`.
    pipelines: [[wgpu::RenderPipeline; 2]; 2],
    /// The depth pre-pass: depth only, and the opaque draws after it testing LessEqual.
    probe_depth_pipeline: wgpu::RenderPipeline,
    probe_equal_pipelines: [wgpu::RenderPipeline; 2],
    /// Soft translucent surfaces (smoke, mist, fading markers): blended without the
    /// alpha-to-coverage dither, and without writing depth.
    translucent_pipeline: wgpu::RenderPipeline,
    /// Additive light (see [`Material3D::additive`]).
    additive_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    /// Depth only, no fragment stage: solid casters without a reveal window (the GPU rejects
    /// hidden fragments early; the cut-out pipeline above cannot).
    shadow_solid_pipeline: wgpu::RenderPipeline,
    /// The shadow map was cleared and nothing has been drawn into it since.
    shadow_map_empty: bool,
    /// Draws of the last frame that needed the alpha-cutout pipeline.
    pub last_cutout_draws: usize,
    /// Draws of the last frame skipped because they were out of view.
    pub last_culled_draws: usize,
    /// Triangles the last frame actually submitted to the main pass.
    pub last_drawn_triangles: usize,
    /// The last frame redrew the whole cached shadow map at once (why it was slow, if it was).
    pub last_shadow_full: bool,
    /// Per draw of the last frame: triangles submitted (0 when culled).
    pub last_draw_triangles: Vec<u32>,
    /// Skip draws whose bounds are entirely off screen (on by default; off
    /// only to verify that culling never changes the image).
    pub frustum_culling: bool,
    /// No haze at all, whatever the atmosphere says (a view from far above, for a map).
    pub no_fog: bool,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    object_buffer: wgpu::Buffer,
    object_bind_group: wgpu::BindGroup,
    object_bind_group_layout: wgpu::BindGroupLayout,
    object_stride: u64,
    object_capacity: usize,
    object_upload: Vec<u8>,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    meshes: Vec<GpuMesh3D>,
    textures: Vec<GpuTexture3D>,
    white_texture: TextureHandle,
    sample_count: u32,
    /// The HDR frame, its bloom and the tone-mapping composite into the caller's target.
    post: crate::post3d::Post3D,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    depth_size: (u32, u32),
    #[allow(dead_code)]
    shadow_texture: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    shadow_bind_group: wgpu::BindGroup,
    /// Group 3's layout and the shadow map's sampler: the group is made anew each frame, with
    /// the bounced light gathered for it.
    shadow_bind_group_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow_cache: ShadowCache,
    /// The graphics settings.
    quality: Quality3D,
    /// Puts the cached map back into the frame's map over small rectangles only (where last
    /// frame's moving casters were), instead of copying all of its 4096² every frame.
    shadow_restore: ShadowRestore,
    /// The light bounced off what is on screen.
    gi: crate::gi3d::ScreenGi,
    /// Simulated water: bodies, droplets, their solvers.
    water: crate::water3d::Water3D,
    /// GPU skinning of characters.
    skinner: crate::skin3d::Skinner3D,
    /// GPU crowds (armies of one model each), and which mesh each one fills.
    crowds: crate::crowd3d::Crowds3D,
    crowd_meshes: Vec<(MeshHandle, usize)>,
    /// `replace_mesh` gives a GPU skin to a skinned mesh even in a slot that had none (set once
    /// a mesh was uploaded with [`Self::upload_skinned_mesh`]: such slots get recycled).
    recycled_skins: bool,
    /// The opaque pipelines again, mirrored and unsampled, for the water's reflection.
    reflect_pipelines: [[wgpu::RenderPipeline; 2]; 2],
    reflection: Reflection,
    /// Frames rendered (for the shadow cache).
    frame_index: u64,
    /// The scene's finish (ink lines, colour grade); none by default.
    finish: Finish3D,
    outline_pipeline: wgpu::RenderPipeline,
    outline_layout: wgpu::BindGroupLayout,
    outline_buffer: wgpu::Buffer,
}

impl Wgpu3DRenderer {
    /// Create a renderer targeting `format` and a depth buffer of `size`.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: (u32, u32),
    ) -> Self {
        Self::new_with_msaa(device, queue, format, size, 1)
    }

    /// Create a renderer with an explicit color/depth MSAA sample count.
    ///
    /// The public default remains single-sampled for headless callers. The
    /// desktop preview opts into 4x when the adapter advertises it, giving
    /// bevels, foliage cutouts, and wall silhouettes the same clean edge
    /// treatment users expect from Three.js antialiasing.
    pub fn new_with_msaa(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        sample_count: u32,
    ) -> Self {
        let sample_count = match sample_count {
            2 | 4 | 8 | 16 => sample_count,
            _ => 1,
        };
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
                // Every draw's data, read by the instance index (see `MESH_SHADER`).
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
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
                    // The normal map shares the albedo sampler.  Every
                    // texture pair gets a flat [0.5, 0.5, 1] fallback when no
                    // authored normal is supplied, so legacy assets remain
                    // visually unchanged.
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
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
                    // The bounced light (gi3d) and its sampler.
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("velvet-3d-linear-sampler"),
            // World surfaces deliberately use UVs outside 0..1 so a small
            // 1K albedo can tile across a large terrain without stretching.
            // Repeating is also the conventional glTF default for authored
            // material UVs; assets that stay inside 0..1 render unchanged.
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            // The authored castle is viewed at a steep isometric angle. A
            // real mip chain plus anisotropic filtering keeps floor stones
            // and wall details crisp instead of shimmering into a flat blur
            // as the camera moves away.
            anisotropy_clamp: 8,
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
                // Reuse the material bind group so alpha-masked foliage casts
                // the same cutout silhouette that the color pass displays.
                bind_group_layouts: &[&camera_layout, &object_layout, &texture_bind_group_layout],
                push_constant_ranges: &[],
            });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-shader"),
            // (`VELVET_EXP=pcf1`: one shadow tap, to measure what the sixteen cost.)
            source: wgpu::ShaderSource::Wgsl(if experiment("pcf1") {
                MESH_SHADER
                    .replace(
                        "i < 16; i = i + 1) {
        let offset = rot",
                        "i < 1; i = i + 1) {
        let offset = rot",
                    )
                    .replace("return visibility / 16.0;", "return visibility;")
                    .into()
            } else {
                MESH_SHADER.into()
            }),
        });
        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-shadow-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_SHADER.into()),
        });
        // Four variants of the one shading program: with or without alpha
        // cutout (`discard` disables early depth rejection), and with or
        // without a normal map (a flat map skips its sample and derivatives).
        // The finish's ink: a full-screen pass reading the depth (multisampled when the frame is).
        let outline_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-outline-layout"),
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
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: sample_count > 1,
                    },
                    count: None,
                },
            ],
        });
        let outline_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-outline-uniform"),
            size: std::mem::size_of::<OutlineUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let outline_source = OUTLINE_SHADER
            .replace(
                "__DEPTH__",
                if sample_count > 1 {
                    "texture_depth_multisampled_2d"
                } else {
                    "texture_depth_2d"
                },
            )
            .replace("__SAMPLES__", &sample_count.max(1).to_string());
        let outline_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-outline-shader"),
            source: wgpu::ShaderSource::Wgsl(outline_source.into()),
        });
        let outline_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("velvet-3d-outline-pipeline-layout"),
                bind_group_layouts: &[&outline_layout],
                push_constant_ranges: &[],
            });
        let outline_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("velvet-3d-outline-pipeline"),
            layout: Some(&outline_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &outline_shader,
                entry_point: Some("vs_outline"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &outline_shader,
                entry_point: Some("fs_outline"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    // Premultiplied ink over the frame; the frame's alpha stays.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let pipelines = [false, true].map(|cutout| {
            [false, true].map(|normal_map| {
                create_mesh_pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    crate::post3d::HDR_FORMAT,
                    sample_count,
                    cutout,
                    normal_map,
                    BlendMode3D::Solid,
                    false,
                )
            })
        });
        let probe_depth_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("velvet-3d-probe-depth"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MeshVertex3D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4],
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
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState { count: sample_count, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
            cache: None,
        });
        let probe_equal_pipelines = [false, true].map(|normal_map| {
            create_mesh_pipeline_with(
                device,
                &pipeline_layout,
                &shader,
                crate::post3d::HDR_FORMAT,
                sample_count,
                false,
                normal_map,
                BlendMode3D::Solid,
                false,
                Some(wgpu::CompareFunction::LessEqual),
            )
        });
        // The water's reflection: the scene mirrored in its surface, at half size, unsampled.
        let reflect_pipelines = [false, true].map(|cutout| {
            [false, true].map(|normal_map| {
                create_mesh_pipeline(
                    device,
                    &pipeline_layout,
                    &shader,
                    crate::post3d::HDR_FORMAT,
                    1,
                    cutout,
                    normal_map,
                    BlendMode3D::Solid,
                    true,
                )
            })
        });
        let translucent_pipeline = create_mesh_pipeline(
            device,
            &pipeline_layout,
            &shader,
            crate::post3d::HDR_FORMAT,
            sample_count,
            true,
            false,
            BlendMode3D::Translucent,
            false,
        );
        let additive_pipeline = create_mesh_pipeline(
            device,
            &pipeline_layout,
            &shader,
            crate::post3d::HDR_FORMAT,
            sample_count,
            true,
            false,
            BlendMode3D::Additive,
            false,
        );
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
            fragment: Some(wgpu::FragmentState {
                module: &shadow_shader,
                entry_point: Some("fs_shadow"),
                targets: &[],
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
        let shadow_solid_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("velvet-3d-shadow-solid-pipeline"),
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
        // Packed: an array the shaders index (its stride is the struct's size).
        let object_stride = object_size;
        let object_capacity = OBJECT_UNIFORM_CAPACITY;
        let object_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-object-buffer"),
            size: object_stride * object_capacity as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let object_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-object-bind-group"),
            layout: &object_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: object_buffer.as_entire_binding(),
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
            &PreparedTexture3D::new(&white_texture, None).expect("1x1 white texture"),
        );
        let post = crate::post3d::Post3D::new(device, format, size, sample_count);
        BC7_ENABLED.store(
            device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
            std::sync::atomic::Ordering::Relaxed,
        );
        let reflection = Reflection::new(device, &camera_layout);
        let water = crate::water3d::Water3D::new(
            device,
            &camera_layout,
            &reflection.layout,
            crate::post3d::HDR_FORMAT,
            sample_count,
        );
        let (depth_texture, depth_view) = create_depth_texture(device, size, sample_count);
        let shadow_texture = create_shadow_texture(device);
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let cached = [create_shadow_texture(device), create_shadow_texture(device)];
        let cached_views = [
            cached[0].create_view(&wgpu::TextureViewDescriptor::default()),
            cached[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let build_camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-shadow-build-camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let build_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-shadow-build-camera-bind-group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: build_camera.as_entire_binding(),
            }],
        });
        let shadow_restore = ShadowRestore::new(device, &cached_views);
        let shadow_cache = ShadowCache {
            maps: cached,
            views: cached_views,
            front: 0,
            current: None,
            build: None,
            previous: KeySet::default(),
            build_camera,
            build_camera_bind_group,
        };
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
        let gi = crate::gi3d::ScreenGi::new(device, queue, sample_count);
        let shadow_bind_group = frame_group(
            device,
            &shadow_bind_group_layout,
            &shadow_view,
            &shadow_sampler,
            gi.result().0,
            gi.sampler(),
        );
        Self {
            shading_style: ShadingStyle3D::Lit,
            directional_shadows: true,
            toon_style: ToonStyle3D::default(),
            pipelines,
            probe_depth_pipeline,
            probe_equal_pipelines,
            translucent_pipeline,
            additive_pipeline,
            shadow_pipeline,
            shadow_solid_pipeline,
            shadow_map_empty: false,
            last_cutout_draws: 0,
            last_culled_draws: 0,
            last_drawn_triangles: 0,
            last_shadow_full: false,
            last_draw_triangles: Vec::new(),
            frustum_culling: true,
            no_fog: false,
            camera_buffer,
            camera_bind_group,
            object_buffer,
            object_bind_group,
            object_bind_group_layout: object_layout,
            object_stride,
            object_capacity,
            object_upload: Vec::new(),
            texture_bind_group_layout,
            sampler,
            meshes: Vec::new(),
            textures: vec![white_gpu],
            white_texture: TextureHandle(0),
            sample_count,
            post,
            depth_texture,
            depth_view,
            depth_size: (size.0.max(1), size.1.max(1)),
            shadow_texture,
            shadow_view,
            shadow_bind_group,
            shadow_bind_group_layout,
            shadow_sampler,
            gi,
            shadow_cache,
            quality: Quality3D::default(),
            shadow_restore,
            water,
            skinner: crate::skin3d::Skinner3D::new(device),
            crowds: crate::crowd3d::Crowds3D::new(device),
            crowd_meshes: Vec::new(),
            recycled_skins: false,
            reflect_pipelines,
            reflection,
            frame_index: 0,
            finish: Finish3D::default(),
            outline_pipeline,
            outline_layout,
            outline_buffer,
        }
    }

    /// Registers a body of water: from now on it can be shown, touched and splashed.
    pub fn add_water_body(
        &mut self,
        device: &wgpu::Device,
        body: crate::water3d::WaterBody3D,
    ) -> crate::water3d::WaterBodyHandle {
        self.water.add_body(device, body)
    }

    /// Registers running or falling water (a channel, a cascade, a waterfall).
    pub fn add_water_stream(
        &mut self,
        device: &wgpu::Device,
        stream: crate::water3d::WaterStream3D,
    ) -> crate::water3d::WaterStreamHandle {
        self.water.add_stream(device, stream)
    }

    /// Shows or hides running water.
    pub fn set_water_stream_visible(
        &mut self,
        stream: crate::water3d::WaterStreamHandle,
        visible: bool,
    ) {
        self.water.set_stream_visible(stream, visible);
    }

    /// Shows (and simulates) a body of water, or hides (and pauses) it.
    pub fn set_water_visible(&mut self, body: crate::water3d::WaterBodyHandle, visible: bool) {
        self.water.set_visible(body, visible);
    }

    /// The body's description (its grid, mask and depth), for queries.
    pub fn water_body(
        &self,
        body: crate::water3d::WaterBodyHandle,
    ) -> Option<&crate::water3d::WaterBody3D> {
        self.water.body(body)
    }

    /// Disturbs the water round (x, z): pushes the surface down by `strength` metres over
    /// `radius` (a drop, a step, a paddle). It ripples out and reflects off the banks.
    pub fn water_touch(&mut self, x: f32, z: f32, radius: f32, strength: f32) {
        self.water.touch(x, z, radius, strength);
    }

    /// Throws a droplet (SPH) from `position` at `velocity`; it lives `life` seconds unless it
    /// falls back into a body first (where it splashes).
    pub fn water_emit(&mut self, position: Vec3, velocity: Vec3, life: f32) {
        self.water.emit(position, velocity, life);
    }

    /// Lets the water run `dt` seconds by the next frame.
    pub fn advance_water(&mut self, dt: f32) {
        self.water.advance(dt);
    }

    /// Select the scene-wide lighting style without recompiling pipelines.
    /// Alpha masks, glass blending, fog and shadow maps keep their semantics.
    pub fn set_shading_style(&mut self, style: ShadingStyle3D) {
        self.shading_style = style;
    }

    /// Enable or disable the directional shadow-map pass and return the old
    /// value. Interiors lit only by local lights (caves) skip a whole second
    /// geometry pass; the cleared map then reports every fragment as lit.
    pub fn replace_directional_shadows(&mut self, enabled: bool) -> bool {
        std::mem::replace(&mut self.directional_shadows, enabled)
    }

    /// Temporarily override the art profile for one scene and return the old
    /// profile, so callers can restore it when sharing a renderer with a game.
    pub fn replace_toon_style(&mut self, style: ToonStyle3D) -> ToonStyle3D {
        std::mem::replace(&mut self.toon_style, style)
    }

    /// The bounced light last gathered, read back (width, height, rgba: the light and the depth
    /// along the view), to look at it; none while it is off.
    pub fn read_bounced_light(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(u32, u32, Vec<[f32; 4]>)> {
        self.gi.read(device, queue)
    }

    /// The graphics settings from now on.
    pub fn set_quality(&mut self, quality: Quality3D) {
        self.quality = quality;
    }

    /// The graphics settings in use.
    pub fn quality(&self) -> Quality3D {
        self.quality
    }

    /// Set the finish (ink lines and colour grade) of the frames that follow; the default has
    /// none. Returns the previous one, so a scene can set its own and give it back.
    pub fn replace_finish(&mut self, finish: Finish3D) -> Finish3D {
        std::mem::replace(&mut self.finish, finish)
    }

    /// Resize color/depth storage to match the target surface.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.depth_size == size {
            return;
        }
        self.post.resize(device, size);
        let (texture, view) = create_depth_texture(device, size, self.sample_count);
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
        self.object_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-object-buffer-grown"),
            size: self.object_stride * capacity as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.object_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-object-bind-group-grown"),
            layout: &self.object_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.object_buffer.as_entire_binding(),
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
        let lods = build_lods(device, mesh);
        self.meshes.push(GpuMesh3D {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len() as u32,
            bounds: std::cell::Cell::new(vertex_bounds(&mesh.vertices)),
            updated: std::cell::Cell::new(0),
            skin: None,
            lods,
        });
        Ok(handle)
    }

    /// Upload a skinned mesh that the GPU deforms: pose it each frame with
    /// [`Self::set_skin_joints`] (joint matrices from [`GltfModel::joint_matrices`]). Each
    /// character needs its own upload (its own deformed vertices). Falls back to a plain upload
    /// when the mesh carries no skin.
    pub fn upload_skinned_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh: &Mesh3D,
    ) -> Result<MeshHandle, Render3DError> {
        self.recycled_skins = true;
        if mesh.skinning.is_none() {
            return self.upload_mesh(device, mesh);
        }
        if mesh.vertices.is_empty() || mesh.indices.is_empty() {
            return Err(Render3DError::EmptyMesh);
        }
        if let Some(&index) = mesh
            .indices
            .iter()
            .find(|&&i| i as usize >= mesh.vertices.len())
        {
            return Err(Render3DError::InvalidIndex {
                index,
                vertices: mesh.vertices.len(),
            });
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-skinned-vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let handle = MeshHandle(self.meshes.len() as u32);
        let skin = self.skinner.add(device, handle, mesh, &vertex_buffer);
        self.meshes.push(GpuMesh3D {
            vertex_buffer,
            index_buffer,
            vertex_count: mesh.vertices.len(),
            index_count: mesh.indices.len() as u32,
            bounds: std::cell::Cell::new(vertex_bounds(&mesh.vertices)),
            updated: std::cell::Cell::new(0),
            skin,
            lods: Vec::new(),
        });
        Ok(handle)
    }

    /// Uploads a crowd: up to `capacity` soldiers of one model, drawn as one mesh (see
    /// [`crate::crowd3d`]). Draw it with an identity transform; place its soldiers each frame with
    /// [`Self::set_crowd`].
    pub fn upload_crowd(
        &mut self,
        device: &wgpu::Device,
        model: &crate::crowd3d::CrowdMesh3D,
        capacity: usize,
    ) -> Result<MeshHandle, Render3DError> {
        if model.vertices.is_empty() || model.indices.is_empty() {
            return Err(Render3DError::EmptyMesh);
        }
        let capacity = capacity.max(1);
        let per = model.vertices.len();
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-crowd-vertices"),
            size: (per * capacity * std::mem::size_of::<MeshVertex3D>()) as u64,
            usage: wgpu::BufferUsages::VERTEX
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let mut indices = Vec::with_capacity(model.indices.len() * capacity);
        for s in 0..capacity {
            let base = (s * per) as u32;
            indices.extend(model.indices.iter().map(|i| i + base));
        }
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-crowd-indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let crowd = self
            .crowds
            .add(device, model, capacity as u32, &vertex_buffer);
        let handle = MeshHandle(self.meshes.len() as u32);
        self.meshes.push(GpuMesh3D {
            vertex_buffer,
            index_buffer,
            vertex_count: per * capacity,
            index_count: 0,
            bounds: std::cell::Cell::new([[0.0; 3], [0.0; 3]]),
            updated: std::cell::Cell::new(0),
            skin: None,
            lods: Vec::new(),
        });
        self.crowd_meshes.push((handle, crowd));
        Ok(handle)
    }

    /// This frame's soldiers of a crowd from [`Self::upload_crowd`] (at most its capacity; `time`
    /// drives the idle sway and the capes). Returns how many it holds.
    pub fn set_crowd(
        &mut self,
        queue: &wgpu::Queue,
        handle: MeshHandle,
        instances: &[crate::crowd3d::CrowdInstance3D],
        time: f32,
    ) -> usize {
        let Some(&(_, crowd)) = self.crowd_meshes.iter().find(|(h, _)| *h == handle) else {
            return 0;
        };
        let n = self.crowds.set(queue, crowd, instances, time);
        let per_indices = self.crowds.crowds[crowd].index_count;
        let frame = self.frame_index;
        if let Some(mesh) = self.meshes.get_mut(handle.0 as usize) {
            mesh.index_count = n * per_indices;
            let mut lo = [f32::INFINITY; 3];
            let mut hi = [f32::NEG_INFINITY; 3];
            for inst in &instances[..n as usize] {
                let reach = 3.0 * inst.scale.max(0.1);
                for a in 0..3 {
                    lo[a] = lo[a].min(inst.position[a] - reach);
                    hi[a] = hi[a].max(inst.position[a] + reach);
                }
            }
            mesh.bounds.set(if n > 0 {
                [lo, hi]
            } else {
                [[0.0; 3], [0.0; 3]]
            });
            mesh.updated.set(frame);
        }
        n as usize
    }

    /// Poses a mesh from [`Self::upload_skinned_mesh`]: the GPU deforms it before the next frame
    /// is drawn.
    pub fn set_skin_joints(
        &mut self,
        queue: &wgpu::Queue,
        handle: MeshHandle,
        joints: &[Mat4],
    ) -> Result<(), Render3DError> {
        let mesh = self
            .meshes
            .get(handle.0 as usize)
            .ok_or_else(|| Render3DError::Gpu(format!("unknown mesh handle {}", handle.0)))?;
        let Some(skin) = mesh.skin else {
            return Err(Render3DError::Gpu(format!(
                "mesh {} has no GPU skin",
                handle.0
            )));
        };
        if let Some(bounds) = self.skinner.set_joints(queue, skin, joints) {
            mesh.bounds.set(bounds);
        }
        mesh.updated.set(self.frame_index);
        Ok(())
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
        mesh.bounds.set(vertex_bounds(vertices));
        mesh.updated.set(self.frame_index);
        Ok(())
    }

    /// Replace an uploaded mesh in place (same handle, new buffers).
    ///
    /// For CPU-driven surfaces whose topology changes (marching-squares
    /// waterlines, LOD swaps): recreates both GPU buffers and updates the
    /// counts, so callers never leak one upload per frame.
    pub fn replace_mesh(
        &mut self,
        device: &wgpu::Device,
        handle: MeshHandle,
        mesh: &Mesh3D,
    ) -> Result<(), Render3DError> {
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
        let was_skinned = self.meshes.get(handle.0 as usize).and_then(|m| m.skin);
        // A GPU-skinned slot, or one taking a skinned mesh where a skinned one could go (a pool of
        // recycled character meshes), keeps or gains its GPU skin.
        let skinned = mesh.skinning.is_some() && (was_skinned.is_some() || self.recycled_skins);
        let mut usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST;
        if skinned {
            usage |= wgpu::BufferUsages::STORAGE;
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage,
        });
        // A GPU-skinned mesh stays skinned (its skin rebuilt in place for the new mesh).
        let skin = match (skinned, was_skinned) {
            (true, Some(index)) => {
                self.skinner
                    .replace(device, index, handle, mesh, &vertex_buffer)
            }
            (true, None) => self.skinner.add(device, handle, mesh, &vertex_buffer),
            (false, Some(index)) => {
                // No skin needed any more: free it.
                self.skinner.remove(index);
                None
            }
            (false, None) => None,
        };
        let slot = self
            .meshes
            .get_mut(handle.0 as usize)
            .ok_or_else(|| Render3DError::Gpu(format!("unknown mesh handle {}", handle.0)))?;
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("velvet-3d-indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        slot.vertex_buffer = vertex_buffer;
        slot.index_buffer = index_buffer;
        slot.vertex_count = mesh.vertices.len();
        slot.index_count = mesh.indices.len() as u32;
        slot.lods = if skin.is_none() {
            build_lods(device, mesh)
        } else {
            Vec::new()
        };
        slot.bounds.set(vertex_bounds(&mesh.vertices));
        slot.updated.set(self.frame_index);
        slot.skin = skin;
        Ok(())
    }

    /// Upload an RGBA8 albedo texture and return its stable handle.
    pub fn upload_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &TextureData3D,
    ) -> Result<TextureHandle, Render3DError> {
        self.upload_texture_with_normal(device, queue, texture, None)
    }

    /// Upload an albedo texture and a paired tangent-space normal map.
    ///
    /// A missing normal map uses a flat normal automatically, preserving the
    /// old forward-renderer appearance for all existing assets.  Normal maps
    /// are uploaded as linear UNORM data; unlike albedo, they must not receive
    /// sRGB decoding.
    pub fn upload_texture_with_normal(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &TextureData3D,
        normal: Option<&TextureData3D>,
    ) -> Result<TextureHandle, Render3DError> {
        let prepared = PreparedTexture3D::new(texture, normal)?;
        Ok(self.upload_prepared_texture(device, queue, &prepared))
    }

    /// Upload a texture whose mip chains were already built (possibly on a
    /// loader thread): only the GPU copy happens here, so streaming art in
    /// mid-game costs no CPU image work on the frame.
    pub fn upload_prepared_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        prepared: &PreparedTexture3D,
    ) -> TextureHandle {
        let gpu = create_gpu_texture(
            device,
            queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            prepared,
        );
        let handle = TextureHandle(self.textures.len() as u32);
        self.textures.push(gpu);
        handle
    }

    /// Puts `texture` in place of the albedo uploaded as `handle` (same handle, flat normal), so
    /// every draw that uses it shows the new picture from the next frame.
    pub fn replace_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        handle: TextureHandle,
        texture: &TextureData3D,
    ) -> Result<(), Render3DError> {
        let prepared = PreparedTexture3D::new(texture, None)?;
        let gpu = create_gpu_texture(
            device,
            queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            &prepared,
        );
        if let Some(slot) = self.textures.get_mut(handle.0 as usize) {
            *slot = gpu;
        }
        Ok(())
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
        self.render_with_atmosphere(
            device,
            queue,
            target,
            size,
            camera,
            lighting,
            Atmosphere3D::default(),
            draws,
            clear,
        );
    }

    /// Render one 3D pass from a resolved environment frame.
    ///
    /// This is the one-call path for games that use [`DayNightCycle3D`]: the
    /// renderer applies its lighting, fog, and clear colour together so each
    /// scene does not need to duplicate conversion glue.
    pub fn render_with_environment(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
        environment: EnvironmentFrame3D,
        draws: &[MeshDraw3D],
    ) {
        self.render_with_environment_and_lights(
            device,
            queue,
            target,
            size,
            camera,
            environment,
            &[],
            draws,
        );
    }

    /// Render a resolved environment plus a compact set of local lights.
    ///
    /// Up to sixteen lights are uploaded in one camera uniform. Extra lights
    /// are ignored deliberately, keeping frame cost deterministic for dense
    /// authored worlds.
    pub fn render_with_environment_and_lights(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
        environment: EnvironmentFrame3D,
        local_lights: &[PointLight3D],
        draws: &[MeshDraw3D],
    ) {
        self.render_with_atmosphere_and_celestial(
            device,
            queue,
            target,
            size,
            camera,
            environment.lighting,
            environment.atmosphere,
            environment.celestial,
            local_lights,
            draws,
            wgpu::Color {
                r: f64::from(environment.clear_color[0]),
                g: f64::from(environment.clear_color[1]),
                b: f64::from(environment.clear_color[2]),
                a: f64::from(environment.clear_color[3]),
            },
        );
    }

    /// Render one 3D pass with distance and height-based atmospheric haze.
    pub fn render_with_atmosphere(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
        lighting: Lighting3D,
        atmosphere: Atmosphere3D,
        draws: &[MeshDraw3D],
        clear: wgpu::Color,
    ) {
        self.render_with_atmosphere_and_celestial(
            device,
            queue,
            target,
            size,
            camera,
            lighting,
            atmosphere,
            Celestial3D::default(),
            &[],
            draws,
            clear,
        );
    }

    // Full frame context in one call (device/queue/target/size/camera/
    // lighting/atmosphere/celestial/lights/draws/clear); grouping would
    // churn every call site for no runtime gain.
    #[allow(clippy::too_many_arguments)]
    fn render_with_atmosphere_and_celestial(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
        lighting: Lighting3D,
        atmosphere: Atmosphere3D,
        celestial: Celestial3D,
        local_lights: &[PointLight3D],
        draws: &[MeshDraw3D],
        clear: wgpu::Color,
    ) {
        let cpu_started = std::time::Instant::now();
        let mut marks: Vec<(&str, std::time::Instant)> = Vec::new();
        let profiling = experiment("cpuprof");
        let mut mark = |name: &'static str| {
            if profiling {
                marks.push((name, std::time::Instant::now()));
            }
        };
        self.resize(device, size);
        self.ensure_object_capacity(device, draws.len());
        let saved_shadows = self.directional_shadows;
        if experiment("noshadow") || self.quality.shadow_samples == 0 {
            self.directional_shadows = false;
        }
        let local_lights = if experiment("nolights") {
            &[][..]
        } else {
            local_lights
        };
        let draws_all = draws;
        let kept: Vec<MeshDraw3D>;
        let draws = if experiment("noblend") {
            kept = draws_all
                .iter()
                .copied()
                .filter(|d| !(d.material.is_additive() || d.material.base_color[3] < 0.99))
                .collect();
            &kept[..]
        } else {
            draws_all
        };
        let aspect = size.0.max(1) as f32 / size.1.max(1) as f32;
        let mut light_direction = lighting.direction_to_light.normalize_or_zero();
        if light_direction.length_squared() <= 1e-6 {
            light_direction = Lighting3D::default().direction_to_light.normalize_or_zero();
        }
        self.frame_index += 1;
        let frame_index = self.frame_index;
        // The shadow casters of this frame: (draw index, key, still).
        let casters: Vec<(usize, u64, bool)> = if self.directional_shadows {
            draws
                .iter()
                .enumerate()
                .filter_map(|(index, draw)| {
                    // Translucent geometry (markers, particles) and authored floor receivers
                    // cast nothing (see the pass below).
                    let shadow_only = draw.material.is_shadow_only();
                    if draw.material.base_color[3] < 0.95
                        || (draw.material.occlusion_window[0] < -0.5 && !shadow_only)
                    {
                        return None;
                    }
                    let mesh = self.meshes.get(draw.mesh.0 as usize)?;
                    // Too low to throw a shadow worth drawing (grass, pebbles, the ground itself).
                    let bounds = mesh.bounds.get();
                    let up = draw.transform.transform_vector3(Vec3::Y).length();
                    if (bounds[1][1] - bounds[0][1]) * up < SHADOW_MIN_HEIGHT && !shadow_only {
                        return None;
                    }
                    let key = caster_key(draw);
                    let window = draw.material.occlusion_window;
                    let still = self.shadow_cache.previous.contains(&key)
                        && !(mesh.updated.get() > 0
                            && mesh.updated.get() + SHADOW_ANIMATED_FRAMES >= frame_index)
                        && !(window[2] > 0.0 && window[3] > 0.0);
                    Some((index, key, still))
                })
                .collect()
        } else {
            Vec::new()
        };
        mark("casters");
        // Finish a build: its map takes over.
        if self
            .shadow_cache
            .build
            .as_ref()
            .is_some_and(|b| b.step >= SHADOW_BUILD_STEPS)
        {
            let build = self.shadow_cache.build.take().expect("a finished build");
            self.shadow_cache.front = 1 - self.shadow_cache.front;
            self.shadow_cache.current = Some(build.frame);
            self.shadow_restore.version += 1;
        }
        let cell = (
            (camera.target.x / SHADOW_CELL).round() as i32,
            (camera.target.z / SHADOW_CELL).round() as i32,
        );
        let extent = ((lighting.shadow_extent.max(1.0) + 2.0 * SHADOW_CELL) / 4.0).ceil() * 4.0;
        let wanted = |keys| ShadowFrame {
            view_projection: shadow_projection(light_direction, cell, extent),
            direction: light_direction,
            center: cell,
            extent,
            keys,
        };
        let still_keys = || {
            casters
                .iter()
                .filter(|c| c.2)
                .map(|c| c.1)
                .collect::<KeySet>()
        };
        let turn = |a: Vec3, b: Vec3| a.dot(b).clamp(-1.0, 1.0).acos();
        // Redraw at once: nothing cached yet, the sun jumped, the view jumped, or the extent
        // changed.
        let full = self.directional_shadows
            && match &self.shadow_cache.current {
                None => true,
                Some(f) => {
                    let jumped = (f.center.0 - cell.0).abs().max((f.center.1 - cell.1).abs()) > 1;
                    f.extent != extent
                        || turn(f.direction, light_direction) > SHADOW_TURN_JUMP
                        || jumped
                }
            };
        self.last_shadow_full = full;
        if full {
            self.shadow_cache.build = None;
            self.shadow_cache.current = Some(wanted(still_keys()));
            self.shadow_restore.version += 1;
        } else if self.directional_shadows && self.shadow_cache.build.is_none() {
            // Follow gradually: the sun turned a little, the view entered the next cell,
            // something new keeps still now, or something cached has gone (moved, hidden, or
            // revealed by the see-through window); its old shadow lingers for a few frames.
            if let Some(f) = &self.shadow_cache.current {
                let newly_still = casters.iter().any(|c| c.2 && !f.keys.contains(&c.1));
                let gone = {
                    let seen: KeySet = casters
                        .iter()
                        .map(|c| c.1)
                        .filter(|k| f.keys.contains(k))
                        .collect();
                    seen.len() < f.keys.len()
                };
                if turn(f.direction, light_direction) > SHADOW_TURN_STEP
                    || f.center != cell
                    || newly_still
                    || gone
                {
                    self.shadow_cache.build = Some(ShadowBuild {
                        frame: wanted(still_keys()),
                        step: 0,
                    });
                }
            }
        }
        let (light_view_projection, shadow_extent) = match &self.shadow_cache.current {
            Some(f) => (f.view_projection, f.extent),
            None => (shadow_projection(light_direction, cell, extent), extent),
        };
        let mut local_light_positions = [[0.0; 4]; MAX_POINT_LIGHTS_3D];
        let mut local_light_colors = [[0.0; 4]; MAX_POINT_LIGHTS_3D];
        let mut local_light_count = 0usize;
        for light in local_lights {
            if local_light_count >= MAX_POINT_LIGHTS_3D {
                break;
            }
            if ![light.position.x, light.position.y, light.position.z]
                .iter()
                .all(|component| component.is_finite())
                || !light.color.iter().all(|channel| channel.is_finite())
                || !light.intensity.is_finite()
                || !light.radius.is_finite()
                || light.intensity <= 0.0
                || light.radius <= 0.05
            {
                continue;
            }
            local_light_positions[local_light_count] = [
                light.position.x,
                light.position.y,
                light.position.z,
                light.radius.clamp(0.05, 128.0),
            ];
            local_light_colors[local_light_count] = [
                light.color[0].max(0.0),
                light.color[1].max(0.0),
                light.color[2].max(0.0),
                light.intensity.clamp(0.0, 128.0),
            ];
            local_light_count += 1;
        }
        // The light bounced off the frame before: gathered (and submitted) before this frame's
        // passes, which overwrite the depth and colour it reads.
        mark("shadow-plan+lights");
        let gi_on = self.finish.gi > 0.0 && self.quality.bounced_light && !experiment("nogi");
        if gi_on {
            let mut gi_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("velvet-3d-gi"),
            });
            self.gi.gather(
                device,
                queue,
                &mut gi_encoder,
                &self.depth_view,
                self.post.scene_color(),
                self.finish.gi_radius,
                self.depth_size,
            );
            queue.submit(std::iter::once(gi_encoder.finish()));
        } else {
            self.gi.reset();
        }
        let (gi_view_projection, gi_camera, gi_forward) = match self.gi.result().1 {
            Some(c) if gi_on => (
                c.view_projection.to_cols_array(),
                [
                    c.position.x,
                    c.position.y,
                    c.position.z,
                    self.finish.gi.max(0.0),
                ],
                [
                    c.forward.x,
                    c.forward.y,
                    c.forward.z,
                    self.finish.gi_ambient.clamp(0.0, 1.0),
                ],
            ),
            _ => (Mat4::IDENTITY.to_cols_array(), [0.0; 4], [0.0; 4]),
        };
        self.shadow_bind_group = frame_group(
            device,
            &self.shadow_bind_group_layout,
            &self.shadow_view,
            &self.shadow_sampler,
            self.gi.result().0,
            self.gi.sampler(),
        );
        mark("gi");
        let toon = self.toon_style;
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
            moon_direction: [
                celestial.moon_direction.x,
                celestial.moon_direction.y,
                celestial.moon_direction.z,
                celestial.moon_visibility,
            ],
            moon_color: [
                celestial.moon_color[0],
                celestial.moon_color[1],
                celestial.moon_color[2],
                celestial.moon_intensity,
            ],
            reflection_color: [
                celestial.reflection_color[0],
                celestial.reflection_color[1],
                celestial.reflection_color[2],
                celestial.reflection_strength,
            ],
            horizon_tint: [
                celestial.horizon_tint[0],
                celestial.horizon_tint[1],
                celestial.horizon_tint[2],
                celestial.horizon_strength,
            ],
            fog_color: [
                atmosphere.fog_color[0],
                atmosphere.fog_color[1],
                atmosphere.fog_color[2],
                1.0,
            ],
            fog_params: if self.no_fog {
                [1.0e6, 2.0e6, 0.0, 0.0]
            } else {
                [
                    atmosphere.fog_start.max(0.0),
                    atmosphere
                        .fog_end
                        .max(atmosphere.fog_start.max(0.0) + 0.001),
                    atmosphere.fog_density.max(0.0),
                    atmosphere.height_falloff.max(0.0),
                ]
            },
            focus_position: [camera.target.x, camera.target.y, camera.target.z, aspect],
            // z: 1 for an orthographic camera (one view direction for the whole frame).
            viewport_size: [
                size.0.max(1) as f32,
                size.1.max(1) as f32,
                if camera.orthographic_height.is_some() {
                    1.0
                } else {
                    0.0
                },
                0.0,
            ],
            local_light_meta: [
                local_light_count as f32,
                if self.directional_shadows { 0.0 } else { 1.0 },
                // The shadow map's depth range and one texel's width, in metres.
                SHADOW_DEPTH - 1.0,
                2.0 * shadow_extent / SHADOW_MAP_SIZE as f32,
            ],
            local_light_positions,
            local_light_colors,
            toon_params: [
                if self.shading_style == ShadingStyle3D::Toon {
                    1.0
                } else {
                    0.0
                },
                toon.softness,
                toon.specular,
                toon.normal_detail,
            ],
            toon_shadow: [
                toon.shadow_tint[0],
                toon.shadow_tint[1],
                toon.shadow_tint[2],
                toon.rim,
            ],
            grade: [self.finish.saturation, self.finish.contrast, 0.0, 0.0],
            clip_plane: [0.0, 0.0, 0.0, 1.0],
            gi_view_projection,
            gi_camera,
            gi_forward,
            quality: [
                self.quality.shadow_samples.clamp(1, 16) as f32,
                0.0,
                0.0,
                0.0,
            ],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        // Preserve dynamic-uniform alignment while issuing one staging copy.
        // Dense worlds otherwise allocate a separate upload for every draw,
        // on top of the animated character vertex uploads each frame.
        self.object_upload
            .resize(self.object_stride as usize * draws.len(), 0);
        // Each draw's uniform (placement, normal matrix, the lights that reach it), in chunks on
        // the worker pool: thousands of draws in a forest.
        let bounds: Vec<Option<[[f32; 3]; 2]>> = draws
            .iter()
            .map(|d| self.meshes.get(d.mesh.0 as usize).map(|m| m.bounds.get()))
            .collect();
        let every = if local_light_count == 0 {
            0
        } else {
            u32::MAX >> (32 - local_light_count.min(32))
        };
        let all_lights = experiment("allights");
        let stride = self.object_stride as usize;
        const CHUNK: usize = 512;
        // And whether each is on screen at all (its box outside the camera's view: skipped).
        let screen_vp = camera.view_projection(aspect);
        let culling = self.frustum_culling;
        let mut on_screen = vec![true; draws.len()];
        // And which of its levels of detail: by how tall it shows (pixels), when the quality
        // allows simpler ones at all.
        let mut levels = vec![0u8; draws.len()];
        let lod_px: Option<(f32, f32)> = match self.quality.lod {
            0 => None,
            1 => Some((40.0, 16.0)),
            _ => Some((120.0, 48.0)),
        };
        #[allow(clippy::type_complexity)]
        let parts: Vec<(usize, &mut [u8], &mut [bool], &mut [u8])> = self
            .object_upload
            .chunks_mut(stride * CHUNK)
            .zip(on_screen.chunks_mut(CHUNK))
            .zip(levels.chunks_mut(CHUNK))
            .enumerate()
            .map(|(chunk, ((out, seen), lod))| (chunk, out, seen, lod))
            .collect();
        let fill = |(chunk, out, seen, lod): (usize, &mut [u8], &mut [bool], &mut [u8])| {
            for (k, slot) in out.chunks_mut(stride).enumerate() {
                let draw_index = chunk * CHUNK + k;
                let draw = &draws[draw_index];
                if culling {
                    if let Some(b) = bounds[draw_index] {
                        seen[k] = !outside_view(&(screen_vp * draw.transform).to_cols_array(), b);
                    }
                }
                if let (Some((one, two)), Some(b)) = (lod_px, bounds[draw_index]) {
                    if let Some(r) = screen_rect(screen_vp * draw.transform, b, size) {
                        let tall = (r[3] - r[1]).max(r[2] - r[0]);
                        lod[k] = if tall < two {
                            2
                        } else if tall < one {
                            1
                        } else {
                            0
                        };
                    }
                }
                let mask = if all_lights {
                    every
                } else {
                    bounds[draw_index].map_or(0, |b| {
                        light_mask(draw.transform, b, &local_light_positions, local_light_count)
                    })
                };
                let object = ObjectUniform {
                    model: draw.transform.to_cols_array(),
                    base_color: draw.material.base_color,
                    occlusion_window: draw.material.occlusion_window,
                    normal_matrix: normal_matrix(draw.transform),
                    light_mask: [mask, 0, 0, 0],
                };
                let bytes = bytemuck::bytes_of(&object);
                slot[..bytes.len()].copy_from_slice(bytes);
            }
        };
        crate::work::each(parts, &fill);
        if !self.object_upload.is_empty() {
            queue.write_buffer(&self.object_buffer, 0, &self.object_upload);
        }
        mark("objects");
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("velvet-3d-frame"),
        });
        // Skinned characters, deformed on the GPU.
        self.skinner.dispatch(&mut encoder);
        // Crowds, posed on the GPU.
        self.crowds.dispatch(&mut encoder);
        // The water's solvers.
        let water_on = !experiment("nowater");
        if water_on {
            self.water.step(queue, &mut encoder);
        }
        // Without directional shadows the map only needs clearing once.
        if self.directional_shadows || !self.shadow_map_empty {
            self.shadow_map_empty = !self.directional_shadows;
            // Draws `list` into a shadow map with the given camera bind group.
            let draw_casters = |pass: &mut wgpu::RenderPass<'_>,
                                list: &mut dyn Iterator<Item = usize>,
                                view_projection: Mat4| {
                let mut bound_solid = None;
                let mut bound_texture: Option<u32> = None;
                let mut bound_mesh: Option<u32> = None;
                let mut run: Option<DrawRun> = None;
                pass.set_bind_group(1, &self.object_bind_group, &[]);
                for draw_index in list {
                    let draw = &draws[draw_index];
                    let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                        continue;
                    };
                    // Outside the light's volume it can cast nothing into the map.
                    if self.frustum_culling
                        && outside_view(
                            &(view_projection * draw.transform).to_cols_array(),
                            mesh.bounds.get(),
                        )
                    {
                        continue;
                    }
                    let texture = draw
                        .texture
                        .or(draw.material.texture)
                        .unwrap_or(self.white_texture);
                    let texture = self.textures.get(texture.0 as usize);
                    let window = draw.material.occlusion_window;
                    // Revealed walls still cast: the cut-out shader removes only their circle.
                    let solid =
                        texture.is_some_and(|t| !t.cutout) && !(window[2] > 0.0 && window[3] > 0.0);
                    let texture_id = draw
                        .texture
                        .or(draw.material.texture)
                        .unwrap_or(self.white_texture)
                        .0;
                    let level = levels.get(draw_index).copied().unwrap_or(0);
                    let (indices, index_count) = mesh.level(level);
                    let geometry = draw.mesh.0 * 4 + u32::from(level);
                    let key = (geometry, texture_id, usize::from(solid));
                    if DrawRun::extends(&run, draw_index, key) {
                        if let Some(r) = run.as_mut() {
                            r.end += 1;
                        }
                        continue;
                    }
                    DrawRun::flush(&mut run, pass);
                    if bound_solid != Some(solid) {
                        pass.set_pipeline(if solid {
                            &self.shadow_solid_pipeline
                        } else {
                            &self.shadow_pipeline
                        });
                        bound_solid = Some(solid);
                    }
                    if let Some(texture) = texture {
                        if bound_texture != Some(texture_id) {
                            pass.set_bind_group(2, &texture.bind_group, &[]);
                            bound_texture = Some(texture_id);
                        }
                    }
                    if bound_mesh != Some(geometry) {
                        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                        bound_mesh = Some(geometry);
                    }
                    run = Some(DrawRun {
                        first: draw_index as u32,
                        end: draw_index as u32 + 1,
                        count: index_count,
                        key,
                    });
                }
                DrawRun::flush(&mut run, pass);
            };
            let depth_pass = |encoder: &mut wgpu::CommandEncoder,
                              view: &wgpu::TextureView,
                              clear: bool| {
                encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("velvet-3d-shadow-pass"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view,
                            depth_ops: Some(wgpu::Operations {
                                load: if clear {
                                    wgpu::LoadOp::Clear(1.0)
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    })
                    .forget_lifetime()
            };
            let cache = &self.shadow_cache;
            if self.directional_shadows {
                let front = cache.front;
                // A full redraw of what keeps still, into the map in use.
                if full {
                    let mut pass = depth_pass(&mut encoder, &cache.views[front], true);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    let keys = &cache.current.as_ref().expect("a cached frame").keys;
                    draw_casters(
                        &mut pass,
                        &mut casters.iter().filter(|c| keys.contains(&c.1)).map(|c| c.0),
                        light_view_projection,
                    );
                }
                // One step of a gradual build into the other map.
                if let Some(build) = &cache.build {
                    let mut uniform = camera_uniform;
                    uniform.light_view_projection = build.frame.view_projection.to_cols_array();
                    queue.write_buffer(&cache.build_camera, 0, bytemuck::bytes_of(&uniform));
                    let mut pass =
                        depth_pass(&mut encoder, &cache.views[1 - front], build.step == 0);
                    pass.set_bind_group(0, &cache.build_camera_bind_group, &[]);
                    let step = build.step as u64;
                    let keys = &build.frame.keys;
                    draw_casters(
                        &mut pass,
                        &mut casters
                            .iter()
                            .filter(|c| {
                                keys.contains(&c.1) && c.1 % SHADOW_BUILD_STEPS as u64 == step
                            })
                            .map(|c| c.0),
                        build.frame.view_projection,
                    );
                }
                // This frame's map: the cached one, and whatever moves on top. It already holds the
                // cached map but where last frame's moving casters drew, unless the cached map
                // changed since: then it is copied whole; else only those rectangles are put back.
                let source = (front, self.shadow_restore.version);
                if experiment("fullcopy") || self.shadow_restore.synced != Some(source) {
                    encoder.copy_texture_to_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &cache.maps[front],
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyTextureInfo {
                            texture: &self.shadow_texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::Extent3d {
                            width: SHADOW_MAP_SIZE,
                            height: SHADOW_MAP_SIZE,
                            depth_or_array_layers: 1,
                        },
                    );
                } else if !self.shadow_restore.dirty.is_empty() {
                    let mut pass = depth_pass(&mut encoder, &self.shadow_view, false);
                    pass.set_pipeline(&self.shadow_restore.pipeline);
                    pass.set_bind_group(0, &self.shadow_restore.groups[front], &[]);
                    for r in &self.shadow_restore.dirty {
                        pass.set_scissor_rect(r[0], r[1], r[2], r[3]);
                        pass.draw(0..3, 0..1);
                    }
                }
                // Where this frame's moving casters land on the map (texels), to put back next.
                let mut dirty = Vec::new();
                for c in casters
                    .iter()
                    .filter(|c| !keys_of_current(cache).contains(&c.1))
                {
                    let draw = &draws[c.0];
                    if let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) {
                        if let Some(r) = shadow_footprint(
                            light_view_projection * draw.transform,
                            mesh.bounds.get(),
                        ) {
                            dirty.push(r);
                        }
                    }
                }
                let whole = experiment("fullcopy") || self.shadow_restore.synced != Some(source);
                self.shadow_restore.synced = Some(source);
                self.shadow_restore.dirty = merge_rects(dirty);
                if experiment("shadowstats") {
                    let area: u64 = self
                        .shadow_restore
                        .dirty
                        .iter()
                        .map(|r| r[2] as u64 * r[3] as u64)
                        .sum();
                    eprintln!(
                        "[shadow] frame {frame_index}: whole copy {whole}, movers {}, rects {}, area {:.1}% , full {full}, build {:?}",
                        casters.iter().filter(|c| !keys_of_current(cache).contains(&c.1)).count(),
                        self.shadow_restore.dirty.len(),
                        area as f64 * 100.0 / (SHADOW_MAP_SIZE as f64 * SHADOW_MAP_SIZE as f64),
                        cache.build.as_ref().map(|b| b.step),
                    );
                }
                let mut pass = depth_pass(&mut encoder, &self.shadow_view, false);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                let keys = &cache.current.as_ref().expect("a cached frame").keys;
                if !experiment("nomovers") {
                    draw_casters(
                        &mut pass,
                        &mut casters.iter().filter(|c| !keys.contains(&c.1)).map(|c| c.0),
                        light_view_projection,
                    );
                }
            } else {
                // Cleared once: every fragment lit.
                let _pass = depth_pass(&mut encoder, &self.shadow_view, true);
                self.shadow_restore.synced = None;
            }
        }
        if let Some(build) = self.shadow_cache.build.as_mut() {
            build.step += 1;
        }
        self.shadow_cache.previous = casters.iter().map(|c| c.1).collect();
        mark("shadow-pass");
        // The water's reflection: what stands above the nearest water, seen in its surface.
        // Only when some water is on screen to show it (nothing else reads it).
        let screen = camera.view_projection(aspect).to_cols_array();
        let mirror = if water_on
            && self.quality.water_reflections
            && !experiment("norefl")
            && (experiment("allrefl")
                || self.water.any_body_on_screen(|b| outside_view(&screen, b)))
        {
            self.water.reflection_level(camera.target)
        } else {
            None
        };
        if let Some(level) = mirror {
            let half = ((size.0 / 2).max(1), (size.1 / 2).max(1));
            self.reflection.ensure(device, half);
            let mut uniform = camera_uniform;
            // y → 2h − y: the camera mirrored in the plane.
            let flip = Mat4::from_translation(Vec3::new(0.0, level, 0.0))
                * Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
                * Mat4::from_translation(Vec3::new(0.0, -level, 0.0));
            let mirrored_vp = camera.view_projection(aspect) * flip;
            uniform.view_projection = mirrored_vp.to_cols_array();
            uniform.camera_position[1] = 2.0 * level - uniform.camera_position[1];
            uniform.focus_position[1] = 2.0 * level - uniform.focus_position[1];
            uniform.viewport_size[0] = half.0 as f32;
            uniform.viewport_size[1] = half.1 as f32;
            // Only what is above the water (a hair under it, so the banks meet their image).
            uniform.clip_plane = [0.0, 1.0, 0.0, -(level - 0.03)];
            // (No bounced light in the mirror: it was gathered for the camera's view.)
            uniform.gi_camera[3] = 0.0;
            queue.write_buffer(
                &self.reflection.camera_buffer,
                0,
                bytemuck::bytes_of(&uniform),
            );
            // Only where the water showing it is on screen (a margin for its wobble and the
            // half-size target's filtering) is the reflection ever read: drawn there only.
            let screen_vp = camera.view_projection(aspect);
            let water_rect = if experiment("allrefl") {
                None
            } else {
                self.water.bodies_at_level(level).into_iter().try_fold(
                    [f32::MAX, f32::MAX, f32::MIN, f32::MIN],
                    |acc, b| {
                        screen_rect(screen_vp, b, size).map(|r| {
                            [
                                acc[0].min(r[0]),
                                acc[1].min(r[1]),
                                acc[2].max(r[2]),
                                acc[3].max(r[3]),
                            ]
                        })
                    },
                )
            }
            .map(|r| {
                const MARGIN: f32 = 24.0;
                [
                    (r[0] - MARGIN).max(0.0),
                    (r[1] - MARGIN).max(0.0),
                    (r[2] + MARGIN).min(size.0 as f32),
                    (r[3] + MARGIN).min(size.1 as f32),
                ]
            })
            .filter(|r| r[2] > r[0] && r[3] > r[1]);
            let views = self.reflection.views.as_ref().expect("reflection target");
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-reflection-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &views.color,
                    resolve_target: None,
                    // Cleared to nothing: its alpha says where something is mirrored (the sky
                    // is left to the water's own painting).
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &views.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.reflection.camera_bind_group, &[]);
            pass.set_bind_group(3, &self.shadow_bind_group, &[]);
            if let Some(r) = water_rect {
                // In the half-size target's pixels.
                let x0 = (r[0] * 0.5).floor() as u32;
                let y0 = (r[1] * 0.5).floor() as u32;
                let x1 = ((r[2] * 0.5).ceil() as u32).min(half.0);
                let y1 = ((r[3] * 0.5).ceil() as u32).min(half.1);
                if x1 > x0 && y1 > y0 {
                    pass.set_scissor_rect(x0, y0, x1 - x0, y1 - y0);
                }
            }
            let mut bound = None;
            let mut run: Option<DrawRun> = None;
            pass.set_bind_group(1, &self.object_bind_group, &[]);
            // Which draws the mirror needs, worked out in chunks on the worker pool (thousands of
            // draws in a forest): solid, inside the mirrored view, and with an image that lands on
            // the water.
            let mut mirrored = vec![false; draws.len()];
            {
                let parts: Vec<(usize, &mut [bool])> =
                    mirrored.chunks_mut(CHUNK).enumerate().collect();
                let test = |(chunk, out): (usize, &mut [bool])| {
                    for (k, keep) in out.iter_mut().enumerate() {
                        let i = chunk * CHUNK + k;
                        let draw = &draws[i];
                        if draw.material.is_shadow_only()
                            || draw.material.is_additive()
                            || draw.material.base_color[3] < 0.99
                        {
                            continue;
                        }
                        let Some(b) = bounds[i] else {
                            continue;
                        };
                        let m = mirrored_vp * draw.transform;
                        if outside_view(&m.to_cols_array(), b) {
                            continue;
                        }
                        // Its image misses the water altogether: nothing of it would be read.
                        if let (Some(w), Some(r)) = (water_rect, screen_rect(m, b, size)) {
                            if r[2] < w[0] || r[0] > w[2] || r[3] < w[1] || r[1] > w[3] {
                                continue;
                            }
                        }
                        *keep = true;
                    }
                };
                crate::work::each(parts, &test);
            }
            for (draw_index, draw) in draws.iter().enumerate() {
                if !mirrored[draw_index] {
                    continue;
                }
                let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                    continue;
                };
                let texture = self.textures.get(
                    draw.texture
                        .or(draw.material.texture)
                        .unwrap_or(self.white_texture)
                        .0 as usize,
                );
                let window = draw.material.occlusion_window;
                let cutout =
                    texture.is_none_or(|t| t.cutout) || (window[2] > 0.0 && window[3] > 0.0);
                let normal_map = texture.is_none_or(|t| t.normal_map);
                let variant = (usize::from(cutout), usize::from(normal_map));
                let texture_id = draw
                    .texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture)
                    .0;
                let key = (draw.mesh.0, texture_id, variant.0 * 2 + variant.1);
                if DrawRun::extends(&run, draw_index, key) {
                    if let Some(r) = run.as_mut() {
                        r.end += 1;
                    }
                    continue;
                }
                DrawRun::flush(&mut run, &mut pass);
                if bound != Some(variant) {
                    pass.set_pipeline(&self.reflect_pipelines[variant.0][variant.1]);
                    bound = Some(variant);
                }
                if let Some(texture) = texture {
                    pass.set_bind_group(2, &texture.bind_group, &[]);
                }
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                run = Some(DrawRun {
                    first: draw_index as u32,
                    end: draw_index as u32 + 1,
                    count: mesh.index_count,
                    key,
                });
            }
            DrawRun::flush(&mut run, &mut pass);
        }
        mark("reflection");
        let prepass = self.quality.depth_prepass || experiment("prepass");
        let in_prepass = |draw: &MeshDraw3D| -> bool {
            if draw.material.is_shadow_only()
                || draw.material.is_additive()
                || draw.material.base_color[3] < 0.99
            {
                return false;
            }
            let window = draw.material.occlusion_window;
            let texture = self.textures.get(
                draw.texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture)
                    .0 as usize,
            );
            !(texture.is_none_or(|t| t.cutout) || (window[2] > 0.0 && window[3] > 0.0))
        };
        if prepass {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-probe-prepass"),
                color_attachments: &[],
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
            pass.set_pipeline(&self.probe_depth_pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(1, &self.object_bind_group, &[]);
            pass.set_bind_group(3, &self.shadow_bind_group, &[]);
            let mut run: Option<DrawRun> = None;
            let mut bound_mesh: Option<u32> = None;
            for (draw_index, draw) in draws.iter().enumerate() {
                if !on_screen[draw_index] || !in_prepass(draw) {
                    continue;
                }
                let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                    continue;
                };
                let tex = draw
                    .texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture);
                let key = (draw.mesh.0, 0, 0);
                if DrawRun::extends(&run, draw_index, key) {
                    if let Some(r) = run.as_mut() {
                        r.end += 1;
                    }
                    continue;
                }
                DrawRun::flush(&mut run, &mut pass);
                if let Some(t) = self.textures.get(tex.0 as usize) {
                    pass.set_bind_group(2, &t.bind_group, &[]);
                }
                if bound_mesh != Some(draw.mesh.0) {
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    bound_mesh = Some(draw.mesh.0);
                }
                run = Some(DrawRun {
                    first: draw_index as u32,
                    end: draw_index as u32 + 1,
                    count: mesh.index_count,
                    key,
                });
            }
            DrawRun::flush(&mut run, &mut pass);
        }
        // The water reads it (or the empty one, with the flag down).
        {
            let mut flags = camera_uniform;
            // (2: show only the reflection on the water, to check it: VELVET_EXP=reflshow.)
            flags.grade[2] = if mirror.is_none() {
                0.0
            } else if experiment("reflshow") {
                2.0
            } else {
                1.0
            };
            flags.grade[3] = mirror.unwrap_or(0.0);
            queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&flags));
        }
        self.water.set_reflection(self.reflection.group.clone());
        {
            let (color_view, resolve_target) = self.post.scene_attachment();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: if prepass {
                            wgpu::LoadOp::Load
                        } else {
                            wgpu::LoadOp::Clear(1.0)
                        },
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_bind_group(3, &self.shadow_bind_group, &[]);
            let mut bound_variant = None;
            let mut bound_texture: Option<u32> = None;
            let mut bound_mesh: Option<u32> = None;
            let mut run: Option<DrawRun> = None;
            pass.set_bind_group(1, &self.object_bind_group, &[]);
            self.last_cutout_draws = 0;
            self.last_culled_draws = 0;
            self.last_drawn_triangles = 0;
            self.last_draw_triangles.clear();
            self.last_draw_triangles.resize(draws.len(), 0);
            // Solid surfaces first, then what blends over them (translucent and added light,
            // which write no depth), each group in the order given.
            let blended = |draw: &MeshDraw3D| {
                draw.material.is_additive() || draw.material.base_color[3] < 0.99
            };
            // The water goes between the two (`None` in the order).
            let ordered = draws
                .iter()
                .enumerate()
                .filter(|(_, d)| !blended(d))
                .map(Some)
                .chain(std::iter::once(None))
                .chain(
                    draws
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| blended(d))
                        .map(Some),
                );
            for entry in ordered {
                let Some((draw_index, draw)) = entry else {
                    DrawRun::flush(&mut run, &mut pass);
                    if water_on && self.water.any_visible() {
                        self.water.draw(&mut pass);
                        // The water changed the pipeline, its groups and buffers.
                        bound_variant = None;
                        bound_texture = None;
                        bound_mesh = None;
                        pass.set_bind_group(1, &self.object_bind_group, &[]);
                    }
                    continue;
                };
                if draw.material.is_shadow_only() {
                    continue;
                }
                let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                    continue;
                };
                // Entirely off screen: skip its vertex work altogether (worked out with the
                // uniforms, on the worker pool).
                if !on_screen[draw_index] {
                    self.last_culled_draws += 1;
                    continue;
                }
                let texture = draw
                    .texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture);
                let texture = self.textures.get(texture.0 as usize);
                let window = draw.material.occlusion_window;
                let cutout =
                    texture.is_none_or(|t| t.cutout) || (window[2] > 0.0 && window[3] > 0.0);
                let normal_map = texture.is_none_or(|t| t.normal_map);
                self.last_cutout_draws += usize::from(cutout);
                // 0..3 the solid variants, 4 translucent, 5 additive.
                let variant = if draw.material.is_additive() {
                    5
                } else if draw.material.base_color[3] < 0.99 {
                    4
                } else {
                    usize::from(cutout) * 2 + usize::from(normal_map)
                };
                let (indices, index_count) = mesh.level(levels[draw_index]);
                self.last_drawn_triangles += index_count as usize / 3;
                self.last_draw_triangles[draw_index] = index_count / 3;
                let texture_id = draw
                    .texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture)
                    .0;
                // The mesh and its level: what the buffers bound are.
                let geometry = draw.mesh.0 * 4 + u32::from(levels[draw_index]);
                let key = (geometry, texture_id, variant);
                if DrawRun::extends(&run, draw_index, key) {
                    if let Some(r) = run.as_mut() {
                        r.end += 1;
                    }
                    continue;
                }
                DrawRun::flush(&mut run, &mut pass);
                let variant = if prepass && variant < 2 {
                    10 + variant
                } else {
                    variant
                };
                if bound_variant != Some(variant) {
                    pass.set_pipeline(match variant {
                        4 => &self.translucent_pipeline,
                        5 => &self.additive_pipeline,
                        10 | 11 => &self.probe_equal_pipelines[variant - 10],
                        v => &self.pipelines[v / 2][v % 2],
                    });
                    bound_variant = Some(variant);
                }
                // What is still bound from the draw before is not bound again (the water's
                // drawing in between forgets it).
                if let Some(texture) = texture {
                    if bound_texture != Some(texture_id) {
                        pass.set_bind_group(2, &texture.bind_group, &[]);
                        bound_texture = Some(texture_id);
                    }
                }
                if bound_mesh != Some(geometry) {
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                    bound_mesh = Some(geometry);
                }
                run = Some(DrawRun {
                    first: draw_index as u32,
                    end: draw_index as u32 + 1,
                    count: index_count,
                    key,
                });
            }
            DrawRun::flush(&mut run, &mut pass);
        }
        mark("main-pass");
        let finish = self.finish;
        self.post.finish(
            queue,
            &mut encoder,
            target,
            crate::post3d::PostSettings {
                bloom_strength: if experiment("nobloom") {
                    0.0
                } else {
                    finish.bloom.max(0.0)
                },
                bloom_threshold: finish.bloom_threshold.max(0.0),
                bloom_knee: (finish.bloom_threshold * 0.5).max(0.05),
                saturation: finish.saturation,
                contrast: finish.contrast,
                exposure: finish.exposure.max(0.0),
                bloom_levels: self.quality.bloom_levels as usize,
            },
            clear,
        );
        if self.finish.ink > 0.0 || self.finish.occlusion > 0.0 {
            self.draw_outline(device, queue, &mut encoder, target, size, camera);
        }
        mark("post");
        queue.submit(std::iter::once(encoder.finish()));
        mark("submit");
        if profiling {
            thread_local! {
                static SUMS: std::cell::RefCell<(u32, Vec<(&'static str, f64)>)> = const { std::cell::RefCell::new((0, Vec::new())) };
            }
            SUMS.with(|sums| {
                let mut sums = sums.borrow_mut();
                let mut last = cpu_started;
                for (i, (name, at)) in marks.iter().enumerate() {
                    let ms = at.duration_since(last).as_secs_f64() * 1000.0;
                    last = *at;
                    if sums.1.len() <= i {
                        sums.1.push((name, 0.0));
                    }
                    sums.1[i].1 += ms;
                }
                sums.0 += 1;
                if sums.0 == 120 {
                    let line: Vec<String> = sums
                        .1
                        .iter()
                        .map(|(n, t)| format!("{n} {:.2}", t / 120.0))
                        .collect();
                    eprintln!("[cpuprof] {} (draws {})", line.join(", "), draws.len());
                    sums.0 = 0;
                    sums.1.clear();
                }
            });
        }
        if gi_on {
            self.gi
                .shaded(crate::gi3d::GiCamera::of(&camera, self.depth_size));
        }
        self.directional_shadows = saved_shadows;
        crate::RENDER3D_CPU_NS.fetch_add(
            cpu_started.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// The ink lines of the finish, over the frame just drawn into `target`, read from its depth.
    fn draw_outline(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
        camera: Camera3D,
    ) {
        let finish = self.finish;
        let height = size.1.max(1) as f32;
        let (orthographic, edges, scale) = match camera.orthographic_height {
            // Silhouettes from a step of a fifth of a metre in depth (the prototype's, at its
            // camera distance), creases only where the slope across the screen turns sharply
            // (a wall on the ground, a roof's ridge), not between the cards of a tree's leaves.
            Some(h) => (1.0, [0.2, 0.7, 0.85, 1.7], h.max(0.01) / height),
            None => (0.0, [0.006, 0.02, 0.0022, 0.006], 0.0),
        };
        let uniform = OutlineUniform {
            params: [
                finish.ink.clamp(0.0, 1.0),
                finish.ink_thickness.max(1.0),
                finish.ink_fade[0],
                finish.ink_fade[1].max(finish.ink_fade[0] + 0.01),
            ],
            ink: [
                finish.ink_color[0],
                finish.ink_color[1],
                finish.ink_color[2],
                orthographic,
            ],
            clip: [
                camera.near,
                camera.far,
                size.0.max(1) as f32,
                size.1.max(1) as f32,
            ],
            edges,
            // The contact shadows only for the orthographic camera (metres per pixel known).
            scale: [
                scale,
                if orthographic > 0.5 && !experiment("noao") {
                    finish.occlusion.clamp(0.0, 1.0)
                } else {
                    0.0
                },
                finish.occlusion_radius.max(0.05),
                0.0,
            ],
        };
        queue.write_buffer(&self.outline_buffer, 0, bytemuck::bytes_of(&uniform));
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-outline"),
            layout: &self.outline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.outline_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.depth_view),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("velvet-3d-outline-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.outline_pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The water's reflection target: the mirrored scene at half the frame's size, its camera, and
/// the bind group the water samples it through.
struct Reflection {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    views: Option<ReflectionViews>,
    size: (u32, u32),
    /// What the water binds (a 1×1 stand-in until the first reflection).
    group: Option<std::sync::Arc<wgpu::BindGroup>>,
}

struct ReflectionViews {
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
}

impl Reflection {
    fn new(device: &wgpu::Device, camera_layout: &wgpu::BindGroupLayout) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-reflection-layout"),
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
            label: Some("velvet-3d-reflection-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("velvet-3d-reflection-camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("velvet-3d-reflection-camera"),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let mut reflection = Self {
            layout,
            sampler,
            camera_buffer,
            camera_bind_group,
            views: None,
            size: (0, 0),
            group: None,
        };
        reflection.ensure(device, (1, 1));
        reflection
    }

    /// The target at this size (made again when the frame's size changes).
    fn ensure(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.size == size && self.views.is_some() {
            return;
        }
        let extent = wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velvet-3d-reflection"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: crate::post3d::HDR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("velvet-3d-reflection-depth"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth = depth.create_view(&wgpu::TextureViewDescriptor::default());
        self.group = Some(std::sync::Arc::new(device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                label: Some("velvet-3d-reflection-group"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&color),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            },
        )));
        self.views = Some(ReflectionViews { color, depth });
        self.size = size;
    }
}

/// Albedo textures go to the GPU as BC7 (set by the renderer when its device takes them).
static BC7_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Below this PSNR (dB) a texture stays uncompressed: the loss would show.
const BC7_MIN_PSNR: f64 = 34.0;

/// Group 3 of the mesh shader: the shadow map and its comparison sampler, the bounced light and
/// its sampler.
fn frame_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shadow_view: &wgpu::TextureView,
    shadow_sampler: &wgpu::Sampler,
    gi_view: &wgpu::TextureView,
    gi_sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("velvet-3d-shadow-texture-bind-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(shadow_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(gi_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(gi_sampler),
            },
        ],
    })
}

/// Profiling switches (VELVET_EXP=noshadow,nobloom,nolights,noblend,nowater,norefl,noao,nobc,nogi):
/// each frame's cost without one part, to see what it costs.
fn experiment(name: &str) -> bool {
    static FLAGS: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    FLAGS
        .get_or_init(|| std::env::var("VELVET_EXP").unwrap_or_default())
        .split(',')
        .any(|f| f == name)
}

fn create_depth_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    sample_count: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-depth"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        // Read back by the finish's ink pass.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// The sun's shadow map, cached. Whatever keeps still (the ground, the wood, the walls) is
/// drawn into a cached map only when the sun has moved half a degree or the view has crossed
/// into another cell, and then spread over a few frames into a second map that takes over
/// when it is done. Each frame starts from a copy of the cached map and adds only what moves
/// (her, people, animated or moving things). A caster is "still" when the same mesh stood with
/// the same transform last frame and its vertices have not been animated for a while.
/// Where a box (`bounds`, placed by `clip`) lands on a `size` screen, in pixels (x0, y0, x1, y1);
/// `None` when part of it is behind the camera (then it may be anywhere).
fn screen_rect(clip: Mat4, bounds: [[f32; 3]; 2], size: (u32, u32)) -> Option<[f32; 4]> {
    let [lo, hi] = bounds;
    let m = clip.to_cols_array();
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for i in 0..8 {
        let p = [
            if i & 1 == 0 { lo[0] } else { hi[0] },
            if i & 2 == 0 { lo[1] } else { hi[1] },
            if i & 4 == 0 { lo[2] } else { hi[2] },
        ];
        let x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
        let y = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
        let w = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
        if w.is_nan() || w <= 1e-4 || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let (u, v) = (
            (x / w * 0.5 + 0.5) * size.0 as f32,
            (0.5 - y / w * 0.5) * size.1 as f32,
        );
        x0 = x0.min(u);
        y0 = y0.min(v);
        x1 = x1.max(u);
        y1 = y1.max(v);
    }
    Some([x0, y0, x1, y1])
}

/// Draws next to each other in the list with the same mesh, texture and pipeline: drawn as one
/// call, its instances the draws (each reads its own data by its index), in the same order.
struct DrawRun {
    first: u32,
    end: u32,
    count: u32,
    key: (u32, u32, usize),
}

impl DrawRun {
    /// Draw `index` (key `key`, `count` indices) joins this run if it comes right after it.
    fn extends(run: &Option<DrawRun>, index: usize, key: (u32, u32, usize)) -> bool {
        run.as_ref()
            .is_some_and(|r| r.end as usize == index && r.key == key)
    }

    /// The run so far, drawn (if any).
    fn flush(run: &mut Option<DrawRun>, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(r) = run.take() {
            pass.draw_indexed(0..r.count, 0, r.first..r.end);
        }
    }
}

/// The keys of the cached frame in use.
fn keys_of_current(cache: &ShadowCache) -> &KeySet {
    static EMPTY: std::sync::OnceLock<KeySet> = std::sync::OnceLock::new();
    cache
        .current
        .as_ref()
        .map_or_else(|| EMPTY.get_or_init(Default::default), |f| &f.keys)
}

/// The texels of the shadow map a box (`bounds`, placed by `clip`: light projection * model)
/// can draw into, two texels wider all round; `None` when it falls outside the map.
fn shadow_footprint(clip: Mat4, bounds: [[f32; 3]; 2]) -> Option<[u32; 4]> {
    let [lo, hi] = bounds;
    let m = clip.to_cols_array();
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for i in 0..8 {
        let p = [
            if i & 1 == 0 { lo[0] } else { hi[0] },
            if i & 2 == 0 { lo[1] } else { hi[1] },
            if i & 4 == 0 { lo[2] } else { hi[2] },
        ];
        let x = m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12];
        let y = m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13];
        let w = m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15];
        if !(x.is_finite() && y.is_finite() && w.is_finite()) || w.abs() < 1e-6 {
            // Unprojectable: the whole map.
            return Some([0, 0, SHADOW_MAP_SIZE, SHADOW_MAP_SIZE]);
        }
        let (u, v) = ((x / w) * 0.5 + 0.5, 1.0 - ((y / w) * 0.5 + 0.5));
        x0 = x0.min(u);
        y0 = y0.min(v);
        x1 = x1.max(u);
        y1 = y1.max(v);
    }
    let size = SHADOW_MAP_SIZE as f32;
    let to = |v: f32, pad: f32| (v * size + pad).clamp(0.0, size);
    let (ax, ay, bx, by) = (
        to(x0, -2.0).floor(),
        to(y0, -2.0).floor(),
        to(x1, 2.0).ceil(),
        to(y1, 2.0).ceil(),
    );
    if bx <= ax || by <= ay {
        return None;
    }
    Some([ax as u32, ay as u32, (bx - ax) as u32, (by - ay) as u32])
}

/// Rectangles whose union is hardly bigger than the two of them become one (no area is put back
/// for nothing); past 128 of them, the smallest unions are taken until they fit.
fn merge_rects(rects: Vec<[u32; 4]>) -> Vec<[u32; 4]> {
    // Many (the frames after a zone change, when every caster is still a mover): the pairwise
    // merge below would take seconds, so they go to a coarse grid of the map first.
    if rects.len() > 48 {
        return grid_rects(&rects);
    }
    pair_rects(rects)
}

/// Rectangles covering `rects`, made of a 32×32 grid's cells: each row's runs of touched cells,
/// runs of the same span in rows one under the other joined. Always covers them.
fn grid_rects(rects: &[[u32; 4]]) -> Vec<[u32; 4]> {
    const CELLS: u32 = 32;
    let tile = SHADOW_MAP_SIZE.div_ceil(CELLS);
    let mut touched = [[false; CELLS as usize]; CELLS as usize];
    for r in rects {
        if r[2] == 0 || r[3] == 0 {
            continue;
        }
        let (x0, y0) = (r[0] / tile, r[1] / tile);
        let x1 = ((r[0] + r[2] - 1) / tile).min(CELLS - 1);
        let y1 = ((r[1] + r[3] - 1) / tile).min(CELLS - 1);
        for row in touched.iter_mut().take(y1 as usize + 1).skip(y0 as usize) {
            for cell in row.iter_mut().take(x1 as usize + 1).skip(x0 as usize) {
                *cell = true;
            }
        }
    }
    let mut out: Vec<[u32; 4]> = Vec::new();
    // The runs of the row above, as indices into `out`, to grow down when the span matches.
    let mut above: Vec<usize> = Vec::new();
    for (y, row) in touched.iter().enumerate() {
        let mut here = Vec::new();
        let mut x = 0;
        while x < CELLS as usize {
            if !row[x] {
                x += 1;
                continue;
            }
            let start = x;
            while x < CELLS as usize && row[x] {
                x += 1;
            }
            let (rx, rw) = (start as u32 * tile, (x - start) as u32 * tile);
            let rw = rw.min(SHADOW_MAP_SIZE - rx);
            let ry = y as u32 * tile;
            let rh = tile.min(SHADOW_MAP_SIZE - ry);
            if let Some(&i) = above.iter().find(|&&i| out[i][0] == rx && out[i][2] == rw) {
                out[i][3] += rh;
                here.push(i);
            } else {
                out.push([rx, ry, rw, rh]);
                here.push(out.len() - 1);
            }
        }
        above = here;
    }
    out
}

/// Pairs of rectangles joined while that wastes little (or while there are over 128).
fn pair_rects(mut rects: Vec<[u32; 4]>) -> Vec<[u32; 4]> {
    let area = |r: &[u32; 4]| r[2] as u64 * r[3] as u64;
    let union = |a: [u32; 4], b: [u32; 4]| {
        let (x0, y0) = (a[0].min(b[0]), a[1].min(b[1]));
        let (x1, y1) = (
            (a[0] + a[2]).max(b[0] + b[2]),
            (a[1] + a[3]).max(b[1] + b[3]),
        );
        [x0, y0, x1 - x0, y1 - y0]
    };
    loop {
        let mut best: Option<(usize, usize, u64)> = None;
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                let u = union(rects[i], rects[j]);
                let waste = area(&u).saturating_sub(area(&rects[i]) + area(&rects[j]));
                let cheap = area(&u) * 10 <= (area(&rects[i]) + area(&rects[j])) * 11;
                if (cheap || rects.len() > 128) && best.is_none_or(|b| waste < b.2) {
                    best = Some((i, j, waste));
                }
            }
        }
        let Some((i, j, _)) = best else {
            return rects;
        };
        let b = rects.swap_remove(j);
        rects[i] = union(rects[i], b);
    }
}

/// Writes the cached map's depth into the frame's map inside a scissor (no copy of a part of a
/// depth texture exists): a full-screen triangle that loads each texel and outputs it as depth.
struct ShadowRestore {
    pipeline: wgpu::RenderPipeline,
    groups: [wgpu::BindGroup; 2],
    /// Last frame's moving casters' rectangles on the frame's map.
    dirty: Vec<[u32; 4]>,
    /// Which cached map (and of which version) the frame's map holds outside `dirty`.
    synced: Option<(usize, u64)>,
    /// Bumped whenever the map in use changes content (redrawn, or swapped for a new build).
    version: u64,
}

const SHADOW_RESTORE_SHADER: &str = r#"
@group(0) @binding(0) var cached: texture_depth_2d;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32((index << 1u) & 2u) * 2.0 - 1.0;
    let y = f32(index & 2u) * 2.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

struct Out {
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> Out {
    var out: Out;
    out.depth = textureLoad(cached, vec2<i32>(position.xy), 0);
    return out;
}
"#;

impl ShadowRestore {
    fn new(device: &wgpu::Device, views: &[wgpu::TextureView; 2]) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("velvet-3d-shadow-restore-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let group = |view: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("velvet-3d-shadow-restore-group"),
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                }],
            })
        };
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("velvet-3d-shadow-restore-shader"),
            source: wgpu::ShaderSource::Wgsl(SHADOW_RESTORE_SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("velvet-3d-shadow-restore-pipeline-layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("velvet-3d-shadow-restore-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        Self {
            pipeline,
            groups: [group(&views[0]), group(&views[1])],
            dirty: Vec::new(),
            synced: None,
            version: 0,
        }
    }
}

struct ShadowCache {
    maps: [wgpu::Texture; 2],
    views: [wgpu::TextureView; 2],
    /// The map in use.
    front: usize,
    current: Option<ShadowFrame>,
    build: Option<ShadowBuild>,
    /// Every caster's key last frame.
    previous: KeySet,
    /// A camera uniform for the build pass (its light projection is the new frame's).
    build_camera: wgpu::Buffer,
    build_camera_bind_group: wgpu::BindGroup,
}

#[derive(Clone)]
struct ShadowFrame {
    view_projection: Mat4,
    direction: Vec3,
    center: (i32, i32),
    extent: f32,
    keys: KeySet,
}

struct ShadowBuild {
    frame: ShadowFrame,
    step: u32,
}

/// The view moves in cells this wide (metres) before the cache follows; the map reaches two
/// cells past the scene's shadow extent so the view never leaves it while a new one builds.
const SHADOW_CELL: f32 = 4.0;
/// Build steps of a new cached map.
const SHADOW_BUILD_STEPS: u32 = 6;
/// The sun may turn this far (radians) before the cache follows it gradually, and this far
/// before it is redrawn at once.
const SHADOW_TURN_STEP: f32 = 0.5 * std::f32::consts::PI / 180.0;
const SHADOW_TURN_JUMP: f32 = 5.0 * std::f32::consts::PI / 180.0;
/// Vertices animated within this many frames make a caster move.
const SHADOW_ANIMATED_FRAMES: u64 = 600;

fn shadow_projection(direction: Vec3, center: (i32, i32), extent: f32) -> Mat4 {
    let up = if direction.y.abs() > 0.99 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let target = Vec3::new(
        center.0 as f32 * SHADOW_CELL,
        0.5,
        center.1 as f32 * SHADOW_CELL,
    );
    Mat4::orthographic_rh_zo(-extent, extent, -extent, extent, 1.0, SHADOW_DEPTH)
        * Mat4::look_at_rh(target + direction * SHADOW_DISTANCE, target, up)
}

/// A caster's identity from frame to frame: its mesh, its placement (bit for bit) and whether it
/// is see-through. A quick multiply-and-rotate mix: these keys only meet each other in one run.
fn caster_key(draw: &MeshDraw3D) -> u64 {
    const K: u64 = 0x9e37_79b9_7f4a_7c15;
    let mix = |h: u64, v: u64| (h.rotate_left(5) ^ v).wrapping_mul(K);
    let mut h = mix(0x51_7cc1_b727_220a, u64::from(draw.mesh.0));
    let m = draw.transform.to_cols_array();
    for pair in m.as_chunks::<2>().0 {
        h = mix(
            h,
            u64::from(pair[0].to_bits()) | u64::from(pair[1].to_bits()) << 32,
        );
    }
    let window = draw.material.occlusion_window;
    h = mix(h, u64::from(window[2] > 0.0 && window[3] > 0.0));
    // A final avalanche, so the low bits (the build's step picks by them) are well spread.
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^ (h >> 33)
}

/// Hashes nothing more: the keys put in it are hashes already.
#[derive(Default, Clone, Copy)]
struct KeyHasher(u64);

impl std::hash::Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 << 8) | u64::from(b);
        }
    }
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }
}

/// A set of caster keys.
type KeySet = std::collections::HashSet<u64, std::hash::BuildHasherDefault<KeyHasher>>;

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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn validate_texture_data(texture: &TextureData3D) -> Result<(), Render3DError> {
    let expected = texture.width.max(1) as usize * texture.height.max(1) as usize * 4;
    if texture.rgba.len() < expected {
        return Err(Render3DError::Gpu(format!(
            "texture buffer too small: {} < {expected}",
            texture.rgba.len()
        )));
    }
    Ok(())
}

fn mip_level_count(width: u32, height: u32) -> u32 {
    let largest = width.max(height).max(1);
    32 - largest.leading_zeros()
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

/// sRGB byte to linear, and linear (in 1/16383 steps) to sRGB byte: the mip chain's conversions
/// as lookups (each `powf` cost more than everything else in a zone's load).
fn srgb_tables() -> &'static ([f32; 256], Vec<u8>) {
    static TABLES: std::sync::OnceLock<([f32; 256], Vec<u8>)> = std::sync::OnceLock::new();
    TABLES.get_or_init(|| {
        let mut to_linear = [0.0f32; 256];
        for (k, v) in to_linear.iter_mut().enumerate() {
            *v = srgb_to_linear(k as f32 / 255.0);
        }
        let to_srgb = (0..LINEAR_STEPS + 1)
            .map(|k| (linear_to_srgb(k as f32 / LINEAR_STEPS as f32) * 255.0).round() as u8)
            .collect();
        (to_linear, to_srgb)
    })
}

const LINEAR_STEPS: usize = 16383;

/// One mip level from the one above: each pixel the mean of its 2x2 (colour averaged in linear
/// light, normals as vectors), the rows shared among the cores.
fn downsample(source: &[u8], width: u32, height: u32, normal_map: bool) -> (u32, u32, Vec<u8>) {
    let next_width = (width / 2).max(1);
    let next_height = (height / 2).max(1);
    let mut next = vec![0_u8; (next_width * next_height * 4) as usize];
    let (to_linear, to_srgb) = srgb_tables();
    let row_bytes = (next_width * 4) as usize;
    let rows = |first: u32, out: &mut [u8]| {
        for (r, out_row) in out.chunks_exact_mut(row_bytes).enumerate() {
            let y = first + r as u32;
            for x in 0..next_width {
                let mut alpha = 0u32;
                let mut acc = [0.0_f32; 3];
                let mut samples = 0u32;
                for sy in 0..2 {
                    let sample_y = (y * 2 + sy).min(height - 1);
                    for sx in 0..2 {
                        let sample_x = (x * 2 + sx).min(width - 1);
                        let index = ((sample_y * width + sample_x) * 4) as usize;
                        if index + 3 >= source.len() {
                            continue;
                        }
                        let px = &source[index..index + 4];
                        if normal_map {
                            for c in 0..3 {
                                acc[c] += px[c] as f32 / 255.0 * 2.0 - 1.0;
                            }
                        } else {
                            for c in 0..3 {
                                acc[c] += to_linear[px[c] as usize];
                            }
                        }
                        alpha += px[3] as u32;
                        samples += 1;
                    }
                }
                let n = samples.max(1);
                let o = (x * 4) as usize;
                if normal_map {
                    let length = (acc[0] * acc[0] + acc[1] * acc[1] + acc[2] * acc[2]).sqrt();
                    let unit = if length > 0.0001 {
                        [acc[0] / length, acc[1] / length, acc[2] / length]
                    } else {
                        [0.0, 0.0, 1.0]
                    };
                    for c in 0..3 {
                        out_row[o + c] = ((unit[c] * 0.5 + 0.5) * 255.0).round() as u8;
                    }
                } else {
                    for c in 0..3 {
                        let linear = (acc[c] / n as f32).clamp(0.0, 1.0);
                        out_row[o + c] = to_srgb[(linear * LINEAR_STEPS as f32).round() as usize];
                    }
                }
                out_row[o + 3] = (alpha as f32 / n as f32).round() as u8;
            }
        }
    };
    let threads = crate::work::threads();
    if next_height as usize * row_bytes < 64 * 1024 || threads < 2 {
        rows(0, &mut next);
    } else {
        // (On the kept threads: making new ones for every level cost memory never given back.)
        let per = (next_height as usize).div_ceil(threads);
        crate::work::each(
            next.chunks_mut(per * row_bytes).enumerate().collect(),
            &|(k, chunk): (usize, &mut [u8])| rows((k * per) as u32, chunk),
        );
    }
    (next_width, next_height, next)
}

/// Build a complete CPU mip chain.  wgpu can generate mipmaps on a render
/// pass, but doing it here keeps uploads deterministic on every backend and
/// lets normal maps be averaged as vectors instead of blurring their encoded
/// RGB values into a visibly lumpy surface.
fn build_mip_chain(data: &TextureData3D, normal_map: bool) -> Vec<(u32, u32, Vec<u8>)> {
    let mut width = data.width.max(1);
    let mut height = data.height.max(1);
    let source_len = (width * height * 4) as usize;
    let mut levels = vec![(width, height, data.rgba[..source_len].to_vec())];
    while width > 1 || height > 1 {
        let level = {
            let source = levels
                .last()
                .map(|(_, _, rgba)| rgba.as_slice())
                .unwrap_or(&[]);
            downsample(source, width, height, normal_map)
        };
        width = level.0;
        height = level.1;
        levels.push(level);
    }
    levels
}

/// The mip chain the slow, exact way (each conversion computed): the reference the fast one is
/// held to.
#[cfg(test)]
fn build_mip_chain_reference(data: &TextureData3D, normal_map: bool) -> Vec<(u32, u32, Vec<u8>)> {
    let mut width = data.width.max(1);
    let mut height = data.height.max(1);
    let mut levels = vec![(
        width,
        height,
        data.rgba[..(width * height * 4) as usize].to_vec(),
    )];
    while width > 1 || height > 1 {
        let (nw, nh) = ((width / 2).max(1), (height / 2).max(1));
        let source = levels.last().unwrap().2.clone();
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let (mut a, mut c, mut nrm, mut n) = (0.0f32, [0.0f32; 3], [0.0f32; 3], 0.0f32);
                for sy in 0..2 {
                    for sx in 0..2 {
                        let i = ((((y * 2 + sy).min(height - 1)) * width
                            + (x * 2 + sx).min(width - 1))
                            * 4) as usize;
                        for k in 0..3 {
                            let v = source[i + k] as f32 / 255.0;
                            if normal_map {
                                nrm[k] += v * 2.0 - 1.0
                            } else {
                                c[k] += srgb_to_linear(v)
                            }
                        }
                        a += source[i + 3] as f32 / 255.0;
                        n += 1.0;
                    }
                }
                let o = ((y * nw + x) * 4) as usize;
                if normal_map {
                    let l = (nrm[0] * nrm[0] + nrm[1] * nrm[1] + nrm[2] * nrm[2]).sqrt();
                    let u = if l > 0.0001 {
                        nrm.map(|v| v / l)
                    } else {
                        [0.0, 0.0, 1.0]
                    };
                    for k in 0..3 {
                        next[o + k] = ((u[k] * 0.5 + 0.5) * 255.0).round() as u8;
                    }
                } else {
                    for k in 0..3 {
                        next[o + k] = (linear_to_srgb(c[k] / n) * 255.0).round() as u8;
                    }
                }
                next[o + 3] = ((a / n) * 255.0).round() as u8;
            }
        }
        levels.push((nw, nh, next));
        width = nw;
        height = nh;
    }
    levels
}

fn upload_mip_chain(queue: &wgpu::Queue, texture: &wgpu::Texture, levels: &[(u32, u32, Vec<u8>)]) {
    for (mip_level, (width, height, rgba)) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip_level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(*height),
            },
            wgpu::Extent3d {
                width: *width,
                height: *height,
                depth_or_array_layers: 1,
            },
        );
    }
}

#[allow(clippy::too_many_arguments)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum BlendMode3D {
    /// Opaque and alpha-cut surfaces (alpha to coverage under MSAA), depth written.
    Solid,
    /// Premultiplied over, smooth (no coverage dither), depth tested but not written.
    Translucent,
    /// Added light, depth tested but not written.
    Additive,
}

#[allow(clippy::too_many_arguments)]
fn create_mesh_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    sample_count: u32,
    cutout: bool,
    normal_map: bool,
    mode: BlendMode3D,
    mirrored: bool,
) -> wgpu::RenderPipeline {
    create_mesh_pipeline_with(
        device,
        layout,
        shader,
        format,
        sample_count,
        cutout,
        normal_map,
        mode,
        mirrored,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn create_mesh_pipeline_with(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    sample_count: u32,
    cutout: bool,
    normal_map: bool,
    mode: BlendMode3D,
    mirrored: bool,
    compare: Option<wgpu::CompareFunction>,
) -> wgpu::RenderPipeline {
    let constants = std::collections::HashMap::from([(
        "NORMAL_MAP".to_string(),
        if normal_map { 1.0 } else { 0.0 },
    )]);
    let label = match (mode, cutout, normal_map) {
        (BlendMode3D::Translucent, _, _) => "velvet-3d-translucent-pipeline",
        (BlendMode3D::Additive, _, _) => "velvet-3d-additive-pipeline",
        (_, true, true) => "velvet-3d-pipeline",
        (_, true, false) => "velvet-3d-pipeline-flat",
        (_, false, true) => "velvet-3d-opaque-pipeline",
        (_, false, false) => "velvet-3d-opaque-pipeline-flat",
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
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
            module: shader,
            entry_point: Some(if cutout { "fs_main" } else { "fs_opaque" }),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                // Premultiplied into the HDR buffer: its alpha is the coverage. Added light
                // leaves the coverage alone.
                blend: Some(if mode == BlendMode3D::Additive {
                    wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }
                } else {
                    wgpu::BlendState {
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
                    }
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants,
                ..Default::default()
            },
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            // A mirrored view turns every triangle's winding over.
            cull_mode: Some(if mirrored {
                wgpu::Face::Front
            } else {
                wgpu::Face::Back
            }),
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: mode == BlendMode3D::Solid,
            depth_compare: compare.unwrap_or(if mode == BlendMode3D::Solid {
                wgpu::CompareFunction::Less
            } else {
                wgpu::CompareFunction::LessEqual
            }),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            mask: !0,
            alpha_to_coverage_enabled: sample_count > 1 && mode == BlendMode3D::Solid,
        },
        multiview: None,
        cache: None,
    })
}

/// An albedo + normal pair with its mip chains built on the CPU, ready for a
/// cheap GPU upload. Building it is the expensive part and is thread-safe.
#[derive(Clone, Debug)]
pub struct PreparedTexture3D {
    albedo: Vec<(u32, u32, Vec<u8>)>,
    /// The albedo's mips as BC7 blocks, when the device takes them and the image survives it.
    albedo_bc7: Option<Vec<Vec<u8>>>,
    normal: Vec<(u32, u32, Vec<u8>)>,
    cutout: bool,
    normal_map: bool,
}

impl PreparedTexture3D {
    /// Validate the images and build both mip chains; a missing normal map
    /// becomes a flat one.
    pub fn new(
        texture: &TextureData3D,
        normal: Option<&TextureData3D>,
    ) -> Result<Self, Render3DError> {
        validate_texture_data(texture)?;
        if let Some(normal) = normal {
            validate_texture_data(normal)?;
        }
        // A flat tangent-space normal is the identity perturbation.
        let flat = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![128, 128, 255, 255],
        };
        // The shader cuts out texels under alpha 0.38; filtering never
        // produces an alpha below the smallest source texel.
        let cutout = texture
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|texel| texel[3] < 100);
        let albedo = build_mip_chain(texture, false);
        // A quarter of the memory as BC7, unless the image would show it (a guard on its PSNR).
        let albedo_bc7 = (BC7_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
            && texture.width.is_multiple_of(4)
            && texture.height.is_multiple_of(4)
            && texture.width * texture.height >= 64 * 64
            && !experiment("nobc"))
        .then(|| {
            let mut levels = Vec::with_capacity(albedo.len());
            for (k, (w, h, rgba)) in albedo.iter().enumerate() {
                let (blocks, psnr) = crate::bc7::encode_measured(*w, *h, rgba);
                if k == 0 && psnr < BC7_MIN_PSNR {
                    return None;
                }
                levels.push(blocks);
            }
            Some(levels)
        })
        .flatten();
        // Compressed, the plain levels are only their sizes from here on.
        let albedo = if albedo_bc7.is_some() {
            albedo
                .into_iter()
                .map(|(w, h, _)| (w, h, Vec::new()))
                .collect()
        } else {
            albedo
        };
        Ok(Self {
            albedo,
            albedo_bc7,
            normal: build_mip_chain(normal.unwrap_or(&flat), true),
            cutout,
            normal_map: normal.is_some(),
        })
    }
}

fn create_gpu_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    prepared: &PreparedTexture3D,
) -> GpuTexture3D {
    let albedo_levels = &prepared.albedo;
    let (width, height) = (albedo_levels[0].0, albedo_levels[0].1);
    let bc7 = prepared.albedo_bc7.as_ref().filter(|_| {
        device
            .features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-albedo"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mip_level_count(width, height),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: if bc7.is_some() {
            wgpu::TextureFormat::Bc7RgbaUnormSrgb
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        },
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    match bc7 {
        Some(levels) => {
            for (mip, ((w, h, _), blocks)) in albedo_levels.iter().zip(levels).enumerate() {
                let (bw, bh) = (w.div_ceil(4), h.div_ceil(4));
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: mip as u32,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    blocks,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bw * 16),
                        rows_per_image: Some(bh),
                    },
                    // A compressed mip's copy covers whole blocks.
                    wgpu::Extent3d {
                        width: bw * 4,
                        height: bh * 4,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        None => upload_mip_chain(queue, &texture, albedo_levels),
    }
    let normal_levels = &prepared.normal;
    let (normal_width, normal_height) = (normal_levels[0].0, normal_levels[0].1);
    let normal_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-normal"),
        size: wgpu::Extent3d {
            width: normal_width,
            height: normal_height,
            depth_or_array_layers: 1,
        },
        mip_level_count: mip_level_count(normal_width, normal_height),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    upload_mip_chain(queue, &normal_texture, normal_levels);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let normal_view = normal_texture.create_view(&wgpu::TextureViewDescriptor::default());
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
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&normal_view),
            },
        ],
    });
    GpuTexture3D {
        texture,
        normal_texture,
        bind_group,
        cutout: prepared.cutout,
        normal_map: prepared.normal_map,
    }
}

/// The mesh shader's source (for the shader tests of other modules).
#[cfg(test)]
pub(crate) fn mesh_shader_source() -> &'static str {
    MESH_SHADER
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
    gi_view_projection: mat4x4<f32>,
    gi_camera: vec4<f32>,
    gi_forward: vec4<f32>,
    quality: vec4<f32>,
};
struct Object {
    model: mat4x4<f32>,
    base_color: vec4<f32>,
    occlusion_window: vec4<f32>,
    normal_matrix: mat3x3<f32>,
    light_mask: vec4<u32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
// Every draw's data; this one's, read at the top of each stage by its instance.
@group(1) @binding(0) var<storage, read> objects: array<Object>;
var<private> object: Object;
@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(2) var normal_texture: texture_2d<f32>;
@group(3) @binding(0) var shadow_texture: texture_depth_2d;
@group(3) @binding(1) var shadow_sampler: sampler_comparison;
@group(3) @binding(2) var gi_texture: texture_2d<f32>;
@group(3) @binding(3) var gi_sampler: sampler;

// The light bounced onto this point by what is round it (see gi3d.rs): read where the point was
// on the frame it was gathered for, four taps round it (a little blur over the gather's noise),
// each only if the surface there is this one.
fn bounced_light(world: vec3<f32>) -> vec3<f32> {
    if (camera.gi_camera.w <= 0.0) {
        return vec3<f32>(0.0);
    }
    let clip = camera.gi_view_projection * vec4<f32>(world, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0))) {
        return vec3<f32>(0.0);
    }
    let along = dot(world - camera.gi_camera.xyz, camera.gi_forward.xyz);
    let texel = 1.0 / vec2<f32>(textureDimensions(gi_texture));
    var light = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = 0; i < 4; i = i + 1) {
        let corner = vec2<f32>(f32(i & 1) * 2.0 - 1.0, f32(i >> 1u) * 2.0 - 1.0);
        let tap = textureSampleLevel(gi_texture, gi_sampler, uv + corner * texel * 0.9, 0.0);
        let w = 1.0 - smoothstep(0.2, 0.8, abs(tap.a - along));
        light = light + tap.rgb * w;
        weight = weight + w;
    }
    return light / max(weight, 0.0001) * min(weight, 1.0) * camera.gi_camera.w;
}

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @builtin(instance_index) instance: u32,
};
struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) world_position: vec3<f32>,
    @location(4) light_clip: vec4<f32>,
    @location(5) @interpolate(flat) instance: u32,
    // The draw's own values the fragments read, handed down so they need no buffer read.
    @location(6) @interpolate(flat) base_color: vec4<f32>,
    @location(7) @interpolate(flat) occlusion_window: vec4<f32>,
    @location(8) @interpolate(flat) light_mask: u32,
};

@vertex
fn vs_main(input: VertexIn) -> VertexOut {
    object = objects[input.instance];
    let world = object.model * vec4<f32>(input.position, 1.0);
    var out: VertexOut;
    out.instance = input.instance;
    out.base_color = object.base_color;
    out.occlusion_window = object.occlusion_window;
    out.light_mask = object.light_mask.x;
    out.clip = camera.view_projection * world;
    // Inverse transpose via cofactors (computed per draw on the CPU):
    // non-uniformly fitted cave art must not bend its lighting normals.
    out.normal = normalize(object.normal_matrix * input.normal);
    out.color = input.color;
    out.uv = input.uv;
    out.world_position = world.xyz;
    // Only read when directional shadows are on (`local_light_meta.y` 0).
    out.light_clip = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    if (camera.local_light_meta.y < 0.5) {
        let offset = out.normal * max(camera.local_light_meta.w, 0.001) * 1.5;
        out.light_clip = camera.light_view_projection * vec4<f32>(world.xyz + offset, 1.0);
    }
    return out;
}

const POISSON: array<vec2<f32>, 16> = array<vec2<f32>, 16>(
    vec2<f32>(-0.94201624, -0.39906216), vec2<f32>(0.94558609, -0.76890725),
    vec2<f32>(-0.09418410, -0.92938870), vec2<f32>(0.34495938, 0.29387760),
    vec2<f32>(-0.91588581, 0.45771432), vec2<f32>(-0.81544232, -0.87912464),
    vec2<f32>(-0.38277543, 0.27676845), vec2<f32>(0.97484398, 0.75648379),
    vec2<f32>(0.44323325, -0.97511554), vec2<f32>(0.53742981, -0.47373420),
    vec2<f32>(-0.26496911, -0.41893023), vec2<f32>(0.79197514, 0.19090188),
    vec2<f32>(-0.24188840, 0.99706507), vec2<f32>(-0.81409955, 0.91437590),
    vec2<f32>(0.19984126, 0.78641367), vec2<f32>(0.14383161, -0.14100790),
);

fn shadow_visibility(light_clip: vec4<f32>, normal: vec3<f32>) -> f32 {
    let ndc = light_clip.xyz / light_clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5));
    if (uv.x <= 0.0 || uv.x >= 1.0 || uv.y <= 0.0 || uv.y >= 1.0 || ndc.z <= 0.0 || ndc.z >= 1.0) {
        return 1.0;
    }
    let light = normalize(camera.light_direction.xyz);
    let range = max(camera.local_light_meta.z, 1.0);
    let texel_m = max(camera.local_light_meta.w, 0.001);
    // Steeper to the light, more bias; never less than a texel and a half.
    let ndl = clamp(dot(normal, light), 0.0, 1.0);
    let slope = sqrt(max(1.0 - ndl * ndl, 0.0)) / max(ndl, 0.2);
    let bias = (texel_m * 1.5 + texel_m * 1.5 * slope) / range;
    let texel = 1.0 / vec2<f32>(textureDimensions(shadow_texture));
    // A per-pixel turn of the disc: banding becomes a fine grain the toon ramp smooths.
    let angle = fract(sin(dot(floor(uv / texel), vec2<f32>(12.9898, 78.233))) * 43758.5453) * 6.2831853;
    let rot = mat2x2<f32>(cos(angle), sin(angle), -sin(angle), cos(angle));
    // As many of the disc's taps as the quality asks (all sixteen: the full look).
    let taps = i32(clamp(camera.quality.x, 1.0, 16.0));
    var visibility = 0.0;
    if (taps == 16) {
        // The four taps at the disc's rim first: when they agree (all lit, all in shadow: most of
        // the screen) the inner twelve would too, and are skipped; in a penumbra the other twelve
        // are added and the sum is the full disc's, as before.
        let rim = array<i32, 4>(0, 1, 7, 13);
        for (var k: i32 = 0; k < 4; k = k + 1) {
            let offset = rot * POISSON[rim[k]] * texel * 1.5;
            visibility = visibility + textureSampleCompareLevel(shadow_texture, shadow_sampler, uv + offset, ndc.z - bias);
        }
        if (visibility < 0.0001 || visibility > 3.9999) {
            return visibility * 0.25;
        }
        for (var i: i32 = 0; i < 16; i = i + 1) {
            if (i == 0 || i == 1 || i == 7 || i == 13) {
                continue;
            }
            let offset = rot * POISSON[i] * texel * 1.5;
            visibility = visibility + textureSampleCompareLevel(shadow_texture, shadow_sampler, uv + offset, ndc.z - bias);
        }
        return visibility / 16.0;
    }
    for (var i: i32 = 0; i < taps; i = i + 1) {
        let offset = rot * POISSON[i] * texel * 1.5;
        visibility = visibility + textureSampleCompareLevel(shadow_texture, shadow_sampler, uv + offset, ndc.z - bias);
    }
    return visibility / f32(taps);
}

// Pipeline constant: false for textures without a normal map, which then
// shade with the interpolated geometric normal directly.
override NORMAL_MAP: bool = true;

// Reconstruct a tangent frame from screen-space derivatives so meshes that
// only provide positions, normals and UVs (including the generated meadow)
// can still use a tangent-space normal map. A flat [0.5, 0.5, 1] sample falls
// back to the authored geometric normal without changing legacy materials.
fn normal_from_map(input: VertexOut) -> vec3<f32> {
    let base_normal = normalize(input.normal);
    if (!NORMAL_MAP) {
        return base_normal;
    }
    let sampled = textureSample(normal_texture, albedo_sampler, input.uv).xyz * 2.0
        - vec3<f32>(1.0, 1.0, 1.0);
    let position_dx = dpdx(input.world_position);
    let position_dy = dpdy(input.world_position);
    let uv_dx = dpdx(input.uv);
    let uv_dy = dpdy(input.uv);
    let determinant = uv_dx.x * uv_dy.y - uv_dx.y * uv_dy.x;
    if (abs(determinant) < 0.00001) {
        return base_normal;
    }
    var tangent = (position_dx * uv_dy.y - position_dy * uv_dx.y) / determinant;
    tangent = normalize(tangent - base_normal * dot(base_normal, tangent));
    if (length(tangent) < 0.00001) {
        return base_normal;
    }
    var bitangent = normalize(cross(base_normal, tangent));
    let expected_bitangent = (position_dy * uv_dx.x - position_dx * uv_dy.x) / determinant;
    if (dot(bitangent, expected_bitangent) < 0.0) {
        bitangent = -bitangent;
    }
    // Keep the detail readable but avoid turning high-frequency grass pixels
    // into a faceted surface. The normal map itself remains full strength
    // data and can be reused by scenes with a different tuning later.
    let detail = select(0.72, camera.toon_params.w, camera.toon_params.x > 0.5);
    let mapped = normalize(vec3<f32>(sampled.xy * detail, max(sampled.z, 0.28)));
    return normalize(tangent * mapped.x + bitangent * mapped.y + base_normal * mapped.z);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn distribution_ggx(normal: vec3<f32>, halfway: vec3<f32>, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let ndoth = max(dot(normal, halfway), 0.0);
    let denominator = ndoth * ndoth * (alpha2 - 1.0) + 1.0;
    return alpha2 / max(3.14159265 * denominator * denominator, 0.0001);
}

fn geometry_schlick_ggx(ndot: f32, roughness: f32) -> f32 {
    let k = ((roughness + 1.0) * (roughness + 1.0)) / 8.0;
    return ndot / max(ndot * (1.0 - k) + k, 0.0001);
}

// Shared band positions/weights with ToonStyle3D::diffuse. Angular bands only:
// distance falloff remains continuous so moving lamps never produce rings.
fn toon_diffuse(x: f32) -> f32 {
    let w = camera.toon_params.y;
    return 0.08 + 0.28 * smoothstep(0.22-w, 0.22+w, x)
        + 0.31 * smoothstep(0.55-w, 0.55+w, x)
        + 0.33 * smoothstep(0.85-w, 0.85+w, x);
}

fn evaluate_brdf(
    albedo: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    light_dir: vec3<f32>,
    radiance: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {
    let ndotl = max(dot(normal, light_dir), 0.0);
    let ndotv = max(dot(normal, view_dir), 0.001);
    if (camera.toon_params.x > 0.5) {
        let band = toon_diffuse(ndotl);
        let shade = mix(camera.toon_shadow.rgb, vec3<f32>(1.0), band);
        let halfway = normalize(view_dir + light_dir);
        let threshold = mix(0.92, 0.99, roughness);
        let highlight = smoothstep(threshold-0.012, threshold+0.012,
            max(dot(normal, halfway), 0.0)) * smoothstep(0.08, 0.22, ndotl);
        let diffuse = albedo * shade * band;
        let specular = vec3<f32>(highlight * camera.toon_params.z * (1.0-roughness*0.6));
        return (diffuse + specular) * radiance / 3.14159265;
    }
    if (ndotl <= 0.0) {
        return vec3<f32>(0.0);
    }
    let halfway = normalize(view_dir + light_dir);
    let f0 = vec3<f32>(0.04);
    let fresnel = fresnel_schlick(max(dot(halfway, view_dir), 0.0), f0);
    let distribution = distribution_ggx(normal, halfway, roughness);
    let geometry = geometry_schlick_ggx(ndotv, roughness)
        * geometry_schlick_ggx(ndotl, roughness);
    let specular = distribution * geometry * fresnel
        / max(4.0 * ndotv * ndotl, 0.001);
    let diffuse_weight = (vec3<f32>(1.0) - fresnel) * 0.96;
    let diffuse = diffuse_weight * albedo / 3.14159265;
    return (diffuse + specular) * radiance * ndotl;
}

fn local_lighting(
    albedo: vec3<f32>,
    normal: vec3<f32>,
    view_dir: vec3<f32>,
    world_position: vec3<f32>,
    roughness: f32,
) -> vec3<f32> {
    var total = vec3<f32>(0.0);
    // Only the lights that reach this draw (worked out per draw on the CPU), in order.
    var mask = object.light_mask.x;
    loop {
        if (mask == 0u) {
            break;
        }
        let index = firstTrailingBit(mask);
        mask = mask & (mask - 1u);
        let packed_position = camera.local_light_positions[index];
        let packed_color = camera.local_light_colors[index];
        let to_light = packed_position.xyz - world_position;
        let distance_squared = max(dot(to_light, to_light), 0.0001);
        let radius = max(packed_position.w, 0.05);
        // Out of reach: skip before any square root or lighting maths.
        if (distance_squared >= radius * radius) {
            continue;
        }
        let distance = sqrt(distance_squared);
        let normalized_distance = distance / radius;
        let cutoff = max(1.0 - normalized_distance * normalized_distance
            * normalized_distance * normalized_distance, 0.0);
        let attenuation = cutoff * cutoff / (1.0 + distance_squared * 0.22);
        let radiance = packed_color.rgb * packed_color.w * attenuation;
        total = total + evaluate_brdf(
            albedo,
            normal,
            view_dir,
            to_light / distance,
            radiance,
            roughness,
        );
    }
    return total;
}

// Solid surfaces: no `discard` anywhere, so the GPU can reject hidden
// fragments before shading them. Identical output for textures without
// transparent texels and draws without a reveal window.
@fragment
fn fs_opaque(input: VertexOut) -> @location(0) vec4<f32> {
    return shade(input, false);
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    return shade(input, true);
}

fn shade(input: VertexOut, cutout: bool) -> vec4<f32> {
    object.base_color = input.base_color;
    object.occlusion_window = input.occlusion_window;
    object.light_mask = vec4<u32>(input.light_mask, 0u, 0u, 0u);
    if (dot(camera.clip_plane.xyz, input.world_position) + camera.clip_plane.w < 0.0) {
        discard;
    }
    let normal = normal_from_map(input);
    let light_dir = normalize(camera.light_direction.xyz);
    var view_dir = normalize(camera.camera_position.xyz - input.world_position);
    if (camera.viewport_size.z > 0.5) {
        view_dir = normalize(camera.camera_position.xyz - camera.focus_position.xyz);
    }
    // Authored floor receivers use the same sentinel as the shadow pass. They
    // remain fully key-lit so a bad/coplanar shadow sample cannot turn an
    // entire room floor black; contact AO still anchors props to the tiles.
    // `local_light_meta.y` marks a frame without directional shadows: the
    // map is empty, so skip its nine samples.
    // (Not worked out where it is not used: a floor receiver is always lit, and with no sun at
    // all the visibility multiplies nothing.)
    var visibility = 1.0;
    if (camera.local_light_meta.y < 0.5
        && object.occlusion_window.x >= -0.5
        && any(camera.light_color.rgb != vec3<f32>(0.0))) {
        visibility = shadow_visibility(input.light_clip, normal);
    }
    if (camera.toon_params.x > 0.5) {
        visibility = smoothstep(0.32, 0.68, visibility);
    }
    let hemisphere = clamp(normal.y * 0.5 + 0.5, 0.0, 1.0);
    let sky = camera.ambient_color.rgb * (0.52 + hemisphere * 0.48);
    let bounce = camera.bounce_color.rgb * (1.0 - hemisphere) * 0.62;
    let moon_dir = normalize(camera.moon_direction.xyz);
    let reflection_band = max(normal.y, 0.0)
        * (0.35 + 0.65 * pow(max(dot(normal, moon_dir), 0.0), 2.0));
    let lunar_reflection = camera.reflection_color.rgb
        * camera.reflection_color.a
        * reflection_band;
    let twilight = camera.horizon_tint.rgb
        * camera.horizon_tint.a
        * (1.0 - hemisphere)
        * 0.35;
    let albedo = textureSample(albedo_texture, albedo_sampler, input.uv);
    // KayKit foliage and decals use glTF MASK materials. Treat their alpha as
    // a cutout when the draw itself is opaque; blending the transparent texel
    // fringe was the source of the soft rectangular halos that made assets
    // look lower quality than the Three.js preview. Explicitly translucent
    // gameplay markers keep their old blended path.
    // Added light keeps its soft texture edges (no cut).
    if (cutout && object.base_color.a > 0.95 && albedo.a < 0.38 && object.occlusion_window.x > -1.5) {
        discard;
    }
    let base = object.base_color.rgb * input.color.rgb * albedo.rgb;
    if (object.occlusion_window.x < -1.5) {
        let to_camera = length(camera.camera_position.xyz - input.world_position);
        let haze_span = max(camera.fog_params.y - camera.fog_params.x, 0.001);
        var haze = clamp((to_camera - camera.fog_params.x) / haze_span, 0.0, 1.0);
        // A sky dome is never hazed (`Material3D::sky`).
        if (object.occlusion_window.w < -0.5) {
            haze = 0.0;
        }
        return vec4<f32>(base * (1.0 - haze), clamp(object.base_color.a * input.color.a * albedo.a, 0.0, 1.0));
    }
    let luminance = dot(base, vec3<f32>(0.2126, 0.7152, 0.0722));
    // The source materials are authored around a 0.45 roughness. The old
    // luminance-only range (0.48..0.78) made every prop chalky and removed
    // the small highlights that communicate bevels at the isometric scale.
    let roughness = clamp(0.56 - luminance * 0.14, 0.36, 0.68);
    // Analytic contact/cavity AO keeps this forward path inexpensive while
    // still reacting to the asset's grounded placement. `window.y` is an
    // optional per-material strength; derivatives add a restrained crease
    // term on bevels without requiring a deferred depth pyramid.
    let ground_contact = exp(-max(input.world_position.y, 0.0) * 2.75)
        * clamp(normal.y * 0.5 + 0.5, 0.0, 1.0);
    let normal_variation = clamp(
        (length(dpdx(normal)) + length(dpdy(normal))) * 0.55,
        0.0,
        1.0,
    );
    let authored_ao = clamp(object.occlusion_window.y, 0.0, 1.0);
    let ambient_ao = clamp(
        1.0
            - authored_ao * (0.12 + ground_contact * 0.50)
            - normal_variation * (0.045 + authored_ao * 0.13),
        0.58,
        1.0,
    );
    let direct = evaluate_brdf(
        base,
        normal,
        view_dir,
        light_dir,
        camera.light_color.rgb * 2.35 * mix(0.24, 1.0, visibility),
        roughness,
    );
    // (By day the moon gives nothing: not worked out.)
    var moon = vec3<f32>(0.0);
    if (any(camera.moon_color.rgb * camera.moon_color.a != vec3<f32>(0.0))) {
        moon = evaluate_brdf(
            base,
            normal,
            view_dir,
            moon_dir,
            camera.moon_color.rgb * camera.moon_color.a * 1.8,
            roughness,
        );
    }
    let local = local_lighting(base, normal, view_dir, input.world_position, roughness);
    var rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0) * 0.065;
    if (camera.toon_params.x > 0.5) {
        rim = smoothstep(0.68, 0.82, 1.0-max(dot(normal, view_dir), 0.0)) * camera.toon_shadow.a;
    }
    // With the bounced light on, part of the flat fill gives way to it.
    var flat_fill = 1.0;
    if (camera.gi_camera.w > 0.0) {
        flat_fill = camera.gi_forward.w;
    }
    let indirect = base * ((sky + bounce) * flat_fill + lunar_reflection + twilight + bounced_light(input.world_position)) * ambient_ao;
    let emission = base * max(object.occlusion_window.x, 0.0);
    let lit_color = indirect + direct + moon + local + camera.ambient_color.rgb * rim + emission;
    let distance_to_camera = length(camera.camera_position.xyz - input.world_position);
    let fog_span = max(camera.fog_params.y - camera.fog_params.x, 0.001);
    let linear_fog = clamp((distance_to_camera - camera.fog_params.x) / fog_span, 0.0, 1.0);
    let exponential_fog = 1.0 - exp(-distance_to_camera * max(camera.fog_params.z, 0.0));
    let height_fog = clamp(max(input.world_position.y - 0.15, 0.0) * camera.fog_params.w, 0.0, 1.0);
    let fog_factor = clamp(max(linear_fog, exponential_fog * 0.55) + linear_fog * height_fog * 0.35, 0.0, 1.0);
    // Linear HDR: the composite tone maps, grades and blooms it.
    let color = max(mix(lit_color, camera.fog_color.rgb, fog_factor), vec3<f32>(0.0));
    var alpha = object.base_color.a * input.color.a * albedo.a;
    // A wall keeps its full shape and lighting; only a soft screen-space
    // window centred on the camera focus lets the Queen read through it. The
    // aspect correction makes the window circular even on a widescreen view.
    let window = object.occlusion_window;
    if (window.z > 0.0 && window.w > 0.0) {
        let focus_clip = camera.view_projection * vec4<f32>(camera.focus_position.xyz, 1.0);
        if (focus_clip.w > 0.0001) {
            let focus_ndc = focus_clip.xy / focus_clip.w;
            // Fragment-stage `position` is in viewport pixels, not the clip
            // coordinates emitted by the vertex stage. Convert it explicitly
            // before comparing it with the projected focus point.
            let viewport = max(camera.viewport_size.xy, vec2<f32>(1.0, 1.0));
            let fragment_ndc = vec2<f32>(
                input.clip.x / viewport.x * 2.0 - 1.0,
                1.0 - input.clip.y / viewport.y * 2.0,
            );
            let aspect = max(camera.focus_position.w, 0.01);
            let delta = vec2<f32>(
                (fragment_ndc.x - focus_ndc.x) * aspect,
                fragment_ndc.y - focus_ndc.y,
            );
            let distance_to_focus = length(delta);
            let inner = window.z * 0.46;
            let feather = 1.0 - smoothstep(inner, window.z, distance_to_focus);
            alpha = alpha * (1.0 - feather * clamp(window.w, 0.0, 1.0));
            // Do not let the fully transparent centre keep writing depth. It
            // would hide props submitted behind the wall even though the
            // colour has been removed. The feathered rim remains blended and
            // keeps the reveal polished instead of looking like a hard cut.
            if cutout && alpha < 0.035 {
                discard;
            }
        }
    }
    return vec4<f32>(color, alpha);
}
"#;

const OUTLINE_SHADER: &str = r#"
struct Outline {
    params: vec4<f32>,
    ink: vec4<f32>,
    clip: vec4<f32>,
    edges: vec4<f32>,
    scale: vec4<f32>,
};
@group(0) @binding(0) var<uniform> outline: Outline;
@group(0) @binding(1) var depth_texture: __DEPTH__;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_outline(@builtin(vertex_index) index: u32) -> VertexOut {
    // One triangle covering the whole target.
    let x = f32((index << 1u) & 2u);
    let y = f32(index & 2u);
    var out: VertexOut;
    out.clip = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return out;
}

// Distance along the view of what one depth sample holds (`raw` 1.0: nothing there).
fn view_depth(coord: vec2<i32>, sample_index: i32) -> f32 {
    let size = vec2<i32>(outline.clip.zw);
    let c = clamp(coord, vec2<i32>(0), size - vec2<i32>(1));
    let d = textureLoad(depth_texture, c, sample_index);
    let near = outline.clip.x;
    let far = outline.clip.y;
    if (outline.ink.w > 0.5) {
        return near + d * (far - near);
    }
    return near * far / max(far - d * (far - near), 0.000001);
}

// How much of a line passes through one sample.
fn edge_at(p: vec2<i32>, s: i32) -> f32 {
    let raw = textureLoad(depth_texture, p, s);
    if (raw >= 0.99999) {
        return 0.0;
    }
    let t = max(i32(round(outline.params.y)), 1);
    let c = view_depth(p, s);
    let l = view_depth(p - vec2<i32>(t, 0), s);
    let r = view_depth(p + vec2<i32>(t, 0), s);
    let u = view_depth(p - vec2<i32>(0, t), s);
    let d = view_depth(p + vec2<i32>(0, t), s);
    var sil = 0.0;
    var crease = 0.0;
    if (outline.ink.w > 0.5) {
        // Orthographic: depth is affine across a plane, so its second difference is zero on
        // flat faces; both measured in metres, the crease per metre of screen.
        let jump = max(max(l - c, r - c), max(u - c, d - c));
        sil = smoothstep(outline.edges.x, outline.edges.y, jump);
        let lap = (abs(l + r - 2.0 * c) + abs(u + d - 2.0 * c)) / (outline.scale.x * f32(t));
        crease = smoothstep(outline.edges.z, outline.edges.w, lap) * step(jump, outline.edges.y);
    } else {
        // Perspective: 1/depth is the affine one; both relative to the distance.
        let jump = max(max(l - c, r - c), max(u - c, d - c)) / c;
        sil = smoothstep(outline.edges.x, outline.edges.y, jump);
        let ic = 1.0 / c;
        let lap = abs(1.0 / l + 1.0 / r - 2.0 * ic) + abs(1.0 / u + 1.0 / d - 2.0 * ic);
        crease = smoothstep(outline.edges.z, outline.edges.w, lap / ic) * step(jump, outline.edges.y);
    }
    let e = max(sil, crease * 0.8);
    return e * (1.0 - smoothstep(outline.params.z, outline.params.w, c));
}

// Where the pixel's surface is, in metres in the orthographic view (x, y across the screen, z
// toward the camera).
fn view_point(p: vec2<i32>) -> vec3<f32> {
    let s = outline.scale.x;
    return vec3<f32>(f32(p.x) * s, -f32(p.y) * s, -view_depth(p, 0));
}

fn hash12(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Contact shadow: how much of the hemisphere over the surface the nearby depth closes off (the
// corner of a wall and the ground, under eaves, round the feet of things). Its normal is made from
// the depth itself (the side with the smaller step, so edges do not smear).
fn occlusion_at(p: vec2<i32>) -> f32 {
    let strength = outline.scale.y;
    if (strength <= 0.0 || outline.ink.w < 0.5) {
        return 0.0;
    }
    if (textureLoad(depth_texture, p, 0) >= 0.99999) {
        return 0.0;
    }
    let c = view_point(p);
    let l = view_point(p - vec2<i32>(1, 0));
    let r = view_point(p + vec2<i32>(1, 0));
    let u = view_point(p - vec2<i32>(0, 1));
    let d = view_point(p + vec2<i32>(0, 1));
    let dx = select(c - l, r - c, abs(r.z - c.z) < abs(c.z - l.z));
    let dy = select(c - u, d - c, abs(d.z - c.z) < abs(c.z - u.z));
    var n = normalize(cross(dx, dy));
    if (n.z < 0.0) {
        n = -n;
    }
    let radius = outline.scale.z;
    let px = radius / max(outline.scale.x, 0.00001);
    let spin = hash12(vec2<f32>(p)) * 6.2831853;
    var occluded = 0.0;
    for (var k = 0; k < 10; k = k + 1) {
        let f = (f32(k) + 0.5) / 10.0;
        let a = spin + f32(k) * 2.3999632;
        let q = p + vec2<i32>(vec2<f32>(cos(a), sin(a)) * px * sqrt(f));
        let v = view_point(q) - c;
        let dist = length(v);
        if (dist < 0.0001) {
            continue;
        }
        occluded = occluded + max(dot(n, v) / dist - 0.15, 0.0) * (1.0 - smoothstep(radius * 0.6, radius * 1.6, dist));
    }
    return clamp(occluded / 10.0 * 2.2, 0.0, 1.0) * strength;
}

@fragment
fn fs_outline(input: VertexOut) -> @location(0) vec4<f32> {
    let p = vec2<i32>(input.clip.xy);
    // Every sample of the pixel: the lines come out antialiased like the frame.
    var e = 0.0;
    if (outline.params.x > 0.0) {
        for (var s = 0; s < __SAMPLES__; s = s + 1) {
            e = e + edge_at(p, s);
        }
    }
    let k = clamp(e / f32(__SAMPLES__) * outline.params.x, 0.0, 1.0);
    let ao = occlusion_at(p);
    if (k <= 0.002 && ao <= 0.002) {
        discard;
    }
    // Over the frame (premultiplied): colour * (1 - 0.8k)(1 - ao) + ink * 0.08k.
    return vec4<f32>(outline.ink.rgb * 0.08 * k, 1.0 - (1.0 - 0.8 * k) * (1.0 - ao));
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
// The same layout as the mesh shader's (an array: every field counts for its stride).
struct Object {
    model: mat4x4<f32>,
    base_color: vec4<f32>,
    occlusion_window: vec4<f32>,
    normal_matrix: mat3x3<f32>,
    light_mask: vec4<u32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<storage, read> objects: array<Object>;
var<private> object: Object;
@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance: u32,
};

struct ShadowOut {
    @builtin(position) light_clip: vec4<f32>,
    @location(0) camera_clip: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) instance: u32,
    @location(3) @interpolate(flat) base_color: vec4<f32>,
    @location(4) @interpolate(flat) occlusion_window: vec4<f32>,
};

@vertex
fn vs_shadow(input: VertexIn) -> ShadowOut {
    object = objects[input.instance];
    let world = object.model * vec4<f32>(input.position, 1.0);
    var out: ShadowOut;
    out.instance = input.instance;
    out.base_color = object.base_color;
    out.occlusion_window = object.occlusion_window;
    out.light_clip = camera.light_view_projection * world;
    out.camera_clip = camera.view_projection * world;
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_shadow(input: ShadowOut) {
    object.base_color = input.base_color;
    object.occlusion_window = input.occlusion_window;
    let window = object.occlusion_window;
    let albedo = textureSample(albedo_texture, albedo_sampler, input.uv);
    if (object.base_color.a > 0.95 && albedo.a < 0.38) {
        discard;
    }
    if (window.z <= 0.0 || window.w <= 0.0 || input.camera_clip.w <= 0.0001) {
        return;
    }
    let focus_clip = camera.view_projection * vec4<f32>(camera.focus_position.xyz, 1.0);
    if (focus_clip.w <= 0.0001) {
        return;
    }
    let fragment_ndc = input.camera_clip.xy / input.camera_clip.w;
    let focus_ndc = focus_clip.xy / focus_clip.w;
    let aspect = max(camera.focus_position.w, 0.01);
    let delta = vec2<f32>(
        (fragment_ndc.x - focus_ndc.x) * aspect,
        fragment_ndc.y - focus_ndc.y,
    );
    // Shadow maps cannot blend a feather. Remove only the fully transparent
    // centre and retain the surrounding penumbra, so activating the reveal
    // never makes an entire wall's shadow pop out of existence.
    if (length(delta) < window.z * 0.62) {
        discard;
    }
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_of_shadow_rects_merge_at_once_and_stay_covered() {
        // As after a zone change: every caster of the forest a mover.
        let mut seed = 7u32;
        let mut next = |m: u32| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) % m
        };
        let rects: Vec<[u32; 4]> = (0..5000)
            .map(|_| {
                let (x, y) = (next(SHADOW_MAP_SIZE - 300), next(SHADOW_MAP_SIZE - 300));
                [x, y, 1 + next(250), 1 + next(250)]
            })
            .collect();
        let started = std::time::Instant::now();
        let merged = merge_rects(rects.clone());
        assert!(
            started.elapsed() < std::time::Duration::from_millis(50),
            "{:?}",
            started.elapsed()
        );
        assert!(merged.len() <= 32 * 16);
        let inside = |p: (u32, u32)| {
            merged
                .iter()
                .any(|m| p.0 >= m[0] && p.1 >= m[1] && p.0 < m[0] + m[2] && p.1 < m[1] + m[3])
        };
        for r in &rects {
            for p in [
                (r[0], r[1]),
                (r[0] + r[2] - 1, r[1] + r[3] - 1),
                (r[0] + r[2] / 2, r[1] + r[3] / 2),
            ] {
                assert!(inside(p), "{r:?} left out");
            }
        }
        for m in &merged {
            assert!(m[0] + m[2] <= SHADOW_MAP_SIZE && m[1] + m[3] <= SHADOW_MAP_SIZE);
        }
    }

    #[test]
    fn heightfield_builds_indexed_grid() {
        // 3x3 dome: center high.
        let heights = vec![0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0];
        let mesh = Mesh3D::heightfield(3, 4.0, &heights, Material3D::default());
        assert_eq!(mesh.vertices.len(), 9);
        assert_eq!(mesh.indices.len(), 4 * 6);
        // Center vertex carries the peak height and an up-ish normal.
        let center = &mesh.vertices[4];
        assert!((center.position[1] - 2.0).abs() < 1e-6);
        assert!(center.normal[1] > 0.9);
        assert!(mesh.vertices.iter().all(|v| v.normal[0].is_finite()
            && v.normal[1].is_finite()
            && v.normal[2].is_finite()));
        // Edge-adjacent vertex tilts away from the peak (-x side).
        assert!(mesh.vertices[3].normal[0] < -0.1);
        assert!(mesh.vertices[3].normal[1] > 0.0);
        // Short input degrades to flat instead of panicking.
        let flat = Mesh3D::heightfield(3, 4.0, &[1.0], Material3D::default());
        assert_eq!(flat.vertices.len(), 9);
    }

    #[test]
    fn camera_projection_is_finite() {
        let camera = Camera3D::default();
        let values = camera.view_projection(16.0 / 9.0).to_cols_array();
        assert!(values.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn environment_presets_have_distinct_palettes() {
        let lived = EnvironmentPreset3D::Lived.sample(0.5);
        let demonic = EnvironmentPreset3D::Demonic.sample(0.5);
        assert_eq!(EnvironmentPreset3D::Lived.label(), "Vivida");
        assert_eq!(EnvironmentPreset3D::Demonic.label(), "Demoníaca");
        assert_ne!(lived.clear_color, demonic.clear_color);
        assert_ne!(lived.atmosphere.fog_color, demonic.atmosphere.fog_color);
        assert_ne!(lived.lighting.ambient_color, demonic.lighting.ambient_color);
    }

    #[test]
    fn day_night_cycle_wraps_phase() {
        let mut cycle = DayNightCycle3D::new(EnvironmentPreset3D::Lived, 10.0);
        cycle.set_phase(0.95);
        cycle.advance(1.0);
        assert!((cycle.phase() - 0.05).abs() < 1e-6);
        cycle.advance(f32::NAN);
        assert!((cycle.phase() - 0.05).abs() < 1e-6);
    }

    #[test]
    fn environment_samples_stay_finite_across_cycle() {
        for preset in [EnvironmentPreset3D::Lived, EnvironmentPreset3D::Demonic] {
            for phase in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let frame = preset.sample(phase);
                assert!(frame.lighting.direction_to_light.length() > 0.99);
                assert!(frame.clear_color.iter().all(|value| value.is_finite()));
                assert!(frame
                    .atmosphere
                    .fog_color
                    .iter()
                    .all(|value| value.is_finite()));
            }
        }
    }

    #[test]
    fn celestial_cycle_exposes_moon_and_twilight() {
        let midnight = EnvironmentPreset3D::Lived.sample_with_moon_phase(0.0, 0.5);
        let noon = EnvironmentPreset3D::Lived.sample_with_moon_phase(0.5, 0.5);
        let dawn = EnvironmentPreset3D::Lived.sample_with_moon_phase(0.25, 0.5);
        assert!(midnight.celestial.moon_intensity > noon.celestial.moon_intensity);
        assert!(midnight.celestial.reflection_strength > noon.celestial.reflection_strength);
        assert!(dawn.celestial.horizon_strength > noon.celestial.horizon_strength);
        assert!((midnight.celestial.moon_phase - 0.5).abs() < 1e-6);

        let new_moon = EnvironmentPreset3D::Lived.sample_with_moon_phase(0.0, 0.0);
        assert!(new_moon.celestial.moon_intensity < midnight.celestial.moon_intensity);
    }

    #[test]
    fn environment_cycle_changes_are_continuous() {
        let mut previous = EnvironmentPreset3D::Lived.sample(0.0);
        for index in 1..=240 {
            let frame = EnvironmentPreset3D::Lived.sample(index as f32 / 240.0);
            let clear_delta = frame
                .clear_color
                .iter()
                .zip(previous.clear_color)
                .map(|(current, old)| (current - old).abs())
                .fold(0.0, f32::max);
            let light_delta = frame
                .lighting
                .ambient_color
                .iter()
                .zip(previous.lighting.ambient_color)
                .map(|(current, old)| (current - old).abs())
                .fold(0.0, f32::max);
            assert!(
                clear_delta < 0.05,
                "clear jump at phase {index}: {clear_delta:?} {:?} -> {:?}",
                previous.clear_color,
                frame.clear_color
            );
            assert!(
                light_delta < 0.05,
                "light jump at phase {index}: {light_delta:?}"
            );
            previous = frame;
        }
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
    fn texture_mips_are_complete_and_normalized() {
        let texture = TextureData3D {
            width: 3,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255, 32, 64, 96,
                255, 200, 160, 120, 255,
            ],
        };
        let levels = build_mip_chain(&texture, false);
        assert_eq!(levels.len(), mip_level_count(3, 2) as usize);
        assert_eq!(
            levels.last().map(|(width, height, _)| (*width, *height)),
            Some((1, 1))
        );

        let normal = TextureData3D {
            width: 2,
            height: 2,
            rgba: [128_u8, 128, 255, 255].repeat(4),
        };
        let normal_levels = build_mip_chain(&normal, true);
        assert_eq!(normal_levels.last().unwrap().2, vec![128, 128, 255, 255]);
    }

    #[test]
    fn material_ao_strength_is_clamped_in_window_payload() {
        let material = Material3D::default().with_ambient_occlusion(1.5);
        assert_eq!(material.occlusion_window[1], 1.0);
        let material = material.with_ambient_occlusion(-0.5);
        assert_eq!(material.occlusion_window[1], 0.0);
    }

    #[test]
    fn emissive_material_preserves_ao_and_reveal_payload() {
        let m = Material3D::rgb(0.1, 0.8, 0.7)
            .with_ambient_occlusion(0.4)
            .with_emission(2.0);
        assert_eq!(m.occlusion_window, [2.0, 0.4, 0.0, 0.0]);
        assert_eq!(m.with_emission(f32::NAN).occlusion_window[0], 0.0);
        assert_eq!(m.with_emission(-2.0).occlusion_window[0], 0.0);
        assert_eq!(m.with_emission(20.0).occlusion_window[0], 16.0);
    }

    #[test]
    fn nature_assets_keep_authored_normal_maps() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/demon-queen/data/assets/3d/stylized_nature_test/CommonTree_1.gltf");
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("nature tree should load");
        assert!(model
            .meshes
            .iter()
            .any(|mesh| mesh.normal_texture_index.is_some()));
    }

    #[test]
    fn uv_sphere_has_indexed_triangles() {
        let mesh = Mesh3D::uv_sphere(1.0, 12, 6, Material3D::default());
        assert_eq!(mesh.vertices.len(), (12 + 1) * (6 + 1));
        assert_eq!(mesh.indices.len(), 12 * 6 * 6);
        assert!(mesh
            .indices
            .iter()
            .all(|index| (*index as usize) < mesh.vertices.len()));
    }

    #[test]
    fn the_fast_mip_chain_matches_the_exact_one() {
        for (w, h) in [(67u32, 41u32), (1024, 512)] {
            let rgba: Vec<u8> = (0..w * h * 4)
                .map(|k| (k.wrapping_mul(2654435761) >> 24) as u8)
                .collect();
            let data = TextureData3D {
                width: w,
                height: h,
                rgba,
            };
            for normal in [false, true] {
                let fast = build_mip_chain(&data, normal);
                let exact = build_mip_chain_reference(&data, normal);
                assert_eq!(fast.len(), exact.len());
                for (a, b) in fast.iter().zip(&exact) {
                    assert_eq!((a.0, a.1), (b.0, b.1));
                    let worst =
                        a.2.iter()
                            .zip(&b.2)
                            .map(|(x, y)| (*x as i32 - *y as i32).abs())
                            .max()
                            .unwrap_or(0);
                    assert!(worst <= 1, "{w}x{h} level {}x{} off by {worst}", a.0, a.1);
                }
            }
        }
    }

    /// Where a zone's load time goes: `cargo test -p velvet-render --release load_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn load_cost() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/demon-queen/data/assets/3d");
        for file in [
            "baths/baths_places.glb",
            "baths/baths_ground.glb",
            "baths/baths_room_recepcion.glb",
            "villa/villa_town.glb",
            "villa/villa_life.glb",
            "bosque/bosque_places.glb",
            "brothel/brothel_ground.glb",
        ] {
            let path = root.join(file);
            if !path.exists() {
                continue;
            }
            let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let t0 = std::time::Instant::now();
            let (document, buffers, images) = gltf::import(&path).unwrap();
            let import = t0.elapsed().as_secs_f64() * 1000.0;
            let pixels: usize = images.iter().map(|i| (i.width * i.height) as usize).sum();
            let t1 = std::time::Instant::now();
            let model = parse_gltf(document, buffers, images).unwrap();
            let parse = t1.elapsed().as_secs_f64() * 1000.0;
            let t2 = std::time::Instant::now();
            for t in &model.textures {
                std::hint::black_box(build_mip_chain(t, false));
            }
            let mips = t2.elapsed().as_secs_f64() * 1000.0;
            let verts: usize = model.meshes.iter().map(|m| m.vertices.len()).sum();
            println!(
                "LOAD {file}: {:.1} MB, {} images {:.1} Mpx, {} meshes {} k verts | import+decode {import:.0} ms, parse {parse:.0} ms, mips {mips:.0} ms",
                bytes as f64 / 1e6,
                model.textures.len(),
                pixels as f64 / 1e6,
                model.meshes.len(),
                verts / 1000
            );
        }
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
    fn a_model_packed_whole_loads_as_the_original() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_animated.glb");
        let Ok(original) = std::fs::read(&path) else {
            return;
        };
        let mut packed = b"VGLZ".to_vec();
        packed.extend_from_slice(&1u32.to_le_bytes());
        packed.extend_from_slice(&(original.len() as u64).to_le_bytes());
        packed.extend_from_slice(&miniz_oxide::deflate::compress_to_vec_zlib(&original, 9));
        assert!(packed.len() < original.len());
        assert_eq!(
            unpack_model_bytes(&packed).unwrap().as_ref(),
            original.as_slice()
        );
        // Not packed: given back as it is.
        assert!(matches!(
            unpack_model_bytes(&original).unwrap(),
            std::borrow::Cow::Borrowed(_)
        ));
        let a = GltfModel::load_bytes(&original).unwrap();
        let b = GltfModel::load_bytes(&packed).unwrap();
        assert_eq!(format!("{:?}", a.meshes), format!("{:?}", b.meshes));
        assert_eq!(format!("{:?}", a.textures), format!("{:?}", b.textures));
        assert_eq!(a.animations, b.animations);
        // A cut stream is an error, not a short model.
        assert!(unpack_model_bytes(&packed[..packed.len() / 2]).is_err());
    }

    /// Every model of a packed release loads as its original: the same geometry, animations
    /// and texture sizes, the textures close (QA, by hand, after tools/build_release.py):
    /// `VELVET_PACKED_RELEASE=<release folder> cargo test … every_packed_model -- --ignored`.
    #[test]
    #[ignore]
    fn every_packed_model_loads_as_its_original() {
        let Some(release) = std::env::var_os("VELVET_PACKED_RELEASE").map(std::path::PathBuf::from)
        else {
            return;
        };
        let project =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../demos/demon-queen");
        fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "glb") {
                    out.push(path);
                }
            }
        }
        let mut files = Vec::new();
        walk(&release.join("data/assets/3d"), &mut files);
        assert!(!files.is_empty());
        let (mut checked, mut worst) = (0, (f64::MAX, String::new()));
        for packed_path in &files {
            let rel = packed_path.strip_prefix(&release).unwrap();
            let original = GltfModel::load(project.join(rel))
                .unwrap_or_else(|e| panic!("{}: {e}", rel.display()));
            let packed = GltfModel::load(packed_path)
                .unwrap_or_else(|e| panic!("{} packed: {e}", rel.display()));
            assert_eq!(
                format!("{:?}", original.meshes),
                format!("{:?}", packed.meshes),
                "{}",
                rel.display()
            );
            assert_eq!(original.animations, packed.animations, "{}", rel.display());
            assert_eq!(
                original.textures.len(),
                packed.textures.len(),
                "{}",
                rel.display()
            );
            for (a, b) in original.textures.iter().zip(&packed.textures) {
                assert_eq!(
                    (a.width, a.height),
                    (b.width, b.height),
                    "{}",
                    rel.display()
                );
                let mse = a
                    .rgba
                    .iter()
                    .zip(&b.rgba)
                    .map(|(x, y)| (*x as f64 - *y as f64).powi(2))
                    .sum::<f64>()
                    / a.rgba.len().max(1) as f64;
                let psnr = 10.0 * (255.0f64 * 255.0 / mse.max(1e-9)).log10();
                assert!(psnr > 30.0, "{}: a texture at {psnr:.1} dB", rel.display());
                if psnr < worst.0 {
                    worst = (psnr, rel.display().to_string());
                }
            }
            checked += 1;
        }
        eprintln!(
            "{checked} packed models load as their originals (worst texture {:.1} dB, {})",
            worst.0, worst.1
        );
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

    #[test]
    fn green_dress_queen_glb_exposes_walk_run_and_triangle_budget() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/demon-queen/data/assets/3d/characters/queen_green_dress.glb");
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("green-dress Queen GLB should load");
        assert_eq!(model.meshes.len(), 2);
        assert!(!model.textures.is_empty());
        // Hair inherits scalp-vertex deformation along with the body.
        assert!(model.meshes[0].skinning.is_some());
        assert!(model.meshes[1].skinning.is_some());
        let total_triangles: usize = model.meshes.iter().map(|mesh| mesh.indices.len() / 3).sum();
        assert!(total_triangles < 10_000);
        let walk = model
            .animation_index("Queen_Walk")
            .expect("green walk clip");
        let run = model.animation_index("Queen_Run").expect("green run clip");
        let walk_sample = model
            .sample_animation(0, walk, 0.25)
            .expect("green walk sample");
        let run_sample = model
            .sample_animation(0, run, 0.25)
            .expect("green run sample");
        assert_eq!(walk_sample.len(), model.meshes[0].vertices.len());
        assert_eq!(run_sample.len(), model.meshes[0].vertices.len());
        assert!(!model.meshes[1].vertices.is_empty());
        for clip in [walk, run] {
            let hair = model
                .sample_animation(1, clip, 0.25)
                .expect("scalp-bound hair sample");
            assert_eq!(hair.len(), model.meshes[1].vertices.len());
            assert!(hair
                .iter()
                .all(|v| v.position.iter().all(|x| x.is_finite())));
        }
    }

    #[test]
    fn hair_run_queen_glb_exposes_walk_run_and_skinned_hair() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_hair_run.glb");
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("hair run Queen GLB should load");
        assert_eq!(model.meshes.len(), 2);
        assert!(!model.textures.is_empty());
        let walk = model.animation_index("Queen_Walk").expect("hair walk clip");
        let run = model.animation_index("Queen_Run").expect("run clip");
        assert!(model.meshes.iter().all(|mesh| mesh.skinning.is_some()));
        let hair = model
            .sample_animation(0, run, 0.25)
            .expect("skinned hair run sample");
        let body = model
            .sample_animation(1, walk, 0.25)
            .expect("skinned body walk sample");
        assert_eq!(hair.len(), model.meshes[0].vertices.len());
        assert_eq!(body.len(), model.meshes[1].vertices.len());
    }

    #[test]
    fn hair_lite_queen_glb_exposes_walk_run_and_skinned_cards() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_hair_lite_run.glb",
        );
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("lite hair-card Queen GLB should load");
        assert_eq!(model.meshes.len(), 2);
        assert!(!model.textures.is_empty());
        let walk = model.animation_index("Queen_Walk").expect("lite walk clip");
        let run = model.animation_index("Queen_Run").expect("lite run clip");
        assert!(model.meshes.iter().all(|mesh| mesh.skinning.is_some()));
        assert!(model.meshes[0].vertices.len() < 200_000);
        let hair = model
            .sample_animation(0, run, 0.25)
            .expect("skinned lite hair run sample");
        let body = model
            .sample_animation(1, walk, 0.25)
            .expect("skinned lite body walk sample");
        assert_eq!(hair.len(), model.meshes[0].vertices.len());
        assert_eq!(body.len(), model.meshes[1].vertices.len());
    }

    #[test]
    fn hair_lowpoly_queen_glb_stays_under_ten_thousand_faces() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_hair_lowpoly_run.glb",
        );
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("low-poly Queen GLB should load");
        assert_eq!(model.meshes.len(), 2);
        let walk = model
            .animation_index("Queen_Walk")
            .expect("low-poly walk clip");
        let run = model
            .animation_index("Queen_Run")
            .expect("low-poly run clip");
        assert!(model.meshes.iter().all(|mesh| mesh.skinning.is_some()));
        assert!(model.meshes[0].indices.len() / 3 < 10_000);
        let hair = model
            .sample_animation(0, run, 0.25)
            .expect("skinned low-poly hair run sample");
        let body = model
            .sample_animation(1, walk, 0.25)
            .expect("skinned low-poly body walk sample");
        assert_eq!(hair.len(), model.meshes[0].vertices.len());
        assert_eq!(body.len(), model.meshes[1].vertices.len());
    }

    #[test]
    fn hair_micro_queen_glb_keeps_authored_shape_under_ten_thousand_faces() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../../demos/demon-queen/data/assets/3d/characters/queen_chibi_v2_hair_micro_run.glb",
        );
        if !path.exists() {
            return;
        }
        let model = GltfModel::load(path).expect("micro hair Queen GLB should load");
        assert_eq!(model.meshes.len(), 2);
        let walk = model
            .animation_index("Queen_Walk")
            .expect("micro walk clip");
        let run = model.animation_index("Queen_Run").expect("micro run clip");
        assert!(model.meshes.iter().all(|mesh| mesh.skinning.is_some()));
        assert!(model.meshes[0].indices.len() / 3 < 10_000);
        let hair = model
            .sample_animation(0, run, 0.25)
            .expect("skinned micro hair run sample");
        let body = model
            .sample_animation(1, walk, 0.25)
            .expect("skinned micro body walk sample");
        assert_eq!(hair.len(), model.meshes[0].vertices.len());
        assert_eq!(body.len(), model.meshes[1].vertices.len());
    }
}
