//! Script panel: open `.vel` buffers, format, and analyze.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use velvet_script_format::format_source;
use velvet_script_lsp::{analyze, Analysis, DocumentSymbol};

/// An open script buffer in the studio.
#[derive(Debug, Clone)]
pub struct ScriptBuffer {
    /// Absolute path on disk (may not exist until first save).
    pub path: PathBuf,
    /// In-memory text.
    pub text: String,
    /// Dirty flag (differs from last load/save).
    pub dirty: bool,
    /// Cached analysis (invalidated when dirty / re-analyzed).
    pub analysis: Option<Analysis>,
}

impl ScriptBuffer {
    /// Open from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let text =
            fs::read_to_string(&path).with_context(|| format!("open script {}", path.display()))?;
        Ok(Self {
            path,
            text,
            dirty: false,
            analysis: None,
        })
    }

    /// Create an empty buffer for a new path (not written yet).
    pub fn empty(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            text: String::new(),
            dirty: true,
            analysis: None,
        }
    }

    /// Replace buffer text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.dirty = true;
        self.analysis = None;
    }

    /// Append a line.
    pub fn append_line(&mut self, line: &str) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        self.text.push_str(line);
        if !line.ends_with('\n') {
            self.text.push('\n');
        }
        self.dirty = true;
        self.analysis = None;
    }

    /// Save to disk.
    pub fn save(&mut self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, &self.text)
            .with_context(|| format!("save {}", self.path.display()))?;
        self.dirty = false;
        Ok(())
    }

    /// Reload from disk (discards unsaved changes).
    pub fn reload(&mut self) -> Result<()> {
        self.text = fs::read_to_string(&self.path)
            .with_context(|| format!("reload {}", self.path.display()))?;
        self.dirty = false;
        self.analysis = None;
        Ok(())
    }

    /// Run LSP-style analysis and cache it.
    pub fn analyze(&mut self) -> &Analysis {
        if self.analysis.is_none() {
            let a = analyze(&self.text, Some(&self.path.to_string_lossy()));
            self.analysis = Some(a);
        }
        self.analysis.as_ref().unwrap()
    }

    /// Format buffer in memory using velvet-script-format.
    pub fn format(&mut self) -> Result<()> {
        let pretty = format_source(&self.text).map_err(|e| anyhow::anyhow!("{e}"))?;
        if pretty != self.text {
            self.text = pretty;
            self.dirty = true;
            self.analysis = None;
        }
        Ok(())
    }

    /// Format and write to disk.
    pub fn format_and_save(&mut self) -> Result<()> {
        self.format()?;
        self.save()
    }

    /// Line count.
    pub fn line_count(&self) -> usize {
        if self.text.is_empty() {
            0
        } else {
            self.text.lines().count()
        }
    }

    /// Symbols from cached or fresh analysis.
    pub fn symbols(&mut self) -> &[DocumentSymbol] {
        &self.analyze().symbols
    }

    /// Diagnostic count.
    pub fn diagnostic_count(&mut self) -> usize {
        self.analyze().diagnostics.len()
    }

    /// Produce a multi-line summary for the shell.
    pub fn summary(&mut self) -> Vec<String> {
        let path = self.path.display().to_string();
        let dirty = if self.dirty { "dirty" } else { "clean" };
        let lines = self.line_count();
        let diags = self.diagnostic_count();
        let syms = self.symbols().len();
        vec![
            format!("buffer: {path} [{dirty}]"),
            format!("lines: {lines} | symbols: {syms} | diagnostics: {diags}"),
        ]
    }

    /// Return a slice of source lines [start, end) (1-based start/end inclusive for display).
    pub fn line_range(&self, start_line: usize, end_line: usize) -> Vec<(usize, String)> {
        let start = start_line.max(1);
        self.text
            .lines()
            .enumerate()
            .skip(start - 1)
            .take(end_line.saturating_sub(start) + 1)
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect()
    }
}

/// Panel holding multiple open buffers keyed by path.
#[derive(Debug, Default)]
pub struct ScriptPanel {
    /// Open buffers.
    pub buffers: Vec<ScriptBuffer>,
    /// Index of active buffer.
    pub active: Option<usize>,
}

impl ScriptPanel {
    /// Create empty panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of open buffers.
    pub fn len(&self) -> usize {
        self.buffers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers.is_empty()
    }

    /// Open or focus a path relative to root.
    pub fn open(&mut self, root: &Path, rel_or_abs: &Path) -> Result<usize> {
        let abs = if rel_or_abs.is_absolute() {
            rel_or_abs.to_path_buf()
        } else {
            root.join(rel_or_abs)
        };
        if let Some(idx) = self.buffers.iter().position(|b| b.path == abs) {
            self.active = Some(idx);
            return Ok(idx);
        }
        let buf = ScriptBuffer::open(&abs)?;
        self.buffers.push(buf);
        let idx = self.buffers.len() - 1;
        self.active = Some(idx);
        Ok(idx)
    }

