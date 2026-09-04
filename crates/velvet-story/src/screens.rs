//! Easy VN screens: say / menu / nvl in 3 lines.
//!
//! A [`ScreenPreset`] describes a full layout (say box, choice menu, nvl page)
//! without hand-rolling coordinates. Authors pick one preset + optional tweaks.
//!
//! Ren'Py-style game screens (`screen inv(…) { … }`) lower to [`ScreenDef`]:
//! data the host renders (params, widget tree, image refs). Dynamic values
//! keep their source form ([`ScreenValueRt::Expr`]) so hosts resolve them
//! against session variables at display time.

use serde::{Deserialize, Serialize};

use crate::value::{from_ast_expr, StoryValue};
use velvet_script_ast::{Expr, ScreenButton, ScreenNode, ScreenParam, ScreenProperty};

/// Which screen family to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenMode {
    /// Classic ADV say box (namebox + single line).
    #[default]
    Say,
    /// Choice menu over dimmed say.
    Menu,
    /// NVL full-page multi-line log.
    Nvl,
}

/// Layout for a VN screen (0..=1 relative coords, 1280x720 design space).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenLayout {
    /// Mode this layout targets.
    pub mode: ScreenMode,
    /// Say box x,y,w,h (relative).
    pub say_rect: [f32; 4],
    /// Namebox visible.
    pub namebox_visible: bool,
    /// Max visible dialogue lines (1 = ADV, N = NVL).
    pub max_lines: usize,
    /// Choice menu centered.
    pub menu_centered: bool,
    /// Dim background behind menu 0..=1.
    pub menu_dim: f32,
    /// Font scale multiplier.
    pub font_scale: f32,
}

impl Default for ScreenLayout {
    fn default() -> Self {
        Self::say_classic()
    }
}

impl ScreenLayout {
    /// Classic ADV say: bottom box, namebox on, 3 wrapped lines.
    pub fn say_classic() -> Self {
        Self {
            mode: ScreenMode::Say,
            say_rect: [0.08, 0.72, 0.84, 0.22],
            namebox_visible: true,
            max_lines: 3,
            menu_centered: true,
            menu_dim: 0.45,
            font_scale: 1.0,
        }
    }

    /// Minimal say: smaller box, no namebox frame (narrator VN).
    pub fn say_minimal() -> Self {
        Self {
            mode: ScreenMode::Say,
            say_rect: [0.12, 0.78, 0.76, 0.16],
            namebox_visible: false,
            max_lines: 2,
            menu_centered: true,
            menu_dim: 0.35,
            font_scale: 1.0,
        }
    }

    /// NVL page: full-screen text log.
    pub fn nvl_page() -> Self {
        Self {
            mode: ScreenMode::Nvl,
            say_rect: [0.08, 0.08, 0.84, 0.84],
            namebox_visible: true,
            max_lines: 24,
            menu_centered: true,
            menu_dim: 0.25,
            font_scale: 1.0,
        }
    }

    /// Clamp to sane ranges.
    pub fn validated(mut self) -> Self {
        for v in self.say_rect.iter_mut() {
            *v = v.clamp(0.0, 1.0);
        }
        if self.say_rect[2] < 0.1 {
            self.say_rect[2] = 0.1;
        }
        if self.say_rect[3] < 0.05 {
            self.say_rect[3] = 0.05;
        }
        self.max_lines = self.max_lines.clamp(1, 40);
        self.menu_dim = self.menu_dim.clamp(0.0, 0.9);
        self.font_scale = self.font_scale.clamp(0.7, 2.5);
        self
    }
}

/// Named preset: 3-line authoring.
///
/// ```vel
/// screen say_classic
/// screen nvl_page
/// screen say_minimal
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenPreset {
    /// Preset id.
    pub id: String,
    /// Layout.
    pub layout: ScreenLayout,
}

