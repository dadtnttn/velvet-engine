use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fontdue::{Font, FontSettings};

use crate::assets::ImageAsset;

pub const WIDTH: usize = 960;
pub const HEIGHT: usize = 640;

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
}

pub struct FontSystem {
    regular: Font,
    bold: Font,
}

impl FontSystem {
    pub fn load_system() -> Result<Self> {
        let windir = std::env::var_os("WINDIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let fonts = windir.join("Fonts");
        let regular = load_first_font(&[
            fonts.join("segoeui.ttf"),
            fonts.join("arial.ttf"),
            fonts.join("tahoma.ttf"),
        ])?;
        let bold = load_first_font(&[
            fonts.join("segoeuib.ttf"),
            fonts.join("arialbd.ttf"),
            fonts.join("tahomabd.ttf"),
        ])
        .or_else(|_| {
            load_first_font(&[
                fonts.join("segoeui.ttf"),
                fonts.join("arial.ttf"),
                fonts.join("tahoma.ttf"),
            ])
        })?;
        Ok(Self { regular, bold })
    }

    pub fn font(&self, bold: bool) -> &Font {
        if bold {
            &self.bold
        } else {
            &self.regular
        }
    }
}

fn load_first_font(paths: &[PathBuf]) -> Result<Font> {
    let mut errors = Vec::new();
    for path in paths {
        match fs::read(path) {
            Ok(bytes) => {
                return Font::from_bytes(bytes, FontSettings::default()).map_err(|error| {
                    anyhow::anyhow!("no se pudo interpretar {}: {error}", path.display())
                });
            }
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    anyhow::bail!(
        "no se encontró una fuente del sistema: {}",
        errors.join(" | ")
    )
}

pub struct Canvas {
    pub pixels: Vec<u32>,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            pixels: vec![0xff000000; WIDTH * HEIGHT],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    pub fn rect(&mut self, rect: Rect, color: u32) {
        let x0 = rect.x.max(0) as usize;
        let y0 = rect.y.max(0) as usize;
        let x1 = (rect.x + rect.w).clamp(0, WIDTH as i32) as usize;
        let y1 = (rect.y + rect.h).clamp(0, HEIGHT as i32) as usize;
        for y in y0..y1 {
            let start = y * WIDTH + x0;
            let end = y * WIDTH + x1;
            self.pixels[start..end].fill(color);
        }
    }

    pub fn alpha_rect(&mut self, rect: Rect, color: u32) {
        let x0 = rect.x.max(0) as usize;
        let y0 = rect.y.max(0) as usize;
        let x1 = (rect.x + rect.w).clamp(0, WIDTH as i32) as usize;
        let y1 = (rect.y + rect.h).clamp(0, HEIGHT as i32) as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let index = y * WIDTH + x;
                self.pixels[index] = blend(self.pixels[index], color);
            }
        }
    }

    pub fn border(&mut self, rect: Rect, thickness: i32, color: u32) {
        self.rect(Rect::new(rect.x, rect.y, rect.w, thickness), color);
        self.rect(
            Rect::new(rect.x, rect.y + rect.h - thickness, rect.w, thickness),
            color,
        );
        self.rect(Rect::new(rect.x, rect.y, thickness, rect.h), color);
        self.rect(
            Rect::new(rect.x + rect.w - thickness, rect.y, thickness, rect.h),
            color,
        );
    }

    pub fn gradient_vertical(&mut self, rect: Rect, top: u32, bottom: u32) {
        let x0 = rect.x.max(0) as usize;
        let y0 = rect.y.max(0) as usize;
        let x1 = (rect.x + rect.w).clamp(0, WIDTH as i32) as usize;
        let y1 = (rect.y + rect.h).clamp(0, HEIGHT as i32) as usize;
        let height = (y1.saturating_sub(y0)).max(1);
        for (line, y) in (y0..y1).enumerate() {
            let t = line as f32 / height as f32;
            let color = lerp_color(top, bottom, t);
            self.pixels[y * WIDTH + x0..y * WIDTH + x1].fill(color);
        }
    }

    pub fn image_cover(&mut self, image: &ImageAsset, rect: Rect) {
        let target_ratio = rect.w as f32 / rect.h.max(1) as f32;
        let source_ratio = image.width as f32 / image.height.max(1) as f32;
        let (crop_x, crop_y, crop_w, crop_h) = if source_ratio > target_ratio {
            let crop_w = (image.height as f32 * target_ratio) as usize;
            ((image.width - crop_w) / 2, 0, crop_w, image.height)
        } else {
            let crop_h = (image.width as f32 / target_ratio) as usize;
            (0, (image.height - crop_h) / 2, image.width, crop_h)
        };
        self.image_region(image, crop_x, crop_y, crop_w, crop_h, rect, true);
    }

    pub fn image_fit(&mut self, image: &ImageAsset, rect: Rect, alpha: bool) {
        let scale = (rect.w as f32 / image.width.max(1) as f32)
            .min(rect.h as f32 / image.height.max(1) as f32);
        let width = (image.width as f32 * scale).round() as i32;
        let height = (image.height as f32 * scale).round() as i32;
        let target = Rect::new(
            rect.x + (rect.w - width) / 2,
            rect.y + (rect.h - height) / 2,
            width,
            height,
        );
        self.image_region(image, 0, 0, image.width, image.height, target, alpha);
    }

    #[allow(clippy::too_many_arguments)]
    fn image_region(
        &mut self,
        image: &ImageAsset,
        source_x: usize,
        source_y: usize,
        source_w: usize,
        source_h: usize,
        rect: Rect,
        alpha: bool,
    ) {
        if rect.w <= 0 || rect.h <= 0 || source_w == 0 || source_h == 0 {
            return;
        }
        for dy in 0..rect.h {
            let py = rect.y + dy;
            if py < 0 || py >= HEIGHT as i32 {
                continue;
            }
            let sy = source_y + (dy as usize * source_h / rect.h as usize).min(source_h - 1);
            for dx in 0..rect.w {
                let px = rect.x + dx;
                if px < 0 || px >= WIDTH as i32 {
                    continue;
                }
                let sx = source_x + (dx as usize * source_w / rect.w as usize).min(source_w - 1);
                let source = image.pixels[sy * image.width + sx];
                let index = py as usize * WIDTH + px as usize;
                self.pixels[index] = if alpha {
                    blend(self.pixels[index], source)
                } else {
                    source | 0xff000000
                };
            }
        }
    }

    pub fn text_line(
        &mut self,
        font: &Font,
        text: &str,
        x: i32,
        baseline_y: i32,
        size: f32,
        color: u32,
    ) -> i32 {
        let mut pen_x = x as f32;
        for character in text.chars() {
            if character == '\t' {
                pen_x += size * 1.4;
                continue;
            }
            let (metrics, bitmap) = font.rasterize(character, size);
            let glyph_x = pen_x as i32 + metrics.xmin;
            let glyph_y = baseline_y - metrics.height as i32 - metrics.ymin;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let alpha = bitmap[row * metrics.width + column] as u32;
                    if alpha == 0 {
                        continue;
                    }
                    let px = glyph_x + column as i32;
                    let py = glyph_y + row as i32;
                    if px < 0 || py < 0 || px >= WIDTH as i32 || py >= HEIGHT as i32 {
                        continue;
                    }
                    let source = (alpha << 24) | (color & 0x00ff_ffff);
                    let index = py as usize * WIDTH + px as usize;
                    self.pixels[index] = blend(self.pixels[index], source);
                }
            }
            pen_x += metrics.advance_width;
        }
        pen_x.round() as i32
    }

    pub fn measure_text(font: &Font, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|character| font.metrics(character, size).advance_width)
            .sum()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text_wrapped(
        &mut self,
        font: &Font,
        text: &str,
        x: i32,
        y: i32,
        max_width: i32,
        size: f32,
        line_height: i32,
        color: u32,
    ) -> i32 {
        let mut cursor_y = y;
        for paragraph in text.split('\n') {
            if paragraph.is_empty() {
                cursor_y += line_height;
                continue;
            }
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let candidate = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && Self::measure_text(font, &candidate, size) > max_width as f32
                {
                    self.text_line(font, &line, x, cursor_y + size as i32, size, color);
                    cursor_y += line_height;
                    line.clear();
                    line.push_str(word);
                } else {
                    line = candidate;
                }
            }
            if !line.is_empty() {
                self.text_line(font, &line, x, cursor_y + size as i32, size, color);
                cursor_y += line_height;
            }
        }
        cursor_y
    }

    pub fn button(
        &mut self,
        fonts: &FontSystem,
        rect: Rect,
        label: &str,
        hovered: bool,
        enabled: bool,
        selected: bool,
    ) {
        let (top, bottom, border, text) = if !enabled {
            (0xff343541, 0xff292a34, 0xff51525e, 0xff8e909a)
        } else if selected || hovered {
            (0xffe95f86, 0xffaa3f67, 0xffffa4bb, 0xffffffff)
        } else {
            (0xff333649, 0xff232536, 0xff666a82, 0xfff1edf5)
        };
        self.gradient_vertical(rect, top, bottom);
        self.border(rect, 2, border);
        let font = fonts.font(true);
        let mut size = 17.0;
        while size > 10.0 && Self::measure_text(font, label, size) > (rect.w - 18) as f32 {
            size -= 1.0;
        }
        let width = Self::measure_text(font, label, size) as i32;
        let x = rect.x + (rect.w - width) / 2;
        let y = rect.y + (rect.h - size as i32) / 2 + size as i32 - 2;
        self.text_line(font, label, x, y, size, text);
    }

    pub fn stat_bar(
        &mut self,
        fonts: &FontSystem,
        rect: Rect,
        label: &str,
        value: i32,
        color: u32,
    ) {
        self.text_line(
            fonts.font(true),
            label,
            rect.x,
            rect.y + 14,
            13.0,
            0xfff4eff7,
        );
        let bar = Rect::new(rect.x, rect.y + 20, rect.w, 11);
        self.rect(bar, 0xff252735);
        let fill = Rect::new(bar.x, bar.y, bar.w * value.clamp(0, 100) / 100, bar.h);
        self.rect(fill, color);
        self.border(bar, 1, 0xff676a78);
    }
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | b as u32
}

fn blend(destination: u32, source: u32) -> u32 {
    let alpha = (source >> 24) & 0xff;
    if alpha == 0 {
        return destination;
    }
    if alpha == 255 {
        return source | 0xff00_0000;
    }
    let inverse = 255 - alpha;
    let sr = (source >> 16) & 0xff;
    let sg = (source >> 8) & 0xff;
    let sb = source & 0xff;
    let dr = (destination >> 16) & 0xff;
    let dg = (destination >> 8) & 0xff;
    let db = destination & 0xff;
    let r = (sr * alpha + dr * inverse) / 255;
    let g = (sg * alpha + dg * inverse) / 255;
    let b = (sb * alpha + db * inverse) / 255;
    0xff00_0000 | (r << 16) | (g << 8) | b
}

fn lerp_color(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let av = ((a >> shift) & 0xff) as f32;
        let bv = ((b >> shift) & 0xff) as f32;
        (av + (bv - av) * t).round() as u32
    };
    0xff00_0000 | (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

pub fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("no se pudo crear {}", parent.display()))?;
    }
    Ok(())
}
