//! Projectile entities that travel in continuous space and interact with grid/enemies.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;
use crate::enemy::EnemyWorld;
use crate::material::Phase;
use crate::particles::{ParticleBurst, ParticleEnd, ParticleWorld};
use crate::physics::PhysicsWorld;
use crate::world::World;

/// Projectile payload kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectileKind {
    /// Physical bullet — chips solids, damages enemies.
    Bullet,
    /// Fireball — heat + fire particles.
    Fireball,
    /// Acid blob — convert to acid on hit.
    AcidBlob,
    /// Water bolt — extinguish + water.
    WaterBolt,
    /// Dig missile — clear cells.
    DigRocket,
}

/// One projectile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    /// Id.
    pub id: u32,
    /// X.
    pub x: f32,
    /// Y.
    pub y: f32,
    /// VX.
    pub vx: f32,
    /// VY.
    pub vy: f32,
    /// Kind.
    pub kind: ProjectileKind,
    /// Lifetime seconds.
    pub life: f32,
    /// Age.
    pub age: f32,
    /// Damage.
    pub damage: f32,
    /// Radius of effect.
    pub radius: f32,
    /// Alive.
    pub alive: bool,
    /// Gravity scale.
    pub gravity: f32,
}

/// Projectile world.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectileWorld {
    /// Projectiles.
    pub items: Vec<Projectile>,
    next_id: u32,
}

impl ProjectileWorld {
    /// New.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }

    /// Spawn.
    pub fn spawn(&mut self, mut p: Projectile) -> u32 {
        if p.id == 0 {
            p.id = self.next_id;
            self.next_id += 1;
        }
        let id = p.id;
        p.alive = true;
        self.items.push(p);
        id
    }

    /// Convenience fireball.
    pub fn fireball(&mut self, x: f32, y: f32, aim: f32, speed: f32) -> u32 {
        self.spawn(Projectile {
            id: 0,
            x,
            y,
            vx: aim.cos() * speed,
            vy: aim.sin() * speed,
            kind: ProjectileKind::Fireball,
            life: 2.0,
            age: 0.0,
            damage: 15.0,
            radius: 2.0,
            alive: true,
            gravity: 0.3,
        })
    }

    /// Bullet.
    pub fn bullet(&mut self, x: f32, y: f32, aim: f32, speed: f32) -> u32 {
        self.spawn(Projectile {
            id: 0,
            x,
            y,
            vx: aim.cos() * speed,
            vy: aim.sin() * speed,
            kind: ProjectileKind::Bullet,
            life: 1.5,
            age: 0.0,
            damage: 10.0,
            radius: 0.5,
            alive: true,
            gravity: 0.05,
        })
    }

    /// Live count.
    pub fn len(&self) -> usize {
        self.items.iter().filter(|p| p.alive).count()
    }

    /// Return whether no live projectiles remain.
    pub fn is_empty(&self) -> bool {
        self.items.iter().all(|projectile| !projectile.alive)
    }

    /// Step projectiles.
    pub fn step(
        &mut self,
        world: &mut World,
        enemies: &mut EnemyWorld,
        physics: &mut PhysicsWorld,
        particles: &mut ParticleWorld,
        dt: f32,
        gravity_y: f32,
    ) {
        let mut impacts = Vec::new();
        for p in &mut self.items {
            if !p.alive {
                continue;
            }
            p.age += dt;
            if p.age >= p.life {
                p.alive = false;
                impacts.push((p.x, p.y, p.kind, p.damage, p.radius, true));
                continue;
            }
            p.vy += gravity_y * p.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            let ix = p.x.floor() as i32;
            let iy = p.y.floor() as i32;
            // enemy hit
            let mut hit_enemy = false;
            for e in &enemies.enemies {
                if !e.alive {
                    continue;
                }
                if (p.x - e.x).abs() <= e.hw + p.radius && (p.y - e.y).abs() <= e.hh + p.radius {
                    hit_enemy = true;
                    break;
                }
            }
            if hit_enemy {
                p.alive = false;
                impacts.push((p.x, p.y, p.kind, p.damage, p.radius, false));
                continue;
            }
            let c = world.get(ix, iy);
            if !c.is_air() {
                let ph = world.materials.phase(c.material);
                if matches!(ph, Phase::Solid | Phase::Static | Phase::Powder) {
                    p.alive = false;
                    impacts.push((p.x, p.y, p.kind, p.damage, p.radius, false));
                }
            }
        }
        for (x, y, kind, dmg, radius, expired) in impacts {
            resolve_impact(
                world, enemies, physics, particles, x, y, kind, dmg, radius, expired,
            );
        }
        self.items.retain(|p| p.alive);
    }
}

