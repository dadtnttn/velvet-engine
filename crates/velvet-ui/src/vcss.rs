//! Bridge between retained UI nodes and VCSS stylesheets.

use velvet_math::Color;
use velvet_style::{resolve, ComputedStyle, StyleQuery, StyleScope, Stylesheet};

use crate::{NodeId, UiNode, UiStyle, UiTree, WidgetKind};

/// Build the VCSS query for one node, including its ancestor context and live
/// pseudo states.
///
/// Widget element names are `panel`, `label`, `button`, `image`,
/// `progress-bar`, `slider`, `toggle`, and `text-field`. A node's [`UiNode::name`]
/// is its VCSS id, while [`UiNode::classes`] supplies its classes.
pub fn style_query_for_node(tree: &UiTree, id: NodeId) -> Option<StyleQuery> {
    let node = tree.get(id)?;
    let mut ancestors = Vec::new();
    let mut visited = Vec::new();
    let mut parent = node.parent;

    while let Some(parent_id) = parent {
        if visited.contains(&parent_id) {
            break;
        }
        visited.push(parent_id);
        let Some(ancestor) = tree.get(parent_id) else {
            break;
        };
        ancestors.push(scope_for_node(tree, ancestor));
        parent = ancestor.parent;
    }
    ancestors.reverse();

    Some(StyleQuery {
        element: Some(element_name(&node.widget).into()),
        id: (!node.name.is_empty()).then(|| node.name.clone()),
        classes: node.classes.clone(),
        states: states_for_node(tree, node),
        ancestors,
    })
}

/// Merge supported VCSS properties into a base [`UiStyle`].
///
/// Properties absent from `computed` retain their base values. `width` and
/// `height` become fixed dimensions by setting both the corresponding minimum
/// and maximum; explicit `min-*` and `max-*` declarations then refine them.
pub fn apply_computed_style(base: &UiStyle, computed: &ComputedStyle) -> UiStyle {
    let mut style = base.clone();

    if let Some(color) = color(computed, "background") {
        style.background = color;
    }
    if let Some(color) = color(computed, "color") {
        style.text_color = color;
    }
    if let Some(color) = color(computed, "border-color") {
        style.border = color;
    }
    if let Some(value) = number(computed, "border-width") {
        style.border_width = value.max(0.0);
    }
    if let Some(value) = number(computed, "border-radius") {
        style.radius = value.max(0.0);
    }
    if let Some(value) = number(computed, "opacity") {
        style.opacity = value.clamp(0.0, 1.0);
    }
    if let Some(value) = number(computed, "font-size") {
        style.font_size = value.max(0.0);
    }
    if let Some(value) = number(computed, "flex-grow") {
        style.flex_grow = value.max(0.0);
    }
    if let Some(value) = number(computed, "width") {
        let value = value.max(0.0);
        style.min_size.0 = value;
        style.max_size.0 = value;
    }
    if let Some(value) = number(computed, "height") {
        let value = value.max(0.0);
        style.min_size.1 = value;
        style.max_size.1 = value;
    }
    if let Some(value) = number(computed, "min-width") {
        style.min_size.0 = value.max(0.0);
    }
    if let Some(value) = number(computed, "min-height") {
        style.min_size.1 = value.max(0.0);
    }
    if let Some(value) = number(computed, "max-width") {
        style.max_size.0 = value.max(0.0);
    }
    if let Some(value) = number(computed, "max-height") {
        style.max_size.1 = value.max(0.0);
    }

    style.padding = box_values(computed, "padding", style.padding, true);
    style.margin = box_values(computed, "margin", style.margin, false);
    if let Some(value) = number(computed, "gap") {
        style.gap = value.max(0.0);
    }

    style
}

/// Resolve one node against a stylesheet without mutating the retained tree.
///
/// This is deliberately a per-node adapter: pseudo states change every frame,
/// so writing resolved values back across the whole tree would make transient
/// overrides sticky. Keep the node's retained style as the stable base and use
/// this returned value for painting. If layout must consume VCSS dimensions,
/// preserve or restore that base before resolving the next interaction state.
pub fn resolve_node_style(tree: &UiTree, id: NodeId, sheet: &Stylesheet) -> Option<UiStyle> {
    let node = tree.get(id)?;
    let query = style_query_for_node(tree, id)?;
    Some(apply_computed_style(&node.style, &resolve(sheet, &query)))
}

