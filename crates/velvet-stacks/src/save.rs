//! Board saves: one serde document that restores the cards, stacks, jobs and random state of a board.
//!
//! The save holds the state only. The catalog is not in it: a board is loaded with a catalog, and
//! the load fails if the save names a card or recipe that catalog does not have. Events still
//! queued when the board is saved are not written; take them with [`Board::take_events`] first.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::board::{Board, Card, Job, Stack};
use crate::def::Catalog;
use crate::document::{check_envelope, json_error, FORMAT, VERSION};
use crate::error::StacksError;
use crate::rng::Xorshift64;

/// The save document. `rng` is the generator's state as a decimal string, so that JavaScript keeps
/// all 64 bits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoardSave {
    /// Always `velvet-stacks`.
    pub format: String,
    /// Always 1.
    pub version: u32,
    /// Always `board`.
    pub kind: String,
    /// The random generator's state, in decimal.
    pub rng: String,
    /// The id the next card gets.
    pub next_card: u64,
    /// The id the next stack gets.
    pub next_stack: u64,
    /// Every card on the board.
    pub cards: Vec<Card>,
    /// Every stack on the board, with its job if it runs one.
    pub stacks: Vec<StackSave>,
}

/// One stack in a save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StackSave {
    /// The stack's id.
    pub id: u64,
    /// Its cards, bottom to top.
    pub cards: Vec<u64>,
    /// The job it runs, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job: Option<JobSave>,
}

/// A running job in a save.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSave {
    /// The recipe the job runs.
    pub recipe: String,
    /// Seconds the job has run.
    pub elapsed: f32,
}

impl Board {
    /// The board's state as a save document.
    pub fn save(&self) -> BoardSave {
        let parts = self.parts();
        BoardSave {
            format: FORMAT.to_string(),
            version: VERSION,
            kind: "board".to_string(),
            rng: parts.rng.state().to_string(),
            next_card: parts.counters.0,
            next_stack: parts.counters.1,
            cards: parts.cards.values().cloned().collect(),
            stacks: parts
                .stacks
                .values()
                .map(|s| StackSave {
                    id: s.id,
                    cards: s.cards.clone(),
                    job: s.job.as_ref().map(|j| JobSave {
                        recipe: j.recipe.clone(),
                        elapsed: j.elapsed,
                    }),
                })
                .collect(),
        }
    }

    /// Restores a board from a save and the catalog it was played with.
    ///
    /// The save is checked: every card and recipe it names must exist, every card must sit in
    /// exactly one stack, and ids must lie below the counters. Afterwards every stack is matched once,
    /// so a save edited by hand cannot keep a job that no longer matches; the events of that check
    /// are not reported.
    pub fn load(catalog: Catalog, save: BoardSave) -> Result<Self, StacksError> {
        check_envelope(&save.format, save.version, &save.kind, "board")?;
        let state: u64 = save
            .rng
            .parse()
            .map_err(|_| StacksError::InvalidSave(format!("bad random state `{}`", save.rng)))?;
        let rng = Xorshift64::from_state(state)
            .ok_or_else(|| StacksError::InvalidSave("random state is zero".into()))?;

        let mut cards: BTreeMap<u64, Card> = BTreeMap::new();
        for card in save.cards {
            if catalog.card(&card.def).is_none() {
                return Err(StacksError::UnknownCard {
                    id: card.def,
                    context: format!("card {}", card.id),
                });
            }
            if card.id == 0 || card.id >= save.next_card {
                return Err(StacksError::InvalidSave(format!(
                    "card id {} is outside the counter",
                    card.id
                )));
            }
            if !(card.x.is_finite() && card.y.is_finite()) {
                return Err(StacksError::InvalidSave(format!(
                    "card {} has a position that is not a number",
                    card.id
                )));
            }
            let id = card.id;
            if cards.insert(id, card).is_some() {
                return Err(StacksError::InvalidSave(format!(
                    "card {id} is listed twice"
                )));
            }
        }

        let mut stacks: BTreeMap<u64, Stack> = BTreeMap::new();
        let mut placed: BTreeSet<u64> = BTreeSet::new();
        for stack in save.stacks {
            if stack.id == 0 || stack.id >= save.next_stack {
                return Err(StacksError::InvalidSave(format!(
                    "stack id {} is outside the counter",
                    stack.id
                )));
            }
            if stack.cards.is_empty() {
                return Err(StacksError::InvalidSave(format!(
                    "stack {} is empty",
                    stack.id
                )));
            }
            for card in &stack.cards {
                if !cards.contains_key(card) {
                    return Err(StacksError::InvalidSave(format!(
                        "stack {} holds unknown card {card}",
                        stack.id
                    )));
                }
                if !placed.insert(*card) {
                    return Err(StacksError::InvalidSave(format!(
                        "card {card} is in two stacks"
                    )));
                }
            }
            let job = match stack.job {
                None => None,
                Some(job) => {
                    if catalog.recipe(&job.recipe).is_none() {
                        return Err(StacksError::UnknownRecipe(job.recipe));
                    }
                    if !(job.elapsed.is_finite() && job.elapsed >= 0.0) {
                        return Err(StacksError::InvalidSave(format!(
                            "stack {} has a job time that is not a number",
                            stack.id
                        )));
                    }
                    Some(Job {
                        recipe: job.recipe,
                        elapsed: job.elapsed,
                    })
                }
            };
            let id = stack.id;
            let entry = Stack {
                id,
                cards: stack.cards,
                job,
            };
            if stacks.insert(id, entry).is_some() {
                return Err(StacksError::InvalidSave(format!(
                    "stack {id} is listed twice"
                )));
            }
        }
        if placed.len() != cards.len() {
            return Err(StacksError::InvalidSave("a card is in no stack".into()));
        }

        let mut board = Board::from_parts(
            catalog,
            rng,
            (save.next_card, save.next_stack),
            cards,
            stacks,
        );
        board.settle();
        board.take_events();
        Ok(board)
    }

