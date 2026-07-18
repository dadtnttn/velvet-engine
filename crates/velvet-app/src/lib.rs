//! # velvet-app
//!
//! Application shell for Velvet Engine: plugin registration, system schedules,
//! resources, states, and the headless/windowed run loops.
//!
//! ```ignore
//! use velvet_app::prelude::*;
//!
//! App::new()
//!     .add_plugin(MyPlugin)
//!     .add_system(Update, my_system)
//!     .run();
//! ```

#![deny(missing_docs)]

mod app;
mod change_tick;
mod exclusive;
mod ordering;
mod plugin;
mod schedule;
mod stage;
mod system;
mod world_resources;

#[cfg(feature = "window")]
mod window_runner;

pub mod prelude;

pub use app::{App, AppExitCode, HeadlessRunner, Runner};
pub use change_tick::{ChangeCursor, ChangeCursorMap, ChangeTicks, Tick};
pub use exclusive::{run_exclusive_commands, ExclusiveCommand, ExclusiveSystemQueue};
pub use ordering::{labels as system_labels, OrderError, SystemLabel, SystemOrderGraph};
pub use plugin::{Plugin, PluginGroup, PluginRegistration};
pub use schedule::{IntoSystemConfigs, ScheduleLabel, Schedules, SystemStage};
pub use stage::{StageEdge, StageFrameStats, StageId, StageKey, StageSchedule};
pub use system::{BoxedSystem, SystemFn, SystemId};
pub use world_resources::{Res, ResMut, Resource, ResourceId, Resources};

#[cfg(feature = "window")]
pub use window_runner::{WindowFrameHook, WindowInitHook, WindowResizeHook, WindowRunner};

/// Re-export core run mode for convenience.
pub use velvet_core::RunMode;
