//! Line breaking and alignment.

use crate::markup::{RichSpan, RichText};
use crate::measure::{grapheme_advances, measure_width};
use crate::style::TextStyle;

/// Horizontal alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Left.
    #[default]
    Left,
    /// Center.
    Center,
    /// Right.
    Right,
}

/// One laid-out line.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignedLine {
    /// Spans on this line (text only, split).
    pub text: String,
    /// Style for the line (simplified: dominant style).
    pub style: TextStyle,
    /// X offset after alignment.
    pub x: f32,
    /// Y position.
    pub y: f32,
    /// Measured width.
    pub width: f32,
}

/// Layout result.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextLayout {
    /// Lines.
    pub lines: Vec<AlignedLine>,
    /// Total height.
    pub height: f32,
    /// Max line width.
    pub width: f32,
}

impl TextLayout {
    /// Layout rich text into lines within `max_width`.
    pub fn layout(rich: &RichText, max_width: f32, align: TextAlign) -> Self {
        let plain = rich.plain();
        let style = rich
            .spans
            .iter()
            .find_map(|s| match s {
                RichSpan::Text { style, .. } => Some(style.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let mut lines = Vec::new();
        let mut y = 0.0;
        let line_h = style.size * style.line_height;
        let mut max_w = 0.0f32;

        for paragraph in plain.split('\n') {
            let mut current = String::new();
            for word in paragraph.split_inclusive(char::is_whitespace) {
                let trial = format!("{current}{word}");
                let w = measure_width(&trial, &style);
                if w > max_width && !current.is_empty() {
                    let lw = measure_width(&current, &style);
                    let x = align_x(lw, max_width, align);
                    lines.push(AlignedLine {
                        text: std::mem::take(&mut current),
                        style: style.clone(),
                        x,
                        y,
                        width: lw,
                    });
                    max_w = max_w.max(lw);
                    y += line_h;
                    current.push_str(word.trim_start());
                } else {
                    current = trial;
                }
            }
            if !current.is_empty() || paragraph.is_empty() {
                let lw = measure_width(&current, &style);
                let x = align_x(lw, max_width, align);
                lines.push(AlignedLine {
                    text: current,
                    style: style.clone(),
                    x,
                    y,
                    width: lw,
                });
                max_w = max_w.max(lw);
                y += line_h;
            }
        }

        // Character-level wrap if single word exceeds
        if lines.is_empty() {
            lines.push(AlignedLine {
                text: String::new(),
                style,
                x: 0.0,
                y: 0.0,
                width: 0.0,
            });
        }

        let _ = grapheme_advances; // used by typewriter
        TextLayout {
            height: y,
            width: max_w.min(max_width),
            lines,
        }
    }
}

fn align_x(line_w: f32, max_w: f32, align: TextAlign) -> f32 {
    match align {
        TextAlign::Left => 0.0,
        TextAlign::Center => ((max_w - line_w) * 0.5).max(0.0),
        TextAlign::Right => (max_w - line_w).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::parse_rich_text;

    #[test]
    fn wraps_long_line() {
        let r = parse_rich_text("one two three four five six seven eight nine ten").unwrap();
        let layout = TextLayout::layout(&r, 80.0, TextAlign::Left);
        assert!(layout.lines.len() > 1);
    }
}
