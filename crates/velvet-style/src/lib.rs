//! # velvet-style
//!
//! **`.vcss`** — one author language that blends **CSS** and **JS-lite** for UI
//! look + motion orchestration.
//!
//! | Side | What |
//! |------|------|
//! | **CSS** | selectors, cascade, colors, `@keyframes`, `animation:` |
//! | **JS-lite** | `@script { let, fn, for, if, play, animate, wait, on }` |
//!
//! Runtime playback still uses **velvet-anim** tools (`timeline_from_plan`).
//!
//! ## Example
//!
//! ```css
//! .button { background: #0a0c16; color: #d2af64; height: 52; }
//! .button:selected { background: #501e78; glow: #dc50dc; }
//!
//! @keyframes deal {
//!   from { opacity: 0; y: -80; scale: 0.65; yaw: 0.9; }
//!   to   { opacity: 1; y: 0;   scale: 1;    yaw: 0; }
//! }
//!
//! @script {
//!   let stagger = 0.08;
//!   fn dealHand(count) {
//!     for (let i = 0; i < count; i = i + 1) {
//!       play("deal", {
//!         target: "card" + i,
//!         delay: i * stagger,
//!         duration: 0.32,
//!         ease: "cubic_out"
//!       });
//!     }
//!   }
//!   on("menu.open", fn () {
//!     animate("#logo", { opacity: [0, 1], y: [-24, 0] }, 0.45);
//!   });
//! }
//! ```

#![deny(missing_docs)]

mod animation;
pub mod fmt;
mod host;
mod parse;
mod resolve;
mod runtime;
mod script;
mod value;

pub use animation::{
    animation_spec_from_computed, plan_animation, plan_from_spec, vanim_to_vcss, AnimationSpec,
    ChannelPlan, KeyframeStop, Keyframes, TimelinePlan,
};
pub use fmt::{display_value, format_stylesheet, format_stylesheet_min};
pub use host::StyleStoryHost;
pub use parse::{
    parse_stylesheet, parse_stylesheet_with_imports, MediaAnd, MediaBlock, MediaCondition,
    MediaContext, MediaOrientation, MediaQuery, StyleParseError, StyleRule, Stylesheet, SvgDef,
};
pub use resolve::{
    expand_box_shorthands, explain_cascade, resolve, resolve_expanded, resolve_inherited,
    ComputedStyle, ExplainStep, StyleQuery, StyleRegistry, StyleScope,
};
pub use runtime::{
    computed_number, is_numeric_style_value, plan_channel_tween, plan_transition, StyleRuntime,
};
pub use script::{
    actions_to_timelines, eval_script_fn, parse_script, run_event, run_event_drain, run_function,
    run_function_with_runtime, EventHandler, Function, JsValue, ScriptError, ScriptModule,
    ScriptRun, StyleAction,
};
pub use value::{
    contrast_pick, eval_calc, eval_calc_in_place, expand_shorthand, hsl_to_rgb, mix_concrete,
    parse_alpha_function, parse_auto_contrast_fn, parse_box_shadow, parse_calc_func, parse_color,
    parse_color_fn_deferred, parse_color_mix, parse_color_or_var, parse_gradient,
    parse_oklch_function, parse_value, relative_luminance, subst_calc_vars, BoxShadow, CalcExpr,
    Color, Gradient, GradientStop, StyleValue, KNOWN_PROPERTIES,
};

/// Run a named `@script` function from a parsed stylesheet.
pub fn call_style_fn(
    sheet: &Stylesheet,
    name: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    run_function(&sheet.script, Some(sheet), name, args)
}

/// Run script fn with a mutable [`StyleRuntime`] for `set`/`query`.
pub fn call_style_fn_rt(
    sheet: &Stylesheet,
    name: &str,
    args: &[JsValue],
    runtime: &mut StyleRuntime,
) -> Result<ScriptRun, ScriptError> {
    run_function_with_runtime(&sheet.script, Some(sheet), name, args, Some(runtime))
}

