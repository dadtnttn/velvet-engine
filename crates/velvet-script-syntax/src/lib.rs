//! Lossless syntax tree (Phase 4)
//!
//! **Status:** scaffold for Phase 4. Real implementation lands in that phase.
//! This module is intentionally thin so the workspace stays compilable.

#![allow(missing_docs)]

/// Crate version string.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Human-readable crate name.
pub fn crate_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    #[test]
    fn crate_metadata() {
        assert!(!super::crate_name().is_empty());
        assert!(!super::crate_version().is_empty());
    }
}
