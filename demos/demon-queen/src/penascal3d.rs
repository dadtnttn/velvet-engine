//! El Peñascal, the goblin village of the north where the act 1 boss sits (DemonQueen-Lore/ACTO1.md
//! §8), built in its Three.js prototype (`demos/demon-queen-penascal-threejs`) and exported by its
//! `tools/export-game.mjs`: the GLB parts live in `data/assets/3d/penascal/`, the plan in
//! `data/penascal/penascal.json`.
//!
//! A market plaza in a horseshoe of grey cliffs, its three houses dug into the north face, and round
//! it, half scattered among the crags: the gorge up from the forest (the way in), the lower quarter
//! to the west and the wolf pens to the east, with more houses dug into their rock. In the middle of
//! the plaza the great flat market stone where Varrak, the orc who lost his spark, sits like a king
//! (the user's orc: `characters/varrak.glb`, `tools/varrak_blender.py`).
//!
//! As in the castle the camera frames the place she is in and the walls between it and her drop;
//! a cliff drops as a whole line (otherwise the crown of the next piece would hang over the place).
use super::*;
use serde::Deserialize;

use velvet_render::{CutawayWalls3D, ToonFire3D};

const RADIUS: f32 = 0.3;
/// How low a dropped wall goes (a fraction of its height).
const STUB: f32 = 0.12;
/// Varrak's size here: the user's orc chief («orco jefe varak puerta») is 0.71 m tall in its file;
/// 2.9 m in the village (twice the Queen), as the prototype has him.
pub(crate) const VARRAK_SCALE: f32 = 4.05;

// ------------------------------------------------------------------ the plan
#[derive(Deserialize)]
pub(crate) struct Room {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    #[allow(dead_code)]
    pub icon: String,
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

fn default_door_h() -> f32 {
    2.5
}

#[derive(Deserialize)]
pub(crate) struct Door {
    #[serde(default)]
    #[allow(dead_code)]
    pub id: String,
    pub axis: String,
    pub at: f32,
    pub c: f32,
    pub w: f32,
    pub outside: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub gate: bool,
    #[serde(default = "default_door_h")]
    #[allow(dead_code)]
    pub h: f32,
}

#[derive(Deserialize)]
pub(crate) struct Wall {
    pub axis: String,
    pub at: f32,
    pub a: f32,
    pub b: f32,
    pub door: bool,
    #[allow(dead_code)]
    pub h: f32,
    #[serde(default)]
    pub cliff: bool,
}

#[derive(Deserialize)]
pub(crate) struct Hotspot {
    pub id: String,
    #[allow(dead_code)]
    pub room: String,
    pub x: f32,
    pub z: f32,
    pub radius: f32,
    pub icon: String,
    pub label: String,
    pub text: String,
}

#[derive(Deserialize)]
pub(crate) struct Lamp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: String,
    pub power: f32,
    pub wall: i32,
}

#[derive(Deserialize)]
pub(crate) struct Flame {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub color: String,
    pub size: f32,
    pub kind: String,
    pub wall: i32,
}

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Point {
    pub x: f32,
    pub z: f32,
}

/// The market stone (the chief's seat).
#[derive(Deserialize, Clone, Copy)]
pub(crate) struct Stone {
    pub x: f32,
    pub z: f32,
    #[allow(dead_code)]
    pub w: f32,
    pub d: f32,
    #[allow(dead_code)]
    pub h: f32,
}

#[derive(Deserialize)]
pub(crate) struct Plan {
    pub rooms: Vec<Room>,
    pub doors: Vec<Door>,
    #[serde(rename = "wallT")]
    pub wall_t: f32,
    pub walls: Vec<Wall>,
    /// [x, z, r, level]: rocks, braziers, people.
    pub circles: Vec<Vec<f32>>,
    /// [x, z, hx, hz, level]: stalls, fences, tubs.
    pub boxes: Vec<Vec<f32>>,
    pub hotspots: Vec<Hotspot>,
    /// At the gorge's far end, up from the forest.
    pub spawn: Point,
    /// In the plaza, a few steps from the stone (back from the fight).
    #[serde(rename = "spawnFromFight")]
    pub spawn_from_fight: Point,
    pub stone: Stone,
    pub lamps: Vec<Lamp>,
    pub flames: Vec<Flame>,
}

