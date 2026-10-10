//! The Demon Queen, seated at her feast (a port of `queen.js`). Built with
//! anatomical proportions: a head about 40 px tall, a real neck and sloping
//! shoulders, bust, waist and hips, and arms whose shoulder, elbow and wrist
//! are fused into one form. Her gown is painted onto that form.
use std::rc::Rc;

use super::palette::Palette;
use super::parts::{add, bell_cuff, elbow, fist, mul, perp, sub, unit, CuffOpts, FistOpts};
use super::pixel::{
    catmull, clamp, jr, Figure, Material, Recolor, Rgb, RimLight, Shade, Surface, O, P2,
};
use super::sdf::{capsule, circle, ellipse, poly, smin, union, Field};

pub const QUEEN_SIZE: (usize, usize) = (200, 184);
/// Where her mouth is in figure space (bites, crumbs, pops).
pub const QUEEN_MOUTH: (f64, f64) = (100.0, 70.0);

const K: Rgb = [30.0, 12.0, 36.0];
const EYE: &[(char, Rgb)] = &[
    ('K', K),
    ('L', [92.0, 60.0, 118.0]),
    ('w', [238.0, 236.0, 250.0]),
    ('1', [120.0, 48.0, 8.0]),
    ('2', [196.0, 110.0, 16.0]),
    ('3', [240.0, 170.0, 40.0]),
    ('4', [255.0, 222.0, 110.0]),
    ('P', [44.0, 12.0, 4.0]),
    ('W', [255.0, 255.0, 255.0]),
];
const EYE_OPEN: [&str; 6] = [
    "..KKKKK.", "KKK111KK", ".w12PP2w", ".w23PW3w", "..w344w.", "...LLL..",
];
const EYE_HAPPY: [&str; 4] = ["........", "..KKKK..", ".K....K.", "K......K"];
const MOUTH: &[(char, Rgb)] = &[
    ('K', [60.0, 20.0, 44.0]),
    ('W', [255.0, 250.0, 244.0]),
    ('r', [96.0, 18.0, 36.0]),
    ('T', [214.0, 96.0, 116.0]),
];
const MOUTH_CHOMP: [&str; 5] = [".KKKKK.", "KWWWWWK", "KrrrrrK", "KrTTTrK", ".KKKKK."];
const MOUTH_CHEW_A: [&str; 2] = ["K.K.K", ".K.K."];
const MOUTH_CHEW_B: [&str; 2] = [".K.K.", "K.K.K"];
const MOUTH_SMILE: [&str; 2] = ["K...K", ".KWK."];

const GOLD_PX: [Rgb; 2] = [[240.0, 186.0, 72.0], [156.0, 96.0, 26.0]];
const NAIL: Rgb = [64.0, 22.0, 92.0];

#[derive(Clone, Debug, PartialEq)]
pub struct QueenPose {
    pub hx: f64,
    pub hy: f64,
    pub hand: Option<P2>,
    pub goblet_hand: Option<P2>,
    pub meat: Option<f64>,
    pub has_meat: bool,
    pub aim: Option<P2>,
    pub mouth: &'static str,
    pub eyes: &'static str,
    pub puff: f64,
    pub forearm: Option<f64>,
}

impl Default for QueenPose {
    fn default() -> Self {
        Self {
            hx: 0.0,
            hy: 0.0,
            hand: None,
            goblet_hand: None,
            meat: None,
            has_meat: false,
            aim: None,
            mouth: "smile",
            eyes: "open",
            puff: 0.0,
            forearm: None,
        }
    }
}

fn ring_on_stroke(fig: &mut Figure, pts: &[P2], t: f64, r: f64, m: &Rc<Material>) {
    let c = catmull(pts, 8);
    let i = (c.len() - 2).min((t * (c.len() as f64 - 1.0)).floor() as usize);
    let [x0, y0] = c[i];
    let [x1, y1] = c[i + 1];
    let l = {
        let l = (x1 - x0).hypot(y1 - y0);
        if l == 0.0 {
            1.0
        } else {
            l
        }
    };
    let (nx, ny) = (-(y1 - y0) / l, (x1 - x0) / l);
    fig.capsule(
        x0 - nx * r,
        y0 - ny * r,
        x0 + nx * r,
        y0 + ny * r,
        1.4,
        1.4,
        m,
        O::new().line(true),
    );
}

