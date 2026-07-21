//! Top HUD chrome inspired by the Nightfall Casino reference:
//! compact collector identity at the left and segmented meta counters at the right.

use crate::render::{panel, text, RgbImage};
use crate::title_font::{draw_font_text, ui_font};
use crate::ui::theme::{Theme, WH, WW};
use velvet_story::pack_rgb;

const COPPER: (u8, u8, u8) = (166, 101, 58);
const COPPER_BRIGHT: (u8, u8, u8) = (226, 151, 91);
const COPPER_DIM: (u8, u8, u8) = (79, 48, 59);
const MAGENTA: (u8, u8, u8) = (234, 49, 169);
const VIOLET: (u8, u8, u8) = (174, 73, 246);
const HUD_FILL: (u8, u8, u8) = (7, 6, 15);

/// Paint collector profile (reference style) + meta strip.
pub fn paint_meta_hud(
    pixels: &mut [u32],
    theme: &Theme,
    portrait: Option<&RgbImage>,
    chips: i64,
    crystals: i64,
    mult: f32,
    player_name: &str,
    player_rank: &str,
    level: u32,
    xp_frac: f32,
) {
    paint_profile_card(
        pixels,
        theme,
        portrait,
        21,
        21,
        player_name,
        player_rank,
        level,
        xp_frac,
    );
    paint_resource_strip(pixels, theme, chips, crystals, mult);
}

fn paint_resource_strip(pixels: &mut [u32], theme: &Theme, chips: i64, crystals: i64, mult: f32) {
    const X: i32 = 855;
    const Y: i32 = 17;
    const W: i32 = 410;
    const H: i32 = 57;

    paint_cut_panel(pixels, X + 3, Y + 4, W, H, (0, 0, 5), 0.62, COPPER_DIM);
    paint_cut_panel(pixels, X, Y, W, H, HUD_FILL, 0.9, COPPER);
    paint_corner_ticks(pixels, X, Y, W, H);

    for sx in [983, 1089, 1192] {
        paint_line(pixels, sx, Y + 8, sx, Y + H - 8, COPPER_DIM);
        set_pixel(pixels, sx, Y + 29, COPPER_BRIGHT);
        set_pixel(pixels, sx, Y + 30, COPPER_BRIGHT);
    }

    paint_chip(pixels, 873, 34, 14);
    paint_hud_text(pixels, 900, 34, "CHIPS", 9.5, COPPER_BRIGHT, 1.0, 1);
    paint_hud_text(
        pixels,
        900,
        58,
        &format_counter(chips),
        17.0,
        theme.text,
        1.0,
        1,
    );

    paint_crystal(pixels, 1001, 29, 14, 23);
    paint_hud_text(pixels, 1022, 34, "CRYSTALS", 9.5, COPPER_BRIGHT, 1.0, 1);
    paint_hud_text(
        pixels,
        1022,
        58,
        &format_counter(crystals),
        17.0,
        theme.text,
        1.0,
        1,
    );

    paint_hud_text(pixels, 1104, 34, "MULTIPLIER", 9.0, COPPER_BRIGHT, 1.0, 1);
    paint_hud_text(
        pixels,
        1104,
        58,
        &format!("x{mult:.1}"),
        17.0,
        theme.text,
        1.0,
        1,
    );

    paint_menu_button(pixels, 1205, 24, 48, 43);
}

