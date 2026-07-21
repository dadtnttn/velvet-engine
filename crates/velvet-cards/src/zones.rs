//! Pure zone math: library, hand, discard.

use serde::{Deserialize, Serialize};
use velvet_math::Pcg32;

use crate::catalog::CardId;
use crate::deck::DeckList;
use crate::error::CardError;

/// Named zone for moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneKind {
    /// Draw pile (top = last index for pop-from-end draws).
    Library,
    /// Player hand.
    Hand,
    /// Discard pile.
    Discard,
}

/// Zone containers for tooling / sim smoke (not a full game rules engine).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardZones {
    /// Library — **top of deck is the last element** (draw pops back).
    pub library: Vec<CardId>,
    /// Hand.
    pub hand: Vec<CardId>,
    /// Discard.
    pub discard: Vec<CardId>,
}

impl CardZones {
    /// Empty zones.
    pub fn new() -> Self {
        Self::default()
    }

    /// Put an entire deck list into the library (hand/discard empty).
    pub fn from_deck_list(deck: &DeckList) -> Self {
        Self {
            library: deck.cards.clone(),
            hand: Vec::new(),
            discard: Vec::new(),
        }
    }

    /// Total cards across zones.
    pub fn total(&self) -> usize {
        self.library.len() + self.hand.len() + self.discard.len()
    }

    /// Shuffle the library with a **seeded** RNG (reproducible).
    pub fn shuffle_library(&mut self, seed: u64) {
        shuffle_in_place(&mut self.library, seed);
    }

    /// Draw up to `n` cards from library top into hand.
    ///
    /// Returns the ids drawn (library → hand order: first drawn first in return vec).
    pub fn draw(&mut self, n: usize) -> Result<Vec<CardId>, CardError> {
        if n > self.library.len() {
            return Err(CardError::NotEnough {
                zone: ZoneKind::Library,
                need: n,
                have: self.library.len(),
            });
        }
        let mut drawn = Vec::with_capacity(n);
        for _ in 0..n {
            let id = self.library.pop().expect("checked len");
            self.hand.push(id.clone());
            drawn.push(id);
        }
        Ok(drawn)
    }

    /// Move hand\[index\] to discard.
    pub fn discard_from_hand(&mut self, index: usize) -> Result<CardId, CardError> {
        self.move_card(ZoneKind::Hand, ZoneKind::Discard, index)
    }

    /// Move a card at `index` from `from` to the end of `to`.
    pub fn move_card(
        &mut self,
        from: ZoneKind,
        to: ZoneKind,
        index: usize,
    ) -> Result<CardId, CardError> {
        if from == to {
            // No-op relocate within same zone: still validate index.
            let zone = self.zone_mut(from);
            if index >= zone.len() {
                return Err(CardError::IndexOutOfRange {
                    zone: from,
                    index,
                    len: zone.len(),
                });
            }
            return Ok(zone[index].clone());
        }
        let id = {
            let src = self.zone_mut(from);
            if index >= src.len() {
                return Err(CardError::IndexOutOfRange {
                    zone: from,
                    index,
                    len: src.len(),
                });
            }
            src.remove(index)
        };
        self.zone_mut(to).push(id.clone());
        Ok(id)
    }

    /// Immutable slice for a zone.
    pub fn zone(&self, kind: ZoneKind) -> &[CardId] {
        match kind {
            ZoneKind::Library => &self.library,
            ZoneKind::Hand => &self.hand,
            ZoneKind::Discard => &self.discard,
        }
    }

    fn zone_mut(&mut self, kind: ZoneKind) -> &mut Vec<CardId> {
        match kind {
            ZoneKind::Library => &mut self.library,
            ZoneKind::Hand => &mut self.hand,
            ZoneKind::Discard => &mut self.discard,
        }
    }
}

/// Fisher–Yates shuffle using seeded [`Pcg32`].
pub fn shuffle_in_place(cards: &mut [CardId], seed: u64) {
    if cards.len() < 2 {
        return;
    }
    let mut rng = Pcg32::from_seed(seed);
    for i in (1..cards.len()).rev() {
        let j = rng.next_u32() as usize % (i + 1);
        cards.swap(i, j);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::DeckList;

    #[test]
    fn seeded_shuffle_is_reproducible() {
        let mut a = vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
            "f".into(),
        ];
        let mut b = a.clone();
        shuffle_in_place(&mut a, 0xC0FFEE);
        shuffle_in_place(&mut b, 0xC0FFEE);
        assert_eq!(a, b);
        // Not identity for this seed (extremely likely).
        let identity = ["a", "b", "c", "d", "e", "f"];
        assert_ne!(a, identity);
    }

    #[test]
    fn different_seeds_diverge() {
        let base = vec!["1".into(), "2".into(), "3".into(), "4".into(), "5".into()];
        let mut a = base.clone();
        let mut b = base;
        shuffle_in_place(&mut a, 1);
        shuffle_in_place(&mut b, 2);
        assert_ne!(a, b);
    }

    #[test]
    fn draw_and_discard_zones() {
        let deck = DeckList::from_ids(["a", "b", "c", "d"]);
        let mut z = CardZones::from_deck_list(&deck);
        assert_eq!(z.library.len(), 4);
        // Top is last: draw pulls "d" first.
        let drawn = z.draw(2).unwrap();
        assert_eq!(drawn, vec!["d", "c"]);
        assert_eq!(z.hand, vec!["d", "c"]);
        assert_eq!(z.library, vec!["a", "b"]);
        let id = z.discard_from_hand(0).unwrap();
        assert_eq!(id, "d");
        assert_eq!(z.hand, vec!["c"]);
        assert_eq!(z.discard, vec!["d"]);
        assert_eq!(z.total(), 4);
    }

    #[test]
    fn move_library_to_hand() {
        let mut z = CardZones::from_deck_list(&DeckList::from_ids(["x", "y"]));
        z.move_card(ZoneKind::Library, ZoneKind::Hand, 0).unwrap();
        assert_eq!(z.library, vec!["y"]);
        assert_eq!(z.hand, vec!["x"]);
    }

    #[test]
    fn draw_not_enough_errors() {
        let mut z = CardZones::from_deck_list(&DeckList::from_ids(["only"]));
        let err = z.draw(2).unwrap_err();
        assert!(matches!(
            err,
            CardError::NotEnough {
                zone: ZoneKind::Library,
                need: 2,
                have: 1
            }
        ));
    }
}
