//! Free-particle layer for creators — bursts, emitters, settle→grid conversion.
//!
//! Particles fly in continuous space, collide with solid cells, and convert into
//! cellular materials (or clear cells) when they settle, expire, or hit.

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, MaterialId};
use crate::events::SimEvent;
use crate::material::Phase;
use crate::world::World;

/// How a particle ends its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ParticleEnd {
    /// Become a grid cell of `material`.
    #[default]
    ConvertToCell,
    /// Erase grid cell at impact (dig / dissolve FX).
    ClearCell,
    /// Leave heat only.
    HeatOnly,
    /// Spawn secondary burst.
    BurstSecondary,
    /// Simply vanish.
    Vanish,
}

/// One free particle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreeParticle {
    /// Instance id.
    pub id: u32,
    /// World X (float cells).
    pub x: f32,
    /// World Y.
    pub y: f32,
    /// Velocity X (cells/s).
    pub vx: f32,
    /// Velocity Y.
    pub vy: f32,
    /// Material payload (grid conversion target).
    pub material: MaterialId,
    /// Age seconds.
    pub age: f32,
    /// Max lifetime seconds.
    pub lifetime: f32,
    /// Radius for collision (cells).
    pub radius: f32,
    /// Bounce coefficient 0..=1.
    pub bounce: f32,
    /// Gravity scale (1 = normal world gravity).
    pub gravity_scale: f32,
    /// Drag per second.
    pub drag: f32,
    /// Temperature carried.
    pub temp: f32,
    /// End behavior.
    pub end: ParticleEnd,
    /// Secondary spawn count if BurstSecondary.
    pub secondary_count: u16,
    /// Alive.
    pub alive: bool,
    /// Frames stuck against solid (for settle).
    pub stuck: u8,
    /// Color override RGBA (0 alpha = use material).
    pub color: [u8; 4],
    /// Z-order for render.
    pub z: f32,
}

impl FreeParticle {
    /// New particle.
    pub fn new(id: u32, x: f32, y: f32, material: MaterialId) -> Self {
        Self {
            id,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            material,
            age: 0.0,
            lifetime: 1.5,
            radius: 0.35,
            bounce: 0.15,
            gravity_scale: 1.0,
            drag: 0.4,
            temp: 20.0,
            end: ParticleEnd::ConvertToCell,
            secondary_count: 0,
            alive: true,
            stuck: 0,
            color: [0, 0, 0, 0],
            z: 50.0,
        }
    }

    /// With velocity.
    pub fn with_vel(mut self, vx: f32, vy: f32) -> Self {
        self.vx = vx;
        self.vy = vy;
        self
    }

    /// Lifetime.
    pub fn with_life(mut self, life: f32) -> Self {
        self.lifetime = life.max(0.05);
        self
    }

    /// End mode.
    pub fn with_end(mut self, end: ParticleEnd) -> Self {
        self.end = end;
        self
    }

    /// Hot particle.
    pub fn with_temp(mut self, t: f32) -> Self {
        self.temp = t;
        self
    }
}

/// Continuous emitter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleEmitter {
    /// Id.
    pub id: u32,
    /// Origin X.
    pub x: f32,
    /// Origin Y.
    pub y: f32,
    /// Material to emit.
    pub material: MaterialId,
    /// Particles per second.
    pub rate: f32,
    /// Cone angle radians (half).
    pub cone: f32,
    /// Base direction angle (radians, 0 = +X).
    pub angle: f32,
    /// Speed min.
    pub speed_min: f32,
    /// Speed max.
    pub speed_max: f32,
    /// Lifetime min.
    pub life_min: f32,
    /// Life max.
    pub life_max: f32,
    /// Gravity scale.
    pub gravity_scale: f32,
    /// End behavior.
    pub end: ParticleEnd,
    /// Enabled.
    pub enabled: bool,
    /// Accumulator for fractional spawns.
    pub accum: f32,
    /// Max particles this emitter may create (0 = unlimited).
    pub budget: u32,
    /// Spawned so far.
    pub spawned: u32,
    /// Temperature.
    pub temp: f32,
    /// Optional attach to enemy id.
    pub follow_enemy: Option<u32>,
}

