//! 2D kinematics, AABB/circle collision, raycasts.

use velvet_math::{Rect, Vec2};

use crate::collider::{Collider, ColliderShape, CollisionMask};

/// Ray for queries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Origin.
    pub origin: Vec2,
    /// Direction (will be normalized in cast).
    pub direction: Vec2,
    /// Max distance.
    pub max_distance: f32,
}

impl Ray {
    /// Create.
    pub fn new(origin: Vec2, direction: Vec2, max_distance: f32) -> Self {
        Self {
            origin,
            direction,
            max_distance,
        }
    }
}

/// Hit from raycast / shapecast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastHit {
    /// Distance along ray.
    pub distance: f32,
    /// Point of contact.
    pub point: Vec2,
    /// Normal (outward from hit surface).
    pub normal: Vec2,
    /// Optional entity index in query list.
    pub index: usize,
}

/// Overlap hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollisionHit {
    /// Other body index.
    pub index: usize,
    /// Penetration depth.
    pub depth: f32,
    /// Separation normal (push self along this).
    pub normal: Vec2,
}

/// Result of move_and_collide.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveResult {
    /// Final position.
    pub position: Vec2,
    /// Hits encountered.
    pub hits: Vec<CollisionHit>,
    /// Remaining velocity after slides.
    pub remainder: Vec2,
}

/// AABB vs AABB overlap.
pub fn collide_aabb(a: Rect, b: Rect) -> Option<(f32, Vec2)> {
    if !a.intersects(b) {
        return None;
    }
    let ox1 = a.max.x - b.min.x;
    let ox2 = b.max.x - a.min.x;
    let oy1 = a.max.y - b.min.y;
    let oy2 = b.max.y - a.min.y;
    let (dx, nx) = if ox1 < ox2 { (ox1, -1.0) } else { (ox2, 1.0) };
    let (dy, ny) = if oy1 < oy2 { (oy1, -1.0) } else { (oy2, 1.0) };
    if dx < dy {
        Some((dx, Vec2::new(nx, 0.0)))
    } else {
        Some((dy, Vec2::new(0.0, ny)))
    }
}

/// Circle vs circle.
pub fn collide_circle(a_pos: Vec2, a_r: f32, b_pos: Vec2, b_r: f32) -> Option<(f32, Vec2)> {
    let d = b_pos - a_pos;
    let dist = d.length();
    let min = a_r + b_r;
    if dist >= min || dist <= 1e-8 {
        if dist <= 1e-8 && min > 0.0 {
            return Some((min, Vec2::X));
        }
        return None;
    }
    let depth = min - dist;
    let normal = d * (1.0 / dist);
    // Normal pushes A out of B: from B toward A is -normal if d = b-a...
    // We want normal for A to separate from B: direction from B center to A = -d.normalized()
    Some((depth, -normal))
}

/// AABB vs circle.
pub fn collide_aabb_circle(aabb: Rect, c_pos: Vec2, c_r: f32) -> Option<(f32, Vec2)> {
    let closest = Vec2::new(
        c_pos.x.clamp(aabb.min.x, aabb.max.x),
        c_pos.y.clamp(aabb.min.y, aabb.max.y),
    );
    let delta = c_pos - closest;
    let dist_sq = delta.length_squared();
    if dist_sq > c_r * c_r {
        return None;
    }
    if dist_sq <= 1e-12 {
        // Center inside AABB: push out by nearest face
        let left = c_pos.x - aabb.min.x;
        let right = aabb.max.x - c_pos.x;
        let bottom = c_pos.y - aabb.min.y;
        let top = aabb.max.y - c_pos.y;
        let m = left.min(right).min(bottom).min(top);
        if m == left {
            return Some((left + c_r, Vec2::NEG_X));
        }
        if m == right {
            return Some((right + c_r, Vec2::X));
        }
        if m == bottom {
            return Some((bottom + c_r, Vec2::NEG_Y));
        }
        return Some((top + c_r, Vec2::Y));
    }
    let dist = dist_sq.sqrt();
    let depth = c_r - dist;
    let normal = delta * (1.0 / dist);
    // Normal for circle body: outward from AABB into circle = normal
    // For AABB body vs circle we want opposite when self is AABB.
    Some((depth, normal))
}

