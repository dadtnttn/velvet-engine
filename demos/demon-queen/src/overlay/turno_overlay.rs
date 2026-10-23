//! «Al ritmo de la Reina» on the intimate scenes' stage (`explore/turno.rs`), drawn in code: over
//! the picture, in its lower right corner, her beat as a ring closing on a pink one (a green flash
//! on a beat on time, a red one off it) with his run of beats beside it, which view is on in the
//! upper left, and on the picture's lower edge a slanted plate with the shift's name and a pip for
//! each of her loops and the ending. Right of the vial, his column: his name, his face on a
//! medallion with a heart that beats faster as the column fills (blue while he holds back, red when
//! it is nearly over) and the tip so far on the plaque. What the keys light up only changes light
//! and colour, never a place.
use super::persona_screens::{fill, rim, slanted};
use super::scene_overlay::{hex3, TurnoView, Uv};
use super::ui_paint::*;
use super::*;

const PINK: u32 = 0xff6ea0;
const GOOD: u32 = 0x3fbf76;
const BAD: u32 = 0xd8344a;
const HOLD: u32 = 0x7fc8ff;
/// A run long enough that each beat is worth two gold (`turno::STREAK_GOLD`).
const RUN_GOLD: u16 = 8;

/// Her beat in the picture's corner, the view's tag and the shift's pips.
pub(super) fn over_picture(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    tv: &TurnoView,
    accent: [u8; 3],
    time: f32,
    s: f32,
) {
    view_tag(font, v, pic, tv, time, s);

    pips(font, v, pic, tv, accent, time, s);
    cut_in(font, v, pic, tv, accent, s);

    if !tv.beating {
        return;
    }
    // The beat, bottom right: the ring closes on hers.
    let c = [pic.x + pic.w - 74.0 * s, pic.y + pic.h - 74.0 * s];
    let target = 34.0 * s;
    let k = tv.ring as f32 / 1000.0;
    let hit = (1.0 - tv.hit_ms as f32 / 260.0).clamp(0.0, 1.0);
    let miss = (1.0 - tv.miss_ms as f32 / 320.0).clamp(0.0, 1.0);
    halo(v, c, target, 26.0 * s, rgba(SHADOW, 0.55));
    disc(v, c, target - 4.0 * s, rgba(0x14080e, 0.72));
    if hit > 0.0 {
        halo(
            v,
            c,
            target,
            (14.0 + 24.0 * hit) * s,
            rgba(GOOD, 0.75 * hit),
        );
    }
    if miss > 0.0 {
        halo(
            v,
            c,
            target,
            (10.0 + 18.0 * miss) * s,
            rgba(BAD, 0.7 * miss),
        );
    }
    // Her ring, fixed; the window narrows (a thinner ring) while he holds back.
    let rim_color = if hit > 0.0 {
        mix(PINK, 0xffffff, hit)
    } else {
        PINK
    };
    let width = if tv.held { 3.0 * s } else { 5.0 * s };
    arc(
        v,
        c,
        target,
        width,
        0.0,
        1.0,
        rgba(rim_color, 0.95),
        rgba(rim_color, 0.0),
    );
    // The closing ring: from far out to hers, brightening as it comes.
    let radius = target * (2.2 - 1.2 * k);
    arc(
        v,
        c,
        radius,
        3.0 * s,
        0.0,
        1.0,
        rgba(0xffffff, 0.25 + 0.7 * k),
        rgba(0xffffff, 0.0),
    );
    // The heart of it: her colour, pulsing on the beat.
    let pulse = (1.0 - k).powi(6);
    disc(
        v,
        c,
        (9.0 + 5.0 * pulse) * s,
        rgba(mix(hex3(accent), PINK, 0.5), 0.85),
    );
    if hit > 0.0 {
        text_centered(
            font,
            v,
            Face::Title,
            "¡BIEN!",
            c[0],
            c[1] - target - 34.0 * s - 10.0 * s * hit,
            18.0 * s,
            rgba(0xe8ffe8, hit),
        );
    } else if miss > 0.0 {
        text_centered(
            font,
            v,
            Face::Title,
            "FUERA",
            c[0],
            c[1] - target - 34.0 * s,
            16.0 * s,
            rgba(0xffc0c8, miss),
        );
    }

    // The first time: what to do, over the ring.
    if tv.teach {
        let a = 0.75 + 0.25 * wave(time, 1.6).abs();
        let line = "Pulsa ESPACIO cuando se cierre el anillo";
        let size = 17.0 * s;
        let w = text_width(font, Face::Ui, line, size) + 28.0 * s;
        let r = Rect::new(
            pic.x + pic.w - w - 16.0 * s,
            c[1] - target - 92.0 * s,
            w,
            32.0 * s,
        );
        panel(
            v,
            r,
            16.0 * s,
            Panel::fill(0x14080e, 0x07030c, 0.88 * a).rim(PINK, a, 1.6 * s),
        );
        text_centered(
            font,
            v,
            Face::Ui,
            line,
            r.x + r.w * 0.5,
            r.y + (r.h - size) * 0.5 - 1.0 * s,
            size,
            rgba(CREAM, a),
        );
    }

    // His run, left of the ring: gold once each beat is worth two.
    if tv.streak >= 3 {
        let gold = tv.streak >= RUN_GOLD;
        let color = if gold { 0xffd56b } else { CREAM };
        let rc = [c[0] - target - 58.0 * s, c[1] + 4.0 * s];
        let bump = 1.0 + 0.18 * hit;
        if gold {
            halo(v, rc, 26.0 * s, 20.0 * s, rgba(0xffb030, 0.35 + 0.2 * hit));
        }
        let count = format!("×{}", tv.streak);
        let size = 30.0 * s * bump;
        text_centered(
            font,
            v,
            Face::Title,
            &count,
            rc[0] + 1.5 * s,
            rc[1] - size * 0.62 + 2.0 * s,
            size,
            rgba(SHADOW, 0.8),
        );
        text_centered(
            font,
            v,
            Face::Title,
            &count,
            rc[0],
            rc[1] - size * 0.62,
            size,
            rgba(color, 1.0),
        );
        text_centered(
            font,
            v,
            Face::Ui,
            if gold { "RACHA · ORO ×2" } else { "RACHA" },
            rc[0],
            rc[1] + 12.0 * s,
            12.0 * s,
            rgba(color, 0.9),
        );
    }
}

