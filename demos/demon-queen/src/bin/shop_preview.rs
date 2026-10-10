//! Small GLB turntable composed into the existing UI batch, without another
//! surface, shadow pass or per-row renderer. Models/textures decode once.
//! Faces are backface culled, then opaque and translucent surfaces are depth composited.
//! Only the selected item is projected at the overlay's existing 30 Hz cadence.
use super::{MapVertex, Rect, Vec3};
use std::sync::OnceLock;
use velvet_render::GltfModel;

struct Vertex {
    p: Vec3,
    n: Vec3,
    c: [f32; 4],
    metallic: f32,
    roughness: f32,
    fluid: bool,
}
struct Preview {
    vertices: Vec<Vertex>,
    triangles: Vec<[usize; 3]>,
    /// The item's own colour, used to light its stage.
    accent: [f32; 3],
}

/// The most colourful average of the model's vertices, brightened to glow.
fn dominant_accent(vertices: &[Vertex]) -> [f32; 3] {
    let (mut sum, mut weight) = ([0.0f32; 3], 0.0f32);
    for v in vertices.iter().filter(|v| v.c[3] > 0.5) {
        let hi = v.c[0].max(v.c[1]).max(v.c[2]);
        let lo = v.c[0].min(v.c[1]).min(v.c[2]);
        let w = (hi - lo) + 0.04;
        for k in 0..3 {
            sum[k] += v.c[k] * w;
        }
        weight += w;
    }
    if weight <= 0.0 {
        return [0.55, 0.75, 0.70];
    }
    let mut c = [sum[0] / weight, sum[1] / weight, sum[2] / weight];
    let peak = c[0].max(c[1]).max(c[2]).max(0.001);
    for channel in &mut c {
        // Keep the hue, lift it so it reads as light and not as paint.
        *channel = (*channel / peak * 0.85).clamp(0.12, 1.0);
    }
    c
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LiquidMotion {
    pub(super) slope_x: f32,
    pub(super) slope_z: f32,
    pub(super) wave_strength: f32,
}

impl Default for LiquidMotion {
    fn default() -> Self {
        Self {
            slope_x: 0.0,
            slope_z: 0.0,
            wave_strength: 0.0015,
        }
    }
}

/// Small damped-spring liquid solver for the bottle preview.
/// Dragging the bottle adds angular impulses; gravity sets its resting surface.
#[derive(Default)]
pub(super) struct LiquidPhysics {
    active: bool,
    last_time: Option<f32>,
    last_rotation: Option<(f32, f32)>,
    slope_x: f32,
    slope_z: f32,
    velocity_x: f32,
    velocity_z: f32,
    wave_energy: f32,
}

impl LiquidPhysics {
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(super) fn update(&mut self, time: f32, rotation: Option<(f32, f32)>) -> LiquidMotion {
        let dt = self
            .last_time
            .map(|last| (time - last).clamp(0.0, 0.05))
            .unwrap_or(1.0 / 30.0);
        if !self.active {
            self.active = true;
            self.wave_energy = 0.24;
        }

        if let (Some((last_yaw, last_pitch)), Some((yaw, pitch))) = (self.last_rotation, rotation) {
            let delta_yaw = (yaw - last_yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            let delta_pitch = pitch - last_pitch;
            self.velocity_x = (self.velocity_x - delta_yaw * 3.0).clamp(-1.6, 1.6);
            self.velocity_z = (self.velocity_z - delta_pitch * 3.0).clamp(-1.6, 1.6);
            self.wave_energy =
                (self.wave_energy + (delta_yaw.abs() + delta_pitch.abs()) * 1.8).clamp(0.0, 1.0);
        }

        let (target_slope_x, target_slope_z) = rotation
            .map(|(yaw, pitch)| {
                let tilt = pitch.tan();
                (-yaw.sin() * tilt, yaw.cos() * tilt)
            })
            .unwrap_or((0.0, 0.0));
        let target_slope_x = target_slope_x.clamp(-0.42, 0.42);
        let target_slope_z = target_slope_z.clamp(-0.42, 0.42);
        let steps = ((dt / 0.016).ceil() as usize).clamp(1, 8);
        let step_dt = dt / steps as f32;
        for _ in 0..steps {
            let acceleration_x = (target_slope_x - self.slope_x) * 32.0 - self.velocity_x * 8.0;
            let acceleration_z = (target_slope_z - self.slope_z) * 32.0 - self.velocity_z * 8.0;
            self.velocity_x += acceleration_x * step_dt;
            self.velocity_z += acceleration_z * step_dt;
            self.slope_x += self.velocity_x * step_dt;
            self.slope_z += self.velocity_z * step_dt;
        }
        self.wave_energy *= (-1.5 * dt).exp();
        self.last_time = Some(time);
        self.last_rotation = rotation;

        LiquidMotion {
            slope_x: self.slope_x.clamp(-0.42, 0.42),
            slope_z: self.slope_z.clamp(-0.42, 0.42),
            wave_strength: 0.0015 + self.wave_energy * 0.032,
        }
    }
}

// The source liquid mesh nearly touches the bottle's inner wall at the lower
// belly. Keep a small radial clearance so animation and painter sorting never
// make the liquid look as if it leaks through the glass.
const LIQUID_RADIAL_SCALE: f32 = 0.87;
#[cfg(test)]
const LIQUID_GLASS_CLEARANCE: f32 = 0.01;
const LIQUID_FLOOR_Y: f32 = -0.425;

fn is_liquid_color(color: [f32; 4]) -> bool {
    color[0] > 0.4 && color[1] < 0.04 && color[2] < 0.08
}

fn preview_asset_reference(reference: &str) -> (&str, bool) {
    let Some((asset, query)) = reference.split_once('?') else {
        return (reference, false);
    };
    let pink_liquid = query.split('&').any(|variant| variant == "liquid=pink");
    (asset, pink_liquid)
}

pub(super) fn has_animated_potion_liquid(reference: &str) -> bool {
    preview_asset_reference(reference)
        .0
        .ends_with("health_potion.glb")
}

fn tint_liquid_pink(color: [f32; 4]) -> [f32; 4] {
    let intensity = color[0].max(color[1]).max(color[2]);
    [
        intensity * 0.82,
        intensity * 0.10,
        intensity * 0.45,
        color[3],
    ]
}

fn inset_liquid(position: Vec3) -> Vec3 {
    Vec3::new(
        position.x * LIQUID_RADIAL_SCALE,
        position.y.max(LIQUID_FLOOR_Y),
        position.z * LIQUID_RADIAL_SCALE,
    )
}

/// Material response used by the shop's painter-style 3D preview.
fn shade_color(
    base: [f32; 4],
    normal: Vec3,
    metallic: f32,
    roughness: f32,
    liquid: bool,
) -> [f32; 4] {
    let n = normal.normalize_or_zero();
    let metallic = metallic.clamp(0.0, 1.0);
    let roughness = roughness.clamp(0.08, 1.0);
    // A warm upper-left key plus a broad cool front fill keeps the hand-painted
    // palette vivid while lifting details that the old single light crushed.
    let key_dir = Vec3::new(-0.43, 0.72, 0.55).normalize_or_zero();
    let fill_dir = Vec3::new(0.58, 0.18, 0.79).normalize_or_zero();
    let key = n.dot(key_dir).max(0.0);
    let fill = n.dot(fill_dir).max(0.0);
    let toon = velvet_render::ToonStyle3D::default();
    let band = 0.39 + toon.diffuse(key) * 0.46 + toon.diffuse(fill) * 0.21;
    let view = Vec3::new(0.0, 0.0, 1.0);
    let key_half = (key_dir + view).normalize_or_zero();
    let fill_half = (fill_dir + view).normalize_or_zero();
    let key_reflection = n.dot(key_half).max(0.0);
    let fill_reflection = n.dot(fill_half).max(0.0);
    let key_glint = toon.edge(0.90 + roughness * 0.075, key_reflection)
        * (0.08 + metallic * 0.34)
        * (1.0 - roughness * 0.5);
    let fill_glint =
        toon.edge(0.94 + roughness * 0.04, fill_reflection) * (0.035 + metallic * 0.12);
    let edge = (1.0 - n.z.abs()).powi(3);
    // Sky above and warm bounce from the turntable below give volume where the
    // key and fill do not reach; two rims (warm on the key side, cool opposite)
    // separate the silhouette from the stage.
    let sky = n.y.max(0.0);
    let ground = (-n.y).max(0.0);
    let rim_warm = edge * (0.30 + 0.70 * key);
    let rim_cool = edge * (1.0 - key);
    let mut color = base;
    for (index, channel) in color[..3].iter_mut().enumerate() {
        let warm_tint = [1.04, 1.00, 0.94][index]
            * (toon.shadow_tint[index] + (1.0 - toon.shadow_tint[index]) * toon.diffuse(key));
        let key_spec = [1.0, 0.87, 0.68][index];
        let fill_spec = [0.68, 0.86, 1.0][index];
        let rim_tint = [0.025, 0.055, 0.085][index];
        let reflected_color = *channel * metallic + (1.0 - metallic);
        let hemi = sky * [0.030, 0.045, 0.060][index] + ground * [0.050, 0.032, 0.018][index];
        let rim = rim_warm * [0.120, 0.085, 0.040][index] + rim_cool * [0.030, 0.075, 0.130][index];
        *channel = (*channel * band * warm_tint * (1.0 - metallic * 0.34)
            + *channel * hemi * 2.2
            + reflected_color * (key_glint * key_spec + fill_glint * fill_spec)
            + edge * rim_tint
            + rim)
            .clamp(0.0, 1.0);
    }

    if base[3] < 0.9 {
        // Two studio cards create readable cool and warm glass reflections.
        // Depth ordering below still lets opaque liquid hide rear glass.
        let coolbox = (1.0 - ((n.x + 0.43) / 0.22).powi(2)).max(0.0)
            * (1.0 - ((n.y - 0.27) / 0.72).powi(4)).max(0.0)
            * n.z.max(0.0);
        let warmbox = (1.0 - ((n.x - 0.48) / 0.30).powi(2)).max(0.0)
            * (1.0 - ((n.y - 0.10) / 0.90).powi(4)).max(0.0)
            * n.z.max(0.0);
        let rim = base[3] > 0.6;
        color = [
            0.74 + coolbox * 0.19 + warmbox * 0.13,
            0.84 + coolbox * 0.13 + warmbox * 0.08,
            0.89 + coolbox * 0.10 + warmbox * 0.025,
            ((if rim { 0.10 } else { 0.025 }) + edge * 0.53 + coolbox * 0.64 + warmbox * 0.20)
                .min(0.82),
        ];
    } else if liquid {
        // Keep potion liquids opaque and vivid, with highlights matching their hue.
        let shine = toon.edge(0.95, (n.x * -0.36 + n.y * 0.42 + n.z * 0.83).max(0.0));
        let body = (0.28 + toon.diffuse(key) * 0.50 + toon.diffuse(fill) * 0.15).clamp(0.28, 0.96);
        if base[2] > base[0] * 0.24 {
            color = [
                (base[0] * body + shine * 0.42).min(1.0),
                (base[1] * body + shine * 0.14).min(1.0),
                (base[2] * body + shine * 0.30).min(1.0),
                1.0,
            ];
        } else {
            color = [
                (base[0] * body + shine * 0.45).min(1.0),
                (base[1] * body + shine * 0.075).min(1.0),
                (base[2] * body + shine * 0.055).min(1.0),
                1.0,
            ];
        }
    }
    color
}

fn sort_back_to_front(triangles: &mut [(bool, f32, [usize; 3])]) {
    // Blend every surface in depth order, not in material groups. Opaque water
    // then covers rear glass; a glass face in front is still composited last.
    triangles.sort_unstable_by(|a, b| {
        a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)) // coincident glass first, opaque surface last
    });
}

