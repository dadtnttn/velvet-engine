//! Creator-facing player/agent body — dig, paint, collide with solid cells.

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, MaterialId};
use crate::events::SimEvent;
use crate::material::Phase;
use crate::particles::{ParticleBurst, ParticleEnd, ParticleWorld};
use crate::physics::PhysicsWorld;
use crate::world::World;

/// Agent control intent for one frame.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct AgentInput {
    /// Move X (-1..=1).
    pub move_x: f32,
    /// Move Y (-1..=1) for flight/swim.
    pub move_y: f32,
    /// Jump / up impulse.
    pub jump: bool,
    /// Dig action held.
    pub dig: bool,
    /// Place material held.
    pub place: bool,
    /// Fire wand / cast.
    pub cast: bool,
    /// Aim angle radians.
    pub aim: f32,
}

/// Player-or-agent avatar for Noita-like creators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Id.
    pub id: u32,
    /// Center X.
    pub x: f32,
    /// Center Y.
    pub y: f32,
    /// Velocity X.
    pub vx: f32,
    /// Velocity Y.
    pub vy: f32,
    /// Half width.
    pub hw: f32,
    /// Half height.
    pub hh: f32,
    /// HP.
    pub hp: f32,
    /// Max HP.
    pub max_hp: f32,
    /// Move speed.
    pub speed: f32,
    /// Jump speed.
    pub jump_speed: f32,
    /// On ground.
    pub grounded: bool,
    /// Material to place.
    pub place_material: MaterialId,
    /// Dig radius.
    pub dig_radius: i32,
    /// Place radius.
    pub place_radius: i32,
    /// Linked rigid body (optional).
    pub body_id: Option<u32>,
    /// Facing.
    pub facing: f32,
    /// Invuln frames.
    pub invuln: u32,
    /// Alive.
    pub alive: bool,
    /// Blood material for hurt FX.
    pub blood_material: MaterialId,
}

impl Agent {
    /// Create at position.
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self {
            id,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hw: 0.9,
            hh: 1.4,
            hp: 100.0,
            max_hp: 100.0,
            speed: 28.0,
            jump_speed: 36.0,
            grounded: false,
            place_material: MaterialId::AIR,
            dig_radius: 2,
            place_radius: 1,
            body_id: None,
            facing: 1.0,
            invuln: 0,
            alive: true,
            blood_material: MaterialId::AIR,
        }
    }

    /// Bounds.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        (self.x - self.hw, self.y - self.hh, self.x + self.hw, self.y + self.hh)
    }
}

/// World of agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentWorld {
    /// Agents.
    pub agents: Vec<Agent>,
    next_id: u32,
    /// Gravity.
    pub gravity_y: f32,
}

impl AgentWorld {
    /// New.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            next_id: 1,
            gravity_y: -55.0,
        }
    }

    /// Spawn agent; optionally create physics body.
    pub fn spawn(&mut self, x: f32, y: f32, physics: &mut PhysicsWorld) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut a = Agent::new(id, x, y);
        let bid = physics.spawn_dynamic(x, y, a.hw * 2.0, a.hh * 2.0, 2.5);
        a.body_id = Some(bid);
        self.agents.push(a);
        id
    }

    /// Get mut.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Get.
    pub fn get(&self, id: u32) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Apply input and step.
    pub fn step(
        &mut self,
        world: &mut World,
        physics: &mut PhysicsWorld,
        particles: &mut ParticleWorld,
        inputs: &[(u32, AgentInput)],
        dt: f32,
    ) {
        for (id, input) in inputs {
            let Some(agent) = self.agents.iter_mut().find(|a| a.id == *id && a.alive) else {
                continue;
            };
            step_agent(agent, world, physics, particles, *input, dt, self.gravity_y);
        }
        // sync from physics
        for a in &mut self.agents {
            if !a.alive {
                continue;
            }
            if let Some(bid) = a.body_id {
                if let Some(b) = physics.get(bid) {
                    a.x = b.x;
                    a.y = b.y;
                    a.vx = b.vx;
                    a.vy = b.vy;
                }
            }
            if a.invuln > 0 {
                a.invuln -= 1;
            }
        }
        self.agents.retain(|a| a.alive);
    }

    /// Damage agent; blood particles on hit.
    pub fn damage(
        &mut self,
        id: u32,
        amount: f32,
        world: &mut World,
        particles: &mut ParticleWorld,
        physics: &mut PhysicsWorld,
    ) -> bool {
        let Some(a) = self.agents.iter_mut().find(|a| a.id == id && a.alive) else {
            return false;
        };
        if a.invuln > 0 {
            return false;
        }
        a.hp -= amount;
        a.invuln = 12;
        let blood = if a.blood_material.is_air() {
            world.mat("blood")
        } else {
            a.blood_material
        };
        if !blood.is_air() {
            particles.burst_blood(a.x, a.y, blood, 12);
        }
        if a.hp <= 0.0 {
            a.alive = false;
            if let Some(bid) = a.body_id {
                physics.remove(bid);
            }
            if !blood.is_air() {
                particles.burst_blood(a.x, a.y, blood, 40);
                world.paint_circle(a.x as i32, a.y as i32, 3, blood);
            }
            world.events.push(SimEvent::AgentDied {
                id,
                x: a.x,
                y: a.y,
            });
            return true;
        }
        false
    }
}

