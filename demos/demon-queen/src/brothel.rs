//! El Corazón Carmesí: the brothel's interior, reached from its building in the villa.
//!
//! The house was designed and built in the Three.js prototype
//! (`demos/demon-queen-brothel-threejs`) and exported by its `tools/export-game.mjs`:
//! the baked GLB parts live in `data/assets/3d/brothel/`, the plan in
//! `data/brothel/brothel.json`. This module reads the plan (walls, colliders, the
//! stair between the two floors, lamps, people, hotspots and their lines), walks the
//! Queen through it, and draws it.
//!
//! Two floors: the ground floor (hall, reception, salon, bar, waiting room) and the
//! bedrooms upstairs. The upper floor is only drawn while she is on it (or past the
//! middle of the stair); stepping into a bedroom lifts its front wall and the roofs.
//! A velvet rope keeps the stair shut until she has spoken to Rosalía at the desk.
use super::*;
use serde::Deserialize;

use std::collections::BTreeMap;
use velvet_render::ToonFire3D;

const RADIUS: f32 = 0.28;
const WALL_T: f32 = 0.5;

// ------------------------------------------------------------------ the plan
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Rect {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
}

impl Rect {
    fn contains(self, x: f32, z: f32) -> bool {
        x >= self.x0 && x <= self.x1 && z >= self.z0 && z <= self.z1
    }
}

#[derive(Deserialize)]
pub(crate) struct Stair {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    pub rise: f32,
}

#[derive(Deserialize)]
pub(crate) struct Door {
    pub x: f32,
    pub z0: f32,
    pub z1: f32,
}

#[derive(Deserialize)]
pub(crate) struct Spawn {
    pub x: f32,
    pub z: f32,
}

#[derive(Deserialize)]
pub(crate) struct Wall {
    pub level: u8,
    #[serde(default)]
    pub kind: String,
    pub axis: String,
    pub at: f32,
    pub pieces: Vec<[f32; 2]>,
}

#[derive(Deserialize)]
pub(crate) struct Collider {
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub d: f32,
    pub level: u8,
}

#[derive(Deserialize)]
pub(crate) struct Zone {
    pub id: String,
    pub label: String,
    pub level: u8,
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Room {
    pub index: usize,
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    pub door_x: f32,
    #[serde(default)]
    pub locked: bool,
}

#[derive(Deserialize)]
pub(crate) struct Npc {
    pub id: String,
    #[serde(default)]
    pub level: u8,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    #[serde(default)]
    pub model: Option<String>,
    /// How the prototype placed her: `desk`, `tray`, `sit`, `stand`.
    #[serde(default)]
    pub pose: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Hotspot {
    pub id: String,
    pub label: String,
    pub icon: String,
    #[serde(default)]
    pub level: u8,
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    #[serde(default)]
    pub dialogue: Option<String>,
    pub prompt: String,
    #[serde(default)]
    pub stairs: bool,
    #[serde(default)]
    pub exit: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub room: Option<usize>,
    #[serde(default)]
    pub anchor: Option<String>,
    pub bubble_x: f32,
    pub bubble_z: f32,
}

#[derive(Deserialize)]
pub(crate) struct Line {
    pub who: String,
    pub text: String,
}

#[derive(Deserialize)]
pub(crate) struct Lamp {
    pub level: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub power: f32,
    #[serde(default)]
    pub fire: bool,
    #[serde(default)]
    pub both: bool,
}

#[derive(Deserialize)]
pub(crate) struct Hinge {
    pub room: usize,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub locked: bool,
    pub max: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Plan {
    pub level_height: f32,
    pub gallery_z: f32,
    pub fronts_z: f32,
    pub bounds: Rect,
    pub stair: Stair,
    pub door: Door,
    pub upper_floor: Vec<Rect>,
    pub spawn: Spawn,
    pub walls: Vec<Wall>,
    pub boxes: Vec<Collider>,
    pub zones: Vec<Zone>,
    pub rooms: Vec<Room>,
    pub npcs: Vec<Npc>,
    pub hotspots: Vec<Hotspot>,
    pub dialogues: BTreeMap<String, Vec<Line>>,
    pub lamps: Vec<Lamp>,
    pub doors: Vec<Hinge>,
}

pub(crate) fn plan() -> &'static Plan {
    static PLAN: std::sync::OnceLock<Plan> = std::sync::OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("brothel/brothel.json")).expect("valid brothel plan")
    })
}

fn asset_root() -> std::path::PathBuf {
    runtime_3d_root().join("brothel")
}

// ---------------------------------------------------------------- walking
/// Where the Queen stands: her floor, and whether she is on the stair between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Walker {
    pub x: f32,
    pub z: f32,
    pub level: u8,
    pub on_stair: bool,
}

impl Plan {
    fn wall_blocks(&self, x: f32, z: f32, r: f32, level: u8) -> bool {
        let half = WALL_T / 2.0 + r;
        self.walls.iter().filter(|w| w.level == level).any(|w| {
            w.pieces.iter().any(|&[a, b]| {
                if w.axis == "x" {
                    (z - w.at).abs() < half && x > a - r * 0.5 && x < b + r * 0.5
                } else {
                    (x - w.at).abs() < half && z > a - r * 0.5 && z < b + r * 0.5
                }
            })
        })
    }

    fn on_upper_floor(&self, x: f32, z: f32) -> bool {
        self.upper_floor.iter().any(|q| q.contains(x, z))
    }

    /// The building's outline, [x0, z0, x1, z1] (for the QA walker).
    pub(crate) fn bounds(&self) -> [f32; 4] {
        [
            self.bounds.x0,
            self.bounds.z0,
            self.bounds.x1,
            self.bounds.z1,
        ]
    }

    /// Out of bounds or inside something on this floor (the stair is walked with [`Plan::step`]).
    pub(crate) fn blocked(&self, x: f32, z: f32, level: u8) -> bool {
        let (b, s, r) = (self.bounds, &self.stair, RADIUS);
        if level == 0 {
            if x < b.x0 + 0.5 || z < b.z0 + 0.5 || z > b.z1 - 0.4 || x > b.x1 + 0.2 {
                return true;
            }
            if x > s.x0 - r && x < s.x1 + r && z > s.z0 - r && z < s.z1 + r {
                return true;
            }
        } else {
            for (px, pz) in [(x, z), (x + r, z), (x - r, z), (x, z + r), (x, z - r)] {
                if !self.on_upper_floor(px, pz) {
                    return true;
                }
            }
        }
        if self.wall_blocks(x, z, r, level) {
            return true;
        }
        self.boxes.iter().any(|c| {
            c.level == level && (x - c.x).abs() < c.w / 2.0 + r && (z - c.z).abs() < c.d / 2.0 + r
        })
    }

    /// 0 at the foot of the stair, 1 at its top.
    pub(crate) fn stair_t(&self, x: f32) -> f32 {
        ((self.stair.x1 - x) / (self.stair.x1 - self.stair.x0)).clamp(0.0, 1.0)
    }

    fn in_lane(&self, z: f32) -> bool {
        z >= self.stair.z0 + RADIUS && z <= self.stair.z1 - RADIUS
    }

    /// One step: entered at the stair's foot (once the rope is off) or its top; on it only
    /// its sides stop her, and the floor she is on flips at half its height.
    pub(crate) fn step(&self, s: Walker, dx: f32, dz: f32, rope_open: bool) -> Option<Walker> {
        let (x, z, st, r) = (s.x + dx, s.z + dz, &self.stair, RADIUS);
        if s.on_stair {
            if !self.in_lane(z) {
                return None;
            }
            if x > st.x1 + r {
                return (!self.blocked(x, z, 0)).then_some(Walker {
                    x,
                    z,
                    level: 0,
                    on_stair: false,
                });
            }
            if x < st.x0 - r {
                return (!self.blocked(x, z, 1)).then_some(Walker {
                    x,
                    z,
                    level: 1,
                    on_stair: false,
                });
            }
            let level = if self.stair_t(x) > 0.5 { 1 } else { 0 };
            return Some(Walker {
                x,
                z,
                level,
                on_stair: true,
            });
        }
        if self.in_lane(z) {
            if s.level == 0 && rope_open && s.x >= st.x1 + r - 1e-4 && x < st.x1 + r {
                return Some(Walker {
                    x,
                    z,
                    level: 0,
                    on_stair: true,
                });
            }
            if s.level == 1 && s.x <= st.x0 - r + 1e-4 && x > st.x0 - r {
                return Some(Walker {
                    x,
                    z,
                    level: 1,
                    on_stair: true,
                });
            }
        }
        (!self.blocked(x, z, s.level)).then_some(Walker {
            x,
            z,
            level: s.level,
            on_stair: false,
        })
    }

    /// As far as she can go: straight, else along one axis (sliding along walls).
    pub(crate) fn slide(&self, s: Walker, dx: f32, dz: f32, rope_open: bool) -> Walker {
        // Sliding along an axis counts only when it gets her somewhere (a step straight at a wall
        // slides by nothing); else she eases round the corner; else the little slide there is.
        let len = dx.hypot(dz);
        let real = |v: f32| v.abs() > 0.2 * len;
        self.step(s, dx, dz, rope_open)
            .or_else(|| real(dx).then(|| self.step(s, dx, 0.0, rope_open)).flatten())
            .or_else(|| real(dz).then(|| self.step(s, 0.0, dz, rope_open)).flatten())
            .or_else(|| {
                super::corner_assist(dx, dz, |ax, az| self.step(s, ax, az, rope_open).is_some())
                    .and_then(|(ax, az)| self.step(s, ax, az, rope_open))
            })
            .or_else(|| self.step(s, dx, 0.0, rope_open))
            .or_else(|| self.step(s, 0.0, dz, rope_open))
            .unwrap_or(s)
    }

