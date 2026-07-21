//! VS2 real codegen: HIR modules → OpVs2 instruction streams.
//!
//! Deterministic lower for story + logic so the VM / story host can execute
//! typed handles without Python eval. Matches `velvet-script-hir` shapes.

#![allow(missing_docs)]
#![allow(dead_code)]

use std::collections::HashMap;
use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_hir::{
    HirBinOp, HirExpr, HirItem, HirLit, HirModule, HirPath, HirSpan, HirStmt, HirTy, PrimTy,
};
use velvet_script_syntax::DiagCode;

/// Structured diagnostic for VS2 lower (file + span, not bare strings only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs2Diag {
    pub code: DiagCode,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub start: usize,
    pub end: usize,
    pub node_kind: Option<String>,
}

impl Vs2Diag {
    pub fn unsupported(
        file: impl Into<String>,
        span: HirSpan,
        construct: &str,
        node_kind: &str,
    ) -> Self {
        Self {
            code: DiagCode::UnsupportedHir,
            message: format!(
                "{}: {} (not lowered in VS2 2.5; will not silently compile)",
                DiagCode::UnsupportedHir.message(),
                construct
            ),
            file: file.into(),
            line: span.line.max(1),
            column: span.column.max(1),
            start: span.start,
            end: span.end,
            node_kind: Some(node_kind.into()),
        }
    }

    pub fn invalid_jump(file: impl Into<String>, span: HirSpan, label: &str) -> Self {
        Self {
            code: DiagCode::InvalidJumpTarget,
            message: format!("unresolved label '{label}'"),
            file: file.into(),
            line: span.line.max(1),
            column: span.column.max(1),
            start: span.start,
            end: span.end,
            node_kind: Some("jump".into()),
        }
    }

    pub fn display(&self) -> String {
        format!(
            "{}:{}:{}: [{}] {}",
            self.file,
            self.line,
            self.column,
            self.code.label(),
            self.message
        )
    }
}

/// PC → original source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs2SourceMapEntry {
    pub pc: u32,
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub node_kind: String,
}

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
        Self { op, a, b, line: 0 }
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
    /// Structured diagnostics (preferred).
    pub diags: Vec<Vs2Diag>,
    /// Legacy string diagnostics (mirrors `diags` for older callers).
    pub diagnostics: Vec<String>,
    /// PC → source map.
    pub source_map: Vec<Vs2SourceMapEntry>,
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
            diags: Vec::new(),
            diagnostics: Vec::new(),
            source_map: Vec::new(),
        }
    }

    pub fn push_diag(&mut self, d: Vs2Diag) {
        self.diagnostics.push(d.display());
        self.diags.push(d);
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
            || self.diagnostics.iter().any(|s| {
                s.contains("unresolved") || s.contains("Unsupported") || s.contains("not yet")
            })
    }

    pub fn map_pc(&mut self, pc: u32, file: &str, span: HirSpan, node_kind: &str) {
        self.source_map.push(Vs2SourceMapEntry {
            pc,
            file: file.into(),
            line: span.line.max(1),
            column: span.column.max(1),
            node_kind: node_kind.into(),
        });
    }

    pub fn lookup_pc(&self, pc: u32) -> Option<&Vs2SourceMapEntry> {
        self.source_map
            .iter()
            .filter(|e| e.pc <= pc)
            .max_by_key(|e| e.pc)
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
    let file = name.clone();
    let mut unit = Vs2Unit::new(name);
    let mut ctx = LowerCtx::new();
    for item in &m.items {
        match item {
            HirItem::Fn(f) => lower_fn(&mut unit, &mut ctx, f, &file),
            HirItem::Scene(sc) => lower_scene(&mut unit, &mut ctx, sc, &file),
            HirItem::Struct(s) => unit.push_diag(Vs2Diag::unsupported(
                &file,
                s.span,
                &format!("struct `{}`", s.name),
                "struct",
            )),
            HirItem::Enum(e) => unit.push_diag(Vs2Diag::unsupported(
                &file,
                e.span,
                &format!("enum `{}`", e.name),
                "enum",
            )),
            HirItem::Character(c) => unit.push_diag(Vs2Diag::unsupported(
                &file,
                c.span,
                &format!("character `{}`", c.name),
                "character",
            )),
            HirItem::State { span, .. } => {
                unit.push_diag(Vs2Diag::unsupported(&file, *span, "state block", "state"))
            }
            HirItem::Screen(s) => unit.push_diag(Vs2Diag::unsupported(
                &file,
                s.span,
                &format!("screen `{}`", s.name),
                "screen",
            )),
            HirItem::Mod { name, span, .. } => unit.push_diag(Vs2Diag::unsupported(
                &file,
                *span,
                &format!("mod `{name}`"),
                "mod",
            )),
            HirItem::Use { path, span } => unit.push_diag(Vs2Diag::unsupported(
                &file,
                *span,
                &format!("use `{}`", path.display()),
                "use",
            )),
        }
    }
    let pending = ctx.pending_jumps.clone();
    for (pc, lab) in pending {
        if let Some(&target) = ctx.labels.get(&lab) {
            unit.patch_a(pc, target);
        } else {
            unit.push_diag(Vs2Diag::invalid_jump(&file, HirSpan::unknown(), &lab));
        }
    }
    unit.local_slots = ctx.next_local;
    unit
}

