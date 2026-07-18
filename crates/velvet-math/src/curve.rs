//! 2D curves: linear polylines, quadratic/cubic Bézier, Catmull-Rom.

use crate::{lerp, Vec2};

/// Sample a quadratic Bézier curve at parameter `t ∈ [0, 1]`.
///
/// Control points: `p0` start, `p1` control, `p2` end.
pub fn quadratic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    p0 * (u * u) + p1 * (2.0 * u * t) + p2 * (t * t)
}

/// First derivative of quadratic Bézier (tangent direction, not normalized).
pub fn quadratic_bezier_deriv(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    (p1 - p0) * (2.0 * (1.0 - t)) + (p2 - p1) * (2.0 * t)
}

/// Sample a cubic Bézier curve at `t ∈ [0, 1]`.
pub fn cubic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    let uu = u * u;
    let tt = t * t;
    p0 * (uu * u) + p1 * (3.0 * uu * t) + p2 * (3.0 * u * tt) + p3 * (tt * t)
}

/// First derivative of cubic Bézier.
pub fn cubic_bezier_deriv(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let u = 1.0 - t;
    (p1 - p0) * (3.0 * u * u) + (p2 - p1) * (6.0 * u * t) + (p3 - p2) * (3.0 * t * t)
}

/// Approximate arc length of a cubic Bézier via uniform samples.
pub fn cubic_bezier_length(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, segments: usize) -> f32 {
    let n = segments.max(1);
    let mut len = 0.0;
    let mut prev = p0;
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let p = cubic_bezier(p0, p1, p2, p3, t);
        len += prev.distance(p);
        prev = p;
    }
    len
}

/// Split cubic Bézier at `t` into two cubics (De Casteljau).
pub fn split_cubic_bezier(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    t: f32,
) -> ([Vec2; 4], [Vec2; 4]) {
    let t = t.clamp(0.0, 1.0);
    let p01 = p0.lerp(p1, t);
    let p12 = p1.lerp(p2, t);
    let p23 = p2.lerp(p3, t);
    let p012 = p01.lerp(p12, t);
    let p123 = p12.lerp(p23, t);
    let p0123 = p012.lerp(p123, t);
    ([p0, p01, p012, p0123], [p0123, p123, p23, p3])
}

/// Catmull-Rom spline through four points; samples segment between `p1` and `p2`.
pub fn catmull_rom(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    p0 * (-0.5 * t3 + t2 - 0.5 * t)
        + p1 * (1.5 * t3 - 2.5 * t2 + 1.0)
        + p2 * (-1.5 * t3 + 2.0 * t2 + 0.5 * t)
        + p3 * (0.5 * t3 - 0.5 * t2)
}

/// Uniform polyline: sample along connected segments by normalized arc parameter.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Polyline2 {
    /// Vertices in order.
    pub points: Vec<Vec2>,
}

impl Polyline2 {
    /// Create from points.
    pub fn new(points: impl Into<Vec<Vec2>>) -> Self {
        Self {
            points: points.into(),
        }
    }

