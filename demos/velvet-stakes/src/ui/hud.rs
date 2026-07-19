//! Top HUD chrome matching Nightfall Casino reference:
//! profile card (portrait + name + rank + XP bar + level) and meta counters.

use crate::render::{outline, panel, text, RgbImage};
use crate::ui::theme::{Theme, WW, WH};
use velvet_story::pack_rgb;

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
        28,
        20,
        player_name,
        player_rank,
        level,
        xp_frac,
    );

    // Meta strip top-right
    let mx = WW as i32 - 390;
    panel(pixels, WW, WH, mx, 20, 360, 52, theme.panel, 0.78);
    outline(pixels, WW, WH, mx, 20, 360, 52, (120, 90, 50), 1);
    outline(pixels, WW, WH, mx + 2, 22, 356, 48, theme.neon, 1);
    text(
        pixels,
        WW,
        WH,
        mx + 16,
        36,
        &format!("CHIPS {chips}   CRY {crystals}   x{mult:.1}"),
        theme.gold_soft,
        1,
    );
}

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
    let w = 340;
    let h = 92;
    // Dark panel + gold outer frame
    panel(pixels, WW, WH, x, y, w, h, (12, 8, 22), 0.88);
    outline(pixels, WW, WH, x, y, w, h, (160, 120, 55), 1);
    outline(pixels, WW, WH, x + 2, y + 2, w - 4, h - 4, (90, 70, 140), 1);

    // Portrait well
    let px = x + 10;
    let py = y + 10;
    let ps = 72;
    panel(pixels, WW, WH, px - 2, py - 2, ps + 4, ps + 4, (40, 28, 70), 0.95);
    outline(pixels, WW, WH, px - 2, py - 2, ps + 4, ps + 4, (180, 140, 70), 1);
    if let Some(img) = portrait {
        crate::render::blit_card(pixels, WW, WH, img, px, py, ps, ps, 1.0);
    } else {
        // fallback silhouette
        panel(pixels, WW, WH, px, py, ps, ps, (30, 20, 50), 1.0);
        text(pixels, WW, WH, px + 18, py + 28, "??", theme.gold, 2);
    }

    let tx = px + ps + 14;
    text(pixels, WW, WH, tx, y + 16, name, theme.gold, 1);
    text(pixels, WW, WH, tx, y + 36, rank, theme.muted, 1);

    // XP bar with gem
    let bar_x = tx;
    let bar_y = y + 62;
    let bar_w = 160;
    let bar_h = 10;
    // gem
    paint_gem(pixels, bar_x - 2, bar_y - 2, 14);
    panel(
        pixels,
        WW,
        WH,
        bar_x + 14,
        bar_y,
        bar_w,
        bar_h,
        (30, 18, 40),
        0.95,
    );
    outline(
        pixels,
        WW,
        WH,
        bar_x + 14,
        bar_y,
        bar_w,
        bar_h,
        (100, 70, 130),
        1,
    );
    let fill = ((xp_frac.clamp(0.0, 1.0) * bar_w as f32) as i32).max(2);
    // magenta → gold gradient fill
    for col in 0..fill {
        let t = col as f32 / bar_w as f32;
        let r = (200.0 + 40.0 * t) as u8;
        let g = (60.0 + 100.0 * t) as u8;
        let b = (180.0 - 40.0 * t) as u8;
        panel(
            pixels,
            WW,
            WH,
            bar_x + 14 + col,
            bar_y + 1,
            1,
            bar_h - 2,
            (r, g, b),
            0.95,
        );
    }
    text(
        pixels,
        WW,
        WH,
        bar_x + 14 + bar_w + 10,
        bar_y - 2,
        &format!("Lvl. {level}"),
        theme.gold_soft,
        1,
    );
}

fn paint_gem(pixels: &mut [u32], x: i32, y: i32, size: i32) {
    let cx = x + size / 2;
    let cy = y + size / 2;
    let s = size / 2;
    for dy in -s..=s {
        let span = s - dy.abs();
        for dx in -span..=span {
            let px = cx + dx;
            let py = cy + dy;
            if px < 0 || py < 0 || px >= WW as i32 || py >= WH as i32 {
                continue;
            }
            let edge = dx.abs() + dy.abs();
            let bright = 1.0 - edge as f32 / (s as f32 * 1.5);
            let r = (180.0 + 60.0 * bright) as u8;
            let g = (40.0 + 80.0 * bright) as u8;
            let b = (200.0 + 40.0 * bright) as u8;
            let i = (py as u32 * WW + px as u32) as usize;
            pixels[i] = pack_rgb(r, g, b);
        }
    }
}
