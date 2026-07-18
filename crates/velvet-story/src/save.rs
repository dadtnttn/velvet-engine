//! Versioned story save format (stable DTO, not raw runtime dumps).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::history::History;
use crate::runtime::StorySnapshot;
use crate::value::StoryValue;
use crate::variables::StoryVariables;

/// Current save format version.
pub const SAVE_FORMAT_VERSION: u32 = 1;

/// Save errors.
#[derive(Debug, Error)]
pub enum SaveError {
    /// I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization.
    #[error("serde: {0}")]
    Serde(String),
    /// Checksum mismatch.
    #[error("checksum mismatch (file may be corrupt)")]
    Checksum,
    /// Unsupported version.
    #[error("unsupported save version {found}, max {max}")]
    Version {
        /// Found.
        found: u32,
        /// Supported max.
        max: u32,
    },
    /// Slot missing.
    #[error("save slot not found: {0}")]
    NotFound(String),
}

/// Metadata shown in load menus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveMeta {
    /// Slot id e.g. `slot_1`, `quick`, `auto`.
    pub slot: String,
    /// Display title / chapter.
    pub title: String,
    /// Scene name at save.
    pub scene: String,
    /// Play time seconds.
    pub play_time_secs: f64,
    /// Unix timestamp.
    pub saved_at_unix: i64,
    /// Engine version string.
    pub engine_version: String,
    /// Optional thumbnail path or empty.
    pub thumbnail: String,
    /// Short preview text.
    pub preview: String,
}

/// Full save payload (versioned).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveGame {
    /// Format version.
    pub format_version: u32,
    /// Metadata.
    pub meta: SaveMeta,
    /// Variables play layer.
    pub variables: BTreeMap<String, StoryValue>,
    /// Persistent layer snapshot (optional merge on load).
    #[serde(default)]
    pub persistent: BTreeMap<String, StoryValue>,
    /// Runtime cursor.
    pub snapshot: StorySnapshot,
    /// History.
    #[serde(default)]
    pub history: History,
    /// Seen line keys for skip-read-only.
    #[serde(default)]
    pub seen_lines: Vec<String>,
    /// Unlocked gallery entry ids (optional; empty if unused).
    #[serde(default)]
    pub gallery_unlocked: BTreeSet<String>,
    /// Unlocked glossary term ids (optional; empty if unused).
    #[serde(default)]
    pub glossary_unlocked: BTreeSet<String>,
    /// Payload checksum (hex), computed over canonical body without this field.
    #[serde(default)]
    pub checksum: String,
}

impl SaveGame {
    /// Build from player state.
    pub fn from_parts(
        slot: impl Into<String>,
        title: impl Into<String>,
        vars: &StoryVariables,
        snapshot: StorySnapshot,
        history: History,
        seen_lines: Vec<String>,
        play_time_secs: f64,
        preview: impl Into<String>,
    ) -> Self {
        let meta = SaveMeta {
            slot: slot.into(),
            title: title.into(),
            scene: snapshot.scene.clone(),
            play_time_secs,
            saved_at_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            engine_version: env!("CARGO_PKG_VERSION").into(),
            thumbnail: String::new(),
            preview: preview.into(),
        };
        let mut save = Self {
            format_version: SAVE_FORMAT_VERSION,
            meta,
            variables: vars
                .play
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            persistent: vars
                .persistent
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            snapshot,
            history,
            seen_lines,
            gallery_unlocked: BTreeSet::new(),
            glossary_unlocked: BTreeSet::new(),
            checksum: String::new(),
        };
        save.checksum = save.compute_checksum();
        save
    }

    /// Attach gallery unlock set and recompute checksum.
    pub fn with_gallery_unlocks(mut self, unlocks: BTreeSet<String>) -> Self {
        self.gallery_unlocked = unlocks;
        self.checksum = self.compute_checksum();
        self
    }

    /// Attach glossary unlock set and recompute checksum.
    pub fn with_glossary_unlocks(mut self, unlocks: BTreeSet<String>) -> Self {
        self.glossary_unlocked = unlocks;
        self.checksum = self.compute_checksum();
        self
    }

