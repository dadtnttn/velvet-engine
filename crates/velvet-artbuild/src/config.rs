//! The build config, `art-src/art-build.json`, which is optional. It sets the formats to write, the
//! quality of the lossy formats, the scale, the size limit, and per-id overrides of the fit, the
//! focus and the source picture. The rules are in `docs/formats/ART_BUILD.md`.
//!
//! Every name of this document changes a picture, so a name that the format does not define is
//! refused (`unknown-field`) rather than ignored.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::error::{issue, ErrorCode, Issue, Refusal};
use crate::fit::Fit;
use crate::manifest::{self, Manifest};
use crate::output_format::OutputFormat;

/// The `format` tag of a build config.
pub const FORMAT: &str = "velvet-art-build";

/// The version that this crate reads.
pub const VERSION: u32 = 1;

/// The file name of the build config, inside the source folder.
pub const FILE_NAME: &str = "art-build.json";

/// The AVIF quality when the config does not set one.
pub const DEFAULT_QUALITY_AVIF: u8 = 60;

/// The WebP quality when the config does not set one.
pub const DEFAULT_QUALITY_WEBP: u8 = 85;

/// The size limit of one output, in bytes, when the config does not set one: 1 MiB.
pub const DEFAULT_MAX_BYTES: u64 = 1024 * 1024;

/// The largest scale: output pixels per logical pixel.
pub const MAX_SCALE: u32 = 4;

/// The extensions of source pictures, in the order the lookup tries them.
pub const SOURCE_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "avif"];

/// The quality of the lossy formats, each from 1 to 100. PNG is lossless and has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quality {
    /// The AVIF quality. It is turned into the encoder's CRF: see `ffmpeg::crf_for_quality`.
    pub avif: u8,
    /// The WebP quality, the same scale as the WebP encoder's.
    pub webp: u8,
}

impl Quality {
    /// The quality of a format, or None for PNG, which has none.
    pub fn for_format(self, format: OutputFormat) -> Option<u8> {
        match format {
            OutputFormat::Avif => Some(self.avif),
            OutputFormat::Webp => Some(self.webp),
            OutputFormat::Png => None,
        }
    }
}

impl Default for Quality {
    fn default() -> Self {
        Quality {
            avif: DEFAULT_QUALITY_AVIF,
            webp: DEFAULT_QUALITY_WEBP,
        }
    }
}

/// The overrides of one art id. Each one that is None keeps the default for the id's kind.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IdOverride {
    /// How the source fills the box. Defaults to `cover`.
    pub fit: Option<Fit>,
    /// Which part of the picture a `cover` crop keeps, as `[x, y]` fractions. Defaults to `[0.5, 0]`
    /// for portraits and full bodies, and `[0.5, 0.5]` for the other kinds.
    pub focus: Option<[f64; 2]>,
    /// The file name of the source picture of the base id, such as `lyra-final.png`. Variants keep
    /// the default name.
    pub source: Option<String>,
}

/// A checked build config. Every field has its default when the document leaves it out.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildConfig {
    /// The formats to write. Defaults to all three.
    pub formats: Vec<OutputFormat>,
    /// The quality of the lossy formats.
    pub quality: Quality,
    /// Output pixels per logical pixel, from 1 to [`MAX_SCALE`]. Defaults to 1.
    pub scale: u32,
    /// The size limit of one output, in bytes. Outputs above it are listed in the report.
    pub max_bytes: u64,
    /// The overrides, by art id.
    pub ids: BTreeMap<String, IdOverride>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        BuildConfig {
            formats: OutputFormat::ALL.to_vec(),
            quality: Quality::default(),
            scale: 1,
            max_bytes: DEFAULT_MAX_BYTES,
            ids: BTreeMap::new(),
        }
    }
}

