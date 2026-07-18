//! Typed service / resource locator for engine subsystems.
//!
//! Provides a lightweight registry of named services that plugins can publish
//! and other systems can resolve without hard-wiring crate dependencies.
//! Values are type-erased with `TypeId` and downcast on retrieval.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use thiserror::Error;

/// Errors when resolving services.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// No service registered for the type.
    #[error("service not found for type {0}")]
    NotFound(String),
    /// Downcast failed (type mismatch under same TypeId should not happen).
    #[error("service type mismatch for {0}")]
    TypeMismatch(String),
    /// Service already registered and replace was not requested.
    #[error("service already registered for {0}")]
    AlreadyExists(String),
    /// Lock poisoned.
    #[error("service registry lock poisoned")]
    Poisoned,
}

/// Metadata about a registered service.
#[derive(Debug, Clone)]
pub struct ServiceMeta {
    /// Stable name (defaults to type name).
    pub name: String,
    /// Type id string for diagnostics.
    pub type_name: String,
    /// Plugin that registered it, if known.
    pub provider: Option<String>,
}

struct ServiceSlot {
    meta: ServiceMeta,
    value: Arc<dyn Any + Send + Sync>,
}

/// Thread-safe service registry.
#[derive(Default)]
pub struct ServiceRegistry {
    by_type: HashMap<TypeId, ServiceSlot>,
    by_name: HashMap<String, TypeId>,
}

impl fmt::Debug for ServiceRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceRegistry")
            .field("count", &self.by_type.len())
            .field("names", &self.by_name.keys().cloned().collect::<Vec<_>>())
            .finish()
    }
}

impl ServiceRegistry {
    /// Create empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered services.
    pub fn len(&self) -> usize {
        self.by_type.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.by_type.is_empty()
    }

    /// Register a service. Fails if already present unless `replace` is true.
    pub fn insert<T: Any + Send + Sync + 'static>(
        &mut self,
        value: T,
        name: impl Into<String>,
        provider: Option<String>,
        replace: bool,
    ) -> Result<(), ServiceError> {
        let tid = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>().to_string();
        let name = name.into();
        if self.by_type.contains_key(&tid) && !replace {
            return Err(ServiceError::AlreadyExists(type_name));
        }
        if let Some(old_tid) = self.by_name.get(&name) {
            if *old_tid != tid && !replace {
                return Err(ServiceError::AlreadyExists(name));
            }
            if *old_tid != tid {
                self.by_type.remove(old_tid);
            }
        }
        let meta = ServiceMeta {
            name: name.clone(),
            type_name,
            provider,
        };
        self.by_type.insert(
            tid,
            ServiceSlot {
                meta,
                value: Arc::new(value),
            },
        );
        self.by_name.insert(name, tid);
        Ok(())
    }

    /// Convenience: insert replacing any prior instance.
    pub fn provide<T: Any + Send + Sync + 'static>(
        &mut self,
        value: T,
        name: impl Into<String>,
    ) -> Result<(), ServiceError> {
        self.insert(value, name, None, true)
    }

    /// Get shared handle to a service.
    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Result<Arc<T>, ServiceError> {
        let tid = TypeId::of::<T>();
        let slot = self
            .by_type
            .get(&tid)
            .ok_or_else(|| ServiceError::NotFound(std::any::type_name::<T>().into()))?;
        slot.value
            .clone()
            .downcast::<T>()
            .map_err(|_| ServiceError::TypeMismatch(std::any::type_name::<T>().into()))
    }

    /// Get by registered name, then downcast.
    pub fn get_named<T: Any + Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Result<Arc<T>, ServiceError> {
        let tid = *self
            .by_name
            .get(name)
            .ok_or_else(|| ServiceError::NotFound(name.into()))?;
        let slot = self
            .by_type
            .get(&tid)
            .ok_or_else(|| ServiceError::NotFound(name.into()))?;
        slot.value
            .clone()
            .downcast::<T>()
            .map_err(|_| ServiceError::TypeMismatch(std::any::type_name::<T>().into()))
    }

    /// Metadata for a type, if registered.
    pub fn meta<T: Any + Send + Sync + 'static>(&self) -> Option<&ServiceMeta> {
        self.by_type.get(&TypeId::of::<T>()).map(|s| &s.meta)
    }

    /// All service names.
    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<_> = self.by_name.keys().cloned().collect();
        v.sort();
        v
    }

    /// Remove by type.
    pub fn remove<T: Any + Send + Sync + 'static>(&mut self) -> bool {
        let tid = TypeId::of::<T>();
        if let Some(slot) = self.by_type.remove(&tid) {
            self.by_name.remove(&slot.meta.name);
            true
        } else {
            false
        }
    }

    /// Clear all services.
    pub fn clear(&mut self) {
        self.by_type.clear();
        self.by_name.clear();
    }
}

