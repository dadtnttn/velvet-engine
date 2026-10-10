//! Los Baños termales: the hot springs behind the villa's bath house, reached through its door.
//!
//! The valley was designed and built in the Three.js prototype (`demos/demon-queen-baths-threejs`)
//! and exported by its `tools/export-game.mjs`: the baked GLB parts live in
//! `data/assets/3d/baths/`, the plan in `data/baths/baths.json`. The plan is in the prototype's own
//! frame (the springs are drawn alone, so nothing else shares it); its two rooms stand far off at
//! their own origin. This module reads the plan, walks the Queen through the valley (she can wade
//! into every pool) and its rooms, simulates the pools with the renderer's water, and draws it.
use super::*;
use serde::Deserialize;
use std::collections::BTreeMap;
use velvet_render::{WaterBodyHandle, WaterStreamHandle};

/// Her radius against posts, rocks and walls.
const BODY: f32 = 0.3;
/// How many of the lamps nearest her light the valley at once.
const LIGHTS: usize = 20;

// ------------------------------------------------------------------ the plan
#[derive(Deserialize)]
pub(crate) struct Pool {
    #[allow(dead_code)]
    pub id: String,
    pub x: f32,
    pub z: f32,
    pub rx: f32,
    pub rz: f32,
    pub seed: f32,
    #[allow(dead_code)]
    pub depth: f32,
    pub level: f32,
    pub fall: Option<Fall>,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Fall {
    pub x: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Deserialize)]
pub(crate) struct PathLine {
    #[allow(dead_code)]
    pub id: String,
    pub width: f32,
    pub points: Vec<[f32; 2]>,
}

#[derive(Deserialize)]
pub(crate) struct Building {
    #[allow(dead_code)]
    pub id: String,
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub d: f32,
}

