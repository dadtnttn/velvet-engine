//! Simulation debug overlays and integrity checks for authors.

use crate::cell::CellFlags;
use crate::chunk::CHUNK_SIZE;
use crate::material::Phase;
use crate::world::World;

/// Integrity report.
#[derive(Debug, Clone, Default)]
pub struct IntegrityReport {
    /// Loaded chunks.
    pub chunks: usize,
    /// Non-air cells.
    pub occupied: usize,
    /// Cells with MOVED flag stuck.
    pub stuck_moved_flags: usize,
    /// NaN temps.
    pub bad_temps: usize,
    /// Negative life (impossible with u16 — always 0).
    pub ok: bool,
}

/// Scan world for anomalies.
pub fn check_integrity(world: &World) -> IntegrityReport {
    let mut r = IntegrityReport {
        chunks: world.loaded_chunks().count(),
        occupied: world.occupied_cells(),
        ..Default::default()
    };
    if let Some((x0, y0, x1, y1)) = world.loaded_bounds() {
        for y in y0..y1 {
            for x in x0..x1 {
                let c = world.get(x, y);
                if c.temp.is_nan() || c.temp.is_infinite() {
                    r.bad_temps += 1;
                }
                if c.flags.contains(CellFlags::MOVED) {
                    r.stuck_moved_flags += 1;
                }
            }
        }
    }
    r.ok = r.bad_temps == 0;
    r
}

/// ASCII debug dump of a small window (for tests / CLI).
pub fn ascii_window(world: &World, x0: i32, y0: i32, w: i32, h: i32) -> String {
    let mut s = String::new();
    for y in (y0..y0 + h).rev() {
        for x in x0..x0 + w {
            let c = world.get(x, y);
            let ch = if c.is_air() {
                '.'
            } else {
                match world.materials.phase(c.material) {
                    Phase::Powder => '*',
                    Phase::Liquid => '~',
                    Phase::Solid => '#',
                    Phase::Static => 'X',
                    Phase::Gas => ':',
                    Phase::Plasma => '^',
                }
            };
            s.push(ch);
        }
        s.push('\n');
    }
    s
}

/// Count burning cells.
pub fn count_burning(world: &World) -> usize {
    let mut n = 0usize;
    if let Some((x0, y0, x1, y1)) = world.loaded_bounds() {
        for y in y0..y1 {
            for x in x0..x1 {
                let c = world.get(x, y);
                if c.flags.contains(CellFlags::BURNING)
                    || world.materials.phase(c.material) == Phase::Plasma
                {
                    n += 1;
                }
            }
        }
    }
    n
}

/// Estimate active chunk ratio.
pub fn active_chunk_ratio(world: &World) -> f32 {
    let total = world.loaded_chunks().count();
    if total == 0 {
        return 0.0;
    }
    let active = world.active_chunk_coords().len();
    active as f32 / total as f32
}

/// Material diversity (unique non-air materials in bounds).
pub fn material_diversity(world: &World) -> usize {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    if let Some((x0, y0, x1, y1)) = world.loaded_bounds() {
        for y in y0..y1 {
            for x in x0..x1 {
                let c = world.get(x, y);
                if !c.is_air() {
                    set.insert(c.material.0);
                }
            }
        }
    }
    set.len()
}

/// Validate chunk size constant assumptions.
pub fn chunk_layout_ok() -> bool {
    CHUNK_SIZE >= 16 && CHUNK_SIZE <= 128 && (CHUNK_SIZE as u32).is_power_of_two()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::cell::Cell;
    use crate::world::WorldConfig;

    #[test]
    fn integrity_and_ascii() {
        assert!(chunk_layout_ok());
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-4, 0, 4, 1, ids.bedrock);
        world.set(0, 3, Cell::of(ids.sand));
        let rep = check_integrity(&world);
        assert!(rep.ok);
        assert!(rep.occupied >= 1);
        let dump = ascii_window(&world, -4, 0, 8, 6);
        assert!(
            dump.contains('X'),
            "bedrock missing from dump:
{dump}"
        );
        assert!(
            dump.contains('*'),
            "sand missing from dump:
{dump}"
        );
        assert!(material_diversity(&world) >= 1);
    }
}
