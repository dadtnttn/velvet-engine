//! Rigid-body physics from scratch — AABB bodies colliding with the cellular grid.
//!
//! No external physics engine. Discrete Euler integration + grid occupancy tests.

use serde::{Deserialize, Serialize};

use crate::cell::MaterialId;
use crate::events::SimEvent;
use crate::material::Phase;
use crate::world::World;

/// Axis-aligned rigid body in world cell space (float positions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RigidBody {
    /// Stable id.
    pub id: u32,
    /// Center X (world cells).
    pub x: f32,
    /// Center Y.
    pub y: f32,
    /// Half-width.
    pub hw: f32,
    /// Half-height.
    pub hh: f32,
    /// Velocity X.
    pub vx: f32,
    /// Velocity Y.
    pub vy: f32,
    /// Angular velocity (unused for AABB but reserved).
    pub omega: f32,
    /// Mass.
    pub mass: f32,
    /// Restitution 0..=1.
    pub restitution: f32,
    /// Friction.
    pub friction: f32,
    /// Static (immovable).
    pub is_static: bool,
    /// Sensor (no collision response).
    pub is_sensor: bool,
    /// Enabled.
    pub enabled: bool,
}

impl RigidBody {
    /// Dynamic box.
    pub fn dynamic(id: u32, x: f32, y: f32, w: f32, h: f32, mass: f32) -> Self {
        Self {
            id,
            x,
            y,
            hw: w * 0.5,
            hh: h * 0.5,
            vx: 0.0,
            vy: 0.0,
            omega: 0.0,
            mass: mass.max(0.001),
            restitution: 0.1,
            friction: 0.4,
            is_static: false,
            is_sensor: false,
            enabled: true,
        }
    }

    /// Static obstacle.
    pub fn static_box(id: u32, x: f32, y: f32, w: f32, h: f32) -> Self {
        let mut b = Self::dynamic(id, x, y, w, h, 1e9);
        b.is_static = true;
        b.vx = 0.0;
        b.vy = 0.0;
        b
    }

    /// AABB min/max.
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        (
            self.x - self.hw,
            self.y - self.hh,
            self.x + self.hw,
            self.y + self.hh,
        )
    }
}

/// World of rigid bodies coupled to the cellular grid.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhysicsWorld {
    /// Bodies.
    pub bodies: Vec<RigidBody>,
    /// Gravity Y (cells/s²), typically negative.
    pub gravity_y: f32,
    /// Substeps per sim step.
    pub substeps: u32,
    /// Next id.
    next_id: u32,
}

impl PhysicsWorld {
    /// Create.
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            gravity_y: -40.0,
            substeps: 4,
            next_id: 1,
        }
    }

    /// Spawn dynamic body; returns id.
    pub fn spawn_dynamic(&mut self, x: f32, y: f32, w: f32, h: f32, mass: f32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.push(RigidBody::dynamic(id, x, y, w, h, mass));
        id
    }

    /// Spawn static.
    pub fn spawn_static(&mut self, x: f32, y: f32, w: f32, h: f32) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.bodies.push(RigidBody::static_box(id, x, y, w, h));
        id
    }

    /// Get body mut.
    pub fn get_mut(&mut self, id: u32) -> Option<&mut RigidBody> {
        self.bodies.iter_mut().find(|b| b.id == id)
    }

    /// Get body.
    pub fn get(&self, id: u32) -> Option<&RigidBody> {
        self.bodies.iter().find(|b| b.id == id)
    }

    /// Remove body.
    pub fn remove(&mut self, id: u32) {
        self.bodies.retain(|b| b.id != id);
    }

    /// Apply impulse.
    pub fn apply_impulse(&mut self, id: u32, ix: f32, iy: f32) {
        if let Some(b) = self.get_mut(id) {
            if b.is_static {
                return;
            }
            b.vx += ix / b.mass;
            b.vy += iy / b.mass;
        }
    }

    /// Step rigid bodies against cellular solid grid.
    pub fn step(&mut self, world: &mut World, dt: f32) {
        let n = self.substeps.max(1);
        let h = dt / n as f32;
        for _ in 0..n {
            self.substep(world, h);
        }
        // body-body resolve (simple pairwise)
        self.resolve_body_pairs();
    }

    fn substep(&mut self, world: &mut World, dt: f32) {
        let g = self.gravity_y;
        // collect updates without holding dual borrows
        let mut contacts: Vec<(u32, f32)> = Vec::new();
        for body in self.bodies.iter_mut() {
            if !body.enabled || body.is_static {
                continue;
            }
            body.vy += g * dt;
            body.x += body.vx * dt;
            body.y += body.vy * dt;

            // collide with solid cells
            if let Some(speed) = resolve_grid(body, world) {
                contacts.push((body.id, speed));
            }
        }
        for (id, speed) in contacts {
            world
                .events
                .push(SimEvent::BodyContact { body_id: id, speed });
        }
    }

    fn resolve_body_pairs(&mut self) {
        let n = self.bodies.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (left, right) = self.bodies.split_at_mut(j);
                let a = &mut left[i];
                let b = &mut right[0];
                if !a.enabled || !b.enabled {
                    continue;
                }
                if a.is_sensor || b.is_sensor {
                    continue;
                }
                let (ax0, ay0, ax1, ay1) = a.bounds();
                let (bx0, by0, bx1, by1) = b.bounds();
                let overlap_x = (ax1.min(bx1) - ax0.max(bx0)).max(0.0);
                let overlap_y = (ay1.min(by1) - ay0.max(by0)).max(0.0);
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }
                // separate along smaller axis
                if overlap_x < overlap_y {
                    let push = overlap_x * 0.5;
                    let dir = if a.x < b.x { -1.0 } else { 1.0 };
                    if !a.is_static {
                        a.x += dir * push;
                        a.vx *= -a.restitution;
                    }
                    if !b.is_static {
                        b.x -= dir * push;
                        b.vx *= -b.restitution;
                    }
                } else {
                    let push = overlap_y * 0.5;
                    let dir = if a.y < b.y { -1.0 } else { 1.0 };
                    if !a.is_static {
                        a.y += dir * push;
                        a.vy *= -a.restitution;
                    }
                    if !b.is_static {
                        b.y -= dir * push;
                        b.vy *= -b.restitution;
                    }
                }
            }
        }
    }
}

