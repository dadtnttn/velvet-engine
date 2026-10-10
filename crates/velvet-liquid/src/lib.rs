//! Pure-math liquid simulation: shallow-water heightfields and SPH particles.
//!
//! Deterministic `f32`/`f64` math, no engine dependencies. Couples with
//! `velvet-terrain` heightfields (rain, puddles, bedrock) and exports water
//! surfaces through the terrain mesher.

pub mod shallow;
pub mod spectrum;
pub mod sph;

pub use shallow::{
    archimedes_force, calm_chop, contour_triangles, foam_mask, gerstner_displace, GerstnerWave,
    ShallowParams, ShallowWater,
};
pub use spectrum::{SpectralWave, WaveSpectrum, GRAVITY};
pub use sph::{SphParams, SphSim};
