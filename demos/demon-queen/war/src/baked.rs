//! The battle maps built in the war map prototype (demos/demon-queen-warmap-threejs: its 26
//! battlefields, one per region), baked for the game by its `export_battles_game.py` (the land)
//! and `export_game_battle.py` (the structures) into `data/guerra/batallas/<id>/`:
//!
//! - `campo.bin`: the 2 m node grid (the field's own): heights, the water's surface, woods, marsh,
//!   roads, cliffs, the ground's colour, grass; the trees and the rocks.
//! - `estructuras.bin`: a 1 m grid of what stands on the land: a solid, a deck to walk on (the
//!   bridges), or one of the works (walls, towers, gates, keeps, palisades) that can be broken.
//! - `obras.json`: those works, the walled enclosures, the siege engines the prototype had
//!   standing, the anchors (banners, torches); `campo.json`: names, deployment, light.
//!
//! The game reads a map from its folder and installs it here; `Battle::new` uses it when the
//! setup's map names one (otherwise the hand-made fields of `maps.rs`).
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock, RwLock};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::maps::Rect;

/// The battles, their ids and their regions (the campaign's), in the prototype's order.
pub const BATTLE_MAPS: [(&str, &str); 26] = [
    ("asedio_castillo", "castillo"),
    ("asedio_fortaleza", "marca"),
    ("asedio_ciudad", "valdesanto"),
    ("asedio_capital", "capital"),
    ("puente", "puerta"),
    ("vado", "vado"),
    ("paso_montana", "paso"),
    ("aldea", "villa"),
    ("riscos_runas", "cueva"),
    ("bosque", "bosque"),
    ("claro", "claro"),
    ("colinas", "madriguera"),
    ("cantera", "cantera"),
    ("costa", "caleta"),
    ("cienaga", "cienaga"),
    ("marisma", "desembocadura"),
    ("penascal", "penascal"),
    ("nido", "nido"),
    ("cubil", "cubil"),
    ("cabeza_puente", "cabeza"),
    ("trigales", "trigales"),
    ("molinos", "molinoalto"),
    ("puerto_rio", "ribera"),
    ("monasterio", "monasterio"),
    ("puerto", "puerto_santo"),
    ("minas", "minas"),
];

/// The battle map of a campaign region, if it has one.
pub fn map_of_region(region: &str) -> Option<&'static str> {
    BATTLE_MAPS
        .iter()
        .find(|(_, r)| *r == region)
        .map(|(id, _)| *id)
}

/// A tree or a rock: where, how turned and how big, and its kind (trees: 0 pine, 1 broadleaf,
/// 2 dark wood, 3 dead, 4 bush; rocks 0..2).
#[derive(Clone, Copy, Debug)]
pub struct Scatter {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rot: f32,
    pub scale: f32,
    pub kind: u8,
}

/// The land on the field's 2 m node grid.
pub struct Land {
    pub nx: usize,
    pub nz: usize,
    pub x0: f32,
    pub z0: f32,
    pub res: f32,
    pub h: Vec<f32>,
    /// The water's surface at each node (`NO_WATER` where there is none), and the same reaching
    /// one ring of nodes past the water's edge (the sheet's banks).
    pub water: Vec<f32>,
    pub water_banks: Vec<f32>,
    pub forest: Vec<f32>,
    pub marsh: Vec<f32>,
    pub road: Vec<f32>,
    pub cliff: Vec<f32>,
    /// The ground's colour (sRGB).
    pub colour: Vec<[u8; 3]>,
    pub grass: Vec<u8>,
    pub crop: Vec<u8>,
    pub reeds: Vec<u8>,
    pub trees: Vec<Scatter>,
    pub rocks: Vec<Scatter>,
}

pub const NO_WATER: f32 = -1000.0;

/// What stands on each metre of the land.
pub struct Blocks {
    pub w: usize,
    pub d: usize,
    pub x0: f32,
    pub z0: f32,
    pub res: f32,
    /// 0 free, 1 solid, 2 deck (walk on it at `top`), 3 a work (`work` - 1).
    pub kind: Vec<u8>,
    pub work: Vec<u16>,
    pub top: Vec<f32>,
}

