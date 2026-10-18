//! Jobs: matching stacks against the recipes, and starting, keeping, cancelling and finishing jobs.
//!
//! A stack is matched only when it changes (it is marked dirty), so the work per change is the size
//! of that stack times the number of recipes, never the size of the board. A stack runs the first
//! recipe, in precedence order, that matches it exactly. A finished job uses up its consumable
//! cards, makes its outputs next to the stack, and matches the stack again, so a repeatable recipe
//! keeps running for as long as its cards stay.

use crate::board::{Board, BoardEvent, Job};
use crate::def::CardDef;

/// Distance, in board units, from a finished job's stack to its first output. Each further output
/// lands one more step along the same row.
pub const OUTPUT_STEP: f32 = 1.5;

/// The longest step one [`Board::tick`] takes, in seconds (an hour). A longer `dt` counts as this:
/// a job finishes at most once per tick anyway, and it keeps the times finite (a huge `dt` would
/// otherwise push a job's time to infinity, which no save can hold).
pub const MAX_TICK: f32 = 3600.0;

impl Board {
    /// Matches every stack marked dirty, in stack-id order, until none is left.
    pub(crate) fn settle(&mut self) {
        while let Some(stack) = self.dirty.pop_first() {
            self.match_stack(stack);
        }
    }

    /// Re-matches one stack. It starts a job for the first recipe that matches, keeps the running
    /// job when that recipe still matches, and cancels the job (or replaces it) otherwise.
    pub(crate) fn match_stack(&mut self, stack: u64) {
        let Some(cards) = self.stacks.get(&stack).map(|s| s.cards.clone()) else {
            return;
        };
        let winner = self.first_match(&cards);
        let running = self.stacks.get(&stack).and_then(|s| s.job.clone());
        match (running, winner) {
            (None, None) => {}
            (Some(job), None) => {
                self.set_job(stack, None);
                self.pending.push(BoardEvent::JobCancelled {
                    stack,
                    recipe: job.recipe,
                });
            }
            (None, Some(recipe)) => self.start_job(stack, recipe, 0.0),
            (Some(job), Some(recipe)) if job.recipe == recipe => {}
            (Some(job), Some(recipe)) => {
                self.pending.push(BoardEvent::JobCancelled {
                    stack,
                    recipe: job.recipe,
                });
                self.start_job(stack, recipe, 0.0);
            }
        }
    }

    /// Advances every running job by `dt` seconds, finishes the jobs that are due, and returns the
    /// events queued since the last call.
    ///
    /// A job finishes at most once per tick. Time past its duration carries into its next run, so a
    /// long tick never loses production. A negative or non-finite `dt` counts as zero, and one over
    /// [`MAX_TICK`] as [`MAX_TICK`]. A job started
    /// during the tick begins at zero (or with the carried time, when it repeats a finished job) and
    /// advances from the next tick on.
    pub fn tick(&mut self, dt: f32) -> Vec<BoardEvent> {
        let dt = if dt.is_finite() && dt > 0.0 {
            dt.min(MAX_TICK)
        } else {
            0.0
        };
        let running: Vec<u64> = self.running.iter().copied().collect();
        let mut due = Vec::new();
        for stack in running {
            let Some(job) = self.stacks.get_mut(&stack).and_then(|s| s.job.as_mut()) else {
                continue;
            };
            job.elapsed += dt;
            let is_due = self
                .catalog
                .recipe(&job.recipe)
                .is_some_and(|recipe| job.elapsed >= recipe.seconds);
            if is_due {
                due.push(stack);
            }
        }
        for stack in due {
            self.finish_job(stack);
        }
        self.settle();
        self.take_events()
    }

    /// Progress, from 0 to 1, of the job on the stack that holds `card`. `None` when that stack runs
    /// no job. A job whose recipe takes no time reports 1.
    pub fn progress(&self, card: u64) -> Option<f32> {
        let job = self.running_job(card)?;
        let seconds = self.catalog.recipe(&job.recipe)?.seconds;
        if seconds <= 0.0 {
            return Some(1.0);
        }
        Some((job.elapsed / seconds).clamp(0.0, 1.0))
    }

