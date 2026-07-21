//! Hot-chunk efficient stepping — only simulate active regions.

use std::collections::HashSet;
use std::time::Instant;

use crate::chunk::{ChunkCoord, CHUNK_SIZE};
use crate::sim::SimConfig;
use crate::world::World;

/// Hot region tracker for efficient simulation.
#[derive(Debug, Clone, Default)]
pub struct HotChunkTracker {
    /// Chunks that must simulate this frame.
    hot: HashSet<ChunkCoord>,
    /// Frames since last activity per chunk (approx).
    cool: Vec<(ChunkCoord, u32)>,
    /// Max cool frames before sleep.
    pub sleep_after: u32,
}

impl HotChunkTracker {
    /// New.
    pub fn new() -> Self {
        Self {
            hot: HashSet::new(),
            cool: Vec::new(),
            sleep_after: 30,
        }
    }

    /// Mark world position hot.
    pub fn touch(&mut self, x: i32, y: i32) {
        self.hot.insert(ChunkCoord::from_cell(x, y));
        // neighbors
        for dy in -1..=1 {
            for dx in -1..=1 {
                self.hot.insert(ChunkCoord::from_cell(
                    x + dx * CHUNK_SIZE as i32 / 2,
                    y + dy * CHUNK_SIZE as i32 / 2,
                ));
            }
        }
    }

    /// Mark chunk hot.
    pub fn touch_chunk(&mut self, c: ChunkCoord) {
        self.hot.insert(c);
    }

    /// Collect hot list from world activity + tracker.
    pub fn collect(&mut self, world: &World) -> Vec<ChunkCoord> {
        for c in world.active_chunk_coords() {
            self.hot.insert(c);
        }
        let mut v: Vec<_> = self.hot.iter().copied().collect();
        v.sort_by_key(|c| (c.y, c.x));
        v
    }

    /// Clear hot set (after step).
    pub fn clear_hot(&mut self) {
        self.hot.clear();
    }
}

/// Step only hot chunks (efficient path).
pub fn step_hot(world: &mut World, cfg: &SimConfig, tracker: &mut HotChunkTracker) {
    world.events.clear();
    let coords = tracker.collect(world);
    if coords.is_empty() {
        world.tick = world.tick.wrapping_add(1);
        return;
    }
    // Tracker touch must be able to wake sleeping static terrain (dig, cast, etc.).
    for c in &coords {
        if let Some(ch) = world.chunk_mut(*c) {
            ch.sleeping = false;
        }
    }
    crate::sim::step_chunks(world, cfg, &coords);
    tracker.clear_hot();
    // re-touch chunks that remained active (moved this step)
    for c in world.active_chunk_coords() {
        tracker.touch_chunk(c);
    }
    world.tick = world.tick.wrapping_add(1);
}

/// Timed multi-step for perf tests. Returns elapsed milliseconds.
pub fn timed_steps(world: &mut World, cfg: &SimConfig, steps: u32, use_hot: bool) -> f64 {
    let mut tracker = HotChunkTracker::new();
    // seed hot from all loaded
    for c in world.loaded_chunks() {
        tracker.touch_chunk(c);
    }
    let t0 = Instant::now();
    for _ in 0..steps {
        if use_hot {
            step_hot(world, cfg, &mut tracker);
        } else {
            crate::sim::step(world, cfg);
        }
    }
    t0.elapsed().as_secs_f64() * 1000.0
}

/// Fill two chunks with mixed materials for perf scenes.
pub fn fill_perf_scene(
    world: &mut World,
    sand: crate::cell::MaterialId,
    water: crate::cell::MaterialId,
    stone: crate::cell::MaterialId,
) {
    // chunk (0,0) and (1,0) — 64*64*2 cells
    for y in 0..CHUNK_SIZE as i32 {
        for x in 0..(CHUNK_SIZE as i32 * 2) {
            let cell = if y < 3 {
                stone
            } else if (x + y) % 7 == 0 {
                water
            } else if (x * 3 + y) % 5 == 0 {
                sand
            } else if y > 40 && (x % 11 == 0) {
                sand
            } else {
                continue;
            };
            world.set(x, y, crate::cell::Cell::of(cell));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::WorldConfig;

    #[test]
    fn perf_two_chunks_under_budget() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(
            reg,
            WorldConfig {
                max_loaded_chunks: 64,
                ..WorldConfig::default()
            },
        );
        fill_perf_scene(&mut world, ids.sand, ids.water, ids.stone);
        let occupied = world.occupied_cells();
        assert!(
            occupied >= CHUNK_SIZE * 2,
            "need substantial cells, got {occupied}"
        );
        let cfg = SimConfig {
            parallel: false,
            ..SimConfig::default()
        };
        // 30 steps on 2 chunks — budget generous for debug builds
        let ms = timed_steps(&mut world, &cfg, 30, true);
        // 15 seconds hard budget so cold debug CI still passes
        assert!(
            ms < 15_000.0,
            "perf budget exceeded: {ms:.1}ms for 30 steps, occupied={occupied}"
        );
        eprintln!("PERF_OK hot_steps=30 occupied={occupied} ms={ms:.2}");
    }

    #[test]
    fn hot_step_moves_sand() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(-5, 0, 5, 1, ids.bedrock);
        world.set(0, 15, crate::cell::Cell::of(ids.sand));
        let mut tracker = HotChunkTracker::new();
        tracker.touch(0, 15);
        let cfg = SimConfig::default();
        for _ in 0..40 {
            step_hot(&mut world, &cfg, &mut tracker);
            tracker.touch(0, 5);
        }
        let mut found = false;
        for y in 0..16 {
            if world.get(0, y).material == ids.sand {
                found = true;
                assert!(y < 15, "sand should fall");
            }
        }
        assert!(found);
    }
}
