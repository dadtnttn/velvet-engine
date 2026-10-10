//! El combate contra Varrak en primera persona (ACTO1.md §8; el usuario, 2026-10-10), portado del
//! prototipo (`demon-queen-penascal-threejs/src/combate/`): la plaza del Peñascal al caer la noche,
//! primero la manada y después el orco. La espada le rompe la guardia (o se la rompe un peñasco si
//! se le lleva a cargar a cuatro patas contra él) y, mientras está aturdido, el sello le manda la
//! chispa hasta llenar la barra y despertarlo. Cada cuarto de barra le arranca un recuerdo.
//!
//! Aquí está lo que pasa: la plaza y sus obstáculos (`data/penascal/combate.json`, exportado del
//! prototipo con `tools/export-combate.mjs`), la Reina, Varrak, la manada, lo que vuela y hace daño
//! y las frases de Chiquita. Lo que se ve lo dibuja `zones/boss.rs`; el HUD, `bin/boss_overlay.rs`.
use super::*;
use dungeon::combat::Fx;
use dungeon::fp;
use dungeon::sound::{Sfx, Sound};

/// The model is 0.71 m tall: 3.3 m in the fight.
pub(crate) const VARRAK_SCALE: f32 = 4.6;
/// Spark per second while it flows (a quarter takes ~3.6 s of a stun).
const SPARK_RATE: f32 = 7.0;
const STUN_SECONDS: f32 = 5.2;
const CRASH_STUN_SECONDS: f32 = 6.0;
const GALLOP_SPEED: f32 = 10.5;
/// Her eyes above her feet.
pub(crate) const EYE: f32 = 1.62;
const GRAVITY: f32 = 18.0;

fn wrap(a: f32) -> f32 {
    a.sin().atan2(a.cos())
}

fn smoothstep(x: f32, a: f32, b: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A small deterministic random source (the fight replays the same from the same seed).
#[derive(Clone, Debug)]
pub(crate) struct Rng(u32);

impl Rng {
    pub(crate) fn new(seed: u32) -> Self {
        Self(seed | 1)
    }
    /// 0..1.
    pub(crate) fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x >> 8) as f32 / 16_777_216.0
    }
}

/// The words the seal tears loose (ACTO1.md §8), at a quarter, a half and three quarters of the bar.
pub(crate) struct Memory {
    pub at: f32,
    pub word: &'static str,
    pub chiquita: &'static str,
    /// Its name on the bar's notch.
    pub label: &'static str,
}

pub(crate) const MEMORIES: [Memory; 3] = [
    Memory {
        at: 25.0,
        word: "…puer… ta…",
        chiquita: "Se acuerda de la puerta. Sigue.",
        label: "puerta",
    },
    Memory {
        at: 50.0,
        word: "…a… jedrez… no… toca…",
        chiquita: "Le tocaba mover a él. Mentira. Me tocaba a mí.",
        label: "ajedrez",
    },
    Memory {
        at: 75.0,
        word: "…Ma… jes… tad…",
        chiquita: "Ya casi. No te pares ahora, que se te apaga.",
        label: "Majestad",
    },
];

// ------------------------------------------------------------------ the plan
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Oval {
    pub rx: f32,
    pub rz: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Throne {
    pub x: f32,
    pub z: f32,
    #[allow(dead_code)]
    pub w: f32,
    pub d: f32,
    #[allow(dead_code)]
    pub h: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Spot {
    pub x: f32,
    pub z: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Start {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Breakable {
    pub kind: String,
    pub x: f32,
    pub z: f32,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Lamp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: String,
    pub power: f32,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Flame {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: String,
    pub size: f32,
    pub kind: String,
}

/// The plaza of the fight, as the prototype exports it.
#[derive(Deserialize)]
pub(crate) struct Plan {
    /// The cliffs' foot: an ellipse.
    pub arena: Oval,
    /// The market stone he sits on (facing south, at the gate she comes in by).
    pub throne: Throne,
    /// His cleaver, driven into the ground at his right hand.
    #[serde(rename = "cleaverSpot")]
    pub cleaver_spot: Spot,
    pub start: Start,
    /// The doorways the pack comes out of.
    pub dens: Vec<Spot>,
    /// [x, z, r, breakable or -1].
    pub circles: Vec<[f32; 4]>,
    /// [x, z, hx, hz, breakable or -1].
    pub boxes: Vec<[f32; 5]>,
    pub breakables: Vec<Breakable>,
    pub lamps: Vec<Lamp>,
    pub flames: Vec<Flame>,
    /// The cliff's height all round (360 samples from angle 0, east, towards the south).
    pub cliff: Vec<f32>,
}

pub(crate) fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("penascal/combate.json")).expect("valid fight plan")
    })
}

/// Whether the fight has been exported into the game.
pub(crate) fn available() -> bool {
    exploration3d_state::data_dir()
        .join("penascal")
        .join("combate.json")
        .exists()
        && runtime_3d_root()
            .join("penascal")
            .join("combate_static.glb")
            .exists()
}

// ------------------------------------------------------------------ the arena
/// The outward normal of the oval at angle `a`.
pub(crate) fn ring_normal(a: f32) -> (f32, f32) {
    let o = plan().arena;
    let (x, z) = (a.cos() / o.rx, a.sin() / o.rz);
    let l = x.hypot(z).max(1e-6);
    (x / l, z / l)
}

/// A point on the cliffs' foot at angle `a` (0 east, π/2 south), `inset` in towards the middle.
pub(crate) fn ring_point(a: f32, inset: f32) -> (f32, f32) {
    let o = plan().arena;
    let n = ring_normal(a);
    (o.rx * a.cos() - n.0 * inset, o.rz * a.sin() - n.1 * inset)
}

/// The angle (the ellipse's parameter) of a point's direction from the middle.
pub(crate) fn angle_of(x: f32, z: f32) -> f32 {
    let o = plan().arena;
    (z / o.rz).atan2(x / o.rx)
}

fn inside_ring(x: f32, z: f32, r: f32) -> bool {
    let (fx, fz) = ring_point(angle_of(x, z), 0.9 + r);
    x * x + z * z < fx * fx + fz * fz
}

/// The cliff's height at angle `a`.
pub(crate) fn cliff_height(a: f32) -> f32 {
    let c = &plan().cliff;
    if c.is_empty() {
        return 10.5;
    }
    let f = a.rem_euclid(std::f32::consts::TAU) / std::f32::consts::TAU * c.len() as f32;
    let i = f.floor() as usize % c.len();
    let k = f - f.floor();
    c[i] * (1.0 - k) + c[(i + 1) % c.len()] * k
}

/// What a body ran into.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    None,
    Cliff,
    Prop,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Collision {
    pub x: f32,
    pub z: f32,
    pub hit: Hit,
}

/// What stands in the plaza now: the breakable things still whole.
#[derive(Clone, Debug)]
pub(crate) struct Arena {
    pub broken: Vec<bool>,
}

impl Arena {
    pub(crate) fn new() -> Self {
        Self {
            broken: vec![false; plan().breakables.len()],
        }
    }

    fn on(&self, b: f32) -> bool {
        b < 0.0 || !self.broken.get(b as usize).copied().unwrap_or(true)
    }

    /// Pushes a body of radius `r` at (x, z) out of what it is in: the cliffs, stones, stalls.
    pub(crate) fn collide(&self, x: f32, z: f32, r: f32) -> Collision {
        let p = plan();
        let (mut x, mut z) = (x, z);
        let mut hit = Hit::None;
        if !inside_ring(x, z, r) {
            let (fx, fz) = ring_point(angle_of(x, z), 0.9 + r + 0.01);
            x = fx;
            z = fz;
            hit = Hit::Cliff;
        }
        for c in &p.circles {
            if !self.on(c[3]) {
                continue;
            }
            let (dx, dz) = (x - c[0], z - c[1]);
            let d = dx.hypot(dz);
            let m = c[2] + r;
            if d < m && d > 1e-6 {
                x = c[0] + dx / d * m;
                z = c[1] + dz / d * m;
                if hit == Hit::None {
                    hit = Hit::Prop;
                }
            }
        }
        for b in &p.boxes {
            if !self.on(b[4]) {
                continue;
            }
            let (dx, dz) = (x - b[0], z - b[1]);
            let px = b[2] + r - dx.abs();
            let pz = b[3] + r - dz.abs();
            if px > 0.0 && pz > 0.0 {
                let sign = |v: f32| if v < 0.0 { -1.0 } else { 1.0 };
                if px < pz {
                    x += sign(dx) * px;
                } else {
                    z += sign(dz) * pz;
                }
                if hit == Hit::None {
                    hit = Hit::Prop;
                }
            }
        }
        Collision { x, z, hit }
    }

    /// The breakable things a body of radius `r` at (x, z) touches (a charge goes through them).
    pub(crate) fn touching(&self, x: f32, z: f32, r: f32) -> Vec<usize> {
        let p = plan();
        (0..p.breakables.len())
            .filter(|&i| !self.broken[i])
            .filter(|&i| {
                p.circles
                    .iter()
                    .any(|c| c[3] as i32 == i as i32 && (x - c[0]).hypot(z - c[1]) < c[2] + r)
                    || p.boxes.iter().any(|b| {
                        b[4] as i32 == i as i32
                            && (x - b[0]).abs() < b[2] + r
                            && (z - b[1]).abs() < b[3] + r
                    })
            })
            .collect()
    }
}

// ------------------------------------------------------------------ the Queen
/// What she does this step.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Input {
    /// -1..1: back/forward, left/right.
    pub forward: f32,
    pub strafe: f32,
    pub run: bool,
    /// Pressed this step.
    pub jump: bool,
    pub attack: bool,
    /// Held: the seal.
    pub channel: bool,
}

/// The Queen in first person: where she stands and looks, her vitality, her sword and her seal.
#[derive(Clone, Debug)]
pub(crate) struct Player {
    pub x: f32,
    pub z: f32,
    pub y: f32,
    vy: f32,
    /// 0 looking north (-z); the mouse turns it.
    pub yaw: f32,
    pub pitch: f32,
    push: (f32, f32),
    pub health: f32,
    pub max_health: f32,
    untouchable: f32,
    pub time: f32,
    pub channeling: bool,
    pub frozen: bool,
    /// Blows land (only while the fight is on).
    vulnerable: bool,
    /// She cannot fall (QA).
    pub god: bool,
    shake_time: f32,
    shake_amp: f32,
    /// The camera's flinch: pitch, yaw and roll offsets.
    pub shake: Vec3,
    /// The swing on (its combo index and time into it), where it started from, its pose now.
    pub swing: Option<(usize, f32)>,
    pub from: fp::Pose,
    pub pose: fp::Pose,
    queued: bool,
    struck: bool,
    hit_stop: f32,
    /// 0 at rest, 1 thrust out sending the spark.
    pub cast: f32,
    /// The seal's glow (brighter while the spark flows).
    pub glow: f32,
    pub moving: bool,
    pub running: bool,
    /// The walk's bob of her eyes.
    pub bob: f32,
    /// The blows taken this step (for the fight's reactions).
    hurts: Vec<f32>,
}

