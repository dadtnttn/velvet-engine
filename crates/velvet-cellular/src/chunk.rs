//! Fixed-size chunk of cells — unit of streaming and parallel sim.

use serde::{Deserialize, Serialize};

use crate::cell::Cell;

/// Default chunk edge length in cells (power of two).
pub const CHUNK_SIZE: usize = 64;
/// Cells per chunk.
pub const CHUNK_CELLS: usize = CHUNK_SIZE * CHUNK_SIZE;

/// Chunk coordinate in world chunk space (not cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ChunkCoord {
    /// Chunk X.
    pub x: i32,
    /// Chunk Y (up is +Y in world; sim gravity pulls −Y).
    pub y: i32,
}

impl ChunkCoord {
    /// Create.
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Containing chunk for a world cell.
    pub fn from_cell(cx: i32, cy: i32) -> Self {
        Self {
            x: cx.div_euclid(CHUNK_SIZE as i32),
            y: cy.div_euclid(CHUNK_SIZE as i32),
        }
    }

    /// Origin cell (bottom-left of chunk) in world coords.
    pub fn origin_cell(self) -> (i32, i32) {
        (self.x * CHUNK_SIZE as i32, self.y * CHUNK_SIZE as i32)
    }
}

/// One simulation chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Coordinate.
    pub coord: ChunkCoord,
    /// Row-major cells: index = ly * CHUNK_SIZE + lx (local).
    pub cells: Vec<Cell>,
    /// Generation / dirty revision for render.
    pub revision: u64,
    /// Whether any cell moved last step.
    pub active: bool,
    /// Sleep until neighbor wakes.
    pub sleeping: bool,
}

impl Chunk {
    /// Empty air chunk.
    pub fn empty(coord: ChunkCoord) -> Self {
        Self {
            coord,
            cells: vec![Cell::air(); CHUNK_CELLS],
            revision: 0,
            active: true,
            sleeping: false,
        }
    }

    /// Local index.
    #[inline]
    pub fn idx(lx: usize, ly: usize) -> usize {
        ly * CHUNK_SIZE + lx
    }

    /// Get local.
    #[inline]
    pub fn get(&self, lx: usize, ly: usize) -> Cell {
        self.cells[Self::idx(lx, ly)]
    }

    /// Get mut local.
    #[inline]
    pub fn get_mut(&mut self, lx: usize, ly: usize) -> &mut Cell {
        let i = Self::idx(lx, ly);
        &mut self.cells[i]
    }

    /// Set and bump revision.
    pub fn set(&mut self, lx: usize, ly: usize, cell: Cell) {
        self.cells[Self::idx(lx, ly)] = cell;
        self.revision = self.revision.wrapping_add(1);
        self.active = true;
        self.sleeping = false;
    }

    /// Fill all with cell.
    pub fn fill(&mut self, cell: Cell) {
        for c in &mut self.cells {
            *c = cell;
        }
        self.revision = self.revision.wrapping_add(1);
        self.active = true;
        self.sleeping = false;
    }

    /// Count non-air.
    pub fn solid_count(&self) -> usize {
        self.cells.iter().filter(|c| !c.is_air()).count()
    }
}
