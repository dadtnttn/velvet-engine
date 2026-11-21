//! Fonts and text: TrueType/OpenType fonts through ab_glyph, drawn as anti-aliased paths (so text is sharp at any scale
//! and can carry an outline), measured and wrapped the way the canvas lays it out. A font of outlines has one colour;
//! what needs its own (emoji) is drawn from pictures the font is given (`Font::with_pictures`).

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use ab_glyph::{Font as _, FontArc, GlyphId, OutlineCurve};

use crate::color::Color;
use crate::geometry::Point;
use crate::image::Image;

/// Where a font's pictures come from: asked once for each character that may be drawn as one (an emoji, a symbol).
pub type Pictures = Arc<dyn Fn(char) -> Option<Image> + Send + Sync>;

/// A character's picture (an image keeps smaller copies of itself as it is drawn, so it is locked while drawn).
pub(crate) type Picture = Arc<Mutex<Image>>;

/// A picture's side, and the room it takes on its line, in ems: a little larger than the letters, as emoji are.
pub(crate) const PICTURE_SIDE: f32 = 1.12;
const PICTURE_ADVANCE: f32 = 1.2;

/// A glyph of a line laid out: its path (in its font's units, with y down), where its origin sits from the line's left
/// end and from its baseline (logical pixels), and what turns its font's units into logical pixels.
pub(crate) struct Glyph {
    pub path: Option<tiny_skia::Path>,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

/// A line laid out: its glyphs and its pictures (each one's left edge in logical pixels).
pub(crate) struct Laid {
    pub glyphs: Vec<Glyph>,
    pub pictures: Vec<(Picture, f32)>,
}

/// What a character is drawn as.
enum Drawn {
    Glyph(GlyphId),
    Picture(Picture),
    Nothing,
}

/// A stretch of a line drawn one way.
enum Item {
    Picture(Picture),
    /// Letters of one font of the chain (0 is the font itself, 1 the first behind it), set one after another.
    Plain(usize, String),
    /// Letters of one font that read right only once shaped (joined, reordered, their marks placed), and whether they
    /// run from right to left.
    Shaped(usize, String, bool),
}

/// What a line hands over as it is walked: a glyph of one of the chain's fonts, or a picture and its left edge.
enum Put<'a> {
    Glyph {
        font: &'a Font,
        id: GlyphId,
        x: f32,
        y: f32,
        scale: f32,
    },
    Picture(Picture, f32),
}

/// A glyph of a shaped stretch, in font units: which one, how far the pen moves after it, where it sits from the pen,
/// and whether a letter ends with it (a letter and its marks are one, for the spacing between letters).
#[derive(Clone, Copy)]
struct Placed {
    id: u16,
    advance: f32,
    dx: f32,
    dy: f32,
    ends: bool,
}

/// How many shaped stretches a font keeps (a line is measured many times while it is wrapped).
#[cfg_attr(not(feature = "shaping"), allow(dead_code))]
const SHAPED_KEPT: usize = 4096;

/// The stretches a font has shaped, by their letters and their direction.
type Shaped = HashMap<(String, bool), Arc<Vec<Placed>>>;

/// A character of a script written from right to left: Hebrew, Arabic and their neighbours.
fn backwards(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

/// A character of a script whose letters join, stack or change places: the right-to-left ones, the scripts of India,
/// Thai and its neighbours.
fn complex(c: char) -> bool {
    backwards(c)
        || matches!(c as u32, 0x0900..=0x0DFF | 0x0E00..=0x0EFF | 0x0F00..=0x0FFF | 0x1000..=0x109F | 0x1780..=0x17FF)
}

/// A mark that belongs to the letter before it: a joiner, a non-joiner, an accent set over the letter.
fn attaches(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x200C | 0x200D)
}

/// A character of a script written with no spaces between its words, where a line may end after any letter: Chinese
/// characters, kana, and the punctuation and full-width forms that go with them.
fn unspaced(c: char) -> bool {
    matches!(c as u32, 0x2E80..=0x312F | 0x3190..=0x9FFF | 0xF900..=0xFAFF | 0xFF00..=0xFFEF | 0x20000..=0x3FFFF)
}

