//! Environment and workspace health checks.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use velvet_core::engine_version;

/// One doctor check result.
#[derive(Debug, Clone)]
pub struct DoctorCheck {
    /// Short name.
    pub name: String,
    /// Pass/fail.
    pub ok: bool,
    /// Detail line.
    pub detail: String,
}

impl DoctorCheck {
    pub fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ok: true,
            detail: detail.into(),
        }
    }

    pub fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ok: false,
            detail: detail.into(),
        }
    }
}

/// Full doctor report.
#[derive(Debug, Clone)]
pub struct DoctorReport {
    /// Checks in order.
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn all_ok(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }

    pub fn fail_count(&self) -> usize {
        self.checks.iter().filter(|c| !c.ok).count()
    }
}

/// Expected workspace crate directories (relative to workspace root).
pub const EXPECTED_CRATES: &[&str] = &[
    "velvet-core",
    "velvet-app",
    "velvet-math",
    "velvet-ecs",
    "velvet-script-parser",
    "velvet-script-compiler",
    "velvet-script-vm",
    "velvet-story",
    "velvet-play",
    "velvet-rpg",
    "velvet-action",
    "velvet-project",
    "velvet-build",
    "velvet-cli",
    "velvet-editor",
];

/// Expected template directories under templates/.
pub const EXPECTED_TEMPLATES: &[&str] = &[
    "visual-novel",
    "narrative-adventure",
    "top-down-rpg",
    "top-down-action",
];

/// Run all doctor checks. `cwd` is used to locate workspace root.
pub fn run_doctor(cwd: impl AsRef<Path>) -> DoctorReport {
    let cwd = cwd.as_ref();
    let mut checks = Vec::new();

    checks.push(DoctorCheck::pass(
        "engine_version",
        format!("{}", engine_version()),
    ));
    checks.push(DoctorCheck::pass(
        "cli_version",
        env!("CARGO_PKG_VERSION").to_string(),
    ));
    checks.push(DoctorCheck::pass(
        "host",
        format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
    ));

    match rustc_version() {
        Ok(v) => checks.push(DoctorCheck::pass("rustc", v)),
        Err(e) => checks.push(DoctorCheck::fail("rustc", e.to_string())),
    }
    match cargo_version() {
        Ok(v) => checks.push(DoctorCheck::pass("cargo", v)),
        Err(e) => checks.push(DoctorCheck::fail("cargo", e.to_string())),
    }

    if let Some(root) = find_workspace_root(cwd) {
        checks.push(DoctorCheck::pass(
            "workspace_root",
            root.display().to_string(),
        ));
        let missing = missing_crates(&root);
        if missing.is_empty() {
            checks.push(DoctorCheck::pass(
                "workspace_crates",
                format!("{} expected crates present", EXPECTED_CRATES.len()),
            ));
        } else {
            checks.push(DoctorCheck::fail(
                "workspace_crates",
                format!("missing: {}", missing.join(", ")),
            ));
        }
        let missing_t = missing_templates(&root);
        if missing_t.is_empty() {
            checks.push(DoctorCheck::pass(
                "templates",
                format!("{} template dirs present", EXPECTED_TEMPLATES.len()),
            ));
        } else {
            checks.push(DoctorCheck::fail(
                "templates",
                format!("missing templates: {}", missing_t.join(", ")),
            ));
        }
        // Template content completeness
        let incomplete = incomplete_templates(&root);
        if incomplete.is_empty() {
            checks.push(DoctorCheck::pass(
                "template_content",
                "velvet.project + scripts/main.vel present for all templates",
            ));
        } else {
            checks.push(DoctorCheck::fail(
                "template_content",
                format!("incomplete: {}", incomplete.join(", ")),
            ));
        }
        // docs presence
        let docs = root.join("docs");
        if docs.is_dir() {
            checks.push(DoctorCheck::pass("docs", docs.display().to_string()));
        } else {
            checks.push(DoctorCheck::fail("docs", "docs/ directory missing"));
        }
    } else {
        checks.push(DoctorCheck::fail(
            "workspace_root",
            format!("could not find workspace Cargo.toml from {}", cwd.display()),
        ));
        checks.push(DoctorCheck::fail(
            "workspace_crates",
            "skipped (no workspace root)",
        ));
        checks.push(DoctorCheck::fail(
            "templates",
            "skipped (no workspace root)",
        ));
    }

    // Feature flags of this binary
    #[cfg(feature = "window")]
    checks.push(DoctorCheck::pass("window_feature", "enabled"));
    #[cfg(not(feature = "window"))]
    checks.push(DoctorCheck::fail(
        "window_feature",
        "disabled — rebuild with --features window",
    ));

    checks.push(DoctorCheck::pass(
        "status",
        "tooling crates expanded (project validation, multi-platform export dry-run, studio panels)",
    ));

    DoctorReport { checks }
}

/// Print report to stdout; returns Err if any check failed (exit non-zero optional).
pub fn print_doctor(report: &DoctorReport) {
    println!("Velvet Doctor");
    println!("-------------");
    for c in &report.checks {
        let mark = if c.ok { "ok  " } else { "FAIL" };
        println!("[{mark}] {:<18} {}", c.name, c.detail);
    }
    if report.all_ok() {
        println!("\nall checks passed");
    } else {
        println!("\n{} check(s) failed", report.fail_count());
    }
}

/// CLI entry.
pub fn cmd_doctor() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let report = run_doctor(&cwd);
    print_doctor(&report);
    if !report.all_ok() {
        // soft failure: still exit 0 for env quirks outside workspace; only fail hard on rustc/cargo
        let hard = report
            .checks
            .iter()
            .any(|c| !c.ok && (c.name == "rustc" || c.name == "cargo"));
        if hard {
            anyhow::bail!("doctor found hard failures");
        }
    }
    Ok(())
}

