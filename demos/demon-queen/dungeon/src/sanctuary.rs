//! Sanctuary art kit (Role::Rest): a stepped dais against the shrine wall
//! with a hooded guardian statue holding a sword before a sun halo, rows of
//! votive candles on every step, crystal braziers whose crystals burn in the
//! biome's glow colour, prayer rugs, ferns and carved sun reliefs. Built into
//! the same two static meshes as the treasury (detail + emissive glow).
use super::treasury::{
    annulus, bar, block, candle, disc, lathe, star, tube_ring, DEEP_RED, GOLD_TRIM, RED,
};
use super::visual::{crystal, quad, tint, tri};
use std::f32::consts::TAU;
use velvet_dungeon::{horizontal, rng::Rng};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const MARBLE: [f32; 3] = [0.5, 0.45, 0.39];
const BRONZE: [f32; 3] = [0.34, 0.2, 0.09];

/// Dais tiers: (depth from the wall, width inset, top height).
pub const DAIS: [(f32, f32, f32); 3] = [(1.55, 0.0, 0.15), (1.2, 0.5, 0.3), (0.88, 1.0, 0.45)];

/// Where the statue stands, relative to the wall midpoint.
pub fn statue_base(wall: Vec3, inward: Vec3) -> Vec3 {
    wall + inward * 0.5 + Vec3::Y * DAIS[2].2
}

/// Stepped dais with gilded risers and a red runner, then the statue.
pub(super) fn shrine(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    wall: Vec3,
    inward: Vec3,
    width: f32,
    stone: [f32; 3],
    glow_color: [f32; 3],
) {
    let f = horizontal(inward).normalize_or_zero();
    let s = Vec3::new(f.z, 0.0, -f.x);
    let dais_stone = tint(stone, 1.08);
    for (k, &(depth, inset, top)) in DAIS.iter().enumerate() {
        let hw = (width - inset) * 0.5;
        // Solid tier from the floor to its tread, pushed 0.1 m into the wall.
        block(
            m,
            wall + f * ((depth - 0.1) * 0.5) + Vec3::Y * (top * 0.5),
            s,
            Vec3::Y,
            f,
            Vec3::new(hw, top * 0.5, (depth + 0.1) * 0.5),
            tint(dais_stone, 0.92 + k as f32 * 0.05),
        );
        // Bullnosed tread edge and a gilded inlay on the riser.
        block(
            m,
            wall + f * (depth - 0.02) + Vec3::Y * (top - 0.02),
            s,
            Vec3::Y,
            f,
            Vec3::new(hw + 0.02, 0.025, 0.04),
            tint(dais_stone, 1.18),
        );
        bar(
            m,
            wall + f * (depth + 0.003) + Vec3::Y * (top - 0.075) - s * (hw - 0.08),
            wall + f * (depth + 0.003) + Vec3::Y * (top - 0.075) + s * (hw - 0.08),
            0.012,
            0.004,
            Vec3::Y,
            GOLD_TRIM,
        );
        // Red runner over the tread and down the riser, gold-edged.
        let next = if k + 1 < DAIS.len() {
            DAIS[k + 1].0
        } else {
            0.2
        };
        let rw = 0.46;
        let t0 = wall + f * depth + Vec3::Y * (top + 0.004);
        let t1 = wall + f * next + Vec3::Y * (top + 0.004);
        quad(
            m,
            [t0 - s * rw, t0 + s * rw, t1 + s * rw, t1 - s * rw],
            Vec3::Y,
            RED,
        );
        let r0 = wall + f * (depth + 0.006) + Vec3::Y * top;
        let r1 = r0 - Vec3::Y * 0.15;
        quad(
            m,
            [r0 - s * rw, r0 + s * rw, r1 + s * rw, r1 - s * rw],
            f,
            tint(RED, 0.85),
        );
        for side in [-1.0f32, 1.0] {
            bar(
                m,
                t0 + s * (side * (rw - 0.03)),
                t1 + s * (side * (rw - 0.03)),
                0.014,
                0.003,
                Vec3::Y,
                GOLD_TRIM,
            );
        }
    }
    // The guardian is built at human scale, then enlarged about its base.
    let base = statue_base(wall, f);
    let (mut body, mut eyes) = (super::visual::empty(), super::visual::empty());
    statue(&mut body, &mut eyes, base, f, glow_color);
    merge_scaled(m, &body, base, STATUE_SCALE);
    merge_scaled(glow, &eyes, base, STATUE_SCALE);
}

