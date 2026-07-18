//! Configuration validation helpers.

use crate::config::{EngineConfig, LogConfig, WindowConfig};
use crate::error::{CoreError, Result};
use crate::RunMode;

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Informational.
    Info,
    /// Suspicious but runnable.
    Warning,
    /// Must fix before run in production.
    Error,
}

/// One validation finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// Severity.
    pub severity: Severity,
    /// Dot-path field name (e.g. `window.width`).
    pub field: String,
    /// Human message.
    pub message: String,
}

impl ValidationIssue {
    /// Create issue.
    pub fn new(severity: Severity, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity,
            field: field.into(),
            message: message.into(),
        }
    }

    /// Error shorthand.
    pub fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, field, message)
    }

    /// Warning shorthand.
    pub fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, field, message)
    }

    /// Info shorthand.
    pub fn info(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(Severity::Info, field, message)
    }
}

/// Result of validating a config object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    /// Findings.
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    /// Empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push issue.
    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }

    /// Whether any errors.
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Whether any warnings.
    pub fn has_warnings(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Warning)
    }

    /// Error count.
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    /// Convert to `Result` — `Ok` if no errors (warnings allowed).
    pub fn into_result(self) -> Result<Self> {
        if self.has_errors() {
            let msg = self
                .issues
                .iter()
                .filter(|i| i.severity == Severity::Error)
                .map(|i| format!("{}: {}", i.field, i.message))
                .collect::<Vec<_>>()
                .join("; ");
            Err(CoreError::Config(msg))
        } else {
            Ok(self)
        }
    }

    /// Merge another report.
    pub fn extend(&mut self, other: ValidationReport) {
        self.issues.extend(other.issues);
    }
}

/// Validate window configuration.
pub fn validate_window(cfg: &WindowConfig) -> ValidationReport {
    let mut r = ValidationReport::new();
    if cfg.width == 0 {
        r.push(ValidationIssue::error("window.width", "width must be > 0"));
    }
    if cfg.height == 0 {
        r.push(ValidationIssue::error(
            "window.height",
            "height must be > 0",
        ));
    }
    if cfg.virtual_width == 0 {
        r.push(ValidationIssue::error(
            "window.virtual_width",
            "virtual_width must be > 0",
        ));
    }
    if cfg.virtual_height == 0 {
        r.push(ValidationIssue::error(
            "window.virtual_height",
            "virtual_height must be > 0",
        ));
    }
    if cfg.width > 16_384 || cfg.height > 16_384 {
        r.push(ValidationIssue::warning(
            "window.size",
            "window dimensions exceed 16k; may fail on some GPUs",
        ));
    }
    if cfg.title.trim().is_empty() {
        r.push(ValidationIssue::warning(
            "window.title",
            "empty window title",
        ));
    }
    let aspect = cfg.virtual_width as f32 / cfg.virtual_height.max(1) as f32;
    if !(0.25..=4.0).contains(&aspect) {
        r.push(ValidationIssue::warning(
            "window.virtual_aspect",
            format!("unusual virtual aspect ratio {aspect:.2}"),
        ));
    }
    r
}

/// Validate log config.
pub fn validate_log(cfg: &LogConfig) -> ValidationReport {
    let mut r = ValidationReport::new();
    if cfg.filter.trim().is_empty() {
        r.push(ValidationIssue::warning(
            "log.filter",
            "empty filter; tracing may be silent",
        ));
    }
    if cfg.filter.len() > 2048 {
        r.push(ValidationIssue::error(
            "log.filter",
            "filter string too long",
        ));
    }
    r
}

