//! Assets prelude.

pub use crate::bundle::{AssetBundle, BundleEntry};
pub use crate::handle::{AssetId, AssetState, Handle, StrongHandle};
pub use crate::hot_reload::HotReloader;
pub use crate::loader::DependencyGraph;
pub use crate::missing::{MissingAssetConfig, MissingAssetPolicy};
pub use crate::path::{AssetPath, VirtualPath};
pub use crate::plugin::AssetsPlugin;
pub use crate::registry::{AssetEvent, Assets};
pub use crate::{BytesAsset, TextAsset};
