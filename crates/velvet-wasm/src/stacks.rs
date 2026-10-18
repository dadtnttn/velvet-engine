//! Cards on a board for JavaScript: velvet-stacks' catalog and board, played through JSON.
//!
//! The exports, as JavaScript sees them:
//!
//! - `parseCatalog(json)` checks a catalog document and returns it in its normal form: every field
//!   written out, and the recipes in precedence order.
//! - `new StacksBoard(catalogJson, seed)` makes an empty board that plays with the catalog. Its
//!   methods are `spawn(def, x, y)`, `moveStack(card, x, y)`, `dropOn(card, onto)`, `split(card)`,
//!   `remove(card)`, `tick(dt)`, `takeEvents()`, `progress(card)`, `runningRecipe(card)`, `cards()`,
//!   `stacks()`, `save()`, `setOutputPlacement(placementJson)` and `outputPlacement()`.
//! - `StacksBoard.load(catalogJson, saveJson)` restores a board from a save.
//!
//! The output placement is the document of `docs/formats/STACKS.md`: `{"kind": "row"}` (the default),
//! or `{"kind": "free", "footprint": [w, h], "bounds": [x, y, width, height]}` with `bounds` left out
//! for the whole table. It is not in a save, so a game sets it again after `load`.
//!
//! The JSON shapes are the ones in `docs/formats/STACKS.md`: the catalog, the board save, and the
//! events (tagged by `event`). The two snapshots are `cards()`, a list of `{id, def, x, y}`, and
//! `stacks()`, a list of `{id, cards, x, y, job}`. In a stack, `cards` lists the card ids bottom to
//! top, `x` and `y` are the position of the bottom card, and `job` is `{recipe, progress}` or null.
//! Both lists are sorted by id.
//!
//! ## Numbers
//!
//! - Seeds cross as **decimal text**, so a seed keeps all 64 bits. Text that is not digits, or is
//!   above 2^64 - 1, fails with `bad-input`.
//! - Card and stack ids cross as **JavaScript numbers**. They count from 1, so a number holds one
//!   exactly up to 2^53 - 1. A card id that is not a whole number from 1 up to that limit fails
//!   with `bad-id`, and so does a save whose ids or counters go past it.
//! - Positions and times are binary32 numbers, as the format says. A position that is not a finite
//!   number fails with `bad-input`.
//!
//! ## Failures
//!
//! A failure is `{"error", "code"}`, as for the guion. The constructor and `load` throw that text.
//! The codes are the engine's refusals in kebab-case: `bad-input`, `bad-id`, `unsupported-document`,
//! `empty-card-id`, `duplicate-card`, `zero-max`, `unknown-card`, `duplicate-recipe`,
//! `unknown-recipe`, `no-ingredients`, `zero-count`, `bad-seconds`, `bad-chance`, `no-such-card`,
//! `same-stack`, `drop-refused`, `stack-full`, `bad-placement`, `invalid-save` and `internal`.

use serde::Serialize;
use velvet_stacks::{Board, BoardSave, Catalog, OutputPlacement, StacksError};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{json, respond, Failure};

/// The largest id a JavaScript number holds exactly: 2^53 - 1.
const MAX_JS_ID: u64 = (1 << 53) - 1;

impl From<StacksError> for Failure {
    fn from(error: StacksError) -> Self {
        Failure::new(code_of(&error), error.to_string())
    }
}

/// The boundary code of an engine refusal.
fn code_of(error: &StacksError) -> &'static str {
    match error {
        StacksError::Parse(_) => "bad-input",
        StacksError::UnsupportedDocument { .. } => "unsupported-document",
        StacksError::EmptyCardId => "empty-card-id",
        StacksError::DuplicateCard(_) => "duplicate-card",
        StacksError::ZeroMax { .. } => "zero-max",
        StacksError::UnknownCard { .. } => "unknown-card",
        StacksError::DuplicateRecipe(_) => "duplicate-recipe",
        StacksError::UnknownRecipe(_) => "unknown-recipe",
        StacksError::NoIngredients { .. } => "no-ingredients",
        StacksError::ZeroCount { .. } => "zero-count",
        StacksError::BadSeconds { .. } => "bad-seconds",
        StacksError::BadChance { .. } => "bad-chance",
        StacksError::NoSuchCard(_) => "no-such-card",
        StacksError::SameStack(_) => "same-stack",
        StacksError::DropRefused { .. } => "drop-refused",
        StacksError::StackFull { .. } => "stack-full",
        StacksError::BadPlacement(_) => "bad-placement",
        StacksError::InvalidSave(_) => "invalid-save",
    }
}

