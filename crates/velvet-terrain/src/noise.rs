//! Deterministic 2D noise: seeded simplex, fBm, ridged multifractal and
//! domain warping.
//!
//! Everything derives from one `u64` seed through SplitMix64, so the same
//! seed always builds the same world on every platform (no `HashMap`
//! iteration, no system RNG, `f32` math only).

/// SplitMix64: tiny deterministic PRNG for seeding tables.
#[derive(Debug, Clone)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// New generator from a seed.
    pub fn new(seed: u64) -> Self {
        // SplitMix64 forbids the zero stream seed.
        Self {
            state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15),
        }
    }

    /// Next `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Next `f32` in `[0, 1)`.
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as f32) / 16_777_216.0
    }

    /// Next `f32` in `[lo, hi)`.
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }
}

/// 2D gradient vectors (8 compass + diagonals, unit-ish).
const GRAD2: [(f32, f32); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (1.0, 1.0),
    (-1.0, 1.0),
    (1.0, -1.0),
    (-1.0, -1.0),
];

/// Seeded 2D simplex noise (Stefan Gustavson style), output roughly `[-1, 1]`.
#[derive(Debug, Clone)]
pub struct Simplex2 {
    perm: [u8; 512],
}

impl Simplex2 {
    /// Build permutation tables from `seed` (Fisher–Yates over SplitMix64).
    pub fn new(seed: u64) -> Self {
        let mut rng = SplitMix64::new(seed);
        let mut p: Vec<u8> = (0..=255u8).collect();
        for i in (1..256).rev() {
            let j = (rng.next_u64() % (i as u64 + 1)) as usize;
            p.swap(i, j);
        }
        let mut perm = [0u8; 512];
        for (i, slot) in perm.iter_mut().enumerate() {
            *slot = p[i & 255];
        }
        Self { perm }
    }

    /// Sample at `(x, y)`.
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        const F2: f32 = 0.3660254; // 0.5 * (sqrt(3) - 1), f32-rounded
        const G2: f32 = 0.21132487; // (3 - sqrt(3)) / 6, f32-rounded
        let s = (x + y) * F2;
        let (i, j) = (fast_floor(x + s), fast_floor(y + s));
        let t = (i + j) as f32 * G2;
        let (x0, y0) = (x - (i as f32 - t), y - (j as f32 - t));
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let (x1, y1) = (x0 - i1 as f32 + G2, y0 - j1 as f32 + G2);
        let (x2, y2) = (x0 - 1.0 + 2.0 * G2, y0 - 1.0 + 2.0 * G2);
        let ii = (i & 255) as usize;
        let jj = (j & 255) as usize;
        let mut n = 0.0;
        n += corner(&self.perm, ii, jj, x0, y0);
        n += corner(&self.perm, (ii + i1) & 255, (jj + j1) & 255, x1, y1);
        n += corner(&self.perm, (ii + 1) & 255, (jj + 1) & 255, x2, y2);
        // ~[-1, 1] scale.
        70.0 * n
    }
}

fn fast_floor(v: f32) -> i32 {
    if v >= 0.0 {
        v as i32
    } else {
        v as i32 - 1
    }
}

fn corner(perm: &[u8; 512], i: usize, j: usize, x: f32, y: f32) -> f32 {
    let mut t = 0.5 - x * x - y * y;
    if t <= 0.0 {
        return 0.0;
    }
    t *= t;
    let g = GRAD2[perm[(i + perm[j] as usize) & 255] as usize & 7];
    t * t * (g.0 * x + g.1 * y)
}

/// Fractal Brownian motion over a simplex field.
#[derive(Debug, Clone)]
pub struct Fbm {
    /// Base noise.
    pub noise: Simplex2,
    /// Octaves (1..=8 typical).
    pub octaves: u32,
    /// Frequency multiplier per octave.
    pub lacunarity: f32,
    /// Amplitude multiplier per octave.
    pub gain: f32,
}

impl Fbm {
    /// New fBm field.
    pub fn new(seed: u64, octaves: u32, lacunarity: f32, gain: f32) -> Self {
        Self {
            noise: Simplex2::new(seed),
            octaves: octaves.max(1),
            lacunarity,
            gain,
        }
    }

