//! Post-process effect stack (CPU params for GPU passes later).

use serde::{Deserialize, Serialize};
use velvet_math::Color;

/// A single post-process effect with enable flag and progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostEffect {
    /// Full-screen fade toward a color. `progress` 0 = clear, 1 = solid color.
    Fade {
        /// Target fade color.
        color: Color,
        /// Normalized intensity `0..=1`.
        progress: f32,
        /// Whether active.
        enabled: bool,
    },
    /// Brief flash overlay (e.g. hit flash).
    Flash {
        /// Flash color.
        color: Color,
        /// Normalized intensity `0..=1`.
        progress: f32,
        /// Whether active.
        enabled: bool,
    },
    /// Color grading: lift/gamma/gain style simple params.
    ColorGrade {
        /// Multiplicative gain on RGB.
        gain: Color,
        /// Additive lift on RGB (usually small).
        lift: Color,
        /// Gamma exponent (1 = identity).
        gamma: f32,
        /// Mix amount `0..=1` (0 = bypass).
        progress: f32,
        /// Whether active.
        enabled: bool,
    },
}

impl PostEffect {
    /// Create a disabled fade to black.
    pub fn fade_black() -> Self {
        Self::Fade {
            color: Color::BLACK,
            progress: 0.0,
            enabled: false,
        }
    }

    /// Create a white flash at full intensity.
    pub fn flash_white(progress: f32) -> Self {
        Self::Flash {
            color: Color::WHITE,
            progress: progress.clamp(0.0, 1.0),
            enabled: true,
        }
    }

    /// Identity color grade (no visible change when enabled at progress 1).
    pub fn color_grade_identity() -> Self {
        Self::ColorGrade {
            gain: Color::WHITE,
            lift: Color::rgba(0.0, 0.0, 0.0, 0.0),
            gamma: 1.0,
            progress: 1.0,
            enabled: false,
        }
    }

    /// Whether the effect is enabled.
    pub fn enabled(&self) -> bool {
        match self {
            Self::Fade { enabled, .. }
            | Self::Flash { enabled, .. }
            | Self::ColorGrade { enabled, .. } => *enabled,
        }
    }

    /// Set enabled flag.
    pub fn set_enabled(&mut self, on: bool) {
        match self {
            Self::Fade { enabled, .. }
            | Self::Flash { enabled, .. }
            | Self::ColorGrade { enabled, .. } => *enabled = on,
        }
    }

    /// Normalized progress / mix `0..=1`.
    pub fn progress(&self) -> f32 {
        match self {
            Self::Fade { progress, .. }
            | Self::Flash { progress, .. }
            | Self::ColorGrade { progress, .. } => *progress,
        }
    }

    /// Set progress clamped to `0..=1`.
    pub fn set_progress(&mut self, p: f32) {
        let p = p.clamp(0.0, 1.0);
        match self {
            Self::Fade { progress, .. }
            | Self::Flash { progress, .. }
            | Self::ColorGrade { progress, .. } => *progress = p,
        }
    }

    /// Effect kind name for diagnostics.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Fade { .. } => "fade",
            Self::Flash { .. } => "flash",
            Self::ColorGrade { .. } => "color_grade",
        }
    }

    /// Apply color grade (or return input) for a single sample color.
    pub fn apply_to_color(&self, input: Color) -> Color {
        if !self.enabled() {
            return input;
        }
        match self {
            Self::Fade {
                color, progress, ..
            } => input.lerp(*color, *progress),
            Self::Flash {
                color, progress, ..
            } => {
                // Additive-ish flash toward flash color.
                let flashed = Color::rgba(
                    (input.r + color.r * progress).min(1.0),
                    (input.g + color.g * progress).min(1.0),
                    (input.b + color.b * progress).min(1.0),
                    input.a,
                );
                input.lerp(flashed, *progress)
            }
            Self::ColorGrade {
                gain,
                lift,
                gamma,
                progress,
                ..
            } => {
                let g = gamma.max(1e-3);
                let graded = Color::rgba(
                    ((input.r + lift.r).max(0.0).powf(1.0 / g) * gain.r).clamp(0.0, 2.0),
                    ((input.g + lift.g).max(0.0).powf(1.0 / g) * gain.g).clamp(0.0, 2.0),
                    ((input.b + lift.b).max(0.0).powf(1.0 / g) * gain.b).clamp(0.0, 2.0),
                    input.a,
                );
                input.lerp(graded, *progress)
            }
        }
    }
}

