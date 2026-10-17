//! Content classification and the automatic codec choice.
//!
//! The choice must be deterministic (same input, same archive), so it never looks at clocks:
//! it compresses a fixed sample with LZ4 and Zstandard and scores each candidate with a cost
//! model of fixed constants (see [`Profile`]).

use crate::codec::{compress_block, Codec};
use crate::format::{MAX_BLOCK_SIZE, MIN_BLOCK_SIZE};

/// What a resource is, as far as compression cares. Stored in each entry so loaders and
/// tools can tell, e.g., a GPU texture from a PNG without parsing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u16)]
pub enum ContentClass {
    /// Not recognized.
    Unknown = 0,
    /// Text (JSON, Markdown, RON, scripts, shaders...).
    Text = 1,
    /// Uncompressed binary data.
    Binary = 2,
    /// PNG, JPEG, WebP, GIF, AVIF: already entropy coded.
    CompressedImage = 3,
    /// Ogg, MP3, FLAC, Opus, AAC: already compressed.
    CompressedAudio = 4,
    /// MP4, WebM, MKV: already compressed.
    Video = 5,
    /// GPU block-compressed texture (DDS, KTX2 without supercompression): BCn/ASTC/ETC data
    /// kept bit-exact for the GPU, still worth a generic codec.
    GpuTexture = 6,
    /// glTF/GLB and meshoptimizer-encoded geometry: designed to be followed by a generic codec.
    Mesh = 7,
    /// ZIP, gzip, zstd, LZ4, 7z, or a supercompressed KTX2/Basis texture.
    Archive = 8,
    /// Fonts (TTF/OTF compressible, WOFF2 not).
    Font = 9,
    /// Uncompressed audio (WAV, AIFF).
    Pcm = 10,
}

impl ContentClass {
    /// From the on-disk id (unknown ids map to [`ContentClass::Unknown`]).
    pub fn from_id(id: u16) -> Self {
        use ContentClass::*;
        match id {
            1 => Text,
            2 => Binary,
            3 => CompressedImage,
            4 => CompressedAudio,
            5 => Video,
            6 => GpuTexture,
            7 => Mesh,
            8 => Archive,
            9 => Font,
            10 => Pcm,
            _ => Unknown,
        }
    }

    /// Short name for tools.
    pub fn name(self) -> &'static str {
        use ContentClass::*;
        match self {
            Unknown => "unknown",
            Text => "text",
            Binary => "binary",
            CompressedImage => "image",
            CompressedAudio => "audio",
            Video => "video",
            GpuTexture => "gpu-texture",
            Mesh => "mesh",
            Archive => "archive",
            Font => "font",
            Pcm => "pcm",
        }
    }

    /// Already compressed: a generic codec would spend time for (almost) nothing.
    pub fn is_precompressed(self) -> bool {
        matches!(
            self,
            ContentClass::CompressedImage
                | ContentClass::CompressedAudio
                | ContentClass::Video
                | ContentClass::Archive
        )
    }
}

/// Classify by signature first, extension second. `head` is the first bytes of the file
/// (64 are enough).
pub fn classify(name: &str, head: &[u8]) -> ContentClass {
    use ContentClass::*;
    let starts = |sig: &[u8]| head.starts_with(sig);
    if starts(b"\x89PNG\r\n\x1a\n") || starts(&[0xFF, 0xD8, 0xFF]) || starts(b"GIF8") {
        return CompressedImage;
    }
    if head.len() >= 12 && &head[0..4] == b"RIFF" {
        match &head[8..12] {
            b"WEBP" => return CompressedImage,
            b"WAVE" => return Pcm,
            _ => {}
        }
    }
    if starts(b"OggS") || starts(b"fLaC") || starts(b"ID3") {
        return CompressedAudio;
    }
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        let brand = &head[8..12];
        return if brand == b"avif" || brand == b"heic" || brand == b"mif1" {
            CompressedImage
        } else {
            Video
        };
    }
    if starts(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Video; // Matroska / WebM
    }
    if starts(b"PK\x03\x04")
        || starts(&[0x1F, 0x8B])
        || starts(&[0x28, 0xB5, 0x2F, 0xFD])
        || starts(&[0x04, 0x22, 0x4D, 0x18])
        || starts(b"7z\xBC\xAF\x27\x1C")
        || starts(b"VLZ\x1A")
        || starts(b"wOF2")
    {
        return Archive;
    }
    if starts(b"\xABKTX 20\xBB\r\n\x1A\n") {
        // supercompressionScheme (u32 at 44): 0 none, 1 BasisLZ, 2 Zstandard, 3 ZLIB.
        let scheme = head
            .get(44..48)
            .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
            .unwrap_or(0);
        return if scheme == 0 { GpuTexture } else { Archive };
    }
    if starts(b"\xABKTX 11\xBB") || starts(b"DDS ") {
        return GpuTexture;
    }
    if starts(b"sB") && name.to_ascii_lowercase().ends_with(".basis") {
        return Archive;
    }
    if starts(b"glTF") {
        return Mesh;
    }
    if starts(b"wOFF") || starts(&[0, 1, 0, 0]) || starts(b"OTTO") || starts(b"true") {
        return Font;
    }

    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "avif" | "heic" => CompressedImage,
        "ogg" | "oga" | "opus" | "mp3" | "flac" | "aac" | "m4a" => CompressedAudio,
        "mp4" | "m4v" | "webm" | "mkv" | "mov" | "ivf" => Video,
        "zip" | "gz" | "zst" | "lz4" | "7z" | "xz" | "bz2" | "basis" | "woff2" | "vlz" => Archive,
        "dds" | "ktx" | "ktx2" => GpuTexture,
        "glb" | "gltf" | "meshopt" | "obj" | "fbx" | "bin" => Mesh,
        "ttf" | "otf" | "woff" => Font,
        "wav" | "aiff" | "aif" | "pcm" => Pcm,
        "json" | "md" | "txt" | "ron" | "toml" | "csv" | "vel" | "vs3" | "vcss" | "wgsl"
        | "glsl" | "hlsl" | "xml" | "yaml" | "yml" | "html" | "css" | "js" | "svg" | "po" => Text,
        _ => {
            if !head.is_empty() && std::str::from_utf8(head).is_ok() {
                Text
            } else if head.is_empty() {
                Unknown
            } else {
                Binary
            }
        }
    }
}

