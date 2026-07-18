//! Full pipeline: parse → include resolve → sema → lower → VS2 unit → host execution.

use std::path::Path;

use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_vm::{Vs2Host, Vs2MiniVm};

use crate::ast::StoryFile;
use crate::commands::CommandRegistry;
use crate::diag::StoryDiag;
use crate::format::format_source;
use crate::load::{load_story_path, load_story_source};
use crate::lower::{dump_lowered, lower, LowerOutput};
use crate::parser::{parse, ParseResult};
use crate::sema::{self, SemaResult};
use crate::source_map::SourceMap;
use crate::studio::StudioModel;
use crate::to_story_program::to_story_program;
use velvet_story::{StoryPlayer, StoryProgram, StoryWait};

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
    let base = Path::new(file).parent();
    let (story, load_diags) = match load_story_source(source, file, base) {
        Ok(v) => v,
        Err(e) => {
            let parsed = parse(source, file);
            let mut diags = parsed.diags.clone();
            diags.push(StoryDiag::error(
                "VST043",
                e,
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

/// Build result (lowered unit).
#[derive(Debug)]
pub struct BuildResult {
    /// Check.
    pub check: CheckResult,
    /// Lowered (if check ok or partial).
    pub lowered: Option<LowerOutput>,
    /// Ok.
    pub ok: bool,
}

/// Build: check + lower (on include-resolved file).
///
/// Prefer **StoryProgram** as the canonical IR. The OpVs2 unit is derived from
/// that program when possible (single spine); legacy direct HIR lower is only
/// a last-resort fallback.
pub fn build_source(source: &str, file: &str, cmds: &CommandRegistry) -> BuildResult {
    let check = check_source(source, file, cmds);
    if !check.ok {
        return BuildResult {
            check,
            lowered: None,
            ok: false,
        };
    }
    // Single spine: AST → StoryProgram → OpVs2
    let mut check = check;
    let lowered = match to_story_program(&check.file, file) {
        Ok(prog) => {
            let unit = crate::from_program::story_program_to_vs2(&prog);
            // Build a LowerOutput-compatible package via legacy helper + replace unit
            let mut lo = lower(&check.file);
            lo.unit = unit;
            lo
        }
        Err(e) => {
            check.diags.push(crate::diag::StoryDiag::error(
                "VST060",
                e.to_string(),
                file,
                crate::span::Span::unknown(),
            ));
            lower(&check.file)
        }
    };
    check.diags.extend(lowered.diags.clone());
    let ok = !check.diags.iter().any(|d| d.is_error());
    BuildResult {
        check,
        lowered: Some(lowered),
        ok,
    }
}

/// Build from path (includes on disk).
pub fn build_path(path: &Path, cmds: &CommandRegistry) -> Result<BuildResult, String> {
    let check = check_path(path, cmds)?;
    if !check.ok {
        return Ok(BuildResult {
            check,
            lowered: None,
            ok: false,
        });
    }
    let lowered = lower(&check.file);
    let mut check = check;
    check.diags.extend(lowered.diags.clone());
    let ok = !check.diags.iter().any(|d| d.is_error());
    Ok(BuildResult {
        check,
        lowered: Some(lowered),
        ok,
    })
}

/// Run result (observable host state).
#[derive(Debug)]
pub struct RunResult {
    /// Build.
    pub build: BuildResult,
    /// Dialogue lines produced.
    pub dialogue: Vec<String>,
    /// Host log.
    pub log: Vec<String>,
    /// Variables in host state.
    pub state: Vec<(String, String)>,
    /// Instructions executed.
    pub steps: usize,
    /// Ok.
    pub ok: bool,
}

/// Execute lowered story on VS2 host (existing OpVs2 / Vs2MiniVm — not a second language VM).
pub fn run_source(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    choice_index: usize,
) -> RunResult {
    let build = build_source(source, file, cmds);
    run_build(build, choice_index)
}

/// Run from path with include resolution.
pub fn run_path(path: &Path, cmds: &CommandRegistry, choice_index: usize) -> Result<RunResult, String> {
    let build = build_path(path, cmds)?;
    Ok(run_build(build, choice_index))
}

fn run_build(build: BuildResult, choice_index: usize) -> RunResult {
    // shared body was previously inline in run_source
    let _cmds_marker = choice_index;
    if !build.ok {
        return RunResult {
            build,
            dialogue: vec![],
            log: vec![],
            state: vec![],
            steps: 0,
            ok: false,
        };
    }
    let lowered = build.lowered.as_ref().unwrap();
    let mut host = Vs2Host::new();
    host.pool = lowered.unit.pool.strings.clone();
    // seed translations from msg ids
    for (id, text) in &lowered.msg_ids {
        host.set_translation(id, text);
    }
    // also map pool strings that look like msg keys if host LoadMsg uses pool
    for (i, s) in host.pool.clone().iter().enumerate() {
        if let Some((_, t)) = lowered.msg_ids.iter().find(|(id, _)| id == s) {
            host.set_translation(s, t);
        }
        let _ = i;
    }

    let code: Vec<(OpVs2, u32, u32)> = lowered
        .unit
        .code
        .iter()
        .map(|ins| (ins.op, ins.a, ins.b))
        .collect();

    // Prefer `start` scene; else earliest entry PC among scenes.
    let start_pc = lowered
        .unit
        .entry_scenes
        .get("start")
        .copied()
        .or_else(|| lowered.unit.entry_scenes.values().copied().min())
        .unwrap_or(0) as usize;

    let mut vm = Vs2MiniVm::new(host);
    vm.load(code);
    vm.pc = start_pc;

    // Seed choice before run so LoadState("__choice") works on first Menu.
    vm.host
        .store_state("__choice", &choice_index.to_string());
    // Also prime local slot  if lower stored choice early — host state is enough.

    let mut steps = 0usize;
    let max = 10_000usize;
    while steps < max && !vm.halted {
        if vm.pc >= vm.code.len() {
            break;
        }
        let (op, a, b) = vm.code[vm.pc];
        if op == OpVs2::Menu {
            vm.host
                .store_state("__choice", &choice_index.to_string());
        }
        if op == OpVs2::JumpScene {
            // resolve entry by pool name
            let name = vm.host.pool_str(a);
            if let Some(&pc) = lowered.unit.entry_scenes.get(&name) {
                vm.pc = if b != 0 { b as usize } else { pc as usize };
                steps += 1;
                continue;
            }
        }
        if op == OpVs2::CallScene {
            let name = vm.host.pool_str(a);
            if let Some(&pc) = lowered.unit.entry_scenes.get(&name) {
                // return to next instruction after CallScene
                vm.call_stack.push(vm.pc + 1);
                vm.pc = if b != 0 { b as usize } else { pc as usize };
                steps += 1;
                continue;
            }
        }
        if op == OpVs2::Choice {
            // only execute body of selected choice: skip others via heuristic
            // bodies are sequential; for demo we run all (story may goto)
            let _ = (a, b);
        }
        if !vm.step() {
            break;
        }
        steps += 1;
    }

    let dialogue: Vec<String> = vm
        .host
        .dialogue
        .iter()
        .map(|d| format!("{}: {}", d.speaker, d.text))
        .collect();
    let log = vm.host.log.clone();
    let state: Vec<(String, String)> = vm.host.state.into_iter().collect();

    RunResult {
        build,
        dialogue,
        log,
        state,
        steps,
        ok: true,
    }
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

/// Dump AST as JSON.
pub fn dump_ast_json(source: &str, file: &str) -> Result<String, String> {
    let p = parse(source, file);
    serde_json::to_string_pretty(&p.file).map_err(|e| e.to_string())
}

/// Dump lowered disasm.
pub fn dump_lowered_text(source: &str, file: &str, cmds: &CommandRegistry) -> Result<String, String> {
    let b = build_source(source, file, cmds);
    if !b.ok {
        let msgs: Vec<_> = b.check.diags.iter().map(|d| d.display()).collect();
        return Err(msgs.join("\n"));
    }
    Ok(dump_lowered(&b.lowered.unwrap().unit))
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
    let check = check_source(source, file, cmds);
    if !check.ok {
        let msgs: Vec<_> = check.diags.iter().map(|d| d.display()).collect();
        return Err(msgs.join("\n"));
    }
    to_story_program(&check.file, title).map_err(|e| e.to_string())
}

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
            StoryWait::Ready => {
                player.advance();
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

/// Check → StoryProgram → product run (writer primary path).
pub fn run_source_product(
    source: &str,
    file: &str,
    cmds: &CommandRegistry,
    choice: usize,
) -> Result<ProgramRunResult, String> {
    let prog = build_story_program(source, file, cmds, file)?;
    Ok(run_story_program(prog, choice, 256))
}

/// Source map from build.
pub fn source_map_of(source: &str, file: &str, cmds: &CommandRegistry) -> Option<SourceMap> {
    let b = build_source(source, file, cmds);
    b.lowered.map(|l| l.map)
}
