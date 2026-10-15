//! `velvet vlz …` — create, list, extract, verify and inspect VLZ (Velvet Compressed) archives.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use velvet_vlz::{
    collect_directory, pack_to_file, Codec, Method, OpenOptions, PackOptions, Profile, VlzArchive,
};

/// Options of `velvet vlz pack`, as parsed by clap.
pub struct VlzPackArgs {
    pub input: PathBuf,
    pub output: PathBuf,
    pub method: String,
    pub profile: String,
    pub block_size_kib: Option<u32>,
    pub align: u32,
    pub threads: usize,
    pub zstd_level: i32,
    pub zstd_high_level: i32,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.2} {}", UNITS[u])
    }
}

fn ratio(stored: u64, raw: u64) -> String {
    if raw == 0 {
        "-".into()
    } else {
        format!("{:.1}%", stored as f64 * 100.0 / raw as f64)
    }
}

fn codec_name(id: u8) -> String {
    Codec::from_id(id).map_or_else(|| format!("codec#{id}"), |c| c.name().to_string())
}

fn parse_method(method: &str, profile: &str) -> Result<Method> {
    let profile = Profile::parse(profile)
        .with_context(|| format!("unknown profile {profile:?} (speed|balanced|size)"))?;
    if method == "auto" {
        return Ok(Method::Auto(profile));
    }
    let codec = Codec::parse(method)
        .with_context(|| format!("unknown method {method:?} (auto|raw|lz4|zstd|zstd-high)"))?;
    Ok(Method::Fixed(codec))
}

fn open(archive: &Path) -> Result<VlzArchive> {
    VlzArchive::open_with(archive, OpenOptions::default())
        .with_context(|| format!("open {}", archive.display()))
}

/// `velvet vlz pack <input> <output>`: a directory (or one file) into an archive.
pub fn cmd_vlz_pack(args: VlzPackArgs) -> Result<()> {
    let method = parse_method(&args.method, &args.profile)?;
    let opts = PackOptions {
        method,
        block_size: args.block_size_kib.map(|k| k.saturating_mul(1024)),
        alignment: args.align,
        threads: args.threads,
        zstd_level: args.zstd_level,
        zstd_high_level: args.zstd_high_level,
        ..PackOptions::default()
    };
    let mut inputs = if args.input.is_dir() {
        collect_directory(&args.input).with_context(|| format!("scan {}", args.input.display()))?
    } else if args.input.is_file() {
        let name = args
            .input
            .file_name()
            .and_then(|n| n.to_str())
            .context("input file name is not UTF-8")?
            .to_string();
        vec![velvet_vlz::PackInput::file(name, &args.input)]
    } else {
        bail!("{} does not exist", args.input.display());
    };
    inputs.retain(|i| {
        (args.include.is_empty()
            || args
                .include
                .iter()
                .any(|p| velvet_build::path_matches(&i.name, p)))
            && !args
                .exclude
                .iter()
                .any(|p| velvet_build::path_matches(&i.name, p))
    });
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let started = Instant::now();
    let report = pack_to_file(inputs, &args.output, &opts)
        .with_context(|| format!("pack {}", args.output.display()))?;
    let secs = started.elapsed().as_secs_f64();
    println!(
        "vlz pack ok {} entries, {} -> {} ({} of the original) in {secs:.2} s ({:.0} MiB/s) -> {}",
        report.entries,
        human(report.raw_bytes),
        human(report.archive_bytes),
        ratio(report.archive_bytes, report.raw_bytes),
        report.raw_bytes as f64 / 1_048_576.0 / secs.max(1e-9),
        args.output.display()
    );
    println!(
        "  method {} · {} entries left as they were (already compressed)",
        match method {
            Method::Auto(p) => format!("auto ({})", p.name()),
            Method::Fixed(c) => c.name().to_string(),
        },
        report.precompressed_entries
    );
    for codec in Codec::ALL {
        let t = report.per_codec[codec.id() as usize];
        if t.blocks > 0 {
            println!(
                "  {:<9} {:>7} blocks {:>12} -> {:>12} ({})",
                codec.name(),
                t.blocks,
                human(t.raw_bytes),
                human(t.stored_bytes),
                ratio(t.stored_bytes, t.raw_bytes)
            );
        }
    }
    Ok(())
}

