//! Assets packed in an Android APK (and its install-time asset packs), read through the NDK's
//! `AAssetManager`.

use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::OnceLock;

use ndk::asset::AssetManager;

use crate::asset_index::{AssetIndex, IndexEntry};
use crate::path::AssetPath;
use crate::reader::AssetReader;
use crate::source::{Source, SourceError};

/// A [`Source`] over the `assets/` folder of the running APK.
///
/// Paths are relative to `assets/` (`scenes/arena.json` reads `assets/scenes/arena.json`).
/// Install-time asset packs of an app bundle are merged into the same asset manager by the
/// system, so their files are read the same way.
///
/// The NDK cannot list sub-directories, so directory queries use the index the packager writes
/// ([`AssetIndex::FILE_NAME`]); without it, [`ApkSource::list_dir`] reports files only.
pub struct ApkSource {
    manager: AssetManager,
    index: OnceLock<Option<AssetIndex>>,
}

impl std::fmt::Debug for ApkSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApkSource")
            .field("indexed_files", &self.index().map(AssetIndex::len))
            .finish()
    }
}

impl ApkSource {
    /// A source over the asset manager of the running application (for example
    /// `AndroidApp::asset_manager()`).
    pub fn new(manager: AssetManager) -> Self {
        Self {
            manager,
            index: OnceLock::new(),
        }
    }

    /// The packaged index, read once on first use (`None` when the package has none).
    pub fn index(&self) -> Option<&AssetIndex> {
        self.index
            .get_or_init(|| {
                let name = CString::new(AssetIndex::FILE_NAME).ok()?;
                let mut asset = self.manager.open(&name)?;
                let mut text = String::new();
                asset.read_to_string(&mut text).ok()?;
                Some(AssetIndex::parse(&text))
            })
            .as_ref()
    }

    /// Whether `dir` is a directory of the package.
    pub fn is_dir(&self, dir: &AssetPath) -> bool {
        let path = dir.virtual_path.as_str();
        match self.index() {
            Some(index) => index.is_dir(path),
            // Without an index: a directory that lists at least one file.
            None => self.list_files(path).is_some_and(|files| !files.is_empty()),
        }
    }

    /// Files and sub-directories directly inside `dir`, or `None` when it is not a directory.
    pub fn list_dir(&self, dir: &AssetPath) -> Option<Vec<IndexEntry>> {
        let path = dir.virtual_path.as_str();
        match self.index() {
            Some(index) => index.list_dir(path),
            None => {
                let files = self.list_files(path)?;
                (!files.is_empty() || path.is_empty()).then(|| {
                    files
                        .into_iter()
                        .map(|name| IndexEntry {
                            name,
                            is_dir: false,
                        })
                        .collect()
                })
            }
        }
    }

    /// The files (not sub-directories) the NDK lists in `dir`.
    fn list_files(&self, dir: &str) -> Option<Vec<String>> {
        let name = CString::new(dir.trim_end_matches('/')).ok()?;
        let entries = self.manager.open_dir(&name)?;
        let mut names: Vec<String> = entries
            .filter_map(|n| n.into_string().ok())
            .filter(|n| n != AssetIndex::FILE_NAME)
            .collect();
        names.sort();
        Some(names)
    }

    fn c_path(path: &AssetPath) -> Result<CString, SourceError> {
        CString::new(path.virtual_path.as_str())
            .map_err(|_| SourceError::NotFound(path.to_string()))
    }

    fn open_asset(&self, path: &AssetPath) -> Result<ndk::asset::Asset, SourceError> {
        let name = Self::c_path(path)?;
        self.manager
            .open(&name)
            .ok_or_else(|| SourceError::NotFound(path.to_string()))
    }

    /// A streaming reader: a file range over the APK when the entry is stored uncompressed
    /// (what the Velvet packager does for media), else the bytes read into memory.
    pub fn open_reader(&self, path: &AssetPath) -> Result<AssetReader, SourceError> {
        let mut asset = self.open_asset(path)?;
        if let Ok(fd) = asset.open_file_descriptor() {
            let file = File::from(fd.fd);
            return Ok(AssetReader::from_file_range(
                file,
                fd.offset as u64,
                fd.size as u64,
            )?);
        }
        let mut bytes = Vec::with_capacity(asset.length());
        asset.read_to_end(&mut bytes)?;
        Ok(AssetReader::from_bytes(bytes))
    }
}

impl Source for ApkSource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        let mut asset = self.open_asset(path)?;
        let mut bytes = Vec::with_capacity(asset.length());
        asset.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    fn exists(&self, path: &AssetPath) -> bool {
        self.open_asset(path).is_ok()
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("apk://assets")]
    }

    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        if let Some(size) = self
            .index()
            .and_then(|i| i.file_size(path.virtual_path.as_str()))
        {
            return Some(size);
        }
        self.open_asset(path).ok().map(|a| a.length() as u64)
    }

    fn read_range(
        &self,
        path: &AssetPath,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, SourceError> {
        let mut asset = self.open_asset(path)?;
        let size = asset.length() as u64;
        if offset.checked_add(len as u64).is_none_or(|end| end > size) {
            return Err(SourceError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("range {offset}+{len} outside {path} ({size} bytes)"),
            )));
        }
        asset.seek(SeekFrom::Start(offset))?;
        let mut out = vec![0u8; len];
        asset.read_exact(&mut out)?;
        Ok(out)
    }

    fn open(&self, path: &AssetPath) -> Result<AssetReader, SourceError> {
        self.open_reader(path)
    }
}
