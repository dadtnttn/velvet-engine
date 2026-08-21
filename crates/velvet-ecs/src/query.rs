//! Simple query helpers over the world.

use crate::change_detect::Tick;
use crate::component::Component;
use crate::entity::Entity;
use crate::world::World;

/// Query for entities with component `T`.
pub struct Query<'w, T: Component> {
    world: &'w World,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T: Component> Query<'w, T> {
    /// Create from world.
    pub fn new(world: &'w World) -> Self {
        Self {
            world,
            _marker: std::marker::PhantomData,
        }
    }

    /// Iterate matching entities.
    pub fn iter(&self) -> QueryIter<'_, T> {
        QueryIter {
            inner: self
                .world
                .components()
                .iter::<T>()
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }

    /// Get component for entity.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.world.get::<T>(entity)
    }
}

/// Owning iterator over query results (snapshot of refs not possible; collects pairs of entity + clone if needed).
///
/// This iterator holds owned copies of component references as pointers is unsafe;
/// instead we only expose entity ids here for the snapshot form.
pub struct QueryIter<'a, T: Component> {
    inner: std::vec::IntoIter<(Entity, &'a T)>,
}

impl<'a, T: Component> Iterator for QueryIter<'a, T> {
    type Item = (Entity, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Query entities with both A and B.
pub fn query2<A: Component, B: Component>(world: &World) -> Vec<(Entity, &A, &B)> {
    let mut out = Vec::new();
    for (e, a) in world.components().iter::<A>() {
        if let Some(b) = world.get::<B>(e) {
            out.push((e, a, b));
        }
    }
    out
}

/// Query entities with A, B, and C.
pub fn query3<A: Component, B: Component, C: Component>(
    world: &World,
) -> Vec<(Entity, &A, &B, &C)> {
    let mut out = Vec::new();
    for (e, a) in world.components().iter::<A>() {
        if let Some(b) = world.get::<B>(e) {
            if let Some(c) = world.get::<C>(e) {
                out.push((e, a, b, c));
            }
        }
    }
    out
}

/// Entities that have `T`.
pub fn entities_with<T: Component>(world: &World) -> Vec<Entity> {
    world.components().iter::<T>().map(|(e, _)| e).collect()
}

/// First entity with `T`, if any.
pub fn first_with<T: Component>(world: &World) -> Option<Entity> {
    world.components().iter::<T>().next().map(|(e, _)| e)
}

/// Count entities with `T`.
pub fn count_with<T: Component>(world: &World) -> usize {
    world.components().iter::<T>().count()
}

/// Entities that have `A` but not `B`.
pub fn entities_with_without<A: Component, B: Component>(world: &World) -> Vec<Entity> {
    world
        .components()
        .iter::<A>()
        .filter_map(|(e, _)| {
            if world.get::<B>(e).is_none() {
                Some(e)
            } else {
                None
            }
        })
        .collect()
}

/// Map a query of `T` through `f`, collecting results.
pub fn map_components<T: Component, R>(
    world: &World,
    mut f: impl FnMut(Entity, &T) -> R,
) -> Vec<R> {
    world
        .components()
        .iter::<T>()
        .map(|(e, c)| f(e, c))
        .collect()
}

/// Query entities with A, B, C, and D.
pub fn query4<A: Component, B: Component, C: Component, D: Component>(
    world: &World,
) -> Vec<(Entity, &A, &B, &C, &D)> {
    let mut out = Vec::new();
    for (e, a) in world.components().iter::<A>() {
        if let (Some(b), Some(c), Some(d)) =
            (world.get::<B>(e), world.get::<C>(e), world.get::<D>(e))
        {
            out.push((e, a, b, c, d));
        }
    }
    out
}

/// Entities that have all of A, B.
pub fn entities_with2<A: Component, B: Component>(world: &World) -> Vec<Entity> {
    query2::<A, B>(world)
        .into_iter()
        .map(|(e, _, _)| e)
        .collect()
}

/// Entities that have A and B but not C.
pub fn entities_with2_without<A: Component, B: Component, C: Component>(
    world: &World,
) -> Vec<Entity> {
    query2::<A, B>(world)
        .into_iter()
        .filter_map(|(e, _, _)| {
            if world.get::<C>(e).is_none() {
                Some(e)
            } else {
                None
            }
        })
        .collect()
}

/// For each entity with `T`, mutate via `f` (marks change detection through get_mut).
pub fn for_each_mut<T: Component>(world: &mut World, mut f: impl FnMut(Entity, &mut T)) {
    let entities: Vec<Entity> = entities_with::<T>(world);
    for e in entities {
        if let Some(c) = world.get_mut::<T>(e) {
            f(e, c);
        }
    }
}

/// Find first entity matching predicate on component `T`.
pub fn find_with<T: Component>(
    world: &World,
    mut pred: impl FnMut(Entity, &T) -> bool,
) -> Option<Entity> {
    world
        .components()
        .iter::<T>()
        .find_map(|(e, c)| if pred(e, c) { Some(e) } else { None })
}

/// Collect component clones for all entities with `T`.
pub fn collect_cloned<T: Component + Clone>(world: &World) -> Vec<(Entity, T)> {
    world
        .components()
        .iter::<T>()
        .map(|(e, c)| (e, c.clone()))
        .collect()
}

/// Partition entities with `T` by predicate.
pub fn partition_with<T: Component>(
    world: &World,
    mut pred: impl FnMut(Entity, &T) -> bool,
) -> (Vec<Entity>, Vec<Entity>) {
    let mut yes = Vec::new();
    let mut no = Vec::new();
    for (e, c) in world.components().iter::<T>() {
        if pred(e, c) {
            yes.push(e);
        } else {
            no.push(e);
        }
    }
    (yes, no)
}

/// Any entity with `T` satisfying predicate.
pub fn any_with<T: Component>(world: &World, mut pred: impl FnMut(&T) -> bool) -> bool {
    world.components().iter::<T>().any(|(_, c)| pred(c))
}

/// All entities with `T` satisfy predicate.
pub fn all_with<T: Component>(world: &World, mut pred: impl FnMut(&T) -> bool) -> bool {
    world.components().iter::<T>().all(|(_, c)| pred(c))
}

// ---------------------------------------------------------------------------
// Change-filtered queries
//
// Inserts and `get_mut` mark components changed; `get_mut_silent` does not.
// `since` uses strict comparison against the world tick, so a cursor holding
// `Tick(0)` sees everything inserted so far.
// ---------------------------------------------------------------------------

/// `(entity, component)` pairs whose `T` changed after `since`.
///
/// Inserts count as changes; removals drop tracking entirely.
pub fn iter_changed<T: Component>(world: &World, since: Tick) -> Vec<(Entity, &T)> {
    world
        .components()
        .iter::<T>()
        .filter(|(e, _)| world.change_detection().is_changed::<T>(*e, since))
        .collect()
}

/// Entities with both A and B where **A** changed after `since`.
pub fn query2_changed<A: Component, B: Component>(
    world: &World,
    since: Tick,
) -> Vec<(Entity, &A, &B)> {
    let mut out = Vec::new();
    for (e, a) in iter_changed::<A>(world, since) {
        if let Some(b) = world.get::<B>(e) {
            out.push((e, a, b));
        }
    }
    out
}

/// Map over `T` values that changed after `since`.
pub fn map_changed<T: Component, R>(
    world: &World,
    since: Tick,
    mut f: impl FnMut(Entity, &T) -> R,
) -> Vec<R> {
    iter_changed(world, since)
        .into_iter()
        .map(|(e, c)| f(e, c))
        .collect()
}

/// For each entity with `T` changed after `since`, mutate via `f`.
///
/// Mutation through [`World::get_mut`] re-marks the component as changed at the
/// current tick.
pub fn for_each_changed_mut<T: Component>(
    world: &mut World,
    since: Tick,
    mut f: impl FnMut(Entity, &mut T),
) {
    let entities: Vec<Entity> = iter_changed::<T>(world, since)
        .into_iter()
        .map(|(e, _)| e)
        .collect();
    for e in entities {
        if let Some(c) = world.get_mut::<T>(e) {
            f(e, c);
        }
    }
}

/// Number of entities whose `T` changed after `since`.
pub fn count_changed<T: Component>(world: &World, since: Tick) -> usize {
    iter_changed::<T>(world, since).len()
}

/// Frame-scoped change cursor over one component type.
///
/// Holds the last tick it observed; each read yields only newer changes and
/// advances to the world's current tick. Create it once per system and reuse
/// across frames.
#[derive(Debug, Clone, Copy)]
pub struct ChangedCursor {
    last: Tick,
}

impl ChangedCursor {
    /// Cursor that will report every change recorded so far.
    pub fn new() -> Self {
        Self { last: Tick(0) }
    }

    /// Cursor starting at an explicit tick (exclusive).
    pub fn since(last: Tick) -> Self {
        Self { last }
    }

    /// Last observed tick.
    pub fn last_seen(&self) -> Tick {
        self.last
    }

    /// Pairs of `T` changed since the previous read; advances the cursor.
    pub fn read<'a, T: Component>(&mut self, world: &'a World) -> Vec<(Entity, &'a T)> {
        let out = iter_changed(world, self.last);
        self.last = world.change_detection().current();
        out
    }

    /// Entity ids of `T` changed since the previous read; advances the cursor.
    pub fn read_entities<T: Component>(&mut self, world: &World) -> Vec<Entity> {
        let out: Vec<Entity> = iter_changed::<T>(world, self.last)
            .into_iter()
            .map(|(e, _)| e)
            .collect();
        self.last = world.change_detection().current();
        out
    }
}

impl Default for ChangedCursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    #[derive(Clone, Debug, PartialEq)]
    struct Pos(f32);
    #[derive(Clone, Debug, PartialEq)]
    struct Vel(f32);
    #[derive(Clone, Debug, PartialEq)]
    struct Tag;
    #[derive(Clone, Debug, PartialEq)]
    struct Hp(i32);
    #[derive(Clone, Debug, PartialEq)]
    struct Mana(i32);

