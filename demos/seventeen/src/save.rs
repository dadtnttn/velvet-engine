use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use velvet_script_vs3::{bool_val, int, map_val, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub effects_volume: f32,
    pub screen_shake: bool,
    pub distortion: bool,
    pub flashes: bool,
    pub high_contrast: bool,
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 0.75,
            music_volume: 0.45,
            effects_volume: 0.75,
            screen_shake: true,
            distortion: true,
            flashes: true,
            high_contrast: false,
            fullscreen: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: i64,
    pub seed: i64,
    pub room: i64,
    pub deaths: i64,
    pub score: i64,
    pub memory0: bool,
    pub memory1: bool,
    pub memory2: bool,
    pub pistol: bool,
    pub shotgun: bool,
    pub blade: bool,
    pub ending_a: bool,
    pub ending_b: bool,
}

impl SaveData {
    pub fn from_vs3(value: &Value) -> Result<Self> {
        let i = |key| -> Result<i64> {
            value
                .map_get(key)
                .map_err(anyhow::Error::msg)?
                .and_then(|item| item.as_i64())
                .with_context(|| format!("save field `{key}`"))
        };
        let b = |key| -> Result<bool> {
            match value.map_get(key).map_err(anyhow::Error::msg)? {
                Some(Value::Bool(flag)) => Ok(flag),
                _ => anyhow::bail!("save field `{key}` is not a boolean"),
            }
        };
        Ok(Self {
            version: i("version")?,
            seed: i("seed")?,
            room: i("room")?,
            deaths: i("deaths")?,
            score: i("score")?,
            memory0: b("memory0")?,
            memory1: b("memory1")?,
            memory2: b("memory2")?,
            pistol: b("pistol")?,
            shotgun: b("shotgun")?,
            blade: b("blade")?,
            ending_a: b("ending_a")?,
            ending_b: b("ending_b")?,
        })
    }

    pub fn to_vs3(&self) -> Value {
        map_val([
            ("version".into(), int(self.version)),
            ("seed".into(), int(self.seed)),
            ("room".into(), int(self.room.clamp(1, 5))),
            ("deaths".into(), int(self.deaths.max(0))),
            ("score".into(), int(self.score.max(0))),
            ("memory0".into(), bool_val(self.memory0)),
            ("memory1".into(), bool_val(self.memory1)),
            ("memory2".into(), bool_val(self.memory2)),
            ("pistol".into(), bool_val(self.pistol)),
            ("shotgun".into(), bool_val(self.shotgun)),
            ("blade".into(), bool_val(self.blade)),
            ("ending_a".into(), bool_val(self.ending_a)),
            ("ending_b".into(), bool_val(self.ending_b)),
        ])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct DiskState {
    #[serde(default)]
    settings: Settings,
    save: Option<SaveData>,
}

pub struct SaveStore {
    path: PathBuf,
    state: DiskState,
    warning: Option<String>,
}

impl SaveStore {
    pub fn load() -> Self {
        let path = save_path();
        if !path.exists() {
            return Self {
                path,
                state: DiskState::default(),
                warning: None,
            };
        }
        match fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))
            .and_then(|text| serde_json::from_str(&text).context("parse save JSON"))
        {
            Ok(state) => Self {
                path,
                state,
                warning: None,
            },
            Err(error) => Self {
                path,
                state: DiskState::default(),
                warning: Some(format!("Guardado danado; se ignoro: {error:#}")),
            },
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.state.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.state.settings
    }

    pub fn save(&self) -> Option<&SaveData> {
        self.state.save.as_ref()
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    pub fn set_save(&mut self, save: SaveData) -> Result<()> {
        self.state.save = Some(save);
        self.flush()
    }

    pub fn clear_save(&mut self) -> Result<()> {
        self.state.save = None;
        self.flush()
    }

    pub fn flush(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(&self.state)?;
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, json).with_context(|| format!("write {}", temporary.display()))?;
        if self.path.exists() {
            fs::remove_file(&self.path)
                .with_context(|| format!("replace {}", self.path.display()))?;
        }
        fs::rename(&temporary, &self.path)
            .with_context(|| format!("commit {}", self.path.display()))?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn save_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .join("Velvet Grid Studio")
        .join("17")
        .join("save.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_round_trips_through_vs3_value() {
        let input = SaveData {
            version: 1,
            seed: 17,
            room: 4,
            deaths: 3,
            score: 420,
            memory0: true,
            memory1: false,
            memory2: true,
            pistol: true,
            shotgun: true,
            blade: false,
            ending_a: false,
            ending_b: false,
        };
        let output = SaveData::from_vs3(&input.to_vs3()).unwrap();
        assert_eq!(output.room, 4);
        assert!(output.memory0 && output.memory2);
    }
}