pub(crate) fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("penascal/penascal.json")).expect("valid Peñascal plan")
    })
}

/// Whether the place has been exported into the game (its plan and its parts are there).
pub(crate) fn available() -> bool {
    asset_root().join("penascal_static.glb").exists()
        && exploration3d_state::data_dir()
            .join("penascal")
            .join("penascal.json")
            .exists()
}

fn asset_root() -> PathBuf {
    runtime_3d_root().join("penascal")
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

impl Plan {
    /// Whether a circle of radius `r` at (x, z) cannot stand there: outside the places and their
    /// doorways (the way out is F, not walking), or in a rock, a stall, a fence.
    pub(crate) fn blocked(&self, x: f32, z: f32, r: f32) -> bool {
        let inset = self.wall_t / 2.0 + 0.05;
        let in_room = |room: &Room| {
            x > room.x0 + inset + r
                && x < room.x1 - inset - r
                && z > room.z0 + inset + r
                && z < room.z1 - inset - r
        };
        let in_door = |d: &Door| {
            let (across, along) = if d.axis == "x" {
                (x - d.at, z - d.c)
            } else {
                (z - d.at, x - d.c)
            };
            across.abs() < inset + 0.6 && along.abs() < d.w / 2.0 - 0.05 - r
        };
        let open =
            self.rooms.iter().any(in_room) || self.doors.iter().any(|d| !d.outside && in_door(d));
        if !open {
            return true;
        }
        self.circles
            .iter()
            .any(|c| c.len() >= 3 && (x - c[0]).hypot(z - c[1]) < c[2] + r)
            || self
                .boxes
                .iter()
                .any(|b| b.len() >= 4 && (x - b[0]).abs() < b[2] + r && (z - b[1]).abs() < b[3] + r)
    }

    /// The place at (x, z), if she is in one (not in a doorway).
    pub(crate) fn room_at(&self, x: f32, z: f32) -> Option<usize> {
        self.rooms.iter().position(|r| r.contains(x, z))
    }

    /// The bounds of the whole village (x0, x1, z0, z1).
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

    /// Whether wall `w` drops for room `room` with the camera along `yaw` (the camera stands at
    /// +sin, +cos from what it looks at): the room's walls on the camera's side, and a cliff on
    /// that line whole (the prototype's rule).
    fn wall_drops(&self, w: &Wall, room: &Room, yaw: f32) -> bool {
        let (sx, sz) = (yaw.sin(), yaw.cos());
        let within =
            |a: f32, b: f32, lo: f32, hi: f32| w.cliff || (a >= lo - 0.01 && b <= hi + 0.01);
        let near = |a: f32, b: f32| (a - b).abs() < 1e-3;
        if w.axis == "z" && within(w.a, w.b, room.x0, room.x1) {
            if near(w.at, room.z1) && sz > 0.25 {
                return true;
            }
            if near(w.at, room.z0) && sz < -0.25 {
                return true;
            }
        }
        if w.axis == "x" && within(w.a, w.b, room.z0, room.z1) {
            if near(w.at, room.x1) && sx > 0.25 {
                return true;
            }
            if near(w.at, room.x0) && sx < -0.25 {
                return true;
            }
        }
        false
    }

    /// A hotspot by id.
    pub(crate) fn hotspot(&self, id: &str) -> Option<&Hotspot> {
        self.hotspots.iter().find(|h| h.id == id)
    }
}

/// The game's icon for a hotspot's.
pub(crate) fn icon(name: &str) -> Icon {
    match name {
        "door" => Icon::Leave,
        "crown" => Icon::Crown,
        "fire" => Icon::Feast,
        "tea" => Icon::Feast,
        "book" => Icon::Memory,
        "barrel" => Icon::Chest,
        "shield" => Icon::Gear,
        "notice" => Icon::Rune,
        _ => Icon::Talk,
    }
}

/// The verb on a hotspot's prompt.
fn verb(id: &str) -> &'static str {
    match id {
        "salida" => "Bajar",
        "varrak" => "Desafiar",
        "vigia" => "Mirar",
        _ => "Mirar",
    }
}

