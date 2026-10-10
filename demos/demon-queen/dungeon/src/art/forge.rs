//! La Forja en ruinas (web prototype `galleries/forge.js`): a smithy the
//! order abandoned mid-work — the great forge, anvils with a glowing blade,
//! a smelter pouring into ingot moulds, a quench trough, armour stands, a
//! grindstone, tool boards, coal, ingots and scrap, soot and embers.
use super::kit::*;
use super::mine;
use super::stage::*;
use super::themekit::{ingot_stack, weapon_rack};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const BRICK: C3 = [0.36, 0.2, 0.14];
const SOOT: C3 = [0.07, 0.06, 0.06];
const STEEL: C3 = [0.62, 0.64, 0.68];
const LEATHER: C3 = [0.32, 0.18, 0.1];
const FIRE: C3 = [1.0, 0.45, 0.12];
const MOLTEN: C3 = [1.0, 0.58, 0.16];
const COAL: C3 = [0.09, 0.08, 0.08];

/// The great forge against a wall: brick firebox of glowing coals, a
/// riveted hood and flue, a tool rail and a leather bellows alongside.
#[allow(clippy::too_many_arguments)]
pub fn great_forge(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    hw: f32,
    ceiling: f32,
    bellows_side: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    let d = 1.15;
    for row in 0..7 {
        let y = 0.07 + row as f32 * 0.13;
        let offset = if row % 2 == 1 { 0.14 } else { 0.0 };
        let mut x = -hw + offset;
        while x < hw - 0.05 {
            let w = 0.13f32.min(hw - x);
            let shade = rng.range(0.75, 1.2);
            kit_block(
                m,
                l(x + w, y, d + 0.01),
                tangent,
                VY,
                inward,
                v(w - 0.012, 0.058, 0.05),
                tint(BRICK, shade),
            );
            x += 0.28;
        }
        for sgn in [-1.0, 1.0] {
            let mut z = 0.2;
            while z < d {
                let shade = rng.range(0.75, 1.2);
                kit_block(
                    m,
                    l(sgn * (hw + 0.01), y, z + 0.13 + (row % 2) as f32 * 0.07),
                    inward,
                    VY,
                    tangent,
                    v(0.13, 0.058, 0.05),
                    tint(BRICK, shade),
                );
                z += 0.28;
            }
        }
    }
    chamfer_box(
        m,
        l(0.0, 0.47, d * 0.5),
        tangent,
        VY,
        inward,
        v(hw - 0.04, 0.45, d * 0.5 - 0.04),
        0.03,
        tint(BRICK, 0.5),
    );
    chamfer_box(
        m,
        l(0.0, 0.95, d * 0.5 + 0.02),
        tangent,
        VY,
        inward,
        v(hw + 0.1, 0.05, d * 0.5 + 0.08),
        0.02,
        tint(stone, 0.8),
    );
    for _ in 0..40 {
        let (x, z) = (rng.range(-hw + 0.2, hw - 0.2), rng.range(0.2, d - 0.15));
        let p = l(x, 1.0, z);
        if rng.chance(0.55) {
            let hot = rng.range(0.0, 0.5);
            let phase = rng.range(0.0, TAU);
            rock(
                glow,
                p,
                v(0.07, 0.04, 0.06),
                mix_color(FIRE, [1.0, 0.8, 0.4], hot),
                phase,
            );
        } else {
            let shade = rng.range(0.8, 1.4);
            let phase = rng.range(0.0, TAU);
            rock(
                m,
                p + Y(0.02),
                v(0.08, 0.05, 0.07),
                tint(COAL, shade),
                phase,
            );
        }
    }
    for _ in 0..10 {
        let x = rng.range(-hw, hw);
        let shade = rng.range(0.6, 1.0);
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            l(x, 1.0, d - 0.02),
            v(0.06, 0.04, 0.05),
            tint([0.3, 0.2, 0.16], shade),
            phase,
        );
    }
    let hood_y = 1.75;
    let lo = [
        (-hw - 0.15, d + 0.18),
        (hw + 0.15, d + 0.18),
        (hw + 0.15, 0.02),
        (-hw - 0.15, 0.02),
    ];
    let hi = [(-0.38, 0.62), (0.38, 0.62), (0.38, 0.1), (-0.38, 0.1)];
    let p = |(x, z): (f32, f32), y: f32| l(x, y, z);
    for k in 0..4 {
        let a = p(lo[k], hood_y);
        let b = p(lo[(k + 1) % 4], hood_y);
        let c = p(hi[(k + 1) % 4], hood_y + 1.0);
        let dd = p(hi[k], hood_y + 1.0);
        let n = norm((a + b) * 0.5 - l(0.0, hood_y + 0.5, 0.4));
        quad(m, [a, b, c, dd], n, tint(IRON, 1.15 - k as f32 * 0.05));
        quad(m, [a, b, c, dd], -n, tint(SOOT, 1.0));
        let mut t = 0.1;
        while t < 1.0 {
            let q = a.lerp(b, t) + n * 0.01 + Y(0.05);
            disc(m, q, n, 0.02, 5, tint(IRON, 1.8));
            t += 0.2;
        }
    }
    for k in 0..4 {
        bar(
            m,
            p(lo[k], hood_y),
            p(lo[(k + 1) % 4], hood_y),
            0.035,
            0.035,
            VY,
            tint(IRON, 1.5),
        );
    }
    let flue_top = ceiling;
    kit_block(
        m,
        l(0.0, (hood_y + 1.0 + flue_top) * 0.5, 0.36),
        tangent,
        VY,
        inward,
        v(0.36, (flue_top - hood_y - 1.0) * 0.5, 0.27),
        tint(BRICK, 0.8),
    );
    let mut y = hood_y + 1.2;
    while y < flue_top {
        kit_block(
            m,
            l(0.0, y, 0.64),
            tangent,
            VY,
            inward,
            v(0.37, 0.01, 0.01),
            tint(BRICK, 0.5),
        );
        y += 0.26;
    }
    let rail = l(0.0, hood_y - 0.08, d + 0.24);
    bar(
        m,
        rail - tangent * hw,
        rail + tangent * hw,
        0.018,
        0.018,
        VY,
        tint(IRON, 1.4),
    );
    for k in 0..6 {
        let x = -hw + 0.25 + k as f32 * ((hw * 2.0 - 0.5) / 5.0);
        let hook = rail + tangent * x;
        let len = rng.range(0.55, 0.85);
        let lean = tangent * rng.range(-0.08, 0.08);
        if k % 3 == 0 {
            for sgn in [-1.0, 1.0] {
                bar(
                    m,
                    hook + tangent * (sgn * 0.02),
                    hook + tangent * (sgn * 0.035) - Y(len) + lean,
                    0.008,
                    0.008,
                    inward,
                    tint(IRON, 1.3),
                );
            }
            tube_ring(m, hook - Y(0.04), inward, 0.025, 0.006, 8, tint(IRON, 1.5));
        } else if k % 3 == 1 {
            bar(
                m,
                hook,
                hook - Y(len) + lean,
                0.008,
                0.008,
                inward,
                tint(IRON, 1.4),
            );
            bar(
                m,
                hook - Y(len) + lean,
                hook - Y(len - 0.12) + lean + tangent * 0.08,
                0.008,
                0.008,
                inward,
                tint(IRON, 1.4),
            );
        } else {
            bar(
                m,
                hook,
                hook - Y(len * 0.7),
                0.014,
                0.014,
                inward,
                tint(WOOD, 1.1),
            );
            kit_block(
                m,
                hook - Y(len * 0.7),
                tangent,
                VY,
                inward,
                v(0.08, 0.035, 0.035),
                tint(IRON, 1.5),
            );
        }
    }
    let side = bellows_side;
    let bel = l(side * (hw + 0.62), 0.55, d * 0.55);
    for k in 0..5 {
        let t = k as f32 / 4.0;
        let w = 0.3 - (t - 0.5).abs() * 0.16;
        chamfer_box(
            m,
            bel + Y(t * 0.34 - 0.17),
            tangent,
            VY,
            inward,
            v(0.5 * (1.0 - t * 0.2), 0.03, w),
            0.015,
            if k % 2 == 1 {
                tint(LEATHER, 0.8)
            } else {
                tint(LEATHER, 1.1)
            },
        );
    }
    chamfer_box(
        m,
        bel + Y(0.2),
        tangent,
        VY,
        inward,
        v(0.56, 0.025, 0.32),
        0.01,
        tint(WOOD, 1.1),
    );
    chamfer_box(
        m,
        bel - Y(0.2),
        tangent,
        VY,
        inward,
        v(0.56, 0.025, 0.32),
        0.01,
        tint(WOOD, 0.95),
    );
    lathe(
        m,
        bel + tangent * (-side * 0.55),
        tangent * -side,
        &[(0.0, 0.06), (0.2, 0.035), (0.35, 0.025)],
        8,
        tint(IRON, 1.3),
        true,
    );
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            bel + tangent * (sgn * 0.4) - Y(0.22),
            bel + tangent * (sgn * 0.4) - Y(0.55),
            0.04,
            0.04,
            inward,
            tint(WOOD, 0.9),
        );
    }
    bar(
        m,
        bel + Y(0.22) + tangent * (side * 0.5),
        bel + Y(0.75) + tangent * (side * 0.95),
        0.022,
        0.022,
        inward,
        tint(WOOD, 1.2),
    );
}

