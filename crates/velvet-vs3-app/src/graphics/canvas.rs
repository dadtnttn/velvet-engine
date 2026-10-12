use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && y >= self.y as f32
            && x < (self.x + self.w) as f32
            && y < (self.y + self.h) as f32
    }
    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        (x0 < x1 && y0 < y1).then_some(Rect::new(x0, y0, x1 - x0, y1 - y0))
    }
}

/// Cached glyph coverage keyed by font identity, glyph code and pixel size.
type GlyphKey = (usize, u32, u32);
/// Upper bound on cached glyphs before the cache is reset.
const MAX_CACHED_GLYPHS: usize = 16_384;
/// A glyph's coverage grown by a radius (outlined text): glyph key and
/// radius bits.
type OutlineKey = (GlyphKey, u32);
/// Grown coverage: offset from the glyph's own box, size and coverage.
struct Outline {
    margin: i32,
    width: usize,
    height: usize,
    coverage: Vec<u8>,
}

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    clip: Option<Rect>,
    opacity: f32,
    glyphs: HashMap<GlyphKey, (fontdue::Metrics, Vec<u8>)>,
    outlines: HashMap<OutlineKey, Outline>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0xff000000; width * height],
            clip: None,
            opacity: 1.0,
            glyphs: HashMap::new(),
            outlines: HashMap::new(),
        }
    }
    /// Rasterizes a glyph once per font and size.
    fn cache_glyph(
        &mut self,
        font: &super::fonts::LoadedFont,
        glyph: super::layout::GlyphRef,
        size: f32,
    ) -> GlyphKey {
        let key = (font.key, glyph.code(), size.to_bits());
        if !self.glyphs.contains_key(&key) {
            if self.glyphs.len() >= MAX_CACHED_GLYPHS {
                self.glyphs.clear();
            }
            let raster = match glyph {
                super::layout::GlyphRef::Char(ch) => font.font.rasterize(ch, size),
                super::layout::GlyphRef::Index(i) => font.font.rasterize_indexed(i, size),
            };
            self.glyphs.insert(key, raster);
        }
        key
    }
    /// Draws one glyph with its pen at (`x`, `baseline`); `alpha` maps
    /// coverage to alpha (see [`alpha_table`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn glyph(
        &mut self,
        font: &super::fonts::LoadedFont,
        glyph: super::layout::GlyphRef,
        size: f32,
        x: f32,
        baseline: i32,
        color: u32,
        alpha: &[u8; 256],
    ) {
        let key = self.cache_glyph(font, glyph, size);
        let Some(visible) = self.visible() else {
            return;
        };
        let (metrics, coverage) = &self.glyphs[&key];
        let mask = Mask {
            x: x.round() as i32 + metrics.xmin,
            y: baseline - metrics.height as i32 - metrics.ymin,
            width: metrics.width,
            height: metrics.height,
            coverage,
        };
        mask.draw(&mut self.pixels, self.width, visible, color, alpha);
    }
    /// The glyph's shape grown by `radius` pixels (soft edged), drawn in
    /// `color`: an outline when the glyph is drawn on top.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn glyph_outline(
        &mut self,
        font: &super::fonts::LoadedFont,
        glyph: super::layout::GlyphRef,
        size: f32,
        x: f32,
        baseline: i32,
        color: u32,
        radius: f32,
        alpha: &[u8; 256],
    ) {
        let key = self.cache_glyph(font, glyph, size);
        let (metrics, coverage) = &self.glyphs[&key];
        let outline_key = (key, radius.to_bits());
        if !self.outlines.contains_key(&outline_key) {
            if self.outlines.len() >= MAX_CACHED_GLYPHS {
                self.outlines.clear();
            }
            let grown = grow(coverage, metrics.width, metrics.height, radius);
            self.outlines.insert(outline_key, grown);
        }
        let Some(visible) = self.visible() else {
            return;
        };
        let outline = &self.outlines[&outline_key];
        let mask = Mask {
            x: x.round() as i32 + metrics.xmin - outline.margin,
            y: baseline - metrics.height as i32 - metrics.ymin - outline.margin,
            width: outline.width,
            height: outline.height,
            coverage: &outline.coverage,
        };
        mask.draw(&mut self.pixels, self.width, visible, color, alpha);
    }
    pub(crate) fn opacity(&self) -> f32 {
        self.opacity
    }
    pub fn set_clip(&mut self, clip: Option<Rect>) {
        self.clip = clip;
    }
    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }
    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(apply_opacity(color, self.opacity));
    }
    /// `clear` limited to `area` (the clip does not apply, as in `clear`).
    pub fn clear_area(&mut self, area: Rect, color: u32) {
        if let Some(area) = area.intersect(self.bounds()) {
            let value = apply_opacity(color, self.opacity);
            self.rows(area).for_each(|row| row.fill(value));
        }
    }
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width as i32, self.height as i32)
    }
    pub fn visible(&self) -> Option<Rect> {
        self.clip.unwrap_or(self.bounds()).intersect(self.bounds())
    }
    /// The part of `rect` that can be drawn (clip and canvas applied).
    pub(crate) fn drawable(&self, rect: Rect) -> Option<Rect> {
        self.visible().and_then(|visible| visible.intersect(rect))
    }
    fn rows(&mut self, area: Rect) -> impl Iterator<Item = &mut [u32]> {
        let (x, w) = (area.x as usize, area.w as usize);
        self.pixels
            .chunks_exact_mut(self.width)
            .skip(area.y as usize)
            .take(area.h as usize)
            .map(move |row| &mut row[x..x + w])
    }
    /// `source` over `area` (already clipped), like `blend` on each pixel.
    fn blend_area(&mut self, area: Rect, source: u32) {
        let a = source >> 24;
        if a == 0 {
            return;
        }
        if a == 255 {
            return self.rows(area).for_each(|row| row.fill(source));
        }
        let (rb, g, inv) = premultiplied(source, a);
        for row in self.rows(area) {
            for pixel in row {
                *pixel = mix_premultiplied(*pixel, rb, g, inv);
            }
        }
    }
    /// `color` on every pixel of `area` (already clipped): blended when
    /// `alpha` or the canvas is translucent, else written as is.
    fn paint(&mut self, area: Rect, color: u32, alpha: bool) {
        let source = apply_opacity(color, self.opacity);
        if alpha || self.opacity < 1.0 {
            self.blend_area(area, source)
        } else {
            self.rows(area).for_each(|row| row.fill(source))
        }
    }
    /// `paint` on pixels `x0..x1` of row `y`.
    fn span(&mut self, y: i32, x0: i32, x1: i32, color: u32, alpha: bool) {
        if let Some(area) = self.drawable(Rect::new(x0, y, x1 - x0, 1)) {
            self.paint(area, color, alpha);
        }
    }
    pub fn rect(&mut self, rect: Rect, color: u32) {
        if let Some(area) = self.drawable(rect) {
            self.paint(area, color, false);
        }
    }
    pub fn alpha_rect(&mut self, rect: Rect, color: u32) {
        if let Some(area) = self.drawable(rect) {
            self.paint(area, color, true);
        }
    }
    pub fn border(&mut self, r: Rect, t: i32, c: u32) {
        let t = t.max(1);
        self.rect(Rect::new(r.x, r.y, r.w, t), c);
        self.rect(Rect::new(r.x, r.y + r.h - t, r.w, t), c);
        self.rect(Rect::new(r.x, r.y, t, r.h), c);
        self.rect(Rect::new(r.x + r.w - t, r.y, t, r.h), c);
    }
    pub fn circle(&mut self, cx: i32, cy: i32, r: i32, c: u32) {
        let r = r.max(1);
        for dy in -r..=r {
            let dx = isqrt(i64::from(r * r - dy * dy)) as i32;
            self.span(cy + dy, cx - dx, cx + dx + 1, c, false);
        }
    }
    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        self.ellipse_impl(cx, cy, rx, ry, c, false)
    }
    pub fn alpha_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        self.ellipse_impl(cx, cy, rx, ry, c, true)
    }
    /// Pixels with dx²·ry² + dy²·rx² ≤ rx²·ry², one span per row.
    fn ellipse_impl(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32, a: bool) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let rx2 = i64::from(rx) * i64::from(rx);
        let ry2 = i64::from(ry) * i64::from(ry);
        for dy in -ry..=ry {
            let room = rx2 * ry2 - i64::from(dy) * i64::from(dy) * rx2;
            let dx = isqrt(room / ry2) as i32;
            self.span(cy + dy, cx - dx, cx + dx + 1, c, a);
        }
    }
    pub fn round_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, t: i32, c: u32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).max(1);
        let r = (t / 2).max(1);
        for s in 0..=steps {
            let q = s as f32 / steps as f32;
            self.circle(
                (x0 as f32 + dx as f32 * q).round() as i32,
                (y0 as f32 + dy as f32 * q).round() as i32,
                r,
                c,
            );
        }
    }
    pub fn rounded_rect(&mut self, r: Rect, radius: i32, c: u32) {
        let q = radius.clamp(0, r.w.min(r.h) / 2);
        self.rect(Rect::new(r.x + q, r.y, r.w - 2 * q, r.h), c);
        self.rect(Rect::new(r.x, r.y + q, r.w, r.h - 2 * q), c);
        self.circle(r.x + q, r.y + q, q, c);
        self.circle(r.x + r.w - q - 1, r.y + q, q, c);
        self.circle(r.x + q, r.y + r.h - q - 1, q, c);
        self.circle(r.x + r.w - q - 1, r.y + r.h - q - 1, q, c);
    }
}

