//! Chunk RLE compression for saves / network / streaming.

use crate::cell::{Cell, CellFlags, MaterialId};
use crate::chunk::{Chunk, ChunkCoord, CHUNK_CELLS, CHUNK_SIZE};

/// Run-length encoded chunk payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkRle {
    /// Coord.
    pub coord: ChunkCoord,
    /// Runs: (repeat, material, life, flags bits, temp quant 0..255).
    pub runs: Vec<(u16, u16, u16, u8, u8)>,
}

fn quant_temp(t: f32) -> u8 {
    ((t + 50.0) / 8.0).clamp(0.0, 255.0) as u8
}

fn dequant_temp(q: u8) -> f32 {
    q as f32 * 8.0 - 50.0
}

/// Compress chunk cells with RLE on material+life+flags+temp bucket.
pub fn compress_chunk(chunk: &Chunk) -> ChunkRle {
    let mut runs = Vec::new();
    if chunk.cells.is_empty() {
        return ChunkRle {
            coord: chunk.coord,
            runs,
        };
    }
    let mut cur_mat = chunk.cells[0].material.0;
    let mut cur_life = chunk.cells[0].life;
    let mut cur_flags = chunk.cells[0].flags.0;
    let mut cur_temp = quant_temp(chunk.cells[0].temp);
    let mut count: u16 = 1;
    for c in chunk.cells.iter().skip(1) {
        let tq = quant_temp(c.temp);
        if c.material.0 == cur_mat
            && c.life == cur_life
            && c.flags.0 == cur_flags
            && tq == cur_temp
            && count < u16::MAX
        {
            count += 1;
        } else {
            runs.push((count, cur_mat, cur_life, cur_flags, cur_temp));
            cur_mat = c.material.0;
            cur_life = c.life;
            cur_flags = c.flags.0;
            cur_temp = tq;
            count = 1;
        }
    }
    runs.push((count, cur_mat, cur_life, cur_flags, cur_temp));
    ChunkRle {
        coord: chunk.coord,
        runs,
    }
}

/// Decompress into a chunk.
pub fn decompress_chunk(rle: &ChunkRle) -> Chunk {
    let mut chunk = Chunk::empty(rle.coord);
    let mut i = 0usize;
    for &(count, mat, life, flags, tq) in &rle.runs {
        for _ in 0..count {
            if i >= CHUNK_CELLS {
                break;
            }
            chunk.cells[i] = Cell {
                material: MaterialId(mat),
                temp: dequant_temp(tq),
                pressure: 0.0,
                life,
                meta: 0,
                flags: CellFlags(flags),
            };
            i += 1;
        }
    }
    chunk.revision = 1;
    chunk.active = true;
    chunk
}

/// Byte size estimate of RLE vs raw.
pub fn compression_ratio(chunk: &Chunk) -> f32 {
    let raw = CHUNK_CELLS * std::mem::size_of::<Cell>();
    let rle = compress_chunk(chunk);
    let enc = rle.runs.len() * 8;
    if enc == 0 {
        return 1.0;
    }
    raw as f32 / enc as f32
}

/// Encode many chunks.
pub fn compress_world_chunks<'a>(chunks: impl Iterator<Item = &'a Chunk>) -> Vec<ChunkRle> {
    chunks.map(compress_chunk).collect()
}

/// Round-trip integrity check helper.
pub fn roundtrip_ok(chunk: &Chunk) -> bool {
    let r = compress_chunk(chunk);
    let back = decompress_chunk(&r);
    if back.cells.len() != chunk.cells.len() {
        return false;
    }
    chunk
        .cells
        .iter()
        .zip(back.cells.iter())
        .all(|(a, b)| a.material == b.material && a.life == b.life)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin::builtin_registry;
    use crate::world::{World, WorldConfig};

    #[test]
    fn rle_roundtrip_and_ratio() {
        let (reg, ids) = builtin_registry();
        let mut world = World::new(reg, WorldConfig::default());
        world.paint_rect(0, 0, CHUNK_SIZE as i32, 4, ids.stone);
        world.paint_rect(0, 4, 10, 10, ids.sand);
        let ch = world
            .chunk(ChunkCoord::new(0, 0))
            .cloned()
            .unwrap_or_else(|| Chunk::empty(ChunkCoord::new(0, 0)));
        assert!(roundtrip_ok(&ch));
        let ratio = compression_ratio(&ch);
        assert!(ratio > 1.0, "ratio={ratio}");
    }
}
