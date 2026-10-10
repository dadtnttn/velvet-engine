//! La Biblioteca hundida (web prototype `galleries/library.js`): a reading
//! hall half swallowed by the cave — bookcases in every bay, ladders, a
//! brass orrery, reading tables buried in books, lecterns with floating
//! runes, a globe, book towers, rugs, and a corner sunk into black water.
use super::kit::*;
use super::plan::Pond;
use super::stage::*;
use super::themekit::skull;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

const SHELF_WOOD: C3 = [0.3, 0.17, 0.09];
const LEATHER: [C3; 8] = [
    [0.42, 0.08, 0.07],
    [0.12, 0.26, 0.16],
    [0.1, 0.14, 0.34],
    [0.36, 0.2, 0.08],
    [0.3, 0.12, 0.4],
    [0.52, 0.4, 0.18],
    [0.2, 0.12, 0.08],
    [0.46, 0.3, 0.2],
];
const PAPER: C3 = [0.86, 0.8, 0.64];
const BRASS: C3 = [0.72, 0.52, 0.2];
const WATER: C3 = [0.04, 0.07, 0.08];
pub const RUNE: C3 = [0.5, 0.8, 1.0];
const PI: f32 = std::f32::consts::PI;

fn leather(rng: &mut ArtRng) -> C3 {
    LEATHER[rng.index(LEATHER.len())]
}

/// One shelf row of books from x0 to x1 at height y (local frame).
#[allow(clippy::too_many_arguments)]
fn book_row(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    x0: f32,
    x1: f32,
    y: f32,
    depth: f32,
    max_h: f32,
    rng: &mut ArtRng,
) {
    let mut x = x0;
    while x < x1 - 0.05 {
        let roll = rng.unit64();
        if roll < 0.08 {
            // A lying stack.
            let n = 2 + rng.index(4);
            let w = rng.range(0.16, 0.24);
            for k in 0..n {
                let c = v(
                    x + w / 2.0 + rng.range(-0.01, 0.01),
                    y + 0.02 + k as f32 * 0.035,
                    depth * 0.55,
                );
                let base = leather(rng);
                let shade = rng.range(0.7, 1.1);
                kit_block(
                    m,
                    c,
                    VX,
                    VY,
                    VZ,
                    v(w / 2.0, 0.016, depth * 0.36),
                    tint(base, shade),
                );
            }
            x += w + 0.02;
            continue;
        }
        if roll < 0.13 {
            // An object among the books: skull, candle, jar or scroll bundle.
            let kind = rng.index(4);
            let c = v(x + 0.09, y, depth * 0.5);
            match kind {
                0 => {
                    let yaw = rng.range(-0.4, 0.4);
                    skull(m, c + Y(0.075), 0.7, yaw, 0.0, [0.8, 0.74, 0.62]);
                }
                1 => {
                    let h = rng.range(0.08, 0.2);
                    wax_candle(m, glow, c, 0.03, h, rng, WAX, true);
                }
                2 => lathe(
                    m,
                    c,
                    VY,
                    &[
                        (0.0, 0.05),
                        (0.03, 0.06),
                        (0.12, 0.055),
                        (0.14, 0.03),
                        (0.17, 0.035),
                    ],
                    8,
                    tint([0.3, 0.46, 0.4], 0.8),
                    true,
                ),
                _ => {
                    for s in 0..3 {
                        lathe(
                            m,
                            c + v(0.0, 0.03 + s as f32 * 0.05, 0.0) - VZ * (depth * 0.35),
                            VZ,
                            &[(0.0, 0.025), (depth * 0.7, 0.025)],
                            6,
                            tint(PAPER, 0.85),
                            true,
                        );
                    }
                }
            }
            x += 0.2;
            continue;
        }
        let w = rng.range(0.03, 0.075);
        let h = max_h.min(rng.range(0.2, 0.36));
        let base = leather(rng);
        let color = tint(base, rng.range(0.7, 1.15));
        let lean = if rng.chance(0.12) {
            rng.range(-0.28, 0.28)
        } else {
            0.0
        };
        let up = VY * lean.cos() + VX * lean.sin();
        let c = v(
            x + w / 2.0 + if lean > 0.0 { h * 0.2 } else { 0.0 },
            y + h / 2.0,
            depth * 0.55,
        );
        kit_block(
            m,
            c,
            VX * lean.cos() - VY * lean.sin(),
            up,
            VZ,
            v(w / 2.0, h / 2.0, depth * 0.38),
            color,
        );
        if rng.chance(0.55) {
            for t in [0.22, 0.78] {
                kit_block(
                    m,
                    c + up * (h * (t - 0.5)) + VZ * (depth * 0.385),
                    VX,
                    up,
                    VZ,
                    v(w / 2.0 + 0.002, 0.006, 0.003),
                    tint(GOLD, 0.8),
                );
            }
        }
        x += w + if lean != 0.0 { h * 0.35 } else { 0.006 };
    }
}

/// Where a bookcase stands: its wall frame and an optional topple.
#[derive(Clone, Copy)]
pub struct CaseFrame {
    pub center: Vec3,
    pub inward: Vec3,
    pub tangent: Vec3,
    /// (angle, lift, roll) of a fallen case.
    pub tilt: Option<(f32, f32, f32)>,
}

