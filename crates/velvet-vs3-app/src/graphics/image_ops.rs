use super::canvas::{alpha_table, composite, Canvas, Rect};
use crate::assets::ImageAsset;

/// The parts of each image row that are not fully transparent, so drawing
/// skips the empty pixels and copies the solid ones in one go.
pub struct Spans {
    /// Per row, its range in `spans`.
    rows: Vec<(u32, u32)>,
    /// Start, end and whether every pixel is opaque, in row order.
    spans: Vec<(u32, u32, bool)>,
}

impl Spans {
    /// `None` when the image is too fragmented for spans to pay off.
    pub fn of(image: &ImageAsset) -> Option<Spans> {
        let mut rows = Vec::with_capacity(image.height);
        let mut spans: Vec<(u32, u32, bool)> = Vec::new();
        let limit = image.width * image.height / 8 + 64;
        for row in image.pixels.chunks_exact(image.width.max(1)) {
            let first = spans.len() as u32;
            let mut x = 0;
            while x < row.len() {
                let a = row[x] >> 24;
                if a == 0 {
                    x += 1;
                    continue;
                }
                let solid = a == 255;
                let start = x;
                while x < row.len() && (row[x] >> 24 == 255) == solid && row[x] >> 24 != 0 {
                    x += 1;
                }
                spans.push((start as u32, x as u32, solid));
                if spans.len() > limit {
                    return None;
                }
            }
            rows.push((first, spans.len() as u32));
        }
        Some(Spans { rows, spans })
    }
}

