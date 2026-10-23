//! The Queen's castle from the inside, built in its Three.js prototype
//! (`demos/demon-queen-castle-threejs`) and exported by its `tools/export-game.mjs`: the GLB parts
//! live in `data/assets/3d/castle/`, the plan in `data/castle/castle.json`.
//!
//! Nine rooms on a 3 x 3 grid and a balcony: the royal bedroom, the throne room and the war room
//! to the north; the library, the great hall and the dining room in the middle; the seal chamber,
//! the entrance hall (the gate out) and Lira and Nemi's room to the south. A stone stair in the hall
//! climbs to the upper floor over the south and west wings: a gallery over the hall, the infirmary,
//! the store, the armoury and the barracks; from up there the rest of the ground floor is roofs.
//! The camera frames the room she is in; the walls of that room that stand between the camera and
//! her drop to a stub (and what hangs on them goes), so every room is seen whole and its neighbours
//! stay closed. The upper floor shows only while she is up there.
use super::*;
use serde::Deserialize;

use velvet_render::{CutawayWalls3D, PlanRoom3D, PlanWall3D, ToonFire3D};

const RADIUS: f32 = 0.3;
/// How low a dropped wall goes (a fraction of its height).
const STUB: f32 = 0.12;
/// Where she wakes at the start of the story: in the bedroom, by the bed.
const BEDROOM_SPAWN: (f32, f32) = (-17.6, -12.6);
/// The two maids' rounds (closed loops): Lira about the kitchen, Nemi in their room.
const ROUNDS: [&[(f32, f32)]; 2] = [
    &[
        (20.2, 1.0),
        (22.2, 1.0),
        (22.2, 5.8),
        (19.2, 5.8),
        (19.2, 1.0),
    ],
    &[(13.6, 13.4), (20.4, 13.4), (20.4, 15.9), (13.6, 15.9)],
];
const MAID_SPEED: f32 = 0.75;
const MAID_RADIUS: f32 = 0.4;

// ------------------------------------------------------------------ the doctor upstairs
/// Nyxara's clips (`tools/doctor_blender.py`), in the order `DOCTOR_CLIPS` names them.
pub(crate) const DOCTOR_CLIPS: [&str; 5] =
    ["Nyx_Idle", "Nyx_Walk", "Nyx_Check", "Nyx_Write", "Nyx_Talk"];
pub(crate) const DOC_IDLE: usize = 0;
pub(crate) const DOC_WALK: usize = 1;
pub(crate) const DOC_CHECK: usize = 2;
pub(crate) const DOC_WRITE: usize = 3;
pub(crate) const DOC_TALK: usize = 4;
/// Her walk in the world: her stride (0.55 of her height a second, from the Blender script) at the
/// characters' scale, so her feet plant.
const DOCTOR_SPEED: f32 = 0.5524 * 2.0;
const DOCTOR_RADIUS: f32 = 0.4;
/// Her round in the infirmary: where she stops, which way she faces there, what she does and for
/// how long. She walks straight from each stop to the next and back to the first.
const DOCTOR_STOPS: [(f32, f32, f32, usize, f32); 8] = [
    // the four beds along the west wall, one patient at a time
    (-22.2, 11.6, -std::f32::consts::FRAC_PI_2, DOC_CHECK, 3.6),
    (-22.2, 14.6, -std::f32::consts::FRAC_PI_2, DOC_CHECK, 3.6),
    (-22.2, 17.6, -std::f32::consts::FRAC_PI_2, DOC_CHECK, 3.6),
    (-22.2, 20.3, -std::f32::consts::FRAC_PI_2, DOC_CHECK, 3.6),
    // round the brazier, to her desk to write it all down
    (-16.4, 20.3, 0.0, DOC_IDLE, 0.0),
    (-16.0, 15.6, std::f32::consts::FRAC_PI_2, DOC_WRITE, 4.8),
    // the medicine cabinet by the door
    (-13.5, 10.1, std::f32::consts::PI, DOC_CHECK, 3.6),
    (-18.5, 11.2, 0.0, DOC_IDLE, 2.0),
];

/// Where the doctor is in her round at time `t` (s): her place, which way she faces, her clip and
/// how long she has been in it, the clip before it (and how long ago it ended) for the crossfade.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Doctor {
    pub at: Vec3,
    pub facing: f32,
    pub clip: usize,
    pub since: f32,
    pub prev: usize,
}

fn turn_toward(from: f32, to: f32, k: f32) -> f32 {
    let mut d = (to - from) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    from + d * k.clamp(0.0, 1.0)
}

/// Her round as a schedule: (walk from stop i to i+1, then stay).
fn doctor_round_secs() -> f32 {
    let n = DOCTOR_STOPS.len();
    (0..n)
        .map(|i| {
            let a = DOCTOR_STOPS[i];
            let b = DOCTOR_STOPS[(i + 1) % n];
            a.4 + (b.0 - a.0).hypot(b.1 - a.1) / DOCTOR_SPEED
        })
        .sum()
}

pub(crate) fn doctor_at(t: f32) -> Doctor {
    let n = DOCTOR_STOPS.len();
    let mut left = t.rem_euclid(doctor_round_secs());
    for i in 0..n {
        let a = DOCTOR_STOPS[i];
        let b = DOCTOR_STOPS[(i + 1) % n];
        let heading = (b.0 - a.0).atan2(b.1 - a.1);
        let came = DOCTOR_STOPS[(i + n - 1) % n];
        let came_heading = (a.0 - came.0).atan2(a.1 - came.1);
        if left < a.4 {
            // Staying: she turns to what she tends.
            return Doctor {
                at: Vec3::new(a.0, 0.0, a.1),
                facing: turn_toward(came_heading, a.2, left / 0.45),
                clip: a.3,
                since: left,
                prev: DOC_WALK,
            };
        }
        left -= a.4;
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        let secs = len / DOCTOR_SPEED;
        if left < secs {
            let k = left / secs.max(1e-4);
            let from = if a.4 > 0.0 { a.2 } else { came_heading };
            return Doctor {
                at: Vec3::new(a.0 + (b.0 - a.0) * k, 0.0, a.1 + (b.1 - a.1) * k),
                facing: turn_toward(from, heading, left / 0.4),
                clip: DOC_WALK,
                since: left,
                prev: if a.4 > 0.0 { a.3 } else { DOC_WALK },
            };
        }
        left -= secs;
    }
    let a = DOCTOR_STOPS[0];
    Doctor {
        at: Vec3::new(a.0, 0.0, a.1),
        facing: a.2,
        clip: DOC_IDLE,
        since: 0.0,
        prev: DOC_IDLE,
    }
}

/// What she says when the Queen speaks to her.
const DOCTOR_LINES: [&str; 6] = [
    "—Majestad. Siéntese, que le tomo el pulso. …Rápido. Como siempre.",
    "—Las camas están vacías. Me gustaría que siguieran así. Mírese a usted.",
    "—¿Ha comido hoy? No me mienta: lo apunto todo.",
    "—El sello de su vientre ha latido esta noche. Lo he medido. Dos veces.",
    "—No toque el botiquín. Lo de la etiqueta roja no es caramelo.",
    "—Si se marea, avíseme. Si se desmaya, también. Por escrito, si puede.",
];

// ------------------------------------------------------------------ the plan
#[derive(Deserialize)]
pub(crate) struct Room {
    pub id: String,
    pub title: String,
    pub icon: String,
    /// The floor: 0 the ground floor, 1 the upper.
    #[serde(default)]
    pub level: u8,
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    #[allow(dead_code)]
    pub floor: String,
    pub open: bool,
}

impl Room {
    pub(crate) fn centre(&self) -> (f32, f32) {
        ((self.x0 + self.x1) * 0.5, (self.z0 + self.z1) * 0.5)
    }
    fn contains(&self, x: f32, z: f32) -> bool {
        x > self.x0 && x < self.x1 && z > self.z0 && z < self.z1
    }
}

#[derive(Deserialize)]
pub(crate) struct Door {
    #[serde(default)]
    pub level: u8,
    pub axis: String,
    pub at: f32,
    pub c: f32,
    pub w: f32,
    pub outside: bool,
}

#[derive(Deserialize)]
pub(crate) struct Wall {
    #[serde(default)]
    pub level: u8,
    pub axis: String,
    pub at: f32,
    pub a: f32,
    pub b: f32,
    pub door: bool,
    #[allow(dead_code)]
    pub h: f32,
}

