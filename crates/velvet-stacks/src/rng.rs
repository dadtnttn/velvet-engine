//! The engine's only source of randomness: a seeded xorshift generator.
//!
//! The generator's state is part of a saved board, so a loaded board rolls the same numbers as the
//! board it was saved from. The arithmetic is plain 64-bit integer work, so a JavaScript runtime
//! can reproduce it exactly with BigInt.

use serde::{Deserialize, Serialize};

/// Xorshift64 with the shifts 13, 7 and 17, and a 64-bit state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Starts a generator from `seed`. The seed is first mixed with one step of SplitMix64, so that
    /// small seeds (1, 2, 42) still start from a state with every bit spread out; a raw small state
    /// would give near-zero first draws. A mix that comes out as zero (an all-zero state would stay
    /// zero forever) is replaced by a fixed constant.
    pub fn new(seed: u64) -> Self {
        let mixed = splitmix64(seed);
        let state = if mixed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            mixed
        };
        Self { state }
    }

    /// Restores a generator from a saved state. `None` for zero, which no generator reaches.
    pub(crate) fn from_state(state: u64) -> Option<Self> {
        (state != 0).then_some(Self { state })
    }

    /// The state to save.
    pub(crate) fn state(&self) -> u64 {
        self.state
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// The next number in `[0, 1)`, built from the top 24 random bits so that it is exact in both
    /// `f32` and a JavaScript double.
    pub fn next_unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }
}

/// One step of SplitMix64: a fixed bijection on 64 bits that spreads every input bit over the output.
fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_values_match_the_specification() {
        let mut one = Xorshift64::new(1);
        assert_eq!(one.state(), 0x910a_2dec_8902_5cc1);
        assert_eq!(one.next_u64(), 0x7274_658b_cb6f_4838);
        assert_eq!(f64::from(one.next_unit()), 0.8223768472671509);
        let mut forty_two = Xorshift64::new(42);
        assert_eq!(forty_two.state(), 0xbdd7_3226_2feb_6e95);
        assert_eq!(forty_two.next_u64(), 0xfb4d_394f_8ead_bd08);
    }

    #[test]
    fn small_seeds_start_spread_out() {
        let high = (1..=64u64)
            .filter(|&s| Xorshift64::new(s).next_unit() >= 0.5)
            .count();
        assert!(high >= 20, "only {high} of 64 first draws are at least 0.5");
    }

    #[test]
    fn same_seed_gives_the_same_sequence() {
        let mut a = Xorshift64::new(42);
        let mut b = Xorshift64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn zero_seed_still_moves() {
        let mut rng = Xorshift64::new(0);
        assert_ne!(rng.next_u64(), 0);
    }

    #[test]
    fn unit_values_stay_in_range() {
        let mut rng = Xorshift64::new(7);
        for _ in 0..1000 {
            let v = rng.next_unit();
            assert!((0.0..1.0).contains(&v), "{v}");
        }
    }

    #[test]
    fn restored_state_continues_the_sequence() {
        let mut rng = Xorshift64::new(9);
        rng.next_u64();
        let mut restored = Xorshift64::from_state(rng.state()).expect("non-zero");
        assert_eq!(rng.next_u64(), restored.next_u64());
        assert!(Xorshift64::from_state(0).is_none());
    }
}
