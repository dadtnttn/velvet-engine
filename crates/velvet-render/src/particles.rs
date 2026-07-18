//! CPU particle batch description for GPU upload.

use velvet_math::{Color, Vec2};

use crate::texture::TextureId;

/// Single particle instance (CPU-side; GPU upload is separate).
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// World position.
    pub position: Vec2,
    /// Linear velocity (units per second).
    pub velocity: Vec2,
    /// Multiplicative color.
    pub color: Color,
    /// Age in seconds.
    pub age: f32,
    /// Lifetime in seconds; particle dies when `age >= lifetime`.
    pub lifetime: f32,
    /// Base size in world units.
    pub size: f32,
    /// Angular velocity radians per second.
    pub angular_velocity: f32,
    /// Current rotation radians.
    pub rotation: f32,
}

impl Particle {
    /// Create a particle at rest with the given lifetime and size.
    pub fn new(position: Vec2, lifetime: f32, size: f32) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            color: Color::WHITE,
            age: 0.0,
            lifetime: lifetime.max(1e-4),
            size: size.max(0.0),
            angular_velocity: 0.0,
            rotation: 0.0,
        }
    }

    /// Normalized life progress in `0..=1`.
    pub fn life_t(&self) -> f32 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// Remaining life fraction `1 - life_t`.
    pub fn remaining_t(&self) -> f32 {
        1.0 - self.life_t()
    }

    /// Whether the particle is still alive.
    pub fn alive(&self) -> bool {
        self.age < self.lifetime
    }

    /// Color with alpha scaled by remaining life (simple fade-out).
    pub fn faded_color(&self) -> Color {
        self.color.with_alpha(self.color.a * self.remaining_t())
    }
}

/// GPU-friendly packed particle for future buffer upload.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ParticleGpu {
    /// Position xy.
    pub position: [f32; 2],
    /// Size.
    pub size: f32,
    /// Rotation radians.
    pub rotation: f32,
    /// RGBA.
    pub color: [f32; 4],
    /// Normalized age `0..=1`.
    pub age_t: f32,
    /// Padding for 16-byte alignment friendliness.
    pub _pad: [f32; 3],
}

impl From<&Particle> for ParticleGpu {
    fn from(p: &Particle) -> Self {
        let c = p.faded_color();
        Self {
            position: [p.position.x, p.position.y],
            size: p.size,
            rotation: p.rotation,
            color: c.to_array(),
            age_t: p.life_t(),
            _pad: [0.0; 3],
        }
    }
}

/// Emitter-style configuration used when spawning particles.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleEmitter {
    /// Spawn origin.
    pub origin: Vec2,
    /// Base velocity.
    pub velocity: Vec2,
    /// Random half-range added to velocity (deterministic via seed helpers).
    pub velocity_jitter: Vec2,
    /// Particle lifetime seconds.
    pub lifetime: f32,
    /// Start size.
    pub size: f32,
    /// Start color.
    pub color: Color,
    /// Texture used when batched for draw.
    pub texture: TextureId,
    /// Gravity acceleration applied each tick.
    pub gravity: Vec2,
    /// Drag coefficient (velocity *= 1 - drag * dt).
    pub drag: f32,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            velocity: Vec2::ZERO,
            velocity_jitter: Vec2::ZERO,
            lifetime: 1.0,
            size: 8.0,
            color: Color::WHITE,
            texture: TextureId::NONE,
            gravity: Vec2::new(0.0, -200.0),
            drag: 0.0,
        }
    }
}

/// CPU particle batch: simulation + packing for GPU upload.
#[derive(Debug, Clone, Default)]
pub struct ParticleBatch {
    particles: Vec<Particle>,
    /// Texture for this batch (one texture per batch for simple uploads).
    pub texture: TextureId,
    /// Shared gravity if emitter not used per-spawn.
    pub gravity: Vec2,
    /// Shared drag.
    pub drag: f32,
    /// Max particles retained (oldest culled when exceeded).
    pub capacity: usize,
}

impl ParticleBatch {
    /// Create an empty batch with capacity limit.
    pub fn new(capacity: usize) -> Self {
        Self {
            particles: Vec::with_capacity(capacity.min(1024)),
            texture: TextureId::NONE,
            gravity: Vec2::new(0.0, -200.0),
            drag: 0.0,
            capacity: capacity.max(1),
        }
    }

