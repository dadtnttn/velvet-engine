//! Load Velvet Arcana flow from `.vstory` (author language) with stakes + style commands.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use velvet_story::{SharedCommandHost, StoryPlayer};
use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::pipeline::build_story_program;

use crate::host::{register_stakes_commands, StakesHost};

const EMBEDDED_STORY: &str = include_str!("../data/story/main.vstory");

/// Locate story file next to demo data.
pub fn story_path(data_root: &Path) -> PathBuf {
    let candidates = [
        data_root.join("story/main.vstory"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/story/main.vstory"),
        PathBuf::from("demos/velvet-stakes/data/story/main.vstory"),
    ];
    candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| data_root.join("story/main.vstory"))
}

/// Compile `.vstory` → StoryProgram and start player with host.
pub fn boot_player(host: Arc<StakesHost>, data_root: &Path) -> Result<StoryPlayer> {
    let path = story_path(data_root);
    let source = if path.exists() {
        std::fs::read_to_string(&path).with_context(|| format!("read story {}", path.display()))?
    } else {
        EMBEDDED_STORY.to_string()
    };
    let file = path.to_str().unwrap_or("main.vstory").to_string();

    let mut cmds = CommandRegistry::builtin();
    register_stakes_commands(&mut cmds);

    let program = build_story_program(&source, &file, &cmds, "Velvet Arcana")
        .map_err(|e| anyhow::anyhow!("vstory compile: {e}"))?;

    if program.scenes.is_empty() {
        bail!("story has no scenes");
    }

    let shared: SharedCommandHost = host;
    Ok(StoryPlayer::start_with_host(program, shared))
}
