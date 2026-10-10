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

/// Cached glyph coverage keyed by font identity, character and pixel size.
type GlyphKey = (usize, char, u32);
/// Upper bound on cached glyphs before the cache is reset.
const MAX_CACHED_GLYPHS: usize = 16_384;

pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    clip: Option<Rect>,
    opacity: f32,
    glyphs: HashMap<GlyphKey, (fontdue::Metrics, Vec<u8>)>,
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
        }
    }
    /// Rasterizes `ch` once per font and size; later calls reuse the coverage.
    pub(crate) fn glyph(
        &mut self,
        font: &fontdue::Font,
        ch: char,
        size: f32,
    ) -> (fontdue::Metrics, &[u8]) {
        let key = (font as *const fontdue::Font as usize, ch, size.to_bits());
        if !self.glyphs.contains_key(&key) {
            if self.glyphs.len() >= MAX_CACHED_GLYPHS {
                self.glyphs.clear();
            }
            self.glyphs.insert(key, font.rasterize(ch, size));
        }
        let (metrics, bitmap) = &self.glyphs[&key];
        (*metrics, bitmap.as_slice())
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
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.width as i32, self.height as i32)
    }
    pub fn visible(&self) -> Option<Rect> {
        self.clip.unwrap_or(self.bounds()).intersect(self.bounds())
    }
    pub fn accepts(&self, x: i32, y: i32) -> bool {
        self.visible()
            .is_some_and(|r| x >= r.x && y >= r.y && x < r.x + r.w && y < r.y + r.h)
    }
    pub fn put(&mut self, x: i32, y: i32, color: u32, alpha: bool) {
        if !self.accepts(x, y) {
            return;
        }
        let index = y as usize * self.width + x as usize;
        let source = apply_opacity(color, self.opacity);
        self.pixels[index] = if alpha || self.opacity < 1.0 {
            blend(self.pixels[index], source)
        } else {
            source
        };
    }
    pub fn rect(&mut self, rect: Rect, color: u32) {
        let Some(area) = rect.intersect(self.visible().unwrap_or(Rect::new(0, 0, 0, 0))) else {
            return;
        };
        let source = apply_opacity(color, self.opacity);
        for y in area.y..area.y + area.h {
            for x in area.x..area.x + area.w {
                let i = y as usize * self.width + x as usize;
                self.pixels[i] = if self.opacity < 1.0 {
                    blend(self.pixels[i], source)
                } else {
                    source
                };
            }
        }
    }
    pub fn alpha_rect(&mut self, rect: Rect, color: u32) {
        let Some(area) = rect.intersect(self.visible().unwrap_or(Rect::new(0, 0, 0, 0))) else {
            return;
        };
        for y in area.y..area.y + area.h {
            for x in area.x..area.x + area.w {
                self.put(x, y, color, true);
            }
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
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= r * r {
                    self.put(x, y, c, false);
                }
            }
        }
    }
    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        self.ellipse_impl(cx, cy, rx, ry, c, false)
    }
    pub fn alpha_ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32) {
        self.ellipse_impl(cx, cy, rx, ry, c, true)
    }
    fn ellipse_impl(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, c: u32, a: bool) {
        if rx <= 0 || ry <= 0 {
            return;
        }
        let rx2 = (rx * rx) as i64;
        let ry2 = (ry * ry) as i64;
        let limit = rx2 * ry2;
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let dx = (x - cx) as i64;
                let dy = (y - cy) as i64;
                if dx * dx * ry2 + dy * dy * rx2 <= limit {
                    self.put(x, y, c, a);
                }
            }
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
pub(crate) fn blend(destination: u32, source: u32) -> u32 {
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
}
