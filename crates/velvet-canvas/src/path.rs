//! Paths in logical pixels: lines, curves and the usual shapes, built once and filled or stroked any number of times.

use crate::geometry::{Point, Rect};

/// A finished path.
#[derive(Debug, Clone)]
pub struct Path(pub(crate) tiny_skia::Path);

impl Path {
    /// A rectangle with rounded corners (`radius` is clamped to half the shorter side).
    pub fn round_rect(r: Rect, radius: f32) -> Option<Path> {
        let mut b = PathBuilder::new();
        b.round_rect(r, radius);
        b.finish()
    }

    /// A circle.
    pub fn circle(center: Point, radius: f32) -> Option<Path> {
        tiny_skia::PathBuilder::from_circle(center.x, center.y, radius).map(Path)
    }

    /// A closed polygon through the points.
    pub fn polygon(points: &[Point]) -> Option<Path> {
        let mut b = PathBuilder::new();
        let (first, rest) = points.split_first()?;
        b.move_to(first.x, first.y);
        for p in rest {
            b.line_to(p.x, p.y);
        }
        b.close();
        b.finish()
    }

    /// An open line through the points.
    pub fn polyline(points: &[Point]) -> Option<Path> {
        let mut b = PathBuilder::new();
        let (first, rest) = points.split_first()?;
        b.move_to(first.x, first.y);
        for p in rest {
            b.line_to(p.x, p.y);
        }
        b.finish()
    }

    /// The smallest rectangle around the path.
    pub fn bounds(&self) -> Rect {
        let b = self.0.bounds();
        Rect::new(b.x(), b.y(), b.width(), b.height())
    }
}

/// Builds a path: moves, lines, curves and whole shapes.
#[derive(Debug, Default)]
pub struct PathBuilder(tiny_skia::PathBuilder);

impl PathBuilder {
    pub fn new() -> Self {
        Self(tiny_skia::PathBuilder::new())
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.0.move_to(x, y);
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.0.line_to(x, y);
        self
    }

    pub fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) -> &mut Self {
        self.0.quad_to(x1, y1, x, y);
        self
    }

    pub fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) -> &mut Self {
        self.0.cubic_to(x1, y1, x2, y2, x, y);
        self
    }

    pub fn close(&mut self) -> &mut Self {
        self.0.close();
        self
    }

    /// Adds a rectangle with rounded corners as a closed contour.
    pub fn round_rect(&mut self, r: Rect, radius: f32) -> &mut Self {
        let radius = radius.max(0.0).min(r.w / 2.0).min(r.h / 2.0);
        if radius <= 0.0 {
            if let Some(rect) = r.to_skia() {
                self.0.push_rect(rect);
            }
            return self;
        }
        // A quarter circle as a cubic: the control points sit at 0.5523 of the radius.
        let c = radius * (1.0 - 0.552_284_8);
        let (x, y, w, h) = (r.x, r.y, r.w, r.h);
        self.0.move_to(x + radius, y);
        self.0.line_to(x + w - radius, y);
        self.0.cubic_to(x + w - c, y, x + w, y + c, x + w, y + radius);
        self.0.line_to(x + w, y + h - radius);
        self.0.cubic_to(x + w, y + h - c, x + w - c, y + h, x + w - radius, y + h);
        self.0.line_to(x + radius, y + h);
        self.0.cubic_to(x + c, y + h, x, y + h - c, x, y + h - radius);
        self.0.line_to(x, y + radius);
        self.0.cubic_to(x, y + c, x + c, y, x + radius, y);
        self.0.close();
        self
    }

    /// Adds a circle as a closed contour.
    pub fn circle(&mut self, center: Point, radius: f32) -> &mut Self {
        self.0.push_circle(center.x, center.y, radius);
        self
    }

    /// The path, or None when nothing was added.
    pub fn finish(self) -> Option<Path> {
        self.0.finish().map(Path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_rect_stays_inside_its_rectangle() {
        let p = Path::round_rect(Rect::new(10.0, 20.0, 100.0, 50.0), 12.0).unwrap();
        let b = p.bounds();
        assert!((b.x - 10.0).abs() < 0.01 && (b.y - 20.0).abs() < 0.01);
        assert!((b.w - 100.0).abs() < 0.01 && (b.h - 50.0).abs() < 0.01);
    }

    #[test]
    fn empty_shapes_give_no_path() {
        assert!(Path::polygon(&[]).is_none());
        assert!(PathBuilder::new().finish().is_none());
    }
}
