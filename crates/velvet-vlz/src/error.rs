//! Errors of the VLZ container.

use thiserror::Error;

use crate::format::VERSION_MAJOR;

/// Everything that can go wrong reading or writing a VLZ archive.
///
/// Reading never panics on malformed input: every size, offset and count of an archive is
/// checked before it is used, and anything inconsistent ends here.
#[derive(Debug, Error)]
pub enum VlzError {
    /// Underlying I/O failure (including a file shorter than its index says).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// The file does not start with the VLZ magic.
    #[error("not a VLZ archive (bad magic)")]
    BadMagic,
    /// A major version this build cannot read.
    #[error(
        "unsupported VLZ version {major}.{minor} (this build reads {}.x)",
        VERSION_MAJOR
    )]
    IncompatibleVersion {
        /// Major version found.
        major: u16,
        /// Minor version found.
        minor: u16,
    },
    /// Header flags this build does not understand.
    #[error("unsupported VLZ header flags {0:#x}")]
    UnsupportedFlags(u32),
    /// Structurally invalid archive (sizes, offsets, counts, names).
    #[error("corrupt VLZ archive: {0}")]
    Corrupt(String),
    /// A checksum did not match (header, index or a block).
    #[error("checksum mismatch: {0}")]
    Checksum(String),
    /// A block uses a codec id no decoder is registered for.
    #[error("unsupported codec id {0}")]
    UnsupportedCodec(u8),
    /// No entry with that name.
    #[error("entry not found: {0}")]
    NotFound(String),
    /// A size above the configured safety limits.
    #[error("limit exceeded: {0}")]
    Limit(String),
    /// A byte range outside the entry.
    #[error("range {offset}+{len} is outside {name} ({size} bytes)")]
    Range {
        /// Entry name.
        name: String,
        /// Requested offset.
        offset: u64,
        /// Requested length.
        len: u64,
        /// Entry size.
        size: u64,
    },
    /// Invalid argument from the caller (names, options).
    #[error("invalid input: {0}")]
    Invalid(String),
    /// A compressor failed.
    #[error("compression failed: {0}")]
    Compress(String),
    /// A decompressor rejected a block.
    #[error("decompression failed: {0}")]
    Decompress(String),
}

/// Result alias of this crate.
pub type Result<T> = std::result::Result<T, VlzError>;

impl From<VlzError> for std::io::Error {
    fn from(value: VlzError) -> Self {
        use std::io::ErrorKind;
        match value {
            VlzError::Io(e) => e,
            VlzError::NotFound(name) => std::io::Error::new(ErrorKind::NotFound, name),
            VlzError::Invalid(_) | VlzError::Range { .. } => {
                std::io::Error::new(ErrorKind::InvalidInput, value.to_string())
            }
            other => std::io::Error::new(ErrorKind::InvalidData, other.to_string()),
        }
    }
}

pub(crate) fn corrupt(message: impl Into<String>) -> VlzError {
    VlzError::Corrupt(message.into())
}