fn load(path: &str) -> Option<Preview> {
    let (asset_reference, pink_liquid) = preview_asset_reference(path);
    let has_liquid_physics = asset_reference.ends_with("health_potion.glb");
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join(asset_reference)))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| root.join(asset_reference));
    let model = GltfModel::load(&path)
        .map_err(|e| eprintln!("[shop-preview] {}: {e}", path.display()))
        .ok()?;
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (mi, mesh) in model.meshes.iter().enumerate() {
        let base = vertices.len();
        let transform = model.mesh_transform(mi);
        let fluid_mesh = has_liquid_physics && is_liquid_color(mesh.material.base_color);
        for v in &mesh.vertices {
            let mut c = mesh.material.base_color;
            for k in 0..4 {
                c[k] *= v.color[k];
            }
            if let Some(texture) = mesh.texture_index.and_then(|i| model.textures.get(i)) {
                let x = (v.uv[0].rem_euclid(1.0) * texture.width as f32) as usize;
                let y = (v.uv[1].rem_euclid(1.0) * texture.height as f32) as usize;
                let pixel = (y * texture.width as usize + x) * 4;
                for k in 0..3 {
                    c[k] *= (texture.rgba[pixel + k] as f32 / 255.0).powf(2.2);
                }
            }
            if pink_liquid && fluid_mesh {
                c = tint_liquid_pink(c);
            }
            let p =
                transform.transform_point3(Vec3::new(v.position[0], v.position[1], v.position[2]));
            vertices.push(Vertex {
                p,
                n: transform
                    .transform_vector3(Vec3::new(v.normal[0], v.normal[1], v.normal[2]))
                    .normalize_or_zero(),
                c,
                metallic: mesh.metallic_factor,
                roughness: mesh.roughness_factor,
                fluid: fluid_mesh,
            });
        }
        for t in mesh.indices.chunks_exact(3) {
            triangles.push([
                base + t[0] as usize,
                base + t[1] as usize,
                base + t[2] as usize,
            ]);
        }
    }
    let minimum = Vec3::new(
        vertices.iter().map(|v| v.p.x).fold(f32::INFINITY, f32::min),
        vertices.iter().map(|v| v.p.y).fold(f32::INFINITY, f32::min),
        vertices.iter().map(|v| v.p.z).fold(f32::INFINITY, f32::min),
    );
    let maximum = Vec3::new(
        vertices
            .iter()
            .map(|v| v.p.x)
            .fold(f32::NEG_INFINITY, f32::max),
        vertices
            .iter()
            .map(|v| v.p.y)
            .fold(f32::NEG_INFINITY, f32::max),
        vertices
            .iter()
            .map(|v| v.p.z)
            .fold(f32::NEG_INFINITY, f32::max),
    );
    let center = (minimum + maximum) * 0.5;
    let extent = maximum - minimum;
    let size = extent.x.max(extent.y).max(extent.z).max(0.001);
    for v in &mut vertices {
        v.p = Vec3::new(
            (v.p.x - center.x) / size,
            (v.p.y - center.y) / size,
            (v.p.z - center.z) / size,
        );
        if v.fluid {
            v.p = inset_liquid(v.p);
        }
    }
    let accent = dominant_accent(&vertices);
    Some(Preview {
        vertices,
        triangles,
        accent,
    })
}