    pub(crate) fn floor_y(&self, s: Walker) -> f32 {
        if s.on_stair {
            self.stair_t(s.x) * self.stair.rise
        } else {
            s.level as f32 * self.level_height
        }
    }

    /// The floor on screen: hers, or on the stair the one she is nearer to.
    pub(crate) fn view_level(&self, s: Walker) -> u8 {
        if s.on_stair {
            (self.stair_t(s.x) > 0.5) as u8
        } else {
            s.level
        }
    }

    pub(crate) fn room_at(&self, s: Walker) -> Option<&Room> {
        if s.on_stair || s.level != 1 {
            return None;
        }
        self.rooms
            .iter()
            .find(|r| s.x > r.x0 && s.x < r.x1 && s.z > r.z0 + 0.1 && s.z < r.z1 - 0.2)
    }

    pub(crate) fn zone_at(&self, x: f32, z: f32, level: u8) -> Option<&Zone> {
        self.zones
            .iter()
            .find(|q| q.level == level && x >= q.x0 && x < q.x1 && z >= q.z0 && z < q.z1)
    }

    /// Where the bubble or the spoken line of a hotspot floats.
    fn anchor(&self, h: &Hotspot) -> Vec3 {
        Vec3::new(h.bubble_x, h.level as f32 * self.level_height, h.bubble_z)
    }
}

// ------------------------------------------------------------- finding the way
/// The walkable grid's cell (m): fine enough for the gaps between the furniture.
const CELL: f32 = 0.125;

/// One floor as a grid of cells where she fits (built once from [`Plan::blocked`]).
struct Grid {
    x0: f32,
    z0: f32,
    cols: usize,
    rows: usize,
    free: Vec<bool>,
}

impl Grid {
    fn new(p: &Plan, level: u8) -> Self {
        let (x0, z0) = (p.bounds.x0, p.bounds.z0);
        let cols = ((p.bounds.x1 - x0) / CELL).ceil() as usize;
        let rows = ((p.bounds.z1 - z0) / CELL).ceil() as usize;
        let mut free = vec![false; cols * rows];
        for r in 0..rows {
            for c in 0..cols {
                let (x, z) = (x0 + (c as f32 + 0.5) * CELL, z0 + (r as f32 + 0.5) * CELL);
                free[r * cols + c] = !p.blocked(x, z, level);
            }
        }
        Self {
            x0,
            z0,
            cols,
            rows,
            free,
        }
    }

    fn center(&self, cell: usize) -> (f32, f32) {
        let (c, r) = (cell % self.cols, cell / self.cols);
        (
            self.x0 + (c as f32 + 0.5) * CELL,
            self.z0 + (r as f32 + 0.5) * CELL,
        )
    }

    fn free_at(&self, c: i32, r: i32) -> bool {
        c >= 0
            && r >= 0
            && (c as usize) < self.cols
            && (r as usize) < self.rows
            && self.free[r as usize * self.cols + c as usize]
    }

    /// The free cell nearest (x, z): where a girl on her sofa first stands.
    fn nearest_free(&self, x: f32, z: f32) -> Option<usize> {
        let c0 = ((x - self.x0) / CELL).floor() as i32;
        let r0 = ((z - self.z0) / CELL).floor() as i32;
        for ring in 0i32..24 {
            let mut best: Option<(f32, usize)> = None;
            for dr in -ring..=ring {
                for dc in -ring..=ring {
                    if dr.abs() != ring && dc.abs() != ring {
                        continue;
                    }
                    let (c, r) = (c0 + dc, r0 + dr);
                    if self.free_at(c, r) {
                        let cell = r as usize * self.cols + c as usize;
                        let (cx, cz) = self.center(cell);
                        let d = (cx - x).hypot(cz - z);
                        if best.map_or(true, |(bd, _)| d < bd) {
                            best = Some((d, cell));
                        }
                    }
                }
            }
            if let Some((_, cell)) = best {
                return Some(cell);
            }
        }
        None
    }
}

fn grids() -> &'static [Grid; 2] {
    static GRIDS: std::sync::OnceLock<[Grid; 2]> = std::sync::OnceLock::new();
    GRIDS.get_or_init(|| {
        let p = plan();
        [Grid::new(p, 0), Grid::new(p, 1)]
    })
}

impl Plan {
    /// Nothing in the way on a straight line from `a` to `b` on this floor.
    fn clear_line(&self, level: u8, a: (f32, f32), b: (f32, f32)) -> bool {
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let n = (len / 0.04).ceil().max(1.0) as usize;
        (0..=n).all(|i| {
            let t = i as f32 / n as f32;
            !self.blocked(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, level)
        })
    }

    /// A way across one floor from `from` to `to`, round whatever is in the way: A* on the
    /// grid, then straightened (a corner stays only where the straight line would hit
    /// something). `from` may be inside something (a girl on her sofa): she steps out first.
    pub(crate) fn find_path(
        &self,
        level: u8,
        from: (f32, f32),
        to: (f32, f32),
    ) -> Option<Vec<(f32, f32)>> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;
        let grid = &grids()[level as usize];
        let start = grid.nearest_free(from.0, from.1)?;
        let goal = grid.nearest_free(to.0, to.1)?;
        let n = grid.cols * grid.rows;
        let octile = |a: usize, b: usize| {
            let dc = (a % grid.cols).abs_diff(b % grid.cols) as u32;
            let dr = (a / grid.cols).abs_diff(b / grid.cols) as u32;
            10 * dc.max(dr) + 4 * dc.min(dr)
        };
        let mut cost = vec![u32::MAX; n];
        let mut came = vec![usize::MAX; n];
        let mut open = BinaryHeap::new();
        cost[start] = 0;
        open.push(Reverse((octile(start, goal), start)));
        while let Some(Reverse((_, cell))) = open.pop() {
            if cell == goal {
                break;
            }
            let (c, r) = ((cell % grid.cols) as i32, (cell / grid.cols) as i32);
            for (dc, dr) in [
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ] {
                let (nc, nr) = (c + dc, r + dr);
                if !grid.free_at(nc, nr) {
                    continue;
                }
                // No cutting a corner between two blocked cells.
                if dc != 0 && dr != 0 && !(grid.free_at(c + dc, r) && grid.free_at(c, r + dr)) {
                    continue;
                }
                let next = nr as usize * grid.cols + nc as usize;
                let step = if dc != 0 && dr != 0 { 14 } else { 10 };
                let g = cost[cell] + step;
                if g < cost[next] {
                    cost[next] = g;
                    came[next] = cell;
                    open.push(Reverse((g + octile(next, goal), next)));
                }
            }
        }
        if cost[goal] == u32::MAX {
            return None;
        }
        let mut cells = vec![goal];
        while let Some(&last) = cells.last() {
            if last == start {
                break;
            }
            cells.push(came[last]);
        }
        cells.reverse();
        let mut points: Vec<(f32, f32)> = cells.iter().map(|&c| grid.center(c)).collect();
        if !self.blocked(to.0, to.1, level) {
            points.push(to);
        }
        // Straighten: from each kept point, on to the farthest one in plain sight.
        let mut out = Vec::new();
        let first_inside = self.blocked(from.0, from.1, level);
        out.push(from);
        let mut i = 0;
        if first_inside {
            // Out of the sofa first, then on.
            out.push(points[0]);
        } else {
            points.insert(0, from);
        }
        while i + 1 < points.len() {
            let mut j = points.len() - 1;
            while j > i + 1 && !self.clear_line(level, points[i], points[j]) {
                j -= 1;
            }
            out.push(points[j]);
            i = j;
        }
        out.dedup_by(|a, b| (a.0 - b.0).hypot(a.1 - b.1) < 0.02);
        Some(out)
    }

    /// The foot of the stair (in its lane, clear of it) and the landing at its top.
    fn stair_ends(&self) -> ((f32, f32), (f32, f32)) {
        let lane = (self.stair.z0 + self.stair.z1) * 0.5;
        (
            (self.stair.x1 + RADIUS + 0.35, lane),
            (self.stair.x0 - RADIUS - 0.3, lane),
        )
    }

    /// From the ground floor up the stair to `to` upstairs.
    pub(crate) fn way_up(&self, from: (f32, f32), to: (f32, f32)) -> Option<Route> {
        let (foot, top) = self.stair_ends();
        let below = self.find_path(0, from, foot)?;
        let above = self.find_path(1, top, to)?;
        let mut points: Vec<Waypoint> = below
            .into_iter()
            .map(|(x, z)| Waypoint { x, z, level: 0 })
            .collect();
        points.extend(above.into_iter().map(|(x, z)| Waypoint { x, z, level: 1 }));
        Some(Route::new(points))
    }
}

// ---------------------------------------------------------------- routes
/// A point of a route, and the floor it is on (a ground point followed by an upper one is
/// the stair between them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Waypoint {
    pub x: f32,
    pub z: f32,
    pub level: u8,
}

/// A walked line through waypoints, measured.
#[derive(Clone, Debug, Default)]
pub(crate) struct Route {
    points: Vec<Waypoint>,
    at: Vec<f32>,
}

impl Route {
    pub(crate) fn new(mut points: Vec<Waypoint>) -> Self {
        points.dedup_by(|a, b| (a.x - b.x).hypot(a.z - b.z) < 0.01 && a.level == b.level);
        let mut at = Vec::with_capacity(points.len());
        let mut total = 0.0;
        for (i, p) in points.iter().enumerate() {
            if i > 0 {
                total += (p.x - points[i - 1].x).hypot(p.z - points[i - 1].z);
            }
            at.push(total);
        }
        Self { points, at }
    }

