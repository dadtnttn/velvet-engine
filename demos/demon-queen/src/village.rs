//! The villa: the village the Queen walks into at the end of the forest trail.
//!
//! It was designed and built in the Three.js prototype (`demos/demon-queen-village-threejs`)
//! and exported by its `tools/export-game.mjs`: the baked GLB parts live in
//! `data/assets/3d/villa/`, the plan in `data/villa/villa.json`. The plan's origin is the
//! plaza's middle (x east, z south, as in the game); the game puts it at
//! `fantasy_village_center()`. This module reads the plan and answers what the rest of the game
//! asks of the village: where she may walk, how high the ground is, which door she stands at,
//! where the gate back to the forest is, its lamps, its plants and its map.
//! (Re-export after any change to the prototype.)
use super::*;
use serde::Deserialize;

/// Half the side of the square her movement is clamped to, round the plaza (the real limits are
/// `blocked`'s: the clearing's edge, the houses, the brook, the fences).
pub(super) const RADIUS: f32 = 30.0;
/// How near a door the prompt shows, and how near F works: measured from the spot in front of
/// the door (the prototype's hotspot).
pub(super) const BUILDING_SHOW_RADIUS: f32 = 5.2;
pub(super) const BUILDING_INTERACT_RADIUS: f32 = 1.6;
/// Her radius against walls, posts and fences.
const BODY: f32 = 0.3;
/// The paving (and the grass) lie this far above the bare ground she walks on.
const SURFACE_LIFT: f32 = 0.025;
/// How many of the lamps nearest her light the street at once.
const LAMP_LIGHTS: usize = 28;
/// Lamps fade out between these distances from her, so none ever switches off at once.
const LAMP_FADE: [f32; 2] = [14.0, 22.0];

// ------------------------------------------------------------------ the plan
#[derive(Deserialize, Clone, Copy)]
pub(super) struct Circle {
    pub x: f32,
    pub z: f32,
    pub r: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(super) struct Area {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(super) struct Gate {
    pub x: f32,
    pub z: f32,
    pub w: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(super) struct Bridge {
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub len: f32,
}

#[derive(Deserialize, Clone, Copy)]
pub(super) struct Forecourt {
    pub x0: f32,
    pub x1: f32,
    pub z0: f32,
    pub z1: f32,
    pub h: f32,
}

#[derive(Deserialize)]
pub(super) struct Street {
    #[allow(dead_code)]
    pub id: String,
    pub kind: String,
    pub w: f32,
    pub pts: Vec<[f32; 2]>,
}

#[derive(Deserialize)]
pub(super) struct Brook {
    pub w: f32,
    pub pts: Vec<[f32; 2]>,
}

#[derive(Deserialize, Clone, Copy)]
pub(super) struct Door {
    pub x: f32,
    pub z: f32,
    pub nx: f32,
    pub nz: f32,
}

#[derive(Deserialize)]
pub(super) struct Building {
    pub id: String,
    #[allow(dead_code)]
    pub short: String,
    pub x: f32,
    pub z: f32,
    pub w: f32,
    pub d: f32,
    pub yaw: f32,
    pub door: Door,
    pub corners: Vec<[f32; 2]>,
}

#[derive(Deserialize, Clone, Copy)]
struct BoxCollider {
    x: f32,
    z: f32,
    hw: f32,
    hd: f32,
    yaw: f32,
}

#[derive(Deserialize)]
struct Fence {
    pts: Vec<[f32; 2]>,
    #[serde(default)]
    closed: bool,
}

#[derive(Deserialize)]
pub(super) struct Hotspot {
    pub id: String,
    pub kind: String,
    #[allow(dead_code)]
    pub label: String,
    #[allow(dead_code)]
    pub title: String,
    pub x: f32,
    pub z: f32,
    #[allow(dead_code)]
    pub r: f32,
    #[serde(default)]
    pub text: String,
}

/// The villa's places that are not houses (Grizna's corral): the nearest one in reach, if any.
pub(super) fn talk_spot(position: Vec3) -> Option<&'static Hotspot> {
    let (x, z) = local(position.x, position.z);
    plan()
        .hotspots
        .iter()
        .filter(|h| h.kind == "talk" && !h.id.starts_with("villa_"))
        .map(|h| (h, (h.x - x).hypot(h.z - z)))
        .filter(|(h, d)| *d <= h.r.max(1.6))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(h, _)| h)
}

/// A villa spot by id: where it is in the world, and its title.
pub(super) fn spot(id: &str) -> Option<(Vec3, &'static str)> {
    let h = plan().hotspots.iter().find(|h| h.id == id)?;
    let (x, z) = world(h.x, h.z);
    Some((Vec3::new(x, surface_height(x, z), z), h.title.as_str()))
}

/// The square she can walk in, in the world (x0, x1, z0, z1): the plan's bounds.
pub(super) fn walk_bounds() -> [f32; 4] {
    let b = &plan().bounds;
    let (x0, z0) = world(b.x0, b.z0);
    let (x1, z1) = world(b.x1, b.z1);
    [x0.min(x1), x0.max(x1), z0.min(z1), z0.max(z1)]
}

#[derive(Deserialize, Clone, Copy)]
pub(super) struct Spawn {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
}

#[derive(Deserialize)]
pub(super) struct Lamp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub power: f32,
    pub color: String,
}