fn case_mapper(frame: CaseFrame) -> Mapper {
    let CaseFrame {
        center,
        inward,
        tangent,
        tilt,
    } = frame;
    let to_world = move |p: Vec3| center + tangent * p.x + Y(p.y) + inward * p.z;
    let to_world_n = move |n: Vec3| tangent * n.x + Y(n.y) + inward * n.z;
    match tilt {
        None => Mapper {
            point: Box::new(to_world),
            normal: Box::new(to_world_n),
        },
        Some((angle, lift, roll)) => {
            let (ca, sa) = (angle.cos(), angle.sin());
            let (cr, sr) = (roll.cos(), roll.sin());
            let rot = move |p: Vec3| {
                let y = p.y * ca - p.z * sa;
                let z = p.y * sa + p.z * ca;
                v(p.x * cr - y * sr, p.x * sr + y * cr, z)
            };
            Mapper {
                point: Box::new(move |p| to_world(rot(p) + Y(lift))),
                normal: Box::new(move |n| to_world_n(rot(n))),
            }
        }
    }
}

/// A grand bookcase built into a wall bay: plinth, carved side panels,
/// crown moulding, shelves crammed with books and the odd curiosity.
pub fn grand_bookcase(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    width: f32,
    height: f32,
    sparse: bool,
    frame: CaseFrame,
    rng: &mut ArtRng,
) {
    let mut local = empty();
    let mut lglow = empty();
    let w = width * 0.5;
    let h = height;
    let d = 0.42;
    let wood = tint(SHELF_WOOD, rng.range(0.9, 1.1));
    chamfer_box(
        &mut local,
        v(0.0, h * 0.5, 0.02),
        VX,
        VY,
        VZ,
        v(w, h * 0.5, 0.02),
        0.005,
        tint(wood, 0.5),
    );
    for sgn in [-1.0, 1.0] {
        chamfer_box(
            &mut local,
            v(sgn * (w + 0.05), h * 0.5, d * 0.5),
            VX,
            VY,
            VZ,
            v(0.06, h * 0.5, d * 0.5 + 0.02),
            0.012,
            tint(wood, 0.95),
        );
        for y in [h * 0.3, h * 0.7] {
            kit_block(
                &mut local,
                v(sgn * (w + 0.05), y, d + 0.024),
                VX,
                VY,
                VZ,
                v(0.035, h * 0.16, 0.006),
                tint(wood, 0.75),
            );
        }
    }
    chamfer_box(
        &mut local,
        v(0.0, 0.08, d * 0.5 + 0.01),
        VX,
        VY,
        VZ,
        v(w + 0.1, 0.08, d * 0.5 + 0.04),
        0.012,
        tint(wood, 0.8),
    );
    chamfer_box(
        &mut local,
        v(0.0, h + 0.05, d * 0.5 + 0.02),
        VX,
        VY,
        VZ,
        v(w + 0.14, 0.05, d * 0.5 + 0.06),
        0.012,
        tint(wood, 1.1),
    );
    chamfer_box(
        &mut local,
        v(0.0, h + 0.13, d * 0.5 + 0.03),
        VX,
        VY,
        VZ,
        v(w + 0.18, 0.035, d * 0.5 + 0.08),
        0.01,
        tint(wood, 1.2),
    );
    kit_block(
        &mut local,
        v(0.0, h + 0.2, d * 0.5 + 0.02),
        VX,
        VY,
        VZ,
        v(0.18, 0.05, 0.02),
        tint(GOLD, 0.7),
    );
    let shelves = (((h - 0.2) / 0.46).round() as usize).max(4);
    let step = (h - 0.18) / shelves as f32;
    for k in 0..shelves {
        let y = 0.16 + k as f32 * step;
        kit_block(
            &mut local,
            v(0.0, y, d * 0.5),
            VX,
            VY,
            VZ,
            v(w, 0.018, d * 0.5),
            tint(wood, 1.05),
        );
        kit_block(
            &mut local,
            v(0.0, y - 0.004, d + 0.004),
            VX,
            VY,
            VZ,
            v(w, 0.022, 0.006),
            tint(wood, 1.25),
        );
        if sparse && rng.chance(0.4) {
            continue;
        }
        book_row(
            &mut local,
            &mut lglow,
            -w + 0.03,
            w - 0.03,
            y + 0.018,
            d,
            step - 0.06,
            rng,
        );
    }
    let map = case_mapper(frame);
    append(m, &local, &map, [1.0; 3]);
    append(glow, &lglow, &map, [1.0; 3]);
}

/// A library ladder leaning against a bookcase.
pub fn ladder(m: &mut Mesh3D, foot: Vec3, top: Vec3, inward: Vec3) {
    let up = norm(top - foot);
    let side = norm(up.cross(inward));
    for sgn in [-1.0, 1.0] {
        bar(
            m,
            foot + side * (sgn * 0.22),
            top + side * (sgn * 0.2),
            0.025,
            0.018,
            inward,
            tint(WOOD, 1.15),
        );
    }
    let len = (top - foot).length();
    let mut d = 0.25;
    while d < len - 0.1 {
        bar(
            m,
            foot + up * d - side * 0.21,
            foot + up * d + side * 0.21,
            0.014,
            0.014,
            up,
            tint(WOOD, 1.0),
        );
        d += 0.3;
    }
    for sgn in [-1.0, 1.0] {
        tube_ring(
            m,
            top + side * (sgn * 0.2) + inward * -0.03,
            side,
            0.04,
            0.01,
            10,
            tint(BRASS, 1.1),
        );
    }
}

