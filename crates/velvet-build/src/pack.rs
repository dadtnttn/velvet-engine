//! Asset directory packing with checksums and exclude filters.

use std::fs;
use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use walkdir::WalkDir;

/// Pack errors.
#[derive(Debug, Error)]
pub enum PackError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Serde.
    #[error("serde: {0}")]
    Serde(String),
    /// Bad filter pattern.
    #[error("filter: {0}")]
    Filter(String),
}

/// One packed file entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackFile {
    /// Relative path with `/`.
    pub path: String,
    /// Size bytes.
    pub size: u64,
    /// Sha256 hex.
    pub sha256: String,
}

/// Asset pack manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPack {
    /// Version.
    pub version: u32,
    /// Files.
    pub files: IndexMap<String, PackFile>,
    /// Total size.
    pub total_size: u64,
    /// Exclude patterns applied (if any).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excludes: Vec<String>,
    /// Include-only patterns (if any).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub includes: Vec<String>,
}

impl AssetPack {
    /// To JSON.
    pub fn to_json_pretty(&self) -> Result<String, PackError> {
        serde_json::to_string_pretty(self).map_err(|e| PackError::Serde(e.to_string()))
    }

    /// File count.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Empty check.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Options for packing a directory.
#[derive(Debug, Clone, Default)]
pub struct PackOptions {
    /// Glob-like exclude patterns (matched against relative `/` paths).
    ///
    /// Supported syntax (simple, not full gitignore):
    /// - `*.tmp` — suffix/extension style
    /// - `**/*.psd` — any depth extension
    /// - `raw/` or `raw/**` — directory prefix
    /// - exact relative path `notes/todo.txt`
    pub exclude: Vec<String>,
    /// If non-empty, only paths matching at least one include pattern are kept
    /// (excludes still apply after).
    pub include: Vec<String>,
    /// Skip hidden files (name starts with `.`).
    pub skip_hidden: bool,
    /// Maximum file size to include (0 = unlimited).
    pub max_file_size: u64,
}

impl PackOptions {
    /// Exclude common editor / source junk.
    pub fn default_excludes() -> Self {
        Self {
            exclude: vec![
                "**/.DS_Store".into(),
                "**/Thumbs.db".into(),
                "**/*.tmp".into(),
                "**/*.bak".into(),
                "**/*~".into(),
                "**/.git/**".into(),
            ],
            include: vec![],
            skip_hidden: true,
            max_file_size: 0,
        }
    }
}

/// Walk directory and build pack manifest (does not compress; records checksums).
pub fn pack_directory(root: impl AsRef<Path>) -> Result<AssetPack, PackError> {
    pack_directory_with(root, &PackOptions::default())
}

/// Pack with filters.
pub fn pack_directory_with(
    root: impl AsRef<Path>,
    opts: &PackOptions,
) -> Result<AssetPack, PackError> {
    let root = root.as_ref();
    let mut pack = AssetPack {
        version: 1,
        excludes: opts.exclude.clone(),
        includes: opts.include.clone(),
        ..Default::default()
    };
    if !root.exists() {
        return Ok(pack);
    }
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if opts.skip_hidden
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
        {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if !opts.include.is_empty() && !opts.include.iter().any(|p| path_matches(&rel, p)) {
            continue;
        }
        if opts.exclude.iter().any(|p| path_matches(&rel, p)) {
            continue;
        }

        let data = fs::read(path)?;
        let size = data.len() as u64;
        if opts.max_file_size > 0 && size > opts.max_file_size {
            continue;
        }
        let hash = Sha256::digest(&data);
        let sha = hash.iter().map(|b| format!("{b:02x}")).collect::<String>();
        pack.total_size += size;
        pack.files.insert(
            rel.clone(),
            PackFile {
                path: rel,
                size,
                sha256: sha,
            },
        );
    }
    Ok(pack)
}

/// Simple glob-ish matcher for relative paths using `/`.
///
/// Patterns:
/// - `*` matches within a single path segment
/// - `**` matches across segments
/// - trailing `/` implies `/**`
pub fn path_matches(path: &str, pattern: &str) -> bool {
    let path = path.trim_start_matches("./");
    let mut pattern = pattern.trim_start_matches("./").to_string();
    if pattern.ends_with('/') {
        pattern.push_str("**");
    }
    match_glob(path, &pattern)
}

fn match_glob(path: &str, pattern: &str) -> bool {
    // Fast paths
    if pattern == "**" || pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return path == pattern || path.starts_with(&(pattern.to_string() + "/"));
    }

    // Convert simple glob to a recursive matcher
    let parts: Vec<&str> = pattern.split('/').collect();
    let segs: Vec<&str> = if path.is_empty() {
        vec![]
    } else {
        path.split('/').collect()
    };
    match_segments(&segs, &parts)
}

