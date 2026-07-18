//! Hierarchy components stored in the ECS world.

use serde::{Deserialize, Serialize};
use velvet_ecs::{Entity, World};

/// Display name component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Name(pub String);

impl Name {
    /// Create.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// As str.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Parent entity link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parent(pub Entity);

/// Child list (maintained by hierarchy systems / scene loader).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Children(pub Vec<Entity>);

impl Children {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push unique child.
    pub fn add(&mut self, child: Entity) {
        if !self.0.contains(&child) {
            self.0.push(child);
        }
    }

    /// Remove child.
    pub fn remove(&mut self, child: Entity) {
        self.0.retain(|e| *e != child);
    }

    /// Number of children.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Attach `child` under `parent`, maintaining [`Parent`] and [`Children`].
pub fn attach_child(world: &mut World, parent: Entity, child: Entity) {
    if let Some(old) = world.get::<Parent>(child).map(|p| p.0) {
        if old != parent {
            if let Some(ch) = world.get_mut::<Children>(old) {
                ch.remove(child);
            }
        }
    }
    world.insert(child, Parent(parent));
    if let Some(ch) = world.get_mut::<Children>(parent) {
        ch.add(child);
    } else {
        let mut ch = Children::new();
        ch.add(child);
        world.insert(parent, ch);
    }
}

/// Detach `child` from its parent if any.
pub fn detach_child(world: &mut World, child: Entity) {
    if let Some(Parent(p)) = world.get::<Parent>(child).copied() {
        if let Some(ch) = world.get_mut::<Children>(p) {
            ch.remove(child);
        }
        // drop Parent component by replacing with empty — World may not have remove; re-insert none
        // Use components remove if available
        world.remove::<Parent>(child);
    }
}

/// Depth-first walk of descendants (not including `root`).
pub fn walk_descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut stack: Vec<Entity> = world
        .get::<Children>(root)
        .map(|c| c.0.clone())
        .unwrap_or_default();
    while let Some(e) = stack.pop() {
        out.push(e);
        if let Some(ch) = world.get::<Children>(e) {
            stack.extend(ch.0.iter().copied());
        }
    }
    out
}

/// Find first descendant (or self) with matching [`Name`].
pub fn find_by_name(world: &World, root: Entity, name: &str) -> Option<Entity> {
    if world
        .get::<Name>(root)
        .map(|n| n.as_str() == name)
        .unwrap_or(false)
    {
        return Some(root);
    }
    walk_descendants(world, root).into_iter().find(|&e| {
        world
            .get::<Name>(e)
            .map(|n| n.as_str() == name)
            .unwrap_or(false)
    })
}

/// Ancestors from parent up to root (excluding `entity`).
pub fn ancestors(world: &World, entity: Entity) -> Vec<Entity> {
    let mut out = Vec::new();
    let mut cur = world.get::<Parent>(entity).map(|p| p.0);
    let mut guard = 0;
    while let Some(e) = cur {
        if guard > 256 {
            break;
        }
        guard += 1;
        out.push(e);
        cur = world.get::<Parent>(e).map(|p| p.0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attach_and_find() {
        let mut w = World::new();
        let root = w.spawn();
        w.insert(root, Name::new("root"));
        let child = w.spawn();
        w.insert(child, Name::new("child"));
        attach_child(&mut w, root, child);
        assert_eq!(w.get::<Children>(root).unwrap().len(), 1);
        assert_eq!(find_by_name(&w, root, "child"), Some(child));
        assert_eq!(ancestors(&w, child), vec![root]);
        detach_child(&mut w, child);
        assert!(w.get::<Children>(root).unwrap().is_empty());
    }
}