/// The view, top left: a plain tag when the take has one view; a button (its key, where it takes
/// him) when it has two, lit and marked «¡NUEVA!» while the take is new.
fn view_tag(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    tv: &TurnoView,
    time: f32,
    s: f32,
) {
    let now = if tv.side { "DE LADO" } else { "SUS OJOS" };
    let (x, y) = (pic.x + 16.0 * s, pic.y + 16.0 * s);
    if !tv.views {
        let size = 15.0 * s;
        let w = text_width(font, Face::Ui, now, size) + 24.0 * s;
        let r = Rect::new(x, y, w, 30.0 * s);
        panel(
            v,
            r,
            15.0 * s,
            Panel::fill(0x14080e, 0x07030c, 0.82).rim(GOLD, 0.9, 1.5 * s),
        );
        text_centered(
            font,
            v,
            Face::Ui,
            now,
            r.x + r.w * 0.5,
            r.y + (r.h - size) * 0.5 - 1.0 * s,
            size,
            rgba(CREAM, 1.0),
        );
        return;
    }
    let fresh = (1.0 - tv.view_new as f32 / 3500.0).clamp(0.0, 1.0);
    let pulse = fresh * (0.5 + 0.5 * wave(time, 2.4).abs());
    let target = if tv.side {
        "VER SUS OJOS"
    } else {
        "VER DE LADO"
    };
    let size = 17.0 * s;
    let cap = 38.0 * s;
    let tag = "¡NUEVA!";
    let ts = 13.0 * s;
    let tag_w = text_width(font, Face::Title, tag, ts) + 16.0 * s;
    let label_w = text_width(font, Face::Title, target, size);
    let w = cap + 14.0 * s + label_w + 26.0 * s + if fresh > 0.0 { tag_w + 10.0 * s } else { 0.0 };
    let h = 42.0 * s;
    if pulse > 0.0 {
        soft(
            v,
            Rect::new(x, y, w, h),
            12.0 * s,
            18.0 * s,
            rgba(0xffd56b, 0.55 * pulse),
            true,
        );
    }
    let plate = slanted(x, y, w, h, 7.0 * s);
    fill(
        v,
        slanted(x + 4.0 * s, y + 5.0 * s, w, h, 7.0 * s),
        rgba(SHADOW, 0.6),
    );
    fill(
        v,
        plate,
        rgba(mix(0x14080e, 0x5a3a10, 0.3 + 0.5 * pulse), 0.94),
    );
    rim(
        v,
        plate,
        1.8 * s,
        rgba(mix(GOLD, 0xffffff, pulse * 0.6), 1.0),
    );
    keycap(
        font,
        v,
        Rect::new(
            x + 10.0 * s,
            y + (h - 30.0 * s) * 0.5,
            cap - 6.0 * s,
            30.0 * s,
        ),
        "V",
        17.0 * s,
        pulse > 0.3,
        false,
        0xffd56b,
        1.0,
    );
    text(
        font,
        v,
        Face::Title,
        target,
        x + cap + 16.0 * s,
        y + (h - size) * 0.5 - 2.0 * s,
        size,
        rgba(CREAM, 1.0),
    );
    if fresh > 0.0 {
        let r = Rect::new(
            x + cap + 16.0 * s + label_w + 10.0 * s,
            y + (h - 22.0 * s) * 0.5,
            tag_w,
            22.0 * s,
        );
        panel(
            v,
            r,
            4.0 * s,
            Panel::fill(0xffd56b, 0xe8a030, fresh).rim(0x1a0a10, fresh, 1.5 * s),
        );
        text_centered(
            font,
            v,
            Face::Title,
            tag,
            r.x + r.w * 0.5,
            r.y + (r.h - ts) * 0.5 - 1.0 * s,
            ts,
            rgba(0x1a0a10, fresh),
        );
    }
}

