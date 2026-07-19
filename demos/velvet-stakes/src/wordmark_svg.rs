//! SVG title wordmark for Velvet Arcana — authored as vector paths, rasterized
//! via `velvet_image::rasterize_simple_svg` (same stack as `.vcss` `@svg`).

use velvet_image::{build_svg_multipath, rasterize_simple_svg};
use velvet_story::pack_rgb;

use crate::logo::{crop_to_content, feather_alpha, RgbaBuf};

/// viewBox width of the title SVG.
pub const TITLE_VB_W: u32 = 720;
/// viewBox height of the title SVG.
pub const TITLE_VB_H: u32 = 240;

/// Default raster size for lobby title (crisp on 1280×720).
pub const TITLE_RASTER_W: u32 = 720;
/// Default raster height.
pub const TITLE_RASTER_H: u32 = 240;

/// Combined path `d` for `@svg logo_title` in casino.vcss (single path, multi-subpath).
pub fn title_path_d() -> String {
    letter_paths().join(" ")
}

/// Full SVG document for the title wordmark (transparent bg, gold fill).
pub fn title_wordmark_svg_xml() -> String {
    let paths: Vec<&str> = letter_paths_static();
    build_svg_multipath(
        TITLE_VB_W,
        TITLE_VB_H,
        "#e8c078",
        "#f0d8a0",
        1.2,
        &paths,
    )
}

/// Rasterize the SVG title into an RGBA buffer (alpha already correct — no black key).
pub fn rasterize_title_wordmark(w: u32, h: u32) -> Option<RgbaBuf> {
    let xml = title_wordmark_svg_xml();
    let img = rasterize_simple_svg(&xml, w.max(64), h.max(32)).ok()?;
    Some(rgba_image_to_buf(&img))
}

/// Rasterize from a stylesheet `@svg` XML or any SVG string.
pub fn rasterize_svg_wordmark(xml: &str, w: u32, h: u32) -> Option<RgbaBuf> {
    let img = rasterize_simple_svg(xml, w.max(64), h.max(32)).ok()?;
    Some(rgba_image_to_buf(&img))
}

/// Load SVG file from disk and rasterize.
pub fn load_title_wordmark_svg(path: &std::path::Path, w: u32, h: u32) -> Option<RgbaBuf> {
    let s = std::fs::read_to_string(path).ok()?;
    rasterize_svg_wordmark(&s, w, h)
}

fn rgba_image_to_buf(img: &velvet_image::RgbaImage) -> RgbaBuf {
    let n = (img.width * img.height) as usize;
    let mut rgb = Vec::with_capacity(n);
    let mut a = Vec::with_capacity(n);
    for i in 0..n {
        let o = i * 4;
        let r = img.pixels[o];
        let g = img.pixels[o + 1];
        let b = img.pixels[o + 2];
        let alpha = img.pixels[o + 3];
        rgb.push(pack_rgb(r, g, b));
        a.push(alpha);
    }
    feather_alpha(&mut a, img.width as usize, img.height as usize, 1);
    let full = (img.width, img.height, rgb, a);
    crop_to_content(&full, 8, 8)
}

/// Prefer: procedural SVG wordmark (always crisp) → `logo_title.svg` →
/// stylesheet `@svg logo_title` → PNG soft-key.
pub fn resolve_title_wordmark(
    sheet_svg_xml: Option<&str>,
    ui_dir: &std::path::Path,
    fallback_png: Option<RgbaBuf>,
) -> Option<RgbaBuf> {
    // Procedural multi-path SVG is the shipped title (vector → raster)
    if let Some(buf) = rasterize_title_wordmark(TITLE_RASTER_W, TITLE_RASTER_H) {
        return Some(buf);
    }
    let svg_path = ui_dir.join("logo_title.svg");
    if svg_path.exists() {
        if let Some(buf) = load_title_wordmark_svg(&svg_path, TITLE_RASTER_W, TITLE_RASTER_H) {
            return Some(buf);
        }
    }
    if let Some(xml) = sheet_svg_xml {
        if let Some(buf) = rasterize_svg_wordmark(xml, TITLE_RASTER_W, TITLE_RASTER_H) {
            return Some(buf);
        }
    }
    fallback_png
}

