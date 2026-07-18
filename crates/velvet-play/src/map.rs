//! Tilemap grid and layers.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use velvet_math::{Rect, Vec2};

use crate::collider::{Collider, CollisionLayer};

/// Tile flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TileFlags {
    /// Blocks movement.
    pub solid: bool,
    /// Trigger id if any (0 = none).
    pub trigger_id: u16,
    /// Custom type id.
    pub kind: u16,
}

/// One tile cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Tile {
    /// Tileset index (0 = empty).
    pub id: u16,
    /// Flags.
    pub flags: TileFlags,
}

impl Tile {
    /// Empty.
    pub const EMPTY: Self = Self {
        id: 0,
        flags: TileFlags {
            solid: false,
            trigger_id: 0,
            kind: 0,
        },
    };

    /// Solid tile.
    pub fn solid(id: u16) -> Self {
        Self {
            id,
            flags: TileFlags {
                solid: true,
                trigger_id: 0,
                kind: 0,
            },
        }
    }

    /// Empty check.
    pub fn is_empty(self) -> bool {
        self.id == 0 && !self.flags.solid
    }
}

/// One layer of tiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileLayer {
    /// Name.
    pub name: String,
    /// Width in tiles.
    pub width: u32,
    /// Height in tiles.
    pub height: u32,
    /// Row-major tiles.
    pub tiles: Vec<Tile>,
    /// Collision enabled for this layer.
    pub collision: bool,
}

impl TileLayer {
    /// Create filled with empty.
    pub fn empty(name: impl Into<String>, width: u32, height: u32) -> Self {
        let n = (width as usize).saturating_mul(height as usize);
        Self {
            name: name.into(),
            width,
            height,
            tiles: vec![Tile::EMPTY; n],
            collision: true,
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height {
            return None;
        }
        Some((y as u32 * self.width + x as u32) as usize)
    }

    /// Get tile.
    pub fn get(&self, x: i32, y: i32) -> Tile {
        self.index(x, y)
            .and_then(|i| self.tiles.get(i).copied())
            .unwrap_or(Tile::EMPTY)
    }

    /// Set tile.
    pub fn set(&mut self, x: i32, y: i32, tile: Tile) -> bool {
        if let Some(i) = self.index(x, y) {
            self.tiles[i] = tile;
            true
        } else {
            false
        }
    }

    /// Fill rectangle with tile.
    pub fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, tile: Tile) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, tile);
            }
        }
    }
}

/// Map errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TileMapError {
    /// Invalid size.
    #[error("invalid map size")]
    InvalidSize,
    /// Layer missing.
    #[error("layer not found: {0}")]
    LayerNotFound(String),
}

/// Multi-layer tilemap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TileMap {
    /// Tile pixel/world size.
    pub tile_size: f32,
    /// Layers (bottom to top).
    pub layers: Vec<TileLayer>,
}

impl TileMap {
    /// Create with one collision layer.
    pub fn new(width: u32, height: u32, tile_size: f32) -> Result<Self, TileMapError> {
        if width == 0 || height == 0 || tile_size <= 0.0 {
            return Err(TileMapError::InvalidSize);
        }
        Ok(Self {
            tile_size,
            layers: vec![TileLayer::empty("main", width, height)],
        })
    }

    /// Primary layer.
    pub fn main_layer(&self) -> &TileLayer {
        &self.layers[0]
    }

    /// Primary layer mut.
    pub fn main_layer_mut(&mut self) -> &mut TileLayer {
        &mut self.layers[0]
    }

    /// World width.
    pub fn world_width(&self) -> f32 {
        self.main_layer().width as f32 * self.tile_size
    }

    /// World height.
    pub fn world_height(&self) -> f32 {
        self.main_layer().height as f32 * self.tile_size
    }

