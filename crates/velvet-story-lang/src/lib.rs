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
pub mod locale;
pub mod lexer;
pub mod load;
pub mod lower;
pub mod parser;
pub mod pipeline;
pub mod sema;
pub mod source_map;
pub mod span;
pub mod studio;
pub mod from_program;
pub mod to_story_program;
pub mod token;

pub use commands::{CommandRegistry, CommandSpec};
pub use diag::StoryDiag;
pub use format::{format_source, is_idempotent};
pub use locale::{
    apply_locale_from_env, diag_locale, set_diag_locale, DiagLocale,
};
pub use load::{load_story_path, load_story_source};
pub use pipeline::{
    build_path, build_source, build_story_program, check_path, check_source, dump_ast_json,
    dump_lowered_text, run_path, run_source, run_source_product, run_story_program, BuildResult,
    CheckResult, ProgramRunResult, RunResult,
};
pub use studio::{build_model, StudioModel};
pub use to_story_program::{to_story_program, ToProgramError};

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
        assert!(
            r.log.iter().any(|l| l.contains("command combat.start")),
            "expected host log for combat.start, got {:?}",
            r.log
        );
        assert!(
            r.log
                .iter()
                .any(|l| l.contains("forest_guardian")),
            "expected enemy ident forest_guardian in command args log, got {:?}",
            r.log
        );
        assert!(
            r.state
                .iter()
                .any(|(k, v)| k == "__last_command" && v == "combat.start"),
            "state={:?}",
            r.state
        );
    }

    #[test]
    fn bad_if_string_condition_errors() {
        set_diag_locale(DiagLocale::Es);
        let src = "scene a\nif \"luna\":\n    goto a\n";
        let cmds = CommandRegistry::builtin();
        let c = check_source(src, "badif.vstory", &cmds);
        assert!(!c.ok);
        let d = c.diags.iter().find(|d| d.code == "VST030").expect("VST030");
        assert!(d.display().contains("badif.vstory"));
        assert!(
            d.message.contains("verdadero o falso")
                || d.message.contains("texto")
                || d.message.contains("true or false"),
            "msg={}",
            d.message
        );
    }

    #[test]
    fn diag_locale_switches_vst027_text() {
        let src = "scene a\ngoto missing_xyz\n";
        let cmds = CommandRegistry::builtin();
        let mut texts = std::collections::BTreeMap::new();
        for loc in DiagLocale::all() {
            set_diag_locale(*loc);
            let c = check_source(src, "loc.vstory", &cmds);
            let d = c.diags.iter().find(|d| d.code == "VST027").expect("VST027");
            assert_eq!(d.code, "VST027");
            assert!(
                d.message.contains("missing_xyz"),
                "locale {:?} msg={}",
                loc,
                d.message
            );
            texts.insert(loc.code(), d.display());
        }
        // Spanish cue
        assert!(
            texts["es"].contains("escena") || texts["es"].contains("etiqueta"),
            "{}",
            texts["es"]
        );
        // English cue
        assert!(
            texts["en"].to_ascii_lowercase().contains("scene")
                || texts["en"].contains("label"),
            "{}",
            texts["en"]
        );
        // Japanese / German / Chinese differ from English & Spanish
        assert_ne!(texts["es"], texts["en"]);
        assert_ne!(texts["en"], texts["ja"]);
        assert_ne!(texts["en"], texts["de"]);
        assert_ne!(texts["en"], texts["zh"]);
        assert!(texts["ja"].contains("シーン") || texts["ja"].contains("ラベル"));
        assert!(texts["de"].contains("Szene") || texts["de"].contains("Label"));
        assert!(texts["zh"].contains("场景") || texts["zh"].contains("标签"));
        // suggestion labels localized
        assert!(texts["es"].contains("Sugerencia:"));
        assert!(texts["en"].contains("Suggestion:"));
        set_diag_locale(DiagLocale::Es);
    }

    #[test]
    fn no_second_vm_types() {
        // Vs2MiniVm and OpVs2 come from velvet-script-* (existing stack).
        let _ = std::any::type_name::<velvet_script_vm::Vs2MiniVm>();
        let _ = std::any::type_name::<velvet_script_bytecode::opcodes_vs2::OpVs2>();
    }
}
