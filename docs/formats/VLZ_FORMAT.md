# VLZ v1 — Velvet Compressed: binary format

VLZ is Velvet Engine's container for compressed resources. One `.vlz` file holds one or many
resources ("entries"); each entry is split into blocks that decode independently, so an entry
can be read whole, decoded in parallel, or read by byte range without touching the rest.

Reference implementation: `crates/velvet-vlz` (`format.rs` holds the layout and every check
listed here). This document is normative for version 1.0.

## Conventions

- All integers are **little-endian**, unsigned unless stated, whatever the host.
- Offsets are absolute byte offsets from the start of the file.
- Sizes are in bytes. "Raw" means uncompressed, "stored" means as written in the file.
- Checksums are **XXH3-64** (seed 0) of the bytes named.
- Fields marked *reserved* are written as 0 and ignored by readers of this version.

## File layout

```text
0             Header                 64 bytes (header_size)
header_size   Block payloads         each starts at a multiple of `alignment`, zero padding between
index_offset  Index                  entry table | block table | name table
```

The writer streams payloads first and writes the index last, then rewrites the header; a
reader needs only the header and the index to locate any byte of any entry.

## Header (64 bytes)

| Offset | Size | Field | Meaning |
|---:|---:|---|---|
| 0 | 4 | `magic` | `56 4C 5A 1A` (`"VLZ"` + 0x1A) |
| 4 | 2 | `version_major` | 1. A reader rejects any other major version. |
| 6 | 2 | `version_minor` | 0. Readers accept any minor of their major. |
| 8 | 4 | `header_size` | 64 in v1.0; readers accept 64–4096 (later minors may append fields). |
| 12 | 4 | `flags` | Feature bits. None are defined in v1.0; a reader rejects bits it does not know. |
| 16 | 8 | `index_offset` | Offset of the index; ≥ `header_size`. |
| 24 | 8 | `index_size` | Size of the index; ≤ 512 MiB. |
| 32 | 4 | `entry_count` | Number of entry records. |
| 36 | 4 | `block_count` | Number of block records. |
| 40 | 4 | `alignment` | Payload alignment: a power of two, 1–65536 (default 16). |
| 44 | 4 | `block_size` | Nominal block size used by the writer (informative). |
| 48 | 8 | `index_checksum` | XXH3-64 of the `index_size` index bytes. |
| 56 | 8 | `header_checksum` | XXH3-64 of header bytes 0–55. |

Reading order: magic → version → header checksum → flags and ranges. Checking the version
before the checksum lets a future major version change the header freely.

## Index

`index_size = entry_count × 48 + block_count × 32 + name_table_size`, in this order:

### Entry record (48 bytes)

| Offset | Size | Field | Meaning |
|---:|---:|---|---|
| 0 | 4 | `name_offset` | Offset of the name in the name table. |
| 4 | 4 | `name_len` | Name length in bytes. |
| 8 | 8 | `raw_size` | Uncompressed size of the entry. |
| 16 | 8 | `stored_size` | Sum of the stored sizes of its blocks. |
| 24 | 4 | `first_block` | Index of its first block in the block table. |
| 28 | 4 | `block_count` | Number of blocks (0 for an empty entry). |
| 32 | 8 | `content_hash` | XXH3-64 of the whole uncompressed content. |
| 40 | 2 | `content_class` | What the content is (table below); a hint for tools and loaders. |
| 42 | 2 | `flags` | *Reserved.* |
| 44 | 4 | `block_raw_size` | Raw size of every block except the last (0 when there are no blocks). |

Because all blocks but the last have the same raw size, the block holding raw byte `x` is
`first_block + x / block_raw_size`: random access needs no search.

### Block record (32 bytes)

| Offset | Size | Field | Meaning |
|---:|---:|---|---|
| 0 | 8 | `offset` | Offset of the stored bytes. |
| 8 | 4 | `stored_size` | Stored size (> 0). |
| 12 | 4 | `raw_size` | Raw size: 1 to 16 MiB (`MAX_BLOCK_SIZE`). |
| 16 | 8 | `checksum` | XXH3-64 of the stored bytes. |
| 24 | 1 | `codec` | Codec id (table below). |
| 25 | 1 | `level` | Compression level used, two's complement (informative). |
| 26 | 6 | — | *Reserved.* |

