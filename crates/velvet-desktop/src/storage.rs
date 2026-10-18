//! The files a game keeps on this machine: its data folder, save slots and settings.
//!
//! The data folder is `%APPDATA%\Velvet\<game-id>` on Windows,
//! `~/Library/Application Support/Velvet/<game-id>` on macOS, and `$XDG_DATA_HOME/velvet/<game-id>`
//! (or `~/.local/share/velvet/<game-id>`) elsewhere. Saves are `saves/<slot>.json`, and the settings
//! are one JSON object in `settings.json`. Every write goes to a temporary file that then replaces
//! the target, so a crash never leaves a half-written file behind.

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// Largest save, in bytes of UTF-8 text.
pub const MAX_SAVE_BYTES: usize = 16 * 1024 * 1024;

/// Largest settings file, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 1024 * 1024;

/// Name of the settings file, inside the data folder.
pub const SETTINGS_FILE: &str = "settings.json";

/// Name of the folder that holds the save slots, inside the data folder.
pub const SAVES_DIR: &str = "saves";

const MAX_SLOT_LEN: usize = 64;
const MAX_KEY_LEN: usize = 128;

/// The operating system families that decide where the data folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Windows: `%APPDATA%`.
    Windows,
    /// macOS: `~/Library/Application Support`.
    MacOs,
    /// Every other system: `$XDG_DATA_HOME`, or `~/.local/share`.
    Other,
}

/// The platform this build runs on.
pub const fn current_platform() -> Platform {
    if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::MacOs
    } else {
        Platform::Other
    }
}

/// The data folder of a game on this machine, or `None` when the environment has no base folder.
pub fn data_dir_for(game_id: &str) -> Option<PathBuf> {
    data_dir_with(current_platform(), game_id, |name| std::env::var_os(name))
}

/// The data folder of a game, with the platform and the environment passed in.
///
/// `env` looks up an environment variable. Empty values count as unset.
pub fn data_dir_with(
    platform: Platform,
    game_id: &str,
    env: impl Fn(&str) -> Option<OsString>,
) -> Option<PathBuf> {
    let variable = |name: &str| env(name).filter(|value| !value.is_empty());
    match platform {
        Platform::Windows => {
            variable("APPDATA").map(|base| PathBuf::from(base).join("Velvet").join(game_id))
        }
        Platform::MacOs => variable("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Velvet")
                .join(game_id)
        }),
        Platform::Other => variable("XDG_DATA_HOME")
            .map(|base| PathBuf::from(base).join("velvet").join(game_id))
            .or_else(|| {
                variable("HOME").map(|home| {
                    PathBuf::from(home)
                        .join(".local")
                        .join("share")
                        .join("velvet")
                        .join(game_id)
                })
            }),
    }
}

/// Whether `name` is a Windows device name (`CON`, `NUL`, `COM1`...), in any letter case.
///
/// Such names cannot be files or folders on Windows, so they are refused everywhere.
pub fn is_reserved_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && matches!(upper.as_bytes().get(3), Some(b'1'..=b'9')))
}

/// Checks a save slot name: 1 to 64 letters, digits, `-` or `_`, and not a Windows device name.
pub fn validate_slot(slot: &str) -> Result<(), StorageError> {
    let plain = !slot.is_empty()
        && slot.len() <= MAX_SLOT_LEN
        && slot
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
    if plain && !is_reserved_name(slot) {
        Ok(())
    } else {
        Err(StorageError::InvalidSlot(slot.to_string()))
    }
}

/// Checks a settings key: 1 to 128 letters, digits, `.`, `-` or `_`.
pub fn validate_key(key: &str) -> Result<(), StorageError> {
    let plain = !key.is_empty()
        && key.len() <= MAX_KEY_LEN
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'));
    if plain {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_string()))
    }
}

/// Lists the save slots, sorted by name. A missing folder means no saves.
///
/// Files that are not `<valid-slot>.json` (temporary files, notes, folders) are skipped.
pub fn list_saves(data_dir: &Path) -> Result<Vec<String>, StorageError> {
    let dir = data_dir.join(SAVES_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error(&dir, source)),
    };
    let mut slots = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| io_error(&dir, source))?;
        let file_name = entry.file_name();
        let Some(slot) = file_name
            .to_str()
            .and_then(|name| name.strip_suffix(".json"))
        else {
            continue;
        };
        if validate_slot(slot).is_ok() && entry.file_type().is_ok_and(|kind| kind.is_file()) {
            slots.push(slot.to_string());
        }
    }
    slots.sort();
    Ok(slots)
}

