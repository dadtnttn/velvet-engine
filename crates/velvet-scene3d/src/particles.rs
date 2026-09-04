//! Deterministic CPU 3D particles with GPU-friendly upload views.
//!
//! [`ParticleSystem3D`] simulates position / velocity / gravity / drag /
//! life on fixed steps with a seeded RNG, so effects replay identically.
//! Hosts upload the packed [`ParticleSystem3D::positions`],
//! [`ParticleSystem3D::colors`], and [`ParticleSystem3D::sizes`] slices
//! (one entry per live particle) into point sprites or instanced quads.

use serde::{Deserialize, Serialize};
use velvet_math::{Vec3, XorShift32};

/// Spawn configuration for one emitter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Emitter3D {
    /// Emission point.
    #[serde(default)]
    pub origin: Vec3,
    /// Base initial velocity.
    #[serde(default)]
    pub velocity: Vec3,
    /// Random spread added per axis to the initial velocity.
    #[serde(default)]
    pub spread: Vec3,
    /// Particles per second (continuous).
    #[serde(default = "default_rate")]
    pub rate: f32,
    /// One-shot burst count on [`ParticleSystem3D::burst`].
    #[serde(default)]
    pub burst_count: u32,
    /// Lifetime range seconds `[min, max]`.
    #[serde(default = "default_life")]
    pub life: [f32; 2],
    /// Gravity applied to `y` velocity (negative = down).
    #[serde(default = "default_gravity")]
    pub gravity: f32,
    /// Linear drag per second (0 = none).
    #[serde(default)]
    pub drag: f32,
    /// Start size in world units.
    #[serde(default = "default_size")]
    pub size: f32,
    /// End size multiplier over life.
    #[serde(default = "default_size_over_life")]
    pub size_over_life: f32,
    /// Start RGBA.
    #[serde(default = "default_color")]
    pub color: [f32; 4],
    /// End RGBA (faded toward over life).
    #[serde(default = "default_end_color")]
    pub end_color: [f32; 4],
    /// Emission accumulator (serializable for saves).
    #[serde(default)]
    pub accumulator: f32,
}

fn default_rate() -> f32 {
    20.0
}
fn default_life() -> [f32; 2] {
    [0.6, 1.2]
}
fn default_gravity() -> f32 {
    -9.0
}
fn default_size() -> f32 {
    0.12
}
fn default_size_over_life() -> f32 {
    0.2
}
fn default_color() -> [f32; 4] {
    [1.0, 0.85, 0.5, 1.0]
}
fn default_end_color() -> [f32; 4] {
    [1.0, 0.4, 0.1, 0.0]
}

impl Default for Emitter3D {
    fn default() -> Self {
        Self {
            origin: Vec3::ZERO,
            velocity: Vec3::ZERO,
            spread: Vec3::ZERO,
            rate: default_rate(),
            burst_count: 0,
            life: default_life(),
            gravity: default_gravity(),
            drag: 0.0,
            size: default_size(),
            size_over_life: default_size_over_life(),
            color: default_color(),
            end_color: default_end_color(),
            accumulator: 0.0,
        }
    }
}

/// One live particle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Particle3D {
    /// Position.
    pub position: Vec3,
    /// Velocity.
    pub velocity: Vec3,
    /// Remaining life seconds.
    pub life: f32,
    /// Initial life seconds (for progress).
    pub max_life: f32,
    /// Start size.
    pub size: f32,
    /// Gravity snapshot from the emitter.
    pub gravity: f32,
    /// Drag snapshot from the emitter.
    pub drag: f32,
    /// Start color snapshot.
    pub color: [f32; 4],
    /// End color snapshot.
    pub end_color: [f32; 4],
    /// End size multiplier snapshot.
    pub size_over_life: f32,
}

/// Particle system: emitters + live particles + seeded RNG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleSystem3D {
    /// Emitters.
    pub emitters: Vec<Emitter3D>,
    /// Live particles.
    pub particles: Vec<Particle3D>,
    /// RNG state (serializable → replays stay identical across saves).
    pub rng_state: u32,
    /// Hard cap on live particles (oldest die first).
    pub max_particles: usize,
}

impl ParticleSystem3D {
    /// Empty system with a seed.
    pub fn new(seed: u32) -> Self {
        Self {
            emitters: Vec::new(),
            particles: Vec::new(),
            rng_state: seed.max(1),
            max_particles: 1024,
        }
    }

    /// Advance simulation by `dt` (clamped to 1/20 for stability).
    pub fn tick(&mut self, dt: f32) {
        let dt = dt.clamp(0.0, 0.05);
        let mut rng = XorShift32::new(self.rng_state);
        // Count spawns first (emitter borrow), then spawn (system borrow).
        let mut jobs: Vec<(Emitter3D, usize)> = Vec::new();
        for emitter in &mut self.emitters {
            emitter.accumulator += emitter.rate.max(0.0) * dt;
            let mut n = 0;
            while emitter.accumulator >= 1.0 {
                emitter.accumulator -= 1.0;
                n += 1;
            }
            if n > 0 {
                jobs.push((emitter.clone(), n));
            }
        }
        for (emitter, n) in &jobs {
            for _ in 0..*n {
                self.spawn(emitter, &mut rng);
            }
        }
        for p in &mut self.particles {
            p.life -= dt;
            p.velocity.y += p.gravity * dt;
            let drag = 1.0 / (1.0 + p.drag * dt);
            p.velocity = Vec3::new(p.velocity.x * drag, p.velocity.y, p.velocity.z * drag);
            p.position = Vec3::new(
                p.position.x + p.velocity.x * dt,
                p.position.y + p.velocity.y * dt,
                p.position.z + p.velocity.z * dt,
            );
        }
        self.particles.retain(|p| p.life > 0.0);
        while self.particles.len() > self.max_particles {
            self.particles.remove(0);
        }
        self.rng_state = rng.next_u32().max(1);
    }

