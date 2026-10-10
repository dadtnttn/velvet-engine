//! Chiquita's hair, swinging. Her four chains of hair bones (two down her back, one in front of each
//! shoulder, each ending in a tip bone) follow her clip like weights on springs: every joint pulled
//! toward where the clip puts it, damped, the bones' lengths kept, kept out of her head, her torso
//! and her arms. So the hair lags behind what she does, swings when she turns, streams when she
//! leaps at the screen, and settles after. A breath of air keeps it from ever being quite still.
//!
//! It runs in her model's space. Her moves across the screen and her turns (the game's, not the
//! clip's) come in as the frame's own acceleration: what makes hair trail behind a body that moves.
//! The result is written back into the pose as the chain bones' rotations.
use velvet_math::{Mat4, Vec3};
use velvet_render::{GltfModel, Pose3D};

use super::super::retarget::{mat_rotation, q_between, q_conj, q_mul, q_norm, Quat};

/// One chain: its joints' names (root first, the tip last), how hard it is pulled to the clip, how
/// much it is damped, and how far (radians) a bone may swing from where the clip has it.
struct Chain {
    names: Vec<String>,
    stiff: f32,
    damp: f32,
    cone: f32,
    pos: Vec<Vec3>,
    prev: Vec<Vec3>,
}

pub(super) struct HairSim {
    chains: Vec<Chain>,
    /// The skull's centre in the head bone's frame (the head's collider).
    skull: Vec3,
    ready: bool,
    last_clock: u32,
    /// Where she stood on the screen (feet, height in px, yaw), and how fast the frame was moving
    /// (model units/s, radians/s).
    last_place: Option<([f32; 2], f32, f32)>,
    velocity: Vec3,
    spin: f32,
}

fn pos(m: Mat4) -> Vec3 {
    Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2])
}

