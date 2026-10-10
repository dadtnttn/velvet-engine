//! Theme kit (web prototype `art/themekit.js`): the pieces that give each
//! chamber its personality. Every piece writes into a vertex-coloured `m`
//! (lit) and `glow` (emissive).
use super::kit::*;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

pub const BONE: C3 = [0.84, 0.78, 0.66];

/// Skull: cranium, sockets, cheekbones and jaw.
pub fn skull(m: &mut Mesh3D, p: Vec3, s: f32, yaw: f32, tilt: f32, color: C3) {
    let mut head = empty();
    lathe(
        &mut head,
        Vec3::ZERO,
        VZ,
        &[
            (-0.1, 0.0),
            (-0.09, 0.062),
            (-0.03, 0.1),
            (0.04, 0.096),
            (0.09, 0.07),
            (0.11, 0.03),
            (0.115, 0.0),
        ],
        10,
        color,
        false,
    );
    for sx in [-1.0, 1.0] {
        disc(
            &mut head,
            v(sx * 0.036, 0.012, 0.104),
            VZ,
            0.026,
            8,
            [0.06, 0.04, 0.04],
        );
    }
    disc(
        &mut head,
        v(0.0, -0.032, 0.108),
        VZ,
        0.012,
        5,
        [0.08, 0.06, 0.05],
    );
    kit_block(
        &mut head,
        v(0.0, -0.078, 0.066),
        VX,
        VY,
        VZ,
        v(0.046, 0.022, 0.036),
        tint(color, 0.88),
    );
    for k in -2..=2 {
        kit_block(
            &mut head,
            v(k as f32 * 0.016, -0.06, 0.1),
            VX,
            VY,
            VZ,
            v(0.006, 0.01, 0.004),
            tint(color, 1.08),
        );
    }
    append(m, &head, &pose_mapper(p, yaw, tilt, 0.0, s), [1.0; 3]);
}

const WOOD: C3 = [0.34, 0.2, 0.11];
const INGOT: C3 = [1.0, 0.72, 0.2];

fn frame_of(yaw: f32) -> (Vec3, Vec3) {
    let f = yaw_dir(yaw);
    (f, v(f.z, 0.0, -f.x))
}

/// A rack of swords and spears.
pub fn weapon_rack(m: &mut Mesh3D, base: Vec3, yaw: f32) {
    let (f, s) = frame_of(yaw);
    for sx in [-0.62, 0.62] {
        kit_block(
            m,
            base + s * sx + Y(0.8),
            s,
            VY,
            f,
            v(0.05, 0.8, 0.07),
            tint(WOOD, 0.9),
        );
    }
    for y in [0.25, 1.1, 1.52] {
        kit_block(
            m,
            base + Y(y),
            s,
            VY,
            f,
            v(0.66, 0.04, 0.08),
            tint(WOOD, 1.05),
        );
    }
    for k in 0..5 {
        let x = -0.44 + k as f32 * 0.22;
        let p = base + s * x + f * 0.02;
        if k % 2 == 0 {
            // Sword: blade, guard, grip, pommel.
            bar(
                m,
                p + Y(0.3),
                p + Y(1.25),
                0.035,
                0.008,
                s,
                [0.7, 0.72, 0.76],
            );
            kit_block(
                m,
                p + Y(1.27),
                s,
                VY,
                f,
                v(0.1, 0.02, 0.025),
                [0.5, 0.36, 0.14],
            );
            bar(
                m,
                p + Y(1.29),
                p + Y(1.45),
                0.018,
                0.018,
                s,
                [0.22, 0.12, 0.08],
            );
            kit_block(
                m,
                p + Y(1.47),
                s,
                VY,
                f,
                v(0.028, 0.028, 0.028),
                [0.6, 0.44, 0.16],
            );
        } else {
            // Spear.
            bar(m, p + Y(0.22), p + Y(1.7), 0.018, 0.018, s, tint(WOOD, 1.2));
            lathe(
                m,
                p + Y(1.7),
                VY,
                &[(0.0, 0.045), (0.18, 0.0)],
                4,
                [0.68, 0.7, 0.74],
                true,
            );
        }
    }
}

/// One gold bar: a trapezoid prism, bright bevelled top.
fn ingot(m: &mut Mesh3D, c: Vec3, x: Vec3, z: Vec3, color: C3) {
    let (bw, bd, tw, td, h) = (0.13, 0.055, 0.105, 0.04, 0.055);
    let p = |sx: f32, sy: f32, sz: f32| {
        let (w, d) = if sy > 0.0 { (tw, td) } else { (bw, bd) };
        c + x * (sx * w) + Y(if sy > 0.0 { h } else { 0.0 }) + z * (sz * d)
    };
    let top = [
        p(-1.0, 1.0, -1.0),
        p(1.0, 1.0, -1.0),
        p(1.0, 1.0, 1.0),
        p(-1.0, 1.0, 1.0),
    ];
    let bottom = [
        p(-1.0, 0.0, -1.0),
        p(1.0, 0.0, -1.0),
        p(1.0, 0.0, 1.0),
        p(-1.0, 0.0, 1.0),
    ];
    quad(m, top, VY, tint(color, 1.2));
    quad(
        m,
        [bottom[0], bottom[1], top[1], top[0]],
        norm(-z + Y(0.4)),
        tint(color, 0.85),
    );
    quad(
        m,
        [bottom[2], bottom[3], top[3], top[2]],
        norm(z + Y(0.4)),
        tint(color, 1.0),
    );
    quad(
        m,
        [bottom[1], bottom[2], top[2], top[1]],
        norm(x + Y(0.4)),
        tint(color, 0.92),
    );
    quad(
        m,
        [bottom[3], bottom[0], top[0], top[3]],
        norm(-x + Y(0.4)),
        tint(color, 0.78),
    );
}

/// A pyramid of gold bars, courses laid crosswise.
pub fn ingot_stack(m: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let courses = 2 + rng.index(3);
    for level in 0..courses {
        let turn = yaw + (level % 2) as f32 * std::f32::consts::PI / 2.0;
        let x = v(turn.cos(), 0.0, -turn.sin());
        let z = v(turn.sin(), 0.0, turn.cos());
        let count = courses - level + 1;
        for k in 0..count {
            let offset = (k as f32 - (count - 1) as f32 * 0.5) * 0.12;
            let shade = rng.range(0.9, 1.08);
            ingot(
                m,
                base + z * offset + Y(level as f32 * 0.056),
                x,
                z,
                tint(INGOT, shade),
            );
        }
    }
}

