//! Chiquita's comments, as a comic's speech balloons: each pops out of her with a tail pointing at
//! her mouth and its words typing out as fast as her mouth moves, then drifts up and fades on its
//! own; the newest hangs from her, the older ones stack above it, smaller and dimmer. The
//! balloon's shape says how she says it: rounded when she talks, spiky when she is cross, a cloud
//! when she mutters to herself, wobbling pink when she is flustered, a burst when she shouts,
//! sparkling when she is pleased. When she is out in 3D they come from her; without her model,
//! from a little medallion with her face on the right. Nothing waits for a key, and nothing here
//! can be clicked.
use std::f32::consts::TAU;

use super::persona::*;
use super::ui_paint::*;
use super::*;

/// One comment in the air.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct FloatLine {
    pub text: String,
    /// Since it was said, and how long it stays (ms).
    pub age_ms: u32,
    pub life_ms: u32,
    pub portrait: Option<PortraitRef>,
    /// Her flustered face.
    pub jealous: bool,
    /// How she says it: 0 talking, 1 cross, 2 muttering, 3 flustered, 4 shouting, 5 pleased.
    pub mood: u8,
    /// How long its words take to come out (ms): as long as her mouth moves.
    pub reveal_ms: u32,
}

type Uv = ([f32; 2], [f32; 2]);

/// Her colour, the balloons' paper and ink.
const LILAC_CHQ: u32 = 0xd9a6ff;
const PLUM: u32 = 0x7b3fb8;
const PAPER: u32 = 0xfffaff;
const INK: u32 = 0x2a1240;

/// Where her medallion sits (the mascot's place), under the corner map, on the right.
pub(super) fn anchor(w: f32, h: f32) -> [f32; 2] {
    let s = h / 1080.0;
    [w - 86.0 * s, 772.0 * s]
}

/// Scales everything `paint` adds by `k` about `pivot` (a pop as it comes in). Each shape is drawn
/// from its own local coordinates, so it scales whole and keeps its edges.
fn scaled(
    v: &mut Vec<MapVertex>,
    pivot: [f32; 2],
    k: f32,
    paint: impl FnOnce(&mut Vec<MapVertex>),
) {
    let start = v.len();
    paint(v);
    if (k - 1.0).abs() < 1e-4 {
        return;
    }
    for vertex in &mut v[start..] {
        vertex.position[0] = pivot[0] + (vertex.position[0] - pivot[0]) * k;
        vertex.position[1] = pivot[1] + (vertex.position[1] - pivot[1]) * k;
    }
}

/// 0 → 1 overshooting by `over` before settling (a pop).
fn pop(t: f32, over: f32) -> f32 {
    let t = t.clamp(0.0, 1.0) - 1.0;
    1.0 + (over + 1.0) * t * t * t + over * t * t
}

/// `over`: her feet and how tall she stands on the screen when she is out in 3D (the balloons come
/// from her head); `None`: from her medallion.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    floats: &[FloatLine],
    faces: &[Option<Uv>],
    over: Option<([f32; 2], f32)>,
    time: f32,
) {
    if floats.is_empty() {
        return;
    }
    let s = h / 1080.0;
    let (mouth, top) = match over {
        Some((feet, tall)) => {
            // Her mouth, a little to the side her face is on, and the top of her head.
            (
                [feet[0] + tall * 0.05, feet[1] - tall * 0.62],
                [feet[0], feet[1] - tall * 0.99],
            )
        }
        None => {
            let at = anchor(w, h);
            medallion(v, at, floats, faces, time, s);
            (
                [at[0] - 20.0 * s, at[1] - 40.0 * s],
                [at[0], at[1] - 66.0 * s],
            )
        }
    };
    balloons(font, v, w, h, floats, mouth, top, time, s);
}

