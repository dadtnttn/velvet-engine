//! # velvet-story-lang
//!
//! **Velvet Story** — writer-friendly narrative language that lowers to
//! **Velvet Script 2** (HIR + `OpVs2` + existing host/VM). No second bytecode VM.
//!
//! Pipeline:
//! ```text
//! .vstory → lexer → parser → AST → sema → lower → Hir + Vs2Unit → Vs2Host
//! ```
//!
//! Maturity: **ALPHA** (authoring layer for writers).

#![deny(missing_docs)]

pub mod ast;
pub mod commands;
pub mod diag;
pub mod format;
pub mod i18n_extract;
pub mod lexer;
pub mod lower;
pub mod parser;
pub mod pipeline;
pub mod sema;
pub mod source_map;
pub mod span;
pub mod studio;
pub mod token;

pub use commands::{CommandRegistry, CommandSpec};
pub use diag::StoryDiag;
pub use format::{format_source, is_idempotent};
pub use pipeline::{
    build_source, check_path, check_source, dump_ast_json, dump_lowered_text, run_source,
    BuildResult, CheckResult, RunResult,
};
pub use studio::{build_model, StudioModel};

/// Welcome sample used in acceptance tests (from product requirements).
pub const WELCOME_SAMPLE: &str = r#"scene start

background bedroom
show luna happy

luna:
    Hola. Bienvenido a Velvet.

set affection = 0

choice:
    "Saludar":
        add affection 1

        luna:
            Me alegra verte.

        goto ending

    "No responder":
        narrator:
            Luna guarda silencio.

        goto ending

scene ending

if affection > 0:
    narrator:
        Parece que esta historia empieza bien.
else:
    narrator:
        Quizás mañana sea diferente.

end
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandRegistry;

    #[test]
    fn welcome_check_ok() {
        let cmds = CommandRegistry::builtin();
        let c = check_source(WELCOME_SAMPLE, "welcome.vstory", &cmds);
        for d in &c.diags {
            if d.is_error() {
                eprintln!("{}", d.display());
            }
        }
        assert!(c.ok, "check failed: {:?}", c.diags);
    }

    #[test]
    fn welcome_run_e2e() {
        let cmds = CommandRegistry::builtin();
        let r = run_source(WELCOME_SAMPLE, "welcome.vstory", &cmds, 0);
        assert!(r.ok, "run failed");
        assert!(
            !r.dialogue.is_empty() || !r.log.is_empty() || r.steps > 0,
            "expected observable output; dialogue={:?} log={:?} steps={}",
            r.dialogue,
            r.log,
            r.steps
        );
    }

    #[test]
    fn format_idempotent() {
        assert!(is_idempotent(WELCOME_SAMPLE));
    }

    #[test]
    fn goto_missing_errors() {
        let src = "scene a\n    goto missing_scene\n";
        let cmds = CommandRegistry::builtin();
        let c = check_source(src, "bad.vstory", &cmds);
        assert!(!c.ok);
        assert!(c.diags.iter().any(|d| d.code == "VST027"));
        assert!(c.diags.iter().any(|d| d.display().contains("missing_scene")));
    }

    #[test]
    fn external_command_call() {
        let src = r#"
scene start
call combat.start:
    enemy: forest_guardian
    difficulty: 3
    can_escape: true
end
"#;
        let cmds = CommandRegistry::builtin();
        let c = check_source(src, "combat.vstory", &cmds);
        assert!(c.ok, "{:?}", c.diags);
        let r = run_source(src, "combat.vstory", &cmds, 0);
        assert!(r.ok);
    }

    #[test]
    fn no_second_vm_types() {
        // Vs2MiniVm and OpVs2 come from velvet-script-* (existing stack).
        let _ = std::any::type_name::<velvet_script_vm::Vs2MiniVm>();
        let _ = std::any::type_name::<velvet_script_bytecode::opcodes_vs2::OpVs2>();
    }
}