    pub(crate) fn length(&self) -> f32 {
        self.at.last().copied().unwrap_or(0.0)
    }

    /// Where she is `s` metres along (her floor, or the stair) and which way she is walking.
    pub(crate) fn sample(&self, s: f32) -> (Walker, f32) {
        let p = plan();
        let Some(first) = self.points.first() else {
            return (
                Walker {
                    x: p.spawn.x,
                    z: p.spawn.z,
                    level: 0,
                    on_stair: false,
                },
                0.0,
            );
        };
        if self.points.len() == 1 {
            let w = Walker {
                x: first.x,
                z: first.z,
                level: first.level,
                on_stair: false,
            };
            return (w, 0.0);
        }
        let s = s.clamp(0.0, self.length());
        let i = self
            .at
            .windows(2)
            .position(|w| s <= w[1])
            .unwrap_or(self.points.len() - 2);
        let (a, b) = (self.points[i], self.points[i + 1]);
        let span = (self.at[i + 1] - self.at[i]).max(1e-5);
        let t = ((s - self.at[i]) / span).clamp(0.0, 1.0);
        let (x, z) = (a.x + (b.x - a.x) * t, a.z + (b.z - a.z) * t);
        let heading = (b.x - a.x).atan2(b.z - a.z);
        let walker = if a.level == b.level {
            Walker {
                x,
                z,
                level: a.level,
                on_stair: false,
            }
        } else if x < p.stair.x1 + RADIUS && x > p.stair.x0 - RADIUS {
            Walker {
                x,
                z,
                level: (p.stair_t(x) > 0.5) as u8,
                on_stair: true,
            }
        } else {
            Walker {
                x,
                z,
                level: (x < p.stair.x0) as u8,
                on_stair: false,
            }
        };
        (walker, heading)
    }

    /// The way of whoever follows this route from `start` (beside its first point, on the
    /// ground floor) and ends at `end` instead of its last point: the same way, then from its
    /// last turn a way of her own to `end`.
    fn followed_from(&self, start: (f32, f32), end: (f32, f32)) -> Route {
        let p = plan();
        let mut points = vec![Waypoint {
            x: start.0,
            z: start.1,
            level: 0,
        }];
        let n = self.points.len();
        if n < 2 {
            points.push(Waypoint {
                x: end.0,
                z: end.1,
                level: self.points.last().map_or(0, |w| w.level),
            });
            return Route::new(points);
        }
        points.extend_from_slice(&self.points[..n - 1]);
        let turn = self.points[n - 2];
        match p.find_path(turn.level, (turn.x, turn.z), end) {
            Some(tail) => points.extend(tail.into_iter().skip(1).map(|(x, z)| Waypoint {
                x,
                z,
                level: turn.level,
            })),
            None => points.push(Waypoint {
                x: end.0,
                z: end.1,
                level: turn.level,
            }),
        }
        Route::new(points)
    }
}

// ---------------------------------------------------------------- the escort
/// How fast the escort walks (m/s): an unhurried pace; the Queen's walk is slowed to match.
pub(crate) const ESCORT_SPEED: f32 = 1.7;
/// How far behind her guide the Queen keeps (m, along the way).
const ESCORT_GAP: f32 = 1.05;
/// The greeting: how long they stand face to face before going up (s).
const GREET: f32 = 1.4;

/// Where the girl and the Queen end in each free room, in front of the bed: (girl, queen).
fn room_spots(room: usize) -> ((f32, f32), (f32, f32)) {
    match room {
        0 => ((-4.75, -2.6), (-5.7, -2.5)),
        _ => ((0.6, -2.6), (-0.35, -2.5)),
    }
}

