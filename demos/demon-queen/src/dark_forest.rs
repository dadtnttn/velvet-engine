//! The Dark Forest: the wood between the castle's gate and the villa's, as it was designed in the
//! Three.js prototype (`demos/demon-queen-forest-threejs`) and exported by its
//! `tools/export-game.mjs`: the baked GLB parts live in `data/assets/3d/bosque/`, the plan in
//! `data/bosque/bosque.json`. The prototype is laid out in the game's own frame (the trail is
//! `nature_trail_center`, the castle's gate is at z -12, the villa's at z 20.05, the mine and the
//! mage where they always were), so the plan needs no offset. This module reads the plan and
//! answers what the rest of the game asks of the wood: where she may walk, how high the ground
//! is, its lamps, fires and glowing fungi, and its light at each hour.
//! (Re-export after any change to the prototype.)
use super::*;
use serde::Deserialize;

/// Her radius against trunks, posts and stones.
const BODY: f32 = 0.28;
/// Her feet stand this far above the bare ground (the trail's earth and the moss lie on it).
const SURFACE_LIFT: f32 = 0.02;
/// How many of the lights nearest her are lit at once.
const LIGHTS: usize = 28;
/// Lights fade out between these distances from her, so none ever switches off at once.
const FADE: [f32; 2] = [17.0, 25.0];

