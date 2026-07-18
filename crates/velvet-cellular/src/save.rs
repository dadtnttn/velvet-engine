//! World save / load (JSON) for author tools and games.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::chunk::{Chunk, ChunkCoord};
use crate::material::MaterialRegistry;
use crate::physics::PhysicsWorld;
use crate::world::{World, WorldConfig};

/// Save errors.
#[derive(Debug, Error)]
pub enum SaveError {
    /// IO.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Message.
    #[error("{0}")]
    Msg(String),
}

/// Serializable world snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSave {
    /// Format version.
    pub version: u32,
    /// Tick.
    pub tick: u64,
    /// Rng.
    pub rng: u64,
    /// Config.
    pub config: WorldConfig,
    /// Materials.
    pub materials: MaterialRegistry,
    /// Chunks.
    pub chunks: Vec<(ChunkCoord, Chunk)>,
    /// Optional rigid bodies.
    pub physics: Option<PhysicsWorld>,
    /// Optional enemies (v2 creation layer).
    #[serde(default)]
    pub enemies: Option<crate::enemy::EnemyWorld>,
    /// Optional brush state.
    #[serde(default)]
    pub brush: Option<crate::brush::Brush>,
    /// Optional free particles.
    #[serde(default)]
    pub particles: Option<crate::particles::ParticleWorld>,
}

impl WorldSave {
    /// Capture from live world + optional physics.
    pub fn capture(world: &World, physics: Option<&PhysicsWorld>) -> Self {
        let chunks: Vec<_> = world
            .chunks_map()
            .iter()
            .map(|(c, ch)| (*c, ch.clone()))
            .collect();
        Self {
            version: 2,
            tick: world.tick,
            rng: world.rng,
            config: world.config.clone(),
            materials: world.materials.clone(),
            chunks,
            physics: physics.cloned(),
            enemies: None,
            brush: None,
            particles: None,
        }
    }

    /// Restore into a new World + physics.
    pub fn restore(self) -> Result<(World, Option<PhysicsWorld>), SaveError> {
        if self.version == 0 {
            return Err(SaveError::Msg("invalid save version 0".into()));
        }
        let mut world = World::new(self.materials, self.config);
        world.tick = self.tick;
        world.rng = self.rng;
        let map = self.chunks.into_iter().collect();
        world.restore_chunks(map);
        Ok((world, self.physics))
    }

    /// Write JSON file.
    pub fn write_path(&self, path: impl AsRef<Path>) -> Result<(), SaveError> {
        let s = serde_json::to_string_pretty(self)?;
        fs::write(path, s)?;
        Ok(())
    }

    /// Read JSON file.
    pub fn read_path(path: impl AsRef<Path>) -> Result<Self, SaveError> {
        let s = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&s)?)
    }
}

/// Convenience: save world.
pub fn save_world(
    world: &World,
    physics: Option<&PhysicsWorld>,
    path: impl AsRef<Path>,
) -> Result<(), SaveError> {
    WorldSave::capture(world, physics).write_path(path)
}

/// Convenience: load world.
pub fn load_world(path: impl AsRef<Path>) -> Result<(World, Option<PhysicsWorld>), SaveError> {
    WorldSave::read_path(path)?.restore()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::register_builtin_materials;
    use crate::cell::Cell;
    use tempfile::tempdir;

    #[test]
    fn save_load_roundtrip_preserves_cells() {
        let mut reg = MaterialRegistry::new();
        register_builtin_materials(&mut reg).unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        let sand = world.mat("sand");
        world.set(3, 5, Cell::of(sand));
        world.set(4, 5, Cell::of(sand));
        world.tick = 42;

        let dir = tempdir().unwrap();
        let path = dir.path().join("w.json");
        save_world(&world, None, &path).unwrap();
        let (w2, _) = load_world(&path).unwrap();
        assert_eq!(w2.tick, 42);
        assert_eq!(w2.get(3, 5).material, sand);
        assert_eq!(w2.get(4, 5).material, sand);
        assert!(w2.get(0, 0).is_air());
    }
}