    /// The recipe running on the stack that holds `card`, if any.
    pub fn running_recipe(&self, card: u64) -> Option<&str> {
        self.running_job(card).map(|job| job.recipe.as_str())
    }

    fn running_job(&self, card: u64) -> Option<&Job> {
        let stack = self.stack_id_of(card)?;
        self.stacks.get(&stack)?.job.as_ref()
    }

    /// The definitions of `cards`, bottom to top, or `None` if one of them is missing.
    fn defs_of(&self, cards: &[u64]) -> Option<Vec<&CardDef>> {
        cards.iter().map(|&card| self.def_of(card)).collect()
    }

    /// The first recipe, in precedence order, that the cards match exactly.
    fn first_match(&self, cards: &[u64]) -> Option<String> {
        let defs = self.defs_of(cards)?;
        self.catalog
            .recipes()
            .iter()
            .find(|recipe| recipe.matches(&defs))
            .map(|recipe| recipe.id.clone())
    }

    fn start_job(&mut self, stack: u64, recipe: String, elapsed: f32) {
        self.pending.push(BoardEvent::JobStarted {
            stack,
            recipe: recipe.clone(),
        });
        self.set_job(stack, Some(Job { recipe, elapsed }));
    }

    fn set_job(&mut self, stack: u64, job: Option<Job>) {
        if job.is_some() {
            self.running.insert(stack);
        } else {
            self.running.remove(&stack);
        }
        if let Some(s) = self.stacks.get_mut(&stack) {
            s.job = job;
        }
    }

