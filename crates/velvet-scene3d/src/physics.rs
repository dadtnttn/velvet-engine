//! Minimal deterministic 3D physics: AABB colliders, raycasts, and a
//! capsule character controller.
//!
//! No external engine: colliders bake from [`crate::flatten::FlatNode3D`]
//! (primitives get analytic bounds; `.glb` nodes need authored `cube`
//! hitbox children), and the controller resolves overlaps along the
//! axis of least penetration (collide-and-slide). All stepping is pure
//! functions of `(world, dt)` — same inputs, same positions, suitable
//! for replays and tests.

use velvet_math::{Mat4, Vec3};

use crate::doc::MeshKind3D;
use crate::flatten::FlatNode3D;

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb3D {
    /// Minimum corner.
    pub min: Vec3,
    /// Maximum corner.
    pub max: Vec3,
}

impl Aabb3D {
    /// Build from corners (order-independent).
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self {
            min: Vec3::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: Vec3::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    /// Center point.
    pub fn center(self) -> Vec3 {
        Vec3::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
            (self.min.z + self.max.z) * 0.5,
        )
    }

    /// Full extents.
    pub fn size(self) -> Vec3 {
        Vec3::new(
            self.max.x - self.min.x,
            self.max.y - self.min.y,
            self.max.z - self.min.z,
        )
    }

    /// Closed-volume overlap (touching faces count as overlap).
    pub fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Point inside (boundary included).
    pub fn contains_point(self, p: Vec3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    /// Closest point on/in the box to `p`.
    pub fn closest_point(self, p: Vec3) -> Vec3 {
        Vec3::new(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
            p.z.clamp(self.min.z, self.max.z),
        )
    }

    /// Slab raycast. Returns entry distance `t` along `dir` when
    /// `0 <= t <= max_t` (origin inside counts as `t = 0`).
    pub fn ray_intersect(self, origin: Vec3, dir: Vec3, max_t: f32) -> Option<f32> {
        let mut t_min = 0.0f32;
        let mut t_max = max_t.max(0.0);
        for (o, d, lo, hi) in [
            (origin.x, dir.x, self.min.x, self.max.x),
            (origin.y, dir.y, self.min.y, self.max.y),
            (origin.z, dir.z, self.min.z, self.max.z),
        ] {
            if d.abs() < 1e-8 {
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let mut t1 = (lo - o) / d;
                let mut t2 = (hi - o) / d;
                if t1 > t2 {
                    std::mem::swap(&mut t1, &mut t2);
                }
                t_min = t_min.max(t1);
                t_max = t_max.min(t2);
                if t_min > t_max {
                    return None;
                }
            }
        }
        Some(t_min.max(0.0))
    }

    /// Expand uniformly in every direction.
    pub fn grown(self, amount: f32) -> Self {
        Self {
            min: Vec3::new(
                self.min.x - amount,
                self.min.y - amount,
                self.min.z - amount,
            ),
            max: Vec3::new(
                self.max.x + amount,
                self.max.y + amount,
                self.max.z + amount,
            ),
        }
    }
}

/// One baked static collider.
#[derive(Debug, Clone, PartialEq)]
pub struct Collider3D {
    /// Node path the collider came from.
    pub id: String,
    /// World-space bounds.
    pub bounds: Aabb3D,
}

/// Static collision world (gravity + collider list).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PhysicsWorld3D {
    /// Static colliders.
    pub colliders: Vec<Collider3D>,
    /// Downward acceleration (Y-up, positive number).
    pub gravity: f32,
}

impl PhysicsWorld3D {
    /// Empty world with Earth-like gravity.
    pub fn new() -> Self {
        Self {
            colliders: Vec::new(),
            gravity: 22.0,
        }
    }

    /// Add a collider.
    pub fn add_collider(&mut self, id: impl Into<String>, bounds: Aabb3D) {
        self.colliders.push(Collider3D {
            id: id.into(),
            bounds,
        });
    }
}

