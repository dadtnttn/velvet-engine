//! The build and the check. Both read the manifest and the build config, find the source of each art
//! name, decode it, and fit it to the box at the output size. A build then writes each output that is
//! new or changed, and updates the stamps; a check does the same planning but writes nothing. Both end
//! with a scan of `art/` for orphans and heavy files.

use std::collections::{BTreeMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use image::RgbaImage;
use serde_json::{json, Value};
use walkdir::WalkDir;

use crate::config::{self, BuildConfig, IdOverride};
use crate::encode;
use crate::error::{ArtBuildError, Refusal};
use crate::ffmpeg;
use crate::fit::{self, Fit};
use crate::manifest::{self, Entry, Kind, Manifest};
use crate::output_format::OutputFormat;
use crate::report::{
    ArtReport, AspectWarning, Failure, HeavyOutput, MissingSource, OutputRecord, SkippedOutput,
};
use crate::source;
use crate::stamps::{self, Stamp, Stamps};

/// The largest side of an output, in pixels: the largest WebP side. Larger outputs are skipped.
pub const MAX_OUTPUT_SIDE: u32 = 16383;

/// The extensions that make a file a picture of an art name: the four formats of the art manifest.
const ART_EXTENSIONS: [&str; 4] = ["avif", "webp", "png", "svg"];

/// How far a source's proportions may differ from its box's before the report warns, in percent.
const ASPECT_LIMIT_PERCENT: u64 = 15;

/// Options of a build or a check.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtBuildOptions {
    /// The game folder. It holds `art/`, with the manifest and the outputs, and by default `art-src/`.
    pub game: PathBuf,
    /// The folder of source pictures. None means `<game>/art-src`. The build config is read from it too.
    pub src: Option<PathBuf>,
    /// The formats to write. None means the formats of the build config, which defaults to all three.
    pub formats: Option<Vec<OutputFormat>>,
    /// Rebuild every output, even the ones that are up to date.
    pub force: bool,
    /// The ffmpeg program for WebP and AVIF. None means `ffmpeg`, found on the PATH.
    pub ffmpeg: Option<PathBuf>,
}

impl ArtBuildOptions {
    /// Options for the game folder `game`, with the defaults for everything else.
    pub fn new(game: impl Into<PathBuf>) -> Self {
        ArtBuildOptions {
            game: game.into(),
            src: None,
            formats: None,
            force: false,
            ffmpeg: None,
        }
    }
}

/// Builds the outputs that are new or changed, updates the stamps, and reports what happened.
///
/// # Errors
/// Returns an error when the manifest or the build config is refused, or when `art/` or the stamps
/// cannot be read. A source or an output that fails does not stop the build: it is listed in the report.
pub fn build(options: &ArtBuildOptions) -> Result<ArtReport, ArtBuildError> {
    run(options, false)
}

/// Reports what a build would do, and the problems it would find. Nothing is written.
///
/// # Errors
/// The same errors as [`build`].
pub fn check(options: &ArtBuildOptions) -> Result<ArtReport, ArtBuildError> {
    run(options, true)
}

/// The shared run of the build and the check.
fn run(options: &ArtBuildOptions, dry_run: bool) -> Result<ArtReport, ArtBuildError> {
    let art_dir = options.game.join("art");
    let src_dir = options
        .src
        .clone()
        .unwrap_or_else(|| options.game.join("art-src"));
    let manifest = read_manifest(&art_dir.join("manifest.json"))?;
    let config = read_config(&src_dir.join(config::FILE_NAME), &manifest)?;
    let formats = chosen_formats(options.formats.as_deref(), &config);
    let program: OsString = options
        .ffmpeg
        .clone()
        .map_or_else(|| OsString::from("ffmpeg"), PathBuf::into_os_string);
    let mut build = Build {
        art_dir: art_dir.clone(),
        src_dir,
        config: &config,
        formats: &formats,
        unavailable: unavailable_formats(&formats, &program),
        program,
        force: options.force,
        dry_run,
        stamps: Stamps::load(&art_dir),
        stamps_changed: false,
        report: ArtReport::new(dry_run),
    };
    for entry in &manifest.entries {
        build.entry(entry);
    }

    let (orphans, heavy) = scan_art(&art_dir, &manifest, config.max_bytes)?;
    build.report.orphans = orphans;
    build.report.heavy = heavy;
    if !dry_run && build.stamps_changed {
        build.stamps.retain_existing(&art_dir);
        if let Err(message) = build.stamps.save(&art_dir) {
            build.report.failed.push(Failure {
                path: stamps::FILE_NAME.to_string(),
                message,
            });
        }
    }
    Ok(build.report)
}

