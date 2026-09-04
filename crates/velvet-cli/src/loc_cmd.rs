//! Localization CLI commands.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_story::prelude::*;

/// Extract strings from a .vel file into JSON (or PO with --format).
pub fn cmd_loc_extract(path: PathBuf, out: PathBuf, format: &str) -> Result<()> {
    let source = std::fs::read_to_string(&path)?;
    let cat = velvet_build::extract_from_source(&source, &path.to_string_lossy());
    let body = match format {
        "po" | "pot" => cat.to_po(),
        "properties" | "props" => cat.to_properties(),
        _ => cat.to_json_pretty().map_err(|e| anyhow::anyhow!("{e}"))?,
    };
    std::fs::write(&out, body)?;
    println!(
        "wrote {} ({} keys, format={format})",
        out.display(),
        cat.entries.len()
    );
    Ok(())
}

/// Validate target locale against source catalog.
pub fn cmd_loc_validate(source: PathBuf, target: PathBuf) -> Result<()> {
    let s_text = std::fs::read_to_string(&source)?;
    let t_text = std::fs::read_to_string(&target)?;
    let s = velvet_build::load_catalog_auto(&s_text, &source.to_string_lossy(), "source")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let t = velvet_build::load_catalog_auto(&t_text, &target.to_string_lossy(), "target")
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let errs = velvet_build::validate_catalog(&s, &t);
    if errs.is_empty() {
        println!("ok");
    } else {
        for e in &errs {
            println!("error: {e}");
        }
        bail!("{} localization issue(s)", errs.len());
    }
    Ok(())
}

/// Convert catalog between formats.
pub fn cmd_loc_convert(input: PathBuf, out: PathBuf, locale: &str) -> Result<()> {
    let text = std::fs::read_to_string(&input)?;
    let cat = velvet_build::load_catalog_auto(&text, &input.to_string_lossy(), locale)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let out_s = out.to_string_lossy().to_ascii_lowercase();
    let body = if out_s.ends_with(".po") || out_s.ends_with(".pot") {
        cat.to_po()
    } else if out_s.ends_with(".properties") || out_s.ends_with(".txt") {
        cat.to_properties()
    } else {
        cat.to_json_pretty().map_err(|e| anyhow::anyhow!("{e}"))?
    };
    std::fs::write(&out, body)?;
    println!(
        "converted {} -> {} ({} keys)",
        input.display(),
        out.display(),
        cat.entries.len()
    );
    Ok(())
}

