//! UI styling.

use serde::{Deserialize, Serialize};
use velvet_math::Color;

use crate::NinePatchMargins;

/// Asset reference resolved from VCSS `background-image`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiImageSource {
    /// Raster or external vector path from `url("...")`.
    Url(String),
    /// Named inline `@svg` reference from `svg(name)`.
    Svg(String),
}

/// Nine-slice source and destination margins resolved from VCSS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UiNineSlice {
    /// Fixed margins in source texture pixels.
    pub source: NinePatchMargins,
    /// Rendered border thickness in logical UI pixels.
    pub destination: NinePatchMargins,
}

/// Resolved gradient for a background (positions filled 0..=1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiGradient {
    /// True for radial (circle), false for linear.
    pub radial: bool,
    /// Linear angle in degrees (`to bottom` = 180).
    pub angle_deg: f32,
    /// Radial center `[x, y]` 0..=1.
    pub center: [f32; 2],
    /// Stops as (color, position 0..=1).
    pub stops: Vec<(Color, f32)>,
}

/// Resolved `box-shadow` layer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UiBoxShadow {
    /// Offset `(x, y)` in px.
    pub offset: (f32, f32),
    /// Blur radius in px.
    pub blur: f32,
    /// Spread radius in px.
    pub spread: f32,
    /// Shadow color.
    pub color: Color,
    /// Inner shadow.
    pub inset: bool,
}

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
    /// Optional gradient painted over the flat background.
    #[serde(default)]
    pub background_gradient: Option<UiGradient>,
    /// Box shadows (painted outside-to-inside in order).
    #[serde(default)]
    pub box_shadows: Vec<UiBoxShadow>,
    /// Optional image painted behind widget content.
    pub background_image: Option<UiImageSource>,
    /// Optional nine-slice parameters for the background image.
    pub nine_slice: Option<UiNineSlice>,
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
    /// Gap between children in row, column, and grid layouts.
    #[serde(default = "default_gap")]
    pub gap: f32,
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
            background_gradient: None,
            box_shadows: Vec::new(),
            background_image: None,
            nine_slice: None,
            border: Color::rgba(1.0, 1.0, 1.0, 0.15),
            border_width: 1.0,
            radius: 6.0,
            padding: (8.0, 8.0, 8.0, 8.0),
            margin: (0.0, 0.0, 0.0, 0.0),
            gap: default_gap(),
            min_size: (0.0, 0.0),
            max_size: (0.0, 0.0),
            flex_grow: 0.0,
            opacity: 1.0,
            text_color: Color::WHITE,
            font_size: 18.0,
        }
    }
}

fn default_gap() -> f32 {
    8.0
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
