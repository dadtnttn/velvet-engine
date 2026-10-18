//! Runs the shared stacks fixtures (`docs/formats/fixtures/stacks`) through the `StacksBoard`
//! boundary, the way the JavaScript package `@velvet/cards` does.
//!
//! The same cases run in `crates/velvet-stacks/tests/fixtures.rs` and in the JavaScript tests. A
//! case's `error` names an engine refusal such as `DropRefused`; at the boundary that refusal is the
//! code `drop-refused`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use velvet_stacks::{BoardEvent, Card};
use velvet_wasm::stacks::{parse_catalog, StacksBoard};

/// The shared fixtures folder, next to the format specification.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/formats/fixtures/stacks")
}

/// One conformance case.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    format: String,
    version: u32,
    kind: String,
    name: String,
    seed: String,
    steps: Vec<Step>,
    #[serde(default)]
    expect: Option<Expect>,
}

/// One action or check. A step compares events only when it lists them.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Step {
    Spawn {
        def: String,
        at: [f32; 2],
        #[serde(default)]
        events: Option<Vec<BoardEvent>>,
    },
    Drop {
        card: u64,
        onto: u64,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        events: Option<Vec<BoardEvent>>,
    },
    Split {
        card: u64,
        #[serde(default)]
        events: Option<Vec<BoardEvent>>,
    },
    Move {
        card: u64,
        to: [f32; 2],
    },
    Remove {
        card: u64,
        #[serde(default)]
        events: Option<Vec<BoardEvent>>,
    },
    Tick {
        dt: f32,
        #[serde(default)]
        events: Option<Vec<BoardEvent>>,
    },
    Check {
        #[serde(default)]
        progress: BTreeMap<String, Option<f32>>,
        #[serde(default)]
        running: BTreeMap<String, Option<String>>,
        #[serde(default)]
        position: BTreeMap<String, [f32; 2]>,
    },
    Reload,
}

/// What the table must hold at the end of a case.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expect {
    /// Number of cards on the table, by definition.
    #[serde(default)]
    cards: BTreeMap<String, usize>,
    /// Every stack on the table, as card ids bottom to top (compared in any order).
    #[serde(default)]
    stacks: Option<Vec<Vec<u64>>>,
}

/// The part of a stack in the `stacks()` snapshot that the cases read.
#[derive(Deserialize)]
struct StackRow {
    cards: Vec<u64>,
}

/// Parses a reply of the boundary, which must be JSON.
fn reply(text: &str) -> Value {
    serde_json::from_str(text).expect("the boundary replies with JSON")
}

/// The failure code of a reply, or None when the reply is not a failure.
fn failure_code(outcome: &Value) -> Option<&str> {
    outcome["code"].as_str()
}

/// The boundary code of an engine error name: `DropRefused` is `drop-refused`.
fn code_of_name(name: &str) -> String {
    let mut code = String::new();
    for (index, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                code.push('-');
            }
            code.push(ch.to_ascii_lowercase());
        } else {
            code.push(ch);
        }
    }
    code
}

/// Takes the events that a board has queued, as engine events.
fn take_events(board: &mut StacksBoard) -> Vec<BoardEvent> {
    serde_json::from_str(&board.take_events()).expect("a list of events")
}

/// Compares the events that a step lists with the events it produced.
fn check_events(at: &str, got: Vec<BoardEvent>, want: &Option<Vec<BoardEvent>>) {
    if let Some(want) = want {
        assert_eq!(&got, want, "{at}: events");
    }
}

/// The card id that a key of a case names.
fn card_key(text: &str) -> u64 {
    text.parse()
        .unwrap_or_else(|_| panic!("`{text}` is not a card id"))
}

/// The cards on the table, from the `cards()` snapshot.
fn table_cards(board: &StacksBoard) -> Vec<Card> {
    serde_json::from_str(&board.cards()).expect("a list of cards")
}

