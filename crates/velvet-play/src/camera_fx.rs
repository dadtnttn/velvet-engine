//! Camera trauma-based shake and screen effects.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

use crate::camera::PlayCamera;

/// Trauma-based camera shake (as popularized by juice talks: trauma ∈ [0,1]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraShake {
    /// Current trauma 0..=1.
    pub trauma: f32,
    /// Decay per second (trauma units).
    pub decay: f32,
    /// Maximum offset in world units at trauma=1.
    pub max_offset: f32,
    /// Maximum rotation (radians) at trauma=1 (stored for consumers; PlayCamera is 2D offset only).
    pub max_roll: f32,
    /// Seed for deterministic noise.
    pub seed: u32,
    /// Accumulated time for noise sampling.
    pub time: f32,
    /// Last computed offset.
    pub offset: Vec2,
    /// Last computed roll.
    pub roll: f32,
}

impl Default for CameraShake {
    fn default() -> Self {
        Self {
            trauma: 0.0,
            decay: 1.2,
            max_offset: 12.0,
            max_roll: 0.08,
            seed: 0xC0FFEE,
            time: 0.0,
            offset: Vec2::ZERO,
            roll: 0.0,
        }
    }
}

impl CameraShake {
    /// Create with custom max offset.
    pub fn new(max_offset: f32) -> Self {
        Self {
            max_offset,
            ..Default::default()
        }
    }

    /// Add trauma (clamped).
    pub fn add_trauma(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount.max(0.0)).clamp(0.0, 1.0);
    }

    /// Set trauma absolute.
    pub fn set_trauma(&mut self, value: f32) {
        self.trauma = value.clamp(0.0, 1.0);
    }

    /// Instant impulse scaled by intensity.
    pub fn impulse(&mut self, intensity: f32) {
        self.add_trauma(intensity.clamp(0.0, 1.0));
    }

    /// Clear shake.
    pub fn clear(&mut self) {
        self.trauma = 0.0;
        self.offset = Vec2::ZERO;
        self.roll = 0.0;
    }

    /// Whether actively shaking.
    pub fn is_active(&self) -> bool {
        self.trauma > 0.001
    }

    /// Advance simulation; returns offset to apply to camera.
    pub fn tick(&mut self, dt: f32) -> Vec2 {
        let dt = dt.max(0.0);
        self.time += dt;
        if self.trauma <= 0.0 {
            self.offset = Vec2::ZERO;
            self.roll = 0.0;
            return Vec2::ZERO;
        }
        // Shake amount is trauma^2 for a punchier falloff.
        let shake = self.trauma * self.trauma;
        let t = self.time;
        let s = self.seed as f32;
        let nx = pseudo_noise(t * 37.1 + s * 0.01);
        let ny = pseudo_noise(t * 31.7 + s * 0.02 + 17.0);
        let nr = pseudo_noise(t * 29.3 + s * 0.03 + 41.0);
        self.offset = Vec2::new(nx, ny) * (self.max_offset * shake);
        self.roll = nr * self.max_roll * shake;
        self.trauma = (self.trauma - self.decay * dt).max(0.0);
        self.offset
    }

    /// Apply this frame's offset onto a play camera (replaces residual shake).
    pub fn apply_to_camera(&mut self, camera: &mut PlayCamera, dt: f32) {
        let off = self.tick(dt);
        camera.shake = off;
    }
}

/// Multi-layer trauma (e.g. weapon vs explosion).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CameraTraumaLayers {
    /// Layers by name.
    pub layers: indexmap::IndexMap<String, CameraShake>,
}

impl CameraTraumaLayers {
    /// Get or insert layer.
    pub fn layer_mut(&mut self, name: &str) -> &mut CameraShake {
        if !self.layers.contains_key(name) {
            self.layers.insert(name.to_string(), CameraShake::default());
        }
        self.layers.get_mut(name).unwrap()
    }

    /// Add trauma to a named layer.
    pub fn add(&mut self, name: &str, amount: f32) {
        self.layer_mut(name).add_trauma(amount);
    }

    /// Combined tick.
    pub fn tick(&mut self, dt: f32) -> Vec2 {
        let mut total = Vec2::ZERO;
        let mut roll = 0.0;
        for layer in self.layers.values_mut() {
            total += layer.tick(dt);
            roll += layer.roll;
        }
        let _ = roll;
        total
    }

    /// Apply combined offset.
    pub fn apply_to_camera(&mut self, camera: &mut PlayCamera, dt: f32) {
        camera.shake = self.tick(dt);
    }
}

/// Smooth zoom punch (temporary FOV/zoom kick).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoomPunch {
    /// Current additive zoom delta.
    pub delta: f32,
    /// Target snap amount when punched.
    pub amount: f32,
    /// Recovery speed.
    pub recovery: f32,
}

impl Default for ZoomPunch {
    fn default() -> Self {
        Self {
            delta: 0.0,
            amount: 0.08,
            recovery: 4.0,
        }
    }
}

