//! Single spine: [`StoryProgram`] → `OpVs2` / `Vs2Unit` (fallback host only).
//!
//! Prefer product `StoryPlayer` for execution. This path exists so the alternate
//! backend is **derived** from the same IR, not a second independent lower.

use indexmap::IndexMap;
use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_compiler::vs2_codegen::{Vs2Instr, Vs2Unit};
use velvet_story::{
    AssignOp, StoryArithOp, StoryCmpOp, StoryCond, StoryExpr, StoryOp, StoryOperand, StoryProgram,
    StoryValue,
};

use crate::source_map::SourceMap;
use crate::to_story_program::OpSrc;

/// Lower a product StoryProgram into a VS2 host unit (debug / fallback).
pub fn story_program_to_vs2(program: &StoryProgram) -> Vs2Unit {
    story_program_to_vs2_mapped(program, &IndexMap::new()).0
}

/// Lower StoryProgram → OpVs2 **and** a PC-aware source map from parallel origins.
pub fn story_program_to_vs2_mapped(
    program: &StoryProgram,
    origins: &IndexMap<String, Vec<OpSrc>>,
) -> (Vs2Unit, SourceMap) {
    let mut unit = Vs2Unit::new(program.title.clone());
    let mut map = SourceMap::new(program.title.clone());
    for (name, scene) in &program.scenes {
        let entry = unit.pc();
        unit.entry_scenes.insert(name.clone(), entry);
        // Scene boundary entry (file from first op origin when available).
        let scene_file = origins
            .get(name)
            .and_then(|s| s.first())
            .map(|s| match s {
                OpSrc::Leaf { file, .. } | OpSrc::If { file, .. } | OpSrc::Choice { file, .. } => {
                    file.clone()
                }
            })
            .unwrap_or_else(|| program.title.clone());
        map.push_in_file(
            scene_file,
            crate::span::Span::at(1, 1, 0, 0),
            "scene",
            format!("scene {name}"),
            Some(entry),
        );
        let srcs = origins.get(name);
        for (i, op) in scene.ops.iter().enumerate() {
            let src = srcs.and_then(|s| s.get(i));
            emit_op_mapped(&mut unit, &mut map, op, src);
        }
        unit.emit(Vs2Instr::new(OpVs2::Ret));
    }
    velvet_script_compiler::vs2_codegen::link_scenes(&mut unit);
    (unit, map)
}

fn map_leaf(map: &mut SourceMap, src: Option<&OpSrc>, pc: u32) {
    match src {
        Some(OpSrc::Leaf {
            file,
            span,
            kind,
            gen,
        }) => {
            map.push_in_file(file.clone(), *span, kind.clone(), gen.clone(), Some(pc));
        }
        Some(OpSrc::If { file, span, .. }) => {
            map.push_in_file(file.clone(), *span, "if", "cond", Some(pc));
        }
        Some(OpSrc::Choice { file, span, .. }) => {
            map.push_in_file(file.clone(), *span, "choice", "menu", Some(pc));
        }
        None => {}
    }
}

fn emit_op_mapped(unit: &mut Vs2Unit, map: &mut SourceMap, op: &StoryOp, src: Option<&OpSrc>) {
    let pc = unit.pc();
    map_leaf(map, src, pc);
    match (op, src) {
        (
            StoryOp::If {
                cond,
                then_ops,
                else_ops,
            },
            Some(OpSrc::If {
                then,
                else_ops: e_src,
                ..
            }),
        ) => {
            emit_cond(unit, cond);
            let j_else = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0));
            for (o, s) in then_ops.iter().zip(then.iter()) {
                emit_op_mapped(unit, map, o, Some(s));
            }
            // leftover ops if origins shorter
            for o in then_ops.iter().skip(then.len()) {
                emit_op(unit, o);
            }
            let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0));
            let else_pc = unit.pc();
            unit.patch_a(j_else, else_pc);
            for (o, s) in else_ops.iter().zip(e_src.iter()) {
                emit_op_mapped(unit, map, o, Some(s));
            }
            for o in else_ops.iter().skip(e_src.len()) {
                emit_op(unit, o);
            }
            let end = unit.pc();
            unit.patch_a(j_end, end);
        }
        (StoryOp::Choice { options }, Some(OpSrc::Choice { arms, .. })) => {
            unit.emit(Vs2Instr::with_a(OpVs2::Menu, options.len() as u32));
            let choice_key = unit.pool.intern("__choice");
            let mut end_jumps: Vec<u32> = Vec::new();
            for (i, arm) in options.iter().enumerate() {
                let lid = unit.pool.intern(arm.text.as_str());
                unit.emit(Vs2Instr::with_ab(OpVs2::Choice, lid, i as u32));
                unit.emit(Vs2Instr::with_a(OpVs2::LoadState, choice_key));
                let idx = unit.pool.intern(i.to_string());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, idx));
                unit.emit(Vs2Instr::new(OpVs2::Eq));
                let j_skip = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0));
                let arm_src = arms.get(i);
                for (j, o) in arm.body.iter().enumerate() {
                    let s = arm_src.and_then(|a| a.get(j));
                    emit_op_mapped(unit, map, o, s);
                }
                let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0));
                end_jumps.push(j_end);
                let skip_pc = unit.pc();
                unit.patch_a(j_skip, skip_pc);
            }
            let end_pc = unit.pc();
            for j in end_jumps {
                unit.patch_a(j, end_pc);
            }
        }
        _ => emit_op(unit, op),
    }
}

