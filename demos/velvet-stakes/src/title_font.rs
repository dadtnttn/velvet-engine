//! Title wordmark painted with a real **serif font** (not SVG paths, not bitmap plate).
//!
//! Loads an elegant Georgia / Constantia / Times serif from OS fonts when available;
//! falls back to large softbuffer bitmap glyphs so headless CI still paints.

use std::path::PathBuf;
use std::sync::OnceLock;

use fontdue::Font;
use velvet_story::pack_rgb;

use crate::ui::theme::{WH, WW};

/// Logical title lines.
pub const TITLE_LINE1: &str = "VELVET";
/// Second line of the wordmark.
pub const TITLE_LINE2: &str = "ARCANA";
/// Subtitle under the wordmark.
pub const TITLE_SUB: &str = "NIGHTFALL CASINO";

static TITLE_FONT: OnceLock<Option<Font>> = OnceLock::new();
static UI_FONT: OnceLock<Option<Font>> = OnceLock::new();

/// Load a display serif once (Georgia → Constantia → Times → bold fallbacks).
pub fn title_font() -> Option<&'static Font> {
    TITLE_FONT.get_or_init(load_display_serif).as_ref()
}

/// Load a readable sans-serif UI font once for menu labels and supporting copy.
pub fn ui_font() -> Option<&'static Font> {
    UI_FONT.get_or_init(load_ui_sans).as_ref()
}

fn load_display_serif() -> Option<Font> {
    for path in font_candidates() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(font) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                eprintln!("title font: {}", path.display());
                return Some(font);
            }
        }
    }
    eprintln!("title font: no system serif found — bitmap fallback");
    None
}

fn load_ui_sans() -> Option<Font> {
    for path in ui_font_candidates() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(font) = Font::from_bytes(bytes, fontdue::FontSettings::default()) {
                eprintln!("ui font: {}", path.display());
                return Some(font);
            }
        }
    }
    eprintln!("ui font: no system sans found - bitmap fallback");
    None
}

fn font_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    // Bundled optional font under data/fonts/
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/fonts");
    for name in [
        "Cinzel-Bold.ttf",
        "PlayfairDisplay-Bold.ttf",
        "Georgia-Bold.ttf",
        "georgia.ttf",
        "georgiab.ttf",
        "times.ttf",
        "timesbd.ttf",
    ] {
        out.push(bundled.join(name));
    }
    // Windows
    if let Ok(windir) = std::env::var("WINDIR") {
        let fonts = PathBuf::from(windir).join("Fonts");
        for name in [
            "georgia.ttf",
            "constan.ttf",
            "GARA.TTF",
            "times.ttf",
            "georgiab.ttf",
            "constanb.ttf",
            "timesbd.ttf",
            "cambriab.ttf",
            "garabd.ttf",
        ] {
            out.push(fonts.join(name));
        }
    }
    // Linux common
    for p in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSerif-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSerif.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    ] {
        out.push(PathBuf::from(p));
    }
    // macOS
    for p in [
        "/System/Library/Fonts/Supplemental/Georgia.ttf",
        "/System/Library/Fonts/Supplemental/Times New Roman.ttf",
        "/Library/Fonts/Georgia Bold.ttf",
    ] {
        out.push(PathBuf::from(p));
    }
    out
}

fn ui_font_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/fonts");
    for name in [
        "Inter-SemiBold.ttf",
        "SourceSans3-Semibold.ttf",
        "SegoeUI.ttf",
    ] {
        out.push(bundled.join(name));
    }
    if let Ok(windir) = std::env::var("WINDIR") {
        let fonts = PathBuf::from(windir).join("Fonts");
        for name in ["seguisb.ttf", "segoeui.ttf", "arial.ttf", "calibri.ttf"] {
            out.push(fonts.join(name));
        }
    }
    for path in [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationSans-Regular.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ] {
        out.push(PathBuf::from(path));
    }
    out
}

/// Measure text width in pixels at `px` size.
pub fn measure_text(font: &Font, text: &str, px: f32) -> f32 {
    let mut w = 0.0f32;
    for ch in text.chars() {
        let (metrics, _) = font.rasterize(ch, px);
        w += metrics.advance_width;
    }
    w
}

/// Draw text with baseline at `(x, baseline_y)`. Alpha-blends coverage.
pub fn draw_font_text(
    pixels: &mut [u32],
    font: &Font,
    x: f32,
    baseline_y: f32,
    text: &str,
    px: f32,
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let op = opacity.clamp(0.0, 1.0);
    let mut pen_x = x;
    for ch in text.chars() {
        let (metrics, bitmap) = font.rasterize(ch, px);
        let ox = pen_x + metrics.xmin as f32;
        let oy = baseline_y - metrics.height as f32 - metrics.ymin as f32;
        blit_glyph(
            pixels,
            ox.round() as i32,
            oy.round() as i32,
            metrics.width,
            metrics.height,
            &bitmap,
            rgb,
            op,
        );
        pen_x += metrics.advance_width;
    }
}