impl Player {
    fn new(x: f32, z: f32, yaw: f32) -> Self {
        Self {
            x,
            z,
            y: 0.0,
            vy: 0.0,
            yaw,
            pitch: 0.0,
            push: (0.0, 0.0),
            health: 100.0,
            max_health: 100.0,
            untouchable: 0.0,
            time: 0.0,
            channeling: false,
            frozen: false,
            vulnerable: false,
            god: false,
            shake_time: 0.0,
            shake_amp: 0.09,
            shake: Vec3::ZERO,
            swing: None,
            from: fp::REST,
            pose: fp::REST,
            queued: false,
            struck: false,
            hit_stop: 0.0,
            cast: 0.0,
            glow: 0.0,
            moving: false,
            running: false,
            bob: 0.0,
            hurts: Vec::new(),
        }
    }

    /// The mouse moved (pixels).
    pub(crate) fn look(&mut self, dx: f32, dy: f32) {
        if self.frozen {
            return;
        }
        let (dx, dy) = (dx.clamp(-140.0, 140.0), dy.clamp(-140.0, 140.0));
        self.yaw -= dx * 0.0022;
        self.pitch = (self.pitch - dy * 0.0021).clamp(-1.35, 1.35);
    }

    /// Where she looks: a unit vector.
    pub(crate) fn forward(&self) -> Vec3 {
        Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub(crate) fn eye(&self) -> Vec3 {
        Vec3::new(self.x, self.y + EYE, self.z)
    }

    /// A blow: `amount` of vitality, pushed away from (fx, fz) by `force`. Whether it landed.
    pub(crate) fn hurt(&mut self, amount: f32, fx: f32, fz: f32, force: f32) -> bool {
        if !self.vulnerable || self.untouchable > 0.0 || self.health <= 0.0 {
            return false;
        }
        if !self.god {
            self.health = (self.health - amount).max(0.0);
        }
        self.untouchable = 0.55;
        let (dx, dz) = (self.x - fx, self.z - fz);
        let d = dx.hypot(dz).max(1e-3);
        self.push = (dx / d * force, dz / d * force);
        self.kick(0.32, 0.16);
        self.hurts.push(amount);
        true
    }

    pub(crate) fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(self.max_health);
    }

    /// The camera flinches (a blow, a smash landing nearby).
    pub(crate) fn kick(&mut self, seconds: f32, amp: f32) {
        self.shake_amp = if self.shake_time > 0.0 {
            amp.max(self.shake_amp)
        } else {
            amp
        };
        self.shake_time = self.shake_time.max(seconds);
    }

    /// Start (or chain) a swing.
    fn attack(&mut self) {
        if self.frozen {
            return;
        }
        match self.swing {
            Some((combo, t)) => {
                let s = fp::SWINGS[combo];
                if s.chain > 0.0 && t > s.strike && t < s.chain {
                    self.queued = true;
                }
            }
            None => self.start_swing(0),
        }
    }

    fn start_swing(&mut self, combo: usize) {
        self.swing = Some((combo, 0.0));
        self.from = self.pose;
        self.struck = false;
        self.queued = false;
    }

    /// Freeze the swing for a beat when the blade bites.
    fn impact(&mut self, seconds: f32) {
        self.hit_stop = seconds;
    }

    /// One step: walking, jumping, the sword's swing and the seal's thrust. The swing's power on
    /// the frame its blade crosses the middle of the screen.
    fn update(&mut self, input: Input, dt: f32, arena: &Arena) -> Option<f32> {
        self.time += dt;
        self.hurts.clear();
        let free = !self.frozen;
        let (fwd, side) = if free {
            (input.forward, input.strafe)
        } else {
            (0.0, 0.0)
        };
        self.moving = fwd != 0.0 || side != 0.0;
        self.channeling = free && input.channel;
        let run = free && input.run && !self.channeling;
        self.running = run && self.moving;
        let speed = if self.channeling {
            2.4
        } else if run {
            6.8
        } else {
            4.3
        };
        let (s, c) = self.yaw.sin_cos();
        let (mut mx, mut mz) = (-s * fwd + c * side, -c * fwd - s * side);
        let len = mx.hypot(mz);
        if len > 0.0 {
            mx = mx / len * speed * dt;
            mz = mz / len * speed * dt;
        }
        mx += self.push.0 * dt;
        mz += self.push.1 * dt;
        let k = (-dt * 7.0).exp();
        self.push = (self.push.0 * k, self.push.1 * k);
        let to = arena.collide(self.x + mx, self.z + mz, 0.38);
        self.x = to.x;
        self.z = to.z;
        if free && input.jump && self.y <= 0.0 {
            self.vy = 6.4;
        }
        // Jumping: one floor (the plaza is flat).
        if self.y > 0.0 || self.vy > 0.0 {
            self.vy -= 19.0 * dt;
            self.y += self.vy * dt;
            if self.y <= 0.0 {
                self.y = 0.0;
                self.vy = 0.0;
            }
        }
        self.untouchable = (self.untouchable - dt).max(0.0);
        self.bob = if self.moving && self.y <= 0.0 {
            (self.time * if run { 15.0 } else { 10.0 }).sin() * 0.014
        } else {
            0.0
        };
        if free && input.attack {
            self.attack();
        }
        let strike = self.update_sword(dt);
        if self.shake_time > 0.0 {
            self.shake_time = (self.shake_time - dt).max(0.0);
            let k = self.shake_time * self.shake_amp;
            let t = self.time * 61.0;
            self.shake = Vec3::new(
                (t * 1.3).sin() * 0.5 * k,
                (t * 1.7).cos() * 0.5 * k,
                (t * 2.3).sin() * 0.3 * k,
            );
        } else {
            self.shake = Vec3::ZERO;
        }
        strike
    }

    fn update_sword(&mut self, dt: f32) -> Option<f32> {
        let mut step = dt;
        if self.hit_stop > 0.0 {
            step = 0.0;
            self.hit_stop -= dt;
        }
        let mut strike = None;
        if let Some((combo, t)) = self.swing {
            let t = t + step;
            let s = fp::SWINGS[combo];
            self.pose = fp::pose(combo, t.min(s.duration), self.from);
            self.swing = Some((combo, t));
            if !self.struck && t >= s.strike {
                self.struck = true;
                strike = Some(if combo == 2 { 1.45 } else { 1.0 });
            }
            if self.queued && t >= fp::CHAIN_COMMIT {
                self.start_swing((combo + 1) % fp::SWINGS.len());
            } else if t >= s.duration {
                self.swing = None;
            }
        } else {
            // Idle: breathe; walking: bob and sway with the stride.
            let pace = if self.running { 13.0 } else { 9.5 };
            let bob = if self.moving {
                (self.time * pace).sin()
            } else {
                0.0
            };
            let target = fp::REST.hand
                + Vec3::new(
                    bob * 0.012,
                    if self.moving { bob.abs() * -0.016 } else { 0.0 }
                        + (self.time * 1.6).sin() * 0.004,
                    0.0,
                );
            self.pose.hand = self.pose.hand.lerp(target, 1.0 - (-dt * 14.0).exp());
            self.pose.blade = self
                .pose
                .blade
                .lerp(fp::REST.blade, 1.0 - (-dt * 12.0).exp())
                .normalize_or_zero();
            self.pose.roll += (fp::REST.roll - self.pose.roll) * (1.0 - (-dt * 12.0).exp());
        }
        let want = if self.channeling { 1.0 } else { 0.0 };
        self.cast += (want - self.cast) * (1.0 - (-dt * 12.0).exp());
        strike
    }
}

// ------------------------------------------------------------------ Varrak
/// His clips (`tools/varrak_blender.py`), by the name in the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Clip {
    Sit,
    SitRoar,
    Rise,
    Idle,
    Walk,
    Roar,
    Smash,
    Sweep,
    Stun,
    Hit,
    Memory,
    Kneel,
    Drop,
    Gallop,
}

impl Clip {
    pub(crate) const ALL: [Clip; 14] = [
        Clip::Sit,
        Clip::SitRoar,
        Clip::Rise,
        Clip::Idle,
        Clip::Walk,
        Clip::Roar,
        Clip::Smash,
        Clip::Sweep,
        Clip::Stun,
        Clip::Hit,
        Clip::Memory,
        Clip::Kneel,
        Clip::Drop,
        Clip::Gallop,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Clip::Sit => "Varrak_Sit",
            Clip::SitRoar => "Varrak_SitRoar",
            Clip::Rise => "Varrak_Rise",
            Clip::Idle => "Varrak_Idle",
            Clip::Walk => "Varrak_Walk",
            Clip::Roar => "Varrak_Roar",
            Clip::Smash => "Varrak_Smash",
            Clip::Sweep => "Varrak_Sweep",
            Clip::Stun => "Varrak_Stun",
            Clip::Hit => "Varrak_Hit",
            Clip::Memory => "Varrak_Memory",
            Clip::Kneel => "Varrak_Kneel",
            Clip::Drop => "Varrak_Drop",
            Clip::Gallop => "Varrak_Gallop",
        }
    }

    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|c| *c == self).unwrap_or(0)
    }

    /// Loops (else it plays once and holds its last frame).
    pub(crate) fn looping(self) -> bool {
        matches!(
            self,
            Clip::Sit | Clip::Idle | Clip::Walk | Clip::Stun | Clip::Gallop
        )
    }
}

/// Which clip plays and how far in, and the one it fades from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Anim {
    pub clip: Clip,
    pub t: f32,
    pub speed: f32,
    pub backwards: bool,
    pub from: Option<(Clip, f32, bool)>,
    /// 0..1 into the cross-fade.
    pub fade: f32,
    fade_len: f32,
}

impl Anim {
    fn new(clip: Clip) -> Self {
        Self {
            clip,
            t: 0.0,
            speed: 1.0,
            backwards: false,
            from: None,
            fade: 1.0,
            fade_len: 0.0,
        }
    }

    /// Fade to a clip; `backwards` plays a one-shot from its end (Drop backwards: getting up).
    fn play(&mut self, clip: Clip, fade: f32, speed: f32, backwards: bool) {
        if self.clip == clip && !backwards {
            self.speed = speed;
            return;
        }
        self.from = Some((self.clip, self.t, self.backwards));
        self.clip = clip;
        self.t = 0.0;
        self.speed = speed;
        self.backwards = backwards;
        self.fade_len = fade.max(0.0);
        self.fade = if fade > 0.0 { 0.0 } else { 1.0 };
    }

    fn advance(&mut self, dt: f32) {
        self.t += dt * self.speed;
        if let Some((_, t, _)) = &mut self.from {
            *t += dt;
        }
        self.fade = if self.fade_len > 0.0 {
            (self.fade + dt / self.fade_len).min(1.0)
        } else {
            1.0
        };
        if self.fade >= 1.0 {
            self.from = None;
        }
    }

