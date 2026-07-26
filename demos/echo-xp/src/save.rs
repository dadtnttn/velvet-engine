use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use velvet_script_vs3::{bool_val, float_val, int, map_val, string_val, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub master_volume: f32,
    pub effects_volume: f32,
    pub music_volume: f32,
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            effects_volume: 0.9,
            music_volume: 0.7,
            fullscreen: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveData {
    pub version: u32,
    pub seed: i64,
    pub phase: String,
    pub operator_name: String,
    pub mail_read: bool,
    pub case_file_read: bool,
    pub photo_inspected: bool,
    pub tape_listened: bool,
    pub deleted_file_found: bool,
    pub classification: String,
    pub classification_attempts: i64,
    pub case_complete: bool,
    pub truth_unlocked: bool,
    pub anomaly_level: i64,
    pub system_corruption: f32,
    pub name_swap: bool,
    pub time_glitch: bool,
    pub extra_icon: bool,
    pub save_revision: i64,
    pub settings: Settings,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: 1,
            seed: 1407,
            phase: "boot".to_string(),
            operator_name: "OPERATOR".to_string(),
            mail_read: false,
            case_file_read: false,
            photo_inspected: false,
            tape_listened: false,
            deleted_file_found: false,
            classification: String::new(),
            classification_attempts: 0,
            case_complete: false,
            truth_unlocked: false,
            anomaly_level: 0,
            system_corruption: 0.0,
            name_swap: false,
            time_glitch: false,
            extra_icon: false,
            save_revision: 0,
            settings: Settings::default(),
        }
    }
}

impl SaveData {
    pub fn to_vs3(&self) -> Value {
        map_val([
            ("version".into(), int(self.version as i64)),
            ("seed".into(), int(self.seed)),
            ("phase".into(), string_val(&self.phase)),
            ("operator_name".into(), string_val(&self.operator_name)),
            ("mail_read".into(), bool_val(self.mail_read)),
            ("case_file_read".into(), bool_val(self.case_file_read)),
            ("photo_inspected".into(), bool_val(self.photo_inspected)),
            ("tape_listened".into(), bool_val(self.tape_listened)),
            (
                "deleted_file_found".into(),
                bool_val(self.deleted_file_found),
            ),
            ("classification".into(), string_val(&self.classification)),
            (
                "classification_attempts".into(),
                int(self.classification_attempts),
            ),
            ("case_complete".into(), bool_val(self.case_complete)),
            ("truth_unlocked".into(), bool_val(self.truth_unlocked)),
            ("anomaly_level".into(), int(self.anomaly_level)),
            (
                "system_corruption".into(),
                float_val(self.system_corruption as f64),
            ),
            ("name_swap".into(), bool_val(self.name_swap)),
            ("time_glitch".into(), bool_val(self.time_glitch)),
            ("extra_icon".into(), bool_val(self.extra_icon)),
            ("save_revision".into(), int(self.save_revision)),
        ])
    }

    pub fn from_vs3(val: &Value) -> Result<Self> {
        let field = |key: &str| -> Result<Value> {
            val.map_get(key)
                .map_err(|e| anyhow::anyhow!(e))?
                .ok_or_else(|| anyhow::anyhow!("Missing field `{key}`"))
        };

        Ok(Self {
            version: field("version")?.as_i64().unwrap_or(1) as u32,
            seed: field("seed")?.as_i64().unwrap_or(1407),
            phase: field("phase")?.as_str().unwrap_or("desktop").to_string(),
            operator_name: field("operator_name")?
                .as_str()
                .unwrap_or("OPERATOR")
                .to_string(),
            mail_read: field("mail_read")?.is_truthy(),
            case_file_read: field("case_file_read")?.is_truthy(),
            photo_inspected: field("photo_inspected")?.is_truthy(),
            tape_listened: field("tape_listened")?.is_truthy(),
            deleted_file_found: field("deleted_file_found")?.is_truthy(),
            classification: field("classification")?
                .as_str()
                .unwrap_or_default()
                .to_string(),
            classification_attempts: field("classification_attempts")?.as_i64().unwrap_or(0),
            case_complete: field("case_complete")?.is_truthy(),
            truth_unlocked: field("truth_unlocked")?.is_truthy(),
            anomaly_level: field("anomaly_level")?.as_i64().unwrap_or(0),
            system_corruption: field("system_corruption")?.as_f64().unwrap_or(0.0) as f32,
            name_swap: field("name_swap")?.is_truthy(),
            time_glitch: field("time_glitch")?.is_truthy(),
            extra_icon: field("extra_icon")?.is_truthy(),
            save_revision: field("save_revision")?.as_i64().unwrap_or(0),
            settings: Settings::default(),
        })
    }
}

pub struct SaveStore {
    path: PathBuf,
    data: SaveData,
    warning: Option<String>,
}

impl SaveStore {
    pub fn load() -> Self {
        let path = save_path();
        if let Ok(content) = fs::read_to_string(&path) {
            match serde_json::from_str::<SaveData>(&content) {
                Ok(data) => {
                    return Self {
                        path,
                        data,
                        warning: None,
                    }
                }
                Err(err) => {
                    let warning = format!("Save corrupt, resetting: {err}");
                    return Self {
                        path,
                        data: SaveData::default(),
                        warning: Some(warning),
                    };
                }
            }
        }

        Self {
            path,
            data: SaveData::default(),
            warning: None,
        }
    }

    pub fn data(&self) -> &SaveData {
        &self.data
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }

    pub fn save(&mut self, data: SaveData) -> Result<()> {
        self.data = data;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.data)?;
        let tmp_path = self.path.with_extension("tmp");
        fs::write(&tmp_path, json)?;
        fs::rename(tmp_path, &self.path)?;
        Ok(())
    }
}

fn save_path() -> PathBuf {
    if let Some(app_data) = std::env::var_os("APPDATA") {
        PathBuf::from(app_data)
            .join("VelvetEngine")
            .join("echo-xp")
            .join("save.json")
    } else {
        PathBuf::from("echo_xp_save.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_data_vs3_roundtrip() {
        let save = SaveData {
            seed: 9999,
            case_complete: true,
            classification: "MANDELA-CLASS INTRUSION".to_string(),
            ..SaveData::default()
        };

        let val = save.to_vs3();
        let restored = SaveData::from_vs3(&val).unwrap();

        assert_eq!(restored.seed, 9999);
        assert!(restored.case_complete);
        assert_eq!(restored.classification, "MANDELA-CLASS INTRUSION");
    }
}
