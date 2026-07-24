//! Offline, deterministic resolution for versioned local VS3 packages.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use semver::VersionReq;
use sha2::{Digest, Sha256};
use velvet_script_ast::Item;
use velvet_script_parser::parse_file;

use crate::{
    Vs3Error, Vs3LockedPackage, Vs3PackageLock, Vs3PackageManifest, VS3_PACKAGE_LOCK,
    VS3_PACKAGE_MANIFEST,
};

#[derive(Debug)]
pub(crate) struct ResolvedPackageGraph {
    pub(crate) manifest: Vs3PackageManifest,
    pub(crate) lock: Vs3PackageLock,
    pub(crate) bundle_root: String,
    pub(crate) sources: BTreeMap<String, String>,
    pub(crate) aliases: BTreeMap<String, String>,
}

#[derive(Debug)]
struct ResolvedPackage {
    manifest: Vs3PackageManifest,
    directory: PathBuf,
    manifest_source: String,
    sources: BTreeMap<String, String>,
    dependency_names: Vec<String>,
}

#[derive(Default)]
struct ResolveState {
    packages: BTreeMap<String, ResolvedPackage>,
    directories: BTreeMap<PathBuf, String>,
    visiting: Vec<PathBuf>,
}

pub(crate) fn resolve_unlocked(path: &Path) -> Result<ResolvedPackageGraph, Vs3Error> {
    let root_manifest_path = locate_manifest(path)?;
    let root_directory = root_manifest_path
        .parent()
        .expect("manifest has a parent")
        .canonicalize()
        .map_err(|error| package_error(&root_manifest_path, error))?;
    let mut state = ResolveState::default();
    let root_name = resolve_one(&root_manifest_path, None, &mut state)?;
    build_graph(root_name, root_directory, state)
}

pub(crate) fn resolve_locked(path: &Path) -> Result<ResolvedPackageGraph, Vs3Error> {
    let graph = resolve_unlocked(path)?;
    let manifest_path = locate_manifest(path)?;
    let lock_path = manifest_path
        .parent()
        .expect("manifest has a parent")
        .join(VS3_PACKAGE_LOCK);
    let existing = Vs3PackageLock::load(&lock_path).map_err(|error| Vs3Error::Bundle {
        path: lock_path.display().to_string(),
        message: format!(
            "a current `{VS3_PACKAGE_LOCK}` is required; run the lock update first: {error}"
        ),
    })?;
    if existing != graph.lock {
        return Err(Vs3Error::Bundle {
            path: lock_path.display().to_string(),
            message: format!("`{VS3_PACKAGE_LOCK}` is stale; regenerate it before compiling"),
        });
    }
    Ok(graph)
}

pub(crate) fn update_lock(path: &Path) -> Result<Vs3PackageLock, Vs3Error> {
    let graph = resolve_unlocked(path)?;
    let manifest_path = locate_manifest(path)?;
    let lock_path = manifest_path
        .parent()
        .expect("manifest has a parent")
        .join(VS3_PACKAGE_LOCK);
    graph
        .lock
        .write(&lock_path)
        .map_err(|error| Vs3Error::Bundle {
            path: lock_path.display().to_string(),
            message: error.to_string(),
        })?;
    Ok(graph.lock)
}

