//! Card authoring tools CLI (`velvet cards …`) — tools, not a playable game.

use std::path::PathBuf;

use anyhow::{bail, Result};
use velvet_cards::{
    load_catalog_json, load_deck_json, validate_deck, CardZones, DeckRules, DeckViolation,
};

/// Validate a deck list against a catalog (optional size/copy caps).
pub fn cmd_cards_validate(
    catalog: PathBuf,
    deck: PathBuf,
    max_copies: Option<usize>,
    min_size: Option<usize>,
    max_size: Option<usize>,
) -> Result<()> {
    let cat = load_catalog_json(&catalog).map_err(|e| anyhow::anyhow!("{e}"))?;
    let list = load_deck_json(&deck).map_err(|e| anyhow::anyhow!("{e}"))?;
    let rules = DeckRules {
        max_copies,
        min_size,
        max_size,
    };
    let v = validate_deck(&cat, &list, &rules);
    println!(
        "catalog={} cards={} deck={} size={} ok={}",
        catalog.display(),
        cat.len(),
        deck.display(),
        v.size,
        v.ok
    );
    for x in &v.violations {
        match x {
            DeckViolation::UnknownCard { id, index } => {
                println!("  violation: unknown card `{id}` at index {index}");
            }
            DeckViolation::TooManyCards { size, max } => {
                println!("  violation: too many cards size={size} max={max}");
            }
            DeckViolation::TooFewCards { size, min } => {
                println!("  violation: too few cards size={size} min={min}");
            }
            DeckViolation::TooManyCopies { id, count, max } => {
                println!("  violation: too many copies of `{id}` count={count} max={max}");
            }
        }
    }
    if !v.ok {
        bail!(
            "deck validation failed ({} violation(s))",
            v.violations.len()
        );
    }
    println!("validate: OK");
    Ok(())
}

/// Shuffle (seeded) + optional draw; print zone sizes and contents (tooling dump).
pub fn cmd_cards_zones(
    catalog: PathBuf,
    deck: PathBuf,
    seed: u64,
    draw: usize,
    discard_hand_index: Option<usize>,
) -> Result<()> {
    let cat = load_catalog_json(&catalog).map_err(|e| anyhow::anyhow!("{e}"))?;
    let list = load_deck_json(&deck).map_err(|e| anyhow::anyhow!("{e}"))?;
    let v = validate_deck(&cat, &list, &DeckRules::open());
    if !v.ok {
        bail!(
            "deck invalid against catalog ({} violation(s)); fix before zone ops",
            v.violations.len()
        );
    }

    let mut zones = CardZones::from_deck_list(&list);
    zones.shuffle_library(seed);
    let library_after_shuffle: Vec<String> = zones.library.clone();

    let drawn = if draw > 0 {
        zones.draw(draw).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        Vec::new()
    };

    if let Some(idx) = discard_hand_index {
        let id = zones
            .discard_from_hand(idx)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("discarded_from_hand index={idx} id={id}");
    }

    println!("seed={seed}");
    println!(
        "zones library={} hand={} discard={} total={}",
        zones.library.len(),
        zones.hand.len(),
        zones.discard.len(),
        zones.total()
    );
    println!("library_order={}", library_after_shuffle.join(","));
    println!("drawn={}", drawn.join(","));
    println!("hand={}", zones.hand.join(","));
    println!("discard={}", zones.discard.join(","));
    Ok(())
}
