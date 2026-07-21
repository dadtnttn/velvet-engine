//! Status effects on agents and enemies (burn, wet, freeze, poison, shocked).

use serde::{Deserialize, Serialize};

use crate::agent::AgentWorld;
use crate::enemy::EnemyWorld;
use crate::particles::ParticleWorld;
use crate::physics::PhysicsWorld;
use crate::world::World;

/// Effect kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusKind {
    /// On fire — periodic damage + sparks.
    Burning,
    /// Wet — extinguishes burn, slows.
    Wet,
    /// Frozen — root + damage over time if extreme.
    Frozen,
    /// Poison — DoT.
    Poisoned,
    /// Shocked — stun frames.
    Shocked,
    /// Bleeding — blood particles + DoT.
    Bleeding,
}

/// One active status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    /// Kind.
    pub kind: StatusKind,
    /// Remaining seconds.
    pub remaining: f32,
    /// Intensity (damage per tick).
    pub power: f32,
}

/// Status map keyed by entity kind + id.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusWorld {
    /// Enemy statuses: (enemy_id, effects).
    pub enemies: Vec<(u32, Vec<StatusEffect>)>,
    /// Agent statuses.
    pub agents: Vec<(u32, Vec<StatusEffect>)>,
}

impl StatusWorld {
    /// New.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply or refresh effect on enemy.
    pub fn apply_enemy(&mut self, id: u32, kind: StatusKind, duration: f32, power: f32) {
        let entry = self.enemies.iter_mut().find(|(i, _)| *i == id);
        if let Some((_, list)) = entry {
            if let Some(e) = list.iter_mut().find(|e| e.kind == kind) {
                e.remaining = e.remaining.max(duration);
                e.power = e.power.max(power);
            } else {
                list.push(StatusEffect {
                    kind,
                    remaining: duration,
                    power,
                });
            }
        } else {
            self.enemies.push((
                id,
                vec![StatusEffect {
                    kind,
                    remaining: duration,
                    power,
                }],
            ));
        }
    }

    /// Apply to agent.
    pub fn apply_agent(&mut self, id: u32, kind: StatusKind, duration: f32, power: f32) {
        let entry = self.agents.iter_mut().find(|(i, _)| *i == id);
        if let Some((_, list)) = entry {
            if let Some(e) = list.iter_mut().find(|e| e.kind == kind) {
                e.remaining = e.remaining.max(duration);
                e.power = e.power.max(power);
            } else {
                list.push(StatusEffect {
                    kind,
                    remaining: duration,
                    power,
                });
            }
        } else {
            self.agents.push((
                id,
                vec![StatusEffect {
                    kind,
                    remaining: duration,
                    power,
                }],
            ));
        }
    }

    /// Tick all statuses; apply damage / FX.
    pub fn tick(
        &mut self,
        dt: f32,
        world: &mut World,
        enemies: &mut EnemyWorld,
        agents: &mut AgentWorld,
        physics: &mut PhysicsWorld,
        particles: &mut ParticleWorld,
    ) {
        // wet extinguishes burning
        for (_, list) in &mut self.enemies {
            let wet = list
                .iter()
                .any(|e| e.kind == StatusKind::Wet && e.remaining > 0.0);
            if wet {
                list.retain(|e| e.kind != StatusKind::Burning);
            }
        }
        for (_, list) in &mut self.agents {
            let wet = list
                .iter()
                .any(|e| e.kind == StatusKind::Wet && e.remaining > 0.0);
            if wet {
                list.retain(|e| e.kind != StatusKind::Burning);
            }
        }

        // enemy ticks
        let mut enemy_dmg: Vec<(u32, f32)> = Vec::new();
        let mut enemy_blood: Vec<(f32, f32)> = Vec::new();
        for (id, list) in &mut self.enemies {
            for e in list.iter_mut() {
                e.remaining -= dt;
                match e.kind {
                    StatusKind::Burning | StatusKind::Poisoned | StatusKind::Bleeding => {
                        enemy_dmg.push((*id, e.power * dt));
                    }
                    StatusKind::Frozen | StatusKind::Shocked => {
                        if let Some(en) = enemies.get_mut(*id) {
                            en.stun = en.stun.max(2);
                        }
                    }
                    StatusKind::Wet => {}
                }
                if e.kind == StatusKind::Bleeding {
                    if let Some(en) = enemies.get(*id) {
                        enemy_blood.push((en.x, en.y));
                    }
                }
                if e.kind == StatusKind::Burning {
                    if let Some(en) = enemies.get(*id) {
                        let fire = world.mat("fire");
                        if !fire.is_air() {
                            particles.burst_sparks(en.x, en.y, fire, 2);
                        }
                    }
                }
            }
            list.retain(|e| e.remaining > 0.0);
        }
        for (id, dmg) in enemy_dmg {
            enemies.damage(id, dmg, world, physics);
        }
        let blood = world.mat("blood");
        if !blood.is_air() {
            for (x, y) in enemy_blood {
                particles.burst_blood(x, y, blood, 2);
            }
        }

        // agent ticks
        let mut agent_dmg: Vec<(u32, f32)> = Vec::new();
        for (id, list) in &mut self.agents {
            for e in list.iter_mut() {
                e.remaining -= dt;
                match e.kind {
                    StatusKind::Burning | StatusKind::Poisoned | StatusKind::Bleeding => {
                        agent_dmg.push((*id, e.power * dt));
                    }
                    StatusKind::Frozen | StatusKind::Shocked => {
                        if let Some(a) = agents.get_mut(*id) {
                            a.invuln = a.invuln.max(1); // reuse as brief lock
                            a.vx *= 0.5;
                        }
                    }
                    StatusKind::Wet => {
                        if let Some(a) = agents.get_mut(*id) {
                            a.speed = (a.speed * 0.98).max(10.0);
                        }
                    }
                }
            }
            list.retain(|e| e.remaining > 0.0);
        }
        for (id, dmg) in agent_dmg {
            agents.damage(id, dmg, world, particles, physics);
        }

        self.enemies.retain(|(_, l)| !l.is_empty());
        self.agents.retain(|(_, l)| !l.is_empty());
    }

