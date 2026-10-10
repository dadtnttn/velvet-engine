//! Sanctuary art kit (web prototype `sanctuary.js`): a stepped dais with the
//! winged Guardian of the Embers before a sun halo, the rest font (octagonal
//! plinth, moat of ember water fed by spouts) on a sun mosaic, ember
//! braziers, votive candles, prayer rugs and cushions, censers, clay pots,
//! petals and sun reliefs.
use super::kit::*;
use super::themekit::candle;
use super::treasury::{DEEP_RED, GOLD_TRIM, RED};
use std::f32::consts::PI;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const MARBLE: C3 = [0.5, 0.45, 0.39];
const BRONZE: C3 = [0.34, 0.2, 0.09];

/// Dais tiers: (depth from the wall, width inset, top height).
pub const DAIS: [(f32, f32, f32); 3] = [(1.55, 0.0, 0.15), (1.26, 0.5, 0.3), (1.0, 1.0, 0.45)];
pub const STATUE_SCALE: f32 = 1.28;

pub fn statue_base(wall: Vec3, inward: Vec3) -> Vec3 {
    wall + inward * 0.5 + Y(DAIS[2].2)
}

fn lerp3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

#[allow(clippy::too_many_arguments)]
pub fn shrine(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    wall: Vec3,
    inward: Vec3,
    width: f32,
    stone: C3,
    glow_color: C3,
    rng: &mut ArtRng,
) {
    let f = norm(hz(inward));
    let s = v(f.z, 0.0, -f.x);
    let dais_stone = tint(stone, 1.08);
    for (k, &(depth, inset, top)) in DAIS.iter().enumerate() {
        let hw = (width - inset) * 0.5;
        kit_block(
            m,
            wall + f * ((depth - 0.1) * 0.5) + Y(top * 0.5),
            s,
            VY,
            f,
            v(hw, top * 0.5, (depth + 0.1) * 0.5),
            tint(dais_stone, 0.92 + k as f32 * 0.05),
        );
        kit_block(
            m,
            wall + f * (depth - 0.02) + Y(top - 0.02),
            s,
            VY,
            f,
            v(hw + 0.02, 0.025, 0.04),
            tint(dais_stone, 1.18),
        );
        let inlay = wall + f * (depth + 0.003) + Y(top - 0.075);
        bar(
            m,
            inlay - s * (hw - 0.08),
            inlay + s * (hw - 0.08),
            0.012,
            0.004,
            VY,
            GOLD_TRIM,
        );
        let next = if k + 1 < DAIS.len() {
            DAIS[k + 1].0
        } else {
            0.2
        };
        let rw = 0.46;
        let t0 = wall + f * depth + Y(top + 0.004);
        let t1 = wall + f * next + Y(top + 0.004);
        quad(
            m,
            [t0 - s * rw, t0 + s * rw, t1 + s * rw, t1 - s * rw],
            VY,
            RED,
        );
        let r0 = wall + f * (depth + 0.006) + Y(top);
        let r1 = r0 - Y(0.15);
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
                VY,
                GOLD_TRIM,
            );
        }
        // Offered petals drift across each tread.
        for _ in 0..2 {
            let along = rng.range(-hw * 0.7, hw * 0.7);
            petals(
                m,
                wall + f * ((depth + next) * 0.5) + s * along + Y(top),
                (0.16f32).min((depth - next) * 0.45),
                6,
                rng,
                0.0,
                0.0,
            );
        }
    }
    let base = statue_base(wall, f);
    let mut body = empty();
    let mut eyes = empty();
    guardian(&mut body, &mut eyes, base, f, glow_color);
    merge_scaled(m, &body, base, STATUE_SCALE);
    merge_scaled(glow, &eyes, base, STATUE_SCALE);
}

// ------------------------------------------------ guardian statue

/// Radius of a (height, radius) profile at `y` (linear between samples).
fn profile_at(profile: &[(f32, f32)], y: f32) -> f32 {
    if y <= profile[0].0 {
        return profile[0].1;
    }
    for k in 1..profile.len() {
        let (y1, r1) = profile[k];
        if y <= y1 {
            let (y0, r0) = profile[k - 1];
            return r0 + (r1 - r0) * (y - y0) / (y1 - y0);
        }
    }
    profile[profile.len() - 1].1
}

/// The point a fraction `t` along a polyline.
fn along(pts: &[Vec3], t: f32) -> Vec3 {
    let x = t.clamp(0.0, 1.0) * (pts.len() - 1) as f32;
    let k = (pts.len() - 2).min(x.floor() as usize);
    lerp3(pts[k], pts[k + 1], x - k as f32)
}

// Silhouette of the guardian's robe: (height above the pedestal, radius).
const ROBE: [(f32, f32); 13] = [
    (0.0, 0.41),
    (0.04, 0.42),
    (0.3, 0.375),
    (0.6, 0.325),
    (0.85, 0.275),
    (0.98, 0.245),
    (1.12, 0.255),
    (1.26, 0.28),
    (1.36, 0.295),
    (1.44, 0.285),
    (1.5, 0.2),
    (1.55, 0.11),
    (1.62, 0.085),
];

