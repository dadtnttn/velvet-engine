//! Policy for missing / failed assets.

use serde::{Deserialize, Serialize};

use crate::path::VirtualPath;

/// How to handle a missing asset at runtime.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MissingAssetPolicy {
    /// Use a loud pink checker placeholder (debug-friendly).
    #[default]
    PinkChecker,
    /// Substitute a fixed fallback virtual path.
    FallbackPath {
        /// Replacement path.
        path: VirtualPath,
    },
    /// Treat as hard error (propagate to caller).
    Error,
}

impl MissingAssetPolicy {
    /// Fallback to a path.
    pub fn fallback(path: impl Into<VirtualPath>) -> Self {
        Self::FallbackPath { path: path.into() }
    }

    /// Policy name for logs.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PinkChecker => "pink_checker",
            Self::FallbackPath { .. } => "fallback_path",
            Self::Error => "error",
        }
    }
}

/// Result of resolving a missing asset against a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingResolution {
    /// Use engine pink-checker placeholder (no path).
    UsePinkChecker,
    /// Load this path instead.
    UsePath(VirtualPath),
    /// Fail the load.
    Fail,
}

/// Resolve what to do for a missing `requested` path.
pub fn resolve_missing(policy: &MissingAssetPolicy, requested: &VirtualPath) -> MissingResolution {
    let _ = requested;
    match policy {
        MissingAssetPolicy::PinkChecker => MissingResolution::UsePinkChecker,
        MissingAssetPolicy::FallbackPath { path } => MissingResolution::UsePath(path.clone()),
        MissingAssetPolicy::Error => MissingResolution::Fail,
    }
}

/// Well-known placeholder color (magenta/pink) as 8-bit RGBA.
pub const PINK_CHECKER_RGBA: [u8; 4] = [255, 0, 255, 255];

/// Generate a small N×N checkerboard RGBA8 buffer (two colors).
pub fn pink_checker_rgba8(size: u32, cell: u32) -> Vec<u8> {
    let size = size.max(1);
    let cell = cell.max(1);
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let cx = x / cell;
            let cy = y / cell;
            let pink = (cx + cy) % 2 == 0;
            if pink {
                data.extend_from_slice(&PINK_CHECKER_RGBA);
            } else {
                data.extend_from_slice(&[20, 20, 20, 255]);
            }
        }
    }
    data
}

/// Registry-level missing-asset settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingAssetConfig {
    /// Active policy.
    pub policy: MissingAssetPolicy,
    /// Optional per-kind overrides keyed by kind string (`texture`, `audio`, …).
    #[serde(default)]
    pub by_kind: indexmap::IndexMap<String, MissingAssetPolicy>,
}

impl Default for MissingAssetConfig {
    fn default() -> Self {
        Self {
            policy: MissingAssetPolicy::PinkChecker,
            by_kind: indexmap::IndexMap::new(),
        }
    }
}

impl MissingAssetConfig {
    /// Resolve policy for a kind (override or global).
    pub fn policy_for(&self, kind: Option<&str>) -> &MissingAssetPolicy {
        if let Some(k) = kind {
            if let Some(p) = self.by_kind.get(k) {
                return p;
            }
        }
        &self.policy
    }

    /// Resolve missing for kind + path.
    pub fn resolve(&self, kind: Option<&str>, requested: &VirtualPath) -> MissingResolution {
        resolve_missing(self.policy_for(kind), requested)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pink_checker_policy() {
        let r = resolve_missing(
            &MissingAssetPolicy::PinkChecker,
            &VirtualPath::new("missing.png"),
        );
        assert_eq!(r, MissingResolution::UsePinkChecker);
    }

    #[test]
    fn fallback_path_policy() {
        let p = MissingAssetPolicy::fallback("placeholders/missing.png");
        let r = resolve_missing(&p, &VirtualPath::new("nope.png"));
        assert!(
            matches!(r, MissingResolution::UsePath(ref v) if v.as_str() == "placeholders/missing.png")
        );
    }

    #[test]
    fn error_policy() {
        let r = resolve_missing(&MissingAssetPolicy::Error, &VirtualPath::new("x"));
        assert_eq!(r, MissingResolution::Fail);
    }

    #[test]
    fn checker_buffer_size() {
        let buf = pink_checker_rgba8(4, 2);
        assert_eq!(buf.len(), 4 * 4 * 4);
        // (0,0) pink
        assert_eq!(&buf[0..4], &PINK_CHECKER_RGBA);
    }

    #[test]
    fn config_by_kind() {
        let mut cfg = MissingAssetConfig::default();
        cfg.by_kind
            .insert("audio".into(), MissingAssetPolicy::Error);
        assert!(matches!(
            cfg.resolve(Some("audio"), &VirtualPath::new("a.ogg")),
            MissingResolution::Fail
        ));
        assert!(matches!(
            cfg.resolve(Some("texture"), &VirtualPath::new("t.png")),
            MissingResolution::UsePinkChecker
        ));
    }
}
