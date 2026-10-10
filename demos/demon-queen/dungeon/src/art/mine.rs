//! La Mina abandonada (web prototype `galleries/mine.js`): a worked-out
//! gallery. Rails run from the entrance into a timbered tunnel mouth in the
//! far wall, with a siding ending at a buffer stop beside a toppled cart;
//! timber sets brace the walls and beams cross the vault under hanging
//! lanterns; ore bursts glowing out of a wall over a spoil heap with a
//! pickaxe in it; a work corner holds a bench, tools, a crate of dynamite,
//! sacks, rope and buckets; gravel and nuggets litter it.
use super::kit::*;
use super::plan::Ctx;
use super::stage::*;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const TIMBER: C3 = [0.3, 0.19, 0.11];
const OLD_TIMBER: C3 = [0.24, 0.16, 0.1];
const ROCK: C3 = [0.3, 0.27, 0.26];
const RUST: C3 = [0.34, 0.16, 0.08];
const ROPE: C3 = [0.62, 0.5, 0.3];
const SACK: C3 = [0.52, 0.44, 0.3];
const DYNAMITE: C3 = [0.72, 0.1, 0.07];

// ---------------------------------------------------------------- models

/// Rails along a path: sleepers, tie plates, spikes and two iron rails.
pub fn rail_path(m: &mut Mesh3D, pts: &[Vec3], rng: &mut ArtRng) {
    let gauge = 0.34;
    let mut acc = 0.3;
    let mut travelled = 0.0;
    let mut left = vec![];
    let mut right = vec![];
    for k in 0..pts.len() {
        let a = pts[k.saturating_sub(1)];
        let b = pts[(k + 1).min(pts.len() - 1)];
        let dir = norm(hz(b - a));
        let n = v(-dir.z, 0.0, dir.x);
        left.push(pts[k] + n * gauge + Y(0.1));
        right.push(pts[k] - n * gauge + Y(0.1));
        if k == 0 {
            continue;
        }
        let seg = hz(pts[k] - pts[k - 1]).length();
        travelled += seg;
        while acc < travelled {
            let t = 1.0 - (travelled - acc) / seg;
            let c = pts[k - 1].lerp(pts[k], t) + Y(0.04);
            let wood = tint(OLD_TIMBER, rng.range(0.8, 1.15));
            kit_block(m, c, n, VY, dir, v(0.56, 0.035, 0.085), wood);
            for sgn in [-1.0, 1.0] {
                let plate = c + n * (sgn * gauge) + Y(0.04);
                kit_block(m, plate, n, VY, dir, v(0.07, 0.008, 0.06), tint(RUST, 0.9));
                for o in [-0.045, 0.045] {
                    kit_block(
                        m,
                        plate + n * o + Y(0.012),
                        n,
                        VY,
                        dir,
                        v(0.012, 0.012, 0.012),
                        tint(IRON, 1.4),
                    );
                }
            }
            acc += rng.range(0.55, 0.66);
        }
    }
    for rail in [&left, &right] {
        for k in 0..rail.len().saturating_sub(1) {
            bar(m, rail[k], rail[k + 1], 0.03, 0.028, VY, tint(RUST, 1.05));
            bar(
                m,
                rail[k] + Y(0.022),
                rail[k + 1] + Y(0.022),
                0.018,
                0.008,
                VY,
                [0.46, 0.42, 0.4],
            );
        }
    }
}

/// Timber buffer stop at the end of a line.
pub fn buffer_stop(m: &mut Mesh3D, base: Vec3, yaw: f32) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    for sgn in [-1.0, 1.0] {
        kit_block(
            m,
            l(sgn * 0.42, 0.42, 0.0),
            s,
            VY,
            f,
            v(0.08, 0.42, 0.08),
            tint(TIMBER, 0.95),
        );
        bar(
            m,
            l(sgn * 0.42, 0.05, -0.6),
            l(sgn * 0.42, 0.62, -0.02),
            0.06,
            0.06,
            s,
            tint(TIMBER, 0.8),
        );
        lathe(
            m,
            l(sgn * 0.34, 0.52, 0.09),
            f,
            &[(0.0, 0.07), (0.08, 0.08), (0.1, 0.05)],
            8,
            tint(RUST, 1.1),
            true,
        );
    }
    kit_block(
        m,
        l(0.0, 0.52, 0.02),
        s,
        VY,
        f,
        v(0.56, 0.1, 0.09),
        tint(TIMBER, 1.1),
    );
    kit_block(
        m,
        l(0.0, 0.53, 0.115),
        s,
        VY,
        f,
        v(0.3, 0.045, 0.006),
        [0.72, 0.62, 0.2],
    );
    for k in -2..=2 {
        kit_block(
            m,
            l(k as f32 * 0.12, 0.53, 0.12),
            s,
            VY,
            f,
            v(0.03, 0.045, 0.004),
            [0.08, 0.06, 0.05],
        );
    }
}

