//! UI styling.

use serde::{Deserialize, Serialize};
use velvet_math::Color;

/// Color with optional name.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UiColor(pub Color);

impl Default for UiColor {
    fn default() -> Self {
        Self(Color::WHITE)
    }
}

/// Box style.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiStyle {
    /// Background.
    pub background: Color,
    /// Border color.
    pub border: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub radius: f32,
    /// Padding.
    pub padding: (f32, f32, f32, f32),
    /// Margin.
    pub margin: (f32, f32, f32, f32),
    /// Min size.
    pub min_size: (f32, f32),
    /// Max size (0 = none).
    pub max_size: (f32, f32),
    /// Flex grow.
    pub flex_grow: f32,
    /// Opacity.
    pub opacity: f32,
    /// Text color.
    pub text_color: Color,
    /// Font size.
    pub font_size: f32,
}

impl Default for UiStyle {
    fn default() -> Self {
        Self {
            background: Color::rgba(0.12, 0.12, 0.16, 0.92),
            border: Color::rgba(1.0, 1.0, 1.0, 0.15),
            border_width: 1.0,
            radius: 6.0,
            padding: (8.0, 8.0, 8.0, 8.0),
            margin: (0.0, 0.0, 0.0, 0.0),
            min_size: (0.0, 0.0),
            max_size: (0.0, 0.0),
            flex_grow: 0.0,
            opacity: 1.0,
            text_color: Color::WHITE,
            font_size: 18.0,
        }
    }
}

impl UiStyle {
    /// Dialogue panel preset.
    pub fn dialogue_panel() -> Self {
        Self {
            background: Color::rgba(0.05, 0.05, 0.1, 0.88),
            padding: (24.0, 20.0, 24.0, 20.0),
            radius: 12.0,
            ..Default::default()
        }
    }

    /// Button preset.
    pub fn button() -> Self {
        Self {
            background: Color::rgba(0.25, 0.2, 0.35, 1.0),
            padding: (16.0, 10.0, 16.0, 10.0),
            min_size: (80.0, 36.0),
            radius: 8.0,
            ..Default::default()
        }
    }
}
