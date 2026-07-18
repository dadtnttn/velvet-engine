//! CPU debug overlay: stats text lines and shape draw list.

use velvet_math::{Color, Vec2};

use crate::stats::RenderStats;

/// A single line of debug text (screen-space).
#[derive(Debug, Clone, PartialEq)]
pub struct DebugTextLine {
    /// Text content.
    pub text: String,
    /// Screen position (logical pixels).
    pub position: Vec2,
    /// Color.
    pub color: Color,
    /// Font size in pixels.
    pub size: f32,
}

impl DebugTextLine {
    /// Create a line at position.
    pub fn new(text: impl Into<String>, position: Vec2) -> Self {
        Self {
            text: text.into(),
            position,
            color: Color::rgb(0.85, 0.95, 0.85),
            size: 14.0,
        }
    }
}

/// Debug shape primitives (screen or world space; consumer decides).
#[derive(Debug, Clone, PartialEq)]
pub enum DebugShape {
    /// Axis-aligned rect outline or fill.
    Rect {
        /// Min corner.
        min: Vec2,
        /// Max corner.
        max: Vec2,
        /// Color.
        color: Color,
        /// Filled when true.
        filled: bool,
        /// Stroke width when not filled.
        thickness: f32,
    },
    /// Circle.
    Circle {
        /// Center.
        center: Vec2,
        /// Radius.
        radius: f32,
        /// Color.
        color: Color,
        /// Filled.
        filled: bool,
        /// Stroke width.
        thickness: f32,
    },
    /// Line segment.
    Line {
        /// Start.
        a: Vec2,
        /// End.
        b: Vec2,
        /// Color.
        color: Color,
        /// Thickness.
        thickness: f32,
    },
    /// Cross / point marker.
    Cross {
        /// Center.
        center: Vec2,
        /// Half extent.
        size: f32,
        /// Color.
        color: Color,
    },
}

/// Frame debug overlay: text + shapes + embedded stats snapshot.
#[derive(Debug, Clone, Default)]
pub struct DebugOverlay {
    /// Text lines for this frame.
    pub lines: Vec<DebugTextLine>,
    /// Shapes for this frame.
    pub shapes: Vec<DebugShape>,
    /// Whether overlay is drawn.
    pub enabled: bool,
    /// Last stats snapshot used for auto lines.
    pub stats: RenderStats,
}

impl DebugOverlay {
    /// Create disabled overlay.
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Clear per-frame geometry (keeps enabled flag).
    pub fn clear(&mut self) {
        self.lines.clear();
        self.shapes.clear();
    }

    /// Push a text line.
    pub fn text(&mut self, text: impl Into<String>, position: Vec2) {
        self.lines.push(DebugTextLine::new(text, position));
    }

    /// Push a colored text line.
    pub fn text_colored(&mut self, text: impl Into<String>, position: Vec2, color: Color) {
        self.lines.push(DebugTextLine {
            text: text.into(),
            position,
            color,
            size: 14.0,
        });
    }

    /// Push a shape.
    pub fn shape(&mut self, shape: DebugShape) {
        self.shapes.push(shape);
    }

    /// Axis-aligned rect outline.
    pub fn rect_outline(&mut self, min: Vec2, max: Vec2, color: Color) {
        self.shapes.push(DebugShape::Rect {
            min,
            max,
            color,
            filled: false,
            thickness: 1.0,
        });
    }

    /// Filled rect.
    pub fn rect_filled(&mut self, min: Vec2, max: Vec2, color: Color) {
        self.shapes.push(DebugShape::Rect {
            min,
            max,
            color,
            filled: true,
            thickness: 0.0,
        });
    }

    /// Line.
    pub fn line(&mut self, a: Vec2, b: Vec2, color: Color) {
        self.shapes.push(DebugShape::Line {
            a,
            b,
            color,
            thickness: 1.0,
        });
    }

    /// Circle outline.
    pub fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.shapes.push(DebugShape::Circle {
            center,
            radius,
            color,
            filled: false,
            thickness: 1.0,
        });
    }

    /// Snapshot stats and append standard HUD lines at top-left.
    pub fn push_frame_stats(&mut self, stats: &RenderStats, origin: Vec2, line_height: f32) {
        self.stats = stats.clone();
        let rows = [
            format!("sprites: {}", stats.sprites_submitted),
            format!("draws: {}", stats.draw_calls),
            format!("tex binds: {}", stats.texture_binds),
            format!("tris: {}", stats.triangles),
            format!("particles: {}", stats.particles),
            format!("cpu us: {}", stats.cpu_encode_us),
            format!("gpu us: {}", stats.gpu_time_us),
        ];
        for (i, row) in rows.iter().enumerate() {
            self.text(
                row.clone(),
                Vec2::new(origin.x, origin.y + line_height * i as f32),
            );
        }
    }

    /// Number of text lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Number of shapes.
    pub fn shape_count(&self) -> usize {
        self.shapes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stats_lines_and_shapes() {
        let mut overlay = DebugOverlay::new();
        let mut stats = RenderStats {
            sprites_submitted: 12,
            ..Default::default()
        };
        stats.finish_draw_calls(3);
        stats.particles = 40;
        overlay.push_frame_stats(&stats, Vec2::new(8.0, 8.0), 16.0);
        assert!(overlay.line_count() >= 5);
        assert!(overlay.lines.iter().any(|l| l.text.contains("sprites: 12")));
        overlay.rect_outline(Vec2::ZERO, Vec2::new(10.0, 10.0), Color::RED);
        overlay.line(Vec2::ZERO, Vec2::ONE, Color::GREEN);
        assert_eq!(overlay.shape_count(), 2);
        overlay.clear();
        assert_eq!(overlay.line_count(), 0);
        assert_eq!(overlay.shape_count(), 0);
    }
}