// ------------------------------------------------------------ theme pieces
// (web `themekit.js` plus the few `treasury.js` / `sanctuary.js` pieces the
// non-gallery themes use: candles, fern, rubble, remains, chain, lantern.)

const IRON: C3 = [0.15, 0.13, 0.13];
const FLESH: [C3; 3] = [[0.16, 0.02, 0.03], [0.34, 0.04, 0.06], [0.52, 0.08, 0.1]];
pub const BLOOD_GLOW: C3 = [0.95, 0.12, 0.1];
const PI: f32 = std::f32::consts::PI;

/// A thick vein along `pts`: dark flesh with glowing nodes pulsing on it.
pub fn vein(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    pts: &[Vec3],
    r0: f32,
    r1: f32,
    rng: &mut ArtRng,
    nodes: bool,
) {
    tube(m, pts, &|t| r0 + (r1 - r0) * t, 7, &|t, a| {
        mix_color(
            FLESH[0],
            FLESH[2],
            0.35 + 0.35 * (a * TAU + t * 9.0).sin() * 0.5 + 0.2 * (1.0 - t),
        )
    });
    if !nodes {
        return;
    }
    let mut run = 0.0;
    for i in 1..pts.len() {
        run += (pts[i] - pts[i - 1]).length();
        if run < 0.7 {
            continue;
        }
        run = 0.0;
        let r = (r0 + (r1 - r0) * (i as f32 / (pts.len() - 1) as f32)) * 1.25;
        let shade = rng.range(0.35, 0.55);
        let phase = rng.range(0.0, TAU);
        rock(
            glow,
            pts[i] - Y(r * 0.5),
            v(r, r * 1.1, r),
            tint(BLOOD_GLOW, shade),
            phase,
        );
    }
}

/// Glowing crimson cracks branching over the floor from `center`.
pub fn floor_cracks(
    glow: &mut Mesh3D,
    center: Vec3,
    count: usize,
    inner: f32,
    outer: f32,
    rng: &mut ArtRng,
    color: C3,
) {
    for k in 0..count {
        let mut a = (k as f32 / count as f32) * TAU + rng.range(-0.2, 0.2);
        let mut p = center + v(a.cos() * inner, 0.0, a.sin() * inner);
        let mut w = 0.09;
        let steps = 6 + rng.index(4);
        let len = (outer - inner) / steps as f32;
        for s in 0..steps {
            a += rng.range(-0.35, 0.35);
            let q = p + v(a.cos() * len, 0.0, a.sin() * len);
            let d = norm(q - p);
            let n = v(-d.z, 0.0, d.x);
            let w2 = w * 0.82;
            quad(
                glow,
                [
                    p + n * w + Y(0.014),
                    q + n * w2 + Y(0.014),
                    q - n * w2 + Y(0.014),
                    p - n * w + Y(0.014),
                ],
                VY,
                tint(color, 0.5 - s as f32 * 0.03),
            );
            if rng.chance(0.3) {
                let sign = if rng.chance(0.5) { -1.0 } else { 1.0 };
                let b = a + sign * rng.range(0.6, 1.1);
                let e = q + v(b.cos() * len * 0.7, 0.0, b.sin() * len * 0.7);
                let db = norm(e - q);
                let nb = v(-db.z, 0.0, db.x);
                quad(
                    glow,
                    [
                        q + nb * (w2 * 0.6) + Y(0.013),
                        e + Y(0.013),
                        e + Y(0.013),
                        q - nb * (w2 * 0.6) + Y(0.013),
                    ],
                    VY,
                    tint(color, 0.32),
                );
            }
            p = q;
            w = w2;
        }
    }
}

/// The floor sigil under the Heart: rings, spokes and runes, flush.
pub fn heart_sigil(glow: &mut Mesh3D, m: &mut Mesh3D, center: Vec3, r: f32) {
    let y = |h: f32| center + Y(h);
    let ring = |glow: &mut Mesh3D, r0: f32, r1: f32, c: C3| {
        for i in 0..64 {
            let a0 = (i as f32 / 64.0) * TAU;
            let a1 = ((i + 1) as f32 / 64.0) * TAU;
            let p = |a: f32, rr: f32| y(0.012) + v(a.cos() * rr, 0.0, a.sin() * rr);
            quad(glow, [p(a0, r1), p(a1, r1), p(a1, r0), p(a0, r0)], VY, c);
        }
    };
    // A dark basalt disc inlaid in the floor, then the glowing lines on it.
    for i in 0..48 {
        let a0 = (i as f32 / 48.0) * TAU;
        let a1 = ((i + 1) as f32 / 48.0) * TAU;
        tri(
            m,
            y(0.006),
            y(0.006) + v(a0.cos() * r, 0.0, a0.sin() * r),
            y(0.006) + v(a1.cos() * r, 0.0, a1.sin() * r),
            VY,
            tint([0.1, 0.05, 0.06], 1.0 + 0.12 * (i as f32 * 2.1).sin()),
        );
    }
    ring(glow, r - 0.12, r, tint(BLOOD_GLOW, 0.5));
    ring(glow, r * 0.72 - 0.06, r * 0.72, tint(BLOOD_GLOW, 0.42));
    ring(glow, r * 0.3 - 0.05, r * 0.3, tint(BLOOD_GLOW, 0.55));
    // Six-pointed star between the rings.
    let pts: Vec<Vec3> = (0..6)
        .map(|k| {
            y(0.013)
                + v(
                    ((k as f32 / 6.0) * TAU).cos() * r * 0.7,
                    0.0,
                    ((k as f32 / 6.0) * TAU).sin() * r * 0.7,
                )
        })
        .collect();
    for k in 0..6 {
        let (a, b) = (pts[k], pts[(k + 2) % 6]);
        let d = norm(b - a);
        let n = v(-d.z, 0.0, d.x) * 0.035;
        quad(
            glow,
            [a + n, b + n, b - n, a - n],
            VY,
            tint(BLOOD_GLOW, 0.4),
        );
    }
    // Runes around the outer band.
    for k in 0..24 {
        let a = (k as f32 / 24.0) * TAU;
        let c = y(0.014) + v(a.cos() * r * 0.86, 0.0, a.sin() * r * 0.86);
        let t = v(-a.sin(), 0.0, a.cos());
        let o = v(a.cos(), 0.0, a.sin());
        let h = if k % 3 == 0 { 0.16 } else { 0.1 };
        quad(
            glow,
            [
                c - t * 0.025 - o * h,
                c + t * 0.025 - o * h,
                c + t * 0.025 + o * h,
                c - t * 0.025 + o * h,
            ],
            VY,
            tint(BLOOD_GLOW, 0.45),
        );
        if k % 2 == 0 {
            quad(
                glow,
                [
                    c - t * 0.09 + o * 0.04,
                    c + t * 0.09 + o * 0.04,
                    c + t * 0.09 + o * 0.08,
                    c - t * 0.09 + o * 0.08,
                ],
                VY,
                tint(BLOOD_GLOW, 0.35),
            );
        }
    }
}

