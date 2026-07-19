//! Casino menu buttons — ornate plates + icons (style-matched, original art).

use std::path::Path;

use crate::render::{blit_card, load_rgb, outline, panel, text, RgbImage};
use crate::ui::theme::{MenuItem, Theme, TITLE_ITEMS, WW, WH};

/// Loaded button chrome.
pub struct ButtonChrome {
    pub plate_selected: Option<RgbImage>,
    pub plate_normal: Option<RgbImage>,
    pub icons: [Option<RgbImage>; 5],
}

impl ButtonChrome {
    /// Load from `data/ui/buttons/`.
    pub fn load(dir: &Path) -> Self {
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
        }
    }

    pub fn ready(&self) -> bool {
        self.plate_selected.is_some() && self.plate_normal.is_some()
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
        Self {
            x: 48,
            y0: 198,
            w: 400,
            h: 56,
            gap: 14,
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
    for (i, item) in TITLE_ITEMS.iter().enumerate() {
        let y = layout.y0 + i as i32 * (layout.h + layout.gap);
        paint_one_button(
            pixels,
            theme,
            chrome,
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
    chrome: &ButtonChrome,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    item: &MenuItem,
    index: usize,
    selected: bool,
) {
    // Base plate art
    let plate = if selected {
        chrome.plate_selected.as_ref()
    } else {
        chrome.plate_normal.as_ref()
    };
    if let Some(p) = plate {
        blit_card(pixels, WW, WH, p, x, y, w, h, 1.0);
    } else {
        // Procedural fallback matching the reference look
        paint_procedural_plate(pixels, theme, x, y, w, h, selected);
    }

    // Extra gold frame + diamond corners (code-perfected ornaments)
    paint_gold_frame(pixels, theme, x, y, w, h, selected);

    // Soft outer glow when selected
    if selected {
        outline(pixels, WW, WH, x - 1, y - 1, w + 2, h + 2, (180, 60, 200), 1);
        // magenta edge accent under gold
        for t in 0..2 {
            let c = (200u8.saturating_sub(t as u8 * 40), 40, 160);
            outline(
                pixels,
                WW,
                WH,
                x + 3 + t,
                y + 3 + t,
                w - 6 - t * 2,
                h - 6 - t * 2,
                c,
                1,
            );
        }
    }

    // Icon
    let icon_size = h - 16;
    let icon_x = x + 14;
    let icon_y = y + 8;
    if let Some(Some(icon)) = chrome.icons.get(index) {
        // slight circular backdrop
        panel(
            pixels,
            WW,
            WH,
            icon_x - 2,
            icon_y - 2,
            icon_size + 4,
            icon_size + 4,
            (8, 4, 16),
            0.45,
        );
        blit_card(
            pixels,
            WW,
            WH,
            icon,
            icon_x,
            icon_y,
            icon_size,
            icon_size,
            1.0,
        );
    } else {
        paint_fallback_icon(pixels, theme, icon_x, icon_y, icon_size, index, selected);
    }

    // Label — gold like the reference
    let label_x = x + 14 + icon_size + 18;
    let label_y = y + h / 2 - 8;
    let color = if selected {
        (255, 220, 140)
    } else {
        theme.gold_soft
    };
    // dual-pass soft glow on selected label
    if selected {
        text(
            pixels,
            WW,
            WH,
            label_x + 1,
            label_y + 1,
            item.label,
            (120, 40, 160),
            2,
        );
    }
    text(pixels, WW, WH, label_x, label_y, item.label, color, 2);
}

/// Procedural plate if art missing — still close to the screenshot.
fn paint_procedural_plate(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    selected: bool,
) {
    if selected {
        // purple glow fill gradient-ish
        for row in 0..h {
            let t = row as f32 / h as f32;
            let r = (40.0 + 80.0 * (1.0 - (t - 0.5).abs() * 2.0)) as u8;
            let g = (10.0 + 20.0 * t) as u8;
            let b = (70.0 + 100.0 * (1.0 - t * 0.5)) as u8;
            panel(pixels, WW, WH, x, y + row, w, 1, (r, g, b), 0.92);
        }
        // sparkle dots
        let seeds = [7i32, 19, 31, 47, 61, 83, 97, 113, 131, 151];
        for (i, s) in seeds.iter().enumerate() {
            let i = i as i32;
            let span_x = (w - 40).max(1);
            let span_y = (h - 16).max(1);
            let px = x + 20 + (s.wrapping_mul(13).wrapping_add(i * 29)).rem_euclid(span_x);
            let py = y + 8 + (s.wrapping_mul(7).wrapping_add(i * 11)).rem_euclid(span_y);
            panel(pixels, WW, WH, px, py, 2, 2, (255, 180, 255), 0.85);
        }
    } else {
        panel(pixels, WW, WH, x, y, w, h, (12, 10, 22), 0.92);
        // subtle top edge
        panel(pixels, WW, WH, x, y, w, 1, (40, 35, 55), 0.5);
    }
    let _ = theme;
}

fn paint_gold_frame(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    selected: bool,
) {
    let gold = theme.gold;
    let gold_dim = (160, 120, 60);
    let c = if selected { gold } else { gold_dim };
    // outer thin gold
    outline(pixels, WW, WH, x, y, w, h, c, 1);
    // inner gold line
    outline(pixels, WW, WH, x + 3, y + 3, w - 6, h - 6, c, 1);
    // diamond corners
    let d = 5;
    for (cx, cy) in [
        (x + 6, y + 6),
        (x + w - 7, y + 6),
        (x + 6, y + h - 7),
        (x + w - 7, y + h - 7),
    ] {
        paint_diamond(pixels, cx, cy, d, gold);
    }
    // mid-side diamonds (like the reference bar ornaments)
    paint_diamond(pixels, x + w / 2, y + 4, 3, gold_dim);
    paint_diamond(pixels, x + w / 2, y + h - 5, 3, gold_dim);
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

fn paint_fallback_icon(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    size: i32,
    index: usize,
    selected: bool,
) {
    let c = if selected {
        theme.gold
    } else {
        theme.gold_soft
    };
    // simple geometric marks
    match index {
        0 => {
            // star cross
            panel(pixels, WW, WH, x + size / 2 - 1, y + 4, 3, size - 8, c, 0.9);
            panel(pixels, WW, WH, x + 4, y + size / 2 - 1, size - 8, 3, c, 0.9);
        }
        1 => {
            panel(pixels, WW, WH, x + 6, y + 8, size / 2, size - 14, c, 0.7);
            panel(pixels, WW, WH, x + size / 3, y + 6, size / 2, size - 14, c, 0.9);
        }
        2 => {
            outline(
                pixels,
                WW,
                WH,
                x + 6,
                y + 6,
                size - 12,
                size - 12,
                c,
                2,
            );
        }
        3 => {
            outline(
                pixels,
                WW,
                WH,
                x + 8,
                y + 8,
                size - 16,
                size - 16,
                c,
                2,
            );
            panel(
                pixels,
                WW,
                WH,
                x + size / 2 - 2,
                y + size / 2 - 2,
                5,
                5,
                c,
                0.9,
            );
        }
        _ => {
            outline(
                pixels,
                WW,
                WH,
                x + 8,
                y + 8,
                size - 16,
                size - 16,
                c,
                2,
            );
            panel(
                pixels,
                WW,
                WH,
                x + size / 2 - 1,
                y + 10,
                3,
                size / 2,
                c,
                0.9,
            );
        }
    }
}