/// An anvil on a tree stump with hammers, tongs and a red-hot blade.
pub fn anvil_station(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    lathe_fn(
        m,
        l(0.0, 0.0, 0.0),
        VY,
        &[
            (0.0, 0.36),
            (0.05, 0.31),
            (0.3, 0.28),
            (0.52, 0.29),
            (0.54, 0.27),
        ],
        12,
        &|j| tint([0.3, 0.2, 0.12], 0.8 + j as f32 * 0.06),
        true,
    );
    disc(m, l(0.0, 0.545, 0.0), VY, 0.26, 12, [0.46, 0.34, 0.2]);
    for r in [0.08, 0.15, 0.22] {
        tube_ring(m, l(0.0, 0.547, 0.0), VY, r, 0.004, 14, [0.32, 0.22, 0.14]);
    }
    let top = 0.55;
    kit_block(
        m,
        l(0.0, top + 0.07, 0.0),
        s,
        VY,
        f,
        v(0.14, 0.07, 0.1),
        tint(IRON, 1.4),
    );
    kit_block(
        m,
        l(0.0, top + 0.17, 0.0),
        s,
        VY,
        f,
        v(0.09, 0.05, 0.07),
        tint(IRON, 1.5),
    );
    kit_block(
        m,
        l(0.0, top + 0.27, 0.0),
        s,
        VY,
        f,
        v(0.26, 0.06, 0.11),
        tint(IRON, 1.8),
    );
    lathe(
        m,
        l(0.26, top + 0.27, 0.0),
        s,
        &[(0.0, 0.09), (0.1, 0.06), (0.24, 0.012)],
        7,
        tint(IRON, 1.7),
        true,
    );
    kit_block(
        m,
        l(-0.3, top + 0.28, 0.0),
        s,
        VY,
        f,
        v(0.05, 0.045, 0.09),
        tint(IRON, 1.7),
    );
    kit_block(
        m,
        l(0.0, top + 0.334, 0.0),
        s,
        VY,
        f,
        v(0.24, 0.004, 0.1),
        [0.72, 0.72, 0.75],
    );
    let blade = l(-0.05, top + 0.345, 0.02);
    quad(
        glow,
        [
            blade - s * 0.22 - f * 0.025,
            blade + s * 0.2 - f * 0.02,
            blade + s * 0.26,
            blade - s * 0.22 + f * 0.025,
        ],
        VY,
        tint(MOLTEN, 0.75),
    );
    bar(
        m,
        blade - s * 0.22,
        blade - s * 0.5 + f * 0.05,
        0.012,
        0.01,
        VY,
        tint(IRON, 1.3),
    );
    let hammer = l(0.05, 0.56, -0.12);
    bar(
        m,
        hammer,
        hammer + s * 0.3 + f * -0.1,
        0.016,
        0.016,
        VY,
        tint(WOOD, 1.1),
    );
    kit_block(
        m,
        hammer + Y(0.035),
        s,
        VY,
        f,
        v(0.035, 0.035, 0.07),
        tint(IRON, 1.5),
    );
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            l(0.24, 0.56, 0.12),
            l(-0.04, 0.565, 0.16 + sgn * 0.03),
            0.008,
            0.008,
            VY,
            tint(IRON, 1.3),
        );
    }
    pebbles(m, l(0.0, 0.0, 0.0), 0.7, 10, rng, [0.3, 0.3, 0.32]);
    blob(m, l(0.0, 0.0, 0.0), 0.8, SOOT, rng, 0.006, 14, 0.4);
}

