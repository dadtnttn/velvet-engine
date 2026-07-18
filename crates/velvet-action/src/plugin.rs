//! Action plugin.

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::PluginError;
use velvet_time::Time;

use crate::projectile::ProjectileSystem;
use crate::score::ScoreBoard;

/// Registers score + projectile resources and ticks them.
#[derive(Default)]
pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn name(&self) -> &'static str {
        "velvet.action"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        if app.resource::<ScoreBoard>().is_none() {
            app.insert_resource(ScoreBoard::default());
        }
        if app.resource::<ProjectileSystem>().is_none() {
            app.insert_resource(ProjectileSystem::default());
        }
        app.add_system(ScheduleLabel::Update, |app| {
            let dt = app
                .resource::<Time>()
                .map(|t| t.scaled_delta_secs())
                .unwrap_or(0.0);
            if let Some(score) = app.resource_mut::<ScoreBoard>() {
                score.tick(dt);
            }
        });
        Ok(())
    }
}
