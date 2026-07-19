//! Poker hand evaluation + Balatro-like chip/mult base tables.

#![allow(dead_code)]

use std::fmt;

/// Suit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Hearts,
    Diamonds,
    Clubs,
    Spades,
}

impl Suit {
    pub fn all() -> [Suit; 4] {
        [Suit::Hearts, Suit::Diamonds, Suit::Clubs, Suit::Spades]
    }

    pub fn letter(self) -> char {
        match self {
            Suit::Hearts => 'H',
            Suit::Diamonds => 'D',
            Suit::Clubs => 'C',
            Suit::Spades => 'S',
        }
    }

    pub fn from_letter(c: char) -> Option<Suit> {
        match c.to_ascii_uppercase() {
            'H' => Some(Suit::Hearts),
            'D' => Some(Suit::Diamonds),
            'C' => Some(Suit::Clubs),
            'S' => Some(Suit::Spades),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Suit::Hearts => "Hearts",
            Suit::Diamonds => "Diamonds",
            Suit::Clubs => "Clubs",
            Suit::Spades => "Spades",
        }
    }
}

/// Rank A,2..10,J,Q,K — `value` is sort rank (A high = 14 for straights; chip face uses chip_value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rank(pub u8); // 2..=14 (14 = Ace)

impl Rank {
    pub fn from_char(c: char) -> Option<Rank> {
        match c.to_ascii_uppercase() {
            'A' => Some(Rank(14)),
            'K' => Some(Rank(13)),
            'Q' => Some(Rank(12)),
            'J' => Some(Rank(11)),
            'T' => Some(Rank(10)),
            d @ '2'..='9' => Some(Rank(d as u8 - b'0')),
            _ => None,
        }
    }

    pub fn label(self) -> char {
        match self.0 {
            14 => 'A',
            13 => 'K',
            12 => 'Q',
            11 => 'J',
            10 => 'T',
            n @ 2..=9 => (b'0' + n) as char,
            _ => '?',
        }
    }

    /// Balatro-ish chip contribution of the rank.
    pub fn chip_value(self) -> i64 {
        match self.0 {
            14 => 11, // Ace
            13 | 12 | 11 | 10 => 10,
            n => n as i64,
        }
    }
}

/// Playing card id string e.g. `AH`, `TS`, `2C`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayingCard {
    pub rank: Rank,
    pub suit: Suit,
}

impl PlayingCard {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Self { rank, suit }
    }

    pub fn id(&self) -> String {
        format!("{}{}", self.rank.label(), self.suit.letter())
    }

    pub fn parse(id: &str) -> Option<Self> {
        let b = id.as_bytes();
        if b.len() != 2 {
            return None;
        }
        let rank = Rank::from_char(b[0] as char)?;
        let suit = Suit::from_letter(b[1] as char)?;
        Some(Self { rank, suit })
    }

    pub fn short(&self) -> String {
        format!("{}{}", self.rank.label(), self.suit.letter())
    }
}

impl fmt::Display for PlayingCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short())
    }
}

/// Full 52-card deck as zone ids.
pub fn standard_deck_ids() -> Vec<String> {
    let mut out = Vec::with_capacity(52);
    for suit in Suit::all() {
        for r in 2u8..=14 {
            out.push(PlayingCard::new(Rank(r), suit).id());
        }
    }
    out
}

/// Poker hand category (best 5-card combination among selected).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HandKind {
    HighCard = 0,
    Pair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}

