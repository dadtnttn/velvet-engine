//! Writes one rendered picture in one output format. PNG is written here with the `image` crate;
//! WebP and AVIF go through ffmpeg (see `ffmpeg.rs`). Every file is written atomically.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use image::codecs::png::{CompressionType, FilterType as PngFilter, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

use crate::config::Quality;
use crate::ffmpeg;
use crate::files;
use crate::output_format::OutputFormat;

/// Writes `picture` to `target` in `format`, at the quality of the config.
///
/// # Errors
/// Returns the reason when the file cannot be encoded or written. Nothing is left at `target` then.
pub fn write(
    format: OutputFormat,
    picture: &RgbaImage,
    target: &Path,
    quality: Quality,
    ffmpeg_program: &OsStr,
) -> Result<(), String> {
    files::write_atomic(target, |temporary| match format {
        OutputFormat::Png => write_png(picture, temporary),
        OutputFormat::Webp | OutputFormat::Avif => {
            ffmpeg::encode(ffmpeg_program, format, picture, quality, temporary)
        }
    })
}

/// Writes a picture as PNG: RGBA, 8 bits a channel, with the default compression and an adaptive filter.
fn write_png(picture: &RgbaImage, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|error| format!("cannot create the file: {error}"))?;
    let mut writer = BufWriter::new(file);
    let (width, height) = picture.dimensions();
    PngEncoder::new_with_quality(&mut writer, CompressionType::Default, PngFilter::Adaptive)
        .write_image(picture.as_raw(), width, height, ExtendedColorType::Rgba8)
        .map_err(|error| format!("cannot encode the PNG: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("cannot write the PNG: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_round_trips_its_pixels() {
        let folder =
            std::env::temp_dir().join(format!("velvet-artbuild-png-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let picture = RgbaImage::from_fn(5, 4, |x, y| {
            image::Rgba([x as u8 * 40, y as u8 * 50, 7, 200])
        });
        let target = folder.join("out.png");
        write(
            OutputFormat::Png,
            &picture,
            &target,
            Quality::default(),
            OsStr::new("ffmpeg"),
        )
        .unwrap();
        let back = image::open(&target).unwrap().to_rgba8();
        assert_eq!(back, picture);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
