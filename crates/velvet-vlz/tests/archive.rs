//! VLZ v1 behaviour: round trips, random access, edge cases, corruption, limits, determinism,
//! concurrency, and a committed archive that pins the on-disk format across platforms.

use std::io::Write;
use std::sync::Arc;

use velvet_vlz::format::{BLOCK_RECORD_SIZE, ENTRY_RECORD_SIZE};
use velvet_vlz::{
    pack_to_vec, Codec, Header, Method, OpenOptions, PackInput, PackOptions, Profile, ReadLimits,
    VlzArchive, VlzError,
};

/// Deterministic pseudo-random bytes (xorshift), compressible to taste.
fn bytes(len: usize, seed: u64, entropy_bits: u32) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mask = if entropy_bits >= 8 {
        0xFF
    } else {
        (1u8 << entropy_bits) - 1
    };
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x as u8) & mask
        })
        .collect()
}

fn text(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 64);
    let mut i = 0u32;
    while out.len() < len {
        out.extend_from_slice(format!("{{\"id\":{i},\"tag\":\"node_{}\"}},\n", i % 53).as_bytes());
        i += 1;
    }
    out.truncate(len);
    out
}

fn sample_inputs() -> Vec<(String, Vec<u8>)> {
    vec![
        ("data/empty.bin".into(), Vec::new()),
        ("data/one.bin".into(), vec![7]),
        ("story/intro.json".into(), text(150_000)),
        ("noise/random.bin".into(), bytes(300_000, 1, 8)),
        ("tex/skewed.bin".into(), bytes(2_500_000, 2, 3)),
        ("img/fake.png".into(), {
            let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
            v.extend(bytes(70_000, 3, 8));
            v
        }),
        ("big/text.txt".into(), text(5_000_000)),
    ]
}

fn to_inputs(items: &[(String, Vec<u8>)]) -> Vec<PackInput> {
    items
        .iter()
        .map(|(n, d)| PackInput::bytes(n.clone(), d.clone()))
        .collect()
}

fn write_temp(data: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(data).unwrap();
    f.flush().unwrap();
    f
}

fn open_bytes(data: &[u8]) -> Result<VlzArchive, VlzError> {
    let f = write_temp(data);
    VlzArchive::open(f.path())
}

fn methods() -> Vec<Method> {
    vec![
        Method::Fixed(Codec::Raw),
        Method::Fixed(Codec::Lz4),
        Method::Fixed(Codec::Zstd),
        Method::Fixed(Codec::ZstdHigh),
        Method::Auto(Profile::Speed),
        Method::Auto(Profile::Balanced),
        Method::Auto(Profile::Size),
    ]
}

#[test]
fn round_trip_every_method() {
    let items = sample_inputs();
    for method in methods() {
        let opts = PackOptions {
            method,
            zstd_high_level: 12, // keeps the test quick; same decoder as 19
            ..PackOptions::default()
        };
        let (data, report) = pack_to_vec(to_inputs(&items), &opts).unwrap();
        assert_eq!(report.entries, items.len());
        assert_eq!(report.archive_bytes, data.len() as u64);
        let archive = open_bytes(&data).unwrap();
        assert_eq!(archive.len(), items.len());
        for (name, content) in &items {
            assert_eq!(&archive.read(name).unwrap(), content, "{method:?} {name}");
        }
        assert!(archive.verify().is_ok(), "{method:?}");
    }
}

#[test]
fn auto_leaves_precompressed_and_noise_raw_and_compresses_text() {
    let items = sample_inputs();
    let (data, report) = pack_to_vec(to_inputs(&items), &PackOptions::default()).unwrap();
    let archive = open_bytes(&data).unwrap();
    let codec_of = |name: &str| archive.blocks(archive.find(name).unwrap())[0].codec;
    assert_eq!(codec_of("img/fake.png"), Codec::Raw.id());
    assert_eq!(codec_of("noise/random.bin"), Codec::Raw.id());
    assert_ne!(codec_of("story/intro.json"), Codec::Raw.id());
    assert_ne!(codec_of("big/text.txt"), Codec::Raw.id());
    assert!(report.stored_bytes < report.raw_bytes / 2);
    assert_eq!(report.precompressed_entries, 1);
}