#[derive(Deserialize)]
struct Heights {
    x0: f32,
    z0: f32,
    step: f32,
    nx: usize,
    nz: usize,
    values: Vec<f32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Hotspot {
    #[allow(dead_code)]
    pub id: String,
    pub label: String,
    pub icon: String,
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    pub text: String,
    #[serde(default)]
    pub to_village: bool,
    #[serde(default)]
    pub enter: Option<String>,
    #[serde(default)]
    pub exit: bool,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Point {
    pub x: f32,
    pub z: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Lamp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Collider {
    x: f32,
    z: f32,
    w: f32,
    d: f32,
}

#[derive(Deserialize)]
pub(crate) struct Room {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub title: String,
    pub w: f32,
    pub d: f32,
    pub center: Point,
    pub spawn: Point,
    colliders: Vec<Collider>,
    pub lamps: Vec<Lamp>,
    pub hotspots: Vec<Hotspot>,
}

#[derive(Deserialize)]
pub(crate) struct Plant {
    pub model: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub s: f32,
    pub yaw: f32,
    #[serde(default)]
    pub tilt: f32,
}

#[derive(Deserialize)]
pub(crate) struct Plan {
    pub half: f32,
    pub pools: Vec<Pool>,
    pub paths: Vec<PathLine>,
    pub buildings: Vec<Building>,
    circles: Vec<[f32; 3]>,
    heights: Heights,
    pub hotspots: Vec<Hotspot>,
    pub lamps: Vec<Lamp>,
    pub plants: Vec<Plant>,
    pub rooms: BTreeMap<String, Room>,
    /// The channels between the pools (their course and width).
    #[serde(default)]
    pub channels: Vec<Channel>,
    /// Decks, bridges, their steps and abutments: [i, j, height] on the heights' grid.
    #[serde(default)]
    floors: Vec<[f32; 3]>,
    /// The spa girls (spa_girl.glb): sitting in the pools, swimming, on the loungers.
    #[serde(default, rename = "spaGirls")]
    pub spa_girls: Vec<SpaGirl>,
    #[serde(skip)]
    floor_grid: OnceLock<Vec<f32>>,
}

#[derive(Deserialize)]
pub(crate) struct Channel {
    #[allow(dead_code)]
    pub id: String,
    pub width: f32,
    /// [x, z, level in, level out].
    pub points: Vec<[f32; 4]>,
}

/// The springs' placed sounds: bubbles in every pool, the falls splashing, the deer-scarer's
/// clack, now and then a bather shifting in the water. Indoors, the private bath's tub.
pub(crate) fn emitters(room: Option<&str>) -> &'static [super::game_audio::Emitter] {
    use super::game_audio::ambience_amb as Amb;
    use super::game_audio::Emitter;
    static VALLEY: OnceLock<Vec<Emitter>> = OnceLock::new();
    static ROOM: OnceLock<Vec<Emitter>> = OnceLock::new();
    if room.is_some() {
        return ROOM.get_or_init(|| {
            let p = plan();
            p.rooms
                .get("privada")
                .map(|r| {
                    vec![Emitter {
                        amb: Amb::Bubble,
                        at: Vec3::new(r.center.x + 2.6, 0.6, r.center.z - 2.9),
                        every: (2.5, 6.0),
                        volume: 0.35,
                        radius: 3.0,
                        level: 0,
                    }]
                })
                .unwrap_or_default()
        });
    }
    VALLEY.get_or_init(|| {
        let p = plan();
        let mut out = Vec::new();
        for pool in &p.pools {
            out.push(Emitter {
                amb: Amb::Bubble,
                at: Vec3::new(pool.x, pool.level, pool.z),
                every: (2.0, 5.5),
                volume: 0.4,
                radius: 4.0,
                level: 0,
            });
            out.push(Emitter {
                amb: Amb::Slosh,
                at: Vec3::new(pool.x, pool.level, pool.z),
                every: (14.0, 32.0),
                volume: 0.35,
                radius: 5.0,
                level: 0,
            });
            if let Some(f) = pool.fall {
                out.push(Emitter {
                    amb: Amb::Splash,
                    at: Vec3::new(f.x, pool.level, f.z),
                    every: (2.5, 5.0),
                    volume: 0.45,
                    radius: 5.0,
                    level: 0,
                });
            }
        }
        // The deer-scarer by the upper terrace (where the prototype puts it).
        if let Some(upper) = p.pools.iter().find(|q| q.id == "pozas-alta") {
            out.push(Emitter {
                amb: Amb::Clack,
                at: Vec3::new(upper.x + 4.5, upper.level + 1.0, upper.z + 3.5),
                every: (9.0, 14.0),
                volume: 0.55,
                radius: 6.0,
                level: 0,
            });
        }
        out
    })
}

/// One spa girl: a clip of spa_girl.glb, where she is (or the loop she swims), her bow's hue.
#[derive(Deserialize, Clone)]
pub(crate) struct SpaGirl {
    pub clip: String,
    /// The hue her bow is turned to (0..1); none keeps the model's pink.
    #[serde(default)]
    pub bow: Option<f32>,
    pub scale: f32,
    #[serde(default)]
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub z: f32,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub swim: Option<SwimLoop>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SwimLoop {
    /// The loop her hips follow.
    pub points: Vec<[f32; 2]>,
    /// Metres a second.
    pub speed: f32,
    /// Which way her head lies in the model (the yaw that turns it forward).
    pub head_yaw: f32,
    /// Her hips in the model's frame, scaled (x, z): the loop carries them, not the origin.
    pub hips: [f32; 2],
}

impl SwimLoop {
    /// Where her hips are `along` metres round the loop, and the way she heads.
    pub(crate) fn at(&self, along: f32) -> (Vec3, f32) {
        let n = self.points.len();
        let seg = |i: usize| {
            let a = self.points[i % n];
            let b = self.points[(i + 1) % n];
            (a, b, (b[0] - a[0]).hypot(b[1] - a[1]))
        };
        let total: f32 = (0..n).map(|i| seg(i).2).sum();
        let mut left = along.rem_euclid(total.max(1e-3));
        for i in 0..n {
            let (a, b, len) = seg(i);
            if left <= len || i == n - 1 {
                let t = (left / len.max(1e-4)).clamp(0.0, 1.0);
                let p = Vec3::new(a[0] + (b[0] - a[0]) * t, 0.0, a[1] + (b[1] - a[1]) * t);
                return (p, (b[0] - a[0]).atan2(b[1] - a[1]));
            }
            left -= len;
        }
        (Vec3::ZERO, 0.0)
    }
}

/// In place, RGBA bytes: the bow's saturated pinks take `hue` (0..1), keeping their shade. (Kept
/// in step with the prototype's `recolourBow`.)
pub(crate) fn recolour_bow(rgba: &mut [u8], hue: f32) {
    for px in rgba.chunks_exact_mut(4) {
        let (r, g, b) = (
            px[0] as f32 / 255.0,
            px[1] as f32 / 255.0,
            px[2] as f32 / 255.0,
        );
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let s = if max > 0.0 { (max - min) / max } else { 0.0 };
        if s < 0.35 || max < 0.45 || r < g || r < b {
            continue;
        }
        let h = if max > min {
            ((g - b) / (max - min) + 6.0) % 6.0 / 6.0
        } else {
            0.0
        };
        if !(h > 0.9 || h < 0.03) {
            continue;
        }
        // Into HSL, the hue swapped, back.
        let l = (max + min) / 2.0;
        let d = max - min;
        let sl = if d == 0.0 {
            0.0
        } else {
            d / (1.0 - (2.0 * l - 1.0).abs())
        };
        let q = if l < 0.5 {
            l * (1.0 + sl)
        } else {
            l + sl - l * sl
        };
        let p = 2.0 * l - q;
        let channel = |t: f32| {
            let t = t.rem_euclid(1.0);
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };
        px[0] = (channel(hue + 1.0 / 3.0) * 255.0).round().clamp(0.0, 255.0) as u8;
        px[1] = (channel(hue) * 255.0).round().clamp(0.0, 255.0) as u8;
        px[2] = (channel(hue - 1.0 / 3.0) * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

pub(crate) fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("baths/baths.json")).expect("valid baths plan")
    })
}

fn asset_root() -> PathBuf {
    runtime_3d_root().join("baths")
}

// ------------------------------------------------------------- geometry
fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Pool {
    fn blob(&self, angle: f32) -> f32 {
        let s = self.seed;
        1.0 + 0.12 * (2.0 * angle + s).sin()
            + 0.08 * (3.0 * angle + s * 1.7).sin()
            + 0.05 * (5.0 * angle + s * 2.3).sin()
    }

    /// 0 at the pool's middle, 1 on its shore, more outside.
    pub(crate) fn t(&self, x: f32, z: f32) -> f32 {
        let (dx, dz) = (x - self.x, z - self.z);
        (dx / self.rx).hypot(dz / self.rz) / self.blob(dz.atan2(dx))
    }

    /// The shore at an angle, for the map.
    pub(crate) fn shore(&self, angle: f32) -> (f32, f32) {
        let r = self.blob(angle);
        (
            self.x + angle.cos() * self.rx * r,
            self.z + angle.sin() * self.rz * r,
        )
    }
}

impl Plan {
    /// The bare ground's height (the prototype's `groundHeight`, on its quarter-metre grid).
    pub(crate) fn ground(&self, x: f32, z: f32) -> f32 {
        let h = &self.heights;
        let fx = ((x - h.x0) / h.step).clamp(0.0, (h.nx - 1) as f32 - 1e-3);
        let fz = ((z - h.z0) / h.step).clamp(0.0, (h.nz - 1) as f32 - 1e-3);
        let (i, j) = (fx.floor() as usize, fz.floor() as usize);
        let (u, v) = (fx - i as f32, fz - j as f32);
        let at = |i: usize, j: usize| h.values[j * h.nx + i];
        let a = at(i, j) + (at(i + 1, j) - at(i, j)) * u;
        let b = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * u;
        a + (b - a) * v
    }

    /// How far into a pool (x, z) is: 0 out of the water, 1 in its middle.
    pub(crate) fn in_pool(&self, x: f32, z: f32) -> f32 {
        self.pools
            .iter()
            .map(|p| 1.0 - smooth(0.55, 1.0, p.t(x, z)))
            .fold(0.0, f32::max)
    }

    /// The pool (x, z) is in, if any.
    pub(crate) fn pool_at(&self, x: f32, z: f32) -> Option<&Pool> {
        self.pools.iter().find(|p| p.t(x, z) < 1.0)
    }

    /// The top of a deck, a bridge or a step at (x, z), if she is on one.
    pub(crate) fn floor(&self, x: f32, z: f32) -> Option<f32> {
        let h = &self.heights;
        let grid = self.floor_grid.get_or_init(|| {
            let mut g = vec![f32::NAN; h.nx * h.nz];
            for &[i, j, v] in &self.floors {
                let (i, j) = (i as usize, j as usize);
                if i < h.nx && j < h.nz {
                    g[j * h.nx + i] = v;
                }
            }
            g
        });
        let fx = (x - h.x0) / h.step;
        let fz = (z - h.z0) / h.step;
        if fx < 0.0 || fz < 0.0 || fx > (h.nx - 1) as f32 || fz > (h.nz - 1) as f32 {
            return None;
        }
        let at = |i: usize, j: usize| grid[j.min(h.nz - 1) * h.nx + i.min(h.nx - 1)];
        // On a floor only where its nearest sample is; its height blended from the samples round
        // it that are floor too.
        if at(fx.round() as usize, fz.round() as usize).is_nan() {
            return None;
        }
        let (i, j) = (fx.floor() as usize, fz.floor() as usize);
        let (u, v) = (fx - i as f32, fz - j as f32);
        let (mut sum, mut weight) = (0.0, 0.0);
        for (di, dj, w) in [
            (0, 0, (1.0 - u) * (1.0 - v)),
            (1, 0, u * (1.0 - v)),
            (0, 1, (1.0 - u) * v),
            (1, 1, u * v),
        ] {
            let value = at(i + di, j + dj);
            if !value.is_nan() {
                sum += value * w;
                weight += w;
            }
        }
        (weight > 1e-4).then(|| sum / weight)
    }

    /// Where her feet stand outdoors: on a deck or a bridge when she is on one; else the ground, a
    /// little lower in a pool's bed.
    pub(crate) fn stand(&self, x: f32, z: f32) -> f32 {
        let ground = self.ground(x, z) - 0.1 * self.in_pool(x, z);
        match self.floor(x, z) {
            Some(f) if f > ground => f,
            _ => ground,
        }
    }

    /// What her feet are on (for the sound of her steps).
    pub(crate) fn ground_kind(&self, x: f32, z: f32) -> super::game_audio::Ground {
        use super::game_audio::Ground;
        if let Some(f) = self.floor(x, z) {
            if f > self.ground(x, z) - 0.1 * self.in_pool(x, z) {
                // Boards, or the stone of an abutment or a stair (stone sits lower than planks).
                return if self
                    .paths
                    .iter()
                    .any(|p| p.id == "alta" || p.id == "cubierta")
                    && f - self.ground(x, z) < 0.02
                {
                    Ground::Stone
                } else {
                    Ground::Wood
                };
            }
        }
        if self.in_pool(x, z) > 0.15 || self.in_channel(x, z) {
            return Ground::Water;
        }
        if self.on_path(x, z) {
            Ground::Stone
        } else {
            Ground::Grass
        }
    }

    fn on_path(&self, x: f32, z: f32) -> bool {
        self.paths.iter().any(|p| {
            p.points.windows(2).any(|w| {
                let (a, b) = (w[0], w[1]);
                let (abx, abz) = (b[0] - a[0], b[1] - a[1]);
                let len2 = (abx * abx + abz * abz).max(1e-6);
                let t = (((x - a[0]) * abx + (z - a[1]) * abz) / len2).clamp(0.0, 1.0);
                (x - (a[0] + abx * t)).hypot(z - (a[1] + abz * t)) < p.width * 0.5
            })
        })
    }

    /// Wading in one of the channels (its bed under the water's level).
    fn in_channel(&self, x: f32, z: f32) -> bool {
        self.channels.iter().any(|c| {
            c.points.windows(2).any(|w| {
                let (a, b) = (w[0], w[1]);
                let (abx, abz) = (b[0] - a[0], b[1] - a[1]);
                let len2 = (abx * abx + abz * abz).max(1e-6);
                let t = (((x - a[0]) * abx + (z - a[1]) * abz) / len2).clamp(0.0, 1.0);
                (x - (a[0] + abx * t)).hypot(z - (a[1] + abz * t)) < c.width * 0.45
            })
        })
    }

    /// Is she up on a deck, a bridge or a step (not in the water under it)?
    pub(crate) fn on_floor(&self, x: f32, z: f32) -> bool {
        self.floor(x, z)
            .is_some_and(|f| f > self.ground(x, z) - 0.1 * self.in_pool(x, z))
    }

    /// Can she stand at (x, z) outdoors? She wades into the pools; posts, rocks, people and
    /// houses stop her, and so do the valley's walls.
    pub(crate) fn blocked_outdoor(&self, x: f32, z: f32, r: f32) -> bool {
        if (x * 0.86).hypot(z) > self.half - 3.4 {
            return true;
        }
        if self
            .buildings
            .iter()
            .any(|b| (x - b.x).abs() < b.w / 2.0 + r && (z - b.z).abs() < b.d / 2.0 + r)
        {
            return true;
        }
        self.circles
            .iter()
            .any(|c| (x - c[0]).hypot(z - c[1]) < c[2] + r)
    }
}

impl Room {
    /// Out of the room, or in something (for the QA walker).
    pub(crate) fn blocked_public(&self, x: f32, z: f32, r: f32) -> bool {
        self.blocked(x, z, r)
    }

    fn blocked(&self, x: f32, z: f32, r: f32) -> bool {
        if (x - self.center.x).abs() > self.w / 2.0 - 0.5
            || (z - self.center.z).abs() > self.d / 2.0 - 0.5
        {
            return true;
        }
        self.colliders
            .iter()
            .any(|c| (x - c.x).abs() < c.w / 2.0 + r && (z - c.z).abs() < c.d / 2.0 + r)
    }
}

pub(crate) fn icon(name: &str) -> Icon {
    match name {
        "bell" => Icon::Council,
        "spa" => Icon::Bath,
        "lounge" | "bed" => Icon::Rest,
        "gate" => Icon::House,
        "tea" => Icon::Tavern,
        "locker" => Icon::Chest,
        _ => Icon::Door,
    }
}

// ---------------------------------------------------------------- the visit
/// What pressing F did, for the explorer to carry out.
pub(crate) enum Action {
    None,
    /// Back out to the villa.
    Leave,
    /// Something to read: the Queen's thought.
    Say(&'static str),
}

/// The springs as the Queen experiences them: out in the valley or in one of its rooms.
#[derive(Clone, Debug)]
pub(crate) struct State {
    /// The room she is in (`None`: the valley).
    pub room: Option<String>,
    pub x: f32,
    pub z: f32,
    /// Where to put her back in the villa when she leaves.
    pub return_to: Vec3,
    pub return_yaw: f32,
    /// The hour of the world's day (0..24), for the girls' routines.
    pub hour: f32,
    /// The girls standing about (x, z, radius): she walks round them.
    pub others: Vec<[f32; 3]>,
    /// Soaking in a pool, or lying on a lounger.
    pub relax: Relax,
    /// Seconds she has stood still (in a pool she sits back after a moment).
    pub still: f32,
    /// How far into the change between standing and relaxing (0 standing, 1 relaxed).
    pub relax_blend: f32,
    /// The photo spot Mugo is leading her to (`mugo::PHOTO_SPOTS`); `None`: he keeps his post.
    pub guide: Option<usize>,
    /// Where Mugo is (x, z) and whether he is at the spot, pointing (from the scene).
    pub mugo: Option<([f32; 2], bool)>,
}

/// The Queen taking it easy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Relax {
    None,
    /// Sitting back in the water.
    Soaking,
    /// On a lounger: where it is and the way her head goes.
    Lying {
        x: f32,
        z: f32,
        y: f32,
        yaw: f32,
    },
}

/// The loungers she can lie on: (x, z, top, the turn of a body lying on it).
pub(crate) fn loungers() -> Vec<(f32, f32, f32, f32)> {
    plan()
        .spa_girls
        .iter()
        .filter(|g| g.clip == "Spa_Lie")
        .map(|g| (g.x, g.z, plan().ground(g.x, g.z) + 0.47, g.yaw))
        .collect()
}

impl State {
    pub(crate) fn enter(return_to: Vec3, return_yaw: f32) -> Self {
        // In through the gate, the path ahead of her.
        Self {
            room: None,
            x: -8.2,
            z: 10.6,
            return_to,
            return_yaw,
            hour: 12.0,
            others: Vec::new(),
            relax: Relax::None,
            still: 0.0,
            relax_blend: 0.0,
            guide: None,
            mugo: None,
        }
    }

    /// Standing where Mugo has brought her, with him pointing at the view: the spot.
    pub(crate) fn photo_here(&self) -> Option<usize> {
        if self.room.is_some() {
            return None;
        }
        let spot = self.guide?;
        let (_, there) = self.mugo?;
        let [x, z] = super::mugo::PHOTO_SPOTS.get(spot)?.stand();
        (there && (self.x - x).hypot(self.z - z) < 1.6).then_some(spot)
    }

    /// Close enough to Mugo to talk to him.
    pub(crate) fn mugo_near(&self) -> bool {
        self.room.is_none()
            && self
                .mugo
                .is_some_and(|([x, z], _)| (self.x - x).hypot(self.z - z) < 1.7)
    }

    fn room_plan(&self) -> Option<&'static Room> {
        self.room.as_deref().and_then(|id| plan().rooms.get(id))
    }

    pub(crate) fn position(&self) -> Vec3 {
        let y = if self.room.is_some() {
            0.0
        } else {
            plan().stand(self.x, self.z)
        };
        Vec3::new(self.x, y, self.z)
    }

    pub(crate) fn wading(&self) -> f32 {
        if self.room.is_some() || plan().on_floor(self.x, self.z) {
            0.0
        } else {
            plan().in_pool(self.x, self.z)
        }
    }

    pub(crate) fn walk(&mut self, dx: f32, dz: f32) {
        let p = plan();
        let room = self.room_plan();
        let blocked = |x: f32, z: f32| match room {
            Some(r) => r.blocked(x, z, BODY),
            None => {
                p.blocked_outdoor(x, z, BODY)
                    || self
                        .others
                        .iter()
                        .any(|o| (x - o[0]).hypot(z - o[1]) < o[2] + BODY)
            }
        };
        let (x, z) = (self.x, self.z);
        // Sliding along an axis counts only when it gets her somewhere; else she eases round the
        // corner; else the little slide there is.
        let len = dx.hypot(dz);
        let real = |v: f32| v.abs() > 0.2 * len;
        if !blocked(x + dx, z + dz) {
            (self.x, self.z) = (x + dx, z + dz);
        } else if real(dx) && !blocked(x + dx, z) {
            self.x = x + dx;
        } else if real(dz) && !blocked(x, z + dz) {
            self.z = z + dz;
        } else if let Some((ax, az)) =
            super::corner_assist(dx, dz, |ax, az| !blocked(x + ax, z + az))
        {
            (self.x, self.z) = (x + ax, z + az);
        } else if !blocked(x + dx, z) {
            self.x = x + dx;
        } else if !blocked(x, z + dz) {
            self.z = z + dz;
        }
    }

    fn hotspots(&self) -> &'static [Hotspot] {
        match self.room_plan() {
            Some(r) => &r.hotspots,
            None => &plan().hotspots,
        }
    }