/// Cubby rack of rolled scrolls.
pub fn scroll_rack(
    m: &mut Mesh3D,
    center: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    height: f32,
    rng: &mut ArtRng,
) {
    let w = width * 0.5;
    let h = height;
    let l = |x: f32, y: f32, z: f32| center + tangent * x + Y(y) + inward * z;
    chamfer_box(
        m,
        l(0.0, h * 0.5, 0.2),
        tangent,
        VY,
        inward,
        v(w + 0.05, h * 0.5, 0.2),
        0.01,
        tint(SHELF_WOOD, 0.6),
    );
    let cols = ((w * 2.0 / 0.3).round() as usize).max(3);
    let rows = ((h / 0.3).round() as usize).max(4);
    let cw = (w * 2.0) / cols as f32;
    let rh = h / rows as f32;
    for r in 0..rows {
        for c in 0..cols {
            let cc = l(-w + cw * (c as f32 + 0.5), rh * (r as f32 + 0.5), 0.405);
            quad(
                m,
                [
                    cc + tangent * (-cw * 0.44) - Y(rh * 0.44),
                    cc + tangent * (cw * 0.44) - Y(rh * 0.44),
                    cc + tangent * (cw * 0.44) + Y(rh * 0.44),
                    cc + tangent * (-cw * 0.44) + Y(rh * 0.44),
                ],
                inward,
                [0.06, 0.04, 0.03],
            );
            let n = rng.index(4);
            for k in 0..n {
                let p = cc
                    + tangent * rng.range(-cw * 0.25, cw * 0.25)
                    + Y(-rh * 0.3 + k as f32 * 0.05);
                let shade = rng.range(0.75, 1.0);
                lathe(
                    m,
                    p - inward * 0.3,
                    inward,
                    &[(0.0, 0.03), (0.4, 0.03)],
                    7,
                    tint(PAPER, shade),
                    true,
                );
                tube_ring(
                    m,
                    p + inward * 0.02,
                    inward,
                    0.031,
                    0.006,
                    8,
                    [0.6, 0.1, 0.08],
                );
            }
        }
    }
    for c in 0..=cols {
        kit_block(
            m,
            l(-w + cw * c as f32, h * 0.5, 0.39),
            tangent,
            VY,
            inward,
            v(0.012, h * 0.5, 0.02),
            tint(SHELF_WOOD, 1.1),
        );
    }
    for r in 0..=rows {
        kit_block(
            m,
            l(0.0, rh * r as f32, 0.39),
            tangent,
            VY,
            inward,
            v(w, 0.012, 0.02),
            tint(SHELF_WOOD, 1.1),
        );
    }
}

/// An open book: two curved page blocks between leather covers.
pub fn open_book(m: &mut Mesh3D, base: Vec3, yaw: f32, size: f32, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * (x * size) + Y(y * size) + f * (z * size);
    let cover = leather(rng);
    for sgn in [-1.0, 1.0] {
        quad(
            m,
            [
                l(0.0, 0.006, -0.16),
                l(sgn * 0.25, 0.006, -0.16),
                l(sgn * 0.25, 0.006, 0.16),
                l(0.0, 0.006, 0.16),
            ],
            VY,
            tint(cover, 0.8),
        );
        grid_surface(
            m,
            3,
            5,
            &|i, j| {
                let x = sgn * (0.02 + (j as f32 / 5.0) * 0.22);
                let lift = 0.03 * ((j as f32 / 5.0) * PI * 0.9 + 0.2).sin() + 0.012;
                l(x, lift, -0.15 + (i as f32 / 3.0) * 0.3)
            },
            &|_, j| tint(PAPER, 0.85 + 0.12 * (j as f32 / 5.0)),
            false,
            Some(&|_, _, _| VY),
            None,
        );
        for k in 0..6 {
            let z = -0.1 + k as f32 * 0.04;
            kit_block(
                m,
                l(sgn * 0.13, 0.046, z),
                s,
                VY,
                f,
                v(0.07 * size, 0.001, 0.004 * size),
                [0.25, 0.2, 0.16],
            );
        }
    }
}

