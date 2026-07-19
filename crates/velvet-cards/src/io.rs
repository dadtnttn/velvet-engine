//! JSON load helpers for catalogs and decks.

use std::path::Path;

use crate::catalog::CardCatalog;
use crate::deck::DeckList;
use crate::error::CardError;

/// Parse catalog from JSON string.
pub fn parse_catalog_json(json: &str) -> Result<CardCatalog, CardError> {
    serde_json::from_str(json).map_err(|e| CardError::Parse(e.to_string()))
}

/// Parse deck list from JSON string.
pub fn parse_deck_json(json: &str) -> Result<DeckList, CardError> {
    serde_json::from_str(json).map_err(|e| CardError::Parse(e.to_string()))
}

/// Load catalog from a JSON file.
pub fn load_catalog_json(path: &Path) -> Result<CardCatalog, CardError> {
    let s = std::fs::read_to_string(path).map_err(|e| CardError::Io(e.to_string()))?;
    parse_catalog_json(&s)
}

/// Load deck from a JSON file.
pub fn load_deck_json(path: &Path) -> Result<DeckList, CardError> {
    let s = std::fs::read_to_string(path).map_err(|e| CardError::Io(e.to_string()))?;
    parse_deck_json(&s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::{validate_deck, DeckRules};
    use crate::zones::CardZones;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }

    #[test]
    fn roundtrip_json_catalog_deck() {
        let cat_json = r#"{
          "cards": {
            "strike": { "id": "strike", "name": "Strike", "cost": 1, "tags": ["attack"] },
            "guard": { "id": "guard", "name": "Guard", "cost": 1, "tags": ["defense"] }
          }
        }"#;
        let deck_json = r#"{ "name": "starter", "cards": ["strike", "strike", "guard"] }"#;
        let cat = parse_catalog_json(cat_json).unwrap();
        let deck = parse_deck_json(deck_json).unwrap();
        assert_eq!(cat.len(), 2);
        assert_eq!(deck.len(), 3);
        assert!(validate_deck(&cat, &deck, &DeckRules::open()).ok);
    }

    /// Drives shipped file loaders + validate + seeded zones on real fixtures.
    #[test]
    fn fixture_catalog_deck_validate_and_seeded_zones() {
        let dir = fixtures_dir();
        let cat = load_catalog_json(&dir.join("sample_catalog.json")).expect("catalog");
        let deck = load_deck_json(&dir.join("sample_deck.json")).expect("deck");
        let v = validate_deck(&cat, &deck, &DeckRules::open());
        assert!(v.ok, "{:?}", v.violations);
        assert_eq!(v.size, 10);

        let mut z1 = CardZones::from_deck_list(&deck);
        let mut z2 = CardZones::from_deck_list(&deck);
        z1.shuffle_library(42);
        z2.shuffle_library(42);
        assert_eq!(z1.library, z2.library, "same seed must match");
        let drawn = z1.draw(5).expect("draw");
        assert_eq!(drawn.len(), 5);
        assert_eq!(z1.hand.len(), 5);
        assert_eq!(z1.library.len(), 5);
        z1.discard_from_hand(0).expect("discard");
        assert_eq!(z1.hand.len(), 4);
        assert_eq!(z1.discard.len(), 1);
        assert_eq!(z1.total(), 10);

        let bad = load_deck_json(&dir.join("bad_deck_unknown.json")).expect("bad deck");
        let bad_v = validate_deck(&cat, &bad, &DeckRules::open());
        assert!(!bad_v.ok);
        assert!(bad_v.violations.iter().any(|x| matches!(
            x,
            crate::deck::DeckViolation::UnknownCard { id, .. } if id == "nope_card"
        )));
    }
}
