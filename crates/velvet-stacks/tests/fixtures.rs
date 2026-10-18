//! Conformance tests: every case under `docs/formats/fixtures/stacks/cases` must play out as written.
//!
//! The same files are the contract the JavaScript runtime has to pass too. A case is a list of
//! steps on a board built from `catalog.json`; a step may list the events it must produce, and the
//! case ends with the cards and stacks the table must hold. See `docs/formats/STACKS.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use pretty_assertions::assert_eq;
use serde::Deserialize;
use velvet_stacks::{Board, BoardEvent, Catalog};

/// The shared fixtures folder, next to the format specification.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("formats")
        .join("fixtures")
        .join("stacks")
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

fn check_events(at: &str, got: Vec<BoardEvent>, want: &Option<Vec<BoardEvent>>) {
    if let Some(want) = want {
        assert_eq!(got, *want, "{at}: events");
    }
}

fn card_id(text: &str) -> u64 {
    text.parse()
        .unwrap_or_else(|_| panic!("`{text}` is not a card id"))
}

fn run_case(path: &Path, catalog: &Catalog) {
    let file = path.display().to_string();
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{file}: {e}"));
    let case: Case = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let name = format!("{file} ({})", case.name);
    assert_eq!(case.format, "velvet-stacks", "{name}");
    assert_eq!(case.version, 1, "{name}");
    assert_eq!(case.kind, "case", "{name}");
    let seed: u64 = case
        .seed
        .parse()
        .unwrap_or_else(|_| panic!("{name}: seed `{}` is not a number", case.seed));

    let mut board = Board::new(catalog.clone(), seed);
    for (index, step) in case.steps.iter().enumerate() {
        let at = format!("{name}, step {index}");
        match step {
            Step::Spawn {
                def,
                at: pos,
                events,
            } => {
                board
                    .spawn(def, (pos[0], pos[1]))
                    .unwrap_or_else(|e| panic!("{at}: {e}"));
                check_events(&at, board.take_events(), events);
            }
            Step::Drop {
                card,
                onto,
                error,
                events,
            } => {
                match (board.drop_on(*card, *onto), error) {
                    (Ok(()), None) => {}
                    (Err(e), Some(expected)) => assert!(
                        format!("{e:?}").starts_with(expected.as_str()),
                        "{at}: expected {expected}, got {e:?}"
                    ),
                    (Ok(()), Some(expected)) => {
                        panic!("{at}: expected {expected}, but the drop succeeded")
                    }
                    (Err(e), None) => panic!("{at}: the drop failed: {e}"),
                }
                check_events(&at, board.take_events(), events);
            }
            Step::Split { card, events } => {
                board.split(*card).unwrap_or_else(|e| panic!("{at}: {e}"));
                check_events(&at, board.take_events(), events);
            }
            Step::Move { card, to } => {
                board
                    .move_stack(*card, (to[0], to[1]))
                    .unwrap_or_else(|e| panic!("{at}: {e}"));
                assert!(
                    board.take_events().is_empty(),
                    "{at}: moving queues no events"
                );
            }
            Step::Remove { card, events } => {
                board.remove(*card).unwrap_or_else(|e| panic!("{at}: {e}"));
                check_events(&at, board.take_events(), events);
            }
            Step::Tick { dt, events } => {
                let got = board.tick(*dt);
                check_events(&at, got, events);
            }
            Step::Check {
                progress,
                running,
                position,
            } => {
                for (card, want) in progress {
                    let got = board.progress(card_id(card));
                    assert_eq!(got, *want, "{at}: progress of card {card}");
                }
                for (card, want) in running {
                    let got = board.running_recipe(card_id(card));
                    assert_eq!(got, want.as_deref(), "{at}: running recipe of card {card}");
                }
                for (card, want) in position {
                    let got = board.card(card_id(card)).map(|c| [c.x, c.y]);
                    assert_eq!(got, Some(*want), "{at}: position of card {card}");
                }
            }
            Step::Reload => {
                let json = board.to_json().unwrap_or_else(|e| panic!("{at}: {e}"));
                board = Board::from_json(catalog.clone(), &json)
                    .unwrap_or_else(|e| panic!("{at}: {e}"));
            }
        }
    }

    if let Some(expect) = &case.expect {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for card in board.cards() {
            *counts.entry(card.def.clone()).or_insert(0) += 1;
        }
        assert_eq!(counts, expect.cards, "{name}: cards on the table");
        if let Some(stacks) = &expect.stacks {
            let mut got: Vec<Vec<u64>> = board.stacks().map(|(_, cards)| cards.to_vec()).collect();
            got.sort();
            let mut want = stacks.clone();
            want.sort();
            assert_eq!(got, want, "{name}: stacks on the table");
        }
    }
}

#[test]
fn the_shared_catalog_loads_and_round_trips() {
    let text = fs::read_to_string(fixtures_dir().join("catalog.json")).expect("catalog.json");
    let catalog = Catalog::from_json(&text).expect("catalog.json loads");
    assert!(
        catalog.recipes().len() >= 12,
        "the fixtures need at least twelve recipes"
    );
    let again = Catalog::from_json(&catalog.to_json().expect("writes")).expect("reads back");
    assert_eq!(again, catalog);
}

#[test]
fn every_case_plays_as_written() {
    let dir = fixtures_dir();
    let text = fs::read_to_string(dir.join("catalog.json")).expect("catalog.json");
    let catalog = Catalog::from_json(&text).expect("catalog.json loads");
    let mut paths: Vec<PathBuf> = fs::read_dir(dir.join("cases"))
        .expect("the cases folder")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    paths.sort();
    assert!(paths.len() >= 8, "found only {} cases", paths.len());
    for path in &paths {
        run_case(path, &catalog);
    }
}