/// Sweetheart neckline height at a horizontal offset from the centre.
fn neckline(u: f64) -> f64 {
    let pts: [(f64, f64); 9] = [
        (0.0, 116.0),
        (3.0, 113.0),
        (6.0, 109.0),
        (9.0, 107.5),
        (12.0, 107.5),
        (16.0, 108.5),
        (20.0, 109.0),
        (24.0, 107.0),
        (30.0, 104.0),
    ];
    for i in 1..pts.len() {
        if u <= pts[i].0 {
            let (u0, y0) = pts[i - 1];
            let (u1, y1) = pts[i];
            return y0 + ((y1 - y0) * (u - u0)) / (u1 - u0);
        }
    }
    104.0
}

pub struct Arm {
    pub e: P2,
    pub d: P2,
    pub wrist: P2,
    pub field: Field,
    pub bx: [f64; 4],
}

/// One arm as a single fused field: deltoid, upper arm and forearm.
fn arm_field(sh: P2, grip: P2, bend: P2, l1: f64, l2: f64) -> Arm {
    let e = elbow(sh, grip, l1, l2, bend);
    let d = unit(sub(grip, e));
    let wrist = add(grip, mul(d, -6.5));
    let field = smin(
        3.2,
        vec![
            circle(sh[0], sh[1] + 1.0, 7.6, 7.6),
            capsule(sh[0], sh[1], e[0], e[1], 7.0, 5.2, None),
            capsule(e[0], e[1], wrist[0], wrist[1], 5.2, 3.6, None),
        ],
    );
    let xs = [sh[0], e[0], wrist[0]];
    let ys = [sh[1], e[1], wrist[1]];
    let mn = |v: &[f64]| v.iter().cloned().fold(f64::INFINITY, f64::min);
    let mx = |v: &[f64]| v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Arm {
        e,
        d,
        wrist,
        field,
        bx: [
            mn(&xs) - 10.0,
            mn(&ys) - 10.0,
            mx(&xs) + 10.0,
            mx(&ys) + 10.0,
        ],
    }
}

/// Bare shoulder cap above a sleeve edged in gold.
fn shoulder_cap(fig: &mut Figure, arm: i16, sh: P2, skin: &Rc<Material>) {
    let edge = move |x: f64| sh[1] + 3.0 - 0.28 * (x + 0.5 - sh[0]).abs();
    fig.recolor(
        skin,
        &move |x, y| {
            (x as f64 + 0.5 - sh[0]).hypot(y as f64 + 0.5 - sh[1]) < 9.0
                && (y as f64 + 0.5) < edge(x as f64)
        },
        Recolor::shapes(&[arm]),
    );
    let mut x = (sh[0] - 9.0).floor() as i32;
    while (x as f64) <= sh[0] + 9.0 {
        let y = jr(edge(x as f64)) as i32;
        if let Some((shape, _)) = fig.at(x, y) {
            if shape == arm {
                fig.px(x as f64, y as f64, GOLD_PX[0]);
                fig.px(x as f64, (y + 1) as f64, GOLD_PX[1]);
            }
        }
        x += 1;
    }
}

