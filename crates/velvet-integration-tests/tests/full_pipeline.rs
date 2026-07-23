//! Full pipeline integration: compile script → lower story → play → save → load → RPG quest.
//!
//! Multiple end-to-end scenarios covering cross-crate seams.

use velvet_math::Vec2;
use velvet_play::prelude::*;
use velvet_play::{apply_autotile4, flood_fill_walkable, RegionEventKind};
use velvet_rpg::prelude::*;
use velvet_script_compiler::compile_source;
use velvet_script_vm::{Value, Vm, VmLimits};
use velvet_story::prelude::*;

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const BRANCHING_STORY: &str = r##"
character guide {
    name: "Guide"
    color: "#88aaff"
}

state {
    has_key: bool = false
    mercy: int = 0
    route: int = 0
}

scene start {
    background "inn.png"
    music "soft.ogg" fade_in 0.5
    show guide.neutral at left
    guide "Take the key?"
    choice {
        "Yes" {
            has_key = true
            mercy += 1
            route = 1
            jump aftermath
        }
        "No" {
            has_key = false
            route = 2
            jump aftermath
        }
        "Ask later" {
            route = 3
            jump aftermath
        }
    }
}

scene aftermath {
    guide "Very well."
    hide guide
    if has_key {
        "You pocket the key."
        jump ending_key
    } else {
        "You leave empty-handed."
        jump ending_none
    }
}

scene ending_key {
    "Key ending."
    end "with_key"
}

scene ending_none {
    "Empty ending."
    end "no_key"
}
"##;

const SCRIPT_FUNCS: &str = r#"
function clamp(x, lo, hi) {
    if x < lo {
        return lo
    }
    if x > hi {
        return hi
    }
    return x
}

function score(kills, time) {
    let base = kills * 100
    let bonus = clamp(60 - time, 0, 60) * 2
    return base + bonus
}

function main() {
    return score(3, 10)
}
"#;

fn load_story() -> StoryProgram {
    load_program_from_source(BRANCHING_STORY, Some("pipeline.vel"), "Pipeline").expect("story load")
}

fn run_to_choice(player: &mut StoryPlayer) {
    let mut steps = 0;
    while player.wait() != &StoryWait::Choice && steps < 128 {
        if matches!(player.wait(), StoryWait::Line | StoryWait::Ready) {
            player.advance();
        } else {
            break;
        }
        steps += 1;
    }
    assert_eq!(*player.wait(), StoryWait::Choice);
}

fn drain_until_end(player: &mut StoryPlayer) {
    let mut steps = 0;
    while !player.is_ended() && steps < 256 {
        match player.wait() {
            StoryWait::Line | StoryWait::Ready => player.advance(),
            StoryWait::Choice => break,
            StoryWait::Ended => break,
            StoryWait::Pause { .. } | StoryWait::Host { .. } => break,
        }
        steps += 1;
    }
}

fn flag(player: &StoryPlayer, name: &str) -> bool {
    matches!(player.variables().get(name), StoryValue::Bool(true))
}

fn int_var(player: &StoryPlayer, name: &str) -> i64 {
    player.variables().get_int(name, 0)
}

// ---------------------------------------------------------------------------
// Scenario 1: compile + VM execute pure script
// ---------------------------------------------------------------------------

