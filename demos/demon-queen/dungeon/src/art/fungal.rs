//! La Gruta de hongos (web prototype `galleries/fungal.js`): a gallery the
//! fungi took back — a towering mother mushroom with glowing gills, rings of
//! ink caps, bells, puffballs, coral and cup fungi tied by mycelium,
//! brackets up the pilasters, moss, a fallen log and glow-worm threads.
use super::kit::*;
use super::stage::*;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const STEM: C3 = [0.66, 0.63, 0.56];
pub const CAPS: [C3; 4] = [
    [0.3, 1.0, 0.85],
    [0.95, 0.35, 1.0],
    [0.4, 0.7, 1.0],
    [0.55, 1.0, 0.45],
];
const MOSS: C3 = [0.14, 0.3, 0.22];
const BARK: C3 = [0.24, 0.17, 0.12];

/// The mother mushroom: twisted grooved stem, root flares, a frilled skirt,
/// a wavy warty cap with glowing gills beneath, brackets on the stem.
pub fn giant_mushroom(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    h: f32,
    r_cap: f32,
    cap: C3,
    rng: &mut ArtRng,
) {
    let phase = rng.range(0.0, TAU);
    let lean = move |y: f32| {
        v(
            (y * 0.9 + phase).sin() * 0.2 * (y / h),
            0.0,
            (y * 0.7 + phase).cos() * 0.14 * (y / h),
        )
    };
    let profile = [
        (0.0f32, 0.7f32),
        (0.1, 0.52),
        (0.35, 0.38),
        (1.0, 0.31),
        (h * 0.6, 0.29),
        (h - 0.35, 0.35),
        (h, 0.52),
    ];
    let radius_at = move |y: f32| {
        for k in 1..profile.len() {
            if y <= profile[k].0 {
                let (y0, r0) = profile[k - 1];
                let (y1, r1) = profile[k];
                return r0 + (r1 - r0) * (y - y0) / (y1 - y0).max(1e-6);
            }
        }
        profile[profile.len() - 1].1
    };
    const SR: usize = 26;
    const SC: usize = 30;
    let stem_at = |i: usize, j: usize| {
        let y = (i as f32 / SR as f32) * h;
        let a = (j as f32 / SC as f32) * TAU;
        let r = radius_at(y) * (1.0 + 0.07 * (3.0 * a + y * 2.2).sin() + 0.04 * (8.0 * a).sin());
        base + lean(y) + v(a.cos() * r, y, a.sin() * r)
    };
    grid_surface(
        m,
        SR,
        SC,
        &stem_at,
        &|i, j| {
            let a = (j as f32 / SC as f32) * TAU;
            let groove = (0.5 + 0.5 * (8.0 * a + (i as f32 / SR as f32) * 1.5).sin()).powi(3);
            tint(
                STEM,
                (0.62 + 0.3 * (1.0 - groove)) * (0.78 + 0.28 * (i as f32 / SR as f32)),
            )
        },
        true,
        Some(&|p, i, _| {
            p - (base + lean((i as f32 / SR as f32) * h) + Y((i as f32 / SR as f32) * h))
        }),
        None,
    );
    // Faint luminous veins climb the stem.
    for vv in 0..5 {
        let a0 = (vv as f32 / 5.0) * TAU + rng.range(0.0, 0.4);
        let pts: Vec<Vec3> = (0..=16)
            .map(|i| {
                let y = 0.2 + (i as f32 / 16.0) * (h - 0.5);
                let a = a0 + (y * 1.7 + vv as f32).sin() * 0.25;
                let r = radius_at(y)
                    * (1.0 + 0.07 * (3.0 * a + y * 2.2).sin() + 0.04 * (8.0 * a).sin())
                    + 0.012;
                base + lean(y) + v(a.cos() * r, y, a.sin() * r)
            })
            .collect();
        tube(glow, &pts, &|t| 0.012 * (1.0 - t * 0.5), 4, &|_, _| {
            tint(cap, 0.32)
        });
    }
    // Root flares hug the floor.
    for k in 0..6 {
        let a = (k as f32 / 6.0) * TAU + rng.range(-0.2, 0.2);
        let d = v(a.cos(), 0.0, a.sin());
        let reach = rng.range(1.2, 1.6);
        let pts = bezier(
            base + d * 0.4 + Y(0.4),
            base + d * 0.8 + Y(0.1),
            base + d * reach + Y(0.06),
            10,
        );
        tube(
            m,
            &pts,
            &|t| 0.22 * (1.0 - t).powf(0.7) + 0.05,
            7,
            &|t, around| {
                tint(
                    STEM,
                    (0.6 + 0.2 * (around * TAU * 3.0).sin()) * (0.9 - t * 0.2),
                )
            },
        );
        lathe(
            m,
            pts[pts.len() - 1],
            d,
            &[(-0.02, 0.05), (0.02, 0.04), (0.05, 0.0)],
            6,
            tint(STEM, 0.6),
            false,
        );
    }
    // The frilled skirt hanging from the stem.
    let skirt_y = h * 0.72;
    let sr = radius_at(skirt_y);
    let skirt_at = |i: usize, j: usize| {
        let a = (j as f32 / 40.0) * TAU;
        let t = i as f32 / 4.0;
        let r = sr + 0.03 + t * 0.26 * (1.0 + 0.15 * (9.0 * a).sin());
        base + lean(skirt_y)
            + v(
                a.cos() * r,
                skirt_y - t * 0.3 - t * t * 0.08 * (7.0 * a).sin(),
                a.sin() * r,
            )
    };
    grid_surface(
        m,
        4,
        40,
        &skirt_at,
        &|i, _| tint(STEM, 1.05 - i as f32 * 0.06),
        true,
        Some(&|p, _, _| p - (base + Y(skirt_y + 0.6))),
        Some(&|_, _| tint(STEM, 0.6)),
    );
    // The cap: wavy rim, a raised dome with warts, glowing gills beneath.
    let top = base + lean(h) + Y(h);
    let rim_r =
        move |a: f32| r_cap * (1.0 + 0.05 * (5.0 * a + phase).sin() + 0.03 * (11.0 * a).sin());
    const CR: usize = 12;
    const CC: usize = 56;
    let under_at = move |i: usize, j: usize| {
        let a = (j as f32 / CC as f32) * TAU;
        let t = 0.18 + (i as f32 / CR as f32) * 0.82;
        let r = rim_r(a) * t;
        top + v(
            a.cos() * r,
            0.34 * t.powf(0.8) - 0.3 * smooth(0.82, 1.0, t),
            a.sin() * r,
        )
    };
    grid_surface(
        m,
        CR,
        CC,
        &under_at,
        &|i, _| tint(cap, 0.22 + 0.08 * (i as f32 / CR as f32)),
        true,
        Some(&|_, _, _| Y(-1.0)),
        None,
    );
    let top_at = move |i: usize, j: usize| {
        let a = (j as f32 / CC as f32) * TAU;
        let t = 1.0 - i as f32 / CR as f32;
        let r = rim_r(a) * t;
        let rim = under_at(CR, j).y;
        let dome = 0.95 * (1.0 - t * t).max(0.0).powf(0.62);
        top + v(
            a.cos() * r,
            rim - top.y + dome + 0.04 * (3.0 * a).sin() * t,
            a.sin() * r,
        )
    };
    grid_surface(
        m,
        CR,
        CC,
        &top_at,
        &|i, j| {
            mix_color(
                tint(cap, 0.3),
                tint(cap, 0.72),
                (1.0 - i as f32 / CR as f32) * 0.8
                    + 0.1 * (5.0 * (j as f32 / CC as f32) * TAU).sin(),
            )
        },
        true,
        Some(&|p, _, _| p - (top - Y(0.5))),
        None,
    );
    // Gills: thin glowing lamellae just under the cap.
    for k in 0..90 {
        let a = (k as f32 / 90.0) * TAU;
        let j = ((a / TAU) * CC as f32).round() as usize % CC;
        let p0 = under_at(1, j) - Y(0.012);
        let p1 = under_at(CR - 1, j) - Y(0.012);
        let side = v(-a.sin(), 0.0, a.cos()) * 0.012;
        quad(
            glow,
            [p0 - side, p1 - side * 2.0, p1 + side * 2.0, p0 + side],
            Y(-1.0),
            tint(cap, if k % 3 == 0 { 0.62 } else { 0.44 }),
        );
    }
    // Warts and glowing specks on the dome.
    for _ in 0..34 {
        let i = 2 + rng.index(CR - 3);
        let j = rng.index(CC);
        let p = top_at(i, j);
        if rng.chance(0.35) {
            let r = rng.range(0.03, 0.06);
            disc(glow, p + Y(0.02), norm(p - top), r, 6, tint(cap, 0.7));
        } else {
            lathe(
                m,
                p - Y(0.01),
                norm(p - (top - Y(0.6))),
                &[(0.0, 0.08), (0.04, 0.06), (0.07, 0.0)],
                6,
                mix_color(tint(cap, 0.9), [0.9, 0.88, 0.8], 0.5),
                true,
            );
        }
    }
    // Brackets up the stem.
    for k in 0..4 {
        let y = rng.range(0.8, h * 0.62);
        let a = rng.range(0.0, TAU);
        let d = v(a.cos(), 0.0, a.sin());
        let count = rng.index(2) + 2;
        let size = rng.range(0.16, 0.26);
        shelf_stack(
            m,
            glow,
            base + lean(y) + d * (radius_at(y) - 0.02) + Y(y),
            d,
            count,
            size,
            rng,
            CAPS[(k + 1) % CAPS.len()],
        );
    }
}

