//! Achievements: what the Queen has done, and what it earns her.
//!
//! The game only *reports* what happens, as short event names ("visit:termas",
//! "talk:mago", "shop:buy:pocion_pequena", ...) through [`Progress::record`]. Which of those
//! events count for something is data: `data/achievements/achievements.json` lists the
//! achievements (each a [`Condition`] over the events, gold, items or other achievements) and
//! what each one grants ([`Reward`]: an outfit for the wardrobe, items, gold, or a flag the
//! story can test). Adding an achievement is a JSON edit; adding a kind of event is one
//! `record` call where it happens. See `data/achievements/README.md` for the format and the
//! events the game reports.
//!
//! The progress (event counters, what is unlocked and when, the outfits and flags earned) is
//! saved with the 3D session.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

fn one() -> u32 {
    1
}

fn default_icon() -> String {
    "crown".to_owned()
}

/// When an achievement is earned.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Condition {
    /// An event has happened at least `count` times.
    Event {
        event: String,
        #[serde(default = "one")]
        count: u32,
    },
    /// At least `count` different events starting with `prefix` have happened
    /// (e.g. "visit:" with 5: five different places visited).
    Distinct { prefix: String, count: u32 },
    /// Another achievement is unlocked.
    Achievement { id: String },
    /// She holds at least this much gold.
    Gold { at_least: u32 },
    /// She carries at least `count` of an item.
    Item {
        id: String,
        #[serde(default = "one")]
        count: u32,
    },
    /// Every one of these.
    All { of: Vec<Condition> },
    /// Any one of these.
    Any { of: Vec<Condition> },
}

/// What an achievement grants, once, when it is unlocked.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Reward {
    /// An outfit of the catalog's `outfits` becomes wearable in the wardrobe.
    Outfit { id: String },
    /// Items into the inventory (ids of the shop's catalog).
    Item {
        id: String,
        #[serde(default = "one")]
        count: u32,
    },
    /// Gold into her purse.
    Gold { amount: u32 },
    /// A named flag, for the story or the world to test ([`Progress::has_flag`]).
    Flag { id: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Achievement {
    pub id: String,
    pub title: String,
    pub description: String,
    /// The badge's icon name (see `icon`).
    #[serde(default = "default_icon")]
    pub icon: String,
    /// Hidden achievements show as "???" until unlocked.
    #[serde(default)]
    pub hidden: bool,
    pub condition: Condition,
    #[serde(default)]
    pub rewards: Vec<Reward>,
}

/// One outfit of the wardrobe. `slot` is the wardrobe's index for it; `starter` outfits are
/// worn from the start, the others need a [`Reward::Outfit`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Outfit {
    pub id: String,
    pub slot: u8,
    pub name: String,
    #[serde(default)]
    pub starter: bool,
    /// Its own Queen model (a GLB in the characters folder, Mixamo-rigged with Queen_Walk /
    /// Queen_Run / Queen_Idle clips), for outfits past the two built in.
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Catalog {
    #[allow(dead_code)]
    pub version: u32,
    pub outfits: Vec<Outfit>,
    pub achievements: Vec<Achievement>,
}

impl Catalog {
    pub(crate) fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    #[allow(dead_code)]
    pub(crate) fn achievement(&self, id: &str) -> Option<&Achievement> {
        self.achievements.iter().find(|a| a.id == id)
    }

    pub(crate) fn outfit_in_slot(&self, slot: u8) -> Option<&Outfit> {
        self.outfits.iter().find(|o| o.slot == slot)
    }

    /// What is wrong with the catalog, if anything: duplicate ids, rewards or conditions that
    /// name unknown outfits, items or achievements. `known_item` answers for item ids.
    pub(crate) fn problems(&self, known_item: impl Fn(&str) -> bool) -> Vec<String> {
        let mut out = Vec::new();
        let mut ids = BTreeSet::new();
        for a in &self.achievements {
            if !ids.insert(a.id.as_str()) {
                out.push(format!("logro repetido: {}", a.id));
            }
        }
        let mut slots = BTreeSet::new();
        for o in &self.outfits {
            if !slots.insert(o.slot) {
                out.push(format!("dos atuendos en la ranura {}", o.slot));
            }
        }
        let outfit = |id: &str| self.outfits.iter().any(|o| o.id == id);
        fn walk(c: &Condition, f: &mut dyn FnMut(&Condition)) {
            f(c);
            if let Condition::All { of } | Condition::Any { of } = c {
                for c in of {
                    walk(c, f);
                }
            }
        }
        for a in &self.achievements {
            walk(&a.condition, &mut |c| match c {
                Condition::Achievement { id } if !ids.contains(id.as_str()) => {
                    out.push(format!("{}: pide un logro que no existe: {id}", a.id))
                }
                Condition::Achievement { id } if id == &a.id => {
                    out.push(format!("{}: se pide a sí mismo", a.id))
                }
                Condition::Item { id, .. } if !known_item(id) => {
                    out.push(format!("{}: objeto desconocido: {id}", a.id))
                }
                Condition::All { of } | Condition::Any { of } if of.is_empty() => {
                    out.push(format!("{}: grupo de condiciones vacío", a.id))
                }
                _ => {}
            });
            for r in &a.rewards {
                match r {
                    Reward::Outfit { id } if !outfit(id) => {
                        out.push(format!("{}: atuendo desconocido: {id}", a.id))
                    }
                    Reward::Item { id, .. } if !known_item(id) => {
                        out.push(format!("{}: objeto desconocido: {id}", a.id))
                    }
                    _ => {}
                }
            }
        }
        out
    }
}

/// The game's catalog (`data/achievements/achievements.json`).
pub(crate) fn catalog() -> &'static Catalog {
    static DATA: OnceLock<Catalog> = OnceLock::new();
    DATA.get_or_init(|| {
        let catalog = Catalog::parse(include_str!("../data/achievements/achievements.json"))
            .expect("valid achievements catalog");
        // Mistakes in the data are reported when the game starts, not when a reward is due.
        for problem in
            catalog.problems(|id| super::shop::catalog().products.iter().any(|p| p.id == id))
        {
            eprintln!("[logros] {problem}");
        }
        catalog
    })
}

