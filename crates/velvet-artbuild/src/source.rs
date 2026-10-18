//! Finds the source picture of each art name, in `art-src/`, and decodes it. A source is an AI
//! generation, a drawing or a photograph of any size, in PNG, JPEG or AVIF.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};

use crate::config::SOURCE_EXTENSIONS;
use crate::manifest::Kind;

/// A source picture that was found on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The path of the file.
    pub path: PathBuf,
    /// The file's path inside the source folder, with `/` separators, such as `portrait/lyra.png`.
    pub relative: String,
}

/// Finds the source of one art name.
///
/// Without an explicit file name, the candidates are `<stem>.png`, `<stem>.jpg`, `<stem>.jpeg` and
/// `<stem>.avif`, in that order, first in the source folder and then in its folder for the kind
/// (`<src>/<kind>/`). The first file that exists wins. With an explicit file name (a base picture's
/// `source` override), the only candidates are that file, first in the source folder and then in the
/// kind's folder.
pub fn find(src: &Path, kind: Kind, stem: &str, explicit: Option<&str>) -> Option<Found> {
    let names: Vec<String> = match explicit {
        Some(name) => vec![name.to_string()],
        None => SOURCE_EXTENSIONS
            .iter()
            .map(|extension| format!("{stem}.{extension}"))
            .collect(),
    };
    for folder in ["", kind.as_str()] {
        for name in &names {
            let relative = if folder.is_empty() {
                name.clone()
            } else {
                format!("{folder}/{name}")
            };
            let path = src.join(&relative);
            if path.is_file() {
                return Some(Found { path, relative });
            }
        }
    }
    None
}

/// Decodes a source picture to RGBA. The format is read from the bytes, so a wrong extension does
/// not matter. JPEG and PNG pictures are turned upright when they carry an EXIF orientation. A
/// picture with more than 8 bits a channel is reduced to 8.
///
/// # Errors
/// Returns the reason when the bytes are not a picture that can be read, or when the picture is empty.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, String> {
    let picture = if velvet_avif::is_avif(bytes) {
        velvet_avif::decode_avif(bytes)
            .map_err(|error| format!("cannot decode the AVIF picture: {error}"))?
    } else {
        decode_other(bytes)?.to_rgba8()
    };
    if picture.width() == 0 || picture.height() == 0 {
        return Err("the picture is empty".to_string());
    }
    Ok(picture)
}

/// Decodes a PNG or JPEG (or any other format that the `image` crate reads here) and applies its
/// orientation.
fn decode_other(bytes: &[u8]) -> Result<DynamicImage, String> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|error| format!("cannot read the picture: {error}"))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|error| format!("cannot decode the picture: {error}"))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut picture = DynamicImage::from_decoder(decoder)
        .map_err(|error| format!("cannot decode the picture: {error}"))?;
    picture.apply_orientation(orientation);
    Ok(picture)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_png_decodes_to_rgba_with_its_size() {
        let picture = RgbaImage::from_pixel(7, 3, image::Rgba([1, 2, 3, 4]));
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(picture.clone())
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.dimensions(), (7, 3));
        assert_eq!(decoded.get_pixel(0, 0).0, [1, 2, 3, 4]);
    }

    #[test]
    fn bytes_that_are_not_a_picture_are_refused() {
        assert!(decode(b"not a picture at all").is_err());
    }

    #[test]
    fn find_prefers_the_source_folder_then_the_kind_folder_and_then_the_extension_order() {
        let root =
            std::env::temp_dir().join(format!("velvet-artbuild-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("portrait")).unwrap();
        std::fs::write(root.join("portrait").join("lyra.png"), b"x").unwrap();
        std::fs::write(root.join("lyra.jpg"), b"x").unwrap();

        let found = find(&root, Kind::Portrait, "lyra", None).unwrap();
        assert_eq!(found.relative, "lyra.jpg");
        assert!(find(&root, Kind::Portrait, "lyra.happy", None).is_none());
        assert_eq!(
            find(&root, Kind::Portrait, "lyra", Some("lyra.png"))
                .unwrap()
                .relative,
            "portrait/lyra.png"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
