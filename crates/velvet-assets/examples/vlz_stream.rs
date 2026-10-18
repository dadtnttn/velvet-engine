//! Stream a whole `.vlz` archive into `Assets` while a 60 Hz frame loop keeps running.
//!
//! ```text
//! velvet vlz pack <assets-dir> game.vlz
//! cargo run --release -p velvet-assets --example vlz_stream -- game.vlz [loose-dir] [max-images]
//! ```
//!
//! Up to `max-images` (150) PNG/JPEG entries are decoded to `ImageAsset` on the streamer's
//! workers (the title art first, at high priority; a decoded 1080p image is ~8 MB of RAM);
//! everything else loads as bytes at low priority. A few loads are cancelled
//! and a few prefetched to show both. The main thread only calls `pump_async` once a frame.

use std::sync::Arc;
use std::time::{Duration, Instant};

use velvet_assets::{
    register_image_loaders, AssetState, AssetStreamer, Assets, BytesAsset, FileSource, ImageAsset,
    LayeredSource, LoadPriority, Source, StreamConfig, VlzSource,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(archive) = args.next() else {
        eprintln!("usage: vlz_stream <archive.vlz> [loose-dir]");
        std::process::exit(2);
    };
    let vlz = VlzSource::open(&archive).unwrap_or_else(|e| {
        eprintln!("cannot open {archive}: {e}");
        std::process::exit(1);
    });
    let entries: Vec<(String, u64)> = vlz
        .archive()
        .entries()
        .map(|e| (e.name.to_string(), e.raw_size))
        .collect();
    // Archive first, loose files (mods, files being edited) after it.
    let mut layered = LayeredSource::new().with(Arc::new(vlz));
    if let Some(loose) = args.next().filter(|l| l != "-") {
        layered.push(Arc::new(FileSource::new(loose)));
    }
    let max_images: usize = args.next().and_then(|n| n.parse().ok()).unwrap_or(150);
    let source: Arc<dyn Source> = Arc::new(layered);
    let mut assets = Assets::with_source(Arc::clone(&source));
    register_image_loaders(assets.loaders_mut());
    let streamer = AssetStreamer::new(StreamConfig::default());

    let started = Instant::now();
    let mut images = Vec::new();
    let mut others = Vec::new();
    for (name, _) in &entries {
        let lower = name.to_ascii_lowercase();
        let is_image =
            lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg");
        if is_image && (lower.contains("/title/") || images.len() < max_images) {
            let priority = if lower.contains("/title/") {
                LoadPriority::High
            } else {
                LoadPriority::Normal
            };
            images.push(assets.load_async::<ImageAsset>(&streamer, name.as_str(), priority));
        } else if lower.ends_with(".mp4") {
            // Large media streamed later by its own player: warm the byte cache only.
            streamer.prefetch(Arc::clone(&source), name.as_str());
        } else {
            others.push(assets.load_async::<BytesAsset>(
                &streamer,
                name.as_str(),
                LoadPriority::Low,
            ));
        }
    }
    // Change of plans: the player left before these were needed.
    let cancelled = others
        .iter()
        .rev()
        .take(5)
        .filter(|h| assets.cancel_async(&streamer, **h))
        .count();
    println!(
        "{} entries: {} images decoding, {} byte loads ({} cancelled), {} prefetches, {} workers",
        entries.len(),
        images.len(),
        others.len(),
        cancelled,
        entries.iter().filter(|(n, _)| n.ends_with(".mp4")).count(),
        streamer.worker_count()
    );

    let frame = Duration::from_secs_f64(1.0 / 60.0);
    let mut busy = Vec::new();
    let mut frames = 0u64;
    while assets.pending_async() > 0 || !streamer.wait_idle(Duration::ZERO) {
        let t = Instant::now();
        assets.pump_async(&streamer, 8);
        busy.push(t.elapsed().as_secs_f64() * 1000.0);
        frames += 1;
        if frames.is_multiple_of(60) {
            let s = streamer.stats();
            println!(
                "  t={:5.1}s pending {:5} · read {:7.1} MiB · in flight {:6.1} MiB · cache {:6.1} MiB",
                started.elapsed().as_secs_f64(),
                assets.pending_async(),
                s.bytes_read as f64 / 1_048_576.0,
                s.inflight_bytes as f64 / 1_048_576.0,
                s.cache_bytes as f64 / 1_048_576.0
            );
        }
        if let Some(rest) = frame.checked_sub(t.elapsed()) {
            std::thread::sleep(rest);
        }
    }
    let loaded_images = images
        .iter()
        .filter(|h| assets.state(**h) == AssetState::Loaded)
        .count();
    let pixels: u64 = images
        .iter()
        .filter_map(|h| assets.get(*h))
        .map(|img| u64::from(img.width) * u64::from(img.height))
        .sum();
    busy.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let p99 = busy[((busy.len() - 1) as f64 * 0.99) as usize];
    let s = streamer.stats();
    println!(
        "done in {:.2} s over {frames} frames: {loaded_images}/{} images decoded ({:.0} Mpixels), \
         {} failed, {} cancelled, cache hits {}; main thread pump p99 {p99:.3} ms, max {:.3} ms",
        started.elapsed().as_secs_f64(),
        images.len(),
        pixels as f64 / 1e6,
        s.failed,
        s.cancelled,
        s.cache_hits,
        busy.last().copied().unwrap_or(0.0)
    );
}