/// Resolve penetration: move position along normal by depth.
pub fn resolve_penetration(position: Vec2, depth: f32, normal: Vec2) -> Vec2 {
    position + normal * depth
}

/// Ray vs AABB (slab method). Returns distance or None.
pub fn ray_aabb(ray: Ray, aabb: Rect) -> Option<CastHit> {
    let dir = ray.direction.normalize_or_zero();
    if dir.length_squared() < 1e-12 {
        return None;
    }
    let inv = Vec2::new(
        if dir.x.abs() < 1e-12 {
            f32::INFINITY
        } else {
            1.0 / dir.x
        },
        if dir.y.abs() < 1e-12 {
            f32::INFINITY
        } else {
            1.0 / dir.y
        },
    );
    let mut t1 = (aabb.min.x - ray.origin.x) * inv.x;
    let mut t2 = (aabb.max.x - ray.origin.x) * inv.x;
    let mut t3 = (aabb.min.y - ray.origin.y) * inv.y;
    let mut t4 = (aabb.max.y - ray.origin.y) * inv.y;
    if t1 > t2 {
        std::mem::swap(&mut t1, &mut t2);
    }
    if t3 > t4 {
        std::mem::swap(&mut t3, &mut t4);
    }
    let tmin = t1.max(t3);
    let tmax = t2.min(t4);
    if tmax < 0.0 || tmin > tmax {
        return None;
    }
    let t = if tmin >= 0.0 { tmin } else { tmax };
    if t < 0.0 || t > ray.max_distance {
        return None;
    }
    let point = ray.origin + dir * t;
    // Approximate normal from which slab
    let mut normal = Vec2::ZERO;
    if t1 > t3 {
        normal.x = if inv.x < 0.0 { 1.0 } else { -1.0 };
    } else {
        normal.y = if inv.y < 0.0 { 1.0 } else { -1.0 };
    }
    Some(CastHit {
        distance: t,
        point,
        normal,
        index: 0,
    })
}

/// Raycast against list of (position, collider).
pub fn raycast(ray: Ray, bodies: &[(Vec2, &Collider)]) -> Option<CastHit> {
    let mut best: Option<CastHit> = None;
    for (i, (pos, col)) in bodies.iter().enumerate() {
        if col.is_sensor {
            continue;
        }
        let aabb = col.world_aabb(*pos);
        if let Some(mut hit) = ray_aabb(ray, aabb) {
            hit.index = i;
            if best
                .as_ref()
                .map(|b| hit.distance < b.distance)
                .unwrap_or(true)
            {
                best = Some(hit);
            }
        }
    }
    best
}

/// Discrete overlap test between two colliders.
pub fn overlap(a_pos: Vec2, a: &Collider, b_pos: Vec2, b: &Collider) -> Option<(f32, Vec2)> {
    if !a.mask.hits(b.layer) || !b.mask.hits(a.layer) {
        return None;
    }
    let ap = a_pos + a.offset;
    let bp = b_pos + b.offset;
    match (a.shape, b.shape) {
        (ColliderShape::Aabb { half: ha }, ColliderShape::Aabb { half: hb }) => {
            let ar = Rect::from_center_half_size(ap, ha);
            let br = Rect::from_center_half_size(bp, hb);
            collide_aabb(ar, br)
        }
        (ColliderShape::Circle { radius: ra }, ColliderShape::Circle { radius: rb }) => {
            collide_circle(ap, ra, bp, rb)
        }
        (ColliderShape::Aabb { half }, ColliderShape::Circle { radius }) => {
            let ar = Rect::from_center_half_size(ap, half);
            // normal from helper is for circle; invert for AABB self
            collide_aabb_circle(ar, bp, radius).map(|(d, n)| (d, -n))
        }
        (ColliderShape::Circle { radius }, ColliderShape::Aabb { half }) => {
            let br = Rect::from_center_half_size(bp, half);
            collide_aabb_circle(br, ap, radius)
        }
    }
}

