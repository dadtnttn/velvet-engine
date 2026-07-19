//! **Tools** for 3D-style image billboards (not premade cutscenes).
//!
//! - [`Pose3D`] — yaw/pitch/roll/opacity/foil/depth you control  
//! - [`project_image`] — perspective project a rectangle to screen corners  
//! - [`Fx3dCamera`] — projection parameters  
//!
//! Build your own pack-open, shop flip, etc. by tweening these fields or using
//! [`crate::track::Timeline`]. Optional sample recipes live in [`crate::recipes`].

use serde::{Deserialize, Serialize};
use velvet_math::{Vec2, Vec3};

/// Full 3D-ish pose for an image billboard — author sets every field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pose3D {
    /// Screen-space center.
    pub pos: Vec2,
    /// Uniform scale (1 = native size).
    pub scale: f32,
    /// Yaw (Y) radians.
    pub yaw: f32,
    /// Pitch (X) radians.
    pub pitch: f32,
    /// Roll (Z) radians.
    pub roll: f32,
    /// Opacity `0..=1`.
    pub opacity: f32,
    /// Foil / highlight phase `0..=1` (you map this to UVs/shaders).
    pub foil: f32,
    /// Depth bias (larger = farther).
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

    /// Lerp (pass already-eased `t`).
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

    /// Facing scalar: positive ≈ front, negative ≈ back.
    pub fn facing_sign(&self) -> f32 {
        self.yaw.cos() * self.pitch.cos()
    }

    /// Front faces camera (rough).
    pub fn show_front(&self) -> bool {
        self.facing_sign() >= 0.0
    }

    /// Read a named channel (for tracks / script).
    pub fn get_channel(&self, ch: Pose3DChannel) -> f32 {
        match ch {
            Pose3DChannel::X => self.pos.x,
            Pose3DChannel::Y => self.pos.y,
            Pose3DChannel::Scale => self.scale,
            Pose3DChannel::Yaw => self.yaw,
            Pose3DChannel::Pitch => self.pitch,
            Pose3DChannel::Roll => self.roll,
            Pose3DChannel::Opacity => self.opacity,
            Pose3DChannel::Foil => self.foil,
            Pose3DChannel::Depth => self.depth,
        }
    }

    /// Write a named channel.
    pub fn set_channel(&mut self, ch: Pose3DChannel, value: f32) {
        match ch {
            Pose3DChannel::X => self.pos.x = value,
            Pose3DChannel::Y => self.pos.y = value,
            Pose3DChannel::Scale => self.scale = value,
            Pose3DChannel::Yaw => self.yaw = value,
            Pose3DChannel::Pitch => self.pitch = value,
            Pose3DChannel::Roll => self.roll = value,
            Pose3DChannel::Opacity => self.opacity = value.clamp(0.0, 1.0),
            Pose3DChannel::Foil => self.foil = value,
            Pose3DChannel::Depth => self.depth = value,
        }
    }
}

/// Animatable channels of [`Pose3D`] (tools; you compose keyframes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pose3DChannel {
    /// Position X.
    X,
    /// Position Y.
    Y,
    /// Scale.
    Scale,
    /// Yaw.
    Yaw,
    /// Pitch.
    Pitch,
    /// Roll.
    Roll,
    /// Opacity.
    Opacity,
    /// Foil phase.
    Foil,
    /// Depth.
    Depth,
}

impl Pose3DChannel {
    /// Parse author name.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "scale" => Some(Self::Scale),
            "yaw" | "rot_y" | "ry" => Some(Self::Yaw),
            "pitch" | "rot_x" | "rx" => Some(Self::Pitch),
            "roll" | "rot_z" | "rz" => Some(Self::Roll),
            "opacity" | "alpha" => Some(Self::Opacity),
            "foil" | "shimmer" => Some(Self::Foil),
            "depth" | "z" => Some(Self::Depth),
            _ => None,
        }
    }
}

/// Four projected corners (TL, TR, BR, BL) — draw tool output.
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
    /// Opacity.
    pub opacity: f32,
    /// Foil phase pass-through.
    pub foil: f32,
    /// Front face flag.
    pub front: bool,
    /// Sort key (larger = farther).
    pub sort_z: f32,
}

impl ProjectedQuad {
    /// Corners array.
    pub fn corners(&self) -> [Vec2; 4] {
        [self.tl, self.tr, self.br, self.bl]
    }

    /// AABB min.
    pub fn min(&self) -> Vec2 {
        let xs = [self.tl.x, self.tr.x, self.br.x, self.bl.x];
        let ys = [self.tl.y, self.tr.y, self.br.y, self.bl.y];
        Vec2::new(
            xs.iter().cloned().fold(f32::INFINITY, f32::min),
            ys.iter().cloned().fold(f32::INFINITY, f32::min),
        )
    }

