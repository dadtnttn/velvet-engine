//! The game folder's manifest, `velvet-game.json`.
//!
//! Only `id` and `title` are required; every other field has a default. Unknown keys are
//! rejected, so a misspelled key fails loudly instead of being ignored. The optional `data` object
//! holds hints for the `velvet-js` tools and is not read by the host.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::storage;

/// File name of the manifest, inside the game folder.
pub const MANIFEST_FILE: &str = "velvet-game.json";

/// Largest window width or height, in logical pixels, that a manifest may ask for.
pub const MAX_WINDOW_SIDE: u32 = 16_384;

const MAX_ID_LEN: usize = 64;
const MAX_TITLE_CHARS: usize = 120;
const MAX_VERSION_CHARS: usize = 64;
const MAX_ENTRY_LEN: usize = 256;

/// A game manifest that has been read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Lowercase identifier. It also names the game's data folder.
    pub id: String,
    /// Window title and display name.
    pub title: String,
    /// Game version, which the game reads as `window.velvet.game.version`.
    pub version: String,
    /// Page opened first, relative to the game folder, with forward slashes.
    pub entry: String,
    /// Window size, minimum size and state.
    pub window: WindowConfig,
}

/// Window settings from the manifest's `window` object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowConfig {
    /// Initial width, in logical pixels.
    pub width: u32,
    /// Initial height, in logical pixels.
    pub height: u32,
    /// Smallest width the user can resize to, in logical pixels.
    pub min_width: u32,
    /// Smallest height the user can resize to, in logical pixels.
    pub min_height: u32,
    /// Whether the user can resize the window.
    pub resizable: bool,
    /// Whether the window starts in borderless fullscreen.
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            min_width: 960,
            min_height: 540,
            resizable: true,
            fullscreen: false,
        }
    }
}

/// Why a manifest could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Read {
        /// The file that failed.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The text is not valid JSON, misses `id` or `title`, or has an unknown key.
    #[error("invalid velvet-game.json: {0}")]
    Parse(#[from] serde_json::Error),
    /// The id does not match `^[a-z0-9][a-z0-9_-]{0,63}$`, or it is a Windows device name.
    #[error(
        "invalid id `{0}`: use 1 to 64 lowercase letters, digits, `-` or `_`, starting with a \
         letter or digit, and not a Windows device name"
    )]
    InvalidId(String),
    /// A field has a value that is not allowed.
    #[error("invalid {field}: {reason}")]
    Invalid {
        /// The field name, as written in the manifest.
        field: &'static str,
        /// Why the value was refused.
        reason: String,
    },
}

impl Manifest {
    /// Reads `velvet-game.json` from `folder` and checks it.
    pub fn load(folder: &Path) -> Result<Self, ManifestError> {
        let path = folder.join(MANIFEST_FILE);
        let text = fs::read_to_string(&path).map_err(|source| ManifestError::Read {
            path: path.clone(),
            source,
        })?;
        Self::from_json(&text)
    }

    /// Parses and checks the manifest text.
    ///
    /// A leading byte order mark is ignored, because Windows editors often write one.
    pub fn from_json(text: &str) -> Result<Self, ManifestError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let raw: RawManifest = serde_json::from_str(text)?;
        Self::try_from(raw)
    }
}

/// The manifest as written in the file, before the checks.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    id: String,
    title: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default = "default_entry")]
    entry: String,
    #[serde(default)]
    window: WindowConfig,
    /// Hints for the `velvet-js` tools: where the game keeps its data. The host does not read
    /// them, so any JSON value is accepted here; `velvet-js check` validates the hints.
    #[serde(default)]
    #[allow(dead_code)]
    data: Option<serde_json::Value>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_entry() -> String {
    "index.html".to_string()
}

impl TryFrom<RawManifest> for Manifest {
    type Error = ManifestError;

    fn try_from(raw: RawManifest) -> Result<Self, Self::Error> {
        if !is_valid_id(&raw.id) {
            return Err(ManifestError::InvalidId(raw.id));
        }
        check_text("title", &raw.title, MAX_TITLE_CHARS)?;
        check_text("version", &raw.version, MAX_VERSION_CHARS)?;
        if !is_safe_entry(&raw.entry) {
            return Err(invalid(
                "entry",
                "use a relative path inside the game folder with forward slashes, for example \
                 index.html or game/start.html",
            ));
        }
        raw.window.check()?;
        Ok(Self {
            id: raw.id,
            title: raw.title,
            version: raw.version,
            entry: raw.entry,
            window: raw.window,
        })
    }
}

