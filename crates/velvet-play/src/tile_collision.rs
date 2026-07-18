//! Helpers for resolving entity movement against solid tilemap cells.

use velvet_math::{Rect, Vec2};

use crate::map::TileMap;
use crate::physics::{collide_aabb, CollisionHit};

/// Result of resolving an entity AABB against solid tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct TileCollisionResult {
    /// Corrected position (center of the entity AABB).
    pub position: Vec2,
    /// Hits against solid tiles this step.
    pub hits: Vec<CollisionHit>,
    /// Whether any solid was touched.
    pub collided: bool,
}

/// Build an AABB from center + half extents.
#[inline]
pub fn aabb_from_center(center: Vec2, half_extents: Vec2) -> Rect {
    Rect::from_center_half_size(center, half_extents)
}

fn any_solid_overlap(map: &TileMap, center: Vec2, half_extents: Vec2) -> bool {
    let aabb = aabb_from_center(center, half_extents);
    let pad = aabb_from_center(center, half_extents + Vec2::splat(0.5));
    for (tile_center, col) in map.solid_colliders_in_aabb(pad) {
        let tile_aabb = col.world_aabb(tile_center);
        if collide_aabb(aabb, tile_aabb).is_some() {
            return true;
        }
    }
    false
}

/// Whether the entity AABB overlaps any solid tile.
pub fn overlaps_solid(map: &TileMap, center: Vec2, half_extents: Vec2) -> bool {
    any_solid_overlap(map, center, half_extents)
}

/// Separate an entity AABB out of solid tiles (multi-pass depenetration).
pub fn resolve_tile_penetration(
    map: &TileMap,
    mut center: Vec2,
    half_extents: Vec2,
    max_iterations: u32,
) -> TileCollisionResult {
    let mut hits = Vec::new();
    let iterations = max_iterations.max(1);
    for _ in 0..iterations {
        let aabb = aabb_from_center(center, half_extents);
        let pad = aabb_from_center(center, half_extents + Vec2::splat(1.0));
        let solids = map.solid_colliders_in_aabb(pad);
        let mut moved = false;
        // Resolve the deepest penetration first for stability.
        let mut best: Option<(f32, Vec2, usize)> = None;
        for (i, (tile_center, col)) in solids.iter().enumerate() {
            let tile_aabb = col.world_aabb(*tile_center);
            if let Some((depth, normal)) = collide_aabb(aabb, tile_aabb) {
                if best.map(|b| depth > b.0).unwrap_or(true) {
                    best = Some((depth, normal, i));
                }
            }
        }
        if let Some((depth, normal, i)) = best {
            center += normal * (depth + 0.001);
            hits.push(CollisionHit {
                index: i,
                depth,
                normal,
            });
            moved = true;
        }
        if !moved {
            break;
        }
    }
    let collided = !hits.is_empty();
    TileCollisionResult {
        position: center,
        hits,
        collided,
    }
}

