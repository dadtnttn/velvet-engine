//! # velvet-cards
//!
//! **Authoring tools only** (see `docs/architecture/TOOLS.md`).
//!
//! - Not a playable card game, TCG rules engine, AI, or match UI  
//! - Demos (`card-duel`, `velvet-stakes`) compose these tools; they are not this crate  
//!
//! ## Tools
//!
//! 1. [`CardCatalog`] — define cards (id, name, cost, tags/types).
//! 2. [`DeckList`] — ordered multiset of card ids + [`validate_deck`].
//! 3. [`CardZones`] — library / hand / discard with seeded shuffle, draw, moves.

#![deny(missing_docs)]

mod catalog;
mod deck;
mod error;
mod io;
mod zones;

pub use catalog::{CardCatalog, CardDef, CardId};
pub use deck::{validate_deck, DeckList, DeckRules, DeckValidation, DeckViolation};
pub use error::CardError;
pub use io::{load_catalog_json, load_deck_json, parse_catalog_json, parse_deck_json};
pub use zones::{shuffle_in_place, CardZones, ZoneKind};