/// The output placement that a placement document names. Text that is not a placement fails with
/// `bad-input`; a placement the rules refuse fails later, with `bad-placement`.
fn parse_placement(text: &str) -> Result<OutputPlacement, Failure> {
    serde_json::from_str(text).map_err(|error| {
        Failure::new(
            "bad-input",
            format!("the output placement is not valid JSON of the expected shape: {error}"),
        )
    })
}

/// The id a JavaScript number names: a whole number from 1 up to 2^53 - 1. `what` names the
/// argument in the message.
fn js_id(value: f64, what: &str) -> Result<u64, Failure> {
    if !value.is_finite() || value.fract() != 0.0 || value < 1.0 {
        return Err(Failure::new(
            "bad-id",
            format!("{what} must be a whole number from 1 up, not {value}"),
        ));
    }
    if value > MAX_JS_ID as f64 {
        return Err(Failure::new(
            "bad-id",
            format!("{what} is {value}, above 2^53 - 1: JavaScript numbers cannot hold it exactly"),
        ));
    }
    Ok(value as u64)
}

/// The position a JavaScript caller passed: two finite numbers.
fn position(x: f32, y: f32) -> Result<(f32, f32), Failure> {
    if x.is_finite() && y.is_finite() {
        Ok((x, y))
    } else {
        Err(Failure::new(
            "bad-input",
            format!("a position must be two finite numbers, not ({x}, {y})"),
        ))
    }
}

/// The seed that decimal text names: digits only, and no more than 2^64 - 1.
fn parse_seed(text: &str) -> Result<u64, Failure> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Failure::new(
            "bad-input",
            format!("the seed {text:?} is not decimal digits: a seed is a whole number from 0 to 2^64 - 1"),
        ));
    }
    text.parse::<u64>()
        .map_err(|_| Failure::new("bad-input", format!("the seed {text} is above 2^64 - 1")))
}

/// Reads a catalog document, checked by the engine.
fn read_catalog(text: &str) -> Result<Catalog, Failure> {
    Catalog::from_json(text).map_err(Failure::from)
}

/// Refuses a save with an id or a counter that a JavaScript number cannot hold exactly.
fn check_ids(save: &BoardSave) -> Result<(), Failure> {
    let ids = save
        .cards
        .iter()
        .map(|card| card.id)
        .chain(save.stacks.iter().map(|stack| stack.id))
        .chain([save.next_card, save.next_stack]);
    match ids.max() {
        Some(largest) if largest > MAX_JS_ID => Err(Failure::new(
            "bad-id",
            format!("the save holds the id {largest}, above 2^53 - 1: JavaScript numbers cannot hold it exactly"),
        )),
        _ => Ok(()),
    }
}

/// Refuses a save that JSON could not write back exactly: a position or a job time that is not a
/// finite number, which JSON writes as null.
fn check_finite(save: &BoardSave) -> Result<(), Failure> {
    let positions = save
        .cards
        .iter()
        .all(|card| card.x.is_finite() && card.y.is_finite());
    let times = save
        .stacks
        .iter()
        .all(|stack| stack.job.as_ref().is_none_or(|job| job.elapsed.is_finite()));
    if positions && times {
        Ok(())
    } else {
        Err(Failure::new(
            "invalid-save",
            "a position or a job time is not a finite number (it overflowed binary32), so the board cannot be saved",
        ))
    }
}

/// A board of cards that plays with a catalog. JavaScript holds it as `StacksBoard`; the module
/// docs list its methods.
#[wasm_bindgen]
pub struct StacksBoard {
    board: Board,
}