/// A cover pillar strangled by veins that crawl down into the floor.
pub fn vein_pillar(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    height: f32,
    half: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let c = base + Y(height * 0.5);
    chamfer_box(
        m,
        c,
        VX,
        VY,
        VZ,
        v(half, height * 0.5, half),
        0.06,
        tint(stone, 0.55),
    );
    for y in [0.3, height - 0.35] {
        chamfer_box(
            m,
            base + Y(y),
            VX,
            VY,
            VZ,
            v(half + 0.08, 0.22, half + 0.08),
            0.05,
            tint(stone, 0.7),
        );
    }
    for k in 0..3 {
        let turns = 1.1 + k as f32 * 0.25;
        let phase = rng.range(0.0, TAU);
        let mut pts: Vec<Vec3> = (0..=36)
            .map(|i| {
                let t = i as f32 / 36.0;
                let a = phase + t * turns * TAU;
                let rr = half * 1.28 + (t * 13.0 + k as f32).sin() * 0.03;
                base + v(a.cos() * rr, height * (1.0 - t) + 0.05, a.sin() * rr)
            })
            .collect();
        // It spills over the floor at the foot of the pillar.
        let last = *pts.last().unwrap();
        let out = norm(hz(last - base));
        pts.push(with_y(last + out * 0.5, base.y + 0.06));
        pts.push(with_y(last + out * 1.1, base.y + 0.03));
        vein(m, glow, &pts, 0.09 - k as f32 * 0.012, 0.05, rng, true);
    }
}

/// A heap of bones and skulls.
pub fn bone_pile(m: &mut Mesh3D, center: Vec3, radius: f32, rng: &mut ArtRng) {
    let count = (6.0 + radius * 10.0).round() as usize;
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.unit64().sqrt() as f32;
        let lift = 0.03 + (1.0 - r / radius) * radius * 0.35 * rng.unit();
        let p = center + v(a.cos() * r, lift, a.sin() * r);
        if rng.chance(0.22) {
            let s = rng.range(0.8, 1.15);
            let yaw = rng.range(0.0, TAU);
            let tilt = rng.range(-0.5, 0.3);
            let shade = rng.range(0.85, 1.05);
            skull(m, p + Y(0.07), s, yaw, tilt, tint(BONE, shade));
        } else {
            let x = rng.range(0.0, TAU).cos();
            let y = rng.range(-0.3, 0.3);
            let z = rng.range(0.0, TAU).sin();
            let d = norm(v(x, y, z));
            let len = rng.range(0.18, 0.42);
            let (a0, a1) = (p - d * (len / 2.0), p + d * (len / 2.0));
            let shade = rng.range(0.85, 1.0);
            bar(m, a0, a1, 0.018, 0.018, VY, tint(BONE, shade));
            for q in [a0, a1] {
                kit_block(m, q, VX, VY, VZ, v(0.03, 0.026, 0.03), tint(BONE, 1.05));
            }
        }
    }
}

/// Iron tripod brazier with coals of a chosen colour.
pub fn brazier(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, fire: C3, rng: &mut ArtRng, s: f32) {
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            base + d * (0.34 * s),
            base + d * (0.18 * s) + Y(0.78 * s),
            0.03 * s,
            0.03 * s,
            VY,
            IRON,
        );
    }
    tube_ring(
        m,
        base + Y(0.42 * s),
        VY,
        0.24 * s,
        0.02 * s,
        18,
        tint(IRON, 1.3),
    );
    lathe(
        m,
        base + Y(0.74 * s),
        VY,
        &[
            (0.0, 0.12 * s),
            (0.08 * s, 0.3 * s),
            (0.2 * s, 0.4 * s),
            (0.22 * s, 0.38 * s),
        ],
        12,
        tint(IRON, 1.2),
        true,
    );
    for _ in 0..9 {
        let a = rng.range(0.0, TAU);
        let r = rng.range(0.0, 0.28 * s);
        let y = 0.9 * s + rng.range(0.0, 0.05);
        let shade = rng.range(0.5, 0.8);
        rock(
            glow,
            base + v(a.cos() * r, y, a.sin() * r),
            v(0.07 * s, 0.05 * s, 0.06 * s),
            tint(fire, shade),
            a,
        );
    }
}

/// A straw training dummy on a post, with a painted target.
pub fn dummy(m: &mut Mesh3D, base: Vec3, yaw: f32) {
    let (f, s) = frame_of(yaw);
    bar(m, base, base + Y(1.1), 0.05, 0.05, s, tint(WOOD, 0.9));
    kit_block(
        m,
        base + Y(0.04),
        s,
        VY,
        f,
        v(0.3, 0.04, 0.3),
        tint(WOOD, 0.7),
    );
    lathe(
        m,
        base + Y(0.8),
        VY,
        &[(0.0, 0.16), (0.2, 0.24), (0.5, 0.22), (0.62, 0.14)],
        9,
        [0.72, 0.6, 0.32],
        true,
    );
    lathe(
        m,
        base + Y(1.44),
        VY,
        &[(0.0, 0.1), (0.08, 0.15), (0.22, 0.14), (0.3, 0.0)],
        9,
        [0.76, 0.64, 0.34],
        true,
    );
    bar(
        m,
        base + Y(1.28) - s * 0.42,
        base + Y(1.28) + s * 0.42,
        0.04,
        0.04,
        VY,
        tint(WOOD, 1.0),
    );
    for r in [0.14f32, 0.09, 0.04] {
        disc(
            m,
            base + Y(1.1) + f * (0.235 - r * 0.02),
            f,
            r,
            12,
            if r == 0.09 {
                [0.86, 0.8, 0.7]
            } else {
                [0.6, 0.12, 0.1]
            },
        );
    }
    for y in [0.9, 1.25] {
        tube_ring(m, base + Y(y), VY, 0.23, 0.012, 14, [0.4, 0.3, 0.16]);
    }
}

