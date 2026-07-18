//! VS2 real codegen: HIR modules → OpVs2 instruction streams.
//!
//! Deterministic lower for story + logic so the VM / story host can execute
//! typed handles without Python eval. Matches `velvet-script-hir` shapes.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_hir::{
    HirBinOp, HirExpr, HirItem, HirLit, HirModule, HirPath, HirStmt, HirTy, PrimTy,
};

/// One encoded instruction (opcode + up to 2 immediate operands).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs2Instr {
    pub op: OpVs2,
    pub a: u32,
    pub b: u32,
    pub line: u32,
}

impl Vs2Instr {
    pub fn new(op: OpVs2) -> Self {
        Self {
            op,
            a: 0,
            b: 0,
            line: 0,
        }
    }
    pub fn with_a(op: OpVs2, a: u32) -> Self {
        Self {
            op,
            a,
            b: 0,
            line: 0,
        }
    }
    pub fn with_ab(op: OpVs2, a: u32, b: u32) -> Self {
        Self {
            op,
            a,
            b,
            line: 0,
        }
    }
    pub fn at_line(mut self, line: u32) -> Self {
        self.line = line;
        self
    }
    pub fn encode(&self) -> [u32; 4] {
        [self.op as u16 as u32, self.a, self.b, self.line]
    }
    pub fn disasm(&self) -> String {
        format!(
            "L{:>4}  {:<16} a={} b={}",
            self.line,
            self.op.name(),
            self.a,
            self.b
        )
    }
}

/// String / scene / layer constant pool for a compiled unit.
#[derive(Debug, Clone, Default)]
pub struct Vs2Pool {
    pub strings: Vec<String>,
    index: HashMap<String, u32>,
}

impl Vs2Pool {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn intern(&mut self, s: impl Into<String>) -> u32 {
        let s = s.into();
        if let Some(&i) = self.index.get(&s) {
            return i;
        }
        let i = self.strings.len() as u32;
        self.index.insert(s.clone(), i);
        self.strings.push(s);
        i
    }
    pub fn get(&self, i: u32) -> Option<&str> {
        self.strings.get(i as usize).map(|s| s.as_str())
    }
    pub fn len(&self) -> usize {
        self.strings.len()
    }
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

/// Compiled VS2 unit (one .vel module after lower).
#[derive(Debug, Clone, Default)]
pub struct Vs2Unit {
    pub module_name: String,
    pub pool: Vs2Pool,
    pub code: Vec<Vs2Instr>,
    pub entry_scenes: HashMap<String, u32>,
    pub fn_entries: HashMap<String, u32>,
    pub local_slots: u32,
    pub diagnostics: Vec<String>,
}

impl Vs2Unit {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            pool: Vs2Pool::new(),
            code: Vec::new(),
            entry_scenes: HashMap::new(),
            fn_entries: HashMap::new(),
            local_slots: 0,
            diagnostics: Vec::new(),
        }
    }
    pub fn emit(&mut self, instr: Vs2Instr) -> u32 {
        let pc = self.code.len() as u32;
        self.code.push(instr);
        pc
    }
    pub fn patch_a(&mut self, pc: u32, a: u32) {
        if let Some(i) = self.code.get_mut(pc as usize) {
            i.a = a;
        }
    }
    pub fn pc(&self) -> u32 {
        self.code.len() as u32
    }
    pub fn disasm(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("; unit {}\n", self.module_name));
        out.push_str(&format!("; pool {} strings\n", self.pool.len()));
        for (i, ins) in self.code.iter().enumerate() {
            out.push_str(&format!("{:04}  {}\n", i, ins.disasm()));
        }
        out
    }
    pub fn encode_blob(&self) -> Vec<u32> {
        let mut v = Vec::with_capacity(self.code.len() * 4 + 4);
        v.push(0x5653_3201); // magic VS2\x01
        v.push(self.code.len() as u32);
        v.push(self.pool.len() as u32);
        v.push(self.local_slots);
        for ins in &self.code {
            v.extend_from_slice(&ins.encode());
        }
        v
    }
}

/// Lowering context (locals + labels).
#[derive(Debug, Default)]
pub struct LowerCtx {
    pub locals: HashMap<String, u32>,
    pub next_local: u32,
    pub labels: HashMap<String, u32>,
    pub pending_jumps: Vec<(u32, String)>,
}

impl LowerCtx {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn local(&mut self, name: &str) -> u32 {
        if let Some(&i) = self.locals.get(name) {
            return i;
        }
        let i = self.next_local;
        self.next_local += 1;
        self.locals.insert(name.to_string(), i);
        i
    }
    pub fn bind_label(&mut self, name: &str, pc: u32) {
        self.labels.insert(name.to_string(), pc);
    }
}

/// Codegen entry: lower a HIR module into a Vs2Unit.
pub fn lower_module(m: &HirModule) -> Vs2Unit {
    let name = m
        .file
        .clone()
        .unwrap_or_else(|| format!("mod_e{}", m.edition));
    let mut unit = Vs2Unit::new(name);
    let mut ctx = LowerCtx::new();
    for item in &m.items {
        match item {
            HirItem::Fn(f) => lower_fn(&mut unit, &mut ctx, f),
            HirItem::Scene(sc) => lower_scene(&mut unit, &mut ctx, sc),
            HirItem::Struct(_)
            | HirItem::Enum(_)
            | HirItem::Character(_)
            | HirItem::State { .. }
            | HirItem::Screen(_)
            | HirItem::Mod { .. }
            | HirItem::Use { .. } => {}
        }
    }
    let pending = ctx.pending_jumps.clone();
    for (pc, lab) in pending {
        if let Some(&target) = ctx.labels.get(&lab) {
            unit.patch_a(pc, target);
        } else {
            unit.diagnostics
                .push(format!("unresolved label '{lab}'"));
        }
    }
    unit.local_slots = ctx.next_local;
    unit
}