fn resolve_one(
    manifest_path: &Path,
    expected: Option<(&str, &VersionReq)>,
    state: &mut ResolveState,
) -> Result<String, Vs3Error> {
    let manifest_path = manifest_path
        .canonicalize()
        .map_err(|error| package_error(manifest_path, error))?;
    let directory = manifest_path
        .parent()
        .expect("manifest path has parent")
        .to_path_buf();
    if let Some(index) = state
        .visiting
        .iter()
        .position(|path| path == &manifest_path)
    {
        let mut cycle = state.visiting[index..]
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(manifest_path.display().to_string());
        return Err(Vs3Error::Bundle {
            path: manifest_path.display().to_string(),
            message: format!("cyclic package dependency: {}", cycle.join(" -> ")),
        });
    }

    let manifest_source =
        fs::read_to_string(&manifest_path).map_err(|error| package_error(&manifest_path, error))?;
    let manifest =
        Vs3PackageManifest::parse(&manifest_source).map_err(|error| Vs3Error::Bundle {
            path: manifest_path.display().to_string(),
            message: error.to_string(),
        })?;
    if let Some((expected_name, requirement)) = expected {
        if manifest.name != expected_name {
            return Err(Vs3Error::Bundle {
                path: manifest_path.display().to_string(),
                message: format!(
                    "dependency `{expected_name}` resolved package `{}`",
                    manifest.name
                ),
            });
        }
        if !requirement.matches(&manifest.version) {
            return Err(Vs3Error::Bundle {
                path: manifest_path.display().to_string(),
                message: format!(
                    "dependency `{expected_name}` requires `{requirement}` but resolved `{}`",
                    manifest.version
                ),
            });
        }
    }
    if let Some(existing_name) = state.directories.get(&directory) {
        if existing_name != &manifest.name {
            return Err(Vs3Error::Bundle {
                path: manifest_path.display().to_string(),
                message: format!(
                    "package directory is already registered as `{existing_name}`, not `{}`",
                    manifest.name
                ),
            });
        }
    }
    if let Some(existing) = state.packages.get(&manifest.name) {
        if existing.directory != directory || existing.manifest.version != manifest.version {
            return Err(Vs3Error::Bundle {
                path: manifest_path.display().to_string(),
                message: format!(
                    "package `{}` resolves to conflicting paths or versions",
                    manifest.name
                ),
            });
        }
        return Ok(manifest.name);
    }

    state.visiting.push(manifest_path.clone());
    let sources = load_package_sources(&directory, &manifest)?;
    let mut dependency_names = Vec::new();
    for (name, dependency) in &manifest.dependencies {
        let dependency_dir = directory.join(Path::new(&dependency.path));
        let dependency_manifest = dependency_dir.join(VS3_PACKAGE_MANIFEST);
        let resolved_name = resolve_one(
            &dependency_manifest,
            Some((name, &dependency.requirement)),
            state,
        )?;
        dependency_names.push(resolved_name);
    }
    state.visiting.pop();
    dependency_names.sort();
    state
        .directories
        .insert(directory.clone(), manifest.name.clone());
    let name = manifest.name.clone();
    state.packages.insert(
        name.clone(),
        ResolvedPackage {
            manifest,
            directory,
            manifest_source,
            sources,
            dependency_names,
        },
    );
    Ok(name)
}

fn load_package_sources(
    directory: &Path,
    manifest: &Vs3PackageManifest,
) -> Result<BTreeMap<String, String>, Vs3Error> {
    let canonical_root = directory
        .canonicalize()
        .map_err(|error| package_error(directory, error))?;
    let mut pending = manifest.modules.values().cloned().collect::<Vec<_>>();
    let mut sources = BTreeMap::new();
    while let Some(relative) = pending.pop() {
        if sources.contains_key(&relative) {
            continue;
        }
        let actual = canonical_root.join(path_from_virtual(&relative));
        let canonical = actual
            .canonicalize()
            .map_err(|error| package_error(&actual, error))?;
        if !canonical.starts_with(&canonical_root) {
            return Err(Vs3Error::Bundle {
                path: relative,
                message: "package source escapes its declared directory".into(),
            });
        }
        let source =
            fs::read_to_string(&canonical).map_err(|error| package_error(&canonical, error))?;
        let parsed = parse_file(&source, Some(&relative)).map_err(|error| Vs3Error::Bundle {
            path: relative.clone(),
            message: format!("cannot parse package source: {error}"),
        })?;
        for item in parsed.module.items {
            if let Item::Import { path, .. } = item {
                if is_relative_source_import(&path) {
                    pending.push(normalize_relative_source(&relative, &path)?);
                }
            }
        }
        sources.insert(relative, source);
    }
    Ok(sources)
}

fn build_graph(
    root_name: String,
    root_directory: PathBuf,
    state: ResolveState,
) -> Result<ResolvedPackageGraph, Vs3Error> {
    let root_manifest = state
        .packages
        .get(&root_name)
        .expect("resolved root package exists")
        .manifest
        .clone();
    let mut targets = BTreeMap::new();
    for package in state.packages.values() {
        for (module, relative) in &package.manifest.modules {
            let virtual_path = virtual_package_path(&package.manifest.name, relative);
            if targets.insert(module.clone(), virtual_path).is_some() {
                return Err(Vs3Error::Bundle {
                    path: module.clone(),
                    message: "duplicate stable module identity".into(),
                });
            }
        }
    }

    let mut sources = BTreeMap::new();
    for package in state.packages.values() {
        for (relative, source) in &package.sources {
            let virtual_path = virtual_package_path(&package.manifest.name, relative);
            let rewritten = rewrite_stable_imports(&virtual_path, source, &targets)?;
            sources.insert(virtual_path, rewritten);
        }
    }

    let bundle_root = "__vs3_package_root.vel".to_string();
    let mut aliases = BTreeMap::new();
    let mut synthetic = String::from("// @edition 3\n");
    for (index, (module, target)) in targets.iter().enumerate() {
        let alias = format!("_package_{index}");
        synthetic.push_str(&format!("import \"{target}\" as {alias}\n"));
        aliases.insert(alias, module.clone());
    }
    sources.insert(bundle_root.clone(), synthetic);

    let mut locked = Vec::new();
    for package in state.packages.values() {
        locked.push(Vs3LockedPackage {
            name: package.manifest.name.clone(),
            version: package.manifest.version.clone(),
            source: format!(
                "path+{}",
                relative_directory(&root_directory, &package.directory)?
            ),
            checksum: checksum_package(package),
            dependencies: package.dependency_names.clone(),
        });
    }
    let lock = Vs3PackageLock::new(locked).map_err(|error| Vs3Error::Bundle {
        path: VS3_PACKAGE_LOCK.into(),
        message: error.to_string(),
    })?;
    Ok(ResolvedPackageGraph {
        manifest: root_manifest,
        lock,
        bundle_root,
        sources,
        aliases,
    })
}