/// The Guardian of the Embers: a hooded, winged figure in a folded robe and
/// an open cloak, hands crossed on a planted greatsword with glowing runes,
/// under a rayed sun halo.
fn guardian(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, facing: Vec3, glow_color: C3) {
    let f = norm(hz(facing));
    let s = v(f.z, 0.0, -f.x);
    let at = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    let gold = GOLD_TRIM;
    let marble = |k: f32| tint(MARBLE, k);

    // Pedestal: plinth, die with a gilded plaque, cornice.
    chamfer_box(
        m,
        at(0.0, 0.07, 0.0),
        s,
        VY,
        f,
        v(0.46, 0.07, 0.43),
        0.025,
        marble(0.74),
    );
    chamfer_box(
        m,
        at(0.0, 0.2, 0.0),
        s,
        VY,
        f,
        v(0.4, 0.07, 0.36),
        0.02,
        marble(0.84),
    );
    chamfer_box(
        m,
        at(0.0, 0.29, 0.0),
        s,
        VY,
        f,
        v(0.45, 0.025, 0.42),
        0.015,
        marble(0.95),
    );
    kit_block(
        m,
        at(0.0, 0.2, 0.364),
        s,
        VY,
        f,
        v(0.2, 0.042, 0.006),
        tint(gold, 0.85),
    );
    star(m, at(0.0, 0.2, 0.372), f, VY, 0.036, 0.016, 8, gold);
    disc(
        glow,
        at(0.0, 0.2, 0.376),
        f,
        0.011,
        8,
        tint(glow_color, 0.7),
    );

    let y0 = 0.315;
    let depth_of = |y: f32| lerp(0.72, 0.64, smooth(0.85, 1.2, y));
    let axis = |y: f32| at(0.0, y0 + y, 0.0);
    let ring =
        |y: f32, r: f32, a: f32, ez: f32| axis(y) + s * (a.cos() * r) + f * (a.sin() * r * ez);

    // Robe: folds deepen toward the hem, their valleys baked darker.
    const ROWS: usize = 30;
    const COLS: usize = 44;
    let fold = |a: f32, y: f32| (11.0 * a + 0.6 * (3.0 * a).sin() + 0.8 * y).cos();
    let fold_amp = |y: f32| 0.075 * (1.0 - smooth(0.1, 0.95, y)) + 0.01;
    let robe_y = |i: usize| (i as f32 / ROWS as f32) * 1.62;
    grid_surface(
        m,
        ROWS,
        COLS,
        &|i, j| {
            let y = robe_y(i);
            let a = (j as f32 / COLS as f32) * TAU;
            ring(
                y,
                profile_at(&ROBE, y) * (1.0 + fold_amp(y) * fold(a, y)),
                a,
                depth_of(y),
            )
        },
        &|i, j| {
            let y = robe_y(i);
            let cavity = fold((j as f32 / COLS as f32) * TAU, y) * (fold_amp(y) / 0.05).min(1.0);
            marble(
                (0.88 + 0.12 * cavity)
                    * (if y < 0.05 { 0.82 } else { 1.0 })
                    * (0.94 + 0.06 * (y / 1.62)),
            )
        },
        true,
        Some(&|p, i, _| p - axis(robe_y(i))),
        None,
    );

    // Belt with a gilded buckle.
    let belt_y = 0.97;
    let belt_r = profile_at(&ROBE, belt_y) * (1.0 + fold_amp(belt_y)) + 0.012;
    let belt: Vec<Vec3> = (0..33)
        .map(|k| ring(belt_y, belt_r, (k as f32 / 32.0) * TAU, depth_of(belt_y)))
        .collect();
    tube_c(m, &belt, 0.022, 6, tint(gold, 0.8));
    chamfer_box(
        m,
        ring(belt_y, belt_r + 0.02, PI / 2.0, depth_of(belt_y)),
        s,
        VY,
        f,
        v(0.05, 0.045, 0.012),
        0.008,
        gold,
    );

    // Stole hanging from the belt, gilded edges, a fringe and the sun emblem.
    const SR: usize = 16;
    const SC: usize = 4;
    let stole_y = |i: usize| 0.95 - (i as f32 / SR as f32) * 0.86;
    let stole_at = |i: usize, j: usize| {
        let y = stole_y(i);
        let x = ((j as f32 / SC as f32) * 2.0 - 1.0) * lerp(0.075, 0.1, 1.0 - (y - 0.09) / 0.86);
        let r = profile_at(&ROBE, y) * (1.0 + fold_amp(y)) + 0.012;
        let z = depth_of(y) * (r * r - x * x).max(0.0).sqrt();
        axis(y) + s * x + f * z
    };
    grid_surface(
        m,
        SR,
        SC,
        &stole_at,
        &|i, _| marble(1.03 - 0.07 * (i as f32 / SR as f32)),
        false,
        Some(&|_, _, _| f),
        None,
    );
    for j in [0, SC] {
        let edge: Vec<Vec3> = (0..=SR).map(|i| stole_at(i, j)).collect();
        tube_c(m, &edge, 0.01, 5, gold);
    }
    for k in 0..=6 {
        let p = lerp3(stole_at(SR, 0), stole_at(SR, SC), k as f32 / 6.0);
        bar(m, p, p - Y(0.05), 0.006, 0.004, f, tint(gold, 0.9));
    }
    let emblem = stole_at((SR as f32 * 0.42).round() as usize, SC / 2) + f * 0.006;
    star(m, emblem, f, VY, 0.07, 0.032, 8, gold);
    disc(m, emblem + f * 0.003, f, 0.028, 12, tint(gold, 1.1));
    disc(
        glow,
        emblem + f * 0.006,
        f,
        0.012,
        8,
        tint(glow_color, 0.75),
    );

    // Open cloak falling from the shoulders, gilded along its edges and hem.
    const CR: usize = 24;
    const CC: usize = 40;
    let cloak_y = |i: usize| 0.02 + (i as f32 / CR as f32) * 1.5;
    let gap = |y: f32| lerp(1.05, 0.3, smooth(0.0, 1.44, y));
    let cloak_r = |y: f32| {
        if y < 1.44 {
            (profile_at(&ROBE, y) + 0.035).max(lerp(0.47, 0.32, y / 1.44))
        } else {
            lerp(0.32, 0.22, (y - 1.44) / 0.08)
        }
    };
    let cloak_fold = |a: f32, y: f32| (7.0 * a + 0.5 * (2.0 * a).sin() + 1.1 * y).cos();
    let cloak_amp = |y: f32| 0.085 * (1.0 - smooth(0.2, 1.3, y)) + 0.012;
    let cloak_a = |i: usize, j: usize| {
        let g = gap(cloak_y(i));
        PI / 2.0 + g + (j as f32 / CC as f32) * (TAU - 2.0 * g)
    };
    let cloak_at = |i: usize, j: usize| {
        let y = cloak_y(i);
        let a = cloak_a(i, j);
        ring(
            y,
            cloak_r(y) * (1.0 + cloak_amp(y) * cloak_fold(a, y)),
            a,
            lerp(0.86, 0.72, (y / 1.44).min(1.0)),
        )
    };
    grid_surface(
        m,
        CR,
        CC,
        &cloak_at,
        &|i, j| {
            let y = cloak_y(i);
            marble(
                (0.8 + 0.12 * cloak_fold(cloak_a(i, j), y) * (cloak_amp(y) / 0.05).min(1.0))
                    * if y < 0.06 { 0.85 } else { 1.0 },
            )
        },
        false,
        Some(&|p, i, _| p - axis(cloak_y(i))),
        Some(&|_, _| marble(0.48)),
    );
    for j in [0, CC] {
        let edge: Vec<Vec3> = (0..=CR).map(|i| cloak_at(i, j)).collect();
        tube_c(m, &edge, 0.016, 6, gold);
    }
    let hem: Vec<Vec3> = (0..=CC).map(|j| cloak_at(0, j)).collect();
    tube_c(m, &hem, 0.014, 6, tint(gold, 0.9));
    // A sun brooch on a chain closes the collar.
    let collar_l = cloak_at(CR - 2, 0);
    let collar_r = cloak_at(CR - 2, CC);
    let brooch = (collar_l + collar_r) * 0.5 + f * 0.02;
    tube_c(m, &[collar_l, brooch - Y(0.03), collar_r], 0.008, 5, gold);
    star(m, brooch, f, VY, 0.06, 0.03, 8, tint(gold, 1.05));
    disc(m, brooch + f * 0.004, f, 0.03, 12, gold);
    disc(
        glow,
        brooch + f * 0.008,
        f,
        0.015,
        8,
        tint(glow_color, 0.85),
    );

    // Greatsword planted before the guardian, its runes smouldering.
    let g = |y: f32| at(0.0, y0 + y, 0.385);
    let steel = [0.74, 0.72, 0.7];
    let blade = [
        (0.0, 0.0, 0.0),
        (0.08, 0.042, 0.01),
        (0.3, 0.05, 0.013),
        (0.97, 0.062, 0.015),
    ];
    for k in 0..blade.len() - 1 {
        let (ya, wa, ta) = blade[k];
        let (yb, wb, tb) = blade[k + 1];
        for e in [-1.0f32, 1.0] {
            for d in [-1.0f32, 1.0] {
                let q = [
                    g(ya) + s * (e * wa),
                    g(yb) + s * (e * wb),
                    g(yb) + f * (d * tb),
                    g(ya) + f * (d * ta),
                ];
                quad(
                    m,
                    q,
                    norm(f * d + s * (e * 0.3)),
                    tint(
                        steel,
                        if d > 0.0 {
                            if e > 0.0 {
                                1.05
                            } else {
                                0.9
                            }
                        } else {
                            0.7
                        },
                    ),
                );
            }
        }
    }
    for y in [0.34, 0.5, 0.66, 0.82] {
        let c = g(y) + f * 0.017;
        quad(
            glow,
            [c - Y(0.035), c + s * 0.013, c + Y(0.035), c - s * 0.013],
            f,
            tint(glow_color, 0.8),
        );
    }
    tube_c(
        m,
        &bezier(g(0.95) - s * 0.22, g(1.03), g(0.95) + s * 0.22, 10),
        0.022,
        6,
        gold,
    );
    for e in [-1.0f32, 1.0] {
        lathe(
            m,
            g(0.92) + s * (e * 0.222),
            VY,
            &[(0.0, 0.0), (0.02, 0.028), (0.045, 0.024), (0.06, 0.0)],
            6,
            gold,
            false,
        );
    }
    lathe(
        m,
        g(0.99),
        VY,
        &[(0.0, 0.026), (0.23, 0.024)],
        8,
        [0.3, 0.08, 0.06],
        false,
    );
    lathe(
        m,
        g(1.22),
        VY,
        &[
            (0.0, 0.02),
            (0.015, 0.042),
            (0.04, 0.046),
            (0.065, 0.03),
            (0.08, 0.0),
        ],
        8,
        gold,
        false,
    );
    disc(
        glow,
        g(1.262) + f * 0.046,
        f,
        0.016,
        8,
        tint(glow_color, 0.9),
    );

    // Bell sleeves reaching forward; the hands rest on the grip.
    for side in [-1.0f32, 1.0] {
        let shoulder = at(side * 0.27, y0 + 1.38, 0.0);
        let bend = at(side * 0.36, y0 + 1.0, 0.12);
        let wrist = at(
            side * 0.075,
            y0 + if side > 0.0 { 1.12 } else { 1.03 },
            0.35,
        );
        let pts = bezier(shoulder, bend, wrist, 10);
        tube(m, &pts, &|t| 0.075 + 0.05 * t * t, 10, &|t, _| {
            marble(0.88 + 0.1 * t)
        });
        let out = norm(wrist - pts[pts.len() - 2]);
        tube_ring(m, wrist - out * 0.012, out, 0.12, 0.014, 16, marble(1.06));
        tube_ring(
            m,
            wrist - out * 0.03,
            out,
            0.113,
            0.006,
            16,
            tint(gold, 0.8),
        );
        disc(m, wrist - out * 0.004, out, 0.118, 12, marble(0.3));
        chamfer_box(
            m,
            g(if side > 0.0 { 1.12 } else { 1.03 }),
            s,
            VY,
            f,
            v(0.055, 0.042, 0.048),
            0.018,
            marble(1.0),
        );
    }

    // Deep hood; its lining is dark, so the face reads as shadow and embers.
    let hc = at(0.0, y0 + 1.78, 0.02);
    const HR: usize = 14;
    const HC: usize = 28;
    let hood_x = |i: usize| -0.235 + (i as f32 / HR as f32) * 0.36;
    let hood_rho = |x: f32| {
        if x < 0.0 {
            0.225 * (1.0 - (x / 0.235).powi(2)).max(0.0).sqrt()
        } else {
            0.225 - 0.3 * x * x
        }
    };
    let hood_at = |i: usize, j: usize| {
        let x = hood_x(i);
        let u = i as f32 / HR as f32;
        let phi = (j as f32 / HC as f32) * TAU;
        let rho = hood_rho(x);
        let sn = phi.sin();
        let up = sn * rho * 1.18 + 0.075 * sn.max(0.0).powi(6) * (1.0 - u).powf(1.5)
            - 0.05 * (-sn).max(0.0).powi(2);
        let side = phi.cos() * rho * (1.0 + 0.22 * (-sn).max(0.0).powi(2));
        hc + f * x + s * side + Y(up)
    };
    grid_surface(
        m,
        HR,
        HC,
        &hood_at,
        &|i, j| {
            marble(
                0.84 + 0.08 * ((j as f32 / HC as f32) * TAU).sin() + 0.06 * (i as f32 / HR as f32),
            )
        },
        true,
        Some(&|p, _, _| p - hc),
        Some(&|_, _| [0.05, 0.04, 0.035]),
    );
    let rim: Vec<Vec3> = (0..=HC).map(|j| hood_at(HR, j)).collect();
    tube_c(m, &rim, 0.022, 6, marble(1.0));
    // Face: only the chin catches the light; the eyes burn in the shadow.
    let fc = hc + f * 0.035 + Y(-0.03);
    grid_surface(
        m,
        6,
        16,
        &|i, j| {
            let th = (i as f32 / 6.0) * (PI / 2.0);
            let ph = (j as f32 / 16.0) * TAU;
            fc + f * (th.cos() * 0.08)
                + s * (th.sin() * ph.cos() * 0.105)
                + Y(th.sin() * ph.sin() * 0.13)
        },
        &|i, j| {
            let up = ((i as f32 / 6.0) * (PI / 2.0)).sin() * ((j as f32 / 16.0) * TAU).sin();
            marble(0.62 - 0.46 * smooth(-0.35, 0.5, up))
        },
        true,
        Some(&|p, _, _| p - fc + f * 0.05),
        None,
    );
    for side in [-1.0f32, 1.0] {
        disc(
            glow,
            fc + f * 0.077 + s * (side * 0.038) + Y(0.035),
            f,
            0.016,
            8,
            tint(glow_color, 0.95),
        );
    }

    // Wings: layered stone feathers hanging from an arched bone.
    for side in [-1.0f32, 1.0] {
        let arc = bezier(
            at(side * 0.1, y0 + 1.3, -0.19),
            at(side * 0.32, y0 + 1.98, -0.26),
            at(side * 0.54, y0 + 1.7, -0.3),
            14,
        );
        let nw = norm(f * 0.9 + s * (side * 0.42));
        let mut feather = |t: f32, spread: f32, len: f32, width: f32, layer: f32, shade: f32| {
            let root = along(&arc, t) + nw * (0.01 + layer * 0.016);
            let dir = norm(Y(-1.0) + s * (side * spread));
            let across = norm(dir.cross(nw));
            let spine: Vec<(Vec3, f32)> = (0..=6)
                .map(|k| {
                    let u = k as f32 / 6.0;
                    let c = root + dir * (len * u) + s * (side * 0.05 * len * u * u)
                        - f * (0.02 * len * u * u);
                    (
                        c,
                        width
                            * (PI * (0.1 + u * 0.92).min(1.0)).sin().max(0.0).powf(0.5)
                            * (1.0 - 0.3 * u),
                    )
                })
                .collect();
            for k in 0..6 {
                let (c0, w0) = spine[k];
                let (c1, w1) = spine[k + 1];
                let q = [
                    c0 - across * w0,
                    c0 + across * w0,
                    c1 + across * w1,
                    c1 - across * w1,
                ];
                let col = marble(shade * (0.8 + 0.22 * (k as f32 / 6.0)));
                quad(m, q, nw, col);
                quad(m, q.map(|p| p - nw * 0.012), -nw, tint(col, 0.72));
            }
        };
        for k in 0..6 {
            feather(
                0.7 + k as f32 * 0.058,
                0.06 + k as f32 * 0.02,
                1.34 + k as f32 * 0.04,
                0.07,
                0.0,
                0.9 + (k % 2) as f32 * 0.05,
            );
        }
        for k in 0..9 {
            feather(
                0.06 + k as f32 * 0.08,
                0.02 + k as f32 * 0.01,
                0.72 + k as f32 * 0.07,
                0.068,
                1.0,
                0.94 + (k % 2) as f32 * 0.05,
            );
        }
        for k in 0..10 {
            feather(
                0.04 + k as f32 * 0.1,
                0.08,
                0.26 + k as f32 * 0.012,
                0.055,
                2.0,
                1.0,
            );
        }
        tube(m, &arc, &|t| 0.055 - 0.025 * t, 8, &|_, _| marble(0.96));
    }

    // Rayed sun halo behind the hood.
    let halo = hc + Y(0.1) - f * 0.2;
    tube_ring(m, halo, f, 0.36, 0.024, 36, gold);
    tube_ring(m, halo, f, 0.26, 0.011, 28, tint(gold, 0.85));
    for k in 0..24 {
        let a = k as f32 * TAU / 24.0;
        let d = s * a.cos() + Y(a.sin());
        let len = if k % 2 == 0 { 0.16 } else { 0.09 };
        let a0 = halo + d * 0.385;
        let a1 = halo + d * (0.385 + len);
        let w = norm(f.cross(d)) * if k % 2 == 0 { 0.028 } else { 0.02 };
        tri(m, a0 - w, a0 + w, a1, f, gold);
        tri(m, a0 - w, a0 + w, a1, -f, tint(gold, 0.7));
    }
    annulus(
        glow,
        halo + f * 0.02,
        f,
        0.335,
        0.355,
        40,
        tint(glow_color, 0.4),
        0.0,
    );
}

