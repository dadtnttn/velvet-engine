//! Background streaming: priorities, cancellation, dedupe, cache and prefetch, in-flight memory
//! budget, error and panic handling, and many concurrent loads from a real VLZ archive.

use std::any::{Any, TypeId};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use velvet_assets::vlz::{pack_to_file, PackInput, PackOptions};
use velvet_assets::{
    AssetEvent, AssetLoader, AssetPath, AssetState, AssetStreamer, Assets, BytesAsset, LoadError,
    LoadPriority, MemorySource, Source, SourceError, StreamConfig, StreamPayload, TextAsset,
    VlzSource,
};

const WAIT: Duration = Duration::from_secs(20);

/// A memory source that logs reads, tracks concurrency, and holds reads of `gate*` paths
/// until opened.
#[derive(Default)]
struct Probe {
    files: MemorySource,
    open: Mutex<bool>,
    cv: Condvar,
    log: Mutex<Vec<String>>,
    reads: AtomicUsize,
    now: AtomicUsize,
    peak: AtomicUsize,
}

impl Probe {
    fn with(files: &[(&str, Vec<u8>)]) -> Arc<Self> {
        let mut mem = MemorySource::new();
        for (p, d) in files {
            mem.insert(*p, d.clone());
        }
        Arc::new(Self {
            files: mem,
            ..Self::default()
        })
    }
    fn release(&self) {
        *self.open.lock().unwrap() = true;
        self.cv.notify_all();
    }
    fn log(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }
}

impl Source for Probe {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        let now = self.now.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(now, Ordering::SeqCst);
        self.reads.fetch_add(1, Ordering::SeqCst);
        let name = path.virtual_path.as_str().to_owned();
        self.log.lock().unwrap().push(name.clone());
        if name.starts_with("gate") {
            let mut open = self.open.lock().unwrap();
            while !*open {
                open = self.cv.wait(open).unwrap();
            }
        }
        if name == "panic.bin" {
            self.now.fetch_sub(1, Ordering::SeqCst);
            panic!("source exploded");
        }
        std::thread::sleep(Duration::from_millis(5));
        let out = self.files.read(path);
        self.now.fetch_sub(1, Ordering::SeqCst);
        out
    }
    fn exists(&self, path: &AssetPath) -> bool {
        self.files.exists(path)
    }
    fn roots(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("probe://")]
    }
    fn size_hint(&self, path: &AssetPath) -> Option<u64> {
        self.files.size_hint(path)
    }
}

fn one_worker() -> AssetStreamer {
    AssetStreamer::new(StreamConfig {
        workers: 1,
        ..StreamConfig::default()
    })
}