/// What the conditions read besides the events: her purse and her inventory.
pub(crate) struct World<'a> {
    pub gold: u32,
    pub inventory: &'a BTreeMap<String, u32>,
}

/// An achievement just unlocked, with what it grants (outfits and flags are already recorded
/// in the progress; gold and items are for the caller to hand over).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Unlocked {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub rewards: Vec<Reward>,
}

/// Saved with the session.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct Progress {
    /// How many times each event has happened.
    pub counters: BTreeMap<String, u32>,
    /// Unlocked achievements, with the time (unix seconds) each was earned.
    pub unlocked: BTreeMap<String, u64>,
    /// Outfits earned by rewards.
    pub outfits: BTreeSet<String>,
    /// Flags earned by rewards.
    pub flags: BTreeSet<String>,
}

impl Progress {
    /// One more of `event`. Returns its new count.
    pub(crate) fn record(&mut self, event: &str) -> u32 {
        let count = self.counters.entry(event.to_owned()).or_default();
        *count = count.saturating_add(1);
        *count
    }

    pub(crate) fn count(&self, event: &str) -> u32 {
        self.counters.get(event).copied().unwrap_or(0)
    }

    pub(crate) fn is_unlocked(&self, id: &str) -> bool {
        self.unlocked.contains_key(id)
    }

    #[allow(dead_code)]
    pub(crate) fn has_flag(&self, id: &str) -> bool {
        self.flags.contains(id)
    }

    /// Gives her an outfit outside the achievements (a scene hands it over); true when it is new.
    pub(crate) fn grant_outfit(&mut self, id: &str) -> bool {
        self.outfits.insert(id.to_owned())
    }

    /// Can the outfit in this wardrobe slot be worn? Slots the catalog does not name stay
    /// wearable (the wardrobe's built-in looks).
    pub(crate) fn outfit_available(&self, catalog: &Catalog, slot: u8) -> bool {
        match catalog.outfit_in_slot(slot) {
            Some(o) => o.starter || self.outfits.contains(&o.id),
            None => true,
        }
    }