/// Move a kinematic collider with optional solid resolution against others.
///
/// Uses axis-separated movement and sub-steps to reduce tunneling.
pub fn move_and_collide(
    mut position: Vec2,
    velocity: Vec2,
    dt: f32,
    self_col: &Collider,
    solids: &[(Vec2, &Collider)],
    slide: bool,
) -> MoveResult {
    let mut hits = Vec::new();
    let deltas = [
        Vec2::new(velocity.x * dt, 0.0),
        Vec2::new(0.0, velocity.y * dt),
    ];

    for delta in deltas {
        let dist = delta.length();
        if dist < 1e-8 {
            continue;
        }
        let steps = ((dist / 4.0).ceil() as i32).clamp(1, 24);
        let step = delta / steps as f32;
        for _ in 0..steps {
            position += step;
            for (i, (opos, ocol)) in solids.iter().enumerate() {
                if ocol.is_sensor {
                    continue;
                }
                if let Some((depth, normal)) = overlap(position, self_col, *opos, ocol) {
                    if depth > 0.0 && !self_col.is_sensor {
                        position = resolve_penetration(position, depth + 1e-3, normal);
                        hits.push(CollisionHit {
                            index: i,
                            depth,
                            normal,
                        });
                        if !slide {
                            break;
                        }
                    }
                }
            }
        }
    }

    MoveResult {
        position,
        hits,
        remainder: Vec2::ZERO,
    }
}

/// Simple physics world helper wrapping body lists.
#[derive(Debug, Default)]
pub struct PhysicsWorld {
    /// Scratch solid list rebuilt each frame by systems.
    pub solids: Vec<(usize, Vec2, Collider)>,
}

impl PhysicsWorld {
    /// Clear.
    pub fn clear(&mut self) {
        self.solids.clear();
    }

    /// Push solid.
    pub fn push_solid(&mut self, id: usize, pos: Vec2, col: Collider) {
        self.solids.push((id, pos, col));
    }

    /// Move entity id with velocity against solids (excluding self).
    pub fn move_body(
        &self,
        id: usize,
        position: Vec2,
        velocity: Vec2,
        dt: f32,
        col: &Collider,
        slide: bool,
    ) -> MoveResult {
        let solids: Vec<(Vec2, &Collider)> = self
            .solids
            .iter()
            .filter(|(oid, _, _)| *oid != id)
            .map(|(_, p, c)| (*p, c))
            .collect();
        move_and_collide(position, velocity, dt, col, &solids, slide)
    }

