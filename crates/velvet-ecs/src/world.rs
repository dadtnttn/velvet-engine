//! World: entities + components + resources + commands.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use slotmap::SlotMap;
use tracing::trace;

use crate::change_detect::ChangeDetection;
use crate::commands::CommandQueue;
use crate::component::{Component, ComponentStorage};
use crate::entity::{Entity, EntityMeta};
use crate::events::EventQueue;
use crate::registry::ComponentRegistry;

/// ECS world.
#[derive(Default)]
pub struct World {
    entities: SlotMap<Entity, EntityMeta>,
    components: ComponentStorage,
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    commands: CommandQueue,
    change_detection: ChangeDetection,
    events: EventQueue,
    registry: ComponentRegistry,
}

impl World {
    /// Create empty world.
    pub fn new() -> Self {
        // `ChangeDetection::new()` starts at tick 1 so first-frame inserts
        // are detectable against a reader that last saw `Tick(0)`.
        Self {
            change_detection: ChangeDetection::new(),
            ..Self::default()
        }
    }

    /// Spawn empty entity.
    pub fn spawn(&mut self) -> Entity {
        self.spawn_with_meta(EntityMeta::new())
    }

    /// Spawn named entity.
    pub fn spawn_named(&mut self, name: impl Into<String>) -> Entity {
        self.spawn_with_meta(EntityMeta::named(name))
    }

    /// Spawn with meta.
    pub fn spawn_with_meta(&mut self, meta: EntityMeta) -> Entity {
        let e = self.entities.insert(meta);
        trace!(?e, "spawn");
        e
    }

    /// Spawn and insert one component.
    pub fn spawn_with<T: Component>(&mut self, component: T) -> Entity {
        let e = self.spawn();
        self.insert(e, component);
        e
    }

    /// Despawn entity and components.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if self.entities.remove(entity).is_some() {
            self.components.remove_entity(entity);
            trace!(?entity, "despawn");
            true
        } else {
            false
        }
    }

    /// Whether entity exists.
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains_key(entity)
    }

    /// Entity meta.
    pub fn meta(&self, entity: Entity) -> Option<&EntityMeta> {
        self.entities.get(entity)
    }

    /// Mutable meta.
    pub fn meta_mut(&mut self, entity: Entity) -> Option<&mut EntityMeta> {
        self.entities.get_mut(entity)
    }

    /// Insert component.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) -> Option<T> {
        if !self.contains(entity) {
            return None;
        }
        let prev = self.components.insert(entity, value);
        self.change_detection.mark_changed::<T>(entity);
        prev
    }

    /// Remove component.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let prev = self.components.remove::<T>(entity);
        if prev.is_some() {
            self.change_detection.on_remove::<T>(entity);
        }
        prev
    }

    /// Get component.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.components.get::<T>(entity)
    }

    /// Get mut component (marks changed).
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        if self.components.has::<T>(entity) {
            self.change_detection.mark_changed::<T>(entity);
        }
        self.components.get_mut::<T>(entity)
    }

    /// Get mut without marking change detection.
    pub fn get_mut_silent<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut::<T>(entity)
    }

    /// Has component.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.components.has::<T>(entity)
    }

    /// Component storage.
    pub fn components(&self) -> &ComponentStorage {
        &self.components
    }

    /// Mutable component storage.
    pub fn components_mut(&mut self) -> &mut ComponentStorage {
        &mut self.components
    }

    /// Entity count.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Iterate all entities.
    pub fn iter_entities(&self) -> impl Iterator<Item = (Entity, &EntityMeta)> + '_ {
        self.entities.iter()
    }

    /// Insert resource.
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        self.resources
            .insert(TypeId::of::<T>(), Box::new(value))
            .and_then(|b| b.downcast::<T>().ok().map(|b| *b))
    }

    /// Get resource.
    pub fn resource<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref::<T>())
    }

    /// Get mut resource.
    pub fn resource_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut::<T>())
    }

    /// Command queue.
    pub fn commands(&mut self) -> &mut CommandQueue {
        &mut self.commands
    }

    /// Apply deferred commands.
    pub fn apply_commands(&mut self) {
        let mut q = std::mem::take(&mut self.commands);
        q.apply(self);
        self.commands = q;
    }

    /// Change detection registry.
    pub fn change_detection(&self) -> &ChangeDetection {
        &self.change_detection
    }

    /// Mutable change detection.
    pub fn change_detection_mut(&mut self) -> &mut ChangeDetection {
        &mut self.change_detection
    }

    /// Advance change-detection tick (call once per frame).
    pub fn advance_tick(&mut self) {
        self.change_detection.advance();
    }

    /// Event queue.
    pub fn events(&self) -> &EventQueue {
        &self.events
    }

    /// Mutable event queue.
    pub fn events_mut(&mut self) -> &mut EventQueue {
        &mut self.events
    }

    /// Send a typed event.
    pub fn send_event<E: Send + Sync + 'static>(&mut self, event: E) {
        self.events.send(event);
    }

    /// Component type registry metadata.
    pub fn registry(&self) -> &ComponentRegistry {
        &self.registry
    }

    /// Mutable component registry.
    pub fn registry_mut(&mut self) -> &mut ComponentRegistry {
        &mut self.registry
    }

    /// Clear world.
    pub fn clear(&mut self) {
        self.entities.clear();
        self.components = ComponentStorage::new();
        self.commands = CommandQueue::new();
        self.change_detection.clear();
        self.events = EventQueue::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_math::Vec2;

    #[derive(Debug, Clone, PartialEq)]
    struct Position(Vec2);
    #[derive(Debug, Clone, PartialEq)]
    struct Velocity(Vec2);
    #[derive(Debug, Clone, PartialEq)]
    struct Health {
        current: f32,
        max: f32,
    }

    #[test]
    fn spawn_insert_query() {
        let mut world = World::new();
        let e = world.spawn_named("player");
        world.insert(e, Position(Vec2::new(1.0, 2.0)));
        world.insert(
            e,
            Health {
                current: 100.0,
                max: 100.0,
            },
        );
        assert!(world.has::<Position>(e));
        assert_eq!(world.get::<Position>(e).unwrap().0.x, 1.0);
        assert_eq!(world.entity_count(), 1);

        let mut count = 0;
        for (entity, pos) in world.components().iter::<Position>() {
            assert_eq!(entity, e);
            assert_eq!(pos.0.y, 2.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn despawn_removes_components() {
        let mut world = World::new();
        let e = world.spawn_with(Position(Vec2::ZERO));
        assert!(world.despawn(e));
        assert!(!world.contains(e));
        assert!(world.get::<Position>(e).is_none());
    }

    #[test]
    fn deferred_commands() {
        let mut world = World::new();
        let e = world.spawn();
        world.commands().insert(e, Velocity(Vec2::new(3.0, 0.0)));
        assert!(!world.has::<Velocity>(e));
        world.apply_commands();
        assert_eq!(world.get::<Velocity>(e).unwrap().0.x, 3.0);
    }

    #[test]
    fn resources() {
        let mut world = World::new();
        world.insert_resource(42u32);
        assert_eq!(*world.resource::<u32>().unwrap(), 42);
    }

    #[test]
    fn query2_join() {
        let mut world = World::new();
        let a = world.spawn();
        world.insert(a, Position(Vec2::X));
        world.insert(a, Velocity(Vec2::Y));
        let b = world.spawn();
        world.insert(b, Position(Vec2::Y));
        let pairs = crate::query::query2::<Position, Velocity>(&world);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, a);
    }
}
