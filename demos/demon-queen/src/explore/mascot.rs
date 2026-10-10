//! Chiquita, out on the screen in 3D: where she stands, what she does and when she shows. She
//! springs up from the bottom-right corner when she has something to say (her words hang over her
//! head) and ducks back down a while after she has said it; in a scene or in the Queen's bed she
//! stays. What she says sets her mood (talking, jealous with her arms crossed, her back turned on
//! what she should not watch, flustered, cheering, wagging a finger), and some things set off a
//! gag: she flies at the screen and hits the glass, or she steps in front of a scene that goes to
//! black and punches the screen to pieces. Her mouth moves while her words come out, she blinks,
//! she glares through half-shut eyes when she sulks and squeezes them shut when it hurts. She can
//! be poked (she complains) and picked up with the pointer: she dangles kicking while carried,
//! swinging with how she is moved, and when let go she falls (or flies, if thrown), bounces off the
//! screen's edges and floor and stays where she lands. The overlay draws her
//! (`map_overlay::MascotView`) and swings her hair; this only directs her.
use super::*;

/// What she is asked to do: by what she says, or by what happens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Cue {
    Talk,
    Jealous,
    /// She turns her back on the scene ("me voy a mirar la pared"), peeking now and then.
    Away,
    Flustered,
    Cheer,
    /// A hand on her hip, a finger wagged at the screen.
    Scold,
    /// She flies at the screen and splats against the glass.
    Splat,
    /// She steps in front of the scene and punches the screen to pieces (until the scene goes on).
    Smash,
    /// She flies a loop in the air at the screen's right, arms up, and lands back where she was.
    Swoop,
    /// She flies in front of the hypnotist's ring like a goalkeeper, arms wide, and follows it as
    /// it swings (`mascot_cover_track`) for a while.
    Cover,
    /// Hypnotised herself: slumped and swaying, her head lolling, spirals in her eyes.
    Dazed,
    /// She leaps beside the scene's picture and punches at whoever is in it.
    Jab,
}

/// Her clips, as `map_overlay::MASCOT_CLIPS` names them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Clip {
    Idle = 0,
    Talk = 1,
    Jealous = 2,
    Away = 3,
    Flustered = 4,
    Cheer = 5,
    Splat = 6,
    Punch = 7,
    Enter = 8,
    Scold = 9,
    Held = 10,
    Cover = 11,
    Dizzy = 12,
}

impl Clip {
    /// How long it lasts (s), and whether it loops.
    fn length(self) -> (f32, bool) {
        match self {
            Clip::Cover => (1.0, true),
            Clip::Dizzy => (2.4, true),
            Clip::Idle => (4.0, true),
            Clip::Talk => (2.0, true),
            Clip::Jealous => (2.4, true),
            Clip::Away => (3.0, true),
            Clip::Flustered => (1.6, true),
            Clip::Cheer => (1.2, true),
            Clip::Splat => (0.6, false),
            Clip::Punch => (1.6, false),
            Clip::Enter => (0.9, false),
            Clip::Scold => (1.2, true),
            Clip::Held => (1.0, true),
        }
    }

    /// How far her eyelids are down in it (sulking through half-shut eyes).
    fn lids(self) -> f32 {
        match self {
            Clip::Jealous => 0.4,
            Clip::Away => 0.3,
            Clip::Scold => 0.22,
            Clip::Dizzy => 0.42,
            _ => 0.0,
        }
    }
}

/// A gag playing, and when it began.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Gag {
    Splat(f32),
    /// Smashing the screen; `Some` once the scene has moved on (when she let it go).
    Smash(f32, Option<f32>),
    /// A loop in the air, from where she stood (her feet and height then).
    Swoop(f32, [f32; 2], f32),
    /// In front of the hypnotist's ring, from where she stood (her feet and height then).
    Cover(f32, [f32; 2], f32),
    /// Beside the picture, punching at it, from where she stood.
    Jab(f32, [f32; 2], f32),
}

/// How long her loop in the air takes, and where it turns (its middle and radius, px).
const SWOOP_SECS: f32 = 2.3;
const SWOOP_AT: [f32; 2] = [1690.0, 420.0];
const SWOOP_RADIUS: f32 = 170.0;
/// How long she keeps the ring covered (the game counts the same), and how tall she is in front
/// of it.
pub(super) const COVER_SECS: f32 = 2.8;
const COVER_HEIGHT: f32 = 430.0;
/// Where she punches from, beside the picture's right edge (facing it), and how tall she is
/// there; when her fist lands (s into the gag).
const JAB_FEET: [f32; 2] = [1418.0, 760.0];
const JAB_HEIGHT: f32 = 470.0;
const JAB_YAW: f32 = -72.0;
pub(super) const JAB_HIT: f32 = 0.92;
const JAB_SECS: f32 = 2.5;
/// How long she stays hypnotised.
pub(super) const DAZED_SECS: f32 = 3.4;

/// Where she stands: her feet on the screen and how tall she is there (px).
const CORNER: ([f32; 2], f32) = ([1790.0, 1078.0], 290.0);
/// Beside the Queen's bed menu, when she is the one being teased.
const STAGE: ([f32; 2], f32) = ([1690.0, 612.0], 430.0);
/// In front of a scene's picture.
const CENTER: ([f32; 2], f32) = ([960.0, 800.0], 620.0);
/// Where she lands on the glass when she splats against it, and how big she is there.
const SPLAT_FEET: [f32; 2] = [1260.0, 960.0];
const SPLAT_HEIGHT: f32 = 860.0;
/// Turned away from the scene (a quarter turn and more to her left).
const AWAY_YAW: f32 = 112.0;
/// How long her entrance takes (her `Chq_Enter` clip).
const ENTER_SECS: f32 = 0.9;
/// The screen's floor and walls for her when she is let go (px; her feet).
const FLOOR: f32 = 1078.0;
const WALLS: [f32; 2] = [70.0, 1850.0];
/// Falling (px/s²), and the hardest she can be thrown (px/s).
const GRAVITY: f32 = 2600.0;
const THROW_MAX: f32 = 4200.0;

