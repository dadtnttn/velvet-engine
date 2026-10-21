//! La Mina 2.0: the hand-made mine, modelled in its own project (demon-queen-mine-threejs) and put
//! into the game by its export (art/tools/export_game.py) as `data/mina/mina.json` plus its meshes,
//! textures and floor map under `data/assets/3d/mina/`.
//!
//! The run is the procedural mine's own: the same actors, combat, chests, runes, rest, Custodio,
//! Heart and way out, fed with a blueprint built from the file instead of the generator. The floor
//! map (velvet-dungeon's `FloorGrid`) says where one can stand; rooms only name the places and
//! group what is in them. Its seed is [`MINE_SEED`], so a save of it comes back as the mine.
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use serde::Deserialize;
use velvet_dungeon::{
    gameplay::{Spawn, SpawnKind},
    Blueprint, Config, FloorGrid, Role, Room, Shape, Stats, Treasury, GENERATOR_VERSION,
};
use velvet_math::Vec3;

use super::slime::{Slime, Tint};

/// The hand-made mine's seed: it marks a run (and a save) as this mine, not a generated one.
pub const MINE_SEED: u64 = 0x4D49_4E41_3230_0001;

#[derive(Deserialize)]
struct File {
    version: u32,
    files: Files,
    spawn: SpawnPoint,
    rooms: Vec<RoomData>,
    spawns: Vec<SpawnData>,
    #[serde(default)]
    gates: Vec<GateData>,
    #[serde(default)]
    lights: Vec<LightData>,
    #[serde(default)]
    npcs: Vec<NpcData>,
    #[serde(default)]
    waters: Vec<WaterData>,
    #[serde(default)]
    lake: Option<WaterData>,
    #[serde(default)]
    heart: Option<HeartData>,
}

/// The Heart: the middle of its body and the floor under it.
#[derive(Deserialize)]
struct HeartData {
    center: [f32; 3],
    floor: [f32; 3],
}

/// Standing water: an ellipse (`c`, `rx`, `ry` on the plan) or a rectangle (`rect`: x0, z0, x1, z1).
#[derive(Deserialize)]
struct WaterData {
    #[serde(default)]
    c: Option<[f32; 2]>,
    #[serde(default)]
    rx: f32,
    #[serde(default)]
    ry: f32,
    #[serde(default)]
    rect: Option<[f32; 4]>,
    level: f32,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    glow: Option<String>,
}

#[derive(Deserialize)]
struct Files {
    rock: String,
    props: String,
    floors: String,
}

#[derive(Deserialize)]
struct SpawnPoint {
    p: [f32; 3],
    #[serde(default)]
    yaw: f32,
}

#[derive(Deserialize)]
struct RoomData {
    id: String,
    name: String,
    role: String,
    #[serde(default)]
    biome: usize,
    center: [f32; 3],
    #[serde(rename = "box")]
    middle: [f32; 3],
    half: [f32; 2],
    #[serde(default)]
    height: f32,
}

#[derive(Deserialize)]
struct SpawnData {
    kind: String,
    room: usize,
    p: [f32; 3],
    #[serde(default)]
    face: [f32; 2],
    #[serde(default)]
    definition: String,
    #[serde(default)]
    health: f32,
    #[serde(default)]
    damage: f32,
    #[serde(default)]
    speed: f32,
    #[serde(default)]
    gold: u32,
    #[serde(default)]
    loot: Option<Loot>,
    /// A champion's name, height in metres, power and own reward.
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    size: Option<f32>,
    #[serde(default)]
    power: Option<String>,
    #[serde(default)]
    reward: Option<Loot>,
}

/// What a named champion does besides its blows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    /// Embers rain on her and round her now and then.
    Sparks,
    /// A volley of violet orbs at her now and then.
    Volley,
    /// At two thirds and a third of its life it raises the dead round it.
    Raise,
    /// At half its life it rages and calls two of its own.
    Fury,
    None,
}

/// A foe of the mine with a name of its own (the HUD's bar shows it), its height, its power and
/// what it leaves when it falls besides the usual.
#[derive(Clone, Debug, PartialEq)]
pub struct Champion {
    pub name: String,
    /// Its height in metres.
    pub size: f32,
    pub power: Power,
    pub reward: Loot,
}

/// The model a champion of the mine wears (its own build and gear), by what it is: the named ones by
/// their power, the Custodio of the seal (the mine's only other miniboss) and the Heart's guardian.
pub fn champion_model(mine: Option<&Authored>, spawn: &Spawn) -> Option<&'static str> {
    let mine = mine?;
    if let Some(champion) = mine.champions.get(&spawn.id) {
        return Some(match champion.power {
            Power::Sparks => "herrero",
            Power::Volley => "bibliotecaria",
            Power::Raise => "senor",
            Power::Fury => "centinela",
            Power::None => return None,
        });
    }
    match spawn.kind {
        SpawnKind::Miniboss => Some("custodio"),
        SpawnKind::Boss => Some("guardian"),
        _ => None,
    }
}

/// What a champion says: on seeing her, when it rages, now and then while they fight, and as it falls.
pub struct Lines {
    pub wake: &'static str,
    pub rage: &'static str,
    pub taunts: &'static [&'static str],
    pub fall: &'static str,
}

/// Each champion's words (by its model).
pub fn champion_lines(model: &str) -> Option<&'static Lines> {
    static HERRERO: Lines = Lines {
        wake: "¿Una reina en mi forja? Hace tres siglos que no templo un acero tan fino.",
        rage: "¡Más fuego! ¡Que el yunque cante!",
        taunts: &[
            "Quieta, majestad. Solo voy a darte forma.",
            "Te enfrías demasiado rápido.",
            "Cada golpe, una chispa. Cada chispa, un recuerdo.",
        ],
        fall: "Por fin... se apaga la fragua.",
    };
    static BIBLIOTECARIA: Lines = Lines {
        wake: "Silencio. En esta sala no se habla desde hace trescientos años.",
        rage: "¿Has arrancado una página? ¡Eso se paga con huesos!",
        taunts: &[
            "Baja la voz, majestad.",
            "Tu nombre no figura en mis registros. Todavía.",
            "Te archivaré con los olvidados.",
        ],
        fall: "Cerrad el libro... con cuidado.",
    };
    static SENOR: Lines = Lines {
        wake: "Arrodíllate. En esta cripta solo hay un rey.",
        rage: "¡Levantaos, mis fieles! ¡Vuestro señor os llama!",
        taunts: &[
            "Una reina sin corte no es más que una visitante.",
            "Mi corte entera murió por mí. ¿Quién moriría por ti?",
            "Inclina la cabeza. Así te la quito antes.",
        ],
        fall: "Ni la muerte... me quitó la corona.",
    };
    static CENTINELA: Lines = Lines {
        wake: "Ni una moneda sale de aquí. Ni una.",
        rage: "¡Ladrona! ¡LADRONA!",
        taunts: &[
            "Te he contado los pasos. Y las monedas.",
            "El oro no se mueve. Tú tampoco.",
            "¿Sabes cuántos lo intentaron? Están debajo del oro.",
        ],
        fall: "El oro... era lo único... que me quedaba.",
    };
    static CUSTODIO: Lines = Lines {
        wake: "El sello no se abre. Lo juré hace mil años, y lo cumplo.",
        rage: "La llave es mía. ¡MÍA!",
        taunts: &[
            "Detrás de esa puerta late algo que no debes despertar.",
            "Mi juramento pesa más que tu corona.",
            "Retrocede, reina. Por tu bien.",
        ],
        fall: "Que el Corazón... te perdone.",
    };
    static GUARDIAN: Lines = Lines {
        wake: "Late... ¿lo oyes? Late por mí.",
        rage: "¡SANGRE PARA EL CORAZÓN!",
        taunts: &[
            "Cada latido me hace más grande.",
            "Tu sangre suena igual que la suya.",
            "Quédate. El Corazón tiene hambre.",
        ],
        fall: "El latido... se... calla...",
    };
    Some(match model {
        "herrero" => &HERRERO,
        "bibliotecaria" => &BIBLIOTECARIA,
        "senor" => &SENOR,
        "centinela" => &CENTINELA,
        "custodio" => &CUSTODIO,
        "guardian" => &GUARDIAN,
        _ => return None,
    })
}

