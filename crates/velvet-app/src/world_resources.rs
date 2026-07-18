//! Type-map resources stored on `App`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Marker trait for app-global resources.
pub trait Resource: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Resource for T {}

/// Opaque resource type id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(TypeId);

impl ResourceId {
    /// Id for type `T`.
    pub fn of<T: Resource>() -> Self {
        Self(TypeId::of::<T>())
    }
}

/// Storage for typed resources.
#[derive(Default)]
pub struct Resources {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    /// Create empty storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or replace a resource.
    pub fn insert<T: Resource>(&mut self, value: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|b| b.downcast::<T>().ok().map(|b| *b))
    }

    /// Remove a resource.
    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|b| b.downcast::<T>().ok().map(|b| *b))
    }

    /// Whether resource exists.
    pub fn contains<T: Resource>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    /// Immutable borrow.
    pub fn get<T: Resource>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// Mutable borrow.
    pub fn get_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<T>())
    }

    /// Get or insert default.
    pub fn get_or_insert_with<T: Resource, F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if !self.contains::<T>() {
            self.insert(f());
        }
        self.get_mut::<T>().expect("just inserted")
    }

    /// Number of resources.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Phantom helper for documenting resource access patterns in APIs.
pub struct Res<'a, T: Resource> {
    _marker: PhantomData<&'a T>,
}

/// Mutable resource access marker.
pub struct ResMut<'a, T: Resource> {
    _marker: PhantomData<&'a mut T>,
}