/// What she says when picked up, dropped, thrown, landing hard, and poked.
const GRABBED: [&str; 5] = [
    "—¡Eh! ¡Bájame ahora mismo!",
    "—¡Que no soy un llavero, Majestad!",
    "—¡Las manos quietas! …Bueno, quietas no. Pero QUIETAS.",
    "—¿Me estás paseando? ¿En serio?",
    "—¡Esto es secuestro de sello!",
];
const THROWN: [&str; 3] = [
    "—¡¿ME HAS LANZADO?!",
    "—¡AAAAAH!",
    "—¡Esto no es un juego de pelota!",
];
const LANDED: [&str; 4] = [
    "—Ya está. Contenta, ¿no?",
    "—Me quedo aquí. Que conste que lo elijo yo.",
    "—Hmph. Buen sitio. Lo iba a elegir yo de todas formas.",
    "—Si me vuelves a coger, te muerdo.",
];
const LANDED_HARD: [&str; 3] = [
    "—¡Au! …El suelo de la pantalla está duro.",
    "—Veo estrellitas. Y una reina malvada.",
    "—Nada roto. Mi orgullo, quizá. Te odio.",
];
const POKED: [(&str, Cue); 6] = [
    ("—¡Oye! ¿Qué toqueteas?", Cue::Flustered),
    ("—Eso era mi mejilla. Mía.", Cue::Scold),
    ("—Un toque más y te muerdo el dedo.", Cue::Jealous),
    (
        "—¿Eso era un toque o una caricia? Decídete.",
        Cue::Flustered,
    ),
    ("—Sí, sigo aquí. No, no me voy a ir.", Cue::Talk),
    ("—¡Cosquillas no! ¡Que te conozco!", Cue::Flustered),
];
const POKED_ENOUGH: &str = "—¡BASTA! ¡Me vas a gastar el sello de tanto tocarlo!";

/// Being carried by the pointer: where the pointer is and where it took her, how far her feet are
/// from it, since when, whether she has been moved yet, and how fast she is going.
#[derive(Clone, Copy, Debug)]
struct Held {
    cursor: [f32; 2],
    grab: [f32; 2],
    offset: [f32; 2],
    since: f32,
    moved: bool,
    vel: [f32; 2],
}

/// Let go: falling or flying, how fast, and the hardest she has hit something on the way.
#[derive(Clone, Copy, Debug)]
struct Fly {
    vel: [f32; 2],
    hardest: f32,
}

/// The glass she broke.
#[derive(Clone, Copy, Debug)]
struct Glass {
    center: [f32; 2],
    reach: f32,
    seed: u32,
    at: f32,
    fall_at: Option<f32>,
    fade_at: Option<f32>,
}

/// A pose held still for the QA captures: a clip at a time, where she stands, her face.
#[derive(Clone, Copy, Debug)]
pub(super) struct QaPose {
    pub(super) clip: u8,
    pub(super) time: f32,
    pub(super) feet: [f32; 2],
    pub(super) height: f32,
    pub(super) yaw: f32,
    /// Her eyes shut and her mouth open (0..=1).
    pub(super) face: (f32, f32),
}

pub(super) struct Mascot {
    /// 0 down out of sight, 1 all the way up; whether she is on her way up.
    out: f32,
    rising: bool,
    /// When she last sprang up into view.
    entered: f32,
    mood: Cue,
    mood_until: f32,
    clip: Clip,
    clip_at: f32,
    prev: Clip,
    prev_at: f32,
    switched: f32,
    feet: [f32; 2],
    height: f32,
    /// Her turn (degrees) and how fast it is going: a spring, so she whips round and settles.
    yaw: f32,
    yaw_vel: f32,
    gag: Option<Gag>,
    glass: Option<Glass>,
    last_word: f32,
    /// While her words come out (her mouth moves).
    talk_from: f32,
    talk_until: f32,
    /// Her blinks: when the current ones began, and when the next comes.
    blinks: [f32; 2],
    next_blink: f32,
    seed: u32,
    qa: Option<QaPose>,
    /// Picked up by the pointer, or let go and falling.
    held: Option<Held>,
    fly: Option<Fly>,
    /// Where the player put her (her feet), and for which of her places (0 the corner, 1 beside
    /// the bed's menu): she stays there until her place changes.
    placed: Option<([f32; 2], u8)>,
    /// Her swing while carried or flying (degrees) and how fast it goes.
    swing: f32,
    swing_vel: f32,
    /// When she last touched down, and was last poked (a squash each).
    landed: f32,
    poked: f32,
    /// Recent pokes, and which of her lines comes next.
    pokes: Vec<f32>,
    line: usize,
    /// Where the hypnotist's ring is on the screen, for her to cover it (px).
    cover_at: [f32; 2],
}