fn emit_op(unit: &mut Vs2Unit, op: &StoryOp) {
    match op {
        StoryOp::Nop | StoryOp::Label { .. } => {
            unit.emit(Vs2Instr::new(OpVs2::Nop));
        }
        // Explicit return from call_scene — same Ret opcode MiniVm uses with call_stack.
        StoryOp::Return => {
            unit.emit(Vs2Instr::new(OpVs2::Ret));
        }
        StoryOp::Background { path } => {
            let id = unit.pool.intern(path.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::Background, id));
        }
        StoryOp::Music { path, .. } => {
            let id = unit.pool.intern(path.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::Music, id));
        }
        StoryOp::Sound { path } => {
            let id = unit.pool.intern(path.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::PlaySfx, id));
        }
        StoryOp::Show { target, at } => {
            let id = unit.pool.intern(target.as_str());
            let at_id = at
                .as_ref()
                .map(|a| unit.pool.intern(a.as_str()))
                .unwrap_or(0);
            unit.emit(Vs2Instr::with_ab(OpVs2::ShowChar, id, at_id));
        }
        StoryOp::Hide { target } => {
            let id = unit.pool.intern(target.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::HideChar, id));
        }
        StoryOp::Dialogue { speaker, text } => {
            let sp = unit.pool.intern(speaker.as_deref().unwrap_or("narrator"));
            let mid = unit.pool.intern(text.as_str());
            // Use LoadConst of text then Say (msg id = full text for host t fallback)
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, mid));
            unit.emit(Vs2Instr::with_a(OpVs2::Say, sp));
        }
        StoryOp::Jump { target } => {
            let id = unit.pool.intern(target.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::JumpScene, id));
        }
        StoryOp::Call { target } => {
            let id = unit.pool.intern(target.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::CallScene, id));
        }
        StoryOp::Assign {
            name,
            assign_op,
            value,
        } => {
            let kid = unit.pool.intern(name.as_str());
            match assign_op {
                AssignOp::Set => {
                    emit_expr(unit, value);
                    unit.emit(Vs2Instr::with_a(OpVs2::StoreState, kid));
                }
                AssignOp::Add => {
                    // score += n  →  LoadState score; push n; Add; StoreState score
                    unit.emit(Vs2Instr::with_a(OpVs2::LoadState, kid));
                    emit_expr(unit, value);
                    unit.emit(Vs2Instr::new(OpVs2::Add));
                    unit.emit(Vs2Instr::with_a(OpVs2::StoreState, kid));
                }
                AssignOp::Sub => {
                    unit.emit(Vs2Instr::with_a(OpVs2::LoadState, kid));
                    emit_expr(unit, value);
                    unit.emit(Vs2Instr::new(OpVs2::Sub));
                    unit.emit(Vs2Instr::with_a(OpVs2::StoreState, kid));
                }
            }
        }
        StoryOp::If {
            cond,
            then_ops,
            else_ops,
        } => {
            emit_cond(unit, cond);
            // JumpIf jumps when falsy
            let j_else = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0));
            for o in then_ops {
                emit_op(unit, o);
            }
            let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0));
            let else_pc = unit.pc();
            unit.patch_a(j_else, else_pc);
            for o in else_ops {
                emit_op(unit, o);
            }
            let end = unit.pc();
            unit.patch_a(j_end, end);
        }
        StoryOp::Choice { options } => {
            // Match StoryPlayer: only the selected arm runs.
            // Runner seeds host state `__choice` (see pipeline::run_build).
            unit.emit(Vs2Instr::with_a(OpVs2::Menu, options.len() as u32));
            let choice_key = unit.pool.intern("__choice");
            let mut end_jumps: Vec<u32> = Vec::new();
            for (i, arm) in options.iter().enumerate() {
                let lid = unit.pool.intern(arm.text.as_str());
                unit.emit(Vs2Instr::with_ab(OpVs2::Choice, lid, i as u32));
                // if __choice != i → skip body (JumpIf when falsy)
                unit.emit(Vs2Instr::with_a(OpVs2::LoadState, choice_key));
                let idx = unit.pool.intern(i.to_string());
                unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, idx));
                unit.emit(Vs2Instr::new(OpVs2::Eq));
                let j_skip = unit.emit(Vs2Instr::with_a(OpVs2::JumpIf, 0));
                for o in &arm.body {
                    emit_op(unit, o);
                }
                let j_end = unit.emit(Vs2Instr::with_a(OpVs2::Jump, 0));
                end_jumps.push(j_end);
                let skip_pc = unit.pc();
                unit.patch_a(j_skip, skip_pc);
            }
            let end_pc = unit.pc();
            for j in end_jumps {
                unit.patch_a(j, end_pc);
            }
        }
        StoryOp::End { .. } => {
            unit.emit(Vs2Instr::new(OpVs2::Ret));
        }
        StoryOp::HostCall { name, args } => {
            let cid = unit.pool.intern(name.as_str());
            for (_k, v) in args {
                emit_value(unit, v);
            }
            unit.emit(Vs2Instr::with_ab(OpVs2::Call, cid, args.len() as u32));
        }
        StoryOp::Pause { .. } => {
            unit.emit(Vs2Instr::new(OpVs2::Await));
        }
        StoryOp::Transition { name } => {
            let id = unit.pool.intern(name.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::TransitionPlay, id));
        }
    }
}

