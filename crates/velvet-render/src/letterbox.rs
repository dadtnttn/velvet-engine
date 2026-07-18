//! Virtual resolution and letterboxing.

use serde::{Deserialize, Serialize};
use velvet_math::{Rect, Vec2};

/// How virtual resolution maps to the physical window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScalingMode {
    /// Keep aspect ratio, pad with bars (default).
    #[default]
    Letterbox,
    /// Keep aspect ratio, crop overflow.
    Crop,
    /// Stretch to fill (distorts).
    Stretch,
    /// Integer scale only (pixel art).
    IntegerScale,
}

/// Computed drawable region inside a physical framebuffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Letterbox {
    /// Pixel rect where the virtual image is drawn.
    pub drawable: Rect,
    /// Scale applied from virtual to physical.
    pub scale: f32,
    /// Physical framebuffer size.
    pub physical: Vec2,
    /// Virtual resolution.
    pub virtual_size: Vec2,
}

impl Letterbox {
    /// Drawable origin.
    pub fn offset(self) -> Vec2 {
        self.drawable.min
    }

    /// Drawable size.
    pub fn size(self) -> Vec2 {
        self.drawable.size()
    }
}

/// Compute letterbox / scaling for a virtual resolution into a physical size.
pub fn compute_letterbox(
    physical_w: f32,
    physical_h: f32,
    virtual_w: f32,
    virtual_h: f32,
    mode: ScalingMode,
) -> Letterbox {
    let physical = Vec2::new(physical_w.max(1.0), physical_h.max(1.0));
    let virtual_size = Vec2::new(virtual_w.max(1.0), virtual_h.max(1.0));

    match mode {
        ScalingMode::Stretch => Letterbox {
            drawable: Rect::from_pos_size(Vec2::ZERO, physical),
            scale: 1.0,
            physical,
            virtual_size,
        },
        ScalingMode::Letterbox | ScalingMode::IntegerScale | ScalingMode::Crop => {
            let sx = physical.x / virtual_size.x;
            let sy = physical.y / virtual_size.y;
            let mut scale = match mode {
                ScalingMode::Crop => sx.max(sy),
                ScalingMode::IntegerScale => sx.min(sy).floor().max(1.0),
                _ => sx.min(sy),
            };
            if !scale.is_finite() || scale <= 0.0 {
                scale = 1.0;
            }
            let draw_w = virtual_size.x * scale;
            let draw_h = virtual_size.y * scale;
            let ox = ((physical.x - draw_w) * 0.5).max(0.0);
            let oy = ((physical.y - draw_h) * 0.5).max(0.0);
            // For crop, drawable may extend outside; clamp to physical for viewport.
            let drawable = if mode == ScalingMode::Crop {
                Rect::from_pos_size(
                    Vec2::new((physical.x - draw_w) * 0.5, (physical.y - draw_h) * 0.5),
                    Vec2::new(draw_w, draw_h),
                )
            } else {
                Rect::from_pos_size(Vec2::new(ox, oy), Vec2::new(draw_w, draw_h))
            };
            Letterbox {
                drawable,
                scale,
                physical,
                virtual_size,
            }
        }
    }
}

/// Convert a wgpu viewport from a letterbox (scissor/viewport in pixels).
pub fn letterbox_viewport(lb: Letterbox) -> (f32, f32, f32, f32) {
    let r = lb.drawable;
    // Clamp to physical bounds for valid GPU viewport.
    let x = r.min.x.max(0.0);
    let y = r.min.y.max(0.0);
    let w = r.width().min(lb.physical.x - x).max(1.0);
    let h = r.height().min(lb.physical.y - y).max(1.0);
    (x, y, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_centers_16_9_in_4_3() {
        let lb = compute_letterbox(800.0, 600.0, 1920.0, 1080.0, ScalingMode::Letterbox);
        assert!(lb.scale > 0.0);
        // Bars on top/bottom or sides.
        let area = lb.drawable.area();
        assert!(area < 800.0 * 600.0);
        assert!((lb.drawable.center().x - 400.0).abs() < 1.0);
    }

    #[test]
    fn integer_scale_is_whole() {
        let lb = compute_letterbox(800.0, 600.0, 320.0, 180.0, ScalingMode::IntegerScale);
        assert!((lb.scale - lb.scale.floor()).abs() < 1e-5);
        assert!(lb.scale >= 1.0);
    }

    #[test]
    fn stretch_fills() {
        let lb = compute_letterbox(100.0, 50.0, 10.0, 10.0, ScalingMode::Stretch);
        assert!((lb.drawable.width() - 100.0).abs() < 1e-5);
        assert!((lb.drawable.height() - 50.0).abs() < 1e-5);
    }
}
