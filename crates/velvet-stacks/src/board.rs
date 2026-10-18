//! The board: cards lying on a table, grouped into stacks, each stack running at most one job.
//!
//! Every action that changes a stack queues the events it caused and re-matches that one stack
//! (see [`crate::job`]). [`Board::take_events`] hands the queue out; [`Board::tick`] returns it too.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::def::{CardDef, Catalog};
use crate::error::StacksError;
use crate::placement::OutputPlacement;
use crate::rng::Xorshift64;

/// One card on the board. Every card of a stack carries the position of the stack's bottom card.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Card {
    /// Instance id: unique for the life of the board, counted from 1 in creation order.
    pub id: u64,
    /// The definition this card is an instance of.
    pub def: String,
    /// Horizontal position, in board units.
    pub x: f32,
    /// Vertical position, in board units.
    pub y: f32,
}

/// Something that happened on the board. Serialized with an `event` tag, so fixtures and UIs read
/// the same stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum BoardEvent {
    /// A stack began a job for `recipe`. The job starts at zero progress.
    JobStarted {
        /// The stack.
        stack: u64,
        /// The recipe the stack matches.
        recipe: String,
    },
    /// A running job stopped, because its stack no longer matches `recipe`. A job that is replaced
    /// by another recipe is cancelled and then started again.
    JobCancelled {
        /// The stack.
        stack: u64,
        /// The recipe that stopped.
        recipe: String,
    },
    /// A job completed. The cards it used up and the cards it made follow.
    JobFinished {
        /// The stack.
        stack: u64,
        /// The recipe that completed.
        recipe: String,
    },
    /// A card was created: by [`Board::spawn`], or as an output of a finished job.
    CardSpawned {
        /// The new card.
        card: u64,
        /// Its definition.
        def: String,
        /// Its position.
        x: f32,
        /// Its position.
        y: f32,
    },
    /// A finished job used this card up.
    CardConsumed {
        /// The card.
        card: u64,
        /// Its definition.
        def: String,
    },
    /// A stack's cards changed. `cards` lists them bottom to top; an empty list means the stack is gone.
    StackChanged {
        /// The stack.
        stack: u64,
        /// Its cards, bottom to top.
        cards: Vec<u64>,
    },
}

/// A pile of cards, bottom first, with the job it may be running. Crate-internal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Stack {
    pub(crate) id: u64,
    pub(crate) cards: Vec<u64>,
    pub(crate) job: Option<Job>,
}

/// A running job: the recipe and the seconds it has run. Crate-internal.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Job {
    pub(crate) recipe: String,
    pub(crate) elapsed: f32,
}

/// Borrowed view of the state a save writes. Crate-internal.
pub(crate) struct SavedParts<'a> {
    pub(crate) rng: &'a Xorshift64,
    pub(crate) counters: (u64, u64),
    pub(crate) cards: &'a BTreeMap<u64, Card>,
    pub(crate) stacks: &'a BTreeMap<u64, Stack>,
}

/// The table: every card, every stack, every running job, and the random state.
///
/// The board plays with the [`Catalog`] it was built with. Its state, not the catalog, is what
/// [`Board::save`] writes.
#[derive(Debug, Clone)]
pub struct Board {
    pub(crate) catalog: Catalog,
    pub(crate) rng: Xorshift64,
    pub(crate) next_card: u64,
    pub(crate) next_stack: u64,
    pub(crate) cards: BTreeMap<u64, Card>,
    pub(crate) stacks: BTreeMap<u64, Stack>,
    pub(crate) stack_of_card: BTreeMap<u64, u64>,
    pub(crate) dirty: BTreeSet<u64>,
    pub(crate) running: BTreeSet<u64>,
    pub(crate) pending: Vec<BoardEvent>,
    /// Where finished jobs put their copies. Configuration, so it is not in a save.
    pub(crate) output_placement: OutputPlacement,
}

impl Board {
    /// An empty board that plays with `catalog`. `seed` drives every chance roll. The outputs of
    /// finished jobs use the row placement until [`Board::set_output_placement`] changes it.
    pub fn new(catalog: Catalog, seed: u64) -> Self {
        Self {
            catalog,
            rng: Xorshift64::new(seed),
            next_card: 1,
            next_stack: 1,
            cards: BTreeMap::new(),
            stacks: BTreeMap::new(),
            stack_of_card: BTreeMap::new(),
            dirty: BTreeSet::new(),
            running: BTreeSet::new(),
            pending: Vec::new(),
            output_placement: OutputPlacement::Row,
        }
    }

    /// The catalog this board plays with.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// The card with instance id `id`.
    pub fn card(&self, id: u64) -> Option<&Card> {
        self.cards.get(&id)
    }