    fn holds(&self, c: &Condition, world: &World) -> bool {
        match c {
            Condition::Event { event, count } => self.count(event) >= *count,
            Condition::Distinct { prefix, count } => {
                self.counters
                    .iter()
                    .filter(|(k, v)| k.starts_with(prefix.as_str()) && **v > 0)
                    .count() as u32
                    >= *count
            }
            Condition::Achievement { id } => self.is_unlocked(id),
            Condition::Gold { at_least } => world.gold >= *at_least,
            Condition::Item { id, count } => {
                world.inventory.get(id).copied().unwrap_or(0) >= *count
            }
            Condition::All { of } => of.iter().all(|c| self.holds(c, world)),
            Condition::Any { of } => of.iter().any(|c| self.holds(c, world)),
        }
    }

    /// Unlocks every achievement whose condition now holds (chains included: one unlocked
    /// achievement may complete another), records their outfits and flags, and returns them.
    pub(crate) fn evaluate(&mut self, catalog: &Catalog, world: &World, now: u64) -> Vec<Unlocked> {
        let mut out = Vec::new();
        loop {
            let ready: Vec<&Achievement> = catalog
                .achievements
                .iter()
                .filter(|a| !self.is_unlocked(&a.id) && self.holds(&a.condition, world))
                .collect();
            if ready.is_empty() {
                return out;
            }
            for a in ready {
                self.unlocked.insert(a.id.clone(), now);
                for r in &a.rewards {
                    match r {
                        Reward::Outfit { id } => {
                            self.outfits.insert(id.clone());
                        }
                        Reward::Flag { id } => {
                            self.flags.insert(id.clone());
                        }
                        _ => {}
                    }
                }
                out.push(Unlocked {
                    id: a.id.clone(),
                    title: a.title.clone(),
                    icon: a.icon.clone(),
                    rewards: a.rewards.clone(),
                });
            }
        }
    }

    /// The achievements as a player would see them: (title, description, unlocked), hidden
    /// ones veiled until earned. For a future achievements screen.
    #[allow(dead_code)]
    pub(crate) fn listing(&self, catalog: &Catalog) -> Vec<(String, String, bool)> {
        catalog
            .achievements
            .iter()
            .map(|a| {
                let done = self.is_unlocked(&a.id);
                if a.hidden && !done {
                    ("???".to_owned(), "Logro oculto".to_owned(), false)
                } else {
                    (a.title.clone(), a.description.clone(), done)
                }
            })
            .collect()
    }
}

