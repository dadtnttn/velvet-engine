//! Card definitions, selectors and stack rules, and the catalog that holds them with their recipes.
//!
//! A catalog is checked when it is built, so every id a later lookup can meet is known to exist.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::document::{check_envelope, json_error, FORMAT, VERSION};
use crate::error::StacksError;
use crate::recipe::Recipe;

/// The stack height a card allows when its definition leaves `max` out.
pub const DEFAULT_MAX_STACK: u32 = 30;

/// One kind of card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDef {
    /// Id, unique in the catalog. Recipes and stack rules name cards by it.
    pub id: String,
    /// Display name, as an i18n key. The engine never shows it.
    pub name: String,
    /// What the card is: `character`, `resource`, `building`, `zone`, `item`, and so on. Free text.
    pub kind: String,
    /// Free tags, such as `gather`. A selector can match on one.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Which cards may lie on this one, and how high the stack built on it may grow.
    #[serde(default)]
    pub stack: StackRule,
    /// Value in the game's currency. The engine does not read it.
    #[serde(default)]
    pub value: i32,
}

/// Picks cards. Every field that is set must match; a selector with no field set matches every card.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selector {
    /// The card's id is exactly this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The card's kind is exactly this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The card carries this tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

impl Selector {
    /// Whether `def` satisfies every field this selector sets.
    pub fn matches(&self, def: &CardDef) -> bool {
        self.id.as_deref().is_none_or(|id| def.id == id)
            && self.kind.as_deref().is_none_or(|kind| def.kind == kind)
            && self
                .tag
                .as_deref()
                .is_none_or(|tag| def.tags.iter().any(|t| t == tag))
    }
}

/// What may be dropped onto a card, and how many cards the stack may hold after a drop onto it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackRule {
    /// Cards that may be dropped onto this one. Empty means any card.
    #[serde(default)]
    pub accepts: Vec<Selector>,
    /// The most cards the stack may hold after a drop onto this card (this card is then the top).
    #[serde(default = "default_max")]
    pub max: u32,
}

fn default_max() -> u32 {
    DEFAULT_MAX_STACK
}

impl Default for StackRule {
    fn default() -> Self {
        Self {
            accepts: Vec::new(),
            max: DEFAULT_MAX_STACK,
        }
    }
}

impl StackRule {
    /// Whether a card of `def` may be dropped onto a card with this rule.
    pub fn accepts_card(&self, def: &CardDef) -> bool {
        self.accepts.is_empty() || self.accepts.iter().any(|s| s.matches(def))
    }
}

/// Every card definition and recipe of a game, checked once when it is built.
///
/// Recipes are kept in precedence order (see [`Recipe::cmp_precedence`]): the first recipe in
/// [`Catalog::recipes`] that matches a stack is the one that runs.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "CatalogDoc")]
pub struct Catalog {
    cards: IndexMap<String, CardDef>,
    recipes: Vec<Recipe>,
}

/// The catalog document as JSON sees it. Use [`Catalog::from_json`] to read one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CatalogDoc {
    format: String,
    version: u32,
    kind: String,
    cards: Vec<CardDef>,
    #[serde(default)]
    recipes: Vec<Recipe>,
}

impl Catalog {
    /// Builds a catalog from card definitions and recipes, checking every rule of [`StacksError`].
    pub fn new(cards: Vec<CardDef>, mut recipes: Vec<Recipe>) -> Result<Self, StacksError> {
        let mut map: IndexMap<String, CardDef> = IndexMap::with_capacity(cards.len());
        for card in cards {
            if card.id.is_empty() {
                return Err(StacksError::EmptyCardId);
            }
            if card.stack.max == 0 {
                return Err(StacksError::ZeroMax { card: card.id });
            }
            if map.contains_key(&card.id) {
                return Err(StacksError::DuplicateCard(card.id));
            }
            map.insert(card.id.clone(), card);
        }
        let known = |id: &str| map.contains_key(id);
        for card in map.values() {
            let context = format!("the stack rule of card `{}`", card.id);
            for selector in &card.stack.accepts {
                check_selector(selector, &known, &context)?;
            }
        }
        {
            let mut ids = BTreeSet::new();
            for recipe in &recipes {
                recipe.validate(&known)?;
                if !ids.insert(recipe.id.as_str()) {
                    return Err(StacksError::DuplicateRecipe(recipe.id.clone()));
                }
            }
        }
        recipes.sort_by(Recipe::cmp_precedence);
        Ok(Self {
            cards: map,
            recipes,
        })
    }