fn letter_paths() -> Vec<String> {
    letter_paths_static()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Geometric luxury letterforms (solid silhouettes).
/// Line 1: VELVET · Line 2: ARCANA — viewBox 720×240.
fn letter_paths_static() -> Vec<&'static str> {
    vec![
        // ── VELVET (y 28..118) ────────────────────────────────────
        // V
        "M 36 28 L 70 28 L 100 100 L 130 28 L 164 28 L 110 118 L 90 118 Z",
        // E
        "M 180 28 L 268 28 L 268 48 L 208 48 L 208 62 L 256 62 L 256 80 L 208 80 L 208 96 L 268 96 L 268 118 L 180 118 Z",
        // L
        "M 284 28 L 314 28 L 314 96 L 370 96 L 370 118 L 284 118 Z",
        // V
        "M 386 28 L 420 28 L 450 100 L 480 28 L 514 28 L 460 118 L 440 118 Z",
        // E
        "M 530 28 L 618 28 L 618 48 L 558 48 L 558 62 L 606 62 L 606 80 L 558 80 L 558 96 L 618 96 L 618 118 L 530 118 Z",
        // T
        "M 634 28 L 708 28 L 708 50 L 686 50 L 686 118 L 656 118 L 656 50 L 634 50 Z",
        // ── ARCANA (y 140..220) ───────────────────────────────────
        // A = solid triangle + crossbar rect (no hole)
        "M 40 220 L 90 140 L 140 220 L 116 220 L 90 172 L 64 220 Z",
        "M 68 188 L 112 188 L 112 202 L 68 202 Z",
        // R = stem + filled bowl + leg
        "M 160 140 L 190 140 L 190 220 L 160 220 Z",
        "M 190 140 L 250 140 C 280 140 294 156 294 172 C 294 188 280 200 250 200 L 190 200 Z",
        "M 230 200 L 290 220 L 262 220 L 210 204 Z",
        // C = outer arc minus inner (even-odd, one path two subpaths)
        "M 400 156 C 384 140 356 140 340 156 C 324 172 324 200 340 216 C 356 232 384 232 400 216 L 382 200 C 374 208 364 208 356 200 C 348 192 348 180 356 172 C 364 164 374 164 382 172 Z",
        // A
        "M 420 220 L 470 140 L 520 220 L 496 220 L 470 172 L 444 220 Z",
        "M 448 188 L 492 188 L 492 202 L 448 202 Z",
        // N
        "M 540 140 L 568 140 L 568 188 L 620 140 L 650 140 L 650 220 L 622 220 L 622 172 L 570 220 L 540 220 Z",
        // A (fits viewBox width 720)
        "M 664 220 L 692 140 L 720 220 L 700 220 L 692 188 L 684 220 Z",
        "M 682 188 L 702 188 L 702 200 L 682 200 Z",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_wordmark_raster_has_alpha_and_gold() {
        let buf = rasterize_title_wordmark(720, 240).expect("raster");
        let (w, h, rgb, a) = &buf;
        assert!(*w > 100 && *h > 40, "cropped size {}x{}", w, h);
        let soft = a.iter().filter(|&&v| (1..=254).contains(&v)).count();
        let solid = a.iter().filter(|&&v| v > 200).count();
        assert!(solid > 500, "solid gold pixels {solid}");
        // gold-ish samples
        let mut gold = 0usize;
        for (i, &p) in rgb.iter().enumerate() {
            if a[i] < 100 {
                continue;
            }
            let r = ((p >> 16) & 0xFF) as u32;
            let g = ((p >> 8) & 0xFF) as u32;
            let b = (p & 0xFF) as u32;
            if r > 140 && g > 90 && r > b {
                gold += 1;
            }
        }
        assert!(gold > 400, "gold letter pixels {gold}");
        let _ = soft;
    }

    #[test]
    fn title_svg_xml_is_valid_shape() {
        let xml = title_wordmark_svg_xml();
        assert!(xml.contains("viewBox"));
        assert!(xml.contains("<path"));
        assert!(xml.contains("#e8c078"));
    }
}