/// Emissive crystal with baked facet contrast, so it reads as cut stone.
#[allow(clippy::too_many_arguments)]
fn ember_crystal(
    glow: &mut Mesh3D,
    base: Vec3,
    radius: f32,
    height: f32,
    color: C3,
    sides: usize,
    phase: f32,
    lean: f32,
) {
    let up = norm(VY + v(phase.cos(), 0.0, phase.sin()) * lean);
    let (u, w) = basis(up);
    let light = norm(v(0.35, 0.8, 0.5));
    let shade = |n: Vec3| 0.3 + 0.52 * n.dot(light).max(0.0) + 0.12 * n.y.max(0.0);
    let ring_at = |k: usize, level: usize| {
        let a = phase + k as f32 * TAU / sides as f32;
        let (h, wd) = match level {
            0 => (0.0, 0.8),
            1 => (0.22, 1.0),
            _ => (0.72, 0.7),
        };
        base + up * (height * h) + u * (a.cos() * radius * wd) + w * (a.sin() * radius * wd)
    };
    let tip = base + up * height;
    let mid = base + up * (height * 0.22);
    for k in 0..sides {
        let (p0, q0) = (ring_at(k, 0), ring_at(k + 1, 0));
        let (p1, q1) = (ring_at(k, 1), ring_at(k + 1, 1));
        let (p2, q2) = (ring_at(k, 2), ring_at(k + 1, 2));
        let side = norm((p1 + q1) * 0.5 - mid);
        let top = norm(side + up * 0.7);
        quad(glow, [p0, q0, q1, p1], side, tint(color, shade(side) * 0.8));
        quad(glow, [p1, q1, q2, p2], side, tint(color, shade(side)));
        tri(glow, p2, q2, tip, top, tint(color, shade(top) * 1.15));
    }
}

