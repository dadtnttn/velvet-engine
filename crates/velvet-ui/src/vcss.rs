//! Bridge between retained UI nodes and VCSS stylesheets.

use velvet_math::{Color, Vec2};
use velvet_style::{
    parse_box_shadow, resolve_inherited, ComputedStyle, MediaContext, MediaOrientation, StyleQuery,
    StyleScope, StyleValue, Stylesheet,
};

use crate::{
    NinePatchMargins, NodeId, UiImageSource, UiNineSlice, UiNode, UiStyle, UiTree, WidgetKind,
};

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
        ..Default::default()
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
    style.background_gradient = computed.props.get("background").and_then(|v| match v {
        StyleValue::Gradient(g) => {
            // All-or-nothing: a stop that still holds var() (undefined
            // variable) drops the whole gradient, as before.
            let n = g.stops.len();
            let mut stops = Vec::with_capacity(n);
            for (i, s) in g.stops.iter().enumerate() {
                let pos = s.position.unwrap_or(i as f32 / (n - 1).max(1) as f32);
                stops.push((to_math_color(s.color.as_color()?), pos));
            }
            if stops.is_empty() {
                return None;
            }
            Some(crate::UiGradient {
                radial: g.radial,
                angle_deg: g.angle_deg,
                center: g.center,
                stops,
            })
        }
        _ => None,
    });
    style.box_shadows = computed
        .props
        .get("box-shadow")
        .and_then(|v| match v {
            StyleValue::Shadow(layers) => Some(layers.clone()),
            StyleValue::Keyword(s) | StyleValue::String(s) => parse_box_shadow(s),
            _ => None,
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|sh| {
            // Layers whose color still holds var() are dropped; the rest
            // are kept.
            sh.color.as_color().map(|c| crate::UiBoxShadow {
                offset: (sh.offset_x, sh.offset_y),
                blur: sh.blur,
                spread: sh.spread,
                color: to_math_color(c),
                inset: sh.inset,
            })
        })
        .collect();
    if let Some(source) = image_source(computed, "background-image") {
        style.background_image = source;
    }
    let source_touched = has_margin_property(computed, "nine-slice");
    let destination_touched = has_margin_property(computed, "nine-slice-width");
    if source_touched || destination_touched {
        let previous = style.nine_slice;
        let source = margin_values(
            computed,
            "nine-slice",
            previous
                .map(|slice| slice.source)
                .unwrap_or_else(|| NinePatchMargins::uniform(0.0)),
        );
        let destination_fallback = if source_touched {
            source
        } else {
            previous.map(|slice| slice.destination).unwrap_or(source)
        };
        let destination = margin_values(computed, "nine-slice-width", destination_fallback);
        style.nine_slice = Some(UiNineSlice {
            source,
            destination,
        });
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
    resolve_node_style_with_media(tree, id, sheet, &MediaContext::default())
}

/// Resolve with explicit `@media` facts (reduced motion, contrast, …).
pub fn resolve_node_style_with_media(
    tree: &UiTree,
    id: NodeId,
    sheet: &Stylesheet,
    media: &MediaContext,
) -> Option<UiStyle> {
    let node = tree.get(id)?;
    let mut query = style_query_for_node(tree, id)?;
    query.reduced_motion = media.reduced_motion;
    query.high_contrast = media.high_contrast;
    query.orientation = media.orientation;
    query.viewport_width = media.viewport_width;
    // Inherited resolution: text properties (`color`, `font-*`, custom
    // properties, …) flow from ancestor nodes, like CSS.
    Some(apply_computed_style(
        &node.style,
        &resolve_inherited(sheet, &query),
    ))
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

    /// Resolve with explicit `@media` facts.
    pub fn vcss_style_with_media(
        &self,
        id: NodeId,
        sheet: &Stylesheet,
        media: &MediaContext,
    ) -> Option<UiStyle> {
        resolve_node_style_with_media(self, id, sheet, media)
    }

    /// Resolve with `@media` facts: accessibility flags plus the layout
    /// viewport (orientation/width derive from it).
    pub fn vcss_style_auto(
        &self,
        id: NodeId,
        sheet: &Stylesheet,
        reduced_motion: bool,
        high_contrast: bool,
        viewport: Vec2,
    ) -> Option<UiStyle> {
        let (w, h) = (viewport.x, viewport.y);
        resolve_node_style_with_media(
            self,
            id,
            sheet,
            &MediaContext {
                reduced_motion,
                high_contrast,
                orientation: Some(if w >= h {
                    MediaOrientation::Landscape
                } else {
                    MediaOrientation::Portrait
                }),
                viewport_width: Some(w),
            },
        )
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
        WidgetKind::Dropdown { .. } => "dropdown",
        WidgetKind::Tabs { .. } => "tabs",
        WidgetKind::Tooltip { .. } => "tooltip",
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
    use crate::tree::InputModality;
    let mut states = Vec::new();
    if !node.enabled {
        states.push("disabled".into());
    }
    let focused = tree.focus.focused == Some(node.id)
        || matches!(node.widget, WidgetKind::TextField { focused: true, .. });
    if focused {
        states.push("focus".into());
        // Ring only for keyboard arrivals (mouse users get hover instead).
        if tree.input_modality == InputModality::Keyboard {
            states.push("focus-visible".into());
        }
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
    if matches!(node.widget, WidgetKind::Dropdown { open: true, .. }) {
        states.push("open".into());
    }
    states
}

fn to_math_color(color: velvet_style::Color) -> Color {
    Color::rgba(
        color.r as f32 / 255.0,
        color.g as f32 / 255.0,
        color.b as f32 / 255.0,
        color.a,
    )
}

fn color(computed: &ComputedStyle, property: &str) -> Option<Color> {
    computed
        .props
        .get(property)
        .and_then(|value| value.as_color())
        .map(to_math_color)
}

fn number(computed: &ComputedStyle, property: &str) -> Option<f32> {
    computed
        .props
        .get(property)
        .and_then(|value| value.as_f32())
}

fn image_source(computed: &ComputedStyle, property: &str) -> Option<Option<UiImageSource>> {
    computed.props.get(property).and_then(|value| match value {
        StyleValue::Url(path) => Some(Some(UiImageSource::Url(path.clone()))),
        StyleValue::SvgRef(name) => Some(Some(UiImageSource::Svg(name.clone()))),
        StyleValue::Keyword(value) if value.eq_ignore_ascii_case("none") => Some(None),
        _ => None,
    })
}

fn has_margin_property(computed: &ComputedStyle, property: &str) -> bool {
    computed.props.contains_key(property)
        || ["top", "right", "bottom", "left"]
            .iter()
            .any(|side| computed.props.contains_key(&format!("{property}-{side}")))
}

fn margin_values(
    computed: &ComputedStyle,
    property: &str,
    fallback: NinePatchMargins,
) -> NinePatchMargins {
    let mut values = computed
        .props
        .get(property)
        .and_then(style_number_list)
        .map(css_margins)
        .unwrap_or(fallback);
    for (side, slot) in [
        ("top", &mut values.top),
        ("right", &mut values.right),
        ("bottom", &mut values.bottom),
        ("left", &mut values.left),
    ] {
        if let Some(value) = number(computed, &format!("{property}-{side}")) {
            *slot = value.max(0.0);
        }
    }
    values
}

fn style_number_list(value: &StyleValue) -> Option<Vec<f32>> {
    if let Some(value) = value.as_f32() {
        return Some(vec![value.max(0.0)]);
    }
    let values: Vec<f32> = value
        .as_str()?
        .split_whitespace()
        .map(|token| token.trim_end_matches("px").parse::<f32>())
        .collect::<Result<_, _>>()
        .ok()?;
    (!values.is_empty() && values.len() <= 4).then_some(values)
}

fn css_margins(values: Vec<f32>) -> NinePatchMargins {
    let value = |index: usize| values[index].max(0.0);
    match values.len() {
        1 => NinePatchMargins::uniform(value(0)),
        2 => NinePatchMargins::new(value(1), value(0), value(1), value(0)),
        3 => NinePatchMargins::new(value(1), value(0), value(1), value(2)),
        4 => NinePatchMargins::new(value(3), value(0), value(1), value(2)),
        _ => NinePatchMargins::uniform(0.0),
    }
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
    use velvet_style::{parse_stylesheet, MediaContext};

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
    }

    #[test]
    fn child_and_not_selectors_work_through_nested_tree() {
        let mut ui = UiTree::with_root("main");
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().add_class("menu");
        let panel = ui.add_node("panel", WidgetKind::Panel, Some(root));
        let button = ui.add_node(
            "go",
            WidgetKind::Button {
                label: "Go".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
            Some(panel),
        );
        ui.get_mut(button).unwrap().add_class("action");
        let sheet = parse_stylesheet(
            r#"
            .menu > .action { opacity: 0.2; }
            .menu .action:not(.ghost) { opacity: 0.9; }
            "#,
        )
        .unwrap();
        // Grandchild: child rule misses (panel sits between), :not hits.
        let style = ui.vcss_style(button, &sheet).unwrap();
        close(style.opacity, 0.9);
        // Ghost veto drops back to the retained default.
        ui.get_mut(button).unwrap().add_class("ghost");
        let style = ui.vcss_style(button, &sheet).unwrap();
        close(style.opacity, 1.0);
    }

    #[test]
    fn media_and_chrome_map_through_bridge() {
        let mut ui = UiTree::with_root("main");
        let root = ui.root().unwrap();
        let sheet = parse_stylesheet(
            r#"
            .banner {
              background: linear-gradient(to bottom, red, blue);
              box-shadow: 0 2px 8px #000000;
              opacity: 1;
            }
            @media (prefers-reduced-motion) {
              .banner { opacity: 0.5; }
            }
            "#,
        )
        .unwrap();
        ui.get_mut(root).unwrap().add_class("banner");
        let style = ui.vcss_style(root, &sheet).unwrap();
        close(style.opacity, 1.0);
        let gradient = style.background_gradient.as_ref().expect("gradient");
        assert_eq!(gradient.stops.len(), 2);
        assert_eq!(gradient.stops[0].1, 0.0);
        assert_eq!(gradient.stops[1].1, 1.0);
        assert_eq!(style.box_shadows.len(), 1);
        close(style.box_shadows[0].blur, 8.0);
        // Reduced motion swaps opacity through the same bridge call.
        let calm = ui
            .vcss_style_with_media(
                root,
                &sheet,
                &MediaContext {
                    reduced_motion: true,
                    ..Default::default()
                },
            )
            .unwrap();
        close(calm.opacity, 0.5);
        // Viewport-derived flags work too (800px landscape here).
        let wide = ui
            .vcss_style_auto(root, &sheet, false, false, Vec2::new(800.0, 600.0))
            .unwrap();
        close(wide.opacity, 1.0);
    }

    #[test]
    fn vars_inside_color_functions_apply_through_bridge() {
        let mut ui = UiTree::with_root("main");
        let root = ui.root().unwrap();
        let sheet = parse_stylesheet(
            r#"
            :root { --gold: #ebc878; }
            .banner {
              background: color-mix(in srgb, var(--gold) 70%, #501e78);
              box-shadow: 0 2px 8px var(--gold);
              width: calc(8px + 4px);
            }
            "#,
        )
        .unwrap();
        ui.get_mut(root).unwrap().add_class("banner");
        let style = ui.vcss_style(root, &sheet).unwrap();
        // Same mix authored concrete: 70% gold + 30% plum.
        let mixed = velvet_style::parse_color("color-mix(in srgb, #ebc878 70%, #501e78)").unwrap();
        let bg = style.background;
        assert!((bg.r - mixed.r as f32 / 255.0).abs() < 0.01, "r {bg:?}");
        assert!((bg.g - mixed.g as f32 / 255.0).abs() < 0.01, "g {bg:?}");
        assert!((bg.b - mixed.b as f32 / 255.0).abs() < 0.01, "b {bg:?}");
        assert_eq!(style.box_shadows.len(), 1);
        let sh = &style.box_shadows[0];
        assert!((sh.color.r - 0xeb as f32 / 255.0).abs() < 0.01);
    }

    #[test]
    fn text_props_inherit_from_parent_nodes() {
        let mut ui = UiTree::with_root("main");
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().add_class("menu");
        let button = ui.add_node(
            "ok",
            WidgetKind::Button {
                label: "Ok".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
            Some(root),
        );
        ui.get_mut(button).unwrap().add_class("button");
        let sheet = parse_stylesheet(
            r#"
            .menu { color: #ebc878; font-size: 20; opacity: 0.5; }
            .button { font-size: 14; }
            "#,
        )
        .unwrap();
        let style = ui.vcss_style(button, &sheet).unwrap();
        // color flows down, own font-size wins, opacity never inherits.
        assert!((style.text_color.r - 0xeb as f32 / 255.0).abs() < 0.01);
        assert!((style.font_size - 14.0).abs() < 1e-4);
        assert!((style.opacity - 1.0).abs() < 1e-4);
    }

    #[test]
    fn border_and_background_shorthands_apply_through_bridge() {
        let mut ui = UiTree::with_root("main");
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().add_class("card");
        let sheet = parse_stylesheet(
            r#"
            .card { border: 2px solid red; background: url(a.png); }
            "#,
        )
        .unwrap();
        let style = ui.vcss_style(root, &sheet).unwrap();
        assert!((style.border_width - 2.0).abs() < 1e-4);
        assert!((style.border.r - 1.0).abs() < 1e-4);
        assert_eq!(
            style.background_image,
            Some(UiImageSource::Url("a.png".into()))
        );
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
                busy: false,
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
    fn image_and_nine_slice_properties_map_onto_ui_style() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        ui.get_mut(root).unwrap().add_class("ornate");
        let sheet = parse_stylesheet(
            r#"
            .ornate {
                background-image: url("ui/button.png");
                nine-slice: 120 180 100 180;
                nine-slice-width: 14 28;
            }
            "#,
        )
        .unwrap();

        let style = resolve_node_style(&ui, root, &sheet).unwrap();
        assert_eq!(
            style.background_image,
            Some(UiImageSource::Url("ui/button.png".into()))
        );
        let slice = style.nine_slice.unwrap();
        assert_eq!(
            slice.source,
            NinePatchMargins::new(180.0, 120.0, 180.0, 100.0)
        );
        assert_eq!(
            slice.destination,
            NinePatchMargins::new(28.0, 14.0, 28.0, 14.0)
        );
    }

    #[test]
    fn pseudo_state_can_swap_background_image() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        let button = ui.add_node(
            "start",
            WidgetKind::Button {
                label: "Start".into(),
                pressed: false,
                hovered: true,
                busy: false,
            },
            Some(root),
        );
        let sheet = parse_stylesheet(
            r#"
            button { background-image: url("idle.png"); }
            button:hover { background-image: url("hover.png"); }
            "#,
        )
        .unwrap();
        let style = resolve_node_style(&ui, button, &sheet).unwrap();
        assert_eq!(
            style.background_image,
            Some(UiImageSource::Url("hover.png".into()))
        );
    }

    #[test]
    fn nine_slice_width_can_override_a_base_style() {
        let base = UiStyle {
            nine_slice: Some(UiNineSlice {
                source: NinePatchMargins::uniform(100.0),
                destination: NinePatchMargins::uniform(20.0),
            }),
            ..UiStyle::default()
        };
        let sheet = parse_stylesheet(".compact { nine-slice-width: 8 16; }").unwrap();
        let computed = resolve_inherited(&sheet, &StyleQuery::class("compact"));
        let style = apply_computed_style(&base, &computed);
        let slice = style.nine_slice.unwrap();
        assert_eq!(slice.source, NinePatchMargins::uniform(100.0));
        assert_eq!(
            slice.destination,
            NinePatchMargins::new(16.0, 8.0, 16.0, 8.0)
        );
    }

    #[test]
    fn vcss_gap_changes_column_layout() {
        let mut ui = UiTree::with_root("root");
        let root = ui.root().unwrap();
        {
            let root_node = ui.get_mut(root).unwrap();
            root_node.add_class("menu");
            root_node.layout = LayoutType::Column { wrap: false };
        }
        let first = ui.add_node(
            "first",
            WidgetKind::Button {
                label: "A".into(),
                pressed: false,
                hovered: false,
                busy: false,
            },
            Some(root),
        );
        let second = ui.add_node(
            "second",
            WidgetKind::Button {
                label: "B".into(),
                pressed: false,
                hovered: false,
                busy: false,
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