/// `velvet vlz list <archive>`.
pub fn cmd_vlz_list(archive: PathBuf, json: bool) -> Result<()> {
    let a = open(&archive)?;
    let codecs_of = |id| {
        let mut names: Vec<String> = a.blocks(id).iter().map(|b| codec_name(b.codec)).collect();
        names.sort();
        names.dedup();
        names.join("+")
    };
    if json {
        let entries: Vec<_> = a
            .entries()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "raw_size": e.raw_size,
                    "stored_size": e.stored_size,
                    "blocks": e.block_count,
                    "block_size": e.block_raw_size,
                    "class": e.content_class.name(),
                    "codecs": codecs_of(e.id),
                    "content_hash": format!("{:016x}", e.content_hash),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }
    println!(
        "{:>12} {:>12} {:>7} {:>6} {:<12} {:<14} name",
        "raw", "stored", "ratio", "blocks", "class", "codec"
    );
    for e in a.entries() {
        println!(
            "{:>12} {:>12} {:>7} {:>6} {:<12} {:<14} {}",
            human(e.raw_size),
            human(e.stored_size),
            ratio(e.stored_size, e.raw_size),
            e.block_count,
            e.content_class.name(),
            codecs_of(e.id),
            e.name
        );
    }
    println!("{} entries in {}", a.len(), archive.display());
    Ok(())
}

