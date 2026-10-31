//! A game's Android settings: `[package.metadata.velvet.android]` in its Cargo.toml.
//!
//! ```toml
//! [package.metadata.velvet.android]
//! package = "com.example.mygame"      # application id (required)
//! label = "My Game"                   # launcher name (default: the crate name)
//! version_code = 3                    # must grow with every upload (default: from the version)
//! # version_name = "1.2.0"            # default: the crate version
//! min_sdk = 26                        # AAudio (cpal/rodio) needs 26
//! target_sdk = 36                     # Google Play's requirement since 2026-08-31
//! abis = ["arm64-v8a", "x86_64"]      # x86_64 for emulators and Chromebooks
//! orientation = "sensorLandscape"
//! assets = ["scenes", { from = "../data", to = "data", exclude = ["**/*.psd"] }]
//! icon = "android/icon.png"           # square PNG, 512 px recommended
//! ```
//!
//! Paths are relative to the crate's folder.

use serde::{Deserialize, Serialize};

/// Highest `versionCode` Google Play accepts.
pub const MAX_VERSION_CODE: u32 = 2_100_000_000;

/// The Android settings of one game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AndroidAppConfig {
    /// Application id, reverse-DNS (`com.studio.game`); it can never change once published.
    pub package: String,
    /// Name under the launcher icon (default: the crate name).
    pub label: Option<String>,
    /// Integer version; every upload to Google Play needs a larger one (default: derived from
    /// the version name, `major * 1_000_000 + minor * 1_000 + patch`).
    pub version_code: Option<u32>,
    /// Version shown to players (default: the crate's version).
    pub version_name: Option<String>,
    /// Oldest Android API level supported (26 = Android 8.0, needed by AAudio).
    pub min_sdk: u32,
    /// API level the app targets (Google Play requires 36 for new uploads from 2026-08-31).
    pub target_sdk: u32,
    /// API level whose `android.jar` the resources are linked against (default: the target, or
    /// the newest installed platform above it).
    pub compile_sdk: Option<u32>,
    /// ABIs to build: `arm64-v8a` (phones), `x86_64` (emulators, Chromebooks), `armeabi-v7a`,
    /// `x86`.
    pub abis: Vec<String>,
    /// The native library's name without `lib`/`.so` (default: the crate's library name).
    pub lib_name: Option<String>,
    /// Folders packed under the APK's `assets/` (and read with `velvet_platform::files::assets`).
    pub assets: Vec<AssetSpec>,
    /// Install-time asset packs of the app bundle (AAB only; a plain APK holds them in its own
    /// `assets/`). Google Play allows up to 1.5 GB per pack and 4 GB of install-time content.
    pub asset_packs: Vec<AssetPackSpec>,
    /// Launcher icon: a square PNG (512 px recommended). A plain icon is drawn without it.
    pub icon: Option<String>,
    /// Background colour of the adaptive icon, `#RRGGBB` (default: the icon's corner colour).
    pub icon_background: Option<String>,
    /// `sensorLandscape`, `landscape`, `sensorPortrait`, `portrait`, `fullSensor`, `user` or
    /// `unspecified`.
    pub orientation: String,
    /// Hide the status bar from the start (the theme); immersive mode at run time hides the
    /// navigation bar too (`velvet_platform::screen::set_immersive`).
    pub fullscreen: bool,
    /// How the window treats display cutouts: `shortEdges` (draw into them on the short edges;
    /// read `velvet_platform::screen::safe_area`), `default`, `never` or `always`.
    pub cutout: String,
    /// Extra permissions (`android.permission.VIBRATE`, ...). A game reading only its own
    /// files and saves needs none.
    pub permissions: Vec<String>,
    /// Extra `<uses-feature>` names, declared not required.
    pub uses_features: Vec<String>,
    /// Cargo features to enable for the Android build.
    pub cargo_features: Vec<String>,
    /// Build without the crate's default features.
    pub no_default_features: bool,
    /// Extensions stored uncompressed (they are compressed already, and stored entries can be
    /// streamed from the APK); default: [`DEFAULT_STORED_EXTENSIONS`].
    pub store_uncompressed: Option<Vec<String>>,
    /// Strip debug information from the packaged libraries (the unstripped ones stay in the
    /// target folder for symbolising crashes). Default true.
    pub strip: bool,
}

impl Default for AndroidAppConfig {
    fn default() -> Self {
        Self {
            package: String::new(),
            label: None,
            version_code: None,
            version_name: None,
            min_sdk: 26,
            target_sdk: 36,
            compile_sdk: None,
            abis: vec!["arm64-v8a".into()],
            lib_name: None,
            assets: Vec::new(),
            asset_packs: Vec::new(),
            icon: None,
            icon_background: None,
            orientation: "sensorLandscape".into(),
            fullscreen: true,
            cutout: "shortEdges".into(),
            permissions: Vec::new(),
            uses_features: Vec::new(),
            cargo_features: Vec::new(),
            no_default_features: false,
            store_uncompressed: None,
            strip: true,
        }
    }
}