/// Riveted iron-banded ore cart on spoked wheels, heaped with ore.
pub fn mine_cart(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    roll: f32,
    lift: f32,
    ore: C3,
    rng: &mut ArtRng,
) {
    let mut local = empty();
    let mut lglow = empty();
    let (top, bottom, hl, hlb, y0, y1) = (0.52f32, 0.4f32, 0.62f32, 0.5f32, 0.36f32, 0.88f32);
    let corner = |sx: f32, sz: f32, y: f32| {
        let t = (y - y0) / (y1 - y0);
        v(
            sx * (bottom + (top - bottom) * t),
            y,
            sz * (hlb + (hl - hlb) * t),
        )
    };
    let faces = [
        (
            [
                corner(-1.0, -1.0, y0),
                corner(1.0, -1.0, y0),
                corner(1.0, -1.0, y1),
                corner(-1.0, -1.0, y1),
            ],
            -VZ,
        ),
        (
            [
                corner(1.0, 1.0, y0),
                corner(-1.0, 1.0, y0),
                corner(-1.0, 1.0, y1),
                corner(1.0, 1.0, y1),
            ],
            VZ,
        ),
        (
            [
                corner(-1.0, 1.0, y0),
                corner(-1.0, -1.0, y0),
                corner(-1.0, -1.0, y1),
                corner(-1.0, 1.0, y1),
            ],
            -VX,
        ),
        (
            [
                corner(1.0, -1.0, y0),
                corner(1.0, 1.0, y0),
                corner(1.0, 1.0, y1),
                corner(1.0, -1.0, y1),
            ],
            VX,
        ),
    ];
    for (q, n) in faces {
        quad(&mut local, q, n, tint(RUST, 0.95));
        quad(
            &mut local,
            q.map(|p| with_y(p * 0.97, p.y)),
            -n,
            tint(RUST, 0.55),
        );
    }
    quad(
        &mut local,
        [
            corner(-1.0, -1.0, y0),
            corner(1.0, -1.0, y0),
            corner(1.0, 1.0, y0),
            corner(-1.0, 1.0, y0),
        ],
        -VY,
        tint(RUST, 0.5),
    );
    for y in [y0 + 0.04, 0.62, y1 - 0.02] {
        let w = bottom + (top - bottom) * ((y - y0) / (y1 - y0)) + 0.012;
        let l = hlb + (hl - hlb) * ((y - y0) / (y1 - y0)) + 0.012;
        for (a, b) in [
            (v(-w, y, -l), v(w, y, -l)),
            (v(w, y, -l), v(w, y, l)),
            (v(w, y, l), v(-w, y, l)),
            (v(-w, y, l), v(-w, y, -l)),
        ] {
            bar(&mut local, a, b, 0.024, 0.012, VY, tint(IRON, 1.3));
        }
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                for k in [-0.2f32, 0.2] {
                    let _ = sz;
                    kit_block(
                        &mut local,
                        v(sx * w, y, sz * l) + v(k * sx * 0.5, 0.0, 0.0),
                        VX,
                        VY,
                        VZ,
                        v(0.012, 0.012, 0.012),
                        tint(IRON, 1.8),
                    );
                }
            }
        }
    }
    for sx in [-1.0, 1.0] {
        for sz in [-0.36, 0.36] {
            let c = v(sx * 0.46, 0.2, sz);
            tube_ring(&mut local, c, VX, 0.17, 0.028, 16, tint(IRON, 1.25));
            lathe(
                &mut local,
                c - VX * 0.03,
                VX,
                &[(0.0, 0.045), (0.06, 0.04)],
                8,
                tint(IRON, 1.6),
                true,
            );
            for k in 0..6 {
                let a = k as f32 * TAU / 6.0;
                bar(
                    &mut local,
                    c,
                    c + v(0.0, a.sin() * 0.15, a.cos() * 0.15),
                    0.012,
                    0.012,
                    VX,
                    tint(IRON, 1.4),
                );
            }
        }
        bar(
            &mut local,
            v(sx * 0.4, 0.2, -0.46),
            v(sx * 0.4, 0.2, 0.46),
            0.03,
            0.03,
            VY,
            tint(IRON, 1.1),
        );
    }
    for sz in [-1.0, 1.0] {
        bar(
            &mut local,
            v(-0.16, 0.5, sz * (hl + 0.03)),
            v(0.16, 0.5, sz * (hl + 0.03)),
            0.02,
            0.02,
            VY,
            tint(IRON, 1.4),
        );
        tube_ring(
            &mut local,
            v(0.0, 0.42, sz * (hl + 0.06)),
            VX,
            0.04,
            0.01,
            10,
            tint(IRON, 1.5),
        );
    }
    // The load: ore heaped between the walls (its bed meets them a hand
    // below the rim), rocks bedded in it. A toppled cart has spilled its load.
    if roll.abs() < 0.3 {
        let fill = 0.74;
        let t = (fill - y0) / (y1 - y0);
        let w = (bottom + (top - bottom) * t) * 0.97;
        let l = (hlb + (hl - hlb) * t) * 0.97;
        heap_load(
            &mut local,
            &mut lglow,
            &[
                v(-w, fill, -l),
                v(w, fill, -l),
                v(w, fill, l),
                v(-w, fill, l),
            ],
            &Load {
                crown: 0.17,
                rocks: 13,
                size: [0.05, 0.1],
                bed: tint(ROCK, 0.55),
                stone: ROCK,
                ore: Some(ore),
                clear: 0.015,
            },
            rng,
        );
    }
    let map = pose_mapper(base + Y(lift), yaw, 0.0, roll, 1.0);
    append(m, &local, &map, [1.0; 3]);
    append(glow, &lglow, &map, [1.0; 3]);
}