#[derive(Deserialize)]
pub(super) struct Steam {
    pub x: f32,
    pub z: f32,
    pub r: f32,
}

#[derive(Deserialize)]
pub(super) struct Mill {
    /// The wheel's frame in the village (column-major), its spin about the frame's z axis.
    pub matrix: [f32; 16],
    pub speed: f32,
}

#[derive(Deserialize)]
pub(super) struct Plant {
    pub model: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub s: f32,
    pub yaw: f32,
    /// The leaves' tint, for the species whose leaves vary plant to plant.
    pub tint: Option<[f32; 3]>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Plan {
    pub clearing: Circle,
    pub plaza: Circle,
    #[allow(dead_code)]
    pub bounds: Area,
    pub gate: Gate,
    pub bridge: Bridge,
    pub forecourt: Forecourt,
    pub streets: Vec<Street>,
    pub brook: Brook,
    pub pool_fence: Area,
    pub buildings: Vec<Building>,
    circles: Vec<Circle>,
    boxes: Vec<BoxCollider>,
    fences: Vec<Fence>,
    pub hotspots: Vec<Hotspot>,
    pub spawn: Spawn,
    pub lamps: Vec<Lamp>,
    pub smoke: Vec<[f32; 3]>,
    pub steam: Vec<Steam>,
    pub mill: Mill,
    pub plants: Vec<Plant>,
}

pub(super) fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("villa/villa.json")).expect("valid villa plan")
    })
}

