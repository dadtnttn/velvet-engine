//! Simple query helpers over the world.

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
}
