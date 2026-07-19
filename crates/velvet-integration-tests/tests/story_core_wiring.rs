//! Cross-crate wiring: boot, combat host, dialogue bridge, presentation.

use indexmap::IndexMap;
use velvet_action::{finish_combat, CombatStoryHost};
use velvet_rpg::{DialogueBridge, DialogueMapping, DialogueResolveContext};
use velvet_story::{
    AssignOp, BgmIntent, StoryExpr, StoryOp, StoryPlayer, StoryProgram, StoryScene, StoryValue,
    StoryWait, VnSession,
};
use velvet_story_lang::run_story_path_headless;

#[test]
fn combat_host_from_action_crate_suspends_resume() {
    use std::sync::Arc;

    let host = Arc::new(CombatStoryHost::new());
    let shared: velvet_story::SharedCommandHost = host.clone();

    let mut scenes = IndexMap::new();
    scenes.insert(
        "start".into(),
        StoryScene {
            name: "start".into(),
            ops: vec![
                StoryOp::HostCall {
                    name: "combat.start".into(),
                    args: {
                        let mut m = IndexMap::new();
                        m.insert("enemy".into(), StoryValue::String("wolf".into()));
                        m.insert("difficulty".into(), StoryValue::Int(2));
                        m
                    },
                },
                StoryOp::Assign {
                    name: "post".into(),
                    assign_op: AssignOp::Set,
                    value: StoryExpr::value(StoryValue::Int(1)),
                },
                StoryOp::End {
                    ending: Some("ok".into()),
                },
            ],
            labels: IndexMap::new(),
        },
    );
    let mut prog = StoryProgram::new("it_combat");
    prog.entry = "start".into();
    prog.scenes = scenes;

    let mut player = StoryPlayer::start_with_host(prog, shared);
    assert!(matches!(
        player.wait(),
        StoryWait::Host { token } if token == CombatStoryHost::WAIT_TOKEN
    ));
    assert_eq!(player.variables().get_int("post", 0), 0);
    finish_combat(player.variables_mut(), true);
    player
        .resume_host(CombatStoryHost::WAIT_TOKEN)
        .expect("resume");
    assert_eq!(player.variables().get_int("post", 0), 1);
    assert_eq!(player.ending(), Some("ok"));
}

#[test]
fn dialogue_bridge_starts_story_scene() {
    let mut bridge = DialogueBridge::new();
    bridge.register(DialogueMapping::new("npc_a", "talk_a"));

    let mut scenes = IndexMap::new();
    scenes.insert(
        "town".into(),
        StoryScene {
            name: "town".into(),
            ops: vec![StoryOp::End { ending: None }],
            labels: IndexMap::new(),
        },
    );
    scenes.insert(
        "talk_a".into(),
        StoryScene {
            name: "talk_a".into(),
            ops: vec![
                StoryOp::Dialogue {
                    speaker: None,
                    text: "Bridge line".into(),
                },
                StoryOp::End { ending: None },
            ],
            labels: IndexMap::new(),
        },
    );
    let mut prog = StoryProgram::new("it_bridge");
    prog.entry = "town".into();
    prog.scenes = scenes;
    let mut player = StoryPlayer::start(prog);
    let scene = bridge
        .start_dialogue("npc_a", &DialogueResolveContext::default(), &mut player)
        .unwrap();
    assert_eq!(scene, "talk_a");
    assert_eq!(player.scene_name(), "talk_a");
    assert!(player.current_text().contains("Bridge line"));
}

#[test]
fn product_music_sound_intents_after_ingest() {
    let mut scenes = IndexMap::new();
    scenes.insert(
        "start".into(),
        StoryScene {
            name: "start".into(),
            ops: vec![
                StoryOp::Music {
                    path: "m.ogg".into(),
                    fade_in: None,
                },
                StoryOp::Sound {
                    path: "s.ogg".into(),
                },
                StoryOp::End { ending: None },
            ],
            labels: IndexMap::new(),
        },
    );
    let mut prog = StoryProgram::new("it_audio");
    prog.entry = "start".into();
    prog.scenes = scenes;
    let mut session = VnSession::new(StoryPlayer::start(prog));
    session.ingest_events();
    let intents = session.bgm.drain_intents();
    assert!(intents
        .iter()
        .any(|i| matches!(i, BgmIntent::Play { path, .. } if path == "m.ogg")));
    assert_eq!(session.presentation.last_sfx.as_deref(), Some("s.ogg"));
}

#[test]
fn boot_vstory_path_headless_nonempty_dialogue() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut f = NamedTempFile::with_suffix(".vstory").unwrap();
    write!(
        f,
        "scene start\nnarrator:\n    Integration boot line.\nend\n"
    )
    .unwrap();
    let r = run_story_path_headless(f.path(), 0, 64).expect("boot");
    assert!(
        r.dialogue
            .iter()
            .any(|l| l.contains("Integration boot line")),
        "{:?}",
        r.dialogue
    );
}