/// Ember brazier: stone plinth and baluster, three bronze claws holding a
/// flared bowl with a crenellated gold rim, glowing coals and cut crystals.
#[allow(clippy::too_many_arguments)]
pub fn crystal_brazier(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    k: f32,
    stone: C3,
    glow_color: C3,
    rng: &mut ArtRng,
) {
    let yaw = rng.range(0.0, TAU);
    let f = v(yaw.sin(), 0.0, yaw.cos());
    let s = v(f.z, 0.0, -f.x);
    let gold = GOLD_TRIM;
    chamfer_box(
        m,
        base + Y(0.06 * k),
        s,
        VY,
        f,
        v(0.25 * k, 0.06 * k, 0.25 * k),
        0.02 * k,
        tint(stone, 0.9),
    );
    chamfer_box(
        m,
        base + Y(0.135 * k),
        s,
        VY,
        f,
        v(0.2 * k, 0.015 * k, 0.2 * k),
        0.008 * k,
        tint(stone, 1.08),
    );
    let shades = [0.8, 0.95, 1.0, 0.9, 1.05, 1.1];
    let prof: Vec<(f32, f32)> = [
        (0.0, 0.15),
        (0.04, 0.13),
        (0.08, 0.1),
        (0.4, 0.08),
        (0.46, 0.1),
        (0.5, 0.12),
    ]
    .iter()
    .map(|&(h, r)| (h * k, r * k))
    .collect();
    lathe_fn(
        m,
        base + Y(0.15 * k),
        VY,
        &prof,
        10,
        &|j| tint(stone, shades[j.min(5)]),
        true,
    );
    tube_ring(
        m,
        base + Y(0.42 * k),
        VY,
        0.085 * k,
        0.012 * k,
        14,
        tint(gold, 0.8),
    );
    let bowl_y = 0.66 * k;
    let bowl: Vec<(f32, f32)> = [
        (0.0, 0.11),
        (0.04, 0.2),
        (0.1, 0.3),
        (0.16, 0.36),
        (0.19, 0.38),
    ]
    .iter()
    .map(|&(h, r)| (h * k, r * k))
    .collect();
    lathe_fn(
        m,
        base + Y(bowl_y),
        VY,
        &bowl,
        16,
        &|j| tint(BRONZE, 0.85 + j as f32 * 0.1),
        true,
    );
    for c in 0..3 {
        let a = yaw + c as f32 * TAU / 3.0;
        let d = v(a.cos(), 0.0, a.sin());
        let claw = bezier(
            base + Y(0.56 * k) + d * (0.09 * k),
            base + Y(0.62 * k) + d * (0.32 * k),
            base + Y(0.84 * k) + d * (0.385 * k),
            8,
        );
        tube(m, &claw, &|t| (0.022 - 0.01 * t) * k, 5, &|_, _| {
            tint(BRONZE, 1.25)
        });
    }
    let rim_y = bowl_y + 0.19 * k;
    tube_ring(
        m,
        base + Y(rim_y),
        VY,
        0.38 * k,
        0.018 * k,
        24,
        tint(gold, 0.85),
    );
    for c in 0..12 {
        let a = yaw + c as f32 * TAU / 12.0;
        let d = v(a.cos(), 0.0, a.sin());
        let t = v(-d.z, 0.0, d.x);
        let p = base + Y(rim_y) + d * (0.38 * k);
        let h = (if c % 2 == 1 { 0.055 } else { 0.1 }) * k;
        tri(
            m,
            p - t * (0.035 * k),
            p + t * (0.035 * k),
            p + Y(h),
            d,
            tint(gold, 0.95),
        );
        tri(
            m,
            p - t * (0.035 * k),
            p + t * (0.035 * k),
            p + Y(h),
            -d,
            tint(gold, 0.6),
        );
    }
    // A bed of coals, glowing between the dark lumps.
    let coal_y = rim_y - 0.03 * k;
    disc(
        m,
        base + Y(coal_y - 0.012 * k),
        VY,
        0.35 * k,
        16,
        [0.08, 0.05, 0.04],
    );
    for _ in 0..9 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.0, 0.27) * k;
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            base + v(a.cos() * r, coal_y - 0.02 * k, a.sin() * r),
            v(0.07, 0.05, 0.06) * k,
            [0.11, 0.07, 0.05],
            phase,
        );
    }
    for _ in 0..7 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.05, 0.3) * k;
        let shade = rng.range(0.35, 0.55);
        let phase = rng.range(0.0, TAU);
        rock(
            glow,
            base + v(a.cos() * r, coal_y - 0.015 * k, a.sin() * r),
            v(0.045, 0.03, 0.04) * k,
            tint(glow_color, shade),
            phase,
        );
    }
    for i in 0..5 {
        let a = i as f32 * TAU / 5.0 + rng.range(-0.3, 0.3);
        let r = if i == 0 {
            0.0
        } else {
            rng.range(0.1, 0.19) * k
        };
        let p = base + Y(coal_y) + v(a.cos() * r, 0.0, a.sin() * r);
        let cr = (if i == 0 {
            0.085
        } else {
            rng.range(0.045, 0.065)
        }) * k;
        let ch = (if i == 0 { 0.5 } else { rng.range(0.2, 0.34) }) * k;
        ember_crystal(
            glow,
            p,
            cr,
            ch,
            tint(glow_color, 0.72),
            if i == 0 { 6 } else { 5 },
            a,
            if i == 0 { 0.0 } else { 0.35 },
        );
    }
}

// ------------------------------------------------ rest font

/// Glowing rune scratched into a face: a stem and two ticks.
fn rune(glow: &mut Mesh3D, c: Vec3, n: Vec3, up: Vec3, size: f32, color: C3, variant: usize) {
    let side = norm(up.cross(n));
    let mut stroke = |a: Vec3, b: Vec3, w: f32| {
        let d = norm(b - a);
        let t = norm(n.cross(d)) * w;
        quad(glow, [a - t, b - t, b + t, a + t], n, color);
    };
    let w = size * 0.09;
    stroke(c - up * (size * 0.5), c + up * (size * 0.5), w);
    match variant % 3 {
        0 => {
            stroke(
                c + up * (size * 0.1),
                c + up * (size * 0.42) + side * (size * 0.3),
                w,
            );
            stroke(c - up * (size * 0.2), c + side * (size * 0.28), w);
        }
        1 => {
            stroke(c + up * (size * 0.3), c + side * (size * 0.3), w);
            stroke(c + side * (size * 0.3), c - up * (size * 0.3), w);
        }
        _ => {
            stroke(
                c + up * (size * 0.35) - side * (size * 0.28),
                c + up * (size * 0.35) + side * (size * 0.28),
                w,
            );
            stroke(
                c - up * (size * 0.1) - side * (size * 0.25),
                c - up * (size * 0.35),
                w,
            );
        }
    }
}