/// Enlargement of the guardian statue about its base.
pub const STATUE_SCALE: f32 = 1.28;

fn merge_scaled(into: &mut Mesh3D, from: &Mesh3D, origin: Vec3, k: f32) {
    let offset = into.vertices.len() as u32;
    for v in &from.vertices {
        let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
        let q = origin + (p - origin) * k;
        let mut out = *v;
        out.position = q.to_array();
        into.vertices.push(out);
    }
    into.indices.extend(from.indices.iter().map(|i| i + offset));
}

/// Hooded guardian: bell robe with folds, mantle, deep hood with glowing
/// eyes, arms resting on a sword planted before it, and a rayed halo.
fn statue(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, f: Vec3, glow_color: [f32; 3]) {
    let s = Vec3::new(f.z, 0.0, -f.x);
    // Plinth.
    block(
        m,
        base + Vec3::Y * 0.1,
        s,
        Vec3::Y,
        f,
        Vec3::new(0.46, 0.1, 0.38),
        tint(MARBLE, 0.8),
    );
    block(
        m,
        base + Vec3::Y * 0.215,
        s,
        Vec3::Y,
        f,
        Vec3::new(0.5, 0.015, 0.42),
        tint(MARBLE, 0.95),
    );
    let o = base + Vec3::Y * 0.23;
    let robe = |j: usize| {
        tint(
            MARBLE,
            [0.86, 0.9, 0.94, 0.98, 1.0, 1.04, 1.02, 0.98, 0.9][j.min(8)],
        )
    };
    lathe(
        m,
        o,
        Vec3::Y,
        &[
            (0.0, 0.4),
            (0.08, 0.42),
            (0.45, 0.36),
            (0.95, 0.3),
            (1.28, 0.27),
            (1.42, 0.33),
            (1.55, 0.33),
            (1.64, 0.22),
            (1.7, 0.12),
            (1.72, 0.09),
        ],
        14,
        &robe,
        true,
    );
    // Vertical robe folds on the front.
    for (k, x) in [-0.22f32, -0.1, 0.03, 0.16].into_iter().enumerate() {
        let r = 0.37 - k as f32 * 0.005;
        let top = 1.0 - (x.abs() * 0.6);
        let a = o + s * x + f * (r - 0.01) + Vec3::Y * 0.06;
        let b = o + s * (x * 0.8) + f * 0.27 + Vec3::Y * top;
        bar(m, a, b, 0.022, 0.018, s, tint(MARBLE, 0.82));
    }
    // Hood and the shadowed face.
    let h = o + Y(1.62);
    lathe(
        m,
        h,
        Vec3::Y,
        &[
            (0.0, 0.14),
            (0.07, 0.2),
            (0.22, 0.225),
            (0.34, 0.19),
            (0.45, 0.11),
            (0.52, 0.02),
        ],
        14,
        &|_| tint(MARBLE, 0.96),
        true,
    );
    // Shadowed hood opening on the front surface, a pale mask inside it
    // and two ember eyes.
    let face = h + Y(0.21) + f * 0.228;
    disc(m, face, f, 0.135, 16, [0.05, 0.035, 0.03]);
    disc(
        m,
        face - Y(0.02) + f * 0.006,
        f,
        0.078,
        12,
        tint(MARBLE, 0.72),
    );
    block(
        m,
        face - Y(0.07) + f * 0.008,
        s,
        Vec3::Y,
        f,
        Vec3::new(0.035, 0.03, 0.006),
        tint(MARBLE, 0.5),
    );
    for side in [-1.0f32, 1.0] {
        disc(
            glow,
            face + Y(0.0) + s * (side * 0.032) + f * 0.012,
            f,
            0.016,
            8,
            tint(glow_color, 0.8),
        );
    }
    // Arms from the shoulders down to hands folded on the pommel.
    let hands = o + Y(1.1) + f * 0.36;
    for side in [-1.0f32, 1.0] {
        let shoulder = o + Y(1.5) + s * (side * 0.3) + f * 0.04;
        let elbow = o + Y(1.2) + s * (side * 0.28) + f * 0.2;
        let wrist = hands + s * (side * 0.07);
        bar(m, shoulder, elbow, 0.075, 0.075, f, tint(MARBLE, 0.93));
        bar(m, elbow, wrist, 0.065, 0.065, f, tint(MARBLE, 0.97));
        block(
            m,
            wrist + Y(0.01),
            s,
            Vec3::Y,
            f,
            Vec3::new(0.05, 0.045, 0.05),
            tint(MARBLE, 1.02),
        );
    }
    // The sword: blade planted on the plinth, guard and grip at the hands.
    let steel = [0.72, 0.7, 0.68];
    let tip = o + Y(0.02) + f * 0.38;
    let guard = hands - Y(0.1);
    bar(m, tip + Y(0.08), guard, 0.045, 0.01, s, steel);
    tri(
        m,
        tip + Y(0.08) - s * 0.045,
        tip + Y(0.08) + s * 0.045,
        tip,
        f,
        steel,
    );
    tri(
        m,
        tip + Y(0.08) - s * 0.045,
        tip + Y(0.08) + s * 0.045,
        tip,
        -f,
        tint(steel, 0.8),
    );
    bar(
        m,
        guard - s * 0.2,
        guard + s * 0.2,
        0.025,
        0.03,
        Vec3::Y,
        GOLD_TRIM,
    );
    bar(m, guard, guard + Y(0.2), 0.022, 0.022, s, [0.3, 0.08, 0.06]);
    lathe(
        m,
        guard + Y(0.2),
        Vec3::Y,
        &[(0.0, 0.02), (0.03, 0.04), (0.07, 0.0)],
        8,
        &|_| GOLD_TRIM,
        false,
    );
    // Rayed halo behind the head.
    let halo = h + Y(0.28) - f * 0.14;
    tube_ring(m, halo, f, 0.44, 0.024, 36, GOLD_TRIM);
    tube_ring(m, halo, f, 0.3, 0.012, 28, tint(GOLD_TRIM, 0.85));
    for k in 0..16 {
        let a = k as f32 * TAU / 16.0;
        let d = s * a.cos() + Vec3::Y * a.sin();
        let len = if k % 2 == 0 { 0.2 } else { 0.11 };
        let a0 = halo + d * 0.47;
        let a1 = halo + d * (0.47 + len);
        let side = f.cross(d).normalize_or_zero();
        tri(m, a0 - side * 0.03, a0 + side * 0.03, a1, f, GOLD_TRIM);
        tri(
            m,
            a0 - side * 0.03,
            a0 + side * 0.03,
            a1,
            -f,
            tint(GOLD_TRIM, 0.7),
        );
    }
    annulus(
        glow,
        halo + f * 0.03,
        f,
        0.41,
        0.435,
        40,
        tint(glow_color, 0.35),
        0.0,
    );
}