// ------------------------------------------------------------- geometry
/// The plan's frame from the world's, and back.
fn local(x: f32, z: f32) -> (f32, f32) {
    let c = fantasy_village_center();
    (x - c.x, z - c.z)
}
fn world(x: f32, z: f32) -> (f32, f32) {
    let c = fantasy_village_center();
    (x + c.x, z + c.z)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Distance from a point to the segment a-b.
fn segment_distance(px: f32, pz: f32, a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    let t = (((px - a[0]) * dx + (pz - a[1]) * dz) / (dx * dx + dz * dz).max(1e-9)).clamp(0.0, 1.0);
    (px - a[0] - dx * t).hypot(pz - a[1] - dz * t)
}

/// Distance from a point to a polyline.
fn line_distance(px: f32, pz: f32, pts: &[[f32; 2]]) -> f32 {
    pts.windows(2)
        .map(|w| segment_distance(px, pz, w[0], w[1]))
        .fold(f32::INFINITY, f32::min)
}

/// Signed distance to an oriented box (negative inside).
fn box_distance(px: f32, pz: f32, b: BoxCollider) -> f32 {
    let (s, c) = b.yaw.sin_cos();
    let (dx, dz) = (px - b.x, pz - b.z);
    let u = (dx * c - dz * s).abs() - b.hw;
    let v = (dx * s + dz * c).abs() - b.hd;
    u.max(0.0).hypot(v.max(0.0)) + u.max(v).min(0.0)
}

fn building_box(b: &Building) -> BoxCollider {
    BoxCollider {
        x: b.x,
        z: b.z,
        hw: b.w / 2.0,
        hd: b.d / 2.0,
        yaw: b.yaw,
    }
}

/// Over the brook's water (not on the bridge), in the plan's frame.
fn in_brook(x: f32, z: f32, grow: f32) -> bool {
    let p = plan();
    if (x - p.bridge.x).abs() < p.bridge.w / 2.0 + 0.05
        && (z - p.bridge.z).abs() < p.bridge.len / 2.0 + 0.6
    {
        return false;
    }
    line_distance(x, z, &p.brook.pts) < p.brook.w / 2.0 + grow
}

/// Can a body of radius r stand at (x, z) of the plan? (The prototype's `blocked`.)
fn blocked_local(x: f32, z: f32, r: f32) -> bool {
    let p = plan();
    let on_road_out = x.abs() < 1.55 && z > p.gate.z - 0.5 && z < p.bounds.z1 - 1.0;
    if !on_road_out && (x - p.clearing.x).hypot(z - p.clearing.z) > p.clearing.r - r {
        return true;
    }
    if z > p.gate.z - 0.4
        && z < p.gate.z + 0.4
        && x.abs() > p.gate.w / 2.0 - r
        && x.abs() < p.gate.w / 2.0 + 9.0
    {
        return true;
    }
    if p.buildings
        .iter()
        .any(|b| box_distance(x, z, building_box(b)) < r)
    {
        return true;
    }
    if p.circles.iter().any(|c| (x - c.x).hypot(z - c.z) < c.r + r) {
        return true;
    }
    if p.boxes.iter().any(|b| box_distance(x, z, *b) < r) {
        return true;
    }
    for f in &p.fences {
        let mut pts = f.pts.clone();
        if f.closed {
            if let Some(first) = f.pts.first() {
                pts.push(*first);
            }
        }
        if line_distance(x, z, &pts) < 0.12 + r {
            return true;
        }
    }
    let pf = p.pool_fence;
    x > pf.x0 - r && x < pf.x1 + r && z > pf.z0 - r && z < pf.z1 + r
}

pub(super) fn blocked(position: Vec3) -> bool {
    let (x, z) = local(position.x, position.z);
    blocked_local(x, z, BODY)
}

/// The walking surface of the plan: the bridge humps over the brook, the council's forecourt is a
/// step up, the rest is level. (The prototype's `groundY`.)
fn walk_height(x: f32, z: f32) -> f32 {
    let p = plan();
    let b = p.bridge;
    if (x - b.x).abs() < b.w / 2.0 + 0.1 && (z - b.z).abs() < b.len / 2.0 + 0.9 {
        let t = (1.0 - (z - b.z).abs() / (b.len / 2.0 + 0.9)).clamp(0.0, 1.0);
        return 0.42 * (t * std::f32::consts::FRAC_PI_2).sin();
    }
    let f = p.forecourt;
    if x > f.x0 && x < f.x1 && z > f.z0 && z < f.z1 {
        return f.h;
    }
    0.0
}

/// The bare ground: flat in the clearing, the brook's channel cut into it, forested slopes rising
/// beyond. (The prototype's `terrainY`; the exported terrain mesh follows it.)
fn terrain_height(x: f32, z: f32) -> f32 {
    let p = plan();
    let d = (x - p.clearing.x).hypot(z - p.clearing.z);
    let n = (x * 0.17 + z * 0.05).sin() * 0.6
        + (z * 0.21 - x * 0.09).cos() * 0.5
        + ((x + z) * 0.31).sin() * 0.25;
    let mut y = smooth(p.clearing.r + 1.5, p.clearing.r + 16.0, d) * (2.2 + n);
    let b = line_distance(x, z, &p.brook.pts);
    y -= 0.42 * smooth(p.brook.w / 2.0 + 0.7, p.brook.w / 2.0 - 0.15, b);
    y
}

/// Where her feet go at a world point: on the paving, the grass, the bridge or the forecourt.
pub(super) fn surface_height(x: f32, z: f32) -> f32 {
    let (x, z) = local(x, z);
    // Wading the brook, her feet go down into its bed.
    if in_brook(x, z, 0.0) {
        return terrain_height(x, z) + SURFACE_LIFT;
    }
    walk_height(x, z).max(terrain_height(x, z)) + SURFACE_LIFT
}

// ------------------------------------------------------------- buildings
/// Stable hooks for the villa VNs: the building's id (the story's key) and its display name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BuildingAnchor {
    pub id: &'static str,
    pub label: &'static str,
    pub position: Vec3,
    pub footprint_index: usize,
}

