//! Virtual machine.

use thiserror::Error;
use velvet_script_bytecode::{lookup_native, BytecodeModule, Chunk, NativeId, Op};
use velvet_script_compiler::{compile_source, CompileError};

use crate::stdlib;
use crate::value::Value;

/// Configurable VM limits (sandbox).
#[derive(Debug, Clone)]
pub struct VmLimits {
    /// Max instructions per `run` / frame.
    pub max_instructions: u64,
    /// Approx max value-memory units.
    pub max_memory_units: usize,
    /// Max call stack depth.
    pub max_recursion: usize,
    /// Max value stack depth.
    pub max_stack: usize,
    /// When true, disallow host side-effects beyond Print capture.
    pub sandbox: bool,
}

impl Default for VmLimits {
    fn default() -> Self {
        Self {
            max_instructions: 1_000_000,
            max_memory_units: 1_000_000,
            max_recursion: 256,
            max_stack: 65_536,
            sandbox: true,
        }
    }
}

/// VM errors with optional location and stack trace.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VmError {
    /// Compile failure.
    #[error("compile error: {0}")]
    Compile(String),
    /// Runtime error.
    #[error("{message}")]
    Runtime {
        /// Message.
        message: String,
        /// Location `file:line:col` if known.
        location: Option<String>,
        /// Stack frames.
        stack_trace: Vec<String>,
    },
    /// Instruction budget exhausted.
    #[error(
        "instruction limit exceeded ({limit} per call); raise `max_instructions`, split the work across calls, or find hot spots with `velvet app profile` / `velvet vs3 run --profile`"
    )]
    InstructionLimit {
        /// Limit.
        limit: u64,
        /// Script frames active when the budget ran out (innermost first).
        stack_trace: Vec<String>,
    },
    /// Memory budget.
    #[error("memory limit exceeded ({limit} units)")]
    MemoryLimit {
        /// Limit.
        limit: usize,
    },
    /// Recursion limit.
    #[error("recursion limit exceeded ({limit})")]
    RecursionLimit {
        /// Limit.
        limit: usize,
    },
    /// Cancelled by host.
    #[error("execution cancelled")]
    Cancelled,
}

impl From<CompileError> for VmError {
    fn from(value: CompileError) -> Self {
        Self::Compile(value.to_string())
    }
}

/// Output of a VM run.
#[derive(Debug, Clone, PartialEq)]
pub struct VmOutput {
    /// Return value of main.
    pub value: Value,
    /// Printed lines.
    pub printed: Vec<String>,
    /// Instructions executed.
    pub instructions: u64,
}

#[derive(Debug)]
struct CallFrame {
    function: u16,
    ip: usize,
    stack_base: usize,
}

/// Read-only view of a call frame (for debugger).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallFrameView {
    /// Function index.
    pub function: u16,
    /// Instruction pointer.
    pub ip: usize,
    /// Stack base for locals.
    pub stack_base: usize,
}

/// Per-function profile counters.
#[derive(Debug, Clone, Copy, Default)]
struct FunctionProfile {
    calls: u64,
    instructions: u64,
}

#[derive(Debug, Clone)]
struct Profile {
    functions: Vec<FunctionProfile>,
    natives: std::collections::BTreeMap<u16, FunctionProfile>,
}

/// One row of a profiling report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileEntry {
    /// Function name (`native:<name>` for natives).
    pub name: String,
    /// Times called.
    pub calls: u64,
    /// Instructions executed inside the function itself (self cost),
    /// including the cost charged for natives it called.
    pub instructions: u64,
}

/// Whether a native calls back into script functions (needs the VM).
pub(crate) fn is_callback_native(native: NativeId) -> bool {
    use NativeId as N;
    matches!(
        native,
        N::Call
            | N::ListMap
            | N::ListFilter
            | N::ListFind
            | N::ListAny
            | N::ListAll
            | N::ListCount
            | N::ListReduce
            | N::ListSortBy
    )
}

/// Velvet Script virtual machine.
#[derive(Debug)]
pub struct Vm {
    module: BytecodeModule,
    profile: Option<Profile>,
    globals: Vec<Value>,
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    limits: VmLimits,
    instructions: u64,
    total_instructions: u64,
    memory_units: usize,
    printed: Vec<String>,
    cancelled: bool,
}

impl Vm {
    /// Create VM for a compiled module.
    pub fn new(module: BytecodeModule, limits: VmLimits) -> Self {
        let global_count = module.globals.len();
        // Also need space for function exports bound as globals at runtime.
        let mut globals = vec![Value::Null; global_count.max(16)];
        // Bind exported functions into globals by name if listed.
        for (name, &fidx) in &module.exports {
            if let Some(pos) = module.globals.iter().position(|g| g == name) {
                if pos >= globals.len() {
                    globals.resize(pos + 1, Value::Null);
                }
                globals[pos] = Value::Function(fidx);
            }
        }
        // Bind stdlib natives into globals when the name matches.
        for native in NativeId::all() {
            if let Some(pos) = module.globals.iter().position(|g| g == native.name()) {
                if pos >= globals.len() {
                    globals.resize(pos + 1, Value::Null);
                }
                // Prefer script functions if already bound.
                if matches!(globals[pos], Value::Null) {
                    globals[pos] = Value::Native(native.as_u16());
                }
            }
        }
        Self {
            module,
            profile: None,
            globals,
            stack: Vec::new(),
            frames: Vec::new(),
            limits,
            instructions: 0,
            total_instructions: 0,
            memory_units: 0,
            printed: Vec::new(),
            cancelled: false,
        }
    }