/// The rest font: a two-step octagonal plinth whose moat of ember water is
/// fed by four spouts, a fluted baluster and a wide gilded basin.
pub fn rest_font(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    facing: Vec3,
    stone: C3,
    glow_color: C3,
) {
    let f = norm(hz(facing));
    let s = v(f.z, 0.0, -f.x);
    let at = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    let gold = GOLD_TRIM;
    let water = |k: f32| tint(glow_color, k);
    let oct = |r: f32, y: f32| -> Vec<Vec3> {
        (0..9)
            .map(|k| {
                let a = PI / 8.0 + (k % 8) as f32 * TAU / 8.0;
                at(a.cos() * r, y, a.sin() * r)
            })
            .collect()
    };
    let walls = |m: &mut Mesh3D, r: f32, y0: f32, y1: f32, color: C3, facing_in: bool| {
        let lo = oct(r, y0);
        let hi = oct(r, y1);
        for k in 0..8 {
            let n = norm((lo[k] + lo[k + 1]) * 0.5 - at(0.0, y0, 0.0));
            quad(
                m,
                [lo[k], lo[k + 1], hi[k + 1], hi[k]],
                if facing_in { -n } else { n },
                tint(color, 0.9 + (k % 2) as f32 * 0.08),
            );
        }
    };
    let band = |target: &mut Mesh3D, r0: f32, r1: f32, y: f32, color: &dyn Fn(usize) -> C3| {
        let a = oct(r0, y);
        let b = oct(r1, y);
        for k in 0..8 {
            quad(target, [a[k], a[k + 1], b[k + 1], b[k]], VY, color(k));
        }
    };
    // Step one, gilded along its edge.
    walls(m, 1.12, 0.0, 0.08, tint(stone, 0.8), false);
    band(m, 0.9, 1.12, 0.08, &|k| {
        tint(stone, 0.95 + (k % 2) as f32 * 0.05)
    });
    let edge = oct(1.08, 0.083);
    for k in 0..8 {
        bar(m, edge[k], edge[k + 1], 0.012, 0.003, VY, tint(gold, 0.9));
    }
    // Step two holds a moat of ember water; runes glow on its faces.
    walls(m, 0.95, 0.08, 0.22, tint(stone, 1.04), false);
    band(m, 0.86, 0.95, 0.22, &|_| tint(stone, 1.14));
    walls(m, 0.86, 0.17, 0.22, tint(stone, 0.62), true);
    band(glow, 0.6, 0.86, 0.175, &|_| water(0.42));
    for r in [0.66, 0.73] {
        annulus(
            glow,
            at(0.0, 0.178, 0.0),
            VY,
            r,
            r + 0.012,
            32,
            water(0.66),
            0.0,
        );
    }
    walls(m, 0.6, 0.17, 0.26, tint(stone, 0.92), false);
    let curb = oct(0.6, 0.26);
    for k in 0..8 {
        tri(
            m,
            at(0.0, 0.26, 0.0),
            curb[k],
            curb[k + 1],
            VY,
            tint(stone, 1.0 + (k % 2) as f32 * 0.05),
        );
    }
    let faces = oct(0.95, 0.15);
    for k in 0..8 {
        let mid = (faces[k] + faces[k + 1]) * 0.5;
        let n = norm(mid - at(0.0, 0.15, 0.0));
        rune(glow, mid + n * 0.004, n, VY, 0.08, water(0.7), k);
    }
    // Fluted baluster with a gilded ring.
    const PED: [(f32, f32); 9] = [
        (0.26, 0.27),
        (0.3, 0.27),
        (0.33, 0.22),
        (0.38, 0.17),
        (0.5, 0.145),
        (0.6, 0.145),
        (0.66, 0.17),
        (0.71, 0.23),
        (0.75, 0.27),
    ];
    let flute = |y: f32, a: f32| {
        smooth(0.36, 0.4, y) * (1.0 - smooth(0.6, 0.64, y)) * (0.5 + 0.5 * (12.0 * a).cos())
    };
    let ped_y = |i: usize| 0.26 + (i as f32 / 18.0) * 0.49;
    grid_surface(
        m,
        18,
        48,
        &|i, j| {
            let y = ped_y(i);
            let a = (j as f32 / 48.0) * TAU;
            let r = profile_at(&PED, y) * (1.0 - 0.1 * flute(y, a));
            at(a.cos() * r, y, a.sin() * r)
        },
        &|i, j| {
            tint(
                stone,
                1.04 - 0.24 * flute(ped_y(i), (j as f32 / 48.0) * TAU),
            )
        },
        true,
        Some(&|p, i, _| p - at(0.0, ped_y(i), 0.0)),
        None,
    );
    tube_ring(m, at(0.0, 0.5, 0.0), VY, 0.15, 0.012, 16, gold);
    // Basin: a thick lip, a gilded band, studs and dark inner walls.
    let lip = [0.85, 0.92, 1.0, 1.05, 1.1];
    lathe_fn(
        m,
        at(0.0, 0.75, 0.0),
        VY,
        &[
            (0.0, 0.27),
            (0.05, 0.4),
            (0.12, 0.52),
            (0.2, 0.6),
            (0.25, 0.63),
            (0.27, 0.63),
        ],
        20,
        &|j| tint(stone, lip[j.min(4)]),
        false,
    );
    annulus(
        m,
        at(0.0, 1.02, 0.0),
        VY,
        0.53,
        0.63,
        40,
        tint(stone, 1.16),
        0.0,
    );
    lathe(
        m,
        at(0.0, 1.02, 0.0),
        VY,
        &[(0.0, 0.53), (-0.07, 0.51)],
        20,
        tint(stone, 0.55),
        false,
    );
    tube_ring(m, at(0.0, 0.995, 0.0), VY, 0.632, 0.012, 32, gold);
    for k in 0..8 {
        let a = k as f32 * TAU / 8.0;
        let d = s * a.cos() + f * a.sin();
        lathe(
            m,
            base + d * 0.625 + Y(0.93),
            d,
            &[(0.0, 0.028), (0.014, 0.022), (0.026, 0.0)],
            6,
            gold,
            false,
        );
    }
    // Ember water with slow rings of light.
    disc(glow, at(0.0, 0.958, 0.0), VY, 0.515, 32, water(0.5));
    for (r, k) in [(0.1, 0.95), (0.24, 0.72), (0.38, 0.64)] {
        annulus(glow, at(0.0, 0.96, 0.0), VY, r, r + 0.02, 32, water(k), 0.0);
    }
    // Four spouts pour the basin into the moat.
    for k in 0..4 {
        let a = PI / 8.0 + k as f32 * PI / 2.0;
        let d = s * a.cos() + f * a.sin();
        let t = v(-d.z, 0.0, d.x);
        let lip = base + d * 0.7 + Y(0.955);
        chamfer_box(
            m,
            lip,
            d,
            VY,
            t,
            v(0.09, 0.035, 0.055),
            0.012,
            tint(stone, 1.1),
        );
        tube_ring(m, lip + d * 0.03, d, 0.05, 0.01, 12, gold);
        let stream = bezier(
            base + d * 0.77 + Y(0.955),
            base + d * 0.9 + Y(0.93),
            base + d * 0.8 + Y(0.18),
            16,
        );
        let kf = k as f32;
        tube(
            glow,
            &stream,
            &|u| (0.03 - 0.008 * u) * (1.0 + 0.22 * (u * 23.0 + kf).sin()),
            7,
            &|u, around| water(0.36 + 0.22 * (around * TAU).cos().max(0.0) + 0.12 * (1.0 - u)),
        );
        annulus(
            glow,
            base + d * 0.8 + Y(0.181),
            VY,
            0.03,
            0.085,
            12,
            water(0.95),
            0.0,
        );
        annulus(
            glow,
            base + d * 0.8 + Y(0.18),
            VY,
            0.1,
            0.115,
            16,
            water(0.7),
            0.0,
        );
    }
}

