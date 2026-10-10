//! The world's stage: guion scenes played full screen (the springs' and the brothel's welcomes, the
//! girls' talks). A painted background that changes behind a slanted wipe; the cast standing at the
//! left and the right (the one speaking lit, the other dimmed; a new expression cross-fades over the
//! old, a surprise shakes or hops); and over them Persona's flourishes: the title on a slash, a
//! stamp, a flash, an item shown off on a card, a triptych of panels, a reveal, and the list of what
//! the scene unlocked. The pictures are textures of their own, drawn in the stage's own passes (under
//! the overlay's shapes, or over them for what sits on a card); the conversation box is the world's.
use std::collections::HashMap;
use std::f32::consts::PI;

use super::persona::*;
use super::ui_paint::*;
use super::*;

/// One of the cast on the stage.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StageActor {
    /// The cut-out on show (under `data/assets`, without `.png`).
    pub image: String,
    /// The one before it, while the new one fades in over it (`swap` 0 → 1).
    pub prev: Option<String>,
    pub swap: f32,
    pub right: bool,
    /// Coming on (0 → 1); going off when `leaving`.
    pub enter: f32,
    pub leaving: bool,
    /// Seconds since the last shake and the last hop (large: none).
    pub shake: f32,
    pub bump: f32,
    /// Speaking (or thinking): lit; else dimmed.
    pub lit: bool,
    /// Turning on the spot: the frames, and how far through them (seconds).
    pub spin: Option<(Vec<String>, f32)>,
    /// Her size against the others (1: the cast's height; a goblin is shorter).
    pub scale: f32,
}

/// An item shown off: its picture (or SVG), name and line, and how long it has been up.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StageItem {
    pub icon: String,
    pub name: String,
    pub text: String,
    pub age: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StageView {
    pub bg: Option<String>,
    pub bg_prev: Option<String>,
    /// The wipe to `bg` (0 → 1; 1 when done). A wipe with no new picture only sweeps its slash.
    pub wipe: f32,
    pub actors: Vec<StageActor>,
    /// The title card's two lines and its age (seconds).
    pub title: Option<(String, String, f32)>,
    /// A stamp's words, gold or red, and its age.
    pub stamp: Option<(String, bool, f32)>,
    /// Seconds since the last flash (large: none).
    pub flash: f32,
    pub item: Option<StageItem>,
    /// The triptych's panels (picture, age), left to right.
    pub panels: Vec<Option<(String, f32)>>,
    /// A picture revealed big in the middle, and its age.
    pub reveal: Option<(String, f32)>,
    /// The scene is over: what it unlocked, and the card's age.
    pub outro: Option<(Vec<String>, f32)>,
    /// The outro's banner («ESCENA COMPLETA», «FOTO GUARDADA»…).
    pub outro_title: String,
    /// Seconds the stage has been up (the curtain opening).
    pub age: f32,
    /// The box's accent colour of whoever speaks (for the slashes).
    pub accent: u32,
    /// The side of the one speaking or thinking (true: the right), for the band behind her.
    pub speaker: Option<bool>,
    /// Seconds the line or the choice on show has been up (its entrance).
    pub line_age: f32,
}

/// A picture's size and where its figure's head is (the first row with something solid), from the
/// stage's texture cache.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PictureInfo {
    pub size: [f32; 2],
    pub top: f32,
}

/// The pictures drawn in one run of the stage pass: which picture, whether over the overlay's
/// shapes, and the run's vertices (screen pixels).
pub(crate) struct PictureRun {
    pub image: String,
    pub over: bool,
    pub vertices: Vec<MapVertex>,
}

const MODE_CUTOUT: f32 = 15.0;

fn quad_uv(run: &mut Vec<MapVertex>, corners: [[f32; 2]; 4], uvs: [[f32; 2]; 4], tint: [f32; 4]) {
    for i in [0, 1, 2, 0, 2, 3] {
        run.push(MapVertex {
            position: corners[i],
            uv: uvs[i],
            color: tint,
            text: MODE_CUTOUT,
            ..MapVertex::BLANK
        });
    }
}

/// `image` laid over `r`, its uv cropped to cover it (centred, or from the top with `top`).
fn cover(r: Rect, size: [f32; 2]) -> ([f32; 2], [f32; 2]) {
    let k = (r.w / size[0]).max(r.h / size[1]);
    let (w, h) = (r.w / k / size[0], r.h / k / size[1]);
    let u0 = (1.0 - w) * 0.5;
    let v0 = (1.0 - h) * 0.5;
    ([u0, v0], [u0 + w, v0 + h])
}

fn runs_push<'a>(runs: &'a mut Vec<PictureRun>, image: &str, over: bool) -> &'a mut Vec<MapVertex> {
    let fresh = runs
        .last()
        .is_none_or(|r| r.image != image || r.over != over);
    if fresh {
        runs.push(PictureRun {
            image: image.to_owned(),
            over,
            vertices: Vec::new(),
        });
    }
    &mut runs.last_mut().unwrap().vertices
}

