//! El combate contra Varrak en primera persona (ACTO1.md §8; el usuario, 2026-10-10), portado del
//! prototipo (`demon-queen-penascal-threejs/src/combate/`): la plaza del Peñascal al caer la noche,
//! primero la manada y después el orco. La espada le rompe la guardia (o se la rompe un peñasco si
//! se le lleva a cargar a cuatro patas contra él) y, mientras está aturdido, el sello le manda la
//! chispa hasta llenar la barra y despertarlo. Cada cuarto de barra le arranca un recuerdo.
//!
//! Aquí está lo que pasa: la plaza y sus obstáculos (`data/penascal/combate.json`, exportado del
//! prototipo con `tools/export-combate.mjs`), la Reina, Varrak, la manada, lo que vuela y hace daño
//! y las frases de Chiquita. Lo que se ve lo dibuja `zones/boss.rs`; el HUD, `overlay/boss_overlay.rs`.
use super::*;
use dungeon::combat::Fx;
use dungeon::fp;
use dungeon::sound::{Sfx, Sound};

/// The model is 0.71 m tall: 3.3 m in the fight.
pub(crate) const VARRAK_SCALE: f32 = 4.6;
/// Spark per second while it flows (a quarter takes ~3.6 s of a stun).
const SPARK_RATE: f32 = 7.0;
/// Stunned by the sword (his guard broken) he shakes it off sooner than after a crash.
const STUN_SECONDS: f32 = 3.6;
const CRASH_STUN_SECONDS: f32 = 6.5;
const GALLOP_SPEED: f32 = 12.5;
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
    /// A full turn with the cleaver out: what she gets for standing behind him.
    Spin,
    /// A leap at her that lands with the cleaver in the ground.
    Leap,
    /// He reaches for her with his free fist (she has stood too close too long).
    Reach,
    /// He holds her up in his fist before his face (a quick event decides).
    Grab,
    /// She is up on his back, the sword in his shoulder (a quick event decides).
    Ridden,
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
    /// The goblins' net of stones dropped on him.
    Net,
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
    /// A heavy footfall at (x, z): how heavy (dust and a tremor when she is near).
    Step(f32, f32, f32),
    Stunned(Why),
    Memory(usize),
    Awake,
    /// He has her in his fist.
    Grab,
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
    /// Spark per second while it flows (Nyxara's «chispa viva» quickens it).
    pub spark_rate: f32,
    /// Seconds added to every daze (her «sello firme»).
    pub stun_bonus: f32,
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
    /// Seconds left in the sticky sword's goo (everything he does at a slower pace).
    pub slowed: f32,
    /// Seconds left smouldering (the ember sword): his guard burns away.
    pub burning: f32,
    /// Off the ground in a leap (m).
    pub hop: f32,
    /// How his body is carried, for the renderer: leaning forward, rolled into a turn, thrown
    /// back by a blow (radians), and a clock for his breath.
    pub lean: f32,
    pub tilt: f32,
    pub flinch: f32,
    pub life: f32,
    last: (f32, f32, f32),
    /// Seconds she has stood behind him; blows taken in the back.
    behind: f32,
    rage: u32,
    /// Blows chained (a smash into a sweep, a charge into another).
    combo: u8,
    leap_ready: f32,
    leap_from: (f32, f32),
    leap_to: (f32, f32),
    step_t: f32,
    told_back: bool,
    told_leap: bool,
    /// Seconds since a blow last landed (left alone, his guard comes back).
    since_hit: f32,
    /// Seconds before a blow can make him stagger again (between, he only flinches).
    stagger_cd: f32,
    /// A quick event has him: only his animation goes on.
    pub held: bool,
    /// Seconds before he can reach for her again.
    grab_ready: f32,
    /// What stunned him last.
    pub why: Why,
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
            spark_rate: SPARK_RATE,
            stun_bonus: 0.0,
            cooldown: 1.5,
            charge_ready: 3.0,
            struck: false,
            gallop: Gallop::default(),
            awake: false,
            charges: 0,
            recover: 0.7,
            anim: Anim::new(Clip::Sit),
            blade: BladeMode::Ground,
            blade_fly: 0.0,
            slowed: 0.0,
            burning: 0.0,
            hop: 0.0,
            lean: 0.0,
            tilt: 0.0,
            flinch: 0.0,
            life: 0.0,
            last: (t.x, t.z, 0.0),
            behind: 0.0,
            rage: 0,
            combo: 0,
            leap_ready: 7.0,
            leap_from: (t.x, t.z),
            leap_to: (t.x, t.z),
            step_t: 0.0,
            told_back: false,
            told_leap: false,
            since_hit: 0.0,
            stagger_cd: 0.0,
            held: false,
            grab_ready: 9.0,
            why: Why::Guard,
        }
    }

    /// Where his root stands (on the stone while he sits on it; in the air mid-leap).
    pub(crate) fn root(&self) -> Vec3 {
        let t = plan().throne;
        let z = if matches!(self.state, VState::Throne | VState::ThroneRoar) {
            t.z + 0.18
        } else {
            self.z
        };
        Vec3::new(self.x, self.hop, z)
    }

    /// A step of his feet: every so often a footfall (dust and a tremor when running).
    fn footfall(&mut self, dt: f32, rate: f32, heavy: f32, ev: &mut Vec<Event>) {
        self.step_t += dt * rate;
        if self.step_t >= 1.0 {
            self.step_t -= 1.0;
            ev.push(Event::Step(self.x, self.z, heavy));
        }
    }

    /// She stands behind him: he whirls round with the cleaver out.
    fn start_spin(&mut self, ev: &mut Vec<Event>) {
        self.set(VState::Spin);
        self.anim.play(Clip::Sweep, 0.1, 1.3, false);
        self.rage = 0;
        self.behind = 0.0;
        ev.push(Event::Float("¡GRRR!", true, 1.2));
        ev.push(Event::Sfx(Sfx::Roar, Some(self.root()), 0.55));
    }

    /// From mid-range he leaps at her and lands cleaver first.
    fn start_leap(&mut self, px: f32, pz: f32, ev: &mut Vec<Event>) {
        self.set(VState::Leap);
        self.anim.play(Clip::Smash, 0.12, 1.0, false);
        self.leap_from = (self.x, self.z);
        self.leap_to = (px, pz);
        ev.push(Event::Sfx(Sfx::Roar, Some(self.root()), 0.6));
        ev.push(Event::Float("¡HRAAH!", true, 1.2));
        if !self.told_leap {
            self.told_leap = true;
            ev.push(Event::Chiquita(
                "¡Que salta! Apártate, y cuando caiga, Espacio para saltar la onda.",
                0.0,
                3.6,
            ));
        }
    }

    /// How he carries his body this step (for the renderer): forward into a run, rolled into a
    /// turn, a breath; thrown back by a blow.
    fn carry(&mut self, dt: f32) {
        self.life += dt;
        let (lx, lz, lyaw) = self.last;
        let speed = (self.x - lx).hypot(self.z - lz) / dt.max(1e-3);
        let turn = wrap(self.yaw - lyaw) / dt.max(1e-3);
        self.last = (self.x, self.z, self.yaw);
        let upright = !self.on_fours();
        let lean = if upright {
            (speed * 0.035).min(0.2)
        } else {
            0.0
        } + match self.state {
            VState::Leap if self.t < 0.3 => 0.18,
            VState::Leap => -0.12,
            VState::Roar => -0.08,
            _ => 0.0,
        };
        let tilt = if upright {
            (-turn * 0.05).clamp(-0.16, 0.16)
        } else {
            0.0
        };
        let k = (dt * 8.0).min(1.0);
        self.lean += (lean - self.lean) * k;
        self.tilt += (tilt - self.tilt) * k;
        self.flinch = (self.flinch - dt * 3.5).max(0.0);
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
    fn strike(
        &mut self,
        eye: Vec3,
        dir: Vec3,
        power: f32,
        fx: dungeon::WeaponFx,
        ev: &mut Vec<Event>,
    ) -> bool {
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
        if fx.slow > 0.0 {
            self.slowed = fx.slow;
            ev.push(Event::Burst(at, [0.45, 0.95, 0.42], 14, 2.2));
        }
        if fx.burn > 0.0 {
            self.burning = fx.burn;
            ev.push(Event::Burst(at, [1.0, 0.55, 0.15], 16, 3.0));
        }
        let mut loss = 5.0 * power;
        // A blow in the opening after his own swing bites deeper.
        let opening = (self.state == VState::Smash && self.t > 1.0)
            || (self.state == VState::Sweep && self.t > 0.9)
            || (self.state == VState::Leap && self.t > 1.0)
            || matches!(self.state, VState::Recover | VState::GetUp | VState::Dazed);
        if opening {
            loss *= 1.5;
        }
        // His back is hide like rock: a blow there hardly counts, and it makes him turn.
        if !opening && self.off_facing(eye.x, eye.z).abs() > 1.9 {
            loss *= 0.3;
            self.rage += 1;
            if !self.told_back {
                self.told_back = true;
                ev.push(Event::Chiquita(
                    "¡Por la espalda solo le haces cosquillas! Tiene el lomo de piedra: haz que se estrelle.",
                    0.0,
                    4.2,
                ));
            }
        }
        self.flinch = 1.0;
        self.since_hit = 0.0;
        self.guard = (self.guard - loss).max(0.0);
        ev.push(Event::Sfx(Sfx::HitBone, Some(at), 1.0));
        ev.push(Event::Burst(at, [1.0, 0.81, 0.54], 16, 4.0));
        if self.guard <= 0.0 {
            self.stun(STUN_SECONDS, Why::Guard, ev);
        } else if matches!(self.state, VState::Chase | VState::Recover) && self.stagger_cd <= 0.0 {
            // He staggers now and then; between, blows only make him flinch and come on.
            self.stagger_cd = 3.5;
            self.set(VState::Hit);
            self.anim.play(Clip::Hit, 0.08, 1.0, false);
        }
        true
    }

    fn stun(&mut self, seconds: f32, why: Why, ev: &mut Vec<Event>) {
        self.set(VState::Stunned);
        self.why = why;
        self.stun_left = seconds + self.stun_bonus;
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
        self.spark = (self.spark + self.spark_rate * dt).min(next.map_or(100.0, |m| m.at));
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
        self.combo = 0;
        self.cooldown = 0.25 + rng.next() * 0.45;
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
        // In the goo he goes at a slower pace; the embers eat at his guard.
        let slow = self.slowed > 0.0;
        self.slowed = (self.slowed - dt).max(0.0);
        if self.burning > 0.0 {
            self.burning = (self.burning - dt).max(0.0);
            if self.fighting() && !self.open() && self.guard > 0.0 {
                self.guard = (self.guard - 3.0 * dt).max(0.0);
                if self.guard <= 0.0 {
                    self.stun(STUN_SECONDS, Why::Guard, ev);
                }
            }
        }
        let dt = if slow { dt * 0.6 } else { dt };
        self.carry(dt);
        if self.held {
            // A quick event has him (she rides him, he holds her, he kneels): it decides.
            self.t += dt;
            self.anim.advance(dt);
            if self.blade_fly > 0.0 {
                self.blade_fly -= dt;
            }
            return;
        }
        // Left alone a while, his guard closes again: the sword alone is slow work.
        self.since_hit += dt;
        self.stagger_cd = (self.stagger_cd - dt).max(0.0);
        if self.awake && !matches!(self.state, VState::Drop | VState::Gallop) {
            self.charge_ready -= dt;
            self.leap_ready -= dt;
            self.grab_ready -= dt;
        }
        if self.awake && self.fighting() && !self.open() && self.since_hit > 2.5 {
            self.guard = (self.guard + 5.0 * dt).min(100.0);
        }
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
                let off = self.turn_to(px, pz, 3.0, dt);
                // Behind him she does not stay long.
                if self.off_facing(px, pz).abs() > 1.9 && d < 4.2 {
                    self.behind += dt;
                } else {
                    self.behind = (self.behind - dt).max(0.0);
                }
                if (self.behind > 0.55 || self.rage >= 3) && d < 5.2 {
                    self.start_spin(ev);
                } else if self.spark >= 50.0
                    && self.grab_ready <= 0.0
                    && d < 2.8
                    && off < 0.5
                    && p.y < 0.6
                {
                    // From halfway on: stand in his face and he grabs at her.
                    self.set(VState::Reach);
                    self.anim.play(Clip::Sweep, 0.15, 0.7, false);
                    ev.push(Event::Sfx(Sfx::EnemySwing, Some(self.root()), 0.8));
                    ev.push(Event::Float("¡TE TENGO!", true, 1.4));
                } else if self.charge_ready <= 0.0 && off < 0.7 && (d > 5.0 || self.cooldown <= 0.0)
                {
                    // Down on all fours and charge (from close, he backs off first).
                    self.start_drop(ev);
                } else if d > 6.0 && d < 12.0 && self.leap_ready <= 0.0 && off < 0.5 {
                    self.start_leap(px, pz, ev);
                } else if d < 4.8 && off < 0.55 && self.cooldown <= 0.0 {
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
                } else if d > 3.0 {
                    // Far off he comes at a run.
                    let run = d > 6.0;
                    self.anim
                        .play(Clip::Walk, 0.2, if run { 1.75 } else { 1.3 }, false);
                    self.step(arena, if run { 4.8 } else { 3.3 }, dt, 1.05);
                    self.footfall(
                        dt,
                        if run { 2.6 } else { 1.9 },
                        if run { 1.0 } else { 0.55 },
                        ev,
                    );
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
                // Still in front of him: the swing goes on into a sweep.
                if self.t > 1.45
                    && self.combo == 0
                    && d < 4.8
                    && self.off_facing(px, pz).abs() < 0.9
                    && rng.next() < 0.55
                {
                    self.combo = 1;
                    self.set(VState::Sweep);
                    self.anim.play(Clip::Sweep, 0.12, 1.15, false);
                    ev.push(Event::Sfx(Sfx::EnemySwing, Some(self.root()), 0.7));
                } else if self.t > 1.75 {
                    self.after(0.8, rng);
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
                if self.t > 1.25 && self.combo == 0 && d < 4.6 && rng.next() < 0.4 {
                    self.combo = 1;
                    self.set(VState::Smash);
                    self.anim.play(Clip::Smash, 0.12, 1.15, false);
                    ev.push(Event::Sfx(Sfx::EnemySwing, Some(self.root()), 0.7));
                } else if self.t > 1.5 {
                    self.after(0.7, rng);
                }
            }
            VState::Spin => {
                // A whole turn, cleaver out: nobody stands within its reach.
                if self.t > 0.2 && self.t < 0.85 {
                    self.yaw = wrap(self.yaw + std::f32::consts::TAU / 0.65 * dt);
                }
                if self.t > 0.3 && self.t - dt <= 0.3 {
                    ev.push(Event::Ring(self.x, self.z, 5.5));
                    ev.push(Event::Sfx(Sfx::EnemySwing, Some(self.root()), 1.0));
                }
                if !self.struck && self.t > 0.35 && self.t < 0.8 && d < 5.0 && p.y < 1.0 {
                    self.struck = true;
                    p.hurt(20.0, self.x, self.z, 13.0);
                }
                if self.t > 1.25 {
                    self.after(0.5, rng);
                }
            }
            VState::Leap => {
                if self.t < 0.3 {
                    // He crouches and fixes on where she is.
                    self.turn_to(px, pz, 4.0, dt);
                    self.leap_from = (self.x, self.z);
                    self.leap_to = (px, pz);
                } else if self.t < 0.95 {
                    let k = ((self.t - 0.3) / 0.65).clamp(0.0, 1.0);
                    let (fx, fz) = self.leap_from;
                    let (tx, tz) = self.leap_to;
                    // He lands a stride short of her, the cleaver coming down on her.
                    let dd = (tx - fx).hypot(tz - fz).max(0.01);
                    let reach = ((dd - 2.2) / dd).max(0.0) * k;
                    let c = arena.collide(fx + (tx - fx) * reach, fz + (tz - fz) * reach, 1.0);
                    self.x = c.x;
                    self.z = c.z;
                    self.hop = 2.6 * 4.0 * k * (1.0 - k);
                } else if !self.struck {
                    self.struck = true;
                    self.hop = 0.0;
                    ev.push(Event::Smash(
                        self.x + self.yaw.sin() * 1.8,
                        self.z + self.yaw.cos() * 1.8,
                    ));
                    ev.push(Event::Ring(self.x, self.z, 6.0));
                    if d < 3.8 && p.y < 0.6 {
                        p.hurt(30.0, self.x, self.z, 14.0);
                    }
                }
                if self.t > 1.9 {
                    self.leap_ready = 7.0 + rng.next() * 3.0;
                    self.after(0.9, rng);
                }
            }
            VState::Reach => {
                if self.t < 0.35 {
                    self.turn_to(px, pz, 3.2, dt);
                }
                if self.t >= 0.55 && !self.struck {
                    self.struck = true;
                    if d < 3.0
                        && self.off_facing(px, pz).abs() < 0.75
                        && p.y < 0.9
                        && p.vulnerable
                        && p.untouchable <= 0.0
                    {
                        self.set(VState::Grab);
                        self.held = true;
                        self.anim.play(Clip::Roar, 0.2, 0.8, false);
                        ev.push(Event::Grab);
                    } else {
                        self.grab_ready = 6.0;
                    }
                }
                if self.state == VState::Reach && self.t > 1.1 {
                    self.grab_ready = self.grab_ready.max(6.0);
                    self.after(0.5, rng);
                }
            }
            VState::Grab | VState::Ridden => {
                // Let go without a verdict (it should not happen): back to the fight.
                self.after(0.4, rng);
            }
            VState::Drop => {
                // He paws the ground and fixes on her; too close, he backs off for a run-up.
                self.turn_to(px, pz, 2.4, dt);
                if d < 6.5 && self.t < 1.0 {
                    self.step(arena, -4.5, dt, 1.05);
                    self.footfall(dt, 3.0, 0.7, ev);
                }
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
                    if self.t > 0.6 || d < 7.0 {
                        self.gallop.locked = true;
                    }
                }
                self.footfall(dt, 5.5, 1.3, ev);
                // Whatever is in the way flies to pieces; a spire stops him dead.
                let (fx, fz) = (self.x + self.yaw.sin() * 0.9, self.z + self.yaw.cos() * 0.9);
                for thing in arena.touching(fx, fz, 1.1) {
                    arena.broken[thing] = true;
                    if matches!(plan().breakables[thing].kind.as_str(), "spire" | "cauldron") {
                        ev.push(Event::BreakSpire(thing));
                        self.guard = 0.0;
                        self.stun(CRASH_STUN_SECONDS, Why::Spire, ev);
                        self.combo = 0;
                        self.charge_ready = 4.0 + rng.next() * 2.0;
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
                    self.guard = (self.guard - 55.0).max(0.0);
                    self.combo = 0;
                    self.charge_ready = 4.0 + rng.next() * 2.0;
                    if self.guard <= 0.0 {
                        self.stun(CRASH_STUN_SECONDS, Why::Cliff, ev);
                    } else {
                        self.set(VState::Dazed);
                        self.anim.play(Clip::Stun, 0.15, 1.0, false);
                        self.blade = BladeMode::Hand;
                        self.blade_fly = 0.3;
                        ev.push(Event::Float("…grr…", true, 2.0));
                    }
                } else if self.gallop.left <= 0.0 || (self.gallop.hit_her && self.t > 1.0) {
                    if self.combo == 0 && rng.next() < 0.45 {
                        // He skids round and comes again at once.
                        self.combo = 1;
                        ev.push(Event::Float("¡GRAAAH!", true, 1.4));
                        self.start_drop(ev);
                        self.t = 0.5;
                    } else {
                        self.combo = 0;
                        self.charge_ready = 3.0 + rng.next() * 2.0;
                        self.get_up(0.6);
                    }
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
                self.turn_to(px, pz, 2.6, dt);
                if self.off_facing(px, pz).abs() > 1.9 && d < 4.2 {
                    self.behind += dt;
                }
                if self.behind > 0.8 && d < 5.0 {
                    self.start_spin(ev);
                } else if self.t > self.recover {
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
    /// Seconds left in the sticky sword's goo.
    slowed: f32,
    /// Seconds left smouldering.
    burning: f32,
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
            slowed: 0.0,
            burning: 0.0,
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
    fn strike(
        &mut self,
        eye: Vec3,
        dir: Vec3,
        power: f32,
        fx: dungeon::WeaponFx,
        ev: &mut Vec<Event>,
    ) -> usize {
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
            if fx.slow > 0.0 {
                w.slowed = fx.slow;
                ev.push(Event::Burst(
                    Vec3::new(w.x, 0.8, w.z),
                    [0.45, 0.95, 0.42],
                    10,
                    1.8,
                ));
            }
            if fx.burn > 0.0 {
                w.burning = fx.burn;
                ev.push(Event::Burst(
                    Vec3::new(w.x, 0.8, w.z),
                    [1.0, 0.55, 0.15],
                    10,
                    2.4,
                ));
            }
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
            let slow = self.wolves[i].slowed > 0.0;
            {
                let w = &mut self.wolves[i];
                w.slowed = (w.slowed - dt).max(0.0);
                if w.burning > 0.0 {
                    w.burning = (w.burning - dt).max(0.0);
                    w.hp = (w.hp - 4.0 * dt).max(1.0);
                }
            }
            let dt = if slow { dt * 0.5 } else { dt };
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

// ------------------------------------------------------------------ the quick events
/// The quick events (DemonQueen-Lore/VARRAK_COMBATE_2.md): earned, rare, on the fight's own keys,
/// slowed down and pinned to where it happens; failing one costs a blow, never a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QteKind {
    /// He is about to run her down on all fours: Space, and she leaps aside.
    Dodge,
    /// Crashed and dazed: up on his back, the sword in his shoulder, the seal on his nape.
    Mount,
    /// He has her in his fist: the sword in his hand to get free.
    Grab,
    /// He walks under the goblins' net of stones while she is by its post: cut the rope.
    Rope,
    /// The spark is full and he kneels: hold the seal, then call him by his name.
    Awaken,
}

/// What a quick event asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum QteAsk {
    /// A click before the window closes.
    Press,
    /// Space before the window closes.
    Jump,
    /// A click when the closing ring meets its mark (`beat` s in, give or take `slack`).
    Beat { beat: f32, slack: f32 },
    /// The seal held `need` s in all (it drains `decay` a second let go).
    Hold { need: f32, decay: f32 },
    /// Clicks (`per` each) or the seal held (`hold` a second) to fill it.
    Mash { per: f32, hold: f32 },
}

/// A quick event on show.
#[derive(Clone, Debug)]
pub(crate) struct Qte {
    pub kind: QteKind,
    pub ask: QteAsk,
    /// Real seconds into it, and how long it waits (0: for ever).
    pub t: f32,
    pub window: f32,
    /// 0..1 (held, mashed).
    pub fill: f32,
    /// Which step of how many (the ride is four: the jump up, two stabs, the seal).
    pub step: u8,
    pub steps: u8,
    /// The clock's pace while it is up.
    pub slow: f32,
    /// Where it happens (the prompt sits there).
    pub anchor: Vec3,
    /// The key, what to do, a line under.
    pub key: &'static str,
    pub text: &'static str,
    pub sub: &'static str,
}

/// Up on his back or held in his fist: which, for how long, how hard he bucks (the camera).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ride {
    pub kind: QteKind,
    pub t: f32,
    pub buck: f32,
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

/// How long the pack holds the plaza before he rises anyway.
const PACK_SECS: f32 = 25.0;

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
    /// The sword she wears: how deep it cuts and what it carries (from the backpack).
    pub blade_damage: f32,
    pub blade: dungeon::WeaponFx,
    /// The last cure taken (the next one waits a moment).
    cured_at: f32,
    /// When he last crashed into rock (Chiquita reminds her how to win if it is long ago).
    last_crash: f32,
    crash_hints: usize,
    /// The quick event up, the clock's pace (slow while one is up), her ride on him or in his
    /// fist (the camera follows him then).
    pub qte: Option<Qte>,
    pub time_scale: f32,
    pub ride: Option<Ride>,
    /// Real seconds (quick events time themselves in it), and the last verdict: good, when.
    pub clock: f32,
    pub stamp: Option<(bool, f32)>,
    dodge_cd: f32,
    rope_cd: f32,
    /// The goblins' net is down, and when it lands.
    pub rope_cut: bool,
    net_falls: Option<f32>,
    /// The ride was offered in this daze.
    mounted: bool,
    /// Chiquita has told her about each quick event once.
    taught: [bool; 5],
    last_dodge: f32,
    /// The seal held in the last call (the spark pours in the awakening).
    pouring: bool,
    /// The cauldron's oil burning on the ground: where, how far, until when.
    pub spill: Option<(f32, f32, f32, f32)>,
    told_fire: bool,
    /// The last phase (three memories out): the cliffs shake stones loose on her, each marked
    /// on the ground before it lands. Seconds to the next one.
    rockfall: f32,
    told_rockfall: bool,
    /// Sparks off his nape while the seal is held there.
    nape_fx: f32,
    /// Where a stone will land: the mark on the ground (x, z, radius, when it lands).
    pub marks: Vec<(f32, f32, f32, f32)>,
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
            blade_damage: 1.0,
            blade: dungeon::WeaponFx::default(),
            cured_at: -99.0,
            last_crash: 0.0,
            crash_hints: 0,
            qte: None,
            time_scale: 1.0,
            ride: None,
            clock: 0.0,
            stamp: None,
            dodge_cd: 0.0,
            rope_cd: 0.0,
            rope_cut: false,
            net_falls: None,
            mounted: false,
            taught: [false; 5],
            last_dodge: -99.0,
            pouring: false,
            spill: None,
            told_fire: false,
            rockfall: 3.0,
            told_rockfall: false,
            nape_fx: 0.0,
            marks: Vec::new(),
        }
    }

    /// Whether a cure would do anything now (hurt, in the fight, not just after another).
    /// Nyxara's gifts in this fight: the spark flows `spark` times as fast, every daze lasts
    /// `stun` seconds longer, and her vitality is `vitality` times as much.
    pub(crate) fn set_gifts(&mut self, spark: f32, stun: f32, vitality: f32) {
        self.varrak.spark_rate = SPARK_RATE * spark;
        self.varrak.stun_bonus = stun;
        let full = self.player.health >= self.player.max_health;
        self.player.max_health = 100.0 * vitality;
        if full {
            self.player.health = self.player.max_health;
        }
    }

    pub(crate) fn can_cure(&self) -> bool {
        matches!(self.phase, Phase::Pack | Phase::Duel)
            && self.player.health > 0.0
            && self.player.health < self.player.max_health
            && self.time - self.cured_at > 1.2
    }

    /// A line from Chiquita now.
    pub(crate) fn say(&mut self, text: &str) {
        self.chiquita(text, 0.0, 2.8);
    }

    /// Something from the backpack drunk or eaten mid-fight.
    pub(crate) fn cure(&mut self, amount: f32) {
        self.cured_at = self.time;
        self.player.heal(amount);
        self.sfx(Sfx::Drink, None, 0.8);
        let line = if amount >= 80.0 {
            "Eso es. Hasta la última gota: este cuerpo lo quiero entero."
        } else if amount >= 40.0 {
            "Bebe, bebe. Y no me lo vuelvas a abollar."
        } else {
            "Algo es algo. Mastica rápido."
        };
        self.chiquita(line, 0.0, 2.8);
    }

    fn chiquita(&mut self, text: &str, delay: f32, keep: f32) {
        self.lines.push_back(Line {
            // Typed out letter by letter: in the player's language from the start.
            text: demon_queen_dungeon::i18n::tr_string(text),
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

    /// What she should do now, for the HUD: a heading, its detail, and how urgent it is
    /// (0 the usual, 1 an opening to take, 2 a danger to dodge).
    pub(crate) fn objective(&self) -> Option<(String, String, u8)> {
        let v = &self.varrak;
        Some(match self.phase {
            Phase::Intro | Phase::Won | Phase::Lost => return None,
            Phase::Pack => {
                let left = self.pack.fighting();
                let wait = (PACK_SECS - (self.time - self.started)).max(0.0).ceil() as u32;
                if left == 0 {
                    (
                        "Varrak se levanta".into(),
                        "Prepárate: viene a por ti.".into(),
                        0,
                    )
                } else {
                    (
                        format!("Acaba con la manada · {left}"),
                        format!(
                            "Clic: espada. Varrak se levanta en {wait} s o al caer el último lobo."
                        ),
                        0,
                    )
                }
            }
            Phase::Duel => match v.state {
                VState::Rise | VState::Roar => (
                    "Varrak se levanta".into(),
                    "Aléjate y mira su emblema: es lo que hay que despertar.".into(),
                    0,
                ),
                VState::Stunned if v.why != Why::Guard && !self.mounted => (
                    "¡Estrellado! Súbete o el sello".into(),
                    "Acércate y clic: a su lomo (vale más). O mantén clic derecho a su emblema."
                        .into(),
                    1,
                ),
                VState::Stunned | VState::Memory => (
                    "¡Aturdido! Mantén el sello".into(),
                    "Mantén clic derecho apuntando a su emblema hasta que lo suelte.".into(),
                    1,
                ),
                VState::Reach => ("¡Va a agarrarte!".into(), "Apártate de su puño.".into(), 2),
                VState::Grab | VState::Ridden | VState::Kneel => return None,
                VState::Drop | VState::Gallop => (
                    "¡Carga a cuatro patas!".into(),
                    "Ponte delante de un peñasco y apártate (Mayús) en el último momento.".into(),
                    2,
                ),
                VState::Smash => (
                    "¡Golpe al suelo!".into(),
                    "Salta la onda con Espacio.".into(),
                    2,
                ),
                VState::Spin => (
                    "¡Gira con el hacha!".into(),
                    "Fuera de su alcance. Por la espalda no se le pega.".into(),
                    2,
                ),
                VState::Leap => (
                    "¡Salta hacia ti!".into(),
                    "Apártate y, al caer, salta la onda con Espacio.".into(),
                    2,
                ),
                VState::Dazed | VState::Recover => (
                    "Está atontado".into(),
                    "¡Ataca! Clic con la espada para romperle la guardia.".into(),
                    1,
                ),
                _ => (
                    "Rómpele la guardia".into(),
                    format!(
                        "Clic: espada. Guardia {}%. Aturdido, la chispa le devuelve un recuerdo.",
                        v.guard.round() as i32
                    ),
                    0,
                ),
            },
        })
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
        self.last_crash = self.time;
        self.chiquita(
            "Ahí viene. Nunca por la espalda: gira. Que se estrelle contra las agujas, el caldero o la pared, o llévalo bajo la red de la grúa.",
            1.0,
            4.6,
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
        let power = power * self.blade_damage;
        let wolves = self.pack.strike(eye, dir, power, self.blade, ev);
        let him = self.varrak.strike(eye, dir, power, self.blade, ev);
        if (wolves > 0 || him) && self.blade.leech > 0.0 {
            // The heart sword drinks a little back.
            self.player.heal(self.blade.leech);
        }
        if wolves > 0 || him {
            // The blade bites: a beat of hit-stop and a small jolt.
            self.player.impact(if him { 0.1 } else { 0.065 });
            self.player.kick(0.12, 0.05);
        }
    }

    /// One step of the fight.
    pub(crate) fn update(&mut self, input: Input, dt: f32) {
        self.sounds.clear();
        let mut ev = Vec::new();
        let fighting = matches!(self.phase, Phase::Pack | Phase::Duel);
        let mut input = if self.phase == Phase::Intro {
            Input::default()
        } else {
            input
        };
        // The quick event takes the keys it asks for first, and slows the clock while it is up.
        let real = dt;
        self.clock += real;
        self.update_qte(&mut input, real, &mut ev);
        let pace = self.qte.as_ref().map_or(1.0, |q| q.slow);
        self.time_scale += (pace - self.time_scale) * (real * 12.0).min(1.0);
        let dt = real * self.time_scale;
        self.time += dt;
        if let Some(power) = self.player.update(input, dt, &self.arena) {
            self.strike(power, &mut ev);
        }
        // He rises when the pack is down, or once she has had a fair while with it.
        if self.phase == Phase::Pack
            && self.time - self.started > 3.5
            && (self.pack.fighting() == 0 || self.time - self.started > PACK_SECS)
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
        if self.pouring {
            // The awakening: the seal pours into him on its own (no aim needed now).
            self.flowing = true;
            self.fizzle = false;
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
        // Long without a crash: Chiquita reminds her what beats him.
        if self.phase == Phase::Duel
            && self.varrak.awake
            && !self.varrak.open()
            && self.time - self.last_crash > 24.0
        {
            self.last_crash = self.time;
            let lines = [
                "La espada sola no lo tumba. ¡Que cargue contra una aguja de roca!",
                "Ponte delante del risco. Cuando cargue, Mayús y fuera de su camino.",
                "Aléjate y se pondrá a cuatro patas. Eso es lo que quieres.",
            ];
            let line = lines[self.crash_hints % lines.len()];
            self.crash_hints += 1;
            self.chiquita(line, 0.0, 4.0);
        }
        self.update_effects(dt);
        // The burning oil: it hurts her if she stands in it, and eats his guard while he is up.
        if let Some((sx, sz, r, until)) = self.spill {
            if self.time > until {
                self.spill = None;
            } else if fighting && self.ride.is_none() {
                let p = &mut self.player;
                if (p.x - sx).hypot(p.z - sz) < r
                    && p.y < 0.6
                    && p.untouchable <= 0.0
                    && p.hurt(5.0, sx, sz, 2.0)
                {
                    if !self.told_fire {
                        self.told_fire = true;
                        self.chiquita("¡Que quema! ¡Fuera del aceite!", 0.0, 2.6);
                    }
                }
                let v = &mut self.varrak;
                if (v.x - sx).hypot(v.z - sz) < r + 0.6 && v.fighting() && !v.open() && !v.held {
                    v.burning = v.burning.max(3.0);
                }
            }
        }
        // The last phase: stones off the cliffs, a mark on the ground where each will land.
        if self.phase == Phase::Duel
            && self.varrak.memories >= MEMORIES.len()
            && self.varrak.fighting()
            && !self.varrak.held
            && self.ride.is_none()
        {
            self.rockfall -= dt;
            if self.rockfall <= 0.0 {
                self.rockfall = 4.0 + self.rng.next() * 3.0;
                if !self.told_rockfall {
                    self.told_rockfall = true;
                    self.lines.clear();
                    self.line = None;
                    self.chiquita(
                        "¡El risco se viene abajo! Donde veas el círculo, cae una piedra.",
                        0.0,
                        3.6,
                    );
                }
                for k in 0..2 {
                    let a = self.rng.next() * std::f32::consts::TAU;
                    let d = if k == 0 {
                        self.rng.next() * 1.2
                    } else {
                        2.5 + self.rng.next() * 2.5
                    };
                    let to = self.arena.collide(
                        self.player.x + a.cos() * d,
                        self.player.z + a.sin() * d,
                        0.6,
                    );
                    let r = 0.5 + self.rng.next() * 0.4;
                    let delay = 0.5 + k as f32 * 0.5;
                    self.rocks.push(Rock {
                        at: Vec3::new(to.x, 15.0, to.z),
                        v: Vec3::ZERO,
                        r,
                        spin: Vec3::new(2.0, 1.0, 3.0),
                        delay,
                        falling: true,
                        rest: false,
                    });
                    // 15 m of fall takes ~1.3 s.
                    self.marks
                        .push((to.x, to.z, r + 1.2, self.time + delay + 1.3));
                }
                let a = angle_of(self.player.x, self.player.z);
                let (cx, cz) = ring_point(a, 0.5);
                self.sfx(Sfx::Rubble, Some(Vec3::new(cx, 6.0, cz)), 0.6);
            }
        }
        let now = self.time;
        self.marks.retain(|m| m.3 > now - 0.2);
        // The goblins' net lands.
        if self.net_falls.is_some_and(|at| self.time >= at) {
            self.net_falls = None;
            self.net_lands();
        }
        self.offer_qte();
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
                self.last_crash = self.time;
                let b = &plan().breakables[i];
                let (bx, bz) = (b.x, b.z);
                if b.kind == "cauldron" {
                    // The oil goes over the ground and catches: a patch of fire for a while.
                    self.spill = Some((bx, bz, 3.4, self.time + 16.0));
                    self.sfx(Sfx::SpellBurst, Some(Vec3::new(bx, 0.6, bz)), 1.0);
                    self.chiquita(
                        "¡El caldero! Ahora arde todo eso: no te metas en el fuego.",
                        0.4,
                        3.4,
                    );
                }
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
            Event::Step(x, z, heavy) => {
                let d = (self.player.x - x).hypot(self.player.z - z);
                if d < 16.0 {
                    self.player.kick(0.12, 0.035 * heavy * (1.0 - d / 16.0));
                }
                if heavy >= 1.0 {
                    let seed = (self.rng.next() * 1e6) as u32;
                    self.fx.push(Fx::dust(Vec3::new(x, 0.15, z), seed));
                }
                self.sfx(Sfx::Land, Some(Vec3::new(x, 0.1, z)), 0.3 * heavy.min(1.2));
            }
            Event::RamCliff(x, z) => {
                self.last_crash = self.time;
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
                self.mounted = false;
                let line = match why {
                    _ if why != Why::Guard && self.clock - self.last_dodge < 3.0 => {
                        "¡Olé! Se ha comido la roca. ¡Súbete o el sello!"
                    }
                    Why::Guard => "¡Ahora! El sello.",
                    Why::Spire => "¡Se ha dado de morros contra la roca! ¡El sello, ya!",
                    Why::Cliff => "¡Contra la pared! Ahora, el sello.",
                    Why::Net => "¡Enterrado en piedras! ¡Súbete a su lomo, ya!",
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
            Event::Grab => self.grabbed(),
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
            "smash" | "sweep" | "gallop" | "spire" | "cauldron" | "spark" | "memory" | "won"
            | "spin" | "leap" | "run" => {
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
                    "spin" => {
                        // She stands at his back: he whirls round.
                        in_front(self, -2.6);
                        self.varrak.start_spin(&mut ev);
                    }
                    "leap" => {
                        in_front(self, 9.0);
                        let (px, pz) = (self.player.x, self.player.z);
                        self.varrak.start_leap(px, pz, &mut ev);
                    }
                    "run" => {
                        in_front(self, 12.0);
                        self.varrak.set(VState::Chase);
                        self.varrak.cooldown = 9.0;
                        self.varrak.charge_ready = 9.0;
                        self.varrak.leap_ready = 9.0;
                    }
                    "gallop" => {
                        in_front(self, 11.0);
                        self.player.x += 3.0;
                        self.varrak.start_drop(&mut ev);
                    }
                    "spire" | "cauldron" => {
                        // A spire (or the cauldron) between him and her, he charges.
                        if let Some(i) = plan().breakables.iter().position(|b| b.kind == what) {
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
            "dodge" | "mount" | "ride" | "grab" | "rope" | "awaken" | "rockfall" => {
                for w in &mut self.pack.wolves {
                    w.gone = true;
                    w.visible = false;
                }
                self.phase = Phase::Duel;
                self.varrak.awake = true;
                self.varrak.blade = BladeMode::Hand;
                self.varrak.held = false;
                self.qte = None;
                self.ride = None;
                self.player.frozen = false;
                self.varrak.x = 0.0;
                self.varrak.z = -1.0;
                self.varrak.yaw = 0.0;
                match what {
                    "dodge" => {
                        // He charges at her from far off, a spire behind her.
                        in_front(self, 11.0);
                        self.dodge_cd = 0.0;
                        self.varrak.start_drop(&mut ev);
                    }
                    "mount" | "ride" => {
                        in_front(self, 2.6);
                        self.varrak.spark = 30.0;
                        self.varrak.memories = 1;
                        self.varrak.stun(30.0, Why::Spire, &mut ev);
                        self.mounted = false;
                        if what == "ride" {
                            self.mounted = true;
                            self.mount(&mut ev);
                        }
                    }
                    "rockfall" => {
                        // The last phase: he keeps off a while, the cliffs let stones go.
                        self.varrak.z = -8.0;
                        in_front(self, 9.0);
                        self.player.yaw = 0.0;
                        self.varrak.spark = 78.0;
                        self.varrak.memories = MEMORIES.len();
                        self.varrak.set(VState::Chase);
                        self.varrak.cooldown = 9.0;
                        self.varrak.charge_ready = 9.0;
                        self.varrak.leap_ready = 9.0;
                        self.rockfall = 0.2;
                    }
                    "grab" => {
                        in_front(self, 2.2);
                        self.varrak.spark = 55.0;
                        self.varrak.memories = 2;
                        self.varrak.set(VState::Grab);
                        self.varrak.held = true;
                        self.varrak.anim.play(Clip::Roar, 0.0, 0.8, false);
                        ev.push(Event::Grab);
                    }
                    "rope" => {
                        if let Some((rope, net)) = self.rope_and_net() {
                            let (r, n) = (&plan().breakables[rope], &plan().breakables[net]);
                            self.player.x = r.x + 0.5;
                            self.player.z = r.z + 0.5;
                            self.varrak.x = n.x - 5.0;
                            self.varrak.z = n.z - 0.5;
                            self.varrak.set(VState::Chase);
                            self.varrak.cooldown = 9.0;
                            self.varrak.charge_ready = 9.0;
                            self.varrak.leap_ready = 9.0;
                            self.rope_cd = 0.0;
                        }
                    }
                    _ => {
                        in_front(self, 5.0);
                        self.varrak.spark = 100.0;
                        self.varrak.memories = MEMORIES.len();
                        self.varrak.set(VState::Kneel);
                        self.varrak.anim.play(Clip::Kneel, 0.0, 1.0, false);
                        self.varrak.blade = BladeMode::Dropped;
                    }
                }
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

// ------------------------------------------------------------------ the quick events at work
impl Fight {
    #[allow(clippy::too_many_arguments)]
    fn ask(
        &mut self,
        kind: QteKind,
        ask: QteAsk,
        window: f32,
        slow: f32,
        (step, steps): (u8, u8),
        anchor: Vec3,
        key: &'static str,
        text: &'static str,
        sub: &'static str,
    ) {
        self.qte = Some(Qte {
            kind,
            ask,
            t: 0.0,
            window,
            fill: 0.0,
            step,
            steps,
            slow,
            anchor,
            key,
            text,
            sub,
        });
    }

    /// Chiquita tells her about a quick event the first time it comes up.
    fn teach(&mut self, kind: QteKind, line: &'static str) {
        let i = kind as usize;
        if !self.taught[i] {
            self.taught[i] = true;
            // A quick event will not wait for the line before it: this one goes first.
            self.lines.clear();
            self.line = None;
            self.chiquita(line, 0.0, 3.2);
        }
    }

    /// The goblins' rope post and their net of stones, if the plaza has them.
    fn rope_and_net(&self) -> Option<(usize, usize)> {
        let b = &plan().breakables;
        let rope = b.iter().position(|b| b.kind == "rope")?;
        let net = b.iter().position(|b| b.kind == "net")?;
        Some((rope, net))
    }

    /// The back of his neck (where the seal goes when she rides him).
    pub(crate) fn nape(&self) -> Vec3 {
        let v = &self.varrak;
        let r = v.root();
        Vec3::new(r.x - v.yaw.sin() * 0.3, r.y + 2.75, r.z - v.yaw.cos() * 0.3)
    }

    /// His face (where she hangs in his fist).
    pub(crate) fn face(&self) -> Vec3 {
        let v = &self.varrak;
        let r = v.root();
        Vec3::new(
            r.x + v.yaw.sin() * 0.35,
            r.y + 2.95,
            r.z + v.yaw.cos() * 0.35,
        )
    }

    /// Takes the keys the quick event asks for, times it and decides it.
    fn update_qte(&mut self, input: &mut Input, real: f32, ev: &mut Vec<Event>) {
        let seal = input.channel;
        self.dodge_cd -= real;
        self.rope_cd -= real;
        self.pouring = false;
        if let Some(r) = self.ride.as_mut() {
            r.t += real;
            r.buck = (r.buck - real * 2.2).max(0.0);
            // Up there nothing else reaches her.
            self.player.untouchable = self.player.untouchable.max(0.2);
        }
        let Some(mut q) = self.qte.take() else {
            return;
        };
        q.t += real;
        let (click, jump) = (input.attack, input.jump);
        match q.ask {
            QteAsk::Jump => input.jump = false,
            QteAsk::Hold { .. } => input.attack = false,
            _ => input.attack = false,
        }
        if q.kind == QteKind::Mount && q.step == 3 && seal {
            self.nape_fx -= real;
            if self.nape_fx <= 0.0 {
                self.nape_fx = 0.09;
                let nape = self.nape();
                ev.push(Event::Burst(nape, [0.75, 0.55, 1.0], 6, 3.0));
            }
            if let Some(r) = self.ride.as_mut() {
                r.buck = r.buck.max(0.45);
            }
        }
        if q.kind == QteKind::Awaken {
            // She looks at him while she calls him back.
            let chest = self.varrak_chest();
            let p = &mut self.player;
            let (dx, dz) = (chest.x - p.x, chest.z - p.z);
            let yaw = (-dx).atan2(-dz);
            let k = (real * 3.0).min(1.0);
            p.yaw += wrap(yaw - p.yaw) * k;
            let pitch = (chest.y - (p.y + EYE)).atan2(dx.hypot(dz));
            p.pitch += (pitch - p.pitch) * k;
            self.pouring = seal && q.step == 0;
        }
        let late = q.window > 0.0 && q.t > q.window;
        let before = q.fill;
        let verdict = match q.ask {
            QteAsk::Press => click.then_some(true).or(late.then_some(false)),
            QteAsk::Jump => jump.then_some(true).or(late.then_some(false)),
            QteAsk::Beat { beat, slack } => {
                if click {
                    Some((q.t - beat).abs() <= slack)
                } else if q.t > beat + slack {
                    Some(false)
                } else {
                    None
                }
            }
            QteAsk::Hold { need, decay } => {
                q.fill = if seal {
                    q.fill + real / need
                } else {
                    q.fill - decay * real
                }
                .clamp(0.0, 1.0);
                (q.fill >= 1.0).then_some(true).or(late.then_some(false))
            }
            QteAsk::Mash { per, hold } => {
                let gain = (if click { per } else { 0.0 }) + (if seal { hold * real } else { 0.0 });
                q.fill = (q.fill + gain - 0.1 * real).clamp(0.0, 1.0);
                (q.fill >= 1.0).then_some(true).or(late.then_some(false))
            }
        };
        // The awakening: his memories come up one by one as the seal pours in.
        if q.kind == QteKind::Awaken && q.step == 0 {
            for (i, m) in MEMORIES.iter().enumerate() {
                let at = (i as f32 + 1.0) / (MEMORIES.len() as f32 + 1.0);
                if before < at && q.fill >= at {
                    ev.push(Event::Float(m.word, false, 1.6));
                    ev.push(Event::Sfx(Sfx::RuneChime, Some(self.varrak_chest()), 0.8));
                }
            }
        }
        match verdict {
            None => self.qte = Some(q),
            Some(good) => self.decide(q, good, ev),
        }
    }

    fn decide(&mut self, q: Qte, good: bool, ev: &mut Vec<Event>) {
        let offer = q.kind == QteKind::Mount && q.step == 0;
        if good || !offer {
            self.stamp = Some((good, self.clock));
        }
        if good {
            self.sfx(Sfx::RuneChime, None, 0.7);
        }
        match (q.kind, q.step) {
            (QteKind::Dodge, _) => {
                if good {
                    self.dodge();
                }
            }
            (QteKind::Mount, 0) => {
                if good {
                    self.mount(ev);
                }
            }
            (QteKind::Mount, 1 | 2) => {
                if good {
                    self.stab(q.step, ev);
                } else {
                    self.thrown(ev);
                }
            }
            (QteKind::Mount, _) => {
                if good {
                    self.seal_nape(ev);
                } else {
                    self.thrown(ev);
                }
            }
            (QteKind::Grab, _) => {
                if good {
                    self.break_free(ev);
                } else {
                    self.slammed(ev);
                }
            }
            (QteKind::Rope, _) => {
                if good {
                    self.cut_rope();
                } else {
                    self.rope_cd = 5.0;
                }
            }
            (QteKind::Awaken, 0) => {
                let chest = self.varrak_chest();
                self.ask(
                    QteKind::Awaken,
                    QteAsk::Press,
                    0.0,
                    0.5,
                    (1, 2),
                    chest,
                    "CLIC",
                    "«¡Varrak!»",
                    "Llámalo por su nombre",
                );
            }
            (QteKind::Awaken, _) => {
                self.varrak.held = false;
                self.varrak.set(VState::Awake);
                self.flash = 1.0;
                ev.push(Event::Awake);
            }
        }
    }

    /// The moments the quick events wait for.
    fn offer_qte(&mut self) {
        if self.qte.is_some() || self.phase != Phase::Duel {
            return;
        }
        let (px, pz) = (self.player.x, self.player.z);
        let v = &self.varrak;
        let (fx, fz) = (v.yaw.sin(), v.yaw.cos());
        let (rx, rz) = (px - v.x, pz - v.z);
        let d = rx.hypot(rz);
        // The spark is full: he kneels, and she calls him back.
        if v.state == VState::Kneel && !v.held {
            self.varrak.held = true;
            let chest = self.varrak_chest();
            self.ask(
                QteKind::Awaken,
                QteAsk::Hold {
                    need: 2.6,
                    decay: 0.15,
                },
                0.0,
                0.5,
                (0, 2),
                chest,
                "CLIC DCHO.",
                "Devuélvele su nombre",
                "Mantén el sello",
            );
            self.teach(QteKind::Awaken, "No sueltes. Devuélveselo todo.");
            return;
        }
        // On all fours, about to run her down.
        if v.state == VState::Gallop
            && v.gallop.locked
            && !v.gallop.hit_her
            && self.dodge_cd <= 0.0
            && self.player.y < 0.5
        {
            let along = rx * fx + rz * fz;
            let side = (fx * rz - fz * rx).abs();
            if along > 0.8 && along < GALLOP_SPEED * 0.5 && side < 1.8 {
                self.dodge_cd = 9.0;
                let head = v.root() + Vec3::new(fx * 1.2, 1.5, fz * 1.2);
                self.ask(
                    QteKind::Dodge,
                    QteAsk::Jump,
                    1.0,
                    0.16,
                    (0, 1),
                    head,
                    "ESPACIO",
                    "¡Apártate!",
                    "Que se estrelle detrás de ti",
                );
                self.teach(QteKind::Dodge, "¡Ahora! ¡Espacio!");
                return;
            }
        }
        // Crashed and dazed, she close in front of him: up on his back.
        if v.state == VState::Stunned
            && v.why != Why::Guard
            && !self.mounted
            && d < 3.6
            && v.stun_left > 1.0
        {
            let f = self.player.forward();
            let facing = (f.x * -rx + f.z * -rz) / d.max(1e-3) > 0.3;
            if facing {
                self.mounted = true;
                let window = (v.stun_left - 0.5).min(4.0);
                let nape = self.nape();
                self.ask(
                    QteKind::Mount,
                    QteAsk::Press,
                    window,
                    1.0,
                    (0, 4),
                    nape,
                    "CLIC",
                    "¡Súbete a su lomo!",
                    "o mantén el sello desde aquí",
                );
                self.teach(
                    QteKind::Mount,
                    "¡Está en el suelo! Clic y súbete a su lomo: el sello en la nuca prende de golpe.",
                );
                return;
            }
        }
        // Under the goblins' net while she stands by its post.
        if !self.rope_cut && self.rope_cd <= 0.0 && v.fighting() && !v.open() && !v.held {
            if let Some((rope, net)) = self.rope_and_net() {
                let (r, n) = (&plan().breakables[rope], &plan().breakables[net]);
                if (v.x - n.x).hypot(v.z - n.z) < 2.9 && (px - r.x).hypot(pz - r.z) < 5.5 {
                    self.rope_cd = 6.0;
                    self.ask(
                        QteKind::Rope,
                        QteAsk::Press,
                        1.3,
                        0.2,
                        (0, 1),
                        Vec3::new(r.x, 1.6, r.z),
                        "CLIC",
                        "¡Corta la cuerda!",
                        "Está debajo de la red",
                    );
                    self.teach(
                        QteKind::Rope,
                        "¡Está debajo de la red de piedras! ¡Corta la cuerda!",
                    );
                }
            }
        }
    }

    /// She leaps aside of his line; he runs on blind (into whatever is behind her).
    fn dodge(&mut self) {
        let v = &self.varrak;
        let (fx, fz) = (v.yaw.sin(), v.yaw.cos());
        let (rx, rz) = (self.player.x - v.x, self.player.z - v.z);
        let cross = fx * rz - fz * rx;
        let mut side = cross.signum();
        if cross.abs() < 0.3 {
            // Dead ahead: the side with more room.
            let (x, z) = (self.player.x, self.player.z);
            let l = self.arena.collide(x - fz * 3.0, z + fx * 3.0, 0.38);
            let r = self.arena.collide(x + fz * 3.0, z - fx * 3.0, 0.38);
            side = if (l.x - x).hypot(l.z - z) >= (r.x - x).hypot(r.z - z) {
                1.0
            } else {
                -1.0
            };
        }
        let (dx, dz) = (-fz * side, fx * side);
        let to = self
            .arena
            .collide(self.player.x + dx * 1.2, self.player.z + dz * 1.2, 0.38);
        let p = &mut self.player;
        p.x = to.x;
        p.z = to.z;
        p.push = (dx * 15.0, dz * 15.0);
        p.untouchable = 0.9;
        p.vy = 4.0;
        p.kick(0.25, 0.08);
        self.last_dodge = self.clock;
        self.sfx(Sfx::Jump, None, 0.9);
    }

    /// Up on his back: he rears and roars; the sword's turn.
    fn mount(&mut self, ev: &mut Vec<Event>) {
        let v = &mut self.varrak;
        v.set(VState::Ridden);
        v.held = true;
        v.anim.play(Clip::Roar, 0.2, 1.0, false);
        ev.push(Event::Sfx(Sfx::Roar, Some(v.root()), 1.0));
        ev.push(Event::Float("¡¿GRAAH?!", true, 1.6));
        self.player.frozen = true;
        self.player.swing = None;
        self.ride = Some(Ride {
            kind: QteKind::Mount,
            t: 0.0,
            buck: 1.0,
        });
        let nape = self.nape();
        self.ask(
            QteKind::Mount,
            QteAsk::Beat {
                beat: 1.0,
                slack: 0.22,
            },
            1.6,
            1.0,
            (1, 4),
            nape,
            "CLIC",
            "¡Clávale la espada!",
            "Cuando el anillo toque el centro",
        );
    }

    /// The sword goes into his shoulder: he bucks; the next step.
    fn stab(&mut self, step: u8, ev: &mut Vec<Event>) {
        let nape = self.nape();
        let side = if step == 1 { 0.45 } else { -0.45 };
        let (s, c) = self.varrak.yaw.sin_cos();
        let at = nape + Vec3::new(c * side, 0.1, -s * side);
        ev.push(Event::Burst(at, [1.0, 0.82, 0.55], 28, 5.5));
        ev.push(Event::Sfx(Sfx::HitBone, Some(at), 1.0));
        ev.push(Event::Sfx(Sfx::Roar, Some(self.varrak.root()), 0.8));
        ev.push(Event::Float("¡AAARGH!", true, 1.2));
        self.varrak.anim.play(Clip::Hit, 0.08, 1.1, false);
        self.varrak.flinch = 1.0;
        self.player.start_swing(if step == 1 { 0 } else { 1 });
        self.player.kick(0.35, 0.12);
        if let Some(r) = self.ride.as_mut() {
            r.buck = 1.0;
        }
        if step == 1 {
            self.ask(
                QteKind::Mount,
                QteAsk::Beat {
                    beat: 0.8,
                    slack: 0.2,
                },
                1.4,
                1.0,
                (2, 4),
                nape,
                "CLIC",
                "¡Agárrate!",
                "Se encabrita: otra vez, al compás",
            );
        } else {
            self.ask(
                QteKind::Mount,
                QteAsk::Hold {
                    need: 1.5,
                    decay: 0.35,
                },
                3.4,
                1.0,
                (3, 4),
                nape,
                "CLIC DCHO.",
                "¡El sello en la nuca!",
                "Mantenlo hasta que prenda",
            );
        }
    }

    /// The seal on his nape: a great share of the spark at once; she drops off in front of him.
    fn seal_nape(&mut self, ev: &mut Vec<Event>) {
        let nape = self.nape();
        ev.push(Event::Burst(nape, [0.75, 0.5, 1.0], 60, 7.0));
        ev.push(Event::Sfx(Sfx::SpellBurst, Some(nape), 1.0));
        ev.push(Event::Sfx(Sfx::SealOpen, Some(nape), 0.8));
        self.flash = 0.9;
        self.dismount(3.8);
        let v = &mut self.varrak;
        v.held = false;
        let next = MEMORIES.get(v.memories).map_or(100.0, |m| m.at);
        v.spark = (v.spark + 24.0).min(next);
        if v.spark >= next && v.memories < MEMORIES.len() {
            v.memories += 1;
            v.set(VState::Memory);
            v.anim.play(Clip::Memory, 0.3, 1.0, false);
            ev.push(Event::Memory(v.memories));
        } else if v.spark >= 100.0 {
            v.set(VState::Kneel);
            v.anim.play(Clip::Kneel, 0.4, 1.0, false);
            v.blade = BladeMode::Dropped;
        } else {
            v.set(VState::Stunned);
            v.stun_left = 3.0;
            v.anim.play(Clip::Stun, 0.25, 1.0, false);
        }
        self.mounted = true;
    }

    /// She comes down `ahead` m in front of him, facing him.
    fn dismount(&mut self, ahead: f32) {
        let v = &self.varrak;
        let (x, z) = (v.x + v.yaw.sin() * ahead, v.z + v.yaw.cos() * ahead);
        let to = self.arena.collide(x, z, 0.38);
        let (vx, vz) = (v.x, v.z);
        let p = &mut self.player;
        p.x = to.x;
        p.z = to.z;
        p.y = 0.0;
        p.vy = 0.0;
        p.yaw = (-(vx - p.x)).atan2(-(vz - p.z));
        p.pitch = 0.1;
        p.frozen = false;
        self.ride = None;
    }

    /// He throws her off his back and is up again with his guard back.
    fn thrown(&mut self, ev: &mut Vec<Event>) {
        self.varrak.held = false;
        self.dismount(5.5);
        let (vx, vz) = (self.varrak.x, self.varrak.z);
        self.player.untouchable = 0.0;
        self.player.hurt(14.0, vx, vz, 9.0);
        self.varrak.shake_off(true, ev);
        self.chiquita(
            "¡Agárrate mejor, que la espalda que cae es la mía!",
            0.3,
            3.0,
        );
    }

    /// He has her in his fist, up before his face.
    fn grabbed(&mut self) {
        self.player.frozen = true;
        self.player.swing = None;
        self.player.channeling = false;
        self.ride = Some(Ride {
            kind: QteKind::Grab,
            t: 0.0,
            buck: 0.6,
        });
        let face = self.face();
        self.sfx(Sfx::Roar, Some(face), 1.0);
        self.ask(
            QteKind::Grab,
            QteAsk::Mash {
                per: 0.13,
                hold: 0.55,
            },
            3.2,
            1.0,
            (0, 1),
            face,
            "CLIC",
            "¡Suéltate!",
            "Machaca clic, o mantén el sello",
        );
        self.teach(
            QteKind::Grab,
            "¡Te ha cogido! ¡Clávale la espada en la mano!",
        );
    }

    /// The sword in his hand: he lets go with a howl and staggers.
    fn break_free(&mut self, ev: &mut Vec<Event>) {
        let face = self.face();
        ev.push(Event::Burst(face - Vec3::Y * 0.8, [1.0, 0.8, 0.5], 30, 6.0));
        ev.push(Event::Sfx(Sfx::HitBone, Some(face), 1.0));
        ev.push(Event::Float("¡AAAARGH!", true, 1.6));
        self.player.start_swing(0);
        self.dismount(2.8);
        let v = &mut self.varrak;
        v.held = false;
        v.grab_ready = 14.0;
        v.guard = (v.guard - 40.0).max(0.0);
        if v.guard <= 0.0 {
            v.stun(STUN_SECONDS, Why::Guard, ev);
        } else {
            v.set(VState::Dazed);
            v.anim.play(Clip::Stun, 0.15, 1.0, false);
        }
    }

    /// He slams her down and throws her away.
    fn slammed(&mut self, ev: &mut Vec<Event>) {
        self.dismount(6.0);
        let (vx, vz) = (self.varrak.x, self.varrak.z);
        let v = &mut self.varrak;
        v.held = false;
        v.grab_ready = 12.0;
        v.after(0.8, &mut self.rng);
        ev.push(Event::Sfx(
            Sfx::Shockwave,
            Some(Vec3::new(vx, 0.2, vz)),
            1.0,
        ));
        self.player.untouchable = 0.0;
        self.player.hurt(26.0, vx, vz, 14.0);
    }

    /// The rope parts: the net of stones drops.
    fn cut_rope(&mut self) {
        let Some((rope, net)) = self.rope_and_net() else {
            return;
        };
        self.rope_cut = true;
        let (r, n) = (&plan().breakables[rope], &plan().breakables[net]);
        let (rx, rz, nx, nz) = (r.x, r.z, n.x, n.z);
        for i in [rope, net] {
            if let Some(b) = self.arena.broken.get_mut(i) {
                *b = true;
            }
        }
        self.breaks.push(Break {
            index: rope,
            from: (rx, rz),
            force: 2.0,
            lift: 1.5,
            spin: 3.0,
        });
        self.breaks.push(Break {
            index: net,
            from: (nx, nz),
            force: 1.0,
            lift: 0.0,
            spin: 1.2,
        });
        self.sfx(Sfx::BoneCrack, Some(Vec3::new(rx, 1.6, rz)), 1.0);
        self.net_falls = Some(self.time + 0.45);
    }

    /// The stones come down: on him, if he is still there.
    fn net_lands(&mut self) {
        let Some((_, net)) = self.rope_and_net() else {
            return;
        };
        let n = &plan().breakables[net];
        let (nx, nz) = (n.x, n.z);
        let mut ev = Vec::new();
        ev.push(Event::Sfx(Sfx::Rubble, Some(Vec3::new(nx, 0.5, nz)), 1.0));
        ev.push(Event::Ring(nx, nz, 6.0));
        ev.push(Event::Dust(nx, nz));
        ev.push(Event::Dust(nx + 1.0, nz - 0.8));
        let d = (self.player.x - nx).hypot(self.player.z - nz);
        self.player.kick(0.5, 0.16 * (1.0 - d / 14.0).max(0.25));
        let v = &mut self.varrak;
        if (v.x - nx).hypot(v.z - nz) < 3.4 && v.fighting() && !v.held && !v.open() {
            v.guard = 0.0;
            v.stun(CRASH_STUN_SECONDS + 2.0, Why::Net, &mut ev);
            self.last_crash = self.time;
        } else {
            ev.push(Event::Chiquita(
                "¡Se ha apartado! Las piedras para nada.",
                0.2,
                3.0,
            ));
        }
        for e in ev {
            self.answer(e);
        }
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
        if let Some(q) = &f.qte {
            match q.ask {
                QteAsk::Press => input.attack = true,
                QteAsk::Jump => input.jump = true,
                QteAsk::Beat { beat, .. } => input.attack = (q.t - beat).abs() < 0.02,
                QteAsk::Hold { .. } => input.channel = true,
                QteAsk::Mash { .. } => input.attack = (q.t * 20.0).fract() < 0.5,
            }
            return input;
        }
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

    fn step(f: &mut Fight, input: Input, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            f.update(input, 1.0 / 60.0);
        }
    }

    #[test]
    fn a_charge_at_her_slows_time_and_space_leaps_her_aside() {
        let mut f = Fight::new(2, false, 7);
        f.player.god = true;
        f.qa("dodge");
        let mut asked = false;
        for _ in 0..300 {
            f.update(Input::default(), 1.0 / 60.0);
            if f.qte.as_ref().is_some_and(|q| q.kind == QteKind::Dodge) {
                asked = true;
                break;
            }
        }
        assert!(asked, "no dodge offered");
        step(&mut f, Input::default(), 0.3);
        assert!(f.time_scale < 0.5, "time not slowed: {}", f.time_scale);
        let before = f.player.health;
        let jump = Input {
            jump: true,
            ..Input::default()
        };
        f.update(jump, 1.0 / 60.0);
        assert!(f.qte.is_none());
        f.player.god = false;
        step(&mut f, Input::default(), 1.5);
        assert!(f.player.health >= before, "the charge still hit her");
    }

    #[test]
    fn after_a_crash_she_can_ride_him_and_seal_his_nape() {
        let mut f = Fight::new(2, false, 9);
        f.qa("mount");
        f.qa_look();
        step(&mut f, Input::default(), 0.1);
        assert!(
            f.qte.as_ref().is_some_and(|q| q.kind == QteKind::Mount),
            "no ride offered"
        );
        let spark = f.varrak.spark;
        for _ in 0..600 {
            let i = tests_bot_qte(&f);
            f.update(i, 1.0 / 60.0);
            if f.qte.is_none() && f.ride.is_none() {
                break;
            }
        }
        assert!(
            f.varrak.spark >= spark + 19.0,
            "spark {} -> {}",
            spark,
            f.varrak.spark
        );
        assert!(!f.player.frozen);
    }

    #[test]
    fn a_missed_beat_throws_her_off_his_back() {
        let mut f = Fight::new(2, false, 11);
        f.qa("ride");
        let health = f.player.health;
        // a click far too soon
        f.update(
            Input {
                attack: true,
                ..Input::default()
            },
            1.0 / 60.0,
        );
        assert!(f.ride.is_none(), "still riding");
        assert!(f.player.health < health);
        assert_eq!(f.varrak.state, VState::Roar);
    }

    #[test]
    fn in_his_fist_mashing_or_holding_the_seal_frees_her_and_doing_nothing_hurts() {
        let mut f = Fight::new(2, false, 13);
        f.qa("grab");
        step(&mut f, Input::default(), 0.05);
        assert!(f.qte.as_ref().is_some_and(|q| q.kind == QteKind::Grab));
        let hold = Input {
            channel: true,
            ..Input::default()
        };
        step(&mut f, hold, 2.8);
        assert!(f.ride.is_none(), "still held");
        assert!(matches!(f.varrak.state, VState::Dazed | VState::Stunned));
        let mut g = Fight::new(2, false, 13);
        g.qa("grab");
        let health = g.player.health;
        step(&mut g, Input::default(), 3.6);
        assert!(g.ride.is_none());
        assert!(g.player.health < health - 20.0);
    }

    #[test]
    fn the_awakening_cannot_be_failed_and_ends_in_his_name() {
        let mut f = Fight::new(2, false, 17);
        f.qa("awaken");
        step(&mut f, Input::default(), 6.0);
        assert!(f
            .qte
            .as_ref()
            .is_some_and(|q| q.kind == QteKind::Awaken && q.step == 0));
        assert_eq!(f.phase, Phase::Duel);
        let hold = Input {
            channel: true,
            ..Input::default()
        };
        step(&mut f, hold, 3.0);
        assert!(f.qte.as_ref().is_some_and(|q| q.step == 1));
        f.update(
            Input {
                attack: true,
                ..Input::default()
            },
            1.0 / 60.0,
        );
        step(&mut f, Input::default(), 0.2);
        assert_eq!(f.phase, Phase::Won);
    }

    fn tests_bot_qte(f: &Fight) -> Input {
        bot(f)
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
            // The last call of the awakening is a click.
            let mut step_input = input;
            step_input.attack = f
                .qte
                .as_ref()
                .is_some_and(|q| q.kind == QteKind::Awaken && q.step == 1);
            f.update(step_input, 1.0 / 60.0);
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
    fn the_fused_swords_slow_burn_and_heal_and_a_cure_heals() {
        if !available() {
            return;
        }
        let mut f = Fight::new(4, false, 11);
        f.player.god = true;
        f.blade = dungeon::WeaponFx {
            slow: 3.0,
            burn: 4.0,
            leech: 4.0,
        };
        f.begin();
        let (mut slowed, mut burned) = (false, false);
        for _ in 0..(60 * 120) {
            aim(&mut f);
            let input = bot(&f);
            f.update(input, 1.0 / 60.0);
            slowed |= f.varrak.slowed > 0.0 || f.pack.wolves.iter().any(|w| w.slowed > 0.0);
            burned |= f.varrak.burning > 0.0 || f.pack.wolves.iter().any(|w| w.burning > 0.0);
            if slowed && burned {
                break;
            }
        }
        assert!(slowed && burned, "the sword's gifts never reached a foe");
        f.player.health = 40.0;
        f.cured_at = -99.0;
        assert!(f.can_cure());
        f.cure(45.0);
        assert_eq!(f.player.health, 85.0);
        assert!(!f.can_cure(), "a second cure waits a moment");
    }

    #[test]
    fn he_fights_hard_charges_often_and_answers_her_at_his_back() {
        if !available() {
            return;
        }
        let mut f = Fight::new(2, true, 7);
        f.player.god = true;
        f.begin();
        let mut spun = false;
        let mut charges = 0;
        let mut time_in = std::collections::BTreeMap::<String, f32>::new();
        for i in 0..(60 * 90) {
            aim(&mut f);
            let mut input = bot(&f);
            if i == 60 * 70 {
                charges = f.varrak.charges;
            }
            // The last 20 s she sneaks behind him and hacks at his back.
            if i >= 60 * 70 && f.phase == Phase::Duel && !f.varrak.open() {
                let v = &f.varrak;
                let (bx, bz) = (v.x - v.yaw.sin() * 2.4, v.z - v.yaw.cos() * 2.4);
                f.player.x = bx;
                f.player.z = bz;
                input.attack = true;
            }
            // (No rides and no waking him here: this is about how he fights.)
            if f.qte
                .as_ref()
                .is_some_and(|q| q.kind == QteKind::Mount && q.step == 0)
            {
                input.attack = false;
            }
            f.update(input, 1.0 / 60.0);
            f.varrak.spark = f.varrak.spark.min(45.0);
            spun |= f.varrak.state == VState::Spin;
            *time_in
                .entry(format!("{:?}/{:?}", f.phase, f.varrak.state))
                .or_insert(0.0) += 1.0 / 60.0;
        }
        eprintln!("[test] 70 s: {charges} charges, then spun {spun}; {time_in:.1?}");
        assert!(charges >= 4, "only {charges} charges in 70 s");
        assert!(spun, "standing at his back never made him turn on her");
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