#[derive(Deserialize)]
pub(crate) struct Hotspot {
    pub id: String,
    #[allow(dead_code)]
    pub room: String,
    #[serde(default)]
    pub level: u8,
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    pub icon: String,
    pub label: String,
    pub text: String,
}

#[derive(Deserialize)]
pub(crate) struct Lamp {
    #[serde(default)]
    pub level: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: String,
    pub power: f32,
    pub wall: i32,
}

#[derive(Deserialize)]
pub(crate) struct Flame {
    #[serde(default)]
    pub level: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: String,
    pub size: f32,
    pub kind: String,
    pub wall: i32,
}

#[derive(Deserialize)]
pub(crate) struct Point {
    pub x: f32,
    pub z: f32,
}

/// The hall's stair up to the gallery: across x0..x1, its foot at z_bottom (on the ground floor),
/// its top at z_top (on the upper floor).
#[derive(Deserialize)]
pub(crate) struct Stair {
    pub x0: f32,
    pub x1: f32,
    #[serde(rename = "zTop")]
    pub z_top: f32,
    #[serde(rename = "zBottom")]
    pub z_bottom: f32,
}

fn default_level_h() -> f32 {
    6.0
}

#[derive(Deserialize)]
pub(crate) struct Plan {
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    #[serde(rename = "wallT")]
    pub wall_t: f32,
    /// How high the upper floor stands.
    #[serde(rename = "levelH", default = "default_level_h")]
    pub level_h: f32,
    pub stair: Option<Stair>,
    pub walls: Vec<Wall>,
    /// [x, z, r, level]: pillars, braziers.
    pub circles: Vec<Vec<f32>>,
    /// [x, z, hx, hz, level]: the furniture.
    pub boxes: Vec<Vec<f32>>,
    pub hotspots: Vec<Hotspot>,
    pub spawn: Point,
    pub lamps: Vec<Lamp>,
    pub flames: Vec<Flame>,
    pub books: Option<Point>,
}

/// Where the story starts her: in the bedroom.
pub(crate) fn bedroom_spawn() -> (f32, f32) {
    BEDROOM_SPAWN
}

/// Just inside the gate, coming in from outside: inside the patio's gate (the castle's gate since
/// the patio de armas, 2026-10-10), else the plan's spawn in the hall.
pub(crate) fn gate_spawn() -> (f32, f32) {
    let p = plan();
    p.hotspots
        .iter()
        .find(|h| h.id == GATE)
        .map(|h| (h.x, h.z - 1.8))
        .filter(|&(x, z)| !p.blocked(x, z, RADIUS, 0))
        .unwrap_or((p.spawn.x, p.spawn.z))
}

/// The hotspot of the castle's way out (the patio's gate).
pub(crate) const GATE: &str = "puerta-castillo";

pub(crate) fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("castle/castle.json")).expect("valid castle plan")
    })
}

fn asset_root() -> PathBuf {
    runtime_3d_root().join("castle")
}

/// "#rrggbb" as linear RGB.
fn colour(hex: &str) -> [f32; 3] {
    let h = hex.trim_start_matches('#');
    let channel = |i: usize| {
        u8::from_str_radix(h.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255) as f32 / 255.0
    };
    let lin = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    [lin(channel(0)), lin(channel(2)), lin(channel(4))]
}

/// Where the Queen stands: her floor, and whether she is on the stair between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Walker {
    pub x: f32,
    pub z: f32,
    pub level: u8,
    pub on_stair: bool,
}

impl Plan {
    /// Whether a circle of radius `r` at (x, z) cannot stand there on floor `level`: outside its
    /// rooms and doorways, or in its furniture, or over the stair (its mass below, its well above:
    /// the stair itself is walked with [`Plan::step`]).
    pub(crate) fn blocked(&self, x: f32, z: f32, r: f32, level: u8) -> bool {
        let inset = self.wall_t / 2.0 + 0.05;
        let in_room = |room: &Room| {
            let m = if room.open { r + 0.25 } else { r };
            room.level == level
                && x > room.x0 + inset + m
                && x < room.x1 - inset - m
                && z > room.z0 + inset + m
                && z < room.z1 - inset - m
        };
        let in_door = |d: &Door| {
            let (across, along) = if d.axis == "x" {
                (x - d.at, z - d.c)
            } else {
                (z - d.at, x - d.c)
            };
            d.level == level && across.abs() < inset + 0.6 && along.abs() < d.w / 2.0 - 0.05 - r
        };
        let open =
            self.rooms.iter().any(in_room) || self.doors.iter().any(|d| !d.outside && in_door(d));
        if !open {
            return true;
        }
        if self
            .stair
            .as_ref()
            .is_some_and(|s| x + r > s.x0 && x - r < s.x1 && z + r > s.z_top && z - r < s.z_bottom)
        {
            return true;
        }
        let here = |v: &[f32], i: usize| v.get(i).copied().unwrap_or(0.0) as u8 == level;
        self.circles
            .iter()
            .any(|c| here(c, 3) && (x - c[0]).hypot(z - c[1]) < c[2] + r)
            || self
                .boxes
                .iter()
                .any(|b| here(b, 4) && (x - b[0]).abs() < b[2] + r && (z - b[1]).abs() < b[3] + r)
    }

    /// The room at (x, z) on floor `level`, if she is in one (not in a doorway).
    pub(crate) fn room_at(&self, x: f32, z: f32, level: u8) -> Option<usize> {
        self.rooms
            .iter()
            .position(|r| r.level == level && r.contains(x, z))
    }

    /// How far up the stair z is: 0 at its foot, 1 at its top.
    pub(crate) fn stair_t(&self, z: f32) -> f32 {
        self.stair.as_ref().map_or(0.0, |s| {
            ((s.z_bottom - z) / (s.z_bottom - s.z_top)).clamp(0.0, 1.0)
        })
    }

    /// Whether a circle of radius `r` at x fits between the stair's wall and its balustrade.
    fn in_stair_lane(&self, x: f32, r: f32) -> bool {
        self.stair
            .as_ref()
            .is_some_and(|s| x > s.x0 + 0.05 + r && x < s.x1 - 0.12 - r)
    }

    /// One step: she is on the stair once her body would touch its footprint from its foot (on
    /// the ground floor, heading north) or its top (upstairs, heading south), and steps off either
    /// end onto that floor once clear of it; on it only its sides stop her.
    pub(crate) fn step(&self, w: Walker, dx: f32, dz: f32) -> Option<Walker> {
        let (x, z, r) = (w.x + dx, w.z + dz, RADIUS);
        let Some(s) = &self.stair else {
            return (!self.blocked(x, z, r, w.level)).then_some(Walker { x, z, ..w });
        };
        let (foot, head) = (s.z_bottom + r, s.z_top - r);
        if w.on_stair {
            if !self.in_stair_lane(x, r) {
                return None;
            }
            if z >= foot {
                return (!self.blocked(x, z, r, 0)).then_some(Walker {
                    x,
                    z,
                    level: 0,
                    on_stair: false,
                });
            }
            if z <= head {
                return (!self.blocked(x, z, r, 1)).then_some(Walker {
                    x,
                    z,
                    level: 1,
                    on_stair: false,
                });
            }
            return Some(Walker { x, z, ..w });
        }
        if self.in_stair_lane(x, r) {
            if w.level == 0 && w.z >= foot - 1e-4 && z < foot {
                return Some(Walker {
                    x,
                    z,
                    level: 0,
                    on_stair: true,
                });
            }
            if w.level == 1 && w.z <= head + 1e-4 && z > head {
                return Some(Walker {
                    x,
                    z,
                    level: 1,
                    on_stair: true,
                });
            }
        }
        (!self.blocked(x, z, r, w.level)).then_some(Walker { x, z, ..w })
    }

    /// Her feet's height: the floor's, or up the stair (along the middle of the treads).
    pub(crate) fn height(&self, w: Walker) -> f32 {
        if !w.on_stair {
            return w.level as f32 * self.level_h;
        }
        let t = self.stair_t(w.z);
        t * self.level_h + 0.1 * (t * 30.0).min((1.0 - t) * 30.0).min(1.0)
    }

    /// The floor on screen: hers, or on the stair the upper one once she is high enough to be seen
    /// over the gallery's floor.
    pub(crate) fn view_level(&self, w: Walker) -> u8 {
        if w.on_stair {
            u8::from(self.stair_t(w.z) > 0.8)
        } else {
            w.level
        }
    }

