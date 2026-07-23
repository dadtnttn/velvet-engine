//! `velvet vs3 check|run` — official VS3 game-logic entry.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_script_vs3::{compile, detect_edition, Value, Vs3Package, Vs3TaskStatus};

/// Parse/compile a VS3 file (`// @edition 3` required).
pub fn cmd_vs3_check(path: PathBuf) -> Result<()> {
    if path.is_dir() {
        let mut package = Vs3Package::new();
        let mut function_count = 0usize;
        for file_path in vs3_files(&path)? {
            let source = std::fs::read_to_string(&file_path)
                .with_context(|| format!("read {}", file_path.display()))?;
            let file = file_path.to_string_lossy();
            let module = compile(&source, Some(file.as_ref())).map_err(|error| {
                for diagnostic in error.diagnostics() {
                    eprintln!("{}", diagnostic.display());
                }
                anyhow::anyhow!("{error}")
            })?;
            function_count += module.user_function_count();
            let name = module_name(&path, &file_path)?;
            package
                .insert(name, module)
                .map_err(|error| anyhow::anyhow!("{error}"))?;
        }
        println!(
            "ok: vs3 package  modules={}  functions={function_count}",
            package.module_names().len()
        );
        return Ok(());
    }
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
/// Args support typed scalar, vector/matrix prefixes plus `j:` for JSON lists/maps.
pub fn cmd_vs3_run(
    path: PathBuf,
    call: String,
    args: Vec<String>,
    cooperative: bool,
    responses: Vec<String>,
) -> Result<()> {
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
    if cooperative {
        let mut task = module
            .start(&call, &values)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let responses = parse_args(&responses)?;
        let mut responses = responses.into_iter();
        let mut status = task.resume().map_err(|error| anyhow::anyhow!("{error}"))?;
        loop {
            match status {
                Vs3TaskStatus::Yielded(value) => {
                    println!("yield => {value}");
                    let Some(response) = responses.next() else {
                        println!("suspended (pass --resume VALUE to continue)");
                        break;
                    };
                    status = task
                        .resume_with(response)
                        .map_err(|error| anyhow::anyhow!("{error}"))?;
                }
                Vs3TaskStatus::Complete(value) => {
                    println!("=> {value}");
                    break;
                }
                Vs3TaskStatus::Waiting { .. } => unreachable!("CLI does not install a host"),
            }
        }
    } else {
        let value = module
            .call(&call, &values)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        println!("=> {value}");
    }
    Ok(())
}

/// Format a VS3 file in place while preserving its edition directive/comments.
pub fn cmd_vs3_fmt(path: PathBuf) -> Result<()> {
    if path.is_dir() {
        let files = vs3_files(&path)?;
        for file in &files {
            format_file(file)?;
        }
        println!(
            "formatted {} VS3 files under {}",
            files.len(),
            path.display()
        );
        return Ok(());
    }
    format_file(&path)?;
    println!("formatted {}", path.display());
    Ok(())
}

fn format_file(path: &std::path::Path) -> Result<()> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if detect_edition(&source) != velvet_script_vs3::Edition::Vs3 {
        bail!("{} is not marked `// @edition 3`", path.display());
    }
    let formatted = velvet_script_format::format_source(&source)
        .with_context(|| format!("format {}", path.display()))?;
    std::fs::write(path, formatted).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn vs3_files(root: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.with_context(|| format!("walk {}", root.display()))?;
        if !entry.file_type().is_file()
            || entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("vel")
        {
            continue;
        }
        let source = std::fs::read_to_string(entry.path())
            .with_context(|| format!("read {}", entry.path().display()))?;
        if detect_edition(&source) == velvet_script_vs3::Edition::Vs3 {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    if files.is_empty() {
        bail!("no edition-3 .vel files found under {}", root.display());
    }
    Ok(files)
}

fn module_name(root: &std::path::Path, file: &std::path::Path) -> Result<String> {
    let relative = file
        .strip_prefix(root)
        .with_context(|| format!("{} is outside {}", file.display(), root.display()))?;
    let mut components: Vec<String> = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let Some(last) = components.last_mut() else {
        bail!("empty module path for {}", file.display());
    };
    if let Some(stem) = std::path::Path::new(last)
        .file_stem()
        .and_then(|stem| stem.to_str())
    {
        *last = stem.to_string();
    }
    Ok(components.join("."))
}

fn parse_args(args: &[String]) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for a in args {
        if let Some(rest) = a.strip_prefix("i:") {
            let n: i64 = rest.parse().with_context(|| format!("bad int arg {a}"))?;
            out.push(Value::Int(n));
        } else if let Some(rest) = a.strip_prefix("b:") {
            let b = match rest {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => bail!("bad bool arg {a}"),
            };
            out.push(Value::Bool(b));
        } else if let Some(rest) = a.strip_prefix("f:") {
            let value: f64 = rest.parse().with_context(|| format!("bad float arg {a}"))?;
            out.push(Value::Float(value));
        } else if let Some(rest) = a.strip_prefix("s:") {
            out.push(Value::String(std::rc::Rc::from(rest)));
        } else if let Some(rest) = a.strip_prefix("v2:") {
            out.push(Value::Vec2(parse_float_components::<2>(rest, a)?));
        } else if let Some(rest) = a.strip_prefix("v3:") {
            out.push(Value::Vec3(parse_float_components::<3>(rest, a)?));
        } else if let Some(rest) = a.strip_prefix("v4:") {
            out.push(Value::Vec4(parse_float_components::<4>(rest, a)?));
        } else if let Some(rest) = a.strip_prefix("q:") {
            out.push(Value::Quat(parse_float_components::<4>(rest, a)?));
        } else if let Some(rest) = a.strip_prefix("m3:") {
            out.push(Value::Mat3(parse_float_components::<9>(rest, a)?));
        } else if let Some(rest) = a.strip_prefix("m4:") {
            out.push(Value::Mat4(parse_float_components::<16>(rest, a)?));
        } else if let Some(rest) = a.strip_prefix("j:") {
            let json: serde_json::Value =
                serde_json::from_str(rest).with_context(|| format!("bad JSON arg {a}"))?;
            out.push(json_to_value(json)?);
        } else if let Ok(n) = a.parse::<i64>() {
            out.push(Value::Int(n));
        } else if let Ok(value) = a.parse::<f64>() {
            out.push(Value::Float(value));
        } else if a == "true" {
            out.push(Value::Bool(true));
        } else if a == "false" {
            out.push(Value::Bool(false));
        } else if a == "null" {
            out.push(Value::Null);
        } else {
            out.push(Value::String(std::rc::Rc::from(a.as_str())));
        }
    }
    Ok(out)
}

fn parse_float_components<const N: usize>(input: &str, original: &str) -> Result<[f64; N]> {
    let values = input
        .split(',')
        .map(str::trim)
        .map(|component| {
            component
                .parse::<f64>()
                .with_context(|| format!("bad mathematical arg {original}"))
        })
        .collect::<Result<Vec<_>>>()?;
    values.try_into().map_err(|values: Vec<f64>| {
        anyhow::anyhow!(
            "mathematical arg {original} expects {N} components, got {}",
            values.len()
        )
    })
}

fn json_to_value(json: serde_json::Value) -> Result<Value> {
    Ok(match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(value) => Value::Bool(value),
        serde_json::Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                Value::Int(integer)
            } else {
                Value::Float(
                    value
                        .as_f64()
                        .ok_or_else(|| anyhow::anyhow!("JSON number is outside VS3 range"))?,
                )
            }
        }
        serde_json::Value::String(value) => Value::String(std::rc::Rc::from(value)),
        serde_json::Value::Array(values) => Value::list(
            values
                .into_iter()
                .map(json_to_value)
                .collect::<Result<Vec<_>>>()?,
        ),
        serde_json::Value::Object(entries) => Value::map(
            entries
                .into_iter()
                .map(|(key, value)| Ok((key, json_to_value(value)?)))
                .collect::<Result<Vec<_>>>()?,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_cli_arguments() {
        let values = parse_args(&[
            "f:1.5".into(),
            "null".into(),
            "v3:1,2,3".into(),
            "j:{\"name\":\"Ada\",\"scores\":[1,2]}".into(),
        ])
        .unwrap();
        assert_eq!(values[0], Value::Float(1.5));
        assert_eq!(values[1], Value::Null);
        assert_eq!(
            values[3]
                .get_index(&Value::String(std::rc::Rc::from("name")))
                .unwrap()
                .as_str(),
            Some("Ada")
        );
        assert_eq!(values[2], Value::Vec3([1.0, 2.0, 3.0]));
    }

    #[test]
    fn derives_stable_dotted_module_name() {
        let root = std::path::Path::new("project");
        let file = root.join("game").join("score.vel");
        assert_eq!(module_name(root, &file).unwrap(), "game.score");
    }
}
