//! Single spine: [`StoryProgram`] → `OpVs2` / `Vs2Unit` (fallback host only).
//!
//! Prefer product `StoryPlayer` for execution. This path exists so the alternate
//! backend is **derived** from the same IR, not a second independent lower.

use velvet_script_bytecode::opcodes_vs2::OpVs2;
use velvet_script_compiler::vs2_codegen::{Vs2Instr, Vs2Unit};
use velvet_story::{StoryCmpOp, StoryCond, StoryOp, StoryOperand, StoryProgram, StoryValue};

/// Lower a product StoryProgram into a VS2 host unit (debug / fallback).
pub fn story_program_to_vs2(program: &StoryProgram) -> Vs2Unit {
    let mut unit = Vs2Unit::new(program.title.clone());
    // entry: prefer program.entry
    for (name, scene) in &program.scenes {
        let entry = unit.pc();
        unit.entry_scenes.insert(name.clone(), entry);
        for op in &scene.ops {
            emit_op(&mut unit, op);
        }
        // Scene fall-through ends with Ret: returns from CallScene or halts if root.
        unit.emit(Vs2Instr::new(OpVs2::Ret));
    }
    // link scene jumps
    velvet_script_compiler::vs2_codegen::link_scenes(&mut unit);
    unit
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
            let sp = unit
                .pool
                .intern(speaker.as_deref().unwrap_or("narrator"));
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
        StoryOp::Assign { name, value, .. } => {
            let kid = unit.pool.intern(name.as_str());
            emit_value(unit, value);
            unit.emit(Vs2Instr::with_a(OpVs2::StoreState, kid));
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
            unit.emit(Vs2Instr::with_a(OpVs2::Menu, options.len() as u32));
            for (i, arm) in options.iter().enumerate() {
                let lid = unit.pool.intern(arm.text.as_str());
                unit.emit(Vs2Instr::with_ab(OpVs2::Choice, lid, i as u32));
                for o in &arm.body {
                    emit_op(unit, o);
                }
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
        let rets = helper_code.iter().filter(|i| matches!(i.op, OpVs2::Ret)).count();
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
