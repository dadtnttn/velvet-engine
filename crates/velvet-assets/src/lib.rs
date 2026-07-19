//! # velvet-assets
//!
//! Handle-based asset storage with load states, virtual paths, and hot-reload hooks.

#![deny(missing_docs)]

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
mod refcount;
mod registry;
mod source;

pub mod prelude;

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
pub use refcount::{LiveSet, RcHandle, RefCountTable, TrackedStrongHandle};
pub use registry::{AssetEvent, Assets};
pub use source::{FileSource, MemorySource, Source};

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
