//! Hand-made floors: a level modelled by hand (not generated) carries a grid of floors baked from its
//! collision mesh. Each column of the grid lists the floors over one another under that spot of the
//! plan, top to bottom: the floor's height at the column's centre and its slope, the height of
//! whatever is over it, and the room it belongs to. With a grid, a [`Blueprint`](crate::Blueprint)
//! answers [`surface`](crate::Blueprint::surface) and [`room_at`](crate::Blueprint::room_at) from it
//! alone, so the controller, the actors and the line of sight walk the modelled level exactly as
//! they walk a generated one.
//!
//! The file a level's bake writes (little-endian):
//!
//! ```text
//! b"DQFL", u32 version (1 or 2), f32 x0, f32 z0, f32 cell, u32 w, u32 h, u32 spans,
//! u32 offsets[w * h + 1]          column i = row * w + col, rows along +z
//! per span: f32 floor, f32 ceiling, f32 slope_x, f32 slope_z [, u16 room (version 2)]
//! ```
//!
//! A floor whose ceiling is at the floor itself is a spot where something stands at body height (a
//! thin wall, a post): no one fits there.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use velvet_math::Vec3;

/// One floor of a column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorSpan {
    /// Height of the floor at the column's centre.
    pub floor: f32,
    /// Height of whatever is over the floor.
    pub ceiling: f32,
    /// Rise of the floor per unit of x and of z around the column's centre.
    pub slope: [f32; 2],
    /// The room it belongs to, if any.
    pub room: Option<u16>,
}

/// The floors of a hand-made level.
#[derive(Clone, Debug, PartialEq)]
pub struct FloorGrid {
    pub x0: f32,
    pub z0: f32,
    pub cell: f32,
    pub width: u32,
    pub height: u32,
    /// The highest ledge one walks up without jumping (a modelled level has roots, sills and
    /// curbs a little higher than a generated one's treads).
    pub step: f32,
    /// How far the bake already keeps bodies from walls: a column with something solid this close
    /// to its centre at body height is no floor. Bodies are that much narrower over this grid.
    pub margin: f32,
    /// The room a body needs over a floor: the bake measured its beams, lintels and low rock for
    /// a body this tall, so a taller one would stop at places its walk goes through.
    pub head: f32,
    offsets: Vec<u32>,
    spans: Vec<FloorSpan>,
}

/// A modelled level's step: the prototypes' capsule walks over about this much.
pub const AUTHORED_STEP: f32 = 0.45;
/// The bakes' wall margin (the mine's floors.py keeps floors 0.18 m from walls).
pub const AUTHORED_MARGIN: f32 = 0.18;
/// The bakes' headroom: the prototypes' capsule stands 1.64 m (floors.py's HEAD).
pub const AUTHORED_HEAD: f32 = 1.64;

impl FloorGrid {
    /// A grid from its columns: `columns[row * width + col]` lists that column's floors.
    pub fn new(
        x0: f32,
        z0: f32,
        cell: f32,
        width: u32,
        height: u32,
        columns: Vec<Vec<FloorSpan>>,
    ) -> Self {
        assert_eq!(
            columns.len(),
            (width * height) as usize,
            "one list of floors per column"
        );
        let mut offsets = Vec::with_capacity(columns.len() + 1);
        let mut spans = Vec::new();
        offsets.push(0);
        for column in columns {
            spans.extend(column);
            offsets.push(spans.len() as u32);
        }
        Self {
            x0,
            z0,
            cell,
            width,
            height,
            step: AUTHORED_STEP,
            margin: AUTHORED_MARGIN,
            head: AUTHORED_HEAD,
            offsets,
            spans,
        }
    }

