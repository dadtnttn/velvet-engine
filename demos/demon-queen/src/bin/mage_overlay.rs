//! The hypnotist's stall in the wood (`explore/mage.rs`), in the manner of Persona's menus: the
//! world dims under slashes of violet with a halftone and speed lines; his big copper ring swings
//! slowly on its string behind him; he stands cut out at the left, his name thrown across his
//! chest. On the right, what can be done on tilted tiles with the red cursor at the lit one; his
//! word in a comic speech box. Talking is a short visual novel scene in a black box at the foot,
//! the Queen's face cutting in when she speaks, and his question with its two answers on tiles. The
//! shop is a counter of his wares, each drawn in code. When he springs the ring on her, it swings
//! at the screen in a burst of spirals and everything goes to black. What the keys and the pointer
//! light only changes light and colour, never a place or a size.
use std::f32::consts::{PI, TAU};

use super::persona::*;
use super::ui_paint::*;
use super::*;

type Uv = ([f32; 2], [f32; 2]);

/// What the overlay shows of his stall.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct MageView {
    /// 0 his stall, 1 talking, 2 the shop, 3 his ambush.
    pub page: u8,
    pub choice: u8,
    /// Since the page and the stall opened (ms): their entrances.
    pub page_ms: u32,
    pub open_ms: u32,
    /// The tiles of the page (or the answers to his question).
    pub options: Vec<MageOption>,
    /// What he says on his stall's page or the shop: the words and their age (ms).
    pub said: Option<(String, u32)>,
    /// The talk's line on show.
    pub line: Option<MageLine>,
    /// His question is up.
    pub asking: bool,
    pub question: String,
    pub gold: u32,
    /// The ware just bought and since when (ms).
    pub bought: Option<(u8, u32)>,
    /// His figure, and the Queen's face.
    pub mage: Option<PortraitRef>,
    pub queen: Option<PortraitRef>,
}

/// One tile: its words, the hint under them, its emblem (0 talk, 1 shop, 2 leave, 3 «no
/// demasiado», 4 «no», 10.. a ware), and for a ware its price, how many she has, whether she can
/// have no more, what it does.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct MageOption {
    pub label: String,
    pub hint: String,
    pub icon: u8,
    pub price: u32,
    pub owned: u32,
    pub locked: bool,
    pub effect: String,
}

/// A line of the talk: who (0 him, 1 the Queen, 2 the Queen thinking), the words, how many
/// letters are out, its age (ms).
#[derive(Clone, Debug, Default, Hash, PartialEq)]
pub struct MageLine {
    pub who: u8,
    pub text: String,
    pub shown: u16,
    pub age_ms: u32,
}

const VIOLET: u32 = 0x9a3ad8;
const DEEP: u32 = 0x2a0a4a;
const QUEEN: u32 = 0xff6ea0;
const COPPER: u32 = 0xd98a3a;
/// How far the tiles lean.
const TILE_ANGLE: f32 = -0.05;

/// The tiles of a page (0 his stall, 1 the answers, 2 the shop).
fn tiles(w: f32, h: f32, kind: u8, n: usize) -> Vec<Rect> {
    let s = h / 1080.0;
    let x = (w * 0.5 - 100.0 * s).max(820.0 * s);
    let tw = (w - x - 90.0 * s).min(900.0 * s);
    match kind {
        2 => {
            let row_h = 104.0 * s;
            (0..n)
                .map(|i| {
                    let last = i + 1 == n;
                    let y = 330.0 * s + i as f32 * (row_h + 16.0 * s);
                    if last {
                        Rect::new(x + 40.0 * s, y + 6.0 * s, tw * 0.42, 64.0 * s)
                    } else {
                        Rect::new(x + i as f32 * 18.0 * s, y, tw, row_h)
                    }
                })
                .collect()
        }
        1 => (0..n)
            .map(|i| {
                Rect::new(
                    x + 60.0 * s + i as f32 * 30.0 * s,
                    330.0 * s + i as f32 * 150.0 * s,
                    tw - 60.0 * s,
                    118.0 * s,
                )
            })
            .collect(),
        _ => (0..n)
            .map(|i| {
                Rect::new(
                    x + i as f32 * 30.0 * s,
                    360.0 * s + i as f32 * 148.0 * s,
                    tw,
                    120.0 * s,
                )
            })
            .collect(),
    }
}

fn inside_turned(r: Rect, angle: f32, x: f32, y: f32) -> bool {
    let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
    let (sn, cs) = (-angle).sin_cos();
    let (dx, dy) = (x - c[0], y - c[1]);
    r.contains(c[0] + dx * cs - dy * sn, c[1] + dx * sn + dy * cs)
}

/// The tile under the pointer (the screen's logical 1080p).
pub(crate) fn hit(w: f32, h: f32, kind: u8, n: usize, x: f32, y: f32) -> Option<usize> {
    tiles(w, h, kind, n)
        .iter()
        .position(|r| inside_turned(*r, TILE_ANGLE, x, y))
}