fn emit_cond(unit: &mut Vs2Unit, cond: &StoryCond) {
    match cond {
        StoryCond::Var { name } => {
            let kid = unit.pool.intern(name.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadState, kid));
        }
        StoryCond::Const { value } => {
            let id = unit.pool.intern(if *value { "1" } else { "0" });
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
        }
        StoryCond::Not { inner } => {
            emit_cond(unit, inner);
            unit.emit(Vs2Instr::new(OpVs2::Not));
        }
        StoryCond::And { left, right } => {
            emit_cond(unit, left);
            emit_cond(unit, right);
            unit.emit(Vs2Instr::new(OpVs2::And));
        }
        StoryCond::Or { left, right } => {
            emit_cond(unit, left);
            emit_cond(unit, right);
            unit.emit(Vs2Instr::new(OpVs2::Or));
        }
        StoryCond::Cmp { left, op, right } => {
            emit_operand(unit, left);
            emit_operand(unit, right);
            let opc = match op {
                StoryCmpOp::Eq => OpVs2::Eq,
                StoryCmpOp::Ne => OpVs2::Ne,
                StoryCmpOp::Lt => OpVs2::Lt,
                StoryCmpOp::Le => OpVs2::Le,
                StoryCmpOp::Gt => OpVs2::Gt,
                StoryCmpOp::Ge => OpVs2::Ge,
            };
            unit.emit(Vs2Instr::new(opc));
        }
    }
}

fn emit_operand(unit: &mut Vs2Unit, op: &StoryOperand) {
    match op {
        StoryOperand::Var { name } => {
            let kid = unit.pool.intern(name.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadState, kid));
        }
        StoryOperand::Value { value } => emit_value(unit, value),
    }
}

fn emit_expr(unit: &mut Vs2Unit, e: &StoryExpr) {
    match e {
        StoryExpr::Value { value } => emit_value(unit, value),
        StoryExpr::Var { name } => {
            let kid = unit.pool.intern(name.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadState, kid));
        }
        StoryExpr::Neg { inner } => {
            let z = unit.pool.intern("0");
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, z));
            emit_expr(unit, inner);
            unit.emit(Vs2Instr::new(OpVs2::Sub));
        }
        StoryExpr::Binary { op, left, right } => {
            emit_expr(unit, left);
            emit_expr(unit, right);
            let opc = match op {
                StoryArithOp::Add => OpVs2::Add,
                StoryArithOp::Sub => OpVs2::Sub,
                StoryArithOp::Mul => OpVs2::Mul,
                StoryArithOp::Div => OpVs2::Div,
            };
            unit.emit(Vs2Instr::new(opc));
        }
    }
}

