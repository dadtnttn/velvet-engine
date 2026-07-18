//! Deterministic action replay for author tools / tests / netcode hooks.

use serde::{Deserialize, Serialize};

use crate::brush::{BrushMode, BrushShape};
use crate::particles::ParticleWorld;
use crate::session::CellularSession;
use crate::world::World;

/// One recorded author action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayAction {
    /// Step sim N times.
    Step(u32),
    /// Paint circle.
    Paint {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Radius.
        r: i32,
        /// Material key.
        material: String,
    },
    /// Brush stamp.
    Brush {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Mode name.
        mode: String,
        /// Shape name.
        shape: String,
        /// Radius.
        radius: i32,
        /// Material key.
        material: String,
    },
    /// Particle burst.
    Burst {
        /// X.
        x: f32,
        /// Y.
        y: f32,
        /// Material.
        material: String,
        /// Count.
        count: u32,
    },
    /// Cast spell.
    Cast {
        /// Spell key.
        spell: String,
        /// X.
        x: f32,
        /// Y.
        y: f32,
    },
    /// Dig.
    Dig {
        /// X.
        x: i32,
        /// Y.
        y: i32,
        /// Radius.
        r: i32,
    },
    /// Seed platform.
    SeedDemo,
}

/// Recording buffer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplayLog {
    /// Actions in order.
    pub actions: Vec<ReplayAction>,
    /// RNG seed snapshot.
    pub seed: u64,
}

impl ReplayLog {
    /// New with seed.
    pub fn new(seed: u64) -> Self {
        Self {
            actions: Vec::new(),
            seed,
        }
    }

    /// Push.
    pub fn push(&mut self, a: ReplayAction) {
        self.actions.push(a);
    }

    /// Len.
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

/// Apply one action to session.
pub fn apply_action(session: &mut CellularSession, action: &ReplayAction) {
    match action {
        ReplayAction::Step(n) => session.step_n(*n),
        ReplayAction::Paint { x, y, r, material } => {
            session.paint(*x, *y, *r, material);
        }
        ReplayAction::Brush {
            x,
            y,
            mode,
            shape,
            radius,
            material,
        } => {
            session.brush_material(material);
            session.brush_radius(*radius);
            session.brush_mode(parse_mode(mode));
            session.brush_shape(parse_shape(shape));
            session.brush_down(*x, *y);
            session.brush_up();
        }
        ReplayAction::Burst {
            x,
            y,
            material,
            count,
        } => {
            session.particle_burst(*x, *y, material, *count);
        }
        ReplayAction::Cast { spell, x, y } => {
            let _ = session.cast_spell(spell, *x, *y);
        }
        ReplayAction::Dig { x, y, r } => {
            crate::agent::dig_at(&mut session.world, &mut session.particles, *x, *y, *r);
            session.hot.touch(*x, *y);
        }
        ReplayAction::SeedDemo => session.seed_demo_platform(),
    }
}

/// Replay entire log onto session.
pub fn play_log(session: &mut CellularSession, log: &ReplayLog) {
    session.world.config.seed = log.seed;
    session.world.rng = log.seed | 1;
    for a in &log.actions {
        apply_action(session, a);
    }
}

fn parse_mode(s: &str) -> BrushMode {
    match s.to_ascii_lowercase().as_str() {
        "erase" => BrushMode::Erase,
        "replace" => BrushMode::Replace,
        "heat" => BrushMode::Heat,
        "cool" => BrushMode::Cool,
        "ignite" => BrushMode::Ignite,
        "bleed" => BrushMode::Bleed,
        "dig" => BrushMode::Dig,
        "sample" => BrushMode::Sample,
        _ => BrushMode::Paint,
    }
}

fn parse_shape(s: &str) -> BrushShape {
    match s.to_ascii_lowercase().as_str() {
        "square" => BrushShape::Square,
        "diamond" => BrushShape::Diamond,
        "point" => BrushShape::Point,
        "spray" => BrushShape::Spray,
        _ => BrushShape::Circle,
    }
}

/// Hash of occupied cells for determinism checks.
pub fn world_fingerprint(world: &World) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    if let Some((x0, y0, x1, y1)) = world.loaded_bounds() {
        for y in y0..y1 {
            for x in x0..x1 {
                let c = world.get(x, y);
                if c.is_air() {
                    continue;
                }
                h ^= (c.material.0 as u64)
                    .wrapping_mul(0x9E37)
                    .wrapping_add(x as u64)
                    .wrapping_add((y as u64) << 32);
                h = h.rotate_left(7).wrapping_mul(0x100_0000_01b3);
            }
        }
    }
    h
}

/// Fingerprint particles.
pub fn particle_fingerprint(pw: &ParticleWorld) -> u64 {
    let mut h = 0u64;
    for p in pw.particles.iter().filter(|p| p.alive) {
        h ^= (p.x.to_bits() as u64).wrapping_add(p.y.to_bits() as u64);
        h = h.rotate_left(3).wrapping_add(p.material.0 as u64);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::CellularSession;
    use crate::world::WorldConfig;

    #[test]
    fn replay_is_deterministic() {
        let mut log = ReplayLog::new(42);
        log.push(ReplayAction::SeedDemo);
        log.push(ReplayAction::Paint {
            x: 0,
            y: 12,
            r: 3,
            material: "sand".into(),
        });
        log.push(ReplayAction::Burst {
            x: 2.0,
            y: 15.0,
            material: "water".into(),
            count: 12,
        });
        log.push(ReplayAction::Step(25));

        let mut a = CellularSession::with_builtins(WorldConfig {
            seed: 42,
            ..WorldConfig::default()
        });
        a.use_hot = false;
        play_log(&mut a, &log);
        let fa = world_fingerprint(&a.world);

        let mut b = CellularSession::with_builtins(WorldConfig {
            seed: 42,
            ..WorldConfig::default()
        });
        b.use_hot = false;
        play_log(&mut b, &log);
        let fb = world_fingerprint(&b.world);
        assert_eq!(fa, fb, "replay fingerprints must match");
        assert!(a.world.occupied_cells() > 0);
    }
}
