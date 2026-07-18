//! Combat helpers for agents/enemies on the cellular grid.

use crate::agent::AgentWorld;
use crate::cell::Cell;
use crate::enemy::EnemyWorld;
use crate::particles::{ParticleEnd, ParticleWorld};
use crate::physics::PhysicsWorld;
use crate::world::World;

/// Hit scan result.
#[derive(Debug, Clone)]
pub struct HitScan {
    /// Hit cell X.
    pub x: i32,
    /// Hit cell Y.
    pub y: i32,
    /// Distance traveled.
    pub dist: f32,
    /// Enemy id if hit.
    pub enemy_id: Option<u32>,
    /// Agent id if hit.
    pub agent_id: Option<u32>,
}

/// Raycast for combat — first solid or entity along aim.
pub fn hitscan(
    world: &World,
    enemies: &EnemyWorld,
    agents: &AgentWorld,
    x0: f32,
    y0: f32,
    aim: f32,
    max_dist: f32,
) -> Option<HitScan> {
    let dx = aim.cos();
    let dy = aim.sin();
    let steps = (max_dist * 2.0) as i32;
    let mut x = x0;
    let mut y = y0;
    for i in 0..=steps {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        // entities first
        for e in &enemies.enemies {
            if !e.alive {
                continue;
            }
            if (x - e.x).abs() <= e.hw && (y - e.y).abs() <= e.hh {
                return Some(HitScan {
                    x: ix,
                    y: iy,
                    dist: i as f32 * 0.5,
                    enemy_id: Some(e.id),
                    agent_id: None,
                });
            }
        }
        for a in &agents.agents {
            if !a.alive {
                continue;
            }
            if (x - a.x).abs() <= a.hw && (y - a.y).abs() <= a.hh {
                return Some(HitScan {
                    x: ix,
                    y: iy,
                    dist: i as f32 * 0.5,
                    enemy_id: None,
                    agent_id: Some(a.id),
                });
            }
        }
        let c = world.get(ix, iy);
        if !c.is_air() {
            let phase = world.materials.phase(c.material);
            if matches!(
                phase,
                crate::material::Phase::Solid
                    | crate::material::Phase::Static
                    | crate::material::Phase::Powder
            ) {
                return Some(HitScan {
                    x: ix,
                    y: iy,
                    dist: i as f32 * 0.5,
                    enemy_id: None,
                    agent_id: None,
                });
            }
        }
        x += dx * 0.5;
        y += dy * 0.5;
    }
    None
}

/// Apply hitscan damage / terrain chip.
pub fn fire_hitscan(
    world: &mut World,
    enemies: &mut EnemyWorld,
    agents: &mut AgentWorld,
    physics: &mut PhysicsWorld,
    particles: &mut ParticleWorld,
    x0: f32,
    y0: f32,
    aim: f32,
    max_dist: f32,
    damage: f32,
) -> Option<HitScan> {
    let hit = hitscan(world, enemies, agents, x0, y0, aim, max_dist)?;
    if let Some(eid) = hit.enemy_id {
        enemies.damage(eid, damage, world, physics);
        let blood = world.mat("blood");
        if !blood.is_air() {
            particles.burst_blood(hit.x as f32, hit.y as f32, blood, 10);
        }
    } else if let Some(aid) = hit.agent_id {
        agents.damage(aid, damage, world, particles, physics);
    } else {
        // chip terrain
        let c = world.get(hit.x, hit.y);
        if !c.is_air()
            && world.materials.phase(c.material) != crate::material::Phase::Static
        {
            let mat = c.material;
            world.set(hit.x, hit.y, Cell::air());
            particles.spawn(
                crate::particles::FreeParticle::new(
                    0,
                    hit.x as f32 + 0.5,
                    hit.y as f32 + 0.5,
                    mat,
                )
                .with_vel(aim.cos() * -2.0, aim.sin() * -2.0 + 3.0)
                .with_life(0.6)
                .with_end(ParticleEnd::ConvertToCell),
            );
        }
    }
    Some(hit)
}

/// Melee splash damage in radius.
pub fn melee_splash(
    world: &mut World,
    enemies: &mut EnemyWorld,
    physics: &mut PhysicsWorld,
    particles: &mut ParticleWorld,
    x: f32,
    y: f32,
    radius: f32,
    damage: f32,
) -> u32 {
    let mut hits = 0u32;
    let ids: Vec<u32> = enemies
        .enemies
        .iter()
        .filter(|e| e.alive)
        .filter(|e| {
            let dx = e.x - x;
            let dy = e.y - y;
            dx * dx + dy * dy <= radius * radius
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        if enemies.damage(id, damage, world, physics) || true {
            hits += 1;
            let blood = world.mat("blood");
            if !blood.is_air() {
                particles.burst_blood(x, y, blood, 6);
            }
        }
    }
    hits
}

/// Knockback rigid body at id.
pub fn knockback(physics: &mut PhysicsWorld, body_id: u32, aim: f32, force: f32) {
    physics.apply_impulse(body_id, aim.cos() * force, aim.sin() * force);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::enemy::register_builtin_enemies;
    use crate::world::WorldConfig;

    #[test]
    fn hitscan_hits_enemy_and_wall() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut enemies = EnemyWorld::new();
        register_builtin_enemies(&mut enemies);
        let mut physics = PhysicsWorld::new();
        let mut particles = ParticleWorld::default();
        let agents = AgentWorld::new();
        let id = enemies.spawn("slime", 5.0, 0.0, &mut physics).unwrap();
        let hit = hitscan(&world, &enemies, &agents, 0.0, 0.0, 0.0, 20.0).expect("hit");
        assert_eq!(hit.enemy_id, Some(id));
        world.paint_rect(8, -1, 10, 2, ids.stone);
        let wall = hitscan(&world, &EnemyWorld::new(), &agents, 0.0, 0.0, 0.0, 20.0);
        assert!(wall.is_some());
        let _ = fire_hitscan(
            &mut world,
            &mut enemies,
            &mut AgentWorld::new(),
            &mut physics,
            &mut particles,
            0.0,
            0.0,
            0.0,
            20.0,
            50.0,
        );
    }
}
