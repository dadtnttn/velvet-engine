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
    /// User prefers reduced motion (`(prefers-reduced-motion)`).
    pub reduced_motion: bool,
    /// User prefers more contrast (`(prefers-contrast: more)`).
    pub high_contrast: bool,
    /// Current orientation (`(orientation: …)`).
    pub orientation: Option<crate::parse::MediaOrientation>,
    /// Viewport width in px (`(min-width: …)` / `(max-width: …)`).
    pub viewport_width: Option<f32>,
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

    /// Flag reduced motion for `@media` evaluation.
    pub fn with_reduced_motion(mut self, on: bool) -> Self {
        self.reduced_motion = on;
        self
    }

    /// Flag high contrast for `@media` evaluation.
    pub fn with_high_contrast(mut self, on: bool) -> Self {
        self.high_contrast = on;
        self
    }

    /// Set orientation for `@media` evaluation.
    pub fn with_orientation(mut self, orientation: crate::parse::MediaOrientation) -> Self {
        self.orientation = Some(orientation);
        self
    }

    /// Set viewport width in px for `@media` evaluation.
    pub fn with_viewport_width(mut self, width: f32) -> Self {
        self.viewport_width = Some(width);
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

#[derive(Debug)]
struct CompoundSelector<'a> {
    element: Option<&'a str>,
    ids: Vec<&'a str>,
    classes: Vec<&'a str>,
    states: Vec<&'a str>,
    /// `:not(X)` arguments (simple compounds, never nested functions).
    nots: Vec<CompoundSelector<'a>>,
    /// `:is(A, …)` alternatives (at least one must match when non-empty).
    is_groups: Vec<CompoundSelector<'a>>,
}

/// One selector compound plus how it connects to its predecessor:
/// `false` = descendant (space), `true` = direct child (`>`).
type SelectorStep<'a> = (bool, CompoundSelector<'a>);

/// Split a selector into compounds and `>` tokens. Whitespace inside
/// `(…)` never splits (`:is(.a, .b)` stays one token); a bare `>` splits
/// even without surrounding spaces (`A>B`).
fn split_selector_tokens(selector: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut depth = 0u32;
    let mut start: Option<usize> = None;
    let bytes = selector.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'(' => {
                depth += 1;
                if start.is_none() {
                    start = Some(i);
                }
                i += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                if start.is_none() {
                    start = Some(i);
                }
                i += 1;
            }
            _ if c.is_ascii_whitespace() && depth == 0 => {
                if let Some(s) = start.take() {
                    tokens.push(&selector[s..i]);
                }
                i += 1;
            }
            b'>' if depth == 0 => {
                if let Some(s) = start.take() {
                    tokens.push(&selector[s..i]);
                }
                tokens.push(&selector[i..i + 1]);
                i += 1;
            }
            _ => {
                if start.is_none() {
                    start = Some(i);
                }
                i += 1;
            }
        }
    }
    if let Some(s) = start {
        tokens.push(&selector[s..]);
    }
    tokens
}

fn parse_selector(selector: &str) -> Option<Vec<SelectorStep<'_>>> {
    let mut steps = Vec::new();
    let mut child = false;
    for token in split_selector_tokens(selector) {
        if token == ">" {
            // Leading, trailing, or doubled `>` never matches.
            if steps.is_empty() || child {
                return None;
            }
            child = true;
            continue;
        }
        steps.push((child, parse_compound_selector(token)?));
        child = false;
    }
    if steps.is_empty() || child {
        return None;
    }
    Some(steps)
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
        nots: Vec::new(),
        is_groups: Vec::new(),
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
            // Attribute selectors, combinators, and malformed suffixes are
            // unsupported and must never partially match.
            return None;
        }
        cursor += marker.len_utf8();
        let (ident, end) = parse_ident_at(selector, cursor)?;
        cursor = end;
        match marker {
            '.' => parsed.classes.push(ident),
            '#' => parsed.ids.push(ident),
            ':' if ident == "not" || ident == "is" => {
                let func = parse_pseudo_function(selector, &mut cursor, ident)?;
                match func {
                    (true, groups) => parsed.is_groups.extend(groups),
                    (false, groups) => parsed.nots.extend(groups),
                }
            }
            ':' => parsed.states.push(ident),
            _ => unreachable!(),
        }
    }
    Some(parsed)
}

/// Parse `(A, B, …)` after `:not`/`:is` at `cursor` (which must sit on
/// `(`). Returns (is_is, groups). Only simple compounds inside: no
/// combinators, no nested functions, no empty groups.
fn parse_pseudo_function<'a>(
    selector: &'a str,
    cursor: &mut usize,
    ident: &str,
) -> Option<(bool, Vec<CompoundSelector<'a>>)> {
    let open = selector[*cursor..].find('(')?;
    if !selector[*cursor..*cursor + open].trim().is_empty() {
        return None;
    }
    *cursor += open + 1;
    let mut groups = Vec::new();
    loop {
        let rest = &selector[*cursor..];
        let end = rest.find([',', ')'])?;
        let arg = rest[..end].trim();
        if arg.is_empty() {
            return None;
        }
        let group = parse_simple_compound(arg)?;
        groups.push(group);
        *cursor += end;
        let closer = selector[*cursor..].chars().next()?;
        *cursor += closer.len_utf8();
        if closer == ')' {
            break;
        }
    }
    if groups.is_empty() {
        return None;
    }
    Some((ident == "is", groups))
}

