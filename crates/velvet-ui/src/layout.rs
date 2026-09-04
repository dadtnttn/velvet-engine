//! Layout engines: stack, row, column, grid.

use velvet_math::Vec2;

use crate::node::{NodeId, UiNode, UiRect};

/// Layout algorithm for children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LayoutType {
    /// Overlay stack (same origin + padding).
    #[default]
    Stack,
    /// Horizontal row (`wrap` flows onto next lines).
    Row {
        /// Wrap onto next line when exceeding content width.
        #[serde(default)]
        wrap: bool,
    },
    /// Vertical column (`wrap` flows into next columns).
    Column {
        /// Wrap into next column when exceeding content height.
        #[serde(default)]
        wrap: bool,
    },
    /// Grid with columns.
    Grid {
        /// Column count.
        columns: u32,
    },
    /// Vertical column scrolled by the container's `scroll` offset.
    Scroll,
}

/// Anchor point inside a parent rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Anchor {
    /// Top-left.
    TopLeft,
    /// Top edge, horizontally centered.
    #[default]
    TopCenter,
    /// Top-right.
    TopRight,
    /// Left edge, vertically centered.
    CenterLeft,
    /// Centered both ways.
    Center,
    /// Right edge, vertically centered.
    CenterRight,
    /// Bottom-left.
    BottomLeft,
    /// Bottom edge, horizontally centered.
    BottomCenter,
    /// Bottom-right.
    BottomRight,
}

/// Constraints for layout.
#[derive(Debug, Clone, Copy)]
pub struct LayoutConstraints {
    /// Available size.
    pub available: Vec2,
}

/// Result metrics.
#[derive(Debug, Clone, Copy)]
pub struct LayoutResult {
    /// Used size.
    pub size: Vec2,
}

/// Layout children into parent rect; updates child.rect.
pub fn layout_children(
    parent: &UiNode,
    children: &mut [&mut UiNode],
    parent_rect: UiRect,
) -> LayoutResult {
    let pad = parent.style.padding;
    let content = UiRect {
        pos: Vec2::new(parent_rect.pos.x + pad.0, parent_rect.pos.y + pad.1),
        size: Vec2::new(
            (parent_rect.size.x - pad.0 - pad.2).max(0.0),
            (parent_rect.size.y - pad.1 - pad.3).max(0.0),
        ),
    };

    // Content extent (bottom-right past the content origin); Scroll hosts
    // use it for scrollbar math instead of guessing from child counts.
    let mut extent = Vec2::ZERO;
    let mut track = |pos: Vec2, size: Vec2| {
        extent.x = extent.x.max(pos.x + size.x - content.pos.x);
        extent.y = extent.y.max(pos.y + size.y - content.pos.y);
    };

    match parent.layout {
        LayoutType::Stack => {
            for c in children.iter_mut() {
                let m = c.style.margin;
                c.rect = UiRect {
                    pos: Vec2::new(content.pos.x + m.0, content.pos.y + m.1),
                    size: measure_child(c, content.size),
                };
                track(c.rect.pos, c.rect.size);
            }
        }
        LayoutType::Row { wrap } => {
            let gap = parent.style.gap.max(0.0);
            let right = content.pos.x + content.size.x;
            let mut x = content.pos.x;
            let mut y = content.pos.y;
            let mut line_h = 0.0f32;
            for c in children.iter_mut() {
                let m = c.style.margin;
                let size = measure_child(c, content.size);
                let total_w = size.x + m.0 + m.2;
                if wrap && x > content.pos.x && x + total_w > right + 1e-4 {
                    x = content.pos.x;
                    y += line_h + gap;
                    line_h = 0.0;
                }
                c.rect = UiRect {
                    pos: Vec2::new(x + m.0, y + m.1),
                    size,
                };
                track(c.rect.pos, size);
                x += total_w + gap;
                line_h = line_h.max(size.y + m.1 + m.3);
            }
        }
        LayoutType::Column { wrap } => {
            let gap = parent.style.gap.max(0.0);
            let bottom = content.pos.y + content.size.y;
            let mut x = content.pos.x;
            let mut y = content.pos.y;
            let mut col_w = 0.0f32;
            for c in children.iter_mut() {
                let m = c.style.margin;
                let size = measure_child(c, content.size);
                let total_h = size.y + m.1 + m.3;
                if wrap && y > content.pos.y && y + total_h > bottom + 1e-4 {
                    y = content.pos.y;
                    x += col_w + gap;
                    col_w = 0.0;
                }
                c.rect = UiRect {
                    pos: Vec2::new(x + m.0, y + m.1),
                    size,
                };
                track(c.rect.pos, size);
                y += total_h + gap;
                col_w = col_w.max(size.x + m.0 + m.2);
            }
        }
        LayoutType::Scroll => {
            let gap = parent.style.gap.max(0.0);
            let mut y = content.pos.y - parent.scroll.y;
            for c in children.iter_mut() {
                let m = c.style.margin;
                let size = measure_child(c, content.size);
                c.rect = UiRect {
                    pos: Vec2::new(content.pos.x + m.0, y + m.1),
                    size,
                };
                // Extent in content space (unscrolled) for scrollbars.
                extent.y = extent
                    .y
                    .max(y + m.1 + size.y - content.pos.y + parent.scroll.y);
                extent.x = extent.x.max(size.x + m.0 + m.2);
                y += size.y + m.1 + m.3 + gap;
            }
        }
        LayoutType::Grid { columns } => {
            let cols = columns.max(1) as f32;
            let gap = parent.style.gap.max(0.0);
            let cell_w = ((content.size.x - gap * (cols - 1.0)) / cols).max(0.0);
            for (i, c) in children.iter_mut().enumerate() {
                let col = (i as u32) % columns.max(1);
                let row = (i as u32) / columns.max(1);
                let size = measure_child(c, Vec2::new(cell_w, content.size.y));
                c.rect = UiRect {
                    pos: Vec2::new(
                        content.pos.x + col as f32 * (cell_w + gap),
                        content.pos.y + row as f32 * (size.y + gap),
                    ),
                    size: Vec2::new(cell_w, size.y),
                };
                track(c.rect.pos, c.rect.size);
            }
        }
    }

    LayoutResult {
        size: Vec2::new(extent.x.max(0.0), extent.y.max(0.0)),
    }
}

