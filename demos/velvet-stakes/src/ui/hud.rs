//! Top HUD chrome (profile + chips / crystals / mult).

use crate::render::{outline, panel, text};
use crate::ui::theme::{Theme, WW, WH};

/// Paint collector profile + meta counters.
pub fn paint_meta_hud(
    pixels: &mut [u32],
    theme: &Theme,
    chips: i64,
    crystals: i64,
    mult: f32,
    player_name: &str,
    player_rank: &str,
) {
    // Profile
    panel(pixels, WW, WH, 28, 22, 280, 58, theme.panel, 0.78);
    outline(pixels, WW, WH, 28, 22, 280, 58, theme.neon, 1);
    text(pixels, WW, WH, 42, 32, player_name, theme.gold, 1);
    text(pixels, WW, WH, 42, 52, player_rank, theme.muted, 1);

    // Meta strip
    panel(pixels, WW, WH, WW as i32 - 380, 18, 350, 48, theme.panel, 0.75);
    outline(
        pixels,
        WW,
        WH,
        WW as i32 - 380,
        18,
        350,
        48,
        theme.neon,
        1,
    );
    text(
        pixels,
        WW,
        WH,
        WW as i32 - 360,
        32,
        &format!("CHIPS {chips}  ·  CRY {crystals}  ·  x{mult:.1}"),
        theme.text,
        1,
    );
}
