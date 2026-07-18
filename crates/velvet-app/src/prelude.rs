//! Prelude for application authors.

pub use crate::app::{App, AppExitCode, HeadlessRunner, Runner};
pub use crate::change_tick::{ChangeCursor, ChangeTicks, Tick};
pub use crate::ordering::{labels as system_labels, SystemLabel, SystemOrderGraph};
pub use crate::plugin::{NullPlugin, Plugin, PluginGroup};
pub use crate::schedule::ScheduleLabel;
pub use crate::world_resources::{Res, ResMut, Resource, Resources};
#[cfg(feature = "window")]
pub use crate::{WindowFrameHook, WindowInitHook, WindowResizeHook, WindowRunner};
pub use velvet_core::prelude::*;
pub use velvet_events::{AppExit, AppLifecycleEvent, EventReader, EventWriter, Events};
pub use velvet_time::{FixedTime, Time, Timer};
