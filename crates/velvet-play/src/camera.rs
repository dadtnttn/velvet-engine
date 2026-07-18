//! Gameplay camera follow and bounds.

use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

/// Play camera state (CPU; render Camera2D can mirror this).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayCamera {
    /// World center.
    pub position: Vec2,
    /// Zoom (1 = default).
    pub zoom: f32,
    /// Viewport size in world units.
    pub viewport: Vec2,
    /// Optional hard bounds.
    pub bounds: Option<CameraBounds>,
    /// Shake offset residual.
    pub shake: Vec2,
}

impl Default for PlayCamera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            viewport: Vec2::new(320.0, 180.0),
            bounds: None,
            shake: Vec2::ZERO,
        }
    }
}

impl PlayCamera {
    /// Visible world rect.
    pub fn visible_rect(&self) -> Rect {
        let half = self.viewport * (0.5 / self.zoom.max(1e-6));
        Rect::from_center_half_size(self.position + self.shake, half)
    }

    /// Clamp position to bounds.
    pub fn clamp_to_bounds(&mut self) {
        if let Some(b) = self.bounds {
            let half = self.viewport * (0.5 / self.zoom.max(1e-6));
            let min = b.rect.min + half;
            let max = b.rect.max - half;
            self.position.x = self.position.x.clamp(min.x.min(max.x), max.x.max(min.x));
            self.position.y = self.position.y.clamp(min.y.min(max.y), max.y.max(min.y));
        }
    }

    /// Apply decaying shake.
    pub fn tick_shake(&mut self, dt: f32) {
        self.shake *= (1.0 - 8.0 * dt).clamp(0.0, 1.0);
        if self.shake.length_squared() < 0.01 {
            self.shake = Vec2::ZERO;
        }
    }

    /// Add impulse shake.
    pub fn add_shake(&mut self, amount: Vec2) {
        self.shake += amount;
    }
}

/// Camera world bounds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraBounds {
    /// Allowed area.
    pub rect: Rect,
}

/// Follow target with lerp.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera2dFollow {
    /// Lerp factor per second (higher = snappier).
    pub stiffness: f32,
    /// Look-ahead along velocity.
    pub look_ahead: f32,
    /// Deadzone half-extents (camera doesn't move if target inside).
    pub deadzone: Vec2,
}

impl Default for Camera2dFollow {
    fn default() -> Self {
        Self {
            stiffness: 6.0,
            look_ahead: 0.15,
            deadzone: Vec2::ZERO,
        }
    }
}

impl Camera2dFollow {
    /// Update camera toward target.
    pub fn update(&self, camera: &mut PlayCamera, target: Vec2, target_vel: Vec2, dt: f32) {
        let desired = target + target_vel * self.look_ahead;
        let delta = desired - camera.position;
        if delta.x.abs() > self.deadzone.x || delta.y.abs() > self.deadzone.y {
            let t = (self.stiffness * dt).clamp(0.0, 1.0);
            camera.position = camera.position.lerp(desired, t);
        }
        camera.clamp_to_bounds();
        camera.tick_shake(dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follow_moves_toward_target() {
        let mut cam = PlayCamera::default();
        let follow = Camera2dFollow {
            stiffness: 100.0,
            look_ahead: 0.0,
            deadzone: Vec2::ZERO,
        };
        follow.update(&mut cam, Vec2::new(100.0, 0.0), Vec2::ZERO, 1.0);
        assert!(cam.position.x > 50.0);
    }

    #[test]
    fn bounds_clamp() {
        let mut cam = PlayCamera {
            position: Vec2::new(1000.0, 0.0),
            viewport: Vec2::new(100.0, 100.0),
            bounds: Some(CameraBounds {
                rect: Rect::from_pos_size(Vec2::ZERO, Vec2::new(200.0, 200.0)),
            }),
            ..Default::default()
        };
        cam.clamp_to_bounds();
        assert!(cam.position.x <= 150.0);
    }
}
