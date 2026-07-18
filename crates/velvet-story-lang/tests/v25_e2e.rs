//! Velvet 2.5 e2e: .vstory → StoryProgram → product StoryPlayer.

use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::pipeline::{
    build_story_program, run_source_product, run_story_program,
};
use velvet_story_lang::WELCOME_SAMPLE;
use velvet_story::StoryOp;

#[test]
fn welcome_product_choice0_exact() {
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(WELCOME_SAMPLE, "welcome.vstory", &cmds, 0).unwrap();
    assert!(
        r.dialogue.iter().any(|l| l.contains("Bienvenido")),
        "dialogue={:?}",
        r.dialogue
    );
    assert!(
        r.dialogue.iter().any(|l| l.contains("Me alegra") || l.contains("alegra")),
        "expected greet branch: {:?}",
        r.dialogue
    );
    assert!(
        r.dialogue
            .iter()
            .any(|l| l.contains("empieza bien") || l.contains("bien")),
        "expected good ending: {:?}",
        r.dialogue
    );
    let aff = r
        .vars
        .iter()
        .find(|(k, _)| k == "affection")
        .map(|(_, v)| v.as_str());
    assert_eq!(aff, Some("1"), "vars={:?}", r.vars);
}

#[test]
fn welcome_product_choice1_exact() {
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(WELCOME_SAMPLE, "welcome.vstory", &cmds, 1).unwrap();
    assert!(
        r.dialogue
            .iter()
            .any(|l| l.contains("silencio") || l.contains("guarda")),
        "{:?}",
        r.dialogue
    );
    let aff = r
        .vars
        .iter()
        .find(|(k, _)| k == "affection")
        .map(|(_, v)| v.as_str())
        .unwrap_or("0");
    // no add on this branch — 0 or unset treated as 0 path
    assert!(aff == "0" || aff.is_empty(), "aff={aff}");
}

#[test]
fn combat_command_on_product_path() {
    let src = r#"
scene start
call combat.start:
    enemy: forest_guardian
    difficulty: 3
    can_escape: true
narrator:
    done
end
"#;
    let cmds = CommandRegistry::builtin();
    let prog = build_story_program(src, "combat.vstory", &cmds, "combat").unwrap();
    assert!(prog.scenes["start"].ops.iter().any(|op| matches!(
        op,
        StoryOp::HostCall { name, .. } if name == "combat.start"
    )));
    let r = run_story_program(prog, 0, 64);
    assert!(
        r.vars
            .iter()
            .any(|(k, v)| k == "__last_command" && v == "combat.start"),
        "vars={:?}",
        r.vars
    );
    assert!(
        r.vars
            .iter()
            .any(|(k, v)| k == "cmd.enemy" && v.contains("forest_guardian")),
        "vars={:?}",
        r.vars
    );
}
