//! Asset handles and load states.

use std::fmt;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

new_key_type! {
    /// Slotmap key for stored assets.
    pub struct AssetId;
}

/// Lifecycle of an asset entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AssetState {
    /// Not loaded.
    #[default]
    Unloaded,
    /// Load in progress.
    Loading,
    /// Available.
    Loaded,
    /// Failed with error stored separately.
    Failed,
    /// Hot-reload in progress.
    Reloading,
}

impl fmt::Display for AssetState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unloaded => write!(f, "unloaded"),
            Self::Loading => write!(f, "loading"),
            Self::Loaded => write!(f, "loaded"),
            Self::Failed => write!(f, "failed"),
            Self::Reloading => write!(f, "reloading"),
        }
    }
}

static GENERATION: AtomicU64 = AtomicU64::new(1);

/// Generation counter for hot-reload invalidation.
pub fn next_generation() -> u64 {
    GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// Typed weak-ish handle (copyable id + generation).
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    /// Slot id.
    pub id: AssetId,
    /// Generation at creation / last reload.
    pub generation: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Handle<T> {
    /// Create handle.
    pub fn new(id: AssetId, generation: u64) -> Self {
        Self {
            id,
            generation,
            _marker: PhantomData,
        }
    }

    /// Placeholder null handle (default key).
    pub fn none() -> Self {
        Self::new(AssetId::default(), 0)
    }

    /// Whether this is the none handle.
    pub fn is_none(self) -> bool {
        self.generation == 0
    }
}

/// Strong handle keeping an asset alive (reference counted in registry).
#[derive(Debug)]
pub struct StrongHandle<T> {
    handle: Handle<T>,
    // Drop hook would decrement refcount; Phase 2 keeps strong = same as handle + flag.
    _keep: (),
}

impl<T> StrongHandle<T> {
    /// Wrap.
    pub fn new(handle: Handle<T>) -> Self {
        Self { handle, _keep: () }
    }

    /// Weak view.
    pub fn handle(&self) -> Handle<T> {
        self.handle
    }
}

impl<T> Clone for StrongHandle<T> {
    fn clone(&self) -> Self {
        Self::new(self.handle)
    }
}
