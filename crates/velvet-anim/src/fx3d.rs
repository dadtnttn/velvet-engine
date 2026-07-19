//! 3D-style effects on **2D images** (cards, pack art, UI panels).
//!
//! Velvetâ€™s product path is 2D; this module projects a textured quad through a
//! simple perspective camera so authors can **generate** pack-open, card flip,
//! tilt, and foil shimmer without a full 3D mesh pipeline.
//!
//! Typical pack-open flow: [`PackOpenFx::start`] â†’ tick each frame â†’ sample
//! [`ProjectedQuad`] corners for softbuffer/wgpu.

use serde::{Deserialize, Serialize};
use velvet_math::{Ease, Vec2, Vec3};

/// Full 3D-ish pose for an image billboard.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pose3D {
    /// Screen-space center (pixels or virtual units).
    pub pos: Vec2,
    /// Uniform scale (1 = native size).
    pub scale: f32,
    /// Yaw (Y axis) radians â€” card flip / pack turn.
    pub yaw: f32,
    /// Pitch (X axis) radians â€” lean toward camera.
    pub pitch: f32,
    /// Roll (Z axis) radians â€” 2D spin.
    pub roll: f32,
    /// Opacity `0..=1`.
    pub opacity: f32,
    /// Foil / holographic shimmer phase `0..=1` (maps to highlight UV offset).
    pub foil: f32,
    /// Depth bias (larger = farther; used for sort + perspective).
    pub depth: f32,
}

impl Default for Pose3D {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            scale: 1.0,
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            opacity: 1.0,
            foil: 0.0,
            depth: 0.0,
        }
    }
}

impl Pose3D {
    /// Flat image at position.
    pub fn flat(pos: Vec2) -> Self {
        Self {
            pos,
            ..Default::default()
        }
    }

    /// Lerp (already eased `t`).
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            pos: Vec2::new(
                self.pos.x + (other.pos.x - self.pos.x) * t,
                self.pos.y + (other.pos.y - self.pos.y) * t,
            ),
            scale: self.scale + (other.scale - self.scale) * t,
            yaw: self.yaw + (other.yaw - self.yaw) * t,
            pitch: self.pitch + (other.pitch - self.pitch) * t,
            roll: self.roll + (other.roll - self.roll) * t,
            opacity: self.opacity + (other.opacity - self.opacity) * t,
            foil: self.foil + (other.foil - self.foil) * t,
            depth: self.depth + (other.depth - self.depth) * t,
        }
    }

    /// Face visibility: `1` front, `0` edge-on, negative = back face.
    pub fn facing_sign(&self) -> f32 {
        self.yaw.cos() * self.pitch.cos()
    }

    /// True if the â€œfrontâ€ of the card faces the camera (roughly).
    pub fn show_front(&self) -> bool {
        self.facing_sign() >= 0.0
    }
}

/// Four projected corners in screen space (TL, TR, BR, BL).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectedQuad {
    /// Top-left.
    pub tl: Vec2,
    /// Top-right.
    pub tr: Vec2,
    /// Bottom-right.
    pub br: Vec2,
    /// Bottom-left.
    pub bl: Vec2,
    /// Average opacity after perspective fade.
    pub opacity: f32,
    /// Foil phase.
    pub foil: f32,
    /// Which face to draw.
    pub front: bool,
    /// Approximate depth for sorting (larger farther).
    pub sort_z: f32,
}

impl ProjectedQuad {
    /// Axis-aligned bounding box min.
    pub fn min(&self) -> Vec2 {
        let xs = [self.tl.x, self.tr.x, self.br.x, self.bl.x];
        let ys = [self.tl.y, self.tr.y, self.br.y, self.bl.y];
        Vec2::new(
            xs.iter().cloned().fold(f32::INFINITY, f32::min),
            ys.iter().cloned().fold(f32::INFINITY, f32::min),
        )
    }

    /// Axis-aligned bounding box max.
    pub fn max(&self) -> Vec2 {
        let xs = [self.tl.x, self.tr.x, self.br.x, self.bl.x];
        let ys = [self.tl.y, self.tr.y, self.br.y, self.bl.y];
        Vec2::new(
            xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        )
    }

    /// Corners as array.
    pub fn corners(&self) -> [Vec2; 4] {
        [self.tl, self.tr, self.br, self.bl]
    }
}

/// Camera / projection settings for image FX.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Fx3dCamera {
    /// Distance from camera to the plane `z=0` (virtual units).
    pub distance: f32,
    /// Focal length scale (higher = less extreme perspective).
    pub focal: f32,
    /// Extra scale applied after projection.
    pub screen_scale: f32,
}