/// Simple compound without pseudo functions (for `:not`/`:is` args).
fn parse_simple_compound(selector: &str) -> Option<CompoundSelector<'_>> {
    let mut parsed = CompoundSelector {
        element: None,
        ids: Vec::new(),
        classes: Vec::new(),
        states: Vec::new(),
        nots: Vec::new(),
        is_groups: Vec::new(),
    };
    if selector.is_empty() {
        return None;
    }
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
            return None;
        }
        cursor += marker.len_utf8();
        let (ident, end) = parse_ident_at(selector, cursor)?;
        cursor = end;
        match marker {
            '.' => parsed.classes.push(ident),
            '#' => parsed.ids.push(ident),
            ':' => {
                // Nested :not()/:is() and other functions stay unsupported.
                if selector[cursor..].starts_with('(') {
                    return None;
                }
                parsed.states.push(ident);
            }
            _ => unreachable!(),
        }
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

/// Specificity of one compound: base plus `:not()` args, plus the most
/// specific `:is()` alternative (CSS rule; `:is()` itself adds nothing).
fn compound_specificity(part: &CompoundSelector) -> (u16, u16, u16) {
    let mut sum = (
        part.ids.len() as u16,
        (part.classes.len() + part.states.len()) as u16,
        u16::from(part.element.is_some()),
    );
    for not in &part.nots {
        let (a, b, c) = compound_specificity(not);
        sum.0 = sum.0.saturating_add(a);
        sum.1 = sum.1.saturating_add(b);
        sum.2 = sum.2.saturating_add(c);
    }
    let mut best = (0, 0, 0);
    for group in &part.is_groups {
        best = best.max(compound_specificity(group));
    }
    (
        sum.0.saturating_add(best.0),
        sum.1.saturating_add(best.1),
        sum.2.saturating_add(best.2),
    )
}

/// CSS-like specificity tuple: `(ids, classes/states, element types)`.
/// Combinators (`>` vs space) contribute nothing, like CSS.
fn specificity(selector: &str) -> (u16, u16, u16) {
    let Some(compounds) = parse_selector(selector) else {
        return (0, 0, 0);
    };
    compounds.into_iter().fold((0, 0, 0), |mut sum, (_, part)| {
        let (a, b, c) = compound_specificity(&part);
        sum.0 = sum.0.saturating_add(a);
        sum.1 = sum.1.saturating_add(b);
        sum.2 = sum.2.saturating_add(c);
        sum
    })
}

