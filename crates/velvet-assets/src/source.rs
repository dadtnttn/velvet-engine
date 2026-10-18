//! Asset data sources.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::path::AssetPath;

/// Source errors.
#[derive(Debug, Error)]
pub enum SourceError {
    /// Not found.
    #[error("asset not found: {0}")]
    NotFound(String),
    /// I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Reads raw bytes for an asset path.
///
/// Implementations are called from streaming worker threads ([`crate::AssetStreamer`]), so
/// they must be cheap to share (`Send + Sync`) and must not assume the main thread.
pub trait Source: Send + Sync {
    /// Read bytes.
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError>;
    /// Whether the path exists.
    fn exists(&self, path: &AssetPath) -> bool;
    /// List of roots for diagnostics.
    fn roots(&self) -> Vec<PathBuf>;

    /// Size in bytes when known without reading (used to budget memory before a load).
    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        let _ = path;
        None
    }

    /// Read `len` bytes at `offset`. The default reads everything and slices; sources that can
    /// seek (files, VLZ archives) read only what is asked.
    fn read_range(
        &self,
        path: &AssetPath,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, SourceError> {
        let all = self.read(path)?;
        slice_range(&all, path, offset, len).map(<[u8]>::to_vec)
    }
}

fn slice_range<'a>(
    data: &'a [u8],
    path: &AssetPath,
    offset: u64,
    len: usize,
) -> Result<&'a [u8], SourceError> {
    let start = usize::try_from(offset).ok();
    match start.and_then(|s| s.checked_add(len).map(|e| (s, e))) {
        Some((s, e)) if e <= data.len() => Ok(&data[s..e]),
        _ => Err(range_error(path, offset, len, data.len() as u64)),
    }
}

fn range_error(path: &AssetPath, offset: u64, len: usize, size: u64) -> SourceError {
    SourceError::Io(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("range {offset}+{len} outside {path} ({size} bytes)"),
    ))
}

/// Filesystem directory source.
#[derive(Debug, Clone)]
pub struct FileSource {
    root: PathBuf,
}

impl FileSource {
    /// Create from root directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl Source for FileSource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        let full = path.resolve_fs(&self.root);
        if !full.exists() {
            return Err(SourceError::NotFound(path.to_string()));
        }
        Ok(std::fs::read(full)?)
    }

    fn exists(&self, path: &AssetPath) -> bool {
        path.resolve_fs(&self.root).exists()
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![self.root.clone()]
    }

    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        std::fs::metadata(path.resolve_fs(&self.root))
            .ok()
            .filter(|m| m.is_file())
            .map(|m| m.len())
    }

    fn read_range(
        &self,
        path: &AssetPath,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, SourceError> {
        use std::io::{Read, Seek, SeekFrom};
        let full = path.resolve_fs(&self.root);
        let mut file = std::fs::File::open(&full).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => SourceError::NotFound(path.to_string()),
            _ => SourceError::Io(e),
        })?;
        let size = file.metadata()?.len();
        if offset.checked_add(len as u64).is_none_or(|end| end > size) {
            return Err(range_error(path, offset, len, size));
        }
        file.seek(SeekFrom::Start(offset))?;
        let mut out = vec![0u8; len];
        file.read_exact(&mut out)?;
        Ok(out)
    }
}

/// In-memory source for tests and embedded packs.
#[derive(Debug, Default, Clone)]
pub struct MemorySource {
    files: HashMap<String, Vec<u8>>,
}

impl MemorySource {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert bytes.
    pub fn insert(&mut self, path: impl AsRef<str>, data: impl Into<Vec<u8>>) {
        self.files.insert(
            crate::path::VirtualPath::new(path).as_str().to_string(),
            data.into(),
        );
    }
}

impl Source for MemorySource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        self.files
            .get(path.virtual_path.as_str())
            .cloned()
            .ok_or_else(|| SourceError::NotFound(path.to_string()))
    }

    fn exists(&self, path: &AssetPath) -> bool {
        self.files.contains_key(path.virtual_path.as_str())
    }

    fn roots(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("memory://")]
    }

    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        self.files
            .get(path.virtual_path.as_str())
            .map(|d| d.len() as u64)
    }

    fn read_range(
        &self,
        path: &AssetPath,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, SourceError> {
        let data = self
            .files
            .get(path.virtual_path.as_str())
            .ok_or_else(|| SourceError::NotFound(path.to_string()))?;
        slice_range(data, path, offset, len).map(<[u8]>::to_vec)
    }
}

