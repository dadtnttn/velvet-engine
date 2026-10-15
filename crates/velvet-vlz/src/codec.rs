//! Block codecs: RAW, LZ4 (block format), Zstandard and Zstandard at a high level, plus the
//! extension point for codecs added later (GDeflate and others) without a hard dependency.

use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;

use zstd::zstd_safe::{CCtx, DCtx};

use crate::error::{Result, VlzError};

/// Codec ids 0–3 are defined by VLZ v1. Ids 4–127 are reserved for future standard codecs
/// (4 is set aside for GDeflate) and 128–255 for application codecs; blocks using them need a
/// [`BlockDecoder`] registered on the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum Codec {
    /// Stored as is.
    Raw = 0,
    /// LZ4 block format: the fastest decompression.
    Lz4 = 1,
    /// Zstandard, balanced level.
    Zstd = 2,
    /// Zstandard, high level: the same decoder, a smaller archive, a slower pack.
    ZstdHigh = 3,
}

/// Default level of [`Codec::Zstd`].
pub const DEFAULT_ZSTD_LEVEL: i32 = 3;
/// Default level of [`Codec::ZstdHigh`].
pub const DEFAULT_ZSTD_HIGH_LEVEL: i32 = 19;
/// Codec id reserved for GDeflate.
pub const RESERVED_GDEFLATE_ID: u8 = 4;

impl Codec {
    /// All built-in codecs, by id.
    pub const ALL: [Codec; 4] = [Codec::Raw, Codec::Lz4, Codec::Zstd, Codec::ZstdHigh];

    /// From an on-disk id.
    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.get(id as usize).copied()
    }

    /// On-disk id.
    pub fn id(self) -> u8 {
        self as u8
    }

    /// Short lowercase name (`raw`, `lz4`, `zstd`, `zstd-high`).
    pub fn name(self) -> &'static str {
        match self {
            Codec::Raw => "raw",
            Codec::Lz4 => "lz4",
            Codec::Zstd => "zstd",
            Codec::ZstdHigh => "zstd-high",
        }
    }

    /// Parse a name as written by [`Codec::name`].
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Largest stored size a block of `raw` bytes can have with `codec`.
pub fn max_stored_size(codec: Codec, raw: usize) -> usize {
    match codec {
        Codec::Raw => raw,
        Codec::Lz4 => lz4_flex::block::get_maximum_output_size(raw),
        Codec::Zstd | Codec::ZstdHigh => zstd::zstd_safe::compress_bound(raw),
    }
}

thread_local! {
    static CCTX: RefCell<Option<CCtx<'static>>> = const { RefCell::new(None) };
    static DCTX: RefCell<Option<DCtx<'static>>> = const { RefCell::new(None) };
}

/// Compress `src` into `out` (cleared first). Deterministic for a given codec, level and
/// library version: the output does not depend on threads or timing.
pub fn compress_block(codec: Codec, level: i32, src: &[u8], out: &mut Vec<u8>) -> Result<()> {
    out.clear();
    match codec {
        Codec::Raw => out.extend_from_slice(src),
        Codec::Lz4 => {
            out.resize(max_stored_size(codec, src.len()), 0);
            let n = lz4_flex::block::compress_into(src, out)
                .map_err(|e| VlzError::Compress(format!("lz4: {e}")))?;
            out.truncate(n);
        }
        Codec::Zstd | Codec::ZstdHigh => {
            out.reserve(max_stored_size(codec, src.len()));
            CCTX.with(|cell| {
                let mut slot = cell.borrow_mut();
                let cctx = slot.get_or_insert_with(CCtx::create);
                cctx.compress(out, src, level)
                    .map(|_| ())
                    .map_err(|code| VlzError::Compress(format!("zstd: {}", zstd_error(code))))
            })?;
        }
    }
    Ok(())
}

/// Decompress `src` into exactly `dst.len()` bytes. Fails (never writes past `dst`) when the
/// block decodes to any other size: a block cannot inflate beyond what its record declares.
pub fn decompress_block(codec: Codec, src: &[u8], dst: &mut [u8]) -> Result<()> {
    let n = match codec {
        Codec::Raw => {
            if src.len() != dst.len() {
                return Err(VlzError::Decompress("raw block size mismatch".into()));
            }
            dst.copy_from_slice(src);
            src.len()
        }
        Codec::Lz4 => lz4_flex::block::decompress_into(src, dst)
            .map_err(|e| VlzError::Decompress(format!("lz4: {e}")))?,
        Codec::Zstd | Codec::ZstdHigh => DCTX.with(|cell| {
            let mut slot = cell.borrow_mut();
            let dctx = slot.get_or_insert_with(DCtx::create);
            dctx.decompress(dst, src)
                .map_err(|code| VlzError::Decompress(format!("zstd: {}", zstd_error(code))))
        })?,
    };
    if n != dst.len() {
        return Err(VlzError::Decompress(format!(
            "{codec} block decoded to {n} bytes, record says {}",
            dst.len()
        )));
    }
    Ok(())
}