/// The plate's buttons: Space on her beat (lighting up as the ring closes, green on a beat caught,
/// red off it), Shift to hold back (blue while held), V for the other view (lit and marked new
/// when the take has one, dim with «Vista única» when it has not). Only light and colour change.
pub(super) fn draw_buttons(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    plate: Rect,
    tv: &TurnoView,
    accent: u32,
    time: f32,
    s: f32,
) {
    let gap = 14.0 * s;
    let bw = (plate.w - 48.0 * s - gap * 2.0) / 3.0;
    let bh = 64.0 * s;
    let y = plate.y + 56.0 * s;
    let hit = (1.0 - tv.hit_ms as f32 / 260.0).clamp(0.0, 1.0);
    let miss = (1.0 - tv.miss_ms as f32 / 320.0).clamp(0.0, 1.0);
    let beat = if tv.beating {
        (tv.ring as f32 / 1000.0).powi(6)
    } else {
        0.0
    };
    let fresh = (1.0 - tv.view_new as f32 / 3500.0).clamp(0.0, 1.0);
    let target = if tv.side {
        "Ver sus ojos"
    } else {
        "Ver de lado"
    };
    struct B<'a> {
        key: &'a str,
        label: &'a str,
        color: u32,
        light: f32,
        on: bool,
    }
    let space_color = if hit > 0.0 {
        GOOD
    } else if miss > 0.0 {
        BAD
    } else {
        PINK
    };
    let buttons = [
        B {
            key: "ESPACIO",
            label: "Al compás",
            color: space_color,
            light: beat.max(hit).max(miss),
            on: tv.beating,
        },
        B {
            key: "SHIFT",
            label: if tv.held { "Aguantando" } else { "Contenerte" },
            color: HOLD,
            light: if tv.held { 1.0 } else { 0.0 },
            on: true,
        },
        B {
            key: "V",
            label: if tv.views { target } else { "Vista única" },
            color: 0xffd56b,
            light: if tv.views {
                0.35 + 0.65 * fresh * (0.5 + 0.5 * wave(time, 2.4).abs())
            } else {
                0.0
            },
            on: tv.views,
        },
    ];
    // Every key's width, and one size for the three words: the largest that fits them all.
    let ks = 13.0 * s;
    let key_w = |key: &str| (text_width(font, Face::Title, key, ks) + 18.0 * s).max(34.0 * s);
    let room_for = |key: &str| bw - (16.0 * s + key_w(key) + 12.0 * s) - 24.0 * s;
    let mut ls = 21.0 * s;
    while ls > 13.0 * s
        && buttons
            .iter()
            .any(|b| text_width(font, Face::Title, b.label, ls) > room_for(b.key))
    {
        ls -= 0.5 * s;
    }
    for (i, b) in buttons.iter().enumerate() {
        let x = plate.x + 24.0 * s + i as f32 * (bw + gap);
        let r = Rect::new(x, y, bw, bh);
        let alpha = if b.on { 1.0 } else { 0.45 };
        if b.light > 0.05 && b.on {
            soft(
                v,
                r,
                14.0 * s,
                16.0 * s,
                rgba(b.color, 0.45 * b.light),
                true,
            );
        }
        let quad = slanted(x, y, bw, bh, 8.0 * s);
        fill(
            v,
            slanted(x + 4.0 * s, y + 5.0 * s, bw, bh, 8.0 * s),
            rgba(SHADOW, 0.55 * alpha),
        );
        fill(
            v,
            quad,
            rgba(mix(0x1a0c20, b.color, 0.12 + 0.4 * b.light), 0.95 * alpha),
        );
        rim(
            v,
            quad,
            1.8 * s,
            rgba(mix(GOLD_DEEP, b.color, 0.3 + 0.7 * b.light), alpha),
        );
        let kw = key_w(b.key);
        keycap(
            font,
            v,
            Rect::new(x + 16.0 * s, y + (bh - 34.0 * s) * 0.5, kw, 34.0 * s),
            b.key,
            ks,
            b.light > 0.5 && b.on,
            !b.on,
            if i == 0 { accent } else { b.color },
            alpha,
        );
        // One word, between the key and the button's slanted right edge.
        let tx = x + 16.0 * s + kw + 12.0 * s;
        text(
            font,
            v,
            Face::Title,
            b.label,
            tx,
            y + (bh - ls) * 0.5 - 2.0 * s,
            ls,
            rgba(CREAM, alpha),
        );
    }
}