/// Bake colliders from flattened nodes.
///
/// Primitives get analytic world bounds (8 corners through the world
/// matrix, so rotated nodes stay correct). `Gltf` / `Empty` nodes and
/// nodes flagged `collider: false` (dynamic bodies) are skipped —
/// author static hitboxes as plain primitives instead.
pub fn bake_colliders(flat: &[FlatNode3D]) -> Vec<Collider3D> {
    let mut out = Vec::new();
    for node in flat {
        if !node.collider {
            continue;
        }
        let local = match &node.mesh {
            MeshKind3D::Cube => Some(Aabb3D::new(
                Vec3::new(-0.5, -0.5, -0.5),
                Vec3::new(0.5, 0.5, 0.5),
            )),
            MeshKind3D::Sphere { .. } => Some(Aabb3D::new(
                Vec3::new(-1.0, -1.0, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
            )),
            // Planes get a thin skin so bodies can land on them
            // (a zero-volume box collides with nothing).
            MeshKind3D::Plane => Some(Aabb3D::new(
                Vec3::new(-0.5, -0.02, -0.5),
                Vec3::new(0.5, 0.02, 0.5),
            )),
            MeshKind3D::Heightfield {
                resolution,
                size,
                heights,
            } => heightfield_bounds(*resolution, *size, heights),
            MeshKind3D::Gltf { .. } | MeshKind3D::Empty => None,
        };
        if let Some(local) = local {
            out.push(Collider3D {
                id: node.id.clone(),
                bounds: transform_aabb(node.world, local),
            });
        }
    }
    out
}

/// Local-space bounds of a heightfield (XZ extent, Y span), or None when
/// the grid is malformed (see `validate_scene`).
pub fn heightfield_bounds(resolution: u32, size: f32, heights: &[f32]) -> Option<Aabb3D> {
    if resolution < 2 || heights.len() != resolution as usize * resolution as usize {
        return None;
    }
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for h in heights {
        if !h.is_finite() {
            return None;
        }
        lo = lo.min(*h);
        hi = hi.max(*h);
    }
    let half = size * 0.5;
    Some(Aabb3D::new(
        Vec3::new(-half, lo, -half),
        Vec3::new(half, hi, half),
    ))
}

/// Bilinear heightfield sample in node-local space.
///
/// Returns None outside `±size/2` or for malformed grids. Hosts transform
/// the query point by the inverse node matrix first (identity in the
/// common unrotated case); walkers add their foot offset to the result.
pub fn sample_heightfield(
    resolution: u32,
    size: f32,
    heights: &[f32],
    x: f32,
    z: f32,
) -> Option<f32> {
    if resolution < 2 || heights.len() != resolution as usize * resolution as usize {
        return None;
    }
    let half = size * 0.5;
    if x < -half || x > half || z < -half || z > half {
        return None;
    }
    let res = resolution as usize;
    let fx = ((x + half) / size * (res - 1) as f32).clamp(0.0, (res - 1) as f32);
    let fz = ((z + half) / size * (res - 1) as f32).clamp(0.0, (res - 1) as f32);
    let (i0, j0) = (fx.floor() as usize, fz.floor() as usize);
    let (i1, j1) = ((i0 + 1).min(res - 1), (j0 + 1).min(res - 1));
    let (tx, tz) = (fx - i0 as f32, fz - j0 as f32);
    let a = heights[j0 * res + i0];
    let b = heights[j0 * res + i1];
    let c = heights[j1 * res + i0];
    let d = heights[j1 * res + i1];
    if !(a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite()) {
        return None;
    }
    Some(a * (1.0 - tx) * (1.0 - tz) + b * tx * (1.0 - tz) + c * (1.0 - tx) * tz + d * tx * tz)
}

/// World-space AABB of a local box through a matrix (8 corners).
pub fn transform_aabb(world: Mat4, local: Aabb3D) -> Aabb3D {
    let mut out: Option<Aabb3D> = None;
    for &x in &[local.min.x, local.max.x] {
        for &y in &[local.min.y, local.max.y] {
            for &z in &[local.min.z, local.max.z] {
                let p = transform_point(world, Vec3::new(x, y, z));
                out = Some(match out {
                    None => Aabb3D::new(p, p),
                    Some(b) => Aabb3D::new(
                        Vec3::new(b.min.x.min(p.x), b.min.y.min(p.y), b.min.z.min(p.z)),
                        Vec3::new(b.max.x.max(p.x), b.max.y.max(p.y), b.max.z.max(p.z)),
                    ),
                });
            }
        }
    }
    out.unwrap_or(local)
}

/// Transform a point by a column-major matrix (w = 1).
pub fn transform_point(world: Mat4, p: Vec3) -> Vec3 {
    Vec3::new(
        world.x_axis[0] * p.x + world.y_axis[0] * p.y + world.z_axis[0] * p.z + world.w_axis[0],
        world.x_axis[1] * p.x + world.y_axis[1] * p.y + world.z_axis[1] * p.z + world.w_axis[1],
        world.x_axis[2] * p.x + world.y_axis[2] * p.y + world.z_axis[2] * p.z + world.w_axis[2],
    )
}

/// One raycast hit.
#[derive(Debug, Clone, PartialEq)]
pub struct RayHit3D {
    /// Collider node path.
    pub id: String,
    /// Distance along the ray.
    pub t: f32,
    /// World hit point.
    pub point: Vec3,
}

/// Nearest ray hit in the world, if any.
pub fn raycast(world: &PhysicsWorld3D, origin: Vec3, dir: Vec3, max_t: f32) -> Option<RayHit3D> {
    let len = (dir.x * dir.x + dir.y * dir.y + dir.z * dir.z).sqrt();
    if len < 1e-8 || max_t <= 0.0 {
        return None;
    }
    let dir = Vec3::new(dir.x / len, dir.y / len, dir.z / len);
    let mut best: Option<RayHit3D> = None;
    for collider in &world.colliders {
        if let Some(t) = collider.bounds.ray_intersect(origin, dir, max_t) {
            let replace = match &best {
                None => true,
                Some(b) => t < b.t,
            };
            if replace {
                best = Some(RayHit3D {
                    id: collider.id.clone(),
                    t,
                    point: Vec3::new(
                        origin.x + dir.x * t,
                        origin.y + dir.y * t,
                        origin.z + dir.z * t,
                    ),
                });
            }
        }
    }
    best
}

/// Capsule character controller (feet position + AABB approximation).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterController3D {
    /// Feet position.
    pub position: Vec3,
    /// Current velocity (gravity accumulates into `y`).
    pub velocity: Vec3,
    /// Capsule radius.
    pub radius: f32,
    /// Capsule height.
    pub height: f32,
    /// Standing on something after the last step.
    pub grounded: bool,
}

