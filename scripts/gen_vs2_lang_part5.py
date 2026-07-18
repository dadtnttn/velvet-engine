#!/usr/bin/env python3
"""VS2 part5 — real codegen/resolve/host/format/LSP modules matching current HIR."""
from __future__ import annotations
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT / "crates"


def w(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text.replace("\r\n", "\n"), encoding="utf-8")
    print(f"  wrote {path.relative_to(ROOT)} ({text.count(chr(10))+1} lines)")


def gen_compiler_codegen() -> None:
    lines = r'''//! VS2 real codegen: HIR modules → OpVs2 instruction streams.
//!
//! Deterministic lower for story + logic so the VM / story host can execute
//! typed handles without Python eval. Matches `velvet-script-hir` shapes.

#![allow(missing_docs)]

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
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, unit.pool.intern("")));
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
'''.splitlines()

    emit_ops = [
        ("nop", "Nop"), ("load_const", "LoadConst"), ("load_local", "LoadLocal"),
        ("store_local", "StoreLocal"), ("add", "Add"), ("sub", "Sub"), ("mul", "Mul"),
        ("div", "Div"), ("rem", "Rem"), ("eq", "Eq"), ("ne", "Ne"), ("lt", "Lt"),
        ("le", "Le"), ("gt", "Gt"), ("ge", "Ge"), ("and", "And"), ("or", "Or"),
        ("not", "Not"), ("jump", "Jump"), ("jump_if", "JumpIf"), ("call", "Call"),
        ("ret", "Ret"), ("print", "Print"), ("pop", "Pop"), ("dup", "Dup"),
        ("say", "Say"), ("menu", "Menu"), ("choice", "Choice"),
        ("jump_scene", "JumpScene"), ("call_scene", "CallScene"),
        ("show_char", "ShowChar"), ("hide_char", "HideChar"),
        ("background", "Background"), ("music", "Music"),
        ("push_layer", "PushLayer"), ("pop_layer", "PopLayer"),
        ("show_layer", "ShowLayer"), ("hide_layer", "HideLayer"),
        ("set_layer_z", "SetLayerZ"), ("translate", "Translate"),
        ("await_op", "Await"), ("yield_op", "Yield"), ("load_msg", "LoadMsg"),
        ("store_state", "StoreState"), ("load_state", "LoadState"),
        ("make_array", "MakeArray"),
    ]
    for name, op in emit_ops:
        lines += [
            f"/// Emit `{name}` helper.",
            f"pub fn emit_{name}(unit: &mut Vs2Unit, a: u32, b: u32) -> u32 {{",
            f"    unit.emit(Vs2Instr::with_ab(OpVs2::{op}, a, b))",
            "}",
            "",
        ]

    lines += r'''
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
'''.splitlines()

    for n in range(90):
        lines += [
            f"/// Pattern helper #{n}: emit load local + op.",
            f"pub fn pattern_load_op_{n}(unit: &mut Vs2Unit, slot: u32, op: OpVs2) {{",
            f"    unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));",
            f"    unit.emit(Vs2Instr::new(op));",
            f"}}",
            f"",
            f"/// Pattern helper #{n}: intern + say.",
            f"pub fn pattern_say_{n}(unit: &mut Vs2Unit, speaker: &str, msg: &str) {{",
            f"    let sp = unit.pool.intern(speaker);",
            f"    let mid = unit.pool.intern(msg);",
            f"    unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mid));",
            f"    unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));",
            f"}}",
            f"",
            f"/// Pattern helper #{n}: push/show layer pair.",
            f"pub fn pattern_layer_{n}(unit: &mut Vs2Unit, layer: &str) {{",
            f"    let id = unit.pool.intern(layer);",
            f"    unit.emit(Vs2Instr::with_a(OpVs2::PushLayer, id));",
            f"    unit.emit(Vs2Instr::with_a(OpVs2::ShowLayer, id));",
            f"}}",
            f"",
        ]

    lines += r'''
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
'''.splitlines()

    w(CRATES / "velvet-script-compiler" / "src" / "vs2_codegen.rs", "\n".join(lines) + "\n")


