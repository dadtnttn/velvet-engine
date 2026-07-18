//! High-level world queries for authors (counts, scans, samples).

use crate::cell::MaterialId;
use crate::material::Phase;
use crate::world::World;

/// Histogram of materials in a rect.
#[derive(Debug, Clone, Default)]
pub struct MaterialHistogram {
    /// Pairs (material, count).
    pub counts: Vec<(MaterialId, usize)>,
    /// Total non-air.
    pub solidish: usize,
    /// Air cells sampled.
    pub air: usize,
}

/// Count materials in region.
pub fn histogram(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> MaterialHistogram {
    use std::collections::HashMap;
    let mut map: HashMap<u16, usize> = HashMap::new();
    let mut air = 0usize;
    let mut solidish = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let c = world.get(x, y);
            if c.is_air() {
                air += 1;
            } else {
                solidish += 1;
                *map.entry(c.material.0).or_default() += 1;
            }
        }
    }
    let mut counts: Vec<_> = map
        .into_iter()
        .map(|(id, n)| (MaterialId(id), n))
        .collect();
    counts.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    MaterialHistogram {
        counts,
        solidish,
        air,
    }
}

/// Find all cells of a material (capped).
pub fn find_material(
    world: &World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    mat: MaterialId,
    max: usize,
) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if world.get(x, y).material == mat {
                out.push((x, y));
                if out.len() >= max {
                    return out;
                }
            }
        }
    }
    out
}

/// Bounding box of non-air cells in region.
pub fn non_air_bounds(
    world: &World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Option<(i32, i32, i32, i32)> {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut any = false;
    for y in y0..y1 {
        for x in x0..x1 {
            if !world.get(x, y).is_air() {
                any = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    if any {
        Some((min_x, min_y, max_x + 1, max_y + 1))
    } else {
        None
    }
}

/// Sample surface Y for each X (highest non-air).
pub fn surface_profile(world: &World, x0: i32, x1: i32, y0: i32, y1: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for x in x0..x1 {
        let mut top = None;
        for y in (y0..y1).rev() {
            if !world.get(x, y).is_air() {
                top = Some(y);
                break;
            }
        }
        if let Some(y) = top {
            out.push((x, y));
        }
    }
    out
}

/// Count by phase.
pub fn phase_counts(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> PhaseCounts {
    let mut c = PhaseCounts::default();
    for y in y0..y1 {
        for x in x0..x1 {
            let cell = world.get(x, y);
            if cell.is_air() {
                c.air += 1;
                continue;
            }
            match world.materials.phase(cell.material) {
                Phase::Powder => c.powder += 1,
                Phase::Liquid => c.liquid += 1,
                Phase::Solid => c.solid += 1,
                Phase::Static => c.static_cells += 1,
                Phase::Gas => c.gas += 1,
                Phase::Plasma => c.plasma += 1,
            }
        }
    }
    c
}

/// Phase histogram.
#[derive(Debug, Clone, Default)]
pub struct PhaseCounts {
    /// Air.
    pub air: usize,
    /// Powder.
    pub powder: usize,
    /// Liquid.
    pub liquid: usize,
    /// Solid.
    pub solid: usize,
    /// Static.
    pub static_cells: usize,
    /// Gas.
    pub gas: usize,
    /// Plasma.
    pub plasma: usize,
}

/// Temperature stats.
#[derive(Debug, Clone, Default)]
pub struct TempStats {
    /// Min.
    pub min: f32,
    /// Max.
    pub max: f32,
    /// Average.
    pub avg: f32,
    /// Samples.
    pub n: usize,
}

/// Temperature range over non-air cells.
pub fn temperature_stats(world: &World, x0: i32, y0: i32, x1: i32, y1: i32) -> TempStats {
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    let mut sum = 0.0f32;
    let mut n = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let c = world.get(x, y);
            if c.is_air() {
                continue;
            }
            min = min.min(c.temp);
            max = max.max(c.temp);
            sum += c.temp;
            n += 1;
        }
    }
    if n == 0 {
        return TempStats::default();
    }
    TempStats {
        min,
        max,
        avg: sum / n as f32,
        n,
    }
}

/// Nearest cell of material from point (BFS).
pub fn nearest_material(
    world: &World,
    x: i32,
    y: i32,
    mat: MaterialId,
    max_radius: i32,
) -> Option<(i32, i32)> {
    use std::collections::VecDeque;
    let mut q = VecDeque::new();
    let mut seen = std::collections::HashSet::new();
    q.push_back((x, y, 0));
    seen.insert((x, y));
    while let Some((cx, cy, d)) = q.pop_front() {
        if d > max_radius {
            break;
        }
        if world.get(cx, cy).material == mat {
            return Some((cx, cy));
        }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let nx = cx + dx;
            let ny = cy + dy;
            if seen.insert((nx, ny)) {
                q.push_back((nx, ny, d + 1));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::cell::Cell;
    use crate::world::WorldConfig;

    #[test]
    fn histogram_and_surface() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-5, 0, 5, 2, ids.bedrock);
        world.paint_circle(0, 5, 3, ids.sand);
        let h = histogram(&world, -10, 0, 10, 12);
        assert!(h.solidish > 0);
        assert!(h.counts.iter().any(|(m, _)| *m == ids.sand || *m == ids.bedrock));
        let surf = surface_profile(&world, -5, 5, 0, 12);
        assert!(!surf.is_empty());
        let pc = phase_counts(&world, -10, 0, 10, 12);
        assert!(pc.static_cells > 0 || pc.solid > 0 || pc.powder > 0);
        let ts = temperature_stats(&world, -10, 0, 10, 12);
        assert!(ts.n > 0);
        assert!(nearest_material(&world, 0, 20, ids.sand, 30).is_some());
    }
}