fn rewrite_stable_imports(
    owner: &str,
    source: &str,
    targets: &BTreeMap<String, String>,
) -> Result<String, Vs3Error> {
    let mut output = String::with_capacity(source.len());
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let Some(after_import) = trimmed.strip_prefix("import \"") else {
            output.push_str(line);
            continue;
        };
        let Some(end) = after_import.find('"') else {
            return Err(Vs3Error::Bundle {
                path: owner.into(),
                message: "unterminated import path".into(),
            });
        };
        let target = &after_import[..end];
        if is_relative_source_import(target) {
            output.push_str(line);
            continue;
        }
        let Some(virtual_target) = targets.get(target) else {
            return Err(Vs3Error::Bundle {
                path: owner.into(),
                message: format!("unknown stable package module `{target}`"),
            });
        };
        let replacement = relative_virtual_path(owner, virtual_target)?;
        let prefix_len = line.len() - trimmed.len() + "import \"".len();
        let end_index = prefix_len + target.len();
        output.push_str(&line[..prefix_len]);
        output.push_str(&replacement);
        output.push_str(&line[end_index..]);
    }
    Ok(output)
}

fn checksum_package(package: &ResolvedPackage) -> String {
    let mut digest = Sha256::new();
    digest.update(b"velvet-package-v1\0");
    digest.update(package.manifest_source.as_bytes());
    digest.update([0]);
    for (path, source) in &package.sources {
        digest.update(path.as_bytes());
        digest.update([0]);
        digest.update(source.as_bytes());
        digest.update([0xff]);
    }
    format!("sha256:{:x}", digest.finalize())
}

fn locate_manifest(path: &Path) -> Result<PathBuf, Vs3Error> {
    let candidate = if path.is_dir() {
        path.join(VS3_PACKAGE_MANIFEST)
    } else {
        path.to_path_buf()
    };
    if candidate.file_name().and_then(|name| name.to_str()) != Some(VS3_PACKAGE_MANIFEST) {
        return Err(Vs3Error::Bundle {
            path: candidate.display().to_string(),
            message: format!("expected `{VS3_PACKAGE_MANIFEST}` or its directory"),
        });
    }
    candidate
        .canonicalize()
        .map_err(|error| package_error(&candidate, error))
}

fn normalize_relative_source(current: &str, target: &str) -> Result<String, Vs3Error> {
    let mut parts = current.split('/').collect::<Vec<_>>();
    parts.pop();
    let normalized_target = target.replace('\\', "/");
    for part in normalized_target.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(Vs3Error::Bundle {
                        path: current.into(),
                        message: format!("relative import `{target}` escapes its package"),
                    });
                }
            }
            value => parts.push(value),
        }
    }
    if parts.is_empty() {
        return Err(Vs3Error::Bundle {
            path: current.into(),
            message: format!("invalid relative import `{target}`"),
        });
    }
    Ok(parts.join("/"))
}

fn is_relative_source_import(target: &str) -> bool {
    target.ends_with(".vel") || target.contains('/') || target.contains('\\')
}

fn virtual_package_path(package: &str, relative: &str) -> String {
    format!("packages/{package}/{relative}")
}

fn path_from_virtual(value: &str) -> PathBuf {
    value.split('/').collect()
}

fn relative_virtual_path(owner: &str, target: &str) -> Result<String, Vs3Error> {
    let mut from = owner.split('/').collect::<Vec<_>>();
    from.pop();
    let to = target.split('/').collect::<Vec<_>>();
    let mut common = 0;
    while common < from.len() && common < to.len() && from[common] == to[common] {
        common += 1;
    }
    let mut parts = vec![".."; from.len() - common];
    parts.extend_from_slice(&to[common..]);
    if parts.is_empty() {
        return Err(Vs3Error::Bundle {
            path: owner.into(),
            message: "module cannot import itself".into(),
        });
    }
    Ok(parts.join("/"))
}

