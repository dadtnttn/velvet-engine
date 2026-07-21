//! Text shaping: cluster-based advances (HarfBuzz-class path via rustybuzz when a
//! font is available; engine cluster shaper otherwise).

use std::path::Path;
use std::sync::OnceLock;

use unicode_segmentation::UnicodeSegmentation;

use crate::style::TextStyle;

/// One shaped glyph/cluster advance.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    /// Cluster text (may be multi-codepoint after virama/mark merge).
    pub cluster: String,
    /// Advance width in pixels.
    pub advance: f32,
    /// Glyph id when shaped with a font; 0 for engine path.
    pub glyph_id: u32,
}

/// Result of shaping a string.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeResult {
    /// Glyphs / clusters in visual order.
    pub glyphs: Vec<ShapedGlyph>,
    /// Total advance width.
    pub width: f32,
    /// Backend used: `rustybuzz` or `engine`.
    pub backend: &'static str,
}

/// Naive width: one advance per Unicode scalar (not grapheme / not shaped).
pub fn naive_codepoint_width(text: &str, size: f32) -> f32 {
    text.chars().count() as f32 * size * 0.55
}

/// Shape `text` under `style` and return advances.
///
/// Prefer rustybuzz + loaded font when [`set_shape_font_bytes`] or a system font
/// path succeeds; otherwise use the engine cluster shaper (still not unicode-width
/// alone — merges Devanagari virama clusters and applies script advances).
pub fn shape_text(text: &str, style: &TextStyle) -> ShapeResult {
    if let Some(font) = SHAPE_FONT.get() {
        if let Some(r) = shape_with_rustybuzz(text, style, font) {
            return r;
        }
    }
    if let Some(bytes) = try_load_system_font() {
        if let Some(r) = shape_with_rustybuzz(text, style, &bytes) {
            let _ = SHAPE_FONT.set(bytes);
            return r;
        }
    }
    shape_engine(text, style)
}

/// Measured shaped width (primary product measure path).
pub fn shape_measure_width(text: &str, style: &TextStyle) -> f32 {
    shape_text(text, style).width
}

static SHAPE_FONT: OnceLock<Vec<u8>> = OnceLock::new();

/// Install font bytes for the rustybuzz path (tests / hosts).
pub fn set_shape_font_bytes(bytes: Vec<u8>) -> bool {
    SHAPE_FONT.set(bytes).is_ok()
}

/// Whether a font is currently loaded for rustybuzz.
pub fn shape_font_loaded() -> bool {
    SHAPE_FONT.get().is_some() || try_load_system_font().is_some()
}

fn try_load_system_font() -> Option<Vec<u8>> {
    static CACHED: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            let candidates = [
                r"C:\Windows\Fonts\Nirmala.ttf",
                r"C:\Windows\Fonts\arial.ttf",
                r"C:\Windows\Fonts\segoeui.ttf",
                r"C:\Windows\Fonts\malgun.ttf",
                r"C:\Windows\Fonts\msyh.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
            ];
            for p in candidates {
                if Path::new(p).is_file() {
                    if let Ok(b) = std::fs::read(p) {
                        if b.len() > 1000 {
                            return Some(b);
                        }
                    }
                }
            }
            None
        })
        .clone()
}

fn shape_with_rustybuzz(text: &str, style: &TextStyle, font_data: &[u8]) -> Option<ShapeResult> {
    let face = rustybuzz::ttf_parser::Face::parse(font_data, 0).ok()?;
    let rb_face = rustybuzz::Face::from_face(face);
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.guess_segment_properties();
    let output = rustybuzz::shape(&rb_face, &[], buffer);
    let units = rb_face.units_per_em() as f32;
    if units <= 0.0 {
        return None;
    }
    let scale = style.size / units;
    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    let mut glyphs = Vec::with_capacity(infos.len());
    let mut width = 0.0f32;
    for (info, pos) in infos.iter().zip(positions.iter()) {
        let adv = pos.x_advance as f32 * scale + style.letter_spacing;
        // Cluster slice approx from original string by cluster index
        let cluster = text
            .chars()
            .nth(info.cluster as usize)
            .map(|c| c.to_string())
            .unwrap_or_default();
        glyphs.push(ShapedGlyph {
            cluster,
            advance: adv,
            glyph_id: info.glyph_id,
        });
        width += adv;
    }
    // Empty text
    if text.is_empty() {
        return Some(ShapeResult {
            glyphs: vec![],
            width: 0.0,
            backend: "rustybuzz",
        });
    }
    // If shaping produced zero width for non-empty (missing glyphs), fall back.
    if width <= 0.0 && !text.chars().all(|c| c.is_whitespace()) {
        return None;
    }
    Some(ShapeResult {
        glyphs,
        width,
        backend: "rustybuzz",
    })
}

