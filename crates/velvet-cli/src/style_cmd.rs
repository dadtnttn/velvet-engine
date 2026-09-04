//! `velvet style check|dump|run|eval|explain` — author tooling for `.vcss`.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_style::{
    check_stylesheet, explain_cascade, parse_stylesheet, parse_stylesheet_with_imports, resolve,
    StyleQuery, StyleScope,
};

/// Parse and report a stylesheet file.
pub fn cmd_style_check(path: PathBuf) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let report = if let Some(parent) = path.parent() {
        match parse_stylesheet_with_imports(&src, parent) {
            Ok(sheet) => {
                println!(
                    "ok {} — rules={} keyframes={} fns={} imports={}",
                    path.display(),
                    sheet.rules.len(),
                    sheet.keyframes.len(),
                    sheet.script.functions.len(),
                    sheet.imports.len()
                );
                for w in velvet_style::lint_stylesheet(&sheet) {
                    println!("warn: {w}");
                }
                return Ok(());
            }
            Err(e) => {
                // fall through to plain check message
                let r = check_stylesheet(&src);
                if !r.ok {
                    bail!("{}: {}", path.display(), e);
                }
                r
            }
        }
    } else {
        check_stylesheet(&src)
    };
    if !report.ok {
        bail!(
            "{}: {}",
            path.display(),
            report.error.unwrap_or_else(|| "parse failed".into())
        );
    }
    println!(
        "ok {} — rules={} keyframes={} fns={} imports={}",
        path.display(),
        report.rules,
        report.keyframes,
        report.functions,
        report.imports
    );
    for w in &report.warnings {
        println!("warn: {w}");
    }
    Ok(())
}

/// Resolve a class (+ optional state) and print props.
pub fn cmd_style_dump(path: PathBuf, class: String, state: Option<String>) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let sheet = if let Some(parent) = path.parent() {
        parse_stylesheet_with_imports(&src, parent).or_else(|_| parse_stylesheet(&src))?
    } else {
        parse_stylesheet(&src)?
    };
    let mut q = StyleQuery::class(class.clone());
    if let Some(s) = state.clone() {
        q = q.with_state(s);
    }
    let c = resolve(&sheet, &q);
    println!("dump {} class={} state={:?}", path.display(), class, state);
    for (k, v) in &c.props {
        println!("  {k}: {v:?}");
    }
    println!("props={}", c.props.len());
    Ok(())
}

/// Run a named `@script` function and print actions + return value.
pub fn cmd_style_run(path: PathBuf, name: String, args: Vec<String>) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let sheet = if let Some(parent) = path.parent() {
        parse_stylesheet_with_imports(&src, parent).or_else(|_| parse_stylesheet(&src))?
    } else {
        parse_stylesheet(&src)?
    };
    let js_args: Vec<velvet_style::JsValue> = args.iter().map(|a| js_arg(a)).collect();
    let run =
        velvet_style::call_style_fn(&sheet, &name, &js_args).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("run {name} ({} action(s)):", run.actions.len());
    for action in &run.actions {
        println!("  {action:?}");
    }
    for line in &run.logs {
        println!("  log: {line}");
    }
    println!("value: {:?}", run.value);
    Ok(())
}

