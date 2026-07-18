//! Stick deadzone curves and axial filtering.

use serde::{Deserialize, Serialize};
use velvet_math::{clamp, Vec2};

/// Deadzone shape applied to a 2D stick.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum DeadzoneShape {
    /// Independent per-axis (cross-shaped).
    Axial,
    /// Radial (circular) deadzone — default for gamepads.
    #[default]
    Radial,
    /// Scaled radial: remap [inner, outer] → [0, 1] smoothly.
    ScaledRadial,
    /// Nightingale / bow-tie hybrid (axial + radial mix).
    Hybrid {
        /// Blend 0 = pure radial, 1 = pure axial.
        axial_blend: f32,
    },
}

/// Deadzone parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeadzoneConfig {
    /// Inner deadzone radius / threshold (0..=1).
    pub inner: f32,
    /// Outer clamp radius (0..=1), typically 1.0.
    pub outer: f32,
    /// Shape.
    pub shape: DeadzoneShape,
    /// Optional response exponent after deadzone (1 = linear, >1 = fine aim).
    pub exponent: f32,
    /// Invert Y.
    pub invert_y: bool,
}

impl Default for DeadzoneConfig {
    fn default() -> Self {
        Self {
            inner: 0.2,
            outer: 1.0,
            shape: DeadzoneShape::ScaledRadial,
            exponent: 1.0,
            invert_y: false,
        }
    }
}

impl DeadzoneConfig {
    /// Common gamepad defaults.
    pub fn gamepad() -> Self {
        Self::default()
    }

    /// Tight aim stick.
    pub fn aim() -> Self {
        Self {
            inner: 0.12,
            outer: 1.0,
            shape: DeadzoneShape::ScaledRadial,
            exponent: 1.4,
            invert_y: false,
        }
    }

    /// Apply config to a raw stick vector in approximately -1..=1.
    pub fn apply(self, raw: Vec2) -> Vec2 {
        apply_deadzone(raw, self)
    }
}

/// Apply deadzone configuration to a stick vector.
pub fn apply_deadzone(raw: Vec2, cfg: DeadzoneConfig) -> Vec2 {
    let mut v = raw;
    if cfg.invert_y {
        v.y = -v.y;
    }
    // Clamp to unit circle-ish first.
    v = clamp_stick(v);

    let inner = cfg.inner.clamp(0.0, 1.0);
    let outer = cfg.outer.clamp(inner + 1e-5, 2.0);

    let mut out = match cfg.shape {
        DeadzoneShape::Axial => apply_axial(v, inner, outer),
        DeadzoneShape::Radial => apply_radial(v, inner, outer, false),
        DeadzoneShape::ScaledRadial => apply_radial(v, inner, outer, true),
        DeadzoneShape::Hybrid { axial_blend } => {
            let a = apply_axial(v, inner, outer);
            let r = apply_radial(v, inner, outer, true);
            let t = axial_blend.clamp(0.0, 1.0);
            a.lerp(r, 1.0 - t)
        }
    };

    if cfg.exponent > 0.0 && (cfg.exponent - 1.0).abs() > 1e-5 {
        let len = out.length();
        if len > 1e-8 {
            let new_len = len.powf(cfg.exponent);
            out *= new_len / len;
        }
    }
    out
}

fn clamp_stick(v: Vec2) -> Vec2 {
    let len = v.length();
    if len > 1.0 {
        v * (1.0 / len)
    } else {
        v
    }
}

fn apply_axial(v: Vec2, inner: f32, outer: f32) -> Vec2 {
    Vec2::new(apply_axis(v.x, inner, outer), apply_axis(v.y, inner, outer))
}

fn apply_axis(x: f32, inner: f32, outer: f32) -> f32 {
    let ax = x.abs();
    if ax <= inner {
        0.0
    } else if ax >= outer {
        x.signum()
    } else {
        let t = (ax - inner) / (outer - inner);
        x.signum() * t
    }
}

fn apply_radial(v: Vec2, inner: f32, outer: f32, scale: bool) -> Vec2 {
    let len = v.length();
    if len <= inner || len < 1e-8 {
        return Vec2::ZERO;
    }
    let dir = v / len;
    if !scale {
        // Hard radial: zero inside, full direction outside (snappy).
        if len >= outer {
            dir
        } else {
            dir // still full magnitude direction with partial? use unscaled len
                * ((len - inner) / (1.0 - inner).max(1e-5)).min(1.0)
        }
    } else {
        // Scaled: remap [inner, outer] → [0, 1]
        let t = ((len - inner) / (outer - inner)).clamp(0.0, 1.0);
        dir * t
    }
}