    /// Seconds into `clip` to sample for playback time `t` (`duration`: its length).
    pub(crate) fn sample(clip: Clip, t: f32, backwards: bool, duration: f32) -> f32 {
        if backwards {
            (duration - t).max(0.0)
        } else if clip.looping() {
            t
        } else {
            t.min((duration - 1e-3).max(0.0))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VState {
    Throne,
    ThroneRoar,
    Rise,
    Roar,
    Chase,
    Hit,
    Smash,
    Sweep,
    Drop,
    Gallop,
    GetUp,
    Dazed,
    Recover,
    Stunned,
    Memory,
    Kneel,
    Awake,
}

/// Where his cleaver is: driven into the ground, in his fist, on his back (on all fours), dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BladeMode {
    Ground,
    Hand,
    Back,
    Dropped,
}

/// Why he was stunned (Chiquita says something different).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Why {
    Guard,
    Spire,
    Cliff,
}

#[derive(Clone, Copy, Debug, Default)]
struct Gallop {
    locked: bool,
    left: f32,
    hit_her: bool,
}

/// What happens in the fight that the fight answers (sounds, words, effects, broken things).
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Sfx(Sfx, Option<Vec3>, f32),
    /// A word over his head: its text, whether a growl (else a memory), how long.
    Float(&'static str, bool, f32),
    Chiquita(&'static str, f32, f32),
    Burst(Vec3, [f32; 3], usize, f32),
    Dust(f32, f32),
    Ring(f32, f32, f32),
    Smash(f32, f32),
    Fours(u32),
    BreakSpire(usize),
    BreakProp(usize),
    RamCliff(f32, f32),
    Stunned(Why),
    Memory(usize),
    Awake,
}

/// Varrak, el Colmillo: where he is, what he does, his guard and the spark in him.
#[derive(Clone, Debug)]
pub(crate) struct Varrak {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub state: VState,
    pub t: f32,
    /// 0..100: the sword (or a rock) takes it; at 0 he is stunned.
    pub guard: f32,
    /// 0..100: the spark sent; full, he wakes.
    pub spark: f32,
    pub memories: usize,
    spark_this_stun: f32,
    stun_left: f32,
    cooldown: f32,
    charge_ready: f32,
    struck: bool,
    gallop: Gallop,
    /// On his feet (the fight has really begun).
    pub awake: bool,
    charges: u32,
    recover: f32,
    pub anim: Anim,
    pub blade: BladeMode,
    /// The cleaver flying to where it goes next (seconds left).
    pub blade_fly: f32,
}

impl Varrak {
    fn new() -> Self {
        let t = plan().throne;
        Self {
            x: t.x,
            z: t.z,
            yaw: 0.0,
            state: VState::Throne,
            t: 0.0,
            guard: 100.0,
            spark: 0.0,
            memories: 0,
            spark_this_stun: 0.0,
            stun_left: 0.0,
            cooldown: 1.5,
            charge_ready: 5.0,
            struck: false,
            gallop: Gallop::default(),
            awake: false,
            charges: 0,
            recover: 0.7,
            anim: Anim::new(Clip::Sit),
            blade: BladeMode::Ground,
            blade_fly: 0.0,
        }
    }

    /// Where his root stands (on the stone while he sits on it).
    pub(crate) fn root(&self) -> Vec3 {
        let t = plan().throne;
        let z = if matches!(self.state, VState::Throne | VState::ThroneRoar) {
            t.z + 0.18
        } else {
            self.z
        };
        Vec3::new(self.x, 0.0, z)
    }

    fn set(&mut self, state: VState) {
        self.state = state;
        self.t = 0.0;
        self.struck = false;
    }

    pub(crate) fn fighting(&self) -> bool {
        !matches!(
            self.state,
            VState::Throne | VState::ThroneRoar | VState::Rise | VState::Kneel | VState::Awake
        )
    }

    pub(crate) fn on_fours(&self) -> bool {
        matches!(self.state, VState::Drop | VState::Gallop | VState::GetUp)
    }

    /// The seal can send him the spark.
    pub(crate) fn open(&self) -> bool {
        matches!(self.state, VState::Stunned | VState::Memory)
    }

    /// His chest when nothing better is known (the renderer sends the emblem's place).
    fn chest_guess(&self) -> Vec3 {
        let r = self.root();
        let h = if self.on_fours() {
            1.4
        } else if self.open() {
            2.1
        } else {
            2.45
        };
        Vec3::new(r.x + self.yaw.sin() * 0.6, h, r.z + self.yaw.cos() * 0.6)
    }

    /// The fight begins: he lifts his head on the stone and roars; the pack comes.
    fn wake(&mut self, ev: &mut Vec<Event>) {
        if self.state != VState::Throne {
            return;
        }
        self.set(VState::ThroneRoar);
        self.anim.play(Clip::SitRoar, 0.3, 1.0, false);
        ev.push(Event::Sfx(Sfx::Roar, Some(self.root()), 1.0));
        ev.push(Event::Float("*ruge*", true, 2.0));
    }

    /// The pack is down: he stands, takes his cleaver, and comes.
    fn rise(&mut self) {
        if !matches!(self.state, VState::Throne | VState::ThroneRoar) {
            return;
        }
        self.set(VState::Rise);
        self.anim.play(Clip::Rise, 0.35, 1.0, false);
    }

    fn turn_to(&mut self, px: f32, pz: f32, rate: f32, dt: f32) -> f32 {
        let want = (px - self.x).atan2(pz - self.z);
        let diff = wrap(want - self.yaw);
        self.yaw += diff.clamp(-rate * dt, rate * dt);
        diff.abs()
    }

    /// The angle of her off his facing (+ to his left).
    fn off_facing(&self, px: f32, pz: f32) -> f32 {
        wrap((px - self.x).atan2(pz - self.z) - self.yaw)
    }

    fn step(&mut self, arena: &Arena, speed: f32, dt: f32, r: f32) -> Option<Collision> {
        let (nx, nz) = (
            self.x + self.yaw.sin() * speed * dt,
            self.z + self.yaw.cos() * speed * dt,
        );
        let c = arena.collide(nx, nz, r);
        let blocked = (c.x - nx).hypot(c.z - nz) > 0.02;
        self.x = c.x;
        self.z = c.z;
        blocked.then_some(c)
    }

    /// The sword reaches him? `eye`, `dir` from her. Whether it struck him.
    fn strike(&mut self, eye: Vec3, dir: Vec3, power: f32, ev: &mut Vec<Event>) -> bool {
        let (dx, dz) = (self.x - eye.x, self.z - eye.z);
        let d = dx.hypot(dz).max(1e-3);
        if d - 1.15 > 2.5 {
            return false;
        }
        let flat = dir.x.hypot(dir.z).max(1e-3);
        if (dx * dir.x + dz * dir.z) / (d * flat) < 0.75f32.cos() {
            return false;
        }
        if !self.fighting() {
            return false;
        }
        let at = Vec3::new(
            self.x - dx / d,
            if self.on_fours() { 1.0 } else { 1.7 },
            self.z - dz / d,
        );
        if matches!(self.state, VState::Stunned | VState::Memory | VState::Dazed) {
            ev.push(Event::Sfx(Sfx::HitSlime, Some(at), 0.9));
            ev.push(Event::Burst(at, [0.85, 0.78, 1.0], 14, 4.0));
            return true;
        }
        let mut loss = 9.0 * power;
        // A blow in the opening after his own swing bites deeper.
        if (self.state == VState::Smash && self.t > 1.0)
            || (self.state == VState::Sweep && self.t > 0.9)
            || matches!(self.state, VState::Recover | VState::GetUp)
        {
            loss *= 1.5;
        }
        self.guard = (self.guard - loss).max(0.0);
        ev.push(Event::Sfx(Sfx::HitBone, Some(at), 1.0));
        ev.push(Event::Burst(at, [1.0, 0.81, 0.54], 16, 4.0));
        if self.guard <= 0.0 {
            self.stun(STUN_SECONDS, Why::Guard, ev);
        } else if matches!(self.state, VState::Chase | VState::Recover) {
            self.set(VState::Hit);
            self.anim.play(Clip::Hit, 0.08, 1.0, false);
        }
        true
    }

    fn stun(&mut self, seconds: f32, why: Why, ev: &mut Vec<Event>) {
        self.set(VState::Stunned);
        self.stun_left = seconds;
        self.spark_this_stun = 0.0;
        self.guard = 0.0;
        self.anim.play(Clip::Stun, 0.25, 1.0, false);
        ev.push(Event::Sfx(Sfx::BoneRattle, Some(self.root()), 1.0));
        ev.push(Event::Float("…grr…", true, 2.0));
        ev.push(Event::Stunned(why));
    }

    /// The spark from the seal: flows only into a stunned (or remembering) Varrak in front of her.
    /// (whether it flows, whether he is open to it).
    fn channel(
        &mut self,
        dt: f32,
        eye: Vec3,
        dir: Vec3,
        chest: Vec3,
        ev: &mut Vec<Event>,
    ) -> (bool, bool) {
        let to = chest - eye;
        let d = to.length().max(1e-3);
        let aim = (to * (1.0 / d)).dot(dir);
        let open = self.open();
        if !open || d > 14.0 || aim < 0.93 {
            return (false, open);
        }
        if self.state == VState::Memory {
            return (true, open);
        }
        let before = self.spark;
        let next = MEMORIES.get(self.memories);
        self.spark = (self.spark + SPARK_RATE * dt).min(next.map_or(100.0, |m| m.at));
        self.spark_this_stun += self.spark - before;
        // A memory torn loose at each quarter.
        if let Some(m) = next {
            if self.spark >= m.at {
                self.memories += 1;
                self.set(VState::Memory);
                self.anim.play(Clip::Memory, 0.3, 1.0, false);
                ev.push(Event::Memory(self.memories));
            }
        } else if self.spark >= 100.0 {
            self.set(VState::Kneel);
            self.anim.play(Clip::Kneel, 0.4, 1.0, false);
            ev.push(Event::Sfx(Sfx::SealOpen, Some(self.root()), 1.0));
            // The cleaver falls from his hand.
            self.blade = BladeMode::Dropped;
        }
        (true, open)
    }

    fn start_drop(&mut self, ev: &mut Vec<Event>) {
        // Down on all fours: the cleaver goes on his back, he scrapes the ground and snorts.
        self.set(VState::Drop);
        self.anim.play(Clip::Drop, 0.22, 1.0, false);
        self.blade = BladeMode::Back;
        self.blade_fly = 0.3;
        ev.push(Event::Sfx(Sfx::Rubble, Some(self.root()), 0.35));
        self.charges += 1;
        ev.push(Event::Fours(self.charges));
    }

    fn get_up(&mut self, after: f32) {
        self.set(VState::GetUp);
        self.recover = after;
        self.anim.play(Clip::Drop, 0.2, 1.2, true);
    }

    fn after(&mut self, seconds: f32, rng: &mut Rng) {
        self.set(VState::Recover);
        self.recover = seconds;
        self.cooldown = 0.6 + rng.next() * 0.9;
        self.anim.play(Clip::Idle, 0.3, 1.0, false);
    }

    /// The end of a daze: he shakes his head, roars and comes on with his guard back.
    fn shake_off(&mut self, remembered: bool, ev: &mut Vec<Event>) {
        // If nobody sent him spark he gets some of his strength back.
        let left = !remembered && self.spark_this_stun < 4.0;
        if left {
            let floor = if self.memories > 0 {
                MEMORIES[self.memories - 1].at
            } else {
                0.0
            };
            self.spark = (self.spark - 5.0).max(floor);
        }
        self.guard = 100.0;
        if self.blade == BladeMode::Back {
            self.blade = BladeMode::Hand;
            self.blade_fly = 0.3;
        }
        self.set(VState::Roar);
        self.anim.play(Clip::Roar, 0.3, 1.0, false);
        ev.push(Event::Sfx(Sfx::Roar, Some(self.root()), 1.0));
        ev.push(Event::Float("*ruge*", true, 2.0));
        if left {
            ev.push(Event::Chiquita(
                "Lo has dejado respirar. Otra vez.",
                0.0,
                3.6,
            ));
        }
    }

    fn update(
        &mut self,
        dt: f32,
        p: &mut Player,
        arena: &mut Arena,
        rng: &mut Rng,
        ev: &mut Vec<Event>,
    ) {
        self.t += dt;
        let (px, pz) = (p.x, p.z);
        let d = (px - self.x).hypot(pz - self.z);
        let throne = plan().throne;
        match self.state {
            VState::Throne => {}
            VState::ThroneRoar => {
                if self.t > 2.2 {
                    self.set(VState::Throne);
                    self.anim.play(Clip::Sit, 0.5, 1.0, false);
                }
            }
            VState::Rise => {
                self.turn_to(px, pz, 0.6, dt);
                // Halfway up he reaches for the cleaver: it flies the last bit into his hand.
                if self.blade == BladeMode::Ground && self.t > 0.85 {
                    self.blade = BladeMode::Hand;
                    self.blade_fly = 0.25;
                    let c = plan().cleaver_spot;
                    ev.push(Event::Sfx(
                        Sfx::EnemySwing,
                        Some(Vec3::new(c.x, 0.9, c.z)),
                        0.8,
                    ));
                }
                // Off the stone with a step forward as he straightens.
                if self.t > 0.9 {
                    let k = smoothstep(self.t, 0.9, 1.6);
                    self.z = (throne.z + 0.18)
                        + ((throne.z + throne.d / 2.0 + 1.15) - (throne.z + 0.18)) * k;
                }
                if self.t > 1.6 {
                    self.awake = true;
                    self.set(VState::Roar);
                    self.anim.play(Clip::Roar, 0.3, 1.0, false);
                    ev.push(Event::Sfx(Sfx::Roar, Some(self.root()), 1.0));
                    ev.push(Event::Float("*ruge*", true, 2.0));
                }
            }
            VState::Roar => {
                if !self.struck && self.t > 1.25 {
                    self.struck = true;
                    ev.push(Event::Ring(self.x, self.z, 7.0));
                    if d < 6.5 {
                        p.hurt(6.0, self.x, self.z, 9.0);
                    }
                }
                if self.t > 2.2 {
                    self.set(VState::Chase);
                    self.anim.play(Clip::Walk, 0.3, 1.0, false);
                }
            }
            VState::Chase => {
                self.cooldown -= dt;
                self.charge_ready -= dt;
                let off = self.turn_to(px, pz, 1.9, dt);
                // Now and then, from a distance: down on all fours and charge.
                if d > 7.0 && self.charge_ready <= 0.0 && off < 0.6 {
                    self.start_drop(ev);
                } else if d < 4.4 && off < 0.5 && self.cooldown <= 0.0 {
                    let side = self.off_facing(px, pz).abs();
                    let sweep = if d < 3.0 || side > 0.3 {
                        rng.next() < 0.65
                    } else {
                        rng.next() < 0.35
                    };
                    self.set(if sweep { VState::Sweep } else { VState::Smash });
                    self.anim.play(
                        if sweep { Clip::Sweep } else { Clip::Smash },
                        if sweep { 0.15 } else { 0.18 },
                        1.0,
                        false,
                    );
                    ev.push(Event::Sfx(Sfx::EnemySwing, Some(self.root()), 0.6));
                } else if d > 3.2 {
                    self.anim.play(Clip::Walk, 0.25, 1.0, false);
                    self.step(arena, 2.25, dt, 1.05);
                } else {
                    self.anim.play(Clip::Idle, 0.3, 1.0, false);
                }
            }
            VState::Hit => {
                if self.t > 0.42 {
                    self.set(VState::Chase);
                    self.anim.play(Clip::Walk, 0.2, 1.0, false);
                }
            }
            VState::Smash => {
                // He tracks her through the wind-up, then commits.
                if self.t < 0.62 {
                    self.turn_to(px, pz, 1.6, dt);
                }
                if !self.struck && self.t >= 0.95 {
                    self.struck = true;
                    let (hx, hz) = (self.x + self.yaw.sin() * 3.4, self.z + self.yaw.cos() * 3.4);
                    ev.push(Event::Smash(hx, hz));
                    let off = self.off_facing(px, pz).abs();
                    if d < 5.4 && off < 0.62 && p.y < 0.4 {
                        p.hurt(28.0, self.x, self.z, 10.0);
                    } else if (px - hx).hypot(pz - hz) < 2.0 {
                        p.hurt(28.0, hx, hz, 10.0);
                    }
                }
                if self.t > 2.0 {
                    self.after(1.3, rng);
                }
            }
            VState::Sweep => {
                if self.t < 0.5 {
                    self.turn_to(px, pz, 1.8, dt);
                }
                if !self.struck && self.t >= 0.72 {
                    if self.t - dt < 0.72 {
                        ev.push(Event::Sfx(Sfx::EnemySwing, Some(self.root()), 1.0));
                    }
                    let off = self.off_facing(px, pz);
                    // The blade comes from his right (-) across to his left (+).
                    if d < 5.0 && off > -1.9 && off < 1.6 && p.y < 1.2 {
                        self.struck = true;
                        p.hurt(22.0, self.x, self.z, 11.0);
                    }
                    if self.t >= 0.86 {
                        self.struck = true;
                    }
                }
                if self.t > 1.7 {
                    self.after(1.1, rng);
                }
            }
            VState::Drop => {
                // He paws the ground and fixes on her.
                self.turn_to(px, pz, 2.4, dt);
                if self.t > 0.45 && self.t - dt <= 0.45 {
                    ev.push(Event::Dust(
                        self.x + self.yaw.sin() * 1.4,
                        self.z + self.yaw.cos() * 1.4,
                    ));
                }
                if self.t > 1.05 {
                    self.set(VState::Gallop);
                    self.anim.play(Clip::Gallop, 0.12, 1.0, false);
                    self.gallop = Gallop {
                        locked: false,
                        left: 3.2,
                        hit_her: false,
                    };
                    ev.push(Event::Sfx(Sfx::Roar, Some(self.root()), 0.7));
                }
            }
            VState::Gallop => {
                self.gallop.left -= dt;
                // He homes in on her at first, then runs blind in a straight line.
                if !self.gallop.locked {
                    self.turn_to(px, pz, 2.2, dt);
                    if self.t > 0.45 || d < 8.5 {
                        self.gallop.locked = true;
                    }
                }
                // Whatever is in the way flies to pieces; a spire stops him dead.
                let (fx, fz) = (self.x + self.yaw.sin() * 0.9, self.z + self.yaw.cos() * 0.9);
                for thing in arena.touching(fx, fz, 1.1) {
                    arena.broken[thing] = true;
                    if plan().breakables[thing].kind == "spire" {
                        ev.push(Event::BreakSpire(thing));
                        self.guard = 0.0;
                        self.stun(CRASH_STUN_SECONDS, Why::Spire, ev);
                        self.charge_ready = 9.0 + rng.next() * 4.0;
                        self.anim.advance(dt);
                        return;
                    }
                    ev.push(Event::BreakProp(thing));
                }
                let blocked = self.step(arena, GALLOP_SPEED, dt, 1.0);
                if !self.gallop.hit_her && d < 2.2 {
                    self.gallop.hit_her = true;
                    p.hurt(30.0, self.x, self.z, 16.0);
                }
                if blocked.is_some_and(|c| c.hit == Hit::Cliff) {
                    // He rams the cliff: rocks come down, he is dazed and his guard shaken.
                    ev.push(Event::RamCliff(
                        self.x + self.yaw.sin() * 1.5,
                        self.z + self.yaw.cos() * 1.5,
                    ));
                    self.guard = (self.guard - 35.0).max(0.0);
                    self.charge_ready = 8.0 + rng.next() * 4.0;
                    if self.guard <= 0.0 {
                        self.stun(STUN_SECONDS, Why::Cliff, ev);
                    } else {
                        self.set(VState::Dazed);
                        self.anim.play(Clip::Stun, 0.15, 1.0, false);
                        self.blade = BladeMode::Hand;
                        self.blade_fly = 0.3;
                        ev.push(Event::Float("…grr…", true, 2.0));
                    }
                } else if self.gallop.left <= 0.0 || (self.gallop.hit_her && self.t > 1.0) {
                    self.charge_ready = 8.0 + rng.next() * 4.0;
                    self.get_up(0.8);
                }
            }
            VState::GetUp => {
                if self.t > 0.4 && self.blade == BladeMode::Back {
                    self.blade = BladeMode::Hand;
                    self.blade_fly = 0.3;
                }
                if self.t > 0.8 {
                    let r = self.recover;
                    self.after(r, rng);
                }
            }
            VState::Dazed => {
                if self.t > 2.4 {
                    self.after(0.5, rng);
                }
            }
            VState::Recover => {
                self.turn_to(px, pz, 1.2, dt);
                if self.t > self.recover {
                    self.set(VState::Chase);
                    self.anim.play(Clip::Walk, 0.3, 1.0, false);
                }
            }
            VState::Stunned => {
                self.stun_left -= dt;
                if self.stun_left <= 0.0 {
                    self.shake_off(false, ev);
                }
            }
            VState::Memory => {
                // The word said, he shakes it off and fights on.
                if self.t > 2.4 {
                    self.shake_off(true, ev);
                }
            }
            VState::Kneel => {
                if self.t > 2.6 {
                    self.set(VState::Awake);
                    ev.push(Event::Awake);
                }
            }
            VState::Awake => {}
        }
        self.anim.advance(dt);
        if self.blade_fly > 0.0 {
            self.blade_fly -= dt;
        }
    }
}

// ------------------------------------------------------------------ the pack
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WState {
    Lie,
    Hidden,
    Enter,
    Circle,
    Crouch,
    Leap,
    Recover,
    Hurt,
    Flee,
}

/// A wolf without a spark: where it is, what it does, and how its body is held (for the renderer).
#[derive(Clone, Debug)]
pub(crate) struct Wolf {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub state: WState,
    pub t: f32,
    phase: f32,
    hp: f32,
    orbit: f32,
    ring: f32,
    next: f32,
    push: (f32, f32),
    pub gone: bool,
    pub visible: bool,
    den: Option<usize>,
    delay: f32,
    leap: (f32, f32, bool),
    /// Its fur (0 or 1).
    pub variant: u8,
    pub body_y: f32,
    pub body_pitch: f32,
    /// (hip, knee) of each leg: front left, front right, back left, back right.
    pub legs: [(f32, f32); 4],
    pub neck: f32,
    pub jaw: f32,
    pub tail: f32,
}

/// Each leg: (x, z, front).
pub(crate) const WOLF_LEGS: [(f32, f32, bool); 4] = [
    (-0.17, 0.42, true),
    (0.17, 0.42, true),
    (-0.17, -0.48, false),
    (0.17, -0.48, false),
];

/// The pack (ACTO1.md §8, «La manada»): beaten, they are not killed; they yelp and run off.
#[derive(Clone, Debug)]
pub(crate) struct Pack {
    pub wolves: Vec<Wolf>,
    lunging: Option<usize>,
}

impl Pack {
    fn make(&mut self, x: f32, z: f32, yaw: f32, state: WState, rng: &mut Rng) -> usize {
        let id = self.wolves.len();
        self.wolves.push(Wolf {
            x,
            z,
            yaw,
            state,
            t: rng.next() * 3.0,
            phase: rng.next() * 6.0,
            hp: 30.0,
            orbit: if rng.next() < 0.5 { 1.0 } else { -1.0 },
            ring: 4.8 + rng.next() * 2.2,
            next: 1.5 + rng.next() * 2.5,
            push: (0.0, 0.0),
            gone: false,
            visible: true,
            den: None,
            delay: 0.0,
            leap: (0.0, 0.0, false),
            variant: (id % 2) as u8,
            body_y: 0.78,
            body_pitch: 0.0,
            legs: [(0.0, 0.0); 4],
            neck: 0.25,
            jaw: 0.1,
            tail: -0.2,
        });
        id
    }

    /// Two lying by the throne, the rest waiting in the dark doorways.
    fn new(count: usize, rng: &mut Rng) -> Self {
        let p = plan();
        let mut pack = Self {
            wolves: Vec::new(),
            lunging: None,
        };
        pack.make(p.throne.x - 2.8, p.throne.z + 2.6, 0.5, WState::Lie, rng);
        pack.make(p.throne.x + 2.9, p.throne.z + 2.3, -0.4, WState::Lie, rng);
        for i in 2..count {
            if p.dens.is_empty() {
                break;
            }
            let den = i % p.dens.len();
            let (x, z) = (p.dens[den].x, p.dens[den].z);
            let w = pack.make(x, z, 0.0, WState::Hidden, rng);
            pack.wolves[w].visible = false;
            pack.wolves[w].den = Some(den);
        }
        for w in &mut pack.wolves {
            pose_wolf(w, 0.0, 0.0);
        }
        pack
    }

    /// At his roar: the lying ones get up, the hidden ones come out of their doorways.
    fn rouse(&mut self, ev: &mut Vec<Event>) {
        let mut k = 0.0;
        for w in &mut self.wolves {
            match w.state {
                WState::Lie => {
                    w.state = WState::Circle;
                    ev.push(Event::Sfx(Sfx::Roar, Some(Vec3::new(w.x, 0.6, w.z)), 0.25));
                }
                WState::Hidden => {
                    w.state = WState::Enter;
                    w.delay = 0.4 + k * 0.7;
                    k += 1.0;
                }
                _ => {}
            }
        }
    }

    /// More of them (the second wave), out of the doorways.
    fn more(&mut self, n: usize, rng: &mut Rng) {
        let p = plan();
        if p.dens.is_empty() {
            return;
        }
        for i in 0..n {
            let den = (i + 1) % p.dens.len();
            let w = self.make(p.dens[den].x, p.dens[den].z, 0.0, WState::Enter, rng);
            self.wolves[w].delay = 0.6 + i as f32 * 0.8;
            self.wolves[w].visible = false;
            self.wolves[w].den = Some(den);
        }
    }

    /// The sword: every wolf in the arc in front of her takes the blow. How many it hit.
    fn strike(&mut self, eye: Vec3, dir: Vec3, power: f32, ev: &mut Vec<Event>) -> usize {
        let mut hits = 0;
        for (i, w) in self.wolves.iter_mut().enumerate() {
            let able = matches!(
                w.state,
                WState::Circle
                    | WState::Crouch
                    | WState::Leap
                    | WState::Recover
                    | WState::Hurt
                    | WState::Enter
            );
            if !able || !w.visible {
                continue;
            }
            let (dx, dz) = (w.x - eye.x, w.z - eye.z);
            let d = dx.hypot(dz);
            if d > 2.7 {
                continue;
            }
            let flat = dir.x.hypot(dir.z).max(1e-3);
            if (dx * dir.x + dz * dir.z) / (d.max(1e-3) * flat) < 0.85f32.cos() {
                continue;
            }
            w.hp -= 12.0 * power;
            w.push = (dx / d.max(1e-3) * 5.0, dz / d.max(1e-3) * 5.0);
            hits += 1;
            ev.push(Event::Burst(
                Vec3::new(w.x, 0.9, w.z),
                [1.0, 0.85, 0.63],
                10,
                3.0,
            ));
            if self.lunging == Some(i) {
                self.lunging = None;
            }
            w.t = 0.0;
            if w.hp <= 0.0 {
                w.state = WState::Flee;
                ev.push(Event::Sfx(
                    Sfx::SlimeDeath,
                    Some(Vec3::new(w.x, 0.6, w.z)),
                    0.9,
                ));
            } else {
                w.state = WState::Hurt;
                ev.push(Event::Sfx(
                    Sfx::HitSlime,
                    Some(Vec3::new(w.x, 0.6, w.z)),
                    0.7,
                ));
            }
        }
        hits
    }

    /// How many still fight (not fled, not waiting in the dark).
    pub(crate) fn fighting(&self) -> usize {
        self.wolves
            .iter()
            .filter(|w| !w.gone && !matches!(w.state, WState::Flee | WState::Hidden))
            .count()
    }

    fn update(
        &mut self,
        dt: f32,
        p: &mut Player,
        arena: &Arena,
        rng: &mut Rng,
        ev: &mut Vec<Event>,
    ) {
        let dens = &plan().dens;
        for i in 0..self.wolves.len() {
            if self.wolves[i].gone {
                continue;
            }
            let w = &mut self.wolves[i];
            w.t += dt;
            let (dx, dz) = (p.x - w.x, p.z - w.z);
            let d = dx.hypot(dz);
            let face = dx.atan2(dz);
            let (mut speed, mut want, mut gait) = (0.0, w.yaw, 0.0);
            match w.state {
                WState::Lie | WState::Hidden => {}
                WState::Enter => {
                    w.delay -= dt;
                    if w.delay <= 0.0 {
                        w.visible = true;
                        w.state = WState::Circle;
                        w.t = 0.0;
                        ev.push(Event::Sfx(Sfx::Roar, Some(Vec3::new(w.x, 0.6, w.z)), 0.22));
                    }
                }
                WState::Circle => {
                    // Keep a ring round her, drifting round it, facing her.
                    let around = (w.x - p.x).atan2(w.z - p.z) + w.orbit * 0.9 * dt;
                    let (tx, tz) = (p.x + around.sin() * w.ring, p.z + around.cos() * w.ring);
                    let (mx, mz) = (tx - w.x, tz - w.z);
                    let m = mx.hypot(mz);
                    speed = (m * 2.2).min(5.5);
                    want = if m > 0.4 { mx.atan2(mz) } else { face };
                    if m < 1.2 {
                        want = face;
                    }
                    gait = speed / 5.5;
                    w.next -= dt;
                    if w.next <= 0.0 && self.lunging.is_none() && d < 8.5 && p.y < 0.5 {
                        w.state = WState::Crouch;
                        w.t = 0.0;
                        self.lunging = Some(i);
                        ev.push(Event::Sfx(Sfx::Roar, Some(Vec3::new(w.x, 0.6, w.z)), 0.3));
                    }
                }
                WState::Crouch => {
                    want = face;
                    if w.t > 0.5 {
                        w.state = WState::Leap;
                        w.t = 0.0;
                        w.leap = (dx / d.max(1e-3), dz / d.max(1e-3), false);
                    }
                }
                WState::Leap => {
                    want = w.leap.0.atan2(w.leap.1);
                    speed = 9.5;
                    gait = 1.0;
                    if !w.leap.2 && d < 1.25 {
                        w.leap.2 = true;
                        if p.hurt(8.0, w.x, w.z, 4.0) {
                            ev.push(Event::Sfx(
                                Sfx::HitSlime,
                                Some(Vec3::new(w.x, 0.8, w.z)),
                                0.8,
                            ));
                        }
                    }
                    if w.t > 0.48 {
                        w.state = WState::Recover;
                        w.t = 0.0;
                        if self.lunging == Some(i) {
                            self.lunging = None;
                        }
                    }
                }
                WState::Recover => {
                    want = face;
                    if w.t > 0.7 {
                        w.state = WState::Circle;
                        w.t = 0.0;
                        w.next = 2.4 + rng.next() * 2.2;
                        w.orbit = if rng.next() < 0.5 { 1.0 } else { -1.0 };
                    }
                }
                WState::Hurt => {
                    if w.t > 0.4 {
                        w.state = WState::Circle;
                        w.t = 0.0;
                        w.next = 1.6 + rng.next() * 1.5;
                    }
                }
                WState::Flee => {
                    // To the nearest doorway (its own, if it came from one).
                    let den = w.den.or_else(|| {
                        (0..dens.len()).min_by(|&a, &b| {
                            (dens[a].x - w.x)
                                .hypot(dens[a].z - w.z)
                                .total_cmp(&(dens[b].x - w.x).hypot(dens[b].z - w.z))
                        })
                    });
                    if let Some(den) = den {
                        let (mx, mz) = (dens[den].x - w.x, dens[den].z - w.z);
                        want = mx.atan2(mz);
                        if mx.hypot(mz) < 1.0 {
                            w.gone = true;
                            w.visible = false;
                        }
                    }
                    speed = 7.5;
                    gait = 1.0;
                    if w.t > 6.0 {
                        w.gone = true;
                        w.visible = false;
                    }
                }
            }
            // Turn, move, keep apart from each other, collide.
            w.yaw += wrap(want - w.yaw).clamp(-7.0 * dt, 7.0 * dt);
            let mut nx = w.x + w.yaw.sin() * speed * dt + w.push.0 * dt;
            let mut nz = w.z + w.yaw.cos() * speed * dt + w.push.1 * dt;
            let k = (-dt * 6.0).exp();
            w.push = (w.push.0 * k, w.push.1 * k);
            let flee = w.state == WState::Flee;
            let (wx, wz) = (w.x, w.z);
            let _ = (wx, wz);
            for j in 0..self.wolves.len() {
                if j == i {
                    continue;
                }
                let o = &self.wolves[j];
                if o.gone || !o.visible {
                    continue;
                }
                let (ox, oz) = (nx - o.x, nz - o.z);
                let od = ox.hypot(oz);
                if od < 1.3 && od > 1e-4 {
                    nx += ox / od * (1.3 - od) * 0.5;
                    nz += oz / od * (1.3 - od) * 0.5;
                }
            }
            let w = &mut self.wolves[i];
            if flee {
                w.x = nx;
                w.z = nz;
            } else {
                let c = arena.collide(nx, nz, 0.5);
                w.x = c.x;
                w.z = c.z;
            }
            pose_wolf(w, dt, gait);
        }
    }
}

/// The legs and body, in code: lying, trotting, galloping, crouching, the leap.
fn pose_wolf(w: &mut Wolf, dt: f32, gait: f32) {
    use std::f32::consts::PI;
    w.phase += dt * (6.0 + gait * 10.0);
    let lying = w.state == WState::Lie;
    let crouch = if w.state == WState::Crouch {
        (w.t / 0.3).min(1.0)
    } else {
        0.0
    };
    let leap = if w.state == WState::Leap {
        ((w.t / 0.48).min(1.0) * PI).sin()
    } else {
        0.0
    };
    w.body_y = if lying {
        0.32
    } else {
        0.78 - crouch * 0.22 + leap * 0.55 + w.phase.sin().abs() * 0.05 * gait
    };
    w.body_pitch = -crouch * 0.12 - leap * 0.25 + (w.phase * 2.0).sin() * 0.04 * gait;
    for (k, (lx, _, front)) in WOLF_LEGS.iter().enumerate() {
        let off = if *front { 0.0 } else { PI } + if *lx > 0.0 { 0.6 } else { 0.0 };
        w.legs[k] = if lying {
            if *front {
                (-1.3, 0.2)
            } else {
                (1.4, -2.2)
            }
        } else {
            let swing = (w.phase + off).sin() * (0.25 + gait * 0.55);
            let hip = swing - if *front { crouch * 0.5 } else { -crouch * 0.6 }
                + if leap > 0.0 {
                    if *front {
                        -1.1 * leap
                    } else {
                        1.0 * leap
                    }
                } else {
                    0.0
                };
            let knee = if *front { 1.0 } else { -1.0 }
                * (-(w.phase + off).cos()).max(0.0)
                * (0.3 + gait * 0.6)
                + if *front { crouch * 0.6 } else { -crouch * 0.9 };
            (hip, knee)
        };
    }
    w.neck = if lying {
        if w.t % 6.0 < 3.0 {
            0.5
        } else {
            0.2
        }
    } else {
        0.25 + crouch * 0.25 - leap * 0.2
    };
    w.jaw = 0.1
        + if matches!(w.state, WState::Crouch | WState::Leap) {
            0.35
        } else {
            0.05 * (w.t * 9.0).sin()
        };
    w.tail = if w.state == WState::Flee {
        0.9
    } else {
        -0.2 + (w.t * 3.0).sin() * 0.1
    };
}

// ------------------------------------------------------------------ what flies
/// A rock off the cliff (a ram) or a chip off a spire: where, how fast, how big.
#[derive(Clone, Debug)]
pub(crate) struct Rock {
    pub at: Vec3,
    v: Vec3,
    pub r: f32,
    /// How fast it turns.
    pub spin: Vec3,
    delay: f32,
    /// Falling off the cliff (it hurts where it lands); else a chip.
    falling: bool,
    pub rest: bool,
}

/// The cleaver's shockwave racing over the ground (jump it).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Wave {
    pub x: f32,
    pub z: f32,
    pub r: f32,
    pub reach: f32,
    speed: f32,
    caught: bool,
}

/// A breakable thing going to pieces: which, and away from where.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Break {
    pub index: usize,
    pub from: (f32, f32),
    pub force: f32,
    pub lift: f32,
    pub spin: f32,
}

