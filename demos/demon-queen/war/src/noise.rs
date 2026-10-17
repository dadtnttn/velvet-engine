//! Small deterministic helpers: a seeded random stream and smooth value noise, shared by the
//! battle and the campaign (the same numbers on every machine and every run).

/// A fast seeded random stream in [0, 1) (mulberry32).
#[derive(Clone, Debug)]
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// The next draw in [0, 1). (A generator's draw, not an `Iterator`: it never ends.)
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b_79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f64 / 4_294_967_296.0) as f32
    }

    /// A whole number in 0..n.
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f32) as usize).min(n.saturating_sub(1))
    }
}

pub fn clamp(v: f32, a: f32, b: f32) -> f32 {
    if v < a {
        a
    } else if v > b {
        b
    } else {
        v
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 0 at `a`, 1 at `b`, eased (either order).
pub fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash2(i: i32, j: i32, seed: i32) -> f32 {
    let mut h = i.wrapping_mul(374_761_393)
        ^ j.wrapping_mul(668_265_263)
        ^ seed.wrapping_mul(2_147_483_647);
    h = (h ^ ((h as u32) >> 13) as i32).wrapping_mul(1_274_126_177);
    ((h ^ ((h as u32) >> 16) as i32) as u32) as f32 / 4_294_967_296.0
}

/// Smooth value noise in [0, 1].
pub fn noise(x: f32, z: f32, seed: i32) -> f32 {
    let (i, j) = (x.floor() as i32, z.floor() as i32);
    let (fx, fz) = (x - i as f32, z - j as f32);
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sz = fz * fz * (3.0 - 2.0 * fz);
    let a = hash2(i, j, seed);
    let b = hash2(i + 1, j, seed);
    let c = hash2(i, j + 1, seed);
    let d = hash2(i + 1, j + 1, seed);
    a + (b - a) * sx + (c - a) * sz + (a - b - c + d) * sx * sz
}

/// Fractal noise in [0, 1].
pub fn fbm(x: f32, z: f32, seed: i32, octaves: u32) -> f32 {
    let (mut s, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for o in 0..octaves {
        s += noise(x * f, z * f, seed + o as i32 * 17) * amp;
        norm += amp;
        amp *= 0.5;
        f *= 2.03;
    }
    s / norm
}

/// A point of a resampled polyline: where, how far along, and the direction there.
#[derive(Clone, Copy, Debug, Default)]
pub struct LinePoint {
    pub x: f32,
    pub z: f32,
    /// Arc length from the start (metres).
    pub s: f32,
    pub tx: f32,
    pub tz: f32,
}

/// A polyline smoothed (Chaikin, `rounds` times) and resampled every `step` metres.
pub fn resample(pts: &[[f32; 2]], step: f32, rounds: u32) -> Vec<LinePoint> {
    let mut p: Vec<[f32; 2]> = pts.to_vec();
    for _ in 0..rounds {
        let mut q = vec![p[0]];
        for w in p.windows(2) {
            let ([ax, az], [bx, bz]) = (w[0], w[1]);
            q.push([ax * 0.75 + bx * 0.25, az * 0.75 + bz * 0.25]);
            q.push([ax * 0.25 + bx * 0.75, az * 0.25 + bz * 0.75]);
        }
        q.push(*p.last().unwrap());
        p = q;
    }
    let mut out = vec![LinePoint {
        x: p[0][0],
        z: p[0][1],
        ..Default::default()
    }];
    let (mut s, mut carry) = (0.0f32, 0.0f32);
    for w in p.windows(2) {
        let ([ax, az], [bx, bz]) = (w[0], w[1]);
        let len = (bx - ax).hypot(bz - az);
        let mut d = step - carry;
        while d <= len {
            let t = d / len;
            out.push(LinePoint {
                x: ax + (bx - ax) * t,
                z: az + (bz - az) * t,
                s: s + d,
                ..Default::default()
            });
            d += step;
        }
        carry = len - (d - step);
        s += len;
    }
    let last = *p.last().unwrap();
    let tail = out.last().unwrap();
    if (last[0] - tail.x).hypot(last[1] - tail.z) > step * 0.3 {
        out.push(LinePoint {
            x: last[0],
            z: last[1],
            s,
            ..Default::default()
        });
    }
    let n = out.len();
    for i in 0..n {
        let a = out[i.saturating_sub(1)];
        let b = out[(i + 1).min(n - 1)];
        let l = (b.x - a.x).hypot(b.z - a.z);
        let l = if l > 0.0 { l } else { 1.0 };
        out[i].tx = (b.x - a.x) / l;
        out[i].tz = (b.z - a.z) / l;
    }
    out
}

/// Nearest sample of a resampled line: distance, its index, arc length and signed side.
#[derive(Clone, Copy, Debug)]
pub struct Near {
    pub d: f32,
    pub i: usize,
    pub s: f32,
    pub side: f32,
}

pub fn line_near(x: f32, z: f32, line: &[LinePoint]) -> Near {
    let (mut best, mut bi) = (f32::MAX, 0usize);
    for (i, p) in line.iter().enumerate() {
        let (dx, dz) = (x - p.x, z - p.z);
        let d2 = dx * dx + dz * dz;
        if d2 < best {
            best = d2;
            bi = i;
        }
    }
    let c = line[bi];
    Near {
        d: best.sqrt(),
        i: bi,
        s: c.s,
        side: (x - c.x) * -c.tz + (z - c.z) * c.tx,
    }
}

/// Angle difference b - a, wrapped to (-π, π].
pub fn ang_diff(a: f32, b: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    let mut d = (b - a) % TAU;
    if d > PI {
        d -= TAU;
    }
    if d < -PI {
        d += TAU;
    }
    d
}

pub fn turn_to(a: f32, b: f32, k: f32) -> f32 {
    a + clamp(ang_diff(a, b), -k, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_is_deterministic_and_in_range() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..1000 {
            let x = a.next();
            assert_eq!(x, b.next());
            assert!((0.0..1.0).contains(&x));
        }
        // The same first values as the prototype's mulberry32(7).
        let mut r = Rng::new(7);
        for want in [0.011_704_753, 0.061_958_257, 0.976_907_6] {
            let got = r.next();
            assert!((got - want).abs() < 1e-6, "{got} != {want}");
        }
    }

    #[test]
    fn noise_matches_the_prototype() {
        for (got, want) in [
            (noise(3.7, 1.2, 3), 0.556_373_08),
            (noise(-5.5, 8.25, 21), 0.506_161_2),
            (fbm(1.3, 2.9, 7, 3), 0.294_582_2),
        ] {
            assert!((got - want).abs() < 1e-5, "{got} != {want}");
        }
    }

    #[test]
    fn noise_is_smooth_and_bounded() {
        for k in 0..200 {
            let x = k as f32 * 0.37;
            let v = noise(x, x * 0.7, 3);
            assert!((0.0..=1.0).contains(&v));
            let w = noise(x + 0.001, x * 0.7, 3);
            assert!((v - w).abs() < 0.01);
        }
    }

    #[test]
    fn a_resampled_line_is_evenly_spaced() {
        let line = resample(&[[0.0, 0.0], [100.0, 0.0]], 2.0, 0);
        assert!(line.len() >= 50);
        for w in line.windows(2) {
            assert!((w[1].s - w[0].s - 2.0).abs() < 0.01);
        }
        let n = line_near(50.0, 3.0, &line);
        assert!((n.d - 3.0).abs() < 0.01 && n.side.abs() > 2.9);
    }
}