fn lower_fn(unit: &mut Vs2Unit, ctx: &mut LowerCtx, f: &velvet_script_hir::HirFn, file: &str) {
    let entry = unit.pc();
    unit.fn_entries.insert(f.name.clone(), entry);
    unit.map_pc(entry, file, f.span, "fn");
    for (name, _) in &f.params {
        let _ = ctx.local(name);
    }
    lower_expr(unit, ctx, &f.body, file);
    unit.emit(Vs2Instr::new(OpVs2::Ret));
}

fn lower_scene(
    unit: &mut Vs2Unit,
    ctx: &mut LowerCtx,
    sc: &velvet_script_hir::HirScene,
    file: &str,
) {
    let entry = unit.pc();
    unit.entry_scenes.insert(sc.name.clone(), entry);
    unit.map_pc(entry, file, sc.span, "scene");
    ctx.bind_label(&sc.name, entry);
    for st in &sc.body {
        lower_stmt(unit, ctx, st, file);
    }
    unit.emit(Vs2Instr::new(OpVs2::Ret));
}

fn lower_stmt(unit: &mut Vs2Unit, ctx: &mut LowerCtx, st: &HirStmt, file: &str) {
    match st {
        HirStmt::Expr { expr, .. } => {
            lower_expr(unit, ctx, expr, file);
            unit.emit(Vs2Instr::new(OpVs2::Pop));
        }
        HirStmt::Let { name, init, .. } => {
            if let Some(v) = init {
                lower_expr(unit, ctx, v, file);
            } else {
                let empty = unit.pool.intern("");
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, empty));
            }
            let slot = ctx.local(name);
            unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot));
        }
        HirStmt::Assign { target, value, .. } => {
            lower_expr(unit, ctx, value, file);
            let name = target.display();
            let slot = ctx.local(&name);
            unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot));
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                lower_expr(unit, ctx, v, file);
            }
            unit.emit(Vs2Instr::new(OpVs2::Ret));
        }
        HirStmt::Say { speaker, msg, .. } => {
            let sp = unit.pool.intern(speaker.as_deref().unwrap_or("narrator"));
            lower_expr(unit, ctx, msg, file);
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
        HirStmt::Show { character, at, .. } => {
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

fn lower_expr(unit: &mut Vs2Unit, ctx: &mut LowerCtx, e: &HirExpr, file: &str) {
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
            lower_expr(unit, ctx, lhs, file);
            lower_expr(unit, ctx, rhs, file);
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
                lower_expr(unit, ctx, a, file);
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
        HirExpr::Field {
            base, field, span, ..
        } => {
            unit.push_diag(Vs2Diag::unsupported(
                file,
                *span,
                &format!("field access `.{field}`"),
                "field",
            ));
            // Do not silently compile as base-only.
            lower_expr(unit, ctx, base, file);
            unit.emit(Vs2Instr::new(OpVs2::Pop));
            let z = unit.pool.intern("0");
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, z));
        }
        HirExpr::If {
            cond,
            then_br,
            else_br,
            ..
        } => {
            lower_expr(unit, ctx, cond, file);
            let j_else = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0));
            lower_expr(unit, ctx, then_br, file);
            let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0));
            let else_pc = unit.pc();
            unit.patch_a(j_else, else_pc);
            if let Some(eb) = else_br {
                lower_expr(unit, ctx, eb, file);
            } else {
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, 0));
            }
            let end = unit.pc();
            unit.patch_a(j_end, end);
        }
        HirExpr::Block { stmts, tail, .. } => {
            for s in stmts {
                lower_stmt(unit, ctx, s, file);
            }
            if let Some(t) = tail {
                lower_expr(unit, ctx, t, file);
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

/// Emit load-msg + say (single helper, not N clones).
pub fn pattern_say(unit: &mut Vs2Unit, speaker: &str, msg: &str) {
    let sp = unit.pool.intern(speaker);
    let mid = unit.pool.intern(msg);
    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));
    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
}