    /// The board's state as pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, StacksError> {
        serde_json::to_string_pretty(&self.save()).map_err(json_error)
    }

    /// Restores a board from JSON written by [`Board::to_json`].
    pub fn from_json(catalog: Catalog, json: &str) -> Result<Self, StacksError> {
        let save: BoardSave = serde_json::from_str(json).map_err(json_error)?;
        Self::load(catalog, save)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::catalog;

    /// Plays the same script on `board` and returns every event it reported.
    fn play(board: &mut Board, ticks: usize) -> Vec<crate::BoardEvent> {
        let mut out = Vec::new();
        for _ in 0..ticks {
            out.extend(board.tick(0.5));
        }
        out
    }

    fn started_worker_board() -> Board {
        let mut b = Board::new(catalog(), 3);
        let forest = b.spawn("forest", (0.0, 0.0)).expect("forest");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.drop_on(worker, forest).expect("drop");
        b.take_events();
        b
    }

    #[test]
    fn a_loaded_board_behaves_like_the_original() {
        let mut original = started_worker_board();
        original.tick(1.0);
        let json = original.to_json().expect("writes");
        let mut restored = Board::from_json(catalog(), &json).expect("reads");
        assert_eq!(play(&mut original, 12), play(&mut restored, 12));
        assert_eq!(original.save(), restored.save());
    }

    #[test]
    fn the_random_state_is_saved() {
        let mut b = Board::new(catalog(), 11);
        let market = b.spawn("market", (0.0, 0.0)).expect("market");
        let berries = b.spawn("berries", (0.0, 0.0)).expect("berries");
        b.drop_on(berries, market).expect("drop");
        let second = b.spawn("berries", (0.0, 0.0)).expect("berries");
        b.drop_on(second, market).expect("drop");
        b.take_events();
        let mut restored = Board::load(catalog(), b.save()).expect("loads");
        assert_eq!(b.tick(1.0), restored.tick(1.0));
    }

    #[test]
    fn a_save_naming_unknown_cards_is_refused() {
        let mut save = started_worker_board().save();
        save.cards[0].def = "dragon".into();
        assert!(matches!(
            Board::load(catalog(), save),
            Err(StacksError::UnknownCard { .. })
        ));
    }

    #[test]
    fn a_save_with_a_card_in_two_stacks_is_refused() {
        let mut b = Board::new(catalog(), 3);
        b.spawn("wood", (0.0, 0.0)).expect("wood");
        b.spawn("stone", (0.0, 0.0)).expect("stone");
        let mut save = b.save();
        save.stacks[1].cards.push(1);
        assert!(matches!(
            Board::load(catalog(), save),
            Err(StacksError::InvalidSave(_))
        ));
    }

    #[test]
    fn a_save_of_another_document_is_refused() {
        let mut save = started_worker_board().save();
        save.kind = "catalog".into();
        assert!(matches!(
            Board::load(catalog(), save),
            Err(StacksError::UnsupportedDocument { .. })
        ));
    }
}
