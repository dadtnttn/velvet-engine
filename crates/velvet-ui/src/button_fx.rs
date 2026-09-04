//! Animated buttons: hover grow, press squash, release pop, disabled fade.
//!
//! [`AnimatedButton`] watches the plain `hovered`/`pressed`/`enabled`
//! flags the tree already computes and turns transitions into tweens on
//! an owned [`UiAnimator`]. Hosts read [`ButtonSample`] each tick and
//! apply scale/opacity when painting. With reduced motion everything
//! snaps instantly (no tweens are ever played).

use crate::animation::{Ease, TweenProperty, UiAnimator, UiTween};

/// Visual state of an animated button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVisual {
    /// At rest.
    #[default]
    Idle,
    /// Pointer over.
    Hover,
    /// Pressed down.
    Pressed,
    /// Disabled (faded, ignores input visuals).
    Disabled,
}

/// Tuning for [`AnimatedButton`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonFxPreset {
    /// Scale while hovered.
    pub hover_scale: f32,
    /// Scale while pressed.
    pub press_scale: f32,
    /// Opacity while disabled.
    pub disabled_opacity: f32,
    /// Seconds into hover/press tweens.
    pub in_time: f32,
    /// Seconds back to idle (release pop).
    pub out_time: f32,
}

impl Default for ButtonFxPreset {
    fn default() -> Self {
        Self {
            hover_scale: 1.04,
            press_scale: 0.94,
            disabled_opacity: 0.45,
            in_time: 0.12,
            out_time: 0.18,
        }
    }
}

/// Current painted values for a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonSample {
    /// Scale multiplier.
    pub scale: f32,
    /// Opacity.
    pub opacity: f32,
}

impl Default for ButtonSample {
    fn default() -> Self {
        Self {
            scale: 1.0,
            opacity: 1.0,
        }
    }
}

/// Button feel controller (one per animated button, keyed by node).
#[derive(Debug, Clone)]
pub struct AnimatedButton {
    preset: ButtonFxPreset,
    state: ButtonVisual,
    animator: UiAnimator,
    reduced_motion: bool,
}

impl AnimatedButton {
    /// New controller with a preset.
    pub fn with_preset(preset: ButtonFxPreset) -> Self {
        Self {
            preset,
            state: ButtonVisual::Idle,
            animator: UiAnimator::new(),
            reduced_motion: false,
        }
    }

    /// Current visual state.
    pub fn state(&self) -> ButtonVisual {
        self.state
    }

    /// Reduced motion snaps instead of tweening.
    pub fn set_reduced_motion(&mut self, on: bool) {
        self.reduced_motion = on;
    }

    /// Feed live flags; plays tweens on state changes. Returns nothing —
    /// read painted values from [`AnimatedButton::tick`].
    pub fn update(&mut self, hovered: bool, pressed: bool, enabled: bool) {
        let next = if !enabled {
            ButtonVisual::Disabled
        } else if pressed {
            ButtonVisual::Pressed
        } else if hovered {
            ButtonVisual::Hover
        } else {
            ButtonVisual::Idle
        };
        if next == self.state {
            return;
        }
        // Snapshot BEFORE flipping state: sample_now() falls back to the
        // state's target when no tween runs, which would read the goal.
        let current = self.sample_now();
        self.state = next;
        if self.reduced_motion {
            return;
        }
        let (scale, opacity, time) = match next {
            ButtonVisual::Idle => (1.0, 1.0, self.preset.out_time),
            ButtonVisual::Hover => (self.preset.hover_scale, 1.0, self.preset.in_time),
            ButtonVisual::Pressed => (self.preset.press_scale, 1.0, self.preset.in_time),
            ButtonVisual::Disabled => (1.0, self.preset.disabled_opacity, self.preset.in_time),
        };
        // Blend from the live sample so rapid hover/press flips never pop.
        self.animator
            .play("scale", scaled_tween(current.scale, scale, time));
        self.animator
            .play("opacity", faded_tween(current.opacity, opacity, time));
    }

    /// Advance and read painted values.
    pub fn tick(&mut self, dt: f32) -> ButtonSample {
        if self.reduced_motion {
            return self.target_sample();
        }
        let _ = self.animator.tick(dt);
        self.sample_now()
    }

