//! `velvet vs3 check|run` — official VS3 game-logic entry.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_script_vs3::{compile, detect_edition, Value};

/// Parse/compile a VS3 file (`// @edition 3` required).
pub fn cmd_vs3_check(path: PathBuf) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let ed = detect_edition(&source);
    println!("edition={}", ed.as_str());
    let file = path.to_string_lossy();
    match compile(&source, Some(file.as_ref())) {
        Ok(m) => {
            let mut names = m.function_names();
            names.sort();
            println!(
                "ok: vs3 logic unit  functions={}  [{}]",
                names.len(),
                names.join(", ")
            );
            for d in &m.diagnostics {
                println!("warn: {}", d.display());
            }
            Ok(())
        }
        Err(e) => {
            for d in e.diagnostics() {
                println!("{}", d.display());
            }
            if e.diagnostics().is_empty() {
                println!("{e}");
            }
            bail!("vs3 check failed for {}", path.display());
        }
    }
}

/// Compile and call a named function with int/bool/string args.
///
/// Args syntax: `i:42`, `b:true`, `s:hello`, or bare integers.
pub fn cmd_vs3_run(path: PathBuf, call: String, args: Vec<String>) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let file = path.to_string_lossy();
    let module = compile(&source, Some(file.as_ref())).map_err(|e| {
        for d in e.diagnostics() {
            eprintln!("{}", d.display());
        }
        anyhow::anyhow!("{e}")
    })?;
    let values = parse_args(&args)?;
    let v = module
        .call(&call, &values)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("=> {v}");
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for a in args {
        if let Some(rest) = a.strip_prefix("i:") {
            let n: i64 = rest
                .parse()
                .with_context(|| format!("bad int arg {a}"))?;
            out.push(Value::Int(n));
        } else if let Some(rest) = a.strip_prefix("b:") {
            let b = match rest {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => bail!("bad bool arg {a}"),
            };
            out.push(Value::Bool(b));
        } else if let Some(rest) = a.strip_prefix("s:") {
            out.push(Value::String(std::rc::Rc::from(rest)));
        } else if let Ok(n) = a.parse::<i64>() {
            out.push(Value::Int(n));
        } else if a == "true" {
            out.push(Value::Bool(true));
        } else if a == "false" {
            out.push(Value::Bool(false));
        } else {
            out.push(Value::String(std::rc::Rc::from(a.as_str())));
        }
    }
    Ok(out)
}