/// Table-top clutter: open books, stacks, inkwell and quill, candles, scrolls.
#[allow(clippy::too_many_arguments)]
pub fn desk_clutter(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    hw: f32,
    hd: f32,
    rng: &mut ArtRng,
) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, z: f32| base + s * x + f * z;
    for _ in 0..2 {
        let p = l(
            rng.range(-hw * 0.6, hw * 0.6),
            rng.range(-hd * 0.4, hd * 0.4),
        );
        let turn = yaw + rng.range(-0.4, 0.4);
        let size = rng.range(0.9, 1.2);
        open_book(m, p, turn, size, rng);
    }
    for _ in 0..3 {
        let c = l(
            rng.range(-hw * 0.85, hw * 0.85),
            rng.range(-hd * 0.6, hd * 0.6),
        );
        let n = 2 + rng.index(4);
        for b in 0..n {
            let byaw = yaw + rng.range(-0.4, 0.4);
            let bf = v(byaw.sin(), 0.0, byaw.cos());
            let half = v(rng.range(0.1, 0.14), 0.02, rng.range(0.14, 0.18));
            let base = leather(rng);
            let shade = rng.range(0.7, 1.1);
            kit_block(
                m,
                c + Y(0.022 + b as f32 * 0.042),
                v(bf.z, 0.0, -bf.x),
                VY,
                bf,
                half,
                tint(base, shade),
            );
        }
    }
    let ink = l(hw * 0.4, hd * 0.5);
    lathe(
        m,
        ink,
        VY,
        &[(0.0, 0.035), (0.04, 0.04), (0.06, 0.02), (0.075, 0.022)],
        8,
        [0.08, 0.1, 0.14],
        true,
    );
    let tip = ink + Y(0.05);
    tube(
        m,
        &[tip, tip + v(0.04, 0.12, 0.02), tip + v(0.1, 0.26, 0.05)],
        &|t| 0.004 + t * 0.012,
        4,
        &|_, _| [0.9, 0.9, 0.86],
    );
    for _ in 0..2 {
        let p = l(
            rng.range(-hw * 0.9, hw * 0.9),
            rng.range(-hd * 0.8, hd * 0.8),
        );
        let r = rng.range(0.028, 0.04);
        let h = rng.range(0.08, 0.22);
        wax_candle(m, glow, p, r, h, rng, WAX, true);
    }
    for _ in 0..2 {
        let c = l(
            rng.range(-hw * 0.7, hw * 0.7),
            rng.range(-hd * 0.7, hd * 0.7),
        ) + Y(0.028);
        let (a1, a2) = (rng.range(0.0, TAU), rng.range(0.0, TAU));
        let d = v(a1.cos(), 0.0, a2.sin());
        lathe(
            m,
            c - d * 0.2,
            d,
            &[(0.0, 0.026), (0.4, 0.026)],
            7,
            tint(PAPER, 0.9),
            true,
        );
    }
    if rng.chance(0.5) {
        let yaw = rng.range(0.0, TAU);
        skull(
            m,
            l(-hw * 0.75, hd * 0.4) + Y(0.07),
            0.7,
            yaw,
            0.0,
            [0.8, 0.74, 0.62],
        );
    }
    for k in 0..6 {
        let c = l(rng.range(-hw, hw), rng.range(-hd, hd)) + Y(0.003 + k as f32 * 0.0005);
        let t = rng.range(0.0, TAU);
        let u = v(t.cos(), 0.0, t.sin()) * 0.1;
        let w = v(-t.sin(), 0.0, t.cos()) * 0.14;
        quad(
            m,
            [c - u - w, c + u - w, c + u + w, c - u + w],
            VY,
            tint(PAPER, rng.range(0.8, 1.0)),
        );
    }
}

/// A lectern with a great open tome; its runes rise glowing into the air.
pub fn lectern_tome(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut ArtRng) {
    let (f, s) = frame_of(yaw);
    let l = |x: f32, y: f32, z: f32| base + s * x + Y(y) + f * z;
    chamfer_box(
        m,
        l(0.0, 0.05, 0.0),
        s,
        VY,
        f,
        v(0.3, 0.05, 0.3),
        0.015,
        tint(DARK_WOOD, 1.1),
    );
    lathe(
        m,
        l(0.0, 0.1, 0.0),
        VY,
        &[
            (0.0, 0.1),
            (0.08, 0.07),
            (0.6, 0.06),
            (0.7, 0.09),
            (0.95, 0.08),
        ],
        10,
        tint(DARK_WOOD, 1.2),
        false,
    );
    let desk_c = l(0.0, 1.12, 0.02);
    let tilt_f = f * 0.4f32.cos() + Y(-0.4f32.sin());
    let up = f * 0.4f32.sin() + Y(0.4f32.cos());
    chamfer_box(
        m,
        desk_c,
        s,
        up,
        tilt_f,
        v(0.36, 0.03, 0.26),
        0.01,
        tint(DARK_WOOD, 1.25),
    );
    let mut tome = empty();
    open_book(&mut tome, Vec3::ZERO, 0.0, 1.5, rng);
    let to_book = Mapper {
        point: Box::new(move |p| desk_c + up * 0.035 + s * p.x + up * p.y + tilt_f * p.z),
        normal: Box::new(move |n| s * n.x + up * n.y + tilt_f * n.z),
    };
    append(m, &tome, &to_book, [1.0; 3]);
    for k in 0..16 {
        let a = k as f32 * 0.9;
        let r = 0.12 + k as f32 * 0.012;
        let c = desk_c + Y(0.2 + k as f32 * 0.07) + s * (a.cos() * r) + f * (a.sin() * r);
        let n = norm(c - (desk_c + Y(0.2 + k as f32 * 0.07)));
        let g = 0.03 + (k % 3) as f32 * 0.008;
        let t = v(-n.z, 0.0, n.x);
        let q = [c - t * g, c + Y(g), c + t * g, c - Y(g)];
        quad(glow, q, n, tint(RUNE, 0.7 - k as f32 * 0.025));
        quad(glow, q, -n, tint(RUNE, 0.55 - k as f32 * 0.02));
    }
}

/// A terrestrial globe on a turned stand with a brass meridian.
pub fn globe(m: &mut Mesh3D, b: Vec3, rng: &mut ArtRng) {
    for k in 0..3 {
        let a = k as f32 * TAU / 3.0;
        let d = v(a.cos(), 0.0, a.sin());
        tube_c(
            m,
            &bezier(b + Y(0.4), b + d * 0.2 + Y(0.25), b + d * 0.28, 6),
            0.02,
            5,
            tint(DARK_WOOD, 1.2),
        );
    }
    lathe(
        m,
        b + Y(0.38),
        VY,
        &[(0.0, 0.04), (0.14, 0.03), (0.2, 0.05)],
        8,
        tint(DARK_WOOD, 1.2),
        false,
    );
    let c = b + Y(0.85);
    let axis = norm(v(0.3, 1.0, 0.0));
    lathe_fn(
        m,
        c - axis * 0.3,
        axis,
        &[
            (0.0, 0.0),
            (0.04, 0.17),
            (0.12, 0.27),
            (0.3, 0.3),
            (0.48, 0.27),
            (0.56, 0.17),
            (0.6, 0.0),
        ],
        16,
        &|j| mix_color([0.62, 0.56, 0.36], [0.3, 0.42, 0.5], (j % 2) as f32 * 0.6),
        true,
    );
    for _ in 0..7 {
        let t = rng.range(0.0, TAU);
        let h = rng.range(-0.18, 0.2);
        let r = (0.3f32 * 0.3 - h * h).sqrt();
        let p = c + axis * h + v(t.cos() * r, 0.0, t.sin() * r);
        let n = norm(p - c);
        let size = rng.range(0.05, 0.1);
        disc(m, p + n * 0.003, n, size, 6, [0.36, 0.46, 0.26]);
    }
    tube_ring(
        m,
        c,
        norm(v(-axis.y, axis.x, 0.0)),
        0.33,
        0.012,
        24,
        tint(BRASS, 1.1),
    );
    tube_ring(m, c - Y(0.02), VY, 0.35, 0.02, 24, tint(DARK_WOOD, 1.2));
}