impl Default for Mascot {
    fn default() -> Self {
        Self {
            out: 0.0,
            rising: false,
            entered: -10.0,
            mood: Cue::Talk,
            mood_until: 0.0,
            clip: Clip::Idle,
            clip_at: 0.0,
            prev: Clip::Idle,
            prev_at: 0.0,
            switched: -10.0,
            feet: CORNER.0,
            height: CORNER.1,
            yaw: 0.0,
            yaw_vel: 0.0,
            gag: None,
            glass: None,
            last_word: -100.0,
            talk_from: -100.0,
            talk_until: -100.0,
            blinks: [-10.0, -10.0],
            next_blink: 1.5,
            seed: 7,
            qa: None,
            held: None,
            fly: None,
            placed: None,
            swing: 0.0,
            swing_vel: 0.0,
            landed: -10.0,
            poked: -10.0,
            pokes: Vec::new(),
            line: 0,
            cover_at: [960.0, 300.0],
        }
    }
}

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Out past the end and back (a pop): 0 → 1, overshooting a little.
fn back_out(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0) - 1.0;
    let c = 1.25;
    1.0 + (c + 1.0) * x * x * x + c * x * x
}

/// Back a little before going (anticipation): 0 → 1, dipping under 0 first.
fn back_in(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let c = 1.25;
    (c + 1.0) * x * x * x - c * x * x
}

fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// A small random number stream (0..1).
fn roll(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed % 10_000) as f32 / 10_000.0
}

/// Her mouth while words come out, `t` seconds in: syllables about nine a second, each its own
/// size, now and then a pause between words.
fn flap(t: f32) -> f32 {
    let beat = 0.115;
    let n = (t / beat).floor();
    let f = t / beat - n;
    let h = ((n * 12.9898 + 4.1).sin() * 43_758.547).fract().abs();
    let size = if h < 0.16 { 0.04 } else { 0.32 + 0.68 * h };
    size * (f * std::f32::consts::PI).sin().max(0.0).powf(0.8)
}

impl App {
    /// Something for Chiquita to do.
    pub(super) fn mascot_cue(&mut self, cue: Cue) {
        let now = self.elapsed;
        let in_scene = self.bed_scene_open();
        let m = &mut self.mascot;
        m.last_word = now;
        match cue {
            Cue::Splat => {
                if m.gag.is_none() {
                    m.gag = Some(Gag::Splat(now));
                    m.seed = m.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                }
            }
            Cue::Smash => {
                if !matches!(m.gag, Some(Gag::Smash(..))) {
                    m.gag = Some(Gag::Smash(now, None));
                    m.seed = m.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                }
            }
            Cue::Swoop => {
                if m.gag.is_none() && m.held.is_none() && m.fly.is_none() {
                    m.gag = Some(Gag::Swoop(now, m.feet, m.height));
                }
            }
            Cue::Cover => {
                if m.held.is_none() && m.fly.is_none() && !matches!(m.gag, Some(Gag::Smash(..))) {
                    m.gag = Some(Gag::Cover(now, m.feet, m.height));
                    m.mood = Cue::Scold;
                    m.mood_until = now + COVER_SECS + 0.6;
                }
            }
            Cue::Jab => {
                if m.held.is_none() && m.fly.is_none() && !matches!(m.gag, Some(Gag::Smash(..))) {
                    m.gag = Some(Gag::Jab(now, m.feet, m.height));
                }
            }
            Cue::Dazed => {
                m.mood = Cue::Dazed;
                m.mood_until = now + DAZED_SECS;
            }
            Cue::Away => {
                m.mood = Cue::Away;
                m.mood_until = if in_scene { f32::INFINITY } else { now + 6.0 };
            }
            Cue::Talk => {
                // Talking does not end a sulk she is in.
                if m.mood == Cue::Talk || now >= m.mood_until {
                    m.mood = Cue::Talk;
                    m.mood_until = now + 2.6;
                }
            }
            other => {
                m.mood = other;
                m.mood_until = now + 4.5;
            }
        }
    }

    /// Chiquita says `text` (from `delay` seconds on), doing `cue` as she says it.
    pub(super) fn mascot_say(&mut self, cue: Cue, text: &str, delay: f32) {
        self.mascot_cue(cue);
        let now = self.elapsed;
        let m = &mut self.mascot;
        let words = (text.chars().count() as f32 * 0.048).clamp(0.5, 3.4);
        m.talk_from = now + delay;
        m.talk_until = now + delay + words;
        m.last_word = m.last_word.max(now + delay);
    }

    /// Holds her in a pose for the QA captures (`None` lets her go).
    pub(super) fn set_mascot_qa(&mut self, pose: Option<QaPose>) {
        self.mascot.qa = pose;
    }

    /// Where the hypnotist's ring is now (screen px), for her to keep in front of it.
    pub(super) fn mascot_cover_track(&mut self, at: [f32; 2]) {
        self.mascot.cover_at = at;
    }

    /// She is hypnotised right now.
    pub(super) fn mascot_dazed(&self) -> bool {
        self.mascot.mood == Cue::Dazed && self.elapsed < self.mascot.mood_until
    }

    /// Whether the pointer at (x, y) (the 1920×1080 screen) is on her: her big head, or her body
    /// under it.
    pub(super) fn mascot_hit(&self, x: f32, y: f32) -> bool {
        let m = &self.mascot;
        if m.gag.is_some()
            || m.qa.is_some()
            || self.map_open
            || (m.out < 0.6 && m.held.is_none() && m.fly.is_none())
        {
            return false;
        }
        let Some(view) = self.mascot_view() else {
            return false;
        };
        if !view.shown() {
            return false;
        }
        let tall = view.height as f32 * view.squash[1] as f32 / 1000.0;
        let up = view.feet[1] as f32 - y;
        if !(0.0..=tall).contains(&up) {
            return false;
        }
        let half = if up > tall * 0.55 {
            tall * 0.27
        } else {
            tall * 0.17
        };
        (x - view.feet[0] as f32).abs() <= half
    }