/// A champion's colour: its eyes, the light about it, its name when it speaks.
pub fn champion_color(model: &str) -> [f32; 3] {
    match model {
        "herrero" => [1.0, 0.5, 0.12],
        "bibliotecaria" => [0.78, 0.5, 1.0],
        "senor" => [0.45, 1.0, 0.45],
        "centinela" => [1.0, 0.82, 0.3],
        "custodio" => [1.0, 0.22, 0.14],
        "guardian" => [1.0, 0.1, 0.12],
        _ => [1.0, 0.9, 0.8],
    }
}

/// How big a foe stands: a champion's own height, else its kind's.
pub fn foe_size(mine: Option<&Authored>, spawn: &Spawn) -> f32 {
    mine.and_then(|m| m.champions.get(&spawn.id))
        .map(|c| c.size / 1.95)
        .unwrap_or_else(|| super::combat::body_size(spawn.kind))
}

/// What a chest of the mine holds (rolled by the export: the same every expedition).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Loot {
    #[serde(default)]
    pub gold: u32,
    #[serde(default)]
    pub potions: u32,
    #[serde(default)]
    pub herbs: u32,
    #[serde(default)]
    pub materials: BTreeMap<String, u32>,
    /// The stone sword lies in this one.
    #[serde(default)]
    pub relic: bool,
}

#[derive(Deserialize)]
struct GateData {
    id: String,
    p: [f32; 3],
    half: [f32; 2],
    height: f32,
}

#[derive(Deserialize)]
struct LightData {
    p: [f32; 3],
    #[serde(default)]
    kind: String,
    color: String,
    intensity: f32,
    distance: f32,
}

#[derive(Deserialize)]
struct NpcData {
    id: String,
    kicker: String,
    name: String,
    p: [f32; 3],
    #[serde(default)]
    face: [f32; 2],
    #[serde(default)]
    lines: Vec<String>,
}

/// A way the Queen cannot take until the seal is turned and the Custodio's key is hers.
#[derive(Clone, Debug, PartialEq)]
pub struct Gate {
    pub id: String,
    /// The middle of its floor.
    pub center: Vec3,
    /// Half its size on the plan (x, z).
    pub half: [f32; 2],
    pub height: f32,
}

impl Gate {
    /// Whether a body of `radius` at `p` stands in it.
    pub fn holds(&self, p: Vec3, radius: f32) -> bool {
        (p.x - self.center.x).abs() < self.half[0] + radius
            && (p.z - self.center.z).abs() < self.half[1] + radius
            && p.y > self.center.y - 1.0
            && p.y < self.center.y + self.height
    }
}

/// A lamp of the mine.
#[derive(Clone, Debug, PartialEq)]
pub struct Lamp {
    pub position: Vec3,
    /// What it is: a hanging lantern, a candle, a crystal's glow, the room's fill…
    pub kind: String,
    /// Linear colour.
    pub color: [f32; 3],
    pub intensity: f32,
    pub distance: f32,
}

/// Standing water of the mine: its shore and how it looks.
#[derive(Clone, Debug, PartialEq)]
pub struct Pool {
    pub center: Vec3,
    pub outline: Vec<Vec3>,
    /// Its colour in the deep and what it gives back of the light.
    pub color: [f32; 3],
    pub reflect: [f32; 3],
}

/// A square metre of the mine's floor on its chart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartCell {
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub room: usize,
}

/// Someone standing in the mine (for now, a placeholder figure with a name over the head).
#[derive(Clone, Debug, PartialEq)]
pub struct Npc {
    pub id: String,
    pub kicker: String,
    pub name: String,
    pub position: Vec3,
    pub face: Vec3,
    /// What they say, one line each time she talks to them.
    pub lines: Vec<String>,
}

/// What the hand-made mine adds to a run: its art, the names of its places, how each spawn
/// faces, the chests' contents, its gates, lamps and people.
#[derive(Debug, PartialEq)]
pub struct Authored {
    /// The rock and the props, relative to the 3D assets' folder.
    pub rock: String,
    pub props: String,
    /// Each room's name, as the HUD shows it.
    pub names: Vec<String>,
    /// Which way each spawn faces (flat, unit), by spawn id.
    pub faces: Vec<Vec3>,
    /// The chests' contents, by spawn id.
    pub loot: BTreeMap<usize, Loot>,
    pub gates: Vec<Gate>,
    pub lamps: Vec<Lamp>,
    pub npcs: Vec<Npc>,
    pub pools: Vec<Pool>,
    /// The foes with names of their own, by spawn id.
    pub champions: BTreeMap<usize, Champion>,
    /// The Heart's middle (it beats round it) and the floor under it (its chamber breathes round it).
    pub heart: Option<(Vec3, Vec3)>,
    /// The floor, metre by metre, for the map: every floor with room over it and a room.
    pub chart: Vec<ChartCell>,
    /// Where the Queen comes in, and which way she looks.
    pub spawn: Vec3,
    pub spawn_yaw: f32,
}

impl Authored {
    /// Which way a spawn faces.
    pub fn face(&self, spawn: usize) -> Option<Vec3> {
        self.faces
            .get(spawn)
            .copied()
            .filter(|f| f.length_squared() > 0.5)
    }
}

/// The game's data folder (`data/`): beside the 3D assets' folder.
fn data_root() -> PathBuf {
    super::assets::root().join("..").join("..")
}

/// Whether the mine's file is there to be played.
pub fn available() -> bool {
    data_root().join("mina/mina.json").is_file()
}

fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

