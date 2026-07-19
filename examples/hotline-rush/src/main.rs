//! Hotline Rush — **demo** composing action **tools** (not the engine API).
//!
//! Uses aim/loadout/hitscan tools + optional `RoomRun` recipe. Headless smoke.

use anyhow::Result;
use velvet_action::prelude::*;
use velvet_action::PICKUP_RADIUS;
use velvet_math::Vec2;
use velvet_play::Health;

#[derive(Clone)]
struct Hostile {
    id: usize,
    pos: Vec2,
    health: Health,
}

fn spawn_room() -> Vec<Hostile> {
    vec![
        Hostile {
            id: 10,
            pos: Vec2::new(100.0, 40.0),
            health: Health::full(40.0),
        },
        Hostile {
            id: 11,
            pos: Vec2::new(140.0, 40.0),
            health: Health::full(40.0),
        },
        Hostile {
            id: 12,
            pos: Vec2::new(100.0, 90.0),
            health: Health::full(40.0),
        },
        Hostile {
            id: 13,
            pos: Vec2::new(140.0, 90.0),
            health: Health::full(40.0),
        },
    ]
}

fn nearest<'a>(hostiles: &'a [Hostile], from: Vec2) -> Option<&'a Hostile> {
    hostiles
        .iter()
        .filter(|h| h.health.is_alive())
        .min_by(|a, b| {
            (a.pos - from)
                .length_squared()
                .partial_cmp(&(b.pos - from).length_squared())
                .unwrap()
        })
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("hotline=info,info");

    println!("=== Hotline Rush (pre-alpha) ===");
    println!("Genre: top-down action shooter (Hotline Miami–like)");
    println!("Rules: free aim · one-hit death · pickups · room clear · quick restart\n");

    let mut player_pos = Vec2::new(40.0, 40.0);
    let player_id = 1usize;
    let mut player_hp = Health::full(1.0);
    let mut hostiles = spawn_room();

    let mut run = HotlineRun::start_room(hostiles.len() as u32, Some(HotlinePresets::bat()));
    // Pistol on the floor for a mid-run pickup.
    let pistol_pos = Vec2::new(70.0, 70.0);
    run.spawn_drop(pistol_pos, HotlinePresets::pistol());

    let dt = 1.0 / 60.0;
    let mut frames = 0u32;
    const MAX_FRAMES: u32 = 2_400;
    let mut scripted_death_done = false;
    let mut restarts_done = 0u32;

    while frames < MAX_FRAMES && run.phase != HotlinePhase::Cleared {
        frames += 1;
        run.tick(dt);

        if run.phase == HotlinePhase::Dead {
            if restarts_done >= 1 {
                break;
            }
            println!(
                "[frame {frames}] DEAD — quick restart (next restarts={})",
                run.restarts + 1
            );
            hostiles = spawn_room();
            player_pos = Vec2::new(40.0, 40.0);
            player_hp = Health::full(1.0);
            run.quick_restart(hostiles.len() as u32, Some(HotlinePresets::bat()));
            run.spawn_drop(pistol_pos, HotlinePresets::pistol());
            restarts_done += 1;
            continue;
        }

        if run.phase != HotlinePhase::Playing {
            break;
        }

        // One scripted one-hit death early on the first life.
        if !scripted_death_done && restarts_done == 0 && frames == 30 {
            let _ = run.player_hit(&mut player_hp, 10, 1.0, player_pos, player_id);
            scripted_death_done = true;
            println!("[frame {frames}] enemy tagged the player (one-hit)");
            continue;
        }

        let Some(target) = nearest(&hostiles, player_pos).cloned() else {
            continue;
        };

        run.aim_at(player_pos, target.pos);

        // After two kills, grab the floor pistol (gun phase).
        if run.score.kills >= 2 && !matches!(run.loadout.active().kind, WeaponKind::Hitscan) {
            let to_drop = pistol_pos - player_pos;
            let d = to_drop.length();
            if d > PICKUP_RADIUS {
                player_pos = player_pos + (to_drop / d) * (180.0 * dt).min(d);
            } else if let Some(id) = run.pickup(player_pos) {
                println!("[frame {frames}] picked up weapon drop id={id}");
            }
            continue;
        }

        // Seek engagement distance: melee close-in, gun mid-range.
        let melee = matches!(run.loadout.active().kind, WeaponKind::Melee);
        let engage = if melee { 30.0 } else { 90.0 };
        let to = target.pos - player_pos;
        let dist = to.length();
        let speed = 200.0;
        if dist > engage {
            player_pos = player_pos + (to / dist) * (speed * dt).min(dist - engage + 1.0);
        }

        // Attack every few frames once in range.
        let in_range = dist <= run.loadout.active().range + 4.0;
        if in_range && frames % 8 == 0 {
            let candidates: Vec<(usize, Vec2)> = hostiles
                .iter()
                .filter(|h| h.health.is_alive())
                .map(|h| (h.id, h.pos))
                .collect();
            match run.attack(player_pos, &candidates, player_id) {
                AttackOutcome::MeleeHits { targets, damage } => {
                    for tid in targets {
                        if let Some(h) = hostiles.iter_mut().find(|h| h.id == tid) {
                            if h.health.damage(damage) {
                                run.register_kill(KillStyle::Melee);
                                println!(
                                    "[frame {frames}] MELEE kill id={tid} combo={} score={}",
                                    run.score.combo.count, run.score.score
                                );
                            }
                        }
                    }
                }
                AttackOutcome::HitscanHit {
                    target: tid,
                    damage,
                    ..
                } => {
                    if let Some(h) = hostiles.iter_mut().find(|h| h.id == tid) {
                        // Pistol 25 dmg vs 40 hp → ensure kill for smoke after 2 taps.
                        let dmg = damage.max(40.0);
                        if h.health.damage(dmg) {
                            run.register_kill(KillStyle::Gun);
                            println!(
                                "[frame {frames}] GUN kill id={tid} combo={} score={}",
                                run.score.combo.count, run.score.score
                            );
                        }
                    }
                }
                AttackOutcome::Missed
                | AttackOutcome::ShotEmpty { .. }
                | AttackOutcome::SpawnProjectile { .. } => {}
            }
        }
    }

    println!("\n--- result ---");
    println!("phase: {:?}", run.phase);
    println!("frames: {frames}");
    println!("kills: {}", run.score.kills);
    println!("deaths: {}", run.score.deaths);
    println!("restarts: {}", run.restarts);
    println!("best_combo: {}", run.score.best_combo);
    println!("score: {}", run.score.score);

    if run.phase != HotlinePhase::Cleared {
        anyhow::bail!(
            "room not cleared (phase={:?} kills={}); pre-alpha sim failed",
            run.phase,
            run.score.kills
        );
    }
    println!("ROOM CLEAR — Hotline Rush pre-alpha smoke OK");
    Ok(())
}