/// Tunnel mouth on a wall: a double timber set with braces and lagging
/// boards, a black opening, a lantern on the cap, a warning board.
pub fn mine_portal(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    hw: f32,
    h: f32,
    rng: &mut ArtRng,
) {
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    quad(
        m,
        [
            l(-hw + 0.1, 0.0, 0.03),
            l(hw - 0.1, 0.0, 0.03),
            l(hw - 0.1, h - 0.1, 0.03),
            l(-hw + 0.1, h - 0.1, 0.03),
        ],
        inward,
        [0.012, 0.01, 0.01],
    );
    for k in 0..4 {
        let z = 0.03 - k as f32 * 0.001;
        let g = 0.03 + k as f32 * 0.012;
        kit_block(
            m,
            l(0.0, 0.01 + k as f32 * 0.001, z + 0.02),
            tangent,
            VY,
            inward,
            v(hw - 0.12, 0.004, 0.02),
            [g, g * 0.9, g * 0.85],
        );
    }
    for depth in [0.14f32, 0.52] {
        let wood = tint(TIMBER, if depth > 0.3 { 1.05 } else { 0.8 });
        for sgn in [-1.0, 1.0] {
            let shade = rng.range(0.9, 1.1);
            kit_block(
                m,
                l(sgn * hw, h * 0.5, depth),
                tangent,
                VY,
                inward,
                v(0.12, h * 0.5, 0.12),
                tint(wood, shade),
            );
            lathe(
                m,
                l(sgn * hw, 0.0, depth),
                VY,
                &[(0.0, 0.16), (0.08, 0.15), (0.1, 0.0)],
                6,
                tint(ROCK, 0.8),
                true,
            );
        }
        kit_block(
            m,
            l(0.0, h + 0.1, depth),
            tangent,
            VY,
            inward,
            v(hw + 0.28, 0.12, 0.14),
            tint(wood, 1.1),
        );
        for sgn in [-1.0, 1.0] {
            bar(
                m,
                l(sgn * hw, h - 0.55, depth),
                l(sgn * (hw - 0.5), h, depth),
                0.07,
                0.07,
                inward,
                tint(wood, 0.95),
            );
        }
    }
    for sgn in [-1.0, 1.0] {
        let mut y = 0.35;
        while y < h {
            let shade = rng.range(0.8, 1.15);
            kit_block(
                m,
                l(sgn * (hw + 0.02), y, 0.33),
                inward,
                VY,
                tangent,
                v(0.2, 0.09, 0.03),
                tint(OLD_TIMBER, shade),
            );
            y += 0.42;
        }
    }
    let mut x = -hw;
    while x <= hw {
        let shade = rng.range(0.8, 1.2);
        kit_block(
            m,
            l(x, h + 0.26, 0.33),
            tangent,
            VY,
            inward,
            v(0.14, 0.03, 0.22),
            tint(OLD_TIMBER, shade),
        );
        x += 0.3;
    }
    // Lantern on a hook from the cap, a warning board with a skull.
    let hook = l(0.0, h - 0.02, 0.62);
    bar(m, hook, hook - Y(0.18), 0.01, 0.01, VX, IRON);
    lantern(m, glow, hook - Y(0.2), rng);
    let board = l(hw * 0.55, h - 0.5, 0.67);
    kit_block(
        m,
        board,
        tangent,
        VY,
        inward,
        v(0.26, 0.17, 0.02),
        tint(WOOD, 1.2),
    );
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            board + tangent * (sgn * 0.2) + Y(0.17),
            board + tangent * (sgn * 0.2) + Y(0.46),
            0.008,
            0.008,
            inward,
            IRON,
        );
    }
    let face = board + inward * 0.022;
    disc(m, face + Y(0.03), inward, 0.07, 10, [0.86, 0.8, 0.7]);
    for sgn in [-1.0, 1.0] {
        disc(
            m,
            face + Y(0.04) + tangent * (sgn * 0.028) + inward * 0.002,
            inward,
            0.018,
            6,
            [0.08, 0.05, 0.04],
        );
    }
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            face + tangent * (sgn * 0.13) - Y(0.12),
            face - tangent * (sgn * 0.13) + Y(0.1),
            0.02,
            0.004,
            inward,
            [0.62, 0.08, 0.06],
        );
    }
}

/// A tin miner's lantern: cage, glass glowing, carry ring.
pub fn lantern(m: &mut Mesh3D, glow: &mut Mesh3D, c: Vec3, rng: &mut ArtRng) {
    lathe(
        m,
        c + Y(0.1),
        VY,
        &[(0.0, 0.07), (0.04, 0.05), (0.07, 0.015)],
        8,
        tint(IRON, 1.4),
        true,
    );
    lathe(
        m,
        c - Y(0.12),
        VY,
        &[(0.0, 0.06), (0.03, 0.075), (0.04, 0.075)],
        8,
        tint(IRON, 1.3),
        true,
    );
    for k in 0..4 {
        let a = k as f32 * TAU / 4.0 + 0.4;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            c + d * 0.062 - Y(0.09),
            c + d * 0.062 + Y(0.1),
            0.007,
            0.007,
            d,
            IRON,
        );
    }
    tube_ring(m, c + Y(0.2), VX, 0.035, 0.006, 10, tint(IRON, 1.5));
    lathe(
        glow,
        c - Y(0.08),
        VY,
        &[(0.0, 0.045), (0.08, 0.05), (0.16, 0.03)],
        8,
        [0.9, 0.55, 0.2],
        true,
    );
    // The candle inside glows whole (web: waxCandle(glow, glow, ...)).
    let (mut body, mut flame) = (empty(), empty());
    wax_candle(
        &mut body,
        &mut flame,
        c - Y(0.06),
        0.018,
        0.05,
        rng,
        [1.0, 0.8, 0.45],
        true,
    );
    append(glow, &body, &identity(), [1.0; 3]);
    append(glow, &flame, &identity(), [1.0; 3]);
}

/// An ore outcrop breaking out of a wall bay, laced with glowing veins and
/// bristling with ore crystals.
pub fn ore_outcrop(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    ore: C3,
    rng: &mut ArtRng,
) {
    let w = width * 0.5;
    let hh = 2.3;
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    let reach = |x: f32, y: f32| (1.0 - (x / w).powi(2) - ((y - 0.2) / hh).powi(2)).max(0.0);
    let mut lumps: Vec<(f32, f32, f32, f32)> = vec![];
    for _ in 0..46 {
        let x = rng.range(-w, w);
        let y = rng.range(0.0, hh);
        let r = reach(x, y);
        if r <= 0.05 {
            continue;
        }
        let size = 0.16 + 0.3 * r * rng.range(0.7, 1.1);
        let depth = 0.04 + r * 0.4;
        lumps.push((x, y, depth, size));
        let sy = rng.range(0.75, 1.05);
        let shade = rng.range(0.5, 0.85);
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            l(x, y - size * 0.35, depth * 0.6),
            v(size, size * sy, size * 0.8),
            tint(ROCK, shade),
            phase,
        );
    }
    // Veins run over the faces of the rock, glowing through the cracks.
    for &(x, y, depth, size) in &lumps {
        if !rng.chance(0.55) {
            continue;
        }
        let face = l(x, y - size * 0.1, depth * 0.6 + size * 0.72);
        let mut p = face + tangent * (-size * 0.5) + Y(rng.range(-0.1, 0.1));
        for _ in 0..4 {
            let q = p + tangent * (size * 0.28) + Y(rng.range(-0.1, 0.12));
            let n = Y(0.012 + rng.range(0.0, 0.01));
            let shade = rng.range(0.16, 0.26);
            quad(
                glow,
                [p - n, q - n * 0.6, q + n * 0.6, p + n],
                inward,
                tint(ore, shade),
            );
            p = q;
        }
    }
    if lumps.is_empty() {
        return;
    }
    for _ in 0..9 {
        let (x, y, depth, size) = lumps[rng.index(lumps.len())];
        let base = l(x, y, depth * 0.6 + size * 0.5);
        let up = rng.range(0.0, 0.7);
        let side = rng.range(-0.4, 0.4);
        let dir = norm(inward + Y(up) + tangent * side);
        for _ in 0..3 {
            let (a, b, c) = (
                rng.range(-0.3, 0.3),
                rng.range(-0.2, 0.3),
                rng.range(-0.3, 0.3),
            );
            let lean = norm(dir + v(a, b, c));
            let len = rng.range(0.1, 0.24);
            let shade = rng.range(0.24, 0.36);
            lathe(
                glow,
                base + lean * 0.01,
                lean,
                &[(0.0, 0.034), (len * 0.7, 0.028), (len, 0.0)],
                5,
                tint(ore, shade),
                true,
            );
        }
    }
}