    pub(crate) fn nearest(&self) -> Option<&'static Hotspot> {
        self.hotspots()
            .iter()
            .map(|h| (h, (h.x - self.x).hypot(h.z - self.z)))
            .filter(|(h, d)| *d < h.radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    /// A free lounger within reach (none in a room or while she relaxes).
    fn lounger_near(&self) -> Option<(f32, f32, f32, f32)> {
        if self.room.is_some() || self.relax != Relax::None {
            return None;
        }
        loungers().into_iter().find(|&(x, z, ..)| {
            (x - self.x).hypot(z - self.z) < 1.4
                && !self.others.iter().any(|o| (o[0] - x).hypot(o[1] - z) < 0.8)
        })
    }

    pub(crate) fn prompt(&self) -> Option<Prompt> {
        if let Relax::Lying { .. } = self.relax {
            return Some(Prompt::new(Icon::Bath, "Levantarse").detail("Tumbona"));
        }
        if let Some(spot) = self.photo_here() {
            return Some(
                Prompt::new(Icon::Memory, "Hacer una foto")
                    .detail(super::mugo::PHOTO_SPOTS[spot].name),
            );
        }
        if self.mugo_near() {
            return Some(Prompt::new(Icon::Talk, "Hablar").detail("Mugo"));
        }
        if self.lounger_near().is_some() {
            return Some(Prompt::new(Icon::Bath, "Tumbarse").detail("Tumbona al sol"));
        }
        let h = self.nearest()?;
        let (verb, detail) = if h.to_village {
            ("Volver a la villa", "El portón".to_owned())
        } else if h.exit {
            ("Salir", "A las pozas".to_owned())
        } else if h.enter.is_some() {
            ("Entrar", h.label.clone())
        } else {
            ("Mirar", h.label.clone())
        };
        Some(Prompt::new(icon(&h.icon), verb).detail(detail))
    }

    pub(crate) fn interact(&mut self) -> Action {
        if let Relax::Lying { .. } = self.relax {
            self.relax = Relax::None;
            return Action::None;
        }
        if let Some((x, z, y, yaw)) = self.lounger_near() {
            self.relax = Relax::Lying { x, z, y, yaw };
            return Action::Say("Un rato al sol, entre vapor y pinos.");
        }
        let Some(h) = self.nearest() else {
            return Action::None;
        };
        if h.to_village {
            return Action::Leave;
        }
        if let Some(id) = h.enter.as_deref() {
            if let Some(room) = plan().rooms.get(id) {
                self.room = Some(id.to_owned());
                (self.x, self.z) = (room.spawn.x, room.spawn.z);
            }
            return Action::None;
        }
        if h.exit {
            // Back out at that house's door.
            let id = self.room.take();
            if let Some(door) = plan()
                .hotspots
                .iter()
                .find(|o| o.enter.as_deref() == id.as_deref())
            {
                (self.x, self.z) = (door.x + 1.0, door.z);
            }
            return Action::None;
        }
        Action::Say(h.text.as_str())
    }

    /// The lamps that light where she is.
    pub(crate) fn lights(&self, elapsed: f32, moon_visibility: f32) -> Vec<PointLight3D> {
        if let Some(room) = self.room_plan() {
            return room
                .lamps
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let breath = 1.0 + (elapsed * 3.0 + i as f32 * 1.7).sin() * 0.04;
                    PointLight3D::new(
                        Vec3::new(l.x, l.y, l.z),
                        [1.0, 0.56, 0.24],
                        6.5 * breath,
                        6.0,
                    )
                })
                .collect();
        }
        let night = ((moon_visibility - 0.12) / 0.48).clamp(0.0, 1.0);
        let mut lamps: Vec<(usize, &Lamp)> = plan().lamps.iter().enumerate().collect();
        lamps.sort_by(|a, b| {
            (a.1.x - self.x)
                .hypot(a.1.z - self.z)
                .total_cmp(&(b.1.x - self.x).hypot(b.1.z - self.z))
        });
        lamps
            .into_iter()
            .take(LIGHTS)
            .map(|(i, l)| {
                let fade = 1.0 - smooth(16.0, 24.0, (l.x - self.x).hypot(l.z - self.z));
                let breath = 0.94 + (elapsed * 3.1 + i as f32 * 1.73).sin() * 0.035;
                PointLight3D::new(
                    Vec3::new(l.x, l.y, l.z),
                    [1.0, 0.62, 0.3],
                    3.6 * (0.1 + 0.9 * night) * breath * fade,
                    6.5,
                )
            })
            .collect()
    }
}