/// Several sources searched in order: the first that has a path serves it. Typical use is a
/// packed archive first and loose files after it, so shipped `.vlz` content and the current
/// loose resources (or mods, or files being edited) work together.
///
/// A source that has the path but fails to read it (corruption, I/O) reports that error; the
/// next source is tried only on "not found", so damage is never hidden by a fallback.
#[derive(Clone, Default)]
pub struct LayeredSource {
    layers: Vec<std::sync::Arc<dyn Source>>,
}

impl std::fmt::Debug for LayeredSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayeredSource")
            .field("roots", &self.roots())
            .finish()
    }
}

impl LayeredSource {
    /// Empty (finds nothing).
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder: add a layer searched after the existing ones.
    pub fn with(mut self, source: std::sync::Arc<dyn Source>) -> Self {
        self.layers.push(source);
        self
    }

    /// Add a layer searched after the existing ones.
    pub fn push(&mut self, source: std::sync::Arc<dyn Source>) {
        self.layers.push(source);
    }

    /// Number of layers.
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// No layers.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    fn first_with(&self, path: &AssetPath) -> Option<&std::sync::Arc<dyn Source>> {
        self.layers.iter().find(|s| s.exists(path))
    }
}

impl Source for LayeredSource {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        match self.first_with(path) {
            Some(s) => s.read(path),
            None => Err(SourceError::NotFound(path.to_string())),
        }
    }

    fn exists(&self, path: &AssetPath) -> bool {
        self.first_with(path).is_some()
    }

    fn roots(&self) -> Vec<PathBuf> {
        self.layers.iter().flat_map(|s| s.roots()).collect()
    }

    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        self.first_with(path).and_then(|s| s.size_hint(path))
    }

    fn read_range(
        &self,
        path: &AssetPath,
        offset: u64,
        len: usize,
    ) -> Result<Vec<u8>, SourceError> {
        match self.first_with(path) {
            Some(s) => s.read_range(path, offset, len),
            None => Err(SourceError::NotFound(path.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn ranges_and_sizes() {
        let mut mem = MemorySource::new();
        mem.insert("a.bin", (0u8..100).collect::<Vec<_>>());
        let p = AssetPath::virtual_path("a.bin");
        assert_eq!(mem.size_hint(&p), Some(100));
        assert_eq!(mem.read_range(&p, 10, 3).unwrap(), vec![10, 11, 12]);
        assert!(mem.read_range(&p, 99, 2).is_err());
        assert!(mem.read_range(&p, u64::MAX, 1).is_err());

        let dir = std::env::temp_dir().join(format!("velvet-src-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("f.bin"), (0u8..50).collect::<Vec<_>>()).unwrap();
        let fs = FileSource::new(&dir);
        let f = AssetPath::virtual_path("f.bin");
        assert_eq!(fs.size_hint(&f), Some(50));
        assert_eq!(fs.read_range(&f, 48, 2).unwrap(), vec![48, 49]);
        assert!(fs.read_range(&f, 49, 2).is_err());
        assert!(matches!(
            fs.read_range(&AssetPath::virtual_path("nope"), 0, 1),
            Err(SourceError::NotFound(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn layers_search_in_order() {
        let mut first = MemorySource::new();
        first.insert("shared.txt", b"packed".to_vec());
        let mut second = MemorySource::new();
        second.insert("shared.txt", b"loose".to_vec());
        second.insert("only_loose.txt", b"mod".to_vec());
        let layered = LayeredSource::new()
            .with(Arc::new(first))
            .with(Arc::new(second));
        assert_eq!(layered.read(&"shared.txt".into()).unwrap(), b"packed");
        assert_eq!(layered.read(&"only_loose.txt".into()).unwrap(), b"mod");
        assert!(matches!(
            layered.read(&"none".into()),
            Err(SourceError::NotFound(_))
        ));
        assert_eq!(layered.size_hint(&"only_loose.txt".into()), Some(3));
    }
}