impl ScreenPreset {
    /// Build from id (`say_classic` / `say_minimal` / `nvl_page` / `menu`).
    pub fn from_id(id: &str) -> Option<Self> {
        let layout = match id.trim().to_ascii_lowercase().as_str() {
            "say" | "say_classic" | "classic" => ScreenLayout::say_classic(),
            "minimal" | "say_minimal" => ScreenLayout::say_minimal(),
            "nvl" | "nvl_page" => ScreenLayout::nvl_page(),
            "menu" => ScreenLayout {
                mode: ScreenMode::Menu,
                ..ScreenLayout::say_classic()
            },
            _ => return None,
        };
        Some(Self {
            id: id.trim().to_string(),
            layout: layout.validated(),
        })
    }

    /// All preset ids for UI/docs.
    pub fn all_ids() -> &'static [&'static str] {
        &["say_classic", "say_minimal", "nvl_page", "menu"]
    }

    /// One-line summary for launcher UI.
    pub fn summary(&self) -> String {
        format!(
            "{} mode={:?} lines={} namebox={}",
            self.id,
            self.layout.mode,
            self.layout.max_lines,
            if self.layout.namebox_visible {
                "on"
            } else {
                "off"
            }
        )
    }
}

/// NVL page buffer: keeps last N lines for full-page mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NvlPage {
    /// Lines `Speaker: text`.
    lines: Vec<String>,
    /// Max lines kept.
    pub capacity: usize,
}

impl NvlPage {
    /// New buffer.
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: Vec::new(),
            capacity: capacity.clamp(2, 60),
        }
    }

    /// Push a line, evicting oldest.
    pub fn push(&mut self, speaker: &str, text: &str) {
        let line = if speaker.trim().is_empty() {
            text.to_string()
        } else {
            format!("{}: {}", speaker.trim(), text)
        };
        self.lines.push(line);
        while self.lines.len() > self.capacity {
            self.lines.remove(0);
        }
    }

    /// Clear page.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Visible lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Joined page text.
    pub fn page_text(&self) -> String {
        self.lines.join("\n")
    }
}

/// Widget kinds accepted inside game screens (mirrors the parser list,
/// plus legacy `button`).
pub const SCREEN_WIDGETS: &[&str] = &[
    "text",
    "textbutton",
    "image",
    "bar",
    "vbox",
    "hbox",
    "fixed",
    "use",
    "timer",
    "key",
    "input",
    "button",
];

/// A screen value: literal, or a dynamic expression in source form
/// (resolved by the host against session variables at display time,
/// e.g. `text score` shows the `score` variable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScreenValueRt {
    /// Literal value.
    Lit {
        /// Value.
        value: StoryValue,
    },
    /// Dynamic expression (source form, e.g. `items` or `Return("ok")`).
    Expr {
        /// Source form.
        source: String,
    },
}

impl ScreenValueRt {
    /// Literal helper.
    pub fn lit(value: StoryValue) -> Self {
        Self::Lit { value }
    }

    /// Display form for hosts (literals render, expressions show source).
    pub fn display(&self) -> String {
        match self {
            Self::Lit { value } => value.display_str(),
            Self::Expr { source } => source.clone(),
        }
    }
}

/// One named screen property (`title: "Bag"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenPropRt {
    /// Property name.
    pub name: String,
    /// Value.
    pub value: ScreenValueRt,
}

/// One widget in a game screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenChildRt {
    /// Widget kind (`text`, `vbox`, `timer`, …).
    pub kind: String,
    /// Properties in author order.
    #[serde(default)]
    pub props: Vec<ScreenPropRt>,
    /// Positional args for `use <screen> (…)` (empty otherwise).
    #[serde(default)]
    pub args: Vec<ScreenValueRt>,
    /// Nested children (containers).
    #[serde(default)]
    pub children: Vec<ScreenChildRt>,
}

/// One screen parameter (`gold = 0`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenParamRt {
    /// Parameter name.
    pub name: String,
    /// Default when the caller omits it (None = required).
    pub default: Option<StoryValue>,
}