/// The smelter: a round brick kiln with a glowing mouth and a spout.
pub fn smelter(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    lathe_fn(
        m,
        l(0.0, 0.0, 0.0),
        VY,
        &[
            (0.0, 0.62),
            (0.15, 0.6),
            (0.9, 0.5),
            (1.35, 0.38),
            (1.55, 0.3),
            (1.6, 0.26),
        ],
        14,
        &|j| tint(BRICK, 0.7 + j as f32 * 0.06),
        true,
    );
    let mut y = 0.15;
    while y < 1.5 {
        tube_ring(
            m,
            l(0.0, y, 0.0),
            VY,
            0.6 - y * 0.2 + 0.005,
            0.006,
            20,
            tint(BRICK, 0.45),
        );
        y += 0.14;
    }
    for y in [0.35f32, 1.0] {
        tube_ring(
            m,
            l(0.0, y, 0.0),
            VY,
            0.6 - y * 0.2 + 0.02,
            0.018,
            20,
            tint(IRON, 1.3),
        );
    }
    let mouth = l(0.0, 0.5, 0.56);
    disc(glow, mouth, f, 0.2, 10, tint(FIRE, 0.9));
    disc(glow, mouth + f * 0.004, f, 0.12, 10, [1.0, 0.85, 0.5]);
    tube_ring(m, mouth + f * 0.01, f, 0.22, 0.03, 12, [0.32, 0.3, 0.29]);
    lathe(
        m,
        l(0.0, 1.6, 0.0),
        VY,
        &[(0.0, 0.24), (0.3, 0.2), (0.5, 0.2)],
        10,
        tint(BRICK, 0.6),
        false,
    );
    disc(glow, l(0.0, 1.62, 0.0), VY, 0.18, 10, tint(FIRE, 0.6));
    let lip = l(0.0, 0.8, 0.5);
    bar(
        m,
        lip,
        lip + f * 0.35 - Y(0.12),
        0.08,
        0.03,
        VY,
        tint(IRON, 1.2),
    );
    tube(
        glow,
        &[
            lip + f * 0.34 - Y(0.1),
            lip + f * 0.42 - Y(0.4),
            lip + f * 0.44 - Y(0.76),
        ],
        &|t| 0.035 + t * 0.01,
        6,
        &|_, _| tint(MOLTEN, 0.85),
    );
}

/// A stone channel of molten metal ending at a row of ingot moulds.
pub fn molten_channel(m: &mut Mesh3D, glow: &mut Mesh3D, pts: &[Vec3], stone: C3) {
    for k in 0..pts.len() - 1 {
        let (a, b) = (pts[k], pts[k + 1]);
        let d = norm(hz(b - a));
        let n = v(-d.z, 0.0, d.x);
        for sgn in [-1.0, 1.0] {
            bar(
                m,
                a + n * (sgn * 0.12) + Y(0.05),
                b + n * (sgn * 0.12) + Y(0.05),
                0.05,
                0.05,
                VY,
                tint(stone, 0.7 - sgn * 0.05),
            );
        }
        quad(
            glow,
            [
                a + n * 0.075 + Y(0.06),
                b + n * 0.075 + Y(0.06),
                b - n * 0.075 + Y(0.06),
                a - n * 0.075 + Y(0.06),
            ],
            VY,
            tint(MOLTEN, 0.62 + 0.1 * (k as f32).sin()),
        );
    }
    let end = pts[pts.len() - 1];
    let d = norm(hz(end - pts[pts.len() - 2]));
    let n = v(-d.z, 0.0, d.x);
    for k in -1..=1 {
        let c = end + d * 0.4 + n * (k as f32 * 0.34);
        chamfer_box(
            m,
            c + Y(0.06),
            n,
            VY,
            d,
            v(0.14, 0.06, 0.3),
            0.01,
            tint(IRON, 1.1),
        );
        let q = [
            c + n * -0.1 + d * -0.24 + Y(0.125),
            c + n * 0.1 + d * -0.24 + Y(0.125),
            c + n * 0.1 + d * 0.24 + Y(0.125),
            c + n * -0.1 + d * 0.24 + Y(0.125),
        ];
        if k == 0 {
            quad(glow, q, VY, tint(MOLTEN, 0.7));
        } else {
            quad(m, q, VY, tint([0.44, 0.38, 0.34], 1.2));
        }
    }
}

