//! Asset registry and cache.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use indexmap::IndexMap;
use slotmap::SlotMap;
use tracing::{debug, warn};

use crate::async_queue::LoadPriority;
use crate::handle::{next_generation, AssetId, AssetState, Handle};
use crate::loader::{LoadError, LoadRequest, LoaderRegistry};
use crate::path::AssetPath;
use crate::source::{FileSource, MemorySource, Source};
use crate::streaming::{AssetStreamer, StreamPayload, StreamTicket};

/// Events emitted by the asset system.
#[derive(Debug, Clone)]
pub enum AssetEvent {
    /// Loaded successfully.
    Loaded {
        /// Path.
        path: AssetPath,
        /// Id.
        id: AssetId,
    },
    /// Failed.
    Failed {
        /// Path.
        path: AssetPath,
        /// Message.
        message: String,
    },
    /// Reloaded.
    Reloaded {
        /// Path.
        path: AssetPath,
        /// Id.
        id: AssetId,
    },
}

struct Entry {
    #[allow(dead_code)] // kept for diagnostics / unload by path
    path: AssetPath,
    type_id: TypeId,
    state: AssetState,
    generation: u64,
    data: Option<Box<dyn Any + Send + Sync>>,
    error: Option<String>,
    #[allow(dead_code)] // reserved for future handle refcounting
    refcount: u32,
}

/// Central asset manager.
pub struct Assets {
    entries: SlotMap<AssetId, Entry>,
    by_path: IndexMap<String, AssetId>,
    loaders: LoaderRegistry,
    source: Arc<dyn Source>,
    events: Vec<AssetEvent>,
    hot_reload: bool,
    /// Background loads in flight: streamer ticket → entry waiting for it.
    async_pending: HashMap<u64, AssetId>,
}

impl Default for Assets {
    fn default() -> Self {
        Self::memory()
    }
}

impl Assets {
    /// In-memory source (tests).
    pub fn memory() -> Self {
        Self::with_source(Arc::new(MemorySource::new()))
    }

    /// Filesystem root.
    pub fn from_directory(root: impl Into<PathBuf>) -> Self {
        Self::with_source(Arc::new(FileSource::new(root)))
    }

    /// Custom source.
    pub fn with_source(source: Arc<dyn Source>) -> Self {
        Self {
            entries: SlotMap::with_key(),
            by_path: IndexMap::new(),
            loaders: LoaderRegistry::with_defaults(),
            source,
            events: Vec::new(),
            hot_reload: true,
            async_pending: HashMap::new(),
        }
    }

    /// The current source (shared with streamers and tools).
    pub fn source(&self) -> Arc<dyn Source> {
        Arc::clone(&self.source)
    }