/// The whole stall.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &MageView,
    mage: Option<Uv>,
    queen: Option<Uv>,
    pointer: Option<(f32, f32)>,
    time: f32,
) {
    let s = h / 1080.0;
    let open = view.open_ms as f32 / 1000.0;
    let age = view.page_ms as f32 / 1000.0;
    backdrop(v, w, h, open, time, s);
    big_ring(v, w, h, view, time, s);
    let queen_speaks =
        view.page == 1 && view.line.as_ref().is_some_and(|l| l.who != 0) && !view.asking;
    figure(font, v, w, h, view, mage, open, queen_speaks, time, s);
    match view.page {
        1 => draw_talk(font, v, w, h, view, queen, pointer, age, time, s),
        2 => draw_shop(font, v, w, h, view, pointer, age, time, s),
        3 => draw_ambush(font, v, w, h, age, time, s),
        _ => draw_main(font, v, w, h, view, pointer, age, time, s),
    }
    if view.page != 3 {
        purse(
            font,
            v,
            Rect::new(w - 300.0 * s, 40.0 * s, 210.0 * s, 60.0 * s),
            view.gold,
            s,
        );
    }
}

/// Slashes of violet sweeping in, a halftone and speed lines over the dimmed world.
fn backdrop(v: &mut Vec<MapVertex>, w: f32, h: f32, open: f32, time: f32, s: f32) {
    push_rect(v, Rect::new(0.0, 0.0, w, h), rgba(0x05020a, 0.72));
    let sweep = entrance(open, 0.0, 0.36);
    let angle = -0.24;
    let center = [w * 0.55, h * 0.5];
    shifted(v, [-(1.0 - sweep) * w * 1.2, 0.0], |v| {
        slash(
            v,
            center,
            w * 1.9,
            620.0 * s,
            angle,
            rgba(mix(DEEP, VIOLET, 0.42), 0.95),
            rgba(0x10031c, 0.95),
        );
        slash(
            v,
            [center[0], center[1] + 336.0 * s],
            w * 1.9,
            30.0 * s,
            angle,
            rgba(0x0b0508, 0.96),
            rgba(0x0b0508, 0.96),
        );
        slash(
            v,
            [center[0], center[1] - 322.0 * s],
            w * 1.9,
            6.0 * s,
            angle,
            rgba(GOLD_HI, 0.85),
            rgba(GOLD, 0.85),
        );
        slash(
            v,
            [center[0], center[1] - 362.0 * s],
            w * 1.9,
            56.0 * s,
            angle,
            rgba(0xd65cff, 0.5),
            rgba(DEEP, 0.5),
        );
    });
    faded(v, sweep, |v| {
        halftone(
            v,
            Rect::new(w * 0.52, 0.0, w * 0.48, h),
            26.0 * s,
            12.0 * s,
            [0.9, -0.7],
            rgba(0xd65cff, 0.15),
        );
        speed_lines(
            v,
            Rect::new(0.0, h * 0.16, w, h * 0.66),
            angle,
            22,
            time,
            rgba(0xffffff, 0.07),
        );
    });
}

/// His great copper ring, swinging slowly on its string behind him, glowing.
fn big_ring(v: &mut Vec<MapVertex>, w: f32, h: f32, view: &MageView, time: f32, s: f32) {
    let _ = w;
    if view.page == 3 {
        return;
    }
    let pivot = [420.0 * s, -20.0 * s];
    let length = 330.0 * s;
    let a = (time * 1.15).sin() * 0.42;
    let c = [pivot[0] + a.sin() * length, pivot[1] + a.cos() * length];
    let r = 78.0 * s;
    let face = 0.35 + 0.65 * (time * 0.9).cos().abs();
    wire(
        v,
        pivot,
        [c[0], c[1] - r * face],
        3.0 * s,
        rgba(0xe8d0a0, 0.6),
        rgba(0xe8d0a0, 0.8),
        0.0,
        true,
    );
    halo(v, c, r * 1.1, 46.0 * s, rgba(0xffa050, 0.28));
    ring_shape(v, c, r, r * face, 14.0 * s, 1.0);
    let g = time * 2.3;
    sparkle(
        v,
        [c[0] + g.cos() * r, c[1] + g.sin() * r * face],
        14.0 * s,
        rgba(0xfff0c8, 0.9),
        0.8,
    );
    let _ = h;
}

/// A ring seen at a slant: an ellipse of copper with a dark edge.
fn ring_shape(v: &mut Vec<MapVertex>, c: [f32; 2], rx: f32, ry: f32, width: f32, alpha: f32) {
    let n = 40;
    let p = |k: usize| {
        let t = k as f32 / n as f32 * TAU;
        [c[0] + t.cos() * rx, c[1] + t.sin() * ry.max(rx * 0.06)]
    };
    for k in 0..n {
        wire(
            v,
            p(k),
            p(k + 1),
            width * 1.35,
            rgba(0x4a2408, 0.85 * alpha),
            rgba(0x4a2408, 0.85 * alpha),
            0.0,
            false,
        );
    }
    for k in 0..n {
        wire(
            v,
            p(k),
            p(k + 1),
            width,
            rgba(COPPER, alpha),
            rgba(COPPER, alpha),
            0.0,
            true,
        );
    }
}