pub(crate) fn apply_opacity(color: u32, opacity: f32) -> u32 {
    let a = ((color >> 24) & 0xff) as f32;
    (((a * opacity).round() as u32) << 24) | (color & 0x00ff_ffff)
}
/// Every alpha after `opacity`, as `apply_opacity` computes it.
pub(crate) fn alpha_table(opacity: f32) -> [u8; 256] {
    std::array::from_fn(|a| (a as f32 * opacity).round() as u8)
}
/// Reference blend (the fast paths match it exactly).
#[cfg(test)]
fn blend(destination: u32, source: u32) -> u32 {
    let a = (source >> 24) & 0xff;
    if a == 0 {
        return destination;
    }
    if a == 255 {
        return source | 0xff00_0000;
    }
    let inv = 255 - a;
    let channel =
        |s: u32| ((((source >> s) & 0xff) * a + ((destination >> s) & 0xff) * inv) / 255) << s;
    0xff00_0000 | channel(16) | channel(8) | channel(0)
}
/// `x / 255` (rounded down) for the two 16-bit lanes of `x` (red and
/// blue), exact for lanes up to 255·255.
#[inline(always)]
fn div255_lanes(x: u32) -> u32 {
    ((x + 0x0001_0001 + ((x >> 8) & 0x00ff_00ff)) >> 8) & 0x00ff_00ff
}
#[inline(always)]
fn div255(x: u32) -> u32 {
    (x + 1 + (x >> 8)) >> 8
}
/// The source's share of a blend: red and blue lanes, green, and what is
/// left for the destination.
#[inline(always)]
fn premultiplied(source: u32, a: u32) -> (u32, u32, u32) {
    (
        (source & 0x00ff_00ff) * a,
        ((source >> 8) & 0xff) * a,
        255 - a,
    )
}
#[inline(always)]
fn mix_premultiplied(destination: u32, rb: u32, g: u32, inv: u32) -> u32 {
    let rb = div255_lanes(rb + (destination & 0x00ff_00ff) * inv);
    let g = div255(g + ((destination >> 8) & 0xff) * inv);
    0xff00_0000 | rb | (g << 8)
}
/// `blend` of `source`'s color at alpha `a` (1..=254), in integers.
#[inline(always)]
pub(crate) fn mix(destination: u32, source: u32, a: u32) -> u32 {
    let (rb, g, inv) = premultiplied(source, a);
    mix_premultiplied(destination, rb, g, inv)
}
/// One pixel of an image over the canvas: `blend` after `alpha`.
#[inline(always)]
pub(crate) fn composite(pixel: &mut u32, source: u32, alpha: &[u8; 256]) {
    let a = u32::from(alpha[(source >> 24) as usize]);
    if a == 255 {
        *pixel = source | 0xff00_0000;
    } else if a != 0 {
        *pixel = mix(*pixel, source, a);
    }
}
/// Coverage placed on the canvas.
struct Mask<'a> {
    x: i32,
    y: i32,
    width: usize,
    height: usize,
    coverage: &'a [u8],
}
impl Mask<'_> {
    fn draw(
        &self,
        pixels: &mut [u32],
        stride: usize,
        visible: Rect,
        color: u32,
        alpha: &[u8; 256],
    ) {
        let area = Rect::new(self.x, self.y, self.width as i32, self.height as i32);
        let Some(area) = visible.intersect(area) else {
            return;
        };
        let rgb = color & 0x00ff_ffff;
        for py in area.y..area.y + area.h {
            let from = (py - self.y) as usize * self.width + (area.x - self.x) as usize;
            let source = &self.coverage[from..from + area.w as usize];
            let at = py as usize * stride + area.x as usize;
            let row = &mut pixels[at..at + area.w as usize];
            for (pixel, &c) in row.iter_mut().zip(source) {
                let a = u32::from(alpha[c as usize]);
                if a == 255 {
                    *pixel = 0xff00_0000 | rgb;
                } else if a != 0 {
                    *pixel = mix(*pixel, rgb, a);
                }
            }
        }
    }
}

