//! Known Velvet modules, enable flags, and inter-module dependencies.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

/// Built-in module identifiers recognized by the engine tooling.
pub const KNOWN_MODULES: &[&str] = &[
    "core", "app", "story", "play", "rpg", "action", "ui", "render", "audio", "input", "scene",
    "ecs", "assets", "script",
];

/// Descriptor for a logical engine module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Short id used in `velvet.project` `modules` list.
    pub id: String,
    /// Human-readable name.
    pub display_name: String,
    /// One-line purpose.
    pub description: String,
    /// Module ids that must also be enabled.
    pub requires: Vec<String>,
    /// Optional soft recommendations (warnings only).
    pub recommends: Vec<String>,
    /// Whether this is a high-level gameplay module (vs infrastructure).
    pub gameplay: bool,
}

/// Registry of known modules and their dependency edges.
#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    modules: BTreeMap<String, ModuleInfo>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

impl ModuleRegistry {
    /// Built-in Velvet module graph.
    pub fn builtin() -> Self {
        let mut reg = Self {
            modules: BTreeMap::new(),
        };
        reg.insert(ModuleInfo {
            id: "core".into(),
            display_name: "Core".into(),
            description: "Config, errors, diagnostics, versioning".into(),
            requires: vec![],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "app".into(),
            display_name: "App".into(),
            description: "Application loop, schedules, plugins".into(),
            requires: vec!["core".into()],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "assets".into(),
            display_name: "Assets".into(),
            description: "Handles, loaders, hot-reload".into(),
            requires: vec!["core".into()],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "ecs".into(),
            display_name: "ECS".into(),
            description: "Entities, components, queries".into(),
            requires: vec!["core".into()],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "scene".into(),
            display_name: "Scene".into(),
            description: "Scene graph, prefabs, load/unload".into(),
            requires: vec!["ecs".into()],
            recommends: vec!["assets".into()],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "render".into(),
            display_name: "Render".into(),
            description: "wgpu 2D rendering".into(),
            requires: vec!["core".into(), "assets".into()],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "audio".into(),
            display_name: "Audio".into(),
            description: "Buses, music, SFX, voice".into(),
            requires: vec!["core".into()],
            recommends: vec!["assets".into()],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "input".into(),
            display_name: "Input".into(),
            description: "Actions, bindings, devices".into(),
            requires: vec!["core".into()],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "ui".into(),
            display_name: "UI".into(),
            description: "Widgets, layout, dialogue UI".into(),
            requires: vec!["core".into()],
            recommends: vec!["render".into(), "input".into()],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "script".into(),
            display_name: "Script".into(),
            description: "Velvet Script pipeline (parse/compile/VM)".into(),
            requires: vec!["core".into()],
            recommends: vec![],
            gameplay: false,
        });
        reg.insert(ModuleInfo {
            id: "story".into(),
            display_name: "Story".into(),
            description: "Visual novel / narrative runtime".into(),
            requires: vec!["script".into()],
            recommends: vec!["ui".into(), "audio".into()],
            gameplay: true,
        });
        reg.insert(ModuleInfo {
            id: "play".into(),
            display_name: "Play".into(),
            description: "2D maps, physics, camera, AI helpers".into(),
            requires: vec!["ecs".into()],
            recommends: vec!["input".into(), "render".into(), "scene".into()],
            gameplay: true,
        });
        reg.insert(ModuleInfo {
            id: "rpg".into(),
            display_name: "RPG".into(),
            description: "Stats, inventory, quests, party".into(),
            requires: vec!["play".into()],
            recommends: vec!["story".into(), "ui".into()],
            gameplay: true,
        });
        reg.insert(ModuleInfo {
            id: "action".into(),
            display_name: "Action".into(),
            description: "Combat, weapons, enemies, score".into(),
            requires: vec!["play".into()],
            recommends: vec!["input".into()],
            gameplay: true,
        });
        reg
    }

    fn insert(&mut self, info: ModuleInfo) {
        self.modules.insert(info.id.clone(), info);
    }

    /// Lookup module info.
    pub fn get(&self, id: &str) -> Option<&ModuleInfo> {
        self.modules.get(id)
    }