/// His figure at the left, his name across it.
#[allow(clippy::too_many_arguments)]
fn figure(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &MageView,
    uv: Option<Uv>,
    open: f32,
    dim: bool,
    time: f32,
    s: f32,
) {
    let _ = w;
    let a = entrance(open, 0.06, 0.4);
    let side = 720.0 * s;
    let mut r = Rect::new(30.0 * s, h - side + 10.0 * s, side, side);
    // Ambush: he lunges forward.
    if view.page == 3 {
        let k = entrance(view.page_ms as f32 / 1000.0, 0.0, 0.4);
        let grow_by = 70.0 * s * k;
        r = Rect::new(
            r.x + 60.0 * s * k - grow_by * 0.5,
            r.y - grow_by,
            r.w + grow_by,
            r.h + grow_by,
        );
    }
    let breathe = (time * 1.4).sin() * 3.0 * s;
    faded(v, a, |v| {
        shifted(v, [-(1.0 - a) * 300.0 * s, breathe], |v| {
            // His shadow on the slash.
            soft(
                v,
                Rect::new(r.x + 40.0 * s, r.y + 60.0 * s, r.w * 0.8, r.h),
                60.0 * s,
                50.0 * s,
                rgba(0x05020a, 0.55),
                false,
            );
            if let Some(uv) = uv {
                let t = if dim { 0.55 } else { 1.0 };
                cutout(v, r, uv, [t, t, t * 1.05, 1.0]);
            }
        });
    });
    if view.page == 3 {
        return;
    }
    // His name thrown across his chest.
    let na = entrance(open, 0.25, 0.3);
    let base = [50.0 * s, h - 210.0 * s];
    faded(v, na * if dim { 0.6 } else { 1.0 }, |v| {
        shifted(v, [(1.0 - na) * -80.0 * s, 0.0], |v| {
            turned(v, base, -0.09, |v| {
                shout(
                    font,
                    v,
                    Face::Title,
                    "El Mago",
                    base[0],
                    base[1],
                    112.0 * s,
                    0xffffff,
                    0x6a1a9a,
                    1.0,
                );
                text_outline(
                    font,
                    v,
                    Face::Italic,
                    "Hipnotista ambulante · su claro del bosque",
                    base[0] + 14.0 * s,
                    base[1] + 124.0 * s,
                    30.0 * s,
                    rgba(0xe6c8ff, 1.0),
                    rgba(0x0b0508, 0.9),
                );
            });
        });
    });
}

/// His stall's page: what can be done, his word.
#[allow(clippy::too_many_arguments)]
fn draw_main(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &MageView,
    pointer: Option<(f32, f32)>,
    age: f32,
    time: f32,
    s: f32,
) {
    let rects = tiles(w, h, 0, view.options.len());
    heading(
        font,
        v,
        rects.first().map_or(w * 0.5, |r| r.x),
        236.0 * s,
        "¿Qué haces con el mago?",
        age,
        s,
    );
    for (i, (r, o)) in rects.iter().zip(&view.options).enumerate() {
        let a = entrance(age, 0.14 + 0.07 * i as f32, 0.3);
        let lit = i == view.choice as usize;
        let hover = pointer.is_some_and(|(x, y)| inside_turned(*r, TILE_ANGLE, x, y));
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 280.0 * s, 0.0], |v| {
                turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], TILE_ANGLE, |v| {
                    tile(font, v, *r, o, lit, hover, time, s);
                });
            });
        });
        if lit && a > 0.95 {
            tile_cursor(v, *r, time, s);
        }
    }
    if let Some((text, ms)) = &view.said {
        let k = entrance(*ms as f32 / 1000.0, 0.0, 0.25);
        speech_box(
            font,
            v,
            [
                rects.first().map_or(860.0 * s, |r| r.x + 40.0 * s),
                200.0 * s,
            ],
            860.0 * s,
            "Mago",
            text,
            0xb05cff,
            -0.02,
            k,
            s,
        );
    }
    hints(
        font,
        v,
        h,
        &[("W S", "elegir"), ("Enter", "aceptar"), ("Esc", "irse")],
        s,
    );
}

/// A big title over the tiles.
fn heading(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    x: f32,
    y: f32,
    words: &str,
    age: f32,
    s: f32,
) {
    let a = entrance(age, 0.08, 0.3);
    faded(v, a, |v| {
        turned(v, [x, y], -0.06, |v| {
            let size = 48.0 * s;
            shout(
                font,
                v,
                Face::Title,
                words,
                x,
                y,
                size,
                0xffffff,
                0x6a1a9a,
                1.0,
            );
            let tw = text_width(font, Face::Title, words, size);
            wire(
                v,
                [x, y + size + 12.0 * s],
                [x + tw + 60.0 * s, y + size + 12.0 * s],
                6.0 * s,
                rgba(0xffffff, 0.95),
                rgba(0xffffff, 0.0),
                0.0,
                false,
            );
        });
    });
}