impl UiTree {
    /// Build the live VCSS selector query for one retained node.
    pub fn vcss_query(&self, id: NodeId) -> Option<StyleQuery> {
        style_query_for_node(self, id)
    }

    /// Resolve one retained node's VCSS style without mutating the tree.
    ///
    /// Recompute whenever hover, active, focus, or disabled state changes. Keep
    /// the retained node style as the base unless layout needs the result; if it
    /// does, preserve that base so transient overrides cannot become sticky.
    pub fn vcss_style(&self, id: NodeId, sheet: &Stylesheet) -> Option<UiStyle> {
        resolve_node_style(self, id, sheet)
    }
}

fn element_name(widget: &WidgetKind) -> &'static str {
    match widget {
        WidgetKind::Panel => "panel",
        WidgetKind::Label { .. } => "label",
        WidgetKind::Button { .. } => "button",
        WidgetKind::Image { .. } => "image",
        WidgetKind::ProgressBar { .. } => "progress-bar",
        WidgetKind::Slider { .. } => "slider",
        WidgetKind::Toggle { .. } => "toggle",
        WidgetKind::TextField { .. } => "text-field",
    }
}

fn scope_for_node(tree: &UiTree, node: &UiNode) -> StyleScope {
    StyleScope {
        element: Some(element_name(&node.widget).into()),
        id: (!node.name.is_empty()).then(|| node.name.clone()),
        classes: node.classes.clone(),
        states: states_for_node(tree, node),
    }
}

fn states_for_node(tree: &UiTree, node: &UiNode) -> Vec<String> {
    let mut states = Vec::new();
    if !node.enabled {
        states.push("disabled".into());
    }
    if tree.focus.focused == Some(node.id)
        || matches!(node.widget, WidgetKind::TextField { focused: true, .. })
    {
        states.push("focus".into());
    }
    if matches!(node.widget, WidgetKind::Button { hovered: true, .. }) {
        states.push("hover".into());
    }
    if matches!(
        node.widget,
        WidgetKind::Button { pressed: true, .. } | WidgetKind::Slider { dragging: true, .. }
    ) {
        states.push("active".into());
    }
    states
}

fn color(computed: &ComputedStyle, property: &str) -> Option<Color> {
    computed
        .props
        .get(property)
        .and_then(|value| value.as_color())
        .map(|color| {
            Color::rgba(
                color.r as f32 / 255.0,
                color.g as f32 / 255.0,
                color.b as f32 / 255.0,
                color.a,
            )
        })
}

fn number(computed: &ComputedStyle, property: &str) -> Option<f32> {
    computed
        .props
        .get(property)
        .and_then(|value| value.as_f32())
}

