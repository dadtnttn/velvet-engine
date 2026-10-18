//! A small catalog shared by the unit tests of this crate. Not part of the public API.

use crate::def::{CardDef, Catalog, Selector, StackRule};
use crate::recipe::{Ingredient, Output, Recipe};

fn card(id: &str, kind: &str, tags: &[&str], stack: StackRule) -> CardDef {
    CardDef {
        id: id.into(),
        name: format!("card.{id}"),
        kind: kind.into(),
        tags: tags.iter().map(|t| (*t).into()).collect(),
        stack,
        value: 0,
    }
}

fn by_id(id: &str) -> Selector {
    Selector {
        id: Some(id.into()),
        ..Selector::default()
    }
}

fn by_tag(tag: &str) -> Selector {
    Selector {
        tag: Some(tag.into()),
        ..Selector::default()
    }
}

fn by_kind(kind: &str) -> Selector {
    Selector {
        kind: Some(kind.into()),
        ..Selector::default()
    }
}

fn need(select: Selector, count: u32, consume: bool) -> Ingredient {
    Ingredient {
        select,
        count,
        consume,
    }
}

fn make(card: &str, count: u32, chance: f32) -> Output {
    Output {
        card: card.into(),
        count,
        chance,
    }
}

fn recipe(
    id: &str,
    ingredients: Vec<Ingredient>,
    outputs: Vec<Output>,
    seconds: f32,
    priority: i32,
) -> Recipe {
    Recipe {
        id: id.into(),
        ingredients,
        seconds,
        outputs,
        priority,
    }
}

/// The test catalog: a worker that gathers, zones it can stand on, and a few recipes that cover
/// matching, consumption, priority and chance.
pub(crate) fn catalog() -> Catalog {
    let zone = StackRule {
        accepts: vec![by_kind("character")],
        max: 2,
    };
    let cards = vec![
        card(
            "worker",
            "character",
            &["gather"],
            StackRule {
                accepts: Vec::new(),
                max: 3,
            },
        ),
        card("forest", "zone", &[], zone.clone()),
        card("berry_bush", "zone", &[], zone),
        card("wood", "resource", &["wood"], StackRule::default()),
        card("stone", "resource", &["stone"], StackRule::default()),
        card("berries", "resource", &["food"], StackRule::default()),
        card("coin", "resource", &["money"], StackRule::default()),
        card(
            "house",
            "building",
            &["shelter"],
            StackRule {
                accepts: Vec::new(),
                max: 1,
            },
        ),
        card(
            "market",
            "building",
            &["trade"],
            StackRule {
                accepts: Vec::new(),
                max: 3,
            },
        ),
    ];
    let recipes = vec![
        recipe(
            "gather_wood",
            vec![
                need(by_tag("gather"), 1, false),
                need(by_id("forest"), 1, false),
            ],
            vec![make("wood", 1, 1.0)],
            2.0,
            0,
        ),
        recipe(
            "build_house",
            vec![need(by_id("wood"), 2, true), need(by_id("stone"), 1, true)],
            vec![make("house", 1, 1.0)],
            1.0,
            0,
        ),
        recipe(
            "pick_berries",
            vec![
                need(by_tag("gather"), 1, false),
                need(by_id("berry_bush"), 1, false),
            ],
            vec![make("berries", 1, 1.0)],
            2.0,
            0,
        ),
        recipe(
            "pick_berries_bonus",
            vec![
                need(by_tag("gather"), 1, false),
                need(by_id("berry_bush"), 1, false),
            ],
            vec![make("berries", 2, 1.0)],
            2.0,
            1,
        ),
        recipe(
            "sell_berries",
            vec![
                need(by_id("market"), 1, false),
                need(by_id("berries"), 2, true),
            ],
            vec![make("coin", 1, 0.5)],
            1.0,
            0,
        ),
    ];
    Catalog::new(cards, recipes).expect("the test catalog is valid")
}