#[allow(clippy::too_many_arguments)]
fn paint_profile_card(
    pixels: &mut [u32],
    theme: &Theme,
    portrait: Option<&RgbImage>,
    x: i32,
    y: i32,
    name: &str,
    rank: &str,
    level: u32,
    xp_frac: f32,
) {
    const W: i32 = 237;
    const H: i32 = 81;
    paint_cut_panel(pixels, x + 3, y + 4, W, H, (0, 0, 5), 0.64, COPPER_DIM);
    paint_cut_panel(pixels, x, y, W, H, HUD_FILL, 0.91, COPPER);
    paint_corner_ticks(pixels, x, y, W, H);

    let portrait_x = x + 7;
    let portrait_y = y + 6;
    const PORTRAIT_SIZE: i32 = 70;
    panel(
        pixels,
        WW,
        WH,
        portrait_x - 2,
        portrait_y - 2,
        PORTRAIT_SIZE + 4,
        PORTRAIT_SIZE + 4,
        (35, 14, 53),
        0.96,
    );
    paint_rect_frame(
        pixels,
        portrait_x - 2,
        portrait_y - 2,
        PORTRAIT_SIZE + 4,
        PORTRAIT_SIZE + 4,
        COPPER_BRIGHT,
    );
    paint_rect_frame(
        pixels,
        portrait_x,
        portrait_y,
        PORTRAIT_SIZE,
        PORTRAIT_SIZE,
        (91, 43, 121),
    );

    if let Some(img) = portrait {
        blit_portrait_cover(
            pixels,
            img,
            portrait_x + 1,
            portrait_y + 1,
            PORTRAIT_SIZE - 2,
            PORTRAIT_SIZE - 2,
        );
    } else {
        panel(
            pixels,
            WW,
            WH,
            portrait_x + 1,
            portrait_y + 1,
            PORTRAIT_SIZE - 2,
            PORTRAIT_SIZE - 2,
            (24, 13, 42),
            1.0,
        );
        paint_profile_silhouette(pixels, portrait_x + PORTRAIT_SIZE / 2, portrait_y + 17);
    }

    let text_x = x + 88;
    paint_hud_text(pixels, text_x, y + 27, name, 16.0, theme.gold_soft, 1.0, 1);
    paint_hud_text(pixels, text_x, y + 45, rank, 11.5, (194, 143, 121), 0.95, 1);

    let bar_x = text_x + 12;
    let bar_y = y + 61;
    const BAR_W: i32 = 86;
    const BAR_H: i32 = 7;
    paint_crystal(pixels, text_x - 1, bar_y - 3, 10, 14);
    panel(
        pixels,
        WW,
        WH,
        bar_x,
        bar_y,
        BAR_W,
        BAR_H,
        (25, 12, 33),
        0.98,
    );
    paint_rect_frame(pixels, bar_x, bar_y, BAR_W, BAR_H, COPPER_DIM);
    paint_progress_fill(pixels, bar_x + 1, bar_y + 1, BAR_W - 2, BAR_H - 2, xp_frac);
    paint_hud_text(
        pixels,
        bar_x + BAR_W + 8,
        y + 69,
        &format!("LVL {level}"),
        10.5,
        theme.gold_soft,
        1.0,
        1,
    );
}

fn paint_progress_fill(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, frac: f32) {
    let fill = (frac.clamp(0.0, 1.0) * w as f32).round() as i32;
    for col in 0..fill {
        let t = col as f32 / w.max(1) as f32;
        let rgb = (
            (218.0 + 23.0 * t) as u8,
            (34.0 + 57.0 * t) as u8,
            (143.0 + 48.0 * t) as u8,
        );
        panel(pixels, WW, WH, x + col, y, 1, h, rgb, 0.98);
    }
    if fill > 3 {
        panel(pixels, WW, WH, x + 1, y, fill - 2, 1, (255, 154, 222), 0.82);
    }
}

fn paint_menu_button(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32) {
    paint_cut_panel(pixels, x, y, w, h, (16, 11, 24), 0.96, COPPER);
    paint_rect_frame(pixels, x + 5, y + 5, w - 10, h - 10, COPPER_DIM);
    for offset in [0, 7, 14] {
        paint_line(
            pixels,
            x + 14,
            y + 15 + offset,
            x + 33,
            y + 15 + offset,
            COPPER_BRIGHT,
        );
        paint_line(
            pixels,
            x + 15,
            y + 16 + offset,
            x + 32,
            y + 16 + offset,
            (118, 65, 58),
        );
    }
}

fn paint_chip(pixels: &mut [u32], cx: i32, cy: i32, radius: i32) {
    let outer_sq = radius * radius;
    let inner = radius - 4;
    let inner_sq = inner * inner;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d = dx * dx + dy * dy;
            if d > outer_sq {
                continue;
            }
            let rgb = if d >= inner_sq {
                if (dx.abs() < 3 && dy.abs() > inner - 2) || (dy.abs() < 3 && dx.abs() > inner - 2)
                {
                    COPPER_BRIGHT
                } else {
                    (126, 55, 125)
                }
            } else {
                (30, 13, 43)
            };
            set_pixel(pixels, cx + dx, cy + dy, rgb);
        }
    }
    paint_rect_frame(pixels, cx - 4, cy - 4, 9, 9, MAGENTA);
    set_pixel(pixels, cx, cy, COPPER_BRIGHT);
}