    /// Whether wall `w` stands between a camera looking along `yaw` and room `room` (so it drops).
    fn wall_faces_camera(&self, w: &Wall, room: &Room, yaw: f32) -> bool {
        // A wall on axis "z" runs along x (it lies at z = at), one on "x" along z.
        let along_x = match w.axis.as_str() {
            "z" => true,
            "x" => false,
            _ => return false,
        };
        // The balcony (open) has no walls of its own: the wall of the room it hangs from hides it.
        let room_box = PlanRoom3D {
            x0: room.x0,
            x1: room.x1,
            z0: room.z0,
            z1: room.z1,
            open: room.open,
        };
        w.level == room.level
            && room_box.wall_faces_camera(
                &PlanWall3D {
                    along_x,
                    at: w.at,
                    a: w.a,
                    b: w.b,
                },
                yaw,
            )
    }

    /// The bounds of the whole castle (x0, x1, z0, z1).
    pub(crate) fn bounds(&self) -> [f32; 4] {
        self.rooms
            .iter()
            .fold([f32::MAX, f32::MIN, f32::MAX, f32::MIN], |b, r| {
                [
                    b[0].min(r.x0),
                    b[1].max(r.x1),
                    b[2].min(r.z0),
                    b[3].max(r.z1),
                ]
            })
    }
}

/// The game's icon for a hotspot's.
pub(crate) fn icon(name: &str) -> Icon {
    match name {
        "door" => Icon::Leave,
        "crown" => Icon::Crown,
        "dress" => Icon::Wardrobe,
        "bed" => Icon::Rest,
        "mirror" => Icon::Heart,
        "heart" => Icon::Heart,
        "moon" => Icon::Castle,
        "map" => Icon::Council,
        "book" => Icon::Memory,
        "seal" => Icon::Rune,
        "tea" => Icon::Feast,
        "maid" => Icon::Talk,
        // The upper floor: the portraits, the infirmary, the armoury, the store.
        "frame" => Icon::Memory,
        "cross" => Icon::Heart,
        "sword" => Icon::Gear,
        "box" => Icon::Chest,
        // the patio: its gate out, the archery butts, the well
        "gate" => Icon::Leave,
        "target" => Icon::Gear,
        "well" => Icon::Feast,
        _ => Icon::Castle,
    }
}

/// The verb on a hotspot's prompt.
fn verb(id: &str) -> &'static str {
    match id {
        "puerta-castillo" => "Salir",
        "porton" => "Mirar",
        "trono" => "Sentarse",
        "vestidor" => "Cambiarse",
        "cama" => "Descansar",
        "tocador" => "Arreglarse",
        "sofa" => "Invitar a una cita",
        "sillon" => "Caricias",
        "balcon" => "Asomarse",
        "catalejo" => "Mirar por el catalejo",
        "mesa-guerra" => "Planear la guerra",
        "estudio" => "Leer",
        "sello" => "Tocar el sello",
        "hogar" => "Mirar",
        "lira" | "nemi" | "nyxara" => "Hablar",
        _ => "Mirar",
    }
}

// ------------------------------------------------------------------ the Queen in it
/// What pressing F did, for the explorer to carry out.
pub(crate) enum Action {
    None,
    Leave,
    Wardrobe,
    Sleep,
    Say(&'static str),
}

/// A maid's line, said over her head for a while.
#[derive(Clone, Debug)]
pub(crate) struct Talk {
    pub maid: usize,
    pub text: &'static str,
    pub until: f32,
}

/// What the court has done to the castle: the rooms Lira's works restored and what the throne
/// decided, as the castle shows it (the court's life in its rooms, `zones/castle_life.rs`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Life {
    pub infirmary: bool,
    pub armory: bool,
    pub kitchen: bool,
    pub barracks: bool,
    pub library: bool,
    /// Bertila in the yard: 0 not yet, 1 a goat, 2 the crown's goat (with her crown).
    pub goat: u8,
    /// Her kids: 0 none, 1 two kids, 2 two kids and a bell for her.
    pub kids: u8,
    /// The refugees from the north: 0 none, 1 camped and fed, 2 working for their roof.
    pub refugees: u8,
    /// The smith's new swords (a crate by the yard's rack), her ploughs (by the wall).
    pub swords: bool,
    pub ploughs: bool,
    /// Durgo and Pimpa's wedding garlands on the gate; the village's feast lanterns.
    pub wedding: bool,
    pub feast: bool,
    /// The stone with the capital's dead.
    pub memorial: bool,
}

/// Where the court's people stand: the guards at the gate and in the barracks (x, z, facing), the
/// armoury's smith at her anvil, the refugees' fire, their tents and themselves round it.
pub(crate) const GATE_GUARDS: [(f32, f32, f32); 2] = [(-2.45, 35.3, 0.0), (2.45, 35.3, 0.0)];
pub(crate) const BARRACK_GUARDS: [(f32, f32, f32); 2] = [(16.0, 20.9, 1.82), (20.0, 21.0, -1.86)];
pub(crate) const SMITH: (f32, f32) = (20.9, 10.62);
/// The armoury's forge against its back wall, and the anvil before it.
pub(crate) const FORGE: (f32, f32) = (20.85, 8.72);
pub(crate) const ANVIL: (f32, f32) = (20.9, 9.95);
pub(crate) const CAMPFIRE: (f32, f32) = (18.3, 28.9);
pub(crate) const TENTS: [(f32, f32, f32); 2] = [(15.4, 30.4, 0.5), (21.4, 31.2, -0.4)];
pub(crate) const REFUGEES: [(f32, f32); 4] = [
    (19.45, 29.15),
    (18.75, 27.85),
    (17.2, 28.35),
    (17.55, 29.85),
];

impl Life {
    /// What stands in her way because of it: boxes (x, z, half width, half depth, floor).
    pub(crate) fn obstacles(&self) -> Vec<[f32; 5]> {
        let mut out = Vec::new();
        // the armoury's forge and anvil are there restored or not (cold until it is)
        out.extend([
            [FORGE.0, FORGE.1, 0.76, 0.42, 1.0],
            [ANVIL.0, ANVIL.1, 0.32, 0.22, 1.0],
        ]);
        if self.armory {
            out.push([SMITH.0, SMITH.1, 0.28, 0.28, 1.0]);
        }
        if !self.library {
            out.extend([[-23.2, -1.8, 0.85, 0.65, 0.0], [-12.7, 5.5, 0.7, 0.6, 0.0]]);
        }
        if self.barracks {
            out.extend(GATE_GUARDS.map(|(x, z, _)| [x, z, 0.3, 0.3, 0.0]));
            out.extend(BARRACK_GUARDS.map(|(x, z, _)| [x, z, 0.3, 0.3, 1.0]));
        }
        if self.refugees > 0 {
            out.extend(TENTS.map(|(x, z, _)| [x, z, 1.1, 0.95, 0.0]));
            out.push([CAMPFIRE.0, CAMPFIRE.1, 0.55, 0.55, 0.0]);
            out.extend(REFUGEES.map(|(x, z)| [x, z, 0.26, 0.26, 0.0]));
        }
        if self.swords {
            out.push([-22.9, 27.7, 0.5, 0.35, 0.0]);
        }
        if self.ploughs {
            out.push([-7.4, 22.95, 1.0, 0.3, 0.0]);
        }
        if self.memorial {
            out.push([-18.0, 36.35, 0.55, 0.4, 0.0]);
        }
        out
    }
}

/// The castle as the Queen lives it: where she is, which walls are down, where the maids are.
#[derive(Clone, Debug)]
pub(crate) struct State {
    pub x: f32,
    pub z: f32,
    /// Her floor (0 the ground floor, 1 the upper), and whether she is on the stair between.
    pub level: u8,
    pub on_stair: bool,
    /// The room she is in, on the floor in view (the last one, in a doorway).
    pub room: usize,
    /// The walls, dropping out of the camera's way and rising again.
    walls: CutawayWalls3D,
    /// How far along her round each maid is (metres).
    maid_along: [f32; 2],
    pub talk: Option<Talk>,
    /// A maid kept still (the Queen is in a conversation with her).
    pub held: Option<usize>,
    /// A maid kept at a place of her own instead of her round (Nemi at the arena while she is
    /// training the Queen): where she stands.
    pub maid_post: [Option<(f32, f32)>; 2],
    /// Every wall up whatever the camera (the castle seen from above, for its map).
    pub keep_walls: bool,
    /// The doctor upstairs, where her round has her now (and how long she has been held talking).
    pub doctor: Doctor,
    doctor_clock: f32,
    /// What the court has done to the castle (set by the game every frame from the story).
    pub life: Life,
}

