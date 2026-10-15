//! `velvet-bench vlz --input <dir>`: VLZ against the original files, on real resources.
//!
//! For each method (original loose files, RAW, LZ4, Zstandard, Zstandard high, and the
//! automatic method with each profile) it measures, on this machine:
//! - packing: wall time, process CPU time, peak heap, allocations;
//! - archive size and ratio;
//! - decoding every entry on one thread and with parallel blocks (best of 3, warm OS cache);
//! - streaming everything through `Assets` + `AssetStreamer` as fast as the workers go: time,
//!   throughput, CPU time, peak heap (which includes the loaded assets themselves),
//!   allocations;
//! - the main thread's busy time per frame of a 60 Hz loop (fixed workload + `pump_async` of
//!   at most 8 assets a frame) while streaming, over at least 300 frames, against a baseline
//!   without streaming.
//!
//! Everything printed is measured; the only fixed numbers are the simulated frame workload and
//! the 16.67 ms frame budget, both reported with the results. The OS file cache is warm (the
//! files were just read), so "load" is decode-bound, not a cold-disk figure.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use velvet_assets::vlz::{
    collect_directory, pack_to_file, Codec, Method, OpenOptions, PackInput, PackOptions,
    PackSource, Profile, VlzArchive,
};
use velvet_assets::{
    AssetStreamer, Assets, BytesAsset, FileSource, LoadPriority, Source, StreamConfig, VlzSource,
};

use crate::alloc_stats;

