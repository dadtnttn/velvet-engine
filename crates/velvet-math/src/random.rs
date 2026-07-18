//! Lightweight deterministic PRNGs (PCG-XSH-RR and xorshift32) for gameplay.

use crate::Vec2;

/// 32-bit PCG (XSH-RR) generator — good statistical quality, small state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Default for Pcg32 {
    fn default() -> Self {
        Self::new(0x853C_49E6_748F_EA9Bu64, 0xDA3E_39CB_94B9_5BDBu64)
    }
}

impl Pcg32 {
    /// Create with explicit state and stream (stream must be odd; forced odd).
    pub fn new(state: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(state);
        rng.next_u32();
        rng
    }

    /// Seed from a single 64-bit value.
    pub fn from_seed(seed: u64) -> Self {
        Self::new(seed, seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    /// Next raw 32-bit value.
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Next `u64` from two `u32`s.
    pub fn next_u64(&mut self) -> u64 {
        ((self.next_u32() as u64) << 32) | self.next_u32() as u64
    }

    /// Uniform `f32` in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        // 24 bits of mantissa precision.
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Uniform `f64` in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Inclusive integer range.
    pub fn gen_range_i32(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let span = (max as i64 - min as i64 + 1) as u32;
        min.wrapping_add((self.next_u32() % span) as i32)
    }

    /// Exclusive max `usize` range `[0, max)`.
    pub fn gen_index(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u32() as usize) % max
    }

    /// Uniform float in `[min, max)`.
    pub fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }

    /// Boolean with probability `p` of true.
    pub fn gen_bool(&mut self, p: f32) -> bool {
        self.next_f32() < p.clamp(0.0, 1.0)
    }

    /// Point uniformly in unit square `[0,1)²`.
    pub fn gen_vec2(&mut self) -> Vec2 {
        Vec2::new(self.next_f32(), self.next_f32())
    }

    /// Point uniformly in axis-aligned box.
    pub fn gen_in_rect(&mut self, min: Vec2, max: Vec2) -> Vec2 {
        Vec2::new(
            self.gen_range_f32(min.x, max.x),
            self.gen_range_f32(min.y, max.y),
        )
    }

    /// Point uniformly in unit disk.
    pub fn gen_in_unit_disk(&mut self) -> Vec2 {
        // Rejection sampling.
        loop {
            let p = Vec2::new(self.next_f32() * 2.0 - 1.0, self.next_f32() * 2.0 - 1.0);
            if p.length_squared() <= 1.0 {
                return p;
            }
        }
    }

    /// Unit direction (uniform angle).
    pub fn gen_unit_vec2(&mut self) -> Vec2 {
        let a = self.gen_range_f32(0.0, std::f32::consts::TAU);
        Vec2::from_angle(a)
    }

    /// Shuffle a slice in place (Fisher–Yates).
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.gen_index(i + 1);
            slice.swap(i, j);
        }
    }

    /// Choose a reference from a non-empty slice.
    pub fn choose<'a, T>(&mut self, slice: &'a [T]) -> Option<&'a T> {
        if slice.is_empty() {
            None
        } else {
            Some(&slice[self.gen_index(slice.len())])
        }
    }

    /// Weighted pick by non-negative weights; returns index.
    pub fn choose_weighted(&mut self, weights: &[f32]) -> Option<usize> {
        let sum: f32 = weights.iter().map(|w| w.max(0.0)).sum();
        if sum <= 0.0 || weights.is_empty() {
            return None;
        }
        let mut r = self.next_f32() * sum;
        for (i, w) in weights.iter().enumerate() {
            r -= w.max(0.0);
            if r <= 0.0 {
                return Some(i);
            }
        }
        Some(weights.len() - 1)
    }

    /// Gaussian (Box–Muller) sample with mean/std.
    pub fn gen_gaussian(&mut self, mean: f32, std_dev: f32) -> f32 {
        let u1 = self.next_f32().max(1e-7);
        let u2 = self.next_f32();
        let mag = (-2.0 * u1.ln()).sqrt();
        let z0 = mag * (std::f32::consts::TAU * u2).cos();
        mean + std_dev * z0
    }
}

/// Classic xorshift32 — tiny and fast, lower quality than PCG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct XorShift32 {
    state: u32,
}

impl Default for XorShift32 {
    fn default() -> Self {
        Self::new(0xA3C5_9AC3)
    }
}

