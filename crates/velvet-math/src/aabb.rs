//! Axis-aligned bounding boxes (2D) and spatial helpers.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{Rect, Vec2};

/// Axis-aligned bounding box (min/max corners).
///
/// Functionally similar to [`Rect`] but oriented toward culling, physics, and
/// spatial queries with inclusive max by convention in overlap tests.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Aabb2 {
    /// Minimum corner.
    pub min: Vec2,
    /// Maximum corner.
    pub max: Vec2,
}

impl Aabb2 {
    /// Empty/invalid AABB at origin.
    pub const ZERO: Self = Self {
        min: Vec2::ZERO,
        max: Vec2::ZERO,
    };

    /// Create from min/max, ensuring min ≤ max.
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// From center and half-extents.
    pub fn from_center_extents(center: Vec2, half: Vec2) -> Self {
        let half = half.abs();
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// From a center and uniform half-size.
    pub fn from_center_radius(center: Vec2, radius: f32) -> Self {
        Self::from_center_extents(center, Vec2::splat(radius.abs()))
    }

    /// From position and size (size may be negative; normalized).
    pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self::new(pos, pos + size)
    }

    /// From a [`Rect`].
    pub fn from_rect(rect: Rect) -> Self {
        Self::new(rect.min, rect.max)
    }

    /// Convert to [`Rect`].
    pub fn to_rect(self) -> Rect {
        Rect::from_min_max(self.min, self.max)
    }

    /// Build from a point cloud (empty if no points).
    pub fn from_points(points: &[Vec2]) -> Option<Self> {
        let mut iter = points.iter().copied();
        let first = iter.next()?;
        let mut min = first;
        let mut max = first;
        for p in iter {
            min = min.min(p);
            max = max.max(p);
        }
        Some(Self { min, max })
    }

    /// Merge two AABBs.
    pub fn merge(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Expand to include a point.
    pub fn expand_to_point(self, p: Vec2) -> Self {
        Self {
            min: self.min.min(p),
            max: self.max.max(p),
        }
    }

    /// Inflate by padding on all sides.
    pub fn inflate(self, padding: f32) -> Self {
        Self {
            min: Vec2::new(self.min.x - padding, self.min.y - padding),
            max: Vec2::new(self.max.x + padding, self.max.y + padding),
        }
    }

    /// Inflate by per-axis padding.
    pub fn inflate_xy(self, pad: Vec2) -> Self {
        Self {
            min: self.min - pad,
            max: self.max + pad,
        }
    }

    /// Translate by offset.
    pub fn translate(self, offset: Vec2) -> Self {
        Self {
            min: self.min + offset,
            max: self.max + offset,
        }
    }

    /// Scale about a pivot (typically center).
    pub fn scale_about(self, pivot: Vec2, scale: Vec2) -> Self {
        let min = pivot + (self.min - pivot) * scale;
        let max = pivot + (self.max - pivot) * scale;
        Self::new(min, max)
    }

    /// Center.
    #[inline]
    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Full size.
    #[inline]
    pub fn size(self) -> Vec2 {
        self.max - self.min
    }

    /// Half extents.
    #[inline]
    pub fn half_extents(self) -> Vec2 {
        self.size() * 0.5
    }

    /// Surface area (perimeter for 2D).
    pub fn perimeter(self) -> f32 {
        let s = self.size();
        2.0 * (s.x + s.y)
    }

    /// Area (may be zero/negative if invalid).
    pub fn area(self) -> f32 {
        let s = self.size();
        s.x * s.y
    }

    /// Whether min ≤ max on both axes.
    pub fn is_valid(self) -> bool {
        self.max.x >= self.min.x && self.max.y >= self.min.y
    }

    /// Whether area is positive.
    pub fn is_empty(self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }

    /// Inclusive contains point.
    pub fn contains_point(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Whether `other` is fully inside `self`.
    pub fn contains_aabb(self, other: Self) -> bool {
        other.min.x >= self.min.x
            && other.min.y >= self.min.y
            && other.max.x <= self.max.x
            && other.max.y <= self.max.y
    }

    /// Overlap test (touching edges count as overlap).
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Strict overlap (positive area intersection).
    pub fn overlaps(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }

    /// Intersection AABB, or `None` if empty.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if max.x >= min.x && max.y >= min.y {
            Some(Self { min, max })
        } else {
            None
        }
    }