fn previews() -> &'static std::collections::HashMap<String, Option<Preview>> {
    // Catalog is immutable for this process; cache failures too (icon fallback). Besides the
    // shop's goods, Rosalía's elixir turns on her offer's stage.
    static CACHE: OnceLock<std::collections::HashMap<String, Option<Preview>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        super::super::shop::catalog()
            .products
            .iter()
            .filter_map(|p| p.preview_model.clone())
            .chain(std::iter::once(
                super::venue_overlay::ELIXIR_MODEL.to_owned(),
            ))
            .map(|p| {
                let preview = load(&p);
                (p, preview)
            })
            .collect()
    })
}

/// The stage colour of a previewed model, if it has one.
pub(super) fn accent(path: Option<&str>) -> Option<[f32; 3]> {
    previews().get(path?)?.as_ref().map(|m| m.accent)
}

fn fan_ellipse(out: &mut Vec<MapVertex>, cx: f32, cy: f32, rx: f32, ry: f32, color: [f32; 4]) {
    const N: usize = 48;
    for j in 0..N {
        let a = j as f32 * std::f32::consts::TAU / N as f32;
        let b = (j + 1) as f32 * std::f32::consts::TAU / N as f32;
        super::push_triangle(
            out,
            [
                [cx, cy],
                [cx + a.cos() * rx, cy + a.sin() * ry],
                [cx + b.cos() * rx, cy + b.sin() * ry],
            ],
            color,
        );
    }
}

fn ring_ellipse(
    out: &mut Vec<MapVertex>,
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    width: f32,
    color: [f32; 4],
) {
    const N: usize = 64;
    for j in 0..N {
        let a = j as f32 * std::f32::consts::TAU / N as f32;
        let b = (j + 1) as f32 * std::f32::consts::TAU / N as f32;
        let p = |t: f32, k: f32| [cx + t.cos() * (rx + k), cy + t.sin() * (ry + k * ry / rx)];
        super::push_triangle(
            out,
            [p(a, -width * 0.5), p(a, width * 0.5), p(b, width * 0.5)],
            color,
        );
        super::push_triangle(
            out,
            [p(a, -width * 0.5), p(b, width * 0.5), p(b, -width * 0.5)],
            color,
        );
    }
}

