//! Las Raíces anegadas (web prototype `galleries/overgrowth.js`): a gallery
//! drowned by a tree far above — a colossus of roots pouring from the vault
//! into a pool of black water with lily pads, roots crawling from the walls,
//! vines and seed pods, a fallen column, ferns, flowers and fireflies.
use super::fungal::moss_carpet;
use super::kit::*;
use super::plan::Pond;
use super::stage::*;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const BARK: C3 = [0.3, 0.21, 0.14];
const BARK_DARK: C3 = [0.18, 0.12, 0.08];
const MOSS: C3 = [0.2, 0.36, 0.16];
const SAP: C3 = [0.62, 1.0, 0.42];
const WATER: C3 = [0.04, 0.09, 0.08];
const LEAF: C3 = [0.2, 0.42, 0.18];
const BLOOM: C3 = [0.45, 0.8, 1.0];

fn bark_color(t: f32, around: f32) -> C3 {
    mix_color(
        BARK_DARK,
        BARK,
        0.45 + 0.4 * (around * 19.0 + t * 5.0).sin().abs(),
    )
}

/// The root colossus: roots pour from the vault, braid into a lobed trunk,
/// then splay into buttress roots; sap glows in veins, moss clings on top.
pub fn root_colossus(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    ceiling: f32,
    reach_r: f32,
    rng: &mut ArtRng,
) {
    let knot = base + Y(2.5);
    for k in 0..9 {
        let a = (k as f32 / 9.0) * TAU + rng.range(-0.2, 0.2);
        let d = v(a.cos(), 0.0, a.sin());
        let top = with_y(base + d * (reach_r * rng.range(0.55, 0.9)), ceiling + 0.3);
        let pts = bezier(
            top,
            top.lerp(knot, 0.5) + d * 0.5 - Y(0.2),
            knot + d * 0.35,
            12,
        );
        tube(m, &pts, &|t| 0.24 - t * 0.06, 8, &bark_color);
        if k % 2 == 0 {
            let vein: Vec<Vec3> = pts
                .iter()
                .enumerate()
                .map(|(i, &p)| p + d * (-0.2 + 0.02 * (i as f32).sin()) + Y(0.18))
                .collect();
            tube(glow, &vein, &|_| 0.01, 4, &|_, _| tint(SAP, 0.22));
        }
        if k % 3 == 0 {
            let pod = pts[(pts.len() as f32 * 0.4).floor() as usize] - Y(0.2);
            bar(m, pod + Y(0.2), pod + Y(0.02), 0.008, 0.008, VX, BARK);
            lathe(
                glow,
                pod - Y(0.12),
                VY,
                &[
                    (0.0, 0.0),
                    (0.04, 0.06),
                    (0.1, 0.07),
                    (0.14, 0.03),
                    (0.16, 0.0),
                ],
                8,
                tint(SAP, 0.55),
                true,
            );
        }
    }
    const TR: usize = 16;
    const TC: usize = 28;
    let trunk_at = |i: usize, j: usize| {
        let y = 0.35 + (i as f32 / TR as f32) * 2.35;
        let a = (j as f32 / TC as f32) * TAU + y * 0.45;
        let r = 0.52 + 0.16 * (3.0 * a).sin().abs() + 0.12 * (1.0 - y / 0.9).max(0.0);
        base + v(a.cos() * r, y, a.sin() * r)
    };
    grid_surface(
        m,
        TR,
        TC,
        &trunk_at,
        &|i, j| {
            let a = (j as f32 / TC as f32) * TAU + (0.35 + (i as f32 / TR as f32) * 2.35) * 0.45;
            mix_color(BARK_DARK, BARK, 0.35 + 0.6 * (3.0 * a).sin().abs())
        },
        true,
        Some(&|p, i, _| p - (base + Y(0.35 + (i as f32 / TR as f32) * 2.35))),
        None,
    );
    for k in 0..3 {
        let a0 = (k as f32 / 3.0) * TAU;
        let pts: Vec<Vec3> = (0..=14)
            .map(|i| {
                let y = 0.4 + (i as f32 / 14.0) * 2.2;
                let a = a0 + y * 0.45 + std::f32::consts::PI / 6.0;
                let r =
                    0.52 + 0.16 * (3.0 * a).sin().abs() + 0.12 * (1.0 - y / 0.9).max(0.0) + 0.02;
                base + v(a.cos() * r, y, a.sin() * r)
            })
            .collect();
        tube_c(glow, &pts, 0.011, 4, tint(SAP, 0.22));
    }
    for k in 0..9 {
        let a = (k as f32 / 9.0) * TAU + rng.range(-0.25, 0.25);
        let d = v(a.cos(), 0.0, a.sin());
        let reach = reach_r * rng.range(0.75, 1.1);
        let start = base + d * 0.45 + Y(0.7);
        let raw = wander(start, base + d * reach + Y(-0.05), 14, 0.25, rng, 0.0);
        let n = raw.len();
        let pts: Vec<Vec3> = raw
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                let t = i as f32 / (n - 1) as f32;
                with_y(
                    p,
                    base.y + lerp(0.7, 0.08, (t * 2.2).min(1.0))
                        - if t > 0.9 { (t - 0.9) * 1.2 } else { 0.0 },
                )
            })
            .collect();
        tube(
            m,
            &pts,
            &|t| 0.3 * (1.0 - t).powf(0.8) + 0.035,
            8,
            &bark_color,
        );
        let mut i = 2;
        while i < pts.len() - 2 {
            let shade = rng.range(0.8, 1.2);
            blob(
                m,
                pts[i] + Y(0.2 * (1.0 - i as f32 / pts.len() as f32).powf(0.8) + 0.02),
                0.12,
                tint(MOSS, shade),
                rng,
                0.0,
                7,
                0.4,
            );
            i += 3;
        }
        if k % 3 == 1 {
            let len = pts.len() as f32;
            let vein: Vec<Vec3> = pts
                .iter()
                .enumerate()
                .map(|(i, &p)| p + Y(0.3 * (1.0 - i as f32 / len).powf(0.8) + 0.03))
                .collect();
            tube(glow, &vein, &|t| 0.014 * (1.0 - t) + 0.004, 4, &|_, _| {
                tint(SAP, 0.32)
            });
        }
    }
    for _ in 0..6 {
        let a = rng.range(0.0, TAU);
        let top = knot + v(a.cos() * 0.6, 0.1, a.sin() * 0.6);
        let length = rng.range(0.8, 1.6);
        hanging_vine(m, glow, top, length, rng);
    }
}