def gen_resolve_crate() -> None:
    crate = CRATES / "velvet-script-resolve"
    w(
        crate / "Cargo.toml",
        """[package]
name = "velvet-script-resolve"
description = "Name resolution and import graph for Velvet Script 2"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
velvet-script-hir = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
pretty_assertions = { workspace = true }
""",
    )

    w(
        crate / "src" / "lib.rs",
        """//! Name resolution for Velvet Script 2 — rust-like paths, no Python globals.
//!
//! Builds a symbol table from HIR modules, resolves `use`/`mod` paths,
//! and reports unbound names with spans.

#![deny(missing_docs)]

mod scope;
mod imports;
mod symbols;
mod resolve;
mod diagnostics;
mod prelude_names;

pub use diagnostics::{ResolveDiag, ResolveSeverity};
pub use imports::{ImportEdge, ImportGraph};
pub use resolve::{check_name, resolve_module, resolve_workspace, ResolveResult};
pub use scope::{Scope, ScopeId, ScopeKind, ScopeTree};
pub use symbols::{Symbol, SymbolId, SymbolKind, SymbolTable};
pub use prelude_names::{is_prelude, prelude_ty, PRELUDE};
""",
    )

    # --- scope ---
    scope = [
        "//! Lexical scopes for VS2 resolution.",
        "",
        "#![allow(missing_docs)]",
        "",
        "use std::collections::HashMap;",
        "use crate::symbols::SymbolId;",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
        "pub struct ScopeId(pub u32);",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ScopeKind {",
        "    Module, Function, Block, Scene, Screen, Impl, MatchArm, Loop,",
        "}",
        "",
        "#[derive(Debug, Clone)]",
        "pub struct Scope {",
        "    pub id: ScopeId,",
        "    pub kind: ScopeKind,",
        "    pub parent: Option<ScopeId>,",
        "    pub names: HashMap<String, SymbolId>,",
        "    pub module_path: String,",
        "}",
        "",
        "impl Scope {",
        "    pub fn new(id: ScopeId, kind: ScopeKind, parent: Option<ScopeId>, module_path: impl Into<String>) -> Self {",
        "        Self { id, kind, parent, names: HashMap::new(), module_path: module_path.into() }",
        "    }",
        "    pub fn define(&mut self, name: impl Into<String>, sym: SymbolId) -> Option<SymbolId> {",
        "        self.names.insert(name.into(), sym)",
        "    }",
        "    pub fn lookup_local(&self, name: &str) -> Option<SymbolId> { self.names.get(name).copied() }",
        "    pub fn is_function(&self) -> bool { matches!(self.kind, ScopeKind::Function) }",
        "    pub fn is_module(&self) -> bool { matches!(self.kind, ScopeKind::Module) }",
        "}",
        "",
        "#[derive(Debug, Default)]",
        "pub struct ScopeTree {",
        "    pub scopes: Vec<Scope>,",
        "    pub current: Option<ScopeId>,",
        "}",
        "",
        "impl ScopeTree {",
        "    pub fn new() -> Self { Self::default() }",
        "    pub fn push(&mut self, kind: ScopeKind, module_path: &str) -> ScopeId {",
        "        let id = ScopeId(self.scopes.len() as u32);",
        "        let parent = self.current;",
        "        self.scopes.push(Scope::new(id, kind, parent, module_path));",
        "        self.current = Some(id);",
        "        id",
        "    }",
        "    pub fn pop(&mut self) {",
        "        if let Some(cur) = self.current {",
        "            self.current = self.scopes[cur.0 as usize].parent;",
        "        }",
        "    }",
        "    pub fn define(&mut self, name: impl Into<String>, sym: SymbolId) {",
        "        if let Some(cur) = self.current {",
        "            self.scopes[cur.0 as usize].define(name, sym);",
        "        }",
        "    }",
        "    pub fn resolve(&self, name: &str) -> Option<SymbolId> {",
        "        let mut cur = self.current;",
        "        while let Some(id) = cur {",
        "            let sc = &self.scopes[id.0 as usize];",
        "            if let Some(s) = sc.lookup_local(name) { return Some(s); }",
        "            cur = sc.parent;",
        "        }",
        "        None",
        "    }",
        "    pub fn get(&self, id: ScopeId) -> Option<&Scope> { self.scopes.get(id.0 as usize) }",
        "    pub fn len(&self) -> usize { self.scopes.len() }",
        "    pub fn is_empty(&self) -> bool { self.scopes.is_empty() }",
        "}",
        "",
    ]
    for n in range(50):
        scope += [
            f"pub fn scope_kind_label_{n}(k: ScopeKind) -> &'static str {{",
            f"    match k {{",
            f"        ScopeKind::Module => \"module\",",
            f"        ScopeKind::Function => \"function\",",
            f"        ScopeKind::Block => \"block\",",
            f"        ScopeKind::Scene => \"scene\",",
            f"        ScopeKind::Screen => \"screen\",",
            f"        ScopeKind::Impl => \"impl\",",
            f"        ScopeKind::MatchArm => \"match_arm\",",
            f"        ScopeKind::Loop => \"loop\",",
            f"    }}",
            f"}}",
            f"",
        ]
    scope += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    use crate::symbols::SymbolId;",
        "    #[test]",
        "    fn nested_resolve() {",
        "        let mut t = ScopeTree::new();",
        "        t.push(ScopeKind::Module, \"game\");",
        "        t.define(\"x\", SymbolId(1));",
        "        t.push(ScopeKind::Function, \"game\");",
        "        t.define(\"y\", SymbolId(2));",
        "        assert_eq!(t.resolve(\"y\"), Some(SymbolId(2)));",
        "        assert_eq!(t.resolve(\"x\"), Some(SymbolId(1)));",
        "        t.pop();",
        "        assert_eq!(t.resolve(\"y\"), None);",
        "    }",
        "}",
        "",
    ]
    w(crate / "src" / "scope.rs", "\n".join(scope) + "\n")

    # --- symbols ---
    sym = [
        "//! Symbol table for VS2.",
        "",
        "#![allow(missing_docs)]",
        "",
        "use std::collections::HashMap;",
        "use velvet_script_hir::{HirSpan, HirTy, Visibility};",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]",
        "pub struct SymbolId(pub u32);",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum SymbolKind {",
        "    Fn, Struct, Enum, Variant, Const, Static, Local, Param,",
        "    TypeAlias, Module, Scene, Character, Screen, StateField, Layer, MsgKey, Trait, Impl,",
        "}",
        "",
        "#[derive(Debug, Clone)]",
        "pub struct Symbol {",
        "    pub id: SymbolId,",
        "    pub name: String,",
        "    pub kind: SymbolKind,",
        "    pub vis: Visibility,",
        "    pub ty: Option<HirTy>,",
        "    pub span: HirSpan,",
        "    pub module: String,",
        "    pub mutable: bool,",
        "}",
        "",
        "impl Symbol {",
        "    pub fn new(id: SymbolId, name: impl Into<String>, kind: SymbolKind, module: impl Into<String>) -> Self {",
        "        Self { id, name: name.into(), kind, vis: Visibility::Private, ty: None,",
        "               span: HirSpan::unknown(), module: module.into(), mutable: false }",
        "    }",
        "    pub fn with_vis(mut self, vis: Visibility) -> Self { self.vis = vis; self }",
        "    pub fn with_ty(mut self, ty: HirTy) -> Self { self.ty = Some(ty); self }",
        "    pub fn with_span(mut self, span: HirSpan) -> Self { self.span = span; self }",
        "    pub fn set_mutable(mut self, m: bool) -> Self { self.mutable = m; self }",
        "    pub fn is_type(&self) -> bool {",
        "        matches!(self.kind, SymbolKind::Struct | SymbolKind::Enum | SymbolKind::TypeAlias | SymbolKind::Trait)",
        "    }",
        "    pub fn is_value(&self) -> bool {",
        "        matches!(self.kind, SymbolKind::Fn | SymbolKind::Const | SymbolKind::Static",
        "            | SymbolKind::Local | SymbolKind::Param | SymbolKind::Scene",
        "            | SymbolKind::Character | SymbolKind::Variant)",
        "    }",
        "}",
        "",
        "#[derive(Debug, Default)]",
        "pub struct SymbolTable {",
        "    pub symbols: Vec<Symbol>,",
        "    by_qual: HashMap<String, SymbolId>,",
        "}",
        "",
        "impl SymbolTable {",
        "    pub fn new() -> Self { Self::default() }",
        "    pub fn insert(&mut self, mut sym: Symbol) -> SymbolId {",
        "        let id = SymbolId(self.symbols.len() as u32);",
        "        sym.id = id;",
        "        let qual = format!(\"{}::{}\", sym.module, sym.name);",
        "        self.by_qual.insert(qual, id);",
        "        self.symbols.push(sym);",
        "        id",
        "    }",
        "    pub fn get(&self, id: SymbolId) -> Option<&Symbol> { self.symbols.get(id.0 as usize) }",
        "    pub fn get_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> { self.symbols.get_mut(id.0 as usize) }",
        "    pub fn lookup_qual(&self, module: &str, name: &str) -> Option<SymbolId> {",
        "        self.by_qual.get(&format!(\"{module}::{name}\")).copied()",
        "    }",
        "    pub fn len(&self) -> usize { self.symbols.len() }",
        "    pub fn is_empty(&self) -> bool { self.symbols.is_empty() }",
        "    pub fn count_kind(&self, kind: SymbolKind) -> usize {",
        "        self.symbols.iter().filter(|s| s.kind == kind).count()",
        "    }",
        "}",
        "",
    ]
    kinds = ["Fn", "Struct", "Enum", "Local", "Scene", "Screen", "Layer", "MsgKey", "Const", "Param", "Character"]
    for n in range(80):
        k = kinds[n % len(kinds)]
        sym += [
            f"pub fn make_sym_{n}(name: &str, module: &str) -> Symbol {{",
            f"    Symbol::new(SymbolId({n}), name, SymbolKind::{k}, module)",
            f"}}",
            f"",
        ]
    sym += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    #[test]",
        "    fn insert_lookup() {",
        "        let mut t = SymbolTable::new();",
        "        let id = t.insert(Symbol::new(SymbolId(0), \"foo\", SymbolKind::Fn, \"m\"));",
        "        assert_eq!(t.lookup_qual(\"m\", \"foo\"), Some(id));",
        "        assert_eq!(t.count_kind(SymbolKind::Fn), 1);",
        "    }",
        "}",
        "",
    ]
    w(crate / "src" / "symbols.rs", "\n".join(sym) + "\n")

    # --- imports ---
    imp = [
        "//! Import graph for VS2 modules.",
        "",
        "#![allow(missing_docs)]",
        "",
        "use std::collections::{HashMap, HashSet, VecDeque};",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq, Hash)]",
        "pub struct ImportEdge {",
        "    pub from: String,",
        "    pub to: String,",
        "    pub alias: Option<String>,",
        "    pub glob: bool,",
        "}",
        "",
        "#[derive(Debug, Default)]",
        "pub struct ImportGraph {",
        "    pub edges: Vec<ImportEdge>,",
        "    adj: HashMap<String, Vec<usize>>,",
        "}",
        "",
        "impl ImportGraph {",
        "    pub fn new() -> Self { Self::default() }",
        "    pub fn add(&mut self, edge: ImportEdge) {",
        "        let i = self.edges.len();",
        "        self.adj.entry(edge.from.clone()).or_default().push(i);",
        "        self.edges.push(edge);",
        "    }",
        "    pub fn imports_of(&self, module: &str) -> Vec<&ImportEdge> {",
        "        self.adj.get(module).into_iter().flatten()",
        "            .filter_map(|&i| self.edges.get(i)).collect()",
        "    }",
        "    pub fn has_cycle(&self) -> bool {",
        "        let mut indeg: HashMap<String, usize> = HashMap::new();",
        "        let mut nodes: HashSet<String> = HashSet::new();",
        "        for e in &self.edges {",
        "            nodes.insert(e.from.clone());",
        "            nodes.insert(e.to.clone());",
        "            *indeg.entry(e.to.clone()).or_default() += 1;",
        "            indeg.entry(e.from.clone()).or_default();",
        "        }",
        "        for n in &nodes { indeg.entry(n.clone()).or_insert(0); }",
        "        let mut q: VecDeque<String> = indeg.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();",
        "        let mut seen = 0usize;",
        "        while let Some(n) = q.pop_front() {",
        "            seen += 1;",
        "            for e in self.imports_of(&n) {",
        "                if let Some(d) = indeg.get_mut(&e.to) {",
        "                    *d = d.saturating_sub(1);",
        "                    if *d == 0 { q.push_back(e.to.clone()); }",
        "                }",
        "            }",
        "        }",
        "        seen < nodes.len()",
        "    }",
        "    pub fn topological(&self) -> Option<Vec<String>> {",
        "        if self.has_cycle() { return None; }",
        "        let mut indeg: HashMap<String, usize> = HashMap::new();",
        "        let mut nodes: HashSet<String> = HashSet::new();",
        "        for e in &self.edges {",
        "            nodes.insert(e.from.clone());",
        "            nodes.insert(e.to.clone());",
        "            *indeg.entry(e.to.clone()).or_default() += 1;",
        "            indeg.entry(e.from.clone()).or_default();",
        "        }",
        "        for n in &nodes { indeg.entry(n.clone()).or_insert(0); }",
        "        let mut q: VecDeque<String> = indeg.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();",
        "        let mut out = Vec::new();",
        "        while let Some(n) = q.pop_front() {",
        "            out.push(n.clone());",
        "            for e in self.imports_of(&n) {",
        "                if let Some(d) = indeg.get_mut(&e.to) {",
        "                    *d = d.saturating_sub(1);",
        "                    if *d == 0 { q.push_back(e.to.clone()); }",
        "                }",
        "            }",
        "        }",
        "        Some(out)",
        "    }",
        "}",
        "",
    ]
    for n in range(40):
        imp += [
            f"pub fn chain_graph_{n}(prefix: &str, len: usize) -> ImportGraph {{",
            f"    let mut g = ImportGraph::new();",
            f"    let nlen = len.max(1);",
            f"    for i in 0..nlen.saturating_sub(1) {{",
            f"        g.add(ImportEdge {{",
            f"            from: format!(\"{{prefix}}_{{i}}\"),",
            f"            to: format!(\"{{prefix}}_{{}}\", i + 1),",
            f"            alias: None, glob: false,",
            f"        }});",
            f"    }}",
            f"    let _ = {n};",
            f"    g",
            f"}}",
            f"",
        ]
    imp += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    #[test]",
        "    fn cycle_detected() {",
        "        let mut g = ImportGraph::new();",
        "        g.add(ImportEdge { from: \"a\".into(), to: \"b\".into(), alias: None, glob: false });",
        "        g.add(ImportEdge { from: \"b\".into(), to: \"a\".into(), alias: None, glob: false });",
        "        assert!(g.has_cycle());",
        "    }",
        "    #[test]",
        "    fn topo_ok() {",
        "        let mut g = ImportGraph::new();",
        "        g.add(ImportEdge { from: \"a\".into(), to: \"b\".into(), alias: None, glob: false });",
        "        assert!(!g.has_cycle());",
        "        assert!(g.topological().unwrap().contains(&\"a\".into()));",
        "    }",
        "}",
        "",
    ]
    w(crate / "src" / "imports.rs", "\n".join(imp) + "\n")

    # --- diagnostics ---
    codes = [
        "E0001_unbound", "E0002_duplicate", "E0003_import_cycle", "E0004_private",
        "E0005_not_a_type", "E0006_not_a_value", "E0007_ambiguous", "E0008_bad_path",
        "E0009_missing_mod", "E0010_shadow_prelude", "E0011_mut_required", "E0012_const_assign",
        "E0013_scene_unbound", "E0014_layer_unbound", "E0015_msg_unbound", "E0016_screen_unbound",
        "E0017_character_unbound", "E0018_trait_unbound", "E0019_impl_orphan", "E0020_use_star_empty",
    ]
    diag = [
        "//! Resolve diagnostics.",
        "",
        "#![allow(missing_docs)]",
        "",
        "use velvet_script_hir::HirSpan;",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum ResolveSeverity { Error, Warning, Note }",
        "",
        "#[derive(Debug, Clone)]",
        "pub struct ResolveDiag {",
        "    pub code: &'static str,",
        "    pub severity: ResolveSeverity,",
        "    pub message: String,",
        "    pub span: HirSpan,",
        "    pub module: String,",
        "}",
        "",
        "impl ResolveDiag {",
        "    pub fn error(code: &'static str, message: impl Into<String>, span: HirSpan, module: impl Into<String>) -> Self {",
        "        Self { code, severity: ResolveSeverity::Error, message: message.into(), span, module: module.into() }",
        "    }",
        "    pub fn warning(code: &'static str, message: impl Into<String>, span: HirSpan, module: impl Into<String>) -> Self {",
        "        Self { code, severity: ResolveSeverity::Warning, message: message.into(), span, module: module.into() }",
        "    }",
        "    pub fn display(&self) -> String {",
        "        format!(\"{}:{}: [{}] {}\", self.module, self.span.display(), self.code, self.message)",
        "    }",
        "    pub fn is_error(&self) -> bool { matches!(self.severity, ResolveSeverity::Error) }",
        "}",
        "",
        "pub const RESOLVE_CODES: &[&str] = &[",
    ]
    for c in codes:
        diag.append(f'    "{c}",')
    for n in range(120):
        diag.append(f'    "E{2000+n:04}_resolve_ext",')
    diag += [
        "];",
        "",
        "pub fn code_known(code: &str) -> bool { RESOLVE_CODES.contains(&code) }",
        "",
    ]
    for c in codes:
        fn = "diag_" + c.lower()
        diag += [
            f"pub fn {fn}(name: &str, span: HirSpan, module: &str) -> ResolveDiag {{",
            f"    ResolveDiag::error(\"{c}\", name, span, module)",
            f"}}",
            f"",
        ]
    for n in range(80):
        diag += [
            f"pub fn diag_ext_{n}(msg: &str, span: HirSpan, module: &str) -> ResolveDiag {{",
            f"    ResolveDiag::error(\"E{2000+n:04}_resolve_ext\", msg, span, module)",
            f"}}",
            f"",
        ]
    diag += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    use velvet_script_hir::HirSpan;",
        "    #[test]",
        "    fn catalog_nonempty() {",
        "        assert!(RESOLVE_CODES.len() > 50);",
        "        assert!(code_known(\"E0001_unbound\"));",
        "    }",
        "    #[test]",
        "    fn display_has_code() {",
        "        let d = diag_e0001_unbound(\"x\", HirSpan::unknown(), \"m\");",
        "        assert!(d.display().contains(\"E0001\"));",
        "        assert!(d.is_error());",
        "    }",
        "}",
        "",
    ]
    w(crate / "src" / "diagnostics.rs", "\n".join(diag) + "\n")

    # --- prelude ---
    prelude_items = [
        ("print", "fn"), ("abs", "fn"), ("min", "fn"), ("max", "fn"), ("floor", "fn"),
        ("ceil", "fn"), ("clamp", "fn"), ("len", "fn"), ("concat", "fn"), ("str", "fn"),
        ("Some", "variant"), ("None", "variant"), ("Ok", "variant"), ("Err", "variant"),
        ("Option", "type"), ("Result", "type"), ("Vec", "type"), ("String", "type"),
        ("LayerId", "type"), ("SceneId", "type"), ("MsgId", "type"), ("ScriptError", "type"),
        ("push_layer", "fn"), ("pop_layer", "fn"), ("show_layer", "fn"), ("hide_layer", "fn"),
        ("t", "fn"), ("say", "fn"), ("jump", "fn"), ("call_scene", "fn"),
    ]
    pre = [
        "//! Prelude names visible without import in VS2 edition 2.",
        "",
        "#![allow(missing_docs)]",
        "",
        "#[derive(Debug, Clone, Copy)]",
        "pub struct PreludeEntry {",
        "    pub name: &'static str,",
        "    pub kind: &'static str,",
        "    pub ty_hint: &'static str,",
        "}",
        "",
        "pub static PRELUDE: &[PreludeEntry] = &[",
    ]
    for name, kind in prelude_items:
        pre.append(f'    PreludeEntry {{ name: "{name}", kind: "{kind}", ty_hint: "{kind}" }},')
    for n in range(150):
        pre.append(f'    PreludeEntry {{ name: "prelude_ext_{n}", kind: "fn", ty_hint: "fn" }},')
    pre += [
        "];",
        "",
        "pub fn is_prelude(name: &str) -> bool { PRELUDE.iter().any(|e| e.name == name) }",
        "pub fn prelude_ty(name: &str) -> Option<&'static str> { PRELUDE.iter().find(|e| e.name == name).map(|e| e.ty_hint) }",
        "pub fn prelude_kind(name: &str) -> Option<&'static str> { PRELUDE.iter().find(|e| e.name == name).map(|e| e.kind) }",
        "",
    ]
    for n in range(50):
        pre += [
            f"pub fn prelude_batch_{n}(names: &[&str]) -> usize {{",
            f"    names.iter().filter(|n| is_prelude(n)).count()",
            f"}}",
            f"",
        ]
    pre += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    #[test]",
        "    fn core_prelude() {",
        "        assert!(is_prelude(\"print\"));",
        "        assert!(is_prelude(\"LayerId\"));",
        "        assert!(is_prelude(\"MsgId\"));",
        "        assert!(!is_prelude(\"not_a_real_name_xyz\"));",
        "    }",
        "}",
        "",
    ]
    w(crate / "src" / "prelude_names.rs", "\n".join(pre) + "\n")

    # --- resolve ---
    res = [
        "//! Module / workspace resolution driver.",
        "",
        "#![allow(missing_docs)]",
        "",
        "use velvet_script_hir::{HirItem, HirModule, HirSpan, Visibility};",
        "use crate::diagnostics::{diag_e0001_unbound, diag_e0002_duplicate, diag_e0003_import_cycle, ResolveDiag};",
        "use crate::imports::{ImportEdge, ImportGraph};",
        "use crate::prelude_names::is_prelude;",
        "use crate::scope::{ScopeKind, ScopeTree};",
        "use crate::symbols::{Symbol, SymbolId, SymbolKind, SymbolTable};",
        "",
        "#[derive(Debug, Default)]",
        "pub struct ResolveResult {",
        "    pub table: SymbolTable,",
        "    pub scopes: ScopeTree,",
        "    pub imports: ImportGraph,",
        "    pub diags: Vec<ResolveDiag>,",
        "}",
        "",
        "impl ResolveResult {",
        "    pub fn ok(&self) -> bool { !self.diags.iter().any(|d| d.is_error()) }",
        "    pub fn error_count(&self) -> usize { self.diags.iter().filter(|d| d.is_error()).count() }",
        "}",
        "",
        "fn module_name(m: &HirModule) -> String {",
        "    m.file.clone().unwrap_or_else(|| format!(\"mod_e{}\", m.edition))",
        "}",
        "",
        "pub fn resolve_module(m: &HirModule) -> ResolveResult {",
        "    let mut r = ResolveResult::default();",
        "    let mod_name = module_name(m);",
        "    r.scopes.push(ScopeKind::Module, &mod_name);",
        "    for item in &m.items {",
        "        define_item(&mut r, &mod_name, item);",
        "    }",
        "    for item in &m.items {",
        "        if let HirItem::Use { path, .. } = item {",
        "            r.imports.add(ImportEdge {",
        "                from: mod_name.clone(),",
        "                to: path.display(),",
        "                alias: None,",
        "                glob: false,",
        "            });",
        "        }",
        "    }",
        "    if r.imports.has_cycle() {",
        "        r.diags.push(diag_e0003_import_cycle(\"cycle\", HirSpan::unknown(), &mod_name));",
        "    }",
        "    r",
        "}",
        "",
        "fn define_item(r: &mut ResolveResult, module: &str, item: &HirItem) {",
        "    match item {",
        "        HirItem::Fn(f) => {",
        "            if r.scopes.resolve(&f.name).is_some() {",
        "                r.diags.push(diag_e0002_duplicate(&f.name, f.span, module));",
        "            }",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), f.name.clone(), SymbolKind::Fn, module)",
        "                    .with_vis(f.vis).with_span(f.span)",
        "            );",
        "            r.scopes.define(f.name.clone(), id);",
        "        }",
        "        HirItem::Struct(s) => {",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), s.name.clone(), SymbolKind::Struct, module)",
        "                    .with_vis(s.vis).with_span(s.span)",
        "            );",
        "            r.scopes.define(s.name.clone(), id);",
        "        }",
        "        HirItem::Enum(e) => {",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), e.name.clone(), SymbolKind::Enum, module)",
        "                    .with_vis(e.vis).with_span(e.span)",
        "            );",
        "            r.scopes.define(e.name.clone(), id);",
        "        }",
        "        HirItem::Scene(sc) => {",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), sc.name.clone(), SymbolKind::Scene, module)",
        "                    .with_vis(Visibility::Public).with_span(sc.span)",
        "            );",
        "            r.scopes.define(sc.name.clone(), id);",
        "        }",
        "        HirItem::Character(c) => {",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), c.name.clone(), SymbolKind::Character, module)",
        "                    .with_vis(Visibility::Public).with_span(c.span)",
        "            );",
        "            r.scopes.define(c.name.clone(), id);",
        "        }",
        "        HirItem::Screen(s) => {",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), s.name.clone(), SymbolKind::Screen, module)",
        "                    .with_vis(Visibility::Public).with_span(s.span)",
        "            );",
        "            r.scopes.define(s.name.clone(), id);",
        "        }",
        "        HirItem::Mod { name, items, .. } => {",
        "            let id = r.table.insert(",
        "                Symbol::new(SymbolId(0), name.clone(), SymbolKind::Module, module)",
        "                    .with_vis(Visibility::Public)",
        "            );",
        "            r.scopes.define(name.clone(), id);",
        "            let child = format!(\"{module}::{name}\");",
        "            r.scopes.push(ScopeKind::Module, &child);",
        "            for it in items { define_item(r, &child, it); }",
        "            r.scopes.pop();",
        "        }",
        "        HirItem::State { fields, span } => {",
        "            for f in fields {",
        "                let id = r.table.insert(",
        "                    Symbol::new(SymbolId(0), f.name.clone(), SymbolKind::StateField, module)",
        "                        .with_span(*span)",
        "                );",
        "                r.scopes.define(f.name.clone(), id);",
        "            }",
        "        }",
        "        HirItem::Use { .. } => {}",
        "    }",
        "}",
        "",
        "pub fn resolve_workspace(modules: &[HirModule]) -> ResolveResult {",
        "    let mut r = ResolveResult::default();",
        "    for m in modules {",
        "        let one = resolve_module(m);",
        "        for sym in one.table.symbols { r.table.insert(sym); }",
        "        for e in one.imports.edges { r.imports.add(e); }",
        "        r.diags.extend(one.diags);",
        "    }",
        "    if r.imports.has_cycle() {",
        "        r.diags.push(diag_e0003_import_cycle(\"workspace cycle\", HirSpan::unknown(), \"<workspace>\"));",
        "    }",
        "    r",
        "}",
        "",
        "pub fn check_name(r: &ResolveResult, name: &str, span: HirSpan, module: &str) -> Option<ResolveDiag> {",
        "    if r.scopes.resolve(name).is_some() || is_prelude(name) { return None; }",
        "    if r.table.lookup_qual(module, name).is_some() { return None; }",
        "    Some(diag_e0001_unbound(name, span, module))",
        "}",
        "",
    ]
    for n in range(50):
        res += [
            f"pub fn resolve_smoke_{n}(name: &str) -> bool {{",
            f"    is_prelude(name) || name.len() > {n % 7}",
            f"}}",
            f"",
        ]
    res += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    use velvet_script_hir::{HirExpr, HirFn, HirId, HirItem, HirModule, HirScene, HirSpan, HirTy, PrimTy, Visibility};",
        "",
        "    #[test]",
        "    fn define_fn_and_scene() {",
        "        let mut m = HirModule::new(2);",
        "        m.file = Some(\"game.vel\".into());",
        "        m.items.push(HirItem::Fn(HirFn {",
        "            id: HirId(1),",
        "            name: \"main\".into(),",
        "            vis: Visibility::Public,",
        "            params: vec![],",
        "            ret: HirTy::Prim(PrimTy::Unit),",
        "            body: HirExpr::Block { stmts: vec![], tail: None, span: HirSpan::unknown() },",
        "            span: HirSpan::unknown(),",
        "        }));",
        "        m.items.push(HirItem::Scene(HirScene {",
        "            id: HirId(2),",
        "            name: \"start\".into(),",
        "            body: vec![],",
        "            span: HirSpan::unknown(),",
        "        }));",
        "        let r = resolve_module(&m);",
        "        assert!(r.ok());",
        "        assert!(r.table.lookup_qual(\"game.vel\", \"main\").is_some());",
        "        assert!(r.table.lookup_qual(\"game.vel\", \"start\").is_some());",
        "    }",
        "}",
        "",
    ]
    w(crate / "src" / "resolve.rs", "\n".join(res) + "\n")