fn emit_value(unit: &mut Vs2Unit, v: &StoryValue) {
    match v {
        StoryValue::Null => {
            let id = unit.pool.intern("");
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
        }
        StoryValue::Bool(b) => {
            let id = unit.pool.intern(if *b { "1" } else { "0" });
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
        }
        StoryValue::Int(n) => {
            let id = unit.pool.intern(n.to_string());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
        }
        StoryValue::Float(f) => {
            let id = unit.pool.intern(f.to_string());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
        }
        StoryValue::String(s) => {
            let id = unit.pool.intern(s.as_str());
            unit.emit(Vs2Instr::with_a(OpVs2::LoadConst, id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandRegistry;
    use crate::pipeline::build_story_program;
    use crate::WELCOME_SAMPLE;

    #[test]
    fn vs2_derived_from_story_program() {
        let cmds = CommandRegistry::builtin();
        let prog = build_story_program(WELCOME_SAMPLE, "w.vstory", &cmds, "w").unwrap();
        let unit = story_program_to_vs2(&prog);
        assert!(unit.entry_scenes.contains_key("start"));
        assert!(unit
            .code
            .iter()
            .any(|i| matches!(i.op, OpVs2::Say | OpVs2::Background)));
    }

    #[test]
    fn assign_add_sub_emit_load_add_store() {
        let src = r#"
scene start
set score = 5
add score 2
sub score 1
end
"#;
        let cmds = CommandRegistry::builtin();
        let prog = build_story_program(src, "as.vstory", &cmds, "as").unwrap();
        let unit = story_program_to_vs2(&prog);
        let ops: Vec<_> = unit.code.iter().map(|i| i.op).collect();
        // Must load prior state before add/sub (not only StoreState of RHS).
        assert!(
            ops.iter().any(|o| matches!(o, OpVs2::Add)),
            "expected Add in {ops:?}"
        );
        assert!(
            ops.iter().any(|o| matches!(o, OpVs2::Sub)),
            "expected Sub in {ops:?}"
        );
        assert!(
            ops.iter().filter(|o| matches!(o, OpVs2::LoadState)).count() >= 2,
            "add/sub need LoadState; ops={ops:?}"
        );
    }

    #[test]
    fn choice_emits_guards_not_only_sequential_bodies() {
        let src = r#"
scene start
choice:
    "A":
        set path = 1
    "B":
        set path = 2
end
"#;
        let cmds = CommandRegistry::builtin();
        let prog = build_story_program(src, "ch.vstory", &cmds, "ch").unwrap();
        let unit = story_program_to_vs2(&prog);
        let ops: Vec<_> = unit.code.iter().map(|i| i.op).collect();
        assert!(ops.iter().any(|o| matches!(o, OpVs2::Menu)));
        assert!(
            ops.iter().any(|o| matches!(o, OpVs2::Eq)),
            "choice arms must compare __choice, got {ops:?}"
        );
        assert!(
            ops.iter().filter(|o| matches!(o, OpVs2::JumpIf)).count() >= 2,
            "each arm needs JumpIf skip, got {ops:?}"
        );
    }

    #[test]
    fn return_is_ret_not_nop() {
        let src = r#"
scene helper
return

scene start
call scene helper
end
"#;
        let cmds = CommandRegistry::builtin();
        let prog = build_story_program(src, "r.vstory", &cmds, "r").unwrap();
        let unit = story_program_to_vs2(&prog);
        // Helper scene must contain an explicit Ret from StoryOp::Return (not only trailing Ret).
        let helper_pc = *unit.entry_scenes.get("helper").unwrap() as usize;
        let start_pc = *unit.entry_scenes.get("start").unwrap() as usize;
        let helper_end = if helper_pc < start_pc {
            start_pc
        } else {
            unit.code.len()
        };
        let helper_code = &unit.code[helper_pc..helper_end.min(unit.code.len())];
        let rets = helper_code
            .iter()
            .filter(|i| matches!(i.op, OpVs2::Ret))
            .count();
        assert!(
            rets >= 1,
            "expected Ret from return in helper, got {:?}",
            helper_code.iter().map(|i| i.op).collect::<Vec<_>>()
        );
        assert!(
            !helper_code.iter().any(|i| matches!(i.op, OpVs2::Nop)),
            "return must not lower to Nop"
        );
    }
}