pub const FREE: u8 = 0;
pub const SOLID: u8 = 1;
pub const DECK: u8 = 2;
pub const WORK: u8 = 3;

impl Blocks {
    pub fn cell(&self, x: f32, z: f32) -> Option<usize> {
        let i = ((x - self.x0) / self.res).floor();
        let j = ((z - self.z0) / self.res).floor();
        if i < 0.0 || j < 0.0 || i >= self.w as f32 || j >= self.d as f32 {
            return None;
        }
        Some(j as usize * self.w + i as usize)
    }
}

/// What a work is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkKind {
    /// A stretch of curtain wall.
    Wall,
    /// A tower on the wall (or standing alone).
    Tower,
    /// A gate's doors and portcullis.
    Gate,
    /// The keep: the heart of the place, what a siege is for.
    Keep,
    /// A wooden palisade.
    Palisade,
}

/// A work as the prototype built it (map metres, x east, z south).
#[derive(Clone, Debug, Deserialize)]
pub struct WorkSpec {
    pub id: usize,
    pub kind: String,
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub z: Option<f32>,
    #[serde(default)]
    pub a: Option<[f32; 2]>,
    #[serde(default)]
    pub b: Option<[f32; 2]>,
    #[serde(default)]
    pub r: Option<f32>,
    #[serde(default)]
    pub w: Option<f32>,
    #[serde(default)]
    pub d: Option<f32>,
    #[serde(default)]
    pub yaw: Option<f32>,
    #[serde(default)]
    pub base: Option<f32>,
    #[serde(default)]
    pub top: Option<f32>,
    #[serde(default)]
    pub thick: Option<f32>,
    #[serde(default)]
    pub gate: Option<[f32; 2]>,
    #[serde(default)]
    pub pts: Option<Vec<[f32; 2]>>,
}

impl WorkSpec {
    pub fn kind(&self) -> WorkKind {
        match self.kind.as_str() {
            "muro" => WorkKind::Wall,
            "puerta" => WorkKind::Gate,
            "torreon" => WorkKind::Keep,
            "empalizada" => WorkKind::Palisade,
            _ => WorkKind::Tower,
        }
    }

    /// Its middle on the map.
    pub fn centre(&self) -> [f32; 2] {
        if let (Some(a), Some(b)) = (self.a, self.b) {
            return [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
        }
        if let (Some(x), Some(z)) = (self.x, self.z) {
            return [x, z];
        }
        if let Some(p) = &self.pts {
            if !p.is_empty() {
                let n = p.len() as f32;
                return [
                    p.iter().map(|q| q[0]).sum::<f32>() / n,
                    p.iter().map(|q| q[1]).sum::<f32>() / n,
                ];
            }
        }
        [0.0, 0.0]
    }

    /// The nearest point of the work's footprint to (x, z) (its line, or its centre).
    pub fn nearest(&self, x: f32, z: f32) -> [f32; 2] {
        let seg = |a: [f32; 2], b: [f32; 2]| {
            let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
            let l2 = (dx * dx + dz * dz).max(1e-6);
            let t = (((x - a[0]) * dx + (z - a[1]) * dz) / l2).clamp(0.0, 1.0);
            [a[0] + dx * t, a[1] + dz * t]
        };
        if let (Some(a), Some(b)) = (self.a, self.b) {
            return seg(a, b);
        }
        if let Some(p) = &self.pts {
            let mut best = self.centre();
            let mut bd = f32::INFINITY;
            for w in p.windows(2) {
                let q = seg(w[0], w[1]);
                let d = (q[0] - x).hypot(q[1] - z);
                if d < bd {
                    bd = d;
                    best = q;
                }
            }
            return best;
        }
        self.centre()
    }

    /// How tall it stands (its walk or roof line).
    pub fn top(&self) -> f32 {
        self.top.unwrap_or(4.0)
    }
}

/// A siege engine standing on the prototype's field.
#[derive(Clone, Debug, Deserialize)]
pub struct EngineSpec {
    pub kind: String,
    pub x: f32,
    pub z: f32,
    #[serde(default)]
    pub yaw: f32,
}

/// A banner, a torch, a fire, a glow.
#[derive(Clone, Debug, Deserialize)]
pub struct Anchor {
    pub kind: String,
    pub p: [f32; 3],
    #[serde(default)]
    pub size: Option<f32>,
    #[serde(default)]
    pub faction: Option<String>,
}

/// The light the prototype gave the battle.
#[derive(Clone, Debug, Default)]
pub struct Look {
    pub sky: String,
    pub grade: String,
    pub exposure: f32,
    pub sun_dir: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_power: f32,
    pub hemi_sky: [f32; 3],
    pub hemi_ground: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_density: f32,
}

pub struct BattleMap {
    pub id: String,
    pub name: String,
    pub kicker: String,
    pub blurb: String,
    /// The prototype's kind of battle: "siege", "river", "pass", "coast", "field"...
    pub kind: String,
    pub region: String,
    pub features: Vec<String>,
    pub attacker_zone: Rect,
    /// Where the defenders stand: a rectangle of the field, or (sieges) inside the walls.
    pub defender_zone: Rect,
    pub defends_walls: bool,
    pub enclosures: Vec<Vec<[f32; 2]>>,
    pub land: Land,
    pub blocks: Blocks,
    pub works: Vec<WorkSpec>,
    pub engines: Vec<EngineSpec>,
    pub anchors: Vec<Anchor>,
    pub look: Look,
}

impl BattleMap {
    pub fn is_siege(&self) -> bool {
        self.kind == "siege" || !self.enclosures.is_empty()
    }

