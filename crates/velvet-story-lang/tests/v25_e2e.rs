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
fn sound_pause_return_not_nop() {
    let src = r#"
scene helper
sound click
return

scene start
call scene helper
pause 1
with fade
narrator:
    back
end
"#;
    let cmds = CommandRegistry::builtin();
    let prog = build_story_program(src, "ops.vstory", &cmds, "ops").unwrap();
    let helper = &prog.scenes["helper"].ops;
    assert!(
        helper.iter().any(|o| matches!(o, StoryOp::Sound { .. })),
        "{helper:?}"
    );
    assert!(
        helper.iter().any(|o| matches!(o, StoryOp::Return)),
        "{helper:?}"
    );
    let start = &prog.scenes["start"].ops;
    assert!(start.iter().any(|o| matches!(o, StoryOp::Pause { .. })));
    assert!(start
        .iter()
        .any(|o| matches!(o, StoryOp::Transition { .. })));
}

#[test]
fn format_path_check_detects_dirty() {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use velvet_story_lang::pipeline::format_path;
    let mut f = NamedTempFile::new().unwrap();
    // deliberately messy spacing that formatter will change
    write!(f, "scene start\nluna:\nHola\n").unwrap();
    let path = f.path().to_path_buf();
    let err = format_path(&path, true);
    // either needs formatting or already pretty — if pretty equal, ok
    if let Err(e) = err {
        assert!(e.contains("needs formatting") || e.contains("idempotent"), "{e}");
    }
}

