//! Deck lists and validation against a catalog.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::catalog::{CardCatalog, CardId};
use crate::error::CardError;

/// Ordered multiset of card ids (list order = default library order before shuffle).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckList {
    /// Card ids; duplicates allowed (copies).
    #[serde(default)]
    pub cards: Vec<CardId>,
    /// Optional author label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl DeckList {
    /// Empty deck.
    pub fn new() -> Self {
        Self::default()
    }

    /// From an iterator of ids.
    pub fn from_ids(ids: impl IntoIterator<Item = impl Into<CardId>>) -> Self {
        Self {
            cards: ids.into_iter().map(Into::into).collect(),
            name: None,
        }
    }

    /// Total cards (counting copies).
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Empty?
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Count copies of each id.
    pub fn counts(&self) -> HashMap<&str, usize> {
        let mut m = HashMap::new();
        for id in &self.cards {
            *m.entry(id.as_str()).or_insert(0) += 1;
        }
        m
    }
}

/// Optional constraints applied during validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckRules {
    /// Maximum total cards (None = unlimited).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_size: Option<usize>,
    /// Minimum total cards (None = no minimum).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_size: Option<usize>,
    /// Maximum copies of any single card id (None = unlimited).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_copies: Option<usize>,
}

impl Default for DeckRules {
    fn default() -> Self {
        Self {
            max_size: None,
            min_size: None,
            max_copies: None,
        }
    }
}

impl DeckRules {
    /// Common “constructed” style: 40–60 cards, max 3 copies.
    pub fn constructed_default() -> Self {
        Self {
            min_size: Some(40),
            max_size: Some(60),
            max_copies: Some(3),
        }
    }

    /// Loose authoring: only require known cards (no size/copy caps).
    pub fn open() -> Self {
        Self::default()
    }
}

/// One validation problem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeckViolation {
    /// Id not in catalog.
    UnknownCard {
        /// Bad id.
        id: CardId,
        /// First list index where it appears.
        index: usize,
    },
    /// Total size above max.
    TooManyCards {
        /// Actual size.
        size: usize,
        /// Allowed max.
        max: usize,
    },
    /// Total size below min.
    TooFewCards {
        /// Actual size.
        size: usize,
        /// Required min.
        min: usize,
    },
    /// Too many copies of one id.
    TooManyCopies {
        /// Card id.
        id: CardId,
        /// Count found.
        count: usize,
        /// Allowed max.
        max: usize,
    },
}

/// Result of validating a deck list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckValidation {
    /// True when `violations` is empty.
    pub ok: bool,
    /// Problems found.
    pub violations: Vec<DeckViolation>,
    /// Total cards checked.
    pub size: usize,
}

impl DeckValidation {
    /// Convert to `Result` for call sites that prefer errors.
    pub fn into_result(self) -> Result<(), CardError> {
        if self.ok {
            Ok(())
        } else {
            Err(CardError::ValidationFailed(self.violations.len()))
        }
    }
}

/// Validate `deck` against `catalog` and optional `rules`.
pub fn validate_deck(catalog: &CardCatalog, deck: &DeckList, rules: &DeckRules) -> DeckValidation {
    let mut violations = Vec::new();
    let size = deck.len();

    for (index, id) in deck.cards.iter().enumerate() {
        if !catalog.contains(id) {
            // One violation per first occurrence of each unknown id is nicer,
            // but listing every index is more precise for tooling — report first only per id.
            if !violations
                .iter()
                .any(|v| matches!(v, DeckViolation::UnknownCard { id: u, .. } if u == id))
            {
                violations.push(DeckViolation::UnknownCard {
                    id: id.clone(),
                    index,
                });
            }
        }
    }

    if let Some(max) = rules.max_size {
        if size > max {
            violations.push(DeckViolation::TooManyCards { size, max });
        }
    }
    if let Some(min) = rules.min_size {
        if size < min {
            violations.push(DeckViolation::TooFewCards { size, min });
        }
    }
    if let Some(max_copies) = rules.max_copies {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for id in &deck.cards {
            *counts.entry(id.as_str()).or_insert(0) += 1;
        }
        for (id, count) in counts {
            if count > max_copies {
                violations.push(DeckViolation::TooManyCopies {
                    id: id.to_string(),
                    count,
                    max: max_copies,
                });
            }
        }
    }

    DeckValidation {
        ok: violations.is_empty(),
        violations,
        size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::CardDef;

    fn sample_catalog() -> CardCatalog {
        let mut c = CardCatalog::new();
        c.insert(CardDef::new("strike", "Strike", 1));
        c.insert(CardDef::new("guard", "Guard", 1));
        c.insert(CardDef::new("fireball", "Fireball", 3).with_tag("spell"));
        c
    }

    #[test]
    fn valid_open_deck() {
        let cat = sample_catalog();
        let deck = DeckList::from_ids(["strike", "strike", "guard", "fireball"]);
        let v = validate_deck(&cat, &deck, &DeckRules::open());
        assert!(v.ok, "{:?}", v.violations);
        assert_eq!(v.size, 4);
    }

    #[test]
    fn rejects_unknown_card() {
        let cat = sample_catalog();
        let deck = DeckList::from_ids(["strike", "nope"]);
        let v = validate_deck(&cat, &deck, &DeckRules::open());
        assert!(!v.ok);
        assert!(matches!(
            &v.violations[0],
            DeckViolation::UnknownCard { id, index: 1 } if id == "nope"
        ));
    }

    #[test]
    fn rejects_too_many_copies() {
        let cat = sample_catalog();
        let deck = DeckList::from_ids(["strike", "strike", "strike", "strike"]);
        let rules = DeckRules {
            max_copies: Some(3),
            ..DeckRules::default()
        };
        let v = validate_deck(&cat, &deck, &rules);
        assert!(!v.ok);
        assert!(v
            .violations
            .iter()
            .any(|x| matches!(x, DeckViolation::TooManyCopies { count: 4, .. })));
    }

    #[test]
    fn rejects_size_bounds() {
        let cat = sample_catalog();
        let deck = DeckList::from_ids(["strike", "guard"]);
        let rules = DeckRules {
            min_size: Some(3),
            max_size: Some(10),
            max_copies: None,
        };
        let v = validate_deck(&cat, &deck, &rules);
        assert!(!v.ok);
        assert!(matches!(
            v.violations[0],
            DeckViolation::TooFewCards { size: 2, min: 3 }
        ));
    }
}
