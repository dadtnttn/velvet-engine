//! UI tween animations (opacity, offset, scale).

use velvet_math::Vec2;

/// Easing function for UI tweens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ease {
    /// Linear.
    #[default]
    Linear,
    /// Smooth start/end.
    SmoothStep,
    /// Ease out cubic.
    EaseOutCubic,
    /// Ease in cubic.
    EaseInCubic,
}

impl Ease {
    /// Evaluate ease at `t` in `0..=1`.
    pub fn eval(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::SmoothStep => t * t * (3.0 - 2.0 * t),
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInCubic => t * t * t,
        }
    }
}

/// What is being animated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TweenProperty {
    /// Opacity `0..=1`.
    Opacity {
        /// Start.
        from: f32,
        /// End.
        to: f32,
    },
    /// Pixel offset.
    Offset {
        /// Start.
        from: Vec2,
        /// End.
        to: Vec2,
    },
    /// Uniform scale.
    Scale {
        /// Start.
        from: f32,
        /// End.
        to: f32,
    },
}

/// Active UI tween.
#[derive(Debug, Clone, PartialEq)]
pub struct UiTween {
    /// Property.
    pub property: TweenProperty,
    /// Duration seconds.
    pub duration: f32,
    /// Elapsed seconds.
    pub elapsed: f32,
    /// Easing.
    pub ease: Ease,
    /// Finished flag.
    finished: bool,
}

impl UiTween {
    /// Create tween.
    pub fn new(property: TweenProperty, duration: f32, ease: Ease) -> Self {
        Self {
            property,
            duration: duration.max(1e-4),
            elapsed: 0.0,
            ease,
            finished: false,
        }
    }

    /// Opacity fade.
    pub fn opacity(from: f32, to: f32, duration: f32) -> Self {
        Self::new(
            TweenProperty::Opacity { from, to },
            duration,
            Ease::SmoothStep,
        )
    }

    /// Offset slide.
    pub fn offset(from: Vec2, to: Vec2, duration: f32) -> Self {
        Self::new(
            TweenProperty::Offset { from, to },
            duration,
            Ease::EaseOutCubic,
        )
    }

    /// Normalized progress `0..=1`.
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Eased progress.
    pub fn eased_t(&self) -> f32 {
        self.ease.eval(self.progress())
    }

    /// Finished.
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Advance by `dt`; returns current sample.
    pub fn tick(&mut self, dt: f32) -> TweenSample {
        if !self.finished {
            self.elapsed += dt.max(0.0);
            if self.elapsed >= self.duration {
                self.elapsed = self.duration;
                self.finished = true;
            }
        }
        self.sample()
    }

    /// Current sample without advancing.
    pub fn sample(&self) -> TweenSample {
        let t = self.eased_t();
        match self.property {
            TweenProperty::Opacity { from, to } => TweenSample {
                opacity: Some(from + (to - from) * t),
                offset: None,
                scale: None,
            },
            TweenProperty::Offset { from, to } => TweenSample {
                opacity: None,
                offset: Some(Vec2::new(
                    from.x + (to.x - from.x) * t,
                    from.y + (to.y - from.y) * t,
                )),
                scale: None,
            },
            TweenProperty::Scale { from, to } => TweenSample {
                opacity: None,
                offset: None,
                scale: Some(from + (to - from) * t),
            },
        }
    }
}

/// Sampled tween output (sparse fields).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TweenSample {
    /// Opacity if animating opacity.
    pub opacity: Option<f32>,
    /// Offset if animating offset.
    pub offset: Option<Vec2>,
    /// Scale if animating scale.
    pub scale: Option<f32>,
}

/// Tracks multiple concurrent UI tweens.
#[derive(Debug, Default, Clone)]
pub struct UiAnimator {
    tweens: Vec<(String, UiTween)>,
}

impl UiAnimator {
    /// Empty animator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start or replace a named tween.
    pub fn play(&mut self, name: impl Into<String>, tween: UiTween) {
        let name = name.into();
        if let Some((_, t)) = self.tweens.iter_mut().find(|(n, _)| *n == name) {
            *t = tween;
        } else {
            self.tweens.push((name, tween));
        }
    }

    /// Tick all; remove finished. Returns samples keyed by name.
    pub fn tick(&mut self, dt: f32) -> Vec<(String, TweenSample)> {
        let mut out = Vec::new();
        for (name, tween) in &mut self.tweens {
            out.push((name.clone(), tween.tick(dt)));
        }
        self.tweens.retain(|(_, t)| !t.is_finished());
        out
    }

    /// Sample named tween.
    pub fn sample(&self, name: &str) -> Option<TweenSample> {
        self.tweens
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.sample())
    }

    /// Active count.
    pub fn len(&self) -> usize {
        self.tweens.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.tweens.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opacity_tween() {
        let mut t = UiTween::opacity(0.0, 1.0, 1.0);
        let s = t.tick(0.5);
        assert!((s.opacity.unwrap() - 0.5).abs() < 0.05); // smoothstep ~0.5
        t.tick(0.6);
        assert!(t.is_finished());
        assert!((t.sample().opacity.unwrap() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn animator_removes_finished() {
        let mut a = UiAnimator::new();
        a.play("fade", UiTween::opacity(1.0, 0.0, 0.2));
        a.tick(0.25);
        assert!(a.is_empty());
    }
}
