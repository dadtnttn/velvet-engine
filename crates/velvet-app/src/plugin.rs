//! Plugin trait and registration helpers.

use crate::app::App;
use velvet_core::plugin::{PluginError, PluginId};

/// A modular unit of engine functionality.
///
/// Plugins are registered on [`App`], ordered by dependencies, then built.
pub trait Plugin: Send + Sync + 'static {
    /// Unique human-readable name (also used as default id).
    fn name(&self) -> &'static str;

    /// Stable id; defaults to [`Self::name`].
    fn id(&self) -> PluginId {
        PluginId::new(self.name())
    }

    /// Plugins that must be registered and built before this one.
    fn dependencies(&self) -> &[PluginId] {
        &[]
    }

    /// Whether this plugin should be active given app state (default true).
    fn is_enabled(&self, _app: &App) -> bool {
        true
    }

    /// Register systems, resources, and events.
    fn build(&self, app: &mut App) -> Result<(), PluginError>;

    /// Called after all plugins have been built (cross-plugin wiring).
    fn finish(&self, app: &mut App) -> Result<(), PluginError> {
        let _ = app;
        Ok(())
    }
}

/// Metadata captured at registration time.
pub struct PluginRegistration {
    /// Boxed plugin.
    pub plugin: Box<dyn Plugin>,
    /// Id snapshot.
    pub id: PluginId,
    /// Declared dependencies.
    pub dependencies: Vec<PluginId>,
}

/// Group of plugins added together.
pub trait PluginGroup {
    /// Add all plugins in the group to the app.
    fn build(self, app: &mut App);
}

/// Empty helper plugin used in tests.
#[derive(Default)]
pub struct NullPlugin;

impl Plugin for NullPlugin {
    fn name(&self) -> &'static str {
        "velvet.null"
    }

    fn build(&self, _app: &mut App) -> Result<(), PluginError> {
        Ok(())
    }
}
