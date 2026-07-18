//! Plugin registry helpers: version checks, dependency graphs, enable sets.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::plugin::{PluginError, PluginId, PluginInfo, VersionReq};
use crate::version::Version;

/// Declared dependency with optional version requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginDependency {
    /// Dependency id.
    pub id: PluginId,
    /// Optional version requirement.
    pub version_req: Option<VersionReq>,
}

impl PluginDependency {
    /// Id only.
    pub fn id(id: impl Into<PluginId>) -> Self {
        Self {
            id: id.into(),
            version_req: None,
        }
    }

    /// With version requirement.
    pub fn with_req(id: impl Into<PluginId>, req: VersionReq) -> Self {
        Self {
            id: id.into(),
            version_req: Some(req),
        }
    }
}

/// Runtime registration entry (metadata only — build hooks live in velvet-app).
#[derive(Debug, Clone)]
pub struct PluginEntry {
    /// Info.
    pub info: PluginInfo,
    /// Dependencies with version requirements.
    pub dependencies: Vec<PluginDependency>,
    /// Whether enabled.
    pub enabled: bool,
    /// Optional required engine version.
    pub engine_req: Option<VersionReq>,
}

impl PluginEntry {
    /// From static info with id-only deps.
    pub fn from_info(info: PluginInfo) -> Self {
        let dependencies = info
            .dependencies
            .iter()
            .map(|d| PluginDependency::id(d.clone()))
            .collect();
        Self {
            info,
            dependencies,
            enabled: true,
            engine_req: None,
        }
    }
}

/// Registry of plugin metadata for validation and ordering.
#[derive(Debug, Default, Clone)]
pub struct PluginRegistry {
    entries: HashMap<PluginId, PluginEntry>,
}

