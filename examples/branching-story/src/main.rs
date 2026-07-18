//! Branching story demo — exercises multiple endings via scripted choice paths.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use velvet_story::prelude::*;

fn story_path() -> PathBuf {
    let candidates = [
        PathBuf::from("examples/branching-story/story/routes.vel"),
        PathBuf::from("story/routes.vel"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("story/routes.vel"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("story/routes.vel"))
}

fn play_with_choices(choices: &[usize]) -> Result<(String, i64, i64, bool)> {
    let path = story_path();
    let source = std::fs::read_to_string(&path).with_context(|| format!("{}", path.display()))?;
    let program = load_program_from_source(&source, Some(&path.to_string_lossy()), "Branching")?;
    let mut player = StoryPlayer::start(program);
    let mut qi = 0usize;
    let mut steps = 0;

    loop {
        steps += 1;
        if steps > 5000 {
            bail!("step limit");
        }
        match player.wait().clone() {
            StoryWait::Line => player.advance(),
            StoryWait::Choice => {
                let pick = choices.get(qi).copied().unwrap_or(0);
                qi += 1;
                player.choose(pick).map_err(|e| anyhow::anyhow!(e))?;
            }
            StoryWait::Ended => break,
            StoryWait::Ready => player.advance(),
        }
    }

    let ending = player.variables().get("ending").display_str();
    let elena = player.variables().get_int("elena_rel", 0);
    let marco = player.variables().get_int("marco_rel", 0);
    let key = player.variables().get("has_key").is_truthy();
    Ok((ending, elena, marco, key))
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("branching=info,info");

    // Path A: joke + search Elena → key true, elena_rel higher
    let (e1, el1, m1, k1) = play_with_choices(&[0, 0, 0])?;
    println!("Path A (Elena): ending={e1} elena={el1} marco={m1} key={k1}");

    // Path B: ask key + search Marco
    let (e2, el2, m2, k2) = play_with_choices(&[1, 1, 0])?;
    println!("Path B (Marco): ending={e2} elena={el2} marco={m2} key={k2}");

    // Path C: alone + wait
    let (e3, el3, m3, k3) = play_with_choices(&[0, 2, 1])?;
    println!("Path C (Alone/Wait): ending={e3} elena={el3} marco={m3} key={k3}");

    assert!(k1, "Elena path should find key");
    assert!(k2, "Marco path should find key");
    assert!(!k3, "Alone path should not find key");
    assert!(
        el1 > el3,
        "Elena path should raise elena_rel more than alone"
    );
    assert_ne!(e1, "none");
    assert_eq!(e3, "timeless");

    // Save/load on path A mid-run
    let path = story_path();
    let source = std::fs::read_to_string(&path)?;
    let program = load_program_from_source(&source, None, "Branching")?;
    let mut player = StoryPlayer::start(program.clone());
    player.advance(); // first line
    if matches!(player.wait(), StoryWait::Line) {
        player.advance();
    }
    // at first choice
    if matches!(player.wait(), StoryWait::Choice) {
        player.choose(0).unwrap();
    }
    let dir = tempfile::tempdir()?;
    let store = SaveStore::new(dir.path());
    store.write(&player.to_save("mid"))?;
    let mut p2 = StoryPlayer::start(program);
    p2.load_save(store.read("mid")?)
        .map_err(|e| anyhow::anyhow!(e))?;
    assert_eq!(p2.variables().get_int("elena_rel", 0), 1);

    println!("branching-story demo OK");
    Ok(())
}
