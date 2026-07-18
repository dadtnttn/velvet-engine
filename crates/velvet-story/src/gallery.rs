//! Image gallery unlock tracking for CGs and stills.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Gallery-related errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GalleryError {
    /// Entry id is unknown to the catalog.
    #[error("gallery entry not found: {0}")]
    NotFound(String),
}

/// Static definition of a gallery image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GalleryEntry {
    /// Stable id used in unlock sets and saves.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Asset path or handle key.
    pub path: String,
    /// Optional group (chapter, character, etc.).
    #[serde(default)]
    pub group: String,
    /// Sort order within the gallery UI (lower first).
    #[serde(default)]
    pub sort_order: i32,
    /// Whether the entry starts unlocked.
    #[serde(default)]
    pub unlocked_by_default: bool,
}

impl GalleryEntry {
    /// Create a simple gallery entry.
    pub fn new(id: impl Into<String>, title: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            path: path.into(),
            group: String::new(),
            sort_order: 0,
            unlocked_by_default: false,
        }
    }

    /// Builder: set group.
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Builder: set sort order.
    pub fn with_sort_order(mut self, sort_order: i32) -> Self {
        self.sort_order = sort_order;
        self
    }

    /// Builder: unlock by default.
    pub fn default_unlocked(mut self) -> Self {
        self.unlocked_by_default = true;
        self
    }
}

/// Runtime gallery catalog + unlock state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Gallery {
    /// Known entries keyed by id (insertion order not required; sorted for UI).
    #[serde(default)]
    pub entries: BTreeMap<String, GalleryEntry>,
    /// Unlocked entry ids.
    #[serde(default)]
    pub unlocked: BTreeSet<String>,
}

/// File format for `gallery.json` sample packs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GalleryFile {
    /// Entries list.
    #[serde(default)]
    pub entries: Vec<GalleryEntry>,
}

impl Gallery {
    /// Empty gallery.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from a `gallery.json` document (list of entries).
    pub fn from_json(text: &str) -> Result<Self, GalleryError> {
        let file: GalleryFile =
            serde_json::from_str(text).map_err(|e| GalleryError::NotFound(format!("json: {e}")))?;
        let mut g = Self::new();
        g.register_all(file.entries);
        Ok(g)
    }