// ------------------------------------------------------------------ the fight
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The card with what to do; a click begins.
    Intro,
    Pack,
    Duel,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug)]
enum Timed {
    Wake,
    Rouse,
    MoreWolves(usize),
    Card,
}

/// Chiquita's lines at the bottom: typed out, kept a while, then gone; lines wait their turn.
#[derive(Clone, Debug)]
pub(crate) struct Line {
    pub text: String,
    at: f32,
    keep: f32,
}

/// The fight: the plaza, her, him, the pack, what flies, what is said.
pub(crate) struct Fight {
    pub phase: Phase,
    pub time: f32,
    started: f32,
    pub arena: Arena,
    pub player: Player,
    pub varrak: Varrak,
    pub pack: Pack,
    /// Sparks, dust, rings (the dungeon's own effects).
    pub fx: Vec<Fx>,
    pub rocks: Vec<Rock>,
    pub waves: Vec<Wave>,
    pub breaks: Vec<Break>,
    lines: VecDeque<Line>,
    /// The line on show: its text, when it came, until when.
    pub line: Option<(String, f32, f32)>,
    /// A word over his head: text, a growl (else a memory), until when.
    pub float: Option<(&'static str, bool, f32)>,
    pub vignette: f32,
    pub flash: f32,
    /// When the light of the seal began to rise (won).
    pub won_at: Option<f32>,
    /// The end card is up.
    pub card: bool,
    wave2: bool,
    hint_shown: bool,
    last_body_line: f32,
    /// The spark this step: flowing into him, or fizzling off the seal.
    pub flowing: bool,
    pub fizzle: bool,
    /// How strong the beam is (it fades in and out).
    pub beam: f32,
    spark_sound: f32,
    /// What the fight sounded like this step (taken by the world).
    pub sounds: Vec<Sound>,
    rng: Rng,
    timers: Vec<(f32, Timed)>,
    /// His chest's emblem and a point over his head in the world (sent by the renderer).
    pub chest: Option<Vec3>,
    pub over: Option<Vec3>,
    /// How the Council chose to deal with the apagados (fewer wolves if they were hunted).
    hunted: bool,
}

impl Fight {
    /// A new fight; `wolves`: how many the pack has (2 to 6).
    pub(crate) fn new(wolves: usize, hunted: bool, seed: u32) -> Self {
        let mut rng = Rng::new(seed);
        let p = plan();
        let pack = Pack::new(wolves.clamp(2, 6), &mut rng);
        Self {
            phase: Phase::Intro,
            time: 0.0,
            started: 0.0,
            arena: Arena::new(),
            player: Player::new(p.start.x, p.start.z, p.start.yaw),
            varrak: Varrak::new(),
            pack,
            fx: Vec::new(),
            rocks: Vec::new(),
            waves: Vec::new(),
            breaks: Vec::new(),
            lines: VecDeque::new(),
            line: None,
            float: None,
            vignette: 0.0,
            flash: 0.0,
            won_at: None,
            card: false,
            wave2: false,
            hint_shown: false,
            last_body_line: -99.0,
            flowing: false,
            fizzle: false,
            beam: 0.0,
            spark_sound: 0.0,
            sounds: Vec::new(),
            rng,
            timers: Vec::new(),
            chest: None,
            over: None,
            hunted,
        }
    }

