//! The wardrobe (the dressing table's «Ropero de la Reina») and the achievements screen in the
//! game's Persona manner, as the shop and the stall are: the world dims under slashes of the
//! screen's colour with a halftone and speed lines; a big title thrown across the top; the outfits
//! as tall slanted cards with the Queen herself in each (her status cut-out), the one in focus lit
//! with a red pointer over it; the achievements as slanted tiles with a seal when earned, and her
//! counts on a plate at the side. What the keys light changes only light and colour, never a place
//! or a size. Authored at 1920×1080.
use super::persona::*;
use super::ui_paint::*;
use super::*;

type Uv = ([f32; 2], [f32; 2]);

const INK_DARK: u32 = 0x0e0816;
const BONE: u32 = 0xf6efe4;
const ROSE: u32 = 0xff5c9a;
const VIOLET: u32 = 0x9a5cff;
const POINTER: u32 = 0xe2334a;

fn rgb(c: [f32; 4]) -> u32 {
    let to = |v: f32| ((v.clamp(0.0, 1.0)) * 255.0).round() as u32;
    (to(c[0]) << 16) | (to(c[1]) << 8) | to(c[2])
}

fn slanted(x: f32, y: f32, w: f32, h: f32, lean: f32) -> [[f32; 2]; 4] {
    [[x + lean, y], [x + w + lean, y], [x + w - lean, y + h], [x - lean, y + h]]
}

fn fill(v: &mut Vec<MapVertex>, q: [[f32; 2]; 4], color: [f32; 4]) {
    push_triangle(v, [q[0], q[1], q[2]], color);
    push_triangle(v, [q[0], q[2], q[3]], color);
    push_outline_fringe(v, &q, color);
}

fn rim(v: &mut Vec<MapVertex>, q: [[f32; 2]; 4], width: f32, color: [f32; 4]) {
    for i in 0..4 {
        wire(v, q[i], q[(i + 1) % 4], width, color, color, 0.0, false);
    }
}

/// The world dimmed under the screen's slashes, a halftone and speed lines.
fn backdrop(v: &mut Vec<MapVertex>, width: f32, height: f32, accent: u32, deep: u32, time: f32) {
    push_rect(v, Rect::new(0.0, 0.0, width, height), rgba(0x05030a, 0.74));
    let angle = -0.2;
    let c = [width * 0.55, height * 0.52];
    slash(v, c, width * 2.2, height * 0.56, angle, rgba(mix(deep, accent, 0.35), 0.9), rgba(deep, 0.94));
    slash(v, [c[0], c[1] + height * 0.3], width * 2.2, 24.0, angle, rgba(0x05030a, 0.96), rgba(0x05030a, 0.96));
    slash(v, [c[0], c[1] - height * 0.29], width * 2.2, 6.0, angle, rgba(GOLD_HI, 0.85), rgba(GOLD, 0.85));
    slash(v, [c[0], c[1] - height * 0.33], width * 2.2, 46.0, angle, rgba(accent, 0.28), rgba(deep, 0.4));
    halftone(
        v,
        Rect::new(width * 0.55, 0.0, width * 0.45, height),
        26.0,
        12.0,
        [0.9, -0.7],
        rgba(accent, 0.12),
    );
    speed_lines(v, Rect::new(0.0, height * 0.16, width, height * 0.68), angle, 22, time, rgba(0xffffff, 0.06));
}