/// Floating ember core of the rest font: a cut bipyramid (scene-animated).
pub fn font_core_mesh(color: C3) -> Mesh3D {
    let mut glow = empty();
    let sides = 6;
    let light = norm(v(0.35, 0.8, 0.5));
    let shade = |n: Vec3| 1.8 * (0.34 + 0.5 * n.dot(light).max(0.0) + 0.14 * n.y.abs());
    let top = v(0.0, 0.3, 0.0);
    let bottom = v(0.0, -0.2, 0.0);
    let ring_at = |k: usize, y: f32, r: f32| {
        v(
            (k as f32 * TAU / sides as f32).cos() * r,
            y,
            (k as f32 * TAU / sides as f32).sin() * r,
        )
    };
    for k in 0..sides {
        let (a, b) = (ring_at(k, 0.02, 0.12), ring_at(k + 1, 0.02, 0.12));
        let (a2, b2) = (ring_at(k, 0.12, 0.1), ring_at(k + 1, 0.12, 0.1));
        let out = norm(with_y((a + b) * 0.5, 0.0));
        quad(&mut glow, [a, b, b2, a2], out, tint(color, shade(out)));
        let up = norm(out + Y(0.6));
        tri(&mut glow, a2, b2, top, up, tint(color, shade(up) * 1.2));
        let down = norm(out + Y(-0.8));
        tri(
            &mut glow,
            a,
            b,
            bottom,
            down,
            tint(color, shade(down) * 0.8),
        );
    }
    glow
}

// ------------------------------------------------ nave dressing

/// Sun mosaic set into the floor around the font.
pub fn sun_mosaic(
    m: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    facing: Vec3,
    stone: C3,
    rng: &mut ArtRng,
) {
    let f = norm(hz(facing));
    let phase = f.z.atan2(f.x);
    let k = radius / 1.9;
    let c = center + Y(0.012);
    let p = |a: f32, r: f32| c + v(a.cos() * r * k, 0.002, a.sin() * r * k);
    disc(m, c, VY, radius, 48, tint(stone, 0.45));
    for i in 0..16 {
        let a = phase + i as f32 * TAU / 16.0;
        let long = i % 2 == 0;
        let hw = TAU / 60.0;
        tri(
            m,
            p(a - hw, 1.04),
            p(a + hw, 1.04),
            p(a, if long { 1.46 } else { 1.34 }),
            VY,
            if long {
                tint(GOLD_TRIM, 0.95)
            } else {
                tint(RED, 0.9)
            },
        );
    }
    annulus(m, c + Y(0.003), VY, 1.47 * k, 1.5 * k, 64, GOLD_TRIM, 0.0);
    annulus(m, c + Y(0.003), VY, 1.83 * k, 1.86 * k, 64, GOLD_TRIM, 0.0);
    for (r0, r1, count, shift) in [(1.52, 1.66, 40, 0.0), (1.68, 1.81, 46, 0.5)] {
        for i in 0..count {
            let a0 = phase + (i as f32 + shift + 0.07) * TAU / count as f32;
            let a1 = phase + (i as f32 + shift + 0.93) * TAU / count as f32;
            let col = if i % 6 == 0 {
                tint(GOLD_TRIM, 0.9)
            } else if i % 2 == 0 {
                tint(RED, 0.75 + rng.range(0.0, 0.18))
            } else {
                tint(stone, 1.0 + rng.range(-0.1, 0.1))
            };
            quad(
                m,
                [
                    p(a0, r0 + 0.008),
                    p(a1, r0 + 0.008),
                    p(a1, r1 - 0.008),
                    p(a0, r1 - 0.008),
                ],
                VY,
                col,
            );
        }
    }
}

/// Red prayer cushion with gold tassels and a sun button.
pub fn cushion(m: &mut Mesh3D, base: Vec3, yaw: f32) {
    let f = v(yaw.sin(), 0.0, yaw.cos());
    let s = v(f.z, 0.0, -f.x);
    chamfer_box(m, base + Y(0.05), s, VY, f, v(0.2, 0.05, 0.16), 0.035, RED);
    chamfer_box(
        m,
        base + Y(0.092),
        s,
        VY,
        f,
        v(0.15, 0.012, 0.11),
        0.01,
        tint(RED, 1.15),
    );
    disc(m, base + Y(0.106), VY, 0.03, 10, GOLD_TRIM);
    for (x, z) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let corner = base + s * (x * 0.19) + f * (z * 0.15) + Y(0.07);
        lathe(
            m,
            corner,
            VY,
            &[(-0.06, 0.0), (-0.04, 0.018), (0.0, 0.01)],
            5,
            GOLD_TRIM,
            false,
        );
    }
}

/// Scattered red petals between radii `inner` and `radius`, `y` above `center`.
pub fn petals(
    m: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    count: usize,
    rng: &mut ArtRng,
    y: f32,
    inner: f32,
) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let r = (inner * inner + (radius * radius - inner * inner) * rng.unit()).sqrt();
        let p = center + v(a.cos() * r, 0.006 + y, a.sin() * r);
        let turn = rng.range(0.0, TAU);
        let d = v(turn.cos(), 0.0, turn.sin()) * rng.range(0.02, 0.032);
        let t = v(-d.z, 0.0, d.x) * 0.6;
        let shade = rng.range(0.8, 1.3);
        quad(
            m,
            [p - d, p + t, p + d, p - t],
            VY,
            tint([0.62, 0.06, 0.08], shade),
        );
    }
}

/// Bronze censer on a chain, embers glowing through its pierced lid.
pub fn censer(m: &mut Mesh3D, glow: &mut Mesh3D, top: Vec3, length: f32, glow_color: C3) {
    let iron = [0.14, 0.12, 0.11];
    let mut y = 0.0;
    let mut alt = false;
    while y < length {
        bar(
            m,
            top - Y(y),
            top - Y(y + 0.09),
            0.02,
            0.007,
            if alt { VX } else { VZ },
            tint(iron, 1.3),
        );
        y += 0.075;
        alt = !alt;
    }
    let c = top - Y(length + 0.16);
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        bar(
            m,
            top - Y(length),
            c + v(a.cos() * 0.1, 0.05, a.sin() * 0.1),
            0.005,
            0.005,
            VY,
            tint(GOLD_TRIM, 0.8),
        );
    }
    lathe_fn(
        m,
        c - Y(0.1),
        VY,
        &[(0.0, 0.03), (0.03, 0.08), (0.1, 0.11), (0.15, 0.1)],
        10,
        &|j| tint(BRONZE, 0.9 + j as f32 * 0.12),
        true,
    );
    lathe(
        m,
        c + Y(0.05),
        VY,
        &[(0.0, 0.1), (0.05, 0.08), (0.1, 0.04), (0.13, 0.012)],
        10,
        tint(BRONZE, 1.2),
        false,
    );
    tube_ring(m, c + Y(0.05), VY, 0.105, 0.01, 16, GOLD_TRIM);
    for k in 0..6 {
        let a = k as f32 * TAU / 6.0;
        disc(
            glow,
            c + v(a.cos() * 0.087, 0.08, a.sin() * 0.087),
            norm(v(a.cos(), 0.5, a.sin())),
            0.016,
            6,
            tint(glow_color, 0.8),
        );
    }
}

