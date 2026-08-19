//! Official declarative application runtime for Velvet Script 3.
//!
//! A package exports `app_manifest`, `app_init`, `app_update`, and `app_action`.
//! Rust remains a generic capability host for windows, physical input, resources,
//! and a deterministic software interpreter for 2D primitive commands.

mod app;
mod assets;
mod graphics;
mod input;
mod model;
mod platform;
mod scene;
mod script;

pub use app::{
    capture_package, capture_package_with_action, run_package, validate_package, AppOptions, Vs3App,
};
pub use assets::{AppLimits, AppManifest, AppSpec, FontSpec, ImageSpec};
pub use model::{FrameView, RuntimeEvent, VisualCommandView};
pub use script::ScriptRuntime;
