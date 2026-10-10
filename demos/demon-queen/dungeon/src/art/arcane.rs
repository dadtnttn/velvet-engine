//! Arcane library kit for the hidden chamber (web prototype `arcane.js`): a
//! stepped rune dais under the relic chest, bookshelves filling the arched
//! niches, a violet runner and banners, ivy with violet blooms and a candle
//! chandelier with a crystal.
use super::kit::*;
use super::themekit::candle;
use super::treasury::GOLD_TRIM;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

pub const VIOLET: C3 = [0.3, 0.12, 0.44];
pub const VIOLET_DEEP: C3 = [0.13, 0.05, 0.2];
pub const LAVENDER: C3 = [0.7, 0.55, 0.88];
pub const RUNE: C3 = [0.62, 0.32, 1.0];
/// Dais tiers (radius, top height); the relic chest stands on the top.
pub const ARCANE_DAIS: [(f32, f32); 2] = [(1.75, 0.15), (1.35, 0.3)];
pub const ARCANE_DAIS_TOP: f32 = 0.3;

const IRON: C3 = [0.13, 0.11, 0.11];
const LEATHER: [C3; 6] = [
    [0.42, 0.08, 0.07],
    [0.12, 0.26, 0.16],
    [0.1, 0.14, 0.34],
    [0.36, 0.2, 0.08],
    [0.3, 0.12, 0.4],
    [0.52, 0.4, 0.18],
];
const WOOD: C3 = [0.3, 0.18, 0.1];

/// An arched niche in a wall bay (web `grandNiches`).
#[derive(Clone, Copy, Debug)]
pub struct Niche {
    pub center: Vec3,
    pub inward: Vec3,
    pub tangent: Vec3,
    pub width: f32,
    pub half: f32,
    pub spring: f32,
    pub top: f32,
    pub depth: f32,
}

/// Two stepped stone tiers with glowing rune strokes on the risers.
pub fn arcane_dais(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    facing: Vec3,
    stone: C3,
    rng: &mut ArtRng,
) {
    let mut y = 0.0;
    for (k, &(r, top)) in ARCANE_DAIS.iter().enumerate() {
        lathe(
            m,
            base + Y(y),
            VY,
            &[(0.0, r + 0.02), (top - y - 0.03, r), (top - y, r - 0.03)],
            28,
            tint(stone, 1.02 + k as f32 * 0.06),
            true,
        );
        tube_ring(
            m,
            base + Y(top - 0.012),
            VY,
            r - 0.015,
            0.014,
            36,
            tint(stone, 1.25),
        );
        let strokes = (r * 10.0).floor() as usize;
        for i in 0..strokes {
            let a = i as f32 * TAU / strokes as f32 + k as f32 * 0.2;
            let d = v(a.cos(), 0.0, a.sin());
            let t = v(-d.z, 0.0, d.x);
            let c = base + d * (r + 0.004) + Y(y + (top - y) * 0.5);
            let w = if i % 3 == 0 { 0.05 } else { 0.03 };
            quad(
                glow,
                [
                    c - t * w - Y(0.035),
                    c + t * w - Y(0.035),
                    c + t * w + Y(0.035),
                    c - t * w + Y(0.035),
                ],
                d,
                tint(RUNE, 0.5),
            );
        }
        y = top;
    }
    // Votive candles on the lower tread, leaving the approach clear.
    let outer = ARCANE_DAIS[0].0;
    let (inner, inner_top) = ARCANE_DAIS[1];
    let f = norm(hz(facing));
    let count = 18;
    for i in 0..count {
        let a = i as f32 * TAU / count as f32;
        let d = v(a.cos(), 0.0, a.sin());
        if d.dot(f) > 0.8 || rng.chance(0.25) {
            continue;
        }
        let r = rng.range(0.028, 0.045);
        let h = rng.range(0.08, 0.3);
        candle(
            m,
            glow,
            base + d * ((outer + inner) * 0.5) + Y(ARCANE_DAIS[0].1),
            r,
            h,
            rng,
        );
    }
    for _ in 0..6 {
        let a = rng.range(0.0, TAU);
        let d = v(a.cos(), 0.0, a.sin());
        if d.dot(f) > 0.5 {
            continue;
        }
        let r = rng.range(0.03, 0.045);
        let h = rng.range(0.12, 0.3);
        candle(m, glow, base + d * (inner - 0.18) + Y(inner_top), r, h, rng);
    }
}

