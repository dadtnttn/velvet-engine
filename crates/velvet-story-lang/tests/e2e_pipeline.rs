//! End-to-end: Velvet Story → lower → OpVs2 → Vs2Host → observable result.

use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::pipeline::{build_source, check_source, run_source};
use velvet_story_lang::{format_source, is_idempotent, WELCOME_SAMPLE};

#[test]
fn e2e_welcome_story() {
    let cmds = CommandRegistry::builtin();
    let check = check_source(WELCOME_SAMPLE, "welcome.vstory", &cmds);
    assert!(check.ok, "{:?}", check.diags);

    let build = build_source(WELCOME_SAMPLE, "welcome.vstory", &cmds);
    assert!(build.ok);
    let unit = build.lowered.as_ref().unwrap();
    assert!(!unit.unit.code.is_empty());
    assert!(unit.unit.entry_scenes.contains_key("start"));
    assert!(unit.unit.entry_scenes.contains_key("ending"));
    assert!(!unit.msg_ids.is_empty());
    assert!(!unit.map.entries.is_empty());

    let run = run_source(WELCOME_SAMPLE, "welcome.vstory", &cmds, 0);
    assert!(run.ok);
    assert!(run.steps > 0);
    // Host should produce dialogue or presentation side-effects
    assert!(
        !run.dialogue.is_empty() || run.log.iter().any(|l| l.contains("say")),
        "dialogue={:?} log={:?}",
        run.dialogue,
        run.log
    );
}

#[test]
fn e2e_uses_existing_vs2_opcodes() {
    let cmds = CommandRegistry::builtin();
    let build = build_source(WELCOME_SAMPLE, "w.vstory", &cmds);
    let unit = build.lowered.unwrap();
    // At least one story opcode from velvet-script-bytecode
    use velvet_script_bytecode::opcodes_vs2::OpVs2;
    let has_story = unit.unit.code.iter().any(|i| {
        matches!(
            i.op,
            OpVs2::Say | OpVs2::Background | OpVs2::ShowChar | OpVs2::JumpScene | OpVs2::Menu
        )
    });
    assert!(has_story);
}

#[test]
fn e2e_format_idempotent() {
    let a = format_source(WELCOME_SAMPLE);
    let b = format_source(&a);
    assert_eq!(a, b);
    assert!(is_idempotent(WELCOME_SAMPLE));
}

#[test]
fn e2e_source_map_points_to_vstory() {
    let cmds = CommandRegistry::builtin();
    let build = build_source(WELCOME_SAMPLE, "stories/welcome.vstory", &cmds);
    let map = &build.lowered.unwrap().map;
    assert_eq!(map.file, "stories/welcome.vstory");
    assert!(map.entries.iter().any(|e| e.node_kind == "scene"));
    assert!(map
        .entries
        .iter()
        .any(|e| e.origin.file.contains("welcome")));
}

#[test]
fn e2e_command_from_vs2_registry() {
    let src = r#"
scene start
call combat.start:
    enemy: forest_guardian
    difficulty: 2
    can_escape: false
end
"#;
    let cmds = CommandRegistry::builtin();
    assert!(check_source(src, "c.vstory", &cmds).ok);
    let r = run_source(src, "c.vstory", &cmds, 0);
    assert!(r.ok);
    assert!(
        r.log.iter().any(|l| l.contains("command combat.start")),
        "log={:?}",
        r.log
    );
    assert!(
        r.log.iter().any(|l| l.contains("forest_guardian")),
        "bare Ident enemy must lower as LoadConst, log={:?}",
        r.log
    );
    assert!(r
        .state
        .iter()
        .any(|(k, v)| k == "__last_command" && v.contains("combat.start")));
}

#[test]
fn e2e_include_multi_file_goto() {
    use std::fs;
    use tempfile::tempdir;
    use velvet_story_lang::pipeline::{check_path, run_path};

    let dir = tempdir().unwrap();
    let part = dir.path().join("part.vstory");
    let main = dir.path().join("main.vstory");
    fs::write(&part, "scene hallway\nnarrator:\n    En el pasillo.\nend\n").unwrap();
    fs::write(
        &main,
        "include \"part.vstory\"\n\nscene start\nnarrator:\n    Inicio.\ngoto hallway\n",
    )
    .unwrap();
    let cmds = CommandRegistry::builtin();
    let c = check_path(&main, &cmds).unwrap();
    assert!(c.ok, "{:?}", c.diags);
    assert!(c
        .file
        .items
        .iter()
        .any(|i| matches!(i, velvet_story_lang::ast::TopItem::Scene(s) if s.name == "hallway")));
    let r = run_path(&main, &cmds, 0).unwrap();
    assert!(r.ok);
    assert!(
        r.dialogue
            .iter()
            .any(|d| d.contains("pasillo") || d.contains("Inicio")),
        "dialogue={:?}",
        r.dialogue
    );
}

#[test]
fn e2e_bad_if_diag_points_to_vstory() {
    let src = r#"
scene start
if "luna":
    narrator:
        bad
end
"#;
    let cmds = CommandRegistry::builtin();
    let c = check_source(src, "stories/chapter_1.vstory", &cmds);
    assert!(!c.ok);
    let d = c.diags.iter().find(|d| d.code == "VST030").unwrap();
    let text = d.display();
    assert!(text.contains("stories/chapter_1.vstory"));
    assert!(text.contains("VST030"));
}