/// Reads a save. `None` means the slot is empty.
pub fn read_save(data_dir: &Path, slot: &str) -> Result<Option<String>, StorageError> {
    validate_slot(slot)?;
    let path = save_path(data_dir, slot);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.len() > MAX_SAVE_BYTES as u64 => Err(StorageError::TooLarge {
            what: "save",
            size: usize::try_from(metadata.len()).unwrap_or(usize::MAX),
            limit: MAX_SAVE_BYTES,
        }),
        Ok(_) => fs::read_to_string(&path)
            .map(Some)
            .map_err(|source| io_error(&path, source)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io_error(&path, source)),
    }
}

/// Writes a save, replacing the slot's previous content.
pub fn write_save(data_dir: &Path, slot: &str, text: &str) -> Result<(), StorageError> {
    validate_slot(slot)?;
    if text.len() > MAX_SAVE_BYTES {
        return Err(StorageError::TooLarge {
            what: "save",
            size: text.len(),
            limit: MAX_SAVE_BYTES,
        });
    }
    atomic_write(&save_path(data_dir, slot), text.as_bytes())
}

/// Deletes a save. Deleting an empty slot is not an error.
pub fn remove_save(data_dir: &Path, slot: &str) -> Result<(), StorageError> {
    validate_slot(slot)?;
    let path = save_path(data_dir, slot);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(&path, source)),
    }
}

/// Reads all settings. A missing file means no settings.
pub fn read_settings(data_dir: &Path) -> Result<Map<String, Value>, StorageError> {
    let path = data_dir.join(SETTINGS_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Map::new()),
        Err(source) => return Err(io_error(&path, source)),
    };
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(StorageError::TooLarge {
            what: "settings",
            size: bytes.len(),
            limit: MAX_SETTINGS_BYTES,
        });
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(Value::Object(settings)) => Ok(settings),
        _ => Err(StorageError::CorruptSettings(path)),
    }
}

/// Reads one setting. A missing key gives `null`.
pub fn get_setting(data_dir: &Path, key: &str) -> Result<Value, StorageError> {
    validate_key(key)?;
    let settings = read_settings(data_dir)?;
    Ok(settings.get(key).cloned().unwrap_or(Value::Null))
}

/// Sets one setting and keeps the others.
///
/// A settings file that is not a JSON object is reported, not overwritten.
pub fn set_setting(data_dir: &Path, key: &str, value: Value) -> Result<(), StorageError> {
    validate_key(key)?;
    let mut settings = read_settings(data_dir)?;
    settings.insert(key.to_string(), value);
    let path = data_dir.join(SETTINGS_FILE);
    let bytes = serde_json::to_vec(&Value::Object(settings))
        .map_err(|error| io_error(&path, io::Error::new(io::ErrorKind::InvalidData, error)))?;
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(StorageError::TooLarge {
            what: "settings",
            size: bytes.len(),
            limit: MAX_SETTINGS_BYTES,
        });
    }
    atomic_write(&path, &bytes)
}

fn save_path(data_dir: &Path, slot: &str) -> PathBuf {
    data_dir.join(SAVES_DIR).join(format!("{slot}.json"))
}

/// Writes `bytes` to a temporary file next to `path`, then renames it over `path`.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io_error(
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "not a file path"),
        ));
    };
    fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    // The process id keeps two running copies of the game from sharing one temporary name.
    let mut temp_name = name.to_os_string();
    temp_name.push(format!(".{}.tmp", std::process::id()));
    let temp = parent.join(temp_name);
    let result = write_then_rename(&temp, path, bytes);
    if result.is_err() {
        // Best effort: the temporary file is useless after a failed write.
        let _ = fs::remove_file(&temp);
    }
    result.map_err(|source| io_error(path, source))
}

fn write_then_rename(temp: &Path, target: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = File::create(temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temp, target)
}