/// Violet runner with lavender borders and eye motifs.
pub fn arcane_runner(m: &mut Mesh3D, glow: &mut Mesh3D, from: Vec3, to: Vec3, hw: f32) {
    let axis = norm(hz(to - from));
    let side = v(-axis.z, 0.0, axis.x);
    let len = hz(to - from).length();
    let rect = |target: &mut Mesh3D, y: f32, s0: f32, s1: f32, t0: f32, t1: f32, c: C3| {
        let p = |s: f32, t: f32| from + side * s + axis * t + Y(y);
        quad(target, [p(s0, t0), p(s1, t0), p(s1, t1), p(s0, t1)], VY, c);
    };
    rect(m, 0.01, -hw, hw, 0.0, len, VIOLET_DEEP);
    rect(
        m,
        0.011,
        -hw + 0.1,
        hw - 0.1,
        0.08,
        len - 0.04,
        tint(VIOLET, 0.95),
    );
    for s in [-1.0f32, 1.0] {
        rect(
            m,
            0.012,
            s * (hw - 0.06) - 0.02,
            s * (hw - 0.06) + 0.02,
            0.03,
            len,
            LAVENDER,
        );
        rect(
            m,
            0.012,
            s * (hw - 0.19) - 0.01,
            s * (hw - 0.19) + 0.01,
            0.12,
            len - 0.05,
            tint(LAVENDER, 0.8),
        );
        rect(
            glow,
            0.0125,
            s * (hw - 0.06) - 0.005,
            s * (hw - 0.06) + 0.005,
            0.03,
            len,
            tint(RUNE, 0.25),
        );
    }
    let motifs = ((len - 0.5) / 1.1).floor().max(0.0) as usize;
    for k in 0..motifs {
        let c = from + axis * (0.7 + k as f32 * 1.1) + Y(0.0125);
        quad(
            m,
            [
                c - axis * 0.3,
                c + side * 0.2,
                c + axis * 0.3,
                c - side * 0.2,
            ],
            VY,
            tint(LAVENDER, 0.85),
        );
        quad(
            m,
            [
                c - axis * 0.22 + Y(0.001),
                c + side * 0.14 + Y(0.001),
                c + axis * 0.22 + Y(0.001),
                c - side * 0.14 + Y(0.001),
            ],
            VY,
            VIOLET_DEEP,
        );
        disc(m, c + Y(0.002), VY, 0.06, 10, LAVENDER);
        disc(m, c + Y(0.003), VY, 0.028, 8, VIOLET_DEEP);
    }
    let tassels = (hw * 2.0 / 0.07).floor() as usize;
    for k in 0..tassels {
        let s = -hw + 0.035 + k as f32 * 0.07;
        rect(m, 0.0105, s - 0.016, s + 0.016, -0.1, 0.0, LAVENDER);
    }
}

