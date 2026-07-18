//! Grid navigation and A*.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;

use crate::map::TileMap;

/// Integer grid point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NavPoint {
    /// X tile.
    pub x: i32,
    /// Y tile.
    pub y: i32,
}

impl NavPoint {
    /// Create.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan distance.
    pub fn manhattan(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

/// Path of world positions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    /// Waypoints in world space (tile centers).
    pub points: Vec<Vec2>,
}

impl Path {
    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Length.
    pub fn len(&self) -> usize {
        self.points.len()
    }
}

/// Walkability grid from tilemap.
#[derive(Debug, Clone)]
pub struct GridNav {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Tile size.
    pub tile_size: f32,
    /// solid[y * width + x]
    solid: Vec<bool>,
    /// Allow diagonal moves.
    pub diagonal: bool,
}

impl GridNav {
    /// From tilemap main collision layer.
    pub fn from_tilemap(map: &TileMap) -> Self {
        let layer = map.main_layer();
        let width = layer.width as i32;
        let height = layer.height as i32;
        let mut solid = vec![false; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                solid[(y * width + x) as usize] = layer.get(x, y).flags.solid;
            }
        }
        Self {
            width,
            height,
            tile_size: map.tile_size,
            solid,
            diagonal: false,
        }
    }

    /// Walkable.
    pub fn walkable(&self, p: NavPoint) -> bool {
        if p.x < 0 || p.y < 0 || p.x >= self.width || p.y >= self.height {
            return false;
        }
        !self.solid[(p.y * self.width + p.x) as usize]
    }

    /// World center of tile.
    pub fn to_world(&self, p: NavPoint) -> Vec2 {
        Vec2::new(
            (p.x as f32 + 0.5) * self.tile_size,
            (p.y as f32 + 0.5) * self.tile_size,
        )
    }

