//! Versioned VS3 package manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use semver::{Version, VersionReq};
use serde::Deserialize;
use thiserror::Error;

/// Canonical VS3 package manifest file name.
pub const VS3_PACKAGE_MANIFEST: &str = "velvet.package.toml";

/// Error produced while parsing or validating a package manifest or lockfile.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Vs3PackageFormatError {
    /// The TOML document is not syntactically valid or does not match the schema.
    #[error("invalid package TOML: {0}")]
    Toml(String),
    /// The document is syntactically valid but violates a VS3 package invariant.
    #[error("invalid package: {0}")]
    Invalid(String),
}

/// One explicitly declared local package dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs3Dependency {
    /// Required dependency version range.
    pub requirement: VersionReq,
    /// Manifest-relative path to the dependency package directory.
    pub path: String,
}

/// Stable identity and source mapping for one VS3 package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vs3PackageManifest {
    /// Stable package name used as the first module-name segment.
    pub name: String,
    /// Semantic package version.
    pub version: Version,
    /// Language edition. Package manifests currently require edition 3.
    pub edition: u32,
    /// Fully qualified module identity used as the package entry module.
    pub root: String,
    /// Fully qualified module identities mapped to package-relative `.vel` files.
    pub modules: BTreeMap<String, String>,
    /// Local path dependencies keyed by their expected package names.
    pub dependencies: BTreeMap<String, Vs3Dependency>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    package: RawPackage,
    modules: BTreeMap<String, String>,
    #[serde(default)]
    dependencies: BTreeMap<String, RawDependency>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPackage {
    name: String,
    version: String,
    edition: u32,
    root: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDependency {
    version: String,
    path: String,
}

impl Vs3PackageManifest {
    /// Parse and validate a `velvet.package.toml` document.
    pub fn parse(source: &str) -> Result<Self, Vs3PackageFormatError> {
        let raw: RawManifest = toml::from_str(source)
            .map_err(|error| Vs3PackageFormatError::Toml(error.to_string()))?;
        let version = Version::parse(&raw.package.version).map_err(|error| {
            Vs3PackageFormatError::Invalid(format!(
                "package version `{}` is not semantic versioning: {error}",
                raw.package.version
            ))
        })?;
        let mut dependencies = BTreeMap::new();
        for (name, dependency) in raw.dependencies {
            let requirement = VersionReq::parse(&dependency.version).map_err(|error| {
                Vs3PackageFormatError::Invalid(format!(
                    "dependency `{name}` has invalid version requirement `{}`: {error}",
                    dependency.version
                ))
            })?;
            dependencies.insert(
                name,
                Vs3Dependency {
                    requirement,
                    path: dependency.path,
                },
            );
        }
        let manifest = Self {
            name: raw.package.name,
            version,
            edition: raw.package.edition,
            root: raw.package.root,
            modules: raw.modules,
            dependencies,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    /// Read and validate a package manifest from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Vs3PackageFormatError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|error| {
            Vs3PackageFormatError::Invalid(format!("cannot read {}: {error}", path.display()))
        })?;
        Self::parse(&source)
    }

    /// Return the package-relative source path for a fully qualified module identity.
    pub fn module_path(&self, module: &str) -> Option<&str> {
        self.modules.get(module).map(String::as_str)
    }

    /// Validate all cross-field package invariants.
    pub fn validate(&self) -> Result<(), Vs3PackageFormatError> {
        if !valid_identifier(&self.name) {
            return invalid(format!(
                "package name `{}` must be one identifier",
                self.name
            ));
        }
        if self.edition != 3 {
            return invalid(format!(
                "package `{}` uses unsupported edition {}; expected 3",
                self.name, self.edition
            ));
        }
        if self.modules.is_empty() {
            return invalid(format!(
                "package `{}` must declare at least one module",
                self.name
            ));
        }
        if !self.modules.contains_key(&self.root) {
            return invalid(format!(
                "root module `{}` is not declared in `[modules]`",
                self.root
            ));
        }
        let prefix = format!("{}.", self.name);
        let mut paths = BTreeSet::new();
        for (module, path) in &self.modules {
            if !valid_dotted_identifier(module)
                || !(module == &self.name || module.starts_with(&prefix))
            {
                return invalid(format!(
                    "module `{module}` must be `{}` or start with `{prefix}`",
                    self.name
                ));
            }
            validate_module_path(path)?;
            if !paths.insert(path) {
                return invalid(format!(
                    "module source `{path}` is assigned to more than one identity"
                ));
            }
        }
        for (name, dependency) in &self.dependencies {
            if !valid_identifier(name) {
                return invalid(format!("dependency name `{name}` must be one identifier"));
            }
            if name == &self.name {
                return invalid(format!("package `{}` cannot depend on itself", self.name));
            }
            validate_dependency_path(&dependency.path)?;
        }
        Ok(())
    }
}