/// Reads a build config. The ids it overrides must be declared by `manifest`.
///
/// # Errors
/// Returns a [`Refusal`] when the text is not JSON, when the envelope is wrong, when a name is not
/// defined, when a value is out of range, or when an override names an id that the manifest does not
/// declare. The refusal lists every problem; its code is the code of the first one.
pub fn parse(text: &str, manifest: &Manifest) -> Result<BuildConfig, Refusal> {
    let value: Value = serde_json::from_str(text).map_err(|error| {
        Refusal::single(
            ErrorCode::Json,
            format!("the text is not valid JSON: {error}"),
            None,
        )
    })?;
    let Some(document) = value.as_object() else {
        return Err(Refusal::single(
            ErrorCode::Structure,
            "the build config is not a JSON object",
            None,
        ));
    };
    if document.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(Refusal::single(
            ErrorCode::Format,
            format!("the format tag is not \"{FORMAT}\""),
            Some("format"),
        ));
    }
    if document.get("version").and_then(Value::as_f64) != Some(f64::from(VERSION)) {
        return Err(Refusal::single(
            ErrorCode::Version,
            format!("the version is not {VERSION}"),
            Some("version"),
        ));
    }

    let mut issues = Vec::new();
    check_fields(
        document,
        &[
            "format",
            "version",
            "formats",
            "quality",
            "scale",
            "max_bytes",
            "ids",
        ],
        None,
        &mut issues,
    );
    let mut config = BuildConfig::default();
    if let Some(value) = document.get("formats") {
        if let Some(formats) = read_formats(value, &mut issues) {
            config.formats = formats;
        }
    }
    if let Some(value) = document.get("quality") {
        if let Some(quality) = read_quality(value, &mut issues) {
            config.quality = quality;
        }
    }
    if let Some(value) = document.get("scale") {
        if let Some(scale) = read_whole(value, 1, u64::from(MAX_SCALE), "scale", &mut issues) {
            config.scale = scale as u32;
        }
    }
    if let Some(value) = document.get("max_bytes") {
        if let Some(bytes) = read_whole(value, 1, 1 << 53, "max_bytes", &mut issues) {
            config.max_bytes = bytes;
        }
    }
    if let Some(value) = document.get("ids") {
        if let Some(ids) = read_ids(value, manifest, &mut issues) {
            config.ids = ids;
        }
    }
    match Refusal::from_issues(issues) {
        Some(refusal) => Err(refusal),
        None => Ok(config),
    }
}

/// True when `name` is a plain source file name: no folder, a name, and one of
/// [`SOURCE_EXTENSIONS`]. Such a name is looked up inside the source folder only.
pub fn valid_source_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\'])
        && name.rsplit_once('.').is_some_and(|(stem, extension)| {
            !stem.is_empty() && SOURCE_EXTENSIONS.contains(&extension)
        })
}

/// Adds an `unknown-field` issue for each key that the object does not define.
fn check_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    parent: Option<&str>,
    issues: &mut Vec<Issue>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            let path = match parent {
                Some(parent) => format!("{parent}.{key}"),
                None => key.clone(),
            };
            issues.push(issue(
                ErrorCode::UnknownField,
                &path,
                format!("\"{key}\" is not a field of the build config"),
            ));
        }
    }
}

/// Reads a whole number from `min` to `max`, both included.
fn read_whole(
    value: &Value,
    min: u64,
    max: u64,
    path: &str,
    issues: &mut Vec<Issue>,
) -> Option<u64> {
    let number = value
        .as_f64()
        .filter(|n| n.fract() == 0.0 && *n >= min as f64 && *n <= max as f64);
    if number.is_none() {
        issues.push(issue(
            ErrorCode::Value,
            path,
            format!("{path} must be a whole number from {min} to {max}"),
        ));
    }
    number.map(|n| n as u64)
}

