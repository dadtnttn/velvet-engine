//! Files: the game's read-only packaged files, and the writable folders for saves and caches.
//!
//! **Packaged files** ([`assets`]) are what ships with the game. On desktop they are a folder
//! (set with [`set_assets_root`]; else `VELVET_ASSETS_DIR`, else `assets/` beside the
//! executable, else `assets/` in the working directory). On Android they are the APK's
//! `assets/` (and install-time asset packs), which `velvet android` fills from the same folders.
//! Paths are relative and use `/`: `assets().read("scenes/arena.json")` works on both, where
//! `std::fs::read("assets/scenes/arena.json")` works on desktop only.
//!
//! **Writable folders** ([`data_dir`], [`cache_dir`], [`external_data_dir`]) are per user and
//! per game; on Android they are the app's private storage, removed with the app.

use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use velvet_assets::{AssetPath, AssetReader, IndexEntry, Source, SourceError};

static ASSETS: OnceLock<GameAssets> = OnceLock::new();
static DESKTOP_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Sets the desktop folder of the packaged files (the folder whose contents become the APK's
/// `assets/`). Call it before the first [`assets`]; returns false (and changes nothing) when the
/// files were already opened. Ignored on Android.
pub fn set_assets_root(dir: impl Into<PathBuf>) -> bool {
    if ASSETS.get().is_some() {
        return false;
    }
    *DESKTOP_ROOT.lock().unwrap_or_else(|e| e.into_inner()) = Some(dir.into());
    true
}

/// The game's packaged, read-only files.
pub fn assets() -> &'static GameAssets {
    ASSETS.get_or_init(GameAssets::for_platform)
}

/// Read-only access to the files a game ships with, the same on every platform.
pub struct GameAssets {
    source: Arc<dyn Source>,
    backend: Backend,
}

enum Backend {
    Directory(PathBuf),
    #[cfg(target_os = "android")]
    Apk(Arc<velvet_assets::ApkSource>),
}

impl std::fmt::Debug for GameAssets {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameAssets")
            .field("root", &self.describe())
            .finish()
    }
}

impl GameAssets {
    /// Packaged files in a folder (what desktop builds use).
    pub fn directory(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            source: Arc::new(velvet_assets::FileSource::new(&root)),
            backend: Backend::Directory(root),
        }
    }

    /// The running APK's assets.
    #[cfg(target_os = "android")]
    pub fn apk(manager: ndk::asset::AssetManager) -> Self {
        let apk = Arc::new(velvet_assets::ApkSource::new(manager));
        Self {
            source: apk.clone(),
            backend: Backend::Apk(apk),
        }
    }

    fn for_platform() -> Self {
        #[cfg(target_os = "android")]
        {
            match crate::android::app() {
                Some(app) => Self::apk(app.asset_manager()),
                None => {
                    eprintln!(
                        "[velvet-platform] assets opened before android_main: no packaged files"
                    );
                    Self::directory(PathBuf::new())
                }
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            Self::directory(desktop_root())
        }
    }

    /// Where the files come from, for logs.
    pub fn describe(&self) -> String {
        match &self.backend {
            Backend::Directory(root) => root.display().to_string(),
            #[cfg(target_os = "android")]
            Backend::Apk(_) => "apk://assets".into(),
        }
    }

    /// The files as a velvet-assets [`Source`] (for `Assets`, `AssetStreamer`, layering with
    /// other sources).
    pub fn source(&self) -> Arc<dyn Source> {
        self.source.clone()
    }

    /// The whole file.
    pub fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        self.source.read(&asset_path(path)).map_err(to_io)
    }

    /// The whole file as UTF-8 text.
    pub fn read_to_string(&self, path: &str) -> io::Result<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{path}: {e}")))
    }

    /// A seekable reader that streams the file (music, video) instead of loading it whole,
    /// when the package stores it uncompressed (`velvet android` does for media).
    pub fn open(&self, path: &str) -> io::Result<AssetReader> {
        self.source.open(&asset_path(path)).map_err(to_io)
    }

    /// Whether the path is a packaged file or directory.
    pub fn exists(&self, path: &str) -> bool {
        self.is_file(path) || self.is_dir(path)
    }

    /// Whether the path is a packaged file.
    pub fn is_file(&self, path: &str) -> bool {
        match &self.backend {
            Backend::Directory(root) => asset_path(path).resolve_fs(root).is_file(),
            #[cfg(target_os = "android")]
            Backend::Apk(apk) => apk.exists(&asset_path(path)),
        }
    }

    /// Whether the path is a packaged directory.
    pub fn is_dir(&self, path: &str) -> bool {
        match &self.backend {
            Backend::Directory(root) => asset_path(path).resolve_fs(root).is_dir(),
            #[cfg(target_os = "android")]
            Backend::Apk(apk) => apk.is_dir(&asset_path(path)),
        }
    }

    /// The size of a file in bytes.
    pub fn len(&self, path: &str) -> Option<u64> {
        self.source.size_hint(&asset_path(path))
    }

    /// The files and directories directly inside `dir`, sorted by name.
    pub fn list_dir(&self, dir: &str) -> io::Result<Vec<IndexEntry>> {
        match &self.backend {
            Backend::Directory(root) => {
                let mut out = Vec::new();
                for entry in std::fs::read_dir(asset_path(dir).resolve_fs(root))? {
                    let entry = entry?;
                    out.push(IndexEntry {
                        name: entry.file_name().to_string_lossy().into_owned(),
                        is_dir: entry.file_type()?.is_dir(),
                    });
                }
                out.sort();
                Ok(out)
            }
            #[cfg(target_os = "android")]
            Backend::Apk(apk) => apk.list_dir(&asset_path(dir)).ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("no directory {dir}"))
            }),
        }
    }
}

