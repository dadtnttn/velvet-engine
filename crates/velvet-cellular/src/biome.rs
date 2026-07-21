//! Biome stamps for authors — compose caves, ores, vegetation quickly.

use crate::cell::Cell;
use crate::growth::{growth_pass, GrowthConfig};
use crate::procgen::{generate_caves, scatter_blobs, CaveOptions};
use crate::reaction_chain::apply_reaction_chains;
use crate::world::World;

/// Biome stamp parameters.
#[derive(Debug, Clone)]
pub struct BiomeStamp {
    /// Rect min X.
    pub x0: i32,
    /// Min Y.
    pub y0: i32,
    /// Max X exclusive.
    pub x1: i32,
    /// Max Y exclusive.
    pub y1: i32,
    /// Seed.
    pub seed: u64,
    /// Fill caves.
    pub caves: bool,
    /// Scatter coal/iron/gold ores.
    pub ores: bool,
    /// Plant life pass.
    pub vegetation: bool,
    /// Open threshold for caves.
    pub cave_open: f32,
}

impl Default for BiomeStamp {
    fn default() -> Self {
        Self {
            x0: -64,
            y0: -8,
            x1: 64,
            y1: 48,
            seed: 1,
            caves: true,
            ores: true,
            vegetation: true,
            cave_open: 0.5,
        }
    }
}

/// Apply a cave + ore + vegetation biome into the world.
pub fn stamp_biome(world: &mut World, stamp: &BiomeStamp) -> BiomeStats {
    let mut stats = BiomeStats::default();
    let stone = world.mat("stone");
    let bed = world.mat("bedrock");
    let dirt = world.mat("dirt");
    let grass = world.mat("grass");
    let coal = world.mat("coal_ore");
    let iron = world.mat("iron_ore");
    let gold = world.mat("gold_ore");

    if stamp.caves {
        generate_caves(
            world,
            &CaveOptions {
                x0: stamp.x0,
                y0: stamp.y0,
                x1: stamp.x1,
                y1: stamp.y1,
                solid: if stone.is_air() { bed } else { stone },
                border: bed,
                open_threshold: stamp.cave_open,
                scale: 0.09,
                seed: stamp.seed,
                border_thickness: 2,
            },
        );
        stats.caves = true;
    }

    // surface dirt/grass band
    if !dirt.is_air() {
        for x in stamp.x0..stamp.x1 {
            // find highest solid
            let mut top = None;
            for y in (stamp.y0..stamp.y1).rev() {
                if !world.get(x, y).is_air() {
                    top = Some(y);
                    break;
                }
            }
            if let Some(y) = top {
                if y + 1 < stamp.y1 {
                    world.set(x, y, Cell::of(dirt));
                    if !grass.is_air() && world.get(x, y + 1).is_air() {
                        world.set(x, y + 1, Cell::of(grass));
                        stats.grass += 1;
                    }
                    stats.dirt += 1;
                }
            }
        }
    }

    if stamp.ores {
        if !coal.is_air() {
            scatter_blobs(
                world,
                stamp.x0 + 4,
                stamp.y0 + 4,
                stamp.x1 - 4,
                stamp.y1 - 4,
                coal,
                12,
                2,
                stamp.seed ^ 0xC0A1,
            );
            stats.ore_scatters += 12;
        }
        if !iron.is_air() {
            scatter_blobs(
                world,
                stamp.x0 + 4,
                stamp.y0 + 4,
                stamp.x1 - 4,
                stamp.y1 - 4,
                iron,
                8,
                2,
                stamp.seed ^ 0x49524F4E,
            );
            stats.ore_scatters += 8;
        }
        if !gold.is_air() {
            scatter_blobs(
                world,
                stamp.x0 + 4,
                stamp.y0 + 4,
                stamp.x1 - 4,
                stamp.y1 - 4,
                gold,
                4,
                1,
                stamp.seed ^ 0x474F4C44,
            );
            stats.ore_scatters += 4;
        }
    }

    if stamp.vegetation {
        let mut gcfg = GrowthConfig {
            vine_up: 0.4,
            moss_spread: 0.2,
            seed_sprout: 0.3,
            ..GrowthConfig::default()
        };
        gcfg.max_ops = 128;
        for _ in 0..5 {
            stats.growth_ops += growth_pass(world, stamp.x0, stamp.y0, stamp.x1, stamp.y1, &gcfg);
        }
    }

    stats.reactions = apply_reaction_chains(
        world,
        stamp.x0,
        stamp.y0,
        stamp.x1,
        stamp.y1.min(stamp.y0 + 16),
        32,
    );
    stats.occupied = world.occupied_cells();
    stats
}