/// The big title thrown across the top, its line under it, and a tag with what the screen is.
fn heading(font: &FontAtlas, v: &mut Vec<MapVertex>, x: f32, y: f32, title: &str, tag: &str, accent: u32) {
    turned(v, [x, y], -0.04, |v| {
        shout(font, v, Face::Title, title, x, y, 64.0, 0xffffff, mix(accent, 0x000000, 0.45), 1.0);
        let tw = text_width(font, Face::Title, title, 64.0);
        wire(
            v,
            [x, y + 78.0],
            [x + tw + 80.0, y + 78.0],
            6.0,
            rgba(0xffffff, 0.95),
            rgba(0xffffff, 0.0),
            0.0,
            false,
        );
        let w = text_width(font, Face::Display, tag, 18.0) + 30.0;
        fill(v, slanted(x + 6.0, y + 94.0, w, 30.0, 8.0), rgba(BONE, 1.0));
        text(font, v, Face::Display, tag, x + 21.0, y + 97.0, 18.0, rgba(INK_DARK, 1.0));
    });
}

/// A row of keys at the foot, each on a cap with what it does.
fn key_row(font: &FontAtlas, v: &mut Vec<MapVertex>, x: f32, y: f32, keys: &[(&str, &str)], accent: u32) {
    let mut x = x;
    for (key, what) in keys {
        let cap = (text_width(font, Face::Title, key, 17.0) + 24.0).max(40.0);
        keycap(font, v, Rect::new(x, y, cap, 36.0), key, 17.0, false, false, accent, 1.0);
        text_shadow(font, v, Face::Italic, what, x + cap + 10.0, y + 7.0, 20.0, rgba(CREAM, 0.95));
        x += cap + 10.0 + text_width(font, Face::Italic, what, 20.0) + 30.0;
    }
}

/// The red pointer over the card in focus, bobbing.
fn pointer_down(v: &mut Vec<MapVertex>, tip: [f32; 2], time: f32) {
    let bob = (time * 5.0).sin() * 5.0;
    let tip = [tip[0], tip[1] + bob];
    let s = 26.0;
    push_triangle(
        v,
        [[tip[0], tip[1] + 4.0], [tip[0] - s * 0.7, tip[1] - s - 2.0], [tip[0] + s * 0.7, tip[1] - s - 2.0]],
        rgba(0xffffff, 1.0),
    );
    push_triangle(
        v,
        [tip, [tip[0] - s * 0.55, tip[1] - s], [tip[0] + s * 0.55, tip[1] - s]],
        rgba(POINTER, 1.0),
    );
}

