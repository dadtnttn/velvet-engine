//! Audio plugin.

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::PluginError;
use velvet_time::Time;

use crate::engine::AudioEngine;

/// Registers [`AudioEngine`] and ticks it each frame.
#[derive(Default)]
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn name(&self) -> &'static str {
        "velvet.audio"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        app.insert_resource(AudioEngine::new());
        app.add_system(ScheduleLabel::PostUpdate, |app| {
            let dt = app
                .resource::<Time>()
                .map(|t| t.scaled_delta_secs())
                .unwrap_or(0.0);
            if let Some(audio) = app.resource_mut::<AudioEngine>() {
                audio.tick(dt);
            }
        });
        Ok(())
    }
}