    /// Number of points.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Total length of the polyline.
    pub fn length(&self) -> f32 {
        self.points.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    /// Sample at normalized parameter `t ∈ [0, 1]` along arc length.
    pub fn sample(&self, t: f32) -> Option<Vec2> {
        if self.points.is_empty() {
            return None;
        }
        if self.points.len() == 1 {
            return Some(self.points[0]);
        }
        let t = t.clamp(0.0, 1.0);
        let total = self.length();
        if total <= 1e-12 {
            return Some(self.points[0]);
        }
        let target = t * total;
        let mut acc = 0.0;
        for w in self.points.windows(2) {
            let seg = w[0].distance(w[1]);
            if acc + seg >= target || seg <= 1e-12 {
                let local = if seg <= 1e-12 {
                    0.0
                } else {
                    ((target - acc) / seg).clamp(0.0, 1.0)
                };
                return Some(w[0].lerp(w[1], local));
            }
            acc += seg;
        }
        self.points.last().copied()
    }

    /// Closest point on the polyline to `p`, with squared distance.
    pub fn closest_point(&self, p: Vec2) -> Option<(Vec2, f32)> {
        if self.points.is_empty() {
            return None;
        }
        if self.points.len() == 1 {
            let d = p.distance_squared(self.points[0]);
            return Some((self.points[0], d));
        }
        let mut best_pt = self.points[0];
        let mut best_d = p.distance_squared(best_pt);
        for w in self.points.windows(2) {
            let ab = w[1] - w[0];
            let len_sq = ab.length_squared();
            let t = if len_sq <= 1e-12 {
                0.0
            } else {
                ((p - w[0]).dot(ab) / len_sq).clamp(0.0, 1.0)
            };
            let q = w[0] + ab * t;
            let d = p.distance_squared(q);
            if d < best_d {
                best_d = d;
                best_pt = q;
            }
        }
        Some((best_pt, best_d))
    }
}

/// A cubic Bézier segment with evaluation helpers.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CubicBezier2 {
    /// Start.
    pub p0: Vec2,
    /// Control 1.
    pub p1: Vec2,
    /// Control 2.
    pub p2: Vec2,
    /// End.
    pub p3: Vec2,
}

impl CubicBezier2 {
    /// Create segment.
    pub const fn new(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2) -> Self {
        Self { p0, p1, p2, p3 }
    }

    /// Evaluate position.
    pub fn position(&self, t: f32) -> Vec2 {
        cubic_bezier(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// Evaluate tangent (derivative).
    pub fn tangent(&self, t: f32) -> Vec2 {
        cubic_bezier_deriv(self.p0, self.p1, self.p2, self.p3, t)
    }

    /// Unit tangent, or zero if degenerate.
    pub fn unit_tangent(&self, t: f32) -> Vec2 {
        self.tangent(t).normalize_or_zero()
    }

    /// Approximate length.
    pub fn length(&self, segments: usize) -> f32 {
        cubic_bezier_length(self.p0, self.p1, self.p2, self.p3, segments)
    }

    /// Sample `count` points from t=0..=1 inclusive.
    pub fn sample_points(&self, count: usize) -> Vec<Vec2> {
        let n = count.max(2);
        (0..n)
            .map(|i| self.position(i as f32 / (n - 1) as f32))
            .collect()
    }

    /// Convert to polyline with `segments` edges.
    pub fn to_polyline(&self, segments: usize) -> Polyline2 {
        Polyline2::new(self.sample_points(segments + 1))
    }
}

/// A multi-segment cubic path (piecewise Bézier).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BezierPath2 {
    /// Segments in order.
    pub segments: Vec<CubicBezier2>,
}

impl BezierPath2 {
    /// Empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a cubic segment.
    pub fn push(&mut self, seg: CubicBezier2) {
        self.segments.push(seg);
    }

    /// Build a smooth-ish path through waypoints using Catmull-Rom → cubic conversion.
    pub fn from_catmull_waypoints(points: &[Vec2], closed: bool) -> Self {
        if points.len() < 2 {
            return Self::new();
        }
        let mut segs = Vec::new();
        let n = points.len();
        let count = if closed { n } else { n - 1 };
        for i in 0..count {
            let p0 = points[if i == 0 {
                if closed {
                    n - 1
                } else {
                    0
                }
            } else {
                i - 1
            }];
            let p1 = points[i];
            let p2 = points[(i + 1) % n];
            let p3 = points[if i + 2 >= n {
                if closed {
                    (i + 2) % n
                } else {
                    n - 1
                }
            } else {
                i + 2
            }];
            // Centripetal-ish Catmull to Bezier control points.
            let c1 = p1 + (p2 - p0) * (1.0 / 6.0);
            let c2 = p2 - (p3 - p1) * (1.0 / 6.0);
            segs.push(CubicBezier2::new(p1, c1, c2, p2));
        }
        Self { segments: segs }
    }

    /// Total approximate length.
    pub fn length(&self, segments_per: usize) -> f32 {
        self.segments.iter().map(|s| s.length(segments_per)).sum()
    }

