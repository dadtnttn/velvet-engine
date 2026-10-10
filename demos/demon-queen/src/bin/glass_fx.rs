//! The screen's glass, when Chiquita hits it. From where she hit, a star of cracks runs out in a
//! blink (jagged rays, rings of cracks between them, a crushed spot and a flash at the middle),
//! drawn as glass breaks show: a dark line under a pale one, a glint along it. When it falls apart
//! the pieces between the rays and the rings drop away, turning, and fade. Everything comes from a
//! seed, so the same blow always breaks the same way.
use super::ui_paint::*;
use super::*;

/// A crack in the screen.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq)]
pub struct Crack {
    /// Where she hit (px) and how far the cracks run (px).
    pub center: [i32; 2],
    pub reach: u16,
    pub seed: u32,
    /// Since it cracked (ms).
    pub age_ms: u32,
    /// Since it began to fall apart (ms), once it has.
    pub fall_ms: Option<u32>,
    /// How much of it is left, 0..=1000 (it fades away when it is not falling).
    pub alpha: u16,
}

/// A small deterministic random stream.
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 100_000) as f32 / 100_000.0
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next()
    }
}

/// The cracks' shape: each ray as a polyline out from the centre (with each point's distance).
struct Pattern {
    rays: Vec<Vec<([f32; 2], f32)>>,
    rings: Vec<f32>,
}

fn pattern(crack: &Crack) -> Pattern {
    let mut rng = Rng(crack.seed.max(1).wrapping_mul(2_654_435_761) | 1);
    let reach = crack.reach.max(40) as f32;
    let n = 9 + (rng.next() * 4.0) as usize;
    let c = [crack.center[0] as f32, crack.center[1] as f32];
    let mut rays = Vec::with_capacity(n);
    for i in 0..n {
        let mut angle = (i as f32 + rng.range(-0.3, 0.3)) / n as f32 * std::f32::consts::TAU;
        let length = reach * rng.range(0.6, 1.15);
        let mut p = [c[0] + angle.cos() * 5.0, c[1] + angle.sin() * 5.0];
        let mut d = 5.0;
        let mut points = vec![(p, d)];
        while d < length {
            let step = rng.range(26.0, 58.0).min(length - d + 1.0);
            angle += rng.range(-0.24, 0.24);
            p = [p[0] + angle.cos() * step, p[1] + angle.sin() * step];
            d += step;
            points.push((p, d));
        }
        rays.push(points);
    }
    Pattern {
        rays,
        rings: vec![reach * 0.2, reach * 0.42, reach * 0.68],
    }
}

