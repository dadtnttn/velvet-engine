//! Resolve styles for a class list + pseudo-state.

use indexmap::IndexMap;

use crate::parse::{StyleRule, Stylesheet};
use crate::value::{Color, StyleValue};

/// One ancestor in a contextual selector query.
///
/// Ancestors supplied to [`StyleQuery`] are ordered from the root toward the
/// queried element's direct parent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleScope {
    /// Optional element type, such as `menu` or `button`.
    pub element: Option<String>,
    /// Element id without `#`.
    pub id: Option<String>,
    /// Class names without `.`.
    pub classes: Vec<String>,
    /// Pseudo states active on this ancestor.
    pub states: Vec<String>,
}

impl StyleScope {
    /// Scope containing one class.
    pub fn class(name: impl Into<String>) -> Self {
        Self {
            classes: vec![name.into()],
            ..Default::default()
        }
    }

    /// Set the scope element type.
    pub fn with_element(mut self, element: impl Into<String>) -> Self {
        self.element = Some(element.into());
        self
    }

    /// Set the scope id.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a class to the scope.
    pub fn with_class(mut self, name: impl Into<String>) -> Self {
        self.classes.push(name.into());
        self
    }

    /// Add a pseudo state to the scope.
    pub fn with_state(mut self, name: impl Into<String>) -> Self {
        self.states.push(name.into());
        self
    }
}

/// Element query for matching.
#[derive(Debug, Clone, Default)]
pub struct StyleQuery {
    /// Optional element type, such as `button`.
    pub element: Option<String>,
    /// Element id without `#`.
    pub id: Option<String>,
    /// Class names without `.`.
    pub classes: Vec<String>,
    /// Pseudo states: `selected`, `hover`, `disabled`, …
    pub states: Vec<String>,
    /// Ancestor scopes, ordered root-to-parent, for descendant selectors.
    pub ancestors: Vec<StyleScope>,
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

    /// Set the element type used by type selectors.
    pub fn with_element(mut self, element: impl Into<String>) -> Self {
        self.element = Some(element.into());
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

    /// Append an ancestor scope.
    ///
    /// Call this in root-to-parent order when adding more than one ancestor.
    pub fn with_ancestor(mut self, scope: StyleScope) -> Self {
        self.ancestors.push(scope);
        self
    }

    /// Append an ancestor scope containing one class.
    pub fn with_ancestor_class(self, class: impl Into<String>) -> Self {
        self.with_ancestor(StyleScope::class(class))
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

#[derive(Debug)]
struct CompoundSelector<'a> {
    element: Option<&'a str>,
    ids: Vec<&'a str>,
    classes: Vec<&'a str>,
    states: Vec<&'a str>,
}

fn parse_selector(selector: &str) -> Option<Vec<CompoundSelector<'_>>> {
    let compounds: Option<Vec<_>> = selector
        .split_whitespace()
        .map(parse_compound_selector)
        .collect();
    compounds.filter(|parts| !parts.is_empty())
}

fn parse_compound_selector(selector: &str) -> Option<CompoundSelector<'_>> {
    if selector.is_empty() {
        return None;
    }
    let mut parsed = CompoundSelector {
        element: None,
        ids: Vec::new(),
        classes: Vec::new(),
        states: Vec::new(),
    };
    let mut cursor = 0usize;
    let first = selector.chars().next()?;
    if first == '*' {
        cursor = first.len_utf8();
    } else if !matches!(first, '.' | '#' | ':') {
        let (element, end) = parse_ident_at(selector, cursor)?;
        parsed.element = Some(element);
        cursor = end;
    }

    while cursor < selector.len() {
        let marker = selector[cursor..].chars().next()?;
        if !matches!(marker, '.' | '#' | ':') {
            // Attribute selectors, combinators, pseudo functions, and malformed
            // suffixes are unsupported and must never partially match.
            return None;
        }
        cursor += marker.len_utf8();
        let (ident, end) = parse_ident_at(selector, cursor)?;
        match marker {
            '.' => parsed.classes.push(ident),
            '#' => parsed.ids.push(ident),
            ':' => parsed.states.push(ident),
            _ => unreachable!(),
        }
        cursor = end;
    }
    Some(parsed)
}

