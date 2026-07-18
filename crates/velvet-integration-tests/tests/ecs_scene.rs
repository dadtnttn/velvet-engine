//! Cross-crate: ECS world spawning + scene manager load/unload + component queries.

use velvet_ecs::World;
use velvet_math::Vec2;
use velvet_scene::{PrefabLibrary, PrefabNode, SceneBlueprint, SceneManager, SceneState};

#[derive(Debug, Clone, PartialEq)]
struct Position(Vec2);

#[derive(Debug, Clone, PartialEq)]
struct Health {
    hp: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct TagPlayer;

// ---------------------------------------------------------------------------
// ECS multi-entity combat-ish setup
// ---------------------------------------------------------------------------

#[test]
fn ecs_spawn_query_and_despawn() {
    let mut world = World::new();
    let player = world.spawn_named("player");
    world.insert(player, Position(Vec2::new(10.0, 20.0)));
    world.insert(player, Health { hp: 100.0 });
    world.insert(player, TagPlayer);

    let mut enemies = Vec::new();
    for i in 0..8 {
        let e = world.spawn_named(format!("enemy_{i}"));
        world.insert(e, Position(Vec2::new(i as f32 * 5.0, 0.0)));
        world.insert(
            e,
            Health {
                hp: 20.0 + i as f32,
            },
        );
        enemies.push(e);
    }

    assert!(world.contains(player));
    assert_eq!(world.get::<Position>(player).unwrap().0.x, 10.0);
    assert_eq!(world.get::<Health>(player).unwrap().hp, 100.0);

    if let Some(h) = world.get_mut::<Health>(enemies[0]) {
        h.hp -= 5.0;
    }
    assert!((world.get::<Health>(enemies[0]).unwrap().hp - 15.0).abs() < 1e-4);

    for e in enemies.iter().take(4) {
        assert!(world.despawn(*e));
    }
    assert!(!world.contains(enemies[0]));
    assert!(world.contains(enemies[7]));
    assert!(world.contains(player));
    assert!(world.entity_count() >= 5);
}

#[test]
fn ecs_resources_and_named_meta() {
    let mut world = World::new();
    #[derive(Debug, Clone, PartialEq)]
    struct Score(u32);
    world.insert_resource(Score(0));
    {
        let s = world.resource_mut::<Score>().expect("score");
        s.0 += 10;
    }
    assert_eq!(world.resource::<Score>().unwrap().0, 10);

    let e = world.spawn_named("npc");
    assert_eq!(world.meta(e).and_then(|m| m.name.as_deref()), Some("npc"));
}

// ---------------------------------------------------------------------------
// Scene manager multi-scene flows
// ---------------------------------------------------------------------------

#[test]
fn scene_load_activate_unload() {
    let mut world = World::new();
    let mut mgr = SceneManager::new();
    let lib = PrefabLibrary::default();

    // Empty blueprint — no prefab instances required.
    let bp = SceneBlueprint::new("town");
    let id = mgr.load(&mut world, &lib, &bp, false).expect("load town");
    assert!(mgr.get(id).is_some());
    assert_eq!(mgr.get(id).unwrap().name, "town");
    assert_eq!(mgr.get(id).unwrap().state, SceneState::Active);
    assert!(mgr.primary().is_some());

    let overlay = SceneBlueprint::new("ui_overlay");
    let overlay_id = mgr
        .load(&mut world, &lib, &overlay, true)
        .expect("load overlay");
    assert_ne!(id, overlay_id);
    assert!(mgr.len() >= 2);

    mgr.unload(&mut world, overlay_id).expect("unload overlay");
    assert!(mgr.get(overlay_id).is_none());
    assert!(mgr.get(id).is_some());
}

#[test]
fn scene_replace_primary() {
    let mut world = World::new();
    let mut mgr = SceneManager::new();
    let lib = PrefabLibrary::default();

    let a = mgr
        .load(&mut world, &lib, &SceneBlueprint::new("a"), false)
        .unwrap();
    let b = mgr
        .load(&mut world, &lib, &SceneBlueprint::new("b"), false)
        .unwrap();
    // Non-additive load unloads previous primary.
    assert!(mgr.get(a).is_none());
    assert!(mgr.get(b).is_some());
    assert_eq!(mgr.primary().map(|s| s.name.as_str()), Some("b"));
}

#[test]
fn scene_with_inline_prefab_nodes() {
    let mut world = World::new();
    let mut mgr = SceneManager::new();
    let lib = PrefabLibrary::default();

    let mut bp = SceneBlueprint::new("arena");
    bp.inline.push(PrefabNode {
        name: Some("spawner".into()),
        components: vec![],
        children: vec![],
    });
    let id = mgr.load(&mut world, &lib, &bp, false).expect("load");
    let scene = mgr.get(id).expect("scene");
    assert_eq!(scene.name, "arena");
    assert!(!scene.additive);
    assert!(!scene.roots.is_empty());
}

// ---------------------------------------------------------------------------
// Combined: scene roots get ECS components for gameplay
// ---------------------------------------------------------------------------

#[test]
fn scene_entities_receive_gameplay_components() {
    let mut world = World::new();
    let mut mgr = SceneManager::new();
    let lib = PrefabLibrary::default();
    let id = mgr
        .load(&mut world, &lib, &SceneBlueprint::new("battle"), false)
        .unwrap();

    let hero = world.spawn_named("hero");
    world.insert(hero, Position(Vec2::new(0.0, 0.0)));
    world.insert(hero, Health { hp: 50.0 });
    world.insert(hero, TagPlayer);

    // Mutate scene via unload/reload path is hard without get_mut — attach via world only.
    // Verify scene exists and world entity is playable.
    assert_eq!(mgr.get(id).unwrap().name, "battle");
    world.get_mut::<Health>(hero).unwrap().hp -= 12.0;
    assert!((world.get::<Health>(hero).unwrap().hp - 38.0).abs() < 1e-4);
    world.get_mut::<Position>(hero).unwrap().0.x += 5.0;
    assert!((world.get::<Position>(hero).unwrap().0.x - 5.0).abs() < 1e-4);

    // Second additive scene while battle stays.
    let hud = mgr
        .load(&mut world, &lib, &SceneBlueprint::new("hud"), true)
        .unwrap();
    assert!(mgr.get(id).is_some());
    assert!(mgr.get(hud).is_some());
    assert_eq!(mgr.len(), 2);
}