fn match_segments(segs: &[&str], pats: &[&str]) -> bool {
    if pats.is_empty() {
        return segs.is_empty();
    }
    if pats[0] == "**" {
        // match zero or more segments
        if pats.len() == 1 {
            return true;
        }
        // try consuming 0..n segments
        for i in 0..=segs.len() {
            if match_segments(&segs[i..], &pats[1..]) {
                return true;
            }
        }
        return false;
    }
    if segs.is_empty() {
        return false;
    }
    if !match_segment(segs[0], pats[0]) {
        return false;
    }
    match_segments(&segs[1..], &pats[1..])
}

fn match_segment(seg: &str, pat: &str) -> bool {
    if pat == "*" {
        return true;
    }
    if !pat.contains('*') {
        return seg == pat;
    }
    // simple * wildcards within segment
    let mut pit = pat.chars().peekable();
    let mut sit = seg.chars();
    while let Some(pc) = pit.next() {
        if pc == '*' {
            // greedy-ish: if pattern ends, ok
            let rest: String = pit.collect();
            if rest.is_empty() {
                return true;
            }
            let remaining: String = sit.collect();
            // try every split
            for i in 0..=remaining.len() {
                // only split on char boundaries
                if remaining.is_char_boundary(i) && match_segment(&remaining[i..], &rest) {
                    return true;
                }
            }
            return false;
        } else {
            match sit.next() {
                Some(sc) if sc == pc => {}
                _ => return false,
            }
        }
    }
    sit.next().is_none()
}

/// Copy directory recursively for export (honors optional excludes).
pub fn copy_dir(src: &Path, dst: &Path) -> Result<(), PackError> {
    copy_dir_with(src, dst, &PackOptions::default())
}

/// Copy with pack options (include/exclude/skip_hidden).
pub fn copy_dir_with(src: &Path, dst: &Path, opts: &PackOptions) -> Result<(), PackError> {
    fs::create_dir_all(dst)?;
    for entry in WalkDir::new(src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path.strip_prefix(src).unwrap_or(path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if entry.file_type().is_file() {
            if opts.skip_hidden
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
            {
                continue;
            }
            if !opts.include.is_empty() && !opts.include.iter().any(|p| path_matches(&rel_str, p)) {
                continue;
            }
            if opts.exclude.iter().any(|p| path_matches(&rel_str, p)) {
                continue;
            }
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

/// Ensure path exists as dir.
pub fn ensure_dir(path: impl AsRef<Path>) -> Result<PathBuf, PackError> {
    let p = path.as_ref().to_path_buf();
    fs::create_dir_all(&p)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/b.txt"), b"world").unwrap();
        let pack = pack_directory(dir.path()).unwrap();
        assert_eq!(pack.files.len(), 2);
        assert!(pack.total_size >= 10);
    }

    #[test]
    fn exclude_globs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("keep.png"), b"img").unwrap();
        fs::write(dir.path().join("drop.tmp"), b"tmp").unwrap();
        fs::create_dir_all(dir.path().join("raw")).unwrap();
        fs::write(dir.path().join("raw/x.psd"), b"psd").unwrap();

        let pack = pack_directory_with(
            dir.path(),
            &PackOptions {
                exclude: vec!["**/*.tmp".into(), "raw/**".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pack.len(), 1);
        assert!(pack.files.contains_key("keep.png"));
    }

    #[test]
    fn include_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.png"), b"1").unwrap();
        fs::write(dir.path().join("b.txt"), b"2").unwrap();
        let pack = pack_directory_with(
            dir.path(),
            &PackOptions {
                include: vec!["**/*.png".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pack.len(), 1);
    }

    #[test]
    fn path_match_cases() {
        assert!(path_matches("a/b.txt", "**/*.txt"));
        assert!(path_matches("raw/x.psd", "raw/**"));
        assert!(path_matches("notes/todo.txt", "notes/todo.txt"));
        assert!(!path_matches("a.png", "**/*.tmp"));
    }

    #[test]
    fn pack_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let pack = pack_directory(dir.path()).unwrap();
        assert_eq!(pack.files.len(), 0);
        assert_eq!(pack.total_size, 0);
    }

    #[test]
    fn pack_nested_and_size() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("a/b")).unwrap();
        fs::write(dir.path().join("a/b/c.bin"), vec![1u8; 100]).unwrap();
        fs::write(dir.path().join("root.txt"), b"hi").unwrap();
        let pack = pack_directory(dir.path()).unwrap();
        assert_eq!(pack.files.len(), 2);
        assert!(pack.total_size >= 102);
        assert!(pack
            .files
            .keys()
            .any(|k| k.contains("c.bin") || k.ends_with("c.bin")));
    }

    #[test]
    fn exclude_and_include_combined() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("keep.png"), b"1").unwrap();
        fs::write(dir.path().join("drop.png"), b"2").unwrap();
        fs::write(dir.path().join("notes.txt"), b"3").unwrap();
        let pack = pack_directory_with(
            dir.path(),
            &PackOptions {
                include: vec!["**/*.png".into()],
                exclude: vec!["drop.png".into()],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(pack.len(), 1);
        assert!(pack.files.contains_key("keep.png"));
    }

    #[test]
    fn ensure_dir_creates() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("x/y/z");
        let p = ensure_dir(&nested).unwrap();
        assert!(p.is_dir());
    }
}
