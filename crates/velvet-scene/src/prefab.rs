//! Prefab definitions and instantiation.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use velvet_ecs::{Entity, World};
use velvet_math::{Transform2D, Vec2};

use crate::hierarchy::{Children, Name, Parent};

/// Prefab identifier (asset path or name).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PrefabId(pub String);

impl PrefabId {
    /// Create.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// As str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PrefabId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Prefab errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrefabError {
    /// Missing prefab.
    #[error("prefab not found: {0}")]
    NotFound(String),
    /// Invalid data.
    #[error("invalid prefab: {0}")]
    Invalid(String),
}

/// Serializable component blob for prefabs (versioned, stable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PrefabComponent {
    /// Name.
    Name {
        /// Value.
        value: String,
    },
    /// 2D transform.
    Transform {
        /// Translation.
        translation: [f32; 2],
        /// Rotation radians.
        rotation: f32,
        /// Scale.
        scale: [f32; 2],
    },
    /// Sprite reference (path only — render binds later).
    Sprite {
        /// Texture path.
        texture: String,
        /// Size.
        size: [f32; 2],
        /// Z layer.
        z: f32,
    },
    /// Generic key/value properties for game modules.
    Properties {
        /// Map.
        values: IndexMap<String, PrefabValue>,
    },
}

/// Simple serializable values for prefab properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PrefabValue {
    /// Bool.
    Bool(bool),
    /// Int.
    Int(i64),
    /// Float.
    Float(f64),
    /// String.
    String(String),
}

/// One node in a prefab hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrefabNode {
    /// Optional name.
    pub name: Option<String>,
    /// Components.
    pub components: Vec<PrefabComponent>,
    /// Children nodes.
    #[serde(default)]
    pub children: Vec<PrefabNode>,
}

/// Prefab asset.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prefab {
    /// Id / path.
    pub id: PrefabId,
    /// Format version.
    pub version: u32,
    /// Root node.
    pub root: PrefabNode,
}

impl Prefab {
    /// Simple named transform prefab.
    pub fn simple(id: impl Into<PrefabId>, name: &str, pos: Vec2) -> Self {
        Self {
            id: id.into(),
            version: 1,
            root: PrefabNode {
                name: Some(name.into()),
                components: vec![
                    PrefabComponent::Name { value: name.into() },
                    PrefabComponent::Transform {
                        translation: [pos.x, pos.y],
                        rotation: 0.0,
                        scale: [1.0, 1.0],
                    },
                ],
                children: vec![],
            },
        }
    }

    /// Serialize to RON.
    pub fn to_ron(&self) -> Result<String, PrefabError> {
        let pretty = ron::ser::PrettyConfig::new();
        ron::ser::to_string_pretty(self, pretty).map_err(|e| PrefabError::Invalid(e.to_string()))
    }

    /// Parse RON.
    pub fn from_ron(text: &str) -> Result<Self, PrefabError> {
        ron::from_str(text).map_err(|e| PrefabError::Invalid(e.to_string()))
    }

    /// Instantiate into a world; returns root entity.
    pub fn instantiate(&self, world: &mut World, parent: Option<Entity>) -> Entity {
        spawn_node(world, &self.root, parent)
    }

    /// Merge another prefab's root children and components into this root.
    ///
    /// Component policy: append components; children are appended.
    /// Name of `other` root is ignored unless this root has no name.
    pub fn merge_from(&mut self, other: &Prefab) {
        if self.root.name.is_none() {
            self.root.name = other.root.name.clone();
        }
        self.root
            .components
            .extend(other.root.components.iter().cloned());
        self.root
            .children
            .extend(other.root.children.iter().cloned());
    }

    /// Overlay property map on the root [`PrefabComponent::Properties`] (creates if missing).
    pub fn overlay_properties(&mut self, values: IndexMap<String, PrefabValue>) {
        if let Some(PrefabComponent::Properties { values: existing }) = self
            .root
            .components
            .iter_mut()
            .find(|c| matches!(c, PrefabComponent::Properties { .. }))
        {
            for (k, v) in values {
                existing.insert(k, v);
            }
        } else {
            self.root
                .components
                .push(PrefabComponent::Properties { values });
        }
    }
}

