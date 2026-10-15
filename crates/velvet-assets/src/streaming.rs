//! Background asset streaming: worker threads read from any [`Source`] (loose files, a VLZ
//! archive, memory) and decode with the registered [`AssetLoader`]s, so the main / render
//! thread only picks up finished assets ([`crate::Assets::pump_async`]).
//!
//! - Priorities ([`LoadPriority`]): higher first, FIFO among equals.
//! - Cancellation: checked before reading, after reading and before delivery; a read or decode
//!   already running finishes but its result is dropped.
//! - Memory: bytes being read or decoded are budgeted (`max_inflight_bytes`, from
//!   [`Source::size_hint`]); a job that would exceed it waits for others to finish, but one job
//!   always runs, so an asset larger than the budget still loads. Raw bytes can be kept in an
//!   LRU cache with its own budget (always for prefetches and byte requests).
//! - A panic in a source or loader becomes a [`LoadError`], not a dead worker.
//!
//! Throughput is bounded by storage and cores: when they cannot keep up, loads queue up and
//! complete later; nothing here can make a slow disk fast.

use std::any::Any;
use std::collections::{BTreeMap, BinaryHeap, HashMap};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::async_queue::LoadPriority;
use crate::loader::{AssetLoader, LoadError};
use crate::path::AssetPath;
use crate::source::Source;

/// Streamer settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamConfig {
    /// Worker threads (0 = half the cores, between 1 and 4, leaving room for the main and
    /// render threads).
    pub workers: usize,
    /// Budget of the raw-bytes cache.
    pub cache_budget_bytes: usize,
    /// Bytes allowed in flight (being read or decoded) at once.
    pub max_inflight_bytes: u64,
    /// Also keep the raw bytes of decoded assets in the cache (off: the decoded asset in
    /// [`crate::Assets`] is the cache, keeping both would double the memory).
    pub cache_decoded_sources: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            workers: 0,
            cache_budget_bytes: 128 * 1024 * 1024,
            max_inflight_bytes: 256 * 1024 * 1024,
            cache_decoded_sources: false,
        }
    }
}

impl StreamConfig {
    fn worker_count(&self) -> usize {
        if self.workers > 0 {
            return self.workers;
        }
        let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
        (cores / 2).clamp(1, 4)
    }
}

/// Id of one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StreamTicket(pub u64);

/// What a finished request carries.
pub enum StreamPayload {
    /// Raw bytes (byte requests).
    Bytes(Arc<Vec<u8>>),
    /// A decoded asset (typed requests), as produced by its loader.
    Asset(Box<dyn Any + Send + Sync>),
}

impl std::fmt::Debug for StreamPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamPayload::Bytes(b) => write!(f, "Bytes({} bytes)", b.len()),
            StreamPayload::Asset(_) => f.write_str("Asset(..)"),
        }
    }
}

/// A finished request.
#[derive(Debug)]
pub struct StreamCompletion {
    /// Its ticket.
    pub ticket: StreamTicket,
    /// Path requested.
    pub path: AssetPath,
    /// Outcome.
    pub result: Result<StreamPayload, LoadError>,
}

/// Counters of an [`AssetStreamer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamStats {
    /// Requests accepted.
    pub requested: u64,
    /// Requests finished successfully (prefetches included).
    pub completed: u64,
    /// Requests that failed.
    pub failed: u64,
    /// Requests cancelled before delivery.
    pub cancelled: u64,
    /// Reads served by the byte cache.
    pub cache_hits: u64,
    /// Reads that went to the source.
    pub cache_misses: u64,
    /// Bytes read from sources.
    pub bytes_read: u64,
    /// Bytes held by the cache now.
    pub cache_bytes: u64,
    /// Entries in the cache now.
    pub cache_entries: u64,
    /// Bytes in flight now.
    pub inflight_bytes: u64,
    /// Highest bytes in flight seen.
    pub peak_inflight_bytes: u64,
    /// Requests waiting.
    pub queued: u64,
    /// Requests being worked on.
    pub active: u64,
}