/// A vine hanging from `top`: a thin stem with leaves and the odd bloom.
pub fn hanging_vine(m: &mut Mesh3D, glow: &mut Mesh3D, top: Vec3, length: f32, rng: &mut ArtRng) {
    let sway = v(rng.range(-0.15, 0.15), 0.0, rng.range(-0.15, 0.15));
    let pts = bezier(
        top,
        top - Y(length * 0.5) + sway,
        top - Y(length) + sway * 0.4,
        10,
    );
    tube_c(m, &pts, 0.012, 4, tint(LEAF, 0.6));
    for &p in pts.iter().skip(1) {
        for _sgn in [-1.0, 1.0] {
            if !rng.chance(0.7) {
                continue;
            }
            let a = rng.range(0.0, TAU);
            let d = norm(v(a.cos(), -0.4, a.sin())) * rng.range(0.05, 0.09);
            let side = norm(v(-d.z, 0.0, d.x)) * 0.03;
            let leaf = [p, p + d * 0.5 + side, p + d, p + d * 0.5 - side];
            let s1 = rng.range(0.8, 1.3);
            quad(m, leaf, VY, tint(LEAF, s1));
            let s2 = rng.range(0.5, 0.8);
            quad(m, leaf, Y(-1.0), tint(LEAF, s2));
        }
    }
    if rng.chance(0.4) {
        lathe(
            glow,
            pts[pts.len() - 1] - Y(0.05),
            VY,
            &[(0.0, 0.0), (0.02, 0.03), (0.05, 0.02), (0.06, 0.0)],
            6,
            tint(BLOOM, 0.7),
            true,
        );
    }
}

