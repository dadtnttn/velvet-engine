//! Casino menu buttons — drawn to match the Nightfall reference look.
//!
//! Original assets are optional; the **primary** path is a careful procedural
//! recreation (thin gold frames, diamond corners, selected magenta glow).
//! User screenshots are never shipped as game files.

use std::path::Path;

use crate::render::{blit_card, load_rgb, outline, panel, text, RgbImage};
use crate::ui::theme::{MenuItem, Theme, TITLE_ITEMS, WW, WH};
use velvet_story::pack_rgb;

/// Optional decorative plates/icons (never required).
pub struct ButtonChrome {
    pub plate_selected: Option<RgbImage>,
    pub plate_normal: Option<RgbImage>,
    pub icons: [Option<RgbImage>; 5],
    /// Prefer procedural drawing (true = match reference closely).
    pub procedural: bool,
}

impl ButtonChrome {
    pub fn load(dir: &Path) -> Self {
        let _ = dir; // art optional; we draw the real look in code
        Self {
            plate_selected: None,
            plate_normal: None,
            icons: [None, None, None, None, None],
            procedural: true,
        }
    }

    /// Load optional plates if present (still may force procedural).
    pub fn load_with_art(dir: &Path, use_art: bool) -> Self {
        if !use_art {
            return Self::load(dir);
        }
        Self {
            plate_selected: load_rgb(&dir.join("plate_selected.jpg")),
            plate_normal: load_rgb(&dir.join("plate_normal.jpg")),
            icons: [
                load_rgb(&dir.join("icon_start.jpg")),
                load_rgb(&dir.join("icon_collection.jpg")),
                load_rgb(&dir.join("icon_shop.jpg")),
                load_rgb(&dir.join("icon_options.jpg")),
                load_rgb(&dir.join("icon_quit.jpg")),
            ],
            procedural: true, // always procedural for fidelity
        }
    }

    pub fn ready(&self) -> bool {
        true
    }
}

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
        // Proportions close to the reference: long thin bars, tight gaps
        Self {
            x: 52,
            y0: 204,
            w: 420,
            h: 52,
            gap: 12,
        }
    }
}

/// Draw the full START RUN / COLLECTION / … column.
pub fn paint_button_column(
    pixels: &mut [u32],
    theme: &Theme,
    chrome: &ButtonChrome,
    layout: &ButtonColumnLayout,
    selected: usize,
) {
    let _ = chrome;
    for (i, item) in TITLE_ITEMS.iter().enumerate() {
        let y = layout.y0 + i as i32 * (layout.h + layout.gap);
        paint_one_button(
            pixels,
            theme,
            layout.x,
            y,
            layout.w,
            layout.h,
            item,
            i,
            i == selected,
        );
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
) {
    // Outer soft shadow
    panel(
        pixels,
        WW,
        WH,
        x + 3,
        y + 4,
        w,
        h,
        (0, 0, 0),
        0.35,
    );

    // Fill
    if selected {
        paint_selected_fill(pixels, x, y, w, h);
    } else {
        // near-black navy bar
        panel(pixels, WW, WH, x, y, w, h, (10, 12, 22), 0.94);
        // slight top highlight
        panel(pixels, WW, WH, x + 4, y + 2, w - 8, 1, (40, 45, 70), 0.35);
    }

    // Gold double frame + diamonds (core of the reference look)
    paint_ornate_gold_border(pixels, x, y, w, h, selected);

    // Icon circle area
    let icon_s = h - 18;
    let icon_x = x + 14;
    let icon_y = y + (h - icon_s) / 2;
    paint_menu_icon(pixels, theme, icon_x, icon_y, icon_s, index, selected);

    // Gold label (reference uses elegant gold — we approximate with pixel font)
    let label_x = icon_x + icon_s + 16;
    let label_y = y + h / 2 - 7;
    let gold = if selected {
        (255, 228, 150)
    } else {
        (210, 175, 100)
    };
    if selected {
        // magenta bloom behind text
        text(
            pixels,
            WW,
            WH,
            label_x + 1,
            label_y + 1,
            item.label,
            (160, 50, 200),
            2,
        );
    }
    text(pixels, WW, WH, label_x, label_y, item.label, gold, 2);

    let _ = theme;
}

/// Selected bar: purple nebula sweep left→right + sparkles.
fn paint_selected_fill(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32) {
    // base black
    panel(pixels, WW, WH, x, y, w, h, (6, 4, 14), 0.96);
    for col in 0..w {
        let t = col as f32 / w as f32;
        // bright magenta-purple near left-center, fade to dark on right
        let glow = (1.0 - (t - 0.22).abs() * 1.6).clamp(0.0, 1.0);
        let glow = glow * glow;
        let r = (25.0 + 160.0 * glow) as u8;
        let g = (8.0 + 30.0 * glow) as u8;
        let b = (55.0 + 160.0 * glow) as u8;
        // vertical soft falloff
        for row in 0..h {
            let v = 1.0 - ((row as f32 / h as f32) - 0.5).abs() * 1.2;
            let v = v.clamp(0.15, 1.0);
            let a = 0.55 + 0.4 * glow * v;
            let px = x + col;
            let py = y + row;
            if px < 0 || py < 0 || px >= WW as i32 || py >= WH as i32 {
                continue;
            }
            let i = (py as u32 * WW + px as u32) as usize;
            pixels[i] = blend(pixels[i], pack_rgb(r, g, b), a * 0.85);
        }
    }
    // sparkles
    let sparks: [(i32, i32); 14] = [
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
        (290, 10),
        (330, 34),
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
                0.75,
            );
        }
    }
    // pink outer rim (reference selected has magenta edge)
    outline(pixels, WW, WH, x, y, w, h, (220, 80, 220), 1);
    outline(pixels, WW, WH, x + 1, y + 1, w - 2, h - 2, (140, 40, 180), 1);
}