/// A robed, hooded mourner statue on a plinth.
pub fn statue(m: &mut Mesh3D, base: Vec3, yaw: f32, stone: C3) {
    let (f, s) = frame_of(yaw);
    chamfer_box(
        m,
        base + Y(0.15),
        s,
        VY,
        f,
        v(0.42, 0.15, 0.42),
        0.03,
        tint(stone, 0.7),
    );
    let mut figure = empty();
    lathe(
        &mut figure,
        v(0.0, 0.3, 0.0),
        VY,
        &[
            (0.0, 0.3),
            (0.2, 0.34),
            (0.9, 0.26),
            (1.3, 0.2),
            (1.5, 0.22),
            (1.62, 0.14),
            (1.75, 0.0),
        ],
        12,
        tint(stone, 1.05),
        true,
    );
    lathe(
        &mut figure,
        v(0.0, 1.72, 0.02),
        VY,
        &[
            (0.0, 0.16),
            (0.14, 0.2),
            (0.3, 0.17),
            (0.42, 0.06),
            (0.46, 0.0),
        ],
        10,
        tint(stone, 1.0),
        true,
    );
    disc(
        &mut figure,
        v(0.0, 1.88, 0.17),
        VZ,
        0.1,
        8,
        tint(stone, 0.35),
    );
    kit_block(
        &mut figure,
        v(0.0, 1.2, 0.2),
        VX,
        VY,
        VZ,
        v(0.16, 0.08, 0.08),
        tint(stone, 1.15),
    );
    append(m, &figure, &pose_mapper(base, yaw, 0.0, 0.0, 1.0), [1.0; 3]);
}

/// The memory stone: a monolith with a carved face and glowing runes.
pub fn monolith(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, stone: C3, color: C3) {
    let (f, s) = frame_of(yaw);
    chamfer_box(
        m,
        base + Y(0.12),
        s,
        VY,
        f,
        v(0.7, 0.12, 0.5),
        0.04,
        tint(stone, 0.65),
    );
    chamfer_box(
        m,
        base + Y(1.25),
        s,
        VY,
        f,
        v(0.5, 1.05, 0.28),
        0.08,
        tint(stone, 0.9),
    );
    lathe(
        m,
        base + Y(2.3),
        f,
        &[(-0.28, 0.35), (0.28, 0.35)],
        3,
        tint(stone, 0.9),
        true,
    );
    let face = base + Y(1.55) + f * 0.29;
    for sx in [-0.13, 0.13] {
        disc(glow, face + s * sx + Y(0.1), f, 0.05, 6, tint(color, 0.8));
    }
    kit_block(
        m,
        face - Y(0.08),
        s,
        VY,
        f,
        v(0.04, 0.1, 0.03),
        tint(stone, 1.1),
    );
    kit_block(
        m,
        face - Y(0.28),
        s,
        VY,
        f,
        v(0.1, 0.015, 0.02),
        tint(stone, 0.5),
    );
    for k in 0..6 {
        let y = 0.45 + k as f32 * 0.14;
        let w = [0.2, 0.1, 0.16, 0.08, 0.18, 0.12][k];
        let c = base + Y(y) + f * 0.29;
        quad(
            glow,
            [
                c - s * w,
                c + s * w,
                c + s * w + Y(0.035),
                c - s * w + Y(0.035),
            ],
            f,
            tint(color, 0.55),
        );
    }
}

/// A standing sigil stone, its glyph glowing in its own colour.
pub fn sigil_stone(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    stone: C3,
    color: C3,
    glyph: usize,
) {
    let (f, s) = frame_of(yaw);
    chamfer_box(
        m,
        base + Y(0.1),
        s,
        VY,
        f,
        v(0.46, 0.1, 0.46),
        0.03,
        tint(stone, 0.6),
    );
    chamfer_box(
        m,
        base + Y(1.05),
        s,
        VY,
        f,
        v(0.32, 0.95, 0.32),
        0.06,
        tint(stone, 0.85),
    );
    lathe(
        m,
        base + Y(2.0),
        VY,
        &[(0.0, 0.33), (0.22, 0.0)],
        4,
        tint(stone, 0.9),
        true,
    );
    for face in [f, -f] {
        let c = base + Y(1.25) + face * 0.33;
        let side = v(face.z, 0.0, -face.x);
        let mut stroke = |a: Vec3, b: Vec3| {
            let d = norm(b - a);
            let n = norm(d.cross(face)) * 0.028;
            quad(glow, [a + n, b + n, b - n, a - n], face, tint(color, 0.8));
        };
        let p = |x: f32, y: f32| c + side * x + Y(y);
        match glyph {
            0 => {
                stroke(p(0.0, 0.34), p(0.0, -0.34));
                stroke(p(-0.2, 0.12), p(0.2, 0.12));
                stroke(p(-0.16, -0.2), p(0.16, -0.2));
            }
            1 => {
                stroke(p(-0.2, 0.3), p(0.0, -0.32));
                stroke(p(0.2, 0.3), p(0.0, -0.32));
                stroke(p(-0.22, 0.02), p(0.22, 0.02));
            }
            _ => {
                stroke(p(0.0, 0.32), p(0.24, 0.0));
                stroke(p(0.24, 0.0), p(0.0, -0.32));
                stroke(p(0.0, -0.32), p(-0.24, 0.0));
                stroke(p(-0.24, 0.0), p(0.0, 0.32));
            }
        }
        disc(
            glow,
            p(0.0, 0.0) + face * 0.004,
            face,
            0.045,
            8,
            tint(color, 1.0),
        );
    }
}