impl WindowConfig {
    fn check(&self) -> Result<(), ManifestError> {
        let sides = [
            ("window.width", self.width),
            ("window.height", self.height),
            ("window.min_width", self.min_width),
            ("window.min_height", self.min_height),
        ];
        for (field, value) in sides {
            if !(1..=MAX_WINDOW_SIDE).contains(&value) {
                return Err(invalid(
                    field,
                    format!("must be between 1 and {MAX_WINDOW_SIDE}"),
                ));
            }
        }
        if self.min_width > self.width {
            return Err(invalid(
                "window.min_width",
                "must not be larger than window.width",
            ));
        }
        if self.min_height > self.height {
            return Err(invalid(
                "window.min_height",
                "must not be larger than window.height",
            ));
        }
        Ok(())
    }
}

/// Whether `id` matches `^[a-z0-9][a-z0-9_-]{0,63}$` and is not a Windows device name.
///
/// The id names a folder under the user's data folder, so device names such as `con` are refused.
pub fn is_valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() <= MAX_ID_LEN
        && matches!(bytes.first(), Some(byte) if byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
        && !storage::is_reserved_name(id)
}

/// Whether `entry` is a relative path made of plain segments separated by `/`.
///
/// Absolute paths, empty segments, `.` and `..`, backslashes and drive letters are refused.
fn is_safe_entry(entry: &str) -> bool {
    !entry.is_empty()
        && entry.len() <= MAX_ENTRY_LEN
        && entry.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
}

fn check_text(field: &'static str, value: &str, max_chars: usize) -> Result<(), ManifestError> {
    if value.trim().is_empty() {
        return Err(invalid(field, "must not be empty"));
    }
    if value.chars().count() > max_chars {
        return Err(invalid(
            field,
            format!("must be at most {max_chars} characters"),
        ));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(field, "must not contain control characters"));
    }
    Ok(())
}

