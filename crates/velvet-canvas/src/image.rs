//! Images: PNG, WebP or JPEG files kept as premultiplied pixels, with smaller copies made on demand so a big picture
//! drawn small stays smooth.

use std::cell::RefCell;
use std::fmt;

use crate::geometry::Rect;

/// An image could not be read.
#[derive(Debug)]
pub struct ImageError(String);

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the image could not be read: {}", self.0)
    }
}

impl std::error::Error for ImageError {}

/// How an image fills an area without being deformed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fit {
    /// All of it inside the area, centred; the area may show around it.
    Contain,
    /// The whole area covered, centred; the image may be cut.
    Cover,
}

/// An image in memory.
#[derive(Debug)]
pub struct Image {
    pub(crate) pixmap: tiny_skia::Pixmap,
    /// Smaller copies already made, by their pixel size.
    smaller: RefCell<Vec<tiny_skia::Pixmap>>,
}

impl Image {
    /// Reads a PNG, WebP or JPEG file.
    pub fn decode(bytes: &[u8]) -> Result<Image, ImageError> {
        let decoded = ::image::load_from_memory(bytes)
            .map_err(|e| ImageError(e.to_string()))?
            .into_rgba8();
        let (w, h) = decoded.dimensions();
        Image::from_rgba(w, h, decoded.into_raw())
    }

    /// An image from straight (not premultiplied) RGBA pixels, row by row.
    pub fn from_rgba(width: u32, height: u32, mut rgba: Vec<u8>) -> Result<Image, ImageError> {
        if rgba.len() != (width as usize) * (height as usize) * 4 {
            return Err(ImageError(format!(
                "{} bytes for {width}×{height} pixels",
                rgba.len()
            )));
        }
        for i in (0..rgba.len()).step_by(4) {
            let a = rgba[i + 3] as u16;
            if a < 255 {
                for c in &mut rgba[i..i + 3] {
                    *c = ((*c as u16 * a + 127) / 255) as u8;
                }
            }
        }
        let size = tiny_skia::IntSize::from_wh(width, height)
            .ok_or_else(|| ImageError("an empty image".into()))?;
        let pixmap =
            tiny_skia::Pixmap::from_vec(rgba, size).ok_or_else(|| ImageError("bad size".into()))?;
        Ok(Image {
            pixmap,
            smaller: RefCell::new(Vec::new()),
        })
    }

    pub fn width(&self) -> u32 {
        self.pixmap.width()
    }

    pub fn height(&self) -> u32 {
        self.pixmap.height()
    }

    /// Where the image goes inside an area, keeping its proportions.
    pub fn fit(&self, area: Rect, fit: Fit) -> Rect {
        let (iw, ih) = (self.width() as f32, self.height() as f32);
        let scale = match fit {
            Fit::Contain => (area.w / iw).min(area.h / ih),
            Fit::Cover => (area.w / iw).max(area.h / ih),
        };
        let (w, h) = (iw * scale, ih * scale);
        Rect::new(
            area.x + (area.w - w) / 2.0,
            area.y + (area.h - h) / 2.0,
            w,
            h,
        )
    }

    /// The pixels to draw at a size: the image itself, or a smaller copy when it is drawn at less than about half its
    /// size (sampling the big one would skip pixels and shimmer).
    pub(crate) fn pixels_for(&self, pw: u32, ph: u32, draw: impl FnOnce(&tiny_skia::Pixmap)) {
        if pw * 2 >= self.width() || ph * 2 >= self.height() || pw == 0 || ph == 0 {
            draw(&self.pixmap);
            return;
        }
        let mut smaller = self.smaller.borrow_mut();
        if let Some(found) = smaller.iter().find(|p| p.width() == pw && p.height() == ph) {
            draw(found);
            return;
        }
        let source =
            ::image::RgbaImage::from_raw(self.width(), self.height(), self.pixmap.data().to_vec());
        let Some(source) = source else {
            draw(&self.pixmap);
            return;
        };
        // Premultiplied pixels resample correctly as they are (no dark fringes at the edges).
        let resized =
            ::image::imageops::resize(&source, pw, ph, ::image::imageops::FilterType::Triangle);
        let Some(pixmap) = tiny_skia::IntSize::from_wh(pw, ph)
            .and_then(|size| tiny_skia::Pixmap::from_vec(resized.into_raw(), size))
        else {
            draw(&self.pixmap);
            return;
        };
        if smaller.len() >= 4 {
            smaller.remove(0);
        }
        smaller.push(pixmap);
        draw(smaller.last().expect("just pushed"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rgba_premultiplies_and_checks_the_size() {
        let img = Image::from_rgba(1, 1, vec![200, 100, 50, 128]).unwrap();
        assert_eq!(img.pixmap.data(), &[100, 50, 25, 128]);
        assert!(Image::from_rgba(2, 2, vec![0; 4]).is_err());
    }

    #[test]
    fn fit_keeps_the_proportions() {
        let img = Image::from_rgba(200, 100, vec![255; 200 * 100 * 4]).unwrap();
        assert_eq!(
            img.fit(Rect::new(0.0, 0.0, 100.0, 100.0), Fit::Contain),
            Rect::new(0.0, 25.0, 100.0, 50.0)
        );
        assert_eq!(
            img.fit(Rect::new(0.0, 0.0, 100.0, 100.0), Fit::Cover),
            Rect::new(-50.0, 0.0, 200.0, 100.0)
        );
    }
}
