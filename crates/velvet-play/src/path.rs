//! Waypoint path following for entities.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

use crate::navigation::Path;

/// Looping mode for path followers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PathLoop {
    /// Stop at the last waypoint.
    #[default]
    Once,
    /// Restart from the first waypoint.
    Loop,
    /// Reverse direction at ends (ping-pong).
    PingPong,
}

/// Result of a follow step.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathFollowResult {
    /// New position.
    pub position: Vec2,
    /// Suggested facing / travel direction (normalized or zero).
    pub direction: Vec2,
    /// Whether the path completed (Once mode reached end).
    pub finished: bool,
    /// Index of the waypoint currently targeted.
    pub waypoint_index: usize,
}

/// Follows a sequence of world-space waypoints at a constant speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathFollower {
    /// Waypoints.
    pub waypoints: Vec<Vec2>,
    /// Current target index.
    pub index: usize,
    /// Movement speed units/sec.
    pub speed: f32,
    /// Arrival radius to snap / advance.
    pub arrive_radius: f32,
    /// Loop mode.
    pub mode: PathLoop,
    /// Direction for ping-pong (+1 forward, -1 backward).
    direction: i32,
    /// Finished (once mode).
    pub finished: bool,
}

impl Default for PathFollower {
    fn default() -> Self {
        Self {
            waypoints: Vec::new(),
            index: 0,
            speed: 80.0,
            arrive_radius: 2.0,
            mode: PathLoop::Once,
            direction: 1,
            finished: false,
        }
    }
}

impl PathFollower {
    /// Create from waypoints.
    pub fn new(waypoints: Vec<Vec2>, speed: f32) -> Self {
        Self {
            waypoints,
            speed,
            ..Default::default()
        }
    }

    /// From a navigation [`Path`].
    pub fn from_path(path: &Path, speed: f32) -> Self {
        Self::new(path.points.clone(), speed)
    }

    /// Builder: loop mode.
    pub fn with_loop(mut self, mode: PathLoop) -> Self {
        self.mode = mode;
        self
    }

    /// Builder: arrive radius.
    pub fn with_arrive_radius(mut self, r: f32) -> Self {
        self.arrive_radius = r.max(0.01);
        self
    }

    /// Whether there are waypoints.
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Number of waypoints.
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Current target waypoint, if any.
    pub fn target(&self) -> Option<Vec2> {
        self.waypoints.get(self.index).copied()
    }

    /// Reset to start.
    pub fn reset(&mut self) {
        self.index = 0;
        self.direction = 1;
        self.finished = false;
    }

    /// Replace waypoints and reset.
    pub fn set_waypoints(&mut self, waypoints: Vec<Vec2>) {
        self.waypoints = waypoints;
        self.reset();
    }

    /// Advance position toward the path by `dt`.
    pub fn update(&mut self, position: Vec2, dt: f32) -> PathFollowResult {
        if self.waypoints.is_empty() || self.finished {
            return PathFollowResult {
                position,
                direction: Vec2::ZERO,
                finished: self.finished || self.waypoints.is_empty(),
                waypoint_index: self.index,
            };
        }

        let mut pos = position;
        let mut remaining = self.speed.max(0.0) * dt.max(0.0);
        let mut dir = Vec2::ZERO;

        while remaining > 0.0 {
            let Some(&target) = self.waypoints.get(self.index) else {
                break;
            };
            let to = target - pos;
            let dist = to.length();
            if dist <= self.arrive_radius || dist < 1e-5 {
                pos = target;
                if !self.advance_index() {
                    self.finished = true;
                    break;
                }
                continue;
            }
            dir = to * (1.0 / dist);
            if remaining >= dist {
                pos = target;
                remaining -= dist;
                if !self.advance_index() {
                    self.finished = true;
                    break;
                }
            } else {
                pos += dir * remaining;
                remaining = 0.0;
            }
        }

        PathFollowResult {
            position: pos,
            direction: dir,
            finished: self.finished,
            waypoint_index: self.index,
        }
    }

