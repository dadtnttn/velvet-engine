//! Procedural generation helpers for creators (caves, platforms, arenas).

use crate::cell::{Cell, MaterialId};
use crate::world::World;

/// Seeded value noise (no external crate).
fn hash2(x: i32, y: i32, seed: u64) -> f32 {
    let mut n = (x as u64)
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add((y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F))
        .wrapping_add(seed);
    n = (n ^ (n >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94D049BB133111EB);
    n = n ^ (n >> 31);
    (n as f32) / (u64::MAX as f32)
}

/// Smooth value noise.
fn noise2(x: f32, y: f32, seed: u64) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let n00 = hash2(x0, y0, seed);
    let n10 = hash2(x0 + 1, y0, seed);
    let n01 = hash2(x0, y0 + 1, seed);
    let n11 = hash2(x0 + 1, y0 + 1, seed);
    let nx0 = n00 + (n10 - n00) * sx;
    let nx1 = n01 + (n11 - n01) * sx;
    nx0 + (nx1 - nx0) * sy
}

/// FBM noise.
fn fbm(x: f32, y: f32, seed: u64, octaves: u32) -> f32 {
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut sum = 0.0f32;
    let mut norm = 0.0f32;
    for i in 0..octaves {
        sum += noise2(x * freq, y * freq, seed.wrapping_add(i as u64 * 17)) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm.max(1e-5)
}

/// Cave generation options.
#[derive(Debug, Clone)]
pub struct CaveOptions {
    /// World rect min X.
    pub x0: i32,
    /// Min Y.
    pub y0: i32,
    /// Max X exclusive.
    pub x1: i32,
    /// Max Y exclusive.
    pub y1: i32,
    /// Fill solid material.
    pub solid: MaterialId,
    /// Floor / border material.
    pub border: MaterialId,
    /// Threshold 0..=1 (higher = more open).
    pub open_threshold: f32,
    /// Noise scale.
    pub scale: f32,
    /// Seed.
    pub seed: u64,
    /// Border thickness.
    pub border_thickness: i32,
}

impl Default for CaveOptions {
    fn default() -> Self {
        Self {
            x0: -64,
            y0: -16,
            x1: 64,
            y1: 48,
            solid: MaterialId::AIR,
            border: MaterialId::AIR,
            open_threshold: 0.48,
            scale: 0.08,
            seed: 1,
            border_thickness: 2,
        }
    }
}

/// Generate a cave field: solid stone with open air pockets.
pub fn generate_caves(world: &mut World, opt: &CaveOptions) {
    let solid = opt.solid;
    let border = opt.border;
    let t = opt.border_thickness;
    for y in opt.y0..opt.y1 {
        for x in opt.x0..opt.x1 {
            let edge = x < opt.x0 + t
                || x >= opt.x1 - t
                || y < opt.y0 + t
                || y >= opt.y1 - t;
            if edge {
                world.set(x, y, Cell::of(border));
                continue;
            }
            let n = fbm(x as f32 * opt.scale, y as f32 * opt.scale, opt.seed, 4);
            if n > opt.open_threshold {
                world.set(x, y, Cell::air());
            } else {
                world.set(x, y, Cell::of(solid));
            }
        }
    }
}

/// Scatter material blobs (ores, blood pools, water pockets).
pub fn scatter_blobs(
    world: &mut World,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    material: MaterialId,
    count: u32,
    radius: i32,
    seed: u64,
) {
    let mut rng = seed | 1;
    let next = |r: &mut u64| -> u32 {
        let mut x = *r;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *r = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    };
    let w = (x1 - x0).max(1) as u32;
    let h = (y1 - y0).max(1) as u32;
    for _ in 0..count {
        let x = x0 + (next(&mut rng) % w) as i32;
        let y = y0 + (next(&mut rng) % h) as i32;
        let r = 1 + (next(&mut rng) as i32 % radius.max(1));
        world.paint_circle(x, y, r, material);
    }
}

/// Flat arena: floor + side walls + optional ceiling.
pub fn generate_arena(
    world: &mut World,
    cx: i32,
    floor_y: i32,
    half_w: i32,
    height: i32,
    wall: MaterialId,
    floor: MaterialId,
    ceiling: bool,
) {
    let x0 = cx - half_w;
    let x1 = cx + half_w;
    world.paint_rect(x0, floor_y - 2, x1, floor_y, floor);
    world.paint_rect(x0, floor_y, x0 + 2, floor_y + height, wall);
    world.paint_rect(x1 - 2, floor_y, x1, floor_y + height, wall);
    if ceiling {
        world.paint_rect(x0, floor_y + height - 2, x1, floor_y + height, wall);
    }
}

/// Platform steps for vertical progression.
pub fn generate_platforms(
    world: &mut World,
    x0: i32,
    x1: i32,
    y0: i32,
    count: u32,
    spacing: i32,
    material: MaterialId,
    seed: u64,
) {
    let mut rng = seed | 1;
    let next = |r: &mut u64| -> u32 {
        let mut x = *r;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        *r = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 32) as u32
    };
    let w = (x1 - x0).max(8);
    for i in 0..count {
        let y = y0 + i as i32 * spacing;
        let len = 6 + (next(&mut rng) % 10) as i32;
        let px = x0 + (next(&mut rng) % (w as u32).saturating_sub(len as u32).max(1)) as i32;
        world.paint_rect(px, y, px + len, y + 2, material);
    }
}

/// Cellular automata smooth pass (open/close caves).
pub fn cave_smooth(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, solid: MaterialId, iterations: u32) {
    for _ in 0..iterations {
        let mut changes: Vec<(i32, i32, bool)> = Vec::new();
        for y in (y0 + 1)..(y1 - 1) {
            for x in (x0 + 1)..(x1 - 1) {
                let mut walls = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        if !world.get(x + dx, y + dy).is_air() {
                            walls += 1;
                        }
                    }
                }
                let is_solid = !world.get(x, y).is_air();
                if walls >= 5 && !is_solid {
                    changes.push((x, y, true));
                } else if walls <= 2 && is_solid {
                    changes.push((x, y, false));
                }
            }
        }
        for (x, y, to_solid) in changes {
            if to_solid {
                world.set(x, y, Cell::of(solid));
            } else {
                world.set(x, y, Cell::air());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::WorldConfig;

    #[test]
    fn caves_generate_mix_of_air_and_solid() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        generate_caves(
            &mut world,
            &CaveOptions {
                x0: -32,
                y0: 0,
                x1: 32,
                y1: 32,
                solid: ids.stone,
                border: ids.bedrock,
                open_threshold: 0.5,
                scale: 0.12,
                seed: 42,
                border_thickness: 1,
            },
        );
        let mut air = 0;
        let mut solid = 0;
        for y in 1..31 {
            for x in -31..31 {
                if world.get(x, y).is_air() {
                    air += 1;
                } else {
                    solid += 1;
                }
            }
        }
        assert!(air > 50 && solid > 50, "air={air} solid={solid}");
    }
}
