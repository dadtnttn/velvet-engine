//! Local card definitions + Balatro-like scoring for the illustrated set.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use velvet_cards::{CardCatalog, CardDef, DeckList};

/// Runtime stats for scoring (tools-friendly, not a full TCG).
#[derive(Debug, Clone)]
pub struct CardStats {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub chips: i64,
    pub mult: i64,
    /// Extra cards drawn after this card is played.
    pub bonus_draw: usize,
    pub kind: CardKind,
    /// Path to illustration (loaded via ArtBank).
    #[allow(dead_code)]
    pub art: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    Attack,
    Defense,
    Spell,
    Skill,
}

impl CardKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Attack => "ATK",
            Self::Defense => "DEF",
            Self::Spell => "SPL",
            Self::Skill => "SKL",
        }
    }
}

impl CardStats {
    /// Compact rules copy for the gameplay card footer/tooltip.
    pub fn rules_text(&self) -> String {
        let mut parts = vec![format!("+{} chips", self.chips)];
        if self.mult > 1 {
            parts.push(format!("+{} mult", self.mult - 1));
        }
        if self.bonus_draw > 0 {
            parts.push(format!("draw {}", self.bonus_draw));
        }
        parts.join(" · ")
    }
}

/// Built-in illustrated set (5 cards).
pub fn illustrated_stats(art_dir: &Path) -> Vec<CardStats> {
    vec![
        CardStats {
            id: "strike".into(),
            name: "Strike".into(),
            cost: 1,
            chips: 18,
            mult: 1,
            bonus_draw: 0,
            kind: CardKind::Attack,
            art: art_dir.join("strike.jpg"),
        },
        CardStats {
            id: "guard".into(),
            name: "Guard".into(),
            cost: 1,
            chips: 12,
            mult: 1,
            bonus_draw: 0,
            kind: CardKind::Defense,
            art: art_dir.join("guard.jpg"),
        },
        CardStats {
            id: "fireball".into(),
            name: "Fireball".into(),
            cost: 3,
            chips: 35,
            mult: 2,
            bonus_draw: 0,
            kind: CardKind::Spell,
            art: art_dir.join("fireball.jpg"),
        },
        CardStats {
            id: "focus".into(),
            name: "Focus".into(),
            cost: 1,
            chips: 8,
            mult: 1,
            bonus_draw: 1,
            kind: CardKind::Skill,
            art: art_dir.join("focus.jpg"),
        },
        CardStats {
            id: "bash".into(),
            name: "Bash".into(),
            cost: 2,
            chips: 28,
            mult: 1,
            bonus_draw: 0,
            kind: CardKind::Attack,
            art: art_dir.join("bash.jpg"),
        },
    ]
}

/// Catalog for velvet-cards validation + starter deck list.
pub fn make_catalog_and_deck(
    art_dir: &Path,
) -> (CardCatalog, DeckList, HashMap<String, CardStats>) {
    let stats = illustrated_stats(art_dir);
    let mut map = HashMap::new();
    let mut cat = CardCatalog::new();
    for s in &stats {
        cat.insert(
            CardDef::new(&s.id, &s.name, s.cost)
                .with_tag(s.kind.label().to_lowercase())
                .with_type(s.kind.label()),
        );
        map.insert(s.id.clone(), s.clone());
    }
    // Balanced starter deck (20 cards)
    let deck = DeckList::from_ids([
        "strike", "strike", "strike", "strike", "guard", "guard", "guard", "bash", "bash", "bash",
        "focus", "focus", "focus", "fireball", "fireball", "strike", "guard", "bash", "focus",
        "fireball",
    ]);
    (cat, deck, map)
}

/// Score a selected hand Balatro-style: chips × mult with set bonuses.
pub fn score_played(ids: &[String], stats: &HashMap<String, CardStats>) -> HandScore {
    if ids.is_empty() {
        return HandScore {
            chips: 0,
            mult: 1,
            total: 0,
            label: "Empty".into(),
        };
    }
    let mut chips: i64 = 5; // high-card base
    let mut mult: i64 = 1;
    let mut attacks = 0u32;
    let mut defs = 0u32;
    let mut spells = 0u32;
    let mut skills = 0u32;
    let mut counts: HashMap<&str, u32> = HashMap::new();

    for id in ids {
        *counts.entry(id.as_str()).or_insert(0) += 1;
        if let Some(s) = stats.get(id) {
            chips += s.chips;
            mult += s.mult.saturating_sub(1); // only extra mult from card
            match s.kind {
                CardKind::Attack => attacks += 1,
                CardKind::Defense => defs += 1,
                CardKind::Spell => spells += 1,
                CardKind::Skill => skills += 1,
            }
        }
    }
    // pair/triple of same card id
    let max_copy = counts.values().copied().max().unwrap_or(0);
    if max_copy >= 3 {
        chips += 40;
        mult += 3;
    } else if max_copy >= 2 {
        chips += 15;
        mult += 1;
    }
    // type synergies
    if attacks >= 3 {
        mult += 2;
        chips += 20;
    } else if attacks >= 2 {
        mult += 1;
    }
    if defs >= 2 {
        chips += 25;
        mult += 1;
    }
    if spells >= 1 && attacks >= 1 {
        mult += 1; // spell + attack
        chips += 10;
    }
    if skills >= 1 {
        chips += 5 * skills as i64;
    }
    if mult < 1 {
        mult = 1;
    }
    let label = combo_label(attacks, defs, spells, skills, max_copy);
    let total = chips * mult;
    HandScore {
        chips,
        mult,
        total,
        label,
    }
}

fn combo_label(a: u32, d: u32, s: u32, k: u32, copies: u32) -> String {
    if copies >= 3 {
        return "Triple!".into();
    }
    if copies >= 2 && a >= 2 {
        return "Twin Strike".into();
    }
    if a >= 3 {
        return "Assault".into();
    }
    if a >= 2 {
        return "Double Attack".into();
    }
    if d >= 2 {
        return "Iron Wall".into();
    }
    if s >= 1 && a >= 1 {
        return "Spellblade".into();
    }
    if s >= 1 {
        return "Arcane".into();
    }
    if k >= 1 && a + d + s == 0 {
        return "Focus Flow".into();
    }
    if a + d + s + k >= 4 {
        return "Full Hand".into();
    }
    "Play".into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandScore {
    pub chips: i64,
    pub mult: i64,
    pub total: i64,
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> HashMap<String, CardStats> {
        illustrated_stats(Path::new("."))
            .into_iter()
            .map(|card| (card.id.clone(), card))
            .collect()
    }

    #[test]
    fn empty_selection_has_zero_total() {
        assert_eq!(
            score_played(&[], &stats()),
            HandScore {
                chips: 0,
                mult: 1,
                total: 0,
                label: "Empty".into(),
            }
        );
    }

    #[test]
    fn preview_reports_pair_and_spellblade_combos() {
        let stats = stats();
        let pair = score_played(&["strike".into(), "strike".into()], &stats);
        assert_eq!(pair.label, "Twin Strike");
        assert_eq!(pair.total, 168);

        let spellblade = score_played(&["strike".into(), "fireball".into()], &stats);
        assert_eq!(spellblade.label, "Spellblade");
        assert_eq!(spellblade.total, 204);
    }

    #[test]
    fn card_rules_copy_matches_runtime_effects() {
        let stats = stats();
        assert_eq!(stats["focus"].rules_text(), "+8 chips · draw 1");
        assert_eq!(stats["fireball"].rules_text(), "+35 chips · +1 mult");
    }
}
