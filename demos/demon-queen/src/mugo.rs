//! Mugo, the goblin who keeps the springs' rules, out in the valley: he keeps his post by the gate
//! with his hands behind his back and, once asked, leads the Queen from one good spot for a photo
//! to the next (the spots of the approved selfies), walking the springs' navmesh round her and
//! the girls, waiting for her when she lags, and pointing at the view when they get there.
//!
//! The game says where he goes (`baths::State::guide`, a spot of [`PHOTO_SPOTS`]); this side poses
//! him (his clips come from `tools/mugo_blender.py`) and reports where he is and whether he is
//! there.

use super::baths::{capybara_focus, plan};
use super::nav::{Crowd, NavMesh, Walker};
use super::*;
use velvet_render::Animator3D;

/// A good spot for a photo: the landmark it looks at, and from where (the camera of the approved
/// selfie: `distance` from it at `azimuth`, as the selfie capture placed it).
pub(crate) struct PhotoSpot {
    pub id: &'static str,
    pub name: &'static str,
    landmark: [f32; 2],
    distance: f32,
    azimuth: f32,
}

const AZIMUTHS: [f32; 8] = [0.0, 1.5708, 3.1416, 4.7124, 0.7854, 2.3562, 3.927, 5.4978];

/// The spots in the order he shows them (the capybaras' landmark is wherever the herd dozes).
pub(crate) const PHOTO_SPOTS: [PhotoSpot; 6] = [
    PhotoSpot {
        id: "torii",
        name: "El torii",
        landmark: [-8.2, 10.6],
        distance: 6.0,
        azimuth: AZIMUTHS[5],
    },
    PhotoSpot {
        id: "capibaras",
        name: "Las capibaras",
        landmark: [f32::NAN, f32::NAN],
        distance: 3.2,
        azimuth: AZIMUTHS[4],
    },
    PhotoSpot {
        id: "pozas",
        name: "Las pozas",
        landmark: [4.0, -3.2],
        distance: 6.0,
        azimuth: AZIMUTHS[1],
    },
    PhotoSpot {
        id: "cascada",
        name: "La cascada",
        landmark: [0.2, -7.6],
        distance: 5.5,
        azimuth: AZIMUTHS[4],
    },
    PhotoSpot {
        id: "puente",
        name: "El puente",
        landmark: [2.0, -8.62],
        distance: 5.0,
        azimuth: AZIMUTHS[6],
    },
    PhotoSpot {
        id: "pabellon",
        name: "El pabellón",
        landmark: [10.4, -4.0],
        distance: 6.0,
        azimuth: AZIMUTHS[5],
    },
];

impl PhotoSpot {
    pub(crate) fn landmark(&self) -> [f32; 2] {
        if self.landmark[0].is_nan() {
            capybara_focus().map_or([4.0, -3.2], |(x, z)| [x, z])
        } else {
            self.landmark
        }
    }

    /// Where she stands for it (the selfie's camera).
    pub(crate) fn stand(&self) -> [f32; 2] {
        let [x, z] = self.landmark();
        [
            x + self.azimuth.sin() * self.distance,
            z + self.azimuth.cos() * self.distance,
        ]
    }

    /// Where Mugo waits: a step aside from her spot, off her line of sight.
    fn post(&self) -> [f32; 2] {
        let [sx, sz] = self.stand();
        let [lx, lz] = self.landmark();
        let (dx, dz) = (lx - sx, lz - sz);
        let l = dx.hypot(dz).max(1e-3);
        [
            sx - dz / l * 1.3 - dx / l * 0.4,
            sz + dx / l * 1.3 - dz / l * 0.4,
        ]
    }
}

/// The first spot not photographed yet (`foto_<id>` in the story).
pub(crate) fn next_spot(taken: impl Fn(&str) -> bool) -> Option<usize> {
    PHOTO_SPOTS.iter().position(|s| !taken(s.id))
}

#[derive(Clone, Copy, PartialEq)]
enum Doing {
    Post,
    Walking,
    Waiting,
    Pointing,
}

