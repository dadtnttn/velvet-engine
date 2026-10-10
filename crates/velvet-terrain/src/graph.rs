//! Terrain height graphs: parameter sets sampling a heightfield.
//!
//! A [`TerrainGraph`] combines base fBm relief, ridged mountains masked by a
//! second field, domain warping and an island falloff into one height
//! function `h(x, z)` in world units. [`Heightfield`] bakes it to a grid.

use serde::{Deserialize, Serialize};

use crate::noise::{DomainWarp, Fbm};

/// Author parameters for one terrain (all serialized in `TerrainDoc`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainParams {
    /// Deterministic seed.
    pub seed: u64,
    /// Grid resolution (vertices per side, ≥ 2).
    pub resolution: u32,
    /// World size in units (XZ extent).
    pub size: f32,
    /// Base relief amplitude in units.
    pub base_height: f32,
    /// Base fBm octaves / frequency / gain.
    pub base_octaves: u32,
    pub base_frequency: f32,
    /// Mountain amplitude (masked by `mask_*`).
    pub mountain_height: f32,
    /// Ridged octaves.
    pub mountain_octaves: u32,
    /// Mask field octaves + threshold/softness for where mountains rise.
    pub mask_octaves: u32,
    pub mask_threshold: f32,
    pub mask_softness: f32,
    /// Domain-warp strength (0 = off).
    pub warp_strength: f32,
    /// Island falloff: 0 = infinite plain, >0 pulls edges to sea level.
    pub falloff: f32,
    /// Sea level in units (for biome classification, not clamped).
    pub sea_level: f32,
    /// Cordillera amplitude multiplier (0 = off).
    #[serde(default)]
    pub range_amount: f32,
    /// Z of the range axis, normalized -0.5..0.5.
    #[serde(default)]
    pub range_center: f32,
    /// Band half-width, normalized (full width = 2x).
    #[serde(default)]
    pub range_width: f32,
    /// Ridged crest height in units.
    #[serde(default)]
    pub range_height: f32,
    /// Plain flatten factor 0..=1 (1 = keep relief) past `plain_start`.
    #[serde(default)]
    pub plain_keep: f32,
    /// Normalized z where the plain begins.
    #[serde(default)]
    pub plain_start: f32,
    /// Lake center, normalized XZ.
    #[serde(default)]
    pub lake_center: [f32; 2],
    /// Lake radius, normalized.
    #[serde(default)]
    pub lake_radius: f32,
    /// Lake dig depth in units.
    #[serde(default)]
    pub lake_depth: f32,
    /// Still-water level for lake fill.
    #[serde(default)]
    pub water_level: f32,
}

impl Default for TerrainParams {
    fn default() -> Self {
        Self {
            seed: 1337,
            resolution: 129,
            size: 256.0,
            base_height: 18.0,
            base_octaves: 4,
            base_frequency: 3.0,
            mountain_height: 55.0,
            mountain_octaves: 4,
            mask_octaves: 2,
            mask_threshold: 0.15,
            mask_softness: 0.25,
            warp_strength: 0.35,
            falloff: 1.5,
            sea_level: 0.0,
            range_amount: 0.0,
            range_center: -0.28,
            range_width: 0.16,
            range_height: 95.0,
            plain_keep: 1.0,
            plain_start: 0.08,
            lake_center: [0.1, 0.3],
            lake_radius: 0.0,
            lake_depth: 14.0,
            water_level: 1.5,
        }
    }
}

impl TerrainParams {
    /// Cordillera + plain + lake composition: an east-west mountain range
    /// in the north, flattened farmland south, and a dug lake with water.
    pub fn cordillera(seed: u64) -> Self {
        Self {
            seed,
            resolution: 129,
            size: 320.0,
            base_height: 10.0,
            base_octaves: 4,
            base_frequency: 2.5,
            mountain_height: 20.0,
            mountain_octaves: 3,
            mask_octaves: 2,
            mask_threshold: 0.2,
            mask_softness: 0.3,
            warp_strength: 0.5,
            falloff: 0.0,
            sea_level: 0.0,
            range_amount: 1.0,
            range_center: -0.28,
            range_width: 0.16,
            range_height: 95.0,
            plain_keep: 0.07,
            plain_start: 0.08,
            lake_center: [0.1, 0.3],
            lake_radius: 0.11,
            lake_depth: 14.0,
            water_level: -2.0,
        }
    }
}

/// Sampled height graph: pure function of `(x, z)`.
#[derive(Debug, Clone)]
pub struct TerrainGraph {
    params: TerrainParams,
    base: Fbm,
    mountain: Fbm,
    mask: Fbm,
    range_noise: Fbm,
    warp: DomainWarp,
}

