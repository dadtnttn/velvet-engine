//! 3x3 matrix for 2D affine transforms (column-major).

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Vec2;

/// 3×3 matrix stored in column-major order (compatible with GPU conventions).
///
/// Columns are `(m00, m01, m02)`, `(m10, m11, m12)`, `(m20, m21, m22)`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Mat3 {
    /// Column 0.
    pub x_axis: [f32; 3],
    /// Column 1.
    pub y_axis: [f32; 3],
    /// Column 2.
    pub z_axis: [f32; 3],
}

impl Default for Mat3 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat3 {
    /// Identity matrix.
    pub const IDENTITY: Self = Self {
        x_axis: [1.0, 0.0, 0.0],
        y_axis: [0.0, 1.0, 0.0],
        z_axis: [0.0, 0.0, 1.0],
    };

    /// Zero matrix.
    pub const ZERO: Self = Self {
        x_axis: [0.0, 0.0, 0.0],
        y_axis: [0.0, 0.0, 0.0],
        z_axis: [0.0, 0.0, 0.0],
    };

    /// Create from three columns.
    pub const fn from_cols(x: [f32; 3], y: [f32; 3], z: [f32; 3]) -> Self {
        Self {
            x_axis: x,
            y_axis: y,
            z_axis: z,
        }
    }

    /// Translation matrix.
    pub fn from_translation(t: Vec2) -> Self {
        Self {
            x_axis: [1.0, 0.0, 0.0],
            y_axis: [0.0, 1.0, 0.0],
            z_axis: [t.x, t.y, 1.0],
        }
    }

    /// Uniform scale matrix.
    pub fn from_scale(scale: Vec2) -> Self {
        Self {
            x_axis: [scale.x, 0.0, 0.0],
            y_axis: [0.0, scale.y, 0.0],
            z_axis: [0.0, 0.0, 1.0],
        }
    }

    /// Rotation matrix (radians, counter-clockwise).
    pub fn from_angle(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            x_axis: [c, s, 0.0],
            y_axis: [-s, c, 0.0],
            z_axis: [0.0, 0.0, 1.0],
        }
    }

    /// TRS: translation * rotation * scale (applied to column vectors as M * v).
    pub fn from_scale_angle_translation(scale: Vec2, angle: f32, translation: Vec2) -> Self {
        let (s, c) = angle.sin_cos();
        Self {
            x_axis: [c * scale.x, s * scale.x, 0.0],
            y_axis: [-s * scale.y, c * scale.y, 0.0],
            z_axis: [translation.x, translation.y, 1.0],
        }
    }

    /// Matrix multiplication `self * rhs`.
    pub fn mul_mat3(self, rhs: Self) -> Self {
        let mut out = Self::ZERO;
        for col in 0..3 {
            let r = match col {
                0 => rhs.x_axis,
                1 => rhs.y_axis,
                _ => rhs.z_axis,
            };
            let x = self.x_axis[0] * r[0] + self.y_axis[0] * r[1] + self.z_axis[0] * r[2];
            let y = self.x_axis[1] * r[0] + self.y_axis[1] * r[1] + self.z_axis[1] * r[2];
            let z = self.x_axis[2] * r[0] + self.y_axis[2] * r[1] + self.z_axis[2] * r[2];
            match col {
                0 => out.x_axis = [x, y, z],
                1 => out.y_axis = [x, y, z],
                _ => out.z_axis = [x, y, z],
            }
        }
        out
    }

    /// Transform a 2D point (w = 1).
    pub fn transform_point2(self, p: Vec2) -> Vec2 {
        Vec2 {
            x: self.x_axis[0] * p.x + self.y_axis[0] * p.y + self.z_axis[0],
            y: self.x_axis[1] * p.x + self.y_axis[1] * p.y + self.z_axis[1],
        }
    }

    /// Transform a 2D vector (w = 0).
    pub fn transform_vector2(self, v: Vec2) -> Vec2 {
        Vec2 {
            x: self.x_axis[0] * v.x + self.y_axis[0] * v.y,
            y: self.x_axis[1] * v.x + self.y_axis[1] * v.y,
        }
    }

    /// Determinant of the upper-left 2×2 (affine linear part) with full 3×3.
    pub fn determinant(self) -> f32 {
        let a = self.x_axis;
        let b = self.y_axis;
        let c = self.z_axis;
        a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
            + a[2] * (b[0] * c[1] - b[1] * c[0])
    }

    /// Inverse, or `None` if singular.
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / det;
        let a = self.x_axis;
        let b = self.y_axis;
        let c = self.z_axis;

        let m00 = (b[1] * c[2] - b[2] * c[1]) * inv;
        let m01 = (a[2] * c[1] - a[1] * c[2]) * inv;
        let m02 = (a[1] * b[2] - a[2] * b[1]) * inv;
        let m10 = (b[2] * c[0] - b[0] * c[2]) * inv;
        let m11 = (a[0] * c[2] - a[2] * c[0]) * inv;
        let m12 = (a[2] * b[0] - a[0] * b[2]) * inv;
        let m20 = (b[0] * c[1] - b[1] * c[0]) * inv;
        let m21 = (a[1] * c[0] - a[0] * c[1]) * inv;
        let m22 = (a[0] * b[1] - a[1] * b[0]) * inv;

        Some(Self {
            x_axis: [m00, m01, m02],
            y_axis: [m10, m11, m12],
            z_axis: [m20, m21, m22],
        })
    }

    /// Flatten to column-major `[f32; 9]`.
    pub fn to_cols_array(self) -> [f32; 9] {
        [
            self.x_axis[0],
            self.x_axis[1],
            self.x_axis[2],
            self.y_axis[0],
            self.y_axis[1],
            self.y_axis[2],
            self.z_axis[0],
            self.z_axis[1],
            self.z_axis[2],
        ]
    }
}

impl core::ops::Mul for Mat3 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.mul_mat3(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_point() {
        let m = Mat3::from_translation(Vec2::new(10.0, -5.0));
        let p = m.transform_point2(Vec2::new(1.0, 2.0));
        assert!((p.x - 11.0).abs() < 1e-5);
        assert!((p.y - (-3.0)).abs() < 1e-5);
    }

    #[test]
    fn inverse_roundtrip() {
        let m = Mat3::from_scale_angle_translation(Vec2::new(2.0, 3.0), 0.4, Vec2::new(5.0, -2.0));
        let inv = m.inverse().expect("invertible");
        let p = Vec2::new(3.0, 7.0);
        let back = inv.transform_point2(m.transform_point2(p));
        assert!((back.x - p.x).abs() < 1e-4);
        assert!((back.y - p.y).abs() < 1e-4);
    }
}