/// The wardrobe: an outfit a card, the Queen in it.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_wardrobe(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    selection: u8,
    equipped: u8,
    view: &WardrobeView,
    faces: &[Option<Uv>],
    time: f32,
) {
    backdrop(v, width, height, ROSE, 0x2a0a26, time);
    let count = view.outfits.len().max(2);
    heading(
        font,
        v,
        110.0,
        46.0,
        "Ropero de la Reina",
        &format!("ELIGE UN ATUENDO · {count} ATUENDOS"),
        ROSE,
    );
    let looks = outfit_looks_n(count);
    let (gap, card_h, lean) = (46.0, 720.0, 22.0);
    let card_w = ((width - 260.0 - gap * (count as f32 - 1.0)) / count as f32).min(430.0);
    let total = card_w * count as f32 + gap * (count as f32 - 1.0);
    let x0 = (width - total) * 0.5;
    let y0 = 236.0;
    for (index, look) in looks.iter().enumerate().take(count) {
        let x = x0 + index as f32 * (card_w + gap);
        let selected = index as u8 == selection % count as u8;
        let worn = index as u8 == equipped % count as u8;
        let locked = view.locked.get(index).copied().unwrap_or(false);
        let accent = rgb(look.glow);
        let card = slanted(x, y0, card_w, card_h, lean);
        // Its shadow, its ground (the outfit's colour deepening down), its rim.
        fill(v, slanted(x + 14.0, y0 + 16.0, card_w, card_h, lean), rgba(0x000000, 0.6));
        if selected {
            soft(v, Rect::new(x, y0, card_w, card_h), 12.0, 30.0, rgba(accent, 0.45 + 0.12 * wave(time, 0.8)), true);
        }
        let top = if locked { 0x1a1420 } else { mix(rgb(look.dress_dark), 0x000000, if selected { 0.15 } else { 0.45 }) };
        push_triangle(v, [card[0], card[1], card[2]], rgba(top, 0.97));
        push_triangle(v, [card[0], card[2], card[3]], rgba(INK_DARK, 0.97));
        push_outline_fringe(v, &card, rgba(INK_DARK, 0.97));
        // A pool of light behind her.
        let cx = x + card_w * 0.5;
        halo(v, [cx, y0 + 300.0], card_w * 0.55, 90.0, rgba(accent, if locked { 0.05 } else if selected { 0.32 } else { 0.14 }));
        // The Queen in it (her cut-out), or the drawn figure until there is one.
        let stage_h = 520.0;
        match faces.get(index).copied().flatten() {
            Some(uv) => {
                let h = stage_h;
                let w = h * 432.0 / 738.0;
                let r = Rect::new(cx - w * 0.5, y0 + 18.0, w, h);
                let tint = if locked {
                    [0.08, 0.06, 0.1, 1.0]
                } else if selected {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.62, 0.58, 0.68, 1.0]
                };
                cutout(v, r, uv, tint);
            }
            None => draw_outfit_figure(v, cx, y0 + stage_h, stage_h - 40.0, index, look),
        }
        if locked {
            padlock(v, [cx, y0 + 300.0], 70.0, GOLD, 0.95);
        }
        // The bottom band: number, name, line, state.
        let band = slanted(x, y0 + card_h - 186.0, card_w, 186.0, lean * 186.0 / card_h);
        push_triangle(v, [band[0], band[1], band[2]], rgba(0x0b0610, 0.92));
        push_triangle(v, [band[0], band[2], band[3]], rgba(0x0b0610, 0.92));
        let (name, detail) = view
            .outfits
            .get(index)
            .map(|(n, d)| (n.as_str(), if d.is_empty() { look.detail } else { d.as_str() }))
            .unwrap_or((look.name, look.detail));
        let detail = if locked { "Se gana con un logro" } else { detail };
        let plate_w = text_width(font, Face::Title, name, 32.0).min(card_w - 70.0) + 36.0;
        let py = y0 + card_h - 166.0;
        fill(
            v,
            slanted(x + 30.0, py, plate_w, 48.0, 10.0),
            rgba(if selected { BONE } else { 0x2a2032 }, 1.0),
        );
        let size = 32.0f32.min(32.0 * (card_w - 70.0) / text_width(font, Face::Title, name, 32.0).max(1.0));
        text(font, v, Face::Title, name, x + 48.0, py + 6.0, size, rgba(if selected { INK_DARK } else { BONE }, 1.0));
        let lines = wrap(font, Face::Italic, detail, 20.0, card_w - 70.0);
        for (i, l) in lines.iter().take(2).enumerate() {
            text_shadow(font, v, Face::Italic, l, x + 34.0, py + 60.0 + i as f32 * 25.0, 20.0, rgba(CREAM, 0.9));
        }
        // State: worn, or ENTER to wear it (in focus), on a chip.
        let (chip, chip_fill, chip_ink) = if worn {
            ("EQUIPADO", accent, INK_DARK)
        } else if selected && !locked {
            ("ENTER · EQUIPAR", GOLD_HI, INK_DARK)
        } else {
            ("", 0, 0)
        };
        if !chip.is_empty() {
            let cw = text_width(font, Face::Display, chip, 18.0) + 28.0;
            fill(v, slanted(x + card_w - cw - 30.0, y0 + card_h - 52.0, cw, 32.0, 8.0), rgba(chip_fill, 1.0));
            text(font, v, Face::Display, chip, x + card_w - cw - 16.0, y0 + card_h - 49.0, 18.0, rgba(chip_ink, 1.0));
        }
        // Its number, and its rim (bone in focus).
        keycap(
            font,
            v,
            Rect::new(x + 30.0, y0 + 20.0, 38.0, 38.0),
            &format!("{}", index + 1),
            18.0,
            selected,
            locked,
            accent,
            1.0,
        );
        rim(v, card, if selected { 4.0 } else { 1.5 }, rgba(if selected { BONE } else { mix(accent, 0x000000, 0.4) }, 1.0));
        if selected {
            pointer_down(v, [cx, y0 - 14.0], time);
        }
    }
    let direct = format!("1-{}", count.max(1));
    key_row(
        font,
        v,
        110.0,
        height - 66.0,
        &[("A D", "elegir"), (direct.as_str(), "directo"), ("Enter", "equipar"), ("F · Esc", "cerrar")],
        ROSE,
    );
}

