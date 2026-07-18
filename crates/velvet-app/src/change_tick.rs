//! Resource change ticks — detect when resources were last mutated.

use std::any::TypeId;
use std::collections::HashMap;

use crate::world_resources::Resource;

/// Monotonic change tick counter for the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Tick(pub u64);

impl Tick {
    /// Zero tick.
    pub const ZERO: Self = Self(0);

    /// Advance and return the new tick.
    pub fn bump(&mut self) -> Tick {
        self.0 = self.0.saturating_add(1);
        *self
    }

    /// Whether `self` is strictly newer than `other`.
    pub fn is_newer_than(self, other: Tick) -> bool {
        self.0 > other.0
    }
}

/// Tracks last-changed ticks per resource type.
#[derive(Debug, Clone, Default)]
pub struct ChangeTicks {
    /// Global world tick (incremented each frame or each mutation).
    world_tick: Tick,
    /// TypeId → last changed tick.
    last_changed: HashMap<TypeId, Tick>,
    /// TypeId → last added tick (first insert).
    added: HashMap<TypeId, Tick>,
}

impl ChangeTicks {
    /// Create.
    pub fn new() -> Self {
        Self::default()
    }

    /// Current world tick.
    pub fn world_tick(&self) -> Tick {
        self.world_tick
    }

    /// Advance world tick (call once per frame).
    pub fn advance_world(&mut self) -> Tick {
        self.world_tick.bump()
    }

    /// Mark resource type `T` as changed at the current world tick.
    pub fn mark_changed<T: Resource>(&mut self) {
        self.mark_changed_id(TypeId::of::<T>());
    }

    /// Mark by type id.
    pub fn mark_changed_id(&mut self, id: TypeId) {
        self.last_changed.insert(id, self.world_tick);
        self.added.entry(id).or_insert(self.world_tick);
    }

    /// Mark as added (also changed).
    pub fn mark_added<T: Resource>(&mut self) {
        let id = TypeId::of::<T>();
        self.added.insert(id, self.world_tick);
        self.last_changed.insert(id, self.world_tick);
    }

    /// Last changed tick for `T`, if ever.
    pub fn last_changed<T: Resource>(&self) -> Option<Tick> {
        self.last_changed.get(&TypeId::of::<T>()).copied()
    }

    /// Whether `T` changed since `since` (exclusive).
    pub fn is_changed_since<T: Resource>(&self, since: Tick) -> bool {
        self.last_changed::<T>()
            .map(|t| t.is_newer_than(since))
            .unwrap_or(false)
    }

    /// Whether `T` was changed this world tick.
    pub fn is_changed_this_tick<T: Resource>(&self) -> bool {
        self.last_changed::<T>() == Some(self.world_tick)
    }

    /// Whether `T` was added this tick.
    pub fn is_added_this_tick<T: Resource>(&self) -> bool {
        self.added.get(&TypeId::of::<T>()).copied() == Some(self.world_tick)
    }

    /// Whether `T` was added after `since`.
    pub fn is_added_since<T: Resource>(&self, since: Tick) -> bool {
        self.added
            .get(&TypeId::of::<T>())
            .map(|t| t.is_newer_than(since))
            .unwrap_or(false)
    }

    /// Snapshot of last-changed map size.
    pub fn tracked_count(&self) -> usize {
        self.last_changed.len()
    }

    /// Clear tracking (not world tick).
    pub fn clear_tracking(&mut self) {
        self.last_changed.clear();
        self.added.clear();
    }

    /// Read system helper: store last seen tick and check change.
    pub fn check_and_update_seen<T: Resource>(&self, seen: &mut Tick) -> bool {
        let changed = self.is_changed_since::<T>(*seen);
        if let Some(t) = self.last_changed::<T>() {
            *seen = t;
        }
        changed
    }
}

/// Per-system change detection cursor for a resource type.
#[derive(Debug, Clone, Copy)]
pub struct ChangeCursor {
    /// Last observed tick.
    pub last_seen: Tick,
}

impl Default for ChangeCursor {
    fn default() -> Self {
        Self {
            last_seen: Tick::ZERO,
        }
    }
}

impl ChangeCursor {
    /// Create at zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if resource changed since last check; updates cursor.
    pub fn update<T: Resource>(&mut self, ticks: &ChangeTicks) -> bool {
        ticks.check_and_update_seen::<T>(&mut self.last_seen)
    }
}

/// Map of named cursors for systems that track multiple resources.
#[derive(Debug, Clone, Default)]
pub struct ChangeCursorMap {
    map: HashMap<TypeId, Tick>,
}

impl ChangeCursorMap {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check and update for `T`.
    pub fn update<T: Resource>(&mut self, ticks: &ChangeTicks) -> bool {
        let id = TypeId::of::<T>();
        let seen = self.map.entry(id).or_insert(Tick::ZERO);
        ticks.check_and_update_seen::<T>(seen)
    }

    /// Last seen for `T`.
    pub fn last_seen<T: Resource>(&self) -> Tick {
        self.map
            .get(&TypeId::of::<T>())
            .copied()
            .unwrap_or(Tick::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_and_detect() {
        let mut ticks = ChangeTicks::new();
        ticks.advance_world(); // tick 1
        ticks.mark_changed::<u32>();
        assert!(ticks.is_changed_this_tick::<u32>());
        assert!(ticks.is_changed_since::<u32>(Tick::ZERO));
        assert!(!ticks.is_changed_since::<u32>(Tick(1)));

        let mut cursor = ChangeCursor::new();
        assert!(cursor.update::<u32>(&ticks));
        assert!(!cursor.update::<u32>(&ticks));

        ticks.advance_world();
        ticks.mark_changed::<u32>();
        assert!(cursor.update::<u32>(&ticks));
    }

    #[test]
    fn added_tracking() {
        let mut ticks = ChangeTicks::new();
        ticks.advance_world();
        ticks.mark_added::<String>();
        assert!(ticks.is_added_this_tick::<String>());
        ticks.advance_world();
        assert!(!ticks.is_added_this_tick::<String>());
        assert!(ticks.is_added_since::<String>(Tick::ZERO));
    }

    #[test]
    fn cursor_map() {
        let mut ticks = ChangeTicks::new();
        let mut map = ChangeCursorMap::new();
        ticks.advance_world();
        ticks.mark_changed::<i32>();
        assert!(map.update::<i32>(&ticks));
        assert!(!map.update::<i32>(&ticks));
    }
}