/// Reads the manifest of an art folder.
fn read_manifest(path: &Path) -> Result<Manifest, ArtBuildError> {
    let text = read_text(path)?;
    manifest::parse(&text).map_err(|refusal| refused(path, refusal))
}

/// Reads the build config, when the source folder has one. Without a file, the defaults apply.
fn read_config(path: &Path, manifest: &Manifest) -> Result<BuildConfig, ArtBuildError> {
    if !path.is_file() {
        return Ok(BuildConfig::default());
    }
    let text = read_text(path)?;
    config::parse(&text, manifest).map_err(|refusal| refused(path, refusal))
}

/// Reads a whole text file.
fn read_text(path: &Path) -> Result<String, ArtBuildError> {
    fs::read_to_string(path).map_err(|source| ArtBuildError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Wraps a refusal with the file it refers to.
fn refused(path: &Path, refusal: Refusal) -> ArtBuildError {
    ArtBuildError::Refused {
        file: path.to_path_buf(),
        refusal,
    }
}

/// The formats to write: the ones given on the command line, or else the config's. Always in the order
/// of preference, each once.
fn chosen_formats(given: Option<&[OutputFormat]>, config: &BuildConfig) -> Vec<OutputFormat> {
    let mut formats = given.map_or_else(|| config.formats.clone(), <[OutputFormat]>::to_vec);
    formats.sort();
    formats.dedup();
    formats
}

/// The formats that cannot be written here, with the reason, when ffmpeg is missing or lacks an encoder.
fn unavailable_formats(
    formats: &[OutputFormat],
    program: &OsStr,
) -> BTreeMap<OutputFormat, String> {
    let mut reasons = BTreeMap::new();
    let needs_ffmpeg: Vec<OutputFormat> = formats
        .iter()
        .copied()
        .filter(|format| format.needs_ffmpeg())
        .collect();
    if needs_ffmpeg.is_empty() {
        return reasons;
    }
    match ffmpeg::probe(program) {
        Err(reason) => {
            for format in needs_ffmpeg {
                reasons.insert(format, reason.clone());
            }
        }
        Ok(encoders) => {
            for format in needs_ffmpeg {
                let present = match format {
                    OutputFormat::Webp => encoders.libwebp,
                    _ => encoders.libaom_av1,
                };
                if !present {
                    let encoder = if format == OutputFormat::Webp {
                        "libwebp"
                    } else {
                        "libaom-av1"
                    };
                    reasons.insert(format, format!("this ffmpeg has no {encoder} encoder"));
                }
            }
        }
    }
    reasons
}

/// True when `art/<kind>/` has a file for the art name, in any of the four formats.
fn has_any_file(art_dir: &Path, kind: Kind, stem: &str) -> bool {
    ART_EXTENSIONS.iter().any(|extension| {
        art_dir
            .join(kind.as_str())
            .join(format!("{stem}.{extension}"))
            .is_file()
    })
}

/// One source picture, decoded, with what the outputs of its name need.
struct Job {
    /// The kind of the id: the folder of the outputs.
    kind: Kind,
    /// The art name, which is also the output's file name without extension.
    stem: String,
    /// The source's path inside the source folder.
    source_relative: String,
    /// The SHA-256 of the source's bytes.
    source_sha256: String,
    /// The box in logical pixels.
    box_size: [u32; 2],
    /// How the source fills the box.
    fit: Fit,
    /// The focus of a cover crop.
    focus: [f64; 2],
}

impl Job {
    /// The output size in pixels: the box times the scale.
    fn output_size(&self, scale: u32) -> [u32; 2] {
        [self.box_size[0] * scale, self.box_size[1] * scale]
    }

    /// The effective settings of one output: everything that changes its bytes. The keys are kept in
    /// sorted order, so the hash of the settings does not depend on any serializer option.
    fn settings(
        &self,
        format: OutputFormat,
        config: &BuildConfig,
    ) -> BTreeMap<&'static str, Value> {
        BTreeMap::from([
            ("pipeline", json!(stamps::PIPELINE)),
            ("format", json!(format.extension())),
            ("box", json!(self.box_size)),
            ("scale", json!(config.scale)),
            ("fit", json!(self.fit.as_str())),
            (
                "focus",
                if self.fit == Fit::Cover {
                    json!(self.focus)
                } else {
                    Value::Null
                },
            ),
            ("quality", json!(config.quality.for_format(format))),
        ])
    }
}

/// The state of one run: the planning and, unless it is a check, the writing.
struct Build<'a> {
    art_dir: PathBuf,
    src_dir: PathBuf,
    config: &'a BuildConfig,
    formats: &'a [OutputFormat],
    unavailable: BTreeMap<OutputFormat, String>,
    program: OsString,
    force: bool,
    dry_run: bool,
    stamps: Stamps,
    stamps_changed: bool,
    report: ArtReport,
}