/// A stack of bracket fungi on a surface: ringed shelves with glowing lips.
#[allow(clippy::too_many_arguments)]
pub fn shelf_stack(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    p: Vec3,
    normal: Vec3,
    count: usize,
    size: f32,
    rng: &mut ArtRng,
    color: C3,
) {
    let n = norm(hz(normal));
    let side = v(-n.z, 0.0, n.x);
    for k in 0..count {
        let w = size * (1.0 - k as f32 * 0.18) * rng.range(0.85, 1.1);
        let c = p + Y(k as f32 * size * 0.42) + side * rng.range(-0.06, 0.06);
        let rows = 4usize;
        let cols = 14usize;
        let thick = w * 0.22;
        let body = mix_color([0.52, 0.44, 0.34], color, 0.35);
        let at = |i: usize, j: usize, y: f32| {
            let a = (j as f32 / cols as f32) * std::f32::consts::PI;
            let r =
                w * (0.2 + 0.8 * (i as f32 / rows as f32)) * (1.0 + 0.06 * (j as f32 * 1.7).sin());
            c + side * (a.cos() * r)
                + n * (a.sin() * r * 0.85)
                + Y(0.2 * w * (1.0 - (i as f32 / rows as f32).powi(2)) + y)
        };
        grid_surface(
            m,
            rows,
            cols,
            &|i, j| at(i, j, 0.0),
            &|i, _| {
                tint(
                    body,
                    0.62 + 0.14 * (i % 2) as f32 + 0.2 * (i as f32 / rows as f32),
                )
            },
            false,
            Some(&|_, _, _| VY),
            None,
        );
        grid_surface(
            m,
            rows,
            cols,
            &|i, j| at(i, j, -thick * (1.0 - (i as f32 / rows as f32) * 0.6)),
            &|_, _| tint(body, 0.42),
            false,
            Some(&|_, _, _| Y(-1.0)),
            None,
        );
        for j in 0..cols {
            let a0 = at(rows, j, 0.0);
            let a1 = at(rows, j + 1, 0.0);
            let b0 = at(rows, j, -thick * 0.4);
            let b1 = at(rows, j + 1, -thick * 0.4);
            let out = norm(hz((a0 + a1) * 0.5 - c));
            quad(m, [a0, a1, b1, b0], out, tint(body, 0.8));
            quad(
                glow,
                [b0, b1, b1 + Y(0.01), b0 + Y(0.01)],
                out,
                tint(color, 0.34),
            );
        }
    }
}