/// How a girl looks this frame when she is not simply at her place (where she is, which way
/// she faces, what she is doing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GirlLook {
    pub npc: usize,
    pub at: Vec3,
    pub yaw: f32,
    pub mode: GirlMode,
    /// Her speed while walking (m/s): her steps are paced to it.
    pub speed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GirlMode {
    /// At her place, as the prototype recorded her (at the bar, on her sofa).
    Idle,
    Walk,
    Stand,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EscortPhase {
    /// She gets up and comes to the Queen.
    Approach,
    /// Face to face: she says her word.
    Greet,
    /// Up the stair and along the corridor, the Queen a step behind.
    Lead,
    /// In the room.
    Arrived,
}

/// A girl taking the Queen up to her room.
#[derive(Clone, Debug)]
pub(crate) struct Escort {
    /// The girl: her index among the plan's people, and her id.
    pub npc: usize,
    pub id: &'static str,
    phase: EscortPhase,
    phase_at: f32,
    clock: f32,
    /// Hers to the Queen; then hers up to the room; and the Queen's, from where she stands.
    approach: Route,
    lead: Route,
    queen_route: Route,
    girl_s: f32,
    queen_s: f32,
    /// How far along her route the Queen's place behind the girl starts.
    queen_offset: f32,
    pub girl: GirlLook,
    queen_yaw: f32,
    /// The Queen stepped this frame (her walk plays).
    pub queen_moving: bool,
    /// What the girl says when she reaches her.
    line: &'static str,
    /// Reported to the host (who opens the girl's guion in the room) once.
    reported: bool,
}

/// The way an angle turns toward another by at most `rate` radians.
fn turn_toward(from: f32, to: f32, rate: f32) -> f32 {
    let d =
        (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + d.clamp(-rate, rate)
}

impl State {
    /// `id` (one of the house's girls) takes her up to `room`; `line` is her word at the desk.
    /// False when she is not in the house or there is no way (the menu then opens her guion).
    pub(crate) fn start_escort(&mut self, id: &str, room: usize, line: &'static str) -> bool {
        let p = plan();
        let Some((npc, girl)) = p.npcs.iter().enumerate().find(|(_, n)| n.id == id) else {
            return false;
        };
        if self.walker.level != 0 || self.walker.on_stair {
            return false;
        }
        let queen = (self.walker.x, self.walker.z);
        let (girl_end, queen_end) = room_spots(room);
        let Some(lead_from_queen) = p.way_up(queen, girl_end) else {
            return false;
        };
        // Where she meets the Queen: a free step beside her, on the way to the stair.
        let ahead = lead_from_queen.sample(0.9).0;
        let mut meet = (ahead.x, ahead.z);
        if p.blocked(meet.0, meet.1, 0) || !p.clear_line(0, queen, meet) {
            meet = (0..16)
                .map(|k| {
                    let a = k as f32 * std::f32::consts::TAU / 16.0;
                    (queen.0 + a.sin() * 0.9, queen.1 + a.cos() * 0.9)
                })
                .find(|&m| !p.blocked(m.0, m.1, 0) && p.clear_line(0, queen, m))
                .unwrap_or(queen);
        }
        let seat = (girl.x, girl.z);
        let Some(to_queen) = p.find_path(0, seat, meet) else {
            return false;
        };
        let Some(lead) = p.way_up(meet, girl_end) else {
            return false;
        };
        let approach = Route::new(
            to_queen
                .into_iter()
                .map(|(x, z)| Waypoint { x, z, level: 0 })
                .collect(),
        );
        let queen_route = lead.followed_from(queen, queen_end);
        let queen_offset = (meet.0 - queen.0).hypot(meet.1 - queen.1);
        self.talk = None;
        self.escort = Some(Escort {
            npc,
            id: p.npcs[npc].id.as_str(),
            phase: EscortPhase::Approach,
            phase_at: 0.0,
            clock: 0.0,
            approach,
            lead,
            queen_route,
            girl_s: 0.0,
            queen_s: 0.0,
            queen_offset,
            girl: GirlLook {
                npc,
                at: Vec3::new(girl.x, 0.0, girl.z),
                yaw: girl.yaw,
                mode: GirlMode::Idle,
                speed: 0.0,
            },
            queen_yaw: 0.0,
            queen_moving: false,
            line,
            reported: false,
        });
        self.staying = None;
        true
    }

    /// One step of the escort. Moves the Queen (her walker) and the girl; `queen_yaw` is the
    /// Queen's facing now. Returns the girl's id once, when they are in the room.
    pub(crate) fn tick_escort(&mut self, dt: f32, queen_yaw: &mut f32) -> Option<&'static str> {
        let p = plan();
        let walker_now = self.walker;
        let escort = self.escort.as_mut()?;
        escort.clock += dt;
        let since = escort.clock - escort.phase_at;
        escort.queen_yaw = *queen_yaw;
        let before = (walker_now.x, walker_now.z);
        let mut queen = walker_now;
        let seat = &p.npcs[escort.npc];
        match escort.phase {
            EscortPhase::Approach => {
                // Rising from her seat: slow for a moment, then her pace.
                let ease = (since / 0.7).clamp(0.0, 1.0);
                let speed = ESCORT_SPEED * (0.35 + 0.65 * ease * ease * (3.0 - 2.0 * ease));
                escort.girl_s = (escort.girl_s + speed * dt).min(escort.approach.length());
                let (w, heading) = escort.approach.sample(escort.girl_s);
                escort.girl.at = Vec3::new(w.x, p.floor_y(w), w.z);
                escort.girl.yaw = if since < 0.12 {
                    turn_toward(seat.yaw, heading, 6.0 * dt)
                } else {
                    turn_toward(escort.girl.yaw, heading, 9.0 * dt)
                };
                escort.girl.mode = GirlMode::Walk;
                escort.girl.speed = speed;
                // The Queen turns to watch her come.
                let to_girl = (escort.girl.at.x - queen.x).atan2(escort.girl.at.z - queen.z);
                escort.queen_yaw = turn_toward(escort.queen_yaw, to_girl, 4.0 * dt);
                if escort.girl_s >= escort.approach.length() - 1e-3 {
                    escort.phase = EscortPhase::Greet;
                    escort.phase_at = escort.clock;
                }
            }
            EscortPhase::Greet => {
                let to_queen = (queen.x - escort.girl.at.x).atan2(queen.z - escort.girl.at.z);
                escort.girl.yaw = turn_toward(escort.girl.yaw, to_queen, 7.0 * dt);
                escort.girl.mode = GirlMode::Stand;
                escort.girl.speed = 0.0;
                let to_girl = (escort.girl.at.x - queen.x).atan2(escort.girl.at.z - queen.z);
                escort.queen_yaw = turn_toward(escort.queen_yaw, to_girl, 7.0 * dt);
                if since >= GREET {
                    escort.phase = EscortPhase::Lead;
                    escort.phase_at = escort.clock;
                    escort.girl_s = 0.0;
                }
            }
            EscortPhase::Lead => {
                let lead_len = escort.lead.length();
                escort.girl_s = (escort.girl_s + ESCORT_SPEED * dt).min(lead_len);
                let girl_done = escort.girl_s >= lead_len - 1e-3;
                let (w, heading) = escort.lead.sample(escort.girl_s);
                escort.girl.at = Vec3::new(w.x, p.floor_y(w), w.z);
                let queen_len = escort.queen_route.length();
                let target = if escort.girl_s < lead_len - 1e-3 {
                    (escort.girl_s + escort.queen_offset - ESCORT_GAP).clamp(0.0, queen_len)
                } else {
                    queen_len
                };
                if escort.queen_s < target {
                    escort.queen_s = (escort.queen_s + ESCORT_SPEED * 1.1 * dt).min(target);
                }
                let (qw, qheading) = escort.queen_route.sample(escort.queen_s);
                queen = qw;
                let queen_done = escort.queen_s >= queen_len - 1e-3;
                if girl_done {
                    // Waiting for her by the bed, she turns to her.
                    let to_queen = (queen.x - escort.girl.at.x).atan2(queen.z - escort.girl.at.z);
                    escort.girl.yaw = turn_toward(escort.girl.yaw, to_queen, 6.0 * dt);
                    escort.girl.mode = GirlMode::Stand;
                    escort.girl.speed = 0.0;
                } else {
                    escort.girl.yaw = turn_toward(escort.girl.yaw, heading, 9.0 * dt);
                    escort.girl.mode = GirlMode::Walk;
                    escort.girl.speed = ESCORT_SPEED;
                }
                let moved = (queen.x - before.0).hypot(queen.z - before.1);
                if moved > 1e-4 {
                    escort.queen_yaw = turn_toward(escort.queen_yaw, qheading, 9.0 * dt);
                }
                if girl_done && queen_done {
                    escort.phase = EscortPhase::Arrived;
                    escort.phase_at = escort.clock;
                }
            }
            EscortPhase::Arrived => {
                let to_girl = (escort.girl.at.x - queen.x).atan2(escort.girl.at.z - queen.z);
                escort.queen_yaw = turn_toward(escort.queen_yaw, to_girl, 7.0 * dt);
                let to_queen = (queen.x - escort.girl.at.x).atan2(queen.z - escort.girl.at.z);
                escort.girl.yaw = turn_toward(escort.girl.yaw, to_queen, 7.0 * dt);
                escort.girl.mode = GirlMode::Stand;
            }
        }
        escort.queen_moving = (queen.x - before.0).hypot(queen.z - before.1) > 1e-4;
        *queen_yaw = escort.queen_yaw;
        let report = (escort.phase == EscortPhase::Arrived
            && escort.clock - escort.phase_at > 0.35
            && !escort.reported)
            .then(|| {
                escort.reported = true;
                escort.id
            });
        self.walker = queen;
        if report.is_some() {
            // From here she simply stays in her room with the Queen.
            if let Some(escort) = self.escort.take() {
                self.staying = Some(GirlLook {
                    mode: GirlMode::Stand,
                    speed: 0.0,
                    ..escort.girl
                });
            }
        }
        report
    }

    /// Esc during the escort: they are in the room at once.
    pub(crate) fn skip_escort(&mut self, queen_yaw: &mut f32) {
        let p = plan();
        let Some(escort) = self.escort.as_mut() else {
            return;
        };
        let (gw, _) = escort.lead.sample(escort.lead.length());
        let (qw, _) = escort.queen_route.sample(escort.queen_route.length());
        escort.girl_s = escort.lead.length();
        escort.queen_s = escort.queen_route.length();
        escort.girl.at = Vec3::new(gw.x, p.floor_y(gw), gw.z);
        escort.girl.yaw = (qw.x - gw.x).atan2(qw.z - gw.z);
        escort.girl.mode = GirlMode::Stand;
        escort.queen_yaw = (gw.x - qw.x).atan2(gw.z - qw.z);
        *queen_yaw = escort.queen_yaw;
        escort.phase = EscortPhase::Arrived;
        escort.phase_at = escort.clock - 1.0;
        self.walker = qw;
    }

    /// What the escort is doing (for the visual QA): `approach`, `greet`, `lead`, `arrived`.
    pub(crate) fn escort_phase(&self) -> Option<&'static str> {
        self.escort.as_ref().map(|e| match e.phase {
            EscortPhase::Approach => "approach",
            EscortPhase::Greet => "greet",
            EscortPhase::Lead => "lead",
            EscortPhase::Arrived => "arrived",
        })
    }

    /// The girls not simply at their places this frame.
    pub(crate) fn girl_looks(&self) -> Vec<GirlLook> {
        self.escort
            .as_ref()
            .map(|e| e.girl)
            .into_iter()
            .chain(self.staying)
            .collect()
    }

    /// While the girl greets her (and a moment after), her word over her head.
    fn escort_bubble(&self, camera: Camera3D) -> Option<map_overlay::NpcSpeechBubble> {
        let escort = self.escort.as_ref()?;
        let shown = match escort.phase {
            EscortPhase::Greet => true,
            EscortPhase::Lead => escort.clock - escort.phase_at < 1.6,
            _ => false,
        };
        if !shown {
            return None;
        }
        let (x, y) = project_npc_anchor(camera, escort.girl.at + Vec3::new(0.0, 2.25, 0.0))?;
        let speaker: &'static str = match escort.id {
            "nina" => "Nina",
            "alba" => "Alba",
            "noa" => "Noa",
            other => other,
        };
        Some(map_overlay::NpcSpeechBubble {
            x,
            y,
            speaker,
            text: escort.line,
            opacity: 1.0,
            accent: [0.90, 0.30, 0.46, 1.0],
        })
    }

    /// The camera's anchor while a girl comes for her: between the two, so she is seen coming.
    pub(crate) fn escort_focus(&self, queen: Vec3) -> Vec3 {
        match &self.escort {
            Some(e) if e.phase == EscortPhase::Approach || e.phase == EscortPhase::Greet => {
                let d = e.girl.at - queen;
                let len = d.length();
                if len < 1e-3 {
                    queen
                } else {
                    queen + d * (len.min(7.0) * 0.5 / len)
                }
            }
            _ => queen,
        }
    }
}

// ------------------------------------------------------------------- map
/// The icon a hotspot shows (on the map and in the F prompt).
pub(crate) fn icon_for(h: &Hotspot) -> Icon {
    match h.icon.as_str() {
        "door" => Icon::Leave,
        "bed" => {
            if h.locked {
                Icon::Lock
            } else {
                Icon::Rest
            }
        }
        "beer" => Icon::Tavern,
        "alert" => Icon::Memory,
        "sofa" => Icon::Rest,
        "stairs" => Icon::Door,
        _ => Icon::Talk,
    }
}

/// What the visit asks for next: speak to Rosalía, then climb, then pick a free room.
pub(crate) fn goals(level: u8, rope_open: bool) -> Vec<&'static Hotspot> {
    let p = plan();
    let ids: &[&str] = if !rope_open {
        &["recepcionista"]
    } else if level == 0 {
        &["escalera"]
    } else {
        &["hab1", "hab2"]
    };
    p.hotspots
        .iter()
        .filter(|h| ids.contains(&h.id.as_str()))
        .collect()
}

/// The red runner laid from the street door to the far end of the corridor, per floor
/// (the same path the prototype lays the carpet on), as world x/z points.
pub(crate) fn runner(level: u8) -> Vec<(f32, f32)> {
    let p = plan();
    let lane = (p.stair.z0 + p.stair.z1) * 0.5;
    if level == 0 {
        let zc = (p.door.z0 + p.door.z1) * 0.5;
        vec![
            (p.door.x + 0.2, zc),
            (0.75, zc),
            (0.75, lane),
            (p.stair.x1, lane),
            (p.stair.x0, lane),
        ]
    } else {
        let hall = (p.fronts_z + p.gallery_z) * 0.5 - 0.15;
        vec![
            (p.stair.x0, lane),
            (-6.8, lane),
            (-6.8, hall),
            (p.bounds.x1 - 0.55, hall),
        ]
    }
}

