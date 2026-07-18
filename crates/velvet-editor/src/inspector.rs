//! Selection inspector: file metadata, script symbols, and project fields.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use velvet_script_lsp::{analyze, Analysis, DocumentSymbol};

use crate::project_browser::load_project_info;

/// What the inspector is currently showing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Selection {
    /// Nothing selected.
    #[default]
    None,
    /// A filesystem path (file or directory).
    File(PathBuf),
    /// A symbol inside a script file.
    Symbol {
        /// Containing file.
        file: PathBuf,
        /// Symbol name.
        name: String,
        /// Kind (function, scene, character, …).
        kind: String,
    },
    /// Project root metadata.
    Project,
}

/// File metadata snapshot for the inspector pane.
#[derive(Debug, Clone)]
pub struct FileMeta {
    /// Absolute or project-relative path as given.
    pub path: PathBuf,
    /// Exists on disk.
    pub exists: bool,
    /// Is a directory.
    pub is_dir: bool,
    /// Size in bytes (0 for dirs / missing).
    pub size: u64,
    /// Extension without leading dot.
    pub extension: Option<String>,
    /// Modified time as Unix seconds if available.
    pub modified_unix: Option<u64>,
    /// Read-only bit when known.
    pub readonly: bool,
}

impl FileMeta {
    /// Collect metadata for a path.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let meta = fs::metadata(&path).ok();
        let exists = meta.is_some();
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let readonly = meta
            .as_ref()
            .map(|m| m.permissions().readonly())
            .unwrap_or(false);
        let modified_unix = meta
            .as_ref()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        Self {
            path,
            exists,
            is_dir,
            size,
            extension,
            modified_unix,
            readonly,
        }
    }
}

/// Human-readable inspector report lines.
#[derive(Debug, Clone, Default)]
pub struct InspectorReport {
    /// Title line.
    pub title: String,
    /// Key/value style detail lines.
    pub lines: Vec<String>,
    /// Symbols when inspecting a `.vel` file.
    pub symbols: Vec<DocumentSymbol>,
    /// Diagnostics count when analyzing scripts.
    pub diagnostic_count: usize,
}

/// Build an inspector report for the current selection relative to a project root.
pub fn inspect(root: &Path, selection: &Selection) -> Result<InspectorReport> {
    match selection {
        Selection::None => Ok(InspectorReport {
            title: "Nothing selected".into(),
            lines: vec!["Use `select <path>` or `select-symbol <file> <name>`.".into()],
            ..Default::default()
        }),
        Selection::Project => inspect_project(root),
        Selection::File(path) => inspect_file(root, path),
        Selection::Symbol { file, name, kind } => inspect_symbol(root, file, name, kind),
    }
}

fn inspect_project(root: &Path) -> Result<InspectorReport> {
    let mut lines = vec![format!("root: {}", root.display())];
    if let Some(p) = load_project_info(root)? {
        lines.push(format!("name: {}", p.name));
        lines.push(format!("identifier: {}", p.identifier));
        lines.push(format!("version: {}", p.version));
        lines.push(format!("modules: {}", p.modules.join(", ")));
        lines.push(format!("entry_scene: {}", p.entry_scene));
        lines.push(format!("assets_dir: {}", p.assets_dir));
        lines.push(format!("window.title: {}", p.window.title));
        lines.push(format!(
            "window.size: {}x{}",
            p.window.width, p.window.height
        ));
        Ok(InspectorReport {
            title: format!("Project: {}", p.name),
            lines,
            ..Default::default()
        })
    } else {
        lines.push("no velvet.project found".into());
        Ok(InspectorReport {
            title: "Project (unconfigured)".into(),
            lines,
            ..Default::default()
        })
    }
}