    /// Tile from world.
    pub fn from_world(&self, pos: Vec2) -> NavPoint {
        NavPoint::new(
            (pos.x / self.tile_size).floor() as i32,
            (pos.y / self.tile_size).floor() as i32,
        )
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    pos: NavPoint,
    f: i32,
    g: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* pathfinding on grid.
pub fn astar(nav: &GridNav, start: NavPoint, goal: NavPoint) -> Option<Path> {
    if !nav.walkable(start) || !nav.walkable(goal) {
        return None;
    }
    if start == goal {
        return Some(Path {
            points: vec![nav.to_world(start)],
        });
    }

    let mut open = BinaryHeap::new();
    open.push(Node {
        pos: start,
        f: start.manhattan(goal),
        g: 0,
    });
    let mut came: HashMap<NavPoint, NavPoint> = HashMap::new();
    let mut g_score: HashMap<NavPoint, i32> = HashMap::new();
    g_score.insert(start, 0);
    let mut closed = HashSet::new();

    let neighbors = if nav.diagonal {
        [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ]
        .as_slice()
    } else {
        [(1, 0), (-1, 0), (0, 1), (0, -1)].as_slice()
    };

    while let Some(Node { pos, g, .. }) = open.pop() {
        if pos == goal {
            let mut path = vec![goal];
            let mut cur = goal;
            while let Some(&prev) = came.get(&cur) {
                path.push(prev);
                cur = prev;
                if cur == start {
                    break;
                }
            }
            path.reverse();
            return Some(Path {
                points: path.into_iter().map(|p| nav.to_world(p)).collect(),
            });
        }
        if !closed.insert(pos) {
            continue;
        }
        for (dx, dy) in neighbors {
            let next = NavPoint::new(pos.x + dx, pos.y + dy);
            if !nav.walkable(next) {
                continue;
            }
            // Prevent corner cut on diagonal
            if *dx != 0
                && *dy != 0
                && (!nav.walkable(NavPoint::new(pos.x + dx, pos.y))
                    || !nav.walkable(NavPoint::new(pos.x, pos.y + dy)))
            {
                continue;
            }
            let cost = if *dx != 0 && *dy != 0 { 14 } else { 10 };
            let tentative = g + cost;
            if tentative < *g_score.get(&next).unwrap_or(&i32::MAX) {
                came.insert(next, pos);
                g_score.insert(next, tentative);
                let f = tentative + next.manhattan(goal) * 10;
                open.push(Node {
                    pos: next,
                    f,
                    g: tentative,
                });
            }
        }
    }
    None
}

/// Hierarchical / coarse grid for long-range pathfinding.
///
/// Fine cells are grouped into `sector_size x sector_size` sectors. A coarse graph
/// connects walkable sector centers; fine A* is used within/between portal cells.
#[derive(Debug, Clone)]
pub struct HierarchicalNav {
    /// Fine navigation grid.
    pub fine: GridNav,
    /// Sector size in tiles (e.g. 8).
    pub sector_size: i32,
    /// Coarse solid flags (sector blocked if all fine cells solid).
    coarse_solid: Vec<bool>,
    /// Coarse width in sectors.
    pub coarse_w: i32,
    /// Coarse height in sectors.
    pub coarse_h: i32,
}

impl HierarchicalNav {
    /// Build from fine grid.
    pub fn from_fine(fine: GridNav, sector_size: i32) -> Self {
        let sector_size = sector_size.max(2);
        let coarse_w = (fine.width + sector_size - 1) / sector_size;
        let coarse_h = (fine.height + sector_size - 1) / sector_size;
        let mut coarse_solid = vec![true; (coarse_w * coarse_h) as usize];
        for cy in 0..coarse_h {
            for cx in 0..coarse_w {
                let mut any_walk = false;
                for dy in 0..sector_size {
                    for dx in 0..sector_size {
                        let p = NavPoint::new(cx * sector_size + dx, cy * sector_size + dy);
                        if fine.walkable(p) {
                            any_walk = true;
                            break;
                        }
                    }
                    if any_walk {
                        break;
                    }
                }
                coarse_solid[(cy * coarse_w + cx) as usize] = !any_walk;
            }
        }
        Self {
            fine,
            sector_size,
            coarse_solid,
            coarse_w,
            coarse_h,
        }
    }

    /// From tilemap.
    pub fn from_tilemap(map: &TileMap, sector_size: i32) -> Self {
        Self::from_fine(GridNav::from_tilemap(map), sector_size)
    }

    fn coarse_walkable(&self, cx: i32, cy: i32) -> bool {
        if cx < 0 || cy < 0 || cx >= self.coarse_w || cy >= self.coarse_h {
            return false;
        }
        !self.coarse_solid[(cy * self.coarse_w + cx) as usize]
    }

    fn sector_of(&self, p: NavPoint) -> (i32, i32) {
        (p.x / self.sector_size, p.y / self.sector_size)
    }

    /// Representative walkable point near sector center, if any.
    pub fn sector_portal(&self, cx: i32, cy: i32) -> Option<NavPoint> {
        if !self.coarse_walkable(cx, cy) {
            return None;
        }
        let base_x = cx * self.sector_size;
        let base_y = cy * self.sector_size;
        let center = NavPoint::new(base_x + self.sector_size / 2, base_y + self.sector_size / 2);
        if self.fine.walkable(center) {
            return Some(center);
        }
        for dy in 0..self.sector_size {
            for dx in 0..self.sector_size {
                let p = NavPoint::new(base_x + dx, base_y + dy);
                if self.fine.walkable(p) {
                    return Some(p);
                }
            }
        }
        None
    }

    /// Coarse A* over sectors returning sector centers as waypoints.
    pub fn coarse_path(&self, start: NavPoint, goal: NavPoint) -> Option<Vec<NavPoint>> {
        let (sx, sy) = self.sector_of(start);
        let (gx, gy) = self.sector_of(goal);
        if !self.coarse_walkable(sx, sy) || !self.coarse_walkable(gx, gy) {
            return None;
        }
        if (sx, sy) == (gx, gy) {
            return Some(vec![start, goal]);
        }

        #[derive(Copy, Clone, Eq, PartialEq)]
        struct CNode {
            x: i32,
            y: i32,
            f: i32,
            g: i32,
        }
        impl Ord for CNode {
            fn cmp(&self, other: &Self) -> Ordering {
                other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
            }
        }
        impl PartialOrd for CNode {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut open = BinaryHeap::new();
        open.push(CNode {
            x: sx,
            y: sy,
            f: (sx - gx).abs() + (sy - gy).abs(),
            g: 0,
        });
        let mut came: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
        let mut g_score: HashMap<(i32, i32), i32> = HashMap::new();
        g_score.insert((sx, sy), 0);
        let mut closed = HashSet::new();

        while let Some(CNode { x, y, g, .. }) = open.pop() {
            if x == gx && y == gy {
                let mut path = vec![(gx, gy)];
                let mut cur = (gx, gy);
                while let Some(&prev) = came.get(&cur) {
                    path.push(prev);
                    cur = prev;
                    if cur == (sx, sy) {
                        break;
                    }
                }
                path.reverse();
                let mut points = Vec::new();
                points.push(start);
                for (cx, cy) in path {
                    if let Some(p) = self.sector_portal(cx, cy) {
                        if points.last() != Some(&p) {
                            points.push(p);
                        }
                    }
                }
                if points.last() != Some(&goal) {
                    points.push(goal);
                }
                return Some(points);
            }
            if !closed.insert((x, y)) {
                continue;
            }
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let nx = x + dx;
                let ny = y + dy;
                if !self.coarse_walkable(nx, ny) {
                    continue;
                }
                let tentative = g + 1;
                if tentative < *g_score.get(&(nx, ny)).unwrap_or(&i32::MAX) {
                    came.insert((nx, ny), (x, y));
                    g_score.insert((nx, ny), tentative);
                    let f = tentative + (nx - gx).abs() + (ny - gy).abs();
                    open.push(CNode {
                        x: nx,
                        y: ny,
                        f,
                        g: tentative,
                    });
                }
            }
        }
        None
    }

    /// Hierarchical path: coarse waypoints stitched with fine A*.
    pub fn find_path(&self, start: NavPoint, goal: NavPoint) -> Option<Path> {
        if start == goal {
            return astar(&self.fine, start, goal);
        }
        // Same sector or small maps: fine only.
        let (sx, sy) = self.sector_of(start);
        let (gx, gy) = self.sector_of(goal);
        if (sx, sy) == (gx, gy) || self.coarse_w * self.coarse_h <= 4 {
            return astar(&self.fine, start, goal);
        }
        let coarse = self.coarse_path(start, goal)?;
        let mut points = Vec::new();
        for window in coarse.windows(2) {
            let a = window[0];
            let b = window[1];
            let seg = astar(&self.fine, a, b)?;
            if points.is_empty() {
                points.extend(seg.points);
            } else {
                // skip first point to avoid duplicates
                points.extend(seg.points.into_iter().skip(1));
            }
        }
        if points.is_empty() {
            None
        } else {
            Some(Path { points })
        }
    }
}

/// Smooth a path by removing unnecessary intermediate waypoints (line-of-sight).
pub fn smooth_path(nav: &GridNav, path: &Path) -> Path {
    if path.points.len() <= 2 {
        return path.clone();
    }
    let mut out = vec![path.points[0]];
    let mut i = 0;
    while i < path.points.len() - 1 {
        let mut j = path.points.len() - 1;
        let mut found = i + 1;
        while j > i + 1 {
            if line_of_sight(nav, path.points[i], path.points[j]) {
                found = j;
                break;
            }
            j -= 1;
        }
        out.push(path.points[found]);
        i = found;
    }
    Path { points: out }
}

fn line_of_sight(nav: &GridNav, a: Vec2, b: Vec2) -> bool {
    let steps = ((a - b).length() / (nav.tile_size * 0.5)).ceil().max(1.0) as i32;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let p = a.lerp(b, t);
        if !nav.walkable(nav.from_world(p)) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::TileMap;

    #[test]
    fn path_around_wall() {
        let map = TileMap::from_ascii(
            "\
#####
#..##
##..#
#####",
            16.0,
        )
        .unwrap();
        let nav = GridNav::from_tilemap(&map);
        let path = astar(&nav, NavPoint::new(1, 1), NavPoint::new(3, 2)).expect("path");
        assert!(path.len() >= 2);
    }

    #[test]
    fn no_path_when_blocked() {
        let map = TileMap::from_ascii("#.#\n###\n#.#", 8.0).unwrap();
        let nav = GridNav::from_tilemap(&map);
        assert!(astar(&nav, NavPoint::new(0, 0), NavPoint::new(2, 2)).is_none());
    }

    #[test]
    fn hierarchical_finds_path() {
        let map = TileMap::from_ascii(
            "\
##########
#........#
#.######.#
#........#
##########",
            16.0,
        )
        .unwrap();
        let hnav = HierarchicalNav::from_tilemap(&map, 4);
        let path = hnav
            .find_path(NavPoint::new(1, 1), NavPoint::new(8, 3))
            .expect("path");
        assert!(path.len() >= 2);
    }

    #[test]
    fn smooth_shortens_or_equals() {
        let map = TileMap::from_ascii(
            "\
#####
#...#
#...#
#####",
            16.0,
        )
        .unwrap();
        let nav = GridNav::from_tilemap(&map);
        let path = astar(&nav, NavPoint::new(1, 1), NavPoint::new(3, 2)).unwrap();
        let sm = smooth_path(&nav, &path);
        assert!(sm.len() <= path.len());
        assert!(!sm.is_empty());
    }

    #[test]
    fn long_corridor_path() {
        // 1x20 open corridor with walls on sides
        let mut rows = vec!["######################".to_string()];
        rows.push("#....................#".to_string());
        rows.push("######################".to_string());
        let ascii = rows.join("\n");
        let map = TileMap::from_ascii(&ascii, 16.0).unwrap();
        let nav = GridNav::from_tilemap(&map);
        let path = astar(&nav, NavPoint::new(1, 1), NavPoint::new(20, 1)).expect("long path");
        assert!(path.len() >= 15, "len={}", path.len());
        // World points should be roughly increasing in x.
        let mut prev_x = f32::MIN;
        for p in &path.points {
            assert!(p.x + 1.0 >= prev_x, "non-monotonic x");
            prev_x = p.x;
        }
    }

    #[test]
    fn maze_finds_path_around_blockers() {
        let map = TileMap::from_ascii(
            "\
###########
#.#.......#
#.#.#####.#
#.#.#...#.#
#...#.#.#.#
#####.#.#.#
#.....#...#
#.#####.###
#.........#
###########",
            16.0,
        )
        .unwrap();
        let nav = GridNav::from_tilemap(&map);
        let path = astar(&nav, NavPoint::new(1, 1), NavPoint::new(9, 8)).expect("maze path");
        assert!(path.len() >= 4);
        // Start and end near intended tiles.
        let start = path.points.first().unwrap();
        let end = path.points.last().unwrap();
        assert!(start.x < 40.0);
        assert!(end.x > 100.0 || path.len() > 5);
    }

    #[test]
    fn same_start_end_trivial() {
        let map = TileMap::from_ascii("...\n...\n...", 8.0).unwrap();
        let nav = GridNav::from_tilemap(&map);
        let path = astar(&nav, NavPoint::new(1, 1), NavPoint::new(1, 1)).unwrap();
        assert!(path.len() <= 2);
    }

    #[test]
    fn manhattan_heuristic_properties() {
        let a = NavPoint::new(0, 0);
        let b = NavPoint::new(3, 4);
        assert_eq!(a.manhattan(b), 7);
        assert_eq!(b.manhattan(a), 7);
        assert_eq!(a.manhattan(a), 0);
    }
}
