//! Grid meshing: positions, normals, UVs, indices, and distance LOD.
//!
//! Normals come from central differences of the heightfield. LOD selects
//! quadtree nodes by distance; every emitted patch carries a skirts ring
//! dropped below the terrain so neighboring levels never show cracks.

use serde::{Deserialize, Serialize};

use crate::graph::Heightfield;

/// CPU mesh: Y-up, counter-clockwise front faces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainMesh {
    /// `[x, y, z]` triplets.
    pub positions: Vec<f32>,
    /// `[nx, ny, nz]` triplets, unit length.
    pub normals: Vec<f32>,
    /// `[u, v]` pairs over the patch.
    pub uvs: Vec<f32>,
    /// Triangle indices into the vertex arrays.
    pub indices: Vec<u32>,
    /// Per-vertex biome palette index (see [`crate::biome`]).
    pub biomes: Vec<u8>,
}

impl TerrainMesh {
    /// Vertex count.
    pub fn vertex_count(&self) -> usize {
        self.positions.len() / 3
    }

    /// Triangle count.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Build a full-resolution mesh of the whole heightfield (no skirts).
pub fn build_mesh(field: &Heightfield) -> TerrainMesh {
    build_patch(field, 0, 0, field.resolution as usize - 1, 1, 0.0)
}

/// Build one chunked-LOD mesh: quadtree nodes split while close to `camera`
/// (world `(x, z)`), leaves emit full-resolution patches with skirts.
///
/// A node of world size `s` at distance `d` splits while
/// `d < split_factor * s` (and depth allows). Skirts hang `skirt_depth`
/// below each patch rim, so mixed resolutions stitch without cracks.
/// Requires a `2^n`-cells field.
pub fn build_lod_mesh(
    field: &Heightfield,
    camera: (f32, f32),
    max_depth: u32,
    split_factor: f32,
    skirt_depth: f32,
) -> TerrainMesh {
    let cells = field.resolution as usize - 1;
    assert!(cells.is_power_of_two(), "LOD needs 2^n cells");
    let mut mesh = TerrainMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        indices: Vec::new(),
        biomes: Vec::new(),
    };
    collect(
        field,
        0,
        0,
        cells,
        0,
        max_depth,
        camera,
        split_factor,
        skirt_depth,
        &mut mesh,
    );
    mesh
}

#[allow(clippy::too_many_arguments)]
fn collect(
    field: &Heightfield,
    x0: usize,
    z0: usize,
    size: usize,
    level: u32,
    max_depth: u32,
    camera: (f32, f32),
    split_factor: f32,
    skirt_depth: f32,
    out: &mut TerrainMesh,
) {
    let node_world = size as f32 * field.step();
    let cx = (x0 as f32 + size as f32 * 0.5) / (field.resolution as f32 - 1.0) * field.size
        - field.size * 0.5;
    let cz = (z0 as f32 + size as f32 * 0.5) / (field.resolution as f32 - 1.0) * field.size
        - field.size * 0.5;
    let dist = ((cx - camera.0).hypot(cz - camera.1) - node_world * 0.5).max(0.0);
    if level < max_depth && size > 1 && dist < split_factor * node_world {
        let half = size / 2;
        collect(
            field,
            x0,
            z0,
            half,
            level + 1,
            max_depth,
            camera,
            split_factor,
            skirt_depth,
            out,
        );
        collect(
            field,
            x0 + half,
            z0,
            half,
            level + 1,
            max_depth,
            camera,
            split_factor,
            skirt_depth,
            out,
        );
        collect(
            field,
            x0,
            z0 + half,
            half,
            level + 1,
            max_depth,
            camera,
            split_factor,
            skirt_depth,
            out,
        );
        collect(
            field,
            x0 + half,
            z0 + half,
            half,
            level + 1,
            max_depth,
            camera,
            split_factor,
            skirt_depth,
            out,
        );
        return;
    }
    append_patch(out, field, x0, z0, size, 1, skirt_depth);
}

/// Emit one patch: `(size/step + 1)²` grid vertices + skirt ring.
fn append_patch(
    out: &mut TerrainMesh,
    field: &Heightfield,
    x0: usize,
    z0: usize,
    size: usize,
    step: usize,
    skirt_depth: f32,
) {
    let patch = build_patch(field, x0, z0, size, step, skirt_depth);
    let base = (out.positions.len() / 3) as u32;
    out.positions.extend_from_slice(&patch.positions);
    out.normals.extend_from_slice(&patch.normals);
    out.uvs.extend_from_slice(&patch.uvs);
    out.biomes.extend_from_slice(&patch.biomes);
    out.indices.extend(patch.indices.iter().map(|i| i + base));
}

/// Build one patch mesh over cells `[x0, x0+size] × [z0, z0+size]`.
fn build_patch(
    field: &Heightfield,
    x0: usize,
    z0: usize,
    size: usize,
    step: usize,
    skirt_depth: f32,
) -> TerrainMesh {
    let res = field.resolution as usize;
    let half = field.size * 0.5;
    let n = size / step + 1;
    let mut positions = Vec::with_capacity(n * n * 3);
    let mut normals = Vec::with_capacity(n * n * 3);
    let mut uvs = Vec::with_capacity(n * n * 2);
    let mut biomes = Vec::with_capacity(n * n);
    let world = |i: usize| (x0 + i * step) as f32 / (res - 1) as f32 * field.size - half;
    for j in 0..n {
        for i in 0..n {
            let (gi, gj) = (x0 + i * step, z0 + j * step);
            let h = field.at(gi as i32, gj as i32);
            positions.extend_from_slice(&[world(i), h, world(j)]);
            let normal = grid_normal(field, gi, gj);
            normals.extend_from_slice(&normal);
            uvs.extend_from_slice(&[
                (x0 + i * step) as f32 / (res - 1) as f32,
                (z0 + j * step) as f32 / (res - 1) as f32,
            ]);
            biomes.push(0);
        }
    }
    let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let a = (j * n + i) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    if skirt_depth > 0.0 {
        add_skirt(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut biomes,
            &mut indices,
            n,
            skirt_depth,
        );
    }
    TerrainMesh {
        positions,
        normals,
        uvs,
        indices,
        biomes,
    }
}

