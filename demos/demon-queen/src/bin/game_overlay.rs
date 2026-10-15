//! The brothel girls' minigames on the intimate scenes' stage (`explore/play_scene.rs`), drawn in
//! code around the clip: the room's light in the colour of the posture, flashing with what happens;
//! over the picture her shout in a comic balloon, the stamps («¡RACHA ×3!», «¡TOC, TOC!») and the
//! copla; left of it, until the game starts, the card of how to play; under it the plate with her
//! words to pick (a fuse burning down over them) or the keys and her air; right of the vial, in
//! Noa's game, the wall: the neighbour's medallion and the noise climbing its bricks toward the
//! line where they knock. What the keys light up only changes light and colour, never a place.
use super::persona::*;
use super::scene_overlay::{hex3, picture_rect, SceneView, Uv};
use super::scene_overlay::{GameRules, GameView};
use super::ui_paint::*;
use super::*;

/// Good, bad and quiet.
const RIGHT: u32 = 0x3fbf76;
const WRONG: u32 = 0xd8344a;
const COLD: u32 = 0x7a9cff;

/// The room's light round the frame in the posture's colour, and an event's flash over everything.
#[allow(clippy::too_many_arguments)]
pub(super) fn room_light(
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    frame: Rect,
    game: &GameView,
    heat: f32,
    time: f32,
    s: f32,
) {
    let tint = hex3(game.tint);
    let breath = 0.82 + 0.18 * (time * 1.3).sin();
    soft(
        v,
        grow(frame, 24.0 * s),
        40.0 * s,
        (120.0 + 90.0 * heat) * s,
        rgba(tint, (0.08 + 0.14 * heat) * breath),
        false,
    );
    let pulse = game.pulse as f32 / 1000.0;
    if pulse > 0.01 {
        let flash = hex3(game.pulse_tint);
        push_rect(
            v,
            Rect::new(0.0, 0.0, width, height),
            rgba(flash, 0.09 * pulse),
        );
        soft(
            v,
            frame,
            28.0 * s,
            (70.0 + 110.0 * pulse) * s,
            rgba(flash, 0.5 * pulse),
            true,
        );
    }
    if let Some(hv) = &game.hypno {
        super::hipno_overlay::room(v, width, height, hv, time, s);
    }
    // Someone listening on the other side: the edges of the room go cold.
    if game.neighbour == 2 {
        let k = (game.neighbour_ms as f32 / 600.0).clamp(0.0, 1.0);
        let throb = 0.75 + 0.25 * wave(time, 0.9).abs();
        vignette(
            v,
            Rect::new(0.0, 0.0, width, height),
            0.0,
            220.0 * s,
            rgba(COLD, 0.22 * k * throb),
        );
    }
}

/// The light laid over the clip: the posture's colour from the edges, the flash, the cold when
/// they listen.
pub(super) fn picture_light(
    v: &mut Vec<MapVertex>,
    pic: Rect,
    game: &GameView,
    has_frame: bool,
    time: f32,
    s: f32,
) {
    if !has_frame {
        return;
    }
    let r = 16.0 * s;
    let tint = hex3(game.tint);
    vignette(
        v,
        pic,
        r,
        pic.w * 0.2,
        rgba(tint, 0.08 + 0.03 * (time * 1.7).sin()),
    );
    let pulse = game.pulse as f32 / 1000.0;
    if pulse > 0.01 {
        let flash = hex3(game.pulse_tint);
        panel(
            v,
            pic,
            r,
            Panel::fill(flash, mix(flash, 0xffffff, 0.3), 0.06 * pulse),
        );
        vignette(v, pic, r, pic.w * 0.26, rgba(flash, 0.28 * pulse));
    }
    if game.neighbour == 2 {
        let k = (game.neighbour_ms as f32 / 600.0).clamp(0.0, 1.0);
        vignette(v, pic, r, pic.w * 0.18, rgba(COLD, 0.2 * k));
    }
    if game.neighbour == 3 && game.neighbour_ms < 900 {
        let k = 1.0 - game.neighbour_ms as f32 / 900.0;
        panel(v, pic, r, Panel::fill(0x000000, 0x000000, 0.35 * k));
    }
}

/// Over the picture: her shout, a stamp, the copla.
pub(super) fn over_picture(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    game: &GameView,
    over: bool,
    time: f32,
    s: f32,
) {
    if over {
        return;
    }
    if let Some(shout) = &game.shout {
        draw_shout(
            font,
            v,
            pic,
            &shout.text,
            shout.wild,
            shout.age_ms as f32 / 1000.0,
            time,
            s,
        );
    }
    if let Some(verse) = &game.verse {
        let at = [pic.x + 26.0 * s, pic.y + pic.h - 30.0 * s];
        speech_box(
            font,
            v,
            at,
            pic.w - 52.0 * s,
            &format!("{} · {}", game.who, demon_queen_dungeon::i18n::tr("copla")),
            verse,
            hex3(game.accent),
            -0.015,
            1.0,
            s,
        );
    }
    if let Some(hv) = &game.hypno {
        super::hipno_overlay::over_picture(font, v, pic, hv, time, s);
    }
    if let Some((words, color, age)) = &game.stamp {
        draw_stamp(
            font,
            v,
            pic,
            words,
            hex3(*color),
            *age as f32 / 1000.0,
            time,
            s,
        );
    }
}

/// Out past the end and back (a pop).
fn pop(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0) - 1.0;
    let c = 1.6;
    1.0 + (c + 1.0) * x * x * x + c * x * x
}