/// Black water pooled about a point: an irregular shore, lily pads with
/// glowing blooms, reeds and stepping stones.
pub fn root_pool(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    stone: C3,
    rng: &mut ArtRng,
    wet: &mut Vec<Pond>,
) {
    blob(
        m,
        center,
        radius * 1.1,
        tint([0.16, 0.2, 0.14], 0.9),
        rng,
        0.008,
        20,
        0.35,
    );
    let outline = shoreline(center, radius, rng, 20, 0.28);
    pond(
        wet,
        &outline,
        center.y + 0.013,
        WATER,
        reflection_of(Some(tint(SAP, 0.14))),
        1.0,
        1.08,
        None,
    );
    for _ in 0..(radius * 7.0).round() as usize {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.range(0.3, 0.85);
        let c = center + v(a.cos() * d, 0.016, a.sin() * d);
        let r = rng.range(0.1, 0.2);
        let notch = rng.range(0.0, TAU);
        for s in 0..10 {
            let a0 = notch + 0.35 + (s as f32 / 10.0) * (TAU - 0.7);
            let a1 = notch + 0.35 + ((s + 1) as f32 / 10.0) * (TAU - 0.7);
            tri(
                m,
                c,
                c + v(a0.cos() * r, 0.0, a0.sin() * r),
                c + v(a1.cos() * r, 0.0, a1.sin() * r),
                VY,
                tint(LEAF, 0.9 + (s % 2) as f32 * 0.12),
            );
        }
        if rng.chance(0.35) {
            for p in 0..6 {
                let pa = (p as f32 / 6.0) * TAU;
                let tip = c + v(pa.cos() * 0.07, 0.05, pa.sin() * 0.07);
                let across = v(-pa.sin(), 0.0, pa.cos()) * 0.025;
                tri(
                    glow,
                    c + Y(0.01),
                    tip + across,
                    tip - across,
                    VY,
                    tint(BLOOM, 0.36),
                );
            }
            disc(glow, c + Y(0.03), VY, 0.02, 6, [1.0, 0.9, 0.6]);
        }
    }
    for _ in 0..5 {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.range(0.4, 0.8);
        let shade = rng.range(0.6, 0.8);
        rock(
            m,
            center + v(a.cos() * d, -0.02, a.sin() * d),
            v(0.22, 0.09, 0.2),
            tint(stone, shade),
            a,
        );
    }
    for _ in 0..(radius * 5.0).round() as usize {
        let a = rng.range(0.0, TAU);
        let (k1, k2) = (rng.range(0.92, 1.08), rng.range(0.92, 1.08));
        let c = center + v(a.cos() * radius * k1, 0.0, a.sin() * radius * k2);
        let count = 3 + rng.index(4);
        reeds(m, c, count, rng);
    }
}

/// Reeds and cattails.
pub fn reeds(m: &mut Mesh3D, base: Vec3, count: usize, rng: &mut ArtRng) {
    for _ in 0..count {
        let b = base + v(rng.range(-0.12, 0.12), 0.0, rng.range(-0.12, 0.12));
        let h = rng.range(0.5, 1.0);
        let lean = norm(v(rng.range(-0.15, 0.15), 1.0, rng.range(-0.15, 0.15)));
        let top = b + lean * h;
        let shade = rng.range(0.8, 1.2);
        bar(m, b, top, 0.008, 0.008, VX, tint(LEAF, shade));
        if rng.chance(0.5) {
            lathe(
                m,
                top - lean * 0.14,
                lean,
                &[(0.0, 0.02), (0.02, 0.028), (0.12, 0.028), (0.14, 0.01)],
                6,
                [0.32, 0.2, 0.1],
                true,
            );
        } else {
            let d = norm(v(rng.range(-1.0, 1.0), 0.0, rng.range(-1.0, 1.0)));
            quad(
                m,
                [b, b + d * 0.02, top + d * 0.12, top + d * 0.1],
                norm(d.cross(VY)),
                tint(LEAF, 1.1),
            );
        }
    }
}

