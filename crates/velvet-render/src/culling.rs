//! AABB vs camera culling helpers.

use velvet_math::{Aabb2, Rect, Vec2};

use crate::camera::Camera2D;

/// Result of a cull test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullResult {
    /// Fully outside — skip draw.
    Outside,
    /// Partially visible.
    Partial,
    /// Fully inside frustum/bounds.
    Inside,
}

impl CullResult {
    /// Whether the object should be drawn.
    pub fn is_visible(self) -> bool {
        !matches!(self, Self::Outside)
    }
}

/// Axis-aligned frustum in world space (camera visible bounds, optionally padded).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraFrustum2D {
    /// World AABB of the view.
    pub bounds: Aabb2,
}

impl CameraFrustum2D {
    /// From camera visible bounds.
    pub fn from_camera(camera: &Camera2D) -> Self {
        Self {
            bounds: Aabb2::from_rect(camera.visible_bounds()),
        }
    }

    /// From camera with margin in world units (positive expands).
    pub fn from_camera_padded(camera: &Camera2D, padding: f32) -> Self {
        Self {
            bounds: Aabb2::from_rect(camera.visible_bounds()).inflate(padding),
        }
    }

    /// From explicit rect.
    pub fn from_rect(rect: Rect) -> Self {
        Self {
            bounds: Aabb2::from_rect(rect),
        }
    }

    /// Cull an AABB.
    pub fn cull_aabb(&self, aabb: Aabb2) -> CullResult {
        if !self.bounds.intersects(aabb) {
            CullResult::Outside
        } else if self.bounds.contains_aabb(aabb) {
            CullResult::Inside
        } else {
            CullResult::Partial
        }
    }

    /// Cull a point.
    pub fn cull_point(&self, p: Vec2) -> CullResult {
        if self.bounds.contains_point(p) {
            CullResult::Inside
        } else {
            CullResult::Outside
        }
    }

    /// Cull a circle as its AABB (conservative).
    pub fn cull_circle(&self, center: Vec2, radius: f32) -> CullResult {
        self.cull_aabb(Aabb2::from_center_radius(center, radius))
    }

    /// Cull sprite-like AABB from center, size, and optional rotation (OOBB → AABB).
    pub fn cull_oriented_rect(&self, center: Vec2, size: Vec2, rotation: f32) -> CullResult {
        let half = size * 0.5;
        let aabb = if rotation.abs() < 1e-5 {
            Aabb2::from_center_extents(center, half)
        } else {
            let m = velvet_math::Mat3::from_scale_angle_translation(Vec2::ONE, rotation, center);
            Aabb2::from_center_extents(Vec2::ZERO, half).transformed(m)
        };
        self.cull_aabb(aabb)
    }
}

/// Filter a list of AABBs, returning indices that are visible.
pub fn cull_aabbs(frustum: &CameraFrustum2D, aabbs: &[Aabb2]) -> Vec<usize> {
    aabbs
        .iter()
        .enumerate()
        .filter(|(_, a)| frustum.cull_aabb(**a).is_visible())
        .map(|(i, _)| i)
        .collect()
}

/// Count visible among AABBs.
pub fn count_visible(frustum: &CameraFrustum2D, aabbs: &[Aabb2]) -> usize {
    aabbs
        .iter()
        .filter(|a| frustum.cull_aabb(**a).is_visible())
        .count()
}

/// Expand camera bounds for shadow/LODs etc.
pub fn expanded_visible_bounds(camera: &Camera2D, scale: f32) -> Rect {
    let b = camera.visible_bounds();
    let c = b.center();
    let half = b.size() * 0.5 * scale.max(0.0);
    Rect::from_center_half_size(c, half)
}

/// Whether a world-space rect is on screen for this camera (AABB test).
pub fn is_on_screen(camera: &Camera2D, world_aabb: Aabb2, padding: f32) -> bool {
    CameraFrustum2D::from_camera_padded(camera, padding)
        .cull_aabb(world_aabb)
        .is_visible()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_inside() {
        let cam = Camera2D::virtual_res(100.0, 100.0);
        let f = CameraFrustum2D::from_camera(&cam);
        let inside = Aabb2::from_center_extents(Vec2::ZERO, Vec2::splat(10.0));
        assert_eq!(f.cull_aabb(inside), CullResult::Inside);
        let outside = Aabb2::from_center_extents(Vec2::new(1000.0, 0.0), Vec2::splat(1.0));
        assert_eq!(f.cull_aabb(outside), CullResult::Outside);
    }

    #[test]
    fn partial_overlap() {
        let cam = Camera2D::virtual_res(100.0, 100.0);
        let f = CameraFrustum2D::from_camera(&cam);
        // Camera bounds ~[-50,50]; box from 40..60 overlaps edge.
        let partial = Aabb2::from_pos_size(Vec2::new(40.0, 0.0), Vec2::new(30.0, 10.0));
        assert_eq!(f.cull_aabb(partial), CullResult::Partial);
    }

    #[test]
    fn cull_list() {
        let cam = Camera2D::virtual_res(50.0, 50.0);
        let f = CameraFrustum2D::from_camera(&cam);
        let boxes = [
            Aabb2::from_center_radius(Vec2::ZERO, 1.0),
            Aabb2::from_center_radius(Vec2::new(500.0, 0.0), 1.0),
        ];
        let vis = cull_aabbs(&f, &boxes);
        assert_eq!(vis, vec![0]);
        assert_eq!(count_visible(&f, &boxes), 1);
    }

    #[test]
    fn oriented_larger_than_unrotated() {
        let cam = Camera2D::virtual_res(20.0, 20.0);
        let f = CameraFrustum2D::from_camera(&cam);
        // Thin long rect rotated may leave view.
        let r = f.cull_oriented_rect(Vec2::new(0.0, 30.0), Vec2::new(100.0, 2.0), 0.0);
        assert_eq!(r, CullResult::Outside);
    }

    #[test]
    fn is_on_screen_padding() {
        let cam = Camera2D::virtual_res(100.0, 100.0);
        let edge = Aabb2::from_center_extents(Vec2::new(55.0, 0.0), Vec2::splat(2.0));
        assert!(!is_on_screen(&cam, edge, 0.0));
        assert!(is_on_screen(&cam, edge, 10.0));
    }
}
