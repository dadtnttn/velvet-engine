//! The file formats that the build writes. Hand-made SVG files are not written by the build: they
//! are art that a person draws, and the build leaves them alone.

use serde::Serialize;

/// A file format that the build writes. The order is the preference of the runtime: AVIF first,
/// then WebP, then PNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// AV1 still picture in an AVIF file, with its alpha as a second stream. Needs ffmpeg.
    Avif,
    /// WebP, lossy with alpha. Needs ffmpeg.
    Webp,
    /// PNG, lossless with alpha, written by the Rust encoder of the `image` crate.
    Png,
}

impl OutputFormat {
    /// Every format, in the order of preference.
    pub const ALL: [OutputFormat; 3] = [OutputFormat::Avif, OutputFormat::Webp, OutputFormat::Png];

    /// The file extension of the format, without the dot.
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Avif => "avif",
            OutputFormat::Webp => "webp",
            OutputFormat::Png => "png",
        }
    }

    /// The format with this name (`avif`, `webp` or `png`), or None for any other name.
    pub fn from_name(name: &str) -> Option<OutputFormat> {
        OutputFormat::ALL
            .into_iter()
            .find(|format| format.extension() == name)
    }

    /// True for the formats that need ffmpeg, because no Rust encoder writes them here.
    pub fn needs_ffmpeg(self) -> bool {
        matches!(self, OutputFormat::Avif | OutputFormat::Webp)
    }
}
