//! Sparse chunked world — streaming, cell access, paint surface.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::cell::{Cell, MaterialId};
use crate::chunk::{Chunk, ChunkCoord, CHUNK_SIZE};
use crate::events::{EventQueue, SimEvent};
use crate::material::MaterialRegistry;

/// World configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    /// Max loaded chunks (LRU unload).
    pub max_loaded_chunks: usize,
    /// Gravity acceleration in cells/step² (applied as discrete bias).
    pub gravity: f32,
    /// Ambient temperature °C.
    pub ambient_temp: f32,
    /// Seed for deterministic jitter.
    pub seed: u64,
    /// Event queue capacity.
    pub event_capacity: usize,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            max_loaded_chunks: 256,
            gravity: 1.0,
            ambient_temp: 20.0,
            seed: 0xC0FFEE,
            event_capacity: 4096,
        }
    }
}

/// Sparse cellular world.
#[derive(Debug, Clone)]
pub struct World {
    /// Config.
    pub config: WorldConfig,
    /// Materials.
    pub materials: MaterialRegistry,
    /// Loaded chunks.
    chunks: HashMap<ChunkCoord, Chunk>,
    /// LRU order (front = oldest).
    lru: VecDeque<ChunkCoord>,
    /// Simulation tick.
    pub tick: u64,
    /// RNG state.
    pub rng: u64,
    /// Events this step.
    pub events: EventQueue,
    /// Keep-alive chunk set (camera / player).
    keep: HashSet<ChunkCoord>,
}

impl World {
    /// Create empty world with registry (air only unless caller fills).
    pub fn new(materials: MaterialRegistry, config: WorldConfig) -> Self {
        let events = EventQueue::with_capacity(config.event_capacity);
        let rng = config.seed | 1;
        Self {
            config,
            materials,
            chunks: HashMap::new(),
            lru: VecDeque::new(),
            tick: 0,
            rng,
            events,
            keep: HashSet::new(),
        }
    }