/// Parse summary for CLI / tooling.
#[derive(Debug, Clone)]
pub struct StyleCheckReport {
    /// Rules count.
    pub rules: usize,
    /// Keyframes count.
    pub keyframes: usize,
    /// Script functions.
    pub functions: usize,
    /// Import paths.
    pub imports: usize,
    /// Whether parse succeeded.
    pub ok: bool,
    /// Error message if any.
    pub error: Option<String>,
    /// Non-fatal warnings (unknown properties, unresolved vars, …).
    pub warnings: Vec<String>,
}

/// Collect lint warnings for a parsed sheet.
///
/// Covers unknown properties (with did-you-mean hints), `animation-name`
/// without a matching `@keyframes`, `var(--x)` without fallback or
/// definition (including `var()` inside function arguments), and
/// `transition-property` naming unknown properties. Base rules and
/// `@media` blocks are both scanned; `@script` bodies are not (dynamic).
/// Custom properties defined but never used via `var()` warn as unused.
pub fn lint_stylesheet(sheet: &Stylesheet) -> Vec<String> {
    use crate::value::KNOWN_PROPERTIES;
    let mut warnings = Vec::new();
    // Custom properties defined anywhere (base + @media) satisfy var().
    let defined_vars: std::collections::HashSet<&str> = sheet
        .rules
        .iter()
        .chain(sheet.media.iter().flat_map(|block| block.rules.iter()))
        .flat_map(|rule| rule.declarations.keys())
        .filter(|name| name.starts_with("--"))
        .map(String::as_str)
        .collect();
    let mut lint_rule = |rule: &crate::parse::StyleRule, sel: &str| {
        for (name, value) in &rule.declarations {
            // custom properties are always fine
            if name.starts_with("--") {
                continue;
            }
            if !KNOWN_PROPERTIES.contains(&name.as_str()) {
                match suggest_property(name) {
                    Some(hint) => warnings.push(format!(
                        "unknown property `{name}` on `{sel}` (did you mean `{hint}`?)"
                    )),
                    None => warnings.push(format!("unknown property `{name}` on `{sel}`")),
                }
                continue;
            }
            if name == "animation-name" || name == "animation" {
                for anim in animation_names(value) {
                    if !sheet.keyframes.contains_key(anim) {
                        warnings.push(format!(
                            "unknown keyframes `{anim}` on `{sel}` (no @keyframes {anim})"
                        ));
                    }
                }
            }
            if name == "transition-property" {
                if let Some(prop) = single_keyword(value) {
                    if prop != "none" && prop != "all" && !KNOWN_PROPERTIES.contains(&prop) {
                        warnings.push(format!("unknown transition-property `{prop}` on `{sel}`"));
                    }
                }
            }
            check_unresolved_vars(value, &defined_vars, sel, &mut warnings);
        }
    };
    for rule in &sheet.rules {
        let sel = rule.selectors.join(", ");
        lint_rule(rule, &sel);
    }
    for block in &sheet.media {
        for rule in &block.rules {
            let sel = format!(
                "@media {} {{ {} }}",
                block.query_text,
                rule.selectors.join(", ")
            );
            lint_rule(rule, &sel);
        }
    }
    // Dead custom properties: defined but never referenced via var().
    let mut used_vars = std::collections::HashSet::new();
    for rule in sheet
        .rules
        .iter()
        .chain(sheet.media.iter().flat_map(|block| block.rules.iter()))
    {
        for value in rule.declarations.values() {
            collect_used_vars(value, &mut used_vars);
        }
    }
    let mut dead: Vec<&&str> = defined_vars
        .iter()
        .filter(|name| !used_vars.contains(**name))
        .collect();
    dead.sort();
    for name in dead {
        warnings.push(format!("unused {name} (defined but never used via var())"));
    }
    warnings
}

/// Closest known property within edit distance 2 (for did-you-mean hints).
fn suggest_property(name: &str) -> Option<&'static str> {
    use crate::value::KNOWN_PROPERTIES;
    let mut best: Option<(&'static str, usize)> = None;
    for candidate in KNOWN_PROPERTIES {
        let candidate: &'static str = candidate;
        let dist = edit_distance(name, candidate);
        // (no `Option::is_none_or`: MSRV is 1.80)
        let better = match best {
            None => true,
            Some((_, d)) => dist < d,
        };
        if dist <= 2 && better {
            best = Some((candidate, dist));
        }
    }
    best.map(|(candidate, _)| candidate)
}

