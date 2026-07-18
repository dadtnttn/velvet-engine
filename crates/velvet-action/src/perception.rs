//! Vision and hearing.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Perception parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PerceptionConfig {
    /// View distance.
    pub view_distance: f32,
    /// Half FOV radians.
    pub fov_half: f32,
    /// Hearing radius.
    pub hear_radius: f32,
}

impl Default for PerceptionConfig {
    fn default() -> Self {
        Self {
            view_distance: 160.0,
            fov_half: std::f32::consts::FRAC_PI_3,
            hear_radius: 80.0,
        }
    }
}

/// Perception component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Perception {
    /// Config.
    pub config: PerceptionConfig,
    /// Facing direction.
    pub facing: Vec2,
    /// Last known target position.
    pub last_seen: Option<Vec2>,
    /// Alert level 0..=1.
    pub alert: f32,
}

impl Default for Perception {
    fn default() -> Self {
        Self {
            config: PerceptionConfig::default(),
            facing: Vec2::Y,
            last_seen: None,
            alert: 0.0,
        }
    }
}

/// Whether observer can see target (no occlusion — host supplies clear LOS flag).
pub fn see_target(
    observer: Vec2,
    facing: Vec2,
    target: Vec2,
    cfg: PerceptionConfig,
    line_clear: bool,
) -> bool {
    if !line_clear {
        return false;
    }
    let offset = target - observer;
    let dist = offset.length();
    if dist > cfg.view_distance || dist < 1e-4 {
        return false;
    }
    let dir = offset * (1.0 / dist);
    let face = facing.normalize_or_zero();
    let angle = face.dot(dir).clamp(-1.0, 1.0).acos();
    angle <= cfg.fov_half
}

/// Hearing check (distance only).
pub fn hear(observer: Vec2, sound: Vec2, cfg: PerceptionConfig) -> bool {
    (sound - observer).length() <= cfg.hear_radius
}

impl Perception {
    /// Update from senses.
    pub fn sense(&mut self, self_pos: Vec2, target: Option<Vec2>, line_clear: bool, loud: bool) {
        let mut saw = false;
        if let Some(t) = target {
            if see_target(self_pos, self.facing, t, self.config, line_clear) {
                self.last_seen = Some(t);
                self.alert = 1.0;
                saw = true;
            } else if loud && hear(self_pos, t, self.config) {
                self.last_seen = Some(t);
                self.alert = (self.alert + 0.4).min(1.0);
            }
        }
        if !saw {
            self.alert = (self.alert - 0.05).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fov_and_hear() {
        let cfg = PerceptionConfig::default();
        assert!(see_target(
            Vec2::ZERO,
            Vec2::Y,
            Vec2::new(0.0, 50.0),
            cfg,
            true
        ));
        assert!(!see_target(
            Vec2::ZERO,
            Vec2::Y,
            Vec2::new(100.0, 0.0),
            cfg,
            true
        ));
        assert!(hear(Vec2::ZERO, Vec2::new(40.0, 0.0), cfg));
    }
}