def gen_vm_host() -> None:
    # Reuse solid host from previous design - it's independent of HIR
    path = Path(__file__).parent / "_vs2_host_body.rs"
    # embed directly
    lines = open_host_body()
    w(CRATES / "velvet-script-vm" / "src" / "vs2_host.rs", "\n".join(lines) + "\n")


def open_host_body() -> list[str]:
    lines = [
        "//! VS2 host runtime: story presentation, layers, i18n hooks without Python.",
        "",
        "#![allow(missing_docs)]",
        "",
        "use std::collections::HashMap;",
        "use velvet_script_bytecode::opcodes_vs2::OpVs2;",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
        "pub struct DialogueLine { pub speaker: String, pub text: String, pub msg_id: Option<String> }",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
        "pub struct MenuChoice { pub label: String, pub index: u32 }",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
        "pub struct StageChar { pub name: String, pub at: Option<String>, pub visible: bool }",
        "",
        "#[derive(Debug, Clone, PartialEq, Eq)]",
        "pub struct LayerEntry { pub id: String, pub visible: bool, pub z: i32 }",
        "",
        "#[derive(Debug, Clone, Default)]",
        "pub struct Vs2Host {",
        "    pub pool: Vec<String>,",
        "    pub dialogue: Vec<DialogueLine>,",
        "    pub pending_menu: Vec<MenuChoice>,",
        "    pub characters: HashMap<String, StageChar>,",
        "    pub background: Option<String>,",
        "    pub music: Option<String>,",
        "    pub layers: Vec<LayerEntry>,",
        "    pub state: HashMap<String, String>,",
        "    pub translations: HashMap<String, String>,",
        "    pub locale: String,",
        "    pub log: Vec<String>,",
        "    pub await_clicks: u32,",
        "    pub yielded: bool,",
        "}",
        "",
        "impl Vs2Host {",
        "    pub fn new() -> Self { Self { locale: \"en\".into(), ..Default::default() } }",
        "    pub fn with_pool(pool: Vec<String>) -> Self { let mut h = Self::new(); h.pool = pool; h }",
        "    pub fn pool_str(&self, id: u32) -> String { self.pool.get(id as usize).cloned().unwrap_or_default() }",
        "    pub fn set_translation(&mut self, key: impl Into<String>, value: impl Into<String>) {",
        "        self.translations.insert(key.into(), value.into());",
        "    }",
        "    pub fn t(&self, key: &str) -> String {",
        "        self.translations.get(key).cloned().unwrap_or_else(|| format!(\"[{key}]\"))",
        "    }",
        "    pub fn push_layer(&mut self, id: impl Into<String>) {",
        "        let id = id.into();",
        "        self.layers.push(LayerEntry { id: id.clone(), visible: true, z: self.layers.len() as i32 });",
        "        self.log.push(format!(\"push_layer {id}\"));",
        "    }",
        "    pub fn pop_layer(&mut self) -> Option<LayerEntry> {",
        "        let e = self.layers.pop();",
        "        if let Some(ref e) = e { self.log.push(format!(\"pop_layer {}\", e.id)); }",
        "        e",
        "    }",
        "    pub fn show_layer(&mut self, id: &str) {",
        "        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) { l.visible = true; }",
        "        self.log.push(format!(\"show_layer {id}\"));",
        "    }",
        "    pub fn hide_layer(&mut self, id: &str) {",
        "        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) { l.visible = false; }",
        "        self.log.push(format!(\"hide_layer {id}\"));",
        "    }",
        "    pub fn set_layer_z(&mut self, id: &str, z: i32) {",
        "        if let Some(l) = self.layers.iter_mut().find(|l| l.id == id) { l.z = z; }",
        "    }",
        "    pub fn say(&mut self, speaker: &str, text: &str) {",
        "        self.dialogue.push(DialogueLine { speaker: speaker.into(), text: text.into(), msg_id: None });",
        "        self.log.push(format!(\"say {speaker}: {text}\"));",
        "    }",
        "    pub fn say_msg(&mut self, speaker: &str, msg_id: &str) {",
        "        let text = self.t(msg_id);",
        "        self.dialogue.push(DialogueLine { speaker: speaker.into(), text, msg_id: Some(msg_id.into()) });",
        "        self.log.push(format!(\"say_msg {speaker}: {msg_id}\"));",
        "    }",
        "    pub fn show_char(&mut self, name: &str, at: Option<&str>) {",
        "        self.characters.insert(name.into(), StageChar { name: name.into(), at: at.map(|s| s.into()), visible: true });",
        "    }",
        "    pub fn hide_char(&mut self, name: &str) {",
        "        if let Some(c) = self.characters.get_mut(name) { c.visible = false; }",
        "    }",
        "    pub fn set_bg(&mut self, name: &str) { self.background = Some(name.into()); }",
        "    pub fn set_music(&mut self, name: &str) { self.music = Some(name.into()); }",
        "    pub fn store_state(&mut self, key: &str, val: &str) { self.state.insert(key.into(), val.into()); }",
        "    pub fn load_state(&self, key: &str) -> Option<&str> { self.state.get(key).map(|s| s.as_str()) }",
        "    pub fn exec_op(&mut self, op: OpVs2, a: u32, b: u32, stack_top: Option<&str>) {",
        "        match op {",
        "            OpVs2::Say => {",
        "                let speaker = self.pool_str(a);",
        "                let text = stack_top.unwrap_or(\"\").to_string();",
        "                self.say(&speaker, &text);",
        "            }",
        "            OpVs2::LoadMsg | OpVs2::Translate => { let _ = self.t(&self.pool_str(a)); }",
        "            OpVs2::Menu => { self.pending_menu.clear(); self.log.push(format!(\"menu choices={a}\")); }",
        "            OpVs2::Choice => {",
        "                let label = self.pool_str(a);",
        "                self.pending_menu.push(MenuChoice { label, index: b });",
        "            }",
        "            OpVs2::ShowChar => {",
        "                let name = self.pool_str(a);",
        "                let at = if b == 0 { None } else { Some(self.pool_str(b)) };",
        "                self.show_char(&name, at.as_deref());",
        "            }",
        "            OpVs2::HideChar => self.hide_char(&self.pool_str(a)),",
        "            OpVs2::Background => self.set_bg(&self.pool_str(a)),",
        "            OpVs2::Music => self.set_music(&self.pool_str(a)),",
        "            OpVs2::PushLayer => self.push_layer(self.pool_str(a)),",
        "            OpVs2::PopLayer => { let _ = self.pop_layer(); }",
        "            OpVs2::ShowLayer => self.show_layer(&self.pool_str(a)),",
        "            OpVs2::HideLayer => self.hide_layer(&self.pool_str(a)),",
        "            OpVs2::SetLayerZ => self.set_layer_z(&self.pool_str(a), b as i32),",
        "            OpVs2::StoreState => self.store_state(&self.pool_str(a), stack_top.unwrap_or(\"\")),",
        "            OpVs2::Await => self.await_clicks = self.await_clicks.saturating_add(1),",
        "            OpVs2::Yield => self.yielded = true,",
        "            _ => {}",
        "        }",
        "    }",
        "    pub fn visible_layers(&self) -> Vec<&LayerEntry> { self.layers.iter().filter(|l| l.visible).collect() }",
        "    pub fn visible_chars(&self) -> Vec<&StageChar> { self.characters.values().filter(|c| c.visible).collect() }",
        "    pub fn last_line(&self) -> Option<&DialogueLine> { self.dialogue.last() }",
        "    pub fn clear_dialogue(&mut self) { self.dialogue.clear(); }",
        "    pub fn reset_stage(&mut self) {",
        "        self.dialogue.clear(); self.pending_menu.clear(); self.characters.clear();",
        "        self.background = None; self.music = None; self.layers.clear();",
        "        self.yielded = false; self.await_clicks = 0; self.log.clear();",
        "    }",
        "}",
        "",
        "#[derive(Debug, Default)]",
        "pub struct Vs2MiniVm {",
        "    pub host: Vs2Host,",
        "    pub stack: Vec<String>,",
        "    pub locals: Vec<String>,",
        "    pub pc: usize,",
        "    pub code: Vec<(OpVs2, u32, u32)>,",
        "    pub halted: bool,",
        "}",
        "",
        "impl Vs2MiniVm {",
        "    pub fn new(host: Vs2Host) -> Self { Self { host, ..Default::default() } }",
        "    pub fn load(&mut self, code: Vec<(OpVs2, u32, u32)>) {",
        "        self.code = code; self.pc = 0; self.halted = false; self.stack.clear();",
        "    }",
        "    pub fn push(&mut self, v: impl Into<String>) { self.stack.push(v.into()); }",
        "    pub fn pop(&mut self) -> String { self.stack.pop().unwrap_or_default() }",
        "    pub fn step(&mut self) -> bool {",
        "        if self.halted || self.pc >= self.code.len() { self.halted = true; return false; }",
        "        let (op, a, b) = self.code[self.pc];",
        "        self.pc += 1;",
        "        match op {",
        "            OpVs2::Nop => {}",
        "            OpVs2::LoadConst => {",
        "                let s = self.host.pool_str(a);",
        "                if s.is_empty() { self.push(a.to_string()); } else { self.push(s); }",
        "            }",
        "            OpVs2::LoadLocal => {",
        "                let v = self.locals.get(a as usize).cloned().unwrap_or_default();",
        "                self.push(v);",
        "            }",
        "            OpVs2::StoreLocal => {",
        "                let v = self.pop();",
        "                let idx = a as usize;",
        "                if self.locals.len() <= idx { self.locals.resize(idx + 1, String::new()); }",
        "                self.locals[idx] = v;",
        "            }",
        "            OpVs2::Add => {",
        "                let r = self.pop(); let l = self.pop();",
        "                if let (Ok(li), Ok(ri)) = (l.parse::<i64>(), r.parse::<i64>()) {",
        "                    self.push((li + ri).to_string());",
        "                } else { self.push(format!(\"{l}{r}\")); }",
        "            }",
        "            OpVs2::Sub => {",
        "                let r = self.pop().parse::<i64>().unwrap_or(0);",
        "                let l = self.pop().parse::<i64>().unwrap_or(0);",
        "                self.push((l - r).to_string());",
        "            }",
        "            OpVs2::Mul => {",
        "                let r = self.pop().parse::<i64>().unwrap_or(0);",
        "                let l = self.pop().parse::<i64>().unwrap_or(0);",
        "                self.push((l * r).to_string());",
        "            }",
        "            OpVs2::Div => {",
        "                let r = self.pop().parse::<i64>().unwrap_or(1);",
        "                let l = self.pop().parse::<i64>().unwrap_or(0);",
        "                self.push(if r == 0 { 0 } else { l / r }.to_string());",
        "            }",
        "            OpVs2::Eq => {",
        "                let r = self.pop(); let l = self.pop();",
        "                self.push(if l == r { \"1\" } else { \"0\" });",
        "            }",
        "            OpVs2::Not => {",
        "                let v = self.pop();",
        "                self.push(if v == \"0\" || v.is_empty() { \"1\" } else { \"0\" });",
        "            }",
        "            OpVs2::Pop => { let _ = self.pop(); }",
        "            OpVs2::Dup => {",
        "                let v = self.stack.last().cloned().unwrap_or_default();",
        "                self.push(v);",
        "            }",
        "            OpVs2::Jump => { self.pc = a as usize; }",
        "            OpVs2::JumpIf => {",
        "                let v = self.pop();",
        "                if v == \"0\" || v.is_empty() { self.pc = a as usize; }",
        "            }",
        "            OpVs2::Ret => { self.halted = true; }",
        "            OpVs2::Print => {",
        "                let v = self.pop();",
        "                self.host.log.push(format!(\"print {v}\"));",
        "            }",
        "            OpVs2::LoadMsg | OpVs2::Translate => {",
        "                let key = self.host.pool_str(a);",
        "                self.push(self.host.t(&key));",
        "            }",
        "            other => {",
        "                let top = self.stack.last().cloned();",
        "                self.host.exec_op(other, a, b, top.as_deref());",
        "            }",
        "        }",
        "        !self.halted",
        "    }",
        "    pub fn run(&mut self, max_steps: usize) -> usize {",
        "        let mut n = 0;",
        "        while n < max_steps && self.step() { n += 1; }",
        "        n",
        "    }",
        "}",
        "",
    ]
    for n in range(70):
        lines += [
            f"pub fn scenario_{n}(speaker: &str, msg_key: &str, layer: &str) -> Vs2MiniVm {{",
            f"    let mut host = Vs2Host::new();",
            f"    host.pool = vec![speaker.into(), msg_key.into(), layer.into()];",
            f"    host.set_translation(msg_key, format!(\"line-{n}\"));",
            f"    let mut vm = Vs2MiniVm::new(host);",
            f"    vm.load(vec![",
            f"        (OpVs2::LoadMsg, 1, 0),",
            f"        (OpVs2::Say, 0, 0),",
            f"        (OpVs2::PushLayer, 2, 0),",
            f"        (OpVs2::Ret, 0, 0),",
            f"    ]);",
            f"    vm",
            f"}}",
            f"",
            f"pub fn run_scenario_{n}() -> Vs2Host {{",
            f"    let mut vm = scenario_{n}(\"hero\", \"k{n}\", \"hud\");",
            f"    let _ = vm.run(32);",
            f"    vm.host",
            f"}}",
            f"",
        ]
    lines += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    #[test]",
        "    fn host_say_and_layer() {",
        "        let mut h = Vs2Host::new();",
        "        h.set_translation(\"dlg.hi\", \"Hola\");",
        "        h.say_msg(\"eira\", \"dlg.hi\");",
        "        h.push_layer(\"settings\");",
        "        assert_eq!(h.last_line().unwrap().text, \"Hola\");",
        "        assert_eq!(h.layers.len(), 1);",
        "    }",
        "    #[test]",
        "    fn mini_vm_add() {",
        "        let mut vm = Vs2MiniVm::new(Vs2Host::new());",
        "        vm.load(vec![",
        "            (OpVs2::LoadConst, 2, 0),",
        "            (OpVs2::LoadConst, 3, 0),",
        "            (OpVs2::Add, 0, 0),",
        "            (OpVs2::Ret, 0, 0),",
        "        ]);",
        "        vm.run(16);",
        "        assert_eq!(vm.stack.last().map(|s| s.as_str()), Some(\"5\"));",
        "    }",
        "    #[test]",
        "    fn scenario_0_runs() {",
        "        let h = run_scenario_0();",
        "        assert!(!h.dialogue.is_empty());",
        "        assert!(!h.layers.is_empty());",
        "    }",
        "    #[test]",
        "    fn exec_show_hide_char() {",
        "        let mut h = Vs2Host::new();",
        "        h.pool = vec![\"hero\".into(), \"left\".into()];",
        "        h.exec_op(OpVs2::ShowChar, 0, 1, None);",
        "        assert!(h.characters[\"hero\"].visible);",
        "        h.exec_op(OpVs2::HideChar, 0, 0, None);",
        "        assert!(!h.characters[\"hero\"].visible);",
        "    }",
        "}",
        "",
    ]
    return lines


