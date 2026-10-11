//! One line of text → positioned glyphs.
//!
//! Plain Latin/Cyrillic text takes the fast path (one glyph per character,
//! fontdue advances). Lines with complex scripts (Devanagari, Thai, Khmer,
//! Myanmar...) or right-to-left scripts (Hebrew, Arabic) are split into
//! directional runs, each run into per-font segments, and complex segments are
//! shaped with rustybuzz (glyph substitution, reordering, mark positioning).
//! Right-to-left runs are laid out in visual order, so Hebrew reads correctly
//! even inside left-aligned UI.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::fonts::{FontChain, LoadedFont};

/// What to rasterize: a Unicode character (fontdue maps it) or a glyph index
/// produced by the shaper.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GlyphRef {
    Char(char),
    Index(u16),
}

impl GlyphRef {
    pub fn code(self) -> u32 {
        match self {
            GlyphRef::Char(c) => c as u32,
            GlyphRef::Index(i) => 0x8000_0000 | u32::from(i),
        }
    }
}

pub struct Placed {
    pub font: Arc<LoadedFont>,
    pub glyph: GlyphRef,
    /// Pen position (left edge) relative to the line start.
    pub x: f32,
    /// Vertical offset, positive = up.
    pub dy: f32,
}

pub fn is_rtl(ch: char) -> bool {
    matches!(ch as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

fn is_complex(ch: char) -> bool {
    matches!(ch as u32,
        0x0900..=0x0DFF // Indic
        | 0x0E00..=0x0EFF // Thai, Lao
        | 0x0F00..=0x0FFF // Tibetan
        | 0x1000..=0x109F // Myanmar
        | 0x1780..=0x17FF // Khmer
        | 0xA8E0..=0xA8FF
    ) || is_rtl(ch)
}

fn is_mark(ch: char) -> bool {
    // Combining marks stay with the base character's font.
    matches!(ch as u32,
        0x0300..=0x036F | 0x0591..=0x05C7 | 0x064B..=0x065F | 0x0900..=0x0903
        | 0x093A..=0x094F | 0x0951..=0x0957 | 0x0962..=0x0963 | 0x0E31 | 0x0E34..=0x0E3A
        | 0x0E47..=0x0E4E | 0x200C | 0x200D)
}

fn needs_layout(text: &str) -> bool {
    text.chars().any(|c| is_complex(c) || (c as u32) >= 0x2E80)
}

/// Shaped glyphs: (glyph id, advance, x offset, y offset).
type Shaped = Rc<Vec<(u16, f32, f32, f32)>>;

thread_local! {
    static SHAPED: RefCell<HashMap<(usize, String, u32, bool), Shaped>> =
        RefCell::new(HashMap::new());
}
use std::rc::Rc;

/// Shape `text` with `font` at `size`.
fn shape(font: &LoadedFont, text: &str, size: f32, rtl: bool) -> Shaped {
    let key = (font.key, text.to_owned(), size.to_bits(), rtl);
    if let Some(hit) = SHAPED.with(|c| c.borrow().get(&key).cloned()) {
        return hit;
    }
    let mut out = Vec::new();
    if let Some(face) = rustybuzz::Face::from_slice(font.bytes.as_slice(), 0) {
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        buffer.set_direction(if rtl {
            rustybuzz::Direction::RightToLeft
        } else {
            rustybuzz::Direction::LeftToRight
        });
        let glyphs = rustybuzz::shape(&face, &[], buffer);
        let upem = f32::from(face.units_per_em().max(1) as u16).max(1.0);
        let scale = size / upem;
        for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
            out.push((
                info.glyph_id as u16,
                pos.x_advance as f32 * scale,
                pos.x_offset as f32 * scale,
                pos.y_offset as f32 * scale,
            ));
        }
    } else {
        // No shaping data: fall back to characters (reversed for RTL).
        let chars: Vec<char> = if rtl {
            text.chars().rev().collect()
        } else {
            text.chars().collect()
        };
        for ch in chars {
            let idx = font.font.lookup_glyph_index(ch);
            let adv = font.font.metrics_indexed(idx, size).advance_width;
            out.push((idx, adv, 0.0, 0.0));
        }
    }
    let out = Rc::new(out);
    SHAPED.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 8192 {
            c.clear();
        }
        c.insert(key, out.clone());
    });
    out
}

