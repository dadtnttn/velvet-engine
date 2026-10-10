//! Navigation: where people can walk, the way there, and how they keep out of each other's way.
//!
//! - [`NavMesh::build`] samples a zone's own walkability (the game's `blocked`/ground functions,
//!   so the navmesh can never disagree with where its characters can walk) on a fine grid, refuses
//!   steps too high to take, and merges the walkable cells into convex rectangles: the navmesh's
//!   polygons. Neighbouring rectangles share portals (the stretch of edge one can cross).
//! - [`NavMesh::find_path`] runs A* over the rectangles (entering each through the nearest point
//!   of its portal) and pulls the corridor tight with the funnel algorithm ("simple stupid funnel",
//!   Mononen): the path turns only at corners.
//! - [`Crowd`] moves agents along their paths and resolves their meetings with ORCA (optimal
//!   reciprocal collision avoidance: van den Berg, Guy, Lin & Manocha, 2011), each agent taking
//!   half the effort of avoiding the other, then keeps them on the navmesh.

use velvet_math::Vec3;

/// A rectangle of walkable cells, `[i0, i1) × [j0, j1)`.
#[derive(Debug, Clone, Copy)]
pub struct NavRect {
    pub i0: u32,
    pub j0: u32,
    pub i1: u32,
    pub j1: u32,
}

#[derive(Debug, Clone, Copy)]
struct Link {
    to: u32,
    /// The portal's ends (x, z).
    a: [f32; 2],
    b: [f32; 2],
}

/// A zone's navmesh.
pub struct NavMesh {
    x0: f32,
    z0: f32,
    cell: f32,
    w: usize,
    h: usize,
    max_step: f32,
    /// Per cell, the ground's height, NaN where nobody walks.
    height: Vec<f32>,
    rect_of: Vec<u32>,
    pub rects: Vec<NavRect>,
    links: Vec<Vec<Link>>,
    /// Per rectangle, what a metre across it costs (1 on dry ground; more in water...).
    cost: Vec<f32>,
}