/// The achievements: tiles down the left, her counts on a plate at the right.
pub(super) fn draw_achievements(
    font: &FontAtlas,
    v: &mut Vec<MapVertex>,
    width: f32,
    height: f32,
    view: &AchievementsView,
    time: f32,
) {
    backdrop(v, width, height, GOLD, 0x24160a, time);
    let earned = view.rows.iter().filter(|r| r.done).count();
    heading(
        font,
        v,
        110.0,
        46.0,
        "Logros",
        "LO QUE LA REINA HA HECHO, Y LO QUE LE HA VALIDO",
        GOLD,
    );
    // How many: a burst at the top right.
    let c = [width - 420.0, 120.0];
    let words = format!("{earned} / {}", view.rows.len());
    turned(v, c, -0.12, |v| {
        burst(v, c, 54.0, 82.0, 14, time * 0.25, rgba(GOLD_HI, 0.96), rgba(INK_DARK, 0.95), 5.0);
        let w = text_width(font, Face::Title, &words, 38.0);
        shout(font, v, Face::Title, &words, c[0] - w * 0.5, c[1] - 22.0, 38.0, 0xffffff, 0x8a5a10, 1.0);
    });
    let (x, mut y) = (130.0, 236.0);
    let tile_w = 1060.0;
    if view.rows.is_empty() {
        fill(v, slanted(x, y, tile_w, 150.0, 16.0), rgba(0x120a1a, 0.94));
        text_outline(font, v, Face::Title, "Todavía no hay logros", x + 40.0, y + 30.0, 40.0, rgba(BONE, 1.0), rgba(0x000000, 0.8));
        text_shadow(
            font,
            v,
            Face::Italic,
            "Se irán añadiendo: lo que ya has hecho queda apuntado.",
            x + 44.0,
            y + 90.0,
            22.0,
            rgba(CREAM, 0.9),
        );
    }
    for (i, row) in view.rows.iter().enumerate().take(8) {
        let h = 84.0;
        let tx = x + i as f32 * 6.0;
        let q = slanted(tx, y, tile_w, h, 10.0);
        fill(v, slanted(tx + 10.0, y + 10.0, tile_w, h, 10.0), rgba(0x000000, 0.55));
        let (top, rim_c) = if row.done {
            (mix(0x2a1a08, GOLD, 0.18), GOLD_HI)
        } else {
            (0x150e1c, 0x4a3a52)
        };
        push_triangle(v, [q[0], q[1], q[2]], rgba(top, 0.96));
        push_triangle(v, [q[0], q[2], q[3]], rgba(INK_DARK, 0.96));
        push_outline_fringe(v, &q, rgba(INK_DARK, 0.96));
        fill(v, slanted(tx, y, 12.0, h, 10.0), rgba(if row.done { GOLD } else { 0x4a3a52 }, 1.0));
        rim(v, q, if row.done { 2.0 } else { 1.0 }, rgba(rim_c, 0.9));
        // Its icon.
        match row.icon {
            Some(icon) => push_prompt_badge(v, icon, tx + 60.0, y + h * 0.5, 26.0, if row.done { 1.0 } else { 0.55 }),
            None => {
                disc(v, [tx + 60.0, y + h * 0.5], 26.0, rgba(0x241a2c, 1.0));
                let w = text_width(font, Face::Title, "?", 28.0);
                text(font, v, Face::Title, "?", tx + 60.0 - w * 0.5, y + h * 0.5 - 18.0, 28.0, rgba(0x8a7a92, 1.0));
            }
        }
        text_outline(
            font,
            v,
            Face::Title,
            &row.title,
            tx + 104.0,
            y + 10.0,
            30.0,
            rgba(if row.done { BONE } else { 0xb8aec0 }, 1.0),
            rgba(0x000000, 0.8),
        );
        text_shadow(font, v, Face::Italic, &row.detail, tx + 106.0, y + 48.0, 19.0, rgba(CREAM, if row.done { 0.92 } else { 0.7 }));
        // What it gives, on a chip at the right; a seal when earned.
        if !row.reward.is_empty() {
            let rw = text_width(font, Face::Ui, &row.reward, 18.0) + 32.0;
            let rx = tx + tile_w - rw - 110.0;
            fill(v, slanted(rx, y + 24.0, rw, 34.0, 8.0), rgba(if row.done { GOLD_HI } else { 0x2a2032 }, 1.0));
            text(font, v, Face::Ui, &row.reward, rx + 16.0, y + 29.0, 18.0, rgba(if row.done { INK_DARK } else { 0xb8aec0 }, 1.0));
        }
        if row.done {
            let sc = [tx + tile_w - 50.0, y + h * 0.5];
            disc(v, sc, 26.0, rgba(INK_DARK, 0.9));
            disc(v, sc, 22.0, rgba(GOLD_HI, 1.0));
            wire(v, [sc[0] - 10.0, sc[1]], [sc[0] - 3.0, sc[1] + 8.0], 5.0, rgba(INK_DARK, 1.0), rgba(INK_DARK, 1.0), 0.0, false);
            wire(v, [sc[0] - 3.0, sc[1] + 8.0], [sc[0] + 11.0, sc[1] - 8.0], 5.0, rgba(INK_DARK, 1.0), rgba(INK_DARK, 1.0), 0.0, false);
        } else {
            padlock(v, [tx + tile_w - 50.0, y + h * 0.5], 30.0, 0x6a5a72, 0.9);
        }
        y += h + 16.0;
    }
    // Her counts.
    let (sx, sy, sw) = (width - 600.0, 236.0, 470.0);
    let sh = 90.0 + view.stats.len() as f32 * 48.0;
    fill(v, slanted(sx + 14.0, sy + 14.0, sw, sh, 16.0), rgba(0x000000, 0.55));
    fill(v, slanted(sx, sy, sw, sh, 16.0), rgba(0x120a1a, 0.95));
    fill(v, slanted(sx, sy, sw, 12.0, 1.0), rgba(GOLD, 1.0));
    text_outline(font, v, Face::Title, "Su camino", sx + 36.0, sy + 26.0, 34.0, rgba(BONE, 1.0), rgba(0x000000, 0.8));
    for (i, (label, value)) in view.stats.iter().enumerate() {
        let ry = sy + 86.0 + i as f32 * 48.0;
        wire(v, [sx + 30.0, ry - 6.0], [sx + sw - 30.0, ry - 6.0], 1.0, rgba(GOLD_DEEP, 0.5), rgba(GOLD_DEEP, 0.5), 0.0, false);
        text_shadow(font, v, Face::Ui, label, sx + 36.0, ry + 4.0, 21.0, rgba(CREAM, 0.95));
        let w = text_width(font, Face::Title, value, 26.0);
        text_outline(font, v, Face::Title, value, sx + sw - 40.0 - w, ry, 26.0, rgba(GOLD_HI, 1.0), rgba(0x000000, 0.8));
    }
    key_row(font, v, 110.0, height - 66.0, &[("L · Esc", "cerrar")], GOLD);
    let _ = VIOLET;
}
