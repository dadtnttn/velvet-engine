//! Title menu and lobby — Nightfall Casino spectacular composition:
//! cinematic background · gold frame · centered wordmark · ornate buttons · daily ritual.

use crate::logo::RgbaBuf;
use crate::render::{blit_card, blit_cover, fill, outline, panel, text, ArtBank, RgbImage};
use crate::title_font::{draw_font_text, measure_text, title_font, ui_font};
use crate::ui::buttons::{paint_button_column, ButtonColumnLayout, MenuInteraction};
use crate::ui::hud::paint_meta_hud;
use crate::ui::theme::{Theme, WH, WW};
use velvet_script_layers::ScreenBlueprint;
use velvet_style::{resolve, StyleQuery, Stylesheet};

/// Full title / lobby paint (reference-faithful chrome + spectacular polish).
// Public immediate-mode paint boundary; arguments mirror independent live UI inputs.
#[allow(clippy::too_many_arguments)]
pub fn paint_title_menu(
    pixels: &mut [u32],
    theme: &Theme,
    menu_bg: Option<&RgbImage>,
    _logo_title: Option<&RgbaBuf>,
    portrait: Option<&RgbImage>,
    sheet: &Stylesheet,
    screen: &ScreenBlueprint,
    menu_sel: usize,
    interaction: MenuInteraction,
    chips: i64,
    crystals: i64,
    mult: f32,
) {
    if let Some(bg) = menu_bg {
        blit_cover(pixels, WW, WH, bg);
    } else {
        fill(pixels, WW, WH, theme.void);
    }

    // Reference composition: deep left scrim for title/actions while the city,
    // bar and foreground card table remain unobstructed.
    paint_button_column_shade(pixels, 0, 104, 470, 558, 0.38);
    paint_top_vignette(pixels, 110, 0.30);
    paint_bottom_vignette(pixels, 64, 0.48);
    paint_navigation_shell(pixels, theme);
    paint_ambient_sparks(pixels, theme);

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

    paint_reference_title(pixels, sheet, &screen.title, &screen.subtitle);

    let layout = ButtonColumnLayout::from_style(sheet, screen);
    paint_button_column(pixels, theme, sheet, screen, &layout, menu_sel, interaction);
    paint_selection_preview(pixels, theme, screen, menu_sel);

    paint_daily_ritual(pixels, theme, sheet);
    paint_menu_footer(pixels, theme, screen);
}

/// One-line art-deco wordmark anchored above the left navigation column.
fn paint_reference_title(pixels: &mut [u32], sheet: &Stylesheet, title: &str, subtitle: &str) {
    const CX: i32 = 236;
    const RULE_Y: i32 = 125;
    let title = if title.is_empty() {
        "VELVET ARCANA"
    } else {
        title
    };
    let subtitle = if subtitle.is_empty() {
        "NIGHTFALL CASINO"
    } else {
        subtitle
    };
    let title_color = resolve(sheet, &StyleQuery::class("logo-title"))
        .color_text()
        .rgb_tuple();
    let subtitle_color = resolve(sheet, &StyleQuery::class("logo-sub"))
        .color_text()
        .rgb_tuple();

    paint_menu_text(
        pixels,
        53,
        123,
        "PRIVATE TABLE  •  NIGHT 17",
        9.5,
        (204, 151, 103),
        0.92,
        1,
    );
    panel(pixels, WW, WH, 39, 115, 5, 5, (234, 49, 169), 0.98);

    paint_gold_rule(pixels, 42, RULE_Y + 10, CX - 14, title_color);
    paint_mini_diamond(pixels, CX, RULE_Y + 10, title_color);
    paint_gold_rule(pixels, CX + 14, RULE_Y + 10, 430, title_color);

    if let Some(font) = title_font() {
        let mut size = 53.0;
        let measured = measure_text(font, title, size);
        if measured > 374.0 {
            size *= 374.0 / measured;
        }
        let width = measure_text(font, title, size);
        draw_font_text(
            pixels,
            font,
            CX as f32 - width * 0.5 + 1.0,
            196.0,
            title,
            size,
            (18, 6, 20),
            0.9,
        );
        draw_font_text(
            pixels,
            font,
            CX as f32 - width * 0.5,
            195.0,
            title,
            size,
            title_color,
            1.0,
        );

        let sub_size = 16.0;
        let sub_width = measure_text(font, subtitle, sub_size);
        draw_font_text(
            pixels,
            font,
            CX as f32 - sub_width * 0.5,
            226.0,
            subtitle,
            sub_size,
            subtitle_color,
            0.98,
        );
        let left = CX - sub_width.round() as i32 / 2;
        let right = CX + sub_width.round() as i32 / 2;
        paint_gold_rule(pixels, 54, 220, left - 14, subtitle_color);
        paint_mini_diamond(pixels, left - 8, 220, subtitle_color);
        paint_mini_diamond(pixels, right + 8, 220, subtitle_color);
        paint_gold_rule(pixels, right + 14, 220, 418, subtitle_color);
    } else {
        let title_w = estimate_text_w(title, 3);
        text(pixels, WW, WH, CX - title_w / 2, 155, title, title_color, 3);
        let subtitle_w = estimate_text_w(subtitle, 1);
        text(
            pixels,
            WW,
            WH,
            CX - subtitle_w / 2,
            210,
            subtitle,
            subtitle_color,
            1,
        );
    }
}