    #[test]
    fn query_helpers() {
        let mut w = World::new();
        let e1 = w.spawn();
        w.insert(e1, Pos(1.0));
        w.insert(e1, Vel(2.0));
        let e2 = w.spawn();
        w.insert(e2, Pos(3.0));
        w.insert(e2, Tag);

        assert_eq!(count_with::<Pos>(&w), 2);
        assert_eq!(entities_with_without::<Pos, Vel>(&w), vec![e2]);
        let q = query2::<Pos, Vel>(&w);
        assert_eq!(q.len(), 1);
        assert_eq!(q[0].0, e1);
        let names = map_components::<Pos, f32>(&w, |_, p| p.0);
        assert!(names.contains(&1.0) && names.contains(&3.0));
    }

    #[test]
    fn query4_and_partition() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1.0));
        w.insert(e, Vel(2.0));
        w.insert(e, Hp(10));
        w.insert(e, Mana(5));
        assert_eq!(query4::<Pos, Vel, Hp, Mana>(&w).len(), 1);
        let (hi, lo) = partition_with::<Hp>(&w, |_, h| h.0 >= 10);
        assert_eq!(hi, vec![e]);
        assert!(lo.is_empty());
        assert!(any_with::<Mana>(&w, |m| m.0 == 5));
        assert!(all_with::<Hp>(&w, |h| h.0 > 0));
    }

    #[test]
    fn for_each_mut_updates() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(1.0));
        for_each_mut::<Pos>(&mut w, |_, p| p.0 += 1.0);
        assert_eq!(w.get::<Pos>(e).unwrap().0, 2.0);
    }

    #[test]
    fn find_and_collect() {
        let mut w = World::new();
        let e = w.spawn();
        w.insert(e, Pos(9.0));
        assert_eq!(find_with::<Pos>(&w, |_, p| p.0 > 5.0), Some(e));
        let cloned = collect_cloned::<Pos>(&w);
        assert_eq!(cloned.len(), 1);
    }

    #[test]
    fn iter_changed_sees_inserts_and_mutations() {
        let mut w = World::new();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, Hp(10));
        w.insert(b, Hp(20));

        // Cursor at Tick(0) sees every insert.
        let seen = iter_changed::<Hp>(&w, Tick(0));
        assert_eq!(seen.len(), 2);
        let current = w.change_detection().current();

        // Nothing new since `current`.
        assert!(iter_changed::<Hp>(&w, current).is_empty());

        // Next frame: the world tick advances first, then systems mutate.
        w.advance_tick();
        w.get_mut::<Hp>(a).unwrap().0 = 11;
        w.get_mut_silent::<Hp>(b).unwrap().0 = 21;
        let seen = iter_changed::<Hp>(&w, current);
        assert_eq!(seen.iter().map(|(e, _)| *e).collect::<Vec<_>>(), vec![a]);
        assert_eq!(seen[0].1 .0, 11);
    }

    #[test]
    fn cursor_reports_each_change_exactly_once() {
        let mut w = World::new();
        let a = w.spawn();
        let b = w.spawn();
        w.insert(a, Mana(5));
        w.insert(b, Mana(7));

        let mut cursor = ChangedCursor::new();
        let first = cursor.read::<Mana>(&w);
        assert_eq!(first.len(), 2);

        // Frame 2: advance, no changes.
        w.advance_tick();
        assert!(cursor.read::<Mana>(&w).is_empty());

        // Frame 3: advance again, then mutate.
        w.advance_tick();
        w.get_mut::<Mana>(b).unwrap().0 += 1;
        let second = cursor.read_entities::<Mana>(&w);
        assert_eq!(second, vec![b]);
        w.advance_tick();
        assert!(cursor.read::<Mana>(&w).is_empty());
    }

    #[test]
    fn query2_changed_filters_on_first_component() {
        let mut w = World::new();
        let moving = w.spawn();
        w.insert(moving, Pos(1.0));
        w.insert(moving, Vel(1.0));
        let still = w.spawn();
        w.insert(still, Pos(2.0));
        w.insert(still, Vel(2.0));

        let since = w.change_detection().current();
        w.advance_tick();
        w.get_mut::<Pos>(moving).unwrap().0 = 5.0;

        let pairs = query2_changed::<Pos, Vel>(&w, since);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, moving);
        assert_eq!(pairs[0].2 .0, 1.0);
    }

    #[test]
    fn changed_map_and_for_each() {
        let mut w = World::new();
        let a = w.spawn();
        w.insert(a, Hp(3));
        let since = w.change_detection().current();

        w.advance_tick();
        w.insert(a, Hp(4));
        let values = map_changed::<Hp, i32>(&w, since, |_, h| h.0);
        assert_eq!(values, vec![4]);

        for_each_changed_mut::<Hp>(&mut w, since, |_, h| h.0 *= 2);
        assert_eq!(w.get::<Hp>(a).unwrap().0, 8);

        // The mutation above re-marked the component.
        assert_eq!(count_changed::<Hp>(&w, since), 1);
        w.advance_tick();
        assert_eq!(count_changed::<Hp>(&w, w.change_detection().current()), 0);
    }
}
