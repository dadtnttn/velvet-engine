//! Visual pose sample for any animated target (card, sprite, UI, …).

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Snapshot of transform + opacity used by renderers and card UIs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnimPose {
    /// World / layout position.
    pub pos: Vec2,
    /// Uniform scale (1.0 = rest).
    pub scale: f32,
    /// Rotation in radians.
    pub rotation: f32,
    /// Opacity `0..=1`.
    pub opacity: f32,
}

impl Default for AnimPose {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            scale: 1.0,
            rotation: 0.0,
            opacity: 1.0,
        }
    }
}

impl AnimPose {
    /// Pose at a position.
    pub fn at(pos: Vec2) -> Self {
        Self {
            pos,
            ..Default::default()
        }
    }

    /// Hidden (opacity 0).
    pub fn hidden(pos: Vec2) -> Self {
        Self {
            pos,
            opacity: 0.0,
            ..Default::default()
        }
    }

    /// Linear blend toward `other` by `t` in `0..=1` (already eased).
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            pos: Vec2::new(
                self.pos.x + (other.pos.x - self.pos.x) * t,
                self.pos.y + (other.pos.y - self.pos.y) * t,
            ),
            scale: self.scale + (other.scale - self.scale) * t,
            rotation: self.rotation + (other.rotation - self.rotation) * t,
            opacity: self.opacity + (other.opacity - self.opacity) * t,
        }
    }
}

/// Which scalar/vector field a tween drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimField {
    /// Position X.
    X,
    /// Position Y.
    Y,
    /// Both axes as independent tweens share this for presets.
    Scale,
    /// Rotation radians.
    Rotation,
    /// Opacity.
    Opacity,
}