/// Snap stick to 4-way cardinal if past threshold (d-pad style).
pub fn snap_cardinal(v: Vec2, threshold: f32) -> Vec2 {
    if v.length() < threshold {
        return Vec2::ZERO;
    }
    if v.x.abs() > v.y.abs() {
        Vec2::new(v.x.signum(), 0.0)
    } else {
        Vec2::new(0.0, v.y.signum())
    }
}

/// Snap to 8-way.
pub fn snap_8way(v: Vec2, threshold: f32) -> Vec2 {
    if v.length() < threshold {
        return Vec2::ZERO;
    }
    let angle = v.y.atan2(v.x);
    let sector =
        ((angle + std::f32::consts::PI) / (std::f32::consts::FRAC_PI_4 / 1.0)).round() as i32;
    // Use 8 directions from angle.
    let step = (angle / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
    let _ = sector;
    Vec2::from_angle(step)
}

/// Smooth stick over time (exponential).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickFilter {
    /// Current filtered value.
    pub value: Vec2,
    /// Smoothing half-life seconds-ish via alpha per second.
    pub alpha: f32,
}

impl Default for StickFilter {
    fn default() -> Self {
        Self {
            value: Vec2::ZERO,
            alpha: 20.0,
        }
    }
}

impl StickFilter {
    /// Create.
    pub fn new(alpha: f32) -> Self {
        Self {
            value: Vec2::ZERO,
            alpha: alpha.max(0.0),
        }
    }

    /// Filter sample with dt.
    pub fn tick(&mut self, sample: Vec2, dt: f32) -> Vec2 {
        let a = 1.0 - (-self.alpha * dt.max(0.0)).exp();
        self.value = self.value.lerp(sample, clamp(a, 0.0, 1.0));
        self.value
    }

    /// Reset.
    pub fn reset(&mut self) {
        self.value = Vec2::ZERO;
    }
}

/// Pipeline: deadzone then filter.
#[derive(Debug, Clone)]
pub struct StickPipeline {
    /// Deadzone config.
    pub deadzone: DeadzoneConfig,
    /// Optional low-pass.
    pub filter: StickFilter,
    /// Whether filter enabled.
    pub filter_enabled: bool,
}

impl Default for StickPipeline {
    fn default() -> Self {
        Self {
            deadzone: DeadzoneConfig::gamepad(),
            filter: StickFilter::default(),
            filter_enabled: false,
        }
    }
}

impl StickPipeline {
    /// Process raw stick.
    pub fn process(&mut self, raw: Vec2, dt: f32) -> Vec2 {
        let v = self.deadzone.apply(raw);
        if self.filter_enabled {
            self.filter.tick(v, dt)
        } else {
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_deadzone_zero() {
        let cfg = DeadzoneConfig {
            inner: 0.25,
            shape: DeadzoneShape::ScaledRadial,
            ..Default::default()
        };
        let v = apply_deadzone(Vec2::new(0.1, 0.0), cfg);
        assert!(v.length() < 1e-5);
    }

    #[test]
    fn full_deflection() {
        let cfg = DeadzoneConfig::gamepad();
        let v = apply_deadzone(Vec2::new(1.0, 0.0), cfg);
        assert!((v.x - 1.0).abs() < 1e-3);
    }

    #[test]
    fn axial_independent() {
        let cfg = DeadzoneConfig {
            shape: DeadzoneShape::Axial,
            inner: 0.2,
            ..Default::default()
        };
        let v = apply_deadzone(Vec2::new(0.1, 0.9), cfg);
        assert!(v.x.abs() < 1e-5);
        assert!(v.y > 0.5);
    }

    #[test]
    fn exponent_softens() {
        let mut cfg = DeadzoneConfig::gamepad();
        cfg.inner = 0.0;
        cfg.exponent = 2.0;
        let v = apply_deadzone(Vec2::new(0.5, 0.0), cfg);
        assert!(v.x < 0.5 && v.x > 0.0);
    }

    #[test]
    fn filter_moves_toward() {
        let mut f = StickFilter::new(50.0);
        let v = f.tick(Vec2::X, 0.1);
        assert!(v.x > 0.0 && v.x <= 1.0);
    }

    #[test]
    fn pipeline() {
        let mut p = StickPipeline::default();
        let v = p.process(Vec2::new(0.05, 0.0), 0.016);
        assert!(v.length() < 1e-4);
    }

    #[test]
    fn snap_to_cardinal_axes() {
        let v = snap_cardinal(Vec2::new(0.8, 0.1), 0.2);
        assert!((v.x - 1.0).abs() < 1e-5);
        assert!(v.y.abs() < 1e-5);
    }
}
