//! Dash movement helper: cooldown and i-frame windows.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Configuration for a dash.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashConfig {
    /// Distance covered over the dash duration.
    pub distance: f32,
    /// Duration of the dash movement (seconds).
    pub duration: f32,
    /// Cooldown before another dash (seconds, starts after dash ends).
    pub cooldown: f32,
    /// I-frame duration from dash start (seconds).
    pub iframe_secs: f32,
    /// Whether dash direction is normalized before use.
    pub normalize_dir: bool,
}

impl Default for DashConfig {
    fn default() -> Self {
        Self {
            distance: 80.0,
            duration: 0.15,
            cooldown: 0.6,
            iframe_secs: 0.12,
            normalize_dir: true,
        }
    }
}

/// Dash runtime state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DashState {
    /// Config.
    pub config: DashConfig,
    /// Remaining dash time (0 = not dashing).
    pub dash_left: f32,
    /// Remaining cooldown.
    pub cooldown_left: f32,
    /// Remaining i-frames.
    pub iframe_left: f32,
    /// Current dash direction (unit or raw).
    pub direction: Vec2,
    /// Speed used for this dash (distance / duration).
    pub speed: f32,
}

impl Default for DashState {
    fn default() -> Self {
        Self {
            config: DashConfig::default(),
            dash_left: 0.0,
            cooldown_left: 0.0,
            iframe_left: 0.0,
            direction: Vec2::ZERO,
            speed: 0.0,
        }
    }
}

impl DashState {
    /// Create with config.
    pub fn new(config: DashConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Whether currently dashing.
    pub fn is_dashing(&self) -> bool {
        self.dash_left > 0.0
    }

    /// Whether invulnerable from dash i-frames.
    pub fn is_invulnerable(&self) -> bool {
        self.iframe_left > 0.0
    }

    /// Whether a dash can start.
    pub fn can_dash(&self) -> bool {
        !self.is_dashing() && self.cooldown_left <= 0.0
    }

    /// Try to start a dash in `dir`. Returns false if on cooldown / already dashing / zero dir.
    pub fn try_dash(&mut self, dir: Vec2) -> bool {
        if !self.can_dash() {
            return false;
        }
        let mut d = dir;
        if self.config.normalize_dir {
            let len = d.length();
            if len < 1e-5 {
                return false;
            }
            d *= 1.0 / len;
        } else if d.length_squared() < 1e-8 {
            return false;
        }
        let dur = self.config.duration.max(1e-4);
        self.direction = d;
        self.speed = self.config.distance / dur;
        self.dash_left = dur;
        self.iframe_left = self.config.iframe_secs.min(dur + self.config.cooldown);
        true
    }

    /// Tick timers and return displacement for this frame (to add to position).
    pub fn tick(&mut self, dt: f32) -> Vec2 {
        let dt = dt.max(0.0);
        let mut delta = Vec2::ZERO;
        if self.dash_left > 0.0 {
            let step = dt.min(self.dash_left);
            delta = self.direction * self.speed * step;
            self.dash_left -= step;
            if self.dash_left <= 0.0 {
                self.dash_left = 0.0;
                self.cooldown_left = self.config.cooldown;
            }
        } else if self.cooldown_left > 0.0 {
            self.cooldown_left = (self.cooldown_left - dt).max(0.0);
        }
        if self.iframe_left > 0.0 {
            self.iframe_left = (self.iframe_left - dt).max(0.0);
        }
        delta
    }

    /// Cancel dash early (starts cooldown).
    pub fn cancel(&mut self) {
        if self.is_dashing() {
            self.dash_left = 0.0;
            self.cooldown_left = self.config.cooldown;
        }
    }

    /// Cooldown fraction remaining 0..=1.
    pub fn cooldown_fraction(&self) -> f32 {
        if self.config.cooldown <= 0.0 {
            return 0.0;
        }
        (self.cooldown_left / self.config.cooldown).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dash_moves_and_cools_down() {
        let mut d = DashState::new(DashConfig {
            distance: 100.0,
            duration: 0.2,
            cooldown: 0.5,
            iframe_secs: 0.1,
            normalize_dir: true,
        });
        assert!(d.try_dash(Vec2::X));
        assert!(d.is_dashing());
        assert!(d.is_invulnerable());
        let mut traveled = 0.0;
        for _ in 0..12 {
            let delta = d.tick(0.02);
            traveled += delta.length();
        }
        assert!((traveled - 100.0).abs() < 1.5);
        assert!(!d.is_dashing());
        assert!(!d.can_dash());
        // finish cooldown
        d.tick(0.6);
        assert!(d.can_dash());
    }

    #[test]
    fn reject_zero_dir_and_double() {
        let mut d = DashState::default();
        assert!(!d.try_dash(Vec2::ZERO));
        assert!(d.try_dash(Vec2::Y));
        assert!(!d.try_dash(Vec2::X));
    }

    #[test]
    fn cancel_starts_cooldown() {
        let mut d = DashState::default();
        d.try_dash(Vec2::X);
        d.cancel();
        assert!(!d.is_dashing());
        assert!(!d.can_dash());
    }

    #[test]
    fn iframes_expire() {
        let mut d = DashState::new(DashConfig {
            iframe_secs: 0.05,
            duration: 0.2,
            ..Default::default()
        });
        d.try_dash(Vec2::X);
        d.tick(0.06);
        assert!(!d.is_invulnerable());
        assert!(d.is_dashing());
    }

    #[test]
    fn diagonal_normalized_distance() {
        let mut d = DashState::new(DashConfig {
            distance: 50.0,
            duration: 0.1,
            cooldown: 0.0,
            iframe_secs: 0.0,
            normalize_dir: true,
        });
        assert!(d.try_dash(Vec2::new(1.0, 1.0)));
        let mut traveled = 0.0;
        for _ in 0..20 {
            let delta = d.tick(0.01);
            traveled += delta.length();
        }
        assert!((traveled - 50.0).abs() < 2.0, "traveled={traveled}");
    }

    #[test]
    fn cooldown_fraction_progresses() {
        let mut d = DashState::new(DashConfig {
            distance: 10.0,
            duration: 0.05,
            cooldown: 1.0,
            iframe_secs: 0.0,
            normalize_dir: true,
        });
        d.try_dash(Vec2::X);
        // Finish dash
        for _ in 0..10 {
            d.tick(0.02);
        }
        assert!(!d.can_dash());
        let f0 = d.cooldown_fraction();
        assert!(f0 > 0.0);
        d.tick(0.5);
        let f1 = d.cooldown_fraction();
        assert!(f1 < f0, "cooldown fraction did not decrease: {f0} -> {f1}");
        assert!(f1 <= 0.5 + f32::EPSILON);
        d.tick(1.0);
        assert!(d.can_dash());
        assert_eq!(d.cooldown_fraction(), 0.0);
    }

    #[test]
    fn non_normalized_keeps_magnitude_direction() {
        let mut d = DashState::new(DashConfig {
            distance: 20.0,
            duration: 0.1,
            cooldown: 0.0,
            iframe_secs: 0.0,
            normalize_dir: false,
        });
        assert!(d.try_dash(Vec2::new(2.0, 0.0)));
        assert!(d.is_dashing());
        assert_eq!(d.direction, Vec2::new(2.0, 0.0));
        let delta = d.tick(0.1);
        assert!((delta.x - 40.0).abs() < 1e-4, "delta={delta:?}");
        assert!(delta.y.abs() < 1e-6);
    }
}
