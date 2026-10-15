//! VLZ v1 binary layout: header, entry and block records, and the checks that make a file
//! safe to read. The full specification is `docs/VLZ_FORMAT.md`; every multi-byte field is
//! little-endian, whatever the host.
//!
//! ```text
//! offset 0            header (64 bytes)
//! header_size ..      block payloads, each starting on a multiple of `alignment`
//! index_offset ..     index: entry table, block table, name table
//! ```

use xxhash_rust::xxh3::xxh3_64;

use crate::codec::{max_stored_size, Codec};
use crate::error::{corrupt, Result, VlzError};

/// File signature: `V`, `L`, `Z`, then 0x1A (stops `type` on DOS consoles, like PNG's).
pub const MAGIC: [u8; 4] = *b"VLZ\x1A";
/// Major version: a reader rejects any other.
pub const VERSION_MAJOR: u16 = 1;
/// Minor version written by this build (readers accept any minor of their major).
pub const VERSION_MINOR: u16 = 0;
/// Size of the v1.0 header.
pub const HEADER_SIZE: usize = 64;
/// Largest header a v1 reader accepts (later minors may append fields).
pub const MAX_HEADER_SIZE: u32 = 4096;
/// Size of one entry record.
pub const ENTRY_RECORD_SIZE: usize = 48;
/// Size of one block record.
pub const BLOCK_RECORD_SIZE: usize = 32;
/// Smallest uniform block size a writer may choose (a lone block may be smaller).
pub const MIN_BLOCK_SIZE: u32 = 4 * 1024;
/// Largest uncompressed block: bounds every allocation a block can cause.
pub const MAX_BLOCK_SIZE: u32 = 16 * 1024 * 1024;
/// Largest payload alignment.
pub const MAX_ALIGNMENT: u32 = 64 * 1024;
/// Largest index a reader loads.
pub const MAX_INDEX_SIZE: u64 = 512 * 1024 * 1024;
/// Longest entry name, in bytes.
pub const MAX_NAME_LEN: usize = 4096;
/// Header flags understood by this build (none are defined in v1.0).
pub const KNOWN_FLAGS: u32 = 0;

/// Decoded file header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Major version.
    pub version_major: u16,
    /// Minor version.
    pub version_minor: u16,
    /// Size of the header as written.
    pub header_size: u32,
    /// Feature flags (must be a subset of [`KNOWN_FLAGS`]).
    pub flags: u32,
    /// Absolute offset of the index.
    pub index_offset: u64,
    /// Size of the index in bytes.
    pub index_size: u64,
    /// Number of entries.
    pub entry_count: u32,
    /// Number of blocks.
    pub block_count: u32,
    /// Payload alignment (power of two).
    pub alignment: u32,
    /// Nominal block size the writer used (informative).
    pub block_size: u32,
    /// XXH3-64 of the index bytes.
    pub index_checksum: u64,
}