fn parse_ident_at(input: &str, start: usize) -> Option<(&str, usize)> {
    let mut end = start;
    for (offset, ch) in input[start..].char_indices() {
        if ch.is_alphanumeric() || matches!(ch, '-' | '_') {
            end = start + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    (end > start).then(|| (&input[start..end], end))
}

/// CSS-like specificity tuple: `(ids, classes/states, element types)`.
fn specificity(selector: &str) -> (u16, u16, u16) {
    let Some(compounds) = parse_selector(selector) else {
        return (0, 0, 0);
    };
    compounds.into_iter().fold((0, 0, 0), |mut sum, part| {
        sum.0 = sum.0.saturating_add(part.ids.len() as u16);
        sum.1 = sum
            .1
            .saturating_add((part.classes.len() + part.states.len()) as u16);
        if part.element.is_some() {
            sum.2 = sum.2.saturating_add(1);
        }
        sum
    })
}

fn selector_matches(selector: &str, query: &StyleQuery) -> bool {
    let Some(compounds) = parse_selector(selector) else {
        return false;
    };
    let Some(current) = compounds.last() else {
        return false;
    };
    if !compound_matches(
        current,
        query.element.as_deref(),
        query.id.as_deref(),
        &query.classes,
        &query.states,
    ) {
        return false;
    }

    let mut ancestor_start = 0usize;
    for required in &compounds[..compounds.len() - 1] {
        let Some(relative) = query.ancestors[ancestor_start..].iter().position(|scope| {
            compound_matches(
                required,
                scope.element.as_deref(),
                scope.id.as_deref(),
                &scope.classes,
                &scope.states,
            )
        }) else {
            return false;
        };
        ancestor_start += relative + 1;
    }
    true
}

fn compound_matches(
    selector: &CompoundSelector<'_>,
    element: Option<&str>,
    id: Option<&str>,
    classes: &[String],
    states: &[String],
) -> bool {
    if let Some(required) = selector.element {
        if element != Some(required) {
            return false;
        }
    }
    if selector.ids.iter().any(|required| id != Some(*required)) {
        return false;
    }
    if selector
        .classes
        .iter()
        .any(|required| !classes.iter().any(|actual| actual == required))
    {
        return false;
    }
    !selector
        .states
        .iter()
        .any(|required| !states.iter().any(|actual| actual == required))
}

type MatchedRule<'a> = (usize, (u16, u16, u16), &'a StyleRule, &'a str);

/// Resolve computed style from sheet + query (cascade by specificity then order).
///
/// Custom properties (`--name`) are collected from `:root` and matched rules, then
/// `var(--name)` values are substituted recursively; missing references without
/// a fallback stay as [`StyleValue::Var`]. Only custom properties from `:root`
/// participate in element resolution.
pub fn resolve(sheet: &Stylesheet, query: &StyleQuery) -> ComputedStyle {
    let mut matched: Vec<MatchedRule<'_>> = Vec::new();
    for (order, rule) in sheet.rules.iter().enumerate() {
        for sel in &rule.selectors {
            let is_root = sel.trim() == ":root";
            if is_root || selector_matches(sel, query) {
                // :root always participates (for tokens); specificity stays low
                let spec = if is_root { (0, 0, 0) } else { specificity(sel) };
                matched.push((order, spec, rule, sel.as_str()));
            }
        }
    }
    matched.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    let mut props = IndexMap::new();
    for (_, _, rule, selector) in &matched {
        let root_tokens_only = selector.trim() == ":root";
        for (k, v) in &rule.declarations {
            if root_tokens_only && !k.starts_with("--") {
                continue;
            }
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
        resolved.insert(k, resolve_var_value(&v, &vars, &mut Vec::new(), 0));
    }
    ComputedStyle { props: resolved }
}

fn resolve_var_value(
    value: &StyleValue,
    vars: &IndexMap<String, StyleValue>,
    stack: &mut Vec<String>,
    depth: u8,
) -> StyleValue {
    if depth > 8 {
        return value.clone();
    }
    match value {
        StyleValue::Var(name) => resolve_named_var(name, None, vars, stack, depth),
        StyleValue::VarFallback { name, fallback } => {
            resolve_named_var(name, Some(fallback), vars, stack, depth)
        }
        other => other.clone(),
    }
}

fn resolve_named_var(
    name: &str,
    fallback: Option<&StyleValue>,
    vars: &IndexMap<String, StyleValue>,
    stack: &mut Vec<String>,
    depth: u8,
) -> StyleValue {
    let key = if name.starts_with("--") {
        name.to_string()
    } else {
        format!("--{name}")
    };
    let cyclic = stack.iter().any(|active| active == &key);
    if !cyclic {
        if let Some(value) = vars.get(&key) {
            stack.push(key.clone());
            let resolved = resolve_var_value(value, vars, stack, depth + 1);
            stack.pop();
            if !is_unresolved_var(&resolved) {
                return resolved;
            }
        }
    }
    if let Some(fallback) = fallback {
        return resolve_var_value(fallback, vars, stack, depth + 1);
    }
    StyleValue::Var(key)
}

fn is_unresolved_var(value: &StyleValue) -> bool {
    matches!(value, StyleValue::Var(_) | StyleValue::VarFallback { .. })
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
        style.props.entry(key).or_insert(StyleValue::Number(n));
    }
    // convenience x/y if missing
    let x_key = format!("{base}-x");
    let y_key = format!("{base}-y");
    style.props.entry(x_key).or_insert(StyleValue::Number(n));
    style.props.entry(y_key).or_insert(StyleValue::Number(n));
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
        let sel = resolve(&sheet, &StyleQuery::class("button").with_state("selected"));
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
    fn root_exports_only_custom_properties() {
        let sheet = parse_stylesheet(
            r#"
            :root { --gold: #ebc878; background: #ff0000; opacity: 0.1; }
            .button { color: var(--gold); }
            "#,
        )
        .unwrap();
        let computed = resolve(&sheet, &StyleQuery::class("button"));
        assert_eq!(computed.color_text().r, 0xeb);
        assert!(!computed.props.contains_key("background"));
        assert!(!computed.props.contains_key("opacity"));
    }

    #[test]
    fn var_fallback_resolves_missing_nested_and_cyclic_values() {
        let sheet = parse_stylesheet(
            r#"
            :root {
              --cycle-a: var(--cycle-b);
              --cycle-b: var(--cycle-a);
            }
            .button {
              color: var(--missing, var(--also-missing, #123456));
              border-color: var(--cycle-a, #654321);
              width: var(--missing-width, 42);
              height: var(--still-missing);
            }
            "#,
        )
        .unwrap();
        let computed = resolve(&sheet, &StyleQuery::class("button"));
        assert_eq!(computed.color_text(), Color::rgb(0x12, 0x34, 0x56));
        assert_eq!(computed.border_color(), Color::rgb(0x65, 0x43, 0x21));
        assert_eq!(computed.number("width", 0.0), 42.0);
        assert_eq!(
            computed.props.get("height"),
            Some(&StyleValue::Var("--still-missing".into()))
        );
    }

    #[test]
    fn compound_selector_uses_full_specificity() {
        let sheet = parse_stylesheet(
            r#"
            #cta.button:hover { color: #ff0000; }
            #cta { color: #0000ff; }
            "#,
        )
        .unwrap();
        let query = StyleQuery::class("button")
            .with_id("cta")
            .with_state("hover");
        assert_eq!(resolve(&sheet, &query).color_text(), Color::rgb(255, 0, 0));
    }

    #[test]
    fn descendant_selectors_require_ordered_ancestor_context() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            .menu .button { color: #ff0000; }
            .shell .panel .button:selected { background: #123456; }
            "#,
        )
        .unwrap();
        let plain = StyleQuery::class("button").with_state("selected");
        assert_eq!(resolve(&sheet, &plain).color_text().r, 0xaa);

        let contextual = plain
            .clone()
            .with_ancestor_class("shell")
            .with_ancestor_class("menu")
            .with_ancestor_class("panel");
        let computed = resolve(&sheet, &contextual);
        assert_eq!(computed.color_text(), Color::rgb(255, 0, 0));
        assert_eq!(computed.background(), Color::rgb(0x12, 0x34, 0x56));

        let wrong_order = plain
            .with_ancestor_class("panel")
            .with_ancestor_class("shell");
        assert!(!resolve(&sheet, &wrong_order)
            .props
            .contains_key("background"));
    }

    #[test]
    fn unsupported_selectors_never_partially_match() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            .menu > .button { color: #ff0000; }
            .button[disabled] { color: #00ff00; }
            button.button { background: #123456; }
            "#,
        )
        .unwrap();
        let scoped = StyleQuery::class("button").with_ancestor_class("menu");
        let computed = resolve(&sheet, &scoped);
        assert_eq!(computed.color_text().r, 0xaa);
        assert!(!computed.props.contains_key("background"));

        let typed = scoped.with_element("button");
        assert_eq!(
            resolve(&sheet, &typed).background(),
            Color::rgb(0x12, 0x34, 0x56)
        );
    }

    #[test]
    fn margin_shorthand_expands() {
        let sheet = parse_stylesheet(".box { margin: 8; width: 100; }").unwrap();
        let c = resolve_expanded(&sheet, &StyleQuery::class("box"));
        assert_eq!(c.number("margin-top", 0.0), 8.0);
        assert_eq!(c.number("margin-x", 0.0), 8.0);
        assert_eq!(c.number("width", 0.0), 100.0);
    }

    #[test]
    fn hover_and_disabled_states() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            .button:hover { color: #ffffff; }
            .button:disabled { color: #444444; opacity: 0.4; }
            "#,
        )
        .unwrap();
        let hover = resolve(&sheet, &StyleQuery::class("button").with_state("hover"));
        assert_eq!(hover.color_text().r, 0xff);
        let dis = resolve(&sheet, &StyleQuery::class("button").with_state("disabled"));
        assert_eq!(dis.color_text().r, 0x44);
        assert!((dis.number("opacity", 1.0) - 0.4).abs() < 1e-4);
    }
}
