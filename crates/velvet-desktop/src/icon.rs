//! The game's icon: a square PNG in the game folder, named by the manifest's `icon`.
//!
//! The window shows it from the start (see `app`), and `velvet build --runtime js` writes it into
//! the program's file. This module only reads and decodes the file. It needs no window, so the
//! packaging step can use it too.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// A decoded icon: its size and its pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconImage {
    /// Width, in pixels.
    pub width: u32,
    /// Height, in pixels.
    pub height: u32,
    /// The pixels from the top row down, four bytes each: red, green, blue and alpha.
    pub rgba: Vec<u8>,
}

/// Why an icon could not be read.
#[derive(Debug, Error)]
pub enum IconError {
    /// The file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The file that failed.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The bytes are not a PNG image, or the image cannot be decoded.
    #[error("not a readable PNG: {0}")]
    Decode(#[from] png::DecodingError),
    /// The image uses a colour type that the conversion to RGBA does not cover.
    #[error("unsupported PNG colour type {0:?}")]
    ColourType(png::ColorType),
}

/// Reads and decodes the PNG file at `path`.
pub fn load(path: &Path) -> Result<IconImage, IconError> {
    let bytes = fs::read(path).map_err(|source| IconError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    decode_png(&bytes)
}

/// Decodes PNG bytes into 8-bit RGBA pixels.
///
/// Palette, grey and 16-bit images are converted. A grey or colour image without alpha is opaque.
pub fn decode_png(bytes: &[u8]) -> Result<IconImage, IconError> {
    let mut decoder = png::Decoder::new(io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or(png::DecodingError::LimitsExceeded)?;
    let mut buffer = vec![0; size];
    let info = reader.next_frame(&mut buffer)?;
    buffer.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
            .collect(),
        png::ColorType::Grayscale => buffer
            .iter()
            .flat_map(|&grey| [grey, grey, grey, 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buffer
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
            .collect(),
        other => return Err(IconError::ColourType(other)),
    };
    Ok(IconImage {
        width: info.width,
        height: info.height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes pixels as an 8-bit PNG of the given colour type.
    fn png_of(width: u32, height: u32, color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(pixels).expect("pixels");
        writer.finish().expect("finish");
        bytes
    }

    #[test]
    fn an_rgba_png_keeps_its_pixels() {
        let pixels = [1, 2, 3, 4, 5, 6, 7, 8];
        let image = decode_png(&png_of(2, 1, png::ColorType::Rgba, &pixels)).expect("decodes");
        assert_eq!((image.width, image.height), (2, 1));
        assert_eq!(image.rgba, pixels);
    }

    #[test]
    fn colour_and_grey_images_become_rgba() {
        let rgb = decode_png(&png_of(1, 1, png::ColorType::Rgb, &[9, 8, 7])).expect("decodes");
        assert_eq!(rgb.rgba, [9, 8, 7, 255]);
        let grey = decode_png(&png_of(1, 1, png::ColorType::Grayscale, &[50])).expect("decodes");
        assert_eq!(grey.rgba, [50, 50, 50, 255]);
        let grey_alpha =
            decode_png(&png_of(1, 1, png::ColorType::GrayscaleAlpha, &[60, 128])).expect("decodes");
        assert_eq!(grey_alpha.rgba, [60, 60, 60, 128]);
    }

    #[test]
    fn a_palette_image_is_expanded_with_its_transparency() {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(vec![255, 0, 0, 0, 255, 0]);
        encoder.set_trns(vec![0]);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(&[0, 1]).expect("pixels");
        writer.finish().expect("finish");
        let image = decode_png(&bytes).expect("decodes");
        assert_eq!(image.rgba, [255, 0, 0, 0, 0, 255, 0, 255]);
    }

    #[test]
    fn bytes_that_are_not_a_png_are_refused() {
        assert!(matches!(
            decode_png(b"not a picture"),
            Err(IconError::Decode(_))
        ));
    }

    #[test]
    fn a_png_file_is_loaded_and_a_missing_one_is_a_read_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("icon.png");
        fs::write(&path, png_of(1, 1, png::ColorType::Rgba, &[1, 2, 3, 4])).expect("write");
        assert_eq!(load(&path).expect("loads").rgba, [1, 2, 3, 4]);
        assert!(matches!(
            load(&dir.path().join("missing.png")),
            Err(IconError::Read { .. })
        ));
    }
}