/// A glowing rune circle on the floor.
pub fn rune_circle(glow: &mut Mesh3D, center: Vec3, r: f32, color: C3, spokes: usize) {
    let ring = |glow: &mut Mesh3D, r0: f32, r1: f32, k: f32| {
        for i in 0..56 {
            let a0 = (i as f32 / 56.0) * TAU;
            let a1 = ((i + 1) as f32 / 56.0) * TAU;
            let p = |a: f32, rr: f32| center + v(a.cos() * rr, 0.013, a.sin() * rr);
            quad(
                glow,
                [p(a0, r1), p(a1, r1), p(a1, r0), p(a0, r0)],
                VY,
                tint(color, k),
            );
        }
    };
    ring(glow, r - 0.08, r, 0.5);
    ring(glow, r * 0.62 - 0.05, r * 0.62, 0.42);
    for k in 0..spokes {
        let a = (k as f32 / spokes as f32) * TAU;
        let d = v(a.cos(), 0.0, a.sin());
        let n = v(-d.z, 0.0, d.x) * 0.025;
        let a0 = center + d * (r * 0.62) + Y(0.013);
        let a1 = center + d * (r - 0.08) + Y(0.013);
        quad(glow, [a0 + n, a1 + n, a1 - n, a0 - n], VY, tint(color, 0.4));
        let g = center + d * (r * 0.81) + Y(0.014);
        quad(
            glow,
            [
                g + n * 3.0 + d * 0.06,
                g - n * 3.0 + d * 0.06,
                g - n * 3.0 - d * 0.06,
                g + n * 3.0 - d * 0.06,
            ],
            VY,
            tint(color, 0.55),
        );
    }
}

/// A spider web across a corner: radial threads and a spiral.
pub fn web(m: &mut Mesh3D, center: Vec3, facing: Vec3, size: f32) {
    let side = v(facing.z, 0.0, -facing.x);
    let color = [0.8, 0.8, 0.84];
    let p = |a: f32, r: f32| center + side * (a.cos() * r) + Y(a.sin() * r);
    let mut thread = |a: Vec3, b: Vec3| {
        let d = norm(b - a);
        let n = norm(d.cross(facing)) * 0.008;
        quad(m, [a + n, b + n, b - n, a - n], facing, color);
    };
    let spokes = 9;
    for k in 0..spokes {
        thread(p(0.0, 0.0), p((k as f32 / spokes as f32) * TAU, size));
    }
    for ring in 1..=5 {
        let r = (ring as f32 / 5.4) * size;
        for k in 0..spokes {
            thread(
                p((k as f32 / spokes as f32) * TAU, r),
                p(((k + 1) as f32 / spokes as f32) * TAU, r * 0.97),
            );
        }
    }
}

/// A nest of twigs and pale eggs.
pub fn nest(m: &mut Mesh3D, center: Vec3, r: f32, rng: &mut ArtRng) {
    for _ in 0..40 {
        let a = rng.range(0.0, TAU);
        let rr = r * rng.range(0.6, 1.0);
        let p = center + v(a.cos() * rr, rng.range(0.03, 0.2), a.sin() * rr);
        let dy = rng.range(-0.2, 0.2);
        let d = v(-a.sin(), dy, a.cos()) * rng.range(0.2, 0.4);
        let shade = rng.range(0.7, 1.2);
        bar(
            m,
            p - d,
            p + d,
            0.014,
            0.014,
            VY,
            tint([0.32, 0.24, 0.16], shade),
        );
    }
    for _ in 0..3 {
        let a = rng.range(0.0, TAU);
        let p = center + v(a.cos() * r * 0.3, 0.08, a.sin() * r * 0.3);
        let x = rng.range(-0.3, 0.3);
        let z = rng.range(-0.3, 0.3);
        lathe(
            m,
            p,
            norm(v(x, 1.0, z)),
            &[(0.0, 0.08), (0.06, 0.11), (0.16, 0.09), (0.22, 0.0)],
            8,
            [0.78, 0.74, 0.62],
            true,
        );
    }
}

/// Ring of stones and crossed logs; the flame is a fixture.
pub fn campfire(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, rng: &mut ArtRng) {
    for k in 0..9 {
        let a = (k as f32 / 9.0) * TAU;
        rock(
            m,
            base + v(a.cos() * 0.42, 0.0, a.sin() * 0.42),
            v(0.13, 0.1, 0.11),
            [0.3, 0.28, 0.27],
            a,
        );
    }
    for k in 0..4 {
        let a = (k as f32 / 4.0) * TAU + 0.4;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            base + d * 0.34 + Y(0.04),
            base + Y(0.3),
            0.05,
            0.05,
            VY,
            tint(WOOD, 0.7),
        );
    }
    for _ in 0..8 {
        let a = rng.range(0.0, TAU);
        let x = a.cos() * rng.range(0.0, 0.2);
        let z = a.sin() * rng.range(0.0, 0.2);
        rock(
            glow,
            base + v(x, 0.04, z),
            v(0.06, 0.04, 0.05),
            [0.9, 0.34, 0.08],
            a,
        );
    }
}

/// A rolled-out bedroll with a pillow.
pub fn bedroll(m: &mut Mesh3D, base: Vec3, yaw: f32, color: C3) {
    let (f, s) = frame_of(yaw);
    chamfer_box(
        m,
        base + Y(0.06),
        s,
        VY,
        f,
        v(0.38, 0.06, 0.95),
        0.05,
        color,
    );
    lathe(
        m,
        base + Y(0.13) - f * 0.95,
        s,
        &[(-0.38, 0.12), (0.38, 0.12)],
        8,
        tint(color, 1.15),
        true,
    );
    chamfer_box(
        m,
        base + Y(0.12) + f * 0.72,
        s,
        VY,
        f,
        v(0.24, 0.06, 0.14),
        0.05,
        [0.82, 0.78, 0.7],
    );
}

/// A small A-frame tent of canvas on poles.
pub fn tent(m: &mut Mesh3D, base: Vec3, yaw: f32, canvas: C3) {
    let (f, s) = frame_of(yaw);
    let (h, w, d) = (1.35, 0.95, 1.1);
    let ridge_a = base + Y(h) - f * d;
    let ridge_b = base + Y(h) + f * d;
    for sgn in [-1.0f32, 1.0] {
        let foot_a = base - f * d + s * (sgn * w);
        let foot_b = base + f * d + s * (sgn * w);
        quad(
            m,
            [foot_a, foot_b, ridge_b, ridge_a],
            norm(s * sgn + Y(0.6)),
            tint(canvas, if sgn > 0.0 { 0.8 } else { 1.05 }),
        );
    }
    tri(
        m,
        base - f * d + s * -w,
        base - f * d + s * w,
        ridge_a,
        -f,
        tint(canvas, 0.7),
    );
    for p in [ridge_a, ridge_b] {
        bar(
            m,
            with_y(p, base.y),
            p + Y(0.12),
            0.03,
            0.03,
            s,
            tint(WOOD, 1.1),
        );
    }
    bar(
        m,
        ridge_a + Y(0.02),
        ridge_b + Y(0.02),
        0.025,
        0.025,
        VY,
        tint(WOOD, 1.1),
    );
}