fn paint_navigation_shell(pixels: &mut [u32], theme: &Theme) {
    const X: i32 = 27;
    const Y: i32 = 104;
    const W: i32 = 420;
    const H: i32 = 558;

    panel(pixels, WW, WH, X + 6, Y + 7, W, H, (0, 0, 0), 0.52);
    panel(pixels, WW, WH, X, Y, W, H, (5, 3, 12), 0.76);
    outline(pixels, WW, WH, X, Y, W, H, (91, 53, 47), 1);
    outline(pixels, WW, WH, X + 4, Y + 4, W - 8, H - 8, (42, 28, 48), 1);

    for &(cx, cy, sx, sy) in &[
        (X + 12, Y + 12, 1, 1),
        (X + W - 13, Y + 12, -1, 1),
        (X + 12, Y + H - 13, 1, -1),
        (X + W - 13, Y + H - 13, -1, -1),
    ] {
        let hx = if sx > 0 { cx } else { cx - 18 };
        let vy = if sy > 0 { cy } else { cy - 18 };
        panel(pixels, WW, WH, hx, cy, 19, 1, theme.gold_soft, 0.78);
        panel(pixels, WW, WH, cx, vy, 1, 19, theme.gold_soft, 0.78);
        paint_mini_diamond(pixels, cx, cy, theme.gold_soft);
    }

    panel(
        pixels,
        WW,
        WH,
        X + W - 2,
        Y + 78,
        2,
        H - 156,
        theme.neon,
        0.24,
    );
    panel(
        pixels,
        WW,
        WH,
        X + W - 1,
        Y + 190,
        2,
        150,
        (234, 49, 169),
        0.52,
    );
}

fn paint_ambient_sparks(pixels: &mut [u32], theme: &Theme) {
    const SPARKS: &[(i32, i32, i32, f32)] = &[
        (476, 151, 2, 0.62),
        (517, 311, 1, 0.45),
        (611, 183, 1, 0.48),
        (706, 118, 2, 0.52),
        (788, 272, 1, 0.42),
        (835, 446, 2, 0.48),
        (1008, 168, 1, 0.50),
        (1136, 377, 1, 0.42),
        (1210, 211, 2, 0.44),
    ];
    for &(x, y, radius, alpha) in SPARKS {
        panel(
            pixels,
            WW,
            WH,
            x - radius * 3,
            y,
            radius * 6 + 1,
            1,
            theme.neon,
            alpha * 0.22,
        );
        panel(
            pixels,
            WW,
            WH,
            x,
            y - radius * 3,
            1,
            radius * 6 + 1,
            theme.neon,
            alpha * 0.22,
        );
        paint_mini_diamond(pixels, x, y, (226, 151, 91));
    }
}

