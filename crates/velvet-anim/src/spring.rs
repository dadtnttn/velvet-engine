//! Damped springs: smooth follow-through without keyframes.
//!
//! A [`Spring`] integrates `a = -k·(x − target) − c·v` with semi-implicit
//! Euler in fixed substeps (≤ 1/240 s), so even stiff springs stay stable
//! at 60 fps frame times. Retarget mid-flight for UI follow, camera
//! lag, or jelly selections. Deterministic for fixed `dt` sequences.

use serde::{Deserialize, Serialize};

/// One-dimensional damped spring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring {
    /// Current value.
    pub value: f32,
    /// Current velocity.
    pub velocity: f32,
    /// Rest target.
    pub target: f32,
    /// Stiffness k (≥ 0).
    pub stiffness: f32,
    /// Damping c (≥ 0).
    pub damping: f32,
}

impl Spring {
    /// Largest stable-feeling stiffness (higher needs smaller frames;
    /// substeps cover up to this at 60 fps).
    pub const MAX_STIFFNESS: f32 = 500.0;

    /// New spring at rest unless told otherwise.
    pub fn new(value: f32, target: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
            target,
            stiffness: stiffness.clamp(0.0, Self::MAX_STIFFNESS),
            damping: damping.max(0.0),
        }
    }

    /// Retarget (velocity carries over).
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Snap both value and velocity.
    pub fn snap(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
    }

    /// Advance; returns the new value.
    pub fn tick(&mut self, dt: f32) -> f32 {
        let dt = dt.max(0.0);
        if dt == 0.0 {
            return self.value;
        }
        // Fixed substeps keep stiff springs stable at any frame rate.
        let steps = ((dt / (1.0 / 240.0)).ceil() as usize).max(1);
        let h = dt / steps as f32;
        for _ in 0..steps {
            let accel = -self.stiffness * (self.value - self.target) - self.damping * self.velocity;
            self.velocity += accel * h;
            self.value += self.velocity * h;
        }
        self.value
    }

    /// Settled within tolerances.
    pub fn settled(&self, pos_tol: f32, vel_tol: f32) -> bool {
        (self.value - self.target).abs() <= pos_tol && self.velocity.abs() <= vel_tol
    }
}

/// Critical damping for a stiffness (`c = 2·√k`, no overshoot).
pub fn critical_damping(stiffness: f32) -> f32 {
    2.0 * stiffness.max(0.0).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_to_target() {
        let mut s = Spring::new(0.0, 10.0, 60.0, critical_damping(60.0));
        for _ in 0..120 {
            s.tick(1.0 / 60.0);
        }
        assert!((s.value - 10.0).abs() < 0.05, "{}", s.value);
        assert!(s.settled(0.05, 0.5));
    }

    #[test]
    fn stiff_spring_stays_finite() {
        let mut s = Spring::new(0.0, 1.0, Spring::MAX_STIFFNESS, 8.0);
        for _ in 0..600 {
            s.tick(1.0 / 60.0);
        }
        assert!(s.value.is_finite());
        assert!(s.velocity.is_finite());
        // Bounded overshoot: never flings past twice the target here.
        assert!(s.value.abs() < 2.5, "{}", s.value);
    }

    #[test]
    fn retarget_mid_flight() {
        let mut s = Spring::new(0.0, 10.0, 60.0, critical_damping(60.0));
        for _ in 0..30 {
            s.tick(1.0 / 60.0);
        }
        assert!(s.value > 1.0);
        s.set_target(-5.0);
        for _ in 0..180 {
            s.tick(1.0 / 60.0);
        }
        assert!((s.value + 5.0).abs() < 0.05, "{}", s.value);
    }

    #[test]
    fn zero_stiffness_holds_still() {
        let mut s = Spring::new(3.0, 99.0, 0.0, 0.0);
        s.tick(1.0);
        assert_eq!(s.value, 3.0);
        // …and therefore honestly reports "not settled" (never arrives).
        assert!(!s.settled(0.01, 0.01));
    }
}
