//! Action Arena demo — movement, aiming, enemies, perception, combat, score, quick restart.

use anyhow::Result;
use velvet_action::prelude::*;
use velvet_math::{Transform2D, Vec2};
use velvet_play::prelude::*;

struct Actor {
    id: usize,
    pos: Vec2,
    vel: Vec2,
    health: Health,
    team: u8, // 0 player, 1 enemy
    ai: Option<EnemyAi>,
    perception: Option<Perception>,
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("action=info,info");

    let map = TileMap::from_ascii(
        "\
##############
#............#
#..##....##..#
#............#
#............#
#..##....##..#
#............#
##############",
        16.0,
    )?;

    let mut world = PlayWorld::new(map);
    let pid = world.spawn_player(Vec2::new(48.0, 48.0), 180.0);

    let mut actors = vec![Actor {
        id: pid,
        pos: Vec2::new(48.0, 48.0),
        vel: Vec2::ZERO,
        health: Health::full(100.0),
        team: 0,
        ai: None,
        perception: None,
    }];

    // Enemies
    for (i, p) in [(180.0, 48.0), (180.0, 96.0), (100.0, 100.0)]
        .iter()
        .enumerate()
    {
        let id = world.spawn(PlayEntity {
            id: 0,
            transform: Transform2D::from_translation(Vec2::new(p.0, p.1)),
            velocity: Velocity::ZERO,
            collider: Some(Collider {
                layer: CollisionLayer::ENEMY,
                mask: CollisionMask::from_layers(CollisionLayer::WORLD | CollisionLayer::PLAYER),
                ..Collider::aabb(Vec2::splat(6.0))
            }),
            kinematic: Some(KinematicBody::default()),
            speed: Some(Speed(70.0)),
            facing: Facing::default(),
            player: false,
            trigger: None,
            interactable: None,
            alive: true,
        });
        let patrol = PatrolPath::new(vec![
            Vec2::new(p.0, p.1),
            Vec2::new(p.0 - 30.0, p.1),
            Vec2::new(p.0, p.1 + 20.0),
        ]);
        actors.push(Actor {
            id,
            pos: Vec2::new(p.0, p.1),
            vel: Vec2::ZERO,
            health: Health::full(40.0),
            team: 1,
            ai: Some(EnemyAi::guard(patrol)),
            perception: Some(Perception::default()),
        });
        let _ = i;
    }

    let mut weapon = Weapon::pistol("sidearm");
    let mut projectiles = ProjectileSystem::default();
    let mut score = ScoreBoard::default();
    let mut restarts = 0u32;

    // Simulate arena seconds
    let dt = 1.0 / 60.0;
    for frame in 0..600 {
        // Player AI for demo: move toward nearest enemy and shoot
        let player_pos = world.entities.get(&pid).map(|e| e.position()).unwrap();
        let nearest_enemy = actors
            .iter()
            .filter(|a| a.team == 1 && a.health.is_alive())
            .min_by(|a, b| {
                (a.pos - player_pos)
                    .length_squared()
                    .partial_cmp(&(b.pos - player_pos).length_squared())
                    .unwrap()
            })
            .map(|a| a.pos);

        if let Some(target) = nearest_enemy {
            let dir = (target - player_pos).normalize_or_zero();
            world.set_player_input(dir);
            if weapon.fire() {
                projectiles.spawn(Projectile::spawn(
                    player_pos + dir * 10.0,
                    dir,
                    320.0,
                    weapon.damage,
                    1.5,
                    pid,
                ));
            }
        } else {
            world.set_player_input(Vec2::ZERO);
        }
        weapon.tick(dt);
        world.step(dt);

        // Sync player actor pos
        if let Some(e) = world.entities.get(&pid) {
            actors[0].pos = e.position();
            actors[0].vel = e.velocity.linear;
        }

        // Enemy AI + perception
        let player_pos_ai = actors[0].pos;
        for a in actors
            .iter_mut()
            .filter(|a| a.team == 1 && a.health.is_alive())
        {
            if let Some(perc) = &mut a.perception {
                perc.facing = a.vel.normalize_or_zero();
                if perc.facing.length_squared() < 0.1 {
                    perc.facing = Vec2::X;
                }
                // LOS: simple always clear in open areas
                perc.sense(a.pos, Some(player_pos_ai), true, false);
            }
            if let Some(ai) = &mut a.ai {
                let alert = a.perception.as_ref().map(|p| p.alert).unwrap_or(0.0);
                let last = a.perception.as_ref().and_then(|p| p.last_seen);
                a.vel = ai.desired_velocity(a.pos, alert, last);
            }
            if let Some(e) = world.entities.get_mut(&a.id) {
                e.velocity.linear = a.vel;
            }
        }
        // Step already moved; sync positions
        for a in actors.iter_mut() {
            if let Some(e) = world.entities.get(&a.id) {
                a.pos = e.position();
            }
        }

        // Projectiles vs enemies
        let mut targets: Vec<(usize, Vec2, f32, &mut Health)> = actors
            .iter_mut()
            .filter(|a| a.team == 1)
            .map(|a| (a.id, a.pos, 8.0, &mut a.health))
            .collect();
        let hits = projectiles.tick(dt, &mut targets);
        for (id, _) in hits {
            if let Some(a) = actors.iter_mut().find(|a| a.id == id) {
                if !a.health.is_alive() {
                    score.add_kill(100);
                    if let Some(e) = world.entities.get_mut(&id) {
                        e.alive = false;
                    }
                }
            }
        }
        score.tick(dt);

        // Quick restart if player "dies" (demo: force once)
        if frame == 300 && restarts == 0 {
            // Simulate death and restart at checkpoint
            score.add_death();
            restarts += 1;
            world
                .checkpoints
                .insert(Checkpoint::new("start", Vec2::new(48.0, 48.0), "arena"));
            world.checkpoints.activate("start");
            if let Some(spawn) = world.checkpoints.respawn_position() {
                if let Some(e) = world.entities.get_mut(&pid) {
                    e.transform.translation = spawn;
                    e.alive = true;
                }
                actors[0].health = Health::full(100.0);
                actors[0].pos = spawn;
            }
            println!("quick restart at checkpoint");
        }
    }

    let kills = score.kills;
    let pts = score.score;
    println!("action-arena demo");
    println!("  kills = {kills}");
    println!("  score = {pts}");
    println!("  best_combo = {}", score.best_combo);
    println!("  restarts = {restarts}");
    println!(
        "  enemies_alive = {}",
        actors
            .iter()
            .filter(|a| a.team == 1 && a.health.is_alive())
            .count()
    );

    assert!(
        kills >= 1 || projectiles.list.is_empty(),
        "expected combat activity"
    );
    assert_eq!(restarts, 1);
    println!("action-arena OK");
    Ok(())
}
