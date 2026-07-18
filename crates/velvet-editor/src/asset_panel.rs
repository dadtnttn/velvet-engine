//! Asset browser panel: list project assets by type with filters.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

/// High-level asset kind derived from extension / location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AssetKind {
    /// Images (png, jpg, jpeg, webp, bmp, gif).
    Image,
    /// Audio (ogg, wav, mp3, flac).
    Audio,
    /// Velvet Script.
    Script,
    /// Scene / prefab data (ron, json scene files).
    Scene,
    /// Fonts.
    Font,
    /// Localization catalogs.
    Locale,
    /// Other / unknown.
    Other,
}

impl AssetKind {
    /// Classify by file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "tga" => Self::Image,
            "ogg" | "wav" | "mp3" | "flac" | "opus" => Self::Audio,
            "vel" => Self::Script,
            "ron" | "scene" | "prefab" => Self::Scene,
            "ttf" | "otf" | "woff" | "woff2" => Self::Font,
            "po" | "pot" => Self::Locale,
            _ => Self::Other,
        }
    }

    /// Display name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Script => "script",
            Self::Scene => "scene",
            Self::Font => "font",
            Self::Locale => "locale",
            Self::Other => "other",
        }
    }

    /// Parse filter token.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "image" | "images" | "img" | "texture" => Some(Self::Image),
            "audio" | "sound" | "sfx" | "music" => Some(Self::Audio),
            "script" | "scripts" | "vel" => Some(Self::Script),
            "scene" | "scenes" => Some(Self::Scene),
            "font" | "fonts" => Some(Self::Font),
            "locale" | "loc" | "i18n" => Some(Self::Locale),
            "other" | "misc" => Some(Self::Other),
            _ => None,
        }
    }
}

/// One asset entry.
#[derive(Debug, Clone)]
pub struct AssetEntry {
    /// Absolute path.
    pub path: PathBuf,
    /// Path relative to project root.
    pub relative: PathBuf,
    /// Kind.
    pub kind: AssetKind,
    /// Size bytes.
    pub size: u64,
    /// Extension without dot.
    pub extension: String,
}

/// Filter for the asset panel.
#[derive(Debug, Clone, Default)]
pub struct AssetFilter {
    /// Restrict to kinds (empty = all).
    pub kinds: Vec<AssetKind>,
    /// Substring match on relative path (case-insensitive).
    pub name_contains: Option<String>,
    /// Extension filter without dot.
    pub extension: Option<String>,
    /// Max entries to return (0 = unlimited).
    pub limit: usize,
}

impl AssetFilter {
    /// Match only one kind.
    pub fn kind(kind: AssetKind) -> Self {
        Self {
            kinds: vec![kind],
            ..Default::default()
        }
    }

    /// Whether an entry matches.
    pub fn matches(&self, entry: &AssetEntry) -> bool {
        if !self.kinds.is_empty() && !self.kinds.contains(&entry.kind) {
            return false;
        }
        if let Some(ref ext) = self.extension {
            if !entry.extension.eq_ignore_ascii_case(ext) {
                return false;
            }
        }
        if let Some(ref needle) = self.name_contains {
            let hay = entry.relative.to_string_lossy().to_ascii_lowercase();
            if !hay.contains(&needle.to_ascii_lowercase()) {
                return false;
            }
        }
        true
    }
}