/// What the automatic choice optimizes for.
///
/// Each profile is a cost model in picoseconds per byte (fixed, so packing stays
/// deterministic): `cost = stored·(io + size_weight) + raw·decode(codec)`. The decode rates
/// are conservative single-core figures (RAW ≈ 12 GB/s copy, LZ4 ≈ 4 GB/s, Zstandard
/// ≈ 1.2 GB/s at any level); they rank codecs and are not measurements of a given machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Profile {
    /// Runtime on fast storage (NVMe ≈ 3 GB/s): decode speed first, LZ4 almost always.
    Speed,
    /// Default: SATA SSD / cold cache (≈ 500 MB/s): Zstandard where it saves enough to pay
    /// for its slower decode, LZ4 elsewhere.
    #[default]
    Balanced,
    /// Distribution (download, HDD, optical): size first, Zstandard at the high level.
    Size,
}

impl Profile {
    /// Parse `speed`, `balanced` or `size`.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "speed" => Some(Profile::Speed),
            "balanced" => Some(Profile::Balanced),
            "size" => Some(Profile::Size),
            _ => None,
        }
    }

    /// Name as parsed by [`Profile::parse`].
    pub fn name(self) -> &'static str {
        match self {
            Profile::Speed => "speed",
            Profile::Balanced => "balanced",
            Profile::Size => "size",
        }
    }

    fn io_ps_per_byte(self) -> u128 {
        match self {
            Profile::Speed => 333,     // ~3 GB/s
            Profile::Balanced => 2000, // ~500 MB/s
            Profile::Size => 10_000,   // ~100 MB/s
        }
    }

    fn size_weight_ps_per_byte(self) -> u128 {
        match self {
            Profile::Speed | Profile::Balanced => 0,
            Profile::Size => 50_000, // as if downloaded at ~20 MB/s
        }
    }

    fn zstd_codec(self) -> Codec {
        if self == Profile::Size {
            Codec::ZstdHigh
        } else {
            Codec::Zstd
        }
    }
}

fn decode_ps_per_byte(codec: Codec) -> u128 {
    match codec {
        Codec::Raw => 83,
        Codec::Lz4 => 250,
        Codec::Zstd | Codec::ZstdHigh => 833,
    }
}

/// How an entry's blocks are encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// The same codec for every block (blocks that would not shrink still fall back to RAW).
    Fixed(Codec),
    /// Chosen per entry from its content.
    Auto(Profile),
}

impl Default for Method {
    fn default() -> Self {
        Method::Auto(Profile::Balanced)
    }
}

/// Entries this small are stored RAW: a block record costs more than any saving.
pub const TINY_ENTRY: u64 = 64;
/// A compressed block must save at least 1/32 (~3 %) of its size, or it is stored RAW.
pub fn worth_storing(raw: usize, stored: usize) -> bool {
    stored < raw - raw / 32
}

const SAMPLE_CHUNK: usize = 32 * 1024;

/// Up to three 32 KiB slices of `data` (start, middle, end): a deterministic sample.
pub fn sample_ranges(len: u64) -> Vec<(u64, usize)> {
    let chunk = SAMPLE_CHUNK as u64;
    if len <= 3 * chunk {
        return vec![(0, len as usize)];
    }
    vec![
        (0, SAMPLE_CHUNK),
        (len / 2 - chunk / 2, SAMPLE_CHUNK),
        (len - chunk, SAMPLE_CHUNK),
    ]
}