/// The point of `ray` at distance `d` from the centre (its last point past its end).
fn along(ray: &[([f32; 2], f32)], d: f32) -> Option<[f32; 2]> {
    if ray.last().map_or(true, |l| l.1 < d) {
        return None;
    }
    for pair in ray.windows(2) {
        let ((a, da), (b, db)) = (pair[0], pair[1]);
        if d <= db {
            let t = ((d - da) / (db - da).max(1e-3)).clamp(0.0, 1.0);
            return Some([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    None
}

/// One crack line: a dark groove, the pale glass edge over it, a glint along it.
fn line(v: &mut Vec<MapVertex>, a: [f32; 2], b: [f32; 2], weight: f32, alpha: f32) {
    let shadow = [a[0] + 1.2, a[1] + 1.5];
    let shadow_b = [b[0] + 1.2, b[1] + 1.5];
    wire(
        v,
        shadow,
        shadow_b,
        3.4 * weight,
        rgba(0x0b0610, 0.5 * alpha),
        rgba(0x0b0610, 0.5 * alpha),
        0.0,
        false,
    );
    wire(
        v,
        a,
        b,
        1.9 * weight,
        rgba(0xe8eeff, 0.92 * alpha),
        rgba(0xe8eeff, 0.92 * alpha),
        0.0,
        false,
    );
    wire(
        v,
        [a[0] - 0.6, a[1] - 0.6],
        [b[0] - 0.6, b[1] - 0.6],
        0.8 * weight,
        rgba(0xffffff, 0.75 * alpha),
        rgba(0xffffff, 0.75 * alpha),
        0.0,
        false,
    );
}

pub(super) fn draw(v: &mut Vec<MapVertex>, crack: &Crack, time: f32) {
    let alpha = crack.alpha.min(1000) as f32 / 1000.0;
    if alpha <= 0.0 {
        return;
    }
    let age = crack.age_ms as f32 / 1000.0;
    let c = [crack.center[0] as f32, crack.center[1] as f32];
    let pattern = pattern(crack);
    let reach = crack.reach.max(40) as f32;
    if let Some(ms) = crack.fall_ms {
        shatter(v, &pattern, c, ms as f32 / 1000.0, crack.seed, alpha);
        return;
    }
    // The cracks run out in a blink.
    let grown = reach * (1.0 - (1.0 - (age / 0.16).min(1.0)).powi(3)) * 1.2;
    // The flash of the blow and a faint bloom of broken glass round it.
    let flash = (1.0 - age / 0.4).max(0.0);
    if flash > 0.0 {
        halo(
            v,
            c,
            reach * 0.25,
            reach * 0.5,
            rgba(0xffffff, 0.55 * flash * flash * alpha),
        );
    }
    halo(
        v,
        c,
        reach * 0.4,
        reach * 0.35,
        rgba(0xdfe8ff, 0.07 * alpha),
    );
    for ray in &pattern.rays {
        for pair in ray.windows(2) {
            let ((a, da), (b, db)) = (pair[0], pair[1]);
            if da > grown {
                break;
            }
            let b = if db > grown {
                let t = ((grown - da) / (db - da).max(1e-3)).clamp(0.0, 1.0);
                [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
            } else {
                b
            };
            let weight = 1.25 - 0.55 * (da / reach).min(1.0);
            line(v, a, b, weight, alpha);
        }
    }
    let mut rng = Rng(crack.seed.max(1).wrapping_mul(97) | 1);
    for (k, &r) in pattern.rings.iter().enumerate() {
        if r > grown {
            break;
        }
        let keep = [0.9, 0.7, 0.45][k];
        let n = pattern.rays.len();
        for i in 0..n {
            let (Some(a), Some(b)) = (
                along(&pattern.rays[i], r * rng.range(0.9, 1.1)),
                along(&pattern.rays[(i + 1) % n], r * rng.range(0.9, 1.1)),
            ) else {
                continue;
            };
            if rng.next() > keep {
                continue;
            }
            // A ring crack sags toward the middle a little.
            let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
            let inward = [
                (c[0] - mid[0]) * rng.range(0.03, 0.1),
                (c[1] - mid[1]) * rng.range(0.03, 0.1),
            ];
            let m = [mid[0] + inward[0], mid[1] + inward[1]];
            line(v, a, m, 0.8, alpha * 0.9);
            line(v, m, b, 0.8, alpha * 0.9);
        }
    }
    // The crushed spot: white grit and a few chips.
    disc(v, c, 9.0, rgba(0xf4f6ff, 0.55 * alpha));
    for i in 0..10 {
        let a = rng.range(0.0, std::f32::consts::TAU);
        let d = rng.range(4.0, 22.0);
        let p = [c[0] + a.cos() * d, c[1] + a.sin() * d];
        let s = rng.range(2.0, 5.0);
        push_triangle(
            v,
            [
                [p[0], p[1] - s],
                [p[0] + s, p[1] + s * 0.6],
                [p[0] - s * 0.8, p[1] + s * 0.4],
            ],
            rgba(0xffffff, 0.7 * alpha),
        );
        let _ = i;
    }
    let glint = 0.5 + 0.5 * (time * 3.0).sin();
    sparkle(
        v,
        [c[0] - reach * 0.08, c[1] - reach * 0.1],
        10.0,
        rgba(0xffffff, 0.6 * glint * alpha),
        0.6,
    );
}

/// The glass falling apart: the cells between rays and rings drop, turning, and fade.
fn shatter(v: &mut Vec<MapVertex>, pattern: &Pattern, c: [f32; 2], t: f32, seed: u32, alpha: f32) {
    let mut rng = Rng(seed.max(1).wrapping_mul(31) | 1);
    let n = pattern.rays.len();
    let mut radii = vec![6.0];
    radii.extend(pattern.rings.iter().copied());
    radii.push(f32::INFINITY);
    for k in 0..radii.len() - 1 {
        for i in 0..n {
            let (r0, r1) = (radii[k], radii[k + 1]);
            let ray_a = &pattern.rays[i];
            let ray_b = &pattern.rays[(i + 1) % n];
            let outer =
                |ray: &[([f32; 2], f32)], r: f32| along(ray, r).or_else(|| ray.last().map(|l| l.0));
            let (Some(a0), Some(b0)) = (outer(ray_a, r0), outer(ray_b, r0)) else {
                continue;
            };
            let (Some(a1), Some(b1)) = (outer(ray_a, r1), outer(ray_b, r1)) else {
                continue;
            };
            let corners = [a0, b0, b1, a1];
            let mid = [
                corners.iter().map(|p| p[0]).sum::<f32>() / 4.0,
                corners.iter().map(|p| p[1]).sum::<f32>() / 4.0,
            ];
            let delay = rng.range(0.0, 0.18) + k as f32 * 0.04;
            let tt = (t - delay).max(0.0);
            let out = [mid[0] - c[0], mid[1] - c[1]];
            let vx = out[0] * rng.range(0.4, 1.0) + rng.range(-60.0, 60.0);
            let vy = rng.range(-140.0, 20.0);
            let spin = rng.range(-3.5, 3.5);
            let dx = vx * tt;
            let dy = vy * tt + 0.5 * 2200.0 * tt * tt;
            let fade = (1.0 - tt / 0.9).clamp(0.0, 1.0) * alpha;
            if fade <= 0.0 {
                continue;
            }
            let (s, co) = (spin * tt).sin_cos();
            let moved: Vec<[f32; 2]> = corners
                .iter()
                .map(|p| {
                    let (x, y) = (p[0] - mid[0], p[1] - mid[1]);
                    [mid[0] + dx + x * co - y * s, mid[1] + dy + x * s + y * co]
                })
                .collect();
            let tint = rgba(0xdfe8ff, 0.16 * fade);
            push_triangle(v, [moved[0], moved[1], moved[2]], tint);
            push_triangle(v, [moved[0], moved[2], moved[3]], tint);
            for e in 0..4 {
                line(v, moved[e], moved[(e + 1) % 4], 0.8, fade);
            }
        }
    }
}
