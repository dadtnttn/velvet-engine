//! Treasury art kit (web prototype `treasury.js`): carpet and round rug,
//! coin-covered hoards, altars with relics, painted urns, candelabra, the
//! red-and-gold chandelier, heraldic banners, swords stuck in the gold, the
//! loot hall's coffers, coin towers, amber crystals, iron braziers and flame
//! banners, and the grand hall's crown block, gem ledge and royal crest.
use super::kit::*;
use super::themekit::candle;
use std::cell::RefCell;
use std::f32::consts::{FRAC_PI_2, PI};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

pub const RED: C3 = [0.48, 0.05, 0.042];
pub const DEEP_RED: C3 = [0.22, 0.028, 0.024];
pub const GOLD_TRIM: C3 = [0.9, 0.58, 0.16];
pub const DARK_GOLD: C3 = [0.58, 0.33, 0.06];
const IRON: C3 = [0.13, 0.11, 0.1];
const STEEL: C3 = [0.74, 0.77, 0.84];
const NAVY: C3 = [0.07, 0.09, 0.17];
const GEMS: [C3; 4] = [
    [0.9, 0.06, 0.12],
    [0.08, 0.32, 1.0],
    [0.1, 0.8, 0.28],
    [0.7, 0.2, 1.0],
];

fn lerp3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

pub fn carpet(m: &mut Mesh3D, glow: &mut Mesh3D, from: Vec3, to: Vec3, hw: f32) {
    let axis = norm(hz(to - from));
    let side = v(-axis.z, 0.0, axis.x);
    let len = hz(to - from).length();
    let rect = |target: &mut Mesh3D, y: f32, s0: f32, s1: f32, t0: f32, t1: f32, c: C3| {
        let p = |s: f32, t: f32| from + side * s + axis * t + Y(y);
        quad(target, [p(s0, t0), p(s1, t0), p(s1, t1), p(s0, t1)], VY, c);
    };
    rect(m, 0.01, -hw, hw, 0.0, len, DEEP_RED);
    rect(
        m,
        0.011,
        -hw + 0.11,
        hw - 0.11,
        0.1,
        len - 0.05,
        tint(RED, 0.86),
    );
    for s in [-1.0f32, 1.0] {
        let outer = s * (hw - 0.065);
        rect(m, 0.012, outer - 0.022, outer + 0.022, 0.03, len, GOLD_TRIM);
        let inner = s * (hw - 0.2);
        rect(
            m,
            0.012,
            inner - 0.011,
            inner + 0.011,
            0.14,
            len - 0.06,
            tint(GOLD_TRIM, 0.9),
        );
        rect(
            glow,
            0.0125,
            outer - 0.006,
            outer + 0.006,
            0.03,
            len,
            [0.28, 0.15, 0.03],
        );
    }
    let motifs = ((len - 0.4) / 0.9).floor().max(0.0) as usize;
    for k in 0..motifs {
        let c = from + axis * (0.55 + k as f32 * 0.9) + Y(0.0125);
        let mut diamond = |cc: Vec3, a: f32, b: f32, color: C3| {
            quad(
                m,
                [cc - axis * a, cc + side * b, cc + axis * a, cc - side * b],
                VY,
                color,
            )
        };
        diamond(c, 0.2, 0.13, GOLD_TRIM);
        diamond(c + Y(0.001), 0.12, 0.075, DEEP_RED);
        diamond(c + Y(0.002), 0.05, 0.03, GOLD_TRIM);
    }
    let tassels = (hw * 2.0 / 0.07).floor() as usize;
    for k in 0..tassels {
        let s = -hw + 0.035 + k as f32 * 0.07;
        rect(
            m,
            0.0105,
            s - 0.018,
            s + 0.018,
            -0.1,
            0.0,
            [0.92, 0.72, 0.36],
        );
    }
}

pub fn rug(m: &mut Mesh3D, glow: &mut Mesh3D, c: Vec3, r: f32) {
    for k in 0..56 {
        let a = k as f32 * TAU / 56.0;
        let d = v(a.cos(), 0.0, a.sin());
        let t = v(-d.z, 0.0, d.x);
        let p0 = c + d * r + Y(0.0105);
        let p1 = c + d * (r + 0.09) + Y(0.0105);
        quad(
            m,
            [p0 - t * 0.02, p1 - t * 0.02, p1 + t * 0.02, p0 + t * 0.02],
            VY,
            [0.92, 0.72, 0.36],
        );
    }
    annulus(m, c + Y(0.01), VY, 0.0, r, 48, DEEP_RED, 0.0);
    annulus(m, c + Y(0.0112), VY, r * 0.86, r * 0.92, 48, GOLD_TRIM, 0.0);
    annulus(
        m,
        c + Y(0.0112),
        VY,
        0.0,
        r * 0.83,
        48,
        tint(RED, 0.92),
        0.0,
    );
    for k in 0..28 {
        let a = k as f32 * TAU / 28.0;
        disc(
            m,
            c + v(a.cos(), 0.0, a.sin()) * (r * 0.955) + Y(0.0118),
            VY,
            0.03,
            6,
            GOLD_TRIM,
        );
    }
    star(m, c + Y(0.0124), VY, VX, r * 0.78, r * 0.33, 8, GOLD_TRIM);
    star(
        m,
        c + Y(0.013),
        VY,
        VX,
        r * 0.62,
        r * 0.28,
        8,
        tint(RED, 0.8),
    );
    annulus(m, c + Y(0.0136), VY, r * 0.24, r * 0.29, 32, GOLD_TRIM, 0.0);
    annulus(m, c + Y(0.0136), VY, 0.0, r * 0.24, 32, DEEP_RED, 0.0);
    annulus(
        glow,
        c + Y(0.0145),
        VY,
        r * 0.885,
        r * 0.895,
        64,
        [0.34, 0.19, 0.04],
        0.0,
    );
    annulus(
        glow,
        c + Y(0.0145),
        VY,
        r * 0.285,
        r * 0.292,
        40,
        [0.3, 0.17, 0.035],
        0.0,
    );
}

fn coin_color(rng: &mut ArtRng) -> C3 {
    tint(GOLD_TRIM, rng.range(0.82, 1.12))
}

fn heap_radius(t: f32) -> f32 {
    let k = t.clamp(0.0, 1.0);
    (1.0 - k).powf(0.85) * (1.0 - 0.15 * k)
}

fn heap_height(d: f32, height: f32) -> f32 {
    let dd = d.clamp(0.0, 1.0);
    let mut t = 1.0 - dd;
    for _ in 0..6 {
        t = (t + (heap_radius(t) - dd) * 0.8).clamp(0.0, 1.0);
    }
    height * t
}

pub fn hoard(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    centre: Vec3,
    radius: f32,
    height: f32,
    rng: &mut ArtRng,
) {
    let (rings, sides) = (5, 16);
    let point = |ring: usize, i: usize| {
        let t = ring as f32 / rings as f32;
        let r = radius * heap_radius(t) * 0.97;
        let a = i as f32 * TAU / sides as f32 + ring as f32 * 0.2;
        centre + v(a.cos() * r, height * t, a.sin() * r)
    };
    let top = centre + Y(height);
    for ring in 0..rings {
        for i in 0..sides {
            let j = (i + 1) % sides;
            let shade = 0.78 + ring as f32 * 0.07;
            if ring + 1 == rings {
                tri(
                    m,
                    point(ring, i),
                    point(ring, j),
                    top,
                    VY,
                    tint(DARK_GOLD, shade),
                );
            } else {
                let q = [
                    point(ring, i),
                    point(ring, j),
                    point(ring + 1, j),
                    point(ring + 1, i),
                ];
                quad(
                    m,
                    q,
                    norm((q[0] + q[1] + q[2] + q[3]) * 0.25 - centre),
                    tint(DARK_GOLD, shade),
                );
            }
        }
    }
    let count = (radius * radius * 260.0).floor() as usize + 30;
    for k in 0..count {
        let rs = radius * 0.98 * rng.unit64().sqrt() as f32;
        let a = rng.range(0.0, TAU);
        let y = heap_height(rs / radius, height);
        let slope = (heap_height((rs / radius - 0.05).max(0.0), height)
            - heap_height((rs / radius + 0.05).min(1.0), height))
            / (0.1 * radius);
        let radial = v(a.cos(), 0.0, a.sin());
        let normal = norm(VY + radial * slope);
        let nx = rng.range(-0.4, 0.4);
        let nz = rng.range(-0.4, 0.4);
        let n = norm(normal + v(nx, 0.0, nz));
        let color = coin_color(rng);
        let size = rng.range(0.06, 0.092);
        coin(
            m,
            centre + v(a.cos() * rs, y, a.sin() * rs) + n * 0.008,
            n,
            size,
            0.018,
            color,
            k as f32,
        );
    }
    let stacks = rng.index(4) + 4;
    for _ in 0..stacks {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.range(0.92, 1.22);
        let base = centre + v(a.cos() * d, 0.0, a.sin() * d);
        let layers = 3 + rng.index(9);
        for l in 0..layers {
            let jx = rng.range(-0.008, 0.008);
            let jz = rng.range(-0.008, 0.008);
            let color = coin_color(rng);
            coin(
                m,
                base + v(jx, 0.0, jz) + Y(0.009 + l as f32 * 0.017),
                VY,
                0.062,
                0.016,
                color,
                l as f32 * 0.4,
            );
        }
    }
    let gems = 3 + rng.index(3);
    for _ in 0..gems {
        let rs = radius * rng.range(0.1, 0.8);
        let a = rng.range(0.0, TAU);
        let y = heap_height(rs / radius, height);
        let gem = GEMS[rng.index(GEMS.len())];
        let r = rng.range(0.035, 0.05);
        let h = rng.range(0.08, 0.13);
        crystal(
            glow,
            centre + v(a.cos() * rs, y - 0.02, a.sin() * rs),
            r,
            h,
            tint(gem, 0.5),
            5,
        );
    }
    if rng.chance(0.55) {
        let rs = radius * rng.range(0.0, 0.3);
        let a = rng.range(0.0, TAU);
        let y = heap_height(rs / radius, height);
        let yaw = rng.range(0.0, TAU);
        let lean = rng.range(0.1, 0.35);
        sword(m, centre + v(a.cos() * rs, y, a.sin() * rs), yaw, lean);
    }
}