#[test]
fn random_access_reads_match_slices() {
    let content = text(3_000_000);
    let opts = PackOptions {
        method: Method::Fixed(Codec::Lz4),
        block_size: Some(64 * 1024),
        ..PackOptions::default()
    };
    let (data, _) = pack_to_vec(vec![PackInput::bytes("t.txt", content.clone())], &opts).unwrap();
    let archive = open_bytes(&data).unwrap();
    let id = archive.find("t.txt").unwrap();
    assert_eq!(archive.blocks(id).len(), 46);
    let mut x = 12_345u64;
    for _ in 0..300 {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let offset = (x >> 16) % content.len() as u64;
        let len = ((x >> 40) as usize % 200_000).min(content.len() - offset as usize);
        let got = archive.read_range(id, offset, len).unwrap();
        assert_eq!(&got[..], &content[offset as usize..offset as usize + len]);
    }
    // Block-exact, empty and edge ranges.
    assert_eq!(
        archive.read_range(id, 65_536, 65_536).unwrap(),
        &content[65_536..131_072]
    );
    assert!(archive
        .read_range(id, content.len() as u64, 0)
        .unwrap()
        .is_empty());
    assert!(matches!(
        archive.read_range(id, content.len() as u64 - 1, 2),
        Err(VlzError::Range { .. })
    ));
    assert!(matches!(
        archive.read_range(id, u64::MAX, 1),
        Err(VlzError::Range { .. })
    ));
}

#[test]
fn empty_archive_and_empty_entries() {
    let (data, report) = pack_to_vec(Vec::new(), &PackOptions::default()).unwrap();
    assert_eq!(report.entries, 0);
    let archive = open_bytes(&data).unwrap();
    assert!(archive.is_empty());
    assert!(archive.verify().is_ok());
    assert!(matches!(archive.read("x"), Err(VlzError::NotFound(_))));

    let (data, _) = pack_to_vec(
        vec![PackInput::bytes("e", Vec::new())],
        &PackOptions::default(),
    )
    .unwrap();
    let archive = open_bytes(&data).unwrap();
    let id = archive.find("e").unwrap();
    assert_eq!(archive.entry(id).block_count, 0);
    assert!(archive.read_entry(id).unwrap().is_empty());
}

#[test]
fn small_blocks_and_exact_multiples() {
    for len in [4096usize, 8192, 4095, 4097, 12_288 + 1] {
        let content = bytes(len, len as u64, 2);
        let opts = PackOptions {
            method: Method::Fixed(Codec::Zstd),
            block_size: Some(4096),
            ..PackOptions::default()
        };
        let (data, _) = pack_to_vec(vec![PackInput::bytes("b", content.clone())], &opts).unwrap();
        let archive = open_bytes(&data).unwrap();
        let id = archive.find("b").unwrap();
        assert_eq!(archive.entry(id).block_count as usize, len.div_ceil(4096));
        assert_eq!(archive.read_entry(id).unwrap(), content);
    }
}

#[test]
fn names_are_normalized_and_found_either_way() {
    let (data, _) = pack_to_vec(
        vec![PackInput::bytes(r".\dir\sub\file.txt", b"hi".to_vec())],
        &PackOptions::default(),
    )
    .unwrap();
    let archive = open_bytes(&data).unwrap();
    assert!(archive.contains("dir/sub/file.txt"));
    assert!(archive.contains(r"dir\sub\file.txt"));
    assert!(archive.contains("/dir/sub/file.txt"));
    assert!(pack_to_vec(
        vec![
            PackInput::bytes("a", vec![]),
            PackInput::bytes("./a", vec![])
        ],
        &PackOptions::default()
    )
    .is_err());
    assert!(pack_to_vec(
        vec![PackInput::bytes("../escape", vec![])],
        &PackOptions::default()
    )
    .is_err());
}

#[test]
fn packing_is_deterministic_across_thread_counts() {
    let items = sample_inputs();
    let mut previous: Option<Vec<u8>> = None;
    for threads in [1, 2, 7] {
        let opts = PackOptions {
            threads,
            window_bytes: 1_000_000, // several windows
            ..PackOptions::default()
        };
        let (data, _) = pack_to_vec(to_inputs(&items), &opts).unwrap();
        if let Some(p) = &previous {
            assert!(p == &data, "{threads} threads changed the archive");
        }
        previous = Some(data);
    }
    // Input order does not matter either.
    let mut reversed = to_inputs(&items);
    reversed.reverse();
    let (data, _) = pack_to_vec(reversed, &PackOptions::default()).unwrap();
    assert!(previous.unwrap() == data);
}

