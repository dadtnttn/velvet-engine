//! Reading VLZ archives: lookup, whole and partial reads, parallel block decode, verification.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use xxhash_rust::xxh3::xxh3_64;

use crate::classify::ContentClass;
use crate::codec::{Codec, DecoderRegistry};
use crate::error::{corrupt, Result, VlzError};
use crate::format::{normalize_name, BlockRecord, EntryRecord, Header, Index, HEADER_SIZE};
use crate::io::PositionalFile;
use crate::pool::{BufferPool, PoolStats};

/// Safety limits applied while reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadLimits {
    /// Largest entry a whole-entry read will allocate (bigger ones can still be read by range).
    pub max_entry_size: u64,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_entry_size: 4 * 1024 * 1024 * 1024,
        }
    }
}

/// How an archive is opened and read.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    /// Safety limits.
    pub limits: ReadLimits,
    /// Check each block's stored bytes against its checksum (XXH3, ~30 GB/s: on by default).
    pub verify_checksums: bool,
    /// Also hash every whole-entry read against the entry's content hash.
    pub verify_content: bool,
    /// Threads one archive may use to decode the blocks of large entries, counting the caller
    /// (1 = never spawn). Shared by all concurrent reads of the archive.
    pub parallel_threads: usize,
    /// Entries smaller than this decode on the calling thread only.
    pub parallel_min_bytes: u64,
    /// Idle capacity kept by the stored-bytes buffer pool.
    pub pool_bytes: usize,
    /// Decoders for codec ids beyond the built-in ones.
    pub decoders: DecoderRegistry,
}

impl Default for OpenOptions {
    fn default() -> Self {
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        Self {
            limits: ReadLimits::default(),
            verify_checksums: true,
            verify_content: false,
            parallel_threads: cores.clamp(1, 4),
            parallel_min_bytes: 1024 * 1024,
            pool_bytes: 32 * 1024 * 1024,
            decoders: DecoderRegistry::default(),
        }
    }
}

/// Index of an entry inside one archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(pub u32);

/// Public view of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryInfo<'a> {
    /// Id inside the archive.
    pub id: EntryId,
    /// Normalized name (`/` separators).
    pub name: &'a str,
    /// Uncompressed size.
    pub raw_size: u64,
    /// Stored size (all blocks).
    pub stored_size: u64,
    /// Number of blocks.
    pub block_count: u32,
    /// Uncompressed size of each block but the last.
    pub block_raw_size: u32,
    /// What the content is.
    pub content_class: ContentClass,
    /// XXH3-64 of the uncompressed content.
    pub content_hash: u64,
}

/// Result of [`VlzArchive::verify`].
#[derive(Debug, Clone, Default)]
pub struct VerifyReport {
    /// Entries checked.
    pub entries: usize,
    /// Blocks checked.
    pub blocks: usize,
    /// Uncompressed bytes decoded.
    pub raw_bytes: u64,
    /// Entries that failed, with the reason.
    pub failures: Vec<(String, String)>,
}

impl VerifyReport {
    /// No failures.
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Totals for one codec id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CodecTotals {
    /// Blocks.
    pub blocks: u64,
    /// Uncompressed bytes.
    pub raw_bytes: u64,
    /// Stored bytes.
    pub stored_bytes: u64,
}

/// Result of [`VlzArchive::stats`].
#[derive(Debug, Clone, Default)]
pub struct ArchiveStats {
    /// Archive size on disk.
    pub file_bytes: u64,
    /// Index size.
    pub index_bytes: u64,
    /// Entries.
    pub entries: usize,
    /// Blocks.
    pub blocks: usize,
    /// Sum of the uncompressed sizes.
    pub raw_bytes: u64,
    /// Sum of the stored sizes.
    pub stored_bytes: u64,
    /// Per codec id (only ids present).
    pub per_codec: Vec<(u8, CodecTotals)>,
    /// Per content class: (class, entries, raw, stored).
    pub per_class: Vec<(ContentClass, u64, u64, u64)>,
}

/// An open VLZ archive. `Sync`: share it (e.g. in an `Arc`) and read from many threads.
#[derive(Debug)]
pub struct VlzArchive {
    file: PositionalFile,
    path: Option<PathBuf>,
    header: Header,
    index: Index,
    opts: OpenOptions,
    pool: BufferPool,
    helpers_busy: AtomicUsize,
}

impl VlzArchive {
    /// Open with default options.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(path, OpenOptions::default())
    }

