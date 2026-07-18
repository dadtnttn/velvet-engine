//! Text measurement — product path uses real shaping ([`crate::shape`]).

use crate::shape::{shape_measure_width, shape_text};
use crate::style::TextStyle;

/// Metrics for a string under a style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMetrics {
    /// Width in pixels.
    pub width: f32,
    /// Height in pixels.
    pub height: f32,
    /// Baseline from top.
    pub baseline: f32,
}

/// Per-glyph placeholder metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphMetrics {
    /// Advance width.
    pub advance: f32,
    /// Height.
    pub height: f32,
}

/// Measure width using the shaping path (rustybuzz when font available, else engine clusters).
pub fn measure_width(text: &str, style: &TextStyle) -> f32 {
    shape_measure_width(text, style)
}

/// Measure full metrics.
#[allow(dead_code)]
pub fn measure(text: &str, style: &TextStyle) -> TextMetrics {
    TextMetrics {
        width: measure_width(text, style),
        height: style.size * style.line_height,
        baseline: style.size * 0.8,
    }
}

/// Cluster / glyph advances from the active shaper.
pub fn grapheme_advances(text: &str, style: &TextStyle) -> Vec<(String, f32)> {
    shape_text(text, style)
        .glyphs
        .into_iter()
        .map(|g| (g.cluster, g.advance))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wider_text_has_more_width() {
        let s = TextStyle::default();
        assert!(measure_width("hello world", &s) > measure_width("hi", &s));
    }
}