fn measure_child(node: &UiNode, available: Vec2) -> Vec2 {
    let mut w = node.style.min_size.0;
    let mut h = node.style.min_size.1;
    match &node.widget {
        crate::widget::WidgetKind::Label { text, .. } => {
            w = w.max(text.len() as f32 * node.style.font_size * 0.5);
            h = h.max(node.style.font_size * 1.4);
        }
        crate::widget::WidgetKind::Button { label, .. } => {
            w = w.max(label.len() as f32 * node.style.font_size * 0.55 + 32.0);
            h = h.max(node.style.font_size * 1.8);
        }
        crate::widget::WidgetKind::ProgressBar { .. }
        | crate::widget::WidgetKind::Slider { .. } => {
            w = w.max(120.0);
            h = h.max(20.0);
        }
        crate::widget::WidgetKind::Toggle { .. } => {
            w = w.max(48.0);
            h = h.max(28.0);
        }
        crate::widget::WidgetKind::Image { preferred, .. } => {
            w = w.max(preferred.x);
            h = h.max(preferred.y);
        }
        crate::widget::WidgetKind::Panel | crate::widget::WidgetKind::TextField { .. } => {
            w = w.max(available.x.min(200.0));
            h = h.max(32.0);
        }
        crate::widget::WidgetKind::Dropdown { options, .. } => {
            let longest = options.iter().map(|o| o.len()).max().unwrap_or(4);
            w = w.max(longest as f32 * node.style.font_size * 0.55 + 40.0);
            h = h.max(node.style.font_size * 1.8);
        }
        crate::widget::WidgetKind::Tabs { tabs, .. } => {
            w = w.max(available.x.min(80.0 * tabs.len().max(1) as f32));
            h = h.max(node.style.font_size * 1.8);
        }
        crate::widget::WidgetKind::Tooltip { .. } => {
            w = w.max(8.0);
            h = h.max(8.0);
        }
    }
    if node.style.max_size.0 > 0.0 {
        w = w.min(node.style.max_size.0);
    }
    if node.style.max_size.1 > 0.0 {
        h = h.min(node.style.max_size.1);
    }
    // Grow
    if node.style.flex_grow > 0.0 {
        w = available.x;
    }
    Vec2::new(w, h)
}

/// Anchor a child rect inside a parent rect with a margin offset.
pub fn anchor_in(parent: UiRect, child_size: Vec2, anchor: Anchor, margin: Vec2) -> UiRect {
    let x = match anchor {
        Anchor::TopLeft | Anchor::CenterLeft | Anchor::BottomLeft => parent.pos.x + margin.x,
        Anchor::TopCenter | Anchor::Center | Anchor::BottomCenter => {
            parent.pos.x + (parent.size.x - child_size.x) * 0.5
        }
        Anchor::TopRight | Anchor::CenterRight | Anchor::BottomRight => {
            parent.pos.x + parent.size.x - child_size.x - margin.x
        }
    };
    let y = match anchor {
        Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => parent.pos.y + margin.y,
        Anchor::CenterLeft | Anchor::Center | Anchor::CenterRight => {
            parent.pos.y + (parent.size.y - child_size.y) * 0.5
        }
        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
            parent.pos.y + parent.size.y - child_size.y - margin.y
        }
    };
    UiRect {
        pos: Vec2::new(x, y),
        size: child_size,
    }
}