/// `coverage` (w×h) dilated by a disk of `radius` pixels with a soft rim.
fn grow(coverage: &[u8], w: usize, h: usize, radius: f32) -> Outline {
    let r = radius.max(0.0);
    let margin = r.ceil() as i32 + 1;
    let mut disk = Vec::new();
    for dy in -margin..=margin {
        for dx in -margin..=margin {
            let distance = ((dx * dx + dy * dy) as f32).sqrt();
            let weight = (r + 0.5 - distance).clamp(0.0, 1.0);
            if weight > 0.0 {
                disk.push((dx, dy, (weight * 255.0).round() as u32));
            }
        }
    }
    let (width, height) = (w + 2 * margin as usize, h + 2 * margin as usize);
    let mut grown = vec![0u8; width * height];
    for y in 0..h {
        for x in 0..w {
            let c = u32::from(coverage[y * w + x]);
            if c == 0 {
                continue;
            }
            for &(dx, dy, weight) in &disk {
                let gx = (x as i32 + margin + dx) as usize;
                let gy = (y as i32 + margin + dy) as usize;
                let cell = &mut grown[gy * width + gx];
                *cell = (*cell).max(((c * weight + 127) / 255) as u8);
            }
        }
    }
    Outline {
        margin,
        width,
        height,
        coverage: grown,
    }
}