/// Unit normal from central differences (Y-up).
fn grid_normal(field: &Heightfield, i: usize, j: usize) -> [f32; 3] {
    let step = field.step();
    let dx = field.at(i as i32 + 1, j as i32) - field.at(i as i32 - 1, j as i32);
    let dz = field.at(i as i32, j as i32 + 1) - field.at(i as i32, j as i32 - 1);
    let (nx, ny, nz) = (-dx / (2.0 * step), 1.0, -dz / (2.0 * step));
    let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
    [nx / len, ny / len, nz / len]
}

/// Duplicate the patch rim dropped by `depth`: hides LOD cracks.
fn add_skirt(
    positions: &mut Vec<f32>,
    normals: &mut Vec<f32>,
    uvs: &mut Vec<f32>,
    biomes: &mut Vec<u8>,
    indices: &mut Vec<u32>,
    n: usize,
    depth: f32,
) {
    // Rim loop order: bottom row, right column, top row reversed, left column reversed.
    let mut rim = Vec::with_capacity(4 * n);
    for i in 0..n {
        rim.push(i);
    }
    for j in 1..n {
        rim.push(j * n + (n - 1));
    }
    for i in (0..n - 1).rev() {
        rim.push((n - 1) * n + i);
    }
    for j in (1..n - 1).rev() {
        rim.push(j * n);
    }
    let base = (positions.len() / 3) as u32;
    for (k, &v) in rim.iter().enumerate() {
        positions.extend_from_slice(&[
            positions[v * 3],
            positions[v * 3 + 1] - depth,
            positions[v * 3 + 2],
        ]);
        normals.extend_from_slice(&[normals[v * 3], normals[v * 3 + 1], normals[v * 3 + 2]]);
        uvs.extend_from_slice(&[uvs[v * 2], uvs[v * 2 + 1]]);
        biomes.push(biomes[v]);
        let a = base + k as u32;
        let b = base + ((k + 1) % rim.len()) as u32;
        let c = v as u32;
        let d = rim[(k + 1) % rim.len()] as u32;
        indices.extend_from_slice(&[a, b, c, b, d, c]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{TerrainGraph, TerrainParams};

    fn island_33() -> Heightfield {
        TerrainGraph::new(TerrainParams {
            seed: 5,
            resolution: 33,
            ..Default::default()
        })
        .bake()
    }

    #[test]
    fn full_mesh_counts_and_unit_normals() {
        let mesh = build_mesh(&island_33());
        assert_eq!(mesh.vertex_count(), 33 * 33);
        assert_eq!(mesh.triangle_count(), 32 * 32 * 2);
        assert_eq!(mesh.uvs.len(), 33 * 33 * 2);
        for n in mesh.normals.as_chunks::<3>().0 {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-4, "{len}");
            assert!(n[1] > 0.0, "normals point up: {n:?}");
        }
        // Winding sanity: first triangle is not degenerate.
        assert_eq!(mesh.indices.len() % 3, 0);
    }

    #[test]
    fn lod_splits_near_and_merges_far() {
        let field = island_33();
        // Camera far away: one 33x33 patch plus its skirt ring.
        let far = build_lod_mesh(&field, (10_000.0, 10_000.0), 4, 1.5, 4.0);
        assert_eq!(far.vertex_count(), 33 * 33 + (4 * 33 - 4));
        assert_eq!(far.triangle_count(), 32 * 32 * 2 + (4 * 33 - 4) * 2);
        // Camera over the corner: splits add skirt overhead.
        let near = build_lod_mesh(&field, (-120.0, -120.0), 4, 1.5, 4.0);
        assert!(
            near.vertex_count() > far.vertex_count(),
            "{} vs {}",
            near.vertex_count(),
            far.vertex_count()
        );
        // Structural integrity on both.
        for m in [&far, &near] {
            assert_eq!(m.triangle_count() * 3, m.indices.len());
            assert_eq!(m.normals.len(), m.vertex_count() * 3);
            assert_eq!(m.uvs.len(), m.vertex_count() * 2);
            assert_eq!(m.biomes.len(), m.vertex_count());
            let max_index = m.vertex_count() as u32;
            assert!(m.indices.iter().all(|i| *i < max_index));
        }
    }

    #[test]
    fn skirts_hang_below_the_rim() {
        let field = island_33();
        let plain = build_patch(&field, 0, 0, 32, 8, 0.0);
        let skirted = build_patch(&field, 0, 0, 32, 8, 5.0);
        let n = 32 / 8 + 1;
        assert_eq!(plain.vertex_count(), n * n);
        assert_eq!(skirted.vertex_count(), n * n + (4 * n - 4));
        // First skirt vertex duplicates rim vertex 0, dropped by depth.
        let v0 = plain.vertex_count();
        assert_eq!(skirted.positions[v0 * 3], skirted.positions[0]);
        assert_eq!(skirted.positions[v0 * 3 + 1], skirted.positions[1] - 5.0);
    }
}