/// Long violet banner with an eye sigil and a swallowtail hem.
#[allow(clippy::too_many_arguments)]
pub fn arcane_banner(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    top: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    length: f32,
) {
    let off = inward * 0.09;
    let rod = top + off + inward * 0.02;
    bar(
        m,
        rod - tangent * (width * 0.5 + 0.08),
        rod + tangent * (width * 0.5 + 0.08),
        0.018,
        0.018,
        VY,
        GOLD_TRIM,
    );
    for s in [-1.0f32, 1.0] {
        bar(
            m,
            rod + tangent * (s * width * 0.4),
            top + tangent * (s * width * 0.4) - inward * 0.05,
            0.012,
            0.012,
            VY,
            IRON,
        );
    }
    let hw = width * 0.5;
    let p = |x: f32, y: f32| top + off + tangent * x - Y(y);
    for k in 0..6 {
        let x0 = -hw + width * k as f32 / 6.0;
        let x1 = -hw + width * (k + 1) as f32 / 6.0;
        let fold = |i: usize| if i % 2 == 0 { 0.0 } else { 0.02 };
        let a = |y: f32| p(x0, y) + inward * fold(k);
        let b = |y: f32| p(x1, y) + inward * fold(k + 1);
        let mut n = norm((b(0.5) - a(0.5)).cross(-VY));
        if n.dot(inward) < 0.0 {
            n = -n;
        }
        let edge = k == 0 || k == 5;
        quad(
            m,
            [a(0.03), b(0.03), b(length), a(length)],
            n,
            tint(
                if edge { VIOLET_DEEP } else { VIOLET },
                if k % 2 == 0 { 0.88 } else { 1.05 },
            ),
        );
        quad(
            m,
            [a(0.03), b(0.03), b(length), a(length)],
            -n,
            tint(VIOLET, 0.45),
        );
    }
    for (n, color) in [(inward, VIOLET), (-inward, tint(VIOLET, 0.45))] {
        tri(
            m,
            p(-hw, length),
            p(0.0, length),
            p(-hw * 0.5, length + 0.32),
            n,
            color,
        );
        tri(
            m,
            p(0.0, length),
            p(hw, length),
            p(hw * 0.5, length + 0.32),
            n,
            color,
        );
    }
    let front = |x: f32, y: f32, d: f32| p(x, y) + inward * d;
    for s in [-1.0f32, 1.0] {
        let x = s * (hw - 0.07);
        quad(
            m,
            [
                front(x - 0.018, 0.08, 0.028),
                front(x + 0.018, 0.08, 0.028),
                front(x + 0.018, length - 0.02, 0.028),
                front(x - 0.018, length - 0.02, 0.028),
            ],
            inward,
            LAVENDER,
        );
    }
    // Eye sigil: lozenge frame, iris and pupil, rays below.
    let e = (length * 0.38).min(0.8);
    let c = front(0.0, e, 0.03);
    let r = (width * 0.3).min(0.22);
    let up = VY;
    quad(
        m,
        [
            c - up * (r * 1.3),
            c + tangent * r,
            c + up * (r * 1.3),
            c - tangent * r,
        ],
        inward,
        LAVENDER,
    );
    let i2 = inward * 0.002;
    quad(
        m,
        [
            c - up * (r * 1.05) + i2,
            c + tangent * (r * 0.8) + i2,
            c + up * (r * 1.05) + i2,
            c - tangent * (r * 0.8) + i2,
        ],
        inward,
        VIOLET_DEEP,
    );
    disc(m, c + inward * 0.004, inward, r * 0.42, 14, LAVENDER);
    disc(
        glow,
        c + inward * 0.006,
        inward,
        r * 0.2,
        10,
        tint(RUNE, 0.55),
    );
    for x in [-0.07f32, 0.0, 0.07] {
        quad(
            m,
            [
                front(x - 0.01, e + r * 1.5, 0.028),
                front(x + 0.01, e + r * 1.5, 0.028),
                front(x + 0.01, length - 0.12, 0.028),
                front(x - 0.01, length - 0.12, 0.028),
            ],
            inward,
            tint(LAVENDER, 0.85),
        );
    }
}

/// Book of random size and binding standing (or leaning) on a shelf.
#[allow(clippy::too_many_arguments)]
fn book(
    m: &mut Mesh3D,
    base: Vec3,
    along: Vec3,
    into: Vec3,
    width: f32,
    height: f32,
    depth: f32,
    color: C3,
    lean: f32,
) {
    let up = norm(VY * lean.cos() + along * lean.sin());
    let x = norm(along * lean.cos() - VY * lean.sin());
    let c = base + up * (height * 0.5) + x * (width * 0.5);
    kit_block(
        m,
        c,
        x,
        up,
        into,
        v(width * 0.5, height * 0.5, depth * 0.5),
        color,
    );
    kit_block(
        m,
        c - into * (depth * 0.5 + 0.002) + up * (height * 0.28),
        x,
        up,
        into,
        v(width * 0.5 - 0.004, 0.012, 0.003),
        GOLD_TRIM,
    );
}