pub fn sword(m: &mut Mesh3D, entry: Vec3, yaw: f32, lean: f32) {
    let tilt = yaw_dir(yaw);
    let up = norm(VY * lean.cos() + tilt * lean.sin());
    let flat = v(tilt.z, 0.0, -tilt.x);
    let at = |t: f32| entry + up * t;
    bar(m, at(-0.18), at(0.55), 0.034, 0.006, flat, STEEL);
    let n = norm(flat.cross(up));
    tri(
        m,
        at(0.55) - flat * 0.034,
        at(0.55) + flat * 0.034,
        at(0.62),
        n,
        tint(STEEL, 1.1),
    );
    tri(
        m,
        at(0.55) - flat * 0.034,
        at(0.55) + flat * 0.034,
        at(0.62),
        -n,
        tint(STEEL, 0.9),
    );
    let hilt = |t: f32| entry - up * t;
    bar(
        m,
        hilt(0.18) - flat * 0.14,
        hilt(0.18) + flat * 0.14,
        0.018,
        0.02,
        up,
        GOLD_TRIM,
    );
    bar(
        m,
        hilt(0.2),
        hilt(0.36),
        0.018,
        0.018,
        flat,
        [0.32, 0.07, 0.05],
    );
    lathe(
        m,
        hilt(0.4),
        up,
        &[(0.0, 0.0), (0.02, 0.03), (0.04, 0.028), (0.06, 0.0)],
        6,
        GOLD_TRIM,
        false,
    );
}

pub fn altar(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    relic: &str,
    stone: C3,
    rng: &mut ArtRng,
) {
    let front = yaw_dir(yaw);
    let side = v(front.z, 0.0, -front.x);
    let y = |h: f32| base + Y(h);
    cuboid(m, y(0.08), v(0.4, 0.08, 0.4), tint(stone, 0.72));
    cuboid(m, y(0.18), v(0.34, 0.02, 0.34), GOLD_TRIM);
    cuboid(m, y(0.56), v(0.28, 0.36, 0.28), tint(stone, 1.0));
    for face in [side, -side, -front] {
        let across = v(-face.z, 0.0, face.x);
        let c = y(0.56) + face * 0.281;
        let frame_color = tint(stone, 1.22);
        for sy in [-1.0f32, 1.0] {
            bar(
                m,
                c + Y(sy * 0.25) - across * 0.19,
                c + Y(sy * 0.25) + across * 0.19,
                0.018,
                0.012,
                VY,
                frame_color,
            );
        }
        for sx in [-1.0f32, 1.0] {
            bar(
                m,
                c + across * (sx * 0.19) - Y(0.25),
                c + across * (sx * 0.19) + Y(0.25),
                0.018,
                0.012,
                across,
                frame_color,
            );
        }
        disc(m, c + face * 0.006, face, 0.1, 16, tint(stone, 0.8));
        star(m, c + face * 0.012, face, VY, 0.085, 0.036, 8, GOLD_TRIM);
        disc(m, c + face * 0.016, face, 0.026, 10, tint(RED, 1.1));
    }
    cuboid(m, y(0.94), v(0.32, 0.02, 0.32), GOLD_TRIM);
    cuboid(m, y(1.01), v(0.37, 0.05, 0.37), tint(stone, 1.18));
    let top = 1.063;
    let hw = 0.26;
    let f = front * 0.375;
    quad(
        m,
        [
            y(top) - side * hw,
            y(top) + side * hw,
            y(top) + side * hw + f,
            y(top) - side * hw + f,
        ],
        VY,
        RED,
    );
    let drop_top = y(top) + front * 0.376;
    let lower = drop_top - Y(0.42);
    let tip = drop_top - Y(0.58);
    quad(
        m,
        [
            drop_top - side * hw,
            drop_top + side * hw,
            lower + side * hw,
            lower - side * hw,
        ],
        front,
        RED,
    );
    tri(m, lower - side * hw, lower + side * hw, tip, front, RED);
    let edge = front * 0.003;
    for s in [-1.0f32, 1.0] {
        let a = drop_top + side * (s * (hw - 0.03)) + edge;
        let b = lower + side * (s * (hw - 0.03)) + edge;
        bar(m, a, b, 0.012, 0.002, side, GOLD_TRIM);
        bar(m, b, tip + edge, 0.012, 0.002, side, GOLD_TRIM);
    }
    star(
        m,
        drop_top - Y(0.2) + edge * 2.0,
        front,
        VY,
        0.085,
        0.035,
        8,
        GOLD_TRIM,
    );
    lathe(
        m,
        tip - Y(0.02),
        -VY,
        &[(0.0, 0.015), (0.03, 0.02), (0.08, 0.0)],
        6,
        GOLD_TRIM,
        false,
    );
    let t = y(top + 0.005);
    match relic {
        "crown" => {
            for k in 0..16 {
                let a = rng.range(0.0, TAU);
                let r = 0.2 * rng.unit64().sqrt() as f32;
                let h = 0.012 + (0.2 - r) * 0.25;
                let nx = rng.range(-0.3, 0.3);
                let nz = rng.range(-0.3, 0.3);
                let n = norm(VY + v(nx, 0.0, nz));
                let color = coin_color(rng);
                coin(
                    m,
                    t + v(a.cos() * r, h, a.sin() * r),
                    n,
                    0.055,
                    0.015,
                    color,
                    k as f32,
                );
            }
            append(
                m,
                &crown_mesh(),
                &trs_yaw(t + Y(0.06), yaw, 0.52),
                [1.08, 1.02, 0.9],
            );
            for k in 0..4 {
                let a = yaw + k as f32 * FRAC_PI_2 + 0.4;
                crystal(
                    glow,
                    t + v(a.cos() * 0.14, 0.12, a.sin() * 0.14),
                    0.022,
                    0.05,
                    tint(GEMS[k % 3], 0.55),
                    5,
                );
            }
        }
        "helm" => {
            lathe_fn(
                m,
                t,
                VY,
                &[
                    (0.0, 0.2),
                    (0.06, 0.24),
                    (0.14, 0.2),
                    (0.2, 0.1),
                    (0.26, 0.075),
                    (0.3, 0.12),
                    (0.38, 0.145),
                    (0.44, 0.15),
                    (0.47, 0.148),
                    (0.55, 0.13),
                    (0.61, 0.08),
                    (0.64, 0.0),
                ],
                12,
                &|j| {
                    if j == 7 {
                        [0.12, 0.08, 0.05]
                    } else {
                        tint(GOLD_TRIM, 0.92)
                    }
                },
                true,
            );
            for k in 0..6 {
                let a = -0.9 + k as f32 * 0.36;
                let p0 = t + Y(0.6 + a.cos() * 0.06) + front * (a.sin() * 0.14);
                let p1 = t + Y(0.62 + a.cos() * 0.09) + front * (a.sin() * 0.14 - 0.07);
                bar(m, p0, p1, 0.022, 0.03, side, tint(RED, 1.1));
            }
            append(
                m,
                &crown_mesh(),
                &trs_yaw(t + Y(0.6), yaw, 0.34),
                [1.05, 1.0, 0.9],
            );
        }
        "armillary" => {
            lathe(
                m,
                t,
                VY,
                &[
                    (0.0, 0.12),
                    (0.03, 0.1),
                    (0.04, 0.04),
                    (0.2, 0.025),
                    (0.22, 0.05),
                    (0.24, 0.02),
                ],
                10,
                GOLD_TRIM,
                true,
            );
            let c = t + Y(0.46);
            tube_ring(m, c, VY, 0.21, 0.011, 28, GOLD_TRIM);
            tube_ring(
                m,
                c,
                norm(VX + Y(0.4)),
                0.21,
                0.011,
                28,
                tint(GOLD_TRIM, 0.92),
            );
            tube_ring(
                m,
                c,
                norm(VZ + VX * 0.5),
                0.19,
                0.01,
                28,
                tint(GOLD_TRIM, 1.05),
            );
            tube_ring(m, c, side, 0.235, 0.014, 28, DARK_GOLD);
            bar(m, c - Y(0.235), c - Y(0.22), 0.02, 0.02, side, GOLD_TRIM);
            lathe(
                m,
                c - Y(0.07),
                VY,
                &[
                    (0.0, 0.0),
                    (0.02, 0.05),
                    (0.07, 0.07),
                    (0.12, 0.05),
                    (0.14, 0.0),
                ],
                10,
                tint(GOLD_TRIM, 1.05),
                false,
            );
            star(m, c + Y(0.3), front, VY, 0.08, 0.03, 8, GOLD_TRIM);
            star(m, c + Y(0.3), -front, VY, 0.08, 0.03, 8, GOLD_TRIM);
            crystal(glow, c - Y(0.01), 0.03, 0.07, [0.5, 0.3, 0.06], 6);
        }
        "gems" => {
            lathe(
                m,
                t,
                VY,
                &[(0.0, 0.07), (0.02, 0.14), (0.05, 0.19), (0.055, 0.17)],
                12,
                GOLD_TRIM,
                true,
            );
            for (k, &(dx, dz, r, h)) in [
                (0.0, 0.0, 0.07, 0.3),
                (0.09, 0.05, 0.05, 0.2),
                (-0.08, 0.06, 0.045, 0.17),
                (0.02, -0.1, 0.04, 0.14),
            ]
            .iter()
            .enumerate()
            {
                let p = t + side * dx + front * dz + Y(0.04);
                crystal(glow, p, r, h, tint(GEMS[k % GEMS.len()], 0.6), 6);
                crystal(m, p, r * 1.08, h * 0.96, tint(GEMS[k % GEMS.len()], 0.7), 6);
            }
        }
        _ => {
            lathe_fn(
                m,
                t,
                VY,
                &[
                    (0.0, 0.1),
                    (0.02, 0.1),
                    (0.035, 0.03),
                    (0.16, 0.025),
                    (0.18, 0.05),
                    (0.26, 0.1),
                    (0.3, 0.11),
                    (0.33, 0.11),
                ],
                12,
                &|j| if j == 4 { tint(RED, 1.2) } else { GOLD_TRIM },
                false,
            );
            disc(m, t + Y(0.3), VY, 0.1, 12, [0.3, 0.02, 0.03]);
            for k in 0..3 {
                let a = yaw + 1.2 + k as f32 * 1.9;
                let b = t + v(a.cos() * 0.2, 0.0, a.sin() * 0.2);
                for l in 0..3 + k * 2 {
                    let color = coin_color(rng);
                    coin(
                        m,
                        b + Y(0.009 + l as f32 * 0.017),
                        VY,
                        0.05,
                        0.016,
                        color,
                        l as f32,
                    );
                }
            }
        }
    }
}