    /// Sample in roughly `[-1, 1]` (normalized by amplitude sum).
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut sum = 0.0;
        let mut norm = 0.0;
        for _ in 0..self.octaves {
            sum += amp * self.noise.sample(x * freq, y * freq);
            norm += amp;
            amp *= self.gain;
            freq *= self.lacunarity;
        }
        sum / norm.max(1e-6)
    }

    /// Ridged multifractal (`(1 - |n|)^2` per octave): sharp mountain crests.
    pub fn ridged(&self, x: f32, y: f32) -> f32 {
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut sum = 0.0;
        let mut norm = 0.0;
        for _ in 0..self.octaves {
            let n = 1.0 - self.noise.sample(x * freq, y * freq).abs();
            sum += amp * n * n;
            norm += amp;
            amp *= self.gain;
            freq *= self.lacunarity;
        }
        sum / norm.max(1e-6)
    }
}

/// Domain warping (Quilez style): warp lookup coordinates with two offset
/// fBm fields before the final sample. Produces realistic valleys.
#[derive(Debug, Clone)]
pub struct DomainWarp {
    warp_x: Fbm,
    warp_y: Fbm,
    /// Warp strength in noise units.
    pub strength: f32,
}

impl DomainWarp {
    /// New warp from a seed (`strength` ~0.25 of the base frequency scale).
    pub fn new(seed: u64, strength: f32) -> Self {
        Self {
            warp_x: Fbm::new(seed ^ 0x9E37_79B9, 3, 2.0, 0.5),
            warp_y: Fbm::new(seed ^ 0x7F4A_7C15, 3, 2.0, 0.5),
            strength,
        }
    }

    /// Warp `(x, y)` into warped coordinates.
    pub fn warp(&self, x: f32, y: f32) -> (f32, f32) {
        const OFF: f32 = 5.2;
        let qx = self.warp_x.sample(x + OFF, y + OFF);
        let qy = self.warp_y.sample(x - OFF, y - OFF);
        (x + self.strength * qx, y + self.strength * qy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplex_is_deterministic_and_bounded() {
        let a = Simplex2::new(42);
        let b = Simplex2::new(42);
        let c = Simplex2::new(43);
        // Note: integer lattice points are exact zeros for any seed.
        for (x, y) in [(0.3, 0.0), (3.7, -1.2), (100.25, 55.5), (-8.0, 9.0)] {
            let v = a.sample(x, y);
            assert_eq!(v, b.sample(x, y));
            assert!((-1.5..=1.5).contains(&v), "{v}");
            // Different seed, (almost surely) different value.
            assert!((v - c.sample(x, y)).abs() > 1e-6);
        }
    }

    #[test]
    fn simplex_origin_anchor() {
        // Characterization anchor: guards accidental table changes.
        let n = Simplex2::new(1234);
        let v = n.sample(0.5, -0.25);
        assert!((v - 0.653_277_75).abs() < 1e-5, "{v}");
    }

    #[test]
    fn fbm_stays_in_range_and_varies_with_octaves() {
        let f = Fbm::new(7, 4, 2.0, 0.5);
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..64 {
            let v = f.sample(i as f32 * 0.37, i as f32 * -0.11);
            lo = lo.min(v);
            hi = hi.max(v);
            assert!((-1.01..=1.01).contains(&v), "{v}");
        }
        assert!(hi - lo > 0.5, "flat field: {lo}..{hi}");
        let r = Fbm::new(7, 4, 2.0, 0.5).ridged(2.0, 3.0);
        assert!((0.0..=1.01).contains(&r), "{r}");
    }

    #[test]
    fn warp_moves_but_stays_finite() {
        let w = DomainWarp::new(99, 0.3);
        let (x, y) = w.warp(10.0, -4.0);
        assert!(x.is_finite() && y.is_finite());
        assert!((x - 10.0).abs() + (y + 4.0).abs() > 1e-4);
        assert!((x - 10.0).abs() <= 0.31 && (y + 4.0).abs() <= 0.31);
    }

    #[test]
    fn splitmix_streams_differ() {
        let mut a = SplitMix64::new(1);
        let mut b = SplitMix64::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
        let f = SplitMix64::new(5).next_f32();
        assert!((0.0..1.0).contains(&f));
    }
}