    /// Next random u32.
    pub fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D)) >> 32) as u32
    }

    /// Random bool with probability p (0..=1).
    pub fn chance(&mut self, p: f32) -> bool {
        if p <= 0.0 {
            return false;
        }
        if p >= 1.0 {
            return true;
        }
        (self.next_u32() as f32 / u32::MAX as f32) < p
    }

    /// Mark chunk keep-alive (camera).
    pub fn keep_chunk(&mut self, c: ChunkCoord) {
        self.keep.insert(c);
    }

    /// Clear keep set.
    pub fn clear_keep(&mut self) {
        self.keep.clear();
    }

    /// Ensure chunk loaded (creates empty).
    pub fn ensure_chunk(&mut self, coord: ChunkCoord) -> &mut Chunk {
        if !self.chunks.contains_key(&coord) {
            self.chunks.insert(coord, Chunk::empty(coord));
            self.lru.push_back(coord);
            self.events.push(SimEvent::ChunkLoaded {
                cx: coord.x,
                cy: coord.y,
            });
            self.evict_if_needed();
        } else {
            // touch LRU
            if let Some(i) = self.lru.iter().position(|c| *c == coord) {
                self.lru.remove(i);
                self.lru.push_back(coord);
            }
        }
        self.chunks.get_mut(&coord).unwrap()
    }

    fn evict_if_needed(&mut self) {
        while self.chunks.len() > self.config.max_loaded_chunks {
            let Some(old) = self.lru.pop_front() else {
                break;
            };
            if self.keep.contains(&old) {
                self.lru.push_back(old);
                // avoid infinite loop
                if self.lru.iter().all(|c| self.keep.contains(c)) {
                    break;
                }
                continue;
            }
            self.chunks.remove(&old);
            self.events.push(SimEvent::ChunkUnloaded {
                cx: old.x,
                cy: old.y,
            });
        }
    }

    /// Loaded chunk coords.
    pub fn loaded_chunks(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.chunks.keys().copied()
    }

    /// Chunk ref if loaded.
    pub fn chunk(&self, c: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&c)
    }

    /// Chunk mut if loaded.
    pub fn chunk_mut(&mut self, c: ChunkCoord) -> Option<&mut Chunk> {
        self.chunks.get_mut(&c)
    }

    /// Get cell at world coords (air if unloaded — does not allocate).
    pub fn get(&self, x: i32, y: i32) -> Cell {
        let cc = ChunkCoord::from_cell(x, y);
        let Some(chunk) = self.chunks.get(&cc) else {
            return Cell::air();
        };
        let (ox, oy) = cc.origin_cell();
        let lx = (x - ox) as usize;
        let ly = (y - oy) as usize;
        chunk.get(lx, ly)
    }

    /// Set cell (loads chunk).
    pub fn set(&mut self, x: i32, y: i32, cell: Cell) {
        let cc = ChunkCoord::from_cell(x, y);
        let (ox, oy) = cc.origin_cell();
        let lx = (x - ox) as usize;
        let ly = (y - oy) as usize;
        let prev = self.get(x, y);
        let chunk = self.ensure_chunk(cc);
        chunk.set(lx, ly, cell);
        if prev.material != cell.material {
            self.events.push(SimEvent::MaterialChanged {
                x,
                y,
                from: prev.material,
                to: cell.material,
            });
        }
        // wake neighbors
        self.wake_neighbors(x, y);
    }

    /// Swap two cells if both chunks loadable.
    pub fn swap(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let a = self.get(x0, y0);
        let b = self.get(x1, y1);
        // direct set without double events noise
        let cc0 = ChunkCoord::from_cell(x0, y0);
        let cc1 = ChunkCoord::from_cell(x1, y1);
        {
            let (ox, oy) = cc0.origin_cell();
            let chunk = self.ensure_chunk(cc0);
            chunk.set((x0 - ox) as usize, (y0 - oy) as usize, b);
        }
        {
            let (ox, oy) = cc1.origin_cell();
            let chunk = self.ensure_chunk(cc1);
            chunk.set((x1 - ox) as usize, (y1 - oy) as usize, a);
        }
        self.wake_neighbors(x0, y0);
        self.wake_neighbors(x1, y1);
    }

    /// Wake chunk + 8-neighbors for sleep system.
    pub fn wake_neighbors(&mut self, x: i32, y: i32) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cc = ChunkCoord::from_cell(x + dx, y + dy);
                if let Some(ch) = self.chunks.get_mut(&cc) {
                    ch.sleeping = false;
                    ch.active = true;
                }
            }
        }
    }

    /// Material id by key.
    pub fn mat(&self, key: &str) -> MaterialId {
        self.materials
            .id(key)
            .unwrap_or(MaterialId::AIR)
    }

    /// Paint filled rectangle [x0,x1) × [y0,y1).
    pub fn paint_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, material: MaterialId) {
        let (x0, x1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
        let (y0, y1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in y0..y1 {
            for x in x0..x1 {
                self.set(x, y, Cell::of(material));
            }
        }
    }

    /// Paint disk.
    pub fn paint_circle(&mut self, cx: i32, cy: i32, radius: i32, material: MaterialId) {
        let r2 = radius * radius;
        for y in (cy - radius)..=(cy + radius) {
            for x in (cx - radius)..=(cx + radius) {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= r2 {
                    self.set(x, y, Cell::of(material));
                }
            }
        }
    }

    /// Clear circle to air.
    pub fn erase_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        self.paint_circle(cx, cy, radius, MaterialId::AIR);
    }

    /// Flood fill (bounded).
    pub fn flood_fill(&mut self, x: i32, y: i32, material: MaterialId, max_cells: usize) -> usize {
        let target = self.get(x, y).material;
        if target == material {
            return 0;
        }
        let mut stack = vec![(x, y)];
        let mut n = 0usize;
        let mut seen = HashSet::new();
        while let Some((cx, cy)) = stack.pop() {
            if n >= max_cells {
                break;
            }
            if !seen.insert((cx, cy)) {
                continue;
            }
            if self.get(cx, cy).material != target {
                continue;
            }
            self.set(cx, cy, Cell::of(material));
            n += 1;
            stack.push((cx + 1, cy));
            stack.push((cx - 1, cy));
            stack.push((cx, cy + 1));
            stack.push((cx, cy - 1));
        }
        n
    }

    /// Raycast through grid; returns first non-air hit.
    pub fn raycast(
        &self,
        x0: f32,
        y0: f32,
        dx: f32,
        dy: f32,
        max_dist: f32,
    ) -> Option<(i32, i32, Cell)> {
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return None;
        }
        let (dx, dy) = (dx / len, dy / len);
        let steps = max_dist.ceil() as i32;
        let mut x = x0;
        let mut y = y0;
        for _ in 0..=steps {
            let ix = x.floor() as i32;
            let iy = y.floor() as i32;
            let c = self.get(ix, iy);
            if !c.is_air() {
                return Some((ix, iy, c));
            }
            x += dx;
            y += dy;
        }
        None
    }

    /// Count non-air in loaded world.
    pub fn occupied_cells(&self) -> usize {
        self.chunks.values().map(|c| c.solid_count()).sum()
    }

    /// Snapshot of chunk map for save.
    pub(crate) fn chunks_map(&self) -> &HashMap<ChunkCoord, Chunk> {
        &self.chunks
    }

    pub(crate) fn chunks_map_mut(&mut self) -> &mut HashMap<ChunkCoord, Chunk> {
        &mut self.chunks
    }

    /// Replace chunk storage (load).
    pub(crate) fn restore_chunks(&mut self, chunks: HashMap<ChunkCoord, Chunk>) {
        self.lru = chunks.keys().copied().collect();
        self.chunks = chunks;
    }

    /// Iterate active (non-sleeping) chunk coords.
    pub fn active_chunk_coords(&self) -> Vec<ChunkCoord> {
        self.chunks
            .iter()
            .filter(|(_, ch)| ch.active && !ch.sleeping)
            .map(|(c, _)| *c)
            .collect()
    }

    /// Put every loaded chunk to sleep (after large procgen). Simulation
    /// resumes only where something is touched / wakes.
    pub fn sleep_all_chunks(&mut self) {
        for ch in self.chunks.values_mut() {
            ch.active = false;
            ch.sleeping = true;
        }
    }

    /// World cell bounds of loaded area (min inclusive, max exclusive).
    pub fn loaded_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        if self.chunks.is_empty() {
            return None;
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for c in self.chunks.keys() {
            let (ox, oy) = c.origin_cell();
            min_x = min_x.min(ox);
            min_y = min_y.min(oy);
            max_x = max_x.max(ox + CHUNK_SIZE as i32);
            max_y = max_y.max(oy + CHUNK_SIZE as i32);
        }
        Some((min_x, min_y, max_x, max_y))
    }
}
