//! Checkpoints and simple play saves metadata.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

/// Checkpoint identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CheckpointId(pub String);

impl CheckpointId {
    /// Create.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Checkpoint in the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Id.
    pub id: CheckpointId,
    /// Spawn position.
    pub position: Vec2,
    /// Optional scene / map name.
    pub map: String,
    /// Active (last touched).
    pub active: bool,
}

impl Checkpoint {
    /// Create.
    pub fn new(id: impl Into<String>, position: Vec2, map: impl Into<String>) -> Self {
        Self {
            id: CheckpointId::new(id),
            position,
            map: map.into(),
            active: false,
        }
    }
}

/// Registry of checkpoints + last active.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointStore {
    /// All checkpoints.
    pub points: IndexMap<String, Checkpoint>,
    /// Last activated id.
    pub last: Option<String>,
}

impl CheckpointStore {
    /// Insert.
    pub fn insert(&mut self, cp: Checkpoint) {
        self.points.insert(cp.id.0.clone(), cp);
    }

    /// Activate by id.
    pub fn activate(&mut self, id: &str) -> bool {
        for (k, cp) in self.points.iter_mut() {
            cp.active = k == id;
        }
        if self.points.contains_key(id) {
            self.last = Some(id.into());
            true
        } else {
            false
        }
    }

    /// Respawn position.
    pub fn respawn_position(&self) -> Option<Vec2> {
        self.last
            .as_ref()
            .and_then(|id| self.points.get(id))
            .map(|c| c.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_respawn() {
        let mut store = CheckpointStore::default();
        store.insert(Checkpoint::new("a", Vec2::new(1.0, 2.0), "m1"));
        store.insert(Checkpoint::new("b", Vec2::new(5.0, 6.0), "m1"));
        assert!(store.activate("b"));
        assert_eq!(store.respawn_position(), Some(Vec2::new(5.0, 6.0)));
    }
}