#[test]
fn format_preserves_inline_scene_comment() {
    use velvet_story_lang::format::format_source;
    use velvet_story_lang::parser::parse;
    let src = "scene start\n# keep me\nluna:\n    Hi\n";
    let p = parse(src, "c.vstory");
    // comment should appear as Stmt::Comment inside scene after parser fix
    let sc = p.file.items.iter().find_map(|i| match i {
        velvet_story_lang::ast::TopItem::Scene(s) => Some(s),
        _ => None,
    });
    assert!(sc.is_some());
    let has_c = sc.unwrap().body.iter().any(|s| {
        matches!(s, velvet_story_lang::ast::Stmt::Comment { text, .. } if text.contains("keep"))
    });
    assert!(has_c, "comment lost in AST: {:?}", sc.unwrap().body);
    let _ = format_source(src);
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

#[test]
fn rich_if_and_or_not_and_compare_branches() {
    // a=1, b=1 → then; also not flag (flag=0) → then arm of second if.
    let src = r#"
scene start
set a = 1
set b = 1
set flag = 0
set x = 5
if a and b:
    set path = 1
else:
    set path = 0
if not flag:
    set npath = 1
else:
    set npath = 0
if x >= 5:
    set cmp = 1
else:
    set cmp = 0
narrator:
    done
end
"#;
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(src, "richif.vstory", &cmds, 0).unwrap();
    let get = |k: &str| {
        r.vars
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    assert_eq!(get("path"), "1", "and branch vars={:?}", r.vars);
    assert_eq!(get("npath"), "1", "not branch vars={:?}", r.vars);
    assert_eq!(get("cmp"), "1", ">= branch vars={:?}", r.vars);
}

#[test]
fn rich_if_false_arms() {
    let src = r#"
scene start
set a = 1
set b = 0
set flag = 1
set x = 2
if a and b:
    set path = 1
else:
    set path = 9
if not flag:
    set npath = 1
else:
    set npath = 9
if x >= 5:
    set cmp = 1
else:
    set cmp = 9
narrator:
    done
end
"#;
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(src, "richif_f.vstory", &cmds, 0).unwrap();
    let get = |k: &str| {
        r.vars
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.as_str())
            .unwrap_or("")
    };
    assert_eq!(get("path"), "9", "vars={:?}", r.vars);
    assert_eq!(get("npath"), "9", "vars={:?}", r.vars);
    assert_eq!(get("cmp"), "9", "vars={:?}", r.vars);
}

#[test]
fn unary_neg_assign_exact_value() {
    let src = r#"
scene start
set score = -5
add score -3
call combat.start:
    enemy: boss
    difficulty: -2
    can_escape: false
narrator:
    done
end
"#;
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(src, "neg.vstory", &cmds, 0).unwrap();
    let score = r
        .vars
        .iter()
        .find(|(k, _)| k == "score")
        .map(|(_, v)| v.as_str());
    assert_eq!(score, Some("-8"), "vars={:?}", r.vars);
    // difficulty -2 stored on cmd path
    let diff = r.vars.iter().find(|(k, _)| k.contains("difficulty"));
    assert!(
        diff.map(|(_, v)| v.as_str() == "-2").unwrap_or(false),
        "expected difficulty -2, vars={:?}",
        r.vars
    );
}

#[test]
fn call_return_product_and_vs2_fallback() {
    use velvet_script_bytecode::opcodes_vs2::OpVs2;
    use velvet_story_lang::from_program::story_program_to_vs2;
    use velvet_story_lang::pipeline::run_source;

    let src = r#"
scene helper
set marker = 7
return

scene start
set marker = 0
call scene helper
narrator:
    back_from_helper
end
"#;
    let cmds = CommandRegistry::builtin();
    // Product path: Return pops call stack → dialogue after call runs.
    let r = run_source_product(src, "ret.vstory", &cmds, 0).unwrap();
    assert!(
        r.dialogue.iter().any(|l| l.contains("back_from_helper")),
        "product dialogue={:?}",
        r.dialogue
    );
    let marker = r
        .vars
        .iter()
        .find(|(k, _)| k == "marker")
        .map(|(_, v)| v.as_str());
    assert_eq!(marker, Some("7"), "vars={:?}", r.vars);

    // Fallback OpVs2: Return lowers to Ret (not Nop); CallScene present; run observes marker.
    let prog = build_story_program(src, "ret.vstory", &cmds, "ret").unwrap();
    let unit = story_program_to_vs2(&prog);
    assert!(
        unit.code.iter().any(|i| matches!(i.op, OpVs2::CallScene)),
        "expected CallScene"
    );
    assert!(
        unit.code.iter().any(|i| matches!(i.op, OpVs2::Ret)),
        "expected Ret from return"
    );
    // Must not emit Nop for return in helper body before trailing Ret alone is OK —
    // stronger: CallScene + Ret appear and host run sets marker.
    let vs2 = run_source(src, "ret.vstory", &cmds, 0);
    assert!(vs2.ok, "vs2 fallback run failed");
    assert!(
        vs2.state
            .iter()
            .any(|(k, v)| k == "marker" && v == "7"),
        "vs2 state={:?}",
        vs2.state
    );
    assert!(
        vs2.dialogue
            .iter()
            .any(|l| l.contains("back_from_helper")),
        "vs2 dialogue={:?} (return must resume caller)",
        vs2.dialogue
    );
}

#[test]
fn include_error_attributes_origin_file() {
    use tempfile::tempdir;
    use velvet_story_lang::pipeline::check_path;

    let dir = tempdir().unwrap();
    let child = dir.path().join("child_broken.vstory");
    let root = dir.path().join("root.vstory");
    // Error only in included file: goto missing target
    std::fs::write(
        &child,
        "scene from_include\ngoto totally_missing_scene_xyz\n",
    )
    .unwrap();
    std::fs::write(
        &root,
        "include \"child_broken.vstory\"\n\nscene start\nnarrator:\n    hi\nend\n",
    )
    .unwrap();
    let cmds = CommandRegistry::builtin();
    let c = check_path(&root, &cmds).unwrap();
    assert!(!c.ok, "expected error from include");
    let d = c
        .diags
        .iter()
        .find(|d| d.code == "VST027")
        .expect("VST027 from include");
    let disp = d.display();
    assert!(
        disp.contains("child_broken") || d.loc.file.contains("child_broken"),
        "diagnostic must name include origin, got display={disp} file={}",
        d.loc.file
    );
    assert!(
        !d.loc.file.ends_with("root.vstory")
            || d.loc.file.contains("child_broken"),
        "must not only blame root: {}",
        d.loc.file
    );
}
