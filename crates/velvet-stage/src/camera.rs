//! The camera of a stage: where the eye is, what it looks at, and how wide it sees.

use velvet_math::{Mat4, Vec3};

/// A perspective camera, right-handed, Y up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// The vertical field of view, in degrees.
    pub fov: f32,
    /// The nearest and farthest distances drawn.
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// A camera at `eye` looking at `target`, 45° high, seeing from 1 to 5000 units.
    pub fn new(eye: Vec3, target: Vec3) -> Camera {
        Camera { eye, target, up: Vec3::Y, fov: 45.0, near: 1.0, far: 5000.0 }
    }

    pub fn view(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye, self.target, self.up)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), aspect, self.near, self.far)
    }

    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Where the camera looks, as a unit vector.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.eye).normalize_or_zero()
    }

    /// Part of the way to another camera (eye, target and field of view).
    pub fn lerp(&self, other: &Camera, t: f32) -> Camera {
        Camera {
            eye: self.eye.lerp(other.eye, t),
            target: self.target.lerp(other.target, t),
            up: self.up.lerp(other.up, t).normalize_or_zero(),
            fov: self.fov + (other.fov - self.fov) * t,
            near: self.near + (other.near - self.near) * t,
            far: self.far + (other.far - self.far) * t,
        }
    }
}

/// A half-line from a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    /// A unit vector.
    pub direction: Vec3,
}

impl Ray {
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}
