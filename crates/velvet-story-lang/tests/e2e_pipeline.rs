//! End-to-end: Velvet Story → StoryProgram → StoryPlayer → observable result.

use velvet_story::StoryOp;
use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::pipeline::{build_story_program, check_source, run_source_product};
use velvet_story_lang::{format_source, is_idempotent, WELCOME_SAMPLE};

#[test]
fn e2e_welcome_story() {
    let cmds = CommandRegistry::builtin();
    let check = check_source(WELCOME_SAMPLE, "welcome.vstory", &cmds);
    assert!(check.ok, "{:?}", check.diags);

    let program = build_story_program(WELCOME_SAMPLE, "welcome.vstory", &cmds, "welcome").unwrap();
    assert!(program.scenes.contains_key("start"));
    assert!(program.scenes.contains_key("ending"));

    let run = run_source_product(WELCOME_SAMPLE, "welcome.vstory", &cmds, 0).unwrap();
    assert!(run.steps > 0);
    assert!(!run.dialogue.is_empty(), "dialogue={:?}", run.dialogue);
}

#[test]
fn e2e_format_idempotent() {
    let a = format_source(WELCOME_SAMPLE);
    let b = format_source(&a);
    assert_eq!(a, b);
    assert!(is_idempotent(WELCOME_SAMPLE));
}

#[test]
fn e2e_command_reaches_the_program_as_a_host_call() {
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
    let program = build_story_program(src, "c.vstory", &cmds, "c").unwrap();
    let (name, args) = program
        .scenes
        .values()
        .flat_map(|s| s.ops.iter())
        .find_map(|op| match op {
            StoryOp::HostCall { name, args } => Some((name.clone(), args.clone())),
            _ => None,
        })
        .expect("a host call");
    assert_eq!(name, "combat.start");
    assert!(
        args.values()
            .any(|v| v.display_str().contains("forest_guardian")),
        "a bare identifier stays a value: {args:?}"
    );
}

#[test]
fn e2e_include_multi_file_goto() {
    use std::fs;
    use tempfile::tempdir;
    use velvet_story_lang::pipeline::{check_path, run_path_product};

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
    let r = run_path_product(&main, &cmds, 0).unwrap();
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