/// Shared service hub wrapped in `RwLock` for multi-threaded systems.
#[derive(Clone, Default)]
pub struct ServiceHub {
    inner: Arc<RwLock<ServiceRegistry>>,
}

impl ServiceHub {
    /// Create empty hub.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register / replace a service.
    pub fn provide<T: Any + Send + Sync + 'static>(
        &self,
        value: T,
        name: impl Into<String>,
    ) -> Result<(), ServiceError> {
        let mut g = self.inner.write().map_err(|_| ServiceError::Poisoned)?;
        g.provide(value, name)
    }

    /// Resolve service.
    pub fn get<T: Any + Send + Sync + 'static>(&self) -> Result<Arc<T>, ServiceError> {
        let g = self.inner.read().map_err(|_| ServiceError::Poisoned)?;
        g.get::<T>()
    }

    /// Resolve by name.
    pub fn get_named<T: Any + Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Result<Arc<T>, ServiceError> {
        let g = self.inner.read().map_err(|_| ServiceError::Poisoned)?;
        g.get_named::<T>(name)
    }

    /// Snapshot names.
    pub fn names(&self) -> Result<Vec<String>, ServiceError> {
        let g = self.inner.read().map_err(|_| ServiceError::Poisoned)?;
        Ok(g.names())
    }

    /// Count.
    pub fn len(&self) -> Result<usize, ServiceError> {
        let g = self.inner.read().map_err(|_| ServiceError::Poisoned)?;
        Ok(g.len())
    }

    /// Whether empty.
    pub fn is_empty(&self) -> Result<bool, ServiceError> {
        Ok(self.len()? == 0)
    }
}

/// Build settings snapshot shared as a service.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BuildProfileService {
    /// Profile name: debug / release / custom.
    pub profile: String,
    /// Whether assets are optimized.
    pub optimize_assets: bool,
    /// Strip symbols flag.
    pub strip_debug: bool,
}

impl Default for BuildProfileService {
    fn default() -> Self {
        Self {
            profile: "debug".into(),
            optimize_assets: false,
            strip_debug: false,
        }
    }
}

/// Locale preference service.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LocaleService {
    /// Active locale BCP-47-ish tag.
    pub locale: String,
    /// Fallback chain.
    pub fallbacks: Vec<String>,
}

impl Default for LocaleService {
    fn default() -> Self {
        Self {
            locale: "en".into(),
            fallbacks: vec!["en".into()],
        }
    }
}

impl LocaleService {
    /// Create with primary locale.
    pub fn new(locale: impl Into<String>) -> Self {
        let locale = locale.into();
        Self {
            fallbacks: vec![locale.clone()],
            locale,
        }
    }

    /// Resolve preferred locale list (primary first).
    pub fn chain(&self) -> Vec<String> {
        let mut out = vec![self.locale.clone()];
        for f in &self.fallbacks {
            if !out.contains(f) {
                out.push(f.clone());
            }
        }
        out
    }
}

/// Feature flag map service.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FeatureFlags {
    flags: HashMap<String, bool>,
}

impl FeatureFlags {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set flag.
    pub fn set(&mut self, name: impl Into<String>, enabled: bool) {
        self.flags.insert(name.into(), enabled);
    }

    /// Query flag (default false).
    pub fn enabled(&self, name: &str) -> bool {
        self.flags.get(name).copied().unwrap_or(false)
    }

    /// Query with default.
    pub fn enabled_or(&self, name: &str, default: bool) -> bool {
        self.flags.get(name).copied().unwrap_or(default)
    }

    /// All flags sorted.
    pub fn all(&self) -> Vec<(String, bool)> {
        let mut v: Vec<_> = self.flags.iter().map(|(k, v)| (k.clone(), *v)).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }
}

/// Platform capability bits reported at startup.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlatformCaps {
    /// OS family: windows / linux / macos / other.
    pub os: String,
    /// CPU arch.
    pub arch: String,
    /// Whether a windowing backend is compiled in.
    pub has_window: bool,
    /// Whether GPU backend is available (best-effort).
    pub has_gpu: bool,
    /// Whether audio device is available (best-effort).
    pub has_audio: bool,
}

impl PlatformCaps {
    /// Detect from compile-time / runtime env (no GPU probe).
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.into(),
            arch: std::env::consts::ARCH.into(),
            // Core has no window feature; binaries that enable windows report true via override.
            has_window: false,
            has_gpu: true,
            has_audio: true,
        }
    }
}

