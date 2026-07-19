//! Casino menu buttons driven by **velvet-style** (`.vcss`) + procedural chrome.

use crate::render::{outline, panel, text};
use crate::ui::theme::{MenuItem, Theme, TITLE_ITEMS, WW, WH};
use velvet_story::pack_rgb;
use velvet_style::{resolve, Color, ComputedStyle, StyleQuery, Stylesheet};

/// Layout for the main menu button column.
pub struct ButtonColumnLayout {
    pub x: i32,
    pub y0: i32,
    pub w: i32,
    pub h: i32,
    pub gap: i32,
}

impl Default for ButtonColumnLayout {
    fn default() -> Self {
        Self {
            x: 52,
            y0: 204,
            w: 420,
            h: 52,
            gap: 12,
        }
    }
}

/// Draw START RUN / COLLECTION / … using stylesheet rules.
pub fn paint_button_column(
    pixels: &mut [u32],
    theme: &Theme,
    sheet: &Stylesheet,
    layout: &ButtonColumnLayout,
    selected: usize,
) {
    // layout height/gap from .button if present
    let base = resolve(sheet, &StyleQuery::class("button"));
    let h = base.number("height", layout.h as f32) as i32;
    let gap = base.number("gap", layout.gap as f32) as i32;
    let w = layout.w;

    for (i, item) in TITLE_ITEMS.iter().enumerate() {
        let y = layout.y0 + i as i32 * (h + gap);
        let mut q = StyleQuery::class("button").with_id(item.id);
        if i == selected {
            q = q.with_state("selected");
        }
        let style = resolve(sheet, &q);
        paint_one_button(pixels, theme, layout.x, y, w, h, item, i, i == selected, &style);
    }
}

fn paint_one_button(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    item: &MenuItem,
    index: usize,
    selected: bool,
    style: &ComputedStyle,
) {
    // shadow
    panel(pixels, WW, WH, x + 3, y + 4, w, h, (0, 0, 0), 0.35);

    let bg = style.background();
    let fg = style.color_text();
    let border = style.border_color();
    let glow = style.color("glow", Color::rgba(0, 0, 0, 0.0));
    let glow_s = style.number("glow-strength", if selected { 0.85 } else { 0.0 });

    if selected && glow_s > 0.05 {
        paint_selected_fill(pixels, x, y, w, h, bg, glow, glow_s);
        outline(
            pixels,
            WW,
            WH,
            x,
            y,
            w,
            h,
            glow.rgb_tuple(),
            1,
        );
    } else {
        panel(
            pixels,
            WW,
            WH,
            x,
            y,
            w,
            h,
            bg.rgb_tuple(),
            bg.a.clamp(0.5, 1.0),
        );
        panel(pixels, WW, WH, x + 4, y + 2, w - 8, 1, (40, 45, 70), 0.35);
    }

    paint_ornate_gold_border(pixels, x, y, w, h, border.rgb_tuple(), selected);

    let icon_s = style.number("icon-size", (h - 18) as f32) as i32;
    let pad = style.number("padding-x", 14.0) as i32;
    let icon_x = x + pad;
    let icon_y = y + (h - icon_s) / 2;
    let icon_name = style.keyword("icon", icon_fallback(index));
    paint_menu_icon(pixels, theme, icon_x, icon_y, icon_s, icon_name, selected);

    let label_x = icon_x + icon_s + 16;
    let label_y = y + h / 2 - 7;
    let gold = fg.rgb_tuple();
    if selected {
        text(
            pixels,
            WW,
            WH,
            label_x + 1,
            label_y + 1,
            item.label,
            glow.rgb_tuple(),
            2,
        );
    }
    text(pixels, WW, WH, label_x, label_y, item.label, gold, 2);
}

fn icon_fallback(index: usize) -> &'static str {
    match index {
        0 => "star",
        1 => "cards",
        2 => "chip",
        3 => "gear",
        _ => "power",
    }
}

fn paint_selected_fill(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    bg: Color,
    glow: Color,
    strength: f32,
) {
    panel(pixels, WW, WH, x, y, w, h, (6, 4, 14), 0.96);
    for col in 0..w {
        let t = col as f32 / w as f32;
        let g = (1.0 - (t - 0.22).abs() * 1.6).clamp(0.0, 1.0);
        let g = g * g * strength;
        let r = (bg.r as f32 * (1.0 - g) + glow.r as f32 * g) as u8;
        let gr = (bg.g as f32 * (1.0 - g) + glow.g as f32 * g) as u8;
        let b = (bg.b as f32 * (1.0 - g) + glow.b as f32 * g) as u8;
        for row in 0..h {
            let v = 1.0 - ((row as f32 / h as f32) - 0.5).abs() * 1.2;
            let v = v.clamp(0.15, 1.0);
            let a = 0.5 + 0.45 * g * v;
            let px = x + col;
            let py = y + row;
            if px < 0 || py < 0 || px >= WW as i32 || py >= WH as i32 {
                continue;
            }
            let i = (py as u32 * WW + px as u32) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(r, gr, b), a);
        }
    }
    let sparks: [(i32, i32); 12] = [
        (38, 12),
        (72, 28),
        (110, 18),
        (155, 32),
        (190, 14),
        (230, 26),
        (270, 16),
        (310, 30),
        (350, 20),
        (95, 36),
        (210, 38),
        (60, 20),
    ];
    for (sx, sy) in sparks {
        if sx < w - 8 && sy < h - 4 {
            panel(
                pixels,
                WW,
                WH,
                x + sx,
                y + sy,
                2,
                2,
                (255, 200, 255),
                0.7 * strength,
            );
        }
    }
}