// ------------------------------------------------------------------ the Queen in it
/// What pressing F did, for the explorer to carry out.
pub(crate) enum Action {
    None,
    /// Down the gorge to the forest.
    Leave,
    /// The chief on his stone.
    Varrak,
    Say(&'static str),
}

/// El Peñascal as the Queen lives it: where she is, which walls are down.
#[derive(Clone, Debug)]
pub(crate) struct State {
    pub x: f32,
    pub z: f32,
    /// The place she is in (the last one, in a doorway).
    pub room: usize,
    walls: CutawayWalls3D,
    /// Every wall up whatever the camera (seen from above, for a map).
    pub keep_walls: bool,
}

impl State {
    fn at(x: f32, z: f32) -> Self {
        let p = plan();
        Self {
            x,
            z,
            room: p.room_at(x, z).unwrap_or(0),
            walls: CutawayWalls3D::new(p.walls.len(), STUB),
            keep_walls: false,
        }
    }

    /// At the gorge's far end, up from the forest.
    pub(crate) fn way_in() -> Self {
        let s = plan().spawn;
        Self::at(s.x, s.z)
    }

    /// Back where she was (a saved or a QA spot) if that is free ground here, else at the way in.
    pub(crate) fn restore(position: Vec3) -> Self {
        if plan().blocked(position.x, position.z, RADIUS) {
            Self::way_in()
        } else {
            Self::at(position.x, position.z)
        }
    }

    /// In the plaza by the stone (back from the fight).
    pub(crate) fn by_the_stone() -> Self {
        let s = plan().spawn_from_fight;
        Self::at(s.x, s.z)
    }

    pub(crate) fn position(&self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }

    /// One step, sliding along whatever is in the way: straight on, else along the wall (one axis),
    /// else turned a little round a rock or a jamb.
    pub(crate) fn walk(&mut self, dx: f32, dz: f32) {
        let p = plan();
        let (x, z) = (self.x, self.z);
        let turned = |a: f32| {
            let (s, c) = a.sin_cos();
            let k = c.max(0.2);
            ((dx * c - dz * s) * k, (dx * s + dz * c) * k)
        };
        let mut tries = vec![(dx, dz)];
        // Pressed against a doorway's jamb, heading through it: edge along the wall into it first.
        let step = dx.hypot(dz);
        if let Some(d) = p.doors.iter().filter(|d| !d.outside).find(|d| {
            let (across, along, push) = if d.axis == "x" {
                (x - d.at, z - d.c, (d.at - x) * dx)
            } else {
                (z - d.at, x - d.c, (d.at - z) * dz)
            };
            across.abs() < 1.4 && along.abs() < d.w / 2.0 + 0.6 && push > 0.0
        }) {
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
        if let Some((nx, nz)) = tries
            .iter()
            .filter(|(sx, sz)| sx.abs() > 1e-6 || sz.abs() > 1e-6)
            .map(|&(sx, sz)| (x + sx, z + sz))
            .find(|&(nx, nz)| !p.blocked(nx, nz, RADIUS))
        {
            self.x = nx;
            self.z = nz;
        }
        if let Some(room) = p.room_at(self.x, self.z) {
            self.room = room;
        }
    }

    /// The walls drop out of the camera's way and rise again.
    pub(crate) fn update(&mut self, dt: f32, camera_yaw: f32) {
        let p = plan();
        let room = &p.rooms[self.room];
        let keep = self.keep_walls;
        self.walls
            .update(dt, |i| !keep && p.wall_drops(&p.walls[i], room, camera_yaw));
    }

    /// How tall wall `i` stands this frame (1 whole .. STUB dropped).
    pub(crate) fn wall_scale(&self, i: usize) -> f32 {
        self.walls.scale(i)
    }

    /// Whether what hangs on wall `i` is shown (not while it is down).
    pub(crate) fn wall_dressed(&self, i: usize) -> bool {
        self.walls.dressed(i)
    }

    /// Where the camera looks: the place she is in, leaning towards her (further in the big open
    /// places, as the prototype's camera does).
    pub(crate) fn focus(&self) -> Vec3 {
        let p = plan();
        let r = &p.rooms[self.room];
        let (cx, cz) = r.centre();
        let lean = ((r.x1 - r.x0).max(r.z1 - r.z0) - 12.0)
            .mul_add(0.5, 0.0)
            .clamp(3.0, 7.0);
        let k = if lean > 3.0 { 0.7 } else { 0.45 };
        Vec3::new(
            cx + ((self.x - cx) * k).clamp(-lean, lean),
            0.0,
            cz + ((self.z - cz) * k).clamp(-lean, lean),
        )
    }

    /// The hotspot within reach.
    pub(crate) fn nearest(&self) -> Option<&'static Hotspot> {
        plan()
            .hotspots
            .iter()
            .map(|h| (h, (h.x - self.x).hypot(h.z - self.z)))
            .filter(|(h, d)| *d < h.radius.max(1.6))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h)
    }

