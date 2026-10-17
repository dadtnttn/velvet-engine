//! «No mires el anillo» on the intimate scenes' stage (`explore/hipno.rs`), drawn in code: over the
//! picture the hypnotist's copper ring swinging on its string (burning brighter when he whispers or
//! gives an order), the trance's spiral turning round it, her gaze as an eye that slides under it
//! (sea green while it keeps away, pink when it is caught), his whispers drifting up in wavering
//! violet letters, and his order in a purple balloon at the top. Right of the vial, a column of her
//! waking: an eye that opens as «Despertar» fills (fangs, as «Lucidez», with her mouth busy). What
//! the keys light up only changes light and colour, never a place.
use std::f32::consts::{PI, TAU};

use super::persona::*;
use super::scene_overlay::HypnoView;
use super::ui_paint::*;
use super::*;

const COPPER: u32 = 0xd98a3a;
const COPPER_HI: u32 = 0xffd08a;
const VIOLET: u32 = 0xb05cff;
const MINT: u32 = 0x5affc8;
const CAUGHT: u32 = 0xff6ef0;
const WHISPER: u32 = 0xf2c2ff;

/// A point given in per mille of the picture (plus 200) on the screen.
fn at(pic: Rect, p: [u16; 2]) -> [f32; 2] {
    [
        pic.x + (p[0] as f32 - 200.0) / 1000.0 * pic.w,
        pic.y + (p[1] as f32 - 200.0) / 1000.0 * pic.h,
    ]
}

fn inside(pic: Rect, p: [f32; 2]) -> bool {
    p[0] > pic.x + 2.0
        && p[0] < pic.x + pic.w - 2.0
        && p[1] > pic.y + 2.0
        && p[1] < pic.y + pic.h - 2.0
}

/// An ellipse drawn as a loop of wires (`rx` across, `ry` up), turned by `angle`.
#[allow(clippy::too_many_arguments)]
fn ellipse(
    v: &mut Vec<MapVertex>,
    c: [f32; 2],
    rx: f32,
    ry: f32,
    angle: f32,
    width: f32,
    color: [f32; 4],
    metal: bool,
) {
    let n = 28;
    let (sa, ca) = angle.sin_cos();
    let p = |k: usize| {
        let t = k as f32 / n as f32 * TAU;
        let (x, y) = (t.cos() * rx, t.sin() * ry);
        [c[0] + x * ca - y * sa, c[1] + x * sa + y * ca]
    };
    for k in 0..n {
        wire(v, p(k), p(k + 1), width, color, color, 0.0, metal);
    }
}

/// Everything over the picture.
pub(super) fn over_picture(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    hv: &HypnoView,
    time: f32,
    s: f32,
) {
    if hv.mode == 0 {
        draw_ring(v, pic, hv, time, s);
        draw_gaze(font, v, pic, hv, time, s);
        for (text, age, seed) in &hv.whispers {
            draw_whisper(font, v, pic, hv, text, *age as f32 / 1000.0, *seed, time, s);
        }
    }
    if let Some((text, age)) = &hv.order {
        draw_order(font, v, pic, text, *age as f32 / 1000.0, time, s);
    }
    // The smelling salts: a sharp sea-green bite of air.
    let salt = hv.salt_ms as f32 / 1000.0;
    if salt < 0.9 {
        let k = 1.0 - salt / 0.9;
        vignette(
            v,
            pic,
            16.0 * s,
            pic.w * (0.2 + 0.25 * k),
            rgba(MINT, 0.55 * k),
        );
    }
}

/// The light the trance throws on the room: a slow violet breath from the edges, deeper as it
/// sinks; a pink beat while her gaze is caught; gold while Chiquita guards her.
pub(super) fn room(
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    hv: &HypnoView,
    time: f32,
    s: f32,
) {
    let trance = hv.trance as f32 / 1000.0;
    let screen = Rect::new(0.0, 0.0, width, height);
    if hv.mode == 0 {
        let breath = 0.6 + 0.4 * (time * 0.9).sin().abs();
        vignette(
            v,
            screen,
            0.0,
            (180.0 + 220.0 * trance) * s,
            rgba(VIOLET, (0.05 + 0.22 * trance) * breath),
        );
        if hv.on_ring {
            vignette(
                v,
                screen,
                0.0,
                140.0 * s,
                rgba(CAUGHT, 0.08 + 0.05 * wave(time, 1.4).abs()),
            );
        }
        if hv.covered {
            vignette(v, screen, 0.0, 160.0 * s, rgba(0xffc86a, 0.12));
        }
    } else if hv.mode == 1 {
        vignette(
            v,
            screen,
            0.0,
            260.0 * s,
            rgba(0xff4a9a, 0.12 + 0.04 * (time * 1.3).sin()),
        );
    }
}

