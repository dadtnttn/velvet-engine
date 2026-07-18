//! Lower Velvet Story AST → VS2 HIR + OpVs2 unit (no second VM).

use std::collections::HashMap;

use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_compiler::vs2_codegen::{
    lower_module as hir_to_unit, Vs2Instr, Vs2Unit,
};
use velvet_script_hir::{
    HirExpr, HirId, HirItem, HirLit, HirModule, HirPath, HirScene, HirSpan, HirStmt, PathSeg,
};

use crate::ast::*;
use crate::diag::{adapt_internal, StoryDiag};
use crate::source_map::SourceMap;
use crate::span::Span;

/// Lowering output.
#[derive(Debug)]
pub struct LowerOutput {
    /// HIR module (story-shaped items).
    pub hir: HirModule,
    /// Executable VS2 unit (primary).
    pub unit: Vs2Unit,
    /// Source map.
    pub map: SourceMap,
    /// Diagnostics from lowering.
    pub diags: Vec<StoryDiag>,
    /// Stable msg ids emitted.
    pub msg_ids: Vec<(String, String)>,
}

/// Lower a validated story file to VS2 representations.
pub fn lower(file: &StoryFile) -> LowerOutput {
    let mut hir = HirModule::new(2);
    hir.file = Some(file.file.clone());
    let mut map = SourceMap::new(&file.file);
    let mut diags = Vec::new();
    let mut msg_ids = Vec::new();
    let mut unit = Vs2Unit::new(file.file.clone());
    let mut locals: HashMap<String, u32> = HashMap::new();
    let mut next_local = 0u32;

    let mut local = |name: &str, locals: &mut HashMap<String, u32>, next: &mut u32| -> u32 {
        if let Some(&i) = locals.get(name) {
            return i;
        }
        let i = *next;
        *next += 1;
        locals.insert(name.to_string(), i);
        i
    };

    for item in &file.items {
        let TopItem::Scene(sc) = item else {
            continue;
        };
        let entry = unit.pc();
        unit.entry_scenes.insert(sc.name.clone(), entry);
        let origin = sc
            .origin_file
            .as_deref()
            .unwrap_or(file.file.as_str());
        map.push_in_file(
            origin,
            sc.span,
            "scene",
            format!("scene {}", sc.name),
            Some(entry),
        );

        // also HIR scene shell
        let mut hir_body = Vec::new();
        let mut id = 1u32;

        for st in &sc.body {
            lower_stmt(
                st,
                &mut unit,
                &mut map,
                &mut diags,
                &mut msg_ids,
                &mut locals,
                &mut next_local,
                &mut local,
                &mut hir_body,
                &mut id,
                origin,
                &sc.name,
            );
        }
        unit.emit(Vs2Instr::new(OpVs2::Ret).at_line(sc.span.line));

        hir.items.push(HirItem::Scene(HirScene {
            id: HirId(id),
            name: sc.name.clone(),
            body: hir_body,
            span: to_hir_span(sc.span),
        }));
    }

    // link scene jumps
    velvet_script_compiler::vs2_codegen::link_scenes(&mut unit);
    unit.local_slots = next_local;

    // validate via existing helper
    if let Err(errs) = velvet_script_compiler::vs2_codegen::validate_unit(&unit) {
        for e in errs {
            diags.push(adapt_internal(&file.file, Span::unknown(), &e));
        }
    }

    // ensure HIR path also lowerable
    let _from_hir = hir_to_unit(&hir);
    let _ = _from_hir;

    LowerOutput {
        hir,
        unit,
        map,
        diags,
        msg_ids,
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_stmt(
    st: &Stmt,
    unit: &mut Vs2Unit,
    map: &mut SourceMap,
    diags: &mut Vec<StoryDiag>,
    msg_ids: &mut Vec<(String, String)>,
    locals: &mut HashMap<String, u32>,
    next_local: &mut u32,
    local: &mut dyn FnMut(&str, &mut HashMap<String, u32>, &mut u32) -> u32,
    hir_body: &mut Vec<HirStmt>,
    id: &mut u32,
    file: &str,
    scene: &str,
) {
    match st {
        Stmt::Background { id: bg, span } => {
            let pid = unit.pool.intern(bg.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::Background, pid).at_line(span.line));
            map.push_in_file(file, *span, "background", bg.clone(), Some(pc));
            hir_body.push(HirStmt::Background {
                path: bg.clone(),
                span: to_hir_span(*span),
            });
        }
        Stmt::Music { id: m, span } => {
            let pid = unit.pool.intern(m.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::Music, pid).at_line(span.line));
            map.push_in_file(file, *span, "music", m.clone(), Some(pc));
            hir_body.push(HirStmt::Music {
                path: m.clone(),
                fade_in: None,
                span: to_hir_span(*span),
            });
        }
        Stmt::Sound { id: s, span } => {
            let pid = unit.pool.intern(s.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::PlaySfx, pid).at_line(span.line));
            map.push_in_file(file, *span, "sound", s.clone(), Some(pc));
        }
        Stmt::Show {
            character,
            expression,
            at,
            span,
        } => {
            let cid = unit.pool.intern(character.as_str());
            let at_id = at
                .as_ref()
                .map(|a| unit.pool.intern(a.as_str()))
                .unwrap_or(0);
            let pc = unit.emit(Vs2Instr::with_ab(OpVs2::ShowChar, cid, at_id).at_line(span.line));
            map.push_in_file(file, *span, "show", character.clone(), Some(pc));
            hir_body.push(HirStmt::Show {
                character: character.clone(),
                expr: expression.clone(),
                at: at.clone(),
                span: to_hir_span(*span),
            });
        }
        Stmt::Hide { character, span } => {
            let cid = unit.pool.intern(character.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::HideChar, cid).at_line(span.line));
            map.push_in_file(file, *span, "hide", character.clone(), Some(pc));
            hir_body.push(HirStmt::Hide {
                character: character.clone(),
                span: to_hir_span(*span),
            });
        }
        Stmt::Dialogue {
            speaker,
            msg_id,
            text,
            span,
        } => {
            let mid = msg_id
                .clone()
                .unwrap_or_else(|| stable_msg_id(scene, speaker, text));
            msg_ids.push((mid.clone(), text.clone()));
            let sp = unit.pool.intern(speaker.as_str());
            let mk = unit.pool.intern(mid.as_str());
            // store translation in pool; host loads via LoadMsg
            unit.pool.intern(text.as_str()); // ensure text exists
            let pc_load = unit.emit(Vs2Instr::with_a(OpVs2::LoadMsg, mk).at_line(span.line));
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::Say, sp).at_line(span.line));
            map.push_in_file(file, *span, "dialogue", mid.clone(), Some(pc));
            let _ = pc_load;
            hir_body.push(HirStmt::Say {
                speaker: Some(speaker.clone()),
                msg: HirExpr::Lit {
                    lit: HirLit::MsgId(mid),
                    span: to_hir_span(*span),
                },
                span: to_hir_span(*span),
            });
        }
        Stmt::Goto { target, span } => {
            let tid = unit.pool.intern(target.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::JumpScene, tid).at_line(span.line));
            map.push_in_file(file, *span, "goto", target.clone(), Some(pc));
            hir_body.push(HirStmt::Jump {
                target: target.clone(),
                span: to_hir_span(*span),
            });
        }
        Stmt::CallScene { target, span } => {
            let tid = unit.pool.intern(target.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::CallScene, tid).at_line(span.line));
            map.push_in_file(file, *span, "call_scene", target.clone(), Some(pc));
            hir_body.push(HirStmt::CallScene {
                target: target.clone(),
                span: to_hir_span(*span),
            });
        }
        Stmt::Return { span } | Stmt::End { span } => {
            let pc = unit.emit(Vs2Instr::new(OpVs2::Ret).at_line(span.line));
            map.push_in_file(file, *span, "end", "ret", Some(pc));
            hir_body.push(HirStmt::Return {
                value: None,
                span: to_hir_span(*span),
            });
        }
        Stmt::Set { name, value, span } => {
            emit_expr(value, unit, locals, next_local, local);
            let slot = local(name, locals, next_local);
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot).at_line(span.line));
            // also store state by name for host
            let kid = unit.pool.intern(name.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
            unit.emit(Vs2Instr::with_a(OpVs2::StoreState, kid));
            map.push_in_file(file, *span, "set", name.clone(), Some(pc));
            hir_body.push(HirStmt::Let {
                name: name.clone(),
                mutable: true,
                ty: None,
                init: Some(expr_to_hir(value)),
                span: to_hir_span(*span),
            });
        }
        Stmt::Add { name, value, span } => {
            let slot = local(name, locals, next_local);
            unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot).at_line(span.line));
            emit_expr(value, unit, locals, next_local, local);
            unit.emit(Vs2Instr::new(OpVs2::Add).at_line(span.line));
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot).at_line(span.line));
            let kid = unit.pool.intern(name.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot));
            unit.emit(Vs2Instr::with_a(OpVs2::StoreState, kid));
            map.push_in_file(file, *span, "add", name.clone(), Some(pc));
        }
        Stmt::Sub { name, value, span } => {
            let slot = local(name, locals, next_local);
            unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot).at_line(span.line));
            emit_expr(value, unit, locals, next_local, local);
            unit.emit(Vs2Instr::new(OpVs2::Sub).at_line(span.line));
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, slot).at_line(span.line));
            map.push_in_file(file, *span, "sub", name.clone(), Some(pc));
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
            span,
        } => {
            emit_expr(cond, unit, locals, next_local, local);
            // JumpIf jumps when falsy
            let j_else = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0).at_line(span.line));
            map.push_in_file(file, *span, "if", "cond", Some(j_else));
            for s in then_body {
                lower_stmt(
                    s, unit, map, diags, msg_ids, locals, next_local, local, hir_body, id, file,
                    scene,
                );
            }
            let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0).at_line(span.line));
            let else_pc = unit.pc();
            unit.patch_a(j_else, else_pc);
            if let Some(eb) = else_body {
                for s in eb {
                    lower_stmt(
                        s, unit, map, diags, msg_ids, locals, next_local, local, hir_body, id,
                        file, scene,
                    );
                }
            }
            let end = unit.pc();
            unit.patch_a(j_end, end);
        }
        Stmt::Choice { options, span } => {
            let pc = unit.emit(
                Vs2Instr::with_a(OpVs2::Menu, options.len() as u32).at_line(span.line),
            );
            map.push_in_file(file, *span, "choice", "menu", Some(pc));
            // Selected index lives in host state `__choice` (set by runner / UI).
            let choice_key = unit.pool.intern("__choice");
            let choice_slot = local("__choice", locals, next_local);
            // LoadState is host-side; also keep a local the runner can StoreLocal.
            unit.emit(Vs2Instr::with_a(OpVs2::LoadState, choice_key).at_line(span.line));
            unit.emit(Vs2Instr::with_a(OpVs2::StoreLocal, choice_slot).at_line(span.line));

            let mut end_jumps: Vec<u32> = Vec::new();
            for (i, opt) in options.iter().enumerate() {
                let mid = opt
                    .msg_id
                    .clone()
                    .unwrap_or_else(|| stable_msg_id(scene, "choice", &opt.label));
                msg_ids.push((mid.clone(), opt.label.clone()));
                let lid = unit.pool.intern(opt.label.as_str());
                unit.emit(Vs2Instr::with_ab(OpVs2::Choice, lid, i as u32).at_line(opt.span.line));

                // if __choice != i → skip body
                unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, choice_slot).at_line(opt.span.line));
                let idx = unit.pool.intern(i.to_string());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, idx).at_line(opt.span.line));
                unit.emit(Vs2Instr::new(OpVs2::Eq).at_line(opt.span.line));
                let j_skip = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0).at_line(opt.span.line));
                for s in &opt.body {
                    lower_stmt(
                        s, unit, map, diags, msg_ids, locals, next_local, local, hir_body, id,
                        file, scene,
                    );
                }
                let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0).at_line(opt.span.line));
                end_jumps.push(j_end);
                let skip_pc = unit.pc();
                unit.patch_a(j_skip, skip_pc);
            }
            let end_pc = unit.pc();
            for j in end_jumps {
                unit.patch_a(j, end_pc);
            }
        }
        Stmt::CallCommand { name, args, span } => {
            // Encode as single Call (host logs command name + args). Do not emit
            // ActionFire after Call — that would re-pop stack arguments.
            // Bare idents in kwargs are asset/keyword literals (LoadConst), unless
            // the name was previously assigned with `set`/`add` (then LoadLocal).
            let cid = unit.pool.intern(name.as_str());
            for (k, v) in args {
                let _ = unit.pool.intern(k.as_str());
                emit_cmd_arg(v, unit, locals, next_local, local);
            }
            let pc = unit.emit(
                Vs2Instr::with_ab(OpVs2::Call, cid, args.len() as u32).at_line(span.line),
            );
            map.push_in_file(file, *span, "call", name.clone(), Some(pc));
        }
        Stmt::Pause { span, .. } => {
            let pc = unit.emit(Vs2Instr::new(OpVs2::Await).at_line(span.line));
            map.push_in_file(file, *span, "pause", "await", Some(pc));
        }
        Stmt::Transition { name, span } => {
            let tid = unit.pool.intern(name.as_str());
            let pc = unit.emit(Vs2Instr::with_a(OpVs2::TransitionPlay, tid).at_line(span.line));
            map.push_in_file(file, *span, "transition", name.clone(), Some(pc));
        }
        Stmt::Label { name, span } => {
            map.push_in_file(file, *span, "label", name.clone(), Some(unit.pc()));
        }
        Stmt::Comment { .. } => {}
    }
    *id += 1;
}