impl Build<'_> {
    /// Plans and writes the pictures of one entry: its base picture, then each variant.
    fn entry(&mut self, entry: &Entry) {
        let overrides = self.config.ids.get(&entry.id).cloned().unwrap_or_default();
        for (variant, stem) in entry.stems() {
            let explicit = if variant.is_none() {
                overrides.source.as_deref()
            } else {
                None
            };
            self.name(entry, &overrides, variant, stem, explicit);
        }
    }

    /// Plans and writes the outputs of one art name.
    fn name(
        &mut self,
        entry: &Entry,
        overrides: &IdOverride,
        variant: Option<String>,
        stem: String,
        explicit: Option<&str>,
    ) {
        let Some(found) = source::find(&self.src_dir, entry.kind, &stem, explicit) else {
            let has_file = has_any_file(&self.art_dir, entry.kind, &stem);
            self.report.missing.push(MissingSource {
                id: entry.id.clone(),
                kind: entry.kind.as_str().to_string(),
                name: stem,
                variant,
                has_file,
            });
            return;
        };
        let bytes = match fs::read(&found.path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.failed(found.relative, format!("cannot read the file: {error}"));
                return;
            }
        };
        let picture = match source::decode(&bytes) {
            Ok(picture) => picture,
            Err(message) => {
                self.failed(found.relative, message);
                return;
            }
        };
        let job = Job {
            kind: entry.kind,
            stem,
            source_relative: found.relative,
            source_sha256: stamps::sha256_hex(&bytes),
            box_size: entry.size,
            fit: overrides.fit.unwrap_or(Fit::Cover),
            focus: overrides
                .focus
                .unwrap_or_else(|| fit::default_focus(entry.kind)),
        };
        self.check_aspect(&job, picture.dimensions());
        let mut rendered: Option<RgbaImage> = None;
        let formats = self.formats;
        for &format in formats {
            self.output(&job, format, &picture, &mut rendered);
        }
    }

    /// Plans and, unless it is a check, writes one output of a name.
    fn output(
        &mut self,
        job: &Job,
        format: OutputFormat,
        picture: &RgbaImage,
        rendered: &mut Option<RgbaImage>,
    ) {
        let relative = format!("{}/{}.{}", job.kind.as_str(), job.stem, format.extension());
        let target = self.art_dir.join(&relative);
        let [width, height] = job.output_size(self.config.scale);
        let settings_sha256 = stamps::settings_sha256(&job.settings(format, self.config));
        let mut record = OutputRecord {
            path: relative.clone(),
            format: format.extension().to_string(),
            source: job.source_relative.clone(),
            width,
            height,
            bytes: None,
        };
        if !self.force
            && target.is_file()
            && self
                .stamps
                .matches(&relative, &job.source_sha256, &settings_sha256)
        {
            record.bytes = file_size(&target);
            self.report.up_to_date.push(record);
            return;
        }
        if let Some(reason) = self.unavailable.get(&format) {
            self.skip(relative, format, reason.clone());
            return;
        }
        if width > MAX_OUTPUT_SIDE || height > MAX_OUTPUT_SIDE {
            let reason = format!("the output would be {width}x{height} pixels; the largest side is {MAX_OUTPUT_SIDE}");
            self.skip(relative, format, reason);
            return;
        }
        if self.dry_run {
            self.report.built.push(record);
            return;
        }
        let out =
            rendered.get_or_insert_with(|| fit::render(picture, job.fit, job.focus, width, height));
        match encode::write(format, out, &target, self.config.quality, &self.program) {
            Ok(()) => {
                record.bytes = file_size(&target);
                self.stamps.set(
                    relative,
                    Stamp {
                        source: job.source_relative.clone(),
                        source_sha256: job.source_sha256.clone(),
                        settings_sha256,
                    },
                );
                self.stamps_changed = true;
                self.report.built.push(record);
            }
            Err(message) => self.report.failed.push(Failure {
                path: relative,
                message,
            }),
        }
    }

    /// Notes a source-level failure.
    fn failed(&mut self, path: String, message: String) {
        self.report.failed.push(Failure { path, message });
    }

    /// Notes an output that is not written.
    fn skip(&mut self, path: String, format: OutputFormat, reason: String) {
        self.report.skipped.push(SkippedOutput {
            path,
            format: format.extension().to_string(),
            reason,
        });
    }

    /// Warns when the source's proportions differ from the box's by more than 15 percent. A contain fit
    /// shows the whole picture, so it cannot lose content and is not checked.
    fn check_aspect(&mut self, job: &Job, source_size: (u32, u32)) {
        if job.fit == Fit::Contain || !differs_too_much(source_size, job.box_size) {
            return;
        }
        let (source_width, source_height) = source_size;
        let picture = u64::from(source_width) * u64::from(job.box_size[1]);
        let box_side = u64::from(source_height) * u64::from(job.box_size[0]);
        self.report.aspect.push(AspectWarning {
            name: job.stem.clone(),
            source: job.source_relative.clone(),
            source_size: [source_width, source_height],
            box_size: job.box_size,
            fit: job.fit.as_str().to_string(),
            difference_percent: ((picture as f64 / box_side as f64) - 1.0).abs() * 100.0,
        });
    }
}