fn lower_fn(unit: &mut Vs2Unit, ctx: &mut LowerCtx, f: &velvet_script_hir::HirFn) {
    let entry = unit.pc();
    unit.fn_entries.insert(f.name.clone(), entry);
    for (name, _) in &f.params {
        let _ = ctx.local(name);
    }
    lower_expr(unit, ctx, &f.body);
    unit.emit(Vs2Instr::new(OpVs2::Ret));
}

fn lower_scene(unit: &mut Vs2Unit, ctx: &mut LowerCtx, sc: &velvet_script_hir::HirScene) {
    let entry = unit.pc();
    unit.entry_scenes.insert(sc.name.clone(), entry);
    ctx.bind_label(&sc.name, entry);
    for st in &sc.body {
        lower_stmt(unit, ctx, st);
    }
    unit.emit(Vs2Instr::new(OpVs2::Ret));
}

fn lower_stmt(unit: &mut Vs2Unit, ctx: &mut LowerCtx, st: &HirStmt) {
    match st {
        HirStmt::Expr { expr, .. } => {
            lower_expr(unit, ctx, expr);
            unit.emit(Vs2Instr::new(OpVs2::Pop));
        }
        HirStmt::Let { name, init, .. } => {
            if let Some(v) = init {
                lower_expr(unit, ctx, v);
            } else {
                let empty = unit.pool.intern("");
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, empty));
            }
            let slot = ctx.local(name);
            unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot));
        }
        HirStmt::Assign { target, value, .. } => {
            lower_expr(unit, ctx, value);
            let name = target.display();
            let slot = ctx.local(&name);
            unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot));
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                lower_expr(unit, ctx, v);
            }
            unit.emit(Vs2Instr::new(OpVs2::Ret));
        }
        HirStmt::Say { speaker, msg, .. } => {
            let sp = unit
                .pool
                .intern(speaker.as_deref().unwrap_or("narrator"));
            lower_expr(unit, ctx, msg);
            unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
        }
        HirStmt::Jump { target, .. } => {
            let id = unit.pool.intern(target.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::JumpScene, id));
        }
        HirStmt::CallScene { target, .. } => {
            let id = unit.pool.intern(target.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::CallScene, id));
        }
        HirStmt::Show {
            character, at, ..
        } => {
            let id = unit.pool.intern(character.as_str());
            let at_id = at
                .as_ref()
                .map(|a| unit.pool.intern(a.as_str()))
                .unwrap_or(0);
            unit.emit(Vs2Instr::with_ab(OpVs2::ShowChar, id, at_id));
        }
        HirStmt::Hide { character, .. } => {
            let id = unit.pool.intern(character.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::HideChar, id));
        }
        HirStmt::Background { path, .. } => {
            let id = unit.pool.intern(path.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::Background, id));
        }
        HirStmt::Music { path, .. } => {
            let id = unit.pool.intern(path.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::Music, id));
        }
        HirStmt::PushLayer { id, .. } => {
            let lid = unit.pool.intern(id.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, lid));
        }
        HirStmt::PopLayer { .. } => {
            unit.emit(Vs2Instr::new(OpVs2::PopLayer));
        }
        HirStmt::ShowLayer { id, .. } => {
            let lid = unit.pool.intern(id.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, lid));
        }
        HirStmt::HideLayer { id, .. } => {
            let lid = unit.pool.intern(id.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::HideLayer, lid));
        }
    }
}

