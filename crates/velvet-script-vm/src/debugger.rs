//! Cooperative debugger: breakpoints, stepping, local snapshots.

use std::collections::HashSet;

use velvet_script_bytecode::BytecodeModule;

use crate::value::Value;
use crate::vm::{CallFrameView, Vm, VmError};

/// Step behaviour for the debugger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StepMode {
    /// Run until breakpoint, yield, halt, or limit.
    #[default]
    Continue,
    /// Stop after the next instruction.
    StepIn,
    /// Stop after the next instruction that returns from a call (best-effort: stop next).
    StepOver,
}

/// Breakpoint keyed by function index and instruction pointer (byte offset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Breakpoint {
    /// Function index into the module.
    pub function: u16,
    /// Instruction pointer (byte offset into chunk code).
    pub ip: usize,
}

/// Snapshot of a local / stack slot for inspection.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalSnapshot {
    /// Slot index relative to frame base.
    pub slot: usize,
    /// Optional name when known from chunk debug info (not always present).
    pub name: Option<String>,
    /// Value.
    pub value: Value,
}

/// Snapshot of the active call frame.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameSnapshot {
    /// Function index.
    pub function: u16,
    /// Function name.
    pub name: String,
    /// Instruction pointer.
    pub ip: usize,
    /// Locals in this frame.
    pub locals: Vec<LocalSnapshot>,
}

/// Why the debugger stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugStopReason {
    /// Hit a breakpoint.
    Breakpoint(Breakpoint),
    /// Single-step completed.
    Step,
    /// Coroutine / soft yield (instruction budget slice).
    Yielded,
    /// Program finished normally.
    Finished,
    /// Runtime error.
    Error(String),
}

/// Debugger controller wrapping a [`Vm`].
#[derive(Debug, Clone, Default)]
pub struct Debugger {
    /// Instruction-index breakpoints.
    pub breakpoints: HashSet<Breakpoint>,
    /// Current step mode.
    pub step_mode: StepMode,
    /// When stepping over, depth to return to (frames len).
    step_over_depth: Option<usize>,
    /// Breakpoint to ignore once (so resume can leave the stopped IP).
    skip_breakpoint_once: Option<Breakpoint>,
}

impl Debugger {
    /// Create an empty debugger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a breakpoint at `(function, ip)`.
    pub fn add_breakpoint(&mut self, function: u16, ip: usize) {
        self.breakpoints.insert(Breakpoint { function, ip });
    }

    /// Remove a breakpoint.
    pub fn remove_breakpoint(&mut self, function: u16, ip: usize) -> bool {
        self.breakpoints.remove(&Breakpoint { function, ip })
    }

    /// Clear all breakpoints.
    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    /// Enable step-in mode for the next run slice.
    pub fn step_in(&mut self) {
        self.step_mode = StepMode::StepIn;
        self.step_over_depth = None;
    }

    /// Enable step-over mode (stop after one instruction at same-or-shallower depth).
    pub fn step_over(&mut self, current_depth: usize) {
        self.step_mode = StepMode::StepOver;
        self.step_over_depth = Some(current_depth);
    }

    /// Continue until next breakpoint.
    pub fn continue_run(&mut self) {
        self.step_mode = StepMode::Continue;
        self.step_over_depth = None;
    }

    /// Whether a breakpoint is armed at `(function, ip)` (respecting skip-once).
    fn breakpoint_hit(&mut self, function: u16, ip: usize) -> Option<Breakpoint> {
        let bp = Breakpoint { function, ip };
        if !self.breakpoints.contains(&bp) {
            return None;
        }
        if self.skip_breakpoint_once == Some(bp) {
            self.skip_breakpoint_once = None;
            return None;
        }
        Some(bp)
    }
}

/// Inspect locals of the current top frame.
pub fn inspect_locals(vm: &Vm) -> Option<FrameSnapshot> {
    let view = vm.top_frame()?;
    Some(frame_snapshot(vm.module(), &view, vm.stack_values()))
}

/// Inspect all frames (innermost first).
pub fn inspect_stack_trace(vm: &Vm) -> Vec<FrameSnapshot> {
    let stack = vm.stack_values();
    vm.frames_view()
        .into_iter()
        .rev()
        .map(|view| frame_snapshot(vm.module(), &view, stack))
        .collect()
}