/// Extract story loc keys (scene:op:dialogue / choice) to `tl/en/strings.json` map
/// and optional target language scaffold.
pub fn cmd_loc_extract_story(
    path: PathBuf,
    project: Option<PathBuf>,
    lang: Option<String>,
) -> Result<()> {
    let source =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let program = load_program_from_source(
        &source,
        Some(&path.to_string_lossy()),
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("story"),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let cat = extract_loc_keys(&program);
    let root = project.unwrap_or_else(|| {
        path.parent()
            .and_then(|p| {
                if p.file_name().and_then(|n| n.to_str()) == Some("scripts") {
                    p.parent().map(|x| x.to_path_buf())
                } else {
                    Some(p.to_path_buf())
                }
            })
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let empty = TranslationTable::new();
    let en =
        write_tl_scaffold(&root, &program, "en", &empty).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("ok: wrote EN catalog {} ({} keys)", en.display(), cat.len());
    if let Some(lang) = lang {
        if lang != "en" {
            // If target already exists, keep existing translations where present.
            let mut table = TranslationTable::new();
            if let Ok(existing) = load_tl_table(&root, &lang) {
                table = existing;
            }
            for e in &cat.entries {
                table
                    .entry(e.key.clone())
                    .or_insert_with(|| e.source.clone());
            }
            let path = write_tl_scaffold(&root, &program, &lang, &table)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("ok: wrote/updated tl/{lang} at {}", path.display());
        }
    }
    println!("layout: {}/tl/<lang>/strings.json", root.display());
    Ok(())
}

/// Validate story `tl/<lang>/strings.json` maps against the script source.
/// Reports missing keys (shown in English at runtime) and stale extra keys.
/// Fails when any checked language misses keys.
pub fn cmd_loc_validate_story(project: PathBuf, lang: Option<String>) -> Result<()> {
    let vel = story_entry(&project)?;
    let source =
        std::fs::read_to_string(&vel).with_context(|| format!("read {}", vel.display()))?;
    let program = load_program_from_source(
        &source,
        Some(&vel.to_string_lossy()),
        vel.file_stem().and_then(|s| s.to_str()).unwrap_or("story"),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let cat = extract_loc_keys(&program);
    let want: Vec<String> = cat.entries.iter().map(|e| e.key.clone()).collect();
    let langs = match lang {
        Some(l) => vec![l.trim().to_ascii_lowercase()],
        None => tl_lang_dirs(&project)
            .into_iter()
            .filter(|l| l != "en")
            .collect(),
    };
    if langs.is_empty() {
        println!("no non-English tl/ languages under {}", project.display());
        return Ok(());
    }
    let mut bad = 0usize;
    for l in &langs {
        let path = project.join("tl").join(l).join("strings.json");
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let map: std::collections::BTreeMap<String, String> =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let missing: Vec<&str> = want
            .iter()
            .map(String::as_str)
            .filter(|k| !map.contains_key(*k))
            .collect();
        let stale: Vec<&String> = map.keys().filter(|k| !want.contains(k)).collect();
        if missing.is_empty() && stale.is_empty() {
            println!("ok: tl/{l} ({} keys)", want.len());
        } else {
            bad += 1;
            println!("tl/{l}: {} missing, {} stale", missing.len(), stale.len());
            for k in missing {
                println!("  missing: {k}");
            }
            for k in stale {
                println!("  stale: {k}");
            }
        }
    }
    if bad > 0 {
        bail!("{bad} language(s) need translation updates");
    }
    Ok(())
}

/// Story entry `scripts/main.vel` under a project dir (or the file itself).
fn story_entry(project: &PathBuf) -> Result<PathBuf> {
    use std::path::Path;
    let p = Path::new(project);
    if p.is_file() {
        return Ok(p.to_path_buf());
    }
    for cand in ["scripts/main.vel", "main.vel"] {
        let vel = p.join(cand);
        if vel.is_file() {
            return Ok(vel);
        }
    }
    bail!(
        "no story found under {} (expected scripts/main.vel)",
        project.display()
    );
}

/// Non-English `tl/*` language dirs, sorted.
fn tl_lang_dirs(project: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let tl = project.join("tl");
    if let Ok(rd) = std::fs::read_dir(tl) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                if let Some(n) = e.file_name().to_str() {
                    let n = n.to_ascii_lowercase();
                    if n != "en" && !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
        }
    }
    out.sort();
    out
}

/// Print available languages for a project (en + tl/*).
pub fn cmd_loc_langs(project: PathBuf) -> Result<()> {
    let mut langs = vec!["en".to_string()];
    let tl = project.join("tl");
    if tl.is_dir() {
        for e in std::fs::read_dir(&tl)?.flatten() {
            if e.path().is_dir() {
                if let Some(n) = e.file_name().to_str() {
                    let n = n.to_ascii_lowercase();
                    if n != "en" && !langs.iter().any(|l| l == &n) {
                        langs.push(n);
                    }
                }
            }
        }
    }
    langs.sort();
    for l in langs {
        println!("{l}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn extract_story_writes_tl_layout() {
        let dir = tempdir().unwrap();
        let scripts = dir.path().join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        let vel = scripts.join("main.vel");
        fs::write(
            &vel,
            r#"
character h { name: "H" }
scene main {
    h "Hello friend"
    choice {
        "Yes" { jump end }
        "No" { jump end }
    }
}
scene end { "Ending: Bye" }
"#,
        )
        .unwrap();
        cmd_loc_extract_story(vel, Some(dir.path().to_path_buf()), Some("es".into())).unwrap();
        assert!(dir.path().join("tl/en/strings.json").is_file());
        assert!(dir.path().join("tl/es/strings.json").is_file());
        let es_text = fs::read_to_string(dir.path().join("tl/es/strings.json")).unwrap();
        assert!(es_text.contains("Hello"));
    }

    #[test]
    fn extract_json_and_po() {
        let dir = tempdir().unwrap();
        let vel = dir.path().join("s.vel");
        fs::write(&vel, r#"aria "Hello""#).unwrap();
        let json = dir.path().join("out.json");
        cmd_loc_extract(vel.clone(), json.clone(), "json").unwrap();
        assert!(json.exists());
        let po = dir.path().join("out.po");
        cmd_loc_extract(vel, po.clone(), "po").unwrap();
        let text = fs::read_to_string(po).unwrap();
        assert!(text.contains("msgid"));
    }

    #[test]
    fn extract_multi_line_story() {
        let dir = tempdir().unwrap();
        let vel = dir.path().join("story.vel");
        fs::write(
            &vel,
            r##"
character n { name: "N" }
scene start {
    n "Hello"
    n "World"
    choice {
        "Yes" { jump end }
        "No" { jump end }
    }
}
scene end { "Done" end }
"##,
        )
        .unwrap();
        let json = dir.path().join("loc.json");
        cmd_loc_extract(vel, json.clone(), "json").unwrap();
        let text = fs::read_to_string(json).unwrap();
        for expected in ["Hello", "World", "Yes", "No", "Done"] {
            assert!(text.contains(expected), "missing {expected} in {text}");
        }
    }

    #[test]
    fn validate_story_flags_missing_and_stale_keys() {
        let dir = tempdir().unwrap();
        let scripts = dir.path().join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(
            scripts.join("main.vel"),
            "scene main {\n \"Hello\"\n \"World\"\n}\n",
        )
        .unwrap();
        // complete fr passes
        cmd_loc_extract_story(
            scripts.join("main.vel"),
            Some(dir.path().to_path_buf()),
            Some("fr".into()),
        )
        .unwrap();
        cmd_loc_validate_story(dir.path().to_path_buf(), Some("fr".into())).unwrap();
        // drop one key -> missing; add a bogus one -> stale
        let fr_path = dir.path().join("tl/fr/strings.json");
        let mut map: std::collections::BTreeMap<String, String> =
            serde_json::from_str(&fs::read_to_string(&fr_path).unwrap()).unwrap();
        let first = map.keys().next().cloned().unwrap();
        map.remove(&first);
        map.insert("stale:key".into(), "x".into());
        fs::write(&fr_path, serde_json::to_string_pretty(&map).unwrap()).unwrap();
        assert!(cmd_loc_validate_story(dir.path().to_path_buf(), Some("fr".into())).is_err());
        // no langs at all is fine
        let empty = tempdir().unwrap();
        fs::create_dir_all(empty.path().join("scripts")).unwrap();
        fs::write(
            empty.path().join("scripts/main.vel"),
            "scene main {\n \"Hi\"\n}\n",
        )
        .unwrap();
        cmd_loc_validate_story(empty.path().to_path_buf(), None).unwrap();
    }

    #[test]
    fn extract_missing_file_errors() {
        let dir = tempdir().unwrap();
        let r = cmd_loc_extract(
            dir.path().join("missing.vel"),
            dir.path().join("out.json"),
            "json",
        );
        assert!(r.is_err());
    }
}