pub fn urn(m: &mut Mesh3D, base: Vec3, height: f32, style: usize, yaw: f32) {
    let body = match style % 3 {
        0 => RED,
        1 => NAVY,
        _ => tint(GOLD_TRIM, 0.85),
    };
    let band = if style % 3 == 2 { RED } else { GOLD_TRIM };
    let s = height;
    let profile: Vec<(f32, f32)> = [
        (0.0, 0.1),
        (0.04, 0.13),
        (0.07, 0.15),
        (0.2, 0.26),
        (0.42, 0.3),
        (0.62, 0.26),
        (0.76, 0.16),
        (0.84, 0.1),
        (0.9, 0.1),
        (0.94, 0.14),
        (0.97, 0.14),
        (0.97, 0.09),
    ]
    .iter()
    .map(|&(y, r)| (y * s, r * s * 0.95))
    .collect();
    lathe_fn(
        m,
        base,
        VY,
        &profile,
        12,
        &|j| {
            if [0, 1, 5, 8, 9].contains(&j) {
                band
            } else if j == 10 {
                [0.06, 0.03, 0.02]
            } else {
                body
            }
        },
        true,
    );
    for k in 0..2 {
        let a = yaw + k as f32 * PI;
        let d = v(a.cos(), 0.0, a.sin());
        let t = v(-d.z, 0.0, d.x);
        let steps = 7;
        let point = |i: usize| {
            let u = i as f32 / steps as f32;
            base + Y(s * (0.86 - u * 0.24))
                + d * (s * 0.95 * (0.1 + u * 0.16) + (u * PI).sin() * s * 0.09)
        };
        for i in 0..steps {
            bar(
                m,
                point(i),
                point(i + 1),
                0.016 * s.max(0.6),
                0.012 * s.max(0.6),
                t,
                band,
            );
        }
    }
    for k in 0..6 {
        let a = yaw + k as f32 * TAU / 6.0;
        let d = v(a.cos(), 0.0, a.sin());
        let t = v(-d.z, 0.0, d.x);
        let c = base + Y(0.42 * s) + d * (0.3 * s * 0.95 + 0.004);
        let h = 0.07 * s;
        quad(
            m,
            [c - Y(h), c + t * (h * 0.6), c + Y(h), c - t * (h * 0.6)],
            d,
            band,
        );
    }
}

pub fn candelabrum(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let gold = GOLD_TRIM;
    for k in 0..3 {
        let a = yaw + k as f32 * TAU / 3.0;
        bar(
            m,
            base + Y(0.09),
            base + v(a.cos(), 0.0, a.sin()) * 0.24 + Y(0.015),
            0.02,
            0.015,
            VY,
            tint(gold, 0.85),
        );
    }
    lathe(
        m,
        base,
        VY,
        &[(0.0, 0.0), (0.04, 0.12), (0.08, 0.1), (0.1, 0.04)],
        10,
        gold,
        false,
    );
    lathe_fn(
        m,
        base + Y(0.1),
        VY,
        &[
            (0.0, 0.03),
            (0.2, 0.024),
            (0.24, 0.05),
            (0.28, 0.024),
            (0.78, 0.02),
            (0.82, 0.05),
            (0.86, 0.026),
            (1.06, 0.026),
        ],
        8,
        &|j| if j % 2 == 1 { tint(gold, 1.08) } else { gold },
        false,
    );
    let cup = |m: &mut Mesh3D, c: Vec3| {
        lathe(
            m,
            c,
            VY,
            &[(0.0, 0.02), (0.03, 0.055), (0.045, 0.06)],
            8,
            gold,
            true,
        )
    };
    let top = base + Y(1.16);
    cup(m, top);
    let h = rng.range(0.18, 0.24);
    candle(m, glow, top + Y(0.045), 0.036, h, rng);
    for k in 0..4 {
        let a = yaw + FRAC_PI_2 * 0.5 + k as f32 * FRAC_PI_2;
        let d = v(a.cos(), 0.0, a.sin());
        let root = base + Y(0.96);
        let elbow = root + d * 0.3;
        bar(m, root, elbow, 0.014, 0.014, VY, gold);
        bar(m, elbow, elbow + Y(0.1), 0.014, 0.014, d, gold);
        cup(m, elbow + Y(0.1));
        let h = rng.range(0.13, 0.19);
        candle(m, glow, elbow + Y(0.145), 0.032, h, rng);
    }
}