/// A hollow stone quench trough holding dark water, tongs dipped in it.
pub fn quench_trough(
    m: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    stone: C3,
    wet: &mut Vec<super::plan::Pond>,
) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    let st = tint(stone, 0.72);
    chamfer_box(
        m,
        l(0.0, 0.07, 0.0),
        s,
        VY,
        f,
        v(0.36, 0.07, 0.75),
        0.03,
        tint(st, 0.9),
    );
    for sgn in [-1.0, 1.0] {
        chamfer_box(
            m,
            l(sgn * 0.315, 0.3, 0.0),
            s,
            VY,
            f,
            v(0.045, 0.26, 0.75),
            0.02,
            tint(st, 1.02),
        );
        chamfer_box(
            m,
            l(0.0, 0.3, sgn * 0.705),
            s,
            VY,
            f,
            v(0.27, 0.26, 0.045),
            0.02,
            tint(st, 0.96),
        );
    }
    quad(
        m,
        [
            l(-0.27, 0.141, -0.66),
            l(0.27, 0.141, -0.66),
            l(0.27, 0.141, 0.66),
            l(-0.27, 0.141, 0.66),
        ],
        VY,
        tint(st, 0.35),
    );
    let level = 0.46;
    let basin = [
        l(-0.268, level, -0.658),
        l(0.268, level, -0.658),
        l(0.268, level, 0.658),
        l(-0.268, level, 0.658),
    ];
    pond(
        wet,
        &basin,
        base.y + level,
        [0.035, 0.045, 0.05],
        CAVE_REFLECTION,
        0.75,
        1.0,
        None,
    );
    for k in 0..3 {
        let x = -0.16 + k as f32 * 0.14;
        bar(
            m,
            l(x, 0.3, 0.28 - k as f32 * 0.12),
            l(x + 0.04 * (k as f32 - 1.0), 0.64, -0.78),
            0.011,
            0.009,
            VY,
            tint(IRON, 1.3),
        );
    }
}

/// A post and crossbar wearing a breastplate, pauldrons and a helm.
pub fn armor_stand(m: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    kit_block(
        m,
        l(0.0, 0.04, 0.0),
        s,
        VY,
        f,
        v(0.28, 0.04, 0.28),
        tint(WOOD, 0.8),
    );
    bar(
        m,
        l(0.0, 0.08, 0.0),
        l(0.0, 1.7, 0.0),
        0.04,
        0.04,
        f,
        tint(WOOD, 1.0),
    );
    bar(
        m,
        l(-0.36, 1.42, 0.0),
        l(0.36, 1.42, 0.0),
        0.035,
        0.035,
        VY,
        tint(WOOD, 1.1),
    );
    let steel = tint(STEEL, rng.range(0.7, 0.95));
    grid_surface(
        m,
        6,
        16,
        &|i, j| {
            let y = 0.95 + (i as f32 / 6.0) * 0.5;
            let a = (j as f32 / 16.0) * TAU;
            let w = 0.2 + 0.06 * ((i as f32 / 6.0) * std::f32::consts::PI).sin();
            l(a.cos() * w, y, a.sin() * w * 0.7 + 0.02)
        },
        &|_, j| tint(steel, 0.8 + 0.25 * ((j as f32 / 16.0) * TAU).sin()),
        true,
        Some(&|p, _, _| p - l(0.0, p.y - base.y, 0.0)),
        None,
    );
    bar(
        m,
        l(0.0, 1.43, 0.17),
        l(0.0, 1.0, 0.18),
        0.012,
        0.012,
        VY,
        tint(steel, 1.2),
    );
    for sgn in [-1.0, 1.0] {
        lathe(
            m,
            l(sgn * 0.28, 1.38, 0.02),
            norm(s * sgn + Y(0.5)),
            &[(0.0, 0.13), (0.06, 0.11), (0.1, 0.05), (0.12, 0.0)],
            9,
            tint(steel, 1.05),
            true,
        );
    }
    lathe(
        m,
        l(0.0, 1.62, 0.0),
        VY,
        &[
            (0.0, 0.13),
            (0.04, 0.14),
            (0.18, 0.13),
            (0.26, 0.08),
            (0.3, 0.0),
        ],
        11,
        tint(steel, 1.0),
        true,
    );
    kit_block(
        m,
        l(0.0, 1.72, 0.13),
        s,
        VY,
        f,
        v(0.08, 0.008, 0.01),
        [0.05, 0.05, 0.06],
    );
    bar(
        m,
        l(0.0, 1.9, 0.0),
        l(0.0, 1.94, -0.18),
        0.01,
        0.04,
        s,
        [0.55, 0.1, 0.08],
    );
    for sgn in [-1.0, 1.0] {
        kit_block(
            m,
            l(sgn * 0.12, 0.92, 0.1),
            s,
            VY,
            f,
            v(0.09, 0.05, 0.04),
            tint(steel, 0.9),
        );
    }
}