    /// Tile coords from world position (floor).
    pub fn world_to_tile(&self, pos: Vec2) -> (i32, i32) {
        (
            (pos.x / self.tile_size).floor() as i32,
            (pos.y / self.tile_size).floor() as i32,
        )
    }

    /// World rect of tile.
    pub fn tile_world_rect(&self, x: i32, y: i32) -> Rect {
        Rect::from_pos_size(
            Vec2::new(x as f32 * self.tile_size, y as f32 * self.tile_size),
            Vec2::splat(self.tile_size),
        )
    }

    /// Whether world point is solid on collision layers.
    pub fn is_solid_at(&self, pos: Vec2) -> bool {
        let (tx, ty) = self.world_to_tile(pos);
        for layer in &self.layers {
            if layer.collision && layer.get(tx, ty).flags.solid {
                return true;
            }
        }
        false
    }

    /// Collect solid colliders overlapping a world AABB (for physics).
    pub fn solid_colliders_in_aabb(&self, aabb: Rect) -> Vec<(Vec2, Collider)> {
        let mut out = Vec::new();
        let min_t = self.world_to_tile(aabb.min);
        let max_t = self.world_to_tile(aabb.max);
        let half = Vec2::splat(self.tile_size * 0.5);
        for layer in &self.layers {
            if !layer.collision {
                continue;
            }
            for ty in min_t.1.saturating_sub(1)..=max_t.1 + 1 {
                for tx in min_t.0.saturating_sub(1)..=max_t.0 + 1 {
                    if layer.get(tx, ty).flags.solid {
                        let center = Vec2::new(
                            (tx as f32 + 0.5) * self.tile_size,
                            (ty as f32 + 0.5) * self.tile_size,
                        );
                        let mut col = Collider::aabb(half);
                        col.layer = CollisionLayer::WORLD;
                        out.push((center, col));
                    }
                }
            }
        }
        out
    }

    /// Build a room with walls from ASCII ( `#` solid, `.` empty, `T` trigger).
    pub fn from_ascii(ascii: &str, tile_size: f32) -> Result<Self, TileMapError> {
        let lines: Vec<&str> = ascii
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            return Err(TileMapError::InvalidSize);
        }
        let height = lines.len() as u32;
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u32;
        let mut map = Self::new(width, height, tile_size)?;
        let layer = map.main_layer_mut();
        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                let tile = match ch {
                    '#' | 'X' => Tile::solid(1),
                    'T' => Tile {
                        id: 2,
                        flags: TileFlags {
                            solid: false,
                            trigger_id: 1,
                            kind: 1,
                        },
                    },
                    'D' => Tile {
                        id: 3,
                        flags: TileFlags {
                            solid: true,
                            trigger_id: 0,
                            kind: 2, // door
                        },
                    },
                    _ => Tile::EMPTY,
                };
                layer.set(x as i32, y as i32, tile);
            }
        }
        Ok(map)
    }
}

/// Wang-blob / bitmask autotiling helpers for 4-connected neighbors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutotileMask(pub u8);

impl AutotileMask {
    /// North neighbor solid/same.
    pub const N: u8 = 1;
    /// East.
    pub const E: u8 = 2;
    /// South.
    pub const S: u8 = 4;
    /// West.
    pub const W: u8 = 8;

    /// Raw bits.
    pub fn bits(self) -> u8 {
        self.0
    }

    /// Index 0..15 for a 4-neighbor tileset layout.
    pub fn index4(self) -> u16 {
        u16::from(self.0 & 0x0F)
    }
}

/// Compute 4-neighbor autotile mask for a predicate on tiles.
pub fn autotile_mask4(
    layer: &TileLayer,
    x: i32,
    y: i32,
    same: impl Fn(Tile) -> bool,
) -> AutotileMask {
    let mut m = 0u8;
    if same(layer.get(x, y - 1)) {
        m |= AutotileMask::N;
    }
    if same(layer.get(x + 1, y)) {
        m |= AutotileMask::E;
    }
    if same(layer.get(x, y + 1)) {
        m |= AutotileMask::S;
    }
    if same(layer.get(x - 1, y)) {
        m |= AutotileMask::W;
    }
    AutotileMask(m)
}