impl XorShift32 {
    /// Create; zero seeds are replaced.
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    /// Next raw u32.
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform f32 in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Inclusive integer range.
    pub fn gen_range_i32(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let span = (max as i64 - min as i64 + 1) as u32;
        min.wrapping_add((self.next_u32() % span) as i32)
    }

    /// Uniform float range `[min, max)`.
    pub fn gen_range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
}

/// SplitMix64 — useful for seeding other generators from a single seed.
#[derive(Debug, Clone, Copy)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// Create from seed.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next u64.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Derive a [`Pcg32`].
    pub fn next_pcg32(&mut self) -> Pcg32 {
        let s = self.next_u64();
        let stream = self.next_u64();
        Pcg32::new(s, stream)
    }
}

/// Hash a 64-bit value to another (for seed derivation).
pub fn hash_u64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// Combine two seeds.
pub fn mix_seed(a: u64, b: u64) -> u64 {
    hash_u64(a ^ hash_u64(b.wrapping_add(0x9E37_79B9_7F4A_7C15)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcg_deterministic() {
        let mut a = Pcg32::from_seed(42);
        let mut b = Pcg32::from_seed(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn pcg_range_bounds() {
        let mut rng = Pcg32::from_seed(7);
        for _ in 0..200 {
            let v = rng.gen_range_i32(-3, 3);
            assert!((-3..=3).contains(&v));
            let f = rng.gen_range_f32(1.0, 2.0);
            assert!((1.0..2.0).contains(&f));
        }
    }

    #[test]
    fn unit_disk() {
        let mut rng = Pcg32::from_seed(99);
        for _ in 0..100 {
            let p = rng.gen_in_unit_disk();
            assert!(p.length_squared() <= 1.0 + 1e-5);
        }
    }

    #[test]
    fn shuffle_permutes() {
        let mut rng = Pcg32::from_seed(1);
        let mut v = vec![1, 2, 3, 4, 5];
        rng.shuffle(&mut v);
        v.sort();
        assert_eq!(v, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn weighted_choice() {
        let mut rng = Pcg32::from_seed(3);
        let mut counts = [0u32; 3];
        for _ in 0..3000 {
            let i = rng.choose_weighted(&[0.0, 1.0, 0.0]).unwrap();
            counts[i] += 1;
        }
        assert_eq!(counts[0], 0);
        assert_eq!(counts[2], 0);
        assert_eq!(counts[1], 3000);
    }

    #[test]
    fn xorshift_nonzero() {
        let mut x = XorShift32::new(0);
        assert_ne!(x.next_u32(), 0);
        let a = x.next_u32();
        let b = x.next_u32();
        assert_ne!(a, b);
    }

    #[test]
    fn splitmix_seeds_pcg() {
        let mut sm = SplitMix64::new(123);
        let mut r1 = sm.next_pcg32();
        let mut sm2 = SplitMix64::new(123);
        let mut r2 = sm2.next_pcg32();
        assert_eq!(r1.next_u32(), r2.next_u32());
    }

    #[test]
    fn gaussian_mean_rough() {
        let mut rng = Pcg32::from_seed(11);
        let mut sum = 0.0;
        let n = 2000;
        for _ in 0..n {
            sum += rng.gen_gaussian(0.0, 1.0);
        }
        let mean = sum / n as f32;
        assert!(mean.abs() < 0.15, "mean={mean}");
    }

    #[test]
    fn property_pcg_f32_in_unit_interval() {
        for seed in [0u64, 1, 42, 999_999, u64::MAX / 3] {
            let mut rng = Pcg32::from_seed(seed);
            for _ in 0..500 {
                let f = rng.next_f32();
                assert!((0.0..1.0).contains(&f), "f={f} seed={seed}");
                let d = rng.next_f64();
                assert!((0.0..1.0).contains(&d), "d={d}");
            }
        }
    }

    #[test]
    fn property_different_seeds_diverge() {
        let mut a = Pcg32::from_seed(1);
        let mut b = Pcg32::from_seed(2);
        let mut same = 0u32;
        for _ in 0..100 {
            if a.next_u32() == b.next_u32() {
                same += 1;
            }
        }
        assert!(same < 5, "streams should almost never collide, same={same}");
    }

    #[test]
    fn property_gen_index_and_bool() {
        let mut rng = Pcg32::from_seed(12345);
        assert_eq!(rng.gen_index(0), 0);
        for max in [1usize, 2, 7, 100] {
            for _ in 0..200 {
                let i = rng.gen_index(max);
                assert!(i < max || max == 0);
            }
        }
        let mut trues = 0u32;
        for _ in 0..1000 {
            if rng.gen_bool(0.3) {
                trues += 1;
            }
        }
        // Roughly 30% — allow wide band for stability.
        assert!(trues > 150 && trues < 500, "trues={trues}");
        assert!(!rng.gen_bool(0.0));
        assert!(rng.gen_bool(1.0));
    }

    #[test]
    fn property_rect_and_disk_and_vec2() {
        let mut rng = Pcg32::from_seed(77);
        let min = Vec2::new(-2.0, 1.0);
        let max = Vec2::new(3.0, 4.0);
        for _ in 0..200 {
            let p = rng.gen_in_rect(min, max);
            assert!(p.x >= min.x && p.x < max.x);
            assert!(p.y >= min.y && p.y < max.y);
            let u = rng.gen_vec2();
            assert!((0.0..1.0).contains(&u.x) && (0.0..1.0).contains(&u.y));
            let d = rng.gen_in_unit_disk();
            assert!(d.length_squared() <= 1.0 + 1e-4);
        }
    }

    #[test]
    fn property_shuffle_preserves_multiset() {
        let mut rng = Pcg32::from_seed(55);
        for n in [0usize, 1, 2, 10, 25] {
            let original: Vec<i32> = (0..n as i32).collect();
            let mut v = original.clone();
            rng.shuffle(&mut v);
            let mut sorted = v.clone();
            sorted.sort();
            assert_eq!(sorted, original);
        }
    }

    #[test]
    fn property_weighted_distribution() {
        let mut rng = Pcg32::from_seed(9);
        let weights = [1.0_f32, 3.0, 6.0];
        let mut counts = [0u32; 3];
        let trials = 6000u32;
        for _ in 0..trials {
            let i = rng.choose_weighted(&weights).unwrap();
            counts[i] += 1;
        }
        // Expected ratios 1:3:6 => ~10%, 30%, 60%
        let p0 = counts[0] as f32 / trials as f32;
        let p1 = counts[1] as f32 / trials as f32;
        let p2 = counts[2] as f32 / trials as f32;
        assert!((p0 - 0.1).abs() < 0.05, "p0={p0}");
        assert!((p1 - 0.3).abs() < 0.08, "p1={p1}");
        assert!((p2 - 0.6).abs() < 0.08, "p2={p2}");
        assert!(rng.choose_weighted(&[]).is_none());
        assert!(rng.choose_weighted(&[0.0, 0.0]).is_none());
    }

    #[test]
    fn property_hash_mix_seed_stable() {
        assert_eq!(hash_u64(0), hash_u64(0));
        assert_ne!(hash_u64(1), hash_u64(2));
        assert_eq!(mix_seed(1, 2), mix_seed(1, 2));
        assert_ne!(mix_seed(1, 2), mix_seed(2, 1));
        // Avalanche-ish: flipping one bit changes output a lot.
        let a = hash_u64(0x1000);
        let b = hash_u64(0x1001);
        assert_ne!(a, b);
        let bits = (a ^ b).count_ones();
        assert!(bits > 8, "bits flipped={bits}");
    }

    #[test]
    fn property_xorshift_and_splitmix_streams() {
        let mut x = XorShift32::new(1);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            seen.insert(x.next_u32());
        }
        assert!(seen.len() > 190);
        let mut sm = SplitMix64::new(999);
        let a = sm.next_u64();
        let b = sm.next_u64();
        assert_ne!(a, b);
        let mut r = sm.next_pcg32();
        let v1 = r.next_u32();
        let mut sm2 = SplitMix64::new(999);
        // Advance past the same two next_u64 calls
        let _ = sm2.next_u64();
        let _ = sm2.next_u64();
        let mut r2 = sm2.next_pcg32();
        assert_eq!(v1, r2.next_u32());
    }

    #[test]
    fn property_gaussian_variance_rough() {
        let mut rng = Pcg32::from_seed(123);
        let n = 4000;
        let mut sum = 0.0f32;
        let mut sum_sq = 0.0f32;
        for _ in 0..n {
            let v = rng.gen_gaussian(2.0, 1.5);
            sum += v;
            sum_sq += v * v;
        }
        let mean = sum / n as f32;
        let var = sum_sq / n as f32 - mean * mean;
        assert!((mean - 2.0).abs() < 0.2, "mean={mean}");
        // Variance should be around 2.25
        assert!((var - 2.25).abs() < 0.8, "var={var}");
    }
}