/// A leaning tower of books on the floor.
pub fn book_tower(m: &mut Mesh3D, base: Vec3, count: usize, rng: &mut ArtRng) {
    let mut p = base;
    let mut yaw = rng.range(0.0, TAU);
    for _ in 0..count {
        let bw = rng.range(0.13, 0.19);
        let bd = rng.range(0.17, 0.24);
        let h = rng.range(0.035, 0.07);
        yaw += rng.range(-0.3, 0.3);
        let f = v(yaw.sin(), 0.0, yaw.cos());
        let s = v(f.z, 0.0, -f.x);
        let cover = leather(rng);
        let shade = rng.range(0.7, 1.1);
        kit_block(
            m,
            p + Y(h / 2.0),
            s,
            VY,
            f,
            v(bw, h / 2.0, bd),
            tint(cover, shade),
        );
        kit_block(
            m,
            p + Y(h / 2.0) + f * 0.004,
            s,
            VY,
            f,
            v(bw - 0.012, h / 2.0 - 0.006, bd - 0.004 + 0.006),
            tint(PAPER, 0.85),
        );
        let (dx, dz) = (rng.range(-0.015, 0.015), rng.range(-0.015, 0.015));
        p = p + Y(h) + v(dx, 0.0, dz);
    }
}

/// Loose pages scattered over the floor.
pub fn loose_pages(m: &mut Mesh3D, center: Vec3, radius: f32, count: usize, rng: &mut ArtRng) {
    for k in 0..count {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.unit64().sqrt() as f32;
        let c = center + v(a.cos() * d, 0.006 + k as f32 * 0.0003, a.sin() * d);
        let t = rng.range(0.0, TAU);
        let u = v(t.cos(), rng.range(0.0, 0.08), t.sin()) * 0.075;
        let w = v(-t.sin(), rng.range(0.0, 0.05), t.cos()) * 0.105;
        quad(
            m,
            [c - u - w, c + u - w, c + u + w, c - u + w],
            VY,
            tint(PAPER, rng.range(0.5, 0.72)),
        );
        if rng.chance(0.5) {
            for line in 0..3 {
                let o0 = -0.5 + line as f32 * 0.3;
                let o1 = -0.47 + line as f32 * 0.3;
                quad(
                    m,
                    [
                        c - u * 0.7 + w * o0 + Y(0.001),
                        c + u * 0.7 + w * o0 + Y(0.001),
                        c + u * 0.7 + w * o1 + Y(0.001),
                        c - u * 0.7 + w * o1 + Y(0.001),
                    ],
                    VY,
                    [0.3, 0.25, 0.2],
                );
            }
        }
    }
}

/// A faded rug with a border and a medallion.
pub fn library_rug(m: &mut Mesh3D, center: Vec3, yaw: f32, hx: f32, hz: f32) {
    let (f, s) = frame_of(yaw);
    let c = center + Y(0.012);
    let rect = |m: &mut Mesh3D, y: f32, ax: f32, az: f32, col: C3| {
        quad(
            m,
            [
                c + s * -ax + f * -az + Y(y),
                c + s * ax + f * -az + Y(y),
                c + s * ax + f * az + Y(y),
                c + s * -ax + f * az + Y(y),
            ],
            VY,
            col,
        );
    };
    let base = [0.14, 0.2, 0.3];
    rect(m, 0.0, hx, hz, tint(base, 0.9));
    rect(m, 0.001, hx - 0.12, hz - 0.12, tint([0.5, 0.36, 0.18], 0.9));
    rect(m, 0.002, hx - 0.18, hz - 0.18, tint(base, 1.1));
    for k in 0..16 {
        let a = (k as f32 / 16.0) * TAU;
        let p =
            c + s * (a.cos() * hx.min(hz) * 0.45) + f * (a.sin() * hx.min(hz) * 0.45) + Y(0.003);
        disc(m, p, VY, 0.06, 6, tint([0.6, 0.44, 0.2], 0.9));
    }
    disc(
        m,
        c + Y(0.003),
        VY,
        hx.min(hz) * 0.3,
        16,
        tint([0.46, 0.12, 0.1], 0.9),
    );
    for sgn in [-1.0, 1.0] {
        for k in 0..(hx * 2.0 / 0.07).floor() as usize {
            let x = -hx + 0.035 + k as f32 * 0.07;
            let p0 = c + s * x + f * (sgn * hz);
            quad(
                m,
                [
                    p0 - s * 0.015,
                    p0 + s * 0.015,
                    p0 + s * 0.015 + f * (sgn * 0.08),
                    p0 - s * 0.015 + f * (sgn * 0.08),
                ],
                VY,
                [0.82, 0.74, 0.54],
            );
        }
    }
}