fn round_length(points: &[(f32, f32)]) -> f32 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| (b.0 - a.0).hypot(b.1 - a.1))
        .sum()
}

/// Where along a closed round, and which way it heads.
fn on_round(points: &[(f32, f32)], along: f32) -> (Vec3, f32) {
    let mut left = along.rem_euclid(round_length(points));
    for (a, b) in points.iter().zip(points.iter().cycle().skip(1)) {
        let len = (b.0 - a.0).hypot(b.1 - a.1);
        if left <= len {
            let t = left / len.max(1e-4);
            return (
                Vec3::new(a.0 + (b.0 - a.0) * t, 0.0, a.1 + (b.1 - a.1) * t),
                (b.0 - a.0).atan2(b.1 - a.1),
            );
        }
        left -= len;
    }
    (Vec3::new(points[0].0, 0.0, points[0].1), 0.0)
}

impl State {
    fn at(x: f32, z: f32) -> Self {
        Self::placed(Walker {
            x,
            z,
            level: 0,
            on_stair: false,
        })
    }

    fn placed(w: Walker) -> Self {
        let p = plan();
        let room = p
            .room_at(w.x, w.z, p.view_level(w))
            .or_else(|| p.room_at(w.x, w.z, w.level))
            .unwrap_or(0);
        Self {
            x: w.x,
            z: w.z,
            level: w.level,
            on_stair: w.on_stair,
            room,
            walls: CutawayWalls3D::new(p.walls.len(), STUB),
            maid_along: [0.0, 3.0],
            talk: None,
            held: None,
            maid_post: [None, None],
            keep_walls: false,
            doctor: doctor_at(0.0),
            doctor_clock: 0.0,
            life: Life::default(),
        }
    }

    pub(crate) fn walker(&self) -> Walker {
        Walker {
            x: self.x,
            z: self.z,
            level: self.level,
            on_stair: self.on_stair,
        }
    }

    /// The floor in view (see [`Plan::view_level`]).
    pub(crate) fn view_level(&self) -> u8 {
        plan().view_level(self.walker())
    }

    /// Just inside the gate, coming in from outside.
    pub(crate) fn at_gate() -> Self {
        let (x, z) = gate_spawn();
        Self::at(x, z)
    }

    /// In the bedroom (the start of the story).
    pub(crate) fn in_bedroom() -> Self {
        Self::at(BEDROOM_SPAWN.0, BEDROOM_SPAWN.1)
    }

    /// Back where she was saved (her floor by its height), if that is a free spot of this castle
    /// or the stair; else in the bedroom.
    pub(crate) fn restore(position: Vec3) -> Self {
        let p = plan();
        let level = u8::from(position.y > p.level_h * 0.5);
        let (x, z) = (position.x, position.z);
        let on_stair = p.in_stair_lane(x, RADIUS)
            && p.stair
                .as_ref()
                .is_some_and(|s| z > s.z_top - RADIUS && z < s.z_bottom + RADIUS);
        if on_stair || !p.blocked(x, z, RADIUS, level) {
            Self::placed(Walker {
                x,
                z,
                level,
                on_stair,
            })
        } else {
            Self::in_bedroom()
        }
    }

    pub(crate) fn position(&self) -> Vec3 {
        Vec3::new(self.x, plan().height(self.walker()), self.z)
    }

    /// Visual QA: the doctor's round restarted so she stands near `at` (a fresh state starts her
    /// round from its beginning): the clock that puts her closest to it.
    pub(crate) fn doctor_qa_hold(&mut self, at: Vec3) {
        let total = doctor_round_secs();
        let mut best = (f32::MAX, 0.0);
        let mut t = 0.0;
        while t < total {
            let d = doctor_at(t).at;
            let dist = (d.x - at.x).hypot(d.z - at.z);
            if dist < best.0 {
                best = (dist, t);
            }
            t += 0.05;
        }
        self.doctor_clock = best.1;
        self.doctor = doctor_at(best.1);
    }

    /// Where whoever is talking stands: a maid, or the doctor (2).
    fn speaker_at(&self, who: usize) -> Vec3 {
        if who == 2 {
            self.doctor.at
        } else {
            self.maid_positions()[who]
        }
    }

    fn maid_positions(&self) -> [Vec3; 2] {
        [self.maid(0).0, self.maid(1).0]
    }

    /// Where maid `i` is and which way she faces: at her post (turned to the Queen), else along
    /// her round.
    pub(crate) fn maid(&self, i: usize) -> (Vec3, f32) {
        match self.maid_post[i] {
            Some((x, z)) => (Vec3::new(x, 0.0, z), (self.x - x).atan2(self.z - z)),
            None => on_round(ROUNDS[i], self.maid_along[i]),
        }
    }

    /// The place of a hotspot by its id (a maid's: where she is now), and its floor.
    pub(crate) fn place_of(&self, id: &str) -> Option<(f32, f32, u8)> {
        match id {
            "lira" | "nemi" => {
                let (at, _) = self.maid(usize::from(id == "nemi"));
                Some((at.x, at.z, 0))
            }
            _ => plan()
                .hotspots
                .iter()
                .find(|h| h.id == id)
                .map(|h| (h.x, h.z, h.level)),
        }
    }

    /// The Queen brought to (x, z) on floor `level`: on the free spot nearest it a step and a half
    /// away (on the side she comes from, if it is free), turned to it. Whether she could be placed.
    pub(crate) fn bring_to(&mut self, x: f32, z: f32, level: u8) -> Option<f32> {
        let p = plan();
        let from = (self.x - x).atan2(self.z - z);
        for ring in [1.5f32, 2.0, 2.6, 3.2] {
            for k in 0..16 {
                let turn = (k as f32 / 2.0).ceil() * if k % 2 == 0 { 1.0 } else { -1.0 };
                let a = from + turn * std::f32::consts::TAU / 16.0;
                let (qx, qz) = (x + a.sin() * ring, z + a.cos() * ring);
                if p.blocked(qx, qz, RADIUS, level) {
                    continue;
                }
                let placed = Self::placed(Walker {
                    x: qx,
                    z: qz,
                    level,
                    on_stair: false,
                });
                self.x = placed.x;
                self.z = placed.z;
                self.level = placed.level;
                self.on_stair = false;
                self.room = placed.room;
                return Some((x - qx).atan2(z - qz));
            }
        }
        None
    }

    /// Where a step to (x, z) puts her, if she can take it: nothing solid there, and no maid in
    /// the way (stepping away from one is always allowed, so a maid who walks up to her never pins
    /// her; the maids keep to the ground floor).
    fn step_to(&self, x: f32, z: f32) -> Option<Walker> {
        let next = plan().step(self.walker(), x - self.x, z - self.z)?;
        let near = |m: Vec3, r: f32| {
            let after = (m.x - x).hypot(m.z - z);
            after > r + RADIUS || after > (m.x - self.x).hypot(m.z - self.z)
        };
        let clear = if next.on_stair {
            true
        } else if next.level == 0 {
            self.maid_positions().iter().all(|m| near(*m, MAID_RADIUS))
        } else {
            near(self.doctor.at, DOCTOR_RADIUS)
        };
        // What the court put in the rooms (she may still step out of it if she is in it).
        let inside = |b: &[f32; 5], x: f32, z: f32| {
            (x - b[0]).abs() < b[2] + RADIUS && (z - b[1]).abs() < b[3] + RADIUS
        };
        let free =
            next.on_stair
                || self.life.obstacles().iter().all(|b| {
                    b[4] as u8 != next.level || !inside(b, x, z) || inside(b, self.x, self.z)
                });
        (clear && free).then_some(next)
    }

