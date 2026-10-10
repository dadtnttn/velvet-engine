//! Grounded kinematic capsule. Collision and rendering share the same surfaces;
//! substeps cap tunnelling and eliminate frame-rate dependent stair failures.
use super::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub sprint: bool,
    pub jump: bool,
}
#[derive(Clone, Debug)]
pub struct Controller {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    pub distance: f32,
}
impl Controller {
    pub fn new(p: Vec3) -> Self {
        Self {
            position: p,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            grounded: true,
            distance: 0.0,
        }
    }
    pub fn look(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.0022;
        self.pitch = (self.pitch - dy * 0.0022).clamp(-1.48, 1.48);
    }
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }
    pub fn eye(&self) -> Vec3 {
        self.position + Vec3::new(0.0, 1.62, 0.0)
    }
    pub fn step(&mut self, map: &Blueprint, input: Input, dt: f32) {
        let dt = dt.clamp(0.0, 0.1);
        let steps = (dt / 0.008).ceil().max(1.0) as usize;
        let h = dt / steps as f32;
        let forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let desired = (forward * input.forward + right * input.strafe).normalize_or_zero()
            * if input.sprint { 5.0 } else { 3.1 };
        if input.jump && self.grounded {
            self.velocity.y = 5.4;
            self.grounded = false;
        }
        for _ in 0..steps {
            let blend = (h * if self.grounded { 18.0 } else { 4.0 }).min(1.0);
            self.velocity.x += (desired.x - self.velocity.x) * blend;
            self.velocity.z += (desired.z - self.velocity.z) * blend;
            let before = self.position;
            for axis in 0..2 {
                let mut p = self.position;
                if axis == 0 {
                    p.x += self.velocity.x * h;
                } else {
                    p.z += self.velocity.z * h;
                }
                if let Some(floor) = map.capsule_floor(p, PLAYER_RADIUS) {
                    if floor <= self.position.y + STEP_HEIGHT + 0.015 {
                        self.position = p;
                        if self.grounded && floor >= p.y - 0.35 {
                            self.position.y = floor;
                        }
                    }
                } else if axis == 0 {
                    self.velocity.x = 0.0;
                } else {
                    self.velocity.z = 0.0;
                }
            }
            let floor = map
                .capsule_floor(self.position, PLAYER_RADIUS)
                .unwrap_or(self.position.y);
            self.velocity.y -= 18.0 * h;
            let next = self.position.y + self.velocity.y * h;
            if next <= floor {
                self.position.y = floor;
                self.velocity.y = 0.0;
                self.grounded = true;
            } else {
                let raised = Vec3::new(self.position.x, next, self.position.z);
                if self.velocity.y > 0.0 && map.capsule_floor(raised, PLAYER_RADIUS).is_none() {
                    self.velocity.y = 0.0;
                } else {
                    self.position.y = next;
                }
                self.grounded = false;
            }
            self.distance += horizontal(self.position - before).length();
        }
    }
}

/// Controller-driven traversal, not teleportation or graph-only reachability.
/// Uses exactly the same acceleration, capsule, gravity and step-up as the game.
pub fn traverse(map: &Blueprint, points: &[Vec3]) -> Result<usize, String> {
    if points.is_empty() {
        return Err("empty route".into());
    }
    let mut c = Controller::new(points[0]);
    let mut frames = 0;
    for target in points.iter().skip(1) {
        let max = (((horizontal(*target - c.position).length() + 4.0) / 3.1) * 120.0) as usize;
        let mut arrived = false;
        for _ in 0..max {
            let delta = horizontal(*target - c.position);
            if delta.length() < 0.14 && (target.y - c.position.y).abs() < 0.35 {
                arrived = true;
                break;
            }
            c.yaw = (-delta.x).atan2(-delta.z);
            c.step(
                map,
                Input {
                    forward: if delta.length() < 0.5 { 0.35 } else { 1.0 },
                    ..Input::default()
                },
                1.0 / 60.0,
            );
            frames += 1;
        }
        if !arrived {
            return Err(format!(
                "capsule stuck at {:?}, target {:?}",
                c.position, target
            ));
        }
    }
    Ok(frames)
}