/// Her face in a little medallion with horns, when her 3D self is not out.
fn medallion(
    v: &mut Vec<MapVertex>,
    at: [f32; 2],
    floats: &[FloatLine],
    faces: &[Option<Uv>],
    time: f32,
    s: f32,
) {
    let newest = floats.last().expect("a float");
    let shown = floats
        .iter()
        .map(|f| {
            let age = f.age_ms as f32 / 1000.0;
            let left = (f.life_ms as f32 - f.age_ms as f32) / 1000.0;
            (age / 0.25).min(left / 0.4).clamp(0.0, 1.0)
        })
        .fold(0.0, f32::max);
    let bob = (time * 2.2).sin() * 4.0 * s;
    let m = [at[0], at[1] + bob];
    let r = 46.0 * s;
    halo(v, m, r, 22.0 * s, rgba(LILAC_CHQ, 0.4 * shown));
    let her_face = faces.last().copied().flatten();
    if her_face.is_none() {
        horns(v, [m[0], m[1] - r * 0.8], r * 1.6, GOLD, shown);
    }
    panel(
        v,
        Rect::new(m[0] - r, m[1] - r, r * 2.0, r * 2.0),
        r,
        Panel::fill(0x3a2058, 0x1c0e2e, shown)
            .rim(LILAC_CHQ, shown, 3.0 * s)
            .bevel(0.8),
    );
    match her_face {
        Some(uv) => face(
            v,
            Rect::new(
                m[0] - r + 5.0 * s,
                m[1] - r + 5.0 * s,
                r * 2.0 - 10.0 * s,
                r * 2.0 - 10.0 * s,
            ),
            r - 5.0 * s,
            uv,
            shown,
        ),
        None => sparkle(v, m, r * 0.5, rgba(0xffffff, shown), 0.8),
    }
    if newest.jealous {
        gem_turned(
            v,
            [m[0] + r * 0.72, m[1] - r * 0.62],
            5.0 * s,
            9.0 * s,
            0.4,
            0x9ad8ff,
            0xffffff,
            shown,
            0.0,
        );
    }
}

/// How a balloon is set: its face and size of type, its paper and ink, its rounding.
struct Look {
    face: Face,
    size: f32,
    paper: u32,
    ink: u32,
    words: u32,
}

fn look(mood: u8, s: f32) -> Look {
    match mood {
        1 => Look {
            face: Face::Body,
            size: 25.0 * s,
            paper: 0xfff0f5,
            ink: 0x5a0c2c,
            words: 0x4a0a26,
        },
        2 => Look {
            face: Face::Italic,
            size: 22.0 * s,
            paper: 0xefe9f7,
            ink: 0x5b4a78,
            words: 0x4a3d66,
        },
        3 => Look {
            face: Face::Body,
            size: 25.0 * s,
            paper: 0xfff0f8,
            ink: 0x8a2a6a,
            words: 0x5a1846,
        },
        4 => Look {
            face: Face::Title,
            size: 31.0 * s,
            paper: PAPER,
            ink: INK,
            words: INK,
        },
        5 => Look {
            face: Face::Body,
            size: 25.0 * s,
            paper: 0xfffbea,
            ink: 0x6a4a10,
            words: 0x3a2a08,
        },
        _ => Look {
            face: Face::Body,
            size: 25.0 * s,
            paper: PAPER,
            ink: INK,
            words: INK,
        },
    }
}

