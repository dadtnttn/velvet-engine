//! Story plugin for App integration.
//!
//! # How hosts wire into the core
//!
//! 1. Load a program (`velvet-story-lang` boot for `.vstory`, or `load_program_from_source`).
//! 2. `StoryPlayer::start` / `start_with_host` — register [`crate::StoryCommandHost`]
//!    (e.g. `velvet_action::CombatStoryHost`) on the player **before** play.
//! 3. Insert `StoryPlayer` or product [`VnSession`] as an app resource.
//! 4. `app.add_plugin(StoryPlugin)` so Update ticks pause/auto/presentation.
//!
//! The plugin does **not** invent a game host; games supply `StoryCommandHost`.

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