    /// Reads a catalog document (`"kind": "catalog"`, format `velvet-stacks`, version 1).
    pub fn from_json(json: &str) -> Result<Self, StacksError> {
        let doc: CatalogDoc = serde_json::from_str(json).map_err(json_error)?;
        Self::try_from(doc)
    }

    /// Writes the catalog as a pretty-printed document. Recipes are written in precedence order.
    pub fn to_json(&self) -> Result<String, StacksError> {
        serde_json::to_string_pretty(&self.to_doc()).map_err(json_error)
    }

    /// The definition of card `id`.
    pub fn card(&self, id: &str) -> Option<&CardDef> {
        self.cards.get(id)
    }

    /// Every card definition, in the order the document lists them.
    pub fn cards(&self) -> impl Iterator<Item = &CardDef> + '_ {
        self.cards.values()
    }

    /// Every recipe, in precedence order.
    pub fn recipes(&self) -> &[Recipe] {
        &self.recipes
    }

    /// The recipe with id `id`.
    pub fn recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.iter().find(|r| r.id == id)
    }

    fn to_doc(&self) -> CatalogDoc {
        CatalogDoc {
            format: FORMAT.to_string(),
            version: VERSION,
            kind: "catalog".to_string(),
            cards: self.cards.values().cloned().collect(),
            recipes: self.recipes.clone(),
        }
    }
}

impl TryFrom<CatalogDoc> for Catalog {
    type Error = StacksError;

    fn try_from(doc: CatalogDoc) -> Result<Self, Self::Error> {
        check_envelope(&doc.format, doc.version, &doc.kind, "catalog")?;
        Self::new(doc.cards, doc.recipes)
    }
}

impl Serialize for Catalog {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_doc().serialize(serializer)
    }
}