    /// Reads a baked grid (see the module's docs).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut at = 0usize;
        let mut take = |n: usize| -> Result<&[u8], String> {
            let part = bytes
                .get(at..at + n)
                .ok_or("floor grid: the file ends early")?;
            at += n;
            Ok(part)
        };
        if take(4)? != b"DQFL" {
            return Err("floor grid: not a DQFL file".into());
        }
        let u32_at = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let f32_at = |b: &[u8]| f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let version = u32_at(take(4)?);
        if !(1..=2).contains(&version) {
            return Err(format!("floor grid: unknown version {version}"));
        }
        let x0 = f32_at(take(4)?);
        let z0 = f32_at(take(4)?);
        let cell = f32_at(take(4)?);
        let width = u32_at(take(4)?);
        let height = u32_at(take(4)?);
        let count = u32_at(take(4)?) as usize;
        if cell.is_nan() || cell <= 0.0 || width == 0 || height == 0 {
            return Err("floor grid: empty".into());
        }
        let columns = (width as usize) * (height as usize);
        let raw = take((columns + 1) * 4)?;
        let offsets: Vec<u32> = (0..=columns).map(|i| u32_at(&raw[i * 4..])).collect();
        if offsets.first() != Some(&0)
            || offsets.last().copied() != Some(count as u32)
            || offsets.windows(2).any(|w| w[0] > w[1])
        {
            return Err("floor grid: bad column offsets".into());
        }
        let stride = if version == 2 { 18 } else { 16 };
        let raw = take(count * stride)?;
        let spans = raw
            .chunks_exact(stride)
            .map(|b| FloorSpan {
                floor: f32_at(&b[0..4]),
                ceiling: f32_at(&b[4..8]),
                slope: [f32_at(&b[8..12]), f32_at(&b[12..16])],
                room: if version == 2 {
                    Some(u16::from_le_bytes([b[16], b[17]])).filter(|&r| r != u16::MAX)
                } else {
                    None
                },
            })
            .collect();
        Ok(Self {
            x0,
            z0,
            cell,
            width,
            height,
            step: AUTHORED_STEP,
            margin: AUTHORED_MARGIN,
            head: AUTHORED_HEAD,
            offsets,
            spans,
        })
    }

    /// The column under a spot of the plan, and its centre.
    fn column(&self, x: f32, z: f32) -> Option<(usize, f32, f32)> {
        let cx = ((x - self.x0) / self.cell).floor();
        let cz = ((z - self.z0) / self.cell).floor();
        if cx < 0.0 || cz < 0.0 || cx >= self.width as f32 || cz >= self.height as f32 {
            return None;
        }
        let (col, row) = (cx as usize, cz as usize);
        Some((
            row * self.width as usize + col,
            self.x0 + (cx + 0.5) * self.cell,
            self.z0 + (cz + 0.5) * self.cell,
        ))
    }

    /// The floors under a spot of the plan, top to bottom, each with its height right there.
    pub fn floors(&self, x: f32, z: f32) -> impl Iterator<Item = (f32, &FloorSpan)> {
        let (range, cx, cz) = match self.column(x, z) {
            Some((i, cx, cz)) => (
                self.offsets[i] as usize..self.offsets[i + 1] as usize,
                cx,
                cz,
            ),
            None => (0..0, 0.0, 0.0),
        };
        self.spans[range]
            .iter()
            .map(move |s| (s.floor + s.slope[0] * (x - cx) + s.slope[1] * (z - cz), s))
    }

    /// The floor a body standing at `feet` (and `body` tall) can be on at (x, z): the highest one no
    /// more than a step over its feet with room over it for the body.
    pub fn surface(&self, x: f32, z: f32, feet: f32, body: f32) -> Option<f32> {
        let mut best: Option<f32> = None;
        for (floor, span) in self.floors(x, z) {
            if floor <= feet + self.step + 0.015
                && span.ceiling >= feet.max(floor) + body - 0.01
                && span.ceiling > span.floor
                && best.is_none_or(|v| floor > v)
            {
                best = Some(floor);
            }
        }
        best
    }

    /// The room of the floor under a point: the highest floor at most half a metre over it. A
    /// floor of no room (the top of a crate, a ledge between places) takes the room of the floors
    /// round it, out to about a metre.
    pub fn room_at(&self, x: f32, y: f32, z: f32) -> Option<usize> {
        let here = |x: f32, z: f32| {
            self.floors(x, z)
                .filter(|(floor, _)| *floor <= y + 0.5 && *floor >= y - 2.5)
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .and_then(|(_, span)| span.room.map(usize::from))
        };
        if let Some(room) = here(x, z) {
            return Some(room);
        }
        for ring in 1..=4 {
            let r = ring as f32 * self.cell;
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                if let Some(room) = here(x + a.cos() * r, z + a.sin() * r) {
                    return Some(room);
                }
            }
        }
        None
    }

    /// How many floors the grid holds.
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Whether the grid holds no floor at all.
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// A walk over the floors from `start` to `goal` (feet positions) for a body `radius` wide: A*
    /// over the columns, eight ways round, at most a step up or down from one column to the next,
    /// never cutting a corner, kept within `reach` metres of `start`. Returns where it turns (its
    /// straight runs pulled tight), the last point being `goal` itself, or nothing when no walk is
    /// found (the goal out of reach, or on no floor near it).
    pub fn path(&self, start: Vec3, goal: Vec3, radius: f32, reach: f32) -> Vec<Vec3> {
        self.path_avoiding(start, goal, radius, reach, &|_| false)
    }

    /// As [`path`](Self::path), keeping clear of spots `blocked` says a body stood on would hit
    /// (things the grid does not hold: a chest, a stone set down on the floor).
    pub fn path_avoiding(
        &self,
        start: Vec3,
        goal: Vec3,
        radius: f32,
        reach: f32,
        blocked: &dyn Fn(Vec3) -> bool,
    ) -> Vec<Vec3> {
        let Some(from) = self.node_near(start, radius) else {
            return Vec::new();
        };
        let Some(to) = self.node_near(goal, radius) else {
            return Vec::new();
        };
        if from == to {
            return vec![goal];
        }
        let width = self.width as usize;
        let goal_column = (to.1 % width, to.1 / width);
        let heuristic = |column: usize| {
            let dx = (column % width).abs_diff(goal_column.0) as u32;
            let dz = (column / width).abs_diff(goal_column.1) as u32;
            10 * dx.max(dz) + 4 * dx.min(dz)
        };
        let window = (reach / self.cell).ceil() as usize;
        let (sx, sz) = (from.1 % width, from.1 / width);
        let inside = |column: usize| {
            (column % width).abs_diff(sx) <= window && (column / width).abs_diff(sz) <= window
        };
        let mut fits = HashMap::new();
        let mut best: HashMap<usize, (u32, usize)> = HashMap::new();
        let mut open = BinaryHeap::new();
        best.insert(from.0, (0, usize::MAX));
        open.push(Reverse((heuristic(from.1), 0u32, from.0, from.1)));
        let mut found = false;
        let mut expanded = 0;
        while let Some(Reverse((_, g, span, column))) = open.pop() {
            if best.get(&span).is_some_and(|&(known, _)| known < g) {
                continue;
            }
            if span == to.0 {
                found = true;
                break;
            }
            expanded += 1;
            if expanded > 40_000 {
                break;
            }
            let floor = self.spans[span].floor;
            let ceiling = self.spans[span].ceiling;
            for (dx, dz) in NEIGHBOURS {
                let Some(next) = self.step_column(column, dx, dz) else {
                    continue;
                };
                if !inside(next) {
                    continue;
                }
                // a diagonal only past two open sides (no corner cut through a wall's end)
                if dx != 0 && dz != 0 {
                    let mut open_side = |ddx: i32, ddz: i32| {
                        self.step_column(column, ddx, ddz)
                            .and_then(|c| self.next_span(c, floor, ceiling))
                            .is_some_and(|s| self.fits(s, radius, &mut fits))
                    };
                    if !open_side(dx, 0) || !open_side(0, dz) {
                        continue;
                    }
                }
                let Some(next_span) = self.next_span(next, floor, ceiling) else {
                    continue;
                };
                if !self.fits(next_span, radius, &mut fits)
                    || next_span != to.0 && blocked(self.span_point(next_span))
                {
                    continue;
                }
                let rise = (self.spans[next_span].floor - floor).abs();
                let cost = g + if dx != 0 && dz != 0 { 14 } else { 10 } + (rise * 20.0) as u32;
                if best
                    .get(&next_span)
                    .is_some_and(|&(known, _)| known <= cost)
                {
                    continue;
                }
                best.insert(next_span, (cost, span));
                open.push(Reverse((cost + heuristic(next), cost, next_span, next)));
            }
        }
        if !found {
            return Vec::new();
        }
        // back from the goal, then pulled tight from the start
        let mut nodes = vec![to.0];
        while let Some(&(_, previous)) = best.get(nodes.last().unwrap()) {
            if previous == usize::MAX {
                break;
            }
            nodes.push(previous);
        }
        nodes.reverse();
        let points: Vec<Vec3> = nodes.iter().map(|&s| self.span_point(s)).collect();
        let mut out = Vec::new();
        let mut anchor = start;
        let mut k = 0;
        while k + 1 < points.len() {
            let mut far = k + 1;
            while far + 1 < points.len()
                && self.clear_walk_avoiding(anchor, points[far + 1], radius, blocked)
            {
                far += 1;
            }
            anchor = points[far];
            out.push(anchor);
            k = far;
        }
        if let Some(last) = out.last_mut() {
            if (last.y - goal.y).abs() <= self.step
                && self.clear_walk_avoiding(*last, goal, radius, blocked)
            {
                *last = goal;
            }
        }
        out
    }

    /// The column `dx`, `dz` columns over from `column`, if the grid goes that far.
    fn step_column(&self, column: usize, dx: i32, dz: i32) -> Option<usize> {
        let width = self.width as i32;
        let x = (column as i32 % width) + dx;
        let z = (column as i32 / width) + dz;
        (x >= 0 && z >= 0 && x < width && z < self.height as i32).then(|| (z * width + x) as usize)
    }

    /// The floor of `column` one walks onto from a floor at `floor` (under `ceiling`): within a
    /// step of it, with room for the body between the two.
    fn next_span(&self, column: usize, floor: f32, ceiling: f32) -> Option<usize> {
        (self.offsets[column] as usize..self.offsets[column + 1] as usize)
            .filter(|&s| {
                let span = &self.spans[s];
                (span.floor - floor).abs() <= self.step
                    && span.ceiling.min(ceiling) - span.floor.max(floor) >= self.head - 0.01
                    && span.ceiling > span.floor
            })
            .max_by(|&a, &b| self.spans[a].floor.total_cmp(&self.spans[b].floor))
    }

    /// Whether a body `radius` wide fits on floor `span`: the bake already keeps its floors the
    /// margin from walls; a wider body needs the columns round it to be floor too.
    fn fits(&self, span: usize, radius: f32, known: &mut HashMap<usize, bool>) -> bool {
        if let Some(&fit) = known.get(&span) {
            return fit;
        }
        let rings = ((radius - self.margin) / self.cell).ceil().max(0.0) as i32;
        let column = self.column_of(span);
        let FloorSpan { floor, ceiling, .. } = self.spans[span];
        let fit = (-rings..=rings).all(|dz| {
            (-rings..=rings).all(|dx| {
                (dx == 0 && dz == 0)
                    || self
                        .step_column(column, dx, dz)
                        .and_then(|c| self.next_span(c, floor, ceiling))
                        .is_some()
            })
        });
        known.insert(span, fit);
        fit
    }

    /// The column a floor belongs to.
    fn column_of(&self, span: usize) -> usize {
        self.offsets.partition_point(|&o| o as usize <= span) - 1
    }

    /// The middle of a floor, on it.
    fn span_point(&self, span: usize) -> Vec3 {
        let column = self.column_of(span);
        let width = self.width as usize;
        Vec3::new(
            self.x0 + ((column % width) as f32 + 0.5) * self.cell,
            self.spans[span].floor,
            self.z0 + ((column / width) as f32 + 0.5) * self.cell,
        )
    }

    /// The floor under feet at `p` (or the nearest one round it a body fits on), with its column.
    fn node_near(&self, p: Vec3, radius: f32) -> Option<(usize, usize)> {
        let mut fits = HashMap::new();
        let (here, _, _) = self.column(p.x, p.z)?;
        let mut best: Option<(f32, usize, usize)> = None;
        for ring in 0..=3i32 {
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dz.abs()) != ring {
                        continue;
                    }
                    let Some(column) = self.step_column(here, dx, dz) else {
                        continue;
                    };
                    for s in self.offsets[column] as usize..self.offsets[column + 1] as usize {
                        let span = &self.spans[s];
                        if span.floor > p.y + self.step + 0.015
                            || span.floor < p.y - 1.0
                            || span.ceiling - span.floor < self.head - 0.01
                            || !self.fits(s, radius, &mut fits)
                        {
                            continue;
                        }
                        let d = (dx * dx + dz * dz) as f32 + (span.floor - p.y).abs();
                        if best.is_none_or(|(b, _, _)| d < b) {
                            best = Some((d, s, column));
                        }
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(_, s, c)| (s, c))
    }

    /// Whether a body walks straight from `a` to `b`: floor all along within a step from one spot
    /// to the next, and room either side for its `radius`.
    pub fn clear_walk(&self, a: Vec3, b: Vec3, radius: f32) -> bool {
        self.clear_walk_avoiding(a, b, radius, &|_| false)
    }

    /// As [`clear_walk`](Self::clear_walk), every spot of the way also clear of what `blocked` says.
    pub fn clear_walk_avoiding(
        &self,
        a: Vec3,
        b: Vec3,
        radius: f32,
        blocked: &dyn Fn(Vec3) -> bool,
    ) -> bool {
        let flat = Vec3::new(b.x - a.x, 0.0, b.z - a.z);
        let length = flat.length();
        let side = if length > 1.0e-4 {
            Vec3::new(-flat.z / length, 0.0, flat.x / length) * (radius - self.margin).max(0.0)
        } else {
            Vec3::ZERO
        };
        let steps = (length / (self.cell * 0.5)).ceil().max(1.0) as usize;
        [Vec3::ZERO, side, -side].iter().all(|&offset| {
            let mut feet = a.y;
            (1..=steps).all(|i| {
                let p = a + (b - a) * (i as f32 / steps as f32) + offset;
                match self.surface(p.x, p.z, feet, self.head) {
                    Some(y)
                        if (y - feet).abs() <= self.step && !blocked(Vec3::new(p.x, y, p.z)) =>
                    {
                        feet = y;
                        true
                    }
                    _ => false,
                }
            })
        })
    }
}