    /// Finishes the job of `stack`: uses up its consumable cards, makes its outputs, and matches the
    /// stack again, so that a repeatable job keeps running with the time it had left over.
    fn finish_job(&mut self, stack: u64) {
        let Some(job) = self.stacks.get(&stack).and_then(|s| s.job.clone()) else {
            return;
        };
        self.set_job(stack, None);
        let Some(recipe) = self.catalog.recipe(&job.recipe).cloned() else {
            return;
        };
        let cards = self.stack_cards(stack).to_vec();
        let origin = self.stack_position(stack);
        let assignment = self.defs_of(&cards).and_then(|defs| recipe.assign(&defs));
        let (Some(origin), Some(assignment)) = (origin, assignment) else {
            self.pending.push(BoardEvent::JobCancelled {
                stack,
                recipe: recipe.id,
            });
            return;
        };
        self.pending.push(BoardEvent::JobFinished {
            stack,
            recipe: recipe.id.clone(),
        });

        // Cards whose ingredient says `consume` leave the table, in stack order.
        let used_up: Vec<bool> = assignment
            .iter()
            .map(|&index| recipe.ingredients.get(index).is_some_and(|i| i.consume))
            .collect();
        let consumed: Vec<u64> = cards
            .iter()
            .zip(used_up)
            .filter_map(|(&card, used)| used.then_some(card))
            .collect();
        if !consumed.is_empty() {
            for &card in &consumed {
                if let Some(instance) = self.cards.remove(&card) {
                    self.stack_of_card.remove(&card);
                    self.pending.push(BoardEvent::CardConsumed {
                        card,
                        def: instance.def,
                    });
                }
            }
            if let Some(s) = self.stacks.get_mut(&stack) {
                s.cards.retain(|c| !consumed.contains(c));
            }
            self.touch_stack(stack);
        }

        // Outputs appear in a row to the right of the stack, in the order the recipe lists them.
        // Only a chance below 1 draws from the generator, once per copy.
        let mut made = 0usize;
        for output in &recipe.outputs {
            for _ in 0..output.count {
                if output.chance < 1.0 && self.rng_mut().next_unit() >= output.chance {
                    continue;
                }
                let pos = (origin.0 + OUTPUT_STEP * (made as f32 + 1.0), origin.1);
                self.create_card(&output.card, pos);
                made += 1;
            }
        }

        // Run the same recipe again if the stack still matches it, keeping the time left over.
        let leftover = (job.elapsed - recipe.seconds).max(0.0);
        self.match_stack(stack);
        if let Some(next) = self
            .stacks
            .get_mut(&stack)
            .and_then(|s| s.job.as_mut())
            .filter(|next| next.recipe == recipe.id)
        {
            next.elapsed = leftover;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Xorshift64;
    use crate::test_support::catalog;

    fn board_with(seed: u64) -> Board {
        Board::new(catalog(), seed)
    }

    fn started(stack: u64, recipe: &str) -> BoardEvent {
        BoardEvent::JobStarted {
            stack,
            recipe: recipe.into(),
        }
    }

    #[test]
    fn a_worker_on_a_forest_keeps_producing_wood() {
        let mut b = board_with(1);
        let forest = b.spawn("forest", (0.0, 0.0)).expect("forest");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.drop_on(worker, forest).expect("drop");
        b.take_events();
        assert_eq!(b.running_recipe(forest), Some("gather_wood"));
        assert!(b.tick(1.0).is_empty());
        assert_eq!(b.progress(forest), Some(0.5));
        let events = b.tick(1.0);
        assert_eq!(
            events,
            vec![
                BoardEvent::JobFinished {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
                BoardEvent::CardSpawned {
                    card: 3,
                    def: "wood".into(),
                    x: 1.5,
                    y: 0.0
                },
                BoardEvent::StackChanged {
                    stack: 3,
                    cards: vec![3]
                },
                started(1, "gather_wood"),
            ]
        );
        assert_eq!(b.progress(forest), Some(0.0));
        assert!(
            b.card(forest).is_some() && b.card(worker).is_some(),
            "workers and zones stay"
        );
    }

    #[test]
    fn building_a_house_uses_up_its_cards() {
        let mut b = board_with(1);
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        let second = b.spawn("wood", (0.0, 0.0)).expect("wood");
        let stone = b.spawn("stone", (0.0, 0.0)).expect("stone");
        b.drop_on(second, wood).expect("drop wood");
        b.take_events();
        b.drop_on(stone, second).expect("drop stone");
        assert_eq!(b.running_recipe(wood), Some("build_house"));
        b.take_events();
        let events = b.tick(1.0);
        assert_eq!(
            events,
            vec![
                BoardEvent::JobFinished {
                    stack: 1,
                    recipe: "build_house".into()
                },
                BoardEvent::CardConsumed {
                    card: 1,
                    def: "wood".into()
                },
                BoardEvent::CardConsumed {
                    card: 2,
                    def: "wood".into()
                },
                BoardEvent::CardConsumed {
                    card: 3,
                    def: "stone".into()
                },
                BoardEvent::StackChanged {
                    stack: 1,
                    cards: vec![]
                },
                BoardEvent::CardSpawned {
                    card: 4,
                    def: "house".into(),
                    x: 1.5,
                    y: 0.0
                },
                BoardEvent::StackChanged {
                    stack: 4,
                    cards: vec![4]
                },
            ]
        );
        assert!(b.card(1).is_none() && b.card(3).is_none());
        assert_eq!(b.stacks().count(), 1, "only the house is left on the table");
    }

    #[test]
    fn an_extra_card_cancels_the_job_and_removing_it_restarts_the_job() {
        let mut b = board_with(1);
        let forest = b.spawn("forest", (0.0, 0.0)).expect("forest");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.drop_on(worker, forest).expect("drop");
        b.tick(1.0);
        b.take_events();
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        b.take_events();
        b.drop_on(wood, worker)
            .expect("a worker takes one more card");
        assert_eq!(
            b.take_events(),
            vec![
                BoardEvent::StackChanged {
                    stack: 3,
                    cards: vec![]
                },
                BoardEvent::StackChanged {
                    stack: 1,
                    cards: vec![forest, worker, wood]
                },
                BoardEvent::JobCancelled {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
            ]
        );
        assert_eq!(b.progress(forest), None);
        b.remove(wood).expect("remove");
        assert_eq!(
            b.take_events(),
            vec![
                BoardEvent::StackChanged {
                    stack: 1,
                    cards: vec![forest, worker]
                },
                started(1, "gather_wood"),
            ]
        );
        assert_eq!(
            b.progress(forest),
            Some(0.0),
            "the restarted job begins at zero"
        );
    }

    #[test]
    fn the_higher_priority_recipe_runs() {
        let mut b = board_with(1);
        let bush = b.spawn("berry_bush", (0.0, 0.0)).expect("bush");
        let worker = b.spawn("worker", (0.0, 0.0)).expect("worker");
        b.drop_on(worker, bush).expect("drop");
        assert_eq!(b.running_recipe(bush), Some("pick_berries_bonus"));
        b.take_events();
        let events = b.tick(2.0);
        assert_eq!(
            events,
            vec![
                BoardEvent::JobFinished {
                    stack: 1,
                    recipe: "pick_berries_bonus".into()
                },
                BoardEvent::CardSpawned {
                    card: 3,
                    def: "berries".into(),
                    x: 1.5,
                    y: 0.0
                },
                BoardEvent::StackChanged {
                    stack: 3,
                    cards: vec![3]
                },
                BoardEvent::CardSpawned {
                    card: 4,
                    def: "berries".into(),
                    x: 3.0,
                    y: 0.0
                },
                BoardEvent::StackChanged {
                    stack: 4,
                    cards: vec![4]
                },
                started(1, "pick_berries_bonus"),
            ]
        );
    }

    #[test]
    fn chance_outputs_follow_the_seeded_generator() {
        let seed = (1..)
            .find(|s| Xorshift64::new(*s).next_unit() < 0.5)
            .expect("seed");
        let unlucky = (1..)
            .find(|s| Xorshift64::new(*s).next_unit() >= 0.5)
            .expect("seed");
        for (seed, expect_coin) in [(seed, true), (unlucky, false)] {
            let mut b = board_with(seed);
            let market = b.spawn("market", (0.0, 0.0)).expect("market");
            let first = b.spawn("berries", (0.0, 0.0)).expect("berries");
            let second = b.spawn("berries", (0.0, 0.0)).expect("berries");
            b.drop_on(first, market).expect("drop");
            b.drop_on(second, market).expect("drop");
            b.take_events();
            let coins = b
                .tick(1.0)
                .iter()
                .filter(|e| matches!(e, BoardEvent::CardSpawned { def, .. } if def == "coin"))
                .count();
            assert_eq!(coins == 1, expect_coin, "seed {seed}");
        }
    }

    #[test]
    fn the_same_seed_and_actions_give_the_same_events() {
        let run = |seed| {
            let mut b = board_with(seed);
            for _ in 0..6 {
                let market = b.spawn("market", (0.0, 0.0)).expect("market");
                let first = b.spawn("berries", (0.0, 0.0)).expect("berries");
                let second = b.spawn("berries", (0.0, 0.0)).expect("berries");
                b.drop_on(first, market).expect("drop");
                b.drop_on(second, market).expect("drop");
            }
            b.tick(1.0)
        };
        assert_eq!(run(5), run(5));
    }

    #[test]
    fn a_huge_tick_keeps_the_job_time_finite() {
        let mut board = board_with(7);
        let worker = board.spawn("worker", (0.0, 0.0)).expect("worker");
        let forest = board.spawn("forest", (5.0, 0.0)).expect("forest");
        board.drop_on(worker, forest).expect("drop");
        board.tick(0.0);
        board.tick(f32::MAX);
        board.tick(f32::MAX);
        let saved = board.to_json().expect("a finite board saves");
        assert!(!saved.contains("null"), "{saved}");
    }

    #[test]
    fn progress_is_none_for_idle_cards() {
        let mut b = board_with(1);
        let wood = b.spawn("wood", (0.0, 0.0)).expect("wood");
        assert_eq!(b.progress(wood), None);
        assert_eq!(b.progress(99), None);
    }
}