    fn chiquita(&mut self, text: &str, delay: f32, keep: f32) {
        self.lines.push_back(Line {
            text: text.to_owned(),
            at: self.time + delay,
            keep,
        });
    }

    fn sfx(&mut self, sfx: Sfx, at: Option<Vec3>, volume: f32) {
        self.sounds.push(Sound { sfx, at, volume });
    }

    /// Her chest's emblem (the spark goes there).
    pub(crate) fn varrak_chest(&self) -> Vec3 {
        self.chest.unwrap_or_else(|| self.varrak.chest_guess())
    }

    /// The fight begins (the click on the card): the pack wakes at his roar.
    pub(crate) fn begin(&mut self) {
        if self.phase != Phase::Intro {
            return;
        }
        self.phase = Phase::Pack;
        self.started = self.time;
        self.player.vulnerable = true;
        self.chiquita("Mira el emblema, no los ojos.", 0.4, 3.6);
        self.timers.push((self.time + 1.3, Timed::Wake));
        self.timers.push((self.time + 2.4, Timed::Rouse));
    }

    fn start_duel(&mut self) {
        if self.phase != Phase::Pack {
            return;
        }
        self.phase = Phase::Duel;
        self.varrak.rise();
        self.chiquita(
            "Ahí viene él. Rómpele la guardia y luego, el sello.",
            1.0,
            3.6,
        );
    }