/// A heap of spoil with glinting nuggets.
pub fn ore_heap(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    r: f32,
    ore: C3,
    rng: &mut ArtRng,
) {
    for _ in 0..(r * 30.0).round() as usize {
        let a = rng.range(0.0, TAU);
        let d = r * rng.unit64().sqrt() as f32;
        let h = (1.0 - d / r) * r * 0.55;
        let size = rng.range(0.06, 0.14);
        let lift = rng.range(0.4, 1.0);
        let shade = rng.range(0.6, 1.1);
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            center + v(a.cos() * d, h * lift, a.sin() * d),
            v(size, size * 0.8, size),
            tint(ROCK, shade),
            phase,
        );
        if rng.chance(0.18) {
            let hgt = rng.range(0.06, 0.12);
            let shade = rng.range(0.45, 0.62);
            crystal(
                glow,
                center + v(a.cos() * d, h + 0.04, a.sin() * d),
                0.03,
                hgt,
                tint(ore, shade),
                5,
            );
        }
    }
}

/// Pickaxe or shovel from `base` along `dir`.
pub fn tool(m: &mut Mesh3D, base: Vec3, dir: Vec3, pick: bool, rng: &mut ArtRng) {
    let d = norm(dir);
    let side = if d.y.abs() < 0.9 {
        norm(VY.cross(d))
    } else {
        VX
    };
    let tip = base + d * 0.95;
    bar(
        m,
        base,
        tip,
        0.022,
        0.022,
        side,
        tint(WOOD, rng.range(0.9, 1.2)),
    );
    if pick {
        let across = side;
        let bend = d * -0.05;
        tube(
            m,
            &[
                tip + across * -0.3 + bend,
                tip + across * -0.12,
                tip,
                tip + across * 0.12,
                tip + across * 0.3 + bend,
            ],
            &|t| 0.03 * (1.0 - (t - 0.5).abs() * 1.5) + 0.006,
            5,
            &|_, _| tint(IRON, 1.5),
        );
    } else {
        let n = norm(d.cross(side));
        let blade = tip + d * 0.2;
        let q = [
            tip + side * 0.12,
            tip - side * 0.12,
            blade - side * 0.11,
            blade + side * 0.11,
        ];
        quad(m, q, n, tint(IRON, 1.6));
        quad(m, q, -n, tint(IRON, 1.2));
        kit_block(
            m,
            base - d * 0.04,
            side,
            d,
            n,
            v(0.07, 0.02, 0.02),
            tint(WOOD, 1.1),
        );
    }
}

/// Wooden crate of red dynamite sticks with a coiled fuse and a warning.
pub fn dynamite_crate(m: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    for (x, z, w, d) in [
        (0.0, 0.3, 0.38, 0.03),
        (0.0, -0.3, 0.38, 0.03),
        (0.36, 0.0, 0.03, 0.3),
        (-0.36, 0.0, 0.03, 0.3),
    ] {
        kit_block(m, l(x, 0.17, z), s, VY, f, v(w, 0.17, d), tint(WOOD, 1.15));
    }
    kit_block(
        m,
        l(0.0, 0.02, 0.0),
        s,
        VY,
        f,
        v(0.37, 0.02, 0.3),
        tint(WOOD, 0.8),
    );
    for row in 0..3 {
        for k in 0..9 {
            let p = l(
                -0.3 + k as f32 * 0.075,
                0.26 + row as f32 * 0.02,
                -0.15 + row as f32 * 0.14,
            );
            lathe(
                m,
                p - f * 0.12,
                f,
                &[(0.0, 0.03), (0.24, 0.03)],
                7,
                tint(DYNAMITE, rng.range(0.85, 1.1)),
                true,
            );
            if row == 2 && k % 3 == 0 {
                bar(
                    m,
                    p + f * 0.1 + Y(0.02),
                    p + f * 0.11 + Y(0.13),
                    0.004,
                    0.004,
                    s,
                    [0.1, 0.08, 0.06],
                );
            }
        }
    }
    let fuse = l(0.45, 0.02, 0.1);
    for k in 0..4 {
        tube_ring(
            m,
            fuse + Y(k as f32 * 0.012),
            VY,
            0.1 - k as f32 * 0.012,
            0.007,
            14,
            [0.16, 0.12, 0.08],
        );
    }
    let sign = l(0.0, 0.2, 0.332);
    quad(
        m,
        [
            sign + s * -0.14 - Y(0.08),
            sign + s * 0.14 - Y(0.08),
            sign + s * 0.14 + Y(0.08),
            sign + s * -0.14 + Y(0.08),
        ],
        f,
        [0.8, 0.7, 0.3],
    );
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            sign + s * (sgn * 0.09) - Y(0.06) + f * 0.002,
            sign - s * (sgn * 0.09) + Y(0.06) + f * 0.002,
            0.014,
            0.003,
            f,
            DYNAMITE,
        );
    }
}