    /// Closest point on the AABB to `p` (clamped).
    pub fn closest_point(self, p: Vec2) -> Vec2 {
        Vec2::new(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
        )
    }

    /// Squared distance from point to AABB (0 if inside).
    pub fn distance_squared_point(self, p: Vec2) -> f32 {
        let c = self.closest_point(p);
        p.distance_squared(c)
    }

    /// Distance from point to AABB.
    pub fn distance_point(self, p: Vec2) -> f32 {
        self.distance_squared_point(p).sqrt()
    }

    /// Minkowski sum (expand by other half-size). Useful for swept tests.
    pub fn minkowski_sum(self, other: Self) -> Self {
        Self {
            min: self.min + other.min,
            max: self.max + other.max,
        }
    }

    /// Support mapping: farthest point in direction `dir`.
    pub fn support(self, dir: Vec2) -> Vec2 {
        Vec2::new(
            if dir.x >= 0.0 { self.max.x } else { self.min.x },
            if dir.y >= 0.0 { self.max.y } else { self.min.y },
        )
    }

    /// Four corners in counter-clockwise order starting at min.
    pub fn corners(self) -> [Vec2; 4] {
        [
            self.min,
            Vec2::new(self.max.x, self.min.y),
            self.max,
            Vec2::new(self.min.x, self.max.y),
        ]
    }

    /// Transform AABB by a 2D affine matrix (returns world AABB of transformed corners).
    pub fn transformed(self, m: crate::Mat3) -> Self {
        let corners = self.corners();
        let mut min = m.transform_point2(corners[0]);
        let mut max = min;
        for c in corners.iter().skip(1) {
            let p = m.transform_point2(*c);
            min = min.min(p);
            max = max.max(p);
        }
        Self { min, max }
    }
}

/// Sweep a moving AABB against a static AABB; returns time of impact in `[0,1]` if any.
///
/// `delta` is the displacement of `moving` over the interval.
pub fn aabb_sweep(moving: Aabb2, static_box: Aabb2, delta: Vec2) -> Option<f32> {
    if moving.overlaps(static_box) {
        return Some(0.0);
    }

    let mut t_enter = 0.0f32;
    let mut t_exit = 1.0f32;

    for axis in 0..2 {
        let (m_min, m_max, s_min, s_max, d) = if axis == 0 {
            (
                moving.min.x,
                moving.max.x,
                static_box.min.x,
                static_box.max.x,
                delta.x,
            )
        } else {
            (
                moving.min.y,
                moving.max.y,
                static_box.min.y,
                static_box.max.y,
                delta.y,
            )
        };

        if d.abs() < 1e-12 {
            if m_max < s_min || m_min > s_max {
                return None;
            }
            continue;
        }

        let inv = 1.0 / d;
        let mut t1 = (s_min - m_max) * inv;
        let mut t2 = (s_max - m_min) * inv;
        if t1 > t2 {
            core::mem::swap(&mut t1, &mut t2);
        }
        t_enter = t_enter.max(t1);
        t_exit = t_exit.min(t2);
        if t_enter > t_exit {
            return None;
        }
    }

    if (0.0..=1.0).contains(&t_enter) {
        Some(t_enter)
    } else {
        None
    }
}