/// Indoors by lamplight: a warm fill, a soft key, no haze.
pub(crate) fn room_mood(mut env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    env.lighting.key_color = [0.62, 0.48, 0.34];
    env.lighting.ambient_color = [0.26, 0.19, 0.15];
    env.lighting.bounce_color = [0.30, 0.16, 0.09];
    env.atmosphere.fog_start = 60.0;
    env.atmosphere.fog_end = 120.0;
    env.atmosphere.fog_density = 0.0;
    env.atmosphere.height_falloff = 0.0;
    env.clear_color = [0.02, 0.012, 0.01, 1.0];
    env.celestial.moon_intensity = 0.0;
    env.celestial.reflection_strength = 0.0;
    env.celestial.horizon_strength = 0.0;
    env
}

// ------------------------------------------------------------------ scene
/// Where a set of draws belongs.
#[derive(Clone, PartialEq)]
enum Place {
    Valley,
    Room(String),
}

struct Water {
    handle: WaterBodyHandle,
    place: Place,
}

/// The springs' GPU side.
pub(crate) struct Scene {
    /// The spa girls' day.
    life: Option<super::spa_life::SpaLife>,
    /// Mugo, keeping his post or showing her the spots for photos.
    mugo: Option<super::mugo::Mugo>,
    /// This frame's girls indoors, ready to draw.
    room_girls: Vec<MeshDraw3D>,
    /// Where the Queen was last frame (her velocity, for the girls to make way).
    queen_last: Option<Vec3>,
    /// The capybaras of the lower pool (None if capybara.glb is missing: then the old ones stay).
    capybaras: Option<Capybaras>,

    valley: Vec<MeshDraw3D>,
    rooms: BTreeMap<String, Vec<MeshDraw3D>>,
    /// Lamp glass and lit windows among the valley's draws: lit with the night.
    lamps: Vec<usize>,
    water: Vec<Water>,
    /// The channels, the cascade and the waterfalls (all in the valley).
    streams: Vec<WaterStreamHandle>,
    puff: Option<MeshHandle>,
    falls: Vec<(Vec3, f32)>,
    last: Option<f32>,
    beat: f32,
    next_drop: f32,
    rng: u32,
    last_touch: Option<Vec3>,
}

fn load_draws(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    file: &str,
    cell: f32,
    lamps: &mut Vec<usize>,
    out: &mut Vec<MeshDraw3D>,
    drop_old_capybaras: bool,
) -> Result<()> {
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255; 4],
    };
    let path = asset_root().join(file);
    let model = super::prefetch::load(&path)
        .with_context(|| format!("load baths part: {}", path.display()))?;
    for (index, mesh) in model.meshes.iter().enumerate() {
        if drop_old_capybaras && is_old_capybara(mesh.material.base_color) {
            continue;
        }
        let name = model
            .mesh_names
            .get(index)
            .map(String::as_str)
            .unwrap_or("");
        let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
        let normal = mesh
            .normal_texture_index
            .and_then(|i| model.textures.get(i));
        let texture = upload_texture_pair(renderer, device, queue, albedo, normal, cache, &white)?;
        let lamp = name == "lampGlass";
        let material = if lamp {
            mesh.material.with_emission(1.2)
        } else {
            mesh.material.with_ambient_occlusion(0.18)
        };
        let transform = model.mesh_transform(index);
        for piece in mesh.split_into_cells(cell) {
            if lamp {
                lamps.push(out.len());
            }
            out.push(MeshDraw3D {
                mesh: renderer.upload_mesh(device, &piece)?,
                transform,
                material,
                texture,
            });
        }
    }
    Ok(())
}

