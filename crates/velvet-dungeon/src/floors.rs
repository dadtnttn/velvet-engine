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
    offsets: Vec<u32>,
    spans: Vec<FloorSpan>,
}

/// A modelled level's step: the prototypes' capsule walks over about this much.
pub const AUTHORED_STEP: f32 = 0.45;
/// The bakes' wall margin (the mine's floors.py keeps floors 0.18 m from walls).
pub const AUTHORED_MARGIN: f32 = 0.18;

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
}

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
        for v in [2u32] {
            bytes.extend(v.to_le_bytes());
        }
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
}