/// The ids the game knows, with the names its maps and prompts show.
fn building_metadata(id: &str) -> (&'static str, &'static str) {
    match id {
        "villa_consejo" => ("villa_consejo", "Casa del Burgomaestre"),
        "villa_burdel" => ("villa_burdel", "Burdel"),
        "villa_tienda" => ("villa_tienda", "Tienda"),
        "villa_taberna" => ("villa_taberna", "Taberna"),
        "villa_casa_aldeanos_a" => ("villa_casa_aldeanos_a", "Casa de aldeanos A"),
        "villa_casa_aldeanos_b" => ("villa_casa_aldeanos_b", "Casa de aldeanos B"),
        "villa_banos_termales" => ("villa_banos_termales", "Baños termales"),
        _ => ("villa_edificio", "Edificio de la villa"),
    }
}

/// The doors that open on nothing yet: the place's name for the «En desarrollo» notice. `None`
/// for a door with something behind it.
pub(super) fn in_development(id: &str) -> Option<&'static str> {
    match id {
        "villa_taberna" => Some("Taberna «El Jabalí Dorado»"),
        "villa_consejo" => Some("Casa del Burgomaestre"),
        "villa_casa_aldeanos_a" | "villa_casa_aldeanos_b" => Some("Casa de aldeanos"),
        _ => None,
    }
}

/// The spot in front of a building's door (where the prototype's hotspot is), in the plan.
fn door_spot(b: &Building) -> (f32, f32) {
    plan()
        .hotspots
        .iter()
        .find(|h| h.id == b.id)
        .map(|h| (h.x, h.z))
        .unwrap_or((b.door.x + b.door.nx * 1.15, b.door.z + b.door.nz * 1.15))
}

/// The anchors never move during a session: computed once.
pub(super) fn building_anchors() -> &'static [BuildingAnchor] {
    static DATA: OnceLock<Vec<BuildingAnchor>> = OnceLock::new();
    DATA.get_or_init(|| {
        plan()
            .buildings
            .iter()
            .enumerate()
            .map(|(footprint_index, b)| {
                let (id, label) = building_metadata(&b.id);
                let (x, z) = world(b.x, b.z);
                BuildingAnchor {
                    id,
                    label,
                    position: Vec3::new(x, surface_height(x, z), z),
                    footprint_index,
                }
            })
            .collect()
    })
}

pub(super) fn building_anchor(footprint_index: usize) -> Option<BuildingAnchor> {
    building_anchors().get(footprint_index).copied()
}

/// How far she is from a building's door (from the spot in front of it).
pub(super) fn building_distance(position: Vec3, footprint_index: usize) -> f32 {
    let Some(b) = plan().buildings.get(footprint_index) else {
        return f32::INFINITY;
    };
    let (x, z) = local(position.x, position.z);
    let (sx, sz) = door_spot(b);
    (x - sx).hypot(z - sz)
}

pub(super) fn nearest_building(position: Vec3) -> Option<(BuildingAnchor, f32)> {
    building_anchors()
        .iter()
        .copied()
        .map(|anchor| (anchor, building_distance(position, anchor.footprint_index)))
        .min_by(|(_, left), (_, right)| left.total_cmp(right))
}

/// Where a building's door is in the world, and which way it faces.
pub(super) fn door_anchor(footprint_index: usize) -> Option<(Vec3, Vec3)> {
    let b = plan().buildings.get(footprint_index)?;
    let (x, z) = world(b.door.x, b.door.z);
    Some((
        Vec3::new(x, surface_height(x, z), z),
        Vec3::new(b.door.nx, 0.0, b.door.nz),
    ))
}

// ------------------------------------------------------------- the gate
/// The way back to the forest: the spot outside the gate.
pub(super) fn entrance() -> Vec3 {
    let p = plan();
    let (lx, lz) = p
        .hotspots
        .iter()
        .find(|h| h.kind == "exit")
        .map(|h| (h.x, h.z))
        .unwrap_or((p.gate.x, p.gate.z + 1.6));
    let (x, z) = world(lx, lz);
    Vec3::new(x, 0.0, z)
}

pub(super) fn entrance_distance(position: Vec3) -> f32 {
    let e = entrance();
    (position.x - e.x).hypot(position.z - e.z)
}