// ------------------------------------------------------------- capybaras
/// The prototype's boxy capybaras, baked into baths_life.glb: their five colours (linear), so
/// they can be left out when the new ones (capybara.glb) load.
const OLD_CAPYBARA_COLOURS: [[f32; 3]; 5] = [
    [0.2705, 0.1144, 0.0452], // fur
    [0.1022, 0.0423, 0.0176], // muzzle and ears
    [0.0160, 0.0080, 0.0044], // eyes and nostrils
    [0.8879, 0.4452, 0.0423], // the yuzu on the head
    [0.0685, 0.2542, 0.0423], // its leaf
];

fn is_old_capybara(colour: [f32; 4]) -> bool {
    OLD_CAPYBARA_COLOURS
        .iter()
        .any(|c| c.iter().zip(colour).all(|(a, b)| (a - b).abs() < 2e-3))
}

#[derive(Clone, Copy, PartialEq)]
enum CapyPiece {
    Body,
    Head,
    /// Left (-1) or right (+1).
    Ear(f32),
    EyesClosed,
    EyesOpen,
    Yuzu,
}

struct CapyPart {
    piece: CapyPiece,
    mesh: MeshHandle,
    material: Material3D,
    texture: Option<TextureHandle>,
    local: Mat4,
}

struct Capybara {
    at: Vec3,
    yaw: f32,
    scale: f32,
    phase: f32,
    /// The little one rides on the back of `herd[mother]`.
    mother: Option<usize>,
}

/// Where the capybaras doze in the lower pool (the prototype's spots): (x, z, level, yaw, scale).
fn capybara_spots() -> Vec<(f32, f32, f32, f32, f32)> {
    let Some(low) = plan().pools.iter().find(|p| p.id == "pozas-baja") else {
        return Vec::new();
    };
    // The prototype's `shorePoint(pool, angle, t)`: a fraction t of the way to the shore.
    let spot = |angle: f32, t: f32| {
        let (sx, sz) = low.shore(angle);
        (low.x + (sx - low.x) * t, low.z + (sz - low.z) * t)
    };
    let (ax, az) = spot(2.6, 0.42);
    let (bx, bz) = spot(2.0, 0.38);
    vec![
        (ax, az, low.level, -0.4, 0.95),
        (bx, bz, low.level, 0.1, 0.8),
    ]
}

/// Where the first capybara dozes (for close-up captures).
pub(crate) fn capybara_focus() -> Option<(f32, f32)> {
    capybara_spots().first().map(|&(x, z, ..)| (x, z))
}

/// Two capybaras dozing up to their necks with a yuzu on the head, and a little one on its
/// mother's back. Built by tools/build_capybara.py; animated here, piece by piece.
pub(crate) struct Capybaras {
    parts: Vec<CapyPart>,
    neck: Vec3,
    herd: Vec<Capybara>,
    next_touch: Vec<f32>,
}

impl Capybaras {
    fn load(
        renderer: &mut Wgpu3DRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    ) -> Result<Self> {
        let white = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        let path = asset_root().join("capybara.glb");
        let model = super::prefetch::load(&path)
            .with_context(|| format!("load capybara: {}", path.display()))?;
        let mut parts = Vec::new();
        let mut neck = None;
        for (index, mesh) in model.meshes.iter().enumerate() {
            let piece = match model
                .mesh_names
                .get(index)
                .map(String::as_str)
                .unwrap_or("")
            {
                "capy_body" => CapyPiece::Body,
                "capy_head" => CapyPiece::Head,
                "capy_ear_L" => CapyPiece::Ear(-1.0),
                "capy_ear_R" => CapyPiece::Ear(1.0),
                "capy_eyes_closed" => CapyPiece::EyesClosed,
                "capy_eyes_open" => CapyPiece::EyesOpen,
                "capy_yuzu" => CapyPiece::Yuzu,
                _ => continue,
            };
            let local = model.mesh_transform(index);
            if piece == CapyPiece::Head {
                let w = local.w_axis;
                neck = Some(Vec3::new(w[0], w[1], w[2]));
            }
            let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
            let normal = mesh
                .normal_texture_index
                .and_then(|i| model.textures.get(i));
            let texture =
                upload_texture_pair(renderer, device, queue, albedo, normal, cache, &white)?;
            parts.push(CapyPart {
                piece,
                mesh: renderer.upload_mesh(device, mesh)?,
                material: mesh.material.with_ambient_occlusion(0.18),
                texture,
                local,
            });
        }
        let neck = neck.context("capybara.glb has no capy_head")?;
        let mut herd: Vec<Capybara> = capybara_spots()
            .into_iter()
            .enumerate()
            .map(|(i, (x, z, level, yaw, scale))| Capybara {
                at: Vec3::new(x, level, z),
                yaw,
                scale,
                phase: i as f32 * 2.3,
                mother: None,
            })
            .collect();
        anyhow::ensure!(!herd.is_empty(), "no lower pool for the capybaras");
        herd.push(Capybara {
            at: Vec3::ZERO,
            yaw: 0.25,
            scale: 0.42,
            phase: 4.1,
            mother: Some(0),
        });
        let next_touch = (0..herd.len()).map(|i| 1.0 + i as f32 * 1.7).collect();
        Ok(Self {
            parts,
            neck,
            herd,
            next_touch,
        })
    }

    /// Where capybara `i` floats this frame (the little one rides its mother's back).
    fn placement(&self, i: usize, t: f32) -> Mat4 {
        let c = &self.herd[i];
        match c.mother {
            None => {
                let bob = 0.012 * (t * 0.8 + c.phase).sin();
                let roll = 0.025 * (t * 0.5 + c.phase).sin();
                // Up to the neck: only the head and the top of the back above the water.
                let sink = 0.11 * c.scale;
                Mat4::from_translation(c.at + Vec3::new(0.0, bob - sink, 0.0))
                    * Mat4::from_rotation_y(c.yaw)
                    * Mat4::from_rotation_z(roll)
                    * Mat4::from_scale(Vec3::new(c.scale, c.scale, c.scale))
            }
            Some(m) => {
                // In her frame (model units): on top of the back, a little behind the shoulders.
                let rel = c.scale / self.herd[m].scale;
                self.placement(m, t)
                    * Mat4::from_translation(Vec3::new(0.0, 0.2 + 0.36 * rel, -0.15))
                    * Mat4::from_rotation_y(c.yaw)
                    * Mat4::from_scale(Vec3::new(rel, rel, rel))
            }
        }
    }

    /// Rings in the water now and then where the grown-ups shift.
    fn tick(&mut self, dt: f32, t: f32) -> Vec<(f32, f32)> {
        let mut touches = Vec::new();
        for i in 0..self.herd.len() {
            if self.herd[i].mother.is_some() {
                continue;
            }
            self.next_touch[i] -= dt;
            if self.next_touch[i] <= 0.0 {
                let jitter = ((t * 12.9898 + i as f32 * 78.233).sin() * 43_758.547)
                    .fract()
                    .abs();
                self.next_touch[i] = 3.0 + 5.0 * jitter;
                let at = self.herd[i].at;
                touches.push((at.x, at.z));
            }
        }
        touches
    }

