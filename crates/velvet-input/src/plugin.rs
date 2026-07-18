//! Input plugin registration.

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::PluginError;

use crate::state::InputState;

/// Registers [`InputState`] and per-frame bookkeeping systems.
#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn name(&self) -> &'static str {
        "velvet.input"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        app.insert_resource(InputState::with_defaults());
        app.add_system(ScheduleLabel::First, |app| {
            if let Some(input) = app.resource_mut::<InputState>() {
                input.begin_frame();
            }
        });
        app.add_system(ScheduleLabel::PreUpdate, |app| {
            if let Some(input) = app.resource_mut::<InputState>() {
                input.end_frame();
            }
        });
        Ok(())
    }
}