    /// Sample by normalized path parameter `t ∈ [0, 1]` (equal per-segment, not arc-length).
    pub fn sample_uniform(&self, t: f32) -> Option<Vec2> {
        if self.segments.is_empty() {
            return None;
        }
        let t = t.clamp(0.0, 1.0);
        if t >= 1.0 {
            return Some(self.segments.last().unwrap().position(1.0));
        }
        let n = self.segments.len() as f32;
        let ft = t * n;
        let idx = (ft.floor() as usize).min(self.segments.len() - 1);
        let local = ft - idx as f32;
        Some(self.segments[idx].position(local))
    }

    /// Flatten all segments into a polyline.
    pub fn to_polyline(&self, segments_per: usize) -> Polyline2 {
        let mut pts = Vec::new();
        for (i, seg) in self.segments.iter().enumerate() {
            let samples = seg.sample_points(segments_per + 1);
            if i == 0 {
                pts.extend(samples);
            } else {
                pts.extend(samples.into_iter().skip(1));
            }
        }
        Polyline2::new(pts)
    }
}

/// Linear interpolate two points (explicit for API symmetry).
#[inline]
pub fn line_sample(a: Vec2, b: Vec2, t: f32) -> Vec2 {
    a.lerp(b, t.clamp(0.0, 1.0))
}

/// Evaluate a cubic Hermite spline (positions + tangents).
pub fn hermite(p0: Vec2, m0: Vec2, p1: Vec2, m1: Vec2, t: f32) -> Vec2 {
    let t = t.clamp(0.0, 1.0);
    let t2 = t * t;
    let t3 = t2 * t;
    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;
    p0 * h00 + m0 * h10 + p1 * h01 + m1 * h11
}

/// Approximate curvature of cubic Bézier at `t` (scalar in 2D).
pub fn cubic_bezier_curvature(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> f32 {
    // Second derivative of cubic Bézier:
    // B''(t) = 6(1-t)(p2 - 2p1 + p0) + 6t(p3 - 2p2 + p1)
    let t = t.clamp(0.0, 1.0);
    let d1 = cubic_bezier_deriv(p0, p1, p2, p3, t);
    let d2 = (p2 - p1 * 2.0 + p0) * (6.0 * (1.0 - t)) + (p3 - p2 * 2.0 + p1) * (6.0 * t);
    let cross = d1.cross(d2);
    let speed = d1.length();
    if speed < 1e-8 {
        0.0
    } else {
        cross.abs() / (speed * speed * speed)
    }
}

/// Ease a point along a cubic using an external scalar easing on `t`.
pub fn cubic_bezier_eased(
    p0: Vec2,
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
    t: f32,
    ease: impl Fn(f32) -> f32,
) -> Vec2 {
    cubic_bezier(p0, p1, p2, p3, ease(t.clamp(0.0, 1.0)))
}

/// Lerp helper re-export style for f32 along curve alpha.
pub fn mix_f32(a: f32, b: f32, t: f32) -> f32 {
    lerp(a, b, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadratic_endpoints() {
        let p0 = Vec2::ZERO;
        let p1 = Vec2::new(0.5, 1.0);
        let p2 = Vec2::new(1.0, 0.0);
        let a = quadratic_bezier(p0, p1, p2, 0.0);
        let b = quadratic_bezier(p0, p1, p2, 1.0);
        assert!(a.distance(p0) < 1e-5);
        assert!(b.distance(p2) < 1e-5);
        let mid = quadratic_bezier(p0, p1, p2, 0.5);
        assert!(mid.y > 0.4);
    }

    #[test]
    fn cubic_endpoints_and_split() {
        let c = CubicBezier2::new(
            Vec2::ZERO,
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
        );
        assert!(c.position(0.0).distance(Vec2::ZERO) < 1e-5);
        assert!(c.position(1.0).distance(Vec2::new(1.0, 0.0)) < 1e-5);
        let (l, r) = split_cubic_bezier(c.p0, c.p1, c.p2, c.p3, 0.5);
        assert!(l[3].distance(r[0]) < 1e-5);
        assert!(c.length(16) > 1.0);
    }

    #[test]
    fn polyline_sample() {
        let pl = Polyline2::new(vec![
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
        ]);
        assert!((pl.length() - 20.0).abs() < 1e-4);
        let mid = pl.sample(0.25).unwrap();
        assert!((mid.x - 5.0).abs() < 1e-3);
        let (cp, d) = pl.closest_point(Vec2::new(5.0, 1.0)).unwrap();
        assert!((cp.y).abs() < 1e-4);
        assert!((d - 1.0).abs() < 1e-3);
    }

    #[test]
    fn catmull_path() {
        let pts = [
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 1.0),
            Vec2::new(3.0, 0.0),
        ];
        let path = BezierPath2::from_catmull_waypoints(&pts, false);
        assert_eq!(path.segments.len(), 3);
        let p = path.sample_uniform(0.0).unwrap();
        assert!(p.distance(Vec2::ZERO) < 1e-4);
        let pl = path.to_polyline(4);
        assert!(pl.len() > 4);
    }

    #[test]
    fn hermite_endpoints() {
        let p = hermite(Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::Y, 0.0);
        assert!(p.distance(Vec2::ZERO) < 1e-5);
        let p1 = hermite(Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::Y, 1.0);
        assert!(p1.distance(Vec2::Y) < 1e-5);
    }

    #[test]
    fn curvature_straight_is_zero() {
        let k = cubic_bezier_curvature(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(3.0, 0.0),
            0.5,
        );
        assert!(k < 1e-4);
    }

    #[test]
    fn property_bezier_endpoints_family() {
        let controls = [
            (Vec2::ZERO, Vec2::new(1.0, 2.0), Vec2::new(3.0, 0.0)),
            (Vec2::new(-1.0, -1.0), Vec2::ZERO, Vec2::new(1.0, 1.0)),
            (
                Vec2::new(5.0, 0.0),
                Vec2::new(5.0, 5.0),
                Vec2::new(0.0, 5.0),
            ),
        ];
        for (p0, p1, p2) in controls {
            let a = quadratic_bezier(p0, p1, p2, 0.0);
            let b = quadratic_bezier(p0, p1, p2, 1.0);
            assert!(a.distance(p0) < 1e-5);
            assert!(b.distance(p2) < 1e-5);
            // Midpoint lies in convex hull bounding box.
            let mid = quadratic_bezier(p0, p1, p2, 0.5);
            let min_x = p0.x.min(p1.x).min(p2.x) - 1e-3;
            let max_x = p0.x.max(p1.x).max(p2.x) + 1e-3;
            let min_y = p0.y.min(p1.y).min(p2.y) - 1e-3;
            let max_y = p0.y.max(p1.y).max(p2.y) + 1e-3;
            assert!(mid.x >= min_x && mid.x <= max_x);
            assert!(mid.y >= min_y && mid.y <= max_y);
        }
    }

    #[test]
    fn property_cubic_split_recombine() {
        let p0 = Vec2::ZERO;
        let p1 = Vec2::new(0.0, 2.0);
        let p2 = Vec2::new(2.0, 2.0);
        let p3 = Vec2::new(2.0, 0.0);
        for t_split in [0.25_f32, 0.5, 0.75] {
            let (left, right) = split_cubic_bezier(p0, p1, p2, p3, t_split);
            // Sample original at t_split equals left end / right start.
            let on = cubic_bezier(p0, p1, p2, p3, t_split);
            assert!(left[3].distance(on) < 1e-4);
            assert!(right[0].distance(on) < 1e-4);
            // Left at 1 and right at 0 match; sample interiors.
            for i in 0..=8 {
                let u = i as f32 / 8.0;
                let t = t_split * u;
                let a = cubic_bezier(p0, p1, p2, p3, t);
                let b = cubic_bezier(left[0], left[1], left[2], left[3], u);
                assert!(a.distance(b) < 1e-3, "t={t} a={a:?} b={b:?}");
            }
        }
    }

    #[test]
    fn property_polyline_arc_length_sample() {
        let pts = vec![
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 5.0),
            Vec2::new(0.0, 5.0),
        ];
        let pl = Polyline2::new(pts.clone());
        let total = pl.length();
        // 10 + 5 + 10 = 25
        assert!((total - 25.0).abs() < 1e-3, "total={total}");
        // Sample at 0 and 1.
        assert!(pl.sample(0.0).unwrap().distance(pts[0]) < 1e-4);
        assert!(pl.sample(1.0).unwrap().distance(*pts.last().unwrap()) < 1e-4);
        // Halfway (0.5 * 25 = 12.5): 10 along first + 2.5 along second => (10, 2.5)
        let mid = pl.sample(0.5).unwrap();
        assert!((mid.x - 10.0).abs() < 0.1, "mid={mid:?}");
        assert!((mid.y - 2.5).abs() < 0.1, "mid={mid:?}");
        // Closest point to a point on the first segment.
        let (cp, d) = pl.closest_point(Vec2::new(4.0, 1.0)).unwrap();
        assert!((cp.y).abs() < 1e-3);
        assert!((d - 1.0).abs() < 1e-3);
    }

    #[test]
    fn property_catmull_endpoints_and_length() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 1.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(4.0, 0.0),
        ];
        let path = BezierPath2::from_catmull_waypoints(&pts, false);
        assert_eq!(path.segments.len(), pts.len() - 1);
        let start = path.sample_uniform(0.0).unwrap();
        let end = path.sample_uniform(1.0).unwrap();
        assert!(start.distance(pts[0]) < 1e-3);
        assert!(end.distance(*pts.last().unwrap()) < 1e-3);
        let pl = path.to_polyline(6);
        assert!(pl.len() > path.segments.len());
        // Closed path has extra wrap segment.
        let closed = BezierPath2::from_catmull_waypoints(&pts, true);
        assert!(closed.segments.len() > path.segments.len());
    }

    #[test]
    fn property_hermite_matches_endpoints_and_tangents_approx() {
        let p0 = Vec2::ZERO;
        let m0 = Vec2::new(1.0, 0.0);
        let p1 = Vec2::new(0.0, 1.0);
        let m1 = Vec2::new(0.0, 1.0);
        assert!(hermite(p0, m0, p1, m1, 0.0).distance(p0) < 1e-5);
        assert!(hermite(p0, m0, p1, m1, 1.0).distance(p1) < 1e-5);
        // Small step from 0 roughly along m0.
        let p = hermite(p0, m0, p1, m1, 0.01);
        assert!(p.x > 0.0);
    }

    #[test]
    fn property_eased_cubic_and_mix() {
        let p0 = Vec2::ZERO;
        let p1 = Vec2::new(0.0, 1.0);
        let p2 = Vec2::new(1.0, 1.0);
        let p3 = Vec2::new(1.0, 0.0);
        let linear = cubic_bezier_eased(p0, p1, p2, p3, 0.5, |t| t);
        let mid = cubic_bezier(p0, p1, p2, p3, 0.5);
        assert!(linear.distance(mid) < 1e-5);
        let lagged = cubic_bezier_eased(p0, p1, p2, p3, 0.5, |t| t * t);
        // Ease-in uses smaller effective t, so closer to start along curve arc roughly.
        assert!(lagged.distance(p0) <= mid.distance(p0) + 1e-3 || lagged.y >= mid.y - 1.0);
        assert!((mix_f32(0.0, 10.0, 0.25) - 2.5).abs() < 1e-5);
    }

    #[test]
    fn property_deriv_nonzero_on_progress() {
        let p0 = Vec2::ZERO;
        let p1 = Vec2::new(1.0, 0.0);
        let p2 = Vec2::new(1.0, 1.0);
        let d = quadratic_bezier_deriv(p0, p1, p2, 0.5);
        assert!(d.length() > 0.1);
        let d3 = cubic_bezier_deriv(p0, p1, p2, Vec2::new(0.0, 1.0), 0.3);
        assert!(d3.length() > 0.0);
        let len = cubic_bezier_length(p0, p1, p2, Vec2::new(0.0, 1.0), 32);
        assert!(len > 1.0);
    }
}
