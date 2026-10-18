//! The stamps file, `art/.build-stamps.json`. For each output it keeps the SHA-256 of the source
//! picture and the SHA-256 of the settings that made the output. An output is up to date when its
//! file exists and both hashes match. Changing a source, an effective setting or the pipeline
//! version makes it stale; `--force` rebuilds everything.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::files;

/// The file name of the stamps, inside `art/`.
pub const FILE_NAME: &str = ".build-stamps.json";

/// The `format` tag of the stamps file.
pub const FORMAT: &str = "velvet-artbuild-stamps";

/// The version of the stamps file.
pub const VERSION: u32 = 1;

/// The version of the picture pipeline: the crop, the resize, the encoders and their settings. Raising it
/// makes every output stale, because the same settings would now give different bytes.
pub const PIPELINE: u32 = 1;

/// What one output was made from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    /// The source's file name inside the source folder, for people reading the file.
    pub source: String,
    /// The SHA-256 of the source file's bytes, in lowercase hex.
    pub source_sha256: String,
    /// The SHA-256 of the effective settings of the output, in lowercase hex.
    pub settings_sha256: String,
}

/// The stamps of the outputs of one art folder, keyed by the output's path inside `art/`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stamps {
    outputs: BTreeMap<String, Stamp>,
}

impl Stamps {
    /// Reads the stamps of an art folder. A missing or unreadable file gives no stamps, so every output
    /// is built again, which is always safe.
    pub fn load(art: &Path) -> Stamps {
        let Ok(text) = std::fs::read_to_string(art.join(FILE_NAME)) else {
            return Stamps::default();
        };
        let Ok(document) = serde_json::from_str::<Value>(&text) else {
            return Stamps::default();
        };
        let envelope_ok = document.get("format").and_then(Value::as_str) == Some(FORMAT)
            && document.get("version").and_then(Value::as_u64) == Some(u64::from(VERSION));
        if !envelope_ok {
            return Stamps::default();
        }
        let outputs = document
            .get("outputs")
            .and_then(|value| serde_json::from_value::<BTreeMap<String, Stamp>>(value.clone()).ok())
            .unwrap_or_default();
        Stamps { outputs }
    }

    /// True when the output at `output` was made from a source with `source_sha256`, with settings
    /// that hash to `settings_sha256`.
    pub fn matches(&self, output: &str, source_sha256: &str, settings_sha256: &str) -> bool {
        self.outputs.get(output).is_some_and(|stamp| {
            stamp.source_sha256 == source_sha256 && stamp.settings_sha256 == settings_sha256
        })
    }

    /// Records that `output` was made with these hashes.
    pub fn set(&mut self, output: String, stamp: Stamp) {
        self.outputs.insert(output, stamp);
    }

    /// Drops the stamps of outputs whose files are gone from `art`, so the file does not grow with
    /// outputs that were deleted.
    pub fn retain_existing(&mut self, art: &Path) {
        self.outputs.retain(|output, _| art.join(output).is_file());
    }

    /// Writes the stamps file into `art`, atomically.
    ///
    /// # Errors
    /// Returns the reason when the file cannot be written.
    pub fn save(&self, art: &Path) -> Result<(), String> {
        let document = serde_json::json!({
            "format": FORMAT,
            "version": VERSION,
            "outputs": self.outputs,
        });
        let text = serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?;
        files::write_atomic(&art.join(FILE_NAME), |temporary| {
            std::fs::write(temporary, text.as_bytes()).map_err(|error| error.to_string())
        })
    }
}

/// The SHA-256 of some bytes, as 64 lowercase hex digits.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The SHA-256 of an output's effective settings, written as JSON. The settings must be a map with
/// its keys in sorted order (a `BTreeMap`), so the same settings always give the same hash.
pub fn settings_sha256(settings: &impl Serialize) -> String {
    let text = serde_json::to_string(settings).unwrap_or_default();
    sha256_hex(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_is_the_standard_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn stamps_survive_a_save_and_a_load_and_only_matching_hashes_count() {
        let folder =
            std::env::temp_dir().join(format!("velvet-artbuild-stamps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("card")).unwrap();
        std::fs::write(folder.join("card").join("card.wood.png"), b"x").unwrap();
        let mut stamps = Stamps::default();
        stamps.set(
            "card/card.wood.png".to_string(),
            Stamp {
                source: "card.wood.png".to_string(),
                source_sha256: "aa".to_string(),
                settings_sha256: "bb".to_string(),
            },
        );
        stamps.set(
            "card/gone.png".to_string(),
            Stamp {
                source: "gone.png".to_string(),
                source_sha256: "aa".to_string(),
                settings_sha256: "bb".to_string(),
            },
        );
        stamps.retain_existing(&folder);
        stamps.save(&folder).unwrap();
        let loaded = Stamps::load(&folder);
        assert_eq!(loaded, stamps);
        assert!(loaded.matches("card/card.wood.png", "aa", "bb"));
        assert!(!loaded.matches("card/card.wood.png", "aa", "cc"));
        assert!(!loaded.matches("card/card.wood.png", "ab", "bb"));
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_damaged_stamps_file_gives_no_stamps() {
        let folder =
            std::env::temp_dir().join(format!("velvet-artbuild-damaged-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(FILE_NAME), b"{ not json").unwrap();
        assert_eq!(Stamps::load(&folder), Stamps::default());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