/// Resolve a class to JSON for theme snapshot testing in CI.
pub fn cmd_style_snapshot(
    path: PathBuf,
    class: String,
    state: Option<String>,
    expect: Option<PathBuf>,
    write: bool,
) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let sheet = if let Some(parent) = path.parent() {
        parse_stylesheet_with_imports(&src, parent).or_else(|_| parse_stylesheet(&src))?
    } else {
        parse_stylesheet(&src)?
    };
    let mut q = StyleQuery::class(class.clone());
    if let Some(s) = state.clone() {
        q = q.with_state(s);
    }
    let computed = resolve(&sheet, &q);
    let mut map = serde_json::Map::new();
    let mut keys: Vec<&String> = computed.props.keys().collect();
    keys.sort();
    for k in keys {
        map.insert(
            k.clone(),
            serde_json::Value::String(velvet_style::display_value(&computed.props[k])),
        );
    }
    let snapshot = serde_json::Value::Object(map);
    let pretty = serde_json::to_string_pretty(&snapshot).unwrap_or_default();
    match (expect, write) {
        (Some(path), true) => {
            std::fs::write(&path, format!("{pretty}\n"))
                .with_context(|| format!("write {}", path.display()))?;
            println!("wrote snapshot {}", path.display());
            Ok(())
        }
        (Some(path), false) => {
            let expected = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            let expected: serde_json::Value = serde_json::from_str(&expected)?;
            if expected == snapshot {
                println!("snapshot ok ({})", path.display());
                Ok(())
            } else {
                bail!("snapshot mismatch for {} --state {:?}:\nexpected:\n{expected:#}\ngot:\n{pretty}", class, state);
            }
        }
        (None, _) => {
            println!("{pretty}");
            Ok(())
        }
    }
}

/// Format a `.vcss` file in place (idempotent; `--minify` squeezes it).
pub fn cmd_style_fmt(path: PathBuf, minify: bool) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let out = if minify {
        velvet_style::format_stylesheet_min(&src).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        velvet_style::format_stylesheet(&src).map_err(|e| anyhow::anyhow!("{e}"))?
    };
    std::fs::write(&path, &out).with_context(|| format!("write {}", path.display()))?;
    println!(
        "formatted {} ({})",
        path.display(),
        if minify { "min" } else { "pretty" }
    );
    Ok(())
}

/// Evaluate one `@script` expression statement via a scratch function.
pub fn cmd_style_eval(path: PathBuf, expr: String) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let sheet = if let Some(parent) = path.parent() {
        parse_stylesheet_with_imports(&src, parent).or_else(|_| parse_stylesheet(&src))?
    } else {
        parse_stylesheet(&src)?
    };
    let probe = format!("fn __eval() {{ return {expr}; }}");
    let module = velvet_style::parse_script(&probe).map_err(|e| anyhow::anyhow!("{e}"))?;
    // Merge sheet globals so helpers like stagger resolve.
    let mut merged = module;
    merged.globals = sheet.script.globals.clone();
    for (name, func) in &sheet.script.functions {
        merged.functions.insert(name.clone(), func.clone());
    }
    let run = velvet_style::run_function(&merged, Some(&sheet), "__eval", &[])
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("value: {:?}", run.value);
    for line in &run.logs {
        println!("  log: {line}");
    }
    Ok(())
}

/// Trace the cascade for one property on a selector and print every
/// matched rule in cascade order (winner flagged), plus the resolved value.
pub fn cmd_style_explain(
    path: PathBuf,
    selector: String,
    prop: String,
    ancestors: Vec<String>,
    reduced_motion: bool,
) -> Result<()> {
    let src = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let sheet = if let Some(parent) = path.parent() {
        parse_stylesheet_with_imports(&src, parent).or_else(|_| parse_stylesheet(&src))?
    } else {
        parse_stylesheet(&src)?
    };
    let mut query = parse_explain_query(&selector)?;
    for ancestor in &ancestors {
        query = query.with_ancestor(parse_explain_scope(ancestor)?);
    }
    query.reduced_motion = reduced_motion;
    let (steps, resolved) = explain_cascade(&sheet, &query, prop.trim());
    println!(
        "explain {} prop={} ({} matched rule(s))",
        selector,
        prop.trim(),
        steps.len()
    );
    for step in &steps {
        let (a, b, c) = step.specificity;
        let media = step
            .media
            .as_deref()
            .map(|m| format!(" @media {m}"))
            .unwrap_or_default();
        let declared = step
            .declared
            .as_ref()
            .map(velvet_style::display_value)
            .unwrap_or_else(|| "(no value)".to_string());
        let mark = if step.wins { "*" } else { " " };
        let rank = if step.order == usize::MAX {
            "--".to_string()
        } else {
            format!("{:<2}", step.order)
        };
        println!(
            "{mark} #{rank} ({a},{b},{c}) {}{media} = {declared}",
            step.selector
        );
    }
    match resolved {
        Some(value) => println!("=> {}", velvet_style::display_value(&value)),
        None => println!("=> (unset)"),
    }
    Ok(())
}

