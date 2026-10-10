//! Pure-math 3D terrain synthesis: noise, erosion, meshing, LOD, biomes.
//!
//! No engine dependencies (only `serde` for documents): sampling, erosion
//! and meshing are deterministic `f32` math, so the same seed always builds
//! the same world on every platform. Bridges to [`Scene3D`](https://github.com/dadtnttn/velvet-engine)
//! heightfields live in `velvet-scene3d`, rendering in `velvet-render`.

pub mod biome;
pub mod erosion;
pub mod graph;
pub mod mesh;
pub mod noise;
pub mod preview;

pub use biome::{classify, paint_biomes, Biome, BiomeWeights, BIOME_COUNT, MOISTURE_SEED_XOR};
pub use erosion::{
    cavity_map, erode_hydraulic, erode_hydraulic_with_flow, erode_thermal, fill_depressions,
    smooth, total_variation, ErosionParams,
};
pub use graph::{Heightfield, TerrainGraph, TerrainParams};
pub use mesh::{build_lod_mesh, build_mesh, TerrainMesh};
pub use noise::{DomainWarp, Fbm, Simplex2, SplitMix64};
pub use preview::preview_rgba;