/// True when a source of `source` pixels has proportions that differ from a box of `box_size` logical
/// pixels by more than [`ASPECT_LIMIT_PERCENT`]. The comparison is exact integer arithmetic: the relative
/// difference is `|sw*bh - sh*bw| / (sh*bw)`, and it must be more than 15 percent.
pub(crate) fn differs_too_much(source: (u32, u32), box_size: [u32; 2]) -> bool {
    let picture = u64::from(source.0) * u64::from(box_size[1]);
    let box_side = u64::from(source.1) * u64::from(box_size[0]);
    picture.abs_diff(box_side) * 100 > ASPECT_LIMIT_PERCENT * box_side
}

/// The size of a file in bytes, when it can be read.
fn file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().map(|meta| meta.len())
}

/// Lists the files of `art/` that the runtime may show (they resolve to an entry), and those that no
/// entry resolves to. The stamps file and the manifest are neither. Also returns the resolving files
/// heavier than `max_bytes`.
fn scan_art(
    art: &Path,
    manifest: &Manifest,
    max_bytes: u64,
) -> Result<(Vec<String>, Vec<HeavyOutput>), ArtBuildError> {
    let mut resolvable: HashSet<(Kind, String)> = HashSet::new();
    for entry in &manifest.entries {
        for (_, stem) in entry.stems() {
            resolvable.insert((entry.kind, stem));
        }
    }
    let mut orphans = Vec::new();
    let mut heavy = Vec::new();
    if !art.is_dir() {
        return Ok((orphans, heavy));
    }
    for item in WalkDir::new(art).min_depth(1).sort_by_file_name() {
        let item = item.map_err(|error| ArtBuildError::Io {
            path: art.to_path_buf(),
            source: io::Error::other(error.to_string()),
        })?;
        if !item.file_type().is_file() {
            continue;
        }
        let relative = relative_text(item.path(), art);
        if relative == "manifest.json" || relative == stamps::FILE_NAME {
            continue;
        }
        if is_resolvable(&relative, &resolvable) {
            let bytes = item.metadata().map(|meta| meta.len()).unwrap_or(0);
            if bytes > max_bytes {
                heavy.push(HeavyOutput {
                    path: relative,
                    bytes,
                    limit: max_bytes,
                });
            }
        } else {
            orphans.push(relative);
        }
    }
    Ok((orphans, heavy))
}

/// A path inside `art/`, with `/` separators.
fn relative_text(path: &Path, art: &Path) -> String {
    let relative = path.strip_prefix(art).unwrap_or(path);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// True when a path inside `art/` is `<kind>/<name>.<extension>` for a name that an entry of that kind
/// declares, with one of the four formats.
fn is_resolvable(relative: &str, resolvable: &HashSet<(Kind, String)>) -> bool {
    let mut parts = relative.split('/');
    let (Some(folder), Some(file), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let Some(kind) = Kind::from_name(folder) else {
        return false;
    };
    let Some((stem, extension)) = file.rsplit_once('.') else {
        return false;
    };
    ART_EXTENSIONS.contains(&extension) && resolvable.contains(&(kind, stem.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_resolvable_path_has_a_kind_folder_a_known_name_and_a_format() {
        let mut resolvable = HashSet::new();
        resolvable.insert((Kind::Card, "card.wood".to_string()));
        assert!(is_resolvable("card/card.wood.png", &resolvable));
        assert!(is_resolvable("card/card.wood.svg", &resolvable));
        assert!(!is_resolvable("card/card.wood.tiff", &resolvable));
        assert!(!is_resolvable("icon/card.wood.png", &resolvable));
        assert!(!is_resolvable("card/old/card.wood.png", &resolvable));
        assert!(!is_resolvable("card.wood.png", &resolvable));
    }

    #[test]
    fn the_aspect_check_is_exact_at_the_limit() {
        // A 100x100 box: 115x100 is exactly 15 percent wider, which does not warn; 116x100 does.
        assert!(!differs_too_much((115, 100), [100, 100]));
        assert!(differs_too_much((116, 100), [100, 100]));
        assert!(differs_too_much((84, 100), [100, 100]));
        assert!(!differs_too_much((85, 100), [100, 100]));
        // Proportions are compared, so a picture twice the box in both sides does not warn.
        assert!(!differs_too_much((2000, 1000), [64, 32]));
    }
}
