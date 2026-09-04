//! Music-room: unlockable BGM/track catalog, mirror of [`crate::gallery::Gallery`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Music-room errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MusicRoomError {
    /// Track id unknown.
    #[error("music track not found: {0}")]
    NotFound(String),
}

/// Static definition of a music track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicTrack {
    /// Stable id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Asset path.
    pub path: String,
    /// Optional group (chapter, mood).
    #[serde(default)]
    pub group: String,
    /// Sort order.
    #[serde(default)]
    pub sort_order: i32,
    /// Starts unlocked.
    #[serde(default)]
    pub unlocked_by_default: bool,
}

impl MusicTrack {
    /// Create.
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

    /// Builder group.
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Builder sort.
    pub fn with_sort_order(mut self, order: i32) -> Self {
        self.sort_order = order;
        self
    }

    /// Builder default unlocked.
    pub fn default_unlocked(mut self) -> Self {
        self.unlocked_by_default = true;
        self
    }
}

/// Runtime music-room catalog + unlocks + play counts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MusicRoom {
    /// Tracks by id.
    #[serde(default)]
    pub tracks: BTreeMap<String, MusicTrack>,
    /// Unlocked ids.
    #[serde(default)]
    pub unlocked: BTreeSet<String>,
    /// Play counts per track.
    #[serde(default)]
    pub plays: BTreeMap<String, u32>,
}

/// File format for `music.json` sample packs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MusicFile {
    /// Tracks list.
    #[serde(default)]
    pub tracks: Vec<MusicTrack>,
}

impl MusicRoom {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load from a `music.json` document (list of tracks).
    pub fn from_json(text: &str) -> Result<Self, MusicRoomError> {
        let file: MusicFile = serde_json::from_str(text)
            .map_err(|e| MusicRoomError::NotFound(format!("json: {e}")))?;
        let mut m = Self::new();
        for t in file.tracks {
            m.register(t);
        }
        Ok(m)
    }

    /// Load from path.
    pub fn from_path(path: &std::path::Path) -> Result<Self, MusicRoomError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| MusicRoomError::NotFound(format!("io {}: {e}", path.display())))?;
        Self::from_json(&text)
    }

    /// Register; auto-unlock when flagged.
    pub fn register(&mut self, track: MusicTrack) {
        let id = track.id.clone();
        let auto = track.unlocked_by_default;
        self.tracks.insert(id.clone(), track);
        if auto {
            self.unlocked.insert(id);
        }
    }

    /// Unlock. Ok(true) = newly unlocked.
    pub fn unlock(&mut self, id: &str) -> Result<bool, MusicRoomError> {
        if !self.tracks.contains_key(id) {
            return Err(MusicRoomError::NotFound(id.into()));
        }
        Ok(self.unlocked.insert(id.into()))
    }

    /// Script-friendly unlock.
    pub fn unlock_if_known(&mut self, id: &str) -> bool {
        self.tracks.contains_key(id) && self.unlocked.insert(id.into())
    }

    /// Whether unlocked.
    pub fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains(id)
    }

    /// Record a listen (only counts when unlocked).
    pub fn record_play(&mut self, id: &str) -> bool {
        if !self.is_unlocked(id) {
            return false;
        }
        *self.plays.entry(id.to_string()).or_insert(0) += 1;
        true
    }

    /// Plays for a track.
    pub fn plays_of(&self, id: &str) -> u32 {
        self.plays.get(id).copied().unwrap_or(0)
    }

    /// Progress (unlocked, total).
    pub fn progress(&self) -> (usize, usize) {
        (self.unlocked.len(), self.tracks.len())
    }

    /// Unlocked tracks sorted.
    pub fn list_unlocked(&self) -> Vec<&MusicTrack> {
        let mut out: Vec<&MusicTrack> = self
            .unlocked
            .iter()
            .filter_map(|id| self.tracks.get(id))
            .collect();
        out.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.id.cmp(&b.id))
        });
        out
    }

    /// Export unlock set for saves.
    pub fn unlock_set(&self) -> BTreeSet<String> {
        self.unlocked.clone()
    }

    /// Merge unlocks from save.
    pub fn apply_unlock_set(&mut self, set: impl IntoIterator<Item = String>) {
        for id in set {
            self.unlocked.insert(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MusicRoom {
        let mut m = MusicRoom::new();
        m.register(MusicTrack::new("bgm01", "Night", "music/night.ogg").default_unlocked());
        m.register(MusicTrack::new("bgm02", "Rain", "music/rain.ogg"));
        m
    }

    #[test]
    fn unlock_and_play_counts() {
        let mut m = sample();
        assert!(m.is_unlocked("bgm01"));
        assert!(!m.is_unlocked("bgm02"));
        assert!(m.unlock("bgm02").unwrap());
        assert!(m.record_play("bgm02"));
        assert_eq!(m.plays_of("bgm02"), 1);
        assert!(!m.record_play("missing"));
    }

    #[test]
    fn progress_and_list() {
        let m = sample();
        assert_eq!(m.progress(), (1, 2));
        assert_eq!(m.list_unlocked().len(), 1);
    }

    #[test]
    fn unlock_set_roundtrip() {
        let mut m = sample();
        m.unlock_if_known("bgm02");
        let set = m.unlock_set();
        let mut m2 = MusicRoom::new();
        m2.register(MusicTrack::new("bgm01", "Night", "x"));
        m2.register(MusicTrack::new("bgm02", "Rain", "y"));
        m2.apply_unlock_set(set);
        assert!(m2.is_unlocked("bgm02"));
    }
}