/// Where the floor gave way: black water, broken slabs, floating books.
pub fn sunken_pool(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    r: f32,
    stone: C3,
    rng: &mut ArtRng,
    wet: &mut Vec<Pond>,
) {
    puddle(wet, center, r, rng, WATER, Some([0.06, 0.12, 0.14]), 0.95);
    for k in 0..16 {
        let a = (k as f32 / 16.0) * TAU + rng.range(-0.1, 0.1);
        let d = r * rng.range(0.95, 1.12);
        let p = center + v(a.cos() * d, 0.02, a.sin() * d);
        let out = v(a.cos(), 0.0, a.sin());
        let tilt_axis = v(-out.z, 0.0, out.x);
        let n = norm(VY + out * -rng.range(0.2, 0.6));
        let w = tilt_axis * rng.range(0.16, 0.26);
        let dd = norm(n.cross(w)) * rng.range(0.14, 0.22);
        quad(
            m,
            [p - w - dd, p + w - dd, p + w + dd, p - w + dd],
            n,
            tint(stone, rng.range(0.6, 0.85)),
        );
    }
    for _ in 0..7 {
        let a = rng.range(0.0, TAU);
        let d = r * 0.8 * rng.unit64().sqrt() as f32;
        let c = center + v(a.cos() * d, 0.02, a.sin() * d);
        if rng.chance(0.5) {
            let yaw = rng.range(0.0, TAU);
            let bf = v(yaw.sin(), 0.0, yaw.cos());
            let cover = leather(rng);
            kit_block(
                m,
                c,
                v(bf.z, 0.0, -bf.x),
                VY,
                bf,
                v(0.12, 0.02, 0.16),
                tint(cover, 0.6),
            );
        } else {
            let t = rng.range(0.0, TAU);
            let u = v(t.cos(), 0.0, t.sin()) * 0.1;
            let w = v(-t.sin(), 0.0, t.cos()) * 0.14;
            quad(
                m,
                [c - u - w, c + u - w, c + u + w, c - u + w],
                VY,
                tint(PAPER, 0.55),
            );
        }
    }
    for rr in [0.35, 0.6] {
        tube_ring(
            glow,
            center + Y(0.012),
            VY,
            r * rr,
            0.004,
            28,
            [0.08, 0.16, 0.18],
        );
    }
}

/// The static base of the orrery: a stepped pedestal and a meridian arc.
pub fn orrery_base(m: &mut Mesh3D, glow: &mut Mesh3D, b: Vec3, stone: C3) {
    lathe_fn(
        m,
        b,
        VY,
        &[
            (0.0, 0.8),
            (0.12, 0.78),
            (0.14, 0.62),
            (0.26, 0.6),
            (0.28, 0.4),
            (0.5, 0.3),
            (0.62, 0.34),
            (0.7, 0.4),
            (0.74, 0.4),
        ],
        18,
        &|j| tint(stone, 0.8 + j as f32 * 0.04),
        true,
    );
    tube_ring(m, b + Y(0.72), VY, 0.41, 0.02, 24, tint(BRASS, 1.1));
    for k in 0..12 {
        let a = (k as f32 / 12.0) * TAU;
        let p = b + v(a.cos() * 0.7, 0.13, a.sin() * 0.7);
        disc(
            glow,
            p + v(a.cos(), 0.0, a.sin()) * 0.005,
            norm(v(a.cos(), 0.4, a.sin())),
            0.03,
            5,
            tint(GOLD, 0.6),
        );
    }
    let top = b + Y(0.74);
    bar(m, top, top + Y(0.5), 0.03, 0.03, VX, tint(BRASS, 1.2));
}

/// One of the orrery's turning brass rings, modelled about its centre.
pub struct OrreryRing {
    pub mesh: Mesh3D,
    pub tilt: f32,
    pub speed: f32,
}

/// The orrery's moving parts: brass rings (spun by the scene) and a sun.
pub fn orrery_parts() -> (Vec<OrreryRing>, Mesh3D) {
    let specs = [
        (0.72, 0.02, 0.35, 0.18),
        (0.58, 0.016, -0.6, -0.26),
        (0.44, 0.014, 1.1, 0.34),
        (0.86, 0.012, 0.1, -0.12),
    ];
    let planets = [
        [0.5, 0.3, 0.2],
        [0.3, 0.4, 0.6],
        [0.6, 0.55, 0.4],
        [0.4, 0.5, 0.36],
    ];
    let mut rings = vec![];
    for (k, &(r, t, tilt, speed)) in specs.iter().enumerate() {
        let mut mesh = empty();
        tube_ring(
            &mut mesh,
            Vec3::ZERO,
            VY,
            r,
            t,
            40,
            tint(BRASS, 1.0 + k as f32 * 0.08),
        );
        for s in 0..8 {
            let a = (s as f32 / 8.0) * TAU;
            disc(
                &mut mesh,
                v(a.cos() * r, t + 0.002, a.sin() * r),
                VY,
                t * 1.5,
                5,
                tint(BRASS, 1.3),
            );
        }
        let planet = v(r, 0.0, 0.0);
        lathe(
            &mut mesh,
            planet - Y(0.06),
            VY,
            &[
                (0.0, 0.0),
                (0.02, 0.045),
                (0.06, 0.06),
                (0.1, 0.045),
                (0.12, 0.0),
            ],
            10,
            planets[k],
            true,
        );
        if k == 1 {
            tube_ring(
                &mut mesh,
                planet,
                norm(v(0.3, 1.0, 0.0)),
                0.1,
                0.006,
                16,
                tint(BRASS, 1.3),
            );
        }
        rings.push(OrreryRing { mesh, tilt, speed });
    }
    let mut sun = empty();
    lathe(
        &mut sun,
        v(0.0, -0.14, 0.0),
        VY,
        &[
            (0.0, 0.0),
            (0.03, 0.08),
            (0.1, 0.13),
            (0.18, 0.13),
            (0.25, 0.08),
            (0.28, 0.0),
        ],
        12,
        [1.0, 0.72, 0.3],
        true,
    );
    for k in 0..12 {
        let a = (k as f32 / 12.0) * TAU;
        let d = norm(v(a.cos(), (a * 2.0).sin() * 0.3, a.sin()));
        lathe(
            &mut sun,
            d * 0.12,
            d,
            &[(0.0, 0.03), (0.12, 0.0)],
            4,
            [1.0, 0.6, 0.2],
            true,
        );
    }
    (rings, sun)
}