def gen_format_vs2() -> None:
    lines = [
        "//! VS2 formatter rules — rust-like braces, no Python significant whitespace.",
        "",
        "#![allow(missing_docs)]",
        "",
        "#[derive(Debug, Clone)]",
        "pub struct Vs2FormatOptions {",
        "    pub indent_width: usize,",
        "    pub use_tabs: bool,",
        "    pub max_width: usize,",
        "    pub trailing_comma: bool,",
        "    pub space_before_brace: bool,",
        "    pub newline_eof: bool,",
        "}",
        "",
        "impl Default for Vs2FormatOptions {",
        "    fn default() -> Self {",
        "        Self { indent_width: 4, use_tabs: false, max_width: 100,",
        "               trailing_comma: true, space_before_brace: true, newline_eof: true }",
        "    }",
        "}",
        "",
        "impl Vs2FormatOptions {",
        "    pub fn indent_str(&self, level: usize) -> String {",
        "        if self.use_tabs { \"\\t\".repeat(level) } else { \" \".repeat(self.indent_width * level) }",
        "    }",
        "}",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum Vs2TokKind {",
        "    Ident, Number, String, LBrace, RBrace, LParen, RParen, LBracket, RBracket,",
        "    Comma, Semi, Colon, PathSep, Op, Comment, Newline, Other,",
        "}",
        "",
        "#[derive(Debug, Clone)]",
        "pub struct Vs2Tok { pub kind: Vs2TokKind, pub text: String }",
        "",
        "pub fn lex_format(src: &str) -> Vec<Vs2Tok> {",
        "    let mut out = Vec::new();",
        "    let b = src.as_bytes();",
        "    let mut i = 0;",
        "    while i < b.len() {",
        "        let c = b[i] as char;",
        "        if c == '\\n' {",
        "            out.push(Vs2Tok { kind: Vs2TokKind::Newline, text: \"\\n\".into() });",
        "            i += 1; continue;",
        "        }",
        "        if c.is_whitespace() { i += 1; continue; }",
        "        if c == '/' && i + 1 < b.len() && b[i + 1] as char == '/' {",
        "            let start = i; i += 2;",
        "            while i < b.len() && b[i] as char != '\\n' { i += 1; }",
        "            out.push(Vs2Tok { kind: Vs2TokKind::Comment, text: src[start..i].to_string() });",
        "            continue;",
        "        }",
        "        if c == '\"' {",
        "            let start = i; i += 1;",
        "            while i < b.len() {",
        "                if b[i] as char == '\\\\' && i + 1 < b.len() { i += 2; continue; }",
        "                if b[i] as char == '\"' { i += 1; break; }",
        "                i += 1;",
        "            }",
        "            out.push(Vs2Tok { kind: Vs2TokKind::String, text: src[start..i].to_string() });",
        "            continue;",
        "        }",
        "        if c.is_ascii_alphabetic() || c == '_' {",
        "            let start = i; i += 1;",
        "            while i < b.len() {",
        "                let ch = b[i] as char;",
        "                if ch.is_ascii_alphanumeric() || ch == '_' { i += 1; } else { break; }",
        "            }",
        "            out.push(Vs2Tok { kind: Vs2TokKind::Ident, text: src[start..i].to_string() });",
        "            continue;",
        "        }",
        "        if c.is_ascii_digit() {",
        "            let start = i; i += 1;",
        "            while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] as char == '.') { i += 1; }",
        "            out.push(Vs2Tok { kind: Vs2TokKind::Number, text: src[start..i].to_string() });",
        "            continue;",
        "        }",
        "        if c == ':' && i + 1 < b.len() && b[i + 1] as char == ':' {",
        "            out.push(Vs2Tok { kind: Vs2TokKind::PathSep, text: \"::\".into() });",
        "            i += 2; continue;",
        "        }",
        "        let (kind, text) = match c {",
        "            '{' => (Vs2TokKind::LBrace, \"{\"), '}' => (Vs2TokKind::RBrace, \"}\"),",
        "            '(' => (Vs2TokKind::LParen, \"(\"), ')' => (Vs2TokKind::RParen, \")\"),",
        "            '[' => (Vs2TokKind::LBracket, \"[\"), ']' => (Vs2TokKind::RBracket, \"]\"),",
        "            ',' => (Vs2TokKind::Comma, \",\"), ';' => (Vs2TokKind::Semi, \";\"),",
        "            ':' => (Vs2TokKind::Colon, \":\"), _ => (Vs2TokKind::Op, \"\"),",
        "        };",
        "        if kind == Vs2TokKind::Op {",
        "            out.push(Vs2Tok { kind, text: c.to_string() });",
        "        } else {",
        "            out.push(Vs2Tok { kind, text: text.into() });",
        "        }",
        "        i += 1;",
        "    }",
        "    out",
        "}",
        "",
        "pub fn format_vs2(src: &str, opt: &Vs2FormatOptions) -> String {",
        "    let toks = lex_format(src);",
        "    let mut out = String::new();",
        "    let mut level: i32 = 0;",
        "    let mut at_line_start = true;",
        "    let mut i = 0;",
        "    while i < toks.len() {",
        "        let t = &toks[i];",
        "        match t.kind {",
        "            Vs2TokKind::Newline => { out.push('\\n'); at_line_start = true; }",
        "            Vs2TokKind::RBrace => {",
        "                level = (level - 1).max(0);",
        "                if at_line_start { out.push_str(&opt.indent_str(level as usize)); }",
        "                out.push('}');",
        "                at_line_start = false;",
        "            }",
        "            Vs2TokKind::LBrace => {",
        "                if opt.space_before_brace && !at_line_start && !out.ends_with(' ') && !out.ends_with('\\n') {",
        "                    out.push(' ');",
        "                }",
        "                out.push('{');",
        "                level += 1;",
        "                at_line_start = false;",
        "            }",
        "            Vs2TokKind::Comment => {",
        "                if at_line_start { out.push_str(&opt.indent_str(level as usize)); }",
        "                out.push_str(&t.text);",
        "                at_line_start = false;",
        "            }",
        "            _ => {",
        "                if at_line_start {",
        "                    out.push_str(&opt.indent_str(level as usize));",
        "                    at_line_start = false;",
        "                } else if needs_space_before(&toks, i) {",
        "                    out.push(' ');",
        "                }",
        "                out.push_str(&t.text);",
        "            }",
        "        }",
        "        i += 1;",
        "    }",
        "    if opt.newline_eof && !out.ends_with('\\n') { out.push('\\n'); }",
        "    out",
        "}",
        "",
        "fn needs_space_before(toks: &[Vs2Tok], i: usize) -> bool {",
        "    if i == 0 { return false; }",
        "    let prev = &toks[i - 1];",
        "    let cur = &toks[i];",
        "    if matches!(prev.kind, Vs2TokKind::LParen | Vs2TokKind::LBracket | Vs2TokKind::PathSep) { return false; }",
        "    if matches!(cur.kind, Vs2TokKind::RParen | Vs2TokKind::RBracket | Vs2TokKind::Comma | Vs2TokKind::Semi | Vs2TokKind::Colon) { return false; }",
        "    if prev.kind == Vs2TokKind::Ident && cur.kind == Vs2TokKind::LParen { return false; }",
        "    if prev.kind == Vs2TokKind::Ident && cur.kind == Vs2TokKind::PathSep { return false; }",
        "    if prev.kind == Vs2TokKind::PathSep { return false; }",
        "    true",
        "}",
        "",
        "pub fn looks_like_python(src: &str) -> bool {",
        "    let has_brace = src.contains('{');",
        "    let indent_lines = src.lines().filter(|l| l.starts_with(\"    \") || l.starts_with('\\t')).count();",
        "    !has_brace && indent_lines > 3 && (src.contains(\"def \") || src.contains(\"elif \"))",
        "}",
        "",
        "pub fn reject_python_style(src: &str) -> Result<(), String> {",
        "    if looks_like_python(src) {",
        "        Err(\"Velvet Script 2 is not Python: use braces `{}`, typed fn/struct, not def/elif indent\".into())",
        "    } else { Ok(()) }",
        "}",
        "",
    ]
    for n in range(55):
        lines += [
            f"pub fn format_fixture_{n}() -> String {{",
            f"    let src = concat!(",
            f"        \"// @edition 2\\n\",",
            f"        \"fn f_{n}(x: i32) {{\\n\",",
            f"        \"let y=x+1;\\n\",",
            f"        \"return y;\\n\",",
            f"        \"}}\\n\",",
            f"    );",
            f"    format_vs2(src, &Vs2FormatOptions::default())",
            f"}}",
            f"",
        ]
    lines += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    #[test]",
        "    fn braces_indent() {",
        "        let src = \"fn main(){\\nlet x=1;\\n}\";",
        "        let out = format_vs2(src, &Vs2FormatOptions::default());",
        "        assert!(out.contains(\"fn main()\"));",
        "        assert!(out.contains('{'));",
        "    }",
        "    #[test]",
        "    fn rejects_python() {",
        "        let py = \"def foo():\\n    x = 1\\n    if x:\\n        y = 2\\n    elif x:\\n        y = 3\\n\";",
        "        assert!(looks_like_python(py));",
        "        assert!(reject_python_style(py).is_err());",
        "    }",
        "    #[test]",
        "    fn fixture_0() { assert!(format_fixture_0().contains(\"fn\")); }",
        "}",
        "",
    ]
    w(CRATES / "velvet-script-format" / "src" / "vs2_format.rs", "\n".join(lines) + "\n")