pub(crate) fn valid_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub(crate) fn valid_dotted_identifier(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(valid_identifier)
}

fn validate_module_path(value: &str) -> Result<(), Vs3PackageFormatError> {
    let path = Path::new(value);
    let safe = !value.is_empty()
        && !value.contains('\\')
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && path.extension().and_then(|extension| extension.to_str()) == Some("vel");
    if safe {
        Ok(())
    } else {
        invalid(format!(
            "module path `{value}` must be a forward-slash package-relative `.vel` path without `.` or `..`"
        ))
    }
}

fn validate_dependency_path(value: &str) -> Result<(), Vs3PackageFormatError> {
    let path = Path::new(value);
    let safe = !value.trim().is_empty()
        && !value.contains('\\')
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::ParentDir));
    if safe {
        Ok(())
    } else {
        invalid(format!(
            "dependency path `{value}` must be an explicit relative directory path"
        ))
    }
}

fn invalid<T>(message: String) -> Result<T, Vs3PackageFormatError> {
    Err(Vs3PackageFormatError::Invalid(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
[package]
name = "game"
version = "1.2.3"
edition = 3
root = "game.entry"

[modules]
"game.entry" = "entry.vel"
"game.rules" = "logic/rules.vel"

[dependencies]
math = { version = "^2.1", path = "../math" }
"#;

    #[test]
    fn parses_manifest_and_semver_requirements() {
        let manifest = Vs3PackageManifest::parse(VALID).unwrap();
        assert_eq!(manifest.name, "game");
        assert_eq!(manifest.version, Version::new(1, 2, 3));
        assert!(manifest.dependencies["math"]
            .requirement
            .matches(&Version::new(2, 4, 0)));
        assert_eq!(manifest.module_path("game.rules"), Some("logic/rules.vel"));
    }

    #[test]
    fn rejects_malformed_module_identities() {
        for invalid_module in [".x", "x.", "x..y", "-x", "game.-rules", "other.rules"] {
            let source = VALID.replace("game.rules", invalid_module);
            assert!(
                Vs3PackageManifest::parse(&source).is_err(),
                "accepted `{invalid_module}`"
            );
        }
    }

    #[test]
    fn rejects_invalid_root_paths_and_versions() {
        assert!(Vs3PackageManifest::parse(
            &VALID.replace("root = \"game.entry\"", "root = \"game.missing\"")
        )
        .is_err());
        assert!(
            Vs3PackageManifest::parse(&VALID.replace("logic/rules.vel", "../rules.vel")).is_err()
        );
        assert!(Vs3PackageManifest::parse(&VALID.replace("1.2.3", "latest")).is_err());
        assert!(Vs3PackageManifest::parse(&VALID.replace("edition = 3", "edition = 2")).is_err());
    }

    #[test]
    fn rejects_duplicate_module_sources_and_self_dependency() {
        let duplicate = VALID.replace("logic/rules.vel", "entry.vel");
        assert!(Vs3PackageManifest::parse(&duplicate).is_err());
        let self_dependency = VALID.replace("math =", "game =");
        assert!(Vs3PackageManifest::parse(&self_dependency).is_err());
    }
}