impl HandKind {
    pub fn name(self) -> &'static str {
        match self {
            HandKind::HighCard => "High Card",
            HandKind::Pair => "Pair",
            HandKind::TwoPair => "Two Pair",
            HandKind::ThreeOfAKind => "Three of a Kind",
            HandKind::Straight => "Straight",
            HandKind::Flush => "Flush",
            HandKind::FullHouse => "Full House",
            HandKind::FourOfAKind => "Four of a Kind",
            HandKind::StraightFlush => "Straight Flush",
            HandKind::RoyalFlush => "Royal Flush",
        }
    }

    /// Base chips and mult (Balatro-inspired, simplified).
    pub fn base_chips_mult(self) -> (i64, i64) {
        match self {
            HandKind::HighCard => (5, 1),
            HandKind::Pair => (10, 2),
            HandKind::TwoPair => (20, 2),
            HandKind::ThreeOfAKind => (30, 3),
            HandKind::Straight => (30, 4),
            HandKind::Flush => (35, 4),
            HandKind::FullHouse => (40, 4),
            HandKind::FourOfAKind => (60, 7),
            HandKind::StraightFlush => (100, 8),
            HandKind::RoyalFlush => (100, 8),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandScore {
    pub kind: HandKind,
    pub chips: i64,
    pub mult: i64,
    pub total: i64,
    /// Cards that scored (subset of selection used in ranking).
    pub scored: Vec<PlayingCard>,
}

/// Evaluate 1–5 selected cards as a poker hand (Balatro plays 1–5).
pub fn evaluate_hand(cards: &[PlayingCard]) -> HandScore {
    if cards.is_empty() {
        return HandScore {
            kind: HandKind::HighCard,
            chips: 0,
            mult: 1,
            total: 0,
            scored: vec![],
        };
    }

    // Use up to 5 cards (if more, take first 5 — UI should cap).
    let cards: Vec<PlayingCard> = cards.iter().take(5).cloned().collect();
    let kind = classify(&cards);
    let (base_chips, base_mult) = kind.base_chips_mult();
    let face: i64 = cards.iter().map(|c| c.rank.chip_value()).sum();
    let chips = base_chips + face;
    let mult = base_mult;
    let total = chips * mult;
    HandScore {
        kind,
        chips,
        mult,
        total,
        scored: cards,
    }
}

fn classify(cards: &[PlayingCard]) -> HandKind {
    let n = cards.len();
    if n == 0 {
        return HandKind::HighCard;
    }

    let mut ranks: Vec<u8> = cards.iter().map(|c| c.rank.0).collect();
    ranks.sort_unstable();
    let flush = n >= 5 && cards.windows(2).all(|w| w[0].suit == w[1].suit)
        || (n >= 5 && {
            let s0 = cards[0].suit;
            cards.iter().all(|c| c.suit == s0)
        });
    // flush for exactly the selected set when all same suit and len>=5
    let is_flush = n >= 5 && cards.iter().all(|c| c.suit == cards[0].suit);

    let is_straight = n >= 5 && is_straight_ranks(&ranks);

    // counts by rank
    let mut counts: Vec<(u8, u8)> = Vec::new();
    {
        let mut i = 0;
        while i < ranks.len() {
            let r = ranks[i];
            let mut c = 0u8;
            while i < ranks.len() && ranks[i] == r {
                c += 1;
                i += 1;
            }
            counts.push((r, c));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.cmp(&a.0)));

    let c1 = counts.first().map(|x| x.1).unwrap_or(0);
    let c2 = counts.get(1).map(|x| x.1).unwrap_or(0);

    if n >= 5 && is_flush && is_straight {
        if ranks.iter().any(|&r| r == 14) && ranks.iter().any(|&r| r == 13) {
            return HandKind::RoyalFlush;
        }
        return HandKind::StraightFlush;
    }
    if c1 >= 4 {
        return HandKind::FourOfAKind;
    }
    if c1 >= 3 && c2 >= 2 {
        return HandKind::FullHouse;
    }
    if is_flush {
        return HandKind::Flush;
    }
    if n >= 5 && is_straight {
        return HandKind::Straight;
    }
    if c1 >= 3 {
        return HandKind::ThreeOfAKind;
    }
    if c1 >= 2 && c2 >= 2 {
        return HandKind::TwoPair;
    }
    if c1 >= 2 {
        return HandKind::Pair;
    }
    let _ = flush;
    HandKind::HighCard
}

fn is_straight_ranks(sorted: &[u8]) -> bool {
    if sorted.len() < 5 {
        return false;
    }
    // unique ranks only
    let mut u = sorted.to_vec();
    u.dedup();
    if u.len() < 5 {
        return false;
    }
    // take last 5 unique for standard; for exactly 5 cards check consecutive
    if u.len() == 5 {
        // A-2-3-4-5 wheel
        if u == [2, 3, 4, 5, 14] {
            return true;
        }
        return u[4] - u[0] == 4 && u.windows(2).all(|w| w[1] == w[0] + 1);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(rank: u8, suit: Suit) -> PlayingCard {
        PlayingCard::new(Rank(rank), suit)
    }

    #[test]
    fn pair_scores() {
        let cards = vec![
            c(14, Suit::Hearts),
            c(14, Suit::Spades),
            c(3, Suit::Clubs),
        ];
        let s = evaluate_hand(&cards);
        assert_eq!(s.kind, HandKind::Pair);
        assert!(s.total > 0);
        assert_eq!(s.total, s.chips * s.mult);
    }

    #[test]
    fn flush_five() {
        let cards = vec![
            c(2, Suit::Hearts),
            c(5, Suit::Hearts),
            c(7, Suit::Hearts),
            c(9, Suit::Hearts),
            c(11, Suit::Hearts),
        ];
        let s = evaluate_hand(&cards);
        assert_eq!(s.kind, HandKind::Flush);
    }

    #[test]
    fn straight_wheel() {
        let cards = vec![
            c(14, Suit::Hearts),
            c(2, Suit::Spades),
            c(3, Suit::Clubs),
            c(4, Suit::Diamonds),
            c(5, Suit::Hearts),
        ];
        let s = evaluate_hand(&cards);
        assert_eq!(s.kind, HandKind::Straight);
    }

    #[test]
    fn deck_52() {
        assert_eq!(standard_deck_ids().len(), 52);
        assert!(PlayingCard::parse("AH").is_some());
        assert!(PlayingCard::parse("TS").is_some());
    }

    #[test]
    fn four_oak() {
        let cards = vec![
            c(7, Suit::Hearts),
            c(7, Suit::Spades),
            c(7, Suit::Clubs),
            c(7, Suit::Diamonds),
            c(2, Suit::Hearts),
        ];
        assert_eq!(evaluate_hand(&cards).kind, HandKind::FourOfAKind);
    }
}