    /// The pointer went down at (x, y): if it is on her, she is picked up (and the click is hers).
    pub(super) fn mascot_press(&mut self, x: f32, y: f32) -> bool {
        if !self.mascot_hit(x, y) {
            return false;
        }
        let now = self.elapsed;
        let m = &mut self.mascot;
        m.fly = None;
        m.held = Some(Held {
            cursor: [x, y],
            grab: [x, y],
            offset: [m.feet[0] - x, m.feet[1] - y],
            since: now,
            moved: false,
            vel: [0.0, 0.0],
        });
        true
    }

    /// The pointer moved while she is held.
    pub(super) fn mascot_drag_to(&mut self, x: f32, y: f32) {
        if let Some(h) = self.mascot.held.as_mut() {
            h.cursor = [x, y];
        }
    }

    /// She is being carried.
    pub(super) fn mascot_held(&self) -> bool {
        self.mascot.held.is_some()
    }

    /// The pointer let go: a quick click is a poke; otherwise she drops (or flies, if thrown).
    pub(super) fn mascot_release(&mut self) {
        let Some(h) = self.mascot.held.take() else {
            return;
        };
        let now = self.elapsed;
        if !h.moved && now - h.since < 0.4 {
            self.mascot_poked();
            return;
        }
        let speed = (h.vel[0] * h.vel[0] + h.vel[1] * h.vel[1]).sqrt();
        let k = if speed > THROW_MAX {
            THROW_MAX / speed
        } else {
            1.0
        };
        self.mascot.fly = Some(Fly {
            vel: [h.vel[0] * k, h.vel[1] * k],
            hardest: 0.0,
        });
        if speed > 1800.0 {
            let line = THROWN[self.mascot.line % THROWN.len()];
            self.mascot.line += 1;
            self.float_chiquita_cue(line, Cue::Splat);
            // (Not the splat gag: only its balloon and her face.)
            self.mascot.gag = None;
        }
    }

    /// A poke: a squish, and a line (too many and she has had enough).
    fn mascot_poked(&mut self) {
        let now = self.elapsed;
        let m = &mut self.mascot;
        m.poked = now;
        m.pokes.retain(|t| now - *t < 4.0);
        m.pokes.push(now);
        if m.pokes.len() >= 5 {
            m.pokes.clear();
            self.float_chiquita_cue(POKED_ENOUGH, Cue::Jealous);
            return;
        }
        let (line, cue) = POKED[m.line % POKED.len()];
        m.line += 1;
        self.float_chiquita_cue(line, cue);
    }

    /// Carried, or let go and falling: the pointer drags her (a little behind it), she swings with
    /// how she is moved; let go she falls, bounces off the walls, the top and the floor, and stays
    /// where she lands. Returns a line to say, if any.
    fn mascot_physics(&mut self, dt: f32, ctx: u8) -> Option<(&'static str, Cue)> {
        let now = self.elapsed;
        let m = &mut self.mascot;
        let mut say = None;
        let mut target_swing = 0.0;
        if let Some(h) = m.held.as_mut() {
            let target = [h.cursor[0] + h.offset[0], h.cursor[1] + h.offset[1]];
            let prev = m.feet;
            let k = 1.0 - (-dt * 28.0).exp();
            m.feet = [
                m.feet[0] + (target[0] - m.feet[0]) * k,
                m.feet[1] + (target[1] - m.feet[1]) * k,
            ];
            let vel = [
                (m.feet[0] - prev[0]) / dt.max(1e-4),
                (m.feet[1] - prev[1]) / dt.max(1e-4),
            ];
            h.vel = [
                h.vel[0] * 0.55 + vel[0] * 0.45,
                h.vel[1] * 0.55 + vel[1] * 0.45,
            ];
            let d = ((h.cursor[0] - h.grab[0]).powi(2) + (h.cursor[1] - h.grab[1]).powi(2)).sqrt();
            if !h.moved && (d > 10.0 || now - h.since > 0.4) {
                h.moved = true;
                say = Some((GRABBED[m.line % GRABBED.len()], Cue::Talk));
                m.line += 1;
            }
            target_swing = (-h.vel[0] * 0.018).clamp(-30.0, 30.0);
            m.out = 1.0;
            m.rising = true;
        } else if let Some(f) = m.fly.as_mut() {
            f.vel[1] += GRAVITY * dt;
            m.feet[0] += f.vel[0] * dt;
            m.feet[1] += f.vel[1] * dt;
            let tall = m.height;
            if m.feet[0] < WALLS[0] || m.feet[0] > WALLS[1] {
                m.feet[0] = m.feet[0].clamp(WALLS[0], WALLS[1]);
                f.hardest = f.hardest.max(f.vel[0].abs());
                f.vel[0] = -f.vel[0] * 0.5;
                m.landed = now;
            }
            if m.feet[1] - tall * 0.99 < 0.0 && f.vel[1] < 0.0 {
                m.feet[1] = tall * 0.99;
                f.hardest = f.hardest.max(-f.vel[1]);
                f.vel[1] = -f.vel[1] * 0.35;
            }
            if m.feet[1] >= FLOOR {
                m.feet[1] = FLOOR;
                let hit = f.vel[1];
                f.hardest = f.hardest.max(hit);
                m.landed = now;
                if hit > 700.0 {
                    f.vel[1] = -hit * 0.36;
                    f.vel[0] *= 0.7;
                } else {
                    let hard = f.hardest > 2100.0;
                    m.fly = None;
                    m.placed = Some((m.feet, ctx));
                    say = Some(if hard {
                        (LANDED_HARD[m.line % LANDED_HARD.len()], Cue::Flustered)
                    } else {
                        (LANDED[m.line % LANDED.len()], Cue::Scold)
                    });
                    m.line += 1;
                }
            }
            if let Some(f) = m.fly.as_ref() {
                target_swing = (-f.vel[0] * 0.012).clamp(-35.0, 35.0);
            }
            m.out = 1.0;
        }
        // The swing is a spring, so she sways on after a stop.
        let stiff = 90.0f32;
        let damp = 2.0 * 0.3 * stiff.sqrt();
        m.swing_vel += ((target_swing - m.swing) * stiff - m.swing_vel * damp) * dt;
        m.swing += m.swing_vel * dt;
        say
    }