fn arr(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

/// The point of segment a–b nearest to p.
fn nearest(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    a + ab * t
}

/// p pushed out of the capsule a–b of radius r.
fn push_out(p: Vec3, a: Vec3, b: Vec3, r: f32) -> Vec3 {
    let c = nearest(p, a, b);
    let d = p - c;
    let l = d.length();
    if l >= r {
        return p;
    }
    if l < 1e-6 {
        return c + Vec3::new(0.0, 0.0, -r);
    }
    c + d * (r / l)
}

impl HairSim {
    /// The chains her model has (`None` when it has no hair rig).
    pub(super) fn new(model: &GltfModel) -> Option<Self> {
        let nodes = model.rest_nodes();
        let has = |n: &str| nodes.iter().any(|x| x.0 == n);
        let mut chains = Vec::new();
        for (prefix, count, stiff, damp, cone) in [
            ("Hair_BL", 4, 60.0, 4.6, 1.0),
            ("Hair_BR", 4, 64.0, 5.0, 1.0),
            ("Hair_SL", 3, 150.0, 9.0, 0.55),
            ("Hair_SR", 3, 140.0, 8.6, 0.55),
        ] {
            let names: Vec<String> = (0..count).map(|k| format!("{prefix}{k}")).collect();
            if names.iter().all(|n| has(n)) {
                chains.push(Chain {
                    names,
                    stiff,
                    damp,
                    cone,
                    pos: Vec::new(),
                    prev: Vec::new(),
                });
            }
        }
        if chains.is_empty() {
            return None;
        }
        let rest = model.rest_pose();
        let head = model.pose_node_transform(&rest, "CC_Base_Head")?;
        // The skull's centre (measured on her mesh; the model is y-up).
        let skull = head
            .inverse()?
            .transform_point3(Vec3::new(0.0, 0.7811, 0.0144));
        Some(Self {
            chains,
            skull,
            ready: false,
            last_clock: 0,
            last_place: None,
            velocity: Vec3::new(0.0, 0.0, 0.0),
            spin: 0.0,
        })
    }

    /// Steps the hair to `clock_ms` (the game's clock) for `pose`, and writes the chain bones'
    /// rotations into it. `place`: her feet on the screen, her height there (px) and her yaw.
    pub(super) fn step(
        &mut self,
        model: &GltfModel,
        pose: &mut Pose3D,
        clock_ms: u32,
        place: ([f32; 2], f32, f32),
    ) {
        let at = |pose: &Pose3D, n: &str| model.pose_node_transform(pose, n);
        // Where the clip puts every joint, and the bones' turns, before anything is changed.
        let mut targets: Vec<Vec<Vec3>> = Vec::with_capacity(self.chains.len());
        let mut turns: Vec<Vec<Quat>> = Vec::with_capacity(self.chains.len());
        for chain in &self.chains {
            let mut t = Vec::with_capacity(chain.names.len());
            let mut r = Vec::with_capacity(chain.names.len());
            for name in &chain.names {
                let Some(m) = at(pose, name) else { return };
                t.push(pos(m));
                r.push(mat_rotation(m));
            }
            targets.push(t);
            turns.push(r);
        }
        let Some(head) = at(pose, "CC_Base_Head") else {
            return;
        };
        let head_turn = mat_rotation(head);
        let elapsed = clock_ms.wrapping_sub(self.last_clock) as f32 / 1000.0;
        self.last_clock = clock_ms;
        if !self.ready || !(0.0..0.25).contains(&elapsed) {
            for (chain, t) in self.chains.iter_mut().zip(&targets) {
                chain.pos = t.clone();
                chain.prev = t.clone();
            }
            self.ready = true;
            self.last_place = Some(place);
            self.velocity = Vec3::new(0.0, 0.0, 0.0);
            self.spin = 0.0;
            return;
        }
        if elapsed <= 0.0 {
            self.write(model, pose, &targets, &turns, head_turn);
            return;
        }

        // The frame's own motion: her moves across the screen (in her model's units: her height in
        // px is one unit) and her turns, as velocities; their change is what the hair feels.
        let (feet, height, yaw) = place;
        let (last_feet, last_height, last_yaw) = self.last_place.unwrap_or(place);
        self.last_place = Some(place);
        let h = height.max(1.0).min(last_height.max(1.0));
        let shift_world = Vec3::new(
            (feet[0] - last_feet[0]) / h,
            -(feet[1] - last_feet[1]) / h,
            0.0,
        );
        // Into her model's frame (turned by her yaw).
        let (s, c) = yaw.sin_cos();
        let shift = Vec3::new(shift_world.x * c, shift_world.y, shift_world.x * s);
        // (Her moves on the screen are not all moves in space: she is felt a third as hard.)
        let velocity = shift * (1.0 / elapsed);
        let spin = (yaw - last_yaw) / elapsed;
        let mut accel = (velocity - self.velocity) * (0.35 / elapsed);
        let alpha = ((spin - self.spin) / elapsed).clamp(-50.0, 50.0);
        self.velocity = velocity;
        self.spin = spin;
        let a = accel.length();
        if a > 14.0 {
            accel = accel * (14.0 / a);
        }

        // Her colliders as the clip has her.
        let skull = head.transform_point3(self.skull);
        let bone = |n: &str| at(pose, n).map(pos);
        let torso = (bone("CC_Base_Spine01"), bone("CC_Base_NeckTwist01"));
        let arms: Vec<(Vec3, Vec3, f32)> = ["L", "R"]
            .iter()
            .flat_map(|s| {
                let u = bone(&format!("CC_Base_{s}_Upperarm"));
                let f = bone(&format!("CC_Base_{s}_Forearm"));
                let hd = bone(&format!("CC_Base_{s}_Hand"));
                [(u, f, 0.032), (f, hd, 0.03)]
            })
            .filter_map(|(a, b, r)| Some((a?, b?, r)))
            .collect();

        let steps = ((elapsed / (1.0 / 120.0)).ceil() as usize).clamp(1, 12);
        let dt = elapsed / steps as f32;
        let t_secs = clock_ms as f32 / 1000.0;
        for (ci, (chain, t)) in self.chains.iter_mut().zip(&targets).enumerate() {
            for _ in 0..steps {
                chain.pos[0] = t[0];
                chain.prev[0] = t[0];
                for k in 1..chain.pos.len() {
                    let p = chain.pos[k];
                    let v = (p - chain.prev[k]) * (1.0 - chain.damp * dt).max(0.0);
                    // Her frame accelerating, and turning: the Euler force about her height, and the
                    // spin flinging the hair out.
                    let euler = Vec3::new(-alpha * p.z, 0.0, alpha * p.x) * 0.7;
                    let fling = Vec3::new(p.x, 0.0, p.z) * (spin * spin * 0.3);
                    let air = Vec3::new(
                        1.1 * (t_secs * 1.7 + ci as f32 * 1.9 + k as f32).sin(),
                        0.0,
                        0.7 * (t_secs * 1.3 + ci as f32 * 2.3 + 0.7 * k as f32).sin(),
                    ) * (k as f32 / (chain.pos.len() - 1) as f32);
                    let a = (t[k] - p) * chain.stiff - accel + euler + fling + air;
                    chain.prev[k] = p;
                    chain.pos[k] = p + v + a * (dt * dt);
                }
                // Bones do not stretch nor swing past their cone; hair does not go through her.
                let (cone_cos, cone_sin) = (chain.cone.cos(), chain.cone.sin());
                for _ in 0..3 {
                    for k in 1..chain.pos.len() {
                        let want = t[k] - t[k - 1];
                        let length = want.length();
                        let mut d = chain.pos[k] - chain.pos[k - 1];
                        let l = d.length().max(1e-6);
                        let w = want * (1.0 / length.max(1e-6));
                        let along = d * (1.0 / l);
                        let cos = along.dot(w);
                        if cos < cone_cos {
                            // Back onto the cone's edge, in the plane of the two directions.
                            let side = along - w * cos;
                            let side = side * (1.0 / side.length().max(1e-6));
                            d = w * cone_cos + side * cone_sin;
                        } else {
                            d = along;
                        }
                        chain.pos[k] = chain.pos[k - 1] + d * length;
                        let mut p = chain.pos[k];
                        p = push_out(p, skull, skull, 0.168);
                        if let (Some(a), Some(b)) = torso {
                            p = push_out(p, a, b, 0.078);
                        }
                        for &(a, b, r) in &arms {
                            p = push_out(p, a, b, r);
                        }
                        chain.pos[k] = p;
                    }
                }
            }
        }
        self.write(model, pose, &targets, &turns, head_turn);
    }

    /// Each chain bone turned from where the clip has it to where the hair is.
    fn write(
        &self,
        model: &GltfModel,
        pose: &mut Pose3D,
        targets: &[Vec<Vec3>],
        turns: &[Vec<Quat>],
        head: Quat,
    ) {
        for ((chain, t), r) in self.chains.iter().zip(targets).zip(turns) {
            let mut parent = head;
            for j in 0..chain.names.len() - 1 {
                let from = t[j + 1] - t[j];
                let to = chain.pos[j + 1] - chain.pos[j];
                let turned = q_norm(q_mul(q_between(arr(from), arr(to)), r[j]));
                let local = q_norm(q_mul(q_conj(parent), turned));
                model.set_pose_node_rotation(pose, &chain.names[j], local);
                parent = turned;
            }
        }
    }
}