/// Reads the formats: a non-empty array of distinct names, each `avif`, `webp` or `png`.
fn read_formats(value: &Value, issues: &mut Vec<Issue>) -> Option<Vec<OutputFormat>> {
    let Some(items) = value.as_array() else {
        issues.push(issue(
            ErrorCode::Structure,
            "formats",
            "formats must be an array of names",
        ));
        return None;
    };
    let mut formats: Vec<OutputFormat> = Vec::new();
    let mut valid = true;
    for (index, item) in items.iter().enumerate() {
        let path = format!("formats[{index}]");
        let Some(name) = item.as_str() else {
            issues.push(issue(
                ErrorCode::Structure,
                &path,
                format!("{path} must be a string"),
            ));
            valid = false;
            continue;
        };
        match OutputFormat::from_name(name) {
            None => {
                issues.push(issue(
                    ErrorCode::Value,
                    &path,
                    format!(
                        "{path} \"{name}\" is not a format the build writes; use avif, webp or png"
                    ),
                ));
                valid = false;
            }
            Some(format) if formats.contains(&format) => {
                issues.push(issue(
                    ErrorCode::Duplicate,
                    &path,
                    format!("{path} repeats the format \"{name}\""),
                ));
                valid = false;
            }
            Some(format) => formats.push(format),
        }
    }
    if items.is_empty() {
        issues.push(issue(
            ErrorCode::Value,
            "formats",
            "formats must name at least one format",
        ));
        valid = false;
    }
    valid.then(|| {
        formats.sort();
        formats
    })
}

/// Reads the qualities: an object with `avif` and `webp`, each a whole number from 1 to 100.
fn read_quality(value: &Value, issues: &mut Vec<Issue>) -> Option<Quality> {
    let Some(object) = value.as_object() else {
        issues.push(issue(
            ErrorCode::Structure,
            "quality",
            "quality must be an object",
        ));
        return None;
    };
    check_fields(object, &["avif", "webp"], Some("quality"), issues);
    let mut quality = Quality::default();
    let mut valid = true;
    if let Some(value) = object.get("avif") {
        match read_whole(value, 1, 100, "quality.avif", issues) {
            Some(avif) => quality.avif = avif as u8,
            None => valid = false,
        }
    }
    if let Some(value) = object.get("webp") {
        match read_whole(value, 1, 100, "quality.webp", issues) {
            Some(webp) => quality.webp = webp as u8,
            None => valid = false,
        }
    }
    valid.then_some(quality)
}

/// Reads the per-id overrides. Every key must be an id that the manifest declares.
fn read_ids(
    value: &Value,
    manifest: &Manifest,
    issues: &mut Vec<Issue>,
) -> Option<BTreeMap<String, IdOverride>> {
    let Some(object) = value.as_object() else {
        issues.push(issue(
            ErrorCode::Structure,
            "ids",
            "ids must be an object, keyed by art id",
        ));
        return None;
    };
    let mut ids = BTreeMap::new();
    let mut valid = true;
    for (key, raw) in object {
        let path = format!("ids[\"{key}\"]");
        if !manifest::valid_id(key) || manifest.entry(key).is_none() {
            issues.push(issue(
                ErrorCode::UnknownId,
                &path,
                format!("the manifest declares no art id \"{key}\""),
            ));
            valid = false;
        }
        match read_override(raw, &path, issues) {
            Some(parsed) => {
                ids.insert(key.clone(), parsed);
            }
            None => valid = false,
        }
    }
    valid.then_some(ids)
}

/// Reads the overrides of one id: `fit`, `focus` and `source`.
fn read_override(value: &Value, path: &str, issues: &mut Vec<Issue>) -> Option<IdOverride> {
    let Some(object) = value.as_object() else {
        issues.push(issue(
            ErrorCode::Structure,
            path,
            format!("{path} must be an object"),
        ));
        return None;
    };
    check_fields(object, &["fit", "focus", "source"], Some(path), issues);
    let mut parsed = IdOverride::default();
    let mut valid = true;
    if let Some(raw) = object.get("fit") {
        let fit_path = format!("{path}.fit");
        match raw.as_str().and_then(Fit::from_name) {
            Some(fit) => parsed.fit = Some(fit),
            None => {
                issues.push(issue(
                    ErrorCode::Value,
                    &fit_path,
                    format!("{fit_path} must be cover, contain or stretch"),
                ));
                valid = false;
            }
        }
    }
    if let Some(raw) = object.get("focus") {
        let focus_path = format!("{path}.focus");
        match read_focus(raw) {
            Some(focus) => parsed.focus = Some(focus),
            None => {
                issues.push(issue(
                    ErrorCode::Value,
                    &focus_path,
                    format!("{focus_path} must be two numbers from 0 to 1"),
                ));
                valid = false;
            }
        }
    }
    if let Some(raw) = object.get("source") {
        let source_path = format!("{path}.source");
        match raw.as_str().filter(|name| valid_source_name(name)) {
            Some(name) => parsed.source = Some(name.to_string()),
            None => {
                issues.push(issue(
                    ErrorCode::Value,
                    &source_path,
                    format!("{source_path} must be a file name in art-src with one of the extensions png, jpg, jpeg or avif"),
                ));
                valid = false;
            }
        }
    }
    valid.then_some(parsed)
}

