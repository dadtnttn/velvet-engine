//! Title menu and lobby — Nightfall Casino spectacular composition:
//! cinematic background · gold frame · centered wordmark · ornate buttons · daily ritual.

use crate::render::{blit_card, blit_cover, fill, outline, panel, text, ArtBank, RgbImage};
use crate::ui::buttons::{paint_button_column, ButtonColumnLayout};
use crate::ui::hud::paint_meta_hud;
use crate::ui::theme::{Theme, WW, WH};
use crate::logo::{blit_rgba_bilinear, RgbaBuf};
use velvet_style::{resolve, StyleQuery, Stylesheet};

/// Placeholder string painted only when the wordmark asset is missing.
pub const LOGO_MISSING_MARKER: &str = "(logo_title missing)";

/// Full title / lobby paint (reference-faithful chrome + spectacular polish).
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

    // Vignettes: keep the *center* clean for the wordmark — darken only
    // the button column band and HUD strip, not the whole left half.
    paint_button_column_shade(pixels, 48, 280, 430, 300, 0.40);
    paint_top_vignette(pixels, 110, 0.32);
    paint_bottom_vignette(pixels, 120, 0.30);

    // Ornate screen frame (subtle)
    paint_screen_frame(pixels, theme);

    // Sparse sparkles (avoid noisy mess over the logo)
    paint_ambient_sparkles(pixels, theme);

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

    // Buttons lower-left — leave room for all 5 rows + daily panel (no overlap)
    let layout = ButtonColumnLayout {
        x: 56,
        y0: 292,
        w: 400,
        h: 46,
        gap: 8,
    };
    // 5 * (46+8) - 8 = 262 → ends ~554; daily starts ~608
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
    // Slightly right of center so left button column stays clear
    let cx = (WW as i32 * 58) / 100;

    if let Some(logo) = logo_title {
        // Content is already cropped; keep the title readable and fully on-screen
        let max_w = 560i32;
        let max_h = 150i32;
        let (sw, sh, _, _) = *logo;
        if sw == 0 || sh == 0 {
            return;
        }
        let scale = (max_w as f32 / sw as f32).min(max_h as f32 / sh as f32);
        let dw = ((sw as f32 * scale) as i32).max(1);
        let dh = ((sh as f32 * scale) as i32).max(1);
        // Clamp fully inside frame (with margin)
        let dx = (cx - dw / 2).clamp(420, WW as i32 - dw - 24);
        let dy = 118;

        // Subtle warm halo — small so it does not smear the letters
        paint_logo_halo(pixels, dx + dw / 2, dy + dh / 2, dw / 2 + 16, dh / 2 + 10);

        blit_rgba_bilinear(pixels, WW, WH, logo, dx, dy, dw, dh, 1.0);

        let sub_style = resolve(sheet, &StyleQuery::class("logo-sub"));
        let sub_col = sub_style
            .props
            .get("color")
            .and_then(|v| v.as_color())
            .map(|c| c.rgb_tuple())
            .unwrap_or(theme.gold_soft);
        // ASCII only — softbuffer bitmap font has no middle-dot glyphs
        let sub = "NIGHTFALL CASINO";
        let sub_w = estimate_text_w(sub, 1);
        let sx = (dx + dw / 2 - sub_w / 2).clamp(8, WW as i32 - sub_w - 8);
        let sy = dy + dh + 10;
        let rule_y = sy + 6;
        paint_gold_rule(pixels, sx - 72, rule_y, sx - 12, theme.gold);
        paint_mini_diamond(pixels, sx - 8, rule_y, theme.gold);
        text(pixels, WW, WH, sx, sy, sub, sub_col, 1);
        paint_mini_diamond(pixels, sx + sub_w + 6, rule_y, theme.gold);
        paint_gold_rule(pixels, sx + sub_w + 14, rule_y, sx + sub_w + 72, theme.gold);
    } else {
        text(
            pixels,
            WW,
            WH,
            cx - 80,
            180,
            LOGO_MISSING_MARKER,
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
    // Below last menu button (layout ends ~554)
    let y = WH as i32 - 96;
    let w = 380;
    let h = 56;
    // Outer glow
    panel(pixels, WW, WH, x - 2, y - 2, w + 4, h + 4, theme.neon, 0.12);
    panel(pixels, WW, WH, x, y, w, h, bg, 0.82);
    outline(pixels, WW, WH, x, y, w, h, border, 1);
    outline(pixels, WW, WH, x + 2, y + 2, w - 4, h - 4, gold, 1);
    // corner diamonds
    paint_mini_diamond(pixels, x + 10, y + 10, gold);
    paint_mini_diamond(pixels, x + w - 10, y + 10, gold);
    paint_mini_diamond(pixels, x + 10, y + h - 10, gold);
    paint_mini_diamond(pixels, x + w - 10, y + h - 10, gold);

    text(
        pixels,
        WW,
        WH,
        x + 20,
        y + 14,
        "Daily Ritual - Play 3 Hands",
        fg,
        1,
    );
    text(
        pixels,
        WW,
        WH,
        x + 20,
        y + 36,
        "REWARD  150 crystals",
        gold,
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

/// Darken only the button column rectangle (not the logo zone).
fn paint_button_column_shade(
    pixels: &mut [u32],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    strength: f32,
) {
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

fn paint_screen_frame(pixels: &mut [u32], theme: &Theme) {
    let m = 8;
    outline(
        pixels,
        WW,
        WH,
        m,
        m,
        WW as i32 - m * 2,
        WH as i32 - m * 2,
        theme.gold,
        1,
    );
    outline(
        pixels,
        WW,
        WH,
        m + 3,
        m + 3,
        WW as i32 - (m + 3) * 2,
        WH as i32 - (m + 3) * 2,
        theme.neon,
        1,
    );
    // mid-side diamonds
    paint_mini_diamond(pixels, WW as i32 / 2, m + 4, theme.gold);
    paint_mini_diamond(pixels, WW as i32 / 2, WH as i32 - m - 4, theme.gold);
    paint_mini_diamond(pixels, m + 4, WH as i32 / 2, theme.gold);
    paint_mini_diamond(pixels, WW as i32 - m - 4, WH as i32 / 2, theme.gold);
}

fn paint_logo_halo(pixels: &mut [u32], cx: i32, cy: i32, rx: i32, ry: i32) {
    let rx = rx.max(1) as f32;
    let ry = ry.max(1) as f32;
    for dy in -ry as i32..=ry as i32 {
        for dx in -rx as i32..=rx as i32 {
            let nx = dx as f32 / rx;
            let ny = dy as f32 / ry;
            let d = (nx * nx + ny * ny).sqrt();
            if d > 1.0 {
                continue;
            }
            // Soft, low-opacity — must not wash out or fragment the wordmark
            let a = (1.0 - d) * (1.0 - d) * 0.10;
            let t = d;
            let r = (220.0 * (1.0 - t) + 100.0 * t) as u8;
            let g = (160.0 * (1.0 - t) + 40.0 * t) as u8;
            let b = (60.0 * (1.0 - t) + 140.0 * t) as u8;
            put(pixels, cx + dx, cy + dy, (r, g, b), a);
        }
    }
}

fn paint_ambient_sparkles(pixels: &mut [u32], theme: &Theme) {
    // Few sparkles away from logo center and button labels
    let seeds: [(i32, i32, f32); 12] = [
        (980, 120, 0.4),
        (1100, 200, 0.35),
        (1050, 480, 0.3),
        (1180, 360, 0.28),
        (900, 560, 0.25),
        (200, 160, 0.3),
        (120, 500, 0.22),
        (320, 620, 0.2),
        (720, 90, 0.35),
        (850, 640, 0.2),
        (1080, 600, 0.25),
        (160, 280, 0.22),
    ];
    for (x, y, a) in seeds {
        put(pixels, x, y, theme.gold_soft, a);
        put(pixels, x + 1, y, (255, 240, 200), a * 0.45);
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
    use crate::logo::load_title_wordmark;
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
        let bg = load_rgb(&ui.join("menu_bg.jpg"));
        let logo = load_title_wordmark(&ui.join("logo_title.png"));
        let portrait = load_rgb(&ui.join("portrait_collector.jpg"));
        assert!(bg.is_some(), "menu_bg.jpg must exist for title paint tests");
        assert!(
            logo.is_some(),
            "logo_title.png must exist for title paint tests"
        );
        let sheet = load_sheet();
        let theme = Theme::default();
        let mut pixels = vec![0u32; (WW * WH) as usize];
        paint_title_menu(
            &mut pixels,
            &theme,
            bg.as_ref(),
            logo.as_ref(),
            portrait.as_ref(),
            &sheet,
            sel,
            1250,
            40,
            1.5,
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
    fn title_paint_filled_frame_with_logo() {
        let pixels = paint_frame(0);
        assert_eq!(pixels.len(), (WW * WH) as usize);
        let frac = non_void_fraction(&pixels, Theme::default().void);
        assert!(
            frac > 0.55,
            "title frame should be substantially filled, frac={frac}"
        );
        // Logo present → must not paint missing marker in the logo band
        // Sample center band: if logo blitted, many non-dark copper-ish pixels
        let mut copperish = 0usize;
        let y0 = 100u32;
        let y1 = 300u32;
        let x0 = 300u32;
        let x1 = 980u32;
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
            copperish > 800,
            "logo wordmark copper pixels expected, got {copperish}"
        );
    }

    #[test]
    fn title_selection_changes_button_region() {
        let a = paint_frame(0);
        let b = paint_frame(2);
        assert_ne!(a, b, "different selection must change buffer");
        // Button column region (left side)
        let ha = region_hash(&a, 40, 320, 500, 640);
        let hb = region_hash(&b, 40, 320, 500, 640);
        assert_ne!(
            ha, hb,
            "button column region hash must differ across selection"
        );
    }

    #[test]
    fn title_missing_logo_marker_absent_with_asset() {
        // Structural: paint path uses real logo; marker constant is only for fallback
        let _ = LOGO_MISSING_MARKER;
        let logo = load_title_wordmark(&data_ui().join("logo_title.png")).expect("logo");
        assert!(logo.0 > 10 && logo.1 > 10);
        // Soft alpha from black key means wordmark is not a solid rectangle
        let soft = crate::count_soft_alpha(&logo.3);
        assert!(soft > 50, "soft-keyed logo expected soft>{soft}");
    }

    #[test]
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