fn gradient_quad(out: &mut Vec<MapVertex>, r: Rect, top: [f32; 4], bottom: [f32; 4]) {
    let v = |x: f32, y: f32, color: [f32; 4]| MapVertex {
        position: [x, y],
        uv: [0.0; 2],
        color,
        text: 0.0,
        ..MapVertex::BLANK
    };
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    for (x, y, c) in [
        (x0, y0, top),
        (x1, y0, top),
        (x1, y1, bottom),
        (x0, y0, top),
        (x1, y1, bottom),
        (x0, y1, bottom),
    ] {
        out.push(v(x, y, c));
    }
}

/// A studio for the item: a pool of its own colour behind it, a beam from
/// above, a lit turntable under it, drifting motes and a soft vignette. All
/// procedural, drawn before the model.
pub(super) fn backdrop(out: &mut Vec<MapVertex>, r: Rect, accent: [f32; 3], time: f32) {
    let t = if std::env::var_os("VELVET_REDUCED_MOTION").is_some() {
        0.0
    } else {
        time
    };
    let cx = r.x + r.w * 0.5;
    let glow = |a: f32| [accent[0], accent[1], accent[2], a];
    // A pool of coloured light behind the item, layered so it falls off softly.
    let gy = r.y + r.h * 0.46;
    for i in 0..10 {
        let k = i as f32 / 10.0;
        fan_ellipse(
            out,
            cx,
            gy,
            r.w * (0.56 - 0.44 * k),
            r.h * (0.50 - 0.38 * k),
            glow(0.026 + 0.010 * k),
        );
    }
    // A cool beam from above.
    let beam_top = [0.85, 0.95, 1.0, 0.075];
    let beam_bottom = [0.85, 0.95, 1.0, 0.0];
    let (bx0, bx1) = (cx - r.w * 0.11, cx + r.w * 0.11);
    let (bx2, bx3) = (cx + r.w * 0.36, cx - r.w * 0.36);
    let by = r.y + r.h * 0.92;
    for (x, y, c) in [
        (bx0, r.y, beam_top),
        (bx1, r.y, beam_top),
        (bx2, by, beam_bottom),
        (bx0, r.y, beam_top),
        (bx2, by, beam_bottom),
        (bx3, by, beam_bottom),
    ] {
        out.push(MapVertex {
            position: [x, y],
            uv: [0.0; 2],
            color: c,
            text: 0.0,
            ..MapVertex::BLANK
        });
    }
    // The turntable: dark disc, a pool of the item's light on it, a lit rim.
    let fy = r.y + r.h * 0.91;
    fan_ellipse(out, cx, fy + 5.0, r.w * 0.41, 20.0, [0.0, 0.0, 0.0, 0.34]);
    fan_ellipse(out, cx, fy, r.w * 0.385, 17.0, [0.05, 0.11, 0.12, 1.0]);
    fan_ellipse(out, cx, fy - 1.0, r.w * 0.30, 13.0, [0.10, 0.20, 0.21, 1.0]);
    for i in 0..5 {
        let k = i as f32 / 5.0;
        fan_ellipse(
            out,
            cx,
            fy - 1.0,
            r.w * (0.29 - 0.22 * k),
            12.0 * (1.0 - 0.7 * k),
            glow(0.05 + 0.03 * k),
        );
    }
    ring_ellipse(
        out,
        cx,
        fy,
        r.w * 0.385,
        17.0,
        2.0,
        [0.72, 0.86, 0.84, 0.42],
    );
    ring_ellipse(out, cx, fy + 1.0, r.w * 0.30, 13.0, 1.0, glow(0.55));
    // Motes rising through the beam.
    for k in 0..18 {
        let seed = k as f32 * 12.9898;
        let fx = (seed.sin() * 43758.547).fract().abs();
        let fs = ((seed * 1.7).sin() * 24634.635).fract().abs();
        let rise = (fs * 1.0 + t * (0.018 + 0.03 * fs)).fract();
        let x = r.x + r.w * (0.14 + 0.72 * fx) + (t * 0.8 + seed).sin() * 4.0;
        let y = r.y + r.h * (0.94 - rise * 0.86);
        let a =
            (0.10 + 0.30 * (1.0 - (rise * 2.0 - 1.0).abs())) * (0.6 + 0.4 * (t * 2.3 + seed).sin());
        let size = 1.4 + 1.6 * fs;
        super::push_triangle(
            out,
            [[x - size, y], [x, y - size], [x + size, y]],
            [0.95, 0.98, 1.0, a],
        );
        super::push_triangle(
            out,
            [[x - size, y], [x + size, y], [x, y + size]],
            [0.95, 0.98, 1.0, a],
        );
    }
    // Vignette so the frame sits back from the object.
    let dark = [0.0, 0.02, 0.03, 0.36];
    let clear = [0.0, 0.02, 0.03, 0.0];
    gradient_quad(out, Rect::new(r.x, r.y, r.w, r.h * 0.16), dark, clear);
    gradient_quad(
        out,
        Rect::new(r.x, r.y + r.h * 0.84, r.w, r.h * 0.16),
        clear,
        dark,
    );
    for (x, w) in [(r.x, r.w * 0.14), (r.x + r.w * 0.86, r.w * 0.14)] {
        let v = |px: f32, py: f32, c: [f32; 4]| MapVertex {
            position: [px, py],
            uv: [0.0; 2],
            color: c,
            text: 0.0,
            ..MapVertex::BLANK
        };
        let (l, rr) = if x == r.x {
            (dark, clear)
        } else {
            (clear, dark)
        };
        for (px, py, c) in [
            (x, r.y, l),
            (x + w, r.y, rr),
            (x + w, r.y + r.h, rr),
            (x, r.y, l),
            (x + w, r.y + r.h, rr),
            (x, r.y + r.h, l),
        ] {
            out.push(v(px, py, c));
        }
    }
}