/// Extensions stored without compression by default: media that is compressed already.
pub const DEFAULT_STORED_EXTENSIONS: &[&str] = &[
    "ogg", "opus", "mp3", "m4a", "aac", "flac", "wav", "mp4", "m4v", "webm", "mkv", "ivf", "png",
    "jpg", "jpeg", "webp", "avif", "gif", "ktx2", "basis", "zip", "vlz", "zst",
];

/// One folder of packaged assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AssetSpec {
    /// A folder of the crate, packed under its own name (`"scenes"` → `assets/scenes/`).
    Dir(String),
    /// A folder packed under another name.
    Mapped {
        /// Folder (relative to the crate).
        from: String,
        /// Where under `assets/` (`""` for the root; default: the folder's name).
        #[serde(default)]
        to: Option<String>,
        /// Globs (relative to `from`) left out: `*` within a name, `**` across folders.
        #[serde(default)]
        exclude: Vec<String>,
    },
}

impl AssetSpec {
    /// The folder, relative to the crate.
    pub fn from(&self) -> &str {
        match self {
            Self::Dir(d) => d,
            Self::Mapped { from, .. } => from,
        }
    }

    /// Where the folder goes under `assets/` (no leading or trailing `/`).
    pub fn to(&self) -> String {
        let named = || {
            std::path::Path::new(self.from())
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let to = match self {
            Self::Dir(_) => named(),
            Self::Mapped { to: Some(to), .. } => to.clone(),
            Self::Mapped { to: None, .. } => named(),
        };
        to.replace('\\', "/").trim_matches('/').to_string()
    }

    /// The exclusion globs.
    pub fn exclude(&self) -> &[String] {
        match self {
            Self::Dir(_) => &[],
            Self::Mapped { exclude, .. } => exclude,
        }
    }
}

/// An asset pack of the app bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetPackSpec {
    /// Pack name (letters, digits and `_`; starts with a letter).
    pub name: String,
    /// `install-time` (installed with the app, read like any packaged asset). `fast-follow` and
    /// `on-demand` need Google's Play Asset Delivery library, which this pipeline does not ship.
    #[serde(default = "install_time")]
    pub delivery: String,
    /// Its folders, packed under `assets/` like [`AndroidAppConfig::assets`].
    pub assets: Vec<AssetSpec>,
}

fn install_time() -> String {
    "install-time".into()
}

/// Android ABIs and their Rust targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AndroidAbi {
    /// 64-bit ARM: every current phone.
    Arm64,
    /// 32-bit ARM: old phones.
    Armv7,
    /// 64-bit x86: emulators, Chromebooks.
    X86_64,
    /// 32-bit x86: old emulators.
    X86,
}

impl AndroidAbi {
    /// From an ABI name (`arm64-v8a`, ...).
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "arm64-v8a" | "arm64" | "aarch64" => Self::Arm64,
            "armeabi-v7a" | "armv7" => Self::Armv7,
            "x86_64" | "x86-64" => Self::X86_64,
            "x86" | "i686" => Self::X86,
            _ => return None,
        })
    }

    /// The ABI's name in an APK (`lib/<name>/`).
    pub fn name(self) -> &'static str {
        match self {
            Self::Arm64 => "arm64-v8a",
            Self::Armv7 => "armeabi-v7a",
            Self::X86_64 => "x86_64",
            Self::X86 => "x86",
        }
    }

    /// The Rust target triple.
    pub fn rust_target(self) -> &'static str {
        match self {
            Self::Arm64 => "aarch64-linux-android",
            Self::Armv7 => "armv7-linux-androideabi",
            Self::X86_64 => "x86_64-linux-android",
            Self::X86 => "i686-linux-android",
        }
    }

    /// The prefix of the NDK's clang wrappers (`<prefix><api>-clang`).
    pub fn clang_prefix(self) -> &'static str {
        match self {
            Self::Arm64 => "aarch64-linux-android",
            Self::Armv7 => "armv7a-linux-androideabi",
            Self::X86_64 => "x86_64-linux-android",
            Self::X86 => "i686-linux-android",
        }
    }

    /// Whether the ABI is 64-bit (where Google Play requires 16 KB page alignment).
    pub fn is_64_bit(self) -> bool {
        matches!(self, Self::Arm64 | Self::X86_64)
    }
}