    pub(crate) fn prompt(&self, varrak_awake: bool) -> Option<Prompt> {
        let h = self.nearest()?;
        if h.id == "varrak" && varrak_awake {
            return Some(Prompt::new(Icon::Talk, "Hablar").detail("Varrak, despierto"));
        }
        let detail = if h.id == "salida" {
            "Al bosque oscuro".to_owned()
        } else {
            h.label.clone()
        };
        Some(Prompt::new(icon(&h.icon), verb(&h.id)).detail(detail))
    }

    /// F: down to the forest, the chief, or look.
    pub(crate) fn interact(&self) -> Action {
        let Some(h) = self.nearest() else {
            return Action::None;
        };
        match h.id.as_str() {
            "salida" => Action::Leave,
            "varrak" => Action::Varrak,
            _ => Action::Say(h.text.as_str()),
        }
    }

    /// The torches and braziers nearest the place she is in that can be seen (not on a dropped
    /// wall), flickering.
    pub(crate) fn lights(&self, elapsed: f32) -> Vec<PointLight3D> {
        let p = plan();
        let focus = self.focus();
        let mut near: Vec<(f32, usize)> = p
            .lamps
            .iter()
            .enumerate()
            .filter(|(_, l)| l.wall < 0 || self.wall_dressed(l.wall as usize))
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

/// Dusk in the hills: the castle's torchlit finish (the flames' light bounced off the rock).
pub(crate) fn finish() -> Finish3D {
    Finish3D {
        gi: 2.2,
        gi_radius: 3.5,
        gi_ambient: 0.75,
        ..Finish3D::default()
    }
}

/// Outdoors at the hour it is, a little colder and more violet among the crags than in the wood.
pub(crate) fn mood(env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    let mut env = super::dark_forest::mood(env);
    env.lighting.ambient_color = [
        env.lighting.ambient_color[0] * 0.9 + 0.03,
        env.lighting.ambient_color[1] * 0.85 + 0.02,
        env.lighting.ambient_color[2] * 0.95 + 0.06,
    ];
    env.atmosphere.fog_start = env.atmosphere.fog_start.max(55.0);
    env.atmosphere.fog_end = env.atmosphere.fog_end.max(130.0);
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

/// Varrak on his stone: his model, his clips (sitting, and on his knees once he is awake), meshes.
pub(crate) struct Varrak {
    pub model: GltfModel,
    pub sit: Option<usize>,
    pub kneel: Option<usize>,
    pub meshes: Vec<(MeshHandle, Material3D, Option<TextureHandle>)>,
}

/// El Peñascal's GPU side.
pub(crate) struct Scene {
    statics: Vec<Part>,
    /// Per wall: its rock, and what hangs on it.
    walls: Vec<(Vec<Part>, Vec<Part>)>,
    /// The goblins and wolves in their poses.
    life: Vec<Part>,
    flame: MeshHandle,
    halo: MeshHandle,
    halo_texture: TextureHandle,
    /// The braziers by the stone and the like.
    fires: Vec<ToonFire3D>,
    pub varrak: Option<Varrak>,
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
        let statics = load_parts(renderer, device, queue, &mut cache, "penascal_static.glb")?
            .into_iter()
            .map(|(_, part)| part)
            .collect();
        let mut walls: Vec<(Vec<Part>, Vec<Part>)> = (0..p.walls.len())
            .map(|_| (Vec::new(), Vec::new()))
            .collect();
        for (name, part) in load_parts(renderer, device, queue, &mut cache, "penascal_walls.glb")? {
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
        let life = if asset_root().join("penascal_life.glb").exists() {
            load_parts(renderer, device, queue, &mut cache, "penascal_life.glb")?
                .into_iter()
                .map(|(_, part)| part)
                .collect()
        } else {
            Vec::new()
        };
        let flame = renderer.upload_mesh(
            device,
            &Mesh3D::uv_sphere(1.0, 10, 8, Material3D::rgb(1.0, 1.0, 1.0)),
        )?;
        let halo = renderer.upload_mesh(device, &quad())?;
        let halo_texture = renderer.upload_texture(device, queue, &halo_texture())?;
        let mut fires = Vec::new();
        for f in p
            .flames
            .iter()
            .filter(|f| f.kind == "hearth" || f.kind == "brazier")
        {
            let width = if f.kind == "hearth" { 0.9 } else { 0.55 };
            fires.push(ToonFire3D::new(
                renderer,
                device,
                Vec3::new(f.x, f.y - 0.05, f.z),
                width,
            )?);
        }
        let varrak = load_varrak(renderer, device, queue);
        eprintln!(
            "[3d] Peñascal loaded in {:.0} ms ({} walls, varrak {})",
            started.elapsed().as_secs_f64() * 1000.0,
            walls.len(),
            if varrak.is_some() { "yes" } else { "no" }
        );
        Ok(Self {
            statics,
            walls,
            life,
            flame,
            halo,
            halo_texture,
            fires,
            varrak,
        })
    }

    /// Everything to draw this frame for `state` (Varrak is posed and drawn by the zone);
    /// `toward` points from the scene to the camera.
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
        push(draws, &self.statics, Mat4::IDENTITY);
        push(draws, &self.life, Mat4::IDENTITY);
        for (i, (body, decor)) in self.walls.iter().enumerate() {
            let s = state.wall_scale(i);
            let dressed = state.wall_dressed(i);
            // A doorway's lintel would float at shin height: it goes with the decor.
            if dressed || !p.walls[i].door {
                push(draws, body, Mat4::from_scale(Vec3::new(1.0, s, 1.0)));
            }
            if dressed {
                push(draws, decor, Mat4::IDENTITY);
            }
        }
        // The torches: a bright tongue and a halo turned to the camera; the braziers as fire.
        let toward = Vec3::new(toward.x, 0.0, toward.z).normalize_or_zero();
        let face = Mat4::from_rotation_y(toward.x.atan2(toward.z));
        for (i, f) in p.flames.iter().enumerate() {
            if f.kind == "hearth" || f.kind == "brazier" {
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
        for fire in &self.fires {
            fire.draws(elapsed, toward, draws);
        }
    }
}

/// Varrak's model, if it is there.
fn load_varrak(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Option<Varrak> {
    let file = runtime_3d_root().join("characters").join("varrak.glb");
    if !file.exists() {
        eprintln!("[3d] Varrak's model is missing: {}", file.display());
        return None;
    }
    let model = match GltfModel::load(&file) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[3d] Varrak's model failed: {e:#}");
            return None;
        }
    };
    let sit = model.animation_index("Varrak_Sit");
    let kneel = model.animation_index("Varrak_Kneel");
    let white = TextureData3D {
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    };
    let mut cache = HashMap::new();
    let mut meshes = Vec::with_capacity(model.meshes.len());
    for mesh in &model.meshes {
        let handle = renderer.upload_skinned_mesh(device, mesh).ok()?;
        let albedo = mesh.texture_index.and_then(|i| model.textures.get(i));
        let normal = mesh
            .normal_texture_index
            .and_then(|i| model.textures.get(i));
        let texture =
            upload_texture_pair(renderer, device, queue, albedo, normal, &mut cache, &white)
                .ok()?;
        meshes.push((handle, mesh.material, texture));
    }
    Some(Varrak {
        model,
        sit,
        kneel,
        meshes,
    })
}

// ------------------------------------------------------------------ tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_way_in_and_the_stone_are_free_and_every_place_is_walked_to() {
        if !available() {
            return;
        }
        let p = plan();
        let way = State::way_in();
        assert!(!p.blocked(way.x, way.z, RADIUS), "the way in is blocked");
        assert_eq!(p.rooms[way.room].id, "paso");
        let stone = State::by_the_stone();
        assert!(!p.blocked(stone.x, stone.z, RADIUS));
        assert_eq!(p.rooms[stone.room].id, "plaza");
        // A flood from the way in, as she walks.
        let [x0, x1, z0, z1] = p.bounds();
        let step = 0.25;
        let nx = ((x1 - x0) / step) as usize + 1;
        let nz = ((z1 - z0) / step) as usize + 1;
        let mut seen = vec![false; nx * nz];
        let cell = |x: f32, z: f32| {
            (
                ((x - x0) / step).round() as i32,
                ((z - z0) / step).round() as i32,
            )
        };
        let (si, sj) = cell(way.x, way.z);
        seen[sj as usize * nx + si as usize] = true;
        let mut queue = vec![(si, sj)];
        while let Some((i, j)) = queue.pop() {
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (a, b) = (i + di, j + dj);
                if a < 0
                    || b < 0
                    || a as usize >= nx
                    || b as usize >= nz
                    || seen[b as usize * nx + a as usize]
                {
                    continue;
                }
                if p.blocked(x0 + a as f32 * step, z0 + b as f32 * step, RADIUS) {
                    continue;
                }
                seen[b as usize * nx + a as usize] = true;
                queue.push((a, b));
            }
        }
        let reach = |x: f32, z: f32, r: f32| {
            (0..nz).any(|j| {
                (0..nx).any(|i| {
                    seen[j * nx + i]
                        && (x0 + i as f32 * step - x).hypot(z0 + j as f32 * step - z) < r
                })
            })
        };
        for room in &p.rooms {
            let (cx, cz) = room.centre();
            assert!(reach(cx, cz, 6.0), "place {}", room.id);
        }
        for h in &p.hotspots {
            assert!(reach(h.x, h.z, h.radius * 0.9), "hotspot {}", h.id);
        }
    }

    #[test]
    fn cliffs_drop_whole_on_the_camera_side() {
        if !available() {
            return;
        }
        let p = plan();
        let plaza = p.rooms.iter().find(|r| r.id == "plaza").unwrap();
        let yaw = std::f32::consts::FRAC_PI_4;
        let down: Vec<&Wall> = p
            .walls
            .iter()
            .filter(|w| p.wall_drops(w, plaza, yaw))
            .collect();
        assert!(!down.is_empty());
        assert!(down
            .iter()
            .all(|w| (w.axis == "z" && (w.at - plaza.z1).abs() < 1e-3)
                || (w.axis == "x" && (w.at - plaza.x1).abs() < 1e-3)));
    }
}