fn is_solid_cell(world: &World, x: i32, y: i32) -> bool {
    let c = world.get(x, y);
    if c.is_air() {
        return false;
    }
    matches!(
        world.materials.phase(c.material),
        Phase::Solid | Phase::Static | Phase::Powder
    )
}

/// Resolve body against solid grid cells; returns impact speed if hit.
fn resolve_grid(body: &mut RigidBody, world: &World) -> Option<f32> {
    let (x0, y0, x1, y1) = body.bounds();
    let min_ix = x0.floor() as i32 - 1;
    let max_ix = x1.ceil() as i32 + 1;
    let min_iy = y0.floor() as i32 - 1;
    let max_iy = y1.ceil() as i32 + 1;
    let mut hit_speed = 0.0f32;
    let mut hit = false;

    for iy in min_iy..=max_iy {
        for ix in min_ix..=max_ix {
            if !is_solid_cell(world, ix, iy) {
                continue;
            }
            // cell AABB [ix,ix+1] x [iy,iy+1]
            let cx0 = ix as f32;
            let cy0 = iy as f32;
            let cx1 = cx0 + 1.0;
            let cy1 = cy0 + 1.0;
            let (bx0, by0, bx1, by1) = body.bounds();
            let ox = (bx1.min(cx1) - bx0.max(cx0)).max(0.0);
            let oy = (by1.min(cy1) - by0.max(cy0)).max(0.0);
            if ox <= 0.0 || oy <= 0.0 {
                continue;
            }
            hit = true;
            let speed = (body.vx * body.vx + body.vy * body.vy).sqrt();
            hit_speed = hit_speed.max(speed);
            if ox < oy {
                let dir = if body.x < (cx0 + cx1) * 0.5 {
                    -1.0
                } else {
                    1.0
                };
                body.x += dir * ox;
                body.vx *= -body.restitution;
                body.vy *= 1.0 - body.friction * 0.1;
            } else {
                let dir = if body.y < (cy0 + cy1) * 0.5 {
                    -1.0
                } else {
                    1.0
                };
                body.y += dir * oy;
                body.vy *= -body.restitution;
                body.vx *= 1.0 - body.friction * 0.15;
                // ground stick
                if dir < 0.0 && body.vy.abs() < 0.5 {
                    body.vy = 0.0;
                }
            }
        }
    }
    if hit {
        Some(hit_speed)
    } else {
        None
    }
}

/// Dig / excavate: destroy solid cells under body footprint (player tool).
pub fn excavate_under_body(
    world: &mut World,
    body: &RigidBody,
    material_filter: Option<MaterialId>,
) {
    let (x0, y0, x1, y1) = body.bounds();
    for iy in (y0.floor() as i32)..=(y1.ceil() as i32) {
        for ix in (x0.floor() as i32)..=(x1.ceil() as i32) {
            let c = world.get(ix, iy);
            if c.is_air() {
                continue;
            }
            if let Some(f) = material_filter {
                if c.material != f {
                    continue;
                }
            }
            if matches!(
                world.materials.phase(c.material),
                Phase::Solid | Phase::Powder | Phase::Static
            ) {
                world.set(ix, iy, crate::cell::Cell::air());
            }
        }
    }
}
