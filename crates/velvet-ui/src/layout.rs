//! Layout engines: stack, row, column, grid.

use velvet_math::Vec2;

use crate::node::{NodeId, UiNode, UiRect};

/// Layout algorithm for children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum LayoutType {
    /// Overlay stack (same origin + padding).
    #[default]
    Stack,
    /// Horizontal row.
    Row,
    /// Vertical column.
    Column,
    /// Grid with columns.
    Grid {
        /// Column count.
        columns: u32,
    },
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

    match parent.layout {
        LayoutType::Stack => {
            for c in children.iter_mut() {
                let m = c.style.margin;
                c.rect = UiRect {
                    pos: Vec2::new(content.pos.x + m.0, content.pos.y + m.1),
                    size: measure_child(c, content.size),
                };
            }
        }
        LayoutType::Row => {
            let mut x = content.pos.x;
            let mut max_h = 0.0f32;
            let gap = 8.0;
            for c in children.iter_mut() {
                let m = c.style.margin;
                let size = measure_child(c, content.size);
                c.rect = UiRect {
                    pos: Vec2::new(x + m.0, content.pos.y + m.1),
                    size,
                };
                x += size.x + m.0 + m.2 + gap;
                max_h = max_h.max(size.y + m.1 + m.3);
            }
            let _ = max_h;
        }
        LayoutType::Column => {
            let mut y = content.pos.y;
            let gap = 8.0;
            for c in children.iter_mut() {
                let m = c.style.margin;
                let size = measure_child(c, content.size);
                c.rect = UiRect {
                    pos: Vec2::new(content.pos.x + m.0, y + m.1),
                    size,
                };
                y += size.y + m.1 + m.3 + gap;
            }
        }
        LayoutType::Grid { columns } => {
            let cols = columns.max(1) as f32;
            let gap = 8.0;
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
            }
        }
    }

    LayoutResult {
        size: parent_rect.size,
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

/// Anchor helper: place rect at bottom center of parent.
pub fn anchor_bottom(parent: UiRect, child_size: Vec2, margin_bottom: f32) -> UiRect {
    UiRect {
        pos: Vec2::new(
            parent.pos.x + (parent.size.x - child_size.x) * 0.5,
            parent.pos.y + parent.size.y - child_size.y - margin_bottom,
        ),
        size: child_size,
    }
}

/// Silence unused NodeId in layout-only unit.
#[allow(dead_code)]
fn _id(_: NodeId) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::WidgetKind;

    #[test]
    fn column_stacks_y() {
        let mut parent = UiNode::new(NodeId(0), "root", WidgetKind::Panel);
        parent.layout = LayoutType::Column;
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
            },
        );
        let mut b = UiNode::new(
            NodeId(2),
            "b",
            WidgetKind::Button {
                label: "Cancel".into(),
                pressed: false,
                hovered: false,
            },
        );
        let mut kids = [&mut a, &mut b];
        layout_children(&parent, &mut kids, parent.rect);
        assert!(b.rect.pos.y > a.rect.pos.y);
    }
}
