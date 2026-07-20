//! End-to-end classic `.vel` product-path tests (phases 1–5).
//!
//! These drive the **shipped** load → player → session APIs — not reimplemented IR.

#![cfg(test)]

use crate::host::command_host_continue;
use crate::load::load_program_from_source;
use crate::localization_hook::{extract_loc_keys, LocKind};
use crate::product::{join_dialogue_lines, say_plain_and_cps, VnSession};
use crate::runtime::{StoryPlayer, StoryWait};
use crate::value::StoryValue;
use crate::vs3_bridge::call_vs3_logic;

fn pump_to_end(player: &mut StoryPlayer, max: usize) {
    let mut steps = 0;
    loop {
        steps += 1;
        assert!(steps < max, "stuck wait={:?}", player.wait());
        match player.wait().clone() {
            StoryWait::Line | StoryWait::Ready => player.advance(),
            StoryWait::Choice => {
                let _ = player.choose(0);
            }
            StoryWait::Ended => break,
            StoryWait::Pause { .. } => {
                player.tick(1.0); // advance pause clock
            }
            StoryWait::Host { token } => {
                let _ = player.resume_host(&token);
            }
            other => panic!("unexpected wait {other:?}"),
        }
    }
}

#[test]
fn phase3_presentation_state_via_vn_session() {
    let src = r#"
character nora { name: "Nora" }
scene main {
    background "bg/station.png"
    transition fade
    show nora.happy at left
    sound "sfx/train.ogg"
    nora "At the platform."
    hide nora
    end
}
"#;
    let program = load_program_from_source(src, Some("pres.vel"), "P").unwrap();
    let mut session = VnSession::new(StoryPlayer::start(program));
    // Pump until dialogue
    let mut g = 0;
    while !matches!(session.player().wait(), StoryWait::Line) && g < 30 {
        session.advance();
        g += 1;
    }
    assert_eq!(
        session.presentation.background.as_deref(),
        Some("bg/station.png")
    );
    assert!(
        session.presentation.sprites.contains_key("nora"),
        "sprites={:?}",
        session.presentation.sprites
    );
    let nora = &session.presentation.sprites["nora"];
    assert_eq!(nora.expression.as_deref(), Some("happy"));
    assert_eq!(nora.at.as_deref(), Some("left"));
    assert_eq!(
        session.presentation.last_transition_name.as_deref(),
        Some("fade")
    );
    assert!(
        !session.presentation.transitions.is_empty()
            || session.presentation.last_transition_name.is_some()
    );
    assert_eq!(
        session.presentation.last_sfx.as_deref(),
        Some("sfx/train.ogg")
    );
    // say screen shows plain dialogue
    assert!(session.say.visible);
    assert!(session.say.full_text.contains("platform"));

    // Continue to hide
    session.say.reveal_all();
    session.advance();
    let mut g = 0;
    while !matches!(session.player().wait(), StoryWait::Ended) && g < 20 {
        if matches!(session.player().wait(), StoryWait::Line) {
            session.say.reveal_all();
        }
        session.advance();
        g += 1;
    }
    assert!(
        !session.presentation.sprites.contains_key("nora"),
        "hide should remove sprite"
    );
}

#[test]
fn phase4_multiline_and_markup_and_loc() {
    let multi = join_dialogue_lines(&[
        "First line of thought.",
        "Second line still hers.",
    ]);
    assert!(multi.contains('\n'));
    assert_eq!(multi.lines().count(), 2);

    let (plain, cps) =
        say_plain_and_cps("{cps=24}Hello {b}world{/b} {color=#ff0}bright{/color}\nnext");
    assert_eq!(cps, Some(24.0));
    assert!(!plain.contains('{'), "tags stripped: {plain}");
    assert!(plain.contains("Hello"));
    assert!(plain.contains("world"));
    assert!(plain.contains("bright"));
    assert!(plain.contains('\n') || plain.contains("next"));

    let (p2, _) = say_plain_and_cps("{w=0.3}Wait… {i}soft{/i}");
    assert!(!p2.contains("{w"));
    assert!(p2.contains("Wait"));
    assert!(p2.contains("soft"));

    let src = r#"
character hero { name: "Hero" }
scene main {
    hero "Hello {b}there{/b}"
    choice {
        "Yes" { jump end }
        "No" { jump end }
    }
}
scene end {
    "Done"
    end
}
"#;
    let program = load_program_from_source(src, Some("loc.vel"), "L").unwrap();
    let cat = extract_loc_keys(&program);
    assert!(!cat.is_empty());
    assert!(cat.entries.iter().any(|e| e.kind == LocKind::Dialogue));
    assert!(cat.entries.iter().any(|e| e.kind == LocKind::Choice));
    // Apply Spanish table
    let mut table = indexmap::IndexMap::new();
    for e in &cat.entries {
        if e.kind == LocKind::Dialogue {
            table.insert(e.key.clone(), "Hola allí".into());
        }
    }
    let translated = cat.apply_to_program(&program, &table);
    let mut session = VnSession::new(StoryPlayer::start(translated));
    let mut g = 0;
    while !matches!(session.player().wait(), StoryWait::Line) && g < 20 {
        session.advance();
        g += 1;
    }
    assert!(
        session.say.full_text.contains("Hola") || session.say.visible_text.contains("Hola"),
        "got '{}'",
        session.say.full_text
    );
}

#[test]
fn phase5_host_ui_flag_no_draw_and_vs3() {
    let src = r#"
state { score: int = 0 }
scene main {
    call ui.flag name "say_visible" on true
    call game.add_score amount 5
    "ok"
    end
}
"#;
    let program = load_program_from_source(src, Some("host.vel"), "H").unwrap();
    let host = command_host_continue(|name, args, vars| {
        if name == "ui.flag" {
            let flag = args
                .get("name")
                .map(|v| v.display_str())
                .unwrap_or_default();
            let on = args.get("on").map(|v| v.is_truthy()).unwrap_or(false);
            vars.set(format!("ui.{flag}"), StoryValue::Bool(on));
            return Ok(());
        }
        if name == "game.add_score" {
            let n = args.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let cur = vars.get_int("score", 0);
            vars.set("score", StoryValue::Int(cur + n));
            return Ok(());
        }
        // Reject any draw-like command
        if name.starts_with("draw") || name == "fill_rect" {
            return Err(crate::host::StoryCommandError::new(
                "draw API not allowed in story host",
            ));
        }
        Ok(())
    });
    let mut player = StoryPlayer::start_with_host(program, host);
    pump_to_end(&mut player, 30);
    assert!(player.variables().get("ui.say_visible").is_truthy());
    assert_eq!(player.variables().get_int("score", 0), 5);
    assert_eq!(
        player.variables().get("__last_command").display_str(),
        "game.add_score"
    );

    // VS3 pure logic used by host (same as bridge tests)
    let v = call_vs3_logic(
        r#"// @edition 3
function bump(x) { return x + 1 }
"#,
        Some("b.vel"),
        "bump",
        &[StoryValue::Int(41)],
    )
    .unwrap();
    assert_eq!(v, StoryValue::Int(42));
}