/// The balloons, the newest hanging from her (its tail at her mouth), the older above it.
#[allow(clippy::too_many_arguments)]
fn balloons(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    floats: &[FloatLine],
    mouth: [f32; 2],
    top: [f32; 2],
    time: f32,
    s: f32,
) {
    let max_width = 440.0 * s;
    let margin = 18.0 * s;
    // Under the top of the screen there must be room for the newest: otherwise it goes beside her.
    let beside = top[1] < 230.0 * s;
    let mut bottom = if beside {
        mouth[1] + 40.0 * s
    } else {
        top[1] - 34.0 * s
    };
    let n = floats.len();
    for (i, f) in floats.iter().enumerate().rev() {
        let newest = i + 1 == n;
        let age = f.age_ms as f32 / 1000.0;
        let left = (f.life_ms as f32 - f.age_ms as f32) / 1000.0;
        let alpha = (left / 0.45).clamp(0.0, 1.0) * if newest { 1.0 } else { 0.82 };
        if alpha <= 0.0 {
            continue;
        }
        let k = if newest { 1.0 } else { 0.84 };
        let lk = look(f.mood, s * k);
        let pad = [28.0 * s * k, 22.0 * s * k];
        let lines = wrap(
            font,
            lk.face,
            &f.text,
            lk.size,
            max_width * k - pad[0] * 2.0,
        );
        let text_w = lines
            .iter()
            .map(|l| text_width(font, lk.face, l, lk.size))
            .fold(0.0, f32::max);
        let line_h = lk.size * 1.32;
        let bw = (text_w + pad[0] * 2.0).max(150.0 * s * k);
        let bh = lines.len() as f32 * line_h + pad[1] * 2.0;
        // Shouts are set in a burst, which needs more room round the words.
        let (bw, bh) = if f.mood == 4 {
            (bw * 1.22, bh * 1.35)
        } else {
            (bw, bh)
        };
        let rise = (1.0 - (left / 0.45).clamp(0.0, 1.0)) * 22.0 * s;
        let drift = if newest {
            0.0
        } else {
            (time * 1.4 + i as f32 * 1.7).sin() * 3.0 * s
        };
        let x = if beside {
            if mouth[0] > w * 0.5 {
                mouth[0] - 120.0 * s - bw
            } else {
                mouth[0] + 120.0 * s
            }
        } else {
            mouth[0] - bw * 0.45
        };
        let x = x.clamp(margin, (w - margin - bw).max(margin));
        let y = (bottom - bh - rise + drift).max(margin);
        let r = Rect::new(x, y, bw, bh);
        bottom = r.y - 18.0 * s;
        // Where its tail points: her mouth for the newest; nowhere for the older.
        let tip = newest.then_some(mouth);
        // Popping out of her (from the tip of its tail), a shake when she is cross.
        let grow = if newest {
            pop(age / 0.24, if f.mood == 4 { 2.2 } else { 1.4 }).max(0.05)
        } else {
            1.0
        };
        let pivot = tip.unwrap_or([r.x + r.w * 0.5, r.y + r.h]);
        let shake = if f.mood == 1 || f.mood == 4 {
            (age * 46.0).sin() * 6.0 * s * (-age * 4.0).exp()
        } else {
            0.0
        };
        let wobble = if f.mood == 3 {
            (time * 6.5).sin() * 0.022
        } else {
            0.0
        };
        faded(v, alpha, |v| {
            scaled(v, pivot, grow, |v| {
                shifted(v, [shake, 0.0], |v| {
                    turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], wobble, |v| {
                        balloon(
                            font,
                            v,
                            r,
                            tip,
                            f,
                            &lk,
                            &lines,
                            line_h,
                            pad,
                            age,
                            time,
                            s * k,
                        );
                    });
                });
            });
        });
        if r.y <= margin + 1.0 && !newest {
            break;
        }
        let _ = h;
    }
}