/// The string, the ring on it and the trance's spiral round it.
fn draw_ring(v: &mut Vec<MapVertex>, pic: Rect, hv: &HypnoView, time: f32, s: f32) {
    let pivot = at(pic, hv.pivot);
    let ring = at(pic, hv.ring);
    let trance = hv.trance as f32 / 1000.0;
    let burn = hv.burn as f32 / 1000.0;
    let dim = if hv.covered { 0.3 } else { 1.0 };
    // The spiral, turning, stronger the deeper she is.
    if trance > 0.12 {
        let a = ((trance - 0.12) * 0.9).min(0.42) * dim;
        let turns = 5.0;
        let reach = pic.w * 0.42;
        let n = 150;
        let mut prev: Option<[f32; 2]> = None;
        for k in 0..=n {
            let u = k as f32 / n as f32;
            let ang = u * turns * TAU - time * 2.4;
            let r = u * reach;
            let p = [ring[0] + ang.cos() * r, ring[1] + ang.sin() * r];
            if let Some(q) = prev {
                if inside(pic, p) && inside(pic, q) {
                    let fade = a * (1.0 - u * 0.65);
                    wire(
                        v,
                        q,
                        p,
                        (2.0 + 2.0 * (1.0 - u)) * s,
                        rgba(VIOLET, fade),
                        rgba(0xff8af0, fade),
                        0.0,
                        false,
                    );
                }
            }
            prev = Some(p);
        }
    }
    // The string from the top of the picture to the ring.
    let top = [pivot[0], pivot[1].max(pic.y + 1.0)];
    let k = (top[1] - pivot[1]) / (ring[1] - pivot[1]).max(1.0);
    let from = [pivot[0] + (ring[0] - pivot[0]) * k, top[1]];
    let rr = 24.0 * s;
    let hang = [ring[0], ring[1] - rr];
    wire(
        v,
        from,
        hang,
        2.2 * s,
        rgba(0xe8d0a0, 0.75 * dim),
        rgba(0xe8d0a0, 0.85 * dim),
        0.0,
        true,
    );
    // Its glow, brighter after a whisper or an order.
    halo(
        v,
        ring,
        rr * 1.2,
        (18.0 + 26.0 * burn) * s,
        rgba(0xffb060, (0.25 + 0.4 * burn) * dim),
    );
    // The ring itself, spinning on the string: its face turning to us and away.
    let face = (hv.turn as f32 / 1000.0).clamp(0.08, 1.0);
    let ry = rr * face;
    ellipse(
        v,
        ring,
        rr,
        ry,
        0.0,
        7.5 * s,
        rgba(0x6a3a14, 0.9 * dim),
        false,
    );
    ellipse(v, ring, rr, ry, 0.0, 5.2 * s, rgba(COPPER, dim), true);
    // A glint running round it.
    let g = time * 3.1;
    let glint = [ring[0] + g.cos() * rr, ring[1] + g.sin() * ry];
    sparkle(v, glint, (6.0 + 6.0 * burn) * s, rgba(COPPER_HI, dim), 0.8);
}

