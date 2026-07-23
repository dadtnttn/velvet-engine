//! Deterministic VS3 package lockfile.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::manifest::{valid_identifier, Vs3PackageFormatError};

/// Canonical VS3 lockfile name.
pub const VS3_PACKAGE_LOCK: &str = "velvet.lock";

/// One exact package selected by dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vs3LockedPackage {
    /// Exact package name.
    pub name: String,
    /// Exact semantic version.
    pub version: Version,
    /// Deterministic source identifier such as `path+../math`.
    pub source: String,
    /// SHA-256 checksum of the manifest and all package `.vel` sources.
    pub checksum: String,
    /// Direct dependency package names in deterministic order.
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Reproducible package resolution recorded in `velvet.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vs3PackageLock {
    /// Lockfile schema version. The current value is `1`.
    #[serde(rename = "version")]
    pub lock_version: u32,
    /// Root and dependency packages sorted by package name.
    #[serde(rename = "package", default)]
    pub packages: Vec<Vs3LockedPackage>,
}

impl Vs3PackageLock {
    /// Construct and canonicalize a version-1 lockfile.
    pub fn new(packages: Vec<Vs3LockedPackage>) -> Result<Self, Vs3PackageFormatError> {
        let mut lock = Self {
            lock_version: 1,
            packages,
        };
        lock.canonicalize();
        lock.validate()?;
        Ok(lock)
    }

    /// Parse, canonicalize, and validate a lockfile document.
    pub fn parse(source: &str) -> Result<Self, Vs3PackageFormatError> {
        let mut lock: Self = toml::from_str(source)
            .map_err(|error| Vs3PackageFormatError::Toml(error.to_string()))?;
        lock.canonicalize();
        lock.validate()?;
        Ok(lock)
    }

    /// Read a lockfile from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Vs3PackageFormatError> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|error| {
            Vs3PackageFormatError::Invalid(format!("cannot read {}: {error}", path.display()))
        })?;
        Self::parse(&source)
    }

    /// Render a canonical TOML representation suitable for source control.
    pub fn render(&self) -> Result<String, Vs3PackageFormatError> {
        let mut canonical = self.clone();
        canonical.canonicalize();
        canonical.validate()?;
        let mut rendered = toml::to_string_pretty(&canonical)
            .map_err(|error| Vs3PackageFormatError::Toml(error.to_string()))?;
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        Ok(rendered)
    }

    /// Write the canonical lockfile to disk.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), Vs3PackageFormatError> {
        let path = path.as_ref();
        fs::write(path, self.render()?).map_err(|error| {
            Vs3PackageFormatError::Invalid(format!("cannot write {}: {error}", path.display()))
        })
    }

    /// Find a locked package by name.
    pub fn package(&self, name: &str) -> Option<&Vs3LockedPackage> {
        self.packages.iter().find(|package| package.name == name)
    }

    /// Validate schema version, package identities, checksums, and graph references.
    pub fn validate(&self) -> Result<(), Vs3PackageFormatError> {
        if self.lock_version != 1 {
            return invalid(format!(
                "unsupported lockfile version {}; expected 1",
                self.lock_version
            ));
        }
        let names = self
            .packages
            .iter()
            .map(|package| package.name.as_str())
            .collect::<BTreeSet<_>>();
        if names.len() != self.packages.len() {
            return invalid("lockfile contains duplicate package names".into());
        }
        for package in &self.packages {
            if !valid_identifier(&package.name) {
                return invalid(format!("locked package name `{}` is invalid", package.name));
            }
            if !package.source.starts_with("path+") || package.source.len() <= "path+".len() {
                return invalid(format!(
                    "locked package `{}` has invalid source `{}`",
                    package.name, package.source
                ));
            }
            if !valid_checksum(&package.checksum) {
                return invalid(format!(
                    "locked package `{}` has invalid SHA-256 checksum",
                    package.name
                ));
            }
            let mut seen = BTreeSet::new();
            for dependency in &package.dependencies {
                if dependency == &package.name {
                    return invalid(format!(
                        "locked package `{}` depends on itself",
                        package.name
                    ));
                }
                if !seen.insert(dependency) {
                    return invalid(format!(
                        "locked package `{}` repeats dependency `{dependency}`",
                        package.name
                    ));
                }
                if !names.contains(dependency.as_str()) {
                    return invalid(format!(
                        "locked package `{}` references missing dependency `{dependency}`",
                        package.name
                    ));
                }
            }
        }
        Ok(())
    }

    fn canonicalize(&mut self) {
        for package in &mut self.packages {
            package.dependencies.sort();
            package.dependencies.dedup();
        }
        self.packages
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
}

fn valid_checksum(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn invalid<T>(message: String) -> Result<T, Vs3PackageFormatError> {
    Err(Vs3PackageFormatError::Invalid(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, dependencies: &[&str]) -> Vs3LockedPackage {
        Vs3LockedPackage {
            name: name.into(),
            version: Version::new(1, 2, 3),
            source: format!("path+../{name}"),
            checksum: format!("sha256:{}", "a".repeat(64)),
            dependencies: dependencies.iter().map(|value| (*value).into()).collect(),
        }
    }

    #[test]
    fn render_is_sorted_and_round_trips() {
        let lock =
            Vs3PackageLock::new(vec![package("game", &["math"]), package("math", &[])]).unwrap();
        let rendered = lock.render().unwrap();
        assert!(
            rendered.find("name = \"game\"").unwrap() < rendered.find("name = \"math\"").unwrap()
        );
        assert_eq!(Vs3PackageLock::parse(&rendered).unwrap(), lock);
    }

    #[test]
    fn rejects_missing_dependency_and_bad_checksum() {
        assert!(Vs3PackageLock::new(vec![package("game", &["missing"])]).is_err());
        let mut bad = package("game", &[]);
        bad.checksum = "sha256:ABC".into();
        assert!(Vs3PackageLock::new(vec![bad]).is_err());
    }

    #[test]
    fn rejects_duplicate_names_and_unknown_schema() {
        assert!(Vs3PackageLock::new(vec![package("game", &[]), package("game", &[])]).is_err());
        let source = "version = 2\npackage = []\n";
        assert!(Vs3PackageLock::parse(source).is_err());
    }
}