fn paint_selection_preview(
    pixels: &mut [u32],
    theme: &Theme,
    screen: &ScreenBlueprint,
    selected: usize,
) {
    let Some(item) = screen.buttons.get(selected) else {
        return;
    };
    const X: i32 = 867;
    const Y: i32 = 527;
    const W: i32 = 370;
    const H: i32 = 118;

    panel(pixels, WW, WH, X + 4, Y + 5, W, H, (0, 0, 0), 0.48);
    panel(pixels, WW, WH, X, Y, W, H, (6, 4, 13), 0.82);
    outline(pixels, WW, WH, X, Y, W, H, (94, 54, 55), 1);
    outline(pixels, WW, WH, X + 4, Y + 4, W - 8, H - 8, (46, 30, 57), 1);

    panel(pixels, WW, WH, X, Y, 3, H, (234, 49, 169), 0.88);
    paint_mini_diamond(pixels, X + 1, Y + 18, theme.gold_soft);
    paint_mini_diamond(pixels, X + 1, Y + H - 18, theme.gold_soft);

    paint_menu_text(
        pixels,
        X + 24,
        Y + 25,
        "CURRENT SELECTION",
        9.5,
        (196, 139, 89),
        0.94,
        1,
    );
    paint_menu_text(
        pixels,
        X + 24,
        Y + 53,
        &item.label,
        18.0,
        theme.gold,
        1.0,
        2,
    );
    paint_menu_text(
        pixels,
        X + 24,
        Y + 78,
        &item.description,
        11.0,
        theme.text,
        0.92,
        1,
    );

    if !item.hotkey.is_empty() {
        panel(
            pixels,
            WW,
            WH,
            X + W - 82,
            Y + 17,
            58,
            25,
            (14, 8, 23),
            0.96,
        );
        outline(
            pixels,
            WW,
            WH,
            X + W - 82,
            Y + 17,
            58,
            25,
            theme.gold_soft,
            1,
        );
        paint_menu_text(
            pixels,
            X + W - 69,
            Y + 35,
            &item.hotkey,
            10.0,
            theme.gold,
            1.0,
            1,
        );
    }

    paint_menu_text(
        pixels,
        X + 24,
        Y + 103,
        "SELECT TO ENTER",
        9.5,
        (176, 121, 194),
        0.88,
        1,
    );
}