#[allow(non_snake_case)]
fn Y(v: f32) -> Vec3 {
    Vec3::Y * v
}

/// Stone pedestal, bronze bowl and a cluster of burning crystals.
pub(super) fn crystal_brazier(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    k: f32,
    stone: [f32; 3],
    glow_color: [f32; 3],
    rng: &mut Rng,
) {
    lathe(
        m,
        base,
        Vec3::Y,
        &[
            (0.0, 0.24 * k),
            (0.07 * k, 0.24 * k),
            (0.12 * k, 0.15 * k),
            (0.62 * k, 0.11 * k),
            (0.68 * k, 0.17 * k),
            (0.74 * k, 0.2 * k),
        ],
        12,
        &|j| tint(stone, [0.7, 0.85, 0.95, 1.0, 1.1][j.min(4)]),
        true,
    );
    let bowl = base + Y(0.74 * k);
    lathe(
        m,
        bowl,
        Vec3::Y,
        &[
            (0.0, 0.14 * k),
            (0.07 * k, 0.3 * k),
            (0.15 * k, 0.37 * k),
            (0.18 * k, 0.35 * k),
        ],
        16,
        &|_| BRONZE,
        true,
    );
    tube_ring(
        m,
        bowl + Y(0.17 * k),
        Vec3::Y,
        0.36 * k,
        0.018 * k,
        24,
        tint(GOLD_TRIM, 0.8),
    );
    disc(
        glow,
        bowl + Y(0.182 * k),
        Vec3::Y,
        0.3 * k,
        16,
        tint(glow_color, 0.12),
    );
    let count = 6;
    for i in 0..count {
        let a = i as f32 * TAU / count as f32 + rng.range(-0.3, 0.3);
        let r = if i == 0 { 0.0 } else { rng.range(0.1, 0.2) * k };
        let p = bowl + Y(0.16 * k) + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        let height = if i == 0 {
            0.62 * k
        } else {
            rng.range(0.24, 0.42) * k
        };
        let radius = if i == 0 {
            0.09 * k
        } else {
            rng.range(0.05, 0.075) * k
        };
        crystal(
            glow,
            p,
            radius,
            height,
            tint(glow_color, rng.range(0.5, 0.62)),
            5 + i % 2,
        );
    }
}

