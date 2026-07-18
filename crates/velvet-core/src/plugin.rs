//! Plugin identification and version requirements.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::version::Version;

/// Stable plugin identifier (human-readable, reverse-DNS recommended).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(pub String);

impl PluginId {
    /// Create from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the raw id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for PluginId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for PluginId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Semantic-ish version requirement: exact major, minimum minor/patch optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionReq {
    /// Required major version.
    pub major: u64,
    /// Minimum minor (inclusive).
    pub minor_min: u64,
    /// Minimum patch when minor equals `minor_min`.
    pub patch_min: u64,
}

impl VersionReq {
    /// Require compatible major and at least minor.patch.
    pub const fn compatible(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor_min: minor,
            patch_min: patch,
        }
    }

    /// Check whether `version` satisfies this requirement (same major, >= minor/patch).
    pub fn matches(&self, version: Version) -> bool {
        if version.major != self.major {
            return false;
        }
        if version.minor > self.minor_min {
            return true;
        }
        if version.minor < self.minor_min {
            return false;
        }
        version.patch >= self.patch_min
    }
}

/// Static metadata about a plugin.
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Unique id.
    pub id: PluginId,
    /// Display name.
    pub name: &'static str,
    /// Plugin version.
    pub version: Version,
    /// Dependency plugin ids.
    pub dependencies: &'static [PluginId],
}

/// Errors produced while resolving or building plugins.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PluginError {
    /// Two plugins registered with the same id.
    #[error("duplicate plugin id '{0}'")]
    Duplicate(PluginId),

    /// Missing dependency.
    #[error("plugin '{plugin}' requires missing dependency '{missing}'")]
    MissingDependency {
        /// Plugin that declared the dependency.
        plugin: PluginId,
        /// Missing dependency id.
        missing: PluginId,
    },

    /// Dependency cycle detected.
    #[error("plugin dependency cycle detected: {0}")]
    Cycle(String),

    /// Version requirement failed.
    #[error("plugin '{plugin}' version {found} does not satisfy requirement for '{required_by}'")]
    VersionMismatch {
        /// Offending plugin.
        plugin: PluginId,
        /// Found version string.
        found: String,
        /// Who required it.
        required_by: PluginId,
    },

    /// Build hook failed.
    #[error("plugin '{plugin}' build failed: {message}")]
    BuildFailed {
        /// Plugin id.
        plugin: PluginId,
        /// Message.
        message: String,
    },

    /// Finish hook failed.
    #[error("plugin '{plugin}' finish failed: {message}")]
    FinishFailed {
        /// Plugin id.
        plugin: PluginId,
        /// Message.
        message: String,
    },

    /// Plugin disabled by configuration.
    #[error("plugin '{0}' is disabled")]
    Disabled(PluginId),
}