/// Emit a command kwarg value.
/// Bare `Ident` is a literal asset/id name unless already assigned as a variable.
fn emit_cmd_arg(
    e: &Expr,
    unit: &mut Vs2Unit,
    locals: &mut HashMap<String, u32>,
    next_local: &mut u32,
    local: &mut dyn FnMut(&str, &mut HashMap<String, u32>, &mut u32) -> u32,
) {
    match e {
        Expr::Ident(name, span) => {
            if let Some(&slot) = locals.get(name) {
                unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot).at_line(span.line));
            } else {
                let id = unit.pool.intern(name.as_str());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id).at_line(span.line));
            }
        }
        other => emit_expr(other, unit, locals, next_local, local),
    }
}

fn emit_expr(
    e: &Expr,
    unit: &mut Vs2Unit,
    locals: &mut HashMap<String, u32>,
    next_local: &mut u32,
    local: &mut dyn FnMut(&str, &mut HashMap<String, u32>, &mut u32) -> u32,
) {
    match e {
        Expr::Int(n, span) => {
            // Intern numeric literals as pool strings so Vs2MiniVm does not
            // confuse small integers with string pool indices.
            let id = unit.pool.intern(n.to_string());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id).at_line(span.line));
        }
        Expr::Float(s, span) => {
            let id = unit.pool.intern(s.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id).at_line(span.line));
        }
        Expr::Bool(b, span) => {
            let id = unit.pool.intern(if *b { "1" } else { "0" });
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id).at_line(span.line));
        }
        Expr::Str(s, span) => {
            let id = unit.pool.intern(s.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id).at_line(span.line));
        }
        Expr::Ident(name, span) => {
            let slot = local(name, locals, next_local);
            unit.emit(Vs2Instr::with_a(OpVs2::LoadLocal, slot).at_line(span.line));
        }
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => {
            emit_expr(left, unit, locals, next_local, local);
            emit_expr(right, unit, locals, next_local, local);
            let opc = match op {
                BinOp::Add => OpVs2::Add,
                BinOp::Sub => OpVs2::Sub,
                BinOp::Mul => OpVs2::Mul,
                BinOp::Div => OpVs2::Div,
                BinOp::Eq => OpVs2::Eq,
                BinOp::Ne => OpVs2::Ne,
                BinOp::Lt => OpVs2::Lt,
                BinOp::Le => OpVs2::Le,
                BinOp::Gt => OpVs2::Gt,
                BinOp::Ge => OpVs2::Ge,
                BinOp::And => OpVs2::And,
                BinOp::Or => OpVs2::Or,
            };
            unit.emit(Vs2Instr::new(opc).at_line(span.line));
        }
        Expr::Unary { op, expr, span } => {
            match op {
                UnaryOp::Not => {
                    emit_expr(expr, unit, locals, next_local, local);
                    unit.emit(Vs2Instr::new(OpVs2::Not).at_line(span.line));
                }
                UnaryOp::Neg => {
                    // Fold literals: -5 → const -5 (as string for host LoadConst).
                    if let Expr::Int(n, _) = expr.as_ref() {
                        let id = unit.pool.intern((-n).to_string());
                        unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id).at_line(span.line));
                    } else {
                        // Stack convention: Sub pops r then l → l - r. Emit 0, x, Sub → 0 - x.
                        let z = unit.pool.intern("0");
                        unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, z).at_line(span.line));
                        emit_expr(expr, unit, locals, next_local, local);
                        unit.emit(Vs2Instr::new(OpVs2::Sub).at_line(span.line));
                    }
                }
            }
        }
    }
}