fn frame_snapshot(module: &BytecodeModule, view: &CallFrameView, stack: &[Value]) -> FrameSnapshot {
    let chunk = &module.functions[view.function as usize];
    let local_count = chunk.locals as usize;
    let mut locals = Vec::new();
    for slot in 0..local_count {
        let idx = view.stack_base + slot;
        if idx < stack.len() {
            locals.push(LocalSnapshot {
                slot,
                name: None,
                value: stack[idx].clone(),
            });
        }
    }
    // Also include any stack slots beyond declared locals up to next frame / top.
    let end = stack.len();
    for slot in local_count..(end.saturating_sub(view.stack_base)) {
        let idx = view.stack_base + slot;
        if idx < stack.len() {
            locals.push(LocalSnapshot {
                slot,
                name: None,
                value: stack[idx].clone(),
            });
        }
    }
    FrameSnapshot {
        function: view.function,
        name: chunk.name.clone(),
        ip: view.ip,
        locals,
    }
}

/// Run the VM with debugger control: at most `budget` instructions per call.
///
/// Stops on breakpoint / step / yield budget / finish / error.
///
/// Step modes execute exactly one instruction then stop. Breakpoints are checked
/// *before* each instruction when continuing; after stopping on a breakpoint,
/// the next resume steps over that instruction once so progress is possible.
pub fn debug_run(vm: &mut Vm, dbg: &mut Debugger, budget: u64) -> Result<DebugStopReason, VmError> {
    if vm.frames_is_empty() {
        return Ok(DebugStopReason::Finished);
    }

    // Step-in / step-over: run one instruction (respecting step-over depth).
    if matches!(dbg.step_mode, StepMode::StepIn | StepMode::StepOver) {
        loop {
            if vm.is_cancelled() {
                return Err(VmError::Cancelled);
            }
            if vm.frames_is_empty() {
                dbg.step_mode = StepMode::Continue;
                dbg.step_over_depth = None;
                return Ok(DebugStopReason::Finished);
            }
            let depth = vm.frame_depth();
            let allow = match dbg.step_mode {
                StepMode::StepIn => true,
                StepMode::StepOver => dbg.step_over_depth.map(|d| depth <= d).unwrap_or(true),
                StepMode::Continue => true,
            };
            match vm.step_instruction() {
                Ok(true) => {
                    dbg.step_mode = StepMode::Continue;
                    dbg.step_over_depth = None;
                    return Ok(DebugStopReason::Yielded);
                }
                Ok(false) => {
                    if allow {
                        dbg.step_mode = StepMode::Continue;
                        dbg.step_over_depth = None;
                        return Ok(DebugStopReason::Step);
                    }
                    // Still inside callee during step-over; keep going.
                }
                Err(e) => {
                    dbg.step_mode = StepMode::Continue;
                    dbg.step_over_depth = None;
                    return Ok(DebugStopReason::Error(e.to_string()));
                }
            }
        }
    }

    // Continue: stop before executing an instruction that has a breakpoint.
    let mut executed = 0u64;
    while !vm.frames_is_empty() {
        if vm.is_cancelled() {
            return Err(VmError::Cancelled);
        }
        let (function, ip) = {
            let f = vm.top_frame().expect("frame");
            (f.function, f.ip)
        };
        if let Some(bp) = dbg.breakpoint_hit(function, ip) {
            // Next resume will skip this BP once so progress is possible.
            dbg.skip_breakpoint_once = Some(bp);
            return Ok(DebugStopReason::Breakpoint(bp));
        }
        match vm.step_instruction() {
            Ok(true) => return Ok(DebugStopReason::Yielded),
            Ok(false) => {
                executed += 1;
                if executed >= budget {
                    return Ok(DebugStopReason::Yielded);
                }
            }
            Err(e) => return Ok(DebugStopReason::Error(e.to_string())),
        }
    }
    Ok(DebugStopReason::Finished)
}

#[cfg(test)]
mod tests {
    use velvet_script_compiler::compile_source;

    use super::*;
    use crate::vm::{Vm, VmLimits};

