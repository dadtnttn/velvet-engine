use super::visuals::{self, Damage};
use crate::assets::GameAssets;
use crate::graphics::{Canvas, FontSystem, SvgCache};
use crate::model::FrameView;
/// Draw `frame` at `scale` canvas pixels per logical unit.
#[allow(clippy::too_many_arguments)]
pub fn render_scene(
    canvas: &mut Canvas,
    fonts: &FontSystem,
    svg: &mut SvgCache,
    assets: &GameAssets,
    frame: &FrameView,
    cursor: Option<[f32; 2]>,
    mouse_down: bool,
    scale: f32,
    damage: &mut Damage,
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
        scale,
        damage,
    );
}
