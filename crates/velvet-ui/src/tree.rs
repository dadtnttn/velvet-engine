//! UI tree and hit testing.

use indexmap::IndexMap;
use velvet_math::Vec2;

use crate::focus::FocusState;
use crate::layout::{layout_children, LayoutType};
use crate::node::{NodeId, UiNode, UiRect};
use crate::theme::Theme;
use crate::widget::WidgetKind;

/// Hit test result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitResult {
    /// Node.
    pub node: NodeId,
    /// Name.
    pub name: String,
}

/// Frame input for UI.
#[derive(Debug, Clone, Copy, Default)]
pub struct UiContext {
    /// Pointer position.
    pub pointer: Vec2,
    /// Pointer pressed.
    pub pointer_pressed: bool,
    /// Pointer released.
    pub pointer_released: bool,
    /// Confirm action.
    pub confirm: bool,
    /// Cancel action.
    pub cancel: bool,
    /// Viewport size.
    pub viewport: Vec2,
}

/// Retained UI tree.
#[derive(Debug, Default)]
pub struct UiTree {
    nodes: IndexMap<u64, UiNode>,
    root: Option<NodeId>,
    next_id: u64,
    /// Focus.
    pub focus: FocusState,
    /// Theme.
    pub theme: Theme,
    /// Clicked node this frame.
    pub clicked: Option<NodeId>,
}

impl UiTree {
    /// Create empty.
    pub fn new() -> Self {
        Self {
            theme: Theme::velvet_dark(),
            ..Default::default()
        }
    }

    /// Create root panel.
    pub fn with_root(name: impl Into<String>) -> Self {
        let mut t = Self::new();
        let id = t.add_node(name, WidgetKind::Panel, None);
        t.root = Some(id);
        t
    }

