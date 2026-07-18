//! Text styles and effects.

use serde::{Deserialize, Serialize};
use velvet_math::Color;

/// Font weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontWeight {
    /// Thin.
    Thin,
    /// Regular.
    #[default]
    Regular,
    /// Bold.
    Bold,
}

/// Visual text effects.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum TextEffect {
    /// No effect.
    #[default]
    None,
    /// Shake with intensity.
    Shake {
        /// Intensity pixels.
        intensity: f32,
    },
    /// Wave vertical motion.
    Wave {
        /// Amplitude.
        amplitude: f32,
        /// Frequency.
        frequency: f32,
    },
    /// Shadow offset.
    Shadow {
        /// Offset x/y.
        offset: (f32, f32),
        /// Shadow color.
        color: Color,
    },
    /// Outline thickness.
    Outline {
        /// Thickness.
        thickness: f32,
        /// Color.
        color: Color,
    },
}

/// Complete style for a span.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextStyle {
    /// Font family key.
    pub font: String,
    /// Size in pixels.
    pub size: f32,
    /// Color.
    pub color: Color,
    /// Weight.
    pub weight: FontWeight,
    /// Italic.
    pub italic: bool,
    /// Underline.
    pub underline: bool,
    /// Effect.
    pub effect: TextEffect,
    /// Letter spacing.
    pub letter_spacing: f32,
    /// Line height multiplier.
    pub line_height: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: "default".into(),
            size: 24.0,
            color: Color::WHITE,
            weight: FontWeight::Regular,
            italic: false,
            underline: false,
            effect: TextEffect::None,
            letter_spacing: 0.0,
            line_height: 1.25,
        }
    }
}

impl TextStyle {
    /// With color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// With size.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Merge override fields from `other` where other has non-default-ish values.
    pub fn overlay(&self, other: &TextStyle) -> TextStyle {
        let mut out = self.clone();
        if other.font != "default" {
            out.font = other.font.clone();
        }
        if (other.size - 24.0).abs() > f32::EPSILON {
            out.size = other.size;
        }
        if other.color != Color::WHITE {
            out.color = other.color;
        }
        out.weight = other.weight;
        out.italic = other.italic || out.italic;
        out.underline = other.underline || out.underline;
        if !matches!(other.effect, TextEffect::None) {
            out.effect = other.effect;
        }
        out
    }
}
