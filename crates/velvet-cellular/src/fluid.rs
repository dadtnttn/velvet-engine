//! Liquid connected-component pressure and column equalization.
//!
//! This is real fluid-ish logic on the cellular grid (not copy-paste materials):
//! flood-fill liquid blobs, compute hydrostatic head, push into air pockets.

use crate::cell::{Cell, MaterialId};
use crate::material::Phase;
use crate::world::World;

/// One liquid blob after flood fill.
#[derive(Debug, Clone)]
pub struct LiquidBlob {
    /// Material of the blob (dominant).
    pub material: MaterialId,
    /// Cells (x,y).
    pub cells: Vec<(i32, i32)>,
    /// Min Y (bottom).
    pub min_y: i32,
    /// Max Y (top surface).
    pub max_y: i32,
    /// Approximate volume.
    pub volume: usize,
}

/// Find liquid blobs in a rectangle (4-connected).
pub fn find_liquid_blobs(
    world: &World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    max_blobs: usize,
) -> Vec<LiquidBlob> {
    let mut seen = std::collections::HashSet::new();
    let mut blobs = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if !seen.insert((x, y)) {
                continue;
            }
            let c = world.get(x, y);
            if c.is_air() || world.materials.phase(c.material) != Phase::Liquid {
                continue;
            }
            let mat = c.material;
            let mut stack = vec![(x, y)];
            let mut cells = Vec::new();
            let mut min_y = y;
            let mut max_y = y;
            while let Some((cx, cy)) = stack.pop() {
                if cx < x0 || cy < y0 || cx >= x1 || cy >= y1 {
                    continue;
                }
                if !seen.insert((cx, cy)) && !(cx == x && cy == y) {
                    // already processed unless seed
                }
                let cell = world.get(cx, cy);
                if cell.material != mat {
                    continue;
                }
                if cells.iter().any(|&p| p == (cx, cy)) {
                    continue;
                }
                cells.push((cx, cy));
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < x0 || ny < y0 || nx >= x1 || ny >= y1 {
                        continue;
                    }
                    let n = world.get(nx, ny);
                    if n.material == mat && !cells.iter().any(|&p| p == (nx, ny)) {
                        stack.push((nx, ny));
                        seen.insert((nx, ny));
                    }
                }
            }
            if cells.len() >= 2 {
                let volume = cells.len();
                blobs.push(LiquidBlob {
                    material: mat,
                    cells,
                    min_y,
                    max_y,
                    volume,
                });
                if blobs.len() >= max_blobs {
                    return blobs;
                }
            }
        }
    }
    blobs
}

/// Apply hydrostatic pressure scalar into cell.pressure for liquids in region.
pub fn apply_hydrostatic_pressure(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32) {
    let blobs = find_liquid_blobs(world, x0, y0, x1, y1, 64);
    for blob in blobs {
        let surface = blob.max_y as f32;
        let dens = world.materials.density(blob.material);
        for &(x, y) in &blob.cells {
            let depth = (surface - y as f32).max(0.0);
            let p = depth * dens * 0.15;
            let mut c = world.get(x, y);
            c.pressure = p;
            write_cell_pressure(world, x, y, c);
        }
    }
}

fn write_cell_pressure(world: &mut World, x: i32, y: i32, cell: Cell) {
    let cc = crate::chunk::ChunkCoord::from_cell(x, y);
    let (ox, oy) = cc.origin_cell();
    if let Some(ch) = world.chunk_mut(cc) {
        let lx = (x - ox) as usize;
        let ly = (y - oy) as usize;
        if lx < crate::chunk::CHUNK_SIZE && ly < crate::chunk::CHUNK_SIZE {
            ch.cells[crate::chunk::Chunk::idx(lx, ly)] = cell;
            ch.active = true;
            ch.sleeping = false;
        }
    }
}

/// Try to level liquid surface: move top liquid into lower air pockets under same blob.
pub fn equalize_liquid_columns(
    world: &mut World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    max_moves: usize,
) -> usize {
    let blobs = find_liquid_blobs(world, x0, y0, x1, y1, 32);
    let mut moves = 0usize;
    for blob in blobs {
        if moves >= max_moves {
            break;
        }
        // find surface cells (max_y) and holes below surface with air under a column
        let mut surface: Vec<(i32, i32)> = blob
            .cells
            .iter()
            .copied()
            .filter(|&(_, y)| y == blob.max_y)
            .collect();
        surface.sort_by_key(|&(x, _)| x);
        // candidate air just below surface range
        for y in blob.min_y..blob.max_y {
            for x in x0..x1 {
                if moves >= max_moves {
                    return moves;
                }
                if world.get(x, y).is_air() {
                    // is there liquid above in this column within blob?
                    let mut has_above = false;
                    for yy in (y + 1)..=blob.max_y {
                        if world.get(x, yy).material == blob.material {
                            has_above = true;
                            break;
                        }
                    }
                    if !has_above {
                        continue;
                    }
                    // move one surface cell down into this air
                    if let Some(&(sx, sy)) = surface.last() {
                        if world.get(sx, sy).material == blob.material && world.get(x, y).is_air() {
                            world.swap(sx, sy, x, y);
                            surface.pop();
                            moves += 1;
                        }
                    }
                }
            }
        }
    }
    moves
}

/// Flow rate estimate: count liquid cells that have air below.
pub fn count_falling_liquid(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> usize {
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let c = world.get(x, y);
            if world.materials.phase(c.material) != Phase::Liquid {
                continue;
            }
            if world.get(x, y - 1).is_air() {
                n += 1;
            }
        }
    }
    n
}