/// Checks that a selector naming a card by id names one the catalog has. `context` says where the
/// selector sits, for the error message.
pub(crate) fn check_selector(
    selector: &Selector,
    known: &impl Fn(&str) -> bool,
    context: &str,
) -> Result<(), StacksError> {
    match selector.id.as_deref() {
        Some(id) if !known(id) => Err(StacksError::UnknownCard {
            id: id.to_string(),
            context: context.to_string(),
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe::{Ingredient, Output};

    fn card(id: &str, kind: &str, tags: &[&str]) -> CardDef {
        CardDef {
            id: id.to_string(),
            name: format!("card.{id}"),
            kind: kind.to_string(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            stack: StackRule::default(),
            value: 0,
        }
    }

    fn recipe(id: &str, priority: i32, ingredient: &str, out: &str) -> Recipe {
        Recipe {
            id: id.to_string(),
            ingredients: vec![Ingredient {
                select: Selector {
                    id: Some(ingredient.to_string()),
                    ..Selector::default()
                },
                count: 1,
                consume: true,
            }],
            seconds: 1.0,
            outputs: vec![Output {
                card: out.to_string(),
                count: 1,
                chance: 1.0,
            }],
            priority,
        }
    }

    fn two_cards() -> Vec<CardDef> {
        vec![
            card("wood", "resource", &["material"]),
            card("plank", "resource", &[]),
        ]
    }

    #[test]
    fn selectors_match_on_every_field_they_set() {
        let worker = card("worker", "character", &["gather"]);
        assert!(Selector::default().matches(&worker));
        let by_id = Selector {
            id: Some("worker".into()),
            ..Selector::default()
        };
        assert!(by_id.matches(&worker));
        let wrong_id = Selector {
            id: Some("forest".into()),
            ..Selector::default()
        };
        assert!(!wrong_id.matches(&worker));
        let both = Selector {
            kind: Some("character".into()),
            tag: Some("gather".into()),
            ..Selector::default()
        };
        assert!(both.matches(&worker));
        let wrong_tag = Selector {
            tag: Some("wood".into()),
            ..Selector::default()
        };
        assert!(!wrong_tag.matches(&worker));
    }

    #[test]
    fn empty_accepts_means_anything() {
        let worker = card("worker", "character", &[]);
        let wood = card("wood", "resource", &[]);
        assert!(StackRule::default().accepts_card(&wood));
        let only_characters = StackRule {
            accepts: vec![Selector {
                kind: Some("character".into()),
                ..Selector::default()
            }],
            max: 2,
        };
        assert!(only_characters.accepts_card(&worker));
        assert!(!only_characters.accepts_card(&wood));
    }

    #[test]
    fn reads_a_minimal_catalog_document() {
        let json = r#"{"format":"velvet-stacks","version":1,"kind":"catalog",
            "cards":[{"id":"wood","name":"card.wood","kind":"resource"}]}"#;
        let catalog = Catalog::from_json(json).expect("valid");
        let wood = catalog.card("wood").expect("present");
        assert_eq!(wood.stack.max, DEFAULT_MAX_STACK);
        assert!(wood.tags.is_empty());
        assert!(catalog.recipes().is_empty());
    }

    #[test]
    fn refuses_a_board_document_as_a_catalog() {
        let json = r#"{"format":"velvet-stacks","version":1,"kind":"board","cards":[]}"#;
        assert!(matches!(
            Catalog::from_json(json),
            Err(StacksError::UnsupportedDocument { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_and_empty_card_ids() {
        let dup = Catalog::new(vec![card("a", "x", &[]), card("a", "x", &[])], vec![]);
        assert_eq!(dup.unwrap_err(), StacksError::DuplicateCard("a".into()));
        let empty = Catalog::new(vec![card("", "x", &[])], vec![]);
        assert_eq!(empty.unwrap_err(), StacksError::EmptyCardId);
    }

    #[test]
    fn rejects_a_zero_stack_limit() {
        let mut wood = card("wood", "resource", &[]);
        wood.stack.max = 0;
        let err = Catalog::new(vec![wood], vec![]).unwrap_err();
        assert!(matches!(err, StacksError::ZeroMax { .. }));
    }

    #[test]
    fn rejects_stack_rules_and_recipes_that_name_unknown_cards() {
        let mut wood = card("wood", "resource", &[]);
        wood.stack.accepts = vec![Selector {
            id: Some("ghost".into()),
            ..Selector::default()
        }];
        let err = Catalog::new(vec![wood], vec![]).unwrap_err();
        assert!(matches!(err, StacksError::UnknownCard { ref id, .. } if id == "ghost"));

        let err = Catalog::new(two_cards(), vec![recipe("r", 0, "ghost", "plank")]).unwrap_err();
        assert!(matches!(err, StacksError::UnknownCard { ref id, .. } if id == "ghost"));
        let err = Catalog::new(two_cards(), vec![recipe("r", 0, "wood", "ghost")]).unwrap_err();
        assert!(matches!(err, StacksError::UnknownCard { ref id, .. } if id == "ghost"));
    }

    #[test]
    fn rejects_duplicate_recipe_ids() {
        let recipes = vec![
            recipe("r", 0, "wood", "plank"),
            recipe("r", 1, "wood", "plank"),
        ];
        let err = Catalog::new(two_cards(), recipes).unwrap_err();
        assert_eq!(err, StacksError::DuplicateRecipe("r".into()));
    }

    #[test]
    fn keeps_recipes_in_precedence_order() {
        let recipes = vec![
            recipe("b0", 0, "wood", "plank"),
            recipe("p5", 5, "wood", "plank"),
            recipe("a0", 0, "wood", "plank"),
        ];
        let catalog = Catalog::new(two_cards(), recipes).expect("valid");
        let ids: Vec<&str> = catalog.recipes().iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["p5", "a0", "b0"]);
    }

    #[test]
    fn round_trips_through_json() {
        let catalog =
            Catalog::new(two_cards(), vec![recipe("r", 2, "wood", "plank")]).expect("valid");
        let json = catalog.to_json().expect("writes");
        assert_eq!(Catalog::from_json(&json).expect("reads"), catalog);
    }
}