fn paint_menu_footer(pixels: &mut [u32], theme: &Theme, screen: &ScreenBlueprint) {
    let footer = if screen.footer.is_empty() {
        "“FORTUNE FAVORS THE BOLD.”"
    } else {
        &screen.footer
    };
    const X: i32 = 15;
    const Y: i32 = 676;
    const W: i32 = 1250;
    const H: i32 = 36;
    panel(pixels, WW, WH, X + 2, Y + 2, W, H, (0, 0, 0), 0.55);
    panel(pixels, WW, WH, X, Y, W, H, (5, 4, 11), 0.88);
    outline(pixels, WW, WH, X, Y, W, H, (78, 48, 42), 1);
    paint_mini_diamond(pixels, X + 20, Y + H / 2, (166, 101, 58));
    paint_menu_text(
        pixels,
        X + 35,
        Y + 23,
        footer,
        12.0,
        (198, 139, 89),
        0.95,
        1,
    );
    paint_menu_text(
        pixels,
        X + W - 276,
        Y + 23,
        "↑↓ / W S  NAVIGATE    ENTER  CONFIRM",
        10.5,
        theme.muted,
        0.88,
        1,
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_menu_text(
    pixels: &mut [u32],
    x: i32,
    baseline: i32,
    value: &str,
    size: f32,
    color: (u8, u8, u8),
    opacity: f32,
    fallback_scale: i32,
) {
    if let Some(font) = ui_font() {
        draw_font_text(
            pixels,
            font,
            x as f32,
            baseline as f32,
            value,
            size,
            color,
            opacity,
        );
    } else {
        text(
            pixels,
            WW,
            WH,
            x,
            baseline - fallback_scale * 7,
            value,
            color,
            fallback_scale,
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

    let w = daily.number("width", 314.0).round() as i32;
    let h = daily.number("height", 69.0).round() as i32;
    let x = daily.number("x", 31.0).round() as i32;
    let y = daily.number("y", 538.0).round() as i32;
    let track = daily
        .color("progress-track", velvet_style::Color::rgb(35, 25, 54))
        .rgb_tuple();
    let progress = daily
        .color("progress-fill", velvet_style::Color::rgb(215, 90, 220))
        .rgb_tuple();

    panel(pixels, WW, WH, x + 3, y + 4, w, h, (0, 0, 0), 0.56);
    panel(pixels, WW, WH, x, y, w, h, bg, 0.93);
    outline(pixels, WW, WH, x, y, w, h, border, 1);
    paint_mini_diamond(pixels, x + 3, y + h / 2, border);
    paint_mini_diamond(pixels, x + w - 4, y + h / 2, border);
    paint_ritual_star(pixels, x + 27, y + 36, theme.neon, gold);

    paint_menu_text(pixels, x + 57, y + 19, "DAILY RITUAL", 10.0, gold, 1.0, 1);
    paint_menu_text(pixels, x + 57, y + 42, "PLAY 3 HANDS", 13.0, fg, 1.0, 1);
    panel(pixels, WW, WH, x + 57, y + 51, 104, 6, track, 0.96);
    panel(pixels, WW, WH, x + 57, y + 51, 69, 6, progress, 1.0);
    outline(pixels, WW, WH, x + 57, y + 51, 104, 6, border, 1);
    paint_menu_text(pixels, x + 170, y + 58, "2 / 3", 10.5, fg, 0.94, 1);

    panel(pixels, WW, WH, x + 223, y + 10, 1, h - 20, border, 0.55);
    paint_menu_text(pixels, x + 242, y + 19, "REWARD", 9.5, gold, 0.95, 1);
    paint_reward_crystal(pixels, x + 244, y + 34, 11, 21, theme.neon);
    paint_menu_text(pixels, x + 267, y + 53, "150", 15.0, fg, 1.0, 1);
}

fn paint_ritual_star(
    pixels: &mut [u32],
    cx: i32,
    cy: i32,
    violet: (u8, u8, u8),
    gold: (u8, u8, u8),
) {
    for radius in (3..18).rev() {
        let alpha = (18 - radius) as f32 / 70.0;
        panel(
            pixels,
            WW,
            WH,
            cx - radius,
            cy - 1,
            radius * 2 + 1,
            3,
            violet,
            alpha,
        );
        panel(
            pixels,
            WW,
            WH,
            cx - 1,
            cy - radius,
            3,
            radius * 2 + 1,
            violet,
            alpha,
        );
    }
    paint_mini_diamond(pixels, cx, cy, gold);
}

fn paint_reward_crystal(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, violet: (u8, u8, u8)) {
    let cx = x + w / 2;
    for row in 0..h {
        let distance = (row - h / 2).abs();
        let span = ((h / 2 - distance) * w / h.max(1)).max(1);
        panel(
            pixels,
            WW,
            WH,
            cx - span,
            y + row,
            span * 2 + 1,
            1,
            violet,
            0.95,
        );
    }
    panel(pixels, WW, WH, cx, y + 2, 1, h - 4, (255, 189, 240), 0.9);
}

/// Darken only the button column rectangle (not the logo zone).
fn paint_button_column_shade(pixels: &mut [u32], x: i32, y: i32, w: i32, h: i32, strength: f32) {
    for row in y.max(0)..(y + h).min(WH as i32) {
        let vy = (row - y) as f32 / h.max(1) as f32;
        let edge_y = (1.0 - (vy - 0.5).abs() * 1.4).clamp(0.35, 1.0);
        for col in x.max(0)..(x + w).min(WW as i32) {
            let vx = (col - x) as f32 / w.max(1) as f32;
            let edge_x = (1.0 - vx * 0.85).clamp(0.0, 1.0);
            let a = strength * edge_x * edge_y;
            let i = (row as u32 * WW + col as u32) as usize;
            pixels[i] = blend_dark(pixels[i], a);
        }
    }
}

fn paint_top_vignette(pixels: &mut [u32], height: i32, strength: f32) {
    for y in 0..height {
        let a = (1.0 - y as f32 / height as f32) * strength;
        for x in 0..WW as i32 {
            let i = (y as u32 * WW + x as u32) as usize;
            pixels[i] = blend_dark(pixels[i], a * 0.65);
        }
    }
}

fn paint_bottom_vignette(pixels: &mut [u32], height: i32, strength: f32) {
    for y in 0..height {
        let a = (1.0 - y as f32 / height as f32) * strength;
        let py = WH as i32 - 1 - y;
        for x in 0..WW as i32 {
            let i = (py as u32 * WW + x as u32) as usize;
            pixels[i] = blend_dark(pixels[i], a * 0.55);
        }
    }
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
    paint_modal_shell(
        pixels,
        theme,
        bg,
        "COLLECTION",
        "Illustrated set — own originals",
    );
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
        text(pixels, WW, WH, 240, 260 + i as i32 * 28, l, theme.text, 1);
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
    outline(pixels, WW, WH, 204, 124, 872, 432, theme.gold, 1);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::load_rgb;
    use std::path::PathBuf;
    use velvet_style::parse_stylesheet;

    fn data_ui() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui")
    }

    fn data_style() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/styles/casino.vcss")
    }

    fn load_sheet() -> Stylesheet {
        let src = std::fs::read_to_string(data_style()).expect("casino.vcss");
        parse_stylesheet(&src).expect("parse vcss")
    }

    fn paint_frame(sel: usize) -> Vec<u32> {
        let ui = data_ui();
        let bg = load_rgb(&ui.join("menu_bg_city.png"));
        let logo = crate::logo::load_title_wordmark(&ui.join("logo_title.png"));
        let portrait = load_rgb(&ui.join("portrait_collector.jpg"));
        assert!(
            bg.is_some(),
            "menu_bg_city.png must exist for title paint tests"
        );
        assert!(logo.is_some(), "logo_title.png must load for title paint");
        let soft = crate::count_soft_alpha(&logo.as_ref().unwrap().3);
        assert!(
            soft > 80,
            "logo should have soft alpha edges (not square), soft={soft}"
        );
        let sheet = load_sheet();
        let screen = crate::live_dev::reload_screen(&ui.join("main_menu.vel"))
            .expect("parse VS2 title menu");
        let theme = Theme::default();
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_title_menu(
            &mut pixels,
            &theme,
            bg.as_ref(),
            logo.as_ref(),
            portrait.as_ref(),
            &sheet,
            &screen,
            sel,
            MenuInteraction::default(),
            12_450,
            870,
            3.2,
        );
        pixels
    }

    fn non_void_fraction(pixels: &[u32], void: (u8, u8, u8)) -> f32 {
        let void_c = velvet_story::pack_rgb(void.0, void.1, void.2);
        let mut non = 0usize;
        for &p in pixels {
            // near-void: very dark and close to theme void
            let r = ((p >> 16) & 0xFF) as i32;
            let g = ((p >> 8) & 0xFF) as i32;
            let b = (p & 0xFF) as i32;
            let vr = ((void_c >> 16) & 0xFF) as i32;
            let vg = ((void_c >> 8) & 0xFF) as i32;
            let vb = (void_c & 0xFF) as i32;
            let dark = r + g + b < 45;
            let near_void = (r - vr).abs() + (g - vg).abs() + (b - vb).abs() < 30;
            if !(dark && near_void) {
                non += 1;
            }
        }
        non as f32 / pixels.len() as f32
    }

    fn region_hash(pixels: &[u32], x0: u32, y0: u32, x1: u32, y1: u32) -> u64 {
        let mut h: u64 = 1469598103934665603;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = pixels[(y * WW + x) as usize] as u64;
                h ^= p;
                h = h.wrapping_mul(1099511628211);
            }
        }
        h
    }

    #[test]
    fn title_paint_fills_frame_and_left_wordmark_band() {
        let pixels = paint_frame(0);
        assert_eq!(pixels.len(), (WW * WH) as usize);
        let frac = non_void_fraction(&pixels, Theme::default().void);
        assert!(
            frac > 0.35,
            "title frame should be substantially filled, frac={frac}"
        );
        // Reference wordmark is a one-line serif title anchored at the left.
        let mut copperish = 0usize;
        let y0 = 115u32;
        let y1 = 230u32;
        let x0 = 20u32;
        let x1 = 440u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let p = pixels[(y * WW + x) as usize];
                let r = ((p >> 16) & 0xFF) as u32;
                let g = ((p >> 8) & 0xFF) as u32;
                let b = (p & 0xFF) as u32;
                if r > 120 && g > 70 && r > b {
                    copperish += 1;
                }
            }
        }
        assert!(
            copperish > 400,
            "left wordmark copper pixels expected, got {copperish}"
        );
    }

    #[test]
    fn title_selection_changes_button_region() {
        let a = paint_frame(0);
        let b = paint_frame(2);
        assert_ne!(a, b, "different selection must change buffer");
        // Button column region (left side)
        let ha = region_hash(&a, 60, 235, 410, 520);
        let hb = region_hash(&b, 60, 235, 410, 520);
        assert_ne!(
            ha, hb,
            "button column region hash must differ across selection"
        );
    }

    #[test]
    fn title_uses_font_wordmark_not_svg_asset() {
        // Font path is preferred; SVG wordmark module removed
        let _ = crate::title_font::TITLE_LINE1;
        let _ = crate::title_font::TITLE_LINE2;
        let mut pixels = vec![0u32; (WW * WH) as usize];
        let (_x, _y, w, h) = crate::title_font::paint_title_wordmark(
            &mut pixels,
            700,
            110,
            (232, 192, 120),
            (240, 210, 160),
        );
        assert!(w > 60 && h > 30);
    }

    #[test]
    #[ignore = "manual visual evidence; run explicitly with --ignored"]
    fn dump_title_menu_png_for_evidence() {
        let pixels = paint_frame(0);
        // Optional dump path via env; always write under target/ for local inspect
        let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
        let _ = std::fs::create_dir_all(&out_dir);
        let path = out_dir.join("title_menu_paint.png");
        write_rgb_png(&path, WW, WH, &pixels);
        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 10_000);
    }

    fn write_rgb_png(path: &std::path::Path, w: u32, h: u32, pixels: &[u32]) {
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for &p in pixels {
            let r = ((p >> 16) & 0xFF) as u8;
            let g = ((p >> 8) & 0xFF) as u8;
            let b = (p & 0xFF) as u8;
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
        image::save_buffer(path, &rgba, w, h, image::ColorType::Rgba8).expect("write png");
    }
}
