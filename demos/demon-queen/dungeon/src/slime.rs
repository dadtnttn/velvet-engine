//! The Threshold slime, and its blue and green cousins that lurk by the trees
//! and mushrooms of the overgrown halls. Gameplay state is independent of
//! rendering and uses the blueprint for every landing; visual deformation
//! reads this same state.
use serde::{Deserialize, Serialize};
use velvet_dungeon::{Blueprint, Shape};
use velvet_math::Vec3;
use velvet_render::{Material3D, Mesh3D, MeshVertex3D};

pub const HEIGHT: f32 = 0.668;
pub const MAX_HEALTH: f32 = 80.0;
const WAKE: f32 = 3.8;
const REACH: f32 = 2.3;
const DAMAGE: f32 = 7.0;
/// Leave room for the player capsule and the slime body after a leap.
pub const CONTACT_DISTANCE: f32 = 0.92;

/// A slime's colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tint {
    #[default]
    Blue,
    Green,
}

impl Tint {
    /// Body colour, brightening with the hurt glow.
    pub fn body(self, glow: f32) -> [f32; 4] {
        match self {
            Tint::Blue => [0.31 + glow * 0.17, 0.81 + glow * 0.14, 1.0, 0.82],
            Tint::Green => [
                0.36 + glow * 0.2,
                0.92 + glow * 0.08,
                0.34 + glow * 0.2,
                0.82,
            ],
        }
    }
    /// Droplets and the glowing core.
    pub fn drop(self) -> [f32; 3] {
        match self {
            Tint::Blue => [0.24, 0.64, 1.0],
            Tint::Green => [0.4, 0.95, 0.3],
        }
    }
    pub fn puddle(self) -> [f32; 3] {
        match self {
            Tint::Blue => [0.11, 0.49, 0.91],
            Tint::Green => [0.2, 0.62, 0.12],
        }
    }
    pub fn core(self, glow: f32) -> [f32; 3] {
        match self {
            Tint::Blue => [0.08 + glow * 0.2, 0.27 + glow * 0.3, 0.81 + glow * 0.1],
            Tint::Green => [0.12 + glow * 0.2, 0.6 + glow * 0.3, 0.1 + glow * 0.1],
        }
    }
    pub fn light(self) -> [f32; 3] {
        match self {
            Tint::Blue => [0.17, 0.55, 1.0],
            Tint::Green => [0.3, 1.0, 0.22],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Sleep,
    Wake,
    Chase,
    Telegraph,
    Pounce,
    Recover,
    Hurt,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hop {
    from: Vec3,
    to: Vec3,
    height: f32,
    crouch: f32,
    air: f32,
    elapsed: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub age: f32,
    pub life: f32,
    pub puddle: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slime {
    pub room: usize,
    pub home: Vec3,
    pub position: Vec3,
    pub health: f32,
    pub mode: Mode,
    pub mode_time: f32,
    pub time: f32,
    pub cooldown: f32,
    pub idle_hop: f32,
    pub hop: Option<Hop>,
    pub lift: f32,
    pub squash: f32,
    pub squash_velocity: f32,
    pub squash_target: f32,
    pub jiggle: f32,
    pub glow: f32,
    pub yaw: f32,
    pub lean: [f32; 2],
    pub wound_at: Vec3,
    pub wound_time: f32,
    pub particles: Vec<Particle>,
    pub burst: bool,
    #[serde(default)]
    pub tint: Tint,
    /// It landed from a hop this tick (for its squelch).
    #[serde(skip)]
    pub landed: bool,
}

fn flat(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn hash(n: u64) -> f32 {
    let mut x = n ^ (n >> 33);
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    ((x ^ (x >> 33)) as u32 as f32) / u32::MAX as f32
}

impl Slime {
    pub fn new(map: &Blueprint) -> Option<Self> {
        let room = map.rooms.get(map.entrance)?;
        if room.shape == Shape::Bridge {
            return None;
        }
        let door = map
            .connections
            .iter()
            .find(|c| c.critical && (c.a == room.id || c.b == room.id))
            .and_then(|c| room.sockets.iter().find(|s| s.connection == c.id))
            .or_else(|| room.sockets.first())?;
        let center = room.center;
        let toward = flat(door.position - center).normalize_or_zero();
        let side = Vec3::new(-toward.z, 0.0, toward.x)
            * if hash(map.seed ^ 0x5157) < 0.5 {
                -1.0
            } else {
                1.0
            };
        let mut position = center + flat(door.position - center) * 0.58;
        'placement: for flank in [2.0, 1.6, 1.2, 0.9] {
            for t in [0.68, 0.64, 0.60, 0.56, 0.52, 0.48, 0.44] {
                let candidate = center + flat(door.position - center) * t + side * flank;
                if room.contains(candidate.x, candidate.z, 0.6)
                    && map.capsule_floor(candidate, 0.42).is_some()
                    && flat(candidate - door.position).length() >= 1.25
                {
                    position = candidate;
                    break 'placement;
                }
            }
        }
        position.y = map.capsule_floor(position, 0.42)?;
        Some(Self::at(room.id, position, map.seed, Tint::Blue))
    }

    /// A slime dozing at `position` in `room`; `salt` varies its rhythm.
    pub fn at(room: usize, position: Vec3, salt: u64, tint: Tint) -> Self {
        Self {
            room,
            home: position,
            position,
            health: MAX_HEALTH,
            mode: Mode::Sleep,
            mode_time: 0.0,
            time: hash(salt) * 10.0,
            cooldown: 1.2,
            idle_hop: 2.0 + hash(salt ^ 0x5151) * 2.0,
            hop: None,
            lift: 0.0,
            squash: 1.0,
            squash_velocity: 0.0,
            squash_target: 1.0,
            jiggle: 0.3,
            glow: 0.0,
            yaw: 0.0,
            lean: [0.0; 2],
            wound_at: Vec3::new(0.0, 0.3, 0.4),
            wound_time: -10.0,
            particles: Vec::new(),
            burst: false,
            tint,
            landed: false,
        }
    }
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.mode_time = 0.0;
    }
    fn face(&mut self, target: Vec3, rate: f32, dt: f32) {
        let d = flat(target - self.position);
        if d.length_squared() < 0.0001 {
            return;
        }
        let want = d.x.atan2(d.z);
        let diff = (want - self.yaw).sin().atan2((want - self.yaw).cos());
        self.yaw += diff.clamp(-rate * dt, rate * dt);
    }
    fn hop_to(&mut self, map: &Blueprint, mut to: Vec3, height: f32, crouch: f32, air: f32) {
        let room = &map.rooms[self.room];
        to.y = self.home.y;
        for _ in 0..3 {
            if room.contains(to.x, to.z, 0.6)
                && map
                    .capsule_floor(to, 0.4)
                    .is_some_and(|y| (y - to.y).abs() < 0.32)
            {
                break;
            }
            to = to + (room.center - to) * 0.35;
        }
        if !room.contains(to.x, to.z, 0.6) {
            to = self.position;
        }
        self.hop = Some(Hop {
            from: self.position,
            to,
            height,
            crouch,
            air,
            elapsed: 0.0,
        });
    }
    fn splash(&mut self, count: usize, speed: f32, radius_scale: f32) {
        for k in 0..count {
            let salt = (self.time.to_bits() as u64) ^ ((k as u64 + 1) << 16);
            let angle = hash(salt) * std::f32::consts::TAU;
            let speed_var = 0.4 + hash(salt ^ 0x22aa) * 0.9;
            let radius = (0.025 + hash(salt ^ 0x5511) * 0.04) * radius_scale;
            self.particles.push(Particle {
                position: self.position + Vec3::Y * 0.17,
                velocity: Vec3::new(
                    angle.cos() * speed * speed_var,
                    speed * (0.8 + hash(salt ^ 0xbbb) * 0.9),
                    angle.sin() * speed * speed_var,
                ),
                radius,
                age: 0.0,
                life: 0.7 + hash(salt ^ 0x919) * 0.5,
                puddle: false,
            });
        }
        if self.particles.len() > 48 {
            self.particles.drain(0..self.particles.len() - 48);
        }
    }
    fn puddle(&mut self, radius: f32, life: f32) {
        self.particles.push(Particle {
            position: self.position + Vec3::Y * 0.012,
            velocity: Vec3::ZERO,
            radius,
            age: 0.0,
            life,
            puddle: true,
        });
    }
    fn land(&mut self, strength: f32) {
        self.squash_velocity -= strength;
        self.jiggle = (self.jiggle + strength * 0.25).min(1.6);
        self.puddle(0.18 + strength * 0.05, 3.5);
    }
    pub fn hit(&mut self, map: &Blueprint, damage: f32, direction: Vec3) {
        if !self.alive() {
            return;
        }
        self.health = (self.health - damage).max(0.0);
        self.wound_at = Vec3::new(0.0, 0.32, 0.39);
        self.wound_time = self.time;
        self.glow = 1.0;
        self.jiggle = 1.6;
        self.squash_velocity -= 2.4;
        self.splash(5, 1.2, 0.8);
        if self.alive() {
            self.hop_to(
                map,
                self.position + flat(direction).normalize_or_zero() * 0.6,
                0.16,
                0.02,
                0.22,
            );
            self.set_mode(Mode::Hurt);
        } else {
            self.hop = None;
            self.set_mode(Mode::Dead);
        }
    }
    /// Returns landing damage. Combat decides how to apply and report it.
    pub fn tick(&mut self, map: &Blueprint, player: Vec3, player_room: usize, dt: f32) -> f32 {
        let dt = dt.clamp(0.0, 0.05);
        let prior = self.position;
        self.time += dt;
        self.mode_time += dt;
        self.cooldown = (self.cooldown - dt).max(0.0);
        self.glow = (self.glow - dt * 2.5).max(0.0);
        let delta = flat(player - self.position);
        let distance = delta.length();
        let same_level = (player.y - self.home.y).abs() < 1.6;
        let mut damage = 0.0;
        match self.mode {
            Mode::Dead => {
                self.squash_target = 0.3;
                if !self.burst && self.mode_time > 0.22 {
                    self.burst = true;
                    self.splash(18, 2.4, 1.3);
                    self.puddle(0.62, 9.0);
                }
            }
            Mode::Sleep => {
                self.squash_target = 1.0 + 0.035 * (self.time * 2.3).sin();
                self.idle_hop -= dt;
                if self.idle_hop <= 0.0 && self.hop.is_none() {
                    self.idle_hop = 2.4 + hash(map.seed ^ self.time.to_bits() as u64) * 1.8;
                    let a = hash(map.seed ^ self.idle_hop.to_bits() as u64) * std::f32::consts::TAU;
                    self.hop_to(
                        map,
                        self.home + Vec3::new(a.cos() * 0.45, 0.0, a.sin() * 0.45),
                        0.18,
                        0.16,
                        0.28,
                    );
                }
                if player_room == self.room && same_level && distance < WAKE {
                    self.set_mode(Mode::Wake);
                }
            }
            Mode::Wake => {
                self.squash_target = if self.mode_time < 0.25 { 1.3 } else { 1.0 };
                self.jiggle = 1.2;
                self.face(player, 6.0, dt);
                if self.mode_time > 0.6 {
                    self.set_mode(Mode::Chase);
                }
            }
            Mode::Chase => {
                self.squash_target = 1.0;
                self.face(player, 5.0, dt);
                if self.hop.is_none() {
                    if player_room == self.room
                        && same_level
                        && distance < REACH
                        && self.cooldown <= 0.0
                    {
                        self.set_mode(Mode::Telegraph);
                    } else if player_room == self.room && same_level && distance > 1.15 {
                        let step = (distance - 1.1).min(0.85);
                        self.hop_to(
                            map,
                            self.position + delta.normalize_or_zero() * step,
                            0.3,
                            0.14,
                            0.34,
                        );
                    }
                }
            }
            Mode::Telegraph => {
                self.squash_target = 0.62;
                self.glow = self.glow.max(0.6 * (self.mode_time / 0.4).min(1.0));
                self.face(player, 7.0, dt);
                if self.mode_time > 0.66 {
                    let reach = flat(player - self.position);
                    let travel = (reach.length() - CONTACT_DISTANCE).clamp(0.0, 3.2);
                    let target = self.position + reach.normalize_or_zero() * travel;
                    self.squash_velocity += 6.0;
                    self.hop_to(map, target, 0.85, 0.01, 0.42);
                    self.set_mode(Mode::Pounce);
                }
            }
            Mode::Pounce => {}
            Mode::Recover => {
                self.squash_target =
                    0.72 + 0.1 * (self.mode_time * 11.0).sin() * (-self.mode_time * 2.0).exp();
                if self.mode_time > 1.0 {
                    self.cooldown = 1.4;
                    self.set_mode(Mode::Chase);
                }
            }
            Mode::Hurt => {
                if self.hop.is_none() {
                    self.set_mode(Mode::Chase);
                }
            }
        }
        let mut landed = false;
        if let Some(hop) = &mut self.hop {
            hop.elapsed += dt;
            if hop.elapsed < hop.crouch {
                self.squash_target = 0.74;
            } else {
                let u = ((hop.elapsed - hop.crouch) / hop.air).clamp(0.0, 1.0);
                self.position = hop.from + (hop.to - hop.from) * u;
                self.position.y = self.home.y;
                self.lift = 4.0 * hop.height * u * (1.0 - u);
                self.squash_target = if u < 0.5 { 1.22 } else { 1.08 };
                landed = u >= 1.0;
            }
        } else {
            self.lift *= (-12.0 * dt).exp();
        }
        self.landed = landed;
        if landed {
            self.hop = None;
            self.lift = 0.0;
            if self.mode == Mode::Pounce {
                if player_room == self.room
                    && same_level
                    && flat(player - self.position).length() < CONTACT_DISTANCE + 0.2
                {
                    damage = DAMAGE;
                }
                self.splash(8, 1.6, 1.0);
                self.land(3.2);
                self.set_mode(Mode::Recover);
            } else {
                self.land(2.1);
            }
        }
        // Underdamped spring, identical coefficients to the web cave.
        self.squash_velocity +=
            ((self.squash_target - self.squash) * 170.0 - self.squash_velocity * 8.5) * dt;
        self.squash = (self.squash + self.squash_velocity * dt).clamp(0.35, 1.6);
        let want_jiggle = if self.mode == Mode::Telegraph {
            1.4
        } else {
            0.3
        };
        self.jiggle += (want_jiggle - self.jiggle) * (1.0 - (-dt * 3.0).exp());
        let velocity = (self.position - prior) * (1.0 / dt.max(0.0001));
        let (s, c) = self.yaw.sin_cos();
        let local_x = c * velocity.x - s * velocity.z;
        let local_z = s * velocity.x + c * velocity.z;
        let blend = 1.0 - (-dt * 8.0).exp();
        self.lean[0] += (-local_x * 0.035 - self.lean[0]) * blend;
        self.lean[1] += (-local_z * 0.035 - self.lean[1]) * blend;
        for p in &mut self.particles {
            p.age += dt;
            if !p.puddle {
                p.velocity.y -= 9.8 * dt;
                p.position += p.velocity * dt;
                if p.position.y < self.home.y + p.radius {
                    p.position.y = self.home.y + p.radius * 0.3;
                    p.velocity = Vec3::ZERO;
                }
            }
        }
        self.particles.retain(|p| p.age < p.life);
        damage
    }
    pub fn valid(&self, map: &Blueprint) -> bool {
        let finite = |v: Vec3| v.to_array().iter().all(|n| n.is_finite());
        self.room < map.rooms.len()
            && finite(self.position)
            && finite(self.home)
            && finite(self.wound_at)
            && self.hop.as_ref().is_none_or(|h| {
                finite(h.from)
                    && finite(h.to)
                    && [h.height, h.crouch, h.air, h.elapsed]
                        .iter()
                        .all(|n| n.is_finite())
                    && h.height >= 0.0
                    && h.crouch >= 0.0
                    && h.air > 0.0
                    && h.elapsed >= 0.0
            })
            && (0.0..=MAX_HEALTH).contains(&self.health)
            && [
                self.mode_time,
                self.time,
                self.cooldown,
                self.idle_hop,
                self.lift,
                self.squash,
                self.squash_velocity,
                self.squash_target,
                self.jiggle,
                self.glow,
                self.yaw,
                self.lean[0],
                self.lean[1],
                self.wound_time,
            ]
            .iter()
            .all(|n| n.is_finite())
            && (0.35..=1.6).contains(&self.squash)
            && self.particles.len() <= 48
            && self.particles.iter().all(|p| {
                finite(p.position)
                    && finite(p.velocity)
                    && p.radius.is_finite()
                    && p.radius > 0.0
                    && p.life.is_finite()
                    && p.life > 0.0
                    && p.age.is_finite()
                    && p.age >= 0.0
            })
            && map.rooms[self.room].contains(self.position.x, self.position.z, 0.4)
    }
}

const PROFILE: [[f32; 2]; 11] = [
    [0.0, 0.0],
    [0.3, 0.0],
    [0.4, 0.035],
    [0.448, 0.11],
    [0.452, 0.21],
    [0.425, 0.31],
    [0.365, 0.41],
    [0.28, 0.49],
    [0.18, 0.56],
    [0.095, 0.61],
    [0.0, HEIGHT],
];
fn sample_profile(i: usize, t: f32) -> [f32; 2] {
    let p0 = PROFILE[i.saturating_sub(1)];
    let p1 = PROFILE[i];
    let p2 = PROFILE[i + 1];
    let p3 = PROFILE[(i + 2).min(PROFILE.len() - 1)];
    let f = |d: usize| {
        0.5 * (2.0 * p1[d]
            + (p2[d] - p0[d]) * t
            + (2.0 * p0[d] - 5.0 * p1[d] + 4.0 * p2[d] - p3[d]) * t * t
            + (3.0 * p1[d] - p0[d] - 3.0 * p2[d] + p3[d]) * t * t * t)
    };
    [f(0).max(0.0), f(1).max(0.0)]
}
/// The source model's Catmull-Rom teardrop, with side lobes and swept crown.
pub fn body_mesh() -> Mesh3D {
    const SEG: usize = 40;
    const PER: usize = 4;
    let rings = (PROFILE.len() - 1) * PER + 1;
    let mut vertices = Vec::with_capacity(rings * (SEG + 1));
    let mut indices = Vec::with_capacity((rings - 1) * SEG * 6);
    for ring in 0..rings {
        let span = (ring / PER).min(PROFILE.len() - 2);
        let t = if ring == rings - 1 {
            1.0
        } else {
            (ring % PER) as f32 / PER as f32
        };
        let [radius, y] = sample_profile(span, t);
        let before = sample_profile(span, (t - 0.01).max(0.0));
        let after = sample_profile(span, (t + 0.01).min(1.0));
        let dr = after[0] - before[0];
        let dy = (after[1] - before[1]).max(0.0001);
        let h = y / HEIGHT;
        for seg in 0..=SEG {
            let angle = seg as f32 / SEG as f32 * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let lobe = 1.0
                + 0.03
                    * (3.0 * angle + 0.6).sin()
                    * (std::f32::consts::PI * (h * 1.2).min(1.0)).sin();
            let normal = Vec3::new(cos * dy, -dr, sin * dy).normalize_or_zero();
            let z = radius * sin * lobe - 0.075 * smoothstep(0.55, 1.0, h).powi(2);
            let mut vertex = MeshVertex3D::new(
                [radius * cos * lobe, y, z],
                normal.to_array(),
                [seg as f32 / SEG as f32, h],
            );
            let depth = (0.35 + 0.65 * (1.0 - h).powf(0.55)).clamp(0.0, 1.0);
            vertex.color = [0.47 + depth * 0.18, 0.78 + depth * 0.12, 1.0, 1.0];
            vertices.push(vertex);
        }
    }
    for ring in 0..rings - 1 {
        for seg in 0..SEG {
            let a = (ring * (SEG + 1) + seg) as u32;
            let b = a + 1;
            let c = a + (SEG + 1) as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D::rgb(1.0, 1.0, 1.0),
        metallic_factor: 0.0,
        roughness_factor: 0.07,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

pub fn mouth_mesh() -> Mesh3D {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for i in 0..=16 {
        let angle = std::f32::consts::PI * i as f32 / 16.0;
        let x = angle.cos() * 0.075;
        let y = 0.235 - angle.sin() * 0.028;
        for delta in [-0.008, 0.008] {
            vertices.push(MeshVertex3D::new(
                [x, y + delta, 0.392],
                [0.0, 0.0, 1.0],
                [0.0, 0.0],
            ));
        }
        if i < 16 {
            let a = (i * 2) as u32;
            indices.extend_from_slice(&[a, a + 1, a + 2, a + 1, a + 3, a + 2]);
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D::rgb(0.04, 0.06, 0.12),
        metallic_factor: 0.0,
        roughness_factor: 0.2,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

pub fn deform(base: &[MeshVertex3D], slime: &Slime) -> Vec<MeshVertex3D> {
    let inv = slime.squash.sqrt().recip();
    let age = slime.time - slime.wound_time;
    base.iter()
        .map(|vertex| {
            let mut v = *vertex;
            let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
            let n = Vec3::new(v.normal[0], v.normal[1], v.normal[2]);
            let h = (p.y / HEIGHT).clamp(0.0, 1.2);
            let free = smoothstep(0.0, 0.2, h);
            let angle = p.z.atan2(p.x);
            let ring = smoothstep(0.03, 0.16, (p.x * p.x + p.z * p.z).sqrt());
            let wave = (p.y * 9.0 - slime.time * 7.3 + angle * 2.0).sin() * 0.5 * ring
                + (p.x * 7.0 + slime.time * 5.1).sin() * 0.3
                + (p.z * 8.0 - slime.time * 6.2 + 1.3).sin() * 0.3;
            let d = (p - slime.wound_at).length();
            let ripple = if age >= 0.0 {
                (d * 24.0 - age * 28.0).sin() * (-d * 2.6 - age * 4.0).exp()
            } else {
                0.0
            };
            let moved = p + n * (slime.jiggle * wave * free * 0.045 + ripple * free * 0.084);
            v.position = [
                moved.x * inv + slime.lean[0] * h * h,
                moved.y * slime.squash + slime.lift,
                moved.z * inv + slime.lean[1] * h * h,
            ];
            let corrected = Vec3::new(
                n.x * slime.squash.sqrt(),
                n.y / slime.squash,
                n.z * slime.squash.sqrt(),
            )
            .normalize_or_zero();
            v.normal = corrected.to_array();
            v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};
    #[test]
    fn model_is_finite_and_closed() {
        let m = body_mesh();
        assert!(m.vertices.len() > 1000);
        assert!(m.indices.iter().all(|i| (*i as usize) < m.vertices.len()));
        assert!(m
            .vertices
            .iter()
            .all(|v| v.position.iter().all(|x| x.is_finite())));
    }
    #[test]
    fn slime_stays_inside_entrance_and_damage_has_a_tell() {
        let map = generate(
            42,
            &Config::default(),
            &Catalog::bundled(),
            &Difficulty::default(),
        )
        .unwrap();
        let mut slime = Slime::new(&map).unwrap();
        let player = slime.position + Vec3::new(0.0, 0.0, 1.6);
        let mut first_damage = None;
        for tick in 0..500 {
            if slime.tick(&map, player, map.entrance, 0.02) > 0.0 && first_damage.is_none() {
                first_damage = Some(tick);
            }
            assert!(map.rooms[slime.room].contains(slime.position.x, slime.position.z, 0.35));
            assert!(
                flat(player - slime.position).length() >= CONTACT_DISTANCE - 0.03,
                "slime entered player capsule on tick {tick}"
            );
        }
        assert!(first_damage.is_some_and(|tick| tick > 60));
    }
    #[test]
    fn spawn_is_reproducible_and_walkable_across_seeds() {
        let catalog = Catalog::bundled();
        let config = Config::default();
        for seed in 0..200 {
            let map = generate(seed, &config, &catalog, &Difficulty::default()).unwrap();
            let first = Slime::new(&map);
            assert_eq!(first, Slime::new(&map), "seed {seed}");
            if let Some(slime) = first {
                assert!(slime.valid(&map), "seed {seed}");
                assert!(
                    map.capsule_floor(slime.position, 0.42).is_some(),
                    "seed {seed}"
                );
                assert!((slime.position - map.spawn()).length() > 1.0, "seed {seed}");
            }
        }
    }
}

/// Blue and green slimes dozing beside the trees and mushrooms of the halls:
/// a few per dungeon, never in the entrance, a champion's or the Heart's room.
pub fn groves(map: &Blueprint, decor: &[super::decor::RoomDecor]) -> Vec<Slime> {
    use velvet_dungeon::Role;
    let mut out = Vec::new();
    for room in &map.rooms {
        if room.id == map.entrance
            || room.shape == Shape::Bridge
            || matches!(room.role, Role::Boss | Role::Miniboss | Role::Entrance)
        {
            continue;
        }
        let Some(dressing) = decor.get(room.id) else {
            continue;
        };
        let mut in_room = 0;
        for (k, prop) in dressing.props.iter().enumerate() {
            if in_room == 2 || out.len() == 12 {
                break;
            }
            let leafy = prop.asset.starts_with("trunk") || prop.asset.contains("mushroom");
            let salt = map.seed ^ ((room.id as u64) << 20) ^ (k as u64).wrapping_mul(0x9E37);
            if !leafy || hash(salt) > 0.4 {
                continue;
            }
            for step in 0..8 {
                let angle = hash(salt ^ 0xa11) * std::f32::consts::TAU + step as f32 * 0.785;
                let reach = prop.radius.max(0.3) + 0.95;
                let mut at = prop.position + Vec3::new(angle.cos(), 0.0, angle.sin()) * reach;
                let clear = room.contains(at.x, at.z, 0.7)
                    && room
                        .sockets
                        .iter()
                        .all(|s| flat(s.position - at).length() > 2.2)
                    && dressing
                        .props
                        .iter()
                        .all(|p| flat(p.position - at).length() > p.radius + 0.5)
                    && out.iter().all(|o: &Slime| flat(o.home - at).length() > 1.6);
                let Some(y) = map
                    .capsule_floor(at, 0.42)
                    .filter(|y| clear && (y - prop.position.y).abs() < 0.6)
                else {
                    continue;
                };
                at.y = y;
                let tint = if hash(salt ^ 0x6e) < 0.5 {
                    Tint::Blue
                } else {
                    Tint::Green
                };
                out.push(Slime::at(room.id, at, salt, tint));
                in_room += 1;
                break;
            }
        }
    }
    out
}
