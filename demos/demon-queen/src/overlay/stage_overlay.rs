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
    /// The one before it and seconds since it changed: it fades out over the new one.
    pub prev: Option<String>,
    pub swap: f32,
    pub right: bool,
    /// Seconds since she came on, or since she started going off (`leaving`).
    pub enter: f32,
    pub leaving: bool,
    /// Seconds since the last shake and the last hop (large: none).
    pub shake: f32,
    pub bump: f32,
    /// Speaking (or thinking): drawn over the others; and her light now (1 lit, 0.55 dimmed,
    /// fading between them when the speaker changes).
    pub lit: bool,
    pub light: f32,
    /// Turning on the spot: the frames, how far through them (seconds), and the picture before.
    pub spin: Option<(Vec<String>, f32, String)>,
    /// Her size against the others (1: the cast's height; a goblin is shorter); with `window`, her
    /// canvas's height in screens, as the web scenes draw her.
    pub scale: f32,
    /// The part of her canvas shown from its top, down to mid-leg (the web's window), and where her
    /// centre stands across the screen (fractions; `None`: her side's third, framed by her head).
    pub window: Option<f32>,
    pub x: Option<f32>,
}

/// A panel of the triptych: its picture, how it sits inside (`[height, x %, y]` in 1080p pixels,
/// as the web's `background-size` and `-position`; `None`: the web's usual), seconds since it came,
/// and since it started flying off.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StagePanel {
    pub image: String,
    pub crop: Option<[f32; 3]>,
    pub age: f32,
    pub out: Option<f32>,
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
    /// A date's memory coming on (`@recuerdo`): its name and seconds since (the ribbon).
    pub memory: Option<(String, f32)>,
    pub item: Option<StageItem>,
    /// The triptych's panels, left to right.
    pub panels: Vec<Option<StagePanel>>,
    /// A picture revealed in three quick cuts, and seconds since it began (the panels fly off
    /// first).
    pub reveal: Option<(String, f32)>,
    /// The scene is over: what it unlocked, and the card's age.
    pub outro: Option<(Vec<String>, f32)>,
    /// The outro's banner («ESCENA COMPLETA», «FOTO GUARDADA»…).
    pub outro_title: String,
    /// Seconds the stage has been up (the curtain opening).
    pub age: f32,
    /// The box's accent colour of whoever speaks (for the slashes).
    pub accent: u32,
    /// The side of the one speaking or thinking (true: the right), the side before, and seconds
    /// since it changed (the band behind her slides in, the old one out).
    pub speaker: Option<bool>,
    pub speaker_prev: Option<bool>,
    pub speaker_age: f32,
    /// Seconds the line or the choice on show has been up (its words typing), and the box's (it
    /// comes in once, when it shows after a card, a bubble or a beat; not with every line).
    pub line_age: f32,
    pub box_age: f32,
    /// A beat plays (a title, a wipe covering, a reveal, a turn, an item shown off): the box waits.
    pub waiting: bool,
    /// The web scene's colours: 0 the brothel's rose, 1 the springs' teal (with steam), 2 act
    /// one's lilac.
    pub theme: u8,
    /// Every picture the scene will show, to load before they come on.
    pub preload: Vec<String>,
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

/// Where an actor stands: her picture's rect. With the web's framing (`window`): her canvas drawn
/// `scale` screens tall, the top `window` of it standing on the screen's foot, whatever the pose's
/// own framing (a raised arm does not move her). Else her head near the top and the screen's foot
/// cutting her at the thighs.
fn actor_rect(
    info: PictureInfo,
    right: bool,
    scale: f32,
    window: Option<f32>,
    x: Option<f32>,
    width: f32,
    height: f32,
) -> Rect {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    let cx = x.map_or(if right { width * 0.735 } else { width * 0.265 }, |f| {
        width * f
    });
    if let Some(part) = window {
        let h = height * scale;
        let w = h * info.size[0] / info.size[1].max(1.0);
        return Rect::new(cx - w * 0.5, height - h * part, w, h);
    }
    // From the head down, the figure fills 1.08 screens: the screen's foot cuts it at mid-thigh.
    // A smaller one stands on the same floor: her head lower, cut at the same height of her body.
    let head = height * (0.055 + 1.08 * (1.0 - scale));
    let k = height * 1.08 * scale / (info.size[1] - info.top).max(1.0);
    let (w, h) = (info.size[0] * k, info.size[1] * k);
    Rect::new(cx - w * 0.5, head - info.top * k, w, h)
}

/// CSS's `cubic-bezier(x1, y1, x2, y2)` at `t` (clamped to 0..1): the web scenes' easings, their
/// overshoot and all.
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let curve = |u: f32, a: f32, b: f32| {
        3.0 * (1.0 - u) * (1.0 - u) * u * a + 3.0 * (1.0 - u) * u * u * b + u * u * u
    };
    let slope = |u: f32| {
        3.0 * (1.0 - u) * (1.0 - u) * x1
            + 6.0 * (1.0 - u) * u * (x2 - x1)
            + 3.0 * u * u * (1.0 - x2)
    };
    let mut u = t;
    for _ in 0..8 {
        let d = slope(u);
        if d.abs() < 1e-5 {
            break;
        }
        u = (u - (curve(u, x1, x2) - t) / d).clamp(0.0, 1.0);
    }
    curve(u, y1, y2)
}

/// CSS's `ease`, the web's transitions' default.
fn ease(t: f32) -> f32 {
    bezier(0.25, 0.1, 0.25, 1.0, t)
}

