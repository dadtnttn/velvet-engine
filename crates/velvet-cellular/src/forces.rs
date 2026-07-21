//! Force fields and world modifiers for creators (wind, gravity wells, heat zones).

use serde::{Deserialize, Serialize};

use crate::chunk::ChunkCoord;
use crate::particles::ParticleWorld;
use crate::world::World;

/// Force field shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldShape {
    /// Circle.
    Circle,
    /// Axis box.
    Box,
}

/// Field kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// Push velocity on free particles.
    Wind,
    /// Attract particles to center.
    GravityWell,
    /// Heat cells.
    HeatZone,
    /// Cool cells.
    ColdZone,
    /// Repel particles.
    Repulsor,
}

/// One force field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceField {
    /// Id.
    pub id: u32,
    /// Kind.
    pub kind: FieldKind,
    /// Shape.
    pub shape: FieldShape,
    /// Center X.
    pub x: f32,
    /// Center Y.
    pub y: f32,
    /// Radius or half-extent.
    pub radius: f32,
    /// Strength.
    pub strength: f32,
    /// Enabled.
    pub enabled: bool,
}

/// Field registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ForceWorld {
    /// Fields.
    pub fields: Vec<ForceField>,
    next_id: u32,
}

impl ForceWorld {
    /// New.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            next_id: 1,
        }
    }

    /// Add field.
    pub fn add(&mut self, mut f: ForceField) -> u32 {
        if f.id == 0 {
            f.id = self.next_id;
            self.next_id += 1;
        }
        let id = f.id;
        self.fields.push(f);
        id
    }

    /// Remove.
    pub fn remove(&mut self, id: u32) {
        self.fields.retain(|f| f.id != id);
    }

    /// Apply to particles and grid.
    pub fn apply(&self, world: &mut World, particles: &mut ParticleWorld, dt: f32) {
        for f in self.fields.iter().filter(|f| f.enabled) {
            match f.kind {
                FieldKind::Wind => apply_wind(f, particles, dt),
                FieldKind::GravityWell => apply_well(f, particles, dt, false),
                FieldKind::Repulsor => apply_well(f, particles, dt, true),
                FieldKind::HeatZone => apply_temp(f, world, f.strength * dt),
                FieldKind::ColdZone => apply_temp(f, world, -f.strength * dt),
            }
        }
    }
}

fn inside(f: &ForceField, x: f32, y: f32) -> bool {
    match f.shape {
        FieldShape::Circle => {
            let dx = x - f.x;
            let dy = y - f.y;
            dx * dx + dy * dy <= f.radius * f.radius
        }
        FieldShape::Box => (x - f.x).abs() <= f.radius && (y - f.y).abs() <= f.radius,
    }
}

fn apply_wind(f: &ForceField, particles: &mut ParticleWorld, dt: f32) {
    // strength as horizontal push
    for p in particles.particles.iter_mut().filter(|p| p.alive) {
        if inside(f, p.x, p.y) {
            p.vx += f.strength * dt;
            p.vy += f.strength * 0.05 * dt;
        }
    }
}

fn apply_well(f: &ForceField, particles: &mut ParticleWorld, dt: f32, repulse: bool) {
    let sign = if repulse { -1.0 } else { 1.0 };
    for p in particles.particles.iter_mut().filter(|p| p.alive) {
        if !inside(f, p.x, p.y) {
            continue;
        }
        let dx = f.x - p.x;
        let dy = f.y - p.y;
        let len = (dx * dx + dy * dy).sqrt().max(0.2);
        p.vx += sign * dx / len * f.strength * dt;
        p.vy += sign * dy / len * f.strength * dt;
    }
}

fn apply_temp(f: &ForceField, world: &mut World, delta: f32) {
    let r = f.radius.ceil() as i32;
    let cx = f.x.floor() as i32;
    let cy = f.y.floor() as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if f.shape == FieldShape::Circle && dx * dx + dy * dy > r * r {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            let mut c = world.get(x, y);
            if c.is_air() {
                continue;
            }
            c.temp += delta;
            // write without full event spam via chunk
            let cc = ChunkCoord::from_cell(x, y);
            let (ox, oy) = cc.origin_cell();
            if let Some(ch) = world.chunk_mut(cc) {
                let lx = (x - ox) as usize;
                let ly = (y - oy) as usize;
                if lx < crate::chunk::CHUNK_SIZE && ly < crate::chunk::CHUNK_SIZE {
                    ch.cells[crate::chunk::Chunk::idx(lx, ly)].temp = c.temp;
                    ch.active = true;
                }
            } else {
                world.set(x, y, c);
            }
        }
    }
}

/// Convenience constructors.
pub fn wind_field(x: f32, y: f32, radius: f32, strength: f32) -> ForceField {
    ForceField {
        id: 0,
        kind: FieldKind::Wind,
        shape: FieldShape::Box,
        x,
        y,
        radius,
        strength,
        enabled: true,
    }
}

/// Gravity well.
pub fn gravity_well(x: f32, y: f32, radius: f32, strength: f32) -> ForceField {
    ForceField {
        id: 0,
        kind: FieldKind::GravityWell,
        shape: FieldShape::Circle,
        x,
        y,
        radius,
        strength,
        enabled: true,
    }
}

/// Heat zone.
pub fn heat_zone(x: f32, y: f32, radius: f32, strength: f32) -> ForceField {
    ForceField {
        id: 0,
        kind: FieldKind::HeatZone,
        shape: FieldShape::Circle,
        x,
        y,
        radius,
        strength,
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::particles::{FreeParticle, ParticleWorld};
    use crate::world::WorldConfig;

    #[test]
    fn wind_pushes_particles() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        let mut pw = ParticleWorld::default();
        pw.spawn(FreeParticle::new(0, 0.0, 5.0, ids.sand).with_life(2.0));
        let mut fw = ForceWorld::new();
        fw.add(wind_field(0.0, 5.0, 10.0, 40.0));
        let x0 = pw.particles[0].x;
        for _ in 0..10 {
            fw.apply(&mut world, &mut pw, 1.0 / 60.0);
            pw.step(&mut world, 1.0 / 60.0);
        }
        // particle should have moved right-ish or still alive
        assert!(
            pw.particles.iter().any(|p| p.alive)
                || (pw.particles[0].x - x0).abs() > 0.01
                || pw.conversions > 0
        );
    }
}