    /// Add node under parent.
    pub fn add_node(
        &mut self,
        name: impl Into<String>,
        widget: WidgetKind,
        parent: Option<NodeId>,
    ) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        let mut node = UiNode::new(id, name, widget);
        node.parent = parent;
        if let Some(p) = parent {
            if let Some(pn) = self.nodes.get_mut(&p.0) {
                pn.children.push(id);
            }
        }
        self.nodes.insert(id.0, node);
        id
    }

    /// Get node.
    pub fn get(&self, id: NodeId) -> Option<&UiNode> {
        self.nodes.get(&id.0)
    }

    /// Get mut.
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut UiNode> {
        self.nodes.get_mut(&id.0)
    }

    /// Root id.
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    /// Rebuild focus order.
    pub fn rebuild_focus_order(&mut self) {
        let mut order = Vec::new();
        if let Some(r) = self.root {
            self.collect_focusable(r, &mut order);
        }
        self.focus.set_order(order);
    }

    fn collect_focusable(&self, id: NodeId, out: &mut Vec<NodeId>) {
        if let Some(n) = self.get(id) {
            if n.focusable && n.visible && n.enabled {
                out.push(id);
            }
            for c in &n.children {
                self.collect_focusable(*c, out);
            }
        }
    }

    /// Layout full tree into viewport.
    pub fn layout(&mut self, viewport: Vec2) {
        let Some(root) = self.root else {
            return;
        };
        if let Some(r) = self.nodes.get_mut(&root.0) {
            r.rect = UiRect {
                pos: Vec2::ZERO,
                size: viewport,
            };
        }
        self.layout_recursive(root);
    }

    fn layout_recursive(&mut self, id: NodeId) {
        let (layout, rect, child_ids) = {
            let n = match self.nodes.get(&id.0) {
                Some(n) => n,
                None => return,
            };
            (n.layout, n.rect, n.children.clone())
        };
        if child_ids.is_empty() {
            return;
        }
        // Unsafe split: clone children data, layout, write back
        let mut child_nodes: Vec<UiNode> = child_ids
            .iter()
            .filter_map(|c| self.nodes.get(&c.0).cloned())
            .collect();
        let parent = self.nodes.get(&id.0).unwrap().clone();
        let mut refs: Vec<&mut UiNode> = child_nodes.iter_mut().collect();
        let _ = layout;
        layout_children(&parent, &mut refs, rect);
        for c in child_nodes {
            let cid = c.id;
            if let Some(slot) = self.nodes.get_mut(&cid.0) {
                slot.rect = c.rect;
            }
            self.layout_recursive(cid);
        }
    }

    /// Hit test front-most node (children take priority over parents).
    pub fn hit_test(&self, point: Vec2) -> Option<HitResult> {
        if let Some(r) = self.root {
            return self.hit_test_node(r, point);
        }
        None
    }

    fn hit_test_node(&self, id: NodeId, point: Vec2) -> Option<HitResult> {
        let n = self.get(id)?;
        if !n.visible {
            return None;
        }
        // Front-to-back: later siblings are drawn on top.
        for c in n.children.iter().rev() {
            if let Some(hit) = self.hit_test_node(*c, point) {
                return Some(hit);
            }
        }
        if n.rect.contains(point) {
            Some(HitResult {
                node: id,
                name: n.name.clone(),
            })
        } else {
            None
        }
    }

    /// Process pointer/button input; updates hover/press and clicked.
    pub fn process(&mut self, ctx: UiContext) {
        self.clicked = None;
        self.layout(ctx.viewport);
        self.rebuild_focus_order();

        // Clear press flags
        for n in self.nodes.values_mut() {
            if let WidgetKind::Button {
                pressed, hovered, ..
            } = &mut n.widget
            {
                *pressed = false;
                *hovered = false;
            }
        }

        let mut pending_click = None;
        if let Some(hit) = self.hit_test(ctx.pointer) {
            let enabled = self
                .nodes
                .get(&hit.node.0)
                .map(|n| n.enabled)
                .unwrap_or(false);
            if let Some(n) = self.nodes.get_mut(&hit.node.0) {
                match &mut n.widget {
                    WidgetKind::Button {
                        pressed, hovered, ..
                    } => {
                        *hovered = true;
                        if ctx.pointer_pressed && enabled {
                            *pressed = true;
                        }
                        if ctx.pointer_released && enabled {
                            pending_click = Some(hit.node);
                        }
                    }
                    WidgetKind::Toggle { value, .. } if ctx.pointer_released && enabled => {
                        *value = !*value;
                        pending_click = Some(hit.node);
                    }
                    WidgetKind::Slider {
                        value, dragging, ..
                    } if enabled => {
                        if ctx.pointer_pressed || *dragging {
                            *dragging = ctx.pointer_pressed || !ctx.pointer_released;
                            let rect = n.rect;
                            if rect.size.x > 0.0 {
                                *value =
                                    ((ctx.pointer.x - rect.pos.x) / rect.size.x).clamp(0.0, 1.0);
                            }
                        }
                        if ctx.pointer_released {
                            *dragging = false;
                        }
                    }
                    _ => {}
                }
            }
        }

        if ctx.confirm {
            if let Some(f) = self.focus.focused {
                if let Some(n) = self.nodes.get_mut(&f.0) {
                    match &mut n.widget {
                        WidgetKind::Button { pressed, .. } => {
                            *pressed = true;
                            pending_click = Some(f);
                        }
                        WidgetKind::Toggle { value, .. } => {
                            *value = !*value;
                            pending_click = Some(f);
                        }
                        _ => {}
                    }
                }
            }
        }
        self.clicked = pending_click;
    }

    /// Build a dialogue box tree (helper).
    pub fn build_dialogue_box(&mut self, speaker: &str, body: &str) -> NodeId {
        let root = self.root.unwrap_or_else(|| {
            let id = self.add_node("root", WidgetKind::Panel, None);
            self.root = Some(id);
            id
        });
        let panel = self.add_node("dialogue_panel", WidgetKind::Panel, Some(root));
        let panel_style = self.theme.panel.clone();
        if let Some(n) = self.get_mut(panel) {
            n.style = panel_style;
            n.layout = LayoutType::Column;
        }
        let sp = self.add_node(
            "speaker",
            WidgetKind::Label {
                text: speaker.into(),
            },
            Some(panel),
        );
        let _ = self.add_node("body", WidgetKind::Label { text: body.into() }, Some(panel));
        let _ = sp;
        panel
    }

    /// Node count.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialogue_and_click() {
        let mut ui = UiTree::with_root("root");
        let panel = ui.build_dialogue_box("Aria", "Hello");
        assert!(ui.len() >= 3);
        ui.layout(Vec2::new(800.0, 600.0));
        let btn = ui.add_node(
            "ok",
            WidgetKind::Button {
                label: "OK".into(),
                pressed: false,
                hovered: false,
            },
            Some(panel),
        );
        ui.layout(Vec2::new(800.0, 600.0));
        let rect = ui.get(btn).unwrap().rect;
        assert!(rect.size.x > 0.0 && rect.size.y > 0.0, "button needs size");
        let mid = rect.pos + rect.size * 0.5;
        // Ensure hit resolves to the button
        let hit = ui.hit_test(mid);
        assert_eq!(hit.map(|h| h.node), Some(btn));
        let ctx = UiContext {
            pointer: mid,
            pointer_pressed: true,
            pointer_released: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        };
        ui.process(ctx);
        assert_eq!(ui.clicked, Some(btn));
    }

    #[test]
    fn confirm_activates_focused_button() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let btn = ui.add_node(
            "go",
            WidgetKind::Button {
                label: "Go".into(),
                pressed: false,
                hovered: false,
            },
            Some(root),
        );
        ui.layout(Vec2::new(400.0, 300.0));
        ui.focus.focused = Some(btn);
        let ctx = UiContext {
            confirm: true,
            viewport: Vec2::new(400.0, 300.0),
            ..Default::default()
        };
        ui.process(ctx);
        assert_eq!(ui.clicked, Some(btn));
    }
}