    /// One step, sliding along whatever is in the way: straight on, else along the wall (one axis),
    /// else turned a little round a column or a door's jamb (so she never sticks on a corner).
    pub(crate) fn walk(&mut self, dx: f32, dz: f32) {
        let (x, z) = (self.x, self.z);
        let turned = |a: f32| {
            let (s, c) = a.sin_cos();
            let k = c.max(0.2);
            ((dx * c - dz * s) * k, (dx * s + dz * c) * k)
        };
        let mut tries = vec![(dx, dz)];
        // Pressed against a doorway's jamb, heading through it: edge along the wall into the
        // opening first (sliding on the other axis, or turning aside, would carry her off the
        // wall's line and away from the opening).
        let step = dx.hypot(dz);
        let p = plan();
        let level = self.level;
        if let Some(d) = p
            .doors
            .iter()
            .filter(|d| !d.outside && d.level == level && !self.on_stair)
            .find(|d| {
                let (across, along, push) = if d.axis == "x" {
                    (x - d.at, z - d.c, (d.at - x) * dx)
                } else {
                    (z - d.at, x - d.c, (d.at - z) * dz)
                };
                across.abs() < 1.4 && along.abs() < d.w / 2.0 + 0.6 && push > 0.0
            })
        {
            let along = if d.axis == "x" { z - d.c } else { x - d.c };
            let toward = -along.signum() * step;
            tries.push(if d.axis == "x" {
                (0.0, toward)
            } else {
                (toward, 0.0)
            });
        }
        if dx.abs() > dz.abs() {
            tries.extend([(dx, 0.0), (0.0, dz)]);
        } else {
            tries.extend([(0.0, dz), (dx, 0.0)]);
        }
        tries.extend([0.6f32, -0.6, 1.2, -1.2].map(turned));
        if let Some(next) = tries
            .iter()
            .filter(|(sx, sz)| sx.abs() > 1e-6 || sz.abs() > 1e-6)
            .find_map(|&(sx, sz)| self.step_to(x + sx, z + sz))
        {
            (self.x, self.z, self.level, self.on_stair) =
                (next.x, next.z, next.level, next.on_stair);
        }
        if let Some(room) = p.room_at(self.x, self.z, self.view_level()) {
            self.room = room;
        }
        if let Some(talk) = &self.talk {
            let m = self.speaker_at(talk.maid);
            if (m.x - self.x).hypot(m.z - self.z) > 3.5 {
                self.talk = None;
            }
        }
    }

    /// The walls drop and rise, the maids go their rounds (they stop to talk, and for her).
    pub(crate) fn update(&mut self, dt: f32, camera_yaw: f32, elapsed: f32) {
        let p = plan();
        let room = &p.rooms[self.room];
        let keep_walls = self.keep_walls;
        let before: Vec<bool> = (0..p.walls.len()).map(|i| self.walls.dressed(i)).collect();
        self.walls.update(dt, |i| {
            !keep_walls && p.wall_faces_camera(&p.walls[i], room, camera_yaw)
        });
        // VELVET_WALL_LOG: say when a wall puts on or takes off what hangs on it (and its door).
        if std::env::var_os("VELVET_WALL_LOG").is_some() {
            for (i, was) in before.iter().enumerate() {
                if *was != self.walls.dressed(i) {
                    eprintln!(
                        "[walls] {i} door={} dressed {} -> {} (room {}, at {:.2},{:.2})",
                        p.walls[i].door, was, !was, room.id, self.x, self.z
                    );
                }
            }
        }
        if self.talk.as_ref().is_some_and(|t| elapsed > t.until) {
            self.talk = None;
        }
        // The doctor goes her round, unless she is talking or the Queen stands just ahead of her.
        let doc_talking = self.talk.as_ref().is_some_and(|t| t.maid == 2);
        let (dx, dz) = (self.x - self.doctor.at.x, self.z - self.doctor.at.z);
        let ahead = dx * self.doctor.facing.sin() + dz * self.doctor.facing.cos();
        let doc_blocked = self.level == 1
            && !self.on_stair
            && self.doctor.clip == DOC_WALK
            && ahead > 0.0
            && dx.hypot(dz) < DOCTOR_RADIUS + RADIUS + 0.45;
        if !doc_talking && !doc_blocked {
            self.doctor_clock += dt;
            self.doctor = doctor_at(self.doctor_clock);
        } else if doc_talking {
            // Turned to the Queen, explaining.
            let to_queen = dx.atan2(dz);
            self.doctor.facing = turn_toward(self.doctor.facing, to_queen, dt * 8.0);
            if self.doctor.clip != DOC_TALK {
                self.doctor.prev = self.doctor.clip;
                self.doctor.clip = DOC_TALK;
                self.doctor.since = 0.0;
            }
            self.doctor.since += dt;
        } else {
            if self.doctor.clip != DOC_IDLE {
                self.doctor.prev = self.doctor.clip;
                self.doctor.clip = DOC_IDLE;
                self.doctor.since = 0.0;
            }
            self.doctor.since += dt;
        }
        for i in 0..2 {
            let talking = self.talk.as_ref().is_some_and(|t| t.maid == i);
            // She waits while the Queen stands in her way (just ahead of her), else walks on.
            let (at, facing) = self.maid(i);
            let (dx, dz) = (self.x - at.x, self.z - at.z);
            let ahead = dx * facing.sin() + dz * facing.cos();
            let blocked = ahead > 0.0 && dx.hypot(dz) < MAID_RADIUS + RADIUS + 0.45;
            if !talking && !blocked && self.held != Some(i) {
                self.maid_along[i] += MAID_SPEED * dt;
            }
        }
    }

    /// How tall wall `i` stands this frame (1 whole .. STUB dropped).
    pub(crate) fn wall_scale(&self, i: usize) -> f32 {
        self.walls.scale(i)
    }

    /// Whether what hangs on wall `i` is shown (not while it is down).
    pub(crate) fn wall_dressed(&self, i: usize) -> bool {
        self.walls.dressed(i)
    }

    /// Where the camera looks: the room she is in, leaning a little towards her, at her height.
    pub(crate) fn focus(&self) -> Vec3 {
        let p = plan();
        let (cx, cz) = p.rooms[self.room].centre();
        Vec3::new(
            cx + ((self.x - cx) * 0.45).clamp(-3.5, 3.5),
            p.height(self.walker()),
            cz + ((self.z - cz) * 0.45).clamp(-3.5, 3.5),
        )
    }

    /// The hotspot within reach on her floor (the maids' go with them).
    pub(crate) fn nearest(&self) -> Option<&'static Hotspot> {
        if self.on_stair {
            return None;
        }
        let maids = self.maid_positions();
        plan()
            .hotspots
            .iter()
            .filter(|h| h.level == self.level)
            .map(|h| {
                let (x, z) = match h.id.as_str() {
                    "lira" => (maids[0].x, maids[0].z),
                    "nemi" => (maids[1].x, maids[1].z),
                    "nyxara" => (self.doctor.at.x, self.doctor.at.z),
                    _ => (h.x, h.z),
                };
                (h, (x - self.x).hypot(z - self.z))
            })
            .filter(|(h, d)| *d < h.radius.max(1.8))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    pub(crate) fn prompt(&self) -> Option<Prompt> {
        let h = self.nearest()?;
        let detail = if h.id == GATE {
            "Al bosque".to_owned()
        } else {
            h.label.clone()
        };
        Some(Prompt::new(icon(&h.icon), verb(&h.id)).detail(detail))
    }

    /// F: leave, open the wardrobe, rest, talk, or look.
    pub(crate) fn interact(&mut self, elapsed: f32) -> Action {
        let Some(h) = self.nearest() else {
            return Action::None;
        };
        match h.id.as_str() {
            GATE => Action::Leave,
            "vestidor" => Action::Wardrobe,
            "cama" => Action::Sleep,
            "nyxara" => {
                let k = ((elapsed / 3.7) as usize) % DOCTOR_LINES.len();
                self.talk = Some(Talk {
                    maid: 2,
                    text: DOCTOR_LINES[k],
                    until: elapsed + 6.0,
                });
                Action::None
            }
            "lira" | "nemi" => {
                let maid = usize::from(h.id == "nemi");
                self.talk = Some(Talk {
                    maid,
                    text: authored_maid_line(maid, "queen", elapsed),
                    until: elapsed + 5.0,
                });
                Action::None
            }
            _ => Action::Say(h.text.as_str()),
        }
    }

    /// The line being said, over the maid who says it.
    pub(crate) fn bubbles(&self, camera: Camera3D) -> Vec<map_overlay::NpcSpeechBubble> {
        let Some(talk) = &self.talk else {
            return Vec::new();
        };
        let at = if talk.maid == 2 {
            self.doctor.at + Vec3::new(0.0, plan().level_h, 0.0)
        } else {
            self.maid(talk.maid).0
        };
        let Some((x, y)) = project_npc_anchor(camera, at + Vec3::new(0.0, 3.4, 0.0)) else {
            return Vec::new();
        };
        vec![map_overlay::NpcSpeechBubble {
            x,
            y,
            speaker: match talk.maid {
                0 => "Lira",
                1 => "Nemi",
                _ => "Nyxara",
            },
            text: talk.text,
            opacity: 1.0,
            accent: [0.66, 0.42, 0.92, 1.0],
        }]
    }

