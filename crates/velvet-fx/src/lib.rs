//! # velvet-fx
//!
//! **BitFX Forge-like procedural pixel VFX generator** built fully on Velvet Engine.
//!
//! Loopable pixel-art effects (fire, smoke, magic, lightning, portals, etc.)
//! generated procedurally — no manual frame-by-frame drawing. Each effect is a
//! [`FxPreset`] (JSON-serializable) that drives a deterministic particle sim
//! and renders to pixel-perfect frames + spritesheets.
//!
//! ## Quick start
//! ```ignore
//! use velvet_fx::{FxPreset, FxSim, PaletteId};
//! let preset = FxPreset::random(0xCAFE);
//! let sim = FxSim::new(preset);
//! let frame = sim.render_frame(0.0); // RgbaImage
//! ```

#![deny(missing_docs)]

pub mod emitter;
pub mod export;
pub mod palette;
pub mod particle;
pub mod path;
pub mod preset;
pub mod randomize;
pub mod sim;

pub use emitter::{EmitterConfig, EmitterShape};
pub use export::{export_json_atlas, export_png, export_spritesheet, AtlasMeta, ExportError};
pub use palette::{palette_colors, Palette, PaletteId, PALETTES};
pub use particle::FxParticle;
pub use path::{PathConfig, PathKind, PathSample};
pub use preset::{Dimensions, FxMeta, FxPreset, LoopMode, ParticleConfig};
pub use randomize::{mutate_preset, random_preset};
pub use sim::{FxSim, SimFrame};