/// A pegboard of tools hung on a wall bay.
pub fn tool_board(
    m: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    rng: &mut ArtRng,
) {
    let w = width * 0.5;
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    kit_block(
        m,
        l(0.0, 1.55, 0.04),
        tangent,
        VY,
        inward,
        v(w, 0.55, 0.03),
        tint(WOOD, 0.8),
    );
    for y in [1.05, 2.05] {
        kit_block(
            m,
            l(0.0, y, 0.08),
            tangent,
            VY,
            inward,
            v(w + 0.04, 0.04, 0.03),
            tint(WOOD, 1.0),
        );
    }
    let mut x = -w + 0.18;
    while x < w - 0.1 {
        let top = l(x, rng.range(1.75, 1.95), 0.1);
        bar(
            m,
            top,
            top + inward * 0.06,
            0.008,
            0.008,
            VY,
            tint(IRON, 1.4),
        );
        let kind = rng.index(4);
        let hang = top + inward * 0.05;
        match kind {
            0 => {
                bar(
                    m,
                    hang,
                    hang - Y(0.45),
                    0.014,
                    0.014,
                    inward,
                    tint(WOOD, 1.1),
                );
                kit_block(
                    m,
                    hang - Y(0.48),
                    tangent,
                    VY,
                    inward,
                    v(0.07, 0.035, 0.03),
                    tint(IRON, 1.5),
                );
            }
            1 => {
                for sgn in [-1.0, 1.0] {
                    bar(
                        m,
                        hang + tangent * (sgn * 0.015),
                        hang + tangent * (sgn * 0.04) - Y(0.55),
                        0.007,
                        0.007,
                        inward,
                        tint(IRON, 1.3),
                    );
                }
            }
            2 => {
                bar(
                    m,
                    hang,
                    hang - Y(0.4),
                    0.01,
                    0.004,
                    inward,
                    tint(STEEL, 1.1),
                );
                bar(
                    m,
                    hang - Y(0.4),
                    hang - Y(0.52),
                    0.012,
                    0.012,
                    inward,
                    tint(WOOD, 1.2),
                );
            }
            _ => tube_ring(m, hang - Y(0.08), inward, 0.07, 0.01, 12, tint(IRON, 1.4)),
        }
        x += rng.range(0.2, 0.3);
    }
}

/// Grindstone wheel on a wooden frame with a crank and a water tray.
pub fn grindstone(m: &mut Mesh3D, base: Vec3, yaw: f32, stone: C3) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    for sgn in [-1.0, 1.0] {
        for z in [-0.25, 0.25] {
            bar(
                m,
                l(sgn * 0.2, 0.0, z),
                l(sgn * 0.14, 0.62, z * 0.4),
                0.03,
                0.03,
                s,
                tint(WOOD, 0.95),
            );
        }
    }
    let hub = l(0.0, 0.66, 0.0);
    lathe(
        m,
        hub - s * 0.06,
        s,
        &[(0.0, 0.34), (0.12, 0.34)],
        16,
        tint(stone, 1.05),
        true,
    );
    disc(m, hub + s * 0.061, s, 0.08, 8, tint(WOOD, 1.1));
    bar(
        m,
        hub - s * 0.24,
        hub + s * 0.3,
        0.02,
        0.02,
        VY,
        tint(IRON, 1.4),
    );
    bar(
        m,
        hub + s * 0.3,
        hub + s * 0.3 + Y(-0.2),
        0.016,
        0.016,
        s,
        tint(IRON, 1.4),
    );
    bar(
        m,
        hub + s * 0.3 + Y(-0.2),
        hub + s * 0.42 + Y(-0.2),
        0.02,
        0.02,
        VY,
        tint(WOOD, 1.2),
    );
    chamfer_box(
        m,
        l(0.0, 0.34, 0.0),
        s,
        VY,
        f,
        v(0.16, 0.04, 0.3),
        0.01,
        tint(WOOD, 0.8),
    );
}

/// Coal heap with a few embers still alive.
pub fn coal_heap(m: &mut Mesh3D, glow: &mut Mesh3D, center: Vec3, r: f32, rng: &mut ArtRng) {
    for _ in 0..(r * 40.0).round() as usize {
        let a = rng.range(0.0, TAU);
        let d = r * rng.unit64().sqrt() as f32;
        let h = (1.0 - d / r) * r * 0.5;
        let q = center + v(a.cos() * d, h * rng.range(0.5, 1.0), a.sin() * d);
        if rng.chance(0.06) {
            rock(glow, q, v(0.05, 0.035, 0.045), tint(FIRE, 0.7), a);
        } else {
            let shade = rng.range(0.8, 1.6);
            rock(m, q, v(0.07, 0.05, 0.06), tint(COAL, shade), a);
        }
    }
}

