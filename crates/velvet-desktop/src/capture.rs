//! Pictures of the game's page: the PNG checks, the size limit, and the text the game receives.
//!
//! The webview takes the picture (see `webview_capture`). This module checks what comes back, turns
//! it into the data URL that `window.capture()` returns, and writes the `--capture` file. It needs
//! no window, so it is tested directly.

use std::io;
use std::path::Path;

use thiserror::Error;

/// The largest PNG that `window.capture()` returns, in bytes. An 8 MiB PNG becomes a data URL of
/// about 11 MB of text, so bigger pictures are refused.
pub const MAX_CAPTURE_BYTES: usize = 8 * 1024 * 1024;

/// The eight bytes that start every PNG file.
const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

/// Why a picture could not be taken or given to the game.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CaptureError {
    /// The PNG is larger than the limit that `window.capture()` keeps.
    #[error("the picture is {size} bytes, more than the limit of {limit} bytes")]
    TooLarge {
        /// The size of the picture.
        size: usize,
        /// The largest size allowed.
        limit: usize,
    },
    /// The bytes that the webview gave are not a PNG.
    #[error("the web view did not give a PNG picture")]
    NotPng,
    /// The webview could not take the picture.
    #[error("the web view could not take the picture: {0}")]
    Failed(String),
    /// This platform cannot take pictures yet.
    #[error("taking pictures is not supported on this platform yet")]
    Unsupported,
    /// The picture did not come back before its time limit.
    #[error("the picture did not finish within {0} seconds")]
    TimedOut(u64),
}

impl CaptureError {
    /// A short machine-readable code, which the game receives as `error.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge { .. } => "too_large",
            Self::NotPng | Self::Failed(_) => "capture_failed",
            Self::Unsupported => "unsupported",
            Self::TimedOut(_) => "timeout",
        }
    }
}

/// The width and height of a PNG, read from its `IHDR` chunk. `None` when the bytes are not a PNG.
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..8)? != PNG_SIGNATURE || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((width, height))
}

/// The PNG as a data URL, which is what `window.capture()` returns.
///
/// A PNG over [`MAX_CAPTURE_BYTES`] is refused, and so is anything that is not a PNG.
pub fn png_data_url(png: &[u8]) -> Result<String, CaptureError> {
    if png.len() > MAX_CAPTURE_BYTES {
        return Err(CaptureError::TooLarge {
            size: png.len(),
            limit: MAX_CAPTURE_BYTES,
        });
    }
    if png_size(png).is_none() {
        return Err(CaptureError::NotPng);
    }
    Ok(format!("data:image/png;base64,{}", base64_encode(png)))
}

/// Writes the PNG to `path`, creating the folders above it when they are missing.
pub fn write_png(path: &Path, png: &[u8]) -> io::Result<()> {
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        std::fs::create_dir_all(folder)?;
    }
    std::fs::write(path, png)
}

/// Standard base64 (RFC 4648), with `=` padding.
pub fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = u32::from(chunk[0]);
        let second = u32::from(chunk.get(1).copied().unwrap_or(0));
        let third = u32::from(chunk.get(2).copied().unwrap_or(0));
        let group = (first << 16) | (second << 8) | third;
        let symbol = |shift: u32| char::from(ALPHABET[((group >> shift) & 63) as usize]);
        text.push(symbol(18));
        text.push(symbol(12));
        text.push(if chunk.len() > 1 { symbol(6) } else { '=' });
        text.push(if chunk.len() > 2 { symbol(0) } else { '=' });
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG header for a picture of the given size, padded with zeros to `total` bytes.
    fn png_with_size(width: u32, height: u32, total: usize) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.resize(total.max(bytes.len()), 0);
        bytes
    }

    #[test]
    fn base64_matches_the_rfc_test_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, encoded) in cases {
            assert_eq!(
                base64_encode(plain.as_bytes()),
                encoded,
                "encoding `{plain}`"
            );
        }
        assert_eq!(base64_encode(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn the_png_size_is_read_from_the_header() {
        assert_eq!(png_size(&png_with_size(1600, 900, 40)), Some((1600, 900)));
        assert_eq!(png_size(&png_with_size(1, 2, 24)), Some((1, 2)));
    }

    #[test]
    fn bytes_that_are_not_a_png_have_no_size() {
        assert_eq!(png_size(b"GIF89a and more bytes here"), None);
        assert_eq!(png_size(&[]), None);
        // The right signature, but the first chunk is not IHDR.
        let mut wrong_chunk = png_with_size(3, 3, 24);
        wrong_chunk[12..16].copy_from_slice(b"tEXt");
        assert_eq!(png_size(&wrong_chunk), None);
    }

    #[test]
    fn a_picture_becomes_a_png_data_url() {
        let png = png_with_size(2, 2, 24);
        let url = png_data_url(&png).expect("a PNG is accepted");
        assert_eq!(
            url,
            format!("data:image/png;base64,{}", base64_encode(&png))
        );
        assert!(url.starts_with("data:image/png;base64,iVBORw0KGgo"));
    }

    #[test]
    fn a_picture_that_is_not_a_png_is_refused() {
        assert_eq!(png_data_url(b"hello"), Err(CaptureError::NotPng));
    }

    #[test]
    fn the_size_limit_is_checked_for_data_urls() {
        let at_limit = png_with_size(4, 4, MAX_CAPTURE_BYTES);
        assert!(
            png_data_url(&at_limit).is_ok(),
            "exactly the limit is allowed"
        );
        let over = png_with_size(4, 4, MAX_CAPTURE_BYTES + 1);
        assert_eq!(
            png_data_url(&over),
            Err(CaptureError::TooLarge {
                size: MAX_CAPTURE_BYTES + 1,
                limit: MAX_CAPTURE_BYTES,
            })
        );
    }

    #[test]
    fn capture_errors_have_stable_codes() {
        assert_eq!(
            CaptureError::TooLarge { size: 2, limit: 1 }.code(),
            "too_large"
        );
        assert_eq!(CaptureError::NotPng.code(), "capture_failed");
        assert_eq!(CaptureError::Failed("x".into()).code(), "capture_failed");
        assert_eq!(CaptureError::Unsupported.code(), "unsupported");
        assert_eq!(CaptureError::TimedOut(30).code(), "timeout");
    }

    #[test]
    fn a_picture_file_is_written_with_its_folders() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("shots").join("deep").join("page.png");
        let png = png_with_size(5, 6, 24);
        write_png(&path, &png).expect("written with its folders");
        assert_eq!(std::fs::read(&path).expect("read back"), png);
    }
}