/// Levenshtein distance over chars (short lint-time strings only).
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut curr = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            curr.push(
                (prev[j] + usize::from(ca != cb))
                    .min(prev[j + 1] + 1)
                    .min(curr[j] + 1),
            );
        }
        prev = curr;
    }
    prev[b.len()]
}

/// Collect every `--name` referenced via `var()` in a value, including
/// function arguments (`calc()` leaves, `color-mix()`, gradient stops,
/// shadow colors) and `var()` text inside plain keywords.
///
/// Quoted strings are literals, never references, so they are skipped.
fn collect_used_vars(
    value: &crate::value::StyleValue,
    out: &mut std::collections::HashSet<String>,
) {
    use crate::value::StyleValue;
    match value {
        StyleValue::Var(name) => {
            out.insert(name.clone());
        }
        StyleValue::VarFallback { name, fallback } => {
            out.insert(name.clone());
            collect_used_vars(fallback, out);
        }
        StyleValue::Calc(expr) => collect_calc_vars(expr, out),
        StyleValue::ColorMix { a, b, .. } => {
            collect_used_vars(a, out);
            collect_used_vars(b, out);
        }
        StyleValue::Alpha { color, .. } => collect_used_vars(color, out),
        StyleValue::AutoContrast { base, dark, light } => {
            collect_used_vars(base, out);
            collect_used_vars(dark, out);
            collect_used_vars(light, out);
        }
        StyleValue::Gradient(g) => {
            for stop in &g.stops {
                collect_used_vars(&stop.color, out);
            }
        }
        StyleValue::Shadow(layers) => {
            for layer in layers {
                collect_used_vars(&layer.color, out);
            }
        }
        StyleValue::Keyword(text) => {
            for name in vars_in_text(text) {
                out.insert(name);
            }
        }
        _ => {}
    }
}

/// `--names` referenced by `var()` leaves in a calc expression.
fn collect_calc_vars(expr: &CalcExpr, out: &mut std::collections::HashSet<String>) {
    use crate::value::CalcExpr;
    match expr {
        CalcExpr::Lit(_) => {}
        CalcExpr::Add(a, b) | CalcExpr::Sub(a, b) | CalcExpr::Mul(a, b) | CalcExpr::Div(a, b) => {
            collect_calc_vars(a, out);
            collect_calc_vars(b, out);
        }
        CalcExpr::Min(args) | CalcExpr::Max(args) => {
            for arg in args {
                collect_calc_vars(arg, out);
            }
        }
        CalcExpr::Clamp(lo, val, hi) => {
            collect_calc_vars(lo, out);
            collect_calc_vars(val, out);
            collect_calc_vars(hi, out);
        }
        CalcExpr::Var { name, fallback } => {
            out.insert(name.clone());
            if let Some(fb) = fallback {
                collect_calc_vars(fb, out);
            }
        }
    }
}