/// One tile: black-edged velvet, a stripe of violet down its left, its emblem, words and hint.
#[allow(clippy::too_many_arguments)]
fn tile(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    r: Rect,
    o: &MageOption,
    lit: bool,
    hover: bool,
    time: f32,
    s: f32,
) {
    let glow = if o.icon == 3 { 0xff6ef0 } else { VIOLET };
    let (top, bottom, rim) = if o.locked {
        (
            if lit { 0x2c1c32 } else { 0x1c1420 },
            0x0e0a12,
            if lit { 0xb8a2c4 } else { 0x6a5a74 },
        )
    } else if lit {
        (
            mix(0x2a0a3a, glow, 0.45),
            mix(0x10040e, glow, 0.16),
            GOLD_HI,
        )
    } else if hover {
        (0x2c1236, 0x12060e, GOLD)
    } else {
        (0x1e0c24, 0x0b0508, GOLD_DEEP)
    };
    soft(
        v,
        Rect::new(r.x + 12.0 * s, r.y + 14.0 * s, r.w, r.h),
        10.0 * s,
        14.0 * s,
        rgba(SHADOW, 0.7),
        false,
    );
    if lit {
        soft(
            v,
            r,
            10.0 * s,
            24.0 * s,
            rgba(glow, 0.42 + 0.12 * wave(time, 0.8)),
            true,
        );
    }
    panel(
        v,
        grow(r, 4.0 * s),
        11.0 * s,
        Panel::fill(0x0b0508, 0x0b0508, 1.0),
    );
    let mut style = Panel::fill(top, bottom, 0.98)
        .rim(rim, 1.0, 2.4 * s)
        .hairline(6.0 * s, if lit { 0.5 } else { 0.22 })
        .bevel(0.6);
    if lit && !o.locked {
        style = style.sheen(time * 0.25);
    }
    panel(v, r, 8.0 * s, style);
    panel(
        v,
        Rect::new(r.x, r.y, 12.0 * s, r.h),
        3.0 * s,
        Panel::fill(
            mix(glow, 0xffffff, 0.25),
            glow,
            if o.locked { 0.4 } else { 1.0 },
        ),
    );
    let c = [r.x + 70.0 * s, r.y + r.h * 0.5];
    emblem(v, o.icon, c, (r.h * 0.34).min(40.0 * s), lit, time, s);
    let ink = if o.locked { 0xb0a0ba } else { INK };
    let small = r.h < 80.0 * s;
    let size = if small { 30.0 * s } else { 38.0 * s };
    let ty = if small {
        r.y + (r.h - size) * 0.5 - 3.0 * s
    } else {
        r.y + 16.0 * s
    };
    text_outline(
        font,
        v,
        Face::Title,
        &o.label,
        r.x + 128.0 * s,
        ty,
        size,
        rgba(ink, 1.0),
        rgba(0x0b0508, 0.85),
    );
    if !small {
        let hint_room = r.w - 150.0 * s - if o.price > 0 { 170.0 * s } else { 0.0 };
        let mut hs = 24.0 * s;
        while hs > 17.0 * s && text_width(font, Face::Italic, &o.hint, hs) > hint_room {
            hs -= 0.5 * s;
        }
        text_shadow(
            font,
            v,
            Face::Italic,
            &o.hint,
            r.x + 130.0 * s,
            r.y + r.h - hs - 18.0 * s,
            hs,
            rgba(CREAM, 0.95),
        );
    }
    if o.price > 0 {
        let px = r.x + r.w - 150.0 * s;
        coin(v, [px, r.y + 34.0 * s], 15.0 * s, 1.0);
        text_shadow(
            font,
            v,
            Face::Ui,
            &format!("{}", o.price),
            px + 24.0 * s,
            r.y + 19.0 * s,
            30.0 * s,
            rgba(GOLD_HI, 1.0),
        );
        let owned = if o.locked {
            "ya es tuyo".to_owned()
        } else if o.owned > 0 {
            format!("tienes {}", o.owned)
        } else {
            String::new()
        };
        if !owned.is_empty() {
            text_shadow(
                font,
                v,
                Face::Italic,
                &owned,
                px - 10.0 * s,
                r.y + r.h - 40.0 * s,
                21.0 * s,
                rgba(if o.locked { 0xd8b0e8 } else { 0xbfe8d0 }, 1.0),
            );
        }
    }
}

/// The red cursor at the lit tile's left end.
fn tile_cursor(v: &mut Vec<MapVertex>, r: Rect, time: f32, s: f32) {
    let (sn, cs) = TILE_ANGLE.sin_cos();
    let half = r.w * 0.5;
    let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
    cursor(
        v,
        [c[0] - half * cs - 16.0 * s, c[1] - half * sn],
        34.0 * s,
        time,
    );
}

