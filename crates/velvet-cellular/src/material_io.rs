//! Load / save material packs as JSON (author data pipeline).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::material::{MaterialDef, MaterialError, MaterialRegistry};

/// IO errors.
#[derive(Debug, Error)]
pub enum MaterialIoError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Registry.
    #[error("material: {0}")]
    Material(#[from] MaterialError),
}

/// JSON pack file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialPack {
    /// Pack name.
    pub name: String,
    /// Materials (order = registration order after air).
    pub materials: Vec<MaterialDef>,
}

/// Load pack and register into registry (skips `air` key if present).
pub fn load_material_pack(
    reg: &mut MaterialRegistry,
    path: impl AsRef<Path>,
) -> Result<usize, MaterialIoError> {
    let s = fs::read_to_string(path)?;
    let pack: MaterialPack = serde_json::from_str(&s)?;
    let mut n = 0;
    for def in pack.materials {
        if def.key == "air" {
            continue;
        }
        if reg.id(&def.key).is_ok() {
            // replace existing
            if let Ok(id) = reg.id(&def.key) {
                reg.set_def(id, def);
                n += 1;
            }
        } else {
            reg.register(def)?;
            n += 1;
        }
    }
    Ok(n)
}

/// Export registry (excluding air) as a pack.
pub fn export_material_pack(reg: &MaterialRegistry, name: &str) -> MaterialPack {
    let materials: Vec<_> = reg
        .all()
        .iter()
        .filter(|d| d.key != "air")
        .cloned()
        .collect();
    MaterialPack {
        name: name.into(),
        materials,
    }
}

/// Write pack to path.
pub fn write_material_pack(pack: &MaterialPack, path: impl AsRef<Path>) -> Result<(), MaterialIoError> {
    let s = serde_json::to_string_pretty(pack)?;
    fs::write(path, s)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtin_materials;
    use crate::material::{MaterialDef, Phase};
    use tempfile::tempdir;

    #[test]
    fn pack_roundtrip() {
        let mut reg = MaterialRegistry::new();
        register_builtin_materials(&mut reg).unwrap();
        let pack = export_material_pack(&reg, "test");
        let dir = tempdir().unwrap();
        let path = dir.path().join("mats.json");
        write_material_pack(&pack, &path).unwrap();
        let mut reg2 = MaterialRegistry::new();
        let n = load_material_pack(&mut reg2, &path).unwrap();
        assert!(n >= 10);
        assert!(reg2.id("sand").is_ok());
        // custom add
        reg2.register(MaterialDef::new("goo", "Goo", Phase::Liquid).density(1.1))
            .unwrap();
        assert!(reg2.id("goo").is_ok());
    }
}
