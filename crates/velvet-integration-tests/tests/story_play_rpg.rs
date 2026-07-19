//! Cross-crate: story choices unlock play interactions and RPG quest progress.

use velvet_math::Vec2;
use velvet_play::prelude::*;
use velvet_rpg::prelude::*;
use velvet_story::prelude::*;

fn sample_story() -> StoryProgram {
    let src = r##"
character guide { name: "Guide" color: "#88aaff" }
state {
    has_key: bool = false
    mercy: int = 0
}
scene start {
    guide "Take the key?"
    choice {
        "Yes" {
            has_key = true
            jump aftermath
        }
        "No" {
            has_key = false
            jump aftermath
        }
    }
}
scene aftermath {
    guide "Proceed."
}
"##;
    load_program_from_source(src, Some("integration.vel"), "Integration").expect("load story")
}

fn run_to_choice(player: &mut StoryPlayer) {
    let mut steps = 0;
    while player.wait() != &StoryWait::Choice && steps < 64 {
        player.advance();
        steps += 1;
    }
    assert_eq!(player.wait(), &StoryWait::Choice, "should reach choice");
}

fn drain_story(player: &mut StoryPlayer) {
    let mut steps = 0;
    while player.wait() != &StoryWait::Ended && steps < 128 {
        match player.wait() {
            StoryWait::Line | StoryWait::Ready => player.advance(),
            StoryWait::Choice => break,
            StoryWait::Ended => break,
            StoryWait::Pause { .. } | StoryWait::Host { .. } => break,
        }
        steps += 1;
    }
}

fn has_bool(player: &StoryPlayer, name: &str) -> bool {
    matches!(player.variables().get(name), StoryValue::Bool(true))
}

#[test]
fn story_choice_sets_key_flag() {
    let mut player = StoryPlayer::start(sample_story());
    run_to_choice(&mut player);
    player.choose(0).expect("choose yes");
    drain_story(&mut player);
    assert!(has_bool(&player, "has_key"));
}

#[test]
fn story_flag_enables_door_interaction() {
    let mut player = StoryPlayer::start(sample_story());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    drain_story(&mut player);
    assert!(has_bool(&player, "has_key"));

    let map = TileMap::from_ascii(
        "\
#####
#...#
#.D.#
#...#
#####",
        16.0,
    )
    .expect("map");
    let mut world = PlayWorld::new(map);
    let _player_id = world.spawn_player(Vec2::new(24.0, 24.0), 120.0);
    // Door entity with interaction gated by story flag
    world.spawn(PlayEntity {
        id: 0,
        transform: velvet_math::Transform2D::from_translation(Vec2::new(40.0, 32.0)),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(8.0))),
        kinematic: None,
        speed: None,
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: if has_bool(&player, "has_key") {
            Some(Interactable::new("door", 24.0))
        } else {
            None
        },
        alive: true,
    });

    world.try_player_interact(true);
    assert!(
        !world.interact_events.is_empty(),
        "key holders can open the door"
    );
}

#[test]
fn save_roundtrip_preserves_variables() {
    let program = sample_story();
    let mut player = StoryPlayer::start(program.clone());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    drain_story(&mut player);

    let save = player.to_save("slot1");
    let json = serde_json::to_string(&save).expect("serialize");
    let loaded: SaveGame = serde_json::from_str(&json).expect("deserialize");

    let mut player2 = StoryPlayer::start(program);
    player2.load_save(loaded).expect("load");
    assert!(has_bool(&player2, "has_key"));
}

#[test]
fn rpg_quest_tracks_story_outcome() {
    let mut journal = QuestJournal::default();
    let mut quest = Quest::new("find_key", "Find the key");
    quest
        .objectives
        .push(QuestObjective::new("pickup", "Pick up key", 1));
    journal.start(quest);

    let mut player = StoryPlayer::start(sample_story());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    drain_story(&mut player);

    if has_bool(&player, "has_key") {
        journal.progress("find_key", "pickup", 1);
    }
    assert!(journal.is_completed("find_key"));
}

#[test]
fn action_score_after_story_fight_path() {
    use velvet_action::prelude::*;

    let mut player = StoryPlayer::start(sample_story());
    run_to_choice(&mut player);
    // Choose "No" — no key path
    player.choose(1).unwrap();
    drain_story(&mut player);
    assert!(!has_bool(&player, "has_key"));

    let mut score = ScoreBoard::default();
    score.add_kill(100);
    assert!(score.score >= 100);
    assert_eq!(score.kills, 1);
}