// ------------------------------------------------------------------ sound
/// The brothel's placed sounds, from its plan: the fire in the grate, glasses and
/// wine at the bar, Rosalía's pen at the desk, a laugh on the salon's sofa, and
/// behind the locked door upstairs, a couple who are not to be disturbed.
pub(crate) fn emitters() -> &'static [game_audio::Emitter] {
    static EMITTERS: std::sync::OnceLock<Vec<game_audio::Emitter>> = std::sync::OnceLock::new();
    EMITTERS.get_or_init(|| {
        use crate::exploration3d::game_audio::ambience_amb as Amb;
        use game_audio::Emitter;
        let p = plan();
        let mut out = Vec::new();
        let npc = |id: &str| {
            p.npcs
                .iter()
                .find(|n| n.id == id)
                .map(|n| Vec3::new(n.x, n.level as f32 * p.level_height + 1.0, n.z))
        };
        if let Some(fire) = p.lamps.iter().find(|l| l.fire) {
            out.push(Emitter {
                amb: Amb::FireCrackle,
                at: Vec3::new(fire.x, fire.y, fire.z),
                every: (0.5, 1.6),
                volume: 0.5,
                radius: 3.0,
                level: 0,
            });
        }
        if let Some(at) = npc("nina") {
            out.push(Emitter {
                amb: Amb::GlassClink,
                at,
                every: (9.0, 20.0),
                volume: 0.45,
                radius: 3.5,
                level: 0,
            });
            out.push(Emitter {
                amb: Amb::Pour,
                at,
                every: (16.0, 34.0),
                volume: 0.4,
                radius: 3.0,
                level: 0,
            });
        }
        if let Some(at) = npc("rosalia") {
            out.push(Emitter {
                amb: Amb::Quill,
                at,
                every: (7.0, 16.0),
                volume: 0.35,
                radius: 2.5,
                level: 0,
            });
        }
        if let Some(at) = npc("alba") {
            out.push(Emitter {
                amb: Amb::Giggle,
                at,
                every: (18.0, 40.0),
                volume: 0.45,
                radius: 4.0,
                level: 0,
            });
            out.push(Emitter {
                amb: Amb::GlassClink,
                at,
                every: (24.0, 50.0),
                volume: 0.3,
                radius: 3.0,
                level: 0,
            });
        }
        if let Some(room) = p.rooms.iter().find(|r| r.locked) {
            let at = Vec3::new(
                (room.x0 + room.x1) * 0.5,
                p.level_height + 1.0,
                (room.z0 + room.z1) * 0.5,
            );
            out.push(Emitter {
                amb: Amb::Moan,
                at,
                every: (6.0, 13.0),
                volume: 0.55,
                radius: 4.5,
                level: 1,
            });
            out.push(Emitter {
                amb: Amb::MoanRise,
                at,
                every: (11.0, 22.0),
                volume: 0.55,
                radius: 4.5,
                level: 1,
            });
            out.push(Emitter {
                amb: Amb::Sigh,
                at,
                every: (15.0, 30.0),
                volume: 0.45,
                radius: 4.0,
                level: 1,
            });
            out.push(Emitter {
                amb: Amb::BedCreak,
                at,
                every: (9.0, 18.0),
                volume: 0.4,
                radius: 4.0,
                level: 1,
            });
        }
        out
    })
}

// ------------------------------------------------------------------ state
/// A conversation in progress: whose lines, which one, and where they are said.
#[derive(Clone, Debug)]
pub(crate) struct Talk {
    pub dialogue: &'static str,
    pub index: usize,
    pub at: Vec3,
}

/// What pressing F did, for the explorer to carry out.
pub(crate) enum Action {
    None,
    Leave,
    OpenVenue,
}

/// The brothel as the Queen experiences it: where she is, what has opened.
#[derive(Clone, Debug)]
pub(crate) struct State {
    pub walker: Walker,
    /// Rosalía has taken the rope off the stair.
    pub rope_open: bool,
    pub talk: Option<Talk>,
    pub door_angle: [f32; 3],
    pub door_shake: [f32; 3],
    /// Where to put her back in the villa when she leaves.
    pub return_to: Vec3,
    pub return_yaw: f32,
    /// A girl taking her up to a room.
    pub escort: Option<Escort>,
    /// The girl who brought her up, staying in her room until the Queen goes down.
    pub staying: Option<GirlLook>,
}

impl State {
    pub(crate) fn enter(return_to: Vec3, return_yaw: f32, rope_open: bool) -> Self {
        let p = plan();
        Self {
            // A few steps in from the street door: in the light, clear of its portal and of
            // the way back out.
            walker: Walker {
                x: p.spawn.x - 1.2,
                z: p.spawn.z,
                level: 0,
                on_stair: false,
            },
            rope_open,
            talk: None,
            door_angle: [0.0; 3],
            door_shake: [0.0; 3],
            return_to,
            return_yaw,
            escort: None,
            staying: None,
        }
    }

    /// Her position in the world (with the floor's height).
    pub(crate) fn position(&self) -> Vec3 {
        let p = plan();
        Vec3::new(self.walker.x, p.floor_y(self.walker), self.walker.z)
    }

    pub(crate) fn walk(&mut self, dx: f32, dz: f32) {
        self.walker = plan().slide(self.walker, dx, dz, self.rope_open);
        // Walking away ends a conversation.
        if let Some(talk) = &self.talk {
            let d = Vec3::new(self.walker.x - talk.at.x, 0.0, self.walker.z - talk.at.z).length();
            if d > 3.2 {
                self.talk = None;
            }
        }
    }

    pub(crate) fn update(&mut self, dt: f32) {
        let p = plan();
        // Back down, the girl who brought her up is at her place again.
        if self.escort.is_none() && self.walker.level == 0 && !self.walker.on_stair {
            self.staying = None;
        }
        let upstairs = self.walker.level == 1 && !self.walker.on_stair;
        let in_room = p.room_at(self.walker).map(|r| r.index);
        for hinge in &p.doors {
            let i = hinge.room.min(2);
            let dx = self.walker.x - (hinge.x + 0.72);
            let dz = self.walker.z - (p.fronts_z + 0.9);
            let near = upstairs && (dx * dx + dz * dz).sqrt() < 1.7;
            let target = if !hinge.locked && (near || in_room == Some(hinge.room)) {
                hinge.max
            } else {
                0.0
            };
            self.door_angle[i] += (target - self.door_angle[i]) * (dt * 7.0).min(1.0);
            self.door_shake[i] = (self.door_shake[i] - dt * 2.4).max(0.0);
        }
    }