impl Default for Fx3dCamera {
    fn default() -> Self {
        Self {
            distance: 4.0,
            focal: 3.2,
            screen_scale: 1.0,
        }
    }
}

/// Project a unit rectangle (half-size `half_w`/`half_h` in local units) with pose.
pub fn project_image(
    pose: &Pose3D,
    half_w: f32,
    half_h: f32,
    cam: &Fx3dCamera,
) -> ProjectedQuad {
    // Local corners in image plane (z=0 local), before rotations.
    let locals = [
        Vec3::new(-half_w, -half_h, 0.0), // TL in y-up local; we'll flip y for screen later
        Vec3::new(half_w, -half_h, 0.0),
        Vec3::new(half_w, half_h, 0.0),
        Vec3::new(-half_w, half_h, 0.0),
    ];

    let (sy, cy) = pose.yaw.sin_cos();
    let (sp, cp) = pose.pitch.sin_cos();
    let (sr, cr) = pose.roll.sin_cos();

    // R = Rz * Ry * Rx (applied to column vectors as R * v)
    let mut projected = [Vec2::ZERO; 4];
    let mut zs = [0.0f32; 4];
    for (i, p) in locals.iter().enumerate() {
        // scale
        let mut v = Vec3::new(p.x * pose.scale, p.y * pose.scale, p.z);
        // pitch X
        let y1 = v.y * cp - v.z * sp;
        let z1 = v.y * sp + v.z * cp;
        v.y = y1;
        v.z = z1;
        // yaw Y
        let x2 = v.x * cy + v.z * sy;
        let z2 = -v.x * sy + v.z * cy;
        v.x = x2;
        v.z = z2;
        // roll Z
        let x3 = v.x * cr - v.y * sr;
        let y3 = v.x * sr + v.y * cr;
        v.x = x3;
        v.y = y3;

        // camera looks -Z; place plane at z = -distance + depth
        let z_cam = cam.distance + pose.depth + v.z;
        let z_safe = z_cam.max(0.15);
        let inv = cam.focal / z_safe;
        // screen y increases down in most 2D engines â†’ flip local y
        projected[i] = Vec2::new(
            pose.pos.x + v.x * inv * cam.screen_scale,
            pose.pos.y - v.y * inv * cam.screen_scale,
        );
        zs[i] = z_safe;
    }

    let avg_z = (zs[0] + zs[1] + zs[2] + zs[3]) * 0.25;
    ProjectedQuad {
        tl: projected[0],
        tr: projected[1],
        br: projected[2],
        bl: projected[3],
        opacity: pose.opacity,
        foil: pose.foil,
        front: pose.show_front(),
        sort_z: avg_z,
    }
}

// --- generators --------------------------------------------------------------

/// Parameters for generating a pack-open sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackOpenParams {
    /// Screen center of the pack.
    pub center: Vec2,
    /// Pack half-size (width/height).
    pub pack_half: Vec2,
    /// Card half-size when fanned.
    pub card_half: Vec2,
    /// How many cards to reveal.
    pub card_count: usize,
    /// Total duration seconds.
    pub duration: f32,
    /// Horizontal fan spacing between cards.
    pub fan_spacing: f32,
    /// Seed for slight random tilt (deterministic via simple hash).
    pub seed: u64,
}

impl Default for PackOpenParams {
    fn default() -> Self {
        Self {
            center: Vec2::new(480.0, 270.0),
            pack_half: Vec2::new(90.0, 120.0),
            card_half: Vec2::new(70.0, 100.0),
            card_count: 5,
            duration: 2.2,
            fan_spacing: 95.0,
            seed: 1,
        }
    }
}

/// Phase of a pack-open cinematic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackPhase {
    /// Pack sits, slight idle tilt.
    Present,
    /// Pack tears / yaws open.
    Tear,
    /// Cards lift out.
    Lift,
    /// Cards fan into a row.
    Fan,
    /// Hold final reveal.
    Hold,
    /// Finished.
    Done,
}

/// One image layer in a generated pack-open (pack art or card face).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackLayer {
    /// Logical id (`pack`, `card_0`â€¦).
    pub id: String,
    /// Role.
    pub kind: PackLayerKind,
    /// 3D pose.
    pub pose: Pose3D,
    /// Half extents for projection.
    pub half: Vec2,
    /// Optional content key (texture path / card id).
    pub content: Option<String>,
}

/// Layer role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackLayerKind {
    /// Sealed pack artwork.
    Pack,
    /// Individual card.
    Card,
}

/// Runtime pack-open effect generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackOpenFx {
    /// Params.
    pub params: PackOpenParams,
    /// Elapsed seconds.
    pub elapsed: f32,
    /// Current phase.
    pub phase: PackPhase,
    /// Layers (pack + cards).
    pub layers: Vec<PackLayer>,
    /// Normalized progress `0..=1`.
    pub progress: f32,
}

