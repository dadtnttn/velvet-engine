//! Paints a loading screen onto the game's 1920×1080 canvas: the 480×270 art
//! at exactly 4× (nearest sampling, so every art pixel is a clean 4×4 block)
//! with the loading HUD over it (title chip, counter, tip, bar and hint).
use super::pixel::Surface;
use super::Loader;
use crate::render::{Canvas, Fonts};

const SCALE: usize = 4;
const GOLD: u32 = 0xffffd36a;
const GOLD_DEEP: u32 = 0xffc9822a;
const INK: u32 = 0xfff4ecff;
const MUTED: u32 = 0xffb9a9cf;
const VIOLET: u32 = 0xffa46cff;
const EDGE: u32 = 0x73ffd36a;

/// The art, 4× nearest, filling the canvas.
pub fn upscale(canvas: &mut Canvas, frame: &Surface) {
    let cw = canvas.w;
    for y in 0..frame.h {
        for x in 0..frame.w {
            let i = (y * frame.w + x) * 4;
            let colour = 0xff00_0000
                | (frame.data[i] as u32) << 16
                | (frame.data[i + 1] as u32) << 8
                | frame.data[i + 2] as u32;
            for dy in 0..SCALE {
                let row = (y * SCALE + dy) * cw + x * SCALE;
                if row + SCALE <= canvas.pixels.len() {
                    canvas.pixels[row..row + SCALE].fill(colour);
                }
            }
        }
    }
}

fn panel(canvas: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    canvas.round_rect(x, y, w, h, 12, 0xd10c0714, 0xd10c0714);
    canvas.round_outline(x, y, w, h, 12, 2.0, EDGE);
}

/// Text with a hard dark shadow, like the pixel font's.
fn shadowed(
    canvas: &mut Canvas,
    fonts: &Fonts,
    text: &str,
    x: i32,
    y: i32,
    size: f32,
    colour: u32,
    shadow: u32,
    bold: bool,
) {
    let d = (size * 0.06).round().max(2.0) as i32;
    canvas.text(fonts, text, x + d, y + d, size, shadow, bold);
    canvas.text(fonts, text, x, y, size, colour, bold);
}