/// A lumpy sack, tied at the neck.
pub fn sack(m: &mut Mesh3D, base: Vec3, h: f32, color: C3, rng: &mut ArtRng) {
    let (a, c) = (rng.range(-0.15, 0.15), rng.range(-0.15, 0.15));
    let lean = norm(v(a, 1.0, c));
    let shades = [0.7, 0.9, 1.0, 0.95, 0.8, 1.0, 0.9];
    lathe_fn(
        m,
        base,
        lean,
        &[
            (0.0, 0.16 * h / 0.6),
            (0.05 * h, 0.26 * h / 0.6),
            (0.4 * h, 0.27 * h / 0.6),
            (0.72 * h, 0.2 * h / 0.6),
            (0.84 * h, 0.07),
            (0.9 * h, 0.08),
            (h, 0.04),
        ],
        9,
        &|j| tint(color, shades[j.min(6)]),
        true,
    );
    tube_ring(m, base + lean * (0.85 * h), lean, 0.075, 0.012, 10, ROPE);
}

pub fn rope_coil(m: &mut Mesh3D, base: Vec3, radius: f32) {
    for k in 0..5 {
        tube_ring(
            m,
            base + Y(0.025 + k as f32 * 0.035),
            VY,
            radius - (k % 2) as f32 * 0.012,
            0.022,
            20,
            tint(ROPE, 0.85 + (k % 2) as f32 * 0.15),
        );
    }
}

/// Wooden bucket with iron hoops, open: ore heaped in it, or water.
pub fn bucket(m: &mut Mesh3D, glow: &mut Mesh3D, b: Vec3, ore: bool, ctx: &mut Ctx) {
    // Staves flaring to an open rim: outside, inside, the rim, the floor.
    lathe(
        m,
        b,
        VY,
        &[(0.0, 0.13), (0.3, 0.16)],
        10,
        tint(WOOD, 1.0),
        false,
    );
    lathe(
        m,
        b,
        VY,
        &[(0.3, 0.146), (0.03, 0.118)],
        10,
        tint(WOOD, 0.62),
        false,
    );
    annulus(m, b + Y(0.3), VY, 0.146, 0.16, 15, tint(WOOD, 1.12), 0.0);
    disc(m, b, -VY, 0.13, 15, tint(WOOD, 0.5));
    disc(m, b + Y(0.03), VY, 0.118, 15, tint(WOOD, 0.45));
    for y in [0.05f32, 0.25] {
        tube_ring(m, b + Y(y), VY, 0.14 + y * 0.1, 0.01, 14, tint(IRON, 1.3));
    }
    tube_c(
        m,
        &bezier(
            b + v(-0.16, 0.3, 0.0),
            b + v(0.0, 0.5, 0.0),
            b + v(0.16, 0.3, 0.0),
            8,
        ),
        0.007,
        4,
        tint(IRON, 1.4),
    );
    let inner = |y: f32| 0.118 + (0.146 - 0.118) * ((y - 0.03) / 0.27);
    if ore {
        let r = inner(0.21);
        let rim: Vec<Vec3> = (0..10)
            .map(|k| {
                b + v(
                    ((k as f32 / 10.0) * TAU).cos() * r,
                    0.21,
                    ((k as f32 / 10.0) * TAU).sin() * r,
                )
            })
            .collect();
        heap_load(
            m,
            glow,
            &rim,
            &Load {
                crown: 0.07,
                rocks: 4,
                size: [0.035, 0.055],
                bed: tint(ROCK, 0.5),
                stone: ROCK,
                ore: None,
                clear: 0.015,
            },
            &mut ctx.rng,
        );
    } else {
        let r = inner(0.2) - 0.002;
        pond(
            &mut ctx.wet,
            &ellipse(b + Y(0.2), r, r, 16),
            b.y + 0.2,
            [0.04, 0.05, 0.055],
            CAVE_REFLECTION,
            0.6,
            1.0,
            None,
        );
    }
}

/// Wheelbarrow with a load of ore.
pub fn wheelbarrow(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    ore: Option<C3>,
    rng: &mut ArtRng,
) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    let tray = [
        l(-0.28, 0.55, -0.35),
        l(0.28, 0.55, -0.35),
        l(0.36, 0.62, 0.35),
        l(-0.36, 0.62, 0.35),
    ];
    let floor = [
        l(-0.2, 0.3, -0.25),
        l(0.2, 0.3, -0.25),
        l(0.24, 0.32, 0.25),
        l(-0.24, 0.32, 0.25),
    ];
    for k in 0..4 {
        let q = [floor[k], floor[(k + 1) % 4], tray[(k + 1) % 4], tray[k]];
        let n = norm((q[0] + q[1]) * 0.5 - l(0.0, 0.45, 0.0));
        quad(m, q, n, tint(RUST, 0.9 + k as f32 * 0.05));
        quad(m, q, -n, tint(RUST, 0.55));
    }
    quad(m, floor, VY, tint(RUST, 0.5));
    tube_ring(m, l(0.0, 0.2, 0.62), s, 0.18, 0.035, 14, tint(IRON, 1.3));
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            l(sgn * 0.2, 0.3, 0.6),
            l(sgn * 0.3, 0.52, -0.9),
            0.025,
            0.025,
            VY,
            tint(WOOD, 1.1),
        );
        bar(
            m,
            l(sgn * 0.2, 0.3, -0.2),
            l(sgn * 0.22, 0.0, -0.3),
            0.02,
            0.02,
            f,
            tint(WOOD, 0.9),
        );
    }
    // The load: heaped between the tray's sides, rocks bedded in it.
    let rim: Vec<Vec3> = (0..4).map(|k| floor[k].lerp(tray[k], 0.55)).collect();
    heap_load(
        m,
        glow,
        &rim,
        &Load {
            crown: 0.09,
            rocks: 7,
            size: [0.04, 0.075],
            bed: tint(ROCK, 0.5),
            stone: ROCK,
            ore,
            clear: 0.015,
        },
        rng,
    );
}