fn expr_to_hir(e: &Expr) -> HirExpr {
    match e {
        Expr::Int(n, span) => HirExpr::Lit {
            lit: HirLit::Int(*n),
            span: to_hir_span(*span),
        },
        Expr::Float(s, span) => HirExpr::Lit {
            lit: HirLit::Float(s.parse().unwrap_or(0.0)),
            span: to_hir_span(*span),
        },
        Expr::Bool(b, span) => HirExpr::Lit {
            lit: HirLit::Bool(*b),
            span: to_hir_span(*span),
        },
        Expr::Str(s, span) => HirExpr::Lit {
            lit: HirLit::Str(s.clone()),
            span: to_hir_span(*span),
        },
        Expr::Ident(name, span) => HirExpr::Path {
            path: HirPath {
                segs: vec![PathSeg(name.clone())],
            },
            span: to_hir_span(*span),
        },
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => HirExpr::Binary {
            op: match op {
                BinOp::Add => velvet_script_hir::HirBinOp::Add,
                BinOp::Sub => velvet_script_hir::HirBinOp::Sub,
                BinOp::Mul => velvet_script_hir::HirBinOp::Mul,
                BinOp::Div => velvet_script_hir::HirBinOp::Div,
                BinOp::Eq => velvet_script_hir::HirBinOp::Eq,
                BinOp::Ne => velvet_script_hir::HirBinOp::Ne,
                BinOp::Lt => velvet_script_hir::HirBinOp::Lt,
                BinOp::Le => velvet_script_hir::HirBinOp::Le,
                BinOp::Gt => velvet_script_hir::HirBinOp::Gt,
                BinOp::Ge => velvet_script_hir::HirBinOp::Ge,
                BinOp::And => velvet_script_hir::HirBinOp::And,
                BinOp::Or => velvet_script_hir::HirBinOp::Or,
            },
            lhs: Box::new(expr_to_hir(left)),
            rhs: Box::new(expr_to_hir(right)),
            span: to_hir_span(*span),
        },
        Expr::Unary { expr, span, .. } => expr_to_hir(expr).clone_with_span(*span),
    }
}

trait CloneSpan {
    fn clone_with_span(&self, span: Span) -> HirExpr;
}

impl CloneSpan for HirExpr {
    fn clone_with_span(&self, _span: Span) -> HirExpr {
        self.clone()
    }
}

fn to_hir_span(s: Span) -> HirSpan {
    HirSpan::at(s.line, s.column, s.start, s.end)
}

/// Stable message id from content (not line numbers).
pub fn stable_msg_id(scene: &str, speaker: &str, text: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in scene.bytes().chain(speaker.bytes()).chain(text.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("msg.{scene}.{speaker}.{:016x}", h)
}

/// Dump unit as readable VS2-ish text (debug).
pub fn dump_lowered(unit: &Vs2Unit) -> String {
    unit.disasm()
}