/// A standing ring gate of stone blocks with glowing runes.
pub fn portal_ring(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    facing: Vec3,
    radius: f32,
    stone: C3,
    color: C3,
) {
    let side = v(facing.z, 0.0, -facing.x);
    let blocks = 18;
    for k in 0..blocks {
        let a = (k as f32 / blocks as f32) * TAU;
        let c = center + side * (a.cos() * radius) + Y(a.sin() * radius);
        let t = side * -a.sin() + Y(a.cos());
        let out = side * a.cos() + Y(a.sin());
        kit_block(
            m,
            c,
            t,
            out,
            facing,
            v(radius * TAU / blocks as f32 * 0.48, 0.2, 0.22),
            tint(stone, if k % 2 == 1 { 0.8 } else { 0.95 }),
        );
        let g = c - out * 0.2 + facing * 0.23;
        quad(
            glow,
            [
                g - t * 0.06,
                g + t * 0.06,
                g + t * 0.06 + out * 0.08,
                g - t * 0.06 + out * 0.08,
            ],
            facing,
            tint(color, 0.7),
        );
    }
    for sgn in [-1.0f32, 1.0] {
        chamfer_box(
            m,
            with_y(center + side * (sgn * radius), center.y - radius + 0.3),
            side,
            VY,
            facing,
            v(0.35, 0.3, 0.35),
            0.04,
            tint(stone, 0.7),
        );
    }
}

/// A tall crystal pylon on a carved stone base.
pub fn pylon(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    stone: C3,
    color: C3,
    rng: &mut ArtRng,
) {
    chamfer_box(
        m,
        base + Y(0.25),
        VX,
        VY,
        VZ,
        v(0.3, 0.25, 0.3),
        0.04,
        tint(stone, 0.75),
    );
    crystal(glow, base + Y(0.48), 0.2, 1.1, tint(color, 0.7), 6);
    for k in 0..3 {
        let a = (k as f32 / 3.0) * TAU + rng.range(0.0, 1.0);
        crystal(
            glow,
            base + Y(0.48) + v(a.cos() * 0.18, 0.0, a.sin() * 0.18),
            0.08,
            0.45,
            tint(color, 0.55),
            5,
        );
    }
}

/// A throne built of bones, crowned with horns and skulls.
pub fn bone_throne(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, stone: C3) {
    let (f, s) = frame_of(yaw);
    chamfer_box(
        m,
        base + Y(0.2),
        s,
        VY,
        f,
        v(1.0, 0.2, 0.8),
        0.05,
        tint(stone, 0.6),
    );
    chamfer_box(
        m,
        base + Y(0.62) - f * 0.05,
        s,
        VY,
        f,
        v(0.6, 0.22, 0.48),
        0.05,
        tint(stone, 0.8),
    );
    chamfer_box(
        m,
        base + Y(1.55) - f * 0.45,
        s,
        VY,
        f,
        v(0.66, 1.1, 0.12),
        0.05,
        tint(stone, 0.72),
    );
    for k in 0..7 {
        let x = -0.55 + k as f32 * 0.183;
        let top = base + Y(2.65 + ((k as f32 / 6.0) * PI).sin() * 0.45) - f * 0.45 + s * x;
        bar(
            m,
            with_y(top, base.y + 1.1) + f * 0.08,
            top + f * 0.08,
            0.03,
            0.03,
            f,
            BONE,
        );
        skull(m, top + f * 0.12, 0.9, yaw, -0.1, BONE);
    }
    for sgn in [-1.0f32, 1.0] {
        let root = base + Y(2.6) - f * 0.45 + s * (sgn * 0.6);
        let pts = bezier(
            root,
            root + s * (sgn * 0.6) + Y(0.3),
            root + s * (sgn * 0.45) + Y(1.1) + f * 0.2,
            10,
        );
        tube(m, &pts, &|t| 0.12 * (1.0 - t) + 0.01, 7, &|t, _| {
            mix_color([0.12, 0.1, 0.1], [0.46, 0.4, 0.36], t)
        });
        kit_block(
            m,
            base + Y(0.95) + s * (sgn * 0.68),
            s,
            VY,
            f,
            v(0.1, 0.12, 0.42),
            tint(stone, 0.85),
        );
        skull(
            m,
            base + Y(1.12) + s * (sgn * 0.68) + f * 0.34,
            1.05,
            yaw,
            0.0,
            BONE,
        );
    }
    disc(glow, base + Y(2.2) - f * 0.32, f, 0.14, 10, [0.9, 0.3, 0.1]);
}

/// The Heart of the cavern, modelled around the origin (about 1.9 m tall).
/// Returns the local points where the vessels end.
pub fn heart_body(m: &mut Mesh3D, glow: &mut Mesh3D, rng: &mut ArtRng) -> Vec<Vec3> {
    let axis = norm(v(0.18, 1.0, 0.08));
    let apex = v(0.08, -0.95, 0.0);
    let flesh = |i: usize| {
        mix_color(
            [0.22, 0.02, 0.04],
            [0.62, 0.08, 0.1],
            (i as f32 / 6.0).min(1.0),
        )
    };
    lathe_fn(
        m,
        apex,
        axis,
        &[
            (0.0, 0.0),
            (0.16, 0.26),
            (0.42, 0.5),
            (0.78, 0.66),
            (1.12, 0.64),
            (1.36, 0.52),
            (1.52, 0.3),
            (1.58, 0.0),
        ],
        9,
        &|i| flesh(i),
        true,
    );
    for sx in [-1.0f32, 1.0] {
        let c = v(sx * 0.3, 0.52, -0.1);
        lathe_fn(
            m,
            c - Y(0.34),
            VY,
            &[
                (0.0, 0.0),
                (0.12, 0.26),
                (0.34, 0.36),
                (0.56, 0.28),
                (0.7, 0.0),
            ],
            8,
            &|i| flesh(i + 1),
            true,
        );
    }
    // Great vessels: aorta arching back, two more rising.
    let vessels = [
        (
            v(0.05, 0.62, 0.05),
            v(0.2, 1.3, 0.1),
            v(-0.3, 1.55, -0.25),
            0.2,
        ),
        (
            v(-0.28, 0.72, -0.05),
            v(-0.45, 1.2, 0.1),
            v(-0.55, 1.6, 0.25),
            0.14,
        ),
        (
            v(0.32, 0.7, -0.12),
            v(0.55, 1.15, -0.2),
            v(0.7, 1.55, -0.35),
            0.13,
        ),
    ];
    let mut ends = vec![];
    for (a, c, b, r) in vessels {
        tube(
            m,
            &bezier(a, c, b, 10),
            &|t| r * (1.0 - t * 0.25),
            8,
            &|t, _| mix_color([0.4, 0.05, 0.08], [0.24, 0.03, 0.05], t),
        );
        ends.push(b);
    }
    // Glowing arteries crawling over the surface.
    for k in 0..7 {
        let phase = (k as f32 / 7.0) * TAU + rng.range(-0.2, 0.2);
        let pts: Vec<Vec3> = (0..=18)
            .map(|i| {
                let t = i as f32 / 18.0;
                let h = 1.45 * (1.0 - t);
                let prof = ((h / 1.5).min(1.0) * PI).sin() * 0.62 + 0.04;
                let a = phase + (t * 5.0 + k as f32).sin() * 0.35;
                apex + axis * h + v(a.cos() * prof, 0.0, a.sin() * prof)
            })
            .collect();
        tube(glow, &pts, &|t| 0.026 * (1.0 - t * 0.5), 5, &|t, _| {
            mix_color([1.0, 0.35, 0.18], [0.9, 0.08, 0.08], t)
        });
    }
    ends
}

