//! Room incidents (web prototype `incidents.js`): one or two things that
//! happened to a room, so no two rooms of the same kind read alike — a
//! collapse under a hole in the vault, a floor split by a crack (glowing with
//! lava in the fire rooms), a leak dripping into a puddle, a row of wall
//! niches with offerings, a torn banner, a dead brazier, a cluster of
//! stalactites, a drain grate. Each fits a theme list.
use super::super::themes::Theme;
use super::kit::*;
use super::plan::*;
use super::stage::*;
use super::themekit::skull;
use std::f32::consts::PI;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Collapse,
    Crack,
    Lava,
    Leak,
    Niches,
    Banner,
    Brazier,
    Stalactites,
    Grate,
}
use Kind::*;

fn fits(theme: Theme) -> &'static [Kind] {
    match theme {
        Theme::Threshold => &[Collapse, Banner, Stalactites, Brazier],
        Theme::Return => &[Crack, Leak, Niches],
        Theme::Seal => &[Niches, Crack, Leak],
        Theme::Memory => &[Leak, Niches, Stalactites],
        Theme::Lair => &[Collapse, Stalactites, Crack, Brazier],
        Theme::Guardpost => &[Banner, Grate, Collapse, Brazier],
        Theme::Custodian => &[Lava, Collapse],
        Theme::Heart => &[Lava, Stalactites],
        Theme::Crypt => &[Niches, Collapse, Leak, Crack],
        Theme::Mine => &[Collapse, Leak, Stalactites, Grate],
        Theme::Fungal => &[Leak, Stalactites, Collapse],
        Theme::Forge => &[Collapse, Lava, Grate, Brazier],
        Theme::Library => &[Leak, Niches, Banner, Collapse],
        Theme::Overgrowth => &[Leak, Crack, Collapse],
    }
}

const DARK: C3 = [0.02, 0.018, 0.018];
const WATER: C3 = [0.05, 0.07, 0.08];
const SHEEN: C3 = [0.08, 0.13, 0.15];

// ---------------------------------------------------------------- models

/// Fallen masonry heaped under a dark hole in the vault.
pub fn collapse_pile(
    m: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    ceiling: f32,
    st: C3,
    rng: &mut ArtRng,
) {
    for _ in 0..14 {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.unit64().sqrt() as f32 * 0.85;
        let h = (1.0 - d / radius) * radius * 0.5;
        let size = v(
            rng.range(0.16, 0.36),
            rng.range(0.1, 0.2),
            rng.range(0.14, 0.3),
        );
        let yaw = rng.range(0.0, TAU);
        let x = norm(v(yaw.cos(), rng.range(-0.3, 0.3), yaw.sin()));
        let z = norm(x.cross(VY));
        let y = norm(z.cross(x));
        let lift = h * rng.range(0.4, 1.0) + size.y;
        let shade = rng.range(0.65, 1.05);
        chamfer_box(
            m,
            center + v(a.cos() * d, lift, a.sin() * d),
            x,
            y,
            z,
            size,
            0.03,
            tint(st, shade),
        );
    }
    for _ in 0..20 {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.range(0.4, 1.25);
        let s = rng.range(0.05, 0.14);
        let shade = rng.range(0.55, 0.95);
        rock(
            m,
            center + v(a.cos() * d, 0.0, a.sin() * d),
            v(s * 1.3, s, s),
            tint(st, shade),
            a,
        );
    }
    blob(
        m,
        center,
        radius * 1.45,
        tint(mix_color(st, [0.5, 0.48, 0.46], 0.4), 0.95),
        rng,
        0.005,
        16,
        0.4,
    );
    // The hole it fell from: a ragged dark patch just under the vault.
    let hole = with_y(center, ceiling - 0.03);
    let pts = 12;
    for k in 0..pts {
        let a0 = (k as f32 / pts as f32) * TAU;
        let a1 = ((k + 1) as f32 / pts as f32) * TAU;
        let r0 = radius * 0.7 * rng.range(0.75, 1.1);
        let r1 = radius * 0.7 * rng.range(0.75, 1.1);
        tri(
            m,
            hole,
            hole + v(a0.cos() * r0, 0.0, a0.sin() * r0),
            hole + v(a1.cos() * r1, 0.0, a1.sin() * r1),
            Y(-1.0),
            DARK,
        );
    }
    for _ in 0..6 {
        let a = rng.range(0.0, TAU);
        let p = hole + v(a.cos() * radius * 0.7, -0.02, a.sin() * radius * 0.7);
        let r = rng.range(0.06, 0.12);
        let len = rng.range(0.15, 0.35);
        crystal(m, p, r, -len, tint(st, 0.7), 5);
    }
}

