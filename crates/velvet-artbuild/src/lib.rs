//! Builds a Velvet JS game's `art/` folder from source pictures: AI generations, drawings and photographs
//! of any size. Each picture is fitted to the box of its art id in the art manifest, at the output size,
//! and written as PNG, and as WebP and AVIF when ffmpeg is installed. A report says what was built, what
//! is missing and what needs a look.
//!
//! The formats are in `docs/formats/ART_MANIFEST.md` (the manifest that the game reads) and
//! `docs/formats/ART_BUILD.md` (the optional build config, the source rules, the stamps and the report).
//!
//! The parts, in the order a build uses them:
//! 1. `manifest`: reads `art/manifest.json` and checks every rule of the format.
//! 2. `config`: reads `art-src/art-build.json`, the optional build config.
//! 3. `source`: finds the source picture of each id and variant, and decodes it.
//! 4. `fit`: crops, fits or stretches the picture to the box at the output size.
//! 5. `encode`, `ffmpeg` and `files`: write each output atomically. `stamps` skips the outputs that are up
//!    to date.
//! 6. `report`: says what happened, as text or JSON.
//!
//! [`build`] writes the outputs. [`check`] reports only and writes nothing. [`run`] is the command that the
//! `velvet` CLI calls for `velvet art build` and `velvet art check`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod cli;
mod config;
mod encode;
mod error;
mod ffmpeg;
mod files;
mod fit;
mod manifest;
mod output_format;
mod pipeline;
mod report;
mod source;
mod stamps;

pub use cli::{run, ArtArgs, ArtCommand};
pub use config::{BuildConfig, IdOverride, Quality};
pub use error::{ArtBuildError, ErrorCode, Issue, Refusal};
pub use fit::Fit;
pub use manifest::{Entry, Kind, Manifest};
pub use output_format::OutputFormat;
pub use pipeline::{build, check, ArtBuildOptions, MAX_OUTPUT_SIDE};
pub use report::{
    ArtReport, AspectWarning, Failure, HeavyOutput, MissingSource, OutputRecord, SkippedOutput,
    REPORT_FORMAT, REPORT_VERSION,
};

/// Reads an art manifest from its JSON text. The same checks as the build run, with no file access.
///
/// # Errors
/// Returns a [`Refusal`] that lists every problem; its code is the code of the first one.
pub fn parse_manifest(text: &str) -> Result<Manifest, Refusal> {
    manifest::parse(text)
}

/// Reads a build config from its JSON text, checked against the manifest that it overrides.
///
/// # Errors
/// Returns a [`Refusal`] that lists every problem; its code is the code of the first one.
pub fn parse_config(text: &str, manifest: &Manifest) -> Result<BuildConfig, Refusal> {
    config::parse(text, manifest)
}
