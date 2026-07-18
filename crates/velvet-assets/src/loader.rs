//! Asset loaders.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;

use crate::path::AssetPath;
use crate::{BytesAsset, TextAsset};

/// Load failure.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Source/read failure.
    #[error("source: {0}")]
    Source(String),
    /// Decode failure.
    #[error("decode: {0}")]
    Decode(String),
    /// No loader for type/extension.
    #[error("no loader for {0}")]
    NoLoader(String),
    /// Type mismatch.
    #[error("type mismatch")]
    TypeMismatch,
}

/// Request to load an asset.
#[derive(Debug, Clone)]
pub struct LoadRequest {
    /// Path.
    pub path: AssetPath,
    /// Force reload even if cached.
    pub force: bool,
}

/// Typed loader interface (type-erased in registry).
pub trait AssetLoader: Send + Sync {
    /// Type name for diagnostics.
    fn type_name(&self) -> &'static str;
    /// Type id of produced asset.
    fn value_type(&self) -> TypeId;
    /// Extensions this loader handles (without dot), empty = any.
    fn extensions(&self) -> &[&'static str];
    /// Decode bytes into boxed Any asset.
    fn load(&self, path: &AssetPath, bytes: &[u8])
        -> Result<Box<dyn Any + Send + Sync>, LoadError>;
}

/// Bytes passthrough loader.
#[derive(Default)]
pub struct BytesLoader;

impl AssetLoader for BytesLoader {
    fn type_name(&self) -> &'static str {
        "BytesAsset"
    }
    fn value_type(&self) -> TypeId {
        TypeId::of::<BytesAsset>()
    }
    fn extensions(&self) -> &[&'static str] {
        &[]
    }
    fn load(
        &self,
        _path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Box<dyn Any + Send + Sync>, LoadError> {
        Ok(Box::new(BytesAsset {
            data: bytes.to_vec(),
        }))
    }
}

/// UTF-8 text loader.
#[derive(Default)]
pub struct TextLoader;

impl AssetLoader for TextLoader {
    fn type_name(&self) -> &'static str {
        "TextAsset"
    }
    fn value_type(&self) -> TypeId {
        TypeId::of::<TextAsset>()
    }
    fn extensions(&self) -> &[&'static str] {
        &["txt", "md", "vel", "ron", "json", "toml", "csv"]
    }
    fn load(
        &self,
        path: &AssetPath,
        bytes: &[u8],
    ) -> Result<Box<dyn Any + Send + Sync>, LoadError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| LoadError::Decode(format!("{path}: {e}")))?
            .to_string();
        Ok(Box::new(TextAsset { text }))
    }
}

/// Registry of loaders by TypeId and extension.
#[derive(Default)]
pub struct LoaderRegistry {
    by_type: HashMap<TypeId, Arc<dyn AssetLoader>>,
    by_ext: HashMap<String, Arc<dyn AssetLoader>>,
}

impl LoaderRegistry {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register defaults (bytes + text).
    pub fn with_defaults() -> Self {
        let mut r = Self::new();
        r.register(BytesLoader);
        r.register(TextLoader);
        r
    }

    /// Register a loader.
    pub fn register<L: AssetLoader + 'static>(&mut self, loader: L) {
        let loader: Arc<dyn AssetLoader> = Arc::new(loader);
        self.by_type
            .insert(loader.value_type(), Arc::clone(&loader));
        for ext in loader.extensions() {
            self.by_ext
                .insert(ext.to_ascii_lowercase(), Arc::clone(&loader));
        }
    }

    /// Loader for type.
    pub fn for_type<T: 'static>(&self) -> Option<Arc<dyn AssetLoader>> {
        self.by_type.get(&TypeId::of::<T>()).cloned()
    }

    /// Loader for extension.
    pub fn for_extension(&self, ext: &str) -> Option<Arc<dyn AssetLoader>> {
        self.by_ext.get(&ext.to_ascii_lowercase()).cloned()
    }

    /// Number of registered type loaders.
    pub fn type_count(&self) -> usize {
        self.by_type.len()
    }
}

/// Thin dependency graph for load ordering (path → depends-on paths).
#[derive(Debug, Clone, Default)]
pub struct DependencyGraph {
    /// Adjacency: asset → list of dependencies that must load first.
    edges: HashMap<String, Vec<String>>,
}

impl DependencyGraph {
    /// Empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare that `asset` depends on `dep` (dep must load before asset).
    pub fn add_dep(&mut self, asset: impl Into<String>, dep: impl Into<String>) {
        let asset = asset.into();
        let dep = dep.into();
        self.edges.entry(asset).or_default().push(dep);
    }

    /// Replace dependency list for an asset.
    pub fn set_deps(&mut self, asset: impl Into<String>, deps: Vec<String>) {
        self.edges.insert(asset.into(), deps);
    }

    /// Dependencies of an asset.
    pub fn deps_of(&self, asset: &str) -> &[String] {
        self.edges.get(asset).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Topological load order for `roots` (dependencies first). Returns `None` on cycle.
    pub fn load_order(&self, roots: &[String]) -> Option<Vec<String>> {
        let mut order = Vec::new();
        let mut visiting = HashMap::<String, u8>::new(); // 0=unseen,1=visiting,2=done
        for r in roots {
            if !self.dfs(r, &mut visiting, &mut order) {
                return None;
            }
        }
        Some(order)
    }

    fn dfs(&self, node: &str, visiting: &mut HashMap<String, u8>, order: &mut Vec<String>) -> bool {
        match visiting.get(node).copied().unwrap_or(0) {
            1 => return false, // cycle
            2 => return true,
            _ => {}
        }
        visiting.insert(node.to_string(), 1);
        for dep in self.deps_of(node) {
            if !self.dfs(dep, visiting, order) {
                return false;
            }
        }
        visiting.insert(node.to_string(), 2);
        if !order.iter().any(|x| x == node) {
            order.push(node.to_string());
        }
        true
    }

    /// Number of nodes with explicit edges.
    pub fn len(&self) -> usize {
        self.edges.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }
}

#[cfg(test)]
mod dep_tests {
    use super::*;

    #[test]
    fn topo_order_deps_first() {
        let mut g = DependencyGraph::new();
        g.add_dep("scene", "hero");
        g.add_dep("hero", "atlas");
        let order = g.load_order(&["scene".into()]).expect("no cycle");
        let atlas = order.iter().position(|x| x == "atlas").unwrap();
        let hero = order.iter().position(|x| x == "hero").unwrap();
        let scene = order.iter().position(|x| x == "scene").unwrap();
        assert!(atlas < hero && hero < scene);
    }

    #[test]
    fn cycle_returns_none() {
        let mut g = DependencyGraph::new();
        g.add_dep("a", "b");
        g.add_dep("b", "a");
        assert!(g.load_order(&["a".into()]).is_none());
    }
}