/// One balloon: its shadow, its tail, its shape by mood, her name on a tab, and its words typing out.
#[allow(clippy::too_many_arguments)]
fn balloon(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    tip: Option<[f32; 2]>,
    f: &FloatLine,
    lk: &Look,
    lines: &[String],
    line_h: f32,
    pad: [f32; 2],
    age: f32,
    time: f32,
    s: f32,
) {
    let edge = 4.0 * s;
    let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
    soft(
        v,
        Rect::new(r.x + 8.0 * s, r.y + 12.0 * s, r.w, r.h),
        24.0 * s,
        12.0 * s,
        rgba(SHADOW, 0.55),
        false,
    );
    // The tail's root, on the side of the balloon nearest her.
    let tail = tip.map(|t| {
        let below = t[1] > r.y + r.h;
        let root = if below {
            [
                (t[0]).clamp(r.x + 40.0 * s, r.x + r.w - 40.0 * s),
                r.y + r.h - 6.0 * s,
            ]
        } else if t[0] > r.x + r.w {
            [
                r.x + r.w - 6.0 * s,
                (t[1]).clamp(r.y + 30.0 * s, r.y + r.h - 30.0 * s),
            ]
        } else {
            [
                r.x + 6.0 * s,
                (t[1]).clamp(r.y + 30.0 * s, r.y + r.h - 30.0 * s),
            ]
        };
        (root, t, below)
    });
    let paper = rgba(lk.paper, 1.0);
    let ink = rgba(lk.ink, 1.0);
    let tail_tri = |v: &mut Vec<MapVertex>, grow: f32, color: [f32; 4]| {
        if let Some((root, tip, below)) = tail {
            let width = 15.0 * s + grow;
            let (a, b) = if below {
                (
                    [root[0] - width, root[1] - grow],
                    [root[0] + width, root[1] - grow],
                )
            } else {
                ([root[0], root[1] - width], [root[0], root[1] + width])
            };
            // The tip stops a little short of her, so it points rather than touches.
            let d = [tip[0] - root[0], tip[1] - root[1]];
            let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1.0);
            let reach = (l - 16.0 * s).max(20.0 * s).min(170.0 * s);
            let end = [
                root[0] + d[0] / l * (reach + grow * 1.6),
                root[1] + d[1] / l * (reach + grow * 1.6),
            ];
            push_triangle(v, [a, b, end], color);
        }
    };
    match f.mood {
        // Muttering: a cloud, and a trail of little bubbles toward her instead of a tail.
        2 => {
            cloud(v, r, 17.0 * s, edge, ink);
            cloud(v, r, 17.0 * s, 0.0, paper);
            if let Some((root, tip, _)) = tail {
                for (k, rad) in [(0.3f32, 9.0f32), (0.55, 6.5), (0.78, 4.5)] {
                    let p = [
                        root[0] + (tip[0] - root[0]) * k,
                        root[1] + (tip[1] - root[1]) * k,
                    ];
                    disc(v, p, (rad + 2.5) * s, ink);
                    disc(v, p, rad * s, paper);
                }
            }
        }
        // Shouting: a burst round the words.
        4 => {
            tail_tri(v, edge * 1.5, ink);
            star(
                v,
                c,
                r.w * 0.5,
                r.h * 0.5,
                13,
                26.0 * s,
                edge * 1.5,
                time,
                ink,
            );
            tail_tri(v, 0.0, paper);
            star(v, c, r.w * 0.5, r.h * 0.5, 13, 26.0 * s, 0.0, time, paper);
        }
        // Cross: spikes all round the edge.
        1 => {
            tail_tri(v, edge, ink);
            spikes(v, r, 16.0 * s, 26.0 * s, rgba(lk.ink, 1.0), age);
            panel(
                v,
                r,
                18.0 * s,
                Panel::fill(lk.paper, mix(lk.paper, 0xffd6e4, 0.6), 1.0).rim(lk.ink, 1.0, edge),
            );
            tail_tri(v, -edge * 0.2, paper);
            // Anger marks in the corner.
            vein(
                v,
                [r.x + r.w - 14.0 * s, r.y + 10.0 * s],
                13.0 * s,
                rgba(0xe0245e, 1.0),
            );
        }
        _ => {
            tail_tri(v, edge, ink);
            let bottom = match f.mood {
                3 => mix(lk.paper, 0xffc8e4, 0.55),
                5 => mix(lk.paper, 0xfff0b8, 0.6),
                _ => mix(lk.paper, 0xe8dcff, 0.55),
            };
            panel(
                v,
                r,
                26.0 * s,
                Panel::fill(lk.paper, bottom, 1.0).rim(lk.ink, 1.0, edge),
            );
            tail_tri(v, -edge * 0.2, paper);
        }
    }
    // Her name on a tab over the top-left corner.
    let name = "Chiquita";
    let ns = 17.0 * s;
    let tab = Rect::new(
        r.x + 18.0 * s,
        r.y - 15.0 * s,
        text_width(font, Face::Title, name, ns) + 26.0 * s,
        28.0 * s,
    );
    panel(v, grow(tab, 2.5 * s), 9.0 * s, Panel::fill(INK, INK, 1.0));
    panel(
        v,
        tab,
        7.0 * s,
        Panel::fill(mix(LILAC_CHQ, 0xffffff, 0.1), PLUM, 1.0),
    );
    text(
        font,
        v,
        Face::Title,
        name,
        tab.x + 13.0 * s,
        tab.y + 4.0 * s,
        ns,
        rgba(0xffffff, 1.0),
    );
    // The words, typing out as she says them (laid out whole, so lines do not jump).
    let total: usize = lines.iter().map(|l| l.chars().count()).sum();
    let reveal = (f.reveal_ms as f32 / 1000.0).max(0.2);
    let mut shown = ((age / reveal).clamp(0.0, 1.0) * total as f32).ceil() as usize;
    let words_h = lines.len() as f32 * line_h;
    let y0 = r.y + (r.h - words_h) * 0.5 + 2.0 * s;
    for (k, line) in lines.iter().enumerate() {
        if shown == 0 {
            break;
        }
        let count = line.chars().count();
        let part: String = line.chars().take(shown).collect();
        shown = shown.saturating_sub(count);
        let lw = text_width(font, lk.face, line, lk.size);
        let lx = if f.mood == 4 || f.mood == 2 {
            r.x + (r.w - lw) * 0.5
        } else {
            r.x + pad[0]
        };
        text(
            font,
            v,
            lk.face,
            &part,
            lx,
            y0 + k as f32 * line_h,
            lk.size,
            rgba(lk.words, 1.0),
        );
    }
    // Flustered: little hearts popping up; pleased: sparkles.
    if f.mood == 3 {
        for (k, (dx, ph)) in [(0.15f32, 0.0f32), (0.88, 0.45), (0.62, 0.8)]
            .iter()
            .enumerate()
        {
            let t = ((time * 0.9 + ph) % 1.0) as f32;
            let p = [r.x + r.w * dx, r.y - 6.0 * s - t * 34.0 * s];
            heart(
                v,
                p,
                (7.0 + 2.0 * k as f32) * s,
                rgba(0xff5fa8, (1.0 - t) * 0.95),
            );
        }
    }
    if f.mood == 5 {
        for (dx, dy, ph) in [
            (-0.02f32, 0.2f32, 0.0f32),
            (1.02, 0.35, 0.33),
            (0.85, -0.12, 0.66),
        ] {
            let tw = 0.5 + 0.5 * ((time * 5.0 + ph * TAU).sin());
            sparkle(
                v,
                [r.x + r.w * dx, r.y + r.h * dy],
                (8.0 + 6.0 * tw) * s,
                rgba(0xffe27a, 0.95),
                0.7,
            );
        }
    }
}

