//! System function types.

use crate::app::App;

/// Opaque system identifier within a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SystemId(pub u64);

/// System callback invoked with exclusive access to the app.
pub trait SystemFn: Send + Sync {
    /// Run the system.
    fn run(&mut self, app: &mut App);
}

impl<F> SystemFn for F
where
    F: for<'a> FnMut(&'a mut App) + Send + Sync + 'static,
{
    fn run(&mut self, app: &mut App) {
        (self)(app);
    }
}

/// Type-erased system stored in a schedule.
pub struct BoxedSystem {
    id: SystemId,
    name: Option<&'static str>,
    func: Box<dyn SystemFn>,
    enabled: bool,
}

impl BoxedSystem {
    /// Create from id and function.
    pub fn new<F: SystemFn + 'static>(id: SystemId, f: F) -> Self {
        Self {
            id,
            name: None,
            func: Box::new(f),
            enabled: true,
        }
    }

    /// System id.
    pub fn id(&self) -> SystemId {
        self.id
    }

    /// Optional debug name.
    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// Set debug name.
    pub fn set_name(&mut self, name: &'static str) {
        self.name = Some(name);
    }

    /// Whether enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable/disable.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Run if enabled.
    pub fn run(&mut self, app: &mut App) {
        if self.enabled {
            self.func.run(app);
        }
    }
}