impl PluginRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an entry; errors on duplicate id.
    pub fn register(&mut self, entry: PluginEntry) -> Result<(), PluginError> {
        let id = entry.info.id.clone();
        if self.entries.contains_key(&id) {
            return Err(PluginError::Duplicate(id));
        }
        self.entries.insert(id, entry);
        Ok(())
    }

    /// Insert or replace.
    pub fn insert(&mut self, entry: PluginEntry) {
        self.entries.insert(entry.info.id.clone(), entry);
    }

    /// Get entry.
    pub fn get(&self, id: &PluginId) -> Option<&PluginEntry> {
        self.entries.get(id)
    }

    /// Mutable get.
    pub fn get_mut(&mut self, id: &PluginId) -> Option<&mut PluginEntry> {
        self.entries.get_mut(id)
    }

    /// Enable plugin.
    pub fn set_enabled(&mut self, id: &PluginId, enabled: bool) -> Result<(), PluginError> {
        self.entries
            .get_mut(id)
            .map(|e| e.enabled = enabled)
            .ok_or_else(|| PluginError::Disabled(id.clone()))
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate entries.
    pub fn iter(&self) -> impl Iterator<Item = &PluginEntry> {
        self.entries.values()
    }

    /// Check engine version requirements for all enabled plugins.
    pub fn check_engine_version(&self, engine: Version) -> Result<(), PluginError> {
        for e in self.entries.values() {
            if !e.enabled {
                continue;
            }
            if let Some(req) = &e.engine_req {
                if !req.matches(engine) {
                    return Err(PluginError::VersionMismatch {
                        plugin: e.info.id.clone(),
                        found: engine.to_string(),
                        required_by: PluginId::new("engine"),
                    });
                }
            }
        }
        Ok(())
    }

    /// Check dependency version requirements among registered plugins.
    pub fn check_dependency_versions(&self) -> Result<(), PluginError> {
        for e in self.entries.values() {
            if !e.enabled {
                continue;
            }
            for dep in &e.dependencies {
                let Some(other) = self.entries.get(&dep.id) else {
                    return Err(PluginError::MissingDependency {
                        plugin: e.info.id.clone(),
                        missing: dep.id.clone(),
                    });
                };
                if !other.enabled {
                    return Err(PluginError::MissingDependency {
                        plugin: e.info.id.clone(),
                        missing: dep.id.clone(),
                    });
                }
                if let Some(req) = &dep.version_req {
                    if !req.matches(other.info.version) {
                        return Err(PluginError::VersionMismatch {
                            plugin: dep.id.clone(),
                            found: other.info.version.to_string(),
                            required_by: e.info.id.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Topological order of enabled plugins (Kahn).
    pub fn resolve_order(&self) -> Result<Vec<PluginId>, PluginError> {
        let enabled: HashSet<PluginId> = self
            .entries
            .values()
            .filter(|e| e.enabled)
            .map(|e| e.info.id.clone())
            .collect();

        let mut indegree: HashMap<PluginId, usize> = HashMap::new();
        let mut adj: HashMap<PluginId, Vec<PluginId>> = HashMap::new();

        for id in &enabled {
            indegree.entry(id.clone()).or_insert(0);
            adj.entry(id.clone()).or_default();
        }

        for e in self.entries.values() {
            if !e.enabled {
                continue;
            }
            for dep in &e.dependencies {
                if !enabled.contains(&dep.id) {
                    return Err(PluginError::MissingDependency {
                        plugin: e.info.id.clone(),
                        missing: dep.id.clone(),
                    });
                }
                // edge: dep -> e (dep must come first)
                adj.entry(dep.id.clone())
                    .or_default()
                    .push(e.info.id.clone());
                *indegree.entry(e.info.id.clone()).or_insert(0) += 1;
            }
        }

        let mut q: VecDeque<PluginId> = indegree
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| id.clone())
            .collect();
        // Stable-ish order by id string.
        let mut q_vec: Vec<_> = q.drain(..).collect();
        q_vec.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        q.extend(q_vec);

        let mut order = Vec::new();
        while let Some(id) = q.pop_front() {
            order.push(id.clone());
            if let Some(children) = adj.get(&id) {
                let mut nexts = Vec::new();
                for child in children {
                    if let Some(d) = indegree.get_mut(child) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            nexts.push(child.clone());
                        }
                    }
                }
                nexts.sort_by(|a, b| a.as_str().cmp(b.as_str()));
                q.extend(nexts);
            }
        }

        if order.len() != enabled.len() {
            return Err(PluginError::Cycle(format!(
                "resolved {} of {} plugins",
                order.len(),
                enabled.len()
            )));
        }
        Ok(order)
    }

    /// Full validation: deps, versions, order.
    pub fn validate(&self, engine: Version) -> Result<Vec<PluginId>, PluginError> {
        self.check_engine_version(engine)?;
        self.check_dependency_versions()?;
        self.resolve_order()
    }

    /// Apply an allow-list of plugin id strings (empty list = all stay as-is).
    pub fn apply_enable_list(&mut self, allowed: &[String]) {
        if allowed.is_empty() {
            return;
        }
        let set: HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
        for (id, e) in self.entries.iter_mut() {
            e.enabled = set.contains(id.as_str()) || set.contains(e.info.name);
        }
    }
}

/// Check a single version requirement (standalone helper).
pub fn check_version_req(
    found: Version,
    req: &VersionReq,
    plugin: PluginId,
    required_by: PluginId,
) -> Result<(), PluginError> {
    if req.matches(found) {
        Ok(())
    } else {
        Err(PluginError::VersionMismatch {
            plugin,
            found: found.to_string(),
            required_by,
        })
    }
}

/// Parse a simple requirement string `"1.2"` meaning major=1, minor_min=2, patch_min=0.
pub fn parse_simple_req(s: &str) -> Option<VersionReq> {
    let v = Version::parse(s)?;
    Some(VersionReq::compatible(v.major, v.minor, v.patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, ver: Version, deps: &[&str]) -> PluginEntry {
        // Leak dep ids for 'static slice on PluginInfo — use owned deps on entry instead.
        let info = PluginInfo {
            id: PluginId::new(id),
            name: "test",
            version: ver,
            dependencies: &[],
        };
        let mut e = PluginEntry::from_info(info);
        e.dependencies = deps
            .iter()
            .map(|d| PluginDependency::id(PluginId::new(*d)))
            .collect();
        e
    }

    #[test]
    fn topo_order() {
        let mut reg = PluginRegistry::new();
        reg.register(entry("a", Version::new(1, 0, 0), &[]))
            .unwrap();
        reg.register(entry("b", Version::new(1, 0, 0), &["a"]))
            .unwrap();
        reg.register(entry("c", Version::new(1, 0, 0), &["b"]))
            .unwrap();
        let order = reg.resolve_order().unwrap();
        assert!(
            order.iter().position(|x| x.as_str() == "a")
                < order.iter().position(|x| x.as_str() == "b")
        );
        assert!(
            order.iter().position(|x| x.as_str() == "b")
                < order.iter().position(|x| x.as_str() == "c")
        );
    }

    #[test]
    fn version_mismatch() {
        let mut reg = PluginRegistry::new();
        reg.register(entry("a", Version::new(1, 0, 0), &[]))
            .unwrap();
        let mut b = entry("b", Version::new(1, 0, 0), &["a"]);
        b.dependencies = vec![PluginDependency::with_req(
            PluginId::new("a"),
            VersionReq::compatible(1, 2, 0),
        )];
        reg.register(b).unwrap();
        assert!(reg.check_dependency_versions().is_err());
    }

    #[test]
    fn cycle_detected() {
        let mut reg = PluginRegistry::new();
        reg.register(entry("a", Version::new(1, 0, 0), &["b"]))
            .unwrap();
        reg.register(entry("b", Version::new(1, 0, 0), &["a"]))
            .unwrap();
        assert!(matches!(reg.resolve_order(), Err(PluginError::Cycle(_))));
    }

    #[test]
    fn enable_list() {
        let mut reg = PluginRegistry::new();
        reg.register(entry("a", Version::new(1, 0, 0), &[]))
            .unwrap();
        reg.register(entry("b", Version::new(1, 0, 0), &[]))
            .unwrap();
        reg.apply_enable_list(&["a".into()]);
        assert!(reg.get(&PluginId::new("a")).unwrap().enabled);
        assert!(!reg.get(&PluginId::new("b")).unwrap().enabled);
    }

    #[test]
    fn parse_req() {
        let r = parse_simple_req("1.2.3").unwrap();
        assert!(r.matches(Version::new(1, 2, 3)));
        assert!(!r.matches(Version::new(1, 1, 9)));
    }
}