/// The badge icon of an achievement, among the game's prompt icons.
pub(crate) fn icon(name: &str) -> super::Icon {
    use super::Icon;
    match name {
        "bath" => Icon::Bath,
        "heart" => Icon::Heart,
        "forest" => Icon::Forest,
        "house" => Icon::House,
        "castle" => Icon::Castle,
        "mine" => Icon::Mine,
        "shop" => Icon::Shop,
        "talk" => Icon::Talk,
        "chest" => Icon::Chest,
        "wardrobe" => Icon::Wardrobe,
        "rune" => Icon::Rune,
        "feast" => Icon::Feast,
        _ => Icon::Crown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "version": 1,
        "outfits": [
            { "id": "reina", "slot": 0, "name": "Vestido de la Reina", "starter": true },
            { "id": "bikini", "slot": 2, "name": "Bikini" }
        ],
        "achievements": [
            { "id": "termas", "title": "Agua caliente", "description": "Visita los baños termales.",
              "icon": "bath", "condition": { "kind": "event", "event": "visit:termas" },
              "rewards": [ { "kind": "outfit", "id": "bikini" }, { "kind": "gold", "amount": 50 } ] },
            { "id": "viajera", "title": "Viajera", "description": "Visita tres lugares.",
              "condition": { "kind": "distinct", "prefix": "visit:", "count": 3 },
              "rewards": [ { "kind": "flag", "id": "conoce_el_mundo" } ] },
            { "id": "todo", "title": "Todo a la vez", "description": "Los dos anteriores.", "hidden": true,
              "condition": { "kind": "all", "of": [ { "kind": "achievement", "id": "termas" }, { "kind": "achievement", "id": "viajera" } ] },
              "rewards": [ { "kind": "item", "id": "pocion_pequena", "count": 2 } ] },
            { "id": "rica", "title": "Rica", "description": "Junta 500 de oro.",
              "condition": { "kind": "gold", "at_least": 500 } }
        ]
    }"#;

    fn world(gold: u32) -> (u32, BTreeMap<String, u32>) {
        (gold, BTreeMap::new())
    }

    #[test]
    fn the_shipped_catalog_is_valid() {
        let c = catalog();
        let problems = c.problems(|id| {
            super::super::shop::catalog()
                .products
                .iter()
                .any(|p| p.id == id)
        });
        assert!(problems.is_empty(), "{problems:?}");
        // The wardrobe's two looks are there and wearable from the start.
        let p = Progress::default();
        assert!(p.outfit_available(c, 0) && p.outfit_available(c, 1));
    }

    #[test]
    fn an_event_unlocks_once_and_grants_its_outfit() {
        let c = Catalog::parse(SAMPLE).unwrap();
        let (gold, inv) = world(0);
        let w = World {
            gold,
            inventory: &inv,
        };
        let mut p = Progress::default();
        assert!(!p.outfit_available(&c, 2), "the bikini starts locked");
        assert!(p.evaluate(&c, &w, 1).is_empty());
        p.record("visit:termas");
        let got = p.evaluate(&c, &w, 2);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "termas");
        assert!(got[0].rewards.contains(&Reward::Gold { amount: 50 }));
        assert!(p.outfit_available(&c, 2), "the bikini is earned");
        // A second visit earns nothing more.
        p.record("visit:termas");
        assert!(p.evaluate(&c, &w, 3).is_empty());
        assert_eq!(p.unlocked["termas"], 2);
    }

    #[test]
    fn distinct_places_and_chains_unlock_together() {
        let c = Catalog::parse(SAMPLE).unwrap();
        let (gold, inv) = world(0);
        let w = World {
            gold,
            inventory: &inv,
        };
        let mut p = Progress::default();
        for e in ["visit:bosque", "visit:villa", "visit:bosque"] {
            p.record(e);
        }
        assert!(p.evaluate(&c, &w, 1).is_empty(), "two places are not three");
        p.record("visit:termas");
        let ids: Vec<String> = p.evaluate(&c, &w, 2).into_iter().map(|u| u.id).collect();
        assert!(
            ids.contains(&"termas".to_owned())
                && ids.contains(&"viajera".to_owned())
                && ids.contains(&"todo".to_owned()),
            "{ids:?}"
        );
        assert!(p.has_flag("conoce_el_mundo"));
    }

    #[test]
    fn gold_and_hidden_listing() {
        let c = Catalog::parse(SAMPLE).unwrap();
        let mut p = Progress::default();
        let (_, inv) = world(0);
        assert!(p
            .evaluate(
                &c,
                &World {
                    gold: 499,
                    inventory: &inv
                },
                1
            )
            .is_empty());
        assert_eq!(
            p.evaluate(
                &c,
                &World {
                    gold: 500,
                    inventory: &inv
                },
                2
            )[0]
            .id,
            "rica"
        );
        let listing = p.listing(&c);
        assert!(
            listing.iter().any(|(t, _, done)| t == "???" && !done),
            "the hidden one is veiled"
        );
        assert!(listing.iter().any(|(t, _, done)| t == "Rica" && *done));
    }

    #[test]
    fn a_broken_catalog_is_reported() {
        let bad = r#"{ "version": 1, "outfits": [], "achievements": [
            { "id": "a", "title": "A", "description": "", "condition": { "kind": "achievement", "id": "nadie" },
              "rewards": [ { "kind": "outfit", "id": "capa" }, { "kind": "item", "id": "no_existe" } ] },
            { "id": "a", "title": "A2", "description": "", "condition": { "kind": "any", "of": [] } }
        ] }"#;
        let c = Catalog::parse(bad).unwrap();
        let problems = c.problems(|_| false);
        assert!(problems.len() >= 4, "{problems:?}");
    }

    #[test]
    fn progress_survives_a_save() {
        let mut p = Progress::default();
        p.record("visit:termas");
        p.unlocked.insert("termas".into(), 7);
        p.outfits.insert("bikini".into());
        let text = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<Progress>(&text).unwrap(), p);
        // An old save without the field reads as empty progress.
        assert_eq!(
            serde_json::from_str::<Progress>("{}").unwrap(),
            Progress::default()
        );
    }
}