pub(super) fn draw(
    out: &mut Vec<MapVertex>,
    path: Option<&str>,
    rect: Rect,
    time: f32,
    manual_rotation: Option<(f32, f32)>,
    liquid_motion: LiquidMotion,
) -> bool {
    let Some(path) = path else { return false };
    let Some(Some(model)) = previews().get(path) else {
        return false;
    };
    let (yaw, pitch) = display_rotation(time, manual_rotation);
    let t = if std::env::var_os("VELVET_REDUCED_MOTION").is_some() {
        0.0
    } else {
        time
    };
    let (sn, cs) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let rotate = |p: Vec3| {
        let x = p.x * cs + p.z * sn;
        let z = -p.x * sn + p.z * cs;
        Vec3::new(x, p.y * cp - z * sp, p.y * sp + z * cp)
    };
    let scale = rect.h * 0.81;
    let cy = rect.y + rect.h * 0.49 + (t * 1.7).sin() * 3.0;
    // Layered soft contact shadow and a faint warm bounce replace the old hard
    // black ellipse. They stay grounded while the model gently floats above.
    for (width, height, color) in [
        (0.30, 10.0, [0.32, 0.20, 0.08, 0.10]),
        (0.24, 7.0, [0.015, 0.045, 0.045, 0.15]),
        (0.17, 4.0, [0.005, 0.018, 0.017, 0.20]),
    ] {
        for j in 0..40 {
            let a = j as f32 * std::f32::consts::TAU / 40.0;
            let b = (j + 1) as f32 * std::f32::consts::TAU / 40.0;
            let cx = rect.x + rect.w * 0.5;
            let y = rect.y + rect.h * 0.91;
            super::push_triangle(
                out,
                [
                    [cx, y],
                    [cx + a.cos() * rect.w * width, y + a.sin() * height],
                    [cx + b.cos() * rect.w * width, y + b.sin() * height],
                ],
                color,
            );
        }
    }
    let projected: Vec<_> = model
        .vertices
        .iter()
        .map(|v| {
            let (local, normal) = if v.fluid {
                (
                    animate_liquid(v.p, t, liquid_motion),
                    animate_liquid_normal(v.p, v.n, t, liquid_motion),
                )
            } else {
                (v.p, v.n)
            };
            let p = rotate(local);
            let n = rotate(normal).normalize_or_zero();
            (p, shade_color(v.c, n, v.metallic, v.roughness, v.fluid))
        })
        .collect();
    let mut sorted: Vec<_> = model
        .triangles
        .iter()
        .filter_map(|tri| {
            let a = projected[tri[0]].0;
            let b = projected[tri[1]].0;
            let c = projected[tri[2]].0;
            let winding = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            if winding <= 0.0 {
                return None;
            }
            Some((projected[tri[0]].1[3] < 0.9, (a.z + b.z + c.z) / 3.0, *tri))
        })
        .collect();
    sort_back_to_front(&mut sorted);
    for (_, _, tri) in sorted {
        for i in tri {
            let (p, c) = projected[i];
            out.push(MapVertex {
                position: [rect.x + rect.w * 0.5 + p.x * scale, cy - p.y * scale],
                uv: [0.0; 2],
                color: c,
                text: 0.0,
                ..MapVertex::BLANK
            });
        }
    }
    true
}

pub(super) fn display_rotation(time: f32, manual_rotation: Option<(f32, f32)>) -> (f32, f32) {
    let t = if std::env::var_os("VELVET_REDUCED_MOTION").is_some() {
        0.0
    } else {
        time
    };
    manual_rotation.unwrap_or((t * 0.35, 0.24))
}

fn animate_liquid(position: Vec3, time: f32, motion: LiquidMotion) -> Vec3 {
    let top = ((position.y + 0.16) / 0.16).clamp(0.0, 1.0);
    let top = top * top * (3.0 - 2.0 * top);
    let swell = (time * 5.2 + position.x * 17.0 + position.z * 12.0).sin() * 0.64
        + (time * 3.7 - position.x * 10.0 + position.z * 16.0 + 0.8).sin() * 0.36;
    let tilt = (position.x * motion.slope_x + position.z * motion.slope_z).clamp(-0.10, 0.10);
    Vec3::new(
        position.x,
        position.y + (tilt + swell * motion.wave_strength) * top,
        position.z,
    )
}