/// Low fern: arching fronds, two-sided, in shaded greens.
pub(super) fn fern(m: &mut Mesh3D, base: Vec3, k: f32, rng: &mut Rng) {
    let fronds = 9;
    for i in 0..fronds {
        let a = i as f32 * TAU / fronds as f32 + rng.range(-0.25, 0.25);
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let side = Vec3::new(-d.z, 0.0, d.x);
        let len = rng.range(0.38, 0.55) * k;
        let lift = rng.range(0.28, 0.4) * k;
        let pts = [
            base,
            base + d * (len * 0.35) + Y(lift * 0.85),
            base + d * (len * 0.7) + Y(lift),
            base + d * len + Y(lift * 0.6),
        ];
        let widths = [0.012 * k, 0.05 * k, 0.04 * k, 0.0];
        let green = [
            0.16 + rng.range(-0.03, 0.04),
            0.36 + rng.range(-0.05, 0.06),
            0.12,
        ];
        for j in 0..3 {
            let (p0, p1) = (pts[j], pts[j + 1]);
            let (w0, w1) = (widths[j], widths[j + 1]);
            let n = (p1 - p0).cross(side).normalize_or_zero();
            let n = if n.y < 0.0 { -n } else { n };
            let c = tint(green, 0.8 + j as f32 * 0.12);
            quad(
                m,
                [
                    p0 - side * w0,
                    p0 + side * w0,
                    p1 + side * w1,
                    p1 - side * w1,
                ],
                n,
                c,
            );
            quad(
                m,
                [
                    p0 - side * w0,
                    p0 + side * w0,
                    p1 + side * w1,
                    p1 - side * w1,
                ],
                -n,
                tint(c, 0.7),
            );
        }
    }
}

/// Carved sun: stone disc, ring, alternating rays and a gilded eye.
pub(super) fn sun_relief(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    r: f32,
    stone: [f32; 3],
    glow_color: [f32; 3],
) {
    let up = Vec3::Y;
    let side = up.cross(n).normalize_or_zero();
    // Back plate recessed into the wall, then the relief proud of it.
    disc(m, c, n, r * 0.62, 24, tint(stone, 1.12));
    annulus(
        m,
        c + n * 0.012,
        n,
        r * 0.5,
        r * 0.6,
        32,
        tint(stone, 1.3),
        0.0,
    );
    for k in 0..12 {
        let a = k as f32 * TAU / 12.0;
        let d = side * a.cos() + up * a.sin();
        let t = n.cross(d).normalize_or_zero();
        let len = if k % 2 == 0 { r } else { r * 0.82 };
        let a0 = c + d * (r * 0.62) + n * 0.01;
        tri(
            m,
            a0 - t * (r * 0.1),
            a0 + t * (r * 0.1),
            c + d * len + n * 0.01,
            n,
            tint(stone, 1.2),
        );
    }
    annulus(m, c + n * 0.02, n, r * 0.2, r * 0.26, 24, GOLD_TRIM, 0.0);
    // Almond eye.
    let eye: Vec<Vec3> = (0..=12)
        .map(|k| {
            let a = k as f32 * TAU / 12.0;
            c + n * 0.024 + side * (a.cos() * r * 0.18) + up * (a.sin() * r * 0.09)
        })
        .collect();
    for k in 0..12 {
        tri(m, c + n * 0.024, eye[k], eye[k + 1], n, GOLD_TRIM);
    }
    disc(glow, c + n * 0.03, n, r * 0.055, 10, tint(glow_color, 0.6));
}