/// Tall ink caps: slender stems, conical caps with dark ragged hems.
pub fn ink_caps(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    height: f32,
    count: usize,
    color: C3,
    rng: &mut ArtRng,
) {
    for k in 0..count {
        let a = rng.range(0.0, TAU);
        let d = if k == 0 { 0.0 } else { rng.range(0.1, 0.35) };
        let b = base + v(a.cos() * d, 0.0, a.sin() * d);
        let h = (if k == 0 { 1.0 } else { rng.range(0.45, 0.85) }) * height;
        let (lx, lz) = (rng.range(-0.12, 0.12), rng.range(-0.12, 0.12));
        let lean = norm(v(lx, 1.0, lz));
        lathe(
            m,
            b,
            lean,
            &[(0.0, 0.05), (0.05, 0.035), (h * 0.9, 0.026), (h, 0.03)],
            6,
            STEM,
            true,
        );
        let top = b + lean * h;
        let r = h * 0.14;
        lathe_fn(
            m,
            top - lean * (h * 0.28),
            lean,
            &[
                (0.0, r * 0.95),
                (h * 0.06, r),
                (h * 0.2, r * 0.8),
                (h * 0.3, r * 0.4),
                (h * 0.34, 0.0),
            ],
            9,
            &|j| {
                mix_color(
                    [0.86, 0.84, 0.78],
                    [0.3, 0.3, 0.32],
                    (1.0 - j as f32 / 2.0).max(0.0),
                )
            },
            true,
        );
        lathe(
            glow,
            top - lean * (h * 0.29),
            lean,
            &[(0.0, r * 0.9), (0.01, r * 0.96)],
            9,
            tint(color, 0.45),
            false,
        );
    }
}

