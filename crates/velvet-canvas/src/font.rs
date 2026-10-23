//! Fonts and text: TrueType/OpenType fonts through ab_glyph, drawn as anti-aliased paths (so text is sharp at any scale
//! and can carry an outline), measured and wrapped the way the canvas lays it out.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use ab_glyph::{Font as _, FontArc, GlyphId, OutlineCurve};

use crate::color::Color;
use crate::geometry::Point;

/// A font could not be read.
#[derive(Debug)]
pub struct FontError(String);

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the font could not be read: {}", self.0)
    }
}

impl std::error::Error for FontError {}

/// A font, cheap to clone. Its glyphs are turned into paths once, the first time they are drawn.
#[derive(Clone)]
pub struct Font {
    inner: Arc<Inner>,
}

struct Inner {
    font: FontArc,
    units_per_em: f32,
    ascent: f32,
    descent: f32,
    paths: Mutex<HashMap<GlyphId, Option<tiny_skia::Path>>>,
}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("units_per_em", &self.inner.units_per_em)
            .finish()
    }
}

impl Font {
    /// Reads a TrueType or OpenType font.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Font, FontError> {
        let font = FontArc::try_from_vec(bytes).map_err(|e| FontError(e.to_string()))?;
        let units_per_em = font.units_per_em().unwrap_or(1000.0);
        let ascent = font.ascent_unscaled();
        let descent = font.descent_unscaled();
        Ok(Font {
            inner: Arc::new(Inner {
                font,
                units_per_em,
                ascent,
                descent,
                paths: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// How far the font rises above the baseline at a size (the size is the em, as in CSS).
    pub fn ascent(&self, size: f32) -> f32 {
        self.inner.ascent * self.scale(size)
    }

    /// How far the font drops below the baseline at a size, as a positive length.
    pub fn descent(&self, size: f32) -> f32 {
        -self.inner.descent * self.scale(size)
    }

    /// The width of a line of text, in logical pixels.
    pub fn measure(&self, text: &str, size: f32, letter_spacing: f32) -> f32 {
        let scale = self.scale(size);
        let mut width = 0.0;
        let mut previous: Option<GlyphId> = None;
        let mut count = 0;
        for c in text.chars() {
            let id = self.inner.font.glyph_id(c);
            if let Some(p) = previous {
                width += self.inner.font.kern_unscaled(p, id) * scale;
            }
            width += self.inner.font.h_advance_unscaled(id) * scale;
            previous = Some(id);
            count += 1;
        }
        width + letter_spacing * (count.max(1) - 1) as f32
    }

    /// Breaks text into lines no wider than `max_width`: at spaces, and inside a word only when the word alone is
    /// too wide. A line break in the text always starts a new line.
    pub fn wrap(&self, text: &str, size: f32, letter_spacing: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() {
                    word.to_string()
                } else {
                    format!("{line} {word}")
                };
                if self.measure(&candidate, size, letter_spacing) <= max_width
                    || line.is_empty() && word.is_empty()
                {
                    line = candidate;
                    continue;
                }
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                // A word wider than the line is cut where it no longer fits.
                let mut piece = String::new();
                for c in word.chars() {
                    piece.push(c);
                    if self.measure(&piece, size, letter_spacing) > max_width
                        && piece.chars().count() > 1
                    {
                        piece.pop();
                        lines.push(std::mem::take(&mut piece));
                        piece.push(c);
                    }
                }
                line = piece;
            }
            lines.push(line);
        }
        lines
    }

    pub(crate) fn scale(&self, size: f32) -> f32 {
        size / self.inner.units_per_em
    }

    /// The glyphs of a line: each glyph's path (in font units, y down) and its pen position in logical pixels.
    pub(crate) fn layout(
        &self,
        text: &str,
        size: f32,
        letter_spacing: f32,
    ) -> Vec<(Option<tiny_skia::Path>, f32)> {
        let scale = self.scale(size);
        let mut out = Vec::new();
        let mut pen = 0.0;
        let mut previous: Option<GlyphId> = None;
        for c in text.chars() {
            let id = self.inner.font.glyph_id(c);
            if let Some(p) = previous {
                pen += self.inner.font.kern_unscaled(p, id) * scale;
            }
            out.push((self.glyph_path(id), pen));
            pen += self.inner.font.h_advance_unscaled(id) * scale + letter_spacing;
            previous = Some(id);
        }
        out
    }

    fn glyph_path(&self, id: GlyphId) -> Option<tiny_skia::Path> {
        let mut paths = self
            .inner
            .paths
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        paths
            .entry(id)
            .or_insert_with(|| outline_path(&self.inner.font, id))
            .clone()
    }
}

/// A glyph's outline as a path in font units, with y pointing down as on the canvas.
fn outline_path(font: &FontArc, id: GlyphId) -> Option<tiny_skia::Path> {
    let outline = font.outline(id)?;
    let mut b = tiny_skia::PathBuilder::new();
    let mut current: Option<ab_glyph::Point> = None;
    let near = |a: ab_glyph::Point, b: ab_glyph::Point| {
        (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3
    };
    for curve in &outline.curves {
        let start = match curve {
            OutlineCurve::Line(p0, _)
            | OutlineCurve::Quad(p0, _, _)
            | OutlineCurve::Cubic(p0, _, _, _) => *p0,
        };
        if !current.is_some_and(|c| near(c, start)) {
            if current.is_some() {
                b.close();
            }
            b.move_to(start.x, -start.y);
        }
        current = Some(match curve {
            OutlineCurve::Line(_, p1) => {
                b.line_to(p1.x, -p1.y);
                *p1
            }
            OutlineCurve::Quad(_, p1, p2) => {
                b.quad_to(p1.x, -p1.y, p2.x, -p2.y);
                *p2
            }
            OutlineCurve::Cubic(_, p1, p2, p3) => {
                b.cubic_to(p1.x, -p1.y, p2.x, -p2.y, p3.x, -p3.y);
                *p3
            }
        });
    }
    if current.is_some() {
        b.close();
    }
    b.finish()
}

/// How a line of text sits on its x: its left edge, its centre or its right edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// How text is drawn: its font and size (the em, in logical pixels), colour, alignment, line height (a factor of the
/// size), letter spacing, and an optional outline and drop shadow.
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font: Font,
    pub size: f32,
    pub color: Color,
    pub align: Align,
    pub line_height: f32,
    pub letter_spacing: f32,
    /// The outline's width (outside the letters) and colour.
    pub outline: Option<(f32, Color)>,
    /// The shadow's offset and colour.
    pub shadow: Option<(Point, Color)>,
}

impl TextStyle {
    pub fn new(font: &Font, size: f32, color: Color) -> Self {
        Self {
            font: font.clone(),
            size,
            color,
            align: Align::Left,
            line_height: 1.25,
            letter_spacing: 0.0,
            outline: None,
            shadow: None,
        }
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn line_height(mut self, factor: f32) -> Self {
        self.line_height = factor;
        self
    }

    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    pub fn outline(mut self, width: f32, color: Color) -> Self {
        self.outline = Some((width, color));
        self
    }

    pub fn shadow(mut self, offset: Point, color: Color) -> Self {
        self.shadow = Some((offset, color));
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// The height of one line.
    pub fn line_px(&self) -> f32 {
        self.size * self.line_height
    }

    /// The width of a line of text in this style.
    pub fn measure(&self, text: &str) -> f32 {
        self.font.measure(text, self.size, self.letter_spacing)
    }

    /// The lines of text that fit a width in this style.
    pub fn wrap(&self, text: &str, max_width: f32) -> Vec<String> {
        self.font
            .wrap(text, self.size, self.letter_spacing, max_width)
    }

    /// From the top of a line box to its baseline.
    pub(crate) fn baseline_offset(&self) -> f32 {
        let ascent = self.font.ascent(self.size);
        let content = ascent + self.font.descent(self.size);
        (self.line_px() - content) / 2.0 + ascent
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn test_font() -> Font {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/Nunito-ExtraBold.ttf"
        );
        Font::from_bytes(std::fs::read(path).expect("the test font")).expect("a font")
    }

    #[test]
    fn measure_grows_with_the_text_and_the_size() {
        let font = test_font();
        let a = font.measure("Pasaporte", 20.0, 0.0);
        assert!(a > 0.0);
        assert!(font.measure("Pasaporte de Velmira", 20.0, 0.0) > a);
        assert!((font.measure("Pasaporte", 40.0, 0.0) - 2.0 * a).abs() < 0.5);
        assert!(font.measure("ab", 20.0, 3.0) > font.measure("ab", 20.0, 0.0));
    }

    #[test]
    fn wrap_keeps_every_line_inside_the_width() {
        let font = test_font();
        let text =
            "Permiso de entrada emitido por el Ministerio de Puertas, Pasos y Sellos de Velmira";
        let lines = font.wrap(text, 18.0, 0.0, 200.0);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(font.measure(line, 18.0, 0.0) <= 200.0 + 0.01, "{line}");
        }
        assert_eq!(lines.join(" "), text);
    }

    #[test]
    fn wrap_cuts_a_word_wider_than_the_line_and_keeps_line_breaks() {
        let font = test_font();
        let lines = font.wrap("Supercalifragilisticoespialidoso", 20.0, 0.0, 80.0);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), "Supercalifragilisticoespialidoso");
        assert_eq!(font.wrap("uno\ndos", 20.0, 0.0, 1000.0), vec!["uno", "dos"]);
    }
}