// ------------------------------------------------------------------ the plan
#[derive(Deserialize, Clone, Copy)]
pub(super) struct Circle {
    pub x: f32,
    pub z: f32,
    pub r: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct BoxCollider {
    x: f32,
    z: f32,
    hw: f32,
    hd: f32,
    yaw: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Castle {
    z: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Gate {
    z: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Pit {
    x0: f32,
    x1: f32,
    z0: f32,
    z1: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Ring {
    x: f32,
    z: f32,
    r: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Log {
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    r: f32,
}

#[derive(Deserialize, Clone, Copy)]
struct Pool {
    x: f32,
    z: f32,
    rx: f32,
    rz: f32,
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
pub(super) struct Lamp {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub power: f32,
    pub color: String,
    /// "lamp" (lanterns: lit at dusk), "fire" (braziers, torches, the campfire: always burning)
    /// or "glow" (fungi, runes, crystals: always faint, brighter at night).
    pub kind: String,
}

#[derive(Deserialize)]
pub(super) struct Plant {
    pub model: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub s: f32,
    pub yaw: f32,
    pub tint: Option<[f32; 3]>,
}

/// A spot the prototype marks: the gates and the way to el Peñascal (kind "exit").
#[derive(Deserialize)]
pub(super) struct Spot {
    pub id: String,
    pub kind: String,
    #[allow(dead_code)]
    pub label: String,
    pub title: String,
    pub x: f32,
    pub z: f32,
    pub r: f32,
    #[serde(default)]
    #[allow(dead_code)]
    pub text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Plan {
    limit: f32,
    castle: Castle,
    village_gate: Gate,
    pit: Pit,
    shrine: Ring,
    fallen: Log,
    pool: Pool,
    circles: Vec<Circle>,
    boxes: Vec<BoxCollider>,
    heights: Heights,
    pub lamps: Vec<Lamp>,
    pub smoke: Vec<[f32; 3]>,
    pub plants: Vec<Plant>,
    #[serde(default)]
    pub hotspots: Vec<Spot>,
}

/// One of the wood's exits by id ("castillo", "villa", "penascal"): where it is (on the ground) and
/// how near she must be to take it.
pub(super) fn exit(id: &str) -> Option<(Vec3, f32)> {
    plan()
        .hotspots
        .iter()
        .find(|h| h.kind == "exit" && h.id == id)
        .map(|h| (Vec3::new(h.x, ground_height(h.x, h.z), h.z), h.r))
}

/// A spot of the wood by id (an exit, a landmark): where it is on the ground, and its title.
pub(super) fn spot(id: &str) -> Option<(Vec3, &'static str)> {
    plan().hotspots.iter().find(|h| h.id == id).map(|h| {
        (
            Vec3::new(h.x, ground_height(h.x, h.z), h.z),
            h.title.as_str(),
        )
    })
}

/// The square she can walk in (x0, x1, z0, z1): between the castle's wall and the villa's fence,
/// the wood's limit either side.
pub(super) fn walk_bounds() -> [f32; 4] {
    let p = plan();
    [
        -p.limit,
        p.limit,
        p.castle.z + 1.0,
        p.village_gate.z.min(p.limit),
    ]
}

pub(super) fn plan() -> &'static Plan {
    static PLAN: OnceLock<Plan> = OnceLock::new();
    PLAN.get_or_init(|| {
        serde_json::from_str(&plan_text("bosque/bosque.json")).expect("valid forest plan")
    })
}

// ------------------------------------------------------------- geometry
fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn segment_distance(px: f32, pz: f32, a: [f32; 2], b: [f32; 2]) -> f32 {
    let (dx, dz) = (b[0] - a[0], b[1] - a[1]);
    let t = (((px - a[0]) * dx + (pz - a[1]) * dz) / (dx * dx + dz * dz).max(1e-9)).clamp(0.0, 1.0);
    (px - a[0] - dx * t).hypot(pz - a[1] - dz * t)
}

/// Signed distance to an oriented box (negative inside).
fn box_distance(px: f32, pz: f32, b: BoxCollider) -> f32 {
    let (s, c) = b.yaw.sin_cos();
    let (dx, dz) = (px - b.x, pz - b.z);
    let u = (dx * c - dz * s).abs() - b.hw;
    let v = (dx * s + dz * c).abs() - b.hd;
    u.max(0.0).hypot(v.max(0.0)) + u.max(v).min(0.0)
}

/// Can a body of radius r stand at (x, z)? (The prototype's `blocked`.)
fn blocked_at(x: f32, z: f32, r: f32) -> bool {
    let p = plan();
    if x.abs() > p.limit - r || z > p.limit - r {
        return true;
    }
    // The castle's curtain wall, the village's fence and gate (both gates are F, not walking).
    if z < p.castle.z + 1.2 + r || z > p.village_gate.z - 0.35 - r {
        return true;
    }
    let g = r + 0.15;
    if x > p.pit.x0 - g && x < p.pit.x1 + g && z > p.pit.z0 - g && z < p.pit.z1 + g {
        return true;
    }
    if p.circles.iter().any(|c| (x - c.x).hypot(z - c.z) < c.r + r) {
        return true;
    }
    if p.boxes.iter().any(|b| box_distance(x, z, *b) < r) {
        return true;
    }
    // The shrine's ring of standing stones, open towards the trail.
    let s = p.shrine;
    if ((x - s.x).hypot(z - s.z) - s.r).abs() < 0.35 + r {
        let a = (z - s.z).atan2(x - s.x) - 0.18;
        if a.sin().atan2(a.cos()).abs() > 0.55 {
            return true;
        }
    }
    let f = p.fallen;
    segment_distance(x, z, [f.x0, f.z0], [f.x1, f.z1]) < f.r + r
}

/// How far into the pool (x, z) is: 0 on the bank, 1 in its middle. She wades in it.
pub(super) fn in_pool(x: f32, z: f32) -> f32 {
    let w = plan().pool;
    (1.0 - ((x - w.x) / w.rx).hypot((z - w.z) / w.rz)).clamp(0.0, 1.0)
}

pub(super) fn blocked(position: Vec3) -> bool {
    blocked_at(position.x, position.z, BODY)
}

/// The bare ground's height (the prototype's `groundY`, sampled on its half-metre grid).
pub(super) fn ground_height(x: f32, z: f32) -> f32 {
    let h = &plan().heights;
    let fx = ((x - h.x0) / h.step).clamp(0.0, (h.nx - 1) as f32 - 1e-3);
    let fz = ((z - h.z0) / h.step).clamp(0.0, (h.nz - 1) as f32 - 1e-3);
    let (i, j) = (fx.floor() as usize, fz.floor() as usize);
    let (u, v) = (fx - i as f32, fz - j as f32);
    let at = |i: usize, j: usize| h.values[j * h.nx + i];
    let a = at(i, j) + (at(i + 1, j) - at(i, j)) * u;
    let b = at(i, j + 1) + (at(i + 1, j + 1) - at(i, j + 1)) * u;
    a + (b - a) * v
}

/// Where her feet go: on the floor, and down on the pool's bed when she wades in (the water
/// comes up to her shins).
pub(super) fn surface_height(x: f32, z: f32) -> f32 {
    let wade = smooth(0.0, 0.45, in_pool(x, z)) * 0.28;
    ground_height(x, z) + SURFACE_LIFT - wade
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

/// The same colour, linear (the renderer's lights and haze are linear).
fn linear(text: &str) -> [f32; 3] {
    hex_color(text).map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    })
}

/// How dark it is, from the moon's visibility (0 by day, 1 at night).
pub(super) fn night(moon_visibility: f32) -> f32 {
    ((moon_visibility - 0.12) / 0.48).clamp(0.0, 1.0)
}

/// The lights nearest `near`: the lanterns along the trail come on at dusk, the braziers, torches
/// and the campfire always burn (and flicker), the fungi, the runes and the crystals give a cold
/// glow that grows with the night.
pub(super) fn lights(elapsed: f32, moon_visibility: f32, near: Vec3) -> Vec<PointLight3D> {
    let night = night(moon_visibility);
    let mut lamps: Vec<(usize, &Lamp)> = plan().lamps.iter().enumerate().collect();
    lamps.sort_by(|a, b| {
        (a.1.x - near.x)
            .hypot(a.1.z - near.z)
            .total_cmp(&(b.1.x - near.x).hypot(b.1.z - near.z))
    });
    lamps
        .into_iter()
        .take(LIGHTS)
        .map(|(i, l)| {
            let k = i as f32;
            let (strength, radius) = match l.kind.as_str() {
                "fire" => {
                    let flicker = 1.0
                        + (elapsed * 9.0 + k).sin() * 0.08
                        + (elapsed * 17.0 + k * 2.0).sin() * 0.05;
                    ((0.9 + 0.4 * night) * flicker, 7.5)
                }
                "glow" => (0.25 + 0.5 * night, 5.0),
                _ => (
                    (0.15 + 0.85 * night) * (0.96 + (elapsed * 3.0 + k * 1.7).sin() * 0.04),
                    7.5,
                ),
            };
            let fade = 1.0 - smooth(FADE[0], FADE[1], (l.x - near.x).hypot(l.z - near.z));
            PointLight3D::new(
                Vec3::new(l.x, l.y, l.z),
                hex_color(&l.color),
                5.5 * l.power * strength * fade,
                radius,
            )
        })
        .collect()
}

/// Where the campfire's smoke rises.
pub(super) fn smoke() -> Vec<Vec3> {
    plan()
        .smoke
        .iter()
        .map(|p| Vec3::new(p[0], p[1], p[2]))
        .collect()
}

// ------------------------------------------------------------- the look
struct Hour {
    sun: &'static str,
    sun_i: f32,
    sky: &'static str,
    ground: &'static str,
    hemi_i: f32,
    fog: &'static str,
    near: f32,
    far: f32,
}

/// The prototype's three hours: a cool green day under the canopy, a long violet-and-teal dusk,
/// a deep blue night.
const DAY: Hour = Hour {
    sun: "#fff0d0",
    sun_i: 2.3,
    sky: "#bcd8c8",
    ground: "#3a5a2a",
    hemi_i: 1.15,
    fog: "#7a9a88",
    near: 10.0,
    far: 70.0,
};
const DUSK: Hour = Hour {
    sun: "#ffb090",
    sun_i: 1.8,
    sky: "#8a7ab0",
    ground: "#2a4030",
    hemi_i: 1.45,
    fog: "#4a4a6a",
    near: 8.0,
    far: 62.0,
};
const NIGHT: Hour = Hour {
    sun: "#8aa6ff",
    sun_i: 0.55,
    sky: "#26365e",
    ground: "#0c160e",
    hemi_i: 0.6,
    fog: "#0a1220",
    near: 6.0,
    far: 52.0,
};

impl Hour {
    /// (key, sky, light off the ground, haze) in the renderer's units: the prototype's lights
    /// converted as the villa's were (a lit face takes 0.75 of the key; three divides by pi).
    fn colours(&self) -> [[f32; 3]; 4] {
        let scale = |c: [f32; 3], k: f32| c.map(|v| v * k);
        [
            scale(linear(self.sun), 0.386 * self.sun_i),
            scale(linear(self.sky), 0.29 * self.hemi_i),
            scale(linear(self.ground), 0.47 * self.hemi_i),
            linear(self.fog),
        ]
    }
}

/// The wood's light, after the prototype's hours, blended by the sun's elevation angle exactly
/// as the villa's is (so the two places change hour together). The haze is the wood's own: it
/// closes in at night and swallows the far trees.
pub(super) fn mood(mut env: EnvironmentFrame3D) -> EnvironmentFrame3D {
    let (day, dusk, night) = hours(&env);
    let (d, k, n) = (DAY.colours(), DUSK.colours(), NIGHT.colours());
    let blend = |i: usize| -> [f32; 3] {
        std::array::from_fn(|c| d[i][c] * day + k[i][c] * dusk + n[i][c] * night)
    };
    env.lighting.key_color = blend(0);
    env.lighting.ambient_color = blend(1);
    env.lighting.bounce_color = blend(2);
    let fog = blend(3);
    env.atmosphere.fog_color = fog;
    env.clear_color = [fog[0], fog[1], fog[2], 1.0];
    // The camera stands about 18 m from her.
    let near = DAY.near * day + DUSK.near * dusk + NIGHT.near * night;
    let far = DAY.far * day + DUSK.far * dusk + NIGHT.far * night;
    env.atmosphere.fog_start = 18.0 + near;
    env.atmosphere.fog_end = 18.0 + far;
    env.atmosphere.fog_density = 0.0;
    env.atmosphere.height_falloff = 0.0;
    // A softer moon: under the canopy the lanterns and fungi light the way.
    env.celestial.moon_intensity *= 1.0 - 0.5 * night;
    env.celestial.reflection_strength *= 1.0 - 0.75 * night;
    // The low sun's orange wash reads as rust on the leaf litter: under the canopy it is violet.
    env.celestial.horizon_tint = [0.62, 0.42, 0.85];
    env
}

/// The wood's toon finish: near-black ink round every shape, fading into the haze, colour a
/// little richer than the renderer's plain grade.
pub(super) fn finish() -> Finish3D {
    Finish3D {
        ink: 0.8,
        // The prototype's line colour (#14100e), linear.
        ink_color: [0.0070, 0.0052, 0.0044],
        ink_thickness: 1.0,
        ink_fade: [26.0, 62.0],
        saturation: 1.08,
        contrast: 1.04,
        // Lamps, flames and glowing things bloom, as in the prototype.
        bloom: 0.3,
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

pub(super) fn toon_style() -> velvet_render::ToonStyle3D {
    velvet_render::ToonStyle3D {
        normal_detail: 0.55,
        rim: 0.09,
        ..Default::default()
    }
}

// ------------------------------------------------------------- the air
/// The wood's air, drawn over the scene each frame: pools of lamplight on the ground under every
/// lantern, fire and glowing thing, the canopy overhead (above the frame: only the sun sees it,
/// so the trail lies in its real, moving leaf shadow, dappled where the light gets through),
/// low mist drifting in the hollows, fireflies over the trail at dusk and night, and wisps round
/// the fungi.
pub(super) struct Air {
    quad: MeshHandle,
    mote: MeshHandle,
    pool: TextureHandle,
    /// Patches of the canopy overhead (two variants), seen only by the sun.
    canopy: [TextureHandle; 2],
    mist: TextureHandle,
}

fn hash(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

/// A soft white disc fading to nothing at its rim.
fn pool_texture() -> TextureData3D {
    let n = 64usize;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (
                (x as f32 + 0.5) / n as f32 * 2.0 - 1.0,
                (y as f32 + 0.5) / n as f32 * 2.0 - 1.0,
            );
            let r = (u * u + v * v).sqrt();
            let a = (1.0 - r).clamp(0.0, 1.0).powf(1.8);
            let i = (y * n + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    TextureData3D {
        width: n as u32,
        height: n as u32,
        rgba,
    }
}

/// A patch of canopy seen from below: leaves in clumps, with the gaps between them where the
/// sun gets through. Only its alpha matters (it is cut out in the shadow map, never drawn).
fn canopy_texture(seed: u32) -> TextureData3D {
    let n = 256usize;
    let mut a = vec![0f32; n * n];
    // Clumps of leaves, each a scatter of small leaf shapes round its middle.
    for c in 0..26u32 {
        let k = seed.wrapping_mul(1000) + c * 11;
        let (cx, cy) = (0.08 + hash(k + 1) * 0.84, 0.08 + hash(k + 2) * 0.84);
        let spread = 0.05 + hash(k + 3) * 0.08;
        for l in 0..60u32 {
            let q = k * 100 + l * 5;
            let r = spread * hash(q + 1).sqrt();
            let t = hash(q + 2) * std::f32::consts::TAU;
            let (lx, ly) = (cx + t.cos() * r, cy + t.sin() * r);
            let size = 0.008 + hash(q + 3) * 0.012;
            let (s, co) = (hash(q + 4) * 3.14).sin_cos();
            let x0 = ((lx - size * 2.0) * n as f32).max(0.0) as usize;
            let x1 = (((lx + size * 2.0) * n as f32) as usize).min(n - 1);
            let y0 = ((ly - size * 2.0) * n as f32).max(0.0) as usize;
            let y1 = (((ly + size * 2.0) * n as f32) as usize).min(n - 1);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (dx, dy) = (
                        (x as f32 + 0.5) / n as f32 - lx,
                        (y as f32 + 0.5) / n as f32 - ly,
                    );
                    let (u, v) = ((dx * co - dy * s) / (size * 1.8), (dx * s + dy * co) / size);
                    if u * u + v * v < 1.0 {
                        a[y * n + x] = 1.0;
                    }
                }
            }
        }
    }
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (u, v) = (
                (x as f32 + 0.5) / n as f32 * 2.0 - 1.0,
                (y as f32 + 0.5) / n as f32 * 2.0 - 1.0,
            );
            let inside = (u * u + v * v).sqrt() < 0.95;
            let i = (y * n + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[
                60,
                90,
                50,
                if inside && a[y * n + x] > 0.5 { 255 } else { 0 },
            ]);
        }
    }
    TextureData3D {
        width: n as u32,
        height: n as u32,
        rgba,
    }
}

/// Mist: soft value noise in a feathered oval.
fn mist_texture() -> TextureData3D {
    let n = 128usize;
    let noise = |x: f32, y: f32, cell: f32, seed: u32| {
        let (fx, fy) = (x / cell, y / cell);
        let (ix, iy) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - ix, fy - iy);
        let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let at = |i: f32, j: f32| {
            hash(
                (i as i32 as u32).wrapping_mul(73_856_093)
                    ^ (j as i32 as u32).wrapping_mul(19_349_663)
                    ^ seed,
            )
        };
        let a = at(ix, iy) + (at(ix + 1.0, iy) - at(ix, iy)) * sx;
        let b = at(ix, iy + 1.0) + (at(ix + 1.0, iy + 1.0) - at(ix, iy + 1.0)) * sx;
        a + (b - a) * sy
    };
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32, y as f32);
            let m = noise(fx, fy, 32.0, 11) * 0.6 + noise(fx, fy, 14.0, 23) * 0.4;
            let (u, v) = (
                (fx + 0.5) / n as f32 * 2.0 - 1.0,
                (fy + 0.5) / n as f32 * 2.0 - 1.0,
            );
            let edge = smooth(1.0, 0.35, (u * u + v * v).sqrt());
            let a = smooth(0.3, 0.85, m) * edge;
            let i = (y * n + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    TextureData3D {
        width: n as u32,
        height: n as u32,
        rgba,
    }
}

pub(super) fn build_air(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<Air> {
    let quad = renderer.upload_mesh(device, &Mesh3D::plane(1.0, Material3D::rgb(1.0, 1.0, 1.0)))?;
    let mote = renderer.upload_mesh(
        device,
        &Mesh3D::uv_sphere(1.0, 8, 5, Material3D::rgb(1.0, 1.0, 1.0)),
    )?;
    Ok(Air {
        quad,
        mote,
        pool: renderer.upload_texture(device, queue, &pool_texture())?,
        canopy: [
            renderer.upload_texture(device, queue, &canopy_texture(1))?,
            renderer.upload_texture(device, queue, &canopy_texture(2))?,
        ],
        mist: renderer.upload_texture(device, queue, &mist_texture())?,
    })
}

/// How much of each hour the light is now: (day, dusk, night), summing to 1.
pub(super) fn hours(env: &EnvironmentFrame3D) -> (f32, f32, f32) {
    let y = env.celestial.sun_direction.y.clamp(-0.999, 0.999);
    let sin = (y * 0.72 / (0.82 * (1.0 - y * y).sqrt())).clamp(-1.0, 1.0);
    let elevation = sin.asin().to_degrees();
    let night = 1.0 - smooth(-70.0, 4.0, elevation);
    let day = smooth(-4.0, 70.0, elevation);
    (day, (1.0 - night - day).max(0.0), night)
}

/// The mist's patches: (x, z, width, depth).
const MIST: [(f32, f32, f32, f32); 8] = [
    (-8.6, 13.2, 12.0, 9.0),
    (-13.5, -4.2, 13.0, 11.0),
    (-17.0, -1.0, 14.0, 12.0),
    (-19.0, -9.0, 12.0, 10.0),
    (-14.0, 6.0, 12.0, 10.0),
    (8.0, 12.5, 11.0, 7.0),
    (15.0, -9.0, 12.0, 9.0),
    (0.0, -9.0, 7.0, 9.0),
];

/// The air's draws for this frame, round `near`.
pub(super) fn air(
    air: &Air,
    elapsed: f32,
    env: &EnvironmentFrame3D,
    near: Vec3,
    out: &mut Vec<MeshDraw3D>,
) {
    let (day, dusk, night) = hours(env);
    let close = |x: f32, z: f32, r: f32| (x - near.x).hypot(z - near.z) < r;
    let flat = |x: f32, z: f32, y: f32, sx: f32, sz: f32, yaw: f32| {
        Mat4::from_translation(Vec3::new(x, y, z))
            * Mat4::from_rotation_y(yaw)
            * Mat4::from_scale(Vec3::new(sx, 1.0, sz))
    };
    // Pools of light under every lamp, fire and glowing thing.
    for (i, l) in plan().lamps.iter().enumerate() {
        if !close(l.x, l.z, 30.0) {
            continue;
        }
        let k = i as f32;
        let (size, strength) = match l.kind.as_str() {
            "fire" => (
                6.0 * l.power.sqrt(),
                (0.5 + 0.5 * night) * (1.0 + (elapsed * 9.0 + k).sin() * 0.06),
            ),
            "glow" => (3.4 * l.power.sqrt().max(0.6), 0.2 + 0.55 * night),
            _ => (5.5, 0.05 + 0.4 * night),
        };
        let c = linear(&l.color).map(|v| v * strength * 0.5);
        let y = ground_height(l.x, l.z) + 0.05;
        out.push(MeshDraw3D {
            mesh: air.quad,
            transform: flat(l.x, l.z, y, size, size, 0.0),
            material: Material3D::additive([c[0], c[1], c[2], 1.0]),
            texture: Some(air.pool),
        });
    }
    // The canopy over the trail and the glades, swaying a little in the wind: its leaves cast
    // the sun's (and the moon's) real shadow, the gaps let the light through in flecks.
    let mut patch = |x: f32, z: f32, k: u32| {
        if !close(x, z, 34.0) {
            return;
        }
        let size = 7.0 + hash(k * 3 + 2) * 4.0;
        let t = elapsed * (0.35 + hash(k * 3 + 5) * 0.2) + k as f32;
        let sway = Vec3::new(t.sin() * 0.12, 0.0, (t * 0.8).cos() * 0.1);
        let height = 8.5 + hash(k * 3 + 6) * 3.0;
        out.push(MeshDraw3D {
            mesh: air.quad,
            transform: Mat4::from_translation(Vec3::new(x, ground_height(x, z) + height, z) + sway)
                * Mat4::from_rotation_y(hash(k * 3 + 3) * 6.0 + t.sin() * 0.015)
                * Mat4::from_scale(Vec3::new(size, 1.0, size)),
            material: Material3D::shadow_only(),
            texture: Some(air.canopy[(k % 2) as usize]),
        });
    };
    let mut z = -12.0f32;
    let mut k = 0u32;
    while z < 20.0 {
        patch(nature_trail_center(z) + (hash(k * 3 + 1) - 0.5) * 3.0, z, k);
        z += 3.6 + hash(k * 3 + 4) * 1.8;
        k += 1;
    }
    for (i, (x, z)) in [
        (-4.4, 4.3),
        (-11.5, -5.2),
        (-8.6, 13.2),
        (8.0, 12.5),
        (-9.4, -10.2),
        (6.0, -2.0),
    ]
    .into_iter()
    .enumerate()
    {
        patch(x, z, 100 + i as u32);
    }
    // Mist lying in the low ground, drifting.
    let mist = [
        0.78 * day + 0.62 * dusk + 0.2 * night,
        0.85 * day + 0.58 * dusk + 0.27 * night,
        0.8 * day + 0.72 * dusk + 0.4 * night,
    ];
    let mist_alpha = 0.11 * day + 0.2 * dusk + 0.3 * night;
    for (i, (x, z, w, d)) in MIST.iter().copied().enumerate() {
        let x = if i >= 7 { nature_trail_center(z) } else { x };
        if !close(x, z, 34.0) {
            continue;
        }
        for layer in 0..2 {
            let t = elapsed * 0.05 + i as f32 * 1.7 + layer as f32 * 2.3;
            let drift = Vec3::new(t.sin() * 0.9, 0.0, (t * 0.7).cos() * 0.7);
            out.push(MeshDraw3D {
                mesh: air.quad,
                transform: flat(
                    x + drift.x,
                    z + drift.z,
                    ground_height(x, z) + 0.3 + layer as f32 * 0.4,
                    w,
                    d,
                    t * 0.1 + layer as f32,
                ),
                material: Material3D {
                    base_color: [mist[0], mist[1], mist[2], mist_alpha],
                    texture: None,
                    occlusion_window: [0.0; 4],
                },
                texture: Some(air.mist),
            });
        }
    }
    // Fireflies over the verges as the light goes, and wisps round the fungi and the shrine.
    let dark = (dusk * 0.6 + night).min(1.0);
    if dark > 0.05 {
        for k in 0..70u32 {
            let z = -12.0 + hash(k * 5 + 1) * 31.0;
            let x = nature_trail_center(z) + (hash(k * 5 + 2) - 0.5) * 20.0;
            if !close(x, z, 26.0) {
                continue;
            }
            let t = elapsed * (0.3 + hash(k * 5 + 3) * 0.4) + k as f32;
            let p = Vec3::new(
                x + t.sin() * 0.6,
                ground_height(x, z) + 0.6 + hash(k * 5 + 4) * 1.4 + (t * 1.3).sin() * 0.25,
                z + (t * 0.8).cos() * 0.6,
            );
            let blink = ((elapsed * 1.7 + k as f32 * 2.1).sin() * 0.5 + 0.5).powf(3.0);
            let g = 3.0 * dark * blink;
            out.push(MeshDraw3D {
                mesh: air.mote,
                transform: Mat4::from_translation(p)
                    * Mat4::from_scale(Vec3::new(0.035, 0.035, 0.035)),
                material: Material3D::additive([1.0 * g, 0.9 * g, 0.35 * g, 1.0]),
                texture: None,
            });
        }
        for (i, l) in plan().lamps.iter().filter(|l| l.kind == "glow").enumerate() {
            if !close(l.x, l.z, 26.0) {
                continue;
            }
            for j in 0..2 {
                let t = elapsed * (0.15 + 0.1 * j as f32) + i as f32 * 3.0 + j as f32 * 1.4;
                let p = Vec3::new(
                    l.x + t.sin() * 1.4,
                    ground_height(l.x, l.z) + 1.0 + (t * 1.3).sin() * 0.5,
                    l.z + (t * 0.8).cos() * 1.4,
                );
                let c = linear(&l.color).map(|v| v * 2.2 * (0.35 + 0.65 * night));
                out.push(MeshDraw3D {
                    mesh: air.mote,
                    transform: Mat4::from_translation(p)
                        * Mat4::from_scale(Vec3::new(0.06, 0.06, 0.06)),
                    material: Material3D::additive([c[0], c[1], c[2], 1.0]),
                    texture: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trail_is_free_from_the_castle_gate_to_the_villa_gate() {
        let mut z = -12.6;
        while z < 19.3 {
            let x = nature_trail_center(z);
            assert!(
                !blocked(Vec3::new(x, 0.0, z)),
                "the trail is blocked at z {z}"
            );
            z += 0.25;
        }
    }

    #[test]
    fn the_gates_the_mine_and_the_mage_can_be_reached() {
        for (name, at) in [
            ("castle", castle_entry_position()),
            ("villa", village_entry_position() - Vec3::new(0.0, 0.0, 1.0)),
            ("mine", mine::entry_position()),
            ("mage", mage_position() + Vec3::new(0.9, 0.0, 0.9)),
        ] {
            assert!(!blocked(at), "{name} cannot be reached: {at:?}");
        }
    }

    #[test]
    fn the_ground_follows_the_prototype() {
        // On the trail it is nearly level; out in the wood it rolls.
        let on_trail = ground_height(nature_trail_center(2.0), 2.0);
        assert!(on_trail.abs() < 0.15, "the trail lies at {on_trail}");
        let (lo, hi) = (-20..20)
            .flat_map(|i| (-12..18).map(move |j| ground_height(i as f32, j as f32)))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| {
                (a.min(v), b.max(v))
            });
        assert!(hi - lo > 0.3 && hi - lo < 3.0, "the floor spans {lo}..{hi}");
    }

    #[test]
    fn the_hours_change_gently() {
        // The game's cycle: 420 s a day. Second by second, no colour of the light jumps.
        let preset = EnvironmentPreset3D::Lived;
        let light = |i: usize| {
            let env = mood(preset.sample(i as f32 / 420.0));
            [
                env.lighting.key_color,
                env.lighting.ambient_color,
                env.atmosphere.fog_color,
            ]
        };
        for i in 0..420 {
            let (a, b) = (light(i), light(i + 1));
            for (x, y) in a.iter().zip(b.iter()) {
                let jump = (0..3).map(|c| (x[c] - y[c]).abs()).fold(0.0f32, f32::max);
                assert!(jump < 0.025, "the light jumps by {jump} at {i} s");
            }
        }
    }
}
