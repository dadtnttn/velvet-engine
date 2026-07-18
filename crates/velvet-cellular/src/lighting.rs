//! Flood-fill light levels for author tools / atmosphere.

use crate::world::World;

/// Light map over a rectangle.
#[derive(Debug, Clone)]
pub struct LightMap {
    /// Origin X.
    pub x0: i32,
    /// Origin Y.
    pub y0: i32,
    /// Width.
    pub w: i32,
    /// Height.
    pub h: i32,
    /// Light 0..=255 row-major.
    pub levels: Vec<u8>,
}

impl LightMap {
    /// Get light at world pos.
    pub fn get(&self, x: i32, y: i32) -> u8 {
        let lx = x - self.x0;
        let ly = y - self.y0;
        if lx < 0 || ly < 0 || lx >= self.w || ly >= self.h {
            return 0;
        }
        self.levels[(ly * self.w + lx) as usize]
    }

    /// Set.
    fn set(&mut self, x: i32, y: i32, v: u8) {
        let lx = x - self.x0;
        let ly = y - self.y0;
        if lx < 0 || ly < 0 || lx >= self.w || ly >= self.h {
            return;
        }
        let i = (ly * self.w + lx) as usize;
        if v > self.levels[i] {
            self.levels[i] = v;
        }
    }
}

/// Opacity of a cell (0 transparent, 255 solid).
pub fn cell_opacity(world: &World, x: i32, y: i32) -> u8 {
    let c = world.get(x, y);
    if c.is_air() {
        return 0;
    }
    match world.materials.phase(c.material) {
        crate::material::Phase::Gas | crate::material::Phase::Plasma => 20,
        crate::material::Phase::Liquid => 80,
        crate::material::Phase::Powder => 160,
        crate::material::Phase::Solid | crate::material::Phase::Static => 230,
    }
}

/// Emit light from emissive materials (fire, lava, plasma).
pub fn is_emissive(world: &World, x: i32, y: i32) -> u8 {
    let c = world.get(x, y);
    if c.is_air() {
        return 0;
    }
    let key = world.materials.get(c.material).key.as_str();
    match key {
        "fire" | "plasma_arc" => 220,
        "lava" => 180,
        "ember" => 140,
        _ if c.temp > 400.0 => 100,
        _ => 0,
    }
}

/// Bake light map with multi-source flood (decay by opacity).
pub fn bake_light(world: &World, x0: i32, y0: i32, w: i32, h: i32, sky: u8) -> LightMap {
    let mut map = LightMap {
        x0,
        y0,
        w,
        h,
        levels: vec![0; (w * h) as usize],
    };
    // sky light from top
    for x in x0..(x0 + w) {
        let mut light = sky;
        for y in (y0..(y0 + h)).rev() {
            map.set(x, y, light);
            let op = cell_opacity(world, x, y);
            light = light.saturating_sub(op / 8 + 1);
        }
    }
    // emissive BFS-ish multi pass
    for _ in 0..4 {
        for y in y0..(y0 + h) {
            for x in x0..(x0 + w) {
                let e = is_emissive(world, x, y);
                if e > 0 {
                    map.set(x, y, e);
                }
                let cur = map.get(x, y);
                if cur < 2 {
                    continue;
                }
                let next = cur.saturating_sub(12);
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    let op = cell_opacity(world, nx, ny);
                    let v = next.saturating_sub(op / 10);
                    map.set(nx, ny, v);
                }
            }
        }
    }
    map
}

/// Average light in region.
pub fn average_light(map: &LightMap) -> f32 {
    if map.levels.is_empty() {
        return 0.0;
    }
    let s: u32 = map.levels.iter().map(|&v| v as u32).sum();
    s as f32 / map.levels.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::cell::Cell;
    use crate::world::WorldConfig;

    #[test]
    fn fire_emits_light() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.set(5, 5, Cell::of(ids.fire).with_temp(900.0));
        let map = bake_light(&world, 0, 0, 16, 16, 40);
        assert!(map.get(5, 5) > 50);
        assert!(average_light(&map) > 0.0);
    }
}