fn lower_expr(unit: &mut Vs2Unit, ctx: &mut LowerCtx, e: &HirExpr) {
    match e {
        HirExpr::Lit { lit, .. } => match lit {
            HirLit::Int(n) => {
                if *n >= 0 && *n <= i32::MAX as i64 {
                    unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, *n as u32));
                } else {
                    let id = unit.pool.intern(n.to_string());
                    unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
                }
            }
            HirLit::Float(f) => {
                let id = unit.pool.intern(f.to_string());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
            }
            HirLit::Bool(b) => {
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, if *b { 1 } else { 0 }));
            }
            HirLit::Str(s) => {
                let id = unit.pool.intern(s.as_str());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
            }
            HirLit::MsgId(s) => {
                let id = unit.pool.intern(s.as_str());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, id));
            }
        },
        HirExpr::Path { path, .. } => {
            let name = path.display();
            let slot = ctx.local(&name);
            unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
        }
        HirExpr::Binary { op, lhs, rhs, .. } => {
            lower_expr(unit, ctx, lhs);
            lower_expr(unit, ctx, rhs);
            let opc = match op {
                HirBinOp::Add => OpVs2::Add,
                HirBinOp::Sub => OpVs2::Sub,
                HirBinOp::Mul => OpVs2::Mul,
                HirBinOp::Div => OpVs2::Div,
                HirBinOp::Rem => OpVs2::Rem,
                HirBinOp::Eq => OpVs2::Eq,
                HirBinOp::Ne => OpVs2::Ne,
                HirBinOp::Lt => OpVs2::Lt,
                HirBinOp::Le => OpVs2::Le,
                HirBinOp::Gt => OpVs2::Gt,
                HirBinOp::Ge => OpVs2::Ge,
                HirBinOp::And => OpVs2::And,
                HirBinOp::Or => OpVs2::Or,
            };
            unit.emit(Vs2Instr::new(opc));
        }
        HirExpr::Call { callee, args, .. } => {
            for a in args {
                lower_expr(unit, ctx, a);
            }
            let name = match callee.as_ref() {
                HirExpr::Path { path, .. } => path.display(),
                _ => "anon".into(),
            };
            let id = unit.pool.intern(name);
            unit.emit(Vs2Instr::with_ab(OpVs2::Call, id, args.len() as u32));
        }
        HirExpr::Translate { key, .. } => {
            let id = unit.pool.intern(key.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::Translate, id));
        }
        HirExpr::Layer { id, .. } => {
            let lid = unit.pool.intern(id.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, lid));
        }
        HirExpr::Field { base, .. } => {
            lower_expr(unit, ctx, base);
            // field access: leave base; host may refine later
        }
        HirExpr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            lower_expr(unit, ctx, cond);
            let j_else = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0));
            lower_expr(unit, ctx, then_br);
            let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0));
            let else_pc = unit.pc();
            unit.patch_a(j_else, else_pc);
            if let Some(eb) = else_br {
                lower_expr(unit, ctx, eb);
            } else {
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, 0));
            }
            let end = unit.pc();
            unit.patch_a(j_end, end);
        }
        HirExpr::Block { stmts, tail, .. } => {
            for s in stmts {
                lower_stmt(unit, ctx, s);
            }
            if let Some(t) = tail {
                lower_expr(unit, ctx, t);
            } else {
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, 0));
            }
        }
    }
}

/// Estimate stack depth (rough).
pub fn estimate_stack_depth(unit: &Vs2Unit) -> i32 {
    let mut depth = 0i32;
    let mut max_d = 0i32;
    for ins in &unit.code {
        match ins.op {
            OpVs2::LoadConst | OpVs2::LoadLocal | OpVs2::LoadMsg | OpVs2::Dup => {
                depth += 1;
            }
            OpVs2::Pop | OpVs2::StoreLocal | OpVs2::StoreState | OpVs2::Ret => {
                depth = (depth - 1).max(0);
            }
            OpVs2::Add
            | OpVs2::Sub
            | OpVs2::Mul
            | OpVs2::Div
            | OpVs2::Rem
            | OpVs2::Eq
            | OpVs2::Ne
            | OpVs2::Lt
            | OpVs2::Le
            | OpVs2::Gt
            | OpVs2::Ge
            | OpVs2::And
            | OpVs2::Or => {
                depth = (depth - 1).max(0);
            }
            OpVs2::Not => {}
            _ => {}
        }
        max_d = max_d.max(depth);
    }
    max_d
}

/// Validate unit basic invariants.
pub fn validate_unit(unit: &Vs2Unit) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    if unit.module_name.is_empty() {
        errs.push("empty module name".into());
    }
    for (i, ins) in unit.code.iter().enumerate() {
        if matches!(ins.op, OpVs2::Jump | OpVs2::JumpIf) && ins.a as usize > unit.code.len() {
            errs.push(format!("pc {i}: jump target out of range {}", ins.a));
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// Type name helper for debug dumps.
pub fn ty_tag(t: &HirTy) -> &'static str {
    match t {
        HirTy::Prim(PrimTy::I32) => "i32",
        HirTy::Prim(PrimTy::I64) => "i64",
        HirTy::Prim(PrimTy::U32) => "u32",
        HirTy::Prim(PrimTy::U64) => "u64",
        HirTy::Prim(PrimTy::F32) => "f32",
        HirTy::Prim(PrimTy::F64) => "f64",
        HirTy::Prim(PrimTy::Bool) => "bool",
        HirTy::Prim(PrimTy::Str) => "str",
        HirTy::Prim(PrimTy::Unit) => "()",
        HirTy::LayerId => "LayerId",
        HirTy::SceneId => "SceneId",
        HirTy::MsgId => "MsgId",
        HirTy::Option(_) => "Option",
        HirTy::Result(_, _) => "Result",
        HirTy::Array(_) => "Array",
        HirTy::Tuple(_) => "Tuple",
        HirTy::Fn(_, _) => "Fn",
        HirTy::Path(_) => "Path",
        HirTy::Infer => "_",
        HirTy::ImageHandle => "ImageHandle",
        HirTy::AudioHandle => "AudioHandle",
        HirTy::EntityId => "EntityId",
        HirTy::Transform => "Transform",
        HirTy::Transition => "Transition",
        HirTy::Action => "Action",
    }
}

/// Path display helper for tests.
pub fn path_leaf(p: &HirPath) -> String {
    p.display()
}
/// Emit `nop` helper.
pub fn emit_nop(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Nop, a, b))
}

/// Emit `load_const` helper.
pub fn emit_load_const(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::LoadConst, a, b))
}

/// Emit `load_local` helper.
pub fn emit_load_local(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::LoadLocal, a, b))
}

/// Emit `store_local` helper.
pub fn emit_store_local(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::StoreLocal, a, b))
}

/// Emit `add` helper.
pub fn emit_add(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Add, a, b))
}

/// Emit `sub` helper.
pub fn emit_sub(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Sub, a, b))
}

/// Emit `mul` helper.
pub fn emit_mul(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Mul, a, b))
}

/// Emit `div` helper.
pub fn emit_div(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Div, a, b))
}

