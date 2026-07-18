//! 2D orthographic camera.

use serde::{Deserialize, Serialize};
use velvet_math::{Mat3, Rect, Vec2};

/// Orthographic camera in world space (Y-up, origin at center by default for view).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera2D {
    /// World-space center the camera looks at.
    pub target: Vec2,
    /// Rotation in radians (counter-clockwise).
    pub rotation: f32,
    /// Zoom multiplier (1.0 = identity). Larger = closer.
    pub zoom: f32,
    /// Virtual viewport width in world units (before zoom).
    pub viewport_width: f32,
    /// Virtual viewport height in world units (before zoom).
    pub viewport_height: f32,
    /// Draw order / layer priority among cameras (higher drawn later / on top for UI cams).
    pub priority: i32,
    /// Whether active.
    pub enabled: bool,
    /// Optional letterbox clear color override.
    pub clear: Option<[f32; 4]>,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            target: Vec2::ZERO,
            rotation: 0.0,
            zoom: 1.0,
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            priority: 0,
            enabled: true,
            clear: None,
        }
    }
}

impl Camera2D {
    /// Camera sized to virtual resolution looking at origin.
    pub fn virtual_res(width: f32, height: f32) -> Self {
        Self {
            viewport_width: width,
            viewport_height: height,
            ..Default::default()
        }
    }

    /// Visible world AABB at current zoom/target (axis-aligned, ignores rotation for bounds).
    pub fn visible_bounds(&self) -> Rect {
        let half = Vec2::new(
            self.viewport_width * 0.5 / self.zoom.max(1e-6),
            self.viewport_height * 0.5 / self.zoom.max(1e-6),
        );
        Rect::from_center_half_size(self.target, half)
    }

    /// View matrix: world → camera space (origin at target, then rotation/zoom).
    pub fn view_matrix(&self) -> Mat3 {
        let t = Mat3::from_translation(-self.target);
        let r = Mat3::from_angle(-self.rotation);
        let s = Mat3::from_scale(Vec2::splat(self.zoom));
        s * r * t
    }

    /// Projection matrix: camera space → NDC-ish clip for orthographic 2D.
    ///
    /// Maps x in [-vw/2, vw/2] and y in [-vh/2, vh/2] to [-1, 1] (Y-up).
    pub fn projection_matrix(&self) -> Mat3 {
        let sx = 2.0 / self.viewport_width.max(1e-6);
        let sy = 2.0 / self.viewport_height.max(1e-6);
        Mat3::from_cols([sx, 0.0, 0.0], [0.0, sy, 0.0], [0.0, 0.0, 1.0])
    }

    /// Combined view-projection matrix.
    pub fn view_projection(&self) -> Mat3 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Transform a world point to NDC (approx, z ignored).
    pub fn world_to_ndc(&self, world: Vec2) -> Vec2 {
        self.view_projection().transform_point2(world)
    }

    /// Transform NDC point back to world (inverse VP).
    pub fn ndc_to_world(&self, ndc: Vec2) -> Option<Vec2> {
        self.view_projection()
            .inverse()
            .map(|m| m.transform_point2(ndc))
    }

    /// Screen pixel (origin top-left, Y-down) to world, given letterboxed drawable rect.
    pub fn screen_to_world(
        &self,
        screen: Vec2,
        drawable_min: Vec2,
        drawable_size: Vec2,
    ) -> Option<Vec2> {
        if drawable_size.x <= 0.0 || drawable_size.y <= 0.0 {
            return None;
        }
        // Normalize to [0,1] in drawable, then to NDC [-1,1] with Y flip.
        let u = (screen.x - drawable_min.x) / drawable_size.x;
        let v = (screen.y - drawable_min.y) / drawable_size.y;
        let ndc = Vec2::new(u * 2.0 - 1.0, 1.0 - v * 2.0);
        self.ndc_to_world(ndc)
    }

    /// Follow a target with optional lerp factor (1.0 = snap).
    pub fn follow(&mut self, target: Vec2, lerp: f32) {
        let t = lerp.clamp(0.0, 1.0);
        self.target = self.target.lerp(target, t);
    }

    /// Shake offset (add to target for one frame; caller manages decay).
    pub fn apply_shake_offset(&mut self, offset: Vec2) {
        self.target += offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_maps_near_ndc_zero() {
        let cam = Camera2D::virtual_res(100.0, 100.0);
        let p = cam.world_to_ndc(Vec2::ZERO);
        assert!(p.x.abs() < 1e-4);
        assert!(p.y.abs() < 1e-4);
    }

    #[test]
    fn zoom_shrinks_visible_bounds() {
        let mut cam = Camera2D::virtual_res(100.0, 100.0);
        let a = cam.visible_bounds().width();
        cam.zoom = 2.0;
        let b = cam.visible_bounds().width();
        assert!((a - 100.0).abs() < 1e-3);
        assert!((b - 50.0).abs() < 1e-3);
    }

    #[test]
    fn screen_to_world_center() {
        let cam = Camera2D::virtual_res(200.0, 100.0);
        let world = cam
            .screen_to_world(Vec2::new(100.0, 50.0), Vec2::ZERO, Vec2::new(200.0, 100.0))
            .unwrap();
        assert!(world.x.abs() < 1e-3);
        assert!(world.y.abs() < 1e-3);
    }
}