struct Args {
    input: PathBuf,
    limit_mb: u64,
    work_ms: f64,
    json: Option<PathBuf>,
    skip_high: bool,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut a = Args {
        input: PathBuf::new(),
        limit_mb: 512,
        work_ms: 6.0,
        json: None,
        skip_high: false,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{arg} needs a value"));
        match arg.as_str() {
            "--input" => a.input = PathBuf::from(value()?),
            "--limit-mb" => {
                a.limit_mb = value()?.parse().map_err(|e| format!("--limit-mb: {e}"))?
            }
            "--work-ms" => a.work_ms = value()?.parse().map_err(|e| format!("--work-ms: {e}"))?,
            "--json" => a.json = Some(PathBuf::from(value()?)),
            "--skip-high" => a.skip_high = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if a.input.as_os_str().is_empty() {
        return Err("usage: velvet-bench vlz --input <dir> [--limit-mb 512] [--work-ms 6] [--json out.json] [--skip-high]".into());
    }
    Ok(a)
}

#[derive(Default, Clone)]
struct FrameStats {
    frames: usize,
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
    over_budget: usize,
}

fn frame_stats(mut busy_ms: Vec<f64>) -> FrameStats {
    if busy_ms.is_empty() {
        return FrameStats::default();
    }
    busy_ms.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let at = |q: f64| busy_ms[((busy_ms.len() - 1) as f64 * q).round() as usize];
    FrameStats {
        frames: busy_ms.len(),
        p50: at(0.50),
        p95: at(0.95),
        p99: at(0.99),
        max: *busy_ms.last().expect("not empty"),
        over_budget: busy_ms.iter().filter(|&&ms| ms > FRAME_MS).count(),
    }
}

#[derive(Default, Clone)]
struct Measure {
    secs: f64,
    cpu_secs: f64,
    peak_heap: u64,
    allocs: u64,
}

fn measure<T>(f: impl FnOnce() -> T) -> (T, Measure) {
    alloc_stats::reset_peak();
    let allocs0 = alloc_stats::allocations();
    let cpu0 = crate::cpu::process_cpu_seconds();
    let t0 = Instant::now();
    let out = f();
    let secs = t0.elapsed().as_secs_f64();
    let cpu = match (cpu0, crate::cpu::process_cpu_seconds()) {
        (Some(a), Some(b)) => b - a,
        _ => f64::NAN,
    };
    let m = Measure {
        secs,
        cpu_secs: cpu,
        peak_heap: alloc_stats::peak_above_start(),
        allocs: alloc_stats::allocations() - allocs0,
    };
    (out, m)
}

#[derive(Default, Clone)]
struct Row {
    label: String,
    pack: Option<Measure>,
    archive_bytes: u64,
    decode_one: f64,
    decode_par: f64,
    load: Measure,
    frames: FrameStats,
}

const FRAME_MS: f64 = 1000.0 / 60.0;

/// A deterministic CPU workload standing in for a frame's game logic and render submission.
fn busy(iterations: u64) -> u64 {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    for i in 0..iterations {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x = x.wrapping_add(i);
    }
    std::hint::black_box(x)
}

/// Iterations of [`busy`] taking about `work_ms`, from the median of 31 probes (the best
/// probe would reflect a turbo burst and undershoot the steady state).
fn calibrate(work_ms: f64) -> u64 {
    let probe = 2_000_000u64;
    let mut times: Vec<f64> = (0..31)
        .map(|_| {
            let t = Instant::now();
            busy(probe);
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    ((probe as f64) * work_ms / times[times.len() / 2]) as u64
}

/// Run frames at 60 Hz: workload, then `pump_async` (≤ 8 assets a frame), then sleep to the
/// next tick. Returns the main thread's busy time per frame.
fn frame_loop(iterations: u64, mut pump: impl FnMut() -> bool, min_frames: usize) -> Vec<f64> {
    let mut busy_ms = Vec::new();
    let start = Instant::now();
    let mut frame = 0u32;
    loop {
        let t = Instant::now();
        busy(iterations);
        let more = pump();
        busy_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        frame += 1;
        if !more && busy_ms.len() >= min_frames {
            break;
        }
        let next = start + Duration::from_secs_f64(f64::from(frame) * FRAME_MS / 1000.0);
        if let Some(wait) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        if start.elapsed() > Duration::from_secs(600) {
            break;
        }
    }
    busy_ms
}

fn pick_inputs(input: &Path, limit_bytes: u64) -> Result<Vec<(String, PathBuf, u64)>, String> {
    let all = collect_directory(input).map_err(|e| e.to_string())?;
    let mut files: Vec<(String, PathBuf, u64)> = all
        .into_iter()
        .filter(|i| !i.name.starts_with("saves/"))
        .filter_map(|i| match i.source {
            PackSource::File(p) => {
                let len = std::fs::metadata(&p).ok()?.len();
                Some((i.name, p, len))
            }
            PackSource::Bytes(_) => None,
        })
        .collect();
    let total: u64 = files.iter().map(|f| f.2).sum();
    if total > limit_bytes {
        // Every k-th file across the whole tree keeps the mix of kinds (art, models, text).
        let stride = total.div_ceil(limit_bytes).max(1) as usize;
        let mut picked = Vec::new();
        let mut held = 0u64;
        for (i, f) in files.into_iter().enumerate() {
            if i % stride == 0 && held + f.2 <= limit_bytes {
                held += f.2;
                picked.push(f);
            }
        }
        files = picked;
    }
    Ok(files)
}

fn decode_all(archive: &VlzArchive) -> f64 {
    let max = archive.entries().map(|e| e.raw_size).max().unwrap_or(0) as usize;
    let mut buf = vec![0u8; max];
    let raw: u64 = archive.entries().map(|e| e.raw_size).sum();
    let mut best = f64::MAX;
    for _ in 0..3 {
        let t = Instant::now();
        for e in archive.entries().collect::<Vec<_>>() {
            archive
                .read_entry_into(e.id, &mut buf[..e.raw_size as usize])
                .expect("decode");
        }
        best = best.min(t.elapsed().as_secs_f64());
    }
    raw as f64 / 1_048_576.0 / best
}

/// Load everything through `Assets` + `AssetStreamer` as fast as the workers go (the main
/// thread pumps everything that is ready, 1 ms apart): streaming throughput.
fn stream_throughput(source: Arc<dyn Source>, names: &[String]) -> Measure {
    let streamer = AssetStreamer::new(StreamConfig::default());
    let mut assets = Assets::with_source(source);
    let ((), m) = measure(|| {
        for n in names {
            assets.load_async::<BytesAsset>(&streamer, n.as_str(), LoadPriority::Normal);
        }
        while assets.pending_async() > 0 {
            assets.pump_async(&streamer, usize::MAX);
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    let failed = streamer.stats().failed;
    assert_eq!(failed, 0, "{failed} loads failed");
    m
}

/// Stream everything while a 60 Hz loop runs (≤ 8 assets picked up a frame), again and again
/// until at least `min_frames` frames were spent streaming.
fn frame_impact(
    source: Arc<dyn Source>,
    names: &[String],
    iterations: u64,
    min_frames: usize,
) -> FrameStats {
    let mut busy_ms = Vec::new();
    while busy_ms.len() < min_frames {
        let streamer = AssetStreamer::new(StreamConfig::default());
        let mut assets = Assets::with_source(Arc::clone(&source));
        for n in names {
            assets.load_async::<BytesAsset>(&streamer, n.as_str(), LoadPriority::Normal);
        }
        busy_ms.extend(frame_loop(
            iterations,
            || {
                assets.pump_async(&streamer, 8);
                assets.pending_async() > 0
            },
            1,
        ));
    }
    frame_stats(busy_ms)
}

fn mib(b: u64) -> f64 {
    b as f64 / 1_048_576.0
}

pub fn run(args: &[String]) -> Result<(), String> {
    let a = parse(args)?;
    let files = pick_inputs(&a.input, a.limit_mb * 1024 * 1024)?;
    let raw_total: u64 = files.iter().map(|f| f.2).sum();
    let names: Vec<String> = files.iter().map(|f| f.0.clone()).collect();
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("VLZ benchmark");
    println!("=============");
    println!(
        "input {} · {} files · {:.1} MiB · {} logical cores · {} / {}",
        a.input.display(),
        files.len(),
        mib(raw_total),
        cores,
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let mut by_kind: Vec<(String, u64, u64)> = Vec::new();
    for (name, _, len) in &files {
        let ext = name
            .rsplit_once('.')
            .map_or("(none)", |(_, e)| e)
            .to_ascii_lowercase();
        match by_kind.iter_mut().find(|k| k.0 == ext) {
            Some(k) => {
                k.1 += 1;
                k.2 += len;
            }
            None => by_kind.push((ext, 1, *len)),
        }
    }
    by_kind.sort_by_key(|k| std::cmp::Reverse(k.2));
    let kinds: Vec<String> = by_kind
        .iter()
        .take(8)
        .map(|(e, n, b)| format!("{e} {n} ({:.1} MiB)", mib(*b)))
        .collect();
    println!("mix: {}", kinds.join(", "));

    let iterations = calibrate(a.work_ms);
    let baseline = frame_stats(frame_loop(iterations, || false, 300));
    println!(
        "frame workload {:.1} ms (calibrated), budget {FRAME_MS:.2} ms; baseline busy p50 {:.2} p99 {:.2} max {:.2} ms",
        a.work_ms, baseline.p50, baseline.p99, baseline.max
    );

    let tmp = std::env::temp_dir().join(format!("velvet-vlz-bench-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();

    // Original loose files.
    {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let t = Instant::now();
            for (_, p, _) in &files {
                std::hint::black_box(std::fs::read(p).expect("read"));
            }
            best = best.min(t.elapsed().as_secs_f64());
        }
        let source: Arc<dyn Source> = Arc::new(FileSource::new(&a.input));
        let load = stream_throughput(Arc::clone(&source), &names);
        let frames = frame_impact(source, &names, iterations, 300);
        let mbps = mib(raw_total) / best;
        rows.push(Row {
            label: "original files".into(),
            pack: None,
            archive_bytes: raw_total,
            decode_one: mbps,
            decode_par: mbps,
            load,
            frames,
        });
    }

    let mut methods = vec![
        ("raw", Method::Fixed(Codec::Raw)),
        ("lz4", Method::Fixed(Codec::Lz4)),
        ("zstd (3)", Method::Fixed(Codec::Zstd)),
    ];
    if !a.skip_high {
        methods.push(("zstd-high (19)", Method::Fixed(Codec::ZstdHigh)));
    }
    methods.extend([
        ("auto speed", Method::Auto(Profile::Speed)),
        ("auto balanced", Method::Auto(Profile::Balanced)),
        ("auto size", Method::Auto(Profile::Size)),
    ]);
    for (label, method) in methods {
        let out = tmp.join(format!("{}.vlz", label.replace([' ', '(', ')'], "_")));
        let inputs: Vec<PackInput> = files
            .iter()
            .map(|(n, p, _)| PackInput::file(n.clone(), p.clone()))
            .collect();
        let opts = PackOptions {
            method,
            ..PackOptions::default()
        };
        let (report, pack) = measure(|| pack_to_file(inputs, &out, &opts));
        let report = report.map_err(|e| format!("{label}: {e}"))?;
        let one = VlzArchive::open_with(
            &out,
            OpenOptions {
                parallel_threads: 1,
                ..OpenOptions::default()
            },
        )
        .map_err(|e| e.to_string())?;
        let decode_one = decode_all(&one);
        drop(one);
        let par = VlzArchive::open(&out).map_err(|e| e.to_string())?;
        let decode_par = decode_all(&par);
        drop(par);
        let source: Arc<dyn Source> = Arc::new(VlzSource::open(&out).map_err(|e| e.to_string())?);
        let load = stream_throughput(Arc::clone(&source), &names);
        let frames = frame_impact(source, &names, iterations, 300);
        let row = Row {
            label: label.into(),
            pack: Some(pack),
            archive_bytes: report.archive_bytes,
            decode_one,
            decode_par,
            load,
            frames,
        };
        eprintln!("  {label}: done");
        rows.push(row);
        let _ = std::fs::remove_file(&out);
    }
    let _ = std::fs::remove_dir_all(&tmp);

    println!();
    println!("| method | size MiB | ratio | pack s | pack MiB/s | pack CPU s | pack peak heap MiB | decode 1 thread MiB/s | decode parallel MiB/s |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|");
    for r in &rows {
        let (ps, pmbps, pcpu, ppeak) = match &r.pack {
            Some(p) => (
                format!("{:.2}", p.secs),
                format!("{:.0}", mib(raw_total) / p.secs),
                format!("{:.2}", p.cpu_secs),
                format!("{:.0}", mib(p.peak_heap)),
            ),
            None => ("-".into(), "-".into(), "-".into(), "-".into()),
        };
        println!(
            "| {} | {:.1} | {:.1}% | {ps} | {pmbps} | {pcpu} | {ppeak} | {:.0} | {:.0} |",
            r.label,
            mib(r.archive_bytes),
            r.archive_bytes as f64 * 100.0 / raw_total as f64,
            r.decode_one,
            r.decode_par
        );
    }
    println!();
    println!("| method | stream s | stream MiB/s | stream CPU s | stream peak heap MiB | allocations | busy p50 ms | busy p95 ms | busy p99 ms | busy max ms | frames over {FRAME_MS:.2} ms |");
    println!("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|");
    println!(
        "| baseline (no streaming) | - | - | - | - | - | {:.2} | {:.2} | {:.2} | {:.2} | {}/{} |",
        baseline.p50,
        baseline.p95,
        baseline.p99,
        baseline.max,
        baseline.over_budget,
        baseline.frames
    );
    for r in &rows {
        let f = &r.frames;
        println!(
            "| {} | {:.3} | {:.0} | {:.2} | {:.0} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {}/{} |",
            r.label,
            r.load.secs,
            mib(raw_total) / r.load.secs,
            r.load.cpu_secs,
            mib(r.load.peak_heap),
            r.load.allocs,
            f.p50,
            f.p95,
            f.p99,
            f.max,
            f.over_budget,
            f.frames
        );
    }

    if let Some(path) = &a.json {
        let mut s = String::from("{\n");
        s.push_str(&format!(
            "  \"input\": {:?}, \"files\": {}, \"raw_bytes\": {raw_total}, \"cores\": {cores}, \"os\": {:?}, \"work_ms\": {},\n",
            a.input.display().to_string(),
            files.len(),
            std::env::consts::OS,
            a.work_ms
        ));
        s.push_str(&format!(
            "  \"baseline\": {{\"p50\": {:.3}, \"p95\": {:.3}, \"p99\": {:.3}, \"max\": {:.3}, \"over\": {}, \"frames\": {}}},\n  \"rows\": [\n",
            baseline.p50, baseline.p95, baseline.p99, baseline.max, baseline.over_budget, baseline.frames
        ));
        for (i, r) in rows.iter().enumerate() {
            let pack = r.pack.as_ref().map_or("null".to_string(), |p| {
                format!(
                    "{{\"secs\": {:.4}, \"cpu_secs\": {:.4}, \"peak_heap\": {}, \"allocs\": {}}}",
                    p.secs, p.cpu_secs, p.peak_heap, p.allocs
                )
            });
            s.push_str(&format!(
                "    {{\"method\": {:?}, \"archive_bytes\": {}, \"pack\": {pack}, \"decode_one_mibs\": {:.1}, \"decode_par_mibs\": {:.1}, \"load\": {{\"secs\": {:.4}, \"cpu_secs\": {:.4}, \"peak_heap\": {}, \"allocs\": {}}}, \"frames\": {{\"p50\": {:.3}, \"p95\": {:.3}, \"p99\": {:.3}, \"max\": {:.3}, \"over\": {}, \"frames\": {}}}}}{}\n",
                r.label, r.archive_bytes, r.decode_one, r.decode_par, r.load.secs, r.load.cpu_secs,
                r.load.peak_heap, r.load.allocs, r.frames.p50, r.frames.p95, r.frames.p99,
                r.frames.max, r.frames.over_budget, r.frames.frames,
                if i + 1 < rows.len() { "," } else { "" }
            ));
        }
        s.push_str("  ]\n}\n");
        std::fs::write(path, s).map_err(|e| e.to_string())?;
        println!("\njson -> {}", path.display());
    }
    Ok(())
}