/// Sweep one axis: sample along the path so tunneling past thin solids is caught.
fn sweep_axis(
    map: &TileMap,
    from: Vec2,
    target_axis: f32,
    horizontal: bool,
    half_extents: Vec2,
) -> (f32, bool) {
    let start = if horizontal { from.x } else { from.y };
    let delta = target_axis - start;
    if delta.abs() < 1e-6 {
        return (start, false);
    }
    let test = |v: f32| {
        let p = if horizontal {
            Vec2::new(v, from.y)
        } else {
            Vec2::new(from.x, v)
        };
        any_solid_overlap(map, p, half_extents)
    };
    // Step size ~ half a tile so we cannot skip solids.
    let tile = map.tile_size.max(1.0);
    let step = (tile * 0.25).max(0.5);
    let steps = ((delta.abs() / step).ceil() as usize).max(1);
    let mut last_free = start;
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let v = start + delta * t;
        if test(v) {
            // Refine last free via binary search between last_free and v.
            let mut lo = last_free;
            let mut hi = v;
            for _ in 0..10 {
                let mid = (lo + hi) * 0.5;
                if test(mid) {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            return (lo, true);
        }
        last_free = v;
    }
    (target_axis, false)
}

/// Move an AABB by `delta`, sliding against solid tiles (axis-separated sweep).
pub fn move_vs_tiles(
    map: &TileMap,
    center: Vec2,
    half_extents: Vec2,
    delta: Vec2,
) -> TileCollisionResult {
    let mut pos = center;
    let mut hits = Vec::new();
    let mut collided = false;

    // X axis
    let (nx, hit_x) = sweep_axis(map, pos, pos.x + delta.x, true, half_extents);
    if hit_x {
        collided = true;
        hits.push(CollisionHit {
            index: 0,
            depth: (pos.x + delta.x - nx).abs(),
            normal: if delta.x > 0.0 {
                Vec2::new(-1.0, 0.0)
            } else {
                Vec2::new(1.0, 0.0)
            },
        });
    }
    pos.x = nx;

    // Y axis
    let (ny, hit_y) = sweep_axis(map, pos, pos.y + delta.y, false, half_extents);
    if hit_y {
        collided = true;
        hits.push(CollisionHit {
            index: 1,
            depth: (pos.y + delta.y - ny).abs(),
            normal: if delta.y > 0.0 {
                Vec2::new(0.0, -1.0)
            } else {
                Vec2::new(0.0, 1.0)
            },
        });
    }
    pos.y = ny;

    TileCollisionResult {
        position: pos,
        hits,
        collided,
    }
}

/// Sample whether the four corners / center of an AABB sit on solid tiles.
pub fn solid_contacts(map: &TileMap, center: Vec2, half_extents: Vec2) -> SolidContacts {
    let hx = half_extents.x;
    let hy = half_extents.y;
    SolidContacts {
        center: map.is_solid_at(center),
        top_left: map.is_solid_at(center + Vec2::new(-hx, hy)),
        top_right: map.is_solid_at(center + Vec2::new(hx, hy)),
        bottom_left: map.is_solid_at(center + Vec2::new(-hx, -hy)),
        bottom_right: map.is_solid_at(center + Vec2::new(hx, -hy)),
    }
}

/// Corner solid samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SolidContacts {
    /// Center sample.
    pub center: bool,
    /// Top-left.
    pub top_left: bool,
    /// Top-right.
    pub top_right: bool,
    /// Bottom-left.
    pub bottom_left: bool,
    /// Bottom-right.
    pub bottom_right: bool,
}

impl SolidContacts {
    /// Any solid contact.
    pub fn any(self) -> bool {
        self.center || self.top_left || self.top_right || self.bottom_left || self.bottom_right
    }

    /// Either bottom corner solid.
    pub fn bottom_any(self) -> bool {
        self.bottom_left || self.bottom_right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wall_map() -> TileMap {
        TileMap::from_ascii(
            "\
#####
#...#
#...#
#####",
            16.0,
        )
        .unwrap()
    }

    #[test]
    fn move_blocked_by_wall() {
        let map = wall_map();
        // Interior cell center around (24, 24)
        let start = Vec2::new(24.0, 24.0);
        let half = Vec2::splat(4.0);
        // Try to move into left wall
        let r = move_vs_tiles(&map, start, half, Vec2::new(-40.0, 0.0));
        // Unimpeded would be x = -16; we should stay in the room.
        assert!(r.position.x > 12.0, "pos.x = {}", r.position.x);
        assert!(r.collided);
        assert!(r.position.x < start.x);
    }

    #[test]
    fn free_move_interior() {
        let map = wall_map();
        let start = Vec2::new(32.0, 32.0);
        let half = Vec2::splat(2.0);
        let r = move_vs_tiles(&map, start, half, Vec2::new(4.0, 0.0));
        assert!((r.position.x - 36.0).abs() < 0.1);
        assert!(!r.collided);
    }

    #[test]
    fn contacts_on_wall() {
        let map = wall_map();
        let c = solid_contacts(&map, Vec2::new(8.0, 8.0), Vec2::splat(1.0));
        assert!(c.any());
    }

    #[test]
    fn depenetrate_spawn_in_wall() {
        let map = wall_map();
        // Center of top-left solid tile
        let stuck = Vec2::new(8.0, 8.0);
        let half = Vec2::splat(6.0);
        let r = resolve_tile_penetration(&map, stuck, half, 8);
        assert!(r.collided);
        assert!(r.position.distance(stuck) > 0.01);
    }
}
