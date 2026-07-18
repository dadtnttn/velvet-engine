//! Prefab variants: base + overlay patches for skinning / difficulty / localization.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::prefab::{
    Prefab, PrefabComponent, PrefabError, PrefabId, PrefabLibrary, PrefabNode, PrefabValue,
};
use velvet_ecs::{Entity, World};
use velvet_math::Vec2;

/// Variant resolution errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VariantError {
    /// Base missing.
    #[error("base prefab not found: {0}")]
    BaseNotFound(String),
    /// Variant missing.
    #[error("variant not found: {0}")]
    VariantNotFound(String),
    /// Invalid patch.
    #[error("invalid variant patch: {0}")]
    Invalid(String),
    /// Prefab error.
    #[error(transparent)]
    Prefab(#[from] PrefabError),
}

/// Patch operations applied on top of a base prefab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum VariantPatch {
    /// Set root name.
    SetName {
        /// Name.
        name: String,
    },
    /// Set transform translation on root (creates transform if needed).
    SetTranslation {
        /// XY.
        translation: [f32; 2],
    },
    /// Set transform scale.
    SetScale {
        /// XY.
        scale: [f32; 2],
    },
    /// Set / replace sprite texture path.
    SetSpriteTexture {
        /// Path.
        texture: String,
    },
    /// Overlay properties.
    SetProperties {
        /// Values.
        values: IndexMap<String, PrefabValue>,
    },
    /// Append a child node.
    AddChild {
        /// Child.
        node: PrefabNode,
    },
    /// Remove children with name.
    RemoveChildNamed {
        /// Name.
        name: String,
    },
    /// Replace entire components list (advanced).
    ReplaceComponents {
        /// Components.
        components: Vec<PrefabComponent>,
    },
}

/// Named variant of a base prefab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabVariant {
    /// Variant id (e.g. `player.hard`, `slime.red`).
    pub id: String,
    /// Base prefab id.
    pub base: String,
    /// Ordered patches.
    pub patches: Vec<VariantPatch>,
    /// Optional documentation.
    #[serde(default)]
    pub description: String,
}

impl PrefabVariant {
    /// Create empty variant.
    pub fn new(id: impl Into<String>, base: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            base: base.into(),
            patches: Vec::new(),
            description: String::new(),
        }
    }

    /// Push patch.
    pub fn with_patch(mut self, patch: VariantPatch) -> Self {
        self.patches.push(patch);
        self
    }

    /// Apply this variant onto a base prefab clone.
    pub fn apply(&self, base: &Prefab) -> Prefab {
        let mut out = base.clone();
        out.id = PrefabId::new(self.id.clone());
        for patch in &self.patches {
            apply_patch(&mut out, patch);
        }
        out
    }
}

fn apply_patch(prefab: &mut Prefab, patch: &VariantPatch) {
    match patch {
        VariantPatch::SetName { name } => {
            prefab.root.name = Some(name.clone());
            // Keep Name component in sync if present.
            for c in &mut prefab.root.components {
                if let PrefabComponent::Name { value } = c {
                    *value = name.clone();
                }
            }
        }
        VariantPatch::SetTranslation { translation } => {
            if let Some(PrefabComponent::Transform { translation: t, .. }) = prefab
                .root
                .components
                .iter_mut()
                .find(|c| matches!(c, PrefabComponent::Transform { .. }))
            {
                *t = *translation;
            } else {
                prefab.root.components.push(PrefabComponent::Transform {
                    translation: *translation,
                    rotation: 0.0,
                    scale: [1.0, 1.0],
                });
            }
        }
        VariantPatch::SetScale { scale } => {
            if let Some(PrefabComponent::Transform { scale: s, .. }) = prefab
                .root
                .components
                .iter_mut()
                .find(|c| matches!(c, PrefabComponent::Transform { .. }))
            {
                *s = *scale;
            } else {
                prefab.root.components.push(PrefabComponent::Transform {
                    translation: [0.0, 0.0],
                    rotation: 0.0,
                    scale: *scale,
                });
            }
        }
        VariantPatch::SetSpriteTexture { texture } => {
            if let Some(PrefabComponent::Sprite { texture: t, .. }) = prefab
                .root
                .components
                .iter_mut()
                .find(|c| matches!(c, PrefabComponent::Sprite { .. }))
            {
                *t = texture.clone();
            } else {
                prefab.root.components.push(PrefabComponent::Sprite {
                    texture: texture.clone(),
                    size: [16.0, 16.0],
                    z: 0.0,
                });
            }
        }
        VariantPatch::SetProperties { values } => {
            prefab.overlay_properties(values.clone());
        }
        VariantPatch::AddChild { node } => {
            prefab.root.children.push(node.clone());
        }
        VariantPatch::RemoveChildNamed { name } => {
            prefab
                .root
                .children
                .retain(|c| c.name.as_deref() != Some(name.as_str()));
        }
        VariantPatch::ReplaceComponents { components } => {
            prefab.root.components = components.clone();
        }
    }
}

/// Library of variants keyed by variant id.
#[derive(Debug, Default, Clone)]
pub struct VariantLibrary {
    variants: IndexMap<String, PrefabVariant>,
}

impl VariantLibrary {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert variant.
    pub fn insert(&mut self, variant: PrefabVariant) {
        self.variants.insert(variant.id.clone(), variant);
    }

    /// Get.
    pub fn get(&self, id: &str) -> Option<&PrefabVariant> {
        self.variants.get(id)
    }