/// A crack splitting the floor, broken slabs sunk along it; lava below.
pub fn floor_rift(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    start: Vec3,
    dir: Vec3,
    length: f32,
    lava: bool,
    st: C3,
    rng: &mut ArtRng,
) {
    let mut p = start + Y(0.008);
    let mut d = norm(hz(dir));
    let steps = ((length / 0.3).round() as usize).max(6);
    for k in 0..steps {
        d = norm(d + v(-d.z, 0.0, d.x) * rng.range(-0.4, 0.4));
        let q = p + d * (length / steps as f32);
        let w = 0.07 * ((k as f32 / steps as f32) * PI).sin() + 0.015;
        let n = v(-d.z, 0.0, d.x);
        quad(
            m,
            [p - n * w, q - n * (w * 0.8), q + n * (w * 0.8), p + n * w],
            VY,
            DARK,
        );
        if lava {
            let g = 0.36 + rng.range(0.0, 0.15);
            let dy = Y(0.002);
            quad(
                glow,
                [
                    p - n * (w * 0.45) - dy,
                    q - n * (w * 0.35) - dy,
                    q + n * (w * 0.35) - dy,
                    p + n * (w * 0.45) - dy,
                ],
                VY,
                [1.0, g, 0.08],
            );
        }
        for sgn in [-1.0f32, 1.0] {
            if !rng.chance(0.45) {
                continue;
            }
            let c = p + n * (sgn * (w + rng.range(0.1, 0.22)));
            let tilt = norm(VY + n * (sgn * rng.range(0.15, 0.35)));
            let u = d * rng.range(0.12, 0.22);
            let s = norm(tilt.cross(u)) * rng.range(0.1, 0.18);
            let shade = rng.range(0.55, 0.8);
            quad(
                m,
                [c - u - s, c + u - s, c + u + s, c - u + s],
                tilt,
                tint(st, shade),
            );
        }
        if k > 1 && rng.chance(0.3) {
            let side = if rng.chance(0.5) { 1.0 } else { -1.0 };
            let len = rng.range(0.4, 0.9);
            crack(
                m,
                q - Y(0.008),
                v(-d.z, 0.0, d.x) * side + d * 0.4,
                len,
                rng,
                [0.03, 0.025, 0.025],
                0.007,
            );
        }
        p = q;
    }
    let along = norm(hz(dir));
    for _ in 0..10 {
        let t = rng.range(0.0, length);
        let x = rng.range(-0.4, 0.4);
        let z = rng.range(-0.4, 0.4);
        let q = start + along * t + v(x, 0.0, z);
        let shade = rng.range(0.55, 0.9);
        rock(
            m,
            q,
            v(0.06, 0.04, 0.05),
            tint(st, shade),
            rng.range(0.0, TAU),
        );
    }
}