/// Her shout: a yellow comic balloon over the top of the picture with a tail down to her, popping
/// in; wild (she is too hot to say it right), it shakes and sprouts spikes.
#[allow(clippy::too_many_arguments)]
fn draw_shout(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    words: &str,
    wild: bool,
    age: f32,
    time: f32,
    s: f32,
) {
    let k = pop(age / 0.3).max(0.05);
    let mut size = 40.0 * s;
    while size > 26.0 * s && text_width(font, Face::Title, words, size) > pic.w - 140.0 * s {
        size -= 1.0 * s;
    }
    let tw = text_width(font, Face::Title, words, size);
    let c = [pic.x + pic.w * 0.5, pic.y + 58.0 * s];
    let w = (tw + 64.0 * s) * k;
    let h = (size + 36.0 * s) * k;
    let r = Rect::new(c[0] - w * 0.5, c[1] - h * 0.5, w, h);
    let angle = -0.05
        + if wild {
            0.07 * (time * 19.0).sin()
        } else {
            0.012 * (time * 2.0).sin()
        };
    turned(v, c, angle, |v| {
        if wild {
            // Spikes out of its corners: she is shouting it.
            for (i, corner) in [
                [r.x, r.y],
                [r.x + r.w, r.y],
                [r.x, r.y + r.h],
                [r.x + r.w, r.y + r.h],
            ]
            .iter()
            .enumerate()
            {
                let spin = time * 3.0 + i as f32;
                burst(
                    v,
                    *corner,
                    10.0 * s * k,
                    34.0 * s * k,
                    7,
                    spin,
                    rgba(0xfff1b0, 1.0),
                    rgba(0x5a2600, 1.0),
                    4.0 * s,
                );
            }
        }
        soft(
            v,
            Rect::new(r.x + 10.0 * s, r.y + 14.0 * s, r.w, r.h),
            r.h * 0.5,
            12.0 * s,
            rgba(SHADOW, 0.65),
            false,
        );
        soft(
            v,
            r,
            r.h * 0.5,
            22.0 * s,
            rgba(0xffd25a, 0.45 + 0.15 * wave(time, 1.4)),
            true,
        );
        // The tail, down toward her.
        let tail_x = c[0] - w * 0.12;
        push_triangle(
            v,
            [
                [tail_x - 26.0 * s * k, r.y + r.h - 6.0 * s],
                [tail_x + 16.0 * s * k, r.y + r.h - 6.0 * s],
                [tail_x - 40.0 * s * k, r.y + r.h + 34.0 * s * k],
            ],
            rgba(0x5a2600, 1.0),
        );
        panel(
            v,
            grow(r, 5.0 * s),
            r.h * 0.5 + 5.0 * s,
            Panel::fill(0x5a2600, 0x3a1600, 1.0),
        );
        push_triangle(
            v,
            [
                [tail_x - 20.0 * s * k, r.y + r.h - 8.0 * s],
                [tail_x + 10.0 * s * k, r.y + r.h - 8.0 * s],
                [tail_x - 32.0 * s * k, r.y + r.h + 24.0 * s * k],
            ],
            rgba(0xffd860, 1.0),
        );
        panel(
            v,
            r,
            r.h * 0.5,
            Panel::fill(0xfff6c8, 0xffcc4a, 1.0).bevel(0.5),
        );
        if k > 0.6 {
            let size = size * k;
            let tw = text_width(font, Face::Title, words, size);
            text_outline(
                font,
                v,
                Face::Title,
                words,
                c[0] - tw * 0.5,
                c[1] - size * 0.62,
                size,
                rgba(0x3a1300, 1.0),
                rgba(0xfff6c8, 0.7),
            );
        }
    });
}

/// A stamp slammed across the picture: big words on a burst of their colour, tilted, punching in
/// and fading after a moment.
#[allow(clippy::too_many_arguments)]
fn draw_stamp(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    words: &str,
    color: u32,
    age: f32,
    time: f32,
    s: f32,
) {
    let life = 1.7;
    if age > life {
        return;
    }
    let k = pop(age / 0.22);
    let fade = 1.0 - ((age - (life - 0.45)) / 0.45).clamp(0.0, 1.0);
    let c = [pic.x + pic.w * 0.5, pic.y + pic.h - 34.0 * s];
    let size = 48.0 * s * (0.6 + 0.4 * k);
    faded(v, fade, |v| {
        turned(v, c, -0.09, |v| {
            halo(v, c, size * 1.6, 30.0 * s, rgba(color, 0.3));
            comic_stamp(
                font,
                v,
                Face::Display,
                words,
                c,
                size * 1.25,
                color,
                true,
                (age / 0.3).min(1.0),
                time,
            );
        });
    });
}

