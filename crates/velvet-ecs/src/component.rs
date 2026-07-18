//! Component storage.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use slotmap::SecondaryMap;

use crate::entity::Entity;

/// Marker trait for components.
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

/// Type-erased component type id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(TypeId);

impl ComponentId {
    /// Id for `T`.
    pub fn of<T: Component>() -> Self {
        Self(TypeId::of::<T>())
    }
}

trait AnyStorage: Send + Sync {
    fn remove(&mut self, entity: Entity);
    fn contains(&self, entity: Entity) -> bool;
    fn clear(&mut self);
    fn len(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

struct TypedStorage<T: Component> {
    map: SecondaryMap<Entity, T>,
}

impl<T: Component> AnyStorage for TypedStorage<T> {
    fn remove(&mut self, entity: Entity) {
        self.map.remove(entity);
    }
    fn contains(&self, entity: Entity) -> bool {
        self.map.contains_key(entity)
    }
    fn clear(&mut self) {
        self.map.clear();
    }
    fn len(&self) -> usize {
        self.map.len()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Heterogeneous component stores keyed by type.
#[derive(Default)]
pub struct ComponentStorage {
    stores: HashMap<TypeId, Box<dyn AnyStorage>>,
}

impl ComponentStorage {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure<T: Component>(&mut self) -> &mut TypedStorage<T> {
        let id = TypeId::of::<T>();
        self.stores.entry(id).or_insert_with(|| {
            Box::new(TypedStorage::<T> {
                map: SecondaryMap::new(),
            })
        });
        self.stores
            .get_mut(&id)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<TypedStorage<T>>()
            .expect("storage type")
    }

    fn get_store<T: Component>(&self) -> Option<&TypedStorage<T>> {
        self.stores
            .get(&TypeId::of::<T>())
            .and_then(|s| s.as_any().downcast_ref::<TypedStorage<T>>())
    }

    fn get_store_mut<T: Component>(&mut self) -> Option<&mut TypedStorage<T>> {
        self.stores
            .get_mut(&TypeId::of::<T>())
            .and_then(|s| s.as_any_mut().downcast_mut::<TypedStorage<T>>())
    }

    /// Insert component, returning previous.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) -> Option<T> {
        self.ensure::<T>().map.insert(entity, value)
    }

    /// Remove component.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        self.get_store_mut::<T>()?.map.remove(entity)
    }

    /// Get ref.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.get_store::<T>()?.map.get(entity)
    }

    /// Get mut.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.get_store_mut::<T>()?.map.get_mut(entity)
    }

    /// Whether entity has component.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.get_store::<T>()
            .map(|s| s.contains(entity))
            .unwrap_or(false)
    }

    /// Remove all components for entity.
    pub fn remove_entity(&mut self, entity: Entity) {
        for store in self.stores.values_mut() {
            store.remove(entity);
        }
    }

    /// Drop every component store (entities themselves are not despawned here).
    pub fn clear_all_stores(&mut self) {
        for store in self.stores.values_mut() {
            store.clear();
        }
        self.stores.clear();
    }

    /// Total component instances across all types.
    pub fn total_instances(&self) -> usize {
        self.stores.values().map(|s| s.len()).sum()
    }

    /// Iterate entities that have `T`.
    pub fn iter<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> + '_ {
        self.get_store::<T>().into_iter().flat_map(|s| s.map.iter())
    }

    /// Mutable iterate `T` — exclusive.
    pub fn iter_mut<T: Component>(&mut self) -> impl Iterator<Item = (Entity, &mut T)> + '_ {
        self.ensure::<T>().map.iter_mut()
    }

    /// Count of component types registered.
    pub fn type_count(&self) -> usize {
        self.stores.len()
    }
}