/// A root crawling along `points`, thick at its start, mossed on top.
pub fn crawl_root(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    pts: &[Vec3],
    radius: f32,
    sap: bool,
    rng: &mut ArtRng,
) {
    tube(
        m,
        pts,
        &|t| radius * (1.0 - t * 0.75) + 0.02,
        7,
        &bark_color,
    );
    let mut i = 1;
    while i + 1 < pts.len() {
        if rng.chance(0.6) {
            let shade = rng.range(0.8, 1.2);
            blob(
                m,
                pts[i] + Y(radius * (1.0 - (i as f32 / pts.len() as f32) * 0.75) + 0.015),
                radius * 0.7,
                tint(MOSS, shade),
                rng,
                0.0,
                7,
                0.4,
            );
        }
        i += 2;
    }
    if sap {
        let len = pts.len() as f32;
        let vein: Vec<Vec3> = pts
            .iter()
            .enumerate()
            .map(|(i, &p)| p + Y(radius * (1.0 - (i as f32 / len) * 0.75) + 0.02))
            .collect();
        tube_c(glow, &vein, 0.01, 4, tint(SAP, 0.3));
    }
}

/// Blue flowers glowing on slender stems.
pub fn glow_flowers(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    count: usize,
    color: C3,
    rng: &mut ArtRng,
) {
    for _ in 0..count {
        let b = base + v(rng.range(-0.25, 0.25), 0.0, rng.range(-0.25, 0.25));
        let h = rng.range(0.15, 0.4);
        let top = b + v(rng.range(-0.05, 0.05), h, rng.range(-0.05, 0.05));
        bar(m, b, top, 0.006, 0.006, VX, tint(LEAF, 1.0));
        for p in 0..5 {
            let a = (p as f32 / 5.0) * TAU;
            let d = norm(v(a.cos(), 0.4, a.sin()));
            let s = v(-a.sin(), 0.0, a.cos()) * 0.02;
            tri(
                glow,
                top,
                top + d * 0.06 + s,
                top + d * 0.06 - s,
                norm(d + Y(1.0)),
                tint(color, 0.34),
            );
        }
        disc(glow, top + Y(0.012), VY, 0.012, 5, [1.0, 0.95, 0.7]);
    }
}

/// A toppled column: fluted drums lying apart, a capital, moss and roots.
pub fn fallen_column(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    let mut z = -1.4;
    for _ in 0..3 {
        let len = rng.range(0.7, 0.95);
        let c = l(rng.range(-0.05, 0.05), 0.3, z + len / 2.0);
        let axis = norm(f + s * rng.range(-0.08, 0.08));
        grid_surface(
            m,
            2,
            24,
            &|i, j| {
                let a = (j as f32 / 24.0) * TAU;
                let r = 0.3 * (1.0 - 0.05 * (0.5 + 0.5 * (12.0 * a).cos()).powi(4));
                c + axis * ((i as f32 / 2.0 - 0.5) * len) + s * (a.cos() * r) + VY * (a.sin() * r)
            },
            &|_, j| tint(stone, 0.85 + 0.1 * (12.0 * (j as f32 / 24.0) * TAU).cos()),
            true,
            Some(&|p, _, _| (p - c) - axis * (p - c).dot(axis)),
            None,
        );
        for e in [-1.0, 1.0] {
            disc(
                m,
                c + axis * (e * len / 2.0),
                axis * e,
                0.3,
                24,
                tint(stone, 0.95),
            );
        }
        blob(m, c + Y(0.3), 0.22, tint(MOSS, 1.0), rng, 0.005, 8, 0.5);
        z += len + rng.range(0.05, 0.25);
    }
    chamfer_box(
        m,
        l(0.1, 0.22, z + 0.35),
        s,
        VY,
        f,
        v(0.42, 0.22, 0.42),
        0.04,
        tint(stone, 1.05),
    );
    chamfer_box(
        m,
        l(0.1, 0.5, z + 0.35),
        s,
        VY,
        f,
        v(0.34, 0.07, 0.34),
        0.03,
        tint(stone, 1.12),
    );
    for k in 0..3 {
        let from = l(rng.range(-0.3, 0.3), 0.62, -1.3 + k as f32 * 0.9);
        let pts = bezier(
            from,
            from + s * 0.5 + Y(0.1),
            with_y(from + s * 1.1, base.y + 0.04),
            8,
        );
        crawl_root(m, glow, &pts, 0.07, false, rng);
    }
}

