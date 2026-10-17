//! Caricias on screen: her room and her art as pictures of the stage's pass (her body as a bent
//! mesh, her face crossing over on its own bent patch), and over them the night's HUD in the
//! manner of the web page it comes from: her name, desire and trust, the candle, the six hands, the
//! garments' handles, what her body is called under the hand, her words, the hearts and rings,
//! the pause and the end's card. The final clip plays as the scene's picture. Authored at
//! 1920×1080.
use std::f32::consts::TAU;

use super::persona::*;
use super::persona_screens::{fill, rim, slanted};
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
    /// The handles: x, y, direction (×100), ready to come off.
    pub handles: Vec<(i32, i32, i32, i32, bool)>,
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
}

const MODE_CUTOUT: f32 = 15.0;
const INK_DARK: u32 = 0x0e0816;
const BONE: u32 = 0xf6efe4;

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

fn meter(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    label: &str,
    k: f32,
    color: u32,
) {
    let w = 340.0;
    text_outline(
        font,
        v,
        Face::Display,
        label,
        x,
        y,
        22.0,
        rgba(BONE, 1.0),
        rgba(0x000000, 0.8),
    );
    let bar = slanted(x, y + 32.0, w, 22.0, 6.0);
    fill(v, bar, rgba(0x120a1a, 0.9));
    if k > 0.0 {
        fill(
            v,
            slanted(x, y + 32.0, w * k.clamp(0.0, 1.0), 22.0, 6.0),
            rgba(color, 1.0),
        );
    }
    rim(v, bar, 1.5, rgba(0xffffff, 0.55));
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
    // The final clip: the scene's picture over the dark.
    if view.reel {
        push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x000000, 1.0));
        let w = width;
        let h = w * 704.0 / 1280.0;
        picture(v, Rect::new(0.0, (height - h) * 0.5, w, h), 0.0);
        text_shadow(
            font,
            v,
            Face::Italic,
            "Esc · saltar",
            width - 220.0 * s,
            height - 50.0 * s,
            22.0 * s,
            rgba(CREAM, 0.7),
        );
        return;
    }
    // Her warmth over the room; the cold when trust is lost.
    let tint = view.tint as f32 / 1000.0;
    if tint > 0.0 {
        // A glow round her middle that fades long before the edges (the web's 70 % ellipse).
        halo(
            v,
            [width * 0.5, height * 0.45],
            width * 0.12,
            width * 0.3,
            rgba(accent, tint * 0.45),
        );
    }
    let cold = view.cold as f32 / 1000.0;
    vignette(
        v,
        Rect::new(0.0, 0.0, width, height),
        0.0,
        260.0 * s,
        rgba(
            if cold > 0.0 { 0x14285a } else { 0x000000 },
            0.45 + cold * 0.4,
        ),
    );
    // The garments' handles.
    for &(hx, hy, dx, dy, ready) in &view.handles {
        let c = at(hx, hy, width, height);
        let d = [dx as f32 / 400.0, dy as f32 / 400.0];
        let pulse = 1.0 + (time * 4.0).sin() * 0.12;
        let col = if ready { accent } else { 0x8a82a8 };
        let a = if ready { 1.0 } else { 0.55 };
        for k in 0..3 {
            let f0 = (30.0 + k as f32 * 16.0) * s;
            let f1 = f0 + 8.0 * s;
            wire(
                v,
                [c[0] + d[0] * f0, c[1] + d[1] * f0],
                [c[0] + d[0] * f1, c[1] + d[1] * f1],
                5.0 * s,
                rgba(col, a),
                rgba(col, a),
                0.0,
                false,
            );
        }
        let r = 20.0 * s * pulse;
        let diamond = [
            [c[0], c[1] - r],
            [c[0] + r, c[1]],
            [c[0], c[1] + r],
            [c[0] - r, c[1]],
        ];
        let edge = [
            [c[0], c[1] - r - 3.0],
            [c[0] + r + 3.0, c[1]],
            [c[0], c[1] + r + 3.0],
            [c[0] - r - 3.0, c[1]],
        ];
        push_triangle(v, [edge[0], edge[1], edge[2]], rgba(0xffffff, a));
        push_triangle(v, [edge[0], edge[2], edge[3]], rgba(0xffffff, a));
        push_triangle(
            v,
            [diamond[0], diamond[1], diamond[2]],
            rgba(if ready { accent } else { 0x3a3352 }, a),
        );
        push_triangle(
            v,
            [diamond[0], diamond[2], diamond[3]],
            rgba(if ready { accent } else { 0x3a3352 }, a),
        );
        if !ready {
            padlock(v, c, 14.0 * s, 0xffffff, 0.9);
        }
    }
    // Hearts, rings, sweat, the startled «!».
    for &(kind, x, y, sc, life) in &view.fx {
        let c = at(x, y, width, height);
        let k = life as f32 / 1000.0;
        let a = (k * 1.6).min(1.0);
        let sc = sc as f32 / 100.0;
        match kind {
            0 => heart(v, c, 11.0 * sc * s, rgba(accent, a)),
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
            2 => disc(v, c, 8.0 * s, rgba(0x9fd8ff, a)),
            _ => text_outline(
                font,
                v,
                Face::Display,
                "!",
                c[0],
                c[1] - 40.0 * s,
                54.0 * s,
                rgba(0xffffff, a),
                rgba(0x000000, a),
            ),
        }
    }
    // Her name at the top left; desire and trust under it; the pace.
    fill(
        v,
        slanted(40.0 * s, 34.0 * s, 360.0 * s, 74.0 * s, 12.0 * s),
        rgba(0x0b0610, 0.88),
    );
    fill(
        v,
        slanted(40.0 * s, 34.0 * s, 10.0 * s, 74.0 * s, 12.0 * s),
        rgba(accent, 1.0),
    );
    text_outline(
        font,
        v,
        Face::Title,
        &view.name,
        66.0 * s,
        38.0 * s,
        40.0 * s,
        rgba(BONE, 1.0),
        rgba(0x000000, 0.8),
    );
    text_shadow(
        font,
        v,
        Face::Italic,
        &view.title.to_uppercase(),
        68.0 * s,
        82.0 * s,
        18.0 * s,
        rgba(CREAM, 0.9),
    );
    meter(
        font,
        v,
        48.0 * s,
        130.0 * s,
        "DESEO",
        view.deseo as f32 / 1000.0,
        accent,
    );
    meter(
        font,
        v,
        48.0 * s,
        200.0 * s,
        "CONFIANZA",
        view.conf as f32 / 1000.0,
        0xeaf2ff,
    );
    if !view.rhythm.is_empty() {
        let fast = view.rhythm.contains('¡');
        text_outline(
            font,
            v,
            Face::Display,
            &view.rhythm,
            50.0 * s,
            268.0 * s,
            22.0 * s,
            rgba(if fast { 0xff5c5c } else { BONE }, 1.0),
            rgba(0x000000, 0.85),
        );
    }
    // The candle, at the top right.
    let secs = view.time_ms / 1000;
    let k = (view.time_ms as f32 / 180_000.0).clamp(0.0, 1.0);
    let (cx, base) = (width - 120.0 * s, 210.0 * s);
    let h = (30.0 + 150.0 * k) * s;
    fill(
        v,
        slanted(cx - 22.0 * s, base - h, 44.0 * s, h, 0.0),
        rgba(0xefe6d2, 1.0),
    );
    fill(
        v,
        slanted(cx - 40.0 * s, base, 80.0 * s, 16.0 * s, 0.0),
        rgba(0x6f5a3a, 1.0),
    );
    let flick = (time * 9.0).sin() * 1.5 * s;
    halo(
        v,
        [cx, base - h - 18.0 * s],
        30.0 * s,
        26.0 * s,
        rgba(0xffcf5a, 0.35),
    );
    disc(
        v,
        [cx, base - h - 18.0 * s + flick],
        10.0 * s,
        rgba(0xffcf5a, 1.0),
    );
    disc(v, [cx, base - h - 14.0 * s], 5.0 * s, rgba(0xfff6d0, 1.0));
    let tt = format!("{}:{:02}", secs / 60, secs % 60);
    let tw = text_width(font, Face::Display, &tt, 26.0 * s);
    text_outline(
        font,
        v,
        Face::Display,
        &tt,
        cx - tw * 0.5,
        base + 22.0 * s,
        26.0 * s,
        rgba(BONE, 1.0),
        rgba(0x000000, 0.8),
    );
    // The six hands at the foot.
    let tools = &super::super::caricias::TOOLS;
    let mut x = 40.0 * s;
    let y = height - 92.0 * s;
    for (i, t) in tools.iter().enumerate() {
        let label = format!("{} · {}", i + 1, t.name);
        let w = text_width(font, Face::Display, &label, 20.0 * s) + 34.0 * s;
        let lit = view.tool as usize == i;
        fill(
            v,
            slanted(x, y, w, 40.0 * s, 9.0 * s),
            rgba(
                if lit { accent } else { 0x120a1a },
                if lit { 1.0 } else { 0.86 },
            ),
        );
        rim(
            v,
            slanted(x, y, w, 40.0 * s, 9.0 * s),
            if lit { 2.5 } else { 1.0 },
            rgba(
                if lit {
                    0xffffff
                } else {
                    mix(accent, 0x000000, 0.4)
                },
                1.0,
            ),
        );
        text(
            font,
            v,
            Face::Display,
            &label,
            x + 17.0 * s,
            y + 8.0 * s,
            20.0 * s,
            rgba(if lit { INK_DARK } else { BONE }, 1.0),
        );
        x += w + 10.0 * s;
    }
    if let Some(t) = tools.get(view.tool as usize) {
        let help = format!(
            "{}  Rueda: cambiar mano · Arrastra ◆: quitar ropa · Clic derecho: beso · Esc: pausa",
            t.help
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            &help,
            42.0 * s,
            height - 40.0 * s,
            20.0 * s,
            rgba(CREAM, 0.92),
        );
    }
    // Her words.
    if let Some((text_line, left)) = &view.line {
        let a = (*left as f32 / 1000.0).clamp(0.0, 1.0);
        let max_w = 980.0 * s;
        let lines = wrap(font, Face::Body, text_line, 30.0 * s, max_w);
        let bh = 70.0 * s + lines.len() as f32 * 40.0 * s;
        let bx = (width - max_w - 80.0 * s) * 0.5;
        let by = height - 150.0 * s - bh;
        faded(v, a, |v| {
            fill(
                v,
                slanted(bx + 10.0 * s, by + 12.0 * s, max_w + 80.0 * s, bh, 12.0 * s),
                rgba(0x000000, 0.5),
            );
            fill(
                v,
                slanted(bx, by, max_w + 80.0 * s, bh, 12.0 * s),
                rgba(0x0d0814, 0.9),
            );
            fill(
                v,
                slanted(bx, by, 10.0 * s, bh, 12.0 * s),
                rgba(accent, 1.0),
            );
            text_outline(
                font,
                v,
                Face::Display,
                &view.name,
                bx + 34.0 * s,
                by + 12.0 * s,
                26.0 * s,
                rgba(accent, 1.0),
                rgba(0x000000, 0.8),
            );
            for (i, l) in lines.iter().enumerate() {
                text_shadow(
                    font,
                    v,
                    Face::Body,
                    l,
                    bx + 36.0 * s,
                    by + 52.0 * s + i as f32 * 40.0 * s,
                    30.0 * s,
                    rgba(INK, 1.0),
                );
            }
        });
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
    // What is under the hand, beside it; the hand.
    let m = at(view.mouse[0], view.mouse[1], width, height);
    if view.playing && !view.hint.is_empty() {
        let w = text_width(font, Face::Display, &view.hint, 20.0 * s) + 28.0 * s;
        let (hx, hy) = (m[0] + 44.0 * s, m[1] + 30.0 * s);
        fill(
            v,
            slanted(hx, hy, w, 34.0 * s, 8.0 * s),
            rgba(0x0b0610, 0.88),
        );
        text(
            font,
            v,
            Face::Display,
            &view.hint,
            hx + 14.0 * s,
            hy + 6.0 * s,
            20.0 * s,
            rgba(BONE, 1.0),
        );
    }
    if view.playing {
        hand(v, m, view.tool, accent);
        if let Some(mx) = view.mirror {
            hand(v, at(mx, view.mouse[1], width, height), view.tool, accent);
        }
    }
    // The pause.
    if let Some(choice) = view.paused {
        push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x05030a, 0.6));
        let (pw, ph) = (560.0 * s, 300.0 * s);
        let (px, py) = ((width - pw) * 0.5, (height - ph) * 0.5);
        fill(v, slanted(px, py, pw, ph, 16.0 * s), rgba(0x120a1a, 0.96));
        rim(v, slanted(px, py, pw, ph, 16.0 * s), 3.0, rgba(accent, 1.0));
        shout(
            font,
            v,
            Face::Title,
            "PAUSA",
            px + 40.0 * s,
            py + 26.0 * s,
            54.0 * s,
            0xffffff,
            mix(accent, 0x000000, 0.4),
            1.0,
        );
        for (i, label) in ["Seguir", "Parar aquí"].into_iter().enumerate() {
            let lit = choice as usize == i;
            let (bx, by) = (px + 40.0 * s, py + 120.0 * s + i as f32 * 74.0 * s);
            fill(
                v,
                slanted(bx, by, pw - 80.0 * s, 56.0 * s, 10.0 * s),
                rgba(if lit { accent } else { 0x1d1428 }, 1.0),
            );
            text(
                font,
                v,
                Face::Title,
                label,
                bx + 26.0 * s,
                by + 10.0 * s,
                32.0 * s,
                rgba(if lit { INK_DARK } else { BONE }, 1.0),
            );
        }
    }
    // The end's card.
    if let Some(card) = &view.card {
        push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x05030a, 0.66));
        let (cw, ch) = (1000.0 * s, 620.0 * s);
        let (cx0, cy0) = ((width - cw) * 0.5, (height - ch) * 0.5);
        fill(
            v,
            slanted(cx0 + 14.0 * s, cy0 + 16.0 * s, cw, ch, 18.0 * s),
            rgba(0x000000, 0.6),
        );
        fill(v, slanted(cx0, cy0, cw, ch, 18.0 * s), rgba(0x120a1a, 0.97));
        fill(v, slanted(cx0, cy0, cw, 12.0 * s, 1.0), rgba(accent, 1.0));
        shout(
            font,
            v,
            Face::Title,
            &card.title,
            cx0 + 50.0 * s,
            cy0 + 40.0 * s,
            64.0 * s,
            0xffffff,
            mix(accent, 0x000000, 0.4),
            1.0,
        );
        for (i, l) in wrap(font, Face::Italic, &card.quote, 30.0 * s, cw - 100.0 * s)
            .iter()
            .take(3)
            .enumerate()
        {
            text_shadow(
                font,
                v,
                Face::Italic,
                l,
                cx0 + 52.0 * s,
                cy0 + 140.0 * s + i as f32 * 40.0 * s,
                30.0 * s,
                rgba(CREAM, 1.0),
            );
        }
        for (i, (label, value)) in card.stats.iter().enumerate() {
            let (col, row) = (i % 2, i / 2);
            let (sx, sy) = (
                cx0 + 52.0 * s + col as f32 * 460.0 * s,
                cy0 + 290.0 * s + row as f32 * 54.0 * s,
            );
            text_shadow(font, v, Face::Ui, label, sx, sy, 24.0 * s, rgba(CREAM, 0.9));
            text_outline(
                font,
                v,
                Face::Title,
                value,
                sx + 240.0 * s,
                sy - 4.0 * s,
                28.0 * s,
                rgba(GOLD_HI, 1.0),
                rgba(0x000000, 0.8),
            );
        }
        let mut kx = cx0 + 52.0 * s;
        let ky = cy0 + ch - 76.0 * s;
        let mut keys = vec![("Enter", "cerrar")];
        if card.clip {
            keys.push(("V", "ver el final otra vez"));
        }
        for (k, what) in keys {
            let kw = text_width(font, Face::Title, k, 20.0 * s) + 26.0 * s;
            keycap(
                font,
                v,
                Rect::new(kx, ky, kw, 40.0 * s),
                k,
                20.0 * s,
                false,
                false,
                accent,
                1.0,
            );
            text_shadow(
                font,
                v,
                Face::Italic,
                what,
                kx + kw + 10.0 * s,
                ky + 8.0 * s,
                22.0 * s,
                rgba(CREAM, 0.95),
            );
            kx += kw + 30.0 * s + text_width(font, Face::Italic, what, 22.0 * s);
        }
    }
}
