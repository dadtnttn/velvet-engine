//! Official declarative application runtime for Velvet Script 3.
//!
//! A package exports `app_manifest`, `app_init`, `app_update`, and `app_action`.
//! Rust remains a generic capability host for windows, physical input, resources,
//! and a deterministic software interpreter for 2D primitive commands.

mod app;
mod assets;
mod audio;
mod graphics;
mod input;
mod locale;
mod model;
mod platform;
mod scene;
mod script;
mod storage;

pub use app::{
    capture_package, capture_package_with_action, profile_package, run_package, validate_package,
    AppOptions, SaveLocation, Vs3App,
};
pub use assets::{AppLimits, AppManifest, AppSpec, FontSpec, ImageSpec};
pub use model::{FrameView, RuntimeEvent, VisualCommandView};
pub use script::ScriptRuntime;
pub use storage::{SaveStore, MAX_SAVE_BYTES, MAX_SLOTS};
pub use velvet_script_vs3::ProfileEntry;
