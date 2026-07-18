//! Axis-aligned rectangles.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::Vec2;

/// Axis-aligned rectangle defined by minimum and maximum corners.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rect {
    /// Minimum corner (inclusive).
    pub min: Vec2,
    /// Maximum corner (exclusive in some raster contexts; treat as inclusive for math).
    pub max: Vec2,
}

impl Rect {
    /// Empty rect at origin.
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    /// Create from min/max.
    #[inline]
    pub const fn from_min_max(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create from position (min) and size.
    #[inline]
    pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self {
            min: pos,
            max: pos + size,
        }
    }

    /// Create from center and half-extents (half size).
    pub fn from_center_half_size(center: Vec2, half: Vec2) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Width.
    #[inline]
    pub fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height.
    #[inline]
    pub fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    /// Size as vector.
    #[inline]
    pub fn size(self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    /// Center point.
    #[inline]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Area (may be negative if inverted).
    #[inline]
    pub fn area(self) -> f32 {
        self.width() * self.height()
    }

    /// Whether the rect has positive area.
    pub fn is_valid(self) -> bool {
        self.max.x >= self.min.x && self.max.y >= self.min.y
    }

    /// Contains a point (inclusive min, exclusive max — half-open).
    pub fn contains_point(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x < self.max.x && p.y >= self.min.y && p.y < self.max.y
    }

    /// Inclusive contains.
    pub fn contains_point_inclusive(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Intersection with another rect, or `None` if empty.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if max.x > min.x && max.y > min.y {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Whether this rect intersects another.
    pub fn intersects(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Union bounding box.
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Expand by padding on all sides.
    pub fn inflate(self, padding: f32) -> Self {
        Self {
            min: Vec2::new(self.min.x - padding, self.min.y - padding),
            max: Vec2::new(self.max.x + padding, self.max.y + padding),
        }
    }

    /// Translate by offset.
    pub fn translate(self, offset: Vec2) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersection_and_contains() {
        let a = Rect::from_pos_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Rect::from_pos_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0));
        assert!(a.intersects(b));
        let i = a.intersection(b).unwrap();
        assert!((i.width() - 5.0).abs() < 1e-5);
        assert!(a.contains_point(Vec2::new(0.0, 0.0)));
        assert!(!a.contains_point(Vec2::new(10.0, 10.0)));
    }
}
