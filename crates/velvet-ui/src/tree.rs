//! UI tree and hit testing.

use indexmap::IndexMap;
use velvet_math::Vec2;

use crate::focus::FocusState;
use crate::layout::{layout_children, LayoutType};
use crate::node::{NodeId, UiNode, UiRect};
use crate::sound::{UiSoundKind, UiSoundQueue};
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
    /// Directional input (focused Tabs/Dropdown consume these).
    pub up: bool,
    /// Directional input.
    pub down: bool,
    /// Directional input.
    pub left: bool,
    /// Directional input.
    pub right: bool,
    /// Frame delta seconds (drives hold-repeat and double-click clocks).
    pub dt: f32,
    /// Viewport size.
    pub viewport: Vec2,
}

/// Which input family drove the last frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputModality {
    /// Pointer buttons (mouse/touch press, release, wheel-clicks).
    #[default]
    Pointer,
    /// Keyboard (confirm, cancel, arrows).
    Keyboard,
}

/// Retained UI tree.
#[derive(Debug)]
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
    /// Arrows consumed by the focused widget this frame (hosts skip
    /// `move_focus` for them so Tabs/Dropdown keep arrow keys).
    pub arrows_consumed: bool,
    /// UI sound requests (hosts drain per frame into audio).
    pub sounds: UiSoundQueue,
    /// Hovered interactive node last frame (edge detection for hover ticks).
    last_hover: Option<NodeId>,
    /// Node double-clicked this frame (inside the double-click window).
    pub double_clicked: Option<NodeId>,
    /// Hold-to-repeat delay seconds before repeats start (buttons).
    pub repeat_delay: f32,
    /// Hold-to-repeat interval seconds between repeats.
    pub repeat_rate: f32,
    /// Double-click window seconds.
    pub double_click_secs: f32,
    /// UI clock seconds (advanced by `dt` every frame).
    ui_time: f32,
    /// Last input modality (pointer buttons vs keyboard keys).
    pub input_modality: InputModality,
    /// Held button under test for repeat: (node, next fire time).
    repeating: Option<(NodeId, f32)>,
    /// Last click (node, time) for double-click detection.
    last_click: Option<(NodeId, f32)>,
}

impl Default for UiTree {
    fn default() -> Self {
        Self {
            nodes: IndexMap::new(),
            root: None,
            next_id: 0,
            focus: FocusState::default(),
            theme: Theme::velvet_dark(),
            clicked: None,
            arrows_consumed: false,
            sounds: UiSoundQueue::new(),
            last_hover: None,
            double_clicked: None,
            input_modality: InputModality::Pointer,
            repeat_delay: 0.5,
            repeat_rate: 0.08,
            double_click_secs: 0.4,
            ui_time: 0.0,
            repeating: None,
            last_click: None,
        }
    }
}

impl UiTree {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
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

