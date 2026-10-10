//! cargo run -p velvet-dungeon --bin dungeon-check -- --count 1000 --out temp/dungeon-report
use velvet_dungeon::{gameplay::Difficulty, *};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
    };
    let start = arg("--seed")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let count = arg("--count")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);
    let out = std::path::PathBuf::from(
        arg("--out")
            .map(String::as_str)
            .unwrap_or("temp/dungeon-report"),
    );
    let config: Config = if let Some(path) = arg("--config") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        Config::default()
    };
    let catalog = Catalog::bundled();
    let mut failed = Vec::new();
    let mut times = Vec::new();
    let mut hashes = std::collections::BTreeSet::new();
    let mut samples = Vec::new();
    std::fs::create_dir_all(&out)?;
    for seed in start..start.saturating_add(count) {
        match generate(seed, &config, &catalog, &Difficulty::default()) {
            Err(e) => failed.push((seed, e)),
            Ok(map) => {
                if args.iter().any(|a| a == "--walk") {
                    for c in &map.connections {
                        let points = [map.rooms[c.a].center, c.start, c.end, map.rooms[c.b].center];
                        if let Err(e) = controller::traverse(&map, &points) {
                            failed.push((seed, format!("forward {}: {e}", c.id)));
                            break;
                        }
                        if let Err(e) = controller::traverse(
                            &map,
                            &points.into_iter().rev().collect::<Vec<_>>(),
                        ) {
                            failed.push((seed, format!("backward {}: {e}", c.id)));
                            break;
                        }
                    }
                }
                times.push(map.stats.generation_ms);
                hashes.insert(map.structural_hash());
                if samples.len() < 4 || count == 1 {
                    std::fs::write(out.join(format!("seed-{seed}.svg")), diagnostics::svg(&map))?;
                    std::fs::write(
                        out.join(format!("seed-{seed}.json")),
                        serde_json::to_vec_pretty(&map)?,
                    )?;
                    samples.push(map.stats);
                }
            }
        }
    }
    times.sort_by(f64::total_cmp);
    let report = serde_json::json!({"version":GENERATOR_VERSION,"first_seed":start,"count":count,"failures":failed,"unique_layouts":hashes.len(),"median_ms":times.get(times.len()/2),"p95_ms":times.get(times.len()*95/100),"max_ms":times.last(),"samples":samples});
    std::fs::write(out.join("report.json"), serde_json::to_vec_pretty(&report)?)?;
    println!("{report}");
    if !failed.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