impl PackOpenFx {
    /// Start a new pack-open sequence.
    pub fn start(params: PackOpenParams) -> Self {
        let mut layers = Vec::new();
        layers.push(PackLayer {
            id: "pack".into(),
            kind: PackLayerKind::Pack,
            pose: Pose3D {
                pos: params.center,
                scale: 1.0,
                yaw: 0.0,
                pitch: -0.08,
                roll: 0.0,
                opacity: 1.0,
                foil: 0.0,
                depth: 0.0,
            },
            half: params.pack_half,
            content: Some("pack".into()),
        });
        for i in 0..params.card_count {
            layers.push(PackLayer {
                id: format!("card_{i}"),
                kind: PackLayerKind::Card,
                pose: Pose3D {
                    pos: params.center,
                    scale: 0.85,
                    yaw: std::f32::consts::PI, // start face-down-ish (back)
                    pitch: 0.0,
                    roll: 0.0,
                    opacity: 0.0,
                    foil: 0.0,
                    depth: 0.1 + i as f32 * 0.02,
                },
                half: params.card_half,
                content: Some(format!("card_{i}")),
            });
        }
        Self {
            params,
            elapsed: 0.0,
            phase: PackPhase::Present,
            layers,
            progress: 0.0,
        }
    }

    /// Convenience: open pack with N cards at center.
    pub fn open_at(center: Vec2, cards: usize, duration: f32) -> Self {
        Self::start(PackOpenParams {
            center,
            card_count: cards.max(1),
            duration: duration.max(0.5),
            ..Default::default()
        })
    }

    /// Finished?
    pub fn is_done(&self) -> bool {
        matches!(self.phase, PackPhase::Done) || self.progress >= 1.0
    }

    /// Advance generator by `dt`.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed += dt;
        let d = self.params.duration.max(0.5);
        self.progress = (self.elapsed / d).clamp(0.0, 1.0);
        self.phase = phase_for(self.progress);
        self.evaluate_poses();
    }

    /// Sample all layers as projected quads (painterâ€™s order: back â†’ front).
    pub fn projected(&self, cam: &Fx3dCamera) -> Vec<(String, ProjectedQuad)> {
        let mut items: Vec<(String, ProjectedQuad, f32)> = self
            .layers
            .iter()
            .filter(|l| l.pose.opacity > 0.001)
            .map(|l| {
                let q = project_image(&l.pose, l.half.x, l.half.y, cam);
                (l.id.clone(), q, q.sort_z)
            })
            .collect();
        // Farther first
        items.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        items.into_iter().map(|(id, q, _)| (id, q)).collect()
    }

    fn evaluate_poses(&mut self) {
        let p = self.progress;
        let center = self.params.center;
        let n = self.params.card_count.max(1);

        // Timeline segments (normalized)
        // 0.00â€“0.15 present
        // 0.15â€“0.35 tear
        // 0.35â€“0.55 lift
        // 0.55â€“0.85 fan
        // 0.85â€“1.00 hold
        for layer in &mut self.layers {
            match layer.kind {
                PackLayerKind::Pack => {
                    let pose = &mut layer.pose;
                    pose.pos = center;
                    if p < 0.15 {
                        let t = Ease::SineInOut.eval(p / 0.15);
                        pose.yaw = 0.12 * (t * std::f32::consts::TAU).sin() * 0.15;
                        pose.pitch = -0.08;
                        pose.opacity = 1.0;
                        pose.foil = t * 0.3;
                    } else if p < 0.35 {
                        let t = Ease::CubicIn.eval((p - 0.15) / 0.20);
                        pose.yaw = t * 1.1;
                        pose.pitch = -0.08 + t * 0.2;
                        pose.scale = 1.0 - t * 0.15;
                        pose.opacity = 1.0 - t * 0.85;
                        pose.foil = 0.3 + t * 0.5;
                    } else {
                        pose.opacity = 0.0;
                        pose.scale = 0.7;
                    }
                }
                PackLayerKind::Card => {
                    let idx = layer
                        .id
                        .strip_prefix("card_")
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    let pose = &mut layer.pose;
                    let stagger = idx as f32 * 0.04;
                    let local = ((p - 0.30 - stagger) / 0.55).clamp(0.0, 1.0);
                    let fan_t = Ease::CubicOut.eval(((p - 0.55) / 0.30).clamp(0.0, 1.0));

                    // Fan target positions
                    let total_w = (n.saturating_sub(1) as f32) * self.params.fan_spacing;
                    let x0 = center.x - total_w * 0.5;
                    let target_x = x0 + idx as f32 * self.params.fan_spacing;
                    let target_y = center.y + 40.0;
                    let tilt = ((idx as f32 + 1.0) * 0.07 + (self.params.seed % 7) as f32 * 0.01)
                        - 0.2;

                    if p < 0.30 + stagger {
                        pose.opacity = 0.0;
                        pose.pos = center;
                        pose.yaw = std::f32::consts::PI;
                    } else if local < 0.45 {
                        // lift + flip
                        let t = Ease::CubicOut.eval(local / 0.45);
                        pose.opacity = t;
                        pose.pos = Vec2::new(
                            center.x + (target_x - center.x) * t * 0.35,
                            center.y - 30.0 * t,
                        );
                        pose.yaw = std::f32::consts::PI * (1.0 - t); // flip to front
                        pose.pitch = -0.2 * (1.0 - t);
                        pose.scale = 0.85 + 0.15 * t;
                        pose.foil = t;
                        pose.depth = 0.05 - t * 0.04;
                    } else {
                        let t = fan_t.max(Ease::CubicOut.eval((local - 0.45) / 0.55));
                        pose.opacity = 1.0;
                        pose.pos = Vec2::new(
                            center.x + (target_x - center.x) * t,
                            center.y + (target_y - center.y) * t,
                        );
                        pose.yaw = 0.0;
                        pose.pitch = tilt * 0.15;
                        pose.roll = tilt * 0.08;
                        pose.scale = 1.0;
                        pose.foil = 0.4 + 0.3 * (p * 6.0 + idx as f32).sin().abs();
                        pose.depth = -0.05 - idx as f32 * 0.01;
                    }
                }
            }
        }
    }
}