    fn win(&mut self) {
        self.phase = Phase::Won;
        self.player.vulnerable = false;
        self.player.frozen = true;
        self.float = Some(("Majestad. Ha tardado usted.", false, self.time + 6.0));
        self.won_at = Some(self.time);
        self.flash = 1.0;
        self.timers.push((self.time + 4.8, Timed::Card));
    }

    fn lose(&mut self) {
        self.phase = Phase::Lost;
        self.player.vulnerable = false;
        self.player.frozen = true;
        self.chiquita("¡Que este cuerpo es mío! ¡Fuera de aquí!", 0.0, 4.0);
        self.timers.push((self.time + 2.6, Timed::Card));
    }

    /// The sword at the moment it crosses the screen: wolves in the arc, and him.
    fn strike(&mut self, power: f32, ev: &mut Vec<Event>) {
        let eye = self.player.eye();
        self.sfx(Sfx::Swing, None, 0.8);
        if !matches!(self.phase, Phase::Pack | Phase::Duel) {
            return;
        }
        let dir = self.player.forward();
        let wolves = self.pack.strike(eye, dir, power, ev);
        let him = self.varrak.strike(eye, dir, power, ev);
        if wolves > 0 || him {
            // The blade bites: a beat of hit-stop and a small jolt.
            self.player.impact(if him { 0.1 } else { 0.065 });
            self.player.kick(0.12, 0.05);
        }
    }