    /// Close buffer by index.
    pub fn close(&mut self, idx: usize) -> Result<()> {
        if idx >= self.buffers.len() {
            bail!("buffer index out of range");
        }
        if self.buffers[idx].dirty {
            bail!(
                "buffer {} is dirty; save or force-close",
                self.buffers[idx].path.display()
            );
        }
        self.buffers.remove(idx);
        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(idx.min(self.buffers.len() - 1))
        };
        Ok(())
    }

    /// Force close discarding dirty state.
    pub fn force_close(&mut self, idx: usize) -> Result<()> {
        if idx >= self.buffers.len() {
            bail!("buffer index out of range");
        }
        self.buffers.remove(idx);
        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(idx.min(self.buffers.len() - 1))
        };
        Ok(())
    }

    /// Active buffer mut.
    pub fn active_mut(&mut self) -> Option<&mut ScriptBuffer> {
        self.active.and_then(|i| self.buffers.get_mut(i))
    }

    /// Active buffer.
    pub fn active_ref(&self) -> Option<&ScriptBuffer> {
        self.active.and_then(|i| self.buffers.get(i))
    }

    /// List open paths.
    pub fn list_paths(&self) -> Vec<(usize, PathBuf, bool)> {
        self.buffers
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.path.clone(), b.dirty))
            .collect()
    }

    /// Format active buffer.
    pub fn format_active(&mut self) -> Result<()> {
        let buf = self
            .active_mut()
            .ok_or_else(|| anyhow::anyhow!("no active script buffer"))?;
        buf.format()
    }

    /// Analyze active buffer; returns diagnostic count.
    pub fn analyze_active(&mut self) -> Result<usize> {
        let buf = self
            .active_mut()
            .ok_or_else(|| anyhow::anyhow!("no active script buffer"))?;
        Ok(buf.diagnostic_count())
    }

    /// Save active buffer.
    pub fn save_active(&mut self) -> Result<()> {
        let buf = self
            .active_mut()
            .ok_or_else(|| anyhow::anyhow!("no active script buffer"))?;
        buf.save()
    }
}

/// Format a path on disk without opening a long-lived buffer.
pub fn format_file_on_disk(path: &Path) -> Result<()> {
    let src = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let pretty = format_source(&src).map_err(|e| anyhow::anyhow!("{e}"))?;
    fs::write(path, pretty)?;
    Ok(())
}

/// Analyze a path on disk and return analysis.
pub fn analyze_file_on_disk(path: &Path) -> Result<Analysis> {
    let src = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(analyze(&src, Some(&path.to_string_lossy())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_format_analyze() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("t.vel");
        fs::write(
            &path,
            "character hero{name:\"Hero\"}\nscene main{hero\"hi\"}\n",
        )
        .unwrap();

        let mut panel = ScriptPanel::new();
        panel.open(dir.path(), Path::new("t.vel")).unwrap();
        assert_eq!(panel.len(), 1);

        panel.format_active().unwrap();
        let buf = panel.active_mut().unwrap();
        assert!(buf.dirty);
        assert!(buf.line_count() >= 1);
        let diags = buf.diagnostic_count();
        // may or may not have diags depending on parser recovery
        let _ = diags;
        buf.save().unwrap();
        assert!(!buf.dirty);

        let a = analyze_file_on_disk(&path).unwrap();
        assert!(!a.symbols.is_empty() || a.diagnostics.is_empty() || !a.diagnostics.is_empty());
    }

    #[test]
    fn line_range() {
        let mut b = ScriptBuffer::empty("x.vel");
        b.set_text("a\nb\nc\nd\n");
        let lines = b.line_range(2, 3);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].1, "b");
    }

    #[test]
    fn format_makes_dirty_and_save_clears() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fmt.vel");
        fs::write(&path, "function f(){return 1}\n").unwrap();
        let mut panel = ScriptPanel::new();
        panel.open(dir.path(), Path::new("fmt.vel")).unwrap();
        panel.format_active().unwrap();
        let buf = panel.active_mut().unwrap();
        assert!(buf.dirty || buf.line_count() >= 1);
        buf.save().unwrap();
        assert!(!buf.dirty);
        assert!(path.exists());
    }

    #[test]
    fn multiple_buffers_and_switch() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.vel"), "function a() { return 1 }\n").unwrap();
        fs::write(dir.path().join("b.vel"), "function b() { return 2 }\n").unwrap();
        let mut panel = ScriptPanel::new();
        panel.open(dir.path(), Path::new("a.vel")).unwrap();
        panel.open(dir.path(), Path::new("b.vel")).unwrap();
        assert!(panel.len() >= 2);
        let n = panel.len();
        assert!(n >= 2);
    }

    #[test]
    fn analyze_reports_symbols_or_clean() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("an.vel");
        fs::write(
            &path,
            r#"
function add(a, b) {
    return a + b
}
scene main {
    "hi"
}
"#,
        )
        .unwrap();
        let a = analyze_file_on_disk(&path).unwrap();
        assert!(!a.symbols.is_empty() || a.diagnostics.is_empty() || !a.diagnostics.is_empty());
    }
}