/// The guard post's fire trap: an iron grate over glowing embers.
pub fn vent(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, rng: &mut ArtRng) {
    for k in -3..=3 {
        let x = k as f32 * 0.14;
        bar(
            m,
            base + v(x, 0.03, -0.5),
            base + v(x, 0.03, 0.5),
            0.025,
            0.02,
            VY,
            [0.2, 0.17, 0.16],
        );
        bar(
            m,
            base + v(-0.5, 0.035, x),
            base + v(0.5, 0.035, x),
            0.025,
            0.02,
            VY,
            [0.24, 0.2, 0.18],
        );
    }
    for _ in 0..10 {
        let x = rng.range(-0.4, 0.4);
        let z = rng.range(-0.4, 0.4);
        let shade = rng.range(0.5, 0.8);
        rock(
            glow,
            base + v(x, 0.0, z),
            v(0.07, 0.03, 0.06),
            tint([1.0, 0.4, 0.08], shade),
            rng.range(0.0, TAU),
        );
    }
}

const TREASURY_WAX: C3 = [0.96, 0.88, 0.68];

/// A candle's flame (web `treasury.js` flame).
fn candle_flame(glow: &mut Mesh3D, base: Vec3, size: f32) {
    lathe(
        glow,
        base,
        VY,
        &[
            (0.0, 0.0),
            (0.012 * size, 0.022 * size),
            (0.04 * size, 0.028 * size),
            (0.08 * size, 0.014 * size),
            (0.11 * size, 0.0),
        ],
        6,
        [1.0, 0.42, 0.07],
        false,
    );
    lathe(
        glow,
        base + Y(0.006 * size),
        VY,
        &[(0.0, 0.0), (0.02 * size, 0.013 * size), (0.05 * size, 0.0)],
        5,
        [1.0, 0.86, 0.5],
        false,
    );
}

/// A wax candle with drips (web `treasury.js` candle).
pub fn candle(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    radius: f32,
    height: f32,
    rng: &mut ArtRng,
) {
    let wax = tint(TREASURY_WAX, rng.range(0.9, 1.04));
    lathe(
        m,
        base,
        VY,
        &[
            (0.0, radius * 1.06),
            (height * 0.1, radius),
            (height, radius),
            (height, radius * 0.7),
        ],
        8,
        wax,
        true,
    );
    for k in 0..2 {
        let a = rng.range(0.0, TAU);
        let d = v(a.cos(), 0.0, a.sin());
        let len = rng.range(0.03, height * 0.45);
        cuboid(
            m,
            base + d * (radius * 0.98) + Y(height - len * 0.5),
            v(0.008 + k as f32 * 0.003, len * 0.5, 0.008),
            tint(wax, 1.06),
        );
    }
    cuboid(
        m,
        base + Y(height + 0.012),
        v(0.003, 0.012, 0.003),
        [0.05, 0.04, 0.03],
    );
    candle_flame(
        glow,
        base + Y(height + 0.014),
        (radius / 0.045).clamp(0.8, 1.35),
    );
}

/// A loose group of candles (web vault piece `candles`).
pub fn candles(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    count: usize,
    spread: f32,
    rng: &mut ArtRng,
) {
    let mut placed: Vec<Vec3> = vec![];
    let mut k = 0;
    while k < count * 4 && placed.len() < count {
        k += 1;
        let a = rng.range(0.0, TAU);
        let r = spread * rng.unit64().sqrt() as f32;
        let p = base + v(a.cos() * r, 0.0, a.sin() * r);
        if placed.iter().all(|&q| hz(q - p).length() > 0.11) {
            placed.push(p);
            let radius = rng.range(0.03, 0.058);
            let height = rng.range(0.1, 0.42);
            candle(m, glow, p, radius, height, rng);
        }
    }
}

/// A low fern (web `sanctuary.js` fern).
pub fn fern(m: &mut Mesh3D, base: Vec3, k: f32, rng: &mut ArtRng) {
    for i in 0..9 {
        let a = i as f32 * TAU / 9.0 + rng.range(-0.25, 0.25);
        let d = v(a.cos(), 0.0, a.sin());
        let side = v(-d.z, 0.0, d.x);
        let len = rng.range(0.38, 0.55) * k;
        let lift = rng.range(0.28, 0.4) * k;
        let pts = [
            base,
            base + d * (len * 0.35) + Y(lift * 0.85),
            base + d * (len * 0.7) + Y(lift),
            base + d * len + Y(lift * 0.6),
        ];
        let widths = [0.012 * k, 0.05 * k, 0.04 * k, 0.0];
        let g0 = 0.16 + rng.range(-0.03, 0.04);
        let g1 = 0.36 + rng.range(-0.05, 0.06);
        let green = [g0, g1, 0.12];
        for j in 0..3 {
            let (p0, p1) = (pts[j], pts[j + 1]);
            let (w0, w1) = (widths[j], widths[j + 1]);
            let mut n = norm((p1 - p0).cross(side));
            if n.y < 0.0 {
                n = -n;
            }
            let c = tint(green, 0.8 + j as f32 * 0.12);
            let leaf = [
                p0 - side * w0,
                p0 + side * w0,
                p1 + side * w1,
                p1 - side * w1,
            ];
            quad(m, leaf, n, c);
            quad(m, leaf, -n, tint(c, 0.7));
        }
    }
}