impl Canvas {
    #[allow(clippy::too_many_arguments)]
    pub fn image_region(
        &mut self,
        image: &ImageAsset,
        sx: usize,
        sy: usize,
        sw: usize,
        sh: usize,
        rect: Rect,
        flip_x: bool,
    ) {
        self.image_region_xy(image, sx, sy, sw, sh, rect, flip_x, false)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn image_region_xy(
        &mut self,
        image: &ImageAsset,
        sx0: usize,
        sy0: usize,
        sw: usize,
        sh: usize,
        rect: Rect,
        flip_x: bool,
        flip_y: bool,
    ) {
        if sw == 0 || sh == 0 || rect.w <= 0 || rect.h <= 0 {
            return;
        }
        if !flip_x && !flip_y && rect.w as usize == sw && rect.h as usize == sh {
            self.blit_unscaled(image, sx0, sy0, rect);
            return;
        }
        let Some(area) = self.drawable(rect) else {
            return;
        };
        let alpha = alpha_table(self.opacity());
        // Nearest source column of each visible column, worked out once.
        let columns: Vec<usize> = (area.x..area.x + area.w)
            .map(|px| {
                let dx = px - rect.x;
                let sample_x = if flip_x { rect.w - 1 - dx } else { dx };
                sx0 + (sample_x as usize * sw / rect.w as usize).min(sw - 1)
            })
            .collect();
        let width = self.width;
        for py in area.y..area.y + area.h {
            let dy = py - rect.y;
            let sample_y = if flip_y { rect.h - 1 - dy } else { dy };
            let sy = sy0 + (sample_y as usize * sh / rect.h as usize).min(sh - 1);
            if sy >= image.height {
                continue;
            }
            let source = &image.pixels[sy * image.width..(sy + 1) * image.width];
            let at = py as usize * width + area.x as usize;
            let row = &mut self.pixels[at..at + area.w as usize];
            for (pixel, &sx) in row.iter_mut().zip(&columns) {
                if let Some(&color) = source.get(sx) {
                    composite(pixel, color, &alpha);
                }
            }
        }
    }
    /// Copies a 1:1 region, clipping once instead of per pixel.
    fn blit_unscaled(&mut self, image: &ImageAsset, sx0: usize, sy0: usize, rect: Rect) {
        let Some(area) = self.drawable(rect) else {
            return;
        };
        let sx = sx0 + (area.x - rect.x) as usize;
        if sx >= image.width {
            return;
        }
        let w = (area.w as usize).min(image.width - sx);
        let opaque = self.opacity() >= 1.0;
        let alpha = alpha_table(self.opacity());
        let width = self.width;
        for py in area.y..area.y + area.h {
            let sy = sy0 + (py - rect.y) as usize;
            if sy >= image.height {
                break;
            }
            let from = sy * image.width + sx;
            let source = &image.pixels[from..from + w];
            let at = py as usize * width + area.x as usize;
            let row = &mut self.pixels[at..at + w];
            for (pixel, &color) in row.iter_mut().zip(source) {
                match color >> 24 {
                    0 => {}
                    255 if opaque => *pixel = color,
                    _ => composite(pixel, color, &alpha),
                }
            }
        }
    }
    /// `blit_unscaled` of a whole image, visiting only its `spans`.
    pub fn image_spans(&mut self, image: &ImageAsset, spans: &Spans, rect: Rect) {
        if rect.w as usize != image.width || rect.h as usize != image.height {
            return self.image_region(image, 0, 0, image.width, image.height, rect, false);
        }
        let Some(area) = self.drawable(rect) else {
            return;
        };
        let opaque = self.opacity() >= 1.0;
        let alpha = alpha_table(self.opacity());
        let (left, right) = ((area.x - rect.x) as u32, (area.x + area.w - rect.x) as u32);
        let width = self.width;
        for py in area.y..area.y + area.h {
            let sy = (py - rect.y) as usize;
            let (first, last) = spans.rows[sy];
            let source = &image.pixels[sy * image.width..(sy + 1) * image.width];
            let row = py as usize * width;
            for &(start, end, solid) in &spans.spans[first as usize..last as usize] {
                let (start, end) = (start.max(left), end.min(right));
                if start >= end {
                    continue;
                }
                let from = &source[start as usize..end as usize];
                let at = row + (rect.x + start as i32) as usize;
                let to = &mut self.pixels[at..at + from.len()];
                if solid && opaque {
                    to.copy_from_slice(from);
                } else {
                    for (pixel, &color) in to.iter_mut().zip(from) {
                        composite(pixel, color, &alpha);
                    }
                }
            }
        }
    }
    pub fn image_fit(&mut self, image: &ImageAsset, rect: Rect) {
        let scale = (rect.w as f32 / image.width.max(1) as f32)
            .min(rect.h as f32 / image.height.max(1) as f32);
        let w = (image.width as f32 * scale).round() as i32;
        let h = (image.height as f32 * scale).round() as i32;
        self.image_region(
            image,
            0,
            0,
            image.width,
            image.height,
            Rect::new(rect.x + (rect.w - w) / 2, rect.y + (rect.h - h) / 2, w, h),
            false,
        );
    }
    pub fn image_fit_content(&mut self, image: &ImageAsset, rect: Rect) {
        let (mut min_x, mut min_y, mut max_x, mut max_y, mut found) =
            (image.width, image.height, 0, 0, false);
        for y in 0..image.height {
            for x in 0..image.width {
                if image.pixels[y * image.width + x] >> 24 == 0 {
                    continue;
                }
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x + 1);
                max_y = max_y.max(y + 1);
            }
        }
        if !found {
            return;
        }
        let sw = max_x - min_x;
        let sh = max_y - min_y;
        let scale = (rect.w as f32 / sw.max(1) as f32).min(rect.h as f32 / sh.max(1) as f32);
        let w = (sw as f32 * scale).round() as i32;
        let h = (sh as f32 * scale).round() as i32;
        self.image_region(
            image,
            min_x,
            min_y,
            sw,
            sh,
            Rect::new(rect.x + (rect.w - w) / 2, rect.y + rect.h - h, w, h),
            false,
        );
    }
    pub fn sprite_frame(
        &mut self,
        image: &ImageAsset,
        frame_width: usize,
        frame_index: usize,
        rect: Rect,
        flip_x: bool,
    ) {
        let fw = frame_width.max(1).min(image.width.max(1));
        let frames = (image.width / fw).max(1);
        self.image_region(
            image,
            (frame_index % frames) * fw,
            0,
            fw,
            image.height,
            rect,
            flip_x,
        );
    }
    pub fn nine_slice(
        &mut self,
        image: &ImageAsset,
        source_border: usize,
        rect: Rect,
        destination_border: i32,
    ) {
        let sbx = source_border.min(image.width / 2);
        let sby = source_border.min(image.height / 2);
        let dbx = destination_border.clamp(0, rect.w / 2);
        let dby = destination_border.clamp(0, rect.h / 2);
        let sx = [0, sbx, image.width - sbx, image.width];
        let sy = [0, sby, image.height - sby, image.height];
        let dx = [rect.x, rect.x + dbx, rect.x + rect.w - dbx, rect.x + rect.w];
        let dy = [rect.y, rect.y + dby, rect.y + rect.h - dby, rect.y + rect.h];
        for row in 0..3 {
            for col in 0..3 {
                let sw = sx[col + 1] - sx[col];
                let sh = sy[row + 1] - sy[row];
                let dw = dx[col + 1] - dx[col];
                let dh = dy[row + 1] - dy[row];
                if sw > 0 && sh > 0 && dw > 0 && dh > 0 {
                    self.image_region(
                        image,
                        sx[col],
                        sy[row],
                        sw,
                        sh,
                        Rect::new(dx[col], dy[row], dw, dh),
                        false,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> ImageAsset {
        // Transparent, solid and half-transparent pixels in runs and alone.
        let alphas = [0u32, 0, 255, 255, 255, 128, 0, 7, 255, 0, 0, 200];
        let pixels = (0..12 * 9)
            .map(|i| (alphas[(i * 7 + i / 12) % 12] << 24) | (i as u32 * 0x010305))
            .collect();
        ImageAsset {
            width: 12,
            height: 9,
            pixels,
        }
    }

    #[test]
    fn spans_draw_exactly_like_the_plain_copy() {
        let image = image();
        let spans = Spans::of(&image).expect("small image");
        for opacity in [1.0, 0.6] {
            for (x, y) in [(3, 2), (-4, -3), (14, 8)] {
                let rect = Rect::new(x, y, 12, 9);
                let mut plain = Canvas::new(20, 14);
                let mut fast = Canvas::new(20, 14);
                for canvas in [&mut plain, &mut fast] {
                    canvas.clear(0xff336699);
                    canvas.set_clip(Some(Rect::new(1, 1, 17, 11)));
                    canvas.set_opacity(opacity);
                }
                plain.blit_unscaled(&image, 0, 0, rect);
                fast.image_spans(&image, &spans, rect);
                assert_eq!(plain.pixels, fast.pixels, "{x},{y} at {opacity}");
            }
        }
    }
}