/// A lowered, host-renderable game screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenDef {
    /// Screen name.
    pub name: String,
    /// Parameters in author order.
    #[serde(default)]
    pub params: Vec<ScreenParamRt>,
    /// Screen-level properties (`title: …`).
    #[serde(default)]
    pub properties: Vec<ScreenPropRt>,
    /// Widget tree.
    #[serde(default)]
    pub children: Vec<ScreenChildRt>,
    /// Image asset refs found in string props (for `check-assets`).
    #[serde(default)]
    pub images: Vec<String>,
    /// `zorder:` prop — paint stacking hint for hosts.
    #[serde(default)]
    pub zorder: Option<f32>,
    /// `tag:` prop — showing a screen replaces others with the same tag.
    #[serde(default)]
    pub tag: Option<String>,
}

impl ScreenDef {
    /// All `use <screen> (args…)` references as (target, arg_count).
    pub fn uses(&self) -> Vec<(String, usize)> {
        let mut out = Vec::new();
        fn walk(node: &ScreenChildRt, out: &mut Vec<(String, usize)>) {
            if node.kind == "use" {
                if let Some(first) = node.props.first() {
                    out.push((first.value.display(), node.args.len()));
                }
            }
            for child in &node.children {
                walk(child, out);
            }
        }
        for child in &self.children {
            walk(child, &mut out);
        }
        out
    }
}

/// Check positional arg count against params (defaults are optional).
pub fn check_call_arity(
    screen: &str,
    params: &[ScreenParamRt],
    args_len: usize,
) -> Result<(), String> {
    let required = params.iter().filter(|p| p.default.is_none()).count();
    if args_len < required {
        return Err(format!(
            "screen `{screen}` needs {required} argument(s), got {args_len}"
        ));
    }
    if args_len > params.len() {
        return Err(format!(
            "screen `{screen}` takes at most {} argument(s), got {args_len}",
            params.len()
        ));
    }
    Ok(())
}

/// Lower a parsed `screen` block into a [`ScreenDef`].
pub fn lower_screen(
    name: &str,
    properties: &[ScreenProperty],
    buttons: &[ScreenButton],
    params: &[ScreenParam],
    children: &[ScreenNode],
) -> Result<ScreenDef, String> {
    let mut seen_params = std::collections::HashSet::new();
    let mut out_params = Vec::new();
    for p in params {
        if !seen_params.insert(p.name.as_str()) {
            return Err(format!("screen `{name}` declares param `{}` twice", p.name));
        }
        let default = match &p.default {
            None => None,
            Some(e) => Some(from_ast_expr(e).ok_or_else(|| {
                format!(
                    "default for param `{}` in screen `{name}` must be a literal",
                    p.name
                )
            })?),
        };
        out_params.push(ScreenParamRt {
            name: p.name.clone(),
            default,
        });
    }
    let mut out_props = Vec::new();
    for prop in properties {
        let value = from_ast_expr(&prop.value).ok_or_else(|| {
            format!(
                "property `{}` in screen `{name}` must be a literal",
                prop.name
            )
        })?;
        out_props.push(ScreenPropRt {
            name: prop.name.clone(),
            value: ScreenValueRt::lit(value),
        });
    }
    let mut out_children = Vec::new();
    // Legacy `button id { … }` becomes a `button` widget with an `id` prop.
    for b in buttons {
        let mut props = vec![ScreenPropRt {
            name: "id".into(),
            value: ScreenValueRt::lit(StoryValue::String(b.id.clone())),
        }];
        for prop in &b.properties {
            let value = from_ast_expr(&prop.value).ok_or_else(|| {
                format!(
                    "property `{}` in button `{}` of screen `{name}` must be a literal",
                    prop.name, b.id
                )
            })?;
            props.push(ScreenPropRt {
                name: prop.name.clone(),
                value: ScreenValueRt::lit(value),
            });
        }
        out_children.push(ScreenChildRt {
            kind: "button".into(),
            props,
            args: Vec::new(),
            children: Vec::new(),
        });
    }
    for node in children {
        out_children.push(lower_node(node)?);
    }
    let mut def = ScreenDef {
        name: name.to_string(),
        params: out_params,
        properties: out_props,
        children: out_children,
        images: Vec::new(),
        zorder: None,
        tag: None,
    };
    // Live props honored by the session: `zorder:` (paint stacking) and
    // `tag:` (showing replaces same-tag screens).
    for prop in &def.properties {
        match (prop.name.as_str(), &prop.value) {
            ("zorder", ScreenValueRt::Lit { value }) => {
                def.zorder = value.as_f64().map(|f| f as f32);
                if def.zorder.is_none() {
                    return Err(format!(
                        "property `zorder` in screen `{name}` must be a number"
                    ));
                }
            }
            ("tag", ScreenValueRt::Lit { value }) => match value {
                StoryValue::String(s) => {
                    def.tag = Some(s.clone());
                }
                _ => {
                    return Err(format!(
                        "property `tag` in screen `{name}` must be a string"
                    ));
                }
            },
            _ => {}
        }
    }
    collect_images(&def.children, &mut def.images);
    Ok(def)
}