    /// Attach both unlock sets and recompute checksum.
    pub fn with_collection_unlocks(
        mut self,
        gallery: BTreeSet<String>,
        glossary: BTreeSet<String>,
    ) -> Self {
        self.gallery_unlocked = gallery;
        self.glossary_unlocked = glossary;
        self.checksum = self.compute_checksum();
        self
    }

    fn body_for_checksum(&self) -> Result<Vec<u8>, SaveError> {
        let mut clone = self.clone();
        clone.checksum.clear();
        serde_json::to_vec(&clone).map_err(|e| SaveError::Serde(e.to_string()))
    }

    /// Compute sha256 hex of body.
    pub fn compute_checksum(&self) -> String {
        let bytes = self.body_for_checksum().unwrap_or_default();
        let hash = Sha256::digest(&bytes);
        hash.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// Validate version + checksum.
    pub fn validate(&self) -> Result<(), SaveError> {
        if self.format_version > SAVE_FORMAT_VERSION {
            return Err(SaveError::Version {
                found: self.format_version,
                max: SAVE_FORMAT_VERSION,
            });
        }
        let expected = self.compute_checksum();
        if !self.checksum.is_empty() && self.checksum != expected {
            return Err(SaveError::Checksum);
        }
        Ok(())
    }

    /// Serialize pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String, SaveError> {
        serde_json::to_string_pretty(self).map_err(|e| SaveError::Serde(e.to_string()))
    }

    /// Parse JSON.
    pub fn from_json(text: &str) -> Result<Self, SaveError> {
        let save: Self = serde_json::from_str(text).map_err(|e| SaveError::Serde(e.to_string()))?;
        save.validate()?;
        Ok(save)
    }

    /// Migrate older formats (v1 identity).
    pub fn migrate(mut self) -> Result<Self, SaveError> {
        if self.format_version == 0 {
            self.format_version = 1;
        }
        if self.format_version > SAVE_FORMAT_VERSION {
            return Err(SaveError::Version {
                found: self.format_version,
                max: SAVE_FORMAT_VERSION,
            });
        }
        self.checksum = self.compute_checksum();
        Ok(self)
    }
}

/// Filesystem save slot store.
#[derive(Debug, Clone)]
pub struct SaveStore {
    root: PathBuf,
}

impl SaveStore {
    /// Create store under directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Ensure directory exists.
    pub fn ensure_dir(&self) -> Result<(), SaveError> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    fn path_for(&self, slot: &str) -> PathBuf {
        self.root.join(format!("{slot}.velsave.json"))
    }

