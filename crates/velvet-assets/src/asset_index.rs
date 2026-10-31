//! The list of files a package carries, written next to them at packaging time.
//!
//! Some package formats cannot list their own directories at run time (the Android NDK's
//! `AAssetDir` reports files but never sub-directories). The packager therefore writes
//! [`AssetIndex::FILE_NAME`] at the root of the packaged assets, and sources read it to answer
//! `is_dir` and directory listings.
//!
//! The format is UTF-8 text, one file per line: `<size in bytes>\t<path with / separators>`,
//! sorted by path. Lines starting with `#` are comments.

use std::collections::BTreeMap;

/// One entry of a directory listing.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexEntry {
    /// The entry's name inside its directory (no separators).
    pub name: String,
    /// True for a directory, false for a file.
    pub is_dir: bool,
}

/// Every packaged file and its size, by path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AssetIndex {
    files: BTreeMap<String, u64>,
}

impl AssetIndex {
    /// Name of the index file at the root of a package's assets.
    pub const FILE_NAME: &'static str = "velvet-asset-index.txt";

    /// Empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds (or replaces) a file. The path is normalised to `/` separators without a leading
    /// `./` or `/`.
    pub fn insert(&mut self, path: &str, size: u64) {
        let path = normalize(path);
        if !path.is_empty() {
            self.files.insert(path, size);
        }
    }

    /// Number of files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// No files.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Size of a file, when the index has it.
    pub fn file_size(&self, path: &str) -> Option<u64> {
        self.files.get(&normalize(path)).copied()
    }

    /// Whether the path is a file of the index.
    pub fn is_file(&self, path: &str) -> bool {
        self.files.contains_key(&normalize(path))
    }

    /// Whether the path is a directory holding at least one file (the root `""` always is).
    pub fn is_dir(&self, path: &str) -> bool {
        let dir = normalize(path);
        if dir.is_empty() {
            return true;
        }
        let prefix = format!("{dir}/");
        self.files
            .range(prefix.clone()..)
            .next()
            .is_some_and(|(p, _)| p.starts_with(&prefix))
    }

    /// Files and sub-directories directly inside `dir`, sorted, or `None` when `dir` is not a
    /// directory of the index.
    pub fn list_dir(&self, dir: &str) -> Option<Vec<IndexEntry>> {
        let dir = normalize(dir);
        if !self.is_dir(&dir) {
            return None;
        }
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let mut out: Vec<IndexEntry> = Vec::new();
        // The paths under a prefix are one contiguous run of the sorted map.
        let under = self
            .files
            .range(prefix.clone()..)
            .map(|(path, _)| path)
            .take_while(|path| path.starts_with(&prefix));
        for path in under {
            let rest = &path[prefix.len()..];
            let entry = match rest.split_once('/') {
                Some((first, _)) => IndexEntry {
                    name: first.to_string(),
                    is_dir: true,
                },
                None => IndexEntry {
                    name: rest.to_string(),
                    is_dir: false,
                },
            };
            out.push(entry);
        }
        out.sort();
        out.dedup();
        Some(out)
    }

    /// All files and their sizes, sorted by path.
    pub fn files(&self) -> impl Iterator<Item = (&str, u64)> {
        self.files.iter().map(|(p, s)| (p.as_str(), *s))
    }

    /// The index as the text written into packages.
    pub fn to_text(&self) -> String {
        let mut out = String::from("# velvet asset index: <size>\\t<path>\n");
        for (path, size) in &self.files {
            out.push_str(&format!("{size}\t{path}\n"));
        }
        out
    }

    /// Parses the text written by [`AssetIndex::to_text`]. Malformed lines are skipped.
    pub fn parse(text: &str) -> Self {
        let mut index = Self::new();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((size, path)) = line.split_once('\t') {
                if let Ok(size) = size.trim().parse() {
                    index.insert(path, size);
                }
            }
        }
        index
    }
}

fn normalize(path: &str) -> String {
    let mut s = path.replace('\\', "/");
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.to_string();
    }
    let s = s.trim_start_matches('/').trim_end_matches('/');
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AssetIndex {
        let mut index = AssetIndex::new();
        index.insert("scenes/arena.scene3d.json", 1200);
        index.insert("audio/music/title.ogg", 5_000_000);
        index.insert("audio/click.wav", 800);
        index.insert(r"data\fonts\ui.ttf", 64_000);
        index.insert("readme.txt", 10);
        index
    }

    #[test]
    fn text_round_trip() {
        let index = sample();
        let text = index.to_text();
        assert!(text.contains("1200\tscenes/arena.scene3d.json\n"));
        assert_eq!(AssetIndex::parse(&text), index);
        assert_eq!(
            AssetIndex::parse("# c\n\nbad line\n7\tok.txt\r\n").file_size("ok.txt"),
            Some(7)
        );
    }

    #[test]
    fn directories_come_from_file_paths() {
        let index = sample();
        assert!(index.is_dir(""));
        assert!(index.is_dir("audio"));
        assert!(index.is_dir("audio/music/"));
        assert!(index.is_dir("./data/fonts"));
        assert!(!index.is_dir("audio/click.wav"));
        assert!(!index.is_dir("aud"));
        assert!(index.is_file("data/fonts/ui.ttf"));
        assert_eq!(index.file_size("/audio/click.wav"), Some(800));

        let root = index.list_dir("").unwrap();
        let names: Vec<_> = root.iter().map(|e| (e.name.as_str(), e.is_dir)).collect();
        assert_eq!(
            names,
            vec![
                ("audio", true),
                ("data", true),
                ("readme.txt", false),
                ("scenes", true)
            ]
        );
        let audio = index.list_dir("audio").unwrap();
        assert_eq!(
            audio,
            vec![
                IndexEntry {
                    name: "click.wav".into(),
                    is_dir: false
                },
                IndexEntry {
                    name: "music".into(),
                    is_dir: true
                },
            ]
        );
        assert!(index.list_dir("nope").is_none());
    }
}
