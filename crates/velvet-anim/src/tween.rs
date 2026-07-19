//! Generic tweens over pose fields.

use serde::{Deserialize, Serialize};
use velvet_math::Ease;

use crate::pose::{AnimField, AnimPose};

/// One active float tween.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloatTween {
    /// Field.
    pub field: AnimField,
    /// Start value.
    pub from: f32,
    /// End value.
    pub to: f32,
    /// Duration seconds.
    pub duration: f32,
    /// Elapsed.
    pub elapsed: f32,
    /// Easing.
    pub ease: Ease,
    /// Delay before starting.
    pub delay: f32,
}

impl FloatTween {
    /// Create.
    pub fn new(field: AnimField, from: f32, to: f32, duration: f32, ease: Ease) -> Self {
        Self {
            field,
            from,
            to,
            duration: duration.max(1e-4),
            elapsed: 0.0,
            ease,
            delay: 0.0,
        }
    }

    /// With delay.
    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay.max(0.0);
        self
    }

    /// Finished?
    pub fn finished(&self) -> bool {
        self.elapsed >= self.delay + self.duration
    }

    /// Tick; returns current value.
    pub fn tick(&mut self, dt: f32) -> f32 {
        self.elapsed += dt;
        self.sample()
    }

    /// Sample without advancing.
    pub fn sample(&self) -> f32 {
        if self.elapsed < self.delay {
            return self.from;
        }
        let local = self.elapsed - self.delay;
        let t = (local / self.duration).clamp(0.0, 1.0);
        let e = self.ease.eval(t);
        self.from + (self.to - self.from) * e
    }
}

/// Apply a float sample onto a pose field.
pub fn apply_field(pose: &mut AnimPose, field: AnimField, value: f32) {
    match field {
        AnimField::X => pose.pos.x = value,
        AnimField::Y => pose.pos.y = value,
        AnimField::Scale => pose.scale = value,
        AnimField::Rotation => pose.rotation = value,
        AnimField::Opacity => pose.opacity = value.clamp(0.0, 1.0),
    }
}

/// Read field from pose.
pub fn read_field(pose: &AnimPose, field: AnimField) -> f32 {
    match field {
        AnimField::X => pose.pos.x,
        AnimField::Y => pose.pos.y,
        AnimField::Scale => pose.scale,
        AnimField::Rotation => pose.rotation,
        AnimField::Opacity => pose.opacity,
    }
}

/// Parse ease name (story / script friendly).
pub fn parse_ease(name: &str) -> Ease {
    match name.trim().to_ascii_lowercase().as_str() {
        "linear" => Ease::Linear,
        "quad_in" | "quadin" => Ease::QuadIn,
        "quad_out" | "quadout" | "ease_out" | "easeout" => Ease::QuadOut,
        "quad_in_out" | "quadinout" => Ease::QuadInOut,
        "cubic_in" | "cubicin" => Ease::CubicIn,
        "cubic_out" | "cubicout" => Ease::CubicOut,
        "cubic_in_out" | "cubicinout" | "smooth" => Ease::CubicInOut,
        "back_out" | "backout" => Ease::BackOut,
        "back_in" | "backin" => Ease::BackIn,
        "elastic_out" | "elasticout" => Ease::ElasticOut,
        "bounce_out" | "bounceout" | "bounce" => Ease::BounceOut,
        "sine_out" | "sineout" => Ease::SineOut,
        "smoothstep" => Ease::Smoothstep,
        "smootherstep" => Ease::Smootherstep,
        _ => Ease::CubicOut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_reaches_end() {
        let mut t = FloatTween::new(AnimField::Opacity, 0.0, 1.0, 1.0, Ease::Linear);
        assert!((t.tick(0.0) - 0.0).abs() < 1e-5);
        assert!((t.tick(0.5) - 0.5).abs() < 1e-5);
        assert!((t.tick(0.5) - 1.0).abs() < 1e-5);
        assert!(t.finished());
    }

    #[test]
    fn delay_holds_start() {
        let mut t = FloatTween::new(AnimField::X, 0.0, 10.0, 1.0, Ease::Linear).with_delay(0.5);
        assert!((t.tick(0.4) - 0.0).abs() < 1e-5);
        // total elapsed 0.5 after +0.1 → just finished delay, at start of tween
        let v = t.tick(0.1);
        assert!((v - 0.0).abs() < 1e-4, "v={v}");
        // +0.5s into 1s tween → ~5
        let v2 = t.tick(0.5);
        assert!((v2 - 5.0).abs() < 0.1, "v2={v2}");
    }
}
