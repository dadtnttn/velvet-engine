//! Render plugin for App integration.

use velvet_app::{App, Plugin, ScheduleLabel};
use velvet_core::plugin::{PluginError, PluginId};
use velvet_math::Vec2;

use crate::batch::SpriteBatch;
use crate::camera::Camera2D;
use crate::profile::RenderProfile;
use crate::stats::RenderStats;
use crate::ClearColor;

/// Resources registered by [`RenderPlugin`] before a GPU context exists.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Profile.
    pub profile: RenderProfile,
    /// Virtual width.
    pub virtual_width: f32,
    /// Virtual height.
    pub virtual_height: f32,
    /// Clear color.
    pub clear: ClearColor,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            profile: RenderProfile::Default,
            virtual_width: 1920.0,
            virtual_height: 1080.0,
            clear: ClearColor::default(),
        }
    }
}

/// Frame-level render world data available to systems (CPU side).
#[derive(Debug, Default)]
pub struct RenderFrame {
    /// Primary camera.
    pub camera: Camera2D,
    /// Sprite batch for the frame.
    pub batch: SpriteBatch,
    /// Stats snapshot.
    pub stats: RenderStats,
}

/// Registers render CPU resources. GPU init happens in the window runner.
#[derive(Default)]
pub struct RenderPlugin {
    /// Config.
    pub config: RenderConfig,
}

impl Plugin for RenderPlugin {
    fn name(&self) -> &'static str {
        "velvet.render"
    }

    fn dependencies(&self) -> &[PluginId] {
        &[]
    }

    fn build(&self, app: &mut App) -> Result<(), PluginError> {
        let mut frame = RenderFrame {
            camera: Camera2D::virtual_res(self.config.virtual_width, self.config.virtual_height),
            ..Default::default()
        };
        frame.camera.viewport_width = self.config.virtual_width;
        frame.camera.viewport_height = self.config.virtual_height;

        app.insert_resource(self.config.clone());
        app.insert_resource(frame);
        app.insert_resource(RenderStats::default());

        app.add_system(ScheduleLabel::PreRender, |app| {
            if let Some(frame) = app.resource_mut::<RenderFrame>() {
                frame.batch.clear();
            }
        });
        app.add_system(ScheduleLabel::PostRender, |app| {
            if let Some(frame) = app.resource::<RenderFrame>() {
                let stats = frame.stats.clone();
                if let Some(out) = app.resource_mut::<RenderStats>() {
                    *out = stats;
                }
            }
        });
        Ok(())
    }
}

impl RenderConfig {
    /// Virtual size as vector.
    pub fn virtual_size(&self) -> Vec2 {
        Vec2::new(self.virtual_width, self.virtual_height)
    }
}