/// What is wrong with a configuration.
pub fn validate(config: &AndroidAppConfig) -> Vec<String> {
    let mut problems = Vec::new();
    if !is_package_name(&config.package) {
        problems.push(format!(
            "package `{}` is not an application id like com.studio.game (two or more parts, \
             letters, digits and _, each starting with a letter)",
            config.package
        ));
    }
    if config.min_sdk < 21 {
        problems.push(format!(
            "min_sdk {} is below 21, the NDK's minimum",
            config.min_sdk
        ));
    }
    if config.target_sdk < config.min_sdk {
        problems.push("target_sdk is below min_sdk".into());
    }
    if config.abis.is_empty() {
        problems.push("abis is empty".into());
    }
    for abi in &config.abis {
        if AndroidAbi::parse(abi).is_none() {
            problems.push(format!(
                "unknown ABI `{abi}` (arm64-v8a, armeabi-v7a, x86_64, x86)"
            ));
        }
    }
    const ORIENTATIONS: &[&str] = &[
        "sensorLandscape",
        "landscape",
        "reverseLandscape",
        "userLandscape",
        "sensorPortrait",
        "portrait",
        "reversePortrait",
        "userPortrait",
        "fullSensor",
        "sensor",
        "user",
        "fullUser",
        "unspecified",
        "locked",
    ];
    if !ORIENTATIONS.contains(&config.orientation.as_str()) {
        problems.push(format!("unknown orientation `{}`", config.orientation));
    }
    if !["shortEdges", "default", "never", "always"].contains(&config.cutout.as_str()) {
        problems.push(format!(
            "unknown cutout mode `{}` (shortEdges, default, never, always)",
            config.cutout
        ));
    }
    if let Some(code) = config.version_code {
        if code == 0 || code > MAX_VERSION_CODE {
            problems.push(format!(
                "version_code {code} is outside 1..={MAX_VERSION_CODE}"
            ));
        }
    }
    if let Some(color) = &config.icon_background {
        if parse_hex_color(color).is_none() {
            problems.push(format!("icon_background `{color}` is not #RRGGBB"));
        }
    }
    for pack in &config.asset_packs {
        let valid = pack
            .name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && pack
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            problems.push(format!("asset pack name `{}` is not valid", pack.name));
        }
        if pack.delivery != "install-time" {
            problems.push(format!(
                "asset pack `{}`: only install-time delivery is supported (fast-follow and \
                 on-demand need the Play Asset Delivery library)",
                pack.name
            ));
        }
    }
    problems
}

/// Whether a name is a valid application id.
pub fn is_package_name(name: &str) -> bool {
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|p| {
            p.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// `major * 1_000_000 + minor * 1_000 + patch` of a `major.minor.patch` version (pre-release and
/// build suffixes ignored).
pub fn version_code_from(version: &str) -> Option<u32> {
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u32>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    if minor >= 1000 || patch >= 1000 {
        return None;
    }
    let code = major.checked_mul(1_000_000)? + minor * 1000 + patch;
    (code > 0 && code <= MAX_VERSION_CODE).then_some(code)
}

/// `#RRGGBB` as RGB.
pub fn parse_hex_color(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_metadata_with_defaults() {
        let json = serde_json::json!({
            "package": "com.example.game",
            "label": "Game",
            "abis": ["arm64-v8a", "x86_64"],
            "assets": ["scenes", {"from": "../data", "to": "data", "exclude": ["**/*.psd"]}],
            "asset_packs": [{"name": "media", "assets": [{"from": "../media", "to": ""}]}]
        });
        let config: AndroidAppConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.min_sdk, 26);
        assert_eq!(config.target_sdk, 36);
        assert_eq!(config.assets[0].to(), "scenes");
        assert_eq!(config.assets[1].to(), "data");
        assert_eq!(config.assets[1].exclude(), &["**/*.psd".to_string()]);
        assert_eq!(config.asset_packs[0].delivery, "install-time");
        assert_eq!(config.asset_packs[0].assets[0].to(), "");
        assert!(validate(&config).is_empty(), "{:?}", validate(&config));
        let unknown = serde_json::json!({"package": "a.b", "colour": 3});
        assert!(serde_json::from_value::<AndroidAppConfig>(unknown).is_err());
    }

    #[test]
    fn validation_catches_mistakes() {
        let config = AndroidAppConfig {
            package: "game".into(),
            min_sdk: 19,
            abis: vec!["mips".into()],
            orientation: "sideways".into(),
            version_code: Some(0),
            icon_background: Some("blue".into()),
            ..Default::default()
        };
        assert_eq!(validate(&config).len(), 6);
        assert!(is_package_name("com.studio.game_2"));
        assert!(!is_package_name("com.2studio.game"));
    }

    #[test]
    fn version_codes_from_versions() {
        assert_eq!(version_code_from("0.3.0"), Some(3000));
        assert_eq!(version_code_from("1.2.3"), Some(1_002_003));
        assert_eq!(version_code_from("0.2.2+fix2"), Some(2002));
        assert_eq!(version_code_from("2.0.0-beta.1"), Some(2_000_000));
        assert_eq!(version_code_from("0.0.0"), None);
        assert_eq!(version_code_from("x"), None);
        assert_eq!(parse_hex_color("#1a2B3c"), Some([0x1a, 0x2b, 0x3c]));
        assert_eq!(
            AndroidAbi::parse("arm64-v8a").unwrap().rust_target(),
            "aarch64-linux-android"
        );
        assert_eq!(AndroidAbi::Armv7.clang_prefix(), "armv7a-linux-androideabi");
    }
}
