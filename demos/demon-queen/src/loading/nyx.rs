//! Nyxara, the royal physician (a port of `nyx.js`): a dark elf with warm
//! umber skin, black-violet hair, reading glasses worn low, a white cap with a
//! burgundy cross, and the ivory, burgundy and black uniform of the royal
//! infirmary painted onto a body whose shoulders, arms and hands are fused.
use std::rc::Rc;

use super::palette::Palette;
use super::parts::{add, band_cuff, elbow, fist, mul, perp, sub, unit, CuffOpts, FistOpts};
use super::pixel::{
    bayer, clamp, jr, material, ramp, raw_material, Figure, MatOpts, Material, Recolor, Rgb,
    RimLight, Shade, Surface, O, P2,
};
use super::sdf::{capsule, circle, ellipse, poly, smin, union, Field};

pub const NYX_SIZE: (usize, usize) = (200, 184);

const K: Rgb = [20.0, 8.0, 22.0];
const EYE: &[(char, Rgb)] = &[
    ('K', K),
    ('L', [104.0, 52.0, 72.0]),
    ('w', [242.0, 232.0, 238.0]),
    ('1', [80.0, 12.0, 46.0]),
    ('2', [158.0, 24.0, 76.0]),
    ('3', [222.0, 54.0, 112.0]),
    ('4', [255.0, 128.0, 174.0]),
    ('P', [28.0, 4.0, 16.0]),
    ('W', [255.0, 255.0, 255.0]),
];
// Focused, heavy-lidded eyes looking down at the flask.
const EYE_FOCUS: [&str; 6] = [
    "..KKKKK.", "KKKKKKKK", ".w1111w.", ".w2PP2W.", "..3443..", "...LLL..",
];
const EYE_BLINK: [&str; 4] = ["........", "........", "KKKKKKKK", ".LLLLLL."];
const EYE_SHUT: [&str; 5] = ["........", "........", "K......K", ".KK..KK.", "...KK..."];
const MOUTH: &[(char, Rgb)] = &[
    ('K', [70.0, 22.0, 36.0]),
    ('r', [120.0, 40.0, 58.0]),
    ('h', [176.0, 96.0, 100.0]),
];
const MOUTH_CALM: [&str; 1] = [".KKK."];
const MOUTH_SMILE: [&str; 2] = ["K...K", ".KrK."];
const CROSS: &[(char, Rgb)] = &[
    ('B', [130.0, 24.0, 54.0]),
    ('b', [88.0, 12.0, 36.0]),
    ('g', [232.0, 180.0, 72.0]),
];
const CAP_CROSS: [&str; 3] = [".BB.", "BBBb", ".Bb."];
const CHEST_CROSS: [&str; 6] = ["..g..", ".gBg.", "gBBBg", ".gBg.", ".gBg.", "..g.."];
const GOLD_PX: [Rgb; 2] = [[234.0, 182.0, 78.0], [150.0, 96.0, 34.0]];

/// Where the flask sits in figure space (for glows, bubbles and drops).
pub struct Flask {
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub neck_top: f64,
    pub tip: P2,
}
pub const FLASK: Flask = Flask {
    x: 104.0,
    y: 141.0,
    r: 9.5,
    neck_top: 119.0,
    tip: [105.0, 108.0],
};

#[derive(Clone, Debug, PartialEq)]
pub struct NyxPose {
    pub hy: f64,
    pub eyes: &'static str,
    pub mouth: &'static str,
    pub squeeze: f64,
    pub level: f64,
    pub liquid: Rgb,
    pub glint: f64,
}

impl Default for NyxPose {
    fn default() -> Self {
        Self {
            hy: 0.0,
            eyes: "focus",
            mouth: "calm",
            squeeze: 0.0,
            level: 0.5,
            liquid: [90.0, 230.0, 160.0],
            glint: 0.0,
        }
    }
}

/// Emissive liquid ramp for a hue.
pub fn liquid_material(color: Rgb) -> Rc<Material> {
    let r: Vec<Rgb> = [0.28, 0.5, 0.75, 0.95, 1.15]
        .iter()
        .map(|k| {
            let add = if *k > 1.0 { 40.0 } else { 0.0 };
            [
                (color[0] * k + add).min(255.0),
                (color[1] * k + add).min(255.0),
                (color[2] * k + add).min(255.0),
            ]
        })
        .collect();
    raw_material(r, 0.4, false, false)
}