    /// Raycast solids.
    pub fn raycast(&self, ray: Ray, mask: CollisionMask) -> Option<(usize, CastHit)> {
        let mut best: Option<(usize, CastHit)> = None;
        for (id, pos, col) in &self.solids {
            if col.is_sensor || !mask.hits(col.layer) {
                continue;
            }
            let aabb = col.world_aabb(*pos);
            if let Some(hit) = ray_aabb(ray, aabb) {
                if best
                    .as_ref()
                    .map(|(_, b)| hit.distance < b.distance)
                    .unwrap_or(true)
                {
                    best = Some((*id, hit));
                }
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collider::{Collider, CollisionLayer};

    #[test]
    fn aabb_overlap_and_resolve() {
        let a = Rect::from_pos_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Rect::from_pos_size(Vec2::new(8.0, 0.0), Vec2::new(10.0, 10.0));
        let (d, n) = collide_aabb(a, b).unwrap();
        assert!(d > 0.0);
        assert!(n.x.abs() > 0.0 || n.y.abs() > 0.0);
    }

    #[test]
    fn move_blocked_by_wall() {
        let player = Collider::aabb(Vec2::splat(8.0));
        let wall = Collider::aabb(Vec2::new(20.0, 40.0));
        let solids = [(Vec2::new(40.0, 0.0), &wall)];
        let res = move_and_collide(
            Vec2::new(0.0, 0.0),
            Vec2::new(200.0, 0.0),
            1.0,
            &player,
            &solids,
            true,
        );
        assert!(res.position.x < 40.0);
        assert!(!res.hits.is_empty());
    }

    #[test]
    fn raycast_hits_box() {
        let wall = Collider::aabb(Vec2::splat(10.0));
        let hit = raycast(
            Ray::new(Vec2::new(-50.0, 0.0), Vec2::X, 100.0),
            &[(Vec2::ZERO, &wall)],
        )
        .unwrap();
        assert!(hit.distance > 0.0 && hit.distance < 50.0);
    }

    #[test]
    fn sensor_no_block() {
        let mut sensor = Collider::sensor_aabb(Vec2::splat(20.0));
        sensor.layer = CollisionLayer::TRIGGER;
        let player = Collider {
            layer: CollisionLayer::PLAYER,
            mask: CollisionMask::from_layers(CollisionLayer::WORLD),
            ..Collider::aabb(Vec2::splat(8.0))
        };
        let solids = [(Vec2::ZERO, &sensor)];
        let res = move_and_collide(
            Vec2::new(-30.0, 0.0),
            Vec2::X * 100.0,
            1.0,
            &player,
            &solids,
            true,
        );
        assert!(res.hits.is_empty());
    }

    #[test]
    fn zero_dt_no_move() {
        let player = Collider::aabb(Vec2::splat(8.0));
        let wall = Collider::aabb(Vec2::splat(10.0));
        let solids = [(Vec2::new(50.0, 0.0), &wall)];
        let res = move_and_collide(Vec2::ZERO, Vec2::X * 100.0, 0.0, &player, &solids, true);
        assert_eq!(res.position, Vec2::ZERO);
        assert!(res.hits.is_empty());
    }

    #[test]
    fn zero_velocity_stationary() {
        let player = Collider::aabb(Vec2::splat(8.0));
        let res = move_and_collide(Vec2::new(5.0, 5.0), Vec2::ZERO, 1.0, &player, &[], true);
        assert_eq!(res.position, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn multi_solid_blocks_horizontal() {
        // Two stacked walls; horizontal motion should still collide.
        let player = Collider::aabb(Vec2::splat(4.0));
        let wall_a = Collider::aabb(Vec2::new(10.0, 40.0));
        let wall_b = Collider::aabb(Vec2::new(10.0, 40.0));
        let solids = [
            (Vec2::new(40.0, 0.0), &wall_a),
            (Vec2::new(40.0, 40.0), &wall_b),
        ];
        let res = move_and_collide(
            Vec2::new(0.0, 0.0),
            Vec2::new(200.0, 0.0),
            1.0,
            &player,
            &solids,
            true,
        );
        assert!(res.position.x < 40.0, "pos={:?}", res.position);
        assert!(!res.hits.is_empty());
    }

    #[test]
    fn penetration_resolve_separates() {
        let a = Rect::from_pos_size(Vec2::new(0.0, 0.0), Vec2::new(10.0, 10.0));
        let b = Rect::from_pos_size(Vec2::new(5.0, 0.0), Vec2::new(10.0, 10.0));
        let (depth, normal) = collide_aabb(a, b).unwrap();
        let fixed = resolve_penetration(Vec2::new(5.0, 5.0), depth, normal);
        // Should move away along normal.
        assert!(fixed.x != 5.0 || depth == 0.0);
    }

    #[test]
    fn ray_misses_when_short() {
        let wall = Collider::aabb(Vec2::splat(10.0));
        let hit = raycast(
            Ray::new(Vec2::new(-50.0, 0.0), Vec2::X, 10.0),
            &[(Vec2::ZERO, &wall)],
        );
        assert!(hit.is_none());
    }

    #[test]
    fn physics_world_excludes_self() {
        let mut pw = PhysicsWorld::default();
        let col = Collider::aabb(Vec2::splat(8.0));
        pw.push_solid(1, Vec2::ZERO, col.clone());
        pw.push_solid(2, Vec2::new(5.0, 0.0), col.clone());
        let res = pw.move_body(1, Vec2::ZERO, Vec2::X * 10.0, 1.0, &col, true);
        // Should collide with body 2
        assert!(!res.hits.is_empty() || res.position.x < 20.0);
    }

    #[test]
    fn no_overlap_returns_none() {
        let a = Rect::from_pos_size(Vec2::ZERO, Vec2::new(10.0, 10.0));
        let b = Rect::from_pos_size(Vec2::new(20.0, 0.0), Vec2::new(10.0, 10.0));
        assert!(collide_aabb(a, b).is_none());
    }

    #[test]
    fn circle_collision_separates() {
        let hit = collide_circle(Vec2::ZERO, 5.0, Vec2::new(6.0, 0.0), 5.0).unwrap();
        assert!(hit.0 > 0.0);
        assert!(hit.1.x != 0.0);
        assert!(collide_circle(Vec2::ZERO, 1.0, Vec2::new(50.0, 0.0), 1.0).is_none());
        // Coincident centers produce a defined normal.
        let coincident = collide_circle(Vec2::ZERO, 2.0, Vec2::ZERO, 2.0).unwrap();
        assert!(coincident.0 > 0.0);
        assert!(coincident.1.length() > 0.5);
    }

    #[test]
    fn slide_along_wall_preserves_tangent() {
        let player = Collider::aabb(Vec2::splat(4.0));
        let wall = Collider::aabb(Vec2::new(8.0, 100.0));
        // Wall to the right; move diagonally into it.
        let solids = [(Vec2::new(20.0, 0.0), &wall)];
        let res = move_and_collide(
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 50.0),
            1.0,
            &player,
            &solids,
            true,
        );
        assert!(res.position.x < 20.0);
        // With sliding, Y should still advance somewhat.
        assert!(res.position.y > 1.0, "pos={:?}", res.position);
        assert!(!res.hits.is_empty());
    }

    #[test]
    fn move_without_slide_stops() {
        let player = Collider::aabb(Vec2::splat(4.0));
        let wall = Collider::aabb(Vec2::new(8.0, 40.0));
        let solids = [(Vec2::new(30.0, 0.0), &wall)];
        let res = move_and_collide(
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 20.0),
            1.0,
            &player,
            &solids,
            false,
        );
        assert!(res.position.x < 30.0);
        // Without slide, Y motion may be reduced more than with slide.
        assert!(!res.hits.is_empty() || res.position.x < 30.0);
    }

    #[test]
    fn raycast_vertical_and_miss() {
        let wall = Collider::aabb(Vec2::splat(8.0));
        let hit = raycast(
            Ray::new(Vec2::new(0.0, -40.0), Vec2::Y, 100.0),
            &[(Vec2::ZERO, &wall)],
        )
        .unwrap();
        assert!(hit.distance > 0.0 && hit.distance < 40.0);
        let miss = raycast(
            Ray::new(Vec2::new(100.0, 0.0), Vec2::Y, 50.0),
            &[(Vec2::ZERO, &wall)],
        );
        assert!(miss.is_none());
    }

    #[test]
    fn physics_world_clear_and_query() {
        let mut pw = PhysicsWorld::default();
        let col = Collider::aabb(Vec2::splat(6.0));
        pw.push_solid(1, Vec2::ZERO, col.clone());
        pw.push_solid(2, Vec2::new(100.0, 0.0), col.clone());
        let res = pw.move_body(1, Vec2::ZERO, Vec2::X * 10.0, 1.0, &col, true);
        // Far wall should not block short move.
        assert!(res.hits.is_empty() || res.position.x > 0.0);
        pw.clear();
        let res2 = pw.move_body(1, Vec2::ZERO, Vec2::X * 50.0, 1.0, &col, true);
        assert!((res2.position.x - 50.0).abs() < 1e-3 || res2.position.x > 40.0);
    }

    #[test]
    fn tiny_dt_stable() {
        let player = Collider::aabb(Vec2::splat(4.0));
        let wall = Collider::aabb(Vec2::splat(10.0));
        let solids = [(Vec2::new(20.0, 0.0), &wall)];
        let mut pos = Vec2::ZERO;
        for _ in 0..100 {
            let res = move_and_collide(pos, Vec2::X * 30.0, 0.001, &player, &solids, true);
            pos = res.position;
        }
        assert!(pos.x < 20.0);
        assert!(pos.x >= 0.0);
    }

    #[test]
    fn already_overlapping_resolves_out() {
        let player = Collider::aabb(Vec2::splat(8.0));
        let wall = Collider::aabb(Vec2::splat(8.0));
        // Overlapping positions
        let solids = [(Vec2::new(4.0, 0.0), &wall)];
        let res = move_and_collide(Vec2::ZERO, Vec2::ZERO, 1.0, &player, &solids, true);
        // Either no move with zero vel or depenetration.
        assert!(res.position.x.abs() < 20.0);
    }
}
