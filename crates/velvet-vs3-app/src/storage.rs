//! Sandboxed save slots for the `storage` capability.
//!
//! Scripts never see paths. A frame emits `storage.save`, `storage.load`,
//! `storage.delete` or `storage.list` events; the host answers each one by
//! calling the optional `app_storage(result)` export. Data is stored as JSON
//! in one file per slot inside the app's own save directory.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use velvet_script_vs3::{json_decode, json_encode, Value};

use crate::assets::AppManifest;
use crate::model::RuntimeEvent;

/// Largest encoded save accepted per slot.
pub const MAX_SAVE_BYTES: usize = 8 * 1024 * 1024;
/// Most slots one app may keep.
pub const MAX_SLOTS: usize = 64;

/// Where save slots live.
#[derive(Debug, Clone)]
pub enum SaveStore {
    /// In-memory slots (captures, tests, headless runs).
    Memory(BTreeMap<String, String>),
    /// One `<slot>.json` per slot in this directory.
    Dir(PathBuf),
}

impl SaveStore {
    pub fn memory() -> Self {
        Self::Memory(BTreeMap::new())
    }

    /// `override_dir`, else `VELVET_SAVE_DIR`, else the per-user data
    /// directory (`%APPDATA%\Velvet\<app>\saves`, `~/Library/Application
    /// Support/Velvet/<app>/saves`, `$XDG_DATA_HOME/velvet/<app>/saves`).
    pub fn for_app(manifest: &AppManifest, override_dir: Option<PathBuf>) -> Self {
        if let Some(dir) =
            override_dir.or_else(|| std::env::var_os("VELVET_SAVE_DIR").map(PathBuf::from))
        {
            return Self::Dir(dir);
        }
        let app = app_folder_name(&manifest.app.title);
        match user_data_dir() {
            Some(base) => Self::Dir(base.join(app).join("saves")),
            None => Self::memory(),
        }
    }

    /// Handle a `storage.*` event, returning the result map for
    /// `app_storage`, or `None` for other events.
    pub fn handle(&mut self, event: &RuntimeEvent, allowed: bool) -> Option<Value> {
        let op = event.kind.strip_prefix("storage.")?;
        let slot = payload_string(event, "slot");
        let mut result = vec![
            ("op".to_string(), string(op)),
            ("slot".to_string(), string(slot.as_deref().unwrap_or(""))),
        ];
        let outcome = if !allowed {
            Err("the app did not declare the `storage` capability".to_string())
        } else {
            match op {
                "save" => self.save(slot.as_deref(), event),
                "load" => self.load(slot.as_deref()),
                "delete" => self.delete(slot.as_deref()),
                "list" => self.list(),
                other => Err(format!("unknown storage operation `{other}`")),
            }
        };
        match outcome {
            Ok(extra) => {
                result.push(("ok".into(), Value::Bool(true)));
                result.push(("error".into(), string("")));
                result.extend(extra);
            }
            Err(error) => {
                result.push(("ok".into(), Value::Bool(false)));
                result.push(("error".into(), string(&error)));
                result.push(("data".into(), Value::Null));
            }
        }
        if let Some(tag) = event
            .payload
            .as_ref()
            .and_then(|p| p.map_get("tag").ok().flatten())
        {
            result.push(("tag".into(), tag));
        }
        Some(Value::map(result))
    }