/// Paint the current frame and HUD. `fade` (0..1) dims everything to black.
pub fn paint(canvas: &mut Canvas, fonts: &Fonts, loader: &mut Loader, fade: f32) {
    let (w, h) = (canvas.w as i32, canvas.h as i32);
    upscale(canvas, loader.frame());

    // Title chip, top left.
    let kicker = loader.kicker().to_uppercase();
    let title = loader.title();
    let subtitle = loader.subtitle();
    let (title_size, sub_size, kick_size) = (44.0, 23.0, 16.0);
    let chip_w = (fonts
        .text_width(title, title_size, true)
        .max(fonts.text_width(subtitle, sub_size, false))
        + 56)
        .max(300);
    panel(canvas, 32, 32, chip_w, 132);
    canvas.text(fonts, &kicker, 54, 46, kick_size, VIOLET, true);
    shadowed(
        canvas, fonts, title, 54, 68, title_size, GOLD, 0xff3a0c20, true,
    );
    canvas.text(fonts, subtitle, 54, 124, sub_size, MUTED, false);

    // Counter, top right; it pops when it goes up.
    let label = loader.counter_label().to_uppercase();
    let bump = loader.counter_bump() as f32;
    let value = loader.counter().to_string();
    let value_size = 64.0 * (1.0 + 0.3 * bump);
    let label_w = fonts.text_width(&label, 18.0, true);
    let value_w = fonts.text_width(&value, value_size, true);
    let box_w = (label_w.max(value_w) + 56).max(150);
    let box_x = w - 32 - box_w;
    panel(canvas, box_x, 32, box_w, 132);
    canvas.text(
        fonts,
        &label,
        box_x + box_w - 28 - label_w,
        46,
        18.0,
        MUTED,
        true,
    );
    let value_colour = if bump > 0.0 { GOLD } else { INK };
    shadowed(
        canvas,
        fonts,
        &value,
        box_x + box_w - 28 - value_w,
        76 - (14.0 * bump) as i32,
        value_size,
        value_colour,
        0xff3a0c20,
        true,
    );

    // Tip, bottom left, on a violet rule.
    let tip = loader.tip();
    let (tip_w, tip_size) = (944, 27.0);
    let lines = canvas.wrap(fonts, tip, tip_w - 60, tip_size, false);
    let tip_h = 40 + 24 + lines.len() as i32 * 36;
    let tip_y = h - 32 - tip_h;
    canvas.round_rect(32, tip_y, tip_w, tip_h, 10, 0xd10c0714, 0xd10c0714);
    canvas.rect(32, tip_y + 6, 8, tip_h - 12, VIOLET);
    canvas.text(fonts, "CONSEJO", 62, tip_y + 14, 17.0, VIOLET, true);
    for (i, line) in lines.iter().enumerate() {
        canvas.text(
            fonts,
            line,
            62,
            tip_y + 42 + i as i32 * 36,
            tip_size,
            INK,
            false,
        );
    }

    // "CARGANDO..." with hopping dots, the bar, and the hint, bottom right.
    let right = w - 40;
    let done = loader.done();
    let now_text = if done { "¡LISTO!" } else { "CARGANDO" };
    let now_size = 76.0;
    let now_colour = if done { 0xffffffff } else { GOLD };
    let dots_w = if done {
        0
    } else {
        fonts.text_width("...", now_size, true)
    };
    let now_w = fonts.text_width(now_text, now_size, true);
    let now_x = right - now_w - dots_w;
    let bar_w = 600;
    let bar_h = 28;
    let hint_y = h - 32 - 26;
    let bar_y = hint_y - 18 - bar_h;
    let now_y = bar_y - 22 - 84;
    // A hard outline in dark red, then the fill.
    for (dx, dy) in [(-3, 0), (3, 0), (0, -3), (0, 3), (5, 5)] {
        canvas.text(
            fonts,
            now_text,
            now_x + dx,
            now_y + dy,
            now_size,
            if (dx, dy) == (5, 5) {
                0xff1a0610
            } else {
                0xff5a1420
            },
            true,
        );
    }
    canvas.text(fonts, now_text, now_x, now_y, now_size, now_colour, true);
    if !done {
        let dot_w = fonts.text_width(".", now_size, true);
        let clock = loader.clock();
        for i in 0..3 {
            // Each dot hops in turn.
            let phase = ((clock / 1.2) - i as f64 * (0.2 / 1.2)).rem_euclid(1.0);
            let hop = if phase > 0.3 && phase < 0.55 { -12 } else { 0 };
            let dx = now_x + now_w + i * dot_w;
            for (ox, oy) in [(-3, 0), (3, 0), (0, -3), (0, 3), (5, 5)] {
                canvas.text(
                    fonts,
                    ".",
                    dx + ox,
                    now_y + hop + oy,
                    now_size,
                    if (ox, oy) == (5, 5) {
                        0xff1a0610
                    } else {
                        0xff5a1420
                    },
                    true,
                );
            }
            canvas.text(fonts, ".", dx, now_y + hop, now_size, GOLD, true);
        }
    }
    let bar_x = right - bar_w;
    canvas.rect(bar_x, bar_y, bar_w, bar_h, 0xff12081c);
    canvas.round_outline(bar_x, bar_y, bar_w, bar_h, 3, 2.0, EDGE);
    let fill_w = ((bar_w - 8) as f64 * loader.progress()).round() as i32;
    if fill_w > 0 {
        let (fx, fy, fh) = (bar_x + 4, bar_y + 4, bar_h - 8);
        canvas.rect(fx, fy, fill_w, fh, GOLD);
        canvas.rect(fx, fy, fill_w, (fh * 3 / 10).max(1), 0xfffff0a8);
        canvas.rect(
            fx,
            fy + fh * 65 / 100,
            fill_w,
            fh - fh * 65 / 100,
            GOLD_DEEP,
        );
        // Segment ticks, like the original bar.
        let mut tx = fx + 20;
        while tx < fx + fill_w {
            canvas.rect(tx, fy, 4, fh, 0x59000000);
            tx += 24;
        }
    }
    let mash_key = if loader.mashing() { 0xfffff0a8 } else { INK };
    let hint_size = 22.0;
    let parts: [(&str, u32); 5] = [
        ("Machaca ", MUTED),
        ("ESPACIO", mash_key),
        (" o haz clic para ayudarla  ·  ", MUTED),
        ("← →", INK),
        (" cambiar", MUTED),
    ];
    let total: i32 = parts
        .iter()
        .map(|(t, _)| fonts.text_width(t, hint_size, false))
        .sum();
    let mut x = right - total;
    for (text, colour) in parts {
        canvas.text(fonts, text, x, hint_y, hint_size, colour, false);
        x += fonts.text_width(text, hint_size, false);
    }

    if fade < 1.0 {
        let alpha = ((1.0 - fade.clamp(0.0, 1.0)) * 255.0) as u32;
        canvas.rect(0, 0, w, h, alpha << 24);
    }
}
