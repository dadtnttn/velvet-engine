//! # velvet-story-lang
//!
//! **Velvet Story** — writer-friendly narrative language.
//!
//! # Product spine (canonical)
//!
//! ```text
//! .vstory → lexer → parser → AST → sema
//!        → StoryProgram          ← canonical IR
//!             ├→ StoryPlayer     ← product runtime (default)
//!             └→ OpVs2 (derived) ← secondary / debug / fallback host
//! ```
//!
//! The primary execution path is **StoryProgram → StoryPlayer**. OpVs2 / Vs2Host
//! are **not** the primary product pipeline; they are derived for dump/fallback.
//!
//! Diagnostic locale is **context-scoped** ([`with_diag_locale`] /
//! [`pipeline::CheckOptions`]) so concurrent multi-doc tools do not race.
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
    apply_locale_from_env, default_diag_locale, diag_locale, diag_message_for, push_diag_locale,
    set_diag_locale, with_diag_locale, DiagLocale, DiagLocaleGuard,
};
pub use load::{load_story_path, load_story_source};
pub use pipeline::{
    build_path, build_source, build_story_program, build_story_program_with, check_path,
    check_path_with, check_source, check_source_with, dump_ast_json, dump_lowered_text, run_path,
    run_path_product, run_source, run_source_product, run_source_product_with, run_story_program,
    BuildResult, CheckOptions, CheckResult, ProgramRunResult, RunResult,
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
        let src = "scene a\nif \"luna\":\n    goto a\n";
        let cmds = CommandRegistry::builtin();
        // Scoped ES so parallel locale tests cannot bleed into this assert.
        let c = check_source_with(
            src,
            "badif.vstory",
            &cmds,
            CheckOptions::new().with_locale(DiagLocale::Es),
        );
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
            // Scoped options — do not rely on process-global alone.
            let c = check_source_with(
                src,
                "loc.vstory",
                &cmds,
                CheckOptions::new().with_locale(*loc),
            );
            let d = c.diags.iter().find(|d| d.code == "VST027").expect("VST027");
            assert_eq!(d.code, "VST027");
            assert_eq!(d.locale, *loc);
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
        // suggestion labels localized (from diag.locale, not process default)
        assert!(texts["es"].contains("Sugerencia:"));
        assert!(texts["en"].contains("Suggestion:"));
    }

    #[test]
    fn concurrent_check_options_locales_no_bleed() {
        use std::sync::mpsc;
        use std::thread;

        let src = "scene a\ngoto missing_xyz\n";
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::new();
        for loc in [DiagLocale::En, DiagLocale::Es, DiagLocale::Ja] {
            let tx = tx.clone();
            let src = src.to_string();
            handles.push(thread::spawn(move || {
                // Global noise must not cross into scoped check.
                set_diag_locale(DiagLocale::Zh);
                let cmds = CommandRegistry::builtin();
                let c = check_source_with(
                    &src,
                    "mt.vstory",
                    &cmds,
                    CheckOptions::new().with_locale(loc),
                );
                let d = c.diags.iter().find(|d| d.code == "VST027").expect("VST027");
                tx.send((loc, d.message.clone(), d.display())).unwrap();
            }));
        }
        drop(tx);
        for h in handles {
            h.join().unwrap();
        }
        set_diag_locale(DiagLocale::Es);
        let mut en_msg = None;
        let mut es_msg = None;
        while let Ok((loc, msg, display)) = rx.recv() {
            match loc {
                DiagLocale::En => {
                    assert!(
                        msg.to_ascii_lowercase().contains("scene") || msg.contains("label"),
                        "{msg}"
                    );
                    assert!(!msg.contains("escena"), "en bled es: {msg}");
                    assert!(!msg.contains("场景"), "en bled zh: {msg}");
                    assert!(display.contains("Suggestion:"), "{display}");
                    en_msg = Some(msg);
                }
                DiagLocale::Es => {
                    assert!(msg.contains("escena") || msg.contains("etiqueta"), "{msg}");
                    assert!(!msg.to_ascii_lowercase().contains("does not exist"), "{msg}");
                    assert!(display.contains("Sugerencia:"), "{display}");
                    es_msg = Some(msg);
                }
                DiagLocale::Ja => {
                    assert!(msg.contains("シーン") || msg.contains("ラベル"), "{msg}");
                    assert!(display.contains("提案:"), "{display}");
                }
                _ => {}
            }
        }
        assert_ne!(en_msg, es_msg);
    }

    #[test]
    fn product_run_is_primary_public_path() {
        let cmds = CommandRegistry::builtin();
        let r = run_source_product(WELCOME_SAMPLE, "welcome.vstory", &cmds, 0)
            .expect("product run");
        assert!(!r.dialogue.is_empty(), "product dialogue empty");
        assert!(r.steps > 0);
        // VS2 secondary still works but is not required for product success.
        let vs2 = run_source(WELCOME_SAMPLE, "welcome.vstory", &cmds, 0);
        assert!(vs2.ok, "secondary vs2 host should still succeed");
    }

    #[test]
    fn no_second_vm_types() {
        // Vs2MiniVm and OpVs2 come from velvet-script-* (existing stack).
        let _ = std::any::type_name::<velvet_script_vm::Vs2MiniVm>();
        let _ = std::any::type_name::<velvet_script_bytecode::opcodes_vs2::OpVs2>();
    }
}