### Name table

The names of all entries, UTF-8, concatenated without separators. A name:

- uses `/` as separator, has no leading `/`, and is at most 4096 bytes;
- has no empty, `.` or `..` segment and contains no `\`, `:` or NUL (so extraction can never
  leave the target folder and names are portable across file systems).

Entries are **sorted by name** (byte order) and names are unique: lookup is a binary search
and the same inputs always produce the same archive.

## Codecs

| Id | Codec | Notes |
|---:|---|---|
| 0 | RAW | Stored as is; `stored_size` = `raw_size`. |
| 1 | LZ4 | LZ4 **block** format (no frame header). |
| 2 | Zstandard | One Zstandard frame per block, balanced level (default 3). |
| 3 | Zstandard high | Same decoder as 2; written at a high level (default 19). |
| 4 | *reserved* | GDeflate. |
| 5–127 | *reserved* | Future standard codecs. |
| 128–255 | application | Need a decoder registered by the application. |

A block decodes to **exactly** `raw_size` bytes or the read fails; a decoder must never write
beyond that. Writers store a block RAW when compressing it saves less than 1/32 of its size.

## Content classes

| Id | Class | Id | Class |
|---:|---|---:|---|
| 0 | unknown | 6 | GPU texture (DDS, KTX/KTX2 without supercompression) |
| 1 | text | 7 | mesh (glTF/GLB, meshoptimizer data) |
| 2 | binary | 8 | archive (ZIP, gzip, zstd, LZ4, 7z, WOFF2, supercompressed KTX2, Basis) |
| 3 | compressed image (PNG, JPEG, WebP, GIF, AVIF) | 9 | font |
| 4 | compressed audio (Ogg, Opus, MP3, FLAC, AAC) | 10 | PCM audio (WAV, AIFF) |
| 5 | video (MP4, WebM, MKV) | | |

Classes 3, 4, 5 and 8 are already compressed: the automatic method stores them RAW. GPU
texture blocks and meshoptimizer streams are compressed losslessly, so their own decoders
(BCn upload, Basis transcoder, meshopt decoder) receive the original bytes.

## Validation a reader performs

Before any entry is served (`Index::parse`):

1. Header: magic, major version, checksum, known flags, `header_size` in range,
   `alignment` a power of two ≤ 65536, index after the header, `index_size` ≤ 512 MiB and at
   least as large as its tables, `index_offset + index_size` ≤ file length (truncation).
2. Index checksum.
3. Blocks: `raw_size` in 1..16 MiB; `stored_size` > 0 and ≤ the codec's worst case
   (`raw_size` for RAW, LZ4/Zstandard bounds, 32 MiB for application codecs); offset aligned;
   blocks in increasing, non-overlapping order between the header and the index.
4. Entries: name inside the name table, valid UTF-8 on character boundaries, a valid name,
   strictly increasing names; blocks contiguous and in table order; together the entries own
   every block exactly once; every block but the last has `block_raw_size`, the last at most
   that; raw and stored sizes add up.

While reading a block: the stored bytes must match `checksum`, and decoding must produce
exactly `raw_size` bytes. A whole-entry read refuses entries above the reader's
`max_entry_size` (4 GiB by default) before allocating; byte-range reads have no such limit.

Together these make decompression bombs impossible (no block inflates past 16 MiB, and every
allocation size is known and bounded before it happens) and make truncated or tampered files
fail with an error instead of a crash.

## Compatibility rules

- New **minor** versions may append header fields (growing `header_size`), define reserved
  fields with "0 = old behaviour" semantics, and add flags. A flag that old readers could not
  safely ignore must make them refuse the file (they reject unknown flags).
- Anything else is a new **major** version.
- `crates/velvet-vlz/tests/fixtures/golden_v1.vlz` is a committed v1.0 archive; the test suite
  reads it and checks that the writer still produces it byte for byte.

## Determinism

For the same inputs, options and codec library versions, the writer produces identical
bytes: entries are sorted by name, nothing depends on time or thread count, and the automatic
codec choice uses compressed sizes of a fixed sample, scored with fixed constants.