    /// Each step: she comes up or goes down, her mood wears off, a gag plays on, she blinks.
    pub(super) fn tick_mascot(&mut self, dt: f32) {
        let now = self.elapsed;
        let in_scene = self.bed_scene_open();
        let black = self.scene_black();
        let teased = self
            .alcove
            .as_ref()
            .is_some_and(|a| a.page == alcove::Page::Chiquita);
        let in_bed = self.alcove.is_some();
        let map = self.map_open;
        let ctx = u8::from(teased);
        if let Some((line, cue)) = self.mascot_physics(dt, ctx) {
            self.float_chiquita_cue(line, cue);
        }
        let m = &mut self.mascot;
        let carried = m.held.is_some_and(|h| h.moved) || m.fly.is_some();
        // A gag's turns.
        match m.gag {
            Some(Gag::Smash(start, None)) if !black && now - start > 1.2 => {
                m.gag = Some(Gag::Smash(start, Some(now)));
                if let Some(g) = m.glass.as_mut() {
                    g.fall_at = Some(now);
                }
            }
            Some(Gag::Smash(_, Some(released))) if now - released > 0.7 => m.gag = None,
            Some(Gag::Splat(start)) if now - start > 3.0 => {
                m.gag = None;
                m.out = 0.0;
                m.rising = false;
            }
            Some(Gag::Swoop(start, _, _)) if now - start > SWOOP_SECS => m.gag = None,
            Some(Gag::Cover(start, _, _)) if now - start > COVER_SECS + 0.45 => m.gag = None,
            Some(Gag::Jab(start, _, _)) if now - start > JAB_SECS => m.gag = None,
            _ => {}
        }
        // The glass cracks when she hits it, and goes when it is done with.
        let hit = match m.gag {
            Some(Gag::Splat(start)) => Some((start + 0.32, 0)),
            Some(Gag::Smash(start, _)) => Some((start + 0.9, 1)),
            Some(Gag::Swoop(..) | Gag::Cover(..) | Gag::Jab(..)) | None => None,
        };
        if let Some((at, kind)) = hit {
            if now >= at && m.glass.map_or(true, |g| g.at < at - 0.05) {
                let (center, reach) = if kind == 0 {
                    let feet = SPLAT_FEET;
                    ([feet[0] - 10.0, feet[1] - SPLAT_HEIGHT * 0.66], 380.0)
                } else {
                    ([CENTER.0[0] - 30.0, CENTER.0[1] - CENTER.1 * 0.55], 560.0)
                };
                m.glass = Some(Glass {
                    center,
                    reach,
                    seed: m.seed,
                    at,
                    fall_at: None,
                    fade_at: None,
                });
            }
        }
        if let Some(g) = m.glass.as_mut() {
            if g.fade_at.is_none() && g.fall_at.is_none() && m.gag.is_none() {
                g.fade_at = Some(now);
            }
            let gone = g.fade_at.is_some_and(|f| now - f > 1.2)
                || g.fall_at.is_some_and(|f| now - f > 1.3);
            if gone {
                m.glass = None;
            }
        }
        if now >= m.mood_until {
            m.mood = Cue::Talk;
        }
        if !in_scene && m.mood == Cue::Away && m.mood_until.is_infinite() {
            m.mood_until = now;
        }
        // Up or down: she springs up into view and ducks back down.
        let wanted = !map
            && (in_scene
                || in_bed
                || m.gag.is_some()
                || now - m.last_word < 7.0
                || m.held.is_some()
                || m.fly.is_some());
        if wanted && !m.rising {
            m.rising = true;
            if m.out <= 0.0 {
                m.entered = now;
            }
        }
        if !wanted {
            m.rising = false;
        }
        let rate = if wanted { dt / 0.3 } else { -dt / 0.38 };
        if !matches!(m.gag, Some(Gag::Splat(_))) {
            m.out = (m.out + rate).clamp(0.0, 1.0);
        }
        // Where she stands (where she was put, while her place is the same), eased there.
        let (mut spot, tall) = if teased { STAGE } else { CORNER };
        match m.placed {
            Some((at, c)) if c == ctx => spot = at,
            Some(_) => m.placed = None,
            None => {}
        }
        let k = (dt * 6.0).min(1.0);
        if m.held.is_some() || m.fly.is_some() {
            m.height += (tall - m.height) * k * 0.3;
        } else if m.gag.is_none() {
            m.feet = lerp2(m.feet, spot, k);
            m.height += (tall - m.height) * k;
            // Her turn is a spring: she whips round, overshoots a little and settles.
            let target = if m.mood == Cue::Away && now < m.mood_until {
                AWAY_YAW
            } else {
                0.0
            };
            let stiff = 150.0f32;
            let damp = 2.0 * 0.58 * stiff.sqrt();
            let steps = ((dt / (1.0 / 120.0)).ceil() as usize).clamp(1, 8);
            let h = dt / steps as f32;
            for _ in 0..steps {
                m.yaw_vel += ((target - m.yaw) * stiff - m.yaw_vel * damp) * h;
                m.yaw += m.yaw_vel * h;
            }
        } else {
            m.yaw_vel = 0.0;
        }
        // Blinks: every few seconds, now and then twice.
        if now >= m.next_blink {
            let twice = roll(&mut m.seed) < 0.22;
            m.blinks = [now, if twice { now + 0.26 } else { -10.0 }];
            m.next_blink = now + 2.0 + 3.4 * roll(&mut m.seed);
        }
        // Her clip.
        let talking = now >= m.talk_from && now < m.talk_until + 0.3;
        let entering = now - m.entered < ENTER_SECS && m.rising;
        let wanted_clip = match m.gag {
            Some(Gag::Splat(_)) => Clip::Splat,
            Some(Gag::Smash(start, released)) => {
                let t = now - start;
                if released.is_some() {
                    Clip::Idle
                } else if t < 0.45 {
                    // Leaping over to the middle.
                    Clip::Enter
                } else if t < 2.05 {
                    Clip::Punch
                } else if talking {
                    Clip::Scold
                } else {
                    Clip::Jealous
                }
            }
            Some(Gag::Swoop(..)) => Clip::Cheer,
            Some(Gag::Cover(start, _, _)) => {
                if now - start < COVER_SECS {
                    Clip::Cover
                } else {
                    Clip::Idle
                }
            }
            Some(Gag::Jab(start, _, _)) => {
                let t = now - start;
                if t < 0.45 {
                    Clip::Enter
                } else if t < 2.05 {
                    Clip::Punch
                } else {
                    Clip::Cheer
                }
            }
            None if carried => Clip::Held,
            None if entering => Clip::Enter,
            None => match m.mood {
                Cue::Jealous => Clip::Jealous,
                Cue::Away => Clip::Away,
                Cue::Flustered => Clip::Flustered,
                Cue::Cheer => Clip::Cheer,
                Cue::Scold => Clip::Scold,
                Cue::Dazed => Clip::Dizzy,
                _ if talking => Clip::Talk,
                _ => Clip::Idle,
            },
        };
        if wanted_clip != m.clip {
            m.prev = m.clip;
            m.prev_at = m.clip_at;
            m.clip = wanted_clip;
            m.clip_at = now;
            m.switched = now;
        }
    }