fn srgb(hex: &str) -> [f32; 3] {
    let hex = hex.trim_start_matches('#');
    let channel = |i: usize| {
        let v =
            u8::from_str_radix(hex.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255) as f32 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    [channel(0), channel(2), channel(4)]
}

/// A pool's shore, and its look from what the file says of its water.
fn pool(w: &WaterData, lake: bool) -> Option<Pool> {
    let level = w.level + 0.01;
    let (center, outline) = if let Some([x0, z0, x1, z1]) = w.rect {
        let (x0, x1, z0, z1) = (x0.min(x1), x0.max(x1), z0.min(z1), z0.max(z1));
        let mut outline = Vec::new();
        let per = 8;
        for (ax, az, bx, bz) in [
            (x0, z0, x1, z0),
            (x1, z0, x1, z1),
            (x1, z1, x0, z1),
            (x0, z1, x0, z0),
        ] {
            for k in 0..per {
                let t = k as f32 / per as f32;
                outline.push(Vec3::new(ax + (bx - ax) * t, level, az + (bz - az) * t));
            }
        }
        (Vec3::new((x0 + x1) * 0.5, level, (z0 + z1) * 0.5), outline)
    } else {
        let c = w.c?;
        let outline = (0..32)
            .map(|k| {
                let a = k as f32 / 32.0 * std::f32::consts::TAU;
                Vec3::new(c[0] + a.cos() * w.rx, level, c[1] + a.sin() * w.ry)
            })
            .collect();
        (Vec3::new(c[0], level, c[1]), outline)
    };
    let glow = w.glow.as_deref().map(srgb);
    let (color, reflect) = match glow {
        // the Heart's blood
        Some(g) if g[0] > g[2] => ([0.11, 0.004, 0.01], [0.42, 0.03, 0.05]),
        // the slimes' water, lit blue from below
        Some(_) => ([0.02, 0.05, 0.12], [0.06, 0.15, 0.36]),
        None if lake => ([0.02, 0.016, 0.05], [0.05, 0.035, 0.11]),
        None if w.rect.is_some() => ([0.042, 0.049, 0.053], [0.17, 0.14, 0.10]),
        None if w
            .color
            .as_deref()
            .map(srgb)
            .is_some_and(|c| c[1] > c[2] * 1.2) =>
        {
            ([0.024, 0.050, 0.035], [0.08, 0.17, 0.13])
        }
        None => ([0.035, 0.045, 0.052], [0.13, 0.14, 0.16]),
    };
    Some(Pool {
        center,
        outline,
        color,
        reflect,
    })
}

fn role(name: &str) -> Role {
    match name {
        "entrance" => Role::Entrance,
        "exit" => Role::Exit,
        "encounter" => Role::Encounter,
        "treasure" => Role::Treasure,
        "secret" => Role::Secret,
        "puzzle" => Role::Puzzle,
        "event" => Role::Event,
        "rest" => Role::Rest,
        "elite" => Role::Elite,
        "miniboss" => Role::Miniboss,
        "boss" => Role::Boss,
        _ => Role::Normal,
    }
}

fn kind(name: &str) -> Option<SpawnKind> {
    Some(match name {
        "enemy" => SpawnKind::Enemy,
        "elite" => SpawnKind::Elite,
        "miniboss" => SpawnKind::Miniboss,
        "boss" => SpawnKind::Boss,
        "chest" => SpawnKind::Chest,
        "seal" => SpawnKind::Seal,
        "rest" => SpawnKind::Rest,
        "memory" => SpawnKind::Memory,
        "exit" => SpawnKind::Exit,
        "entrance" => SpawnKind::Entrance,
        "trap" => SpawnKind::Trap,
        _ => return None,
    })
}

/// The chart's squares one can walk to from `start`: neighbours a metre or two apart (over a
/// narrow crack) and at most a metre and a bit higher or lower.
fn joined(cells: Vec<ChartCell>, start: Vec3) -> Vec<ChartCell> {
    use std::collections::{HashMap, VecDeque};
    let key = |x: f32, z: f32| ((x.floor()) as i32, (z.floor()) as i32);
    let mut at: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, c) in cells.iter().enumerate() {
        at.entry(key(c.x, c.z)).or_default().push(i);
    }
    let Some(first) = at.get(&key(start.x, start.z)).and_then(|list| {
        list.iter().copied().min_by(|&a, &b| {
            (cells[a].y - start.y)
                .abs()
                .total_cmp(&(cells[b].y - start.y).abs())
        })
    }) else {
        return cells;
    };
    let mut keep = vec![false; cells.len()];
    keep[first] = true;
    let mut queue = VecDeque::from([first]);
    while let Some(i) = queue.pop_front() {
        let (cx, cz) = key(cells[i].x, cells[i].z);
        // the near squares, the diagonal ones, and over a missing one (a crack narrower than a metre)
        for (dx, dz) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
            (2, 0),
            (-2, 0),
            (0, 2),
            (0, -2),
        ] {
            for &j in at.get(&(cx + dx, cz + dz)).into_iter().flatten() {
                if !keep[j] && (cells[j].y - cells[i].y).abs() < 1.2 {
                    keep[j] = true;
                    queue.push_back(j);
                }
            }
        }
    }
    cells
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(c, _)| c)
        .collect()
}

