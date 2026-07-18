//! Engine and window configuration.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::RunMode;

/// Top-level engine configuration loaded from project or CLI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Application display name.
    pub name: String,
    /// Run mode.
    pub mode: RunMode,
    /// Window settings.
    pub window: WindowConfig,
    /// Logging.
    pub log: LogConfig,
    /// Fixed update rate in Hz.
    pub fixed_hz: f64,
    /// Target frame rate limit (None = uncapped).
    pub frame_limit: Option<u32>,
    /// Enabled plugin ids (empty = default set decided by host).
    pub plugins: Vec<String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            name: "Velvet Application".into(),
            mode: RunMode::Development,
            window: WindowConfig::default(),
            log: LogConfig::default(),
            fixed_hz: 60.0,
            frame_limit: Some(60),
            plugins: Vec::new(),
        }
    }
}

impl EngineConfig {
    /// Deserialize from RON text.
    pub fn from_ron(text: &str) -> Result<Self> {
        ron::from_str(text).map_err(CoreError::from)
    }

    /// Serialize to pretty RON.
    pub fn to_ron_pretty(&self) -> Result<String> {
        let pretty = ron::ser::PrettyConfig::new().enumerate_arrays(true);
        ron::ser::to_string_pretty(self, pretty).map_err(CoreError::from)
    }

    /// Deserialize from JSON.
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(CoreError::from)
    }

    /// Validate this config (see [`crate::validate::validate_engine_config`]).
    pub fn validate(&self) -> crate::validate::ValidationReport {
        crate::validate::validate_engine_config(self)
    }

    /// Validate and error if any errors are present.
    pub fn ensure_valid(&self) -> Result<()> {
        crate::validate::ensure_valid(self)
    }

    /// Return a sanitized copy safe to run.
    pub fn sanitized(&self) -> Self {
        crate::validate::sanitize_engine_config(self.clone())
    }
}

/// Window creation parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Logical width.
    pub width: u32,
    /// Logical height.
    pub height: u32,
    /// Virtual resolution width (render target).
    pub virtual_width: u32,
    /// Virtual resolution height.
    pub virtual_height: u32,
    /// Start fullscreen.
    pub fullscreen: bool,
    /// Resizable window.
    pub resizable: bool,
    /// VSync preference.
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Velvet Engine".into(),
            width: 1280,
            height: 720,
            virtual_width: 1920,
            virtual_height: 1080,
            fullscreen: false,
            resizable: true,
            vsync: true,
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogConfig {
    /// `tracing` filter directive, e.g. `velvet=debug,info`.
    pub filter: String,
    /// Log to stdout.
    pub stdout: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            filter: "velvet=info,info".into(),
            stdout: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_roundtrip() {
        let cfg = EngineConfig::default();
        let text = cfg.to_ron_pretty().unwrap();
        let back = EngineConfig::from_ron(&text).unwrap();
        assert_eq!(back.name, cfg.name);
        assert_eq!(back.window.width, 1280);
    }
}
