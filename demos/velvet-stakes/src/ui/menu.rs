//! Title menu and lobby sub-screens (Velvet Arcana).

use crate::render::{
    blit_card, blit_cover, fill, outline, panel, text, ArtBank, RgbImage,
};
use crate::ui::hud::paint_meta_hud;
use crate::ui::theme::{MenuItem, Theme, TITLE_ITEMS, WW, WH};

/// Full title / lobby paint.
pub fn paint_title_menu(
    pixels: &mut [u32],
    theme: &Theme,
    menu_bg: Option<&RgbImage>,
    menu_panel: Option<&RgbImage>,
    logo: Option<&RgbImage>,
    menu_sel: usize,
    chips: i64,
    crystals: i64,
    mult: f32,
) {
    if let Some(bg) = menu_bg {
        blit_cover(pixels, WW, WH, bg);
    } else {
        fill(pixels, WW, WH, theme.void);
    }

    // Left readability vignette
    for x in 0..460 {
        let a = (1.0 - x as f32 / 460.0) * 0.5;
        for y in 0..WH as i32 {
            let i = (y as u32 * WW + x as u32) as usize;
            pixels[i] = blend_dark(pixels[i], a);
        }
    }

    paint_meta_hud(
        pixels,
        theme,
        chips,
        crystals,
        mult,
        "The Collector",
        "High Roller  ·  Lvl 17",
    );

    // Logo emblem
    if let Some(lg) = logo {
        blit_card(pixels, WW, WH, lg, 48, 88, 72, 72, 0.95);
    }

    text(pixels, WW, WH, 130, 100, "VELVET ARCANA", theme.gold, 3);
    text(
        pixels,
        WW,
        WH,
        132,
        142,
        "NIGHTFALL CASINO",
        theme.neon,
        1,
    );

    // Optional panel texture behind buttons
    let btn_x = 48;
    let btn_w = 360;
    let btn_h = 50;
    let btn_y0 = 200;
    let panel_h = TITLE_ITEMS.len() as i32 * (btn_h + 12) + 16;
    if let Some(p) = menu_panel {
        blit_card(
            pixels,
            WW,
            WH,
            p,
            btn_x - 12,
            btn_y0 - 12,
            btn_w + 24,
            panel_h + 8,
            0.55,
        );
    }

    for (i, item) in TITLE_ITEMS.iter().enumerate() {
        paint_menu_button(
            pixels,
            theme,
            btn_x,
            btn_y0 + i as i32 * (btn_h + 12),
            btn_w,
            btn_h,
            item,
            i == menu_sel,
        );
    }

    // Daily ritual
    panel(
        pixels,
        WW,
        WH,
        40,
        WH as i32 - 96,
        340,
        56,
        theme.panel,
        0.72,
    );
    outline(
        pixels,
        WW,
        WH,
        40,
        WH as i32 - 96,
        340,
        56,
        theme.neon,
        1,
    );
    text(
        pixels,
        WW,
        WH,
        54,
        WH as i32 - 84,
        "Daily Ritual  ·  Play 3 Hands",
        theme.text,
        1,
    );
    text(
        pixels,
        WW,
        WH,
        54,
        WH as i32 - 64,
        "REWARD  150 crystals",
        theme.gold_soft,
        1,
    );
    text(
        pixels,
        WW,
        WH,
        48,
        WH as i32 - 28,
        "\"FORTUNE FAVORS THE BOLD.\"",
        theme.muted,
        1,
    );
}

fn paint_menu_button(
    pixels: &mut [u32],
    theme: &Theme,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    item: &MenuItem,
    selected: bool,
) {
    let fill_c = if selected {
        theme.panel_sel
    } else {
        theme.panel
    };
    let border = if selected {
        theme.gold
    } else {
        theme.neon
    };
    panel(
        pixels,
        WW,
        WH,
        x,
        y,
        w,
        h,
        fill_c,
        if selected { 0.9 } else { 0.78 },
    );
    outline(
        pixels,
        WW,
        WH,
        x,
        y,
        w,
        h,
        border,
        if selected { 2 } else { 1 },
    );
    let label = if selected {
        format!(">  {}", item.label)
    } else {
        format!("   {}", item.label)
    };
    text(
        pixels,
        WW,
        WH,
        x + 20,
        y + 14,
        &label,
        if selected {
            theme.gold
        } else {
            theme.text
        },
        2,
    );
}

/// Collection screen with card art strip.
pub fn paint_collection(pixels: &mut [u32], theme: &Theme, bg: Option<&RgbImage>, art: &ArtBank) {
    paint_modal_shell(pixels, theme, bg, "COLLECTION", "Illustrated set — own originals");
    let ids = ["strike", "guard", "fireball", "focus", "bash"];
    for (i, id) in ids.iter().enumerate() {
        if let Some(img) = art.images.get(*id) {
            let x = 240 + i as i32 * 150;
            blit_card(pixels, WW, WH, img, x, 280, 130, 180, 1.0);
            text(pixels, WW, WH, x + 8, 470, id, theme.gold_soft, 1);
        }
    }
    text(
        pixels,
        WW,
        WH,
        240,
        510,
        "Enter / Esc = lobby",
        theme.muted,
        1,
    );
}

/// Shop stub (personalized copy).
pub fn paint_shop(pixels: &mut [u32], theme: &Theme, bg: Option<&RgbImage>) {
    paint_modal_shell(
        pixels,
        theme,
        bg,
        "SHOP",
        "Night market — packs and foils (compose with velvet tools)",
    );
    text(
        pixels,
        WW,
        WH,
        240,
        300,
        "Coming modules: pack generator, foil UV, price tables",
        theme.text,
        1,
    );
    text(
        pixels,
        WW,
        WH,
        240,
        510,
        "Enter / Esc = lobby",
        theme.muted,
        1,
    );
}

/// Options / how to play.
pub fn paint_options(pixels: &mut [u32], theme: &Theme, bg: Option<&RgbImage>) {
    paint_modal_shell(pixels, theme, bg, "OPTIONS", "How to play Velvet Arcana");
    let lines = [
        "1-8 select cards   P play hand   D discard",
        "Beat blind TARGET with CHIPS x MULT combos",
        "Focus in a play draws an extra card",
        "Deal animation via velvet-anim Timeline tools",
        "",
        "Enter / Esc = lobby",
    ];
    for (i, l) in lines.iter().enumerate() {
        text(
            pixels,
            WW,
            WH,
            240,
            260 + i as i32 * 28,
            l,
            theme.text,
            1,
        );
    }
}

fn paint_modal_shell(
    pixels: &mut [u32],
    theme: &Theme,
    bg: Option<&RgbImage>,
    title: &str,
    subtitle: &str,
) {
    if let Some(b) = bg {
        blit_cover(pixels, WW, WH, b);
    } else {
        fill(pixels, WW, WH, theme.void);
    }
    panel(pixels, WW, WH, 200, 120, 880, 440, theme.panel, 0.9);
    outline(pixels, WW, WH, 200, 120, 880, 440, theme.neon, 2);
    text(pixels, WW, WH, 240, 150, title, theme.gold, 3);
    text(pixels, WW, WH, 240, 200, subtitle, theme.muted, 1);
}

fn blend_dark(dst: u32, a: f32) -> u32 {
    let a = a.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    velvet_story::pack_rgb(
        (dr * (1.0 - a * 0.85)) as u8,
        (dg * (1.0 - a * 0.85)) as u8,
        (db * (1.0 - a * 0.7)) as u8,
    )
}