pub fn chandelier(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    ceiling: Vec3,
    ring_y: f32,
    radius: f32,
    rng: &mut ArtRng,
    candles: usize,
) {
    let hub_y = ring_y + 0.72;
    let x0 = with_y(ceiling, 0.0);
    let at = |y: f32| x0 + Y(y);
    let mut y = ceiling.y + 0.25;
    let mut alt = false;
    while y > hub_y + 0.12 {
        bar(
            m,
            at(y),
            at(y - 0.1),
            0.028,
            0.01,
            if alt { VX } else { VZ },
            IRON,
        );
        y -= 0.085;
        alt = !alt;
    }
    lathe(
        m,
        at(hub_y - 0.1),
        VY,
        &[
            (0.0, 0.0),
            (0.05, 0.09),
            (0.12, 0.11),
            (0.18, 0.06),
            (0.24, 0.02),
        ],
        10,
        GOLD_TRIM,
        false,
    );
    lathe(
        m,
        at(hub_y - 0.1),
        -VY,
        &[(0.0, 0.05), (0.18, 0.03), (0.36, 0.0)],
        8,
        GOLD_TRIM,
        false,
    );
    for k in 0..4 {
        let a = k as f32 * FRAC_PI_2 + 0.39;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            at(hub_y + 0.06),
            at(ring_y + 0.05) + d * (radius - 0.02),
            0.01,
            0.01,
            VY,
            IRON,
        );
    }
    let c = at(ring_y);
    tube_ring(m, c, VY, radius, 0.06, 32, tint(DEEP_RED, 1.3));
    tube_ring(m, c + Y(0.072), VY, radius, 0.026, 32, GOLD_TRIM);
    tube_ring(m, c - Y(0.072), VY, radius, 0.026, 32, GOLD_TRIM);
    for k in 0..16 {
        let a = k as f32 * TAU / 16.0;
        lathe(
            m,
            c + v(a.cos(), 0.0, a.sin()) * (radius + 0.06) - Y(0.1),
            -VY,
            &[(0.0, 0.018), (0.04, 0.0)],
            5,
            GOLD_TRIM,
            false,
        );
    }
    for k in 0..candles {
        let a = k as f32 * TAU / candles as f32;
        let p = c + v(a.cos(), 0.0, a.sin()) * radius + Y(0.1);
        lathe(
            m,
            p - Y(0.01),
            VY,
            &[(0.0, 0.025), (0.025, 0.05), (0.035, 0.055)],
            8,
            GOLD_TRIM,
            true,
        );
        let h = rng.range(0.14, 0.2);
        candle(m, glow, p + Y(0.03), 0.03, h, rng);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn banner(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    top: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    length: f32,
    sun: bool,
) {
    let off = inward * 0.09;
    let rod = top + off + inward * 0.02;
    bar(
        m,
        rod - tangent * (width * 0.5 + 0.1),
        rod + tangent * (width * 0.5 + 0.1),
        0.018,
        0.018,
        VY,
        GOLD_TRIM,
    );
    for s in [-1.0f32, 1.0] {
        lathe(
            m,
            rod + tangent * (s * (width * 0.5 + 0.1)),
            tangent * s,
            &[(0.0, 0.03), (0.03, 0.04), (0.07, 0.0)],
            6,
            GOLD_TRIM,
            false,
        );
        bar(
            m,
            rod + tangent * (s * width * 0.42),
            top + tangent * (s * width * 0.42) - inward * 0.05,
            0.012,
            0.012,
            VY,
            IRON,
        );
    }
    let hw = width * 0.5;
    let p = |x: f32, y: f32| top + off + tangent * x - Y(y);
    let bottom = length;
    let tip = length + 0.36;
    let strips = 6;
    let fold = |k: usize| if k % 2 == 0 { 0.0 } else { 0.022 };
    for k in 0..strips {
        let x0 = -hw + width * k as f32 / strips as f32;
        let x1 = -hw + width * (k + 1) as f32 / strips as f32;
        let a = |y: f32| p(x0, y) + inward * fold(k);
        let b = |y: f32| p(x1, y) + inward * fold(k + 1);
        let mut n = norm((b(0.5) - a(0.5)).cross(-VY));
        if n.dot(inward) < 0.0 {
            n = -n;
        }
        let shade = if k % 2 == 0 { 0.86 } else { 1.04 };
        quad(
            m,
            [a(0.03), b(0.03), b(bottom), a(bottom)],
            n,
            tint(RED, shade),
        );
        quad(
            m,
            [a(0.03), b(0.03), b(bottom), a(bottom)],
            -n,
            tint(RED, 0.62),
        );
    }
    for (normal, color) in [(inward, RED), (-inward, tint(RED, 0.62))] {
        tri(m, p(-hw, bottom), p(hw, bottom), p(0.0, tip), normal, color);
    }
    let front = |x: f32, yy: f32| p(x, yy) + inward * 0.026;
    let front2 = |x: f32, yy: f32| p(x, yy) + inward * 0.029;
    for s in [-1.0f32, 1.0] {
        let x = s * (hw - 0.07);
        quad(
            m,
            [
                front(x - 0.022, 0.1),
                front(x + 0.022, 0.1),
                front(x + 0.022, bottom - 0.02),
                front(x - 0.022, bottom - 0.02),
            ],
            inward,
            GOLD_TRIM,
        );
        let a = front(s * (hw - 0.07), bottom - 0.02);
        let b = front(0.0, tip - 0.1);
        bar(m, a, b, 0.02, 0.002, inward.cross(norm(b - a)), GOLD_TRIM);
    }
    quad(
        m,
        [
            front(-hw + 0.05, 0.1),
            front(hw - 0.05, 0.1),
            front(hw - 0.05, 0.14),
            front(-hw + 0.05, 0.14),
        ],
        inward,
        GOLD_TRIM,
    );
    let e = (length * 0.42).min(0.72);
    let ec = front(0.0, e);
    annulus(m, ec, inward, 0.2, 0.235, 28, GOLD_TRIM, 0.0);
    if sun {
        star(m, front2(0.0, e), inward, VY, 0.19, 0.11, 16, GOLD_TRIM);
        disc(
            m,
            front2(0.0, e) + inward * 0.003,
            inward,
            0.1,
            16,
            DEEP_RED,
        );
        let side = norm(VY.cross(inward));
        let c = front2(0.0, e) + inward * 0.006;
        let pt = |a: f32| c + side * (a.cos() * 0.075) + Y(a.sin() * 0.038);
        for k in 0..12 {
            tri(
                m,
                c,
                pt(k as f32 * TAU / 12.0),
                pt((k + 1) as f32 * TAU / 12.0),
                inward,
                GOLD_TRIM,
            );
        }
        disc(m, c + inward * 0.003, inward, 0.02, 8, DEEP_RED);
    } else {
        star(m, front2(0.0, e), inward, VY, 0.19, 0.075, 8, GOLD_TRIM);
        disc(
            m,
            front2(0.0, e) + inward * 0.003,
            inward,
            0.045,
            10,
            DEEP_RED,
        );
    }
    annulus(
        glow,
        ec + inward * 0.004,
        inward,
        0.215,
        0.222,
        28,
        [0.25, 0.14, 0.03],
        0.0,
    );
    for x in [-0.09f32, 0.0, 0.09] {
        let y0 = e + 0.3;
        let y1 = (bottom - 0.1).max(y0 + 0.2) + if x == 0.0 { 0.22 } else { 0.0 };
        quad(
            m,
            [
                front(x - 0.012, y0),
                front(x + 0.012, y0),
                front(x + 0.012, y1),
                front(x - 0.012, y1),
            ],
            inward,
            tint(GOLD_TRIM, 0.95),
        );
    }
    lathe(
        m,
        p(0.0, tip) + inward * 0.002,
        -VY,
        &[(0.0, 0.02), (0.04, 0.035), (0.12, 0.0)],
        6,
        GOLD_TRIM,
        false,
    );
    for s in [-1.0f32, 1.0] {
        let a = p(s * hw, bottom);
        let b = p(0.0, tip);
        for k in 0..=9 {
            let q = lerp3(a, b, k as f32 / 9.0) + inward * 0.01;
            bar(
                m,
                q,
                q - Y(0.07),
                0.008,
                0.006,
                tangent,
                tint(GOLD_TRIM, 0.95),
            );
        }
        lathe(
            m,
            a + inward * 0.01,
            -VY,
            &[(0.0, 0.015), (0.03, 0.028), (0.1, 0.0)],
            6,
            GOLD_TRIM,
            false,
        );
    }
}

pub fn coin_scatter(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    centre: Vec3,
    radius: f32,
    count: usize,
    rng: &mut ArtRng,
) {
    for _ in 0..1 + count / 12 {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.unit64().sqrt() as f32;
        let gem = GEMS[rng.index(GEMS.len())];
        crystal(
            glow,
            centre + v(a.cos() * r, 0.01, a.sin() * r),
            0.03,
            0.06,
            tint(gem, 0.5),
            5,
        );
    }
    for s in 0..count / 8 {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.range(0.2, 1.0);
        let base = centre + v(a.cos() * r, 0.0, a.sin() * r);
        let layers = 2 + rng.index(6);
        for l in 0..layers {
            let jx = rng.range(-0.006, 0.006);
            let jz = rng.range(-0.006, 0.006);
            let color = coin_color(rng);
            coin(
                m,
                base + v(jx, 0.0, jz) + Y(0.009 + l as f32 * 0.017),
                VY,
                0.058,
                0.016,
                color,
                (s + l) as f32,
            );
        }
    }
    for k in 0..count {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.unit64().sqrt() as f32;
        let nx = rng.range(-0.25, 0.25);
        let nz = rng.range(-0.25, 0.25);
        let n = norm(VY + v(nx, 0.0, nz));
        let size = rng.range(0.05, 0.07);
        let color = coin_color(rng);
        coin(
            m,
            centre + v(a.cos() * r, 0.012, a.sin() * r),
            n,
            size,
            0.015,
            color,
            k as f32,
        );
    }
}

pub fn chest_hoard(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let x = v(yaw.cos(), 0.0, -yaw.sin());
    let z = yaw_dir(yaw);
    for k in 0..70 {
        let u = rng.range(-1.0, 1.0);
        let w = rng.range(-1.0, 1.0);
        let fall = 1.0 - (u * u * 0.7 + w * w * 0.7).min(1.0);
        let p = base + x * (u * 0.33) + z * (w * 0.24) + Y(0.27 + fall * 0.13);
        let n = norm(VY + x * (u * 0.4) + z * (w * 0.4));
        let size = rng.range(0.045, 0.06);
        let color = coin_color(rng);
        coin(m, p, n, size, 0.014, color, k as f32);
    }
    for k in 0..5 {
        let px = rng.range(-0.25, 0.25);
        let pz = rng.range(-0.15, 0.18);
        let p = base + x * px + z * pz + Y(0.36);
        let r = rng.range(0.03, 0.045);
        let h = rng.range(0.07, 0.11);
        crystal(glow, p, r, h, tint(GEMS[k % GEMS.len()], 0.55), 5);
    }
    for k in 0..26 {
        let t = rng.unit();
        let px = rng.range(-0.3, 0.3);
        let p = base + z * (0.33 + t * 0.55) + x * px + Y(0.012 + (1.0 - t).powi(3) * 0.2);
        let n = norm(VY + z * (0.6 * (1.0 - t)));
        let size = rng.range(0.045, 0.065);
        let color = coin_color(rng);
        coin(m, p, n, size, 0.015, color, k as f32);
    }
}

// ------------------------------------------------ treasury loot hall kit

const COIN: C3 = [1.0, 0.7, 0.2];
const WOOD: C3 = [0.36, 0.2, 0.1];
const IRON_DARK: C3 = [0.13, 0.115, 0.105];
const AMBERS: [C3; 3] = [[1.0, 0.3, 0.04], [1.0, 0.4, 0.07], [0.92, 0.22, 0.03]];
const BANNER: C3 = [0.56, 0.1, 0.04];
const BANNER_DARK: C3 = [0.26, 0.045, 0.02];
const FLAME_GOLD: C3 = [1.0, 0.56, 0.12];

fn loot_coin(rng: &mut ArtRng) -> C3 {
    tint(COIN, rng.range(0.82, 1.1))
}

/// Chunky 12-sided coin with a bevelled face, readable at a distance.
fn chunky_coin(m: &mut Mesh3D, c: Vec3, n: Vec3, radius: f32, thick: f32, color: C3, phase: f32) {
    let (u, w) = basis(n);
    let top = c + n * (thick * 0.5);
    let bottom = c - n * (thick * 0.5);
    let shoulder = top - n * (thick * 0.32);
    let at = |centre: Vec3, r: f32, a: f32| centre + u * (a.cos() * r) + w * (a.sin() * r);
    for i in 0..12 {
        let a0 = i as f32 * TAU / 12.0 + phase;
        let a1 = (i + 1) as f32 * TAU / 12.0 + phase;
        let radial = u * ((a0 + a1) * 0.5).cos() + w * ((a0 + a1) * 0.5).sin();
        tri(
            m,
            top,
            at(top, radius * 0.8, a0),
            at(top, radius * 0.8, a1),
            n,
            tint(color, if i % 2 == 1 { 1.1 } else { 1.0 }),
        );
        quad(
            m,
            [
                at(top, radius * 0.8, a0),
                at(top, radius * 0.8, a1),
                at(shoulder, radius, a1),
                at(shoulder, radius, a0),
            ],
            norm(n + radial),
            tint(color, 1.22),
        );
        quad(
            m,
            [
                at(bottom, radius, a0),
                at(bottom, radius, a1),
                at(shoulder, radius, a1),
                at(shoulder, radius, a0),
            ],
            radial,
            tint(color, 0.74),
        );
        tri(
            m,
            bottom,
            at(bottom, radius, a1),
            at(bottom, radius, a0),
            -n,
            tint(color, 0.55),
        );
    }
}

fn coin_tower(m: &mut Mesh3D, base: Vec3, count: usize, radius: f32, rng: &mut ArtRng) {
    let thick = 0.026;
    for l in 0..count {
        let jx = rng.range(-0.01, 0.01);
        let jz = rng.range(-0.01, 0.01);
        let last = l == count - 1 && rng.chance(0.35);
        let n = if last {
            let nx = rng.range(-0.3, 0.3);
            let nz = rng.range(-0.3, 0.3);
            norm(VY + v(nx, 0.0, nz))
        } else {
            VY
        };
        let color = loot_coin(rng);
        let phase = rng.range(0.0, TAU);
        chunky_coin(
            m,
            base + v(jx, 0.0, jz) + Y(thick * 0.5 + l as f32 * thick * 0.97),
            n,
            radius,
            thick,
            color,
            phase,
        );
    }
}

fn rotate_about(p: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let (c, s) = (angle.cos(), angle.sin());
    p * c + axis.cross(p) * s + axis * (axis.dot(p) * (1.0 - c))
}

/// Small wooden coffer, lid thrown back, heaped with gold.
fn coffer(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let front = yaw_dir(yaw);
    let side = v(front.z, 0.0, -front.x);
    kit_block(
        m,
        base + Y(0.19),
        side,
        VY,
        front,
        v(0.34, 0.19, 0.24),
        WOOD,
    );
    for y in [0.08, 0.3] {
        bar(
            m,
            base + Y(y) + front * 0.243 - side * 0.33,
            base + Y(y) + front * 0.243 + side * 0.33,
            0.006,
            0.004,
            VY,
            tint(WOOD, 0.6),
        );
    }
    for sx in [-0.22, 0.22] {
        bar(
            m,
            base + side * sx + front * 0.248,
            base + side * sx + front * 0.248 + Y(0.38),
            0.028,
            0.006,
            side,
            IRON_DARK,
        );
        bar(
            m,
            base + side * sx - front * 0.248,
            base + side * sx - front * 0.248 + Y(0.38),
            0.028,
            0.006,
            side,
            IRON_DARK,
        );
    }
    kit_block(
        m,
        base + Y(0.3) + front * 0.25,
        side,
        VY,
        front,
        v(0.045, 0.055, 0.012),
        GOLD_TRIM,
    );
    // Lid hinged on the back edge, thrown open by ~110°.
    let hinge = base + Y(0.38) - front * 0.24;
    let angle = -1.95;
    let offset = rotate_about(front * 0.24 + Y(0.06), side, angle);
    let lid_up = rotate_about(VY, side, angle);
    let lid_front = rotate_about(front, side, angle);
    kit_block(
        m,
        hinge + offset,
        side,
        lid_up,
        lid_front,
        v(0.35, 0.06, 0.245),
        tint(WOOD, 1.08),
    );
    for sx in [-0.22, 0.22] {
        kit_block(
            m,
            hinge + offset + side * sx + lid_up * 0.062,
            side,
            lid_up,
            lid_front,
            v(0.028, 0.006, 0.25),
            IRON_DARK,
        );
    }
    for k in 0..42 {
        let u = rng.range(-1.0, 1.0);
        let w = rng.range(-1.0, 1.0);
        let fall = 1.0 - (u * u * 0.6 + w * w * 0.6).min(1.0);
        let n = norm(VY + side * (u * 0.45) + front * (w * 0.45));
        let size = rng.range(0.055, 0.072);
        let color = loot_coin(rng);
        chunky_coin(
            m,
            base + side * (u * 0.3) + front * (w * 0.2) + Y(0.37 + fall * 0.12),
            n,
            size,
            0.024,
            color,
            k as f32,
        );
    }
    for k in 0..3 {
        let sx = rng.range(-0.22, 0.22);
        let fz = rng.range(-0.1, 0.12);
        let h = rng.range(0.08, 0.12);
        crystal(
            glow,
            base + side * sx + front * fz + Y(0.42),
            0.04,
            h,
            tint(GEMS[k % GEMS.len()], 0.55),
            5,
        );
    }
    for k in 0..16 {
        let t = rng.unit();
        let n = norm(VY + front * (0.5 * (1.0 - t)));
        let sx = rng.range(-0.28, 0.28);
        let size = rng.range(0.055, 0.075);
        let color = loot_coin(rng);
        chunky_coin(
            m,
            base + front * (0.28 + t * 0.55) + side * sx + Y(0.014 + (1.0 - t).powi(3) * 0.2),
            n,
            size,
            0.024,
            color,
            k as f32,
        );
    }
}

/// A coffer knocked onto its side, lid hanging open, gold pouring out.
fn toppled_coffer(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let mut src = empty();
    kit_block(
        &mut src,
        v(0.0, 0.19, 0.0),
        VX,
        VY,
        VZ,
        v(0.34, 0.19, 0.24),
        WOOD,
    );
    kit_block(
        &mut src,
        v(0.0, 0.385, 0.0),
        VX,
        VY,
        VZ,
        v(0.3, 0.006, 0.2),
        [0.08, 0.05, 0.03],
    );
    for sx in [-0.22, 0.22] {
        kit_block(
            &mut src,
            v(sx, 0.19, 0.245),
            VX,
            VY,
            VZ,
            v(0.028, 0.19, 0.006),
            IRON_DARK,
        );
        kit_block(
            &mut src,
            v(sx, 0.19, -0.245),
            VX,
            VY,
            VZ,
            v(0.028, 0.19, 0.006),
            IRON_DARK,
        );
    }
    // Lid swung down against the floor from the back hinge.
    kit_block(
        &mut src,
        v(0.0, 0.44, -0.3),
        VX,
        VZ,
        -VY,
        v(0.35, 0.06, 0.245),
        tint(WOOD, 1.08),
    );
    append(
        m,
        &src,
        &pose_mapper(base + Y(0.24), yaw, 0.0, PI / 2.0, 1.0),
        [1.0; 3],
    );
    // The mouth faces local +Y, which the roll turns toward local -X.
    let mouth = v(-yaw.cos(), 0.0, yaw.sin());
    let side = v(mouth.z, 0.0, -mouth.x);
    for k in 0..34 {
        let t = rng.unit();
        let nx = rng.range(-0.3, 0.3);
        let nz = rng.range(-0.3, 0.3);
        let n = norm(VY + v(nx, 0.0, nz));
        let s = rng.range(-0.3, 0.3);
        let p = base + mouth * (0.3 + t * 0.95) + side * (s * (0.6 + t));
        let size = rng.range(0.065, 0.085);
        let color = loot_coin(rng);
        chunky_coin(
            m,
            p + Y(0.014 + (1.0 - t).powi(2) * 0.06),
            n,
            size,
            0.026,
            color,
            k as f32,
        );
    }
    for k in 0..3 {
        let count = 2 + rng.index(4);
        coin_tower(
            m,
            base + mouth * (0.55 + k as f32 * 0.25) + side * ((k as f32 - 1.0) * 0.35),
            count,
            0.08,
            rng,
        );
    }
    crystal(
        glow,
        base + mouth * 0.7 + Y(0.02),
        0.05,
        0.1,
        tint(GEMS[0], 0.55),
        5,
    );
}

/// Knight's helm with a red crest, lying on its side.
fn lying_helm(m: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let mut src = empty();
    let steel = [0.6, 0.6, 0.64];
    lathe(
        &mut src,
        Vec3::ZERO,
        VY,
        &[
            (0.0, 0.155),
            (0.08, 0.165),
            (0.19, 0.152),
            (0.26, 0.11),
            (0.3, 0.05),
            (0.31, 0.0),
        ],
        12,
        steel,
        true,
    );
    kit_block(
        &mut src,
        v(0.0, 0.15, 0.158),
        VX,
        VY,
        VZ,
        v(0.1, 0.012, 0.012),
        [0.03, 0.03, 0.035],
    );
    kit_block(
        &mut src,
        v(0.0, 0.09, 0.162),
        VX,
        VY,
        VZ,
        v(0.018, 0.07, 0.012),
        tint(steel, 1.15),
    );
    for k in 0..7 {
        let a = -0.9 + k as f32 * 0.3;
        let p0 = v(0.0, 0.29 + a.cos() * 0.04, a.sin() * 0.16);
        let p1 = v(0.0, 0.33 + a.cos() * 0.08, a.sin() * 0.2 - 0.06);
        let shade = rng.range(0.9, 1.1);
        bar(
            &mut src,
            p0,
            p1,
            0.02,
            0.028,
            VX,
            tint([0.68, 0.08, 0.05], shade),
        );
    }
    append(
        m,
        &src,
        &pose_mapper(base + Y(0.16), yaw, 0.15, 1.45, 1.0),
        [1.0; 3],
    );
}

/// A sword dropped flat on the floor.
fn dropped_sword(m: &mut Mesh3D, base: Vec3, yaw: f32) {
    sword(m, base + Y(0.035), yaw, PI / 2.0 - 0.04);
}

/// Scattered gold: loose coins, coin towers, gems and one feature.
#[allow(clippy::too_many_arguments)]
pub fn loot_pile(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    centre: Vec3,
    radius: f32,
    feature: &str,
    yaw: f32,
    rng: &mut ArtRng,
    avoid: Option<(Vec3, f32)>,
) {
    let blocked = |p: Vec3| avoid.is_some_and(|(q, r)| (p.x - q.x).hypot(p.z - q.z) < r);
    let loose = (radius * radius * 30.0).round() as usize + 14;
    for k in 0..loose {
        let r = radius * (rng.unit64().powf(0.7)) as f32;
        let a = rng.range(0.0, TAU);
        let p = centre + v(a.cos() * r, 0.0, a.sin() * r);
        if blocked(p) {
            continue;
        }
        let heap = (1.0 - r / (radius * 0.55)).max(0.0) * 0.12 * rng.unit();
        let nx = rng.range(-0.35, 0.35);
        let nz = rng.range(-0.35, 0.35);
        let n = norm(VY + v(nx, 0.0, nz));
        let size = rng.range(0.065, 0.09);
        let color = loot_coin(rng);
        chunky_coin(m, p + Y(0.014 + heap), n, size, 0.026, color, k as f32);
    }
    let towers = 4 + rng.index(5);
    for _ in 0..towers {
        let r = radius * rng.range(0.2, 0.9);
        let a = rng.range(0.0, TAU);
        let p = centre + v(a.cos() * r, 0.0, a.sin() * r);
        if blocked(p) {
            continue;
        }
        let count = 2 + rng.index(12);
        let size = rng.range(0.075, 0.095);
        coin_tower(m, p, count, size, rng);
    }
    let gems = 1 + rng.index(3);
    for k in 0..gems {
        let r = radius * rng.range(0.2, 0.9);
        let a = rng.range(0.0, TAU);
        let p = centre + v(a.cos() * r, 0.0, a.sin() * r);
        let mut gem = empty();
        let gr = rng.range(0.05, 0.07);
        let gh = rng.range(0.1, 0.16);
        crystal(
            &mut gem,
            Vec3::ZERO,
            gr,
            gh,
            tint(
                [[0.95, 0.08, 0.06], [1.0, 0.4, 0.05], [0.1, 0.8, 0.3]][k % 3],
                0.55,
            ),
            5,
        );
        let turn = rng.range(0.0, TAU);
        let tilt = rng.range(0.6, 1.4);
        append(
            glow,
            &gem,
            &pose_mapper(p + Y(0.03), turn, tilt, 0.0, 1.0),
            [1.0; 3],
        );
    }
    let front = yaw_dir(yaw);
    match feature {
        "coffer" => coffer(m, glow, centre, yaw, rng),
        "weapons" => {
            dropped_sword(m, centre + front * 0.35, yaw + 1.2);
            dropped_sword(
                m,
                centre - front * 0.4 + v(front.z, 0.0, -front.x) * 0.3,
                yaw - 0.5,
            );
        }
        "helm" => lying_helm(m, centre + front * 0.2, yaw, rng),
        "toppled" => toppled_coffer(m, glow, centre, yaw, rng),
        _ => {}
    }
}

/// Glowing amber crystals bursting from a bed of broken stone.
#[allow(clippy::too_many_arguments)]
pub fn amber_crystals(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    scale: f32,
    yaw: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    for k in 0..5 {
        let a = yaw + k as f32 * 1.3 + rng.range(-0.3, 0.3);
        let r = rng.range(0.25, 0.62) * scale;
        let size = rng.range(0.18, 0.36) * scale;
        let sy = rng.range(0.8, 1.4);
        let shade = rng.range(0.55, 0.9);
        rock(
            m,
            base + v(a.cos() * r, 0.0, a.sin() * r),
            v(size * 1.3, size * sy, size),
            tint(stone, shade),
            a,
        );
    }
    let shards = 4 + rng.index(3);
    for k in 0..shards {
        let main = k == 0;
        let a = yaw + k as f32 * 2.2 + rng.range(-0.4, 0.4);
        let r = if main {
            0.05
        } else {
            rng.range(0.15, 0.4) * scale
        };
        let mut src = empty();
        let cr = (if main { 0.17 } else { rng.range(0.07, 0.12) }) * scale;
        let ch = (if main { 0.95 } else { rng.range(0.32, 0.62) }) * scale;
        let shade = rng.range(0.4, 0.5);
        crystal(
            &mut src,
            Vec3::ZERO,
            cr,
            ch,
            tint(AMBERS[k % 3], shade),
            5 + (k % 2),
        );
        let tilt = if main { 0.08 } else { rng.range(0.2, 0.5) };
        append(
            glow,
            &src,
            &pose_mapper(
                base + v(a.cos() * r, 0.05, a.sin() * r),
                a + PI / 2.0,
                tilt,
                0.0,
                1.0,
            ),
            [1.0; 3],
        );
    }
}

/// Iron tripod brazier with a slatted, flaring bowl of coals.
pub fn iron_brazier(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let iron = [0.16, 0.13, 0.12];
    for k in 0..3 {
        let a = yaw + k as f32 * TAU / 3.0;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            base + d * 0.36,
            base + d * 0.2 + Y(0.8),
            0.03,
            0.03,
            VY,
            iron,
        );
        kit_block(
            m,
            base + d * 0.37 + Y(0.03),
            v(-d.z, 0.0, d.x),
            VY,
            d,
            v(0.05, 0.03, 0.06),
            tint(iron, 1.2),
        );
    }
    tube_ring(m, base + Y(0.45), VY, 0.25, 0.02, 20, tint(iron, 1.3));
    tube_ring(m, base + Y(0.8), VY, 0.22, 0.03, 20, tint(iron, 1.3));
    disc(m, base + Y(0.79), VY, 0.22, 12, tint(iron, 0.8));
    for k in 0..14 {
        let a = yaw + k as f32 * TAU / 14.0;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            base + d * 0.21 + Y(0.8),
            base + d * 0.43 + Y(1.2),
            0.024,
            0.012,
            v(-d.z, 0.0, d.x),
            tint(iron, if k % 2 == 1 { 1.0 } else { 1.25 }),
        );
    }
    tube_ring(m, base + Y(1.2), VY, 0.43, 0.03, 28, tint(iron, 1.4));
    for _ in 0..7 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.0, 0.22);
        let sy = rng.range(0.07, 0.14);
        rock(
            m,
            base + v(a.cos() * r, 0.86, a.sin() * r),
            v(0.1, sy, 0.09),
            [0.14, 0.05, 0.03],
            a,
        );
    }
    for _ in 0..8 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.0, 0.26);
        let y = 0.9 + rng.range(0.0, 0.08);
        let shade = rng.range(0.55, 0.8);
        rock(
            glow,
            base + v(a.cos() * r, y, a.sin() * r),
            v(0.06, 0.05, 0.055),
            tint([1.0, 0.32, 0.05], shade),
            a,
        );
    }
}