/// Emit `rem` helper.
pub fn emit_rem(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Rem, a, b))
}

/// Emit `eq` helper.
pub fn emit_eq(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Eq, a, b))
}

/// Emit `ne` helper.
pub fn emit_ne(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Ne, a, b))
}

/// Emit `lt` helper.
pub fn emit_lt(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Lt, a, b))
}

/// Emit `le` helper.
pub fn emit_le(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Le, a, b))
}

/// Emit `gt` helper.
pub fn emit_gt(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Gt, a, b))
}

/// Emit `ge` helper.
pub fn emit_ge(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Ge, a, b))
}

/// Emit `and` helper.
pub fn emit_and(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::And, a, b))
}

/// Emit `or` helper.
pub fn emit_or(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Or, a, b))
}

/// Emit `not` helper.
pub fn emit_not(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Not, a, b))
}

/// Emit `jump` helper.
pub fn emit_jump(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Jump, a, b))
}

/// Emit `jump_if` helper.
pub fn emit_jump_if(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::JumpIf, a, b))
}

/// Emit `call` helper.
pub fn emit_call(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Call, a, b))
}

/// Emit `ret` helper.
pub fn emit_ret(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Ret, a, b))
}

/// Emit `print` helper.
pub fn emit_print(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Print, a, b))
}

/// Emit `pop` helper.
pub fn emit_pop(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Pop, a, b))
}

/// Emit `dup` helper.
pub fn emit_dup(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Dup, a, b))
}

/// Emit `say` helper.
pub fn emit_say(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Say, a, b))
}

/// Emit `menu` helper.
pub fn emit_menu(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Menu, a, b))
}

/// Emit `choice` helper.
pub fn emit_choice(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Choice, a, b))
}

/// Emit `jump_scene` helper.
pub fn emit_jump_scene(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::JumpScene, a, b))
}

/// Emit `call_scene` helper.
pub fn emit_call_scene(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::CallScene, a, b))
}

/// Emit `show_char` helper.
pub fn emit_show_char(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::ShowChar, a, b))
}

/// Emit `hide_char` helper.
pub fn emit_hide_char(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::HideChar, a, b))
}

/// Emit `background` helper.
pub fn emit_background(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Background, a, b))
}

/// Emit `music` helper.
pub fn emit_music(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Music, a, b))
}

/// Emit `push_layer` helper.
pub fn emit_push_layer(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::PushLayer, a, b))
}

/// Emit `pop_layer` helper.
pub fn emit_pop_layer(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::PopLayer, a, b))
}

/// Emit `show_layer` helper.
pub fn emit_show_layer(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::ShowLayer, a, b))
}

/// Emit `hide_layer` helper.
pub fn emit_hide_layer(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::HideLayer, a, b))
}

/// Emit `set_layer_z` helper.
pub fn emit_set_layer_z(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::SetLayerZ, a, b))
}

/// Emit `translate` helper.
pub fn emit_translate(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Translate, a, b))
}

/// Emit `await_op` helper.
pub fn emit_await_op(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Await, a, b))
}

/// Emit `yield_op` helper.
pub fn emit_yield_op(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::Yield, a, b))
}

/// Emit `load_msg` helper.
pub fn emit_load_msg(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::LoadMsg, a, b))
}

/// Emit `store_state` helper.
pub fn emit_store_state(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::StoreState, a, b))
}

/// Emit `load_state` helper.
pub fn emit_load_state(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::LoadState, a, b))
}

