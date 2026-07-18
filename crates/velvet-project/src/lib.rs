//! Project model and `velvet.project` format.
//!
//! Provides parsing, validation, module enable flags, and dependency resolution
//! between Velvet engine modules.

#![deny(missing_docs)]

mod modules;
mod validate;

pub use modules::{
    can_enable, enable_with_deps, ModuleInfo, ModuleRegistry, ModuleResolveError, KNOWN_MODULES,
};
pub use validate::{
    validate_project, validate_root, ValidateOptions, ValidationIssue, ValidationReport,
    ValidationSeverity,
};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use velvet_core::config::WindowConfig;

/// Errors loading projects.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// I/O.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// Parse.
    #[error("parse: {0}")]
    Parse(String),
    /// Validation failed with hard errors.
    #[error("validation failed with {0} error(s)")]
    Validation(usize),
}

/// Root project document.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VelvetProject {
    /// Display name.
    pub name: String,
    /// Reverse-DNS identifier.
    pub identifier: String,
    /// Semver string.
    pub version: String,
    /// Enabled modules: story, play, rpg, action, ...
    pub modules: Vec<String>,
    /// Entry scene path (virtual or relative).
    pub entry_scene: String,
    /// Window configuration.
    #[serde(default)]
    pub window: WindowConfig,
    /// Optional asset root.
    #[serde(default = "default_assets")]
    pub assets_dir: String,
}

fn default_assets() -> String {
    "assets".into()
}

impl Default for VelvetProject {
    fn default() -> Self {
        Self {
            name: "Velvet Game".into(),
            identifier: "com.example.velvet_game".into(),
            version: "0.1.0".into(),
            modules: vec!["story".into()],
            entry_scene: "scenes/main.vel".into(),
            window: WindowConfig::default(),
            assets_dir: default_assets(),
        }
    }
}

impl VelvetProject {
    /// Parse from RON text (supports both bare struct and `Project(...)` wrappers loosely).
    pub fn from_ron(text: &str) -> Result<Self, ProjectError> {
        ron::from_str(text).map_err(|e| ProjectError::Parse(e.to_string()))
    }

    /// Serialize pretty RON.
    pub fn to_ron_pretty(&self) -> Result<String, ProjectError> {
        let pretty = ron::ser::PrettyConfig::new().enumerate_arrays(true);
        ron::ser::to_string_pretty(self, pretty).map_err(|e| ProjectError::Parse(e.to_string()))
    }

    /// Load from filesystem path.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, ProjectError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_ron(&text)
    }

    /// Whether a module is enabled (exact name match).
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.iter().any(|m| m == name)
    }

    /// Enable a module if not already present. Returns true if inserted.
    pub fn enable_module(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.has_module(&name) {
            return false;
        }
        self.modules.push(name);
        true
    }

    /// Disable a module. Returns true if removed.
    pub fn disable_module(&mut self, name: &str) -> bool {
        let before = self.modules.len();
        self.modules.retain(|m| m != name);
        self.modules.len() != before
    }

    /// Enable a module and all hard dependencies (using the built-in registry).
    pub fn enable_module_with_deps(&mut self, name: &str) -> Result<(), ModuleResolveError> {
        let mut enabled = self.modules.clone();
        if !enabled.iter().any(|m| m == name) {
            enabled.push(name.to_string());
        }
        self.modules = enable_with_deps(&enabled)?;
        Ok(())
    }

    /// Assets directory as UTF-8 path.
    pub fn assets_path(&self) -> Utf8PathBuf {
        Utf8PathBuf::from(self.assets_dir.as_str())
    }

    /// Validate with default options (no filesystem root).
    pub fn validate(&self) -> ValidationReport {
        validate_project(self, &ValidateOptions::default())
    }

    /// Validate against a project root directory.
    pub fn validate_at(&self, root: impl AsRef<std::path::Path>) -> ValidationReport {
        validate_project(
            self,
            &ValidateOptions {
                root: Some(root.as_ref().to_path_buf()),
                ..ValidateOptions::default()
            },
        )
    }

    /// Resolved dependency-ordered modules (errors if graph invalid).
    pub fn resolved_modules(&self) -> Result<Vec<String>, ModuleResolveError> {
        enable_with_deps(&self.modules)
    }
}

/// Default RON template for `velvet init`.
pub fn default_project_ron(name: &str) -> String {
    default_project_ron_with_modules(name, &["story", "play"])
}

/// Default RON with explicit modules.
pub fn default_project_ron_with_modules(name: &str, modules: &[&str]) -> String {
    let id = format!(
        "com.velvet.{}",
        name.to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
    );
    let project = VelvetProject {
        name: name.into(),
        identifier: id,
        version: "0.1.0".into(),
        modules: modules.iter().map(|s| (*s).to_string()).collect(),
        entry_scene: "scripts/main.vel".into(),
        window: WindowConfig {
            title: name.into(),
            ..WindowConfig::default()
        },
        assets_dir: "assets".into(),
    };
    project
        .to_ron_pretty()
        .unwrap_or_else(|_| "// failed to serialize".into())
}

/// Template-oriented project RON.
pub fn project_ron_for_template(name: &str, template: &str) -> String {
    let modules: &[&str] = match template {
        "visual-novel" => &["story", "ui", "audio"],
        "narrative-adventure" => &["story", "play", "ui"],
        "top-down-rpg" => &["play", "rpg", "story"],
        "top-down-action" => &["play", "action"],
        _ => &["story", "play"],
    };
    default_project_ron_with_modules(name, modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let p = VelvetProject::default();
        let text = p.to_ron_pretty().unwrap();
        let back = VelvetProject::from_ron(&text).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn default_template_parses() {
        let text = default_project_ron("Demo");
        let p = VelvetProject::from_ron(&text).unwrap();
        assert_eq!(p.name, "Demo");
        assert!(p.has_module("story"));
    }

    #[test]
    fn enable_disable_modules() {
        let mut p = VelvetProject::default();
        assert!(p.enable_module("play"));
        assert!(!p.enable_module("play"));
        assert!(p.has_module("play"));
        assert!(p.disable_module("play"));
        assert!(!p.has_module("play"));
    }

    #[test]
    fn enable_with_deps_rpg() {
        let mut p = VelvetProject::default();
        p.modules.clear();
        p.enable_module_with_deps("rpg").unwrap();
        assert!(p.has_module("rpg"));
        assert!(p.has_module("play"));
        assert!(p.has_module("ecs"));
    }

    #[test]
    fn template_modules() {
        let text = project_ron_for_template("Act", "top-down-action");
        let p = VelvetProject::from_ron(&text).unwrap();
        assert!(p.has_module("action"));
        assert!(p.has_module("play"));
    }

    #[test]
    fn validate_ok() {
        let p = VelvetProject::default();
        assert!(p.validate().is_ok());
    }
}
