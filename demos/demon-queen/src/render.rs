//! Framebuffer canvas and system-font text rendering for the demon queen host.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use ab_glyph::{point, Font as AbFont, FontRef, GlyphId, ScaleFont};
use unicode_segmentation::UnicodeSegmentation;
use velvet_math::{Rect, Vec2};
use velvet_style::{Color as StyleColor, ComputedStyle};
use velvet_ui::{
    apply_computed_style, layout_nine_patch_with_destination_margins, NinePatchMargins, UiStyle,
};

struct FontFace {
    bytes: Arc<[u8]>,
    collection_index: u32,
    glyph_cache: RefCell<HashMap<GlyphCacheKey, CachedGlyph>>,
    glyph_cache_bytes: Cell<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphCacheKey {
    glyph_id: u16,
    size_bits: u32,
}

#[derive(Clone)]
struct CachedGlyph {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    coverage: Arc<[u8]>,
}

static FONT_BYTES: OnceLock<Mutex<HashMap<PathBuf, Weak<[u8]>>>> = OnceLock::new();

// Text sizes can be fractional when a label is fitted or animated.  Keeping
// every `(glyph, size)` raster forever turns that normal variation into a
// process-wide leak.  A small byte budget is enough for the active UI while
// putting a hard ceiling on the decoded coverage retained by the renderer.
const GLYPH_CACHE_BUDGET_BYTES: usize = 16 * 1024 * 1024;

fn load_font_bytes(path: &Path) -> Option<Arc<[u8]>> {
    let cache = FONT_BYTES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(bytes) = cache.lock().ok()?.get(path).and_then(Weak::upgrade) {
        return Some(bytes);
    }

    let bytes: Arc<[u8]> = Arc::from(std::fs::read(path).ok()?.into_boxed_slice());
    cache
        .lock()
        .ok()?
        .insert(path.to_path_buf(), Arc::downgrade(&bytes));
    Some(bytes)
}

impl FontFace {
    fn load(path: &Path, collection_indices: &[u32]) -> Option<Self> {
        let bytes = load_font_bytes(path)?;
        for &collection_index in collection_indices {
            if FontRef::try_from_slice_and_index(bytes.as_ref(), collection_index).is_ok() {
                return Some(Self {
                    bytes: Arc::clone(&bytes),
                    collection_index,
                    glyph_cache: RefCell::new(HashMap::new()),
                    glyph_cache_bytes: Cell::new(0),
                });
            }
        }
        None
    }

    fn raster_face(&self) -> Option<FontRef<'_>> {
        FontRef::try_from_slice_and_index(self.bytes.as_ref(), self.collection_index).ok()
    }

    /// `fontdue` and HarfBuzz interpret the requested size as pixels per em.
    /// `ab_glyph` interprets `PxScale` as the full font height, so convert it
    /// to preserve the existing layout and match shaped advances exactly.
    fn raster_scale(raster: &FontRef<'_>, size: f32) -> f32 {
        let units_per_em = raster.units_per_em().unwrap_or(0.0);
        let font_height = raster.height_unscaled();
        if units_per_em > 0.0 && font_height > 0.0 {
            size * font_height / units_per_em
        } else {
            size
        }
    }

    fn shape(&self, value: &str, size: f32) -> Option<Vec<ShapedGlyph>> {
        let face = rustybuzz::Face::from_slice(self.bytes.as_ref(), self.collection_index)?;
        let units_per_em = face.units_per_em() as f32;
        if units_per_em <= 0.0 {
            return None;
        }
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(value);
        buffer.guess_segment_properties();
        let shaped = rustybuzz::shape(&face, &[], buffer);
        let scale = size / units_per_em;
        Some(
            shaped
                .glyph_infos()
                .iter()
                .zip(shaped.glyph_positions())
                .map(|(info, position)| ShapedGlyph {
                    glyph_id: info.glyph_id as u16,
                    x_advance: position.x_advance as f32 * scale,
                    x_offset: position.x_offset as f32 * scale,
                    y_offset: position.y_offset as f32 * scale,
                })
                .collect(),
        )
    }

    fn width(&self, value: &str, size: f32) -> f32 {
        self.shape(value, size)
            .map(|glyphs| glyphs.iter().map(|glyph| glyph.x_advance).sum())
            .unwrap_or_else(|| {
                let Some(raster) = self.raster_face() else {
                    return 0.0;
                };
                let scaled = raster.as_scaled(Self::raster_scale(&raster, size));
                value
                    .chars()
                    .map(|ch| scaled.h_advance(raster.glyph_id(ch)))
                    .sum()
            })
    }

    fn ascent(&self, size: f32) -> f32 {
        self.raster_face()
            .map(|raster| {
                let scale = Self::raster_scale(&raster, size);
                raster.as_scaled(scale).ascent()
            })
            .unwrap_or(size * 0.8)
    }

    fn fallback_shape(&self, value: &str, size: f32) -> Vec<ShapedGlyph> {
        let Some(raster) = self.raster_face() else {
            return Vec::new();
        };
        let scaled = raster.as_scaled(Self::raster_scale(&raster, size));
        value
            .chars()
            .filter(|ch| *ch != '\n')
            .map(|ch| {
                let glyph_id = raster.glyph_id(ch);
                ShapedGlyph {
                    glyph_id: glyph_id.0,
                    x_advance: scaled.h_advance(glyph_id),
                    x_offset: 0.0,
                    y_offset: 0.0,
                }
            })
            .collect()
    }

    fn rasterize(&self, glyph_id: u16, size: f32) -> Option<CachedGlyph> {
        let key = GlyphCacheKey {
            glyph_id,
            size_bits: size.to_bits(),
        };
        if let Some(glyph) = self.glyph_cache.borrow().get(&key) {
            return Some(glyph.clone());
        }

        let raster = self.raster_face()?;
        let scale = Self::raster_scale(&raster, size);
        let glyph = GlyphId(glyph_id).with_scale_and_position(scale, point(0.0, 0.0));
        let outlined = raster.outline_glyph(glyph)?;
        let bounds = outlined.px_bounds();
        let width = (bounds.max.x - bounds.min.x).max(0.0).round() as usize;
        let height = (bounds.max.y - bounds.min.y).max(0.0).round() as usize;
        if width == 0 || height == 0 {
            return None;
        }
        let mut coverage = vec![0; width * height];
        outlined.draw(|x, y, value| {
            let index = y as usize * width + x as usize;
            if let Some(pixel) = coverage.get_mut(index) {
                *pixel = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        });
        let glyph = CachedGlyph {
            left: bounds.min.x.round() as i32,
            top: bounds.min.y.round() as i32,
            width,
            height,
            coverage: Arc::from(coverage.into_boxed_slice()),
        };
        let glyph_bytes = glyph.coverage.len();
        let mut cache = self.glyph_cache.borrow_mut();
        let mut cache_bytes = self.glyph_cache_bytes.get();
        if glyph_bytes >= GLYPH_CACHE_BUDGET_BYTES
            || cache_bytes.saturating_add(glyph_bytes) > GLYPH_CACHE_BUDGET_BYTES
        {
            cache.clear();
            cache_bytes = 0;
        }
        cache.insert(key, glyph.clone());
        self.glyph_cache_bytes
            .set(cache_bytes.saturating_add(glyph_bytes));
        Some(glyph)
    }
}

#[derive(Clone, Copy)]
struct ShapedGlyph {
    glyph_id: u16,
    x_advance: f32,
    x_offset: f32,
    y_offset: f32,
}

/// Loaded system fonts (regular + bold) with script-aware Windows profiles.
pub struct Fonts {
    regular: FontFace,
    bold: FontFace,
    display: FontFace,
    /// The web scenes' faces (Anton, Nunito ExtraBold, Bold Italic, SemiBold Italic) from
    /// `data/fonts`; empty when they are missing (the interface's own are used then).
    stage: Vec<FontFace>,
}

/// The faces the visual novel shares with the web scenes and the world's stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageFace {
    Display,
    Round,
    RoundItalic,
    RoundSoft,
}

/// The web scenes' fonts, shipped with the game.
fn stage_faces() -> Vec<FontFace> {
    let dir = crate::assets_root()
        .parent()
        .map(|p| p.join("fonts"))
        .unwrap_or_else(|| PathBuf::from("data/fonts"));
    let faces: Vec<FontFace> = [
        "Anton-Regular.ttf",
        "Nunito-ExtraBold.ttf",
        "Nunito-BoldItalic.ttf",
        "Nunito-SemiBoldItalic.ttf",
    ]
    .iter()
    .filter_map(|f| FontFace::load(&dir.join(f), &[0]))
    .collect();
    if faces.len() == 4 {
        faces
    } else {
        Vec::new()
    }
}

type FontProfile<'a> = (
    &'a [&'a str],
    &'a [&'a str],
    &'a [&'a str],
    &'a [u32],
    &'a [u32],
    &'a [u32],
);

fn system_font_directories() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        candidates.push(PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    candidates
}

