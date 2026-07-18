//! Scene/prefab-oriented components that are serializable and engine-neutral.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

use crate::prefab::PrefabValue;

/// Lightweight sprite reference for scenes (render resolves paths).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpriteRef {
    /// Texture asset path.
    pub texture: String,
    /// World size.
    pub size: Vec2,
    /// Layer z.
    pub z: f32,
}

/// Arbitrary properties bag.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Properties {
    /// Values.
    pub values: IndexMap<String, PrefabValue>,
}

impl Properties {
    /// Get string.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.values.get(key)? {
            PrefabValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Marker: entity survives non-additive scene replacement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persistent;