fn paint_crystal(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32) {
    let cx = x + w / 2;
    let half = w / 2;
    for row in 0..h {
        let upper = row <= h / 2;
        let span = if upper {
            ((row + 1) * half / (h / 2).max(1)).max(1)
        } else {
            ((h - row) * half / (h / 2).max(1)).max(1)
        };
        for dx in -span..=span {
            let bright = 1.0 - dx.abs() as f32 / (span + 1) as f32;
            let rgb = (
                (151.0 + 82.0 * bright) as u8,
                (42.0 + 57.0 * bright) as u8,
                (210.0 + 43.0 * bright) as u8,
            );
            set_pixel(pixels, cx + dx, y + row, rgb);
        }
    }
    paint_line(pixels, cx, y + 1, cx, y + h - 2, (255, 177, 242));
    paint_line(pixels, x + 1, y + h / 2, cx, y + 1, VIOLET);
    paint_line(pixels, cx, y + 1, x + w - 1, y + h / 2, MAGENTA);
}

fn paint_profile_silhouette(pixels: &mut [u32], cx: i32, top: i32) {
    for dy in -9..=9 {
        let span = (9 * 9 - dy * dy).max(0) as f32;
        let span = span.sqrt() as i32;
        for dx in -span..=span {
            set_pixel(pixels, cx + dx, top + 9 + dy, (96, 44, 121));
        }
    }
    for row in 0..24 {
        let span = 11 + row / 2;
        for dx in -span..=span {
            set_pixel(pixels, cx + dx, top + 21 + row, (72, 31, 102));
        }
    }
}

fn blit_portrait_cover(
    pixels: &mut [u32],
    image: &RgbImage,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let (source_width, source_height, source) = image;
    let crop = (*source_width).min(*source_height);
    if crop == 0 {
        return;
    }
    let crop_x = (*source_width - crop) / 2;
    let crop_y = (*source_height - crop) / 2;
    for row in 0..height {
        let source_y = crop_y + (row as u32 * crop) / height as u32;
        for col in 0..width {
            let source_x = crop_x + (col as u32 * crop) / width as u32;
            let source_index = (source_y * *source_width + source_x) as usize;
            let Some(&color) = source.get(source_index) else {
                continue;
            };
            let target_x = x + col;
            let target_y = y + row;
            if target_x < 0 || target_y < 0 || target_x >= WW as i32 || target_y >= WH as i32 {
                continue;
            }
            let target_index = (target_y as u32 * WW + target_x as u32) as usize;
            if let Some(pixel) = pixels.get_mut(target_index) {
                *pixel = color;
            }
        }
    }
}

fn paint_hud_text(
    pixels: &mut [u32],
    x: i32,
    baseline_y: i32,
    value: &str,
    px: f32,
    rgb: (u8, u8, u8),
    opacity: f32,
    fallback_scale: i32,
) {
    if let Some(font) = ui_font() {
        draw_font_text(
            pixels,
            font,
            x as f32,
            baseline_y as f32,
            value,
            px,
            rgb,
            opacity,
        );
    } else {
        text(
            pixels,
            WW,
            WH,
            x,
            baseline_y - 7 * fallback_scale,
            value,
            rgb,
            fallback_scale,
        );
    }
}

fn paint_cut_panel(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: (u8, u8, u8),
    alpha: f32,
    border: (u8, u8, u8),
) {
    const CUT: i32 = 7;
    for row in 0..h {
        let inset = if row < CUT {
            CUT - row
        } else if row >= h - CUT {
            row - (h - CUT - 1)
        } else {
            0
        };
        for col in inset..w - inset {
            blend_pixel(pixels, x + col, y + row, fill, alpha);
        }
    }

    paint_line(pixels, x + CUT, y, x + w - CUT - 1, y, border);
    paint_line(pixels, x + w - CUT - 1, y, x + w - 1, y + CUT, border);
    paint_line(
        pixels,
        x + w - 1,
        y + CUT,
        x + w - 1,
        y + h - CUT - 1,
        border,
    );
    paint_line(
        pixels,
        x + w - 1,
        y + h - CUT - 1,
        x + w - CUT - 1,
        y + h - 1,
        border,
    );
    paint_line(
        pixels,
        x + w - CUT - 1,
        y + h - 1,
        x + CUT,
        y + h - 1,
        border,
    );
    paint_line(pixels, x + CUT, y + h - 1, x, y + h - CUT - 1, border);
    paint_line(pixels, x, y + h - CUT - 1, x, y + CUT, border);
    paint_line(pixels, x, y + CUT, x + CUT, y, border);
}