/// The background `image` covering the screen, drifting slowly `age` seconds in (a breath of
/// zoom and sway, as the web scenes' backgrounds do).
fn background(
    runs: &mut Vec<PictureRun>,
    image: &str,
    info: PictureInfo,
    width: f32,
    height: f32,
    age: Option<f32>,
    tint: [f32; 4],
) {
    let t = age.unwrap_or(0.0);
    let zoom = 1.05 + 0.02 * (t * 0.13).sin();
    let sway = 10.0 * (height / 1080.0) * (t * 0.11).sin();
    let (w, h) = (width * zoom, height * zoom);
    let screen = Rect::new((width - w) * 0.5 + sway, (height - h) * 0.5, w, h);
    let (uv0, uv1) = cover(screen, info.size);
    let run = runs_push(runs, image, false);
    let (x0, y0, x1, y1) = (screen.x, screen.y, screen.x + screen.w, screen.y + screen.h);
    quad_uv(
        run,
        [[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
        [uv0, [uv1[0], uv0[1]], uv1, [uv0[0], uv1[1]]],
        tint,
    );
}

/// Where an actor stands: her picture's rect (head near the top, cut at the thighs by the screen's
/// foot), centred on her side's third.
fn actor_rect(info: PictureInfo, right: bool, scale: f32, width: f32, height: f32) -> Rect {
    // From the head down, the figure fills 1.08 screens: the screen's foot cuts it at mid-thigh.
    // A smaller one stands on the same floor: her head lower, cut at the same height of her body.
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let figure = (info.size[1] - info.top).max(1.0);
    let k = height * 1.08 * scale / figure;
    let (w, h) = (info.size[0] * k, info.size[1] * k);
    let cx = if right { width * 0.735 } else { width * 0.265 };
    let head = height * (0.055 + 1.08 * (1.0 - scale));
    Rect::new(cx - w * 0.5, head - info.top * k, w, h)
}

/// The pictures of the stage, in the order they are drawn.
pub(crate) fn pictures(
    view: &StageView,
    info: &dyn Fn(&str) -> Option<PictureInfo>,
    width: f32,
    height: f32,
) -> Vec<PictureRun> {
    let mut runs: Vec<PictureRun> = Vec::new();
    let s = height / 1080.0;
    let dim_bg = if view.reveal.is_some() || view.outro.is_some() {
        0.55
    } else {
        1.0
    };
    let bg_tint = [dim_bg, dim_bg, dim_bg, 1.0];
    // The background: the old one until the wipe's bands cover the stage, then the new one,
    // drifting slowly.
    let shown_bg = match (&view.bg_prev, &view.bg) {
        (Some(prev), Some(_)) if view.wipe < 0.5 => Some(prev),
        (_, bg) => bg.as_ref(),
    };
    if let Some(bg) = shown_bg {
        if let Some(i) = info(bg) {
            background(&mut runs, bg, i, width, height, Some(view.age), bg_tint);
        }
    }
    // The band of colour behind whoever speaks, over the place and under the cast.
    if let (Some(right), Some(run)) = (view.speaker, runs.last_mut()) {
        speaker_slash(&mut run.vertices, width, height, right, 1.0);
    }
    // The cast: the dimmed one first, the one speaking over her.
    let mut order: Vec<&StageActor> = view.actors.iter().collect();
    order.sort_by_key(|a| a.lit);
    for a in order {
        if view.reveal.as_ref().is_some_and(|(r, _)| *r == a.image) {
            continue;
        }
        let k_enter = ease_out(a.enter.clamp(0.0, 1.0));
        let alpha = if a.leaving { 1.0 - k_enter } else { k_enter };
        if alpha <= 0.01 {
            continue;
        }
        let light = if a.lit { 1.0 } else { 0.58 };
        let slide = (1.0 - k_enter)
            * 70.0
            * s
            * if a.right { 1.0 } else { -1.0 }
            * if a.leaving { -1.0 } else { 1.0 };
        let shake = if a.shake < 0.5 {
            (a.shake * 55.0).sin() * 16.0 * s * (1.0 - a.shake / 0.5)
        } else {
            0.0
        };
        let hop = if a.bump < 0.34 {
            (a.bump / 0.34 * PI).sin() * 26.0 * s
        } else {
            0.0
        };
        let grow = if a.bump < 0.34 {
            1.0 + (a.bump / 0.34 * PI).sin() * 0.025
        } else {
            1.0
        };
        let mut layers: Vec<(&str, f32)> = Vec::new();
        match &a.spin {
            Some((frames, t)) if !frames.is_empty() => {
                // One slow cross-faded turn: each frame 0.52 s, the next fading in over its last third.
                let per = 0.52;
                let i = ((t / per) as usize).min(frames.len() - 1);
                let into = (t - i as f32 * per) / per;
                layers.push((frames[i].as_str(), 1.0));
                if i + 1 < frames.len() && into > 0.66 {
                    layers.push((frames[i + 1].as_str(), (into - 0.66) / 0.34));
                }
            }
            _ => {
                if let (Some(prev), true) = (&a.prev, a.swap < 1.0) {
                    layers.push((prev.as_str(), 1.0));
                    layers.push((a.image.as_str(), ease_out(a.swap)));
                } else {
                    layers.push((a.image.as_str(), 1.0));
                }
            }
        }
        for (image, layer_alpha) in layers {
            let Some(i) = info(image) else { continue };
            let r = actor_rect(i, a.right, a.scale, width, height);
            let (cx, foot) = (r.x + r.w * 0.5, r.y + r.h);
            let (w, h) = (r.w * grow, r.h * grow);
            let x = cx - w * 0.5 + slide + shake;
            let y = foot - h - hop;
            let run = runs_push(&mut runs, image, false);
            quad_uv(
                run,
                [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                [
                    light * dim_bg,
                    light * dim_bg,
                    light * dim_bg,
                    alpha * layer_alpha,
                ],
            );
        }
    }
    // The triptych: tall slanted panels across the screen.
    for (k, panel) in view.panels.iter().enumerate() {
        let Some((image, age)) = panel else { continue };
        let Some(i) = info(image) else { continue };
        let a = entrance(*age, 0.0, 0.32);
        let r = panel_rect(k, width, height);
        let lean = 40.0 * s;
        let drop = (1.0 - a) * -60.0 * s;
        let (uv0, uv1) = cover(r, i.size);
        let uv = |x: f32, y: f32| {
            [
                uv0[0] + (uv1[0] - uv0[0]) * (x - r.x) / r.w,
                uv0[1] + (uv1[1] - uv0[1]) * (y - r.y) / r.h,
            ]
        };
        let corners = [
            [r.x + lean, r.y],
            [r.x + r.w + lean, r.y],
            [r.x + r.w - lean, r.y + r.h],
            [r.x - lean, r.y + r.h],
        ];
        let uvs = corners.map(|c| uv(c[0], c[1]));
        let run = runs_push(&mut runs, image, false);
        quad_uv(
            run,
            corners.map(|c| [c[0], c[1] + drop]),
            uvs,
            [1.0, 1.0, 1.0, a],
        );
    }
    // The reveal: big, in the middle.
    if let Some((image, age)) = &view.reveal {
        if let Some(i) = info(image) {
            let a = entrance(*age, 0.15, 0.45);
            let figure = (i.size[1] - i.top).max(1.0);
            let k = height * 1.1 / figure * (0.94 + 0.06 * a);
            let (w, h) = (i.size[0] * k, i.size[1] * k);
            let x = width * 0.5 - w * 0.5;
            let y = height * 0.04 - i.top * k;
            let run = runs_push(&mut runs, image, false);
            quad_uv(
                run,
                [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                [1.0, 1.0, 1.0, a],
            );
        }
    }
    // The item's picture, on its card (over the overlay's shapes).
    if let Some(item) = &view.item {
        if let Some(i) = info(&item.icon) {
            let card = item_card(width, height);
            let a = entrance(item.age, 0.12, 0.35);
            let side = card.h - 60.0 * s;
            let k = (side / i.size[0]).min(side / i.size[1]);
            let (w, h) = (i.size[0] * k, i.size[1] * k);
            let float = (item.age * 2.2).sin() * 6.0 * s;
            let cx = card.x + 30.0 * s + side * 0.5;
            let cy = card.y + card.h * 0.5 + float;
            let (x, y) = (cx - w * 0.5, cy - h * 0.5);
            let run = runs_push(&mut runs, &item.icon, true);
            quad_uv(
                run,
                [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                [1.0, 1.0, 1.0, a],
            );
        }
    }
    runs
}

fn panel_rect(k: usize, width: f32, height: f32) -> Rect {
    let s = height / 1080.0;
    let w = (width - 160.0 * s) / 3.0;
    Rect::new(
        80.0 * s + k as f32 * w + 14.0 * s,
        70.0 * s,
        w - 28.0 * s,
        height - 380.0 * s,
    )
}

fn item_card(width: f32, height: f32) -> Rect {
    let s = height / 1080.0;
    let (w, h) = (980.0 * s, 330.0 * s);
    Rect::new((width - w) * 0.5, height * 0.28, w, h)
}

// ───────────────────────────── the web scenes' look ─────────────────────────────
// The stage looks as the termas' and the brothel's web scenes do: the same colours, faces and
// shapes (a slanted dark box with a white rim and a rose offset shadow, a white name plate turned a
// little, thoughts on a white card up top, Chiquita in the corner with her bubble, choices as
// slanted cards with a big number, stamps in Anton with a thick rim, a three-band wipe).

const W_INK: u32 = 0xf6f1ea;
const W_DIM: u32 = 0xd6c4cf;
const W_NIGHT: u32 = 0x0f070c;
const W_ROSE: u32 = 0xff4f9a;
const W_ROSE_D: u32 = 0x8a1030;
const W_GOLD: u32 = 0xe9b949;
const W_RED: u32 = 0xe2334a;
const W_VIOLET: u32 = 0x7b4fd6;
const W_BOX: u32 = 0x10060c;
/// A CSS font size in Nunito or Anton as the atlas's size (it measures a face by its full height:
/// ascent and descent, 1.38 em in Nunito, 1.51 em in Anton).
fn rd(css: f32) -> f32 {
    css * 1.38
}
fn an(css: f32) -> f32 {
    css * 1.51
}

/// A convex shape (corners in order) in one colour.
fn poly(v: &mut Vec<MapVertex>, pts: &[[f32; 2]], color: [f32; 4]) {
    for i in 1..pts.len().saturating_sub(1) {
        push_triangle(v, [pts[0], pts[i], pts[i + 1]], color);
    }
}

/// A quad shaded from `top` (its first two corners) to `bottom` (its last two).
fn quad_grad(v: &mut Vec<MapVertex>, c: [[f32; 2]; 4], top: [f32; 4], bottom: [f32; 4]) {
    for (p, color) in [
        (c[0], top),
        (c[1], top),
        (c[2], bottom),
        (c[0], top),
        (c[2], bottom),
        (c[3], bottom),
    ] {
        v.push(MapVertex {
            position: p,
            color,
            ..MapVertex::BLANK
        });
    }
}

/// A rectangle skewed like CSS `skewX(deg)` about its middle (negative leans its top right).
fn skewed(r: Rect, deg: f32) -> [[f32; 2]; 4] {
    let k = (-deg).to_radians().tan() * r.h * 0.5;
    [
        [r.x + k, r.y],
        [r.x + r.w + k, r.y],
        [r.x + r.w - k, r.y + r.h],
        [r.x - k, r.y + r.h],
    ]
}

/// An offset shadow drawn only where it shows past the shape (as CSS box-shadow): the strips along
/// the shape's bottom and right sides. Under a see-through box a whole shadow would tint it.
fn shadow(v: &mut Vec<MapVertex>, c: [[f32; 2]; 4], dx: f32, dy: f32, color: [f32; 4]) {
    let o = moved4(c, dx, dy);
    // Right side: from the shape's right edge to the shadow's.
    poly(v, &[c[1], o[1], o[2], c[2]], color);
    // Bottom: from the shape's bottom edge to the shadow's, not again under the right strip.
    poly(v, &[c[3], c[2], o[2], o[3]], color);
}

fn moved4(c: [[f32; 2]; 4], dx: f32, dy: f32) -> [[f32; 2]; 4] {
    c.map(|p| [p[0] + dx, p[1] + dy])
}

/// A thick rim along a closed outline.
fn rim(v: &mut Vec<MapVertex>, c: &[[f32; 2]], width: f32, color: [f32; 4]) {
    for i in 0..c.len() {
        line(v, c[i], c[(i + 1) % c.len()], width, color);
    }
}

/// Words with a thick rim around them (a stamp's), the rim drawn as the words shifted all round.
#[allow(clippy::too_many_arguments)]
fn stroked(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    face: Face,
    s: &str,
    x: f32,
    y: f32,
    size: f32,
    fill: [f32; 4],
    stroke: [f32; 4],
    width: f32,
) {
    let steps = 16;
    for k in 0..steps {
        let a = k as f32 * std::f32::consts::TAU / steps as f32;
        text(
            font,
            v,
            face,
            s,
            x + a.cos() * width,
            y + a.sin() * width,
            size,
            stroke,
        );
    }
    text(font, v, face, s, x, y, size, fill);
}

/// `lines` with only their first `shown` characters (the line typing itself out).
fn typed(lines: &[String], shown: usize) -> Vec<String> {
    let mut left = shown;
    let mut out = Vec::new();
    for l in lines {
        let n = l.chars().count();
        if left == 0 {
            break;
        }
        out.push(l.chars().take(left).collect());
        left = left.saturating_sub(n + 1);
    }
    out
}

/// The vignette and the warm glow over the stage, under the box.
pub(crate) fn draw_ui(
    _font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &StageView,
    time: f32,
) {
    let s = height / 1080.0;
    // The curtain opening: black lifting off the stage.
    if view.age < 0.6 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(0x000000, 1.0 - view.age / 0.6),
        );
    }
    vignette(
        v,
        Rect::new(-60.0 * s, -60.0 * s, width + 120.0 * s, height + 120.0 * s),
        0.0,
        420.0 * s,
        rgba(0x0a0006, 0.45),
    );
    let flicker = 0.30 + 0.06 * (time * 1.3).sin() + 0.03 * (time * 3.1).sin();
    halo(
        v,
        [width * 0.18, height * 0.30],
        360.0 * s,
        260.0 * s,
        rgba(0xff7850, 0.022 * flicker / 0.3),
    );
    halo(
        v,
        [width * 0.82, height * 0.25],
        320.0 * s,
        240.0 * s,
        rgba(0xff508c, 0.016 * flicker / 0.3),
    );
}

/// The conversation on the stage: a narration or a line in the box (with its speaker's plate), a
/// thought on its card, Chiquita in her corner, or a choice as cards.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_box(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &DialogueView,
    face: Option<([f32; 2], [f32; 2])>,
    line_age: f32,
    time: f32,
) {
    let s = height / 1080.0;
    let chiquita = view.kind == 0 && view.name.to_lowercase().starts_with("chiquita");
    if !view.choices.is_empty() {
        draw_choices(font, v, width, height, view, line_age);
        return;
    }
    let shown = view.shown.min(view.text.chars().count());
    let done = shown >= view.text.chars().count();
    // ── A thought: on a white card up top.
    if view.kind == 3 {
        let size = rd(36.0) * s;
        let lines = wrap(font, Face::RoundItalic, &view.text, size, 1100.0 * s);
        let tw = lines
            .iter()
            .map(|l| text_width(font, Face::RoundItalic, l, size))
            .fold(0.0, f32::max);
        let label = format!(
            "{} · PENSANDO",
            view.name.trim_end_matches(" (piensa)").to_uppercase()
        );
        let w = tw.max(text_width(font, Face::Display, &label, an(20.0) * s)) + 80.0 * s;
        let h = 22.0 * s * 2.0 + 28.0 * s + lines.len() as f32 * size / 1.38 * 1.4;
        let r = Rect::new((width - w) * 0.5, 120.0 * s, w, h);
        let a = entrance(line_age, 0.0, 0.25);
        faded(v, a, |v| {
            turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], -0.026, |v| {
                panel(
                    v,
                    Rect::new(r.x + 10.0 * s, r.y + 10.0 * s, r.w, r.h),
                    6.0 * s,
                    Panel::fill(W_VIOLET, W_VIOLET, 0.8),
                );
                panel(v, r, 6.0 * s, Panel::fill(0xfaf6f0, 0xfaf6f0, 0.95));
                text(
                    font,
                    v,
                    Face::Display,
                    &label,
                    r.x + 40.0 * s,
                    r.y + 14.0 * s,
                    an(20.0) * s,
                    rgba(W_VIOLET, 1.0),
                );
                for (i, l) in typed(&lines, shown).iter().enumerate() {
                    text(
                        font,
                        v,
                        Face::RoundItalic,
                        l,
                        r.x + 40.0 * s,
                        r.y + 46.0 * s + i as f32 * size / 1.38 * 1.4,
                        size,
                        rgba(0x2b2335, 1.0),
                    );
                }
            });
        });
        return;
    }
    // ── Chiquita: her face in a ring and her bubble, in the top-right corner.
    if chiquita {
        let a = entrance(line_age, 0.0, 0.3);
        let bob = (time * 2.85).sin() * 8.0 * s;
        let x0 = width - 70.0 * s - 620.0 * s;
        let y0 = 70.0 * s - (1.0 - a) * 30.0 * s;
        let size = rd(30.0) * s;
        let bw = 432.0 * s;
        let lines = wrap(font, Face::Round, &view.text, size, bw - 48.0 * s);
        let bh = 18.0 * s * 2.0 + 30.0 * s + lines.len() as f32 * size / 1.38 * 1.35;
        let bubble = Rect::new(x0 + 188.0 * s, y0, bw, bh);
        faded(v, a, |v| {
            let c = [x0 + 85.0 * s, y0 + 85.0 * s + bob];
            disc(v, c, 85.0 * s + 9.0 * s, rgba(W_INK, 1.0));
            disc(v, c, 85.0 * s + 5.0 * s, rgba(W_VIOLET, 1.0));
            match face {
                Some(uv) => super::ui_paint::face(
                    v,
                    Rect::new(c[0] - 85.0 * s, c[1] - 85.0 * s, 170.0 * s, 170.0 * s),
                    85.0 * s,
                    uv,
                    1.0,
                ),
                None => disc(v, c, 85.0 * s, rgba(0x2b1d44, 1.0)),
            }
            panel(
                v,
                Rect::new(bubble.x + 8.0 * s, bubble.y + 8.0 * s, bubble.w, bubble.h),
                24.0 * s,
                Panel::fill(W_VIOLET, W_VIOLET, 1.0),
            );
            panel(v, bubble, 24.0 * s, Panel::fill(W_INK, W_INK, 1.0));
            text(
                font,
                v,
                Face::Display,
                "CHIQUITA",
                bubble.x + 24.0 * s,
                bubble.y + 10.0 * s,
                an(22.0) * s,
                rgba(W_VIOLET, 1.0),
            );
            for (i, l) in typed(&lines, shown).iter().enumerate() {
                text(
                    font,
                    v,
                    Face::Round,
                    l,
                    bubble.x + 24.0 * s,
                    bubble.y + 44.0 * s + i as f32 * size / 1.38 * 1.35,
                    size,
                    rgba(0x241a33, 1.0),
                );
            }
        });
        return;
    }
    // ── The box: slanted, dark, a white rim and a rose shadow.
    let r = Rect::new(
        150.0 * s,
        height - 46.0 * s - 250.0 * s,
        width - 300.0 * s,
        250.0 * s,
    );
    let a = entrance(line_age, 0.0, 0.22);
    let lift = (1.0 - a) * 40.0 * s;
    let c = moved4(skewed(r, -8.0), 0.0, lift);
    faded(v, a, |v| {
        shadow(v, c, 14.0 * s, 14.0 * s, rgba(W_ROSE, 0.85));
        poly(v, &c, rgba(W_BOX, 0.88));
        rim(v, &c, 3.0 * s, rgba(W_INK, 1.0));
    });
    // The speaker's plate: white, turned a little, the Queen's shadow violet, the others' rose.
    let speaking = view.kind == 0 && !view.name.is_empty();
    let queen = matches!(view.name.as_str(), "Reina" | "La Reina" | "Gorou");
    // Their face, when they are not on stage (a conversation out in the world): a slanted card
    // by the plate, half over the box's edge.
    let card = face.filter(|_| speaking);
    if let Some(uv) = card {
        let pr = Rect::new(r.x + 34.0 * s, r.y - 178.0 * s + lift, 196.0 * s, 196.0 * s);
        faded(v, a, |v| {
            turned(v, [pr.x + pr.w * 0.5, pr.y + pr.h * 0.5], -0.07, |v| {
                push_rect(
                    v,
                    Rect::new(pr.x + 10.0 * s, pr.y + 10.0 * s, pr.w, pr.h),
                    rgba(if queen { W_VIOLET } else { W_ROSE }, 1.0),
                );
                push_rect(
                    v,
                    Rect::new(
                        pr.x - 5.0 * s,
                        pr.y - 5.0 * s,
                        pr.w + 10.0 * s,
                        pr.h + 10.0 * s,
                    ),
                    rgba(W_INK, 1.0),
                );
                super::ui_paint::face(v, pr, 3.0 * s, uv, 1.0);
            });
        });
    }
    if speaking {
        let name = view.name.to_uppercase();
        let size = an(46.0) * s;
        let nw = text_width(font, Face::Display, &name, size) + 60.0 * s;
        let plate_x = if card.is_some() {
            r.x + 254.0 * s
        } else {
            r.x + 40.0 * s
        };
        let plate = Rect::new(plate_x, r.y - 46.0 * s + lift, nw, 58.0 * s);
        faded(v, a, |v| {
            turned(
                v,
                [plate.x + plate.w * 0.5, plate.y + plate.h * 0.5],
                -0.052,
                |v| {
                    push_rect(
                        v,
                        Rect::new(plate.x + 8.0 * s, plate.y + 8.0 * s, plate.w, plate.h),
                        rgba(if queen { W_VIOLET } else { W_ROSE }, 1.0),
                    );
                    push_rect(v, plate, rgba(W_INK, 1.0));
                    text(
                        font,
                        v,
                        Face::Display,
                        &name,
                        plate.x + 26.0 * s,
                        plate.y + 6.0 * s - (size - 46.0 * s) * 0.5,
                        size,
                        rgba(W_NIGHT, 1.0),
                    );
                },
            );
        });
    }
    // The words: a line bold and white, narration soft and dim, a sound gold.
    let (face, size, color) = match view.kind {
        1 => (Face::RoundSoft, rd(38.0) * s, W_DIM),
        2 => (Face::RoundItalic, rd(38.0) * s, W_GOLD),
        _ => (Face::Round, rd(38.0) * s, W_INK),
    };
    let lines = wrap(font, face, &view.text, size, r.w - 160.0 * s);
    faded(v, a, |v| {
        for (i, l) in typed(&lines, shown).iter().enumerate() {
            let y = r.y + 36.0 * s + lift + i as f32 * size / 1.38 * 1.45;
            // Along the box's slant.
            let x = r.x + 80.0 * s + (y - r.y - r.h * 0.5) * -(8.0f32).to_radians().tan() * -1.0;
            text(
                font,
                v,
                face,
                l,
                x + 2.0 * s,
                y + 2.0 * s,
                size,
                rgba(0x000000, 0.6),
            );
            text(font, v, face, l, x, y, size, rgba(color, 1.0));
        }
        // The rose triangle that asks to go on.
        if done && view.continue_key {
            let nudge = (time * std::f32::consts::TAU).sin().max(0.0) * 8.0 * s;
            let (tx, ty) = (r.x + r.w - 58.0 * s, r.y + r.h - 44.0 * s + nudge + lift);
            push_triangle(
                v,
                [
                    [tx - 18.0 * s, ty],
                    [tx + 18.0 * s, ty],
                    [tx, ty + 22.0 * s],
                ],
                rgba(W_ROSE, 1.0),
            );
        }
    });
}

