//! Hybrid demo: explore map, fight, talk, choices change map state and ending.

use anyhow::Result;
use velvet_action::prelude::*;
use velvet_math::{Transform2D, Vec2};
use velvet_play::prelude::*;
use velvet_rpg::prelude::*;
use velvet_story::prelude::*;

fn main() -> Result<()> {
    velvet_core::init_tracing_default("hybrid=info,info");

    // Map with a blocked door that can open via story flag
    let map = TileMap::from_ascii(
        "\
############
#..........#
#..##..##..#
#..#....#..#
#..#....D..#
#..##..##..#
#..........#
#....N.....#
############",
        16.0,
    )?;

    let mut world = PlayWorld::new(map);
    let player = world.spawn_player(Vec2::new(32.0, 32.0), 160.0);

    // NPC for dialogue
    world.spawn(PlayEntity {
        id: 0,
        transform: Transform2D::from_translation(Vec2::new(80.0, 112.0)),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(6.0))),
        kinematic: None,
        speed: None,
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: Some(Interactable::new("talk", 24.0)),
        alive: true,
    });

    // Enemy
    let enemy_id = world.spawn(PlayEntity {
        id: 0,
        transform: Transform2D::from_translation(Vec2::new(140.0, 48.0)),
        velocity: Velocity::ZERO,
        collider: Some(Collider {
            layer: CollisionLayer::ENEMY,
            ..Collider::aabb(Vec2::splat(6.0))
        }),
        kinematic: Some(KinematicBody::default()),
        speed: Some(Speed(50.0)),
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: None,
        alive: true,
    });
    let mut enemy_hp = Health::full(30.0);
    let mut enemy_ai = EnemyAi::guard(PatrolPath::new(vec![
        Vec2::new(140.0, 48.0),
        Vec2::new(100.0, 48.0),
    ]));
    let mut perception = Perception::default();

    // Story with choice that sets key_found
    let story_src = r#"
character guide { name: "Nyx" }
state {
    key_found: bool = false
    mercy: int = 0
}
scene meet {
    guide "A thug blocks the east door. Will you fight or negotiate?"
    choice {
        "Fight" {
            jump fought
        }
        "Negotiate" {
            mercy += 1
            key_found = true
            guide "He drops the key. The door unlocks in your mind."
            jump after
        }
    }
}
scene fought {
    guide "Blood on the tiles. The door still needs a key... somehow you find one."
    key_found = true
    jump after
}
scene after {
    guide "Continue east. The ending depends on your mercy."
}
"#;
    let program = load_program_from_source(story_src, Some("hybrid.vel"), "Hybrid")?;
    let mut story = StoryPlayer::start(program);
    let mut story_done = false;
    let mut fought = false;

    let mut weapon = Weapon::melee("knife", 15.0, 28.0, 0.35);
    let mut score = ScoreBoard::default();
    let mut projectiles = ProjectileSystem::default();

    let mut party = Party::default();
    party.add(PartyMember::new("hero", "Runner"));

    // Phase 1: walk to NPC and talk (auto negotiate = choice 1)
    for _ in 0..200 {
        let pos = world.entities.get(&player).unwrap().position();
        if !story_done {
            let dir = (Vec2::new(80.0, 112.0) - pos).normalize_or_zero();
            world.set_player_input(dir);
            world.step(1.0 / 60.0);
            if (pos - Vec2::new(80.0, 112.0)).length() < 28.0 {
                world.try_player_interact(true);
                if world.interact_events.iter().any(|e| e.action == "talk") {
                    // Auto-play story: pick negotiate
                    while !story.is_ended() {
                        match story.wait().clone() {
                            StoryWait::Line => story.advance(),
                            StoryWait::Choice => {
                                // 1 = negotiate if available, else 0
                                let idx = if story.choices().len() > 1 { 1 } else { 0 };
                                if idx == 0 {
                                    fought = true;
                                }
                                let _ = story.choose(idx);
                            }
                            StoryWait::Ended => break,
                            StoryWait::Ready | StoryWait::Pause { .. } => story.advance(),
                            StoryWait::Host { token } => {
                                let _ = story.resume_host(&token);
                            }
                        }
                    }
                    story_done = true;
                }
            }
        } else {
            break;
        }
    }

    let key = story.variables().get("key_found").is_truthy();
    let mercy = story.variables().get_int("mercy", 0);
    println!("story_done={story_done} key_found={key} mercy={mercy} fought={fought}");

    // Phase 2: optional combat if enemy still alive and player approaches
    for _ in 0..180 {
        let pos = world.entities.get(&player).unwrap().position();
        let epos = world
            .entities
            .get(&enemy_id)
            .map(|e| e.position())
            .unwrap_or(Vec2::ZERO);
        if enemy_hp.is_alive() {
            perception.facing = enemy_ai
                .desired_velocity(epos, perception.alert, perception.last_seen)
                .normalize_or_zero();
            perception.sense(epos, Some(pos), true, weapon.cooldown_left > 0.0);
            let vel = enemy_ai.desired_velocity(epos, perception.alert, perception.last_seen);
            if let Some(e) = world.entities.get_mut(&enemy_id) {
                e.velocity.linear = vel;
            }
            // Player seeks enemy and melees
            let dir = (epos - pos).normalize_or_zero();
            world.set_player_input(dir);
            if (epos - pos).length() < 30.0 && weapon.fire() {
                let (_, death) = apply_damage(&mut enemy_hp, enemy_id, player, weapon.damage, epos);
                if death.is_some() {
                    score.add_kill(150);
                    if let Some(e) = world.entities.get_mut(&enemy_id) {
                        e.alive = false;
                    }
                }
            }
        } else {
            world.set_player_input(Vec2::ZERO);
        }
        weapon.tick(1.0 / 60.0);
        world.step(1.0 / 60.0);
        let mut targets = [(enemy_id, epos, 8.0, &mut enemy_hp)];
        projectiles.tick(1.0 / 60.0, &mut targets);
        score.tick(1.0 / 60.0);
    }

    // Phase 3: door consequence — if key_found, open (clear solid door tile)
    if key {
        // Open door tile around D
        let layer = world.map.main_layer_mut();
        for y in 0..layer.height as i32 {
            for x in 0..layer.width as i32 {
                let t = layer.get(x, y);
                if t.flags.kind == 2 || t.id == 3 {
                    layer.set(x, y, Tile::EMPTY);
                }
            }
        }
        println!("door unlocked via narrative key");
    }

    // Ending
    let ending = if mercy > 0 && score.kills == 0 {
        "peaceful"
    } else if score.kills > 0 && mercy == 0 {
        "violent"
    } else {
        "mixed"
    };

    if let Some(leader) = party.leader_mut() {
        leader.inventory.gold += if ending == "peaceful" { 50 } else { 10 };
    }

    println!("hybrid-demo");
    println!("  ending = {ending}");
    println!("  kills = {}", score.kills);
    println!("  key_found = {key}");
    println!(
        "  gold = {}",
        party.leader().map(|m| m.inventory.gold).unwrap_or(0)
    );
    println!(
        "  player = {:?}",
        world.entities.get(&player).map(|e| e.position())
    );

    assert!(story_done, "should complete narrative beat");
    assert!(key, "negotiate path should grant key");
    assert_eq!(ending, "peaceful");
    println!("hybrid-demo OK");
    Ok(())
}
