//! Velvet Studio library — shared by the `velvet-studio` binary and tests.
//!
//! Docking GUI model, canvas drag (wired to [`velvet_document`]), and project tools.

// Existing panel modules predate lib docs; GUI/drag APIs are documented.
#![allow(missing_docs)]

pub mod asset_panel;
pub mod commands;
pub mod console;
pub mod document_edit;
pub mod gui;
pub mod inspector;
pub mod layers;
pub mod project_browser;
pub mod script_panel;
pub mod studio;
pub mod studio_paint;
pub mod vscript;

pub use layers::{
    pct_to_px, px_to_pct, DesignSurface, LayerEdge, LayerEdgeKind, LayerStack, LayerTreeRow,
    ResPreset, ScreenLayer,
};
pub use vscript::{api_catalog, parse_script, validate as validate_vscript, Stmt};

pub use document_edit::{
    design_set_button, drag_region_on_disk, list_regions, require_file, set_visual_property,
};
pub use gui::{
    run_studio_gui, DockPanel, NodesTool, StudioEditorMode, StudioGuiConfig, StudioGuiSession,
    StudioGuiStatus,
};
pub use studio::StudioApp;
