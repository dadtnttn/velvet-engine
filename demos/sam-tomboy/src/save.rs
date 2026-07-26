use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::model::GameState;
use crate::render::ensure_parent;

pub fn save_path() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        PathBuf::from(appdata)
            .join("VelvetEngine")
            .join("SamHonestStranger")
            .join("save.json")
    } else {
        PathBuf::from("demos/sam-tomboy/save/save.json")
    }
}

pub fn exists() -> bool {
    save_path().is_file()
}

pub fn save(state: &GameState) -> Result<()> {
    let path = save_path();
    ensure_parent(&path)?;
    let temporary = path.with_extension("json.tmp");
    let encoded = serde_json::to_vec_pretty(state).context("no se pudo serializar la partida")?;
    fs::write(&temporary, encoded)
        .with_context(|| format!("no se pudo escribir {}", temporary.display()))?;
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    fs::rename(&temporary, &path)
        .with_context(|| format!("no se pudo confirmar {}", path.display()))?;
    Ok(())
}

pub fn load() -> Result<GameState> {
    let path = save_path();
    let bytes = fs::read(&path).with_context(|| format!("no se pudo leer {}", path.display()))?;
    let mut state: GameState = serde_json::from_slice(&bytes)
        .with_context(|| format!("guardado corrupto en {}", path.display()))?;
    state.clamp_stats();
    Ok(state)
}
