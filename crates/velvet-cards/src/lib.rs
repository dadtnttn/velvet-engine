//! # velvet-cards
//!
//! **Authoring tools** for card catalogs, deck lists, and pure zone math.
//!
//! This crate is **not** a playable card game: no AI, no turn engine, no match UI.
//! Games and Studio panels can build on these APIs later.
//!
//! ## Spine
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
