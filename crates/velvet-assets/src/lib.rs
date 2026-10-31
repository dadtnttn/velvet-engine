//! # velvet-assets
//!
//! Handle-based asset storage with load states, virtual paths, and hot-reload hooks.

#![deny(missing_docs)]

#[cfg(target_os = "android")]
mod apk_source;
mod asset_index;
mod async_queue;
mod bundle;
mod handle;
mod hot_reload;
mod image_asset;
mod loader;
mod missing;
mod path;
mod platform;
mod plugin;
mod reader;
mod refcount;
mod registry;
mod source;
mod streaming;
#[cfg(feature = "vlz")]
mod vlz_source;

pub mod prelude;

/// Packaged assets of the running Android app (the APK and its install-time asset packs).
#[cfg(target_os = "android")]
pub use apk_source::ApkSource;
pub use asset_index::{AssetIndex, IndexEntry};
pub use async_queue::{AsyncLoadQueue, LoadPriority, QueueItem, QueuePhase};
pub use bundle::{AssetBundle, BundleEntry, BundleError};
pub use handle::{AssetId, AssetState, Handle, StrongHandle};
pub use hot_reload::HotReloader;
pub use image_asset::{
    decode_image_bytes, register_image_loaders, role_from_path, AtlasFrame, AtlasFrameJson,
    AtlasJson, AtlasLoader, ImageAsset, ImageLoader, ImageRole, PresentAssetCatalog,
    SpriteAtlasAsset,
};
pub use loader::{AssetLoader, DependencyGraph, LoadError, LoadRequest, LoaderRegistry};
pub use missing::{
    pink_checker_rgba8, resolve_missing, MissingAssetConfig, MissingAssetPolicy, MissingResolution,
    PINK_CHECKER_RGBA,
};
pub use path::{AssetPath, VirtualPath};
pub use platform::{
    parse_path_tags, AssetVariant, PathTags, PlatformClass, PlatformProfile, QualityTier,
    VariantCatalog,
};
pub use plugin::AssetsPlugin;
pub use reader::AssetReader;
pub use refcount::{LiveSet, RcHandle, RefCountTable, TrackedStrongHandle};
pub use registry::{AssetEvent, Assets};
pub use source::{FileSource, LayeredSource, MemorySource, Source, SourceError};
pub use streaming::{
    AssetStreamer, StreamCompletion, StreamConfig, StreamPayload, StreamStats, StreamTicket,
};
/// The VLZ container crate (packing, archive inspection, codecs).
#[cfg(feature = "vlz")]
pub use velvet_vlz as vlz;
#[cfg(feature = "vlz")]
pub use vlz_source::VlzSource;

/// Bytes asset (generic raw payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytesAsset {
    /// Raw bytes.
    pub data: Vec<u8>,
}

/// Text asset (UTF-8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAsset {
    /// Text content.
    pub text: String,
}