/// The shift's plate on the picture's lower edge: its name and a pip for each loop (lit behind
/// him, glowing where he is) and a last, larger one for the ending.
fn pips(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    tv: &TurnoView,
    accent: [u8; 3],
    time: f32,
    s: f32,
) {
    let steps = tv.steps.max(1) as usize;
    let size = 14.0 * s;
    let name = tv.act.to_uppercase();
    let nw = text_width(font, Face::Title, &name, size);
    let gap = 26.0 * s;
    let w = nw + 30.0 * s + gap * steps as f32 + 10.0 * s;
    let h = 30.0 * s;
    let x = pic.x + (pic.w - w) * 0.5;
    let y = pic.y + pic.h - h * 0.5;
    let lean = 8.0 * s;
    fill(
        v,
        slanted(x + 4.0 * s, y + 5.0 * s, w, h, lean),
        rgba(SHADOW, 0.6),
    );
    let plate = slanted(x, y, w, h, lean);
    fill(v, plate, rgba(0x14080e, 0.94));
    rim(v, plate, 1.6 * s, rgba(hex3(accent), 0.9));
    text(
        font,
        v,
        Face::Title,
        &name,
        x + 18.0 * s,
        y + (h - size) * 0.5 - 1.0 * s,
        size,
        rgba(CREAM, 1.0),
    );
    let lit = mix(hex3(accent), PINK, 0.4);
    let at = tv.at as usize;
    for i in 0..steps {
        let last = i + 1 == steps;
        let c = [x + nw + 30.0 * s + gap * (i as f32 + 0.5), y + h * 0.5];
        let (hw, hh) = if last {
            (8.0 * s, 10.0 * s)
        } else {
            (6.0 * s, 8.0 * s)
        };
        if i < at {
            gem(v, c, hw, hh, lit, 1.0, 0.0);
        } else if i == at {
            let beat = 0.5 + 0.5 * wave(time, 1.2).abs();
            halo(v, c, hw * 1.6, 10.0 * s, rgba(lit, 0.45 + 0.3 * beat));
            gem(
                v,
                c,
                hw * 1.15,
                hh * 1.15,
                mix(lit, 0xffffff, 0.35 + 0.3 * beat),
                1.0,
                0.6,
            );
        } else {
            gem(
                v,
                c,
                hw,
                hh,
                if last { 0x3a2a10 } else { 0x241830 },
                1.0,
                0.0,
            );
        }
    }
}

