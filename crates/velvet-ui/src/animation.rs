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

/// One crossfade from a frozen sample into a live tween.
#[derive(Debug, Clone)]
struct FadingTween {
    /// Tween name.
    name: String,
    /// Frozen source sample.
    from: TweenSample,
    /// Incoming tween.
    tween: UiTween,
    /// Fade duration seconds.
    fade: f32,
    /// Elapsed fade time.
    elapsed: f32,
}

/// Tracks multiple concurrent UI tweens.
#[derive(Debug, Default, Clone)]
pub struct UiAnimator {
    tweens: Vec<(String, UiTween)>,
    fading: Vec<FadingTween>,
}

impl UiAnimator {
    /// Empty animator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start or replace a named tween.
    pub fn play(&mut self, name: impl Into<String>, tween: UiTween) {
        let name = name.into();
        self.fading.retain(|f| f.name != name);
        if let Some((_, t)) = self.tweens.iter_mut().find(|(n, _)| *n == name) {
            *t = tween;
        } else {
            self.tweens.push((name, tween));
        }
    }

    /// Crossfade into a named tween from its current sample over
    /// `fade_secs` (missing source fields cut in directly).
    pub fn play_fade(&mut self, name: impl Into<String>, tween: UiTween, fade_secs: f32) {
        let name = name.into();
        let from = self.sample(&name).unwrap_or_default();
        self.tweens.retain(|(n, _)| *n != name);
        self.fading.retain(|f| f.name != name);
        if fade_secs <= 0.0 {
            self.tweens.push((name, tween));
            return;
        }
        self.fading.push(FadingTween {
            name,
            from,
            tween,
            fade: fade_secs.max(1e-4),
            elapsed: 0.0,
        });
    }

    /// Tick all; remove finished. Returns samples keyed by name.
    pub fn tick(&mut self, dt: f32) -> Vec<(String, TweenSample)> {
        let mut out = Vec::new();
        for (name, tween) in &mut self.tweens {
            out.push((name.clone(), tween.tick(dt)));
        }
        self.tweens.retain(|(_, t)| !t.is_finished());
        let mut done = Vec::new();
        for (i, fade) in self.fading.iter_mut().enumerate() {
            fade.elapsed += dt.max(0.0);
            let sample = fade.tween.tick(dt);
            let k = (fade.elapsed / fade.fade).clamp(0.0, 1.0);
            out.push((
                fade.name.clone(),
                TweenSample::blend(&fade.from, &sample, k),
            ));
            if fade.elapsed >= fade.fade {
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            let fade = self.fading.remove(i);
            self.tweens.push((fade.name, fade.tween));
        }
        out
    }

    /// Sample named tween (fading entries report the blend).
    pub fn sample(&self, name: &str) -> Option<TweenSample> {
        if let Some(fade) = self.fading.iter().find(|f| f.name == name) {
            let k = (fade.elapsed / fade.fade).clamp(0.0, 1.0);
            return Some(TweenSample::blend(&fade.from, &fade.tween.sample(), k));
        }
        self.tweens
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.sample())
    }

    /// Active count (tweens + fades).
    pub fn len(&self) -> usize {
        self.tweens.len() + self.fading.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.tweens.is_empty() && self.fading.is_empty()
    }
}

impl TweenSample {
    /// Blend two samples (`k` = 0 → `a`). Fields missing on either side
    /// cut in directly instead of interpolating from nothing.
    pub fn blend(a: &TweenSample, b: &TweenSample, k: f32) -> TweenSample {
        let k = k.clamp(0.0, 1.0);
        let num = |x: Option<f32>, y: Option<f32>| match (x, y) {
            (Some(x), Some(y)) => Some(x + (y - x) * k),
            (None, y) => y,
            (x, None) => x,
        };
        TweenSample {
            opacity: num(a.opacity, b.opacity),
            offset: match (a.offset, b.offset) {
                (Some(x), Some(y)) => Some(x.lerp(y, k)),
                (None, y) => y,
                (x, None) => x,
            },
            scale: num(a.scale, b.scale),
        }
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

    #[test]
    fn play_fade_blends_from_current() {
        let mut a = UiAnimator::new();
        a.play("x", UiTween::opacity(0.0, 0.0, 10.0));
        a.tick(0.1);
        // Incoming finishes fast (0.1s) while the fade lasts 1s:
        // midpoint blends frozen 0.0 with finished 1.0 at k=0.5.
        a.play_fade("x", UiTween::opacity(0.0, 1.0, 0.1), 1.0);
        a.tick(0.5);
        let s = a.sample("x").unwrap();
        assert!((s.opacity.unwrap() - 0.5).abs() < 1e-4, "{s:?}");
        // After the fade the live tween owns the name and finishes alone.
        a.tick(1.0);
        let s = a.sample("x").unwrap();
        assert!((s.opacity.unwrap() - 1.0).abs() < 1e-4, "{s:?}");
    }

    #[test]
    fn play_fade_without_source_cuts_in() {
        let mut a = UiAnimator::new();
        a.play_fade("x", UiTween::opacity(0.0, 1.0, 1.0), 1.0);
        // No source fields: blend is the incoming sample immediately.
        let s = a.sample("x").unwrap();
        assert!((s.opacity.unwrap() - 0.0).abs() < 1e-4);
    }
}
