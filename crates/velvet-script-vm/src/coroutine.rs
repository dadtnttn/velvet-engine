//! Cooperative coroutines built on VM instruction slices and `Op::Yield`.

use velvet_script_bytecode::BytecodeModule;
use velvet_script_compiler::compile_source;

use crate::value::Value;
use crate::vm::{Vm, VmError, VmLimits};

/// Handle to a cooperative coroutine.
#[derive(Debug)]
pub struct Coroutine {
    vm: Vm,
    done: bool,
    last_yield: Value,
    /// Soft instruction budget per `resume` when no explicit `Yield` fires.
    pub slice_budget: u64,
}

/// Outcome of resuming a coroutine.
#[derive(Debug, Clone, PartialEq)]
pub enum CoroutineStatus {
    /// Suspended; `value` is the yielded value (or null on soft yield).
    Yielded(Value),
    /// Finished; `value` is the return value.
    Complete(Value),
}

impl Coroutine {
    /// Create a coroutine that will call export `name` with `args` on first resume.
    pub fn from_function(
        module: BytecodeModule,
        name: &str,
        args: &[Value],
        limits: VmLimits,
    ) -> Result<Self, VmError> {
        let fidx = *module.exports.get(name).ok_or_else(|| VmError::Runtime {
            message: format!("unknown function '{name}'"),
            location: None,
            stack_trace: vec![],
        })?;
        let mut vm = Vm::new(module, limits);
        for a in args {
            vm.push_value(a.clone())?;
        }
        vm.begin_call(fidx, args.len() as u8)?;
        Ok(Self {
            vm,
            done: false,
            last_yield: Value::Null,
            slice_budget: 10_000,
        })
    }

    /// Compile source and create a coroutine for a named function.
    pub fn from_source(
        source: &str,
        file: Option<&str>,
        name: &str,
        args: &[Value],
        limits: VmLimits,
    ) -> Result<Self, VmError> {
        let compiled = compile_source(source, file)?;
        Self::from_function(compiled.module, name, args, limits)
    }

    /// Whether the coroutine has completed.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Last yielded value (null if none).
    pub fn last_yield(&self) -> &Value {
        &self.last_yield
    }

    /// Borrow underlying VM (for debugger integration).
    pub fn vm(&self) -> &Vm {
        &self.vm
    }

    /// Mutable VM access.
    pub fn vm_mut(&mut self) -> &mut Vm {
        &mut self.vm
    }

