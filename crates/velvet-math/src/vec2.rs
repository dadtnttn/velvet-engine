//! 2D vector type.

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Two-dimensional vector with `f32` components.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Vec2 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
}

impl Vec2 {
    /// Zero vector.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    /// Unit X.
    pub const X: Self = Self { x: 1.0, y: 0.0 };
    /// Unit Y.
    pub const Y: Self = Self { x: 0.0, y: 1.0 };
    /// One vector.
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    /// Negative unit X.
    pub const NEG_X: Self = Self { x: -1.0, y: 0.0 };
    /// Negative unit Y.
    pub const NEG_Y: Self = Self { x: 0.0, y: -1.0 };

    /// Create a new vector.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Splat a scalar into both components.
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    /// Squared length (avoids sqrt).
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Euclidean length.
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Distance to another point.
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Squared distance to another point.
    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// 2D cross product magnitude (z-component of 3D cross).
    #[inline]
    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// Perpendicular vector rotated 90° counter-clockwise.
    #[inline]
    pub fn perp(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }

    /// Normalize, or return zero if length is near zero.
    pub fn normalize_or_zero(self) -> Self {
        let len = self.length();
        if len > 1e-8 {
            self / len
        } else {
            Self::ZERO
        }
    }

    /// Try to normalize; returns `None` if too small.
    pub fn try_normalize(self) -> Option<Self> {
        let len = self.length();
        if len > 1e-8 {
            Some(self / len)
        } else {
            None
        }
    }

    /// Clamp length to at most `max`.
    pub fn clamp_length_max(self, max: f32) -> Self {
        let len_sq = self.length_squared();
        if len_sq > max * max && len_sq > 0.0 {
            self * (max / len_sq.sqrt())
        } else {
            self
        }
    }

    /// Linear interpolation.
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    /// Component-wise minimum.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// Component-wise maximum.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    /// Absolute value per component.
    #[inline]
    pub fn abs(self) -> Self {
        Self {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    /// Floor per component.
    #[inline]
    pub fn floor(self) -> Self {
        Self {
            x: self.x.floor(),
            y: self.y.floor(),
        }
    }

    /// Ceil per component.
    #[inline]
    pub fn ceil(self) -> Self {
        Self {
            x: self.x.ceil(),
            y: self.y.ceil(),
        }
    }

    /// Rotate by angle in radians (counter-clockwise).
    pub fn rotate(self, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self {
            x: self.x * c - self.y * s,
            y: self.x * s + self.y * c,
        }
    }

    /// Angle of this vector in radians from +X.
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }

    /// Create a unit vector from an angle in radians.
    pub fn from_angle(radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        Self { x: c, y: s }
    }

    /// Reflect incident vector about a normal (normal should be normalized).
    pub fn reflect(self, normal: Self) -> Self {
        self - normal * (2.0 * self.dot(normal))
    }

    /// Project onto another vector.
    pub fn project_onto(self, other: Self) -> Self {
        let denom = other.length_squared();
        if denom <= 1e-12 {
            Self::ZERO
        } else {
            other * (self.dot(other) / denom)
        }
    }

    /// Convert to `[x, y]` array.
    #[inline]
    pub const fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }

    /// From `[x, y]` array.
    #[inline]
    pub const fn from_array(a: [f32; 2]) -> Self {
        Self { x: a[0], y: a[1] }
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        rhs * self
    }
}

impl Mul for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(a: [f32; 2]) -> Self {
        Self::from_array(a)
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from(t: (f32, f32)) -> Self {
        Self::new(t.0, t.1)
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(v: Vec2) -> Self {
        v.to_array()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_and_normalize() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 1e-5);
        let n = v.normalize_or_zero();
        assert!((n.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn rotate_90() {
        let v = Vec2::X.rotate(std::f32::consts::FRAC_PI_2);
        assert!((v.x).abs() < 1e-5);
        assert!((v.y - 1.0).abs() < 1e-5);
    }

    #[test]
    fn dot_cross() {
        assert!((Vec2::X.dot(Vec2::Y)).abs() < f32::EPSILON);
        assert!((Vec2::X.cross(Vec2::Y) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn reflect_horizontal() {
        let incident = Vec2::new(1.0, -1.0);
        let normal = Vec2::Y;
        let r = incident.reflect(normal);
        assert!((r.x - 1.0).abs() < 1e-5);
        assert!((r.y - 1.0).abs() < 1e-5);
    }
}