/// Where she stands when she comes in through the gate, and which way she faces.
pub(super) fn spawn() -> (Vec3, f32) {
    let s = plan().spawn;
    let (x, z) = world(s.x, s.z);
    (Vec3::new(x, 0.0, z), s.yaw)
}

// ------------------------------------------------------------- lights
fn hex_color(text: &str) -> [f32; 3] {
    let value = u32::from_str_radix(text.trim_start_matches('#'), 16).unwrap_or(0xffa552);
    [
        ((value >> 16) & 255) as f32 / 255.0,
        ((value >> 8) & 255) as f32 / 255.0,
        (value & 255) as f32 / 255.0,
    ]
}

/// The lamps that light the street at night: the ones nearest `near` (the renderer takes a few
/// at a time), fading on through blue hour, each breathing a little.
pub(super) fn lights(elapsed: f32, moon_visibility: f32, near: Vec3) -> Vec<PointLight3D> {
    let night = ((moon_visibility - 0.12) / 0.48).clamp(0.0, 1.0);
    let (nx, nz) = local(near.x, near.z);
    let mut lamps: Vec<(usize, &Lamp)> = plan().lamps.iter().enumerate().collect();
    lamps.sort_by(|a, b| {
        (a.1.x - nx)
            .hypot(a.1.z - nz)
            .total_cmp(&(b.1.x - nx).hypot(b.1.z - nz))
    });
    lamps
        .into_iter()
        .take(LAMP_LIGHTS)
        .map(|(i, l)| {
            let (x, z) = world(l.x, l.z);
            let pulse = 0.94 + (elapsed * 3.1 + i as f32 * 1.73).sin() * 0.035;
            let fade = 1.0 - smooth(LAMP_FADE[0], LAMP_FADE[1], (l.x - nx).hypot(l.z - nz));
            PointLight3D::new(
                Vec3::new(x, l.y, z),
                hex_color(&l.color),
                3.6 * l.power * night * pulse * fade,
                7.0,
            )
        })
        .collect()
}

// ------------------------------------------------------------- the look
fn mix3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * k,
        a[1] + (b[1] - a[1]) * k,
        a[2] + (b[2] - a[2]) * k,
    ]
}

/// The villa's light, after the prototype's three hours (its lights converted to the renderer's
/// units: a lit face takes 0.75 of the key, the prototype divides by pi): a clear day under a blue
/// sky, a long golden dusk (low orange sun, peach sky, green light off the lawn) and a deep blue
/// night where the lamps do the work. The hour follows the sun's height in the cycle. The haze
/// stays far off: the camera stands 18 m from her, and the cycle's haze began at 19 m and washed
/// the whole village grey-green.
pub(super) fn mood(mut env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    // (key, sky, light off the ground) per hour.
    const DAY: [[f32; 3]; 3] = [[1.12, 1.01, 0.80], [0.28, 0.35, 0.42], [0.10, 0.17, 0.04]];
    const DUSK: [[f32; 3]; 3] = [[1.22, 0.74, 0.38], [0.37, 0.29, 0.23], [0.07, 0.11, 0.04]];
    const NIGHT: [[f32; 3]; 3] = [[0.10, 0.14, 0.32], [0.05, 0.08, 0.20], [0.02, 0.03, 0.04]];
    // The hours melt into each other slowly, as they do outdoors: the night thins out over the
    // last stretch before the sun reaches the horizon, the golden light peaks with the sun on it,
    // and the day takes over while the sun climbs. (A 7-minute day: each change takes well over
    // a minute; narrow bands made the colour jump in a quarter of that.)
    // The blend follows the sun's elevation angle, which moves at an even pace through the day
    // (its height jumps fastest right at the horizon). The renderer's sun direction is
    // (-cos·0.72, sin·0.82, sin·0.72) normalised; `sin` is recovered from its height.
    let y = env.celestial.sun_direction.y.clamp(-0.999, 0.999);
    let sin = (y * 0.72 / (0.82 * (1.0 - y * y).sqrt())).clamp(-1.0, 1.0);
    let elevation = sin.asin().to_degrees();
    let night = 1.0 - smooth(-70.0, 4.0, elevation);
    let day = smooth(-4.0, 70.0, elevation);
    let dusk = (1.0 - night - day).max(0.0);
    let lighting = &mut env.lighting;
    let own = [
        lighting.key_color,
        lighting.ambient_color,
        lighting.bounce_color,
    ];
    let mut out = [[0.0f32; 3]; 3];
    for (i, row) in out.iter_mut().enumerate() {
        // The night keeps a little of the cycle's own fill under the prototype's deep blue.
        let night_light = mix3(own[i], NIGHT[i], 0.8);
        for (k, v) in row.iter_mut().enumerate() {
            *v = DAY[i][k] * day + DUSK[i][k] * dusk + night_light[k] * night;
        }
    }
    lighting.key_color = out[0];
    lighting.ambient_color = out[1];
    lighting.bounce_color = out[2];
    // A softer moon: its sheen turned the paving white; the lamps light the streets.
    env.celestial.moon_intensity *= 1.0 - 0.55 * night;
    env.celestial.reflection_strength *= 1.0 - 0.75 * night;
    let fog = &mut env.atmosphere;
    fog.fog_start = 34.0;
    fog.fog_end = 105.0;
    fog.fog_density = 0.0;
    fog.height_falloff = 0.0;
    env
}