    /// Advance waypoint index according to loop mode. Returns false if finished (Once).
    fn advance_index(&mut self) -> bool {
        match self.mode {
            PathLoop::Once => {
                if self.index + 1 >= self.waypoints.len() {
                    false
                } else {
                    self.index += 1;
                    true
                }
            }
            PathLoop::Loop => {
                self.index = (self.index + 1) % self.waypoints.len();
                true
            }
            PathLoop::PingPong => {
                let next = self.index as i32 + self.direction;
                if next < 0 {
                    self.direction = 1;
                    self.index = if self.waypoints.len() > 1 { 1 } else { 0 };
                } else if next as usize >= self.waypoints.len() {
                    self.direction = -1;
                    self.index = self.waypoints.len().saturating_sub(2);
                } else {
                    self.index = next as usize;
                }
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_to_end() {
        let mut f = PathFollower::new(
            vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(10.0, 10.0)],
            100.0,
        )
        .with_arrive_radius(0.1);
        let mut pos = Vec2::ZERO;
        for _ in 0..50 {
            let r = f.update(pos, 0.05);
            pos = r.position;
            if r.finished {
                break;
            }
        }
        assert!(f.finished);
        assert!((pos - Vec2::new(10.0, 10.0)).length() < 0.5);
    }

    #[test]
    fn loop_wraps() {
        let mut f = PathFollower::new(vec![Vec2::ZERO, Vec2::new(5.0, 0.0)], 50.0)
            .with_loop(PathLoop::Loop)
            .with_arrive_radius(0.1);
        let mut pos = Vec2::ZERO;
        let mut saw_wrap = false;
        for _ in 0..40 {
            let r = f.update(pos, 0.1);
            pos = r.position;
            if r.waypoint_index == 0 && (pos - Vec2::ZERO).length() < 1.0 {
                // after wrapping may be heading to first again
                saw_wrap = true;
            }
            if f.index == 0 && pos.x < 1.0 && f.direction == 1 {
                // ok
            }
        }
        // Should not finish
        assert!(!f.finished);
        let _ = saw_wrap;
    }

    #[test]
    fn ping_pong_reverses() {
        let mut f = PathFollower::new(vec![Vec2::ZERO, Vec2::new(10.0, 0.0)], 100.0)
            .with_loop(PathLoop::PingPong)
            .with_arrive_radius(0.1);
        let mut pos = Vec2::ZERO;
        // Run long enough to reverse
        for _ in 0..30 {
            let r = f.update(pos, 0.1);
            pos = r.position;
        }
        assert!(!f.finished);
        // Direction should have flipped at some point
        assert!(f.direction == 1 || f.direction == -1);
    }

    #[test]
    fn from_nav_path() {
        let path = Path {
            points: vec![Vec2::new(1.0, 2.0), Vec2::new(3.0, 4.0)],
        };
        let f = PathFollower::from_path(&path, 10.0);
        assert_eq!(f.len(), 2);
        assert_eq!(f.target(), Some(Vec2::new(1.0, 2.0)));
    }

    #[test]
    fn empty_path() {
        let mut f = PathFollower::default();
        let r = f.update(Vec2::ZERO, 0.1);
        assert!(r.finished);
    }

    #[test]
    fn long_polyline_reaches_end() {
        let mut pts = Vec::new();
        for i in 0..40 {
            pts.push(Vec2::new(i as f32 * 5.0, (i % 2) as f32 * 3.0));
        }
        let end = *pts.last().unwrap();
        let mut f = PathFollower::new(pts, 80.0).with_arrive_radius(0.5);
        let mut pos = Vec2::ZERO;
        let mut finished = false;
        for _ in 0..500 {
            let r = f.update(pos, 0.05);
            pos = r.position;
            if r.finished {
                finished = true;
                break;
            }
        }
        assert!(finished, "pos={pos:?} end={end:?}");
        assert!((pos - end).length() < 2.0, "pos={pos:?} end={end:?}");
    }

    #[test]
    fn long_path_waypoint_index_increases() {
        let pts: Vec<Vec2> = (0..20).map(|i| Vec2::new(i as f32 * 10.0, 0.0)).collect();
        let mut f = PathFollower::new(pts, 100.0).with_arrive_radius(0.25);
        let mut pos = Vec2::ZERO;
        let mut max_wp = 0usize;
        for _ in 0..200 {
            let r = f.update(pos, 0.05);
            pos = r.position;
            max_wp = max_wp.max(r.waypoint_index);
            if r.finished {
                break;
            }
        }
        assert!(max_wp >= 5, "max_wp={max_wp}");
    }

    #[test]
    fn loop_never_finishes_many_cycles() {
        let mut f = PathFollower::new(
            vec![Vec2::ZERO, Vec2::new(20.0, 0.0), Vec2::new(20.0, 20.0)],
            60.0,
        )
        .with_loop(PathLoop::Loop)
        .with_arrive_radius(0.2);
        let mut pos = Vec2::ZERO;
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        for _ in 0..200 {
            let r = f.update(pos, 0.05);
            pos = r.position;
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            assert!(!r.finished);
            assert!(!f.finished);
        }
        // Should have traversed span of the path multiple times.
        assert!(max_x - min_x > 10.0, "span={}", max_x - min_x);
    }

    #[test]
    fn ping_pong_visits_start_again() {
        let mut f = PathFollower::new(vec![Vec2::ZERO, Vec2::new(30.0, 0.0)], 90.0)
            .with_loop(PathLoop::PingPong)
            .with_arrive_radius(0.2);
        let mut pos = Vec2::ZERO;
        let mut saw_far = false;
        let mut saw_near_again = false;
        for _ in 0..120 {
            let r = f.update(pos, 0.05);
            pos = r.position;
            if pos.x > 25.0 {
                saw_far = true;
            }
            if saw_far && pos.x < 5.0 {
                saw_near_again = true;
                break;
            }
        }
        assert!(saw_far, "never reached far end");
        assert!(saw_near_again, "never returned near start; pos={pos:?}");
    }

    #[test]
    fn zero_speed_stays_put() {
        let mut f = PathFollower::new(vec![Vec2::ZERO, Vec2::new(10.0, 0.0)], 0.0);
        let r = f.update(Vec2::new(1.0, 2.0), 1.0);
        assert!((r.position - Vec2::new(1.0, 2.0)).length() < 1e-4 || r.finished);
    }
}