/// A choice: the question in Anton, the options as slanted cards with a big number, the one in
/// focus white with a red shadow.
fn draw_choices(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    _height: f32,
    view: &DialogueView,
    age: f32,
) {
    let s = _height / 1080.0;
    if let Some(ask) = view.prompt.as_deref().filter(|a| !a.is_empty()) {
        let ask = ask.to_uppercase();
        let size = 54.0 * s;
        let w = text_width(font, Face::Display, &ask, size);
        let a = entrance(age, 0.0, 0.25);
        faded(v, a, |v| {
            turned(v, [width * 0.5, 250.0 * s], -0.052, |v| {
                text(
                    font,
                    v,
                    Face::Display,
                    &ask,
                    width * 0.5 - w * 0.5 + 6.0 * s,
                    222.0 * s - (size - 54.0 * s) * 0.5,
                    size,
                    rgba(W_ROSE_D, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Display,
                    &ask,
                    width * 0.5 - w * 0.5,
                    216.0 * s - (size - 54.0 * s) * 0.5,
                    size,
                    rgba(W_INK, 1.0),
                );
            });
        });
    }
    let size = rd(36.0) * s;
    let card_w = view
        .choices
        .iter()
        .map(|(l, _)| text_width(font, Face::Round, l, size) + 150.0 * s)
        .fold(700.0 * s, f32::max)
        .min(width - 200.0 * s);
    for (k, (label, enabled)) in view.choices.iter().enumerate() {
        let a = entrance(age, 0.05 + k as f32 * 0.07, 0.25);
        let r = Rect::new(
            width * 0.5 - card_w * 0.5,
            330.0 * s + k as f32 * (88.0 + 22.0) * s,
            card_w,
            88.0 * s,
        );
        let sel = view.selected == k && *enabled;
        let c = moved4(skewed(r, -10.0), (1.0 - a) * 120.0 * s, 0.0);
        faded(v, a, |v| {
            let off = if sel { 14.0 } else { 10.0 } * s;
            shadow(
                v,
                c,
                off,
                off,
                if sel {
                    rgba(W_RED, 1.0)
                } else {
                    rgba(W_ROSE, 0.7)
                },
            );
            poly(
                v,
                &c,
                if sel {
                    rgba(W_INK, 1.0)
                } else {
                    rgba(W_BOX, 0.88)
                },
            );
            rim(v, &c, 3.0 * s, rgba(W_INK, 1.0));
            let num = format!("{}", k + 1);
            let nx = r.x + 22.0 * s + (1.0 - a) * 120.0 * s;
            text(
                font,
                v,
                Face::Display,
                &num,
                nx,
                r.y + 4.0 * s - (an(56.0) - 56.0) * s * 0.5,
                an(56.0) * s,
                rgba(if sel { W_RED } else { W_ROSE }, 1.0),
            );
            let ink = if !enabled {
                rgba(W_DIM, 0.5)
            } else if sel {
                rgba(W_NIGHT, 1.0)
            } else {
                rgba(W_INK, 1.0)
            };
            text(
                font,
                v,
                Face::Round,
                label,
                r.x + 110.0 * s + (1.0 - a) * 120.0 * s,
                r.y + 22.0 * s - (size - 36.0 * s) * 0.5,
                size,
                ink,
            );
        });
    }
}

/// Over the box: the title sliding in, the item shown off, the stamp, the flash, the wipe's bands
/// and the outro.
pub(crate) fn draw_top(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &StageView,
    time: f32,
) {
    let s = height / 1080.0;
    // The triptych's frames.
    for (k, panel_on) in view.panels.iter().enumerate() {
        let Some((_, age)) = panel_on else { continue };
        let a = entrance(*age, 0.0, 0.32);
        let r = panel_rect(k, width, height);
        let lean = 40.0 * s;
        let drop = (1.0 - a) * -60.0 * s;
        let c = [
            [r.x + lean, r.y + drop],
            [r.x + r.w + lean, r.y + drop],
            [r.x + r.w - lean, r.y + r.h + drop],
            [r.x - lean, r.y + r.h + drop],
        ];
        rim(v, &c, 7.0 * s, rgba(W_INK, a));
    }
    // The title: a white plate sliding in from the left, the place small above the big words.
    if let Some((top, main, age)) = &view.title {
        let a_in = entrance(*age, 0.0, 0.5);
        let a_out = 1.0 - ((age - 2.0) / 0.45).clamp(0.0, 1.0);
        if a_out > 0.0 {
            let size = 76.0 * s;
            let w = text_width(font, Face::Title, main, size).max(text_width(
                font,
                Face::Display,
                top,
                an(26.0) * s,
            )) + 230.0 * s;
            let h = 18.0 * s * 2.0 + 36.0 * s + size * 1.1;
            let x = -40.0 * s - (1.0 - a_in) * (w + 80.0 * s);
            let r = Rect::new(x, 380.0 * s, w, h);
            let c = skewed(r, -12.0);
            faded(v, a_out, |v| {
                poly(v, &moved4(c, 14.0 * s, 14.0 * s), rgba(W_ROSE, 1.0));
                poly(v, &c, rgba(W_INK, 1.0));
                text(
                    font,
                    v,
                    Face::Display,
                    top,
                    r.x + 140.0 * s,
                    r.y + 8.0 * s,
                    an(26.0) * s,
                    rgba(W_ROSE_D, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Title,
                    main,
                    r.x + 140.0 * s,
                    r.y + 46.0 * s,
                    size,
                    rgba(W_NIGHT, 1.0),
                );
            });
        }
    }
    // The item: a gold panel on a darkened stage, rays turning behind.
    if let Some(item) = &view.item {
        let a = entrance(item.age, 0.0, 0.45);
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(0x0a0408, 0.6 * a),
        );
        let card = item_card(width, height);
        let c0 = [
            card.x + 30.0 * s + (card.h - 60.0 * s) * 0.5,
            card.y + card.h * 0.5,
        ];
        burst(
            v,
            [width * 0.5, height * 0.5],
            0.0,
            700.0 * s * a,
            18,
            time * 0.35,
            rgba(W_GOLD, 0.12 * a),
            rgba(W_GOLD, 0.0),
            0.0,
        );
        let grow = 0.6 + 0.4 * a;
        let r = Rect::new(
            width * 0.5 - card.w * 0.5 * grow,
            card.y + card.h * 0.5 * (1.0 - grow),
            card.w * grow,
            card.h * grow,
        );
        let c = skewed(r, -6.0);
        faded(v, a, |v| {
            shadow(v, c, 16.0 * s, 16.0 * s, rgba(W_GOLD, 0.6));
            poly(v, &c, rgba(W_BOX, 0.88));
            rim(v, &c, 3.0 * s, rgba(W_GOLD, 1.0));
            halo(v, c0, 150.0 * s, 60.0 * s, rgba(W_GOLD, 0.35));
            let tx = card.x + card.h + 20.0 * s;
            text(
                font,
                v,
                Face::Display,
                "OBJETO OBTENIDO",
                tx,
                card.y + 34.0 * s,
                an(28.0) * s,
                rgba(W_GOLD, 1.0),
            );
            text(
                font,
                v,
                Face::Title,
                &item.name,
                tx,
                card.y + 82.0 * s,
                56.0 * s,
                rgba(W_INK, 1.0),
            );
            let lines = wrap(
                font,
                Face::Round,
                &item.text,
                rd(28.0) * s,
                card.x + card.w - tx - 40.0 * s,
            );
            for (i, l) in lines.iter().enumerate() {
                text(
                    font,
                    v,
                    Face::Round,
                    l,
                    tx,
                    card.y + 166.0 * s + i as f32 * 40.0 * s,
                    rd(28.0) * s,
                    rgba(W_DIM, 1.0),
                );
            }
        });
    }
    // The stamp: big words in Anton with a thick rim, slammed down turned.
    if let Some((words, gold, age)) = &view.stamp {
        let t = age / 1.6;
        if t < 1.0 {
            let pop = if t < 0.14 {
                3.0 - 2.0 * ease_out(t / 0.14)
            } else {
                1.0 + 0.05 * ((t - 0.8) / 0.2).clamp(0.0, 1.0)
            };
            let alpha = if t < 0.14 {
                t / 0.14
            } else if t > 0.8 {
                1.0 - (t - 0.8) / 0.2
            } else {
                1.0
            };
            let angle = if t < 0.14 { -0.21 } else { -0.14 };
            let size = an(190.0) * s * pop;
            let w = text_width(font, Face::Display, words, size);
            let c = if view.item.is_some() {
                [width * 0.27, height * 0.22]
            } else {
                [width * 0.5, height * 0.38]
            };
            faded(v, alpha, |v| {
                turned(v, c, angle, |v| {
                    let (x, y) = (c[0] - w * 0.5, c[1] - size * 0.62);
                    text(
                        font,
                        v,
                        Face::Display,
                        words,
                        x + 14.0 * s,
                        y + 14.0 * s,
                        size,
                        rgba(0x000000, 0.6),
                    );
                    stroked(
                        font,
                        v,
                        Face::Display,
                        words,
                        x,
                        y,
                        size,
                        rgba(if *gold { W_GOLD } else { W_RED }, 1.0),
                        rgba(W_INK, 1.0),
                        8.0 * s * pop,
                    );
                });
            });
        }
    }
    // The flash.
    if view.flash < 0.5 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(0xffffff, 0.85 * (1.0 - view.flash / 0.5)),
        );
    }
    // The wipe: three bands (rose, night, red) sweeping over the stage; the place changes under them.
    if view.wipe < 1.0 {
        let t = view.wipe;
        let k = if t < 0.45 {
            -1.3 + 1.3 * ease_in_out(t / 0.45)
        } else if t < 0.55 {
            0.0
        } else {
            1.3 * ease_in_out((t - 0.55) / 0.45)
        };
        let x0 = k * width;
        let lean = (12.0f32).to_radians().tan() * height;
        let band = |v: &mut Vec<MapVertex>, from: f32, to: f32, color: [f32; 4]| {
            let (a, b) = (x0 + from * width, x0 + to * width);
            poly(
                v,
                &[
                    [a + lean * 0.5, 0.0],
                    [b + lean * 0.5, 0.0],
                    [b - lean * 0.5, height],
                    [a - lean * 0.5, height],
                ],
                color,
            );
        };
        band(v, -0.1, 0.36, rgba(W_ROSE, 1.0));
        band(v, 0.33, 0.69, rgba(W_NIGHT, 1.0));
        band(v, 0.66, 1.1, rgba(W_RED, 1.0));
    }
    // The outro: what the scene unlocked.
    if let Some((unlocks, age)) = &view.outro {
        let a = entrance(*age, 0.0, 0.45);
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(W_NIGHT, 0.75 * a),
        );
        let size = an(110.0) * s;
        let title = view.outro_title.as_str();
        let w = text_width(font, Face::Display, title, size);
        let y = height * 0.28;
        faded(v, a, |v| {
            turned(v, [width * 0.5, y + size * 0.5], 0.0, |v| {
                text(
                    font,
                    v,
                    Face::Display,
                    title,
                    width * 0.5 - w * 0.5 + 10.0 * s,
                    y + 10.0 * s,
                    size,
                    rgba(W_ROSE_D, 1.0),
                );
                text(
                    font,
                    v,
                    Face::Display,
                    title,
                    width * 0.5 - w * 0.5,
                    y,
                    size,
                    rgba(W_GOLD, 1.0),
                );
            });
        });
        let total: f32 = unlocks
            .iter()
            .map(|u| text_width(font, Face::Round, u, rd(30.0) * s) + 60.0 * s)
            .sum::<f32>()
            + 20.0 * s * unlocks.len().saturating_sub(1) as f32;
        let mut x = width * 0.5 - total * 0.5;
        for (i, u) in unlocks.iter().enumerate() {
            let k = entrance(*age, 0.3 + i as f32 * 0.1, 0.3);
            let w = text_width(font, Face::Round, u, rd(30.0) * s) + 60.0 * s;
            let r = Rect::new(x, height * 0.5, w, 64.0 * s);
            let c = skewed(r, -8.0);
            faded(v, k, |v| {
                poly(v, &c, rgba(W_BOX, 0.88));
                rim(v, &c, 3.0 * s, rgba(W_INK, 1.0));
                text(
                    font,
                    v,
                    Face::Round,
                    u,
                    r.x + 30.0 * s,
                    r.y + 8.0 * s,
                    rd(30.0) * s,
                    rgba(W_INK, 1.0),
                );
            });
            x += w + 20.0 * s;
        }
        let k = entrance(*age, 0.9, 0.3);
        let hint = "F · SEGUIR";
        let hw = text_width(font, Face::Display, hint, an(36.0) * s) + 80.0 * s;
        let r = Rect::new(width * 0.5 - hw * 0.5, height * 0.66, hw, 64.0 * s);
        let c = skewed(r, -8.0);
        faded(v, k, |v| {
            poly(v, &moved4(c, 8.0 * s, 8.0 * s), rgba(W_RED, 1.0));
            poly(v, &c, rgba(W_INK, 1.0));
            text(
                font,
                v,
                Face::Display,
                hint,
                r.x + 40.0 * s,
                r.y + 2.0 * s,
                an(36.0) * s,
                rgba(W_NIGHT, 1.0),
            );
        });
    }
}

