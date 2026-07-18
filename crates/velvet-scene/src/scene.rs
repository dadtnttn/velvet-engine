//! Scene instance state.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use velvet_ecs::Entity;

/// Stable scene instance id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneId(pub Uuid);

impl SceneId {
    /// New random id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SceneId {
    fn default() -> Self {
        Self::new()
    }
}

/// Lifecycle of a loaded scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneState {
    /// Being loaded.
    Loading,
    /// Active and updating.
    Active,
    /// Loaded but not updating (paused).
    Suspended,
    /// Unloading.
    Unloading,
}

/// A loaded scene owning a set of root entities.
#[derive(Debug, Clone)]
pub struct Scene {
    /// Id.
    pub id: SceneId,
    /// Asset / logical name (e.g. `city_night`).
    pub name: String,
    /// State.
    pub state: SceneState,
    /// Root entities spawned for this scene.
    pub roots: Vec<Entity>,
    /// Named entity lookup within the scene.
    pub named: IndexMap<String, Entity>,
    /// Whether this scene was loaded additively.
    pub additive: bool,
    /// Persistent flag (survives non-additive loads if set on entities via marker).
    pub keep_on_replace: bool,
}

impl Scene {
    /// Create empty scene shell.
    pub fn new(name: impl Into<String>, additive: bool) -> Self {
        Self {
            id: SceneId::new(),
            name: name.into(),
            state: SceneState::Loading,
            roots: Vec::new(),
            named: IndexMap::new(),
            additive,
            keep_on_replace: false,
        }
    }

    /// Register named entity.
    pub fn register_name(&mut self, name: impl Into<String>, entity: Entity) {
        let name = name.into();
        self.named.insert(name, entity);
        if !self.roots.contains(&entity) {
            // roots only if no parent — caller decides; still track.
        }
    }

    /// Lookup.
    pub fn get(&self, name: &str) -> Option<Entity> {
        self.named.get(name).copied()
    }

    /// Entity count tracked.
    pub fn entity_count(&self) -> usize {
        self.named.len().max(self.roots.len())
    }
}