fn isqrt(v: i64) -> i64 {
    if v <= 0 {
        return 0;
    }
    let mut r = (v as f64).sqrt() as i64;
    while r * r > v {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= v {
        r += 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_and_opacity_are_applied() {
        let mut canvas = Canvas::new(8, 8);
        canvas.clear(0xff000000);
        canvas.set_clip(Some(Rect::new(2, 2, 2, 2)));
        canvas.rect(Rect::new(0, 0, 8, 8), 0xffffffff);
        assert_eq!(canvas.pixels[0], 0xff000000);
        assert_eq!(canvas.pixels[2 * 8 + 2], 0xffffffff);
        canvas.set_clip(None);
        canvas.set_opacity(0.5);
        canvas.rect(Rect::new(0, 0, 1, 1), 0xffff0000);
        assert_ne!(canvas.pixels[0], 0xffff0000);
        assert_ne!(canvas.pixels[0], 0xff000000);
    }

    #[test]
    fn integer_mix_matches_blend_exactly() {
        let colors = [0xff000000, 0xffffffff, 0xff123456, 0xff80ff01, 0xfffe0102];
        for &destination in &colors {
            for &source in &colors {
                for a in 0..=255u32 {
                    let with_alpha = (a << 24) | (source & 0x00ff_ffff);
                    let mut pixel = destination;
                    composite(&mut pixel, with_alpha, &alpha_table(1.0));
                    assert_eq!(pixel, blend(destination, with_alpha), "{a}");
                }
            }
        }
        for opacity in [0.0, 0.25, 0.5, 0.731, 1.0] {
            let table = alpha_table(opacity);
            for a in 0..=255u32 {
                assert_eq!(
                    u32::from(table[a as usize]),
                    apply_opacity(a << 24, opacity) >> 24
                );
            }
        }
    }

    #[test]
    fn shapes_cover_the_same_pixels_as_the_plain_test() {
        let mut canvas = Canvas::new(40, 40);
        canvas.circle(20, 20, 9, 0xffffffff);
        canvas.ellipse(20, 20, 13, 5, 0xffff0000);
        for y in 0..40i32 {
            for x in 0..40i32 {
                let (dx, dy) = (i64::from(x - 20), i64::from(y - 20));
                let in_ellipse = dx * dx * 25 + dy * dy * 169 <= 169 * 25;
                let in_circle = dx * dx + dy * dy <= 81;
                let expected = if in_ellipse {
                    0xffff0000
                } else if in_circle {
                    0xffffffff
                } else {
                    0xff000000
                };
                assert_eq!(canvas.pixels[(y * 40 + x) as usize], expected, "{x},{y}");
            }
        }
    }
}