/// Reads a focus: two numbers from 0 to 1.
pub(crate) fn read_focus(value: &Value) -> Option<[f64; 2]> {
    let items = value.as_array()?;
    match items.as_slice() {
        [x, y] => match (x.as_f64(), y.as_f64()) {
            (Some(x), Some(y)) if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) => {
                Some([x, y])
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        manifest::parse(
            r#"{"format":"velvet-art","version":1,"entries":[{"id":"card.wood","kind":"card","size":[180,260]}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn an_empty_document_gives_the_defaults() {
        let config = parse(r#"{"format":"velvet-art-build","version":1}"#, &manifest()).unwrap();
        assert_eq!(config, BuildConfig::default());
        assert_eq!(
            config.formats,
            vec![OutputFormat::Avif, OutputFormat::Webp, OutputFormat::Png]
        );
        assert_eq!(config.max_bytes, 1024 * 1024);
    }

    #[test]
    fn a_misspelt_name_is_refused_as_an_unknown_field() {
        let refusal = parse(
            r#"{"format":"velvet-art-build","version":1,"sclae":2}"#,
            &manifest(),
        )
        .unwrap_err();
        assert_eq!(refusal.code, ErrorCode::UnknownField);
        assert_eq!(refusal.issues[0].path.as_deref(), Some("sclae"));
    }

    #[test]
    fn an_override_for_an_undeclared_id_is_refused() {
        let refusal = parse(
            r#"{"format":"velvet-art-build","version":1,"ids":{"card.gold":{"fit":"contain"}}}"#,
            &manifest(),
        )
        .unwrap_err();
        assert_eq!(refusal.code, ErrorCode::UnknownId);
    }

    #[test]
    fn overrides_and_quality_are_read() {
        let config = parse(
            r#"{"format":"velvet-art-build","version":1,"formats":["png","webp"],"quality":{"webp":70},"scale":2,
                "ids":{"card.wood":{"fit":"contain","focus":[0,1],"source":"wood.jpg"}}}"#,
            &manifest(),
        )
        .unwrap();
        assert_eq!(config.formats, vec![OutputFormat::Webp, OutputFormat::Png]);
        assert_eq!(config.quality.webp, 70);
        assert_eq!(config.quality.avif, DEFAULT_QUALITY_AVIF);
        assert_eq!(config.scale, 2);
        let wood = &config.ids["card.wood"];
        assert_eq!(wood.fit, Some(Fit::Contain));
        assert_eq!(wood.focus, Some([0.0, 1.0]));
        assert_eq!(wood.source.as_deref(), Some("wood.jpg"));
    }

    #[test]
    fn out_of_range_values_are_refused() {
        for body in [
            r#""scale":5"#,
            r#""scale":0"#,
            r#""quality":{"avif":0}"#,
            r#""formats":["svg"]"#,
            r#""formats":[]"#,
            r#""formats":["png","png"]"#,
            r#""ids":{"card.wood":{"focus":[0.5,1.5]}}"#,
            r#""ids":{"card.wood":{"source":"../x.png"}}"#,
            r#""ids":{"card.wood":{"source":"x.gif"}}"#,
        ] {
            let text = format!(r#"{{"format":"velvet-art-build","version":1,{body}}}"#);
            assert!(
                parse(&text, &manifest()).is_err(),
                "{body} should be refused"
            );
        }
    }

    #[test]
    fn source_names_must_be_plain_picture_files() {
        assert!(valid_source_name("lyra.png"));
        assert!(valid_source_name("lyra.happy.jpeg"));
        assert!(!valid_source_name("portrait/lyra.png"));
        assert!(!valid_source_name(".png"));
        assert!(!valid_source_name("lyra"));
        assert!(!valid_source_name("lyra.PNG"));
    }
}