    /// Borrow the module.
    pub fn module(&self) -> &BytecodeModule {
        &self.module
    }

    /// Request cancellation (checked between instructions).
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Whether cancel was requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Drain captured print output.
    pub fn take_printed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.printed)
    }

    /// Borrow printed lines.
    pub fn printed(&self) -> &[String] {
        &self.printed
    }

    /// Turn per-function instruction profiling on or off. Turning it on
    /// clears previous measurements.
    pub fn set_profiling(&mut self, enabled: bool) {
        self.profile = if enabled {
            Some(Profile {
                functions: vec![FunctionProfile::default(); self.module.functions.len()],
                natives: std::collections::BTreeMap::new(),
            })
        } else {
            None
        };
    }

    /// Whether profiling is on.
    pub fn is_profiling(&self) -> bool {
        self.profile.is_some()
    }

    /// Profiling results sorted by self instructions (highest first). Script
    /// functions use their name; natives appear as `native:<name>` and count
    /// their charged cost. Empty when profiling is off.
    pub fn profile_report(&self) -> Vec<ProfileEntry> {
        let Some(profile) = &self.profile else {
            return Vec::new();
        };
        let mut out: Vec<ProfileEntry> = profile
            .functions
            .iter()
            .enumerate()
            .filter(|(_, f)| f.calls > 0 || f.instructions > 0)
            .map(|(index, f)| ProfileEntry {
                name: self.module.functions[index].name.clone(),
                calls: f.calls,
                instructions: f.instructions,
            })
            .collect();
        for (id, f) in &profile.natives {
            let name = NativeId::from_u16(*id).map_or("?", NativeId::name);
            out.push(ProfileEntry {
                name: format!("native:{name}"),
                calls: f.calls,
                instructions: f.instructions,
            });
        }
        out.sort_by(|a, b| {
            b.instructions
                .cmp(&a.instructions)
                .then(a.name.cmp(&b.name))
        });
        out
    }

    /// Instructions executed so far.
    pub fn instructions(&self) -> u64 {
        self.instructions
    }

    /// Instructions executed over the lifetime of this VM.
    pub fn total_instructions(&self) -> u64 {
        self.total_instructions
    }

    /// Value stack (for debugger).
    pub fn stack_values(&self) -> &[Value] {
        &self.stack
    }

    /// Call-frame depth.
    pub fn frame_depth(&self) -> usize {
        self.frames.len()
    }

    /// Whether there are no active frames.
    pub fn frames_is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Top frame view.
    pub fn top_frame(&self) -> Option<CallFrameView> {
        self.frames.last().map(|f| CallFrameView {
            function: f.function,
            ip: f.ip,
            stack_base: f.stack_base,
        })
    }

    /// All frames outer→inner.
    pub fn frames_view(&self) -> Vec<CallFrameView> {
        self.frames
            .iter()
            .map(|f| CallFrameView {
                function: f.function,
                ip: f.ip,
                stack_base: f.stack_base,
            })
            .collect()
    }

    /// Push a value (public for coroutine / debugger setup).
    pub fn push_value(&mut self, v: Value) -> Result<(), VmError> {
        self.push(v)
    }

    /// Pop a value.
    pub fn pop_value(&mut self) -> Option<Value> {
        self.pop().ok()
    }

    /// Peek value from top (`distance` 0 = top).
    pub fn peek_value(&self, distance: usize) -> Option<&Value> {
        self.stack.get(self.stack.len().checked_sub(1 + distance)?)
    }

    /// Begin a function call with `argc` args already on the stack.
    pub fn begin_call(&mut self, fidx: u16, argc: u8) -> Result<(), VmError> {
        self.call_function(fidx, argc)
    }

    /// Run the main `<script>` function.
    pub fn run(&mut self) -> Result<VmOutput, VmError> {
        self.reset_execution_budget();
        let main = self.module.main_index().ok_or_else(|| VmError::Runtime {
            message: "no main chunk".into(),
            location: None,
            stack_trace: vec![],
        })?;
        self.call_function(main, 0)?;
        self.execute()?;
        let value = self.pop().unwrap_or(Value::Null);
        Ok(VmOutput {
            value,
            printed: std::mem::take(&mut self.printed),
            instructions: self.instructions,
        })
    }

    /// Run the synthetic `<script>` initializer once when the module has one.
    ///
    /// Compiled modules use this chunk to initialize top-level and `state`
    /// globals. Hand-built bytecode modules without `<script>` need no setup.
    pub fn initialize(&mut self) -> Result<(), VmError> {
        if !self.frames.is_empty() {
            return Err(self.runtime_err("cannot initialize while code is running"));
        }
        let Some(&main) = self.module.exports.get("<script>") else {
            return Ok(());
        };
        self.reset_execution_budget();
        self.call_function(main, 0)?;
        self.execute()?;
        let _ = self.pop();
        Ok(())
    }

    /// Call a named export with args (pushed left-to-right).
    pub fn call_name(&mut self, name: &str, args: &[Value]) -> Result<Value, VmError> {
        if self.frames.is_empty() {
            self.reset_execution_budget();
        }
        let fidx = *self
            .module
            .exports
            .get(name)
            .ok_or_else(|| VmError::Runtime {
                message: format!("unknown function '{name}'"),
                location: None,
                stack_trace: vec![],
            })?;
        for a in args {
            self.push(a.clone())?;
        }
        self.call_function(fidx, args.len() as u8)?;
        self.execute()?;
        Ok(self.pop().unwrap_or(Value::Null))
    }

    /// Execute a single instruction.
    ///
    /// Returns `Ok(true)` if the instruction was `Yield` (suspended).
    /// Returns `Ok(false)` for normal progress (including Return that empties frames).
    pub fn step_instruction(&mut self) -> Result<bool, VmError> {
        if self.frames.is_empty() {
            return Ok(false);
        }
        if self.cancelled {
            return Err(VmError::Cancelled);
        }
        self.instructions += 1;
        self.total_instructions += 1;
        if let Some(profile) = &mut self.profile {
            if let Some(frame) = self.frames.last() {
                if let Some(f) = profile.functions.get_mut(frame.function as usize) {
                    f.instructions += 1;
                }
            }
        }
        if self.instructions > self.limits.max_instructions {
            return Err(VmError::InstructionLimit {
                limit: self.limits.max_instructions,
                stack_trace: self.stack_trace(),
            });
        }

        let op_byte = self.read_u8()?;
        let op = Op::from_u8(op_byte)
            .ok_or_else(|| self.runtime_err(format!("invalid opcode {op_byte}")))?;

        match op {
            Op::Constant => {
                let idx = self.read_u16()?;
                let frame = self.frame()?;
                let c = self.module.functions[frame.function as usize]
                    .constants
                    .get(idx as usize)
                    .ok_or_else(|| self.runtime_err("bad constant index"))?
                    .clone();
                self.push(Value::from_constant(&c))?;
            }
            Op::Null => self.push(Value::Null)?,
            Op::True => self.push(Value::Bool(true))?,
            Op::False => self.push(Value::Bool(false))?,
            Op::Pop => {
                let _ = self.pop()?;
            }
            Op::Dup => {
                let v = self.peek(0)?.clone();
                self.push(v)?;
            }
            Op::DefineGlobal => {
                let idx = self.read_u16()? as usize;
                let v = self.pop()?;
                self.set_global_value(idx, v);
            }
            Op::GetGlobal => {
                let idx = self.read_u16()? as usize;
                let v = self.globals.get(idx).cloned().unwrap_or(Value::Null);
                // If null, try resolve export by global name or stdlib.
                let v = if matches!(v, Value::Null) {
                    if let Some(name) = self.module.globals.get(idx) {
                        if let Some(&f) = self.module.exports.get(name) {
                            Value::Function(f)
                        } else if let Some(n) = lookup_native(name) {
                            Value::Native(n.as_u16())
                        } else {
                            v
                        }
                    } else {
                        v
                    }
                } else {
                    v
                };
                self.push(v)?;
            }
            Op::SetGlobal => {
                let idx = self.read_u16()? as usize;
                let v = self.peek(0)?.clone();
                self.set_global_value(idx, v);
            }
            Op::GetLocal => {
                let slot = self.read_u8()? as usize;
                let base = self.frame()?.stack_base;
                let v = self
                    .stack
                    .get(base + slot)
                    .cloned()
                    .ok_or_else(|| self.runtime_err("bad local"))?;
                self.push(v)?;
            }
            Op::SetLocal => {
                let slot = self.read_u8()? as usize;
                let base = self.frame()?.stack_base;
                let v = self.peek(0)?.clone();
                if base + slot >= self.stack.len() {
                    return Err(self.runtime_err("bad local set"));
                }
                let old_units = self.stack[base + slot].stack_units();
                let new_units = v.stack_units();
                self.memory_units = self.memory_units.saturating_sub(old_units);
                self.memory_units = self.memory_units.saturating_add(new_units);
                self.ensure_memory_limit()?;
                self.stack[base + slot] = v;
            }
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(binary_num(op, &a, &b).map_err(|m| self.runtime_err(m))?)?;
            }
            Op::Neg => {
                let a = self.pop()?;
                let v = match a {
                    Value::Int(i) => Value::Int(
                        i.checked_neg()
                            .ok_or_else(|| self.runtime_err("integer overflow"))?,
                    ),
                    Value::Float(f) => Value::Float(-f),
                    value => crate::math::negate_math(&value)
                        .ok_or_else(|| self.runtime_err("negate on non-number"))?,
                };
                self.push(v)?;
            }
            Op::Not => {
                let a = self.pop()?;
                self.push(Value::Bool(!a.is_truthy()))?;
            }
            Op::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a == b))?;
            }
            Op::Ne => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a != b))?;
            }
            Op::Lt | Op::Le | Op::Gt | Op::Ge => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(cmp_op(op, &a, &b).map_err(|m| self.runtime_err(m))?)?;
            }
            Op::Jump => {
                let offset = self.read_u16()? as usize;
                self.frame_mut()?.ip += offset;
            }
            Op::JumpIfFalse => {
                let offset = self.read_u16()? as usize;
                if !self.peek(0)?.is_truthy() {
                    self.frame_mut()?.ip += offset;
                }
            }
            Op::JumpIfTrue => {
                let offset = self.read_u16()? as usize;
                if self.peek(0)?.is_truthy() {
                    self.frame_mut()?.ip += offset;
                }
            }
            Op::Loop => {
                let offset = self.read_u16()? as usize;
                self.frame_mut()?.ip -= offset;
            }
            Op::Call => {
                let argc = self.read_u8()?;
                self.dispatch_call(argc)?;
            }
            Op::NativeCall => {
                let id = self.read_u16()?;
                let argc = self.read_u8()?;
                self.invoke_native(id, argc)?;
            }
            Op::Return => {
                let result = self.pop()?;
                let frame = self.frames.pop().unwrap();
                // Trim stack to base (drop locals) and release their budget.
                self.truncate_stack(frame.stack_base);
                self.push(result)?;
            }
            Op::MakeList => {
                let count = self.read_u16()? as usize;
                let mut items = Vec::with_capacity(count);
                for _ in 0..count {
                    items.push(self.pop()?);
                }
                items.reverse();
                self.push(Value::list(items))?;
            }
            Op::MakeMap => {
                let count = self.read_u16()? as usize;
                let mut map = std::collections::HashMap::with_capacity(count);
                // Stack has k0,v0,k1,v1,... top is last value.
                let mut pairs = Vec::with_capacity(count);
                for _ in 0..count {
                    let v = self.pop()?;
                    let k = self.pop()?;
                    pairs.push((k, v));
                }
                pairs.reverse();
                for (k, v) in pairs {
                    let key = match k {
                        Value::String(s) => s.as_ref().to_string(),
                        other => other.to_string(),
                    };
                    map.insert(key, v);
                }
                self.push(Value::map(map))?;
            }
            Op::GetIndex => {
                let index = self.pop()?;
                let container = self.pop()?;
                let v = container
                    .get_index(&index)
                    .map_err(|m| self.runtime_err(m))?;
                self.push(v)?;
            }
            Op::SetIndex => {
                // stack: container, index, value
                let value = self.pop()?;
                let index = self.pop()?;
                let container = self.pop()?;
                let stored = container
                    .set_index(&index, value)
                    .map_err(|m| self.runtime_err(m))?;
                self.push(stored)?;
            }
            Op::UpdateIndex => {
                let arithmetic = Op::from_u8(self.read_u8()?)
                    .filter(|op| matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div))
                    .ok_or_else(|| self.runtime_err("invalid compound index operator"))?;
                let right = self.pop()?;
                let index = self.pop()?;
                let container = self.pop()?;
                let current = container
                    .get_index(&index)
                    .map_err(|message| self.runtime_err(message))?;
                let updated = binary_num(arithmetic, &current, &right)
                    .map_err(|message| self.runtime_err(message))?;
                let stored = container
                    .set_index(&index, updated)
                    .map_err(|message| self.runtime_err(message))?;
                self.push(stored)?;
            }
            Op::Len => {
                let v = self.pop()?;
                let n = v
                    .len()
                    .ok_or_else(|| self.runtime_err("len on non-sized value"))?;
                self.push(Value::Int(n as i64))?;
            }
            Op::Print => {
                let v = self.pop()?;
                self.printed.push(v.to_string());
            }
            Op::Yield => {
                // Leave top value on stack as yield payload; suspend.
                return Ok(true);
            }
            Op::Halt => {
                self.frames.clear();
            }
        }
        Ok(false)
    }

    fn execute(&mut self) -> Result<(), VmError> {
        while !self.frames.is_empty() {
            let yielded = self.step_instruction()?;
            if yielded {
                return Err(self.runtime_err(
                    "yield requires cooperative execution; use Coroutine or Vs3Module::start",
                ));
            }
        }
        Ok(())
    }

    fn dispatch_call(&mut self, argc: u8) -> Result<(), VmError> {
        let callee = self
            .stack
            .get(self.stack.len() - 1 - argc as usize)
            .cloned()
            .ok_or_else(|| self.runtime_err("call underflow"))?;
        match callee {
            Value::Function(fidx) => {
                let args_start = self.stack.len() - argc as usize;
                let callee_idx = args_start - 1;
                let removed = self.stack.remove(callee_idx);
                self.memory_units = self.memory_units.saturating_sub(removed.stack_units());
                self.call_function(fidx, argc)?;
            }
            Value::Native(id) => {
                let args_start = self.stack.len() - argc as usize;
                let callee_idx = args_start - 1;
                let removed = self.stack.remove(callee_idx);
                self.memory_units = self.memory_units.saturating_sub(removed.stack_units());
                self.invoke_native(id, argc)?;
            }
            other => {
                return Err(self.runtime_err(format!("cannot call {other}")));
            }
        }
        Ok(())
    }

    fn invoke_native(&mut self, id: u16, argc: u8) -> Result<(), VmError> {
        let argc = argc as usize;
        if self.stack.len() < argc {
            return Err(self.runtime_err("native call underflow"));
        }
        let start = self.stack.len() - argc;
        let args: Vec<Value> = self.stack[start..].to_vec();
        self.truncate_stack(start);
        let native = NativeId::from_u16(id)
            .ok_or_else(|| self.runtime_err(format!("unknown native id {id}")))?;
        if self.limits.sandbox
            && matches!(
                native,
                NativeId::PresentShow
                    | NativeId::PresentSetBg
                    | NativeId::PresentUiFlag
                    | NativeId::PresentUiFlagGet
                    | NativeId::PresentHide
            )
        {
            return Err(self.runtime_err(format!(
                "native `{}` requires host permissions (sandbox is enabled)",
                native.name()
            )));
        }
        let spec = native.spec();
        let cost = u64::from(spec.base_cost)
            .saturating_add(crate::math::dynamic_cost(native, &args))
            .saturating_sub(1);
        self.charge_instructions(cost)?;
        if let Some(profile) = &mut self.profile {
            let entry = profile.natives.entry(id).or_default();
            entry.calls += 1;
            entry.instructions += cost + 1;
        }
        if is_callback_native(native) {
            let value = self.call_higher(native, &args)?;
            self.push(value)?;
            return Ok(());
        }
        let out = stdlib::call_native(id, &args).map_err(|m| self.runtime_err(m))?;
        if let Some(line) = out.printed {
            self.printed.push(line);
        }
        self.push(out.value)?;
        Ok(())
    }

    /// Call a function value (script function or native) to completion from
    /// inside a native and return its result.
    fn call_value_sync(&mut self, callee: &Value, args: &[Value]) -> Result<Value, VmError> {
        if args.len() > u8::MAX as usize {
            return Err(self.runtime_err("too many arguments in callback"));
        }
        match callee {
            Value::Native(id) => {
                for a in args {
                    self.push(a.clone())?;
                }
                self.invoke_native(*id, args.len() as u8)?;
                self.pop()
            }
            Value::Function(fidx) => {
                let depth = self.frames.len();
                for a in args {
                    self.push(a.clone())?;
                }
                self.call_function(*fidx, args.len() as u8)?;
                while self.frames.len() > depth {
                    if self.step_instruction()? {
                        return Err(self.runtime_err("yield is not allowed inside a callback"));
                    }
                }
                self.pop()
            }
            other => Err(self.runtime_err(format!(
                "expected a function value, got `{}`",
                other.type_name()
            ))),
        }
    }

    /// Natives that call back into script functions.
    fn call_higher(&mut self, native: NativeId, args: &[Value]) -> Result<Value, VmError> {
        use NativeId as N;
        let spec = native.spec();
        if args.len() < spec.min_args as usize || args.len() > spec.max_args as usize {
            return Err(self.runtime_err(format!(
                "{} expects {}..={} arguments, got {}",
                spec.name,
                spec.min_args,
                spec.max_args,
                args.len()
            )));
        }
        if native == N::Call {
            return self.call_value_sync(&args[0], &args[1..]);
        }
        let items = args[0].list_items().map_err(|_| {
            self.runtime_err(format!(
                "{} expects a list, got `{}`",
                spec.name,
                args[0].type_name()
            ))
        })?;
        let f = args[1].clone();
        if !matches!(f, Value::Function(_) | Value::Native(_)) {
            return Err(self.runtime_err(format!(
                "{} expects a function as second argument, got `{}`",
                spec.name,
                f.type_name()
            )));
        }
        self.charge_instructions(items.len() as u64)?;
        match native {
            N::ListMap => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.call_value_sync(&f, &[item])?);
                }
                Ok(Value::list(out))
            }
            N::ListFilter => {
                let mut out = Vec::new();
                for item in items {
                    if self.call_value_sync(&f, std::slice::from_ref(&item))?.is_truthy() {
                        out.push(item);
                    }
                }
                Ok(Value::list(out))
            }
            N::ListFind => {
                for item in items {
                    if self.call_value_sync(&f, std::slice::from_ref(&item))?.is_truthy() {
                        return Ok(item);
                    }
                }
                Ok(Value::Null)
            }
            N::ListAny | N::ListAll => {
                let want_all = native == N::ListAll;
                for item in items {
                    let hit = self.call_value_sync(&f, &[item])?.is_truthy();
                    if want_all && !hit {
                        return Ok(Value::Bool(false));
                    }
                    if !want_all && hit {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(want_all))
            }
            N::ListCount => {
                let mut n = 0i64;
                for item in items {
                    if self.call_value_sync(&f, &[item])?.is_truthy() {
                        n += 1;
                    }
                }
                Ok(Value::Int(n))
            }
            N::ListReduce => {
                let mut acc = args[2].clone();
                for item in items {
                    acc = self.call_value_sync(&f, &[acc, item])?;
                }
                Ok(acc)
            }
            N::ListSortBy => {
                let mut keyed = Vec::with_capacity(items.len());
                for item in items {
                    let key = self.call_value_sync(&f, std::slice::from_ref(&item))?;
                    keyed.push((key, item));
                }
                keyed.sort_by(|a, b| stdlib::compare_values(&a.0, &b.0));
                Ok(Value::list(keyed.into_iter().map(|(_, v)| v).collect()))
            }
            _ => Err(self.runtime_err(format!("native `{}` is not a callback native", spec.name))),
        }
    }

    fn charge_instructions(&mut self, extra: u64) -> Result<(), VmError> {
        self.instructions = self.instructions.saturating_add(extra);
        self.total_instructions = self.total_instructions.saturating_add(extra);
        if self.instructions > self.limits.max_instructions {
            return Err(VmError::InstructionLimit {
                limit: self.limits.max_instructions,
                stack_trace: self.stack_trace(),
            });
        }
        Ok(())
    }

    fn reset_execution_budget(&mut self) {
        self.instructions = 0;
        self.memory_units = self
            .globals
            .iter()
            .map(|value| value.memory_units().saturating_sub(1))
            .sum();
    }

    fn chunk(&self, idx: u16) -> &Chunk {
        &self.module.functions[idx as usize]
    }

    fn push(&mut self, v: Value) -> Result<(), VmError> {
        if self.stack.len() >= self.limits.max_stack {
            return Err(self.runtime_err("stack overflow"));
        }
        self.memory_units = self.memory_units.saturating_add(v.stack_units());
        self.ensure_memory_limit()?;
        self.stack.push(v);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| self.runtime_err("stack underflow"))?;
        self.memory_units = self.memory_units.saturating_sub(value.stack_units());
        Ok(value)
    }

    fn truncate_stack(&mut self, new_len: usize) {
        if new_len >= self.stack.len() {
            return;
        }
        let released: usize = self.stack[new_len..].iter().map(Value::stack_units).sum();
        self.memory_units = self.memory_units.saturating_sub(released);
        self.stack.truncate(new_len);
    }

    fn set_global_value(&mut self, index: usize, value: Value) {
        if index >= self.globals.len() {
            self.globals.resize(index + 1, Value::Null);
        }
        let old_units = self.globals[index].memory_units().saturating_sub(1);
        let new_units = value.memory_units().saturating_sub(1);
        self.memory_units = self.memory_units.saturating_sub(old_units);
        self.memory_units = self.memory_units.saturating_add(new_units);
        self.globals[index] = value;
    }

    fn ensure_memory_limit(&self) -> Result<(), VmError> {
        if self.memory_units > self.limits.max_memory_units {
            return Err(VmError::MemoryLimit {
                limit: self.limits.max_memory_units,
            });
        }
        Ok(())
    }

    fn peek(&self, distance: usize) -> Result<&Value, VmError> {
        self.stack
            .get(self.stack.len() - 1 - distance)
            .ok_or_else(|| self.runtime_err("stack underflow"))
    }

    fn frame_mut(&mut self) -> Result<&mut CallFrame, VmError> {
        if self.frames.is_empty() {
            return Err(self.runtime_err("no active frame"));
        }
        Ok(self.frames.last_mut().unwrap())
    }

    fn frame(&self) -> Result<&CallFrame, VmError> {
        if self.frames.is_empty() {
            return Err(self.runtime_err("no active frame"));
        }
        Ok(self.frames.last().unwrap())
    }

    fn call_function(&mut self, fidx: u16, argc: u8) -> Result<(), VmError> {
        if self.frames.len() >= self.limits.max_recursion {
            return Err(VmError::RecursionLimit {
                limit: self.limits.max_recursion,
            });
        }
        let chunk = self.chunk(fidx);
        if argc != chunk.arity {
            return Err(self.runtime_err(format!(
                "function '{}' expected {} args, got {argc}",
                chunk.name, chunk.arity
            )));
        }
        let stack_base = self.stack.len() - argc as usize;
        // Locals beyond params are pushed by `let` as the stack grows (Lox-style).
        let _ = chunk.locals;
        if let Some(profile) = &mut self.profile {
            if let Some(f) = profile.functions.get_mut(fidx as usize) {
                f.calls += 1;
            }
        }
        self.frames.push(CallFrame {
            function: fidx,
            ip: 0,
            stack_base,
        });
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, VmError> {
        let (fidx, ip) = {
            let frame = self.frame()?;
            (frame.function, frame.ip)
        };
        let b = *self.module.functions[fidx as usize]
            .code
            .get(ip)
            .ok_or_else(|| self.runtime_err("unexpected end of bytecode"))?;
        self.frame_mut()?.ip = ip + 1;
        Ok(b)
    }

    fn read_u16(&mut self) -> Result<u16, VmError> {
        let lo = self.read_u8()? as u16;
        let hi = self.read_u8()? as u16;
        Ok(lo | (hi << 8))
    }

    fn runtime_err(&self, message: impl Into<String>) -> VmError {
        let message = message.into();
        let location = self.current_location();
        let stack_trace = self.stack_trace();
        VmError::Runtime {
            message,
            location,
            stack_trace,
        }
    }

    fn current_location(&self) -> Option<String> {
        let frame = self.frames.last()?;
        let chunk = &self.module.functions[frame.function as usize];
        let (line, col) = chunk.location_at(frame.ip.saturating_sub(1))?;
        let file = self.module.file.as_deref().unwrap_or("<script>");
        Some(format!("{file}:{line}:{col}"))
    }

    fn stack_trace(&self) -> Vec<String> {
        let mut frames = Vec::new();
        for frame in self.frames.iter().rev() {
            let chunk = &self.module.functions[frame.function as usize];
            let loc = chunk
                .location_at(frame.ip.saturating_sub(1))
                .map(|(l, c)| {
                    format!(
                        " at {}:{}:{}",
                        self.module.file.as_deref().unwrap_or("<script>"),
                        l,
                        c
                    )
                })
                .unwrap_or_default();
            frames.push(format!("  in {}{loc}", chunk.name));
        }
        frames
    }
}

