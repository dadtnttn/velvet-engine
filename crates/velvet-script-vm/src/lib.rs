//! # velvet-script-vm
//!
//! Stack-based VM with instruction, memory, and recursion limits,
//! host stdlib natives, cooperative coroutines, and a simple debugger.

#![deny(missing_docs)]

mod coroutine;
mod debugger;
mod stdlib;
mod value;
mod vm;

pub use coroutine::{Coroutine, CoroutineStatus};
pub use debugger::{
    debug_run, inspect_locals, inspect_stack_trace, Breakpoint, DebugStopReason, Debugger,
    FrameSnapshot, LocalSnapshot, StepMode,
};
pub use stdlib::{call_native, NativeOutput};
pub use value::Value;
pub use velvet_script_bytecode::{lookup_native, NativeId};
pub use vm::{run_source, CallFrameView, Vm, VmError, VmLimits, VmOutput};