/// Reads the mine: its blueprint (with the floor map), what it adds to a run, and its slimes.
pub fn load() -> Result<(Blueprint, Authored, Vec<Slime>), String> {
    let root = data_root();
    let text = std::fs::read_to_string(root.join("mina/mina.json"))
        .map_err(|e| format!("mina.json: {e}"))?;
    let file: File = serde_json::from_str(&text).map_err(|e| format!("mina.json: {e}"))?;
    if file.version != 1 {
        return Err(format!("mina.json: unknown version {}", file.version));
    }
    let floors = std::fs::read(super::assets::root().join(&file.files.floors))
        .map_err(|e| format!("{}: {e}", file.files.floors))?;
    let floors = FloorGrid::from_bytes(&floors)?;
    let mut chart = Vec::new();
    let (w, h) = (
        floors.width as f32 * floors.cell,
        floors.height as f32 * floors.cell,
    );
    let mut z = floors.z0 + 0.5;
    while z < floors.z0 + h {
        let mut x = floors.x0 + 0.5;
        while x < floors.x0 + w {
            for (y, span) in floors.floors(x, z) {
                if let Some(room) = span.room.filter(|_| span.ceiling - span.floor >= 1.6) {
                    chart.push(ChartCell {
                        x,
                        z,
                        y,
                        room: usize::from(room),
                    });
                }
            }
            x += 1.0;
        }
        z += 1.0;
    }
    // Only the floor joined to the way in: the hill's slopes and the tops of things stay off the chart.
    let chart = joined(chart, v3(file.spawn.p));
    if file.rooms.is_empty() {
        return Err("mina.json: no rooms".into());
    }
    let last = file.rooms.len() - 1;
    let rooms: Vec<Room> = file
        .rooms
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let center = v3(r.center);
            let middle = v3(r.middle);
            let (dx, dz) = (middle.x - center.x, middle.z - center.z);
            let (hx, hz) = (r.half[0], r.half[1]);
            let role = role(&r.role);
            Room {
                id: i,
                role,
                critical: true,
                progress: i as f32 / last.max(1) as f32,
                region: i,
                grid: [0; 3],
                center,
                half: r.half,
                height: if r.height > 0.0 { r.height } else { 6.0 },
                shape: Shape::Cavern,
                template: format!("mina:{}", r.id),
                biome: r.biome.min(2),
                outline: vec![
                    [dx - hx, dz - hz],
                    [dx + hx, dz - hz],
                    [dx + hx, dz + hz],
                    [dx - hx, dz + hz],
                ],
                sockets: vec![],
                landmark_seed: MINE_SEED ^ i as u64,
                treasury: (role == Role::Treasure).then_some(Treasury::Grand),
            }
        })
        .collect();
    let entrance = rooms
        .iter()
        .position(|r| r.role == Role::Entrance)
        .unwrap_or(0);
    let exit = rooms
        .iter()
        .position(|r| r.role == Role::Boss)
        .unwrap_or(last);
    let mut spawns = Vec::new();
    let mut faces = Vec::new();
    let mut loot = BTreeMap::new();
    let mut slimes = Vec::new();
    let mut champions = BTreeMap::new();
    for (n, s) in file.spawns.iter().enumerate() {
        let position = v3(s.p);
        let room = s.room.min(last);
        if s.kind == "slime" {
            let tint = if s.definition == "green" {
                Tint::Green
            } else {
                Tint::Blue
            };
            slimes.push(Slime::at(
                room,
                position,
                MINE_SEED ^ (n as u64).wrapping_mul(0x9E37_79B9),
                tint,
            ));
            continue;
        }
        let Some(kind) = kind(&s.kind) else {
            return Err(format!("mina.json: unknown spawn kind {}", s.kind));
        };
        let id = spawns.len();
        if let Some(l) = &s.loot {
            loot.insert(id, l.clone());
        }
        if let Some(name) = &s.name {
            champions.insert(
                id,
                Champion {
                    name: name.clone(),
                    size: s.size.unwrap_or(2.4),
                    power: match s.power.as_deref() {
                        Some("chispas") => Power::Sparks,
                        Some("andanada") => Power::Volley,
                        Some("alzar") => Power::Raise,
                        Some("furia") => Power::Fury,
                        _ => Power::None,
                    },
                    reward: s.reward.clone().unwrap_or_default(),
                },
            );
        }
        faces.push(Vec3::new(s.face[0], 0.0, s.face[1]));
        spawns.push(Spawn {
            id,
            room,
            kind,
            definition: s.definition.clone(),
            position,
            health: s.health,
            damage: s.damage,
            speed: s.speed,
            gold: s.gold,
            budget: 1.0 + rooms[room].progress * 2.2,
        });
    }
    let map = Blueprint {
        seed: MINE_SEED,
        version: GENERATOR_VERSION,
        rules_hash: velvet_dungeon::rng::hash(text.as_bytes()),
        config: Config::default(),
        rooms,
        connections: vec![],
        spawns,
        obstacles: vec![],
        critical_path: (0..file.rooms.len()).collect(),
        entrance,
        exit,
        stats: Stats {
            generation_ms: 0.0,
            rooms: file.rooms.len(),
            critical_length: file.rooms.len(),
            branches: 0,
            loops: 0,
            vertical_connections: 0,
            stacked_rooms: 0,
            volume: 0.0,
            rejected_attempts: 0,
            attempts: 1,
            local_repairs: 0,
            score: 1.0,
        },
        floors: Some(Arc::new(floors)),
    };
    let authored = Authored {
        rock: file.files.rock,
        props: file.files.props,
        names: file.rooms.iter().map(|r| r.name.clone()).collect(),
        faces,
        loot,
        gates: file
            .gates
            .iter()
            .map(|g| Gate {
                id: g.id.clone(),
                center: v3(g.p),
                half: g.half,
                height: g.height,
            })
            .collect(),
        lamps: file
            .lights
            .iter()
            .map(|l| Lamp {
                position: v3(l.p),
                kind: l.kind.clone(),
                color: srgb(&l.color),
                intensity: l.intensity,
                distance: l.distance,
            })
            .collect(),
        npcs: file
            .npcs
            .iter()
            .map(|n| Npc {
                id: n.id.clone(),
                kicker: n.kicker.clone(),
                name: n.name.clone(),
                position: v3(n.p),
                face: Vec3::new(n.face[0], 0.0, n.face[1]),
                lines: n.lines.clone(),
            })
            .collect(),
        pools: file
            .waters
            .iter()
            .filter_map(|w| pool(w, false))
            .chain(file.lake.as_ref().and_then(|w| pool(w, true)))
            .collect(),
        chart,
        champions,
        heart: file.heart.as_ref().map(|h| (v3(h.center), v3(h.floor))),
        spawn: v3(file.spawn.p),
        spawn_yaw: file.spawn.yaw,
    };
    Ok((map, authored, slimes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mine_loads_with_its_expedition_when_it_is_there() {
        if !available() {
            eprintln!("no data/mina/mina.json: skipped");
            return;
        }
        let (map, mine, slimes) = load().unwrap();
        let count = |k: SpawnKind| map.spawns.iter().filter(|s| s.kind == k).count();
        assert_eq!(count(SpawnKind::Seal), 3, "three rune stones");
        let custodians = map
            .spawns
            .iter()
            .filter(|s| s.kind == SpawnKind::Miniboss && map.rooms[s.room].role == Role::Miniboss)
            .count();
        assert_eq!(custodians, 1, "the Custodio");
        assert!(mine.champions.len() >= 4, "named champions round the mine");
        assert_eq!(count(SpawnKind::Boss), 1, "the Heart's guardian");
        assert_eq!(count(SpawnKind::Entrance), 1);
        assert_eq!(count(SpawnKind::Exit), 1);
        assert!(count(SpawnKind::Chest) >= 12, "rooms with rewards");
        assert!(!slimes.is_empty());
        assert!(!mine.gates.is_empty() && !mine.lamps.is_empty());
        // everything stands on a floor of the map, with room for a body over it
        for s in &map.spawns {
            let p = s.position;
            let floor = map.capsule_floor(p + Vec3::Y * 0.05, 0.25);
            assert!(
                floor.is_some_and(|y| (y - p.y).abs() < 0.35),
                "{:?} {:?} stands on no floor",
                s.kind,
                p
            );
        }
        // the way in stands in the entrance's room, and the floors name the rooms
        assert_eq!(map.room_at(mine.spawn), Some(map.entrance));
        let chest = map
            .spawns
            .iter()
            .find(|s| s.kind == SpawnKind::Chest)
            .unwrap();
        assert!(mine.loot.contains_key(&chest.id));
        assert_eq!(map.room_at(chest.position), Some(chest.room));
    }

    /// The routes the prototype's walk test takes (tools/reach.mjs), walked by the game's own
    /// controller over the floor map: the whole mine, from the way in to the Heart.
    #[test]
    fn the_games_walker_goes_everywhere_the_prototypes_does() {
        use velvet_dungeon::controller::{Controller, Input};
        if !available() {
            return;
        }
        let (map, _, _) = load().unwrap();
        let routes: &[(&str, &[[f32; 3]])] = &[
            (
                "entrada → sello",
                &[
                    [0.0, 0.0, 15.0],
                    [1.4, 0.0, 9.0],
                    [1.4, 0.0, 4.0],
                    [0.5, 0.0, 0.0],
                    [0.0, -0.2, -3.0],
                    [0.2, -0.6, -12.0],
                    [1.35, -1.0, -16.5],
                    [1.5, -1.25, -20.5],
                    [1.5, -1.7, -25.0],
                    [0.5, -2.8, -37.0],
                    [0.0, -3.0, -46.0],
                    [2.5, -3.0, -49.0],
                    [6.5, -3.0, -49.8],
                    [10.0, -3.0, -50.6],
                    [17.0, -3.0, -50.2],
                    [24.0, -3.0, -50.0],
                    [27.0, -3.0, -49.0],
                    [30.5, -3.0, -49.0],
                    [32.2, -3.0, -50.8],
                    [41.0, -8.3, -50.8],
                    [49.6, -13.2, -50.8],
                    [52.6, -13.4, -50.6],
                    [53.5, -13.6, -56.0],
                    [52.6, -14.0, -62.0],
                    [52.0, -14.0, -68.6],
                    [52.0, -14.0, -73.0],
                    [52.0, -14.0, -77.4],
                    [53.0, -14.0, -81.0],
                    [53.4, -14.2, -86.5],
                    [53.0, -14.4, -92.5],
                    [53.0, -14.4, -99.0],
                ],
            ),
            (
                "sala → cripta → santuario",
                &[
                    [0.0, -3.0, -52.0],
                    [-4.0, -3.0, -55.0],
                    [-8.4, -3.0, -56.5],
                    [-11.4, -3.0, -57.6],
                    [-12.2, -3.0, -58.8],
                    [-14.8, -3.2, -63.6],
                    [-17.2, -3.5, -68.8],
                    [-18.5, -3.7, -73.5],
                    [-23.9, -3.7, -78.0],
                    [-23.9, -3.7, -86.0],
                    [-22.75, -3.7, -88.6],
                ],
            ),
            (
                "oeste → grieta → bóveda arcana",
                &[
                    [0.0, -3.0, -50.0],
                    [-9.0, -3.0, -53.0],
                    [-17.0, -3.0, -55.6],
                    [-18.7, -3.06, -56.0],
                    [-19.6, -3.1, -53.6],
                    [-20.6, -3.15, -51.4],
                    [-24.8, -3.3, -46.8],
                    [-28.3, -3.45, -43.6],
                    [-28.5, -3.5, -40.6],
                    [-28.5, -3.5, -38.4],
                ],
            ),
            (
                "sala → biblioteca",
                &[
                    [0.0, -3.0, -52.0],
                    [4.6, -3.0, -62.0],
                    [4.2, -3.25, -67.5],
                    [3.5, -3.6, -72.6],
                    [3.5, -3.6, -78.0],
                    [3.5, -3.6, -86.5],
                ],
            ),
            (
                "torno → forja",
                &[
                    [24.0, -3.0, -50.0],
                    [26.0, -3.0, -51.0],
                    [27.7, -3.0, -53.2],
                    [28.6, -3.0, -55.0],
                    [28.8, -3.0, -58.5],
                    [25.8, -3.0, -65.2],
                    [24.2, -3.0, -66.4],
                ],
            ),
            (
                "gruta → raíces → hongos y limos",
                &[
                    [52.6, -14.0, -62.0],
                    [49.5, -14.0, -67.5],
                    [47.5, -14.0, -69.6],
                    [40.5, -14.1, -71.0],
                    [32.0, -14.2, -70.0],
                    [27.5, -14.2, -73.6],
                    [25.5, -14.1, -77.2],
                    [27.5, -14.2, -73.6],
                    [21.5, -14.2, -73.2],
                    [18.8, -14.3, -73.4],
                ],
            ),
            (
                "gruta → tesoro",
                &[
                    [53.0, -14.0, -80.2],
                    [58.8, -14.0, -80.6],
                    [63.5, -14.0, -81.2],
                    [68.2, -14.0, -81.0],
                    [72.0, -14.0, -81.0],
                    [75.5, -14.0, -81.0],
                ],
            ),
            (
                "sello → corazón",
                &[
                    [53.0, -14.4, -99.0],
                    [53.0, -14.4, -103.4],
                    [53.0, -14.4, -107.6],
                    [53.0, -14.4, -109.6],
                    [53.0, -15.0, -112.5],
                    [53.2, -16.0, -118.6],
                    [53.0, -16.0, -121.5],
                    [53.6, -16.0, -124.6],
                ],
            ),
        ];
        let mut stuck = Vec::new();
        for (name, route) in routes {
            let first = route[0];
            let floor = map.capsule_floor(Vec3::new(first[0], first[1] + 0.3, first[2]), 0.28);
            let mut c = Controller::new(Vec3::new(first[0], floor.unwrap_or(first[1]), first[2]));
            'legs: for target in route.iter().skip(1) {
                let target = Vec3::new(target[0], target[1], target[2]);
                let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
                let mut best = f32::INFINITY;
                let mut still = 0.0;
                // Like a player, it steps round what stands in the way: blocked for a moment, it
                // sidesteps one way a while, then the other.
                let mut dodge = 0.0f32;
                let mut side = 1.0f32;
                for _ in 0..60 * 40 {
                    let delta = flat(target - c.position);
                    if delta.length() < 0.7 {
                        continue 'legs;
                    }
                    c.yaw = (-delta.x).atan2(-delta.z);
                    let strafe = if dodge > 0.0 { side } else { 0.0 };
                    dodge -= 1.0 / 60.0;
                    c.step(
                        &map,
                        Input {
                            forward: 1.0,
                            strafe,
                            ..Input::default()
                        },
                        1.0 / 60.0,
                    );
                    if delta.length() < best - 0.05 {
                        best = delta.length();
                        still = 0.0;
                    } else {
                        still += 1.0 / 60.0;
                        if still > 0.5 && dodge <= 0.0 {
                            dodge = 0.7;
                            side = -side;
                        }
                        if still > 8.0 {
                            break;
                        }
                    }
                }
                stuck.push(format!(
                    "{name}: stuck at {:?} on the way to {:?}",
                    c.position, target
                ));
                break;
            }
        }
        assert!(stuck.is_empty(), "{}", stuck.join("\n"));
    }

    /// The expedition in the hand-made mine: a foe comes for her and hits, a chest opens and gives
    /// what it holds, the Heart's throat stays shut until the seal and the key are hers.
    #[test]
    fn the_mines_expedition_plays() {
        use crate::Dungeon;
        use velvet_dungeon::controller::Input;
        if !available() {
            return;
        }
        let mut run = Dungeon::mine().unwrap();
        assert_eq!(run.seed(), MINE_SEED);
        let idle = Input::default();
        // a foe in the loading hall comes for her
        let foe = run
            .actors
            .iter()
            .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Enemy && a.position.y < -2.0)
            .unwrap();
        let home = run.actors[foe].position;
        let (at, floor) = (0..16)
            .find_map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                let at = home + Vec3::new(a.cos(), 0.0, a.sin()) * 2.8;
                run.map
                    .capsule_floor(at + Vec3::Y * 0.3, 0.28)
                    .filter(|y| (y - home.y).abs() < 0.5)
                    .map(|y| (at, y))
            })
            .expect("room beside the foe");
        run.controller.position = Vec3::new(at.x, floor, at.z);
        let start = (run.actors[foe].position - run.controller.position).length();
        for _ in 0..240 {
            run.update(idle, 1.0 / 60.0);
        }
        let near = (run.actors[foe].position - run.controller.position).length();
        assert!(
            near < start || run.health < 100.0,
            "the foe neither came nor hit ({start} -> {near})"
        );
        // a chest: open it, let the lid rise, take what it holds
        let mut run = Dungeon::mine().unwrap();
        let chest = run
            .actors
            .iter()
            .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Chest)
            .unwrap();
        let spawn = run.actors[chest].spawn;
        let loot = run.authored.as_ref().unwrap().loot[&spawn].clone();
        let p = run.actors[chest].position;
        let face = run
            .authored
            .as_ref()
            .unwrap()
            .face(spawn)
            .unwrap_or(Vec3::Z);
        let stand = p + face * 1.2;
        let floor = run.map.capsule_floor(stand + Vec3::Y * 0.3, 0.28).unwrap();
        run.controller.position = Vec3::new(stand.x, floor, stand.z);
        for i in 0..run.actors.len() {
            if run.map.spawns[run.actors[i].spawn].health > 0.0 {
                run.actors[i].health = 0.0;
                run.actors[i].death = 99.0;
            }
        }
        assert_eq!(run.nearby_actor(), Some(chest));
        let gold = run.gold;
        run.interact();
        for _ in 0..180 {
            run.update(idle, 1.0 / 60.0);
        }
        run.interact();
        assert!(run.actors[chest].used, "the chest was not emptied");
        assert_eq!(run.gold, gold + loot.gold);
        // the Heart's throat: shut, then open
        let mine = run.authored.clone().unwrap();
        let gate = &mine.gates[0];
        let before = gate.center + Vec3::new(0.0, 0.0, gate.half[1] + 1.4);
        let floor = run.map.capsule_floor(before + Vec3::Y * 0.3, 0.28).unwrap();
        let walk = Input {
            forward: 1.0,
            ..Input::default()
        };
        for (seal, crest, through) in [(false, false, false), (true, true, true)] {
            run.seal = seal;
            run.crest = crest;
            run.controller.position = Vec3::new(before.x, floor, before.z);
            run.controller.yaw = 0.0;
            for _ in 0..120 {
                run.update(walk, 1.0 / 60.0);
            }
            let passed = run.controller.position.z < gate.center.z - gate.half[1];
            assert_eq!(
                passed, through,
                "seal {seal}, key {crest}: through the gate {passed}"
            );
        }
    }

    /// The Heart awake in the mine: it spits blood orbs at her, raises a wave of servants when its
    /// guardian falls to two thirds, and its ring of blood hurts her if she stands on the floor.
    #[test]
    fn the_heart_fights() {
        use crate::{combat::Fx, Dungeon};
        use velvet_dungeon::controller::Input;
        if !available() {
            return;
        }
        let mut run = Dungeon::mine().unwrap();
        let (_, floor) = run.authored.as_ref().unwrap().heart.unwrap();
        let boss = run
            .actors
            .iter()
            .position(|a| run.map.spawns[a.spawn].kind == SpawnKind::Boss)
            .unwrap();
        let room = run.map.spawns[run.actors[boss].spawn].room;
        // stand in the chamber, the guardian kept from reaching her
        let (at, y) = (0..24)
            .find_map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 24.0;
                let at = floor + Vec3::new(a.cos(), 0.0, a.sin()) * 7.5;
                run.map
                    .capsule_floor(at + Vec3::Y * 0.4, 0.28)
                    .filter(|y| {
                        (y - floor.y).abs() < 0.6
                            && run.map.room_at(Vec3::new(at.x, *y, at.z)) == Some(room)
                    })
                    .map(|y| (at, y))
            })
            .expect("floor in the Heart's chamber");
        run.controller.position = Vec3::new(at.x, y, at.z);
        assert_eq!(run.map.region_at(run.controller.position), room);
        let before = run.actors.len();
        let mut spat = false;
        for _ in 0..(12 * 60) {
            run.actors[boss].cooldown = 9.0;
            run.actors[boss].position = floor + Vec3::new(4.0, 0.0, -4.0);
            let was = (run.controller.position, run.health, run.notice.clone());
            run.update(Input::default(), 1.0 / 60.0);
            assert!(
                (run.controller.position - was.0).length() < 2.0,
                "she was moved from {:?} (health {}, {}) to {:?}: {}",
                was.0,
                was.1,
                was.2,
                run.controller.position,
                run.notice
            );
            spat |= run.fx.iter().any(|fx| matches!(fx, Fx::Spell { .. }));
            run.health = 100.0;
        }
        assert!(
            run.heart.awake,
            "the Heart woke (boss room {room} {}, she is in {} at {:?}, boss health {})",
            run.map.rooms[room].template,
            run.map.rooms[run.map.region_at(run.controller.position)].template,
            run.controller.position,
            run.actors[boss].health
        );
        assert!(spat, "the Heart spat blood at her");
        // its guardian at half its life: a wave of servants
        let full = run.map.spawns[run.actors[boss].spawn].health;
        run.actors[boss].health = full * 0.5;
        run.update(Input::default(), 1.0 / 60.0);
        assert!(run.heart.thresholds >= 1, "the turning point passed");
        assert!(run.actors.len() > before, "it raised servants");
        // the ring of blood: standing on the floor where it passes, she is hurt once
        let origin = floor;
        run.heart.wave = Some((origin, 0.0));
        run.heart.wave_hit = false;
        run.health = 100.0;
        for _ in 0..120 {
            run.actors[boss].cooldown = 9.0;
            run.update(Input::default(), 1.0 / 60.0);
        }
        assert!(run.heart.wave_hit, "the ring caught her");
    }

    /// A named champion: it falls, its own reward bursts out onto the floor with the rest and
    /// reaches her; and the forge's master rains embers on her while she is in his room.
    #[test]
    fn champions_fight_and_pay() {
        use crate::{combat::Fx, Dungeon};
        use velvet_dungeon::controller::Input;
        if !available() {
            return;
        }
        let mut run = Dungeon::mine().unwrap();
        let mine = run.authored.clone().unwrap();
        let (&id, smith) = mine
            .champions
            .iter()
            .find(|(_, c)| c.power == Power::Sparks)
            .expect("the smith");
        let index = run.actors.iter().position(|a| a.spawn == id).unwrap();
        let home = run.actors[index].position;
        let (at, y) = (0..24)
            .find_map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 24.0;
                let at = home + Vec3::new(a.cos(), 0.0, a.sin()) * 4.0;
                run.map
                    .capsule_floor(at + Vec3::Y * 0.4, 0.28)
                    .filter(|y| (y - home.y).abs() < 0.5)
                    .map(|y| (at, y))
            })
            .expect("room by the smith");
        run.controller.position = Vec3::new(at.x, y, at.z);
        let mut embers = false;
        for _ in 0..(9 * 60) {
            run.actors[index].cooldown = 9.0;
            run.actors[index].position = home;
            run.update(Input::default(), 1.0 / 60.0);
            embers |= run.fx.iter().any(
                |fx| matches!(fx, Fx::Spell { color, .. } if color[1] < 0.5 && color[0] > 0.9),
            );
            run.health = 100.0;
        }
        assert!(embers, "{} rains embers", smith.name);
        assert_eq!(
            run.boss_bar().map(|b| b.0.to_string()),
            Some(smith.name.clone())
        );
        // it falls: its reward and the rest lie on the floor, then reach her
        let gold = run.gold;
        run.actors[index].health = 0.5;
        run.controller.yaw = {
            let d = run.actors[index].position - run.controller.position;
            (-d.x).atan2(-d.z)
        };
        run.controller.position = home + (run.controller.position - home).normalize_or_zero() * 1.8;
        run.controller.position.y = home.y;
        run.controller.pitch = 0.0;
        run.swing_sword();
        for _ in 0..60 {
            run.update(Input::default(), 1.0 / 60.0);
            run.health = 100.0;
        }
        assert!(run.actors[index].health <= 0.0, "the blow lands");
        assert!(
            !run.pickups.is_empty() || run.gold > gold,
            "it left something"
        );
        for _ in 0..(14 * 60) {
            run.update(Input::default(), 1.0 / 60.0);
            run.health = 100.0;
        }
        assert!(run.pickups.is_empty(), "all of it reached her");
        assert!(
            run.gold >= gold + smith.reward.gold,
            "its reward reached her purse ({} -> {})",
            gold,
            run.gold
        );
        assert!(run.materials.get("ceniza_arcana").copied().unwrap_or(0) >= 3);
    }

    /// Foes walk round what stands on the floor to reach her, and their feet stay on the floor
    /// the whole way (up and down the rock's slopes): for melee foes of the mine with something
    /// between them and a spot of their own room, she waits there and each comes to her.
    #[test]
    fn foes_walk_round_things_with_their_feet_on_the_floor() {
        use crate::Dungeon;
        use velvet_dungeon::{controller::Input, horizontal};
        if !available() {
            return;
        }
        let base = Dungeon::mine().unwrap();
        let grid = base.map.floors.clone().unwrap();
        let mut tried = 0;
        for (index, actor) in base.actors.iter().enumerate() {
            let spawn = &base.map.spawns[actor.spawn];
            if spawn.kind != SpawnKind::Enemy
                || !matches!(
                    spawn.definition.as_str(),
                    "lurker" | "sentinel" | "bone_guard"
                )
            {
                continue;
            }
            let home = actor.position;
            // a spot of its room it cannot walk to straight, but can round something
            let spot = (0..48).find_map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                let r = 6.0 + (k / 16) as f32 * 1.5;
                let at = home + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let y = base.map.capsule_floor(at + Vec3::Y * 0.4, 0.28)?;
                let at = Vec3::new(at.x, y, at.z);
                ((y - home.y).abs() < 0.8
                    && base.map.room_at(at) == Some(spawn.room)
                    && !grid.clear_walk(home, at, 0.4)
                    && !grid.path(home, at, 0.4, 18.0).is_empty())
                .then_some(at)
            });
            let Some(spot) = spot else {
                continue;
            };
            tried += 1;
            let mut run = Dungeon::mine().unwrap();
            run.controller.position = spot;
            let mut closest = f32::MAX;
            for frame in 0..(12 * 60) {
                run.update(Input::default(), 1.0 / 60.0);
                run.health = 100.0;
                let foe = &run.actors[index];
                let under = run
                    .map
                    .surface(foe.position.x, foe.position.z, foe.position.y, 1.78)
                    .expect("a foe stands on a floor");
                assert!(
                    (foe.position.y - under).abs() < 0.12,
                    "{} {index}: feet {} over a floor at {} (frame {frame})",
                    spawn.definition,
                    foe.position.y,
                    under
                );
                closest = closest.min(horizontal(foe.position - spot).length());
            }
            assert!(
                closest < 2.6,
                "{} {index} from {home:?} reaches her at {spot:?} round what is between (closest {closest})",
                spawn.definition
            );
            if tried == 4 {
                break;
            }
        }
        assert!(
            tried >= 2,
            "melee foes with something between them and her ({tried})"
        );
    }

    /// No foe walks on the spot: with the Queen in its own room some way off, each comes to her
    /// round chests and stones and whatever stands between, and when something does hold it up it
    /// stands ready rather than walking without moving.
    #[test]
    fn no_foe_walks_on_the_spot() {
        use crate::{combat::Gait, Dungeon};
        use velvet_dungeon::{controller::Input, horizontal};
        if !available() {
            return;
        }
        let base = Dungeon::mine().unwrap();
        let mut checked = 0;
        for (index, actor) in base.actors.iter().enumerate() {
            let spawn = &base.map.spawns[actor.spawn];
            if !matches!(
                spawn.kind,
                SpawnKind::Enemy | SpawnKind::Elite | SpawnKind::Miniboss | SpawnKind::Boss
            ) {
                continue;
            }
            let home = actor.position;
            let spot = (0..48).find_map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                let r = 8.0 + (k / 16) as f32 * 1.5;
                let at = home + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                let y = base.map.capsule_floor(at + Vec3::Y * 0.4, 0.28)?;
                let at = Vec3::new(at.x, y, at.z);
                ((y - home.y).abs() < 1.0
                    && base.map.room_at(at) == Some(spawn.room)
                    && !base.map.walk_path(home, at, 0.4, 24.0).is_empty())
                .then_some(at)
            });
            let Some(spot) = spot else {
                continue;
            };
            checked += 1;
            let mut run = Dungeon::mine().unwrap();
            run.controller.position = spot;
            let mut on_the_spot = 0;
            let mut last = run.actors[index].position;
            for _ in 0..(8 * 60) {
                run.update(Input::default(), 1.0 / 60.0);
                run.health = 100.0;
                let a = &run.actors[index];
                if matches!(a.gait, Gait::Walk | Gait::Run)
                    && a.attack.is_none()
                    && horizontal(a.position - last).length() < 0.004
                {
                    on_the_spot += 1;
                }
                last = a.position;
            }
            assert!(
                on_the_spot < 30,
                "{} {index} walks on the spot {:.1} s",
                spawn.definition,
                on_the_spot as f32 / 60.0
            );
        }
        assert!(
            checked >= 10,
            "foes with the Queen in their room ({checked})"
        );
    }

    /// Every champion of the mine wears its own model and has its words; the smith speaks when he moves
    /// on her, rages at half his life, and says his last as he falls.
    #[test]
    fn champions_have_their_own_look_and_voice() {
        use crate::Dungeon;
        use velvet_dungeon::controller::Input;
        if !available() {
            return;
        }
        let mut run = Dungeon::mine().unwrap();
        let mine = run.authored.clone().unwrap();
        let mut models: Vec<&str> = run
            .map
            .spawns
            .iter()
            .filter_map(|s| champion_model(Some(&mine), s))
            .collect();
        models.sort_unstable();
        models.dedup();
        assert_eq!(
            models,
            [
                "bibliotecaria",
                "centinela",
                "custodio",
                "guardian",
                "herrero",
                "senor"
            ]
        );
        assert!(models.iter().all(|m| champion_lines(m).is_some()));
        let (&id, smith) = mine
            .champions
            .iter()
            .find(|(_, c)| c.power == Power::Sparks)
            .unwrap();
        let index = run.actors.iter().position(|a| a.spawn == id).unwrap();
        let home = run.actors[index].position;
        let spot = (0..24)
            .find_map(|k| {
                let a = k as f32 * std::f32::consts::TAU / 24.0;
                let at = home + Vec3::new(a.cos(), 0.0, a.sin()) * 5.0;
                run.map
                    .capsule_floor(at + Vec3::Y * 0.4, 0.28)
                    .filter(|y| (y - home.y).abs() < 0.5)
                    .map(|y| Vec3::new(at.x, y, at.z))
            })
            .unwrap();
        run.controller.position = spot;
        let said = |run: &Dungeon| run.speech.as_ref().map(|s| (s.who.clone(), s.line));
        let mut woke = None;
        for _ in 0..(4 * 60) {
            run.update(Input::default(), 1.0 / 60.0);
            run.health = 100.0;
            if woke.is_none() {
                woke = said(&run);
            }
        }
        let lines = champion_lines("herrero").unwrap();
        assert_eq!(
            woke,
            Some((smith.name.clone(), lines.wake)),
            "he speaks on seeing her"
        );
        run.actors[index].health = run.map.spawns[id].health * 0.45;
        let mut raged = false;
        for _ in 0..(3 * 60) {
            run.update(Input::default(), 1.0 / 60.0);
            run.health = 100.0;
            raged |= said(&run).is_some_and(|(_, l)| l == lines.rage);
        }
        assert!(raged, "he rages aloud");
        run.actors[index].health = 0.0;
        run.update(Input::default(), 1.0 / 60.0);
        assert_eq!(said(&run).map(|s| s.1), Some(lines.fall), "his last words");
    }

    /// The sanctuary's own spot is the top of its altar, walled by its rim: dying (even with a
    /// sanctuary saved there by an older game) brings her back onto free floor beside it, and
    /// standing up there pushing to walk lets her out.
    #[test]
    fn she_is_never_left_boxed_in_at_the_sanctuary() {
        use crate::{walk_reach, Dungeon};
        use velvet_dungeon::{controller::Input, gameplay::SpawnKind, horizontal};
        if !available() {
            return;
        }
        let mut run = Dungeon::mine().unwrap();
        let font = run
            .actors
            .iter()
            .map(|a| run.map.spawns[a.spawn].clone())
            .find(|s| s.kind == SpawnKind::Rest)
            .unwrap()
            .position;
        assert!(walk_reach(&run.map, font, 2.0) < 2.0, "the altar's top is boxed in");
        run.checkpoint = font;
        run.health = 0.0;
        run.update(Input::default(), 1.0 / 60.0);
        let back = run.controller.position;
        assert!(walk_reach(&run.map, back, 2.0) >= 2.0, "she walks away from {back:?}");
        assert!(horizontal(back - font).length() < 4.0, "by the sanctuary");
        // up on the altar, pushing to walk
        run.controller = velvet_dungeon::controller::Controller::new(font);
        let push = Input {
            forward: 1.0,
            ..Input::default()
        };
        for _ in 0..90 {
            run.update(push, 1.0 / 60.0);
        }
        let out = run.controller.position;
        assert!(walk_reach(&run.map, out, 2.0) >= 2.0, "let out to {out:?}");
        // where she comes in, the floor is free: pushing there never moves her by magic
        let run = Dungeon::mine().unwrap();
        assert!(walk_reach(&run.map, run.controller.position, 2.0) >= 2.0);
    }

    /// The shaft's head-frame stands by the stall with all of it on clear floor at his level, is
    /// solid (legs, winch, cart), and its open front can be walked up to.
    #[test]
    fn the_headframe_stands_clear_by_the_stall() {
        use crate::{turn_y, Dungeon, HEADFRAME_FOOT};
        use velvet_dungeon::PLAYER_RADIUS;
        if !available() {
            return;
        }
        let run = Dungeon::mine().unwrap();
        let shaft = run.shaft.expect("a shaft by the stall");
        let yaw = run.shaft_yaw;
        let mut bare = run.map.clone();
        bare.obstacles.clear();
        for (x, z) in HEADFRAME_FOOT {
            let p = shaft + turn_y(Vec3::new(x, 0.0, z), yaw);
            let floor = bare.capsule_floor(p + Vec3::Y * 0.6, 0.3);
            assert!(
                floor.is_some_and(|y| (y - shaft.y).abs() < 0.45),
                "clear at {x} {z}"
            );
        }
        assert_eq!(run.map.obstacles.len(), 6, "legs, winch and cart");
        let leg = shaft + turn_y(Vec3::new(1.3, 0.0, 1.3), yaw);
        assert!(run.map.capsule_floor(leg, PLAYER_RADIUS).is_none(), "a leg is solid");
        // in front of its open side she stands, near enough to go down
        let front = shaft + turn_y(Vec3::new(0.0, 0.0, 2.1), yaw);
        assert!(run.map.capsule_floor(front + Vec3::Y * 0.3, PLAYER_RADIUS).is_some());
    }
}