/// Timber set braced against a pilaster: posts, cap, knee braces, wedges.
pub fn timber_set(
    m: &mut Mesh3D,
    base: Vec3,
    inward: Vec3,
    tangent: Vec3,
    height: f32,
    span: f32,
    rng: &mut ArtRng,
) {
    let l = |x: f32, y: f32, z: f32| base + tangent * x + Y(y) + inward * z;
    for sgn in [-1.0, 1.0] {
        kit_block(
            m,
            l(sgn * span, height * 0.5, 0.36),
            tangent,
            VY,
            inward,
            v(0.11, height * 0.5, 0.11),
            tint(TIMBER, rng.range(0.85, 1.1)),
        );
        bar(
            m,
            l(sgn * span, height - 0.62, 0.36),
            l(sgn * (span - 0.55), height - 0.06, 0.36),
            0.065,
            0.065,
            inward,
            tint(TIMBER, 0.95),
        );
        kit_block(
            m,
            l(sgn * span, 0.05, 0.36),
            tangent,
            VY,
            inward,
            v(0.15, 0.05, 0.15),
            tint(ROCK, 0.7),
        );
        for y in [0.9, 1.7] {
            kit_block(
                m,
                l(sgn * span, y, 0.48),
                tangent,
                VY,
                inward,
                v(0.012, 0.012, 0.012),
                tint(IRON, 1.6),
            );
        }
    }
    kit_block(
        m,
        l(0.0, height + 0.1, 0.36),
        tangent,
        VY,
        inward,
        v(span + 0.22, 0.12, 0.13),
        tint(TIMBER, 1.1),
    );
    for x in [-span * 0.5, span * 0.5] {
        kit_block(
            m,
            l(x, height + 0.26, 0.36),
            tangent,
            VY,
            inward,
            v(0.08, 0.05, 0.1),
            tint(OLD_TIMBER, 1.2),
        );
    }
}

/// Heavy beam across the vault with iron brackets.
pub fn ceiling_beam(m: &mut Mesh3D, a: Vec3, b: Vec3, rng: &mut ArtRng) {
    let d = norm(hz(b - a));
    bar(m, a, b, 0.13, 0.14, VY, tint(TIMBER, rng.range(0.85, 1.05)));
    for p in [a + d * 0.25, b - d * 0.25] {
        bar(m, p - Y(0.16), p + Y(0.16), 0.02, 0.15, d, tint(IRON, 1.2));
    }
}

fn ore_chips(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    ore: C3,
    rng: &mut ArtRng,
) {
    for _ in 0..6 {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.unit64().sqrt() as f32;
        let q = center + v(a.cos() * d, 0.01, a.sin() * d);
        rock(
            m,
            q,
            v(0.04, 0.03, 0.035),
            tint(ROCK, rng.range(0.7, 1.1)),
            a,
        );
        if rng.chance(0.5) {
            crystal(glow, q + Y(0.02), 0.018, 0.05, tint(ore, 0.55), 5);
        }
    }
}

// ---------------------------------------------------------------- planner

