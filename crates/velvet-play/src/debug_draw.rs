//! Collect debug draw primitives for a single frame.

use serde::{Deserialize, Serialize};
use velvet_math::{Color, Vec2};

/// A debug line segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugLine {
    /// Start.
    pub a: Vec2,
    /// End.
    pub b: Vec2,
    /// Color.
    pub color: Color,
    /// Thickness in pixels / world units (renderer interprets).
    pub thickness: f32,
}

/// A debug axis-aligned rectangle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugRect {
    /// Min corner.
    pub min: Vec2,
    /// Max corner.
    pub max: Vec2,
    /// Color.
    pub color: Color,
    /// When true, fill; otherwise stroke.
    pub filled: bool,
    /// Stroke thickness when not filled.
    pub thickness: f32,
}

/// A debug circle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugCircle {
    /// Center.
    pub center: Vec2,
    /// Radius.
    pub radius: f32,
    /// Color.
    pub color: Color,
    /// Filled.
    pub filled: bool,
    /// Stroke thickness.
    pub thickness: f32,
    /// Segment count hint for tessellation.
    pub segments: u32,
}

/// One debug primitive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugPrim {
    /// Line.
    Line(DebugLine),
    /// Rect.
    Rect(DebugRect),
    /// Circle.
    Circle(DebugCircle),
}

/// Frame-scoped debug draw list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugDraw {
    /// Primitives for this frame.
    prims: Vec<DebugPrim>,
    /// When false, add_* are no-ops.
    pub enabled: bool,
}

impl DebugDraw {
    /// Create enabled list.
    pub fn new() -> Self {
        Self {
            prims: Vec::new(),
            enabled: true,
        }
    }

    /// Disabled list (no storage).
    pub fn disabled() -> Self {
        Self {
            prims: Vec::new(),
            enabled: false,
        }
    }

    /// Clear for next frame.
    pub fn clear(&mut self) {
        self.prims.clear();
    }

    /// All primitives.
    pub fn primitives(&self) -> &[DebugPrim] {
        &self.prims
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.prims.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.prims.is_empty()
    }

    /// Add a line.
    pub fn line(&mut self, a: Vec2, b: Vec2, color: Color) {
        self.line_thick(a, b, color, 1.0);
    }

    /// Add a thick line.
    pub fn line_thick(&mut self, a: Vec2, b: Vec2, color: Color, thickness: f32) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Line(DebugLine {
            a,
            b,
            color,
            thickness,
        }));
    }

    /// Stroke a rect from min/max.
    pub fn rect(&mut self, min: Vec2, max: Vec2, color: Color) {
        self.rect_ex(min, max, color, false, 1.0);
    }

    /// Filled rect.
    pub fn rect_filled(&mut self, min: Vec2, max: Vec2, color: Color) {
        self.rect_ex(min, max, color, true, 1.0);
    }

    /// Rect with options.
    pub fn rect_ex(&mut self, min: Vec2, max: Vec2, color: Color, filled: bool, thickness: f32) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Rect(DebugRect {
            min,
            max,
            color,
            filled,
            thickness,
        }));
    }

    /// Stroke circle.
    pub fn circle(&mut self, center: Vec2, radius: f32, color: Color) {
        self.circle_ex(center, radius, color, false, 1.0, 24);
    }

    /// Filled circle.
    pub fn circle_filled(&mut self, center: Vec2, radius: f32, color: Color) {
        self.circle_ex(center, radius, color, true, 1.0, 24);
    }

    /// Circle with options.
    pub fn circle_ex(
        &mut self,
        center: Vec2,
        radius: f32,
        color: Color,
        filled: bool,
        thickness: f32,
        segments: u32,
    ) {
        if !self.enabled {
            return;
        }
        self.prims.push(DebugPrim::Circle(DebugCircle {
            center,
            radius,
            color,
            filled,
            thickness,
            segments: segments.max(3),
        }));
    }

    /// Draw an arrow from `from` toward `to`.
    pub fn arrow(&mut self, from: Vec2, to: Vec2, color: Color) {
        if !self.enabled {
            return;
        }
        self.line(from, to, color);
        let dir = to - from;
        let len = dir.length();
        if len < 1e-4 {
            return;
        }
        let n = dir * (1.0 / len);
        let perp = Vec2::new(-n.y, n.x);
        let head = 6.0_f32.min(len * 0.3);
        let base = to - n * head;
        self.line(to, base + perp * (head * 0.5), color);
        self.line(to, base - perp * (head * 0.5), color);
    }

    /// Cross marker at position.
    pub fn cross(&mut self, pos: Vec2, size: f32, color: Color) {
        let h = size * 0.5;
        self.line(pos + Vec2::new(-h, 0.0), pos + Vec2::new(h, 0.0), color);
        self.line(pos + Vec2::new(0.0, -h), pos + Vec2::new(0.0, h), color);
    }

    /// Drain primitives (take ownership for the renderer).
    pub fn drain(&mut self) -> Vec<DebugPrim> {
        std::mem::take(&mut self.prims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_and_clear() {
        let mut d = DebugDraw::new();
        d.line(Vec2::ZERO, Vec2::X, Color::RED);
        d.rect(Vec2::ZERO, Vec2::ONE, Color::GREEN);
        d.circle(Vec2::ZERO, 5.0, Color::BLUE);
        d.arrow(Vec2::ZERO, Vec2::new(10.0, 0.0), Color::WHITE);
        d.cross(Vec2::ZERO, 4.0, Color::VELVET);
        assert!(d.len() >= 5);
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn disabled_noop() {
        let mut d = DebugDraw::disabled();
        d.line(Vec2::ZERO, Vec2::ONE, Color::WHITE);
        assert!(d.is_empty());
    }

    #[test]
    fn drain_takes() {
        let mut d = DebugDraw::new();
        d.circle_filled(Vec2::ZERO, 1.0, Color::RED);
        let prims = d.drain();
        assert_eq!(prims.len(), 1);
        assert!(d.is_empty());
    }
}
