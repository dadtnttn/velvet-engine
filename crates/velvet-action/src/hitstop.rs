//! Hitstop (hit freeze) timer for impact juice.

use serde::{Deserialize, Serialize};

/// Hitstop configuration scales.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HitstopConfig {
    /// Base freeze duration for a normal hit (seconds).
    pub base_secs: f32,
    /// Extra seconds per point of damage (scaled).
    pub secs_per_damage: f32,
    /// Maximum freeze duration.
    pub max_secs: f32,
    /// Scale applied when the hit is a critical.
    pub crit_mul: f32,
    /// Time scale during hitstop (0 = full freeze; 0.1 = heavy slow-mo).
    pub time_scale: f32,
}

impl Default for HitstopConfig {
    fn default() -> Self {
        Self {
            base_secs: 0.04,
            secs_per_damage: 0.001,
            max_secs: 0.2,
            crit_mul: 1.5,
            time_scale: 0.0,
        }
    }
}

/// Hitstop / freeze-frame controller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hitstop {
    /// Config.
    pub config: HitstopConfig,
    /// Remaining freeze time (real seconds, not scaled).
    remaining: f32,
    /// Queued duration to apply next (for stacking policy).
    queued: f32,
    /// Whether new hits refresh duration (true) or only extend if longer.
    pub refresh_on_hit: bool,
}

impl Default for Hitstop {
    fn default() -> Self {
        Self {
            config: HitstopConfig::default(),
            remaining: 0.0,
            queued: 0.0,
            refresh_on_hit: true,
        }
    }
}

impl Hitstop {
    /// Create with config.
    pub fn new(config: HitstopConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// Whether freeze is active.
    pub fn is_active(&self) -> bool {
        self.remaining > 0.0
    }

    /// Remaining seconds.
    pub fn remaining(&self) -> f32 {
        self.remaining
    }

    /// Current time scale for gameplay systems (1.0 when inactive).
    pub fn time_scale(&self) -> f32 {
        if self.is_active() {
            self.config.time_scale
        } else {
            1.0
        }
    }

    /// Compute duration from damage.
    pub fn duration_for_damage(&self, damage: f32, crit: bool) -> f32 {
        let mut d = self.config.base_secs + damage.max(0.0) * self.config.secs_per_damage;
        if crit {
            d *= self.config.crit_mul;
        }
        d.min(self.config.max_secs)
    }

    /// Trigger hitstop from a hit.
    pub fn trigger_hit(&mut self, damage: f32, crit: bool) {
        let d = self.duration_for_damage(damage, crit);
        self.trigger(d);
    }

    /// Trigger an explicit duration.
    pub fn trigger(&mut self, secs: f32) {
        let secs = secs.clamp(0.0, self.config.max_secs);
        if self.refresh_on_hit {
            self.remaining = self.remaining.max(secs);
        } else if secs > self.remaining {
            self.remaining = secs;
        }
    }

    /// Queue hitstop to apply at end of frame (optional host policy).
    pub fn queue(&mut self, secs: f32) {
        self.queued = self.queued.max(secs.clamp(0.0, self.config.max_secs));
    }

    /// Apply any queued duration.
    pub fn flush_queue(&mut self) {
        if self.queued > 0.0 {
            self.trigger(self.queued);
            self.queued = 0.0;
        }
    }

    /// Tick real time. Returns the scaled `dt` that simulation should use.
    ///
    /// When active with `time_scale == 0`, returns 0.0 (full freeze).
    pub fn tick(&mut self, dt: f32) -> f32 {
        let dt = dt.max(0.0);
        if self.remaining > 0.0 {
            self.remaining = (self.remaining - dt).max(0.0);
            dt * self.config.time_scale
        } else {
            dt
        }
    }

    /// Cancel immediately.
    pub fn clear(&mut self) {
        self.remaining = 0.0;
        self.queued = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freezes_then_resumes() {
        let mut h = Hitstop::default();
        h.trigger(0.1);
        assert!(h.is_active());
        assert_eq!(h.tick(0.05), 0.0);
        assert!(h.is_active());
        let _ = h.tick(0.1);
        assert!(!h.is_active());
        assert!((h.tick(0.016) - 0.016).abs() < 1e-6);
    }

    #[test]
    fn damage_scales_duration() {
        let h = Hitstop::default();
        let d0 = h.duration_for_damage(0.0, false);
        let d1 = h.duration_for_damage(50.0, false);
        let dc = h.duration_for_damage(50.0, true);
        assert!(d1 > d0);
        assert!(dc > d1);
        assert!(dc <= h.config.max_secs + 1e-5);
    }

    #[test]
    fn refresh_takes_max() {
        let mut h = Hitstop::default();
        h.trigger(0.05);
        h.trigger(0.08);
        assert!((h.remaining() - 0.08).abs() < 1e-5);
    }

    #[test]
    fn queue_flush() {
        let mut h = Hitstop::default();
        h.queue(0.12);
        assert!(!h.is_active());
        h.flush_queue();
        assert!(h.is_active());
    }

    #[test]
    fn slow_mo_scale() {
        let mut h = Hitstop::new(HitstopConfig {
            time_scale: 0.25,
            ..Default::default()
        });
        h.trigger(1.0);
        let scaled = h.tick(0.04);
        assert!((scaled - 0.01).abs() < 1e-5);
    }

    #[test]
    fn clear() {
        let mut h = Hitstop::default();
        h.trigger_hit(100.0, true);
        h.clear();
        assert!(!h.is_active());
        assert_eq!(h.time_scale(), 1.0);
    }
}
