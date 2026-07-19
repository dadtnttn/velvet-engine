//! `velvet style check|dump` — author tooling for `.vcss`.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_style::{
    check_stylesheet, parse_stylesheet, parse_stylesheet_with_imports, resolve, StyleQuery,
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
