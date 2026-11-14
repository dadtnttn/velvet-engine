//! Velvet 2.5 e2e: .vstory → StoryProgram → product StoryPlayer.

use velvet_story::StoryOp;
use velvet_story_lang::commands::CommandRegistry;
use velvet_story_lang::pipeline::{build_story_program, run_source_product, run_story_program};
use velvet_story_lang::WELCOME_SAMPLE;

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
        r.dialogue
            .iter()
            .any(|l| l.contains("Me alegra") || l.contains("alegra")),
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
    // No add on this branch: an unset value is normalized to the numeric default.
    assert_eq!(aff, "0");
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
    let error = format_path(&path, true).expect_err("messy source must fail check-only mode");
    assert_eq!(error, "needs formatting");

    let formatted = format_path(&path, false).expect("formatter must rewrite the file");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), formatted);
    assert_eq!(format_path(&path, true).unwrap(), formatted);
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
    let formatted = format_source(src);
    assert!(formatted.contains("# keep me"), "formatted={formatted:?}");
    let reparsed = parse(&formatted, "formatted.vstory");
    let formatted_scene = reparsed
        .file
        .items
        .iter()
        .find_map(|item| match item {
            velvet_story_lang::ast::TopItem::Scene(scene) => Some(scene),
            _ => None,
        })
        .expect("formatted scene");
    assert!(formatted_scene.body.iter().any(|stmt| {
        matches!(stmt, velvet_story_lang::ast::Stmt::Comment { text, .. } if text.contains("keep me"))
    }));
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
fn call_return_product() {
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
    // Return pops the call stack: the dialogue after the call runs.
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
}

#[test]
fn add_sub_runtime() {
    let src = r#"
scene start
set score = 5
add score 2
sub score 1
narrator:
    done
end
"#;
    let cmds = CommandRegistry::builtin();
    let prod = run_source_product(src, "as.vstory", &cmds, 0).unwrap();
    let score = prod
        .vars
        .iter()
        .find(|(k, _)| k == "score")
        .map(|(_, v)| v.as_str());
    assert_eq!(score, Some("6"), "product vars={:?}", prod.vars);
}

#[test]
fn choice_runs_only_selected_arm() {
    let src = r#"
scene start
choice:
    "A":
        set path = 1
        narrator:
            arm_a
    "B":
        set path = 2
        narrator:
            arm_b
end
"#;
    let cmds = CommandRegistry::builtin();
    for (pick, path, own, other) in [(0, "1", "arm_a", "arm_b"), (1, "2", "arm_b", "arm_a")] {
        let r = run_source_product(src, "ch.vstory", &cmds, pick).unwrap();
        let got = r
            .vars
            .iter()
            .find(|(k, _)| k == "path")
            .map(|(_, v)| v.as_str());
        assert_eq!(got, Some(path), "choice {pick} vars={:?}", r.vars);
        assert!(
            r.dialogue.iter().any(|l| l.contains(own)),
            "{:?}",
            r.dialogue
        );
        assert!(
            !r.dialogue.iter().any(|l| l.contains(other)),
            "dialogue leaked {other}: {:?}",
            r.dialogue
        );
    }
}

#[test]
fn include_scenes_reach_the_program() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let child = dir.path().join("chapter2.vstory");
    let root = dir.path().join("main.vstory");
    std::fs::write(
        &child,
        "scene from_ch2\nnarrator:\n    inside_include\nset flag = 1\nend\n",
    )
    .unwrap();
    std::fs::write(
        &root,
        "include \"chapter2.vstory\"\n\nscene start\ngoto from_ch2\n",
    )
    .unwrap();
    let program = velvet_story_lang::load_story_program_from_path(&root).expect("program");
    assert!(
        program.scenes.contains_key("from_ch2"),
        "{:?}",
        program.scenes.keys().collect::<Vec<_>>()
    );
    let r = run_story_program(program, 0, 200);
    assert!(
        r.dialogue.iter().any(|l| l.contains("inside_include")),
        "{:?}",
        r.dialogue
    );
}

