//! 2D particle emitter: spawn, lifetime, velocity, color fade, gravity.

use serde::{Deserialize, Serialize};
use velvet_math::{Color, Vec2};

/// Single live particle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Particle {
    /// World position.
    pub position: Vec2,
    /// Velocity units/sec.
    pub velocity: Vec2,
    /// Remaining lifetime seconds.
    pub life: f32,
    /// Initial lifetime seconds (for fade).
    pub max_life: f32,
    /// Current size.
    pub size: f32,
    /// Start color.
    pub color_start: Color,
    /// End color (at death).
    pub color_end: Color,
}

impl Particle {
    /// Normalized age 0 (birth) ..= 1 (death).
    pub fn age_norm(&self) -> f32 {
        if self.max_life <= 1e-6 {
            return 1.0;
        }
        (1.0 - self.life / self.max_life).clamp(0.0, 1.0)
    }

    /// Interpolated color by remaining life.
    pub fn color(&self) -> Color {
        self.color_start.lerp(self.color_end, self.age_norm())
    }

    /// Alive.
    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }
}

/// Emission shape for initial offsets / directions.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum EmitterShape {
    /// All at origin.
    #[default]
    Point,
    /// Uniform in circle radius.
    Circle {
        /// Radius.
        radius: f32,
    },
    /// Uniform in AABB half-extents.
    Box {
        /// Half size.
        half: Vec2,
    },
}

/// Configuration for a particle burst / continuous emitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmitterConfig {
    /// World origin.
    pub origin: Vec2,
    /// Base velocity.
    pub velocity: Vec2,
    /// Random velocity half-range added per axis.
    pub velocity_jitter: Vec2,
    /// Gravity acceleration units/sec².
    pub gravity: Vec2,
    /// Lifetime seconds.
    pub lifetime: f32,
    /// Lifetime random half-range.
    pub lifetime_jitter: f32,
    /// Start size.
    pub size: f32,
    /// Size at death (lerped).
    pub size_end: f32,
    /// Start color.
    pub color_start: Color,
    /// End color.
    pub color_end: Color,
    /// Emission shape.
    pub shape: EmitterShape,
    /// Max particles retained.
    pub capacity: usize,
    /// Continuous emission rate (particles/sec). 0 = burst only.
    pub rate: f32,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            velocity: Vec2::new(0.0, 40.0),
            velocity_jitter: Vec2::new(20.0, 20.0),
            gravity: Vec2::new(0.0, -80.0),
            lifetime: 1.0,
            lifetime_jitter: 0.2,
            size: 4.0,
            size_end: 0.0,
            color_start: Color::WHITE,
            color_end: Color::TRANSPARENT,
            shape: EmitterShape::Point,
            capacity: 256,
            rate: 0.0,
        }
    }
}

/// Deterministic LCG for spawn jitter (no external RNG dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticleRng {
    state: u32,
}

impl Default for ParticleRng {
    fn default() -> Self {
        Self { state: 0xA341_316C }
    }
}