fn phase_for(p: f32) -> PackPhase {
    if p >= 1.0 {
        PackPhase::Done
    } else if p < 0.15 {
        PackPhase::Present
    } else if p < 0.35 {
        PackPhase::Tear
    } else if p < 0.55 {
        PackPhase::Lift
    } else if p < 0.85 {
        PackPhase::Fan
    } else {
        PackPhase::Hold
    }
}

/// Flip card yaw 0 â†’ Ï€ over duration (helper for single-card flip FX).
pub fn sample_card_flip(t: f32, ease: Ease) -> f32 {
    let e = ease.eval(t.clamp(0.0, 1.0));
    e * std::f32::consts::PI
}

/// Continuous foil shimmer phase from time.
pub fn foil_phase(time_secs: f32, speed: f32) -> f32 {
    ((time_secs * speed) % 1.0 + 1.0) % 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_flat_is_axis_aligned() {
        let pose = Pose3D::flat(Vec2::new(100.0, 200.0));
        let q = project_image(&pose, 50.0, 80.0, &Fx3dCamera::default());
        assert!(q.front);
        // roughly ordered left/right
        assert!(q.tl.x < q.tr.x);
        assert!(q.bl.x < q.br.x);
    }

    #[test]
    fn yaw_pi_shows_back() {
        let mut pose = Pose3D::flat(Vec2::ZERO);
        pose.yaw = std::f32::consts::PI;
        assert!(!pose.show_front());
        let q = project_image(&pose, 40.0, 60.0, &Fx3dCamera::default());
        assert!(!q.front);
    }

    #[test]
    fn pack_open_generates_cards_and_finishes() {
        let mut fx = PackOpenFx::open_at(Vec2::new(400.0, 300.0), 5, 1.5);
        assert_eq!(fx.layers.len(), 6); // pack + 5
        for _ in 0..120 {
            fx.tick(1.0 / 60.0);
        }
        assert!(fx.is_done() || fx.progress >= 0.99);
        let cam = Fx3dCamera::default();
        let quads = fx.projected(&cam);
        // near end, cards visible
        assert!(
            quads.iter().any(|(id, q)| id.starts_with("card") && q.opacity > 0.5),
            "cards should be visible {:?}",
            quads.iter().map(|(i, q)| (i, q.opacity)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn pack_phases_progress() {
        let mut fx = PackOpenFx::open_at(Vec2::ZERO, 3, 2.0);
        assert_eq!(fx.phase, PackPhase::Present);
        fx.tick(0.4); // mid tear-ish depending duration 2.0
        assert!(matches!(
            fx.phase,
            PackPhase::Present | PackPhase::Tear | PackPhase::Lift
        ));
        fx.elapsed = 1.9;
        fx.tick(0.0);
        assert!(matches!(fx.phase, PackPhase::Hold | PackPhase::Done | PackPhase::Fan));
    }

    #[test]
    fn flip_helper() {
        assert!((sample_card_flip(0.0, Ease::Linear)).abs() < 1e-5);
        assert!((sample_card_flip(1.0, Ease::Linear) - std::f32::consts::PI).abs() < 1e-4);
    }
}