// ---------------------------------------------------------------- planner

/// Plan the flooded roots into the plan held by the stage.
pub fn plan(s: &mut Stage) {
    let green = [0.5, 1.0, 0.5];
    let heart = s.floor(s.focus);
    let r = 3.2f32.min(s.room.half[0].min(s.room.half[1]) * 0.55);
    // 1. The root colossus pours from the vault into a pool.
    s.solid(heart, 0.95, false);
    let pool_r = 2.6f32.min(r * 0.85);
    s.piece(move |m, g, ctx| {
        let stone = ctx.stone;
        let super::plan::Ctx { rng, wet, .. } = ctx;
        root_pool(m, g, heart, pool_r, stone, rng, wet);
    });
    let ceiling = s.c.y + s.height;
    s.piece(move |m, g, ctx| root_colossus(m, g, heart, ceiling, r, &mut ctx.rng));
    s.light(heart + Y(2.2), SAP, 3.2, 8.0);
    let side = heart + Y(0.5) + s.right * 1.2;
    s.light(side, [0.4, 0.85, 0.75], 1.6, 5.0);
    // 2. Roots crawl from the walls and across the floor.
    for bay in s.free_bays(None, 1.2).into_iter().take(5) {
        s.take_bay(&bay);
        let up = s.rng.range(2.2, 3.4f32.min(s.spring));
        let top = bay.center + bay.inward * 0.05 + Y(up);
        let mut k = 0;
        loop {
            // The prototype re-rolls this bound every pass of its loop.
            let bound = 2 + s.rng.index(2);
            if k >= bound {
                break;
            }
            let from = top + bay.tangent * s.rng.range(-0.6, 0.6);
            let (fi, ft) = (s.rng.range(0.4, 1.0), s.rng.range(-0.9, 0.9));
            let foot = s.floor(bay.center + bay.inward * fi + bay.tangent * ft);
            let (oi, ot) = (s.rng.range(0.6, 1.4), s.rng.range(-0.8, 0.8));
            let out = s.floor(foot + bay.inward * oi + bay.tangent * ot);
            let mut pts = bezier(from, from + bay.inward * 0.1 - Y(0.8), foot + Y(0.1), 8);
            pts.push(out + Y(0.04));
            let radius = s.rng.range(0.07, 0.13);
            let sap = s.rng.chance(0.3);
            s.piece(move |m, g, ctx| crawl_root(m, g, &pts, radius, sap, &mut ctx.rng));
            k += 1;
        }
        if let Some(p) = s.spot(bay.center + bay.inward * 0.6, 0.3, 0.3) {
            let yaw = s.rng.range(0.0, TAU);
            let h = s.rng.range(0.55, 0.85);
            s.prop("nat_fern", p, yaw, h, 0.0);
        }
    }
    // 3. A fallen column along a wall.
    let mut walls = s.walls.clone();
    s.rng.shuffle(&mut walls);
    for wall in &walls {
        let yaw = yaw_of(wall.tangent);
        let cands: Vec<Vec3> = [0.0f32, 1.4, -1.4]
            .iter()
            .map(|&k| wall.mid + wall.inward * 1.25 + wall.tangent * k)
            .collect();
        let Some(p) = s.place_row(&cands, yaw, 1.8, 0.45, 1.3, 0.7) else {
            continue;
        };
        s.piece(move |m, g, ctx| fallen_column(m, g, p, yaw, ctx.stone, &mut ctx.rng));
        break;
    }
    // 4. Vines and seed pods from the vault.
    for _ in 0..22 {
        let (a, b) = (s.rng.range(-0.8, 0.8), s.rng.range(-0.8, 0.8));
        let p = s.at(a * s.depth, b * s.width);
        if !s.inside(p, 0.4) {
            continue;
        }
        let top = s.top(p) - Y(0.1);
        let length = s.rng.range(0.8, (s.height - 2.5).max(1.2));
        s.piece(move |m, g, ctx| hanging_vine(m, g, top, length, &mut ctx.rng));
    }
    // 5. Green everything: ferns, grass, flowers, clover, moss, blooms.
    let nature = [
        ("nat_fern", 0.6),
        ("nat_grass", 0.35),
        ("nat_grass_wispy", 0.4),
        ("nat_flowers3", 0.3),
        ("nat_clover1", 0.16),
        ("nat_plant1", 0.35),
        ("nat_flowers4", 0.35),
    ];
    let walk = |w: f32| Scatter {
        walkway: w,
        ..Scatter::default()
    };
    s.scatter(26, 0.25, walk(0.35), &mut |s, p, _| {
        let (asset, h) = nature[s.rng.index(nature.len())];
        let yaw = s.rng.range(0.0, TAU);
        let k = s.rng.range(0.8, 1.3);
        s.prop(asset, p, yaw, h * k, 0.0);
    });
    s.scatter(14, 0.5, walk(0.0), &mut |s, p, _| {
        let radius = s.rng.range(0.4, 0.9);
        s.piece(move |m, g, ctx| moss_carpet(m, g, p, radius, green, &mut ctx.rng));
    });
    s.scatter(8, 0.3, walk(0.4), &mut |s, p, _| {
        let count = 4 + s.rng.index(5);
        let color = if s.rng.chance(0.5) {
            [0.45, 0.8, 1.0]
        } else {
            [0.7, 1.0, 0.5]
        };
        s.piece(move |m, g, ctx| glow_flowers(m, g, p, count, color, &mut ctx.rng));
    });
    s.scatter(
        3,
        0.5,
        Scatter {
            walkway: 1.0,
            ring: [0.6, 0.95],
            ..Scatter::default()
        },
        &mut |s, p, _| {
            let asset = ["nat_rock1", "nat_rock2", "nat_rock3"][s.rng.index(3)];
            let yaw = s.rng.range(0.0, TAU);
            let h = s.rng.range(0.4, 0.65);
            s.prop(asset, p, yaw, h, 0.42);
            s.solid(p, 0.36, false);
        },
    );
    s.scatter(5, 0.4, walk(0.0), &mut |s, p, _| {
        let radius = s.rng.range(0.3, 0.6);
        s.puddle(p, radius, Some(WATER), Some(tint(SAP, 0.12)));
    });
    s.scatter(4, 0.2, walk(0.5), &mut |s, p, _| {
        let count = 4 + s.rng.index(4);
        s.piece(move |m, _g, ctx| reeds(m, p, count, &mut ctx.rng));
    });
    // 6. Fireflies and a green wash.
    let top = s.c + Y(s.height - 1.0);
    s.light(top, green, 1.4, 10.0);
    s.plan.sparkle_tint = Some([1.1, 1.6, 0.5]);
    let h = s.height;
    s.sparkles(44, 0.4, h - 0.8, None, None);
}
