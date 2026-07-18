//! Axis helpers.

use velvet_math::Vec2;

/// 1D axis sample.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Axis1d(pub f32);

impl Axis1d {
    /// Clamp to `[-1, 1]`.
    pub fn clamped(self) -> Self {
        Self(self.0.clamp(-1.0, 1.0))
    }

    /// Apply deadzone.
    pub fn with_deadzone(self, deadzone: f32) -> Self {
        if self.0.abs() < deadzone {
            Self(0.0)
        } else {
            self
        }
    }
}

/// 2D axis sample.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Axis2d {
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
}

impl Axis2d {
    /// Create.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// From [`Vec2`].
    pub fn from_vec2(v: Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }

    /// To [`Vec2`].
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Clamp length to 1.
    pub fn clamp_length(self) -> Self {
        let v = self.to_vec2().clamp_length_max(1.0);
        Self::from_vec2(v)
    }

    /// Radial deadzone.
    pub fn with_deadzone(self, deadzone: f32) -> Self {
        let v = self.to_vec2();
        if v.length() < deadzone {
            Self::default()
        } else {
            self
        }
    }
}
