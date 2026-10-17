//! Paints a loading screen onto the game's 1920×1080 canvas: the 480×270 art
//! at exactly 4× (nearest sampling, so every art pixel is a clean 4×4 block)
//! with the loading HUD over it in the game's Persona manner (see `paint`).
use super::pixel::Surface;
use super::Loader;
use crate::render::{Canvas, Fonts, StageFace};

const SCALE: usize = 4;
/// The art, 4× nearest, filling the canvas.
pub fn upscale(canvas: &mut Canvas, frame: &Surface) {
    let (cw, ch) = (canvas.pw, canvas.ph);
    if (cw, ch) == (frame.w * SCALE, frame.h * SCALE) {
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
        return;
    }
    // Another size: each of the canvas's pixels takes the art pixel it falls in.
    let columns: Vec<usize> = (0..cw)
        .map(|x| (x * frame.w / cw.max(1)).min(frame.w - 1))
        .collect();
    for y in 0..ch {
        let sy = (y * frame.h / ch.max(1)).min(frame.h - 1);
        let row = y * cw;
        for (x, &sx) in columns.iter().enumerate() {
            let i = (sy * frame.w + sx) * 4;
            canvas.pixels[row + x] = 0xff00_0000
                | (frame.data[i] as u32) << 16
                | (frame.data[i + 1] as u32) << 8
                | frame.data[i + 2] as u32;
        }
    }
}

/// The scene's light (its dominant colour, 0xRRGGBB), worked out once per screen.
fn scene_light(title: &'static str, frame: &Surface) -> u32 {
    use std::sync::Mutex;
    static LAST: Mutex<Option<(&'static str, u32)>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((t, light)) = *last {
        if t == title {
            return light;
        }
    }
    let c = crate::dominant_light_with(frame.w, frame.h, |x, y| {
        let i = (y * frame.w + x) * 4;
        [frame.data[i], frame.data[i + 1], frame.data[i + 2], 255]
    });
    let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    let light = (to(c[0]) << 16) | (to(c[1]) << 8) | to(c[2]);
    *last = Some((title, light));
    light
}