/// Her new loop, thrown across the top of the picture for a moment: the take's number on a tab
/// and its name on a slanted plate, sliding in and fading out.
fn cut_in(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    tv: &TurnoView,
    accent: [u8; 3],
    s: f32,
) {
    if tv.cut.is_empty() {
        return;
    }
    let t = tv.cut_ms as f32 / 1000.0;
    let alpha = (t / 0.2).min(1.0) * (1.0 - ((t - 1.9) / 0.4).clamp(0.0, 1.0));
    if alpha <= 0.0 {
        return;
    }
    let slide = (1.0 - (t / 0.25).min(1.0)).powi(3) * 60.0 * s;
    let size = 26.0 * s;
    let w = text_width(font, Face::Title, &tv.cut, size) + 64.0 * s;
    let h = 46.0 * s;
    let x = pic.x + (pic.w - w) * 0.5 + slide;
    // under the view's button, top left
    let y = pic.y + 96.0 * s;
    let lean = 12.0 * s;
    fill(
        v,
        slanted(x + 6.0 * s, y + 7.0 * s, w, h, lean),
        rgba(SHADOW, 0.6 * alpha),
    );
    let plate = slanted(x, y, w, h, lean);
    fill(
        v,
        plate,
        rgba(mix(0x14080e, hex3(accent), 0.25), 0.95 * alpha),
    );
    rim(v, plate, 2.0 * s, rgba(hex3(accent), alpha));
    text_centered(
        font,
        v,
        Face::Title,
        &tv.cut,
        x + w * 0.5,
        y + (h - size) * 0.5 - 2.0 * s,
        size,
        rgba(CREAM, alpha),
    );
    // The take's number on its tab, top left of the plate.
    let tag = format!("TOMA {}/{}", tv.at as usize + 1, tv.steps.max(1) - 1);
    let ts = 13.0 * s;
    let tw = text_width(font, Face::Ui, &tag, ts) + 22.0 * s;
    let tab = slanted(x + 4.0 * s, y - 20.0 * s, tw, 22.0 * s, 5.0 * s);
    fill(v, tab, rgba(hex3(accent), alpha));
    text_centered(
        font,
        v,
        Face::Ui,
        &tag,
        x + 4.0 * s + tw * 0.5,
        y - 20.0 * s + (22.0 * s - ts) * 0.5 - 1.0 * s,
        ts,
        rgba(0x1a0a10, alpha),
    );
}

/// A heart of `size` round `c`.
fn heart(v: &mut Vec<MapVertex>, c: [f32; 2], size: f32, color: [f32; 4]) {
    disc(
        v,
        [c[0] - size * 0.45, c[1] - size * 0.25],
        size * 0.55,
        color,
    );
    disc(
        v,
        [c[0] + size * 0.45, c[1] - size * 0.25],
        size * 0.55,
        color,
    );
    push_triangle(
        v,
        [
            [c[0] - size * 0.98, c[1] - size * 0.1],
            [c[0] + size * 0.98, c[1] - size * 0.1],
            [c[0], c[1] + size * 0.95],
        ],
        color,
    );
}