/// One stack, as `stacks()` lists it.
#[derive(Serialize)]
struct StackView {
    id: u64,
    cards: Vec<u64>,
    x: f32,
    y: f32,
    job: Option<JobView>,
}

/// The job a stack runs, as `stacks()` shows it.
#[derive(Serialize)]
struct JobView {
    recipe: String,
    progress: f32,
}

impl StacksBoard {
    /// The empty board for a catalog and a seed, both checked.
    fn create(catalog_json: &str, seed: &str) -> Result<Self, Failure> {
        let catalog = read_catalog(catalog_json)?;
        let seed = parse_seed(seed)?;
        Ok(Self {
            board: Board::new(catalog, seed),
        })
    }

    /// The board that a save describes, played with a catalog. Every check of the engine applies,
    /// and the ids must fit a JavaScript number.
    fn restore(catalog_json: &str, save_json: &str) -> Result<Self, Failure> {
        let catalog = read_catalog(catalog_json)?;
        let save: BoardSave = serde_json::from_str(save_json).map_err(|error| {
            Failure::new(
                "bad-input",
                format!("the save is not valid JSON of the expected shape: {error}"),
            )
        })?;
        check_ids(&save)?;
        Ok(Self {
            board: Board::load(catalog, save)?,
        })
    }

    /// The view of the stack `id` whose cards are `cards` (bottom to top), or None when it has no
    /// card to take a position from.
    fn stack_view(&self, id: u64, cards: &[u64]) -> Option<StackView> {
        let bottom = *cards.first()?;
        let at = self.board.card(bottom)?;
        let job = match (
            self.board.running_recipe(bottom),
            self.board.progress(bottom),
        ) {
            (Some(recipe), Some(progress)) => Some(JobView {
                recipe: recipe.to_owned(),
                progress,
            }),
            _ => None,
        };
        Some(StackView {
            id,
            cards: cards.to_vec(),
            x: at.x,
            y: at.y,
            job,
        })
    }

    /// The board's save, checked so that it can be written and read back exactly.
    fn checked_save(&self) -> Result<BoardSave, Failure> {
        let save = self.board.save();
        check_ids(&save)?;
        check_finite(&save)?;
        Ok(save)
    }
}

#[wasm_bindgen]
impl StacksBoard {
    /// An empty board that plays with `catalogJson`. `seed` is decimal text, so every 64-bit seed
    /// works. A bad catalog or seed fails with the error JSON, which is thrown.
    #[wasm_bindgen(constructor)]
    pub fn new(catalog_json: &str, seed: &str) -> Result<StacksBoard, String> {
        StacksBoard::create(catalog_json, seed).map_err(|failure| failure.to_json())
    }

    /// The board that `saveJson` describes, played with `catalogJson`. A save that does not fit the
    /// catalog, or whose ids a JavaScript number cannot hold, fails with the error JSON, which is
    /// thrown.
    pub fn load(catalog_json: &str, save_json: &str) -> Result<StacksBoard, String> {
        StacksBoard::restore(catalog_json, save_json).map_err(|failure| failure.to_json())
    }

    /// Creates a card of definition `def` at (`x`, `y`), in a stack of its own. Returns
    /// `{"card": id}`. An unknown definition fails with `unknown-card`.
    pub fn spawn(&mut self, def: &str, x: f32, y: f32) -> String {
        let result = position(x, y)
            .and_then(|at| self.board.spawn(def, at).map_err(Failure::from))
            .map(|card| serde_json::json!({ "card": card }));
        respond(result)
    }

    /// Moves the stack that holds `card` so that its bottom card sits at (`x`, `y`). Queues no
    /// events and never changes a job. Returns null.
    #[wasm_bindgen(js_name = "moveStack")]
    pub fn move_stack(&mut self, card: f64, x: f32, y: f32) -> String {
        let result = js_id(card, "`card`").and_then(|card| {
            let at = position(x, y)?;
            self.board.move_stack(card, at).map_err(Failure::from)
        });
        respond(result)
    }