impl ParticleEmitter {
    /// Create emitter.
    pub fn new(id: u32, x: f32, y: f32, material: MaterialId, rate: f32) -> Self {
        Self {
            id,
            x,
            y,
            material,
            rate,
            cone: 0.6,
            angle: std::f32::consts::FRAC_PI_2,
            speed_min: 4.0,
            speed_max: 12.0,
            life_min: 0.4,
            life_max: 1.8,
            gravity_scale: 1.0,
            end: ParticleEnd::ConvertToCell,
            enabled: true,
            accum: 0.0,
            budget: 0,
            spawned: 0,
            temp: 20.0,
            follow_enemy: None,
        }
    }
}

/// Burst request (one-shot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleBurst {
    /// Center X.
    pub x: f32,
    /// Center Y.
    pub y: f32,
    /// Material.
    pub material: MaterialId,
    /// Count.
    pub count: u32,
    /// Speed min.
    pub speed_min: f32,
    /// Speed max.
    pub speed_max: f32,
    /// Lifetime.
    pub lifetime: f32,
    /// Spread full circle if true.
    pub full_circle: bool,
    /// Angle (radians).
    pub angle: f32,
    /// Cone half-angle.
    pub cone: f32,
    /// End mode.
    pub end: ParticleEnd,
    /// Gravity scale.
    pub gravity_scale: f32,
    /// Temperature.
    pub temp: f32,
}

impl Default for ParticleBurst {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            material: MaterialId::AIR,
            count: 16,
            speed_min: 3.0,
            speed_max: 14.0,
            lifetime: 1.2,
            full_circle: true,
            angle: 0.0,
            cone: std::f32::consts::PI,
            end: ParticleEnd::ConvertToCell,
            gravity_scale: 1.0,
            temp: 20.0,
        }
    }
}

/// Particle system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleConfig {
    /// Max live particles (oldest culled).
    pub max_particles: usize,
    /// World gravity cells/s² (usually negative Y).
    pub gravity_y: f32,
    /// Substeps per cellular step.
    pub substeps: u32,
    /// Settle after N stuck frames.
    pub settle_stuck: u8,
    /// Convert when speed below this and near solid.
    pub settle_speed: f32,
    /// Enable grid collision.
    pub collide_grid: bool,
    /// Deterministic RNG seed offset.
    pub seed: u64,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            max_particles: 8192,
            gravity_y: -48.0,
            substeps: 2,
            settle_stuck: 3,
            settle_speed: 1.2,
            collide_grid: true,
            seed: 0x0A71_C1E5,
        }
    }
}

/// Free particle world layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticleWorld {
    /// Live particles.
    pub particles: Vec<FreeParticle>,
    /// Emitters.
    pub emitters: Vec<ParticleEmitter>,
    /// Config.
    pub config: ParticleConfig,
    next_id: u32,
    next_emitter: u32,
    rng: u64,
    /// Conversions this step (stats).
    pub conversions: u32,
    /// Spawns this step.
    pub spawns: u32,
}

impl Default for ParticleWorld {
    fn default() -> Self {
        Self::new(ParticleConfig::default())
    }
}

impl ParticleWorld {
    /// Create.
    pub fn new(config: ParticleConfig) -> Self {
        let rng = config.seed | 1;
        Self {
            particles: Vec::with_capacity(config.max_particles.min(4096)),
            emitters: Vec::new(),
            config,
            next_id: 1,
            next_emitter: 1,
            rng,
            conversions: 0,
            spawns: 0,
        }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    }