/// The eight columns round one.
const NEIGHBOURS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn span(floor: f32, ceiling: f32, room: Option<u16>) -> FloorSpan {
        FloorSpan {
            floor,
            ceiling,
            slope: [0.0, 0.0],
            room,
        }
    }

    /// Two columns: a hall over a cellar, and a column with a post in it.
    fn sample() -> FloorGrid {
        FloorGrid::new(
            0.0,
            0.0,
            1.0,
            2,
            1,
            vec![
                vec![span(0.0, 4.0, Some(0)), span(-5.0, -1.0, Some(1))],
                vec![span(0.0, 0.0, Some(0))],
            ],
        )
    }

    #[test]
    fn the_floor_under_the_feet_wins_not_the_highest() {
        let grid = sample();
        assert_eq!(grid.surface(0.5, 0.5, 0.0, 1.78), Some(0.0));
        assert_eq!(grid.surface(0.5, 0.5, -5.0, 1.78), Some(-5.0));
        assert_eq!(grid.room_at(0.5, 0.0, 0.5), Some(0));
        assert_eq!(grid.room_at(0.5, -4.0, 0.5), Some(1));
    }

    #[test]
    fn a_post_and_the_outside_are_no_floor() {
        let grid = sample();
        assert_eq!(grid.surface(1.5, 0.5, 0.0, 1.78), None);
        assert_eq!(grid.surface(-0.5, 0.5, 0.0, 1.78), None);
        assert_eq!(grid.surface(0.5, 1.5, 0.0, 1.78), None);
    }

    #[test]
    fn a_slope_rises_across_its_column() {
        let grid = FloorGrid::new(
            0.0,
            0.0,
            2.0,
            1,
            1,
            vec![vec![FloorSpan {
                floor: 1.0,
                ceiling: 9.0,
                slope: [0.5, 0.0],
                room: None,
            }]],
        );
        let low = grid.surface(0.2, 1.0, 1.0, 1.78).unwrap();
        let high = grid.surface(1.8, 1.0, 1.3, 1.78).unwrap();
        assert!((low - 0.6).abs() < 1e-5 && (high - 1.4).abs() < 1e-5);
    }

    #[test]
    fn a_baked_file_reads_back() {
        let mut bytes = b"DQFL".to_vec();
        bytes.extend(2u32.to_le_bytes());
        for v in [0.0f32, 0.0, 1.0] {
            bytes.extend(v.to_le_bytes());
        }
        for v in [1u32, 1, 2] {
            bytes.extend(v.to_le_bytes());
        }
        for v in [0u32, 2] {
            bytes.extend(v.to_le_bytes());
        }
        for (floor, ceiling, room) in [(0.0f32, 3.0f32, 7u16), (-4.0, -1.0, u16::MAX)] {
            for v in [floor, ceiling, 0.0, 0.0] {
                bytes.extend(v.to_le_bytes());
            }
            bytes.extend(room.to_le_bytes());
        }
        let grid = FloorGrid::from_bytes(&bytes).unwrap();
        assert_eq!(grid.len(), 2);
        assert_eq!(grid.room_at(0.5, 0.2, 0.5), Some(7));
        assert_eq!(grid.room_at(0.5, -3.0, 0.5), None);
        assert!(FloorGrid::from_bytes(&bytes[..30]).is_err());
    }

    /// A room 6 m square on a 0.25 m grid with a wall across its middle, open at one end: the
    /// walk from one side to the other goes round the wall's end, never through it.
    #[test]
    fn a_walk_goes_round_a_wall_not_through_it() {
        let (w, h) = (24u32, 24u32);
        let columns = (0..w * h)
            .map(|i| {
                let (x, z) = (i % w, i / w);
                let wall = (11..=12).contains(&z) && x < 18;
                vec![span(0.0, if wall { 0.0 } else { 4.0 }, None)]
            })
            .collect();
        let grid = FloorGrid::new(0.0, 0.0, 0.25, w, h, columns);
        let start = Vec3::new(1.0, 0.0, 1.0);
        let goal = Vec3::new(1.0, 0.0, 5.0);
        assert!(
            !grid.clear_walk(start, goal, 0.4),
            "the wall stands between"
        );
        let path = grid.path(start, goal, 0.4, 10.0);
        assert!(!path.is_empty(), "a walk round the wall");
        assert_eq!(*path.last().unwrap(), goal);
        let mut from = start;
        for &to in &path {
            assert!(
                grid.clear_walk(from, to, 0.4),
                "{from:?} -> {to:?} is walkable"
            );
            from = to;
        }
        assert!(
            path.iter().any(|p| p.x > 4.5),
            "it goes round the wall's open end: {path:?}"
        );
        // out of reach, no walk
        assert!(grid.path(start, goal, 0.4, 2.0).is_empty());
    }
}