/// Glowing bells drooping from curved stems.
pub fn bell_cluster(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    count: usize,
    scale: f32,
    color: C3,
    rng: &mut ArtRng,
) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let d = rng.range(0.0, 0.28);
        let b = base + v(a.cos() * d, 0.0, a.sin() * d);
        let h = rng.range(0.3, 0.75) * scale;
        let out = v(a.cos(), 0.0, a.sin());
        let tip = b + Y(h) + out * (h * 0.35);
        tube_c(
            m,
            &bezier(b, b + Y(h * 0.9), tip, 8),
            0.012,
            4,
            tint(STEM, 0.9),
        );
        let down = norm(out * 0.4 + Y(-1.0));
        let r = rng.range(0.05, 0.09) * scale;
        lathe(
            m,
            tip - down * 0.02,
            down,
            &[
                (0.0, 0.0),
                (0.02, r * 0.6),
                (r * 1.2, r),
                (r * 1.5, r * 1.05),
            ],
            8,
            tint(color, 0.5),
            false,
        );
        lathe(
            glow,
            tip + down * (r * 0.3),
            down,
            &[(0.0, r * 0.4), (r, r * 0.85)],
            8,
            tint(color, 0.6),
            false,
        );
        disc(
            glow,
            tip + down * (r * 1.45),
            down,
            r * 0.25,
            6,
            tint(color, 0.9),
        );
    }
}

/// Puffballs of assorted sizes, speckled.
pub fn puffballs(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    count: usize,
    radius: f32,
    color: C3,
    rng: &mut ArtRng,
) {
    for k in 0..count {
        let a = rng.range(0.0, TAU);
        let d = if k == 0 { 0.0 } else { rng.range(0.2, 0.5) };
        let r = (if k == 0 { 1.0 } else { rng.range(0.35, 0.7) }) * radius;
        let c = base + v(a.cos() * d, 0.0, a.sin() * d);
        lathe_fn(
            m,
            c,
            VY,
            &[
                (0.0, r * 0.4),
                (r * 0.2, r * 0.85),
                (r * 0.7, r),
                (r * 1.3, r * 0.82),
                (r * 1.7, r * 0.3),
                (r * 1.8, 0.0),
            ],
            12,
            &|j| tint([0.86, 0.82, 0.72], 0.75 + j as f32 * 0.05),
            true,
        );
        for _ in 0..8 {
            let t = rng.range(0.0, TAU);
            let y = rng.range(0.6, 1.5) * r;
            let rr = (1.0 - ((y - r * 0.9) / (r * 0.9)).powi(2)).max(0.0).sqrt() * r;
            let q = c + v(t.cos() * rr, y, t.sin() * rr);
            let n = norm(q - (c + Y(r * 0.9)));
            let in_glow = rng.chance(0.4);
            let col = if rng.chance(0.4) {
                tint(color, 0.6)
            } else {
                [0.6, 0.56, 0.48]
            };
            disc(
                if in_glow { &mut *glow } else { &mut *m },
                q,
                n,
                r * 0.08,
                5,
                col,
            );
        }
    }
}