/// A tile's emblem, drawn in code.
fn emblem(v: &mut Vec<MapVertex>, icon: u8, c: [f32; 2], r: f32, lit: bool, time: f32, s: f32) {
    let glow = if lit { 1.0 } else { 0.6 };
    panel(
        v,
        Rect::new(c[0] - r, c[1] - r, r * 2.0, r * 2.0),
        r,
        Panel::fill(0x2a1038, 0x0e0414, 1.0)
            .rim(if lit { GOLD_HI } else { GOLD_DEEP }, 1.0, 2.4 * s)
            .bevel(1.0),
    );
    match icon {
        0 => {
            // A speech balloon.
            let b = Rect::new(c[0] - r * 0.55, c[1] - r * 0.42, r * 1.1, r * 0.7);
            panel(v, b, r * 0.3, Panel::fill(0xfff4ff, 0xe0c8f0, glow));
            push_triangle(
                v,
                [
                    [b.x + b.w * 0.25, b.y + b.h - 2.0],
                    [b.x + b.w * 0.5, b.y + b.h - 2.0],
                    [b.x + b.w * 0.18, b.y + b.h + r * 0.3],
                ],
                rgba(0xe0c8f0, glow),
            );
            for k in 0..3 {
                disc(
                    v,
                    [b.x + b.w * (0.3 + 0.2 * k as f32), b.y + b.h * 0.5],
                    r * 0.07,
                    rgba(0x4a1a6a, 1.0),
                );
            }
        }
        1 => {
            // A coin purse.
            let bag = Rect::new(c[0] - r * 0.45, c[1] - r * 0.25, r * 0.9, r * 0.75);
            panel(
                v,
                bag,
                r * 0.35,
                Panel::fill(0xb8864a, 0x6a4420, glow).bevel(0.6),
            );
            wire(
                v,
                [c[0] - r * 0.25, c[1] - r * 0.28],
                [c[0] + r * 0.25, c[1] - r * 0.28],
                3.0 * s,
                rgba(0x3a2410, 1.0),
                rgba(0x3a2410, 1.0),
                0.0,
                false,
            );
            coin(v, [c[0] + r * 0.32, c[1] - r * 0.45], r * 0.22, glow);
        }
        2 => {
            // Away: an arrow.
            let a = rgba(CREAM, glow);
            wire(
                v,
                [c[0] + r * 0.45, c[1]],
                [c[0] - r * 0.4, c[1]],
                4.0 * s,
                a,
                a,
                0.0,
                false,
            );
            wire(
                v,
                [c[0] - r * 0.45, c[1]],
                [c[0] - r * 0.1, c[1] - r * 0.32],
                4.0 * s,
                a,
                a,
                0.0,
                false,
            );
            wire(
                v,
                [c[0] - r * 0.45, c[1]],
                [c[0] - r * 0.1, c[1] + r * 0.32],
                4.0 * s,
                a,
                a,
                0.0,
                false,
            );
        }
        3 => {
            // «No demasiado»: a little spiral, turning.
            let mut prev = c;
            for i in 1..=36 {
                let u = i as f32 / 36.0;
                let ang = u * 2.6 * TAU + time * 3.0;
                let p = [
                    c[0] + ang.cos() * u * r * 0.62,
                    c[1] + ang.sin() * u * r * 0.62,
                ];
                wire(
                    v,
                    prev,
                    p,
                    2.6 * s,
                    rgba(0xff8af0, glow),
                    rgba(0xff8af0, glow),
                    0.0,
                    false,
                );
                prev = p;
            }
        }
        4 => {
            // «No»: a cross.
            let a = rgba(0x8affd0, glow);
            wire(
                v,
                [c[0] - r * 0.4, c[1] - r * 0.4],
                [c[0] + r * 0.4, c[1] + r * 0.4],
                5.0 * s,
                a,
                a,
                0.0,
                false,
            );
            wire(
                v,
                [c[0] + r * 0.4, c[1] - r * 0.4],
                [c[0] - r * 0.4, c[1] + r * 0.4],
                5.0 * s,
                a,
                a,
                0.0,
                false,
            );
        }
        10 => salts(v, c, r, glow, time, s),
        11 => amulet(v, c, r, glow, s),
        _ => pendulum(v, c, r, glow, time, s),
    }
}

/// Smelling salts: a little bottle of green with a cork, fumes curling up.
fn salts(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, glow: f32, time: f32, s: f32) {
    let body = Rect::new(c[0] - r * 0.32, c[1] - r * 0.1, r * 0.64, r * 0.62);
    panel(
        v,
        body,
        r * 0.18,
        Panel::fill(0x9affd0, 0x2a8a6a, glow)
            .rim(0xe8fff4, glow, 1.6 * s)
            .bevel(0.5),
    );
    panel(
        v,
        Rect::new(c[0] - r * 0.14, c[1] - r * 0.3, r * 0.28, r * 0.22),
        r * 0.05,
        Panel::fill(0xd8b07a, 0x8a6a3a, glow),
    );
    for k in 0..3 {
        let phase = (time * 0.6 + k as f32 / 3.0).fract();
        let x = c[0] + (phase * 6.0 + k as f32).sin() * r * 0.18;
        let y = c[1] - r * 0.36 - phase * r * 0.5;
        arc(
            v,
            [x, y],
            (2.0 + 2.0 * phase) * s,
            1.4 * s,
            0.0,
            1.0,
            rgba(0xd8fff0, 0.7 * (1.0 - phase) * glow),
            [0.0; 4],
        );
    }
}

/// The amulet: a brass disc with a closed (and squinting) eye, a loop for its cord.
fn amulet(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, glow: f32, s: f32) {
    arc(
        v,
        [c[0], c[1] - r * 0.58],
        r * 0.14,
        2.4 * s,
        0.0,
        1.0,
        rgba(0xd8b06a, glow),
        [0.0; 4],
    );
    panel(
        v,
        Rect::new(c[0] - r * 0.48, c[1] - r * 0.44, r * 0.96, r * 0.96),
        r * 0.48,
        Panel::fill(0xffe0a0, 0xa07a30, glow)
            .rim(0x6a4a18, glow, 2.0 * s)
            .bevel(0.8),
    );
    // The closed eye: a lid's curve and its lashes.
    let n = 10;
    for k in 0..n {
        let t0 = k as f32 / n as f32;
        let t1 = (k + 1) as f32 / n as f32;
        let p = |t: f32| {
            [
                c[0] - r * 0.28 + r * 0.56 * t,
                c[1] + (t * PI).sin() * r * 0.12,
            ]
        };
        wire(
            v,
            p(t0),
            p(t1),
            2.4 * s,
            rgba(0x4a2a08, 1.0),
            rgba(0x4a2a08, 1.0),
            0.0,
            false,
        );
    }
    for k in 0..4 {
        let t = 0.2 + 0.2 * k as f32;
        let x = c[0] - r * 0.28 + r * 0.56 * t;
        let y = c[1] + (t * PI).sin() * r * 0.12;
        wire(
            v,
            [x, y],
            [x + (t - 0.5) * r * 0.2, y + r * 0.13],
            1.6 * s,
            rgba(0x4a2a08, 1.0),
            rgba(0x4a2a08, 1.0),
            0.0,
            false,
        );
    }
}

