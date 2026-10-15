//! # velvet-vlz — VLZ (Velvet Compressed)
//!
//! Velvet Engine's native container for compressed resources: one file holding one or many
//! resources, each split into independently decodable blocks so a resource can be read
//! whole, in parallel, or by byte range without touching the rest.
//!
//! - Codecs: RAW (0), LZ4 block (1), Zstandard (2), Zstandard high level (3); ids 4–255 are
//!   reserved for plug-in decoders ([`BlockDecoder`]), 4 for GDeflate.
//! - The automatic method ([`Method::Auto`]) leaves already compressed data alone (PNG, JPEG,
//!   Ogg, MP4, ZIP, supercompressed KTX2...) and picks a codec per resource with a
//!   deterministic cost model; GPU texture blocks (BCn/ASTC) and meshoptimizer data are kept
//!   bit-exact, so their own decoders work on what comes out.
//! - Every size, offset and count is validated before use; blocks cannot inflate past the
//!   size their record declares; header, index and blocks carry XXH3-64 checksums.
//! - Little-endian everywhere; the layout is in `docs/VLZ_FORMAT.md`.
//!
//! ```no_run
//! use velvet_vlz::{pack_directory, PackOptions, VlzArchive};
//!
//! pack_directory("assets", "assets.vlz", &PackOptions::default())?;
//! let archive = VlzArchive::open("assets.vlz")?;
//! let bytes = archive.read("sprites/hero.png")?;
//! let id = archive.find("music/theme.ogg").unwrap();
//! let header = archive.read_range(id, 0, 4096)?; // only the first block is decoded
//! # Ok::<(), velvet_vlz::VlzError>(())
//! ```
//!
//! Streaming, priorities, cancellation and the byte cache live in `velvet-assets`
//! (`AssetStreamer`), which reads VLZ through its `VlzSource` like any other source.

#![deny(missing_docs)]

mod classify;
mod codec;
mod error;
pub mod format;
mod io;
mod pool;
mod reader;
mod writer;

pub use classify::{
    auto_block_size, choose_codec, classify, ContentClass, Method, Profile, TINY_ENTRY,
};
pub use codec::{
    compress_block, decompress_block, max_stored_size, BlockDecoder, Codec, DecoderRegistry,
    DEFAULT_ZSTD_HIGH_LEVEL, DEFAULT_ZSTD_LEVEL, RESERVED_GDEFLATE_ID,
};
pub use error::{Result, VlzError};
pub use format::{normalize_name, BlockRecord, Header, MAGIC, VERSION_MAJOR, VERSION_MINOR};
pub use pool::{BufferPool, PoolStats, PooledBuf};
pub use reader::{
    ArchiveStats, CodecTotals, EntryId, EntryInfo, OpenOptions, ReadLimits, VerifyReport,
    VlzArchive,
};
pub use writer::{
    collect_directory, pack, pack_directory, pack_to_file, pack_to_vec, PackInput, PackOptions,
    PackReport, PackSource,
};