    /// Visit every node in insertion order.
    pub fn for_each_node(&self, visit: &mut dyn FnMut(&UiNode)) {
        for node in self.nodes.values() {
            visit(node);
        }
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

    /// Push a sound event tagged with a node's name (missing nodes tag "").
    fn push_sound(&mut self, kind: UiSoundKind, id: NodeId) {
        let name = self
            .nodes
            .get(&id.0)
            .map(|n| n.name.clone())
            .unwrap_or_default();
        self.sounds.push(kind, name);
    }

    /// Set a button's busy flag (spinner): blocks clicks silently.
    pub fn set_busy(&mut self, id: NodeId, busy: bool) {
        if let Some(n) = self.nodes.get_mut(&id.0) {
            if let WidgetKind::Button { busy: slot, .. } = &mut n.widget {
                *slot = busy;
            }
        }
    }

    /// Whether a button is busy.
    pub fn is_busy(&self, id: NodeId) -> bool {
        matches!(
            self.nodes.get(&id.0).map(|n| &n.widget),
            Some(WidgetKind::Button { busy: true, .. })
        )
    }

    /// Option row under `point` for an open dropdown, if any.
    ///
    /// Contract (shared with hosts rendering the popup): options stack
    /// vertically below the header, each row exactly one header tall.
    /// Returns None when closed, empty, or outside the option area.
    pub fn dropdown_option_at(&self, id: NodeId, point: Vec2) -> Option<usize> {
        let node = self.get(id)?;
        let (options, open) = match &node.widget {
            WidgetKind::Dropdown { options, open, .. } if *open && !options.is_empty() => {
                (options.len(), true)
            }
            _ => return None,
        };
        let _ = open;
        let row_h = node.rect.size.y;
        if row_h <= 0.0 {
            return None;
        }
        let rel_y = point.y - (node.rect.pos.y + row_h);
        if rel_y < 0.0 {
            return None;
        }
        let index = (rel_y / row_h).floor() as usize;
        (index < options).then_some(index)
    }

    /// Process pointer/button input; updates hover/press and clicked.
    pub fn process(&mut self, ctx: UiContext) {
        self.clicked = None;
        self.double_clicked = None;
        self.arrows_consumed = false;
        // Modality follows actual input (pointer motion alone is invisible
        // here — hosts with hover deltas can set this field directly).
        if ctx.pointer_pressed || ctx.pointer_released {
            self.input_modality = InputModality::Pointer;
        } else if ctx.confirm || ctx.cancel || ctx.up || ctx.down || ctx.left || ctx.right {
            self.input_modality = InputModality::Keyboard;
        }
        self.ui_time += ctx.dt.max(0.0);
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

        // Hover edge ticks (once per entry, enabled interactive only).
        let hover_hit = self.hit_test(ctx.pointer);
        let hover_id = hover_hit.and_then(|hit| {
            self.nodes.get(&hit.node.0).filter(|n| {
                n.enabled
                    && matches!(
                        n.widget,
                        WidgetKind::Button { .. }
                            | WidgetKind::Slider { .. }
                            | WidgetKind::Toggle { .. }
                            | WidgetKind::Dropdown { .. }
                            | WidgetKind::Tabs { .. }
                    )
            })
        });
        let hover_id = hover_id.map(|n| n.id);
        if hover_id != self.last_hover {
            if let Some(id) = hover_id {
                self.push_sound(UiSoundKind::Hover, id);
            }
            self.last_hover = hover_id;
        }

        let mut pending_click = None;
        // Open-dropdown option rows live outside any node rect, so they
        // resolve before hit testing (first open dropdown wins).
        if ctx.pointer_released {
            let mut picked = None;
            for (key, node) in self.nodes.iter() {
                if !node.enabled {
                    continue;
                }
                if let WidgetKind::Dropdown { open: true, .. } = &node.widget {
                    if let Some(i) = self.dropdown_option_at(NodeId(*key), ctx.pointer) {
                        picked = Some((NodeId(*key), i));
                        break;
                    }
                }
            }
            if let Some((id, i)) = picked {
                if let Some(n) = self.nodes.get_mut(&id.0) {
                    if let WidgetKind::Dropdown {
                        open,
                        selected,
                        highlight,
                        ..
                    } = &mut n.widget
                    {
                        *selected = Some(i);
                        *highlight = i;
                        *open = false;
                        pending_click = Some(id);
                    }
                }
                self.push_sound(UiSoundKind::Pick, id);
            }
        }
        // Hold-to-repeat arming: pressing an enabled, non-busy button
        // starts the clock; staying down over it refires (see below).
        if ctx.pointer_pressed {
            let pressed_button = self.hit_test(ctx.pointer).filter(|hit| {
                self.nodes.get(&hit.node.0).is_some_and(|n| {
                    n.enabled && matches!(n.widget, WidgetKind::Button { busy: false, .. })
                })
            });
            match pressed_button {
                Some(hit) => {
                    if self.repeating.map(|(id, _)| id) != Some(hit.node) {
                        self.repeating = Some((hit.node, self.ui_time + self.repeat_delay));
                    }
                }
                None => self.repeating = None,
            }
        }
        if ctx.pointer_released {
            self.repeating = None;
        }
        if let Some(hit) = self.hit_test(ctx.pointer) {
            let enabled = self
                .nodes
                .get(&hit.node.0)
                .map(|n| n.enabled)
                .unwrap_or(false);
            // Sounds resolve from inspection (no borrow held into handling).
            // Enabled arms first; busy is silent; the fallback below only
            // sees disabled hits.
            if ctx.pointer_released {
                match self.nodes.get(&hit.node.0).map(|n| &n.widget) {
                    Some(WidgetKind::Button { busy: true, .. }) => {}
                    Some(WidgetKind::Button { .. }) if enabled => {
                        self.push_sound(UiSoundKind::Click, hit.node);
                    }
                    Some(WidgetKind::Toggle { .. }) if enabled => {
                        self.push_sound(UiSoundKind::Toggle, hit.node);
                    }
                    Some(WidgetKind::Button { .. }) | Some(WidgetKind::Toggle { .. }) => {
                        self.push_sound(UiSoundKind::Blocked, hit.node);
                    }
                    _ => {}
                }
            }
            if let Some(n) = self.nodes.get_mut(&hit.node.0) {
                // Busy buttons track hover but never press or click.
                let busy = matches!(n.widget, WidgetKind::Button { busy: true, .. });
                match &mut n.widget {
                    WidgetKind::Button {
                        pressed, hovered, ..
                    } => {
                        *hovered = true;
                        if ctx.pointer_pressed && enabled && !busy {
                            *pressed = true;
                        }
                        if ctx.pointer_released && enabled && !busy {
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
                    WidgetKind::Dropdown {
                        open,
                        selected,
                        highlight,
                        options,
                    } if enabled && ctx.pointer_released => {
                        // Header click toggles (option rows resolve above).
                        *open = !*open;
                        if *open && !options.is_empty() {
                            *highlight = selected.unwrap_or(0).min(options.len() - 1);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Hold-to-repeat: pressing an enabled button arms the clock;
        // staying down over it refires click + sound on schedule.
        if ctx.pointer_pressed {
            let pressed_button = self.hit_test(ctx.pointer).filter(|hit| {
                self.nodes
                    .get(&hit.node.0)
                    .is_some_and(|n| n.enabled && matches!(n.widget, WidgetKind::Button { .. }))
            });
            match pressed_button {
                Some(hit) => {
                    if self.repeating.map(|(id, _)| id) != Some(hit.node) {
                        self.repeating = Some((hit.node, self.ui_time + self.repeat_delay));
                    }
                }
                None => self.repeating = None,
            }
        }
        if ctx.pointer_released {
            self.repeating = None;
        }
        if ctx.pointer_pressed {
            if let Some((id, next)) = self.repeating {
                let still_over = self.hit_test(ctx.pointer).map(|h| h.node) == Some(id);
                let armed = self.nodes.get(&id.0).is_some_and(|n| {
                    n.enabled && matches!(n.widget, WidgetKind::Button { busy: false, .. })
                });
                if still_over && armed && self.ui_time >= next {
                    pending_click = Some(id);
                    self.push_sound(UiSoundKind::Click, id);
                    self.repeating = Some((id, next + self.repeat_rate.max(1e-3)));
                }
            }
        }

        // Release outside an open dropdown (neither its header nor its
        // option rows) closes it. The root panel always hits, so "no hit
        // at all" is the wrong test — compare against the popup instead.
        if ctx.pointer_released {
            let hit_id = self.hit_test(ctx.pointer).map(|h| h.node);
            let mut close = Vec::new();
            for (key, node) in self.nodes.iter() {
                if let WidgetKind::Dropdown { open: true, .. } = &node.widget {
                    let id = NodeId(*key);
                    let on_header = hit_id == Some(id);
                    let on_option = self.dropdown_option_at(id, ctx.pointer).is_some();
                    if !on_header && !on_option {
                        close.push(id);
                    }
                }
            }
            for id in close {
                if let Some(n) = self.nodes.get_mut(&id.0) {
                    if let WidgetKind::Dropdown { open, .. } = &mut n.widget {
                        *open = false;
                    }
                }
            }
        }
        // Cancel closes open dropdowns.
        if ctx.cancel {
            for n in self.nodes.values_mut() {
                if let WidgetKind::Dropdown { open, .. } = &mut n.widget {
                    *open = false;
                }
            }
        }

        // Tooltips follow the hovered target by name.
        let hovered_name = self
            .hit_test(ctx.pointer)
            .and_then(|hit| self.nodes.get(&hit.node.0))
            .map(|n| n.name.clone());
        for n in self.nodes.values_mut() {
            if let WidgetKind::Tooltip { target, .. } = &n.widget {
                n.visible = hovered_name.as_deref() == Some(target.as_str());
            }
        }

        // Clicking a direct child of a Tabs node selects its tab.
        // The child keeps no click of its own (headers are plain nodes).
        if ctx.pointer_released {
            if let Some(hit) = self.hit_test(ctx.pointer) {
                let tabs_parent = self
                    .nodes
                    .get(&hit.node.0)
                    .and_then(|n| n.parent)
                    .filter(|p| {
                        matches!(
                            self.nodes.get(&p.0).map(|n| &n.widget),
                            Some(WidgetKind::Tabs { .. })
                        )
                    });
                if let Some(tabs_id) = tabs_parent {
                    let index = self
                        .nodes
                        .get(&tabs_id.0)
                        .map(|t| t.children.iter().position(|c| *c == hit.node).unwrap_or(0))
                        .unwrap_or(0);
                    if let Some(t) = self.nodes.get_mut(&tabs_id.0) {
                        t.widget.tabs_select(index);
                    }
                    pending_click = Some(tabs_id);
                    self.push_sound(UiSoundKind::Select, tabs_id);
                }
            }
        }

        if ctx.confirm {
            if let Some(f) = self.focus.focused {
                // Sound resolves before mutation (no borrow held). Busy
                // buttons stay silent (transient, not a mistake).
                let confirm_sound = match self.nodes.get(&f.0).map(|n| (&n.widget, n.enabled)) {
                    Some((WidgetKind::Button { busy: true, .. }, _)) => None,
                    Some((WidgetKind::Button { .. }, true)) => Some(UiSoundKind::Click),
                    Some((WidgetKind::Toggle { .. }, true)) => Some(UiSoundKind::Toggle),
                    Some((WidgetKind::Dropdown { open: true, .. }, true)) => {
                        Some(UiSoundKind::Pick)
                    }
                    Some((WidgetKind::Button { .. }, false))
                    | Some((WidgetKind::Toggle { .. }, false))
                    | Some((WidgetKind::Dropdown { .. }, false)) => Some(UiSoundKind::Blocked),
                    _ => None,
                };
                if let Some(n) = self.nodes.get_mut(&f.0) {
                    match &mut n.widget {
                        WidgetKind::Button { pressed, busy, .. } if n.enabled && !*busy => {
                            *pressed = true;
                            pending_click = Some(f);
                        }
                        WidgetKind::Toggle { value, .. } if n.enabled => {
                            *value = !*value;
                            pending_click = Some(f);
                        }
                        WidgetKind::Dropdown {
                            open,
                            selected,
                            highlight,
                            options,
                        } if n.enabled => {
                            if *open {
                                if !options.is_empty() {
                                    *selected = Some(*highlight);
                                    pending_click = Some(f);
                                }
                                *open = false;
                            } else {
                                *open = true;
                                if !options.is_empty() {
                                    *highlight = selected.unwrap_or(0).min(options.len() - 1);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(kind) = confirm_sound {
                    self.push_sound(kind, f);
                }
            }
        }

        // Arrow keys for the focused Tabs/Dropdown (hosts skip move_focus
        // when `arrows_consumed` is set so both do not fire).
        if let Some(f) = self.focus.focused {
            let arrows = (ctx.up, ctx.down, ctx.left, ctx.right);
            if arrows != (false, false, false, false) {
                // Sounds resolve before mutation (no borrow held).
                let arrow_sound = match self.nodes.get(&f.0).map(|n| &n.widget) {
                    Some(WidgetKind::Tabs { .. }) if ctx.left || ctx.right => {
                        Some(UiSoundKind::Select)
                    }
                    Some(WidgetKind::Dropdown { open: false, .. }) if ctx.up || ctx.down => {
                        Some(UiSoundKind::Pick)
                    }
                    _ => None,
                };
                if let Some(n) = self.nodes.get_mut(&f.0) {
                    match &mut n.widget {
                        WidgetKind::Tabs { tabs, selected } if !tabs.is_empty() => {
                            let len = tabs.len();
                            if ctx.left {
                                *selected = (*selected + len - 1) % len;
                                pending_click = Some(f);
                            }
                            if ctx.right {
                                *selected = (*selected + 1) % len;
                                pending_click = Some(f);
                            }
                            self.arrows_consumed = ctx.left || ctx.right;
                        }
                        WidgetKind::Dropdown {
                            open,
                            selected,
                            highlight,
                            options,
                        } if !options.is_empty() => {
                            let len = options.len();
                            if ctx.up {
                                *highlight = highlight.saturating_sub(1);
                                if !*open {
                                    *selected = Some(*highlight);
                                    pending_click = Some(f);
                                }
                            }
                            if ctx.down {
                                *highlight = (*highlight + 1).min(len - 1);
                                if !*open {
                                    *selected = Some(*highlight);
                                    pending_click = Some(f);
                                }
                            }
                            self.arrows_consumed = ctx.up || ctx.down;
                        }
                        _ => {}
                    }
                }
                if let Some(kind) = arrow_sound {
                    // Only when something actually changed/picked.
                    if pending_click == Some(f) {
                        self.push_sound(kind, f);
                    }
                }
            }
        }
        self.clicked = pending_click;
        // Double-click: two button clicks on the same node inside the window.
        if let Some(id) = pending_click {
            let is_button = self
                .nodes
                .get(&id.0)
                .is_some_and(|n| matches!(n.widget, WidgetKind::Button { .. }));
            if is_button {
                if let Some((last_id, last_t)) = self.last_click {
                    if last_id == id && self.ui_time - last_t <= self.double_click_secs {
                        self.double_clicked = Some(id);
                    }
                }
                self.last_click = Some((id, self.ui_time));
            }
        }
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
            n.layout = LayoutType::Column { wrap: false };
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
    use crate::widget::Button;

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
                busy: false,
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

    fn click_at(ui: &mut UiTree, point: Vec2) {
        ui.process(UiContext {
            pointer: point,
            pointer_pressed: false,
            pointer_released: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
    }

    fn node_mid(ui: &UiTree, id: NodeId) -> Vec2 {
        let rect = ui.get(id).unwrap().rect;
        rect.pos + rect.size * 0.5
    }

    #[test]
    fn dropdown_opens_picks_option_and_closes_away() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let dd = ui.add_node(
            "lang",
            WidgetKind::Dropdown {
                options: vec!["es".into(), "en".into(), "ja".into()],
                selected: None,
                highlight: 0,
                open: false,
            },
            Some(root),
        );
        ui.layout(Vec2::new(800.0, 600.0));
        // Open via header click (no selection event yet).
        let mid = node_mid(&ui, dd);
        click_at(&mut ui, mid);
        assert!(matches!(
            ui.get(dd).unwrap().widget,
            WidgetKind::Dropdown { open: true, .. }
        ));
        assert_eq!(ui.clicked, None);
        // Second row (index 1) below the header picks + closes + clicks.
        let rect = ui.get(dd).unwrap().rect;
        click_at(
            &mut ui,
            Vec2::new(rect.pos.x + 4.0, rect.pos.y + rect.size.y * 2.5),
        );
        match &ui.get(dd).unwrap().widget {
            WidgetKind::Dropdown {
                selected,
                open,
                highlight,
                ..
            } => {
                assert_eq!(*selected, Some(1));
                assert_eq!(*highlight, 1);
                assert!(!open);
            }
            _ => panic!("dropdown lost"),
        }
        assert_eq!(ui.clicked, Some(dd));
        // Reopen, then click away closes without changing selection.
        let mid = node_mid(&ui, dd);
        click_at(&mut ui, mid);
        assert!(matches!(
            ui.get(dd).unwrap().widget,
            WidgetKind::Dropdown { open: true, .. }
        ));
        click_at(&mut ui, Vec2::new(790.0, 590.0));
        assert!(matches!(
            ui.get(dd).unwrap().widget,
            WidgetKind::Dropdown { open: false, .. }
        ));
        assert_eq!(
            ui.get(dd).unwrap().widget.dropdown_selected(),
            Some(Some(1))
        );
    }

    #[test]
    fn dropdown_keyboard_cycles_and_confirms() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let dd = ui.add_node(
            "lang",
            WidgetKind::Dropdown {
                options: vec!["es".into(), "en".into()],
                selected: None,
                highlight: 0,
                open: false,
            },
            Some(root),
        );
        ui.layout(Vec2::new(800.0, 600.0));
        ui.focus.focused = Some(dd);
        // Closed arrows move selection directly and consume the keys.
        ui.process(UiContext {
            down: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(ui.arrows_consumed);
        assert_eq!(
            ui.get(dd).unwrap().widget.dropdown_selected(),
            Some(Some(1))
        );
        // Confirm opens; arrows move highlight only while open.
        ui.process(UiContext {
            confirm: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(matches!(
            ui.get(dd).unwrap().widget,
            WidgetKind::Dropdown { open: true, .. }
        ));
        // Cancel closes.
        ui.process(UiContext {
            cancel: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(matches!(
            ui.get(dd).unwrap().widget,
            WidgetKind::Dropdown { open: false, .. }
        ));
    }

    #[test]
    fn tabs_select_by_child_click_and_arrows() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let tabs = ui.add_node(
            "modes",
            WidgetKind::Tabs {
                tabs: vec!["a".into(), "b".into(), "c".into()],
                selected: 0,
            },
            Some(root),
        );
        // Headers as plain children map by position.
        let h0 = ui.add_node("h0", WidgetKind::Label { text: "a".into() }, Some(tabs));
        let h1 = ui.add_node("h1", WidgetKind::Label { text: "b".into() }, Some(tabs));
        let h2 = ui.add_node("h2", WidgetKind::Label { text: "c".into() }, Some(tabs));
        ui.layout(Vec2::new(800.0, 600.0));
        let mid = node_mid(&ui, h2);
        click_at(&mut ui, mid);
        assert_eq!(ui.get(tabs).unwrap().widget.tabs_selected(), Some(2));
        assert_eq!(ui.clicked, Some(tabs));
        // Arrows cycle with wrap and consume the keys.
        ui.focus.focused = Some(tabs);
        ui.process(UiContext {
            right: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(ui.arrows_consumed);
        assert_eq!(ui.get(tabs).unwrap().widget.tabs_selected(), Some(0));
        ui.process(UiContext {
            left: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert_eq!(ui.get(tabs).unwrap().widget.tabs_selected(), Some(2));
        let _ = (h0, h1);
    }

    #[test]
    fn tooltip_follows_hover_target() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let btn = ui.add_node("save", Button::widget("Save"), Some(root));
        let tip = ui.add_node(
            "tip",
            WidgetKind::Tooltip {
                text: "Write the save".into(),
                target: "save".into(),
            },
            Some(root),
        );
        ui.layout(Vec2::new(800.0, 600.0));
        assert!(!ui.get(tip).unwrap().visible);
        // Hover the button (no click): tooltip shows.
        let mid = node_mid(&ui, btn);
        ui.process(UiContext {
            pointer: mid,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(ui.get(tip).unwrap().visible);
        // Move away: hides again and never joins focus order.
        ui.process(UiContext {
            pointer: Vec2::new(790.0, 590.0),
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(!ui.get(tip).unwrap().visible);
        ui.rebuild_focus_order();
        assert!(!ui.focus.order.contains(&tip));
    }

    fn sounds(ui: &mut UiTree) -> Vec<(String, String)> {
        ui.sounds
            .drain()
            .into_iter()
            .map(|s| (s.kind.as_str().to_string(), s.node))
            .collect()
    }

    fn column_root(ui: &mut UiTree) -> NodeId {
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().layout = LayoutType::Column { wrap: false };
        root
    }

    #[test]
    fn sound_events_fire_once_per_gesture() {
        let mut ui = UiTree::with_root("root");
        let root = column_root(&mut ui);
        let btn = ui.add_node("ok", Button::widget("OK"), Some(root));
        let dis = ui.add_node(
            "off",
            WidgetKind::Button {
                label: "Off".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
            Some(root),
        );
        ui.get_mut(dis).unwrap().enabled = false;
        ui.layout(Vec2::new(800.0, 600.0));
        // Hover edge ticks once, not per frame.
        let mid = node_mid(&ui, btn);
        ui.process(UiContext {
            pointer: mid,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        eprintln!(
            "DEBUG hit={:?} last_hover={:?}",
            ui.hit_test(mid),
            ui.last_hover
        );
        assert_eq!(
            sounds(&mut ui),
            vec![("hover".to_string(), "ok".to_string())]
        );
        ui.process(UiContext {
            pointer: mid,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert!(sounds(&mut ui).is_empty());
        // Click sounds; disabled release sounds blocked.
        click_at(&mut ui, mid);
        assert_eq!(
            sounds(&mut ui),
            vec![("click".to_string(), "ok".to_string())]
        );
        let mid_off = node_mid(&ui, dis);
        click_at(&mut ui, mid_off);
        assert_eq!(
            sounds(&mut ui),
            vec![("blocked".to_string(), "off".to_string())]
        );
        // Confirm on focused button clicks too.
        ui.focus.focused = Some(btn);
        ui.process(UiContext {
            confirm: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert_eq!(
            sounds(&mut ui),
            vec![("click".to_string(), "ok".to_string())]
        );
    }

    fn hold_process(ui: &mut UiTree, point: Vec2, dt: f32) {
        ui.process(UiContext {
            pointer: point,
            pointer_pressed: true,
            dt,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
    }

    #[test]
    fn hold_repeats_and_double_click_detects() {
        let mut ui = UiTree::with_root("root");
        let root = column_root(&mut ui);
        let btn = ui.add_node("vol", Button::widget("+"), Some(root));
        ui.layout(Vec2::new(800.0, 600.0));
        let mid = node_mid(&ui, btn);
        // Press and hold 1s in 0.1 steps: first fire at 0.5, then every 0.08.
        ui.repeat_delay = 0.5;
        ui.repeat_rate = 0.2;
        let mut fires = 0;
        for _ in 0..10 {
            hold_process(&mut ui, mid, 0.1);
            if ui.clicked == Some(btn) {
                fires += 1;
            }
        }
        // t=0.1..1.0: fires at 0.5, 0.7, 0.9 → 3 repeats.
        assert_eq!(fires, 3, "repeat schedule");
        // Release stops the repeats.
        click_at(&mut ui, mid);
        hold_process(&mut ui, mid, 0.5);
        // Two quick clicks → double-click on the second.
        click_at(&mut ui, mid);
        assert_eq!(ui.double_clicked, None);
        // Advance a little (inside the window) and click again.
        ui.process(UiContext {
            dt: 0.1,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        click_at(&mut ui, mid);
        assert_eq!(ui.double_clicked, Some(btn));
        // Outside the window: no double.
        ui.process(UiContext {
            dt: 1.0,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        click_at(&mut ui, mid);
        assert_eq!(ui.double_clicked, None);
    }

    #[test]
    fn busy_button_blocks_clicks_silently() {
        let mut ui = UiTree::with_root("root");
        let root = column_root(&mut ui);
        let btn = ui.add_node("go", Button::widget("Go"), Some(root));
        ui.layout(Vec2::new(800.0, 600.0));
        ui.set_busy(btn, true);
        assert!(ui.is_busy(btn));
        let mid = node_mid(&ui, btn);
        // Settle hover first (busy buttons still hover, silently).
        ui.process(UiContext {
            pointer: mid,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        assert_eq!(sounds(&mut ui).len(), 1);
        click_at(&mut ui, mid);
        assert_eq!(ui.clicked, None);
        assert!(sounds(&mut ui).is_empty());
        ui.set_busy(btn, false);
        assert!(!ui.is_busy(btn));
        click_at(&mut ui, mid);
        assert_eq!(ui.clicked, Some(btn));
    }

    #[test]
    fn focus_visible_only_after_keyboard() {
        let mut ui = UiTree::with_root("root");
        let root = column_root(&mut ui);
        let btn = ui.add_node("go", Button::widget("Go"), Some(root));
        ui.layout(Vec2::new(800.0, 600.0));
        ui.focus.focused = Some(btn);
        // Pointer click: focus without the visible ring.
        let mid = node_mid(&ui, btn);
        click_at(&mut ui, mid);
        let query = ui.vcss_query(btn).unwrap();
        assert!(query.states.contains(&"focus".to_string()));
        assert!(!query.states.contains(&"focus-visible".to_string()));
        // Keyboard confirm: ring appears.
        ui.process(UiContext {
            confirm: true,
            viewport: Vec2::new(800.0, 600.0),
            ..Default::default()
        });
        let query = ui.vcss_query(btn).unwrap();
        assert!(query.states.contains(&"focus-visible".to_string()));
        assert_eq!(ui.input_modality, InputModality::Keyboard);
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
                busy: false,
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