    fn draws(&self, t: f32, queen: Vec3, out: &mut Vec<MeshDraw3D>) {
        use std::f32::consts::{PI, TAU};
        for (i, c) in self.herd.iter().enumerate() {
            let place = self.placement(i, t);
            let me = place.transform_point3(Vec3::ZERO);
            let ahead = place.transform_vector3(Vec3::Z);
            // She comes near: the head turns to her and the eyes open.
            let (dx, dz) = (queen.x - me.x, queen.z - me.z);
            let near = 1.0 - smooth(2.5, 5.0, dx.hypot(dz));
            let mut to_her = dx.atan2(dz) - ahead.x.atan2(ahead.z);
            to_her = (to_her + PI).rem_euclid(TAU) - PI;
            let idle = 0.18 * (t * 0.21 + c.phase).sin();
            let turn = idle * (1.0 - near) + to_her.clamp(-0.7, 0.7) * near;
            let nod = 0.04 * (t * 0.45 + c.phase).sin() + 0.03 * (1.0 - near);
            let head = Mat4::from_translation(self.neck)
                * Mat4::from_rotation_y(turn)
                * Mat4::from_rotation_x(nod)
                * Mat4::from_translation(-self.neck);
            // Dozing, mostly; awake a while now and then, blinking.
            let awake = near > 0.5 || (t * 0.045 + c.phase * 0.13).fract() > 0.82;
            let blink = (t * 0.37 + c.phase).fract() < 0.035;
            let open = awake && !blink;
            let breath = 0.02 * (t * 1.3 + c.phase).sin();
            for p in &self.parts {
                let local = match p.piece {
                    CapyPiece::Body => {
                        p.local * Mat4::from_scale(Vec3::new(1.0 + breath * 0.5, 1.0 + breath, 1.0))
                    }
                    CapyPiece::Head => head * p.local,
                    CapyPiece::Ear(side) => {
                        let e = (t * 0.29 + c.phase * 0.7 + side * 0.31).fract();
                        let flick = if e < 0.06 {
                            (e / 0.06 * PI * 3.0).sin() * 0.45
                        } else {
                            0.0
                        };
                        head * p.local * Mat4::from_rotation_z(side * flick)
                    }
                    CapyPiece::EyesClosed if open => continue,
                    CapyPiece::EyesOpen if !open => continue,
                    CapyPiece::EyesClosed | CapyPiece::EyesOpen => head * p.local,
                    CapyPiece::Yuzu => {
                        head * p.local
                            * Mat4::from_rotation_z(0.07 * (t * 1.1 + c.phase).sin())
                            * Mat4::from_rotation_x(0.05 * (t * 0.9 + c.phase).cos())
                    }
                };
                out.push(MeshDraw3D {
                    mesh: p.mesh,
                    transform: place * local,
                    material: p.material,
                    texture: p.texture,
                });
            }
        }
    }
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
        let mut valley = Vec::new();
        let mut lamps = Vec::new();
        let capybaras = match Capybaras::load(renderer, device, queue, &mut cache) {
            Ok(c) => Some(c),
            Err(error) => {
                eprintln!("[3d] new capybaras unavailable (keeping the old ones): {error:#}");
                None
            }
        };
        for file in ["baths_ground.glb", "baths_places.glb", "baths_life.glb"] {
            let drop_old = capybaras.is_some() && file == "baths_life.glb";
            load_draws(
                renderer,
                device,
                queue,
                &mut cache,
                file,
                12.0,
                &mut lamps,
                &mut valley,
                drop_old,
            )?;
        }
        let t_valley = started.elapsed().as_secs_f64() * 1000.0;
        let mut rooms = BTreeMap::new();
        for id in p.rooms.keys() {
            let mut draws = Vec::new();
            let mut unused = Vec::new();
            load_draws(
                renderer,
                device,
                queue,
                &mut cache,
                &format!("baths_room_{id}.glb"),
                64.0,
                &mut unused,
                &mut draws,
                false,
            )?;
            // Indoors every lantern burns.
            for i in unused {
                if let Some(d) = draws.get_mut(i) {
                    d.material = d.material.with_emission(1.6);
                }
            }
            rooms.insert(id.clone(), draws);
        }
        let t_rooms = started.elapsed().as_secs_f64() * 1000.0;
        // The plants: each species uploaded once, drawn wherever the plan puts one.
        let white = TextureData3D {
            width: 1,
            height: 1,
            rgba: vec![255; 4],
        };
        type Part = (MeshHandle, Material3D, Option<TextureHandle>, Mat4);
        let mut species: HashMap<&str, Vec<Part>> = HashMap::new();
        for plant in &p.plants {
            if !species.contains_key(plant.model.as_str()) {
                let path = asset_root().join(format!("baths_plant_{}.glb", plant.model));
                let model = super::prefetch::load(&path)
                    .with_context(|| format!("load baths plant: {}", path.display()))?;
                let mut parts = Vec::new();
                for (index, mesh) in model.meshes.iter().enumerate() {
                    let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
                    let normal = mesh
                        .normal_texture_index
                        .and_then(|i| model.textures.get(i));
                    let texture = upload_texture_pair(
                        renderer, device, queue, albedo, normal, &mut cache, &white,
                    )?;
                    parts.push((
                        renderer.upload_mesh(device, mesh)?,
                        mesh.material.with_ambient_occlusion(0.18),
                        texture,
                        model.mesh_transform(index),
                    ));
                }
                species.insert(plant.model.as_str(), parts);
            }
            let placement = Mat4::from_translation(Vec3::new(plant.x, plant.y, plant.z))
                * Mat4::from_rotation_y(plant.yaw)
                * Mat4::from_rotation_x(plant.tilt)
                * Mat4::from_scale(Vec3::new(plant.s, plant.s, plant.s));
            for (mesh, material, texture, local) in &species[plant.model.as_str()] {
                valley.push(MeshDraw3D {
                    mesh: *mesh,
                    transform: placement * *local,
                    material: *material,
                    texture: *texture,
                });
            }
        }
        // The water: the pools and the two tubs are simulated; the falls are drawn as they are.
        let mut water = Vec::new();
        let mut streams = Vec::new();
        let path = asset_root().join("baths_water.glb");
        let model = super::prefetch::load(&path)
            .with_context(|| format!("load baths water: {}", path.display()))?;
        for (index, mesh) in model.meshes.iter().enumerate() {
            let name = model
                .mesh_names
                .get(index)
                .map(String::as_str)
                .unwrap_or("");
            let transform = model.mesh_transform(index);
            if name == "waterStream" {
                // Running and falling water: the ribbon in the world, its colour the data the
                // stream shader reads (steepness, froth, width).
                let vertices = mesh
                    .vertices
                    .iter()
                    .map(|v| {
                        let mut out = *v;
                        let [x, y, z] = v.position;
                        let p = transform.transform_point3(Vec3::new(x, y, z));
                        out.position = [p.x, p.y, p.z];
                        out
                    })
                    .collect();
                let style = velvet_render::WaterStyle3D {
                    shallow: [0.06, 0.5, 0.46],
                    deep: [0.0, 0.2, 0.28],
                    foam: 0.8,
                    ..Default::default()
                };
                streams.push(renderer.add_water_stream(
                    device,
                    velvet_render::WaterStream3D {
                        vertices,
                        indices: mesh.indices.clone(),
                        style,
                        speed: 1.0,
                    },
                ));
                continue;
            }
            let place = match name.split_once(':') {
                Some((_, room)) => Place::Room(room.to_owned()),
                None => Place::Valley,
            };
            let triangles = mesh.world_triangles(transform);
            if let Some(mut body) = water_life::body(water_life::Kind::HotPool, &triangles, None) {
                // Outdoors the water ends where the ground rises out of it (the exported surface
                // runs on a little under the banks): the mask follows the real shore, the depth
                // the real bed.
                if place == Place::Valley {
                    let (w, h) = (body.size.0 as usize, body.size.1 as usize);
                    for j in 0..h {
                        for i in 0..w {
                            let k = j * w + i;
                            let x = body.origin[0] + (i as f32 + 0.5) * body.cell;
                            let z = body.origin[1] + (j as f32 + 0.5) * body.cell;
                            let depth = body.level - p.ground(x, z);
                            body.mask[k] *= smooth(-0.01, 0.06, depth);
                            body.depth[k] = depth.max(0.0) * 1.6;
                        }
                    }
                }
                body.style.shallow = [0.16, 0.62, 0.58];
                body.style.deep = [0.02, 0.26, 0.34];
                body.style.clarity = 0.4;
                body.style.look = if place == Place::Valley {
                    velvet_render::WaterLook3D::Spring
                } else {
                    velvet_render::WaterLook3D::Tub
                };
                water.push(Water {
                    handle: renderer.add_water_body(device, body),
                    place,
                });
            }
        }
        let falls = p
            .pools
            .iter()
            .filter_map(|pool| {
                let f = pool.fall?;
                let (dx, dz) = (pool.x - f.x, pool.z - f.z);
                let len = dx.hypot(dz).max(1e-3);
                Some((
                    Vec3::new(f.x + dx / len * 2.3, pool.level, f.z + dz / len * 2.3),
                    f.w,
                ))
            })
            .collect();
        let puff = renderer
            .upload_mesh(
                device,
                &Mesh3D::uv_sphere(1.0, 10, 6, Material3D::rgb(1.0, 1.0, 1.0)),
            )
            .ok();
        let t_plants_water = started.elapsed().as_secs_f64() * 1000.0;
        // The spa girls and their day.
        let life = match super::spa_life::SpaLife::load(
            renderer,
            device,
            queue,
            &asset_root().join("spa_girl.glb"),
        ) {
            Ok(life) => Some(life),
            Err(error) => {
                eprintln!("[3d] spa girls unavailable: {error:#}");
                None
            }
        };
        let mugo = match super::mugo::Mugo::load(
            renderer,
            device,
            queue,
            &runtime_3d_root().join("characters").join("mugo.glb"),
        ) {
            Ok(mugo) => Some(mugo),
            Err(error) => {
                eprintln!("[3d] Mugo unavailable: {error:#}");
                None
            }
        };
        eprintln!("[3d] baths phases: valley {t_valley:.0} ms, rooms {:.0}, plants+water {:.0}, girls {:.0}", t_rooms - t_valley, t_plants_water - t_rooms, started.elapsed().as_secs_f64() * 1000.0 - t_plants_water);
        eprintln!(
            "[3d] baths loaded in {:.0} ms: {} valley draws, {} rooms, {} water bodies",
            started.elapsed().as_secs_f64() * 1000.0,
            valley.len(),
            rooms.len(),
            water.len()
        );
        Ok(Self {
            life,
            mugo,
            room_girls: Vec::new(),
            queen_last: None,
            capybaras,
            valley,
            rooms,
            lamps,
            water,
            streams,
            puff,
            falls,
            last: None,
            beat: 0.0,
            next_drop: 0.0,
            rng: 0x2545_F491,
            last_touch: None,
        })
    }

    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng & 0xFFFF) as f32 / 65535.0
    }

    /// The water this frame: which bodies show, how long they ran, what touched them.
    pub(crate) fn tick(
        &mut self,
        state: Option<&State>,
        renderer: &mut Wgpu3DRenderer,
        queue: &wgpu::Queue,
        elapsed: f32,
    ) {
        let here = state.map(|s| match &s.room {
            Some(r) => Place::Room(r.clone()),
            None => Place::Valley,
        });
        for w in &self.water {
            renderer.set_water_visible(w.handle, here.as_ref() == Some(&w.place));
        }
        for &s in &self.streams {
            renderer.set_water_stream_visible(s, here == Some(Place::Valley));
        }
        let Some(state) = state else {
            self.last = None;
            return;
        };
        let dt = self
            .last
            .map(|l| (elapsed - l).clamp(0.0, 0.1))
            .unwrap_or(0.0);
        self.last = Some(elapsed);
        renderer.advance_water(dt);
        if let Some(room) = state.room.as_deref() {
            // The room's girls (posed here: `draws` cannot reach the renderer).
            let mut girls = Vec::new();
            if let Some(life) = self.life.as_mut() {
                life.room_draws(renderer, queue, room, dt, &mut girls);
            }
            self.room_girls = girls;
            return;
        }
        if let Some(life) = self.life.as_mut() {
            let me = Vec3::new(state.x, 0.0, state.z);
            let velocity = match self.queen_last {
                Some(last) if dt > 0.0 => (me - last) * (1.0 / dt),
                _ => Vec3::ZERO,
            };
            self.queen_last = Some(me);
            let mut touches = Vec::new();
            life.tick(
                renderer,
                queue,
                state.hour,
                dt,
                (me, velocity),
                &mut touches,
            );
            for (x, z) in touches {
                renderer.water_touch(x, z, 0.3, 0.004);
            }
        }
        if let Some(mugo) = self.mugo.as_mut() {
            mugo.tick(
                renderer,
                queue,
                dt,
                state.guide,
                Vec3::new(state.x, 0.0, state.z),
            );
        }
        if let Some(capybaras) = self.capybaras.as_mut() {
            for (x, z) in capybaras.tick(dt, elapsed) {
                renderer.water_touch(x, z, 0.35, 0.004);
            }
        }
        // Wading: a ring round her legs at each step, a gentle swell when she is still.
        let p = plan();
        let me = Vec3::new(state.x, 0.0, state.z);
        if let Some(pool) = p
            .pool_at(state.x, state.z)
            .filter(|_| !p.on_floor(state.x, state.z))
        {
            if self.last_touch.is_none_or(|t| (t - me).length() > 0.22) {
                renderer.water_touch(state.x, state.z, 0.3, 0.004);
                for _ in 0..3 {
                    let a = self.random() * std::f32::consts::TAU;
                    let v = Vec3::new(a.cos() * 0.5, 1.2 + self.random() * 0.6, a.sin() * 0.5);
                    renderer.water_emit(Vec3::new(state.x, pool.level + 0.03, state.z), v, 1.2);
                }
                self.last_touch = Some(me);
            }
        } else {
            self.last_touch = None;
        }
        // The falls pound their pools; now and then a drop falls from the leaves.
        self.beat -= dt;
        if self.beat <= 0.0 {
            self.beat = 0.09;
            for i in 0..self.falls.len() {
                let (at, w) = self.falls[i];
                let a = self.random() - 0.5;
                renderer.water_touch(
                    at.x + a * w * 0.6,
                    at.z + (self.random() - 0.5) * 0.4,
                    0.25,
                    0.0012,
                );
            }
        }
        self.next_drop -= dt;
        if self.next_drop <= 0.0 {
            self.next_drop = 0.6 + self.random() * 1.6;
            let i = ((self.random() * p.pools.len() as f32) as usize).min(p.pools.len() - 1);
            let pool = &p.pools[i];
            let a = self.random() * std::f32::consts::TAU;
            let k = self.random().sqrt() * 0.7;
            renderer.water_touch(
                pool.x + a.cos() * pool.rx * k,
                pool.z + a.sin() * pool.rz * k,
                0.12,
                0.0015,
            );
        }
    }

    /// Everything to draw for where she is.
    /// The girls standing about (x, z, radius), for the Queen to walk round.
    pub(crate) fn obstacles(&self) -> Vec<[f32; 3]> {
        let mut out = self
            .life
            .as_ref()
            .map(|l| l.obstacles())
            .unwrap_or_default();
        out.extend(self.mugo.as_ref().map(|m| m.obstacle()));
        out
    }

    /// Where Mugo is and whether he stands at his spot pointing (`None`: no Mugo).
    pub(crate) fn mugo_report(&self) -> Option<([f32; 2], bool)> {
        self.mugo.as_ref().map(|m| m.report())
    }

    /// The spa life (QA and reports).
    pub(crate) fn life(&self) -> Option<&super::spa_life::SpaLife> {
        self.life.as_ref()
    }

    pub(crate) fn draws(
        &self,
        state: &State,
        elapsed: f32,
        moon_visibility: f32,
        draws: &mut Vec<MeshDraw3D>,
    ) {
        if let Some(room) = &state.room {
            if let Some(list) = self.rooms.get(room) {
                draws.extend_from_slice(list);
            }
            draws.extend_from_slice(&self.room_girls);
            return;
        }
        let base = draws.len();
        draws.extend_from_slice(&self.valley);
        if let Some(life) = &self.life {
            life.draws(draws);
        }
        if let Some(mugo) = &self.mugo {
            mugo.draws(draws);
        }
        if let Some(capybaras) = &self.capybaras {
            capybaras.draws(elapsed, Vec3::new(state.x, 0.0, state.z), draws);
        }
        let glow = village_window_glow(moon_visibility, 0.1);
        for &i in &self.lamps {
            if let Some(d) = draws.get_mut(base + i) {
                d.material = d.material.with_emission(0.3 + glow * 1.4);
            }
        }
        // Steam rising off every pool: thicker in the cold of the night.
        if let Some(puff) = self.puff {
            let night = ((moon_visibility - 0.12) / 0.48).clamp(0.0, 1.0);
            for (k, pool) in plan().pools.iter().enumerate() {
                let count = (4.0 + pool.rx * pool.rz * 0.6) as usize;
                for i in 0..count {
                    let age = (elapsed * 0.13 + i as f32 / count as f32 + k as f32 * 0.37).fract();
                    let a = i as f32 * 2.39 + k as f32;
                    let r = ((i * 7 % 11) as f32 / 11.0).sqrt() * 0.8;
                    let drift = Vec3::new(
                        (elapsed * 0.3 + a * 3.0).sin() * 0.3 + age * 0.45,
                        0.25 + age * 2.4,
                        (elapsed * 0.27 + a * 2.0).cos() * 0.3 - age * 0.3,
                    );
                    let middle = Vec3::new(
                        pool.x + a.cos() * pool.rx * r,
                        pool.level,
                        pool.z + a.sin() * pool.rz * r,
                    );
                    // Thin wisps that rise clear of the water (not fog lying on it).
                    let size = 0.25 + age * 0.55;
                    let mut material = Material3D::rgb(0.95, 0.98, 0.97);
                    material.base_color[3] = (0.06 + 0.05 * night)
                        * (age * std::f32::consts::PI).sin()
                        * smooth(0.0, 0.25, age);
                    draws.push(MeshDraw3D {
                        mesh: puff,
                        transform: Mat4::from_translation(
                            middle + drift + Vec3::new(0.0, 0.3, 0.0),
                        ) * Mat4::from_scale(Vec3::new(size, size * 1.3, size)),
                        material,
                        texture: None,
                    });
                }
            }
        }
    }
}