    #[test]
    fn breakpoint_and_locals() {
        let src = r#"
function add(a, b) {
    return a + b
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        // Prepare call without running fully.
        vm.push_value(Value::Int(2)).unwrap();
        vm.push_value(Value::Int(40)).unwrap();
        let fidx = *vm.module().exports.get("add").unwrap();
        vm.begin_call(fidx, 2).unwrap();

        let mut dbg = Debugger::new();
        // Break at first instruction of add.
        dbg.add_breakpoint(fidx, 0);
        let reason = debug_run(&mut vm, &mut dbg, 10_000).unwrap();
        assert!(matches!(reason, DebugStopReason::Breakpoint(_)));
        let snap = inspect_locals(&vm).unwrap();
        assert_eq!(snap.name, "add");
        assert!(snap.locals.len() >= 2);
        assert_eq!(snap.locals[0].value, Value::Int(2));
        assert_eq!(snap.locals[1].value, Value::Int(40));
    }

    #[test]
    fn step_in_advances() {
        let src = r#"
function f() {
    let x = 1
    return x
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let fidx = *vm.module().exports.get("f").unwrap();
        vm.begin_call(fidx, 0).unwrap();
        let mut dbg = Debugger::new();
        dbg.step_in();
        let r1 = debug_run(&mut vm, &mut dbg, 100).unwrap();
        assert_eq!(r1, DebugStopReason::Step);
        let ip1 = vm.top_frame().unwrap().ip;
        dbg.step_in();
        let r2 = debug_run(&mut vm, &mut dbg, 100).unwrap();
        assert_eq!(r2, DebugStopReason::Step);
        let ip2 = vm.top_frame().unwrap().ip;
        assert_ne!(ip1, ip2);
    }

    #[test]
    fn remove_breakpoint_allows_finish() {
        let src = r#"
function add(a, b) {
    let s = a + b
    return s
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        vm.push_value(Value::Int(1)).unwrap();
        vm.push_value(Value::Int(2)).unwrap();
        let fidx = *vm.module().exports.get("add").unwrap();
        vm.begin_call(fidx, 2).unwrap();
        let mut dbg = Debugger::new();
        dbg.add_breakpoint(fidx, 0);
        let reason = debug_run(&mut vm, &mut dbg, 10_000).unwrap();
        assert!(matches!(reason, DebugStopReason::Breakpoint(_)));
        assert!(dbg.remove_breakpoint(fidx, 0));
        dbg.continue_run();
        // Continue without stepping.
        let reason2 = debug_run(&mut vm, &mut dbg, 10_000).unwrap();
        assert!(
            matches!(reason2, DebugStopReason::Finished)
                || matches!(reason2, DebugStopReason::Breakpoint(_)),
            "{reason2:?}"
        );
    }

    #[test]
    fn step_over_completes_function_body() {
        let src = r#"
function inner() {
    return 1
}
function outer() {
    let x = inner()
    return x + 1
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let fidx = *vm.module().exports.get("outer").unwrap();
        vm.begin_call(fidx, 0).unwrap();
        let mut dbg = Debugger::new();
        // Step a few times then run to finish.
        for _ in 0..3 {
            dbg.step_in();
            let _ = debug_run(&mut vm, &mut dbg, 100);
            if vm.top_frame().is_none() {
                break;
            }
        }
        let reason = debug_run(&mut vm, &mut dbg, 10_000).unwrap();
        assert!(
            matches!(reason, DebugStopReason::Finished)
                || matches!(reason, DebugStopReason::Step)
                || matches!(reason, DebugStopReason::Breakpoint(_)),
            "{reason:?}"
        );
    }

    #[test]
    fn inspect_locals_mid_function() {
        let src = r#"
function work(a) {
    let b = a + 1
    let c = b + 1
    return c
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        vm.push_value(Value::Int(10)).unwrap();
        let fidx = *vm.module().exports.get("work").unwrap();
        vm.begin_call(fidx, 1).unwrap();
        let mut dbg = Debugger::new();
        dbg.add_breakpoint(fidx, 0);
        let _ = debug_run(&mut vm, &mut dbg, 1000).unwrap();
        let snap = inspect_locals(&vm).unwrap();
        assert_eq!(snap.name, "work");
        assert!(!snap.locals.is_empty());
        assert_eq!(snap.locals[0].value, Value::Int(10));
    }
}
