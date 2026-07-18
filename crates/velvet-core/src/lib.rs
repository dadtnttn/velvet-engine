//! # velvet-core
//!
//! Shared foundation types for Velvet Engine: errors, configuration, diagnostics,
//! plugin identifiers, and runtime mode. This crate does **not** implement the
//! application loop (see `velvet-app`).

#![deny(missing_docs)]

pub mod config;
pub mod diagnostics;
pub mod error;
pub mod hash;
pub mod netcode;
pub mod plugin;
pub mod plugin_registry;
pub mod prelude;
pub mod profiling;
pub mod services;
pub mod validate;
pub mod version;

pub use netcode::{loopback_roundtrip, NetError, NetMessage, NetPeer};

pub use config::{EngineConfig, LogConfig, WindowConfig};
pub use diagnostics::{Diagnostics, FrameStats};
pub use error::{CoreError, Result};
pub use hash::{
    fnv1a64, fnv1a64_str, hex_decode, hex_encode, mix_u64, sha256_hex, sha256_str, RollingSha256,
};
pub use plugin::{PluginError, PluginId, PluginInfo, VersionReq};
pub use plugin_registry::{
    check_version_req, parse_simple_req, PluginDependency, PluginEntry, PluginRegistry,
};
pub use profiling::{DiagnosticSpan, Profiler, SpanGuard, SpanLog, SpanName, SpanStats};
pub use services::{
    BuildProfileService, FeatureFlags, IdGenerator, LocaleService, PathService, PlatformCaps,
    ServiceError, ServiceHub, ServiceMeta, ServiceRegistry,
};
pub use validate::{
    ensure_valid, sanitize_engine_config, validate_engine_config, validate_log, validate_window,
    Severity, ValidationIssue, ValidationReport,
};
pub use version::{engine_version, Version};

/// Runtime execution mode.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum RunMode {
    /// Development: hot-reload, extra validation, verbose diagnostics.
    #[default]
    Development,
    /// Production: optimizations, fewer checks, user-facing paths.
    Production,
}

impl RunMode {
    /// Whether development amenities are enabled.
    pub fn is_development(self) -> bool {
        matches!(self, Self::Development)
    }
}

/// Initialize a basic `tracing` subscriber if none is set.
///
/// Safe to call multiple times; subsequent calls are no-ops if a global
/// subscriber already exists. Prefer configuring subscribers in binaries.
pub fn init_tracing_default(filter: &str) {
    use tracing_subscriber::prelude::*;
    let env = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter));
    let _ = tracing_subscriber::registry()
        .with(env)
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}