/// The practice pendulum: a hex nut on a cord, swinging.
fn pendulum(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, glow: f32, time: f32, s: f32) {
    let a = (time * 2.2).sin() * 0.35;
    let top = [c[0], c[1] - r * 0.62];
    let nut = [top[0] + a.sin() * r * 0.82, top[1] + a.cos() * r * 0.82];
    wire(
        v,
        top,
        nut,
        1.8 * s,
        rgba(0xe8d8c0, glow),
        rgba(0xe8d8c0, glow),
        0.0,
        false,
    );
    let hex = |k: usize| {
        let t = k as f32 / 6.0 * TAU + a;
        [nut[0] + t.cos() * r * 0.26, nut[1] + t.sin() * r * 0.26]
    };
    for k in 0..6 {
        push_triangle(
            v,
            [nut, hex(k), hex(k + 1)],
            rgba(mix(0x9a9aa8, 0x5a5a68, (k % 2) as f32 * 0.5), glow),
        );
    }
    disc(v, nut, r * 0.1, rgba(0x1a1420, 1.0));
}

/// A gold coin.
fn coin(v: &mut Vec<MapVertex>, c: [f32; 2], r: f32, alpha: f32) {
    panel(
        v,
        Rect::new(c[0] - r, c[1] - r, r * 2.0, r * 2.0),
        r,
        Panel::fill(0xffe9a8, 0xb07a2c, alpha)
            .rim(GOLD_DEEP, alpha, (r * 0.14).max(1.0))
            .bevel(0.6),
    );
    arc(
        v,
        c,
        r * 0.62,
        (r * 0.1).max(1.0),
        0.0,
        1.0,
        rgba(0x9a6a28, 0.8 * alpha),
        [0.0; 4],
    );
}

/// Her gold, top right.
fn purse(font: &FontAtlas, v: &mut Vec<MapVertex>, r: Rect, gold: u32, s: f32) {
    soft(
        v,
        Rect::new(r.x, r.y + 8.0 * s, r.w, r.h),
        r.h * 0.5,
        14.0 * s,
        rgba(SHADOW, 0.7),
        false,
    );
    panel(
        v,
        r,
        r.h * 0.5,
        Panel::fill(0x2c1640, 0x10081a, 0.97)
            .rim(GOLD, 1.0, 2.2 * s)
            .bevel(1.0),
    );
    coin(v, [r.x + 34.0 * s, r.y + r.h * 0.5], 17.0 * s, 1.0);
    text_shadow(
        font,
        v,
        Face::Ui,
        &format!("{gold} oro"),
        r.x + 62.0 * s,
        r.y + (r.h - 30.0 * s) * 0.5,
        30.0 * s,
        rgba(GOLD_HI, 1.0),
    );
}

/// Keys at the foot.
fn hints(font: &FontAtlas, v: &mut Vec<MapVertex>, h: f32, keys: &[(&str, &str)], s: f32) {
    let mut x = 830.0 * s;
    let y = h - 78.0 * s;
    for (key, what) in keys {
        let cap_w = (text_width(font, Face::Title, key, 18.0 * s) + 26.0 * s).max(42.0 * s);
        keycap(
            font,
            v,
            Rect::new(x, y, cap_w, 40.0 * s),
            key,
            18.0 * s,
            false,
            false,
            VIOLET,
            1.0,
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            what,
            x + cap_w + 12.0 * s,
            y + 8.0 * s,
            21.0 * s,
            rgba(CREAM, 0.95),
        );
        x += cap_w + 12.0 * s + text_width(font, Face::Italic, what, 21.0 * s) + 36.0 * s;
    }
}