    /// Every card on the board, by id.
    pub fn cards(&self) -> impl Iterator<Item = &Card> + '_ {
        self.cards.values()
    }

    /// Every stack on the board, as (stack id, its cards bottom to top), by stack id.
    pub fn stacks(&self) -> impl Iterator<Item = (u64, &[u64])> + '_ {
        self.stacks.values().map(|s| (s.id, s.cards.as_slice()))
    }

    /// The cards of the stack that holds `card`, bottom to top.
    pub fn stack_of(&self, card: u64) -> Option<&[u64]> {
        let stack = self.stack_of_card.get(&card)?;
        self.stacks.get(stack).map(|s| s.cards.as_slice())
    }

    /// The id of the stack that holds `card`.
    pub fn stack_id_of(&self, card: u64) -> Option<u64> {
        self.stack_of_card.get(&card).copied()
    }

    /// Returns and clears the events queued since the last call.
    pub fn take_events(&mut self) -> Vec<BoardEvent> {
        std::mem::take(&mut self.pending)
    }

    /// Creates a card of definition `def` at `pos`, in a stack of its own, and returns its id.
    pub fn spawn(&mut self, def: &str, pos: (f32, f32)) -> Result<u64, StacksError> {
        if self.catalog.card(def).is_none() {
            return Err(StacksError::UnknownCard {
                id: def.to_string(),
                context: "spawn".to_string(),
            });
        }
        let card = self.create_card(def, pos);
        self.settle();
        Ok(card)
    }

    /// Moves the whole stack that holds `card` so that its bottom card sits at `pos`. Moving queues
    /// no events and never changes a job.
    pub fn move_stack(&mut self, card: u64, pos: (f32, f32)) -> Result<(), StacksError> {
        let stack = self.stack_id(card)?;
        for id in self.stack_cards(stack).to_vec() {
            if let Some(c) = self.cards.get_mut(&id) {
                c.x = pos.0;
                c.y = pos.1;
            }
        }
        Ok(())
    }

    /// Drops the stack from `card` upward onto the stack that holds `target`.
    ///
    /// The landing card is the top card of the target stack. Its [`crate::StackRule`] decides: it
    /// must accept the bottom card of the dragged stack, and the stack may not grow past its `max`.
    /// The dragged cards take the target's position.
    pub fn drop_on(&mut self, card: u64, target: u64) -> Result<(), StacksError> {
        let source = self.stack_id(card)?;
        let dest = self.stack_id(target)?;
        if source == dest {
            return Err(StacksError::SameStack(card));
        }
        let src_cards = self.stack_cards(source).to_vec();
        let dst_cards = self.stack_cards(dest).to_vec();
        let Some(&top) = dst_cards.last() else {
            return Err(StacksError::NoSuchCard(target));
        };
        let Some(index) = src_cards.iter().position(|&c| c == card) else {
            return Err(StacksError::NoSuchCard(card));
        };
        let moved = src_cards[index..].to_vec();
        let (Some(dragged), Some(landing)) = (self.def_of(card), self.def_of(top)) else {
            return Err(StacksError::NoSuchCard(card));
        };
        if !landing.stack.accepts_card(dragged) {
            return Err(StacksError::DropRefused { card, onto: top });
        }
        let height = dst_cards.len() + moved.len();
        if height > landing.stack.max as usize {
            return Err(StacksError::StackFull {
                onto: top,
                max: landing.stack.max,
                height: height as u32,
            });
        }
        let Some((x, y)) = self.stack_position(dest) else {
            return Err(StacksError::NoSuchCard(target));
        };
        if let Some(s) = self.stacks.get_mut(&source) {
            s.cards.truncate(index);
        }
        if let Some(d) = self.stacks.get_mut(&dest) {
            d.cards.extend_from_slice(&moved);
        }
        for id in &moved {
            self.stack_of_card.insert(*id, dest);
            if let Some(c) = self.cards.get_mut(id) {
                c.x = x;
                c.y = y;
            }
        }
        self.touch_stack(source);
        self.touch_stack(dest);
        self.settle();
        Ok(())
    }

    /// Takes `card` and the cards above it off their stack into a new stack, and returns the new
    /// stack's id. Splitting the bottom card changes nothing and returns the stack it is in.
    pub fn split(&mut self, card: u64) -> Result<u64, StacksError> {
        let stack = self.stack_id(card)?;
        let cards = self.stack_cards(stack).to_vec();
        let Some(index) = cards.iter().position(|&c| c == card) else {
            return Err(StacksError::NoSuchCard(card));
        };
        if index == 0 {
            return Ok(stack);
        }
        let moved = cards[index..].to_vec();
        let fresh = self.next_stack;
        self.next_stack += 1;
        if let Some(s) = self.stacks.get_mut(&stack) {
            s.cards.truncate(index);
        }
        for id in &moved {
            self.stack_of_card.insert(*id, fresh);
        }
        self.stacks.insert(
            fresh,
            Stack {
                id: fresh,
                cards: moved.clone(),
                job: None,
            },
        );
        self.touch_stack(stack);
        self.dirty.insert(fresh);
        self.pending.push(BoardEvent::StackChanged {
            stack: fresh,
            cards: moved,
        });
        self.settle();
        Ok(fresh)
    }

    /// Takes `card` off the board. The cards above it stay in the same stack.
    pub fn remove(&mut self, card: u64) -> Result<(), StacksError> {
        let stack = self.stack_id(card)?;
        if let Some(s) = self.stacks.get_mut(&stack) {
            s.cards.retain(|&c| c != card);
        }
        self.cards.remove(&card);
        self.stack_of_card.remove(&card);
        self.touch_stack(stack);
        self.settle();
        Ok(())
    }

    /// Creates a card in a stack of its own, queues its events, and marks the stack for matching.
    pub(crate) fn create_card(&mut self, def: &str, pos: (f32, f32)) -> u64 {
        let (x, y) = pos;
        let card = self.next_card;
        self.next_card += 1;
        let stack = self.next_stack;
        self.next_stack += 1;
        self.cards.insert(
            card,
            Card {
                id: card,
                def: def.to_string(),
                x,
                y,
            },
        );
        self.stack_of_card.insert(card, stack);
        self.stacks.insert(
            stack,
            Stack {
                id: stack,
                cards: vec![card],
                job: None,
            },
        );
        self.pending.push(BoardEvent::CardSpawned {
            card,
            def: def.to_string(),
            x,
            y,
        });
        self.pending.push(BoardEvent::StackChanged {
            stack,
            cards: vec![card],
        });
        self.dirty.insert(stack);
        card
    }

    /// Announces that the cards of `stack` changed, and marks it for matching. A stack left empty
    /// is removed, and its job (if any) is cancelled first.
    pub(crate) fn touch_stack(&mut self, stack: u64) {
        let cards = self.stack_cards(stack).to_vec();
        if cards.is_empty() {
            self.running.remove(&stack);
            if let Some(job) = self.stacks.remove(&stack).and_then(|s| s.job) {
                self.pending.push(BoardEvent::JobCancelled {
                    stack,
                    recipe: job.recipe,
                });
            }
            self.dirty.remove(&stack);
        } else {
            self.dirty.insert(stack);
        }
        self.pending.push(BoardEvent::StackChanged { stack, cards });
    }

    /// The stack that holds `card`, or [`StacksError::NoSuchCard`].
    pub(crate) fn stack_id(&self, card: u64) -> Result<u64, StacksError> {
        self.stack_of_card
            .get(&card)
            .copied()
            .ok_or(StacksError::NoSuchCard(card))
    }

    /// The cards of `stack`, bottom to top; empty when there is no such stack.
    pub(crate) fn stack_cards(&self, stack: u64) -> &[u64] {
        match self.stacks.get(&stack) {
            Some(s) => s.cards.as_slice(),
            None => &[],
        }
    }

    /// The position of a stack: the position of its bottom card.
    pub(crate) fn stack_position(&self, stack: u64) -> Option<(f32, f32)> {
        let bottom = self.stacks.get(&stack)?.cards.first()?;
        let card = self.cards.get(bottom)?;
        Some((card.x, card.y))
    }

    /// The definition of `card`.
    pub(crate) fn def_of(&self, card: u64) -> Option<&CardDef> {
        let instance = self.cards.get(&card)?;
        self.catalog.card(&instance.def)
    }

    /// The random generator, for chance rolls.
    pub(crate) fn rng_mut(&mut self) -> &mut Xorshift64 {
        &mut self.rng
    }

    /// The state a save writes: the generator, the two id counters, the cards and the stacks.
    pub(crate) fn parts(&self) -> SavedParts<'_> {
        SavedParts {
            rng: &self.rng,
            counters: (self.next_card, self.next_stack),
            cards: &self.cards,
            stacks: &self.stacks,
        }
    }

    /// Rebuilds a board from the state a save holds. The caller has checked that state. Every stack
    /// starts marked for matching, so the first [`Board::tick`] or action re-checks it.
    pub(crate) fn from_parts(
        catalog: Catalog,
        rng: Xorshift64,
        counters: (u64, u64),
        cards: BTreeMap<u64, Card>,
        stacks: BTreeMap<u64, Stack>,
    ) -> Self {
        let stack_of_card = stacks
            .values()
            .flat_map(|s| s.cards.iter().map(move |&card| (card, s.id)))
            .collect();
        let dirty = stacks.keys().copied().collect();
        let running = stacks
            .values()
            .filter(|s| s.job.is_some())
            .map(|s| s.id)
            .collect();
        Self {
            catalog,
            rng,
            next_card: counters.0,
            next_stack: counters.1,
            cards,
            stacks,
            stack_of_card,
            dirty,
            running,
            pending: Vec::new(),
            // The placement is not in a save: a loaded board starts with the row, and a game sets its own.
            output_placement: OutputPlacement::Row,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::catalog;

    fn board() -> Board {
        Board::new(catalog(), 1)
    }

    #[test]
    fn spawn_announces_the_card_and_its_stack() {
        let mut b = board();
        let id = b.spawn("wood", (2.0, 3.0)).expect("known card");
        assert_eq!(id, 1);
        assert_eq!(b.stack_of(id), Some(&[1][..]));
        assert_eq!(
            b.take_events(),
            vec![
                BoardEvent::CardSpawned {
                    card: 1,
                    def: "wood".into(),
                    x: 2.0,
                    y: 3.0
                },
                BoardEvent::StackChanged {
                    stack: 1,
                    cards: vec![1]
                },
            ]
        );
    }

    #[test]
    fn spawn_refuses_unknown_definitions() {
        let mut b = board();
        assert!(matches!(
            b.spawn("ghost", (0.0, 0.0)),
            Err(StacksError::UnknownCard { .. })
        ));
    }

    #[test]
    fn drop_puts_the_stack_on_the_target_and_takes_its_position() {
        let mut b = board();
        let forest = b.spawn("forest", (5.0, 5.0)).expect("forest");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.take_events();
        b.drop_on(worker, forest)
            .expect("a worker may stand on a forest");
        assert_eq!(b.stack_of(forest), Some(&[forest, worker][..]));
        assert_eq!(b.card(worker).map(|c| (c.x, c.y)), Some((5.0, 5.0)));
        assert_eq!(
            b.take_events(),
            vec![
                BoardEvent::StackChanged {
                    stack: 2,
                    cards: vec![]
                },
                BoardEvent::StackChanged {
                    stack: 1,
                    cards: vec![forest, worker]
                },
                BoardEvent::JobStarted {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
            ]
        );
    }

    #[test]
    fn drops_are_refused_by_the_landing_rule() {
        let mut b = board();
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        let forest = b.spawn("forest", (0.0, 0.0)).expect("forest");
        // A forest accepts only characters.
        assert_eq!(
            b.drop_on(wood, forest),
            Err(StacksError::DropRefused {
                card: wood,
                onto: forest
            })
        );
        let house = b.spawn("house", (0.0, 0.0)).expect("house");
        let wood2 = b.spawn("wood", (0.0, 0.0)).expect("wood");
        // A house holds one card; a stack that would hold two is full.
        assert_eq!(
            b.drop_on(wood2, house),
            Err(StacksError::StackFull {
                onto: house,
                max: 1,
                height: 2
            })
        );
        assert_eq!(b.drop_on(wood, wood), Err(StacksError::SameStack(wood)));
    }

    #[test]
    fn split_takes_the_card_and_those_above_it() {
        let mut b = board();
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        let stone = b.spawn("stone", (0.0, 0.0)).expect("stone");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.drop_on(stone, wood).expect("stone on wood");
        b.drop_on(worker, stone).expect("worker on stone");
        let fresh = b.split(stone).expect("split");
        assert_eq!(b.stack_of(wood), Some(&[wood][..]));
        assert_eq!(b.stack_of(stone), Some(&[stone, worker][..]));
        assert_eq!(b.stack_id_of(stone), Some(fresh));
        let bottom = b.stack_id_of(wood).expect("wood has a stack");
        assert_eq!(b.split(wood).expect("bottom split"), bottom);
    }

    #[test]
    fn remove_keeps_the_cards_above_in_place() {
        let mut b = board();
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        let stone = b.spawn("stone", (0.0, 0.0)).expect("stone");
        b.drop_on(worker, wood).expect("drop");
        b.drop_on(stone, wood).expect("drop");
        b.remove(worker).expect("remove");
        assert_eq!(b.stack_of(wood), Some(&[wood, stone][..]));
        assert!(b.card(worker).is_none());
        assert!(matches!(b.remove(worker), Err(StacksError::NoSuchCard(_))));
    }

    #[test]
    fn move_stack_moves_every_card_of_the_stack() {
        let mut b = board();
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.drop_on(worker, wood).expect("drop");
        b.move_stack(worker, (9.0, -1.0)).expect("move");
        assert_eq!(b.card(wood).map(|c| (c.x, c.y)), Some((9.0, -1.0)));
        assert_eq!(b.card(worker).map(|c| (c.x, c.y)), Some((9.0, -1.0)));
    }
}
