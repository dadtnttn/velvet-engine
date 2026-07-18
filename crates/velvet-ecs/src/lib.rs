//! # velvet-ecs
//!
//! Lightweight ECS for 2D Velvet games: entities, components, queries, commands, resources.

#![deny(missing_docs)]

mod change_detect;
mod commands;
mod component;
mod entity;
mod events;
mod query;
mod registry;
mod world;

pub mod prelude;

pub use change_detect::{ChangeDetection, ChangedFilter, ComponentTicks, Tick};
pub use commands::{Command, CommandQueue};
pub use component::{Component, ComponentId, ComponentStorage};
pub use entity::{Entity, EntityMeta};
pub use events::{EventQueue, EventReader, EventWriter, Events};
pub use query::{
    all_with, any_with, collect_cloned, count_with, entities_with, entities_with2,
    entities_with2_without, entities_with_without, find_with, first_with, for_each_mut,
    map_components, partition_with, query2, query3, query4, Query, QueryIter,
};
pub use registry::{ComponentMeta, ComponentRegistry, RegistryManifest};
pub use world::World;