impl TerrainGraph {
    /// Build from parameters.
    pub fn new(params: TerrainParams) -> Self {
        let seed = params.seed;
        Self {
            base: Fbm::new(seed, params.base_octaves, 2.0, 0.5),
            mountain: Fbm::new(seed ^ 0x51ED, params.mountain_octaves, 2.1, 0.5),
            mask: Fbm::new(seed ^ 0xBEEF, params.mask_octaves, 2.0, 0.5),
            range_noise: Fbm::new(seed ^ 0xAA19, 4, 2.0, 0.5),
            warp: DomainWarp::new(seed ^ 0x5EED, params.warp_strength),
            params,
        }
    }

    /// Parameters.
    pub fn params(&self) -> &TerrainParams {
        &self.params
    }

    /// Height at world `(x, z)` in units.
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let p = &self.params;
        // Normalized coordinates for the noise fields.
        let nx = x / p.size * p.base_frequency;
        let nz = z / p.size * p.base_frequency;
        let (wx, wz) = if p.warp_strength > 0.0 {
            self.warp.warp(nx, nz)
        } else {
            (nx, nz)
        };
        let base = self.base.sample(wx, wz) * p.base_height;
        let mask_raw = self.mask.sample(nx * 0.5 + 11.3, nz * 0.5 - 7.1);
        let mask = smoothstep(
            p.mask_threshold,
            p.mask_threshold + p.mask_softness,
            mask_raw,
        );
        let mountain = self.mountain.ridged(wx, wz) * p.mountain_height * mask;
        let mut h = base + mountain;
        if p.range_amount > 0.0 {
            // Cordillera: band around the range axis, crests stretched
            // along X (anisotropic sampling) for elongated chains.
            let axis_z = p.range_center * p.size;
            let wobble = self.mask.sample(nx * 0.3 + 3.1, nz * 0.3 - 1.7) * p.size * 0.06;
            let d = ((z - axis_z - wobble).abs()) / p.size;
            let band = smoothstep(p.range_width, p.range_width * 0.35, d);
            let crest = self.range_noise.ridged(wx * 0.7 + 13.7, wz * 2.3 - 5.2);
            h += band * p.range_amount * (crest * p.range_height + base * 0.25);
        }
        if p.plain_keep < 1.0 {
            // Southern plain: damp relief toward a gentle base.
            let m = smoothstep(p.plain_start, p.plain_start + 0.12, z / p.size);
            let gentle = self.base.sample(nx * 0.4, nz * 0.4) * p.base_height * 0.15;
            h = h * (1.0 - m * (1.0 - p.plain_keep)) + gentle * m * (1.0 - p.plain_keep);
        }
        if p.lake_radius > 0.0 {
            // Crater dip for the lake.
            let dx = x / p.size - p.lake_center[0];
            let dz = z / p.size - p.lake_center[1];
            let d = (dx * dx + dz * dz).sqrt();
            h -= p.lake_depth * smoothstep(p.lake_radius, p.lake_radius * 0.35, d);
        }
        if p.falloff > 0.0 {
            // Radial island falloff: 1 at center, 0 past the rim.
            let r = (x * x + z * z).sqrt() / (p.size * 0.5);
            let fall = smoothstep(1.0, 1.0 - p.falloff.clamp(1e-3, 1.0), r);
            h = h * fall + (p.sea_level - p.base_height * 0.6) * (1.0 - fall);
        }
        h
    }

    /// Bake the graph to a [`Heightfield`].
    pub fn bake(&self) -> Heightfield {
        Heightfield::bake(self)
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Baked height grid: `resolution²` heights, row-major (`z` rows).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heightfield {
    /// Vertices per side.
    pub resolution: u32,
    /// World XZ extent.
    pub size: f32,
    /// Heights in units, `heights[z * resolution + x]`.
    pub heights: Vec<f32>,
}

impl Heightfield {
    /// Bake a graph (vertices span `-size/2..=size/2`).
    pub fn bake(graph: &TerrainGraph) -> Self {
        let p = graph.params();
        let res = p.resolution.max(2) as usize;
        let mut heights = Vec::with_capacity(res * res);
        for j in 0..res {
            let z = (j as f32 / (res - 1) as f32 - 0.5) * p.size;
            for i in 0..res {
                let x = (i as f32 / (res - 1) as f32 - 0.5) * p.size;
                heights.push(graph.sample(x, z));
            }
        }
        Self {
            resolution: res as u32,
            size: p.size,
            heights,
        }
    }

    /// Grid step in world units.
    pub fn step(&self) -> f32 {
        self.size / (self.resolution.max(2) - 1) as f32
    }

    /// Height at grid vertex `(i, j)` (clamped).
    pub fn at(&self, i: i32, j: i32) -> f32 {
        let res = self.resolution as i32;
        let i = i.clamp(0, res - 1) as usize;
        let j = j.clamp(0, res - 1) as usize;
        self.heights[j * res as usize + i]
    }

    /// Bilinear sample at world `(x, z)` (clamped to the rim).
    pub fn sample_world(&self, x: f32, z: f32) -> f32 {
        let res = self.resolution as usize;
        let half = self.size * 0.5;
        let fx = ((x + half) / self.size * (res - 1) as f32).clamp(0.0, (res - 1) as f32);
        let fz = ((z + half) / self.size * (res - 1) as f32).clamp(0.0, (res - 1) as f32);
        let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
        let (i1, j1) = ((i0 + 1).min(res - 1), (j0 + 1).min(res - 1));
        let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
        let a = self.heights[j0 * res + i0];
        let b = self.heights[j0 * res + i1];
        let c = self.heights[j1 * res + i0];
        let d = self.heights[j1 * res + i1];
        a * (1.0 - tx) * (1.0 - tz) + b * tx * (1.0 - tz) + c * (1.0 - tx) * tz + d * tx * tz
    }

    /// Min/max heights.
    pub fn range(&self) -> (f32, f32) {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for h in &self.heights {
            lo = lo.min(*h);
            hi = hi.max(*h);
        }
        (lo, hi)
    }

    /// Total sediment (for conservation tests).
    pub fn total(&self) -> f64 {
        self.heights.iter().map(|h| *h as f64).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_is_deterministic() {
        let g = TerrainGraph::new(TerrainParams {
            seed: 99,
            ..Default::default()
        });
        assert_eq!(g.sample(10.0, -3.0), g.sample(10.0, -3.0));
        let h = TerrainGraph::new(TerrainParams {
            seed: 100,
            ..Default::default()
        });
        assert!((g.sample(10.0, -3.0) - h.sample(10.0, -3.0)).abs() > 1e-4);
    }

    #[test]
    fn cordillera_plain_and_lake_compose() {
        let g = TerrainGraph::new(TerrainParams::cordillera(7));
        let size = 320.0;
        // Range band peaks high (sample several x across the axis).
        let mut peak = f32::MIN;
        for i in 0..32 {
            let x = -160.0 + i as f32 * 10.0;
            peak = peak.max(g.sample(x, -0.28 * size));
        }
        assert!(peak > 35.0, "no cordillera: {peak}");
        // Plain is flat: low variance across samples (west of the lake).
        let mut mean = 0.0f64;
        let mut m2 = 0.0f64;
        let mut n = 0u32;
        for j in 0..16 {
            for i in 0..16 {
                let h = g.sample(-100.0 + i as f32 * 4.0, 0.2 * size + j as f32 * 4.0) as f64;
                n += 1;
                let delta = h - mean;
                mean += delta / n as f64;
                m2 += delta * (h - mean);
            }
        }
        let std = (m2 / n as f64).sqrt();
        assert!(std < 3.0, "plain too rough: {std}");
        // Lake bottom sits below water level.
        let lake = g.sample(0.1 * size, 0.3 * size);
        assert!(lake < -2.0, "lake not dug: {lake}");
        // Deterministic.
        let h = TerrainGraph::new(TerrainParams::cordillera(7));
        assert_eq!(g.sample(3.0, -50.0), h.sample(3.0, -50.0));
    }

    #[test]
    fn island_falloff_drowns_the_rim() {
        let g = TerrainGraph::new(TerrainParams::default());
        let center = g.sample(0.0, 0.0);
        let rim = g.sample(127.0, 127.0);
        assert!(center > rim, "{center} vs {rim}");
        assert!(rim < 0.0, "rim below sea: {rim}");
    }

    #[test]
    fn bake_matches_sample_and_interpolates() {
        let g = TerrainGraph::new(TerrainParams {
            resolution: 9,
            size: 8.0,
            ..Default::default()
        });
        let h = g.bake();
        assert_eq!(h.heights.len(), 81);
        // Vertices match direct sampling exactly.
        assert_eq!(h.at(0, 0), g.sample(-4.0, -4.0));
        assert_eq!(h.at(8, 8), g.sample(4.0, 4.0));
        // Bilinear midpoint of a quad corner average.
        let mid = h.sample_world(0.5, 0.5);
        let corners = (h.at(4, 4) + h.at(5, 4) + h.at(4, 5) + h.at(5, 5)) / 4.0;
        assert!((mid - corners).abs() < 1e-4, "{mid} vs {corners}");
        let (lo, hi) = h.range();
        assert!(lo < hi && lo.is_finite() && hi.is_finite());
    }
}