/// Carved sun: stone disc, ring, alternating rays and a gilded eye.
#[allow(clippy::too_many_arguments)]
pub fn sun_relief(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    r: f32,
    stone: C3,
    glow_color: C3,
) {
    let up = VY;
    let side = norm(up.cross(n));
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
        let t = norm(n.cross(d));
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
pub fn prayer_rug(m: &mut Mesh3D, glow: &mut Mesh3D, c: Vec3, along: Vec3, half: [f32; 2]) {
    let a = norm(hz(along));
    let s = v(a.z, 0.0, -a.x);
    let rect = |m: &mut Mesh3D, y: f32, ha: f32, hs: f32, col: C3| {
        let p = |u: f32, w: f32| c + a * u + s * w + Y(y);
        quad(m, [p(-ha, -hs), p(ha, -hs), p(ha, hs), p(-ha, hs)], VY, col);
    };
    rect(m, 0.01, half[0], half[1], DEEP_RED);
    rect(m, 0.011, half[0] - 0.1, half[1] - 0.1, GOLD_TRIM);
    rect(m, 0.012, half[0] - 0.14, half[1] - 0.14, tint(RED, 0.9));
    rect(m, 0.013, half[0] - 0.3, half[1] - 0.3, tint(RED, 0.78));
    rect(m, 0.014, half[0] - 0.34, half[1] - 0.34, tint(RED, 0.9));
    let r = half[0].min(half[1]) * 0.55;
    annulus(m, c + Y(0.016), VY, r * 0.82, r * 0.9, 36, GOLD_TRIM, 0.0);
    star(m, c + Y(0.017), VY, a, r * 0.78, r * 0.34, 12, GOLD_TRIM);
    disc(m, c + Y(0.018), VY, r * 0.3, 20, DEEP_RED);
    annulus(
        glow,
        c + Y(0.019),
        VY,
        r * 0.855,
        r * 0.865,
        48,
        [0.3, 0.16, 0.03],
        0.0,
    );
    for (u, w) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        let p = c + a * (u * (half[0] - 0.22)) + s * (w * (half[1] - 0.22)) + Y(0.016);
        quad(
            m,
            [p - a * 0.07, p + s * 0.07, p + a * 0.07, p - s * 0.07],
            VY,
            GOLD_TRIM,
        );
    }
    for end in [-1.0f32, 1.0] {
        let count = (half[1] * 2.0 / 0.07).floor() as usize;
        for k in 0..count {
            let w = -half[1] + 0.035 + k as f32 * 0.07;
            let p0 = c + a * (end * half[0]) + s * w + Y(0.0105);
            let p1 = p0 + a * (end * 0.09);
            quad(
                m,
                [
                    p0 - s * 0.018,
                    p1 - s * 0.018,
                    p1 + s * 0.018,
                    p0 + s * 0.018,
                ],
                VY,
                [0.92, 0.72, 0.36],
            );
        }
    }
}

/// Votive candles along a tread edge, optionally leaving the centre free.
pub fn candle_row(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    from: Vec3,
    to: Vec3,
    count: usize,
    gap: bool,
    rng: &mut ArtRng,
) {
    let across = norm(hz(to - from));
    let depth = v(-across.z, 0.0, across.x);
    for i in 0..count {
        // f64 like the web version: t = 0.3 / 0.7 sit right on the gap.
        let t64 = (i as f64 + 0.5) / count as f64;
        let t = t64 as f32;
        if gap && (t64 - 0.5).abs() < 0.2 {
            continue;
        }
        if rng.chance(0.18) {
            continue;
        }
        let x = rng.range(-0.03, 0.03);
        let z = rng.range(-0.05, 0.05);
        let p = lerp3(from, to, t) + across * x + depth * z;
        let r = rng.range(0.028, 0.048);
        let h = rng.range(0.08, 0.3);
        candle(m, glow, p, r, h, rng);
    }
}

/// Clay pot with a rolled rim.
pub fn clay_pot(m: &mut Mesh3D, base: Vec3, h: f32) {
    let clay = [0.46, 0.24, 0.13];
    let shades = [0.75, 0.95, 1.05, 1.0, 0.9, 1.1];
    lathe_fn(
        m,
        base,
        VY,
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
        &|j| tint(clay, shades[j.min(5)]),
        true,
    );
    tube_ring(
        m,
        base + Y(0.28 * h),
        VY,
        0.27 * h,
        0.01,
        20,
        tint(clay, 0.6),
    );
    // Painted shoulder: a cream zigzag over a dark line.
    tube_ring(
        m,
        base + Y(0.44 * h),
        VY,
        0.245 * h,
        0.006,
        20,
        [0.12, 0.05, 0.03],
    );
    for k in 0..10 {
        let a0 = k as f32 * TAU / 10.0;
        let a1 = (k as f32 + 0.5) * TAU / 10.0;
        let a2 = (k + 1) as f32 * TAU / 10.0;
        let at = |a: f32, y: f32| base + v(a.cos() * 0.274 * h, y, a.sin() * 0.274 * h);
        tri(
            m,
            at(a0, 0.3 * h),
            at(a2, 0.3 * h),
            at(a1, 0.37 * h),
            v(a1.cos(), 0.0, a1.sin()),
            [0.86, 0.72, 0.5],
        );
    }
}

// ------------------------------------------------ sanctuary additions

/// Free-standing round shrine: three stepped tiers, candles, the statue.
#[allow(clippy::too_many_arguments)]
pub fn round_shrine(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    facing: Vec3,
    stone: C3,
    glow_color: C3,
    rng: &mut ArtRng,
    radius: f32,
) {
    let f = norm(hz(facing));
    let s = v(f.z, 0.0, -f.x);
    let k = radius / 1.25;
    let tiers = [(1.25 * k, 0.16), (0.98 * k, 0.32), (0.72 * k, 0.48)];
    let mut y = 0.0;
    for (tier, &(r, top)) in tiers.iter().enumerate() {
        lathe(
            m,
            base + Y(y),
            VY,
            &[(0.0, r + 0.02), (top - y - 0.03, r), (top - y, r - 0.03)],
            24,
            tint(stone, 1.0 + tier as f32 * 0.05),
            true,
        );
        tube_ring(m, base + Y(top - 0.06), VY, r + 0.004, 0.012, 32, GOLD_TRIM);
        y = top;
    }
    // A red runner falls down the front steps.
    for tier in 0..tiers.len() {
        let (r, top) = tiers[tier];
        let inner = if tier + 1 < tiers.len() {
            tiers[tier + 1].0
        } else {
            0.35
        };
        let t0 = base + f * r + Y(top + 0.004);
        let t1 = base + f * inner + Y(top + 0.004);
        quad(
            m,
            [t0 - s * 0.36, t0 + s * 0.36, t1 + s * 0.36, t1 - s * 0.36],
            VY,
            RED,
        );
        let r0 = base + f * (r + 0.006) + Y(top);
        let r1 = r0 - Y(0.16);
        quad(
            m,
            [r0 - s * 0.36, r0 + s * 0.36, r1 + s * 0.36, r1 - s * 0.36],
            f,
            tint(RED, 0.85),
        );
    }
    // Votive candles ring every tread, leaving the runner clear.
    for (tier, &(r, top)) in tiers.iter().enumerate() {
        let count = (r * 9.0).floor() as usize;
        for i in 0..count {
            let a = i as f32 * TAU / count as f32 + tier as f32 * 0.3;
            let d = v(a.cos(), 0.0, a.sin());
            if d.dot(f) > 0.82 || rng.chance(0.2) {
                continue;
            }
            let inner = if tier + 1 < tiers.len() {
                tiers[tier + 1].0
            } else {
                0.4
            };
            let cr = rng.range(0.028, 0.045);
            let ch = rng.range(0.08, 0.28);
            candle(
                m,
                glow,
                base + d * ((r + inner) * 0.5 + 0.04) + Y(top),
                cr,
                ch,
                rng,
            );
        }
    }
    for (tier, &(r, top)) in tiers.iter().enumerate() {
        let inner = if tier + 1 < tiers.len() {
            tiers[tier + 1].0
        } else {
            0.4
        };
        for _ in 0..3 {
            let a = rng.range(0.0, TAU);
            petals(
                m,
                base + v(a.cos(), 0.0, a.sin()) * ((r + inner) * 0.5) + Y(top),
                (0.14f32).min((r - inner) * 0.4),
                5,
                rng,
                0.0,
                0.0,
            );
        }
    }
    let statue_at = base + Y(tiers[2].1);
    let mut body = empty();
    let mut eyes = empty();
    guardian(&mut body, &mut eyes, statue_at, f, glow_color);
    merge_scaled(m, &body, statue_at, STATUE_SCALE);
    merge_scaled(glow, &eyes, statue_at, STATUE_SCALE);
}