fn paint_corner_ticks(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32) {
    for &(sx, sy, dx, dy) in &[
        (x + 9, y + 4, 1, 1),
        (x + w - 10, y + 4, -1, 1),
        (x + 9, y + h - 5, 1, -1),
        (x + w - 10, y + h - 5, -1, -1),
    ] {
        paint_line(pixels, sx, sy, sx + 5 * dx, sy, COPPER_DIM);
        paint_line(pixels, sx, sy, sx, sy + 5 * dy, COPPER_DIM);
        set_pixel(pixels, sx + 2 * dx, sy + 2 * dy, COPPER_BRIGHT);
    }
}

fn paint_rect_frame(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, rgb: (u8, u8, u8)) {
    paint_line(pixels, x, y, x + w - 1, y, rgb);
    paint_line(pixels, x + w - 1, y, x + w - 1, y + h - 1, rgb);
    paint_line(pixels, x + w - 1, y + h - 1, x, y + h - 1, rgb);
    paint_line(pixels, x, y + h - 1, x, y, rgb);
}

fn paint_line(pixels: &mut [u32], mut x0: i32, mut y0: i32, x1: i32, y1: i32, rgb: (u8, u8, u8)) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        set_pixel(pixels, x0, y0, rgb);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn set_pixel(pixels: &mut [u32], x: i32, y: i32, rgb: (u8, u8, u8)) {
    if x < 0 || y < 0 || x >= WW as i32 || y >= WH as i32 {
        return;
    }
    let index = (y as u32 * WW + x as u32) as usize;
    if let Some(pixel) = pixels.get_mut(index) {
        *pixel = pack_rgb(rgb.0, rgb.1, rgb.2);
    }
}

fn blend_pixel(pixels: &mut [u32], x: i32, y: i32, rgb: (u8, u8, u8), alpha: f32) {
    if x < 0 || y < 0 || x >= WW as i32 || y >= WH as i32 {
        return;
    }
    let index = (y as u32 * WW + x as u32) as usize;
    let Some(dst) = pixels.get_mut(index) else {
        return;
    };
    let t = alpha.clamp(0.0, 1.0);
    let dr = ((*dst >> 16) & 0xff) as f32;
    let dg = ((*dst >> 8) & 0xff) as f32;
    let db = (*dst & 0xff) as f32;
    *dst = pack_rgb(
        (dr + (rgb.0 as f32 - dr) * t) as u8,
        (dg + (rgb.1 as f32 - dg) * t) as u8,
        (db + (rgb.2 as f32 - db) * t) as u8,
    );
}

fn format_counter(value: i64) -> String {
    let negative = value.is_negative();
    let digits = value.unsigned_abs().to_string();
    let mut grouped =
        String::with_capacity(digits.len() + digits.len() / 3 + usize::from(negative));
    if negative {
        grouped.push('-');
    }
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_meta_values_for_hud() {
        assert_eq!(format_counter(12_450), "12,450");
        assert_eq!(format_counter(-1_250_000), "-1,250,000");
        assert_eq!(format_counter(0), "0");
    }

    #[test]
    fn paints_two_compact_hud_islands() {
        let background = pack_rgb(2, 3, 7);
        let mut pixels = vec![background; (WW * WH) as usize];
        paint_meta_hud(
            &mut pixels,
            &Theme::default(),
            None,
            12_450,
            870,
            3.2,
            "THE COLLECTOR",
            "High Roller",
            17,
            0.65,
        );

        let changed = |x0: i32, x1: i32| {
            (12..108)
                .flat_map(|y| (x0..x1).map(move |x| (y * WW as i32 + x) as usize))
                .filter(|&index| pixels[index] != background)
                .count()
        };
        assert!(changed(10, 282) > 8_000, "profile island should be visible");
        assert!(
            changed(838, 1270) > 15_000,
            "resource island should be visible"
        );
        assert_eq!(
            pixels[(40 * WW + 640) as usize],
            background,
            "HUD must leave the center clear"
        );
    }
}