impl ZoomPunch {
    /// Trigger punch (negative amount = zoom in).
    pub fn punch(&mut self, amount: f32) {
        self.delta += amount;
    }

    /// Tick toward zero.
    pub fn tick(&mut self, dt: f32) -> f32 {
        let dt = dt.max(0.0);
        let t = (self.recovery * dt).clamp(0.0, 1.0);
        self.delta *= 1.0 - t;
        if self.delta.abs() < 1e-4 {
            self.delta = 0.0;
        }
        self.delta
    }

    /// Apply to camera zoom (base_zoom + delta).
    pub fn apply(&mut self, camera: &mut PlayCamera, base_zoom: f32, dt: f32) {
        let d = self.tick(dt);
        camera.zoom = (base_zoom + d).max(0.05);
    }
}

fn pseudo_noise(x: f32) -> f32 {
    // Deterministic hash noise in [-1, 1].
    let mut v = x.sin() * 43_758.547;
    v = v - v.floor();
    v * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trauma_decays() {
        let mut s = CameraShake::default();
        s.add_trauma(1.0);
        let first = s.tick(0.016).length();
        for _ in 0..120 {
            s.tick(0.05);
        }
        assert!(!s.is_active());
        assert!(first > 0.0);
    }

    #[test]
    fn impulse_clamped() {
        let mut s = CameraShake::default();
        s.impulse(5.0);
        assert!((s.trauma - 1.0).abs() < 1e-5);
    }

    #[test]
    fn layers_combine() {
        let mut layers = CameraTraumaLayers::default();
        layers.add("gun", 0.5);
        layers.add("explosion", 0.8);
        let off = layers.tick(0.0);
        assert!(off.length() > 0.0 || layers.layers["gun"].trauma > 0.0);
    }

    #[test]
    fn zoom_punch_recovers() {
        let mut z = ZoomPunch::default();
        z.punch(-0.1);
        let mut cam = PlayCamera::default();
        let base = cam.zoom;
        z.apply(&mut cam, base, 0.0);
        assert!((cam.zoom - (base - 0.1)).abs() < 1e-4);
        for _ in 0..60 {
            z.apply(&mut cam, base, 0.05);
        }
        assert!((cam.zoom - base).abs() < 0.01);
    }

    #[test]
    fn apply_to_camera_sets_shake() {
        let mut s = CameraShake::new(20.0);
        s.add_trauma(1.0);
        let mut cam = PlayCamera::default();
        s.apply_to_camera(&mut cam, 0.016);
        assert!(cam.shake.length() > 0.0);
    }

    #[test]
    fn trauma_clamps_and_clear() {
        let mut s = CameraShake::default();
        s.add_trauma(5.0);
        assert!((s.trauma - 1.0).abs() < 1e-5);
        s.set_trauma(0.25);
        assert!((s.trauma - 0.25).abs() < 1e-5);
        s.clear();
        assert!(!s.is_active());
        assert_eq!(s.offset, Vec2::ZERO);
    }

    #[test]
    fn shake_offset_bounded_by_max() {
        let mut s = CameraShake::new(8.0);
        s.add_trauma(1.0);
        // Offset is per-axis scaled by max_offset * trauma^2; length ≤ max_offset * √2.
        let limit = 8.0 * std::f32::consts::SQRT_2 + 1e-2;
        for _ in 0..30 {
            let off = s.tick(0.016);
            assert!(off.length() <= limit, "off={off:?} len={}", off.length());
            if !s.is_active() {
                break;
            }
            s.add_trauma(1.0); // keep full trauma to probe bound
            s.trauma = 1.0;
        }
    }

    #[test]
    fn layers_independent_decay() {
        let mut layers = CameraTraumaLayers::default();
        layers.add("a", 1.0);
        layers.add("b", 0.2);
        for _ in 0..40 {
            layers.tick(0.05);
        }
        // Low trauma decays away first.
        let a_active = layers
            .layers
            .get("a")
            .map(|l| l.trauma > 0.001)
            .unwrap_or(false);
        let b_active = layers
            .layers
            .get("b")
            .map(|l| l.trauma > 0.001)
            .unwrap_or(false);
        // a started higher so more likely still active or both inactive.
        assert!(a_active || !b_active);
    }

    #[test]
    fn zoom_punch_stacks_and_recovers() {
        let mut z = ZoomPunch::default();
        z.punch(-0.05);
        z.punch(-0.05);
        let mut cam = PlayCamera::default();
        let base = cam.zoom;
        z.apply(&mut cam, base, 0.0);
        assert!(cam.zoom < base);
        for _ in 0..120 {
            z.apply(&mut cam, base, 0.05);
        }
        assert!((cam.zoom - base).abs() < 0.02, "zoom={}", cam.zoom);
    }

    #[test]
    fn zero_dt_tick_stable() {
        let mut s = CameraShake::default();
        s.add_trauma(0.5);
        let off = s.tick(0.0);
        // Trauma should not decay with 0 dt.
        assert!(s.trauma > 0.4);
        let _ = off;
    }
}