    /// Start loading in the background and return the handle at once.
    ///
    /// The entry is [`AssetState::Loading`] (and [`Assets::get`] returns `None`) until a later
    /// [`Assets::pump_async`] picks up the result; reading and decoding happen on the
    /// streamer's workers, never on the calling thread. A path already loaded or loading as
    /// `T` returns its existing handle.
    pub fn load_async<T: 'static + Send + Sync>(
        &mut self,
        streamer: &AssetStreamer,
        path: impl Into<AssetPath>,
        priority: LoadPriority,
    ) -> Handle<T> {
        let path = path.into();
        let key = path.to_string();
        let existing = self.by_path.get(&key).copied();
        if let Some(entry) = existing.and_then(|id| self.entries.get(id)) {
            if entry.type_id == TypeId::of::<T>()
                && matches!(
                    entry.state,
                    AssetState::Loaded | AssetState::Reloading | AssetState::Loading
                )
            {
                return Handle::new(existing.expect("found"), entry.generation);
            }
        }
        let Some(loader) = self.loaders.for_type::<T>() else {
            let message = LoadError::NoLoader(std::any::type_name::<T>().into()).to_string();
            warn!(path = %path, %message, "async load refused");
            self.events.push(AssetEvent::Failed { path, message });
            return Handle::none();
        };
        let generation = next_generation();
        let id = match existing.filter(|id| self.entries.contains_key(*id)) {
            Some(id) => {
                let entry = &mut self.entries[id];
                entry.type_id = TypeId::of::<T>();
                entry.state = AssetState::Loading;
                entry.generation = generation;
                entry.data = None;
                entry.error = None;
                id
            }
            None => {
                let id = self.entries.insert(Entry {
                    path: path.clone(),
                    type_id: TypeId::of::<T>(),
                    state: AssetState::Loading,
                    generation,
                    data: None,
                    error: None,
                    refcount: 1,
                });
                self.by_path.insert(key, id);
                id
            }
        };
        let ticket = streamer.request_decode(self.source(), path, loader, priority);
        self.async_pending.insert(ticket.0, id);
        Handle::new(id, generation)
    }

    /// Move up to `max_items` finished background loads into the registry (emitting
    /// [`AssetEvent::Loaded`] / [`AssetEvent::Failed`]). Never blocks: call it once a frame;
    /// `max_items` bounds the work done on this thread per call.
    pub fn pump_async(&mut self, streamer: &AssetStreamer, max_items: usize) -> usize {
        let pending = &self.async_pending;
        let finished = streamer.take_completions(max_items, |t| pending.contains_key(&t.0));
        let mut applied = 0;
        for done in finished {
            let Some(id) = self.async_pending.remove(&done.ticket.0) else {
                continue;
            };
            let Some(entry) = self.entries.get_mut(id) else {
                continue;
            };
            if entry.state != AssetState::Loading {
                continue; // loaded synchronously meanwhile
            }
            applied += 1;
            match done.result {
                Ok(StreamPayload::Asset(data)) => {
                    entry.data = Some(data);
                    entry.state = AssetState::Loaded;
                    entry.error = None;
                    debug!(path = %done.path, "asset loaded (async)");
                    self.events.push(AssetEvent::Loaded {
                        path: done.path,
                        id,
                    });
                }
                Ok(StreamPayload::Bytes(_)) => {
                    let message = "streamer returned bytes for a typed load".to_string();
                    entry.state = AssetState::Failed;
                    entry.error = Some(message.clone());
                    self.events.push(AssetEvent::Failed {
                        path: done.path,
                        message,
                    });
                }
                Err(e) => {
                    let message = e.to_string();
                    warn!(path = %done.path, error = %message, "async asset load failed");
                    entry.state = AssetState::Failed;
                    entry.error = Some(message.clone());
                    self.events.push(AssetEvent::Failed {
                        path: done.path,
                        message,
                    });
                }
            }
        }
        applied
    }

    /// Cancel a background load. The entry goes back to [`AssetState::Unloaded`]; returns
    /// false when there was nothing in flight for it.
    pub fn cancel_async<T>(&mut self, streamer: &AssetStreamer, handle: Handle<T>) -> bool {
        let Some(ticket) = self
            .async_pending
            .iter()
            .find(|(_, id)| **id == handle.id)
            .map(|(t, _)| *t)
        else {
            return false;
        };
        self.async_pending.remove(&ticket);
        streamer.cancel(StreamTicket(ticket));
        if let Some(entry) = self.entries.get_mut(handle.id) {
            if entry.state == AssetState::Loading {
                entry.state = AssetState::Unloaded;
            }
        }
        true
    }

    /// Background loads not yet picked up by [`Assets::pump_async`].
    pub fn pending_async(&self) -> usize {
        self.async_pending.len()
    }

    /// Error message of a failed entry.
    pub fn error<T>(&self, handle: Handle<T>) -> Option<&str> {
        self.entries.get(handle.id)?.error.as_deref()
    }

    /// Mutable loader registry.
    pub fn loaders_mut(&mut self) -> &mut LoaderRegistry {
        &mut self.loaders
    }

    /// Enable/disable hot reload flag (watcher is external).
    pub fn set_hot_reload(&mut self, enabled: bool) {
        self.hot_reload = enabled;
    }

    /// Access underlying memory source if present (tests).
    pub fn memory_source_mut(&mut self) -> Option<&mut MemorySource> {
        // Not recoverable from Arc without interior mutability; use insert API instead.
        let _ = self;
        None
    }

    /// Replace source.
    pub fn set_source(&mut self, source: Arc<dyn Source>) {
        self.source = source;
    }

    /// Load or get cached typed asset.
    pub fn load<T: 'static + Send + Sync>(&mut self, path: impl Into<AssetPath>) -> Handle<T> {
        let path = path.into();
        let key = path.to_string();
        if let Some(&id) = self.by_path.get(&key) {
            if let Some(entry) = self.entries.get(id) {
                if entry.type_id == TypeId::of::<T>()
                    && matches!(entry.state, AssetState::Loaded | AssetState::Reloading)
                {
                    return Handle::new(id, entry.generation);
                }
            }
        }
        match self.load_inner::<T>(LoadRequest {
            path: path.clone(),
            force: false,
        }) {
            Ok(h) => h,
            Err(e) => {
                warn!(error = %e, path = %path, "asset load failed");
                self.events.push(AssetEvent::Failed {
                    path,
                    message: e.to_string(),
                });
                Handle::none()
            }
        }
    }

    /// Force reload path.
    pub fn reload<T: 'static + Send + Sync>(
        &mut self,
        path: impl Into<AssetPath>,
    ) -> Result<Handle<T>, LoadError> {
        self.load_inner::<T>(LoadRequest {
            path: path.into(),
            force: true,
        })
    }

    /// Get typed asset if loaded and generation matches.
    pub fn get<T: 'static>(&self, handle: Handle<T>) -> Option<&T> {
        let entry = self.entries.get(handle.id)?;
        if entry.generation != handle.generation {
            return None;
        }
        if entry.state != AssetState::Loaded && entry.state != AssetState::Reloading {
            return None;
        }
        entry.data.as_ref()?.downcast_ref::<T>()
    }

    /// State for handle.
    pub fn state<T>(&self, handle: Handle<T>) -> AssetState {
        self.entries
            .get(handle.id)
            .map(|e| e.state)
            .unwrap_or(AssetState::Unloaded)
    }

    /// Drain events.
    pub fn drain_events(&mut self) -> Vec<AssetEvent> {
        std::mem::take(&mut self.events)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert prebuilt asset under path.
    pub fn insert<T: 'static + Send + Sync>(
        &mut self,
        path: impl Into<AssetPath>,
        value: T,
    ) -> Handle<T> {
        let path = path.into();
        let key = path.to_string();
        let generation = next_generation();
        let id = self.entries.insert(Entry {
            path: path.clone(),
            type_id: TypeId::of::<T>(),
            state: AssetState::Loaded,
            generation,
            data: Some(Box::new(value)),
            error: None,
            refcount: 1,
        });
        self.by_path.insert(key, id);
        self.events.push(AssetEvent::Loaded { path, id });
        Handle::new(id, generation)
    }

    fn load_inner<T: 'static + Send + Sync>(
        &mut self,
        req: LoadRequest,
    ) -> Result<Handle<T>, LoadError> {
        let key = req.path.to_string();
        let loader = self
            .loaders
            .for_type::<T>()
            .or_else(|| {
                req.path
                    .virtual_path
                    .extension()
                    .and_then(|e| self.loaders.for_extension(e))
            })
            .ok_or_else(|| LoadError::NoLoader(std::any::type_name::<T>().into()))?;

        if loader.value_type() != TypeId::of::<T>() {
            // Extension loader might differ; still require type match for typed API.
            if self.loaders.for_type::<T>().is_none() {
                return Err(LoadError::TypeMismatch);
            }
        }

        let loader = self
            .loaders
            .for_type::<T>()
            .ok_or_else(|| LoadError::NoLoader(std::any::type_name::<T>().into()))?;

        let bytes = self
            .source
            .read(&req.path)
            .map_err(|e| LoadError::Source(e.to_string()))?;

        let data = loader.load(&req.path, &bytes)?;
        data.downcast_ref::<T>().ok_or(LoadError::TypeMismatch)?;

        let generation = next_generation();
        if let Some(&id) = self.by_path.get(&key) {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.state = if req.force {
                    AssetState::Reloading
                } else {
                    AssetState::Loaded
                };
                entry.generation = generation;
                entry.data = Some(data);
                entry.error = None;
                entry.type_id = TypeId::of::<T>();
                entry.state = AssetState::Loaded;
                if req.force {
                    self.events.push(AssetEvent::Reloaded {
                        path: req.path.clone(),
                        id,
                    });
                } else {
                    self.events.push(AssetEvent::Loaded {
                        path: req.path.clone(),
                        id,
                    });
                }
                debug!(path = %req.path, "asset loaded (update)");
                return Ok(Handle::new(id, generation));
            }
        }

        let id = self.entries.insert(Entry {
            path: req.path.clone(),
            type_id: TypeId::of::<T>(),
            state: AssetState::Loaded,
            generation,
            data: Some(data),
            error: None,
            refcount: 1,
        });
        self.by_path.insert(key, id);
        self.events.push(AssetEvent::Loaded {
            path: req.path.clone(),
            id,
        });
        debug!(path = %req.path, "asset loaded");
        Ok(Handle::new(id, generation))
    }

    /// Map source errors for diagnostics.
    pub fn source_exists(&self, path: &AssetPath) -> bool {
        self.source.exists(path)
    }

    /// Hot-reload path if registered (type-erased via re-read with stored type not available —
    /// callers should call typed reload).
    pub fn notify_file_changed(&mut self, path: &AssetPath) {
        if !self.hot_reload {
            return;
        }
        let key = path.to_string();
        if let Some(&id) = self.by_path.get(&key) {
            if let Some(entry) = self.entries.get_mut(id) {
                entry.state = AssetState::Reloading;
            }
            debug!(path = %path, "marked for reload");
            let _ = id;
        }
    }
}

impl From<&str> for AssetPath {
    fn from(value: &str) -> Self {
        AssetPath::virtual_path(value)
    }
}

impl From<String> for AssetPath {
    fn from(value: String) -> Self {
        AssetPath::virtual_path(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BytesAsset, TextAsset};

    #[test]
    fn load_text_from_memory() {
        let mut mem = MemorySource::new();
        mem.insert("story/intro.vel", b"scene main {}");
        let mut assets = Assets::with_source(Arc::new(mem));
        let h = assets.load::<TextAsset>("story/intro.vel");
        assert!(!h.is_none());
        let text = assets.get(h).unwrap();
        assert!(text.text.contains("scene"));
        assert_eq!(assets.state(h), AssetState::Loaded);
    }

    #[test]
    fn insert_bytes() {
        let mut assets = Assets::memory();
        let h = assets.insert(
            "raw/bin",
            BytesAsset {
                data: vec![1, 2, 3],
            },
        );
        assert_eq!(assets.get(h).unwrap().data, vec![1, 2, 3]);
    }

    #[test]
    fn missing_asset() {
        let mut assets = Assets::memory();
        let h = assets.load::<TextAsset>("nope.txt");
        assert!(h.is_none());
    }
}