    /// Resume until `Yield`, return, soft budget, error, or cancel.
    pub fn resume(&mut self) -> Result<CoroutineStatus, VmError> {
        if self.done {
            return Ok(CoroutineStatus::Complete(self.last_yield.clone()));
        }
        let mut executed = 0u64;
        while !self.vm.frames_is_empty() {
            if self.vm.is_cancelled() {
                return Err(VmError::Cancelled);
            }
            match self.vm.step_instruction() {
                Ok(true) => {
                    // Op::Yield: top of stack is yield value (left on stack by convention).
                    self.last_yield = self.vm.peek_value(0).cloned().unwrap_or(Value::Null);
                    return Ok(CoroutineStatus::Yielded(self.last_yield.clone()));
                }
                Ok(false) => {
                    executed += 1;
                    if executed >= self.slice_budget {
                        self.last_yield = Value::Null;
                        return Ok(CoroutineStatus::Yielded(Value::Null));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        self.done = true;
        let value = self.vm.pop_value().unwrap_or(Value::Null);
        self.last_yield = value.clone();
        Ok(CoroutineStatus::Complete(value))
    }

    /// Drain printed lines from the underlying VM.
    pub fn take_printed(&mut self) -> Vec<String> {
        self.vm.take_printed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yield_and_resume() {
        // Manual bytecode would need Yield; use soft budget for cooperative slices.
        let src = r#"
function work() {
    let i = 0
    while i < 100 {
        i += 1
    }
    return i
}
"#;
        let mut co = Coroutine::from_source(src, None, "work", &[], VmLimits::default()).unwrap();
        co.slice_budget = 20;
        let mut saw_yield = false;
        let mut final_v = Value::Null;
        for _ in 0..10_000 {
            match co.resume().unwrap() {
                CoroutineStatus::Yielded(_) => {
                    saw_yield = true;
                    assert!(!co.is_done());
                }
                CoroutineStatus::Complete(v) => {
                    final_v = v;
                    break;
                }
            }
        }
        assert!(saw_yield);
        assert!(co.is_done());
        assert_eq!(final_v, Value::Int(100));
    }

    #[test]
    fn yield_opcode_via_chunk() {
        use velvet_script_bytecode::{BytecodeModule, Chunk, Constant, Op};

        let mut chunk = Chunk::new("gen");
        chunk.arity = 0;
        chunk.emit_constant(Constant::Int(1));
        chunk.emit_op(Op::Yield);
        chunk.emit_constant(Constant::Int(2));
        chunk.emit_op(Op::Yield);
        chunk.emit_constant(Constant::Int(3));
        chunk.emit_op(Op::Return);

        let mut module = BytecodeModule::new();
        module.exports.insert("gen".into(), 0);
        module.functions.push(chunk);

        let mut co = Coroutine::from_function(module, "gen", &[], VmLimits::default()).unwrap();
        match co.resume().unwrap() {
            CoroutineStatus::Yielded(v) => assert_eq!(v, Value::Int(1)),
            other => panic!("expected yield, got {other:?}"),
        }
        match co.resume().unwrap() {
            CoroutineStatus::Yielded(v) => assert_eq!(v, Value::Int(2)),
            other => panic!("expected yield, got {other:?}"),
        }
        match co.resume().unwrap() {
            CoroutineStatus::Complete(v) => assert_eq!(v, Value::Int(3)),
            other => panic!("expected complete, got {other:?}"),
        }
        assert!(co.is_done());
    }

    #[test]
    fn coroutine_from_source_completes() {
        let src = r#"
function main() {
    let s = 0
    let i = 0
    while i < 10 {
        s += i
        i += 1
    }
    return s
}
"#;
        let mut co = Coroutine::from_source(src, None, "main", &[], VmLimits::default()).unwrap();
        co.slice_budget = 5;
        let mut yields = 0u32;
        let mut result = Value::Null;
        for _ in 0..10_000 {
            match co.resume().unwrap() {
                CoroutineStatus::Yielded(_) => yields += 1,
                CoroutineStatus::Complete(v) => {
                    result = v;
                    break;
                }
            }
        }
        assert!(yields >= 1, "expected cooperative yields");
        assert_eq!(result, Value::Int(45));
        assert!(co.is_done());
        // Further resume should stay complete.
        match co.resume().unwrap() {
            CoroutineStatus::Complete(v) => assert_eq!(v, Value::Int(45)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn coroutine_with_args() {
        let src = r#"
function add(a, b) {
    return a + b
}
"#;
        let mut co = Coroutine::from_source(
            src,
            None,
            "add",
            &[Value::Int(20), Value::Int(22)],
            VmLimits::default(),
        )
        .unwrap();
        match co.resume().unwrap() {
            CoroutineStatus::Complete(v) => assert_eq!(v, Value::Int(42)),
            CoroutineStatus::Yielded(_) => {
                // finish
                loop {
                    match co.resume().unwrap() {
                        CoroutineStatus::Complete(v) => {
                            assert_eq!(v, Value::Int(42));
                            break;
                        }
                        CoroutineStatus::Yielded(_) => {}
                    }
                }
            }
        }
    }

    #[test]
    fn yield_then_print_capture() {
        use velvet_script_bytecode::{BytecodeModule, Chunk, Constant, Op};

        let mut chunk = Chunk::new("g");
        chunk.arity = 0;
        chunk.emit_constant(Constant::Int(7));
        chunk.emit_op(Op::Yield);
        chunk.emit_constant(Constant::Int(8));
        chunk.emit_op(Op::Return);
        let mut module = BytecodeModule::new();
        module.exports.insert("g".into(), 0);
        module.functions.push(chunk);
        let mut co = Coroutine::from_function(module, "g", &[], VmLimits::default()).unwrap();
        assert!(!co.is_done());
        match co.resume().unwrap() {
            CoroutineStatus::Yielded(v) => assert_eq!(v, Value::Int(7)),
            other => panic!("{other:?}"),
        }
        assert!(!co.is_done());
        match co.resume().unwrap() {
            CoroutineStatus::Complete(v) => assert_eq!(v, Value::Int(8)),
            other => panic!("{other:?}"),
        }
        assert!(co.is_done());
    }
}