    fn save(
        &mut self,
        slot: Option<&str>,
        event: &RuntimeEvent,
    ) -> Result<Vec<(String, Value)>, String> {
        let slot = valid_slot(slot)?;
        let data = event
            .payload
            .as_ref()
            .and_then(|p| p.map_get("data").ok().flatten())
            .unwrap_or(Value::Null);
        let text = json_encode(&data, false)?;
        if text.len() > MAX_SAVE_BYTES {
            return Err(format!(
                "save `{slot}` is {} bytes; limit is {MAX_SAVE_BYTES}",
                text.len()
            ));
        }
        let bytes = text.len() as i64;
        match self {
            Self::Memory(slots) => {
                if !slots.contains_key(slot) && slots.len() >= MAX_SLOTS {
                    return Err(format!("too many save slots (limit {MAX_SLOTS})"));
                }
                slots.insert(slot.to_string(), text);
            }
            Self::Dir(dir) => {
                fs::create_dir_all(&*dir)
                    .map_err(|e| format!("cannot create save directory: {e}"))?;
                let path = slot_path(dir, slot);
                if !path.exists() && count_slots(dir) >= MAX_SLOTS {
                    return Err(format!("too many save slots (limit {MAX_SLOTS})"));
                }
                // Write then rename so a crash never leaves a half save.
                let temp = dir.join(format!("{slot}.json.tmp"));
                fs::write(&temp, text.as_bytes()).map_err(|e| format!("cannot write save: {e}"))?;
                fs::rename(&temp, &path).map_err(|e| format!("cannot write save: {e}"))?;
            }
        }
        Ok(vec![
            ("bytes".into(), Value::Int(bytes)),
            ("data".into(), Value::Null),
        ])
    }

    fn load(&mut self, slot: Option<&str>) -> Result<Vec<(String, Value)>, String> {
        let slot = valid_slot(slot)?;
        let text = match self {
            Self::Memory(slots) => slots.get(slot).cloned(),
            Self::Dir(dir) => fs::read_to_string(slot_path(dir, slot)).ok(),
        };
        let Some(text) = text else {
            return Err(format!("no save in slot `{slot}`"));
        };
        if text.len() > MAX_SAVE_BYTES {
            return Err(format!("save `{slot}` is too large"));
        }
        let data = json_decode(&text).map_err(|e| format!("save `{slot}` is corrupt: {e}"))?;
        Ok(vec![("data".into(), data)])
    }

    fn delete(&mut self, slot: Option<&str>) -> Result<Vec<(String, Value)>, String> {
        let slot = valid_slot(slot)?;
        let existed = match self {
            Self::Memory(slots) => slots.remove(slot).is_some(),
            Self::Dir(dir) => fs::remove_file(slot_path(dir, slot)).is_ok(),
        };
        Ok(vec![
            ("existed".into(), Value::Bool(existed)),
            ("data".into(), Value::Null),
        ])
    }

    fn list(&mut self) -> Result<Vec<(String, Value)>, String> {
        let mut slots = Vec::new();
        match self {
            Self::Memory(map) => {
                for (slot, text) in map.iter() {
                    slots.push(slot_info(slot, text.len() as i64, 0));
                }
            }
            Self::Dir(dir) => {
                if let Ok(entries) = fs::read_dir(&*dir) {
                    let mut found = Vec::new();
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        let Some(slot) = name.strip_suffix(".json") else {
                            continue;
                        };
                        if valid_slot(Some(slot)).is_err() {
                            continue;
                        }
                        let meta = entry.metadata().ok();
                        let bytes = meta.as_ref().map_or(0, |m| m.len() as i64);
                        let modified = meta
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map_or(0, |d| d.as_secs() as i64);
                        found.push((slot.to_string(), bytes, modified));
                    }
                    found.sort();
                    for (slot, bytes, modified) in found {
                        slots.push(slot_info(&slot, bytes, modified));
                    }
                }
            }
        }
        Ok(vec![
            ("slots".into(), Value::list(slots)),
            ("data".into(), Value::Null),
        ])
    }
}

fn slot_info(slot: &str, bytes: i64, modified: i64) -> Value {
    Value::map([
        ("slot".to_string(), string(slot)),
        ("bytes".to_string(), Value::Int(bytes)),
        ("modified".to_string(), Value::Int(modified)),
    ])
}

/// Slot names are 1..=48 characters of `[A-Za-z0-9_-]`, so they can never
/// name a path outside the save directory.
fn valid_slot(slot: Option<&str>) -> Result<&str, String> {
    let slot = slot.unwrap_or("");
    let ok = !slot.is_empty()
        && slot.len() <= 48
        && slot
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok {
        Ok(slot)
    } else {
        Err(format!(
            "invalid save slot `{slot}`: use 1-48 letters, digits, `_` or `-`"
        ))
    }
}