    /// The flames nearest the room she is in that can be seen (on the floor in view, not on a
    /// dropped wall), flickering.
    pub(crate) fn lights(&self, elapsed: f32) -> Vec<PointLight3D> {
        let p = plan();
        let focus = self.focus();
        let view = self.view_level();
        let mut near: Vec<(f32, usize)> = p
            .lamps
            .iter()
            .enumerate()
            .filter(|(_, l)| l.level == view && (l.wall < 0 || self.wall_dressed(l.wall as usize)))
            .map(|(i, l)| ((l.x - focus.x).hypot(l.z - focus.z), i))
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        near.truncate(24);
        near.into_iter()
            .map(|(_, i)| {
                let l = &p.lamps[i];
                let flicker = 1.0
                    + (elapsed * 7.0 + i as f32 * 1.7).sin() * 0.06
                    + (elapsed * 17.0 + i as f32).sin() * 0.04;
                PointLight3D::new(
                    Vec3::new(l.x, l.y, l.z),
                    colour(&l.color),
                    7.5 * l.power * flicker,
                    9.0,
                )
            })
            .collect()
    }
}

/// The castle's finish: the renderer's plain one, with the torchlight bounced off the floors and
/// walls onto what is round them (the rooms are lit from a few flames; the bounce fills them).
pub(crate) fn finish() -> Finish3D {
    Finish3D {
        gi: 2.6,
        gi_radius: 3.5,
        gi_ambient: 0.7,
        ..Finish3D::default()
    }
}

/// Indoors by torchlight: cool moonlight through the windows, warm flames, violet in the stone.
pub(crate) fn mood(env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    let mut env = super::castle::mood(env);
    env.lighting.ambient_color = [0.24, 0.19, 0.32];
    env.atmosphere.fog_start = 40.0;
    env.atmosphere.fog_end = 90.0;
    env
}

// ------------------------------------------------------------------ the scene
struct Part {
    mesh: MeshHandle,
    transform: Mat4,
    material: Material3D,
    texture: Option<TextureHandle>,
}

fn load_parts(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    file: &str,
) -> Result<Vec<(String, Part)>> {
    let asset = upload_static_asset(renderer, device, queue, &asset_root().join(file), cache)?;
    Ok(asset
        .parts
        .into_iter()
        .zip(asset.part_names)
        .map(|(part, name)| {
            let glow = name.ends_with("lampGlass");
            let material = if glow {
                part.material.with_emission(1.3)
            } else {
                part.material.with_ambient_occlusion(0.18)
            };
            (
                name,
                Part {
                    mesh: part.mesh,
                    transform: part.local_transform,
                    material,
                    texture: part.texture,
                },
            )
        })
        .collect())
}

/// A soft round glow (for the halos round the flames).
fn halo_texture() -> TextureData3D {
    let n = 64;
    let mut rgba = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let d = ((x as f32 + 0.5 - n as f32 / 2.0).hypot(y as f32 + 0.5 - n as f32 / 2.0)
                / (n as f32 / 2.0))
                .min(1.0);
            let a = ((1.0 - d).powf(2.2) * 255.0) as u8;
            rgba.extend_from_slice(&[255, 255, 255, a]);
        }
    }
    TextureData3D {
        width: n as u32,
        height: n as u32,
        rgba,
    }
}

/// A unit quad standing in the x/y plane (turned to the camera when drawn).
fn quad() -> Mesh3D {
    let v =
        |x: f32, y: f32, u: f32, w: f32| MeshVertex3D::new([x, y, 0.0], [0.0, 0.0, 1.0], [u, w]);
    Mesh3D {
        vertices: vec![
            v(-0.5, -0.5, 0.0, 1.0),
            v(0.5, -0.5, 1.0, 1.0),
            v(0.5, 0.5, 1.0, 0.0),
            v(-0.5, 0.5, 0.0, 0.0),
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
        ..Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0))
    }
}

/// The castle's GPU side.
pub(crate) struct Scene {
    statics: Vec<Part>,
    /// The upper floor's rooms, and the ground floor's roofs seen from up there.
    upper: Vec<Part>,
    roofs: Vec<Part>,
    /// Per wall: its stone, and what hangs on it.
    walls: Vec<(Vec<Part>, Vec<Part>)>,
    seal: Vec<Part>,
    seal_glow: Vec<Part>,
    books: Vec<Part>,
    flame: MeshHandle,
    halo: MeshHandle,
    halo_texture: TextureHandle,
    /// The great fires (hearths and braziers), with their floor.
    fires: Vec<(u8, ToonFire3D)>,
    /// The kitchen's hearth (its place among the fires) and the one small fire it keeps until
    /// the kitchen is restored.
    kitchen_fire: Option<(usize, ToonFire3D)>,
    seal_centre: Vec3,
}

impl Scene {
    pub(crate) fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self> {
        let started = Instant::now();
        super::prefetch::prefetch(super::zone_files(&asset_root()));
        let p = plan();
        let mut cache = HashMap::new();
        let statics = load_parts(renderer, device, queue, &mut cache, "castle_static.glb")?
            .into_iter()
            .map(|(_, part)| part)
            .collect();
        let mut optional = |file: &str| -> Result<Vec<Part>> {
            if !asset_root().join(file).exists() {
                return Ok(Vec::new());
            }
            Ok(load_parts(renderer, device, queue, &mut cache, file)?
                .into_iter()
                .map(|(_, part)| part)
                .collect())
        };
        let upper = optional("castle_upper.glb")?;
        let roofs = optional("castle_roofs.glb")?;
        let mut walls: Vec<(Vec<Part>, Vec<Part>)> = (0..p.walls.len())
            .map(|_| (Vec::new(), Vec::new()))
            .collect();
        for (name, part) in load_parts(renderer, device, queue, &mut cache, "castle_walls.glb")? {
            // `w<i>:<material>` the wall, `d<i>:<material>` its decor.
            let tag = name.split(':').next().unwrap_or("");
            let Some(index) = tag
                .get(1..)
                .and_then(|i| i.parse::<usize>().ok())
                .filter(|&i| i < walls.len())
            else {
                continue;
            };
            if tag.starts_with('w') {
                walls[index].0.push(part);
            } else {
                walls[index].1.push(part);
            }
        }
        let (mut seal, mut seal_glow, mut books) = (Vec::new(), Vec::new(), Vec::new());
        for (name, part) in load_parts(renderer, device, queue, &mut cache, "castle_movers.glb")? {
            match name.as_str() {
                "seal" => seal.push(part),
                "sealGlow" => seal_glow.push(part),
                n if n.starts_with("book") => books.push(part),
                _ => {}
            }
        }
        let flame = renderer.upload_mesh(
            device,
            &Mesh3D::uv_sphere(1.0, 10, 8, Material3D::rgb(1.0, 1.0, 1.0)),
        )?;
        let halo = renderer.upload_mesh(device, &quad())?;
        let halo_texture = renderer.upload_texture(device, queue, &halo_texture())?;
        let mut fires = Vec::new();
        let mut kitchen_fire = None;
        for f in p
            .flames
            .iter()
            .filter(|f| f.kind == "hearth" || f.kind == "brazier")
        {
            let width = if f.kind == "hearth" { 0.9 } else { 0.55 };
            let mut fire =
                ToonFire3D::new(renderer, device, Vec3::new(f.x, f.y - 0.05, f.z), width)?;
            if f.kind == "brazier" {
                let c = colour(&f.color);
                fire = if c[2] > c[0] {
                    fire.with_palette([[0.32, 0.06, 0.62], [0.62, 0.24, 0.95], [0.95, 0.72, 1.0]])
                } else {
                    fire
                };
            }
            if f.kind == "hearth" && f.level == 0 && f.x > 10.0 && f.z < 8.0 {
                let small =
                    ToonFire3D::new(renderer, device, Vec3::new(f.x, f.y - 0.05, f.z), 0.32)?;
                kitchen_fire = Some((fires.len(), small));
            }
            fires.push((f.level, fire));
        }
        let seal_centre = p
            .rooms
            .iter()
            .find(|r| r.id == "sello")
            .map_or(Vec3::ZERO, |r| {
                let (x, z) = r.centre();
                Vec3::new(x, 0.0, z)
            });
        eprintln!(
            "[3d] castle loaded in {:.0} ms ({} walls)",
            started.elapsed().as_secs_f64() * 1000.0,
            walls.len()
        );
        Ok(Self {
            statics,
            upper,
            roofs,
            walls,
            seal,
            seal_glow,
            books,
            flame,
            halo,
            halo_texture,
            fires,
            kitchen_fire,
            seal_centre,
        })
    }

