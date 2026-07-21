//! Hot-chunk efficient stepping — only simulate active regions.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::chunk::{ChunkCoord, CHUNK_SIZE};
use crate::sim::SimConfig;
use crate::world::World;

/// Hot region tracker for efficient simulation.
#[derive(Debug, Clone, Default)]
pub struct HotChunkTracker {
    /// Chunks that must simulate this frame.
    hot: HashSet<ChunkCoord>,
    /// Consecutive inactive frames per chunk before it is allowed to sleep.
    cool: HashMap<ChunkCoord, u32>,
    /// Max cool frames before sleep.
    pub sleep_after: u32,
}

impl HotChunkTracker {
    /// New tracker with a short grace period before inactive chunks sleep.
    pub fn new() -> Self {
        Self {
            hot: HashSet::new(),
            cool: HashMap::new(),
            sleep_after: 30,
        }
    }

    /// Mark world position hot, including its neighboring chunk region.
    pub fn touch(&mut self, x: i32, y: i32) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let coord = ChunkCoord::from_cell(
                    x + dx * CHUNK_SIZE as i32 / 2,
                    y + dy * CHUNK_SIZE as i32 / 2,
                );
                self.hot.insert(coord);
                self.cool.remove(&coord);
            }
        }
    }

    /// Mark chunk hot and reset its cooling age.
    pub fn touch_chunk(&mut self, coord: ChunkCoord) {
        self.hot.insert(coord);
        self.cool.remove(&coord);
    }

    /// Collect hot chunks from explicit touches and currently active world chunks.
    pub fn collect(&mut self, world: &World) -> Vec<ChunkCoord> {
        for coord in world.active_chunk_coords() {
            self.hot.insert(coord);
            self.cool.remove(&coord);
        }
        let mut coords: Vec<_> = self.hot.iter().copied().collect();
        coords.sort_by_key(|coord| (coord.y, coord.x));
        coords
    }

    /// Clear explicitly tracked hot chunks.
    pub fn clear_hot(&mut self) {
        self.hot.clear();
    }

    fn finish_step(&mut self, world: &mut World, stepped: &[ChunkCoord]) {
        self.hot.clear();
        let sleep_after = self.sleep_after.max(1);

        for &coord in stepped {
            let active = world
                .chunk(coord)
                .is_some_and(|chunk| chunk.active && !chunk.sleeping);
            if active {
                self.cool.remove(&coord);
                self.hot.insert(coord);
                continue;
            }

            let age = self.cool.entry(coord).or_insert(0);
            *age = age.saturating_add(1);
            if *age < sleep_after {
                // `step_chunks` puts inactive chunks to sleep immediately. The tracker
                // re-opens them during the grace period so slow reactions can settle.
                if let Some(chunk) = world.chunk_mut(coord) {
                    chunk.sleeping = false;
                }
                self.hot.insert(coord);
            } else {
                self.cool.remove(&coord);
            }
        }

        // Cross-chunk movement may activate neighbors not present in `stepped`.
        for coord in world.active_chunk_coords() {
            self.cool.remove(&coord);
            self.hot.insert(coord);
        }
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

    // Explicit touches must be able to wake sleeping static terrain before a dig,
    // spell, reaction, or neighboring material update is processed.
    for coord in &coords {
        if let Some(chunk) = world.chunk_mut(*coord) {
            chunk.sleeping = false;
        }
    }

    crate::sim::step_chunks(world, cfg, &coords);
    tracker.finish_step(world, &coords);
    world.tick = world.tick.wrapping_add(1);
}

/// Timed multi-step for perf tests. Returns elapsed milliseconds.
pub fn timed_steps(world: &mut World, cfg: &SimConfig, steps: u32, use_hot: bool) -> f64 {
    let mut tracker = HotChunkTracker::new();
    for coord in world.loaded_chunks() {
        tracker.touch_chunk(coord);
    }
    let started = Instant::now();
    for _ in 0..steps {
        if use_hot {
            step_hot(world, cfg, &mut tracker);
        } else {
            crate::sim::step(world, cfg);
        }
    }
    started.elapsed().as_secs_f64() * 1000.0
}

/// Fill two chunks with mixed materials for perf scenes.
pub fn fill_perf_scene(
    world: &mut World,
    sand: crate::cell::MaterialId,
    water: crate::cell::MaterialId,
    stone: crate::cell::MaterialId,
) {
    for y in 0..CHUNK_SIZE as i32 {
        for x in 0..(CHUNK_SIZE as i32 * 2) {
            let cell = if y < 3 {
                stone
            } else if (x + y) % 7 == 0 {
                water
            } else if (x * 3 + y) % 5 == 0 || (y > 40 && x % 11 == 0) {
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
        let (registry, ids) = builtin_registry();
        let mut world = World::new(
            registry,
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
        let config = SimConfig {
            parallel: false,
            ..SimConfig::default()
        };
        let elapsed_ms = timed_steps(&mut world, &config, 30, true);
        assert!(
            elapsed_ms < 15_000.0,
            "perf budget exceeded: {elapsed_ms:.1}ms for 30 steps, occupied={occupied}"
        );
        eprintln!("PERF_OK hot_steps=30 occupied={occupied} ms={elapsed_ms:.2}");
    }

    #[test]
    fn hot_step_moves_sand() {
        let (registry, ids) = builtin_registry();
        let mut world = World::new(registry, WorldConfig::default());
        world.paint_rect(-5, 0, 5, 1, ids.bedrock);
        world.set(0, 15, crate::cell::Cell::of(ids.sand));
        let mut tracker = HotChunkTracker::new();
        tracker.touch(0, 15);
        let config = SimConfig::default();
        for _ in 0..40 {
            step_hot(&mut world, &config, &mut tracker);
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

    #[test]
    fn inactive_chunk_uses_cooldown_before_sleeping() {
        let (registry, ids) = builtin_registry();
        let mut world = World::new(registry, WorldConfig::default());
        world.set(0, 0, crate::cell::Cell::of(ids.bedrock));
        let coord = ChunkCoord::new(0, 0);
        let mut tracker = HotChunkTracker::new();
        tracker.sleep_after = 2;
        tracker.touch_chunk(coord);
        let config = SimConfig {
            gravity: false,
            density: false,
            temperature: false,
            fire: false,
            dissolve: false,
            pressure: false,
            alternate_scan: false,
            parallel: false,
        };

        step_hot(&mut world, &config, &mut tracker);
        assert!(!world.chunk(coord).unwrap().sleeping);
        assert!(tracker.hot.contains(&coord));

        step_hot(&mut world, &config, &mut tracker);
        assert!(world.chunk(coord).unwrap().sleeping);
        assert!(!tracker.hot.contains(&coord));
        assert!(!tracker.cool.contains_key(&coord));
    }
}
