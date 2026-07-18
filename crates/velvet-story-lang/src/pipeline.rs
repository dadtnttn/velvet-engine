//! Full pipeline: parse → sema → lower → VS2 unit → host execution.

use std::path::Path;

use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_vm::{Vs2Host, Vs2MiniVm};

use crate::commands::CommandRegistry;
use crate::diag::StoryDiag;
use crate::format::{format_source, is_idempotent};
use crate::lower::{dump_lowered, lower, LowerOutput};
use crate::parser::{parse, ParseResult};
use crate::sema::{self, SemaResult};
use crate::source_map::SourceMap;
use crate::studio::StudioModel;

/// Combined check result.
#[derive(Debug)]
pub struct CheckResult {
    /// Parse.
    pub parsed: ParseResult,
    /// Sema.
    pub sema: SemaResult,
    /// All diags.
    pub diags: Vec<StoryDiag>,
    /// Ok if no errors.
    pub ok: bool,
}

/// Check a story file (no execute).
pub fn check_source(source: &str, file: &str, cmds: &CommandRegistry) -> CheckResult {
    let parsed = parse(source, file);
    let mut diags = parsed.diags.clone();
    let sema = sema::analyze(&parsed.file, cmds);
    diags.extend(sema.diags.clone());
    let ok = !diags.iter().any(|d| d.is_error());
    CheckResult {
        parsed,
        sema,
        diags,
        ok,
    }
}

/// Check path.
pub fn check_path(path: &Path, cmds: &CommandRegistry) -> Result<CheckResult, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let file = path.to_string_lossy().to_string();
    Ok(check_source(&source, &file, cmds))
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

/// Build: check + lower.
pub fn build_source(source: &str, file: &str, cmds: &CommandRegistry) -> BuildResult {
    let check = check_source(source, file, cmds);
    if !check.ok {
        return BuildResult {
            check,
            lowered: None,
            ok: false,
        };
    }
    let lowered = lower(&check.parsed.file);
    let mut check = check;
    check.diags.extend(lowered.diags.clone());
    let ok = !check.diags.iter().any(|d| d.is_error());
    BuildResult {
        check,
        lowered: Some(lowered),
        ok,
    }
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

    // start at first scene entry
    let start_pc = lowered
        .unit
        .entry_scenes
        .values()
        .copied()
        .min()
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
                // patch-like: jump to scene entry
                vm.pc += 1;
                // manual jump
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

/// Format path in place.
pub fn format_path(path: &Path, check_only: bool) -> Result<String, String> {
    let source = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let pretty = format_source(&source);
    if !is_idempotent(&source) && format_source(&pretty) != pretty {
        return Err("formatter not idempotent".into());
    }
    if check_only {
        if pretty != source && pretty != format_source(&source) {
            // compare normalized
            if pretty.trim() != source.trim() {
                return Err("needs formatting".into());
            }
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

/// Source map from build.
pub fn source_map_of(source: &str, file: &str, cmds: &CommandRegistry) -> Option<SourceMap> {
    let b = build_source(source, file, cmds);
    b.lowered.map(|l| l.map)
}
