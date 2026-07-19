//! Title menu and lobby — layout matching Nightfall Casino reference art:
//! profile HUD · centered logo wordmark · ornate buttons · daily ritual.

use crate::render::{blit_card, blit_cover, fill, outline, panel, text, ArtBank, RgbImage};
use crate::ui::buttons::{paint_button_column, ButtonColumnLayout};
use crate::ui::hud::paint_meta_hud;
use crate::ui::theme::{Theme, WW, WH};
use velvet_stakes::{blit_rgba_bilinear, RgbaBuf};
use velvet_style::{resolve, StyleQuery, Stylesheet};

/// Full title / lobby paint (reference-faithful chrome).
pub fn paint_title_menu(
    pixels: &mut [u32],
    theme: &Theme,
    menu_bg: Option<&RgbImage>,
    logo_title: Option<&RgbaBuf>,
    portrait: Option<&RgbImage>,
    sheet: &Stylesheet,
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

    // Soft left vignette so buttons stay readable without crushing the logo
    for x in 0..420 {
        let a = (1.0 - x as f32 / 420.0) * 0.42;
        for y in 160..WH as i32 {
            let i = (y as u32 * WW + x as u32) as usize;
            pixels[i] = blend_dark(pixels[i], a);
        }
    }
    // Top darken for HUD
    for y in 0..120 {
        let a = (1.0 - y as f32 / 120.0) * 0.35;
        for x in 0..WW as i32 {
            let i = (y as u32 * WW + x as u32) as usize;
            pixels[i] = blend_dark(pixels[i], a * 0.6);
        }
    }

    paint_meta_hud(
        pixels,
        theme,
        portrait,
        chips,
        crystals,
        mult,
        "The Collector",
        "High Roller",
        17,
        0.62,
    );

    // Wordmark image only — no procedural title letters
    paint_centered_logo_title(pixels, theme, logo_title, sheet);

    // Buttons lower-left (below wordmark)
    let layout = ButtonColumnLayout {
        x: 48,
        y0: 340,
        w: 400,
        h: 50,
        gap: 10,
    };
    paint_button_column(pixels, theme, sheet, &layout, menu_sel);

    paint_daily_ritual(pixels, theme, sheet);
}

/// Centered elegant wordmark (black background burned out via alpha).
fn paint_centered_logo_title(
    pixels: &mut [u32],
    theme: &Theme,
    logo_title: Option<&RgbaBuf>,
    sheet: &Stylesheet,
) {
    let cx = WW as i32 / 2;

    if let Some(logo) = logo_title {
        // Fit wordmark across center of lobby (wide copper title)
        let max_w = 640i32;
        let max_h = 200i32;
        let (sw, sh, _, _) = *logo;
        let scale = (max_w as f32 / sw as f32).min(max_h as f32 / sh as f32);
        let dw = (sw as f32 * scale) as i32;
        let dh = (sh as f32 * scale) as i32;
        let dx = cx - dw / 2;
        let dy = 108;
        // Bilinear filter — smooth serifs (no square pixel corners)
        blit_rgba_bilinear(pixels, WW, WH, logo, dx, dy, dw, dh, 1.0);

        // Subtitle sits just under the image wordmark
        let sub_style = resolve(sheet, &StyleQuery::class("logo-sub"));
        let sub_col = sub_style
            .props
            .get("color")
            .and_then(|v| v.as_color())
            .map(|c| c.rgb_tuple())
            .unwrap_or(theme.gold_soft);
        let sub = "NIGHTFALL CASINO";
        let sub_w = estimate_text_w(sub, 1);
        let sx = cx - sub_w / 2;
        let sy = dy + dh - 8;
        let rule_y = sy + 6;
        paint_gold_rule(pixels, sx - 80, rule_y, sx - 14, theme.gold);
        paint_mini_diamond(pixels, sx - 10, rule_y, theme.gold);
        text(pixels, WW, WH, sx, sy, sub, sub_col, 1);
        paint_mini_diamond(pixels, sx + sub_w + 8, rule_y, theme.gold);
        paint_gold_rule(pixels, sx + sub_w + 16, rule_y, sx + sub_w + 80, theme.gold);
    } else {
        text(
            pixels,
            WW,
            WH,
            cx - 80,
            180,
            "(logo_title missing)",
            theme.muted,
            1,
        );
    }
}

fn paint_daily_ritual(pixels: &mut [u32], theme: &Theme, sheet: &Stylesheet) {
    let daily = resolve(sheet, &StyleQuery::class("daily"));
    let bg = daily.background().rgb_tuple();
    let border = daily.border_color().rgb_tuple();
    let fg = daily.color_text().rgb_tuple();
    let gold = daily
        .props
        .get("gold")
        .and_then(|v| v.as_color())
        .map(|c| c.rgb_tuple())
        .unwrap_or(theme.gold_soft);

    let x = 40;
    let y = WH as i32 - 100;
    let w = 360;
    let h = 58;
    panel(pixels, WW, WH, x, y, w, h, bg, 0.78);
    outline(pixels, WW, WH, x, y, w, h, border, 1);
    // corner diamonds
    paint_mini_diamond(pixels, x + 8, y + 8, gold);
    paint_mini_diamond(pixels, x + w - 8, y + 8, gold);
    paint_mini_diamond(pixels, x + 8, y + h - 8, gold);
    paint_mini_diamond(pixels, x + w - 8, y + h - 8, gold);

    text(pixels, WW, WH, x + 18, y + 12, "Daily Ritual  ·  Play 3 Hands", fg, 1);
    text(pixels, WW, WH, x + 18, y + 34, "REWARD  150 crystals", gold, 1);

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

fn paint_gold_rule(pixels: &mut [u32], x0: i32, y: i32, x1: i32, gold: (u8, u8, u8)) {
    let (x0, x1) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
    for x in x0..=x1 {
        put(pixels, x, y, gold, 0.85);
        put(pixels, x, y + 1, gold, 0.4);
    }
}

fn paint_mini_diamond(pixels: &mut [u32], cx: i32, cy: i32, gold: (u8, u8, u8)) {
    let size: i32 = 4;
    for dy in -size..=size {
        let span = size - dy.abs();
        for dx in -span..=span {
            put(pixels, cx + dx, cy + dy, gold, 0.95);
        }
    }
}

fn estimate_text_w(s: &str, scale: i32) -> i32 {
    // softbuffer bitmap font ~6px wide per glyph at scale 1
    s.chars().count() as i32 * 6 * scale
}

fn put(pixels: &mut [u32], x: i32, y: i32, rgb: (u8, u8, u8), a: f32) {
    if x < 0 || y < 0 || x >= WW as i32 || y >= WH as i32 {
        return;
    }
    let i = (y as u32 * WW + x as u32) as usize;
    let src = velvet_story::pack_rgb(rgb.0, rgb.1, rgb.2);
    pixels[i] = blend(pixels[i], src, a);
}

fn blend(dst: u32, src: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let dr = ((dst >> 16) & 0xFF) as f32;
    let dg = ((dst >> 8) & 0xFF) as f32;
    let db = (dst & 0xFF) as f32;
    let sr = ((src >> 16) & 0xFF) as f32;
    let sg = ((src >> 8) & 0xFF) as f32;
    let sb = (src & 0xFF) as f32;
    velvet_story::pack_rgb(
        (dr + (sr - dr) * t) as u8,
        (dg + (sg - dg) * t) as u8,
        (db + (sb - db) * t) as u8,
    )
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

/// Shop stub.
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
        "Flow: .vstory   Style: .vcss dealHand",
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