/// The web's entrance, with its little overshoot (`cubic-bezier(.2,.9,.25,1.15)`).
fn pop(t: f32) -> f32 {
    bezier(0.2, 0.9, 0.25, 1.15, t)
}

/// The web's `shake` (0.4 s): a shiver sideways dying out, in 1080p pixels.
fn shake_x(t: f32) -> f32 {
    if !(0.0..0.4).contains(&t) {
        return 0.0;
    }
    const KEYS: [f32; 6] = [0.0, -12.0, 10.0, -6.0, 4.0, 0.0];
    let f = t / 0.4 * 5.0;
    let i = (f as usize).min(4);
    KEYS[i] + (KEYS[i + 1] - KEYS[i]) * ease(f - i as f32)
}

/// The web's `bump` (0.32 s): up 14 px and 2 % bigger at 30 %, and back (1080p pixels, scale).
fn bump_of(t: f32) -> (f32, f32) {
    if !(0.0..0.32).contains(&t) {
        return (0.0, 1.0);
    }
    let f = t / 0.32;
    let k = if f < 0.3 {
        ease(f / 0.3)
    } else {
        1.0 - ease((f - 0.3) / 0.7)
    };
    (14.0 * k, 1.0 + 0.02 * k)
}

/// How long a new face takes to take over from the old one.
const SWAP: f32 = 0.15;
/// A turn's frames, as the web's: one every 0.52 s, each crossing over the last in 0.32 s.
const SPIN_EACH: f32 = 0.52;
const SPIN_FADE: f32 = 0.32;