/// Long red banner with a flame sigil and a ragged, three-tailed hem.
#[allow(clippy::too_many_arguments)]
pub fn flame_banner(
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
        0.02,
        0.02,
        VY,
        IRON_DARK,
    );
    for s in [-1.0f32, 1.0] {
        bar(
            m,
            rod + tangent * (s * width * 0.4),
            top + tangent * (s * width * 0.4) - inward * 0.05,
            0.014,
            0.014,
            VY,
            IRON_DARK,
        );
        lathe(
            m,
            rod + tangent * (s * (width * 0.5 + 0.08)),
            tangent * s,
            &[(0.0, 0.03), (0.03, 0.035), (0.06, 0.0)],
            6,
            tint(IRON_DARK, 1.4),
            false,
        );
    }
    let hw = width * 0.5;
    let p = |x: f32, y: f32| top + off + tangent * x - Y(y);
    let strips = 6;
    let fold = |k: usize| if k % 2 == 0 { 0.0 } else { 0.024 };
    for k in 0..strips {
        let x0 = -hw + width * k as f32 / strips as f32;
        let x1 = -hw + width * (k + 1) as f32 / strips as f32;
        let a = |y: f32| p(x0, y) + inward * fold(k);
        let b = |y: f32| p(x1, y) + inward * fold(k + 1);
        let mut n = norm((b(0.5) - a(0.5)).cross(-VY));
        if n.dot(inward) < 0.0 {
            n = -n;
        }
        let edge = k == 0 || k == strips - 1;
        let color = tint(
            if edge { BANNER_DARK } else { BANNER },
            if k % 2 == 0 { 0.88 } else { 1.05 },
        );
        quad(m, [a(0.03), b(0.03), b(length), a(length)], n, color);
        quad(
            m,
            [a(0.03), b(0.03), b(length), a(length)],
            -n,
            tint(BANNER, 0.5),
        );
    }
    // Ragged hem: three tails of uneven length.
    let tails = [
        (-hw, -hw / 3.0, -hw * 0.66, 0.3),
        (-hw / 3.0, hw / 3.0, 0.0, 0.42),
        (hw / 3.0, hw, hw * 0.66, 0.26),
    ];
    for (x0, x1, xt, drop) in tails {
        for (n, color) in [(inward, BANNER), (-inward, tint(BANNER, 0.5))] {
            tri(
                m,
                p(x0, length),
                p(x1, length),
                p(xt, length + drop),
                n,
                color,
            );
        }
    }
    quad(
        m,
        [
            p(-hw, 0.03) + inward * 0.026,
            p(hw, 0.03) + inward * 0.026,
            p(hw, 0.16) + inward * 0.026,
            p(-hw, 0.16) + inward * 0.026,
        ],
        inward,
        BANNER_DARK,
    );
    // Flame sigil: a three-tongued fire over a small diamond.
    let cy = (length * 0.45).min(0.85);
    let front = |x: f32, y: f32, d: f32| p(x, y) + inward * d;
    let scale = (width * 0.36).min(0.34);
    let outline: [(f32, f32); 12] = [
        (0.0, -1.0),
        (0.18, -0.52),
        (0.42, -0.72),
        (0.36, -0.2),
        (0.55, 0.05),
        (0.35, 0.55),
        (0.0, 0.72),
        (-0.35, 0.55),
        (-0.55, 0.05),
        (-0.36, -0.2),
        (-0.42, -0.72),
        (-0.18, -0.52),
    ];
    let mut flame_at = |k: f32, d: f32, color: C3| {
        let c = front(0.0, cy + 0.05 * k, d);
        for i in 0..outline.len() {
            let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
            tri(
                m,
                c,
                front(a.0 * scale * k, cy + a.1 * scale * k, d),
                front(b.0 * scale * k, cy + b.1 * scale * k, d),
                inward,
                color,
            );
        }
    };
    flame_at(1.0, 0.03, FLAME_GOLD);
    flame_at(0.55, 0.034, [1.0, 0.82, 0.36]);
    let dy = cy + scale * 1.05;
    let d = 0.1;
    quad(
        m,
        [
            front(0.0, dy - d, 0.032),
            front(d * 0.7, dy, 0.032),
            front(0.0, dy + d, 0.032),
            front(-d * 0.7, dy, 0.032),
        ],
        inward,
        FLAME_GOLD,
    );
    annulus(
        glow,
        front(0.0, cy, 0.036),
        inward,
        scale * 0.02,
        scale * 0.08,
        10,
        [0.35, 0.16, 0.03],
        0.0,
    );
}