/// Anchor helper: place rect at bottom center of parent.
pub fn anchor_bottom(parent: UiRect, child_size: Vec2, margin_bottom: f32) -> UiRect {
    anchor_in(
        parent,
        child_size,
        Anchor::BottomCenter,
        Vec2::new(0.0, margin_bottom),
    )
}

/// Silence unused NodeId in layout-only unit.
#[allow(dead_code)]
fn _id(_: NodeId) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;

    fn button_node(id: u64, label: &str) -> UiNode {
        UiNode::new(
            NodeId(id),
            label,
            WidgetKind::Button {
                label: label.into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
        )
    }

    fn row_parent(width: f32, wrap: bool) -> UiNode {
        let mut parent = UiNode::new(NodeId(0), "root", WidgetKind::Panel);
        parent.layout = LayoutType::Row { wrap };
        parent.rect = UiRect {
            pos: Vec2::ZERO,
            size: Vec2::new(width, 400.0),
        };
        parent
    }

    #[test]
    fn row_wraps_to_next_line() {
        let parent = row_parent(100.0, true);
        // Each button measures ~ (2 chars * 18 * 0.55 + 32) = 51.8 wide.
        let mut a = button_node(1, "aa");
        let mut b = button_node(2, "bb");
        let mut kids = [&mut a, &mut b];
        let result = layout_children(&parent, &mut kids, parent.rect);
        assert!(b.rect.pos.y > a.rect.pos.y, "must wrap");
        // New line restarts at the content origin (panel padding = 8).
        assert!((b.rect.pos.x - 8.0).abs() < 1e-3, "{}", b.rect.pos.x);
        assert!(result.size.y > a.rect.size.y, "extent covers both lines");
        // Same content without wrap overflows on one line.
        let parent = row_parent(100.0, false);
        let mut a = button_node(1, "aa");
        let mut b = button_node(2, "bb");
        let mut kids = [&mut a, &mut b];
        layout_children(&parent, &mut kids, parent.rect);
        assert_eq!(b.rect.pos.y, a.rect.pos.y);
        // Content ends at x=92; the second button sticks out past it.
        assert!(
            b.rect.pos.x + b.rect.size.x > 92.0,
            "overflows without wrap"
        );
    }

    #[test]
    fn scroll_container_offsets_and_measures() {
        let mut parent = UiNode::new(NodeId(0), "root", WidgetKind::Panel);
        parent.layout = LayoutType::Scroll;
        parent.rect = UiRect {
            pos: Vec2::ZERO,
            size: Vec2::new(200.0, 100.0),
        };
        parent.scroll = Vec2::new(0.0, 30.0);
        let mut a = button_node(1, "aa");
        let mut b = button_node(2, "bb");
        let mut kids = [&mut a, &mut b];
        let result = layout_children(&parent, &mut kids, parent.rect);
        // First row shifted up by the scroll offset (content origin y = 8).
        assert!((a.rect.pos.y - -22.0).abs() < 1e-3, "{}", a.rect.pos.y);
        assert!(b.rect.pos.y > a.rect.pos.y);
        // Extent reports unscrolled content height for scrollbars.
        assert!(result.size.y > 60.0, "{}", result.size.y);
    }

    #[test]
    fn anchors_hit_nine_points() {
        let parent = UiRect {
            pos: Vec2::ZERO,
            size: Vec2::new(100.0, 100.0),
        };
        let size = Vec2::new(10.0, 10.0);
        let at = |a: Anchor| anchor_in(parent, size, a, Vec2::new(5.0, 5.0)).pos;
        assert_eq!(at(Anchor::TopLeft), Vec2::new(5.0, 5.0));
        assert_eq!(at(Anchor::TopRight), Vec2::new(85.0, 5.0));
        assert_eq!(at(Anchor::Center), Vec2::new(45.0, 45.0));
        assert_eq!(at(Anchor::BottomRight), Vec2::new(85.0, 85.0));
        assert_eq!(at(Anchor::BottomCenter), Vec2::new(45.0, 85.0));
        // Legacy helper delegates.
        assert_eq!(
            anchor_bottom(parent, size, 5.0).pos,
            at(Anchor::BottomCenter)
        );
    }

    #[test]
    fn column_stacks_y() {
        let mut parent = UiNode::new(NodeId(0), "root", WidgetKind::Panel);
        parent.layout = LayoutType::Column { wrap: false };
        parent.rect = UiRect {
            pos: Vec2::ZERO,
            size: Vec2::new(200.0, 400.0),
        };
        let mut a = UiNode::new(
            NodeId(1),
            "a",
            WidgetKind::Button {
                label: "OK".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
        );
        let mut b = UiNode::new(
            NodeId(2),
            "b",
            WidgetKind::Button {
                label: "Cancel".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
        );
        let mut kids = [&mut a, &mut b];
        layout_children(&parent, &mut kids, parent.rect);
        assert!(b.rect.pos.y > a.rect.pos.y);
    }
}