fn paint_ornate_gold_border(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    gold: (u8, u8, u8),
    selected: bool,
) {
    let gold_hi = if selected {
        (
            gold.0.saturating_add(20),
            gold.1.saturating_add(20),
            gold.2.saturating_add(20),
        )
    } else {
        gold
    };
    outline(pixels, WW, WH, x, y, w, h, gold, 1);
    outline(pixels, WW, WH, x + 2, y + 2, w - 4, h - 4, gold_hi, 1);
    let d = 4;
    for (cx, cy) in [
        (x + 5, y + 5),
        (x + w - 6, y + 5),
        (x + 5, y + h - 6),
        (x + w - 6, y + h - 6),
    ] {
        paint_diamond(pixels, cx, cy, d, gold_hi);
    }
    paint_diamond(pixels, x + w / 2, y + 3, 3, gold);
    paint_diamond(pixels, x + w / 2, y + h - 4, 3, gold);
    paint_diamond(pixels, x + 3, y + h / 2, 3, gold);
    paint_diamond(pixels, x + w - 4, y + h / 2, 3, gold);
}

fn paint_diamond(pixels: &mut [u32], cx: i32, cy: i32, size: i32, rgb: (u8, u8, u8)) {
    for dy in -size..=size {
        let span = size - dy.abs();
        panel(
            pixels,
            WW,
            WH,
            cx - span,
            cy + dy,
            span * 2 + 1,
            1,
            rgb,
            0.95,
        );
    }
}

fn paint_menu_icon(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    size: i32,
    icon: &str,
    selected: bool,
) {
    let gold = if selected {
        (255, 220, 140)
    } else {
        theme.gold_soft
    };
    let pink = (255, 120, 220);
    let cx = x + size / 2;
    let cy = y + size / 2;
    let s = size / 2 - 2;
    let kind = icon.to_ascii_lowercase();

    match kind.as_str() {
        "star" => {
            if selected {
                for r in (2..s + 4).rev() {
                    panel(pixels, WW, WH, cx - r, cy - 1, r * 2, 3, pink, 0.08);
                }
            }
            for i in 0..=s {
                let t = 1 + (s - i) / 3;
                panel(pixels, WW, WH, cx - t / 2, cy - s + i, t.max(1), 1, gold, 0.95);
                panel(pixels, WW, WH, cx - t / 2, cy + s - i, t.max(1), 1, gold, 0.95);
                panel(pixels, WW, WH, cx - s + i, cy - t / 2, 1, t.max(1), gold, 0.95);
                panel(pixels, WW, WH, cx + s - i, cy - t / 2, 1, t.max(1), gold, 0.95);
            }
            panel(pixels, WW, WH, cx - 1, cy - 1, 3, 3, (255, 240, 200), 0.95);
        }
        "cards" => {
            outline(pixels, WW, WH, x + 6, y + 8, size / 2 + 2, size - 14, gold, 1);
            outline(
                pixels,
                WW,
                WH,
                x + size / 3,
                y + 6,
                size / 2 + 2,
                size - 14,
                gold,
                1,
            );
        }
        "chip" => {
            draw_circle_outline(pixels, cx, cy, s - 1, gold, 2);
            draw_circle_outline(pixels, cx, cy, s / 2, gold, 1);
        }
        "gear" => {
            outline(
                pixels,
                WW,
                WH,
                cx - s + 2,
                cy - s + 2,
                (s - 2) * 2,
                (s - 2) * 2,
                gold,
                2,
            );
            panel(pixels, WW, WH, cx - 2, cy - 2, 5, 5, gold, 0.85);
        }
        _ => {
            draw_circle_outline(pixels, cx, cy + 1, s - 2, gold, 2);
            panel(pixels, WW, WH, cx - 3, cy - s + 2, 7, 5, (8, 6, 16), 1.0);
            panel(pixels, WW, WH, cx - 1, cy - s + 2, 3, s, gold, 0.95);
        }
    }
}

fn draw_circle_outline(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    r: i32,
    rgb: (u8, u8, u8),
    thickness: i32,
) {
    let r2 = r * r;
    let r_in = (r - thickness).max(0);
    let r_in2 = r_in * r_in;
    for dy in -r..=r {
        for dx in -r..=r {
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 && d2 >= r_in2 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && px < WW as i32 && py < WH as i32 {
                    let i = (py as u32 * WW + px as u32) as usize;
                    pixels[i] = blend(pixels[i], pack_rgb(rgb.0, rgb.1, rgb.2), 0.92);
                }
            }
        }
    }
}

fn blend(dst: u32, src: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((src >> 16) & 0xFF) as f32;
    let sg = ((src >> 8) & 0xFF) as f32;
    let sb = (src & 0xFF) as f32;
    pack_rgb(
        (dr + (sr - dr) * t) as u8,
        (dg + (sg - dg) * t) as u8,
        (db + (sb - db) * t) as u8,
    )
}