    /// Load from path.
    pub fn from_path(path: &std::path::Path) -> Result<Self, GalleryError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| GalleryError::NotFound(format!("io {}: {e}", path.display())))?;
        Self::from_json(&text)
    }

    /// Register an entry; auto-unlocks if `unlocked_by_default`.
    pub fn register(&mut self, entry: GalleryEntry) {
        let id = entry.id.clone();
        let auto = entry.unlocked_by_default;
        self.entries.insert(id.clone(), entry);
        if auto {
            self.unlocked.insert(id);
        }
    }

    /// Register many entries.
    pub fn register_all<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = GalleryEntry>,
    {
        for e in entries {
            self.register(e);
        }
    }

    /// Unlock by id. Returns `Ok(true)` if newly unlocked.
    pub fn unlock(&mut self, id: &str) -> Result<bool, GalleryError> {
        if !self.entries.contains_key(id) {
            return Err(GalleryError::NotFound(id.into()));
        }
        Ok(self.unlocked.insert(id.into()))
    }

    /// Unlock if present; ignores unknown ids (script-friendly).
    pub fn unlock_if_known(&mut self, id: &str) -> bool {
        self.entries.contains_key(id) && self.unlocked.insert(id.into())
    }

    /// Whether unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(id)
    }

    /// Whether the id exists in the catalog.
    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    /// Lookup definition.
    pub fn get(&self, id: &str) -> Option<&GalleryEntry> {
        self.entries.get(id)
    }

    /// Sorted list of unlocked entries (by sort_order, then id).
    pub fn list_unlocked(&self) -> Vec<&GalleryEntry> {
        let mut out: Vec<&GalleryEntry> = self
            .unlocked
            .iter()
            .filter_map(|id| self.entries.get(id))
            .collect();
        out.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.id.cmp(&b.id))
        });
        out
    }

    /// All entries sorted for the gallery browser (locked ones included).
    pub fn list_all(&self) -> Vec<(&GalleryEntry, bool)> {
        let mut ids: Vec<&String> = self.entries.keys().collect();
        ids.sort_by(|a, b| {
            let ea = &self.entries[*a];
            let eb = &self.entries[*b];
            ea.sort_order
                .cmp(&eb.sort_order)
                .then_with(|| ea.id.cmp(&eb.id))
        });
        ids.into_iter()
            .map(|id| {
                let e = &self.entries[id];
                (e, self.unlocked.contains(id.as_str()))
            })
            .collect()
    }

    /// Entries in a group, unlocked only.
    pub fn list_unlocked_in_group(&self, group: &str) -> Vec<&GalleryEntry> {
        self.list_unlocked()
            .into_iter()
            .filter(|e| e.group == group)
            .collect()
    }

    /// Number of unlocked / total for progress UI.
    pub fn progress(&self) -> (usize, usize) {
        (self.unlocked.len(), self.entries.len())
    }

    /// Export unlock set for embedding in saves.
    pub fn unlock_set(&self) -> BTreeSet<String> {
        self.unlocked.clone()
    }

    /// Merge unlocks from a save (union). Unknown ids are kept for forward-compat.
    pub fn apply_unlock_set(&mut self, set: impl IntoIterator<Item = String>) {
        for id in set {
            self.unlocked.insert(id);
        }
    }

    /// Clear all unlocks (debug / new game+ options).
    pub fn clear_unlocks(&mut self) {
        self.unlocked.clear();
        // Re-apply defaults
        for e in self.entries.values() {
            if e.unlocked_by_default {
                self.unlocked.insert(e.id.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gallery() -> Gallery {
        let mut g = Gallery::new();
        g.register(
            GalleryEntry::new("cg01", "Sunset", "cg/sunset.png")
                .with_group("ch1")
                .with_sort_order(2),
        );
        g.register(
            GalleryEntry::new("cg00", "Title", "cg/title.png")
                .with_group("ch1")
                .with_sort_order(1)
                .default_unlocked(),
        );
        g.register(
            GalleryEntry::new("cg02", "Harbor", "cg/harbor.png")
                .with_group("ch2")
                .with_sort_order(1),
        );
        g
    }

    #[test]
    fn default_unlocked_and_progress() {
        let g = sample_gallery();
        assert!(g.is_unlocked("cg00"));
        assert!(!g.is_unlocked("cg01"));
        assert_eq!(g.progress(), (1, 3));
    }

    #[test]
    fn unlock_and_list_sorted() {
        let mut g = sample_gallery();
        assert!(g.unlock("cg01").unwrap());
        assert!(!g.unlock("cg01").unwrap()); // already unlocked
        let list = g.list_unlocked();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, "cg00");
        assert_eq!(list[1].id, "cg01");
    }

    #[test]
    fn unlock_unknown_errors() {
        let mut g = sample_gallery();
        assert!(matches!(
            g.unlock("missing"),
            Err(GalleryError::NotFound(_))
        ));
        assert!(!g.unlock_if_known("missing"));
    }

    #[test]
    fn group_filter_and_list_all() {
        let mut g = sample_gallery();
        g.unlock("cg02").unwrap();
        let ch2 = g.list_unlocked_in_group("ch2");
        assert_eq!(ch2.len(), 1);
        assert_eq!(ch2[0].id, "cg02");
        let all = g.list_all();
        assert_eq!(all.len(), 3);
        // sort: cg00 (order 1), cg02 (order 1 ch2... wait cg02 sort 1, cg01 sort 2)
        // Actually: cg00 order 1, cg02 order 1, cg01 order 2 — tie-break by id: cg00, cg02, cg01
        assert_eq!(all[0].0.id, "cg00");
        assert!(all[0].1);
    }

    #[test]
    fn save_unlock_roundtrip() {
        let mut g = sample_gallery();
        g.unlock("cg01").unwrap();
        let set = g.unlock_set();
        let mut g2 = sample_gallery();
        g2.clear_unlocks();
        assert!(!g2.is_unlocked("cg01"));
        g2.apply_unlock_set(set);
        assert!(g2.is_unlocked("cg00"));
        assert!(g2.is_unlocked("cg01"));
    }

    #[test]
    fn serde_roundtrip() {
        let mut g = sample_gallery();
        g.unlock("cg01").unwrap();
        let json = serde_json::to_string(&g).unwrap();
        let back: Gallery = serde_json::from_str(&json).unwrap();
        assert_eq!(back.unlocked, g.unlocked);
        assert_eq!(back.entries.len(), 3);
    }
}