    /// Resolve variant against a prefab library → concrete Prefab.
    pub fn resolve(
        &self,
        prefabs: &PrefabLibrary,
        variant_id: &str,
    ) -> Result<Prefab, VariantError> {
        let variant = self
            .get(variant_id)
            .ok_or_else(|| VariantError::VariantNotFound(variant_id.into()))?;
        let base = prefabs
            .get(&variant.base)
            .ok_or_else(|| VariantError::BaseNotFound(variant.base.clone()))?;
        Ok(variant.apply(base))
    }

    /// Instantiate resolved variant into world.
    pub fn instantiate(
        &self,
        prefabs: &PrefabLibrary,
        variant_id: &str,
        world: &mut World,
        parent: Option<Entity>,
    ) -> Result<Entity, VariantError> {
        let prefab = self.resolve(prefabs, variant_id)?;
        Ok(prefab.instantiate(world, parent))
    }

    /// List variants for a base id.
    pub fn variants_of_base(&self, base: &str) -> Vec<&PrefabVariant> {
        self.variants.values().filter(|v| v.base == base).collect()
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.variants.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    /// Materialize all variants into a prefab library (ids = variant ids).
    pub fn bake_into(
        &self,
        prefabs: &PrefabLibrary,
        out: &mut PrefabLibrary,
    ) -> Result<usize, VariantError> {
        let mut n = 0;
        for id in self.variants.keys() {
            let p = self.resolve(prefabs, id)?;
            out.insert(p);
            n += 1;
        }
        Ok(n)
    }
}

/// Helper: build a color/skin variant shifting sprite path suffix.
pub fn skin_variant(
    id: impl Into<String>,
    base: impl Into<String>,
    texture: impl Into<String>,
) -> PrefabVariant {
    PrefabVariant::new(id, base).with_patch(VariantPatch::SetSpriteTexture {
        texture: texture.into(),
    })
}

/// Helper: difficulty variant overlaying properties.
pub fn difficulty_variant(
    id: impl Into<String>,
    base: impl Into<String>,
    hp: i64,
    damage: i64,
) -> PrefabVariant {
    let mut values = IndexMap::new();
    values.insert("hp".into(), PrefabValue::Int(hp));
    values.insert("damage".into(), PrefabValue::Int(damage));
    PrefabVariant::new(id, base).with_patch(VariantPatch::SetProperties { values })
}

/// Offset a variant's spawn translation.
pub fn offset_variant(id: impl Into<String>, base: impl Into<String>, pos: Vec2) -> PrefabVariant {
    PrefabVariant::new(id, base).with_patch(VariantPatch::SetTranslation {
        translation: [pos.x, pos.y],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_skin_and_props() {
        let mut prefs = PrefabLibrary::new();
        let mut base = Prefab::simple("enemy", "slime", Vec2::ZERO);
        base.root.components.push(PrefabComponent::Sprite {
            texture: "slime_blue.png".into(),
            size: [16.0, 16.0],
            z: 0.0,
        });
        prefs.insert(base);

        let mut variants = VariantLibrary::new();
        variants.insert(skin_variant("enemy.red", "enemy", "slime_red.png"));
        variants.insert(difficulty_variant("enemy.hard", "enemy", 50, 8));

        let red = variants.resolve(&prefs, "enemy.red").unwrap();
        assert!(red.root.components.iter().any(
            |c| matches!(c, PrefabComponent::Sprite { texture, .. } if texture == "slime_red.png")
        ));

        let hard = variants.resolve(&prefs, "enemy.hard").unwrap();
        assert!(hard.root.components.iter().any(|c| matches!(
            c,
            PrefabComponent::Properties { values } if values.get("hp") == Some(&PrefabValue::Int(50))
        )));
    }

    #[test]
    fn instantiate_variant() {
        let mut prefs = PrefabLibrary::new();
        prefs.insert(Prefab::simple("npc", "npc", Vec2::ZERO));
        let mut variants = VariantLibrary::new();
        variants.insert(offset_variant("npc.door", "npc", Vec2::new(32.0, 0.0)));
        let mut world = World::new();
        let e = variants
            .instantiate(&prefs, "npc.door", &mut world, None)
            .unwrap();
        assert!(world.contains(e));
    }

    #[test]
    fn remove_child_patch() {
        let mut base = Prefab::simple("tree", "tree", Vec2::ZERO);
        base.root.children.push(PrefabNode {
            name: Some("fruit".into()),
            components: vec![],
            children: vec![],
        });
        base.root.children.push(PrefabNode {
            name: Some("leaf".into()),
            components: vec![],
            children: vec![],
        });
        let v =
            PrefabVariant::new("tree.winter", "tree").with_patch(VariantPatch::RemoveChildNamed {
                name: "fruit".into(),
            });
        let out = v.apply(&base);
        assert_eq!(out.root.children.len(), 1);
        assert_eq!(out.root.children[0].name.as_deref(), Some("leaf"));
    }

    #[test]
    fn bake_into_library() {
        let mut prefs = PrefabLibrary::new();
        prefs.insert(Prefab::simple("a", "a", Vec2::ZERO));
        let mut variants = VariantLibrary::new();
        variants.insert(PrefabVariant::new("a.v1", "a"));
        variants.insert(PrefabVariant::new("a.v2", "a"));
        let mut out = PrefabLibrary::new();
        let n = variants.bake_into(&prefs, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out.len(), 2);
    }
}