#[test]
fn command_param_ty_rejects_wrong_types() {
    use velvet_story_lang::pipeline::check_source;

    let bad = r#"
scene start
call combat.start:
    enemy: forest_guardian
    difficulty: "muy difícil"
    can_escape: 42
end
"#;
    let good = r#"
scene start
call combat.start:
    enemy: forest_guardian
    difficulty: 3
    can_escape: true
end
"#;
    let cmds = CommandRegistry::builtin();
    let c_bad = check_source(bad, "bad_cmd.vstory", &cmds);
    assert!(!c_bad.ok, "expected type errors");
    assert!(
        c_bad.diags.iter().any(|d| d.code == "VST028"),
        "expected VST028, diags={:?}",
        c_bad.diags.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
    let c_good = check_source(good, "good_cmd.vstory", &cmds);
    assert!(c_good.ok, "{:?}", c_good.diags);
}

#[test]
fn product_arithmetic_and_var_refs_exact() {
    use velvet_story_lang::pipeline::run_source_product;

    let src = r#"
scene start
set score = 5
set bonus = 2
set total = score + bonus
add score bonus
set half = total / 2
end
"#;
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(src, "arith.vstory", &cmds, 0).expect("product run");
    let get = |k: &str| {
        r.vars
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(get("total"), "7", "vars={:?}", r.vars);
    assert_eq!(get("score"), "7", "vars={:?}", r.vars);
    assert_eq!(get("half"), "3", "vars={:?}", r.vars);
}

#[test]
fn command_defaults_filled_on_host_call() {
    use velvet_story::{StoryOp, StoryValue};
    use velvet_story_lang::pipeline::build_story_program;

    // difficulty and can_escape omitted → defaults 1 and true from registry.
    let src = r#"
scene start
call combat.start:
    enemy: forest_guardian
end
"#;
    let cmds = CommandRegistry::builtin();
    let prog = build_story_program(src, "def.vstory", &cmds, "def").unwrap();
    let sc = prog.scenes.get("start").unwrap();
    let host = sc
        .ops
        .iter()
        .find_map(|op| match op {
            StoryOp::HostCall { name, args } if name == "combat.start" => Some(args),
            _ => None,
        })
        .expect("HostCall");
    assert_eq!(
        host.get("enemy"),
        Some(&StoryValue::String("forest_guardian".into()))
    );
    assert_eq!(host.get("difficulty"), Some(&StoryValue::Int(1)));
    assert_eq!(host.get("can_escape"), Some(&StoryValue::Bool(true)));
}

#[test]
fn command_required_still_errors_without_enemy() {
    use velvet_story_lang::pipeline::check_source;

    let src = r#"
scene start
call combat.start:
    difficulty: 2
end
"#;
    let cmds = CommandRegistry::builtin();
    let c = check_source(src, "req.vstory", &cmds);
    assert!(!c.ok);
    assert!(
        c.diags.iter().any(|d| d.code == "VST022"),
        "expected VST022 missing required, got {:?}",
        c.diags.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn nested_if_inside_choice_product_tail_runs() {
    use velvet_story_lang::pipeline::run_source_product;

    let src = r#"
scene start
set has_key = 1
choice:
    "Entrar":
        set entered = 1
        if has_key:
            set opened = 1
        set finished = 1
end
"#;
    let cmds = CommandRegistry::builtin();
    let r = run_source_product(src, "nest.vstory", &cmds, 0).expect("product");
    let get = |k: &str| {
        r.vars
            .iter()
            .find(|(n, _)| n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    assert_eq!(get("entered"), "1");
    assert_eq!(get("opened"), "1");
    assert_eq!(
        get("finished"),
        "1",
        "tail after nested if inside choice must run; vars={:?}",
        r.vars
    );
}

#[test]
fn studio_model_includes_child_scene() {
    use tempfile::tempdir;
    use velvet_story_lang::studio::build_model;

    let dir = tempdir().unwrap();
    let child = dir.path().join("part.vstory");
    let root = dir.path().join("root.vstory");
    std::fs::write(&child, "scene child_scene\nnarrator:\n    x\nend\n").unwrap();
    let root_src = "include \"part.vstory\"\n\nscene start\nend\n";
    std::fs::write(&root, root_src).unwrap();
    let cmds = CommandRegistry::builtin();
    let model = build_model(root_src, root.to_str().unwrap(), &cmds);
    assert!(
        model.scenes.iter().any(|s| s.name == "child_scene"),
        "scenes={:?}",
        model.scenes.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
}

#[test]
fn diag_five_locales_same_code_different_text() {
    use velvet_story_lang::pipeline::{check_source_with, CheckOptions};
    use velvet_story_lang::DiagLocale;

    let src = "scene start\ngoto no_such_scene\n";
    let cmds = CommandRegistry::builtin();
    let mut displays = Vec::new();
    for loc in DiagLocale::all() {
        let c = check_source_with(
            src,
            "i18n.vstory",
            &cmds,
            CheckOptions::new().with_locale(*loc),
        );
        let d = c.diags.iter().find(|d| d.code == "VST027").expect("VST027");
        assert_eq!(d.code, "VST027");
        assert_eq!(d.locale, *loc);
        displays.push((loc.code(), d.display()));
    }
    let es = &displays.iter().find(|(c, _)| *c == "es").unwrap().1;
    let en = &displays.iter().find(|(c, _)| *c == "en").unwrap().1;
    let ja = &displays.iter().find(|(c, _)| *c == "ja").unwrap().1;
    assert!(es.contains("[VST027]"));
    assert!(en.contains("[VST027]"));
    assert_ne!(es, en);
    assert_ne!(en, ja);
    assert!(es.contains("escena") || es.contains("etiqueta"), "{es}");
    assert!(
        en.to_ascii_lowercase().contains("scene") || en.contains("does not exist"),
        "{en}"
    );
}

#[test]
fn concurrent_scoped_locales_shipped_check_path() {
    use std::sync::mpsc;
    use std::thread;
    use velvet_story_lang::pipeline::{check_source_with, CheckOptions};
    use velvet_story_lang::{set_diag_locale, DiagLocale};

    let src = "scene start\ngoto no_such_scene\n";
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    for (loc, must, must_not) in [
        (DiagLocale::En, "scene", "escena"),
        (DiagLocale::Es, "escena", "does not exist"),
    ] {
        let tx = tx.clone();
        let src = src.to_string();
        handles.push(thread::spawn(move || {
            set_diag_locale(DiagLocale::Zh);
            let cmds = CommandRegistry::builtin();
            let c = check_source_with(
                &src,
                "iso.vstory",
                &cmds,
                CheckOptions::new().with_locale(loc),
            );
            let d = c.diags.iter().find(|d| d.code == "VST027").unwrap();
            tx.send((loc, d.message.clone(), must, must_not)).unwrap();
        }));
    }
    drop(tx);
    for h in handles {
        h.join().unwrap();
    }
    set_diag_locale(DiagLocale::Es);
    while let Ok((loc, msg, must, must_not)) = rx.recv() {
        let ok = msg.contains(must) || msg.to_ascii_lowercase().contains(must);
        assert!(ok, "{loc:?}: missing `{must}` in {msg}");
        assert!(
            !msg.contains(must_not) && !msg.to_ascii_lowercase().contains(must_not),
            "{loc:?}: unexpected `{must_not}` in {msg}"
        );
    }
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
        !d.loc.file.ends_with("root.vstory") || d.loc.file.contains("child_broken"),
        "must not only blame root: {}",
        d.loc.file
    );
}

#[test]
fn logic_file_answers_values_conditions_and_jumps() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("rules.vel"),
        "// @edition 3\nfunction premio(v: map) { return v[\"oro\"] * 2 }\nfunction rico(v: map) { return v[\"oro\"] >= 3 }\nfunction siguiente(v: map) {\n    if v[\"premio\"] >= 10 { return \"tesoro\" }\n    return \"calle\"\n}\n",
    )
    .unwrap();
    let story = dir.path().join("main.vstory");
    let write = |oro: i64| {
        std::fs::write(
            &story,
            format!("logic \"rules.vel\"\n\nscene start\nset oro = {oro}\nset premio = logic.premio\nif logic.rico:\n    narrator:\n        rico\ngoto logic.siguiente\n\nscene tesoro\nnarrator:\n    en_el_tesoro\nend\n\nscene calle\nnarrator:\n    en_la_calle\nend\n"),
        )
        .unwrap();
    };
    write(5);
    let program = velvet_story_lang::load_story_program_from_path(&story).expect("program");
    assert!(program.logic.is_some());
    let r = run_story_program(program, 0, 200);
    assert!(
        r.dialogue.iter().any(|l| l.contains("rico")),
        "{:?}",
        r.dialogue
    );
    assert!(
        r.dialogue.iter().any(|l| l.contains("en_el_tesoro")),
        "{:?}",
        r.dialogue
    );
    let premio = r
        .vars
        .iter()
        .find(|(k, _)| k == "premio")
        .map(|(_, v)| v.as_str());
    assert_eq!(premio, Some("10"), "{:?}", r.vars);
    write(2);
    let r = run_story_program(
        velvet_story_lang::load_story_program_from_path(&story).unwrap(),
        0,
        200,
    );
    assert!(
        !r.dialogue.iter().any(|l| l.contains("rico")),
        "{:?}",
        r.dialogue
    );
    assert!(
        r.dialogue.iter().any(|l| l.contains("en_la_calle")),
        "{:?}",
        r.dialogue
    );
}

#[test]
fn logic_calls_are_checked_against_the_logic_file() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("rules.vel"),
        "// @edition 3\nfunction otra(v: map) { return \"\" }\n",
    )
    .unwrap();
    let story = dir.path().join("main.vstory");
    std::fs::write(
        &story,
        "logic \"rules.vel\"\n\nscene start\ngoto logic.nada\n",
    )
    .unwrap();
    let err = velvet_story_lang::load_story_program_from_path(&story)
        .unwrap_err()
        .to_string();
    assert!(err.contains("no function `nada`"), "{err}");

    let cmds = CommandRegistry::builtin();
    let c =
        velvet_story_lang::check_source("scene start\ngoto logic.siguiente\n", "x.vstory", &cmds);
    assert!(c.diags.iter().any(|d| d.code == "VST070"), "{:?}", c.diags);
    assert!(
        !c.diags.iter().any(|d| d.code == "VST027"),
        "a logic jump is not a missing scene"
    );
}

#[test]
fn logic_path_resolves_once_from_a_relative_story_path() {
    // `velvet story run stories/main.vstory`: the story's path is relative, and its logic file
    // must be read from stories/, not stories/stories/
    let dir = format!(".tmp-logic-path-{}", std::process::id());
    let stories = std::path::Path::new(&dir).join("stories");
    std::fs::create_dir_all(&stories).unwrap();
    std::fs::write(
        stories.join("rules.vel"),
        "// @edition 3\nfunction siguiente(v: map) { return \"fin\" }\n",
    )
    .unwrap();
    std::fs::write(
        stories.join("main.vstory"),
        "logic \"rules.vel\"\n\nscene start\ngoto logic.siguiente\n\nscene fin\nnarrator:\n    al_final\nend\n",
    )
    .unwrap();
    let built = velvet_story_lang::load_story_program_from_path(&stories.join("main.vstory"));
    let _ = std::fs::remove_dir_all(&dir);
    let program = built.expect("a relative story path finds its logic file");
    let r = run_story_program(program, 0, 100);
    assert!(
        r.dialogue.iter().any(|l| l.contains("al_final")),
        "{:?}",
        r.dialogue
    );
}

#[test]
fn logic_reads_and_files_are_checked_before_building() {
    let cmds = CommandRegistry::builtin();
    let c = velvet_story_lang::check_source(
        "scene start\nset premio = logic.premio\nif logic.rico:\n    narrator:\n        rico\nend\n",
        "x.vstory",
        &cmds,
    );
    let vst070 = c.diags.iter().filter(|d| d.code == "VST070").count();
    assert_eq!(vst070, 2, "a value and a condition: {:?}", c.diags);

    let c = velvet_story_lang::check_source(
        "logic reglas.vel\n\nscene start\nnarrator:\n    hola\nend\n",
        "x.vstory",
        &cmds,
    );
    let d = c.diags.iter().find(|d| d.code == "VST071").expect("VST071");
    assert!(!d.display().contains("include"), "{}", d.display());

    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let story = dir.path().join("main.vstory");
    std::fs::write(
        &story,
        "logic \"falta.vel\"\n\nscene start\nnarrator:\n    hola\nend\n",
    )
    .unwrap();
    let c = velvet_story_lang::check_path(&story, &cmds).unwrap();
    assert!(c.diags.iter().any(|d| d.code == "VST072"), "{:?}", c.diags);
}

#[test]
fn logic_errors_name_the_line_of_the_call() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("rules.vel"),
        "// @edition 3\nfunction otra(v: map) { return \"\" }\n",
    )
    .unwrap();
    let story = dir.path().join("main.vstory");
    std::fs::write(
        &story,
        "logic \"rules.vel\"\n\nscene start\nnarrator:\n    hola\nset x = logic.nada\nend\n",
    )
    .unwrap();
    let err = velvet_story_lang::load_story_program_from_path(&story)
        .unwrap_err()
        .to_string();
    assert!(err.contains("line 6"), "{err}");
}