/// Plays one case on a board built from `catalog_json`, and checks every step and the end state.
fn run_case(path: &Path, catalog_json: &str) {
    let file = path.display().to_string();
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{file}: {e}"));
    let case: Case = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let name = format!("{file} ({})", case.name);
    assert_eq!(case.format, "velvet-stacks", "{name}");
    assert_eq!(case.version, 1, "{name}");
    assert_eq!(case.kind, "case", "{name}");

    let mut board =
        StacksBoard::new(catalog_json, &case.seed).unwrap_or_else(|e| panic!("{name}: {e}"));
    for (index, step) in case.steps.iter().enumerate() {
        let at = format!("{name}, step {index}");
        match step {
            Step::Spawn {
                def,
                at: pos,
                events,
            } => {
                let made = reply(&board.spawn(def, pos[0], pos[1]));
                assert!(made.get("card").is_some(), "{at}: the spawn failed: {made}");
                check_events(&at, take_events(&mut board), events);
            }
            Step::Drop {
                card,
                onto,
                error,
                events,
            } => {
                let outcome = reply(&board.drop_on(*card as f64, *onto as f64));
                match error {
                    None => assert_eq!(outcome, Value::Null, "{at}: the drop failed: {outcome}"),
                    Some(expected) => {
                        let want = code_of_name(expected);
                        assert_eq!(
                            failure_code(&outcome),
                            Some(want.as_str()),
                            "{at}: expected {expected}, got {outcome}"
                        );
                    }
                }
                check_events(&at, take_events(&mut board), events);
            }
            Step::Split { card, events } => {
                let outcome = reply(&board.split(*card as f64));
                assert!(
                    outcome.get("stack").is_some(),
                    "{at}: the split failed: {outcome}"
                );
                check_events(&at, take_events(&mut board), events);
            }
            Step::Move { card, to } => {
                let outcome = reply(&board.move_stack(*card as f64, to[0], to[1]));
                assert_eq!(outcome, Value::Null, "{at}: the move failed: {outcome}");
                assert!(
                    take_events(&mut board).is_empty(),
                    "{at}: moving queues no events"
                );
            }
            Step::Remove { card, events } => {
                let outcome = reply(&board.remove(*card as f64));
                assert_eq!(outcome, Value::Null, "{at}: the remove failed: {outcome}");
                check_events(&at, take_events(&mut board), events);
            }
            Step::Tick { dt, events } => {
                let got: Vec<BoardEvent> =
                    serde_json::from_str(&board.tick(*dt)).expect("a list of events");
                check_events(&at, got, events);
            }
            Step::Check {
                progress,
                running,
                position,
            } => {
                for (card, want) in progress {
                    let got = reply(&board.progress(card_key(card) as f64));
                    assert_eq!(
                        got.as_f64(),
                        (*want).map(f64::from),
                        "{at}: progress of card {card}"
                    );
                }
                for (card, want) in running {
                    let got = reply(&board.running_recipe(card_key(card) as f64));
                    assert_eq!(
                        got.as_str(),
                        want.as_deref(),
                        "{at}: running recipe of card {card}"
                    );
                }
                for (card, want) in position {
                    let id = card_key(card);
                    let found = table_cards(&board)
                        .into_iter()
                        .find(|c| c.id == id)
                        .map(|c| [c.x, c.y]);
                    assert_eq!(found, Some(*want), "{at}: position of card {card}");
                }
            }
            Step::Reload => {
                let save = board.save();
                board =
                    StacksBoard::load(catalog_json, &save).unwrap_or_else(|e| panic!("{at}: {e}"));
            }
        }
    }

    if let Some(expect) = &case.expect {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for card in table_cards(&board) {
            *counts.entry(card.def).or_insert(0) += 1;
        }
        assert_eq!(counts, expect.cards, "{name}: cards on the table");
        if let Some(stacks) = &expect.stacks {
            let rows: Vec<StackRow> =
                serde_json::from_str(&board.stacks()).expect("a list of stacks");
            let mut got: Vec<Vec<u64>> = rows.into_iter().map(|row| row.cards).collect();
            got.sort();
            let mut want = stacks.clone();
            want.sort();
            assert_eq!(got, want, "{name}: stacks on the table");
        }
    }
}

#[test]
fn engine_names_map_to_boundary_codes() {
    assert_eq!(code_of_name("DropRefused"), "drop-refused");
    assert_eq!(code_of_name("StackFull"), "stack-full");
    assert_eq!(code_of_name("SameStack"), "same-stack");
    assert_eq!(code_of_name("NoSuchCard"), "no-such-card");
}

#[test]
fn the_shared_catalog_has_a_normal_form_that_reads_back() {
    let text = fs::read_to_string(fixtures_dir().join("catalog.json")).expect("catalog.json");
    let normal = parse_catalog(&text);
    let document = reply(&normal);
    assert!(document.get("error").is_none(), "{normal}");
    let recipes = document["recipes"].as_array().map_or(0, Vec::len);
    assert!(recipes >= 12, "the fixtures need at least twelve recipes");
    assert_eq!(
        parse_catalog(&normal),
        normal,
        "the normal form reads back unchanged"
    );
}

#[test]
fn every_case_plays_as_written() {
    let text = fs::read_to_string(fixtures_dir().join("catalog.json")).expect("catalog.json");
    let mut paths: Vec<PathBuf> = fs::read_dir(fixtures_dir().join("cases"))
        .expect("the cases folder")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 8, "found only {} cases", paths.len());
    for path in &paths {
        run_case(path, &text);
    }
}