    /// One step of the fight.
    pub(crate) fn update(&mut self, input: Input, dt: f32) {
        self.time += dt;
        self.sounds.clear();
        let mut ev = Vec::new();
        let fighting = matches!(self.phase, Phase::Pack | Phase::Duel);
        let input = if self.phase == Phase::Intro {
            Input::default()
        } else {
            input
        };
        if let Some(power) = self.player.update(input, dt, &self.arena) {
            self.strike(power, &mut ev);
        }
        if self.phase == Phase::Pack
            && self.time - self.started > 3.5
            && (self.pack.fighting() == 0 || self.time - self.started > 50.0)
        {
            self.start_duel();
        }
        self.pack
            .update(dt, &mut self.player, &self.arena, &mut self.rng, &mut ev);
        self.varrak.update(
            dt,
            &mut self.player,
            &mut self.arena,
            &mut self.rng,
            &mut ev,
        );
        // The seal.
        let eye = self.player.eye();
        let dir = self.player.forward();
        self.flowing = false;
        self.fizzle = false;
        if self.player.channeling && fighting {
            let chest = self.varrak_chest();
            let (flowing, open) = self.varrak.channel(dt, eye, dir, chest, &mut ev);
            self.flowing = flowing;
            self.fizzle = !flowing;
            if !open && !self.hint_shown && self.varrak.awake {
                self.hint_shown = true;
                self.chiquita(
                    "Así no. Primero rómpele la guardia, o que se dé contra una roca.",
                    0.0,
                    3.6,
                );
            }
        }
        self.player.glow = if self.flowing {
            1.0
        } else if self.player.channeling {
            0.35
        } else {
            0.0
        };
        let want = if self.flowing {
            1.0
        } else if self.fizzle {
            0.35
        } else {
            0.0
        };
        self.beam += (want - self.beam) * (dt * if want > self.beam { 14.0 } else { 6.0 }).min(1.0);
        if self.flowing {
            self.spark_sound -= dt;
            if self.spark_sound <= 0.0 {
                self.spark_sound = 0.45;
                let chest = self.varrak_chest();
                self.sfx(Sfx::SpellCharge, Some(chest), 0.55);
            }
        } else {
            self.spark_sound = 0.0;
        }
        for e in ev {
            self.answer(e);
        }
        // Her blows taken: a cry, the red at the edges, Chiquita minding her body.
        for amount in std::mem::take(&mut self.player.hurts) {
            self.sfx(Sfx::PlayerHurt, None, 1.0);
            self.vignette = (self.vignette + amount / 30.0).min(1.0);
            if amount >= 22.0 && self.time - self.last_body_line > 18.0 {
                self.last_body_line = self.time;
                self.chiquita("¡Que este cuerpo es mío!", 0.0, 3.6);
            }
        }
        self.update_effects(dt);
        if fighting && self.player.health <= 0.0 {
            self.lose();
        }
        self.vignette = (self.vignette - dt * 1.4).max(0.0);
        self.flash = (self.flash - dt * 1.8).max(0.0);
        if self.float.is_some_and(|(_, _, until)| self.time > until) {
            self.float = None;
        }
        self.update_lines();
        // What was set to happen later.
        let now = self.time;
        let due: Vec<Timed> = self
            .timers
            .iter()
            .filter(|(at, _)| *at <= now)
            .map(|(_, t)| *t)
            .collect();
        self.timers.retain(|(at, _)| *at > now);
        for t in due {
            match t {
                Timed::Wake => {
                    let mut ev = Vec::new();
                    self.varrak.wake(&mut ev);
                    for e in ev {
                        self.answer(e);
                    }
                }
                Timed::Rouse => {
                    let mut ev = Vec::new();
                    self.pack.rouse(&mut ev);
                    for e in ev {
                        self.answer(e);
                    }
                }
                Timed::MoreWolves(n) => {
                    self.pack.more(n, &mut self.rng);
                    let den = plan().dens.first().map(|d| Vec3::new(d.x, 0.6, d.z));
                    self.sfx(Sfx::Roar, den, 0.3);
                }
                Timed::Card => self.card = true,
            }
        }
    }

    /// What the fight does when something happens.
    fn answer(&mut self, e: Event) {
        match e {
            Event::Sfx(sfx, at, volume) => self.sfx(sfx, at, volume),
            Event::Float(text, growl, keep) => self.float = Some((text, growl, self.time + keep)),
            Event::Chiquita(text, delay, keep) => self.chiquita(text, delay, keep),
            Event::Burst(at, color, count, speed) => {
                let seed = (self.rng.next() * 1e6) as u32;
                self.fx.push(Fx::burst(at, color, count, speed, seed));
            }
            Event::Dust(x, z) => {
                let seed = (self.rng.next() * 1e6) as u32;
                self.fx.push(Fx::dust(Vec3::new(x, 0.2, z), seed));
            }
            Event::Ring(x, z, radius) => self.fx.push(Fx::Shockwave {
                at: Vec3::new(x, 0.0, z),
                radius,
                age: 0.0,
            }),
            Event::Smash(hx, hz) => {
                self.sfx(Sfx::Shockwave, Some(Vec3::new(hx, 0.2, hz)), 1.0);
                for k in 0..2 {
                    let seed = (self.rng.next() * 1e6) as u32 + k;
                    self.fx.push(Fx::dust(Vec3::new(hx, 0.2, hz), seed));
                }
                let seed = (self.rng.next() * 1e6) as u32;
                self.fx.push(Fx::burst(
                    Vec3::new(hx, 0.2, hz),
                    [1.0, 0.78, 0.56],
                    22,
                    6.0,
                    seed,
                ));
                let d = (self.player.x - hx).hypot(self.player.z - hz);
                self.player.kick(0.4, 0.14 * (1.0 - d / 14.0).max(0.2));
                self.waves.push(Wave {
                    x: hx,
                    z: hz,
                    r: 0.5,
                    reach: 12.0,
                    speed: 9.5,
                    caught: false,
                });
            }
            Event::Fours(n) => {
                if n == 1 {
                    self.chiquita(
                        "¡Se ha puesto a cuatro patas! Ponte delante de un peñasco y apártate en el último momento.",
                        0.1,
                        5.0,
                    );
                } else if self.rng.next() < 0.6 {
                    let lines = [
                        "¡A la roca! ¡Llévalo a la roca!",
                        "¡Otra vez a cuatro patas! ¡Los peñascos!",
                        "Que viene. No te quedes en medio.",
                    ];
                    self.chiquita(lines[n as usize % 3], 0.0, 3.6);
                }
            }
            Event::BreakSpire(i) => {
                let b = &plan().breakables[i];
                let (bx, bz) = (b.x, b.z);
                self.breaks.push(Break {
                    index: i,
                    from: (self.varrak.x, self.varrak.z),
                    force: 4.0,
                    lift: 3.0,
                    spin: 4.0,
                });
                // Chips thrown out, a cloud of dust, the crash; the ground shakes.
                for _ in 0..14 {
                    let a = self.rng.next() * std::f32::consts::TAU;
                    let s = 3.0 + self.rng.next() * 4.0;
                    let r = 0.32 * (0.5 + self.rng.next());
                    self.rocks.push(Rock {
                        at: Vec3::new(
                            bx + a.cos() * 0.4,
                            1.4 + self.rng.next() * 1.2,
                            bz + a.sin() * 0.4,
                        ),
                        v: Vec3::new(a.cos() * s, 3.0 + self.rng.next() * 4.0, a.sin() * s),
                        r,
                        spin: Vec3::new(
                            self.rng.next() * 9.0,
                            self.rng.next() * 9.0,
                            self.rng.next() * 9.0,
                        ),
                        delay: 0.0,
                        falling: false,
                        rest: false,
                    });
                }
                for k in 0..3 {
                    let seed = (self.rng.next() * 1e6) as u32 + k;
                    self.fx
                        .push(Fx::dust(Vec3::new(bx, 0.4 + k as f32 * 0.6, bz), seed));
                }
                self.sfx(Sfx::Rubble, Some(Vec3::new(bx, 1.0, bz)), 1.0);
                let d = (self.player.x - bx).hypot(self.player.z - bz);
                self.player.kick(0.5, 0.16 * (1.0 - d / 18.0).max(0.25));
                self.flash = self.flash.max(0.35);
            }
            Event::BreakProp(i) => {
                let b = &plan().breakables[i];
                let (bx, bz) = (b.x, b.z);
                self.breaks.push(Break {
                    index: i,
                    from: (self.varrak.x, self.varrak.z),
                    force: 9.0,
                    lift: 5.0,
                    spin: 9.0,
                });
                let seed = (self.rng.next() * 1e6) as u32;
                self.fx.push(Fx::dust(Vec3::new(bx, 0.3, bz), seed));
                self.sfx(Sfx::BoneCrack, Some(Vec3::new(bx, 0.6, bz)), 0.9);
            }
            Event::RamCliff(x, z) => {
                self.sfx(Sfx::Rubble, Some(Vec3::new(x, 1.0, z)), 1.0);
                for k in 0..2 {
                    let seed = (self.rng.next() * 1e6) as u32 + k;
                    self.fx.push(Fx::dust(Vec3::new(x, 0.4, z), seed));
                }
                self.player.kick(0.5, 0.14);
                let a = angle_of(x, z);
                for _ in 0..7 {
                    let aa = a + (self.rng.next() - 0.5) * 0.3;
                    let top = cliff_height(aa);
                    let out = 0.9 + self.rng.next() * 1.6;
                    let (px, pz) = ring_point(aa, out - 0.1 * (top - 0.6));
                    self.rocks.push(Rock {
                        at: Vec3::new(px, top + self.rng.next() * 2.0, pz),
                        v: Vec3::new(
                            (self.rng.next() - 0.5) * 1.5,
                            -1.0 - self.rng.next() * 2.0,
                            (self.rng.next() - 0.5) * 1.5,
                        ),
                        r: 0.45 + self.rng.next() * 0.6,
                        spin: Vec3::new(3.0, 0.0, 2.0),
                        delay: self.rng.next() * 0.6,
                        falling: true,
                        rest: false,
                    });
                }
            }
            Event::Stunned(why) => {
                let line = match why {
                    Why::Guard => "¡Ahora! El sello.",
                    Why::Spire => "¡Se ha dado de morros contra la roca! ¡El sello, ya!",
                    Why::Cliff => "¡Contra la pared! Ahora, el sello.",
                };
                self.chiquita(line, 0.0, 3.6);
            }
            Event::Memory(i) => {
                let m = &MEMORIES[(i - 1).min(MEMORIES.len() - 1)];
                self.float = Some((m.word, false, self.time + 3.2));
                self.chiquita(m.chiquita, 0.9, 3.6);
                self.flash = self.flash.max(0.8);
                self.sfx(Sfx::RuneChime, self.chest, 1.0);
                self.player.heal(15.0);
                // Half the bar: he calls the rest of the pack.
                if i == 2 && !self.wave2 {
                    self.wave2 = true;
                    let n = if self.hunted { 1 } else { 2 };
                    self.timers.push((self.time + 2.6, Timed::MoreWolves(n)));
                }
            }
            Event::Awake => self.win(),
        }
    }

    /// Effects that move and hurt: rocks falling and settling, the smash's wave, sparks fading.
    fn update_effects(&mut self, dt: f32) {
        let player = Vec3::new(self.player.x, self.player.y, self.player.z);
        self.fx.retain_mut(|f| f.tick(dt, player).1);
        let mut landed = Vec::new();
        for r in &mut self.rocks {
            if r.rest {
                continue;
            }
            if r.delay > 0.0 {
                r.delay -= dt;
                continue;
            }
            r.v.y -= GRAVITY * dt;
            r.at = r.at + r.v * dt;
            let floor = r.r * 0.45;
            if r.at.y <= floor {
                r.at.y = floor;
                if r.falling {
                    landed.push((r.at.x, r.at.z, r.r));
                    r.falling = false;
                    r.v = Vec3::ZERO;
                    r.rest = true;
                } else if r.v.y < -2.5 {
                    r.v = Vec3::new(r.v.x * 0.55, -r.v.y * 0.3, r.v.z * 0.55);
                } else {
                    r.v = Vec3::new(r.v.x * 0.55, 0.0, r.v.z * 0.55);
                    if r.v.length() < 0.4 {
                        r.rest = true;
                    }
                }
            }
        }
        for (x, z, r) in landed {
            let seed = (self.rng.next() * 1e6) as u32;
            self.fx.push(Fx::dust(Vec3::new(x, 0.2, z), seed));
            self.sfx(Sfx::Rubble, Some(Vec3::new(x, 0.4, z)), 0.5);
            if (self.player.x - x).hypot(self.player.z - z) < r + 1.2 {
                self.player.hurt(14.0, x, z, 6.0);
            }
        }
        let (px, pz, py) = (self.player.x, self.player.z, self.player.y);
        let mut hurt_by = Vec::new();
        for w in &mut self.waves {
            w.r += w.speed * dt;
            if !w.caught && py <= 0.32 && ((px - w.x).hypot(pz - w.z) - w.r).abs() < 0.55 {
                w.caught = true;
                hurt_by.push((w.x, w.z));
            }
        }
        self.waves.retain(|w| w.r < w.reach);
        for (x, z) in hurt_by {
            self.player.hurt(12.0, x, z, 7.0);
        }
        // (the hurts the effects did this step are answered next step)
    }