    /// AABB max.
    pub fn max(&self) -> Vec2 {
        let xs = [self.tl.x, self.tr.x, self.br.x, self.bl.x];
        let ys = [self.tl.y, self.tr.y, self.br.y, self.bl.y];
        Vec2::new(
            xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            ys.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        )
    }
}

/// Camera / projection tool parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Fx3dCamera {
    /// Distance to plane.
    pub distance: f32,
    /// Focal length scale.
    pub focal: f32,
    /// Extra screen scale.
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

/// Image billboard description (data only — you own textures).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageBillboard {
    /// Logical id.
    pub id: String,
    /// Pose.
    pub pose: Pose3D,
    /// Half-width in local units.
    pub half_w: f32,
    /// Half-height in local units.
    pub half_h: f32,
    /// Optional front content key (path / card id).
    pub front: Option<String>,
    /// Optional back content key.
    pub back: Option<String>,
}

impl ImageBillboard {
    /// New billboard.
    pub fn new(id: impl Into<String>, pose: Pose3D, half_w: f32, half_h: f32) -> Self {
        Self {
            id: id.into(),
            pose,
            half_w,
            half_h,
            front: None,
            back: None,
        }
    }

    /// Project with camera.
    pub fn project(&self, cam: &Fx3dCamera) -> ProjectedQuad {
        project_image(&self.pose, self.half_w, self.half_h, cam)
    }

    /// Content key for current face.
    pub fn visible_content(&self) -> Option<&str> {
        if self.pose.show_front() {
            self.front.as_deref().or(self.back.as_deref())
        } else {
            self.back.as_deref().or(self.front.as_deref())
        }
    }
}

/// Project a rectangle with pose → screen quad (**core tool**).
pub fn project_image(
    pose: &Pose3D,
    half_w: f32,
    half_h: f32,
    cam: &Fx3dCamera,
) -> ProjectedQuad {
    let locals = [
        Vec3::new(-half_w, -half_h, 0.0),
        Vec3::new(half_w, -half_h, 0.0),
        Vec3::new(half_w, half_h, 0.0),
        Vec3::new(-half_w, half_h, 0.0),
    ];
    let (sy, cy) = pose.yaw.sin_cos();
    let (sp, cp) = pose.pitch.sin_cos();
    let (sr, cr) = pose.roll.sin_cos();

    let mut projected = [Vec2::ZERO; 4];
    let mut zs = [0.0f32; 4];
    for (i, p) in locals.iter().enumerate() {
        let mut v = Vec3::new(p.x * pose.scale, p.y * pose.scale, p.z);
        let y1 = v.y * cp - v.z * sp;
        let z1 = v.y * sp + v.z * cp;
        v.y = y1;
        v.z = z1;
        let x2 = v.x * cy + v.z * sy;
        let z2 = -v.x * sy + v.z * cy;
        v.x = x2;
        v.z = z2;
        let x3 = v.x * cr - v.y * sr;
        let y3 = v.x * sr + v.y * cr;
        v.x = x3;
        v.y = y3;

        let z_cam = cam.distance + pose.depth + v.z;
        let z_safe = z_cam.max(0.15);
        let inv = cam.focal / z_safe;
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

/// Sort billboards far → near for painter’s algorithm.
pub fn sort_projected(mut items: Vec<(String, ProjectedQuad)>) -> Vec<(String, ProjectedQuad)> {
    items.sort_by(|a, b| {
        b.1.sort_z
            .partial_cmp(&a.1.sort_z)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    items
}

/// Foil phase from time (tool helper; map to your shader/UVs).
pub fn foil_phase(time_secs: f32, speed: f32) -> f32 {
    ((time_secs * speed) % 1.0 + 1.0) % 1.0
}

/// Linear yaw from 0 → π (you choose ease via [`velvet_math::Ease`] before calling).
pub fn yaw_flip_amount(t01: f32) -> f32 {
    t01.clamp(0.0, 1.0) * std::f32::consts::PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_flat_tool() {
        let pose = Pose3D::flat(Vec2::new(100.0, 200.0));
        let q = project_image(&pose, 50.0, 80.0, &Fx3dCamera::default());
        assert!(q.front);
        assert!(q.tl.x < q.tr.x);
    }

    #[test]
    fn channels_roundtrip() {
        let mut p = Pose3D::default();
        p.set_channel(Pose3DChannel::Yaw, 1.25);
        assert!((p.get_channel(Pose3DChannel::Yaw) - 1.25).abs() < 1e-5);
    }

    #[test]
    fn billboard_picks_back_when_flipped() {
        let mut b = ImageBillboard::new("c", Pose3D::flat(Vec2::ZERO), 10.0, 10.0);
        b.front = Some("front".into());
        b.back = Some("back".into());
        b.pose.yaw = std::f32::consts::PI;
        assert_eq!(b.visible_content(), Some("back"));
    }

    #[test]
    fn yaw_flip_amount_ends_at_pi() {
        assert!((yaw_flip_amount(1.0) - std::f32::consts::PI).abs() < 1e-5);
    }
}