fn pump_until_idle(assets: &mut Assets, streamer: &AssetStreamer) {
    let deadline = Instant::now() + WAIT;
    while assets.pending_async() > 0 {
        assets.pump_async(streamer, 64);
        assert!(Instant::now() < deadline, "loads did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn async_load_reaches_loaded_through_pump() {
    let probe = Probe::with(&[("story/intro.txt", b"scene main {}".to_vec())]);
    let mut assets = Assets::with_source(probe.clone());
    let streamer = AssetStreamer::default();
    let h = assets.load_async::<TextAsset>(&streamer, "story/intro.txt", LoadPriority::Normal);
    assert!(!h.is_none());
    assert_eq!(assets.state(h), AssetState::Loading);
    assert!(assets.get(h).is_none());
    pump_until_idle(&mut assets, &streamer);
    assert_eq!(assets.state(h), AssetState::Loaded);
    assert_eq!(assets.get(h).unwrap().text, "scene main {}");
    assert!(assets
        .drain_events()
        .iter()
        .any(|e| matches!(e, AssetEvent::Loaded { .. })));
}

#[test]
fn higher_priority_runs_first() {
    let probe = Probe::with(&[
        ("gate.bin", vec![0]),
        ("low.bin", vec![1]),
        ("normal.bin", vec![2]),
        ("high.bin", vec![3]),
        ("critical.bin", vec![4]),
    ]);
    let mut assets = Assets::with_source(probe.clone());
    let streamer = one_worker();
    assets.load_async::<BytesAsset>(&streamer, "gate.bin", LoadPriority::Normal);
    // Let the single worker take the gate before queueing the rest.
    let deadline = Instant::now() + WAIT;
    while probe.reads.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    assets.load_async::<BytesAsset>(&streamer, "low.bin", LoadPriority::Low);
    assets.load_async::<BytesAsset>(&streamer, "normal.bin", LoadPriority::Normal);
    assets.load_async::<BytesAsset>(&streamer, "high.bin", LoadPriority::High);
    assets.load_async::<BytesAsset>(&streamer, "critical.bin", LoadPriority::Critical);
    probe.release();
    pump_until_idle(&mut assets, &streamer);
    assert_eq!(
        probe.log(),
        [
            "gate.bin",
            "critical.bin",
            "high.bin",
            "normal.bin",
            "low.bin"
        ]
    );
}

#[test]
fn cancelled_loads_are_never_read_or_delivered() {
    let probe = Probe::with(&[
        ("gate.bin", vec![0]),
        ("wanted.bin", vec![1]),
        ("dropped.bin", vec![2]),
    ]);
    let mut assets = Assets::with_source(probe.clone());
    let streamer = one_worker();
    assets.load_async::<BytesAsset>(&streamer, "gate.bin", LoadPriority::Normal);
    while probe.reads.load(Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(1));
    }
    let wanted = assets.load_async::<BytesAsset>(&streamer, "wanted.bin", LoadPriority::Normal);
    let dropped = assets.load_async::<BytesAsset>(&streamer, "dropped.bin", LoadPriority::Normal);
    assert!(assets.cancel_async(&streamer, dropped));
    assert!(
        !assets.cancel_async(&streamer, dropped),
        "second cancel finds nothing"
    );
    probe.release();
    pump_until_idle(&mut assets, &streamer);
    assert!(streamer.wait_idle(WAIT));
    assert_eq!(assets.state(wanted), AssetState::Loaded);
    assert_eq!(assets.state(dropped), AssetState::Unloaded);
    assert!(!probe.log().contains(&"dropped.bin".to_string()));
    assert_eq!(streamer.stats().cancelled, 1);
    // A cancelled asset can be requested again.
    let again = assets.load_async::<BytesAsset>(&streamer, "dropped.bin", LoadPriority::Normal);
    pump_until_idle(&mut assets, &streamer);
    assert_eq!(assets.get(again).unwrap().data, vec![2]);
}

#[test]
fn duplicate_requests_share_one_load() {
    let probe = Probe::with(&[("a.bin", vec![9; 10])]);
    let mut assets = Assets::with_source(probe.clone());
    let streamer = AssetStreamer::default();
    let h1 = assets.load_async::<BytesAsset>(&streamer, "a.bin", LoadPriority::Normal);
    let h2 = assets.load_async::<BytesAsset>(&streamer, "a.bin", LoadPriority::High);
    assert_eq!(h1, h2);
    pump_until_idle(&mut assets, &streamer);
    let h3 = assets.load_async::<BytesAsset>(&streamer, "a.bin", LoadPriority::Normal);
    assert_eq!(h1, h3);
    assert_eq!(probe.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn prefetch_fills_the_cache_and_later_loads_skip_the_read() {
    let probe = Probe::with(&[("music/theme.ogg", vec![7; 1000])]);
    let source: Arc<dyn Source> = probe.clone();
    let mut assets = Assets::with_source(Arc::clone(&source));
    let streamer = AssetStreamer::default();
    streamer.prefetch(Arc::clone(&source), "music/theme.ogg");
    assert!(streamer.wait_idle(WAIT));
    assert!(streamer.is_cached(&source, "music/theme.ogg"));
    let h = assets.load_async::<BytesAsset>(&streamer, "music/theme.ogg", LoadPriority::Normal);
    pump_until_idle(&mut assets, &streamer);
    assert_eq!(assets.get(h).unwrap().data.len(), 1000);
    assert_eq!(probe.reads.load(Ordering::SeqCst), 1);
    let s = streamer.stats();
    assert_eq!((s.cache_hits, s.cache_misses), (1, 1));
    // Typed loads consume the cached bytes (the decoded asset is the cache from now on).
    assert!(!streamer.is_cached(&source, "music/theme.ogg"));
}

#[test]
fn cache_stays_within_its_budget() {
    let files: Vec<(String, Vec<u8>)> = (0..10)
        .map(|i| (format!("f{i}"), vec![i as u8; 60]))
        .collect();
    let refs: Vec<(&str, Vec<u8>)> = files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let source: Arc<dyn Source> = Probe::with(&refs);
    let streamer = AssetStreamer::new(StreamConfig {
        cache_budget_bytes: 150,
        ..StreamConfig::default()
    });
    for (n, _) in &files {
        streamer.prefetch(Arc::clone(&source), n.as_str());
    }
    assert!(streamer.wait_idle(WAIT));
    let s = streamer.stats();
    assert!(s.cache_bytes <= 150, "{s:?}");
    assert_eq!(s.cache_entries, 2);
}

#[test]
fn byte_requests_deliver_bytes() {
    let source: Arc<dyn Source> = Probe::with(&[("blob", vec![1, 2, 3])]);
    let streamer = AssetStreamer::default();
    let t = streamer.request_bytes(Arc::clone(&source), "blob", LoadPriority::High);
    match streamer.wait(t, WAIT).expect("finished").result {
        Ok(StreamPayload::Bytes(b)) => assert_eq!(*b, vec![1, 2, 3]),
        other => panic!("{other:?}"),
    }
    assert!(!streamer.is_pending(t));
    let missing = streamer.request_bytes(source, "missing", LoadPriority::Normal);
    assert!(matches!(
        streamer.wait(missing, WAIT).unwrap().result,
        Err(LoadError::Source(_))
    ));
}

#[test]
fn inflight_budget_limits_concurrent_reads_but_never_deadlocks() {
    let files: Vec<(String, Vec<u8>)> = (0..8).map(|i| (format!("m{i}"), vec![0; 80])).collect();
    let mut refs: Vec<(&str, Vec<u8>)> =
        files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    refs.push(("huge", vec![0; 1000]));
    let probe = Probe::with(&refs);
    let mut assets = Assets::with_source(probe.clone());
    let streamer = AssetStreamer::new(StreamConfig {
        workers: 4,
        max_inflight_bytes: 100,
        ..StreamConfig::default()
    });
    let handles: Vec<_> = files
        .iter()
        .map(|(n, _)| assets.load_async::<BytesAsset>(&streamer, n.as_str(), LoadPriority::Normal))
        .collect();
    let huge = assets.load_async::<BytesAsset>(&streamer, "huge", LoadPriority::Normal);
    pump_until_idle(&mut assets, &streamer);
    assert_eq!(
        probe.peak.load(Ordering::SeqCst),
        1,
        "80+80 bytes exceed the 100-byte budget"
    );
    assert!(handles
        .iter()
        .all(|h| assets.state(*h) == AssetState::Loaded));
    assert_eq!(
        assets.get(huge).unwrap().data.len(),
        1000,
        "bigger than the budget, still loads"
    );
    assert!(streamer.stats().peak_inflight_bytes <= 1000);
}

struct PanickyLoader;
impl AssetLoader for PanickyLoader {
    fn type_name(&self) -> &'static str {
        "Panicky"
    }
    fn value_type(&self) -> TypeId {
        TypeId::of::<Panicky>()
    }
    fn extensions(&self) -> &[&'static str] {
        &[]
    }
    fn load(&self, _: &AssetPath, _: &[u8]) -> Result<Box<dyn Any + Send + Sync>, LoadError> {
        panic!("loader exploded")
    }
}
struct Panicky;

#[test]
fn failures_and_panics_become_failed_entries() {
    let probe = Probe::with(&[
        ("bad.txt", vec![0xFF, 0xFE]),
        ("ok.bin", vec![1]),
        ("x.bin", vec![1]),
    ]);
    let mut assets = Assets::with_source(probe.clone());
    assets.loaders_mut().register(PanickyLoader);
    let streamer = one_worker();
    let missing = assets.load_async::<BytesAsset>(&streamer, "missing.bin", LoadPriority::Normal);
    let bad_utf8 = assets.load_async::<TextAsset>(&streamer, "bad.txt", LoadPriority::Normal);
    let source_panic =
        assets.load_async::<BytesAsset>(&streamer, "panic.bin", LoadPriority::Normal);
    let loader_panic = assets.load_async::<Panicky>(&streamer, "x.bin", LoadPriority::Normal);
    let fine = assets.load_async::<BytesAsset>(&streamer, "ok.bin", LoadPriority::Low);
    pump_until_idle(&mut assets, &streamer);
    for h in [missing, source_panic] {
        assert_eq!(assets.state(h), AssetState::Failed);
    }
    assert_eq!(assets.state(bad_utf8), AssetState::Failed);
    assert_eq!(assets.state(loader_panic), AssetState::Failed);
    assert!(assets.error(loader_panic).unwrap().contains("panicked"));
    assert_eq!(
        assets.state(fine),
        AssetState::Loaded,
        "the worker survived the panics"
    );
    let failed = assets
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, AssetEvent::Failed { .. }))
        .count();
    assert_eq!(failed, 4);
}