/// A cloud: a box with round puffs all round its edge (`grow`: outward, for its ink).
fn cloud(v: &mut Vec<MapVertex>, r: Rect, puff: f32, grow: f32, color: [f32; 4]) {
    let fill = Panel {
        top: color,
        bottom: color,
        ..Panel::fill(0, 0, 1.0)
    };
    panel(
        v,
        Rect::new(r.x - grow, r.y - grow, r.w + grow * 2.0, r.h + grow * 2.0),
        puff,
        fill,
    );
    let step = puff * 1.5;
    let along = |len: f32| ((len / step).floor() as usize).max(2);
    let nx = along(r.w);
    let ny = along(r.h);
    for k in 0..=nx {
        let x = r.x + r.w * k as f32 / nx as f32;
        disc(v, [x, r.y], puff + grow, color);
        disc(v, [x, r.y + r.h], puff + grow, color);
    }
    for k in 1..ny {
        let y = r.y + r.h * k as f32 / ny as f32;
        disc(v, [r.x, y], puff + grow, color);
        disc(v, [r.x + r.w, y], puff + grow, color);
    }
}

/// Spikes all round a box's edge, sharp and uneven (they settle as it comes in).
fn spikes(v: &mut Vec<MapVertex>, r: Rect, depth: f32, step: f32, color: [f32; 4], age: f32) {
    let jag = |k: usize| 0.65 + 0.35 * ((k as f32 * 2.3).sin() * 0.5 + 0.5);
    let grow = 1.0 + 0.5 * (-age * 5.0).exp();
    let mut k = 0usize;
    let mut edge = |v: &mut Vec<MapVertex>, a: [f32; 2], b: [f32; 2], out: [f32; 2]| {
        let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = ((len / step).floor() as usize).max(1);
        for i in 0..n {
            let t0 = i as f32 / n as f32;
            let t1 = (i + 1) as f32 / n as f32;
            let p0 = [a[0] + (b[0] - a[0]) * t0, a[1] + (b[1] - a[1]) * t0];
            let p1 = [a[0] + (b[0] - a[0]) * t1, a[1] + (b[1] - a[1]) * t1];
            let m = [(p0[0] + p1[0]) * 0.5, (p0[1] + p1[1]) * 0.5];
            let d = depth * jag(k) * grow;
            push_triangle(v, [p0, p1, [m[0] + out[0] * d, m[1] + out[1] * d]], color);
            k += 1;
        }
    };
    let inset = 10.0;
    edge(v, [r.x + inset, r.y], [r.x + r.w - inset, r.y], [0.0, -1.0]);
    edge(
        v,
        [r.x + r.w, r.y + inset],
        [r.x + r.w, r.y + r.h - inset],
        [1.0, 0.0],
    );
    edge(
        v,
        [r.x + r.w - inset, r.y + r.h],
        [r.x + inset, r.y + r.h],
        [0.0, 1.0],
    );
    edge(v, [r.x, r.y + r.h - inset], [r.x, r.y + inset], [-1.0, 0.0]);
    // The corners get a spike each, out along the diagonal.
    for (cx, cy, ox, oy) in [
        (r.x, r.y, -1.0f32, -1.0f32),
        (r.x + r.w, r.y, 1.0, -1.0),
        (r.x + r.w, r.y + r.h, 1.0, 1.0),
        (r.x, r.y + r.h, -1.0, 1.0),
    ] {
        let d = depth * 1.15 * grow;
        push_triangle(
            v,
            [
                [cx - ox * 14.0, cy],
                [cx, cy - oy * 14.0],
                [cx + ox * d * 0.75, cy + oy * d * 0.75],
            ],
            color,
        );
    }
}

