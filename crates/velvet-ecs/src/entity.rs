//! Entity identifiers.

use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

new_key_type! {
    /// Stable entity key within a [`crate::World`].
    pub struct Entity;
}

/// Metadata stored per entity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityMeta {
    /// Optional debug name.
    pub name: Option<String>,
    /// Whether the entity is enabled for queries.
    pub enabled: bool,
    /// Generation bump for recycled ids (slotmap handles this; kept for save format).
    pub generation: u32,
}

impl EntityMeta {
    /// New enabled entity meta.
    pub fn new() -> Self {
        Self {
            name: None,
            enabled: true,
            generation: 0,
        }
    }

    /// Named entity.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            enabled: true,
            generation: 0,
        }
    }
}