    /// Whether (x, z) is inside the walls (any enclosure).
    pub fn inside(&self, x: f32, z: f32) -> bool {
        self.enclosures.iter().any(|ring| in_poly(x, z, ring))
    }

    /// The keep's work, if the place has one.
    pub fn keep(&self) -> Option<&WorkSpec> {
        self.works.iter().find(|w| w.kind() == WorkKind::Keep)
    }
}

pub fn in_poly(x: f32, z: f32, pts: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut j = pts.len().wrapping_sub(1);
    for i in 0..pts.len() {
        let (xi, zi) = (pts[i][0], pts[i][1]);
        let (xj, zj) = (pts[j][0], pts[j][1]);
        if (zi > z) != (zj > z) && x < (xj - xi) * (z - zi) / (zj - zi + 1e-12) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

// ------------------------------------------------------------------ reading
struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.at + n > self.b.len() {
            bail!("the file ends early");
        }
        let s = &self.b[self.at..self.at + n];
        self.at += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn f32s(&mut self, n: usize) -> Result<Vec<f32>> {
        Ok(self
            .take(n * 4)?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect())
    }
    fn u16s(&mut self, n: usize) -> Result<Vec<u16>> {
        Ok(self
            .take(n * 2)?
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect())
    }
    fn u8s(&mut self, n: usize) -> Result<Vec<u8>> {
        Ok(self.take(n)?.to_vec())
    }
    fn scatter(&mut self) -> Result<Vec<Scatter>> {
        let n = self.u32()? as usize;
        let v = self.f32s(n * 6)?;
        Ok(v.as_chunks::<6>()
            .0
            .iter()
            .map(|c| Scatter {
                x: c[0],
                y: c[1],
                z: c[2],
                rot: c[3],
                scale: c[4],
                kind: c[5] as u8,
            })
            .collect())
    }
}

fn read_land(bytes: &[u8]) -> Result<Land> {
    let mut r = Reader { b: bytes, at: 0 };
    if r.take(4)? != b"DQCB" {
        bail!("not a battle land");
    }
    let _version = r.u32()?;
    let nx = r.u32()? as usize;
    let nz = r.u32()? as usize;
    let (x0, z0, res) = (r.f32()?, r.f32()?, r.f32()?);
    let n = nx * nz;
    let h = r.f32s(n)?;
    let water = r.f32s(n)?;
    let unit = |v: Vec<u8>| {
        v.into_iter()
            .map(|b| b as f32 / 255.0)
            .collect::<Vec<f32>>()
    };
    let forest = unit(r.u8s(n)?);
    let marsh = unit(r.u8s(n)?);
    let road = unit(r.u8s(n)?);
    let cliff = unit(r.u8s(n)?);
    let colour = r.u8s(n * 3)?.as_chunks::<3>().0.to_vec();
    let grass = r.u8s(n)?;
    let crop = r.u8s(n)?;
    let reeds = r.u8s(n)?;
    let trees = r.scatter()?;
    let rocks = r.scatter()?;
    // the water sheet reaches a ring past its edge, at the level of the water beside it
    let mut water_banks = water.clone();
    for _ in 0..2 {
        let prev = water_banks.clone();
        for j in 0..nz {
            for i in 0..nx {
                let k = j * nx + i;
                if prev[k] > NO_WATER {
                    continue;
                }
                let mut best = NO_WATER;
                for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (ii, jj) = (i as i32 + di, j as i32 + dj);
                    if ii < 0 || jj < 0 || ii >= nx as i32 || jj >= nz as i32 {
                        continue;
                    }
                    best = best.max(prev[jj as usize * nx + ii as usize]);
                }
                water_banks[k] = best;
            }
        }
    }
    Ok(Land {
        nx,
        nz,
        x0,
        z0,
        res,
        h,
        water,
        water_banks,
        forest,
        marsh,
        road,
        cliff,
        colour,
        grass,
        crop,
        reeds,
        trees,
        rocks,
    })
}

fn read_blocks(bytes: &[u8]) -> Result<Blocks> {
    let mut r = Reader { b: bytes, at: 0 };
    if r.take(4)? != b"DQST" {
        bail!("not a battle's structures");
    }
    let _version = r.u32()?;
    let w = r.u32()? as usize;
    let d = r.u32()? as usize;
    let (x0, z0, res) = (r.f32()?, r.f32()?, r.f32()?);
    let n = w * d;
    let kind = r.u8s(n)?;
    let work = r.u16s(n)?;
    let top = r.f32s(n)?;
    Ok(Blocks {
        w,
        d,
        x0,
        z0,
        res,
        kind,
        work,
        top,
    })
}

#[derive(Deserialize)]
struct RectJson {
    x0: f32,
    x1: f32,
    z0: f32,
    z1: f32,
}

#[derive(Deserialize)]
struct CampoJson {
    name: String,
    #[serde(default)]
    kicker: String,
    #[serde(default)]
    blurb: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    features: Vec<String>,
    deploy: serde_json::Value,
    #[serde(default)]
    look: serde_json::Value,
}

#[derive(Deserialize)]
struct ObrasJson {
    works: Vec<WorkSpec>,
    #[serde(default)]
    enclosures: Vec<Vec<[f32; 2]>>,
    #[serde(default)]
    engines: Vec<EngineSpec>,
    #[serde(default)]
    anchors: Vec<Anchor>,
}

fn hex_colour(v: &serde_json::Value) -> Option<[f32; 3]> {
    let s = v.as_str()?.trim_start_matches('#');
    let c = u32::from_str_radix(s, 16).ok()?;
    Some(crate::data::rgb(c))
}

fn look(v: &serde_json::Value) -> Look {
    let f = |p: &serde_json::Value, d: f32| p.as_f64().map_or(d, |x| x as f32);
    let sun = &v["sun"];
    let dir = sun["dir"]
        .as_array()
        .map(|a| {
            let g = |i: usize| a.get(i).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
            [g(0), g(1), g(2)]
        })
        .unwrap_or([-0.5, 0.6, 0.4]);
    let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
        .sqrt()
        .max(1e-6);
    Look {
        sky: v["sky"].as_str().unwrap_or("day").to_owned(),
        grade: v["grade"].as_str().unwrap_or("neutral").to_owned(),
        exposure: f(&v["exposure"], 1.0),
        sun_dir: [dir[0] / len, dir[1] / len, dir[2] / len],
        sun_color: hex_colour(&sun["color"]).unwrap_or([1.0, 0.9, 0.8]),
        sun_power: f(&sun["power"], 3.0),
        hemi_sky: hex_colour(&v["hemi"]["sky"]).unwrap_or([0.6, 0.65, 0.8]),
        hemi_ground: hex_colour(&v["hemi"]["ground"]).unwrap_or([0.35, 0.3, 0.25]),
        fog_color: hex_colour(&v["fog"]["color"]).unwrap_or([0.6, 0.6, 0.65]),
        fog_density: f(&v["fog"]["density"], 0.0015),
    }
}

/// Reads a battle map from its folder (`data/guerra/batallas/<id>`).
pub fn load(dir: &Path) -> Result<BattleMap> {
    let id = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_owned();
    let read = |name: &str| {
        std::fs::read(dir.join(name)).with_context(|| format!("{}", dir.join(name).display()))
    };
    let land = read_land(&read("campo.bin")?)?;
    let blocks = read_blocks(&read("estructuras.bin")?)?;
    let campo: CampoJson = serde_json::from_slice(&read("campo.json")?)?;
    let obras: ObrasJson = serde_json::from_slice(&read("obras.json")?)?;
    let rect = |v: &serde_json::Value| -> Option<Rect> {
        let r: RectJson = serde_json::from_value(v.clone()).ok()?;
        Some(Rect {
            x0: r.x0.min(r.x1),
            x1: r.x0.max(r.x1),
            z0: r.z0.min(r.z1),
            z1: r.z0.max(r.z1),
        })
    };
    let attacker_zone = rect(&campo.deploy["attacker"]).unwrap_or(Rect {
        x0: -170.0,
        x1: 170.0,
        z0: 65.0,
        z1: 148.0,
    });
    let (defender_zone, defends_walls) = match rect(&campo.deploy["defender"]) {
        Some(r) => (r, false),
        None => {
            // inside the walls: the enclosures' box, kept off the walls
            let (mut x0, mut x1, mut z0, mut z1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for p in obras.enclosures.iter().flatten() {
                x0 = x0.min(p[0]);
                x1 = x1.max(p[0]);
                z0 = z0.min(p[1]);
                z1 = z1.max(p[1]);
            }
            if x0 > x1 {
                (
                    Rect {
                        x0: -170.0,
                        x1: 170.0,
                        z0: -148.0,
                        z1: -65.0,
                    },
                    false,
                )
            } else {
                (
                    Rect {
                        x0: x0 + 6.0,
                        x1: x1 - 6.0,
                        z0: z0 + 6.0,
                        z1: z1 - 6.0,
                    },
                    true,
                )
            }
        }
    };
    Ok(BattleMap {
        id,
        name: campo.name,
        kicker: campo.kicker,
        blurb: campo.blurb,
        kind: if campo.kind.is_empty() {
            "field".into()
        } else {
            campo.kind
        },
        region: campo.region,
        features: campo.features,
        attacker_zone,
        defender_zone,
        defends_walls,
        enclosures: obras.enclosures,
        land,
        blocks,
        works: obras.works,
        engines: obras.engines,
        anchors: obras.anchors,
        look: look(&campo.look),
    })
}

fn registry() -> &'static RwLock<HashMap<String, Arc<BattleMap>>> {
    static MAPS: OnceLock<RwLock<HashMap<String, Arc<BattleMap>>>> = OnceLock::new();
    MAPS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Puts a map where `Battle::new` finds it.
pub fn install(map: BattleMap) -> Arc<BattleMap> {
    let m = Arc::new(map);
    registry()
        .write()
        .unwrap()
        .insert(m.id.clone(), Arc::clone(&m));
    m
}

/// A map already installed.
pub fn get(id: &str) -> Option<Arc<BattleMap>> {
    registry().read().unwrap().get(id).cloned()
}

/// The map `id`, read from `root/<id>` the first time it is wanted.
pub fn ensure(id: &str, root: &Path) -> Option<Arc<BattleMap>> {
    if let Some(m) = get(id) {
        return Some(m);
    }
    if !BATTLE_MAPS.iter().any(|(b, _)| *b == id) {
        return None;
    }
    match load(&root.join(id)) {
        Ok(m) => Some(install(m)),
        Err(e) => {
            eprintln!("[guerra] battle map «{id}»: {e:#}");
            None
        }
    }
}
