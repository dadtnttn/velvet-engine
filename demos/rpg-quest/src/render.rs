use std::fs;
use std::path::PathBuf;

use anyhow::Result;
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
}

pub struct FontSystem {
    regular: Font,
    bold: Font,
    pixel: Font,
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
            fonts.join("segoeui.ttf"),
        ])?;
        let pixel = load_first_font(&[
            PathBuf::from("demos/rpg-quest/assets/ui/Toriko.ttf"),
            PathBuf::from("assets/ui/Toriko.ttf"),
            PathBuf::from("../rpg-quest/assets/ui/Toriko.ttf"),
            fonts.join("segoeuib.ttf"),
        ])?;
        Ok(Self {
            regular,
            bold,
            pixel,
        })
    }

    pub fn font(&self, bold: bool) -> &Font {
        if bold {
            &self.bold
        } else {
            &self.regular
        }
    }

    pub fn pixel(&self) -> &Font {
        &self.pixel
    }
}

fn load_first_font(paths: &[PathBuf]) -> Result<Font> {
    let mut errors = Vec::new();
    for path in paths {
        match fs::read(path) {
            Ok(bytes) => {
                return Font::from_bytes(bytes, FontSettings::default()).map_err(|error| {
                    anyhow::anyhow!("fuente {} inválida: {error}", path.display())
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
            self.pixels[y * WIDTH + x0..y * WIDTH + x1].fill(color);
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

    pub fn circle(&mut self, cx: i32, cy: i32, radius: i32, color: u32) {
        let y0 = (cy - radius).max(0);
        let y1 = (cy + radius).min(HEIGHT as i32 - 1);
        let x0 = (cx - radius).max(0);
        let x1 = (cx + radius).min(WIDTH as i32 - 1);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= radius * radius {
                    self.pixels[y as usize * WIDTH + x as usize] = color;
                }
            }
        }
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

    #[allow(clippy::too_many_arguments)]
    pub fn image_region(
        &mut self,
        image: &ImageAsset,
        source_x: usize,
        source_y: usize,
        source_w: usize,
        source_h: usize,
        rect: Rect,
        flip_x: bool,
    ) {
        if source_w == 0 || source_h == 0 || rect.w <= 0 || rect.h <= 0 {
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
                let sample_dx = if flip_x { rect.w - 1 - dx } else { dx };
                let sx =
                    source_x + (sample_dx as usize * source_w / rect.w as usize).min(source_w - 1);
                let source = image.pixels[sy * image.width + sx];
                let index = py as usize * WIDTH + px as usize;
                self.pixels[index] = blend(self.pixels[index], source);
            }
        }
    }

    pub fn nine_slice(
        &mut self,
        image: &ImageAsset,
        source_border: usize,
        rect: Rect,
        dest_border: i32,
    ) {
        if rect.w <= 0 || rect.h <= 0 || image.width == 0 || image.height == 0 {
            return;
        }
        let source_x_border = source_border.min(image.width / 2);
        let source_y_border = source_border.min(image.height / 2);
        let dest_x_border = dest_border.max(0).min(rect.w / 2);
        let dest_y_border = dest_border.max(0).min(rect.h / 2);

        let sx = [
            0,
            source_x_border,
            image.width - source_x_border,
            image.width,
        ];
        let sy = [
            0,
            source_y_border,
            image.height - source_y_border,
            image.height,
        ];
        let dx = [
            rect.x,
            rect.x + dest_x_border,
            rect.x + rect.w - dest_x_border,
            rect.x + rect.w,
        ];
        let dy = [
            rect.y,
            rect.y + dest_y_border,
            rect.y + rect.h - dest_y_border,
            rect.y + rect.h,
        ];

        for row in 0..3 {
            for column in 0..3 {
                let source_w = sx[column + 1] - sx[column];
                let source_h = sy[row + 1] - sy[row];
                let dest_w = dx[column + 1] - dx[column];
                let dest_h = dy[row + 1] - dy[row];
                if source_w == 0 || source_h == 0 || dest_w <= 0 || dest_h <= 0 {
                    continue;
                }
                self.image_region(
                    image,
                    sx[column],
                    sy[row],
                    source_w,
                    source_h,
                    Rect::new(dx[column], dy[row], dest_w, dest_h),
                    false,
                );
            }
        }
    }

    pub fn horizontal_slice(
        &mut self,
        image: &ImageAsset,
        source_cap: usize,
        rect: Rect,
        dest_cap: i32,
    ) {
        if rect.w <= 0 || rect.h <= 0 || image.width == 0 || image.height == 0 {
            return;
        }
        let source_cap = source_cap.min(image.width / 2);
        let dest_cap = dest_cap.max(0).min(rect.w / 2);
        let source_center = image.width.saturating_sub(source_cap * 2);
        let dest_center = rect.w - dest_cap * 2;

        self.image_region(
            image,
            0,
            0,
            source_cap,
            image.height,
            Rect::new(rect.x, rect.y, dest_cap, rect.h),
            false,
        );
        if source_center > 0 && dest_center > 0 {
            self.image_region(
                image,
                source_cap,
                0,
                source_center,
                image.height,
                Rect::new(rect.x + dest_cap, rect.y, dest_center, rect.h),
                false,
            );
        }
        self.image_region(
            image,
            image.width - source_cap,
            0,
            source_cap,
            image.height,
            Rect::new(rect.x + rect.w - dest_cap, rect.y, dest_cap, rect.h),
            false,
        );
    }

    pub fn image_fit(&mut self, image: &ImageAsset, rect: Rect) {
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
        self.image_region(image, 0, 0, image.width, image.height, target, false);
    }

    pub fn sprite_frame(
        &mut self,
        image: &ImageAsset,
        frame_width: usize,
        frame_index: usize,
        rect: Rect,
        flip_x: bool,
    ) {
        let frames = (image.width / frame_width.max(1)).max(1);
        let frame = frame_index % frames;
        self.image_region(
            image,
            frame * frame_width,
            0,
            frame_width.min(image.width),
            image.height,
            rect,
            flip_x,
        );
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
                    line = word.to_owned();
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

    pub fn bar(&mut self, rect: Rect, value: f32, color: u32) {
        self.rect(rect, 0xff211f28);
        let fill = Rect::new(
            rect.x,
            rect.y,
            ((rect.w as f32 * value.clamp(0.0, 1.0)).round() as i32).max(0),
            rect.h,
        );
        self.rect(fill, color);
        self.border(rect, 1, 0xff6d6875);
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