fn animate_liquid_normal(position: Vec3, normal: Vec3, time: f32, motion: LiquidMotion) -> Vec3 {
    if normal.y < 0.5 {
        return normal;
    }
    let top = ((position.y + 0.16) / 0.16).clamp(0.0, 1.0);
    let top = top * top * (3.0 - 2.0 * top);
    let phase_a = time * 5.2 + position.x * 17.0 + position.z * 12.0;
    let phase_b = time * 3.7 - position.x * 10.0 + position.z * 16.0 + 0.8;
    let slope_x = motion.slope_x
        + motion.wave_strength * (0.64 * 17.0 * phase_a.cos() - 0.36 * 10.0 * phase_b.cos());
    let slope_z = motion.slope_z
        + motion.wave_strength * (0.64 * 12.0 * phase_a.cos() + 0.36 * 16.0 * phase_b.cos());
    let surface = Vec3::new(-slope_x, 1.0, -slope_z).normalize_or_zero();
    normal.lerp(surface, top).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_catalog_item_has_a_renderable_svg() {
        let products = &super::super::super::shop::catalog().products;
        assert_eq!(products.len(), 22);
        let mut ids = std::collections::HashSet::new();
        for p in products {
            assert!(ids.insert(&p.id));
            assert!(p.queen_thought.as_ref().is_some_and(|t| !t.is_empty()));
            let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets/ui/shop")
                .join(p.icon_svg.as_ref().unwrap());
            let svg = std::fs::read_to_string(path).unwrap();
            let img = velvet_image::rasterize_simple_svg(&svg, 128, 128).unwrap();
            assert!(
                img.pixels.chunks_exact(4).any(|p| p[3] > 0),
                "{} empty icon",
                p.id
            );
        }
    }

    /// Share of opaque pixels and count of distinct colours of an icon.
    fn icon_stats(svg: &str) -> (f32, usize) {
        let img = velvet_image::rasterize_simple_svg(svg, 128, 128).unwrap();
        let mut colours = std::collections::HashSet::new();
        let mut opaque = 0usize;
        for p in img.pixels.chunks_exact(4).filter(|p| p[3] > 200) {
            opaque += 1;
            colours.insert([p[0] >> 3, p[1] >> 3, p[2] >> 3]);
        }
        (opaque as f32 / (128.0 * 128.0), colours.len())
    }

    #[test]
    fn every_shop_item_is_drawn_as_rich_vector_art() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/ui/shop");
        for product in super::super::super::shop::catalog().products.iter() {
            let file = product
                .icon_svg
                .as_deref()
                .unwrap_or_else(|| panic!("no vector icon for {}", product.id));
            let svg =
                std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert!(svg.contains("<path"), "{file} has no shapes");
            assert!(!svg.contains("<image"), "{file} embeds a picture");
            let (coverage, colours) = icon_stats(&svg);
            assert!(
                (0.10..0.80).contains(&coverage),
                "{file} covers {coverage:.2} of its canvas"
            );
            assert!(colours >= 3, "{file} is too flat ({colours} colours)");
            // The shop must never fall back to raster art for these.
            if let Some(image) = product.icon_image.as_deref() {
                assert!(
                    image.ends_with(".png"),
                    "{}: raster leftovers must stay optional",
                    product.id
                );
            }
        }
    }

    #[test]
    fn crafted_items_and_the_relic_sword_have_vector_icons_too() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/ui/shop");
        let mut files: Vec<&str> = super::super::super::backpack::EXTRAS
            .iter()
            .map(|e| e.icon_svg)
            .collect();
        files.push("item_stone_sword.svg");
        for file in files {
            let svg =
                std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
            let (coverage, colours) = icon_stats(&svg);
            assert!(
                (0.10..0.80).contains(&coverage) && colours >= 3,
                "{file}: {coverage:.2}, {colours}"
            );
        }
    }

    #[test]
    fn new_shop_models_are_linked_and_loadable() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let expected = [
            ("pan_del_dia", "bread_loaf.glb"),
            ("anillo_misterioso", "fantasy_ring.glb"),
            ("pocion_misteriosa", "health_potion.glb"),
            ("tinta_runica", "runic_ink.glb"),
            ("llave_boveda", "vault_key.glb"),
            ("esencia_carmesi", "crimson_essence.glb"),
        ];
        let products = &super::super::super::shop::catalog().products;

        for (id, filename) in expected {
            let product = products
                .iter()
                .find(|product| product.id == id)
                .unwrap_or_else(|| panic!("catalog item missing: {id}"));
            let path = product
                .preview_model
                .as_deref()
                .unwrap_or_else(|| panic!("preview model missing for {id}"));
            let (asset_reference, _) = preview_asset_reference(path);
            assert!(asset_reference.ends_with(filename), "{id} points to {path}");
            assert!(
                root.join(asset_reference).is_file(),
                "missing model: {path}"
            );

            let model = load(path).unwrap_or_else(|| panic!("could not load {path}"));
            assert!(!model.vertices.is_empty(), "empty model: {path}");
            assert!(!model.triangles.is_empty(), "no triangles in {path}");
            assert!(model.vertices.iter().all(|vertex| vertex.p.x.is_finite()
                && vertex.p.y.is_finite()
                && vertex.p.z.is_finite()));
            let min = model.vertices.iter().fold(
                Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
                |acc, vertex| {
                    Vec3::new(
                        acc.x.min(vertex.p.x),
                        acc.y.min(vertex.p.y),
                        acc.z.min(vertex.p.z),
                    )
                },
            );
            let max = model.vertices.iter().fold(
                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
                |acc, vertex| {
                    Vec3::new(
                        acc.x.max(vertex.p.x),
                        acc.y.max(vertex.p.y),
                        acc.z.max(vertex.p.z),
                    )
                },
            );
            let extent = max - min;
            assert!(
                extent.x.max(extent.y).max(extent.z) <= 1.001,
                "oversized {path}"
            );
            assert!(model.triangles.len() <= 12_000, "model too dense: {path}");
        }
        let mystery = products
            .iter()
            .find(|product| product.id == "pocion_misteriosa")
            .and_then(|product| product.preview_model.as_deref())
            .unwrap();
        assert!(mystery.ends_with("?liquid=pink"));
        assert!(has_animated_potion_liquid(mystery));
    }
    #[test]
    fn redesigned_shop_hit_targets() {
        use super::super::super::shop::Tab;
        use super::super::{shop_hit_test, shop_pick, ShopHit};
        let inventory = Default::default();
        let pick = shop_pick(
            Tab::Buy,
            super::super::super::shop::Category::All,
            0,
            &inventory,
            0,
            0,
            0,
        );
        let hit = |x, y| shop_hit_test(x, y, &pick);
        assert!(matches!(hit(700.0, 287.0), Some(ShopHit::Item(0))));
        assert!(matches!(hit(1414.0, 845.0), Some(ShopHit::QuantityMinus)));
        assert!(matches!(hit(1566.0, 845.0), Some(ShopHit::QuantityPlus)));
        assert!(matches!(hit(1393.0, 941.0), Some(ShopHit::Action)));
        assert!(matches!(hit(1719.0, 941.0), Some(ShopHit::Haggle)));
        assert!(matches!(hit(1818.0, 147.0), Some(ShopHit::Close)));
    }
    #[test]
    #[ignore = "GPU screenshot: run explicitly when reviewing shop presentation"]
    fn capture_shop_preview() {
        let instance = wgpu::Instance::default();
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .unwrap();
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .unwrap();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut overlay =
            super::super::MapOverlay::new(&device, &queue, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut venue = super::super::super::venue::Menu::default();
        let venue_tab = std::env::var("DEMON_QUEEN_VENUE_PREVIEW")
            .ok()
            .and_then(|v| v.parse::<usize>().ok());
        if let Some(tab) = venue_tab {
            venue.open = true;
            venue.select_tab(tab);
        }
        use super::super::super::shop::{Category, Tab};
        let scenario = std::env::var("DEMON_QUEEN_SHOP_PREVIEW").unwrap_or_else(|_| "buy".into());
        let (tab, category, selection, gold, quantity) = match scenario.as_str() {
            "page2" => (Tab::Buy, Category::All, 8, 245, 1),
            "long" => (Tab::Buy, Category::All, 9, 245, 1),
            "bread" => (Tab::Buy, Category::All, 2, 245, 1),
            "mystery" => (Tab::Buy, Category::All, 5, 245, 1),
            "crimson" => (Tab::Buy, Category::All, 6, 245, 1),
            "ink" => (Tab::Buy, Category::All, 7, 245, 1),
            "ring" => (Tab::Buy, Category::All, 8, 245, 1),
            "key" => (Tab::Buy, Category::All, 10, 245, 1),
            "sell-empty" => (Tab::Sell, Category::All, 0, 245, 1),
            "sell" => (Tab::Sell, Category::All, 0, 245, 2),
            "filtered" => (Tab::Buy, Category::Materials, 0, 245, 1),
            "unaffordable" => (Tab::Buy, Category::All, 10, 20, 1),
            _ => (Tab::Buy, Category::All, 0, 245, 1),
        };
        let inventory = if scenario == "sell" {
            [
                ("pocion_pequena".to_owned(), 3),
                ("pan_del_dia".to_owned(), 2),
            ]
            .into_iter()
            .collect()
        } else {
            Default::default()
        };
        overlay.draw(
            &device,
            &queue,
            &texture.create_view(&Default::default()),
            (1920, 1080),
            Vec3::new(0.0, 0.0, 0.0),
            0.0,
            false,
            true,
            false,
            false,
            0,
            0,
            true,
            tab,
            category,
            selection,
            quantity,
            gold,
            &inventory,
            &Default::default(),
            None,
            None,
            2.0,
            None,
            &[],
            Some(&venue),
            None,
            None,
            false,
        );
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1920 * 1080 * 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(1920 * 4),
                    rows_per_image: Some(1080),
                },
            },
            wgpu::Extent3d {
                width: 1920,
                height: 1080,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.unwrap());
        device.poll(wgpu::Maintain::Wait);
        let data = slice.get_mapped_range();
        image::save_buffer(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                venue_tab
                    .map(|t| format!("venue-preview-{t}.png"))
                    .unwrap_or_else(|| format!("shop-redesign-{scenario}.png")),
            ),
            &data,
            1920,
            1080,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    #[test]
    fn potion_is_bounded_and_animated() {
        let path = Some("data/assets/3d/items/health_potion.glb");
        let r = Rect::new(0.0, 0.0, 270.0, 270.0);
        let mut a = Vec::new();
        let mut b = Vec::new();
        assert!(draw(&mut a, path, r, 0.0, None, LiquidMotion::default()));
        assert!(draw(&mut b, path, r, 3.0, None, LiquidMotion::default()));
        assert!(!a.is_empty() && a.len() < 15000);
        assert!(a.iter().all(|v| {
            v.position
                .iter()
                .all(|x| x.is_finite() && *x >= 0.0 && *x <= 270.0)
        }));
        assert!(a.iter().zip(&b).any(|(a, b)| a.position != b.position));
        assert!(!draw(
            &mut Vec::new(),
            None,
            r,
            0.0,
            None,
            LiquidMotion::default()
        ));
    }

    #[test]
    fn potion_materials_restore_previous_look_and_keep_liquid_opaque() {
        let body_glass = shade_color(
            [0.68, 0.81, 0.88, 0.32],
            Vec3::new(0.0, 0.0, 1.0),
            0.0,
            0.5,
            false,
        );
        let edge_glass = shade_color(
            [0.68, 0.81, 0.88, 0.32],
            Vec3::new(1.0, 0.0, 0.0),
            0.0,
            0.5,
            false,
        );
        let rim_glass = shade_color(
            [0.72, 0.86, 0.92, 0.82],
            Vec3::new(1.0, 0.0, 0.0),
            0.0,
            0.5,
            false,
        );
        let liquid = shade_color(
            [0.65, 0.012, 0.025, 1.0],
            Vec3::new(0.0, 0.0, 1.0),
            0.0,
            0.5,
            true,
        );

        assert!(body_glass[3] < 0.05);
        assert!(edge_glass[3] > 0.5);
        assert!(rim_glass[3] > 0.6);
        assert_eq!(liquid[3], 1.0);
        assert!(liquid[0] > liquid[1] * 8.0);
    }

    #[test]
    fn material_lighting_respects_metal_and_roughness() {
        let normal = Vec3::new(-0.43, 0.72, 0.55).normalize_or_zero();
        let matte = shade_color([0.55, 0.42, 0.22, 1.0], normal, 0.0, 0.95, false);
        let polished_metal = shade_color([0.55, 0.42, 0.22, 1.0], normal, 0.9, 0.12, false);
        assert!(matte
            .iter()
            .all(|channel| channel.is_finite() && *channel <= 1.0));
        assert!(polished_metal
            .iter()
            .all(|channel| channel.is_finite() && *channel <= 1.0));
        assert_ne!(matte, polished_metal);
    }

    #[test]
    fn potion_liquid_stays_inside_the_glass_with_clearance() {
        let preview = load("data/assets/3d/items/health_potion.glb").unwrap();
        let liquid: Vec<_> = preview
            .vertices
            .iter()
            .filter(|vertex| is_liquid_color(vertex.c))
            .collect();
        let glass: Vec<_> = preview
            .vertices
            .iter()
            .filter(|vertex| vertex.c[3] < 0.9 && vertex.c[2] > vertex.c[0])
            .collect();
        assert!(!liquid.is_empty(), "potion liquid mesh missing");
        assert!(!glass.is_empty(), "bottle glass mesh missing");

        for water in liquid {
            let radius = water.p.x.hypot(water.p.z);
            let inner_wall_radius = glass
                .iter()
                .filter(|wall| (wall.p.y - water.p.y).abs() <= 0.02)
                .map(|wall| wall.p.x.hypot(wall.p.z))
                .filter(|wall_radius| *wall_radius > 0.12)
                .min_by(f32::total_cmp)
                .unwrap_or_else(|| panic!("no glass wall sampled near y={}", water.p.y));
            assert!(
                radius + LIQUID_GLASS_CLEARANCE < inner_wall_radius,
                "liquid intersects glass at y={}: radius={}, inner wall={}",
                water.p.y,
                radius,
                inner_wall_radius
            );
        }
    }

    #[test]
    fn mysterious_potion_reuses_the_bottle_and_only_tints_the_liquid() {
        let red = load("data/assets/3d/items/health_potion.glb").unwrap();
        let pink = load("data/assets/3d/items/health_potion.glb?liquid=pink").unwrap();
        assert_eq!(red.triangles, pink.triangles);
        assert_eq!(red.vertices.len(), pink.vertices.len());
        let mut liquids = 0;
        for (red, pink) in red.vertices.iter().zip(&pink.vertices) {
            assert_eq!(red.p, pink.p, "bottle geometry changed");
            assert_eq!(red.fluid, pink.fluid);
            if red.fluid {
                liquids += 1;
                assert!(pink.c[2] > red.c[2] * 2.0, "liquid did not turn pink");
                assert!(pink.c[1] > red.c[1], "pink liquid needs a warmer hue");
            } else {
                assert_eq!(red.c, pink.c, "glass or cork was tinted");
            }
        }
        assert!(liquids > 0, "potion fill mesh missing");
    }

    #[test]
    fn liquid_physics_slashes_after_drag_then_settles() {
        let mut physics = LiquidPhysics::default();
        let calm = physics.update(0.0, Some((0.0, 0.24)));
        let disturbed = physics.update(1.0 / 30.0, Some((0.45, 0.60)));
        assert!(disturbed.wave_strength > calm.wave_strength);
        assert!(disturbed.slope_x.abs() + disturbed.slope_z.abs() > 0.0);

        let mut settled = disturbed;
        let mut time = 1.0 / 30.0;
        for _ in 0..150 {
            time += 1.0 / 30.0;
            settled = physics.update(time, Some((0.45, 0.60)));
        }
        assert!(settled.wave_strength < disturbed.wave_strength);
        let expected_x = -0.45_f32.sin() * 0.60_f32.tan();
        let expected_z = (0.45_f32.cos() * 0.60_f32.tan()).clamp(-0.42, 0.42);
        assert!((settled.slope_x - expected_x).abs() < 0.03);
        assert!((settled.slope_z - expected_z).abs() < 0.03);
    }

    #[test]
    fn liquid_surface_responds_to_gravity_and_waves() {
        let lower = Vec3::new(0.02, -0.30, 0.01);
        let surface = Vec3::new(0.02, -0.01, 0.01);
        let still = LiquidMotion::default();
        let tilted = LiquidMotion {
            slope_x: 0.2,
            slope_z: -0.1,
            wave_strength: 0.03,
        };
        assert_eq!(animate_liquid(lower, 0.0, tilted), lower);
        assert_ne!(
            animate_liquid(surface, 0.0, still),
            animate_liquid(surface, 1.0, still)
        );
        assert_ne!(
            animate_liquid(surface, 0.0, still),
            animate_liquid(surface, 0.0, tilted)
        );
        let normal = animate_liquid_normal(surface, Vec3::new(0.0, 1.0, 0.0), 0.0, tilted);
        assert!((normal.length() - 1.0).abs() < 0.001);
        assert!(normal.x.abs() + normal.z.abs() > 0.0);
    }

    #[test]
    fn opaque_liquid_draws_over_glass_behind_it() {
        let glass_behind = (true, -0.5, [0, 1, 2]);
        let liquid_in_front = (false, 0.5, [3, 4, 5]);
        let mut triangles = vec![liquid_in_front, glass_behind];

        sort_back_to_front(&mut triangles);

        assert_eq!(triangles, vec![glass_behind, liquid_in_front]);
    }

    #[test]
    fn opaque_liquid_wins_when_glass_depths_match() {
        let glass = (true, 0.0, [0, 1, 2]);
        let liquid = (false, 0.0, [3, 4, 5]);
        let mut triangles = vec![liquid, glass];

        sort_back_to_front(&mut triangles);

        assert_eq!(triangles, vec![glass, liquid]);
    }

    #[test]
    fn manual_rotation_changes_preview_orientation() {
        let path = Some("data/assets/3d/items/health_potion.glb");
        let r = Rect::new(0.0, 0.0, 270.0, 270.0);
        let mut default_view = Vec::new();
        let mut inspected_view = Vec::new();
        assert!(draw(
            &mut default_view,
            path,
            r,
            0.0,
            None,
            LiquidMotion::default()
        ));
        assert!(draw(
            &mut inspected_view,
            path,
            r,
            0.0,
            Some((std::f32::consts::FRAC_PI_2, -0.35)),
            LiquidMotion::default()
        ));
        let default_positions: Vec<_> = default_view.iter().map(|v| v.position).collect();
        let inspected_positions: Vec<_> = inspected_view.iter().map(|v| v.position).collect();
        assert_ne!(default_positions, inspected_positions);
    }

    #[test]
    fn drag_hit_area_is_limited_to_the_selected_3d_preview() {
        let hit = super::super::shop_preview_hit_test;
        assert!(hit(1300.0, 400.0, true));
        assert!(!hit(1300.0, 400.0, false));
        assert!(!hit(1200.0, 400.0, true));
    }
}
