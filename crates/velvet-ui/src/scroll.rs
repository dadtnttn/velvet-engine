//! Scroll view offset clamping and wheel input helpers.

use velvet_math::Vec2;

/// Scrollable viewport state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollView {
    /// Viewport size.
    pub viewport: Vec2,
    /// Full content size.
    pub content: Vec2,
    /// Scroll offset (positive = content moved up/left).
    pub offset: Vec2,
    /// Enable horizontal scrolling.
    pub scroll_x: bool,
    /// Enable vertical scrolling.
    pub scroll_y: bool,
}

impl Default for ScrollView {
    fn default() -> Self {
        Self {
            viewport: Vec2::new(100.0, 100.0),
            content: Vec2::new(100.0, 100.0),
            offset: Vec2::ZERO,
            scroll_x: false,
            scroll_y: true,
        }
    }
}

impl ScrollView {
    /// Create vertical scroll view.
    pub fn vertical(viewport: Vec2, content: Vec2) -> Self {
        Self {
            viewport,
            content,
            offset: Vec2::ZERO,
            scroll_x: false,
            scroll_y: true,
        }
    }

    /// Maximum scroll offset (content - viewport, floored at 0).
    pub fn max_offset(&self) -> Vec2 {
        Vec2::new(
            if self.scroll_x {
                (self.content.x - self.viewport.x).max(0.0)
            } else {
                0.0
            },
            if self.scroll_y {
                (self.content.y - self.viewport.y).max(0.0)
            } else {
                0.0
            },
        )
    }

    /// Clamp `offset` into valid range.
    pub fn clamp_offset(mut self) -> Self {
        let max = self.max_offset();
        self.offset.x = self.offset.x.clamp(0.0, max.x);
        self.offset.y = self.offset.y.clamp(0.0, max.y);
        self
    }

    /// Set offset and clamp.
    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset;
        *self = self.clamp_offset();
    }

    /// Apply wheel delta (typically pixels); Y positive scrolls down content.
    pub fn apply_wheel(&mut self, delta: Vec2) {
        self.offset.x += if self.scroll_x { delta.x } else { 0.0 };
        self.offset.y += if self.scroll_y { -delta.y } else { 0.0 };
        *self = self.clamp_offset();
    }

    /// Scroll by logical lines (`line_height` pixels each).
    pub fn scroll_lines(&mut self, lines: f32, line_height: f32) {
        self.apply_wheel(Vec2::new(0.0, lines * line_height));
    }

    /// Normalized scroll `0..=1` for vertical bar.
    pub fn normalized_y(&self) -> f32 {
        let max = self.max_offset().y;
        if max <= 1e-5 {
            0.0
        } else {
            (self.offset.y / max).clamp(0.0, 1.0)
        }
    }

    /// Set from normalized vertical `0..=1`.
    pub fn set_normalized_y(&mut self, t: f32) {
        let max = self.max_offset().y;
        self.offset.y = t.clamp(0.0, 1.0) * max;
    }

    /// Whether content overflows viewport on Y.
    pub fn overflows_y(&self) -> bool {
        self.content.y > self.viewport.y + 0.5
    }

    /// Content origin for drawing (viewport pos assumed 0; subtract offset).
    pub fn content_origin(&self) -> Vec2 {
        Vec2::new(-self.offset.x, -self.offset.y)
    }
}

/// Clamp a raw scroll offset given sizes (standalone helper).
pub fn clamp_scroll_offset(
    offset: Vec2,
    viewport: Vec2,
    content: Vec2,
    scroll_x: bool,
    scroll_y: bool,
) -> Vec2 {
    ScrollView {
        viewport,
        content,
        offset,
        scroll_x,
        scroll_y,
    }
    .clamp_offset()
    .offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_max() {
        let mut s = ScrollView::vertical(Vec2::new(100.0, 100.0), Vec2::new(100.0, 400.0));
        s.set_offset(Vec2::new(0.0, 999.0));
        assert!((s.offset.y - 300.0).abs() < 1e-4);
        s.set_offset(Vec2::new(0.0, -50.0));
        assert!((s.offset.y).abs() < 1e-4);
    }

    #[test]
    fn wheel_scrolls() {
        let mut s = ScrollView::vertical(Vec2::new(50.0, 50.0), Vec2::new(50.0, 200.0));
        s.apply_wheel(Vec2::new(0.0, -20.0)); // negative y wheel → scroll down
        assert!(s.offset.y > 0.0);
    }

    #[test]
    fn no_overflow_stays_zero() {
        let s = ScrollView::vertical(Vec2::new(100.0, 100.0), Vec2::new(80.0, 80.0));
        assert!(!s.overflows_y());
        assert_eq!(s.max_offset(), Vec2::ZERO);
    }
}