pub(crate) struct Mugo {
    model: GltfModel,
    mesh: MeshHandle,
    material: Material3D,
    texture: Option<TextureHandle>,
    animator: Animator3D,
    idle: usize,
    walk: usize,
    point: usize,
    talk: usize,
    scale: f32,
    forward_yaw: f32,
    nav: NavMesh,
    crowd: Crowd,
    pos: Vec3,
    yaw: f32,
    doing: Doing,
    /// The spot he is going to (`None`: his post by the gate).
    going: Option<usize>,
    gate: [f32; 2],
}

const SPEED: f32 = 1.25;
const WALK_CYCLE: f32 = 0.9;
const STRIDE: f32 = 0.5;
/// Further than this behind him, he stops and waits for her.
const LAG: f32 = 6.5;

impl Mugo {
    pub(crate) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let p = plan();
        let model =
            super::prefetch::load(path).with_context(|| format!("load {}", path.display()))?;
        if model.meshes.is_empty() {
            anyhow::bail!("Mugo has no mesh");
        }
        let clip = |name: &str| {
            model
                .animation_index(name)
                .with_context(|| format!("clip {name}"))
        };
        let (idle, walk, point, talk) = (
            clip("Mugo_Idle")?,
            clip("Mugo_Walk")?,
            clip("Mugo_Point")?,
            clip("Mugo_Talk")?,
        );
        let rest = model.rest_pose();
        let forward_yaw = match (
            model.pose_node_transform(&rest, "CC_Base_L_ToeBase"),
            model.pose_node_transform(&rest, "CC_Base_L_Foot"),
        ) {
            (Some(t), Some(f)) => (t.w_axis[0] - f.w_axis[0]).atan2(t.w_axis[2] - f.w_axis[2]),
            _ => 0.0,
        };
        let mut nav = NavMesh::build([-24.0, -22.0, 24.0, 22.0], 0.25, 0.3, |x, z| {
            (!p.blocked_outdoor(x, z, 0.3)).then(|| p.stand(x, z))
        });
        // He keeps to the paths: a metre of pool costs him eight.
        nav.set_costs(|x, z| 1.0 + 7.0 * p.in_pool(x, z));
        let gate_spot = p
            .hotspots
            .iter()
            .find(|h| h.to_village)
            .map_or([-8.4, 11.0], |h| [h.x, h.z]);
        let gate = nav
            .nearest(gate_spot[0] + 1.6, gate_spot[1] - 1.8, 3.0)
            .unwrap_or(gate_spot);
        let mesh = &model.meshes[0];
        let texture = match mesh.texture_index.and_then(|i| model.textures.get(i)) {
            Some(data) => Some(renderer.upload_texture(device, queue, data)?),
            None => None,
        };
        let handle = renderer.upload_skinned_mesh(device, mesh)?;
        let material = mesh.material;
        let mut crowd = Crowd::default();
        let mut me = Walker::new(gate, 0.28, SPEED);
        me.anchored = false;
        crowd.walkers.push(me);
        // The Queen, for him to go round.
        let mut queen = Walker::new([1000.0, 1000.0], 0.35, 0.0);
        queen.anchored = true;
        crowd.walkers.push(queen);
        let y = p.stand(gate[0], gate[1]);
        Ok(Self {
            animator: Animator3D::new(idle, 0.0),
            model,
            mesh: handle,
            material,
            texture,
            idle,
            walk,
            point,
            talk,
            // He comes up to the Queen's chest (the model is 0.9 m; the chibi Queen stands taller).
            scale: 1.3,
            forward_yaw,
            nav,
            crowd,
            pos: Vec3::new(gate[0], y, gate[1]),
            yaw: -0.6,
            doing: Doing::Post,
            going: None,
            gate,
        })
    }

    /// Where he is (x, z), and whether he is at the spot he was sent to, pointing.
    pub(crate) fn report(&self) -> ([f32; 2], bool) {
        ([self.pos.x, self.pos.z], self.doing == Doing::Pointing)
    }

    /// One frame: go to spot `guide` (or back to his post), round the Queen at `queen`.
    pub(crate) fn tick(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        dt: f32,
        guide: Option<usize>,
        queen: Vec3,
    ) {
        let dt = dt.clamp(0.0, 0.1);
        let p = plan();
        if guide != self.going {
            self.going = guide;
            let goal = guide
                .and_then(|k| PHOTO_SPOTS.get(k))
                .map_or(self.gate, |s| s.post());
            let goal = self.nav.nearest(goal[0], goal[1], 2.5).unwrap_or(goal);
            let from = [self.pos.x, self.pos.z];
            let path = self.nav.find_path(from, goal).unwrap_or_else(|| vec![goal]);
            self.crowd.walkers[0].path = path;
            self.doing = Doing::Walking;
        }
        let q = &mut self.crowd.walkers[1];
        q.pos = [queen.x, queen.z];
        let lag = (queen - self.pos).length();
        let walker = &mut self.crowd.walkers[0];
        // Leading: he waits for her when she falls behind.
        let leading = self.going.is_some();
        let waiting = leading && !walker.arrived() && lag > LAG;
        walker.max_speed = if waiting { 0.0 } else { SPEED };
        self.crowd.step(&self.nav, dt);
        let w = &self.crowd.walkers[0];
        let speed = (w.vel[0] * w.vel[0] + w.vel[1] * w.vel[1]).sqrt();
        self.pos = Vec3::new(w.pos[0], p.stand(w.pos[0], w.pos[1]), w.pos[1]);
        let arrived = w.arrived();
        // Where he faces: the way he walks; at a spot, the view; at his post or waiting, her.
        let want = if speed > 0.15 {
            Some(w.vel[0].atan2(w.vel[1]))
        } else if arrived && leading {
            let spot = &PHOTO_SPOTS[self.going.unwrap_or(0)];
            let [lx, lz] = spot.landmark();
            Some((lx - self.pos.x).atan2(lz - self.pos.z))
        } else if lag < 9.0 {
            Some((queen.x - self.pos.x).atan2(queen.z - self.pos.z))
        } else {
            None
        };
        if let Some(want) = want {
            let d = (want - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += d * (dt * 6.0).min(1.0);
        }
        let doing = if speed > 0.15 {
            Doing::Walking
        } else if arrived && leading {
            Doing::Pointing
        } else if waiting {
            Doing::Waiting
        } else {
            Doing::Post
        };
        let clip = match doing {
            Doing::Walking => self.walk,
            Doing::Pointing => self.point,
            Doing::Waiting => self.talk,
            Doing::Post => self.idle,
        };
        if clip != self.animator.clip() {
            self.animator.play(clip, 0.25);
        }
        self.doing = doing;
        // His feet keep pace with the ground.
        self.animator.set_speed(if doing == Doing::Walking {
            (speed / (STRIDE / WALK_CYCLE)).clamp(0.6, 1.8)
        } else {
            1.0
        });
        self.animator.advance(dt);
        if let Some(joints) = self
            .animator
            .pose(&self.model)
            .and_then(|pose| self.model.joint_matrices(0, &pose))
        {
            let _ = renderer.set_skin_joints(queue, self.mesh, &joints);
        }
    }

    pub(crate) fn draws(&self, draws: &mut Vec<MeshDraw3D>) {
        let s = self.scale;
        draws.push(MeshDraw3D {
            mesh: self.mesh,
            transform: Mat4::from_translation(self.pos)
                * Mat4::from_rotation_y(self.yaw - self.forward_yaw)
                * Mat4::from_scale(Vec3::new(s, s, s))
                * self.model.mesh_transform(0),
            material: self.material.with_ambient_occlusion(0.28),
            texture: self.texture,
        });
    }

    /// Where he stands (x, z, radius), for the Queen to walk round.
    pub(crate) fn obstacle(&self) -> [f32; 3] {
        [self.pos.x, self.pos.z, 0.3]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spots_come_in_order_and_skip_the_ones_taken() {
        assert_eq!(next_spot(|_| false), Some(0));
        assert_eq!(next_spot(|id| id == "torii" || id == "capibaras"), Some(2));
        assert_eq!(next_spot(|_| true), None);
    }

    #[test]
    fn his_post_at_a_spot_is_off_her_line_of_sight() {
        for spot in &PHOTO_SPOTS {
            let [sx, sz] = spot.stand();
            let [px, pz] = spot.post();
            let d = (px - sx).hypot(pz - sz);
            assert!((1.0..2.0).contains(&d), "{}: {d}", spot.id);
        }
    }
}