    /// Everything to draw this frame for `state`; `toward` points from the scene to the camera.
    pub(crate) fn draws(
        &self,
        state: &State,
        elapsed: f32,
        toward: Vec3,
        draws: &mut Vec<MeshDraw3D>,
    ) {
        let p = plan();
        let push = |draws: &mut Vec<MeshDraw3D>, parts: &[Part], world: Mat4| {
            for part in parts {
                draws.push(MeshDraw3D {
                    mesh: part.mesh,
                    transform: world * part.transform,
                    material: part.material,
                    texture: part.texture,
                });
            }
        };
        // The upper floor and the roofs only while she is up there (they would hide her below).
        let view = state.view_level();
        push(draws, &self.statics, Mat4::IDENTITY);
        if view == 1 {
            push(draws, &self.upper, Mat4::IDENTITY);
            push(draws, &self.roofs, Mat4::IDENTITY);
        }
        for (i, (body, decor)) in self.walls.iter().enumerate() {
            let level = p.walls[i].level;
            if level > view {
                continue;
            }
            let s = state.wall_scale(i);
            let dressed = state.wall_dressed(i);
            // A doorway's lintel would float at shin height: it goes with the decor. A wall drops
            // towards its own floor.
            if dressed || !p.walls[i].door {
                let base = Vec3::new(0.0, level as f32 * p.level_h, 0.0);
                push(
                    draws,
                    body,
                    Mat4::from_translation(base)
                        * Mat4::from_scale(Vec3::new(1.0, s, 1.0))
                        * Mat4::from_translation(-base),
                );
            }
            if dressed {
                push(draws, decor, Mat4::IDENTITY);
            }
        }
        // (The ground floor's moving things are under the upper floor's slabs.)
        if view == 1 {
            self.draw_flames(state, elapsed, toward, view, draws);
            return;
        }
        // The seal turns slowly and beats harder as she comes near.
        let near = if p.rooms[state.room].id == "sello" {
            (1.0 - (state.x - self.seal_centre.x).hypot(state.z - self.seal_centre.z) / 6.0)
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let pulse = 0.75 + 0.25 * (elapsed * 1.6).sin() + near * 0.5;
        let spin = Mat4::from_translation(self.seal_centre)
            * Mat4::from_rotation_y(elapsed * 0.06)
            * Mat4::from_translation(-self.seal_centre);
        for part in &self.seal {
            let k = pulse.min(1.25);
            let mut material =
                Material3D::additive([0.77 * k * 1.6, 0.49 * k * 1.6, 1.0 * k * 1.6, 1.0]);
            material.texture = None;
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: spin * part.transform,
                material,
                texture: part.texture,
            });
        }
        for part in &self.seal_glow {
            let a = 0.18 + 0.12 * (elapsed * 1.6).sin() + near * 0.2;
            draws.push(MeshDraw3D {
                mesh: part.mesh,
                transform: part.transform,
                material: Material3D::additive([0.63, 0.31, 1.0, a]),
                texture: part.texture,
            });
        }
        // The books turning over the desk (once the library is in order again).
        if let Some(c) = p.books.as_ref().filter(|_| state.life.library) {
            let n = self.books.len().max(1) as f32;
            for (i, part) in self.books.iter().enumerate() {
                let a = elapsed * 0.35 + i as f32 / n * std::f32::consts::TAU;
                let at = Vec3::new(
                    c.x + a.cos() * 1.1,
                    2.1 + (elapsed * 1.3 + i as f32).sin() * 0.18,
                    c.z + a.sin() * 0.8,
                );
                let turn = Mat4::from_rotation_y(-a)
                    * Mat4::from_rotation_x(0.3 * (elapsed + i as f32).sin())
                    * Mat4::from_rotation_z(0.2 * (elapsed * 0.7 + i as f32).cos());
                draws.push(MeshDraw3D {
                    mesh: part.mesh,
                    transform: Mat4::from_translation(at) * turn * part.transform,
                    material: part.material.with_emission(0.25),
                    texture: part.texture,
                });
            }
        }
        self.draw_flames(state, elapsed, toward, view, draws);
    }

    /// The flames on the floor in view: torches and candles as a bright tongue and a halo turned to
    /// the camera, the great fires as fire.
    fn draw_flames(
        &self,
        state: &State,
        elapsed: f32,
        toward: Vec3,
        view: u8,
        draws: &mut Vec<MeshDraw3D>,
    ) {
        let p = plan();
        let toward = Vec3::new(toward.x, 0.0, toward.z).normalize_or_zero();
        let face = Mat4::from_rotation_y(toward.x.atan2(toward.z));
        for (i, f) in p.flames.iter().enumerate() {
            if f.kind == "hearth" || f.kind == "brazier" || f.level != view {
                continue;
            }
            if f.wall >= 0 && !state.wall_dressed(f.wall as usize) {
                continue;
            }
            let c = colour(&f.color);
            let k = 1.0
                + 0.12 * (elapsed * 9.0 + i as f32 * 1.3).sin()
                + 0.07 * (elapsed * 23.0 + i as f32).sin();
            let s = f.size;
            let base = Vec3::new(f.x, f.y + 0.1 * s, f.z);
            draws.push(MeshDraw3D {
                mesh: self.flame,
                transform: Mat4::from_translation(base)
                    * Mat4::from_scale(Vec3::new(
                        0.05 * s * (2.0 - k),
                        0.13 * s * k,
                        0.05 * s * (2.0 - k),
                    )),
                material: Material3D::rgb(1.0, 0.82 + 0.1 * c[1], 0.5 * c[2] + 0.3)
                    .with_emission(2.4),
                texture: None,
            });
            let size = 0.9 * s;
            draws.push(MeshDraw3D {
                mesh: self.halo,
                transform: Mat4::from_translation(base + toward * 0.05)
                    * face
                    * Mat4::from_scale(Vec3::new(size, size, 1.0)),
                material: Material3D::additive([c[0] * 1.6, c[1] * 1.6, c[2] * 1.6, 0.4 * k]),
                texture: Some(self.halo_texture),
            });
        }
        for (i, (level, fire)) in self.fires.iter().enumerate() {
            if *level != view {
                continue;
            }
            match &self.kitchen_fire {
                Some((k, small)) if *k == i && !state.life.kitchen => {
                    small.draws(elapsed, toward, draws)
                }
                _ => fire.draws(elapsed, toward, draws),
            }
        }
    }
}