/// Stats from biome stamp.
#[derive(Debug, Clone, Default)]
pub struct BiomeStats {
    /// Caves generated.
    pub caves: bool,
    /// Dirt cells painted.
    pub dirt: u32,
    /// Grass cells.
    pub grass: u32,
    /// Ore scatter calls.
    pub ore_scatters: u32,
    /// Growth ops.
    pub growth_ops: usize,
    /// Reaction ops.
    pub reactions: usize,
    /// Occupied after.
    pub occupied: usize,
}

/// Desert stamp: sand dunes + sparse stone.
pub fn stamp_desert(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, seed: u64) {
    let sand = world.mat("sand");
    let stone = world.mat("stone");
    let bed = world.mat("bedrock");
    world.paint_rect(x0, y0, x1, y0 + 2, bed);
    for x in x0..x1 {
        let h = 3 + ((x as u64).wrapping_mul(seed).wrapping_add(13) % 7) as i32;
        for y in (y0 + 2)..(y0 + 2 + h).min(y1) {
            world.set(x, y, Cell::of(sand));
        }
        if x % 11 == 0 && !stone.is_air() {
            world.set(x, y0 + 2, Cell::of(stone));
        }
    }
}

/// Forest stamp: dirt floor + trees (wood columns + leaf caps).
pub fn stamp_forest(world: &mut World, x0: i32, floor_y: i32, width: i32, trees: u32, seed: u64) {
    let dirt = world.mat("dirt");
    let grass = world.mat("grass");
    let wood = world.mat("wood");
    let leaf = world.mat("leaf");
    let bed = world.mat("bedrock");
    world.paint_rect(x0, floor_y - 2, x0 + width, floor_y, bed);
    if !dirt.is_air() {
        world.paint_rect(x0, floor_y, x0 + width, floor_y + 2, dirt);
    }
    if !grass.is_air() {
        world.paint_rect(x0, floor_y + 2, x0 + width, floor_y + 3, grass);
    }
    let mut rng = seed | 1;
    let next = |r: &mut u64| {
        let mut x = *r;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *r = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    };
    for _ in 0..trees {
        let tx = x0 + (next(&mut rng) % width.max(1) as u32) as i32;
        let th = 4 + (next(&mut rng) % 5) as i32;
        if !wood.is_air() {
            for y in 0..th {
                world.set(tx, floor_y + 3 + y, Cell::of(wood));
            }
        }
        if !leaf.is_air() {
            world.paint_circle(tx, floor_y + 3 + th, 3, leaf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::material_catalog::register_catalog_materials;
    use crate::world::WorldConfig;

    #[test]
    fn stamp_biome_and_desert() {
        let (mut reg, _) = builtin_registry();
        register_catalog_materials(&mut reg).unwrap();
        let mut world = World::new(reg, WorldConfig::default());
        let stats = stamp_biome(
            &mut world,
            &BiomeStamp {
                x0: -32,
                y0: 0,
                x1: 32,
                y1: 32,
                seed: 99,
                ..Default::default()
            },
        );
        assert!(stats.occupied > 50);
        stamp_desert(&mut world, 40, 0, 80, 20, 3);
        assert!(world.occupied_cells() > stats.occupied);
        stamp_forest(&mut world, -80, 0, 30, 5, 4);
        assert!(world.occupied_cells() > 0);
    }
}