fn invalid(field: &'static str, reason: impl Into<String>) -> ManifestError {
    ManifestError::Invalid {
        field,
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Manifest, ManifestError> {
        Manifest::from_json(text)
    }

    fn invalid_field(text: &str) -> &'static str {
        match parse(text) {
            Err(ManifestError::Invalid { field, .. }) => field,
            other => panic!("expected an invalid field, got {other:?}"),
        }
    }

    #[test]
    fn minimal_manifest_gets_the_defaults() {
        let manifest = parse(r#"{"id":"demo","title":"Demo"}"#).expect("valid manifest");
        assert_eq!(manifest.id, "demo");
        assert_eq!(manifest.title, "Demo");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.entry, "index.html");
        assert_eq!(manifest.window, WindowConfig::default());
        assert_eq!(manifest.window.width, 1280);
        assert_eq!(manifest.window.height, 720);
        assert_eq!(manifest.window.min_width, 960);
        assert_eq!(manifest.window.min_height, 540);
        assert!(manifest.window.resizable);
        assert!(!manifest.window.fullscreen);
    }

    #[test]
    fn every_field_can_be_set() {
        let text = r#"{
            "id": "my-game_2",
            "title": "My Game",
            "version": "1.2.3",
            "entry": "pages/start.html",
            "window": {
                "width": 1600, "height": 900, "min_width": 800, "min_height": 450,
                "resizable": false, "fullscreen": true
            }
        }"#;
        let manifest = parse(text).expect("valid manifest");
        assert_eq!(manifest.id, "my-game_2");
        assert_eq!(manifest.version, "1.2.3");
        assert_eq!(manifest.entry, "pages/start.html");
        assert_eq!(manifest.window.width, 1600);
        assert_eq!(manifest.window.height, 900);
        assert_eq!(manifest.window.min_width, 800);
        assert_eq!(manifest.window.min_height, 450);
        assert!(!manifest.window.resizable);
        assert!(manifest.window.fullscreen);
    }

    #[test]
    fn a_partial_window_object_keeps_the_other_defaults() {
        let manifest = parse(r#"{"id":"demo","title":"Demo","window":{"width":1000}}"#)
            .expect("valid manifest");
        assert_eq!(manifest.window.width, 1000);
        assert_eq!(manifest.window.height, 720);
        assert!(manifest.window.resizable);
    }

    #[test]
    fn missing_id_or_title_is_a_parse_error() {
        assert!(matches!(
            parse(r#"{"title":"Demo"}"#),
            Err(ManifestError::Parse(_))
        ));
        assert!(matches!(
            parse(r#"{"id":"demo"}"#),
            Err(ManifestError::Parse(_))
        ));
    }

    #[test]
    fn unknown_keys_are_rejected_at_both_levels() {
        assert!(matches!(
            parse(r#"{"id":"demo","title":"Demo","fullScreen":true}"#),
            Err(ManifestError::Parse(_))
        ));
        assert!(matches!(
            parse(r#"{"id":"demo","title":"Demo","window":{"fullscreen_":true}}"#),
            Err(ManifestError::Parse(_))
        ));
    }

    #[test]
    fn the_tools_data_hints_are_accepted_and_ignored() {
        let manifest =
            parse(r#"{"id":"demo","title":"Demo","data":{"roots":["data"],"source":"es"}}"#)
                .expect("data hints are allowed");
        assert_eq!(manifest.id, "demo");
    }

    #[test]
    fn valid_ids_are_accepted() {
        assert!(is_valid_id("a"));
        assert!(is_valid_id("0"));
        assert!(is_valid_id("hello-game"));
        assert!(is_valid_id("a_b-c"));
        assert!(is_valid_id(&"a".repeat(64)));
    }

    #[test]
    fn invalid_ids_are_rejected() {
        let too_long = "a".repeat(65);
        for id in [
            "",
            "Demo",
            "-demo",
            "_demo",
            "de mo",
            "demo!",
            "dem/o",
            "d\u{e9}mo",
            too_long.as_str(),
            "con",
            "nul",
            "com1",
        ] {
            assert!(!is_valid_id(id), "`{id}` should be refused");
            assert!(
                matches!(
                    parse(&format!(r#"{{"id":"{id}","title":"Demo"}}"#)),
                    Err(ManifestError::InvalidId(_))
                ),
                "`{id}` should fail with InvalidId"
            );
        }
    }

    #[test]
    fn a_byte_order_mark_is_ignored() {
        let manifest =
            parse("\u{feff}{\"id\":\"demo\",\"title\":\"Demo\"}").expect("valid manifest");
        assert_eq!(manifest.id, "demo");
    }

    #[test]
    fn window_sizes_must_be_in_range() {
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","window":{"width":0}}"#),
            "window.width"
        );
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","window":{"height":16385}}"#),
            "window.height"
        );
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","window":{"min_width":0}}"#),
            "window.min_width"
        );
    }

    #[test]
    fn the_minimum_size_cannot_exceed_the_size() {
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","window":{"width":800}}"#),
            "window.min_width"
        );
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","window":{"height":500}}"#),
            "window.min_height"
        );
    }

    #[test]
    fn entry_must_be_a_safe_relative_path() {
        for entry in [
            "../x.html",
            "/index.html",
            "a\\b.html",
            "C:/x.html",
            "dir/",
            "a//b.html",
            "./index.html",
            "",
            "sp ace.html",
        ] {
            let text = format!(r#"{{"id":"d","title":"D","entry":"{entry}"}}"#);
            assert_eq!(invalid_field(&text), "entry", "`{entry}` should be refused");
        }
        let manifest = parse(r#"{"id":"d","title":"D","entry":"game/start.html"}"#)
            .expect("a nested entry is fine");
        assert_eq!(manifest.entry, "game/start.html");
    }

    #[test]
    fn title_and_version_are_checked() {
        assert_eq!(invalid_field(r#"{"id":"d","title":"   "}"#), "title");
        assert_eq!(
            invalid_field(&format!(r#"{{"id":"d","title":"{}"}}"#, "t".repeat(121))),
            "title"
        );
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","version":""}"#),
            "version"
        );
        assert_eq!(
            invalid_field(r#"{"id":"d","title":"D","version":"1\n2"}"#),
            "version"
        );
    }

    #[test]
    fn load_reads_the_manifest_from_the_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(
            dir.path().join(MANIFEST_FILE),
            r#"{"id":"from-disk","title":"Disk"}"#,
        )
        .expect("write manifest");
        let manifest = Manifest::load(dir.path()).expect("manifest loads");
        assert_eq!(manifest.id, "from-disk");
        assert_eq!(manifest.title, "Disk");
    }

    #[test]
    fn load_reports_a_folder_without_a_manifest() {
        let dir = tempfile::tempdir().expect("temp dir");
        match Manifest::load(dir.path()) {
            Err(ManifestError::Read { path, .. }) => {
                assert_eq!(path, dir.path().join(MANIFEST_FILE));
            }
            other => panic!("expected a read error, got {other:?}"),
        }
    }
}