fn blit_glyph(
    pixels: &mut [u32],
    x0: i32,
    y0: i32,
    gw: usize,
    gh: usize,
    coverage: &[u8],
    rgb: (u8, u8, u8),
    opacity: f32,
) {
    let src = pack_rgb(rgb.0, rgb.1, rgb.2);
    for row in 0..gh {
        for col in 0..gw {
            let cov = coverage[row * gw + col] as f32 / 255.0;
            let a = cov * opacity;
            if a < 0.02 {
                continue;
            }
            let px = x0 + col as i32;
            let py = y0 + row as i32;
            if px < 0 || py < 0 || px >= WW as i32 || py >= WH as i32 {
                continue;
            }
            let i = (py as u32 * WW + px as u32) as usize;
            pixels[i] = blend(pixels[i], src, a);
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

/// Paint the full title block (VELVET / ARCANA + subtitle) centered near `cx`.
///
/// Uses a real serif when available; otherwise large bitmap text.
pub fn paint_title_wordmark(
    pixels: &mut [u32],
    cx: i32,
    top_y: i32,
    gold: (u8, u8, u8),
    gold_soft: (u8, u8, u8),
) -> (i32, i32, i32, i32) {
    // returns (x, y, w, h) of wordmark block for layout of subtitle rules
    if let Some(font) = title_font() {
        paint_serif_title(pixels, font, cx, top_y, gold, gold_soft)
    } else {
        paint_bitmap_title(pixels, cx, top_y, gold, gold_soft)
    }
}

fn paint_serif_title(
    pixels: &mut [u32],
    font: &Font,
    cx: i32,
    top_y: i32,
    gold: (u8, u8, u8),
    gold_soft: (u8, u8, u8),
) -> (i32, i32, i32, i32) {
    let px1 = 72.0;
    let px2 = 78.0;
    let w1 = measure_text(font, TITLE_LINE1, px1);
    let w2 = measure_text(font, TITLE_LINE2, px2);
    let block_w = w1.max(w2);
    let x1 = cx as f32 - w1 * 0.5;
    let x2 = cx as f32 - w2 * 0.5;
    let baseline1 = top_y as f32 + px1 * 0.85;
    let baseline2 = baseline1 + px2 * 0.95;

    // Soft shadow / purple rim for casino depth
    let shadow = (40, 20, 60);
    draw_font_text(
        pixels,
        font,
        x1 + 3.0,
        baseline1 + 3.0,
        TITLE_LINE1,
        px1,
        shadow,
        0.45,
    );
    draw_font_text(
        pixels,
        font,
        x2 + 3.0,
        baseline2 + 3.0,
        TITLE_LINE2,
        px2,
        shadow,
        0.45,
    );
    // Warm under-glow
    draw_font_text(
        pixels,
        font,
        x1 + 1.0,
        baseline1 + 1.0,
        TITLE_LINE1,
        px1,
        (180, 80, 160),
        0.25,
    );
    draw_font_text(
        pixels,
        font,
        x2 + 1.0,
        baseline2 + 1.0,
        TITLE_LINE2,
        px2,
        (180, 80, 160),
        0.25,
    );
    // Main gold face
    draw_font_text(pixels, font, x1, baseline1, TITLE_LINE1, px1, gold, 1.0);
    draw_font_text(
        pixels,
        font,
        x2,
        baseline2,
        TITLE_LINE2,
        px2,
        gold_soft,
        1.0,
    );
    // Subtle highlight pass (slightly brighter, upper bias via 0 offset)
    let hi = (
        gold.0.saturating_add(25),
        gold.1.saturating_add(20),
        gold.2.saturating_add(10),
    );
    draw_font_text(
        pixels,
        font,
        x1 - 0.5,
        baseline1 - 0.5,
        TITLE_LINE1,
        px1,
        hi,
        0.22,
    );
    draw_font_text(
        pixels,
        font,
        x2 - 0.5,
        baseline2 - 0.5,
        TITLE_LINE2,
        px2,
        hi,
        0.22,
    );

    let left = (cx as f32 - block_w * 0.5) as i32;
    let h = (baseline2 - top_y as f32 + 12.0) as i32;
    (left, top_y, block_w as i32, h)
}

fn paint_bitmap_title(
    pixels: &mut [u32],
    cx: i32,
    top_y: i32,
    gold: (u8, u8, u8),
    gold_soft: (u8, u8, u8),
) -> (i32, i32, i32, i32) {
    use crate::render::text;
    // scale 5 ≈ 30px tall glyphs
    let scale = 5i32;
    let advance = 6 * scale;
    let w1 = TITLE_LINE1.chars().count() as i32 * advance;
    let w2 = TITLE_LINE2.chars().count() as i32 * advance;
    let x1 = cx - w1 / 2;
    let x2 = cx - w2 / 2;
    text(
        pixels,
        WW,
        WH,
        x1 + 2,
        top_y + 2,
        TITLE_LINE1,
        (40, 20, 60),
        scale,
    );
    text(pixels, WW, WH, x1, top_y, TITLE_LINE1, gold, scale);
    let y2 = top_y + 9 * scale + 8;
    text(
        pixels,
        WW,
        WH,
        x2 + 2,
        y2 + 2,
        TITLE_LINE2,
        (40, 20, 60),
        scale,
    );
    text(pixels, WW, WH, x2, y2, TITLE_LINE2, gold_soft, scale);
    let block_w = w1.max(w2);
    (cx - block_w / 2, top_y, block_w, y2 - top_y + 9 * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_font_paints_gold_band() {
        let mut pixels = vec![0u32; (WW * WH) as usize];
        let (_x, _y, w, h) = paint_title_wordmark(
            &mut pixels,
            WW as i32 / 2 + 80,
            110,
            (232, 192, 120),
            (240, 210, 160),
        );
        assert!(w > 80 && h > 40, "wordmark size {w}x{h}");
        let mut gold = 0usize;
        for y in 100..280 {
            for x in 400..1100 {
                let p = pixels[(y * WW + x) as usize];
                let r = (p >> 16) & 0xFF;
                let g = (p >> 8) & 0xFF;
                let b = p & 0xFF;
                if r > 120 && g > 70 && r > b {
                    gold += 1;
                }
            }
        }
        assert!(
            gold > 400,
            "serif/bitmap title should paint gold pixels, got {gold}"
        );
    }
}