    /// The hotspot within reach on her floor, if any.
    pub(crate) fn nearest(&self) -> Option<&'static Hotspot> {
        let p = plan();
        if self.walker.on_stair {
            return None;
        }
        p.hotspots
            .iter()
            .filter(|h| h.level == self.walker.level)
            .map(|h| {
                (
                    h,
                    ((h.x - self.walker.x).powi(2) + (h.z - self.walker.z).powi(2)).sqrt(),
                )
            })
            .filter(|(h, d)| *d < h.radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    /// The F prompt for what is within reach.
    pub(crate) fn prompt(&self) -> Option<Prompt> {
        if let Some(talk) = &self.talk {
            let lines = &plan().dialogues[talk.dialogue];
            let more = talk.index + 1 < lines.len();
            return Some(
                Prompt::new(Icon::Talk, if more { "Continuar" } else { "Terminar" })
                    .detail(lines[talk.index].who.as_str()),
            );
        }
        let h = self.nearest()?;
        let icon = icon_for(h);
        let detail = if h.exit {
            "Volver a la villa".to_owned()
        } else if h.stairs {
            if self.rope_open {
                "El cordón está quitado".to_owned()
            } else {
                "Hay un cordón: habla antes con Rosalía".to_owned()
            }
        } else if h.locked {
            "Cerrada".to_owned()
        } else {
            h.label.clone()
        };
        Some(Prompt::new(icon, h.prompt.clone()).detail(detail))
    }

    /// F: talk, advance a conversation, open Rosalía's book, or leave.
    pub(crate) fn interact(&mut self) -> Action {
        let p = plan();
        if let Some(talk) = &mut self.talk {
            talk.index += 1;
            if talk.index >= p.dialogues[talk.dialogue].len() {
                self.talk = None;
            }
            return Action::None;
        }
        let Some(h) = self.nearest() else {
            return Action::None;
        };
        if h.exit {
            return Action::Leave;
        }
        if h.id == "recepcionista" {
            // Her book of rooms, company and work is the game's own venue menu; meeting her
            // lifts the rope off the stair.
            self.rope_open = true;
            return Action::OpenVenue;
        }
        if h.stairs && self.rope_open {
            return Action::None;
        }
        if h.locked {
            if let Some(room) = h.room {
                self.door_shake[room.min(2)] = 1.0;
            }
        }
        if let Some(key) = h.dialogue.as_deref() {
            if let Some((name, _)) = p.dialogues.get_key_value(key) {
                self.talk = Some(Talk {
                    dialogue: name.as_str(),
                    index: 0,
                    at: p.anchor(h),
                });
            }
        }
        Action::None
    }

    /// The line being said, as a speech bubble over whoever says it.
    pub(crate) fn bubbles(&self, camera: Camera3D) -> Vec<map_overlay::NpcSpeechBubble> {
        if let Some(bubble) = self.escort_bubble(camera) {
            return vec![bubble];
        }
        let Some(talk) = &self.talk else {
            return Vec::new();
        };
        let line = &plan().dialogues[talk.dialogue][talk.index];
        let Some((x, y)) = project_npc_anchor(camera, talk.at + Vec3::new(0.0, 3.1, 0.0)) else {
            return Vec::new();
        };
        vec![map_overlay::NpcSpeechBubble {
            x,
            y,
            speaker: line.who.as_str(),
            text: line.text.as_str(),
            opacity: 1.0,
            accent: [0.90, 0.30, 0.46, 1.0],
        }]
    }

    /// The point lights of the floor on screen: its own key lamps, then those of the other
    /// floor seen from it (the desk lamp, the stair's newel and the landing), fire flickering.
    pub(crate) fn lights(&self, elapsed: f32) -> Vec<PointLight3D> {
        let p = plan();
        let level = p.view_level(self.walker);
        let mut out: Vec<PointLight3D> = p
            .lamps
            .iter()
            .filter(|l| l.level == level)
            .chain(p.lamps.iter().filter(|l| l.level != level && l.both))
            .enumerate()
            .map(|(i, l)| {
                let flicker = if l.fire {
                    // The same breath as the flames themselves.
                    ToonFire3D::strength(elapsed)
                } else {
                    1.0 + (elapsed * 3.0 + i as f32 * 1.7).sin() * 0.04
                };
                let (color, intensity, radius) = if l.fire {
                    ([1.0, 0.42, 0.14], 12.0, 7.5)
                } else {
                    ([1.0, 0.56, 0.24], 7.2 * l.power, 6.4)
                };
                PointLight3D::new(Vec3::new(l.x, l.y, l.z), color, intensity * flicker, radius)
            })
            .collect();
        out.truncate(14);
        out
    }
}

/// The house's finish: the renderer's plain one, with its lamplight bounced off the velvet,
/// the wood and the walls onto what is round them.
pub(crate) fn finish() -> Finish3D {
    Finish3D {
        gi: 2.4,
        gi_radius: 3.0,
        gi_ambient: 0.7,
        ..Finish3D::default()
    }
}

/// Night indoors by lamplight: a low warm fill, a soft key from above, no fog.
pub(crate) fn mood(mut env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    let lighting = &mut env.lighting;
    lighting.key_color = [0.46, 0.36, 0.28];
    lighting.ambient_color = [0.21, 0.145, 0.12];
    lighting.bounce_color = [0.26, 0.12, 0.08];
    let fog = &mut env.atmosphere;
    fog.fog_color = [0.03, 0.018, 0.02];
    fog.fog_start = 40.0;
    fog.fog_end = 90.0;
    fog.fog_density = 0.0;
    fog.height_falloff = 0.0;
    env.clear_color = [0.016, 0.008, 0.012, 1.0];
    env
}

// ------------------------------------------------------------------ scene
struct Part {
    name: String,
    mesh: MeshHandle,
    transform: Mat4,
    material: Material3D,
    texture: Option<TextureHandle>,
}

/// How much a material glows by itself (flames, lamp glass, the night in the windows).
fn emission_for(name: &str) -> f32 {
    match name {
        "flame" | "candleFlame" => 2.2,
        "ember" => 1.6,
        "lampGlass" => 1.4,
        "nightGlass" => 0.35,
        _ => 0.0,
    }
}

fn load_parts(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    file: &str,
) -> Result<Vec<Part>> {
    let asset = upload_static_asset(renderer, device, queue, &asset_root().join(file), cache)?;
    Ok(asset
        .parts
        .into_iter()
        .zip(asset.part_names)
        .map(|(part, name)| {
            let glow = emission_for(&name);
            let mut material = part.material;
            if glow > 0.0 {
                material = material.with_emission(glow);
            } else {
                material = material.with_ambient_occlusion(0.18);
            }
            Part {
                name,
                mesh: part.mesh,
                transform: part.local_transform,
                material,
                texture: part.texture,
            }
        })
        .collect())
}

struct Girl {
    /// Her index among the plan's people.
    npc: usize,
    model: GltfModel,
    meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
    transforms: Vec<Mat4>,
    /// At her place (recorded by the prototype), walking, standing.
    idle: Option<usize>,
    walk: Option<usize>,
    stand: Option<usize>,
    /// How fast her walk carries her as authored (m/s): her steps are paced to her speed.
    stride: f32,
    /// Where she is when she is at her place.
    home: Mat4,
    /// What she holds there (mesh, and where it is left when she goes: on the counter).
    parked: Vec<(usize, Mat4)>,
    animator: Option<velvet_render::Animator3D>,
    last: f32,
}

/// Where the ankle sits over the floor when she stands (m, the girls' scale included).
const ANKLE: f32 = 0.095;

/// The name of a rig's top node (the one the prototype's seat offset is on).
fn root_name(model: &GltfModel) -> Option<String> {
    let nodes = model.rest_nodes();
    let mut i = nodes.iter().position(|(n, ..)| n == "DEF-pelvis")?;
    while let Some(parent) = nodes[i].1 {
        i = parent;
    }
    Some(nodes[i].0.clone())
}

/// A clip from another rig onto a girl's, as `name`: its turns retargeted, and her root lifted
/// frame by frame so the lower ankle is on the floor (no seat offset, no feet in the boards).
fn add_girl_clip(
    model: &mut GltfModel,
    source: &GltfModel,
    clip: usize,
    name: &str,
    map: &[(&str, &str, Option<(&str, &str)>)],
) -> Option<usize> {
    const FRAMES: usize = 48;
    let root = root_name(model)?;
    let (duration, turns) = retarget::retarget_tracks(model, source, clip, map, FRAMES);
    let flat = vec![(
        root.clone(),
        vec![(0.0, Vec3::ZERO), (duration, Vec3::ZERO)],
    )];
    let index = model
        .add_animation_clip(name, duration, turns.clone(), flat)
        .ok()?;
    let ankle = |model: &GltfModel, pose: &velvet_render::Pose3D| {
        ["DEF-footL", "DEF-footR"]
            .iter()
            .filter_map(|b| model.pose_node_transform(pose, b))
            .map(|m| m.w_axis[1])
            .fold(f32::INFINITY, f32::min)
    };
    let lift: Vec<(f32, Vec3)> = (0..=FRAMES)
        .map(|f| {
            let t = duration * f as f32 / FRAMES as f32;
            let low = model
                .pose(index, t.min(duration - 1e-4))
                .map(|pose| ankle(model, &pose))
                .filter(|y| y.is_finite())
                .unwrap_or(ANKLE);
            (t, Vec3::new(0.0, ANKLE - low, 0.0))
        })
        .collect();
    model
        .add_animation_clip(name, duration, turns, vec![(root, lift)])
        .ok()
}

/// How fast a walk clip carries her (m/s): two steps a cycle, each as long as her feet are
/// apart at the widest.
fn stride_speed(model: &GltfModel, clip: usize) -> f32 {
    let duration = model
        .animation_clips
        .get(clip)
        .map(|c| c.duration)
        .unwrap_or(1.0)
        .max(0.1);
    let step = (0..32)
        .filter_map(|f| {
            let pose = model.pose(clip, duration * f as f32 / 32.0)?;
            let l = model.pose_node_transform(&pose, "DEF-footL")?.w_axis;
            let r = model.pose_node_transform(&pose, "DEF-footR")?.w_axis;
            Some((l[0] - r[0]).hypot(l[2] - r[2]))
        })
        .fold(0.0, f32::max);
    if step > 0.05 {
        2.0 * step / duration
    } else {
        1.2
    }
}

/// The girls' borrowed clips, on the castle maids' Mixamo rig: their walk, and a calm stand made
/// from the rig's rest ([`retarget::add_mixamo_stand`]).
struct ClipSources {
    rig: Option<GltfModel>,
    walk: Option<usize>,
    stand: Option<usize>,
}

impl ClipSources {
    fn load() -> Self {
        let path = runtime_3d_root().join("characters/maid_walking4.glb");
        let Ok(mut rig) = GltfModel::load(&path) else {
            eprintln!("[3d] brothel girls: no walking rig at {}", path.display());
            return Self {
                rig: None,
                walk: None,
                stand: None,
            };
        };
        let walk = rig.animation_index("Maid_Walk");
        let stand = retarget::add_mixamo_stand(&mut rig, "Girl_Stand_Source");
        Self {
            rig: Some(rig),
            walk,
            stand,
        }
    }