fn asset_path(path: &str) -> AssetPath {
    AssetPath::virtual_path(path)
}

fn to_io(error: SourceError) -> io::Error {
    match error {
        SourceError::NotFound(path) => io::Error::new(io::ErrorKind::NotFound, path),
        SourceError::Io(e) => e,
    }
}

#[cfg(not(target_os = "android"))]
fn desktop_root() -> PathBuf {
    if let Some(root) = DESKTOP_ROOT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        return root;
    }
    if let Some(dir) = std::env::var_os("VELVET_ASSETS_DIR") {
        return PathBuf::from(dir);
    }
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("assets")))
        .filter(|d| d.is_dir());
    beside.unwrap_or_else(|| PathBuf::from("assets"))
}

/// The writable folder for the game's saves and settings, made if missing:
///
/// - Android: the app's internal storage (`/data/user/0/<package>/files`; private, backed up
///   with the app's data, removed with the app). `app` is not used: the folder is per app.
/// - Windows: `%APPDATA%\<app>`; macOS: `~/Library/Application Support/<app>`;
///   Linux: `$XDG_DATA_HOME/<app>` or `~/.local/share/<app>`.
///
/// `VELVET_DATA_DIR` replaces the desktop folder (tests, portable builds). `app` may hold
/// sub-folders (`"Studio/Game"`) but no `..` or absolute path.
pub fn data_dir(app: &str) -> io::Result<PathBuf> {
    let dir = platform_dir(app, Kind::Data)?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// The folder for caches the game can rebuild (the system may clear it on Android), made if
/// missing: Android `/data/user/0/<package>/cache`, Windows `%LOCALAPPDATA%\<app>\cache`, macOS
/// `~/Library/Caches/<app>`, Linux `$XDG_CACHE_HOME/<app>` or `~/.cache/<app>`.
pub fn cache_dir(app: &str) -> io::Result<PathBuf> {
    let dir = platform_dir(app, Kind::Cache)?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// A folder for large downloaded content, made if missing: on Android the app-specific
/// external storage (`/storage/emulated/0/Android/data/<package>/files`; no permission needed,
/// removed with the app, readable by the user over USB), elsewhere the same as [`data_dir`].
pub fn external_data_dir(app: &str) -> io::Result<PathBuf> {
    let dir = platform_dir(app, Kind::External)?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Clone, Copy)]
enum Kind {
    Data,
    Cache,
    External,
}

fn checked_app(app: &str) -> io::Result<&Path> {
    let path = Path::new(app);
    let fine = !app.is_empty() && path.components().all(|c| matches!(c, Component::Normal(_)));
    if fine {
        Ok(path)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not a relative folder name: {app:?}"),
        ))
    }
}