fn paint_ornate_gold_border(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    selected: bool,
) {
    let gold = if selected {
        (235, 200, 110)
    } else {
        (185, 150, 75)
    };
    let gold_hi = if selected {
        (255, 230, 160)
    } else {
        (210, 175, 95)
    };
    // double frame
    outline(pixels, WW, WH, x, y, w, h, gold, 1);
    outline(pixels, WW, WH, x + 2, y + 2, w - 4, h - 4, gold_hi, 1);
    // corner diamonds (outside-ish)
    let d = 4;
    for (cx, cy) in [
        (x + 5, y + 5),
        (x + w - 6, y + 5),
        (x + 5, y + h - 6),
        (x + w - 6, y + h - 6),
    ] {
        paint_diamond(pixels, cx, cy, d, gold_hi);
    }
    // mid-edge diamonds
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

/// Clean gold line-icons (no JPEG black boxes).
fn paint_menu_icon(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    size: i32,
    index: usize,
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

    match index {
        0 => {
            // four-point star
            if selected {
                // glow
                for r in (2..s + 4).rev() {
                    let a = 0.08;
                    panel(
                        pixels,
                        WW,
                        WH,
                        cx - r,
                        cy - 1,
                        r * 2,
                        3,
                        pink,
                        a,
                    );
                }
            }
            // vertical + horizontal diamond points
            for i in 0..=s {
                let t = 1 + (s - i) / 3;
                panel(pixels, WW, WH, cx - t / 2, cy - s + i, t.max(1), 1, gold, 0.95);
                panel(pixels, WW, WH, cx - t / 2, cy + s - i, t.max(1), 1, gold, 0.95);
                panel(pixels, WW, WH, cx - s + i, cy - t / 2, 1, t.max(1), gold, 0.95);
                panel(pixels, WW, WH, cx + s - i, cy - t / 2, 1, t.max(1), gold, 0.95);
            }
            panel(pixels, WW, WH, cx - 1, cy - 1, 3, 3, (255, 240, 200), 0.95);
        }
        1 => {
            // two cards
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
            panel(
                pixels,
                WW,
                WH,
                x + size / 3 + 4,
                y + 12,
                3,
                3,
                gold,
                0.9,
            );
        }
        2 => {
            // chip ring
            let r = s - 1;
            draw_circle_outline(pixels, cx, cy, r, gold, 2);
            draw_circle_outline(pixels, cx, cy, r / 2, gold, 1);
            panel(pixels, WW, WH, cx - 1, cy - 1, 3, 3, gold, 0.9);
        }
        3 => {
            // gear (octagon-ish)
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
            for (dx, dy) in [(-s, 0), (s, 0), (0, -s), (0, s)] {
                panel(pixels, WW, WH, cx + dx - 1, cy + dy - 1, 3, 3, gold, 0.9);
            }
        }
        _ => {
            // power: circle + stem
            draw_circle_outline(pixels, cx, cy + 1, s - 2, gold, 2);
            // gap at top
            panel(
                pixels,
                WW,
                WH,
                cx - 3,
                cy - s + 2,
                7,
                5,
                (8, 6, 16),
                1.0,
            );
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