fn io_error(path: &Path, source: io::Error) -> StorageError {
    StorageError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Why a save or settings operation failed.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The save slot name is not allowed.
    #[error(
        "invalid save slot `{0}`: use 1 to 64 letters, digits, `-` or `_`, and not a Windows \
         device name"
    )]
    InvalidSlot(String),
    /// The settings key is not allowed.
    #[error("invalid setting key `{0}`: use 1 to 128 letters, digits, `.`, `-` or `_`")]
    InvalidKey(String),
    /// The content is larger than the limit.
    #[error("the {what} is {size} bytes, more than the limit of {limit} bytes")]
    TooLarge {
        /// What is too large, such as `save`.
        what: &'static str,
        /// The size that was given.
        size: usize,
        /// The largest size allowed.
        limit: usize,
    },
    /// The settings file exists but is not a JSON object.
    #[error("{} is not a JSON object; delete it to reset the settings", .0.display())]
    CorruptSettings(PathBuf),
    /// The file system refused the operation.
    #[error("cannot access {}: {source}", path.display())]
    Io {
        /// The file or folder involved.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

impl StorageError {
    /// A short machine-readable code, which the game receives as `error.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidSlot(_) | Self::InvalidKey(_) => "invalid_name",
            Self::TooLarge { .. } => "too_large",
            Self::CorruptSettings(_) => "corrupt_settings",
            Self::Io { .. } => "io",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_from<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn windows_data_dir_lives_under_appdata() {
        let env = env_from(&[("APPDATA", r"C:\Users\me\AppData\Roaming")]);
        let dir = data_dir_with(Platform::Windows, "demo", env);
        let expected = PathBuf::from(r"C:\Users\me\AppData\Roaming")
            .join("Velvet")
            .join("demo");
        assert_eq!(dir, Some(expected));
    }

    #[test]
    fn windows_without_appdata_has_no_data_dir() {
        let env = env_from(&[]);
        assert_eq!(data_dir_with(Platform::Windows, "demo", env), None);
    }

    #[test]
    fn linux_prefers_xdg_data_home() {
        let env = env_from(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/me")]);
        let dir = data_dir_with(Platform::Other, "demo", env);
        assert_eq!(dir, Some(PathBuf::from("/xdg").join("velvet").join("demo")));
    }

    #[test]
    fn linux_falls_back_to_the_home_folder() {
        let env = env_from(&[("HOME", "/home/me")]);
        let dir = data_dir_with(Platform::Other, "demo", env);
        let expected = PathBuf::from("/home/me")
            .join(".local")
            .join("share")
            .join("velvet")
            .join("demo");
        assert_eq!(dir, Some(expected));
    }

    #[test]
    fn an_empty_xdg_data_home_is_ignored() {
        let env = env_from(&[("XDG_DATA_HOME", ""), ("HOME", "/home/me")]);
        let dir = data_dir_with(Platform::Other, "demo", env);
        assert!(dir.is_some_and(|path| path.ends_with(Path::new("share/velvet/demo"))));
    }

    #[test]
    fn linux_without_any_base_has_no_data_dir() {
        let env = env_from(&[]);
        assert_eq!(data_dir_with(Platform::Other, "demo", env), None);
    }

    #[test]
    fn macos_data_dir_lives_under_application_support() {
        let env = env_from(&[("HOME", "/Users/me")]);
        let dir = data_dir_with(Platform::MacOs, "demo", env);
        let expected = PathBuf::from("/Users/me")
            .join("Library")
            .join("Application Support")
            .join("Velvet")
            .join("demo");
        assert_eq!(dir, Some(expected));
    }

    #[test]
    fn device_names_are_reserved_in_any_case() {
        for name in ["CON", "con", "Nul", "PRN", "aux", "COM1", "lpt9", "com5"] {
            assert!(is_reserved_name(name), "`{name}` is a device name");
        }
        for name in ["COM0", "COM10", "CONSOLE", "com", "demo", "nul-game", "lpt"] {
            assert!(!is_reserved_name(name), "`{name}` is an ordinary name");
        }
    }

    #[test]
    fn slot_names_are_checked() {
        let longest = "s".repeat(64);
        for slot in ["slot-1", "Save_01", "a", longest.as_str()] {
            assert!(validate_slot(slot).is_ok(), "`{slot}` should be accepted");
        }
        let too_long = "s".repeat(65);
        for slot in [
            "",
            "../x",
            "a/b",
            "a\\b",
            "a.b",
            "\u{e9}",
            too_long.as_str(),
            "CON",
            "com1",
        ] {
            assert!(
                matches!(validate_slot(slot), Err(StorageError::InvalidSlot(_))),
                "`{slot}` should be refused"
            );
        }
    }

    #[test]
    fn setting_keys_are_checked() {
        for key in ["audio.volume", "ui-scale_2", "a"] {
            assert!(validate_key(key).is_ok(), "`{key}` should be accepted");
        }
        let too_long = "k".repeat(129);
        for key in ["", "a b", "../x", "a/b", "\u{e9}", too_long.as_str()] {
            assert!(
                matches!(validate_key(key), Err(StorageError::InvalidKey(_))),
                "`{key}` should be refused"
            );
        }
    }

    #[test]
    fn a_save_round_trips_and_can_be_replaced() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert_eq!(read_save(dir.path(), "slot1").expect("read"), None);
        write_save(dir.path(), "slot1", "first").expect("write");
        assert_eq!(
            read_save(dir.path(), "slot1").expect("read"),
            Some("first".to_string())
        );
        write_save(dir.path(), "slot1", "second").expect("overwrite");
        assert_eq!(
            read_save(dir.path(), "slot1").expect("read"),
            Some("second".to_string())
        );
    }

    #[test]
    fn a_write_leaves_no_temporary_file_behind() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_save(dir.path(), "slot1", "one").expect("write");
        write_save(dir.path(), "slot1", "two").expect("write again");
        let names: Vec<String> = fs::read_dir(dir.path().join(SAVES_DIR))
            .expect("saves folder")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["slot1.json".to_string()]);
    }

    #[test]
    fn listing_returns_sorted_valid_slots_only() {
        let dir = tempfile::tempdir().expect("temp dir");
        for slot in ["beta", "alpha", "gamma"] {
            write_save(dir.path(), slot, "{}").expect("write");
        }
        let saves = dir.path().join(SAVES_DIR);
        fs::write(saves.join("notes.txt"), "not a save").expect("write note");
        fs::write(saves.join("bad name.json"), "{}").expect("write invalid slot");
        fs::write(saves.join("beta.json.123.tmp"), "half").expect("write temp");
        fs::create_dir(saves.join("folder.json")).expect("make folder");
        assert_eq!(
            list_saves(dir.path()).expect("list"),
            vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
        );
    }

    #[test]
    fn listing_without_a_saves_folder_is_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert_eq!(list_saves(dir.path()).expect("list"), Vec::<String>::new());
    }

    #[test]
    fn removing_a_save_is_idempotent() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_save(dir.path(), "slot1", "data").expect("write");
        remove_save(dir.path(), "slot1").expect("remove");
        assert_eq!(read_save(dir.path(), "slot1").expect("read"), None);
        remove_save(dir.path(), "slot1").expect("removing again is fine");
    }

    #[test]
    fn a_save_over_the_limit_is_refused_without_writing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let text = "x".repeat(MAX_SAVE_BYTES + 1);
        assert!(matches!(
            write_save(dir.path(), "big", &text),
            Err(StorageError::TooLarge { what: "save", .. })
        ));
        assert_eq!(read_save(dir.path(), "big").expect("read"), None);
    }

    #[test]
    fn settings_keep_the_other_keys() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert_eq!(
            get_setting(dir.path(), "audio.volume").expect("get"),
            Value::Null
        );
        set_setting(dir.path(), "audio.volume", Value::from(0.5)).expect("set");
        set_setting(dir.path(), "language", Value::from("es")).expect("set");
        assert_eq!(
            get_setting(dir.path(), "audio.volume").expect("get"),
            Value::from(0.5)
        );
        assert_eq!(
            get_setting(dir.path(), "language").expect("get"),
            Value::from("es")
        );
        set_setting(dir.path(), "language", Value::Null).expect("clear");
        assert_eq!(
            get_setting(dir.path(), "language").expect("get"),
            Value::Null
        );
    }

    #[test]
    fn a_settings_file_that_is_not_an_object_is_reported_and_kept() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(SETTINGS_FILE);
        fs::write(&path, "[1, 2]").expect("write");
        assert!(matches!(
            get_setting(dir.path(), "a"),
            Err(StorageError::CorruptSettings(_))
        ));
        assert!(matches!(
            set_setting(dir.path(), "a", Value::from(1)),
            Err(StorageError::CorruptSettings(_))
        ));
        assert_eq!(fs::read_to_string(&path).expect("read"), "[1, 2]");
    }

    #[test]
    fn settings_that_are_not_json_are_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join(SETTINGS_FILE), "{oops").expect("write");
        assert!(matches!(
            read_settings(dir.path()),
            Err(StorageError::CorruptSettings(_))
        ));
    }

    #[test]
    fn settings_over_the_limit_are_refused_without_writing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let value = Value::from("x".repeat(MAX_SETTINGS_BYTES));
        assert!(matches!(
            set_setting(dir.path(), "big", value),
            Err(StorageError::TooLarge {
                what: "settings",
                ..
            })
        ));
        assert!(!dir.path().join(SETTINGS_FILE).exists());
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(StorageError::InvalidSlot("x".into()).code(), "invalid_name");
        assert_eq!(StorageError::InvalidKey("x".into()).code(), "invalid_name");
        assert_eq!(
            StorageError::CorruptSettings(PathBuf::from("s")).code(),
            "corrupt_settings"
        );
    }
}