fn lower_node(node: &ScreenNode) -> Result<ScreenChildRt, String> {
    match node {
        ScreenNode::Text { text, .. } => Ok(ScreenChildRt {
            kind: "text".into(),
            props: vec![ScreenPropRt {
                name: "text".into(),
                value: dyn_value(text),
            }],
            args: Vec::new(),
            children: Vec::new(),
        }),
        ScreenNode::TextButton { label, action, .. } => Ok(ScreenChildRt {
            kind: "textbutton".into(),
            props: vec![
                ScreenPropRt {
                    name: "label".into(),
                    value: dyn_value(label),
                },
                ScreenPropRt {
                    name: "action".into(),
                    value: dyn_action_value(action),
                },
            ],
            args: Vec::new(),
            children: Vec::new(),
        }),
        ScreenNode::Use { screen, args, .. } => Ok(ScreenChildRt {
            kind: "use".into(),
            props: vec![ScreenPropRt {
                name: "screen".into(),
                value: ScreenValueRt::lit(StoryValue::String(screen.clone())),
            }],
            args: args.iter().map(dyn_value).collect(),
            children: Vec::new(),
        }),
        ScreenNode::Widget {
            kind,
            properties,
            children,
            ..
        } => {
            if !SCREEN_WIDGETS.contains(&kind.as_str()) {
                return Err(format!("unknown screen widget `{kind}`"));
            }
            let mut props = Vec::new();
            for prop in properties {
                // `action:` props keep a parseable call form so hosts and
                // the validator can read name + literal args back.
                let value = if prop.name == "action" {
                    dyn_action_value(&prop.value)
                } else {
                    dyn_value(&prop.value)
                };
                props.push(ScreenPropRt {
                    name: prop.name.clone(),
                    value,
                });
            }
            let mut out_children = Vec::new();
            for child in children {
                out_children.push(lower_node(child)?);
            }
            Ok(ScreenChildRt {
                kind: kind.clone(),
                props,
                args: Vec::new(),
                children: out_children,
            })
        }
    }
}

fn dyn_value(expr: &Expr) -> ScreenValueRt {
    match from_ast_expr(expr) {
        Some(v) => ScreenValueRt::lit(v),
        None => ScreenValueRt::Expr {
            source: expr_display(expr),
        },
    }
}

/// Action value: literals stay literal, calls keep a parseable
/// `Name("lit", …)` source form (see [`parse_screen_action`]).
fn dyn_action_value(expr: &Expr) -> ScreenValueRt {
    if let Some(v) = from_ast_expr(expr) {
        return ScreenValueRt::lit(v);
    }
    match expr {
        Expr::Call { callee, args, .. } => {
            let name = match callee.as_ref() {
                Expr::Ident { name, .. } => name.clone(),
                _ => return dyn_value(expr),
            };
            let mut rendered = Vec::new();
            for arg in args {
                match from_ast_expr(arg) {
                    Some(StoryValue::String(s)) => rendered.push(format!(
                        "\"{}\"",
                        s.replace('\\', "\\\\").replace('"', "\\\"")
                    )),
                    Some(v) => rendered.push(v.display_str()),
                    None => return dyn_value(expr),
                }
            }
            ScreenValueRt::Expr {
                source: format!("{name}({})", rendered.join(", ")),
            }
        }
        _ => dyn_value(expr),
    }
}