/// `velvet vlz extract <archive> <out-dir> [--entry NAME]...`.
pub fn cmd_vlz_extract(archive: PathBuf, output: PathBuf, entries: Vec<String>) -> Result<()> {
    let a = open(&archive)?;
    let ids: Vec<_> = if entries.is_empty() {
        a.entries().map(|e| e.id).collect()
    } else {
        entries
            .iter()
            .map(|n| {
                a.find(n)
                    .with_context(|| format!("no entry {n:?} in {}", archive.display()))
            })
            .collect::<Result<_>>()?
    };
    let started = Instant::now();
    let mut total = 0u64;
    const CHUNK: u64 = 64 * 1024 * 1024;
    for id in &ids {
        let e = a.entry(*id);
        // Names are validated when the archive opens: no `..`, no drive, no absolute path.
        let dest = output.join(e.name.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if e.raw_size <= CHUNK {
            let data = a
                .read_entry(*id)
                .with_context(|| format!("read {}", e.name))?;
            std::fs::write(&dest, data).with_context(|| format!("write {}", dest.display()))?;
        } else {
            use std::io::Write;
            let mut f = std::io::BufWriter::new(std::fs::File::create(&dest)?);
            let mut buf = vec![0u8; CHUNK as usize];
            let mut at = 0;
            while at < e.raw_size {
                let n = CHUNK.min(e.raw_size - at) as usize;
                a.read_range_into(*id, at, &mut buf[..n])
                    .with_context(|| format!("read {}", e.name))?;
                f.write_all(&buf[..n])?;
                at += n as u64;
            }
            f.flush()?;
        }
        total += e.raw_size;
    }
    println!(
        "vlz extract ok {} entries, {} in {:.2} s -> {}",
        ids.len(),
        human(total),
        started.elapsed().as_secs_f64(),
        output.display()
    );
    Ok(())
}

/// `velvet vlz verify <archive>`: decode everything, check every checksum and content hash.
pub fn cmd_vlz_verify(archive: PathBuf) -> Result<()> {
    let started = Instant::now();
    let a = open(&archive)?;
    let report = a.verify();
    let secs = started.elapsed().as_secs_f64();
    if !report.is_ok() {
        for (name, why) in report.failures.iter().take(50) {
            eprintln!("  FAIL {name}: {why}");
        }
        bail!(
            "{} of {} entries failed verification in {}",
            report.failures.len(),
            report.entries,
            archive.display()
        );
    }
    println!(
        "vlz verify ok {} entries, {} blocks, {} decoded in {secs:.2} s ({:.0} MiB/s) — {}",
        report.entries,
        report.blocks,
        human(report.raw_bytes),
        report.raw_bytes as f64 / 1_048_576.0 / secs.max(1e-9),
        archive.display()
    );
    Ok(())
}

/// `velvet vlz stats <archive>`.
pub fn cmd_vlz_stats(archive: PathBuf, json: bool) -> Result<()> {
    let a = open(&archive)?;
    let s = a.stats();
    let h = a.header();
    if json {
        let codecs: Vec<_> = s
            .per_codec
            .iter()
            .map(|(id, t)| {
                serde_json::json!({"codec": codec_name(*id), "blocks": t.blocks,
                    "raw_bytes": t.raw_bytes, "stored_bytes": t.stored_bytes})
            })
            .collect();
        let classes: Vec<_> = s
            .per_class
            .iter()
            .map(|(c, n, raw, stored)| {
                serde_json::json!({"class": c.name(), "entries": n, "raw_bytes": raw, "stored_bytes": stored})
            })
            .collect();
        let doc = serde_json::json!({
            "version": format!("{}.{}", h.version_major, h.version_minor),
            "file_bytes": s.file_bytes, "index_bytes": s.index_bytes,
            "entries": s.entries, "blocks": s.blocks,
            "raw_bytes": s.raw_bytes, "stored_bytes": s.stored_bytes,
            "alignment": h.alignment, "codecs": codecs, "classes": classes,
        });
        println!("{}", serde_json::to_string_pretty(&doc)?);
        return Ok(());
    }
    println!(
        "VLZ {}.{} — {}",
        h.version_major,
        h.version_minor,
        archive.display()
    );
    println!(
        "  {} entries, {} blocks, alignment {} B",
        s.entries, s.blocks, h.alignment
    );
    println!(
        "  original {} · stored {} ({}) · file {} ({}) · index {}",
        human(s.raw_bytes),
        human(s.stored_bytes),
        ratio(s.stored_bytes, s.raw_bytes),
        human(s.file_bytes),
        ratio(s.file_bytes, s.raw_bytes),
        human(s.index_bytes)
    );
    println!("  by codec:");
    for (id, t) in &s.per_codec {
        println!(
            "    {:<10} {:>7} blocks {:>12} -> {:>12} ({})",
            codec_name(*id),
            t.blocks,
            human(t.raw_bytes),
            human(t.stored_bytes),
            ratio(t.stored_bytes, t.raw_bytes)
        );
    }
    println!("  by content:");
    for (class, n, raw, stored) in &s.per_class {
        println!(
            "    {:<12} {:>6} entries {:>12} -> {:>12} ({})",
            class.name(),
            n,
            human(*raw),
            human(*stored),
            ratio(*stored, *raw)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_list_verify_extract_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("assets");
        std::fs::create_dir_all(src.join("story")).unwrap();
        std::fs::write(src.join("story/intro.json"), "{\"a\":1}".repeat(500)).unwrap();
        std::fs::write(src.join("skip.tmp"), "x").unwrap();
        let archive = dir.path().join("out/game.vlz");
        cmd_vlz_pack(VlzPackArgs {
            input: src.clone(),
            output: archive.clone(),
            method: "auto".into(),
            profile: "balanced".into(),
            block_size_kib: None,
            align: 16,
            threads: 0,
            zstd_level: 3,
            zstd_high_level: 19,
            include: vec![],
            exclude: vec!["**/*.tmp".into()],
        })
        .unwrap();
        cmd_vlz_list(archive.clone(), false).unwrap();
        cmd_vlz_list(archive.clone(), true).unwrap();
        cmd_vlz_stats(archive.clone(), false).unwrap();
        cmd_vlz_verify(archive.clone()).unwrap();
        let out = dir.path().join("unpacked");
        cmd_vlz_extract(archive.clone(), out.clone(), vec![]).unwrap();
        assert_eq!(
            std::fs::read(out.join("story/intro.json")).unwrap(),
            std::fs::read(src.join("story/intro.json")).unwrap()
        );
        assert!(!out.join("skip.tmp").exists());
        assert!(parse_method("brotli", "balanced").is_err());
        assert!(parse_method("auto", "fast").is_err());
    }
}