/// The colour band behind whoever speaks (violet on the Queen's side, rose on the other), striped.
pub(crate) fn speaker_slash(
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    right: bool,
    alpha: f32,
) {
    let s = height / 1080.0;
    let w = 760.0 * s;
    let x = if right {
        width - 40.0 * s - w
    } else {
        40.0 * s
    };
    let r = Rect::new(x, -60.0 * s, w, height + 120.0 * s);
    let c = skewed(r, -14.0);
    let (top, bottom) = if right {
        (rgba(W_ROSE, 0.0), rgba(W_ROSE_D, 0.55 * alpha))
    } else {
        (rgba(W_VIOLET, 0.0), rgba(0x3c1e78, 0.55 * alpha))
    };
    quad_grad(v, c, top, bottom);
    // Its stripes.
    let mut sx = 0.0;
    while sx < w {
        let lean = (14.0f32).to_radians().tan() * r.h * 0.5;
        let (a, b) = (x + sx, x + sx + 3.0 * s);
        quad_grad(
            v,
            [
                [a + lean, r.y],
                [b + lean, r.y],
                [b - lean, r.y + r.h],
                [a - lean, r.y + r.h],
            ],
            rgba(0xffffff, 0.0),
            rgba(0xffffff, 0.08 * alpha),
        );
        sx += 22.0 * s;
    }
}

