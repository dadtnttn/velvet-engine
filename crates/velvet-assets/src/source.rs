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
pub trait Source: Send + Sync {
    /// Read bytes.
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError>;
    /// Whether the path exists.
    fn exists(&self, path: &AssetPath) -> bool;
    /// List of roots for diagnostics.
    fn roots(&self) -> Vec<PathBuf>;
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
}