/// Utility: union of many AABBs.
pub fn aabb_union(boxes: &[Aabb2]) -> Option<Aabb2> {
    let mut iter = boxes.iter().copied();
    let first = iter.next()?;
    Some(iter.fold(first, |a, b| a.merge(b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mat3;

    #[test]
    fn contains_and_intersect() {
        let a = Aabb2::from_pos_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Aabb2::from_pos_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0));
        assert!(a.intersects(b));
        let i = a.intersection(b).unwrap();
        assert!((i.size().x - 5.0).abs() < 1e-5);
        assert!(a.contains_point(Vec2::new(0.0, 0.0)));
        assert!(!a.contains_aabb(b));
    }

    #[test]
    fn closest_point_outside() {
        let a = Aabb2::from_pos_size(Vec2::ZERO, Vec2::splat(2.0));
        let c = a.closest_point(Vec2::new(5.0, 1.0));
        assert!((c.x - 2.0).abs() < 1e-5);
        assert!((c.y - 1.0).abs() < 1e-5);
        assert!(a.distance_point(Vec2::new(5.0, 1.0)) > 2.9);
    }

    #[test]
    fn from_points() {
        let aabb = Aabb2::from_points(&[
            Vec2::new(1.0, 2.0),
            Vec2::new(-3.0, 4.0),
            Vec2::new(0.0, -1.0),
        ])
        .unwrap();
        assert!((aabb.min.x - (-3.0)).abs() < 1e-5);
        assert!((aabb.max.y - 4.0).abs() < 1e-5);
    }

    #[test]
    fn transformed_rotation() {
        let a = Aabb2::from_center_extents(Vec2::ZERO, Vec2::splat(1.0));
        let m = Mat3::from_angle(std::f32::consts::FRAC_PI_4);
        let t = a.transformed(m);
        // Rotated square AABB grows.
        assert!(t.size().x > 2.0 - 1e-3);
    }

    #[test]
    fn sweep_hits() {
        let moving = Aabb2::from_pos_size(Vec2::new(0.0, 0.0), Vec2::splat(1.0));
        let static_box = Aabb2::from_pos_size(Vec2::new(3.0, 0.0), Vec2::splat(1.0));
        let t = aabb_sweep(moving, static_box, Vec2::new(5.0, 0.0)).unwrap();
        assert!(t > 0.0 && t < 1.0);
        // Should contact when moving.max.x reaches static.min.x => 1 + 5t = 3 => t=0.4
        assert!((t - 0.4).abs() < 1e-4);
    }

    #[test]
    fn sweep_already_overlapping() {
        let a = Aabb2::from_pos_size(Vec2::ZERO, Vec2::splat(2.0));
        let b = Aabb2::from_pos_size(Vec2::new(1.0, 1.0), Vec2::splat(2.0));
        assert_eq!(aabb_sweep(a, b, Vec2::X), Some(0.0));
    }

    #[test]
    fn union_many() {
        let u = aabb_union(&[
            Aabb2::from_pos_size(Vec2::ZERO, Vec2::ONE),
            Aabb2::from_pos_size(Vec2::new(5.0, 5.0), Vec2::ONE),
        ])
        .unwrap();
        assert!((u.max.x - 6.0).abs() < 1e-5);
    }

    #[test]
    fn property_intersection_symmetric_and_subset() {
        let cases = [
            (
                Aabb2::from_pos_size(Vec2::ZERO, Vec2::new(10.0, 10.0)),
                Aabb2::from_pos_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0)),
            ),
            (
                Aabb2::from_center_extents(Vec2::new(0.0, 0.0), Vec2::splat(2.0)),
                Aabb2::from_center_extents(Vec2::new(1.0, 0.0), Vec2::splat(2.0)),
            ),
            (
                Aabb2::from_pos_size(Vec2::new(-5.0, -5.0), Vec2::splat(3.0)),
                Aabb2::from_pos_size(Vec2::new(-4.0, -4.0), Vec2::splat(1.0)),
            ),
        ];
        for (a, b) in cases {
            assert_eq!(a.intersects(b), b.intersects(a));
            if let Some(i) = a.intersection(b) {
                let j = b.intersection(a).unwrap();
                assert!((i.min.x - j.min.x).abs() < 1e-5);
                assert!((i.max.y - j.max.y).abs() < 1e-5);
                // Intersection is contained in both.
                assert!(a.contains_aabb(i) || i.size().x <= a.size().x + 1e-4);
                assert!(a.contains_point(i.min) || a.intersects(i));
                assert!(b.intersects(i));
            }
        }
        // Disjoint: no intersection.
        let d1 = Aabb2::from_pos_size(Vec2::ZERO, Vec2::ONE);
        let d2 = Aabb2::from_pos_size(Vec2::new(5.0, 5.0), Vec2::ONE);
        assert!(!d1.intersects(d2));
        assert!(d1.intersection(d2).is_none());
    }

    #[test]
    fn property_contains_point_vs_closest() {
        let boxes = [
            Aabb2::from_pos_size(Vec2::ZERO, Vec2::new(4.0, 6.0)),
            Aabb2::from_center_radius(Vec2::new(10.0, -3.0), 2.5),
            Aabb2::from_center_extents(Vec2::new(-2.0, 4.0), Vec2::new(1.0, 3.0)),
        ];
        let samples = [
            Vec2::ZERO,
            Vec2::new(1.0, 1.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(-1.0, 0.0),
            Vec2::new(2.0, 3.0),
            Vec2::new(10.0, -3.0),
        ];
        for aabb in boxes {
            for p in samples {
                let c = aabb.closest_point(p);
                // Closest point always inside or on boundary.
                assert!(
                    c.x >= aabb.min.x - 1e-4
                        && c.x <= aabb.max.x + 1e-4
                        && c.y >= aabb.min.y - 1e-4
                        && c.y <= aabb.max.y + 1e-4
                );
                if aabb.contains_point(p) {
                    assert!(
                        (c - p).length() < 1e-4,
                        "inside point should be closest to itself"
                    );
                    assert!(aabb.distance_point(p) < 1e-4);
                } else {
                    assert!(aabb.distance_point(p) > 0.0);
                    // Distance equals length to closest.
                    assert!((aabb.distance_point(p) - (c - p).length()).abs() < 1e-4);
                }
            }
        }
    }

    #[test]
    fn property_merge_expand_translate() {
        let a = Aabb2::from_pos_size(Vec2::new(0.0, 0.0), Vec2::splat(2.0));
        let b = Aabb2::from_pos_size(Vec2::new(5.0, -1.0), Vec2::splat(1.0));
        let m = a.merge(b);
        assert!(m.contains_aabb(a));
        assert!(m.contains_aabb(b));
        let expanded = a.expand_to_point(Vec2::new(10.0, 10.0));
        assert!(expanded.contains_point(Vec2::new(10.0, 10.0)));
        let t = a.translate(Vec2::new(3.0, -2.0));
        assert!((t.min.x - 3.0).abs() < 1e-5);
        assert!((t.max.y - 0.0).abs() < 1e-5);
        let inf = a.inflate(1.0);
        assert!((inf.size().x - a.size().x - 2.0).abs() < 1e-4);
        let inf2 = a.inflate_xy(Vec2::new(2.0, 0.5));
        assert!((inf2.size().x - a.size().x - 4.0).abs() < 1e-4);
        assert!((inf2.size().y - a.size().y - 1.0).abs() < 1e-4);
    }

    #[test]
    fn property_sweep_monotonic_speed() {
        let moving = Aabb2::from_pos_size(Vec2::new(0.0, 0.0), Vec2::splat(1.0));
        let wall = Aabb2::from_pos_size(Vec2::new(10.0, 0.0), Vec2::splat(1.0));
        // Faster motion should hit earlier in normalized t for same displacement?
        // With displacement vector, t is fraction of displacement.
        let t1 = aabb_sweep(moving, wall, Vec2::new(20.0, 0.0)).unwrap();
        let t2 = aabb_sweep(moving, wall, Vec2::new(40.0, 0.0)).unwrap();
        // Larger displacement reaches sooner in parameter space.
        assert!(t2 < t1);
        assert!(t1 > 0.0 && t1 < 1.0);
        // Miss completely when moving away.
        assert!(aabb_sweep(moving, wall, Vec2::new(-5.0, 0.0)).is_none());
        // Miss when offset vertically with pure horizontal motion and gap.
        let high = Aabb2::from_pos_size(Vec2::new(10.0, 50.0), Vec2::splat(1.0));
        assert!(aabb_sweep(moving, high, Vec2::new(20.0, 0.0)).is_none());
    }

    #[test]
    fn property_from_points_bounds() {
        let mut pts = Vec::new();
        for i in 0..20 {
            pts.push(Vec2::new((i as f32 - 10.0) * 0.5, (i as f32 % 5.0) - 2.0));
        }
        let aabb = Aabb2::from_points(&pts).unwrap();
        for p in &pts {
            assert!(aabb.contains_point(*p));
        }
        assert!(Aabb2::from_points(&[]).is_none());
        assert!(aabb_union(&[]).is_none());
    }

    #[test]
    fn property_scale_about_center() {
        let a = Aabb2::from_center_extents(Vec2::new(5.0, 5.0), Vec2::splat(2.0));
        let scaled = a.scale_about(a.center(), Vec2::splat(2.0));
        assert!((scaled.center() - a.center()).length() < 1e-4);
        assert!((scaled.size().x - a.size().x * 2.0).abs() < 1e-4);
    }

    #[test]
    fn rect_roundtrip() {
        let r = Rect::from_pos_size(Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0));
        let a = Aabb2::from_rect(r);
        let back = a.to_rect();
        assert!((back.min.x - r.min.x).abs() < 1e-5);
        assert!((back.max.y - r.max.y).abs() < 1e-5);
    }
}
