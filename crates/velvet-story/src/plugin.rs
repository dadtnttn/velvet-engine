//! Story plugin for App integration.

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::PluginError;
use velvet_time::Time;

use crate::product::VnSession;
use crate::runtime::StoryPlayer;

/// Registers optional [`StoryPlayer`] / [`VnSession`] resource handling.
///
/// The host inserts a `StoryPlayer` or product `VnSession` after loading;
/// this plugin ticks presentation each frame when present.
#[derive(Default)]
pub struct StoryPlugin;

impl Plugin for StoryPlugin {
    fn name(&self) -> &'static str {
        "velvet.story"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        app.add_system(ScheduleLabel::Update, |app| {
            let dt = app
                .resource::<Time>()
                .map(|t| t.scaled_delta_secs())
                .unwrap_or(0.0);
            // Product session takes priority (ticks say/auto/bgm/presentation).
            if let Some(session) = app.resource_mut::<VnSession>() {
                session.tick(dt);
                return;
            }
            if let Some(story) = app.resource_mut::<StoryPlayer>() {
                story.tick(dt);
            }
        });
        Ok(())
    }
}