fn binary_num(op: Op, a: &Value, b: &Value) -> Result<Value, String> {
    // String concat for Add
    if op == Op::Add {
        if let (Value::String(s), _) = (a, b) {
            return Ok(Value::String(std::rc::Rc::from(format!("{s}{b}"))));
        }
        if let (_, Value::String(s)) = (a, b) {
            return Ok(Value::String(std::rc::Rc::from(format!("{a}{s}"))));
        }
    }
    if let Some(result) = crate::math::binary_math(op, a, b) {
        return result;
    }
    if let (Some(ai), Some(bi)) = (a.as_i64(), b.as_i64()) {
        if !matches!(a, Value::Float(_)) && !matches!(b, Value::Float(_)) {
            let v = match op {
                Op::Add => ai.checked_add(bi),
                Op::Sub => ai.checked_sub(bi),
                Op::Mul => ai.checked_mul(bi),
                Op::Div => ai.checked_div(bi),
                Op::Rem => ai.checked_rem(bi),
                _ => return Err("bad numeric op".into()),
            }
            .ok_or_else(|| {
                if bi == 0 && matches!(op, Op::Div | Op::Rem) {
                    "division by zero".to_string()
                } else {
                    "integer overflow".to_string()
                }
            })?;
            return Ok(Value::Int(v));
        }
    }
    let af = a.as_f64().ok_or_else(|| "expected number".to_string())?;
    let bf = b.as_f64().ok_or_else(|| "expected number".to_string())?;
    let v = match op {
        Op::Add => af + bf,
        Op::Sub => af - bf,
        Op::Mul => af * bf,
        Op::Div => {
            if bf == 0.0 {
                return Err("division by zero".into());
            }
            af / bf
        }
        Op::Rem => {
            if bf == 0.0 {
                return Err("division by zero".into());
            }
            af % bf
        }
        _ => return Err("bad numeric op".into()),
    };
    Ok(Value::Float(v))
}

