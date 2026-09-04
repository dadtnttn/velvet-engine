//! Runtime particle (canvas-space, not grid).

use serde::{Deserialize, Serialize};

/// Live particle instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxParticle {
    /// Spawn time (global).
    pub spawn_time: f32,
    /// Lifetime seconds.
    pub lifetime: f32,
    /// Start position (canvas px, origin at center).
    pub x0: f32,
    /// Start y.
    pub y0: f32,
    /// Velocity px/s.
    pub vx: f32,
    /// Velocity y.
    pub vy: f32,
    /// Phase jitter (random 0..=1 for flicker/offset).
    pub phase: f32,
    /// Palette t at spawn (random offset 0..=0.2 for variation)
    pub pal_off: f32,
    /// Alive flag.
    pub alive: bool,
}

impl FxParticle {
    /// Create.
    pub fn new(
        spawn_time: f32,
        lifetime: f32,
        x0: f32,
        y0: f32,
        vx: f32,
        vy: f32,
        phase: f32,
    ) -> Self {
        Self {
            spawn_time,
            lifetime,
            x0,
            y0,
            vx,
            vy,
            phase,
            pal_off: 0.0,
            alive: true,
        }
    }

    /// Age at global time `now`.
    pub fn age(&self, now: f32) -> f32 {
        (now - self.spawn_time).max(0.0)
    }

    /// Normalized life t 0..=1 (0 birth, 1 death).
    pub fn t(&self, now: f32) -> f32 {
        (self.age(now) / self.lifetime.max(1e-4)).clamp(0.0, 1.0)
    }

    /// Position at time now with gravity/drag applied analytically per frame (coarse but stable).
    /// Actual integration is done by `FxSim` tick; this is helper for sampling if needed.
    pub fn pos(&self, now: f32, gravity: f32, drag: f32, jitter: f32) -> (f32, f32) {
        let dt = self.age(now);
        let drag_f = (1.0 - drag * 0.06).clamp(0.0, 1.0).powf(dt * 20.0);
        let jx = (self.phase * std::f32::consts::TAU + now * 9.0).sin() * jitter * dt * 0.5;
        let jy = (self.phase * 12.0 + now * 7.0).cos() * jitter * dt * 0.5;
        let x = self.x0 + self.vx * dt * drag_f + jx;
        let y = self.y0 + self.vy * dt * drag_f + 0.5 * gravity * dt * dt + jy;
        (x, y)
    }
}