// ---------------------------------------------------------------- planner

/// Plan the sunken library into the plan held by the stage.
pub fn plan(s: &mut Stage) {
    let warm = [1.0, 0.7, 0.4];
    let case_height = 3.1f32.min(s.spring - 0.25);
    // 1. The orrery in the middle, under the vault.
    let heart = s.floor(s.focus);
    s.solid(heart, 0.82, false);
    s.piece(move |m, g, ctx| orrery_base(m, g, heart, ctx.stone));
    s.plan.orrery = Some(heart + Y(1.55));
    s.light(heart + Y(1.6), [1.0, 0.72, 0.36], 3.4, 7.0);
    // 2. The sunken corner: a chamfer corner gives way to black water.
    let mut corners = s.corners.clone();
    s.rng.shuffle(&mut corners);
    let mut sunk = None;
    for corner in &corners {
        let c = s.floor(corner.mid + corner.inward * 2.2);
        if s.walkway(c) < 2.2 || s.crowded(c, 1.6, 0.0) {
            continue;
        }
        sunk = Some((c, corner.clone()));
        break;
    }
    if let Some((c, corner)) = sunk {
        s.piece(move |m, g, ctx| {
            let stone = ctx.stone;
            let super::plan::Ctx { rng, wet, .. } = ctx;
            sunken_pool(m, g, c, 1.75, stone, rng, wet);
        });
        let lean = s.floor(c + corner.inward * -0.25);
        s.solid(lean, 0.5, false);
        for d in [0.9, 1.6] {
            if let Some(q) = s.place(&[lean + corner.inward * d], 0.42, 1.2, 0.6) {
                s.plan.solids.push((q, 0.42));
            }
        }
        let frame = CaseFrame {
            center: lean - Y(0.25),
            inward: corner.inward,
            tangent: corner.tangent,
            tilt: Some((1.1, 0.1, 0.25)),
        };
        s.piece(move |m, g, ctx| grand_bookcase(m, g, 1.8, 2.2, true, frame, &mut ctx.rng));
        s.light(c + Y(0.6), [0.4, 0.75, 0.85], 1.4, 4.5);
    }
    // 3. Bookcases fill every free bay; one bay holds a scroll rack.
    let mut racks = 0;
    let mut cases: Vec<Slot> = vec![];
    let entry = s.entry;
    let mut bays = s.free_bays(Some(entry), 1.3);
    bays.reverse();
    for bay in bays {
        let width = 2.9f32.min(bay.width - 0.3);
        let circles: Vec<Vec3> = [-0.3f32, 0.0, 0.3]
            .iter()
            .map(|&k| s.floor(bay.center + bay.inward * 0.26 + bay.tangent * (k * width)))
            .collect();
        if circles.iter().any(|&p| s.crowded(p, 0.26, 0.02)) {
            continue;
        }
        s.take_bay(&bay);
        for &p in &circles {
            s.solid(p, 0.26, true);
        }
        if racks == 0 && width > 1.6 && s.rng.chance(0.5) {
            let (c, inw, tan) = (s.floor(bay.center), bay.inward, bay.tangent);
            let h = 2.4f32.min(case_height);
            s.plan.wall_pieces.push(c);
            s.piece(move |m, _g, ctx| scroll_rack(m, c, inw, tan, width, h, &mut ctx.rng));
            racks += 1;
            continue;
        }
        let frame = CaseFrame {
            center: s.floor(bay.center),
            inward: bay.inward,
            tangent: bay.tangent,
            tilt: None,
        };
        s.piece(move |m, g, ctx| {
            grand_bookcase(m, g, width, case_height, false, frame, &mut ctx.rng)
        });
        cases.push(bay);
    }
    // Ladders lean on a few of them.
    for bay in cases.iter().take(2) {
        let x = s.rng.range(-0.4, 0.4);
        let foot = s.floor(bay.center + bay.inward * 1.05 + bay.tangent * x);
        if s.crowded(foot, 0.25, 0.02) || s.walkway(foot) < 0.6 {
            continue;
        }
        let top = bay.center + bay.inward * 0.46 + bay.tangent * x + Y(case_height - 0.1);
        let inw = bay.inward;
        s.piece(move |m, _g, _ctx| ladder(m, foot, top, inw));
        s.solid(foot, 0.22, false);
    }
    // 4. Reading tables with chairs and clutter, a chandelier over one.
    let mut tables = 0;
    for (fw, rw) in [
        (-0.35f32, 0.45f32),
        (0.35, -0.45),
        (0.35, 0.45),
        (-0.35, -0.45),
    ] {
        if tables >= 3 {
            break;
        }
        let yaw = yaw_of(s.inward);
        let cands = [
            s.at(fw * s.depth, rw * s.width),
            s.at(fw * s.depth * 1.2, rw * s.width * 1.1),
        ];
        let yaw_right = yaw_of(s.right);
        let Some(p) = s.place_row(&cands, yaw_right, 1.0, 0.55, 1.6, 0.8) else {
            continue;
        };
        tables += 1;
        s.prop("mk_table", p, yaw, 0.82, 0.0);
        let top = p + Y(0.82);
        s.piece(move |m, g, ctx| desk_clutter(m, g, top, yaw_right, 0.85, 0.4, &mut ctx.rng));
        s.light(top + Y(0.5), warm, 1.5, 4.0);
        for sgn in [-1.0f32, 1.0] {
            let side = s.rng.range(-0.4, 0.4);
            if let Some(chair) = s.spot(p + s.inward * (sgn * 0.85) + s.right * side, 0.25, 0.3) {
                let asset = if s.rng.chance(0.5) {
                    "mk_chair"
                } else {
                    "chair"
                };
                let y = yaw_of(s.inward * -sgn) + s.rng.range(-0.4, 0.4);
                s.prop(asset, chair, y, 0.95, 0.0);
            }
        }
        s.piece(move |m, _g, _ctx| library_rug(m, p, yaw_right, 1.5, 1.1));
        if tables == 1 {
            let at = s.top(p) - Y(1.7);
            s.prop("mk_chandelier", at, 0.0, 1.4, 0.0);
        }
    }
    // 5. Lecterns with their floating runes, a globe, book towers.
    for k in 0..2 {
        let kk = k == 1;
        let cands = [
            s.at(
                s.depth * if kk { -0.6 } else { 0.6 },
                s.width * if kk { 0.1 } else { -0.1 },
            ),
            s.at(
                s.depth * if kk { -0.55 } else { 0.55 },
                s.width * if kk { -0.3 } else { 0.3 },
            ),
        ];
        let Some(p) = s.place(&cands, 0.32, 1.6, 0.7) else {
            continue;
        };
        s.plan.solids.push((p, 0.32));
        let yaw = yaw_of(hz(s.c - p));
        s.piece(move |m, g, ctx| lectern_tome(m, g, p, yaw, &mut ctx.rng));
        s.light(p + Y(1.8), RUNE, 1.3, 3.5);
    }
    let cands = [
        s.at(s.depth * 0.1, s.width * 0.7),
        s.at(-s.depth * 0.1, -s.width * 0.7),
    ];
    if let Some(g) = s.place(&cands, 0.35, 1.6, 0.7) {
        s.plan.solids.push((g, 0.35));
        s.piece(move |m, _g, ctx| globe(m, g, &mut ctx.rng));
    }
    let ring = |a: f32, walk: f32| Scatter {
        walkway: walk,
        ring: [a, 1.0],
        ..Scatter::default()
    };
    s.scatter(7, 0.25, ring(0.45, 0.7), &mut |s, p, _| {
        let count = 6 + s.rng.index(10);
        s.piece(move |m, _g, ctx| book_tower(m, p, count, &mut ctx.rng));
    });
    s.scatter(4, 0.3, ring(0.4, 0.5), &mut |s, p, _| {
        let asset = if s.rng.chance(0.5) {
            "mk_books"
        } else {
            "book_stack"
        };
        let yaw = s.rng.range(0.0, TAU);
        let h = s.rng.range(0.2, 0.3);
        s.prop(asset, p, yaw, h, 0.0);
    });
    s.scatter(3, 0.3, ring(0.6, 0.8), &mut |s, p, _| {
        let yaw = s.rng.range(0.0, TAU);
        s.prop("mk_candlesticks", p, yaw, 0.6, 0.0);
    });
    // 6. Pages everywhere, dust, webs on the cases, candle clusters.
    let floor_only = Scatter {
        walkway: 0.0,
        ..Scatter::default()
    };
    s.scatter(10, 0.5, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.3, 0.7);
        let count = 3 + s.rng.index(5);
        s.piece(move |m, _g, ctx| loose_pages(m, p, r, count, &mut ctx.rng));
    });
    s.scatter(10, 0.4, floor_only, &mut |s, p, _| {
        let r = s.rng.range(0.25, 0.55);
        s.dust(p, r, None);
    });
    s.scatter(5, 0.15, ring(0.55, 0.6), &mut |s, p, _| {
        let count = 3 + s.rng.index(3);
        s.candle_cluster(p, count, 0.12);
    });
    for bay in cases.iter().skip(2).take(3) {
        let side = if s.rng.chance(0.5) { 1.0 } else { -1.0 };
        let apex = bay.center
            + bay.tangent * side * (bay.width * 0.5 - 0.2)
            + bay.inward * 0.45
            + Y(case_height + 0.1);
        let a = norm(
            bay.tangent
                * if (apex - bay.center).dot(bay.tangent) > 0.0 {
                    -1.0
                } else {
                    1.0
                }
                + Y(-0.3),
        );
        let b = norm(bay.inward + Y(-0.5));
        let size = s.rng.range(0.4, 0.6);
        s.corner_web(apex, a, b, size);
    }
    let top = s.c + Y(s.height - 1.2);
    s.light(top, warm, 1.2, 9.0);
    s.plan.sparkle_tint = Some([1.2, 1.05, 0.7]);
    let h = s.height;
    s.sparkles(20, 0.5, h - 1.0, None, None);
}