pub fn draw_queen(c: &Palette, pose: &QueenPose) -> Surface {
    let mut fig = Figure::new(
        QUEEN_SIZE.0,
        QUEEN_SIZE.1,
        [-0.5, -0.62, 0.6],
        Some(RimLight {
            dir: [1, 0],
            color: [255.0, 184.0, 110.0],
            k: 0.42,
        }),
    );
    let hx = 100.0 + pose.hx;
    let hy = 55.0 + pose.hy;
    let skin = &c.queen_skin;
    let hair = &c.queen_hair;
    let (horn, gold, dress, crimson, ear_in, meat_m, bone, wine, gem_red) = (
        &c.horn, &c.gold, &c.dress, &c.crimson, &c.ear_in, &c.meat, &c.bone, &c.wine, &c.gem_red,
    );

    // Mane: flows from the crown over the back, spreading past the shoulders.
    let mane: Vec<P2> = vec![
        [hx - 19.0, hy - 6.0],
        [hx - 15.0, hy - 17.0],
        [hx, hy - 22.0],
        [hx + 15.0, hy - 17.0],
        [hx + 19.0, hy - 6.0],
        [hx + 25.0, hy + 16.0],
        [hx + 31.0, hy + 40.0],
        [hx + 33.0, hy + 66.0],
        [hx + 38.0, hy + 92.0],
        [hx + 36.0, hy + 122.0],
        [hx + 30.0, hy + 112.0],
        [hx + 25.0, hy + 124.0],
        [hx - 25.0, hy + 124.0],
        [hx - 30.0, hy + 112.0],
        [hx - 36.0, hy + 122.0],
        [hx - 38.0, hy + 92.0],
        [hx - 33.0, hy + 66.0],
        [hx - 31.0, hy + 40.0],
        [hx - 25.0, hy + 16.0],
    ];
    let mane_id = fig.poly(
        &mane,
        hair,
        O::new().shade(Shade::Cyl).curve(0.8).tone(-0.1),
    );
    fig.shade_where(mane_id, &|x, y| {
        let u = (x + 0.5 - hx).abs() / 20.0;
        let v = (y - hy) / 42.0;
        if v > 0.0 && u < 1.0 {
            -0.5 * (1.0 - u * u) * (v * 3.0).min(1.0) * (1.0 - (v - 0.6).max(0.0))
        } else {
            0.0
        }
    });
    let mut locks: Vec<i16> = Vec::new();
    for side in [-1.0, 1.0] {
        for k in 0..2 {
            let k = k as f64;
            let o = 24.0 + k * 7.0;
            locks.push(fig.stroke(
                &[
                    [hx + side * (o - 7.0), hy + 2.0],
                    [hx + side * (o + 4.0), hy + 26.0],
                    [hx + side * o, hy + 52.0],
                    [hx + side * (o + 7.0), hy + 78.0],
                    [hx + side * (o + 2.0), hy + 104.0],
                    [hx + side * (o + 6.0), hy + 126.0 + k * 3.0],
                ],
                6.2 - k * 1.5,
                0.7,
                hair,
                O::new().flatten(0.55).line_same().tone(-0.06 * k),
            ));
        }
    }
    // Hair falls into shadow as it drops behind the shoulders.
    let mut ids = vec![mane_id];
    ids.extend(locks.iter().copied());
    for id in ids {
        fig.shade_where(id, &|_x, y| -0.3 * clamp((y - hy - 34.0) / 80.0, 0.0, 1.0));
    }

    // Horns rise from under the hair, sweep outward and curl back at the tips.
    for side in [-1.0, 1.0] {
        let pts: Vec<P2> = vec![
            [hx + side * 8.0, hy - 11.0],
            [hx + side * 14.0, hy - 21.0],
            [hx + side * 21.0, hy - 30.0],
            [hx + side * 22.0, hy - 40.0],
            [hx + side * 17.0, hy - 46.0],
        ];
        fig.stroke(&pts, 4.4, 0.9, horn, O::new().steps(8));
        ring_on_stroke(&mut fig, &pts, 0.3, 4.3, gold);
        ring_on_stroke(&mut fig, &pts, 0.52, 3.5, gold);
    }

    // Long pointed ears with rose inner shells and ruby drops.
    for side in [-1.0, 1.0] {
        fig.poly(
            &[
                [hx + side * 12.0, hy - 1.0],
                [hx + side * 37.0, hy - 12.0],
                [hx + side * 31.0, hy - 4.0],
                [hx + side * 13.0, hy + 8.0],
            ],
            skin,
            O::new().shade(Shade::Dome),
        );
        fig.poly(
            &[
                [hx + side * 15.0, hy + 1.0],
                [hx + side * 32.0, hy - 9.0],
                [hx + side * 28.0, hy - 4.0],
                [hx + side * 15.0, hy + 5.0],
            ],
            ear_in,
            O::new().shade(Shade::Flat).line(false),
        );
        fig.ellipse(hx + side * 14.0, hy + 10.0, 1.3, 1.3, gold, O::new());
        fig.ellipse(hx + side * 14.0, hy + 13.0, 1.2, 1.8, gem_red, O::new());
    }

    // Neck rising from the collarbones.
    let neck = capsule(hx, hy + 14.0, 100.0, 90.0, 4.2, 5.8, None);
    fig.sdf(
        skin,
        &*neck,
        [88.0, hy + 6.0, 112.0, 96.0],
        O::new().line(false).curve(0.8),
    );

    // Torso: sloping shoulders, ribcage, a narrow waist and full hips.
    let torso_pts: Vec<P2> = vec![
        [95.0, 86.0],
        [105.0, 86.0],
        [112.0, 88.0],
        [120.0, 91.0],
        [126.0, 94.0],
        [129.0, 99.0],
        [127.0, 106.0],
        [123.0, 114.0],
        [121.0, 122.0],
        [120.0, 130.0],
        [118.0, 138.0],
        [118.0, 143.0],
        [120.0, 150.0],
        [123.0, 158.0],
        [126.0, 166.0],
        [128.0, 184.0],
        [72.0, 184.0],
        [74.0, 166.0],
        [77.0, 158.0],
        [80.0, 150.0],
        [82.0, 143.0],
        [82.0, 138.0],
        [80.0, 130.0],
        [79.0, 122.0],
        [77.0, 114.0],
        [73.0, 106.0],
        [71.0, 99.0],
        [74.0, 94.0],
        [80.0, 91.0],
        [88.0, 88.0],
    ];
    let torso_field = poly(torso_pts, 18.0);
    let torso = fig.sdf(
        dress,
        &*torso_field,
        [70.0, 84.0, 130.0, 184.0],
        O::new().curve(0.9),
    );
    let bust_field = union(vec![
        ellipse(90.5, 113.5, 9.6, 9.0),
        ellipse(109.5, 113.5, 9.6, 9.0),
    ]);
    let bust = fig.sdf(
        dress,
        &*bust_field,
        [79.0, 103.0, 121.0, 124.0],
        O::new().line(false).cast(2),
    );
    fig.texture(
        dress,
        Box::new(|x, y| {
            let u = ((x + ((y.div_euclid(8)) & 1) * 4) % 8) - 4;
            let v = (y % 8) - 4;
            if u.abs() + v.abs() == 3 {
                0.08
            } else {
                0.0
            }
        }),
    );
    // Bare shoulders and decolletage above the neckline; collarbones.
    fig.recolor(
        skin,
        &|x, y| (y as f64) < neckline((x as f64 + 0.5 - 100.0).abs()),
        Recolor::shapes(&[torso, bust]),
    );
    for side in [-1.0, 1.0] {
        fig.fold(
            &[
                [100.0 + side * 3.0, 92.0],
                [100.0 + side * 9.0, 93.0],
                [100.0 + side * 17.0, 92.0],
            ],
            0.14,
            0.8,
        );
    }
    fig.px(100.0, 92.0, [60.0, 54.0, 128.0]);
    // Corset: crimson centre panel, gold lacing, boning seams.
    let panel = |y: f64| {
        if y < 118.0 {
            0.0
        } else if y < 142.0 {
            3.4 + (y - 118.0) * 0.07
        } else {
            5.1 - (y - 142.0) * 0.5
        }
    };
    fig.recolor(
        crimson,
        &|x, y| {
            let (xf, yf) = (x as f64, y as f64);
            yf >= neckline((xf + 0.5 - 100.0).abs())
                && yf < 152.0
                && (xf + 0.5 - 100.0).abs() < panel(yf)
        },
        Recolor::shapes(&[torso, bust]),
    );
    let mut y = 120.0;
    while y < 146.0 {
        fig.line(97.0, y, 103.0, y + 2.0, GOLD_PX[0]);
        fig.line(103.0, y, 97.0, y + 2.0, GOLD_PX[0]);
        y += 4.0;
    }
    for side in [-1.0, 1.0] {
        for y in 118..150 {
            fig.px(
                jr(100.0 + side * panel(y as f64)),
                y as f64,
                GOLD_PX[(y & 1) as usize],
            );
        }
        fig.fold(
            &[[100.0 + side * 12.0, 124.0], [100.0 + side * 13.0, 146.0]],
            0.14,
            0.5,
        );
    }
    // Gold edging along the sweetheart neckline, with rubies at the peaks.
    for x in 71..=129 {
        let y = jr(neckline((x as f64 + 0.5 - 100.0).abs())) as i32;
        let Some((shape, _)) = fig.at(x, y) else {
            continue;
        };
        if shape != torso && shape != bust {
            continue;
        }
        fig.px(x as f64, y as f64, GOLD_PX[0]);
        fig.px(x as f64, (y + 1) as f64, GOLD_PX[1]);
    }
    fig.ellipse(89.5, 108.0, 1.2, 1.2, gem_red, O::new().line(false));
    fig.ellipse(110.5, 108.0, 1.2, 1.2, gem_red, O::new().line(false));
    // Under-bust crease.
    fig.fold(&[[81.0, 121.0], [90.0, 124.0], [98.0, 123.0]], 0.18, 0.5);
    fig.fold(&[[102.0, 123.0], [110.0, 124.0], [119.0, 121.0]], 0.18, 0.5);
    // Chain belt dipping to a ruby medallion; an open crimson underskirt.
    for x in 80..=120 {
        let y = 145.0 + jr((1.0 - ((x as f64 - 100.0) / 21.0).powi(2)) * 4.0);
        fig.px(x as f64, y, GOLD_PX[((x >> 1) & 1) as usize]);
    }
    fig.ellipse(100.0, 149.0, 2.6, 2.6, gold, O::new());
    fig.ellipse(100.0, 149.0, 1.2, 1.2, gem_red, O::new().line(false));
    fig.recolor(
        crimson,
        &|x, y| (y as f64) > 152.0 && (x as f64 + 0.5 - 100.0).abs() < (y as f64 - 152.0) * 0.42,
        Recolor::shapes(&[torso]),
    );
    for y in 153..184 {
        for side in [-1.0, 1.0] {
            fig.px(
                jr(100.0 + side * (y as f64 - 152.0) * 0.42),
                y as f64,
                GOLD_PX[(y & 1) as usize],
            );
        }
    }
    for (a, b) in [
        ([86.0, 154.0], [82.0, 184.0]),
        ([114.0, 154.0], [118.0, 184.0]),
    ] {
        fig.fold(&[a, b], 0.2, 0.5);
    }
    // Choker, and a gold collar necklace with ruby drops over the collarbones.
    fig.capsule(94.5, 86.5, 105.5, 86.5, 1.1, 1.1, gold, O::new());
    fig.ellipse(100.0, 89.5, 1.2, 1.6, gem_red, O::new());
    for x in 86i32..=114 {
        let y = 95.0 + jr((1.0 - ((x as f64 - 100.0) / 14.0).powi(2)) * 5.0);
        fig.px(x as f64, y, GOLD_PX[((x >> 1) & 1) as usize]);
        fig.px(x as f64, y + 1.0, GOLD_PX[1]);
        if x % 5 == 0 && (x - 100).abs() < 12 {
            fig.px(x as f64, y + 2.0, [220.0, 40.0, 70.0]);
            fig.px(x as f64, y + 3.0, [140.0, 16.0, 40.0]);
        }
    }
    fig.ellipse(100.0, 102.5, 1.8, 2.3, gem_red, O::new());

    // Left arm (viewer's right) holds the goblet at the table edge.
    let sh_l: P2 = [125.0, 99.0];
    let grip_l = pose.goblet_hand.unwrap_or([117.0, 146.0]);
    let arm_l = arm_field(sh_l, grip_l, [0.3, 1.0], 29.0, 21.0);
    let arm_l_id = fig.sdf(
        dress,
        &*arm_l.field,
        arm_l.bx,
        O::new()
            .line_same()
            .cast(1)
            .rim_over()
            .joins(vec![[sh_l[0], sh_l[1] + 1.0, 9.0]]),
    );
    shoulder_cap(&mut fig, arm_l_id, sh_l, skin);
    fig.fold(
        &[
            add(arm_l.e, mul(perp(arm_l.d), 2.0)),
            add(add(arm_l.e, mul(arm_l.d, 4.0)), mul(perp(arm_l.d), -2.0)),
        ],
        0.22,
        0.5,
    );
    let (gx, gy) = (grip_l[0], grip_l[1]);
    fig.poly(
        &[
            [gx - 5.5, gy - 21.0],
            [gx + 5.5, gy - 21.0],
            [gx + 4.5, gy - 14.0],
            [gx + 1.0, gy - 11.0],
            [gx - 1.0, gy - 11.0],
            [gx - 4.5, gy - 14.0],
        ],
        gold,
        O::new().shade(Shade::Cyl),
    );
    fig.ellipse(gx, gy - 21.0, 5.0, 1.3, wine, O::new().flat().line(false));
    fig.capsule(gx, gy - 11.0, gx, gy + 7.0, 1.1, 1.1, gold, O::new());
    fig.ellipse(gx, gy + 8.0, 3.6, 1.2, gold, O::new());
    fig.ellipse(gx, gy - 17.0, 1.3, 1.3, gem_red, O::new());
    fist(
        &mut fig,
        grip_l,
        [0.0, 1.0],
        arm_l.wrist,
        skin,
        FistOpts {
            nail: Some(NAIL),
            ring: Some(GOLD_PX),
            ..Default::default()
        },
    );
    bell_cuff(
        &mut fig,
        arm_l.wrist,
        arm_l.d,
        dress,
        Some(crimson),
        Some(GOLD_PX),
        CuffOpts::default(),
    );

    // Face: rounded cranium and a tapering jaw; cheeks bulge when full.
    let puff = pose.puff;
    let mut face_fields: Vec<Field> = vec![
        circle(hx, hy, 15.5, 17.0),
        capsule(hx, hy + 4.0, hx, hy + 15.5, 13.6, 6.4, Some(17.0)),
    ];
    if puff > 0.0 {
        for side in [-1.0, 1.0] {
            face_fields.push(circle(hx + side * 9.5, hy + 12.0, 4.4 + puff * 2.0, 17.0));
        }
    }
    let face_field = smin(3.5, face_fields);
    let face = fig.sdf(
        skin,
        &*face_field,
        [hx - 22.0, hy - 17.0, hx + 22.0, hy + 24.0],
        O::new().cast(1).curve(0.8),
    );
    fig.shade_below(face, 2, 0.22);

    // Side locks framing the face and falling over the collarbones.
    for side in [-1.0, 1.0] {
        fig.stroke(
            &[
                [hx + side * 14.5, hy - 6.0],
                [hx + side * 15.0, hy + 6.0],
                [hx + side * 12.0, hy + 18.0],
                [hx + side * 15.0, hy + 34.0],
                [hx + side * 18.0, hy + 50.0],
                [hx + side * 15.0, hy + 64.0],
            ],
            4.4,
            0.6,
            hair,
            O::new().cast(1).line_same().flatten(0.5),
        );
    }

    // Fringe: pointed locks to the brow line, parted a little off centre.
    let fringe: Vec<P2> = vec![
        [hx - 17.0, hy + 5.0],
        [hx - 18.0, hy - 6.0],
        [hx - 13.0, hy - 15.0],
        [hx - 6.0, hy - 19.0],
        [hx + 2.0, hy - 20.0],
        [hx + 10.0, hy - 18.0],
        [hx + 16.0, hy - 12.0],
        [hx + 18.0, hy - 3.0],
        [hx + 17.0, hy + 5.0],
        [hx + 14.0, hy - 1.0],
        [hx + 12.0, hy + 3.0],
        [hx + 9.0, hy - 4.0],
        [hx + 6.0, hy + 1.0],
        [hx + 3.0, hy - 7.0],
        [hx, hy],
        [hx - 3.0, hy - 6.0],
        [hx - 6.0, hy + 1.0],
        [hx - 9.0, hy - 5.0],
        [hx - 12.0, hy + 3.0],
        [hx - 14.0, hy - 2.0],
    ];
    fig.poly(
        &fringe,
        hair,
        O::new().shade(Shade::Dome).curve(0.9).cast(3),
    );
    for [x0, y0, x1, y1] in [
        [hx - 2.0, hy - 17.0, hx - 4.0, hy - 7.0],
        [hx + 5.0, hy - 17.0, hx + 4.0, hy - 6.0],
        [hx - 10.0, hy - 13.0, hx - 11.0, hy - 4.0],
        [hx + 12.0, hy - 12.0, hx + 11.0, hy - 4.0],
    ] {
        fig.line(x0, y0, x1, y1, [150.0, 152.0, 196.0]);
    }
    for k in -9..=9 {
        let y = hy - 15.0 + jr((k as f64 / 9.0).powi(2) * 3.0);
        if (k as i32).abs() % 4 != 3 {
            fig.px(
                hx + k as f64,
                y,
                if k < 2 {
                    [255.0, 255.0, 255.0]
                } else {
                    [228.0, 232.0, 250.0]
                },
            );
        }
    }

    // Dark spiked crown with rubies.
    fig.poly(
        &[
            [hx - 8.0, hy - 21.0],
            [hx + 8.0, hy - 21.0],
            [hx + 7.0, hy - 17.0],
            [hx - 7.0, hy - 17.0],
        ],
        horn,
        O::new().shade(Shade::Cyl),
    );
    for (dx, h) in [(-7.0, 4.0), (-3.5, 6.0), (0.0, 9.0), (3.5, 6.0), (7.0, 4.0)] {
        fig.poly(
            &[
                [hx + dx - 2.0, hy - 20.0],
                [hx + dx + 2.0, hy - 20.0],
                [hx + dx, hy - 20.0 - h],
            ],
            horn,
            O::new().shade(Shade::Cyl).line_same(),
        );
    }
    fig.line(
        hx - 7.0,
        hy - 18.0,
        hx + 7.0,
        hy - 18.0,
        [218.0, 154.0, 48.0],
    );
    fig.ellipse(hx, hy - 22.0, 1.5, 1.8, gem_red, O::new());

    // Face details: eyes, brows, nose, blush and mouth.
    if pose.eyes == "open" {
        fig.stamp(hx - 11.0, hy + 3.0, &EYE_OPEN, EYE, false);
        fig.stamp(hx + 4.0, hy + 3.0, &EYE_OPEN, EYE, true);
    } else {
        fig.stamp(hx - 11.0, hy + 4.0, &EYE_HAPPY, EYE, false);
        fig.stamp(hx + 4.0, hy + 4.0, &EYE_HAPPY, EYE, true);
    }
    fig.line(
        hx - 10.0,
        hy + 1.0,
        hx - 5.0,
        hy + 1.0,
        [120.0, 118.0, 176.0],
    );
    fig.line(
        hx + 5.0,
        hy + 1.0,
        hx + 10.0,
        hy + 1.0,
        [120.0, 118.0, 176.0],
    );
    fig.px(hx + 1.0, hy + 11.0, [84.0, 80.0, 164.0]);
    for bx in [hx - 12.0, hx + 8.0] {
        fig.px(bx, hy + 10.0, [226.0, 138.0, 200.0]);
        fig.px(bx + 2.0, hy + 10.0, [226.0, 138.0, 200.0]);
        fig.px(bx + 1.0, hy + 11.0, [200.0, 118.0, 186.0]);
    }
    match pose.mouth {
        "chomp" => fig.stamp(hx - 3.0, hy + 12.0, &MOUTH_CHOMP, MOUTH, false),
        "chewA" => fig.stamp(hx - 2.0, hy + 15.0, &MOUTH_CHEW_A, MOUTH, false),
        "chewB" => fig.stamp(hx - 2.0, hy + 15.0, &MOUTH_CHEW_B, MOUTH, false),
        _ => fig.stamp(hx - 2.0, hy + 14.0, &MOUTH_SMILE, MOUTH, false),
    }

    // Right arm (viewer's left), in front of everything: a drumstick to her lips.
    let sh_r: P2 = [75.0, 99.0];
    let grip = pose.hand.unwrap_or([85.0, 143.0]);
    let arm_r = arm_field(sh_r, grip, [-0.3, 1.0], 29.0, pose.forearm.unwrap_or(21.0));
    let arm_r_id = fig.sdf(
        dress,
        &*arm_r.field,
        arm_r.bx,
        O::new()
            .line_same()
            .cast(1)
            .rim_over()
            .joins(vec![[sh_r[0], sh_r[1] + 1.0, 9.0]]),
    );
    shoulder_cap(&mut fig, arm_r_id, sh_r, skin);
    fig.fold(
        &[
            add(arm_r.e, mul(perp(arm_r.d), -2.0)),
            add(add(arm_r.e, mul(arm_r.d, 4.0)), mul(perp(arm_r.d), 2.0)),
        ],
        0.22,
        0.5,
    );
    let mut axis = arm_r.d;
    if pose.has_meat {
        let target = pose.aim.unwrap_or([hx, hy + 15.0]);
        axis = unit(sub(target, grip));
        let [dx, dy] = axis;
        let m = clamp(pose.meat.unwrap_or(1.0), 0.25, 1.0);
        fig.capsule(
            grip[0] - dx * 9.0,
            grip[1] - dy * 9.0,
            grip[0] + dx * 2.0,
            grip[1] + dy * 2.0,
            1.6,
            1.6,
            bone,
            O::new(),
        );
        for k in [-1.0, 1.0] {
            fig.ellipse(
                grip[0] - dx * 9.0 - dy * k * 1.6,
                grip[1] - dy * 9.0 + dx * k * 1.6,
                2.0,
                2.0,
                bone,
                O::new(),
            );
        }
        fig.capsule(
            grip[0] + dx * 5.0,
            grip[1] + dy * 5.0,
            grip[0] + dx * (6.0 + 11.0 * m),
            grip[1] + dy * (6.0 + 11.0 * m),
            3.6,
            3.4 + 4.4 * m,
            meat_m,
            O::new().cast(2),
        );
        fig.px(
            grip[0] + dx * 9.0 - dy * 2.0,
            grip[1] + dy * 9.0 + dx * 2.0 - 1.0,
            [255.0, 230.0, 164.0],
        );
        fig.px(
            grip[0] + dx * 11.0 - dy * 2.0,
            grip[1] + dy * 11.0 + dx * 2.0 - 1.0,
            [255.0, 214.0, 132.0],
        );
    }
    fist(
        &mut fig,
        grip,
        axis,
        arm_r.wrist,
        skin,
        FistOpts {
            nail: Some(NAIL),
            ring: Some(GOLD_PX),
            ..Default::default()
        },
    );
    bell_cuff(
        &mut fig,
        arm_r.wrist,
        arm_r.d,
        dress,
        Some(crimson),
        Some(GOLD_PX),
        CuffOpts::default(),
    );
    fig.render()
}