fn spawn_node(world: &mut World, node: &PrefabNode, parent: Option<Entity>) -> Entity {
    let entity = if let Some(name) = &node.name {
        world.spawn_named(name.clone())
    } else {
        world.spawn()
    };

    if let Some(name) = &node.name {
        world.insert(entity, Name::new(name.clone()));
    }

    for c in &node.components {
        match c {
            PrefabComponent::Name { value } => {
                world.insert(entity, Name::new(value.clone()));
            }
            PrefabComponent::Transform {
                translation,
                rotation,
                scale,
            } => {
                world.insert(
                    entity,
                    Transform2D {
                        translation: Vec2::new(translation[0], translation[1]),
                        rotation: *rotation,
                        scale: Vec2::new(scale[0], scale[1]),
                    },
                );
            }
            PrefabComponent::Sprite { texture, size, z } => {
                world.insert(
                    entity,
                    crate::serde_components::SpriteRef {
                        texture: texture.clone(),
                        size: Vec2::new(size[0], size[1]),
                        z: *z,
                    },
                );
            }
            PrefabComponent::Properties { values } => {
                world.insert(
                    entity,
                    crate::serde_components::Properties {
                        values: values.clone(),
                    },
                );
            }
        }
    }

    if let Some(p) = parent {
        world.insert(entity, Parent(p));
        if let Some(children) = world.get_mut::<Children>(p) {
            children.add(entity);
        } else {
            let mut ch = Children::new();
            ch.add(entity);
            world.insert(p, ch);
        }
    }

    let mut child_entities = Vec::new();
    for child in &node.children {
        child_entities.push(spawn_node(world, child, Some(entity)));
    }
    if !child_entities.is_empty() {
        let mut ch = Children::new();
        for c in child_entities {
            ch.add(c);
        }
        world.insert(entity, ch);
    }

    entity
}

/// Registry of prefabs.
#[derive(Debug, Default)]
pub struct PrefabLibrary {
    prefabs: IndexMap<String, Prefab>,
}

impl PrefabLibrary {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert.
    pub fn insert(&mut self, prefab: Prefab) {
        self.prefabs.insert(prefab.id.as_str().to_string(), prefab);
    }

    /// Get.
    pub fn get(&self, id: &str) -> Option<&Prefab> {
        self.prefabs.get(id)
    }

    /// Instantiate by id.
    pub fn instantiate(
        &self,
        id: &str,
        world: &mut World,
        parent: Option<Entity>,
    ) -> Result<Entity, PrefabError> {
        let prefab = self
            .get(id)
            .ok_or_else(|| PrefabError::NotFound(id.into()))?;
        Ok(prefab.instantiate(world, parent))
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.prefabs.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.prefabs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_roundtrip() {
        let p = Prefab::simple("prefabs/player", "player", Vec2::new(10.0, 20.0));
        let text = p.to_ron().unwrap();
        let back = Prefab::from_ron(&text).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn instantiate_with_child() {
        let mut prefab = Prefab::simple("p", "root", Vec2::ZERO);
        prefab.root.children.push(PrefabNode {
            name: Some("child".into()),
            components: vec![PrefabComponent::Name {
                value: "child".into(),
            }],
            children: vec![],
        });
        let mut world = World::new();
        let root = prefab.instantiate(&mut world, None);
        assert!(world.contains(root));
        let children = world.get::<Children>(root).unwrap();
        assert_eq!(children.0.len(), 1);
        assert!(world.get::<Name>(children.0[0]).is_some());
    }

    #[test]
    fn merge_prefabs() {
        let mut a = Prefab::simple("a", "root", Vec2::ZERO);
        let mut b = Prefab::simple("b", "other", Vec2::new(1.0, 2.0));
        b.root.children.push(PrefabNode {
            name: Some("extra".into()),
            components: vec![],
            children: vec![],
        });
        a.merge_from(&b);
        assert_eq!(a.root.children.len(), 1);
        assert!(
            a.root
                .components
                .iter()
                .filter(|c| matches!(c, PrefabComponent::Transform { .. }))
                .count()
                >= 2
        );

        let mut props = IndexMap::new();
        props.insert("hp".into(), PrefabValue::Int(10));
        a.overlay_properties(props);
        assert!(a
            .root
            .components
            .iter()
            .any(|c| matches!(c, PrefabComponent::Properties { values } if values.get("hp") == Some(&PrefabValue::Int(10)))));
    }
}