fn inspect_file(root: &Path, path: &Path) -> Result<InspectorReport> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let meta = FileMeta::from_path(&abs);
    let rel = abs.strip_prefix(root).unwrap_or(&abs);
    let mut report = InspectorReport {
        title: format!("File: {}", rel.display()),
        lines: vec![
            format!("path: {}", abs.display()),
            format!("exists: {}", meta.exists),
            format!("is_dir: {}", meta.is_dir),
            format!("size: {} bytes", meta.size),
            format!(
                "extension: {}",
                meta.extension.as_deref().unwrap_or("(none)")
            ),
            format!("readonly: {}", meta.readonly),
        ],
        ..Default::default()
    };
    if let Some(t) = meta.modified_unix {
        report.lines.push(format!("modified_unix: {t}"));
    }

    if meta.exists && !meta.is_dir && meta.extension.as_deref() == Some("vel") {
        let src = fs::read_to_string(&abs).with_context(|| format!("read {}", abs.display()))?;
        let analysis = analyze_script(&src, &abs);
        report.diagnostic_count = analysis.diagnostics.len();
        report.symbols = analysis.symbols.clone();
        report.lines.push(format!(
            "symbols: {} | diagnostics: {}",
            analysis.symbols.len(),
            analysis.diagnostics.len()
        ));
        for sym in &analysis.symbols {
            report.lines.push(format!(
                "  - {} ({}) @{}:{}",
                sym.name,
                sym.kind,
                sym.line + 1,
                sym.character + 1
            ));
        }
        for d in analysis.diagnostics.iter().take(20) {
            report.lines.push(format!(
                "  ! {}:{}: {}",
                d.line + 1,
                d.character + 1,
                d.message
            ));
        }
    } else if meta.exists && !meta.is_dir {
        // Show a short preview for small text-ish files
        if meta.size > 0 && meta.size < 8 * 1024 {
            if let Ok(text) = fs::read_to_string(&abs) {
                let preview: String = text.chars().take(240).collect();
                let one_line = preview.replace('\n', "\\n");
                report.lines.push(format!("preview: {one_line}"));
            }
        }
    }
    Ok(report)
}

fn inspect_symbol(root: &Path, file: &Path, name: &str, kind: &str) -> Result<InspectorReport> {
    let abs = if file.is_absolute() {
        file.to_path_buf()
    } else {
        root.join(file)
    };
    let mut report = InspectorReport {
        title: format!("Symbol: {name} ({kind})"),
        lines: vec![
            format!("file: {}", abs.display()),
            format!("name: {name}"),
            format!("kind: {kind}"),
        ],
        ..Default::default()
    };
    if abs.exists() {
        let src = fs::read_to_string(&abs).with_context(|| format!("read {}", abs.display()))?;
        let analysis = analyze_script(&src, &abs);
        report.symbols = analysis.symbols.clone();
        if let Some(sym) = analysis.symbols.iter().find(|s| s.name == name) {
            report
                .lines
                .push(format!("location: {}:{}", sym.line + 1, sym.character + 1));
            report.lines.push(format!("kind_resolved: {}", sym.kind));
            // Extract surrounding source lines for context
            let lines: Vec<&str> = src.lines().collect();
            let line_idx = sym.line as usize;
            let start = line_idx.saturating_sub(2);
            let end = (line_idx + 3).min(lines.len());
            for (i, line) in lines[start..end].iter().enumerate() {
                let mark = if start + i == line_idx { ">" } else { " " };
                report
                    .lines
                    .push(format!("{mark} {:>4} | {}", start + i + 1, line));
            }
        } else {
            report
                .lines
                .push(format!("symbol `{name}` not found in analysis"));
            report.diagnostic_count = analysis.diagnostics.len();
        }
    } else {
        report.lines.push("file does not exist".into());
    }
    Ok(report)
}

fn analyze_script(src: &str, path: &Path) -> Analysis {
    analyze(src, Some(&path.to_string_lossy()))
}

/// Resolve a symbol name inside a file (first match).
pub fn find_symbol_in_file(root: &Path, file: &Path, name: &str) -> Result<Option<DocumentSymbol>> {
    let abs = if file.is_absolute() {
        file.to_path_buf()
    } else {
        root.join(file)
    };
    if !abs.exists() {
        return Ok(None);
    }
    let src = fs::read_to_string(&abs)?;
    let a = analyze(&src, Some(&abs.to_string_lossy()));
    Ok(a.symbols.into_iter().find(|s| s.name == name))
}

/// Print a report to stdout.
pub fn print_report(report: &InspectorReport) {
    println!("{}", report.title);
    for line in &report.lines {
        println!("  {line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn inspect_vel_file_lists_symbols() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("main.vel");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"character hero {{ name: "Hero" }}
scene main {{
    hero "hi"
}}
"#
        )
        .unwrap();
        let report = inspect(dir.path(), &Selection::File(path)).unwrap();
        assert!(report
            .symbols
            .iter()
            .any(|s| s.name == "hero" || s.kind.contains("character") || s.name == "main"));
        assert!(!report.lines.is_empty());
    }

    #[test]
    fn file_meta_missing() {
        let m = FileMeta::from_path("/definitely/not/a/real/path/xyz.vel");
        assert!(!m.exists);
        assert_eq!(m.size, 0);
    }

    #[test]
    fn inspect_none() {
        let r = inspect(Path::new("."), &Selection::None).unwrap();
        assert!(r.title.contains("Nothing"));
    }
}