// ------------------------------------------------------------------ tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn she_walks_in_through_the_gate_and_finds_every_door() {
        let p = plan();
        let s = State::enter(Vec3::ZERO, 0.0);
        assert!(!p.blocked_outdoor(s.x, s.z, BODY), "the gate is free");
        assert!(p.hotspots.iter().any(|h| h.to_village));
        for (id, room) in &p.rooms {
            assert!(
                p.hotspots
                    .iter()
                    .any(|h| h.enter.as_deref() == Some(id.as_str())),
                "a door to {id}"
            );
            assert!(
                !room.blocked(room.spawn.x, room.spawn.z, BODY),
                "{id}'s spawn is free"
            );
            assert!(room.hotspots.iter().any(|h| h.exit), "{id} has a way out");
        }
    }

    #[test]
    fn she_can_wade_into_a_pool_and_sinks_a_little() {
        let p = plan();
        let pool = &p.pools[1];
        assert!(
            !p.blocked_outdoor(pool.x + pool.rx * 0.3, pool.z, BODY)
                || !p.blocked_outdoor(pool.x - pool.rx * 0.3, pool.z, BODY)
        );
        assert!(p.in_pool(pool.x, pool.z) > 0.9);
        assert!(p.stand(pool.x, pool.z) < p.ground(pool.x, pool.z));
    }

    #[test]
    fn her_steps_sound_of_what_she_walks_on() {
        use super::super::game_audio::Ground;
        let p = plan();
        let pool = &p.pools[1];
        assert_eq!(
            p.ground_kind(pool.x, pool.z),
            Ground::Water,
            "in the central pool"
        );
        assert_eq!(p.ground_kind(-0.9, -4.0), Ground::Wood, "on its deck");
        assert_eq!(
            p.ground_kind(0.4, 3.03),
            Ground::Wood,
            "on the plank bridge"
        );
        assert_eq!(p.ground_kind(0.5, 1.0), Ground::Water, "wading the brook");
        assert_eq!(
            p.ground_kind(-4.2, 1.4),
            Ground::Stone,
            "on the gravel path"
        );
        assert!(!p.channels.is_empty());
        assert!(!emitters(None).is_empty() && !emitters(Some("privada")).is_empty());
    }

    #[test]
    fn she_walks_on_the_decks_and_the_bridges_not_through_them() {
        let p = plan();
        // The deck into the central pool: she stands on its boards, above the water, dry.
        let (x, z) = (-0.9, -4.0);
        let pool = &p.pools[1];
        assert!(p.on_floor(x, z), "on the deck");
        assert!(
            p.stand(x, z) > pool.level + 0.2,
            "on top of the boards: {} vs water {}",
            p.stand(x, z),
            pool.level
        );
        // The red bridge: up on its arch over the cascade.
        let (bx, bz) = (2.0, -8.62);
        assert!(p.on_floor(bx, bz), "on the bridge");
        assert!(p.stand(bx, bz) > p.ground(bx, bz) + 0.3);
        // The plank bridge over the brook.
        assert!(p.on_floor(0.4, 3.03));
        assert!(p.stand(0.4, 3.03) > p.ground(0.4, 3.03) + 0.1);
        // Off them, the ground as before.
        assert!(!p.on_floor(-4.2, 1.4));
    }

    #[allow(dead_code)]
    fn _keep() {}

    #[test]
    fn the_capybaras_doze_in_the_lower_pool() {
        let p = plan();
        let low = p.pools.iter().find(|pool| pool.id == "pozas-baja").unwrap();
        let spots = capybara_spots();
        assert_eq!(spots.len(), 2);
        for (x, z, level, _, scale) in spots {
            assert!(
                low.t(x, z) < 0.6,
                "a capybara out in the open water, not by the shore"
            );
            assert!((level - low.level).abs() < 1e-6);
            assert!((0.5..1.2).contains(&scale));
        }
        // Only the prototype's capybara colours are dropped from baths_life.glb.
        assert!(is_old_capybara([0.2705, 0.1144, 0.0452, 1.0]));
        assert!(
            !is_old_capybara([0.9387, 0.5647, 0.0319, 1.0]),
            "the floating yuzu stay"
        );
    }

    #[test]
    fn the_spa_girls_are_placed_and_the_swimmer_goes_round() {
        let p = plan();
        assert!(p.spa_girls.len() >= 5);
        let swimmer = p
            .spa_girls
            .iter()
            .find_map(|g| g.swim.as_ref())
            .expect("someone swims");
        let (a, _) = swimmer.at(0.0);
        let (b, _) = swimmer.at(1.0);
        assert!(
            (a - b).length() > 0.5 && (a - b).length() < 1.5,
            "a metre round the loop"
        );
        // Every bather sits in water, every lounger girl above the ground.
        for g in p.spa_girls.iter().filter(|g| g.swim.is_none()) {
            if g.clip == "Spa_Lie" {
                assert!(g.y > p.ground(g.x, g.z) - 0.2);
            } else {
                assert!(p.pool_at(g.x, g.z).is_some(), "{} sits in a pool", g.clip);
            }
        }
    }

    #[test]
    fn a_bow_takes_its_colour_and_the_bones_keep_theirs() {
        let mut px = [235u8, 90, 110, 255, 230, 220, 205, 255];
        recolour_bow(&mut px, 0.55);
        assert!(px[2] > px[0], "the pink bow turns blue: {px:?}");
        assert_eq!(&px[4..], &[230, 220, 205, 255], "bone stays bone");
    }

    #[test]
    fn entering_a_room_and_leaving_it_brings_her_back_to_its_door() {
        let p = plan();
        let door = p
            .hotspots
            .iter()
            .find(|h| h.enter.is_some())
            .expect("a door");
        let mut s = State::enter(Vec3::ZERO, 0.0);
        (s.x, s.z) = (door.x, door.z);
        assert!(matches!(s.interact(), Action::None));
        assert_eq!(s.room.as_deref(), door.enter.as_deref());
        let exit = s
            .room_plan()
            .unwrap()
            .hotspots
            .iter()
            .find(|h| h.exit)
            .unwrap();
        (s.x, s.z) = (exit.x, exit.z);
        s.interact();
        assert!(s.room.is_none());
        assert!((s.x - door.x).abs() < 1.5 && (s.z - door.z).abs() < 1.5);
    }
}
