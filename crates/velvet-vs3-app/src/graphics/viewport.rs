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
/// Largest canvas pixels per logical unit: 3 is a 1280x720 app on a 4K
/// screen, and keeps every SVG raster within its size limit. Bigger windows
/// stretch the canvas the rest of the way.
pub const MAX_RENDER_SCALE: f32 = 3.0;

/// Canvas pixels per logical unit for a window: the letterbox scale, so the
/// app is drawn at the window's real resolution instead of stretched.
pub fn render_scale(window_w: usize, window_h: usize, canvas_w: usize, canvas_h: usize) -> f32 {
    let (scale, _, _) = transform(window_w, window_h, canvas_w, canvas_h);
    scale.clamp(0.25, MAX_RENDER_SCALE)
}

/// Copy a canvas already drawn at the window's scale, centered (letterbox
/// bars around it), with no resampling.
pub fn blit_letterboxed(
    source: &[u32],
    source_w: usize,
    source_h: usize,
    destination: &mut [u32],
    width: usize,
    height: usize,
) {
    const BAR: u32 = 0xff080a0d;
    let x0 = width.saturating_sub(source_w) / 2;
    let y0 = height.saturating_sub(source_h) / 2;
    let w = source_w.min(width - x0);
    let h = source_h.min(height - y0);
    // Bars only where the canvas does not cover.
    destination[..y0 * width].fill(BAR);
    destination[(y0 + h) * width..].fill(BAR);
    for y in 0..h {
        let at = (y0 + y) * width;
        let row = &mut destination[at..at + width];
        row[..x0].fill(BAR);
        row[x0..x0 + w].copy_from_slice(&source[y * source_w..y * source_w + w]);
        row[x0 + w..].fill(BAR);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_4k_window_draws_at_three_times_the_canvas() {
        assert_eq!(render_scale(3840, 2160, 1280, 720), 3.0);
        assert_eq!(render_scale(1920, 1080, 1280, 720), 1.5);
        // 8K stops at the limit and is stretched the rest of the way.
        assert_eq!(render_scale(7680, 4320, 1280, 720), MAX_RENDER_SCALE);
    }

    #[test]
    fn blit_centers_with_letterbox_bars() {
        let source = vec![0xffffffff; 4 * 2];
        let mut screen = vec![0; 6 * 4];
        blit_letterboxed(&source, 4, 2, &mut screen, 6, 4);
        assert_eq!(screen[0], 0xff080a0d);
        assert_eq!(screen[6 + 1], 0xffffffff);
        assert_eq!(screen[6 + 5], 0xff080a0d);
    }
}