/// Framing arch and pilasters behind the wall statue.
pub fn shrine_arch(m: &mut Mesh3D, wall: Vec3, inward: Vec3, width: f32, stone: C3) {
    let f = norm(hz(inward));
    let s = v(f.z, 0.0, -f.x);
    let hw = (width * 0.5 + 0.05).min(1.35);
    let spring = 2.6;
    for sign in [-1.0f32, 1.0] {
        for k in 0..5 {
            let y0 = k as f32 * spring / 5.0;
            let w = if k % 2 == 0 { 0.24 } else { 0.18 };
            chamfer_box(
                m,
                wall + s * (sign * (hw + w)) + f * 0.12 + Y(y0 + spring / 10.0),
                s,
                VY,
                f,
                v(w - 0.01, spring / 10.0 - 0.01, 0.2),
                0.03,
                tint(stone, 0.92 + (k % 2) as f32 * 0.1),
            );
        }
        chamfer_box(
            m,
            wall + s * (sign * (hw + 0.26)) + f * 0.16 + Y(spring + 0.06),
            s,
            VY,
            f,
            v(0.32, 0.06, 0.24),
            0.02,
            tint(stone, 1.15),
        );
    }
    let pieces = 11;
    for k in 0..pieces {
        let a0 = PI * k as f32 / pieces as f32;
        let a1 = PI * (k + 1) as f32 / pieces as f32;
        let at = |a: f32| wall + s * (-a.cos() * hw) + Y(spring + 0.12 + a.sin() * hw);
        let (p0, p1) = (at(a0), at(a1));
        let tangent = norm(p1 - p0);
        let radial = norm(s * -((a0 + a1) * 0.5).cos() + Y(((a0 + a1) * 0.5).sin()));
        let key = k == pieces / 2;
        chamfer_box(
            m,
            (p0 + p1) * 0.5 + radial * 0.17 + f * 0.12,
            tangent,
            radial,
            f,
            v(
                (p1 - p0).length() * 0.5 - 0.01,
                if key { 0.21 } else { 0.17 },
                0.2,
            ),
            0.03,
            tint(
                stone,
                if key {
                    1.2
                } else {
                    0.98 + (k % 2) as f32 * 0.06
                },
            ),
        );
    }
    // Dark recess inside the arch sets the statue's silhouette off the wall.
    let back = wall + f * 0.02;
    for k in 0..16 {
        let a0 = PI * k as f32 / 16.0;
        let a1 = PI * (k + 1) as f32 / 16.0;
        let p = |a: f32, r: f32| back + s * (-a.cos() * r) + Y(spring + 0.12 + a.sin() * r);
        tri(
            m,
            back + Y(spring + 0.12),
            p(a0, hw - 0.02),
            p(a1, hw - 0.02),
            f,
            tint(stone, 0.45),
        );
    }
    quad(
        m,
        [
            back - s * (hw - 0.02),
            back + s * (hw - 0.02),
            back + s * (hw - 0.02) + Y(spring + 0.12),
            back - s * (hw - 0.02) + Y(spring + 0.12),
        ],
        f,
        tint(stone, 0.45),
    );
}

/// Gilded rune circle around the rest font; it flares when the player rests.
pub fn rest_circle(glow: &mut Mesh3D, centre: Vec3, radius: f32, color: C3) {
    let c = centre + Y(0.024);
    annulus(
        glow,
        c,
        VY,
        radius - 0.05,
        radius,
        64,
        tint(color, 0.55),
        0.0,
    );
    annulus(
        glow,
        c,
        VY,
        radius * 0.8 - 0.025,
        radius * 0.8,
        48,
        tint(color, 0.45),
        0.0,
    );
    for k in 0..12 {
        let a = k as f32 * TAU / 12.0;
        let d = v(a.cos(), 0.0, a.sin());
        let t = v(-d.z, 0.0, d.x);
        let q = c + d * (radius * 0.9);
        let w = if k % 3 == 0 { 0.07 } else { 0.045 };
        quad(
            glow,
            [q - d * w, q + t * (w * 0.7), q + d * w, q - t * (w * 0.7)],
            VY,
            tint(color, 0.6),
        );
    }
}

/// Stone ledge of votive candles against a wall bay.
#[allow(clippy::too_many_arguments)]
pub fn votive_shelf(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    centre: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let hw = width * 0.5;
    chamfer_box(
        m,
        centre + inward * 0.13 + Y(0.98),
        tangent,
        VY,
        inward,
        v(hw, 0.05, 0.2),
        0.015,
        tint(stone, 1.12),
    );
    for s in [-1.0f32, 1.0] {
        chamfer_box(
            m,
            centre + tangent * (s * (hw - 0.12)) + inward * 0.08 + Y(0.78),
            tangent,
            VY,
            inward,
            v(0.06, 0.16, 0.12),
            0.012,
            tint(stone, 0.85),
        );
    }
    let count = ((width / 0.13).floor() as usize).max(4);
    for k in 0..count {
        if rng.chance(0.15) {
            continue;
        }
        let along = -hw
            + 0.1
            + (k as f32 + rng.range(-0.2, 0.2)) * (width - 0.2) / (count.max(2) - 1) as f32;
        let into = rng.range(0.06, 0.22);
        let r = rng.range(0.025, 0.04);
        let h = rng.range(0.06, 0.26);
        candle(
            m,
            glow,
            centre + tangent * along + inward * into + Y(1.03),
            r,
            h,
            rng,
        );
    }
    // Melted wax dripping over the front edge.
    for _ in 0..6 {
        let along = rng.range(-hw + 0.1, hw - 0.1);
        let len = rng.range(0.05, 0.16);
        kit_block(
            m,
            centre + tangent * along + inward * 0.335 + Y(0.96 - len * 0.5),
            tangent,
            VY,
            inward,
            v(0.012, len * 0.5, 0.008),
            [0.9, 0.84, 0.66],
        );
    }
}

/// Bronze offering bowl heaped with coins and petals.
pub fn offering_bowl(m: &mut Mesh3D, base: Vec3, rng: &mut ArtRng) {
    lathe(
        m,
        base,
        VY,
        &[
            (0.0, 0.08),
            (0.04, 0.06),
            (0.08, 0.05),
            (0.12, 0.16),
            (0.18, 0.2),
            (0.2, 0.19),
        ],
        12,
        BRONZE,
        true,
    );
    tube_ring(
        m,
        base + Y(0.2),
        VY,
        0.195,
        0.012,
        20,
        tint(GOLD_TRIM, 0.85),
    );
    for _ in 0..10 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.0, 0.13);
        let y = rng.range(0.0, 0.03);
        let nx = rng.range(-0.3, 0.3);
        let nz = rng.range(-0.3, 0.3);
        let shade = rng.range(0.9, 1.1);
        disc(
            m,
            base + v(a.cos() * r, 0.2 + y, a.sin() * r),
            norm(VY + v(nx, 0.0, nz)),
            0.035,
            8,
            tint(GOLD_TRIM, shade),
        );
    }
    for _ in 0..5 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.03, 0.12);
        disc(
            m,
            base + v(a.cos() * r, 0.225, a.sin() * r),
            VY,
            0.025,
            5,
            [0.75, 0.12, 0.1],
        );
    }
}