    /// `clip` of the rig onto a girl as `name`.
    fn onto(&self, model: &mut GltfModel, clip: Option<usize>, name: &str) -> Option<usize> {
        add_girl_clip(
            model,
            self.rig.as_ref()?,
            clip?,
            name,
            &retarget::DEF_FROM_MIXAMO,
        )
    }
}

/// The bar counter's top (m): what a girl puts down there when she leaves it.
const COUNTER_TOP: f32 = 1.02;

/// Where the things a girl holds at her place (Nina's tray and its cup) are left when she goes:
/// on the counter in front of her if there is one (else where they were).
fn parked_props(
    model: &GltfModel,
    idle: Option<usize>,
    home: Mat4,
    npc: &Npc,
) -> Vec<(usize, Mat4)> {
    let props: Vec<usize> = model
        .meshes
        .iter()
        .enumerate()
        .filter(|(_, m)| m.skinning.is_none())
        .map(|(i, _)| i)
        .collect();
    if props.is_empty() {
        return Vec::new();
    }
    let pose = idle
        .and_then(|c| model.pose(c, 0.0))
        .unwrap_or_else(|| model.rest_pose());
    let held = model.pose_mesh_transforms(&pose);
    let worlds: Vec<(usize, Mat4)> = props.iter().map(|&i| (i, home * held[i])).collect();
    let Some(&(_, first)) = worlds.first() else {
        return worlds;
    };
    let p = plan();
    let (fx, fz) = (npc.yaw.sin(), npc.yaw.cos());
    let counter = p.boxes.iter().find(|b| {
        let (x, z) = (npc.x + fx * 0.8, npc.z + fz * 0.8);
        b.level == npc.level && (x - b.x).abs() < b.w / 2.0 && (z - b.z).abs() < b.d / 2.0
    });
    let Some(counter) = counter else {
        return worlds;
    };
    // On the counter's near edge, in front of where she held it, flat on its top.
    let near_z = counter.z - fz * (counter.d / 2.0 - 0.16);
    let near_x = first.w_axis[0].clamp(
        counter.x - counter.w / 2.0 + 0.2,
        counter.x + counter.w / 2.0 - 0.2,
    );
    let target = Vec3::new(
        if fx.abs() > 0.5 {
            counter.x - fx * (counter.w / 2.0 - 0.16)
        } else {
            near_x
        },
        npc.level as f32 * p.level_height + COUNTER_TOP + 0.012,
        if fz.abs() > 0.5 {
            near_z
        } else {
            first.w_axis[2]
        },
    );
    let delta = target - Vec3::new(first.w_axis[0], first.w_axis[1], first.w_axis[2]);
    worlds
        .into_iter()
        .map(|(i, m)| (i, Mat4::from_translation(delta) * m))
        .collect()
}

/// The brothel's GPU side: every exported part, the door leaf and the girls.
pub(crate) struct Scene {
    ground: Vec<Part>,
    fire: Vec<Part>,
    rope_shut: Vec<Part>,
    rope_open: Vec<Part>,
    upper: Vec<Part>,
    roofs: Vec<Part>,
    part_tall: Vec<Part>,
    part_low: Vec<Part>,
    fronts: Vec<Vec<Part>>,
    door_leaf: Vec<Part>,
    girls: Vec<Girl>,
    /// The animated fire in the grate (the exported logs stay; its cones and coals give way).
    flames: Option<ToonFire3D>,
}

impl Scene {
    pub(crate) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self> {
        let started = Instant::now();
        super::prefetch::prefetch(super::zone_files(&asset_root()));
        let mut cache = HashMap::new();
        let mut parts = |file: &str| load_parts(renderer, device, queue, &mut cache, file);
        let ground = parts("brothel_ground.glb")?;
        let fire: Vec<Part> = parts("brothel_fire.glb")?
            .into_iter()
            .filter(|p| p.name != "flame" && p.name != "ember")
            .collect();
        let rope_shut = parts("brothel_rope_shut.glb")?;
        let rope_open = parts("brothel_rope_open.glb")?;
        let upper = parts("brothel_upper.glb")?;
        let roofs = parts("brothel_roofs.glb")?;
        let part_tall = parts("brothel_part_tall.glb")?;
        let part_low = parts("brothel_part_low.glb")?;
        let fronts = (0..3)
            .map(|i| parts(&format!("brothel_front{i}.glb")))
            .collect::<Result<Vec<_>>>()?;
        let door_leaf = parts("brothel_door_leaf.glb")?;
        let p = plan();
        let white = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        let mut girls = Vec::new();
        let sources = ClipSources::load();
        for (npc_index, npc) in p.npcs.iter().enumerate() {
            let Some(file) = npc.model.as_deref() else {
                continue;
            };
            let mut model = super::prefetch::load(&asset_root().join(file))
                .with_context(|| format!("load {file}"))?;
            // Walking and standing, borrowed and fitted to her rig.
            let walk = sources.onto(&mut model, sources.walk, "Girl_Walk");
            let stand = sources.onto(&mut model, sources.stand, "Girl_Stand");
            let stride = walk.map_or(1.2, |clip| stride_speed(&model, clip));
            let mut meshes = Vec::with_capacity(model.meshes.len());
            for mesh in &model.meshes {
                let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
                let normal = mesh
                    .normal_texture_index
                    .and_then(|i| model.textures.get(i));
                let texture = upload_texture_pair(
                    renderer, device, queue, albedo, normal, &mut cache, &white,
                )?;
                meshes.push((
                    renderer.upload_skinned_mesh(device, mesh)?,
                    mesh.material.with_ambient_occlusion(0.28),
                    texture,
                ));
            }
            let transforms = (0..model.meshes.len())
                .map(|i| model.mesh_transform(i))
                .collect();
            let idle = model.animation_index("Idle");
            let home =
                Mat4::from_translation(Vec3::new(npc.x, npc.level as f32 * p.level_height, npc.z))
                    * Mat4::from_rotation_y(npc.yaw);
            let parked = parked_props(&model, idle, home, npc);
            eprintln!(
                "[3d] brothel {}: walk {walk:?} stand {stand:?} stride {stride:.2} m/s, {} props",
                npc.id,
                parked.len()
            );
            girls.push(Girl {
                npc: npc_index,
                model,
                meshes,
                transforms,
                idle,
                walk,
                stand,
                stride,
                home,
                parked,
                animator: None,
                last: 0.0,
            });
        }
        // The grate's floor sits under the fire's lamp: the flames rise from it.
        let flames = match p.lamps.iter().find(|l| l.fire) {
            Some(l) => Some(ToonFire3D::new(
                renderer,
                device,
                Vec3::new(l.x, l.y - 0.42, l.z - 0.25),
                0.9,
            )?),
            None => None,
        };
        eprintln!(
            "[3d] brothel loaded in {:.0} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(Self {
            ground,
            fire,
            rope_shut,
            rope_open,
            upper,
            roofs,
            part_tall,
            part_low,
            fronts,
            door_leaf,
            girls,
            flames,
        })
    }

    /// Everything to draw this frame for `state`, girls animated at 24 samples a second.
    pub(crate) fn draws(
        &mut self,
        state: &State,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        elapsed: f32,
        toward_camera: Vec3,
        draws: &mut Vec<MeshDraw3D>,
    ) {
        let p = plan();
        let level = p.view_level(state.walker);
        let in_room = p.room_at(state.walker).map(|r| r.index);
        let push = |draws: &mut Vec<MeshDraw3D>, parts: &[Part], world: Mat4| {
            for part in parts {
                draws.push(MeshDraw3D {
                    mesh: part.mesh,
                    transform: world * part.transform,
                    material: part.material,
                    texture: part.texture,
                });
            }
        };
        push(draws, &self.ground, Mat4::IDENTITY);
        // The logs, and the living fire on them.
        push(draws, &self.fire, Mat4::IDENTITY);
        if let Some(flames) = self.flames.as_ref() {
            flames.draws(elapsed, toward_camera, draws);
        }
        push(
            draws,
            if state.rope_open {
                &self.rope_open
            } else {
                &self.rope_shut
            },
            Mat4::IDENTITY,
        );
        if level == 1 {
            push(draws, &self.upper, Mat4::IDENTITY);
            if in_room.is_none() {
                push(draws, &self.roofs, Mat4::IDENTITY);
                push(draws, &self.part_tall, Mat4::IDENTITY);
            } else {
                push(draws, &self.part_low, Mat4::IDENTITY);
            }
            for (i, front) in self.fronts.iter().enumerate() {
                if in_room != Some(i) {
                    push(draws, front, Mat4::IDENTITY);
                }
                if let Some(hinge) = p.doors.iter().find(|h| h.room == i) {
                    let shake = state.door_shake[i];
                    let angle = state.door_angle[i] + (shake * 40.0).sin() * 0.05 * shake;
                    let world = Mat4::from_translation(Vec3::new(hinge.x, hinge.y, hinge.z))
                        * Mat4::from_rotation_y(angle);
                    push(draws, &self.door_leaf, world);
                }
            }
        }
        // The girls: at their places, or walking and standing where the house's escort has
        // them; their clips cross-fade, the GPU deforms them. (QA: `VELVET_BROTHEL_EMPTY` leaves
        // the house without its people, for pictures of the place alone.)
        static EMPTY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let empty = *EMPTY.get_or_init(|| std::env::var_os("VELVET_BROTHEL_EMPTY").is_some());
        let looks = state.girl_looks();
        for girl in self.girls.iter_mut().filter(|_| !empty) {
            let look = looks.iter().find(|l| l.npc == girl.npc);
            let (placement, clip, speed) = match look {
                Some(l) if l.mode != GirlMode::Idle => (
                    Mat4::from_translation(l.at) * Mat4::from_rotation_y(l.yaw),
                    match l.mode {
                        GirlMode::Walk => girl.walk.or(girl.idle),
                        _ => girl.stand.or(girl.idle),
                    },
                    if l.mode == GirlMode::Walk {
                        (l.speed / girl.stride.max(0.1)).clamp(0.3, 2.5)
                    } else {
                        1.0
                    },
                ),
                _ => (girl.home, girl.idle, 1.0),
            };
            let dt = (elapsed - girl.last).clamp(0.0, 0.1);
            girl.last = elapsed;
            if let Some(clip) = clip {
                let animator = girl
                    .animator
                    .get_or_insert_with(|| velvet_render::Animator3D::new(clip, elapsed));
                animator.play(clip, 0.45);
                animator.set_speed(speed);
                animator.advance(dt);
                if let Some(pose) = animator.pose(&girl.model) {
                    for (i, (handle, _, _)) in girl.meshes.iter().enumerate() {
                        if let Some(joints) = girl.model.joint_matrices(i, &pose) {
                            let _ = renderer.set_skin_joints(queue, *handle, &joints);
                        }
                    }
                    girl.transforms = girl.model.pose_mesh_transforms(&pose);
                }
            }
            let away = look.is_some_and(|l| l.mode != GirlMode::Idle);
            for (i, ((mesh, material, texture), transform)) in
                girl.meshes.iter().zip(&girl.transforms).enumerate()
            {
                // Away from her place, what she held there stays behind, on the counter.
                let transform = match girl.parked.iter().find(|(m, _)| *m == i) {
                    Some((_, left)) if away => *left,
                    _ => placement * *transform,
                };
                draws.push(MeshDraw3D {
                    mesh: *mesh,
                    transform,
                    material: *material,
                    texture: *texture,
                });
            }
        }
    }
}

// ------------------------------------------------------------------ tests
#[cfg(test)]
mod tests {
    use super::*;

    fn walk_to(p: &Plan, mut s: Walker, dx: f32, dz: f32, steps: usize, rope: bool) -> Walker {
        for _ in 0..steps {
            s = p.slide(s, dx, dz, rope);
        }
        s
    }

    #[test]
    fn the_spawn_is_free_and_inside() {
        let p = plan();
        assert!(!p.blocked(p.spawn.x, p.spawn.z, 0));
        assert_eq!(
            p.zone_at(p.spawn.x, p.spawn.z, 0).map(|z| z.id.as_str()),
            Some("entrada")
        );
        // Where she appears is free, and out of the street door's reach.
        let inside = State::enter(Vec3::ZERO, 0.0, false);
        assert!(!p.blocked(inside.walker.x, inside.walker.z, 0));
        assert!(inside.nearest().map_or(true, |h| !h.exit));
    }