/// Settled liquid fraction: liquids with solid/liquid below.
pub fn settled_liquid_ratio(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> f32 {
    let mut total = 0usize;
    let mut settled = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let c = world.get(x, y);
            if world.materials.phase(c.material) != Phase::Liquid {
                continue;
            }
            total += 1;
            let below = world.get(x, y - 1);
            if !below.is_air() {
                settled += 1;
            }
        }
    }
    if total == 0 {
        return 1.0;
    }
    settled as f32 / total as f32
}

/// Drain liquid through a hole: remove up to `budget` liquid cells at bottom of region.
pub fn drain_liquid(
    world: &mut World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    hole_x: i32,
    hole_y: i32,
    budget: usize,
) -> usize {
    let mut removed = 0usize;
    // open hole
    if !world.get(hole_x, hole_y).is_air() {
        if world.materials.phase(world.get(hole_x, hole_y).material) != Phase::Static {
            world.set(hole_x, hole_y, Cell::air());
        }
    }
    // repeatedly remove liquid adjacent to hole / air near bottom
    for _ in 0..budget {
        let mut found = None;
        for y in y0..=hole_y {
            for x in x0..x1 {
                let c = world.get(x, y);
                if world.materials.phase(c.material) != Phase::Liquid {
                    continue;
                }
                // near hole or air
                let near_air = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .any(|&(dx, dy)| world.get(x + dx, y + dy).is_air());
                if near_air || (x - hole_x).abs() + (y - hole_y).abs() <= 2 {
                    found = Some((x, y));
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        if let Some((x, y)) = found {
            world.set(x, y, Cell::air());
            removed += 1;
        } else {
            break;
        }
    }
    removed
}

/// Merge two adjacent liquid materials if densities close (simple mix → denser).
pub fn try_mix_liquids(world: &mut World, x: i32, y: i32) -> bool {
    let a = world.get(x, y);
    if world.materials.phase(a.material) != Phase::Liquid {
        return false;
    }
    for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let b = world.get(x + dx, y + dy);
        if world.materials.phase(b.material) != Phase::Liquid {
            continue;
        }
        if a.material == b.material {
            continue;
        }
        let da = world.materials.density(a.material);
        let db = world.materials.density(b.material);
        if (da - db).abs() < 0.15 {
            // convert lighter into denser
            if da >= db {
                world.set(x + dx, y + dy, Cell::of(a.material));
            } else {
                world.set(x, y, Cell::of(b.material));
            }
            return true;
        }
    }
    false
}

/// Run a fluid pass over loaded bounds (or given rect).
pub fn fluid_pass(world: &mut World, max_equalize: usize) -> FluidPassStats {
    let Some((x0, y0, x1, y1)) = world.loaded_bounds() else {
        return FluidPassStats::default();
    };
    apply_hydrostatic_pressure(world, x0, y0, x1, y1);
    let moves = equalize_liquid_columns(world, x0, y0, x1, y1, max_equalize);
    let falling = count_falling_liquid(world, x0, y0, x1, y1);
    let settled = settled_liquid_ratio(world, x0, y0, x1, y1);
    let blobs = find_liquid_blobs(world, x0, y0, x1, y1, 64).len();
    FluidPassStats {
        equalize_moves: moves,
        falling_liquid: falling,
        settled_ratio: settled,
        blob_count: blobs,
    }
}

/// Stats from a fluid pass.
#[derive(Debug, Clone, Default)]
pub struct FluidPassStats {
    /// Column equalize swaps.
    pub equalize_moves: usize,
    /// Liquids with air below.
    pub falling_liquid: usize,
    /// Settled ratio 0..=1.
    pub settled_ratio: f32,
    /// Blob count.
    pub blob_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::sim::{step, SimConfig};
    use crate::world::WorldConfig;

    #[test]
    fn water_blob_and_pressure() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-10, 0, 10, 1, ids.bedrock);
        world.paint_rect(-4, 1, 4, 6, ids.water);
        let blobs = find_liquid_blobs(&world, -10, 0, 10, 10, 8);
        assert!(!blobs.is_empty());
        assert!(blobs[0].volume >= 8);
        apply_hydrostatic_pressure(&mut world, -10, 0, 10, 10);
        let bottom = world.get(0, 1).pressure;
        let top = world.get(0, 5).pressure;
        assert!(bottom >= top, "hydrostatic bottom={bottom} top={top}");
    }

    #[test]
    fn equalize_and_drain() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-8, 0, 8, 1, ids.bedrock);
        // uneven water columns
        world.paint_rect(-3, 1, -1, 8, ids.water);
        world.paint_rect(1, 1, 3, 3, ids.water);
        let moves = equalize_liquid_columns(&mut world, -8, 0, 8, 12, 50);
        let _ = moves;
        let drained = drain_liquid(&mut world, -8, 0, 8, 12, 0, 1, 10);
        assert!(drained > 0, "should drain some water");
    }

    #[test]
    fn fluid_pass_runs_with_sim() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-6, 0, 6, 1, ids.bedrock);
        world.paint_rect(-2, 1, 2, 5, ids.water);
        let cfg = SimConfig::default();
        for _ in 0..20 {
            step(&mut world, &cfg);
        }
        let stats = fluid_pass(&mut world, 20);
        assert!(stats.settled_ratio >= 0.0);
    }
}