    /// Painted values right now (without advancing).
    pub fn sample_now(&self) -> ButtonSample {
        if self.reduced_motion {
            return self.target_sample();
        }
        let scale = self
            .animator
            .sample("scale")
            .and_then(|s| s.scale)
            .unwrap_or(self.target_sample().scale);
        let opacity = self
            .animator
            .sample("opacity")
            .and_then(|s| s.opacity)
            .unwrap_or(self.target_sample().opacity);
        ButtonSample { scale, opacity }
    }

    fn target_sample(&self) -> ButtonSample {
        match self.state {
            ButtonVisual::Idle => ButtonSample {
                scale: 1.0,
                opacity: 1.0,
            },
            ButtonVisual::Hover => ButtonSample {
                scale: self.preset.hover_scale,
                opacity: 1.0,
            },
            ButtonVisual::Pressed => ButtonSample {
                scale: self.preset.press_scale,
                opacity: 1.0,
            },
            ButtonVisual::Disabled => ButtonSample {
                scale: 1.0,
                opacity: self.preset.disabled_opacity,
            },
        }
    }
}

fn scaled_tween(from: f32, to: f32, duration: f32) -> UiTween {
    UiTween::new(
        TweenProperty::Scale { from, to },
        duration,
        Ease::EaseOutCubic,
    )
}

fn faded_tween(from: f32, to: f32, duration: f32) -> UiTween {
    UiTween::new(
        TweenProperty::Opacity { from, to },
        duration,
        Ease::SmoothStep,
    )
}

/// Expanding press ripple for hosts to paint (circle outline data).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressRipple {
    /// Center in UI px.
    pub center: velvet_math::Vec2,
    /// Elapsed seconds.
    pub elapsed: f32,
    /// Total duration seconds.
    pub duration: f32,
    /// Final radius in px.
    pub max_radius: f32,
}

impl PressRipple {
    /// New ripple (hosts spawn one per click, e.g. at the press point).
    pub fn new(center: velvet_math::Vec2, max_radius: f32, duration: f32) -> Self {
        Self {
            center,
            elapsed: 0.0,
            duration: duration.max(1e-4),
            max_radius: max_radius.max(1.0),
        }
    }

    /// Advance; returns false when finished.
    pub fn tick(&mut self, dt: f32) -> bool {
        self.elapsed += dt.max(0.0);
        self.elapsed < self.duration
    }

    /// Current radius.
    pub fn radius(&self) -> f32 {
        let k = (self.elapsed / self.duration).clamp(0.0, 1.0);
        // Ease-out: fast spread, soft landing.
        self.max_radius * (1.0 - (1.0 - k) * (1.0 - k))
    }

    /// Current alpha (1 → 0 over the life).
    pub fn alpha(&self) -> f32 {
        (1.0 - self.elapsed / self.duration).clamp(0.0, 1.0)
    }
}

/// Animated toggle switch: knob slides 0 (off) → 1 (on) on flips.
#[derive(Debug, Clone)]
pub struct AnimatedToggle {
    animator: UiAnimator,
    on: bool,
    reduced_motion: bool,
}

impl AnimatedToggle {
    /// New toggle (knob starts at `on`).
    pub fn new(on: bool) -> Self {
        Self {
            animator: UiAnimator::new(),
            on,
            reduced_motion: false,
        }
    }

    /// Reduced motion snaps the knob instead of sliding it.
    pub fn set_reduced_motion(&mut self, on: bool) {
        self.reduced_motion = on;
    }

    /// Feed the live value; slides the knob on flips (the generic
    /// `Scale` channel carries the 0..=1 knob position internally).
    pub fn update(&mut self, on: bool) {
        if on == self.on {
            return;
        }
        // Snapshot before flipping: knob_now() falls back to the target.
        let from = self.knob_now();
        self.on = on;
        if self.reduced_motion {
            return;
        }
        self.animator.play(
            "knob",
            UiTween::new(
                TweenProperty::Scale {
                    from,
                    to: if on { 1.0 } else { 0.0 },
                },
                0.15,
                Ease::EaseOutCubic,
            ),
        );
    }

    /// Advance; returns knob position 0..=1.
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.reduced_motion {
            return if self.on { 1.0 } else { 0.0 };
        }
        let _ = self.animator.tick(dt);
        self.knob_now()
    }

    fn knob_now(&self) -> f32 {
        if self.reduced_motion {
            return if self.on { 1.0 } else { 0.0 };
        }
        self.animator
            .sample("knob")
            .and_then(|s| s.scale)
            .unwrap_or(if self.on { 1.0 } else { 0.0 })
    }
}

