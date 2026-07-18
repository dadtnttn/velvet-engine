//! Top-level workspace commands: check, test, build, clean, fmt, assets, inspect.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

/// Run cargo check on the workspace (or project path).
pub fn cmd_check(path: PathBuf, release: bool) -> Result<()> {
    let status = cargo_in(&path, &["check", "--workspace"], release)?;
    if !status.success() {
        bail!("velvet check failed with status {status}");
    }
    println!("check ok");
    Ok(())
}

/// Run cargo test on the workspace.
pub fn cmd_test(path: PathBuf, release: bool, filter: Option<String>) -> Result<()> {
    let mut owned = vec!["test".to_string(), "--workspace".to_string()];
    if let Some(f) = filter {
        owned.push("--".into());
        owned.push(f);
    }
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let status = cargo_in(&path, &refs, release)?;
    if !status.success() {
        bail!("velvet test failed with status {status}");
    }
    println!("test ok");
    Ok(())
}

/// Run cargo build.
pub fn cmd_build(path: PathBuf, release: bool, package: Option<String>) -> Result<()> {
    let mut owned = vec!["build".to_string()];
    if let Some(pkg) = package {
        owned.push("-p".into());
        owned.push(pkg);
    } else {
        owned.push("--workspace".into());
    }
    let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    let status = cargo_in(&path, &refs, release)?;
    if !status.success() {
        bail!("velvet build failed with status {status}");
    }
    println!("build ok");
    Ok(())
}

/// cargo clean (workspace or target only).
pub fn cmd_clean(path: PathBuf) -> Result<()> {
    let status = cargo_in(&path, &["clean"], false)?;
    if !status.success() {
        bail!("velvet clean failed with status {status}");
    }
    println!("clean ok");
    Ok(())
}

/// Format Velvet Script files under a directory (and optionally rustfmt if available).
pub fn cmd_fmt(path: PathBuf, rust: bool) -> Result<()> {
    let root = if path.is_dir() {
        path
    } else if path.is_file() {
        cmd_script_fmt_one(&path)?;
        println!("formatted {}", path.display());
        return Ok(());
    } else {
        bail!("path not found: {}", path.display());
    };

    let mut n = 0usize;
    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) == Some("vel") {
            cmd_script_fmt_one(entry.path())?;
            n += 1;
        }
    }
    println!("formatted {n} .vel file(s) under {}", root.display());

    if rust {
        let status = cargo_in(&root, &["fmt", "--all"], false)?;
        if !status.success() {
            bail!("cargo fmt failed with status {status}");
        }
        println!("rustfmt ok");
    }
    Ok(())
}

fn cmd_script_fmt_one(path: &Path) -> Result<()> {
    let source =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let pretty =
        velvet_script_format::format_source(&source).map_err(|e| anyhow::anyhow!("{e}"))?;
    std::fs::write(path, pretty)?;
    Ok(())
}