/// Width of the ivory apron front at a height.
fn apron_half(y: f64) -> f64 {
    let pts: [(f64, f64); 8] = [
        (80.0, 7.0),
        (90.0, 12.0),
        (100.0, 17.5),
        (110.0, 19.5),
        (120.0, 16.0),
        (132.0, 13.5),
        (150.0, 17.0),
        (184.0, 21.0),
    ];
    if y <= pts[0].0 {
        return pts[0].1;
    }
    for i in 1..pts.len() {
        if y <= pts[i].0 {
            let (y0, w0) = pts[i - 1];
            let (y1, w1) = pts[i];
            return w0 + ((w1 - w0) * (y - y0)) / (y1 - y0);
        }
    }
    21.0
}

struct Arm {
    e: P2,
    d: P2,
    wrist: P2,
    field: Field,
    bx: [f64; 4],
}

/// One arm as a single fused field: deltoid, upper arm and forearm.
fn arm_field(sh: P2, grip: P2, bend: P2, l1: f64, l2: f64) -> Arm {
    let e = elbow(sh, grip, l1, l2, bend);
    let d = unit(sub(grip, e));
    let wrist = add(grip, mul(d, -6.2));
    let field = smin(
        3.2,
        vec![
            circle(sh[0], sh[1] + 1.0, 7.2, 7.2),
            capsule(sh[0], sh[1], e[0], e[1], 6.6, 5.0, None),
            capsule(e[0], e[1], wrist[0], wrist[1], 5.0, 3.6, None),
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

/// Frilled ivory cap over the shoulder, scalloped along its hem.
fn shoulder_frill(fig: &mut Figure, sh: P2, side: f64, ivory: &Rc<Material>) {
    let (cx, cy) = (sh[0] - side * 0.5, sh[1] - 1.0);
    let field = smin(
        1.4,
        vec![
            ellipse(cx, cy, 7.4, 4.6),
            ellipse(cx - side * 0.8, cy + 3.2, 6.8, 3.2),
        ],
    );
    fig.sdf(
        ivory,
        &*field,
        [cx - 10.0, cy - 7.0, cx + 10.0, cy + 9.0],
        O::new().cast(2).curve(0.9),
    );
    for k in -6..=6 {
        let x = jr(cx - side * 0.8 + k as f64);
        let y = jr(cy + 3.2 + (1.0 - (k as f64 / 6.8).powi(2)).max(0.0).sqrt() * 3.0);
        let odd = (k & 1) != 0;
        fig.px(
            x,
            y + if odd { 0.0 } else { 1.0 },
            if odd {
                [236.0, 230.0, 230.0]
            } else {
                [150.0, 140.0, 152.0]
            },
        );
    }
    fig.fold(&[[cx - 6.0, cy + 1.0], [cx + 6.0, cy + 1.0]], 0.12, 0.6);
}

pub fn draw_nyx(c: &Palette, pose: &NyxPose) -> Surface {
    let mut fig = Figure::new(
        NYX_SIZE.0,
        NYX_SIZE.1,
        [-0.62, -0.55, 0.56],
        Some(RimLight {
            dir: [-1, 0],
            color: [150.0, 180.0, 255.0],
            k: 0.38,
        }),
    );
    let ear_in = material(
        ramp(&["#4a1a1e", "#7a3232", "#a4524a"]),
        MatOpts::default().dither(0.2).no_outline(),
    );
    let rubber = material(
        ramp(&["#3a0616", "#6a0e28", "#a01e3e", "#d24a66"]),
        MatOpts::default().dither(0.2).line([30, 2, 10]),
    );
    let hx = 100.0;
    let hy = 48.0 + pose.hy;
    let (skin, hair, tip, ivory, burgundy, black, glove, glass, gold, gem_red) = (
        &c.nyx_skin,
        &c.nyx_hair,
        &c.nyx_hair_tip,
        &c.ivory,
        &c.burgundy,
        &c.black,
        &c.glove,
        &c.glass,
        &c.gold,
        &c.gem_red,
    );

    // Long straight hair behind her, down to the waist, fading into violet.
    let back: Vec<P2> = vec![
        [hx - 18.0, hy - 6.0],
        [hx - 14.0, hy - 16.0],
        [hx, hy - 20.0],
        [hx + 14.0, hy - 16.0],
        [hx + 18.0, hy - 6.0],
        [hx + 24.0, hy + 26.0],
        [hx + 30.0, hy + 60.0],
        [hx + 32.0, hy + 100.0],
        [hx + 25.0, hy + 94.0],
        [hx + 19.0, hy + 104.0],
        [hx - 19.0, hy + 104.0],
        [hx - 25.0, hy + 94.0],
        [hx - 32.0, hy + 100.0],
        [hx - 30.0, hy + 60.0],
        [hx - 24.0, hy + 26.0],
    ];
    let back_id = fig.poly(&back, hair, O::new().shade(Shade::Cyl).curve(0.8));
    let fade = hy + 66.0;
    fig.recolor(
        tip,
        &|x, y| {
            let yf = y as f64;
            yf > fade + 12.0 || (yf > fade && bayer(x, y) < (yf - fade) / 12.0)
        },
        Recolor::shapes(&[back_id]),
    );
    fig.shade_where(back_id, &|x, y| {
        let u = (x + 0.5 - hx).abs() / 18.0;
        let v = (y - hy) / 40.0;
        if v > 0.0 && u < 1.0 {
            -0.4 * (1.0 - u * u) * (v * 3.0).min(1.0)
        } else {
            0.0
        }
    });

    // Long dark-elf ears reaching out sideways.
    for side in [-1.0, 1.0] {
        fig.poly(
            &[
                [hx + side * 12.0, hy - 1.0],
                [hx + side * 39.0, hy - 8.0],
                [hx + side * 33.0, hy - 1.0],
                [hx + side * 13.0, hy + 8.0],
            ],
            skin,
            O::new().shade(Shade::Dome),
        );
        fig.poly(
            &[
                [hx + side * 16.0, hy + 1.0],
                [hx + side * 33.0, hy - 5.0],
                [hx + side * 29.0, hy - 1.0],
                [hx + side * 16.0, hy + 5.0],
            ],
            &ear_in,
            O::new().shade(Shade::Flat).line(false),
        );
        fig.ellipse(hx + side * 13.5, hy + 9.0, 1.2, 1.2, gold, O::new());
    }

    // Neck, then the body: sloping shoulders, bust, a slim waist, hips.
    let neck = capsule(hx, hy + 13.0, 100.0, 86.0, 4.0, 5.4, None);
    fig.sdf(
        skin,
        &*neck,
        [88.0, hy + 5.0, 112.0, 92.0],
        O::new().line(false).curve(0.8),
    );
    let torso_pts: Vec<P2> = vec![
        [95.0, 82.0],
        [105.0, 82.0],
        [112.0, 84.0],
        [120.0, 87.0],
        [126.0, 90.0],
        [129.0, 95.0],
        [127.0, 102.0],
        [123.0, 110.0],
        [121.0, 118.0],
        [120.0, 126.0],
        [118.0, 133.0],
        [118.0, 138.0],
        [120.0, 146.0],
        [123.0, 154.0],
        [126.0, 162.0],
        [128.0, 184.0],
        [72.0, 184.0],
        [74.0, 162.0],
        [77.0, 154.0],
        [80.0, 146.0],
        [82.0, 138.0],
        [82.0, 133.0],
        [80.0, 126.0],
        [79.0, 118.0],
        [77.0, 110.0],
        [73.0, 102.0],
        [71.0, 95.0],
        [74.0, 90.0],
        [80.0, 87.0],
        [88.0, 84.0],
    ];
    let torso_field = poly(torso_pts, 18.0);
    let torso = fig.sdf(
        black,
        &*torso_field,
        [70.0, 80.0, 130.0, 184.0],
        O::new().curve(0.9).rim_over(),
    );
    let bust_field = union(vec![
        ellipse(90.5, 108.0, 9.4, 8.8),
        ellipse(109.5, 108.0, 9.4, 8.8),
    ]);
    let bust = fig.sdf(
        black,
        &*bust_field,
        [79.0, 97.0, 121.0, 118.0],
        O::new().line(false).cast(2),
    );
    fig.texture(
        black,
        Box::new(|x, y| if (x + y * 2) % 5 == 0 { 0.05 } else { 0.0 }),
    );
    // Ivory apron-dress over the front, burgundy side panels, a black V at the
    // throat under the collar.
    let u = |x: i32| (x as f64 + 0.5 - 100.0).abs();
    fig.recolor(
        burgundy,
        &|x, y| y > 84 && u(x) < apron_half(y as f64) + 3.0,
        Recolor::shapes(&[torso, bust]),
    );
    fig.recolor(
        ivory,
        &|x, y| {
            y > 84 && u(x) < apron_half(y as f64) && !(y < 95 && u(x) < (95.0 - y as f64) * 0.7)
        },
        Recolor::shapes(&[torso, bust]),
    );
    for y in 85..184 {
        for side in [-1.0, 1.0] {
            let x = jr(100.0 + side * (apron_half(y as f64) + 3.0)) as i32;
            if let Some((shape, _)) = fig.at(x, y) {
                if shape == torso || shape == bust {
                    fig.px(x as f64, y as f64, GOLD_PX[(y & 1) as usize]);
                }
            }
        }
    }
    for k in 0..=7 {
        let y = 84.0 + jr(k as f64 * 1.4);
        for side in [-1.0, 1.0] {
            fig.px(
                jr(100.0 + side * (7 - k) as f64),
                y,
                if (k & 1) != 0 {
                    [240.0, 234.0, 234.0]
                } else {
                    [170.0, 160.0, 170.0]
                },
            );
        }
    }
    fig.stamp(98.0, 99.0, &CHEST_CROSS, CROSS, false);
    fig.fold(&[[82.0, 116.0], [90.0, 119.0], [98.0, 118.0]], 0.18, 0.5);
    fig.fold(&[[102.0, 118.0], [110.0, 119.0], [118.0, 116.0]], 0.18, 0.5);
    for side in [-1.0, 1.0] {
        fig.fold(
            &[
                [100.0 + side * 9.0, 96.0],
                [100.0 + side * 11.0, 112.0],
                [100.0 + side * 8.0, 132.0],
                [100.0 + side * 11.0, 184.0],
            ],
            0.12,
            0.6,
        );
    }
    // Burgundy waist cinch with a gold cross buckle.
    fig.recolor(
        burgundy,
        &|_x, y| (131..=136).contains(&y),
        Recolor::shapes(&[torso]),
    );
    for x in 80..=120 {
        if let Some((s, _)) = fig.at(x, 130) {
            if s == torso {
                fig.px(x as f64, 130.0, GOLD_PX[0]);
            }
        }
        if let Some((s, _)) = fig.at(x, 137) {
            if s == torso {
                fig.px(x as f64, 137.0, GOLD_PX[1]);
            }
        }
    }
    fig.rect(97.0, 130.0, 6.0, 7.0, gold, O::new());
    fig.stamp(
        98.0,
        131.0,
        &[".r.", "rrr", ".r."],
        &[('r', [150.0, 20.0, 52.0])],
        false,
    );
    // Buttons, a pocket with a quill, apron folds and a chatelaine.
    for x in [93.0, 107.0] {
        let mut y = 142.0;
        while y < 182.0 {
            fig.px(x, y, GOLD_PX[0]);
            fig.px(x + 1.0, y + 1.0, GOLD_PX[1]);
            y += 7.0;
        }
    }
    fig.fold(
        &[[84.0, 150.0], [84.0, 160.0], [91.0, 161.0], [92.0, 150.0]],
        0.16,
        0.4,
    );
    fig.line(84.0, 149.0, 92.0, 149.0, [255.0, 252.0, 246.0]);
    let quill = material(
        ramp(&["#6a5a78", "#a898b8", "#dcd0e6", "#fff8ff"]),
        MatOpts::default().dither(0.0).line([50, 40, 60]),
    );
    fig.stroke(
        &[[89.0, 150.0], [86.0, 143.0], [82.0, 138.0]],
        1.0,
        2.0,
        &quill,
        O::new().steps(4),
    );
    for (a, b) in [
        ([96.0, 139.0], [94.0, 184.0]),
        ([104.0, 139.0], [106.0, 184.0]),
        ([112.0, 141.0], [116.0, 184.0]),
    ] {
        fig.fold(&[a, b], 0.14, 0.5);
    }
    for k in 0..9 {
        fig.px(
            119.0 + jr((k as f64 * 0.5).sin() * 1.2),
            137.0 + k as f64,
            GOLD_PX[(k & 1) as usize],
        );
    }
    fig.capsule(119.0, 147.0, 119.0, 153.0, 1.3, 1.3, glass, O::new());
    let drop = raw_material(
        vec![
            [90.0, 20.0, 60.0],
            [170.0, 40.0, 110.0],
            [255.0, 110.0, 190.0],
        ],
        0.0,
        false,
        false,
    );
    fig.capsule(
        119.0,
        150.0,
        119.0,
        153.0,
        0.9,
        0.9,
        &drop,
        O::new().line(false),
    );
    // High black collar with a gold rim and a ruby cross brooch.
    let collar = poly(
        vec![
            [91.0, 75.0],
            [109.0, 75.0],
            [111.0, 85.0],
            [100.0, 87.0],
            [89.0, 85.0],
        ],
        3.0,
    );
    fig.sdf(
        black,
        &*collar,
        [88.0, 74.0, 112.0, 88.0],
        O::new().line_same().cast(1),
    );
    fig.line(91.0, 75.0, 109.0, 75.0, GOLD_PX[1]);
    fig.line(89.0, 85.0, 100.0, 87.0, GOLD_PX[0]);
    fig.line(100.0, 87.0, 111.0, 85.0, GOLD_PX[0]);
    fig.stamp(
        99.0,
        79.0,
        &[".g.", "grg", ".g.", ".r."],
        &[('g', GOLD_PX[0]), ('r', [220.0, 40.0, 70.0])],
        false,
    );

    // Arms: upper arms hang by the ribs; forearms come forward to the work.
    let sh_r: P2 = [75.0, 94.0];
    let grip_r: P2 = [104.0, 124.0];
    let arm_r = arm_field(sh_r, grip_r, [-0.3, 1.0], 28.0, 21.0);
    let sh_l: P2 = [125.0, 94.0];
    let grip_l: P2 = [115.0, 100.0];
    let arm_l = arm_field(sh_l, grip_l, [0.6, 1.0], 28.0, 25.0);

    // Left arm (viewer's right) squeezes the dropper over the flask.
    fig.sdf(
        black,
        &*arm_l.field,
        arm_l.bx,
        O::new()
            .line_same()
            .cast(1)
            .rim_over()
            .joins(vec![[sh_l[0], sh_l[1] + 1.0, 8.5]]),
    );
    fig.fold(
        &[
            add(arm_l.e, mul(perp(arm_l.d), 2.0)),
            add(add(arm_l.e, mul(arm_l.d, 4.0)), mul(perp(arm_l.d), -2.0)),
        ],
        0.22,
        0.5,
    );
    let f = &FLASK;
    let [tx, ty] = f.tip;
    let tube = unit(sub([tx, ty], grip_l));
    fig.capsule(
        grip_l[0] - tube[0] * 4.0,
        grip_l[1] - tube[1] * 4.0,
        tx,
        ty,
        1.5,
        1.0,
        glass,
        O::new(),
    );
    let squeeze = pose.squeeze;
    let bulb = add(grip_l, mul(tube, -7.0));
    fig.ellipse(
        bulb[0],
        bulb[1],
        3.4 - squeeze * 1.0,
        3.2 - squeeze * 0.6,
        &rubber,
        O::new(),
    );
    fist(
        &mut fig,
        grip_l,
        tube,
        arm_l.wrist,
        glove,
        FistOpts {
            size: Some(0.95),
            ..Default::default()
        },
    );
    band_cuff(
        &mut fig,
        arm_l.wrist,
        arm_l.d,
        ivory,
        CuffOpts {
            buttons: Some(GOLD_PX[0]),
            w: Some(4.4),
            ..Default::default()
        },
    );
    shoulder_frill(&mut fig, sh_l, 1.0, ivory);

    // Head: cranium and a tapering jaw, shaded as one dome.
    let face_field = smin(
        3.5,
        vec![
            circle(hx, hy, 15.0, 16.5),
            capsule(hx, hy + 4.0, hx, hy + 15.0, 13.2, 6.2, Some(16.5)),
        ],
    );
    let face = fig.sdf(
        skin,
        &*face_field,
        [hx - 20.0, hy - 17.0, hx + 20.0, hy + 23.0],
        O::new().cast(1).curve(0.8),
    );
    fig.shade_below(face, 2, 0.24);

    // Hime-cut hair: blunt fringe, straight side locks level with the jaw.
    let fringe: Vec<P2> = vec![
        [hx - 17.0, hy + 2.0],
        [hx - 18.0, hy - 7.0],
        [hx - 13.0, hy - 15.0],
        [hx - 6.0, hy - 19.0],
        [hx, hy - 20.0],
        [hx + 6.0, hy - 19.0],
        [hx + 13.0, hy - 15.0],
        [hx + 18.0, hy - 7.0],
        [hx + 17.0, hy + 2.0],
        [hx + 13.0, hy + 1.0],
        [hx + 12.0, hy - 1.0],
        [hx + 6.0, hy - 1.0],
        [hx + 5.0, hy + 1.0],
        [hx - 1.0, hy + 1.0],
        [hx - 2.0, hy - 1.0],
        [hx - 7.0, hy - 1.0],
        [hx - 8.0, hy + 1.0],
        [hx - 13.0, hy + 1.0],
    ];
    fig.poly(
        &fringe,
        hair,
        O::new().shade(Shade::Dome).curve(0.9).cast(3),
    );
    for [x0, y0, x1, y1] in [
        [hx - 4.0, hy - 17.0, hx - 5.0, hy - 3.0],
        [hx + 3.0, hy - 17.0, hx + 3.0, hy - 3.0],
        [hx - 11.0, hy - 13.0, hx - 11.0, hy - 3.0],
        [hx + 10.0, hy - 13.0, hx + 10.0, hy - 3.0],
    ] {
        fig.line(x0, y0, x1, y1, [44.0, 30.0, 76.0]);
    }
    for k in -8..=8 {
        let y = hy - 14.0 + jr((k as f64 / 8.0).powi(2) * 3.0);
        if (k as i32).abs() % 4 != 3 {
            fig.px(
                hx + k as f64,
                y,
                if k < 2 {
                    [128.0, 92.0, 190.0]
                } else {
                    [92.0, 66.0, 146.0]
                },
            );
        }
    }
    for side in [-1.0, 1.0] {
        let lock = poly(
            vec![
                [hx + side * 12.0, hy - 6.0],
                [hx + side * 18.0, hy - 6.0],
                [hx + side * 18.5, hy + 20.0],
                [hx + side * 15.0, hy + 19.0],
                [hx + side * 12.5, hy + 21.0],
            ],
            3.0,
        );
        fig.sdf(
            hair,
            &*lock,
            [hx - 20.0, hy - 8.0, hx + 20.0, hy + 23.0],
            O::new().cast(2).line_same(),
        );
        fig.line(
            hx + side * 15.0,
            hy - 2.0,
            hx + side * 15.0,
            hy + 17.0,
            [70.0, 50.0, 118.0],
        );
    }

    // Nurse cap with the burgundy cross.
    let cap = poly(
        vec![
            [hx - 12.0, hy - 16.0],
            [hx + 10.0, hy - 20.0],
            [hx + 14.0, hy - 11.0],
            [hx - 11.0, hy - 8.0],
        ],
        3.0,
    );
    fig.sdf(
        ivory,
        &*cap,
        [hx - 14.0, hy - 22.0, hx + 16.0, hy - 6.0],
        O::new().cast(2).curve(0.9),
    );
    fig.line(
        hx - 11.0,
        hy - 10.0,
        hx + 13.0,
        hy - 13.0,
        [170.0, 160.0, 172.0],
    );
    fig.stamp(hx - 1.0, hy - 18.0, &CAP_CROSS, CROSS, false);

    // Face details: eyes, glasses worn low, a calm mouth.
    let rows: &[&str] = match pose.eyes {
        "focus" => &EYE_FOCUS,
        "blink" => &EYE_BLINK,
        _ => &EYE_SHUT,
    };
    fig.stamp(hx - 11.0, hy + 3.0, rows, EYE, false);
    fig.stamp(hx + 3.0, hy + 3.0, rows, EYE, true);
    fig.line(hx - 10.0, hy + 1.0, hx - 5.0, hy + 1.0, [40.0, 20.0, 30.0]);
    fig.line(hx + 5.0, hy + 1.0, hx + 10.0, hy + 1.0, [40.0, 20.0, 30.0]);
    let rim: Rgb = [226.0, 172.0, 66.0];
    for cx in [hx - 7.0, hx + 7.0] {
        for a in 0..24 {
            let t = (a as f64 / 24.0) * std::f64::consts::PI * 2.0;
            fig.px(jr(cx + t.cos() * 4.6), jr(hy + 8.0 + t.sin() * 2.6), rim);
        }
    }
    fig.line(hx - 2.0, hy + 7.0, hx + 2.0, hy + 7.0, rim);
    if pose.glint > 0.0 {
        let gx = jr(hx - 11.0 + pose.glint * 8.0);
        fig.px(gx, hy + 7.0, [255.0, 255.0, 255.0]);
        fig.px(gx + 14.0, hy + 7.0, [255.0, 255.0, 255.0]);
    }
    fig.px(hx, hy + 11.0, [72.0, 30.0, 20.0]);
    for bx in [hx - 11.0, hx + 8.0] {
        fig.px(bx, hy + 11.0, [190.0, 96.0, 96.0]);
        fig.px(bx + 2.0, hy + 11.0, [190.0, 96.0, 96.0]);
    }
    if pose.mouth == "smile" {
        fig.stamp(hx - 2.0, hy + 14.0, &MOUTH_SMILE, MOUTH, false);
    } else {
        fig.stamp(hx - 2.0, hy + 15.0, &MOUTH_CALM, MOUTH, false);
    }

    // Right arm (viewer's left) holds the flask by its neck, in front.
    fig.sdf(
        black,
        &*arm_r.field,
        arm_r.bx,
        O::new()
            .line_same()
            .cast(1)
            .rim_over()
            .joins(vec![[sh_r[0], sh_r[1] + 1.0, 8.5]]),
    );
    fig.fold(
        &[
            add(arm_r.e, mul(perp(arm_r.d), -2.0)),
            add(add(arm_r.e, mul(arm_r.d, 4.0)), mul(perp(arm_r.d), 2.0)),
        ],
        0.22,
        0.5,
    );
    let liquid = liquid_material(pose.liquid);
    fig.ellipse(f.x, f.y, f.r, f.r, glass, O::new().round(0.6));
    let surface = f.y + f.r - (f.r * 2.0 - 2.0) * clamp(pose.level, 0.1, 0.95);
    let below = move |_x: i32, y: i32| y as f64 >= surface;
    fig.ellipse(
        f.x,
        f.y,
        f.r - 1.2,
        f.r - 1.2,
        &liquid,
        O::new().line(false).where_(&below),
    );
    fig.capsule(
        f.x,
        f.y - f.r + 1.0,
        f.x,
        f.neck_top,
        2.6,
        2.6,
        glass,
        O::new(),
    );
    fig.ellipse(f.x, f.neck_top, 3.6, 1.3, glass, O::new());
    for (dx, dy) in [(-5.0, -5.0), (-6.0, -3.0), (-4.0, -7.0), (-2.0, -8.0)] {
        fig.px(f.x + dx, f.y + dy, [255.0, 255.0, 255.0]);
    }
    fist(
        &mut fig,
        grip_r,
        [0.0, 1.0],
        arm_r.wrist,
        glove,
        FistOpts {
            size: Some(0.95),
            ..Default::default()
        },
    );
    band_cuff(
        &mut fig,
        arm_r.wrist,
        arm_r.d,
        ivory,
        CuffOpts {
            buttons: Some(GOLD_PX[0]),
            w: Some(4.4),
            ..Default::default()
        },
    );
    shoulder_frill(&mut fig, sh_r, -1.0, ivory);
    let _ = gem_red;
    fig.render()
}