fn relative_directory(root: &Path, target: &Path) -> Result<String, Vs3Error> {
    let root_parts = root.components().collect::<Vec<_>>();
    let target_parts = target.components().collect::<Vec<_>>();
    let mut common = 0;
    while common < root_parts.len()
        && common < target_parts.len()
        && root_parts[common] == target_parts[common]
    {
        common += 1;
    }
    if common == 0
        || matches!(root_parts.first(), Some(Component::Prefix(_)))
            && root_parts.first() != target_parts.first()
    {
        return Err(Vs3Error::Bundle {
            path: target.display().to_string(),
            message: "dependency is on a different filesystem volume".into(),
        });
    }
    let mut parts = vec!["..".to_string(); root_parts.len() - common];
    parts.extend(
        target_parts[common..]
            .iter()
            .filter_map(|component| match component {
                Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
                _ => None,
            }),
    );
    if parts.is_empty() {
        Ok(".".into())
    } else {
        Ok(parts.join("/"))
    }
}

fn package_error(path: &Path, error: impl std::fmt::Display) -> Vs3Error {
    Vs3Error::Bundle {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("velvet-vs3-package-{}-{id}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn write(&self, relative: &str, source: &str) {
            let path = self.root.join(path_from_virtual(relative));
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, source).unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn write_graph(fixture: &Fixture, requirement: &str) {
        fixture.write(
            "game/velvet.package.toml",
            &format!(
                r#"[package]
name = "game"
version = "1.0.0"
edition = 3
root = "game.entry"

[modules]
"game.entry" = "entry.vel"

[dependencies]
math = {{ version = "{requirement}", path = "../math" }}
"#
            ),
        );
        fixture.write(
            "game/entry.vel",
            "// @edition 3\nimport \"math.core\" as math\nexport function run(v: int) { return math.double(v) + 1 }\n",
        );
        fixture.write(
            "math/velvet.package.toml",
            r#"[package]
name = "math"
version = "2.1.0"
edition = 3
root = "math.core"

[modules]
"math.core" = "core.vel"
"#,
        );
        fixture.write(
            "math/core.vel",
            "// @edition 3\nexport function double(v: int) { return v * 2 }\n",
        );
    }

    #[test]
    fn resolves_versioned_graph_and_stable_imports() {
        let fixture = Fixture::new();
        write_graph(&fixture, "^2.0");
        let graph = resolve_unlocked(&fixture.root.join("game")).unwrap();
        assert_eq!(graph.manifest.name, "game");
        assert_eq!(graph.lock.packages.len(), 2);
        assert!(graph.sources["packages/game/entry.vel"].contains("../math/core.vel"));
        assert_eq!(graph.aliases.len(), 2);
    }

    #[test]
    fn rejects_version_mismatch_and_dependency_cycle() {
        let fixture = Fixture::new();
        write_graph(&fixture, "^3.0");
        assert!(resolve_unlocked(&fixture.root.join("game")).is_err());

        write_graph(&fixture, "^2.0");
        fixture.write(
            "math/velvet.package.toml",
            r#"[package]
name = "math"
version = "2.1.0"
edition = 3
root = "math.core"
[modules]
"math.core" = "core.vel"
[dependencies]
game = { version = "^1", path = "../game" }
"#,
        );
        assert!(resolve_unlocked(&fixture.root.join("game")).is_err());
    }

    #[test]
    fn locked_package_compiles_and_exposes_canonical_functions() {
        let fixture = Fixture::new();
        write_graph(&fixture, "^2.0");
        let root = fixture.root.join("game");
        update_lock(&root).unwrap();
        let module = crate::compile_package_path(&root).unwrap();
        assert_eq!(
            module.function_names(),
            vec!["game.entry.run", "math.core.double"]
        );
        assert_eq!(
            module
                .call("game.entry.run", &[crate::Value::Int(20)])
                .unwrap(),
            crate::Value::Int(41)
        );
        assert_eq!(
            module
                .call("math.core.double", &[crate::Value::Int(21)])
                .unwrap(),
            crate::Value::Int(42)
        );
    }

    #[test]
    fn locked_resolution_detects_changed_sources() {
        let fixture = Fixture::new();
        write_graph(&fixture, "^2.0");
        update_lock(&fixture.root.join("game")).unwrap();
        resolve_locked(&fixture.root.join("game")).unwrap();
        fixture.write(
            "math/core.vel",
            "// @edition 3\nexport function double(v: int) { return v * 3 }\n",
        );
        let error = resolve_locked(&fixture.root.join("game")).unwrap_err();
        assert!(error.to_string().contains("stale"));
    }
}
