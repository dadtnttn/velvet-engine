//! # velvet-stacks
//!
//! Cards on a board, in the style of Stacklands: cards lie on a table and stack on each other, and
//! a stack whose cards match a recipe starts a timed job. The job uses up some of its cards and
//! makes others.
//!
//! The engine is pure logic. It does no rendering, reads no files, starts no threads and keeps no
//! clock: the caller passes the time that went by. It is deterministic. Its only randomness is a
//! seeded xorshift generator kept in the board's state, so the same catalog, seed and actions give
//! the same events on every platform. The rules are meant to run in WebAssembly for the JavaScript
//! runtime; the format is in `docs/formats/STACKS.md`.
//!
//! The tools, in the order a game meets them:
//! 1. [`Catalog`]: card definitions and recipes, read from JSON and checked once.
//! 2. [`Board`]: spawn, move, drop, split and remove cards; [`Board::tick`] advances the jobs.
//! 3. [`Board::save`] and [`Board::load`]: the whole state as serde data.

#![deny(missing_docs)]

mod board;
mod def;
mod document;
mod error;
mod job;
mod placement;
mod recipe;
mod rng;
mod save;

#[cfg(test)]
mod test_support;

pub use board::{Board, BoardEvent, Card};
pub use def::{CardDef, Catalog, Selector, StackRule, DEFAULT_MAX_STACK};
pub use document::{FORMAT, VERSION};
pub use error::StacksError;
pub use job::{MAX_TICK, OUTPUT_STEP};
pub use placement::OutputPlacement;
pub use recipe::{Ingredient, Output, Recipe};
pub use rng::Xorshift64;
pub use save::{BoardSave, JobSave, StackSave};
