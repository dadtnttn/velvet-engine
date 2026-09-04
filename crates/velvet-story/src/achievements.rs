//! Story achievements: gallery / music / ending triggers with save persistence.
//!
//! Authors map Steam-style achievement ids to in-story events in
//! `achievements.json`. [`AchievementState`] auto-grants on gallery unlocks,
//! music unlocks and named endings, persists the earned set in saves, and
//! syncs into [`velvet_build::SteamHook`] — which is a local mirror today
//! and lights up against the real Steamworks client when linked.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// What earns an achievement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "on", rename_all = "snake_case")]
pub enum AchievementTrigger {
    /// Gallery entry unlocked.
    Gallery {
        /// Entry id.
        id: String,
    },
    /// Music-room track unlocked.
    Music {
        /// Track id.
        id: String,
    },
    /// Named ending reached (`end "id"`).
    Ending {
        /// Ending id.
        id: String,
    },
}

/// One achievement definition (authored in `achievements.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementDef {
    /// Stable id (`FIRST_ENDING`, also the Steam API name).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Display description.
    #[serde(default)]
    pub description: String,
    /// What earns it.
    pub trigger: AchievementTrigger,
}

impl AchievementDef {
    /// Whether this definition fires for the given event.
    pub fn fires_on(&self, kind: &str, id: &str) -> bool {
        match &self.trigger {
            AchievementTrigger::Gallery { id: want } => kind == "gallery" && want == id,
            AchievementTrigger::Music { id: want } => kind == "music" && want == id,
            AchievementTrigger::Ending { id: want } => kind == "ending" && want == id,
        }
    }
}

/// File format for `achievements.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementFile {
    /// Definitions.
    #[serde(default)]
    pub achievements: Vec<AchievementDef>,
}

/// Runtime achievement catalog + earned set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementState {
    /// Definitions by id.
    #[serde(default)]
    pub defs: Vec<AchievementDef>,
    /// Earned ids (monotonic; persisted in saves).
    #[serde(default)]
    pub earned: BTreeSet<String>,
}

impl AchievementState {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load definitions from `achievements.json` text (earned starts empty).
    pub fn from_json(text: &str) -> Result<Self, String> {
        let file: AchievementFile =
            serde_json::from_str(text).map_err(|e| format!("achievements json: {e}"))?;
        let mut state = Self::new();
        for def in file.achievements {
            state.register(def);
        }
        Ok(state)
    }

    /// Load from path.
    pub fn from_path(path: &std::path::Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("io {}: {e}", path.display()))?;
        Self::from_json(&text)
    }

    /// Register a definition (replaces same id).
    pub fn register(&mut self, def: AchievementDef) {
        if let Some(slot) = self.defs.iter_mut().find(|d| d.id == def.id) {
            *slot = def;
        } else {
            self.defs.push(def);
        }
    }

    /// Grant for a gallery/music/ending event. Returns newly earned ids.
    pub fn grant_for(&mut self, kind: &str, id: &str) -> Vec<String> {
        let mut newly = Vec::new();
        for def in &self.defs {
            if def.fires_on(kind, id) && self.earned.insert(def.id.clone()) {
                newly.push(def.id.clone());
            }
        }
        newly
    }

    /// Whether earned.
    pub fn is_earned(&self, id: &str) -> bool {
        self.earned.contains(id)
    }

    /// Earned ids (for Steam sync / saves).
    pub fn earned_set(&self) -> BTreeSet<String> {
        self.earned.clone()
    }

    /// Merge earned ids from a save (union).
    pub fn apply_earned(&mut self, set: impl IntoIterator<Item = String>) {
        for id in set {
            self.earned.insert(id);
        }
    }

    /// Progress (earned, total).
    pub fn progress(&self) -> (usize, usize) {
        (self.earned.len(), self.defs.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AchievementState {
        AchievementState::from_json(
            r#"{"achievements": [
                {"id": "FIRST_ENDING", "name": "Finished", "trigger": {"on": "ending", "id": "warm"}},
                {"id": "ART", "name": "Art", "trigger": {"on": "gallery", "id": "cg_rooftop"}},
                {"id": "MUSIC", "name": "Music", "trigger": {"on": "music", "id": "soft"}}
            ]}"#,
        )
        .unwrap()
    }

    #[test]
    fn grants_fire_once() {
        let mut a = sample();
        // note: first def has both trigger+id keys; serde flatten picks `on`
        assert_eq!(
            a.grant_for("ending", "warm"),
            vec!["FIRST_ENDING".to_string()]
        );
        assert!(a.grant_for("ending", "warm").is_empty());
        assert!(a.grant_for("gallery", "cg_rooftop").len() == 1);
        assert!(a.grant_for("ending", "cool").is_empty());
        assert_eq!(a.progress(), (2, 3));
    }

    #[test]
    fn broken_json_errors() {
        assert!(AchievementState::from_json("nope").is_err());
    }

    #[test]
    fn earned_roundtrip() {
        let mut a = sample();
        a.grant_for("music", "soft");
        let set = a.earned_set();
        let mut b = AchievementState::new();
        b.apply_earned(set);
        assert!(b.is_earned("MUSIC"));
    }
}