/// The pictures of the stage, in the order they are drawn.
pub(crate) fn pictures(
    view: &StageView,
    info: &dyn Fn(&str) -> Option<PictureInfo>,
    width: f32,
    height: f32,
) -> Vec<PictureRun> {
    let mut runs: Vec<PictureRun> = Vec::new();
    let s = height / 1080.0;
    let lk = look(view.theme);
    let dim_bg = if view.outro.is_some() { 0.55 } else { 1.0 };
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
    // The band of colour behind whoever speaks, over the place and under the cast: it slides in
    // from beyond her side as she starts, and the last one slides back out.
    if let Some(run) = runs.last_mut() {
        let t = view.speaker_age;
        let moved = bezier(0.2, 0.9, 0.25, 1.0, t / 0.45);
        let shown = ease(t / 0.3);
        if let Some(prev) = view.speaker_prev.filter(|p| Some(*p) != view.speaker) {
            speaker_slash(
                &mut run.vertices,
                width,
                height,
                prev,
                1.0 - shown,
                moved,
                &lk,
            );
        }
        if let Some(side) = view.speaker {
            speaker_slash(
                &mut run.vertices,
                width,
                height,
                side,
                shown,
                1.0 - moved,
                &lk,
            );
        }
    }
    // The cast: the dimmed one first, the one speaking over her.
    let mut order: Vec<&StageActor> = view.actors.iter().collect();
    order.sort_by_key(|a| a.lit);
    for a in order {
        // Coming on: in from beyond her side with the web's overshoot, fading in; going off: back
        // out the same way.
        let moved = pop(a.enter / 0.45);
        let shown = ease(a.enter / 0.35);
        let (alpha, out) = if a.leaving {
            (1.0 - shown, moved)
        } else {
            (shown, 1.0 - moved)
        };
        if alpha <= 0.01 {
            continue;
        }
        let slide = out * 160.0 * s * if a.right { 1.0 } else { -1.0 };
        let shake = shake_x(a.shake) * s;
        let (hop, grow) = bump_of(a.bump);
        let hop = hop * s;
        let mut layers: Vec<(&str, f32)> = Vec::new();
        match &a.spin {
            Some((frames, t, before)) if !frames.is_empty() => {
                // As the web turns her: two layers taking turns, each new frame crossing over the
                // last (the first over the picture before the turn).
                let i = ((t / SPIN_EACH) as usize).min(frames.len() - 1);
                let k = ease_in_out((t - i as f32 * SPIN_EACH) / SPIN_FADE);
                let last = if i == 0 {
                    before.as_str()
                } else {
                    frames[i - 1].as_str()
                };
                if i % 2 == 0 {
                    layers.push((last, 1.0));
                    layers.push((frames[i].as_str(), k));
                } else {
                    layers.push((frames[i].as_str(), 1.0));
                    layers.push((last, 1.0 - k));
                }
            }
            _ => {
                // The new face under, the old one over it fading out: never see-through, no pop.
                layers.push((a.image.as_str(), 1.0));
                if let (Some(prev), true) = (&a.prev, a.swap < SWAP) {
                    layers.push((prev.as_str(), 1.0 - ease(a.swap / SWAP)));
                }
            }
        }
        for (image, layer_alpha) in layers {
            if layer_alpha <= 0.002 {
                continue;
            }
            let Some(i) = info(image) else { continue };
            let r = actor_rect(i, a.right, a.scale, a.window, a.x, width, height);
            let (cx, foot) = (r.x + r.w * 0.5, r.y + r.h);
            let (w, h) = (r.w * grow, r.h * grow);
            let x = cx - w * 0.5 + slide + shake;
            let y = foot - h - hop;
            let light = a.light * dim_bg;
            let run = runs_push(&mut runs, image, false);
            quad_uv(
                run,
                [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                [light, light, light, alpha * layer_alpha],
            );
        }
    }
    // The triptych, and a reveal's cuts: slanted panels, each picture upright inside.
    for p in panels_on_show(view, width, height) {
        let Some(i) = info(p.image) else { continue };
        let ih = p.crop[0] * s;
        let iw = ih * i.size[0] / i.size[1].max(1.0);
        // As the web's: the picture sits in the panel grown 60 px each side and 20 px up.
        let (bx, bw) = (p.rect.x - 60.0 * s, p.rect.w + 120.0 * s);
        let ix = bx + (bw - iw) * p.crop[1] / 100.0;
        let iy = p.rect.y - 20.0 * s + p.crop[2] * s;
        let corners = panel_corners(p.rect, 0.0);
        let uvs = corners.map(|c| [(c[0] - ix) / iw, (c[1] - iy) / ih]);
        let run = runs_push(&mut runs, p.image, false);
        quad_uv(
            run,
            corners.map(|c| [c[0], c[1] + p.dy]),
            uvs,
            [1.0, 1.0, 1.0, p.alpha],
        );
    }
    // The item's picture, on its card (over the overlay's shapes), growing with it.
    if let Some(item) = &view.item {
        if let Some(i) = info(&item.icon) {
            let card = item_card(width, height);
            let a = ease(item.age / 0.3);
            let grow = 0.6 + 0.4 * bezier(0.2, 0.9, 0.25, 1.3, item.age / 0.45);
            let side = (card.h - 60.0 * s) * grow;
            let k = (side / i.size[0]).min(side / i.size[1]);
            let (w, h) = (i.size[0] * k, i.size[1] * k);
            let mid = [width * 0.5, card.y + card.h * 0.5];
            let cx = mid[0] + (card.x + 30.0 * s + (card.h - 60.0 * s) * 0.5 - mid[0]) * grow;
            let (x, y) = (cx - w * 0.5, mid[1] - h * 0.5);
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

/// A triptych panel's rect before its slant, as the web lays them out: three 470×690 a little
/// apart near the top; a reveal's cuts bigger (560×980), in the middle.
fn panel_rect(k: usize, fast: bool, width: f32, height: f32) -> Rect {
    let s = height / 1080.0;
    let (w, h, top) = if fast {
        (560.0, 980.0, 50.0)
    } else {
        (470.0, 690.0, 40.0)
    };
    let total = (3.0 * w + 2.0 * 26.0) * s;
    let x0 = (width - total) * 0.5;
    Rect::new(x0 + k as f32 * (w + 26.0) * s, top * s, w * s, h * s)
}

/// How a panel moves: rising from below with the web's overshoot as it comes (0.5 s), flying off
/// the top as it goes (0.45 s): its drop and its opacity.
fn panel_motion(age: f32, out: Option<f32>, h: f32) -> (f32, f32) {
    match out {
        Some(o) => (
            -1.3 * h * bezier(0.6, 0.0, 0.8, 0.4, o / 0.45),
            1.0 - ease(o / 0.45),
        ),
        None => (1.2 * h * (1.0 - pop(age / 0.5)), ease(age / 0.3)),
    }
}

/// A panel's corners, slanted as the web's `skewX(-10deg)`, dropped by `dy`.
fn panel_corners(r: Rect, dy: f32) -> [[f32; 2]; 4] {
    moved4(skewed(r, -10.0), 0.0, dy)
}

/// A panel on show: its rect, picture, how the picture sits in it, its motion and its colour.
struct PanelShow<'a> {
    rect: Rect,
    image: &'a str,
    crop: [f32; 3],
    dy: f32,
    alpha: f32,
    tone: u32,
}

/// The panels on show: the triptych's, and a reveal's three quick cuts of her (her face, her
/// middle, her legs; once the panels have flown off, each with a flash, then off themselves).
fn panels_on_show(view: &StageView, width: f32, height: f32) -> Vec<PanelShow<'_>> {
    const TONES: [u32; 3] = [W_VIOLET, 0x18c3b3, W_RED];
    const CUTS: [[f32; 3]; 3] = [
        [2600.0, 50.0, -150.0],
        [2100.0, 50.0, -620.0],
        [1900.0, 50.0, -880.0],
    ];
    let mut out = Vec::new();
    for (k, p) in view.panels.iter().enumerate().take(3) {
        let Some(p) = p else { continue };
        let rect = panel_rect(k, false, width, height);
        let (dy, alpha) = panel_motion(p.age, p.out, rect.h);
        if alpha > 0.01 {
            out.push(PanelShow {
                rect,
                image: &p.image,
                crop: p.crop.unwrap_or([1080.0, 50.0, -20.0]),
                dy,
                alpha,
                tone: TONES[k],
            });
        }
    }
    if let Some((image, age)) = &view.reveal {
        let t = age - 0.56;
        for (k, crop) in CUTS.iter().enumerate() {
            let came = t - (0.12 + 0.33 * k as f32);
            if came < 0.0 {
                continue;
            }
            let rect = panel_rect(k, true, width, height);
            let gone = t - 2.1;
            let (dy, alpha) = panel_motion(came, (gone >= 0.0).then_some(gone), rect.h);
            if alpha > 0.01 {
                out.push(PanelShow {
                    rect,
                    image,
                    crop: *crop,
                    dy,
                    alpha,
                    tone: TONES[k],
                });
            }
        }
    }
    out
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
/// A web scene's colours.
#[derive(Clone, Copy)]
pub(crate) struct Look {
    /// The box's offset shadow, the next-arrow, the choices' numbers and shadows, the title's
    /// shadow and the wipe's first band.
    accent: u32,
    /// The question's shadow, the title's small line, the outro's shadow.
    accent_d: u32,
    night: u32,
    fill: u32,
    fill_alpha: f32,
    dim: u32,
    /// The name plate's shadow: the Queen's, and the others'.
    queen: u32,
    other: u32,
    /// The bands behind whoever speaks (left, right): the colour 30 % down and at their foot, and
    /// their opacity.
    bands: [([u32; 2], [f32; 2]); 2],
    /// Steam rising over the stage (the springs) instead of the candles' glow.
    steam: bool,
}

/// The colours of each place's web scene: 0 the brothel's (rose), 1 the springs' (teal and red,
/// with steam), 2 act one's (lilac).
pub(crate) fn look(theme: u8) -> Look {
    match theme {
        1 => Look {
            accent: 0x18c3b3,
            accent_d: 0x0b6f69,
            night: 0x0c0a12,
            fill: 0x0a0810,
            fill_alpha: 0.86,
            dim: 0xc9bfd6,
            queen: 0x18c3b3,
            other: W_RED,
            bands: [
                ([0x18c3b3, 0x0b6f69], [0.55, 0.75]),
                ([0xe2334a, 0x781428], [0.45, 0.75]),
            ],
            steam: true,
        },
        2 => Look {
            accent: 0xa77bff,
            accent_d: 0x3b1f6e,
            night: 0x0b0814,
            fill: 0x0c0816,
            fill_alpha: 0.88,
            dim: W_DIM,
            queen: W_VIOLET,
            other: 0xa77bff,
            bands: [
                ([0x7b4fd6, 0x3c1e78], [0.5, 0.75]),
                ([0xa77bff, 0x3b1f6e], [0.45, 0.78]),
            ],
            steam: false,
        },
        _ => Look {
            accent: W_ROSE,
            accent_d: W_ROSE_D,
            night: W_NIGHT,
            fill: W_BOX,
            fill_alpha: 0.88,
            dim: W_DIM,
            queen: W_VIOLET,
            other: W_ROSE,
            bands: [
                ([0x7b4fd6, 0x3c1e78], [0.5, 0.75]),
                ([W_ROSE, W_ROSE_D], [0.45, 0.78]),
            ],
            steam: false,
        },
    }
}

/// A CSS font size in Nunito or Anton as the atlas's size (it measures a face by its full height:
/// ascent and descent, 1.38 em in Nunito, 1.51 em in Anton).
fn rd(css: f32) -> f32 {
    css * 1.38
}
fn an(css: f32) -> f32 {
    css * 1.51
}

/// A convex shape (corners in order) in one colour.
/// A flat rectangle with soft edges (it is often turned: a stepped edge shows).
fn soft_rect(v: &mut Vec<MapVertex>, r: Rect, color: [f32; 4]) {
    poly(
        v,
        &[
            [r.x, r.y],
            [r.x + r.w, r.y],
            [r.x + r.w, r.y + r.h],
            [r.x, r.y + r.h],
        ],
        color,
    );
}

fn poly(v: &mut Vec<MapVertex>, pts: &[[f32; 2]], color: [f32; 4]) {
    for i in 1..pts.len().saturating_sub(1) {
        push_triangle(v, [pts[0], pts[i], pts[i + 1]], color);
    }
    // A soft edge, so a slanted plate is not stepped.
    push_outline_fringe(v, pts, color);
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

/// Under the box: the curtain opening, the vignette, the place's air (the candles' warm glow, or
/// the springs' steam drifting), and the triptych's frames.
pub(crate) fn draw_ui(
    _font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &StageView,
    time: f32,
) {
    let s = height / 1080.0;
    let lk = look(view.theme);
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
    if lk.steam {
        // Steam drifting up and down, 9 s each way, as over the springs' web scene: two faint
        // breaths of white low down, hardly there.
        let k = ease_in_out(0.5 - 0.5 * (time * PI / 9.0).cos());
        let (strength, dy) = (0.25 + 0.2 * k, (20.0 - 50.0 * k) * s);
        for (cx, cy, rx, ry, a) in [
            (0.20, 0.80, 0.30, 0.20, 0.18),
            (0.75, 0.70, 0.25, 0.18, 0.14),
        ] {
            let (w, h) = (width * rx * 0.5, height * ry * 0.4);
            soft(
                v,
                Rect::new(width * cx - w * 0.5, height * cy - h * 0.5 + dy, w, h),
                h * 0.5,
                height * ry * 0.5,
                rgba(0xffffff, a * strength * 0.4),
                false,
            );
        }
    } else {
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
    // The triptych's frames (under the box and the thoughts' card, as in the web): a coloured
    // offset shadow, faint stripes along their slant and a white rim.
    for p in panels_on_show(view, width, height) {
        let c = panel_corners(p.rect, p.dy);
        faded(v, p.alpha, |v| {
            shadow(v, c, 14.0 * s, 14.0 * s, rgba(p.tone, 1.0));
            let mut x = 0.0;
            while x < p.rect.w {
                let (a, b) = (x, (x + 3.0 * s).min(p.rect.w));
                poly(
                    v,
                    &[
                        [c[0][0] + a, c[0][1]],
                        [c[0][0] + b, c[0][1]],
                        [c[3][0] + b, c[3][1]],
                        [c[3][0] + a, c[3][1]],
                    ],
                    rgba(0xffffff, 0.05),
                );
                x += 26.0 * s;
            }
            rim(v, &c, 5.0 * s, rgba(W_INK, 1.0));
        });
    }
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
    box_age: f32,
    time: f32,
    alive: bool,
    theme: u8,
) {
    let s = height / 1080.0;
    let lk = look(theme);
    // By her id: her name on show is translated («Tiny» in English), and then her line went nowhere.
    let chiquita = view.kind == 0 && view.speaker == "chiquita";
    if !view.choices.is_empty() {
        draw_choices(font, v, width, height, view, line_age, &lk);
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
    // ── Chiquita: her bubble beside her head while she says it herself, up in her corner (`alive`:
    // her 3D self is on screen; its tail points at her), else her face in a ring and the bubble
    // beside it, top right.
    if chiquita {
        let a = entrance(line_age, 0.0, 0.3);
        let bob = (time * 2.85).sin() * 8.0 * s;
        let size = rd(30.0) * s;
        let bw = 432.0 * s;
        let lines = wrap(font, Face::Round, &view.text, size, bw - 48.0 * s);
        let bh = 18.0 * s * 2.0 + 30.0 * s + lines.len() as f32 * size / 1.38 * 1.35;
        let (bubble, tail) = if alive {
            let (right, bottom) = (width - 316.0 * s, 628.0 * s);
            let b = Rect::new(right - bw, bottom - bh + (1.0 - a) * 20.0 * s, bw, bh);
            let tip = [width - 258.0 * s, 668.0 * s];
            let tail = [
                [right - 2.0 * s, b.y + b.h - 70.0 * s],
                [right - 2.0 * s, b.y + b.h - 26.0 * s],
                tip,
            ];
            (b, Some(tail))
        } else {
            let x0 = width - 70.0 * s - 620.0 * s;
            let y0 = 70.0 * s - (1.0 - a) * 30.0 * s;
            (Rect::new(x0 + 188.0 * s, y0, bw, bh), None)
        };
        faded(v, a, |v| {
            if !alive {
                let c = [bubble.x - 103.0 * s, bubble.y + 85.0 * s + bob];
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
            }
            if let Some(t) = &tail {
                poly(
                    v,
                    &t.map(|p| [p[0] + 8.0 * s, p[1] + 8.0 * s]),
                    rgba(W_VIOLET, 1.0),
                );
            }
            panel(
                v,
                Rect::new(bubble.x + 8.0 * s, bubble.y + 8.0 * s, bubble.w, bubble.h),
                24.0 * s,
                Panel::fill(W_VIOLET, W_VIOLET, 1.0),
            );
            panel(v, bubble, 24.0 * s, Panel::fill(W_INK, W_INK, 1.0));
            if let Some(t) = &tail {
                poly(v, t, rgba(W_INK, 1.0));
            }
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
    // ── The box: slanted, dark, a white rim and the place's shadow. It comes in once (when it
    // shows after a card, a bubble or a beat) and stays; only its words change.
    let r = Rect::new(
        150.0 * s,
        height - 46.0 * s - 250.0 * s,
        width - 300.0 * s,
        250.0 * s,
    );
    let a = ease(box_age / 0.25);
    let lift = (1.0 - ease(box_age / 0.3)) * 40.0 * s;
    let c = moved4(skewed(r, -8.0), 0.0, lift);
    faded(v, a, |v| {
        shadow(v, c, 14.0 * s, 14.0 * s, rgba(lk.accent, 0.85));
        poly(v, &c, rgba(lk.fill, lk.fill_alpha));
        rim(v, &c, 3.0 * s, rgba(W_INK, 1.0));
    });
    // The speaker's plate: white, turned a little, the Queen's shadow violet, the others' rose.
    let speaking = view.kind == 0 && !view.name.is_empty();
    // By her id, as Chiquita above: her name on show is translated («Queen», «女王»…).
    let queen = matches!(
        view.speaker.trim_end_matches("_thought"),
        "gorou" | "reina" | "queen" | "queen_original"
    );
    // Their face, when they are not on stage (a conversation out in the world): a slanted card
    // by the plate, half over the box's edge.
    let card = face.filter(|_| speaking);
    if let Some(uv) = card {
        let pr = Rect::new(r.x + 34.0 * s, r.y - 178.0 * s + lift, 196.0 * s, 196.0 * s);
        faded(v, a, |v| {
            turned(v, [pr.x + pr.w * 0.5, pr.y + pr.h * 0.5], -0.07, |v| {
                soft_rect(
                    v,
                    Rect::new(pr.x + 10.0 * s, pr.y + 10.0 * s, pr.w, pr.h),
                    rgba(if queen { lk.queen } else { lk.other }, 1.0),
                );
                soft_rect(
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
                    soft_rect(
                        v,
                        Rect::new(plate.x + 8.0 * s, plate.y + 8.0 * s, plate.w, plate.h),
                        rgba(if queen { lk.queen } else { lk.other }, 1.0),
                    );
                    soft_rect(v, plate, rgba(W_INK, 1.0));
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
        1 => (Face::RoundSoft, rd(38.0) * s, lk.dim),
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
                rgba(lk.accent, 1.0),
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
    lk: &Look,
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
                    rgba(lk.accent_d, 1.0),
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
                    rgba(lk.accent, 0.7)
                },
            );
            poly(
                v,
                &c,
                if sel {
                    rgba(W_INK, 1.0)
                } else {
                    rgba(lk.fill, lk.fill_alpha)
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
                rgba(if sel { W_RED } else { lk.accent }, 1.0),
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
    let lk = look(view.theme);
    // The title: a white plate sliding in from the left, the place small above the big words; it
    // stands 1.8 s and slides back out.
    if let Some((top, main, age)) = &view.title {
        let k = bezier(0.2, 0.9, 0.25, 1.0, age / 0.5)
            * (1.0 - bezier(0.2, 0.9, 0.25, 1.0, (age - 1.8) / 0.5));
        if k > 0.0 {
            let size = 76.0 * s;
            let w = text_width(font, Face::Title, main, size).max(text_width(
                font,
                Face::Display,
                top,
                an(26.0) * s,
            )) + 230.0 * s;
            let h = 18.0 * s * 2.0 + 36.0 * s + size * 1.1;
            let x = -1.1 * w + (1.1 * w - 40.0 * s) * k;
            let r = Rect::new(x, 380.0 * s, w, h);
            let c = skewed(r, -12.0);
            faded(v, 1.0, |v| {
                poly(v, &moved4(c, 14.0 * s, 14.0 * s), rgba(lk.accent, 1.0));
                poly(v, &c, rgba(W_INK, 1.0));
                text(
                    font,
                    v,
                    Face::Display,
                    top,
                    r.x + 140.0 * s,
                    r.y + 8.0 * s,
                    an(26.0) * s,
                    rgba(lk.accent_d, 1.0),
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
    // A date's memory: a night plate edged in gold slides in at the top right, «RECUERDO» small
    // over its name, a glint running across; it stands and slides back out.
    if let Some((name, age)) = &view.memory {
        let k = bezier(0.2, 0.9, 0.25, 1.0, (age - 0.25) / 0.5)
            * (1.0 - bezier(0.2, 0.9, 0.25, 1.0, (age - 3.7) / 0.5));
        if k > 0.0 {
            let label = "RECUERDO";
            let (small, big) = (an(24.0) * s, 46.0 * s);
            let w = text_width(font, Face::Title, name, big).max(text_width(
                font,
                Face::Display,
                label,
                small,
            )) + 150.0 * s;
            let h = 34.0 * s + small * 1.2 + big * 1.15;
            let x = width + 40.0 * s - (w + 100.0 * s) * k;
            let r = Rect::new(x, 70.0 * s, w, h);
            let c = skewed(r, -12.0);
            poly(v, &moved4(c, 12.0 * s, 12.0 * s), rgba(0x000000, 0.55));
            poly(v, &c, rgba(W_NIGHT, 0.94));
            poly(
                v,
                &skewed(Rect::new(r.x, r.y, 12.0 * s, r.h), -12.0),
                rgba(W_GOLD, 1.0),
            );
            // the glint, once across
            let g = ((age - 0.6) / 0.7).clamp(0.0, 1.0);
            if g > 0.0 && g < 1.0 {
                let gx = r.x + (r.w + 60.0 * s) * g - 60.0 * s;
                poly(
                    v,
                    &skewed(Rect::new(gx, r.y, 40.0 * s, r.h), -12.0),
                    rgba(0xffffff, 0.22 * (1.0 - (g - 0.5).abs() * 2.0)),
                );
            }
            // a four-pointed sparkle beside the words
            let sc = [r.x + 58.0 * s, r.y + r.h * 0.5];
            let sr = 22.0 * s * (1.0 + 0.08 * (time * 3.0).sin());
            for (dx, dy) in [(1.0, 0.0), (0.0, 1.0)] {
                let (ax, ay) = (dx * sr, dy * sr);
                let (bx, by) = (dy * sr * 0.28, dx * sr * 0.28);
                poly(
                    v,
                    &[
                        [sc[0] - ax, sc[1] - ay],
                        [sc[0] + bx, sc[1] + by],
                        [sc[0] + ax, sc[1] + ay],
                        [sc[0] - bx, sc[1] - by],
                    ],
                    rgba(W_GOLD, 1.0),
                );
            }
            text(
                font,
                v,
                Face::Display,
                label,
                r.x + 100.0 * s,
                r.y + 14.0 * s,
                small,
                rgba(W_GOLD, 1.0),
            );
            text(
                font,
                v,
                Face::Title,
                name,
                r.x + 100.0 * s,
                r.y + 18.0 * s + small * 1.2,
                big,
                rgba(W_INK, 1.0),
            );
        }
    }
    // The item: a gold panel on a darkened stage, rays turning behind; the panel grows in with the
    // web's overshoot.
    if let Some(item) = &view.item {
        let a = ease(item.age / 0.3);
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
        {
            let c = [width * 0.5, height * 0.5];
            let reach = 760.0 * s * a;
            let turn = time * std::f32::consts::TAU / 18.0;
            for k in 0..18 {
                let a0 = turn + (k as f32 * 20.0f32).to_radians();
                let a1 = a0 + 8.0f32.to_radians();
                let at = |ang: f32| [c[0] + ang.cos() * reach, c[1] + ang.sin() * reach];
                // Bright at the middle, fading toward the ends.
                quad_grad(
                    v,
                    [c, c, at(a1), at(a0)],
                    rgba(W_GOLD, 0.085 * a),
                    rgba(W_GOLD, 0.0),
                );
            }
        }
        let grow = 0.6 + 0.4 * bezier(0.2, 0.9, 0.25, 1.3, item.age / 0.45);
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
            // Its words fit across the screen whatever the language makes of them (turned, a
            // little narrower than the screen).
            let base = an(170.0) * s;
            let fit =
                (width * 0.84 / text_width(font, Face::Display, words, base).max(1.0)).min(1.0);
            let size = base * fit * pop;
            // Where the web slams it: over the middle, an item's panel and all (no burst over an
            // item's card: it would hide it).
            let c = [width * 0.5, height * 0.38];
            let colour = if *gold { W_GOLD } else { W_RED };
            faded(v, alpha, |v| {
                turned(v, c, angle, |v| {
                    comic_stamp(
                        font,
                        v,
                        Face::Display,
                        words,
                        c,
                        size,
                        colour,
                        view.item.is_none(),
                        (*age / 0.35).min(1.0),
                        time,
                    );
                });
            });
        }
    }
    // The flash: white, dying out as the web's does (0.5 s, `ease`).
    if view.flash < 0.5 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(0xffffff, 0.85 * (1.0 - ease(view.flash / 0.5))),
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
        band(v, -0.1, 0.36, rgba(lk.accent, 1.0));
        band(v, 0.33, 0.69, rgba(lk.night, 1.0));
        band(v, 0.66, 1.1, rgba(W_RED, 1.0));
    }
    // The outro: what the scene unlocked.
    if let Some((unlocks, age)) = &view.outro {
        let a = entrance(*age, 0.0, 0.45);
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(lk.night, 0.75 * a),
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
                    rgba(lk.accent_d, 1.0),
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

/// The colour band behind whoever speaks, in the place's colours (the Queen's side and the
/// other's): clear at its top, deepening down, finely striped; `out` slides it beyond its side
/// (0 in place, 1 = 200 px out) and `alpha` fades it.
pub(crate) fn speaker_slash(
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    right: bool,
    alpha: f32,
    out: f32,
    lk: &Look,
) {
    if alpha <= 0.01 {
        return;
    }
    let s = height / 1080.0;
    let w = 760.0 * s;
    let shift = out * 200.0 * s * if right { 1.0 } else { -1.0 };
    let x = shift
        + if right {
            width - 40.0 * s - w
        } else {
            40.0 * s
        };
    // As the web's: 1250 px tall, its foot 60 px below the screen's.
    let r = Rect::new(x, height + 60.0 * s - 1250.0 * s, w, 1250.0 * s);
    let c = skewed(r, -14.0);
    let ([mid, foot], [a_mid, a_foot]) = lk.bands[usize::from(right)];
    let at =
        |a: [f32; 2], b: [f32; 2], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    let (l30, r30) = (at(c[0], c[3], 0.3), at(c[1], c[2], 0.3));
    quad_grad(
        v,
        [c[0], c[1], r30, l30],
        rgba(mid, 0.0),
        rgba(mid, a_mid * alpha),
    );
    quad_grad(
        v,
        [l30, r30, c[2], c[3]],
        rgba(mid, a_mid * alpha),
        rgba(foot, a_foot * alpha),
    );
    // Its stripes, along its slant.
    let mut sx = 0.0;
    while sx < w {
        let (a, b) = (sx, (sx + 3.0 * s).min(w));
        poly(
            v,
            &[
                [c[0][0] + a, c[0][1]],
                [c[0][0] + b, c[0][1]],
                [c[3][0] + b, c[3][1]],
                [c[3][0] + a, c[3][1]],
            ],
            rgba(0xffffff, 0.08 * alpha),
        );
        sx += 22.0 * s;
    }
}

fn push_quad(v: &mut Vec<MapVertex>, c: [[f32; 2]; 4], color: [f32; 4]) {
    push_triangle(v, [c[0], c[1], c[2]], color);
    push_triangle(v, [c[0], c[2], c[3]], color);
    push_outline_fringe(v, &c, color);
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

/// The stage's pictures as textures, with their mips. A scene's pictures are decoded ahead on a
/// thread of their own as soon as it starts (a picture decoded as it comes on stutters its
/// entrance) and go up to the GPU a few a frame; one that is wanted before it arrives is waited
/// for, or decoded there and then. The least recently used go past `STAGE_KEEP`.
pub(crate) struct StagePictures {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    loaded: HashMap<String, (wgpu::BindGroup, PictureInfo)>,
    order: Vec<String>,
    failed: std::collections::HashSet<String>,
    root: std::path::PathBuf,
    /// Asked of the loader and not here yet; the loader's channels.
    asked: std::collections::HashSet<String>,
    to_loader: Option<std::sync::mpsc::Sender<String>>,
    from_loader: Option<std::sync::mpsc::Receiver<(String, Option<Decoded>)>>,
}

/// A picture decoded off the main thread: its size, the first row with something solid (where a
/// figure's head is) and its mip chain.
pub(crate) struct Decoded {
    size: [u32; 2],
    top: u32,
    levels: Vec<image::RgbaImage>,
}

/// A whole scene's cast, places and panels fit at once.
const STAGE_KEEP: usize = 40;
/// How many decoded pictures go up to the GPU in one frame.
const STAGE_UPLOADS: usize = 3;

fn decode(root: &std::path::Path, name: &str) -> Option<Decoded> {
    let file = format!("{name}.png");
    // A picture with words on it, in the player's language when there is one.
    let path = demon_queen_dungeon::i18n::localized_picture(std::path::Path::new(&file))
        .unwrap_or_else(|| root.join(&file));
    let rgba = velvet_avif::open_image(path.as_ref()).ok()?.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    // The first row with something solid (in steps of 4 columns): where a figure's head is.
    let top = (0..h)
        .find(|&y| (0..w).step_by(4).any(|x| rgba.get_pixel(x, y)[3] > 128))
        .unwrap_or(0);
    let count = 32 - w.max(h).leading_zeros();
    let mut levels = Vec::with_capacity(count as usize);
    let mut level = rgba;
    for mip in 0..count {
        let next = (mip + 1 < count).then(|| {
            image::imageops::resize(
                &level,
                (level.width() / 2).max(1),
                (level.height() / 2).max(1),
                image::imageops::FilterType::Triangle,
            )
        });
        levels.push(level);
        match next {
            Some(n) => level = n,
            None => break,
        }
    }
    Some(Decoded {
        size: [w, h],
        top,
        levels,
    })
}

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
            asked: Default::default(),
            to_loader: None,
            from_loader: None,
        }
    }

    /// Asks the loader for the scene's pictures it does not have yet (started the first time).
    pub(crate) fn prefetch(&mut self, names: &[String]) {
        for name in names {
            if self.loaded.contains_key(name)
                || self.asked.contains(name)
                || self.failed.contains(name)
            {
                continue;
            }
            if self.to_loader.is_none() {
                let (ask, asks) = std::sync::mpsc::channel::<String>();
                let (done, dones) = std::sync::mpsc::channel();
                let root = self.root.clone();
                let started = std::thread::Builder::new()
                    .name("stage-pictures".to_owned())
                    .spawn(move || {
                        for name in asks {
                            let picture = decode(&root, &name);
                            if done.send((name, picture)).is_err() {
                                break;
                            }
                        }
                    });
                if started.is_err() {
                    return;
                }
                self.to_loader = Some(ask);
                self.from_loader = Some(dones);
            }
            if let Some(tx) = &self.to_loader {
                if tx.send(name.clone()).is_ok() {
                    self.asked.insert(name.clone());
                }
            }
        }
    }

    /// Each frame: a few of the pictures the loader has finished go up to the GPU.
    pub(crate) fn pump(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut arrived = Vec::new();
        if let Some(rx) = &self.from_loader {
            while arrived.len() < STAGE_UPLOADS {
                match rx.try_recv() {
                    Ok(x) => arrived.push(x),
                    Err(_) => break,
                }
            }
        }
        for (name, picture) in arrived {
            self.arrive(device, queue, name, picture);
        }
    }

    /// A decoded picture goes up (or is marked missing).
    fn arrive(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        name: String,
        picture: Option<Decoded>,
    ) {
        self.asked.remove(&name);
        if self.loaded.contains_key(&name) {
            return;
        }
        let Some(picture) = picture else {
            eprintln!("[stage] picture missing: {name}");
            self.failed.insert(name);
            return;
        };
        let group = upload_levels(device, queue, &self.layout, &self.sampler, &picture.levels);
        let info = PictureInfo {
            size: [picture.size[0] as f32, picture.size[1] as f32],
            top: picture.top as f32,
        };
        self.loaded.insert(name.clone(), (group, info));
        self.order.push(name);
        while self.order.len() > STAGE_KEEP {
            let old = self.order.remove(0);
            self.loaded.remove(&old);
        }
    }

    /// The picture's size and head row: here already, or waited for from the loader, or decoded
    /// now.
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
        if self.asked.contains(name) {
            // On its way: what arrives before it goes up too.
            while self.asked.contains(name) {
                let next = match &self.from_loader {
                    Some(rx) => rx.recv_timeout(std::time::Duration::from_secs(3)),
                    None => break,
                };
                match next {
                    Ok((n, picture)) => self.arrive(device, queue, n, picture),
                    Err(_) => break,
                }
            }
            if let Some((_, info)) = self.loaded.get(name) {
                return Some(*info);
            }
        }
        let picture = decode(&self.root, name);
        self.arrive(device, queue, name.to_owned(), picture);
        self.loaded.get(name).map(|(_, i)| *i)
    }

    /// A picture made in the game (not read from a file), put up under `name` (replacing one of
    /// that name).
    pub(crate) fn insert_rgba(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        name: &str,
        width: u32,
        height: u32,
        rgba: &[u8],
    ) {
        let Some(img) = image::RgbaImage::from_raw(width, height, rgba.to_vec()) else {
            return;
        };
        let mut levels = vec![img];
        while levels
            .last()
            .is_some_and(|l| l.width() > 1 && l.height() > 1)
            && levels.len() < 6
        {
            let last = levels.last().expect("a level");
            let next = image::imageops::resize(
                last,
                (last.width() / 2).max(1),
                (last.height() / 2).max(1),
                image::imageops::FilterType::Triangle,
            );
            levels.push(next);
        }
        self.loaded.remove(name);
        self.order.retain(|n| n != name);
        self.failed.remove(name);
        self.asked.remove(name);
        let group = upload_levels(device, queue, &self.layout, &self.sampler, &levels);
        let info = PictureInfo {
            size: [width as f32, height as f32],
            top: 0.0,
        };
        self.loaded.insert(name.to_owned(), (group, info));
        self.order.push(name.to_owned());
    }

    pub(crate) fn group(&self, name: &str) -> Option<&wgpu::BindGroup> {
        self.loaded.get(name).map(|(g, _)| g)
    }

    /// Already loaded: its size and head row.
    pub(crate) fn info(&self, name: &str) -> Option<PictureInfo> {
        self.loaded.get(name).map(|(_, i)| *i)
    }
}

/// A picture with its mip chain made ahead, up to the GPU.
fn upload_levels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    levels: &[image::RgbaImage],
) -> wgpu::BindGroup {
    let (width, height) = levels.first().map_or((1, 1), |l| (l.width(), l.height()));
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("demon-queen-stage-picture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: levels.len().max(1) as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (mip, level) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: mip as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            level.as_raw(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(level.width() * 4),
                rows_per_image: Some(level.height()),
            },
            wgpu::Extent3d {
                width: level.width(),
                height: level.height(),
                depth_or_array_layers: 1,
            },
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("demon-queen-stage-picture-group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

fn ease_in_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