/// A star round an ellipse (a shout's burst), its points of uneven length, flickering.
#[allow(clippy::too_many_arguments)]
fn star(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    rx: f32,
    ry: f32,
    points: usize,
    depth: f32,
    grow: f32,
    time: f32,
    color: [f32; 4],
) {
    let n = points * 2;
    let flick = (time * 9.0).floor();
    let at = |k: usize| {
        let a = k as f32 / n as f32 * TAU - std::f32::consts::FRAC_PI_2;
        let jag = 0.7 + 0.3 * (((k as f32 + flick) * 2.7).sin() * 0.5 + 0.5);
        let (ex, ey) = if k % 2 == 0 {
            (rx + depth * jag, ry + depth * jag)
        } else {
            (rx * 0.97, ry * 0.97)
        };
        [c[0] + a.cos() * (ex + grow), c[1] + a.sin() * (ey + grow)]
    };
    for k in 0..n {
        push_triangle(v, [c, at(k), at((k + 1) % n)], color);
    }
}

/// A heart (two discs and a point).
fn heart(v: &mut Vec<MapVertex>, c: [f32; 2], k: f32, color: [f32; 4]) {
    disc(v, [c[0] - k * 0.5, c[1] - k * 0.2], k * 0.55, color);
    disc(v, [c[0] + k * 0.5, c[1] - k * 0.2], k * 0.55, color);
    push_triangle(
        v,
        [
            [c[0] - k * 1.02, c[1] - k * 0.1],
            [c[0] + k * 1.02, c[1] - k * 0.1],
            [c[0], c[1] + k * 0.95],
        ],
        color,
    );
}

/// A cross-vein anger mark: four little curved strokes about a point.
fn vein(v: &mut Vec<MapVertex>, c: [f32; 2], k: f32, color: [f32; 4]) {
    for q in 0..4 {
        let a = q as f32 * std::f32::consts::FRAC_PI_2 + std::f32::consts::FRAC_PI_4;
        let (s, co) = a.sin_cos();
        let p0 = [c[0] + co * k * 0.25, c[1] + s * k * 0.25];
        let p1 = [
            c[0] + co * k * 0.95 + -s * k * 0.25,
            c[1] + s * k * 0.95 + co * k * 0.25,
        ];
        wire(v, p0, p1, k * 0.22, color, color, 0.0, true);
    }
}