/// The villa's toon finish, as the prototype draws it: dark brown ink round every shape and
/// crease, fading into the forest, and colour a little richer than the renderer's plain grade.
pub(super) fn finish() -> Finish3D {
    Finish3D {
        ink: 0.75,
        // The prototype's line colour (#2a1a0e), linear.
        ink_color: [0.023, 0.010, 0.0044],
        ink_thickness: 1.0,
        ink_fade: [30.0, 70.0],
        saturation: 1.07,
        contrast: 1.03,
        // Lamps, flames and glowing things bloom, as in the prototype.
        bloom: 0.32,
        bloom_threshold: 0.9,
        exposure: 1.0,
        // Soft contact shadow where things meet the ground.
        occlusion: 0.32,
        occlusion_radius: 0.55,
        // Light bounced off the lit ground and walls onto what is round them.
        gi: 1.8,
        gi_radius: 3.0,
        // (The sky lights from above and off screen: most of its fill stays.)
        gi_ambient: 0.85,
    }
}

/// The villa's toon response: more of the painted relief (cobbles, shingles, plaster) than the
/// renderer's default, and a light rim lifting the silhouettes.
pub(super) fn toon_style() -> velvet_render::ToonStyle3D {
    velvet_render::ToonStyle3D {
        normal_detail: 0.55,
        rim: 0.09,
        ..Default::default()
    }
}

// ------------------------------------------------------------- the map
#[derive(Clone, Copy)]
pub(super) struct VegetationPlacement {
    pub asset: &'static str,
    pub x: f32,
    pub z: f32,
    pub scale: f32,
    #[allow(dead_code)]
    pub rotation: f32,
}

/// The trees and bushes, in the world, for the map's leaf marks.
pub(super) fn vegetation_placements() -> &'static [VegetationPlacement] {
    static DATA: OnceLock<Vec<VegetationPlacement>> = OnceLock::new();
    DATA.get_or_init(|| {
        plan()
            .plants
            .iter()
            .filter(|p| {
                p.model.contains("Tree") || p.model.contains("Pine") || p.model.contains("Bush")
            })
            .map(|p| {
                let (x, z) = world(p.x, p.z);
                VegetationPlacement {
                    asset: p.model.as_str(),
                    x,
                    z,
                    scale: p.s,
                    rotation: p.yaw,
                }
            })
            .collect()
    })
}

/// A building's outline, a street's stretch: plan polygons for the map.
pub(super) struct Footprint {
    pub name: String,
    pub polygon: Vec<[f32; 2]>,
}
pub(super) struct Road {
    pub polygon: Vec<[f32; 2]>,
    /// The same piece grown by the kerb's width: drawn first, dark, it outlines the streets.
    pub outline: Vec<[f32; 2]>,
    #[allow(dead_code)]
    pub height: f32,
}
pub(super) struct Layout {
    pub footprints: Vec<Footprint>,
    pub roads: Vec<Road>,
    #[allow(dead_code)]
    pub entrance: [f32; 2],
}

/// A ring of `n` points round (x, z).
fn ring(x: f32, z: f32, r: f32, n: usize) -> Vec<[f32; 2]> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            [x + a.cos() * r, z + a.sin() * r]
        })
        .collect()
}