enum JobKind {
    Bytes,
    Prefetch,
    Decode(Arc<dyn AssetLoader>),
}

struct Job {
    priority: LoadPriority,
    seq: u64,
    ticket: u64,
    kind: JobKind,
    path: AssetPath,
    source: Arc<dyn Source>,
    cancel: Arc<AtomicBool>,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}
impl Eq for Job {}
impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Job {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Max-heap: higher priority first, then the older request.
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

#[derive(Default)]
struct Queue {
    heap: BinaryHeap<Job>,
    seq: u64,
    active: usize,
    inflight_bytes: u64,
    peak_inflight_bytes: u64,
    shutdown: bool,
}

type CacheKey = (usize, String);

#[derive(Default)]
struct ByteCache {
    map: HashMap<CacheKey, (Arc<Vec<u8>>, u64)>,
    order: BTreeMap<u64, CacheKey>,
    tick: u64,
    bytes: usize,
    budget: usize,
}

impl ByteCache {
    fn get(&mut self, key: &CacheKey) -> Option<Arc<Vec<u8>>> {
        self.tick += 1;
        let tick = self.tick;
        let (data, old) = self.map.get_mut(key)?;
        self.order.remove(old);
        *old = tick;
        self.order.insert(tick, key.clone());
        Some(Arc::clone(data))
    }

    fn take(&mut self, key: &CacheKey) -> Option<Arc<Vec<u8>>> {
        let (data, tick) = self.map.remove(key)?;
        self.order.remove(&tick);
        self.bytes -= data.len();
        Some(data)
    }

    fn insert(&mut self, key: CacheKey, data: Arc<Vec<u8>>) {
        if data.len() > self.budget {
            return;
        }
        self.take(&key);
        self.tick += 1;
        self.bytes += data.len();
        self.order.insert(self.tick, key.clone());
        self.map.insert(key, (data, self.tick));
        while self.bytes > self.budget {
            let Some((_, oldest)) = self.order.pop_first() else {
                break;
            };
            if let Some((d, _)) = self.map.remove(&oldest) {
                self.bytes -= d.len();
            }
        }
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.bytes = 0;
    }
}

#[derive(Default)]
struct Counters {
    requested: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    cancelled: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    bytes_read: AtomicU64,
}

struct Shared {
    config: StreamConfig,
    queue: Mutex<Queue>,
    /// New jobs, released budget, shutdown.
    wake: Condvar,
    /// A job finished (for `wait_idle`).
    idle: Condvar,
    done: Mutex<HashMap<u64, StreamCompletion>>,
    done_cv: Condvar,
    flags: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    cache: Mutex<ByteCache>,
    next_ticket: AtomicU64,
    counters: Counters,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

/// Background loader pool. One per game is typical; it can serve several [`crate::Assets`]
/// and any number of sources.
pub struct AssetStreamer {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

impl std::fmt::Debug for AssetStreamer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetStreamer")
            .field("workers", &self.workers.len())
            .field("stats", &self.stats())
            .finish()
    }
}

impl Default for AssetStreamer {
    fn default() -> Self {
        Self::new(StreamConfig::default())
    }
}

impl AssetStreamer {
    /// Start the workers.
    pub fn new(config: StreamConfig) -> Self {
        let count = config.worker_count();
        let shared = Arc::new(Shared {
            cache: Mutex::new(ByteCache {
                budget: config.cache_budget_bytes,
                ..ByteCache::default()
            }),
            config,
            queue: Mutex::new(Queue::default()),
            wake: Condvar::new(),
            idle: Condvar::new(),
            done: Mutex::new(HashMap::new()),
            done_cv: Condvar::new(),
            flags: Mutex::new(HashMap::new()),
            next_ticket: AtomicU64::new(1),
            counters: Counters::default(),
        });
        let workers = (0..count)
            .filter_map(|n| {
                let shared = Arc::clone(&shared);
                std::thread::Builder::new()
                    .name(format!("velvet-stream-{n}"))
                    .spawn(move || worker(&shared))
                    .ok()
            })
            .collect();
        Self { shared, workers }
    }

    /// Settings.
    pub fn config(&self) -> &StreamConfig {
        &self.shared.config
    }

    /// Worker threads running.
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }

    fn submit(
        &self,
        source: Arc<dyn Source>,
        path: AssetPath,
        priority: LoadPriority,
        kind: JobKind,
    ) -> StreamTicket {
        let ticket = self.shared.next_ticket.fetch_add(1, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        lock(&self.shared.flags).insert(ticket, Arc::clone(&cancel));
        self.shared
            .counters
            .requested
            .fetch_add(1, Ordering::Relaxed);
        let mut q = lock(&self.shared.queue);
        q.seq += 1;
        let seq = q.seq;
        q.heap.push(Job {
            priority,
            seq,
            ticket,
            kind,
            path,
            source,
            cancel,
        });
        drop(q);
        self.shared.wake.notify_one();
        StreamTicket(ticket)
    }

    /// Read a resource's bytes in the background (delivered as [`StreamPayload::Bytes`]; the
    /// bytes also go to the cache).
    pub fn request_bytes(
        &self,
        source: Arc<dyn Source>,
        path: impl Into<AssetPath>,
        priority: LoadPriority,
    ) -> StreamTicket {
        self.submit(source, path.into(), priority, JobKind::Bytes)
    }

    /// Read and decode with `loader` in the background (delivered as
    /// [`StreamPayload::Asset`]). [`crate::Assets::load_async`] is the usual entry point.
    pub fn request_decode(
        &self,
        source: Arc<dyn Source>,
        path: impl Into<AssetPath>,
        loader: Arc<dyn AssetLoader>,
        priority: LoadPriority,
    ) -> StreamTicket {
        self.submit(source, path.into(), priority, JobKind::Decode(loader))
    }

    /// Read into the cache at low priority, delivering nothing: a later load of the same path
    /// from the same source skips the read.
    pub fn prefetch(&self, source: Arc<dyn Source>, path: impl Into<AssetPath>) -> StreamTicket {
        self.submit(source, path.into(), LoadPriority::Low, JobKind::Prefetch)
    }

    /// Cancel a request. Returns false when it already finished and was taken (or never
    /// existed).
    pub fn cancel(&self, ticket: StreamTicket) -> bool {
        let flag = lock(&self.shared.flags).remove(&ticket.0);
        let had_result = lock(&self.shared.done).remove(&ticket.0).is_some();
        match flag {
            Some(f) => {
                f.store(true, Ordering::Release);
                self.shared
                    .counters
                    .cancelled
                    .fetch_add(1, Ordering::Relaxed);
                true
            }
            None => had_result,
        }
    }

    /// Whether a request has neither been delivered nor cancelled.
    pub fn is_pending(&self, ticket: StreamTicket) -> bool {
        lock(&self.shared.flags).contains_key(&ticket.0)
    }

    /// Take a finished request without blocking.
    pub fn try_take(&self, ticket: StreamTicket) -> Option<StreamCompletion> {
        lock(&self.shared.done).remove(&ticket.0)
    }

    /// Take up to `max` finished requests accepted by `filter`, without blocking.
    pub fn take_completions(
        &self,
        max: usize,
        mut filter: impl FnMut(StreamTicket) -> bool,
    ) -> Vec<StreamCompletion> {
        let mut done = lock(&self.shared.done);
        let mut picked: Vec<u64> = done
            .keys()
            .copied()
            .filter(|t| filter(StreamTicket(*t)))
            .collect();
        picked.sort_unstable(); // oldest requests first
        picked.truncate(max);
        picked.into_iter().filter_map(|t| done.remove(&t)).collect()
    }

    /// Block until a request finishes (tools, tests, loading screens; not for a frame loop).
    pub fn wait(&self, ticket: StreamTicket, timeout: Duration) -> Option<StreamCompletion> {
        let deadline = Instant::now() + timeout;
        let mut done = lock(&self.shared.done);
        loop {
            if let Some(c) = done.remove(&ticket.0) {
                return Some(c);
            }
            if !self.is_pending(ticket) {
                return None; // cancelled
            }
            let left = deadline.checked_duration_since(Instant::now())?;
            done = self
                .shared
                .done_cv
                .wait_timeout(done, left)
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
    }

    /// Block until nothing is queued or running. Returns false on timeout.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let mut q = lock(&self.shared.queue);
        while !(q.heap.is_empty() && q.active == 0) {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            q = self
                .shared
                .idle
                .wait_timeout(q, left)
                .unwrap_or_else(|p| p.into_inner())
                .0;
        }
        true
    }

    /// Whether the cache holds `path` read from `source`.
    pub fn is_cached(&self, source: &Arc<dyn Source>, path: impl Into<AssetPath>) -> bool {
        lock(&self.shared.cache)
            .map
            .contains_key(&cache_key(source, &path.into()))
    }

    /// Drop every cached byte buffer.
    pub fn clear_cache(&self) {
        lock(&self.shared.cache).clear();
    }

    /// Current counters.
    pub fn stats(&self) -> StreamStats {
        let c = &self.shared.counters;
        let (queued, active, inflight, peak) = {
            let q = lock(&self.shared.queue);
            (
                q.heap.len(),
                q.active,
                q.inflight_bytes,
                q.peak_inflight_bytes,
            )
        };
        let (cache_bytes, cache_entries) = {
            let cache = lock(&self.shared.cache);
            (cache.bytes, cache.map.len())
        };
        StreamStats {
            requested: c.requested.load(Ordering::Relaxed),
            completed: c.completed.load(Ordering::Relaxed),
            failed: c.failed.load(Ordering::Relaxed),
            cancelled: c.cancelled.load(Ordering::Relaxed),
            cache_hits: c.cache_hits.load(Ordering::Relaxed),
            cache_misses: c.cache_misses.load(Ordering::Relaxed),
            bytes_read: c.bytes_read.load(Ordering::Relaxed),
            cache_bytes: cache_bytes as u64,
            cache_entries: cache_entries as u64,
            inflight_bytes: inflight,
            peak_inflight_bytes: peak,
            queued: queued as u64,
            active: active as u64,
        }
    }
}

impl Drop for AssetStreamer {
    fn drop(&mut self) {
        lock(&self.shared.queue).shutdown = true;
        self.shared.wake.notify_all();
        for w in self.workers.drain(..) {
            let _ = w.join();
        }
    }
}

fn cache_key(source: &Arc<dyn Source>, path: &AssetPath) -> CacheKey {
    (Arc::as_ptr(source) as *const () as usize, path.to_string())
}

fn next_job(shared: &Shared) -> Option<Job> {
    let mut q = lock(&shared.queue);
    loop {
        if q.shutdown {
            return None;
        }
        if let Some(job) = q.heap.pop() {
            q.active += 1;
            return Some(job);
        }
        q = shared.wake.wait(q).unwrap_or_else(|p| p.into_inner());
    }
}

/// Wait for room in the in-flight budget, then take `need` bytes of it. With nothing in
/// flight the job always proceeds, so an asset bigger than the budget cannot deadlock.
fn reserve(shared: &Shared, need: u64) {
    let max = shared.config.max_inflight_bytes;
    let mut q = lock(&shared.queue);
    while q.inflight_bytes > 0 && q.inflight_bytes + need > max && !q.shutdown {
        q = shared.wake.wait(q).unwrap_or_else(|p| p.into_inner());
    }
    q.inflight_bytes += need;
    q.peak_inflight_bytes = q.peak_inflight_bytes.max(q.inflight_bytes);
}

fn release(shared: &Shared, held: u64) {
    let mut q = lock(&shared.queue);
    q.inflight_bytes -= held;
    q.active -= 1;
    drop(q);
    shared.wake.notify_all();
    shared.idle.notify_all();
}

fn worker(shared: &Shared) {
    while let Some(job) = next_job(shared) {
        let mut held = 0u64;
        let outcome = run(shared, &job, &mut held);
        release(shared, held);
        // Whoever removes the ticket's flag first decides: `cancel` (nothing is delivered) or
        // this worker (the result is stored). Done is locked first, so a result is visible by
        // the time `is_pending` turns false.
        let mut done = lock(&shared.done);
        let ours = lock(&shared.flags).remove(&job.ticket).is_some();
        if ours {
            match outcome {
                Some(result) => {
                    let counter = if result.is_ok() {
                        &shared.counters.completed
                    } else {
                        &shared.counters.failed
                    };
                    counter.fetch_add(1, Ordering::Relaxed);
                    done.insert(
                        job.ticket,
                        StreamCompletion {
                            ticket: StreamTicket(job.ticket),
                            path: job.path.clone(),
                            result,
                        },
                    );
                }
                None => {
                    shared.counters.completed.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        drop(done);
        shared.done_cv.notify_all();
    }
}

/// Do the job. `None` means nothing to deliver (prefetch, or cancelled).
fn run(shared: &Shared, job: &Job, held: &mut u64) -> Option<Result<StreamPayload, LoadError>> {
    if job.cancel.load(Ordering::Acquire) {
        return None;
    }
    let key = cache_key(&job.source, &job.path);
    let keep_in_cache = match job.kind {
        JobKind::Decode(_) => shared.config.cache_decoded_sources,
        JobKind::Bytes | JobKind::Prefetch => true,
    };
    let cached = {
        let mut cache = lock(&shared.cache);
        if keep_in_cache {
            cache.get(&key)
        } else {
            cache.take(&key)
        }
    };
    let bytes = match cached {
        Some(b) => {
            shared.counters.cache_hits.fetch_add(1, Ordering::Relaxed);
            b
        }
        None => {
            shared.counters.cache_misses.fetch_add(1, Ordering::Relaxed);
            let need = job.source.size_hint(&job.path).unwrap_or(0);
            reserve(shared, need);
            *held = need;
            if job.cancel.load(Ordering::Acquire) {
                return None;
            }
            let read = catch_unwind(AssertUnwindSafe(|| job.source.read(&job.path)));
            let data = match read {
                Ok(Ok(d)) => d,
                Ok(Err(e)) => return Some(Err(LoadError::Source(e.to_string()))),
                Err(_) => {
                    return Some(Err(LoadError::Source(format!(
                        "{}: source panicked",
                        job.path
                    ))))
                }
            };
            shared
                .counters
                .bytes_read
                .fetch_add(data.len() as u64, Ordering::Relaxed);
            let data = Arc::new(data);
            if keep_in_cache {
                lock(&shared.cache).insert(key, Arc::clone(&data));
            }
            data
        }
    };
    if job.cancel.load(Ordering::Acquire) {
        return None;
    }
    match &job.kind {
        JobKind::Prefetch => None,
        JobKind::Bytes => Some(Ok(StreamPayload::Bytes(bytes))),
        JobKind::Decode(loader) => {
            let decoded = catch_unwind(AssertUnwindSafe(|| loader.load(&job.path, &bytes)));
            Some(match decoded {
                Ok(Ok(asset)) => Ok(StreamPayload::Asset(asset)),
                Ok(Err(e)) => Err(e),
                Err(_) => Err(LoadError::Decode(format!("{}: loader panicked", job.path))),
            })
        }
    }
}
