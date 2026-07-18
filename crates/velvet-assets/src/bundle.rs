//! Asset bundle manifests (load/save as JSON or RON).

use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::path::VirtualPath;

/// Bundle serialize/load errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BundleError {
    /// I/O or parse failure.
    #[error("{0}")]
    Message(String),
}

/// One entry in a bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleEntry {
    /// Virtual path of the asset.
    pub path: VirtualPath,
    /// Optional type hint (e.g. `texture`, `audio`, `text`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Optional content hash (hex).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Dependency virtual paths.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<VirtualPath>,
    /// Tags for filtering.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl BundleEntry {
    /// Create a simple path-only entry.
    pub fn new(path: impl Into<VirtualPath>) -> Self {
        Self {
            path: path.into(),
            kind: None,
            hash: None,
            dependencies: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// Builder: set kind.
    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Builder: add dependency.
    pub fn with_dep(mut self, dep: impl Into<VirtualPath>) -> Self {
        self.dependencies.push(dep.into());
        self
    }
}

/// Asset bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AssetBundle {
    /// Bundle name / id.
    pub name: String,
    /// Format version.
    pub version: u32,
    /// Entries keyed by virtual path string for stable ordering.
    pub entries: IndexMap<String, BundleEntry>,
}

impl AssetBundle {
    /// Create empty named bundle.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: 1,
            entries: IndexMap::new(),
        }
    }

    /// Insert or replace an entry.
    pub fn insert(&mut self, entry: BundleEntry) {
        let key = entry.path.as_str().to_string();
        self.entries.insert(key, entry);
    }

    /// Get entry by virtual path.
    pub fn get(&self, path: &str) -> Option<&BundleEntry> {
        self.entries.get(path)
    }

    /// Remove entry.
    pub fn remove(&mut self, path: &str) -> Option<BundleEntry> {
        self.entries.shift_remove(path)
    }

    /// Entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// All virtual paths in insertion order.
    pub fn paths(&self) -> impl Iterator<Item = &str> + '_ {
        self.entries.keys().map(String::as_str)
    }

    /// Collect dependency closure for a root path (BFS, includes root if present).
    pub fn dependency_closure(&self, root: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_string()];
        while let Some(p) = stack.pop() {
            if out.iter().any(|x| x == &p) {
                continue;
            }
            if let Some(entry) = self.entries.get(&p) {
                out.push(p);
                for dep in &entry.dependencies {
                    stack.push(dep.as_str().to_string());
                }
            }
        }
        out
    }

    /// Serialize to pretty JSON.
    pub fn to_json(&self) -> Result<String, BundleError> {
        serde_json::to_string_pretty(self).map_err(|e| BundleError::Message(e.to_string()))
    }

    /// Parse JSON.
    pub fn from_json(text: &str) -> Result<Self, BundleError> {
        serde_json::from_str(text).map_err(|e| BundleError::Message(e.to_string()))
    }

    /// Serialize to pretty RON.
    pub fn to_ron(&self) -> Result<String, BundleError> {
        let pretty = ron::ser::PrettyConfig::new();
        ron::ser::to_string_pretty(self, pretty).map_err(|e| BundleError::Message(e.to_string()))
    }

    /// Parse RON.
    pub fn from_ron(text: &str) -> Result<Self, BundleError> {
        ron::from_str(text).map_err(|e| BundleError::Message(e.to_string()))
    }

    /// Write JSON to a filesystem path.
    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), BundleError> {
        let text = self.to_json()?;
        std::fs::write(path, text).map_err(|e| BundleError::Message(e.to_string()))
    }

    /// Load JSON from a filesystem path.
    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, BundleError> {
        let text =
            std::fs::read_to_string(path).map_err(|e| BundleError::Message(e.to_string()))?;
        Self::from_json(&text)
    }

    /// Write RON to a filesystem path.
    pub fn save_ron(&self, path: impl AsRef<Path>) -> Result<(), BundleError> {
        let text = self.to_ron()?;
        std::fs::write(path, text).map_err(|e| BundleError::Message(e.to_string()))
    }

    /// Load RON from a filesystem path.
    pub fn load_ron(path: impl AsRef<Path>) -> Result<Self, BundleError> {
        let text =
            std::fs::read_to_string(path).map_err(|e| BundleError::Message(e.to_string()))?;
        Self::from_ron(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let mut b = AssetBundle::new("demo");
        b.insert(
            BundleEntry::new("sprites/hero.png")
                .with_kind("texture")
                .with_dep("shaders/sprite.wgsl"),
        );
        b.insert(BundleEntry::new("shaders/sprite.wgsl").with_kind("text"));
        let json = b.to_json().unwrap();
        let b2 = AssetBundle::from_json(&json).unwrap();
        assert_eq!(b2.len(), 2);
        assert_eq!(b2.name, "demo");
        assert!(b2.get("sprites/hero.png").unwrap().dependencies.len() == 1);
    }

    #[test]
    fn ron_roundtrip() {
        let mut b = AssetBundle::new("ron_pack");
        b.insert(BundleEntry::new("a.txt"));
        let text = b.to_ron().unwrap();
        let b2 = AssetBundle::from_ron(&text).unwrap();
        assert_eq!(b2.get("a.txt").map(|e| e.path.as_str()), Some("a.txt"));
    }

    #[test]
    fn dependency_closure_order() {
        let mut b = AssetBundle::new("g");
        b.insert(BundleEntry::new("root").with_dep("mid"));
        b.insert(BundleEntry::new("mid").with_dep("leaf"));
        b.insert(BundleEntry::new("leaf"));
        let c = b.dependency_closure("root");
        assert!(c.contains(&"root".into()));
        assert!(c.contains(&"mid".into()));
        assert!(c.contains(&"leaf".into()));
    }
}