/// Apply autotile ids: for every tile matching `is_ground`, set `id = base_id + mask.index4()`.
pub fn apply_autotile4(
    layer: &mut TileLayer,
    base_id: u16,
    is_ground: impl Fn(Tile) -> bool + Copy,
) {
    let w = layer.width as i32;
    let h = layer.height as i32;
    let mut updates = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let t = layer.get(x, y);
            if !is_ground(t) {
                continue;
            }
            let mask = autotile_mask4(layer, x, y, is_ground);
            let mut nt = t;
            nt.id = base_id.saturating_add(mask.index4());
            updates.push((x, y, nt));
        }
    }
    for (x, y, t) in updates {
        layer.set(x, y, t);
    }
}

/// Flood-fill walkable (non-solid) region starting at tile, returns tile coords.
pub fn flood_fill_walkable(layer: &TileLayer, start_x: i32, start_y: i32) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    if layer.get(start_x, start_y).flags.solid {
        return out;
    }
    let mut stack = vec![(start_x, start_y)];
    let mut seen = std::collections::HashSet::new();
    while let Some((x, y)) = stack.pop() {
        if !seen.insert((x, y)) {
            continue;
        }
        if layer.get(x, y).flags.solid {
            continue;
        }
        if x < 0 || y < 0 || x as u32 >= layer.width || y as u32 >= layer.height {
            continue;
        }
        out.push((x, y));
        stack.push((x + 1, y));
        stack.push((x - 1, y));
        stack.push((x, y + 1));
        stack.push((x, y - 1));
    }
    out
}

/// Count solid neighbors (4-connected).
pub fn solid_neighbor_count(layer: &TileLayer, x: i32, y: i32) -> u8 {
    let mut n = 0u8;
    for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
        if layer.get(x + dx, y + dy).flags.solid {
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_room_solids() {
        let map = TileMap::from_ascii(
            "\
#####
#...#
#...#
#####",
            16.0,
        )
        .unwrap();
        assert!(map.is_solid_at(Vec2::new(8.0, 8.0))); // top-left wall center-ish
        assert!(!map.is_solid_at(Vec2::new(24.0, 24.0))); // interior
        assert_eq!(map.world_width(), 80.0);
    }

    #[test]
    fn colliders_collected() {
        let map = TileMap::from_ascii("##\n..", 10.0).unwrap();
        let cols =
            map.solid_colliders_in_aabb(Rect::from_pos_size(Vec2::ZERO, Vec2::new(30.0, 30.0)));
        assert_eq!(cols.len(), 2);
    }

    #[test]
    fn autotile_mask_corner() {
        let map = TileMap::from_ascii(
            "\
###
#.#
###",
            8.0,
        )
        .unwrap();
        let layer = map.main_layer();
        // Center empty has 4 solid neighbors
        let m = autotile_mask4(layer, 1, 1, |t| t.flags.solid);
        assert_eq!(
            m.bits(),
            AutotileMask::N | AutotileMask::E | AutotileMask::S | AutotileMask::W
        );
    }

    #[test]
    fn apply_autotile_sets_ids() {
        let mut map = TileMap::from_ascii("##\n##", 8.0).unwrap();
        apply_autotile4(map.main_layer_mut(), 100, |t| t.flags.solid);
        let t = map.main_layer().get(0, 0);
        assert!(t.id >= 100);
    }

    #[test]
    fn flood_fill_interior() {
        let map = TileMap::from_ascii(
            "\
#####
#...#
#...#
#####",
            16.0,
        )
        .unwrap();
        let cells = flood_fill_walkable(map.main_layer(), 1, 1);
        assert_eq!(cells.len(), 6);
    }
}