    /// Drops the stack from `card` upward onto the stack that holds `onto`. A refused drop
    /// (`drop-refused`, `stack-full`, `same-stack`) changes nothing. Returns null.
    #[wasm_bindgen(js_name = "dropOn")]
    pub fn drop_on(&mut self, card: f64, onto: f64) -> String {
        let result = js_id(card, "`card`").and_then(|card| {
            let onto = js_id(onto, "`onto`")?;
            self.board.drop_on(card, onto).map_err(Failure::from)
        });
        respond(result)
    }

    /// Takes `card` and the cards above it into a new stack. Returns `{"stack": id}`: the new stack,
    /// or the stack of `card` itself when `card` is the bottom card.
    pub fn split(&mut self, card: f64) -> String {
        let result = js_id(card, "`card`")
            .and_then(|card| self.board.split(card).map_err(Failure::from))
            .map(|stack| serde_json::json!({ "stack": stack }));
        respond(result)
    }

    /// Takes `card` off the table. The cards above it stay in their stack. Returns null.
    pub fn remove(&mut self, card: f64) -> String {
        respond(
            js_id(card, "`card`").and_then(|card| self.board.remove(card).map_err(Failure::from)),
        )
    }

    /// Advances time by `dt` seconds. Returns the events queued since the last call, the events of
    /// earlier actions included, as a JSON array. A negative or non-finite `dt` counts as 0.
    pub fn tick(&mut self, dt: f32) -> String {
        json(&self.board.tick(dt))
    }

    /// Returns and clears the events that actions queued since the last call, as a JSON array. Read
    /// them before `save()`: queued events are not saved.
    #[wasm_bindgen(js_name = "takeEvents")]
    pub fn take_events(&mut self) -> String {
        json(&self.board.take_events())
    }

    /// The progress, from 0 to 1, of the job on the stack that holds `card`. Null when that stack
    /// runs no job.
    pub fn progress(&self, card: f64) -> String {
        respond(js_id(card, "`card`").map(|card| self.board.progress(card)))
    }

    /// The recipe that runs on the stack that holds `card`, as a JSON string, or null.
    #[wasm_bindgen(js_name = "runningRecipe")]
    pub fn running_recipe(&self, card: f64) -> String {
        respond(
            js_id(card, "`card`").map(|card| self.board.running_recipe(card).map(str::to_owned)),
        )
    }

    /// Every card on the table, sorted by id: `[{id, def, x, y}]`.
    pub fn cards(&self) -> String {
        json(&self.board.cards().collect::<Vec<_>>())
    }

    /// Every stack on the table, sorted by id: `[{id, cards, x, y, job}]` (see the module docs).
    pub fn stacks(&self) -> String {
        let views: Vec<StackView> = self
            .board
            .stacks()
            .filter_map(|(id, cards)| self.stack_view(id, cards))
            .collect();
        json(&views)
    }

    /// The board's state as a save document (`docs/formats/STACKS.md`). Queued events are not in
    /// it. A board that cannot be written exactly fails with `bad-id` or `invalid-save`.
    pub fn save(&self) -> String {
        respond(self.checked_save())
    }

    /// Sets where the outputs of finished jobs land, from the placement document `placementJson`
    /// (see the module docs). Text that is not a placement fails with `bad-input`, and a placement the
    /// rules refuse (a footprint side not above zero, a negative bounds side) fails with
    /// `bad-placement`; a refused placement leaves the old one in place. Returns null.
    #[wasm_bindgen(js_name = "setOutputPlacement")]
    pub fn set_output_placement(&mut self, placement_json: &str) -> String {
        let result = parse_placement(placement_json).and_then(|placement| {
            self.board
                .set_output_placement(placement)
                .map_err(Failure::from)
        });
        respond(result)
    }

    /// The output placement in use, as the placement document that `setOutputPlacement` takes.
    #[wasm_bindgen(js_name = "outputPlacement")]
    pub fn output_placement(&self) -> String {
        json(&self.board.output_placement())
    }
}