#[test]
fn a_logic_jump_that_names_no_scene_goes_on() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    let story = dir.path().join("main.vstory");
    std::fs::write(
        &story,
        "logic \"rules.vel\"\n\nscene start\ngoto logic.siguiente\nnarrator:\n    sigue\nend\n\nscene tesoro\nnarrator:\n    en_el_tesoro\nend\n",
    )
    .unwrap();
    for answer in ["\"ninguna\"", "[\"tesoro\"]", "7"] {
        std::fs::write(
            dir.path().join("rules.vel"),
            format!("// @edition 3\nfunction siguiente(v: map) {{ return {answer} }}\n"),
        )
        .unwrap();
        let program = velvet_story_lang::load_story_program_from_path(&story).expect("program");
        let r = run_story_program(program, 0, 100);
        assert!(
            r.dialogue.iter().any(|l| l.contains("sigue")),
            "{answer}: {:?}",
            r.dialogue
        );
        assert!(
            !r.dialogue.iter().any(|l| l.contains("en_el_tesoro")),
            "{answer}: {:?}",
            r.dialogue
        );
    }
}

#[test]
fn a_logic_jump_can_lead_to_any_scene_and_the_logic_is_not_in_the_hash() {
    use tempfile::tempdir;
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("rules.vel"),
        "// @edition 3\nfunction siguiente(v: map) { return \"tesoro\" }\n",
    )
    .unwrap();
    let story = dir.path().join("main.vstory");
    std::fs::write(
        &story,
        "logic \"rules.vel\"\n\nscene start\ngoto logic.siguiente\n\nscene tesoro\nnarrator:\n    t\nend\n\nscene calle\nnarrator:\n    c\nend\n",
    )
    .unwrap();
    let program = velvet_story_lang::load_story_program_from_path(&story).expect("program");
    let map = velvet_story::BranchMap::analyze(&program);
    assert!(
        map.unreachable_scenes.is_empty(),
        "{:?}",
        map.unreachable_scenes
    );
    assert!(
        map.unreachable_endings.is_empty(),
        "{:?}",
        map.unreachable_endings
    );
    let mut without = program.clone();
    without.logic = None;
    assert_eq!(
        program.content_hash(),
        without.content_hash(),
        "saves don't depend on the logic file"
    );
}
