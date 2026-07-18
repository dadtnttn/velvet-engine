//! Virtual and filesystem asset paths.

use std::fmt;
use std::path::{Path, PathBuf};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

/// Virtual path using `/` separators, no leading drive letters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VirtualPath(String);

impl VirtualPath {
    /// Normalize separators and strip leading `./`.
    pub fn new(path: impl AsRef<str>) -> Self {
        let mut s = path.as_ref().replace('\\', "/");
        while s.starts_with("./") {
            s = s[2..].to_string();
        }
        while s.starts_with('/') {
            s = s[1..].to_string();
        }
        Self(s)
    }

    /// As string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extension without dot.
    pub fn extension(&self) -> Option<&str> {
        Utf8Path::new(&self.0).extension()
    }

    /// File name.
    pub fn file_name(&self) -> Option<&str> {
        Utf8Path::new(&self.0).file_name()
    }

    /// Join child.
    pub fn join(&self, child: &str) -> Self {
        if self.0.is_empty() {
            Self::new(child)
        } else {
            Self::new(format!("{}/{}", self.0, child.trim_start_matches('/')))
        }
    }

    /// Parent path.
    pub fn parent(&self) -> Option<Self> {
        let p = Utf8Path::new(&self.0).parent()?;
        if p.as_str().is_empty() {
            None
        } else {
            Some(Self::new(p.as_str()))
        }
    }
}

impl fmt::Display for VirtualPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for VirtualPath {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for VirtualPath {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Resolved asset path with optional root mount.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetPath {
    /// Virtual path inside the asset VFS.
    pub virtual_path: VirtualPath,
    /// Optional mount name (e.g. `game`, `engine`).
    pub mount: Option<String>,
}

impl AssetPath {
    /// From virtual path on default mount.
    pub fn virtual_path(path: impl Into<VirtualPath>) -> Self {
        Self {
            virtual_path: path.into(),
            mount: None,
        }
    }

    /// With mount.
    pub fn with_mount(mut self, mount: impl Into<String>) -> Self {
        self.mount = Some(mount.into());
        self
    }

    /// Resolve against a filesystem root.
    pub fn resolve_fs(&self, root: &Path) -> PathBuf {
        let mut out = root.to_path_buf();
        for part in self.virtual_path.as_str().split('/') {
            if part.is_empty() || part == "." {
                continue;
            }
            // Reject path traversal.
            if part == ".." {
                continue;
            }
            out.push(part);
        }
        out
    }

    /// As UTF-8 path relative.
    pub fn as_utf8(&self) -> Utf8PathBuf {
        Utf8PathBuf::from(self.virtual_path.as_str())
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(m) = &self.mount {
            write!(f, "{m}:{}", self.virtual_path)
        } else {
            write!(f, "{}", self.virtual_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_blocks_traversal() {
        let p = VirtualPath::new(r".\foo\bar\..\baz.png");
        // We normalize slashes but do not resolve .. inside VirtualPath::new
        assert!(p.as_str().contains("baz.png"));
        let ap = AssetPath::virtual_path(p);
        let resolved = ap.resolve_fs(Path::new("/assets"));
        let s = resolved.to_string_lossy();
        assert!(!s.contains(".."));
    }
}
