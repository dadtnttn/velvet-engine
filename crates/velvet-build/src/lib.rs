//! # velvet-build
//!
//! Asset packing (with exclude/include globs), localization extract/validate
//! (JSON + simple PO/properties), and desktop export manifests including
//! multi-platform dry-run.

#![deny(missing_docs)]

mod android;
mod export;
mod localization;
mod pack;
mod steam;
mod web;

pub mod prelude;

pub use android::{
    detect_android_sdk, export_android, try_cross_compile_linux, AndroidExportError,
    AndroidExportOptions, AndroidExportReport, CrossCompileReport,
};
pub use export::{
    export_desktop, export_multi_platform_dry_run, list_zip_entries, write_directory_zip,
    ExportError, ExportManifest, ExportOptions, ExportPlatform, ExportReport, MultiPlatformExport,
};
pub use localization::{
    extract_from_source, load_catalog_auto, validate_catalog, LocalizationCatalog,
    LocalizationEntry, LocalizationError,
};
pub use pack::{
    copy_dir, copy_dir_with, ensure_dir, pack_directory, pack_directory_with, path_matches,
    AssetPack, PackError, PackFile, PackOptions,
};
pub use steam::{write_steam_appid_file, SteamConfig, SteamHook, SteamStatus};
pub use web::{export_web_player, run_web_player_node, WebExportError, WebExportReport};
