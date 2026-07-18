//! Commonly used core types.

pub use crate::config::{EngineConfig, LogConfig, WindowConfig};
pub use crate::diagnostics::{Diagnostics, FrameStats};
pub use crate::error::{CoreError, Result};
pub use crate::plugin::{PluginError, PluginId, PluginInfo, VersionReq};
pub use crate::plugin_registry::{PluginDependency, PluginEntry, PluginRegistry};
pub use crate::profiling::{Profiler, SpanGuard};
pub use crate::validate::{ensure_valid, validate_engine_config, ValidationReport};
pub use crate::version::{engine_version, Version};
pub use crate::RunMode;