/// A leak: a wet stain on the vault with dripstone teats, and the puddle it
/// feeds (with the water shader, its drops fall and ring the water).
pub fn leak(m: &mut Mesh3D, center: Vec3, ceiling: f32, rng: &mut ArtRng, wet: &mut Vec<Pond>) {
    let top = with_y(center, ceiling - 0.025);
    for k in 0..10 {
        let a0 = (k as f32 / 10.0) * TAU;
        let a1 = ((k + 1) as f32 / 10.0) * TAU;
        tri(
            m,
            top,
            top + v(a0.cos() * 0.6, 0.0, a0.sin() * 0.6),
            top + v(a1.cos() * 0.55, 0.0, a1.sin() * 0.55),
            Y(-1.0),
            [0.06, 0.07, 0.07],
        );
    }
    for _ in 0..7 {
        let x = rng.range(-0.4, 0.4);
        let z = rng.range(-0.4, 0.4);
        let p = top + v(x, -0.02, z);
        let length = rng.range(0.08, 0.25);
        let r = rng.range(0.02, 0.05);
        crystal(m, p, r, -length, [0.3, 0.33, 0.34], 5);
        // A bead of water swelling at the tip.
        lathe(
            m,
            p - Y(length + 0.018),
            VY,
            &[(0.0, 0.0), (0.008, 0.009), (0.018, 0.004)],
            5,
            [0.46, 0.55, 0.6],
            true,
        );
    }
    let r = rng.range(0.5, 0.75);
    // The drip that keeps falling, from the heart of the stain.
    let outline = shoreline(center, r, rng, 16, 0.3);
    pond(
        wet,
        &outline,
        center.y + 0.012,
        WATER,
        reflection_of(Some(SHEEN)),
        0.35,
        1.22,
        Some(ceiling - 0.06),
    );
}

/// Three arched niches in a wall slot holding candles, skulls and urns.
pub fn wall_niches(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    st: C3,
    rng: &mut ArtRng,
) {
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    let w = (width / 3.4).min(0.9);
    let hole = [0.04, 0.035, 0.035];
    for k in -1..=1 {
        let x = k as f32 * w * 1.2;
        let y0 = 0.95;
        let h = 0.62;
        let hw = w * 0.42;
        quad(
            m,
            [
                l(x - hw, y0, 0.03),
                l(x + hw, y0, 0.03),
                l(x + hw, y0 + h, 0.03),
                l(x - hw, y0 + h, 0.03),
            ],
            inward,
            hole,
        );
        for s in 0..8 {
            let a0 = PI * s as f32 / 8.0;
            let a1 = PI * (s + 1) as f32 / 8.0;
            tri(
                m,
                l(x, y0 + h, 0.03),
                l(x - a0.cos() * hw, y0 + h + a0.sin() * hw, 0.03),
                l(x - a1.cos() * hw, y0 + h + a1.sin() * hw, 0.03),
                inward,
                hole,
            );
        }
        for s in 0..=8 {
            let a = PI * s as f32 / 8.0;
            let shade = rng.range(0.95, 1.2);
            chamfer_box(
                m,
                l(
                    x - a.cos() * (hw + 0.06),
                    y0 + h + a.sin() * (hw + 0.06),
                    0.07,
                ),
                tangent,
                VY,
                inward,
                v(0.05, 0.05, 0.05),
                0.01,
                tint(st, shade),
            );
        }
        for sgn in [-1.0f32, 1.0] {
            chamfer_box(
                m,
                l(x + sgn * (hw + 0.06), y0 + h * 0.5, 0.07),
                tangent,
                VY,
                inward,
                v(0.05, h * 0.5, 0.05),
                0.01,
                tint(st, 1.05),
            );
        }
        chamfer_box(
            m,
            l(x, y0 - 0.04, 0.12),
            tangent,
            VY,
            inward,
            v(hw + 0.12, 0.04, 0.13),
            0.01,
            tint(st, 1.12),
        );
        match rng.index(3) {
            0 => skull(
                m,
                l(x, y0 + 0.08, 0.14),
                0.85,
                yaw_of(inward),
                0.0,
                [0.8, 0.74, 0.62],
            ),
            1 => {
                let shade = rng.range(0.8, 1.1);
                lathe(
                    m,
                    l(x, y0, 0.14),
                    VY,
                    &[
                        (0.0, 0.05),
                        (0.04, 0.08),
                        (0.16, 0.09),
                        (0.24, 0.05),
                        (0.28, 0.06),
                    ],
                    9,
                    tint([0.42, 0.26, 0.16], shade),
                    true,
                );
            }
            _ => {
                let h = rng.range(0.1, 0.22);
                wax_candle(m, glow, l(x, y0, 0.14), 0.035, h, rng, WAX, true);
            }
        }
        if rng.chance(0.6) {
            let h = rng.range(0.05, 0.12);
            wax_candle(m, glow, l(x + hw * 0.6, y0, 0.12), 0.022, h, rng, WAX, true);
        }
    }
}