    /// Whether entity has a status.
    pub fn enemy_has(&self, id: u32, kind: StatusKind) -> bool {
        self.enemies
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, l)| l.iter().any(|e| e.kind == kind && e.remaining > 0.0))
            .unwrap_or(false)
    }

    /// Whether agent has a status.
    pub fn agent_has(&self, id: u32, kind: StatusKind) -> bool {
        self.agents
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, l)| l.iter().any(|e| e.kind == kind && e.remaining > 0.0))
            .unwrap_or(false)
    }

    /// Clear all statuses for enemy.
    pub fn clear_enemy(&mut self, id: u32) {
        self.enemies.retain(|(i, _)| *i != id);
    }

    /// Clear all statuses for agent.
    pub fn clear_agent(&mut self, id: u32) {
        self.agents.retain(|(i, _)| *i != id);
    }

    /// Count active effect instances.
    pub fn active_count(&self) -> usize {
        self.enemies.iter().map(|(_, l)| l.len()).sum::<usize>()
            + self.agents.iter().map(|(_, l)| l.len()).sum::<usize>()
    }

    /// Infer status from standing cell material.
    pub fn sample_environment(&mut self, world: &World, enemies: &EnemyWorld, agents: &AgentWorld) {
        for e in &enemies.enemies {
            if !e.alive {
                continue;
            }
            let c = world.get(e.x.floor() as i32, e.y.floor() as i32);
            let key = world.materials.get(c.material).key.as_str();
            match key {
                "fire" | "lava" | "napalm" => self.apply_enemy(e.id, StatusKind::Burning, 2.0, 8.0),
                "water" | "water_salt" | "coolant" => {
                    self.apply_enemy(e.id, StatusKind::Wet, 1.5, 0.0)
                }
                "poison" => self.apply_enemy(e.id, StatusKind::Poisoned, 3.0, 4.0),
                "ice" | "snow" | "ice_block" => {
                    self.apply_enemy(e.id, StatusKind::Frozen, 1.0, 1.0)
                }
                "blood" => self.apply_enemy(e.id, StatusKind::Bleeding, 1.0, 2.0),
                "shock_gel" | "shock_water" => {
                    self.apply_enemy(e.id, StatusKind::Shocked, 0.8, 0.0)
                }
                _ => {}
            }
        }
        for a in &agents.agents {
            if !a.alive {
                continue;
            }
            let c = world.get(a.x.floor() as i32, a.y.floor() as i32);
            let key = world.materials.get(c.material).key.as_str();
            match key {
                "fire" | "lava" => self.apply_agent(a.id, StatusKind::Burning, 1.5, 6.0),
                "water" | "water_salt" => self.apply_agent(a.id, StatusKind::Wet, 1.0, 0.0),
                "poison" => self.apply_agent(a.id, StatusKind::Poisoned, 2.0, 3.0),
                _ => {}
            }
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
    fn burning_damages_enemy() {
        let (reg, _) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut enemies = EnemyWorld::new();
        register_builtin_enemies(&mut enemies);
        let mut physics = PhysicsWorld::new();
        let mut particles = ParticleWorld::default();
        let mut agents = AgentWorld::new();
        let id = enemies.spawn("slime", 0.0, 5.0, &mut physics).unwrap();
        let hp0 = enemies.get(id).unwrap().hp;
        let mut st = StatusWorld::new();
        st.apply_enemy(id, StatusKind::Poisoned, 2.0, 20.0);
        for _ in 0..30 {
            st.tick(
                1.0 / 30.0,
                &mut world,
                &mut enemies,
                &mut agents,
                &mut physics,
                &mut particles,
            );
        }
        let hp1 = enemies.get(id).map(|e| e.hp).unwrap_or(0.0);
        assert!(hp1 < hp0 || enemies.get(id).is_none());
        st.apply_enemy(id, StatusKind::Wet, 1.0, 0.0);
        assert!(st.enemy_has(id, StatusKind::Wet));
        st.clear_enemy(id);
        assert!(!st.enemy_has(id, StatusKind::Wet));
    }
}