    /// Write save (atomic via temp + rename when possible).
    pub fn write(&self, save: &SaveGame) -> Result<(), SaveError> {
        self.ensure_dir()?;
        let mut save = save.clone();
        save.checksum = save.compute_checksum();
        let path = self.path_for(&save.meta.slot);
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, save.to_json_pretty()?)?;
        std::fs::rename(&tmp, &path)?;
        // Backup copy
        let bak = path.with_extension("bak.json");
        let _ = std::fs::copy(&path, bak);
        Ok(())
    }

    /// Read slot.
    pub fn read(&self, slot: &str) -> Result<SaveGame, SaveError> {
        let path = self.path_for(slot);
        if !path.exists() {
            return Err(SaveError::NotFound(slot.into()));
        }
        let text = std::fs::read_to_string(&path)?;
        match SaveGame::from_json(&text) {
            Ok(s) => Ok(s.migrate()?),
            Err(SaveError::Checksum) => {
                // Try backup
                let bak = path.with_extension("bak.json");
                if bak.exists() {
                    let text = std::fs::read_to_string(bak)?;
                    Ok(SaveGame::from_json(&text)?.migrate()?)
                } else {
                    Err(SaveError::Checksum)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// List slot metas.
    pub fn list(&self) -> Result<Vec<SaveMeta>, SaveError> {
        self.ensure_dir()?;
        let mut out = Vec::new();
        let rd = std::fs::read_dir(&self.root)?;
        for entry in rd.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if !name.contains(".velsave") || name.ends_with(".tmp") {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(save) = SaveGame::from_json(&text) {
                    out.push(save.meta);
                }
            }
        }
        out.sort_by(|a, b| b.saved_at_unix.cmp(&a.saved_at_unix));
        Ok(out)
    }

    /// Delete slot.
    pub fn delete(&self, slot: &str) -> Result<(), SaveError> {
        let path = self.path_for(slot);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Whether a slot file exists.
    pub fn exists(&self, slot: &str) -> bool {
        self.path_for(slot).exists()
    }

    /// Root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::StorySnapshot;

    #[test]
    fn roundtrip_checksum() {
        let vars = StoryVariables {
            play: [("trust".into(), StoryValue::Int(2))].into_iter().collect(),
            ..Default::default()
        };
        let snap = StorySnapshot {
            scene: "start".into(),
            op_index: 3,
            wait: crate::runtime::StoryWait::Line,
            visible: Default::default(),
            background: Some("bg.png".into()),
            music: None,
            call_stack: vec![],
        };
        let save = SaveGame::from_parts(
            "slot_1",
            "Test",
            &vars,
            snap,
            History::default(),
            vec!["start:0".into()],
            12.5,
            "Hello",
        );
        let json = save.to_json_pretty().unwrap();
        let back = SaveGame::from_json(&json).unwrap();
        assert_eq!(back.meta.slot, "slot_1");
        assert_eq!(back.variables.get("trust"), Some(&StoryValue::Int(2)));
    }

    #[test]
    fn store_write_read() {
        let dir = tempfile::tempdir().unwrap();
        let store = SaveStore::new(dir.path());
        let vars = StoryVariables::default();
        let snap = StorySnapshot {
            scene: "a".into(),
            op_index: 0,
            wait: crate::runtime::StoryWait::Ready,
            visible: Default::default(),
            background: None,
            music: None,
            call_stack: vec![],
        };
        let save = SaveGame::from_parts(
            "quick",
            "Q",
            &vars,
            snap,
            History::default(),
            vec![],
            0.0,
            "",
        );
        store.write(&save).unwrap();
        let loaded = store.read("quick").unwrap();
        assert_eq!(loaded.meta.slot, "quick");
    }

    #[test]
    fn gallery_glossary_unlocks_roundtrip() {
        let vars = StoryVariables::default();
        let snap = StorySnapshot {
            scene: "start".into(),
            op_index: 0,
            wait: crate::runtime::StoryWait::Ready,
            visible: Default::default(),
            background: None,
            music: None,
            call_stack: vec![],
        };
        let save = SaveGame::from_parts(
            "slot_2",
            "CG",
            &vars,
            snap,
            History::default(),
            vec![],
            1.0,
            "",
        )
        .with_collection_unlocks(
            ["cg01".into(), "cg02".into()].into_iter().collect(),
            ["term_a".into()].into_iter().collect(),
        );
        let json = save.to_json_pretty().unwrap();
        let back = SaveGame::from_json(&json).unwrap();
        assert!(back.gallery_unlocked.contains("cg01"));
        assert!(back.glossary_unlocked.contains("term_a"));
        // Older saves without fields still deserialize.
        let minimal = r#"{
            "format_version": 1,
            "meta": {
                "slot": "x",
                "title": "t",
                "scene": "s",
                "play_time_secs": 0.0,
                "saved_at_unix": 0,
                "engine_version": "0",
                "thumbnail": "",
                "preview": ""
            },
            "variables": {},
            "snapshot": {
                "scene": "s",
                "op_index": 0,
                "wait": "Ready",
                "visible": {},
                "background": null,
                "music": null,
                "call_stack": []
            },
            "checksum": ""
        }"#;
        let old = SaveGame::from_json(minimal).unwrap();
        assert!(old.gallery_unlocked.is_empty());
        assert!(old.glossary_unlocked.is_empty());
    }
}