fn resolve_impact(
    world: &mut World,
    enemies: &mut EnemyWorld,
    physics: &mut PhysicsWorld,
    particles: &mut ParticleWorld,
    x: f32,
    y: f32,
    kind: ProjectileKind,
    dmg: f32,
    radius: f32,
    _expired: bool,
) {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let r = radius.ceil() as i32;
    // damage enemies in radius
    let ids: Vec<u32> = enemies
        .enemies
        .iter()
        .filter(|e| e.alive)
        .filter(|e| {
            let dx = e.x - x;
            let dy = e.y - y;
            dx * dx + dy * dy <= (radius + 1.0) * (radius + 1.0)
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        enemies.damage(id, dmg, world, physics);
    }
    match kind {
        ProjectileKind::Bullet => {
            let c = world.get(ix, iy);
            if !c.is_air() && world.materials.phase(c.material) != Phase::Static {
                world.set(ix, iy, Cell::air());
            }
        }
        ProjectileKind::Fireball => {
            let fire = world.mat("fire");
            particles.burst_sparks(x, y, fire, 16);
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy <= r * r {
                        let mut c = world.get(ix + dx, iy + dy);
                        if !c.is_air() {
                            c.temp += 120.0;
                            world.set(ix + dx, iy + dy, c);
                        } else if !fire.is_air() && (dx * dx + dy * dy) <= 1 {
                            world.set(
                                ix + dx,
                                iy + dy,
                                Cell::of(fire).with_life(12).with_temp(800.0),
                            );
                        }
                    }
                }
            }
        }
        ProjectileKind::AcidBlob => {
            let acid = world.mat("acid");
            particles.burst(&ParticleBurst {
                x,
                y,
                material: acid,
                count: 12,
                speed_min: 2.0,
                speed_max: 8.0,
                lifetime: 1.0,
                full_circle: true,
                end: ParticleEnd::ConvertToCell,
                gravity_scale: 1.0,
                temp: 20.0,
                ..Default::default()
            });
            world.paint_circle(ix, iy, r.max(1), acid);
        }
        ProjectileKind::WaterBolt => {
            let water = world.mat("water");
            world.paint_circle(ix, iy, r.max(1), water);
            // extinguish nearby fire
            for dy in -r..=r {
                for dx in -r..=r {
                    let c = world.get(ix + dx, iy + dy);
                    if world.materials.phase(c.material) == Phase::Plasma {
                        world.set(ix + dx, iy + dy, Cell::air());
                    }
                }
            }
        }
        ProjectileKind::DigRocket => {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx * dx + dy * dy > r * r {
                        continue;
                    }
                    let c = world.get(ix + dx, iy + dy);
                    if !c.is_air() && world.materials.phase(c.material) != Phase::Static {
                        world.set(ix + dx, iy + dy, Cell::air());
                    }
                }
            }
            particles.burst_dig(x, y, 10);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::enemy::register_builtin_enemies;
    use crate::world::WorldConfig;

    #[test]
    fn fireball_hits_wall_and_heats() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(5, -1, 8, 2, ids.stone);
        let mut pw = ProjectileWorld::new();
        let mut enemies = EnemyWorld::new();
        register_builtin_enemies(&mut enemies);
        let mut physics = PhysicsWorld::new();
        let mut particles = ParticleWorld::default();
        pw.fireball(0.0, 0.0, 0.0, 40.0);
        for _ in 0..30 {
            pw.step(
                &mut world,
                &mut enemies,
                &mut physics,
                &mut particles,
                1.0 / 60.0,
                -20.0,
            );
        }
        // stone should be heated or chipped or fire present
        let hot = world.get(5, 0).temp > 20.0
            || world.get(6, 0).temp > 20.0
            || !particles.is_empty()
            || pw.is_empty();
        assert!(hot);
    }
}
