use winit::dpi::{PhysicalPosition, PhysicalSize};

pub fn transform(
    window_w: usize,
    window_h: usize,
    canvas_w: usize,
    canvas_h: usize,
) -> (f32, f32, f32) {
    let scale = (window_w as f32 / canvas_w as f32)
        .min(window_h as f32 / canvas_h as f32)
        .max(0.001);
    (
        scale,
        (window_w as f32 - canvas_w as f32 * scale) * 0.5,
        (window_h as f32 - canvas_h as f32 * scale) * 0.5,
    )
}
pub fn window_to_canvas(
    position: PhysicalPosition<f64>,
    size: PhysicalSize<u32>,
    canvas_w: usize,
    canvas_h: usize,
) -> Option<[f32; 2]> {
    let (scale, ox, oy) = transform(
        size.width as usize,
        size.height as usize,
        canvas_w,
        canvas_h,
    );
    let x = (position.x as f32 - ox) / scale;
    let y = (position.y as f32 - oy) / scale;
    (x >= 0.0 && y >= 0.0 && x < canvas_w as f32 && y < canvas_h as f32).then_some([x, y])
}
pub fn scale_letterboxed(
    source: &[u32],
    source_w: usize,
    source_h: usize,
    destination: &mut [u32],
    width: usize,
    height: usize,
) {
    destination.fill(0xff080a0d);
    let (scale, ox, oy) = transform(width, height, source_w, source_h);
    let vw = (source_w as f32 * scale).round() as usize;
    let vh = (source_h as f32 * scale).round() as usize;
    let sx0 = ox.max(0.0).round() as usize;
    let sy0 = oy.max(0.0).round() as usize;
    for dy in 0..vh.min(height.saturating_sub(sy0)) {
        let sy = (dy as f32 / scale) as usize;
        for dx in 0..vw.min(width.saturating_sub(sx0)) {
            let sx = (dx as f32 / scale) as usize;
            destination[(sy0 + dy) * width + sx0 + dx] =
                source[sy.min(source_h - 1) * source_w + sx.min(source_w - 1)];
        }
    }
}