fn slot_path(dir: &Path, slot: &str) -> PathBuf {
    dir.join(format!("{slot}.json"))
}

fn count_slots(dir: &Path) -> usize {
    fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".json"))
            .count()
    })
}

fn app_folder_name(title: &str) -> String {
    let mut name: String = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    name.truncate(64);
    if name.trim_matches('_').is_empty() {
        "app".into()
    } else {
        name
    }
}

fn user_data_dir() -> Option<PathBuf> {
    let env = |key: &str| {
        std::env::var_os(key)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if cfg!(windows) {
        env("APPDATA").map(|dir| dir.join("Velvet"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|home| home.join("Library/Application Support/Velvet"))
    } else {
        env("XDG_DATA_HOME")
            .or_else(|| env("HOME").map(|home| home.join(".local/share")))
            .map(|dir| dir.join("velvet"))
    }
}

fn payload_string(event: &RuntimeEvent, key: &str) -> Option<String> {
    event
        .payload
        .as_ref()?
        .map_get(key)
        .ok()
        .flatten()?
        .as_str()
        .map(str::to_owned)
}

fn string(s: &str) -> Value {
    Value::String(s.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str, payload: Vec<(&str, Value)>) -> RuntimeEvent {
        RuntimeEvent {
            kind: kind.into(),
            payload: Some(Value::map(
                payload.into_iter().map(|(k, v)| (k.to_string(), v)),
            )),
        }
    }

    fn field(value: &Value, key: &str) -> Value {
        value.map_get(key).unwrap().unwrap()
    }

    fn roundtrip(store: &mut SaveStore) {
        let data = Value::map([
            ("age".to_string(), Value::Int(31)),
            ("name".to_string(), string("Ana")),
        ]);
        let saved = store
            .handle(
                &event(
                    "storage.save",
                    vec![("slot", string("auto")), ("data", data)],
                ),
                true,
            )
            .unwrap();
        assert_eq!(field(&saved, "ok"), Value::Bool(true));
        let loaded = store
            .handle(&event("storage.load", vec![("slot", string("auto"))]), true)
            .unwrap();
        assert_eq!(field(&loaded, "ok"), Value::Bool(true));
        assert_eq!(field(&field(&loaded, "data"), "age"), Value::Int(31));
        let listed = store.handle(&event("storage.list", vec![]), true).unwrap();
        match field(&listed, "slots") {
            Value::List(items) => assert_eq!(items.borrow().len(), 1),
            other => panic!("expected list, got {other:?}"),
        }
        let deleted = store
            .handle(
                &event("storage.delete", vec![("slot", string("auto"))]),
                true,
            )
            .unwrap();
        assert_eq!(field(&deleted, "existed"), Value::Bool(true));
        let missing = store
            .handle(&event("storage.load", vec![("slot", string("auto"))]), true)
            .unwrap();
        assert_eq!(field(&missing, "ok"), Value::Bool(false));
    }

    #[test]
    fn memory_store_roundtrip() {
        roundtrip(&mut SaveStore::memory());
    }

    #[test]
    fn dir_store_roundtrip() {
        let dir = std::env::temp_dir().join(format!("velvet-save-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        roundtrip(&mut SaveStore::Dir(dir.clone()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_path_like_slots_and_missing_capability() {
        let mut store = SaveStore::memory();
        for slot in ["", "../x", "a/b", "a.b", "con\u{0}"] {
            let result = store
                .handle(&event("storage.save", vec![("slot", string(slot))]), true)
                .unwrap();
            assert_eq!(field(&result, "ok"), Value::Bool(false), "{slot}");
        }
        let denied = store.handle(&event("storage.list", vec![]), false).unwrap();
        assert_eq!(field(&denied, "ok"), Value::Bool(false));
        assert!(store
            .handle(&event("window.set_title", vec![]), true)
            .is_none());
    }
}