fn box_values(
    computed: &ComputedStyle,
    property: &str,
    current: (f32, f32, f32, f32),
    clamp: bool,
) -> (f32, f32, f32, f32) {
    let normalize = |value: f32| if clamp { value.max(0.0) } else { value };
    let mut values = current;

    if let Some(value) = number(computed, property).map(normalize) {
        values = (value, value, value, value);
    }
    if let Some(value) = number(computed, &format!("{property}-x")).map(normalize) {
        values.0 = value;
        values.2 = value;
    }
    if let Some(value) = number(computed, &format!("{property}-y")).map(normalize) {
        values.1 = value;
        values.3 = value;
    }
    for (side, slot) in [
        ("top", &mut values.1),
        ("right", &mut values.2),
        ("bottom", &mut values.3),
        ("left", &mut values.0),
    ] {
        if let Some(value) = number(computed, &format!("{property}-{side}")).map(normalize) {
            *slot = value;
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutType;
    use velvet_math::Vec2;
    use velvet_style::parse_stylesheet;

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
    }

    #[test]
    fn query_matches_element_id_class_context_and_live_states() {
        let mut ui = UiTree::with_root("main");
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().add_class("menu");
        let group = ui.add_node("actions", WidgetKind::Panel, Some(root));
        ui.get_mut(group).unwrap().add_class("group");
        let button = ui.add_node(
            "start",
            WidgetKind::Button {
                label: "Start".into(),
                pressed: false,
                hovered: true,
            },
            Some(group),
        );
        ui.get_mut(button).unwrap().add_class("action");
        ui.focus.focused = Some(button);

        let sheet = parse_stylesheet(
            r#"
            panel.menu .group button#start.action { background: #123456; }
            button.action:hover { color: #ff0000; }
            button:focus { border-color: #00ff00; }
            button:active { opacity: 0.6; }
            button:disabled { border-width: 3; }
            "#,
        )
        .unwrap();

        let query = ui.vcss_query(button).unwrap();
        assert_eq!(query.element.as_deref(), Some("button"));
        assert_eq!(query.id.as_deref(), Some("start"));
        assert_eq!(query.classes, ["action"]);
        assert_eq!(query.ancestors.len(), 2);
        assert_eq!(query.ancestors[0].classes, ["menu"]);
        assert_eq!(query.ancestors[1].classes, ["group"]);
        assert!(query.states.iter().any(|state| state == "hover"));
        assert!(query.states.iter().any(|state| state == "focus"));

        let style = ui.vcss_style(button, &sheet).unwrap();
        close(style.background.r, 0x12 as f32 / 255.0);
        close(style.text_color.r, 1.0);
        close(style.border.g, 1.0);

        let node = ui.get_mut(button).unwrap();
        node.enabled = false;
        if let WidgetKind::Button { pressed, .. } = &mut node.widget {
            *pressed = true;
        }
        let style = ui.vcss_style(button, &sheet).unwrap();
        close(style.opacity, 0.6);
        close(style.border_width, 3.0);
    }

    #[test]
    fn computed_properties_map_onto_ui_style() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().add_class("card");
        let sheet = parse_stylesheet(
            r#"
            .card {
                background: #10203080;
                color: #abcdef;
                border-color: #fedcba;
                border-width: 2;
                border-radius: 9;
                opacity: 0.7;
                font-size: 22;
                flex-grow: 1;
                width: 100;
                height: 40;
                padding: 4;
                padding-x: 8;
                padding-top: 6;
                margin-y: 5;
                gap: 12;
            }
            "#,
        )
        .unwrap();

        let style = resolve_node_style(&ui, root, &sheet).unwrap();
        close(style.background.r, 0x10 as f32 / 255.0);
        close(style.background.a, 0x80 as f32 / 255.0);
        close(style.text_color.g, 0xcd as f32 / 255.0);
        close(style.border.b, 0xba as f32 / 255.0);
        close(style.border_width, 2.0);
        close(style.radius, 9.0);
        close(style.opacity, 0.7);
        close(style.font_size, 22.0);
        close(style.flex_grow, 1.0);
        assert_eq!(style.min_size, (100.0, 40.0));
        assert_eq!(style.max_size, (100.0, 40.0));
        assert_eq!(style.padding, (8.0, 6.0, 8.0, 4.0));
        assert_eq!(style.margin, (0.0, 5.0, 0.0, 5.0));
        close(style.gap, 12.0);
    }

    #[test]
    fn vcss_gap_changes_column_layout() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        {
            let root_node = ui.get_mut(root).unwrap();
            root_node.add_class("menu");
            root_node.layout = LayoutType::Column;
        }
        let first = ui.add_node(
            "first",
            WidgetKind::Button {
                label: "A".into(),
                pressed: false,
                hovered: false,
            },
            Some(root),
        );
        let second = ui.add_node(
            "second",
            WidgetKind::Button {
                label: "B".into(),
                pressed: false,
                hovered: false,
            },
            Some(root),
        );
        let sheet = parse_stylesheet(".menu { padding: 0; gap: 32; }").unwrap();
        let resolved = ui.vcss_style(root, &sheet).unwrap();
        ui.get_mut(root).unwrap().style = resolved;
        ui.layout(Vec2::new(400.0, 300.0));

        let first = ui.get(first).unwrap().rect;
        let second = ui.get(second).unwrap().rect;
        close(second.pos.y - first.max().y, 32.0);
    }
}
