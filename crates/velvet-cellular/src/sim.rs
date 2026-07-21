//! Simulation orchestrator — multi-pass cellular update.

use crate::chunk::{ChunkCoord, CHUNK_SIZE};
use crate::rules::{
    clear_moved_flags, rule_blood, rule_density_sink, rule_dissolve, rule_explosion, rule_fire,
    rule_gravity, rule_pressure_diffuse, rule_temperature, RuleCtx,
};
use crate::world::World;

/// Which rule passes to run.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Gravity / flow.
    pub gravity: bool,
    /// Density sorting.
    pub density: bool,
    /// Temperature.
    pub temperature: bool,
    /// Fire.
    pub fire: bool,
    /// Dissolve.
    pub dissolve: bool,
    /// Pressure.
    pub pressure: bool,
    /// Scan direction flip each tick (reduces bias).
    pub alternate_scan: bool,
    /// Parallel chunk stepping (feature-dependent).
    pub parallel: bool,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            gravity: true,
            density: true,
            temperature: true,
            fire: true,
            dissolve: true,
            pressure: true,
            alternate_scan: true,
            parallel: cfg!(feature = "parallel"),
        }
    }
}

/// Run one full simulation step.
pub fn step(world: &mut World, cfg: &SimConfig) {
    world.events.clear();
    let coords = world.active_chunk_coords();
    if coords.is_empty() {
        // still step any loaded
        let all: Vec<_> = world.loaded_chunks().collect();
        if all.is_empty() {
            world.tick = world.tick.wrapping_add(1);
            return;
        }
        step_chunks(world, cfg, &all);
    } else {
        step_chunks(world, cfg, &coords);
    }
    world.tick = world.tick.wrapping_add(1);
}

/// Step a specific set of chunks (hot-path / efficient). Does **not** bump `world.tick`.
pub fn step_chunks(world: &mut World, cfg: &SimConfig, coords: &[ChunkCoord]) {
    clear_moved_flags(world, coords);

    // Bottom-to-top for gravity stability: process lower rows first within each chunk pass
    let reverse_x = cfg.alternate_scan && (world.tick % 2 == 1);
    let reverse_y = false;

    // Order chunks by Y ascending (bottom first)
    let mut ordered = coords.to_vec();
    ordered.sort_by_key(|c| (c.y, c.x));

    // Parallel path: process chunks in Y-bands with disjoint X ranges when safe.
    // Default stays sequential for determinism; enable via SimConfig.parallel + feature.
    if cfg.parallel && ordered.len() > 3 {
        #[cfg(feature = "parallel")]
        {
            parallel_step_chunks(world, cfg, &ordered, reverse_x, reverse_y);
        }
        #[cfg(not(feature = "parallel"))]
        {
            for cc in &ordered {
                step_chunk(world, cfg, *cc, reverse_x, reverse_y);
            }
        }
    } else {
        for cc in &ordered {
            step_chunk(world, cfg, *cc, reverse_x, reverse_y);
        }
    }
    // end of step_chunks core

    // Sleep chunks that did not move anything this step.
    // (Previously we kept every solid chunk active forever → full-map sim lag.)
    for cc in coords {
        if let Some(ch) = world.chunk_mut(*cc) {
            if !ch.active {
                ch.sleeping = true;
            }
        }
    }
}

fn step_chunk(
    world: &mut World,
    cfg: &SimConfig,
    cc: ChunkCoord,
    reverse_x: bool,
    reverse_y: bool,
) {
    let (ox, oy) = cc.origin_cell();
    let xs: Vec<usize> = if reverse_x {
        (0..CHUNK_SIZE).rev().collect()
    } else {
        (0..CHUNK_SIZE).collect()
    };
    let ys: Vec<usize> = if reverse_y {
        (0..CHUNK_SIZE).rev().collect()
    } else {
        // bottom-up for gravity
        (0..CHUNK_SIZE).collect()
    };

    for &ly in &ys {
        for &lx in &xs {
            let x = ox + lx as i32;
            let y = oy + ly as i32;
            // skip air quickly
            if world.get(x, y).is_air() {
                continue;
            }
            let mut ctx = RuleCtx { world, x, y };
            if cfg.temperature {
                rule_temperature(&mut ctx);
            }
            if cfg.fire {
                rule_fire(&mut ctx);
            }
            if cfg.fire {
                rule_explosion(&mut ctx);
            }
            if cfg.dissolve {
                rule_dissolve(&mut ctx);
            }
            rule_blood(&mut ctx);
            if cfg.pressure {
                rule_pressure_diffuse(&mut ctx);
            }
            if cfg.density {
                rule_density_sink(&mut ctx);
            }
            if cfg.gravity {
                rule_gravity(&mut ctx);
            }
        }
    }
}

/// Step N times.
pub fn step_n(world: &mut World, cfg: &SimConfig, n: u32) {
    for _ in 0..n {
        step(world, cfg);
    }
}

/// Checkerboard waves (even/odd chunk X) — reduces neighbor bias; ready for
/// future stripe-parallel ownership without changing the author API.
#[cfg(feature = "parallel")]
fn parallel_step_chunks(
    world: &mut World,
    cfg: &SimConfig,
    ordered: &[ChunkCoord],
    reverse_x: bool,
    reverse_y: bool,
) {
    for parity in [0i32, 1] {
        for cc in ordered
            .iter()
            .copied()
            .filter(|c| c.x.rem_euclid(2) == parity)
        {
            step_chunk(world, cfg, cc, reverse_x, reverse_y);
        }
    }
}