/// Build a [`StyleQuery`] from one compound (`button.primary:hover`,
/// `#start`, `.a.b`, …). Descendant/child selectors belong in `--ancestor`.
fn parse_explain_query(text: &str) -> Result<StyleQuery> {
    let (element, id, classes, states) = parse_explain_compound(text)?;
    Ok(StyleQuery {
        element,
        id,
        classes,
        states,
        ..Default::default()
    })
}

/// Build an ancestor [`StyleScope`] from one compound.
fn parse_explain_scope(text: &str) -> Result<StyleScope> {
    let (element, id, classes, states) = parse_explain_compound(text)?;
    Ok(StyleScope {
        element,
        id,
        classes,
        states,
    })
}

/// Element / id / classes / states of one selector compound.
type CompoundParts = (Option<String>, Option<String>, Vec<String>, Vec<String>);

/// Split one compound into element / id / classes / states.
///
/// Rejects combinators (` `, `>`, `+`, `~`) and `:not()`/`:is()` with a
/// pointer to the supported form instead of guessing.
fn parse_explain_compound(text: &str) -> Result<CompoundParts> {
    if text.contains([' ', '>', '+', '~']) {
        bail!("explain takes one compound like `.button.primary:hover`; move ancestors to `--ancestor <compound>` (repeatable, root-to-parent)");
    }
    if text.contains("not(") || text.contains("is(") {
        bail!("explain queries support element/#id/.class/:state only (no :not()/:is())");
    }
    let mut chunks = text.split(':');
    let base = chunks.next().unwrap_or("").trim();
    let states: Vec<String> = chunks
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let mut element = None;
    let mut id = None;
    let mut classes = Vec::new();
    let mut rest = base;
    // Leading element name (up to the first `.` or `#`).
    let cut = rest.find(['.', '#']).unwrap_or(rest.len());
    if cut > 0 {
        element = Some(rest[..cut].to_string());
    }
    rest = &rest[cut..];
    while !rest.is_empty() {
        let marker = rest.as_bytes()[0];
        if marker != b'.' && marker != b'#' {
            bail!("cannot parse selector `{text}` (expected `.class` or `#id`)");
        }
        let end = rest[1..]
            .find(['.', '#'])
            .map(|i| i + 1)
            .unwrap_or(rest.len());
        let name = rest[1..end].trim();
        if name.is_empty() {
            bail!("cannot parse selector `{text}` (empty name)");
        }
        if marker == b'#' {
            if id.is_some() {
                bail!("cannot parse selector `{text}` (one #id per compound)");
            }
            id = Some(name.to_string());
        } else {
            classes.push(name.to_string());
        }
        rest = &rest[end..];
    }
    if element.is_none() && id.is_none() && classes.is_empty() && states.is_empty() {
        bail!("empty selector (try `.button` or `button.primary:hover`)");
    }
    Ok((element, id, classes, states))
}