/// Paint the current frame and HUD in the game's Persona manner: the screen's name on a slanted
/// ink plate with a spine of the scene's own colour, the counter on a slanted card, the tip on an
/// ink plate with its tag, «CARGANDO» big in the display face with hopping dots over a slash of
/// the accent, a slanted ten-step bar and the keys on their caps. `fade` (0..1) dims to black.
pub fn paint(canvas: &mut Canvas, fonts: &Fonts, loader: &mut Loader, fade: f32) {
    use crate::menu_ui as ui;
    let (w, h) = (canvas.w as i32, canvas.h as i32);
    let title = loader.title();
    let light = scene_light(title, loader.frame());
    upscale(canvas, loader.frame());
    let accent = ui::accent(light);
    let deep = ui::deepen(accent, 0.66, 0.14);
    let bright = ui::mix(accent, 0xffffff, 0.45);
    let clock = loader.clock() as f32;
    let tr = |s: &str| demon_queen_dungeon::i18n::tr(s).into_owned();

    // A shade along the foot, so the tip and the bar read on any scene.
    for k in 0..8 {
        let y = h - 320 + k * 40;
        canvas.rect(0, y, w, 40, ((0x12 + k * 0x0b) as u32) << 24 | 0x07030c);
    }

    // ── The screen's name, top left.
    let subtitle = loader.subtitle();
    let title_size = ui::fit_size(fonts, title, 640, 46.0, true);
    let title_w = fonts
        .display_width(title, title_size)
        .max(fonts.text_width(subtitle, 21.0, false)) as f32;
    let plate_w = title_w + 96.0;
    ui::plate(canvas, 46.0, 52.0, plate_w, 112.0, 26.0, 0xb4000000);
    ui::plate(
        canvas,
        36.0,
        42.0,
        plate_w,
        112.0,
        26.0,
        ui::argb(0xf0, 0x0d0814),
    );
    ui::plate(
        canvas,
        36.0,
        42.0,
        18.0,
        112.0,
        26.0,
        ui::argb(0xff, accent),
    );
    ui::outline(
        canvas,
        &ui::plate_points(36.0, 42.0, plate_w, 112.0, 26.0),
        1.4,
        ui::argb(0x90, ui::mix(accent, 0xffffff, 0.4)),
    );
    let kicker = tr(&loader.kicker()).to_uppercase();
    ui::tag(canvas, fonts, 72, 22, &kicker, 14.0, ui::ABYSS, ui::BONE);
    canvas.stage_text(
        fonts,
        StageFace::Display,
        title,
        84 + 3,
        62 + 3,
        title_size,
        0xff000000,
    );
    canvas.stage_text(
        fonts,
        StageFace::Display,
        title,
        84,
        62,
        title_size,
        0xffffffff,
    );
    ui::text_mid(
        canvas,
        fonts,
        subtitle,
        86,
        132,
        21.0,
        ui::argb(0xff, ui::mix(accent, 0xffffff, 0.6)),
        false,
    );

    // ── The counter, top right: its label on a tag, the number big; it pops when it goes up.
    let label = tr(loader.counter_label()).to_uppercase();
    let bump = loader.counter_bump() as f32;
    let value = loader.counter().to_string();
    let value_size = 70.0 * (1.0 + 0.25 * bump);
    let card_w = (fonts.text_width(&label, 14.0, true) + 90).max(200) as f32;
    let cx = w as f32 - 44.0 - card_w;
    ui::plate(canvas, cx + 10.0, 52.0, card_w, 112.0, 22.0, 0xb4000000);
    ui::plate(
        canvas,
        cx,
        42.0,
        card_w,
        112.0,
        22.0,
        ui::argb(0xf0, 0x0d0814),
    );
    ui::plate(
        canvas,
        cx + card_w - 30.0,
        42.0,
        30.0,
        112.0,
        22.0,
        ui::argb(0xff, accent),
    );
    ui::halftone(
        canvas,
        (cx + card_w * 0.5, 98.0, card_w * 0.5, 56.0),
        12.0,
        2.6,
        (cx + card_w, 154.0),
        card_w * 0.7,
        ui::argb(0x26, ui::mix(accent, 0xffffff, 0.4)),
    );
    ui::tag(
        canvas,
        fonts,
        cx as i32 + 20,
        22,
        &label,
        14.0,
        ui::BONE,
        ui::ABYSS,
    );
    let vw = fonts.stage_width(StageFace::Display, &value, value_size);
    let vx = (cx + card_w * 0.5) as i32 - vw / 2 - 10;
    let vy = 60 - (12.0 * bump) as i32;
    canvas.stage_text(
        fonts,
        StageFace::Display,
        &value,
        vx + 4,
        vy + 4,
        value_size,
        0xff000000,
    );
    let value_ink = if bump > 0.0 {
        ui::argb(0xff, bright)
    } else {
        0xffffffff
    };
    canvas.stage_text(
        fonts,
        StageFace::Display,
        &value,
        vx,
        vy,
        value_size,
        value_ink,
    );

    // ── The tip, bottom left: an ink plate with the accent's spine and its tag.
    let tip = loader.tip();
    let (tip_w, tip_size) = (930, 26.0);
    let lines = canvas.wrap(fonts, tip, tip_w - 100, tip_size, false);
    let tip_h = 54 + lines.len() as i32 * 36;
    let tip_y = h - 48 - tip_h;
    let (tx, ty, tw, th) = (40.0, tip_y as f32, tip_w as f32, tip_h as f32);
    ui::plate(canvas, tx + 10.0, ty + 10.0, tw, th, 22.0, 0xb4000000);
    ui::plate(canvas, tx, ty, tw, th, 22.0, ui::argb(0xee, 0x0d0814));
    ui::plate(canvas, tx, ty, 12.0, th, 22.0, ui::argb(0xff, accent));
    ui::tag(
        canvas,
        fonts,
        76,
        tip_y - 18,
        &tr("CONSEJO"),
        14.0,
        accent,
        ui::ink_on(accent) & 0x00ff_ffff,
    );
    for (i, line) in lines.iter().enumerate() {
        canvas.text(
            fonts,
            line,
            80,
            tip_y + 30 + i as i32 * 36,
            tip_size,
            ui::INK,
            false,
        );
    }

    // ── «CARGANDO» with hopping dots (or «¡LISTO!»), the bar and the keys, bottom right.
    let right = w - 52;
    let done = loader.done();
    let now = tr(if done { "¡LISTO!" } else { "CARGANDO" });
    let now_size = ui::fit_size(fonts, &now, 620, 88.0, true);
    let dots_w = if done {
        0
    } else {
        fonts.stage_width(StageFace::Display, "...", now_size)
    };
    let now_w = fonts.stage_width(StageFace::Display, &now, now_size);
    let bar_w = 640;
    let bar_h = 30;
    let keys_y = h - 48 - 30;
    let bar_y = keys_y - 26 - bar_h;
    let now_y = bar_y - 22 - (now_size * 1.1) as i32;
    let now_x = right - now_w - dots_w;
    ui::plate(
        canvas,
        (now_x - 44) as f32,
        (now_y + 24) as f32,
        (now_w + dots_w + 88) as f32,
        now_size * 0.74,
        18.0,
        ui::argb(0xd8, deep),
    );
    let word_ink = if done {
        0xffffffff
    } else {
        ui::argb(0xff, bright)
    };
    canvas.stage_text(
        fonts,
        StageFace::Display,
        &now,
        now_x + 5,
        now_y + 5,
        now_size,
        0xff000000,
    );
    canvas.stage_text(
        fonts,
        StageFace::Display,
        &now,
        now_x,
        now_y,
        now_size,
        word_ink,
    );
    if !done {
        let dot_w = fonts.stage_width(StageFace::Display, ".", now_size);
        for i in 0..3 {
            let phase = ((clock / 1.2) - i as f32 * (0.2 / 1.2)).rem_euclid(1.0);
            let hop = if phase > 0.3 && phase < 0.55 { -12 } else { 0 };
            let dx = now_x + now_w + i * dot_w;
            canvas.stage_text(
                fonts,
                StageFace::Display,
                ".",
                dx + 5,
                now_y + hop + 5,
                now_size,
                0xff000000,
            );
            canvas.stage_text(
                fonts,
                StageFace::Display,
                ".",
                dx,
                now_y + hop,
                now_size,
                word_ink,
            );
        }
    }
    let bar_x = right - bar_w;
    ui::meter(
        canvas,
        bar_x,
        bar_y,
        bar_w,
        bar_h,
        loader.progress() as f32,
        ui::mix(accent, 0xffffff, 0.25),
    );
    let mash = if loader.mashing() {
        "¡ASÍ!"
    } else {
        "machaca para ayudarla"
    };
    let hints = [("ESPACIO · CLIC", mash), ("← →", "cambiar")];
    let hints_w: i32 = hints
        .iter()
        .map(|(k, t)| {
            (fonts.text_width(k, 15.0, true) + 24).max(36)
                + 10
                + fonts.text_width(t, 17.0, false)
                + 26
        })
        .sum::<i32>()
        - 26;
    // An ink plate under the keys, so they read on a pale scene.
    ui::plate(
        canvas,
        (right - hints_w - 30) as f32,
        (keys_y - 8) as f32,
        (hints_w + 48) as f32,
        46.0,
        14.0,
        0xd00d0814,
    );
    ui::key_hints(canvas, fonts, right - hints_w, keys_y, &hints);

    if fade < 1.0 {
        let alpha = ((1.0 - fade.clamp(0.0, 1.0)) * 255.0) as u32;
        canvas.rect(0, 0, w, h, alpha << 24);
    }
}