    /// Her eyelids now (0 open, 1 shut).
    fn mascot_blink(&self) -> f32 {
        let m = &self.mascot;
        let now = self.elapsed;
        // Squeezed shut: hitting the glass, the punch hurting.
        match m.gag {
            Some(Gag::Splat(start)) if (0.3..0.78).contains(&(now - start)) => return 1.0,
            Some(Gag::Smash(start, None)) => {
                let t = now - start;
                if (0.86..0.97).contains(&t) || (1.0..1.5).contains(&t) {
                    return 1.0;
                }
            }
            _ => {}
        }
        let mut shut = 0.0f32;
        for at in m.blinks {
            let t = now - at;
            let b = if !(0.0..0.2).contains(&t) {
                0.0
            } else if t < 0.06 {
                t / 0.06
            } else if t < 0.1 {
                1.0
            } else {
                1.0 - (t - 0.1) / 0.1
            };
            shut = shut.max(b);
        }
        // Sulking, half-shut: from the clip she is in, crossing over from the one before.
        let k = ((now - m.switched) / 0.22).clamp(0.0, 1.0);
        let lids = m.clip.lids() * k + m.prev.lids() * (1.0 - k);
        shut.max(lids)
    }

    /// Her mouth now (0 shut, 1 wide open).
    fn mascot_mouth(&self) -> f32 {
        let m = &self.mascot;
        let now = self.elapsed;
        match m.gag {
            Some(Gag::Splat(start)) => {
                let t = now - start;
                return if t < 0.3 {
                    0.35
                } else {
                    0.95 - 0.4 * ease((t - 0.32) / 0.8)
                };
            }
            Some(Gag::Smash(start, None)) => {
                let t = now - start;
                if t < 0.45 {
                    return 0.25;
                }
                if t < 1.0 {
                    // Teeth set through the wind-up, a shout as she hits.
                    return 0.92 * (-((t - 0.88) / 0.07).powi(2)).exp();
                }
                if t < 1.5 {
                    // «¡Au!»
                    return 0.6;
                }
            }
            _ => {}
        }
        if now >= m.talk_from && now < m.talk_until {
            let quiet = if m.clip == Clip::Jealous { 0.6 } else { 1.0 };
            return flap(now - m.talk_from) * quiet;
        }
        match m.clip {
            Clip::Cheer => 0.5 + 0.2 * (now * 8.0).sin().abs(),
            Clip::Flustered => 0.16 + 0.1 * (now * 11.0).sin(),
            // «¡No mires!» over and over.
            Clip::Cover => 0.45 + 0.4 * (now * 9.0).sin().abs(),
            // Slack-jawed.
            Clip::Dizzy => 0.2 + 0.06 * (now * 2.6).sin(),
            Clip::Enter if now - m.entered < 0.6 => 0.45,
            _ => 0.0,
        }
    }