fn font_profile(language: &str) -> FontProfile<'static> {
    match language {
        "zh-Hans" => (
            &["msyh.ttc", "simsun.ttc"],
            &["msyhbd.ttc", "msyh.ttc"],
            &["msyhbd.ttc", "msyh.ttc"],
            &[0],
            &[0],
            &[0],
        ),
        "zh-Hant" => (
            &["msjh.ttc", "mingliub.ttc"],
            &["msjhbd.ttc", "msjh.ttc"],
            &["msjhbd.ttc", "msjh.ttc"],
            &[0],
            &[0],
            &[0],
        ),
        "hi" => (
            &["Nirmala.ttc", "Nirmala.ttf"],
            &["Nirmala.ttc", "Nirmala.ttf"],
            &["Nirmala.ttc", "Nirmala.ttf"],
            &[0, 1, 2],
            &[1, 0, 2],
            &[1, 0, 2],
        ),
        "ja" => (
            &["YuGothM.ttc", "YuGothR.ttc", "meiryo.ttc"],
            &["YuGothB.ttc", "YuGothM.ttc", "meiryob.ttc"],
            &["YuGothB.ttc", "YuGothM.ttc"],
            &[0],
            &[0],
            &[0],
        ),
        "ko" => (
            &["malgun.ttf", "malgunsl.ttf"],
            &["malgunbd.ttf", "malgun.ttf"],
            &["malgunbd.ttf", "malgun.ttf"],
            &[0],
            &[0],
            &[0],
        ),
        _ => (
            &["segoeui.ttf", "arial.ttf", "tahoma.ttf"],
            &["segoeuib.ttf", "arialbd.ttf", "tahomabd.ttf"],
            &["constan.ttf", "cambria.ttc", "georgia.ttf"],
            &[0],
            &[0],
            &[0],
        ),
    }
}

fn pick_font(dir: &Path, names: &[&str], indices: &[u32]) -> Option<FontFace> {
    names
        .iter()
        .find_map(|name| FontFace::load(&dir.join(name), indices))
}

impl Fonts {
    /// Load from the platform font directories; `None` when unavailable
    /// (headless runs never need this).
    pub fn new() -> Option<Self> {
        Self::new_for_language("en")
    }

    /// Load a font profile with native glyph coverage for the active locale.
    pub fn new_for_language(language: &str) -> Option<Self> {
        for dir in system_font_directories() {
            if let Some(fonts) = Self::from_dir_for_language(&dir, language) {
                return Some(fonts);
            }
        }
        None
    }

    /// Try a specific directory and language profile.
    pub fn from_dir_for_language(dir: &Path, language: &str) -> Option<Self> {
        let profile = font_profile(language);
        if let (Some(regular), Some(bold), Some(display)) = (
            pick_font(dir, profile.0, profile.3),
            pick_font(dir, profile.1, profile.4),
            pick_font(dir, profile.2, profile.5),
        ) {
            return Some(Self {
                regular,
                bold,
                display,
                stage: stage_faces(),
            });
        }

        // Portable Latin fallback for development hosts.
        let regular_pairs: [&[&str]; 2] = [
            &["segoeui.ttf", "arial.ttf", "tahoma.ttf"],
            &["DejaVuSans.ttf", "Ubuntu-R.ttf"],
        ];
        let bold_pairs: [&[&str]; 2] = [
            &["segoeuib.ttf", "arialbd.ttf", "tahomabd.ttf"],
            &["DejaVuSans-Bold.ttf", "Ubuntu-Bold.ttf"],
        ];
        let display_pairs: [&[&str]; 2] = [
            &["constan.ttf", "cambria.ttc", "georgia.ttf"],
            &["DejaVuSerif-Bold.ttf", "Ubuntu-B.ttf"],
        ];
        let pick_groups = |groups: &[&[&str]]| {
            for group in groups {
                for name in *group {
                    let path = dir.join(name);
                    if let Some(font) = FontFace::load(&path, &[0]) {
                        return Some(font);
                    }
                }
            }
            None
        };
        Some(Self {
            regular: pick_groups(&regular_pairs)?,
            bold: pick_groups(&bold_pairs)?,
            display: pick_groups(&display_pairs).or_else(|| pick_groups(&bold_pairs))?,
            stage: stage_faces(),
        })
    }

    fn font(&self, bold: bool) -> &FontFace {
        if bold {
            &self.bold
        } else {
            &self.regular
        }
    }

    /// Advance width of one character at `size`.
    pub(crate) fn advance(&self, ch: char, size: f32, bold: bool) -> f32 {
        self.font(bold).width(&ch.to_string(), size)
    }

    pub(crate) fn display_width(&self, value: &str, size: f32) -> i32 {
        self.display.width(value, size).round() as i32
    }

    pub(crate) fn text_width(&self, value: &str, size: f32, bold: bool) -> i32 {
        self.font(bold).width(value, size).round() as i32
    }

    fn stage_font(&self, face: StageFace) -> &FontFace {
        let index = match face {
            StageFace::Display => 0,
            StageFace::Round => 1,
            StageFace::RoundItalic => 2,
            StageFace::RoundSoft => 3,
        };
        self.stage.get(index).unwrap_or(match face {
            StageFace::Display => &self.display,
            _ => &self.bold,
        })
    }

    pub(crate) fn stage_width(&self, face: StageFace, value: &str, size: f32) -> i32 {
        self.stage_font(face).width(value, size).round() as i32
    }

    /// `value` broken into lines no wider than `max` in `face` (words kept whole).
    pub(crate) fn stage_wrap(
        &self,
        face: StageFace,
        value: &str,
        size: f32,
        max: i32,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in value.split('\n') {
            let mut line = String::new();
            for word in paragraph.split(' ') {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if self.stage_width(face, &candidate, size) <= max || line.is_empty() {
                    line = candidate;
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_owned();
                }
            }
            lines.push(line);
        }
        lines
    }
}

/// One lightweight script-specific face used for language autonyms.
///
/// Keeping only one face per script avoids loading three full CJK font
/// collections merely to paint the language selector.
pub struct NativeLabelFont {
    face: FontFace,
}

impl NativeLabelFont {
    pub fn new_for_language(language: &str) -> Option<Self> {
        let profile = font_profile(language);
        system_font_directories()
            .into_iter()
            .find_map(|dir| pick_font(&dir, profile.0, profile.3).map(|face| Self { face }))
    }

    pub(crate) fn text_width(&self, value: &str, size: f32) -> i32 {
        self.face.width(value, size).round() as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ChoiceTreatment {
    Narrative,
    HubVisit,
    Advance,
}

/// ARGB canvas (0xAARRGGBB, alpha 255 = opaque).
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub pixels: Vec<u32>,
}

impl Canvas {
    /// New canvas filled with `color`.
    pub fn new(w: usize, h: usize, color: u32) -> Self {
        Self {
            w,
            h,
            pixels: vec![color; w * h],
        }
    }

    /// Fill everything.
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    /// Opaque rectangle (clipped).
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        let x0 = x.max(0) as usize;
        let y0 = y.max(0) as usize;
        let x1 = (x + w).min(self.w as i32).max(0) as usize;
        let y1 = (y + h).min(self.h as i32).max(0) as usize;
        for yy in y0..y1.min(self.h) {
            for xx in x0..x1.min(self.w) {
                self.pixels[yy * self.w + xx] = blend(self.pixels[yy * self.w + xx], color);
            }
        }
    }

    /// Vertical gradient between two colors.
    pub fn vgradient(&mut self, top: u32, bottom: u32) {
        let rows = self.h.max(1);
        for row in 0..rows {
            let t = row as f32 / rows as f32;
            let color = lerp(top, bottom, t);
            for pixel in self.pixels[row * self.w..(row + 1) * self.w].iter_mut() {
                *pixel = color;
            }
        }
    }

