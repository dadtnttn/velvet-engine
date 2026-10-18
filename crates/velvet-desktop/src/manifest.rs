//! The game folder's manifest, `velvet-game.json`.
//!
//! Only `id` and `title` are required; every other field has a default. Unknown keys are
//! rejected, so a misspelled key fails loudly instead of being ignored.
//!
//! The optional `icon` is a PNG in the game folder, which the window shows. The optional `release`
//! object and `data` object are hints: `release` for the packaging step (`velvet build`), `data` for
//! the `velvet-js` tools. The host accepts both and does not read them, so a bad hint never stops a
//! game from starting.

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
/// Longest path in the manifest, in bytes: the entry page, the icon, the included files and the
/// exclude patterns.
const MAX_PATH_LEN: usize = 256;

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
    /// The window's icon: a PNG path relative to the game folder, with forward slashes, or None.
    /// The release writes the same icon into the program's file on Windows.
    pub icon: Option<String>,
    /// The `release` object as written, or None. The host does not read it: the packaging step
    /// checks it with [`ReleaseHints::from_value`].
    pub release: Option<serde_json::Value>,
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

/// The `release` object: what a release folder holds beside the game. `velvet build --runtime js`
/// reads it; the host does not.
///
/// Every field is optional, and an unknown key is an error, as in the rest of the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReleaseHints {
    /// Patterns of files in the built web folder that the release leaves out, such as `**/*.psd`
    /// or `dev/**`. `*` matches any characters within one folder name, `?` one character, and `**`
    /// as a whole folder name any number of folders. Matching is case-sensitive, and a pattern
    /// matches file paths only.
    pub exclude: Vec<String>,
    /// Files of the game folder that the release copies to its root, beside the game folder, such
    /// as `LICENSE` or `docs/credits.txt`. Each path follows the rules of `entry`.
    pub include: Vec<String>,
    /// Whether the release is also written as a zip archive.
    pub archive: Archive,
}

/// The archive that a release may be written as, beside its folder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Archive {
    /// The release is the folder only.
    #[default]
    None,
    /// The release folder is also written as `<id>-<version>.zip`, beside it.
    Zip,
}

