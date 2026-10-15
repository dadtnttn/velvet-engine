//! Writing VLZ archives.
//!
//! Packing is deterministic: entries are sorted by name, the automatic codec choice uses no
//! clock, and blocks compressed in parallel are written in index order, so the same inputs
//! and options give the same bytes whatever the thread count. Memory stays bounded: entries
//! are processed in windows of `window_bytes`, and files stored RAW are copied block by block
//! instead of being loaded whole.

use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use xxhash_rust::xxh3::{xxh3_64, Xxh3};

use crate::classify::{
    auto_block_size, choose_codec, classify, sample_ranges, worth_storing, ContentClass, Method,
    Profile,
};
use crate::codec::{compress_block, Codec, DEFAULT_ZSTD_HIGH_LEVEL, DEFAULT_ZSTD_LEVEL};
use crate::error::{Result, VlzError};
use crate::format::{
    normalize_name, BlockRecord, EntryRecord, Header, HEADER_SIZE, MAX_ALIGNMENT, MAX_BLOCK_SIZE,
    MIN_BLOCK_SIZE, VERSION_MAJOR, VERSION_MINOR,
};
use crate::reader::CodecTotals;

/// How to pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackOptions {
    /// Codec choice.
    pub method: Method,
    /// Uniform block size for every entry (`None`: chosen per entry, see
    /// [`auto_block_size`]).
    pub block_size: Option<u32>,
    /// Alignment of every block payload (power of two, 1–65536). 16 by default; 4096 suits
    /// memory mapping or direct uploads of RAW blocks.
    pub alignment: u32,
    /// Compression threads (0 = all cores). Does not change the output.
    pub threads: usize,
    /// Level of [`Codec::Zstd`].
    pub zstd_level: i32,
    /// Level of [`Codec::ZstdHigh`].
    pub zstd_high_level: i32,
    /// Uncompressed bytes held in memory at once while packing.
    pub window_bytes: usize,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            method: Method::default(),
            block_size: None,
            alignment: 16,
            threads: 0,
            zstd_level: DEFAULT_ZSTD_LEVEL,
            zstd_high_level: DEFAULT_ZSTD_HIGH_LEVEL,
            window_bytes: 256 * 1024 * 1024,
        }
    }
}

impl PackOptions {
    fn validate(&self) -> Result<()> {
        if self.alignment == 0
            || !self.alignment.is_power_of_two()
            || self.alignment > MAX_ALIGNMENT
        {
            return Err(VlzError::Invalid(format!(
                "alignment {} (power of two up to {MAX_ALIGNMENT})",
                self.alignment
            )));
        }
        if let Some(bs) = self.block_size {
            if !(MIN_BLOCK_SIZE..=MAX_BLOCK_SIZE).contains(&bs) {
                return Err(VlzError::Invalid(format!(
                    "block size {bs} (from {MIN_BLOCK_SIZE} to {MAX_BLOCK_SIZE})"
                )));
            }
        }
        let levels = zstd::zstd_safe::min_c_level()..=zstd::zstd_safe::max_c_level();
        for level in [self.zstd_level, self.zstd_high_level] {
            if !levels.contains(&level) {
                return Err(VlzError::Invalid(format!("zstd level {level}")));
            }
        }
        Ok(())
    }

    fn level(&self, codec: Codec) -> i32 {
        match codec {
            Codec::Zstd => self.zstd_level,
            Codec::ZstdHigh => self.zstd_high_level,
            Codec::Raw | Codec::Lz4 => 0,
        }
    }

    fn threads(&self) -> usize {
        if self.threads == 0 {
            std::thread::available_parallelism().map_or(1, |n| n.get())
        } else {
            self.threads
        }
    }
}

/// Where an input's bytes come from.
#[derive(Debug, Clone)]
pub enum PackSource {
    /// In memory.
    Bytes(Vec<u8>),
    /// A file read while packing.
    File(PathBuf),
}

/// One resource to pack.
#[derive(Debug, Clone)]
pub struct PackInput {
    /// Entry name (normalized when packing).
    pub name: String,
    /// Its bytes.
    pub source: PackSource,
}

impl PackInput {
    /// From memory.
    pub fn bytes(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            source: PackSource::Bytes(data.into()),
        }
    }

    /// From a file.
    pub fn file(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            source: PackSource::File(path.into()),
        }
    }
}