/// Engine cluster shaper: grapheme-aware + Devanagari virama merge + script advances.
fn shape_engine(text: &str, style: &TextStyle) -> ShapeResult {
    let mut clusters: Vec<String> = Vec::new();
    for g in text.graphemes(true) {
        // Merge when previous ends with Devanagari virama U+094D and this starts with consonant
        if let Some(prev) = clusters.last_mut() {
            if prev.chars().last().map(|c| c as u32) == Some(0x094D) {
                prev.push_str(g);
                continue;
            }
        }
        clusters.push(g.to_string());
    }

    // Additional pass: attach spacing marks that sometimes stay separate
    let mut merged: Vec<String> = Vec::new();
    for c in clusters {
        if let Some(prev) = merged.last_mut() {
            let only_marks = c.chars().all(|ch| {
                let u = ch as u32;
                (0x093A..=0x094F).contains(&u) || (0x0951..=0x0957).contains(&u) || ch == '\u{094D}'
            });
            if only_marks && !c.is_empty() {
                prev.push_str(&c);
                continue;
            }
        }
        merged.push(c);
    }

    let mut glyphs = Vec::new();
    let mut width = 0.0f32;
    for cluster in merged {
        let adv = cluster_advance(&cluster, style);
        width += adv;
        glyphs.push(ShapedGlyph {
            cluster,
            advance: adv,
            glyph_id: 0,
        });
    }
    ShapeResult {
        glyphs,
        width,
        backend: "engine",
    }
}

fn cluster_advance(cluster: &str, style: &TextStyle) -> f32 {
    let size = style.size;
    let mut adv = 0.0f32;
    let chars = cluster.chars().peekable();
    for ch in chars {
        let u = ch as u32;
        // Virama itself does not add width when combining into a conjunct
        if u == 0x094D {
            continue;
        }
        // Combining marks (Devanagari signs) — zero advance in shaped metrics
        if (0x093A..=0x094F).contains(&u) || (0x0951..=0x0957).contains(&u) {
            continue;
        }
        // CJK fullwidth
        if is_cjk(u) {
            adv += size * 1.0;
            continue;
        }
        // Devanagari base
        if (0x0900..=0x097F).contains(&u) {
            adv += size * 0.72;
            continue;
        }
        if ch.is_ascii_whitespace() {
            adv += size * 0.33;
            continue;
        }
        if ch.is_ascii() {
            adv += size * 0.55;
            continue;
        }
        adv += size * 0.6;
    }
    adv + style.letter_spacing
}

fn is_cjk(u: u32) -> bool {
    (0x4E00..=0x9FFF).contains(&u)
        || (0x3040..=0x30FF).contains(&u)
        || (0xAC00..=0xD7AF).contains(&u)
        || (0x3400..=0x4DBF).contains(&u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shaped_differs_from_naive_for_complex_scripts() {
        let style = TextStyle {
            size: 28.0,
            ..TextStyle::default()
        };
        let zh = "你好";
        let hi = "नमस्ते";
        let s_zh = shape_text(zh, &style);
        let s_hi = shape_text(hi, &style);
        let n_zh = naive_codepoint_width(zh, style.size);
        let n_hi = naive_codepoint_width(hi, style.size);
        // Shaped path must run (non-zero) and differ from naive codepoint counting
        assert!(s_zh.width > 0.0, "CJK width");
        assert!(s_hi.width > 0.0, "Devanagari width");
        assert!(
            (s_zh.width - n_zh).abs() > 0.5 || (s_hi.width - n_hi).abs() > 0.5,
            "shaped should differ from naive: zh shaped={} naive={} hi shaped={} naive={} backend={}",
            s_zh.width,
            n_zh,
            s_hi.width,
            n_hi,
            s_zh.backend
        );
        // Determinism
        let s_zh2 = shape_text(zh, &style);
        let s_hi2 = shape_text(hi, &style);
        assert!((s_zh.width - s_zh2.width).abs() < 0.01);
        assert!((s_hi.width - s_hi2.width).abs() < 0.01);
        // Devanagari should form fewer clusters than codepoints when virama merges
        let hi_chars = hi.chars().count();
        assert!(
            s_hi.glyphs.len() < hi_chars || s_hi.backend == "rustybuzz",
            "expected cluster merge or rustybuzz: glyphs={} chars={} backend={}",
            s_hi.glyphs.len(),
            hi_chars,
            s_hi.backend
        );
    }

    #[test]
    fn measure_width_uses_shape_path() {
        let style = TextStyle::default();
        let w = crate::measure::measure_width("你好", &style);
        let s = shape_measure_width("你好", &style);
        assert!(
            (w - s).abs() < 0.01,
            "measure_width should equal shape width"
        );
    }
}