/// Emit `make_array` helper.
pub fn emit_make_array(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {
    unit.emit(Vs2Instr::with_ab(OpVs2::MakeArray, a, b))
}


/// Peephole: remove Nop chains.
pub fn peephole_remove_nops(unit: &mut Vs2Unit) {
    unit.code.retain(|i| i.op != OpVs2::Nop);
}

/// Peephole: LoadConst+Pop → drop.
pub fn peephole_const_pop(unit: &mut Vs2Unit) {
    let mut out = Vec::with_capacity(unit.code.len());
    let mut i = 0;
    while i < unit.code.len() {
        if i + 1 < unit.code.len()
            && unit.code[i].op == OpVs2::LoadConst
            && unit.code[i + 1].op == OpVs2::Pop
        {
            i += 2;
            continue;
        }
        out.push(unit.code[i].clone());
        i += 1;
    }
    unit.code = out;
}

/// Peephole: Dup+Pop → drop.
pub fn peephole_dup_pop(unit: &mut Vs2Unit) {
    let mut out = Vec::with_capacity(unit.code.len());
    let mut i = 0;
    while i < unit.code.len() {
        if i + 1 < unit.code.len()
            && unit.code[i].op == OpVs2::Dup
            && unit.code[i + 1].op == OpVs2::Pop
        {
            i += 2;
            continue;
        }
        out.push(unit.code[i].clone());
        i += 1;
    }
    unit.code = out;
}

/// Run all cheap peepholes.
pub fn optimize_unit(unit: &mut Vs2Unit) {
    peephole_const_pop(unit);
    peephole_dup_pop(unit);
    peephole_remove_nops(unit);
}

/// Link scene names in JumpScene/CallScene to entry PCs when known.
pub fn link_scenes(unit: &mut Vs2Unit) {
    for ins in &mut unit.code {
        if matches!(ins.op, OpVs2::JumpScene | OpVs2::CallScene) {
            if let Some(name) = unit.pool.get(ins.a).map(|s| s.to_string()) {
                if let Some(&pc) = unit.entry_scenes.get(&name) {
                    ins.b = pc;
                }
            }
        }
    }
}

/// Count opcodes of a kind.
pub fn count_op(unit: &Vs2Unit, op: OpVs2) -> usize {
    unit.code.iter().filter(|i| i.op == op).count()
}

/// Story density: ratio of story ops to total.
pub fn story_density(unit: &Vs2Unit) -> f64 {
    if unit.code.is_empty() {
        return 0.0;
    }
    let story = [
        OpVs2::Say,
        OpVs2::Menu,
        OpVs2::Choice,
        OpVs2::JumpScene,
        OpVs2::CallScene,
        OpVs2::ShowChar,
        OpVs2::HideChar,
        OpVs2::Background,
        OpVs2::Music,
        OpVs2::PushLayer,
        OpVs2::PopLayer,
        OpVs2::ShowLayer,
        OpVs2::HideLayer,
        OpVs2::Translate,
        OpVs2::LoadMsg,
    ];
    let n = unit.code.iter().filter(|i| story.contains(&i.op)).count();
    n as f64 / unit.code.len() as f64
}
/// Pattern helper #0: emit load local + op.
pub fn pattern_load_op_0(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #0: intern + say.
pub fn pattern_say_0(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #0: push/show layer pair.
pub fn pattern_layer_0(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #1: emit load local + op.
pub fn pattern_load_op_1(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #1: intern + say.
pub fn pattern_say_1(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #1: push/show layer pair.
pub fn pattern_layer_1(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #2: emit load local + op.
pub fn pattern_load_op_2(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #2: intern + say.
pub fn pattern_say_2(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #2: push/show layer pair.
pub fn pattern_layer_2(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #3: emit load local + op.
pub fn pattern_load_op_3(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #3: intern + say.
pub fn pattern_say_3(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #3: push/show layer pair.
pub fn pattern_layer_3(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #4: emit load local + op.
pub fn pattern_load_op_4(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #4: intern + say.
pub fn pattern_say_4(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #4: push/show layer pair.
pub fn pattern_layer_4(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #5: emit load local + op.
pub fn pattern_load_op_5(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #5: intern + say.
pub fn pattern_say_5(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #5: push/show layer pair.
pub fn pattern_layer_5(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #6: emit load local + op.
pub fn pattern_load_op_6(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #6: intern + say.
pub fn pattern_say_6(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #6: push/show layer pair.
pub fn pattern_layer_6(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #7: emit load local + op.
pub fn pattern_load_op_7(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #7: intern + say.
pub fn pattern_say_7(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #7: push/show layer pair.
pub fn pattern_layer_7(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #8: emit load local + op.
pub fn pattern_load_op_8(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #8: intern + say.
pub fn pattern_say_8(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #8: push/show layer pair.
pub fn pattern_layer_8(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #9: emit load local + op.
pub fn pattern_load_op_9(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #9: intern + say.
pub fn pattern_say_9(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #9: push/show layer pair.
pub fn pattern_layer_9(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #10: emit load local + op.
pub fn pattern_load_op_10(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #10: intern + say.
pub fn pattern_say_10(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #10: push/show layer pair.
pub fn pattern_layer_10(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #11: emit load local + op.
pub fn pattern_load_op_11(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #11: intern + say.
pub fn pattern_say_11(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #11: push/show layer pair.
pub fn pattern_layer_11(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #12: emit load local + op.
pub fn pattern_load_op_12(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #12: intern + say.
pub fn pattern_say_12(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #12: push/show layer pair.
pub fn pattern_layer_12(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #13: emit load local + op.
pub fn pattern_load_op_13(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #13: intern + say.
pub fn pattern_say_13(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #13: push/show layer pair.
pub fn pattern_layer_13(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #14: emit load local + op.
pub fn pattern_load_op_14(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #14: intern + say.
pub fn pattern_say_14(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #14: push/show layer pair.
pub fn pattern_layer_14(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #15: emit load local + op.
pub fn pattern_load_op_15(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #15: intern + say.
pub fn pattern_say_15(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #15: push/show layer pair.
pub fn pattern_layer_15(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #16: emit load local + op.
pub fn pattern_load_op_16(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #16: intern + say.
pub fn pattern_say_16(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #16: push/show layer pair.
pub fn pattern_layer_16(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #17: emit load local + op.
pub fn pattern_load_op_17(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #17: intern + say.
pub fn pattern_say_17(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #17: push/show layer pair.
pub fn pattern_layer_17(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #18: emit load local + op.
pub fn pattern_load_op_18(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #18: intern + say.
pub fn pattern_say_18(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #18: push/show layer pair.
pub fn pattern_layer_18(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #19: emit load local + op.
pub fn pattern_load_op_19(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #19: intern + say.
pub fn pattern_say_19(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #19: push/show layer pair.
pub fn pattern_layer_19(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #20: emit load local + op.
pub fn pattern_load_op_20(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #20: intern + say.
pub fn pattern_say_20(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #20: push/show layer pair.
pub fn pattern_layer_20(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #21: emit load local + op.
pub fn pattern_load_op_21(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #21: intern + say.
pub fn pattern_say_21(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #21: push/show layer pair.
pub fn pattern_layer_21(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #22: emit load local + op.
pub fn pattern_load_op_22(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #22: intern + say.
pub fn pattern_say_22(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #22: push/show layer pair.
pub fn pattern_layer_22(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #23: emit load local + op.
pub fn pattern_load_op_23(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #23: intern + say.
pub fn pattern_say_23(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #23: push/show layer pair.
pub fn pattern_layer_23(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #24: emit load local + op.
pub fn pattern_load_op_24(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #24: intern + say.
pub fn pattern_say_24(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #24: push/show layer pair.
pub fn pattern_layer_24(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #25: emit load local + op.
pub fn pattern_load_op_25(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #25: intern + say.
pub fn pattern_say_25(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #25: push/show layer pair.
pub fn pattern_layer_25(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #26: emit load local + op.
pub fn pattern_load_op_26(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #26: intern + say.
pub fn pattern_say_26(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #26: push/show layer pair.
pub fn pattern_layer_26(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #27: emit load local + op.
pub fn pattern_load_op_27(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #27: intern + say.
pub fn pattern_say_27(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #27: push/show layer pair.
pub fn pattern_layer_27(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #28: emit load local + op.
pub fn pattern_load_op_28(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #28: intern + say.
pub fn pattern_say_28(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #28: push/show layer pair.
pub fn pattern_layer_28(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #29: emit load local + op.
pub fn pattern_load_op_29(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #29: intern + say.
pub fn pattern_say_29(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #29: push/show layer pair.
pub fn pattern_layer_29(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #30: emit load local + op.
pub fn pattern_load_op_30(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #30: intern + say.
pub fn pattern_say_30(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #30: push/show layer pair.
pub fn pattern_layer_30(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #31: emit load local + op.
pub fn pattern_load_op_31(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #31: intern + say.
pub fn pattern_say_31(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #31: push/show layer pair.
pub fn pattern_layer_31(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #32: emit load local + op.
pub fn pattern_load_op_32(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #32: intern + say.
pub fn pattern_say_32(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #32: push/show layer pair.
pub fn pattern_layer_32(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #33: emit load local + op.
pub fn pattern_load_op_33(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #33: intern + say.
pub fn pattern_say_33(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #33: push/show layer pair.
pub fn pattern_layer_33(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #34: emit load local + op.
pub fn pattern_load_op_34(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #34: intern + say.
pub fn pattern_say_34(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #34: push/show layer pair.
pub fn pattern_layer_34(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #35: emit load local + op.
pub fn pattern_load_op_35(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #35: intern + say.
pub fn pattern_say_35(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #35: push/show layer pair.
pub fn pattern_layer_35(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #36: emit load local + op.
pub fn pattern_load_op_36(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #36: intern + say.
pub fn pattern_say_36(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #36: push/show layer pair.
pub fn pattern_layer_36(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #37: emit load local + op.
pub fn pattern_load_op_37(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #37: intern + say.
pub fn pattern_say_37(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #37: push/show layer pair.
pub fn pattern_layer_37(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #38: emit load local + op.
pub fn pattern_load_op_38(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #38: intern + say.
pub fn pattern_say_38(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #38: push/show layer pair.
pub fn pattern_layer_38(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #39: emit load local + op.
pub fn pattern_load_op_39(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #39: intern + say.
pub fn pattern_say_39(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #39: push/show layer pair.
pub fn pattern_layer_39(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #40: emit load local + op.
pub fn pattern_load_op_40(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #40: intern + say.
pub fn pattern_say_40(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #40: push/show layer pair.
pub fn pattern_layer_40(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #41: emit load local + op.
pub fn pattern_load_op_41(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #41: intern + say.
pub fn pattern_say_41(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #41: push/show layer pair.
pub fn pattern_layer_41(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #42: emit load local + op.
pub fn pattern_load_op_42(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #42: intern + say.
pub fn pattern_say_42(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #42: push/show layer pair.
pub fn pattern_layer_42(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #43: emit load local + op.
pub fn pattern_load_op_43(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #43: intern + say.
pub fn pattern_say_43(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #43: push/show layer pair.
pub fn pattern_layer_43(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #44: emit load local + op.
pub fn pattern_load_op_44(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #44: intern + say.
pub fn pattern_say_44(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #44: push/show layer pair.
pub fn pattern_layer_44(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #45: emit load local + op.
pub fn pattern_load_op_45(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #45: intern + say.
pub fn pattern_say_45(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #45: push/show layer pair.
pub fn pattern_layer_45(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #46: emit load local + op.
pub fn pattern_load_op_46(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #46: intern + say.
pub fn pattern_say_46(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #46: push/show layer pair.
pub fn pattern_layer_46(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #47: emit load local + op.
pub fn pattern_load_op_47(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #47: intern + say.
pub fn pattern_say_47(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #47: push/show layer pair.
pub fn pattern_layer_47(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #48: emit load local + op.
pub fn pattern_load_op_48(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #48: intern + say.
pub fn pattern_say_48(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #48: push/show layer pair.
pub fn pattern_layer_48(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #49: emit load local + op.
pub fn pattern_load_op_49(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #49: intern + say.
pub fn pattern_say_49(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #49: push/show layer pair.
pub fn pattern_layer_49(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #50: emit load local + op.
pub fn pattern_load_op_50(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #50: intern + say.
pub fn pattern_say_50(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #50: push/show layer pair.
pub fn pattern_layer_50(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #51: emit load local + op.
pub fn pattern_load_op_51(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #51: intern + say.
pub fn pattern_say_51(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #51: push/show layer pair.
pub fn pattern_layer_51(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #52: emit load local + op.
pub fn pattern_load_op_52(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #52: intern + say.
pub fn pattern_say_52(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #52: push/show layer pair.
pub fn pattern_layer_52(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #53: emit load local + op.
pub fn pattern_load_op_53(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #53: intern + say.
pub fn pattern_say_53(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #53: push/show layer pair.
pub fn pattern_layer_53(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #54: emit load local + op.
pub fn pattern_load_op_54(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #54: intern + say.
pub fn pattern_say_54(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #54: push/show layer pair.
pub fn pattern_layer_54(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #55: emit load local + op.
pub fn pattern_load_op_55(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #55: intern + say.
pub fn pattern_say_55(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #55: push/show layer pair.
pub fn pattern_layer_55(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #56: emit load local + op.
pub fn pattern_load_op_56(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #56: intern + say.
pub fn pattern_say_56(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #56: push/show layer pair.
pub fn pattern_layer_56(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #57: emit load local + op.
pub fn pattern_load_op_57(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #57: intern + say.
pub fn pattern_say_57(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #57: push/show layer pair.
pub fn pattern_layer_57(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #58: emit load local + op.
pub fn pattern_load_op_58(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #58: intern + say.
pub fn pattern_say_58(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #58: push/show layer pair.
pub fn pattern_layer_58(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #59: emit load local + op.
pub fn pattern_load_op_59(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #59: intern + say.
pub fn pattern_say_59(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #59: push/show layer pair.
pub fn pattern_layer_59(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #60: emit load local + op.
pub fn pattern_load_op_60(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #60: intern + say.
pub fn pattern_say_60(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #60: push/show layer pair.
pub fn pattern_layer_60(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #61: emit load local + op.
pub fn pattern_load_op_61(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #61: intern + say.
pub fn pattern_say_61(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #61: push/show layer pair.
pub fn pattern_layer_61(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #62: emit load local + op.
pub fn pattern_load_op_62(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #62: intern + say.
pub fn pattern_say_62(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #62: push/show layer pair.
pub fn pattern_layer_62(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #63: emit load local + op.
pub fn pattern_load_op_63(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #63: intern + say.
pub fn pattern_say_63(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #63: push/show layer pair.
pub fn pattern_layer_63(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #64: emit load local + op.
pub fn pattern_load_op_64(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #64: intern + say.
pub fn pattern_say_64(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #64: push/show layer pair.
pub fn pattern_layer_64(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #65: emit load local + op.
pub fn pattern_load_op_65(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #65: intern + say.
pub fn pattern_say_65(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #65: push/show layer pair.
pub fn pattern_layer_65(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #66: emit load local + op.
pub fn pattern_load_op_66(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #66: intern + say.
pub fn pattern_say_66(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #66: push/show layer pair.
pub fn pattern_layer_66(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #67: emit load local + op.
pub fn pattern_load_op_67(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #67: intern + say.
pub fn pattern_say_67(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #67: push/show layer pair.
pub fn pattern_layer_67(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #68: emit load local + op.
pub fn pattern_load_op_68(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #68: intern + say.
pub fn pattern_say_68(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #68: push/show layer pair.
pub fn pattern_layer_68(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #69: emit load local + op.
pub fn pattern_load_op_69(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #69: intern + say.
pub fn pattern_say_69(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #69: push/show layer pair.
pub fn pattern_layer_69(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #70: emit load local + op.
pub fn pattern_load_op_70(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #70: intern + say.
pub fn pattern_say_70(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #70: push/show layer pair.
pub fn pattern_layer_70(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #71: emit load local + op.
pub fn pattern_load_op_71(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #71: intern + say.
pub fn pattern_say_71(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #71: push/show layer pair.
pub fn pattern_layer_71(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #72: emit load local + op.
pub fn pattern_load_op_72(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #72: intern + say.
pub fn pattern_say_72(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #72: push/show layer pair.
pub fn pattern_layer_72(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #73: emit load local + op.
pub fn pattern_load_op_73(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #73: intern + say.
pub fn pattern_say_73(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #73: push/show layer pair.
pub fn pattern_layer_73(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #74: emit load local + op.
pub fn pattern_load_op_74(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #74: intern + say.
pub fn pattern_say_74(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #74: push/show layer pair.
pub fn pattern_layer_74(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #75: emit load local + op.
pub fn pattern_load_op_75(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #75: intern + say.
pub fn pattern_say_75(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #75: push/show layer pair.
pub fn pattern_layer_75(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #76: emit load local + op.
pub fn pattern_load_op_76(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #76: intern + say.
pub fn pattern_say_76(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #76: push/show layer pair.
pub fn pattern_layer_76(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #77: emit load local + op.
pub fn pattern_load_op_77(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #77: intern + say.
pub fn pattern_say_77(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #77: push/show layer pair.
pub fn pattern_layer_77(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #78: emit load local + op.
pub fn pattern_load_op_78(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #78: intern + say.
pub fn pattern_say_78(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #78: push/show layer pair.
pub fn pattern_layer_78(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #79: emit load local + op.
pub fn pattern_load_op_79(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #79: intern + say.
pub fn pattern_say_79(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #79: push/show layer pair.
pub fn pattern_layer_79(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #80: emit load local + op.
pub fn pattern_load_op_80(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #80: intern + say.
pub fn pattern_say_80(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #80: push/show layer pair.
pub fn pattern_layer_80(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #81: emit load local + op.
pub fn pattern_load_op_81(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #81: intern + say.
pub fn pattern_say_81(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #81: push/show layer pair.
pub fn pattern_layer_81(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #82: emit load local + op.
pub fn pattern_load_op_82(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #82: intern + say.
pub fn pattern_say_82(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #82: push/show layer pair.
pub fn pattern_layer_82(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #83: emit load local + op.
pub fn pattern_load_op_83(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #83: intern + say.
pub fn pattern_say_83(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #83: push/show layer pair.
pub fn pattern_layer_83(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #84: emit load local + op.
pub fn pattern_load_op_84(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #84: intern + say.
pub fn pattern_say_84(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #84: push/show layer pair.
pub fn pattern_layer_84(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #85: emit load local + op.
pub fn pattern_load_op_85(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #85: intern + say.
pub fn pattern_say_85(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #85: push/show layer pair.
pub fn pattern_layer_85(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #86: emit load local + op.
pub fn pattern_load_op_86(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #86: intern + say.
pub fn pattern_say_86(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #86: push/show layer pair.
pub fn pattern_layer_86(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #87: emit load local + op.
pub fn pattern_load_op_87(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #87: intern + say.
pub fn pattern_say_87(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #87: push/show layer pair.
pub fn pattern_layer_87(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #88: emit load local + op.
pub fn pattern_load_op_88(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #88: intern + say.
pub fn pattern_say_88(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #88: push/show layer pair.
pub fn pattern_layer_88(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

/// Pattern helper #89: emit load local + op.
pub fn pattern_load_op_89(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {
    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
    unit.emit(Vs2Instr::new(op));
}

/// Pattern helper #89: intern + say.
pub fn pattern_say_89(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Pattern helper #89: push/show layer pair.
pub fn pattern_layer_89(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}


#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::{
        HirExpr, HirId, HirItem, HirLit, HirModule, HirScene, HirSpan, HirStmt, HirFn,
        Visibility, HirTy, PrimTy,
    };

    fn empty_mod() -> HirModule {
        let mut m = HirModule::new(2);
        m.file = Some("test.vel".into());
        m
    }

    #[test]
    fn pool_intern_dedup() {
        let mut p = Vs2Pool::new();
        let a = p.intern("hi");
        let b = p.intern("hi");
        assert_eq!(a, b);
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn lower_empty_ok() {
        let m = empty_mod();
        let u = lower_module(&m);
        assert!(u.code.is_empty());
        assert!(validate_unit(&u).is_ok());
    }

    #[test]
    fn lower_scene_say() {
        let mut m = empty_mod();
        m.items.push(HirItem::Scene(HirScene {
            id: HirId(1),
            name: "start".into(),
            body: vec![HirStmt::Say {
                speaker: Some("eira".into()),
                msg: HirExpr::Lit {
                    lit: HirLit::MsgId("dlg.hello".into()),
                    span: HirSpan::unknown(),
                },
                span: HirSpan::unknown(),
            }],
            span: HirSpan::unknown(),
        }));
        let u = lower_module(&m);
        assert!(u.entry_scenes.contains_key("start"));
        assert!(count_op(&u, OpVs2::Say) >= 1);
        assert!(count_op(&u, OpVs2::LoadMsg) >= 1);
    }

    #[test]
    fn lower_fn_block() {
        let mut m = empty_mod();
        m.items.push(HirItem::Fn(HirFn {
            id: HirId(1),
            name: "main".into(),
            vis: Visibility::Public,
            params: vec![],
            ret: HirTy::Prim(PrimTy::Unit),
            body: HirExpr::Block {
                stmts: vec![],
                tail: Some(Box::new(HirExpr::Lit {
                    lit: HirLit::Int(1),
                    span: HirSpan::unknown(),
                })),
                span: HirSpan::unknown(),
            },
            span: HirSpan::unknown(),
        }));
        let u = lower_module(&m);
        assert!(u.fn_entries.contains_key("main"));
        assert!(count_op(&u, OpVs2::LoadConst) >= 1);
    }

    #[test]
    fn peephole_drops_const_pop() {
        let mut u = Vs2Unit::new("t");
        u.emit(Vs2Instr::with_a(OpVs2::LoadConst, 1));
        u.emit(Vs2Instr::new(OpVs2::Pop));
        u.emit(Vs2Instr::new(OpVs2::Nop));
        optimize_unit(&mut u);
        assert_eq!(count_op(&u, OpVs2::Nop), 0);
    }

    #[test]
    fn encode_blob_magic() {
        let u = Vs2Unit::new("x");
        let b = u.encode_blob();
        assert_eq!(b[0], 0x5653_3201);
    }

    #[test]
    fn story_density_bounds() {
        let mut u = Vs2Unit::new("s");
        u.emit(Vs2Instr::new(OpVs2::Say));
        u.emit(Vs2Instr::new(OpVs2::Add));
        let d = story_density(&u);
        assert!(d > 0.0 && d <= 1.0);
    }

    #[test]
    fn pattern_say_works() {
        let mut u = Vs2Unit::new("p");
        pattern_say_0(&mut u, "a", "k");
        assert_eq!(count_op(&u, OpVs2::Say), 1);
    }

    #[test]
    fn emit_helpers_smoke() {
        let mut u = Vs2Unit::new("h");
        emit_add(&mut u, 0, 0);
        emit_say(&mut u, 1, 0);
        emit_push_layer(&mut u, 2, 0);
        assert!(u.code.len() >= 3);
    }

    #[test]
    fn ty_tag_layer() {
        assert_eq!(ty_tag(&HirTy::LayerId), "LayerId");
        assert_eq!(ty_tag(&HirTy::MsgId), "MsgId");
    }
}
