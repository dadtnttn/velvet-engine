//! Deterministic dungeon blueprints, independent of renderer and concrete assets.
//! Pipeline: semantic graph -> constrained embedding -> sockets/surfaces ->
//! regions -> contextual gameplay -> physical validation. No GPU/IO in generate.
pub mod controller;
pub mod diagnostics;
pub mod floors;
pub mod gameplay;
mod generator;
pub mod navigation;
pub mod rng;
pub mod validation;

pub use floors::{FloorGrid, FloorSpan};
pub use generator::generate;
use serde::{Deserialize, Serialize};
use velvet_math::Vec3;
pub const GENERATOR_VERSION: u32 = 2;
pub const PLAYER_RADIUS: f32 = 0.28;
pub const PLAYER_HEIGHT: f32 = 1.78;
pub const STEP_HEIGHT: f32 = 0.32;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub critical_path_length: usize,
    pub branch_factor: f32,
    pub loop_probability: f32,
    pub verticality: f32,
    pub room_density: f32,
    pub secret_density: f32,
    pub combat_density: f32,
    pub treasure_density: f32,
    pub danger_level: f32,
    pub corridor_length: f32,
    pub room_scale: f32,
    pub biome_complexity: usize,
    pub decoration_density: f32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            critical_path_length: 12,
            branch_factor: 0.65,
            loop_probability: 0.55,
            verticality: 0.65,
            room_density: 0.72,
            secret_density: 0.18,
            combat_density: 0.60,
            treasure_density: 0.27,
            danger_level: 1.0,
            corridor_length: 12.0,
            room_scale: 1.0,
            biome_complexity: 3,
            decoration_density: 1.0,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !(10..=96).contains(&self.critical_path_length) {
            return Err("critical_path_length must be 10..96".into());
        }
        for (name, v) in [
            ("branch_factor", self.branch_factor),
            ("loop_probability", self.loop_probability),
            ("verticality", self.verticality),
            ("room_density", self.room_density),
            ("secret_density", self.secret_density),
            ("combat_density", self.combat_density),
            ("treasure_density", self.treasure_density),
        ] {
            if !v.is_finite() || !(0.0..=1.0).contains(&v) {
                return Err(format!("{name} must be finite in 0..1"));
            }
        }
        if !self.room_scale.is_finite()
            || !(0.8..=1.5).contains(&self.room_scale)
            || !self.corridor_length.is_finite()
            || !(10.0..=28.0).contains(&self.corridor_length)
            || !self.danger_level.is_finite()
            || !(0.1..=5.0).contains(&self.danger_level)
            || !self.decoration_density.is_finite()
            || !(0.0..=2.0).contains(&self.decoration_density)
            || !(1..=32).contains(&self.biome_complexity)
        {
            return Err(
                "invalid scale, corridor length, danger, decoration or biome complexity".into(),
            );
        }
        Ok(())
    }
    pub fn spacing(&self) -> f32 {
        self.corridor_length + 16.0 * self.room_scale
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Entrance,
    Exit,
    Normal,
    Encounter,
    Treasure,
    Secret,
    Puzzle,
    Event,
    Rest,
    Elite,
    Miniboss,
    Boss,
}
impl Role {
    pub fn label(self) -> &'static str {
        match self {
            Self::Entrance => "Umbral",
            Self::Exit => "Retorno",
            Self::Normal => "Galeria",
            Self::Encounter => "Acechadores",
            Self::Treasure => "Tesoro",
            Self::Secret => "Camara oculta",
            Self::Puzzle => "Sello antiguo",
            Self::Event => "Memoria de piedra",
            Self::Rest => "Santuario",
            Self::Elite => "Centinela",
            Self::Miniboss => "Custodio",
            Self::Boss => "Corazon de la caverna",
        }
    }
    pub fn combat(self) -> bool {
        matches!(
            self,
            Self::Encounter | Self::Elite | Self::Miniboss | Self::Boss
        )
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Vault,
    Cavern,
    Bridge,
    Arena,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomTemplate {
    pub id: String,
    pub shape: Shape,
    pub half_width: [f32; 2],
    pub half_depth: [f32; 2],
    pub height: f32,
    pub weight: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Biome {
    pub id: String,
    pub name: String,
    pub wall: [f32; 3],
    pub floor: [f32; 3],
    pub accent: [f32; 3],
    pub ambient: [f32; 3],
    pub fog: [f32; 3],
    pub danger: f32,
    pub decoration: String,
    pub audio: String,
    pub particles: String,
    pub enemy_pool: Vec<String>,
    pub loot_pool: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyDef {
    pub id: String,
    pub health: f32,
    pub damage: f32,
    pub speed: f32,
    pub cost: f32,
    #[serde(default)]
    pub optional_only: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LootDef {
    pub id: String,
    pub gold_min: u32,
    pub gold_max: u32,
    pub heal: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Catalog {
    pub revision: u32,
    pub templates: Vec<RoomTemplate>,
    pub biomes: Vec<Biome>,
    pub enemies: Vec<EnemyDef>,
    pub loot: Vec<LootDef>,
}
impl Catalog {
    pub fn bundled() -> Self {
        serde_json::from_str(include_str!("../data/caverns.json"))
            .expect("bundled cave definitions")
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.templates.is_empty()
            || self.biomes.is_empty()
            || self.enemies.is_empty()
            || self.loot.is_empty()
        {
            return Err("empty content registry".into());
        }
        for (name, ids) in [
            (
                "templates",
                self.templates
                    .iter()
                    .map(|x| x.id.as_str())
                    .collect::<Vec<_>>(),
            ),
            (
                "biomes",
                self.biomes.iter().map(|x| x.id.as_str()).collect(),
            ),
            (
                "enemies",
                self.enemies.iter().map(|x| x.id.as_str()).collect(),
            ),
            ("loot", self.loot.iter().map(|x| x.id.as_str()).collect()),
        ] {
            let mut seen = std::collections::BTreeSet::new();
            if ids
                .into_iter()
                .any(|id| id.trim().is_empty() || !seen.insert(id))
            {
                return Err(format!("empty or duplicate identifier in {name}"));
            }
        }
        if self.templates.iter().all(|t| t.weight == 0) {
            return Err("all template weights are zero".into());
        }
        for t in &self.templates {
            if t.half_width
                .iter()
                .chain(t.half_depth.iter())
                .any(|v| !v.is_finite() || !(4.0..=8.0).contains(v))
                || t.half_width[0] > t.half_width[1]
                || t.half_depth[0] > t.half_depth[1]
                || !t.height.is_finite()
                || !(3.4..=7.5).contains(&t.height)
            {
                return Err(format!("invalid template {}", t.id));
            }
        }
        for b in &self.biomes {
            if !b.danger.is_finite()
                || !(0.1..=5.0).contains(&b.danger)
                || [&b.wall, &b.floor, &b.accent, &b.ambient, &b.fog]
                    .into_iter()
                    .flatten()
                    .any(|v| !v.is_finite() || !(0.0..=4.0).contains(v))
                || b.enemy_pool.is_empty()
                || b.loot_pool.is_empty()
                || b.enemy_pool
                    .iter()
                    .any(|id| !self.enemies.iter().any(|e| &e.id == id))
                || b.loot_pool
                    .iter()
                    .any(|id| !self.loot.iter().any(|e| &e.id == id))
            {
                return Err(format!("unresolved pools in {}", b.id));
            }
        }
        for e in &self.enemies {
            if [e.health, e.damage, e.speed, e.cost]
                .into_iter()
                .any(|x| !x.is_finite() || x <= 0.0)
                || e.health > 10000.0
                || e.damage > 1000.0
                || e.speed > 5.0
                || e.cost < 1.0
            {
                return Err(format!("invalid enemy {}", e.id));
            }
        }
        for l in &self.loot {
            if l.gold_min > l.gold_max
                || l.gold_max > 100000
                || !l.heal.is_finite()
                || !(0.0..=100.0).contains(&l.heal)
            {
                return Err(format!("invalid loot {}", l.id));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Socket {
    pub connection: usize,
    pub position: Vec3,
    pub outward: Vec3,
    pub width: f32,
    pub height: f32,
    pub kind: String,
    pub compatible: Vec<String>,
    pub clearance: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: usize,
    pub role: Role,
    pub critical: bool,
    pub progress: f32,
    pub region: usize,
    pub grid: [i32; 3],
    pub center: Vec3,
    pub half: [f32; 2],
    pub height: f32,
    pub shape: Shape,
    pub template: String,
    pub biome: usize,
    pub outline: Vec<[f32; 2]>,
    pub sockets: Vec<Socket>,
    pub landmark_seed: u64,
    /// Treasure rooms only: one grand treasury per expedition, the rest
    /// small loot halls.
    #[serde(default)]
    pub treasury: Option<Treasury>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Treasury {
    Grand,
    Small,
}
impl Room {
    pub fn is_grand_treasury(&self) -> bool {
        self.role == Role::Treasure && self.treasury == Some(Treasury::Grand)
    }
    pub fn contains(&self, x: f32, z: f32, margin: f32) -> bool {
        let p = [x - self.center.x, z - self.center.z];
        // Convex, counter-clockwise footprint; signed edge distance. Positive
        // margin shrinks walkable space, independent of tessellation/rendering.
        (0..self.outline.len()).all(|i| {
            let a = self.outline[i];
            let b = self.outline[(i + 1) % self.outline.len()];
            let dx = b[0] - a[0];
            let dz = b[1] - a[1];
            dx * (p[1] - a[1]) - dz * (p[0] - a[0]) >= margin * (dx * dx + dz * dz).sqrt() - 0.001
        })
    }
    pub fn floor_y(&self, x: f32, z: f32) -> f32 {
        if self.shape == Shape::Bridge
            && (x - self.center.x).abs() > 1.8
            && (z - self.center.z).abs() > 1.8
        {
            self.center.y - 4.0
        } else {
            self.center.y
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    Tunnel,
    Ramp,
    Stairs,
    Bridge,
    Secret,
    Shortcut,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connection {
    pub id: usize,
    pub a: usize,
    pub b: usize,
    pub start: Vec3,
    pub end: Vec3,
    pub width: f32,
    pub height: f32,
    pub kind: ConnectionKind,
    pub critical: bool,
}
impl Connection {
    pub fn length(&self) -> f32 {
        horizontal(self.end - self.start).length()
    }
    pub fn project(&self, x: f32, z: f32) -> (f32, f32) {
        let d = horizontal(self.end - self.start);
        let len = d.length();
        let p = Vec3::new(x - self.start.x, 0.0, z - self.start.z);
        (p.dot(d) / (len * len), (p.x * d.z - p.z * d.x).abs() / len)
    }
    pub fn floor_y(&self, t: f32) -> f32 {
        let y = self.start.y + (self.end.y - self.start.y) * t.clamp(0.0, 1.0);
        if self.kind == ConnectionKind::Stairs {
            let n = ((self.end.y - self.start.y).abs() / 0.15).ceil().max(1.0);
            // Step at each tread boundary, matching the rendered risers. The
            // old round() changed height halfway through a tread and left the
            // first/last half-step out of sync with the collision surface.
            let step = (t.clamp(0.0, 1.0) * n + 1.0e-5).floor().min(n);
            self.start.y + (self.end.y - self.start.y) * step / n
        } else {
            y
        }
    }
    pub fn ceiling_height(&self, side_distance: f32) -> f32 {
        let outer = (side_distance / (self.width * 0.5) - 0.52).max(0.0) / 0.48;
        self.height - (self.height - 2.25) * outer.clamp(0.0, 1.0)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub generation_ms: f64,
    pub rooms: usize,
    pub critical_length: usize,
    pub branches: usize,
    pub loops: usize,
    pub vertical_connections: usize,
    pub stacked_rooms: usize,
    pub volume: f32,
    pub rejected_attempts: usize,
    pub attempts: usize,
    pub local_repairs: usize,
    pub score: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Obstacle {
    pub room: usize,
    pub center: Vec3,
    pub half: [f32; 2],
    pub height: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Blueprint {
    pub seed: u64,
    pub version: u32,
    pub rules_hash: u64,
    pub config: Config,
    pub rooms: Vec<Room>,
    pub connections: Vec<Connection>,
    pub spawns: Vec<gameplay::Spawn>,
    pub obstacles: Vec<Obstacle>,
    pub critical_path: Vec<usize>,
    pub entrance: usize,
    pub exit: usize,
    pub stats: Stats,
    /// A hand-made level's floors (see [`floors`]): when present they alone say where one can stand
    /// and which room a spot belongs to; rooms and connections only name and group the places.
    #[serde(skip)]
    pub floors: Option<std::sync::Arc<FloorGrid>>,
}
impl Blueprint {
    pub fn structural_hash(&self) -> u64 {
        // Explicit stable fields: no elapsed time, gameplay, decoration or UI state.
        rng::hash(
            &serde_json::to_vec(&(
                self.version,
                self.seed,
                &self.rooms,
                &self.connections,
                &self.obstacles,
                &self.critical_path,
            ))
            .unwrap(),
        )
    }
    pub fn spawn(&self) -> Vec3 {
        self.rooms[self.entrance].center
    }
    pub fn neighbors(&self, id: usize) -> impl Iterator<Item = (usize, &Connection)> {
        self.connections.iter().filter_map(move |c| {
            if c.a == id {
                Some((c.b, c))
            } else if c.b == id {
                Some((c.a, c))
            } else {
                None
            }
        })
    }
    pub fn room_at(&self, p: Vec3) -> Option<usize> {
        if let Some(grid) = &self.floors {
            return grid
                .room_at(p.x, p.y, p.z)
                .filter(|&r| r < self.rooms.len());
        }
        self.rooms
            .iter()
            .find(|r| {
                r.contains(p.x, p.z, -0.02)
                    && p.y >= r.center.y - 4.2
                    && p.y < r.center.y + r.height
            })
            .map(|r| r.id)
    }
    pub fn region_at(&self, p: Vec3) -> usize {
        self.room_at(p).unwrap_or_else(|| {
            self.connections
                .iter()
                .min_by(|a, b| {
                    let a = (a.start + a.end) * 0.5;
                    let b = (b.start + b.end) * 0.5;
                    (a - p)
                        .length_squared()
                        .total_cmp(&(b - p).length_squared())
                })
                .map(|c| {
                    if (p - c.start).length_squared() < (p - c.end).length_squared() {
                        c.a
                    } else {
                        c.b
                    }
                })
                .unwrap_or(self.entrance)
        })
    }
    /// Floor under feet with body clearance. Highest compatible layer wins,
    /// not the highest XZ floor: stacked rooms must not teleport the player.
    pub fn surface(&self, x: f32, z: f32, feet: f32, body: f32) -> Option<f32> {
        if let Some(grid) = &self.floors {
            return grid.surface(x, z, feet, body);
        }
        let mut best: Option<f32> = None;
        let mut consider = |floor: f32, ceiling: f32| {
            if floor <= feet + STEP_HEIGHT + 0.015
                && ceiling >= feet + body - 0.01
                && best.is_none_or(|v| floor > v)
            {
                best = Some(floor);
            }
        };
        for r in &self.rooms {
            if r.contains(x, z, -0.008) {
                consider(r.floor_y(x, z), r.center.y + r.height);
            }
        }
        for c in &self.connections {
            let (t, d) = c.project(x, z);
            if (-0.001..=1.001).contains(&t) && d <= c.width * 0.5 + 0.008 {
                let y = c.floor_y(t);
                consider(
                    y,
                    c.start.y + (c.end.y - c.start.y) * t + c.ceiling_height(d),
                );
            }
        }
        best
    }
    /// The highest ledge one walks up here without jumping.
    pub fn step_height(&self) -> f32 {
        self.floors.as_ref().map_or(STEP_HEIGHT, |grid| grid.step)
    }
    /// Whether a body `radius` wide standing with its feet at `p` hits one of the loose obstacles
    /// (the same test [`capsule_floor`](Self::capsule_floor) makes).
    pub fn hits_obstacle(&self, p: Vec3, radius: f32) -> bool {
        let radius = match &self.floors {
            Some(grid) => (radius - grid.margin).max(0.06),
            None => radius,
        };
        self.obstacles.iter().any(|o| {
            let dx = (p.x - o.center.x).abs() - o.half[0];
            let dz = (p.z - o.center.z).abs() - o.half[1];
            dx.max(0.0).powi(2) + dz.max(0.0).powi(2) < radius * radius
                && p.y < o.center.y + o.height
                && p.y + PLAYER_HEIGHT > o.center.y
        })
    }

    /// A walk over a hand-made level's floors (see [`FloorGrid::path`]) clear of the loose
    /// obstacles too; empty without floors or when there is none.
    pub fn walk_path(&self, start: Vec3, goal: Vec3, radius: f32, reach: f32) -> Vec<Vec3> {
        let Some(grid) = &self.floors else {
            return Vec::new();
        };
        // a little wider than the body, so the walk passes them, not grazes them
        grid.path_avoiding(start, goal, radius, reach, &|p| {
            self.hits_obstacle(p, radius + 0.15)
        })
    }

    pub fn capsule_floor(&self, p: Vec3, radius: f32) -> Option<f32> {
        // Over hand-made floors the walls already stand back by the bake's margin.
        let radius = match &self.floors {
            Some(grid) => (radius - grid.margin).max(0.06),
            None => radius,
        };
        for o in &self.obstacles {
            let dx = (p.x - o.center.x).abs() - o.half[0];
            let dz = (p.z - o.center.z).abs() - o.half[1];
            if dx.max(0.0).powi(2) + dz.max(0.0).powi(2) < radius * radius
                && p.y < o.center.y + o.height
                && p.y + PLAYER_HEIGHT > o.center.y
            {
                return None;
            }
        }
        let mut y = self.surface(p.x, p.z, p.y, PLAYER_HEIGHT)?;
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            y = y.max(self.surface(
                p.x + a.cos() * radius,
                p.z + a.sin() * radius,
                p.y,
                PLAYER_HEIGHT,
            )?);
        }
        Some(y)
    }
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        let n = ((b - a).length() / 0.3).ceil() as usize;
        (0..=n).all(|i| {
            let p = a.lerp(b, i as f32 / n.max(1) as f32);
            self.surface(p.x, p.z, p.y, 0.02)
                .is_some_and(|y| p.y >= y && p.y - y < 8.0)
                && !self.obstacles.iter().any(|o| {
                    (p.x - o.center.x).abs() < o.half[0]
                        && (p.z - o.center.z).abs() < o.half[1]
                        && p.y > o.center.y
                        && p.y < o.center.y + o.height
                })
        })
    }
    pub fn route(&self, from: usize, to: usize) -> Vec<usize> {
        let mut prev = vec![usize::MAX; self.rooms.len()];
        let mut q = std::collections::VecDeque::from([from]);
        prev[from] = from;
        while let Some(id) = q.pop_front() {
            if id == to {
                break;
            }
            for (n, _) in self.neighbors(id) {
                if prev[n] == usize::MAX {
                    prev[n] = id;
                    q.push_back(n);
                }
            }
        }
        if prev[to] == usize::MAX {
            return vec![];
        }
        let mut path = vec![to];
        while *path.last().unwrap() != from {
            path.push(prev[*path.last().unwrap()]);
        }
        path.reverse();
        path
    }
    pub fn route_points(&self, from: usize, to: usize) -> Vec<Vec3> {
        let route = self.route(from, to);
        let mut points = vec![self.rooms[from].center];
        for pair in route.windows(2) {
            let c = self
                .connections
                .iter()
                .find(|c| (c.a == pair[0] && c.b == pair[1]) || (c.b == pair[0] && c.a == pair[1]))
                .unwrap();
            if c.a == pair[0] {
                points.extend([c.start, c.end]);
            } else {
                points.extend([c.end, c.start]);
            }
            points.push(self.rooms[pair[1]].center);
        }
        points
    }
}
pub fn horizontal(v: Vec3) -> Vec3 {
    Vec3::new(v.x, 0.0, v.z)
}