impl Header {
    /// Encode to the 64-byte on-disk form (the trailing checksum included).
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut b = [0u8; HEADER_SIZE];
        b[0..4].copy_from_slice(&MAGIC);
        b[4..6].copy_from_slice(&self.version_major.to_le_bytes());
        b[6..8].copy_from_slice(&self.version_minor.to_le_bytes());
        b[8..12].copy_from_slice(&self.header_size.to_le_bytes());
        b[12..16].copy_from_slice(&self.flags.to_le_bytes());
        b[16..24].copy_from_slice(&self.index_offset.to_le_bytes());
        b[24..32].copy_from_slice(&self.index_size.to_le_bytes());
        b[32..36].copy_from_slice(&self.entry_count.to_le_bytes());
        b[36..40].copy_from_slice(&self.block_count.to_le_bytes());
        b[40..44].copy_from_slice(&self.alignment.to_le_bytes());
        b[44..48].copy_from_slice(&self.block_size.to_le_bytes());
        b[48..56].copy_from_slice(&self.index_checksum.to_le_bytes());
        let sum = xxh3_64(&b[0..56]);
        b[56..64].copy_from_slice(&sum.to_le_bytes());
        b
    }

    /// Decode and check the header (magic, version, checksum, flags, ranges).
    pub fn decode(b: &[u8]) -> Result<Self> {
        if b.len() < HEADER_SIZE {
            return Err(corrupt("file shorter than the VLZ header"));
        }
        if b[0..4] != MAGIC {
            return Err(VlzError::BadMagic);
        }
        let version_major = le_u16(b, 4);
        let version_minor = le_u16(b, 6);
        if version_major != VERSION_MAJOR {
            return Err(VlzError::IncompatibleVersion {
                major: version_major,
                minor: version_minor,
            });
        }
        if xxh3_64(&b[0..56]) != le_u64(b, 56) {
            return Err(VlzError::Checksum("header".into()));
        }
        let h = Header {
            version_major,
            version_minor,
            header_size: le_u32(b, 8),
            flags: le_u32(b, 12),
            index_offset: le_u64(b, 16),
            index_size: le_u64(b, 24),
            entry_count: le_u32(b, 32),
            block_count: le_u32(b, 36),
            alignment: le_u32(b, 40),
            block_size: le_u32(b, 44),
            index_checksum: le_u64(b, 48),
        };
        if h.flags & !KNOWN_FLAGS != 0 {
            return Err(VlzError::UnsupportedFlags(h.flags & !KNOWN_FLAGS));
        }
        if (h.header_size as usize) < HEADER_SIZE || h.header_size > MAX_HEADER_SIZE {
            return Err(corrupt(format!("header size {}", h.header_size)));
        }
        if h.alignment == 0 || !h.alignment.is_power_of_two() || h.alignment > MAX_ALIGNMENT {
            return Err(corrupt(format!("alignment {}", h.alignment)));
        }
        if h.index_offset < u64::from(h.header_size) {
            return Err(corrupt("index overlaps the header"));
        }
        if h.index_size > MAX_INDEX_SIZE {
            return Err(VlzError::Limit(format!("index of {} bytes", h.index_size)));
        }
        let tables = u64::from(h.entry_count) * ENTRY_RECORD_SIZE as u64
            + u64::from(h.block_count) * BLOCK_RECORD_SIZE as u64;
        if tables > h.index_size {
            return Err(corrupt("index smaller than its tables"));
        }
        Ok(h)
    }
}

/// One resource of the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryRecord {
    /// Offset of the name in the name table.
    pub name_offset: u32,
    /// Length of the name in bytes.
    pub name_len: u32,
    /// Uncompressed size.
    pub raw_size: u64,
    /// Sum of the stored sizes of its blocks.
    pub stored_size: u64,
    /// First block in the block table.
    pub first_block: u32,
    /// Number of blocks.
    pub block_count: u32,
    /// XXH3-64 of the whole uncompressed content.
    pub content_hash: u64,
    /// [`crate::ContentClass`] id.
    pub content_class: u16,
    /// Reserved, written as 0.
    pub flags: u16,
    /// Uncompressed size of every block but the last (0 for an empty entry).
    pub block_raw_size: u32,
}

impl EntryRecord {
    /// Encode into `out` (exactly [`ENTRY_RECORD_SIZE`] bytes).
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.name_offset.to_le_bytes());
        out.extend_from_slice(&self.name_len.to_le_bytes());
        out.extend_from_slice(&self.raw_size.to_le_bytes());
        out.extend_from_slice(&self.stored_size.to_le_bytes());
        out.extend_from_slice(&self.first_block.to_le_bytes());
        out.extend_from_slice(&self.block_count.to_le_bytes());
        out.extend_from_slice(&self.content_hash.to_le_bytes());
        out.extend_from_slice(&self.content_class.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&self.block_raw_size.to_le_bytes());
    }

    fn decode(b: &[u8]) -> Self {
        EntryRecord {
            name_offset: le_u32(b, 0),
            name_len: le_u32(b, 4),
            raw_size: le_u64(b, 8),
            stored_size: le_u64(b, 16),
            first_block: le_u32(b, 24),
            block_count: le_u32(b, 28),
            content_hash: le_u64(b, 32),
            content_class: le_u16(b, 40),
            flags: le_u16(b, 42),
            block_raw_size: le_u32(b, 44),
        }
    }
}

/// One independently decodable block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockRecord {
    /// Absolute file offset of the stored bytes.
    pub offset: u64,
    /// Stored (compressed) size.
    pub stored_size: u32,
    /// Uncompressed size.
    pub raw_size: u32,
    /// XXH3-64 of the stored bytes.
    pub checksum: u64,
    /// Codec id ([`Codec`] for 0–3).
    pub codec: u8,
    /// Compression level used (informative, two's complement).
    pub level: i8,
}