#[test]
fn files_are_streamed_and_alignment_holds() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
    let big = bytes(3_000_000, 9, 8); // incompressible: streamed RAW
    std::fs::write(dir.path().join("a/b/noise.bin"), &big).unwrap();
    std::fs::write(dir.path().join("a/readme.md"), text(40_000)).unwrap();
    let out = dir.path().join("pack.vlz");
    let opts = PackOptions {
        alignment: 4096,
        ..PackOptions::default()
    };
    velvet_vlz::pack_directory(dir.path().join("a"), &out, &opts).unwrap();
    let archive = VlzArchive::open(&out).unwrap();
    assert_eq!(archive.read("b/noise.bin").unwrap(), big);
    for e in archive.entries() {
        for b in archive.blocks(e.id) {
            assert_eq!(b.offset % 4096, 0);
        }
    }
    assert!(!dir.path().join("pack.vlz.tmp").exists());
}

#[test]
fn concurrent_reads_share_one_archive() {
    let items = sample_inputs();
    let (data, _) = pack_to_vec(to_inputs(&items), &PackOptions::default()).unwrap();
    let f = write_temp(&data);
    let archive = Arc::new(
        VlzArchive::open_with(
            f.path(),
            OpenOptions {
                parallel_threads: 3,
                parallel_min_bytes: 1,
                ..OpenOptions::default()
            },
        )
        .unwrap(),
    );
    let items = Arc::new(items);
    let handles: Vec<_> = (0..8)
        .map(|t| {
            let archive = Arc::clone(&archive);
            let items = Arc::clone(&items);
            std::thread::spawn(move || {
                for round in 0..20 {
                    let (name, content) = &items[(t + round) % items.len()];
                    assert_eq!(&archive.read(name).unwrap(), content);
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

// ---------- corruption and hostile input ----------

fn valid_archive() -> Vec<u8> {
    let opts = PackOptions {
        method: Method::Fixed(Codec::Lz4),
        block_size: Some(16 * 1024),
        ..PackOptions::default()
    };
    pack_to_vec(
        vec![
            PackInput::bytes("a.txt", text(100_000)),
            PackInput::bytes("b.bin", bytes(50_000, 4, 4)),
        ],
        &opts,
    )
    .unwrap()
    .0
}

/// Apply `edit` to the index and fix both checksums, so only structural checks can object.
fn tamper_index(data: &[u8], edit: impl FnOnce(&mut [u8], &Header)) -> Vec<u8> {
    let mut data = data.to_vec();
    let header = Header::decode(&data[..64]).unwrap();
    let start = header.index_offset as usize;
    let end = start + header.index_size as usize;
    edit(&mut data[start..end], &header);
    let fixed = Header {
        index_checksum: xxhash_rust::xxh3::xxh3_64(&data[start..end]),
        ..header
    };
    data[..64].copy_from_slice(&fixed.encode());
    data
}

fn block_field(h: &Header, block: usize, at: usize) -> usize {
    h.entry_count as usize * ENTRY_RECORD_SIZE + block * BLOCK_RECORD_SIZE + at
}

fn expect_corrupt(result: Result<VlzArchive, VlzError>, what: &str) {
    match result {
        Err(VlzError::Corrupt(_)) => {}
        Err(other) => panic!("{what}: expected Corrupt, got {other}"),
        Ok(_) => panic!("{what}: accepted"),
    }
}

#[test]
fn header_corruption_is_detected() {
    let good = valid_archive();
    let mut bad = good.clone();
    bad[0] = b'X';
    assert!(matches!(open_bytes(&bad), Err(VlzError::BadMagic)));

    let mut bad = good.clone();
    bad[20] ^= 0x01; // index_offset, checksum not updated
    assert!(matches!(open_bytes(&bad), Err(VlzError::Checksum(m)) if m == "header"));

    let mut bad = good.clone();
    let idx = Header::decode(&bad[..64]).unwrap().index_offset as usize;
    bad[idx + 3] ^= 0x40;
    assert!(matches!(open_bytes(&bad), Err(VlzError::Checksum(m)) if m == "index"));
}

#[test]
fn incompatible_versions_and_flags_are_refused() {
    let good = valid_archive();
    let header = Header::decode(&good[..64]).unwrap();
    let mut bad = good.clone();
    bad[..64].copy_from_slice(
        &Header {
            version_major: 2,
            ..header
        }
        .encode(),
    );
    assert!(matches!(
        open_bytes(&bad),
        Err(VlzError::IncompatibleVersion { major: 2, .. })
    ));

    let mut newer_minor = good.clone();
    newer_minor[..64].copy_from_slice(
        &Header {
            version_minor: 7,
            ..header
        }
        .encode(),
    );
    assert!(
        open_bytes(&newer_minor).is_ok(),
        "a newer minor must stay readable"
    );

    let mut bad = good.clone();
    bad[..64].copy_from_slice(
        &Header {
            flags: 1 << 5,
            ..header
        }
        .encode(),
    );
    assert!(matches!(
        open_bytes(&bad),
        Err(VlzError::UnsupportedFlags(_))
    ));
}

#[test]
fn block_checksum_catches_payload_damage() {
    let mut bad = valid_archive();
    let header = Header::decode(&bad[..64]).unwrap();
    let archive = open_bytes(&bad).unwrap();
    let first = archive.blocks(archive.find("a.txt").unwrap())[0];
    drop(archive);
    let _ = header;
    bad[first.offset as usize + 5] ^= 0xFF;
    let archive = open_bytes(&bad).unwrap(); // the index is intact
    assert!(matches!(archive.read("a.txt"), Err(VlzError::Checksum(_))));
    assert!(archive.read("b.bin").is_ok(), "other entries stay readable");
    let report = archive.verify();
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].0, "a.txt");
}

#[test]
fn truncation_at_any_length_fails_cleanly() {
    let good = valid_archive();
    let step = (good.len() / 300).max(1);
    let mut cut = 0;
    while cut < good.len() {
        let result = open_bytes(&good[..cut]);
        if let Ok(archive) = result {
            panic!(
                "truncated to {cut} of {} bytes but opened ({} entries)",
                good.len(),
                archive.len()
            );
        }
        cut += step;
    }
}

#[test]
fn invalid_offsets_sizes_and_names_are_refused() {
    let good = valid_archive();
    // A block pointing into the index.
    let bad = tamper_index(&good, |idx, h| {
        let at = block_field(h, 0, 0);
        idx[at..at + 8].copy_from_slice(&(h.index_offset - 8).to_le_bytes());
    });
    expect_corrupt(open_bytes(&bad), "block into the index");
    // A block before the end of the header.
    let bad = tamper_index(&good, |idx, h| {
        let at = block_field(h, 0, 0);
        idx[at..at + 8].copy_from_slice(&16u64.to_le_bytes());
    });
    expect_corrupt(open_bytes(&bad), "block in the header");
    // Raw size above the block limit (decompression bomb).
    let bad = tamper_index(&good, |idx, h| {
        let at = block_field(h, 0, 12);
        idx[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    });
    expect_corrupt(open_bytes(&bad), "oversized block");
    // Raw size inflated within the limit: the entry no longer adds up.
    let bad = tamper_index(&good, |idx, h| {
        let at = block_field(h, 0, 12);
        idx[at..at + 4].copy_from_slice(&(15u32 << 20).to_le_bytes());
    });
    expect_corrupt(open_bytes(&bad), "inflated block");
    // A RAW block whose stored and raw sizes differ.
    let bad = tamper_index(&good, |idx, h| {
        idx[block_field(h, 0, 24)] = 0;
    });
    expect_corrupt(open_bytes(&bad), "raw size mismatch");
    // A name escaping the extraction folder.
    let bad = tamper_index(&good, |idx, h| {
        let names =
            h.entry_count as usize * ENTRY_RECORD_SIZE + h.block_count as usize * BLOCK_RECORD_SIZE;
        idx[names..names + 3].copy_from_slice(b"../"); // "a.txt" becomes "../xt"
    });
    expect_corrupt(open_bytes(&bad), "path traversal name");
    // A name outside the name table.
    let bad = tamper_index(&good, |idx, _| {
        idx[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    });
    expect_corrupt(open_bytes(&bad), "name out of range");
    // An entry claiming blocks beyond the table.
    let bad = tamper_index(&good, |idx, _| {
        idx[28..32].copy_from_slice(&1_000_000u32.to_le_bytes());
    });
    expect_corrupt(open_bytes(&bad), "block range out of the table");
    // An index longer than the file.
    let mut bad = good.clone();
    let h = Header::decode(&bad[..64]).unwrap();
    bad[..64].copy_from_slice(
        &Header {
            index_size: h.index_size + 4096,
            ..h
        }
        .encode(),
    );
    assert!(open_bytes(&bad).is_err());
}

#[test]
fn unknown_codecs_are_reported_not_misread() {
    let good = valid_archive();
    let bad = tamper_index(&good, |idx, h| {
        idx[block_field(h, 0, 24)] = 200;
    });
    let archive = open_bytes(&bad).unwrap();
    assert!(matches!(
        archive.read("a.txt"),
        Err(VlzError::UnsupportedCodec(200))
    ));
}

#[test]
fn read_limit_stops_huge_allocations() {
    let (data, _) = pack_to_vec(
        vec![PackInput::bytes("big", text(2_000_000))],
        &PackOptions::default(),
    )
    .unwrap();
    let f = write_temp(&data);
    let archive = VlzArchive::open_with(
        f.path(),
        OpenOptions {
            limits: ReadLimits {
                max_entry_size: 1_000_000,
            },
            ..OpenOptions::default()
        },
    )
    .unwrap();
    assert!(matches!(archive.read("big"), Err(VlzError::Limit(_))));
    let id = archive.find("big").unwrap();
    assert_eq!(archive.read_range(id, 0, 100).unwrap().len(), 100);
}

#[test]
fn random_mutations_never_panic() {
    let good = valid_archive();
    let mut x = 0xDEAD_BEEFu64;
    for round in 0..400 {
        let mut bad = good.clone();
        for _ in 0..1 + round % 4 {
            x = x
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1442695040888963407);
            let at = (x >> 33) as usize % bad.len();
            bad[at] ^= (x >> 8) as u8 | 1;
        }
        // Half the time fix the checksums so the structural checks are exercised too.
        if round % 2 == 0 {
            if let Ok(h) = Header::decode(&bad[..64]) {
                let start = h.index_offset as usize;
                if let Some(end) = start
                    .checked_add(h.index_size as usize)
                    .filter(|&e| e <= bad.len())
                {
                    let fixed = Header {
                        index_checksum: xxhash_rust::xxh3::xxh3_64(&bad[start..end]),
                        ..h
                    };
                    bad[..64].copy_from_slice(&fixed.encode());
                }
            }
        }
        if let Ok(archive) = open_bytes(&bad) {
            for e in archive.entries().collect::<Vec<_>>() {
                let _ = archive.read_entry(e.id);
                let _ = archive.read_range(e.id, e.raw_size / 3, (e.raw_size / 3) as usize);
            }
        }
    }
}

// ---------- the format, pinned ----------

const GOLDEN: &[u8] = include_bytes!("fixtures/golden_v1.vlz");

fn golden_inputs() -> Vec<PackInput> {
    vec![
        PackInput::bytes("hello.txt", b"Hello, Velvet!\n".repeat(64)),
        PackInput::bytes("raw/bytes.bin", (0u8..=255).collect::<Vec<_>>()),
        PackInput::bytes("empty", Vec::new()),
    ]
}

fn golden_options() -> PackOptions {
    PackOptions {
        method: Method::Fixed(Codec::Lz4),
        block_size: Some(4096),
        ..PackOptions::default()
    }
}

/// The committed archive must open and decode on every platform: its fields are fixed
/// little-endian bytes. Regenerate (after a deliberate format change) with
/// `VLZ_WRITE_GOLDEN=1 cargo test -p velvet-vlz --test archive golden`.
#[test]
fn golden_archive_reads_and_rebuilds_byte_for_byte() {
    let (fresh, _) = pack_to_vec(golden_inputs(), &golden_options()).unwrap();
    if std::env::var_os("VLZ_WRITE_GOLDEN").is_some() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_v1.vlz");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, &fresh).unwrap();
        return;
    }
    let archive = open_bytes(GOLDEN).unwrap();
    assert_eq!(archive.header().version_major, 1);
    assert_eq!(archive.len(), 3);
    assert_eq!(
        archive.read("hello.txt").unwrap(),
        b"Hello, Velvet!\n".repeat(64)
    );
    assert_eq!(
        archive.read("raw/bytes.bin").unwrap(),
        (0u8..=255).collect::<Vec<_>>()
    );
    assert!(archive.read("empty").unwrap().is_empty());
    assert_eq!(&GOLDEN[0..4], b"VLZ\x1A");
    assert_eq!(&GOLDEN[4..8], &[1, 0, 0, 0]);
    assert!(
        fresh == GOLDEN,
        "the writer no longer produces the committed v1 archive"
    );
}