/// A heap of fallen stones (web `treasury.js` rubble).
pub fn rubble(m: &mut Mesh3D, base: Vec3, radius: f32, yaw: f32, stone: C3, rng: &mut ArtRng) {
    let count = 4 + rng.index(4);
    for k in 0..count {
        let a = yaw + k as f32 * 1.7 + rng.range(-0.4, 0.4);
        let r = if k == 0 {
            0.0
        } else {
            radius * rng.range(0.25, 0.8)
        };
        let size = radius
            * if k == 0 {
                rng.range(0.45, 0.6)
            } else {
                rng.range(0.16, 0.36)
            };
        let sy = rng.range(0.7, 1.3);
        let shade = rng.range(0.55, 0.95);
        rock(
            m,
            base + v(a.cos() * r, 0.0, a.sin() * r),
            v(size * 1.35, size * sy, size * 1.1),
            tint(stone, shade),
            a,
        );
    }
}

const REMAINS_BONE: C3 = [0.86, 0.8, 0.66];

/// A fallen explorer's skeleton (web `treasury.js` remains).
pub fn remains(m: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let front = yaw_dir(yaw);
    let side = v(front.z, 0.0, -front.x);
    let skull_at = base + front * 0.5 + Y(0.1);
    let mut head = empty();
    lathe(
        &mut head,
        Vec3::ZERO,
        VZ,
        &[
            (-0.1, 0.0),
            (-0.09, 0.06),
            (-0.03, 0.1),
            (0.04, 0.095),
            (0.09, 0.07),
            (0.11, 0.03),
            (0.115, 0.0),
        ],
        10,
        REMAINS_BONE,
        false,
    );
    for sx in [-1.0, 1.0] {
        disc(
            &mut head,
            v(sx * 0.035, 0.015, 0.104),
            VZ,
            0.024,
            8,
            [0.08, 0.06, 0.05],
        );
    }
    disc(
        &mut head,
        v(0.0, -0.03, 0.108),
        VZ,
        0.012,
        5,
        [0.1, 0.08, 0.06],
    );
    kit_block(
        &mut head,
        v(0.0, -0.075, 0.07),
        VX,
        VY,
        VZ,
        v(0.045, 0.02, 0.035),
        tint(REMAINS_BONE, 0.9),
    );
    let turn = yaw + rng.range(-0.6, 0.6);
    let roll = rng.range(-0.4, 0.4);
    append(
        m,
        &head,
        &pose_mapper(skull_at, turn, -0.3, roll, 1.0),
        [1.0; 3],
    );
    for k in 0..4 {
        let at = base + front * (0.22 - k as f32 * 0.07) + Y(0.03);
        for s in [-1.0f32, 1.0] {
            bar(
                m,
                at,
                at + side * (s * 0.14) - front * 0.03,
                0.012,
                0.01,
                VY,
                tint(REMAINS_BONE, 0.95),
            );
        }
    }
    bar(
        m,
        base + front * 0.3 + Y(0.03),
        base - front * 0.1 + Y(0.03),
        0.018,
        0.018,
        VY,
        tint(REMAINS_BONE, 0.9),
    );
    for (a, b) in [
        ([0.1, -0.2], [0.5, -0.45]),
        ([-0.15, 0.1], [-0.55, 0.35]),
        ([-0.1, -0.12], [-0.5, -0.3]),
    ] {
        let p0 = base + side * a[0] + front * a[1] + Y(0.025);
        let p1 = base + side * b[0] + front * b[1] + Y(0.025);
        bar(m, p0, p1, 0.016, 0.016, VY, REMAINS_BONE);
        for p in [p0, p1] {
            kit_block(
                m,
                p,
                side,
                VY,
                front,
                v(0.025, 0.022, 0.025),
                tint(REMAINS_BONE, 1.05),
            );
        }
    }
}

/// An iron chain hanging from the vault (web `treasury.js` hangingChain).
pub fn hanging_chain(m: &mut Mesh3D, top: Vec3, length: f32, rng: &mut ArtRng) {
    let iron = [0.14, 0.12, 0.11];
    let mut y = 0.0;
    let mut alt = rng.chance(0.5);
    while y < length {
        let a = top - Y(y);
        bar(
            m,
            a,
            a - Y(0.1),
            0.026,
            0.009,
            if alt { VX } else { VZ },
            tint(iron, 1.2),
        );
        y += 0.085;
        alt = !alt;
    }
    let end = top - Y(length + 0.08);
    bar(m, end, end - Y(0.12), 0.012, 0.012, VX, iron);
    bar(
        m,
        end - Y(0.12),
        end - Y(0.14) + VX * 0.08,
        0.012,
        0.012,
        VY,
        iron,
    );
}

/// An iron lantern on a chain (web `sanctuary.js` lantern).
pub fn lantern(m: &mut Mesh3D, glow: &mut Mesh3D, top: Vec3, length: f32, rng: &mut ArtRng) {
    let iron = [0.14, 0.12, 0.11];
    let mut y = 0.0;
    let mut alt = rng.chance(0.5);
    while y < length {
        bar(
            m,
            top - Y(y),
            top - Y(y + 0.1),
            0.024,
            0.008,
            if alt { VX } else { VZ },
            tint(iron, 1.25),
        );
        y += 0.085;
        alt = !alt;
    }
    let c = top - Y(length + 0.2);
    lathe(
        m,
        c + Y(0.12),
        VY,
        &[(0.0, 0.13), (0.06, 0.06), (0.1, 0.02)],
        8,
        tint(iron, 1.2),
        false,
    );
    lathe(
        m,
        c - Y(0.16),
        VY,
        &[(0.0, 0.05), (0.04, 0.12), (0.06, 0.12)],
        8,
        tint(iron, 1.1),
        true,
    );
    for k in 0..6 {
        let a = k as f32 * TAU / 6.0;
        let d = v(a.cos(), 0.0, a.sin());
        bar(
            m,
            c + d * 0.115 - Y(0.1),
            c + d * 0.12 + Y(0.12),
            0.012,
            0.012,
            d,
            iron,
        );
    }
    crystal(glow, c - Y(0.1), 0.07, 0.2, [0.95, 0.6, 0.22], 6);
    disc(glow, c - Y(0.095), VY, 0.1, 8, [0.6, 0.34, 0.1]);
}
