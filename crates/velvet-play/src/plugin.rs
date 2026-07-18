//! Play plugin registration.

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::PluginError;
use velvet_input::builtin;
use velvet_input::InputState;
use velvet_time::Time;

use crate::world::PlayWorld;

/// Registers systems that step [`PlayWorld`] when present as a resource.
#[derive(Default)]
pub struct PlayPlugin;

impl Plugin for PlayPlugin {
    fn name(&self) -> &'static str {
        "velvet.play"
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        app.add_system(ScheduleLabel::FixedUpdate, |app| {
            let dt = app
                .resource::<Time>()
                .map(|t| {
                    // Use fixed step from FixedTime if available
                    t.scaled_delta_secs()
                })
                .unwrap_or(1.0 / 60.0);
            // Prefer fixed step size
            let fixed = app
                .resource::<velvet_time::FixedTime>()
                .map(|f| f.step_secs())
                .unwrap_or(dt);

            // Apply input to player before step (copy values to avoid borrow clash).
            let (axis, interact) = app
                .resource::<InputState>()
                .map(|input| {
                    (
                        input.axis2(builtin::MOVE).to_vec2(),
                        input.just_pressed(builtin::INTERACT),
                    )
                })
                .unwrap_or((velvet_math::Vec2::ZERO, false));

            if let Some(world) = app.resource_mut::<PlayWorld>() {
                world.set_player_input(axis);
                if interact {
                    world.try_player_interact(true);
                }
                world.step(fixed);
            }
        });
        Ok(())
    }
}