/// His column, right of the vial: his name, his face with a heart that races as it fills, the
/// bar and the tip on its plaque.
pub(super) fn draw_column(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    frame: Rect,
    tv: &TurnoView,
    face_uv: Option<Uv>,
    time: f32,
    s: f32,
) {
    let level = tv.stamina as f32 / 1000.0;
    let x = frame.x + frame.w + 36.0 * s + 120.0 * s + 30.0 * s;
    let column = Rect::new(x, frame.y, 112.0 * s, frame.h);
    let mx = column.x + column.w * 0.5;
    let near = level > 0.8;
    let rim_color = if tv.held {
        HOLD
    } else if near {
        BAD
    } else {
        GOLD
    };
    soft(
        v,
        Rect::new(column.x, column.y + 14.0 * s, column.w, column.h),
        30.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    if near {
        soft(
            v,
            column,
            30.0 * s,
            26.0 * s,
            rgba(BAD, 0.3 + 0.25 * wave(time, 2.4).abs()),
            true,
        );
    } else if tv.held {
        soft(v, column, 30.0 * s, 22.0 * s, rgba(HOLD, 0.35), true);
    }
    panel(
        v,
        column,
        30.0 * s,
        Panel::velvet(0.95, 2.6 * s)
            .rim(rim_color, 1.0, 2.6 * s)
            .hairline(6.0 * s, 0.4),
    );
    // His name, and what the column is.
    let title = if tv.name.is_empty() {
        "ÉL".to_owned()
    } else {
        tv.name.to_uppercase()
    };
    text_centered(
        font,
        v,
        Face::Title,
        &title,
        mx,
        column.y + 12.0 * s,
        19.0 * s,
        rgba(if near { 0xffd8dc } else { CREAM }, 1.0),
    );
    text_centered(
        font,
        v,
        Face::Ui,
        "aguante",
        mx,
        column.y + 33.0 * s,
        13.0 * s,
        rgba(if tv.held { HOLD } else { 0xe8c890 }, 1.0),
    );

    // The medallion: his face, his heart racing on it as it fills.
    let mc = [mx, column.y + 92.0 * s];
    let mr = 38.0 * s;
    panel(
        v,
        Rect::new(mc[0] - mr, mc[1] - mr, mr * 2.0, mr * 2.0),
        mr,
        Panel::fill(mix(0x1c1428, 0x4a0c1a, level), 0x0b0812, 1.0)
            .rim(rim_color, 1.0, 3.0 * s)
            .bevel(1.0),
    );
    let beat = heartbeat(time, 70.0 + 110.0 * level);
    let red = rgba(mix(0xd04a6a, 0xff2a4a, level), 1.0);
    match face_uv {
        Some(uv) => {
            let fr = mr - 4.0 * s;
            face(
                v,
                Rect::new(mc[0] - fr, mc[1] - fr, fr * 2.0, fr * 2.0),
                fr,
                uv,
                1.0,
            );
            // Flushed as it fills.
            disc(v, mc, fr, rgba(0xff2a4a, 0.28 * level * level));
            // The heart on its badge, lower right.
            let hc = [mc[0] + mr * 0.72, mc[1] + mr * 0.72];
            disc(v, hc, 15.0 * s, rgba(0x14080e, 0.95));
            arc(
                v,
                hc,
                15.0 * s,
                2.0 * s,
                0.0,
                1.0,
                rgba(rim_color, 1.0),
                rgba(rim_color, 1.0),
            );
            heart(v, [hc[0], hc[1] + 1.0 * s], (8.0 + 2.5 * beat) * s, red);
        }
        None => heart(v, mc, (15.0 + 4.0 * beat) * s, red),
    }

    // The bar.
    let bar = Rect::new(
        mx - 24.0 * s,
        column.y + 150.0 * s,
        48.0 * s,
        column.h - 150.0 * s - 70.0 * s,
    );
    panel(
        v,
        bar,
        24.0 * s,
        Panel::fill(0x14101e, 0x08060e, 1.0).rim(GOLD_DEEP, 0.9, 1.6 * s),
    );
    let inner = grow(bar, -5.0 * s);
    let fill_h = inner.h * level.clamp(0.0, 1.0);
    if fill_h > 1.0 {
        let liquid = Rect::new(inner.x, inner.y + inner.h - fill_h, inner.w, fill_h);
        let top = if near {
            mix(0xff8a8a, 0xffffff, wave(time, 3.0).abs() * 0.5)
        } else {
            mix(0x7fc8ff, 0xffffff, level * 0.6)
        };
        panel(
            v,
            liquid,
            inner.w * 0.5,
            Panel::fill(top, mix(0x2a6aa0, 0xa02040, level), 1.0)
                .bevel(0.4)
                .sheen(time * 0.25),
        );
    }
    // The plaque: the tip so far.
    let plaque = Rect::new(
        mx - 46.0 * s,
        column.y + column.h - 60.0 * s,
        92.0 * s,
        42.0 * s,
    );
    panel(
        v,
        plaque,
        21.0 * s,
        Panel::fill(0x3a2a10, 0x140c06, 1.0)
            .rim(GOLD, 1.0, 2.0 * s)
            .bevel(1.0),
    );
    let label = format!("+{} oro", tv.tip);
    let ls = 18.0 * s;
    text_centered(
        font,
        v,
        Face::Ui,
        &label,
        mx,
        plaque.y + (42.0 * s - ls) * 0.5 - 2.0 * s,
        ls,
        rgba(0xffe08a, 1.0),
    );
}