/// Scrap: broken blades, chain links, rings and a dented helm.
pub fn scrap_pile(m: &mut Mesh3D, center: Vec3, radius: f32, rng: &mut ArtRng) {
    for _ in 0..14 {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.unit64().sqrt() as f32;
        let p = center + v(a.cos() * d, 0.02 + rng.range(0.0, 0.08), a.sin() * d);
        let kind = rng.index(4);
        let (d1, dy, d2) = (
            rng.range(0.0, TAU),
            rng.range(-0.2, 0.2),
            rng.range(0.0, TAU),
        );
        let dir = norm(v(d1.cos(), dy, d2.sin()));
        match kind {
            0 => {
                let len = rng.range(0.25, 0.5);
                let shade = rng.range(0.6, 0.9);
                bar(m, p, p + dir * len, 0.03, 0.006, VY, tint(STEEL, shade));
            }
            1 => {
                let (x, z) = (rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
                tube_ring(m, p, norm(v(x, 1.0, z)), 0.04, 0.01, 8, tint(IRON, 1.5));
            }
            2 => {
                let shade = rng.range(0.8, 1.2);
                kit_block(
                    m,
                    p,
                    dir,
                    VY,
                    v(-dir.z, 0.0, dir.x),
                    v(0.12, 0.025, 0.05),
                    tint([0.44, 0.38, 0.34], shade),
                );
            }
            _ => {
                let (x, z) = (rng.range(-0.6, 0.6), rng.range(-0.6, 0.6));
                lathe(
                    m,
                    p,
                    norm(v(x, 1.0, z)),
                    &[(0.0, 0.12), (0.08, 0.12), (0.16, 0.06), (0.18, 0.0)],
                    9,
                    tint(STEEL, 0.7),
                    true,
                );
            }
        }
    }
}

/// Hanging chain hoist with a hook, over the anvils.
pub fn hoist(m: &mut Mesh3D, top: Vec3, length: f32, rng: &mut ArtRng) {
    let end = chain(m, top, length, rng, IRON);
    lathe(
        m,
        end - Y(0.08),
        VY,
        &[(0.0, 0.08), (0.08, 0.08)],
        10,
        tint(IRON, 1.3),
        true,
    );
    tube_c(
        m,
        &bezier(
            end - Y(0.1),
            end - Y(0.3) + v(0.12, 0.0, 0.0),
            end - Y(0.25) + v(-0.04, 0.0, 0.0),
            8,
        ),
        0.02,
        5,
        tint(IRON, 1.5),
    );
}

/// Tools and half-finished work laid out on a bench top.
pub fn bench_tools(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, z: f32| base + s * x + f * z;
    let hammer = l(-0.5, -0.15);
    bar(
        m,
        hammer + Y(0.02),
        hammer + Y(0.02) + s * 0.32,
        0.014,
        0.014,
        VY,
        tint(WOOD, 1.1),
    );
    kit_block(
        m,
        hammer + Y(0.04),
        f,
        VY,
        s,
        v(0.07, 0.035, 0.035),
        tint(IRON, 1.5),
    );
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            l(0.05, 0.1) + Y(0.012),
            l(0.45, 0.14 + sgn * 0.04) + Y(0.012),
            0.008,
            0.008,
            VY,
            tint(IRON, 1.3),
        );
    }
    for k in 0..5 {
        bar(
            m,
            l(0.6 + k as f32 * 0.05, -0.2) + Y(0.01),
            l(0.6 + k as f32 * 0.05, 0.05) + Y(0.01),
            0.008,
            0.006,
            VY,
            tint(STEEL, 1.0),
        );
    }
    lathe(
        m,
        l(-0.1, -0.1),
        VY,
        &[
            (0.0, 0.13),
            (0.04, 0.14),
            (0.16, 0.13),
            (0.22, 0.08),
            (0.25, 0.0),
        ],
        10,
        tint(STEEL, 0.8),
        true,
    );
    quad(
        glow,
        [
            l(-0.9, 0.1) + Y(0.012),
            l(-0.55, 0.12) + Y(0.012),
            l(-0.52, 0.15) + Y(0.012),
            l(-0.9, 0.16) + Y(0.012),
        ],
        VY,
        tint(MOLTEN, 0.35),
    );
}

// ---------------------------------------------------------------- planner