impl ReleaseHints {
    /// Reads a manifest's `release` value, or the defaults when there is none, and checks it.
    ///
    /// # Errors
    ///
    /// [`ManifestError::Invalid`] for an unknown key, a value of the wrong type, a pattern or path
    /// that is not allowed, or an archive name other than `none` and `zip`.
    pub fn from_value(value: Option<&serde_json::Value>) -> Result<Self, ManifestError> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        let hints: Self = serde_json::from_value(value.clone())
            .map_err(|error| invalid("release", error.to_string()))?;
        for pattern in &hints.exclude {
            if !is_safe_pattern(pattern) {
                return Err(invalid(
                    "release.exclude",
                    format!(
                        "\"{pattern}\" must be a relative pattern with forward slashes, such as \
                         **/*.psd or dev/**"
                    ),
                ));
            }
        }
        for path in &hints.include {
            if !is_safe_relative_path(path) {
                return Err(invalid(
                    "release.include",
                    format!(
                        "\"{path}\" must be a relative path inside the game folder with forward \
                         slashes, such as LICENSE"
                    ),
                ));
            }
        }
        Ok(hints)
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
    /// The window's icon, a PNG path with the rules of `entry`. A missing file is not checked
    /// here: the window logs it and starts without an icon.
    #[serde(default)]
    icon: Option<String>,
    /// Hints for the packaging step. The host does not read them, so any JSON value is accepted
    /// here; `velvet build` checks them with [`ReleaseHints::from_value`].
    #[serde(default)]
    release: Option<serde_json::Value>,
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
        if !is_safe_relative_path(&raw.entry) {
            return Err(invalid(
                "entry",
                "use a relative path inside the game folder with forward slashes, for example \
                 index.html or game/start.html",
            ));
        }
        if let Some(icon) = &raw.icon {
            if !is_safe_relative_path(icon) {
                return Err(invalid(
                    "icon",
                    "use a relative path inside the game folder with forward slashes, for example \
                     icon.png or art/icon.png",
                ));
            }
        }
        raw.window.check()?;
        Ok(Self {
            id: raw.id,
            title: raw.title,
            version: raw.version,
            entry: raw.entry,
            window: raw.window,
            icon: raw.icon,
            release: raw.release,
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

/// Whether `path` is a relative path made of plain segments separated by `/`. This is the rule of
/// `entry`, `icon` and `release.include`.
///
/// Absolute paths, empty segments, `.` and `..`, backslashes and drive letters are refused.
fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_LEN
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
}

/// Whether `pattern` is a release pattern: a relative path with forward slashes, whose folder names
/// may hold the glob characters `*` and `?`. Empty or blank text, empty folder names, `.` and `..`,
/// backslashes, colons and control characters are refused.
fn is_safe_pattern(pattern: &str) -> bool {
    !pattern.trim().is_empty()
        && pattern.len() <= MAX_PATH_LEN
        && !pattern
            .chars()
            .any(|c| matches!(c, '\\' | ':') || c.is_control())
        && pattern
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
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

    /// The field that a refused release object names.
    fn release_field(result: Result<ReleaseHints, ManifestError>) -> &'static str {
        match result {
            Err(ManifestError::Invalid { field, .. }) => field,
            other => panic!("expected an invalid field, got {other:?}"),
        }
    }

    /// Reads the release hints of a `release` object written as text.
    fn release_of(text: &str) -> Result<ReleaseHints, ManifestError> {
        let value: serde_json::Value = serde_json::from_str(text).expect("json");
        ReleaseHints::from_value(Some(&value))
    }

    #[test]
    fn the_icon_is_optional_and_follows_the_rules_of_entry() {
        assert_eq!(
            parse(r#"{"id":"d","title":"D"}"#).expect("valid").icon,
            None
        );
        let manifest = parse(r#"{"id":"d","title":"D","icon":"art/icon.png"}"#).expect("valid");
        assert_eq!(manifest.icon.as_deref(), Some("art/icon.png"));
        for icon in [
            "../icon.png",
            "/icon.png",
            "icon two.png",
            "C:/icon.png",
            "",
        ] {
            let text = format!(r#"{{"id":"d","title":"D","icon":"{icon}"}}"#);
            assert_eq!(invalid_field(&text), "icon", "`{icon}` should be refused");
        }
    }

    #[test]
    fn the_host_keeps_the_release_object_and_does_not_read_it() {
        // A bad release object does not stop the game from starting.
        let manifest = parse(r#"{"id":"d","title":"D","release":{"archive":"tar","bogus":1}}"#)
            .expect("the host ignores release");
        assert_eq!(
            manifest.release,
            Some(serde_json::json!({"archive": "tar", "bogus": 1}))
        );
        assert_eq!(
            release_field(ReleaseHints::from_value(manifest.release.as_ref())),
            "release"
        );
    }

    #[test]
    fn release_hints_are_read_strictly() {
        assert_eq!(
            ReleaseHints::from_value(None).expect("no release object"),
            ReleaseHints::default()
        );
        let good = release_of(
            r#"{"exclude":["**/*.psd","dev/**"],"include":["LICENSE"],"archive":"zip"}"#,
        )
        .expect("valid hints");
        assert_eq!(good.exclude, ["**/*.psd", "dev/**"]);
        assert_eq!(good.include, ["LICENSE"]);
        assert_eq!(good.archive, Archive::Zip);
        assert_eq!(
            release_of("{}").expect("an empty object").archive,
            Archive::None
        );
        assert_eq!(release_field(release_of(r#"{"archive":"tar"}"#)), "release");
        assert_eq!(release_field(release_of(r#"{"colour":1}"#)), "release");
        assert_eq!(
            release_field(release_of(r#"{"include":"LICENSE"}"#)),
            "release"
        );
        assert_eq!(
            release_field(release_of(r#"{"exclude":[""]}"#)),
            "release.exclude"
        );
        assert_eq!(
            release_field(release_of(r#"{"exclude":[" "]}"#)),
            "release.exclude"
        );
        assert_eq!(
            release_field(release_of(r#"{"exclude":["dev\\x"]}"#)),
            "release.exclude"
        );
        assert_eq!(
            release_field(release_of(r#"{"exclude":["dev/../x"]}"#)),
            "release.exclude"
        );
        assert_eq!(
            release_field(release_of(r#"{"exclude":["/abs/**"]}"#)),
            "release.exclude"
        );
        assert_eq!(
            release_field(release_of(r#"{"include":["../LICENSE"]}"#)),
            "release.include"
        );
    }
}