/// List and summarize assets under a directory.
pub fn cmd_assets(path: PathBuf, pack_out: Option<PathBuf>) -> Result<()> {
    if !path.exists() {
        bail!("assets path not found: {}", path.display());
    }
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut total_bytes = 0u64;
    let mut files = 0usize;
    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        files += 1;
        if let Ok(meta) = entry.metadata() {
            total_bytes += meta.len();
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("(none)")
            .to_ascii_lowercase();
        *counts.entry(ext).or_default() += 1;
    }
    println!("assets root : {}", path.display());
    println!("files       : {files}");
    println!("bytes       : {total_bytes}");
    println!("by extension:");
    for (ext, n) in &counts {
        println!("  .{ext}: {n}");
    }
    if let Some(out) = pack_out {
        let pack = velvet_build::pack_directory(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
        let json = serde_json::to_string_pretty(&pack)?;
        std::fs::write(&out, json)?;
        println!("wrote pack manifest {}", out.display());
    }
    Ok(())
}

/// Inspect a project, script, or asset pack file.
pub fn cmd_inspect(path: PathBuf) -> Result<()> {
    if !path.exists() {
        bail!("not found: {}", path.display());
    }
    if path.is_dir() {
        let proj = path.join("velvet.project");
        if proj.exists() {
            return inspect_project(&proj);
        }
        let mut files = 0usize;
        for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                files += 1;
            }
        }
        println!("directory: {}", path.display());
        println!("files    : {files}");
        return Ok(());
    }

    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if name == "velvet.project" || ext == "project" {
        return inspect_project(&path);
    }

    match ext.as_str() {
        "vel" => {
            let source = std::fs::read_to_string(&path)?;
            let a = velvet_script_lsp::analyze(&source, Some(&path.to_string_lossy()));
            println!("script     : {}", path.display());
            println!("symbols    : {}", a.symbols.len());
            println!("diagnostics: {}", a.diagnostics.len());
            for s in a.symbols.iter().take(32) {
                println!(
                    "  [{}] {} @{}:{}",
                    s.kind,
                    s.name,
                    s.line + 1,
                    s.character + 1
                );
            }
            for d in &a.diagnostics {
                println!("  ! {} ({}:{})", d.message, d.line + 1, d.character + 1);
            }
            Ok(())
        }
        "json" => {
            let text = std::fs::read_to_string(&path)?;
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                println!("json file : {}", path.display());
                if let Some(obj) = v.as_object() {
                    let keys: Vec<_> = obj.keys().cloned().collect();
                    println!("keys      : {}", keys.join(", "));
                }
                println!("bytes     : {}", text.len());
            }
            Ok(())
        }
        _ => {
            let meta = std::fs::metadata(&path)?;
            println!("file      : {}", path.display());
            println!("bytes     : {}", meta.len());
            Ok(())
        }
    }
}

fn inspect_project(path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let project =
        velvet_project::VelvetProject::from_ron(&text).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("project    : {}", project.name);
    println!("identifier : {}", project.identifier);
    println!("version    : {}", project.version);
    println!("modules    : {}", project.modules.join(", "));
    println!("entry      : {}", project.entry_scene);
    let root = path.parent().unwrap_or(Path::new("."));
    let report = project.validate_at(root);
    if report.is_ok() {
        println!("validation : ok");
    } else {
        println!("validation : {} error(s)", report.error_count());
        for i in &report.issues {
            println!("  - {}", i.message);
        }
    }
    Ok(())
}

fn cargo_in(path: &Path, args: &[&str], release: bool) -> Result<std::process::ExitStatus> {
    let mut cmd = Command::new("cargo");
    cmd.args(args);
    if release {
        cmd.arg("--release");
    }
    let manifest = path.join("Cargo.toml");
    if manifest.exists() {
        cmd.arg("--manifest-path");
        cmd.arg(&manifest);
    } else if path.is_dir() {
        // Prefer cwd that has Cargo.toml walking up
        let mut cur = path.to_path_buf();
        loop {
            if cur.join("Cargo.toml").exists() {
                cmd.current_dir(&cur);
                break;
            }
            if !cur.pop() {
                cmd.current_dir(path);
                break;
            }
        }
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to run cargo {}", args.join(" ")))?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn assets_lists_files() {
        let dir = tempdir().unwrap();
        let assets = dir.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("a.png"), b"x").unwrap();
        std::fs::write(assets.join("b.ogg"), b"y").unwrap();
        cmd_assets(assets, None).unwrap();
    }

    #[test]
    fn inspect_vel_script() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("t.vel");
        let mut f = std::fs::File::create(&p).unwrap();
        writeln!(f, "function main() {{ return 1 }}").unwrap();
        cmd_inspect(p).unwrap();
    }

    #[test]
    fn fmt_single_file() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("t.vel");
        std::fs::write(&p, "function  main(  ){return 1}").unwrap();
        cmd_fmt(p.clone(), false).unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert!(out.contains("function"));
    }
}