    /// Filled circle (for moons, orbs).
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        let radius = r.max(1);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let normalized =
                    (dx as f32 / radius as f32).powi(2) + (dy as f32 / radius as f32).powi(2);
                if normalized <= 1.0 {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && py >= 0 && (px as usize) < self.w && (py as usize) < self.h {
                        let idx = py as usize * self.w + px as usize;
                        self.pixels[idx] = blend(self.pixels[idx], color);
                    }
                }
            }
        }
    }

    /// Filled polygon using an even-odd scanline pass.
    pub fn polygon(&mut self, points: &[(i32, i32)], color: u32) {
        if points.len() < 3 {
            return;
        }
        let min_y = points.iter().map(|point| point.1).min().unwrap_or(0).max(0);
        let max_y = points
            .iter()
            .map(|point| point.1)
            .max()
            .unwrap_or(-1)
            .min(self.h as i32 - 1);
        for y in min_y..=max_y {
            let scan_y = y as f32 + 0.5;
            let mut intersections = Vec::with_capacity(points.len());
            for index in 0..points.len() {
                let (x1, y1) = points[index];
                let (x2, y2) = points[(index + 1) % points.len()];
                let (y1, y2) = (y1 as f32, y2 as f32);
                if (y1 <= scan_y && y2 > scan_y) || (y2 <= scan_y && y1 > scan_y) {
                    let t = (scan_y - y1) / (y2 - y1);
                    intersections.push(x1 as f32 + (x2 - x1) as f32 * t);
                }
            }
            intersections.sort_by(f32::total_cmp);
            for pair in intersections.chunks_exact(2) {
                let x0 = pair[0].ceil() as i32;
                let x1 = pair[1].floor() as i32;
                self.rect(x0, y, x1 - x0 + 1, 1, color);
            }
        }
    }

    /// An anti-aliased convex polygon (any winding; `color`'s alpha kept). Each pixel row is
    /// sampled on four sub-rows and every crossing is measured to the fraction of a pixel, so
    /// slanted plates, slashes and cut corners keep smooth edges.
    pub fn convex(&mut self, points: &[(f32, f32)], color: u32) {
        if points.len() < 3 || (color >> 24) == 0 {
            return;
        }
        let fold = |f: fn(f32, f32) -> f32, start: f32, axis: usize| {
            points
                .iter()
                .map(|p| if axis == 0 { p.0 } else { p.1 })
                .fold(start, f)
        };
        let min_y = fold(f32::min, f32::MAX, 1).floor().max(0.0) as i32;
        let max_y = (fold(f32::max, f32::MIN, 1).ceil() as i32).min(self.h as i32 - 1);
        let min_x = fold(f32::min, f32::MAX, 0).floor().max(0.0) as i32;
        let max_x = (fold(f32::max, f32::MIN, 0).ceil() as i32).min(self.w as i32 - 1);
        if min_y > max_y || min_x > max_x {
            return;
        }
        let width = (max_x - min_x + 1) as usize;
        let mut coverage = vec![0.0f32; width + 1];
        let n = points.len();
        for y in min_y..=max_y {
            let (mut lo, mut hi) = (usize::MAX, 0usize);
            for sub in 0..4 {
                let sy = y as f32 + (sub as f32 + 0.5) * 0.25;
                let (mut xl, mut xr) = (f32::MAX, f32::MIN);
                for i in 0..n {
                    let (x1, y1) = points[i];
                    let (x2, y2) = points[(i + 1) % n];
                    if (y1 <= sy && y2 > sy) || (y2 <= sy && y1 > sy) {
                        let x = x1 + (x2 - x1) * (sy - y1) / (y2 - y1);
                        xl = xl.min(x);
                        xr = xr.max(x);
                    }
                }
                if xl >= xr {
                    continue;
                }
                let a = (xl - min_x as f32).max(0.0);
                let b = (xr - min_x as f32).min(width as f32);
                if a >= b {
                    continue;
                }
                let (ia, ib) = (a.floor() as usize, (b.floor() as usize).min(width));
                if ia == ib {
                    coverage[ia] += 0.25 * (b - a);
                } else {
                    coverage[ia] += 0.25 * (1.0 - (a - ia as f32));
                    for c in &mut coverage[ia + 1..ib] {
                        *c += 0.25;
                    }
                    if ib < width {
                        coverage[ib] += 0.25 * (b - ib as f32);
                    }
                }
                lo = lo.min(ia);
                hi = hi.max(ib.min(width - 1));
            }
            if lo == usize::MAX {
                continue;
            }
            let row = y as usize * self.w + min_x as usize;
            for k in lo..=hi {
                let c = coverage[k].min(1.0);
                coverage[k] = 0.0;
                if c > 0.003 {
                    let pixel = &mut self.pixels[row + k];
                    *pixel = blend(
                        *pixel,
                        if c >= 0.999 {
                            color
                        } else {
                            scale_alpha(color, c)
                        },
                    );
                }
            }
        }
    }

    /// An anti-aliased straight stroke from `a` to `b`, `width` px wide.
    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), width: f32, color: u32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = (dx * dx + dy * dy).sqrt();
        if length < 1e-3 {
            return;
        }
        let (nx, ny) = (-dy / length * width * 0.5, dx / length * width * 0.5);
        self.convex(
            &[
                (a.0 + nx, a.1 + ny),
                (b.0 + nx, b.1 + ny),
                (b.0 - nx, b.1 - ny),
                (a.0 - nx, a.1 - ny),
            ],
            color,
        );
    }

    /// Copy an RGBA8 buffer at native pixel size (clipped to the canvas).
    pub fn blit_rgba(&mut self, x: i32, y: i32, bitmap: &crate::ArtBitmap) {
        for row in 0..bitmap.h {
            let py = y + row as i32;
            if py < 0 || py as usize >= self.h {
                continue;
            }
            let dst_row = py as usize * self.w;
            let src_row = row * bitmap.w;
            for col in 0..bitmap.w {
                let px = x + col as i32;
                if px < 0 || px as usize >= self.w {
                    continue;
                }
                let src = (src_row + col) * 4;
                let alpha = bitmap.rgba[src + 3] as u32;
                if alpha == 255 {
                    self.pixels[dst_row + px as usize] = 0xff00_0000
                        | ((bitmap.rgba[src] as u32) << 16)
                        | ((bitmap.rgba[src + 1] as u32) << 8)
                        | bitmap.rgba[src + 2] as u32;
                } else if alpha > 0 {
                    let idx = dst_row + px as usize;
                    self.pixels[idx] = blend(
                        self.pixels[idx],
                        ((alpha) << 24)
                            | ((bitmap.rgba[src] as u32) << 16)
                            | ((bitmap.rgba[src + 1] as u32) << 8)
                            | bitmap.rgba[src + 2] as u32,
                    );
                }
            }
        }
    }

    /// Copy a native-size RGBA asset with brightness/opacity feedback.
    /// Pixel coordinates remain 1:1; no resampling occurs.
    pub fn blit_rgba_adjusted(
        &mut self,
        x: i32,
        y: i32,
        bitmap: &crate::ArtBitmap,
        brightness: f32,
        opacity: f32,
    ) {
        let brightness = brightness.clamp(0.0, 1.4);
        let opacity = opacity.clamp(0.0, 1.0);
        for row in 0..bitmap.h {
            let py = y + row as i32;
            if py < 0 || py as usize >= self.h {
                continue;
            }
            let dst_row = py as usize * self.w;
            let src_row = row * bitmap.w;
            for col in 0..bitmap.w {
                let px = x + col as i32;
                if px < 0 || px as usize >= self.w {
                    continue;
                }
                let src = (src_row + col) * 4;
                let alpha = (bitmap.rgba[src + 3] as f32 * opacity).round() as u32;
                if alpha == 0 {
                    continue;
                }
                let adjusted =
                    |channel: u8| (channel as f32 * brightness).round().clamp(0.0, 255.0) as u32;
                let color = (alpha << 24)
                    | (adjusted(bitmap.rgba[src]) << 16)
                    | (adjusted(bitmap.rgba[src + 1]) << 8)
                    | adjusted(bitmap.rgba[src + 2]);
                let idx = dst_row + px as usize;
                self.pixels[idx] = blend(self.pixels[idx], color);
            }
        }
    }

    /// Copy an RGBA asset scaled to a destination rectangle.  Layered VN
    /// sprites use this path so the dialogue remains a composition of the
    /// room, characters and chrome instead of a full-screen CG.
    pub fn blit_rgba_scaled(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        bitmap: &crate::ArtBitmap,
        opacity: f32,
    ) {
        if width <= 0 || height <= 0 || bitmap.w == 0 || bitmap.h == 0 {
            return;
        }
        self.blit_rgba_region_scaled(
            bitmap,
            Rect::from_pos_size(Vec2::ZERO, Vec2::new(bitmap.w as f32, bitmap.h as f32)),
            Rect::from_pos_size(
                Vec2::new(x as f32, y as f32),
                Vec2::new(width as f32, height as f32),
            ),
            opacity.clamp(0.0, 1.0),
            None,
        );
    }

    /// Draw one line of text anchored at the top of its line box; returns
    /// pixel width. Glyph outlines are rasterized lazily and cached by
    /// face/glyph/size, avoiding eager expansion of complete CJK fonts.
    pub fn text(
        &mut self,
        fonts: &Fonts,
        value: &str,
        x: i32,
        y: i32,
        size: f32,
        color: u32,
        bold: bool,
    ) -> i32 {
        self.text_with_font(fonts.font(bold), value, x, y, size, color)
    }

    /// Text in one of the web scenes' faces.
    #[allow(clippy::too_many_arguments)]
    pub fn stage_text(
        &mut self,
        fonts: &Fonts,
        face: StageFace,
        value: &str,
        x: i32,
        y: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        self.text_with_font(fonts.stage_font(face), value, x, y, size, color)
    }

    /// Text painted with the script-specific face assigned to an autonym.
    pub fn native_label_text(
        &mut self,
        font: &NativeLabelFont,
        value: &str,
        x: i32,
        y: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        self.text_with_font(&font.face, value, x, y, size, color)
    }

    /// Display-serif text for editorial title treatments.
    pub fn display_text(
        &mut self,
        fonts: &Fonts,
        value: &str,
        x: i32,
        y: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        self.text_with_font(&fonts.display, value, x, y, size, color)
    }

    fn text_with_font(
        &mut self,
        face: &FontFace,
        value: &str,
        x: i32,
        y: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        let physical_size = size;
        let ascent = face.ascent(physical_size);
        let baseline = y as f32 + ascent;
        let start = x as f32;
        let mut cursor = start;
        let shaped = face
            .shape(value, physical_size)
            .unwrap_or_else(|| face.fallback_shape(value, physical_size));
        for glyph in shaped {
            if let Some(bitmap) = face.rasterize(glyph.glyph_id, physical_size) {
                let gx = (cursor + glyph.x_offset).round() as i32 + bitmap.left;
                let gy = (baseline - glyph.y_offset).round() as i32 + bitmap.top;
                for row in 0..bitmap.height {
                    for col in 0..bitmap.width {
                        let coverage = bitmap.coverage[row * bitmap.width + col];
                        if coverage == 0 {
                            continue;
                        }
                        let px = gx + col as i32;
                        let py = gy + row as i32;
                        if px < 0 || py < 0 || (px as usize) >= self.w || (py as usize) >= self.h {
                            continue;
                        }
                        let idx = py as usize * self.w + px as usize;
                        let src = with_alpha(color, coverage);
                        self.pixels[idx] = blend(self.pixels[idx], src);
                    }
                }
            }
            cursor += glyph.x_advance;
        }
        (cursor - start).round() as i32
    }

    /// Word-wrap `text` to `max_width` pixels.
    pub fn wrap(
        &self,
        fonts: &Fonts,
        text: &str,
        max_width: i32,
        size: f32,
        bold: bool,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        for hard_line in text.split('\n') {
            if uses_character_line_breaks(hard_line) {
                let mut current = String::new();
                for grapheme in hard_line.graphemes(true) {
                    let mut candidate = current.clone();
                    candidate.push_str(grapheme);
                    if !current.is_empty() && fonts.text_width(&candidate, size, bold) > max_width {
                        lines.push(std::mem::take(&mut current));
                    }
                    current.push_str(grapheme);
                }
                lines.push(current);
            } else {
                let mut current = String::new();
                for word in hard_line.split_whitespace() {
                    let candidate = if current.is_empty() {
                        word.to_string()
                    } else {
                        format!("{current} {word}")
                    };
                    if !current.is_empty() && fonts.text_width(&candidate, size, bold) > max_width {
                        lines.push(std::mem::take(&mut current));
                    }
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.push_str(word);
                }
                lines.push(current);
            }
        }
        lines
    }

    /// Wrapped paragraph; returns bottom y.
    pub fn paragraph(
        &mut self,
        fonts: &Fonts,
        text: &str,
        x: i32,
        y: i32,
        max_width: i32,
        line_height: i32,
        size: f32,
        color: u32,
        bold: bool,
    ) -> i32 {
        let mut cy = y;
        for line in self.wrap(fonts, text, max_width, size, bold) {
            self.text(fonts, &line, x, cy, size, color, bold);
            cy += line_height;
        }
        cy
    }

    /// VCSS-driven button with optional raster nine-slice chrome.
    #[allow(clippy::too_many_arguments)]
    pub fn styled_button(
        &mut self,
        fonts: &Fonts,
        image: Option<&crate::ArtBitmap>,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        status: Option<&str>,
        computed: &ComputedStyle,
    ) {
        let ui = apply_computed_style(&UiStyle::default(), computed);
        let scale = computed.number("scale", 1.0).clamp(0.85, 1.08);
        let inset_x = ((w as f32 * (1.0 - scale)) * 0.5).round() as i32;
        let inset_y = ((h as f32 * (1.0 - scale)) * 0.5).round() as i32;
        let x = x + inset_x;
        let y = y + inset_y;
        let w = (w - inset_x * 2).max(8);
        let h = (h - inset_y * 2).max(8);
        let opacity = ui.opacity.clamp(0.0, 1.0);
        let source_margins = ui
            .nine_slice
            .map(|slice| slice.source)
            .unwrap_or_else(|| NinePatchMargins::uniform(0.0));
        let destination_margins = ui
            .nine_slice
            .map(|slice| slice.destination)
            .unwrap_or(source_margins);

        if let Some(image) = image {
            let shadow = computed.color("shadow-color", StyleColor::rgb(0, 0, 0));
            let shadow_opacity = computed.number("shadow-opacity", 0.55).clamp(0.0, 1.0) * opacity;
            let shadow_x = computed.number("shadow-offset-x", 3.0).round() as i32;
            let shadow_y = computed.number("shadow-offset-y", 4.0).round() as i32;
            self.blit_nine_slice(
                image,
                Rect::from_pos_size(
                    Vec2::new((x + shadow_x) as f32, (y + shadow_y) as f32),
                    Vec2::new(w as f32, h as f32),
                ),
                source_margins,
                destination_margins,
                shadow_opacity,
                Some(shadow.rgb_tuple()),
            );

            let glow_strength = computed.number("glow-strength", 0.0).clamp(0.0, 1.0);
            if glow_strength > 0.01 {
                let glow = computed.color("glow", StyleColor::rgb(255, 80, 170));
                for (dx, dy) in [(-3, 0), (3, 0), (0, -2), (0, 2)] {
                    self.blit_nine_slice(
                        image,
                        Rect::from_pos_size(
                            Vec2::new((x + dx) as f32, (y + dy) as f32),
                            Vec2::new(w as f32, h as f32),
                        ),
                        source_margins,
                        destination_margins,
                        glow_strength * 0.24,
                        Some(glow.rgb_tuple()),
                    );
                }
            }

            self.blit_nine_slice(
                image,
                Rect::from_pos_size(Vec2::new(x as f32, y as f32), Vec2::new(w as f32, h as f32)),
                source_margins,
                destination_margins,
                opacity,
                None,
            );
        } else {
            self.rect(x, y, w, h, math_color_argb(ui.background, opacity));
            let border = math_color_argb(ui.border, opacity);
            let thickness = ui.border_width.max(1.0).round() as i32;
            self.rect(x, y, w, thickness, border);
            self.rect(x, y + h - thickness, w, thickness, border);
            self.rect(x, y, thickness, h, border);
            self.rect(x + w - thickness, y, thickness, h, border);
        }

        let mut size = ui.font_size.max(10.0);
        let mut label_width = fonts.display_width(label, size);
        let available_width = (w - 72).max(80);
        if label_width > available_width {
            size = (size * available_width as f32 / label_width as f32).max(12.0);
            label_width = fonts.display_width(label, size);
        }
        let color = math_color_argb(ui.text_color, opacity);
        let label_y = y + (h - size.round() as i32) / 2;
        self.display_text(
            fonts,
            label,
            x + (w - label_width) / 2 + 1,
            label_y + 1,
            size,
            0xaa100713,
        );
        self.display_text(
            fonts,
            label,
            x + (w - label_width) / 2,
            label_y,
            size,
            color,
        );
        if let Some(status) = status {
            let status_size = 12.0;
            let status_width: i32 = status
                .chars()
                .map(|c| fonts.advance(c, status_size, false).round() as i32)
                .sum();
            self.text(
                fonts,
                status,
                x + w - status_width - 18,
                y + h / 2 - 7,
                status_size,
                color,
                false,
            );
        }
    }

    /// Resolution-independent narrative choice with explicit focus and hierarchy.
    #[allow(clippy::too_many_arguments)]
    pub fn styled_choice_button(
        &mut self,
        fonts: &Fonts,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        ordinal: usize,
        selected: bool,
        feedback_progress: f32,
        pressed: bool,
        treatment: ChoiceTreatment,
        computed: &ComputedStyle,
    ) {
        let ui = apply_computed_style(&UiStyle::default(), computed);
        let opacity = ui.opacity.clamp(0.0, 1.0);
        let glow_strength = computed.number("glow-strength", 0.0).clamp(0.0, 1.0);
        let glow = computed.color("glow", StyleColor::rgb(255, 118, 173));
        self.styled_panel(x, y, w, h, computed);

        let feedback_progress = feedback_progress.clamp(0.0, 1.0);
        let eased_feedback = 1.0 - (1.0 - feedback_progress).powi(3);
        if eased_feedback > 0.005 {
            let feedback = computed.color("feedback-color", glow);
            let cut = computed.number("corner-cut", 0.0).round().max(0.0) as i32;
            self.chamfered_fill(
                x + 3,
                y + 3,
                w - 6,
                h - 6,
                (cut - 3).max(0),
                style_color_argb(
                    feedback,
                    opacity * eased_feedback * if pressed { 0.18 } else { 0.08 },
                ),
            );
            let sweep_margin = computed.number("feedback-inset", 70.0).round() as i32;
            let sweep_width =
                ((w - sweep_margin * 2).max(1) as f32 * eased_feedback).round() as i32;
            self.rect(
                x + sweep_margin,
                y + h - 5,
                sweep_width,
                if pressed { 3 } else { 2 },
                style_color_argb(feedback, opacity * (0.34 + glow_strength * 0.34)),
            );
        }

        if selected {
            let cut = computed.number("corner-cut", 0.0).round().max(0.0) as i32;
            self.chamfered_outline(
                x - 2,
                y - 2,
                w + 4,
                h + 4,
                cut + 2,
                1,
                style_color_argb(glow, opacity * (0.20 + glow_strength * 0.24)),
            );
        }

        let marker_size = computed
            .number("marker-size", 38.0)
            .round()
            .clamp(24.0, (h - 14).max(24) as f32) as i32;
        let marker_radius = marker_size / 2;
        let marker_center_x = x + computed
            .number("marker-offset-x", 52.0)
            .round()
            .max((marker_radius + 8) as f32) as i32;
        let marker_center_y = y + h / 2;
        let marker_background = computed.color("marker-background", StyleColor::rgb(38, 11, 28));
        let marker_border = computed.color("marker-border-color", glow);
        let marker_text = computed.color("marker-color", StyleColor::rgb(255, 184, 211));
        let marker_border = style_color_argb(marker_border, opacity);
        self.diamond(
            marker_center_x,
            marker_center_y,
            marker_radius + 4,
            style_color_argb(glow, opacity * (0.08 + eased_feedback * 0.20)),
        );
        self.diamond(
            marker_center_x,
            marker_center_y,
            marker_radius + 1,
            marker_border,
        );
        self.diamond(
            marker_center_x,
            marker_center_y,
            (marker_radius - 2).max(1),
            style_color_argb(marker_background, opacity),
        );
        self.diamond(
            marker_center_x,
            marker_center_y,
            (marker_radius / 2).max(2),
            style_color_argb(marker_text, opacity * (0.12 + eased_feedback * 0.18)),
        );
        let ordinal = ordinal.to_string();
        let ordinal_size = computed.number("marker-font-size", 14.0);
        let ordinal_width = fonts.text_width(&ordinal, ordinal_size, true);
        self.text(
            fonts,
            &ordinal,
            marker_center_x - ordinal_width / 2,
            marker_center_y - ordinal_size.round() as i32 / 2 - 1,
            ordinal_size,
            style_color_argb(marker_text, opacity),
            true,
        );

        let text_color = math_color_argb(ui.text_color, opacity);
        let secondary_color = computed.color("secondary-color", StyleColor::rgb(201, 155, 176));
        let secondary_color = style_color_argb(secondary_color, opacity);
        let label_x = marker_center_x + marker_radius + 24;
        let arrow_space = 36;
        let fit_text = |value: &str, target: f32, minimum: f32, max_width: i32, bold: bool| {
            let mut size = target;
            let mut width = fonts.text_width(value, size, bold);
            if width > max_width && width > 0 {
                size = (size * max_width as f32 / width as f32).max(minimum);
                width = fonts.text_width(value, size, bold);
            }
            (size, width)
        };

        match treatment {
            ChoiceTreatment::HubVisit => {
                let (title, secondary) = label.split_once(" — ").unwrap_or((label, ""));
                let (title_size, _) = fit_text(title, ui.font_size.max(16.0), 15.0, w - 104, true);
                let title_y = if secondary.is_empty() {
                    y + (h - title_size.round() as i32) / 2 - 1
                } else {
                    y + 9
                };
                self.text(
                    fonts,
                    title,
                    label_x + 1,
                    title_y + 1,
                    title_size,
                    0x99100713,
                    true,
                );
                self.text(fonts, title, label_x, title_y, title_size, text_color, true);
                if !secondary.is_empty() {
                    let (secondary_size, _) = fit_text(secondary, 12.5, 11.5, w - 110, false);
                    self.text(
                        fonts,
                        secondary,
                        label_x,
                        y + h - 23,
                        secondary_size,
                        secondary_color,
                        false,
                    );
                }
            }
            ChoiceTreatment::Advance => {
                let action = "CONTINUAR HISTORIA";
                let action_size = 10.5;
                let action_width = fonts.text_width(action, action_size, true);
                let action_x = x + w - arrow_space - action_width;
                let (size, _) = fit_text(
                    label,
                    ui.font_size.max(16.0),
                    14.5,
                    action_x - label_x - 22,
                    true,
                );
                let label_y = y + (h - size.round() as i32) / 2 - 1;
                self.text(
                    fonts,
                    label,
                    label_x + 1,
                    label_y + 1,
                    size,
                    0x99100713,
                    true,
                );
                self.text(fonts, label, label_x, label_y, size, text_color, true);
                self.text(
                    fonts,
                    action,
                    action_x,
                    y + (h - action_size.round() as i32) / 2,
                    action_size,
                    secondary_color,
                    true,
                );
            }
            ChoiceTreatment::Narrative => {
                let (size, _) = fit_text(
                    label,
                    ui.font_size.max(16.0),
                    14.5,
                    w - (label_x - x) - arrow_space,
                    false,
                );
                let label_y = y + (h - size.round() as i32) / 2 - 1;
                self.text(
                    fonts,
                    label,
                    label_x + 1,
                    label_y + 1,
                    size,
                    0x99100713,
                    false,
                );
                self.text(fonts, label, label_x, label_y, size, text_color, false);
            }
        }

        let chevron = if selected { "»" } else { "›" };
        let chevron_size = if selected { 21.0 } else { 19.0 };
        let chevron_width = fonts.display_width(chevron, chevron_size);
        self.display_text(
            fonts,
            chevron,
            x + w - chevron_width - 15,
            y + (h - chevron_size.round() as i32) / 2 - 2,
            chevron_size,
            style_color_argb(marker_text, opacity),
        );
    }

    /// Filled disc with a one-pixel anti-aliased edge (`color`'s alpha kept).
    pub fn smooth_disc(&mut self, cx: f32, cy: f32, r: f32, color: u32) {
        let alpha = ((color >> 24) & 0xff) as f32;
        for py in (cy - r - 1.0).floor() as i32..=(cy + r + 1.0).ceil() as i32 {
            for px in (cx - r - 1.0).floor() as i32..=(cx + r + 1.0).ceil() as i32 {
                let d = ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
                let coverage = (r + 0.5 - d).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    self.rect(
                        px,
                        py,
                        1,
                        1,
                        with_alpha(color, (alpha * coverage).round() as u8),
                    );
                }
            }
        }
    }

    /// Anti-aliased ring of `width` centred on radius `r`.
    pub fn smooth_ring(&mut self, cx: f32, cy: f32, r: f32, width: f32, color: u32) {
        let alpha = ((color >> 24) & 0xff) as f32;
        let outer = r + width * 0.5 + 1.0;
        for py in (cy - outer).floor() as i32..=(cy + outer).ceil() as i32 {
            for px in (cx - outer).floor() as i32..=(cx + outer).ceil() as i32 {
                let d = ((px as f32 + 0.5 - cx).powi(2) + (py as f32 + 0.5 - cy).powi(2)).sqrt();
                let coverage = (width * 0.5 + 0.5 - (d - r).abs()).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    self.rect(
                        px,
                        py,
                        1,
                        1,
                        with_alpha(color, (alpha * coverage).round() as u8),
                    );
                }
            }
        }
    }

    /// Darkening ramp: `rgb` at `top_alpha` on the first row to `bottom_alpha`
    /// on the last (alphas 0..1).
    pub fn vertical_scrim(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        rgb: u32,
        top_alpha: f32,
        bottom_alpha: f32,
    ) {
        for row in 0..h.max(0) {
            let t = row as f32 / (h - 1).max(1) as f32;
            let alpha = top_alpha + (bottom_alpha - top_alpha) * t;
            self.rect(
                x,
                y + row,
                w,
                1,
                with_alpha(rgb, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8),
            );
        }
    }

    /// One pixel, alpha-blended (clipped to the canvas).
    fn put(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let index = y as usize * self.w + x as usize;
            self.pixels[index] = blend(self.pixels[index], color);
        }
    }

    /// Rounded rectangle with anti-aliased corners and a vertical gradient
    /// from `top` to `bottom` (ARGB, alpha kept).
    #[allow(clippy::too_many_arguments)]
    pub fn round_rect(&mut self, x: i32, y: i32, w: i32, h: i32, r: i32, top: u32, bottom: u32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.clamp(0, w.min(h) / 2);
        let (fx, fy, fw, fh, fr) = (x as f32, y as f32, w as f32, h as f32, r as f32);
        let denominator = (h - 1).max(1) as f32;
        for row in 0..h {
            let color = lerp_argb(top, bottom, row as f32 / denominator);
            if row >= r && row < h - r {
                self.rect(x, y + row, w, 1, color);
                continue;
            }
            self.rect(x + r, y + row, (w - 2 * r).max(0), 1, color);
            for column in (0..r).chain(w - r..w) {
                let d = rounded_distance(
                    x as f32 + column as f32 + 0.5,
                    y as f32 + row as f32 + 0.5,
                    fx,
                    fy,
                    fw,
                    fh,
                    fr,
                );
                let coverage = (0.5 - d).clamp(0.0, 1.0);
                if coverage > 0.0 {
                    self.put(x + column, y + row, scale_alpha(color, coverage));
                }
            }
        }
    }

    /// Anti-aliased rounded outline of `thickness`, inside the rectangle.
    #[allow(clippy::too_many_arguments)]
    pub fn round_outline(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        r: i32,
        thickness: f32,
        color: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.clamp(0, w.min(h) / 2);
        let (fx, fy, fw, fh, fr) = (x as f32, y as f32, w as f32, h as f32, r as f32);
        let band = thickness.ceil() as i32 + 1;
        let half = thickness * 0.5;
        let paint = |canvas: &mut Canvas, column: i32, row: i32| {
            let d = rounded_distance(
                x as f32 + column as f32 + 0.5,
                y as f32 + row as f32 + 0.5,
                fx,
                fy,
                fw,
                fh,
                fr,
            );
            let coverage = (half + 0.5 - (d + half).abs()).clamp(0.0, 1.0);
            if coverage > 0.0 {
                canvas.put(x + column, y + row, scale_alpha(color, coverage));
            }
        };
        for row in 0..h {
            let at_edge = row < band || row >= h - band;
            let corner = row < r + 1 || row >= h - r - 1;
            if at_edge {
                for column in 0..w {
                    paint(self, column, row);
                }
            } else if corner {
                for column in (0..r + 1).chain(w - r - 1..w) {
                    paint(self, column, row);
                }
            } else {
                for column in (0..band).chain(w - band..w) {
                    paint(self, column, row);
                }
            }
        }
    }

    /// A vector icon (parts in a unit box, y down; `true` = ink, `false` = cut
    /// back to `face`) centred on (cx, cy), `size` pixels across. Coverage is
    /// supersampled once per (id, size) and cached, so a menu can draw many
    /// icons per frame.
    #[allow(clippy::too_many_arguments)]
    pub fn vector_icon(
        &mut self,
        id: usize,
        parts: &[(Vec<[f64; 2]>, bool)],
        cx: i32,
        cy: i32,
        size: i32,
        ink: u32,
        face: u32,
    ) {
        let coverage = ICON_COVERAGE.with(|cache| {
            cache
                .borrow_mut()
                .entry((id, size))
                .or_insert_with(|| Rc::new(rasterize_icon(parts, size)))
                .clone()
        });
        for &(dx, dy, face_cov, ink_cov) in coverage.iter() {
            if face_cov > 0 {
                self.put(
                    cx + dx as i32,
                    cy + dy as i32,
                    scale_alpha(face, face_cov as f32 / 255.0),
                );
            }
            if ink_cov > 0 {
                self.put(
                    cx + dx as i32,
                    cy + dy as i32,
                    scale_alpha(ink, ink_cov as f32 / 255.0),
                );
            }
        }
    }

    /// One narrative decision: a clean glass card with a numbered badge, the
    /// option in large type and, once chosen or hovered, a lit accent edge.
    /// `hover` runs 0..1; `accent` is 0x00RRGGBB.
    #[allow(clippy::too_many_arguments)]
    pub fn story_choice_card(
        &mut self,
        fonts: &Fonts,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        label: &str,
        ordinal: usize,
        selected: bool,
        hover: f32,
        pressed: bool,
        enabled: bool,
        accent: u32,
    ) {
        let lift = if selected { 1.0 } else { hover.clamp(0.0, 1.0) };
        let dim = if enabled { 1.0 } else { 0.45 };
        let a = |alpha: f32| with_alpha(accent, ((alpha * dim).clamp(0.0, 1.0) * 255.0) as u8);
        let radius = 16;
        // A soft drop shadow, then the glass.
        self.round_rect(x - 2, y + 7, w + 4, h, radius + 3, 0x1c000000, 0x2c000000);
        // The glass takes a little of the scene's light (`accent`).
        let tint = |argb: u32, amount: f32| {
            lerp_argb(argb, (argb & 0xff00_0000) | (accent & 0x00ff_ffff), amount)
        };
        self.round_rect(
            x,
            y,
            w,
            h,
            radius,
            lerp_argb(tint(0xe6170b20, 0.15), tint(0xf0301638, 0.30), lift),
            lerp_argb(tint(0xea0e0715, 0.08), tint(0xf0180c22, 0.16), lift),
        );
        if pressed {
            self.round_rect(x, y, w, h, radius, a(0.16), a(0.16));
        }
        self.round_outline(
            x,
            y,
            w,
            h,
            radius,
            1.0 + lift * 0.8,
            lerp_argb(0x24ffffff, a(0.95), lift),
        );
        // Badge: a ring in the accent that fills as the card warms.
        let (bx, by) = ((x + 47) as f32, (y + h / 2) as f32);
        self.smooth_disc(bx, by, 22.0, a(0.10 + 0.30 * lift));
        self.smooth_ring(bx, by, 21.2, 1.6, a(0.55 + 0.45 * lift));
        let number = ordinal.to_string();
        let number_size = 22.0;
        let number_width = fonts.text_width(&number, number_size, true);
        self.text(
            fonts,
            &number,
            bx as i32 - number_width / 2,
            by as i32 - number_size.round() as i32 / 2 - 2,
            number_size,
            lerp_argb(a(0.95), 0xffffffff, lift * 0.7),
            true,
        );
        // The option: one line at full size, two when it is long.
        let text_x = x + 47 + 22 + 24;
        let available = w - (text_x - x) - 64;
        let color = if enabled {
            lerp_argb(0xfff1e6ee, 0xffffffff, lift)
        } else {
            0xff9d8994
        };
        let full = 27.0;
        if fonts.text_width(label, full, false) <= available {
            let ty = y + (h - full.round() as i32) / 2 - 3;
            self.text(fonts, label, text_x + 1, ty + 2, full, 0x88100713, false);
            self.text(fonts, label, text_x, ty, full, color, false);
        } else {
            let size = 23.0;
            let lines = self.wrap(fonts, label, available, size, false);
            if lines.len() <= 2 {
                let line_height = 29;
                let top = y + (h - line_height * lines.len() as i32) / 2 - 2;
                for (i, line) in lines.iter().enumerate() {
                    let ty = top + i as i32 * line_height;
                    self.text(fonts, line, text_x + 1, ty + 2, size, 0x88100713, false);
                    self.text(fonts, line, text_x, ty, size, color, false);
                }
            } else {
                let mut size = size;
                while size > 15.0 && fonts.text_width(label, size, false) > available {
                    size -= 0.5;
                }
                let ty = y + (h - size.round() as i32) / 2 - 3;
                self.text(fonts, label, text_x + 1, ty + 2, size, 0x88100713, false);
                self.text(fonts, label, text_x, ty, size, color, false);
            }
        }
        // A chevron that appears and slides forward as the card is chosen.
        if lift > 0.04 {
            let cx = x + w - 36 + (lift * 5.0).round() as i32;
            let cy = y + h / 2;
            self.polygon(
                &[
                    (cx - 5, cy - 10),
                    (cx - 2, cy - 10),
                    (cx + 5, cy),
                    (cx - 2, cy + 10),
                    (cx - 5, cy + 10),
                    (cx + 2, cy),
                ],
                a(0.9 * lift),
            );
        }
    }

    fn chamfered_fill(&mut self, x: i32, y: i32, w: i32, h: i32, cut: i32, color: u32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let cut = cut.clamp(0, w.min(h) / 2);
        for row in 0..h {
            let inset = if row < cut {
                cut - row
            } else if row >= h - cut {
                row - (h - cut - 1)
            } else {
                0
            };
            self.rect(x + inset, y + row, (w - inset * 2).max(0), 1, color);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn chamfered_gradient_fill(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        cut: i32,
        top: u32,
        bottom: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let cut = cut.clamp(0, w.min(h) / 2);
        let denominator = (h - 1).max(1) as f32;
        for row in 0..h {
            let inset = if row < cut {
                cut - row
            } else if row >= h - cut {
                row - (h - cut - 1)
            } else {
                0
            };
            self.rect(
                x + inset,
                y + row,
                (w - inset * 2).max(0),
                1,
                lerp_argb(top, bottom, row as f32 / denominator),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn chamfered_outline(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        cut: i32,
        thickness: i32,
        color: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let thickness = thickness.clamp(1, w.min(h).max(1) / 2);
        for layer in 0..thickness {
            let x = x + layer;
            let y = y + layer;
            let w = w - layer * 2;
            let h = h - layer * 2;
            if w <= 0 || h <= 0 {
                break;
            }
            let cut = (cut - layer).clamp(0, w.min(h) / 2);
            let horizontal = (w - cut * 2).max(0);
            self.rect(x + cut, y, horizontal, 1, color);
            self.rect(x + cut, y + h - 1, horizontal, 1, color);
            self.rect(x, y + cut, 1, (h - cut * 2).max(0), color);
            self.rect(x + w - 1, y + cut, 1, (h - cut * 2).max(0), color);
            for offset in 0..=cut {
                self.rect(x + cut - offset, y + offset, 1, 1, color);
                self.rect(x + w - cut + offset - 1, y + offset, 1, 1, color);
                self.rect(x + cut - offset, y + h - offset - 1, 1, 1, color);
                self.rect(x + w - cut + offset - 1, y + h - offset - 1, 1, 1, color);
            }
        }
    }

    fn arcane_jewel(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        edge: u32,
        fill: u32,
        core: u32,
        glow: u32,
    ) {
        let radius = radius.max(3);
        self.diamond(cx, cy, radius + 5, scale_alpha(glow, 0.18));
        self.diamond(cx, cy, radius + 2, scale_alpha(edge, 0.35));
        self.diamond(cx, cy, radius, edge);
        self.diamond(cx, cy, (radius - 2).max(1), fill);
        self.diamond(cx, cy, (radius / 2).max(1), core);
        self.diamond(cx - radius / 4, cy - radius / 3, 1, 0xd9ffffff);
    }

    #[allow(clippy::too_many_arguments)]
    fn arcane_chrome(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        cut: i32,
        opacity: f32,
        glow_strength: f32,
        computed: &ComputedStyle,
    ) {
        if computed.keyword("chrome", "plain") != "arcane" || w < 24 || h < 18 {
            return;
        }

        let ornament_opacity = computed.number("ornament-opacity", 0.72).clamp(0.0, 1.0) * opacity;
        let primary = computed.color("ornament-color", StyleColor::rgb(231, 110, 161));
        let secondary = computed.color("ornament-secondary-color", StyleColor::rgb(239, 184, 112));
        let gem = computed.color("gem-color", StyleColor::rgb(192, 70, 213));
        let gem_core = computed.color("gem-core-color", StyleColor::rgb(255, 220, 244));
        let glow = computed.color("glow", primary);
        let primary = style_color_argb(primary, ornament_opacity);
        let secondary = style_color_argb(secondary, ornament_opacity * 0.88);
        let gem = style_color_argb(gem, opacity);
        let gem_core = style_color_argb(gem_core, opacity);
        let glow = style_color_argb(glow, opacity * (0.42 + glow_strength * 0.58));

        let rail_inset = computed
            .number("rail-inset", (cut + 20) as f32)
            .round()
            .max((cut + 4) as f32) as i32;
        let rail_width = (w - rail_inset * 2).max(0);
        let rail_gap = computed.number("rail-center-gap", 0.0).round().max(0.0) as i32;
        let rail_offset = computed.number("rail-offset", 7.0).round().max(2.0) as i32;
        if rail_width > 12 {
            if rail_gap > 0 {
                self.ornament_rule_with_gap(
                    x + w / 2,
                    y + rail_offset,
                    rail_width,
                    rail_gap.min(rail_width - 8),
                    scale_alpha(primary, 0.74),
                );
            } else {
                self.rect(
                    x + rail_inset,
                    y + rail_offset,
                    rail_width,
                    1,
                    scale_alpha(primary, 0.66),
                );
            }
            self.rect(
                x + rail_inset + 18,
                y + h - rail_offset - 1,
                (rail_width - 36).max(0),
                1,
                scale_alpha(secondary, 0.58),
            );
        }

        let corner_span = computed
            .number("corner-ornament-size", 38.0)
            .round()
            .max(8.0) as i32;
        for side in [x + cut + 5, x + w - cut - 5] {
            self.diamond(side, y + rail_offset, 3, primary);
            self.diamond(side, y + h - rail_offset - 1, 3, secondary);
        }
        self.rect(
            x + cut + 7,
            y + rail_offset + 4,
            corner_span,
            1,
            scale_alpha(secondary, 0.48),
        );
        self.rect(
            x + w - cut - corner_span - 7,
            y + rail_offset + 4,
            corner_span,
            1,
            scale_alpha(secondary, 0.48),
        );

        let jewel_radius = computed.number("gem-size", 8.0).round().clamp(3.0, 18.0) as i32;
        if computed.number("ornament-center", 0.0) >= 0.5 {
            self.arcane_jewel(
                x + w / 2,
                y + h - 1,
                jewel_radius,
                secondary,
                gem,
                gem_core,
                glow,
            );
        }
        if computed.number("ornament-sides", 0.0) >= 0.5 {
            let side_radius = (jewel_radius - 2).max(3);
            self.arcane_jewel(
                x + 1,
                y + h / 2,
                side_radius,
                secondary,
                gem,
                gem_core,
                glow,
            );
            self.arcane_jewel(
                x + w - 2,
                y + h / 2,
                side_radius,
                secondary,
                gem,
                gem_core,
                glow,
            );
        }
    }

    /// VCSS-driven panel for dialogue chrome and speaker labels.
    pub fn styled_panel(&mut self, x: i32, y: i32, w: i32, h: i32, computed: &ComputedStyle) {
        let ui = apply_computed_style(&UiStyle::default(), computed);
        let opacity = ui.opacity.clamp(0.0, 1.0);
        let cut = computed.number("corner-cut", 0.0).round().max(0.0) as i32;
        let shadow = computed.color("shadow-color", StyleColor::rgb(0, 0, 0));
        let shadow_opacity = computed.number("shadow-opacity", 0.0).clamp(0.0, 1.0);
        let shadow_x = computed.number("shadow-offset-x", 0.0).round() as i32;
        let shadow_y = computed.number("shadow-offset-y", 0.0).round() as i32;
        if shadow_opacity > 0.0 {
            self.chamfered_fill(
                x + shadow_x,
                y + shadow_y,
                w,
                h,
                cut,
                style_color_argb(shadow, shadow_opacity),
            );
        }

        let glow_strength = computed.number("glow-strength", 0.0).clamp(0.0, 1.0);
        let glow = computed.color("glow", StyleColor::rgb(255, 118, 173));
        let glow_spread = computed.number("glow-spread", 2.0).round().clamp(0.0, 5.0) as i32;
        if glow_strength > 0.0 {
            for ring in (1..=glow_spread).rev() {
                let falloff = 1.0 - (ring - 1) as f32 / glow_spread.max(1) as f32;
                self.chamfered_outline(
                    x - ring,
                    y - ring,
                    w + ring * 2,
                    h + ring * 2,
                    cut + ring,
                    1,
                    style_color_argb(glow, glow_strength * 0.12 * falloff * opacity),
                );
            }
        }

        let background_top = computed.color("background", StyleColor::rgba(16, 7, 22, 0.88));
        let background_bottom = computed.color("background-bottom", background_top);
        self.chamfered_gradient_fill(
            x,
            y,
            w,
            h,
            cut,
            style_color_argb(background_top, opacity),
            style_color_argb(background_bottom, opacity),
        );

        let border = math_color_argb(ui.border, opacity);
        let thickness = ui.border_width.max(1.0).round() as i32;
        self.chamfered_outline(x, y, w, h, cut, thickness, border);

        let inner_border_width = computed
            .number("inner-border-width", 0.0)
            .round()
            .clamp(0.0, 4.0) as i32;
        let inner_inset = computed.number("inner-inset", 7.0).round().max(2.0) as i32;
        if inner_border_width > 0 && w > inner_inset * 2 && h > inner_inset * 2 {
            let inner = computed.color("inner-border-color", glow);
            self.chamfered_outline(
                x + inner_inset,
                y + inner_inset,
                w - inner_inset * 2,
                h - inner_inset * 2,
                (cut - inner_inset).max(0),
                inner_border_width,
                style_color_argb(inner, opacity * 0.62),
            );
        }

        if computed.keyword("chrome", "plain") == "arcane" {
            self.arcane_chrome(x, y, w, h, cut, opacity, glow_strength, computed);
        } else if glow_strength > 0.0 {
            let soft_glow = style_color_argb(glow, glow_strength * 0.12);
            let line_glow = style_color_argb(glow, glow_strength * 0.52);
            let inset = cut + 12;

            // Gentle top wash and inset rails add depth without competing with text.
            self.chamfered_fill(
                x + thickness,
                y + thickness,
                w - thickness * 2,
                27.min(h - thickness * 2),
                (cut - thickness).max(0),
                soft_glow,
            );
            self.rect(x + inset, y + thickness + 2, w - inset * 2, 1, line_glow);
            self.rect(
                x + inset + 18,
                y + h - thickness - 3,
                w - (inset + 18) * 2,
                1,
                style_color_argb(glow, glow_strength * 0.30),
            );
            if cut > 0 {
                self.diamond(x + cut + 3, y + 2, 3, line_glow);
                self.diamond(x + w - cut - 4, y + 2, 3, line_glow);
            }
        }
    }

    fn blit_nine_slice(
        &mut self,
        image: &crate::ArtBitmap,
        destination: Rect,
        source_margins: NinePatchMargins,
        destination_margins: NinePatchMargins,
        opacity: f32,
        tint: Option<(u8, u8, u8)>,
    ) {
        if image.w == 0 || image.h == 0 || destination.width() <= 0.0 || destination.height() <= 0.0
        {
            return;
        }
        let quads = layout_nine_patch_with_destination_margins(
            Vec2::new(image.w as f32, image.h as f32),
            source_margins,
            destination,
            destination_margins,
        );
        for quad in quads {
            self.blit_rgba_region_scaled(image, quad.src, quad.dst, opacity, tint);
        }
    }

    fn blit_rgba_region_scaled(
        &mut self,
        image: &crate::ArtBitmap,
        source: Rect,
        destination: Rect,
        opacity: f32,
        tint: Option<(u8, u8, u8)>,
    ) {
        let dx = destination.min.x.round() as i32;
        let dy = destination.min.y.round() as i32;
        let dw = destination.width().round() as i32;
        let dh = destination.height().round() as i32;
        if source.width() <= 0.0 || source.height() <= 0.0 || dw <= 0 || dh <= 0 {
            return;
        }
        let source_per_pixel_x = source.width() / dw as f32;
        let source_per_pixel_y = source.height() / dh as f32;
        let taps_x = source_per_pixel_x.ceil().clamp(1.0, 4.0) as usize;
        let taps_y = source_per_pixel_y.ceil().clamp(1.0, 4.0) as usize;
        for row in 0..dh {
            let py = dy + row;
            if py < 0 || py as usize >= self.h {
                continue;
            }
            for column in 0..dw {
                let px = dx + column;
                if px < 0 || px as usize >= self.w {
                    continue;
                }
                let mut accumulated_alpha = 0.0;
                let mut accumulated_red = 0.0;
                let mut accumulated_green = 0.0;
                let mut accumulated_blue = 0.0;
                for tap_y in 0..taps_y {
                    let sample_y = source.min.y
                        + (row as f32 + (tap_y as f32 + 0.5) / taps_y as f32) * source_per_pixel_y;
                    let sample_y = sample_y
                        .floor()
                        .clamp(0.0, image.h.saturating_sub(1) as f32)
                        as usize;
                    for tap_x in 0..taps_x {
                        let sample_x = source.min.x
                            + (column as f32 + (tap_x as f32 + 0.5) / taps_x as f32)
                                * source_per_pixel_x;
                        let sample_x = sample_x
                            .floor()
                            .clamp(0.0, image.w.saturating_sub(1) as f32)
                            as usize;
                        let source_index = (sample_y * image.w + sample_x) * 4;
                        let alpha = image.rgba[source_index + 3] as f32 / 255.0;
                        accumulated_alpha += alpha;
                        accumulated_red += image.rgba[source_index] as f32 * alpha;
                        accumulated_green += image.rgba[source_index + 1] as f32 * alpha;
                        accumulated_blue += image.rgba[source_index + 2] as f32 * alpha;
                    }
                }
                let sample_count = (taps_x * taps_y) as f32;
                let average_alpha = accumulated_alpha / sample_count;
                let alpha = (average_alpha * opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
                if alpha == 0 {
                    continue;
                }
                let (red, green, blue) = tint.unwrap_or_else(|| {
                    let divisor = accumulated_alpha.max(f32::EPSILON);
                    (
                        (accumulated_red / divisor).round() as u8,
                        (accumulated_green / divisor).round() as u8,
                        (accumulated_blue / divisor).round() as u8,
                    )
                });
                let source_color =
                    (alpha << 24) | ((red as u32) << 16) | ((green as u32) << 8) | blue as u32;
                let destination_index = py as usize * self.w + px as usize;
                self.pixels[destination_index] =
                    blend(self.pixels[destination_index], source_color);
            }
        }
    }

    /// Draw a cropped RGBA region 1:1 with brightness feedback.
    /// Used by native menu assets; coordinates and dimensions never change.
    #[allow(clippy::too_many_arguments)]
    pub fn blit_region_adjusted(
        &mut self,
        image: &crate::ArtBitmap,
        source_x: i32,
        source_y: i32,
        source_w: i32,
        source_h: i32,
        destination_x: i32,
        destination_y: i32,
        brightness: f32,
        opacity: f32,
    ) {
        if source_w <= 0 || source_h <= 0 {
            return;
        }
        let brightness = brightness.clamp(0.0, 1.4);
        let opacity = opacity.clamp(0.0, 1.0);
        for row in 0..source_h {
            let sy = source_y + row;
            let dy = destination_y + row;
            if sy < 0 || sy as usize >= image.h || dy < 0 || dy as usize >= self.h {
                continue;
            }
            for column in 0..source_w {
                let sx = source_x + column;
                let dx = destination_x + column;
                if sx < 0 || sx as usize >= image.w || dx < 0 || dx as usize >= self.w {
                    continue;
                }
                let source_index = (sy as usize * image.w + sx as usize) * 4;
                let alpha = (image.rgba[source_index + 3] as f32 * opacity).round() as u32;
                if alpha == 0 {
                    continue;
                }
                let adjusted =
                    |channel: u8| (channel as f32 * brightness).round().clamp(0.0, 255.0) as u32;
                let source_color = (alpha << 24)
                    | (adjusted(image.rgba[source_index]) << 16)
                    | (adjusted(image.rgba[source_index + 1]) << 8)
                    | adjusted(image.rgba[source_index + 2]);
                let destination_index = dy as usize * self.w + dx as usize;
                self.pixels[destination_index] =
                    blend(self.pixels[destination_index], source_color);
            }
        }
    }

    /// Animated feedback shared by native raster buttons.
    /// Brightness and overlays change; geometry remains exactly 1:1.
    pub fn native_button_feedback(
        &mut self,
        image: &crate::ArtBitmap,
        source: (i32, i32, i32, i32),
        destination: (i32, i32, i32, i32),
        progress: f32,
        pressed: bool,
    ) {
        if progress <= 0.005 {
            return;
        }
        let (sx, sy, sw, sh) = source;
        let (x, y, w, h) = destination;
        debug_assert_eq!((sw, sh), (w, h));
        let progress = progress.clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - progress).powi(3);
        let brightness = 1.0 + eased * if pressed { 0.30 } else { 0.20 };
        self.blit_region_adjusted(image, sx, sy, sw, sh, x, y, brightness, 1.0);

        let sheen_strength = (std::f32::consts::PI * progress).sin().max(0.0);
        if sheen_strength > 0.01 {
            let margin = (w as f32 * 0.09).round() as i32;
            let travel = (w - margin * 2).max(1);
            let center = x + margin + (travel as f32 * progress).round() as i32;
            let half_width = (w as f32 * 0.025).round().max(12.0) as i32;
            for offset in -half_width..=half_width {
                let falloff = 1.0 - offset.abs() as f32 / half_width.max(1) as f32;
                let alpha = (44.0 * sheen_strength * falloff).round() as u32;
                if alpha > 0 {
                    self.rect(
                        center + offset,
                        y + h / 5,
                        1,
                        h * 3 / 5,
                        (alpha << 24) | 0x00ffe7fa,
                    );
                }
            }
        }

        let rail_alpha = ((if pressed { 170.0 } else { 105.0 }) * eased).round() as u32;
        let rail_margin = (w as f32 * 0.13).round() as i32;
        let rail_width = (w - rail_margin * 2).max(1);
        let rail_thickness = if pressed { 3 } else { 2 };
        self.rect(
            x + rail_margin,
            y + h / 7,
            rail_width,
            rail_thickness,
            (rail_alpha << 24) | 0x00ffe2a8,
        );
        self.rect(
            x + rail_margin,
            y + h - h / 7 - rail_thickness,
            rail_width,
            rail_thickness,
            (rail_alpha << 24) | 0x00ff8ed8,
        );
    }

    /// Fine symmetrical title divider with a central ruby.
    pub fn ornament_rule(&mut self, cx: i32, y: i32, width: i32, color: u32) {
        let half = width / 2;
        self.rect(cx - half, y, half - 16, 1, color);
        self.rect(cx + 17, y, half - 16, 1, color);
        self.diamond(cx, y, 7, color);
        self.diamond(cx, y, 3, 0xffffa5cb);
        self.diamond(cx - half + 4, y, 3, color);
        self.diamond(cx + half - 4, y, 3, color);
    }

    /// Symmetrical divider that leaves breathing room for a centered label.
    pub fn ornament_rule_with_gap(&mut self, cx: i32, y: i32, width: i32, gap: i32, color: u32) {
        let half = width / 2;
        let half_gap = gap / 2;
        let segment = (half - half_gap).max(0);
        self.rect(cx - half, y, segment, 1, color);
        self.rect(cx + half_gap, y, segment, 1, color);
        for x in [cx - half, cx - half_gap, cx + half_gap, cx + half] {
            self.diamond(x, y, 3, color);
        }
    }

    /// Compact gothic crest used above headings and beside engine branding.
    pub fn gothic_crest(&mut self, cx: i32, cy: i32, color: u32) {
        self.diamond(cx, cy, 11, color);
        self.diamond(cx, cy, 5, 0xfff7a1c7);
        self.rect(cx - 28, cy, 18, 1, color);
        self.rect(cx + 11, cy, 18, 1, color);
        self.diamond(cx - 29, cy, 3, color);
        self.diamond(cx + 29, cy, 3, color);
        self.rect(cx - 1, cy - 20, 2, 10, color);
    }

    /// Multi-tone sigil for persistent command bars.
    pub fn command_bar_sigil(
        &mut self,
        cx: i32,
        cy: i32,
        edge: u32,
        gold: u32,
        gem: u32,
        core: u32,
    ) {
        self.rect(cx - 34, cy, 20, 1, scale_alpha(edge, 0.72));
        self.rect(cx + 15, cy, 20, 1, scale_alpha(edge, 0.72));
        self.diamond(cx - 35, cy, 3, gold);
        self.diamond(cx + 35, cy, 3, gold);
        self.diamond(cx, cy, 14, scale_alpha(edge, 0.20));
        self.diamond(cx, cy, 11, gold);
        self.diamond(cx, cy, 8, gem);
        self.diamond(cx, cy, 3, core);
        self.rect(cx, cy - 21, 1, 8, gold);
        self.diamond(cx, cy - 23, 2, edge);
    }

    /// Hairline divider with a centered jewel; keeps command groups scannable.
    pub fn command_bar_divider(&mut self, x: i32, cy: i32, height: i32, color: u32) {
        let half = height.max(10) / 2;
        self.rect(x, cy - half, 1, half - 4, scale_alpha(color, 0.48));
        self.rect(x, cy + 5, 1, half - 4, scale_alpha(color, 0.48));
        self.diamond(x, cy, 3, color);
        self.diamond(x, cy, 1, 0xffffe3f1);
    }

    /// Filled diamond (rotated square), used as ornamental accent.
    pub fn diamond(&mut self, cx: i32, cy: i32, r: i32, color: u32) {
        for dy in -r..=r {
            let half = r - dy.abs();
            self.rect(cx - half, cy + dy, half * 2 + 1, 1, color);
        }
    }
}

fn uses_character_line_breaks(value: &str) -> bool {
    value.chars().any(|ch| {
        matches!(
            ch as u32,
            0x3040..=0x30ff
                | 0x3400..=0x4dbf
                | 0x4e00..=0x9fff
                | 0xac00..=0xd7af
                | 0xf900..=0xfaff
        )
    })
}

/// Per-channel alpha blend (src over dst).
/// Signed distance from a point to a rounded rectangle (negative inside).
fn rounded_distance(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (px - (x + w * 0.5)).abs() - (w * 0.5 - r);
    let qy = (py - (y + h * 0.5)).abs() - (h * 0.5 - r);
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
}

thread_local! {
    static ICON_COVERAGE: RefCell<HashMap<(usize, i32), Rc<Vec<(i16, i16, u8, u8)>>>> = RefCell::new(HashMap::new());
}

/// Even-odd point-in-polygon.
fn inside_polygon(points: &[[f64; 2]], x: f64, y: f64) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let ([xi, yi], [xj, yj]) = (points[i], points[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// (dx, dy, face coverage, ink coverage) per pixel, offsets from the icon's
/// centre, 3×3 supersampled.
fn rasterize_icon(parts: &[(Vec<[f64; 2]>, bool)], size: i32) -> Vec<(i16, i16, u8, u8)> {
    const N: i32 = 3;
    let half = size / 2 + 1;
    let mut out = Vec::new();
    for dy in -half..=half {
        for dx in -half..=half {
            let (mut ink, mut face) = (0u32, 0u32);
            for sy in 0..N {
                for sx in 0..N {
                    let x = (dx as f64 + (sx as f64 + 0.5) / N as f64) / size as f64;
                    let y = (dy as f64 + (sy as f64 + 0.5) / N as f64) / size as f64;
                    let mut state = 0;
                    for (points, is_ink) in parts {
                        if inside_polygon(points, x, y) {
                            state = if *is_ink { 1 } else { 2 };
                        }
                    }
                    match state {
                        1 => ink += 1,
                        2 => face += 1,
                        _ => {}
                    }
                }
            }
            if ink + face > 0 {
                let scale = |n: u32| (n * 255 / (N * N) as u32) as u8;
                out.push((dx as i16, dy as i16, scale(face), scale(ink)));
            }
        }
    }
    out
}

fn blend(dst: u32, src: u32) -> u32 {
    let sa = (src >> 24) & 0xff;
    if sa == 255 {
        return src | 0xff00_0000;
    }
    if sa == 0 {
        return dst;
    }
    let da = 255 - sa;
    let mix = |shift: u32| {
        (((((src >> shift) & 0xff) * sa + ((dst >> shift) & 0xff) * da) / 255) & 0xff) << shift
    };
    0xff00_0000 | mix(16) | mix(8) | mix(0)
}

fn with_alpha(color: u32, coverage: u8) -> u32 {
    (color & 0x00ff_ffff) | ((coverage as u32) << 24)
}

fn scale_alpha(color: u32, factor: f32) -> u32 {
    let alpha = ((color >> 24) & 0xff) as f32;
    let alpha = (alpha * factor.clamp(0.0, 1.0)).round() as u32;
    (color & 0x00ff_ffff) | (alpha << 24)
}

/// Linear interpolation between two opaque RGB colors.
fn lerp(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |shift: u32| {
        let ca = ((a >> shift) & 0xff) as f32;
        let cb = ((b >> shift) & 0xff) as f32;
        ((ca + (cb - ca) * t).round() as u32 & 0xff) << shift
    };
    0xff00_0000 | ch(16) | ch(8) | ch(0)
}

/// Linear interpolation that preserves source alpha for translucent UI layers.
fn lerp_argb(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let start = ((a >> shift) & 0xff) as f32;
        let end = ((b >> shift) & 0xff) as f32;
        ((start + (end - start) * t).round() as u32 & 0xff) << shift
    };
    channel(24) | channel(16) | channel(8) | channel(0)
}

fn math_color_argb(color: velvet_math::Color, opacity: f32) -> u32 {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    let alpha = channel(color.a * opacity.clamp(0.0, 1.0));
    (alpha << 24) | (channel(color.r) << 16) | (channel(color.g) << 8) | channel(color.b)
}

fn style_color_argb(color: StyleColor, opacity: f32) -> u32 {
    let alpha = (color.a * opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    (alpha << 24) | (color.pack_rgb() & 0x00ff_ffff)
}