/// Branching coral fungus, luminous to the tips.
pub fn coral_fungus(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    scale: f32,
    color: C3,
    rng: &mut ArtRng,
) {
    #[allow(clippy::too_many_arguments)]
    fn branch(
        m: &mut Mesh3D,
        glow: &mut Mesh3D,
        p: Vec3,
        dir: Vec3,
        len: f32,
        r: f32,
        depth: usize,
        color: C3,
        rng: &mut ArtRng,
    ) {
        let end = p + dir * len;
        let (jx, jz) = (rng.range(-0.02, 0.02), rng.range(-0.02, 0.02));
        let mid = p.lerp(end, 0.5) + v(jx, 0.0, jz);
        let shade = tint(color, if depth > 1 { 0.5 } else { 0.35 });
        tube(
            if depth > 1 { &mut *glow } else { &mut *m },
            &[p, mid, end],
            &|t| r * (1.0 - t * 0.4),
            5,
            &|_, _| shade,
        );
        if depth >= 3 {
            disc(glow, end, dir, r * 0.9, 5, tint(color, 0.9));
            return;
        }
        for _ in 0..2 + if depth == 0 { 1 } else { 0 } {
            let d = norm(
                dir + v(
                    rng.range(-0.7, 0.7),
                    rng.range(-0.1, 0.4),
                    rng.range(-0.7, 0.7),
                ),
            );
            let l = len * rng.range(0.6, 0.8);
            branch(m, glow, end, d, l, r * 0.7, depth + 1, color, rng);
        }
    }
    for _ in 0..3 {
        let start = base + v(rng.range(-0.1, 0.1), 0.0, rng.range(-0.1, 0.1));
        let dir = norm(v(rng.range(-0.2, 0.2), 1.0, rng.range(-0.2, 0.2)));
        branch(m, glow, start, dir, 0.18 * scale, 0.035, 0, color, rng);
    }
}

/// Cup fungi glowing inside.
pub fn cup_fungi(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    count: usize,
    color: C3,
    rng: &mut ArtRng,
) {
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let d = rng.range(0.0, 0.3);
        let r = rng.range(0.04, 0.09);
        let c = base + v(a.cos() * d, 0.0, a.sin() * d);
        let up = norm(v(rng.range(-0.3, 0.3), 1.0, rng.range(-0.3, 0.3)));
        lathe(
            m,
            c,
            up,
            &[
                (0.0, 0.01),
                (r * 0.5, r * 0.6),
                (r * 1.1, r),
                (r * 1.2, r * 1.05),
            ],
            8,
            tint(color, 0.3),
            false,
        );
        lathe(
            glow,
            c + up * (r * 1.19),
            -up,
            &[(0.0, r * 0.98), (r * 0.7, r * 0.5), (r, 0.0)],
            8,
            tint(color, 0.62),
            false,
        );
    }
}

/// Glowing mycelium threads spreading over the floor.
pub fn mycelium(glow: &mut Mesh3D, pts: &[Vec3], width: f32, color: C3, rng: &mut ArtRng) {
    for k in 0..pts.len().saturating_sub(1) {
        let a = pts[k] + Y(0.012);
        let b = pts[k + 1] + Y(0.012);
        let d = norm(hz(b - a));
        let n = v(-d.z, 0.0, d.x) * (width * (1.0 - (k as f32 / pts.len() as f32) * 0.6));
        quad(glow, [a - n, b - n, b + n, a + n], VY, tint(color, 0.22));
        if rng.chance(0.3) {
            disc(glow, a + Y(0.004), VY, width * 2.5, 6, tint(color, 0.32));
        }
        if rng.chance(0.25) {
            let side = v(-d.z, 0.0, d.x) * if rng.chance(0.5) { 1.0 } else { -1.0 };
            let mut p = a;
            for _ in 0..3 {
                let step = rng.range(0.1, 0.22);
                let (jx, jz) = (rng.range(-0.05, 0.05), rng.range(-0.05, 0.05));
                let q = p + norm(side + d) * step + v(jx, 0.0, jz);
                let dd = norm(hz(q - p));
                let nn = v(-dd.z, 0.0, dd.x) * (width * 0.5);
                quad(
                    glow,
                    [p - nn, q - nn, q + nn, p + nn],
                    VY,
                    tint(color, 0.18),
                );
                p = q;
            }
        }
    }
}

/// Glow-worm silk hanging from the vault, beaded with light.
pub fn glow_threads(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    top: Vec3,
    length: f32,
    color: C3,
    rng: &mut ArtRng,
) {
    bar(m, top, top - Y(length), 0.003, 0.003, VX, [0.7, 0.75, 0.78]);
    let mut y = 0.2;
    while y < length {
        let (r, shade) = (rng.range(0.008, 0.016), rng.range(0.6, 0.95));
        disc(glow, top - Y(y), VZ, r, 5, tint(color, shade));
        let (r, shade) = (rng.range(0.008, 0.016), rng.range(0.6, 0.95));
        disc(glow, top - Y(y), VX, r, 5, tint(color, shade));
        y += rng.range(0.12, 0.3);
    }
    lathe(
        glow,
        top - Y(length + 0.03),
        VY,
        &[(0.0, 0.0), (0.015, 0.018), (0.035, 0.0)],
        6,
        tint(color, 1.0),
        false,
    );
}