// ------------------------------------------------------------------ tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spawns_are_free_and_in_their_rooms() {
        let p = plan();
        let gate = State::at_gate();
        assert!(!p.blocked(gate.x, gate.z, RADIUS, 0));
        assert_eq!(p.rooms[gate.room].id, "patio");
        let bed = State::in_bedroom();
        assert!(!p.blocked(bed.x, bed.z, RADIUS, 0));
        assert_eq!(p.rooms[bed.room].id, "dormitorio");
        // A spot in a wall puts her back in the bedroom.
        let lost = State::restore(Vec3::new(10.0, 0.0, 0.0));
        assert_eq!(p.rooms[lost.room].id, "dormitorio");
        // Saved upstairs, she comes back upstairs.
        let up = State::restore(Vec3::new(1.6, p.level_h, 15.0));
        assert_eq!((up.level, p.rooms[up.room].id.as_str()), (1, "galeria"));
        assert!((up.position().y - p.level_h).abs() < 1e-4);
    }

    #[test]
    fn the_doctors_round_stays_in_the_infirmary_and_clear_of_its_furniture() {
        let p = plan();
        let room = p
            .rooms
            .iter()
            .find(|r| r.id == "enfermeria")
            .expect("the infirmary");
        let total = doctor_round_secs();
        let mut t = 0.0;
        while t < total {
            let d = doctor_at(t);
            assert!(
                room.contains(d.at.x, d.at.z),
                "out of the infirmary at {t:.1}"
            );
            assert!(
                !p.blocked(d.at.x, d.at.z, DOCTOR_RADIUS - 0.15, 1),
                "blocked at {:.1},{:.1}",
                d.at.x,
                d.at.z
            );
            t += 0.1;
        }
        assert!(DOCTOR_CLIPS.len() == 5 && total > 30.0);
    }

    #[test]
    fn the_maids_rounds_are_clear() {
        let p = plan();
        for round in ROUNDS {
            let len = round_length(round);
            let mut along = 0.0;
            while along < len {
                let (at, _) = on_round(round, along);
                assert!(
                    !p.blocked(at.x, at.z, MAID_RADIUS - 0.1, 0),
                    "round blocked at {:.1},{:.1}",
                    at.x,
                    at.z
                );
                along += 0.2;
            }
        }
    }

    #[test]
    fn every_room_and_hotspot_can_be_walked_to_from_the_gate() {
        // A flood over both floors, the way she walks (up and down the stair too).
        let p = plan();
        let [x0, x1, z0, z1] = p.bounds();
        let step = 0.25;
        let nx = ((x1 - x0) / step) as usize + 1;
        let nz = ((z1 - z0) / step) as usize + 1;
        // a layer per floor, and one per floor for the stair (entered from below or above)
        let mut seen = vec![vec![false; nx * nz]; 4];
        let layer = |w: Walker| w.level as usize + if w.on_stair { 2 } else { 0 };
        let cell = |x: f32, z: f32| {
            (
                ((x - x0) / step).round() as i32,
                ((z - z0) / step).round() as i32,
            )
        };
        let snap = |i: i32, j: i32| (x0 + i as f32 * step, z0 + j as f32 * step);
        let (si, sj) = cell(p.spawn.x, p.spawn.z);
        let (sx, sz) = snap(si, sj);
        let start = Walker {
            x: sx,
            z: sz,
            level: 0,
            on_stair: false,
        };
        seen[0][sj as usize * nx + si as usize] = true;
        let mut queue = vec![start];
        while let Some(w) = queue.pop() {
            for (dx, dz) in [(step, 0.0), (-step, 0.0), (0.0, step), (0.0, -step)] {
                let Some(n) = p.step(w, dx, dz) else { continue };
                let (i, j) = cell(n.x, n.z);
                if i < 0
                    || j < 0
                    || i as usize >= nx
                    || j as usize >= nz
                    || seen[layer(n)][j as usize * nx + i as usize]
                {
                    continue;
                }
                seen[layer(n)][j as usize * nx + i as usize] = true;
                let (x, z) = snap(i, j);
                queue.push(Walker { x, z, ..n });
            }
        }
        let reach = |x: f32, z: f32, r: f32, level: u8| {
            let s = &seen[level as usize];
            (0..nz).any(|j| {
                (0..nx).any(|i| {
                    s[j * nx + i] && (x0 + i as f32 * step - x).hypot(z0 + j as f32 * step - z) < r
                })
            })
        };
        for room in &p.rooms {
            let (cx, cz) = room.centre();
            assert!(reach(cx, cz, 6.0, room.level), "room {}", room.id);
        }
        for h in &p.hotspots {
            if h.id != "lira" && h.id != "nemi" {
                assert!(reach(h.x, h.z, h.radius * 0.9, h.level), "hotspot {}", h.id);
            }
        }
        assert!(
            p.rooms.iter().any(|r| r.level == 1),
            "the upper floor is in the plan"
        );
    }

    #[test]
    fn she_climbs_the_stair_to_the_gallery_and_comes_back_down() {
        let p = plan();
        let st = p.stair.as_ref().expect("the hall's stair");
        let lane = (st.x0 + st.x1) / 2.0 - 0.1;
        // From the hall, at the stair's foot, straight up.
        let mut s = State::at(lane, st.z_bottom + 0.8);
        assert_eq!(p.rooms[s.room].id, "vestibulo");
        head_for(&mut s, (lane, st.z_top - 1.2), 12.0);
        assert_eq!(
            (s.level, s.on_stair),
            (1, false),
            "at {:.2},{:.2}",
            s.x,
            s.z
        );
        assert_eq!(p.rooms[s.room].id, "galeria");
        assert!((s.position().y - p.level_h).abs() < 1e-4);
        // Along the gallery and back to the top of the stair, then down to the hall.
        head_for(&mut s, (2.0, 10.4), 8.0);
        assert!(
            (s.x - 2.0).hypot(s.z - 10.4) < 0.3,
            "stuck upstairs at {:.2},{:.2}",
            s.x,
            s.z
        );
        head_for(&mut s, (lane, st.z_top - 1.0), 8.0);
        head_for(&mut s, (lane, st.z_bottom + 0.8), 12.0);
        assert_eq!(
            (s.level, s.on_stair),
            (0, false),
            "at {:.2},{:.2}",
            s.x,
            s.z
        );
        assert_eq!(p.rooms[s.room].id, "vestibulo");
        assert!(s.position().y.abs() < 1e-4);
        // Halfway up, she is half a floor up and the hall is still the room in view.
        let mut mid = State::at(lane, st.z_bottom + 0.8);
        head_for(&mut mid, (lane, (st.z_top + st.z_bottom) / 2.0), 6.0);
        assert!(mid.on_stair);
        assert!((mid.position().y - p.level_h * 0.5).abs() < 0.3);
        assert_eq!(mid.view_level(), 0);
        // The stair's side and the balustrade stop her; under it is solid.
        assert!(p.blocked(st.x1 + 0.2, (st.z_top + st.z_bottom) / 2.0, RADIUS, 0));
        assert!(p.blocked(st.x1 + 0.2, (st.z_top + st.z_bottom) / 2.0, RADIUS, 1));
    }

    #[test]
    fn the_upper_floor_keeps_its_own_walls_and_lights() {
        let p = plan();
        let gallery = p.rooms.iter().find(|r| r.id == "galeria").unwrap();
        let yaw = std::f32::consts::FRAC_PI_4;
        // Up there, only the gallery's own walls drop; the hall's below stay.
        assert!(p
            .walls
            .iter()
            .filter(|w| p.wall_faces_camera(w, gallery, yaw))
            .all(|w| w.level == 1));
        let mut up = State::restore(Vec3::new(1.6, p.level_h, 15.0));
        up.update(1.0, yaw, 0.0);
        assert!(!up.lights(0.0).is_empty());
        assert!(up
            .lights(0.0)
            .iter()
            .all(|l| l.position.y > p.level_h * 0.5));
        let mut down = State::at_gate();
        down.update(1.0, yaw, 0.0);
        assert!(down.lights(0.0).iter().all(|l| l.position.y < p.level_h));
    }

    /// Heads straight for `goal` as a player would (60 steps a second); where she ends up.
    fn head_for(state: &mut State, goal: (f32, f32), seconds: f32) -> (f32, f32) {
        for _ in 0..(seconds * 60.0) as usize {
            let (dx, dz) = (goal.0 - state.x, goal.1 - state.z);
            let d = dx.hypot(dz);
            if d < 0.05 {
                break;
            }
            let step = 2.2 / 60.0;
            state.walk(dx / d * step, dz / d * step);
        }
        (state.x, state.z)
    }

    #[test]
    fn she_slips_through_a_doorway_taken_at_a_slant() {
        // Bedroom to throne room, aimed past the door's jamb from every side.
        for start in [
            (-12.5, -15.2),
            (-12.5, -18.8),
            (-11.2, -15.9),
            (-14.0, -14.0),
        ] {
            let mut s = State::at(start.0, start.1);
            let (x, z) = head_for(&mut s, (-8.0, -17.0), 8.0);
            assert!(
                (x + 8.0).hypot(z + 17.0) < 0.3,
                "from {start:?} stuck at {x:.2},{z:.2}"
            );
        }
        // Pressed flat against the jamb, walking along the wall's line.
        let mut s = State::at(-10.72, -16.0);
        head_for(&mut s, (-9.25, -16.0), 3.0);
        assert!(s.x > -10.6, "stuck on the jamb at {:.2},{:.2}", s.x, s.z);
    }

    #[test]
    fn walls_drop_only_on_the_camera_side() {
        let p = plan();
        let hall = p.rooms.iter().find(|r| r.id == "salon").unwrap();
        let yaw = std::f32::consts::FRAC_PI_4; // the camera south-east of her
        let down: Vec<&Wall> = p
            .walls
            .iter()
            .filter(|w| p.wall_faces_camera(w, hall, yaw))
            .collect();
        assert!(!down.is_empty());
        assert!(down
            .iter()
            .all(|w| (w.axis == "z" && w.at == hall.z1) || (w.axis == "x" && w.at == hall.x1)));
    }
}