/// The village as the map draws it (in the plan's frame): the buildings' outlines (named as the
/// map knows them) and the paving as convex pieces (a quad per stretch of street, a disc at every
/// bend, the plaza).
pub(super) fn layout() -> &'static Layout {
    static DATA: OnceLock<Layout> = OnceLock::new();
    DATA.get_or_init(|| {
        let p = plan();
        let footprints = p
            .buildings
            .iter()
            .map(|b| Footprint {
                name: building_metadata(&b.id).1.to_owned(),
                polygon: b.corners.clone(),
            })
            .collect();
        const KERB: f32 = 0.3;
        let disc = |x: f32, z: f32, r: f32, n: usize| Road {
            polygon: ring(x, z, r, n),
            outline: ring(x, z, r + KERB, n),
            height: SURFACE_LIFT,
        };
        let mut roads = vec![disc(p.plaza.x, p.plaza.z, p.plaza.r, 32)];
        for street in &p.streets {
            let hw = street.w / 2.0;
            let quad = |a: [f32; 2], b: [f32; 2], hw: f32| {
                let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
                let len = dx.hypot(dz).max(1e-6);
                let (nx, nz) = (-dz / len * hw, dx / len * hw);
                vec![
                    [a[0] + nx, a[1] + nz],
                    [b[0] + nx, b[1] + nz],
                    [b[0] - nx, b[1] - nz],
                    [a[0] - nx, a[1] - nz],
                ]
            };
            for w in street.pts.windows(2) {
                roads.push(Road {
                    polygon: quad(w[0], w[1], hw),
                    outline: quad(w[0], w[1], hw + KERB),
                    height: SURFACE_LIFT,
                });
            }
            for pt in &street.pts {
                roads.push(disc(pt[0], pt[1], hw, 12));
            }
            let _ = street.kind.as_str();
        }
        let entrance = {
            let e = entrance();
            let (x, z) = local(e.x, e.z);
            [x, z]
        };
        Layout {
            footprints,
            roads,
            entrance,
        }
    })
}

/// The clearing's middle in the world and its radius (the map's boundary ring).
pub(super) fn clearing() -> (Vec3, f32) {
    let c = plan().clearing;
    let (x, z) = world(c.x, c.z);
    (Vec3::new(x, 0.0, z), c.r)
}

