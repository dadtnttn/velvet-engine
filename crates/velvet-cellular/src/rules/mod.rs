//! Pluggable simulation rules — pure functions over local neighborhoods.

mod blood;
mod density;
mod dissolve;
mod explosion;
mod fire;
mod gravity;
mod pressure;
mod temperature;

pub use blood::{rule_blood, splatter_blood};
pub use density::rule_density_sink;
pub use dissolve::rule_dissolve;
pub use explosion::rule_explosion;
pub use fire::rule_fire;
pub use gravity::rule_gravity;
pub use pressure::rule_pressure_diffuse;
pub use temperature::rule_temperature;

use crate::cell::{Cell, CellFlags};
use crate::material::{MaterialRegistry, Phase};
use crate::world::World;

/// Context for a single cell update attempt.
pub struct RuleCtx<'a> {
    /// World.
    pub world: &'a mut World,
    /// Cell X.
    pub x: i32,
    /// Cell Y.
    pub y: i32,
}

impl RuleCtx<'_> {
    /// Materials.
    pub fn mats(&self) -> &MaterialRegistry {
        &self.world.materials
    }

    /// Get.
    pub fn get(&self, x: i32, y: i32) -> Cell {
        self.world.get(x, y)
    }

    /// Current cell.
    pub fn cell(&self) -> Cell {
        self.world.get(self.x, self.y)
    }

    /// Set current.
    pub fn set_here(&mut self, cell: Cell) {
        self.world.set(self.x, self.y, cell);
    }

    /// Swap with neighbor.
    pub fn swap(&mut self, nx: i32, ny: i32) {
        self.world.swap(self.x, self.y, nx, ny);
        // mark moved on destination roughly via wake
    }

    /// Phase of material.
    pub fn phase(&self, id: crate::cell::MaterialId) -> Phase {
        self.world.materials.phase(id)
    }

    /// Random.
    pub fn chance(&mut self, p: f32) -> bool {
        self.world.chance(p)
    }
}

/// Clear MOVED flags in a region (called between passes).
/// Also clears chunk `active` so only cells that move this step keep the chunk hot.
pub fn clear_moved_flags(world: &mut World, coords: &[crate::chunk::ChunkCoord]) {
    use crate::chunk::CHUNK_SIZE;
    for cc in coords {
        if let Some(ch) = world.chunk_mut(*cc) {
            ch.active = false;
            for c in &mut ch.cells {
                c.flags.remove(CellFlags::MOVED);
            }
            let _ = CHUNK_SIZE;
        }
    }
}