/// Clay amphora / jug by the walls.
pub fn amphora(m: &mut Mesh3D, base: Vec3, h: f32, rng: &mut ArtRng) {
    let clay = tint([0.5, 0.26, 0.13], rng.range(0.8, 1.1));
    let shades = [0.7, 0.9, 1.05, 1.0, 0.9, 0.85, 1.05, 1.1];
    lathe_fn(
        m,
        base,
        VY,
        &[
            (0.0, 0.07 * h),
            (0.06 * h, 0.16 * h),
            (0.3 * h, 0.24 * h),
            (0.55 * h, 0.2 * h),
            (0.72 * h, 0.1 * h),
            (0.86 * h, 0.07 * h),
            (0.95 * h, 0.1 * h),
            (1.0 * h, 0.09 * h),
        ],
        12,
        &|j| tint(clay, shades[j.min(7)]),
        true,
    );
    tube_ring(
        m,
        base + Y(0.42 * h),
        VY,
        0.225 * h,
        0.012,
        18,
        tint(clay, 0.55),
    );
}

// ------------------------------------------------ grand treasury pieces

/// A crown on a velvet cushion atop a plain stone block.
pub fn crown_block(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let front = yaw_dir(yaw);
    let side = v(front.z, 0.0, -front.x);
    let grey = [0.44, 0.42, 0.4];
    kit_block(
        m,
        base + Y(0.06),
        side,
        VY,
        front,
        v(0.5, 0.06, 0.5),
        tint(grey, 0.75),
    );
    kit_block(
        m,
        base + Y(0.47),
        side,
        VY,
        front,
        v(0.42, 0.35, 0.42),
        tint(grey, 1.0),
    );
    kit_block(
        m,
        base + Y(0.86),
        side,
        VY,
        front,
        v(0.46, 0.04, 0.46),
        tint(grey, 1.15),
    );
    let top = base + Y(0.9);
    kit_block(m, top + Y(0.06), side, VY, front, v(0.3, 0.06, 0.3), RED);
    for (sx, sz) in [(-1.0f32, -1.0f32), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        lathe(
            m,
            top + side * (sx * 0.3) + front * (sz * 0.3) + Y(0.06),
            -VY,
            &[(0.0, 0.025), (0.04, 0.03), (0.12, 0.0)],
            6,
            GOLD_TRIM,
            false,
        );
    }
    append(
        m,
        &crown_mesh(),
        &trs_yaw(top + Y(0.12), yaw, 0.78),
        [1.08, 1.02, 0.9],
    );
    for k in 0..5 {
        let a = yaw + k as f32 * TAU / 5.0 + 0.3;
        crystal(
            glow,
            top + v(a.cos() * 0.24, 0.2, a.sin() * 0.24),
            0.035,
            0.08,
            tint(GEMS[k % GEMS.len()], 0.6),
            5,
        );
    }
    for k in 0..14 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.55, 0.95);
        let nx = rng.range(-0.3, 0.3);
        let nz = rng.range(-0.3, 0.3);
        let n = norm(VY + v(nx, 0.0, nz));
        let size = rng.range(0.055, 0.07);
        let color = coin_color(rng);
        coin(
            m,
            base + v(a.cos() * r, 0.012, a.sin() * r),
            n,
            size,
            0.016,
            color,
            k as f32,
        );
    }
}

