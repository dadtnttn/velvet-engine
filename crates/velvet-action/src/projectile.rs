//! Projectiles.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;
use velvet_play::Health;

/// Projectile body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    /// Position.
    pub position: Vec2,
    /// Velocity.
    pub velocity: Vec2,
    /// Damage.
    pub damage: f32,
    /// Lifetime remaining.
    pub life: f32,
    /// Owner id (skip self-hit).
    pub owner: usize,
    /// Radius.
    pub radius: f32,
    /// Alive.
    pub alive: bool,
}

impl Projectile {
    /// Create.
    pub fn spawn(
        position: Vec2,
        direction: Vec2,
        speed: f32,
        damage: f32,
        life: f32,
        owner: usize,
    ) -> Self {
        Self {
            position,
            velocity: direction.normalize_or_zero() * speed,
            damage,
            life,
            owner,
            radius: 3.0,
            alive: true,
        }
    }

    /// Integrate.
    pub fn tick(&mut self, dt: f32) {
        if !self.alive {
            return;
        }
        self.position += self.velocity * dt;
        self.life -= dt;
        if self.life <= 0.0 {
            self.alive = false;
        }
    }
}

/// Simple projectile list system.
#[derive(Debug, Default)]
pub struct ProjectileSystem {
    /// Projectiles.
    pub list: Vec<Projectile>,
}

impl ProjectileSystem {
    /// Spawn.
    pub fn spawn(&mut self, p: Projectile) {
        self.list.push(p);
    }

    /// Tick all; damage enemies in targets (id, pos, radius, health).
    pub fn tick(
        &mut self,
        dt: f32,
        targets: &mut [(usize, Vec2, f32, &mut Health)],
    ) -> Vec<(usize, f32)> {
        let mut hits = Vec::new();
        for p in &mut self.list {
            p.tick(dt);
            if !p.alive {
                continue;
            }
            for (id, pos, radius, hp) in targets.iter_mut() {
                if *id == p.owner || !hp.is_alive() {
                    continue;
                }
                if (*pos - p.position).length() <= *radius + p.radius {
                    let dead = hp.damage(p.damage);
                    hits.push((*id, p.damage));
                    p.alive = false;
                    let _ = dead;
                    break;
                }
            }
        }
        self.list.retain(|p| p.alive);
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projectile_hits() {
        let mut sys = ProjectileSystem::default();
        sys.spawn(Projectile::spawn(Vec2::ZERO, Vec2::X, 100.0, 50.0, 2.0, 0));
        let mut hp = Health::full(100.0);
        let mut targets = [(1usize, Vec2::new(10.0, 0.0), 5.0, &mut hp)];
        for _ in 0..10 {
            sys.tick(0.05, &mut targets);
        }
        assert!(hp.current < 100.0);
    }
}
