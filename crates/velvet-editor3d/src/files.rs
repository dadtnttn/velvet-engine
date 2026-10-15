//! The project's files: a lazily read directory tree for the FileSystem dock and the lists of
//! models and scenes the inspector and the menus offer.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SKIP: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".playwright-mcp",
    "__pycache__",
    ".vs",
    ".idea",
    ".cache",
];

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(Default)]
pub struct FileTree {
    cache: HashMap<PathBuf, Vec<Entry>>,
}

impl FileTree {
    pub fn list(&mut self, dir: &Path) -> Vec<Entry> {
        self.cache
            .entry(dir.to_path_buf())
            .or_insert_with(|| read_dir(dir))
            .clone()
    }
    pub fn refresh(&mut self) {
        self.cache.clear();
    }
}

fn read_dir(dir: &Path) -> Vec<Entry> {
    let mut out: Vec<Entry> = std::fs::read_dir(dir)
        .map(|it| {
            it.filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = e.file_type().ok()?.is_dir();
                    if is_dir && (SKIP.contains(&name.as_str()) || name.starts_with('.')) {
                        return None;
                    }
                    Some(Entry {
                        name,
                        path: e.path(),
                        is_dir,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out
}

/// An icon for the file list.
pub fn icon(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return "🗀";
    }
    let n = name.to_ascii_lowercase();
    if n.ends_with(".scene3d.json") {
        return "🎬";
    }
    match n.rsplit('.').next().unwrap_or("") {
        "glb" | "gltf" => "◈",
        "png" | "jpg" | "jpeg" | "webp" | "avif" => "🖼",
        "ogg" | "wav" | "mp3" => "🔊",
        "mp4" | "ivf" | "webm" => "🎞",
        "rs" | "vel" | "vs3" | "wgsl" | "js" | "mjs" | "ts" | "py" => "📜",
        "md" | "txt" => "📄",
        "json" | "toml" | "ron" | "yaml" | "yml" => "⚙",
        _ => "🗋",
    }
}

/// Project-relative files under `root` whose names end with one of `suffixes` (forward slashes),
/// at most `cap` of them.
pub fn find(root: &Path, suffixes: &[&str], cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in read_dir(&dir) {
            if e.is_dir {
                stack.push(e.path);
            } else if suffixes
                .iter()
                .any(|s| e.name.to_ascii_lowercase().ends_with(s))
            {
                if let Ok(rel) = e.path.strip_prefix(root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                    if out.len() >= cap {
                        out.sort();
                        return out;
                    }
                }
            }
        }
    }
    out.sort();
    out
}

/// Files under `root` whose name holds `needle` (lowercase), at most `cap`.
pub fn search(root: &Path, needle: &str, cap: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in read_dir(&dir) {
            if e.name.to_lowercase().contains(needle) {
                out.push(e.path.clone());
                if out.len() >= cap {
                    return out;
                }
            }
            if e.is_dir {
                stack.push(e.path);
            }
        }
    }
    out.sort();
    out
}

/// `path` relative to `root` with forward slashes (or the full path).
pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
