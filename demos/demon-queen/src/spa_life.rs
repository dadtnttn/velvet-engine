//! The spa girls' day: they come in through the gate in the morning, bathe at their own spot in
//! the pools, take turns on the loungers, one swims laps, and they leave in the evening.
//!
//! Each girl keeps a schedule by the hour (the world's day: noon at phase ½). When her hour turns
//! she gets up (her clip fades into the walk), finds her way on the springs' navmesh to where she
//! is going (A* and the funnel, [`super::nav`]), walking among the others and round the Queen
//! (ORCA), and settles there (the walk fades into sitting, lying or swimming, her body easing
//! into the spot). Her skin is posed on the GPU.

use super::baths::{plan, recolour_bow, SwimLoop};
use super::nav::{Crowd, NavMesh, Walker};
use super::*;
use velvet_render::Animator3D;

/// Where a girl can settle: her root's place, height and heading, the clip she plays there, and
/// where she steps to before settling.
#[derive(Debug, Clone)]
struct Spot {
    at: Vec3,
    yaw: f32,
    clip: usize,
    approach: [f32; 2],
    /// In the water (rings when she shifts).
    wet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Act {
    Away,
    Spot(usize),
    Swim,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// Not at the springs.
    Hidden,
    /// Getting up from where she was (fading into the walk, easing to her feet).
    Rising {
        t: f32,
        from: Vec3,
        from_yaw: f32,
    },
    Walking,
    /// Settling into her spot.
    Settling {
        t: f32,
        from: Vec3,
        from_yaw: f32,
    },
    Settled,
    Swimming,
}

struct Girl {
    mesh: MeshHandle,
    material: Material3D,
    texture: Option<TextureHandle>,
    animator: Animator3D,
    schedule: Vec<(f32, Act)>,
    doing: Act,
    phase: Phase,
    pos: Vec3,
    yaw: f32,
    walker: usize,
    swim_along: f32,
    next_touch: f32,
    offset: f32,
}

struct Clips {
    walk: usize,
    swim: usize,
}

/// A girl indoors: always there, at her place in her room.
struct RoomGirl {
    room: &'static str,
    mesh: MeshHandle,
    texture: Option<TextureHandle>,
    animator: Animator3D,
    at: Vec3,
    yaw: f32,
}

/// The spa girls of the valley.
pub(crate) struct SpaLife {
    /// Indoors: two at the reception's tea table.
    room_girls: Vec<RoomGirl>,
    model: GltfModel,
    scale: f32,
    clips: Clips,
    spots: Vec<Spot>,
    swim: Option<(SwimLoop, f32)>,
    girls: Vec<Girl>,
    crowd: Crowd,
    nav: NavMesh,
    /// Just inside the gate, and outside it where they come from and go.
    gate: [f32; 2],
    door: [f32; 2],
    /// The model's forward in its own frame (yaw of the way her toes point).
    forward_yaw: f32,
    walk_speed: f32,
    started: bool,
}

const WALK_CYCLE: f32 = 0.95;
const STRIDE: f32 = 0.78;
const FADE: f32 = 0.55;

/// The day of each girl: from each hour on, what she does.
fn schedules() -> Vec<Vec<(f32, Act)>> {
    use Act::*;
    vec![
        vec![
            (6.5, Spot(0)),
            (13.0, Spot(7)),
            (15.5, Spot(0)),
            (21.5, Away),
        ],
        vec![
            (7.0, Spot(1)),
            (16.0, Spot(7)),
            (18.0, Spot(1)),
            (22.5, Away),
        ],
        vec![
            (8.0, Spot(2)),
            (14.0, Spot(8)),
            (16.0, Spot(2)),
            (21.0, Away),
        ],
        vec![(6.0, Spot(3)), (19.5, Away)],
        vec![(7.0, Swim), (16.5, Spot(8)), (18.5, Swim), (21.0, Away)],
        vec![(9.0, Spot(7)), (12.5, Spot(4)), (20.0, Away)],
        vec![
            (10.0, Spot(8)),
            (13.5, Spot(5)),
            (17.0, Spot(6)),
            (23.0, Away),
        ],
    ]
}

fn act_at(schedule: &[(f32, Act)], hour: f32) -> Act {
    // The last entry started before this hour (yesterday's last before the first).
    schedule
        .iter()
        .rev()
        .find(|(h, _)| *h <= hour)
        .or(schedule.last())
        .map(|(_, a)| *a)
        .unwrap_or(Act::Away)
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let d = (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    a + d * t
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl SpaLife {
    /// Builds the girls from spa_girl.glb and the plan's spots; `textures` gives each bow hue its
    /// uploaded texture.
    pub(crate) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<Self> {
        let started = Instant::now();
        let p = plan();
        let mut model =
            super::prefetch::load(path).with_context(|| format!("load {}", path.display()))?;
        if model.meshes.is_empty() {
            anyhow::bail!("spa girl has no mesh");
        }
        let walk = gait::add_walk_clip(
            &mut model,
            "Spa_Walk",
            WALK_CYCLE,
            &gait::GaitStyle::default(),
            24,
        )?;
        let clip = |name: &str| {
            model
                .animation_index(name)
                .with_context(|| format!("clip {name}"))
        };
        let (sit, point, lie, swim_clip) = (
            clip("Spa_Sit")?,
            clip("Spa_Point")?,
            clip("Spa_Lie")?,
            clip("Spa_Swim")?,
        );
        let scale = p.spa_girls.first().map_or(1.2, |g| g.scale);
        let bone_y = |clip: usize, bone: &str| -> f32 {
            model
                .pose(clip, 0.0)
                .and_then(|pose| model.pose_node_transform(&pose, bone))
                .map_or(0.3, |m| m.w_axis[1])
                * scale
        };
        // Which way the model faces: its toes ahead of its ankles.
        let rest = model.rest_pose();
        let forward_yaw = match (
            model.pose_node_transform(&rest, "mixamorig:LeftToeBase"),
            model.pose_node_transform(&rest, "mixamorig:LeftFoot"),
        ) {
            (Some(t), Some(f)) => (t.w_axis[0] - f.w_axis[0]).atan2(t.w_axis[2] - f.w_axis[2]),
            _ => 0.0,
        };
        // The navmesh: wherever the Queen can stand, a girl can walk (a little more room kept).
        let mut nav = NavMesh::build([-24.0, -22.0, 24.0, 22.0], 0.25, 0.3, |x, z| {
            (!p.blocked_outdoor(x, z, 0.32)).then(|| p.stand(x, z))
        });
        // They keep to the paths and wade in only where they are going (a metre of pool costs six).
        nav.set_costs(|x, z| 1.0 + 5.0 * p.in_pool(x, z));
        // The spots: the exported placements (pools and loungers), then a few more round the pools.
        let mut spots = Vec::new();
        let mut swim = None;
        for g in &p.spa_girls {
            if let Some(s) = &g.swim {
                swim = Some((s.clone(), g.y));
                continue;
            }
            let clip = match g.clip.as_str() {
                "Spa_Point" => point,
                "Spa_Lie" => lie,
                _ => sit,
            };
            let at = Vec3::new(g.x, g.y, g.z);
            let wet = p.pool_at(g.x, g.z).is_some();
            let approach = if clip == lie {
                // Beside the lounger, where she can stand.
                nav.nearest(g.x, g.z, 2.0).unwrap_or([g.x, g.z])
            } else {
                nav.nearest(g.x, g.z, 1.0).unwrap_or([g.x, g.z])
            };
            spots.push((
                clip == lie,
                Spot {
                    at,
                    yaw: g.yaw,
                    clip,
                    approach,
                    wet,
                },
            ));
        }
        // Pool spots first (0..), loungers after: the schedules count on four pool spots from
        // the plan, three more here, then the loungers (7, 8).
        let mut pools: Vec<Spot> = spots
            .iter()
            .filter(|(lying, _)| !lying)
            .map(|(_, s)| s.clone())
            .collect();
        let loungers: Vec<Spot> = spots
            .iter()
            .filter(|(lying, _)| *lying)
            .map(|(_, s)| s.clone())
            .collect();
        let chest = bone_y(sit, "mixamorig:Spine2");
        for (id, angle, t) in [
            ("pozas-central", 0.9, 0.55),
            ("pozas-baja", 0.4, 0.5),
            ("pozas-alta", 1.5, 0.55),
        ] {
            if let Some(pool) = p.pools.iter().find(|q| q.id == id) {
                let (sx, sz) = pool.shore(angle);
                let (x, z) = (pool.x + (sx - pool.x) * t, pool.z + (sz - pool.z) * t);
                let yaw = (pool.x - x).atan2(pool.z - z);
                pools.push(Spot {
                    at: Vec3::new(x, pool.level - chest + 0.12, z),
                    yaw,
                    clip: sit,
                    approach: nav.nearest(x, z, 1.0).unwrap_or([x, z]),
                    wet: true,
                });
            }
        }
        let spots: Vec<Spot> = pools.into_iter().chain(loungers).collect();
        // The gate: just inside it, and beyond it.
        let gate_spot = p
            .hotspots
            .iter()
            .find(|h| h.to_village)
            .map_or([-8.4, 11.0], |h| [h.x, h.z]);
        let gate = nav
            .nearest(gate_spot[0], gate_spot[1] - 0.6, 3.0)
            .unwrap_or(gate_spot);
        let door = nav
            .nearest(gate_spot[0], gate_spot[1] + 1.4, 3.0)
            .unwrap_or(gate);
        // The girls themselves: a mesh each (posed on the GPU), their bow's colour.
        let base = model.meshes[0]
            .texture_index
            .and_then(|i| model.textures.get(i))
            .cloned();
        let mut textures: Vec<(Option<u32>, TextureHandle)> = Vec::new();
        let mut girls = Vec::new();
        let mut crowd = Crowd::default();
        for (k, schedule) in schedules().into_iter().enumerate() {
            let bow = p.spa_girls.get(k).and_then(|g| g.bow);
            let key = bow.map(|h| (h * 1000.0) as u32);
            let texture = match textures.iter().find(|(kk, _)| *kk == key) {
                Some((_, t)) => Some(*t),
                None => match &base {
                    Some(base) => {
                        let mut data = base.clone();
                        if let Some(hue) = bow {
                            recolour_bow(&mut data.rgba, hue);
                        }
                        let t = renderer.upload_texture(device, queue, &data)?;
                        textures.push((key, t));
                        Some(t)
                    }
                    None => None,
                },
            };
            let mut walker = Walker::new([1000.0 + k as f32 * 3.0, 1000.0], 0.3, 0.0);
            walker.anchored = true;
            crowd.walkers.push(walker);
            girls.push(Girl {
                mesh: renderer.upload_skinned_mesh(device, &model.meshes[0])?,
                material: model.meshes[0].material,
                texture,
                animator: Animator3D::new(walk, k as f32 * 0.37),
                schedule,
                doing: Act::Away,
                phase: Phase::Hidden,
                pos: Vec3::new(door[0], 0.0, door[1]),
                yaw: 0.0,
                walker: k,
                swim_along: k as f32 * 7.0,
                next_touch: k as f32 * 0.6,
                offset: k as f32 * 0.37,
            });
        }
        // Indoors: at the reception's tea table (on the cushions, the room's own frame turned
        // into the world's by its centre). (The private bath is the Queen's alone: rented, it is
        // empty.)
        let mut room_girls = Vec::new();
        for (room, dx, dz, yaw, clip, y, bow, phase) in [
            // (Her hips a little above the cushion: the seated clips sit on the floor at 0.)
            (
                "recepcion",
                -3.8f32,
                3.0f32,
                std::f32::consts::FRAC_PI_2,
                point,
                0.36,
                Some(0.55f32),
                0.2f32,
            ),
            (
                "recepcion",
                -1.4,
                3.0,
                -std::f32::consts::FRAC_PI_2,
                sit,
                0.36,
                Some(0.12),
                0.6,
            ),
        ] {
            let Some(r) = p.rooms.get(room) else { continue };
            let key = bow.map(|h| (h * 1000.0) as u32);
            let texture = match textures.iter().find(|(kk, _)| *kk == key) {
                Some((_, t)) => Some(*t),
                None => match &base {
                    Some(base) => {
                        let mut data = base.clone();
                        if let Some(hue) = bow {
                            recolour_bow(&mut data.rgba, hue);
                        }
                        let t = renderer.upload_texture(device, queue, &data)?;
                        textures.push((key, t));
                        Some(t)
                    }
                    None => None,
                },
            };
            room_girls.push(RoomGirl {
                room,
                mesh: renderer.upload_skinned_mesh(device, &model.meshes[0])?,
                texture,
                animator: Animator3D::new(clip, phase * 3.0),
                at: Vec3::new(r.center.x + dx, y, r.center.z + dz),
                yaw,
            });
        }
        // The Queen walks among them (the last walker: they make way for her).
        let mut queen = Walker::new([2000.0, 2000.0], 0.34, 0.0);
        queen.anchored = true;
        crowd.walkers.push(queen);
        eprintln!(
            "[3d] spa life: {} girls, {} spots, navmesh {} rectangles, built in {:.0} ms",
            girls.len(),
            spots.len(),
            nav.rects.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        Ok(Self {
            room_girls,
            model,
            scale,
            clips: Clips {
                walk,
                swim: swim_clip,
            },
            spots,
            swim,
            girls,
            crowd,
            nav,
            gate,
            door,
            forward_yaw,
            walk_speed: STRIDE / WALK_CYCLE,
            started: false,
        })
    }

    /// Where a girl is going to (the point she walks to) for an activity.
    fn destination(&self, act: Act, from: [f32; 2]) -> [f32; 2] {
        match act {
            Act::Away => self.door,
            Act::Spot(i) => self.spots.get(i).map_or(self.gate, |s| s.approach),
            Act::Swim => match &self.swim {
                // The nearest point of the loop.
                Some((s, _)) => s
                    .points
                    .iter()
                    .copied()
                    .min_by(|a, b| {
                        let da = (a[0] - from[0]).hypot(a[1] - from[1]);
                        let db = (b[0] - from[0]).hypot(b[1] - from[1]);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .unwrap_or(self.gate),
                None => self.gate,
            },
        }
    }

    /// The pose (root place, heading) she settles into for an activity at `along` round the loop.
    fn settled_pose(&self, act: Act, along: f32) -> Option<(Vec3, f32)> {
        match act {
            Act::Spot(i) => self.spots.get(i).map(|s| (s.at, s.yaw)),
            Act::Swim => self.swim.as_ref().map(|(s, y)| {
                let (hips, heading) = s.at(along);
                let yaw = heading - s.head_yaw;
                let (c, sn) = (yaw.cos(), yaw.sin());
                let offset = Vec3::new(
                    s.hips[0] * c + s.hips[1] * sn,
                    0.0,
                    -s.hips[0] * sn + s.hips[1] * c,
                );
                (Vec3::new(hips.x - offset.x, *y, hips.z - offset.z), yaw)
            }),
            Act::Away => None,
        }
    }

    fn clip_for(&self, act: Act) -> usize {
        match act {
            Act::Spot(i) => self.spots.get(i).map_or(self.clips.walk, |s| s.clip),
            Act::Swim => self.clips.swim,
            Act::Away => self.clips.walk,
        }
    }

    /// Where along the swim loop is nearest (x, z).
    fn swim_along_near(&self, x: f32, z: f32) -> f32 {
        let Some((s, _)) = &self.swim else { return 0.0 };
        let mut along = 0.0;
        let mut best = (f32::INFINITY, 0.0);
        let n = s.points.len();
        for i in 0..n {
            let a = s.points[i];
            let b = s.points[(i + 1) % n];
            let d = (a[0] - x).hypot(a[1] - z);
            if d < best.0 {
                best = (d, along);
            }
            along += (b[0] - a[0]).hypot(b[1] - a[1]);
        }
        best.1
    }

    fn start_walk(&mut self, k: usize, to: [f32; 2]) {
        let from = [self.girls[k].pos.x, self.girls[k].pos.z];
        let from = self.nav.nearest(from[0], from[1], 2.0).unwrap_or(from);
        let w = &mut self.crowd.walkers[self.girls[k].walker];
        w.pos = from;
        w.vel = [0.0; 2];
        w.max_speed = self.walk_speed;
        w.anchored = false;
        w.path = self.nav.find_path(from, to).unwrap_or_else(|| vec![to]);
        let g = &mut self.girls[k];
        g.phase = Phase::Walking;
        g.animator.play(self.clips.walk, FADE);
    }

    /// The day moves on: who goes where, the crowd's step, everyone's pose. `queen` is where the
    /// Queen stands and how she moves; `touch` gets (x, z) for the rings the girls make.
    pub(crate) fn tick(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        hour: f32,
        dt: f32,
        queen: (Vec3, Vec3),
        touch: &mut Vec<(f32, f32)>,
    ) {
        let dt = dt.clamp(0.0, 0.1);
        // At first sight everyone is already where her hour has her.
        if !self.started {
            self.started = true;
            for k in 0..self.girls.len() {
                let act = act_at(&self.girls[k].schedule, hour);
                self.girls[k].doing = act;
                self.girls[k].swim_along = self.girls[k].offset * 30.0;
                if let Some((at, yaw)) = self.settled_pose(act, self.girls[k].swim_along) {
                    let clip = self.clip_for(act);
                    let g = &mut self.girls[k];
                    g.pos = at;
                    g.yaw = yaw;
                    g.phase = if act == Act::Swim {
                        Phase::Swimming
                    } else {
                        Phase::Settled
                    };
                    g.animator = Animator3D::new(clip, g.offset * 3.0);
                    let w = &mut self.crowd.walkers[g.walker];
                    w.pos = [at.x, at.z];
                    w.anchored = true;
                    w.radius = 0.38;
                }
            }
        }
        // The Queen, for them to go round.
        if let Some(q) = self.crowd.walkers.last_mut() {
            q.pos = [queen.0.x, queen.0.z];
            q.vel = [queen.1.x, queen.1.z];
        }
        // Whose hour has turned.
        for k in 0..self.girls.len() {
            let want = act_at(&self.girls[k].schedule, hour);
            if want == self.girls[k].doing {
                continue;
            }
            self.girls[k].doing = want;
            match self.girls[k].phase {
                Phase::Hidden => {
                    // In through the gate.
                    self.girls[k].pos = Vec3::new(
                        self.door[0],
                        plan().stand(self.door[0], self.door[1]),
                        self.door[1],
                    );
                    let to = self.destination(want, self.door);
                    self.start_walk(k, to);
                }
                Phase::Settled | Phase::Swimming | Phase::Settling { .. } => {
                    let g = &mut self.girls[k];
                    g.phase = Phase::Rising {
                        t: 0.0,
                        from: g.pos,
                        from_yaw: g.yaw,
                    };
                    g.animator.play(self.clips.walk, FADE);
                }
                Phase::Walking | Phase::Rising { .. } => {
                    // Already on her feet: just a new way.
                    if matches!(self.girls[k].phase, Phase::Walking) {
                        let from = [self.girls[k].pos.x, self.girls[k].pos.z];
                        let to = self.destination(want, from);
                        self.start_walk(k, to);
                    }
                }
            }
        }
        self.crowd.step(&self.nav, dt);
        let p = plan();
        for k in 0..self.girls.len() {
            let wi = self.girls[k].walker;
            match self.girls[k].phase {
                Phase::Hidden => {}
                Phase::Rising { t, from, from_yaw } => {
                    let t = t + dt / FADE;
                    // To her feet beside where she was.
                    let stand_at = self
                        .nav
                        .nearest(from.x, from.z, 2.0)
                        .unwrap_or([from.x, from.z]);
                    let to = Vec3::new(stand_at[0], p.stand(stand_at[0], stand_at[1]), stand_at[1]);
                    let e = smoothstep(t);
                    let g = &mut self.girls[k];
                    g.pos = from + (to - from) * e;
                    g.yaw = lerp_angle(from_yaw, from_yaw, e);
                    if t >= 1.0 {
                        let doing = g.doing;
                        let dest = self.destination(doing, stand_at);
                        self.girls[k].pos = to;
                        self.start_walk(k, dest);
                    } else {
                        self.girls[k].phase = Phase::Rising { t, from, from_yaw };
                    }
                }
                Phase::Walking => {
                    let w = &self.crowd.walkers[wi];
                    let (pos, vel, arrived) = (w.pos, w.vel, w.arrived());
                    let speed = vel[0].hypot(vel[1]);
                    let g = &mut self.girls[k];
                    g.pos = Vec3::new(pos[0], p.stand(pos[0], pos[1]), pos[1]);
                    if speed > 0.05 {
                        let target = vel[0].atan2(vel[1]) - self.forward_yaw;
                        g.yaw = lerp_angle(g.yaw, target, (dt * 8.0).min(1.0));
                    }
                    g.animator
                        .set_speed((speed / self.walk_speed).clamp(0.3, 1.6));
                    if p.in_pool(pos[0], pos[1]) > 0.2 {
                        g.next_touch -= dt;
                        if g.next_touch <= 0.0 {
                            g.next_touch = 0.4;
                            touch.push((pos[0], pos[1]));
                        }
                    }
                    if arrived {
                        g.animator.set_speed(1.0);
                        match g.doing {
                            Act::Away => {
                                g.phase = Phase::Hidden;
                                let w = &mut self.crowd.walkers[wi];
                                w.pos = [1000.0 + k as f32 * 3.0, 1000.0];
                                w.anchored = true;
                            }
                            act => {
                                let from = g.pos;
                                let from_yaw = g.yaw;
                                g.phase = Phase::Settling {
                                    t: 0.0,
                                    from,
                                    from_yaw,
                                };
                                let clip = self.clip_for(act);
                                let along = self.swim_along_near(from.x, from.z);
                                let g = &mut self.girls[k];
                                if act == Act::Swim {
                                    g.swim_along = along;
                                }
                                g.animator.play(clip, FADE);
                                let w = &mut self.crowd.walkers[wi];
                                w.anchored = true;
                                w.radius = 0.38;
                            }
                        }
                    }
                }
                Phase::Settling { t, from, from_yaw } => {
                    let t = t + dt / FADE;
                    let doing = self.girls[k].doing;
                    if doing == Act::Swim {
                        self.girls[k].swim_along +=
                            dt * self.swim.as_ref().map_or(0.3, |(s, _)| s.speed);
                    }
                    let along = self.girls[k].swim_along;
                    if let Some((at, yaw)) = self.settled_pose(doing, along) {
                        let e = smoothstep(t);
                        let g = &mut self.girls[k];
                        g.pos = from + (at - from) * e;
                        g.yaw = lerp_angle(from_yaw, yaw, e);
                        self.crowd.walkers[wi].pos = [g.pos.x, g.pos.z];
                    }
                    let g = &mut self.girls[k];
                    g.phase = if t >= 1.0 {
                        if doing == Act::Swim {
                            Phase::Swimming
                        } else {
                            Phase::Settled
                        }
                    } else {
                        Phase::Settling { t, from, from_yaw }
                    };
                }
                Phase::Settled => {
                    let doing = self.girls[k].doing;
                    let wet =
                        matches!(doing, Act::Spot(i) if self.spots.get(i).is_some_and(|s| s.wet));
                    let base = self.settled_pose(doing, 0.0).map(|(at, _)| at);
                    let g = &mut self.girls[k];
                    if let (true, Some(base)) = (wet, base) {
                        // A gentle bob in the water, and now and then she shifts.
                        g.pos.y = base.y + 0.012 * (g.animator.time() * 0.9 + g.offset * 6.0).sin();
                        g.next_touch -= dt;
                        if g.next_touch <= 0.0 {
                            g.next_touch = 2.5
                                + 4.0 * (g.animator.time() * 7.31 + g.offset * 13.0).sin().abs();
                            touch.push((g.pos.x, g.pos.z));
                        }
                    }
                }
                Phase::Swimming => {
                    let speed = self.swim.as_ref().map_or(0.3, |(s, _)| s.speed);
                    self.girls[k].swim_along += dt * speed;
                    let along = self.girls[k].swim_along;
                    if let Some((at, yaw)) = self.settled_pose(Act::Swim, along) {
                        let g = &mut self.girls[k];
                        g.pos = at + Vec3::new(0.0, 0.01 * (along * 7.0).sin(), 0.0);
                        g.yaw = yaw;
                        let w = &mut self.crowd.walkers[wi];
                        w.pos = [at.x, at.z];
                        g.next_touch -= dt;
                        if g.next_touch <= 0.0 {
                            g.next_touch = 0.35;
                            touch.push((at.x, at.z));
                        }
                    }
                }
            }
        }
        // Their poses: each clip (or the fade between two), deformed on the GPU.
        for g in &mut self.girls {
            if g.phase == Phase::Hidden {
                continue;
            }
            g.animator.advance(dt);
            if let Some(joints) = g
                .animator
                .pose(&self.model)
                .and_then(|pose| self.model.joint_matrices(0, &pose))
            {
                let _ = renderer.set_skin_joints(queue, g.mesh, &joints);
            }
        }
    }

    /// The girls of a room: posed (on the GPU) and drawn.
    pub(crate) fn room_draws(
        &mut self,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        room: &str,
        dt: f32,
        draws: &mut Vec<MeshDraw3D>,
    ) {
        let s = self.scale;
        let material = self.model.meshes[0].material.with_ambient_occlusion(0.28);
        for g in self.room_girls.iter_mut().filter(|g| g.room == room) {
            g.animator.advance(dt);
            if let Some(joints) = g
                .animator
                .pose(&self.model)
                .and_then(|pose| self.model.joint_matrices(0, &pose))
            {
                let _ = renderer.set_skin_joints(queue, g.mesh, &joints);
            }
            draws.push(MeshDraw3D {
                mesh: g.mesh,
                transform: Mat4::from_translation(g.at)
                    * Mat4::from_rotation_y(g.yaw)
                    * Mat4::from_scale(Vec3::new(s, s, s))
                    * self.model.mesh_transform(0),
                material,
                texture: g.texture,
            });
        }
    }

    /// The girls to draw.
    pub(crate) fn draws(&self, draws: &mut Vec<MeshDraw3D>) {
        let s = self.scale;
        for g in self.girls.iter().filter(|g| g.phase != Phase::Hidden) {
            draws.push(MeshDraw3D {
                mesh: g.mesh,
                transform: Mat4::from_translation(g.pos)
                    * Mat4::from_rotation_y(g.yaw)
                    * Mat4::from_scale(Vec3::new(s, s, s))
                    * self.model.mesh_transform(0),
                material: g.material.with_ambient_occlusion(0.28),
                texture: g.texture,
            });
        }
    }

    /// Where the girls stand (x, z, radius), for the Queen to walk round.
    pub(crate) fn obstacles(&self) -> Vec<[f32; 3]> {
        self.girls
            .iter()
            .filter(|g| !matches!(g.phase, Phase::Hidden | Phase::Swimming))
            .map(|g| {
                [
                    g.pos.x,
                    g.pos.z,
                    if matches!(g.phase, Phase::Walking) {
                        0.25
                    } else {
                        0.38
                    },
                ]
            })
            .collect()
    }

    /// (who, phase name, where) for reports.
    pub(crate) fn report(&self) -> Vec<(usize, &'static str, [f32; 2])> {
        self.girls
            .iter()
            .enumerate()
            .map(|(k, g)| {
                let name = match g.phase {
                    Phase::Hidden => "away",
                    Phase::Rising { .. } => "rising",
                    Phase::Walking => "walking",
                    Phase::Settling { .. } => "settling",
                    Phase::Settled => "settled",
                    Phase::Swimming => "swimming",
                };
                (k, name, [g.pos.x, g.pos.z])
            })
            .collect()
    }

    #[allow(dead_code)]
    pub(crate) fn gate(&self) -> [f32; 2] {
        self.gate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hour_has_one_girl_per_spot() {
        let schedules = schedules();
        for step in 0..(24 * 4) {
            let hour = step as f32 / 4.0;
            let mut taken = std::collections::BTreeMap::new();
            for (k, s) in schedules.iter().enumerate() {
                if let Act::Spot(i) = act_at(s, hour) {
                    if let Some(other) = taken.insert(i, k) {
                        panic!("at {hour}h girls {other} and {k} both want spot {i}");
                    }
                }
            }
        }
        // Mornings full, nights empty.
        assert!(schedules.iter().all(|s| act_at(s, 2.0) == Act::Away));
        assert!(
            schedules
                .iter()
                .filter(|s| act_at(s, 11.0) != Act::Away)
                .count()
                >= 6
        );
    }

    #[test]
    fn a_girl_walks_from_the_gate_to_every_spot_without_sticking() {
        let p = plan();
        let nav = NavMesh::build([-24.0, -22.0, 24.0, 22.0], 0.25, 0.3, |x, z| {
            (!p.blocked_outdoor(x, z, 0.32)).then(|| p.stand(x, z))
        });
        let gate = p
            .hotspots
            .iter()
            .find(|h| h.to_village)
            .map(|h| [h.x, h.z])
            .unwrap();
        let gate = nav.nearest(gate[0], gate[1] - 0.6, 3.0).unwrap();
        let mut targets: Vec<[f32; 2]> = p
            .spa_girls
            .iter()
            .filter(|g| g.swim.is_none())
            .map(|g| nav.nearest(g.x, g.z, 2.0).unwrap())
            .collect();
        if let Some(s) = p.spa_girls.iter().find_map(|g| g.swim.as_ref()) {
            targets.push(s.points[0]);
        }
        for to in targets {
            let mut crowd = Crowd::default();
            let mut w = Walker::new(gate, 0.3, 0.82);
            w.path = nav.find_path(gate, to).unwrap();
            let path = w.path.clone();
            crowd.walkers.push(w);
            let mut t = 0.0;
            while !crowd.walkers[0].arrived() && t < 90.0 {
                crowd.step(&nav, 1.0 / 60.0);
                t += 1.0 / 60.0;
            }
            assert!(
                crowd.walkers[0].arrived(),
                "stuck at {:?} going to {to:?} along {path:?}",
                crowd.walkers[0].pos
            );
        }
    }

    #[test]
    fn the_springs_navmesh_joins_the_gate_to_every_pool_and_lounger() {
        let p = plan();
        let nav = NavMesh::build([-24.0, -22.0, 24.0, 22.0], 0.25, 0.3, |x, z| {
            (!p.blocked_outdoor(x, z, 0.32)).then(|| p.stand(x, z))
        });
        let gate = p
            .hotspots
            .iter()
            .find(|h| h.to_village)
            .map(|h| [h.x, h.z])
            .unwrap();
        let gate = nav
            .nearest(gate[0], gate[1] - 0.6, 3.0)
            .expect("the gate is walkable");
        for g in &p.spa_girls {
            if g.swim.is_some() {
                continue;
            }
            let to = nav.nearest(g.x, g.z, 2.0).expect("near every spot");
            let path = nav
                .find_path(gate, to)
                .unwrap_or_else(|| panic!("a way from the gate to {} at {:?}", g.clip, to));
            assert!(!path.is_empty());
        }
    }
}

#[cfg(test)]
mod crowd_tests {
    use super::*;

    /// Two girls come in through the gate in the same moment, bound for different spots: they
    /// make room for each other and both get there (they used to pin one another to the
    /// navmesh's edge by the footbridge).
    #[test]
    fn two_girls_entering_together_both_arrive() {
        let p = plan();
        let nav = NavMesh::build([-24.0, -22.0, 24.0, 22.0], 0.25, 0.3, |x, z| {
            (!p.blocked_outdoor(x, z, 0.32)).then(|| p.stand(x, z))
        });
        let seats: Vec<[f32; 2]> = p
            .spa_girls
            .iter()
            .filter(|g| g.swim.is_none())
            .map(|g| nav.nearest(g.x, g.z, 2.0).unwrap())
            .collect();
        let swim = p
            .spa_girls
            .iter()
            .find_map(|g| g.swim.as_ref())
            .unwrap()
            .points[0];
        let gate = p
            .hotspots
            .iter()
            .find(|h| h.to_village)
            .map(|h| [h.x, h.z])
            .unwrap();
        let door = nav.nearest(gate[0], gate[1] + 1.4, 3.0).unwrap();
        for pair in [[seats[1], swim], [seats[0], seats[2]], [seats[3], seats[4]]] {
            let mut crowd = Crowd::default();
            for to in pair {
                let mut w = Walker::new(door, 0.3, 0.82);
                w.path = nav.find_path(door, to).unwrap();
                crowd.walkers.push(w);
            }
            for _ in 0..(90 * 60) {
                crowd.step(&nav, 1.0 / 60.0);
                if crowd.walkers.iter().all(|w| w.arrived()) {
                    break;
                }
            }
            assert!(
                crowd.walkers.iter().all(|w| w.arrived()),
                "{:?}",
                crowd
                    .walkers
                    .iter()
                    .map(|w| (w.pos, w.path.len()))
                    .collect::<Vec<_>>()
            );
        }
    }
}