    fn rand_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn rand_range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rand_f32()
    }

    /// Live count.
    pub fn len(&self) -> usize {
        self.particles.iter().filter(|p| p.alive).count()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Spawn one particle.
    pub fn spawn(&mut self, mut p: FreeParticle) -> u32 {
        self.cull_if_needed();
        if p.id == 0 {
            p.id = self.next_id;
            self.next_id = self.next_id.wrapping_add(1);
        }
        let id = p.id;
        p.alive = true;
        self.particles.push(p);
        self.spawns = self.spawns.saturating_add(1);
        id
    }

    /// Burst spawn.
    pub fn burst(&mut self, b: &ParticleBurst) -> u32 {
        let mut n = 0u32;
        for i in 0..b.count {
            let t = if b.count <= 1 {
                0.0
            } else {
                i as f32 / (b.count - 1) as f32
            };
            let ang = if b.full_circle {
                t * std::f32::consts::TAU + self.rand_f32() * 0.2
            } else {
                b.angle + (t - 0.5) * 2.0 * b.cone + self.rand_range(-0.05, 0.05)
            };
            let spd = self.rand_range(b.speed_min, b.speed_max);
            let mut p = FreeParticle::new(0, b.x, b.y, b.material)
                .with_vel(ang.cos() * spd, ang.sin() * spd)
                .with_life(b.lifetime * self.rand_range(0.7, 1.3))
                .with_end(b.end)
                .with_temp(b.temp);
            p.gravity_scale = b.gravity_scale;
            self.spawn(p);
            n += 1;
        }
        n
    }

    /// Convenience blood burst.
    pub fn burst_blood(&mut self, x: f32, y: f32, blood: MaterialId, count: u32) -> u32 {
        self.burst(&ParticleBurst {
            x,
            y,
            material: blood,
            count,
            speed_min: 2.0,
            speed_max: 16.0,
            lifetime: 1.4,
            full_circle: true,
            end: ParticleEnd::ConvertToCell,
            gravity_scale: 1.1,
            temp: 37.0,
            ..Default::default()
        })
    }

    /// Convenience sparks (fire/heat).
    pub fn burst_sparks(&mut self, x: f32, y: f32, fire: MaterialId, count: u32) -> u32 {
        self.burst(&ParticleBurst {
            x,
            y,
            material: fire,
            count,
            speed_min: 6.0,
            speed_max: 22.0,
            lifetime: 0.6,
            full_circle: false,
            angle: std::f32::consts::FRAC_PI_2,
            cone: 1.2,
            end: ParticleEnd::HeatOnly,
            gravity_scale: 0.3,
            temp: 900.0,
        })
    }

    /// Dig burst (clear cells on impact).
    pub fn burst_dig(&mut self, x: f32, y: f32, count: u32) -> u32 {
        self.burst(&ParticleBurst {
            x,
            y,
            material: MaterialId::AIR,
            count,
            speed_min: 8.0,
            speed_max: 20.0,
            lifetime: 0.35,
            full_circle: false,
            angle: -std::f32::consts::FRAC_PI_2,
            cone: 0.8,
            end: ParticleEnd::ClearCell,
            gravity_scale: 0.2,
            temp: 20.0,
        })
    }

    /// Add emitter; returns id.
    pub fn add_emitter(&mut self, mut e: ParticleEmitter) -> u32 {
        if e.id == 0 {
            e.id = self.next_emitter;
            self.next_emitter += 1;
        }
        let id = e.id;
        self.emitters.push(e);
        id
    }

    /// Remove emitter.
    pub fn remove_emitter(&mut self, id: u32) {
        self.emitters.retain(|e| e.id != id);
    }

    /// Get emitter mut.
    pub fn emitter_mut(&mut self, id: u32) -> Option<&mut ParticleEmitter> {
        self.emitters.iter_mut().find(|e| e.id == id)
    }

    fn cull_if_needed(&mut self) {
        let live = self.particles.iter().filter(|p| p.alive).count();
        if live < self.config.max_particles {
            if self.particles.len() > self.config.max_particles * 2 {
                self.particles.retain(|p| p.alive);
            }
            return;
        }
        if let Some(i) = self
            .particles
            .iter()
            .enumerate()
            .filter(|(_, p)| p.alive)
            .min_by(|a, b| {
                a.1.age
                    .partial_cmp(&b.1.age)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
        {
            self.particles[i].alive = false;
        }
        self.particles.retain(|p| p.alive);
    }

    /// Step particles + emitters; couples to grid.
    pub fn step(&mut self, world: &mut World, dt: f32) {
        self.conversions = 0;
        self.spawns = 0;
        self.emit_from_emitters(dt);
        let n = self.config.substeps.max(1);
        let h = dt / n as f32;
        for _ in 0..n {
            self.integrate(world, h);
        }
        self.particles.retain(|p| p.alive);
    }

    fn emit_from_emitters(&mut self, dt: f32) {
        let specs: Vec<ParticleEmitter> = self.emitters.clone();
        for (idx, e) in specs.iter().enumerate() {
            if !e.enabled {
                continue;
            }
            if e.budget > 0 && e.spawned >= e.budget {
                continue;
            }
            let mut accum = self.emitters[idx].accum + e.rate * dt;
            let mut spawned = e.spawned;
            while accum >= 1.0 {
                accum -= 1.0;
                if e.budget > 0 && spawned >= e.budget {
                    break;
                }
                let ang = e.angle + (self.rand_f32() - 0.5) * 2.0 * e.cone;
                let spd = self.rand_range(e.speed_min, e.speed_max);
                let life = self.rand_range(e.life_min, e.life_max);
                let mut p = FreeParticle::new(0, e.x, e.y, e.material)
                    .with_vel(ang.cos() * spd, ang.sin() * spd)
                    .with_life(life)
                    .with_end(e.end)
                    .with_temp(e.temp);
                p.gravity_scale = e.gravity_scale;
                self.spawn(p);
                spawned = spawned.saturating_add(1);
            }
            self.emitters[idx].accum = accum;
            self.emitters[idx].spawned = spawned;
        }
    }

    fn integrate(&mut self, world: &mut World, dt: f32) {
        let g = self.config.gravity_y;
        let settle_speed = self.config.settle_speed;
        let settle_stuck = self.config.settle_stuck;
        let collide = self.config.collide_grid;

        let mut ends: Vec<(ParticleEnd, f32, f32, MaterialId, f32, u16)> = Vec::new();

        for p in self.particles.iter_mut() {
            if !p.alive {
                continue;
            }
            p.age += dt;
            if p.age >= p.lifetime {
                ends.push((p.end, p.x, p.y, p.material, p.temp, p.secondary_count));
                p.alive = false;
                continue;
            }
            p.vy += g * p.gravity_scale * dt;
            let drag = (1.0 - p.drag * dt).clamp(0.0, 1.0);
            p.vx *= drag;
            p.vy *= drag;
            let mut nx = p.x + p.vx * dt;
            let mut ny = p.y + p.vy * dt;

            if collide {
                let ix = nx.floor() as i32;
                let iy = ny.floor() as i32;
                if is_blocking(world, ix, iy) {
                    let speed = (p.vx * p.vx + p.vy * p.vy).sqrt();
                    if speed < settle_speed {
                        p.stuck = p.stuck.saturating_add(1);
                    } else {
                        p.stuck = 0;
                    }
                    let cx = ix as f32 + 0.5;
                    let cy = iy as f32 + 0.5;
                    let dx = p.x - cx;
                    let dy = p.y - cy;
                    if dx.abs() > dy.abs() {
                        p.vx *= -p.bounce;
                        nx = p.x;
                    } else {
                        p.vy *= -p.bounce;
                        ny = p.y;
                    }
                    if p.stuck >= settle_stuck {
                        ends.push((p.end, p.x, p.y, p.material, p.temp, p.secondary_count));
                        p.alive = false;
                        continue;
                    }
                } else {
                    p.stuck = 0;
                }
            }
            p.x = nx;
            p.y = ny;
        }

        for (end, x, y, mat, temp, secondary) in ends {
            self.finish_particle(world, end, x, y, mat, temp, secondary);
        }
    }

    fn finish_particle(
        &mut self,
        world: &mut World,
        end: ParticleEnd,
        x: f32,
        y: f32,
        mat: MaterialId,
        temp: f32,
        secondary: u16,
    ) {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        match end {
            ParticleEnd::ConvertToCell => {
                if mat.is_air() {
                    return;
                }
                let cur = world.get(ix, iy);
                if !cur.is_air() {
                    let phase = world.materials.phase(cur.material);
                    if matches!(phase, Phase::Static | Phase::Solid) {
                        for (dx, dy) in [(0, -1), (-1, 0), (1, 0), (0, 1), (0, 0)] {
                            let c = world.get(ix + dx, iy + dy);
                            if c.is_air() {
                                world.set(ix + dx, iy + dy, Cell::of(mat).with_temp(temp));
                                self.conversions = self.conversions.saturating_add(1);
                                world.events.push(SimEvent::ParticleConverted {
                                    x: ix + dx,
                                    y: iy + dy,
                                    material: mat,
                                });
                                return;
                            }
                        }
                        return;
                    }
                }
                world.set(ix, iy, Cell::of(mat).with_temp(temp));
                self.conversions = self.conversions.saturating_add(1);
                world.events.push(SimEvent::ParticleConverted {
                    x: ix,
                    y: iy,
                    material: mat,
                });
            }
            ParticleEnd::ClearCell => {
                let cur = world.get(ix, iy);
                if cur.is_air() {
                    return;
                }
                if world.materials.phase(cur.material) == Phase::Static {
                    return;
                }
                world.set(ix, iy, Cell::air());
                self.conversions = self.conversions.saturating_add(1);
            }
            ParticleEnd::HeatOnly => {
                let mut c = world.get(ix, iy);
                if !c.is_air() {
                    c.temp = c.temp.max(temp);
                    world.set(ix, iy, c);
                }
            }
            ParticleEnd::BurstSecondary => {
                let count = secondary.max(4) as u32;
                self.burst(&ParticleBurst {
                    x,
                    y,
                    material: mat,
                    count,
                    speed_min: 2.0,
                    speed_max: 8.0,
                    lifetime: 0.5,
                    full_circle: true,
                    end: ParticleEnd::ConvertToCell,
                    gravity_scale: 1.0,
                    temp,
                    ..Default::default()
                });
            }
            ParticleEnd::Vanish => {}
        }
    }

    /// Clear all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emitters.clear();
    }

    /// Stamp particles into a color buffer window.
    pub fn stamp_into_buffer(
        &self,
        buf: &mut crate::render_buf::ColorBuffer,
        origin_x: i32,
        origin_y: i32,
        world: &World,
    ) {
        for p in self.particles.iter().filter(|p| p.alive) {
            let px = (p.x.floor() as i32) - origin_x;
            let py = (p.y.floor() as i32) - origin_y;
            if px < 0 || py < 0 || px as u32 >= buf.width || py as u32 >= buf.height {
                continue;
            }
            let rgba = if p.color[3] > 0 {
                p.color
            } else if !p.material.is_air() {
                world.materials.get(p.material).color
            } else {
                [255, 220, 80, 255]
            };
            buf.set_rgba(px as u32, py as u32, rgba[0], rgba[1], rgba[2], 255);
        }
    }
}

fn is_blocking(world: &World, x: i32, y: i32) -> bool {
    let c = world.get(x, y);
    if c.is_air() {
        return false;
    }
    matches!(
        world.materials.phase(c.material),
        Phase::Solid | Phase::Static | Phase::Powder
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::WorldConfig;

    #[test]
    fn burst_then_settle_converts_to_grid() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-20, 0, 20, 2, ids.bedrock);
        let mut pw = ParticleWorld::new(ParticleConfig {
            settle_stuck: 2,
            settle_speed: 50.0,
            gravity_y: -80.0,
            ..ParticleConfig::default()
        });
        for i in 0..24 {
            let p = FreeParticle::new(0, -5.0 + i as f32 * 0.4, 8.0, ids.sand)
                .with_vel(0.0, -5.0)
                .with_life(3.0)
                .with_end(ParticleEnd::ConvertToCell);
            pw.spawn(p);
        }
        assert!(pw.len() >= 20);
        for _ in 0..120 {
            pw.step(&mut world, 1.0 / 60.0);
        }
        let mut sand = 0;
        for y in 0..12 {
            for x in -10..10 {
                if world.get(x, y).material == ids.sand {
                    sand += 1;
                }
            }
        }
        assert!(
            sand > 0 || pw.conversions > 0,
            "expected sand convert: sand={sand} conv={}",
            pw.conversions
        );
    }

    #[test]
    fn emitter_spawns_over_time() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut pw = ParticleWorld::default();
        let mut e = ParticleEmitter::new(0, 0.0, 10.0, ids.water, 120.0);
        e.budget = 40;
        e.life_min = 0.5;
        e.life_max = 1.0;
        pw.add_emitter(e);
        for _ in 0..30 {
            pw.step(&mut world, 1.0 / 60.0);
        }
        assert!(pw.emitters[0].spawned > 0);
        assert!(pw.emitters[0].spawned <= 40);
    }

    #[test]
    fn clear_cell_end_digs_solid() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-2, 0, 3, 3, ids.stone);
        let mut pw = ParticleWorld::default();
        let mut p = FreeParticle::new(0, 0.5, 1.5, MaterialId::AIR)
            .with_vel(0.0, -1.0)
            .with_life(0.05)
            .with_end(ParticleEnd::ClearCell);
        p.gravity_scale = 0.0;
        pw.spawn(p);
        pw.step(&mut world, 0.1);
        // lifetime expired -> clear at position
        assert!(pw.conversions > 0 || world.get(0, 1).is_air() || world.get(0, 0).is_air());
    }
}