/// How to play, on a card where the lines will go: her face and the game's name, then a row for
/// each thing to know (a key or a word, and what it does). It slides in and fades out.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_rules(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    frame: Rect,
    game: &GameView,
    rules: &GameRules,
    face_uv: Option<Uv>,
    time: f32,
    s: f32,
) {
    let right = frame.x - 36.0 * s;
    let width = (right - 54.0 * s).min(530.0 * s);
    if width < 260.0 * s {
        return;
    }
    let age = rules.age_ms as f32 / 1000.0;
    let a = entrance(age, 0.0, 0.45) * (rules.left_ms as f32 / 600.0).clamp(0.0, 1.0);
    let accent = hex3(game.accent);
    let key_w = rules
        .rows
        .iter()
        .map(|(key, _)| text_width(font, Face::Title, key, 21.0 * s) + 26.0 * s)
        .fold(54.0 * s, f32::max);
    let text_x0 = 30.0 * s + key_w + 18.0 * s;
    let row_lines: Vec<Vec<String>> = rules
        .rows
        .iter()
        .map(|(_, what)| wrap(font, Face::Body, what, 22.5 * s, width - text_x0 - 28.0 * s))
        .collect();
    let rows_h: f32 = row_lines
        .iter()
        .map(|l| (l.len() as f32 * 22.5 * s * 1.36).max(48.0 * s) + 18.0 * s)
        .sum();
    let height = (34.0 + 104.0 + 34.0) * s + rows_h + 40.0 * s;
    let card = Rect::new(
        right - width,
        frame.y + 10.0 * s,
        width,
        height.min(frame.h - 20.0 * s),
    );
    faded(v, a, |v| {
        shifted(
            v,
            [-(1.0 - entrance(age, 0.0, 0.45)) * 120.0 * s, 0.0],
            |v| {
                soft(
                    v,
                    Rect::new(card.x, card.y + 18.0 * s, card.w, card.h),
                    26.0 * s,
                    26.0 * s,
                    rgba(SHADOW, 0.8),
                    false,
                );
                soft(
                    v,
                    card,
                    26.0 * s,
                    22.0 * s,
                    rgba(accent, 0.28 + 0.08 * wave(time, 0.5)),
                    true,
                );
                panel(
                    v,
                    card,
                    24.0 * s,
                    Panel {
                        top: rgba(mix(VELVET_TOP, accent, 0.16), 0.97),
                        bottom: rgba(VELVET_BOTTOM, 0.98),
                        ..Panel::velvet(1.0, 3.0 * s)
                    }
                    .hairline(9.0 * s, 0.45),
                );
                for (at, sx, sy, horn) in [
                    ([card.x, card.y], 1.0, 1.0, true),
                    ([card.x + card.w, card.y], -1.0, 1.0, true),
                    ([card.x, card.y + card.h], 1.0, -1.0, false),
                    ([card.x + card.w, card.y + card.h], -1.0, -1.0, false),
                ] {
                    corner(v, at, sx, sy, s, accent, horn, 1.0);
                }
                // Her face in a ring, the game's name beside it.
                let fr = Rect::new(card.x + 30.0 * s, card.y + 34.0 * s, 104.0 * s, 104.0 * s);
                soft(v, fr, 52.0 * s, 16.0 * s, rgba(accent, 0.45), true);
                panel(
                    v,
                    grow(fr, 3.5 * s),
                    55.0 * s,
                    Panel::fill(mix(0x1c0f28, accent, 0.3), 0x0c0612, 1.0).rim(GOLD, 1.0, 2.4 * s),
                );
                match face_uv {
                    Some(uv) => face(v, fr, 52.0 * s, uv, 1.0),
                    None => gem(
                        v,
                        [fr.x + fr.w * 0.5, fr.y + fr.h * 0.5],
                        22.0 * s,
                        28.0 * s,
                        accent,
                        1.0,
                        0.0,
                    ),
                }
                let tx = fr.x + fr.w + 22.0 * s;
                let room = card.x + card.w - 26.0 * s - tx;
                text_shadow(
                    font,
                    v,
                    Face::Italic,
                    "Cómo se juega",
                    tx,
                    fr.y + 4.0 * s,
                    21.0 * s,
                    rgba(mix(accent, 0xffffff, 0.45), 1.0),
                );
                let mut size = 36.0 * s;
                while size > 24.0 * s && wrap(font, Face::Title, &rules.title, size, room).len() > 2
                {
                    size -= 1.0 * s;
                }
                for (i, line) in wrap(font, Face::Title, &rules.title, size, room)
                    .iter()
                    .take(2)
                    .enumerate()
                {
                    shout(
                        font,
                        v,
                        Face::Title,
                        line,
                        tx,
                        fr.y + 34.0 * s + i as f32 * size * 1.08,
                        size,
                        0xffffff,
                        mix(accent, 0x000000, 0.5),
                        1.0,
                    );
                }
                let mut y = fr.y + fr.h + 34.0 * s;
                rule(
                    v,
                    [card.x + card.w * 0.5, y - 12.0 * s],
                    card.w * 0.34,
                    accent,
                    s,
                    true,
                    1.0,
                );
                for (k, ((key, _), lines)) in rules.rows.iter().zip(&row_lines).enumerate() {
                    let appear = entrance(age, 0.25 + k as f32 * 0.12, 0.35);
                    let size = 22.5 * s;
                    let kw = key_w;
                    let text_x = card.x + text_x0;
                    let row_h = (lines.len() as f32 * size * 1.36).max(48.0 * s);
                    if y + row_h > card.y + card.h - 20.0 * s {
                        break;
                    }
                    faded(v, appear, |v| {
                        shifted(v, [(1.0 - appear) * 30.0 * s, 0.0], |v| {
                            let cap = Rect::new(
                                card.x + 30.0 * s,
                                y + (row_h - 46.0 * s) * 0.5,
                                kw,
                                46.0 * s,
                            );
                            keycap(font, v, cap, key, 21.0 * s, false, false, accent, 1.0);
                            for (i, line) in lines.iter().enumerate() {
                                let ly = y
                                    + (row_h - lines.len() as f32 * size * 1.36) * 0.5
                                    + i as f32 * size * 1.36;
                                text_shadow(
                                    font,
                                    v,
                                    Face::Body,
                                    line,
                                    text_x,
                                    ly,
                                    size,
                                    rgba(INK, 1.0),
                                );
                            }
                        });
                    });
                    y += row_h + 18.0 * s;
                }
            },
        );
    });
}

/// The tiles under the picture: three in a row in the plate.
fn chip_rects(pic: Rect, frame: Rect, n: usize, s: f32) -> Vec<Rect> {
    let plate = Rect::new(pic.x, frame.y + frame.h + 22.0 * s, pic.w, 152.0 * s);
    let n = n.max(1);
    let gap = 16.0 * s;
    let w = (plate.w - 2.0 * 22.0 * s - gap * (n - 1) as f32) / n as f32;
    let h = 64.0 * s;
    (0..n)
        .map(|i| {
            Rect::new(
                plate.x + 22.0 * s + i as f32 * (w + gap),
                plate.y + 66.0 * s,
                w,
                h,
            )
        })
        .collect()
}

/// The tile under the pointer (the screen's logical 1080p).
pub(crate) fn chip_hit(width: f32, height: f32, n: usize, x: f32, y: f32) -> Option<usize> {
    let s = height / 1080.0;
    let pic = picture_rect(width, height);
    let frame = grow(pic, 12.0 * s);
    chip_rects(pic, frame, n, s)
        .iter()
        .position(|r| grow(*r, 6.0 * s).contains(x, y))
}