/// Scan project for assets under common roots.
pub fn scan_assets(root: &Path) -> Result<Vec<AssetEntry>> {
    let mut out = Vec::new();
    let candidates = [
        root.join("assets"),
        root.join("scripts"),
        root.join("scenes"),
        root.join("locale"),
        root.join("locales"),
        root.join("content"),
    ];
    let mut roots: Vec<PathBuf> = candidates.into_iter().filter(|p| p.exists()).collect();
    // Always include root scripts/vel if present at top level
    if roots.is_empty() {
        roots.push(root.to_path_buf());
    } else {
        // Also scan root for velvet.project-adjacent scripts
        for sub in ["scripts", "scenes"] {
            let p = root.join(sub);
            if p.exists() && !roots.iter().any(|r| r == &p) {
                roots.push(p);
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    for scan_root in roots {
        for entry in WalkDir::new(&scan_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path().to_path_buf();
            if !seen.insert(path.clone()) {
                continue;
            }
            // Skip hidden / target-like
            let rel_str = path.to_string_lossy();
            if rel_str.contains("target") || rel_str.contains(".git") {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let kind = classify_path(&path, &ext);
            let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            out.push(AssetEntry {
                path,
                relative,
                kind,
                size,
                extension: ext,
            });
        }
    }
    out.sort_by(|a, b| a.relative.cmp(&b.relative));
    Ok(out)
}

fn classify_path(path: &Path, ext: &str) -> AssetKind {
    if ext == "json" {
        let s = path.to_string_lossy().to_ascii_lowercase();
        if s.contains("locale") || s.contains("i18n") || s.contains("lang") {
            return AssetKind::Locale;
        }
    }
    AssetKind::from_extension(ext)
}

/// Apply filter to a full asset list.
pub fn filter_assets(assets: &[AssetEntry], filter: &AssetFilter) -> Vec<AssetEntry> {
    let mut out: Vec<AssetEntry> = assets
        .iter()
        .filter(|e| filter.matches(e))
        .cloned()
        .collect();
    if filter.limit > 0 && out.len() > filter.limit {
        out.truncate(filter.limit);
    }
    out
}

/// Count assets by kind.
pub fn count_by_kind(assets: &[AssetEntry]) -> BTreeMap<AssetKind, usize> {
    let mut map = BTreeMap::new();
    for a in assets {
        *map.entry(a.kind).or_insert(0) += 1;
    }
    map
}

/// Total size of a slice of assets.
pub fn total_size(assets: &[AssetEntry]) -> u64 {
    assets.iter().map(|a| a.size).sum()
}

/// Format a listing for the shell.
pub fn format_listing(assets: &[AssetEntry], show_size: bool) -> Vec<String> {
    assets
        .iter()
        .map(|a| {
            if show_size {
                format!(
                    "[{:>6}] {:>8} B  {}",
                    a.kind.as_str(),
                    a.size,
                    a.relative.display()
                )
            } else {
                format!("[{:>6}]  {}", a.kind.as_str(), a.relative.display())
            }
        })
        .collect()
}

/// Print summary + listing.
pub fn print_assets(root: &Path, filter: &AssetFilter) -> Result<usize> {
    let all = scan_assets(root)?;
    let filtered = filter_assets(&all, filter);
    let counts = count_by_kind(&all);
    println!(
        "Assets under {} ({} total scanned)",
        root.display(),
        all.len()
    );
    for (k, n) in &counts {
        println!("  {}: {n}", k.as_str());
    }
    if !filter.kinds.is_empty() || filter.name_contains.is_some() || filter.extension.is_some() {
        println!("Filtered: {} shown", filtered.len());
    }
    for line in format_listing(&filtered, true) {
        println!("  {line}");
    }
    Ok(filtered.len())
}

/// Parse a free-form filter string from the shell: `images`, `audio foo`, `ext:png`, etc.
pub fn parse_filter_args(args: &[&str]) -> AssetFilter {
    let mut filter = AssetFilter::default();
    for arg in args {
        if let Some(ext) = arg.strip_prefix("ext:") {
            filter.extension = Some(ext.to_string());
            continue;
        }
        if let Some(lim) = arg.strip_prefix("limit:") {
            filter.limit = lim.parse().unwrap_or(0);
            continue;
        }
        if let Some(kind) = AssetKind::parse(arg) {
            filter.kinds.push(kind);
            continue;
        }
        // free text
        filter.name_contains = Some(arg.to_string());
    }
    filter
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn scan_and_filter() {
        let dir = tempdir().unwrap();
        let assets = dir.path().join("assets");
        fs::create_dir_all(assets.join("bg")).unwrap();
        fs::write(assets.join("bg/room.png"), b"fakepng").unwrap();
        fs::write(assets.join("theme.ogg"), b"fakeogg").unwrap();
        fs::create_dir_all(dir.path().join("scripts")).unwrap();
        fs::write(dir.path().join("scripts/main.vel"), b"scene main {}\n").unwrap();

        let list = scan_assets(dir.path()).unwrap();
        assert!(list.len() >= 3);

        let images = filter_assets(&list, &AssetFilter::kind(AssetKind::Image));
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].kind, AssetKind::Image);

        let scripts = filter_assets(&list, &AssetFilter::kind(AssetKind::Script));
        assert_eq!(scripts.len(), 1);

        let by = count_by_kind(&list);
        assert_eq!(*by.get(&AssetKind::Audio).unwrap_or(&0), 1);
    }

    #[test]
    fn parse_filter_args_kinds() {
        let f = parse_filter_args(&["images", "ext:png", "limit:10"]);
        assert_eq!(f.kinds, vec![AssetKind::Image]);
        assert_eq!(f.extension.as_deref(), Some("png"));
        assert_eq!(f.limit, 10);
    }
}