const NONE: u32 = u32::MAX;
/// How far in from a portal's ends a path may turn (m): corners keep off the navmesh's edge.
const PORTAL_MARGIN: f32 = 0.25;
/// The largest rectangle side (cells): keeps A*'s entry points meaningful in open ground.
const MAX_SIDE: usize = 24;

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Twice the signed area of (o, a, b): positive when b lies left of the way from o to a (x right,
/// z down the page: "left" as seen walking along it on the map).
fn cross(o: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn closest_on_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    if len2 < 1e-12 {
        return a;
    }
    let t = (((p[0] - a[0]) * ab[0] + (p[1] - a[1]) * ab[1]) / len2).clamp(0.0, 1.0);
    [a[0] + ab[0] * t, a[1] + ab[1] * t]
}

impl NavMesh {
    /// Builds the navmesh over `[x0, x1] × [z0, z1]` on `cell`-metre cells. `sample(x, z)` gives
    /// the height of the ground where someone can stand, `None` where they cannot (it should
    /// already keep a body's radius clear of walls). Neighbouring cells further apart in height
    /// than `max_step` are not connected.
    pub fn build(
        bounds: [f32; 4],
        cell: f32,
        max_step: f32,
        sample: impl Fn(f32, f32) -> Option<f32>,
    ) -> Self {
        let [x0, z0, x1, z1] = bounds;
        let w = ((x1 - x0) / cell).ceil().max(1.0) as usize;
        let h = ((z1 - z0) / cell).ceil().max(1.0) as usize;
        let mut height = vec![f32::NAN; w * h];
        for j in 0..h {
            for i in 0..w {
                if let Some(y) = sample(x0 + (i as f32 + 0.5) * cell, z0 + (j as f32 + 0.5) * cell)
                {
                    height[j * w + i] = y;
                }
            }
        }
        let mut mesh = Self {
            x0,
            z0,
            cell,
            w,
            h,
            max_step,
            height,
            rect_of: vec![NONE; w * h],
            rects: Vec::new(),
            links: Vec::new(),
            cost: Vec::new(),
        };
        mesh.merge();
        mesh.link();
        mesh.cost = vec![1.0; mesh.rects.len()];
        mesh
    }

    /// What a metre costs where (the mean over each rectangle's cells): paths keep to the cheap
    /// ground and cross the dear only when it saves enough.
    pub fn set_costs(&mut self, cost: impl Fn(f32, f32) -> f32) {
        for (k, r) in self.rects.iter().enumerate() {
            let mut sum = 0.0;
            let mut n: f32 = 0.0;
            for j in r.j0..r.j1 {
                for i in r.i0..r.i1 {
                    sum += cost(
                        self.x0 + (i as f32 + 0.5) * self.cell,
                        self.z0 + (j as f32 + 0.5) * self.cell,
                    )
                    .max(0.1);
                    n += 1.0;
                }
            }
            self.cost[k] = sum / n.max(1.0);
        }
    }

    fn walkable(&self, i: usize, j: usize) -> bool {
        i < self.w && j < self.h && !self.height[j * self.w + i].is_nan()
    }

    fn step_ok(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        (self.height[a.1 * self.w + a.0] - self.height[b.1 * self.w + b.0]).abs() <= self.max_step
    }

    /// Greedy maximal rectangles: grow right along the row, then down while the whole span holds.
    fn merge(&mut self) {
        let (w, h) = (self.w, self.h);
        for j in 0..h {
            for i in 0..w {
                if !self.walkable(i, j) || self.rect_of[j * w + i] != NONE {
                    continue;
                }
                let mut i1 = i + 1;
                while i1 < w
                    && i1 - i < MAX_SIDE
                    && self.walkable(i1, j)
                    && self.rect_of[j * w + i1] == NONE
                    && self.step_ok((i1 - 1, j), (i1, j))
                {
                    i1 += 1;
                }
                let mut j1 = j + 1;
                'grow: while j1 < h && j1 - j < MAX_SIDE {
                    for k in i..i1 {
                        let free = self.walkable(k, j1)
                            && self.rect_of[j1 * w + k] == NONE
                            && self.step_ok((k, j1 - 1), (k, j1));
                        let along = k == i || self.step_ok((k - 1, j1), (k, j1));
                        if !free || !along {
                            break 'grow;
                        }
                    }
                    j1 += 1;
                }
                let index = self.rects.len() as u32;
                for jj in j..j1 {
                    for k in i..i1 {
                        self.rect_of[jj * w + k] = index;
                    }
                }
                self.rects.push(NavRect {
                    i0: i as u32,
                    j0: j as u32,
                    i1: i1 as u32,
                    j1: j1 as u32,
                });
            }
        }
    }

    /// Portals: along every rectangle's right and bottom edges, the runs of cells that step into
    /// one same neighbour.
    fn link(&mut self) {
        let mut links = vec![Vec::new(); self.rects.len()];
        let (x0, z0, c) = (self.x0, self.z0, self.cell);
        for (index, r) in self.rects.iter().enumerate() {
            // Right edge: cells (i1-1, j) → (i1, j).
            let mut run: Option<(u32, u32)> = None; // (neighbour, start j)
            let i = r.i1 as usize;
            for j in r.j0 as usize..=r.j1 as usize {
                let next = if j < r.j1 as usize
                    && i < self.w
                    && self.walkable(i, j)
                    && self.step_ok((i - 1, j), (i, j))
                {
                    self.rect_of[j * self.w + i]
                } else {
                    NONE
                };
                if let Some((to, start)) = run {
                    if to != next {
                        let a = [x0 + i as f32 * c, z0 + start as f32 * c];
                        let b = [x0 + i as f32 * c, z0 + j as f32 * c];
                        links[index].push(Link { to, a, b });
                        links[to as usize].push(Link {
                            to: index as u32,
                            a,
                            b,
                        });
                        run = None;
                    }
                }
                if run.is_none() && next != NONE {
                    run = Some((next, j as u32));
                }
            }
            // Bottom edge: cells (i, j1-1) → (i, j1).
            let mut run: Option<(u32, u32)> = None;
            let j = r.j1 as usize;
            for i in r.i0 as usize..=r.i1 as usize {
                let next = if i < r.i1 as usize
                    && j < self.h
                    && self.walkable(i, j)
                    && self.step_ok((i, j - 1), (i, j))
                {
                    self.rect_of[j * self.w + i]
                } else {
                    NONE
                };
                if let Some((to, start)) = run {
                    if to != next {
                        let a = [x0 + start as f32 * c, z0 + j as f32 * c];
                        let b = [x0 + i as f32 * c, z0 + j as f32 * c];
                        links[index].push(Link { to, a, b });
                        links[to as usize].push(Link {
                            to: index as u32,
                            a,
                            b,
                        });
                        run = None;
                    }
                }
                if run.is_none() && next != NONE {
                    run = Some((next, i as u32));
                }
            }
        }
        self.links = links;
    }

    fn cell_of(&self, x: f32, z: f32) -> Option<(usize, usize)> {
        let i = ((x - self.x0) / self.cell).floor();
        let j = ((z - self.z0) / self.cell).floor();
        (i >= 0.0 && j >= 0.0 && (i as usize) < self.w && (j as usize) < self.h)
            .then_some((i as usize, j as usize))
    }

    /// The rectangle under (x, z), if anyone can stand there.
    pub fn rect_at(&self, x: f32, z: f32) -> Option<u32> {
        let (i, j) = self.cell_of(x, z)?;
        let r = self.rect_of[j * self.w + i];
        (r != NONE).then_some(r)
    }

    /// Can someone stand at (x, z)?
    pub fn walkable_at(&self, x: f32, z: f32) -> bool {
        self.rect_at(x, z).is_some()
    }

    /// Can one walk straight from `a` to `b` (every step of the way on the navmesh)?
    pub fn clear_line(&self, a: [f32; 2], b: [f32; 2]) -> bool {
        let steps = (dist(a, b) / (self.cell * 0.5)).ceil().max(1.0) as usize;
        (0..=steps).all(|k| {
            let t = k as f32 / steps as f32;
            self.walkable_at(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        })
    }

    /// The ground's height at (x, z) on the navmesh.
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        let (i, j) = self.cell_of(x, z)?;
        let y = self.height[j * self.w + i];
        (!y.is_nan()).then_some(y)
    }

    /// The nearest place someone can stand, searching out to `reach` metres.
    pub fn nearest(&self, x: f32, z: f32, reach: f32) -> Option<[f32; 2]> {
        if self.walkable_at(x, z) {
            return Some([x, z]);
        }
        let (ci, cj) = {
            let i = ((x - self.x0) / self.cell).floor() as i64;
            let j = ((z - self.z0) / self.cell).floor() as i64;
            (i, j)
        };
        let rings = (reach / self.cell).ceil() as i64;
        let mut best: Option<([f32; 2], f32)> = None;
        for ring in 1..=rings {
            for dj in -ring..=ring {
                for di in -ring..=ring {
                    if di.abs() != ring && dj.abs() != ring {
                        continue;
                    }
                    let (i, j) = (ci + di, cj + dj);
                    if i < 0 || j < 0 || !self.walkable(i as usize, j as usize) {
                        continue;
                    }
                    let p = [
                        self.x0 + (i as f32 + 0.5) * self.cell,
                        self.z0 + (j as f32 + 0.5) * self.cell,
                    ];
                    let d = dist(p, [x, z]);
                    if best.map_or(true, |(_, bd)| d < bd) {
                        best = Some((p, d));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(p, _)| p)
    }

    /// Every rectangle reachable from (x, z) (the walkable area someone there can get to).
    pub fn reachable_from(&self, x: f32, z: f32) -> Vec<bool> {
        let mut seen = vec![false; self.rects.len()];
        let Some(start) = self.rect_at(x, z) else {
            return seen;
        };
        let mut stack = vec![start];
        seen[start as usize] = true;
        while let Some(r) = stack.pop() {
            for l in &self.links[r as usize] {
                if !seen[l.to as usize] {
                    seen[l.to as usize] = true;
                    stack.push(l.to);
                }
            }
        }
        seen
    }

    /// A top-down picture of the navmesh: `colour(rect)` for walkable cells (rectangle edges a
    /// little darker), dark slate for the rest; `scale` pixels a cell. North (−z) up. RGBA rows.
    pub fn picture(&self, scale: u32, colour: impl Fn(u32) -> [u8; 3]) -> (u32, u32, Vec<u8>) {
        let (w, h) = (self.w as u32 * scale, self.h as u32 * scale);
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let (i, j) = ((x / scale) as usize, (y / scale) as usize);
                let r = self.rect_of[j * self.w + i];
                let c = if r == NONE {
                    [34, 38, 48]
                } else {
                    let base = colour(r);
                    let rect = self.rects[r as usize];
                    let edge = (i as u32 == rect.i0 && x % scale == 0)
                        || (j as u32 == rect.j0 && y % scale == 0);
                    if edge {
                        base.map(|v| (v as f32 * 0.8) as u8)
                    } else {
                        base
                    }
                };
                let k = ((y * w + x) * 4) as usize;
                px[k..k + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        (w, h, px)
    }

    /// Pixel of (x, z) in [`Self::picture`].
    pub fn pixel_of(&self, x: f32, z: f32, scale: u32) -> (i64, i64) {
        (
            ((x - self.x0) / self.cell * scale as f32) as i64,
            ((z - self.z0) / self.cell * scale as f32) as i64,
        )
    }

    /// Does a rectangle touch the edge of the area the navmesh was built over?
    pub fn rect_on_border(&self, r: u32) -> bool {
        let r = self.rects[r as usize];
        r.i0 == 0 || r.j0 == 0 || r.i1 as usize == self.w || r.j1 as usize == self.h
    }

    /// A rectangle's middle (x, z).
    pub fn rect_center(&self, r: u32) -> [f32; 2] {
        let r = self.rects[r as usize];
        [
            self.x0 + (r.i0 + r.i1) as f32 * 0.5 * self.cell,
            self.z0 + (r.j0 + r.j1) as f32 * 0.5 * self.cell,
        ]
    }

    /// A rectangle's area (m²).
    pub fn rect_area(&self, r: u32) -> f32 {
        let r = self.rects[r as usize];
        (r.i1 - r.i0) as f32 * (r.j1 - r.j0) as f32 * self.cell * self.cell
    }

    /// The way from `from` to `to`: the corners to walk through, ending at `to` (the start left
    /// out). `None` when one cannot get there.
    pub fn find_path(&self, from: [f32; 2], to: [f32; 2]) -> Option<Vec<[f32; 2]>> {
        let start = self.rect_at(from[0], from[1])?;
        let goal = self.rect_at(to[0], to[1])?;
        if start == goal {
            return Some(vec![to]);
        }
        // A* over rectangles; each is reached at a point of the portal it was entered by.
        let n = self.rects.len();
        let mut g = vec![f32::INFINITY; n];
        let mut entry = vec![[0.0f32; 2]; n];
        let mut came: Vec<Option<(u32, usize)>> = vec![None; n]; // (from rect, link index in from)
        let mut closed = vec![false; n];
        let mut open = std::collections::BinaryHeap::new();
        g[start as usize] = 0.0;
        entry[start as usize] = from;
        open.push(Node {
            f: dist(from, to),
            r: start,
        });
        while let Some(Node { r, .. }) = open.pop() {
            if closed[r as usize] {
                continue;
            }
            if r == goal {
                break;
            }
            closed[r as usize] = true;
            let p = entry[r as usize];
            for (k, l) in self.links[r as usize].iter().enumerate() {
                if closed[l.to as usize] {
                    continue;
                }
                // Through the portal a little in from its ends.
                let (a, b) = shrink(l.a, l.b, PORTAL_MARGIN);
                let q = closest_on_segment(p, a, b);
                let cost = g[r as usize] + dist(p, q) * self.cost[r as usize];
                if cost < g[l.to as usize] {
                    g[l.to as usize] = cost;
                    entry[l.to as usize] = q;
                    came[l.to as usize] = Some((r, k));
                    open.push(Node {
                        f: cost + dist(q, to),
                        r: l.to,
                    });
                }
            }
        }
        if g[goal as usize].is_infinite() {
            return None;
        }
        // The corridor's portals, from the start, oriented (left, right) for the walk.
        let mut portals = Vec::new();
        let mut r = goal;
        while let Some((prev, k)) = came[r as usize] {
            let l = self.links[prev as usize][k];
            portals.push((prev, l));
            r = prev;
        }
        portals.reverse();
        let mut oriented = vec![(from, from)];
        for (prev, l) in &portals {
            let c0 = self.rect_center(*prev);
            let c1 = self.rect_center(l.to);
            let (a, b) = shrink(l.a, l.b, PORTAL_MARGIN);
            let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
            let ahead = [mid[0] + (c1[0] - c0[0]), mid[1] + (c1[1] - c0[1])];
            if cross(mid, ahead, a) > 0.0 {
                oriented.push((a, b));
            } else {
                oriented.push((b, a));
            }
        }
        oriented.push((to, to));
        Some(funnel(&oriented))
    }
}

fn shrink(a: [f32; 2], b: [f32; 2], by: f32) -> ([f32; 2], [f32; 2]) {
    let len = dist(a, b);
    if len <= 2.0 * by + 1e-4 {
        let m = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
        return (m, m);
    }
    let d = [(b[0] - a[0]) / len * by, (b[1] - a[1]) / len * by];
    ([a[0] + d[0], a[1] + d[1]], [b[0] - d[0], b[1] - d[1]])
}

#[derive(PartialEq)]
struct Node {
    f: f32,
    r: u32,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .f
            .partial_cmp(&self.f)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// String pulling through portals given as (left, right): the corners of the tightest way, the
/// start left out, the end included.
fn funnel(portals: &[([f32; 2], [f32; 2])]) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    let mut apex = portals[0].0;
    let (mut left, mut right) = portals[0];
    let (mut left_i, mut right_i) = (0usize, 0usize);
    let mut i = 1;
    let same = |a: [f32; 2], b: [f32; 2]| dist(a, b) < 1e-5;
    while i < portals.len() {
        let (l, r) = portals[i];
        // The right side closes in (r turns left, towards the inside).
        if cross(apex, right, r) >= 0.0 {
            if same(apex, right) || cross(apex, left, r) < 0.0 {
                right = r;
                right_i = i;
            } else {
                // It crossed the left side: the left corner is on the way.
                out.push(left);
                apex = left;
                let apex_i = left_i;
                left = apex;
                right = apex;
                left_i = apex_i;
                right_i = apex_i;
                i = apex_i + 1;
                continue;
            }
        }
        // The left side closes in.
        if cross(apex, left, l) <= 0.0 {
            if same(apex, left) || cross(apex, right, l) > 0.0 {
                left = l;
                left_i = i;
            } else {
                out.push(right);
                apex = right;
                let apex_i = right_i;
                left = apex;
                right = apex;
                left_i = apex_i;
                right_i = apex_i;
                i = apex_i + 1;
                continue;
            }
        }
        i += 1;
    }
    let end = portals[portals.len() - 1].0;
    if out.last().map_or(true, |p| !same(*p, end)) {
        out.push(end);
    }
    out
}

// --------------------------------------------------------------------------------------- crowd

/// Someone walking.
#[derive(Debug, Clone)]
pub struct Walker {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub radius: f32,
    pub max_speed: f32,
    /// Corners still ahead.
    pub path: Vec<[f32; 2]>,
    /// Standing still (seated, lying): others go round, it does not move.
    pub anchored: bool,
    /// How long it has been getting nowhere (it finds a new way after a moment).
    stuck: f32,
}

impl Walker {
    pub fn new(pos: [f32; 2], radius: f32, max_speed: f32) -> Self {
        Self {
            pos,
            vel: [0.0; 2],
            radius,
            max_speed,
            path: Vec::new(),
            anchored: false,
            stuck: 0.0,
        }
    }

    /// Arrived (nothing left to walk)?
    pub fn arrived(&self) -> bool {
        self.path.is_empty()
    }

    fn preferred(&self) -> [f32; 2] {
        let Some(&next) = self.path.first() else {
            return [0.0; 2];
        };
        let d = [next[0] - self.pos[0], next[1] - self.pos[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if len < 1e-4 {
            return [0.0; 2];
        }
        // Slow down into the last corner.
        let speed = if self.path.len() == 1 {
            self.max_speed.min(len * 1.5)
        } else {
            self.max_speed
        };
        [d[0] / len * speed, d[1] / len * speed]
    }
}

/// Everyone walking in a zone.
#[derive(Debug, Default)]
pub struct Crowd {
    pub walkers: Vec<Walker>,
}

const TIME_HORIZON: f32 = 2.0;

impl Crowd {
    /// One step: everyone heads for their next corner, ORCA settles who gives way, and they move
    /// (never off the navmesh).
    pub fn step(&mut self, mesh: &NavMesh, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let snapshot: Vec<([f32; 2], [f32; 2], f32, bool)> = self
            .walkers
            .iter()
            .map(|w| (w.pos, w.vel, w.radius, w.anchored))
            .collect();
        let mut new_vel = Vec::with_capacity(self.walkers.len());
        for (k, w) in self.walkers.iter().enumerate() {
            if w.anchored {
                new_vel.push([0.0; 2]);
                continue;
            }
            // A slight turn, one way or the other by who it is, breaks the perfectly symmetric
            // meetings ORCA alone would stall in (RVO2's advice: perturb the preferred velocity).
            let pref = {
                let p = w.preferred();
                let a = if k % 2 == 0 { 0.06 } else { -0.06 };
                let (s, c) = (a as f32).sin_cos();
                [p[0] * c - p[1] * s, p[0] * s + p[1] * c]
            };
            let mut lines = Vec::new();
            for (m, &(pos, vel, radius, anchored)) in snapshot.iter().enumerate() {
                if m == k {
                    continue;
                }
                let rel = [pos[0] - w.pos[0], pos[1] - w.pos[1]];
                if rel[0] * rel[0] + rel[1] * rel[1]
                    > (TIME_HORIZON * w.max_speed * 2.0 + radius + w.radius).powi(2)
                {
                    continue;
                }
                // An anchored one does not move: this walker takes all of the avoiding.
                lines.push(orca_line(
                    w.pos,
                    w.vel,
                    w.radius,
                    pos,
                    vel,
                    radius,
                    if anchored { 1.0 } else { 0.5 },
                    dt,
                ));
            }
            new_vel.push(solve(&lines, w.max_speed, pref));
        }
        for (w, v) in self.walkers.iter_mut().zip(new_vel) {
            w.vel = v;
            let before = w.pos;
            let next = [w.pos[0] + v[0] * dt, w.pos[1] + v[1] * dt];
            // Stay on the navmesh: if the step leaves it, the nearest turn of it that does not
            // (sliding along the edge), else stand.
            if mesh.walkable_at(next[0], next[1]) {
                w.pos = next;
            } else {
                for turn in [0.35f32, -0.35, 0.7, -0.7, 1.1, -1.1, 1.5, -1.5] {
                    let (s, c) = turn.sin_cos();
                    let d = [
                        (v[0] * c - v[1] * s) * dt * c.max(0.0),
                        (v[0] * s + v[1] * c) * dt * c.max(0.0),
                    ];
                    let q = [w.pos[0] + d[0], w.pos[1] + d[1]];
                    if mesh.walkable_at(q[0], q[1]) {
                        w.pos = q;
                        break;
                    }
                }
            }
            // Pushed off its line by someone else, it can end up against the navmesh's edge with
            // the next corner behind it: after a moment getting nowhere it finds a new way from
            // where it stands.
            if !w.anchored && !w.path.is_empty() {
                let moved = dist(before, w.pos);
                if moved < w.max_speed * dt * 0.25 {
                    w.stuck += dt;
                } else {
                    w.stuck = (w.stuck - dt).max(0.0);
                }
                if w.stuck > 0.4 {
                    w.stuck = 0.0;
                    let goal = *w.path.last().unwrap();
                    if let Some(path) = mesh.find_path(w.pos, goal) {
                        w.path = path;
                    }
                }
            }
            // Corners reached: the last when she is on it; another when it is right here, or when
            // the one after it is already in plain sight.
            while let Some(&c) = w.path.first() {
                let done = if w.path.len() == 1 {
                    dist(c, w.pos) < 0.08
                } else {
                    dist(c, w.pos) < 0.1
                        || (dist(c, w.pos) < 1.0 && mesh.clear_line(w.pos, w.path[1]))
                };
                if done {
                    w.path.remove(0);
                } else {
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Line {
    point: [f32; 2],
    dir: [f32; 2],
}

fn det(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[1] - a[1] * b[0]
}
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}
fn mul(a: [f32; 2], s: f32) -> [f32; 2] {
    [a[0] * s, a[1] * s]
}

/// The ORCA half-plane of velocities that keep `me` clear of `other` for the time horizon (RVO2's
/// construction), `share` of the avoiding being mine.
#[allow(clippy::too_many_arguments)]
fn orca_line(
    pos: [f32; 2],
    vel: [f32; 2],
    radius: f32,
    other_pos: [f32; 2],
    other_vel: [f32; 2],
    other_radius: f32,
    share: f32,
    dt: f32,
) -> Line {
    let rel_pos = sub(other_pos, pos);
    let rel_vel = sub(vel, other_vel);
    let dist_sq = dot(rel_pos, rel_pos);
    let r = radius + other_radius;
    let r_sq = r * r;
    let inv_tau = 1.0 / TIME_HORIZON;
    let (dir, u);
    if dist_sq > r_sq {
        let w = sub(rel_vel, mul(rel_pos, inv_tau));
        let w_len_sq = dot(w, w);
        let dot1 = dot(w, rel_pos);
        if dot1 < 0.0 && dot1 * dot1 > r_sq * w_len_sq {
            // Onto the cut-off circle.
            let w_len = w_len_sq.sqrt().max(1e-6);
            let unit = mul(w, 1.0 / w_len);
            dir = [unit[1], -unit[0]];
            u = mul(unit, r * inv_tau - w_len);
        } else {
            // Onto a leg of the cone.
            let leg = (dist_sq - r_sq).sqrt();
            dir = if det(rel_pos, w) > 0.0 {
                mul(
                    [
                        rel_pos[0] * leg - rel_pos[1] * r,
                        rel_pos[0] * r + rel_pos[1] * leg,
                    ],
                    1.0 / dist_sq,
                )
            } else {
                mul(
                    [
                        rel_pos[0] * leg + rel_pos[1] * r,
                        -rel_pos[0] * r + rel_pos[1] * leg,
                    ],
                    -1.0 / dist_sq,
                )
            };
            u = sub(mul(dir, dot(rel_vel, dir)), rel_vel);
        }
    } else {
        // Already touching: part within this step.
        let inv_dt = 1.0 / dt.max(1e-4);
        let w = sub(rel_vel, mul(rel_pos, inv_dt));
        let w_len = dot(w, w).sqrt().max(1e-6);
        let unit = mul(w, 1.0 / w_len);
        dir = [unit[1], -unit[0]];
        u = mul(unit, r * inv_dt - w_len);
    }
    Line {
        point: add(vel, mul(u, share)),
        dir,
    }
}

/// RVO2's linear program 1: the best point on line `i` given lines before it.
fn lp1(
    lines: &[Line],
    i: usize,
    radius: f32,
    opt: [f32; 2],
    direction_opt: bool,
) -> Option<[f32; 2]> {
    let l = lines[i];
    let dot_product = dot(l.point, l.dir);
    let discriminant = dot_product * dot_product + radius * radius - dot(l.point, l.point);
    if discriminant < 0.0 {
        return None;
    }
    let sqrt_disc = discriminant.sqrt();
    let mut t_left = -dot_product - sqrt_disc;
    let mut t_right = -dot_product + sqrt_disc;
    for other in &lines[..i] {
        let denominator = det(l.dir, other.dir);
        let numerator = det(other.dir, sub(l.point, other.point));
        if denominator.abs() <= 1e-6 {
            if numerator < 0.0 {
                return None;
            }
            continue;
        }
        let t = numerator / denominator;
        if denominator >= 0.0 {
            t_right = t_right.min(t);
        } else {
            t_left = t_left.max(t);
        }
        if t_left > t_right {
            return None;
        }
    }
    let t = if direction_opt {
        if dot(opt, l.dir) > 0.0 {
            t_right
        } else {
            t_left
        }
    } else {
        dot(l.dir, sub(opt, l.point)).clamp(t_left, t_right)
    };
    Some(add(l.point, mul(l.dir, t)))
}

/// RVO2's linear program 2: the velocity nearest `opt` (or furthest along it) inside every line
/// and within `radius`. Returns the result and the first line it failed on (lines.len() if none).
fn lp2(lines: &[Line], radius: f32, opt: [f32; 2], direction_opt: bool) -> ([f32; 2], usize) {
    let mut result = if direction_opt {
        mul(opt, radius)
    } else if dot(opt, opt) > radius * radius {
        mul(opt, radius / dot(opt, opt).sqrt())
    } else {
        opt
    };
    for i in 0..lines.len() {
        if det(lines[i].dir, sub(lines[i].point, result)) > 0.0 {
            match lp1(lines, i, radius, opt, direction_opt) {
                Some(r) => result = r,
                None => return (result, i),
            }
        }
    }
    (result, lines.len())
}

/// RVO2's linear program 3: when no velocity satisfies everyone, the one that breaks the lines
/// least.
fn lp3(lines: &[Line], begin: usize, radius: f32, mut result: [f32; 2]) -> [f32; 2] {
    let mut distance = 0.0;
    for i in begin..lines.len() {
        if det(lines[i].dir, sub(lines[i].point, result)) > distance {
            let mut projected = Vec::with_capacity(i);
            for j in 0..i {
                let denominator = det(lines[i].dir, lines[j].dir);
                let point = if denominator.abs() <= 1e-6 {
                    if dot(lines[i].dir, lines[j].dir) > 0.0 {
                        continue;
                    }
                    mul(add(lines[i].point, lines[j].point), 0.5)
                } else {
                    add(
                        lines[i].point,
                        mul(
                            lines[i].dir,
                            det(lines[j].dir, sub(lines[i].point, lines[j].point)) / denominator,
                        ),
                    )
                };
                let d = sub(lines[j].dir, lines[i].dir);
                let len = dot(d, d).sqrt().max(1e-6);
                projected.push(Line {
                    point,
                    dir: mul(d, 1.0 / len),
                });
            }
            let temp = result;
            let (r, fail) = lp2(
                &projected,
                radius,
                [-lines[i].dir[1], lines[i].dir[0]],
                true,
            );
            result = if fail < projected.len() { temp } else { r };
            distance = det(lines[i].dir, sub(lines[i].point, result));
        }
    }
    result
}

fn solve(lines: &[Line], max_speed: f32, pref: [f32; 2]) -> [f32; 2] {
    let (result, fail) = lp2(lines, max_speed, pref, false);
    if fail < lines.len() {
        lp3(lines, fail, max_speed, result)
    } else {
        result
    }
}

/// (x, z) of a point.
pub fn flat(p: Vec3) -> [f32; 2] {
    [p.x, p.z]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 × 10 m yard with a wall across the middle, open only at its east end, and a 0.5 m
    /// ledge in the south-west corner (too high to step).
    fn yard() -> NavMesh {
        NavMesh::build([0.0, 0.0, 10.0, 10.0], 0.25, 0.3, |x, z| {
            if (4.8..5.2).contains(&z) && x < 8.5 {
                None
            } else if x < 2.0 && z > 8.0 {
                Some(0.5)
            } else {
                Some(0.0)
            }
        })
    }

    #[test]
    fn the_yard_merges_into_few_rectangles_that_cover_it() {
        let m = yard();
        assert!(m.rects.len() < 60, "{} rectangles", m.rects.len());
        let area: f32 = (0..m.rects.len() as u32).map(|r| m.rect_area(r)).sum();
        assert!((area - (100.0 - 8.5 * 0.5)).abs() < 1.0, "covered {area}");
        assert!(!m.walkable_at(3.0, 5.0) && m.walkable_at(9.0, 5.0));
    }

    #[test]
    fn the_path_goes_round_the_wall_turning_only_at_its_end() {
        let m = yard();
        let path = m.find_path([1.0, 2.0], [1.0, 7.0]).expect("a way round");
        assert_eq!(*path.last().unwrap(), [1.0, 7.0]);
        // Round the wall's east end: some corner near (8.5, 5).
        assert!(
            path.iter()
                .any(|p| (p[0] - 8.5).abs() < 0.8 && (p[1] - 5.0).abs() < 0.8),
            "{path:?}"
        );
        assert!(path.len() <= 4, "pulled tight: {path:?}");
        // Every leg stays walkable.
        let mut at = [1.0, 2.0];
        for p in &path {
            for k in 0..=40 {
                let t = k as f32 / 40.0;
                let q = [at[0] + (p[0] - at[0]) * t, at[1] + (p[1] - at[1]) * t];
                assert!(m.walkable_at(q[0], q[1]), "{q:?} on {path:?}");
            }
            at = *p;
        }
        let length: f32 = std::iter::once([1.0, 2.0])
            .chain(path.iter().copied())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|w| dist(w[0], w[1]))
            .sum();
        assert!(length < 17.0, "short: {length}");
    }

    #[test]
    fn a_straight_line_needs_no_corners_and_a_ledge_is_not_a_step() {
        let m = yard();
        assert_eq!(
            m.find_path([1.0, 1.0], [7.0, 3.0]).unwrap(),
            vec![[7.0, 3.0]]
        );
        // The ledge is its own island: there is no way up.
        assert!(m.find_path([5.0, 9.0], [1.0, 9.0]).is_none());
        assert!(m.walkable_at(1.0, 9.0));
    }

    #[test]
    fn two_walkers_meeting_head_on_pass_each_other() {
        let m = NavMesh::build([0.0, 0.0, 12.0, 6.0], 0.25, 0.3, |_, _| Some(0.0));
        let mut crowd = Crowd::default();
        let mut a = Walker::new([1.0, 3.0], 0.3, 1.2);
        a.path = vec![[11.0, 3.0]];
        let mut b = Walker::new([11.0, 3.02], 0.3, 1.2);
        b.path = vec![[1.0, 3.0]];
        crowd.walkers = vec![a, b];
        let mut closest = f32::INFINITY;
        for _ in 0..900 {
            crowd.step(&m, 1.0 / 60.0);
            closest = closest.min(dist(crowd.walkers[0].pos, crowd.walkers[1].pos));
        }
        assert!(closest > 0.55, "they kept apart: {closest}");
        assert!(
            crowd.walkers.iter().all(|w| w.arrived()),
            "both got there: {:?}",
            crowd.walkers.iter().map(|w| w.pos).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_walker_goes_round_someone_sitting_in_the_way() {
        let m = NavMesh::build([0.0, 0.0, 12.0, 6.0], 0.25, 0.3, |_, _| Some(0.0));
        let mut crowd = Crowd::default();
        let mut a = Walker::new([1.0, 3.0], 0.3, 1.2);
        a.path = vec![[11.0, 3.0]];
        let mut seated = Walker::new([6.0, 3.0], 0.4, 0.0);
        seated.anchored = true;
        crowd.walkers = vec![a, seated];
        let mut closest = f32::INFINITY;
        for _ in 0..900 {
            crowd.step(&m, 1.0 / 60.0);
            closest = closest.min(dist(crowd.walkers[0].pos, crowd.walkers[1].pos));
        }
        assert!(closest > 0.6, "went round: {closest}");
        assert!(
            crowd.walkers[0].arrived(),
            "stuck at {:?} vel {:?}",
            crowd.walkers[0].pos,
            crowd.walkers[0].vel
        );
        assert_eq!(crowd.walkers[1].pos, [6.0, 3.0]);
    }
}
