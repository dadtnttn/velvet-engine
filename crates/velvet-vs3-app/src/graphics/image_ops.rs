use super::canvas::{Canvas, Rect};
use crate::assets::ImageAsset;

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
        for dy in 0..rect.h {
            let py = rect.y + dy;
            let sample_y = if flip_y { rect.h - 1 - dy } else { dy };
            let sy = sy0 + (sample_y as usize * sh / rect.h as usize).min(sh - 1);
            for dx in 0..rect.w {
                let px = rect.x + dx;
                if !self.accepts(px, py) {
                    continue;
                }
                let sample_x = if flip_x { rect.w - 1 - dx } else { dx };
                let sx = sx0 + (sample_x as usize * sw / rect.w as usize).min(sw - 1);
                if sx >= image.width || sy >= image.height {
                    continue;
                }
                self.put(px, py, image.pixels[sy * image.width + sx], true);
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