/// A torn banner hanging from an iron rod.
pub fn torn_banner(
    m: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    top: f32,
    cloth: C3,
    rng: &mut ArtRng,
) {
    let hw = 0.42;
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    bar(
        m,
        l(-hw - 0.08, top, 0.1),
        l(hw + 0.08, top, 0.1),
        0.012,
        0.012,
        VY,
        [0.14, 0.12, 0.11],
    );
    let lengths: Vec<f32> = (0..9).map(|_| rng.range(1.0, 1.75)).collect();
    grid_surface(
        m,
        8,
        8,
        &|i, j| {
            let x = -hw + (j as f32 / 8.0) * hw * 2.0;
            let t = i as f32 / 8.0;
            let len = lengths[j];
            let sway = (j as f32 * 1.3 + t * 2.0).sin() * 0.03 * t;
            l(x, top - 0.02 - t * len, 0.1 + sway + t * 0.03)
        },
        &|i, j| {
            tint(
                cloth,
                (0.8 + 0.2 * (j as f32 * 1.9).sin()) * (1.0 - i as f32 * 0.03),
            )
        },
        false,
        Some(&|_, _, _| inward),
        Some(&|_, _| tint(cloth, 0.5)),
    );
    let trim = [0.78, 0.56, 0.2];
    quad(
        m,
        [
            l(-hw, top - 0.06, 0.103),
            l(hw, top - 0.06, 0.103),
            l(hw, top - 0.1, 0.103),
            l(-hw, top - 0.1, 0.103),
        ],
        inward,
        trim,
    );
    for k in 0..6 {
        let a = (k as f32 / 6.0) * TAU;
        let c = l(0.0, top - 0.55, 0.108);
        quad(
            m,
            [
                c,
                c + tangent * (a.cos() * 0.14) + Y(a.sin() * 0.14),
                c + tangent * ((a + 0.5).cos() * 0.08) + Y((a + 0.5).sin() * 0.08),
                c,
            ],
            inward,
            trim,
        );
    }
}

/// An old iron brazier, cold ash and a last few embers.
pub fn dead_brazier(m: &mut Mesh3D, glow: &mut Mesh3D, b: Vec3, rng: &mut ArtRng) {
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            b + d * 0.3,
            b + d * 0.15 + Y(0.7),
            0.025,
            0.025,
            VY,
            [0.14, 0.12, 0.11],
        );
    }
    lathe(
        m,
        b + Y(0.66),
        VY,
        &[(0.0, 0.1), (0.07, 0.27), (0.18, 0.35), (0.2, 0.34)],
        12,
        [0.16, 0.13, 0.12],
        true,
    );
    disc(m, b + Y(0.84), VY, 0.32, 12, [0.3, 0.28, 0.27]);
    for _ in 0..4 {
        let x = rng.range(-0.18, 0.18);
        let z = rng.range(-0.18, 0.18);
        rock(
            glow,
            b + v(x, 0.84, z),
            v(0.04, 0.02, 0.035),
            [0.7, 0.2, 0.05],
            rng.range(0.0, TAU),
        );
    }
    blob(m, b, 0.55, [0.2, 0.19, 0.19], rng, 0.006, 12, 0.5);
}

