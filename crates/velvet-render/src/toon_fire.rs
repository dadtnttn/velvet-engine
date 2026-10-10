//! A hearth fire drawn in toon style ([`ToonFire3D`]): flame tongues as flat
//! painted silhouettes that face the camera, in three bands of colour (a red rim,
//! an orange body, a pale yellow heart, or a palette of its own) laid one before
//! the other. Each tongue
//! is a flipbook of shapes made at start-up (licking, rippling) played at its own
//! pace, and stretched, swayed and leant every frame on top; small wisps break off
//! the tips and rise; a bed of embers breathes under the logs; sparks drift up,
//! cool from yellow to red and go out. Its light follows the flames' strength.
use crate::{
    Material3D, Mesh3D, MeshDraw3D, MeshHandle, MeshVertex3D, Render3DError, Wgpu3DRenderer,
};
use velvet_math::{Mat4, Vec3};

/// Outline points per side of a tongue.
const SIDE: usize = 12;
/// Shapes per flipbook cycle.
const FRAMES: usize = 16;
const TONGUES: usize = 6;
const WISPS: usize = 4;
const SPARKS: usize = 16;
const EMBERS: usize = 9;
/// The width the flipbook shapes are made at (metres); tongues scale from it.
const UNIT_WIDTH: f32 = 0.24;

/// The three painted bands, back to front: scale of the shape, colour, glow.
const LAYERS: [(f32, [f32; 3], f32); 3] = [
    (1.0, [0.62, 0.07, 0.02], 0.55),
    (0.74, [0.95, 0.30, 0.04], 0.6),
    (0.44, [1.0, 0.70, 0.26], 0.8),
];

/// A smooth wobble that never repeats exactly.
fn wobble(t: f32, seed: f32) -> f32 {
    (t * 1.7 + seed * 3.1).sin() * 0.5
        + (t * 2.9 + seed * 1.3).sin() * 0.3
        + (t * 5.3 + seed * 7.7).sin() * 0.2
}

/// How strongly the fire burns at time `t` (about 0.8..1.2).
fn strength(t: f32) -> f32 {
    1.0 + 0.12 * wobble(t * 2.3, 0.4) + 0.06 * (t * 17.0).sin() * (t * 3.1).sin()
}

/// One flame silhouette of unit height and width `width`, at phase `p` (0..1 of a
/// cycle), as points from the base up the left edge to the tip and down the right.
fn outline(width: f32, p: f32, seed: f32) -> Vec<[f32; 2]> {
    let tau = std::f32::consts::TAU;
    let edge = |side: f32, k: usize| {
        let u = k as f32 / SIDE as f32; // 0 base .. 1 tip
        let belly = (u * std::f32::consts::PI).sin().powf(0.55);
        let taper = (1.0 - u).powf(0.85);
        let pinch = 1.0 - 0.18 * (-((u - 0.55) / 0.12).powi(2)).exp();
        let round = (u / 0.12).clamp(0.0, 1.0).sqrt().max(0.15);
        let w = width * 0.5 * belly * taper * pinch * round;
        // The centre line licks, travelling up; ripples run up each edge.
        let lick = width
            * 0.55
            * u.powf(1.6)
            * ((p * tau - u * 5.0 + seed * 4.0).sin() * 0.65
                + (2.0 * p * tau - u * 8.0 + seed).sin() * 0.35);
        let ripple = width * 0.07 * u * (3.0 * p * tau - u * 14.0 + seed * 9.0 + side * 1.3).sin();
        [lick + side * (w + ripple.max(-w * 0.6)), u]
    };
    let mut out: Vec<[f32; 2]> = (0..=SIDE).map(|k| edge(-1.0, k)).collect();
    out.extend((0..SIDE).rev().map(|k| edge(1.0, k)));
    out
}