#[cfg(target_os = "android")]
fn platform_dir(app: &str, kind: Kind) -> io::Result<PathBuf> {
    checked_app(app)?;
    let missing = || io::Error::new(io::ErrorKind::NotFound, "no Android activity yet");
    let android = crate::android::app().ok_or_else(missing)?;
    let internal = android.internal_data_path().ok_or_else(missing)?;
    Ok(match kind {
        Kind::Data => internal,
        Kind::Cache => internal
            .parent()
            .map(|p| p.join("cache"))
            .unwrap_or_else(|| internal.join("cache")),
        Kind::External => android.external_data_path().unwrap_or(internal),
    })
}

#[cfg(not(target_os = "android"))]
fn platform_dir(app: &str, kind: Kind) -> io::Result<PathBuf> {
    let app = checked_app(app)?;
    if let Some(dir) = std::env::var_os("VELVET_DATA_DIR") {
        let dir = PathBuf::from(dir);
        return Ok(match kind {
            Kind::Cache => dir.join("cache"),
            Kind::Data | Kind::External => dir,
        });
    }
    let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let home = env("HOME").or_else(|| env("USERPROFILE"));
    let no_home = || io::Error::new(io::ErrorKind::NotFound, "no home folder in the environment");
    let base = if cfg!(windows) {
        match kind {
            Kind::Cache => env("LOCALAPPDATA").map(|d| d.join(app).join("cache")),
            Kind::Data | Kind::External => env("APPDATA").map(|d| d.join(app)),
        }
    } else if cfg!(target_os = "macos") {
        home.map(|h| match kind {
            Kind::Cache => h.join("Library/Caches").join(app),
            Kind::Data | Kind::External => h.join("Library/Application Support").join(app),
        })
    } else {
        match kind {
            Kind::Cache => env("XDG_CACHE_HOME")
                .or_else(|| home.map(|h| h.join(".cache")))
                .map(|d| d.join(app)),
            Kind::Data | Kind::External => env("XDG_DATA_HOME")
                .or_else(|| home.map(|h| h.join(".local/share")))
                .map(|d| d.join(app)),
        }
    };
    base.ok_or_else(no_home)
}

/// Writes a file so that a crash or a killed process never leaves it half-written: the bytes go
/// to a temporary file beside it, which is flushed to disk and then renamed over the target.
/// Use it for saves: Android may kill a backgrounded game at any moment.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    let mut temp_name = name.to_os_string();
    temp_name.push(format!(".{}.tmp", std::process::id()));
    let temp = parent.join(temp_name);
    {
        let mut file = std::fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_of_packaged_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("scenes/sub")).unwrap();
        std::fs::write(dir.path().join("scenes/arena.json"), b"{}").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"hello").unwrap();
        let files = GameAssets::directory(dir.path());
        assert_eq!(files.read("scenes/arena.json").unwrap(), b"{}");
        assert_eq!(files.read_to_string("./readme.txt").unwrap(), "hello");
        assert!(files.is_dir("scenes"));
        assert!(files.is_file("scenes/arena.json"));
        assert!(files.exists("scenes/sub"));
        assert!(!files.exists("nothing.bin"));
        assert_eq!(files.len("readme.txt"), Some(5));
        let listed = files.list_dir("scenes").unwrap();
        assert_eq!(
            listed,
            vec![
                IndexEntry {
                    name: "arena.json".into(),
                    is_dir: false
                },
                IndexEntry {
                    name: "sub".into(),
                    is_dir: true
                }
            ]
        );
        assert_eq!(
            files.read("missing.txt").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        let mut reader = files.open("readme.txt").unwrap();
        let mut text = String::new();
        std::io::Read::read_to_string(&mut reader, &mut text).unwrap();
        assert_eq!(text, "hello");
    }

    #[test]
    fn app_folder_names_stay_relative() {
        assert!(checked_app("Studio/Game").is_ok());
        assert!(checked_app("game").is_ok());
        assert!(checked_app("").is_err());
        assert!(checked_app("../escape").is_err());
        assert!(checked_app("/abs").is_err());
    }

    #[test]
    fn atomic_writes_replace_whole_files() {
        let dir = tempfile::tempdir().unwrap();
        let save = dir.path().join("saves/slot1.json");
        write_atomic(&save, b"first").unwrap();
        write_atomic(&save, b"second").unwrap();
        assert_eq!(std::fs::read(&save).unwrap(), b"second");
        let leftovers: Vec<_> = std::fs::read_dir(save.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers.len(), 1);
    }
}
