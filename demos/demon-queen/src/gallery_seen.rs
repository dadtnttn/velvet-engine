//! What the gallery has been shown of the animated scenes: each shot (a folder of frames under
//! `data/assets`, such as `burdel/alba/escena/rapido`) once it plays, kept in `gallery.json` beside
//! the pictures of the visual novel as `escena:<folder>`. The scenes write here as they play; the
//! gallery reads it to know which shots it can show.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{Map, Value};

/// How a shot is named in the gallery's file.
pub(crate) const PREFIX: &str = "escena:";

/// The shots already written this run (so a loop playing again costs nothing).
static WRITTEN: Mutex<Option<BTreeSet<String>>> = Mutex::new(None);

/// The gallery's file (as `game::gallery_path`).
pub(crate) fn gallery_path() -> PathBuf {
    if let Some(path) = std::env::var_os("DEMON_QUEEN_GALLERY") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("DEMON_QUEEN_SAVES") {
        return PathBuf::from(path).join("gallery.json");
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("DemonQueen")
            .join("gallery.json");
    }
    crate::exploration3d_state::source_root().join("data/gallery.json")
}

/// A shot's name from its folder: the part after `assets/`, with `/` between its folders.
pub(crate) fn shot_key(dir: &Path) -> Option<String> {
    let path = dir.to_string_lossy().replace('\\', "/");
    let rest = &path[path.rfind("assets/")? + "assets/".len()..];
    let rest = rest.trim_matches('/');
    (!rest.is_empty()).then(|| format!("{PREFIX}{rest}"))
}

/// How a date's memory is named in the gallery's file (`cita:<picture under data/assets>`).
pub(crate) const MEMORY: &str = "cita:";

/// A date's memory has been shown (`@recuerdo`): the gallery's «Citas» album can show it now.
pub(crate) fn memory(picture: &str) {
    let picture = picture.trim().trim_matches('/');
    if !picture.is_empty() {
        note(format!("{MEMORY}{picture}"));
    }
}

/// The shot in folder `dir` has been played: the gallery can show it from now on.
pub(crate) fn shot(dir: &Path) {
    let Some(key) = shot_key(dir) else {
        return;
    };
    note(key);
}

/// `key` into the gallery's file, once.
fn note(key: String) {
    // The tests play scenes too; they leave the player's gallery alone.
    if cfg!(test) {
        return;
    }
    {
        let mut written = WRITTEN.lock().unwrap_or_else(|e| e.into_inner());
        if !written
            .get_or_insert_with(BTreeSet::new)
            .insert(key.clone())
        {
            return;
        }
    }
    // (The file read here may still be on its way to the disk.)
    crate::exploration3d_state::background::flush();
    if let Err(error) = add(&gallery_path(), &key) {
        eprintln!("no se pudo apuntar la toma en la galería: {error}");
    }
}

fn add(path: &Path, key: &str) -> std::io::Result<()> {
    let mut map: Map<String, Value> = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|v| match v {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();
    map.entry("format")
        .or_insert_with(|| Value::from("demon-queen-gallery"));
    map.entry("schema").or_insert_with(|| Value::from(1));
    let list = map
        .entry("unlocked_assets")
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(items) = list {
        if items.iter().any(|v| v.as_str() == Some(key)) {
            return Ok(());
        }
        items.push(Value::from(key));
        items.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, serde_json::to_vec_pretty(&Value::Object(map))?)?;
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shot_is_named_by_its_folder_under_the_assets_and_written_once() {
        assert_eq!(
            shot_key(Path::new(
                r"C:\x\demon-queen\data\assets\burdel\alba\escena\rapido"
            ))
            .as_deref(),
            Some("escena:burdel/alba/escena/rapido")
        );
        assert_eq!(shot_key(Path::new("nowhere/else")), None);
        let dir = std::env::temp_dir().join(format!("dq-gallery-seen-{}", std::process::id()));
        let path = dir.join("gallery.json");
        let _ = std::fs::remove_dir_all(&dir);
        add(&path, "escena:a/b").unwrap();
        add(&path, "escena:a/b").unwrap();
        add(&path, "native_1080p/prologo/p01").unwrap();
        let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(value["unlocked_assets"].as_array().unwrap().len(), 2);
        assert_eq!(value["format"], "demon-queen-gallery");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