/// Moss carpet: a soft patch with tufts and luminous dots.
pub fn moss_carpet(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    color: C3,
    rng: &mut ArtRng,
) {
    blob(
        m,
        center,
        radius * 1.1,
        tint(MOSS, 0.7),
        rng,
        0.007,
        14,
        0.4,
    );
    blob(
        m,
        center,
        radius * 0.8,
        tint(MOSS, 0.95),
        rng,
        0.009,
        12,
        0.45,
    );
    for _ in 0..(radius * 30.0).round() as usize {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.unit64().sqrt() as f32;
        let p = center + v(a.cos() * d, 0.01, a.sin() * d);
        let h = rng.range(0.03, 0.08);
        let t = rng.range(0.0, TAU);
        let w = v(t.cos(), 0.0, t.sin()) * 0.025;
        let shade = rng.range(0.9, 1.4);
        tri(
            m,
            p - w,
            p + w,
            p + Y(h),
            norm(v(-t.sin(), 0.3, t.cos())),
            tint(MOSS, shade),
        );
        if rng.chance(0.18) {
            let r = rng.range(0.008, 0.016);
            disc(glow, p + Y(0.012), VY, r, 5, tint(color, 0.8));
        }
    }
}

/// A fallen, rotting log sprouting caps and brackets.
pub fn fallen_log(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    length: f32,
    color: C3,
    rng: &mut ArtRng,
) {
    let (f, s) = frame_of(yaw);
    let half = length * 0.5;
    let a = base - f * half + Y(0.3);
    let b = base + f * half + Y(0.26);
    let pts = [
        a,
        a.lerp(b, 0.35) + s * 0.08 + Y(0.03),
        a.lerp(b, 0.7) - s * 0.06,
        b,
    ];
    tube(m, &pts, &|t| 0.3 - t * 0.05, 10, &|t, around| {
        tint(BARK, 0.7 + 0.35 * (around * 21.0 + t * 3.0).sin().abs())
    });
    for (p, dir) in [(a, -f), (b, f)] {
        disc(m, p + dir * 0.005, dir, 0.28, 10, [0.42, 0.3, 0.18]);
        for r in [0.08, 0.16, 0.23] {
            tube_ring(m, p + dir * 0.008, dir, r, 0.006, 12, [0.3, 0.2, 0.12]);
        }
    }
    for _ in 0..4 {
        let t = rng.range(0.15, 0.85);
        let p = a.lerp(b, t);
        let side = s * if rng.chance(0.5) { 1.0 } else { -1.0 };
        let count = rng.index(2) + 1;
        let size = rng.range(0.12, 0.2);
        shelf_stack(
            m,
            glow,
            p + side * 0.26 - Y(0.08),
            side,
            count,
            size,
            rng,
            color,
        );
    }
    for _ in 0..5 {
        let p = a.lerp(b, rng.range(0.1, 0.9)) + Y(0.26);
        let h = rng.range(0.1, 0.24);
        lathe(m, p, VY, &[(0.0, 0.025), (h, 0.018)], 5, STEM, true);
        lathe(
            m,
            p + Y(h),
            VY,
            &[(0.0, h * 0.4), (h * 0.12, h * 0.36), (h * 0.25, 0.0)],
            7,
            tint(color, 0.4),
            true,
        );
        disc(
            glow,
            p + Y(h - 0.004),
            Y(-1.0),
            h * 0.36,
            7,
            tint(color, 0.5),
        );
    }
    blob(
        m,
        base,
        length * 0.62,
        tint(MOSS, 0.75),
        rng,
        0.007,
        16,
        0.3,
    );
}

// ---------------------------------------------------------------- planner

#[derive(Clone, Copy)]
enum Species {
    InkCaps,
    Bells,
    Puffballs,
    Coral,
    Cups,
}

fn species_piece(
    s: &mut Stage,
    kind: Species,
    p: Vec3,
    height: f32,
    radius: f32,
    count: usize,
    scale: f32,
    color: C3,
) {
    s.piece(move |m, g, ctx| match kind {
        Species::InkCaps => ink_caps(m, g, p, height, count, color, &mut ctx.rng),
        Species::Bells => bell_cluster(m, g, p, count, scale, color, &mut ctx.rng),
        Species::Puffballs => puffballs(m, g, p, count, radius, color, &mut ctx.rng),
        Species::Coral => coral_fungus(m, g, p, scale, color, &mut ctx.rng),
        Species::Cups => cup_fungi(m, g, p, count, color, &mut ctx.rng),
    });
}