/// A cluster of stalactites dripping onto a mound.
pub fn stalactites(
    m: &mut Mesh3D,
    center: Vec3,
    ceiling: f32,
    st: C3,
    rng: &mut ArtRng,
    wet: &mut Vec<Pond>,
) {
    for _ in 0..9 {
        let a = rng.range(0.0, TAU);
        let d = rng.range(0.0, 0.9);
        let p = with_y(center + v(a.cos() * d, 0.0, a.sin() * d), ceiling + 0.05);
        let len = rng.range(0.4, 1.4) * (1.0 - d * 0.4);
        let r = rng.range(0.08, 0.2);
        let shade = rng.range(0.6, 0.85);
        crystal(m, p, r, -len, tint(st, shade), 6);
    }
    for _ in 0..5 {
        let a = rng.range(0.0, TAU);
        let d = rng.range(0.0, 0.6);
        let r = rng.range(0.06, 0.14);
        let h = rng.range(0.1, 0.35);
        let shade = rng.range(0.6, 0.85);
        crystal(
            m,
            center + v(a.cos() * d, 0.0, a.sin() * d),
            r,
            h,
            tint(st, shade),
            6,
        );
    }
    puddle(wet, center, 0.45, rng, WATER, Some(SHEEN), 0.3);
}

/// An iron drain grate over darkness, rust bleeding round it.
pub fn drain_grate(m: &mut Mesh3D, center: Vec3, yaw: f32, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let c = center + Y(0.012);
    quad(
        m,
        [
            c + s * -0.42 + f * -0.42,
            c + s * 0.42 + f * -0.42,
            c + s * 0.42 + f * 0.42,
            c + s * -0.42 + f * 0.42,
        ],
        VY,
        [0.015, 0.014, 0.014],
    );
    for k in -3..=3 {
        let x = s * (k as f32 * 0.12);
        bar(
            m,
            c + x + f * -0.44 + Y(0.01),
            c + x + f * 0.44 + Y(0.01),
            0.02,
            0.012,
            VY,
            [0.2, 0.14, 0.1],
        );
    }
    for sgn in [-1.0f32, 1.0] {
        bar(
            m,
            c + f * (sgn * 0.44) + s * -0.46 + Y(0.012),
            c + f * (sgn * 0.44) + s * 0.46 + Y(0.012),
            0.03,
            0.016,
            VY,
            [0.18, 0.12, 0.09],
        );
    }
    blob(m, center, 0.8, [0.3, 0.17, 0.1], rng, 0.004, 12, 0.5);
}

// ---------------------------------------------------------------- planner