/// Ordered stack of post-process effects.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PostProcessStack {
    effects: Vec<PostEffect>,
}

impl PostProcessStack {
    /// Empty stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stack with common defaults: fade, flash, color grade.
    pub fn with_defaults() -> Self {
        Self {
            effects: vec![
                PostEffect::fade_black(),
                PostEffect::Flash {
                    color: Color::WHITE,
                    progress: 0.0,
                    enabled: false,
                },
                PostEffect::color_grade_identity(),
            ],
        }
    }

    /// Push an effect (drawn / applied in order).
    pub fn push(&mut self, effect: PostEffect) {
        self.effects.push(effect);
    }

    /// Effects slice.
    pub fn effects(&self) -> &[PostEffect] {
        &self.effects
    }

    /// Mutable effects.
    pub fn effects_mut(&mut self) -> &mut [PostEffect] {
        &mut self.effects
    }

    /// Number of effects.
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    /// Clear all.
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// First effect of a given kind name (`fade`, `flash`, `color_grade`).
    pub fn find_mut(&mut self, kind: &str) -> Option<&mut PostEffect> {
        self.effects.iter_mut().find(|e| e.kind_name() == kind)
    }

    /// Enable fade to `color` at `progress`.
    pub fn set_fade(&mut self, color: Color, progress: f32) {
        if let Some(e) = self.find_mut("fade") {
            *e = PostEffect::Fade {
                color,
                progress: progress.clamp(0.0, 1.0),
                enabled: progress > 0.0,
            };
        } else {
            self.push(PostEffect::Fade {
                color,
                progress: progress.clamp(0.0, 1.0),
                enabled: progress > 0.0,
            });
        }
    }

    /// Trigger a flash that will decay via [`Self::tick_flash`].
    pub fn trigger_flash(&mut self, color: Color, intensity: f32) {
        if let Some(e) = self.find_mut("flash") {
            *e = PostEffect::Flash {
                color,
                progress: intensity.clamp(0.0, 1.0),
                enabled: true,
            };
        } else {
            self.push(PostEffect::flash_white(intensity));
            if let Some(PostEffect::Flash { color: c, .. }) = self.find_mut("flash") {
                *c = color;
            }
        }
    }

    /// Decay flash progress by `rate` per second.
    pub fn tick_flash(&mut self, dt: f32, rate: f32) {
        if let Some(PostEffect::Flash {
            progress, enabled, ..
        }) = self.find_mut("flash")
        {
            *progress = (*progress - rate.max(0.0) * dt.max(0.0)).max(0.0);
            if *progress <= 0.0 {
                *enabled = false;
            }
        }
    }

    /// Apply the full stack to a sample color (CPU preview / tests).
    pub fn apply_color(&self, mut color: Color) -> Color {
        for e in &self.effects {
            color = e.apply_to_color(color);
        }
        color
    }

    /// Count of enabled effects with progress > 0.
    pub fn active_count(&self) -> usize {
        self.effects
            .iter()
            .filter(|e| e.enabled() && e.progress() > 0.0)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_to_black() {
        let mut stack = PostProcessStack::with_defaults();
        stack.set_fade(Color::BLACK, 1.0);
        let out = stack.apply_color(Color::WHITE);
        assert!(out.r < 0.01);
        assert!(out.g < 0.01);
        assert!(out.b < 0.01);
    }

    #[test]
    fn flash_decays() {
        let mut stack = PostProcessStack::with_defaults();
        stack.trigger_flash(Color::WHITE, 1.0);
        assert_eq!(stack.active_count(), 1);
        stack.tick_flash(0.5, 2.0);
        assert!(stack.find_mut("flash").unwrap().progress() < 0.01);
    }

    #[test]
    fn color_grade_gain() {
        let e = PostEffect::ColorGrade {
            gain: Color::rgb(0.5, 0.5, 0.5),
            lift: Color::rgba(0.0, 0.0, 0.0, 0.0),
            gamma: 1.0,
            progress: 1.0,
            enabled: true,
        };
        let out = e.apply_to_color(Color::WHITE);
        assert!((out.r - 0.5).abs() < 1e-4);
    }

    #[test]
    fn disabled_is_identity() {
        let e = PostEffect::Fade {
            color: Color::RED,
            progress: 1.0,
            enabled: false,
        };
        let c = Color::rgb(0.2, 0.3, 0.4);
        assert_eq!(e.apply_to_color(c), c);
    }
}