/// A flame slab through `points` (as `outline` gives them): a thin prism in the x/y
/// plane facing +z, its faces stitched in strips from the left edge to the right.
/// (A fan from a centre vertex does not show in this renderer, and geometry with
/// no depth at all is dropped, hence the strips and the few millimetres.)
fn fan(points: &[[f32; 2]]) -> Mesh3D {
    const HALF: f32 = 0.012;
    let n = points.len() as u32;
    let mut vertices = Vec::with_capacity(2 * points.len());
    for (z, nz) in [(HALF, 1.0), (-HALF, -1.0)] {
        vertices.extend(
            points
                .iter()
                .map(|p| MeshVertex3D::new([p[0], p[1], z], [0.0, 0.0, nz], [0.5, 0.5])),
        );
    }
    let mut indices = Vec::new();
    let both = |indices: &mut Vec<u32>, a: u32, b: u32, c: u32| {
        indices.extend_from_slice(&[a, b, c, a, c, b])
    };
    // Left edge k (k = 0..=SIDE) pairs with the right edge's point at the same height.
    let side = SIDE as u32;
    let right = |k: u32| if k == 0 { 0 } else { n - k };
    for face in [0, n] {
        for k in 0..side {
            let (l0, l1, r0, r1) = (face + k, face + k + 1, face + right(k), face + right(k + 1));
            if r0 != l0 {
                both(&mut indices, l0, l1, r0);
            }
            if k + 1 < side {
                both(&mut indices, l1, r1, r0);
            }
        }
    }
    // The rim between the faces.
    for i in 0..n {
        let j = (i + 1) % n;
        both(&mut indices, i, n + i, n + j);
        both(&mut indices, i, n + j, j);
    }
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

/// A flipbook: [layer][frame] of flame shapes `width` wide (as a share of UNIT_WIDTH).
fn flipbook(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    width: f32,
    seed: f32,
) -> Result<Vec<Vec<MeshHandle>>, Render3DError> {
    let mut book = Vec::new();
    for (layer, &(scale, _, _)) in LAYERS.iter().enumerate() {
        let mut frames = Vec::new();
        for f in 0..FRAMES {
            let p = f as f32 / FRAMES as f32 + layer as f32 * 0.04;
            frames.push(renderer.upload_mesh(device, &fan(&outline(width * scale, p, seed)))?);
        }
        book.push(frames);
    }
    Ok(book)
}

struct Tongue {
    x: f32,
    depth: f32,
    height: f32,
    seed: f32,
    /// Flipbook cycles per second.
    pace: f32,
    /// [layer][frame]
    frames: Vec<Vec<MeshHandle>>,
}

/// A fire: where it burns, its tongues, and the meshes of its embers and sparks. Made once
/// (its flipbooks are uploaded then), drawn every frame with [`ToonFire3D::draws`].
pub struct ToonFire3D {
    base: Vec3,
    tongues: Vec<Tongue>,
    /// A small flame flipbook for the wisps.
    wisp: Vec<Vec<MeshHandle>>,
    ember: MeshHandle,
    spark: MeshHandle,
    /// The three bands' colours, back to front (the grate's orange unless changed).
    palette: [[f32; 3]; 3],
}

impl ToonFire3D {
    /// `base`: the centre of the grate's floor; `width`: the opening's.
    pub fn new(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        base: Vec3,
        width: f32,
    ) -> Result<Self, Render3DError> {
        let mut tongues = Vec::new();
        for i in 0..TONGUES {
            let u = i as f32 / (TONGUES - 1) as f32 - 0.5; // -0.5 .. 0.5 across
            let centre = 1.0 - (u * 2.0).abs(); // the middle burns tallest
            let w = 0.17 + 0.07 * centre;
            let seed = i as f32 * 1.618;
            tongues.push(Tongue {
                x: u * width * 0.78 + (i as f32 * 1.7).sin() * 0.02,
                depth: 0.03 + ((i * 3) % 4) as f32 * 0.02,
                height: 0.26 + 0.24 * centre + 0.04 * (i as f32 * 2.3).sin(),
                seed,
                pace: 1.0 + 0.3 * (i as f32 * 0.7).sin(),
                frames: flipbook(renderer, device, w / UNIT_WIDTH, seed)?,
            });
        }
        let wisp = flipbook(renderer, device, 1.0, 5.0)?;
        let ember = renderer.upload_mesh(
            device,
            &Mesh3D::uv_sphere(1.0, 10, 6, Material3D::rgb(1.0, 1.0, 1.0)),
        )?;
        let spark = renderer.upload_mesh(
            device,
            &Mesh3D::uv_sphere(1.0, 6, 4, Material3D::rgb(1.0, 1.0, 1.0)),
        )?;
        Ok(Self {
            base,
            tongues,
            wisp,
            ember,
            spark,
            palette: [LAYERS[0].1, LAYERS[1].1, LAYERS[2].1],
        })
    }

    /// The same fire in other colours (a brazier's violet flame), back band to heart.
    pub fn with_palette(mut self, palette: [[f32; 3]; 3]) -> Self {
        self.palette = palette;
        self
    }

    /// How strongly a fire burns at time `t` (about 0.8..1.2): the lights it casts can breathe
    /// with it.
    pub fn strength(t: f32) -> f32 {
        strength(t)
    }

    /// The fire for this frame, turned to face along `toward` (fire -> camera).
    pub fn draws(&self, t: f32, toward: Vec3, draws: &mut Vec<MeshDraw3D>) {
        let toward = Vec3::new(toward.x, 0.0, toward.z).normalize_or_zero();
        let face = Mat4::from_rotation_y(toward.x.atan2(toward.z));
        let burn = strength(t);
        for (i, g) in self.tongues.iter().enumerate() {
            let frame = ((t * g.pace + g.seed).rem_euclid(1.0) * FRAMES as f32) as usize % FRAMES;
            // Each tongue breathes taller and shorter, and leans with the draught.
            let h = g.height
                * burn
                * (0.86
                    + 0.18 * wobble(t * 2.1 * g.pace, g.seed)
                    + 0.06 * (t * 9.0 + g.seed).sin());
            let lean = 0.06 * wobble(t * 1.3, g.seed + 2.0);
            let foot = self.base + Vec3::new(g.x, 0.0, g.depth);
            for (layer, &(scale, _, glow)) in LAYERS.iter().enumerate() {
                let color = self.palette[layer];
                let height = h * (0.42 + 0.58 * scale);
                // Later bands sit a hair before the earlier, so they read from the camera.
                let at = foot
                    + toward * (0.012 * layer as f32 + 0.02 * i as f32 / TONGUES as f32)
                    + Vec3::new(0.0, 0.004 * layer as f32, 0.0);
                draws.push(MeshDraw3D {
                    mesh: g.frames[layer][frame],
                    transform: Mat4::from_translation(at)
                        * face
                        * Mat4::from_rotation_z(lean)
                        * Mat4::from_scale(Vec3::new(UNIT_WIDTH, height, 1.0)),
                    material: Material3D::rgb(color[0], color[1], color[2])
                        .with_emission(glow * (0.9 + 0.2 * burn)),
                    texture: None,
                });
            }
        }
        // Wisps: little flames that break off the tips, rise and shrink away.
        for w in 0..WISPS {
            let period = 0.9 + w as f32 * 0.23;
            let life = ((t + w as f32 * 0.37) / period).fract();
            let size = (1.0 - life).powf(1.3);
            if size < 0.05 {
                continue;
            }
            let source = &self.tongues[(w * 2 + 1) % self.tongues.len()];
            let rise = source.height * burn * (0.7 + 0.8 * life);
            let x = source.x + 0.05 * (t * 3.0 + w as f32).sin() * life;
            let frame =
                ((t * 1.7 + w as f32 * 0.3).rem_euclid(1.0) * FRAMES as f32) as usize % FRAMES;
            for (layer, &(_, _, glow)) in LAYERS.iter().enumerate().take(2) {
                let color = self.palette[layer];
                let at = self.base
                    + Vec3::new(x, rise, source.depth)
                    + toward * (0.03 + 0.01 * layer as f32);
                draws.push(MeshDraw3D {
                    mesh: self.wisp[layer][frame],
                    transform: Mat4::from_translation(at)
                        * face
                        * Mat4::from_scale(Vec3::new(0.07 * size, 0.16 * size, 1.0)),
                    material: Material3D::rgb(color[0], color[1], color[2]).with_emission(glow),
                    texture: None,
                });
            }
        }
        // Embers breathing under the logs.
        for e in 0..EMBERS {
            let u = e as f32 / (EMBERS - 1) as f32 - 0.5;
            let pulse = 0.55
                + 0.45
                    * (t * (1.3 + 0.2 * e as f32) + e as f32 * 2.1)
                        .sin()
                        .max(-0.4);
            let at = self.base + Vec3::new(u * 0.62, 0.015, -0.03 + 0.05 * ((e * 7) % 3) as f32);
            let r = 0.035 + 0.012 * ((e * 5) % 3) as f32;
            draws.push(MeshDraw3D {
                mesh: self.ember,
                transform: Mat4::from_translation(at)
                    * Mat4::from_scale(Vec3::new(r * 1.3, r * 0.4, r * 1.0)),
                material: Material3D::rgb(0.7, 0.08 + 0.12 * pulse, 0.02)
                    .with_emission(0.3 + 0.6 * pulse),
                texture: None,
            });
        }
        // Sparks: up, drifting, cooling from yellow to red, going out.
        for s in 0..SPARKS {
            let period = 1.3 + (s % 5) as f32 * 0.27;
            let life = ((t + s as f32 * 0.613) / period).fract();
            let seed = s as f32 * 2.399;
            let fade = (1.0 - life) * (life / 0.08).min(1.0);
            if fade < 0.04 {
                continue;
            }
            let start = Vec3::new(seed.sin() * 0.28, 0.12, (seed * 1.7).cos() * 0.05);
            let drift = Vec3::new(
                (t * 2.0 + seed).sin() * 0.07 * life,
                life * (0.85 + 0.25 * (seed * 3.0).sin()),
                (t * 1.5 + seed).cos() * 0.04 * life,
            );
            let size = 0.011 * (0.4 + 0.6 * fade);
            let mut material = Material3D::rgb(1.0, 0.85 - 0.6 * life, 0.35 - 0.33 * life)
                .with_emission(2.5 * fade + 0.5);
            material.base_color[3] = fade.min(1.0);
            draws.push(MeshDraw3D {
                mesh: self.spark,
                transform: Mat4::from_translation(self.base + start + drift + toward * 0.05)
                    * Mat4::from_scale(Vec3::new(size, size, size)),
                material,
                texture: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fire_breathes_around_its_level_and_never_dies() {
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        for k in 0..20_000 {
            let s = strength(k as f32 * 0.013);
            low = low.min(s);
            high = high.max(s);
        }
        assert!(low > 0.75 && high < 1.25, "{low}..{high}");
        assert!(high - low > 0.2, "it flickers: {low}..{high}");
    }

    #[test]
    fn a_tongue_rises_from_its_base_to_a_tip() {
        for (p, seed) in [(0.0, 0.0), (0.37, 1.6), (0.9, 4.2)] {
            let points = outline(UNIT_WIDTH, p, seed);
            assert_eq!(points.len(), 2 * SIDE + 1);
            // Up the left edge to the tip, down the right: both edges start at the base.
            assert_eq!(points[0][1], 0.0);
            assert_eq!(points[SIDE][1], 1.0);
            assert_eq!(points[2 * SIDE][1], 0.0);
            for k in 1..SIDE {
                let (left, right) = (points[k], points[2 * SIDE - k]);
                assert_eq!(left[1], right[1], "edges pair up at each height");
                assert!(
                    left[0] <= right[0] + 1e-6,
                    "the left edge stays left: {left:?} {right:?}"
                );
                assert!(right[0] - left[0] < UNIT_WIDTH, "no wider than its width");
            }
        }
    }

    #[test]
    fn a_tongue_is_a_thin_closed_slab() {
        let points = outline(UNIT_WIDTH, 0.25, 1.0);
        let mesh = fan(&points);
        let n = points.len();
        assert_eq!(mesh.vertices.len(), 2 * n);
        assert_eq!(mesh.indices.len() % 3, 0);
        assert!(mesh.indices.iter().all(|&i| (i as usize) < 2 * n));
        // Both faces and the rim are drawn, each triangle both ways round.
        let tris: Vec<[u32; 3]> = mesh.indices.chunks(3).map(|t| [t[0], t[1], t[2]]).collect();
        assert!(tris.chunks(2).all(|pair| pair[0][0] == pair[1][0]
            && pair[0][1] == pair[1][2]
            && pair[0][2] == pair[1][1]));
        let depth: Vec<f32> = mesh.vertices.iter().map(|v| v.position[2]).collect();
        assert!(
            depth.iter().all(|z| z.abs() > 0.0 && z.abs() < 0.02),
            "a few millimetres deep"
        );
    }
}