/// Add one or two incidents fitting `theme` to the stage's plan. `condition`
/// is the room finish's ageing (0 flooded, 1 scorched, 2 ancient, ...).
pub fn plan(s: &mut Stage, theme: Theme, mut rng: ArtRng, condition: Option<usize>) {
    let options = fits(theme);
    let shuffled = rng.shuffled(options);
    let count = 1 + usize::from(rng.chance(0.55));
    let mut picks: Vec<Kind> = shuffled.into_iter().take(count).collect();
    // The room's condition (drawn with its finish) adds what it implies.
    if condition == Some(0) && !picks.contains(&Leak) {
        picks.push(Leak);
    }
    if condition == Some(2) && !picks.contains(&Collapse) && options.contains(&Collapse) {
        picks.push(Collapse);
    }
    let ceiling = s.c.y + s.height;
    let accent = [
        [0.46, 0.08, 0.07],
        [0.12, 0.22, 0.36],
        [0.18, 0.3, 0.14],
        [0.36, 0.26, 0.1],
    ][rng.index(4)];
    for kind in picks {
        match kind {
            Collapse | Brazier => {
                let (dists, r, focus_gap, gap) = if kind == Collapse {
                    ([1.4, 1.8], 0.95, 1.4, 0.8)
                } else {
                    ([1.2, 1.5], 0.35, 1.2, 0.6)
                };
                let corners = rng.shuffled(&s.corners);
                for corner in corners {
                    let cands: Vec<Vec3> = dists
                        .iter()
                        .map(|&d| corner.mid + corner.inward * d)
                        .collect();
                    let Some(p) = s.place(&cands, r, focus_gap, gap) else {
                        continue;
                    };
                    s.plan.solids.push((p, r));
                    if kind == Collapse {
                        s.piece(move |m, _g, ctx| {
                            collapse_pile(m, p, 1.1, ceiling, ctx.stone, &mut ctx.rng)
                        });
                    } else {
                        s.piece(move |m, g, ctx| dead_brazier(m, g, p, &mut ctx.rng));
                    }
                    break;
                }
            }
            Crack | Lava => {
                let wall = s.walls[rng.index(s.walls.len())].clone();
                let start =
                    s.floor(wall.mid + wall.inward * 0.4 + wall.tangent * rng.range(-1.5, 1.5));
                if s.doorway(start) < 1.0 {
                    continue;
                }
                let dir = norm(wall.inward + wall.tangent * rng.range(-0.5, 0.5));
                let length = rng.range(2.5, s.depth.min(5.0));
                let lava = kind == Lava;
                s.piece(move |m, g, ctx| {
                    floor_rift(m, g, start, dir, length, lava, ctx.stone, &mut ctx.rng)
                });
                if lava {
                    s.light(
                        start + wall.inward * 1.5 + Y(0.4),
                        [1.0, 0.35, 0.08],
                        1.8,
                        4.0,
                    );
                }
            }
            Leak => {
                s.scatter(
                    1,
                    0.8,
                    Scatter {
                        walkway: 0.0,
                        ..Default::default()
                    },
                    &mut |s, p, _| {
                        s.piece(move |m, _g, ctx| leak(m, p, ceiling, &mut ctx.rng, &mut ctx.wet));
                    },
                );
            }
            Stalactites => {
                s.scatter(
                    1,
                    1.0,
                    Scatter {
                        walkway: 1.2,
                        ring: [0.35, 0.9],
                        ..Default::default()
                    },
                    &mut |s, p, _| {
                        s.piece(move |m, _g, ctx| {
                            stalactites(m, p, ceiling, ctx.stone, &mut ctx.rng, &mut ctx.wet)
                        });
                        s.solid(p, 0.45, false);
                    },
                );
            }
            Niches | Banner => {
                // A wall slot nothing else hangs on or stands before.
                let bays = s.free_bays(None, 1.5);
                let slot = bays.into_iter().find(|bay| {
                    !s.crowded(bay.center + bay.inward * 0.35, 0.45, 0.0)
                        && !s
                            .plan
                            .mounts
                            .iter()
                            .any(|mount| hdist(mount.anchor, bay.center) < 1.3)
                        && !s
                            .plan
                            .wall_pieces
                            .iter()
                            .any(|&q| hdist(q, bay.center) < 1.4)
                });
                let Some(slot) = slot else {
                    continue;
                };
                s.take_bay(&slot);
                let c = s.floor(slot.center);
                let (inw, tan, width) = (slot.inward, slot.tangent, slot.width);
                s.plan.wall_pieces.push(c);
                if kind == Niches {
                    s.piece(move |m, g, ctx| {
                        wall_niches(m, g, c, inw, tan, width, ctx.stone, &mut ctx.rng)
                    });
                } else {
                    let top = (s.spring - 0.3).min(3.4);
                    s.piece(move |m, _g, ctx| {
                        torn_banner(m, c, inw, tan, top, accent, &mut ctx.rng)
                    });
                }
            }
            Grate => {
                s.scatter(
                    1,
                    0.5,
                    Scatter {
                        walkway: 0.0,
                        ..Default::default()
                    },
                    &mut |s, p, _| {
                        let yaw = rng.range(0.0, TAU);
                        s.piece(move |m, _g, ctx| drain_grate(m, p, yaw, &mut ctx.rng));
                    },
                );
            }
        }
    }
}