impl Default for PlatformCaps {
    fn default() -> Self {
        Self::detect()
    }
}

/// Paths service for project roots.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PathService {
    /// Project root.
    pub project_root: String,
    /// Assets directory (absolute or relative).
    pub assets: String,
    /// User data directory.
    pub user_data: String,
    /// Cache directory.
    pub cache: String,
}

impl PathService {
    /// Create with project root; derives common subdirs.
    pub fn from_project_root(root: impl Into<String>) -> Self {
        let project_root = root.into();
        Self {
            assets: format!("{project_root}/assets"),
            user_data: format!("{project_root}/user"),
            cache: format!("{project_root}/.velvet-cache"),
            project_root,
        }
    }

    /// Join under project root.
    pub fn project_join(&self, rel: &str) -> String {
        format!(
            "{}/{}",
            self.project_root.trim_end_matches(['/', '\\']),
            rel.trim_start_matches(['/', '\\'])
        )
    }
}

/// Seedable runtime id generator service.
#[derive(Debug, Clone)]
pub struct IdGenerator {
    next: u64,
    prefix: String,
}

impl IdGenerator {
    /// Create with prefix.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            next: 1,
            prefix: prefix.into(),
        }
    }

    /// Allocate next id string `prefix-N`.
    pub fn next_id(&mut self) -> String {
        let id = format!("{}-{}", self.prefix, self.next);
        self.next = self.next.saturating_add(1);
        id
    }

    /// Peek without allocating.
    pub fn peek(&self) -> u64 {
        self.next
    }

    /// Reset counter.
    pub fn reset(&mut self, next: u64) {
        self.next = next.max(1);
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new("id")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_get() {
        let mut reg = ServiceRegistry::new();
        reg.provide(42u32, "answer").unwrap();
        let v = reg.get::<u32>().unwrap();
        assert_eq!(*v, 42);
        assert_eq!(reg.names(), vec!["answer".to_string()]);
    }

    #[test]
    fn reject_duplicate_without_replace() {
        let mut reg = ServiceRegistry::new();
        reg.insert(1u8, "n", None, false).unwrap();
        let err = reg.insert(2u8, "n2", None, false).unwrap_err();
        assert!(matches!(err, ServiceError::AlreadyExists(_)));
    }

    #[test]
    fn hub_locale_and_flags() {
        let hub = ServiceHub::new();
        hub.provide(LocaleService::new("es"), "locale").unwrap();
        hub.provide(
            {
                let mut f = FeatureFlags::new();
                f.set("rollback", true);
                f
            },
            "flags",
        )
        .unwrap();
        let loc = hub.get::<LocaleService>().unwrap();
        assert_eq!(loc.locale, "es");
        let flags = hub.get::<FeatureFlags>().unwrap();
        assert!(flags.enabled("rollback"));
        assert!(!flags.enabled("missing"));
    }

    #[test]
    fn path_service_join() {
        let p = PathService::from_project_root("C:/game");
        assert!(p.project_join("assets/x.png").contains("assets"));
    }

    #[test]
    fn id_generator_monotonic() {
        let mut g = IdGenerator::new("ent");
        let a = g.next_id();
        let b = g.next_id();
        assert_ne!(a, b);
        assert!(a.starts_with("ent-"));
    }

    #[test]
    fn locale_chain_dedup() {
        let mut l = LocaleService::new("fr");
        l.fallbacks = vec!["fr".into(), "en".into()];
        assert_eq!(l.chain(), vec!["fr".to_string(), "en".to_string()]);
    }

    #[test]
    fn platform_detect_nonempty() {
        let c = PlatformCaps::detect();
        assert!(!c.os.is_empty());
        assert!(!c.arch.is_empty());
    }

    #[test]
    fn remove_and_clear() {
        let mut reg = ServiceRegistry::new();
        reg.provide(BuildProfileService::default(), "build")
            .unwrap();
        assert!(reg.remove::<BuildProfileService>());
        assert!(reg.is_empty());
        reg.provide(1u16, "x").unwrap();
        reg.clear();
        assert!(reg.is_empty());
    }

    #[test]
    fn named_resolve() {
        let mut reg = ServiceRegistry::new();
        reg.provide(String::from("hi"), "greet").unwrap();
        let s = reg.get_named::<String>("greet").unwrap();
        assert_eq!(&*s, "hi");
        assert!(reg.get_named::<String>("nope").is_err());
    }

    #[test]
    fn feature_all_sorted() {
        let mut f = FeatureFlags::new();
        f.set("z", true);
        f.set("a", false);
        let all = f.all();
        assert_eq!(all[0].0, "a");
        assert_eq!(all[1].0, "z");
    }
}