/// Stone ledge displaying great gems between candles.
pub fn gem_ledge(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let front = yaw_dir(yaw);
    let side = v(front.z, 0.0, -front.x);
    let grey = [0.45, 0.42, 0.39];
    kit_block(
        m,
        base + Y(0.4),
        side,
        VY,
        front,
        v(0.66, 0.4, 0.3),
        tint(grey, 0.92),
    );
    kit_block(
        m,
        base + Y(0.84),
        side,
        VY,
        front,
        v(0.72, 0.05, 0.36),
        tint(grey, 1.15),
    );
    kit_block(
        m,
        base + Y(0.4) + front * 0.302,
        side,
        VY,
        front,
        v(0.5, 0.28, 0.004),
        tint(grey, 0.75),
    );
    let top = base + Y(0.89);
    let colors = [[0.1, 0.45, 1.0], [0.95, 0.08, 0.2], [0.15, 0.85, 0.35]];
    for (k, &(x, r, h)) in [(-0.2, 0.2, 0.42), (0.22, 0.16, 0.36), (0.02, 0.1, 0.24)]
        .iter()
        .enumerate()
    {
        let at = top + side * x + front * 0.02;
        if k == 1 {
            kit_block(m, at + Y(0.04), side, VY, front, v(0.16, 0.04, 0.16), RED);
        }
        let gem_base = at + Y(if k == 1 { 0.08 } else { 0.0 });
        crystal(glow, gem_base, r * 0.62, h, tint(colors[k], 0.62), 6);
        crystal(m, gem_base, r * 0.66, h * 0.97, tint(colors[k], 0.8), 6);
    }
    for x in [-0.58, 0.58] {
        let h = rng.range(0.18, 0.3);
        candle(m, glow, top + side * x, 0.05, h, rng);
    }
    let h = rng.range(0.1, 0.16);
    candle(m, glow, top + side * -0.44 - front * 0.12, 0.04, h, rng);
}