impl CharacterController3D {
    /// New controller standing at `position` (feet).
    pub fn new(position: Vec3, radius: f32, height: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            radius: radius.max(0.05),
            height: height.max(0.2),
            grounded: false,
        }
    }

    /// Body bounds at the current position.
    pub fn bounds(&self) -> Aabb3D {
        Aabb3D::new(
            Vec3::new(
                self.position.x - self.radius,
                self.position.y,
                self.position.z - self.radius,
            ),
            Vec3::new(
                self.position.x + self.radius,
                self.position.y + self.height,
                self.position.z + self.radius,
            ),
        )
    }

    /// Maximum fall speed (prevents tunneling through thin floors).
    pub const MAX_FALL_SPEED: f32 = 30.0;

    /// Step: apply gravity, integrate, resolve overlaps.
    ///
    /// `wish` is the horizontal velocity for this step; `jump` sets the
    /// vertical velocity when grounded. `dt` is clamped to 1/20 max for
    /// stability (callers subdivide longer frames). Overlaps resolve
    /// along the axis of least penetration, so standing on the ground
    /// keeps grounding and walls slide instead of stick.
    pub fn step_move(&mut self, world: &PhysicsWorld3D, wish: Vec3, jump: Option<f32>, dt: f32) {
        let dt = dt.clamp(0.0, 0.05);
        self.velocity.x = wish.x;
        self.velocity.z = wish.z;
        self.velocity.y = (self.velocity.y - world.gravity * dt).max(-Self::MAX_FALL_SPEED);
        if let Some(v) = jump {
            if self.grounded {
                self.velocity.y = v.max(0.0);
                self.grounded = false;
            }
        }
        self.position.x += self.velocity.x * dt;
        self.position.y += self.velocity.y * dt;
        self.position.z += self.velocity.z * dt;
        self.grounded = false;
        self.resolve_all(world);
    }

    fn resolve_all(&mut self, world: &PhysicsWorld3D) {
        for collider in &world.colliders {
            let body = self.bounds();
            let b = collider.bounds;
            if !body.intersects(b) {
                continue;
            }
            let pen_x = (body.max.x.min(b.max.x) - body.min.x.max(b.min.x)).max(0.0);
            let pen_y = (body.max.y.min(b.max.y) - body.min.y.max(b.min.y)).max(0.0);
            let pen_z = (body.max.z.min(b.max.z) - body.min.z.max(b.min.z)).max(0.0);
            let cx = (body.min.x + body.max.x) * 0.5 - (b.min.x + b.max.x) * 0.5;
            let cy = (body.min.y + body.max.y) * 0.5 - (b.min.y + b.max.y) * 0.5;
            let cz = (body.min.z + body.max.z) * 0.5 - (b.min.z + b.max.z) * 0.5;
            if pen_x <= pen_y && pen_x <= pen_z {
                self.position.x += if cx < 0.0 { -pen_x } else { pen_x };
                self.velocity.x = 0.0;
            } else if pen_y <= pen_z {
                if cy < 0.0 {
                    // Head bump on the bottom face.
                    self.position.y -= pen_y;
                } else {
                    // Landed on the top face.
                    self.position.y += pen_y;
                    self.grounded = true;
                }
                self.velocity.y = 0.0;
            } else {
                self.position.z += if cz < 0.0 { -pen_z } else { pen_z };
                self.velocity.z = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{flatten_scene, load_scene_from_str};

    fn v(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn ground_world() -> PhysicsWorld3D {
        let mut world = PhysicsWorld3D::new();
        world.add_collider(
            "ground",
            Aabb3D::new(v(-10.0, -1.0, -10.0), v(10.0, 0.0, 10.0)),
        );
        world
    }

    #[test]
    fn aabb_overlap_and_point() {
        let a = Aabb3D::new(v(0.0, 0.0, 0.0), v(1.0, 1.0, 1.0));
        assert!(a.intersects(Aabb3D::new(v(0.5, 0.5, 0.5), v(2.0, 2.0, 2.0))));
        assert!(a.intersects(Aabb3D::new(v(1.0, 0.0, 0.0), v(2.0, 1.0, 1.0))));
        assert!(!a.intersects(Aabb3D::new(v(1.1, 0.0, 0.0), v(2.0, 1.0, 1.0))));
        assert!(a.contains_point(v(0.5, 0.5, 0.5)));
        assert!(!a.contains_point(v(-0.1, 0.5, 0.5)));
    }

    #[test]
    fn ray_hits_box_face() {
        let world = ground_world();
        let hit = raycast(&world, v(0.0, 5.0, 0.0), v(0.0, -1.0, 0.0), 10.0).unwrap();
        assert_eq!(hit.id, "ground");
        assert!((hit.t - 5.0).abs() < 1e-4);
        assert!((hit.point.y - 0.0).abs() < 1e-4);
        assert!(raycast(&world, v(0.0, 5.0, 0.0), v(0.0, 1.0, 0.0), 10.0).is_none());
        assert!(raycast(&world, v(0.0, 5.0, 0.0), v(0.0, -1.0, 0.0), 2.0).is_none());
    }

    #[test]
    fn controller_falls_and_grounds() {
        let world = ground_world();
        let mut body = CharacterController3D::new(v(0.0, 5.0, 0.0), 0.3, 1.7);
        for _ in 0..120 {
            body.step_move(&world, Vec3::ZERO, None, 1.0 / 60.0);
        }
        assert!(body.grounded);
        assert!((body.position.y - 0.0).abs() < 1e-3);
        assert_eq!(body.velocity.y, 0.0);
    }

    #[test]
    fn controller_slides_along_wall() {
        let mut world = ground_world();
        world.add_collider("wall", Aabb3D::new(v(2.0, 0.0, -10.0), v(3.0, 3.0, 10.0)));
        let mut body = CharacterController3D::new(v(0.0, 0.0, 0.0), 0.3, 1.7);
        body.grounded = true;
        // Diagonal wish into the wall: x stops at the face, z keeps moving.
        for _ in 0..60 {
            body.step_move(&world, v(5.0, 0.0, 2.0), None, 1.0 / 60.0);
        }
        assert!((body.position.x - 1.7).abs() < 1e-3);
        assert!(body.position.z > 1.0);
    }

    #[test]
    fn controller_jumps_and_lands() {
        let world = ground_world();
        let mut body = CharacterController3D::new(v(0.0, 0.0, 0.0), 0.3, 1.7);
        body.grounded = true;
        body.step_move(&world, Vec3::ZERO, Some(7.0), 1.0 / 60.0);
        assert!(!body.grounded);
        assert!(body.position.y > 0.0);
        for _ in 0..180 {
            body.step_move(&world, Vec3::ZERO, None, 1.0 / 60.0);
        }
        assert!(body.grounded);
        assert!((body.position.y - 0.0).abs() < 1e-3);
    }

    #[test]
    fn bake_colliders_from_scene() {
        let scene = load_scene_from_str(
            r#"{"nodes": [
                {"id": "ground", "mesh": {"kind": "plane"},
                 "transform": {"scale": [20, 1, 20]}},
                {"id": "hero", "mesh": {"kind": "gltf", "path": "m.glb"}},
                {"id": "hitbox", "mesh": {"kind": "cube"},
                 "transform": {"position": [1, 1, 0], "scale": [1, 2, 1]}}
            ]}"#,
        )
        .unwrap();
        let colliders = bake_colliders(&flatten_scene(&scene));
        // plane + authored cube hitbox; the glb itself contributes none.
        assert_eq!(colliders.len(), 2);
        let hitbox = colliders.iter().find(|c| c.id == "hitbox").unwrap();
        assert!((hitbox.bounds.min.y - 0.0).abs() < 1e-5);
        assert!((hitbox.bounds.max.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn physics_is_deterministic() {
        let world = ground_world();
        let run = || {
            let mut body = CharacterController3D::new(v(0.0, 3.0, 1.0), 0.3, 1.7);
            for i in 0..90 {
                let jump = if i == 10 { Some(6.0) } else { None };
                body.step_move(&world, v(2.0, 0.0, 0.0), jump, 1.0 / 60.0);
            }
            body.position
        };
        let first = run();
        let second = run();
        assert_eq!(first, second);
    }
}
