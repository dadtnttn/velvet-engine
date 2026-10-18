//! The envelope every velvet-stacks document carries: `format`, `version` and `kind`.
//!
//! A reader accepts a document only when all three match what it reads. Version 1 is the only
//! version so far; a later incompatible change would bump `version` and nothing else.

use crate::error::StacksError;

/// The `format` value of every velvet-stacks document.
pub const FORMAT: &str = "velvet-stacks";

/// The document version this engine reads and writes.
pub const VERSION: u32 = 1;

/// Checks the envelope of a document that should be of kind `expected` (`"catalog"` or `"board"`).
pub(crate) fn check_envelope(
    format: &str,
    version: u32,
    kind: &str,
    expected: &str,
) -> Result<(), StacksError> {
    if format == FORMAT && version == VERSION && kind == expected {
        Ok(())
    } else {
        Err(StacksError::UnsupportedDocument {
            expected: expected.to_string(),
            found: format!("format `{format}`, kind `{kind}`, version {version}"),
        })
    }
}

/// Turns a JSON error into the engine's parse error.
pub(crate) fn json_error(error: serde_json::Error) -> StacksError {
    StacksError::Parse(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_matching_envelope() {
        assert!(check_envelope(FORMAT, VERSION, "catalog", "catalog").is_ok());
    }

    #[test]
    fn refuses_other_kinds_formats_and_versions() {
        assert!(check_envelope(FORMAT, VERSION, "board", "catalog").is_err());
        assert!(check_envelope("velvet-cards", VERSION, "catalog", "catalog").is_err());
        assert!(check_envelope(FORMAT, 2, "catalog", "catalog").is_err());
    }
}