    /// Open with options.
    pub fn open_with(path: impl AsRef<Path>, opts: OpenOptions) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut archive = Self::from_file(file, opts)?;
        archive.path = Some(path.to_path_buf());
        Ok(archive)
    }

    /// Read the header and index from an already open file.
    pub fn from_file(file: File, opts: OpenOptions) -> Result<Self> {
        let file = PositionalFile::new(file)?;
        if file.len() < HEADER_SIZE as u64 {
            return Err(corrupt("file shorter than the VLZ header"));
        }
        let mut head = [0u8; HEADER_SIZE];
        file.read_exact_at(&mut head, 0)?;
        let header = Header::decode(&head)?;
        let end = header
            .index_offset
            .checked_add(header.index_size)
            .ok_or_else(|| corrupt("index range overflows"))?;
        if end > file.len() {
            return Err(corrupt(format!(
                "index ends at {end} but the file has {} bytes (truncated?)",
                file.len()
            )));
        }
        let mut raw_index = vec![0u8; header.index_size as usize];
        file.read_exact_at(&mut raw_index, header.index_offset)?;
        let index = Index::parse(&raw_index, &header, file.len())?;
        let pool = BufferPool::new(opts.pool_bytes);
        Ok(Self {
            file,
            path: None,
            header,
            index,
            opts,
            pool,
            helpers_busy: AtomicUsize::new(0),
        })
    }

    /// Header as read.
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Path it was opened from.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Size of the archive file.
    pub fn file_size(&self) -> u64 {
        self.file.len()
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.index.entries.len()
    }

    /// No entries.
    pub fn is_empty(&self) -> bool {
        self.index.entries.is_empty()
    }

    /// Find an entry by name (binary search on the sorted index). Accepts `\` separators and
    /// a leading `./` or `/`.
    pub fn find(&self, name: &str) -> Option<EntryId> {
        let search = |key: &str| {
            self.index
                .entries
                .binary_search_by(|e| self.index.name(e).as_bytes().cmp(key.as_bytes()))
                .ok()
                .map(|i| EntryId(i as u32))
        };
        search(name).or_else(|| {
            let normal = normalize_name(name).ok()?;
            if normal == name {
                None
            } else {
                search(&normal)
            }
        })
    }

    /// Whether an entry exists.
    pub fn contains(&self, name: &str) -> bool {
        self.find(name).is_some()
    }

    /// Entry by id. Panics on an id from another archive that is out of range.
    pub fn entry(&self, id: EntryId) -> EntryInfo<'_> {
        let e = &self.index.entries[id.0 as usize];
        EntryInfo {
            id,
            name: self.index.name(e),
            raw_size: e.raw_size,
            stored_size: e.stored_size,
            block_count: e.block_count,
            block_raw_size: e.block_raw_size,
            content_class: ContentClass::from_id(e.content_class),
            content_hash: e.content_hash,
        }
    }

    /// All entries, sorted by name.
    pub fn entries(&self) -> impl Iterator<Item = EntryInfo<'_>> + '_ {
        (0..self.index.entries.len() as u32).map(move |i| self.entry(EntryId(i)))
    }

    /// The blocks of an entry.
    pub fn blocks(&self, id: EntryId) -> &[BlockRecord] {
        self.blocks_of(&self.index.entries[id.0 as usize])
    }

    fn blocks_of(&self, e: &EntryRecord) -> &[BlockRecord] {
        let first = e.first_block as usize;
        &self.index.blocks[first..first + e.block_count as usize]
    }

    fn record(&self, name: &str) -> Result<EntryId> {
        self.find(name)
            .ok_or_else(|| VlzError::NotFound(name.to_owned()))
    }

    /// Read a whole entry by name.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        self.read_entry(self.record(name)?)
    }

    /// Read a whole entry.
    pub fn read_entry(&self, id: EntryId) -> Result<Vec<u8>> {
        let e = self.index.entries[id.0 as usize];
        if e.raw_size > self.opts.limits.max_entry_size {
            return Err(VlzError::Limit(format!(
                "{} is {} bytes, above the {}-byte read limit",
                self.index.name(&e),
                e.raw_size,
                self.opts.limits.max_entry_size
            )));
        }
        let size = usize::try_from(e.raw_size)
            .map_err(|_| VlzError::Limit("entry larger than the address space".into()))?;
        let mut out = vec![0u8; size];
        self.read_entry_into(id, &mut out)?;
        Ok(out)
    }

    /// Read a whole entry into `dst`, which must be exactly its size.
    pub fn read_entry_into(&self, id: EntryId, dst: &mut [u8]) -> Result<()> {
        let e = self.index.entries[id.0 as usize];
        if dst.len() as u64 != e.raw_size {
            return Err(VlzError::Invalid(format!(
                "buffer of {} bytes for an entry of {}",
                dst.len(),
                e.raw_size
            )));
        }
        let blocks = self.blocks_of(&e);
        let name = self.index.name(&e);
        let parallel = blocks.len() > 1
            && e.raw_size >= self.opts.parallel_min_bytes
            && self.opts.parallel_threads > 1;
        let helpers = if parallel {
            self.reserve_helpers(blocks.len() - 1)
        } else {
            0
        };
        let result = if helpers == 0 {
            let mut rest = &mut *dst;
            let mut outcome = Ok(());
            for (i, b) in blocks.iter().enumerate() {
                let (here, tail) = rest.split_at_mut(b.raw_size as usize);
                if let Err(err) = self.decode_block(name, i, b, here) {
                    outcome = Err(err);
                    break;
                }
                rest = tail;
            }
            outcome
        } else {
            let r = self.decode_parallel(name, blocks, dst, helpers);
            self.helpers_busy.fetch_sub(helpers, Ordering::AcqRel);
            r
        };
        result?;
        if self.opts.verify_content && xxh3_64(dst) != e.content_hash {
            return Err(VlzError::Checksum(format!("{name}: content")));
        }
        Ok(())
    }

    /// Read `len` bytes at `offset` of an entry, decoding only the blocks that cover them.
    pub fn read_range(&self, id: EntryId, offset: u64, len: usize) -> Result<Vec<u8>> {
        let e = &self.index.entries[id.0 as usize];
        self.check_range(e, offset, len as u64)?;
        let mut out = vec![0u8; len];
        self.read_range_into(id, offset, &mut out)?;
        Ok(out)
    }

    /// Read `dst.len()` bytes at `offset` of an entry into `dst`.
    pub fn read_range_into(&self, id: EntryId, offset: u64, dst: &mut [u8]) -> Result<()> {
        let e = self.index.entries[id.0 as usize];
        let len = dst.len() as u64;
        self.check_range(&e, offset, len)?;
        if len == 0 {
            return Ok(());
        }
        let end = offset + len;
        let name = self.index.name(&e);
        let blocks = self.blocks_of(&e);
        let bs = u64::from(e.block_raw_size);
        let first = (offset / bs) as usize;
        let last = ((end - 1) / bs) as usize;
        for (bi, b) in blocks.iter().enumerate().take(last + 1).skip(first) {
            let start = bi as u64 * bs;
            let stop = start + u64::from(b.raw_size);
            let from = offset.max(start);
            let to = end.min(stop);
            let out = &mut dst[(from - offset) as usize..(to - offset) as usize];
            if from == start && to == stop {
                self.decode_block(name, bi, b, out)?;
            } else {
                let mut tmp = self.pool.take(b.raw_size as usize);
                self.decode_block(name, bi, b, &mut tmp)?;
                out.copy_from_slice(&tmp[(from - start) as usize..(to - start) as usize]);
            }
        }
        Ok(())
    }

    fn check_range(&self, e: &EntryRecord, offset: u64, len: u64) -> Result<()> {
        if offset.checked_add(len).map_or(true, |end| end > e.raw_size) {
            return Err(VlzError::Range {
                name: self.index.name(e).to_owned(),
                offset,
                len,
                size: e.raw_size,
            });
        }
        Ok(())
    }

    fn decode_block(&self, name: &str, i: usize, b: &BlockRecord, dst: &mut [u8]) -> Result<()> {
        debug_assert_eq!(dst.len(), b.raw_size as usize);
        if b.codec == Codec::Raw.id() {
            self.file.read_exact_at(dst, b.offset)?;
            if self.opts.verify_checksums && xxh3_64(dst) != b.checksum {
                return Err(VlzError::Checksum(format!("{name}: block {i}")));
            }
            return Ok(());
        }
        if !self.opts.decoders.supports(b.codec) {
            return Err(VlzError::UnsupportedCodec(b.codec));
        }
        let mut stored = self.pool.take(b.stored_size as usize);
        self.file.read_exact_at(&mut stored, b.offset)?;
        if self.opts.verify_checksums && xxh3_64(&stored) != b.checksum {
            return Err(VlzError::Checksum(format!("{name}: block {i}")));
        }
        self.opts
            .decoders
            .decode(b.codec, &stored, dst)
            .map_err(|err| match err {
                VlzError::Decompress(m) => VlzError::Decompress(format!("{name}: block {i}: {m}")),
                other => other,
            })
    }

    /// Reserve up to `want` helper threads from this archive's shared budget.
    fn reserve_helpers(&self, want: usize) -> usize {
        let limit = self.opts.parallel_threads.saturating_sub(1);
        let mut current = self.helpers_busy.load(Ordering::Acquire);
        loop {
            let take = want.min(limit.saturating_sub(current));
            if take == 0 {
                return 0;
            }
            match self.helpers_busy.compare_exchange_weak(
                current,
                current + take,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return take,
                Err(actual) => current = actual,
            }
        }
    }

    fn decode_parallel(
        &self,
        name: &str,
        blocks: &[BlockRecord],
        dst: &mut [u8],
        helpers: usize,
    ) -> Result<()> {
        let mut parts = Vec::with_capacity(blocks.len());
        let mut rest = dst;
        for (i, b) in blocks.iter().enumerate() {
            let (here, tail) = rest.split_at_mut(b.raw_size as usize);
            parts.push((i, b, here));
            rest = tail;
        }
        let queue = Mutex::new(parts.into_iter());
        let failed = AtomicBool::new(false);
        let first_error: Mutex<Option<(usize, VlzError)>> = Mutex::new(None);
        let work = || loop {
            if failed.load(Ordering::Relaxed) {
                break;
            }
            let next = queue.lock().unwrap_or_else(|p| p.into_inner()).next();
            let Some((i, b, out)) = next else { break };
            if let Err(err) = self.decode_block(name, i, b, out) {
                failed.store(true, Ordering::Relaxed);
                let mut slot = first_error.lock().unwrap_or_else(|p| p.into_inner());
                if slot.as_ref().map_or(true, |(at, _)| i < *at) {
                    *slot = Some((i, err));
                }
            }
        };
        std::thread::scope(|scope| {
            for n in 0..helpers {
                let spawned = std::thread::Builder::new()
                    .name(format!("vlz-decode-{n}"))
                    .spawn_scoped(scope, work);
                if spawned.is_err() {
                    break; // fewer helpers; the calling thread still drains the queue
                }
            }
            work();
        });
        match first_error.into_inner().unwrap_or_else(|p| p.into_inner()) {
            Some((_, err)) => Err(err),
            None => Ok(()),
        }
    }

    /// Decode every entry and check it against its content hash (in parallel).
    pub fn verify(&self) -> VerifyReport {
        let n = self.index.entries.len();
        let threads = std::thread::available_parallelism()
            .map_or(1, |t| t.get())
            .min(n.max(1));
        let next = AtomicUsize::new(0);
        let failures = Mutex::new(Vec::new());
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let e = &self.index.entries[i];
                    let outcome = self.read_entry(EntryId(i as u32)).and_then(|data| {
                        if xxh3_64(&data) == e.content_hash {
                            Ok(())
                        } else {
                            Err(VlzError::Checksum("content hash".into()))
                        }
                    });
                    if let Err(err) = outcome {
                        failures
                            .lock()
                            .unwrap_or_else(|p| p.into_inner())
                            .push((self.index.name(e).to_owned(), err.to_string()));
                    }
                });
            }
        });
        let mut failures = failures.into_inner().unwrap_or_else(|p| p.into_inner());
        failures.sort();
        VerifyReport {
            entries: n,
            blocks: self.index.blocks.len(),
            raw_bytes: self.index.entries.iter().map(|e| e.raw_size).sum(),
            failures,
        }
    }

    /// Size and codec statistics.
    pub fn stats(&self) -> ArchiveStats {
        let mut per_codec: Vec<(u8, CodecTotals)> = Vec::new();
        for b in &self.index.blocks {
            let slot = match per_codec.iter_mut().find(|(id, _)| *id == b.codec) {
                Some(s) => s,
                None => {
                    per_codec.push((b.codec, CodecTotals::default()));
                    per_codec.last_mut().expect("just pushed")
                }
            };
            slot.1.blocks += 1;
            slot.1.raw_bytes += u64::from(b.raw_size);
            slot.1.stored_bytes += u64::from(b.stored_size);
        }
        per_codec.sort_by_key(|(id, _)| *id);
        let mut per_class: Vec<(ContentClass, u64, u64, u64)> = Vec::new();
        for e in &self.index.entries {
            let class = ContentClass::from_id(e.content_class);
            match per_class.iter_mut().find(|c| c.0 == class) {
                Some(c) => {
                    c.1 += 1;
                    c.2 += e.raw_size;
                    c.3 += e.stored_size;
                }
                None => per_class.push((class, 1, e.raw_size, e.stored_size)),
            }
        }
        per_class.sort_by_key(|c| std::cmp::Reverse(c.2));
        ArchiveStats {
            file_bytes: self.file.len(),
            index_bytes: self.header.index_size,
            entries: self.index.entries.len(),
            blocks: self.index.blocks.len(),
            raw_bytes: self.index.entries.iter().map(|e| e.raw_size).sum(),
            stored_bytes: self.index.entries.iter().map(|e| e.stored_size).sum(),
            per_codec,
            per_class,
        }
    }

    /// Counters of the stored-bytes buffer pool.
    pub fn pool_stats(&self) -> PoolStats {
        self.pool.stats()
    }
}
