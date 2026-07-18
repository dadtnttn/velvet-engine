//! Scene manager: load, unload, transition, additive stacks.

use indexmap::IndexMap;
use thiserror::Error;
use tracing::{debug, info};
use velvet_ecs::{Entity, World};

use crate::hierarchy::Name;
use crate::prefab::{PrefabLibrary, PrefabNode};
use crate::scene::{Scene, SceneId, SceneState};
use crate::serde_components::Persistent;

/// Scene manager errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SceneManagerError {
    /// Scene missing.
    #[error("scene not found: {0}")]
    NotFound(String),
    /// Invalid operation.
    #[error("invalid state: {0}")]
    InvalidState(String),
}

/// Events emitted by the manager (for systems / UI).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneEvent {
    /// Loaded.
    Loaded {
        /// Id.
        id: SceneId,
        /// Name.
        name: String,
    },
    /// Unloaded.
    Unloaded {
        /// Id.
        id: SceneId,
        /// Name.
        name: String,
    },
    /// Became active.
    Activated {
        /// Id.
        id: SceneId,
    },
}

/// Description used to build a scene without Velvet Script yet.
#[derive(Debug, Clone)]
pub struct SceneBlueprint {
    /// Name.
    pub name: String,
    /// Named prefab instances: (entity_name, prefab_id).
    pub entities: Vec<(String, String)>,
    /// Extra root nodes (inline).
    pub inline: Vec<PrefabNode>,
}

impl SceneBlueprint {
    /// Empty named scene.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entities: Vec::new(),
            inline: Vec::new(),
        }
    }

    /// Add prefab instance.
    pub fn with_entity(mut self, name: impl Into<String>, prefab: impl Into<String>) -> Self {
        self.entities.push((name.into(), prefab.into()));
        self
    }
}

/// Owns loaded scenes and operates on a shared [`World`].
#[derive(Debug, Default)]
pub struct SceneManager {
    scenes: IndexMap<UuidKey, Scene>,
    /// Active primary scene (non-additive stack top).
    primary: Option<UuidKey>,
    events: Vec<SceneEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct UuidKey(uuid::Uuid);

impl From<SceneId> for UuidKey {
    fn from(value: SceneId) -> Self {
        Self(value.0)
    }
}

impl SceneManager {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a blueprint, optionally additive.
    pub fn load(
        &mut self,
        world: &mut World,
        library: &PrefabLibrary,
        blueprint: &SceneBlueprint,
        additive: bool,
    ) -> Result<SceneId, SceneManagerError> {
        if !additive {
            self.unload_non_persistent(world);
        }

        let mut scene = Scene::new(blueprint.name.clone(), additive);
        scene.state = SceneState::Loading;

        for (entity_name, prefab_id) in &blueprint.entities {
            let entity = library
                .instantiate(prefab_id, world, None)
                .map_err(|e| SceneManagerError::InvalidState(e.to_string()))?;
            // Ensure name component matches instance name.
            world.insert(entity, Name::new(entity_name.clone()));
            scene.roots.push(entity);
            scene.register_name(entity_name.clone(), entity);
        }

        for node in &blueprint.inline {
            let entity = crate::prefab::Prefab {
                id: crate::prefab::PrefabId::new(format!("inline/{}", blueprint.name)),
                version: 1,
                root: node.clone(),
            }
            .instantiate(world, None);
            scene.roots.push(entity);
            if let Some(n) = world.get::<Name>(entity) {
                scene.register_name(n.as_str().to_string(), entity);
            }
        }

        scene.state = SceneState::Active;
        let id = scene.id;
        let name = scene.name.clone();
        if !additive {
            self.primary = Some(id.into());
        }
        self.scenes.insert(id.into(), scene);
        self.events.push(SceneEvent::Loaded {
            id,
            name: name.clone(),
        });
        self.events.push(SceneEvent::Activated { id });
        info!(%name, additive, "scene loaded");
        Ok(id)
    }

    /// Unload a scene by id.
    pub fn unload(&mut self, world: &mut World, id: SceneId) -> Result<(), SceneManagerError> {
        let key = UuidKey::from(id);
        let scene = self
            .scenes
            .swap_remove(&key)
            .ok_or_else(|| SceneManagerError::NotFound(format!("{id:?}")))?;
        for entity in scene.roots {
            despawn_recursive(world, entity);
        }
        if self.primary == Some(key) {
            self.primary = None;
        }
        self.events.push(SceneEvent::Unloaded {
            id: scene.id,
            name: scene.name.clone(),
        });
        debug!(name = %scene.name, "scene unloaded");
        Ok(())
    }