/// Plan the ruined forge into the plan held by the stage.
pub fn plan(s: &mut Stage) {
    let fire = FIRE;
    // 1. The great forge on the far wall (or the first wall with room for it).
    let mut forge_at: Option<(Vec3, Edge)> = None;
    let mut walls = s.walls.clone();
    walls.sort_by_key(|w| mm(w.inward.dot(s.inward)));
    'walls: for wall in &walls {
        for shift in [0.0f32, 1.4, -1.4, 2.4, -2.4] {
            let center = wall.mid + wall.tangent * shift;
            if shift.abs() + 2.2 > wall.length * 0.5 {
                continue;
            }
            if wall
                .doors
                .iter()
                .any(|d| (d.p - center).dot(wall.tangent).abs() < d.width * 0.5 + 2.3)
            {
                continue;
            }
            let circles: Vec<Vec3> = [-0.55f32, 0.55]
                .iter()
                .map(|&x| s.floor(center + wall.tangent * x + wall.inward * 0.62))
                .collect();
            if circles
                .iter()
                .any(|&p| s.crowded(p, 0.6, 0.05) || !s.placer.clear(p, 0.6, 1.5, 0.7))
            {
                continue;
            }
            forge_at = Some((s.floor(center), wall.clone()));
            for &p in &circles {
                s.solid(p, 0.62, true);
            }
            break 'walls;
        }
    }
    if let Some((center, wall)) = forge_at {
        let bellows_side = if s.rng.chance(0.5) { 1.0 } else { -1.0 };
        let (inw, tan) = (wall.inward, wall.tangent);
        let ceiling = s.c.y + s.height + 0.3;
        s.plan.wall_pieces.push(center);
        s.piece(move |m, g, ctx| {
            great_forge(
                m,
                g,
                center,
                inw,
                tan,
                0.95,
                ceiling,
                bellows_side,
                ctx.stone,
                &mut ctx.rng,
            )
        });
        let bel = s.floor(center + tan * (bellows_side * 1.57) + inw * 0.63);
        s.solid(bel, 0.45, true);
        let near: Vec<Slot> = s
            .bays
            .iter()
            .filter(|bay| hdist(bay.center, center) < bay.width * 0.5 + 1.2)
            .cloned()
            .collect();
        for bay in &near {
            s.take_bay(bay);
        }
        let coals = center + inw * 0.6 + Y(1.2);
        s.flame(coals + tan * -0.35, fire, 1.4);
        s.flame(coals + tan * 0.35, fire, 1.2);
        s.light(coals, fire, 6.5, 9.0);
        s.light(center + inw * 1.8 + Y(0.6), fire, 2.2, 5.0);
        for (dx, dz) in [(-1.3f32, 2.4f32), (1.3, 2.5), (0.0, 3.1)] {
            let Some(p) = s.place(&[center + tan * dx + inw * dz], 0.45, 1.4, 0.7) else {
                continue;
            };
            s.plan.solids.push((p, 0.45));
            let yaw = yaw_of(tan) + s.rng.range(-0.3, 0.3);
            s.piece(move |m, g, ctx| anvil_station(m, g, p, yaw, &mut ctx.rng));
            s.light(p + Y(1.0), [1.0, 0.5, 0.18], 1.2, 3.0);
            if s.rng.chance(0.6) {
                let top = s.top(p) + tan * 0.3;
                let length = (s.height - 2.3).max(0.8);
                s.piece(move |m, _g, ctx| hoist(m, top, length, &mut ctx.rng));
            }
        }
    }
    // 2. The smelter and its molten channel to the ingot moulds.
    let cands = [
        s.at(s.depth * 0.25, s.width * 0.55),
        s.at(s.depth * 0.25, -s.width * 0.55),
        s.at(-s.depth * 0.2, s.width * 0.58),
        s.at(-s.depth * 0.2, -s.width * 0.58),
    ];
    if let Some(smelter_at) = s.place(&cands, 0.66, 1.8, 0.8) {
        s.plan.solids.push((smelter_at, 0.66));
        let to_centre = norm(hz(s.c - smelter_at));
        let yaw = yaw_of(to_centre);
        s.piece(move |m, g, _ctx| smelter(m, g, smelter_at, yaw));
        let start = smelter_at + to_centre * 1.0;
        let side = v(-to_centre.z, 0.0, to_centre.x) * if s.rng.chance(0.5) { 1.0 } else { -1.0 };
        let end = start + to_centre * 1.1 + side * 1.0;
        let pts: Vec<Vec3> = bezier(start, start + to_centre * 0.9, end, 8)
            .into_iter()
            .map(|p| s.floor(p))
            .collect();
        if pts.iter().all(|&p| s.inside(p, 0.4)) {
            s.piece(move |m, g, ctx| molten_channel(m, g, &pts, ctx.stone));
        }
        s.light(smelter_at + to_centre * 0.8 + Y(0.7), fire, 3.2, 6.0);
        s.flame(smelter_at + Y(1.7), fire, 0.9);
    }
    // 3. Wall pieces: tool boards, armour, a shield wall; quench trough, grindstone.
    let mut boards = 0;
    for bay in s.free_bays(None, 1.6) {
        if boards >= 3 {
            break;
        }
        let foot = s.floor(bay.center + bay.inward * 0.35);
        if s.crowded(foot, 0.3, 0.05) {
            continue;
        }
        s.take_bay(&bay);
        if boards == 1 {
            for k in [-0.6f32, 0.0, 0.6] {
                s.mount(
                    if k == 0.0 {
                        "gold_relic"
                    } else {
                        "broken_relic"
                    },
                    bay.center + bay.tangent * k + bay.inward * 0.03 + Y(1.7 + k.abs() * 0.3),
                    yaw_of(bay.inward),
                    0.85,
                );
            }
        } else {
            let (c, inw, tan) = (bay.center, bay.inward, bay.tangent);
            let width = 2.2f32.min(bay.width - 0.6);
            s.plan.wall_pieces.push(c);
            s.piece(move |m, _g, ctx| tool_board(m, c, inw, tan, width, &mut ctx.rng));
        }
        s.solid(foot, 0.2, true);
        boards += 1;
    }
    let mut pilasters = s.pilasters.clone();
    s.rng.shuffle(&mut pilasters);
    for pilaster in pilasters.iter().take(3) {
        let Some(p) = s.place(&[pilaster.base + pilaster.inward * 0.95], 0.4, 1.4, 0.7) else {
            continue;
        };
        s.plan.solids.push((p, 0.4));
        let yaw = yaw_of(pilaster.inward);
        s.piece(move |m, _g, ctx| armor_stand(m, p, yaw, &mut ctx.rng));
    }
    let cands = [
        s.at(-s.depth * 0.35, s.width * 0.5),
        s.at(-s.depth * 0.35, -s.width * 0.5),
        s.at(s.depth * 0.1, s.width * 0.6),
    ];
    let yaw_in = yaw_of(s.inward);
    if let Some(trough) = s.place_row(&cands, yaw_in, 0.78, 0.38, 1.4, 0.7) {
        s.piece(move |m, _g, ctx| {
            let stone = ctx.stone;
            quench_trough(m, trough, yaw_in, stone, &mut ctx.wet);
        });
    }
    let cands = [
        s.at(-s.depth * 0.55, s.width * 0.35),
        s.at(-s.depth * 0.55, -s.width * 0.35),
        s.at(s.depth * 0.5, -s.width * 0.4),
    ];
    if let Some(grind) = s.place(&cands, 0.42, 1.4, 0.7) {
        s.plan.solids.push((grind, 0.42));
        let yaw = s.rng.range(0.0, TAU);
        s.piece(move |m, _g, ctx| grindstone(m, grind, yaw, ctx.stone));
    }
    // 4. Stores: coal, ingots, scrap, barrels, weapon racks.
    let mut corners = s.corners.clone();
    s.rng.shuffle(&mut corners);
    for corner in &corners {
        let cands: Vec<Vec3> = [1.2, 1.6]
            .iter()
            .map(|&d| corner.mid + corner.inward * d)
            .collect();
        let Some(p) = s.place(&cands, 0.6, 1.2, 0.6) else {
            continue;
        };
        s.plan.solids.push((p, 0.6));
        match s.rng.index(3) {
            0 => s.piece(move |m, g, ctx| coal_heap(m, g, p, 0.7, &mut ctx.rng)),
            1 => {
                s.piece(move |m, _g, ctx| scrap_pile(m, p, 0.55, &mut ctx.rng));
                let base = p + corner.tangent * 0.3;
                let yaw = s.rng.range(0.0, TAU);
                s.piece(move |m, _g, ctx| ingot_stack(m, base, yaw, &mut ctx.rng));
            }
            _ => {
                let yaw = s.rng.range(0.0, TAU);
                s.prop("barrel_large", p, yaw, 0.95, 0.0);
                let yaw = s.rng.range(0.0, TAU);
                s.prop("keg_deco", p + corner.tangent * 0.62, yaw, 0.75, 0.0);
            }
        }
    }
    let ring = |a: f32, walk: f32| Scatter {
        walkway: walk,
        ring: [a, 1.0],
        ..Scatter::default()
    };
    s.scatter(4, 0.4, ring(0.5, 0.6), &mut |s, p, _| {
        s.piece(move |m, _g, ctx| scrap_pile(m, p, 0.35, &mut ctx.rng))
    });
    let cands = [
        s.at(-s.depth * 0.5, 0.55 * s.width),
        s.at(-s.depth * 0.5, -0.55 * s.width),
        s.at(s.depth * 0.45, 0.55 * s.width),
        s.at(s.depth * 0.45, -0.55 * s.width),
    ];
    if let Some(bench) = s.place_row(&cands, yaw_in, 0.95, 0.45, 1.5, 0.7) {
        let right = s.right;
        s.prop("table_long", bench, yaw_of(right), 0.9, 0.0);
        let top = bench + Y(0.9);
        s.piece(move |m, g, _ctx| bench_tools(m, g, top, yaw_in));
        s.prop("candle_melted", top + s.inward * 0.6, 0.0, 0.18, 0.0);
        s.light(top + Y(0.5), [1.0, 0.62, 0.3], 1.2, 3.5);
        let offset = if (bench - s.c).dot(right) > 0.0 {
            -0.75
        } else {
            0.75
        };
        if let Some(stool) = s.spot(bench + right * offset, 0.22, 0.3) {
            let yaw = s.rng.range(0.0, TAU);
            s.prop("stool", stool, yaw, 0.45, 0.0);
        }
    }
    s.scatter(3, 0.3, ring(0.45, 0.8), &mut |s, p, _| {
        let yaw = s.rng.range(0.0, TAU);
        s.piece(move |m, _g, ctx| ingot_stack(m, p, yaw, &mut ctx.rng));
    });
    s.scatter(4, 0.3, ring(0.55, 0.8), &mut |s, p, _| {
        let h = s.rng.range(0.45, 0.6);
        s.piece(move |m, _g, ctx| mine::sack(m, p, h, [0.22, 0.2, 0.19], &mut ctx.rng));
    });
    s.scatter(3, 0.2, ring(0.5, 0.6), &mut |s, p, _| {
        s.piece(move |m, g, ctx| mine::bucket(m, g, p, false, ctx))
    });
    let mut pilasters = s.pilasters.clone();
    s.rng.shuffle(&mut pilasters);
    for pilaster in &pilasters {
        let cands = [
            pilaster.base + pilaster.inward * 0.9 + pilaster.tangent * 0.9,
            pilaster.base + pilaster.inward * 0.9 - pilaster.tangent * 0.9,
        ];
        let Some(p) = s.place(&cands, 0.45, 1.4, 0.6) else {
            continue;
        };
        let asset = if s.rng.chance(0.5) {
            "barrel_large"
        } else {
            "keg_deco"
        };
        let yaw = s.rng.range(0.0, TAU);
        let h = s.rng.range(0.75, 0.95);
        s.prop(asset, p, yaw, h, 0.42);
        if s.plan.props.len() > 10 {
            break;
        }
    }
    for bay in s.free_bays(None, 1.5).into_iter().take(2) {
        let Some(p) = s.place(&[bay.center + bay.inward * 0.6], 0.62, 1.4, 0.6) else {
            continue;
        };
        s.take_bay(&bay);
        s.plan.solids.push((p, 0.62));
        let yaw = yaw_of(bay.inward) + std::f32::consts::PI;
        s.piece(move |m, _g, _ctx| weapon_rack(m, p, yaw));
    }
    // 5. Soot and scorch marks, metal chips, embers in the air.
    let floor_only = Scatter {
        walkway: 0.0,
        ..Scatter::default()
    };
    s.scatter(12, 0.4, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.22, 0.5);
        s.stain(
            p,
            r,
            mix_color(SOOT, [0.3, 0.26, 0.22], 0.5),
            None,
            Some(0.55),
        );
    });
    s.scatter(10, 0.3, floor_only, &mut |s, p, _| {
        s.pebbles(p, 0.35, 8, Some([0.3, 0.3, 0.33]))
    });
    let top = s.c + Y(s.height - 1.2);
    s.light(top, [1.0, 0.5, 0.2], 1.2, 9.0);
    s.plan.sparkle_tint = Some([1.8, 0.7, 0.25]);
    let h = s.height;
    s.sparkles(34, 0.4, h - 0.8, None, None);
}
