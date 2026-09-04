//! Accessibility: font size, dyslexia font, contrast, reduced motion.
//!
//! One [`AccessibilityPrefs`] struct travels with saves/prefs and tells
//! hosts how to render text and whether to play animations.

use serde::{Deserialize, Serialize};

/// Accessibility preferences (saved with player prefs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessibilityPrefs {
    /// Font scale multiplier (1.0 = normal).
    pub font_scale: f32,
    /// Extra line spacing multiplier.
    pub line_spacing: f32,
    /// Use dyslexia-friendly font when host has one.
    pub dyslexia_font: bool,
    /// High-contrast text/background.
    pub high_contrast: bool,
    /// Disable shake/wipes/moves (instant cuts + no shake offset).
    pub reduced_motion: bool,
    /// Base text size px at scale 1.0.
    pub base_text_px: f32,
}

impl Default for AccessibilityPrefs {
    fn default() -> Self {
        Self {
            font_scale: 1.0,
            line_spacing: 1.0,
            dyslexia_font: false,
            high_contrast: false,
            reduced_motion: false,
            base_text_px: 18.0,
        }
    }
}

impl AccessibilityPrefs {
    /// Validated copy.
    pub fn validated(mut self) -> Self {
        self.font_scale = self.font_scale.clamp(0.8, 3.0);
        self.line_spacing = self.line_spacing.clamp(1.0, 2.5);
        self.base_text_px = self.base_text_px.clamp(12.0, 48.0);
        self
    }

    /// Effective text size px.
    pub fn text_px(&self) -> f32 {
        (self.base_text_px * self.font_scale).clamp(10.0, 96.0)
    }

    /// Effective line height px.
    pub fn line_height_px(&self) -> f32 {
        self.text_px() * 1.35 * self.line_spacing
    }

    /// Whether hosts should animate (false = instant).
    pub fn should_animate(&self) -> bool {
        !self.reduced_motion
    }

    /// Font family hint for hosts.
    pub fn font_family(&self) -> &'static str {
        if self.dyslexia_font {
            "opendyslexic"
        } else {
            "default"
        }
    }

    /// (fg, bg) packed RGB for say box under current contrast setting.
    pub fn say_colors(&self) -> ((u8, u8, u8), (u8, u8, u8)) {
        if self.high_contrast {
            ((255, 255, 255), (0, 0, 0))
        } else {
            ((235, 230, 245), (24, 20, 34))
        }
    }

    /// Zero-out shake offsets when reduced motion.
    pub fn shake_scale(&self) -> f32 {
        if self.reduced_motion {
            0.0
        } else {
            1.0
        }
    }

    /// Force instant transition durations when reduced motion.
    pub fn transition_secs(&self, secs: f32) -> f32 {
        if self.reduced_motion {
            0.0
        } else {
            secs.max(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_and_clamps() {
        let a = AccessibilityPrefs {
            font_scale: 2.0,
            ..Default::default()
        }
        .validated();
        assert!((a.text_px() - 36.0).abs() < 1e-4);
        assert!(a.should_animate());
        let r = AccessibilityPrefs {
            reduced_motion: true,
            font_scale: 99.0,
            ..Default::default()
        }
        .validated();
        assert!(!r.should_animate());
        assert_eq!(r.shake_scale(), 0.0);
        assert_eq!(r.transition_secs(0.5), 0.0);
        assert!((r.text_px() - 54.0).abs() < 1e-4);
    }

    #[test]
    fn contrast_and_font() {
        let hc = AccessibilityPrefs {
            high_contrast: true,
            dyslexia_font: true,
            ..Default::default()
        };
        assert_eq!(hc.say_colors().0, (255, 255, 255));
        assert_eq!(hc.font_family(), "opendyslexic");
        assert_eq!(AccessibilityPrefs::default().font_family(), "default");
    }
}