#[test]
fn many_concurrent_loads_from_a_vlz_archive() {
    let dir = tempfile::tempdir().unwrap();
    let files: Vec<(String, Vec<u8>)> = (0..300)
        .map(|i| {
            let body = format!("{{\"item\":{i},\"pad\":\"{}\"}}", "x".repeat(i * 37 % 5000));
            (format!("data/item_{i:03}.json"), body.into_bytes())
        })
        .collect();
    let inputs = files
        .iter()
        .map(|(n, d)| PackInput::bytes(n.clone(), d.clone()))
        .collect();
    pack_to_file(
        inputs,
        dir.path().join("items.vlz"),
        &PackOptions::default(),
    )
    .unwrap();
    let source = Arc::new(VlzSource::open(dir.path().join("items.vlz")).unwrap());
    let mut assets = Assets::with_source(source);
    let streamer = AssetStreamer::new(StreamConfig {
        workers: 4,
        ..StreamConfig::default()
    });
    let handles: Vec<_> = files
        .iter()
        .enumerate()
        .map(|(i, (n, _))| {
            let p = [LoadPriority::Low, LoadPriority::Normal, LoadPriority::High][i % 3];
            assets.load_async::<TextAsset>(&streamer, n.as_str(), p)
        })
        .collect();
    // Frame loop: pump a bounded number per "frame".
    let deadline = Instant::now() + WAIT;
    while assets.pending_async() > 0 {
        assets.pump_async(&streamer, 16);
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    for ((_, body), h) in files.iter().zip(handles) {
        assert_eq!(assets.get(h).unwrap().text.as_bytes(), &body[..]);
    }
    let s = streamer.stats();
    assert_eq!(s.completed, 300);
    assert_eq!(s.failed, 0);
}

#[test]
fn pump_never_blocks_the_frame() {
    let probe = Probe::with(&[("gate.bin", vec![0])]);
    let mut assets = Assets::with_source(probe.clone());
    let streamer = one_worker();
    let h = assets.load_async::<BytesAsset>(&streamer, "gate.bin", LoadPriority::Normal);
    let started = Instant::now();
    for _ in 0..1000 {
        assert_eq!(assets.pump_async(&streamer, 8), 0);
    }
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(assets.state(h), AssetState::Loading);
    probe.release();
    pump_until_idle(&mut assets, &streamer);
}
