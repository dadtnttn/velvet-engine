//! Curved motion paths: Catmull-Rom chains with eased clocks.
//!
//! [`sample_path`] moves through arbitrary waypoints (endpoint-duplicated
//! Catmull-Rom, linear for two points); [`PathTween`] adds duration, ease
//! and completion, and [`AnimDirector::move_along`] drives a target pose
//! along one. Pure math — same points, same positions.

use serde::{Deserialize, Serialize};
use velvet_math::{catmull_rom, Ease, Vec2};

/// Sample a waypoint chain at `t` 0..=1 (evenly split segments).
///
/// Returns None for an empty chain, the single point for one waypoint,
/// and a straight lerp for two.
pub fn sample_path(points: &[Vec2], t: f32) -> Option<Vec2> {
    match points.len() {
        0 => None,
        1 => Some(points[0]),
        2 => {
            let t = t.clamp(0.0, 1.0);
            Some(Vec2::new(
                points[0].x + (points[1].x - points[0].x) * t,
                points[0].y + (points[1].y - points[0].y) * t,
            ))
        }
        n => {
            let t = t.clamp(0.0, 1.0);
            let segs = (n - 1) as f32;
            let pos = (t * segs).min(segs - 1e-6);
            let seg = pos.floor() as usize;
            let u = pos - seg as f32;
            let at = |i: usize| points[i.clamp(0, n - 1)];
            Some(catmull_rom(
                at(seg.saturating_sub(1)),
                at(seg),
                at(seg + 1),
                at(seg + 2),
                u,
            ))
        }
    }
}

/// Eased traversal of a waypoint chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathTween {
    /// Waypoints.
    pub points: Vec<Vec2>,
    /// Duration seconds.
    pub duration: f32,
    /// Elapsed seconds.
    pub elapsed: f32,
    /// Ease over normalized progress.
    pub ease: Ease,
    /// Finished flag.
    pub finished: bool,
}

impl PathTween {
    /// New traversal (empty chains finish immediately).
    pub fn new(points: Vec<Vec2>, duration: f32, ease: Ease) -> Self {
        let finished = points.len() < 2;
        Self {
            points,
            duration: duration.max(1e-4),
            elapsed: 0.0,
            ease,
            finished,
        }
    }

    /// Normalized progress 0..=1.
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Advance; returns the position (start point before the first tick).
    pub fn tick(&mut self, dt: f32) -> Option<Vec2> {
        if !self.finished {
            self.elapsed += dt.max(0.0);
            if self.elapsed >= self.duration {
                self.elapsed = self.duration;
                self.finished = true;
            }
        }
        sample_path(&self.points, self.ease.eval(self.progress()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_midpoint() {
        let pts = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        let near = |got: Option<Vec2>, want: Vec2| {
            let got = got.unwrap();
            assert!(
                (got.x - want.x).abs() < 1e-3 && (got.y - want.y).abs() < 1e-3,
                "{got:?} != {want:?}"
            );
        };
        near(sample_path(&pts, 0.0), Vec2::new(0.0, 0.0));
        near(sample_path(&pts, 1.0), Vec2::new(0.0, 10.0));
        // Segment joints land on waypoints.
        near(sample_path(&pts, 1.0 / 3.0), Vec2::new(10.0, 0.0));
        // Degenerate inputs never panic.
        assert_eq!(sample_path(&[], 0.5), None);
        assert_eq!(
            sample_path(&[Vec2::new(3.0, 4.0)], 0.9),
            Some(Vec2::new(3.0, 4.0))
        );
    }

    #[test]
    fn tween_finishes_at_end() {
        let mut tween = PathTween::new(vec![Vec2::ZERO, Vec2::new(8.0, 0.0)], 1.0, Ease::Linear);
        assert_eq!(tween.tick(0.0), Some(Vec2::ZERO));
        let mid = tween.tick(0.5).unwrap();
        assert!((mid.x - 4.0).abs() < 1e-4);
        let end = tween.tick(1.0).unwrap();
        assert!((end.x - 8.0).abs() < 1e-4);
        assert!(tween.finished);
    }
}