/// The talk: a black box at the foot with the line typing out, the Queen's face cutting in when
/// she speaks, and at the end his question with its two answers.
#[allow(clippy::too_many_arguments)]
fn draw_talk(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &MageView,
    queen: Option<Uv>,
    pointer: Option<(f32, f32)>,
    age: f32,
    time: f32,
    s: f32,
) {
    let Some(line) = &view.line else {
        return;
    };
    let her = line.who != 0;
    // Her face, cutting in from the right while she speaks.
    if her && !view.asking {
        let k = entrance(line.age_ms as f32 / 1000.0, 0.0, 0.28);
        let r = Rect::new(w - 560.0 * s, h - 760.0 * s, 380.0 * s, 380.0 * s);
        faded(v, k, |v| {
            shifted(v, [(1.0 - k) * 220.0 * s, 0.0], |v| {
                let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
                slash(
                    v,
                    c,
                    760.0 * s,
                    300.0 * s,
                    -0.3,
                    rgba(QUEEN, 0.85),
                    rgba(0x6a0a3a, 0.85),
                );
                turned(v, c, 0.05, |v| {
                    panel(
                        v,
                        grow(r, 8.0 * s),
                        18.0 * s,
                        Panel::fill(0x0b0508, 0x0b0508, 1.0),
                    );
                    panel(
                        v,
                        grow(r, 3.0 * s),
                        15.0 * s,
                        Panel::fill(0x2a0a1a, 0x0b0508, 1.0).rim(0xffd0e4, 1.0, 2.6 * s),
                    );
                    if let Some(uv) = queen {
                        face(v, r, 14.0 * s, uv, 1.0);
                    }
                });
            });
        });
    }
    // The box.
    let a = entrance(age, 0.1, 0.3);
    let r = Rect::new(
        560.0 * s,
        h - 340.0 * s,
        (w - 640.0 * s).min(1300.0 * s),
        230.0 * s,
    );
    let accent = if her { QUEEN } else { 0xd65cff };
    let name = match line.who {
        0 => "MAGO",
        _ => "REINA",
    };
    faded(v, a, |v| {
        shifted(v, [0.0, (1.0 - a) * 60.0 * s], |v| {
            turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], -0.012, |v| {
                soft(
                    v,
                    Rect::new(r.x + 14.0 * s, r.y + 18.0 * s, r.w, r.h),
                    14.0 * s,
                    18.0 * s,
                    rgba(SHADOW, 0.8),
                    false,
                );
                panel(
                    v,
                    grow(r, 6.0 * s),
                    14.0 * s,
                    Panel::fill(0xffffff, 0xe8dcf0, 1.0),
                );
                panel(v, r, 10.0 * s, Panel::fill(0x140818, 0x05020a, 0.98));
                panel(
                    v,
                    Rect::new(r.x, r.y, r.w, 8.0 * s),
                    4.0 * s,
                    Panel::fill(accent, mix(accent, 0x000000, 0.4), 1.0),
                );
                // The name on a tab of their colour.
                let ns = 30.0 * s;
                let tab = Rect::new(
                    r.x + 34.0 * s,
                    r.y - 26.0 * s,
                    text_width(font, Face::Title, name, ns) + 44.0 * s,
                    52.0 * s,
                );
                panel(
                    v,
                    grow(tab, 4.0 * s),
                    8.0 * s,
                    Panel::fill(0x0b0508, 0x0b0508, 1.0),
                );
                panel(
                    v,
                    tab,
                    6.0 * s,
                    Panel::fill(mix(accent, 0xffffff, 0.15), mix(accent, 0x000000, 0.3), 1.0),
                );
                text(
                    font,
                    v,
                    Face::Title,
                    name,
                    tab.x + 22.0 * s,
                    tab.y + 9.0 * s,
                    ns,
                    rgba(0xffffff, 1.0),
                );
                let size = 34.0 * s;
                let face_kind = if line.who == 2 {
                    Face::Italic
                } else {
                    Face::Body
                };
                let ink = if line.who == 2 { LILAC } else { 0xffffff };
                let lines = wrap(font, face_kind, &line.text, size, r.w - 100.0 * s);
                let mut left = line.shown as usize;
                for (i, l) in lines.iter().enumerate() {
                    let count = l.chars().count();
                    let part: String = l.chars().take(left).collect();
                    left = left.saturating_sub(count + 1);
                    text_shadow(
                        font,
                        v,
                        face_kind,
                        &part,
                        r.x + 50.0 * s,
                        r.y + 46.0 * s + i as f32 * size * 1.38,
                        size,
                        rgba(ink, 1.0),
                    );
                }
                let done = line.shown as usize >= line.text.chars().count();
                if done && !view.asking {
                    let bob = (time * 4.0).sin().abs() * 6.0 * s;
                    chevron(
                        v,
                        [r.x + r.w - 46.0 * s, r.y + r.h - 34.0 * s + bob],
                        14.0 * s,
                        rgba(accent, 1.0),
                    );
                }
            });
        });
    });
    // His question's answers.
    if view.asking {
        let rects = tiles(w, h, 1, view.options.len());
        for (i, (rr, o)) in rects.iter().zip(&view.options).enumerate() {
            let k = entrance(line.age_ms as f32 / 1000.0, 0.3 + 0.08 * i as f32, 0.3);
            let lit = i == view.choice as usize;
            let hover = pointer.is_some_and(|(x, y)| inside_turned(*rr, TILE_ANGLE, x, y));
            faded(v, k, |v| {
                shifted(v, [(1.0 - k) * 260.0 * s, 0.0], |v| {
                    turned(v, [rr.x + rr.w * 0.5, rr.y + rr.h * 0.5], TILE_ANGLE, |v| {
                        tile(font, v, *rr, o, lit, hover, time, s);
                    });
                });
            });
            if lit && k > 0.95 {
                tile_cursor(v, *rr, time, s);
            }
        }
        hints(
            font,
            v,
            h,
            &[
                ("W S", "elegir"),
                ("1 2", "contestar"),
                ("Enter", "aceptar"),
            ],
            s,
        );
    } else {
        hints(font, v, h, &[("F", "seguir"), ("Esc", "dejarlo")], s);
    }
}

