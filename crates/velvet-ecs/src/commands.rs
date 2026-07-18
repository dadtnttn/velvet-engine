//! Deferred commands applied after systems.

use crate::component::Component;
use crate::entity::{Entity, EntityMeta};
use crate::world::World;

/// Deferred world mutation.
pub enum Command {
    /// Spawn empty entity with optional name.
    Spawn {
        /// Name.
        name: Option<String>,
        /// Out entity filled when applied (via queue response — simplified: ignored).
        _marker: (),
    },
    /// Despawn entity.
    Despawn {
        /// Target.
        entity: Entity,
    },
    /// Insert boxed component via type-erased apply.
    InsertDyn {
        /// Target.
        entity: Entity,
        /// Apply function.
        apply: Box<dyn FnOnce(&mut World) + Send>,
    },
    /// Remove typed component.
    RemoveDyn {
        /// Apply function.
        apply: Box<dyn FnOnce(&mut World) + Send>,
    },
}

/// Queue of commands.
#[derive(Default)]
pub struct CommandQueue {
    commands: Vec<Command>,
}

impl CommandQueue {
    /// Create empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue despawn.
    pub fn despawn(&mut self, entity: Entity) {
        self.commands.push(Command::Despawn { entity });
    }

    /// Queue insert.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.commands.push(Command::InsertDyn {
            entity,
            apply: Box::new(move |world| {
                world.insert(entity, value);
            }),
        });
    }

    /// Queue remove.
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.commands.push(Command::RemoveDyn {
            apply: Box::new(move |world| {
                world.remove::<T>(entity);
            }),
        });
    }

    /// Queue spawn with name; returns placeholder — use [`World::spawn`] for immediate.
    pub fn spawn_named(&mut self, name: impl Into<String>) {
        self.commands.push(Command::Spawn {
            name: Some(name.into()),
            _marker: (),
        });
    }

    /// Apply all commands to the world.
    pub fn apply(&mut self, world: &mut World) {
        let commands = std::mem::take(&mut self.commands);
        for cmd in commands {
            match cmd {
                Command::Spawn { name, .. } => {
                    let mut meta = EntityMeta::new();
                    meta.name = name;
                    let _ = world.spawn_with_meta(meta);
                }
                Command::Despawn { entity } => {
                    world.despawn(entity);
                }
                Command::InsertDyn { apply, .. } => apply(world),
                Command::RemoveDyn { apply } => apply(world),
            }
        }
    }

    /// Pending count.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