fn selector_matches(selector: &str, query: &StyleQuery) -> bool {
    let Some(compounds) = parse_selector(selector) else {
        return false;
    };
    let Some((_, current)) = compounds.last() else {
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

    // Walk ancestors root-to-parent for descendants; a child step must
    // match the unconsumed tail (i.e. the direct parent chain end).
    // Process right-to-left: each step consumes ancestors from the end.
    // The combinator flag lives on the right-hand compound (`A > B`
    // stores `child` on B), so read it from the right neighbor.
    let mut tail = query.ancestors.len();
    for i in (0..compounds.len() - 1).rev() {
        let (child, required) = (&compounds[i + 1].0, &compounds[i].1);
        if *child {
            // Direct parent only.
            if tail == 0 {
                return false;
            }
            let scope = &query.ancestors[tail - 1];
            if !compound_matches(
                required,
                scope.element.as_deref(),
                scope.id.as_deref(),
                &scope.classes,
                &scope.states,
            ) {
                return false;
            }
            tail -= 1;
        } else {
            // Nearest matching ancestor at or before the tail.
            let found = query.ancestors[..tail].iter().rposition(|scope| {
                compound_matches(
                    required,
                    scope.element.as_deref(),
                    scope.id.as_deref(),
                    &scope.classes,
                    &scope.states,
                )
            });
            match found {
                Some(i) => tail = i,
                None => return false,
            }
        }
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
    if selector
        .states
        .iter()
        .any(|required| !states.iter().any(|actual| actual == required))
    {
        return false;
    }
    // :not(X) vetoes when any argument matches. Args never nest
    // (rejected at parse), so the same matcher applies directly.
    if selector.nots.iter().any(|not| {
        debug_assert!(not.nots.is_empty() && not.is_groups.is_empty());
        compound_matches(not, element, id, classes, states)
    }) {
        return false;
    }
    // :is(A, …) needs one alternative (vacuous when absent).
    if !selector.is_groups.is_empty()
        && !selector.is_groups.iter().any(|group| {
            debug_assert!(group.nots.is_empty() && group.is_groups.is_empty());
            compound_matches(group, element, id, classes, states)
        })
    {
        return false;
    }
    true
}

/// One rule that matched a query, in cascade order (specificity, then source).
struct CollectedMatch<'a> {
    /// Source order across base + matching `@media` rules.
    order: usize,
    /// Selector specificity (`:root` counts as zero).
    specificity: (u16, u16, u16),
    /// The matched rule.
    rule: &'a StyleRule,
    /// The matched selector text.
    selector: &'a str,
    /// `@media` condition text when the rule came from a matching block.
    media: Option<&'a str>,
}

/// Collect matching rules sorted for the cascade (specificity, then source order).
fn collect_matches<'a>(sheet: &'a Stylesheet, query: &'a StyleQuery) -> Vec<CollectedMatch<'a>> {
    let media_ctx = crate::parse::MediaContext {
        reduced_motion: query.reduced_motion,
        high_contrast: query.high_contrast,
        orientation: query.orientation,
        viewport_width: query.viewport_width,
    };
    let mut matched = Vec::new();
    let mut order = 0usize;
    // Base rules first, then matching @media blocks — all in source order
    // so later rules win specificity ties across the boundary.
    let mut all_rules: Vec<(&StyleRule, Option<&str>)> =
        sheet.rules.iter().map(|r| (r, None)).collect();
    for block in &sheet.media {
        if block.query.matches(&media_ctx) {
            all_rules.extend(
                block
                    .rules
                    .iter()
                    .map(|r| (r, Some(block.query_text.as_str()))),
            );
        }
    }
    for (rule, media) in all_rules {
        for sel in &rule.selectors {
            let is_root = sel.trim() == ":root";
            if is_root || selector_matches(sel, query) {
                // :root always participates (for tokens); specificity stays low
                let spec = if is_root { (0, 0, 0) } else { specificity(sel) };
                matched.push(CollectedMatch {
                    order,
                    specificity: spec,
                    rule,
                    selector: sel.as_str(),
                    media,
                });
            }
        }
        order += 1;
    }
    matched.sort_by(|a, b| {
        a.specificity
            .cmp(&b.specificity)
            .then(a.order.cmp(&b.order))
    });
    matched
}

/// One cascade step for [`explain_cascade`].
#[derive(Debug, Clone, PartialEq)]
pub struct ExplainStep {
    /// Matched selector text.
    pub selector: String,
    /// Selector specificity.
    pub specificity: (u16, u16, u16),
    /// Source order across base + matching `@media` rules.
    pub order: usize,
    /// `@media` condition text when the rule came from a matching block.
    pub media: Option<String>,
    /// Value this rule declares for the property (None when it does not).
    pub declared: Option<StyleValue>,
    /// Whether this step wins the cascade for the property.
    pub wins: bool,
}

/// Trace the cascade for one property: every matched rule in cascade order
/// with the value it declares, the winner flagged, plus the final resolved
/// value (after `var()` substitution and calc folding).
///
/// With ancestor context the resolution is inherited ([`resolve_inherited`]):
/// when no own rule declares the property but an ancestor provides it, a
/// synthetic `<inherited from …>` step (order `usize::MAX`, weakest origin)
/// opens the trace and wins.
pub fn explain_cascade(
    sheet: &Stylesheet,
    query: &StyleQuery,
    prop: &str,
) -> (Vec<ExplainStep>, Option<StyleValue>) {
    let matched = collect_matches(sheet, query);
    let is_custom = prop.starts_with("--");
    let mut steps = Vec::with_capacity(matched.len());
    let mut winner: Option<usize> = None;
    for m in &matched {
        let root_tokens_only = m.selector.trim() == ":root";
        let declared = if root_tokens_only && !is_custom {
            None
        } else {
            m.rule.declarations.get(prop).cloned()
        };
        if declared.is_some() {
            winner = Some(steps.len());
        }
        steps.push(ExplainStep {
            selector: m.selector.to_string(),
            specificity: m.specificity,
            order: m.order,
            media: m.media.map(str::to_string),
            declared,
            wins: false,
        });
    }
    if let Some(w) = winner {
        steps[w].wins = true;
    }
    let resolved = resolve_inherited(sheet, query).props.get(prop).cloned();
    // Inherited winner: nearest ancestor providing the property.
    if winner.is_none() && resolved.is_some() && !query.ancestors.is_empty() {
        for i in (0..query.ancestors.len()).rev() {
            let scope = &query.ancestors[i];
            let ancestor_query = StyleQuery {
                element: scope.element.clone(),
                id: scope.id.clone(),
                classes: scope.classes.clone(),
                states: scope.states.clone(),
                ancestors: query.ancestors[..i].to_vec(),
                reduced_motion: query.reduced_motion,
                high_contrast: query.high_contrast,
                orientation: query.orientation,
                viewport_width: query.viewport_width,
            };
            let computed = resolve_inherited(sheet, &ancestor_query);
            if let Some(declared) = computed.props.get(prop).cloned() {
                steps.insert(
                    0,
                    ExplainStep {
                        selector: format!("<inherited from {}>", describe_scope(scope)),
                        specificity: (0, 0, 0),
                        order: usize::MAX,
                        media: None,
                        declared: Some(declared),
                        wins: true,
                    },
                );
                break;
            }
        }
    }
    (steps, resolved)
}

/// Short `button#id.class:state` description of an ancestor scope.
fn describe_scope(scope: &StyleScope) -> String {
    let mut s = scope.element.clone().unwrap_or_default();
    if let Some(id) = &scope.id {
        s.push('#');
        s.push_str(id);
    }
    for c in &scope.classes {
        s.push('.');
        s.push_str(c);
    }
    for st in &scope.states {
        s.push(':');
        s.push_str(st);
    }
    if s.is_empty() {
        s.push_str("<node>");
    }
    s
}

/// Properties that inherit the parent computed value when the element
/// does not declare them (CSS semantics; custom properties always inherit).
fn is_inheritable(prop: &str) -> bool {
    prop.starts_with("--")
        || matches!(
            prop,
            "color" | "font-family" | "font-size" | "font-weight" | "letter-spacing" | "text-align"
        )
}

/// Resolve computed style from sheet + query (cascade by specificity then order).
///
/// Custom properties (`--name`) are collected from `:root` and matched rules, then
/// `var(--name)` values are substituted recursively; missing references without
/// a fallback stay as [`StyleValue::Var`]. Only custom properties from `:root`
/// participate in element resolution.
pub fn resolve(sheet: &Stylesheet, query: &StyleQuery) -> ComputedStyle {
    resolve_with(sheet, query, &IndexMap::new())
}

/// `resolve` with ancestor custom properties seeded at the lowest priority
/// (nearer real ancestors beat `:root`, which beats nothing here).
fn resolve_with(
    sheet: &Stylesheet,
    query: &StyleQuery,
    inherited_vars: &IndexMap<String, StyleValue>,
) -> ComputedStyle {
    let matched = collect_matches(sheet, query);
    // With an inherited seed, `:root` customs stop acting as own
    // declarations (the seed carries the nearer ancestors' values); the
    // fill step in `resolve_inherited` re-adds them as inherited.
    // Without a seed this loop is exactly the old one.
    let has_seed = !inherited_vars.is_empty();
    let mut props = IndexMap::new();
    for m in &matched {
        let is_root = m.selector.trim() == ":root";
        for (k, v) in &m.rule.declarations {
            if is_root && !k.starts_with("--") {
                continue;
            }
            if is_root && has_seed && k.starts_with("--") {
                continue;
            }
            props.insert(k.clone(), v.clone());
        }
    }
    // Custom property map, weakest first: `:root` (farthest ancestor),
    // then the inherited seed (nearer ancestors, nearest already won),
    // then own matched rules in cascade order (direct declarations,
    // including `*`, always beat inherited values).
    //
    // Note: with an empty seed this matches the old collection except when
    // `*` and `:root` define the same custom with `*` first in source —
    // now `*` wins, which is the CSS-correct outcome (`*` matches the
    // element directly; `:root` only inherits).
    let mut vars = IndexMap::new();
    for m in &matched {
        if m.selector.trim() != ":root" {
            continue;
        }
        for (k, v) in &m.rule.declarations {
            if k.starts_with("--") {
                vars.insert(k.clone(), v.clone());
            }
        }
    }
    for (k, v) in inherited_vars {
        vars.insert(k.clone(), v.clone());
    }
    for m in &matched {
        if m.selector.trim() == ":root" {
            continue;
        }
        for (k, v) in &m.rule.declarations {
            if k.starts_with("--") {
                vars.insert(k.clone(), v.clone());
            }
        }
    }
    // Substitute var(), then fold fully-concrete calc() math so computed
    // styles carry plain values (percent math stays for layout hosts).
    let mut resolved = IndexMap::new();
    for (k, v) in props {
        let mut value = resolve_var_value(&v, &vars, &mut Vec::new(), 0);
        crate::value::eval_calc_in_place(&mut value);
        resolved.insert(k, value);
    }
    // `currentcolor`: whole-value keyword resolving against the element's
    // own `color` (only when that is a concrete color; `color:
    // currentcolor` itself stays unresolved, like browsers' canvastext
    // fallback would need a host default for).
    if let Some(StyleValue::Color(text)) = resolved.get("color").cloned() {
        for (k, v) in resolved.iter_mut() {
            if k != "color"
                && matches!(v, StyleValue::Keyword(s) if s.eq_ignore_ascii_case("currentcolor"))
            {
                *v = StyleValue::Color(text);
            }
        }
    }
    ComputedStyle { props: resolved }
}

/// Resolve with CSS inheritance: inheritable properties (`color`,
/// `font-*`, `letter-spacing`, `text-align`, all `--*`) fall back to the
/// nearest ancestor's computed value, and `inherit` / `initial` / `unset`
/// keywords are honored.
///
/// Ancestors come from `query.ancestors` (root-to-parent); each ancestor
/// resolves with its own prefix as context, so grandparent tokens flow
/// through. Queries without ancestors behave exactly like [`resolve`].
pub fn resolve_inherited(sheet: &Stylesheet, query: &StyleQuery) -> ComputedStyle {
    // Nearest ancestor wins per property (root-to-parent order so nearer
    // levels overwrite). `parent_all` feeds explicit `inherit`; `base`
    // keeps the inheritable subset for implicit flow + `unset`.
    let mut parent_all: IndexMap<String, StyleValue> = IndexMap::new();
    for i in 0..query.ancestors.len() {
        let scope = &query.ancestors[i];
        let ancestor_query = StyleQuery {
            element: scope.element.clone(),
            id: scope.id.clone(),
            classes: scope.classes.clone(),
            states: scope.states.clone(),
            ancestors: query.ancestors[..i].to_vec(),
            reduced_motion: query.reduced_motion,
            high_contrast: query.high_contrast,
            orientation: query.orientation,
            viewport_width: query.viewport_width,
        };
        let computed = resolve_inherited(sheet, &ancestor_query);
        for (k, v) in computed.props {
            parent_all.insert(k, v);
        }
    }
    let base: IndexMap<String, StyleValue> = parent_all
        .iter()
        .filter(|(k, _)| is_inheritable(k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let inherited_vars: IndexMap<String, StyleValue> = base
        .iter()
        .filter(|(k, _)| k.starts_with("--"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut own = resolve_with(sheet, query, &inherited_vars);
    // Fill undeclared inheritable properties from the parent chain.
    for (k, v) in &base {
        if is_inheritable(k) {
            own.props.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    // `inherit` / `initial` / `unset` (whole-value keywords only).
    let keywords: Vec<(String, String)> = own
        .props
        .iter()
        .filter_map(|(k, v)| match v {
            StyleValue::Keyword(s)
                if s.eq_ignore_ascii_case("inherit")
                    || s.eq_ignore_ascii_case("initial")
                    || s.eq_ignore_ascii_case("unset") =>
            {
                Some((k.clone(), s.to_ascii_lowercase()))
            }
            _ => None,
        })
        .collect();
    for (k, word) in keywords {
        match word.as_str() {
            // Parent computed value (any property); absent = host default.
            "inherit" => match parent_all.get(&k).cloned() {
                Some(v) => {
                    own.props.insert(k, v);
                }
                None => {
                    own.props.shift_remove(&k);
                }
            },
            // Host default: as if not declared.
            "initial" => {
                own.props.shift_remove(&k);
            }
            // Inherited when inheritable, else host default.
            _ => {
                if is_inheritable(&k) {
                    match base.get(&k).cloned() {
                        Some(v) => {
                            own.props.insert(k, v);
                        }
                        None => {
                            own.props.shift_remove(&k);
                        }
                    }
                } else {
                    own.props.shift_remove(&k);
                }
            }
        }
    }
    own
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
        StyleValue::Calc(expr) => {
            // Substitute var() leaves first so calc math can fold.
            let vars_map: std::collections::HashMap<String, StyleValue> =
                vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            StyleValue::Calc(crate::value::subst_calc_vars(expr, &vars_map))
        }
        StyleValue::ColorMix { a, wa, b, wb } => {
            let ra = resolve_var_value(a, vars, stack, depth + 1);
            let rb = resolve_var_value(b, vars, stack, depth + 1);
            match (&ra, &rb) {
                (StyleValue::Color(ca), StyleValue::Color(cb)) => {
                    StyleValue::Color(crate::value::mix_concrete(*ca, *wa, *cb, *wb))
                }
                _ => StyleValue::ColorMix {
                    a: Box::new(ra),
                    wa: *wa,
                    b: Box::new(rb),
                    wb: *wb,
                },
            }
        }
        StyleValue::Alpha { color, amount } => {
            match resolve_var_value(color, vars, stack, depth + 1) {
                StyleValue::Color(mut c) => {
                    c.a = (*amount).clamp(0.0, 1.0);
                    StyleValue::Color(c)
                }
                rest => StyleValue::Alpha {
                    color: Box::new(rest),
                    amount: *amount,
                },
            }
        }
        StyleValue::AutoContrast { base, dark, light } => {
            let rb = resolve_var_value(base, vars, stack, depth + 1);
            let rd = resolve_var_value(dark, vars, stack, depth + 1);
            let rl = resolve_var_value(light, vars, stack, depth + 1);
            match (&rb, &rd, &rl) {
                (StyleValue::Color(b), StyleValue::Color(d), StyleValue::Color(l)) => {
                    StyleValue::Color(crate::value::contrast_pick(*b, *d, *l))
                }
                _ => StyleValue::AutoContrast {
                    base: Box::new(rb),
                    dark: Box::new(rd),
                    light: Box::new(rl),
                },
            }
        }
        StyleValue::Gradient(g) => {
            let mut g = g.clone();
            for stop in &mut g.stops {
                stop.color = resolve_var_value(&stop.color, vars, stack, depth + 1);
            }
            StyleValue::Gradient(g)
        }
        StyleValue::Shadow(layers) => {
            let layers = layers
                .iter()
                .map(|s| {
                    let mut s = s.clone();
                    s.color = resolve_var_value(&s.color, vars, stack, depth + 1);
                    s
                })
                .collect();
            StyleValue::Shadow(layers)
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
    fn vars_inside_functions_fold_in_resolve() {
        let sheet = parse_stylesheet(
            r#"
            :root {
              --gold: #ebc878;
              --pad: 8px;
            }
            .button {
              color: color-mix(in srgb, var(--gold) 70%, #501e78);
              brand: alpha(var(--gold), 0.5);
              width: calc(var(--pad) + 4px);
              height: min(var(--missing, 6px), 20px);
              background: linear-gradient(180deg, var(--gold), #501e78);
              box-shadow: 0 2px 8px var(--gold);
            }
            "#,
        )
        .unwrap();
        let c = resolve(&sheet, &StyleQuery::class("button"));
        // Identical to authoring the concrete mix by hand.
        assert_eq!(
            c.props.get("color"),
            Some(&StyleValue::Color(
                crate::value::parse_color("color-mix(in srgb, #ebc878 70%, #501e78)").unwrap()
            ))
        );
        assert_eq!(
            c.props.get("brand"),
            Some(&StyleValue::Color(
                crate::value::parse_color("alpha(#ebc878, 0.5)").unwrap()
            ))
        );
        assert_eq!(c.props.get("width"), Some(&StyleValue::Length(12.0)));
        assert_eq!(c.props.get("height"), Some(&StyleValue::Length(6.0)));
        let Some(StyleValue::Gradient(g)) = c.props.get("background") else {
            panic!("expected gradient, got {:?}", c.props.get("background"));
        };
        assert_eq!(g.sample(0.0), Some(Color::rgb(0xeb, 0xc8, 0x78)));
        assert_eq!(
            g.sample(1.0),
            Some(crate::value::parse_color("#501e78").unwrap())
        );
        let Some(StyleValue::Shadow(layers)) = c.props.get("box-shadow") else {
            panic!("expected shadow, got {:?}", c.props.get("box-shadow"));
        };
        assert_eq!(layers.len(), 1);
        assert_eq!(
            layers[0].color,
            StyleValue::Color(Color::rgb(0xeb, 0xc8, 0x78))
        );
    }

    #[test]
    fn unresolved_vars_inside_functions_stay_deferred() {
        let sheet = parse_stylesheet(
            r#"
            .button {
              color: color-mix(in srgb, var(--nope) 70%, #501e78);
              width: calc(var(--nope) + 4px);
            }
            "#,
        )
        .unwrap();
        let c = resolve(&sheet, &StyleQuery::class("button"));
        // Hosts skip deferred values (same as an unknown Keyword before).
        assert!(matches!(
            c.props.get("color"),
            Some(StyleValue::ColorMix { .. })
        ));
        assert!(matches!(c.props.get("width"), Some(StyleValue::Calc(_))));
    }

    #[test]
    fn explain_traces_winner_and_losers() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; background: #000000; }
            .menu .button { color: #bbbbbb; }
            .button.primary:hover { color: #ff0000; }
            @media (prefers-reduced-motion) {
              .button { color: #00ff00; }
            }
            "#,
        )
        .unwrap();
        let mut query = StyleQuery::class("button")
            .with_class("primary")
            .with_state("hover")
            .with_ancestor(StyleScope::class("menu"));
        query.reduced_motion = true;
        let (steps, resolved) = explain_cascade(&sheet, &query, "color");
        // Cascade order: (0,1,0) base, (0,1,0) media (later order wins the
        // tie), (0,2,0) descendant, (0,3,0) hover.
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].selector, ".button");
        assert_eq!(
            steps[0].declared,
            Some(StyleValue::Color(Color::rgb(0xaa, 0xaa, 0xaa)))
        );
        assert!(!steps[0].wins);
        assert_eq!(steps[1].media.as_deref(), Some("(prefers-reduced-motion)"));
        assert!(!steps[1].wins);
        assert_eq!(steps[2].selector, ".menu .button");
        assert!(!steps[2].wins);
        assert_eq!(steps[3].selector, ".button.primary:hover");
        assert!(steps[3].wins);
        assert_eq!(resolved, Some(StyleValue::Color(Color::rgb(255, 0, 0))));
        // A property nobody sets has no winner and no resolved value.
        let (steps, resolved) = explain_cascade(&sheet, &query, "border-color");
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().all(|s| !s.wins && s.declared.is_none()));
        assert_eq!(resolved, None);
    }

    #[test]
    fn explain_shows_inherited_winner() {
        let sheet = parse_stylesheet(
            r#"
            .menu { color: #ebc878; }
            .button { width: 10; }
            "#,
        )
        .unwrap();
        let query = StyleQuery::class("button").with_ancestor(StyleScope::class("menu"));
        let (steps, resolved) = explain_cascade(&sheet, &query, "color");
        assert_eq!(steps.len(), 2);
        assert!(steps[0].selector.contains(".menu"), "{steps:?}");
        assert_eq!(steps[0].order, usize::MAX);
        assert!(steps[0].wins);
        assert_eq!(
            steps[0].declared,
            Some(StyleValue::Color(Color::rgb(0xeb, 0xc8, 0x78)))
        );
        assert!(!steps[1].wins);
        assert_eq!(
            resolved,
            Some(StyleValue::Color(Color::rgb(0xeb, 0xc8, 0x78)))
        );
    }

    #[test]
    fn explain_winner_follows_cascade_not_source_order() {
        let sheet = parse_stylesheet(
            r#"
            .button.primary:hover { color: #ff0000; }
            .button { color: #aaaaaa; }
            "#,
        )
        .unwrap();
        let query = StyleQuery::class("button")
            .with_class("primary")
            .with_state("hover");
        let (steps, resolved) = explain_cascade(&sheet, &query, "color");
        assert_eq!(steps[0].selector, ".button");
        assert!(steps[1].wins);
        assert_eq!(resolved, Some(StyleValue::Color(Color::rgb(255, 0, 0))));
    }

    #[test]
    fn currentcolor_and_auto_contrast_resolve() {
        let sheet = parse_stylesheet(
            r#"
            :root { --bg: #ffffff; }
            .card {
              color: #ebc878;
              border-color: currentcolor;
              background: currentcolor;
              glow: auto-contrast(#501e78);
              veil: auto-contrast(var(--bg), #111111, #eeeeee);
            }
            .ghost { color: currentcolor; }
            "#,
        )
        .unwrap();
        let c = resolve(&sheet, &StyleQuery::class("card"));
        let gold = Color::rgb(0xeb, 0xc8, 0x78);
        assert_eq!(c.props.get("border-color"), Some(&StyleValue::Color(gold)));
        assert_eq!(c.props.get("background"), Some(&StyleValue::Color(gold)));
        // Dark plum gets white text.
        assert_eq!(
            c.props.get("glow"),
            Some(&StyleValue::Color(Color::rgb(255, 255, 255)))
        );
        // Deferred contrast folds against the defined var.
        assert_eq!(
            c.props.get("veil"),
            Some(&StyleValue::Color(Color::rgb(0x11, 0x11, 0x11)))
        );
        // `color: currentcolor` has no text color to inherit: stays put.
        let g = resolve(&sheet, &StyleQuery::class("ghost"));
        assert_eq!(
            g.props.get("color"),
            Some(&StyleValue::Keyword("currentcolor".into()))
        );
    }

    #[test]
    fn inherited_text_props_flow_from_parent() {
        let sheet = parse_stylesheet(
            r#"
            .menu { color: #ebc878; font-size: 20; --accent: #ff0000; opacity: 0.5; }
            .button { font-size: 14; border-color: var(--accent); }
            "#,
        )
        .unwrap();
        let query = StyleQuery::class("button").with_ancestor(StyleScope::class("menu"));
        let c = resolve_inherited(&sheet, &query);
        // Inherited…
        assert_eq!(
            c.props.get("color"),
            Some(&StyleValue::Color(Color::rgb(0xeb, 0xc8, 0x78)))
        );
        // …own declarations win…
        assert_eq!(c.props.get("font-size"), Some(&StyleValue::Number(14.0)));
        // …custom properties inherit AND feed var()…
        assert_eq!(
            c.props.get("--accent"),
            Some(&StyleValue::Color(Color::rgb(255, 0, 0)))
        );
        assert_eq!(
            c.props.get("border-color"),
            Some(&StyleValue::Color(Color::rgb(255, 0, 0)))
        );
        // …and non-inheritable properties never flow.
        assert!(!c.props.contains_key("opacity"));
    }

    #[test]
    fn inherit_initial_unset_keywords() {
        let sheet = parse_stylesheet(
            r#"
            .menu { color: #ebc878; opacity: 0.5; }
            .a { color: inherit; }
            .b { opacity: inherit; }
            .c { color: initial; }
            .d { opacity: unset; }
            .e { color: unset; }
            .f { color: inherit; }
            "#,
        )
        .unwrap();
        let with_parent = |class: &str| {
            resolve_inherited(
                &sheet,
                &StyleQuery::class(class).with_ancestor(StyleScope::class("menu")),
            )
        };
        let gold = StyleValue::Color(Color::rgb(0xeb, 0xc8, 0x78));
        assert_eq!(with_parent("a").props.get("color"), Some(&gold));
        // `inherit` forces even non-inheritable properties.
        assert_eq!(
            with_parent("b").props.get("opacity"),
            Some(&StyleValue::Number(0.5))
        );
        assert!(!with_parent("c").props.contains_key("color"));
        assert!(!with_parent("d").props.contains_key("opacity"));
        assert_eq!(with_parent("e").props.get("color"), Some(&gold));
        // No parent value: `inherit` falls back to absent (host default).
        let orphan = resolve_inherited(&sheet, &StyleQuery::class("f"));
        assert!(!orphan.props.contains_key("color"));
    }

    #[test]
    fn nearer_ancestor_beats_root() {
        let sheet = parse_stylesheet(
            r#"
            :root { --x: #111111; }
            .outer { --x: #222222; color: #333333; }
            .inner { color: #444444; }
            .leaf { border-color: var(--x); }
            "#,
        )
        .unwrap();
        let query = StyleQuery::class("leaf")
            .with_ancestor(StyleScope::class("outer"))
            .with_ancestor(StyleScope::class("inner"));
        let c = resolve_inherited(&sheet, &query);
        assert_eq!(
            c.props.get("border-color"),
            Some(&StyleValue::Color(Color::rgb(0x22, 0x22, 0x22)))
        );
        assert_eq!(
            c.props.get("color"),
            Some(&StyleValue::Color(Color::rgb(0x44, 0x44, 0x44)))
        );
    }

    #[test]
    fn no_ancestors_matches_plain_resolve() {
        // Without ancestors (and without the new keywords, which are
        // inheritance-dependent by definition) both paths agree exactly.
        let sheet = parse_stylesheet(
            r#"
            :root { --gold: #ebc878; }
            .button { color: var(--gold); width: calc(8px + 2px); }
            "#,
        )
        .unwrap();
        let query = StyleQuery::class("button");
        let plain = resolve(&sheet, &query);
        let inherited = resolve_inherited(&sheet, &query);
        assert_eq!(plain.props, inherited.props);
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
    fn child_combinator_matches_direct_parent_only() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            .menu > .button { color: #ff0000; }
            .menu .button { background: #123456; }
            "#,
        )
        .unwrap();
        // Direct child: both rules hit (child wins by order here).
        let direct = StyleQuery::class("button").with_ancestor_class("menu");
        let computed = resolve(&sheet, &direct);
        assert_eq!(computed.color_text(), Color::rgb(255, 0, 0));
        // Grandchild: child rule misses, descendant background still hits.
        let nested = StyleQuery::class("button")
            .with_ancestor_class("menu")
            .with_ancestor_class("panel");
        let computed = resolve(&sheet, &nested);
        assert_eq!(computed.color_text(), Color::rgb(0xaa, 0xaa, 0xaa));
        assert_eq!(computed.background(), Color::rgb(0x12, 0x34, 0x56));
        // No ancestors at all: plain rule only.
        let plain = StyleQuery::class("button");
        assert_eq!(resolve(&sheet, &plain).color_text().r, 0xaa);
    }

    #[test]
    fn not_and_is_filter_compounds() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            .button:not(.ghost) { color: #ff0000; }
            :is(.primary, .cta) { background: #123456; }
            button:not(#skip) { opacity: 0.5; }
            "#,
        )
        .unwrap();
        // Plain button: :not(.ghost) passes, :is misses.
        let plain = StyleQuery::class("button");
        let computed = resolve(&sheet, &plain);
        assert_eq!(computed.color_text(), Color::rgb(255, 0, 0));
        assert!(!computed.props.contains_key("background"));
        // Ghost button: vetoed back to the base color.
        let ghost = StyleQuery::class("button").with_class("ghost");
        assert_eq!(resolve(&sheet, &ghost).color_text().r, 0xaa);
        // Either alternative opens the background.
        let cta = StyleQuery::class("button").with_class("cta");
        assert_eq!(
            resolve(&sheet, &cta).background(),
            Color::rgb(0x12, 0x34, 0x56)
        );
        // Element + :not(#id): typed query without the id keeps opacity…
        let typed = StyleQuery::class("button").with_element("button");
        assert!((resolve(&sheet, &typed).number("opacity", 1.0) - 0.5).abs() < 1e-4);
        // …but the vetoed id loses it.
        let skipped = typed.with_id("skip");
        assert!(!resolve(&sheet, &skipped).props.contains_key("opacity"));
    }

    #[test]
    fn not_adds_argument_specificity() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            .button:not(#cta) { color: #ff0000; }
            "#,
        )
        .unwrap();
        // :not(#cta) = (0,1,0) + (1,0,0) = (1,1,0) beats the plain class.
        let plain = StyleQuery::class("button");
        assert_eq!(resolve(&sheet, &plain).color_text(), Color::rgb(255, 0, 0));
        // …while the vetoed id falls back to base.
        let cta = StyleQuery::class("button").with_id("cta");
        assert_eq!(resolve(&sheet, &cta).color_text().r, 0xaa);
    }

    #[test]
    fn is_takes_max_argument_specificity() {
        let sheet = parse_stylesheet(
            r#"
            .b { color: #aaaaaa; }
            .b#d { color: #0000ff; }
            :is(#a, .b) { color: #00ff00; }
            "#,
        )
        .unwrap();
        // :is() = max((1,0,0), (0,1,0)) = (1,0,0): beats the plain class…
        let b = StyleQuery::class("b");
        assert_eq!(resolve(&sheet, &b).color_text(), Color::rgb(0, 255, 0));
        // …but loses to .b#d at (1,1,0). A sum would give (1,1,0) and tie,
        // letting the later :is rule win — blue winning proves max().
        let bd = StyleQuery::class("b").with_id("d");
        assert_eq!(resolve(&sheet, &bd).color_text(), Color::rgb(0, 0, 255));
    }

    #[test]
    fn media_queries_gate_rules() {
        let sheet = parse_stylesheet(
            r#"
            .banner { opacity: 1; }
            @media (prefers-reduced-motion) {
              .banner { opacity: 0.5; }
            }
            @media (min-width: 600px) and (orientation: landscape) {
              .banner { opacity: 0.25; }
            }
            @media (prefers-contrast: more) {
              .banner { opacity: 0.75; }
            }
            @media not (prefers-reduced-motion) {
              .banner { border-width: 3; }
            }
            "#,
        )
        .unwrap();
        assert_eq!(sheet.media.len(), 4);
        let base = StyleQuery::class("banner");
        assert!((resolve(&sheet, &base).number("opacity", 0.0) - 1.0).abs() < 1e-4);
        // Reduced motion swaps opacity and kills the `not` branch.
        let calm = base.clone().with_reduced_motion(true);
        assert!((resolve(&sheet, &calm).number("opacity", 0.0) - 0.5).abs() < 1e-4);
        assert!(!resolve(&sheet, &calm).props.contains_key("border-width"));
        let motion = base
            .clone()
            .with_viewport_width(800.0)
            .with_orientation(crate::parse::MediaOrientation::Landscape);
        assert!((resolve(&sheet, &motion).number("opacity", 0.0) - 0.25).abs() < 1e-4);
        // Narrow viewport misses min-width but keeps the `not` branch.
        let narrow = base
            .clone()
            .with_viewport_width(400.0)
            .with_orientation(crate::parse::MediaOrientation::Landscape);
        assert!((resolve(&sheet, &narrow).number("opacity", 0.0) - 1.0).abs() < 1e-4);
        // High contrast overrides through its own branch (later = wins ties).
        let contrast = base.clone().with_high_contrast(true);
        assert!((resolve(&sheet, &contrast).number("opacity", 0.0) - 0.75).abs() < 1e-4);
    }

    #[test]
    fn media_accepts_bare_feature_shorthand() {
        let query = crate::parse::MediaQuery::parse("prefers-reduced-motion").unwrap();
        assert!(query.matches(&crate::parse::MediaContext {
            reduced_motion: true,
            ..Default::default()
        }));
    }

    #[test]
    fn media_rejects_nested_at_rules() {
        let err = parse_stylesheet(
            "@media (prefers-reduced-motion) { @keyframes x { from { opacity: 0; } } }",
        )
        .unwrap_err();
        assert!(err.to_string().contains("style rules only"));
        let err = parse_stylesheet("@media { .a { color: red; } }").unwrap_err();
        assert!(err.to_string().contains("@media needs a query"));
    }

    #[test]
    fn malformed_child_selectors_never_match() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
            > .button { color: #ff0000; }
            .menu > { color: #00ff00; }
            .menu >> .button { color: #0000ff; }
            "#,
        )
        .unwrap();
        let scoped = StyleQuery::class("button").with_ancestor_class("menu");
        assert_eq!(resolve(&sheet, &scoped).color_text().r, 0xaa);
    }

    #[test]
    fn unsupported_selectors_never_partially_match() {
        let sheet = parse_stylesheet(
            r#"
            .button { color: #aaaaaa; }
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