    /// For the overlay (`None` when she is down out of sight).
    pub(super) fn mascot_view(&self) -> Option<map_overlay::MascotView> {
        let m = &self.mascot;
        let now = self.elapsed;
        let clock_ms = (now.max(0.0) * 1000.0) as u32;
        if let Some(q) = m.qa {
            let time_ms = (q.time.max(0.0) * 1000.0) as u32;
            return Some(map_overlay::MascotView {
                clip: q.clip,
                time_ms,
                prev_clip: q.clip,
                prev_time_ms: time_ms,
                blend: 1000,
                feet: [q.feet[0].round() as i32, q.feet[1].round() as i32],
                height: q.height.round().max(1.0) as u16,
                yaw: (q.yaw * 10.0) as i16,
                pitch: 0,
                roll: 0,
                squash: [1000; 3],
                alpha: 1000,
                crack: None,
                clock_ms,
                blink: (q.face.0.clamp(0.0, 1.0) * 1000.0) as u16,
                mouth: (q.face.1.clamp(0.0, 1.0) * 1000.0) as u16,
                dazed: if q.clip == Clip::Dizzy as u8 { 1000 } else { 0 },
            });
        }
        let mut feet = m.feet;
        let mut height = m.height;
        let mut yaw = m.yaw;
        let (mut pitch, mut roll) = (0.0f32, 0.0f32);
        let mut squash = [1.0f32, 1.0, 1.0];
        let mut alpha = 1.0f32;
        match m.gag {
            Some(Gag::Splat(start)) => {
                let t = now - start;
                let leap = ease(t / 0.32);
                feet = lerp2(CORNER.0, SPLAT_FEET, leap);
                height = CORNER.1 + (SPLAT_HEIGHT - CORNER.1) * leap * leap;
                yaw = m.yaw * (1.0 - leap);
                // Stretched as she flies at the glass.
                let fly = (leap * std::f32::consts::PI).sin();
                squash = [1.0 - 0.05 * fly, 1.0 + 0.1 * fly, 1.0 - 0.05 * fly];
                if t >= 0.32 {
                    // Pressed flat against the glass (a jolt as she hits), wobbling, then
                    // sliding off it.
                    let stuck = t - 0.32;
                    let jolt = (-stuck * 14.0).exp();
                    squash = [1.06 + 0.06 * jolt, 1.04 - 0.08 * jolt, 0.5];
                    roll = (stuck * 9.0).sin() * 1.6 * (-stuck * 1.5).exp();
                    let slide = ease((t - 1.6) / 0.9);
                    feet[1] += slide * 420.0 + ease((t - 2.4) / 0.5) * 900.0;
                    roll += slide * 9.0;
                    pitch = -6.0 * slide;
                    alpha = 1.0 - ease((t - 2.55) / 0.35);
                }
            }
            Some(Gag::Swoop(start, home, tall)) => {
                // Up from where she stood to the foot of the loop, once round it, and back down;
                // leaning into the turn, a little smaller up in the air.
                let u = ((now - start) / SWOOP_SECS).clamp(0.0, 1.0);
                let bottom = [SWOOP_AT[0], SWOOP_AT[1] + SWOOP_RADIUS];
                let (at, lean) = if u < 0.25 {
                    let k = ease(u / 0.25);
                    let mut p = lerp2(home, bottom, k);
                    p[0] += (k * std::f32::consts::PI).sin() * 60.0;
                    (p, -18.0 * (k * std::f32::consts::PI).sin())
                } else if u < 0.8 {
                    let k = (u - 0.25) / 0.55;
                    let a = std::f32::consts::FRAC_PI_2 + ease(k) * std::f32::consts::TAU;
                    (
                        [
                            SWOOP_AT[0] + a.cos() * SWOOP_RADIUS,
                            SWOOP_AT[1] + a.sin() * SWOOP_RADIUS,
                        ],
                        -(ease(k) * 360.0),
                    )
                } else {
                    let k = ease((u - 0.8) / 0.2);
                    (
                        lerp2(bottom, home, k),
                        12.0 * (1.0 - k) * (k * std::f32::consts::PI).sin(),
                    )
                };
                feet = at;
                let air = (u * std::f32::consts::PI).sin();
                height = tall * (1.0 - 0.14 * air);
                roll = lean.rem_euclid(360.0);
                if roll > 180.0 {
                    roll -= 360.0;
                }
                let stretch = 0.06 * air;
                squash = [1.0 - stretch * 0.5, 1.0 + stretch, 1.0 - stretch * 0.5];
            }
            Some(Gag::Cover(start, home, tall)) => {
                // Up in front of the ring (her chest on it), following it as it swings, bobbing
                // as she treads the air; then back down where she stood.
                let t = now - start;
                let spot = [m.cover_at[0], m.cover_at[1] + COVER_HEIGHT * 0.52];
                let go = back_out((t / 0.35).clamp(0.0, 1.0));
                let back = ease((t - COVER_SECS) / 0.45);
                let bob = (t * 7.0).sin() * 10.0;
                let there = [spot[0], spot[1] + bob];
                feet = lerp2(lerp2(home, there, go), home, back);
                height = tall + (COVER_HEIGHT - tall) * go * (1.0 - back);
                yaw = m.yaw * (1.0 - go);
                // Rocking side to side as she keeps up with the swing.
                roll = (t * 3.2).sin() * 6.0 * (1.0 - back);
                let stretch = 0.07 * (1.0 - go) + 0.2 * back * (1.0 - back);
                squash = [1.0 - stretch * 0.5, 1.0 + stretch, 1.0 - stretch * 0.5];
            }
            Some(Gag::Jab(start, home, tall)) => {
                // A hop over to the picture's side, turned to it; the punch lunges at it; back.
                let t = now - start;
                let go = ease(t / 0.45);
                let back = ease((t - (JAB_SECS - 0.45)) / 0.45);
                feet = lerp2(lerp2(home, JAB_FEET, go), home, back);
                feet[1] -= (go * std::f32::consts::PI).sin() * 110.0
                    + (back * std::f32::consts::PI).sin() * 90.0;
                height = tall + (JAB_HEIGHT - tall) * go * (1.0 - back);
                yaw = m.yaw + (JAB_YAW - m.yaw) * go * (1.0 - back);
                let lunge = (-((t - JAB_HIT) / 0.09).powi(2)).exp();
                feet[0] -= 46.0 * lunge;
                let land = (-(((t - 0.45) / 0.07).powi(2))).exp();
                squash = [1.0 + 0.06 * land, 1.0 - 0.1 * land, 1.0 + 0.06 * land];
            }
            Some(Gag::Smash(start, released)) => {
                let t = now - start;
                let go = ease(t / 0.45);
                let from = (m.feet, m.height);
                feet = lerp2(from.0, CENTER.0, go);
                height = from.1 + (CENTER.1 - from.1) * go;
                yaw = m.yaw * (1.0 - go);
                // A hop up and over on the way there, landing in a squash.
                feet[1] -= (go * std::f32::consts::PI).sin() * 120.0;
                let land = (-(((t - 0.45) / 0.07).powi(2))).exp();
                squash = [1.0 + 0.06 * land, 1.0 - 0.1 * land, 1.0 + 0.06 * land];
                // The punch: she lunges at the glass and rocks back.
                let lunge = (-((t - 0.92) / 0.09).powi(2)).exp();
                height *= 1.0 + 0.16 * lunge;
                feet[1] += 40.0 * lunge;
                if let Some(r) = released {
                    // Back to her corner.
                    let back = ease((now - r) / 0.6);
                    feet = lerp2(CENTER.0, CORNER.0, back);
                    feet[1] -= (back * std::f32::consts::PI).sin() * 140.0;
                    height = CENTER.1 + (CORNER.1 - CENTER.1) * back;
                }
            }
            None => {}
        }
        // Carried or flying she swings (and sways on a moment after); touching down or poked, a
        // squash.
        roll += m.swing;
        // Hypnotised: a slow, loose sway on top of her clip, the spirals in her eyes.
        let dazed = if m.mood == Cue::Dazed {
            let t = now - (m.mood_until - DAZED_SECS);
            ease(t / 0.3) * (1.0 - ease((t - (DAZED_SECS - 0.3)) / 0.3))
        } else {
            0.0
        };
        if dazed > 0.0 && m.gag.is_none() {
            roll += (now * 1.7).sin() * 5.0 * dazed;
            feet[1] -= (now * 1.1).sin().abs() * 14.0 * dazed;
        }
        let land = (-((now - m.landed) / 0.09).powi(2)).exp();
        let poke = (-((now - m.poked) / 0.11).powi(2)).exp();
        let squish = 0.16 * land + 0.1 * poke;
        if squish > 0.002 && m.gag.is_none() {
            squash = [1.0 + squish * 0.8, 1.0 - squish, 1.0 + squish * 0.8];
        }
        // Popping up from below the screen's edge (overshooting a little), or ducking back down
        // (rising a hair first); stretched while she moves.
        let up = if m.rising {
            back_out(m.out)
        } else {
            1.0 - back_in(1.0 - m.out)
        };
        if m.gag.is_none() && m.held.is_none() && m.fly.is_none() {
            feet[1] += (1.0 - up) * (height + 60.0);
            if m.out > 0.0 && m.out < 1.0 {
                let s = 0.08 * (m.out * std::f32::consts::PI).sin();
                squash = [1.0 - s * 0.5, 1.0 + s, 1.0 - s * 0.5];
            }
        }
        if m.gag.is_none() && m.out <= 0.0 && m.glass.is_none() {
            return None;
        }
        let clip_time = |clip: Clip, at: f32| {
            let (length, looping) = clip.length();
            let t = (now - at).max(0.0);
            let t = if looping { t } else { t.min(length - 0.01) };
            (t * 1000.0) as u32
        };
        let glass = m.glass.map(|g| map_overlay::Crack {
            center: [g.center[0] as i32, g.center[1] as i32],
            reach: g.reach as u16,
            seed: g.seed,
            age_ms: ((now - g.at).max(0.0) * 1000.0) as u32,
            fall_ms: g.fall_at.map(|f| ((now - f).max(0.0) * 1000.0) as u32),
            alpha: (g
                .fade_at
                .map_or(1.0, |f| 1.0 - ((now - f) / 1.2).clamp(0.0, 1.0))
                * 1000.0) as u16,
        });
        let shown = m.gag.is_some() || m.out > 0.0;
        Some(map_overlay::MascotView {
            clip: m.clip as u8,
            time_ms: clip_time(m.clip, m.clip_at),
            prev_clip: m.prev as u8,
            prev_time_ms: clip_time(m.prev, m.prev_at),
            blend: (((now - m.switched) / 0.22).clamp(0.0, 1.0) * 1000.0) as u16,
            feet: [feet[0].round() as i32, feet[1].round() as i32],
            height: if shown {
                height.round().max(1.0) as u16
            } else {
                0
            },
            yaw: (yaw * 10.0) as i16,
            pitch: (pitch * 10.0) as i16,
            roll: (roll * 10.0) as i16,
            squash: squash.map(|k| (k * 1000.0) as u16),
            alpha: if shown {
                (alpha.clamp(0.0, 1.0) * 1000.0) as u16
            } else {
                0
            },
            crack: glass,
            clock_ms,
            blink: (self.mascot_blink().clamp(0.0, 1.0) * 1000.0) as u16,
            mouth: (self.mascot_mouth().clamp(0.0, 1.0) * 1000.0) as u16,
            dazed: (dazed.clamp(0.0, 1.0) * 1000.0) as u16,
        })
    }
}