/// A mark no line begins with: what closes, what ends a sentence, the small kana and the long-vowel mark.
fn closes(c: char) -> bool {
    "、。，．・：；？！゛゜ヽヾゝゞ々ー）〕］｝〉》」』】〙〗｠»”’…‥％,.!?:;)]}%ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ".contains(c)
}

/// A mark no line ends with: what opens.
fn opens(c: char) -> bool {
    "（〔［｛〈《「『【〘〖｟«“‘([{".contains(c)
}

/// The stretches of a line in the order they are seen, each with whether it reads from right to left.
fn visual_runs(text: &str) -> Vec<(&str, bool)> {
    if !text.chars().any(backwards) {
        return vec![(text, false)];
    }
    #[cfg(feature = "shaping")]
    {
        let info = unicode_bidi::BidiInfo::new(text, None);
        let mut runs = Vec::new();
        for paragraph in &info.paragraphs {
            let (levels, ranges) = info.visual_runs(paragraph, paragraph.range.clone());
            for range in ranges {
                let backwards = levels[range.start].is_rtl();
                runs.push((&text[range], backwards));
            }
        }
        runs
    }
    #[cfg(not(feature = "shaping"))]
    {
        vec![(text, false)]
    }
}

/// The marks that only modify the emoji before them: variation selectors, the joiner, the keycap, skin tones, tags.
fn modifies(c: char) -> bool {
    matches!(c as u32, 0x200D | 0x20E3 | 0xFE0E | 0xFE0F | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F)
}