/// Her gaze: an eye sliding across under the ring; caught on it, it turns pink; staring on
/// purpose, a heart beats in it.
fn draw_gaze(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    hv: &HypnoView,
    time: f32,
    s: f32,
) {
    let ring = at(pic, hv.ring);
    let x = at(pic, [hv.gaze, 200])[0];
    let c = [x, ring[1] + 70.0 * s];
    let color = if hv.covered {
        0xffc86a
    } else if hv.on_ring || hv.staring {
        CAUGHT
    } else {
        MINT
    };
    let pulse = if hv.on_ring {
        0.85 + 0.15 * wave(time, 2.2).abs()
    } else {
        1.0
    };
    halo(v, c, 14.0 * s, 16.0 * s, rgba(color, 0.35 * pulse));
    // The almond of the eye: an upper and a lower lid.
    let (w, h) = (34.0 * s, 15.0 * s);
    let n = 16;
    for (sign, width) in [(-1.0f32, 3.8 * s), (1.0, 3.0 * s)] {
        for k in 0..n {
            let a0 = k as f32 / n as f32;
            let a1 = (k + 1) as f32 / n as f32;
            let p = |t: f32| [c[0] - w + 2.0 * w * t, c[1] + sign * h * (t * PI).sin()];
            wire(
                v,
                p(a0),
                p(a1),
                width,
                rgba(0x0b0508, 0.75),
                rgba(0x0b0508, 0.75),
                0.0,
                false,
            );
            wire(
                v,
                p(a0),
                p(a1),
                width * 0.6,
                rgba(color, 1.0),
                rgba(color, 1.0),
                0.0,
                false,
            );
        }
    }
    // The pupil looks at the ring.
    let look = ((ring[0] - c[0]) / (pic.w * 0.3)).clamp(-1.0, 1.0) * 8.0 * s;
    disc(v, [c[0] + look, c[1]], 8.0 * s, rgba(0x0b0508, 0.9));
    disc(v, [c[0] + look, c[1]], 6.0 * s, rgba(color, 1.0));
    if hv.staring {
        let beat = heartbeat(time, 96.0);
        sparkle(
            v,
            [c[0] + look, c[1]],
            (9.0 + 5.0 * beat) * s,
            rgba(0xffffff, 0.9),
            0.9,
        );
    }
    // A thread from the eye to the ring when it is caught: the pull.
    if hv.on_ring && !hv.staring {
        let flicker = 0.4 + 0.3 * wave(time, 3.0).abs();
        wire(
            v,
            [c[0], c[1] - h],
            [ring[0], ring[1] + 24.0 * s],
            2.0 * s,
            rgba(CAUGHT, flicker),
            rgba(CAUGHT, 0.0),
            0.0,
            false,
        );
    }
    let label = if hv.covered {
        "Chiquita te tapa"
    } else if hv.staring {
        "¡a propósito!"
    } else if hv.on_ring {
        "mirando el anillo"
    } else {
        "tu mirada"
    };
    let size = 20.0 * s;
    let tw = text_width(font, Face::Italic, label, size);
    let lx = (c[0] - tw * 0.5).clamp(pic.x + 8.0 * s, pic.x + pic.w - tw - 8.0 * s);
    text_outline(
        font,
        v,
        Face::Italic,
        label,
        lx,
        c[1] + 18.0 * s,
        size,
        rgba(color, 1.0),
        rgba(0x0b0508, 0.85),
    );
}

/// One whisper: its words wavering up from beside the ring, fading in and out.
#[allow(clippy::too_many_arguments)]
fn draw_whisper(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    hv: &HypnoView,
    text: &str,
    age: f32,
    seed: u16,
    time: f32,
    s: f32,
) {
    let life = 5.2;
    if age >= life {
        return;
    }
    let alpha = (age / 0.5).min(1.0) * (1.0 - ((age - (life - 1.2)) / 1.2).clamp(0.0, 1.0));
    let ring = at(pic, hv.ring);
    let side = if seed % 2 == 0 { -1.0 } else { 1.0 };
    let size = 25.0 * s;
    let lines = wrap(font, Face::Italic, text, size, pic.w * 0.52);
    let width = lines
        .iter()
        .map(|l| text_width(font, Face::Italic, l, size))
        .fold(0.0, f32::max);
    let drift = age * 16.0 * s;
    let x0 = if side < 0.0 {
        ring[0] - 60.0 * s - width - drift * 0.6
    } else {
        ring[0] + 60.0 * s + drift * 0.6
    };
    let x0 = x0.clamp(pic.x + 14.0 * s, pic.x + pic.w - width - 14.0 * s);
    let y0 = ring[1] - 70.0 * s - (seed % 5) as f32 * 14.0 * s - drift;
    let y0 = y0.max(pic.y + 110.0 * s);
    for (i, line) in lines.iter().enumerate() {
        let mut x = x0;
        let y = y0 + i as f32 * size * 1.3;
        for (k, ch) in line.chars().enumerate() {
            let mut buf = [0u8; 4];
            let glyph: &str = ch.encode_utf8(&mut buf);
            let wobble = (time * 3.2 + k as f32 * 0.55 + seed as f32).sin() * 4.0 * s;
            let tint = mix(
                WHISPER,
                0xff8af0,
                0.5 + 0.5 * (time * 1.7 + k as f32 * 0.3).sin(),
            );
            text_outline(
                font,
                v,
                Face::Italic,
                glyph,
                x,
                y + wobble,
                size,
                rgba(tint, alpha),
                rgba(0x1a0628, 0.8 * alpha),
            );
            x += text_width(font, Face::Italic, glyph, size);
        }
    }
}