/// Rectangular prayer rug with a sun medallion and fringed ends.
pub(super) fn prayer_rug(m: &mut Mesh3D, glow: &mut Mesh3D, c: Vec3, along: Vec3, half: [f32; 2]) {
    let a = horizontal(along).normalize_or_zero();
    let s = Vec3::new(a.z, 0.0, -a.x);
    let rect = |m: &mut Mesh3D, y: f32, ha: f32, hs: f32, col: [f32; 3]| {
        let p = |u: f32, v: f32| c + a * u + s * v + Y(y);
        quad(
            m,
            [p(-ha, -hs), p(ha, -hs), p(ha, hs), p(-ha, hs)],
            Vec3::Y,
            col,
        );
    };
    rect(m, 0.01, half[0], half[1], DEEP_RED);
    rect(m, 0.011, half[0] - 0.1, half[1] - 0.1, GOLD_TRIM);
    rect(m, 0.012, half[0] - 0.14, half[1] - 0.14, tint(RED, 0.9));
    rect(m, 0.013, half[0] - 0.3, half[1] - 0.3, tint(RED, 0.78));
    rect(m, 0.014, half[0] - 0.34, half[1] - 0.34, tint(RED, 0.9));
    let r = half[0].min(half[1]) * 0.55;
    annulus(
        m,
        c + Y(0.016),
        Vec3::Y,
        r * 0.82,
        r * 0.9,
        36,
        GOLD_TRIM,
        0.0,
    );
    star(
        m,
        c + Y(0.017),
        Vec3::Y,
        a,
        r * 0.78,
        r * 0.34,
        12,
        GOLD_TRIM,
    );
    disc(m, c + Y(0.018), Vec3::Y, r * 0.3, 20, DEEP_RED);
    annulus(
        glow,
        c + Y(0.019),
        Vec3::Y,
        r * 0.855,
        r * 0.865,
        48,
        [0.3, 0.16, 0.03],
        0.0,
    );
    for (u, v) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let p = c + a * (u * (half[0] - 0.22)) + s * (v * (half[1] - 0.22)) + Y(0.016);
        quad(
            m,
            [p - a * 0.07, p + s * 0.07, p + a * 0.07, p - s * 0.07],
            Vec3::Y,
            GOLD_TRIM,
        );
    }
    for end in [-1.0f32, 1.0] {
        let count = (half[1] * 2.0 / 0.07) as usize;
        for k in 0..count {
            let v = -half[1] + 0.035 + k as f32 * 0.07;
            let p0 = c + a * (end * half[0]) + s * v + Y(0.0105);
            let p1 = p0 + a * (end * 0.09);
            quad(
                m,
                [
                    p0 - s * 0.018,
                    p1 - s * 0.018,
                    p1 + s * 0.018,
                    p0 + s * 0.018,
                ],
                Vec3::Y,
                [0.92, 0.72, 0.36],
            );
        }
    }
}

/// Votive candles along a tread edge, optionally leaving the centre free.
pub(super) fn candle_row(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    from: Vec3,
    to: Vec3,
    count: u8,
    gap: bool,
    rng: &mut Rng,
) {
    let across = horizontal(to - from).normalize_or_zero();
    let depth = Vec3::new(-across.z, 0.0, across.x);
    for i in 0..count {
        let t = (i as f32 + 0.5) / count as f32;
        if gap && (t - 0.5).abs() < 0.2 {
            continue;
        }
        if rng.chance(0.18) {
            continue;
        }
        let p = from.lerp(to, t) + across * rng.range(-0.03, 0.03) + depth * rng.range(-0.05, 0.05);
        let radius = rng.range(0.028, 0.048);
        let height = rng.range(0.08, 0.3);
        candle(m, glow, p, radius, height, rng);
    }
}

/// Clay pot with a rolled rim.
pub(super) fn clay_pot(m: &mut Mesh3D, base: Vec3, h: f32) {
    let clay = [0.46, 0.24, 0.13];
    lathe(
        m,
        base,
        Vec3::Y,
        &[
            (0.0, 0.1 * h),
            (0.05 * h, 0.2 * h),
            (0.28 * h, 0.27 * h),
            (0.55 * h, 0.22 * h),
            (0.72 * h, 0.12 * h),
            (0.8 * h, 0.13 * h),
            (0.84 * h, 0.1 * h),
        ],
        14,
        &|j| tint(clay, [0.75, 0.95, 1.05, 1.0, 0.9, 1.1][j.min(5)]),
        true,
    );
    tube_ring(
        m,
        base + Y(0.28 * h),
        Vec3::Y,
        0.27 * h,
        0.01,
        20,
        tint(clay, 0.6),
    );
}
