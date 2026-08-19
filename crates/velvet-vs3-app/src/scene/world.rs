use super::visuals;
use crate::assets::GameAssets;
use crate::graphics::{Canvas, FontSystem};
use crate::model::FrameView;
pub fn render_scene(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    assets: &GameAssets,
    frame: &FrameView,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) {
    visuals::render(
        canvas,
        fonts,
        assets,
        &frame.commands,
        frame.time,
        cursor,
        mouse_down,
    );
}