/// Drag glow pulse 0..=1 for sliders (and scrubbers): bright while
/// dragging, breathing with time; 0 when idle.
pub fn drag_glow(dragging: bool, time_secs: f32) -> f32 {
    if !dragging {
        return 0.0;
    }
    0.75 + 0.25 * (time_secs * 6.0).sin()
}

/// Spinner angle in radians for busy buttons (full turn per 0.9 s).
pub fn spinner_angle(elapsed_secs: f32) -> f32 {
    (elapsed_secs.max(0.0) / 0.9 * std::f32::consts::TAU) % std::f32::consts::TAU
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_grows_and_release_settles() {
        let mut button = AnimatedButton::with_preset(ButtonFxPreset::default());
        button.update(false, false, true);
        assert_eq!(
            button.tick(1.0),
            ButtonSample {
                scale: 1.0,
                opacity: 1.0
            }
        );
        button.update(true, false, true);
        assert_eq!(button.state(), ButtonVisual::Hover);
        // Mid-tween: between idle and hover scale.
        let mid = button.tick(0.06);
        assert!(mid.scale > 1.0 && mid.scale < 1.04, "{mid:?}");
        button.tick(1.0);
        assert!((button.tick(0.0).scale - 1.04).abs() < 1e-4);
        // Release pops back to idle.
        button.update(false, false, true);
        button.tick(1.0);
        assert!((button.tick(0.0).scale - 1.0).abs() < 1e-4);
    }

    #[test]
    fn press_squashes_and_disable_fades() {
        let mut button = AnimatedButton::with_preset(ButtonFxPreset::default());
        button.update(true, true, true);
        assert_eq!(button.state(), ButtonVisual::Pressed);
        button.tick(1.0);
        assert!((button.tick(0.0).scale - 0.94).abs() < 1e-4);
        button.update(false, false, false);
        assert_eq!(button.state(), ButtonVisual::Disabled);
        button.tick(1.0);
        let sample = button.tick(0.0);
        assert!((sample.opacity - 0.45).abs() < 1e-4);
        assert!((sample.scale - 1.0).abs() < 1e-4);
    }

    #[test]
    fn ripple_expands_and_fades() {
        let mut ripple = PressRipple::new(velvet_math::Vec2::new(10.0, 10.0), 40.0, 0.4);
        assert_eq!(ripple.radius(), 0.0);
        assert_eq!(ripple.alpha(), 1.0);
        assert!(ripple.tick(0.2));
        // Ease-out midpoint: past half radius, half alpha.
        assert!((ripple.radius() - 30.0).abs() < 1e-4, "{}", ripple.radius());
        assert!((ripple.alpha() - 0.5).abs() < 1e-4);
        assert!(!ripple.tick(0.3));
        assert_eq!(ripple.radius(), 40.0);
        assert_eq!(ripple.alpha(), 0.0);
    }

    #[test]
    fn toggle_knob_slides_and_snaps() {
        let mut toggle = AnimatedToggle::new(false);
        assert_eq!(toggle.tick(1.0), 0.0);
        toggle.update(true);
        // Mid-slide: strictly between off and on.
        let mid = toggle.tick(0.075);
        assert!(mid > 0.0 && mid < 1.0, "{mid}");
        toggle.tick(1.0);
        assert_eq!(toggle.tick(0.0), 1.0);
        // Reduced motion snaps.
        toggle.set_reduced_motion(true);
        toggle.update(false);
        assert_eq!(toggle.tick(0.0), 0.0);
    }

    #[test]
    fn glow_and_spinner_numbers() {
        assert_eq!(drag_glow(false, 99.0), 0.0);
        let glow = drag_glow(true, 0.0);
        assert!((glow - 0.75).abs() < 1e-4, "{glow}");
        assert!((spinner_angle(0.45) - std::f32::consts::PI).abs() < 1e-4);
        assert_eq!(spinner_angle(0.0), 0.0);
    }

    #[test]
    fn reduced_motion_snaps() {
        let mut button = AnimatedButton::with_preset(ButtonFxPreset::default());
        button.set_reduced_motion(true);
        button.update(true, false, true);
        assert_eq!(button.tick(0.0).scale, 1.04);
        assert!(button.animator.is_empty(), "no tweens may play");
        button.update(false, false, false);
        assert_eq!(button.tick(0.0).opacity, 0.45);
    }
}
