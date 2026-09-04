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
    /// receiver sentinel, `y` carries authored ambient-occlusion strength,
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
                    key_color: [0.18, 0.26, 0.42],
                    ambient_color: [0.08, 0.14, 0.22],
                    bounce_color: [0.06, 0.10, 0.14],
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
                    fog_color: [0.06, 0.09, 0.16],
                    fog_start: 12.0,
                    fog_end: 42.0,
                    fog_density: 0.004,
                    height_falloff: 0.024,
                },
                [0.035, 0.080, 0.095, 1.0],
                [0.006, 0.012, 0.035, 1.0],
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
        EnvironmentPreset3D::Lived => ([0.34, 0.48, 0.82], [0.10, 0.22, 0.48], [1.00, 0.30, 0.08]),
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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ObjectUniform {
    model: [f32; 16],
    base_color: [f32; 4],
    occlusion_window: [f32; 4],
}

const OBJECT_UNIFORM_CAPACITY: usize = 256;
const SHADOW_MAP_SIZE: u32 = 2048;
const MAX_POINT_LIGHTS_3D: usize = 16;

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
                texture_index,
                normal_texture_index,
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
    let textures = used_images
        .into_iter()
        .filter_map(|source_index| images.get_mut(source_index)?.take())
        .map(texture_data)
        .collect();
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
    target_format: wgpu::TextureFormat,
    sample_count: u32,
    msaa_texture: wgpu::Texture,
    msaa_view: wgpu::TextureView,
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
            multisample: wgpu::MultisampleState {
                count: sample_count,
                mask: !0,
                alpha_to_coverage_enabled: sample_count > 1,
            },
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
        let flat_normal_texture = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![128, 128, 255, 255],
        };
        let white_gpu = create_gpu_texture(
            device,
            queue,
            &texture_bind_group_layout,
            &sampler,
            &white_texture,
            Some(&flat_normal_texture),
        );
        let (msaa_texture, msaa_view) = create_msaa_texture(device, format, size, sample_count);
        let (depth_texture, depth_view) = create_depth_texture(device, size, sample_count);
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
            target_format: format,
            sample_count,
            msaa_texture,
            msaa_view,
            depth_texture,
            depth_view,
            depth_size: (size.0.max(1), size.1.max(1)),
            shadow_texture,
            shadow_view,
            shadow_bind_group,
        }
    }

    /// Resize color/depth storage to match the target surface.
    pub fn resize(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        let size = (size.0.max(1), size.1.max(1));
        if self.depth_size == size {
            return;
        }
        let (msaa_texture, msaa_view) =
            create_msaa_texture(device, self.target_format, size, self.sample_count);
        let (texture, view) = create_depth_texture(device, size, self.sample_count);
        self.msaa_texture = msaa_texture;
        self.msaa_view = msaa_view;
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
        validate_texture_data(texture)?;
        if let Some(normal) = normal {
            validate_texture_data(normal)?;
        }
        let gpu = create_gpu_texture(
            device,
            queue,
            &self.texture_bind_group_layout,
            &self.sampler,
            texture,
            normal,
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
            fog_params: [
                atmosphere.fog_start.max(0.0),
                atmosphere
                    .fog_end
                    .max(atmosphere.fog_start.max(0.0) + 0.001),
                atmosphere.fog_density.max(0.0),
                atmosphere.height_falloff.max(0.0),
            ],
            focus_position: [camera.target.x, camera.target.y, camera.target.z, aspect],
            viewport_size: [size.0.max(1) as f32, size.1.max(1) as f32, 0.0, 0.0],
            local_light_meta: [local_light_count as f32, 0.0, 0.0, 0.0],
            local_light_positions,
            local_light_colors,
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
        for (draw_index, draw) in draws.iter().enumerate() {
            let object = ObjectUniform {
                model: draw.transform.to_cols_array(),
                base_color: draw.material.base_color,
                occlusion_window: draw.material.occlusion_window,
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
                // Translucent geometry (interaction markers and particles)
                // should not cast an opaque shadow. Revealed walls still enter
                // this pass: the shadow shader cuts only their matching local
                // circle instead of deleting the asset's complete shadow.
                // A negative occlusion-window X is reserved for authored floor
                // receivers. Floors stay lit by the key/ambient passes but do
                // not write themselves into the directional map, which avoids
                // coplanar tile variants producing a black cell.
                if draw.material.base_color[3] < 0.95 || draw.material.occlusion_window[0] < -0.5 {
                    continue;
                }
                let Some(mesh) = self.meshes.get(draw.mesh.0 as usize) else {
                    continue;
                };
                let offset = self.object_stride * draw_index as u64;
                shadow_pass.set_bind_group(1, &self.object_bind_group, &[offset as u32]);
                let texture = draw
                    .texture
                    .or(draw.material.texture)
                    .unwrap_or(self.white_texture);
                if let Some(texture) = self.textures.get(texture.0 as usize) {
                    shadow_pass.set_bind_group(2, &texture.bind_group, &[]);
                }
                shadow_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                shadow_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                shadow_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
        {
            let color_view = if self.sample_count > 1 {
                &self.msaa_view
            } else {
                target
            };
            let resolve_target = (self.sample_count > 1).then_some(target);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("velvet-3d-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target,
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

fn create_msaa_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    sample_count: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("velvet-3d-msaa-color"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
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
        let next_width = (width / 2).max(1);
        let next_height = (height / 2).max(1);
        let source = levels
            .last()
            .map(|(_, _, rgba)| rgba.as_slice())
            .unwrap_or(&[]);
        let mut next = vec![0_u8; (next_width * next_height * 4) as usize];
        for y in 0..next_height {
            for x in 0..next_width {
                let mut alpha = 0.0;
                let mut color = [0.0_f32; 3];
                let mut normal = [0.0_f32; 3];
                let mut samples: f32 = 0.0;
                for sy in 0..2 {
                    let sample_y = (y * 2 + sy).min(height - 1);
                    for sx in 0..2 {
                        let sample_x = (x * 2 + sx).min(width - 1);
                        let index = ((sample_y * width + sample_x) * 4) as usize;
                        if index + 3 >= source.len() {
                            continue;
                        }
                        let rgba = &source[index..index + 4];
                        let rgb = [
                            rgba[0] as f32 / 255.0,
                            rgba[1] as f32 / 255.0,
                            rgba[2] as f32 / 255.0,
                        ];
                        if normal_map {
                            normal[0] += rgb[0] * 2.0 - 1.0;
                            normal[1] += rgb[1] * 2.0 - 1.0;
                            normal[2] += rgb[2] * 2.0 - 1.0;
                        } else {
                            color[0] += srgb_to_linear(rgb[0]);
                            color[1] += srgb_to_linear(rgb[1]);
                            color[2] += srgb_to_linear(rgb[2]);
                        }
                        alpha += rgba[3] as f32 / 255.0;
                        samples += 1.0;
                    }
                }
                let samples = samples.max(1.0);
                let out_index = ((y * next_width + x) * 4) as usize;
                if normal_map {
                    let length =
                        (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2])
                            .sqrt();
                    let unit = if length > 0.0001 {
                        [normal[0] / length, normal[1] / length, normal[2] / length]
                    } else {
                        [0.0, 0.0, 1.0]
                    };
                    next[out_index] = ((unit[0] * 0.5 + 0.5) * 255.0).round() as u8;
                    next[out_index + 1] = ((unit[1] * 0.5 + 0.5) * 255.0).round() as u8;
                    next[out_index + 2] = ((unit[2] * 0.5 + 0.5) * 255.0).round() as u8;
                } else {
                    next[out_index] = (linear_to_srgb(color[0] / samples) * 255.0).round() as u8;
                    next[out_index + 1] =
                        (linear_to_srgb(color[1] / samples) * 255.0).round() as u8;
                    next[out_index + 2] =
                        (linear_to_srgb(color[2] / samples) * 255.0).round() as u8;
                }
                next[out_index + 3] = ((alpha / samples) * 255.0).round() as u8;
            }
        }
        levels.push((next_width, next_height, next));
        width = next_width;
        height = next_height;
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

fn create_gpu_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    data: &TextureData3D,
    normal_data: Option<&TextureData3D>,
) -> GpuTexture3D {
    let width = data.width.max(1);
    let height = data.height.max(1);
    let albedo_levels = build_mip_chain(data, false);
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
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    upload_mip_chain(queue, &texture, &albedo_levels);
    // A flat tangent-space normal is the identity perturbation.  Create it
    // lazily for ordinary albedo uploads; terrain can pass a real map through
    // `upload_texture_with_normal`.
    let flat_normal;
    let normal_data = if let Some(normal_data) = normal_data {
        normal_data
    } else {
        flat_normal = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![128, 128, 255, 255],
        };
        &flat_normal
    };
    let normal_width = normal_data.width.max(1);
    let normal_height = normal_data.height.max(1);
    let normal_levels = build_mip_chain(normal_data, true);
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
    upload_mip_chain(queue, &normal_texture, &normal_levels);
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
    moon_direction: vec4<f32>,
    moon_color: vec4<f32>,
    reflection_color: vec4<f32>,
    horizon_tint: vec4<f32>,
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,
    focus_position: vec4<f32>,
    viewport_size: vec4<f32>,
    local_light_meta: vec4<f32>,
    local_light_positions: array<vec4<f32>, 16>,
    local_light_colors: array<vec4<f32>, 16>,
};
struct Object {
    model: mat4x4<f32>,
    base_color: vec4<f32>,
    occlusion_window: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> object: Object;
@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;
@group(2) @binding(2) var normal_texture: texture_2d<f32>;
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

// Reconstruct a tangent frame from screen-space derivatives so meshes that
// only provide positions, normals and UVs (including the generated meadow)
// can still use a tangent-space normal map. A flat [0.5, 0.5, 1] sample falls
// back to the authored geometric normal without changing legacy materials.
fn normal_from_map(input: VertexOut) -> vec3<f32> {
    let base_normal = normalize(input.normal);
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
    let mapped = normalize(vec3<f32>(sampled.xy * 0.72, max(sampled.z, 0.28)));
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
    let count = u32(clamp(camera.local_light_meta.x, 0.0, 16.0));
    for (var index: u32 = 0u; index < 16u; index = index + 1u) {
        if (index >= count) {
            break;
        }
        let packed_position = camera.local_light_positions[index];
        let packed_color = camera.local_light_colors[index];
        let to_light = packed_position.xyz - world_position;
        let distance_squared = max(dot(to_light, to_light), 0.0001);
        let distance = sqrt(distance_squared);
        let radius = max(packed_position.w, 0.05);
        if (distance >= radius) {
            continue;
        }
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

fn aces_film(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + vec3<f32>(b)))
        / (color * (c * color + vec3<f32>(d)) + vec3<f32>(e)),
        vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    let normal = normal_from_map(input);
    let light_dir = normalize(camera.light_direction.xyz);
    let view_dir = normalize(camera.camera_position.xyz - input.world_position);
    // Authored floor receivers use the same sentinel as the shadow pass. They
    // remain fully key-lit so a bad/coplanar shadow sample cannot turn an
    // entire room floor black; contact AO still anchors props to the tiles.
    var visibility = shadow_visibility(input.light_clip, normal);
    if (object.occlusion_window.x < -0.5) {
        visibility = 1.0;
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
    if (object.base_color.a > 0.95 && albedo.a < 0.38) {
        discard;
    }
    let base = object.base_color.rgb * input.color.rgb * albedo.rgb;
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
    let moon = evaluate_brdf(
        base,
        normal,
        view_dir,
        moon_dir,
        camera.moon_color.rgb * camera.moon_color.a * 1.8,
        roughness,
    );
    let local = local_lighting(base, normal, view_dir, input.world_position, roughness);
    let rim = pow(1.0 - max(dot(normal, view_dir), 0.0), 3.0) * 0.065;
    let indirect = base * (sky + bounce + lunar_reflection + twilight) * ambient_ao;
    let lit_color = indirect + direct + moon + local + camera.ambient_color.rgb * rim;
    let distance_to_camera = length(camera.camera_position.xyz - input.world_position);
    let fog_span = max(camera.fog_params.y - camera.fog_params.x, 0.001);
    let linear_fog = clamp((distance_to_camera - camera.fog_params.x) / fog_span, 0.0, 1.0);
    let exponential_fog = 1.0 - exp(-distance_to_camera * max(camera.fog_params.z, 0.0));
    let height_fog = clamp(max(input.world_position.y - 0.15, 0.0) * camera.fog_params.w, 0.0, 1.0);
    let fog_factor = clamp(max(linear_fog, exponential_fog * 0.55) + linear_fog * height_fog * 0.35, 0.0, 1.0);
    let color = aces_film(max(mix(lit_color, camera.fog_color.rgb, fog_factor) * 1.12, vec3<f32>(0.0)));
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
            if alpha < 0.035 {
                discard;
            }
        }
    }
    return vec4<f32>(color, alpha);
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
    local_light_positions: array<vec4<f32>, 16>,
    local_light_colors: array<vec4<f32>, 16>,
};
struct Object {
    model: mat4x4<f32>,
    base_color: vec4<f32>,
    occlusion_window: vec4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<uniform> object: Object;
@group(2) @binding(0) var albedo_texture: texture_2d<f32>;
@group(2) @binding(1) var albedo_sampler: sampler;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct ShadowOut {
    @builtin(position) light_clip: vec4<f32>,
    @location(0) camera_clip: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_shadow(input: VertexIn) -> ShadowOut {
    let world = object.model * vec4<f32>(input.position, 1.0);
    var out: ShadowOut;
    out.light_clip = camera.light_view_projection * world;
    out.camera_clip = camera.view_projection * world;
    out.uv = input.uv;
    return out;
}

@fragment
fn fs_shadow(input: ShadowOut) {
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