/// Validate full engine config.
pub fn validate_engine_config(cfg: &EngineConfig) -> ValidationReport {
    let mut r = ValidationReport::new();
    if cfg.name.trim().is_empty() {
        r.push(ValidationIssue::error("name", "application name is empty"));
    }
    if cfg.name.len() > 256 {
        r.push(ValidationIssue::error(
            "name",
            "application name exceeds 256 characters",
        ));
    }
    if cfg.fixed_hz <= 0.0 || !cfg.fixed_hz.is_finite() {
        r.push(ValidationIssue::error(
            "fixed_hz",
            "fixed_hz must be a positive finite number",
        ));
    } else if cfg.fixed_hz < 1.0 {
        r.push(ValidationIssue::warning(
            "fixed_hz",
            "fixed_hz < 1 is unusually low",
        ));
    } else if cfg.fixed_hz > 1000.0 {
        r.push(ValidationIssue::warning(
            "fixed_hz",
            "fixed_hz > 1000 is unusually high",
        ));
    }
    if let Some(limit) = cfg.frame_limit {
        if limit == 0 {
            r.push(ValidationIssue::error(
                "frame_limit",
                "frame_limit of 0 is invalid; use None for uncapped",
            ));
        } else if limit > 1000 {
            r.push(ValidationIssue::warning(
                "frame_limit",
                "frame_limit > 1000 is unusually high",
            ));
        }
    }
    for (i, p) in cfg.plugins.iter().enumerate() {
        if p.trim().is_empty() {
            r.push(ValidationIssue::error(
                format!("plugins[{i}]"),
                "empty plugin id",
            ));
        }
    }
    // Duplicate plugin ids
    let mut seen = std::collections::HashSet::new();
    for p in &cfg.plugins {
        if !seen.insert(p.as_str()) {
            r.push(ValidationIssue::warning(
                "plugins",
                format!("duplicate plugin id '{p}'"),
            ));
        }
    }
    if cfg.mode == RunMode::Production && cfg.log.filter.contains("trace") {
        r.push(ValidationIssue::info(
            "log.filter",
            "production mode with trace logging may hurt performance",
        ));
    }
    r.extend(validate_window(&cfg.window));
    r.extend(validate_log(&cfg.log));
    r
}

/// Validate and return the config or a combined error.
pub fn ensure_valid(cfg: &EngineConfig) -> Result<()> {
    validate_engine_config(cfg).into_result().map(|_| ())
}

/// Clamp / sanitize a config into a runnable form (does not error).
pub fn sanitize_engine_config(mut cfg: EngineConfig) -> EngineConfig {
    if cfg.name.trim().is_empty() {
        cfg.name = "Velvet Application".into();
    }
    if cfg.fixed_hz <= 0.0 || !cfg.fixed_hz.is_finite() {
        cfg.fixed_hz = 60.0;
    }
    cfg.fixed_hz = cfg.fixed_hz.clamp(1.0, 1000.0);
    if let Some(0) = cfg.frame_limit {
        cfg.frame_limit = None;
    }
    if cfg.window.width == 0 {
        cfg.window.width = 1280;
    }
    if cfg.window.height == 0 {
        cfg.window.height = 720;
    }
    if cfg.window.virtual_width == 0 {
        cfg.window.virtual_width = 1920;
    }
    if cfg.window.virtual_height == 0 {
        cfg.window.virtual_height = 1080;
    }
    if cfg.window.title.trim().is_empty() {
        cfg.window.title = cfg.name.clone();
    }
    if cfg.log.filter.trim().is_empty() {
        cfg.log.filter = "info".into();
    }
    cfg.plugins.retain(|p| !p.trim().is_empty());
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_valid() {
        let r = validate_engine_config(&EngineConfig::default());
        assert!(!r.has_errors(), "{:?}", r.issues);
    }

    #[test]
    fn zero_size_errors() {
        let mut cfg = EngineConfig::default();
        cfg.window.width = 0;
        let r = validate_engine_config(&cfg);
        assert!(r.has_errors());
        assert!(ensure_valid(&cfg).is_err());
    }

    #[test]
    fn sanitize_fixes() {
        let mut cfg = EngineConfig::default();
        cfg.window.width = 0;
        cfg.fixed_hz = -1.0;
        cfg.frame_limit = Some(0);
        cfg.name = "  ".into();
        let s = sanitize_engine_config(cfg);
        assert!(s.window.width > 0);
        assert!(s.fixed_hz > 0.0);
        assert!(s.frame_limit.is_none());
        assert!(!s.name.trim().is_empty());
        assert!(!validate_engine_config(&s).has_errors());
    }

    #[test]
    fn duplicate_plugins_warning() {
        let cfg = EngineConfig {
            plugins: vec!["a".into(), "a".into()],
            ..Default::default()
        };
        let r = validate_engine_config(&cfg);
        assert!(r.has_warnings());
        assert!(!r.has_errors());
    }
}