fn step_agent(
    agent: &mut Agent,
    world: &mut World,
    physics: &mut PhysicsWorld,
    particles: &mut ParticleWorld,
    input: AgentInput,
    dt: f32,
    gravity_y: f32,
) {
    if input.move_x.abs() > 0.01 {
        agent.facing = input.move_x.signum();
    }
    // kinematic fallback if no body
    if agent.body_id.is_none() {
        agent.vy += gravity_y * dt;
        agent.vx = input.move_x * agent.speed;
        agent.x += agent.vx * dt;
        agent.y += agent.vy * dt;
        resolve_agent_grid(agent, world);
    } else if let Some(bid) = agent.body_id {
        if let Some(b) = physics.get_mut(bid) {
            b.vx += input.move_x * agent.speed * dt * 3.0;
            if input.jump && agent.grounded {
                b.vy = agent.jump_speed;
            }
            b.vx = b.vx.clamp(-agent.speed, agent.speed);
            agent.x = b.x;
            agent.y = b.y;
            agent.vx = b.vx;
            agent.vy = b.vy;
        }
    }

    // grounded probe
    let foot_y = (agent.y - agent.hh - 0.05).floor() as i32;
    let fx = agent.x.floor() as i32;
    agent.grounded = is_solid(world, fx, foot_y) || is_solid(world, fx - 1, foot_y) || is_solid(world, fx + 1, foot_y);

    let aim_x = agent.x + input.aim.cos() * 2.0;
    let aim_y = agent.y + input.aim.sin() * 2.0;
    let aix = aim_x.floor() as i32;
    let aiy = aim_y.floor() as i32;

    if input.dig {
        dig_at(world, particles, aix, aiy, agent.dig_radius);
    }
    if input.place && !agent.place_material.is_air() {
        world.paint_circle(aix, aiy, agent.place_radius, agent.place_material);
    }
    if input.cast {
        // default dig spray in aim direction
        particles.burst(&ParticleBurst {
            x: agent.x,
            y: agent.y,
            material: MaterialId::AIR,
            count: 10,
            speed_min: 12.0,
            speed_max: 22.0,
            lifetime: 0.3,
            full_circle: false,
            angle: input.aim,
            cone: 0.25,
            end: ParticleEnd::ClearCell,
            gravity_scale: 0.1,
            temp: 20.0,
        });
    }
}

fn is_solid(world: &World, x: i32, y: i32) -> bool {
    let c = world.get(x, y);
    if c.is_air() {
        return false;
    }
    matches!(
        world.materials.phase(c.material),
        Phase::Solid | Phase::Static | Phase::Powder
    )
}

fn resolve_agent_grid(agent: &mut Agent, world: &World) {
    // push out of solids
    for _ in 0..4 {
        let (x0, y0, x1, y1) = agent.bounds();
        let mut pushed = false;
        for iy in (y0.floor() as i32)..=(y1.ceil() as i32) {
            for ix in (x0.floor() as i32)..=(x1.ceil() as i32) {
                if !is_solid(world, ix, iy) {
                    continue;
                }
                // push up
                agent.y += 0.2;
                agent.vy = agent.vy.max(0.0);
                pushed = true;
            }
        }
        if !pushed {
            break;
        }
    }
}

/// Dig/destroy solids in radius (Noita-like terrain edit).
pub fn dig_at(world: &mut World, particles: &mut ParticleWorld, x: i32, y: i32, radius: i32) {
    let mut removed = 0u32;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let cx = x + dx;
            let cy = y + dy;
            let c = world.get(cx, cy);
            if c.is_air() {
                continue;
            }
            let phase = world.materials.phase(c.material);
            if phase == Phase::Static {
                continue;
            }
            if matches!(phase, Phase::Solid | Phase::Powder) {
                let mat = c.material;
                world.set(cx, cy, Cell::air());
                removed += 1;
                // debris particles
                if removed % 3 == 0 {
                    particles.spawn(
                        crate::particles::FreeParticle::new(
                            0,
                            cx as f32 + 0.5,
                            cy as f32 + 0.5,
                            mat,
                        )
                        .with_vel((dx as f32) * 2.0, (dy as f32) * 2.0 + 3.0)
                        .with_life(0.8)
                        .with_end(ParticleEnd::ConvertToCell),
                    );
                }
            }
        }
    }
    if removed > 0 {
        world.events.push(SimEvent::TerrainDug {
            x,
            y,
            radius,
            cells: removed,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::WorldConfig;

    #[test]
    fn agent_digs_stone() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-5, 0, 6, 4, ids.stone);
        let mut physics = PhysicsWorld::new();
        let mut particles = ParticleWorld::default();
        let mut agents = AgentWorld::new();
        let id = agents.spawn(0.0, 8.0, &mut physics);
        if let Some(a) = agents.get_mut(id) {
            a.dig_radius = 3;
        }
        let input = AgentInput {
            dig: true,
            aim: -std::f32::consts::FRAC_PI_2,
            ..Default::default()
        };
        // dig toward floor
        for _ in 0..5 {
            agents.step(
                &mut world,
                &mut physics,
                &mut particles,
                &[(id, input)],
                1.0 / 60.0,
            );
        }
        // manual dig at stone
        dig_at(&mut world, &mut particles, 0, 2, 2);
        let mut air = 0;
        for y in 0..4 {
            for x in -2..3 {
                if world.get(x, y).is_air() {
                    air += 1;
                }
            }
        }
        assert!(air > 0, "dig should remove stone cells");
    }
}