    fn update_lines(&mut self) {
        let now = self.time;
        let until = self.line.as_ref().map_or(0.0, |l| l.2);
        if now > until {
            if self.lines.front().is_some_and(|l| l.at <= now) {
                if let Some(l) = self.lines.pop_front() {
                    let n = l.text.chars().count() as f32;
                    let until = now + l.keep.max(1.4 + n * 0.045);
                    self.line = Some((l.text, now, until));
                }
            } else {
                self.line = None;
            }
        }
    }

    /// The line on show as far as it has typed out.
    pub(crate) fn line_typed(&self) -> Option<String> {
        let (text, at, _) = self.line.as_ref()?;
        let n = ((self.time - at) * 45.0).max(0.0) as usize;
        Some(text.chars().take(n).collect())
    }
}

// ------------------------------------------------------------------ visual QA
impl Fight {
    /// Visual QA: the fight put at a moment (`duel`, `smash`, `gallop`, `spire`, `spark`, `memory`,
    /// `won`, `lost`), her a few metres in front of him.
    pub(crate) fn qa(&mut self, what: &str) {
        let mut ev = Vec::new();
        if self.phase == Phase::Intro {
            self.begin();
        }
        let in_front = |f: &mut Fight, d: f32| {
            f.player.x = f.varrak.x + f.varrak.yaw.sin() * d;
            f.player.z = f.varrak.z + f.varrak.yaw.cos() * d;
        };
        match what {
            "duel" => {
                for w in &mut self.pack.wolves {
                    w.gone = true;
                    w.visible = false;
                }
                self.start_duel();
            }
            "smash" | "sweep" | "gallop" | "spire" | "spark" | "memory" | "won" => {
                for w in &mut self.pack.wolves {
                    w.gone = true;
                    w.visible = false;
                }
                self.phase = Phase::Duel;
                self.varrak.awake = true;
                self.varrak.blade = BladeMode::Hand;
                self.varrak.x = 0.0;
                self.varrak.z = -1.0;
                self.varrak.yaw = 0.0;
                match what {
                    "smash" => {
                        in_front(self, 4.2);
                        self.varrak.set(VState::Smash);
                        self.varrak.anim.play(Clip::Smash, 0.0, 1.0, false);
                    }
                    "sweep" => {
                        in_front(self, 3.4);
                        self.varrak.set(VState::Sweep);
                        self.varrak.anim.play(Clip::Sweep, 0.0, 1.0, false);
                    }
                    "gallop" => {
                        in_front(self, 11.0);
                        self.player.x += 3.0;
                        self.varrak.start_drop(&mut ev);
                    }
                    "spire" => {
                        // A spire between him and her, he charges.
                        if let Some(i) = plan().breakables.iter().position(|b| b.kind == "spire") {
                            let (sx, sz) = (plan().breakables[i].x, plan().breakables[i].z);
                            self.varrak.x = sx;
                            self.varrak.z = sz + 6.5;
                            self.varrak.yaw = std::f32::consts::PI;
                            self.player.x = sx + 2.5;
                            self.player.z = sz - 4.0;
                            self.varrak.start_drop(&mut ev);
                        }
                    }
                    "spark" | "memory" | "won" => {
                        in_front(self, 5.5);
                        if what == "memory" {
                            self.varrak.spark = 24.6;
                        } else if what == "won" {
                            self.varrak.spark = 99.6;
                            self.varrak.memories = MEMORIES.len();
                        } else {
                            self.varrak.spark = 12.0;
                        }
                        self.varrak.stun(30.0, Why::Guard, &mut ev);
                    }
                    _ => {}
                }
            }
            "lost" => {
                self.player.health = 0.0;
            }
            _ => {}
        }
        for e in ev {
            self.answer(e);
        }
    }

    /// Visual QA: she looks at his chest (or at the nearest wolf while the pack is out).
    pub(crate) fn qa_look(&mut self) {
        let chest = self.varrak_chest();
        let p = &mut self.player;
        let (dx, dz) = (chest.x - p.x, chest.z - p.z);
        p.yaw = (-dx).atan2(-dz);
        p.pitch = (chest.y - (p.y + EYE)).atan2(dx.hypot(dz)).clamp(-0.5, 0.5);
    }
}

// ------------------------------------------------------------------ tests
#[cfg(test)]
mod tests {
    use super::*;

    /// A player who fights well: close in and swing while his guard is up (keeping out of a charge's
    /// way), channel the seal when he is open.
    fn bot(f: &Fight) -> Input {
        let p = &f.player;
        let v = &f.varrak;
        let mut input = Input::default();
        let (dx, dz) = (v.x - p.x, v.z - p.z);
        let d = dx.hypot(dz);
        if f.phase == Phase::Pack {
            if let Some(w) = f
                .pack
                .wolves
                .iter()
                .filter(|w| w.visible && !w.gone && w.state != WState::Flee)
                .min_by(|a, b| {
                    (a.x - p.x)
                        .hypot(a.z - p.z)
                        .total_cmp(&(b.x - p.x).hypot(b.z - p.z))
                })
            {
                let wd = (w.x - p.x).hypot(w.z - p.z);
                input.forward = if wd > 1.8 { 1.0 } else { 0.0 };
                input.attack = wd < 2.6;
            }
            return input;
        }
        input.channel = v.open();
        if !v.open() && !v.on_fours() {
            input.forward = if d > 2.6 { 1.0 } else { 0.0 };
            input.attack = d < 3.4;
        } else if v.on_fours() {
            // Step aside of the charge.
            input.strafe = 1.0;
        }
        input
    }

    fn aim(f: &mut Fight) {
        let chest = f.varrak_chest();
        let p = &mut f.player;
        let (dx, dz) = (chest.x - p.x, chest.z - p.z);
        p.yaw = (-dx).atan2(-dz);
        p.pitch = (chest.y - (p.y + EYE)).atan2(dx.hypot(dz));
        if f.phase == Phase::Pack {
            if let Some(w) = f
                .pack
                .wolves
                .iter()
                .filter(|w| w.visible && !w.gone && w.state != WState::Flee)
                .min_by(|a, b| {
                    (a.x - p.x)
                        .hypot(a.z - p.z)
                        .total_cmp(&(b.x - p.x).hypot(b.z - p.z))
                })
            {
                let (dx, dz) = (w.x - p.x, w.z - p.z);
                p.yaw = (-dx).atan2(-dz);
                p.pitch = -0.3;
            }
        }
    }

    #[test]
    fn the_plaza_keeps_her_inside_and_out_of_the_stone() {
        if !available() {
            return;
        }
        let a = Arena::new();
        let c = a.collide(0.0, 40.0, 0.38);
        assert_eq!(c.hit, Hit::Cliff);
        assert!(c.z < plan().arena.rz);
        let t = plan().throne;
        let c = a.collide(t.x, t.z, 0.38);
        assert_ne!(c.hit, Hit::None);
        let start = plan().start;
        assert_eq!(a.collide(start.x, start.z, 0.38).hit, Hit::None);
    }

    #[test]
    fn the_spark_flows_only_while_he_is_open_and_tears_three_memories_loose() {
        if !available() {
            return;
        }
        let mut f = Fight::new(2, false, 7);
        f.begin();
        f.phase = Phase::Duel;
        f.varrak.set(VState::Chase);
        f.varrak.awake = true;
        f.player.x = f.varrak.x;
        f.player.z = f.varrak.z + 6.0;
        aim(&mut f);
        // Closed: nothing flows.
        let input = Input {
            channel: true,
            ..Input::default()
        };
        f.update(input, 1.0 / 60.0);
        assert!(!f.flowing);
        // Stunned: it flows while she aims at him, and stops at each memory.
        let mut ev = Vec::new();
        f.varrak.stun(30.0, Why::Guard, &mut ev);
        let mut memories = 0;
        for _ in 0..(60 * 60) {
            if f.varrak.state == VState::Memory {
                memories = memories.max(f.varrak.memories);
            }
            if f.varrak.state == VState::Roar {
                let mut ev = Vec::new();
                f.varrak.stun(30.0, Why::Guard, &mut ev);
            }
            aim(&mut f);
            f.update(input, 1.0 / 60.0);
            if f.phase == Phase::Won {
                break;
            }
        }
        assert_eq!(memories, 3);
        assert!(f.varrak.spark >= 100.0);
        assert_eq!(f.phase, Phase::Won);
    }

    #[test]
    fn a_charge_into_a_spire_breaks_it_and_stuns_him() {
        if !available() {
            return;
        }
        let mut f = Fight::new(2, false, 3);
        f.begin();
        f.phase = Phase::Duel;
        let spire = plan()
            .breakables
            .iter()
            .position(|b| b.kind == "spire")
            .unwrap();
        let (sx, sz) = (plan().breakables[spire].x, plan().breakables[spire].z);
        // He stands 6 m from the spire, she just beyond it: he charges through.
        f.varrak.x = sx;
        f.varrak.z = sz + 6.0;
        f.player.x = sx;
        f.player.z = sz - 3.0;
        f.varrak.yaw = std::f32::consts::PI;
        f.varrak.awake = true;
        let mut ev = Vec::new();
        f.varrak.start_drop(&mut ev);
        for _ in 0..(60 * 4) {
            f.update(Input::default(), 1.0 / 60.0);
            if f.varrak.state == VState::Stunned {
                break;
            }
        }
        assert_eq!(f.varrak.state, VState::Stunned);
        assert!(f.arena.broken[spire]);
        assert!(f.breaks.iter().any(|b| b.index == spire));
    }

    #[test]
    fn a_good_fighter_wins_it() {
        if !available() {
            return;
        }
        let mut f = Fight::new(4, false, 11);
        f.player.god = true;
        f.begin();
        for _ in 0..(60 * 360) {
            aim(&mut f);
            let input = bot(&f);
            f.update(input, 1.0 / 60.0);
            if matches!(f.phase, Phase::Won | Phase::Lost) {
                break;
            }
        }
        assert_eq!(
            f.phase,
            Phase::Won,
            "spark {} guard {} state {:?}",
            f.varrak.spark,
            f.varrak.guard,
            f.varrak.state
        );
        eprintln!(
            "[test] won after {:.0} s of fight, {} charges broke {} things",
            f.time,
            f.varrak.charges,
            f.breaks.len()
        );
    }

    #[test]
    fn he_hurts_her_and_she_can_fall() {
        if !available() {
            return;
        }
        let mut f = Fight::new(2, false, 5);
        f.begin();
        // Standing still in front of him, she is beaten in the end.
        for _ in 0..(60 * 240) {
            f.update(Input::default(), 1.0 / 60.0);
            if f.phase == Phase::Lost {
                break;
            }
        }
        assert_eq!(f.phase, Phase::Lost);
        assert_eq!(f.player.health, 0.0);
    }
}
