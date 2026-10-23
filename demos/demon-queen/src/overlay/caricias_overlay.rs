//! Caricias on screen: her room and her art as pictures of the stage's pass (her body as a bent
//! mesh, her face crossing over on its own bent patch), and over them the night's HUD in the
//! manner of the web page it comes from: her name, desire and trust, the candle, the six hands, the
//! garments' handles, what her body is called under the hand, her words, the hearts and rings,
//! the pause and the end's card. The final clip plays as the scene's picture. Authored at
//! 1920×1080.
use std::f32::consts::TAU;

use super::persona::*;
use super::persona_screens::slanted;
use super::stage_overlay::{PictureInfo, PictureRun};
use super::ui_paint::*;
use super::*;

/// The end's card.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CariciasCard {
    pub title: String,
    pub quote: String,
    pub stats: Vec<(String, String)>,
    pub clip: bool,
}

/// The night as the overlay draws it (positions in 1080p pixels × 4).
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
pub struct CariciasView {
    pub name: String,
    pub title: String,
    pub accent: u32,
    pub bg: String,
    pub art: String,
    pub preload: Vec<String>,
    pub mesh: Vec<[i32; 2]>,
    pub cols: u16,
    pub rows: u16,
    pub face_a: String,
    pub face_b: String,
    pub face_t: u32,
    pub face_mesh: Vec<[i32; 2]>,
    pub face_n: u16,
    /// Desire and trust (‰), the candle's time left (ms).
    pub deseo: u32,
    pub conf: u32,
    pub time_ms: u32,
    pub tool: u8,
    pub rhythm: String,
    pub hint: String,
    pub mouse: [i32; 2],
    /// The second hand's x (two hands).
    pub mirror: Option<i32>,
    /// Her line and how much of it is left to fade (‰).
    pub line: Option<(String, u32)>,
    /// The handles: x, y, direction (×100), ready to come off, ms since it became ready.
    pub handles: Vec<(i32, i32, i32, i32, bool, u32)>,
    /// Hearts (0), rings (1), sweat (2), «!» (3): kind, x, y, scale %, life ‰.
    pub fx: Vec<(u8, i32, i32, u16, u16)>,
    /// Her warmth over the room (‰ of the strongest), the cold of lost trust (‰).
    pub tint: u32,
    pub cold: u32,
    pub toast: Option<(String, u32)>,
    /// The pause, and which choice is lit.
    pub paused: Option<u8>,
    pub card: Option<CariciasCard>,
    /// The final clip plays (the scene's picture).
    pub reel: bool,
    pub playing: bool,
    pub t_ms: u32,
    /// The pace under the hand: how it reads (see `Caricias::pace`) and the needle (‰).
    pub pace: (u8, u16),
    /// Each garment: the desire it asks (‰), whether it is off, ms since it became ready.
    pub thresholds: Vec<(u16, bool, Option<u32>)>,
}

const MODE_CUTOUT: f32 = 15.0;

fn at(view_x: i32, view_y: i32, width: f32, height: f32) -> [f32; 2] {
    [
        view_x as f32 / 4.0 * width / 1920.0,
        view_y as f32 / 4.0 * height / 1080.0,
    ]
}

fn tri_uv(run: &mut Vec<MapVertex>, p: [[f32; 2]; 3], uv: [[f32; 2]; 3], tint: [f32; 4]) {
    for i in 0..3 {
        run.push(MapVertex {
            position: p[i],
            uv: uv[i],
            color: tint,
            text: MODE_CUTOUT,
            ..MapVertex::BLANK
        });
    }
}

/// A grid of screen points laid over a picture's uv (0..1 both ways).
fn grid_run(
    run: &mut Vec<MapVertex>,
    pts: &[[i32; 2]],
    cols: usize,
    rows: usize,
    width: f32,
    height: f32,
    tint: [f32; 4],
) {
    if pts.len() < (cols + 1) * (rows + 1) {
        return;
    }
    let p = |i: usize, j: usize| {
        let q = pts[j * (cols + 1) + i];
        at(q[0], q[1], width, height)
    };
    let uv = |i: usize, j: usize| [i as f32 / cols as f32, j as f32 / rows as f32];
    for j in 0..rows {
        for i in 0..cols {
            tri_uv(
                run,
                [p(i, j), p(i + 1, j), p(i + 1, j + 1)],
                [uv(i, j), uv(i + 1, j), uv(i + 1, j + 1)],
                tint,
            );
            tri_uv(
                run,
                [p(i, j), p(i + 1, j + 1), p(i, j + 1)],
                [uv(i, j), uv(i + 1, j + 1), uv(i, j + 1)],
                tint,
            );
        }
    }
}

