//! Third-person vertical slice: declarative arena, physics hero, chasing
//! enemy, beacon quest, blended locomotion, bursts, and saves.
//!
//! The whole program lives in this library so the desktop binary (`main.rs`) and the Android
//! app run the same code: [`run`] is the windowed game on both (`velvet-platform` gives the event
//! loop, packaged files, save folder, GPU surface lifecycle and touch), [`run_headless`] the
//! scripted CI walk. On Android, `velvet_platform::android_main!` exports the entry point and
//! `velvet android apk -p third-person` packs the library and `scenes/` into an APK (see
//! `[package.metadata.velvet.android]` in Cargo.toml and `docs/platforms/ANDROID.md`).

mod app;
pub mod sim;

use anyhow::{Context, Result};
use velvet_math::Vec3;
use velvet_scene3d::{load_scene_from_str, Scene3D};

use sim::{QuestState, SimInput, ThirdPersonSim};

/// The arena, as a packaged file: `scenes/` beside Cargo.toml on desktop, the APK's
/// `assets/scenes/` on Android.
const SCENE: &str = "scenes/arena.scene3d.json";

fn load_arena() -> Result<Scene3D> {
    // Desktop: the packaged files are this crate's folder (ignored on Android).
    velvet_platform::files::set_assets_root(env!("CARGO_MANIFEST_DIR"));
    let text = velvet_platform::files::assets()
        .read_to_string(SCENE)
        .with_context(|| format!("read {SCENE}"))?;
    load_scene_from_str(&text).map_err(|e| anyhow::anyhow!("{e}"))
}

/// The windowed slice (desktop and Android).
pub fn run() -> Result<()> {
    app::run(load_arena()?)
}

velvet_platform::android_main!(run);

/// Scripted CI walk: steer at the beacon until the quest completes.
pub fn run_headless() -> Result<()> {
    let scene = load_arena()?;
    let mut sim = ThirdPersonSim::new(&scene);
    let mut steps = 0;
    while sim.quest != QuestState::Done && steps < 3600 {
        steps += 1;
        let h = sim.bodies["hero"].position;
        let d = Vec3::new(sim.beacon.x - h.x, 0.0, sim.beacon.z - h.z);
        let len = (d.x * d.x + d.z * d.z).sqrt().max(1e-6);
        sim.tick(
            SimInput {
                move_dir: Vec3::new(d.x / len, 0.0, d.z / len),
                speed: 1.0,
                jump: false,
            },
            1.0 / 60.0,
        );
        if steps % 600 == 0 {
            println!(
                "headless t={:.1}s hero=({:.1},{:.1},{:.1}) hp={} quest={:?} particles={}",
                sim.time,
                h.x,
                h.y,
                h.z,
                sim.hp,
                sim.quest,
                sim.particles.len()
            );
        }
    }
    anyhow::ensure!(
        sim.quest == QuestState::Done,
        "scripted walk did not reach the beacon"
    );
    // Save roundtrip through a temp file.
    let dir = std::env::temp_dir().join("third-person-headless");
    std::fs::create_dir_all(&dir)?;
    let save_path = dir.join("slot.json");
    sim.save_to(&save_path)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let back = velvet_scene3d::SceneSave3D::from_json(
        &std::fs::read_to_string(&save_path).context("read save")?,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut fresh = ThirdPersonSim::new(&scene);
    fresh.load(&back);
    anyhow::ensure!(
        fresh.bodies["hero"].position == sim.bodies["hero"].position,
        "save roundtrip mismatch"
    );
    println!(
        "headless ok: quest done in {steps} steps, save roundtrip ok ({} bodies)",
        back.bodies.len()
    );
    Ok(())
}
