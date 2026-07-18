//! System schedules and ordered stages.

use std::collections::HashMap;
use std::fmt;

use crate::system::{BoxedSystem, SystemId};

/// Label identifying when systems run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScheduleLabel {
    /// Before startup systems.
    PreStartup,
    /// One-shot startup.
    Startup,
    /// After startup.
    PostStartup,
    /// Start of frame.
    First,
    /// Before variable update.
    PreUpdate,
    /// Fixed timestep updates (may run 0..N times per frame).
    FixedUpdate,
    /// Variable update.
    Update,
    /// After variable update.
    PostUpdate,
    /// Before rendering.
    PreRender,
    /// Render submission.
    Render,
    /// After rendering.
    PostRender,
    /// End of frame bookkeeping.
    Last,
    /// Application shutdown.
    Shutdown,
    /// Custom named schedule.
    Custom(&'static str),
}

impl fmt::Display for ScheduleLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreStartup => write!(f, "PreStartup"),
            Self::Startup => write!(f, "Startup"),
            Self::PostStartup => write!(f, "PostStartup"),
            Self::First => write!(f, "First"),
            Self::PreUpdate => write!(f, "PreUpdate"),
            Self::FixedUpdate => write!(f, "FixedUpdate"),
            Self::Update => write!(f, "Update"),
            Self::PostUpdate => write!(f, "PostUpdate"),
            Self::PreRender => write!(f, "PreRender"),
            Self::Render => write!(f, "Render"),
            Self::PostRender => write!(f, "PostRender"),
            Self::Last => write!(f, "Last"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

/// Ordered list of systems for one label.
#[derive(Default)]
pub struct SystemStage {
    systems: Vec<BoxedSystem>,
    next_id: u64,
}

impl SystemStage {
    /// Add a system function.
    pub fn add<F>(&mut self, f: F) -> SystemId
    where
        F: for<'a> FnMut(&'a mut crate::app::App) + Send + Sync + 'static,
    {
        let id = SystemId(self.next_id);
        self.next_id += 1;
        self.systems.push(BoxedSystem::new(id, f));
        id
    }

    /// Number of systems.
    pub fn len(&self) -> usize {
        self.systems.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }

    /// Iterate systems in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &BoxedSystem> {
        self.systems.iter()
    }

    /// Mutable iterate.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BoxedSystem> {
        self.systems.iter_mut()
    }

    /// Mutable access to the underlying system list (for the app runner).
    pub(crate) fn systems_mut(&mut self) -> &mut Vec<BoxedSystem> {
        &mut self.systems
    }
}

/// Collection of labeled schedules.
#[derive(Default)]
pub struct Schedules {
    stages: HashMap<ScheduleLabel, SystemStage>,
}

impl Schedules {
    /// Create empty schedules.
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow or create a stage.
    pub fn stage_mut(&mut self, label: ScheduleLabel) -> &mut SystemStage {
        self.stages.entry(label).or_default()
    }

    /// Immutable stage if present.
    pub fn stage(&self, label: ScheduleLabel) -> Option<&SystemStage> {
        self.stages.get(&label)
    }

    /// Add system to a labeled schedule.
    pub fn add_system<F>(&mut self, label: ScheduleLabel, f: F) -> SystemId
    where
        F: for<'a> FnMut(&'a mut crate::app::App) + Send + Sync + 'static,
    {
        self.stage_mut(label).add(f)
    }

    /// Labels currently registered.
    pub fn labels(&self) -> impl Iterator<Item = ScheduleLabel> + '_ {
        self.stages.keys().copied()
    }
}

/// Trait for ergonomic system registration chaining (placeholder for future config).
pub trait IntoSystemConfigs<Marker> {
    /// Convert into a system fn registration.
    fn into_configs(self) -> Self;
}

impl ScheduleLabel {
    /// Default per-frame order (excluding startup/shutdown/fixed).
    pub fn frame_order() -> &'static [ScheduleLabel] {
        &[
            ScheduleLabel::First,
            ScheduleLabel::PreUpdate,
            ScheduleLabel::Update,
            ScheduleLabel::PostUpdate,
            ScheduleLabel::PreRender,
            ScheduleLabel::Render,
            ScheduleLabel::PostRender,
            ScheduleLabel::Last,
        ]
    }

    /// Startup order.
    pub fn startup_order() -> &'static [ScheduleLabel] {
        &[
            ScheduleLabel::PreStartup,
            ScheduleLabel::Startup,
            ScheduleLabel::PostStartup,
        ]
    }
}
