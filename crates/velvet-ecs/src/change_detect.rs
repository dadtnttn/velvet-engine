//! Per-component change detection ticks.

use std::any::TypeId;
use std::collections::HashMap;

use slotmap::SecondaryMap;

use crate::component::Component;
use crate::entity::Entity;

/// Monotonic world tick used for change detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct Tick(pub u64);

impl Tick {
    /// Advance by one.
    pub fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    /// Whether `change` happened after `last_seen` (strictly greater).
    pub fn is_newer(self, last_seen: Tick) -> bool {
        self.0 > last_seen.0
    }
}

/// Tracks last-changed tick per entity for a component type.
#[derive(Debug, Default)]
pub struct ComponentTicks {
    map: SecondaryMap<Entity, Tick>,
    /// Tick of last insert/remove affecting this store.
    pub store_tick: Tick,
}

impl ComponentTicks {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark entity changed at `tick`.
    pub fn mark(&mut self, entity: Entity, tick: Tick) {
        self.map.insert(entity, tick);
        self.store_tick = tick;
    }

    /// Remove tracking for entity.
    pub fn remove(&mut self, entity: Entity) {
        self.map.remove(entity);
    }

    /// Last change tick for entity.
    pub fn get(&self, entity: Entity) -> Option<Tick> {
        self.map.get(entity).copied()
    }

    /// Whether entity changed after `since`.
    pub fn is_changed(&self, entity: Entity, since: Tick) -> bool {
        self.get(entity).map(|t| t.is_newer(since)).unwrap_or(false)
    }

    /// Entities changed after `since`.
    pub fn changed_since(&self, since: Tick) -> Vec<Entity> {
        self.map
            .iter()
            .filter_map(|(e, t)| if t.is_newer(since) { Some(e) } else { None })
            .collect()
    }

    /// Clear all.
    pub fn clear(&mut self) {
        self.map.clear();
        self.store_tick = Tick(0);
    }
}

/// Registry of change ticks for all component types.
#[derive(Debug, Default)]
pub struct ChangeDetection {
    /// Current world tick.
    pub tick: Tick,
    by_type: HashMap<TypeId, ComponentTicks>,
}

impl ChangeDetection {
    /// Create empty (starts at tick 1 so initial inserts are detectable vs tick 0).
    pub fn new() -> Self {
        Self {
            tick: Tick(1),
            by_type: HashMap::new(),
        }
    }

    /// Advance world tick (call once per frame / system stage).
    pub fn advance(&mut self) {
        self.tick.advance();
    }

    /// Current tick.
    pub fn current(&self) -> Tick {
        self.tick
    }

    fn ticks_mut<T: Component>(&mut self) -> &mut ComponentTicks {
        self.by_type.entry(TypeId::of::<T>()).or_default()
    }

    fn ticks<T: Component>(&self) -> Option<&ComponentTicks> {
        self.by_type.get(&TypeId::of::<T>())
    }

    /// Mark component `T` on entity as changed at current tick.
    pub fn mark_changed<T: Component>(&mut self, entity: Entity) {
        let tick = self.tick;
        self.ticks_mut::<T>().mark(entity, tick);
    }

    /// Mark with explicit tick.
    pub fn mark_changed_at<T: Component>(&mut self, entity: Entity, tick: Tick) {
        self.ticks_mut::<T>().mark(entity, tick);
    }

    /// Clear tracking when component removed.
    pub fn on_remove<T: Component>(&mut self, entity: Entity) {
        if let Some(t) = self.by_type.get_mut(&TypeId::of::<T>()) {
            t.remove(entity);
        }
    }

    /// Whether `T` changed on entity after `since`.
    pub fn is_changed<T: Component>(&self, entity: Entity, since: Tick) -> bool {
        self.ticks::<T>()
            .map(|t| t.is_changed(entity, since))
            .unwrap_or(false)
    }

    /// Whether `T` was added (changed) after `since` — same as is_changed without prior value.
    pub fn is_added<T: Component>(&self, entity: Entity, since: Tick) -> bool {
        self.is_changed::<T>(entity, since)
    }

    /// Entities with `T` changed after `since`.
    pub fn changed_entities<T: Component>(&self, since: Tick) -> Vec<Entity> {
        self.ticks::<T>()
            .map(|t| t.changed_since(since))
            .unwrap_or_default()
    }

    /// Clear all tracking.
    pub fn clear(&mut self) {
        self.by_type.clear();
        self.tick = Tick(0);
    }

    /// Number of component types tracked.
    pub fn type_count(&self) -> usize {
        self.by_type.len()
    }
}

/// System-local change filter: remembers last seen tick.
#[derive(Debug, Clone, Copy, Default)]
pub struct ChangedFilter {
    /// Last tick this filter observed.
    pub last: Tick,
}

impl ChangedFilter {
    /// Create at tick 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Entities of `T` changed since last update; advances filter to current.
    pub fn update<T: Component>(&mut self, detection: &ChangeDetection) -> Vec<Entity> {
        let out = detection.changed_entities::<T>(self.last);
        self.last = detection.current();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[derive(Clone, Debug)]
    struct Health(i32);

    #[test]
    fn marks_and_filters() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Health(10));
        assert_eq!(world.get::<Health>(e).map(|h| h.0), Some(10));
        world.change_detection_mut().mark_changed::<Health>(e);

        let since = Tick(0);
        assert!(world.change_detection().is_changed::<Health>(e, since));
        world.change_detection_mut().advance();
        let now = world.change_detection().current();
        // After advance without re-mark, still newer than 0
        assert!(world.change_detection().is_changed::<Health>(e, since));
        // Not newer than current tick (mark was at previous)
        let mark_tick = Tick(now.0.saturating_sub(1));
        assert!(!world.change_detection().is_changed::<Health>(e, mark_tick));
    }

    #[test]
    fn changed_filter_once() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Health(1));
        world.change_detection_mut().mark_changed::<Health>(e);
        let mut filter = ChangedFilter::new();
        let first = filter.update::<Health>(world.change_detection());
        assert_eq!(first, vec![e]);
        let second = filter.update::<Health>(world.change_detection());
        assert!(second.is_empty());
    }
}