fn moss_piece(s: &mut Stage, center: Vec3, radius: f32, color: C3) {
    s.piece(move |m, g, ctx| moss_carpet(m, g, center, radius, color, &mut ctx.rng));
}

fn shelf_piece(s: &mut Stage, p: Vec3, normal: Vec3, count: usize, size: f32, color: C3) {
    s.piece(move |m, g, ctx| shelf_stack(m, g, p, normal, count, size, &mut ctx.rng, color));
}

/// Plan the fungal grotto into the plan held by the stage.
pub fn plan(s: &mut Stage) {
    let teal = CAPS[0];
    let cap = CAPS[s.rng.index(CAPS.len())];
    // 1. The mother mushroom in the middle.
    let r_cap = 2.4f32.min(s.room.half[0].min(s.room.half[1]) * 0.42);
    let h = 3.5f32.min(s.height - 1.45);
    let heart = s.floor(s.focus);
    s.solid(heart, 0.7, false);
    s.piece(move |m, g, ctx| giant_mushroom(m, g, heart, h, r_cap, cap, &mut ctx.rng));
    s.light(heart + Y(h - 0.4), cap, 4.2, 9.0);
    s.light(heart + Y(h + 0.9), cap, 1.6, 5.0);
    // 2. Clusters of assorted fungi in a ring; mycelium ties them to the mother.
    use Species::*;
    let species = [
        InkCaps, Bells, Puffballs, Coral, Cups, Bells, InkCaps, Puffballs,
    ];
    let mut lit = 0;
    for k in 0..10 {
        let a = (k as f32 / 10.0) * TAU + s.rng.range(-0.2, 0.2);
        let d = s.rng.range(0.5, 0.8);
        let cand = s.c
            + v(
                a.cos() * s.room.half[0] * d,
                0.0,
                a.sin() * s.room.half[1] * d,
            );
        let kind = species[k % species.len()];
        let big = matches!(kind, InkCaps | Puffballs);
        let p = if big {
            s.place(&[cand], 0.4, 1.8, 0.8)
        } else {
            s.spot(cand, 0.3, 0.5)
        };
        let Some(p) = p else {
            continue;
        };
        if big {
            s.plan.solids.push((p, 0.4));
        }
        let color = CAPS[(k + 1) % CAPS.len()];
        let height = s.rng.range(0.9, 1.5);
        let radius = s.rng.range(0.18, 0.3);
        let count = s.rng.index(4) + 4;
        let scale = s.rng.range(1.0, 1.5);
        species_piece(s, kind, p, height, radius, count, scale, color);
        let r = s.rng.range(0.5, 0.8);
        moss_piece(s, p, r, color);
        let pts: Vec<Vec3> = wander(
            heart + norm(hz(p - heart)) * 1.4,
            p,
            12,
            0.4,
            &mut s.rng,
            0.0,
        )
        .into_iter()
        .map(|q| s.floor(q))
        .collect();
        let tint_col = mix_color(teal, color, 0.3);
        s.piece(move |_m, g, ctx| mycelium(g, &pts, 0.02, tint_col, &mut ctx.rng));
        if lit < 5 {
            s.light(p + Y(0.8), color, 1.5, 4.0);
            lit += 1;
        }
    }
    // 3. Brackets climb the pilasters and walls; caps at their feet.
    for pilaster in s.pilasters.clone() {
        if !s.rng.chance(0.75) {
            continue;
        }
        let face = pilaster.base + pilaster.inward * 0.22;
        let mut k = 0;
        loop {
            // The prototype re-rolls this bound every pass of its loop.
            let bound = 1 + s.rng.index(2);
            if k >= bound {
                break;
            }
            let side = pilaster.tangent * s.rng.range(-0.25, 0.25);
            let p = face + side + Y(s.rng.range(0.4, 2.2));
            let count = 2 + s.rng.index(3);
            let size = s.rng.range(0.15, 0.24);
            let color = CAPS[s.rng.index(CAPS.len())];
            shelf_piece(s, p, pilaster.inward, count, size, color);
            k += 1;
        }
        if let Some(foot) = s.spot(face + pilaster.inward * 0.35, 0.2, 0.3) {
            let kind = if s.rng.chance(0.5) { Cups } else { Bells };
            let count = s.rng.index(4) + 3;
            let color = CAPS[s.rng.index(CAPS.len())];
            species_piece(s, kind, foot, 0.0, 0.0, count, 0.8, color);
        }
    }
    for bay in s.free_bays(None, 1.4).into_iter().take(4) {
        s.take_bay(&bay);
        for _ in 0..3 {
            let along = s.rng.range(-0.6, 0.6);
            let up = s.rng.range(0.5, 2.4);
            let p = bay.center + bay.tangent * along + bay.inward * 0.02 + Y(up);
            let count = 2 + s.rng.index(2);
            let size = s.rng.range(0.14, 0.22);
            let color = CAPS[s.rng.index(CAPS.len())];
            shelf_piece(s, p, bay.inward, count, size, color);
        }
        if let Some(foot) = s.spot(bay.center + bay.inward * 0.5, 0.3, 0.3) {
            let yaw = s.rng.range(0.0, TAU);
            let h = s.rng.range(0.5, 0.8);
            s.prop("nat_fern", foot, yaw, h, 0.0);
        }
    }
    // 4. A fallen log along a wall, overgrown.
    let mut walls = s.walls.clone();
    s.rng.shuffle(&mut walls);
    for wall in &walls {
        let yaw = yaw_of(wall.tangent);
        let base = wall.mid + wall.inward * 1.1;
        let Some(p) = s.place_row(
            &[base, base + wall.tangent * 1.5, base - wall.tangent * 1.5],
            yaw,
            1.5,
            0.36,
            1.2,
            0.6,
        ) else {
            continue;
        };
        let color = CAPS[s.rng.index(CAPS.len())];
        s.piece(move |m, g, ctx| fallen_log(m, g, p, yaw, 3.0, color, &mut ctx.rng));
        s.prop(
            "nat_laetiporus",
            p + wall.inward * 0.35 + Y(0.1),
            yaw,
            0.4,
            0.0,
        );
        break;
    }
    // 5. Nature props: mushrooms, ferns, plants and mossy rocks by the walls.
    let ring = |a: f32, b: f32, walk: f32| Scatter {
        walkway: walk,
        ring: [a, b],
        ..Scatter::default()
    };
    s.scatter(10, 0.3, ring(0.62, 1.0, 0.5), &mut |s, p, _| {
        let pick = s.rng.index(5);
        let (asset, h) = [
            ("nat_mushroom", 0.3),
            ("nat_fern", 0.55),
            ("nat_fern", 0.4),
            ("nat_clover2", 0.18),
            ("nat_mushroom", 0.22),
        ][pick];
        let yaw = s.rng.range(0.0, TAU);
        let k = s.rng.range(0.8, 1.25);
        s.prop(asset, p, yaw, h * k, 0.0);
    });
    s.scatter(4, 0.45, ring(0.6, 0.95, 1.0), &mut |s, p, _| {
        let asset = ["nat_rock1", "nat_rock2", "nat_rock3"][s.rng.index(3)];
        let yaw = s.rng.range(0.0, TAU);
        let h = s.rng.range(0.35, 0.6);
        s.prop(asset, p, yaw, h, 0.4);
        s.solid(p, 0.35, false);
        moss_piece(s, p, 0.55, teal);
    });
    // 6. Moss, puddles, glow-worm threads, spores.
    let floor_only = Scatter {
        walkway: 0.0,
        ..Scatter::default()
    };
    s.scatter(14, 0.5, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.35, 0.8);
        let color = CAPS[s.rng.index(CAPS.len())];
        moss_piece(s, p, r, color);
    });
    s.scatter(5, 0.4, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.3, 0.6);
        s.puddle(p, r, Some([0.04, 0.08, 0.09]), Some(tint(teal, 0.25)));
    });
    for _ in 0..34 {
        let (a, b) = (s.rng.range(-0.85, 0.85), s.rng.range(-0.85, 0.85));
        let p = s.at(a * s.depth, b * s.width);
        if !s.inside(p, 0.3) {
            continue;
        }
        if hdist(p, heart) < r_cap + 0.2 {
            continue;
        }
        let top = s.top(p) - Y(0.2);
        let length = s.rng.range(0.8, (s.height - 2.6).max(1.0));
        let color = CAPS[s.rng.index(3)];
        s.piece(move |m, g, ctx| glow_threads(m, g, top, length, color, &mut ctx.rng));
    }
    s.plan.sparkle_tint = Some(tint(cap, 1.4));
    let hh = s.height;
    s.sparkles(26, 0.4, hh - 1.0, None, None);
    s.sparkles(18, h - 0.8, h - 0.1, Some(heart), Some(r_cap * 0.8));
}