/// Checks a catalog document and returns it in its normal form: every field written out, and the
/// recipes in precedence order. A bad catalog fails with the error JSON.
#[wasm_bindgen(js_name = "parseCatalog")]
pub fn parse_catalog(catalog_json: &str) -> String {
    respond(read_catalog(catalog_json))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use velvet_stacks::BoardEvent;

    use super::*;

    /// The shared catalog, which the conformance fixtures read too.
    const CATALOG: &str = include_str!("../../../docs/formats/fixtures/stacks/catalog.json");

    /// Parses a reply of the boundary, which must be JSON.
    fn reply(text: &str) -> Value {
        serde_json::from_str(text).expect("the boundary replies with JSON")
    }

    /// The events that a reply lists, read back as engine events.
    fn events(text: &str) -> Vec<BoardEvent> {
        serde_json::from_str(text).expect("a list of events")
    }

    /// The failure code in a reply, or None when the reply is not a failure.
    fn code_of_reply(text: &str) -> Option<String> {
        reply(text)["code"].as_str().map(str::to_owned)
    }

    #[test]
    fn a_board_plays_a_job_through_json() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        assert_eq!(
            reply(&board.spawn("forest", 0.0, 0.0)),
            json!({ "card": 1 })
        );
        assert_eq!(
            reply(&board.spawn("worker", 0.0, 0.0)),
            json!({ "card": 2 })
        );
        board.take_events();
        assert_eq!(reply(&board.drop_on(2.0, 1.0)), Value::Null);
        assert_eq!(
            events(&board.take_events()),
            vec![
                BoardEvent::StackChanged {
                    stack: 2,
                    cards: vec![]
                },
                BoardEvent::StackChanged {
                    stack: 1,
                    cards: vec![1, 2]
                },
                BoardEvent::JobStarted {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
            ]
        );
        assert_eq!(reply(&board.running_recipe(1.0)), json!("gather_wood"));
        assert!(events(&board.tick(1.0)).is_empty());
        assert_eq!(reply(&board.progress(1.0)), json!(0.5));
        assert_eq!(
            events(&board.tick(1.0)),
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
                BoardEvent::JobStarted {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
            ]
        );
    }

    #[test]
    fn a_save_taken_mid_job_continues_with_the_same_events() {
        let mut board = StacksBoard::new(CATALOG, "9").expect("the shared catalog");
        board.spawn("forest", 0.0, 0.0);
        board.spawn("worker", 0.0, 0.0);
        board.drop_on(2.0, 1.0);
        board.tick(1.0);
        let save = board.save();
        let mut restored = StacksBoard::load(CATALOG, &save).expect("the save loads");
        assert_eq!(
            restored.save(),
            save,
            "a loaded board saves what it was given"
        );
        assert_eq!(board.tick(1.5), restored.tick(1.5));
        assert_eq!(board.progress(1.0), restored.progress(1.0));
    }

    #[test]
    fn ids_that_javascript_numbers_cannot_hold_are_refused() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        let too_big = 9_007_199_254_740_992.0;
        for text in [
            board.progress(too_big),
            board.progress(0.0),
            board.progress(1.5),
            board.progress(f64::NAN),
        ] {
            assert_eq!(code_of_reply(&text).as_deref(), Some("bad-id"), "{text}");
        }
        assert_eq!(reply(&board.progress(9_007_199_254_740_991.0)), Value::Null);
        assert_eq!(
            code_of_reply(&board.drop_on(1.0, too_big)).as_deref(),
            Some("bad-id")
        );
        assert_eq!(
            code_of_reply(&board.remove(-1.0)).as_deref(),
            Some("bad-id")
        );
    }

    #[test]
    fn seeds_are_decimal_digits_of_all_64_bits() {
        assert!(StacksBoard::new(CATALOG, "18446744073709551615").is_ok());
        for seed in ["", "-1", "1.5", "1e3", "+7", "18446744073709551616"] {
            let refused = StacksBoard::new(CATALOG, seed)
                .err()
                .expect("a seed that is not a 64-bit decimal is refused");
            assert_eq!(
                code_of_reply(&refused).as_deref(),
                Some("bad-input"),
                "seed {seed:?}"
            );
        }
    }

    #[test]
    fn refusals_carry_the_engine_code_and_queue_nothing() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        assert_eq!(
            code_of_reply(&board.spawn("ghost", 0.0, 0.0)).as_deref(),
            Some("unknown-card")
        );
        board.spawn("forest", 0.0, 0.0);
        board.spawn("wood", 0.0, 0.0);
        board.spawn("house", 0.0, 0.0);
        board.spawn("wood", 0.0, 0.0);
        board.take_events();
        assert_eq!(
            code_of_reply(&board.drop_on(2.0, 1.0)).as_deref(),
            Some("drop-refused")
        );
        assert_eq!(
            code_of_reply(&board.drop_on(4.0, 3.0)).as_deref(),
            Some("stack-full")
        );
        assert_eq!(
            code_of_reply(&board.drop_on(2.0, 2.0)).as_deref(),
            Some("same-stack")
        );
        assert_eq!(
            code_of_reply(&board.drop_on(2.0, 99.0)).as_deref(),
            Some("no-such-card")
        );
        assert_eq!(
            code_of_reply(&board.split(99.0)).as_deref(),
            Some("no-such-card")
        );
        assert!(
            events(&board.take_events()).is_empty(),
            "refused actions queue nothing"
        );
    }

    #[test]
    fn catalog_and_save_mistakes_carry_codes() {
        let board_doc = r#"{"format":"velvet-stacks","version":1,"kind":"board","cards":[]}"#;
        let refused = StacksBoard::new(board_doc, "1")
            .err()
            .expect("a board is not a catalog");
        assert_eq!(
            code_of_reply(&refused).as_deref(),
            Some("unsupported-document")
        );
        assert_eq!(
            code_of_reply(&parse_catalog("[1]")).as_deref(),
            Some("bad-input")
        );
        let twice = r#"{"format":"velvet-stacks","version":1,"kind":"catalog",
            "cards":[{"id":"a","name":"a","kind":"x"},{"id":"a","name":"a","kind":"x"}]}"#;
        assert_eq!(
            code_of_reply(&parse_catalog(twice)).as_deref(),
            Some("duplicate-card")
        );
        let bad_load = StacksBoard::load(CATALOG, "{}")
            .err()
            .expect("an empty object is not a save");
        assert_eq!(code_of_reply(&bad_load).as_deref(), Some("bad-input"));
    }

    #[test]
    fn a_save_with_ids_javascript_numbers_cannot_hold_is_refused() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        board.spawn("wood", 0.0, 0.0);
        let mut save = reply(&board.save());
        save["next_card"] = json!(9_007_199_254_740_992_u64);
        let refused = StacksBoard::load(CATALOG, &save.to_string())
            .err()
            .expect("the id is too large");
        assert_eq!(code_of_reply(&refused).as_deref(), Some("bad-id"));
    }

    #[test]
    fn the_normal_form_of_a_catalog_is_in_precedence_order() {
        let normal = reply(&parse_catalog(CATALOG));
        assert_eq!(normal["kind"], "catalog");
        assert_eq!(
            normal["recipes"][0]["id"], "gather_berries_bonus",
            "priority first"
        );
        assert_eq!(
            normal["recipes"][1]["id"], "build_market",
            "then the most cards"
        );
        assert_eq!(normal["cards"][8]["id"], "wood");
        assert_eq!(
            normal["cards"][8]["stack"],
            json!({ "accepts": [], "max": 30 })
        );
        let again = reply(&parse_catalog(&parse_catalog(CATALOG)));
        assert_eq!(again, normal, "the normal form is its own normal form");
    }

    #[test]
    fn the_same_seed_and_actions_give_the_same_events() {
        let run = |seed: &str| {
            let mut board = StacksBoard::new(CATALOG, seed).expect("the shared catalog");
            board.spawn("market", 0.0, 0.0);
            board.spawn("berries", 0.0, 0.0);
            board.spawn("berries", 0.0, 0.0);
            board.drop_on(2.0, 1.0);
            board.drop_on(3.0, 1.0);
            board.tick(1.0)
        };
        assert_eq!(run("5"), run("5"));
    }

    #[test]
    fn cards_and_stacks_list_the_table_in_id_order() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        board.spawn("wood", 1.0, 2.0);
        board.spawn("stone", 3.0, 4.0);
        board.drop_on(2.0, 1.0);
        assert_eq!(
            reply(&board.cards()),
            json!([
                { "id": 1, "def": "wood", "x": 1.0, "y": 2.0 },
                { "id": 2, "def": "stone", "x": 1.0, "y": 2.0 }
            ])
        );
        assert_eq!(
            reply(&board.stacks()),
            json!([{ "id": 1, "cards": [1, 2], "x": 1.0, "y": 2.0, "job": null }])
        );
    }

    #[test]
    fn the_output_placement_crosses_as_a_document() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        assert_eq!(reply(&board.output_placement()), json!({ "kind": "row" }));
        assert_eq!(
            reply(
                &board.set_output_placement(
                    r#"{"kind":"free","footprint":[1,1],"bounds":[0,0,4,4]}"#
                )
            ),
            Value::Null
        );
        assert_eq!(
            reply(&board.output_placement()),
            json!({ "kind": "free", "footprint": [1.0, 1.0], "bounds": [0.0, 0.0, 4.0, 4.0] })
        );
        board.set_output_placement(r#"{"kind":"free","footprint":[2.5,1.0]}"#);
        assert_eq!(
            reply(&board.output_placement()),
            json!({ "kind": "free", "footprint": [2.5, 1.0] }),
            "without bounds the document leaves them out"
        );
        board.set_output_placement(r#"{"kind":"row"}"#);
        assert_eq!(reply(&board.output_placement()), json!({ "kind": "row" }));
    }

    #[test]
    fn a_placement_that_fails_keeps_the_old_one_and_names_its_code() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        board.set_output_placement(r#"{"kind":"free","footprint":[1,1]}"#);
        for (text, code) in [
            (r#"{"kind":"free","footprint":[0,1]}"#, "bad-placement"),
            (
                r#"{"kind":"free","footprint":[1,1],"bounds":[0,0,-1,2]}"#,
                "bad-placement",
            ),
            (r#"{"kind":"grid"}"#, "bad-input"),
            (r#"{"kind":"free","footprint":[1,null]}"#, "bad-input"),
            ("not json", "bad-input"),
        ] {
            assert_eq!(
                code_of_reply(&board.set_output_placement(text)).as_deref(),
                Some(code),
                "{text}"
            );
        }
        assert_eq!(
            reply(&board.output_placement()),
            json!({ "kind": "free", "footprint": [1.0, 1.0] })
        );
    }

    #[test]
    fn free_placement_puts_a_job_output_beside_its_stack() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        board.set_output_placement(r#"{"kind":"free","footprint":[1,1]}"#);
        board.spawn("forest", 0.0, 0.0);
        board.spawn("worker", 0.0, 0.0);
        board.drop_on(2.0, 1.0);
        board.take_events();
        assert_eq!(
            events(&board.tick(2.0)),
            vec![
                BoardEvent::JobFinished {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
                BoardEvent::CardSpawned {
                    card: 3,
                    def: "wood".into(),
                    x: 1.0,
                    y: 0.0
                },
                BoardEvent::StackChanged {
                    stack: 3,
                    cards: vec![3]
                },
                BoardEvent::JobStarted {
                    stack: 1,
                    recipe: "gather_wood".into()
                },
            ]
        );
    }

    #[test]
    fn a_save_does_not_hold_the_placement() {
        let mut board = StacksBoard::new(CATALOG, "1").expect("the shared catalog");
        board.set_output_placement(r#"{"kind":"free","footprint":[1,1]}"#);
        let save = board.save();
        assert!(!save.contains("footprint"), "{save}");
        let restored = StacksBoard::load(CATALOG, &save).expect("the save loads");
        assert_eq!(
            reply(&restored.output_placement()),
            json!({ "kind": "row" })
        );
    }
}
