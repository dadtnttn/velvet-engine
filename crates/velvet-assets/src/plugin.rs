//! Assets plugin.

use std::path::PathBuf;
use std::sync::Arc;

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::PluginError;

use crate::registry::Assets;
use crate::source::FileSource;

/// Configuration for [`AssetsPlugin`].
#[derive(Debug, Clone)]
pub struct AssetsPlugin {
    /// Optional filesystem root (default: `assets`).
    pub root: PathBuf,
    /// Use memory source instead of filesystem.
    pub memory: bool,
}

impl Default for AssetsPlugin {
    fn default() -> Self {
        Self {
            root: PathBuf::from("assets"),
            memory: false,
        }
    }
}

impl Plugin for AssetsPlugin {
    fn name(&self) -> &'static str {
        "velvet.assets"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        let assets = if self.memory {
            Assets::memory()
        } else {
            Assets::with_source(Arc::new(FileSource::new(self.root.clone())))
        };
        app.insert_resource(assets);
        app.add_system(ScheduleLabel::Last, |_app| {
            // Future: poll notify watcher and reload.
        });
        Ok(())
    }
}
