//! Product bootstrap: load a writer story file into [`StoryProgram`] / play helpers.
//!
//! Keeps `velvet-story` free of a dependency on this crate (story-lang already
//! depends on story). CLI and runtime call these entry points so `.vstory` and
//! the product player share one path.

use std::path::Path;

use velvet_story::{open_session_from_file, StoryPlayer, StoryProgram, VnSession};

use crate::commands::CommandRegistry;
use crate::pipeline::{build_story_program, run_story_program, ProgramRunResult};

/// Errors loading a story for product play.
#[derive(Debug)]
pub enum BootError {
    /// I/O.
    Io(String),
    /// Compile / semantic.
    Compile(String),
    /// Unsupported path.
    Unsupported(String),
}

impl std::fmt::Display for BootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(s) | Self::Compile(s) | Self::Unsupported(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for BootError {}

/// True when `path` should use the Velvet Story writer pipeline (`.vstory`).
pub fn is_vstory_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("vstory"))
        .unwrap_or(false)
}

/// Load a story file into product [`StoryProgram`].
///
/// * `.vstory` → lexer/parser/sema → StoryProgram (this crate)
/// * `.vel` / other → [`velvet_story::load_program_from_source`] (legacy product IR)
pub fn load_story_program_from_path(path: &Path) -> Result<StoryProgram, BootError> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| BootError::Io(format!("read {}: {e}", path.display())))?;
    let file = path.to_string_lossy().to_string();
    let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("story");
    if is_vstory_path(path) {
        let cmds = CommandRegistry::builtin();
        build_story_program(&source, &file, &cmds, title).map_err(BootError::Compile)
    } else {
        velvet_story::load_program_from_source(&source, Some(&file), title)
            .map_err(|e| BootError::Compile(e.to_string()))
    }
}

/// Open a product [`VnSession`] from `.vstory` or legacy story file.
pub fn open_session_from_story_path(
    path: &Path,
    save_dir: Option<std::path::PathBuf>,
) -> Result<VnSession, BootError> {
    if is_vstory_path(path) {
        let program = load_story_program_from_path(path)?;
        let player = StoryPlayer::start(program);
        let mut session = VnSession::new(player);
        if let Some(dir) = save_dir {
            session = session.with_save_dir(dir);
        }
        if let Some(parent) = path.parent() {
            session = session.with_project_root(parent.to_path_buf());
        }
        Ok(session)
    } else {
        let title = path.file_stem().and_then(|s| s.to_str()).unwrap_or("story");
        open_session_from_file(path, title, save_dir).map_err(|e| BootError::Compile(e.to_string()))
    }
}

/// Headless product run of a story path (choice index fixed).
pub fn run_story_path_headless(
    path: &Path,
    choice: usize,
    max_steps: u32,
) -> Result<ProgramRunResult, BootError> {
    let program = load_story_program_from_path(path)?;
    Ok(run_story_program(program, choice, max_steps))
}

/// Convenience: start a player only.
pub fn start_player_from_path(path: &Path) -> Result<StoryPlayer, BootError> {
    Ok(StoryPlayer::start(load_story_program_from_path(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn boot_vstory_welcome_runs_product_dialogue() {
        let src = r#"
scene start
narrator:
    Hello from boot.
end
"#;
        let mut f = NamedTempFile::with_suffix(".vstory").unwrap();
        write!(f, "{src}").unwrap();
        let r = run_story_path_headless(f.path(), 0, 64).expect("boot run");
        assert!(
            r.dialogue.iter().any(|l| l.contains("Hello from boot")),
            "dialogue={:?}",
            r.dialogue
        );
        assert!(r.ended || r.steps > 0);
    }

    #[test]
    fn boot_loads_vstory_program_has_start() {
        let src = "scene start\nnarrator:\n    x\nend\n";
        let mut f = NamedTempFile::with_suffix(".vstory").unwrap();
        write!(f, "{src}").unwrap();
        let prog = load_story_program_from_path(f.path()).unwrap();
        assert!(prog.scenes.contains_key("start"));
        assert!(!prog.content_hash().is_empty());
    }
}
