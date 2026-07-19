//! Resolve styles for a class list + pseudo-state.

use indexmap::IndexMap;

use crate::parse::{StyleRule, Stylesheet};
use crate::value::{Color, StyleValue};

/// Element query for matching.
#[derive(Debug, Clone, Default)]
pub struct StyleQuery {
    /// Element id without `#`.
    pub id: Option<String>,
    /// Class names without `.`.
    pub classes: Vec<String>,
    /// Pseudo states: `selected`, `hover`, `disabled`, …
    pub states: Vec<String>,
}

impl StyleQuery {
    /// Class only.
    pub fn class(name: impl Into<String>) -> Self {
        Self {
            classes: vec![name.into()],
            ..Default::default()
        }
    }

    /// Add class.
    pub fn with_class(mut self, name: impl Into<String>) -> Self {
        self.classes.push(name.into());
        self
    }

    /// Add state.
    pub fn with_state(mut self, name: impl Into<String>) -> Self {
        self.states.push(name.into());
        self
    }

    /// Set id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Computed style map (property → value).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ComputedStyle {
    /// Flat map.
    pub props: IndexMap<String, StyleValue>,
}

impl ComputedStyle {
    /// Get color prop or default.
    pub fn color(&self, name: &str, default: Color) -> Color {
        self.props
            .get(name)
            .and_then(|v| v.as_color())
            .unwrap_or(default)
    }

    /// Get number/length or default.
    pub fn number(&self, name: &str, default: f32) -> f32 {
        self.props
            .get(name)
            .and_then(|v| v.as_f32())
            .unwrap_or(default)
    }

    /// Keyword or default.
    pub fn keyword<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.props
            .get(name)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
    }

    /// Background convenience.
    pub fn background(&self) -> Color {
        self.color("background", Color::rgb(10, 12, 22))
    }

    /// Text color.
    pub fn color_text(&self) -> Color {
        self.color("color", Color::rgb(220, 210, 235))
    }

    /// Border color.
    pub fn border_color(&self) -> Color {
        self.color("border-color", Color::rgb(185, 150, 75))
    }
}

/// Specificity: (id, class, element) rough.
fn specificity(selector: &str) -> (u16, u16, u16) {
    let mut id = 0u16;
    let mut class = 0u16;
    let mut elem = 0u16;
    for part in selector.split(|c: char| c.is_whitespace()) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if part.starts_with('#') {
            id += 1;
        } else if part.starts_with('.') || part.contains(':') {
            // .button:selected → class + pseudo as class-level
            class += part.matches('.').count() as u16;
            class += part.matches(':').count() as u16;
            if part.starts_with('.') {
                // already counted dots
            } else if !part.starts_with(':') && !part.starts_with('#') {
                elem += 1;
            }
        } else if part.starts_with(':') {
            class += 1;
        } else {
            elem += 1;
        }
    }
    // fix .button counting
    if selector.contains('.') {
        class = class.max(1);
    }
    (id, class, elem)
}

fn selector_matches(selector: &str, q: &StyleQuery) -> bool {
    // support compound: .button:selected  or  #start  or  .button
    let sel = selector.trim();
    if sel.is_empty() {
        return false;
    }

    // split by . and # carefully
    // patterns we support: .class, .class:state, #id, #id:state, .a.b
    let mut rest = sel;
    let mut need_id: Option<&str> = None;
    let mut need_classes: Vec<&str> = Vec::new();
    let mut need_states: Vec<&str> = Vec::new();

    if let Some(stripped) = rest.strip_prefix('#') {
        let (id_part, after) = split_ident(stripped);
        need_id = Some(id_part);
        rest = after;
    }

    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix('.') {
            let (cls, after) = split_ident(stripped);
            need_classes.push(cls);
            rest = after;
        } else if let Some(stripped) = rest.strip_prefix(':') {
            let (st, after) = split_ident(stripped);
            need_states.push(st);
            rest = after;
        } else {
            // element type — accept any for now
            let (_, after) = split_ident(rest);
            rest = after;
            if !rest.is_empty() && !rest.starts_with('.') && !rest.starts_with(':') {
                break;
            }
        }
    }

    if let Some(id) = need_id {
        if q.id.as_deref() != Some(id) {
            return false;
        }
    }
    for c in need_classes {
        if !q.classes.iter().any(|x| x == c) {
            return false;
        }
    }
    for s in need_states {
        if !q.states.iter().any(|x| x == s) {
            return false;
        }
    }
    // if selector had only states without class/id, require state
    true
}