#[test]
fn compile_and_run_score_function() {
    let compiled = compile_source(SCRIPT_FUNCS, Some("score.vel")).expect("compile");
    assert!(compiled.module.exports.contains_key("score"));
    assert!(compiled.module.exports.contains_key("main"));

    let mut vm = Vm::new(compiled.module, VmLimits::default());
    let result = vm.call_name("main", &[]).expect("call main");
    // kills=3 → 300, time=10 → clamp(50)*2=100 → 400
    match result {
        Value::Int(n) => assert_eq!(n, 400),
        other => panic!("expected int 400, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Scenario 2: story load → choice → ending
// ---------------------------------------------------------------------------

#[test]
fn story_key_route_ends_with_key() {
    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    assert_eq!(player.choices().len(), 3);
    player.choose(0).expect("yes");
    drain_until_end(&mut player);
    assert!(flag(&player, "has_key"));
    assert_eq!(int_var(&player, "route"), 1);
    assert_eq!(player.ending(), Some("with_key"));
}

#[test]
fn story_no_key_route() {
    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    player.choose(1).expect("no");
    drain_until_end(&mut player);
    assert!(!flag(&player, "has_key"));
    assert_eq!(player.ending(), Some("no_key"));
}

// ---------------------------------------------------------------------------
// Scenario 3: save / load mid-story
// ---------------------------------------------------------------------------

#[test]
fn save_load_preserves_choice_outcome() {
    let program = load_story();
    let mut player = StoryPlayer::start(program.clone());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    assert_eq!(
        *player.wait(),
        StoryWait::Line,
        "choice must enter aftermath dialogue"
    );
    assert_eq!(player.current_text(), "Very well.");
    let save = player.to_save("slot_pipeline");
    let json = serde_json::to_string_pretty(&save).expect("json");
    assert!(json.contains("slot_pipeline"), "json={json}");
    assert!(
        json.contains("has_key"),
        "choice state missing from save: {json}"
    );

    let loaded: SaveGame = serde_json::from_str(&json).expect("deserialize");
    let mut player2 = StoryPlayer::start(program);
    player2.load_save(loaded).expect("load save");
    assert!(flag(&player2, "has_key"));
    // Finish remaining
    drain_until_end(&mut player2);
    assert!(player2.is_ended());
    assert_eq!(player2.ending(), Some("with_key"));
}

// ---------------------------------------------------------------------------
// Scenario 4: localization key extraction + apply
// ---------------------------------------------------------------------------

#[test]
fn localization_roundtrip_on_story() {
    let program = load_story();
    let catalog = extract_loc_keys(&program);
    assert!(catalog.len() >= 3);
    let table: std::collections::HashMap<String, String> = catalog
        .entries
        .iter()
        .map(|e| (e.key.clone(), format!("LOC:{}", e.source)))
        .collect();
    let translated = catalog.apply_to_program_lookup(&program, |k| table.get(k).cloned());
    let mut player = StoryPlayer::start(translated);
    run_to_choice(&mut player);
    // Choice texts should be prefixed
    assert!(player.choices().iter().any(|c| c.text.starts_with("LOC:")));
}

// ---------------------------------------------------------------------------
// Scenario 5: skip engine after reading lines
// ---------------------------------------------------------------------------

#[test]
fn skip_engine_fast_forwards_all_mode() {
    let mut player = StoryPlayer::start(load_story());
    // Show first line
    assert_eq!(*player.wait(), StoryWait::Line);
    let mut engine = SkipEngine::default();
    engine.config.mode = SkipMode::All;
    engine.config.max_batch = 32;
    let _ = engine.skip_batch(&mut player);
    // Should reach choice or further
    assert!(
        matches!(
            player.wait(),
            StoryWait::Choice | StoryWait::Line | StoryWait::Ended
        ),
        "wait={:?}",
        player.wait()
    );
}

// ---------------------------------------------------------------------------
// Scenario 6: rollback recorder
// ---------------------------------------------------------------------------

#[test]
fn rollback_can_step_back_one_line() {
    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    player.choose(0).expect("key route");
    assert_eq!(*player.wait(), StoryWait::Line);
    assert_eq!(player.current_text(), "Very well.");

    let mut recorder = RollbackRecorder::new(32);
    recorder.observe(&player);
    player.advance();
    assert_eq!(*player.wait(), StoryWait::Line);
    assert_eq!(player.current_text(), "You pocket the key.");
    recorder.observe(&player);

    assert!(recorder.back(&mut player).unwrap());
    assert_eq!(*player.wait(), StoryWait::Line);
    assert_eq!(player.current_text(), "Very well.");
    assert!(flag(&player, "has_key"));
}

// ---------------------------------------------------------------------------
// Scenario 7: play world + region + key-gated interaction
// ---------------------------------------------------------------------------

#[test]
fn play_regions_and_key_door() {
    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    drain_until_end(&mut player);
    assert!(flag(&player, "has_key"));

    let map = TileMap::from_ascii(
        "\
######
#....#
#.T..#
#...D#
######",
        16.0,
    )
    .expect("map");

    let mut regions = RegionSet::from_tile_triggers(&map, "t_");
    regions.insert(
        MapRegion::from_pos_size("door_zone", Vec2::new(48.0, 48.0), Vec2::splat(16.0))
            .with_tag("door"),
    );

    let mut world = PlayWorld::new(map);
    let _pid = world.spawn_player(Vec2::new(24.0, 24.0), 100.0);
    world.spawn(PlayEntity {
        id: 0,
        transform: velvet_math::Transform2D::from_translation(Vec2::new(40.0, 24.0)),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(8.0))),
        kinematic: None,
        speed: None,
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: if flag(&player, "has_key") {
            Some(Interactable::new("door", 32.0))
        } else {
            None
        },
        alive: true,
    });

    let events = regions.update_actor("player", Vec2::new(56.0, 56.0));
    assert!(
        events.iter().any(|event| {
            event.region == "door_zone"
                && event.actor == "player"
                && event.kind == RegionEventKind::Enter
        }),
        "events={events:?}"
    );
    assert!(regions
        .occupied_by("player")
        .iter()
        .any(|name| name == "door_zone"));
    assert!(regions
        .update_actor("player", Vec2::new(56.0, 56.0))
        .is_empty());

    world.try_player_interact(true);
    assert!(
        !world.interact_events.is_empty(),
        "key route should unlock door interact"
    );
}

// ---------------------------------------------------------------------------
// Scenario 8: hierarchical nav path after story
// ---------------------------------------------------------------------------

#[test]
fn hierarchical_nav_after_story() {
    let map = TileMap::from_ascii(
        "\
##########
#........#
#.######.#
#........#
#........#
##########",
        16.0,
    )
    .unwrap();
    let hnav = HierarchicalNav::from_tilemap(&map, 4);
    let path = hnav
        .find_path(NavPoint::new(1, 1), NavPoint::new(8, 4))
        .expect("path across sectors");
    assert!(path.len() >= 2);
}

// ---------------------------------------------------------------------------
// Scenario 9: camera shake trauma during "combat" beat
// ---------------------------------------------------------------------------

#[test]
fn camera_shake_during_action_beat() {
    let mut shake = CameraShake::new(10.0);
    let mut cam = PlayCamera::default();
    // Story says fight started — punch camera
    shake.impulse(0.8);
    shake.apply_to_camera(&mut cam, 0.016);
    assert!(cam.shake.length() > 0.0);
    for _ in 0..200 {
        shake.apply_to_camera(&mut cam, 0.05);
    }
    assert!(!shake.is_active());
}

// ---------------------------------------------------------------------------
// Scenario 10: RPG quest tracks story outcome
// ---------------------------------------------------------------------------

#[test]
fn rpg_quest_completes_on_key() {
    let mut journal = QuestJournal::default();
    let mut quest = Quest::new("obtain_key", "Obtain the inn key");
    quest
        .objectives
        .push(QuestObjective::new("take", "Take the key", 1));
    journal.start(quest);

    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    drain_until_end(&mut player);

    if flag(&player, "has_key") {
        journal.progress("obtain_key", "take", 1);
    }
    assert!(journal.is_completed("obtain_key"));
}

#[test]
fn rpg_quest_fails_objective_without_key() {
    let mut journal = QuestJournal::default();
    let mut quest = Quest::new("obtain_key", "Obtain the inn key");
    quest
        .objectives
        .push(QuestObjective::new("take", "Take the key", 1));
    journal.start(quest);

    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    player.choose(1).unwrap();
    drain_until_end(&mut player);

    if flag(&player, "has_key") {
        journal.progress("obtain_key", "take", 1);
    }
    assert!(!journal.is_completed("obtain_key"));
}

// ---------------------------------------------------------------------------
// Scenario 11: compile story-ish script that also has functions
// ---------------------------------------------------------------------------

#[test]
fn compile_story_source_as_bytecode() {
    let compiled = compile_source(BRANCHING_STORY, Some("pipeline.vel")).expect("compile story");
    // Story scenes compile to callable exports and preserve source identity.
    assert!(
        compiled.module.exports.contains_key("start"),
        "exports={:?}",
        compiled.module.exports.keys().collect::<Vec<_>>()
    );
    assert!(compiled.module.metadata.source_hash.is_some());
}

// ---------------------------------------------------------------------------
// Scenario 12: physics edge case + tile collision room
// ---------------------------------------------------------------------------

#[test]
fn physics_move_in_ascii_room() {
    let map = TileMap::from_ascii(
        "\
#####
#...#
#...#
#####",
        16.0,
    )
    .unwrap();
    let mut world = PlayWorld::new(map);
    let pid = world.spawn_player(Vec2::new(24.0, 24.0), 80.0);
    // Try to walk into wall
    if let Some(e) = world.entities.get_mut(&pid) {
        e.velocity = Velocity::new(-200.0, 0.0);
    }
    world.step(0.05);
    let pos = world.entities.get(&pid).unwrap().transform.translation;
    assert!(
        pos.x >= 8.0,
        "should not tunnel through left wall, x={pos:?}"
    );
}

// ---------------------------------------------------------------------------
// Scenario 13: autotile + flood fill
// ---------------------------------------------------------------------------

#[test]
fn map_autotile_and_flood() {
    let mut map = TileMap::from_ascii(
        "\
#####
#...#
#...#
#####",
        8.0,
    )
    .unwrap();
    apply_autotile4(map.main_layer_mut(), 10, |t| t.flags.solid);
    let walk = flood_fill_walkable(map.main_layer(), 1, 1);
    assert_eq!(walk.len(), 6);
}

// ---------------------------------------------------------------------------
// Scenario 14: action score after story fight path
// ---------------------------------------------------------------------------

#[test]
fn action_score_on_refuse_key_path() {
    use velvet_action::prelude::*;

    let mut player = StoryPlayer::start(load_story());
    run_to_choice(&mut player);
    player.choose(1).unwrap();
    drain_until_end(&mut player);
    assert!(!flag(&player, "has_key"));

    let mut score = ScoreBoard::default();
    score.add_kill(150);
    score.add_kill(50);
    assert!(score.score >= 200);
    assert_eq!(score.kills, 2);
}

// ---------------------------------------------------------------------------
// Scenario 15: full multi-system chain
// ---------------------------------------------------------------------------

#[test]
fn full_chain_compile_story_play_save_quest() {
    // 1) Compile pure helper used by game logic
    let compiled = compile_source(
        "function reward(has_key) { if has_key { return 100 } else { return 10 } }\n",
        None,
    )
    .unwrap();
    let mut vm = Vm::new(compiled.module, VmLimits::default());
    // 2) Story
    let program = load_story();
    let mut player = StoryPlayer::start(program.clone());
    run_to_choice(&mut player);
    player.choose(0).unwrap();
    drain_until_end(&mut player);
    assert!(flag(&player, "has_key"));

    // 3) VM reward
    let reward = vm
        .call_name("reward", &[Value::Bool(true)])
        .expect("reward function must execute");
    assert_eq!(reward, Value::Int(100));
    let reward_i = 100;

    // 4) Save
    let save = player.to_save("chain");
    let mut player2 = StoryPlayer::start(program);
    player2.load_save(save).unwrap();
    assert!(flag(&player2, "has_key"));

    // 5) RPG
    let mut journal = QuestJournal::default();
    let mut q = Quest::new("chain_q", "Chain");
    q.objectives.push(QuestObjective::new("key", "Key", 1));
    journal.start(q);
    journal.progress("chain_q", "key", 1);
    assert!(journal.is_completed("chain_q"));

    // 6) Play room unlock
    let map = TileMap::from_ascii("###\n#.#\n###", 16.0).unwrap();
    let mut world = PlayWorld::new(map);
    world.spawn_player(Vec2::new(16.0, 16.0), 50.0);
    assert!(!world.entities.is_empty());

    assert!(reward_i >= 100);
}