/// Her room and her body (under the HUD), as the stage's pictures.
pub(crate) fn pictures(
    view: &CariciasView,
    info: &dyn Fn(&str) -> Option<PictureInfo>,
    width: f32,
    height: f32,
) -> Vec<PictureRun> {
    let mut runs = Vec::new();
    if view.reel {
        return runs;
    }
    let r = super::super::caricias::art_rect();
    let (sx, sy) = (width / 1920.0, height / 1080.0);
    if info(&view.bg).is_some() {
        let mut v = Vec::new();
        let (x0, y0, x1, y1) = (r[0] * sx, r[1] * sy, (r[0] + r[2]) * sx, (r[1] + r[3]) * sy);
        tri_uv(
            &mut v,
            [[x0, y0], [x1, y0], [x1, y1]],
            [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [1.0; 4],
        );
        tri_uv(
            &mut v,
            [[x0, y0], [x1, y1], [x0, y1]],
            [[0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            [1.0; 4],
        );
        runs.push(PictureRun {
            image: view.bg.clone(),
            over: false,
            vertices: v,
        });
    }
    if !view.art.is_empty() && info(&view.art).is_some() {
        let mut v = Vec::new();
        grid_run(
            &mut v,
            &view.mesh,
            view.cols as usize,
            view.rows as usize,
            width,
            height,
            [1.0; 4],
        );
        runs.push(PictureRun {
            image: view.art.clone(),
            over: false,
            vertices: v,
        });
    }
    // Her face: the old one, the new one over it as it comes.
    let n = view.face_n as usize;
    let t = view.face_t as f32 / 1000.0;
    for (face, alpha) in [(&view.face_a, 1.0), (&view.face_b, t)] {
        if face.is_empty() || alpha <= 0.01 || info(face).is_none() {
            continue;
        }
        let mut v = Vec::new();
        grid_run(
            &mut v,
            &view.face_mesh,
            n,
            n,
            width,
            height,
            [1.0, 1.0, 1.0, alpha],
        );
        runs.push(PictureRun {
            image: face.clone(),
            over: false,
            vertices: v,
        });
    }
    runs
}

/// The names of the pictures on show now (the rest load ahead).
pub(crate) fn shown(view: &CariciasView) -> Vec<&str> {
    let mut out = vec![view.bg.as_str(), view.art.as_str()];
    out.extend([view.face_a.as_str(), view.face_b.as_str()]);
    out.retain(|s| !s.is_empty());
    out
}

fn heart(v: &mut Vec<MapVertex>, c: [f32; 2], s: f32, color: [f32; 4]) {
    disc(v, [c[0] - s * 0.5, c[1] - s * 0.15], s * 0.55, color);
    disc(v, [c[0] + s * 0.5, c[1] - s * 0.15], s * 0.55, color);
    push_triangle(
        v,
        [
            [c[0] - s * 1.02, c[1]],
            [c[0] + s * 1.02, c[1]],
            [c[0], c[1] + s * 1.15],
        ],
        color,
    );
}

fn ring(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, width: f32, color: [f32; 4]) {
    let pts: Vec<[f32; 2]> = (0..=32)
        .map(|k| {
            let a = k as f32 / 32.0 * TAU;
            [c[0] + a.cos() * r, c[1] + a.sin() * r]
        })
        .collect();
    for w in pts.windows(2) {
        wire(v, w[0], w[1], width, color, color, 0.0, false);
    }
}

/// A hand where the pointer is (the tool's shape, simply).
fn hand(v: &mut Vec<MapVertex>, c: [f32; 2], tool: u8, accent: u32) {
    let skin = rgba(0xc9a6e8, 1.0);
    let ink = rgba(0x1a1026, 1.0);
    match tool {
        5 => {
            heart(v, [c[0], c[1]], 17.0, rgba(0xffffff, 1.0));
            heart(v, [c[0], c[1]], 14.0, rgba(0xff4f8a, 1.0));
        }
        _ => {
            disc(v, [c[0], c[1] + 6.0], 21.0, ink);
            disc(v, [c[0], c[1] + 6.0], 18.0, skin);
            let fingers: &[(f32, f32)] = match tool {
                1 => &[(0.0, -26.0)],
                2 => &[(-12.0, -14.0), (-4.0, -18.0), (5.0, -18.0), (13.0, -14.0)],
                3 => &[(-6.0, -20.0), (6.0, -20.0)],
                _ => &[(-13.0, -20.0), (-4.0, -26.0), (5.0, -26.0), (14.0, -20.0)],
            };
            for (dx, dy) in fingers {
                wire(
                    v,
                    [c[0] + dx * 0.5, c[1]],
                    [c[0] + dx, c[1] + dy],
                    11.0,
                    ink,
                    ink,
                    0.0,
                    false,
                );
                wire(
                    v,
                    [c[0] + dx * 0.5, c[1]],
                    [c[0] + dx, c[1] + dy],
                    7.0,
                    skin,
                    skin,
                    0.0,
                    false,
                );
            }
            if tool == 3 {
                disc(v, [c[0], c[1] - 22.0], 4.0, rgba(0xff4f8a, 1.0));
            }
            let _ = accent;
        }
    }
}

// ------------------------------------------------------------------ its own Persona look
//
// (2026-10-23, the user: the shop's look is the shop's alone.) Flat cut-paper shapes with hard
// edges: black, white and her colour; solid offset shadows instead of soft ones; tilted condensed
// letters with a black rim; no gradients, no rounded corners, no glows.

/// The ink and the paper.
const EDGE: u32 = 0x0a0610;
const PAPER: u32 = 0xfbf7f0;
const MINT: u32 = 0x5affc8;

/// How the pace under the hand reads, and its colour.
fn pace_look(kind: u8, accent: u32) -> (&'static str, u32) {
    match kind {
        1 => ("MUY QUIETO", 0x9aa4c8),
        2 => ("¡BUEN RITMO!", MINT),
        3 => ("ALGO RÁPIDO", 0xffd45a),
        4 => ("¡DEMASIADO RÁPIDO!", 0xff5c6c),
        5 => ("APRETANDO", accent),
        _ => ("AÚN NO", 0x8a82a8),
    }
}

/// Black or paper, whichever reads on `col`.
fn ink_on(col: u32) -> u32 {
    let c = rgba(col, 1.0);
    if 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2] > 0.3 {
        EDGE
    } else {
        PAPER
    }
}

/// A flat convex shape.
fn poly(v: &mut Vec<MapVertex>, pts: &[[f32; 2]], color: [f32; 4]) {
    for i in 1..pts.len().saturating_sub(1) {
        push_triangle(v, [pts[0], pts[i], pts[i + 1]], color);
    }
    push_outline_fringe(v, pts, color);
}

fn moved(pts: &[[f32; 2]], d: [f32; 2]) -> Vec<[f32; 2]> {
    pts.iter().map(|p| [p[0] + d[0], p[1] + d[1]]).collect()
}

/// A cut-paper shape: its hard shadow `drop` away, its face, a white edge `edge` wide.
fn cut(
    v: &mut Vec<MapVertex>,
    pts: &[[f32; 2]],
    face: [f32; 4],
    edge: f32,
    drop: Option<([f32; 2], u32)>,
) {
    if let Some((d, col)) = drop {
        poly(v, &moved(pts, d), rgba(col, face[3].min(1.0)));
    }
    poly(v, pts, face);
    if edge > 0.0 {
        let e = rgba(PAPER, face[3].min(1.0));
        for i in 0..pts.len() {
            wire(v, pts[i], pts[(i + 1) % pts.len()], edge, e, e, 0.0, false);
        }
    }
}

/// Letters with a black rim (Anton), as the stage's stamps.
#[allow(clippy::too_many_arguments)]
fn inked(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    words: &str,
    x: f32,
    y: f32,
    size: f32,
    color: u32,
    rim_w: f32,
    alpha: f32,
) {
    text_rimmed(
        font,
        v,
        Face::Display,
        words,
        x,
        y,
        size,
        rgba(color, alpha),
        rim_w,
        rgba(EDGE, alpha),
    );
}

/// A tag: a slanted strip of `col` with dark words, a black shadow, turned a little.
#[allow(clippy::too_many_arguments)]
fn tag(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    words: &str,
    size: f32,
    col: u32,
    ink: u32,
    s: f32,
) {
    let w = text_width(font, Face::Display, words, size) + 36.0 * s;
    let h = size + 16.0 * s;
    let q = slanted(x, y, w, h, 8.0 * s);
    turned(v, [x + w * 0.5, y + h * 0.5], -0.05, |v| {
        cut(v, &q, rgba(col, 1.0), 0.0, Some(([6.0 * s, 6.0 * s], EDGE)));
        text(
            font,
            v,
            Face::Display,
            words,
            x + 18.0 * s,
            y + 7.0 * s,
            size,
            rgba(ink, 1.0),
        );
    });
}

/// A meter: a black slab with a white edge, its name, the bar in its colour, and the number
/// breaking out of it; on desire, a notch for each garment at the desire it asks.
#[allow(clippy::too_many_arguments)]
fn meter(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    label: &str,
    k: f32,
    color: u32,
    notches: &[(u16, bool, Option<u32>)],
    s: f32,
) {
    let slab = slanted(x, y, 440.0 * s, 50.0 * s, 12.0 * s);
    cut(
        v,
        &slab,
        rgba(EDGE, 0.92),
        2.5 * s,
        Some(([7.0 * s, 7.0 * s], mix(color, 0x000000, 0.55))),
    );
    text(
        font,
        v,
        Face::Display,
        label,
        x + 24.0 * s,
        y + 8.0 * s,
        28.0 * s,
        rgba(PAPER, 1.0),
    );
    let (tx, tw) = (x + 178.0 * s, 186.0 * s);
    let track = slanted(tx, y + 15.0 * s, tw, 20.0 * s, 5.0 * s);
    poly(v, &track, rgba(0x2a2233, 1.0));
    let k = k.clamp(0.0, 1.0);
    if k > 0.0 {
        poly(
            v,
            &slanted(tx, y + 15.0 * s, tw * k, 20.0 * s, 5.0 * s),
            rgba(color, 1.0),
        );
    }
    for i in 0..4 {
        let e = rgba(PAPER, 1.0);
        wire(v, track[i], track[(i + 1) % 4], 2.0 * s, e, e, 0.0, false);
    }
    for &(at, off, ready) in notches {
        let nx = tx + tw * (at as f32 / 1000.0);
        let reached = k * 1000.0 >= at as f32;
        let pop = ready.map_or(1.0, |ms| 1.0 + 0.7 * (1.0 - (ms as f32 / 450.0).min(1.0)));
        let h = 10.0 * s * pop;
        let col = if off {
            color
        } else if reached {
            PAPER
        } else {
            0x5a5070
        };
        let tip = [nx, y + 13.0 * s];
        poly(
            v,
            &[
                [tip[0] - h * 0.85, tip[1] - h - 4.0 * s],
                [tip[0] + h * 0.85, tip[1] - h - 4.0 * s],
                [tip[0], tip[1] + 2.0 * s],
            ],
            rgba(EDGE, 1.0),
        );
        poly(
            v,
            &[
                [tip[0] - h * 0.55, tip[1] - h - 1.5 * s],
                [tip[0] + h * 0.55, tip[1] - h - 1.5 * s],
                [tip[0], tip[1] - 1.0 * s],
            ],
            rgba(col, 1.0),
        );
    }
    let n = format!("{}", (k * 100.0).round() as i32);
    turned(v, [x + 412.0 * s, y + 24.0 * s], -0.08, |v| {
        inked(
            font,
            v,
            &n,
            x + 384.0 * s,
            y - 6.0 * s,
            46.0 * s,
            if color == PAPER {
                PAPER
            } else {
                mix(color, 0xffffff, 0.35)
            },
            4.0 * s,
            1.0,
        );
    });
}

/// The pointer at a menu's lit line: a white wedge with a black edge, bobbing.
fn pointer(v: &mut Vec<MapVertex>, tip: [f32; 2], s: f32, time: f32) {
    let b = (time * 6.0).sin() * 4.0 * s;
    let t = [tip[0] + b, tip[1]];
    poly(
        v,
        &[
            [t[0] - 34.0 * s, t[1] - 19.0 * s],
            [t[0] + 3.0 * s, t[1]],
            [t[0] - 34.0 * s, t[1] + 19.0 * s],
        ],
        rgba(EDGE, 1.0),
    );
    poly(
        v,
        &[
            [t[0] - 29.0 * s, t[1] - 13.0 * s],
            [t[0] - 4.0 * s, t[1]],
            [t[0] - 29.0 * s, t[1] + 13.0 * s],
        ],
        rgba(PAPER, 1.0),
    );
}

/// A menu of slanted lines (the six hands, the pause): black ones, the lit one wider in her colour
/// with what it does under its name and the pointer beside it. Returns where it ends.
#[allow(clippy::too_many_arguments)]
fn menu(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    items: &[(&str, &str)],
    lit: usize,
    accent: u32,
    age: f32,
    time: f32,
    s: f32,
) -> f32 {
    let mut y = y;
    for (i, (name, help)) in items.iter().enumerate() {
        let on = i == lit;
        let a = entrance(age, 0.12 + 0.05 * i as f32, 0.28);
        let (h, w, dx) = if on {
            (74.0 * s, 450.0 * s, 26.0 * s)
        } else {
            (46.0 * s, 330.0 * s, 0.0)
        };
        let ink = if on { ink_on(accent) } else { PAPER };
        let lx = x + dx - (1.0 - a) * 320.0 * s;
        let q = slanted(lx, y, w, h, 10.0 * s);
        faded(v, a, |v| {
            cut(
                v,
                &q,
                rgba(if on { accent } else { EDGE }, if on { 1.0 } else { 0.88 }),
                if on { 3.0 * s } else { 2.0 * s },
                Some((
                    [7.0 * s, 7.0 * s],
                    if on { EDGE } else { mix(accent, 0x000000, 0.6) },
                )),
            );
            text(
                font,
                v,
                Face::Display,
                &format!("{}", i + 1),
                lx + 18.0 * s,
                y + if on { 8.0 } else { 5.0 } * s,
                if on { 38.0 } else { 29.0 } * s,
                rgba(if on { ink } else { mix(accent, 0xffffff, 0.3) }, 1.0),
            );
            text(
                font,
                v,
                Face::Display,
                name,
                lx + 58.0 * s,
                y + if on { 3.0 } else { 6.0 } * s,
                if on { 33.0 } else { 27.0 } * s,
                rgba(ink, 1.0),
            );
            if on && !help.is_empty() {
                let room = w - 84.0 * s;
                let size = (20.0 * s)
                    .min(20.0 * s * room / text_width(font, Face::Round, help, 20.0 * s).max(1.0));
                text(
                    font,
                    v,
                    Face::Round,
                    help,
                    lx + 60.0 * s,
                    y + 44.0 * s,
                    size,
                    rgba(ink, 1.0),
                );
            }
        });
        if on && a > 0.95 {
            pointer(v, [lx - 14.0 * s, y + h * 0.5], s, time);
        }
        y += h + 8.0 * s;
    }
    y
}

/// Keys at the foot: each on a white slanted cap, what it does beside it.
fn keys(font: &FontAtlas, v: &mut Vec<MapVertex>, x: f32, y: f32, list: &[(&str, &str)], s: f32) {
    let mut x = x;
    for (key, what) in list {
        let cw = text_width(font, Face::Display, key, 21.0 * s) + 28.0 * s;
        let q = slanted(x, y, cw, 36.0 * s, 6.0 * s);
        cut(
            v,
            &q,
            rgba(PAPER, 1.0),
            0.0,
            Some(([4.0 * s, 4.0 * s], EDGE)),
        );
        text(
            font,
            v,
            Face::Display,
            key,
            x + 13.0 * s,
            y + 5.0 * s,
            21.0 * s,
            rgba(EDGE, 1.0),
        );
        inked(
            font,
            v,
            what,
            x + cw + 10.0 * s,
            y + 3.0 * s,
            24.0 * s,
            PAPER,
            2.5 * s,
            1.0,
        );
        x += cw + 10.0 * s + text_width(font, Face::Display, what, 24.0 * s) + 26.0 * s;
    }
}

/// Her words: a black cut-paper box with a white edge and her colour's hard shadow, her name on a
/// white strip; `bottom_left` is its lower left corner.
#[allow(clippy::too_many_arguments)]
fn speech(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    bottom_left: [f32; 2],
    max_w: f32,
    who: &str,
    words: &str,
    accent: u32,
    alpha: f32,
    s: f32,
) {
    let size = 28.0 * s;
    let lines = wrap(font, Face::Round, words, size, max_w - 60.0 * s);
    let w = lines
        .iter()
        .map(|l| text_width(font, Face::Round, l, size))
        .fold(0.0, f32::max)
        .max(text_width(font, Face::Display, who, 26.0 * s) + 40.0 * s)
        + 60.0 * s;
    let h = 44.0 * s + lines.len() as f32 * size * 1.34;
    let (x, y) = (bottom_left[0], bottom_left[1] - h);
    let shape = [
        [x + 10.0 * s, y],
        [x + w, y - 8.0 * s],
        [x + w - 12.0 * s, y + h],
        [x - 6.0 * s, y + h + 6.0 * s],
    ];
    faded(v, alpha, |v| {
        cut(
            v,
            &shape,
            rgba(EDGE, 0.95),
            3.0 * s,
            Some(([10.0 * s, 10.0 * s], mix(accent, 0x000000, 0.45))),
        );
        let nw = text_width(font, Face::Display, who, 26.0 * s) + 36.0 * s;
        let strip = slanted(x + 18.0 * s, y - 24.0 * s, nw, 40.0 * s, 8.0 * s);
        turned(v, [x + 18.0 * s + nw * 0.5, y - 4.0 * s], -0.04, |v| {
            cut(
                v,
                &strip,
                rgba(PAPER, 1.0),
                0.0,
                Some(([5.0 * s, 5.0 * s], accent)),
            );
            text(
                font,
                v,
                Face::Display,
                who,
                x + 36.0 * s,
                y - 20.0 * s,
                26.0 * s,
                rgba(EDGE, 1.0),
            );
        });
        for (i, l) in lines.iter().enumerate() {
            text(
                font,
                v,
                Face::Round,
                l,
                x + 30.0 * s,
                y + 24.0 * s + i as f32 * size * 1.34,
                size,
                rgba(PAPER, 1.0),
            );
        }
    });
}

/// A heart with an ink edge.
fn inked_heart(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, color: u32, alpha: f32) {
    heart(v, c, size * 1.25, rgba(EDGE, alpha));
    heart(v, [c[0], c[1] - size * 0.04], size, rgba(color, alpha));
}

/// A band across the screen in her colour, jagged and leaning, with a black one under it.
fn band(v: &mut Vec<MapVertex>, width: f32, cy: f32, thick: f32, accent: u32, s: f32) {
    let q = [
        [-40.0 * s, cy - thick * 0.42],
        [width + 40.0 * s, cy - thick * 0.62],
        [width + 40.0 * s, cy + thick * 0.38],
        [-40.0 * s, cy + thick * 0.58],
    ];
    poly(v, &moved(&q, [0.0, 16.0 * s]), rgba(EDGE, 1.0));
    poly(v, &q, rgba(accent, 1.0));
    let e = rgba(PAPER, 1.0);
    wire(v, q[0], q[1], 4.0 * s, e, e, 0.0, false);
    wire(v, q[2], q[3], 4.0 * s, e, e, 0.0, false);
}

/// The HUD over her (and the clip, the pause, the card).
pub fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &CariciasView,
    time: f32,
) {
    let s = height / 1080.0;
    let accent = view.accent;
    let age = view.t_ms as f32 / 1000.0;
    // The final clip: the scene's picture over the dark.
    if view.reel {
        push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x000000, 1.0));
        let w = width;
        let h = w * 704.0 / 1280.0;
        picture(v, Rect::new(0.0, (height - h) * 0.5, w, h), 0.0);
        keys(
            font,
            v,
            width - 220.0 * s,
            height - 60.0 * s,
            &[("Esc", "saltar")],
            s,
        );
        return;
    }
    // The room darkens at the edges, her colour creeping in as she warms (cold blue when trust is
    // lost).
    let tint = (view.tint as f32 / 1000.0).clamp(0.0, 1.0);
    let cold = view.cold as f32 / 1000.0;
    let edge_col = if cold > 0.0 {
        0x14285a
    } else {
        mix(0x000000, mix(accent, 0x000000, 0.55), tint)
    };
    vignette(
        v,
        Rect::new(0.0, 0.0, width, height),
        0.0,
        260.0 * s,
        rgba(edge_col, 0.45 + cold * 0.4 + tint * 0.15),
    );
    if let Some(card) = &view.card {
        draw_card(font, v, width, height, view, card);
        return;
    }
    if let Some(choice) = view.paused {
        draw_pause(font, v, width, height, accent, choice, time);
        return;
    }
    // The garments' handles: a diamond with an ink edge and the way to pull; a fresh one pops.
    for &(hx, hy, dx, dy, ready, since) in &view.handles {
        let c = at(hx, hy, width, height);
        let d = [dx as f32 / 400.0, dy as f32 / 400.0];
        let fresh = ready && since < 1400;
        let pop = if fresh {
            1.0 + 0.45 * (1.0 - since as f32 / 1400.0) * (since as f32 / 70.0).min(1.0)
        } else {
            1.0
        };
        let col = if ready { accent } else { 0x8a82a8 };
        let a = if ready { 1.0 } else { 0.6 };
        if ready {
            for k in 0..3 {
                let f0 = (30.0 + k as f32 * 16.0) * s;
                let f1 = f0 + 8.0 * s;
                let p0 = [c[0] + d[0] * f0, c[1] + d[1] * f0];
                let p1 = [c[0] + d[0] * f1, c[1] + d[1] * f1];
                wire(
                    v,
                    p0,
                    p1,
                    8.0 * s,
                    rgba(EDGE, 1.0),
                    rgba(EDGE, 1.0),
                    0.0,
                    false,
                );
                wire(
                    v,
                    p0,
                    p1,
                    5.0 * s,
                    rgba(col, 1.0),
                    rgba(col, 1.0),
                    0.0,
                    false,
                );
            }
        }
        let r = 19.0 * s * pop;
        let quad = |k: f32| {
            [
                [c[0], c[1] - k],
                [c[0] + k, c[1]],
                [c[0], c[1] + k],
                [c[0] - k, c[1]],
            ]
        };
        for (k, colr) in [
            (r + 7.0 * s, rgba(EDGE, a)),
            (r + 3.5 * s, rgba(PAPER, a)),
            (r, rgba(if ready { accent } else { 0x3a3352 }, a)),
        ] {
            poly(v, &quad(k), colr);
        }
        if !ready {
            padlock(v, c, 13.0 * s, 0xffffff, 0.9);
        }
        if fresh {
            let t = since as f32 / 1000.0;
            faded(v, (1.0 - (t - 1.0).max(0.0) / 0.4).clamp(0.0, 1.0), |v| {
                comic_stamp(
                    font,
                    v,
                    Face::Display,
                    "¡YA!",
                    [c[0] + 58.0 * s, (c[1] - 46.0 * s).max(64.0 * s)],
                    40.0 * s,
                    accent,
                    true,
                    (t / 0.25).min(1.0),
                    time,
                );
            });
        }
    }
    // Hearts, rings, sweat, the startled «!».
    for &(kind, x, y, sc, life) in &view.fx {
        let c = at(x, y, width, height);
        let k = life as f32 / 1000.0;
        let a = (k * 1.6).min(1.0);
        let sc = sc as f32 / 100.0;
        match kind {
            0 => inked_heart(v, c, 11.0 * sc * s, accent, a),
            1 => {
                let q = 1.0 - k;
                ring(
                    v,
                    c,
                    (14.0 + q * 60.0) * sc * s,
                    4.0 * (1.0 - q) + 1.0,
                    rgba(accent, (1.0 - q) * 0.8),
                );
            }
            2 => {
                disc(v, c, 9.5 * s, rgba(EDGE, a));
                disc(v, c, 7.0 * s, rgba(0x9fd8ff, a));
            }
            _ => faded(v, a, |v| {
                comic_stamp(
                    font,
                    v,
                    Face::Display,
                    "!",
                    [c[0], c[1] - 20.0 * s],
                    54.0 * s,
                    0xff5c6c,
                    false,
                    1.0,
                    time,
                );
            }),
        }
    }
    // Her name: big letters with a black rim over a black cut of paper with her colour's shadow and
    // a stripe of it; what the night is called on a white tag.
    let a_name = entrance(age, 0.0, 0.35);
    faded(v, a_name, |v| {
        shifted(v, [-(1.0 - a_name) * 300.0 * s, 0.0], |v| {
            let name_w = text_width(font, Face::Display, &view.name, 100.0 * s);
            let bw = (name_w + 150.0 * s).max(380.0 * s);
            let back = [
                [-20.0 * s, 16.0 * s],
                [bw, -4.0 * s],
                [bw + 36.0 * s, 120.0 * s],
                [-20.0 * s, 146.0 * s],
            ];
            cut(
                v,
                &back,
                rgba(EDGE, 0.94),
                0.0,
                Some(([12.0 * s, 12.0 * s], mix(accent, 0x000000, 0.5))),
            );
            poly(
                v,
                &slanted(26.0 * s, 108.0 * s, bw - 60.0 * s, 14.0 * s, 10.0 * s),
                rgba(accent, 1.0),
            );
            turned(v, [260.0 * s, 64.0 * s], -0.045, |v| {
                inked(
                    font,
                    v,
                    &view.name,
                    44.0 * s,
                    4.0 * s,
                    100.0 * s,
                    PAPER,
                    5.0 * s,
                    1.0,
                );
            });
            tag(
                font,
                v,
                66.0 * s,
                150.0 * s,
                &view.title,
                25.0 * s,
                PAPER,
                EDGE,
                s,
            );
        });
    });
    // Desire (with the garments' notches) and trust.
    let none: &[(u16, bool, Option<u32>)] = &[];
    for (i, (label, k, color, notches)) in [
        (
            "DESEO",
            view.deseo as f32 / 1000.0,
            accent,
            view.thresholds.as_slice(),
        ),
        ("CONFIANZA", view.conf as f32 / 1000.0, PAPER, none),
    ]
    .into_iter()
    .enumerate()
    {
        let a = entrance(age, 0.08 + 0.06 * i as f32, 0.3);
        faded(v, a, |v| {
            shifted(v, [-(1.0 - a) * 300.0 * s, 0.0], |v| {
                meter(
                    font,
                    v,
                    44.0 * s,
                    (222.0 + 68.0 * i as f32) * s,
                    label,
                    k,
                    color,
                    notches,
                    s,
                );
            });
        });
    }
    // The pace under the hand, on a tag in its colour.
    let (pace, needle) = view.pace;
    if pace != 0 && view.playing {
        let (words, col) = pace_look(pace, accent);
        tag(font, v, 56.0 * s, 368.0 * s, words, 26.0 * s, col, EDGE, s);
    }
    // The candle: a card of her colour at the top right, the time breaking out of it.
    let k = (view.time_ms as f32 / 180_000.0).clamp(0.0, 1.0);
    let (cx0, cy0, cw, ch) = (width - 214.0 * s, 26.0 * s, 170.0 * s, 196.0 * s);
    let card = slanted(cx0, cy0, cw, ch, 10.0 * s);
    cut(
        v,
        &card,
        rgba(accent, 1.0),
        3.0 * s,
        Some(([9.0 * s, 9.0 * s], EDGE)),
    );
    text(
        font,
        v,
        Face::Display,
        "VELA",
        cx0 + 26.0 * s,
        cy0 + 10.0 * s,
        22.0 * s,
        rgba(EDGE, 1.0),
    );
    let (cx, base) = (cx0 + cw * 0.5 + 6.0 * s, cy0 + 132.0 * s);
    let hh = (8.0 + 76.0 * k) * s;
    let body = slanted(cx - 14.0 * s, base - hh, 28.0 * s, hh, 0.0);
    cut(
        v,
        &body,
        rgba(PAPER, 1.0),
        0.0,
        Some(([4.0 * s, 4.0 * s], EDGE)),
    );
    poly(
        v,
        &slanted(cx - 30.0 * s, base, 60.0 * s, 10.0 * s, 4.0 * s),
        rgba(EDGE, 1.0),
    );
    let flick = (time * 9.0).sin() * 1.5 * s;
    let flame = [cx, base - hh - 14.0 * s + flick];
    disc(v, flame, 11.0 * s, rgba(EDGE, 1.0));
    disc(v, flame, 8.0 * s, rgba(0xffcf5a, 1.0));
    disc(v, [cx, base - hh - 11.0 * s], 3.5 * s, rgba(0xfff6d0, 1.0));
    let secs = view.time_ms / 1000;
    let tt = format!("{}:{:02}", secs / 60, secs % 60);
    let tw = text_width(font, Face::Display, &tt, 48.0 * s);
    turned(v, [cx0 + cw * 0.5, cy0 + ch - 16.0 * s], -0.07, |v| {
        inked(
            font,
            v,
            &tt,
            cx0 + (cw - tw) * 0.5,
            cy0 + ch - 50.0 * s,
            48.0 * s,
            if secs < 30 { 0xff8a8a } else { PAPER },
            4.0 * s,
            1.0,
        );
    });
    // The six hands down the left.
    let tools = &super::super::caricias::TOOLS;
    let items: Vec<(&str, &str)> = tools.iter().map(|t| (t.name, t.help)).collect();
    menu(
        font,
        v,
        58.0 * s,
        566.0 * s,
        &items,
        view.tool as usize,
        accent,
        age,
        time,
        s,
    );
    keys(
        font,
        v,
        44.0 * s,
        height - 56.0 * s,
        &[
            ("CLIC", "tocar"),
            ("1-6", "mano"),
            ("RUEDA", "cambiar"),
            ("ARRASTRAR", "quitar ropa"),
            ("CLIC DER.", "beso"),
            ("ESC", "pausa"),
        ],
        s,
    );
    // Her words at the right, clear of her.
    if let Some((text_line, left)) = &view.line {
        let a = (*left as f32 / 1000.0).clamp(0.0, 1.0);
        speech(
            font,
            v,
            [width - 610.0 * s, 470.0 * s],
            560.0 * s,
            &view.name,
            text_line,
            accent,
            a,
            s,
        );
    }
    // «¡Su punto débil!»
    if let Some((toast, ms)) = &view.toast {
        let t = *ms as f32 / 1000.0;
        faded(v, (1.0 - (t - 1.3).max(0.0) / 0.6).clamp(0.0, 1.0), |v| {
            comic_stamp(
                font,
                v,
                Face::Display,
                toast,
                [width * 0.5, 180.0 * s],
                80.0 * s,
                accent,
                true,
                (t / 0.3).min(1.0),
                time,
            );
        });
    }
    // The hand: the pace round it as a gauge (the good band marked), what is under it on a tag.
    let m = at(view.mouse[0], view.mouse[1], width, height);
    if view.playing {
        if (1..=4).contains(&pace) {
            let (_, col) = pace_look(pace, accent);
            let (start, span) = (0.625, 0.75);
            let (rad, w) = (40.0 * s, 6.0 * s);
            let c = [m[0], m[1] + 4.0 * s];
            arc(
                v,
                c,
                rad,
                w + 3.0 * s,
                start,
                span,
                rgba(EDGE, 0.85),
                [0.0; 4],
            );
            arc(
                v,
                c,
                rad,
                w,
                start + 0.3 * span,
                0.3 * span,
                rgba(MINT, 0.35),
                [0.0; 4],
            );
            arc(
                v,
                c,
                rad,
                w,
                start,
                span * (needle as f32 / 1000.0),
                rgba(col, 1.0),
                [0.0; 4],
            );
        }
        if !view.hint.is_empty() {
            tag(
                font,
                v,
                m[0] + 50.0 * s,
                m[1] + 26.0 * s,
                &view.hint.replace("  ·", " ·"),
                23.0 * s,
                EDGE,
                PAPER,
                s,
            );
        }
        hand(v, m, view.tool, accent);
        if let Some(mx) = view.mirror {
            hand(v, at(mx, view.mouse[1], width, height), view.tool, accent);
        }
    }
}