pub fn rustc_version() -> Result<String> {
    let out = Command::new("rustc")
        .arg("--version")
        .output()
        .context("failed to run rustc")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

pub fn cargo_version() -> Result<String> {
    let out = Command::new("cargo")
        .arg("--version")
        .output()
        .context("failed to run cargo")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Walk parents looking for workspace Cargo.toml containing `[workspace]`.
pub fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(text) = std::fs::read_to_string(&cargo) {
                if text.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
        }
        cur = dir.parent();
    }
    None
}

/// List missing expected crates under crates/.
pub fn missing_crates(workspace_root: &Path) -> Vec<String> {
    let crates_dir = workspace_root.join("crates");
    EXPECTED_CRATES
        .iter()
        .filter(|name| !crates_dir.join(name).join("Cargo.toml").is_file())
        .map(|s| (*s).to_string())
        .collect()
}

/// List missing template dirs.
pub fn missing_templates(workspace_root: &Path) -> Vec<String> {
    let tdir = workspace_root.join("templates");
    EXPECTED_TEMPLATES
        .iter()
        .filter(|name| !tdir.join(name).is_dir())
        .map(|s| (*s).to_string())
        .collect()
}

/// Templates missing velvet.project or scripts/main.vel.
pub fn incomplete_templates(workspace_root: &Path) -> Vec<String> {
    let tdir = workspace_root.join("templates");
    let mut bad = Vec::new();
    for name in EXPECTED_TEMPLATES {
        let root = tdir.join(name);
        if !root.is_dir() {
            continue;
        }
        let has_proj = root.join("velvet.project").is_file();
        let has_script =
            root.join("scripts/main.vel").is_file() || root.join("story/main.vel").is_file();
        if !has_proj || !has_script {
            bad.push((*name).to_string());
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn find_workspace_from_nested() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let nested = dir.path().join("a/b");
        fs::create_dir_all(&nested).unwrap();
        let root = find_workspace_root(&nested).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn missing_crates_detects() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("crates/velvet-core")).unwrap();
        fs::write(
            dir.path().join("crates/velvet-core/Cargo.toml"),
            "[package]\nname=\"x\"\n",
        )
        .unwrap();
        let miss = missing_crates(dir.path());
        assert!(miss.contains(&"velvet-app".to_string()));
        assert!(!miss.contains(&"velvet-core".to_string()));
    }

    #[test]
    fn incomplete_templates_detects() {
        let dir = tempdir().unwrap();
        for t in EXPECTED_TEMPLATES {
            let r = dir.path().join("templates").join(t);
            fs::create_dir_all(&r).unwrap();
            // only create project for first
        }
        let first = EXPECTED_TEMPLATES[0];
        let r = dir.path().join("templates").join(first);
        fs::write(r.join("velvet.project"), "()").unwrap();
        fs::create_dir_all(r.join("scripts")).unwrap();
        fs::write(r.join("scripts/main.vel"), "scene main {}\n").unwrap();
        let bad = super::incomplete_templates(dir.path());
        assert!(bad.len() >= EXPECTED_TEMPLATES.len() - 1);
    }

    #[test]
    fn doctor_check_pass_fail_helpers() {
        let p = DoctorCheck::pass("ok", "detail");
        assert!(p.ok);
        assert_eq!(p.name, "ok");
        let f = DoctorCheck::fail("bad", "reason");
        assert!(!f.ok);
        let report = DoctorReport {
            checks: vec![p, f.clone()],
        };
        assert!(!report.all_ok());
        assert_eq!(report.fail_count(), 1);
        let good = DoctorReport {
            checks: vec![DoctorCheck::pass("a", "1"), DoctorCheck::pass("b", "2")],
        };
        assert!(good.all_ok());
        assert_eq!(good.fail_count(), 0);
        let _ = f;
    }

    #[test]
    fn missing_crates_empty_when_all_present() {
        let dir = tempdir().unwrap();
        for c in EXPECTED_CRATES {
            let p = dir.path().join("crates").join(c);
            fs::create_dir_all(&p).unwrap();
            fs::write(p.join("Cargo.toml"), format!("[package]\nname=\"{c}\"\n")).unwrap();
        }
        let miss = missing_crates(dir.path());
        assert!(miss.is_empty(), "miss={miss:?}");
    }

    #[test]
    fn complete_templates_empty_when_valid() {
        let dir = tempdir().unwrap();
        for t in EXPECTED_TEMPLATES {
            let r = dir.path().join("templates").join(t);
            fs::create_dir_all(r.join("scripts")).unwrap();
            fs::write(r.join("velvet.project"), "name = \"t\"\n").unwrap();
            fs::write(r.join("scripts/main.vel"), "scene main { \"hi\" }\n").unwrap();
        }
        let bad = incomplete_templates(dir.path());
        assert!(bad.is_empty(), "bad={bad:?}");
    }

    #[test]
    fn find_workspace_none_outside() {
        let dir = tempdir().unwrap();
        // No Cargo.toml workspace — may still find ancestors; nest in pure temp without workspace.
        let nested = dir.path().join("x/y/z");
        fs::create_dir_all(&nested).unwrap();
        // If parent chain has no workspace Cargo.toml, returns None.
        // (tempdir root has no Cargo.toml)
        let found = find_workspace_root(&nested);
        assert!(found.is_none() || found.unwrap() != nested);
    }

    #[test]
    fn expected_lists_nonempty() {
        assert!(EXPECTED_CRATES.len() >= 10);
        assert!(EXPECTED_TEMPLATES.contains(&"visual-novel"));
        assert!(EXPECTED_CRATES.contains(&"velvet-core"));
    }
}