    /// Unload all non-persistent content (used before exclusive load).
    pub fn unload_non_persistent(&mut self, world: &mut World) {
        let ids: Vec<SceneId> = self.scenes.values().map(|s| s.id).collect();
        for id in ids {
            let key = UuidKey::from(id);
            if let Some(scene) = self.scenes.get(&key) {
                // Keep additive persistent scenes with keep flag.
                if scene.keep_on_replace {
                    continue;
                }
            }
            let _ = self.unload(world, id);
        }
        // Also despawn orphan non-persistent entities? keep simple: only scene roots.
    }

    /// Get scene.
    pub fn get(&self, id: SceneId) -> Option<&Scene> {
        self.scenes.get(&UuidKey::from(id))
    }

    /// Primary scene.
    pub fn primary(&self) -> Option<&Scene> {
        self.primary.and_then(|k| self.scenes.get(&k))
    }

    /// All scenes.
    pub fn iter(&self) -> impl Iterator<Item = &Scene> {
        self.scenes.values()
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.scenes.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.scenes.is_empty()
    }

    /// Drain events.
    pub fn drain_events(&mut self) -> Vec<SceneEvent> {
        std::mem::take(&mut self.events)
    }

    /// Find entity by name in any active scene (primary first).
    pub fn find_named(&self, name: &str) -> Option<Entity> {
        if let Some(p) = self.primary() {
            if let Some(e) = p.get(name) {
                return Some(e);
            }
        }
        for scene in self.scenes.values() {
            if let Some(e) = scene.get(name) {
                return Some(e);
            }
        }
        None
    }
}

fn despawn_recursive(world: &mut World, entity: Entity) {
    if world.get::<Persistent>(entity).is_some() {
        return;
    }
    // Collect children first
    let children = world
        .get::<crate::hierarchy::Children>(entity)
        .map(|c| c.0.clone())
        .unwrap_or_default();
    for child in children {
        despawn_recursive(world, child);
    }
    world.despawn(entity);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefab::Prefab;
    use velvet_math::Vec2;

    #[test]
    fn load_unload_scene() {
        let mut world = World::new();
        let mut library = PrefabLibrary::new();
        library.insert(Prefab::simple("prefabs/player", "player", Vec2::ZERO));
        library.insert(Prefab::simple("prefabs/door", "door", Vec2::new(5.0, 0.0)));

        let mut mgr = SceneManager::new();
        let bp = SceneBlueprint::new("city_night")
            .with_entity("player", "prefabs/player")
            .with_entity("door", "prefabs/door");
        let id = mgr.load(&mut world, &library, &bp, false).unwrap();
        assert_eq!(mgr.len(), 1);
        assert_eq!(world.entity_count(), 2);
        assert!(mgr.find_named("player").is_some());

        mgr.unload(&mut world, id).unwrap();
        assert!(mgr.is_empty());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn additive_load_keeps_previous() {
        let mut world = World::new();
        let mut library = PrefabLibrary::new();
        library.insert(Prefab::simple("a", "a", Vec2::ZERO));
        library.insert(Prefab::simple("b", "b", Vec2::ONE));
        let mut mgr = SceneManager::new();
        mgr.load(
            &mut world,
            &library,
            &SceneBlueprint::new("base").with_entity("a", "a"),
            false,
        )
        .unwrap();
        mgr.load(
            &mut world,
            &library,
            &SceneBlueprint::new("overlay").with_entity("b", "b"),
            true,
        )
        .unwrap();
        assert_eq!(mgr.len(), 2);
        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn exclusive_load_replaces() {
        let mut world = World::new();
        let mut library = PrefabLibrary::new();
        library.insert(Prefab::simple("a", "a", Vec2::ZERO));
        library.insert(Prefab::simple("b", "b", Vec2::ONE));
        let mut mgr = SceneManager::new();
        mgr.load(
            &mut world,
            &library,
            &SceneBlueprint::new("one").with_entity("a", "a"),
            false,
        )
        .unwrap();
        mgr.load(
            &mut world,
            &library,
            &SceneBlueprint::new("two").with_entity("b", "b"),
            false,
        )
        .unwrap();
        assert_eq!(mgr.len(), 1);
        assert!(mgr.find_named("b").is_some());
        assert!(mgr.find_named("a").is_none());
    }
}
