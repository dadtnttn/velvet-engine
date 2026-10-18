//! Runs the shared guion fixtures (`docs/formats/fixtures/guion`) through the boundary.
//!
//! Each `<name>.expected.json` names its guion file, and says what parsing that file reports,
//! what each entry lookup finds, and which boxes a run shows after each action. The JavaScript
//! tests in `js/packages/guion` read the same files, so both runtimes answer to one set of results.

use std::path::{Path, PathBuf};

use serde_json::Value;
use velvet_wasm::guion::{GuionConversation, GuionLibrary};

/// The folder of the shared guion fixtures.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/formats/fixtures/guion")
}

/// The expected-output files of the fixtures, in name order.
fn expected_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(fixtures_dir())
        .expect("the fixtures folder exists")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.to_string_lossy().ends_with(".expected.json"))
        .collect();
    files.sort();
    files
}

/// Reads JSON text returned by the boundary (or a fixture), which must be valid.
fn json(text: &str) -> Value {
    serde_json::from_str(text).expect("valid JSON")
}

/// Checks one fixture: the parse summary, the entry lookups, and every run.
fn check_fixture(path: &Path) {
    let expected = json(&std::fs::read_to_string(path).expect("the expected-output file"));
    let file = expected["file"].as_str().expect("the guion file name");
    let text = std::fs::read_to_string(fixtures_dir().join(file)).expect("the guion file");
    let cast = expected
        .get("cast")
        .map_or_else(|| "{}".to_owned(), Value::to_string);
    let mut lib = GuionLibrary::new(&cast).expect("the fixture's cast is valid");

    let summary = json(&lib.add(file, &text));
    assert_eq!(summary, expected["parse"], "{file}: the parse summary");

    for entry in expected["entries"].as_array().into_iter().flatten() {
        let who = entry["who"].as_str().map(str::to_owned);
        let place = entry["place"].as_str().map(str::to_owned);
        let found = json(&lib.entry(who, place, &entry["vars"].to_string()));
        assert_eq!(found["scene"], entry["scene"], "{file}: entry {entry}");
    }

    for run in expected["runs"].as_array().into_iter().flatten() {
        check_run(&lib, file, run);
    }
}

/// Plays one run: starts its scene, performs each action, and checks each box and the variables.
fn check_run(lib: &GuionLibrary, file: &str, run: &Value) {
    let scene = run["scene"].as_str().expect("the run's scene");
    let mut conv: GuionConversation = lib
        .start(scene, &run["vars"].to_string())
        .expect("the run's scene starts");
    for (index, step) in run["steps"].as_array().into_iter().flatten().enumerate() {
        let reply = match (index, step.get("do")) {
            (0, None) => conv.current(),
            (_, Some(Value::String(action))) if action == "next" => conv.advance(),
            (_, Some(Value::Object(choice))) => {
                let option = choice["choose"].as_u64().expect("a choose index");
                conv.choose(u32::try_from(option).expect("an index that fits"))
            }
            _ => panic!("{file}, {scene}: step {index} has no action it understands"),
        };
        check_reply(
            &reply,
            &step["expect"],
            &format!("{file}, {scene}, step {index}"),
        );
    }
    assert_eq!(
        json(&conv.vars()),
        run["varsAfter"],
        "{file}, {scene}: the variables after the run"
    );
}

/// Checks one reply: a box must equal the expected box; an expected `code` must come with a
/// failure of that code.
fn check_reply(reply: &str, expected: &Value, place: &str) {
    let actual = json(reply);
    if expected.get("code").is_some() && expected.get("type").is_none() {
        assert!(
            actual.get("error").is_some(),
            "{place}: expected a failure, got {reply}"
        );
        assert_eq!(
            actual["code"], expected["code"],
            "{place}: the failure code"
        );
    } else {
        assert_eq!(actual, *expected, "{place}");
    }
}

#[test]
fn every_fixture_matches_its_expected_output() {
    let files = expected_files();
    assert!(files.len() >= 5, "found {} fixtures", files.len());
    for path in &files {
        check_fixture(path);
    }
}