/// The codec the automatic method picks for an entry of `raw_len` bytes whose sample is
/// `sample` (the concatenated `sample_ranges`).
pub fn choose_codec(
    class: ContentClass,
    raw_len: u64,
    sample: &[u8],
    profile: Profile,
    zstd_level: i32,
) -> Codec {
    if raw_len < TINY_ENTRY || sample.is_empty() || class.is_precompressed() {
        return Codec::Raw;
    }
    let mut buf = Vec::new();
    let mut measured = |codec: Codec| -> usize {
        match compress_block(codec, zstd_level, sample, &mut buf) {
            Ok(()) => buf.len(),
            Err(_) => sample.len(),
        }
    };
    let lz4 = measured(Codec::Lz4);
    let zstd = measured(Codec::Zstd);
    let n = sample.len() as u128;
    let raw = u128::from(raw_len);
    let io = profile.io_ps_per_byte() + profile.size_weight_ps_per_byte();
    let cost = |codec: Codec, stored_sample: usize| -> u128 {
        let stored = raw * stored_sample as u128 / n;
        stored * io + raw * decode_ps_per_byte(codec)
    };
    let mut best = (cost(Codec::Raw, sample.len()), Codec::Raw);
    if worth_storing(sample.len(), lz4) {
        best = best.min((cost(Codec::Lz4, lz4), Codec::Lz4));
    }
    let zstd_codec = profile.zstd_codec();
    if worth_storing(sample.len(), zstd) {
        best = best.min((cost(zstd_codec, zstd), zstd_codec));
    }
    best.1
}

/// Block size the automatic method uses for an entry of `raw_len` bytes: one block for small
/// resources (no per-block overhead), 256 KiB blocks (1 MiB for [`Profile::Size`] or very large
/// entries) so big ones decode in parallel and can be read partially.
pub fn auto_block_size(raw_len: u64, profile: Profile) -> u32 {
    let (single_up_to, block) = match profile {
        Profile::Size => (4 * 1024 * 1024, 1024 * 1024),
        _ if raw_len > 64 * 1024 * 1024 => (1024 * 1024, 1024 * 1024),
        _ => (1024 * 1024, 256 * 1024),
    };
    if raw_len <= single_up_to {
        (raw_len.max(1) as u32).min(MAX_BLOCK_SIZE)
    } else {
        block.clamp(MIN_BLOCK_SIZE, MAX_BLOCK_SIZE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_win_over_extensions() {
        assert_eq!(
            classify("x.bin", b"\x89PNG\r\n\x1a\n...."),
            ContentClass::CompressedImage
        );
        assert_eq!(classify("a.glb", b"glTF\x02\0\0\0"), ContentClass::Mesh);
        assert_eq!(classify("a.json", b"{\"a\":1}"), ContentClass::Text);
        assert_eq!(
            classify("clip.mp4", b"\0\0\0\x20ftypisom"),
            ContentClass::Video
        );
        let mut ktx = b"\xABKTX 20\xBB\r\n\x1A\n".to_vec();
        ktx.resize(48, 0);
        assert_eq!(classify("t.ktx2", &ktx), ContentClass::GpuTexture);
        ktx[44] = 2;
        assert_eq!(classify("t.ktx2", &ktx), ContentClass::Archive);
    }

    #[test]
    fn precompressed_and_noise_stay_raw() {
        let mut x = 0x2545_F491_4F6C_DD1Du64;
        let noise: Vec<u8> = (0..65_536)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 32) as u8
            })
            .collect();
        for p in [Profile::Speed, Profile::Balanced, Profile::Size] {
            assert_eq!(
                choose_codec(ContentClass::Binary, 65_536, &noise, p, 3),
                Codec::Raw
            );
            assert_eq!(
                choose_codec(ContentClass::CompressedImage, 1 << 20, b"aaaaaaaa", p, 3),
                Codec::Raw
            );
        }
    }

    #[test]
    fn profiles_pull_in_their_direction() {
        let text: Vec<u8> = (0..4000)
            .flat_map(|i| format!("{{\"id\":{i},\"name\":\"item{}\"}},", i % 37).into_bytes())
            .collect();
        let len = text.len() as u64;
        assert_eq!(
            choose_codec(ContentClass::Text, len, &text, Profile::Size, 3),
            Codec::ZstdHigh
        );
        assert_eq!(
            choose_codec(ContentClass::Text, len, &text, Profile::Speed, 3),
            Codec::Lz4
        );
    }

    #[test]
    fn block_sizes() {
        assert_eq!(auto_block_size(0, Profile::Balanced), 1);
        assert_eq!(auto_block_size(500, Profile::Balanced), 500);
        assert_eq!(auto_block_size(10 << 20, Profile::Balanced), 256 * 1024);
        assert_eq!(auto_block_size(100 << 20, Profile::Balanced), 1 << 20);
        assert_eq!(auto_block_size(10 << 20, Profile::Size), 1 << 20);
    }
}