/// Minimal source-form rendering for dynamic screen expressions.
pub fn expr_display(expr: &Expr) -> String {
    match expr {
        Expr::Null { .. } => "null".into(),
        Expr::Bool { value, .. } => value.to_string(),
        Expr::Int { value, .. } => value.to_string(),
        Expr::Float { value, .. } => value.to_string(),
        Expr::String { value, .. } => value.clone(),
        Expr::Ident { name, .. } => name.clone(),
        Expr::List { elements, .. } => {
            let items: Vec<String> = elements.iter().map(expr_display).collect();
            format!("[{}]", items.join(", "))
        }
        Expr::Call { callee, args, .. } => {
            let f = expr_display(callee);
            let items: Vec<String> = args.iter().map(expr_display).collect();
            format!("{f}({})", items.join(", "))
        }
        Expr::Field { object, field, .. } => format!("{}.{field}", expr_display(object)),
        Expr::Index { object, index, .. } => {
            format!("{}[{}]", expr_display(object), expr_display(index))
        }
        Expr::Unary { op, expr, .. } => format!("{op:?}({})", expr_display(expr)),
        Expr::Binary {
            left, op, right, ..
        } => format!("({} {op:?} {})", expr_display(left), expr_display(right)),
        Expr::Map { entries, .. } => {
            let items: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{k}: {}", expr_display(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

fn collect_images(children: &[ScreenChildRt], out: &mut Vec<String>) {
    for child in children {
        for prop in &child.props {
            if let ScreenValueRt::Lit {
                value: StoryValue::String(s),
            } = &prop.value
            {
                if looks_like_image(s) && !out.contains(s) {
                    out.push(s.clone());
                }
            }
        }
        collect_images(&child.children, out);
    }
}

fn looks_like_image(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    [".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp"]
        .iter()
        .any(|ext| lower.ends_with(ext))
}

/// A parsed screen action call (`Return("ok")`, `Jump("start")`, …).
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenActionCall {
    /// Action name.
    pub name: String,
    /// Literal arguments.
    pub args: Vec<StoryValue>,
}

/// Parse an action source string into name + literal args.
///
/// Returns None for dynamic (non-literal) actions, which hosts resolve.
pub fn parse_screen_action(src: &str) -> Option<ScreenActionCall> {
    let src = src.trim();
    let open = src.find('(')?;
    let name = src[..open].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return None;
    }
    let rest = src[open + 1..].trim();
    let inner = rest.strip_suffix(')')?;
    let mut args = Vec::new();
    if !inner.trim().is_empty() {
        for part in split_action_args(inner) {
            args.push(parse_action_literal(part.trim())?);
        }
    }
    Some(ScreenActionCall {
        name: name.to_string(),
        args,
    })
}

fn split_action_args(inner: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut in_str = false;
    let mut start = 0usize;
    let bytes: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if c == '\\' {
                i += 1;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '(' || c == '[' {
            depth += 1;
        } else if c == ')' || c == ']' {
            depth = depth.saturating_sub(1);
        } else if c == ',' && depth == 0 {
            parts.push(&inner[start..i]);
            start = i + 1;
        }
        i += 1;
    }
    parts.push(&inner[start..]);
    parts
}

fn parse_action_literal(part: &str) -> Option<StoryValue> {
    if let Some(s) = part.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Some(StoryValue::String(
            s.replace("\\\"", "\"").replace("\\\\", "\\"),
        ));
    }
    match part.to_ascii_lowercase().as_str() {
        "true" => return Some(StoryValue::Bool(true)),
        "false" => return Some(StoryValue::Bool(false)),
        "null" | "none" => return Some(StoryValue::Null),
        _ => {}
    }
    if let Ok(n) = part.parse::<i64>() {
        return Some(StoryValue::Int(n));
    }
    if let Ok(f) = part.parse::<f64>() {
        return Some(StoryValue::Float(f));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_resolve_in_3_lines() {
        for id in ScreenPreset::all_ids() {
            let p = ScreenPreset::from_id(id).unwrap();
            assert!(!p.summary().is_empty());
        }
        assert!(ScreenPreset::from_id("nope").is_none());
    }

    #[test]
    fn nvl_buffer_evicts_oldest() {
        let mut page = NvlPage::new(3);
        page.push("A", "1");
        page.push("B", "2");
        page.push("", "3");
        page.push("C", "4");
        assert_eq!(page.lines().len(), 3);
        assert!(page.page_text().contains("4"));
        assert!(!page.page_text().contains("A: 1"));
    }

    #[test]
    fn lower_game_screen_keeps_dynamics() {
        use velvet_script_ast::{Expr, ScreenNode, SourceLoc};
        let loc = SourceLoc::unknown;
        let def = lower_screen(
            "inv",
            &[],
            &[],
            &[],
            &[
                ScreenNode::Text {
                    text: Expr::String {
                        value: "Loot:".into(),
                        loc: loc(),
                    },
                    loc: loc(),
                },
                ScreenNode::Text {
                    text: Expr::Ident {
                        name: "items".into(),
                        loc: loc(),
                    },
                    loc: loc(),
                },
                ScreenNode::Widget {
                    kind: "image".into(),
                    properties: vec![velvet_script_ast::ScreenProperty {
                        name: "path".into(),
                        value: Expr::String {
                            value: "ui/bag.png".into(),
                            loc: loc(),
                        },
                        loc: loc(),
                    }],
                    children: Vec::new(),
                    loc: loc(),
                },
            ],
        )
        .unwrap();
        assert_eq!(def.children.len(), 3);
        assert!(matches!(
            def.children[0].props[0].value,
            ScreenValueRt::Lit { .. }
        ));
        assert!(matches!(
            def.children[1].props[0].value,
            ScreenValueRt::Expr { .. }
        ));
        assert_eq!(def.images, vec!["ui/bag.png".to_string()]);
    }

    #[test]
    fn lower_rejects_unknown_widget_and_bad_arity() {
        use velvet_script_ast::{ScreenNode, SourceLoc};
        let loc = SourceLoc::unknown;
        let err = lower_screen(
            "bad",
            &[],
            &[],
            &[],
            &[ScreenNode::Widget {
                kind: "spinner".into(),
                properties: Vec::new(),
                children: Vec::new(),
                loc: loc(),
            }],
        )
        .unwrap_err();
        assert!(err.contains("spinner"), "{err}");
        let def = ScreenDef {
            name: "s".into(),
            params: vec![ScreenParamRt {
                name: "a".into(),
                default: None,
            }],
            properties: Vec::new(),
            children: Vec::new(),
            images: Vec::new(),
            zorder: None,
            tag: None,
        };
        assert!(check_call_arity("s", &def.params, 0).is_err());
        assert!(check_call_arity("s", &def.params, 2).is_err());
        assert!(check_call_arity("s", &def.params, 1).is_ok());
    }

    #[test]
    fn action_call_parses_literals() {
        let c = parse_screen_action("Return(\"ok\")").unwrap();
        assert_eq!(c.name, "Return");
        assert_eq!(c.args, vec![StoryValue::String("ok".into())]);
        let c = parse_screen_action("SetScore(3, true)").unwrap();
        assert_eq!(c.args.len(), 2);
        assert!(parse_screen_action("Return(score)").is_none());
        assert!(parse_screen_action("not an action").is_none());
    }

    #[test]
    fn layout_validated_clamps() {
        let l = ScreenLayout {
            say_rect: [-1.0, 0.0, 99.0, 0.0],
            max_lines: 999,
            menu_dim: 5.0,
            font_scale: 0.1,
            ..ScreenLayout::say_classic()
        }
        .validated();
        assert!(l.say_rect[0] >= 0.0 && l.say_rect[2] <= 1.0);
        assert!(l.max_lines <= 40);
    }
}