fn zstd_error(code: usize) -> &'static str {
    zstd::zstd_safe::get_error_name(code)
}

/// A decoder for a codec id beyond the built-in ones (e.g. a CPU GDeflate decoder, or a GPU
/// path that falls back to CPU). It must write exactly `dst.len()` bytes or fail.
pub trait BlockDecoder: Send + Sync {
    /// Human-readable name for diagnostics.
    fn name(&self) -> &str;
    /// Decode one block.
    fn decode(&self, src: &[u8], dst: &mut [u8]) -> Result<()>;
}

/// Decoders by codec id: built-ins plus registered extensions.
#[derive(Clone, Default)]
pub struct DecoderRegistry {
    extra: Vec<(u8, Arc<dyn BlockDecoder>)>,
}

impl fmt::Debug for DecoderRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<_> = self.extra.iter().map(|(id, d)| (*id, d.name())).collect();
        f.debug_struct("DecoderRegistry")
            .field("extra", &names)
            .finish()
    }
}

impl DecoderRegistry {
    /// Register a decoder for an id outside 0–3 (built-ins cannot be replaced).
    pub fn register(&mut self, id: u8, decoder: Arc<dyn BlockDecoder>) -> Result<()> {
        if Codec::from_id(id).is_some() {
            return Err(VlzError::Invalid(format!("codec id {id} is built in")));
        }
        self.extra.retain(|(i, _)| *i != id);
        self.extra.push((id, decoder));
        Ok(())
    }

    /// Decode one block with whatever handles `id`.
    pub fn decode(&self, id: u8, src: &[u8], dst: &mut [u8]) -> Result<()> {
        if let Some(codec) = Codec::from_id(id) {
            return decompress_block(codec, src, dst);
        }
        match self.extra.iter().find(|(i, _)| *i == id) {
            Some((_, d)) => d.decode(src, dst),
            None => Err(VlzError::UnsupportedCodec(id)),
        }
    }

    /// Whether `id` can be decoded.
    pub fn supports(&self, id: u8) -> bool {
        Codec::from_id(id).is_some() || self.extra.iter().any(|(i, _)| *i == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        (0..200_000u32)
            .flat_map(|i| ((i / 7) as u16).to_le_bytes())
            .collect()
    }

    #[test]
    fn every_codec_round_trips() {
        let src = sample();
        for codec in Codec::ALL {
            let mut out = Vec::new();
            let level = if codec == Codec::ZstdHigh { 19 } else { 3 };
            compress_block(codec, level, &src, &mut out).unwrap();
            assert!(out.len() <= max_stored_size(codec, src.len()));
            let mut back = vec![0u8; src.len()];
            decompress_block(codec, &out, &mut back).unwrap();
            assert_eq!(back, src, "{codec}");
        }
    }

    #[test]
    fn wrong_output_size_is_an_error_not_an_overflow() {
        let src = sample();
        for codec in [Codec::Lz4, Codec::Zstd] {
            let mut out = Vec::new();
            compress_block(codec, 3, &src, &mut out).unwrap();
            let mut short = vec![0u8; src.len() - 1];
            assert!(decompress_block(codec, &out, &mut short).is_err());
            let mut long = vec![0u8; src.len() + 1];
            assert!(decompress_block(codec, &out, &mut long).is_err());
        }
    }

    #[test]
    fn garbage_never_panics() {
        let mut dst = vec![0u8; 4096];
        for seed in 0..64u32 {
            let junk: Vec<u8> = (0..512u32)
                .map(|i| (i.wrapping_mul(2_654_435_761).wrapping_add(seed * 97) >> 13) as u8)
                .collect();
            for codec in [Codec::Lz4, Codec::Zstd] {
                let _ = decompress_block(codec, &junk, &mut dst);
            }
        }
    }

    #[test]
    fn extension_decoders_plug_in() {
        struct Invert;
        impl BlockDecoder for Invert {
            fn name(&self) -> &str {
                "invert"
            }
            fn decode(&self, src: &[u8], dst: &mut [u8]) -> Result<()> {
                if src.len() != dst.len() {
                    return Err(VlzError::Decompress("size".into()));
                }
                for (d, s) in dst.iter_mut().zip(src) {
                    *d = !s;
                }
                Ok(())
            }
        }
        let mut reg = DecoderRegistry::default();
        assert!(reg.register(1, Arc::new(Invert)).is_err());
        assert!(matches!(
            reg.decode(200, &[0], &mut [0]),
            Err(VlzError::UnsupportedCodec(200))
        ));
        reg.register(200, Arc::new(Invert)).unwrap();
        let mut dst = [0u8; 2];
        reg.decode(200, &[0x0F, 0xF0], &mut dst).unwrap();
        assert_eq!(dst, [0xF0, 0x0F]);
    }
}
