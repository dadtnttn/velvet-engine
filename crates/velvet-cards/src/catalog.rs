//! Card definitions and catalogs.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Stable card identifier (author-facing string).
pub type CardId = String;

/// One card definition in a catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDef {
    /// Unique id within the catalog.
    pub id: CardId,
    /// Display name.
    pub name: String,
    /// Numeric cost (mana, energy, AP — game-defined meaning).
    #[serde(default)]
    pub cost: i32,
    /// Free-form tags / types (e.g. `"spell"`, `"creature"`, `"fire"`).
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional primary type label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_type: Option<String>,
    /// Optional short rules text for tooling / export (not executed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl CardDef {
    /// Create a minimal card.
    pub fn new(id: impl Into<CardId>, name: impl Into<String>, cost: i32) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            cost,
            tags: Vec::new(),
            card_type: None,
            text: None,
        }
    }

    /// Builder: add a tag.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Builder: set type.
    pub fn with_type(mut self, t: impl Into<String>) -> Self {
        self.card_type = Some(t.into());
        self
    }
}

/// Ordered map of card definitions (insertion order preserved for tooling).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardCatalog {
    /// Cards keyed by id.
    #[serde(default)]
    pub cards: IndexMap<CardId, CardDef>,
}

impl CardCatalog {
    /// Empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a card; id must match `def.id`.
    pub fn insert(&mut self, def: CardDef) {
        let id = def.id.clone();
        self.cards.insert(id, def);
    }

    /// Lookup by id.
    pub fn get(&self, id: &str) -> Option<&CardDef> {
        self.cards.get(id)
    }

    /// Whether the catalog contains `id`.
    pub fn contains(&self, id: &str) -> bool {
        self.cards.contains_key(id)
    }

    /// Number of distinct cards.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// All ids in catalog order.
    pub fn ids(&self) -> impl Iterator<Item = &CardId> {
        self.cards.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_insert_get() {
        let mut c = CardCatalog::new();
        c.insert(CardDef::new("strike", "Strike", 1).with_tag("attack"));
        assert!(c.contains("strike"));
        assert_eq!(c.get("strike").unwrap().cost, 1);
        assert_eq!(c.get("strike").unwrap().tags, vec!["attack"]);
    }
}
