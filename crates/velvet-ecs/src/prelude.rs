//! ECS prelude.

pub use crate::change_detect::{ChangeDetection, ChangedFilter, Tick};
pub use crate::commands::{Command, CommandQueue};
pub use crate::component::{Component, ComponentId};
pub use crate::entity::{Entity, EntityMeta};
pub use crate::events::{EventQueue, Events};
pub use crate::query::{query2, query3, Query};
pub use crate::registry::{ComponentMeta, ComponentRegistry};
pub use crate::world::World;