/// Scan free text for `var(--name)` references (case-sensitive names).
fn vars_in_text(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let mut head = [0u8; 4];
        head.copy_from_slice(&bytes[i..i + 4]);
        head.make_ascii_lowercase();
        if &head == b"var(" {
            let mut j = i + 4;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let start = j;
            if bytes.get(j) == Some(&b'-') && bytes.get(j + 1) == Some(&b'-') {
                j += 2;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'-' || bytes[j] == b'_')
                {
                    j += 1;
                }
                if j > start + 2 {
                    if let Ok(name) = std::str::from_utf8(&bytes[start..j]) {
                        out.push(name.to_string());
                    }
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Animation names referenced by `animation-name` / `animation`.
/// Shorthand takes the first token of each comma part (`deal 0.32s`
/// → `deal`); `none`/`inherit`/keywords and dynamic values are skipped.
fn animation_names(value: &crate::value::StyleValue) -> Vec<&str> {
    use crate::value::StyleValue;
    let text = match value {
        StyleValue::Keyword(s) | StyleValue::String(s) => s.as_str(),
        _ => return Vec::new(),
    };
    text.split(',')
        .filter_map(|part| part.split_whitespace().next())
        .filter(|name| {
            !matches!(
                *name,
                "none" | "inherit" | "initial" | "unset" | "auto" | "infinite"
            ) && !name.chars().next().is_some_and(|c| c.is_ascii_digit())
        })
        .collect()
}

/// Plain keyword text when the value is a static keyword.
fn single_keyword(value: &crate::value::StyleValue) -> Option<&str> {
    match value {
        crate::value::StyleValue::Keyword(s) => Some(s.as_str()),
        _ => None,
    }
}

/// Warn on `var(--x)` without fallback whose name is never defined.
fn check_unresolved_vars(
    value: &crate::value::StyleValue,
    defined: &std::collections::HashSet<&str>,
    selectors: &str,
    warnings: &mut Vec<String>,
) {
    use crate::value::StyleValue;
    match value {
        StyleValue::Var(name) => {
            if !defined.contains(name.as_str()) {
                warnings.push(format!(
                    "unresolved var({name}) on `{selectors}` (no fallback, -- never defined)"
                ));
            }
        }
        StyleValue::VarFallback { name, fallback } => {
            // Recurse: the fallback may itself reference vars.
            if !defined.contains(name.as_str()) {
                check_unresolved_vars(fallback, defined, selectors, warnings);
            }
        }
        // Calc factors may hold var() leaves (with optional fallbacks).
        StyleValue::Calc(expr) => check_calc_vars(expr, defined, selectors, warnings),
        StyleValue::ColorMix { a, b, .. } => {
            check_unresolved_vars(a, defined, selectors, warnings);
            check_unresolved_vars(b, defined, selectors, warnings);
        }
        StyleValue::Alpha { color, .. } => {
            check_unresolved_vars(color, defined, selectors, warnings);
        }
        StyleValue::AutoContrast { base, dark, light } => {
            check_unresolved_vars(base, defined, selectors, warnings);
            check_unresolved_vars(dark, defined, selectors, warnings);
            check_unresolved_vars(light, defined, selectors, warnings);
        }
        StyleValue::Gradient(g) => {
            for stop in &g.stops {
                check_unresolved_vars(&stop.color, defined, selectors, warnings);
            }
        }
        StyleValue::Shadow(layers) => {
            for layer in layers {
                check_unresolved_vars(&layer.color, defined, selectors, warnings);
            }
        }
        _ => {}
    }
}

/// Warn on `var(--x)` leaves inside calc expressions.
fn check_calc_vars(
    expr: &crate::value::CalcExpr,
    defined: &std::collections::HashSet<&str>,
    selectors: &str,
    warnings: &mut Vec<String>,
) {
    use crate::value::CalcExpr;
    match expr {
        CalcExpr::Lit(_) => {}
        CalcExpr::Add(a, b) | CalcExpr::Sub(a, b) | CalcExpr::Mul(a, b) | CalcExpr::Div(a, b) => {
            check_calc_vars(a, defined, selectors, warnings);
            check_calc_vars(b, defined, selectors, warnings);
        }
        CalcExpr::Min(args) | CalcExpr::Max(args) => {
            for arg in args {
                check_calc_vars(arg, defined, selectors, warnings);
            }
        }
        CalcExpr::Clamp(lo, val, hi) => {
            check_calc_vars(lo, defined, selectors, warnings);
            check_calc_vars(val, defined, selectors, warnings);
            check_calc_vars(hi, defined, selectors, warnings);
        }
        CalcExpr::Var { name, fallback } => {
            if !defined.contains(name.as_str()) {
                match fallback {
                    Some(fb) => check_calc_vars(fb, defined, selectors, warnings),
                    None => warnings.push(format!(
                        "unresolved var({name}) on `{selectors}` (no fallback, -- never defined)"
                    )),
                }
            }
        }
    }
}

/// Check a `.vcss` source string (shipped CLI path).
pub fn check_stylesheet(source: &str) -> StyleCheckReport {
    match parse_stylesheet(source) {
        Ok(s) => {
            let warnings = lint_stylesheet(&s);
            StyleCheckReport {
                rules: s.rules.len(),
                keyframes: s.keyframes.len(),
                functions: s.script.functions.len(),
                imports: s.imports.len(),
                ok: true,
                error: None,
                warnings,
            }
        }
        Err(e) => StyleCheckReport {
            rules: 0,
            keyframes: 0,
            functions: 0,
            imports: 0,
            ok: false,
            error: Some(e.to_string()),
            warnings: Vec::new(),
        },
    }
}

/// Dispatch `on("event", …)` handlers defined in the stylesheet script.
pub fn emit_style_event(
    sheet: &Stylesheet,
    event: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    run_event(&sheet.script, Some(sheet), event, args)
}

/// Dispatch handlers like [`emit_style_event`], dropping fired `once`
/// handlers from the sheet's script module afterwards.
pub fn emit_style_event_drain(
    sheet: &mut Stylesheet,
    event: &str,
    args: &[JsValue],
) -> Result<ScriptRun, ScriptError> {
    let run = run_event(&sheet.script, Some(sheet), event, args)?;
    sheet
        .script
        .handlers
        .retain(|h| !(h.event == event && h.once));
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_flags_unknown_properties_only() {
        let sheet = parse_stylesheet(
            ".button { background: #000; frobnicate: 1; --x: 2; }\n#start { color: red; }",
        )
        .unwrap();
        let warnings = lint_stylesheet(&sheet);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("frobnicate"));
        assert!(warnings[0].contains(".button"));
        // `--x` is defined but never used via var().
        assert!(warnings[1].contains("unused --x"), "{warnings:?}");
        let report = check_stylesheet(".a { color: red; }");
        assert!(report.ok && report.warnings.is_empty());
    }

    #[test]
    fn lint_flags_missing_keyframes_and_vars() {
        let sheet = parse_stylesheet(
            r#"
            @keyframes deal { from { opacity: 0; } to { opacity: 1; } }
            .card { animation-name: deal; }
            .ghost { animation-name: vanish; }
            .sh { animation: deal 0.3s, vanish 1s; }
            .ok { color: var(--ink); }
            .bad { color: var(--nope); }
            .fb { color: var(--nope, red); }
            :root { --ink: #111; }
            .tp { transition-property: opacity; }
            .tpb { transition-property: frobnicate; }
            "#,
        )
        .unwrap();
        let warnings = lint_stylesheet(&sheet);
        let has = |needle: &str| warnings.iter().any(|w| w.contains(needle));
        assert!(has("vanish"), "{warnings:?}");
        assert!(has("--nope"), "{warnings:?}");
        assert!(has("frobnicate"), "{warnings:?}");
        // Defined var, fallback var, real keyframes, known props: silent.
        assert_eq!(warnings.len(), 4, "{warnings:?}");
    }

    #[test]
    fn lint_suggests_known_properties_and_flags_dead_vars() {
        let sheet = parse_stylesheet(
            r#"
            :root { --gold: #ebc878; --dead: 4px; --used-in-calc: 8px; }
            .button {
              backgroud: #000;
              colour: var(--gold);
              width: calc(var(--used-in-calc) + 2px);
            }
            @media (prefers-reduced-motion) {
              .button { opacity: var(--gold); }
            }
            "#,
        )
        .unwrap();
        let warnings = lint_stylesheet(&sheet);
        let has = |needle: &str| warnings.iter().any(|w| w.contains(needle));
        // Did-you-mean (distance <= 2), never inventing names.
        assert!(has("`backgroud`") && has("`background`"), "{warnings:?}");
        // British spelling still points at the real property.
        assert!(has("`colour`") && has("`color`"), "{warnings:?}");
        // --gold used (plain + media), --used-in-calc used in calc: silent.
        assert!(
            !warnings.iter().any(|w| w.contains("--gold")),
            "{warnings:?}"
        );
        assert!(
            !warnings.iter().any(|w| w.contains("--used-in-calc")),
            "{warnings:?}"
        );
        // --dead defined nowhere referenced: flagged once.
        assert_eq!(
            warnings
                .iter()
                .filter(|w| w.contains("unused --dead"))
                .count(),
            1,
            "{warnings:?}"
        );
    }
}