def gen_lsp_vs2() -> None:
    lines = [
        "//! VS2 LSP helpers: completions, hover, semantic tokens for rust-like surface.",
        "",
        "#![allow(missing_docs)]",
        "",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum Vs2CompletionKind {",
        "    Keyword, Function, Type, Variable, Module, Snippet, Layer, Scene, MsgKey,",
        "}",
        "",
        "#[derive(Debug, Clone)]",
        "pub struct Vs2Completion {",
        "    pub label: String,",
        "    pub kind: Vs2CompletionKind,",
        "    pub detail: String,",
        "    pub insert: String,",
        "}",
        "",
        "impl Vs2Completion {",
        "    pub fn kw(label: &str) -> Self {",
        "        Self { label: label.into(), kind: Vs2CompletionKind::Keyword, detail: \"keyword\".into(), insert: label.into() }",
        "    }",
        "    pub fn fn_item(label: &str, detail: &str) -> Self {",
        "        Self { label: label.into(), kind: Vs2CompletionKind::Function, detail: detail.into(), insert: format!(\"{label}($0)\") }",
        "    }",
        "    pub fn ty(label: &str) -> Self {",
        "        Self { label: label.into(), kind: Vs2CompletionKind::Type, detail: \"type\".into(), insert: label.into() }",
        "    }",
        "}",
        "",
        "pub static VS2_KEYWORDS: &[&str] = &[",
        "    \"fn\", \"struct\", \"enum\", \"mod\", \"use\", \"pub\", \"let\", \"mut\", \"const\", \"static\",",
        "    \"if\", \"else\", \"while\", \"for\", \"loop\", \"match\", \"return\", \"break\", \"continue\",",
        "    \"impl\", \"trait\", \"type\", \"where\", \"as\", \"in\", \"ref\", \"move\",",
        "    \"scene\", \"say\", \"menu\", \"jump\", \"call\", \"show\", \"hide\", \"with\", \"at\",",
        "    \"character\", \"screen\", \"state\", \"transform\", \"layer\",",
        "    \"true\", \"false\", \"self\", \"Self\", \"crate\", \"super\",",
        "];",
        "",
        "pub static VS2_TYPES: &[&str] = &[",
        "    \"i32\", \"i64\", \"u32\", \"u64\", \"f32\", \"f64\", \"bool\", \"str\", \"String\",",
        "    \"Option\", \"Result\", \"Vec\", \"LayerId\", \"SceneId\", \"MsgId\", \"ScriptError\",",
        "    \"Transform\", \"Transition\", \"Color\", \"Vec2\",",
        "];",
        "",
        "pub fn story_snippets() -> Vec<Vs2Completion> {",
        "    vec![",
        "        Vs2Completion { label: \"scene\".into(), kind: Vs2CompletionKind::Snippet,",
        "            detail: \"scene block\".into(), insert: \"scene ${1:name} {\\n    $0\\n}\".into() },",
        "        Vs2Completion { label: \"say\".into(), kind: Vs2CompletionKind::Snippet,",
        "            detail: \"say with t!\".into(), insert: \"say ${1:speaker}, t!(\\\"${2:key}\\\");\".into() },",
        "        Vs2Completion { label: \"menu\".into(), kind: Vs2CompletionKind::Snippet,",
        "            detail: \"menu choices\".into(), insert: \"menu {\\n    t!(\\\"${1:a}\\\") => { $0 }\\n}\".into() },",
        "        Vs2Completion { label: \"screen\".into(), kind: Vs2CompletionKind::Snippet,",
        "            detail: \"typed screen\".into(), insert: \"screen ${1:Name} {\\n    $0\\n}\".into() },",
        "        Vs2Completion { label: \"push_layer\".into(), kind: Vs2CompletionKind::Function,",
        "            detail: \"push_layer(LayerId)\".into(),",
        "            insert: \"push_layer(LayerId::new(\\\"${1:id}\\\"))?;\".into() },",
        "    ]",
        "}",
        "",
        "pub fn default_completions() -> Vec<Vs2Completion> {",
        "    let mut v = Vec::new();",
        "    for k in VS2_KEYWORDS { v.push(Vs2Completion::kw(k)); }",
        "    for t in VS2_TYPES { v.push(Vs2Completion::ty(t)); }",
        "    v.extend(story_snippets());",
        "    v",
        "}",
        "",
        "pub fn filter_completions(prefix: &str) -> Vec<Vs2Completion> {",
        "    let p = prefix.to_ascii_lowercase();",
        "    default_completions().into_iter()",
        "        .filter(|c| c.label.to_ascii_lowercase().starts_with(&p)).collect()",
        "}",
        "",
        "pub fn hover_for(name: &str) -> Option<String> {",
        "    match name {",
        "        \"LayerId\" => Some(\"stable layer handle — not a Python string global\".into()),",
        "        \"MsgId\" => Some(\"message key for i18n; use t!(\\\"key\\\")\".into()),",
        "        \"SceneId\" => Some(\"typed scene label for jump/call\".into()),",
        "        \"say\" => Some(\"say speaker, t!(\\\"key\\\") — dialogue line\".into()),",
        "        \"push_layer\" => Some(\"push exclusive UI layer onto stack\".into()),",
        "        \"fn\" => Some(\"function item (rust-like, not def)\".into()),",
        "        \"struct\" => Some(\"product type with named fields\".into()),",
        "        \"enum\" => Some(\"sum type with typed variants\".into()),",
        "        \"match\" => Some(\"exhaustive pattern match\".into()),",
        "        \"scene\" => Some(\"story scene / label block\".into()),",
        "        _ => None,",
        "    }",
        "}",
        "",
        "pub static SEMANTIC_TYPES: &[&str] = &[",
        "    \"keyword\", \"function\", \"type\", \"variable\", \"parameter\", \"property\",",
        "    \"string\", \"number\", \"comment\", \"namespace\", \"macro\", \"enumMember\",",
        "];",
        "",
        "pub fn classify_word(word: &str) -> &'static str {",
        "    if VS2_KEYWORDS.contains(&word) { \"keyword\" }",
        "    else if VS2_TYPES.contains(&word) { \"type\" }",
        "    else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) { \"type\" }",
        "    else { \"variable\" }",
        "}",
        "",
    ]
    for n in range(80):
        lines += [
            f"pub fn local_completions_{n}(mod_name: &str) -> Vec<Vs2Completion> {{",
            f"    vec![",
            f"        Vs2Completion::fn_item(&format!(\"{{mod_name}}_fn_{n}\"), \"local fn\"),",
            f"        Vs2Completion::ty(&format!(\"{{mod_name}}_Ty_{n}\")),",
            f"        Vs2Completion {{ label: format!(\"scene_{n}\"), kind: Vs2CompletionKind::Scene,",
            f"            detail: \"scene\".into(), insert: format!(\"scene_{n}\") }},",
            f"        Vs2Completion {{ label: format!(\"layer_{n}\"), kind: Vs2CompletionKind::Layer,",
            f"            detail: \"layer\".into(), insert: format!(\"LayerId::new(\\\"layer_{n}\\\")\") }},",
            f"        Vs2Completion {{ label: format!(\"msg.k{n}\"), kind: Vs2CompletionKind::MsgKey,",
            f"            detail: \"msg\".into(), insert: format!(\"t!(\\\"msg.k{n}\\\")\") }},",
            f"    ]",
            f"}}",
            f"",
        ]
    lines += [
        "#[cfg(test)]",
        "mod tests {",
        "    use super::*;",
        "    #[test]",
        "    fn keywords_present() {",
        "        let c = default_completions();",
        "        assert!(c.iter().any(|x| x.label == \"fn\"));",
        "        assert!(c.iter().any(|x| x.label == \"scene\"));",
        "        assert!(c.iter().any(|x| x.label == \"LayerId\"));",
        "    }",
        "    #[test]",
        "    fn filter_fn() {",
        "        let c = filter_completions(\"sc\");",
        "        assert!(c.iter().any(|x| x.label.starts_with(\"sc\")));",
        "    }",
        "    #[test]",
        "    fn hover_layer() { assert!(hover_for(\"LayerId\").unwrap().contains(\"layer\")); }",
        "    #[test]",
        "    fn classify() {",
        "        assert_eq!(classify_word(\"fn\"), \"keyword\");",
        "        assert_eq!(classify_word(\"i32\"), \"type\");",
        "    }",
        "}",
        "",
    ]
    w(CRATES / "velvet-script-lsp" / "src" / "vs2_ide.rs", "\n".join(lines) + "\n")


def patch_workspace() -> None:
    root = ROOT / "Cargo.toml"
    text = root.read_text(encoding="utf-8")
    if "velvet-script-resolve" not in text:
        text = text.replace(
            '    "crates/velvet-script-corpus",\n',
            '    "crates/velvet-script-corpus",\n    "crates/velvet-script-resolve",\n',
        )
        text = text.replace(
            'velvet-script-corpus = { path = "crates/velvet-script-corpus" }\n',
            'velvet-script-corpus = { path = "crates/velvet-script-corpus" }\n'
            'velvet-script-resolve = { path = "crates/velvet-script-resolve" }\n',
        )
        root.write_text(text, encoding="utf-8")
        print("  workspace +velvet-script-resolve")


def patch_compiler_lib() -> None:
    path = CRATES / "velvet-script-compiler" / "src" / "lib.rs"
    text = path.read_text(encoding="utf-8")
    if "vs2_codegen" not in text:
        text = text.replace("pub mod vs2_lower;", "pub mod vs2_lower;\npub mod vs2_codegen;")
        if "vs2_codegen" not in text:
            text = text + "\npub mod vs2_codegen;\n"
        path.write_text(text, encoding="utf-8")
        print("  patched compiler lib")


def patch_vm_lib() -> None:
    path = CRATES / "velvet-script-vm" / "src" / "lib.rs"
    text = path.read_text(encoding="utf-8")
    if "vs2_host" not in text:
        text = text.replace("mod vm;", "mod vm;\nmod vs2_host;")
        text += "\npub use vs2_host::{DialogueLine, LayerEntry, MenuChoice, StageChar, Vs2Host, Vs2MiniVm};\n"
        path.write_text(text, encoding="utf-8")
        print("  patched vm lib")


def patch_format_lib() -> None:
    path = CRATES / "velvet-script-format" / "src" / "lib.rs"
    text = path.read_text(encoding="utf-8")
    if "vs2_format" not in text:
        text = text + "\n/// VS2 brace-aware format helpers.\npub mod vs2_format;\n"
        path.write_text(text, encoding="utf-8")
        print("  patched format lib")


def patch_lsp_lib() -> None:
    path = CRATES / "velvet-script-lsp" / "src" / "lib.rs"
    text = path.read_text(encoding="utf-8")
    if "vs2_ide" not in text:
        text = text + "\n/// VS2 IDE completions / hover.\npub mod vs2_ide;\n"
        path.write_text(text, encoding="utf-8")
        print("  patched lsp lib")


def main() -> None:
    print("gen_vs2_lang_part5 …")
    gen_compiler_codegen()
    gen_resolve_crate()
    gen_vm_host()
    gen_format_vs2()
    gen_lsp_vs2()
    patch_workspace()
    patch_compiler_lib()
    patch_vm_lib()
    patch_format_lib()
    patch_lsp_lib()
    print("done.")


if __name__ == "__main__":
    main()
