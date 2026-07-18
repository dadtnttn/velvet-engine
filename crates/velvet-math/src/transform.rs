//! 2D transform (translation, rotation, scale).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Mat3, Vec2};

/// Decomposed 2D transform.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Transform2D {
    /// Translation in world units.
    pub translation: Vec2,
    /// Rotation in radians.
    pub rotation: f32,
    /// Non-uniform scale.
    pub scale: Vec2,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        translation: Vec2::ZERO,
        rotation: 0.0,
        scale: Vec2::ONE,
    };

    /// From translation only.
    pub const fn from_translation(translation: Vec2) -> Self {
        Self {
            translation,
            rotation: 0.0,
            scale: Vec2::ONE,
        }
    }

    /// From translation and rotation.
    pub const fn from_xy_rotation(x: f32, y: f32, rotation: f32) -> Self {
        Self {
            translation: Vec2::new(x, y),
            rotation,
            scale: Vec2::ONE,
        }
    }

    /// Convert to matrix.
    pub fn to_mat3(self) -> Mat3 {
        Mat3::from_scale_angle_translation(self.scale, self.rotation, self.translation)
    }

    /// Transform a point.
    pub fn transform_point(self, p: Vec2) -> Vec2 {
        self.to_mat3().transform_point2(p)
    }

    /// Transform a direction (no translation).
    pub fn transform_vector(self, v: Vec2) -> Vec2 {
        self.to_mat3().transform_vector2(v)
    }

    /// Right-multiply: apply `child` in local space of `self`.
    pub fn mul_transform(self, child: Self) -> Self {
        let mat = self.to_mat3() * child.to_mat3();
        // Extract approximate TRS (assumes positive scale, no shear).
        let translation = Vec2::new(mat.z_axis[0], mat.z_axis[1]);
        let scale_x = Vec2::new(mat.x_axis[0], mat.x_axis[1]).length();
        let scale_y = Vec2::new(mat.y_axis[0], mat.y_axis[1]).length();
        let rotation = mat.x_axis[1].atan2(mat.x_axis[0]);
        Self {
            translation,
            rotation,
            scale: Vec2::new(scale_x, scale_y),
        }
    }

    /// Look-at helper: set rotation so +X points toward `target` from translation.
    pub fn looking_at(mut self, target: Vec2) -> Self {
        let dir = target - self.translation;
        self.rotation = dir.angle();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_point() {
        let t = Transform2D::IDENTITY;
        let p = t.transform_point(Vec2::new(3.0, 4.0));
        assert_eq!(p, Vec2::new(3.0, 4.0));
    }

    #[test]
    fn translate_scale() {
        let t = Transform2D {
            translation: Vec2::new(10.0, 0.0),
            rotation: 0.0,
            scale: Vec2::splat(2.0),
        };
        let p = t.transform_point(Vec2::new(1.0, 1.0));
        assert!((p.x - 12.0).abs() < 1e-5);
        assert!((p.y - 2.0).abs() < 1e-5);
    }
}
