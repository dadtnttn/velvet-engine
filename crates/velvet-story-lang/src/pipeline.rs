//! Full pipeline: parse → include resolve → sema → **StoryProgram** → StoryPlayer.

use std::path::Path;

use crate::ast::StoryFile;
use crate::commands::CommandRegistry;
use crate::diag::StoryDiag;
use crate::format::format_source;
use crate::load::{load_story_path, load_story_source};
use crate::locale::{with_diag_locale, DiagLocale};
use crate::parser::{parse, ParseResult};
use crate::sema::{self, SemaResult};
use crate::studio::StudioModel;
use crate::to_story_program::to_story_program_cmds;
use velvet_story::{StoryPlayer, StoryProgram, StoryValue, StoryWait};

/// Options for check/build that must be isolation-safe across concurrent docs.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckOptions {
    /// When set, all diagnostics for this call use this locale (thread-scoped).
    /// When `None`, uses the effective locale ([`crate::locale::diag_locale`]).
    pub locale: Option<DiagLocale>,
}

impl CheckOptions {
    /// Default options (process / thread effective locale).
    pub fn new() -> Self {
        Self::default()
    }

    /// Force diagnostic language for this check.
    pub fn with_locale(mut self, locale: DiagLocale) -> Self {
        self.locale = Some(locale);
        self
    }
}

/// Combined check result.
#[derive(Debug)]
pub struct CheckResult {
    /// Parse (root file; includes may be merged into `file`).
    pub parsed: ParseResult,
    /// Fully resolved story file (includes expanded).
    pub file: StoryFile,
    /// Sema.
    pub sema: SemaResult,
    /// All diags.
    pub diags: Vec<StoryDiag>,
    /// Ok if no errors.
    pub ok: bool,
}

/// Check a story source (includes resolved relative to `file` parent if path-like).
pub fn check_source(source: &str, file: &str, cmds: &CommandRegistry) -> CheckResult {
    check_source_with(source, file, cmds, CheckOptions::default())
}

/// Check with isolation-safe options (e.g. per-document locale).
pub fn check_source_with(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    opts: CheckOptions,
) -> CheckResult {
    let run = || check_source_inner(source, file, cmds);
    match opts.locale {
        Some(loc) => with_diag_locale(loc, run),
        None => run(),
    }
}

fn check_source_inner(source: &str, file: &str, cmds: &CommandRegistry) -> CheckResult {
    let base = Path::new(file).parent();
    let (story, load_diags) = match load_story_source(source, file, base) {
        Ok(v) => v,
        Err(e) => {
            let parsed = parse(source, file);
            let mut diags = parsed.diags.clone();
            diags.push(StoryDiag::error_key(
                "VST043",
                &[("detail", e.as_str())],
                file,
                crate::span::Span::unknown(),
            ));
            return CheckResult {
                parsed,
                file: StoryFile {
                    file: file.into(),
                    items: vec![],
                },
                sema: SemaResult::default(),
                diags,
                ok: false,
            };
        }
    };
    // Keep raw parse for AST dump compatibility
    let parsed = parse(source, file);
    let mut diags = load_diags;
    diags.extend(parsed.diags.iter().cloned());
    let sema = sema::analyze(&story, cmds);
    diags.extend(sema.diags.clone());
    let ok = !diags.iter().any(|d| d.is_error());
    CheckResult {
        parsed,
        file: story,
        sema,
        diags,
        ok,
    }
}

/// Check path (resolves includes from disk).
pub fn check_path(path: &Path, cmds: &CommandRegistry) -> Result<CheckResult, String> {
    check_path_with(path, cmds, CheckOptions::default())
}

/// Check path with isolation-safe options (e.g. per-document locale).
pub fn check_path_with(
    path: &Path,
    cmds: &CommandRegistry,
    opts: CheckOptions,
) -> Result<CheckResult, String> {
    let run = || check_path_inner(path, cmds);
    match opts.locale {
        Some(loc) => with_diag_locale(loc, run),
        None => run(),
    }
}

fn check_path_inner(path: &Path, cmds: &CommandRegistry) -> Result<CheckResult, String> {
    let (story, load_diags) = load_story_path(path)?;
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().to_string();
    let parsed = parse(&source, &file);
    let mut diags = load_diags;
    diags.extend(parsed.diags.iter().cloned());
    let sema = sema::analyze(&story, cmds);
    diags.extend(sema.diags.clone());
    let ok = !diags.iter().any(|d| d.is_error());
    Ok(CheckResult {
        parsed,
        file: story,
        sema,
        diags,
        ok,
    })
}

/// Product path from disk: StoryProgram → StoryPlayer (author default).
pub fn run_path_product(
    path: &Path,
    cmds: &CommandRegistry,
    choice: usize,
) -> Result<ProgramRunResult, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().to_string();
    run_source_product(&source, &file, cmds, choice)
}

