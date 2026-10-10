//! Biome classification: height + slope + moisture → material weights.
//!
//! Five materials (water, sand, grass, rock, snow) blend with smoothstep
//! bands; [`BiomeWeights`] always sums to ~1 and doubles as splat input.

/// Material count.
pub const BIOME_COUNT: usize = 5;
/// Water, sand, grass, rock, snow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Biome {
    /// Below sea level.
    Water = 0,
    /// Beaches and dry basins.
    Sand = 1,
    /// Vegetated mid slopes.
    Grass = 2,
    /// Steep rock.
    Rock = 3,
    /// High altitude.
    Snow = 4,
}

/// Per-material blend weights (sum ≈ 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeWeights {
    /// `[water, sand, grass, rock, snow]`.
    pub weights: [f32; BIOME_COUNT],
}

impl BiomeWeights {
    /// Dominant material.
    pub fn dominant(&self) -> Biome {
        let mut best = 0;
        for i in 1..BIOME_COUNT {
            if self.weights[i] > self.weights[best] {
                best = i;
            }
        }
        match best {
            0 => Biome::Water,
            1 => Biome::Sand,
            2 => Biome::Grass,
            3 => Biome::Rock,
            _ => Biome::Snow,
        }
    }

    /// Weight sum (for normalization checks).
    pub fn sum(&self) -> f32 {
        self.weights.iter().sum()
    }
}

/// Extra seed xor for the moisture field (keeps it independent).
pub const MOISTURE_SEED_XOR: u64 = 0x51AB_3F09_D2C4_E771;

/// Classify one vertex.
///
/// - `h`: height in units, `sea_level` reference.
/// - `slope`: rise over run (0 = flat).
/// - `moisture`: 0..=1 noise (dry..wet).
/// - `relief`: total height span, scales the bands.
pub fn classify(h: f32, slope: f32, moisture: f32, sea_level: f32, relief: f32) -> BiomeWeights {
    let relief = relief.max(1.0);
    let hn = (h - sea_level) / relief; // ~0..1 above sea
    let moisture = moisture.clamp(0.0, 1.0);
    // Water below sea; sand band just above; grass mid; rock steep/high; snow caps.
    let water = smoothstep(0.005, -0.005, hn);
    let sand = smoothstep(-0.005, 0.01, hn) * (1.0 - smoothstep(0.03, 0.07, hn));
    let snow = smoothstep(0.55, 0.7, hn + slope * 0.15);
    let rock = smoothstep(0.45, 0.9, slope) * (1.0 - snow);
    let mut grass = (1.0 - water) * (1.0 - sand) * (1.0 - rock) * (1.0 - snow);
    // Dryness pushes grass toward sand/rock.
    grass *= 0.35 + 0.65 * moisture;
    let mut weights = [water, sand, grass, rock, snow];
    // Sand absorbs the dry remainder so the sum stays ~1.
    weights[1] += (1.0 - weights.iter().sum::<f32>()).max(0.0);
    let sum = weights.iter().sum::<f32>().max(1e-6);
    for w in &mut weights {
        *w /= sum;
    }
    BiomeWeights { weights }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Fill `mesh.biomes` from a field: slope from vertex normals, moisture
/// from an independent fBm field.
pub fn paint_biomes(
    mesh: &mut crate::mesh::TerrainMesh,
    field: &crate::graph::Heightfield,
    sea_level: f32,
    moisture_seed: u64,
) {
    use crate::noise::Fbm;
    let moisture = Fbm::new(moisture_seed, 3, 2.0, 0.5);
    let (_, hi_raw) = field.range();
    let (lo_raw, _) = field.range();
    let relief = (hi_raw - lo_raw).max(1.0);
    let n = mesh.vertex_count();
    mesh.biomes.clear();
    mesh.biomes.reserve(n);
    for v in 0..n {
        let nx = mesh.normals[v * 3];
        let ny = mesh.normals[v * 3 + 1].max(0.05);
        let nz = mesh.normals[v * 3 + 2];
        let slope = (nx * nx + nz * nz).sqrt() / ny;
        let h = mesh.positions[v * 3 + 1];
        // Moisture sampled in vertex-index space (stable, no UV needed).
        let m = moisture.sample(v as f32 * 0.13, v as f32 * -0.07) * 0.5 + 0.5;
        mesh.biomes
            .push(classify(h, slope, m, sea_level, relief).dominant() as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_covers_all_biomes_on_an_island() {
        use crate::graph::{TerrainGraph, TerrainParams};
        use crate::mesh::build_mesh;
        let field = TerrainGraph::new(TerrainParams {
            seed: 5,
            resolution: 33,
            ..Default::default()
        })
        .bake();
        let mut mesh = build_mesh(&field);
        super::paint_biomes(&mut mesh, &field, 0.0, 77);
        assert_eq!(mesh.biomes.len(), mesh.vertex_count());
        let mut seen = [false; super::BIOME_COUNT];
        for b in &mesh.biomes {
            seen[*b as usize] = true;
        }
        // Island with relief: water, sand/grass, rock, snow all appear.
        assert!(seen.iter().all(|s| *s), "{seen:?}");
    }

    #[test]
    fn weights_sum_to_one_and_make_sense() {
        let relief = 60.0;
        // Deep water.
        let w = classify(-5.0, 0.0, 0.5, 0.0, relief);
        assert_eq!(w.dominant(), Biome::Water);
        // Beach.
        let w = classify(0.5, 0.02, 0.3, 0.0, relief);
        assert!(w.dominant() == Biome::Sand || w.dominant() == Biome::Grass);
        // Meadow.
        let w = classify(15.0, 0.05, 0.8, 0.0, relief);
        assert_eq!(w.dominant(), Biome::Grass);
        // Cliff.
        let w = classify(20.0, 1.2, 0.5, 0.0, relief);
        assert_eq!(w.dominant(), Biome::Rock);
        // Peak.
        let w = classify(55.0, 0.2, 0.5, 0.0, relief);
        assert_eq!(w.dominant(), Biome::Snow);
        // Normalization everywhere on a sweep.
        for i in 0..=20 {
            let h = -10.0 + i as f32 * 4.0;
            for s in [0.0, 0.3, 0.8] {
                let w = classify(h, s, 0.5, 0.0, relief);
                assert!((w.sum() - 1.0).abs() < 1e-4, "{} {} {}", h, s, w.sum());
                assert!(w.weights.iter().all(|v| (0.0..=1.0).contains(v)));
            }
        }
    }
}
