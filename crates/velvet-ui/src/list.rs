//! Virtualized vertical list: window math over N rows.
//!
//! The list never materializes rows itself — hosts ask for
//! [`VirtualList::visible_range`] each frame and build only those row
//! widgets (or draw them directly). Scroll state, selection, and
//! keyboard movement live here so every host behaves the same.

/// Virtual list state over `count` uniform rows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualList {
    /// Total rows.
    pub count: usize,
    /// Row height in px.
    pub row_height: f32,
    /// Scroll offset in px from the top.
    pub scroll: f32,
    /// Selected row.
    pub selected: Option<usize>,
}

impl VirtualList {
    /// New list (empty selection, top scroll).
    pub fn new(count: usize, row_height: f32) -> Self {
        Self {
            count,
            row_height: row_height.max(1.0),
            scroll: 0.0,
            selected: None,
        }
    }

    /// Full content height in px.
    pub fn content_height(&self) -> f32 {
        self.count as f32 * self.row_height
    }

    /// Maximum scroll offset for a viewport height.
    pub fn max_scroll(&self, viewport_h: f32) -> f32 {
        (self.content_height() - viewport_h.max(0.0)).max(0.0)
    }

    /// Clamp scroll into range.
    pub fn set_scroll(&mut self, scroll: f32, viewport_h: f32) {
        self.scroll = scroll.clamp(0.0, self.max_scroll(viewport_h));
    }

    /// Scroll by px delta.
    pub fn scroll_by(&mut self, delta: f32, viewport_h: f32) {
        self.set_scroll(self.scroll + delta, viewport_h);
    }

    /// Visible row range for a viewport height (end-exclusive, clamped).
    ///
    /// Always covers partially visible edge rows so hosts never show gaps.
    pub fn visible_range(&self, viewport_h: f32) -> std::ops::Range<usize> {
        if self.count == 0 || viewport_h <= 0.0 {
            return 0..0;
        }
        let first = (self.scroll / self.row_height).floor() as usize;
        let first = first.min(self.count.saturating_sub(1));
        let shown = (viewport_h / self.row_height).ceil() as usize + 1;
        let end = (first + shown).min(self.count);
        first..end.max(first)
    }

    /// Y offset of a row relative to the viewport top.
    pub fn row_y(&self, index: usize) -> f32 {
        index as f32 * self.row_height - self.scroll
    }

    /// Row index under a viewport-relative y, if any.
    pub fn row_at_y(&self, y: f32, viewport_h: f32) -> Option<usize> {
        if y < 0.0 || y >= viewport_h {
            return None;
        }
        let index = ((y + self.scroll) / self.row_height).floor() as usize;
        (index < self.count).then_some(index)
    }

    /// Select a row (clamped; `None` clears).
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index.filter(|i| *i < self.count);
    }

    /// Move selection by steps, clamping at the ends.
    pub fn move_selection(&mut self, steps: i32) {
        if self.count == 0 {
            self.selected = None;
            return;
        }
        let current = self.selected.unwrap_or(0) as i32;
        let next = (current + steps).clamp(0, self.count as i32 - 1) as usize;
        self.selected = Some(next);
    }

    /// Scroll just enough to reveal `index` for a viewport height.
    pub fn ensure_visible(&mut self, index: usize, viewport_h: f32) {
        if index >= self.count {
            return;
        }
        let top = index as f32 * self.row_height;
        let bottom = top + self.row_height;
        if top < self.scroll {
            self.set_scroll(top, viewport_h);
        } else if bottom > self.scroll + viewport_h {
            self.set_scroll(bottom - viewport_h, viewport_h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_covers_partial_edge_rows() {
        // 100 rows of 20px, viewport 100px, scrolled 10px: rows 0..6
        // (row 0 half-cut at top, row 5 half-cut at bottom).
        let list = VirtualList {
            count: 100,
            row_height: 20.0,
            scroll: 10.0,
            selected: None,
        };
        assert_eq!(list.visible_range(100.0), 0..6);
        assert_eq!(list.row_y(0), -10.0);
        assert_eq!(list.row_at_y(95.0, 100.0), Some(5));
        assert_eq!(list.row_at_y(100.0, 100.0), None);
    }

    #[test]
    fn empty_or_zero_viewport_shows_nothing() {
        let empty = VirtualList::new(0, 20.0);
        assert_eq!(empty.visible_range(100.0), 0..0);
        let list = VirtualList::new(10, 20.0);
        assert_eq!(list.visible_range(0.0), 0..0);
    }

    #[test]
    fn selection_moves_and_reveals() {
        let mut list = VirtualList::new(10, 20.0);
        list.move_selection(3);
        assert_eq!(list.selected, Some(3));
        list.move_selection(100);
        assert_eq!(list.selected, Some(9));
        list.move_selection(-100);
        assert_eq!(list.selected, Some(0));
        list.select(Some(9));
        list.ensure_visible(9, 100.0);
        // Last row bottom (200) minus viewport (100) = scroll 100.
        assert!((list.scroll - 100.0).abs() < 1e-4);
        assert!(list.visible_range(100.0).contains(&9));
        list.select(Some(99));
        assert_eq!(list.selected, None);
    }

    #[test]
    fn scroll_clamps_to_content() {
        let mut list = VirtualList::new(5, 20.0);
        assert_eq!(list.max_scroll(100.0), 0.0);
        list.set_scroll(50.0, 100.0);
        assert_eq!(list.scroll, 0.0);
        assert_eq!(list.visible_range(100.0), 0..5);
        let mut big = VirtualList::new(100, 20.0);
        big.scroll_by(1e6, 100.0);
        assert!((big.scroll - 1900.0).abs() < 1e-4);
        big.scroll_by(-1e6, 100.0);
        assert_eq!(big.scroll, 0.0);
    }
}