fn cmp_op(op: Op, a: &Value, b: &Value) -> Result<Value, String> {
    let af = a.as_f64().ok_or_else(|| "expected number".to_string())?;
    let bf = b.as_f64().ok_or_else(|| "expected number".to_string())?;
    let v = match op {
        Op::Lt => af < bf,
        Op::Le => af <= bf,
        Op::Gt => af > bf,
        Op::Ge => af >= bf,
        _ => return Err("bad cmp".into()),
    };
    Ok(Value::Bool(v))
}

/// Compile and run source in one step.
pub fn run_source(source: &str, file: Option<&str>, limits: VmLimits) -> Result<VmOutput, VmError> {
    let compiled = compile_source(source, file)?;
    let mut vm = Vm::new(compiled.module, limits);
    vm.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_arithmetic() {
        let out = run_source(
            r#"
function add(a, b) {
    return a + b
}
let x = add(2, 40)
"#,
            Some("math.vel"),
            VmLimits::default(),
        )
        .unwrap();
        // main returns null; check global via re-call
        assert!(out.instructions > 0);
        let compiled = compile_source(
            r#"
function add(a, b) {
    return a + b
}
"#,
            None,
        )
        .unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        // Bind add and call
        let v = vm
            .call_name("add", &[Value::Int(2), Value::Int(40)])
            .unwrap();
        assert_eq!(v, Value::Int(42));
    }

    #[test]
    fn run_if_while() {
        let src = r#"
function sum_to(n) {
    let i = 0
    let s = 0
    while i < n {
        i += 1
        s += i
    }
    return s
}
"#;
        let compiled = compile_source(src, Some("loop.vel")).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let v = vm.call_name("sum_to", &[Value::Int(5)]).unwrap();
        assert_eq!(v, Value::Int(15));
    }

    #[test]
    fn instruction_limit() {
        let src = r#"
function forever() {
    while true {
        let x = 1
    }
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(
            compiled.module,
            VmLimits {
                max_instructions: 1000,
                ..Default::default()
            },
        );
        let err = vm.call_name("forever", &[]).unwrap_err();
        assert!(matches!(err, VmError::InstructionLimit { .. }));
    }

    #[test]
    fn recursion_limit() {
        let src = r#"
function boom(n) {
    return boom(n + 1)
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(
            compiled.module,
            VmLimits {
                max_recursion: 32,
                max_instructions: 100_000,
                ..Default::default()
            },
        );
        let err = vm.call_name("boom", &[Value::Int(0)]).unwrap_err();
        assert!(matches!(err, VmError::RecursionLimit { .. }));
    }

    #[test]
    fn division_by_zero_has_location() {
        let src = r#"
function bad() {
    return 1 / 0
}
"#;
        let compiled = compile_source(src, Some("err.vel")).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let err = vm.call_name("bad", &[]).unwrap_err();
        match err {
            VmError::Runtime {
                message,
                location,
                stack_trace,
            } => {
                assert!(message.contains("division"));
                let location = location.expect("runtime errors must retain source location");
                assert!(location.contains("err.vel"), "location={location}");
                assert!(
                    stack_trace.iter().any(|frame| frame.contains("bad")),
                    "stack={stack_trace:?}"
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn narrative_prints() {
        let src = r#"
scene intro {
    "Once upon a time"
    hero "Hello"
}
"#;
        let out = run_source(src, Some("story.vel"), VmLimits::default()).unwrap();
        // main defines scene; call it
        let compiled = compile_source(src, Some("story.vel")).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let _ = vm.run().unwrap();
        let _ = vm.call_name("intro", &[]).unwrap();
        let printed = vm.printed();
        assert!(printed.iter().any(|p| p.contains("Once upon")));
        assert!(printed.iter().any(|p| p.contains("hero: Hello")));
        let _ = out;
    }

    #[test]
    fn globals_and_assign() {
        let src = r#"
state {
    trust: int = 0
}
function bump() {
    trust += 2
    return trust
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let _ = vm.run().unwrap();
        let v = vm.call_name("bump", &[]).unwrap();
        assert_eq!(v, Value::Int(2));
    }

    #[test]
    fn stdlib_natives() {
        let src = r#"
function math_demo() {
    return clamp(abs(min(-3, 9)), 0, 2)
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let v = vm.call_name("math_demo", &[]).unwrap();
        assert_eq!(v, Value::Int(2));
    }

    #[test]
    fn list_index_and_len() {
        let src = r#"
function demo() {
    let xs = [10, 20, 30]
    return len(xs) + xs[1]
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let v = vm.call_name("demo", &[]).unwrap();
        assert_eq!(v, Value::Int(23));
    }

    #[test]
    fn print_native() {
        let src = r#"
function talk() {
    print("hello", 1)
    return 0
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let _ = vm.call_name("talk", &[]).unwrap();
        assert!(vm.printed().iter().any(|p| p.contains("hello")));
    }

    #[test]
    fn memory_limit() {
        let limits = VmLimits {
            max_memory_units: 4,
            ..Default::default()
        };
        let mut vm2 = Vm::new(velvet_script_bytecode::BytecodeModule::new(), limits);
        let err = vm2
            .push_value(Value::String(std::rc::Rc::from("1234567890")))
            .and_then(|_| vm2.push_value(Value::String(std::rc::Rc::from("1234567890"))))
            .and_then(|_| vm2.push_value(Value::String(std::rc::Rc::from("1234567890"))));
        assert!(err.is_err(), "expected memory limit error, got {err:?}");
    }

    #[test]
    fn large_shared_list_stack_accounting_is_linear() {
        let src = r#"
function build(n) {
    let xs = []
    let i = 0
    while i < n {
        list_push(xs, {"index": i, "label": concat("item-", str(i))})
        i += 1
    }
    return len(xs)
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(
            compiled.module,
            VmLimits {
                max_memory_units: 100_000,
                max_instructions: 2_000_000,
                ..Default::default()
            },
        );
        let value = vm.call_name("build", &[Value::Int(2_000)]).unwrap();
        assert_eq!(value, Value::Int(2_000));
    }

    #[test]
    fn while_with_early_return() {
        let src = r#"
function first_neg() {
    let i = 0
    let xs = [1, 2, -1, 9]
    while i < len(xs) {
        if xs[i] < 0 {
            return i
        }
        i += 1
    }
    return -1
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let v = vm.call_name("first_neg", &[]).unwrap();
        assert_eq!(v, Value::Int(2));
    }

    #[test]
    fn while_skips_odds() {
        let src = r#"
function sum_even() {
    let i = 0
    let t = 0
    let xs = [1, 2, 3, 4]
    while i < len(xs) {
        if xs[i] % 2 == 0 {
            t += xs[i]
        }
        i += 1
    }
    return t
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let v = vm.call_name("sum_even", &[]).unwrap();
        assert_eq!(v, Value::Int(6));
    }

    #[test]
    fn nested_calls_and_locals() {
        let src = r#"
function add(a, b) { return a + b }
function mul(a, b) { return a * b }
function expr(x) {
    return mul(add(x, 1), add(x, 2))
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        // (3+1)*(3+2)=20
        let v = vm.call_name("expr", &[Value::Int(3)]).unwrap();
        assert_eq!(v, Value::Int(20));
    }

    #[test]
    fn string_concat_runtime() {
        let src = r#"
function greet(name) {
    return concat("hi ", name, "!")
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        let v = vm
            .call_name("greet", &[Value::String(std::rc::Rc::from("Ada"))])
            .unwrap();
        assert_eq!(v.as_str(), Some("hi Ada!"));
    }

    #[test]
    fn bool_logic_short_paths() {
        let src = r#"
function pick(a, b) {
    if a && b {
        return 1
    } else if a || b {
        return 2
    } else {
        return 3
    }
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        assert_eq!(
            vm.call_name("pick", &[Value::Bool(true), Value::Bool(true)])
                .unwrap(),
            Value::Int(1)
        );
        assert_eq!(
            vm.call_name("pick", &[Value::Bool(true), Value::Bool(false)])
                .unwrap(),
            Value::Int(2)
        );
        assert_eq!(
            vm.call_name("pick", &[Value::Bool(false), Value::Bool(false)])
                .unwrap(),
            Value::Int(3)
        );
    }

    #[test]
    fn stack_limit_on_deep_call() {
        let src = r#"
function deep(n) {
    if n <= 0 {
        return 0
    }
    return deep(n - 1) + 1
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(
            compiled.module,
            VmLimits {
                max_recursion: 8,
                max_instructions: 100_000,
                ..Default::default()
            },
        );
        let err = vm.call_name("deep", &[Value::Int(50)]).unwrap_err();
        assert!(
            matches!(err, VmError::RecursionLimit { .. })
                || matches!(err, VmError::InstructionLimit { .. })
                || matches!(err, VmError::Runtime { .. }),
            "{err:?}"
        );
    }

    #[test]
    fn modulo_and_comparisons() {
        let src = r#"
function f(n) {
    return n % 5 == 2
}
"#;
        let compiled = compile_source(src, None).unwrap();
        let mut vm = Vm::new(compiled.module, VmLimits::default());
        assert_eq!(
            vm.call_name("f", &[Value::Int(7)]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            vm.call_name("f", &[Value::Int(8)]).unwrap(),
            Value::Bool(false)
        );
    }
}