fn push_quad(v: &mut Vec<MapVertex>, c: [[f32; 2]; 4], color: [f32; 4]) {
    push_triangle(v, [c[0], c[1], c[2]], color);
    push_triangle(v, [c[0], c[2], c[3]], color);
}

fn line(v: &mut Vec<MapVertex>, p: [f32; 2], q: [f32; 2], width: f32, color: [f32; 4]) {
    let d = [q[0] - p[0], q[1] - p[1]];
    let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
    let n = [-d[1] / l * width * 0.5, d[0] / l * width * 0.5];
    push_quad(
        v,
        [
            [p[0] + n[0], p[1] + n[1]],
            [q[0] + n[0], q[1] + n[1]],
            [q[0] - n[0], q[1] - n[1]],
            [p[0] - n[0], p[1] - n[1]],
        ],
        color,
    );
}

/// The stage's pictures as textures: loaded the first time each is shown, with their mips; the
/// least recently used dropped past a dozen.
pub(crate) struct StagePictures {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    loaded: HashMap<String, (wgpu::BindGroup, PictureInfo)>,
    order: Vec<String>,
    failed: std::collections::HashSet<String>,
    root: std::path::PathBuf,
}

const STAGE_KEEP: usize = 14;

impl StagePictures {
    pub(crate) fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("demon-queen-stage-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            layout: layout.clone(),
            sampler,
            loaded: HashMap::new(),
            order: Vec::new(),
            failed: Default::default(),
            root: crate::exploration3d_state::data_dir().join("assets"),
        }
    }

    /// The picture's size and head row, loading it the first time.
    pub(crate) fn ensure(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        name: &str,
    ) -> Option<PictureInfo> {
        if let Some((_, info)) = self.loaded.get(name) {
            let info = *info;
            if let Some(i) = self.order.iter().position(|n| n == name) {
                let n = self.order.remove(i);
                self.order.push(n);
            }
            return Some(info);
        }
        if self.failed.contains(name) {
            return None;
        }
        let png = self.root.join(format!("{name}.png"));
        let Ok(image) = image::open(&png) else {
            eprintln!("[stage] picture missing: {}", png.display());
            self.failed.insert(name.to_owned());
            return None;
        };
        let rgba = image.to_rgba8();
        let (w, h) = (rgba.width(), rgba.height());
        // The first row with something solid (in steps of 4 columns): where a figure's head is.
        let top = (0..h)
            .find(|&y| (0..w).step_by(4).any(|x| rgba.get_pixel(x, y)[3] > 128))
            .unwrap_or(0);
        let group = MapPictures::upload(
            device,
            queue,
            &self.layout,
            &self.sampler,
            w,
            h,
            rgba.as_raw(),
        );
        let info = PictureInfo {
            size: [w as f32, h as f32],
            top: top as f32,
        };
        self.loaded.insert(name.to_owned(), (group, info));
        self.order.push(name.to_owned());
        while self.order.len() > STAGE_KEEP {
            let old = self.order.remove(0);
            self.loaded.remove(&old);
        }
        Some(info)
    }

    pub(crate) fn group(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.loaded.get(name).map(|(g, _)| g)
    }

    /// Already loaded: its size and head row.
    pub(crate) fn info(&self, name: &str) -> Option<PictureInfo> {
        self.loaded.get(name).map(|(_, i)| *i)
    }
}

fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