    /// Number of live particles.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Live particles.
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Mutable particles.
    pub fn particles_mut(&mut self) -> &mut [Particle] {
        &mut self.particles
    }

    /// Clear all particles.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Spawn a fully specified particle.
    pub fn spawn(&mut self, particle: Particle) {
        if self.particles.len() >= self.capacity {
            // Drop oldest.
            self.particles.remove(0);
        }
        self.particles.push(particle);
    }

    /// Spawn from emitter; `jitter_sign` is a deterministic (-1..=1) pair.
    pub fn spawn_from_emitter(&mut self, emitter: &ParticleEmitter, jitter_sign: Vec2) {
        let mut p = Particle::new(emitter.origin, emitter.lifetime, emitter.size);
        p.velocity = emitter.velocity
            + Vec2::new(
                emitter.velocity_jitter.x * jitter_sign.x,
                emitter.velocity_jitter.y * jitter_sign.y,
            );
        p.color = emitter.color;
        self.texture = emitter.texture;
        self.gravity = emitter.gravity;
        self.drag = emitter.drag;
        self.spawn(p);
    }

    /// Advance simulation by `dt` seconds; removes dead particles.
    pub fn update(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        let gravity = self.gravity;
        let drag = self.drag.clamp(0.0, 20.0);
        for p in &mut self.particles {
            p.velocity += gravity * dt;
            if drag > 0.0 {
                let factor = (1.0 - drag * dt).max(0.0);
                p.velocity *= factor;
            }
            p.position += p.velocity * dt;
            p.rotation += p.angular_velocity * dt;
            p.age += dt;
        }
        self.particles.retain(Particle::alive);
    }

    /// Pack live particles for GPU upload.
    pub fn pack_gpu(&self) -> Vec<ParticleGpu> {
        self.particles.iter().map(ParticleGpu::from).collect()
    }

    /// Collect positions (for batch debug / tests).
    pub fn positions(&self) -> Vec<Vec2> {
        self.particles.iter().map(|p| p.position).collect()
    }

    /// Collect colors (faded).
    pub fn colors(&self) -> Vec<Color> {
        self.particles.iter().map(Particle::faded_color).collect()
    }

    /// Collect ages.
    pub fn ages(&self) -> Vec<f32> {
        self.particles.iter().map(|p| p.age).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_dies_after_lifetime() {
        let mut batch = ParticleBatch::new(16);
        batch.gravity = Vec2::ZERO;
        batch.spawn(Particle::new(Vec2::ZERO, 0.5, 4.0));
        batch.update(0.4);
        assert_eq!(batch.len(), 1);
        batch.update(0.2);
        assert!(batch.is_empty());
    }

    #[test]
    fn pack_gpu_matches_count() {
        let mut batch = ParticleBatch::new(8);
        batch.gravity = Vec2::ZERO;
        for i in 0..3 {
            let mut p = Particle::new(Vec2::new(i as f32, 0.0), 1.0, 2.0);
            p.color = Color::rgb(1.0, 0.0, 0.0);
            batch.spawn(p);
        }
        let gpu = batch.pack_gpu();
        assert_eq!(gpu.len(), 3);
        assert!((gpu[0].color[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn capacity_drops_oldest() {
        let mut batch = ParticleBatch::new(2);
        batch.spawn(Particle::new(Vec2::new(1.0, 0.0), 10.0, 1.0));
        batch.spawn(Particle::new(Vec2::new(2.0, 0.0), 10.0, 1.0));
        batch.spawn(Particle::new(Vec2::new(3.0, 0.0), 10.0, 1.0));
        assert_eq!(batch.len(), 2);
        assert!((batch.particles()[0].position.x - 2.0).abs() < 1e-5);
    }

    #[test]
    fn emitter_applies_jitter() {
        let mut batch = ParticleBatch::new(4);
        let emitter = ParticleEmitter {
            velocity: Vec2::new(10.0, 0.0),
            velocity_jitter: Vec2::new(5.0, 0.0),
            gravity: Vec2::ZERO,
            lifetime: 1.0,
            ..Default::default()
        };
        batch.spawn_from_emitter(&emitter, Vec2::new(1.0, 0.0));
        assert!((batch.particles()[0].velocity.x - 15.0).abs() < 1e-5);
    }
}