/// The shop: his wares on a counter, the lit one's effect under them.
#[allow(clippy::too_many_arguments)]
fn draw_shop(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    view: &MageView,
    pointer: Option<(f32, f32)>,
    age: f32,
    time: f32,
    s: f32,
) {
    let rects = tiles(w, h, 2, view.options.len());
    heading(
        font,
        v,
        rects.first().map_or(w * 0.5, |r| r.x),
        236.0 * s,
        "El puesto del mago",
        age,
        s,
    );
    for (i, (r, o)) in rects.iter().zip(&view.options).enumerate() {
        let a = entrance(age, 0.14 + 0.07 * i as f32, 0.3);
        let lit = i == view.choice as usize;
        let hover = pointer.is_some_and(|(x, y)| inside_turned(*r, TILE_ANGLE, x, y));
        let flash = view
            .bought
            .filter(|(b, _)| *b as usize == i)
            .map_or(0.0, |(_, ms)| (1.0 - ms as f32 / 700.0).max(0.0));
        faded(v, a, |v| {
            shifted(v, [(1.0 - a) * 280.0 * s, 0.0], |v| {
                turned(v, [r.x + r.w * 0.5, r.y + r.h * 0.5], TILE_ANGLE, |v| {
                    tile(font, v, *r, o, lit, hover, time, s);
                    if flash > 0.0 {
                        soft(v, *r, 10.0 * s, 30.0 * s, rgba(GOLD_HI, 0.7 * flash), true);
                    }
                });
            });
        });
        if lit && a > 0.95 {
            tile_cursor(v, *r, time, s);
        }
    }
    // The lit ware's effect.
    if let (Some(o), Some(last)) = (view.options.get(view.choice as usize), rects.last()) {
        if !o.effect.is_empty() {
            let r = Rect::new(
                last.x + last.w + 40.0 * s,
                last.y - 6.0 * s,
                (w - last.x - last.w - 120.0 * s).max(300.0 * s),
                76.0 * s,
            );
            panel(
                v,
                r,
                12.0 * s,
                Panel::fill(0x2a1038, 0x10040e, 0.95).rim(GOLD, 0.9, 1.6 * s),
            );
            let size = 22.0 * s;
            for (i, l) in wrap(font, Face::Italic, &o.effect, size, r.w - 30.0 * s)
                .iter()
                .take(2)
                .enumerate()
            {
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    l,
                    r.x + 16.0 * s,
                    r.y + 10.0 * s + i as f32 * size * 1.3,
                    size,
                    rgba(0xe8f8f0, 1.0),
                );
            }
        }
    }
    if let Some((text, ms)) = &view.said {
        let k = entrance(*ms as f32 / 1000.0, 0.0, 0.25);
        speech_box(
            font,
            v,
            [
                rects.first().map_or(860.0 * s, |r| r.x + 40.0 * s),
                200.0 * s,
            ],
            860.0 * s,
            "Mago",
            text,
            0xb05cff,
            -0.02,
            k,
            s,
        );
    }
    hints(
        font,
        v,
        h,
        &[("W S", "elegir"), ("Enter", "comprar"), ("Esc", "volver")],
        s,
    );
}

/// His ambush: the ring swings at the screen, spirals burst, «¡A TRAICIÓN!», black.
fn draw_ambush(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    w: f32,
    h: f32,
    age: f32,
    time: f32,
    s: f32,
) {
    let c = [w * 0.55, h * 0.48];
    let k = (age / 1.2).clamp(0.0, 1.0);
    // A flash of violet, then the spiral turning ever faster.
    if age < 0.25 {
        push_rect(
            v,
            Rect::new(0.0, 0.0, w, h),
            rgba(0xffe0ff, 0.6 * (1.0 - age / 0.25)),
        );
    }
    let n = 220;
    let mut prev: Option<[f32; 2]> = None;
    for i in 0..=n {
        let u = i as f32 / n as f32;
        let ang = u * 7.0 * TAU - time * (3.0 + 6.0 * k);
        let rr = u * (300.0 + 900.0 * k) * s;
        let p = [c[0] + ang.cos() * rr, c[1] + ang.sin() * rr];
        if let Some(q) = prev {
            wire(
                v,
                q,
                p,
                (3.0 + 5.0 * u) * s,
                rgba(0xd65cff, 0.5 * (1.0 - u * 0.5)),
                rgba(0xff8af0, 0.5),
                0.0,
                false,
            );
        }
        prev = Some(p);
    }
    // The ring, swinging in from above and filling the screen.
    let swing = (age * 5.0).sin() * 0.3 * (1.0 - k);
    let pivot = [c[0], -40.0 * s];
    let length = (260.0 + 260.0 * k) * s;
    let rc = [
        pivot[0] + swing.sin() * length,
        pivot[1] + swing.cos() * length,
    ];
    let r = (70.0 + 330.0 * k * k) * s;
    wire(
        v,
        pivot,
        [rc[0], rc[1] - r],
        4.0 * s,
        rgba(0xe8d0a0, 0.8),
        rgba(0xe8d0a0, 0.9),
        0.0,
        true,
    );
    halo(
        v,
        rc,
        r,
        (40.0 + 80.0 * k) * s,
        rgba(0xffb060, 0.35 + 0.3 * k),
    );
    ring_shape(v, rc, r, r * (0.4 + 0.6 * k), (12.0 + 26.0 * k) * s, 1.0);
    // The shout.
    let pop = entrance(age, 0.45, 0.22);
    if pop > 0.0 {
        let size = 120.0 * s * (0.6 + 0.4 * pop);
        let words = "¡A TRAICIÓN!";
        let tw = text_width(font, Face::Title, words, size);
        let at = [w * 0.5 - tw * 0.5, h * 0.72];
        turned(v, [w * 0.5, at[1] + size * 0.5], -0.12, |v| {
            burst(
                v,
                [w * 0.5, at[1] + size * 0.5],
                tw * 0.38,
                tw * 0.62 * pop,
                16,
                time * 0.8,
                rgba(0xff4a9a, 0.92),
                rgba(0x0b0508, 0.95),
                6.0 * s,
            );
            shout(
                font,
                v,
                Face::Title,
                words,
                at[0],
                at[1],
                size,
                0xffffff,
                0x6a1a9a,
                pop,
            );
        });
    }
    // Into black for his game.
    let black = ((age - 1.55) / 0.55).clamp(0.0, 1.0);
    if black > 0.0 {
        push_rect(v, Rect::new(0.0, 0.0, w, h), rgba(0x000000, black));
    }
}