    #[test]
    fn the_rope_holds_until_it_is_lifted_and_then_the_stair_leads_up() {
        let p = plan();
        let lane = (p.stair.z0 + p.stair.z1) / 2.0;
        let foot = Walker {
            x: p.stair.x1 + 0.9,
            z: lane,
            level: 0,
            on_stair: false,
        };
        let held = walk_to(p, foot, -0.05, 0.0, 200, false);
        assert_eq!(held.level, 0);
        assert!(!held.on_stair);
        let up = walk_to(p, foot, -0.05, 0.0, 400, true);
        assert_eq!(up.level, 1);
        assert!(!up.on_stair);
        assert!((p.floor_y(up) - p.level_height).abs() < 1e-4);
        // And back down.
        let down = walk_to(p, up, 0.05, 0.0, 400, true);
        assert_eq!(down.level, 0);
        assert!(p.floor_y(down) < 1e-4);
    }

    #[test]
    fn the_house_sounds_from_its_places() {
        let e = emitters();
        assert!(
            e.iter().any(|e| e.level == 0 && e.at.y < 3.0),
            "the ground floor has sounds"
        );
        // The couple is upstairs, behind the locked door.
        let locked = plan().rooms.iter().find(|r| r.locked).unwrap();
        let upstairs: Vec<_> = e.iter().filter(|e| e.level == 1).collect();
        assert!(!upstairs.is_empty());
        for u in upstairs {
            assert!(
                u.at.x > locked.x0
                    && u.at.x < locked.x1
                    && u.at.z > locked.z0
                    && u.at.z < locked.z1
            );
        }
    }

    #[test]
    fn every_hotspot_has_its_lines() {
        let p = plan();
        for h in &p.hotspots {
            if let Some(d) = &h.dialogue {
                assert!(
                    p.dialogues.get(d).is_some_and(|l| !l.is_empty()),
                    "{}",
                    h.id
                );
            }
        }
        assert!(p.hotspots.iter().any(|h| h.exit));
        assert!(p.hotspots.iter().any(|h| h.id == "recepcionista"));
    }

    /// Every point of a route is somewhere she fits (her own floor, or the stair's lane).
    fn route_is_clear(p: &Plan, route: &Route, skip_first: f32) -> Result<(), String> {
        let len = route.length();
        let mut s = skip_first;
        while s <= len {
            let (w, _) = route.sample(s);
            let ok = if w.on_stair {
                p.in_lane(w.z)
            } else {
                !p.blocked(w.x, w.z, w.level)
            };
            if !ok {
                return Err(format!("blocked at {s:.2} m: {w:?}"));
            }
            s += 0.05;
        }
        Ok(())
    }

    #[test]
    fn each_girl_finds_her_way_to_the_desk() {
        let p = plan();
        let desk = p.hotspots.iter().find(|h| h.id == "recepcionista").unwrap();
        for id in ["nina", "alba", "noa"] {
            let girl = p.npcs.iter().find(|n| n.id == id).unwrap();
            let path = p
                .find_path(0, (girl.x, girl.z), (desk.x - 0.9, desk.z))
                .unwrap_or_else(|| panic!("{id}: no way to the desk"));
            let route = Route::new(
                path.into_iter()
                    .map(|(x, z)| Waypoint { x, z, level: 0 })
                    .collect(),
            );
            // Off her sofa first (the first step may start inside it).
            route_is_clear(p, &route, 0.7).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(route.length() < 40.0, "{id}: {:.1} m", route.length());
        }
    }

    #[test]
    fn the_way_up_reaches_each_free_room() {
        let p = plan();
        let desk = p.hotspots.iter().find(|h| h.id == "recepcionista").unwrap();
        for room in [0, 1] {
            let ((gx, gz), (qx, qz)) = room_spots(room);
            assert!(
                !p.blocked(gx, gz, 1) && !p.blocked(qx, qz, 1),
                "room {room} spots free"
            );
            let route = p.way_up((desk.x, desk.z), (gx, gz)).expect("a way up");
            route_is_clear(p, &route, 0.0).unwrap_or_else(|e| panic!("room {room}: {e}"));
            let (end, _) = route.sample(route.length());
            assert_eq!(end.level, 1);
            assert_eq!(p.room_at(end).map(|r| r.index), Some(room), "room {room}");
        }
    }

    #[test]
    fn a_girl_takes_the_queen_up_to_her_room() {
        let p = plan();
        let desk = p.hotspots.iter().find(|h| h.id == "recepcionista").unwrap();
        for (id, room) in [("nina", 1), ("alba", 0), ("noa", 1)] {
            let mut state = State::enter(Vec3::ZERO, 0.0, true);
            state.walker = Walker {
                x: desk.x,
                z: desk.z,
                level: 0,
                on_stair: false,
            };
            assert!(state.start_escort(id, room, "—Sígame."), "{id}");
            let mut yaw = 0.0;
            let mut arrived = None;
            let mut seen_stair = false;
            for _ in 0..(60 * 90) {
                if let Some(who) = state.tick_escort(1.0 / 60.0, &mut yaw) {
                    arrived = Some(who);
                    break;
                }
                seen_stair |= state.walker.on_stair;
                // The Queen never stands anywhere she does not fit.
                let w = state.walker;
                assert!(
                    if w.on_stair {
                        p.in_lane(w.z)
                    } else {
                        !p.blocked(w.x, w.z, w.level)
                    },
                    "{id}: the Queen inside something at {w:?}"
                );
            }
            assert_eq!(arrived, Some(id), "{id} arrives within 90 s");
            assert!(seen_stair, "{id}: they went up the stair");
            assert_eq!(p.room_at(state.walker).map(|r| r.index), Some(room), "{id}");
            // She stays with the Queen in the room, standing.
            let looks = state.girl_looks();
            assert_eq!(looks.len(), 1);
            assert_eq!(looks[0].mode, GirlMode::Stand);
            // Going back down sends her home.
            state.walker = Walker {
                x: desk.x,
                z: desk.z,
                level: 0,
                on_stair: false,
            };
            state.update(0.1);
            assert!(state.girl_looks().is_empty());
        }
    }

    #[test]
    fn skipping_the_escort_lands_in_the_room() {
        let p = plan();
        let desk = p.hotspots.iter().find(|h| h.id == "recepcionista").unwrap();
        let mut state = State::enter(Vec3::ZERO, 0.0, true);
        state.walker = Walker {
            x: desk.x,
            z: desk.z,
            level: 0,
            on_stair: false,
        };
        assert!(state.start_escort("nina", 1, "—Sígame."));
        let mut yaw = 0.0;
        state.skip_escort(&mut yaw);
        assert_eq!(state.tick_escort(1.0 / 60.0, &mut yaw), Some("nina"));
        assert_eq!(p.room_at(state.walker).map(|r| r.index), Some(1));
    }

    /// Where a bone of a girl is in a pose (model space, her scale included).
    fn bone(model: &GltfModel, pose: &velvet_render::Pose3D, name: &str) -> Vec3 {
        let m = model.pose_node_transform(pose, name).unwrap();
        Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2])
    }

    #[test]
    fn the_girls_walk_and_stand_with_their_arms_down() {
        let sources = ClipSources::load();
        if sources.walk.is_none() || sources.stand.is_none() {
            panic!("the girls' clip sources load");
        }
        for id in ["nina", "alba", "noa"] {
            let path = asset_root().join(format!("brothel_girl_{id}.glb"));
            let Ok(mut model) = GltfModel::load(&path) else {
                return;
            };
            let walk = sources
                .onto(&mut model, sources.walk, "Girl_Walk")
                .expect("walk");
            let stand = sources
                .onto(&mut model, sources.stand, "Girl_Stand")
                .expect("stand");
            for (name, clip) in [("walk", walk), ("stand", stand)] {
                for t in [0.0, 0.3, 0.7] {
                    let pose = model.pose(clip, t).unwrap();
                    for side in ["L", "R"] {
                        let shoulder = bone(&model, &pose, &format!("DEF-upper_arm{side}"));
                        let hand = bone(&model, &pose, &format!("DEF-hand{side}"));
                        let head = bone(&model, &pose, "DEF-head");
                        let foot = bone(&model, &pose, &format!("DEF-foot{side}"));
                        eprintln!(
                            "{id} {name} t{t} {side}: shoulder {:.2} {:.2} {:.2} hand {:.2} {:.2} {:.2} head y {:.2} foot y {:.2}",
                            shoulder.x, shoulder.y, shoulder.z, hand.x, hand.y, hand.z, head.y, foot.y
                        );
                        assert!(
                            hand.y < shoulder.y - 0.12,
                            "{id} {name} t{t}: {side} hand at {:.2}, shoulder at {:.2} (arms out like a T)",
                            hand.y,
                            shoulder.y
                        );
                        assert!(
                            head.y > 1.0,
                            "{id} {name}: standing (head at {:.2})",
                            head.y
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn free_bedrooms_can_be_entered_and_the_third_cannot() {
        let p = plan();
        for room in &p.rooms {
            let door = Walker {
                x: room.door_x,
                z: p.fronts_z + 0.8,
                level: 1,
                on_stair: false,
            };
            assert!(
                !p.blocked(door.x, door.z, 1),
                "corridor at room {}",
                room.index
            );
            let inside = walk_to(p, door, 0.0, -0.05, 60, true);
            assert_eq!(
                p.room_at(inside).is_some(),
                !room.locked,
                "room {}",
                room.index
            );
        }
    }
}