fn split_ident(s: &str) -> (&str, &str) {
    let end = s
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// Resolve computed style from sheet + query (cascade by specificity then order).
///
/// Custom properties (`--name`) are collected from `:root` and matched rules, then
/// `var(--name)` values are substituted (one-level; missing vars stay as [`StyleValue::Var`]).
pub fn resolve(sheet: &Stylesheet, query: &StyleQuery) -> ComputedStyle {
    let mut matched: Vec<(usize, (u16, u16, u16), &StyleRule, &str)> = Vec::new();
    for (order, rule) in sheet.rules.iter().enumerate() {
        for sel in &rule.selectors {
            let is_root = sel.trim() == ":root";
            if is_root || selector_matches(sel, query) {
                // :root always participates (for tokens); specificity stays low
                let spec = if is_root {
                    (0, 0, 0)
                } else {
                    specificity(sel)
                };
                matched.push((order, spec, rule, sel.as_str()));
            }
        }
    }
    matched.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut props = IndexMap::new();
    for (_, _, rule, _) in &matched {
        for (k, v) in &rule.declarations {
            props.insert(k.clone(), v.clone());
        }
    }
    // Custom property map (also includes non-var props named --x)
    let mut vars = IndexMap::new();
    for (k, v) in &props {
        if k.starts_with("--") {
            vars.insert(k.clone(), v.clone());
        }
    }
    // Substitute var()
    let mut resolved = IndexMap::new();
    for (k, v) in props {
        resolved.insert(k, resolve_var_value(&v, &vars, 0));
    }
    ComputedStyle { props: resolved }
}

fn resolve_var_value(
    value: &StyleValue,
    vars: &IndexMap<String, StyleValue>,
    depth: u8,
) -> StyleValue {
    if depth > 8 {
        return value.clone();
    }
    match value {
        StyleValue::Var(name) => {
            let key = if name.starts_with("--") {
                name.clone()
            } else {
                format!("--{name}")
            };
            if let Some(v) = vars.get(&key) {
                // Allow var pointing at another var
                resolve_var_value(v, vars, depth + 1)
            } else {
                StyleValue::Var(key)
            }
        }
        other => other.clone(),
    }
}

/// Expand shorthand box props on a computed style (margin/padding).
pub fn expand_box_shorthands(style: &mut ComputedStyle) {
    expand_quad_shorthand(style, "margin");
    expand_quad_shorthand(style, "padding");
}

fn expand_quad_shorthand(style: &mut ComputedStyle, base: &str) {
    let Some(v) = style.props.get(base).cloned() else {
        return;
    };
    let n = v.as_f32().unwrap_or(0.0);
    // 1-value: all sides
    for side in ["top", "right", "bottom", "left"] {
        let key = format!("{base}-{side}");
        style
            .props
            .entry(key)
            .or_insert(StyleValue::Number(n));
    }
    // convenience x/y if missing
    let x_key = format!("{base}-x");
    let y_key = format!("{base}-y");
    style
        .props
        .entry(x_key)
        .or_insert(StyleValue::Number(n));
    style
        .props
        .entry(y_key)
        .or_insert(StyleValue::Number(n));
}

/// Resolve then expand box shorthands.
pub fn resolve_expanded(sheet: &Stylesheet, query: &StyleQuery) -> ComputedStyle {
    let mut c = resolve(sheet, query);
    expand_box_shorthands(&mut c);
    c
}

/// Registry of named sheets for runtime invoke.
#[derive(Debug, Clone, Default)]
pub struct StyleRegistry {
    /// Sheets by name.
    pub sheets: IndexMap<String, Stylesheet>,
    /// Active default sheet name.
    pub active: Option<String>,
}

impl StyleRegistry {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert / replace sheet.
    pub fn insert(&mut self, name: impl Into<String>, sheet: Stylesheet) {
        let name = name.into();
        if self.active.is_none() {
            self.active = Some(name.clone());
        }
        self.sheets.insert(name, sheet);
    }

    /// Load and insert from source text.
    pub fn load_str(
        &mut self,
        name: impl Into<String>,
        source: &str,
    ) -> Result<(), crate::parse::StyleParseError> {
        let mut sheet = crate::parse::parse_stylesheet(source)?;
        let name = name.into();
        sheet.source = Some(name.clone());
        self.insert(name, sheet);
        Ok(())
    }

    /// Resolve using active sheet (or empty).
    pub fn resolve(&self, query: &StyleQuery) -> ComputedStyle {
        let Some(name) = &self.active else {
            return ComputedStyle::default();
        };
        let Some(sheet) = self.sheets.get(name) else {
            return ComputedStyle::default();
        };
        resolve(sheet, query)
    }

    /// Resolve from a named sheet.
    pub fn resolve_in(&self, sheet: &str, query: &StyleQuery) -> ComputedStyle {
        self.sheets
            .get(sheet)
            .map(|s| resolve(s, query))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_stylesheet;

    #[test]
    fn selected_overrides_base() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; background: #000000; }
            .button:selected { color: #ffe496; background: #501e78; }
            "#,
        )
        .unwrap();
        let base = resolve(&sheet, &StyleQuery::class("button"));
        assert_eq!(base.color_text().r, 0xaa);
        let sel = resolve(
            &sheet,
            &StyleQuery::class("button").with_state("selected"),
        );
        assert_eq!(sel.color_text().r, 0xff);
        assert_eq!(sel.background().r, 0x50);
    }

    #[test]
    fn id_selector() {
        let sheet = parse_stylesheet("#start { icon: star; height: 52; }").unwrap();
        let c = resolve(&sheet, &StyleQuery::default().with_id("start"));
        assert_eq!(c.keyword("icon", ""), "star");
        assert_eq!(c.number("height", 0.0), 52.0);
    }

    #[test]
    fn css_variables_from_root() {
        let sheet = parse_stylesheet(
            r#"
            :root {
              --gold: #ebc878;
              --pad: 14;
            }
            .button {
              color: var(--gold);
              padding-x: var(--pad);
              height: 52;
            }
            "#,
        )
        .unwrap();
        let c = resolve(&sheet, &StyleQuery::class("button"));
        let col = c.color_text();
        assert_eq!(col.r, 0xeb);
        assert_eq!(col.g, 0xc8);
        assert_eq!(c.number("padding-x", 0.0), 14.0);
        assert_eq!(c.number("height", 0.0), 52.0);
    }

    #[test]
    fn margin_shorthand_expands() {
        let sheet = parse_stylesheet(".box { margin: 8; width: 100; }").unwrap();
        let c = resolve_expanded(&sheet, &StyleQuery::class("box"));
        assert_eq!(c.number("margin-top", 0.0), 8.0);
        assert_eq!(c.number("margin-x", 0.0), 8.0);
        assert_eq!(c.number("width", 0.0), 100.0);
    }
}