/// Plan the mine into the plan held by the stage.
pub fn plan(s: &mut Stage) {
    let ore: C3 = [1.0, 0.72, 0.28];
    let lamp: C3 = [1.0, 0.66, 0.32];
    // A. Tunnel mouth: the far wall or the first wall with a free stretch.
    let mut portal: Option<(Vec3, Edge)> = None;
    let mut walls = s.walls.clone();
    walls.sort_by_key(|w| mm(w.inward.dot(s.inward)));
    'walls: for wall in &walls {
        for shift in [0.0f32, 1.6, -1.6, 2.6, -2.6] {
            let center = wall.mid + wall.tangent * shift;
            if shift.abs() + 1.4 > wall.length * 0.5 - 0.3 {
                continue;
            }
            if wall
                .doors
                .iter()
                .any(|d| (d.p - center).dot(wall.tangent).abs() < d.width * 0.5 + 1.6)
            {
                continue;
            }
            let posts: Vec<Vec3> = [-1.0, 1.0]
                .iter()
                .map(|&sgn| s.floor(center + wall.tangent * (sgn * 1.05) + wall.inward * 0.32))
                .collect();
            if posts.iter().any(|&p| s.crowded(p, 0.3, 0.05)) {
                continue;
            }
            portal = Some((s.floor(center), wall.clone()));
            for &p in &posts {
                s.solid(p, 0.3, true);
            }
            let near: Vec<Slot> = s
                .bays
                .iter()
                .filter(|bay| hdist(bay.center, center) < bay.width * 0.5 + 1.3)
                .cloned()
                .collect();
            for bay in &near {
                s.take_bay(bay);
            }
            break 'walls;
        }
    }
    if let Some((center, wall)) = &portal {
        let h = 2.5f32.min(s.spring - 0.5);
        let (c, inw, tan) = (*center, wall.inward, wall.tangent);
        s.plan.wall_pieces.push(c);
        s.piece(move |m, g, ctx| mine_portal(m, g, c, inw, tan, 1.0, h, &mut ctx.rng));
        s.light(c + inw * 0.9 + Y(h - 0.3), lamp, 2.4, 6.0);
    }
    // B. The main line runs from the entrance into the tunnel (or to a stop).
    let start = s.floor(s.entry + s.inward * 1.3 + s.right * (s.width * 0.28));
    let end = match &portal {
        Some((c, wall)) => *c + wall.inward * 0.2,
        None => s.floor(s.at(s.depth - 1.0, s.width * 0.28)),
    };
    let ctrl = start.lerp(end, 0.5) + s.right * (s.width * 0.12);
    let line: Vec<Vec3> = bezier(start, ctrl, end, 18)
        .into_iter()
        .map(|p| s.floor(p))
        .collect();
    {
        let pts = line.clone();
        s.piece(move |m, _g, ctx| rail_path(m, &pts, &mut ctx.rng));
    }
    let n = line.len();
    if portal.is_none() {
        if let Some(stop) = s.place(&[line[n - 1], line[n - 3]], 0.5, 1.0, 0.8) {
            s.plan.solids.push((stop, 0.5));
            let yaw = yaw_of(line[n - 1] - line[n - 2]);
            s.piece(move |m, _g, _ctx| buffer_stop(m, stop, yaw));
        }
    }
    // C. A siding curves off to a side wall and ends at a stop by a toppled cart.
    let fork = line[7];
    let along = (fork - s.c).dot(s.inward);
    let mut side_end = None;
    let mut side_sign = 1.0;
    for (df, dr, sgn) in [
        (1.2, 1.4, 1.0),
        (1.2, 1.4, -1.0),
        (0.3, 1.7, 1.0),
        (0.3, 1.7, -1.0),
        (2.1, 2.1, 1.0),
        (2.1, 2.1, -1.0f32),
    ] {
        let p = s.floor(s.at(along + df, sgn * (s.width - dr)));
        if !s.inside(p, 0.7) || !s.placer.clear(p, 0.5, 1.0, 0.8) {
            continue;
        }
        side_end = s.place(&[p], 0.5, 1.0, 0.8);
        side_sign = sgn;
        break;
    }
    if let Some(side_end) = side_end {
        let siding: Vec<Vec3> = bezier(
            fork,
            fork + (line[9] - line[7]) + s.right * (side_sign * 0.8),
            side_end,
            12,
        )
        .into_iter()
        .map(|p| s.floor(p))
        .collect();
        let dir = norm(hz(side_end - siding[siding.len() - 2]));
        {
            let pts = siding.clone();
            s.piece(move |m, _g, ctx| rail_path(m, &pts, &mut ctx.rng));
        }
        s.plan.solids.push((side_end, 0.5));
        let yaw = yaw_of(dir);
        s.piece(move |m, _g, _ctx| buffer_stop(m, side_end, yaw));
        let across = v(-dir.z, 0.0, dir.x);
        if let Some(tipped) = s.place(
            &[
                side_end - dir * 1.6 + across * 1.0,
                side_end - dir * 2.2 + across * -1.1,
            ],
            0.72,
            1.2,
            0.8,
        ) {
            s.plan.solids.push((tipped, 0.72));
            let yaw = yaw_of(dir) + 0.6;
            s.piece(move |m, g, ctx| mine_cart(m, g, tipped, yaw, 1.35, 0.52, ore, &mut ctx.rng));
            let heap = tipped + across * 0.9;
            s.piece(move |m, g, ctx| ore_heap(m, g, heap, 0.7, ore, &mut ctx.rng));
        }
    }
    // D. A loaded cart waits on the main line.
    for k in [11usize, 5, 14, 8] {
        let Some(p) = s.place(&[line[k]], 0.75, 1.3, 0.8) else {
            continue;
        };
        s.plan.solids.push((p, 0.75));
        let yaw = yaw_of(line[k + 1] - line[k - 1]);
        s.piece(move |m, g, ctx| mine_cart(m, g, p, yaw, 0.0, 0.0, ore, &mut ctx.rng));
        break;
    }
    // E. Timber sets on the pilasters, beams across the vault, lanterns.
    let set_height = 3.0f32.min(s.spring - 0.35);
    for pilaster in s.pilasters.clone() {
        if s.doorway(pilaster.base) < 1.2 {
            continue;
        }
        let (b, inw, tan) = (pilaster.base, pilaster.inward, pilaster.tangent);
        s.plan.wall_pieces.push(b);
        s.piece(move |m, _g, ctx| timber_set(m, b, inw, tan, set_height, 0.55, &mut ctx.rng));
        for sgn in [-1.0, 1.0] {
            let p = s.floor(b + tan * (sgn * 0.55) + inw * 0.36);
            s.solid(p, 0.16, true);
        }
    }
    for (a, b) in s.ribs.clone() {
        let ya = with_y(a, s.c.y + set_height + 0.35);
        let yb = with_y(b, s.c.y + set_height + 0.35);
        s.piece(move |m, _g, ctx| ceiling_beam(m, ya, yb, &mut ctx.rng));
        for t in [0.33, 0.67] {
            let p = ya.lerp(yb, t);
            let top = p - Y(0.14);
            s.piece(move |m, g, ctx| {
                let c = chain(m, top, 0.55, &mut ctx.rng, IRON);
                lantern(m, g, c - Y(0.22), &mut ctx.rng);
            });
            s.light(p - Y(0.9), lamp, 2.2, 6.0);
        }
    }
    // F. Ore breaks out of the walls, spoil heaped below with the tools.
    let mut veins = 0;
    for bay in s.free_bays(None, 1.8) {
        if veins >= 2 {
            break;
        }
        let foot = s.floor(bay.center + bay.inward * 0.9);
        if s.crowded(foot, 0.7, 0.1) || s.walkway(foot) < 1.4 {
            continue;
        }
        s.take_bay(&bay);
        let (c, inw, tan) = (s.floor(bay.center), bay.inward, bay.tangent);
        let width = 2.4f32.min(bay.width - 0.4);
        s.plan.wall_pieces.push(c);
        s.piece(move |m, g, ctx| ore_outcrop(m, g, c, inw, tan, width, ore, &mut ctx.rng));
        s.solid(foot, 0.62, false);
        s.piece(move |m, g, ctx| ore_heap(m, g, foot, 0.72, ore, &mut ctx.rng));
        let pick_base = foot + tan * 0.2 + Y(0.28);
        let pick_dir = inw * 0.35 + Y(1.0) + tan * 0.2;
        s.piece(move |m, _g, ctx| tool(m, pick_base, pick_dir, true, &mut ctx.rng));
        if let Some(shovel) = s.spot(foot + tan * 0.95, 0.2, 0.3) {
            let base = shovel + inw * -0.4 + Y(0.05);
            let dir = -inw * 0.25 + Y(1.0);
            s.piece(move |m, _g, ctx| tool(m, base, dir, false, &mut ctx.rng));
        }
        if let Some(pail) = s.spot(foot - tan * 0.95 + inw * 0.2, 0.18, 0.3) {
            s.piece(move |m, g, ctx| bucket(m, g, pail, true, ctx));
        }
        s.light(bay.center + bay.inward * 0.8 + Y(1.4), ore, 2.0, 4.6);
        veins += 1;
    }
    // G. A work corner: bench, stool, tools, dynamite, sacks, rope, barrels.
    let mut corners = s.corners.clone();
    s.rng.shuffle(&mut corners);
    let work = corners[0].clone();
    let cands: Vec<Vec3> = [1.5, 1.9]
        .iter()
        .map(|&d| work.mid + work.inward * d)
        .collect();
    if let Some(bench) = s.place(&cands, 0.8, 1.2, 0.7) {
        let yaw = yaw_of(-work.inward);
        s.plan.solids.push((bench, 0.8));
        let table = if s.rng.chance(0.5) {
            "table_long_broken"
        } else {
            "table_medium"
        };
        s.prop(table, bench, yaw, 0.85, 0.0);
        let lamp_base = bench + Y(0.86) + work.tangent * 0.35;
        s.piece(move |m, g, ctx| lantern(m, g, lamp_base + Y(0.14), &mut ctx.rng));
        s.light(bench + Y(1.3), lamp, 1.8, 4.5);
        if let Some(stool) = s.spot(bench + work.inward * 0.95, 0.22, 0.25) {
            let yaw = s.rng.range(0.0, TAU);
            s.prop("stool", stool, yaw, 0.45, 0.0);
        }
        s.prop(
            "bottle_a_brown",
            bench + Y(0.86) - work.tangent * 0.3,
            0.0,
            0.26,
            0.0,
        );
        s.prop(
            "plate_food",
            bench + Y(0.86) - work.tangent * 0.05 + work.inward * 0.1,
            0.0,
            0.08,
            0.0,
        );
    }
    let cands: Vec<Vec3> = [1.2, 1.5]
        .iter()
        .map(|&d| corners[1].mid + corners[1].inward * d)
        .collect();
    if let Some(crate_at) = s.place(&cands, 0.45, 1.2, 0.6) {
        s.plan.solids.push((crate_at, 0.45));
        let yaw = yaw_of(-corners[1].inward) + s.rng.range(-0.3, 0.3);
        s.piece(move |m, _g, ctx| dynamite_crate(m, crate_at, yaw, &mut ctx.rng));
    }
    for corner in corners.iter().skip(1) {
        let take = 2 + s.rng.index(2);
        for &(asset, h, r) in [
            ("barrel_large", 0.95, 0.42),
            ("box_stacked", 0.9, 0.45),
            ("barrel_stack", 0.7, 0.4),
            ("crates_stacked", 1.1, 0.55),
        ]
        .iter()
        .take(take)
        {
            let a = corner.mid + corner.inward * 1.2 + corner.tangent * s.rng.range(-1.0, 1.0);
            let b = corner.mid + corner.inward * 1.6 + corner.tangent * s.rng.range(-1.2, 1.2);
            if let Some(p) = s.place(&[a, b], r, 1.2, 0.6) {
                let yaw = s.rng.range(0.0, TAU);
                s.prop(asset, p, yaw, h, r);
            }
        }
    }
    let ring = |a: f32, walk: f32| Scatter {
        walkway: walk,
        ring: [a, 1.0],
        ..Scatter::default()
    };
    s.scatter(7, 0.3, ring(0.55, 0.8), &mut |s, p, _| {
        let h = s.rng.range(0.5, 0.7);
        s.piece(move |m, _g, ctx| sack(m, p, h, SACK, &mut ctx.rng));
    });
    s.scatter(3, 0.25, ring(0.55, 0.6), &mut |s, p, _| {
        let r = s.rng.range(0.18, 0.24);
        s.piece(move |m, _g, _ctx| rope_coil(m, p, r));
    });
    s.scatter(3, 0.2, ring(0.5, 0.6), &mut |s, p, _| {
        let with_ore = s.rng.chance(0.5);
        s.piece(move |m, g, ctx| bucket(m, g, p, with_ore, ctx));
    });
    let cands = [
        s.at(-s.depth * 0.25, -s.width * 0.55),
        s.at(s.depth * 0.3, -s.width * 0.55),
        s.at(0.0, s.width * 0.6),
    ];
    if let Some(barrow) = s.place(&cands, 0.55, 1.4, 0.7) {
        s.plan.solids.push((barrow, 0.55));
        let yaw = s.rng.range(0.0, TAU);
        s.piece(move |m, g, ctx| wheelbarrow(m, g, barrow, yaw, Some(ore), &mut ctx.rng));
    }
    // H. Floor: gravel, ore chips, planks, puddles, lanterns left on the ground.
    let floor_only = Scatter {
        walkway: 0.0,
        ..Scatter::default()
    };
    s.scatter(14, 0.5, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.3, 0.6);
        let count = s.rng.index(10) + 8;
        s.pebbles(p, r, count, Some(tint(ROCK, 1.1)));
    });
    s.scatter(10, 0.5, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.3, 0.7);
        s.dust(p, r, Some([0.2, 0.17, 0.15]));
    });
    s.scatter(5, 0.4, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.25, 0.5);
        s.puddle(p, r, Some([0.07, 0.065, 0.06]), None);
    });
    s.scatter(4, 0.2, ring(0.5, 0.7), &mut |s, p, _| {
        s.piece(move |m, g, ctx| lantern(m, g, p + Y(0.14), &mut ctx.rng));
    });
    s.scatter(6, 0.1, floor_only, &mut |s, p, _| {
        s.piece(move |m, g, ctx| ore_chips(m, g, p, 0.3, ore, &mut ctx.rng));
    });
    s.plan.sparkle_tint = Some([1.5, 1.1, 0.55]);
    let h = s.height;
    s.sparkles(20, 0.4, h - 1.0, None, None);
}