/// Wooden shelves inside an arched niche, loaded with books and curios.
pub fn bookshelf(m: &mut Mesh3D, glow: &mut Mesh3D, niche: &Niche, rng: &mut ArtRng) {
    let (half, spring, depth) = (niche.half, niche.spring, niche.depth);
    let t = niche.tangent;
    let n = niche.inward;
    let c = niche.center;
    let back = depth - 0.06;
    let levels: Vec<f32> = [0.5, 1.12, 1.74, 2.36]
        .into_iter()
        .filter(|&y| y < spring - 0.35)
        .collect();
    let last = *levels.last().unwrap_or(&0.5);
    for s in [-1.0f32, 1.0] {
        chamfer_box(
            m,
            c + t * (s * (half - 0.04)) - n * (back * 0.5) + Y((last + 0.1) * 0.5),
            t,
            VY,
            n,
            v(0.03, (last + 0.1) * 0.5, back * 0.5),
            0.008,
            tint(WOOD, 0.8),
        );
    }
    for &y in &levels {
        let shade = rng.range(0.9, 1.1);
        chamfer_box(
            m,
            c - n * (back * 0.5) + Y(y),
            t,
            VY,
            n,
            v(half - 0.02, 0.025, back * 0.5),
            0.008,
            tint(WOOD, shade),
        );
        let mut x = -half + 0.08;
        let shelf_top = y + 0.025;
        while x < half - 0.1 {
            let roll = rng.unit();
            let at = c + t * x - n * rng.range(0.1, 0.18);
            if roll < 0.62 {
                let width = rng.range(0.035, 0.075);
                let height = rng.range(0.2, 0.36);
                let lean = if x > half - 0.3 && rng.chance(0.5) {
                    -0.35
                } else if rng.chance(0.08) {
                    0.3
                } else {
                    0.0
                };
                let bd = rng.range(0.15, 0.22);
                let color = LEATHER[rng.index(LEATHER.len())];
                let shade = rng.range(0.8, 1.15);
                book(
                    m,
                    at + Y(shelf_top),
                    t,
                    -n,
                    width,
                    height,
                    bd,
                    tint(color, shade),
                    lean,
                );
                x += width + 0.004 + if lean != 0.0 { 0.08 } else { 0.0 };
            } else if roll < 0.72 {
                // A stack of books lying flat.
                let mut h = shelf_top;
                let mut k = 0;
                loop {
                    let count = 2 + rng.index(3);
                    if k >= count {
                        break;
                    }
                    let bh = rng.range(0.035, 0.05);
                    let bw = rng.range(0.1, 0.13);
                    let bd = rng.range(0.08, 0.1);
                    let color = LEATHER[rng.index(LEATHER.len())];
                    let shade = rng.range(0.8, 1.1);
                    kit_block(
                        m,
                        at + t * 0.12 + Y(h + bh * 0.5),
                        t,
                        VY,
                        n,
                        v(bw, bh * 0.5, bd),
                        tint(color, shade),
                    );
                    h += bh;
                    k += 1;
                }
                x += 0.28;
            } else if roll < 0.84 {
                // A potion: glass flask with a glowing draught.
                let liquid = [
                    [0.55, 0.2, 1.0],
                    [0.2, 0.9, 0.5],
                    [1.0, 0.3, 0.25],
                    [0.3, 0.6, 1.0],
                ][rng.index(4)];
                let h = rng.range(0.14, 0.22);
                lathe(
                    m,
                    at + t * 0.06 + Y(shelf_top),
                    VY,
                    &[(0.0, 0.04), (h * 0.45, 0.055), (h * 0.75, 0.02), (h, 0.018)],
                    8,
                    [0.5, 0.55, 0.6],
                    false,
                );
                lathe(
                    glow,
                    at + t * 0.06 + Y(shelf_top + 0.005),
                    VY,
                    &[(0.0, 0.035), (h * 0.4, 0.048), (h * 0.45, 0.0)],
                    8,
                    tint(liquid, 0.55),
                    false,
                );
                kit_block(
                    m,
                    at + t * 0.06 + Y(shelf_top + h + 0.015),
                    t,
                    VY,
                    n,
                    v(0.015, 0.018, 0.015),
                    [0.4, 0.26, 0.12],
                );
                x += 0.14;
            } else if roll < 0.92 {
                let r = rng.range(0.025, 0.04);
                let h = rng.range(0.08, 0.2);
                candle(m, glow, at + t * 0.05 + Y(shelf_top), r, h, rng);
                x += 0.12;
            } else {
                // Brass jar.
                lathe(
                    m,
                    at + t * 0.07 + Y(shelf_top),
                    VY,
                    &[
                        (0.0, 0.05),
                        (0.06, 0.07),
                        (0.13, 0.06),
                        (0.16, 0.035),
                        (0.18, 0.04),
                    ],
                    10,
                    [0.55, 0.38, 0.14],
                    true,
                );
                x += 0.16;
            }
        }
    }
    // Candles and wax on the top of the case.
    for _ in 0..3 {
        let along = rng.range(-half + 0.15, half - 0.15);
        let r = rng.range(0.03, 0.045);
        let h = rng.range(0.1, 0.22);
        candle(
            m,
            glow,
            c + t * along - n * 0.15 + Y(last + 0.025) + Y(0.4),
            r,
            h,
            rng,
        );
    }
}