impl BlockRecord {
    /// Encode into `out` (exactly [`BLOCK_RECORD_SIZE`] bytes).
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.offset.to_le_bytes());
        out.extend_from_slice(&self.stored_size.to_le_bytes());
        out.extend_from_slice(&self.raw_size.to_le_bytes());
        out.extend_from_slice(&self.checksum.to_le_bytes());
        out.push(self.codec);
        out.push(self.level as u8);
        out.extend_from_slice(&[0u8; 6]);
    }

    fn decode(b: &[u8]) -> Self {
        BlockRecord {
            offset: le_u64(b, 0),
            stored_size: le_u32(b, 8),
            raw_size: le_u32(b, 12),
            checksum: le_u64(b, 16),
            codec: b[24],
            level: b[25] as i8,
        }
    }
}

/// The parsed, validated index.
#[derive(Debug, Clone)]
pub struct Index {
    /// Entries, sorted by name.
    pub entries: Vec<EntryRecord>,
    /// Blocks, in file order.
    pub blocks: Vec<BlockRecord>,
    /// Name table.
    pub names: String,
}

impl Index {
    /// Name of an entry (the index is validated, so the range is always good).
    pub fn name(&self, e: &EntryRecord) -> &str {
        let start = e.name_offset as usize;
        &self.names[start..start + e.name_len as usize]
    }

    /// Parse `bytes` (the whole index) and check it against the header and file length.
    ///
    /// After this succeeds every record can be used without further bounds checks: names
    /// are valid, sorted and unique; blocks partition the data region in order; each entry
    /// owns a contiguous run of blocks whose sizes add up.
    pub fn parse(bytes: &[u8], h: &Header, file_len: u64) -> Result<Self> {
        if bytes.len() as u64 != h.index_size {
            return Err(corrupt("index length differs from the header"));
        }
        if xxh3_64(bytes) != h.index_checksum {
            return Err(VlzError::Checksum("index".into()));
        }
        let end = h
            .index_offset
            .checked_add(h.index_size)
            .ok_or_else(|| corrupt("index range overflows"))?;
        if end > file_len {
            return Err(corrupt(format!(
                "index ends at {end} but the file has {file_len} bytes (truncated?)"
            )));
        }
        let entry_bytes = h.entry_count as usize * ENTRY_RECORD_SIZE;
        let block_bytes = h.block_count as usize * BLOCK_RECORD_SIZE;
        let (entry_table, rest) = bytes.split_at(entry_bytes);
        let (block_table, name_table) = rest.split_at(block_bytes);
        let names = std::str::from_utf8(name_table)
            .map_err(|_| corrupt("name table is not UTF-8"))?
            .to_owned();

        let mut blocks = Vec::with_capacity(h.block_count as usize);
        let mut next_free = u64::from(h.header_size);
        for (i, chunk) in block_table.chunks_exact(BLOCK_RECORD_SIZE).enumerate() {
            let b = BlockRecord::decode(chunk);
            if b.raw_size == 0 || b.raw_size > MAX_BLOCK_SIZE {
                return Err(corrupt(format!("block {i}: raw size {}", b.raw_size)));
            }
            if b.stored_size == 0 {
                return Err(corrupt(format!("block {i}: empty payload")));
            }
            let bound = match Codec::from_id(b.codec) {
                Some(Codec::Raw) => b.raw_size as usize,
                Some(codec) => max_stored_size(codec, b.raw_size as usize),
                // Codecs added by plug-in decoders: still bounded.
                None => 2 * MAX_BLOCK_SIZE as usize,
            };
            if b.stored_size as usize > bound
                || (b.codec == Codec::Raw.id() && b.stored_size != b.raw_size)
            {
                return Err(corrupt(format!(
                    "block {i}: stored size {} impossible for {} raw bytes",
                    b.stored_size, b.raw_size
                )));
            }
            if b.offset % u64::from(h.alignment) != 0 {
                return Err(corrupt(format!(
                    "block {i}: offset {} is not aligned",
                    b.offset
                )));
            }
            if b.offset < next_free {
                return Err(corrupt(format!("block {i}: overlaps the previous one")));
            }
            let stop = b.offset + u64::from(b.stored_size);
            if stop > h.index_offset {
                return Err(corrupt(format!("block {i}: runs into the index")));
            }
            next_free = stop;
            blocks.push(b);
        }

        let mut entries = Vec::with_capacity(h.entry_count as usize);
        let mut next_block = 0u32;
        let mut previous: Option<&str> = None;
        for (i, chunk) in entry_table.chunks_exact(ENTRY_RECORD_SIZE).enumerate() {
            let e = EntryRecord::decode(chunk);
            let start = e.name_offset as usize;
            let stop = start
                .checked_add(e.name_len as usize)
                .filter(|&s| s <= names.len())
                .ok_or_else(|| corrupt(format!("entry {i}: name outside the name table")))?;
            if !names.is_char_boundary(start) || !names.is_char_boundary(stop) {
                return Err(corrupt(format!("entry {i}: name splits a character")));
            }
            let name = &names[start..stop];
            validate_name(name).map_err(|e| corrupt(format!("entry {i}: {e}")))?;
            if let Some(prev) = previous {
                if prev.as_bytes() >= name.as_bytes() {
                    return Err(corrupt(format!(
                        "entry {i}: names out of order or repeated"
                    )));
                }
            }
            if e.first_block != next_block {
                return Err(corrupt(format!("entry {i}: blocks are not contiguous")));
            }
            let last = e
                .first_block
                .checked_add(e.block_count)
                .filter(|&l| l <= h.block_count)
                .ok_or_else(|| corrupt(format!("entry {i}: blocks outside the table")))?;
            let own = &blocks[e.first_block as usize..last as usize];
            if own.is_empty() {
                if e.raw_size != 0 || e.stored_size != 0 {
                    return Err(corrupt(format!("entry {i}: size without blocks")));
                }
            } else {
                if e.block_raw_size == 0 || e.block_raw_size > MAX_BLOCK_SIZE {
                    return Err(corrupt(format!(
                        "entry {i}: block size {}",
                        e.block_raw_size
                    )));
                }
                let (tail, body) = own.split_last().expect("not empty");
                if body.iter().any(|b| b.raw_size != e.block_raw_size)
                    || tail.raw_size > e.block_raw_size
                {
                    return Err(corrupt(format!("entry {i}: uneven blocks")));
                }
                let raw: u64 = own.iter().map(|b| u64::from(b.raw_size)).sum();
                let stored: u64 = own.iter().map(|b| u64::from(b.stored_size)).sum();
                if raw != e.raw_size || stored != e.stored_size {
                    return Err(corrupt(format!("entry {i}: sizes do not add up")));
                }
            }
            next_block = last;
            previous = Some(name);
            entries.push(e);
        }
        if next_block != h.block_count {
            return Err(corrupt("blocks owned by no entry"));
        }
        Ok(Index {
            entries,
            blocks,
            names,
        })
    }
}

