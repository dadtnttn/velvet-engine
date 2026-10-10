use super::visuals;
use crate::assets::GameAssets;
use crate::graphics::{Canvas, FontSystem, SvgCache};
use crate::model::FrameView;
pub fn render_scene(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    svg: &mut SvgCache,
    assets: &GameAssets,
    frame: &FrameView,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
) {
    visuals::render(
        canvas,
        fonts,
        svg,
        assets,
        &frame.commands,
        frame.time,
        cursor,
        mouse_down,
    );
}