/// Emit push + show layer.
pub fn pattern_layer(unit: &mut Vs2Unit, layer: &str) {
    let id = unit.pool.intern(layer);
    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));
    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_script_hir::{
        HirExpr, HirFn, HirId, HirItem, HirLit, HirModule, HirScene, HirSpan, HirStmt, HirTy,
        PrimTy, Visibility,
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
        assert!(!u.has_errors());
    }

    #[test]
    fn unsupported_struct_enum_mod_use_exact() {
        use velvet_script_hir::{HirEnum, HirPath, HirStruct, PathSeg};
        let mut m = empty_mod();
        m.file = Some("game.vel".into());
        m.items.push(HirItem::Struct(HirStruct {
            id: HirId(1),
            name: "Foo".into(),
            vis: Visibility::Public,
            fields: vec![],
            span: HirSpan::at(3, 1, 0, 10),
        }));
        m.items.push(HirItem::Enum(HirEnum {
            id: HirId(2),
            name: "E".into(),
            vis: Visibility::Public,
            variants: vec![],
            span: HirSpan::at(4, 1, 0, 10),
        }));
        m.items.push(HirItem::Mod {
            name: "inner".into(),
            items: vec![],
            span: HirSpan::at(5, 1, 0, 10),
        });
        m.items.push(HirItem::Use {
            path: HirPath {
                segs: vec![PathSeg("std".into())],
            },
            span: HirSpan::at(6, 1, 0, 10),
        });
        let u = lower_module(&m);
        assert!(u.has_errors());
        assert!(u.diags.len() >= 4);
        for d in &u.diags {
            assert_eq!(d.code, velvet_script_syntax::DiagCode::UnsupportedHir);
            assert_eq!(d.file, "game.vel");
            assert!(d.line >= 1);
            assert!(d.display().contains("game.vel"));
        }
        let kinds: Vec<_> = u
            .diags
            .iter()
            .filter_map(|d| d.node_kind.as_deref())
            .collect();
        assert!(kinds.contains(&"struct"));
        assert!(kinds.contains(&"enum"));
        assert!(kinds.contains(&"mod"));
        assert!(kinds.contains(&"use"));
    }

    #[test]
    fn field_access_unsupported_exact() {
        let mut m = empty_mod();
        m.file = Some("player.vel".into());
        m.items.push(HirItem::Fn(HirFn {
            id: HirId(1),
            name: "main".into(),
            vis: Visibility::Public,
            params: vec![],
            ret: HirTy::Prim(PrimTy::Unit),
            body: HirExpr::Field {
                base: Box::new(HirExpr::Path {
                    path: velvet_script_hir::HirPath::parse("player"),
                    span: HirSpan::at(2, 1, 0, 6),
                }),
                field: "health".into(),
                span: HirSpan::at(2, 1, 0, 13),
            },
            span: HirSpan::at(1, 1, 0, 20),
        }));
        let u = lower_module(&m);
        assert!(u
            .diags
            .iter()
            .any(|d| d.node_kind.as_deref() == Some("field")
                && d.code == velvet_script_syntax::DiagCode::UnsupportedHir
                && d.file == "player.vel"
                && d.message.contains("health")));
    }

    #[test]
    fn source_map_records_scene_file_line() {
        let mut m = empty_mod();
        m.file = Some("story.vel".into());
        m.items.push(HirItem::Scene(HirScene {
            id: HirId(1),
            name: "intro".into(),
            body: vec![],
            span: HirSpan::at(10, 2, 100, 110),
        }));
        let u = lower_module(&m);
        let e = u.lookup_pc(0).expect("map entry");
        assert_eq!(e.file, "story.vel");
        assert_eq!(e.line, 10);
        assert_eq!(e.column, 2);
        assert_eq!(e.node_kind, "scene");
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
        pattern_say(&mut u, "a", "k");
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