    /// Emit `burst_count` particles from emitter `index` immediately.
    pub fn burst(&mut self, index: usize) {
        let mut rng = XorShift32::new(self.rng_state);
        if let Some(emitter) = self.emitters.get(index).cloned() {
            for _ in 0..emitter.burst_count {
                self.spawn(&emitter, &mut rng);
            }
        }
        self.rng_state = rng.next_u32().max(1);
    }

    fn spawn(&mut self, emitter: &Emitter3D, rng: &mut XorShift32) {
        let jitter = |rng: &mut XorShift32, spread: f32| (rng.next_f32() * 2.0 - 1.0) * spread;
        let life = emitter.life[0] + rng.next_f32() * (emitter.life[1] - emitter.life[0]).max(0.0);
        self.particles.push(Particle3D {
            position: emitter.origin,
            velocity: Vec3::new(
                emitter.velocity.x + jitter(rng, emitter.spread.x),
                emitter.velocity.y + jitter(rng, emitter.spread.y),
                emitter.velocity.z + jitter(rng, emitter.spread.z),
            ),
            life: life.max(0.01),
            max_life: life.max(0.01),
            size: emitter.size,
            gravity: emitter.gravity,
            drag: emitter.drag,
            color: emitter.color,
            end_color: emitter.end_color,
            size_over_life: emitter.size_over_life,
        });
    }

    /// Live count.
    pub fn len(&self) -> usize {
        self.particles.len()
    }

    /// Empty check.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Packed positions (one `[x, y, z]` per live particle).
    pub fn positions(&self) -> Vec<[f32; 3]> {
        self.particles
            .iter()
            .map(|p| [p.position.x, p.position.y, p.position.z])
            .collect()
    }

    /// Packed RGBA with life fade applied.
    pub fn colors(&self) -> Vec<[f32; 4]> {
        self.particles
            .iter()
            .map(|p| {
                let k = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
                let (from, to) = (p.color, p.end_color);
                [
                    from[0] + (to[0] - from[0]) * k,
                    from[1] + (to[1] - from[1]) * k,
                    from[2] + (to[2] - from[2]) * k,
                    from[3] + (to[3] - from[3]) * k,
                ]
            })
            .collect()
    }

    /// Packed sizes with over-life multiplier applied.
    pub fn sizes(&self) -> Vec<f32> {
        self.particles
            .iter()
            .map(|p| {
                let k = 1.0 - (p.life / p.max_life).clamp(0.0, 1.0);
                (p.size * (1.0 + (p.size_over_life - 1.0) * k)).max(0.0)
            })
            .collect()
    }
}

impl Default for ParticleSystem3D {
    fn default() -> Self {
        Self::new(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spark_emitter() -> Emitter3D {
        Emitter3D {
            rate: 60.0,
            life: [1.0, 1.0],
            gravity: 0.0,
            drag: 0.0,
            ..Default::default()
        }
    }

    #[test]
    fn emission_matches_rate() {
        let mut sys = ParticleSystem3D::new(7);
        sys.emitters.push(spark_emitter());
        sys.tick(0.05);
        assert_eq!(sys.len(), 3);
    }

    #[test]
    fn particles_die_on_schedule() {
        let mut sys = ParticleSystem3D::new(7);
        sys.emitters.push(spark_emitter());
        sys.tick(0.05);
        assert_eq!(sys.len(), 3);
        // Rate 60/s × life 1.0s ≈ 60 alive at steady state; what matters
        // is stability (no leak, no drain) and the life bound — the exact
        // integer depends on f32 boundary arithmetic, so pin the window.
        for _ in 0..40 {
            sys.tick(0.05);
        }
        let steady = sys.len();
        assert!((55..=60).contains(&steady), "steady={steady}");
        for _ in 0..10 {
            sys.tick(0.05);
        }
        assert_eq!(sys.len(), steady, "population must not drift");
        for p in &sys.particles {
            assert!(p.life <= 1.0);
        }
    }

    #[test]
    fn burst_emits_configured_count() {
        let mut sys = ParticleSystem3D::new(7);
        sys.emitters.push(Emitter3D {
            rate: 0.0,
            burst_count: 25,
            ..Default::default()
        });
        sys.burst(0);
        assert_eq!(sys.len(), 25);
    }

    #[test]
    fn simulation_is_deterministic() {
        let run = || {
            let mut sys = ParticleSystem3D::new(42);
            sys.emitters.push(Emitter3D {
                spread: Vec3::new(1.0, 2.0, 1.0),
                velocity: Vec3::new(0.0, 3.0, 0.0),
                ..spark_emitter()
            });
            for _ in 0..30 {
                sys.tick(1.0 / 60.0);
            }
            sys.positions()
        };
        let first = run();
        let second = run();
        assert_eq!(first, second);
    }

    #[test]
    fn upload_views_stay_in_sync() {
        let mut sys = ParticleSystem3D::new(7);
        sys.emitters.push(spark_emitter());
        sys.tick(0.05);
        // Non-vacuous: particles must actually exist for the sync to mean
        // anything (a 0.016 tick would spawn none at rate 60/s).
        assert_eq!(sys.len(), 3);
        assert_eq!(sys.positions().len(), 3);
        assert_eq!(sys.colors().len(), 3);
        assert_eq!(sys.sizes().len(), 3);
        // Views reflect live state, not stale snapshots.
        for _ in 0..40 {
            sys.tick(0.05);
        }
        assert_eq!(sys.positions().len(), sys.len());
        assert!(sys.len() > 3);
    }
}