/// Whether a character is in the blocks emoji are drawn from (so that letters never ask for a picture).
fn may_be_picture(c: char) -> bool {
    matches!(
        c as u32,
        0x203C | 0x2049 | 0x2122 | 0x2139 | 0x2194..=0x21AA | 0x231A..=0x23FF | 0x24C2 | 0x25AA..=0x25FE | 0x2600..=0x27BF
            | 0x2934 | 0x2935 | 0x2B05..=0x2B55 | 0x3030 | 0x303D | 0x3297 | 0x3299 | 0x1F000..=0x1FAFF
    )
}

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
    face_index: u32,
    units_per_em: f32,
    ascent: f32,
    descent: f32,
    paths: Mutex<HashMap<GlyphId, Option<tiny_skia::Path>>>,
    /// Where this font's pictures come from, and the characters already asked for.
    pictures: Option<Pictures>,
    pictured: Mutex<HashMap<char, Option<Picture>>>,
    /// The fonts behind this one, asked in order for the characters it has no glyph for.
    fallbacks: Vec<Font>,
    /// The stretches already shaped.
    #[cfg_attr(not(feature = "shaping"), allow(dead_code))]
    shaped: Mutex<Shaped>,
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
    /// A font from bytes that stay for the whole run (a file mapped into memory: only the pages read take memory, and
    /// they are the system's to take back).
    pub fn from_static(bytes: &'static [u8]) -> Result<Font, FontError> {
        Self::from_static_index(bytes, 0)
    }

    /// Reads face `index` of a TrueType/OpenType font collection (`.ttc`/`.otc`) from static bytes.
    pub fn from_static_index(bytes: &'static [u8], index: u32) -> Result<Font, FontError> {
        let font = ab_glyph::FontRef::try_from_slice_and_index(bytes, index)
            .map(FontArc::new)
            .map_err(|e| FontError(e.to_string()))?;
        Font::with(font, index)
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Result<Font, FontError> {
        Self::from_bytes_index(bytes, 0)
    }

    /// Reads face `index` of a TrueType/OpenType font collection (`.ttc`/`.otc`) from owned bytes.
    pub fn from_bytes_index(bytes: Vec<u8>, index: u32) -> Result<Font, FontError> {
        let font = ab_glyph::FontVec::try_from_vec_and_index(bytes, index)
            .map(FontArc::new)
            .map_err(|e| FontError(e.to_string()))?;
        Font::with(font, index)
    }

    fn with(font: FontArc, face_index: u32) -> Result<Font, FontError> {
        let units_per_em = font.units_per_em().unwrap_or(1000.0);
        let ascent = font.ascent_unscaled();
        let descent = font.descent_unscaled();
        Ok(Font {
            inner: Arc::new(Inner {
                font,
                face_index,
                units_per_em,
                ascent,
                descent,
                paths: Mutex::new(HashMap::new()),
                pictures: None,
                pictured: Mutex::new(HashMap::new()),
                fallbacks: Vec::new(),
                shaped: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// This font again, with other pictures or other fonts behind it.
    fn again(&self, pictures: Option<Pictures>, fallbacks: Vec<Font>) -> Font {
        Font {
            inner: Arc::new(Inner {
                font: self.inner.font.clone(),
                face_index: self.inner.face_index,
                units_per_em: self.inner.units_per_em,
                ascent: self.inner.ascent,
                descent: self.inner.descent,
                paths: Mutex::new(HashMap::new()),
                pictures,
                pictured: Mutex::new(HashMap::new()),
                fallbacks,
                shaped: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// This font with pictures for what a font of outlines cannot draw: colour emoji. `pictures` is asked once for
    /// each emoji written, and its picture is drawn in the character's place, a little larger than the letters. An
    /// emoji it has no picture for is drawn as the font's own glyph, or left out when the font has none; the marks
    /// that only modify an emoji (variation selectors, the joiner, skin tones) take no room.
    pub fn with_pictures(&self, pictures: Pictures) -> Font {
        self.again(Some(pictures), self.inner.fallbacks.clone())
    }

    /// This font with others behind it: a character it has no glyph for is drawn by the first of `fallbacks` that has
    /// one, at the same size and on the same baseline (a name in another script among the letters, a symbol the
    /// font lacks). The line's height stays this font's. A character none of them has is drawn as this font's own
    /// missing glyph.
    pub fn with_fallbacks(&self, fallbacks: &[Font]) -> Font {
        self.again(self.inner.pictures.clone(), fallbacks.to_vec())
    }

    /// The font of the chain at a place: 0 is this one, 1 the first behind it.
    fn font_at(&self, index: usize) -> &Font {
        match index {
            0 => self,
            n => &self.inner.fallbacks[n - 1],
        }
    }

    /// Which font of the chain draws a character this font has no glyph for (this one, if none has).
    fn font_for(&self, c: char) -> usize {
        self.inner
            .fallbacks
            .iter()
            .position(|font| font.inner.font.glyph_id(c).0 != 0)
            .map_or(0, |i| i + 1)
    }

    fn drawn(&self, c: char) -> Drawn {
        let id = self.inner.font.glyph_id(c);
        let Some(pictures) = &self.inner.pictures else {
            return Drawn::Glyph(id);
        };
        if modifies(c) {
            return Drawn::Nothing;
        }
        if !may_be_picture(c) {
            return Drawn::Glyph(id);
        }
        let mut known = self
            .inner
            .pictured
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let picture = known
            .entry(c)
            .or_insert_with(|| pictures(c).map(|image| Arc::new(Mutex::new(image))))
            .clone();
        match picture {
            Some(picture) => Drawn::Picture(picture),
            None if id.0 == 0 => Drawn::Nothing,
            None => Drawn::Glyph(id),
        }
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
        let (width, count) = self.walk(text, size, letter_spacing, |_| {});
        width + letter_spacing * (count.max(1) - 1) as f32
    }

    /// Breaks text into lines no wider than `max_width`: at spaces, and inside a word only when the word alone is
    /// too wide. A line break in the text always starts a new line. Text written with no spaces between its words
    /// (Chinese, Japanese) may break after any of its letters, never before a mark that closes or after one that
    /// opens; and a zero-width space (U+200B) marks a place where a line may end, for the scripts that need a
    /// dictionary to find their words (Thai).
    pub fn wrap(&self, text: &str, size: f32, letter_spacing: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
            if paragraph.chars().any(|c| c == '\u{200B}' || unspaced(c)) {
                self.wrap_unspaced(paragraph, size, letter_spacing, max_width, &mut lines);
                continue;
            }
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
                line = self.cut_wide(word, size, letter_spacing, max_width, &mut lines);
            }
            lines.push(line);
        }
        lines
    }

    /// A word wider than the line, cut where it no longer fits: the lines it fills are pushed, what is left of it
    /// is returned (the line that goes on).
    fn cut_wide(
        &self,
        word: &str,
        size: f32,
        letter_spacing: f32,
        max_width: f32,
        lines: &mut Vec<String>,
    ) -> String {
        let mut piece = String::new();
        for c in word.chars() {
            piece.push(c);
            if self.measure(&piece, size, letter_spacing) > max_width && piece.chars().count() > 1 {
                piece.pop();
                lines.push(std::mem::take(&mut piece));
                piece.push(c);
            }
        }
        piece
    }

    /// The lines of a paragraph that may break between letters (see `wrap`).
    fn wrap_unspaced(
        &self,
        paragraph: &str,
        size: f32,
        letter_spacing: f32,
        max_width: f32,
        lines: &mut Vec<String>,
    ) {
        // The pieces a line is never cut inside, each with whether a space joins it to the piece before it.
        let mut atoms: Vec<(bool, String)> = Vec::new();
        for (i, word) in paragraph.split(' ').enumerate() {
            let mut atom = String::new();
            let mut spaced = i > 0;
            let mut previous: Option<char> = None;
            for c in word.chars() {
                if c == '\u{200B}' {
                    if !atom.is_empty() {
                        atoms.push((spaced, std::mem::take(&mut atom)));
                        spaced = false;
                    }
                    previous = None;
                    continue;
                }
                let breaks = previous
                    .is_some_and(|p| (unspaced(p) || unspaced(c)) && !closes(c) && !opens(p));
                if breaks && !atom.is_empty() {
                    atoms.push((spaced, std::mem::take(&mut atom)));
                    spaced = false;
                }
                atom.push(c);
                previous = Some(c);
            }
            atoms.push((spaced, atom));
        }
        let mut line = String::new();
        for (spaced, atom) in atoms {
            let candidate = match (line.is_empty(), spaced) {
                (true, _) => atom.clone(),
                (false, true) => format!("{line} {atom}"),
                (false, false) => format!("{line}{atom}"),
            };
            if self.measure(&candidate, size, letter_spacing) <= max_width
                || line.is_empty() && atom.is_empty()
            {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            line = self.cut_wide(&atom, size, letter_spacing, max_width, lines);
        }
        lines.push(line);
    }

    pub(crate) fn scale(&self, size: f32) -> f32 {
        size / self.inner.units_per_em
    }

    /// A line laid out: its glyphs and its pictures, each where the pen was.
    pub(crate) fn layout(&self, text: &str, size: f32, letter_spacing: f32) -> Laid {
        let mut laid = Laid {
            glyphs: Vec::new(),
            pictures: Vec::new(),
        };
        self.walk(text, size, letter_spacing, |put| match put {
            Put::Glyph {
                font,
                id,
                x,
                y,
                scale,
            } => laid.glyphs.push(Glyph {
                path: font.glyph_path(id),
                x,
                y,
                scale,
            }),
            Put::Picture(picture, x) => laid.pictures.push((picture, x)),
        });
        laid
    }

    /// The stretches of a line in the order they are seen: which font of the chain draws each, and how.
    fn items(&self, text: &str) -> Vec<Item> {
        let mut items: Vec<Item> = Vec::new();
        for (piece, reversed) in visual_runs(text) {
            let from = items.len();
            for c in piece.chars() {
                // (a place where a line may end: nothing to draw)
                if c == '\u{200B}' {
                    continue;
                }
                // What only joins or tops the letter before it stays with that letter.
                if attaches(c) && items.len() > from {
                    if let Some(Item::Shaped(_, letters, _)) = items.last_mut() {
                        letters.push(c);
                        continue;
                    }
                }
                match self.drawn(c) {
                    Drawn::Nothing => {}
                    Drawn::Picture(picture) => items.push(Item::Picture(picture)),
                    Drawn::Glyph(id) => {
                        let font = if id.0 != 0 { 0 } else { self.font_for(c) };
                        let shaped = cfg!(feature = "shaping") && (reversed || complex(c));
                        let joins = items.len() > from
                            && match items.last() {
                                Some(Item::Plain(f, _)) => !shaped && *f == font,
                                Some(Item::Shaped(f, _, _)) => shaped && *f == font,
                                _ => false,
                            };
                        if joins {
                            if let Some(Item::Plain(_, letters) | Item::Shaped(_, letters, _)) =
                                items.last_mut()
                            {
                                letters.push(c);
                            }
                        } else if shaped {
                            items.push(Item::Shaped(font, c.to_string(), reversed));
                        } else {
                            items.push(Item::Plain(font, c.to_string()));
                        }
                    }
                }
            }
            if reversed {
                items[from..].reverse();
            }
        }
        items
    }

    /// Walks a line as it is laid out, handing `put` every glyph (its font, where its origin sits from the line's
    /// left end and baseline) and every picture (its left edge). Returns the width its letters take with no spacing
    /// between them, and how many letters and pictures there are.
    fn walk(
        &self,
        text: &str,
        size: f32,
        letter_spacing: f32,
        mut put: impl FnMut(Put<'_>),
    ) -> (f32, usize) {
        let mut pen = 0.0;
        let mut width = 0.0;
        let mut count = 0;
        for item in self.items(text) {
            match item {
                Item::Picture(picture) => {
                    let room = size * PICTURE_ADVANCE;
                    put(Put::Picture(
                        picture,
                        pen + (room - size * PICTURE_SIDE) / 2.0,
                    ));
                    pen += room + letter_spacing;
                    width += room;
                    count += 1;
                }
                Item::Plain(index, letters) => {
                    let font = self.font_at(index);
                    let scale = font.scale(size);
                    let mut previous: Option<GlyphId> = None;
                    for c in letters.chars() {
                        let id = font.inner.font.glyph_id(c);
                        if let Some(p) = previous {
                            let kern = font.inner.font.kern_unscaled(p, id) * scale;
                            pen += kern;
                            width += kern;
                        }
                        put(Put::Glyph {
                            font,
                            id,
                            x: pen,
                            y: 0.0,
                            scale,
                        });
                        let advance = font.inner.font.h_advance_unscaled(id) * scale;
                        pen += advance + letter_spacing;
                        width += advance;
                        previous = Some(id);
                        count += 1;
                    }
                }
                Item::Shaped(index, letters, reversed) => {
                    let font = self.font_at(index);
                    let scale = font.scale(size);
                    for glyph in font.shape(&letters, reversed).iter() {
                        put(Put::Glyph {
                            font,
                            id: GlyphId(glyph.id),
                            x: pen + glyph.dx * scale,
                            y: -glyph.dy * scale,
                            scale,
                        });
                        let advance = glyph.advance * scale;
                        pen += advance;
                        width += advance;
                        if glyph.ends {
                            pen += letter_spacing;
                            count += 1;
                        }
                    }
                }
            }
        }
        (width, count)
    }

    /// The glyphs of a stretch that reads right only once shaped, in the order they are seen.
    #[cfg(feature = "shaping")]
    fn shape(&self, letters: &str, reversed: bool) -> Arc<Vec<Placed>> {
        let key = (letters.to_string(), reversed);
        let mut kept = self
            .inner
            .shaped
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(found) = kept.get(&key) {
            return found.clone();
        }
        let mut placed = Vec::new();
        if let Some(face) =
            rustybuzz::Face::from_slice(self.inner.font.font_data(), self.inner.face_index)
        {
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(letters);
            buffer.set_direction(if reversed {
                rustybuzz::Direction::RightToLeft
            } else {
                rustybuzz::Direction::LeftToRight
            });
            let shaped = rustybuzz::shape(&face, &[], buffer);
            let infos = shaped.glyph_infos();
            for (i, (info, at)) in infos.iter().zip(shaped.glyph_positions()).enumerate() {
                let ends = match infos.get(i + 1) {
                    Some(next) => next.cluster != info.cluster,
                    None => true,
                };
                placed.push(Placed {
                    id: info.glyph_id as u16,
                    advance: at.x_advance as f32,
                    dx: at.x_offset as f32,
                    dy: at.y_offset as f32,
                    ends,
                });
            }
        }
        let placed = Arc::new(placed);
        if kept.len() >= SHAPED_KEPT {
            kept.clear();
        }
        kept.insert(key, placed.clone());
        placed
    }

    #[cfg(not(feature = "shaping"))]
    fn shape(&self, _letters: &str, _reversed: bool) -> Arc<Vec<Placed>> {
        Arc::new(Vec::new())
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

    #[test]
    fn an_emoji_is_a_picture_with_room_of_its_own() {
        let plain = test_font();
        let red = || Image::from_rgba(2, 2, [200, 30, 30, 255].repeat(4)).ok();
        let font = plain.with_pictures(Arc::new(
            move |c| if c == '\u{1F525}' { red() } else { None },
        ));
        // The fire has a picture: it takes its room on the line and is laid out as a picture, not as a glyph.
        let with = font.measure("ok \u{1F525}", 20.0, 0.0);
        assert!((with - font.measure("ok ", 20.0, 0.0) - 20.0 * PICTURE_ADVANCE).abs() < 0.01);
        let laid = font.layout("ok \u{1F525}", 20.0, 0.0);
        assert_eq!((laid.glyphs.len(), laid.pictures.len()), (3, 1));
        // What only modifies it takes no room; an emoji with no picture and no glyph is left out.
        assert_eq!(font.measure("ok \u{1F525}\u{FE0F}", 20.0, 0.0), with);
        assert_eq!(
            font.measure("ok\u{1F359}", 20.0, 0.0),
            font.measure("ok", 20.0, 0.0)
        );
        // Letters are the font's own, as in the font without pictures.
        assert_eq!(
            font.measure("Pasaporte", 20.0, 0.0),
            plain.measure("Pasaporte", 20.0, 0.0)
        );
    }

    fn fixture(name: &str) -> Font {
        let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        Font::from_bytes(std::fs::read(path).expect("the fixture font")).expect("a font")
    }

    #[test]
    fn a_letter_the_font_lacks_is_drawn_by_the_font_behind_it() {
        let latin = test_font();
        let mincho = fixture("ShipporiMincho-subset.ttf");
        let both = latin.with_fallbacks(std::slice::from_ref(&mincho));
        // Alone, the Latin font has only its missing glyph for the character; behind it, the other font draws it.
        let width = mincho.measure("鬼", 30.0, 0.0);
        assert!(width > 20.0);
        assert!((both.measure("鬼", 30.0, 0.0) - width).abs() < 0.01);
        assert!(
            (both.measure("Oni 鬼", 30.0, 0.0) - latin.measure("Oni ", 30.0, 0.0) - width).abs()
                < 0.01
        );
        let laid = both.layout("a鬼", 30.0, 0.0);
        assert_eq!(laid.glyphs.len(), 2);
        assert!(laid.glyphs.iter().all(|g| g.path.is_some()));
        // (each glyph in its own font's units)
        assert!((laid.glyphs[0].scale - latin.scale(30.0)).abs() < 1e-6);
        assert!((laid.glyphs[1].scale - mincho.scale(30.0)).abs() < 1e-6);
        // What the font has is its own, to the last bit.
        assert_eq!(
            both.measure("Pasaporte", 20.0, 1.5),
            latin.measure("Pasaporte", 20.0, 1.5)
        );
    }

    #[test]
    fn text_with_no_spaces_breaks_between_its_letters_but_not_before_a_closing_mark() {
        let font = fixture("ShipporiMincho-subset.ttf");
        let text = "障子の向こう、鬼。「鬼」の向こう。";
        let width = font.measure("障子の", 20.0, 0.0) + 1.0;
        let lines = font.wrap(text, 20.0, 0.0, width);
        assert!(lines.len() > 2, "{lines:?}");
        assert_eq!(lines.concat(), text);
        for line in &lines {
            assert!(!line.starts_with(['、', '。', '」']), "{lines:?}");
            assert!(!line.ends_with('「'), "{lines:?}");
        }
    }

    #[test]
    fn a_zero_width_space_is_a_place_to_break_and_takes_no_room() {
        let font = test_font();
        assert_eq!(
            font.measure("uno\u{200B}dos", 20.0, 0.0),
            font.measure("unodos", 20.0, 0.0)
        );
        let width = font.measure("unodos", 20.0, 0.0) + 1.0;
        let lines = font.wrap("uno\u{200B}dos\u{200B}tres\u{200B}cuatro", 20.0, 0.0, width);
        assert_eq!(lines, vec!["unodos", "tres", "cuatro"]);
        // (with room for all of it, the marks leave no trace)
        assert_eq!(
            font.wrap("uno\u{200B}dos", 20.0, 0.0, 1000.0),
            vec!["unodos"]
        );
    }

    #[cfg(feature = "shaping")]
    #[test]
    fn hebrew_is_laid_from_right_to_left() {
        let hebrew = fixture("NotoSansHebrew-subset.ttf");
        let (alef, bet) = (
            hebrew.measure("א", 30.0, 0.0),
            hebrew.measure("ב", 30.0, 0.0),
        );
        assert!(alef > 5.0 && bet > 5.0 && (alef - bet).abs() > 0.5);
        // «אב» is written alef then bet and read from the right: on the line the bet comes first.
        let laid = hebrew.layout("אב", 30.0, 0.0);
        assert_eq!(laid.glyphs.len(), 2);
        assert!((laid.glyphs[0].x).abs() < 0.01);
        assert!(
            (laid.glyphs[1].x - bet).abs() < 0.5,
            "{} {bet}",
            laid.glyphs[1].x
        );
        assert!((hebrew.measure("אב", 30.0, 0.0) - alef - bet).abs() < 0.5);
        // Among Latin letters the Hebrew word keeps its own direction, drawn by the font behind.
        let both = test_font().with_fallbacks(std::slice::from_ref(&hebrew));
        let laid = both.layout("ab אב", 30.0, 0.0);
        assert_eq!(laid.glyphs.len(), 5);
        let latin = test_font().measure("ab ", 30.0, 0.0);
        assert!((laid.glyphs[3].x - latin).abs() < 0.5);
        assert!((laid.glyphs[4].x - latin - bet).abs() < 0.5);
    }

    #[cfg(feature = "shaping")]
    #[test]
    fn devanagari_letters_join_and_change_places() {
        let font = fixture("NotoSansDevanagari-subset.ttf");
        // ka + virama + ssa is one conjunct, not three glyphs in a row.
        let joined = font.layout("क्ष", 30.0, 0.0);
        assert!(joined.glyphs.len() < 3, "{}", joined.glyphs.len());
        // The short i is written after its consonant and drawn before it.
        let ka = font.layout("क", 30.0, 0.0);
        let ki = font.layout("कि", 30.0, 0.0);
        assert_eq!((ka.glyphs.len(), ki.glyphs.len()), (1, 2));
        assert!(
            ki.glyphs[1].x > 1.0,
            "the consonant stands after its vowel sign"
        );
        assert!(font.measure("कि", 30.0, 0.0) > font.measure("क", 30.0, 0.0));
        // A letter and its marks are one letter for the spacing.
        let spaced = font.measure("नमस्ते", 30.0, 4.0) - font.measure("नमस्ते", 30.0, 0.0);
        assert!(spaced > 0.0 && spaced < 4.0 * 5.0, "{spaced}");
    }
}