/// Ivy draped from a ledge, with small violet blooms.
pub fn ivy(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    from: Vec3,
    to: Vec3,
    inward: Vec3,
    drop: f32,
    rng: &mut ArtRng,
) {
    let along = to - from;
    let length = along.length();
    let dir = norm(along);
    let strands = ((length / 0.15).floor() as usize).max(4);
    for k in 0..strands {
        if rng.chance(0.12) {
            continue;
        }
        let root = from
            + dir * ((k as f32 + rng.range(0.2, 0.8)) * length / strands as f32)
            + inward * 0.05;
        let len = drop * rng.range(0.25, 1.1);
        let leaves = ((len / 0.075).floor() as usize).max(3);
        for i in 0..leaves {
            let f = i as f32 / leaves as f32;
            let at = root - Y(f * len)
                + dir * ((f * 7.0 + k as f32).sin() * 0.05)
                + inward * (0.02 + f * 0.05);
            let size = rng.range(0.06, 0.1) * (1.0 - f * 0.35);
            let tilt = dir * if i % 2 == 1 { 1.0 } else { -1.0 };
            let g0 = 0.1 + rng.range(-0.02, 0.04);
            let g1 = 0.26 + rng.range(-0.05, 0.06);
            let green = [g0, g1, 0.1];
            let a = at;
            let b = at + tilt * size - Y(size * 0.4);
            let c = at + tilt * (size * 0.4) - Y(size * 1.2);
            let shade = rng.range(0.8, 1.2);
            tri(m, a, b, c, inward, tint(green, shade));
            tri(m, a, c, b, -inward, tint(green, 0.6));
            if rng.chance(0.3) {
                let bloom = at + inward * 0.035 - Y(size * 0.5);
                let shade = rng.range(0.85, 1.15);
                disc(m, bloom, inward, 0.04, 5, tint(LAVENDER, shade));
                disc(
                    glow,
                    bloom + inward * 0.004,
                    inward,
                    0.016,
                    5,
                    tint([0.8, 0.4, 1.0], 0.5),
                );
            }
        }
    }
}

/// Iron ring chandelier with candles and a violet crystal pendant.
pub fn crystal_chandelier(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    ceiling: Vec3,
    ring_y: f32,
    radius: f32,
    rng: &mut ArtRng,
) {
    let x0 = with_y(ceiling, 0.0);
    let at = |y: f32| x0 + Y(y);
    let mut y = ceiling.y + 0.2;
    let mut alt = false;
    while y > ring_y + 0.75 {
        bar(
            m,
            at(y),
            at(y - 0.1),
            0.026,
            0.009,
            if alt { VX } else { VZ },
            tint(IRON, 1.3),
        );
        y -= 0.085;
        alt = !alt;
    }
    let hub = at(ring_y + 0.7);
    lathe(
        m,
        hub - Y(0.08),
        VY,
        &[
            (0.0, 0.0),
            (0.04, 0.08),
            (0.1, 0.1),
            (0.16, 0.05),
            (0.2, 0.0),
        ],
        10,
        GOLD_TRIM,
        false,
    );
    for k in 0..6 {
        let a = k as f32 * TAU / 6.0 + 0.3;
        bar(
            m,
            hub,
            at(ring_y + 0.04) + v(a.cos(), 0.0, a.sin()) * (radius - 0.02),
            0.009,
            0.009,
            VY,
            IRON,
        );
    }
    let c = at(ring_y);
    tube_ring(m, c, VY, radius, 0.045, 32, tint(IRON, 1.5));
    tube_ring(m, c + Y(0.05), VY, radius, 0.018, 32, GOLD_TRIM);
    for k in 0..10 {
        let a = k as f32 * TAU / 10.0;
        let p = c + v(a.cos(), 0.0, a.sin()) * radius + Y(0.06);
        lathe(
            m,
            p - Y(0.01),
            VY,
            &[(0.0, 0.022), (0.022, 0.045), (0.03, 0.05)],
            8,
            GOLD_TRIM,
            true,
        );
        let h = rng.range(0.12, 0.2);
        candle(m, glow, p + Y(0.02), 0.028, h, rng);
    }
    // The crystal pendant hangs point-down beneath the ring.
    crystal(glow, c - Y(0.05), 0.16, -0.62, tint(RUNE, 0.6), 6);
    crystal(glow, c - Y(0.02), 0.1, 0.22, tint(RUNE, 0.5), 5);
    for k in 0..5 {
        let a = k as f32 * TAU / 5.0;
        crystal(
            glow,
            c + v(a.cos() * 0.2, -0.04, a.sin() * 0.2),
            0.05,
            -0.26,
            tint(RUNE, 0.45),
            5,
        );
    }
    annulus(
        glow,
        c - Y(0.045),
        VY,
        radius - 0.01,
        radius + 0.01,
        32,
        tint(RUNE, 0.2),
        0.0,
    );
}