/// Parse one CLI arg: numbers, bools, null, or strings.
fn js_arg(raw: &str) -> velvet_style::JsValue {
    use velvet_style::JsValue;
    match raw {
        "true" => JsValue::Bool(true),
        "false" => JsValue::Bool(false),
        "null" | "undefined" => JsValue::Null,
        _ => raw
            .parse::<f32>()
            .map(JsValue::Number)
            .unwrap_or_else(|_| JsValue::String(raw.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn sample_sheet() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ui.vcss");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            ".card {{ width: calc(100px + 20px); background: color-mix(in srgb, red, blue); }}\n\
             @keyframes deal {{ from {{ opacity: 0; }} to {{ opacity: 1; }} }}\n\
             @script {{\n\
             fn deal() {{ play(\"deal\", {{ target: \"c0\", duration: 0.5 }}); return 1; }}\n\
             }}"
        )
        .unwrap();
        (dir, path)
    }

    #[test]
    fn run_executes_named_function() {
        let (_dir, path) = sample_sheet();
        cmd_style_run(path, "deal".into(), Vec::new()).unwrap();
    }

    #[test]
    fn eval_evaluates_expression() {
        let (_dir, path) = sample_sheet();
        cmd_style_eval(path, "1 + 2 * 3".into()).unwrap();
    }

    #[test]
    fn run_missing_function_errors() {
        let (_dir, path) = sample_sheet();
        assert!(cmd_style_run(path, "nope".into(), Vec::new()).is_err());
    }

    #[test]
    fn snapshot_write_then_compare() {
        let (dir, path) = sample_sheet();
        let snap = dir.path().join("button.json");
        cmd_style_snapshot(path.clone(), "card".into(), None, Some(snap.clone()), true).unwrap();
        cmd_style_snapshot(path.clone(), "card".into(), None, Some(snap.clone()), false).unwrap();
        // Drift the sheet: snapshot must fail.
        let mut text = std::fs::read_to_string(&path).unwrap();
        text = text.replace("color-mix(in srgb, red, blue)", "red");
        std::fs::write(&path, text).unwrap();
        assert!(cmd_style_snapshot(path, "card".into(), None, Some(snap), false).is_err());
    }

    #[test]
    fn explain_traces_cascade_and_rejects_compounds() {
        let (_dir, path) = sample_sheet();
        // Winner + resolved value print without error.
        cmd_style_explain(
            path.clone(),
            ".card".into(),
            "width".into(),
            Vec::new(),
            false,
        )
        .unwrap();
        // Unset property still lists the matched rules.
        cmd_style_explain(
            path.clone(),
            ".card".into(),
            "color".into(),
            Vec::new(),
            false,
        )
        .unwrap();
        // Descendant selectors belong in --ancestor; :not() is unsupported.
        assert!(cmd_style_explain(
            path.clone(),
            ".menu .card".into(),
            "width".into(),
            Vec::new(),
            false
        )
        .is_err());
        assert!(cmd_style_explain(
            path,
            ".card:not(.x)".into(),
            "width".into(),
            Vec::new(),
            false
        )
        .is_err());
    }

    #[test]
    fn explain_query_parser_reads_compounds() {
        let q = parse_explain_query("button.primary:hover").unwrap();
        assert_eq!(q.element.as_deref(), Some("button"));
        assert_eq!(q.classes, vec!["primary".to_string()]);
        assert_eq!(q.states, vec!["hover".to_string()]);
        let q = parse_explain_query("#start.big").unwrap();
        assert_eq!(q.id.as_deref(), Some("start"));
        assert!(parse_explain_query("").is_err());
        assert!(parse_explain_query(".a#b#c").is_err());
    }

    #[test]
    fn fmt_rewrites_messy_sheet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("messy.vcss");
        std::fs::write(&path, ".card{width:calc(8px+4px);}\n").unwrap();
        cmd_style_fmt(path.clone(), false).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(out.contains(".card {\n  width: 12px;\n"), "{out:?}");
        // Second run is stable.
        cmd_style_fmt(path.clone(), false).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), out);
    }

    #[test]
    fn fmt_minify_squeezes_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mini.vcss");
        std::fs::write(&path, ".card{width:calc(8px+4px);}\n").unwrap();
        cmd_style_fmt(path.clone(), true).unwrap();
        let out = std::fs::read_to_string(&path).unwrap();
        assert!(!out.contains('\n'), "{out:?}");
        // Min output is stable and pretty-prints back to the same pretty form.
        cmd_style_fmt(path.clone(), true).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), out);
        cmd_style_fmt(path.clone(), false).unwrap();
        let pretty = std::fs::read_to_string(&path).unwrap();
        assert!(pretty.contains(".card {\n  width: 12px;\n"), "{pretty:?}");
    }
}