/// The pause: her colour's band with the word on it, and the two ways on (alone over the night).
fn draw_pause(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    accent: u32,
    choice: u8,
    time: f32,
) {
    let s = height / 1080.0;
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x05030a, 0.68));
    band(v, width, height * 0.36, 170.0 * s, accent, s);
    let pw = text_width(font, Face::Display, "PAUSA", 140.0 * s);
    turned(v, [width * 0.5, height * 0.36], -0.05, |v| {
        inked(
            font,
            v,
            "PAUSA",
            (width - pw) * 0.5,
            height * 0.36 - 92.0 * s,
            140.0 * s,
            PAPER,
            7.0 * s,
            1.0,
        );
    });
    menu(
        font,
        v,
        (width - 430.0 * s) * 0.5,
        height * 0.56,
        &[
            ("Seguir", "Volver con ella"),
            ("Parar aquí", "Lo dejáis por esta noche"),
        ],
        choice as usize,
        accent,
        10.0,
        time,
        s,
    );
}

/// The end's card: the title thrown on her band, her last words, the night's numbers (alone over
/// the night).
fn draw_card(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &CariciasView,
    card: &CariciasCard,
) {
    let s = height / 1080.0;
    let accent = view.accent;
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x05030a, 0.72));
    band(v, width, 150.0 * s, 170.0 * s, accent, s);
    let fit = 1500.0 * s / text_width(font, Face::Display, &card.title, 1.0).max(1.0);
    let size = (130.0 * s).min(fit);
    let tw = text_width(font, Face::Display, &card.title, size);
    turned(v, [width * 0.5, 150.0 * s], -0.05, |v| {
        inked(
            font,
            v,
            &card.title,
            (width - tw) * 0.5,
            150.0 * s - size * 0.66,
            size,
            PAPER,
            7.0 * s,
            1.0,
        );
    });
    speech(
        font,
        v,
        [(width - 900.0 * s) * 0.5, 430.0 * s],
        900.0 * s,
        &view.name,
        &card.quote,
        accent,
        1.0,
        s,
    );
    for (i, (label, value)) in card.stats.iter().enumerate() {
        let (col, row) = (i % 2, i / 2);
        let (x, y) = (
            (width * 0.5 - 470.0 * s) + col as f32 * 490.0 * s,
            (490.0 + row as f32 * 72.0) * s,
        );
        let q = slanted(x, y, 450.0 * s, 58.0 * s, 10.0 * s);
        cut(
            v,
            &q,
            rgba(EDGE, 0.94),
            2.5 * s,
            Some(([7.0 * s, 7.0 * s], mix(accent, 0x000000, 0.5))),
        );
        text(
            font,
            v,
            Face::Display,
            label,
            x + 26.0 * s,
            y + 10.0 * s,
            29.0 * s,
            rgba(PAPER, 1.0),
        );
        let vw = text_width(font, Face::Display, value, 38.0 * s);
        text(
            font,
            v,
            Face::Display,
            value,
            x + 420.0 * s - vw,
            y + 4.0 * s,
            38.0 * s,
            rgba(mix(accent, 0xffffff, 0.35), 1.0),
        );
    }
    let mut list = vec![("ENTER", "cerrar")];
    if card.clip {
        list.push(("V", "ver el final otra vez"));
    }
    keys(font, v, (width - 520.0 * s) * 0.5, 740.0 * s, &list, s);
}