/// Split into directional runs: (text, rtl). Neutrals join the run before them.
fn bidi_runs(text: &str) -> (Vec<(String, bool)>, bool) {
    let mut runs: Vec<(String, bool)> = Vec::new();
    let base_rtl = text.chars().find(|c| c.is_alphabetic()).is_some_and(is_rtl);
    let mut cur = String::new();
    let mut cur_rtl: Option<bool> = None;
    for ch in text.chars() {
        let strong = if is_rtl(ch) {
            Some(true)
        } else if ch.is_alphanumeric() {
            Some(false)
        } else {
            None
        };
        match (strong, cur_rtl) {
            (Some(d), Some(c)) if d != c => {
                runs.push((std::mem::take(&mut cur), c));
                cur_rtl = Some(d);
            }
            (Some(d), None) => cur_rtl = Some(d),
            _ => {}
        }
        cur.push(ch);
    }
    if !cur.is_empty() {
        runs.push((cur, cur_rtl.unwrap_or(base_rtl)));
    }
    if base_rtl {
        // Spaces at a run boundary read better on the other side once reversed.
        runs.reverse();
    }
    (runs, base_rtl)
}

/// Lay out one line; returns glyphs and the total advance.
pub fn layout(chain: &FontChain, text: &str, size: f32) -> (Vec<Placed>, f32) {
    let mut placed = Vec::new();
    let mut pen = 0.0f32;
    if !needs_layout(text) {
        for ch in text.chars() {
            let f = chain.font_for(ch);
            let adv = f.font.metrics(ch, size).advance_width;
            placed.push(Placed {
                font: f.clone(),
                glyph: GlyphRef::Char(ch),
                x: pen,
                dy: 0.0,
            });
            pen += adv;
        }
        return (placed, pen);
    }
    let (runs, _) = bidi_runs(text);
    for (run, rtl) in runs {
        // Per-font segments (marks stay with their base).
        let mut segs: Vec<(Arc<LoadedFont>, String)> = Vec::new();
        for ch in run.chars() {
            let f = if is_mark(ch) && !segs.is_empty() {
                segs.last()
                    .map(|s| s.0.clone())
                    .unwrap_or_else(|| chain.font_for(ch).clone())
            } else {
                chain.font_for(ch).clone()
            };
            match segs.last_mut() {
                Some((sf, s)) if sf.key == f.key => s.push(ch),
                _ => segs.push((f, ch.to_string())),
            }
        }
        if rtl {
            segs.reverse();
        }
        for (f, seg) in segs {
            let complex = rtl || seg.chars().any(is_complex);
            if complex {
                for (gid, adv, xo, yo) in shape(&f, &seg, size, rtl).iter() {
                    placed.push(Placed {
                        font: f.clone(),
                        glyph: GlyphRef::Index(*gid),
                        x: pen + xo,
                        dy: *yo,
                    });
                    pen += adv;
                }
            } else {
                for ch in seg.chars() {
                    let adv = f.font.metrics(ch, size).advance_width;
                    placed.push(Placed {
                        font: f.clone(),
                        glyph: GlyphRef::Char(ch),
                        x: pen,
                        dy: 0.0,
                    });
                    pen += adv;
                }
            }
        }
    }
    (placed, pen)
}

/// Width of one line.
pub fn measure(chain: &FontChain, text: &str, size: f32) -> f32 {
    if !needs_layout(text) {
        return text
            .chars()
            .map(|ch| chain.font_for(ch).font.metrics(ch, size).advance_width)
            .sum();
    }
    layout(chain, text, size).1
}