/// Format path in place (or check-only).
///
/// `check_only`: returns `Err("needs formatting")` when formatted text differs
/// from the file on disk (compares `pretty` to original `source`, not to itself).
pub fn format_path(path: &Path, check_only: bool) -> Result<String, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pretty = format_source(&source);
    let twice = format_source(&pretty);
    if twice != pretty {
        return Err("formatter not idempotent".into());
    }
    if check_only {
        // Compare formatted output to original file contents.
        if pretty != source {
            return Err("needs formatting".into());
        }
        return Ok(pretty);
    }
    std::fs::write(path, &pretty).map_err(|e| e.to_string())?;
    Ok(pretty)
}

/// Dump the canonical [`StoryProgram`] as JSON (what StoryPlayer runs).
pub fn dump_program_json(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    title: &str,
) -> Result<String, String> {
    let program = build_story_program(source, file, cmds, title)?;
    serde_json::to_string_pretty(&program).map_err(|e| e.to_string())
}

/// Dump AST as JSON.
pub fn dump_ast_json(source: &str, file: &str) -> Result<String, String> {
    let p = parse(source, file);
    serde_json::to_string_pretty(&p.file).map_err(|e| e.to_string())
}

/// Studio model from source.
pub fn studio_model(source: &str, file: &str, cmds: &CommandRegistry) -> StudioModel {
    crate::studio::build_model(source, file, cmds)
}

/// Build product [`StoryProgram`] from writer source (Velvet 2.5 primary IR).
pub fn build_story_program(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    title: &str,
) -> Result<StoryProgram, String> {
    build_story_program_with(source, file, cmds, title, CheckOptions::default())
}

// Note: build_story_program_with is defined below and applies command defaults.

/// Result of running a StoryProgram on the product [`StoryPlayer`].
/// Outcome of a product-path story run.
#[derive(Debug)]
pub struct ProgramRunResult {
    /// Dialogue lines shown (`speaker: text`).
    pub dialogue: Vec<String>,
    /// Variable snapshot after run (play layer).
    pub vars: Vec<(String, String)>,
    /// Whether the story reached an end state.
    pub ended: bool,
    /// Steps advanced.
    pub steps: u32,
}

/// Run StoryProgram headless via product StoryPlayer (preferred 2.5 path).
pub fn run_story_program(program: StoryProgram, choice: usize, max_steps: u32) -> ProgramRunResult {
    let mut player = StoryPlayer::start(program);
    let mut dialogue = Vec::new();
    let mut steps = 0u32;
    while steps < max_steps {
        steps += 1;
        match player.wait().clone() {
            StoryWait::Ended => break,
            StoryWait::Line => {
                let sp = player.current_speaker_name().to_string();
                let tx = player.current_text().to_string();
                if !tx.is_empty() {
                    if sp.is_empty() {
                        dialogue.push(format!("narrator: {tx}"));
                    } else {
                        dialogue.push(format!("{sp}: {tx}"));
                    }
                }
                player.advance();
            }
            StoryWait::Choice => {
                let idx = choice.min(player.choices().len().saturating_sub(1));
                let arm = player.choices().get(idx).map(|c| c.index).unwrap_or(0);
                let _ = player.choose(arm);
            }
            StoryWait::Ready | StoryWait::Pause { .. } => {
                player.advance();
            }
            StoryWait::ScreenCall { .. } => {
                // Headless product path: modal screens return null.
                let _ = player.return_from_screen(StoryValue::Null);
            }
            StoryWait::Host { token } => {
                // Headless product path: auto-resume host waits.
                let _ = player.resume_host(&token);
            }
        }
    }
    let mut vars = Vec::new();
    for (k, v) in player.variables().play.iter() {
        vars.push((k.clone(), v.display_str()));
    }
    ProgramRunResult {
        dialogue,
        vars,
        ended: player.is_ended(),
        steps,
    }
}

/// Check → StoryProgram → product run (**writer primary path**).
///
/// This is the authoring runtime surface.
pub fn run_source_product(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    choice: usize,
) -> Result<ProgramRunResult, String> {
    run_source_product_with(source, file, cmds, choice, CheckOptions::default())
}

/// Product run with isolation-safe check options (locale, …).
pub fn run_source_product_with(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    choice: usize,
    opts: CheckOptions,
) -> Result<ProgramRunResult, String> {
    let prog = build_story_program_with(source, file, cmds, file, opts)?;
    Ok(run_story_program(prog, choice, 256))
}

/// Build product StoryProgram with check options.
pub fn build_story_program_with(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    title: &str,
    opts: CheckOptions,
) -> Result<StoryProgram, String> {
    let check = check_source_with(source, file, cmds, opts);
    if !check.ok {
        let msgs: Vec<_> = check.diags.iter().map(|d| d.display()).collect();
        return Err(msgs.join("\n"));
    }
    to_story_program_cmds(&check.file, title, cmds).map_err(|e| e.to_string())
}