/// His order: a purple balloon popping in at the top of the picture.
fn draw_order(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    words: &str,
    age: f32,
    time: f32,
    s: f32,
) {
    let k = {
        let x = (age / 0.28).clamp(0.0, 1.0) - 1.0;
        let c = 1.6;
        (1.0 + (c + 1.0) * x * x * x + c * x * x).max(0.05)
    };
    let mut size = 31.0 * s;
    let room = pic.w - 150.0 * s;
    while size > 22.0 * s && wrap(font, Face::Title, words, size, room).len() > 2 {
        size -= 1.0 * s;
    }
    let lines = wrap(font, Face::Title, words, size, room);
    let tw = lines
        .iter()
        .map(|l| text_width(font, Face::Title, l, size))
        .fold(0.0, f32::max);
    let w = (tw + 96.0 * s) * k;
    let h = (lines.len() as f32 * size * 1.2 + 46.0 * s) * k;
    let c = [pic.x + pic.w * 0.5, pic.y + 30.0 * s + h * 0.5];
    let r = Rect::new(c[0] - w * 0.5, c[1] - h * 0.5, w, h);
    let angle = 0.035 + 0.01 * (time * 2.3).sin();
    turned(v, c, angle, |v| {
        soft(
            v,
            Rect::new(r.x + 10.0 * s, r.y + 14.0 * s, r.w, r.h),
            14.0 * s,
            12.0 * s,
            rgba(SHADOW, 0.7),
            false,
        );
        soft(
            v,
            r,
            14.0 * s,
            20.0 * s,
            rgba(VIOLET, 0.45 + 0.15 * wave(time, 1.6)),
            true,
        );
        panel(
            v,
            grow(r, 5.0 * s),
            16.0 * s,
            Panel::fill(0x0b0508, 0x0b0508, 1.0),
        );
        panel(
            v,
            r,
            12.0 * s,
            Panel::fill(0x6a1a9a, 0x2a0846, 0.98)
                .rim(0xffd2ff, 1.0, 2.0 * s)
                .bevel(0.6)
                .sheen(time * 0.2),
        );
        // A little spiral at its left, turning.
        let sc = [r.x + 34.0 * s * k, c[1]];
        let mut prev = sc;
        for i in 1..=40 {
            let u = i as f32 / 40.0;
            let ang = u * 3.0 * TAU + time * 4.0;
            let p = [
                sc[0] + ang.cos() * u * 15.0 * s * k,
                sc[1] + ang.sin() * u * 15.0 * s * k,
            ];
            wire(
                v,
                prev,
                p,
                2.0 * s,
                rgba(0xffd2ff, 0.9),
                rgba(0xffd2ff, 0.9),
                0.0,
                false,
            );
            prev = p;
        }
        if k > 0.6 {
            let tab = "ORDEN DEL MAGO";
            let ts = 15.0 * s;
            let tab_w = text_width(font, Face::Title, tab, ts) + 22.0 * s;
            let tr = Rect::new(r.x + 22.0 * s, r.y - 13.0 * s, tab_w, 24.0 * s);
            panel(
                v,
                grow(tr, 2.5 * s),
                6.0 * s,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            panel(v, tr, 5.0 * s, Panel::fill(GOLD_HI, GOLD_DEEP, 1.0));
            text(
                font,
                v,
                Face::Title,
                tab,
                tr.x + 11.0 * s,
                tr.y + 3.0 * s,
                ts,
                rgba(0x2a0a10, 1.0),
            );
            for (i, line) in lines.iter().enumerate() {
                let lw = text_width(font, Face::Title, line, size);
                let y = r.y + 24.0 * s + i as f32 * size * 1.2;
                text_outline(
                    font,
                    v,
                    Face::Title,
                    line,
                    c[0] - lw * 0.5 + 14.0 * s,
                    y,
                    size,
                    rgba(0xfff4ff, 1.0),
                    rgba(0x1a0428, 0.9),
                );
            }
        }
    });
}

/// Right of the vial: «Despertar» (an eye opening as it fills) or, with her mouth busy, «Lucidez»
/// (fangs that glow as the bite comes).
pub(super) fn draw_column(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    frame: Rect,
    hv: &HypnoView,
    time: f32,
    s: f32,
) {
    let level = hv.awake as f32 / 1000.0;
    let bite = hv.mode == 1;
    let x = frame.x + frame.w + 36.0 * s + 120.0 * s + 30.0 * s;
    let column = Rect::new(x, frame.y, 112.0 * s, frame.h);
    let mx = column.x + column.w * 0.5;
    let hot = level > 0.8;
    let rim = if hot { MINT } else { GOLD };
    soft(
        v,
        Rect::new(column.x, column.y + 14.0 * s, column.w, column.h),
        30.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    if hot {
        soft(
            v,
            column,
            30.0 * s,
            26.0 * s,
            rgba(MINT, 0.3 + 0.25 * wave(time, 2.4).abs()),
            true,
        );
    }
    let mash = (1.0 - hv.mash_ms as f32 / 160.0).clamp(0.0, 1.0);
    if mash > 0.0 {
        soft(v, column, 30.0 * s, 20.0 * s, rgba(MINT, 0.45 * mash), true);
    }
    panel(
        v,
        column,
        30.0 * s,
        Panel::velvet(0.95, 2.6 * s)
            .rim(rim, 1.0, 2.6 * s)
            .hairline(6.0 * s, 0.4),
    );
    let mut size = 17.0 * s;
    while size > 12.0 * s
        && text_width(font, Face::Title, &hv.awake_label, size) > column.w - 14.0 * s
    {
        size -= 0.5 * s;
    }
    text_centered(
        font,
        v,
        Face::Title,
        &hv.awake_label,
        mx,
        column.y + 16.0 * s,
        size,
        rgba(if hot { 0xd8fff0 } else { CREAM }, 1.0),
    );

    // The medallion: an eye, or fangs.
    let mc = [mx, column.y + 84.0 * s];
    let mr = 36.0 * s;
    panel(
        v,
        Rect::new(mc[0] - mr, mc[1] - mr, mr * 2.0, mr * 2.0),
        mr,
        Panel::fill(mix(0x1c1428, 0x0c3a2c, level), 0x0b0812, 1.0)
            .rim(rim, 1.0, 3.0 * s)
            .bevel(1.0),
    );
    if bite {
        // Two fangs, whiter and glowing as the bite comes.
        let glow = level * (0.7 + 0.3 * wave(time, 3.0).abs());
        halo(
            v,
            mc,
            14.0 * s,
            (10.0 + 14.0 * glow) * s,
            rgba(MINT, 0.5 * glow),
        );
        let lip = rgba(mix(0x8a5a6a, 0xffd8e0, level), 1.0);
        wire(
            v,
            [mc[0] - 22.0 * s, mc[1] - 8.0 * s],
            [mc[0] + 22.0 * s, mc[1] - 8.0 * s],
            4.0 * s,
            lip,
            lip,
            0.0,
            false,
        );
        for side in [-1.0f32, 1.0] {
            let bx = mc[0] + side * 9.0 * s;
            push_triangle(
                v,
                [
                    [bx - 6.0 * s, mc[1] - 7.0 * s],
                    [bx + 6.0 * s, mc[1] - 7.0 * s],
                    [bx, mc[1] + (6.0 + 14.0 * level) * s],
                ],
                rgba(mix(0xb8aab8, 0xffffff, level), 1.0),
            );
        }
    } else {
        // The eye opens as she wakes; while the trance holds it, it is heavy-lidded.
        let open = (0.15 + 0.85 * level).min(1.0);
        let (w, h) = (24.0 * s, 13.0 * s * open);
        let n = 14;
        for sign in [-1.0f32, 1.0] {
            for k in 0..n {
                let p = |t: f32| [mc[0] - w + 2.0 * w * t, mc[1] + sign * h * (t * PI).sin()];
                wire(
                    v,
                    p(k as f32 / n as f32),
                    p((k + 1) as f32 / n as f32),
                    2.6 * s,
                    rgba(0xe8f8f0, 1.0),
                    rgba(0xe8f8f0, 1.0),
                    0.0,
                    false,
                );
            }
        }
        if open > 0.3 {
            disc(v, mc, 6.0 * s * open, rgba(mix(0x6ac8a8, MINT, level), 1.0));
        }
        if hv.covered {
            sparkle(
                v,
                [mc[0] + 22.0 * s, mc[1] - 22.0 * s],
                9.0 * s,
                rgba(0xffe6a0, 0.9),
                0.8,
            );
        }
    }

    // The bar.
    let bar = Rect::new(
        mx - 24.0 * s,
        column.y + 140.0 * s,
        48.0 * s,
        column.h - 140.0 * s - 70.0 * s,
    );
    panel(
        v,
        bar,
        24.0 * s,
        Panel::fill(0x0c1a16, 0x06100c, 1.0).rim(GOLD_DEEP, 0.9, 1.6 * s),
    );
    let inner = grow(bar, -5.0 * s);
    let fill_h = inner.h * level.clamp(0.0, 1.0);
    if fill_h > 1.0 {
        let liquid = Rect::new(inner.x, inner.y + inner.h - fill_h, inner.w, fill_h);
        let top = mix(0x3a9a7a, 0xd8fff0, level);
        if hot {
            soft(
                v,
                liquid,
                inner.w * 0.5,
                14.0 * s,
                rgba(MINT, 0.5 * (0.6 + 0.4 * wave(time, 3.0).abs())),
                true,
            );
        }
        panel(
            v,
            liquid,
            inner.w * 0.5,
            Panel::fill(top, 0x0e4a3a, 1.0)
                .bevel(0.4)
                .sheen(time * 0.25),
        );
        // Bubbles of light rising in it.
        for i in 0..4 {
            let fi = i as f32;
            let phase = (time * (0.4 + 0.1 * fi) + fi * 0.27).fract();
            let y = liquid.y + liquid.h * (1.0 - phase);
            if y > liquid.y + 6.0 * s {
                disc(
                    v,
                    [liquid.x + liquid.w * (0.25 + 0.17 * fi), y],
                    (2.0 + fi % 2.0) * s,
                    rgba(0xe8fff6, 0.6 * (1.0 - phase)),
                );
            }
        }
    }
    // The plaque.
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
        Panel::fill(0x0c3a2c, 0x06140e, 1.0)
            .rim(rim, 1.0, 2.0 * s)
            .bevel(1.0),
    );
    let label = if bite && hot {
        "¡MUERDE!".to_owned()
    } else {
        format!("{}%", (level * 100.0).round() as i32)
    };
    let ls = if bite && hot { 17.0 * s } else { 22.0 * s };
    text_centered(
        font,
        v,
        Face::Ui,
        &label,
        mx,
        plaque.y + (42.0 * s - ls) * 0.5 - 2.0 * s,
        ls,
        rgba(INK, 1.0),
    );
}

/// On the plate: the amulet he sold her, worn (the ring pulls less).
pub(super) fn plate_badges(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    plate: Rect,
    hv: &HypnoView,
    time: f32,
    s: f32,
) {
    if !hv.amulet || hv.mode != 0 {
        return;
    }
    let label = "AMULETO DEL OJO · el anillo tira menos";
    let size = 16.0 * s;
    let w = text_width(font, Face::Title, label, size) + 30.0 * s;
    let r = Rect::new(
        plate.x + (plate.w - w) * 0.5,
        plate.y + plate.h - 40.0 * s,
        w,
        28.0 * s,
    );
    soft(
        v,
        r,
        15.0 * s,
        12.0 * s,
        rgba(0xffd25a, 0.25 + 0.1 * wave(time, 0.7)),
        true,
    );
    panel(
        v,
        r,
        15.0 * s,
        Panel::fill(0x3a2a08, 0x140e04, 1.0).rim(0xffd25a, 1.0, 1.6 * s),
    );
    text_centered(
        font,
        v,
        Face::Title,
        label,
        r.x + r.w * 0.5,
        r.y + 4.0 * s,
        size,
        rgba(0xfff2c8, 1.0),
    );
}

/// A tile's colours for its tone (1 violet: obey, 2 sea green: refuse, 3 gold: mock).
pub(super) fn tone(tone: u8) -> Option<u32> {
    match tone {
        1 => Some(0x9a3ad8),
        2 => Some(0x2aa88a),
        3 => Some(0xd8a43a),
        _ => None,
    }
}