/// What a pack produced.
#[derive(Debug, Clone, Default)]
pub struct PackReport {
    /// Entries written.
    pub entries: usize,
    /// Sum of the uncompressed sizes.
    pub raw_bytes: u64,
    /// Sum of the stored sizes.
    pub stored_bytes: u64,
    /// Final archive size.
    pub archive_bytes: u64,
    /// Per built-in codec, by id.
    pub per_codec: [CodecTotals; 4],
    /// Entries whose content class made the automatic method skip compression.
    pub precompressed_entries: usize,
}

/// Every file under `dir` (recursively, symlinks skipped) as inputs named by their path
/// relative to `dir`, sorted.
pub fn collect_directory(dir: impl AsRef<Path>) -> Result<Vec<PackInput>> {
    let dir = dir.as_ref();
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(here) = stack.pop() {
        let mut items: Vec<_> = std::fs::read_dir(&here)?.collect::<std::io::Result<_>>()?;
        items.sort_by_key(|e| e.file_name());
        for item in items {
            let kind = item.file_type()?;
            let path = item.path();
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                let rel = path.strip_prefix(dir).map_err(|_| {
                    VlzError::Invalid(format!("{} outside {}", path.display(), dir.display()))
                })?;
                let name = rel
                    .to_str()
                    .ok_or_else(|| VlzError::Invalid(format!("{} is not UTF-8", rel.display())))?;
                out.push(PackInput::file(name.replace('\\', "/"), path));
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Pack a directory into `out` (written to a temporary file, then renamed).
pub fn pack_directory(
    dir: impl AsRef<Path>,
    out: impl AsRef<Path>,
    opts: &PackOptions,
) -> Result<PackReport> {
    pack_to_file(collect_directory(dir)?, out, opts)
}

/// Pack into a file (written to `<out>.tmp`, then renamed over `out`).
pub fn pack_to_file(
    inputs: Vec<PackInput>,
    out: impl AsRef<Path>,
    opts: &PackOptions,
) -> Result<PackReport> {
    let out = out.as_ref();
    let mut tmp = out.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let result = (|| {
        let file = File::create(&tmp)?;
        let mut w = BufWriter::with_capacity(1 << 20, file);
        let report = pack(inputs, &mut w, opts)?;
        let file = w.into_inner().map_err(|e| VlzError::Io(e.into_error()))?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&tmp, out)?;
        Ok(report)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Pack into memory.
pub fn pack_to_vec(inputs: Vec<PackInput>, opts: &PackOptions) -> Result<(Vec<u8>, PackReport)> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    let report = pack(inputs, &mut cursor, opts)?;
    Ok((cursor.into_inner(), report))
}

struct Prepared {
    name: String,
    raw_len: u64,
    source: PackSource,
}

struct Plan {
    class: ContentClass,
    codec: Codec,
    block_size: u32,
    /// Loaded content (absent for a file streamed RAW).
    data: Option<Vec<u8>>,
}

/// Pack into any seekable writer.
pub fn pack<W: Write + Seek>(
    inputs: Vec<PackInput>,
    mut out: W,
    opts: &PackOptions,
) -> Result<PackReport> {
    opts.validate()?;
    let mut prepared = Vec::with_capacity(inputs.len());
    for input in inputs {
        let name = normalize_name(&input.name)?;
        let raw_len = match &input.source {
            PackSource::Bytes(b) => b.len() as u64,
            PackSource::File(p) => std::fs::metadata(p)
                .map_err(|e| VlzError::Invalid(format!("{}: {e}", p.display())))?
                .len(),
        };
        prepared.push(Prepared {
            name,
            raw_len,
            source: input.source,
        });
    }
    prepared.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));
    if let Some(w) = prepared.windows(2).find(|w| w[0].name == w[1].name) {
        return Err(VlzError::Invalid(format!("duplicate entry {}", w[0].name)));
    }

    let start = out.stream_position()?;
    out.write_all(&[0u8; HEADER_SIZE])?;
    let mut pos = HEADER_SIZE as u64;
    let mut names = String::new();
    let mut entries = Vec::with_capacity(prepared.len());
    let mut blocks: Vec<BlockRecord> = Vec::new();
    let mut report = PackReport {
        entries: prepared.len(),
        ..PackReport::default()
    };
    let threads = opts.threads();

    let mut i = 0;
    while i < prepared.len() {
        // A window: consecutive entries up to window_bytes (at least one).
        let mut j = i;
        let mut held = 0u64;
        while j < prepared.len()
            && (j == i || held + prepared[j].raw_len <= opts.window_bytes as u64)
        {
            held += prepared[j].raw_len;
            j += 1;
        }
        let window: &[Prepared] = &prepared[i..j];

        let plans: Vec<Plan> =
            parallel_map(window.len(), threads, |k| plan_entry(&window[k], opts))
                .into_iter()
                .collect::<Result<_>>()?;

        // Every compressed block of the window, in order, compressed in parallel.
        let mut jobs = Vec::new();
        for (k, plan) in plans.iter().enumerate() {
            if plan.codec == Codec::Raw || content(plan, &window[k]).is_none() {
                continue;
            }
            let len = window[k].raw_len;
            let bs = u64::from(plan.block_size);
            let mut at = 0;
            while at < len {
                let n = bs.min(len - at);
                jobs.push((k, at as usize, n as usize));
                at += n;
            }
        }
        let compressed: Vec<(u8, i8, Vec<u8>)> = parallel_map(jobs.len(), threads, |q| {
            let (k, at, n) = jobs[q];
            let plan = &plans[k];
            let raw = &content(plan, &window[k]).expect("loaded")[at..at + n];
            let level = opts.level(plan.codec);
            let mut buf = Vec::new();
            compress_block(plan.codec, level, raw, &mut buf)?;
            if worth_storing(raw.len(), buf.len()) {
                Ok((plan.codec.id(), level.clamp(-128, 127) as i8, buf))
            } else {
                Ok((Codec::Raw.id(), 0, Vec::new()))
            }
        })
        .into_iter()
        .collect::<Result<_>>()?;
        let mut compressed = compressed.into_iter();

        for (k, plan) in plans.into_iter().enumerate() {
            let item = &window[k];
            let first_block = blocks.len() as u32;
            let name_offset = names.len() as u32;
            names.push_str(&item.name);
            let mut hasher = Xxh3::new();
            let mut stored_total = 0u64;
            let bs = u64::from(plan.block_size);
            let data = content(&plan, item);
            let mut reader = match (data, &item.source) {
                (None, PackSource::File(p)) => Some(
                    File::open(p)
                        .map_err(|e| VlzError::Invalid(format!("{}: {e}", p.display())))?,
                ),
                _ => None,
            };
            let mut chunk = Vec::new();
            let mut packed: Vec<u8>;
            let mut at = 0u64;
            while at < item.raw_len {
                let n = bs.min(item.raw_len - at) as usize;
                let raw: &[u8] = match data {
                    Some(d) => &d[at as usize..at as usize + n],
                    None => {
                        chunk.resize(n, 0);
                        reader
                            .as_mut()
                            .expect("streamed file")
                            .read_exact(&mut chunk)
                            .map_err(|e| {
                                VlzError::Invalid(format!(
                                    "{}: changed while packing: {e}",
                                    item.name
                                ))
                            })?;
                        &chunk
                    }
                };
                hasher.update(raw);
                let (codec, level, payload): (u8, i8, &[u8]) =
                    if plan.codec == Codec::Raw || data.is_none() {
                        (Codec::Raw.id(), 0, raw)
                    } else {
                        let (c, l, buf) = compressed.next().expect("one result per job");
                        if c == Codec::Raw.id() {
                            (c, l, raw)
                        } else {
                            packed = buf;
                            (c, l, &packed[..])
                        }
                    };
                let pad = (u64::from(opts.alignment) - pos % u64::from(opts.alignment))
                    % u64::from(opts.alignment);
                if pad > 0 {
                    out.write_all(&vec![0u8; pad as usize])?;
                    pos += pad;
                }
                out.write_all(payload)?;
                let record = BlockRecord {
                    offset: pos,
                    stored_size: payload.len() as u32,
                    raw_size: n as u32,
                    checksum: xxh3_64(payload),
                    codec,
                    level,
                };
                pos += payload.len() as u64;
                stored_total += payload.len() as u64;
                let totals = &mut report.per_codec[codec as usize];
                totals.blocks += 1;
                totals.raw_bytes += n as u64;
                totals.stored_bytes += payload.len() as u64;
                blocks.push(record);
                at += n as u64;
            }
            if plan.codec == Codec::Raw && plan.class.is_precompressed() {
                report.precompressed_entries += 1;
            }
            let block_count = blocks.len() as u32 - first_block;
            entries.push(EntryRecord {
                name_offset,
                name_len: item.name.len() as u32,
                raw_size: item.raw_len,
                stored_size: stored_total,
                first_block,
                block_count,
                content_hash: hasher.digest(),
                content_class: plan.class as u16,
                flags: 0,
                block_raw_size: if block_count == 0 { 0 } else { plan.block_size },
            });
            report.raw_bytes += item.raw_len;
            report.stored_bytes += stored_total;
        }
        // Free the window's sources before the next one.
        for item in &mut prepared[i..j] {
            item.source = PackSource::Bytes(Vec::new());
        }
        i = j;
    }

    let pad = (8 - pos % 8) % 8;
    out.write_all(&vec![0u8; pad as usize])?;
    pos += pad;
    let mut index = Vec::with_capacity(entries.len() * 48 + blocks.len() * 32 + names.len());
    for e in &entries {
        e.encode_into(&mut index);
    }
    for b in &blocks {
        b.encode_into(&mut index);
    }
    index.extend_from_slice(names.as_bytes());
    out.write_all(&index)?;
    let header = Header {
        version_major: VERSION_MAJOR,
        version_minor: VERSION_MINOR,
        header_size: HEADER_SIZE as u32,
        flags: 0,
        index_offset: pos,
        index_size: index.len() as u64,
        entry_count: u32::try_from(entries.len())
            .map_err(|_| VlzError::Limit("too many entries".into()))?,
        block_count: u32::try_from(blocks.len())
            .map_err(|_| VlzError::Limit("too many blocks".into()))?,
        alignment: opts.alignment,
        block_size: opts.block_size.unwrap_or(256 * 1024),
        index_checksum: xxh3_64(&index),
    };
    let end = pos + index.len() as u64;
    out.seek(SeekFrom::Start(start))?;
    out.write_all(&header.encode())?;
    out.seek(SeekFrom::Start(start + end))?;
    out.flush()?;
    report.archive_bytes = end;
    Ok(report)
}

fn plan_entry(item: &Prepared, opts: &PackOptions) -> Result<Plan> {
    let profile = match opts.method {
        Method::Auto(p) => p,
        Method::Fixed(_) => Profile::Balanced,
    };
    let block_size = opts
        .block_size
        .unwrap_or_else(|| auto_block_size(item.raw_len, profile));
    let (head, sample) = match &item.source {
        PackSource::Bytes(d) => {
            let head = d[..d.len().min(64)].to_vec();
            let mut sample = Vec::new();
            for (at, n) in sample_ranges(d.len() as u64) {
                sample.extend_from_slice(&d[at as usize..at as usize + n]);
            }
            (head, sample)
        }
        PackSource::File(p) => {
            let mut f =
                File::open(p).map_err(|e| VlzError::Invalid(format!("{}: {e}", p.display())))?;
            let mut sample = Vec::new();
            for (at, n) in sample_ranges(item.raw_len) {
                let mut buf = vec![0u8; n];
                f.seek(SeekFrom::Start(at))?;
                f.read_exact(&mut buf)?;
                sample.extend_from_slice(&buf);
            }
            (sample[..sample.len().min(64)].to_vec(), sample)
        }
    };
    let class = classify(&item.name, &head);
    let codec = match opts.method {
        Method::Fixed(c) => c,
        Method::Auto(p) => choose_codec(class, item.raw_len, &sample, p, opts.zstd_level),
    };
    let data = match (&item.source, codec) {
        (PackSource::Bytes(_), _) | (PackSource::File(_), Codec::Raw) => None,
        (PackSource::File(p), _) => {
            let d =
                std::fs::read(p).map_err(|e| VlzError::Invalid(format!("{}: {e}", p.display())))?;
            if d.len() as u64 != item.raw_len {
                return Err(VlzError::Invalid(format!(
                    "{}: changed while packing",
                    item.name
                )));
            }
            Some(d)
        }
    };
    Ok(Plan {
        class,
        codec,
        block_size,
        data,
    })
}

/// The entry's content when it is in memory (given as bytes, or a file loaded to compress).
fn content<'a>(plan: &'a Plan, item: &'a Prepared) -> Option<&'a [u8]> {
    match (&plan.data, &item.source) {
        (Some(d), _) => Some(d),
        (None, PackSource::Bytes(d)) => Some(d),
        (None, PackSource::File(_)) => None,
    }
}

/// `f(0..n)` on up to `threads` threads; results in index order.
fn parallel_map<T: Send>(n: usize, threads: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    if n == 0 {
        return Vec::new();
    }
    let threads = threads.clamp(1, n);
    if threads == 1 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let slots: Mutex<Vec<Option<T>>> = Mutex::new((0..n).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= n {
                    break;
                }
                let value = f(i);
                slots.lock().unwrap_or_else(|p| p.into_inner())[i] = Some(value);
            });
        }
    });
    slots
        .into_inner()
        .unwrap_or_else(|p| p.into_inner())
        .into_iter()
        .map(|v| v.expect("every index ran"))
        .collect()
}