/// Under the picture: what is happening; her words to pick (the fuse over them, the streak at the
/// side) or the game's keys and her air.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_plate(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    view: &SceneView,
    game: &GameView,
    time: f32,
    s: f32,
) {
    let cx = pic.x + pic.w * 0.5;
    let plate = Rect::new(pic.x, frame.y + frame.h + 22.0 * s, pic.w, 152.0 * s);
    let accent = hex3(game.accent);
    soft(
        v,
        Rect::new(plate.x, plate.y + 12.0 * s, plate.w, plate.h),
        26.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    let urgent = game.chips_left.is_some_and(|l| l < 350) || game.air.is_some_and(|a| a < 250);
    if urgent {
        soft(
            v,
            plate,
            26.0 * s,
            18.0 * s,
            rgba(CRIMSON, 0.4 * (0.6 + 0.4 * wave(time, 2.6).abs())),
            true,
        );
    }
    panel(
        v,
        plate,
        26.0 * s,
        Panel::velvet(0.96, 2.6 * s).hairline(6.0 * s, 0.4),
    );
    for side in [plate.x, plate.x + plate.w] {
        gem(
            v,
            [side, plate.y + plate.h * 0.5],
            7.0 * s,
            11.0 * s,
            accent,
            1.0,
            5.0 * s,
        );
    }
    let mut phase_size = 23.0 * s;
    while phase_size > 17.0 * s
        && text_width(font, Face::Italic, &view.phase, phase_size) > plate.w - 2.0 * 190.0 * s
    {
        phase_size -= 0.5 * s;
    }
    text_centered(
        font,
        v,
        Face::Italic,
        &view.phase,
        cx,
        plate.y + 12.0 * s,
        phase_size,
        rgba(0xeadcf2, 1.0),
    );
    if !game.chips.is_empty() {
        draw_chips(font, v, pic, frame, game, time, s);
    } else if !game.keys.is_empty() {
        draw_keys_row(font, v, plate, game, accent, time, s);
    }
    // Esc, small in the plate's top corner.
    let esc = Rect::new(
        plate.x + plate.w - 70.0 * s,
        plate.y + 10.0 * s,
        50.0 * s,
        28.0 * s,
    );
    keycap(font, v, esc, "Esc", 14.0 * s, false, false, CRIMSON, 0.8);
    let ew = text_width(font, Face::Italic, &view.esc, 17.0 * s);
    text_shadow(
        font,
        v,
        Face::Italic,
        &view.esc,
        esc.x - 10.0 * s - ew,
        esc.y + 4.0 * s,
        17.0 * s,
        rgba(DIM, 1.0),
    );
    // The streak, a flame-coloured badge at the plate's left foot.
    if game.streak >= 2 {
        let hot = game.streak >= 3;
        let label = if hot {
            format!("RACHA ×{} · ×1,5", game.streak)
        } else {
            format!("racha {}", game.streak)
        };
        let size = 19.0 * s;
        let w = text_width(font, Face::Title, &label, size) + 30.0 * s;
        let r = Rect::new(plate.x + 20.0 * s, plate.y + 9.0 * s, w, 30.0 * s);
        let flame = if hot { 0xff8a3d } else { GOLD };
        if hot {
            soft(
                v,
                r,
                15.0 * s,
                14.0 * s,
                rgba(flame, 0.5 + 0.2 * wave(time, 2.0)),
                true,
            );
        }
        panel(
            v,
            r,
            15.0 * s,
            Panel::fill(mix(0x3a1408, flame, 0.5), 0x1a0806, 1.0).rim(flame, 1.0, 1.6 * s),
        );
        text_centered(
            font,
            v,
            Face::Title,
            &label,
            r.x + r.w * 0.5,
            r.y + 3.0 * s,
            size,
            rgba(0xfff2d6, 1.0),
        );
    }
    if let (Some(hv), true) = (&game.hypno, game.chips.is_empty()) {
        super::hipno_overlay::plate_badges(font, v, plate, hv, time, s);
    }
    // Quiet enough: her secret doubling it.
    if game.sneaky && game.kind == 1 {
        let label = "SIGILO ×1,5";
        let size = 19.0 * s;
        let w = text_width(font, Face::Title, label, size) + 30.0 * s;
        let r = Rect::new(plate.x + 20.0 * s, plate.y + 9.0 * s, w, 30.0 * s);
        soft(
            v,
            r,
            15.0 * s,
            14.0 * s,
            rgba(0x5affc8, 0.35 + 0.15 * wave(time, 1.2)),
            true,
        );
        panel(
            v,
            r,
            15.0 * s,
            Panel::fill(0x0c3a2c, 0x06140e, 1.0).rim(0x5affc8, 1.0, 1.6 * s),
        );
        text_centered(
            font,
            v,
            Face::Title,
            label,
            r.x + r.w * 0.5,
            r.y + 3.0 * s,
            size,
            rgba(0xd8fff0, 1.0),
        );
    }
}

/// Her words (or the copla's rhymes) on tilted tiles, each with its number, coming in one by one;
/// the fuse burning down over them.
fn draw_chips(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    game: &GameView,
    time: f32,
    s: f32,
) {
    let accent = hex3(game.accent);
    let rects = chip_rects(pic, frame, game.chips.len(), s);
    let age = game.chips_age as f32 / 1000.0;
    if let (Some(left), Some(first), Some(last)) = (game.chips_left, rects.first(), rects.last()) {
        let left = left as f32 / 1000.0;
        let fuse = Rect::new(
            first.x,
            first.y - 15.0 * s,
            last.x + last.w - first.x,
            7.0 * s,
        );
        panel(
            v,
            fuse,
            3.5 * s,
            Panel::fill(0x1a0d22, 0x120818, 0.95).rim(GOLD_DEEP, 0.8, 1.0 * s),
        );
        if left > 0.01 {
            let lit = Rect::new(fuse.x, fuse.y, fuse.w * left, fuse.h);
            panel(
                v,
                lit,
                3.5 * s,
                Panel::fill(mix(CRIMSON, GOLD_HI, left), mix(0x8a1030, GOLD, left), 1.0),
            );
            let flicker = 0.8 + 0.2 * wave(time, 9.0).abs();
            halo(
                v,
                [lit.x + lit.w, fuse.y + fuse.h * 0.5],
                4.0 * s,
                12.0 * s,
                rgba(0xffb84a, 0.7),
            );
            sparkle(
                v,
                [lit.x + lit.w, fuse.y + fuse.h * 0.5],
                11.0 * s * flicker,
                rgba(0xfff2c8, 1.0),
                0.8,
            );
        }
    }
    for (i, (r, chip)) in rects.iter().zip(&game.chips).enumerate() {
        let appear = entrance(age, i as f32 * 0.07, 0.25);
        let tone = super::hipno_overlay::tone(chip.tone);
        let (top, bottom, rim, glow) = match chip.state {
            1 => (mix(0x1d6a3c, RIGHT, 0.5), 0x0c2a18, 0xbfffd6, Some(RIGHT)),
            2 => (mix(0x5a1018, WRONG, 0.4), 0x1a0608, 0xff9aa8, Some(WRONG)),
            3 => (0x1c1420, 0x0e0a12, 0x6a5a74, None),
            _ => match tone {
                Some(t) => (
                    mix(0x2a0f2a, t, 0.42),
                    mix(0x0e040c, t, 0.14),
                    mix(GOLD, t, 0.4),
                    None,
                ),
                None => (
                    mix(0x3a0f2a, accent, 0.28),
                    mix(0x14040c, accent, 0.1),
                    GOLD,
                    None,
                ),
            },
        };
        let c = [r.x + r.w * 0.5, r.y + r.h * 0.5];
        faded(v, appear * if chip.state == 3 { 0.55 } else { 1.0 }, |v| {
            shifted(v, [0.0, (1.0 - appear) * 26.0 * s], |v| {
                turned(v, c, -0.035, |v| {
                    soft(
                        v,
                        Rect::new(r.x + 8.0 * s, r.y + 10.0 * s, r.w, r.h),
                        8.0 * s,
                        10.0 * s,
                        rgba(SHADOW, 0.7),
                        false,
                    );
                    if let Some(g) = glow {
                        soft(v, *r, 8.0 * s, 22.0 * s, rgba(g, 0.55), true);
                    }
                    panel(
                        v,
                        grow(*r, 3.5 * s),
                        9.0 * s,
                        Panel::fill(0x0b0508, 0x0b0508, 1.0),
                    );
                    panel(
                        v,
                        *r,
                        7.0 * s,
                        Panel::fill(top, bottom, 0.98)
                            .rim(rim, 1.0, 2.0 * s)
                            .bevel(0.6),
                    );
                    let cap = Rect::new(
                        r.x + 10.0 * s,
                        r.y + (r.h - 36.0 * s) * 0.5,
                        36.0 * s,
                        36.0 * s,
                    );
                    keycap(
                        font,
                        v,
                        cap,
                        &format!("{}", i + 1),
                        18.0 * s,
                        chip.state == 1,
                        chip.state == 3,
                        accent,
                        1.0,
                    );
                    let room = r.w - cap.w - 30.0 * s;
                    let tagged = !chip.tag.is_empty();
                    let mut size = if tagged { 22.0 * s } else { 27.0 * s };
                    while size > 14.0 * s && text_width(font, Face::Title, &chip.label, size) > room
                    {
                        size -= 0.5 * s;
                    }
                    let tw = text_width(font, Face::Title, &chip.label, size);
                    let tx = cap.x + cap.w + 8.0 * s + (room - tw) * 0.5;
                    let ty = if tagged {
                        r.y + r.h - size - 9.0 * s
                    } else {
                        r.y + (r.h - size) * 0.5 - 3.0 * s
                    };
                    if tagged {
                        let ts = 12.5 * s;
                        let tag_w = text_width(font, Face::Title, &chip.tag, ts);
                        let ink = mix(tone.unwrap_or(accent), 0xffffff, 0.45);
                        text(
                            font,
                            v,
                            Face::Title,
                            &chip.tag,
                            cap.x + cap.w + 8.0 * s + (room - tag_w) * 0.5,
                            r.y + 7.0 * s,
                            ts,
                            rgba(ink, if chip.state == 3 { 0.5 } else { 1.0 }),
                        );
                    }
                    text_outline(
                        font,
                        v,
                        Face::Title,
                        &chip.label,
                        tx,
                        ty,
                        size,
                        rgba(INK, 1.0),
                        rgba(0x0b0508, 0.85),
                    );
                });
            });
        });
        if chip.state == 1 {
            for k in 0..6 {
                let fk = k as f32;
                let phase = (time * 1.2 + fk * 0.17).fract();
                let x = r.x + r.w * (0.1 + 0.8 * (fk * 0.618).fract());
                let y = r.y - phase * 34.0 * s;
                sparkle(
                    v,
                    [x, y],
                    (4.0 + 5.0 * (1.0 - phase)) * s,
                    rgba(0xd8ffe8, 1.0 - phase),
                    0.8,
                );
            }
        }
    }
}

/// The game's keys in a row (a keycap and what it does), and under them her air while her mouth
/// is busy.
fn draw_keys_row(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    plate: Rect,
    game: &GameView,
    accent: u32,
    time: f32,
    s: f32,
) {
    let cap_h = 50.0 * s;
    let size = 20.0 * s;
    let items: Vec<(f32, f32)> = game
        .keys
        .iter()
        .map(|(key, what, _)| {
            let kw = (text_width(font, Face::Title, key, 21.0 * s) + 30.0 * s).max(52.0 * s);
            (kw, text_width(font, Face::Italic, what, size))
        })
        .collect();
    let gap = 28.0 * s;
    let total: f32 = items.iter().map(|(k, t)| k + 10.0 * s + t).sum::<f32>()
        + gap * (items.len().max(1) - 1) as f32;
    let mut x = plate.x + (plate.w - total) * 0.5;
    let y = plate.y + 48.0 * s;
    for ((key, what, lit), (kw, tw)) in game.keys.iter().zip(&items) {
        let cap = Rect::new(x, y, *kw, cap_h);
        if *lit {
            for i in 0..5 {
                let fi = i as f32;
                let phase = (time * (0.9 + 0.13 * fi) + fi * 0.29).fract();
                let px = cap.x + cap.w * (0.15 + 0.7 * (fi * 0.618).fract());
                sparkle(
                    v,
                    [px, cap.y + 4.0 * s - phase * 26.0 * s],
                    (3.0 + 4.0 * (1.0 - phase)) * s,
                    rgba(0xffd0e4, 1.0 - phase),
                    0.7,
                );
            }
        }
        keycap(font, v, cap, key, 21.0 * s, *lit, false, accent, 1.0);
        text_shadow(
            font,
            v,
            Face::Italic,
            what,
            cap.x + cap.w + 10.0 * s,
            y + (cap_h - size) * 0.5 - 2.0 * s,
            size,
            rgba(if *lit { INK } else { CREAM }, 1.0),
        );
        x += kw + 10.0 * s + tw + gap;
    }
    // Her air: a bar of bubbles that empties while she is held deep.
    if let Some(air) = game.air {
        let air = air as f32 / 1000.0;
        let bar = Rect::new(
            plate.x + 200.0 * s,
            plate.y + 118.0 * s,
            plate.w - 400.0 * s,
            12.0 * s,
        );
        text_shadow(
            font,
            v,
            Face::Italic,
            "aire",
            bar.x - 50.0 * s,
            bar.y - 7.0 * s,
            19.0 * s,
            rgba(0xbfe4ff, 1.0),
        );
        panel(
            v,
            bar,
            6.0 * s,
            Panel::fill(0x0e1a2a, 0x081018, 0.95).rim(0x4a6a8a, 0.9, 1.2 * s),
        );
        if air > 0.01 {
            let low = air < 0.3;
            let blink = if low {
                0.6 + 0.4 * wave(time, 3.0).abs()
            } else {
                1.0
            };
            let fill = Rect::new(bar.x, bar.y, (bar.w * air).max(10.0 * s), bar.h);
            let (top, bottom) = if low {
                (0xff8a9a, 0xb02040)
            } else {
                (0xc8ecff, 0x3a8acc)
            };
            panel(
                v,
                fill,
                6.0 * s,
                Panel::fill(top, bottom, blink).sheen(time * 0.3),
            );
            for i in 0..5 {
                let fi = i as f32;
                let phase = (time * 0.7 + fi * 0.21).fract();
                let bx = fill.x + fill.w * (fi + 0.5) / 5.0;
                if bx < fill.x + fill.w - 4.0 * s {
                    arc(
                        v,
                        [bx, bar.y - phase * 18.0 * s],
                        (2.0 + fi % 2.0) * s,
                        1.2 * s,
                        0.0,
                        1.0,
                        rgba(0xdff4ff, 0.6 * (1.0 - phase)),
                        [0.0; 4],
                    );
                }
            }
        }
    }
}

/// The postures' strip under the plate (Noa's): each posture's name, the open ones lit, the one
/// on show in crimson, the closed ones behind a padlock; A and D at its ends.
fn posture_strip(pic: Rect, frame: Rect, s: f32) -> (Rect, Rect, Rect) {
    let y = frame.y + frame.h + 22.0 * s + 152.0 * s + 14.0 * s;
    let strip = Rect::new(pic.x + 20.0 * s, y, pic.w - 40.0 * s, 52.0 * s);
    let left = Rect::new(strip.x - 64.0 * s, y, 54.0 * s, 52.0 * s);
    let right = Rect::new(strip.x + strip.w + 10.0 * s, y, 54.0 * s, 52.0 * s);
    (strip, left, right)
}

/// A or D (−1, +1) under the pointer, or a posture's cell (its index, as `Err`).
pub(crate) fn pose_hit(
    width: f32,
    height: f32,
    n: usize,
    x: f32,
    y: f32,
) -> Option<Result<i32, usize>> {
    let s = height / 1080.0;
    let pic = picture_rect(width, height);
    let frame = grow(pic, 12.0 * s);
    let (strip, left, right) = posture_strip(pic, frame, s);
    if left.contains(x, y) {
        return Some(Ok(-1));
    }
    if right.contains(x, y) {
        return Some(Ok(1));
    }
    if strip.contains(x, y) && n > 0 {
        let cell = strip.w / n as f32;
        return Some(Err((((x - strip.x) / cell) as usize).min(n - 1)));
    }
    None
}

pub(super) fn draw_postures(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    pic: Rect,
    frame: Rect,
    game: &GameView,
    time: f32,
    s: f32,
) {
    if game.poses.is_empty() {
        return;
    }
    let (strip, left, right) = posture_strip(pic, frame, s);
    let accent = hex3(game.accent);
    turned(
        v,
        [strip.x + strip.w * 0.5, strip.y + strip.h * 0.5],
        -0.02,
        |v| {
            soft(
                v,
                Rect::new(strip.x + 8.0 * s, strip.y + 10.0 * s, strip.w, strip.h),
                8.0 * s,
                10.0 * s,
                rgba(SHADOW, 0.7),
                false,
            );
            panel(
                v,
                grow(strip, 3.5 * s),
                8.0 * s,
                Panel::fill(0x0b0508, 0x0b0508, 1.0),
            );
            panel(
                v,
                strip,
                6.0 * s,
                Panel::fill(0x2a0c22, 0x10040c, 0.97).rim(GOLD, 1.0, 2.0 * s),
            );
            let n = game.poses.len();
            let cell = strip.w / n as f32;
            for (i, (name, open)) in game.poses.iter().enumerate() {
                let lit = i == game.pose as usize;
                let r = Rect::new(
                    strip.x + cell * i as f32 + 5.0 * s,
                    strip.y + 6.0 * s,
                    cell - 10.0 * s,
                    strip.h - 12.0 * s,
                );
                if lit {
                    soft(
                        v,
                        r,
                        6.0 * s,
                        12.0 * s,
                        rgba(mix(CRIMSON, accent, 0.4), 0.45 + 0.1 * wave(time, 0.8)),
                        true,
                    );
                    panel(
                        v,
                        r,
                        5.0 * s,
                        Panel::fill(0xd02a52, 0x7a1030, 1.0).sheen(time * 0.3),
                    );
                }
                let size = 19.0 * s;
                let label = name.to_uppercase();
                let mut fitted = size;
                while fitted > 13.0 * s
                    && text_width(font, Face::Title, &label, fitted) > r.w - 12.0 * s
                {
                    fitted -= 0.5 * s;
                }
                let tw = text_width(font, Face::Title, &label, fitted);
                let ink = if lit {
                    0xffffff
                } else if *open {
                    CREAM
                } else {
                    0x6a5a74
                };
                text_outline(
                    font,
                    v,
                    Face::Title,
                    &label,
                    r.x + (r.w - tw) * 0.5,
                    r.y + (r.h - fitted) * 0.5 - 3.0 * s,
                    fitted,
                    rgba(ink, 1.0),
                    rgba(0x0b0508, 0.85),
                );
                if !*open {
                    padlock(
                        v,
                        [r.x + r.w - 14.0 * s, r.y + 10.0 * s],
                        18.0 * s,
                        0x8a7a96,
                        1.0,
                    );
                }
            }
        },
    );
    let alpha = if game.pose_active { 1.0 } else { 0.45 };
    keycap(
        font,
        v,
        left,
        "A",
        20.0 * s,
        false,
        !game.pose_active,
        CRIMSON,
        alpha,
    );
    keycap(
        font,
        v,
        right,
        "D",
        20.0 * s,
        false,
        !game.pose_active,
        CRIMSON,
        alpha,
    );
}

/// Noa's wall, right of the vial: a column of bricks the noise climbs (green, amber, red), the
/// line where they knock (lower and blue while someone listens), the neighbour's medallion at its
/// head (asleep, stirring, an ear pressed to the wall, a fist) and sound rings when it is loud.
pub(super) fn draw_wall(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    frame: Rect,
    game: &GameView,
    time: f32,
    s: f32,
) {
    if let Some(hv) = &game.hypno {
        super::hipno_overlay::draw_column(font, v, frame, hv, time, s);
        return;
    }
    let Some(noise) = game.noise else {
        return;
    };
    let noise = noise as f32 / 1000.0;
    let limit = game.limit as f32 / 1000.0;
    let listening = game.neighbour == 2;
    let knocking = game.neighbour == 3 && game.neighbour_ms < 1400;
    let x = frame.x + frame.w + 36.0 * s + 120.0 * s + 30.0 * s;
    let column = Rect::new(x, frame.y, 112.0 * s, frame.h);
    let mx = column.x + column.w * 0.5;
    let rim = if listening { COLD } else { GOLD };
    soft(
        v,
        Rect::new(column.x, column.y + 14.0 * s, column.w, column.h),
        30.0 * s,
        20.0 * s,
        rgba(SHADOW, 0.75),
        false,
    );
    if listening {
        soft(
            v,
            column,
            30.0 * s,
            24.0 * s,
            rgba(COLD, 0.35 + 0.15 * wave(time, 0.9).abs()),
            true,
        );
    }
    if knocking {
        soft(v, column, 30.0 * s, 30.0 * s, rgba(0xff9a3a, 0.55), true);
    }
    panel(
        v,
        column,
        30.0 * s,
        Panel::velvet(0.95, 2.6 * s)
            .rim(rim, 1.0, 2.6 * s)
            .hairline(6.0 * s, 0.4),
    );
    text_centered(
        font,
        v,
        Face::Title,
        "PARED",
        mx,
        column.y + 16.0 * s,
        19.0 * s,
        rgba(if listening { 0xcfe0ff } else { CREAM }, 1.0),
    );

    // The neighbour.
    let mc = [mx, column.y + 84.0 * s];
    let mr = 36.0 * s;
    let (fill_top, fill_bottom) = match game.neighbour {
        2 => (0x1c2c5a, 0x0a1022),
        3 => (0x5a2a0a, 0x1a0a04),
        1 => (0x2c1c3a, 0x100818),
        _ => (0x1c1428, 0x0b0812),
    };
    panel(
        v,
        Rect::new(mc[0] - mr, mc[1] - mr, mr * 2.0, mr * 2.0),
        mr,
        Panel::fill(fill_top, fill_bottom, 1.0)
            .rim(rim, 1.0, 3.0 * s)
            .bevel(1.0),
    );
    match game.neighbour {
        0 => {
            // Asleep: a shut eye, and z's drifting up.
            arc(
                v,
                [mc[0], mc[1] - 4.0 * s],
                14.0 * s,
                2.6 * s,
                0.25,
                0.5,
                rgba(0xd8c8e8, 1.0),
                [0.0; 4],
            );
            for k in 0..3 {
                let phase = (time * 0.35 + k as f32 / 3.0).fract();
                let zx = mc[0] + 10.0 * s + phase * 22.0 * s;
                let zy = mc[1] - 10.0 * s - phase * 40.0 * s;
                text(
                    font,
                    v,
                    Face::Title,
                    "z",
                    zx,
                    zy,
                    (15.0 + 10.0 * phase) * s,
                    rgba(0xd8c8e8, 0.95 * (1.0 - phase)),
                );
            }
        }
        1 => {
            // Stirring: an eye half open, looking about.
            let look = (time * 1.6).sin() * 5.0 * s;
            arc(
                v,
                mc,
                15.0 * s,
                2.4 * s,
                0.75,
                0.5,
                rgba(0xe8dcf0, 1.0),
                [0.0; 4],
            );
            disc(
                v,
                [mc[0] + look, mc[1] + 3.0 * s],
                5.0 * s,
                rgba(0xe8dcf0, 1.0),
            );
        }
        2 => {
            // Listening: an ear pressed to the wall, and the waves it catches.
            arc(
                v,
                mc,
                16.0 * s,
                4.0 * s,
                0.05,
                0.6,
                rgba(0xcfe0ff, 1.0),
                [0.0; 4],
            );
            arc(
                v,
                [mc[0] + 2.0 * s, mc[1] + 1.0 * s],
                8.0 * s,
                3.0 * s,
                0.1,
                0.5,
                rgba(0xcfe0ff, 0.9),
                [0.0; 4],
            );
            for k in 0..3 {
                let phase = (time * 0.9 + k as f32 / 3.0).fract();
                arc(
                    v,
                    [mc[0] - 26.0 * s, mc[1]],
                    (10.0 + phase * 26.0) * s,
                    2.0 * s,
                    0.62,
                    0.26,
                    rgba(COLD, 0.8 * (1.0 - phase)),
                    [0.0; 4],
                );
            }
        }
        _ => {
            // Knocking: the words, shaking.
            let jolt = (time * 40.0).sin() * 2.5 * s;
            let tw = text_width(font, Face::Title, "TOC", 24.0 * s);
            shout(
                font,
                v,
                Face::Title,
                "TOC",
                mc[0] - tw * 0.5 + jolt,
                mc[1] - 15.0 * s,
                24.0 * s,
                0xffe0b0,
                0x7a2a00,
                1.0,
            );
        }
    }

    // The bricks the noise climbs.
    let bar = Rect::new(
        mx - 24.0 * s,
        column.y + 140.0 * s,
        48.0 * s,
        column.h - 140.0 * s - 70.0 * s,
    );
    panel(
        v,
        bar,
        8.0 * s,
        Panel::fill(0x14080e, 0x0a0408, 1.0).rim(GOLD_DEEP, 0.9, 1.6 * s),
    );
    let inner = grow(bar, -4.0 * s);
    let fill_h = inner.h * noise.clamp(0.0, 1.0);
    if fill_h > 1.0 {
        let liquid = Rect::new(inner.x, inner.y + inner.h - fill_h, inner.w, fill_h);
        let hot = (noise / limit.max(0.05)).clamp(0.0, 1.0);
        let top = if hot < 0.5 {
            mix(0x6af0b8, 0xffd25a, hot * 2.0)
        } else {
            mix(0xffd25a, 0xff3a4a, (hot - 0.5) * 2.0)
        };
        let near = ((hot - 0.75) / 0.25).clamp(0.0, 1.0);
        if near > 0.0 {
            soft(
                v,
                liquid,
                6.0 * s,
                14.0 * s,
                rgba(0xff3a4a, 0.5 * near * (0.6 + 0.4 * wave(time, 4.0).abs())),
                true,
            );
        }
        panel(
            v,
            liquid,
            6.0 * s,
            Panel::fill(top, mix(top, 0x000000, 0.45), 1.0).bevel(0.4),
        );
    }
    // Mortar: rows of bricks, staggered.
    let row = 22.0 * s;
    let mut y = inner.y + inner.h - row;
    let mut odd = false;
    while y > inner.y + 2.0 * s {
        wire(
            v,
            [inner.x, y],
            [inner.x + inner.w, y],
            1.6 * s,
            rgba(0x05020a, 0.55),
            rgba(0x05020a, 0.55),
            0.0,
            false,
        );
        let jx = inner.x + inner.w * if odd { 0.3 } else { 0.7 };
        wire(
            v,
            [jx, y],
            [jx, y + row],
            1.6 * s,
            rgba(0x05020a, 0.5),
            rgba(0x05020a, 0.5),
            0.0,
            false,
        );
        odd = !odd;
        y -= row;
    }
    // The line where they knock.
    let ly = inner.y + inner.h - inner.h * limit.clamp(0.0, 1.0);
    let line_c = if listening { COLD } else { 0xff5a5a };
    halo(v, [mx, ly], 10.0 * s, 10.0 * s, rgba(line_c, 0.5));
    wire(
        v,
        [bar.x - 12.0 * s, ly],
        [bar.x + bar.w + 12.0 * s, ly],
        3.0 * s,
        rgba(line_c, 1.0),
        rgba(line_c, 1.0),
        4.0 * s,
        false,
    );
    gem(
        v,
        [bar.x - 16.0 * s, ly],
        5.0 * s,
        8.0 * s,
        if listening { 0x7a9cff } else { CRIMSON },
        1.0,
        4.0 * s,
    );
    // Loud: rings of sound beating against the wall.
    if noise > 0.35 {
        let loud = ((noise - 0.35) / 0.65).clamp(0.0, 1.0);
        for k in 0..3 {
            let phase = (time * (1.2 + loud) + k as f32 / 3.0).fract();
            arc(
                v,
                [column.x - 6.0 * s, inner.y + inner.h - fill_h],
                (8.0 + phase * 30.0) * s,
                2.2 * s,
                0.12,
                0.26,
                rgba(0xffe0c0, 0.7 * loud * (1.0 - phase)),
                [0.0; 4],
            );
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
        Panel::fill(0x2c1640, 0x10081a, 1.0)
            .rim(rim, 1.0, 2.0 * s)
            .bevel(1.0),
    );
    let label = format!("{}%", (noise * 100.0).round() as i32);
    text_centered(
        font,
        v,
        Face::Ui,
        &label,
        mx,
        plaque.y + 8.0 * s,
        22.0 * s,
        rgba(INK, 1.0),
    );
}