// ------------------------------------------------ shared treasury extras

const BURLAP: C3 = [0.44, 0.33, 0.2];

/// Burlap sack tied at the neck, gold spilling from its mouth.
pub fn coin_sack(m: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let h = rng.range(0.42, 0.55);
    let shades = [0.8, 0.95, 1.05, 1.0, 0.9, 1.05];
    {
        // The web version draws a fresh shade for every ring it colours.
        let cell = RefCell::new(&mut *rng);
        lathe_fn(
            m,
            base,
            VY,
            &[
                (0.0, 0.16),
                (0.08 * h / 0.5, 0.25),
                (0.26 * h / 0.5, 0.27),
                (0.4 * h / 0.5, 0.18),
                (0.46 * h / 0.5, 0.1),
                (0.56 * h / 0.5, 0.15),
            ],
            12,
            &|j| {
                tint(
                    BURLAP,
                    shades[j.min(5)] * cell.borrow_mut().range(0.95, 1.05),
                )
            },
            true,
        );
    }
    tube_ring(
        m,
        base + Y(0.45 * h / 0.5),
        VY,
        0.1,
        0.018,
        16,
        tint(BURLAP, 0.6),
    );
    let mouth = base + Y(0.56 * h / 0.5);
    for k in 0..8 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.0, 0.1);
        let y = rng.range(-0.01, 0.03);
        let nx = rng.range(-0.3, 0.3);
        let nz = rng.range(-0.3, 0.3);
        let color = coin_color(rng);
        coin(
            m,
            mouth + v(a.cos() * r, y, a.sin() * r),
            norm(VY + v(nx, 0.0, nz)),
            0.05,
            0.015,
            color,
            k as f32,
        );
    }
    let front = yaw_dir(yaw);
    for k in 0..12 {
        let t = rng.unit();
        let s = rng.range(-0.18, 0.18);
        let p = base + front * (0.25 + t * 0.45) + v(front.z, 0.0, -front.x) * s;
        let nx = rng.range(-0.3, 0.3);
        let nz = rng.range(-0.3, 0.3);
        let size = rng.range(0.05, 0.065);
        let color = coin_color(rng);
        coin(
            m,
            p + Y(0.012),
            norm(VY + v(nx, 0.0, nz)),
            size,
            0.015,
            color,
            k as f32,
        );
    }
}

/// Royal crest over the entrance: crowned shield on crossed swords.
pub fn royal_crest(m: &mut Mesh3D, glow: &mut Mesh3D, c: Vec3, inward: Vec3, tangent: Vec3) {
    let f = inward;
    let t = tangent;
    let at = |x: f32, y: f32, d: f32| c + t * x + Y(y) + f * d;
    for s in [-1.0f32, 1.0] {
        bar(
            m,
            at(-s * 0.55, -0.6, 0.02),
            at(s * 0.55, 0.62, 0.02),
            0.035,
            0.01,
            norm(f.cross(VY)),
            STEEL,
        );
        bar(
            m,
            at(-s * 0.62, -0.68, 0.03),
            at(-s * 0.5, -0.54, 0.03),
            0.03,
            0.03,
            VY,
            GOLD_TRIM,
        );
        bar(
            m,
            at(-s * 0.4, -0.5, 0.03),
            at(-s * 0.62, -0.32, 0.03),
            0.02,
            0.025,
            f,
            GOLD_TRIM,
        );
    }
    let outline: [(f32, f32); 7] = [
        (-0.42, 0.42),
        (0.42, 0.42),
        (0.42, -0.05),
        (0.3, -0.35),
        (0.0, -0.55),
        (-0.3, -0.35),
        (-0.42, -0.05),
    ];
    let mut face = |scale: f32, d: f32, color: C3| {
        let centre = at(0.0, 0.0, d);
        for i in 0..outline.len() {
            let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
            tri(
                m,
                centre,
                at(a.0 * scale, a.1 * scale, d),
                at(b.0 * scale, b.1 * scale, d),
                f,
                color,
            );
        }
    };
    face(1.08, 0.05, GOLD_TRIM);
    face(0.92, 0.07, RED);
    kit_block(
        m,
        at(0.0, -0.02, 0.085),
        t,
        VY,
        f,
        v(0.06, 0.4, 0.008),
        GOLD_TRIM,
    );
    kit_block(
        m,
        at(0.0, 0.12, 0.085),
        t,
        VY,
        f,
        v(0.3, 0.05, 0.008),
        GOLD_TRIM,
    );
    star(m, at(0.0, 0.12, 0.095), f, VY, 0.13, 0.05, 8, GOLD_TRIM);
    append(
        m,
        &crown_mesh(),
        &trs_yaw(at(0.0, 0.5, 0.06), f.x.atan2(f.z), 0.85),
        [1.1, 1.02, 0.9],
    );
    annulus(
        glow,
        at(0.0, 0.12, 0.1),
        f,
        0.12,
        0.135,
        24,
        [0.35, 0.2, 0.04],
        0.0,
    );
}
