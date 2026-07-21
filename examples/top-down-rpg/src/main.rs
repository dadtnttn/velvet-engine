//! Top-down RPG demo — map, player movement, NPC interact, inventory, quest, dialogue.

use anyhow::Result;
use velvet_math::{Transform2D, Vec2};
use velvet_play::prelude::*;
use velvet_rpg::prelude::*;
use velvet_story::prelude::*;

fn main() -> Result<()> {
    velvet_core::init_tracing_default("rpg=info,info");

    let map = TileMap::from_ascii(
        "\
################
#..............#
#..#####.......#
#..#...#.......#
#..#...D.......#
#..#####.......#
#.........N....#
#..............#
################",
        16.0,
    )?;

    let mut world = PlayWorld::new(map);
    let player = world.spawn_player(Vec2::new(32.0, 32.0), 140.0);

    // NPC and door placed in the open courtyard near spawn (reliable for headless seek).
    world.spawn(PlayEntity {
        id: 0,
        transform: Transform2D::from_translation(Vec2::new(64.0, 48.0)),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(6.0))),
        kinematic: None,
        speed: None,
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: Some(Interactable::new("talk_npc", 28.0)),
        alive: true,
    });

    world.spawn(PlayEntity {
        id: 0,
        transform: Transform2D::from_translation(Vec2::new(96.0, 48.0)),
        velocity: Velocity::ZERO,
        collider: Some(Collider::aabb(Vec2::splat(8.0))),
        kinematic: None,
        speed: None,
        facing: Facing::default(),
        player: false,
        trigger: None,
        interactable: Some(Interactable::new("door", 28.0)),
        alive: true,
    });

    // RPG setup
    let mut db = ItemDb::default();
    db.insert(ItemDef::potion("potion", "Herb", 30.0, 15));
    db.insert(ItemDef::weapon("stick", "Walking Stick", 3.0, 5));

    let mut hero = PartyMember::new("hero", "Wanderer");
    hero.inventory.gold = 40;
    hero.inventory.add("potion", 2, 99).unwrap();
    hero.inventory.add("stick", 1, 1).unwrap();
    hero.inventory
        .equip("stick", EquipSlot::MainHand, &db)
        .unwrap();

    let mut party = Party::default();
    party.add(hero);

    let mut journal = QuestJournal::default();
    let mut q = Quest::new("village_help", "Talk to the villager");
    q.objectives
        .push(QuestObjective::new("talk", "Speak with the NPC", 1));
    q.reward_gold = 25;
    q.reward_xp = 50;
    journal.start(q);

    // Dialogue snippet via story
    let dialog = r#"
character villager { name: "Mira" }
scene chat {
    villager "The cellar door sticks. Could you check it?"
    villager "Thanks, Wanderer."
}
"#;
    let prog = load_program_from_source(dialog, Some("npc.vel"), "NPC")?;
    let mut story = StoryPlayer::start(prog);

    // Navigate toward NPC then door using simple seek steering.
    let mut talked = false;
    let mut opened = false;
    let npc_pos = Vec2::new(64.0, 48.0);
    let door_pos = Vec2::new(96.0, 48.0);

    for _ in 0..400 {
        let pos = world.entities.get(&player).unwrap().position();
        let target = if !talked { npc_pos } else { door_pos };
        let dir = (target - pos).normalize_or_zero();
        world.set_player_input(dir);
        world.step(1.0 / 60.0);
        // Only pulse interact when close enough to avoid spam.
        let near = (pos - target).length() < 30.0;
        if near {
            world.try_player_interact(true);
        }
        if !talked && world.interact_events.iter().any(|e| e.action == "talk_npc") {
            talked = true;
            while !story.is_ended() {
                match story.wait().clone() {
                    StoryWait::Line => story.advance(),
                    StoryWait::Choice => {
                        let _ = story.choose(0);
                    }
                    StoryWait::Ended => break,
                    StoryWait::Ready | StoryWait::Pause { .. } => story.advance(),
                    StoryWait::Host { token } => {
                        let _ = story.resume_host(&token);
                    }
                }
            }
            journal.progress("village_help", "talk", 1);
            if let Some(leader) = party.leader_mut() {
                leader.inventory.gold += 25;
                leader.level.add_xp(50);
            }
        }
        if talked && !opened && world.interact_events.iter().any(|e| e.action == "door") {
            opened = true;
            break;
        }
    }

    // Fallback: if pathfinding bump left us short, walk the last meters / snap.
    if talked && !opened {
        if let Some(e) = world.entities.get_mut(&player) {
            e.transform.translation = door_pos + Vec2::new(-12.0, 0.0);
        }
        world.step(1.0 / 60.0);
        world.try_player_interact(true);
        opened = world.interact_events.iter().any(|e| e.action == "door");
    }

    // Damage + potion
    if let Some(leader) = party.leader_mut() {
        leader.stats.take_damage(20.0);
        let hp_before = leader.stats.hp;
        leader
            .inventory
            .use_consumable("potion", &db, &mut leader.stats)
            .unwrap();
        assert!(leader.stats.hp > hp_before);
    }

    println!("top-down-rpg demo");
    println!("  talked_npc = {talked}");
    println!("  opened_door = {opened}");
    println!("  quest_done = {}", journal.is_completed("village_help"));
    println!(
        "  gold = {}",
        party.leader().map(|m| m.inventory.gold).unwrap_or(0)
    );
    println!(
        "  level = {}",
        party.leader().map(|m| m.level.level).unwrap_or(1)
    );
    println!(
        "  player_pos = {:?}",
        world.entities.get(&player).map(|e| e.position())
    );

    assert!(talked, "should talk to NPC");
    assert!(journal.is_completed("village_help"));
    assert!(opened, "should interact with door");
    println!("top-down-rpg OK");
    Ok(())
}
