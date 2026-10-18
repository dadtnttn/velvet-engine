//! The conformance fixtures of the art manifest (`docs/formats/fixtures/art/`): every invalid manifest is
//! refused with the code that `cases.json` names, and the valid one is accepted.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The folder of the art fixtures, found from this crate's folder.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("formats")
        .join("fixtures")
        .join("art")
}

/// Reads `cases.json` of the art fixtures.
fn cases() -> Value {
    let text = fs::read_to_string(fixtures().join("cases.json")).expect("cases.json");
    serde_json::from_str(&text).expect("cases.json is JSON")
}

#[test]
fn every_invalid_manifest_is_refused_with_the_code_its_case_names() {
    let cases = cases();
    let invalid = cases["invalid"].as_array().expect("invalid is an array");
    let named: BTreeSet<String> = invalid
        .iter()
        .map(|case| case["file"].as_str().expect("file").to_string())
        .collect();
    let on_disk: BTreeSet<String> = fs::read_dir(fixtures().join("invalid"))
        .expect("invalid folder")
        .map(|entry| {
            format!(
                "invalid/{}",
                entry.expect("entry").file_name().to_string_lossy()
            )
        })
        .collect();
    assert_eq!(
        named, on_disk,
        "cases.json must name every file of invalid/, and only those"
    );

    for case in invalid {
        let file = case["file"].as_str().expect("file");
        let code = case["code"].as_str().expect("code");
        let text = fs::read_to_string(fixtures().join(file)).expect(file);
        let refusal = velvet_artbuild::parse_manifest(&text).expect_err(file);
        assert_eq!(
            refusal.code.as_str(),
            code,
            "{file} is refused for the wrong reason: {refusal}"
        );
        assert!(!refusal.issues.is_empty(), "{file} must list its problem");
    }
}

#[test]
fn the_conformance_manifest_is_accepted_in_full() {
    let cases = cases();
    let file = cases["manifest"].as_str().expect("manifest");
    let text = fs::read_to_string(fixtures().join(file)).expect(file);
    let manifest = velvet_artbuild::parse_manifest(&text).expect("the fixture manifest is valid");
    let ids: Vec<&str> = manifest
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "character.lyra.portrait",
            "character.lyra.fullbody",
            "card.wood",
            "icon.coin",
            "bg.forest",
            "prop.well",
        ]
    );
    let portrait = manifest.entry("character.lyra.portrait").expect("portrait");
    assert_eq!(portrait.size, [480, 640]);
    assert_eq!(portrait.anchor, [0.5, 1.0]);
    assert_eq!(portrait.variants, ["neutral", "happy", "worried"]);
    assert_eq!(portrait.stems().len(), 4);
    assert_eq!(
        manifest.entry("card.wood").expect("card").anchor,
        [0.0, 0.0]
    );
}