/// Normalize a path to an entry name: `/` separators, no leading `./` or `/`.
/// Fails on names that could escape an extraction folder or are not portable.
pub fn normalize_name(path: &str) -> Result<String> {
    let mut s = path.replace('\\', "/");
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.to_owned();
    }
    let s = s.trim_start_matches('/').to_owned();
    validate_name(&s).map_err(|e| VlzError::Invalid(format!("{path:?}: {e}")))?;
    Ok(s)
}

fn validate_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("empty name".into());
    }
    if name.len() > MAX_NAME_LEN {
        return Err(format!("name longer than {MAX_NAME_LEN} bytes"));
    }
    if name.contains(['\\', ':', '\0']) {
        return Err(format!("name {name:?} holds \\, : or NUL"));
    }
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(format!("name {name:?} has an empty, . or .. segment"));
        }
    }
    Ok(())
}

pub(crate) fn le_u16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

pub(crate) fn le_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"))
}

pub(crate) fn le_u64(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("8 bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_layout_is_little_endian_and_fixed() {
        let h = Header {
            version_major: 1,
            version_minor: 0,
            header_size: 64,
            flags: 0,
            index_offset: 0x0102_0304_0506_0708,
            index_size: 0x10,
            entry_count: 0xAABB_CCDD,
            block_count: 2,
            alignment: 16,
            block_size: 262_144,
            index_checksum: 0x1122_3344_5566_7788,
        };
        let b = h.encode();
        assert_eq!(&b[0..4], b"VLZ\x1A");
        assert_eq!(&b[4..8], &[1, 0, 0, 0]);
        assert_eq!(&b[16..24], &[8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(&b[32..36], &[0xDD, 0xCC, 0xBB, 0xAA]);
        assert_eq!(
            &b[48..56],
            &[0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
        );
        // index_size 0x10 is smaller than its tables, so decoding rejects it.
        assert!(Header::decode(&b).is_err());
    }

    #[test]
    fn names_are_normalized_and_unsafe_ones_refused() {
        assert_eq!(normalize_name(r".\a\b.png").unwrap(), "a/b.png");
        assert_eq!(normalize_name("/x/y").unwrap(), "x/y");
        for bad in ["", "a/../b", "../a", "a//b", "c:/x", "a/./b", "a\0b"] {
            assert!(normalize_name(bad).is_err(), "{bad:?} accepted");
        }
    }
}