/// The brook's course in the world (the map draws it).
pub(super) fn brook_path() -> (Vec<[f32; 2]>, f32) {
    let b = &plan().brook;
    (
        b.pts
            .iter()
            .map(|p| {
                let (x, z) = world(p[0], p[1]);
                [x, z]
            })
            .collect(),
        b.w,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_plan_reads_and_holds_the_seven_buildings() {
        let p = plan();
        assert_eq!(p.buildings.len(), 7);
        assert!(p.plants.len() > 1000, "plants: {}", p.plants.len());
        assert!(!p.lamps.is_empty() && !p.smoke.is_empty());
        let ids: Vec<&str> = building_anchors().iter().map(|a| a.id).collect();
        for id in [
            "villa_consejo",
            "villa_burdel",
            "villa_tienda",
            "villa_taberna",
            "villa_casa_aldeanos_a",
            "villa_casa_aldeanos_b",
            "villa_banos_termales",
        ] {
            assert!(ids.contains(&id), "{id}");
        }
    }

    #[test]
    fn she_walks_the_streets_but_not_through_walls_or_water() {
        let c = fantasy_village_center();
        let (spawn, _) = spawn();
        assert!(!blocked(spawn), "the spawn is free");
        assert!(!blocked(entrance()), "the way out is free");
        for b in &plan().buildings {
            let (x, z) = world(b.x, b.z);
            assert!(blocked(Vec3::new(x, 0.0, z)), "inside {}", b.id);
            let (sx, sz) = door_spot(b);
            let (x, z) = world(sx, sz);
            assert!(
                !blocked(Vec3::new(x, 0.0, z)),
                "in front of {}'s door",
                b.id
            );
        }
        // The plaza is open; the well in its middle is not.
        assert!(!blocked(c + Vec3::new(3.0, 0.0, 2.0)));
        assert!(blocked(c));
        // She can wade the brook (down in its bed); the bridge carries her over it dry.
        let p = plan();
        let (bx, bz) = world(p.brook.pts[3][0], p.brook.pts[3][1]);
        assert!(!blocked(Vec3::new(bx, 0.0, bz)), "the brook can be waded");
        assert!(
            surface_height(bx, bz) < -0.15,
            "her feet go down into the brook's bed: {}",
            surface_height(bx, bz)
        );
        let (x, z) = world(p.bridge.x, p.bridge.z);
        assert!(!blocked(Vec3::new(x, 0.0, z)), "on the bridge");
        assert!(surface_height(x, z) > 0.4, "the bridge humps");
        // Beyond the clearing the forest is closed.
        assert!(blocked(c + Vec3::new(RADIUS, 0.0, 0.0)));
    }

    #[test]
    fn every_door_is_reachable_from_the_spawn() {
        // A flood fill over a 0.5 m grid, as the prototype's flow test walks it.
        let (spawn, _) = spawn();
        let c = fantasy_village_center();
        let key = |x: f32, z: f32| ((x / 0.5).round() as i32, (z / 0.5).round() as i32);
        let start = key(spawn.x - c.x, spawn.z - c.z);
        let mut seen = std::collections::HashSet::from([start]);
        let mut queue = std::collections::VecDeque::from([start]);
        while let Some((i, j)) = queue.pop_front() {
            for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let next = (i + di, j + dj);
                if seen.contains(&next)
                    || blocked_local(next.0 as f32 * 0.5, next.1 as f32 * 0.5, 0.28)
                {
                    continue;
                }
                seen.insert(next);
                queue.push_back(next);
            }
        }
        for b in &plan().buildings {
            let (sx, sz) = door_spot(b);
            let near = seen
                .iter()
                .any(|&(i, j)| (i as f32 * 0.5 - sx).hypot(j as f32 * 0.5 - sz) < 0.8);
            assert!(near, "{} unreachable", b.id);
        }
        let (ex, ez) = local(entrance().x, entrance().z);
        assert!(
            seen.iter()
                .any(|&(i, j)| (i as f32 * 0.5 - ex).hypot(j as f32 * 0.5 - ez) < 0.8),
            "the gate unreachable"
        );
    }

    #[test]
    fn the_light_changes_gently_through_the_day() {
        // The game's cycle: 420 s a day. Second by second, no colour of the light moves by more
        // than a small step, so the hours never jump.
        let preset = EnvironmentPreset3D::Lived;
        let seconds = 420;
        let light = |i: usize| {
            let env = mood(preset.sample(i as f32 / seconds as f32));
            let l = env.lighting;
            [l.key_color, l.ambient_color, l.bounce_color]
        };
        let mut worst = (0.0f32, 0usize, 0usize);
        for i in 0..seconds {
            let (a, b) = (light(i), light(i + 1));
            for (j, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                for k in 0..3 {
                    let d = (x[k] - y[k]).abs();
                    if d > worst.0 {
                        worst = (d, i, j);
                    }
                }
            }
        }
        let raw = |i: usize| preset.sample(i as f32 / seconds as f32);
        let (d, i, j) = worst;
        assert!(
            d < 0.025,
            "the light jumps {d} in one second at {i} s (light {j}): sun y {} -> {}, cycle key {:?} -> {:?}",
            raw(i).celestial.sun_direction.y,
            raw(i + 1).celestial.sun_direction.y,
            raw(i).lighting.key_color,
            raw(i + 1).lighting.key_color
        );
    }

    #[test]
    fn lamps_light_only_at_night_and_only_the_nearest() {
        let c = fantasy_village_center();
        let day = lights(0.0, 0.0, c);
        let night = lights(0.0, 1.0, c);
        assert!(day.iter().all(|light| light.intensity < 0.01));
        assert!(night.iter().any(|light| light.intensity > 1.0));
        assert!(night.len() <= LAMP_LIGHTS);
    }

    #[test]
    fn doors_face_out_and_prompts_find_the_nearest() {
        for (i, anchor) in building_anchors().iter().enumerate() {
            let (door, out) = door_anchor(i).expect("every building has a door");
            assert!((out.length() - 1.0).abs() < 0.05, "{} faces out", anchor.id);
            let (nearest, d) = nearest_building(door + out * 1.15).expect("anchors");
            assert_eq!(nearest.id, anchor.id);
            assert!(d <= BUILDING_INTERACT_RADIUS);
        }
    }
}