impl ParticleRng {
    /// Seeded RNG.
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    /// Next u32.
    pub fn next_u32(&mut self) -> u32 {
        // Numerical Recipes LCG
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    /// Float in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Float in [-1, 1).
    pub fn next_signed(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

/// 2D particle emitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmitter {
    /// Config.
    pub config: EmitterConfig,
    /// Live particles.
    particles: Vec<Particle>,
    /// Accumulator for continuous emission.
    emit_accum: f32,
    /// RNG.
    rng: ParticleRng,
    /// Active (emits when rate > 0).
    pub emitting: bool,
}

impl ParticleEmitter {
    /// Create with config.
    pub fn new(config: EmitterConfig) -> Self {
        let cap = config.capacity;
        Self {
            config,
            particles: Vec::with_capacity(cap.min(1024)),
            emit_accum: 0.0,
            rng: ParticleRng::default(),
            emitting: true,
        }
    }

    /// Default emitter at origin.
    pub fn default_at(origin: Vec2) -> Self {
        let cfg = EmitterConfig {
            origin,
            ..Default::default()
        };
        Self::new(cfg)
    }

    /// Seed RNG.
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.rng = ParticleRng::new(seed);
        self
    }

    /// Live particle slice.
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Count.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Spawn a single particle using config + jitter.
    pub fn spawn_one(&mut self) {
        if self.particles.len() >= self.config.capacity {
            // Drop oldest
            self.particles.remove(0);
        }
        let pos = self.sample_origin();
        let jx = self.rng.next_signed() * self.config.velocity_jitter.x;
        let jy = self.rng.next_signed() * self.config.velocity_jitter.y;
        let vel = self.config.velocity + Vec2::new(jx, jy);
        let life_j = self.rng.next_signed() * self.config.lifetime_jitter;
        let max_life = (self.config.lifetime + life_j).max(0.05);
        self.particles.push(Particle {
            position: pos,
            velocity: vel,
            life: max_life,
            max_life,
            size: self.config.size,
            color_start: self.config.color_start,
            color_end: self.config.color_end,
        });
    }

    /// Spawn a burst of `count` particles.
    pub fn burst(&mut self, count: usize) {
        for _ in 0..count {
            self.spawn_one();
        }
    }

    fn sample_origin(&mut self) -> Vec2 {
        match self.config.shape {
            EmitterShape::Point => self.config.origin,
            EmitterShape::Circle { radius } => {
                let a = self.rng.next_f32() * std::f32::consts::TAU;
                let r = self.rng.next_f32().sqrt() * radius;
                self.config.origin + Vec2::new(a.cos() * r, a.sin() * r)
            }
            EmitterShape::Box { half } => {
                let ox = self.rng.next_signed() * half.x;
                let oy = self.rng.next_signed() * half.y;
                self.config.origin + Vec2::new(ox, oy)
            }
        }
    }

    /// Integrate particles by `dt` and emit continuously when rate > 0.
    pub fn update(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        if self.emitting && self.config.rate > 0.0 {
            self.emit_accum += self.config.rate * dt;
            while self.emit_accum >= 1.0 {
                self.emit_accum -= 1.0;
                self.spawn_one();
            }
        }

        let gravity = self.config.gravity;
        let size_start = self.config.size;
        let size_end = self.config.size_end;
        for p in &mut self.particles {
            p.velocity += gravity * dt;
            p.position += p.velocity * dt;
            p.life -= dt;
            let t = p.age_norm();
            p.size = size_start + (size_end - size_start) * t;
        }
        self.particles.retain(|p| p.is_alive());
    }

    /// Clear all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
        self.emit_accum = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_and_die() {
        let mut e = ParticleEmitter::new(EmitterConfig {
            lifetime: 0.5,
            lifetime_jitter: 0.0,
            gravity: Vec2::ZERO,
            velocity: Vec2::new(10.0, 0.0),
            velocity_jitter: Vec2::ZERO,
            capacity: 32,
            ..Default::default()
        })
        .with_seed(42);
        e.burst(10);
        assert_eq!(e.len(), 10);
        e.update(0.25);
        assert_eq!(e.len(), 10);
        assert!(e.particles()[0].position.x > 0.0);
        e.update(0.5);
        assert!(e.is_empty());
    }

    #[test]
    fn color_fades() {
        let p = Particle {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            life: 0.5,
            max_life: 1.0,
            size: 1.0,
            color_start: Color::WHITE,
            color_end: Color::TRANSPARENT,
        };
        let c = p.color();
        assert!((c.a - 0.5).abs() < 1e-4);
    }

    #[test]
    fn continuous_rate() {
        let mut e = ParticleEmitter::new(EmitterConfig {
            rate: 20.0,
            lifetime: 2.0,
            capacity: 100,
            velocity_jitter: Vec2::ZERO,
            lifetime_jitter: 0.0,
            gravity: Vec2::ZERO,
            ..Default::default()
        })
        .with_seed(1);
        e.update(0.5); // ~10 particles
        assert!(e.len() >= 9 && e.len() <= 11);
    }

    #[test]
    fn capacity_cap() {
        let mut e = ParticleEmitter::new(EmitterConfig {
            capacity: 5,
            lifetime: 10.0,
            lifetime_jitter: 0.0,
            velocity_jitter: Vec2::ZERO,
            gravity: Vec2::ZERO,
            ..Default::default()
        })
        .with_seed(7);
        e.burst(20);
        assert_eq!(e.len(), 5);
    }

    #[test]
    fn shapes_offset() {
        let mut e = ParticleEmitter::new(EmitterConfig {
            shape: EmitterShape::Circle { radius: 10.0 },
            lifetime: 1.0,
            lifetime_jitter: 0.0,
            velocity: Vec2::ZERO,
            velocity_jitter: Vec2::ZERO,
            gravity: Vec2::ZERO,
            capacity: 50,
            ..Default::default()
        })
        .with_seed(99);
        e.burst(30);
        let max_r = e
            .particles()
            .iter()
            .map(|p| p.position.length())
            .fold(0.0_f32, f32::max);
        assert!(max_r <= 10.0 + 1e-3);
    }

    #[test]
    fn gravity_pulls_down() {
        let mut e = ParticleEmitter::new(EmitterConfig {
            lifetime: 2.0,
            lifetime_jitter: 0.0,
            gravity: Vec2::new(0.0, -20.0),
            velocity: Vec2::ZERO,
            velocity_jitter: Vec2::ZERO,
            capacity: 16,
            ..Default::default()
        })
        .with_seed(3);
        e.burst(5);
        let y0 = e.particles()[0].position.y;
        e.update(0.5);
        let y1 = e.particles()[0].position.y;
        assert!(y1 < y0, "y0={y0} y1={y1}");
    }

    #[test]
    fn box_shape_within_half_extents() {
        let half = Vec2::new(4.0, 2.0);
        let mut e = ParticleEmitter::new(EmitterConfig {
            shape: EmitterShape::Box { half },
            lifetime: 1.0,
            lifetime_jitter: 0.0,
            velocity: Vec2::ZERO,
            velocity_jitter: Vec2::ZERO,
            gravity: Vec2::ZERO,
            capacity: 80,
            origin: Vec2::new(10.0, 10.0),
            ..Default::default()
        })
        .with_seed(11);
        e.burst(60);
        for p in e.particles() {
            let local = p.position - Vec2::new(10.0, 10.0);
            assert!(local.x.abs() <= half.x + 1e-3, "x={}", local.x);
            assert!(local.y.abs() <= half.y + 1e-3, "y={}", local.y);
        }
    }

    #[test]
    fn clear_empties_and_stops_accum() {
        let mut e = ParticleEmitter::new(EmitterConfig {
            rate: 100.0,
            lifetime: 5.0,
            capacity: 50,
            ..Default::default()
        })
        .with_seed(1);
        e.update(0.2);
        assert!(!e.is_empty());
        e.clear();
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
    }

    #[test]
    fn age_norm_and_size_fade() {
        let p = Particle {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            life: 0.0,
            max_life: 1.0,
            size: 2.0,
            color_start: Color::WHITE,
            color_end: Color::BLACK,
        };
        assert!((p.age_norm() - 1.0).abs() < 1e-5);
        assert!(!p.is_alive());
        let alive = Particle {
            life: 1.0,
            max_life: 1.0,
            ..p
        };
        assert!((alive.age_norm()).abs() < 1e-5);
        assert!(alive.is_alive());
    }

    #[test]
    fn deterministic_seed_reproduces_burst() {
        let cfg = EmitterConfig {
            lifetime: 1.0,
            lifetime_jitter: 0.1,
            velocity: Vec2::new(5.0, 1.0),
            velocity_jitter: Vec2::new(2.0, 2.0),
            shape: EmitterShape::Circle { radius: 3.0 },
            capacity: 20,
            gravity: Vec2::ZERO,
            ..Default::default()
        };
        let mut a = ParticleEmitter::new(cfg.clone()).with_seed(1234);
        let mut b = ParticleEmitter::new(cfg).with_seed(1234);
        a.burst(10);
        b.burst(10);
        assert_eq!(a.len(), b.len());
        for (pa, pb) in a.particles().iter().zip(b.particles().iter()) {
            assert!((pa.position - pb.position).length() < 1e-5);
            assert!((pa.velocity - pb.velocity).length() < 1e-5);
        }
    }
}