    /// All known module ids.
    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.modules.keys().map(|s| s.as_str())
    }

    /// Whether id is known.
    pub fn is_known(&self, id: &str) -> bool {
        self.modules.contains_key(id)
    }

    /// Resolve transitive requirements for a set of enabled modules.
    ///
    /// Returns the full set (enabled ∪ required) in dependency order
    /// (dependencies first), or an error description if a cycle / missing node.
    pub fn resolve_dependencies(
        &self,
        enabled: &[String],
    ) -> Result<Vec<String>, ModuleResolveError> {
        let mut needed: BTreeSet<String> = BTreeSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        for m in enabled {
            if !self.is_known(m) {
                // Unknown modules are allowed as project-specific flags but
                // cannot contribute graph edges.
                needed.insert(m.clone());
                continue;
            }
            if needed.insert(m.clone()) {
                queue.push_back(m.clone());
            }
        }

        while let Some(id) = queue.pop_front() {
            if let Some(info) = self.get(&id) {
                for req in &info.requires {
                    if !self.is_known(req) {
                        return Err(ModuleResolveError::UnknownDependency {
                            module: id.clone(),
                            missing: req.clone(),
                        });
                    }
                    if needed.insert(req.clone()) {
                        queue.push_back(req.clone());
                    }
                }
            }
        }

        // Topological sort (Kahn)
        let mut indeg: BTreeMap<String, usize> = BTreeMap::new();
        let mut adj: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for id in &needed {
            indeg.entry(id.clone()).or_insert(0);
            adj.entry(id.clone()).or_default();
        }
        for id in &needed {
            if let Some(info) = self.get(id) {
                for req in &info.requires {
                    if needed.contains(req) {
                        // edge req -> id (req must come first)
                        adj.entry(req.clone()).or_default().push(id.clone());
                        *indeg.entry(id.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        let mut ready: VecDeque<String> = indeg
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(k, _)| k.clone())
            .collect();
        // stable-ish order
        let mut ready_vec: Vec<String> = ready.drain(..).collect();
        ready_vec.sort();
        ready.extend(ready_vec);

        let mut order = Vec::new();
        while let Some(n) = ready.pop_front() {
            order.push(n.clone());
            if let Some(children) = adj.get(&n) {
                let mut next_ready = Vec::new();
                for c in children {
                    if let Some(d) = indeg.get_mut(c) {
                        *d = d.saturating_sub(1);
                        if *d == 0 {
                            next_ready.push(c.clone());
                        }
                    }
                }
                next_ready.sort();
                for c in next_ready {
                    ready.push_back(c);
                }
            }
        }

        if order.len() != needed.len() {
            return Err(ModuleResolveError::Cycle);
        }
        Ok(order)
    }

    /// Modules that are required transitively but not listed in `enabled`.
    pub fn missing_dependencies(
        &self,
        enabled: &[String],
    ) -> Result<Vec<String>, ModuleResolveError> {
        let full = self.resolve_dependencies(enabled)?;
        let enabled_set: BTreeSet<&str> = enabled.iter().map(|s| s.as_str()).collect();
        Ok(full
            .into_iter()
            .filter(|m| !enabled_set.contains(m.as_str()))
            .collect())
    }

    /// Soft recommendation warnings for enabled modules.
    pub fn recommendation_warnings(&self, enabled: &[String]) -> Vec<String> {
        let set: BTreeSet<&str> = enabled.iter().map(|s| s.as_str()).collect();
        let mut warns = Vec::new();
        for m in enabled {
            if let Some(info) = self.get(m) {
                for rec in &info.recommends {
                    if !set.contains(rec.as_str()) {
                        warns.push(format!("module `{m}` recommends `{rec}` (not enabled)"));
                    }
                }
            }
        }
        warns
    }
}

/// Dependency resolution failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleResolveError {
    /// A required dependency is not in the registry.
    UnknownDependency {
        /// Module that declared the requirement.
        module: String,
        /// Missing id.
        missing: String,
    },
    /// Cycle detected in the graph.
    Cycle,
}

impl std::fmt::Display for ModuleResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownDependency { module, missing } => {
                write!(f, "module `{module}` requires unknown `{missing}`")
            }
            Self::Cycle => write!(f, "module dependency cycle detected"),
        }
    }
}

impl std::error::Error for ModuleResolveError {}

/// Expand enabled modules to include all hard dependencies.
pub fn enable_with_deps(enabled: &[String]) -> Result<Vec<String>, ModuleResolveError> {
    ModuleRegistry::builtin().resolve_dependencies(enabled)
}

/// Whether enabling `module` is valid given current `enabled` (deps may be auto-added).
pub fn can_enable(enabled: &[String], module: &str) -> Result<Vec<String>, ModuleResolveError> {
    let mut next = enabled.to_vec();
    if !next.iter().any(|m| m == module) {
        next.push(module.to_string());
    }
    ModuleRegistry::builtin().resolve_dependencies(&next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpg_pulls_play_and_ecs() {
        let reg = ModuleRegistry::builtin();
        let order = reg.resolve_dependencies(&["rpg".into()]).unwrap();
        assert!(order.contains(&"rpg".into()));
        assert!(order.contains(&"play".into()));
        assert!(order.contains(&"ecs".into()));
        assert!(order.contains(&"core".into()));
        // ecs before play before rpg
        let i_ecs = order.iter().position(|m| m == "ecs").unwrap();
        let i_play = order.iter().position(|m| m == "play").unwrap();
        let i_rpg = order.iter().position(|m| m == "rpg").unwrap();
        assert!(i_ecs < i_play);
        assert!(i_play < i_rpg);
    }

    #[test]
    fn missing_deps_listed() {
        let reg = ModuleRegistry::builtin();
        let miss = reg.missing_dependencies(&["story".into()]).unwrap();
        assert!(miss.iter().any(|m| m == "script"));
    }

    #[test]
    fn recommendations() {
        let reg = ModuleRegistry::builtin();
        let w = reg.recommendation_warnings(&["story".into()]);
        assert!(!w.is_empty());
    }

    #[test]
    fn unknown_module_passthrough() {
        let reg = ModuleRegistry::builtin();
        let order = reg
            .resolve_dependencies(&["my_mod".into(), "story".into()])
            .unwrap();
        assert!(order.contains(&"my_mod".into()));
        assert!(order.contains(&"script".into()));
    }
}
