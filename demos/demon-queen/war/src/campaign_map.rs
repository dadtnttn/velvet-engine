//! The campaign's map: the west of the Dark Continent as a painted diorama on the war table.
//! Sculpted land (the northern cliffs of the Nest, the castle's hill, the cave's crags, the
//! warren's knolls, the marsh, the Dark Forest, the river with the Eastern Gate, the Alliance's
//! golden plains and the fallen capital on its hill), the table's carved frame, the water, the
//! woods, the roads; the provinces painted over it as a picture (owner tint, inked borders,
//! highlights, fog of war) and the cloud over what the Queen cannot see. As the prototype's
//! `campaign/map3d.js`.
use std::sync::OnceLock;

use velvet_math::{Mat4, Vec3};
use velvet_render::{Material3D, Mesh3D, MeshVertex3D};

use crate::data::{adjacent, rgb, Faction, MAP_D, MAP_W, PROVINCES, RIVER_X};
use crate::noise::{clamp, fbm, noise, resample, smooth, LinePoint, Rng};
use crate::prims::{self, Builder};
use crate::rules::Campaign;
use crate::scene::Instance;

pub const W: f32 = MAP_W;
pub const D: f32 = MAP_D;
pub const X0: f32 = -W / 2.0;
pub const Z0: f32 = -D / 2.0;
/// The water line of the diorama.
pub const WATER: f32 = -0.3;
/// Texels of the province picture per map unit.
pub const PX: usize = 5;

fn river() -> &'static [LinePoint] {
    static R: OnceLock<Vec<LinePoint>> = OnceLock::new();
    R.get_or_init(|| {
        resample(
            &[
                [34.0, -80.0],
                [29.0, -48.0],
                [31.0, -20.0],
                [27.0, 0.0],
                [30.0, 22.0],
                [26.0, 50.0],
                [31.0, 80.0],
            ],
            1.5,
            3,
        )
    })
}

/// How far from the river, and on which side (negative: east).
pub fn river_dist(x: f32, z: f32) -> (f32, f32) {
    let r = river();
    let mut best = f32::INFINITY;
    let mut bi = 0;
    for (i, p) in r.iter().enumerate() {
        let d = (x - p.x).powi(2) + (z - p.z).powi(2);
        if d < best {
            best = d;
            bi = i;
        }
    }
    let c = r[bi];
    (best.sqrt(), (x - c.x) * -c.tz + (z - c.z) * c.tx)
}

fn bump(x: f32, z: f32, cx: f32, cz: f32, r: f32, h: f32) -> f32 {
    let d = (x - cx).hypot(z - cz) / r;
    if d < 2.0 {
        h * (-2.6 * d * d).exp()
    } else {
        0.0
    }
}

// ------------------------------------------------------------------ the war map 2.0
/// The land as the war map 2.0 made it (`demos/demon-queen-warmap-threejs`, exported by its
/// `art/tools/export_war_game.py`): heights and colour on a fine grid, the region under every texel
/// of the province picture, the woods, the rivers and the roads. The game loads it from
/// `data/guerra/` and installs it before the campaign is drawn; without it the land is the
/// procedural one of before.
pub struct MapData {
    pub nx: usize,
    pub nz: usize,
    pub x0: f32,
    pub z0: f32,
    pub step: f32,
    pub h: Vec<f32>,
    /// Linear RGB per land vertex.
    pub color: Vec<[f32; 3]>,
    pub rw: usize,
    pub rh: usize,
    /// The region (index in `PROVINCES`) of every picture texel; 255 the water.
    pub regions: Vec<u8>,
    /// Smooth distances to the nearest region border (0..255 = 0..2 units) and province border
    /// (0..3 units), per picture texel: the borders are drawn from them, anti-aliased.
    pub edge_region: Vec<u8>,
    pub edge_group: Vec<u8>,
    /// x, z, kind (0 pine, 1 oak, 2 violet, 3 amber, 4 dead), scale.
    pub trees: Vec<[f32; 4]>,
    pub rivers: Vec<Vec<[f32; 2]>>,
    pub roads: Vec<Vec<[f32; 2]>>,
}

static MAP: OnceLock<MapData> = OnceLock::new();

/// Puts the war map 2.0 in place (once; later calls change nothing). Whether it took.
pub fn install(map: MapData) -> bool {
    MAP.set(map).is_ok()
}

/// The war map 2.0, if the game installed it.
pub fn map_data() -> Option<&'static MapData> {
    MAP.get()
}

#[derive(serde::Deserialize)]
struct MapJson {
    regions: Vec<String>,
    #[serde(default)]
    rivers: Vec<Vec<[f32; 2]>>,
    #[serde(default)]
    roads: Vec<RoadJson>,
}

#[derive(serde::Deserialize)]
struct RoadJson {
    pts: Vec<[f32; 2]>,
}

impl MapData {
    /// Reads `mapa.bin` and `mapa.json`; refuses a map whose regions are not `PROVINCES`.
    pub fn parse(bin: &[u8], json: &str) -> anyhow::Result<MapData> {
        let meta: MapJson = serde_json::from_str(json)?;
        let ids: Vec<&str> = PROVINCES.iter().map(|p| p.id).collect();
        anyhow::ensure!(
            meta.regions.iter().map(|s| s.as_str()).eq(ids.iter().copied()),
            "the map's regions are not the campaign's"
        );
        let mut at = 0usize;
        let mut take = |n: usize| -> anyhow::Result<&[u8]> {
            anyhow::ensure!(at + n <= bin.len(), "mapa.bin is cut short");
            let s = &bin[at..at + n];
            at += n;
            Ok(s)
        };
        anyhow::ensure!(take(4)? == b"DQW2", "not a war map");
        let u32_ = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let f32_ = |b: &[u8]| f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let head = take(32)?;
        let (nx, nz) = (u32_(&head[4..8]) as usize, u32_(&head[8..12]) as usize);
        let (x0, z0, step) = (f32_(&head[12..16]), f32_(&head[16..20]), f32_(&head[20..24]));
        let (lo, hi) = (f32_(&head[24..28]), f32_(&head[28..32]));
        let n = nx * nz;
        let h = take(n * 2)?
            .chunks_exact(2)
            .map(|c| lo + (u16::from_le_bytes([c[0], c[1]]) as f32 / 65535.0) * (hi - lo))
            .collect();
        let lin = |v: u8| {
            let s = v as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        let color = take(n * 3)?
            .chunks_exact(3)
            .map(|c| [lin(c[0]), lin(c[1]), lin(c[2])])
            .collect();
        let dims = take(8)?;
        let (rw, rh) = (u32_(&dims[0..4]) as usize, u32_(&dims[4..8]) as usize);
        let regions = take(rw * rh)?.to_vec();
        let version = u32_(&head[0..4]);
        let (edge_region, edge_group) = if version >= 2 {
            (take(rw * rh)?.to_vec(), take(rw * rh)?.to_vec())
        } else {
            (Vec::new(), Vec::new())
        };
        let nt = u32_(take(4)?) as usize;
        let trees = take(nt * 16)?
            .chunks_exact(16)
            .map(|c| [f32_(&c[0..4]), f32_(&c[4..8]), f32_(&c[8..12]), f32_(&c[12..16])])
            .collect();
        Ok(MapData {
            nx,
            nz,
            x0,
            z0,
            step,
            h,
            color,
            rw,
            rh,
            regions,
            edge_region,
            edge_group,
            trees,
            rivers: meta.rivers,
            roads: meta.roads.into_iter().map(|r| r.pts).collect(),
        })
    }

    fn cell(&self, x: f32, z: f32) -> (usize, usize, f32, f32) {
        let fx = clamp((x - self.x0) / self.step, 0.0, self.nx as f32 - 1.001);
        let fz = clamp((z - self.z0) / self.step, 0.0, self.nz as f32 - 1.001);
        let (i, j) = (fx as usize, fz as usize);
        (i, j, fx - i as f32, fz - j as f32)
    }

    /// The ground's height at a point (bilinear).
    pub fn height(&self, x: f32, z: f32) -> f32 {
        let (i, j, u, v) = self.cell(x, z);
        let k = j * self.nx + i;
        let (a, b, c, d) = (self.h[k], self.h[k + 1], self.h[k + self.nx], self.h[k + self.nx + 1]);
        a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
    }

    /// The region under a point of the map, or 255 (water, off the table).
    pub fn region(&self, x: f32, z: f32) -> u8 {
        let px = ((x - X0) * PX as f32).floor();
        let pz = ((z - Z0) * PX as f32).floor();
        if px < 0.0 || pz < 0.0 || px >= self.rw as f32 || pz >= self.rh as f32 {
            return 255;
        }
        self.regions[pz as usize * self.rw + px as usize]
    }

    /// The region under a point, or the nearest one over the water.
    pub fn region_near(&self, x: f32, z: f32) -> usize {
        let here = self.region(x, z);
        if here != 255 {
            return here as usize;
        }
        for r in 1..80 {
            let d = r as f32 / PX as f32 * 2.0;
            for k in 0..(8 * r) {
                let a = k as f32 / (8 * r) as f32 * std::f32::consts::TAU;
                let id = self.region(x + a.cos() * d, z + a.sin() * d);
                if id != 255 {
                    return id as usize;
                }
            }
        }
        0
    }
}

/// The height of the land (map units, ~metres of the diorama).
pub fn land_height(x: f32, z: f32) -> f32 {
    if let Some(m) = map_data() {
        return m.height(x, z);
    }
    let mut h = 1.0 + (fbm(x / 34.0, z / 34.0, 3, 3) - 0.5) * 2.4;
    let north = smooth(-46.0, -58.0, z) * smooth(20.0, -10.0, x);
    h += north * (9.0 + 4.0 * fbm(x / 12.0, z / 12.0, 9, 2));
    h += bump(x, z, -60.0, -58.0, 10.0, 6.0) + bump(x, z, -76.0, -64.0, 8.0, 5.0);
    h += smooth(-52.0, -70.0, z)
        * smooth(40.0, 70.0, x)
        * (6.0 + 5.0 * fbm(x / 10.0, z / 10.0, 4, 2));
    h += bump(x, z, -62.0, 2.0, 13.0, 5.5);
    h += bump(x, z, -92.0, -18.0, 14.0, 6.0)
        + bump(x, z, -100.0, -8.0, 10.0, 4.0)
        + 2.2
            * (fbm(x / 6.0, z / 6.0, 7, 2) - 0.5).max(0.0)
            * smooth(26.0, 10.0, (x + 92.0).hypot(z + 18.0));
    for (cx, cz) in [(-92.0, 30.0), (-80.0, 42.0), (-96.0, 44.0), (-78.0, 30.0)] {
        h += bump(x, z, cx, cz, 6.0, 2.6);
    }
    h += bump(x, z, -22.0, -34.0, 12.0, 2.2);
    h += bump(x, z, 86.0, -42.0, 15.0, 5.5)
        + bump(x, z, 48.0, -16.0, 9.0, 2.2)
        + bump(x, z, 92.0, 14.0, 10.0, 2.0)
        + bump(x, z, 70.0, 46.0, 18.0, 2.0);
    let m = smooth(22.0, 12.0, ((x + 30.0) * 0.9).hypot(z - 54.0));
    h = h * (1.0 - m) + (WATER - 0.35 + 0.5 * fbm(x / 5.0, z / 5.0, 11, 2)) * m;
    let (rd, _) = river_dist(x, z);
    h = h * smooth(2.5, 14.0, rd) + 0.4 * (1.0 - smooth(2.5, 14.0, rd));
    h -= 2.2 * (1.0 - smooth(1.2, 3.6, rd));
    let edge = (x - X0).min(-X0 - x).min(z - Z0).min(-Z0 - z);
    if edge < 10.0 {
        h = (h - 1.6 * smooth(10.0, 0.0, edge)).max(WATER + 0.25);
    }
    h
}

/// The province of a point (its index in `PROVINCES`): the nearest centre in warped space, never
/// across the river.
pub fn province_at(x: f32, z: f32) -> usize {
    if let Some(m) = map_data() {
        return m.region_near(x, z);
    }
    let wx = x + (noise(x / 9.0, z / 9.0, 21) - 0.5) * 12.0;
    let wz = z + (noise(x / 9.0, z / 9.0, 22) - 0.5) * 12.0;
    let east = river_dist(x, z).1 < 0.0;
    let mut best = 0;
    let mut bd = f32::INFINITY;
    for (i, p) in PROVINCES.iter().enumerate() {
        let mut d = (wx - p.x).powi(2) + (wz - p.z).powi(2);
        if (p.x > RIVER_X) != east {
            d += 1e5;
        }
        if d < bd {
            bd = d;
            best = i;
        }
    }
    best
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn land_color(x: f32, z: f32, h: f32) -> [f32; 3] {
    let (grass, dry, dark) = (rgb(0x7a9446), rgb(0xa8a052), rgb(0x3e5a2e));
    let (rock, gold, mud) = (rgb(0x8a7f96), rgb(0xcfae55), rgb(0x5c5a36));
    let mut c = mix(
        grass,
        dry,
        smooth(0.35, 0.7, fbm(x / 20.0, z / 20.0, 5, 2)) * 0.5,
    );
    c = mix(c, dark, smooth(26.0, 12.0, (x + 20.0).hypot(z - 2.0)) * 0.8);
    if x > RIVER_X + 4.0 {
        let fields = smooth(RIVER_X + 4.0, RIVER_X + 14.0, x) * (1.0 - smooth(-40.0, -56.0, z));
        let stripe = if ((x * 0.8 + z * 0.35) * 0.9).sin() > 0.3 {
            1.0
        } else {
            0.82
        };
        c = mix(
            c,
            gold,
            fields
                * 0.7
                * if noise(x / 7.0, z / 7.0, 33) > 0.45 {
                    1.0
                } else {
                    0.4
                },
        );
        let k = 1.0 - fields * 0.08 + fields * 0.08 * stripe;
        c = [c[0] * k, c[1] * k, c[2] * k];
    }
    c = mix(
        c,
        mud,
        smooth(16.0, 8.0, ((x + 30.0) * 0.9).hypot(z - 54.0)),
    );
    mix(c, rock, smooth(5.5, 9.0, h) * 0.85)
}

/// The land as a height grid (1 unit apart).
pub struct Land {
    pub nx: usize,
    pub nz: usize,
    pub h: Vec<f32>,
    /// Units between two vertices.
    pub step: f32,
}

impl Land {
    pub fn new() -> Land {
        if let Some(m) = map_data() {
            return Land {
                nx: m.nx,
                nz: m.nz,
                h: m.h.clone(),
                step: m.step,
            };
        }
        let nx = W as usize + 1;
        let nz = D as usize + 1;
        let mut h = vec![0.0; nx * nz];
        for j in 0..nz {
            for i in 0..nx {
                h[j * nx + i] = land_height(X0 + i as f32, Z0 + j as f32);
            }
        }
        Land { nx, nz, h, step: 1.0 }
    }

    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let fx = clamp((x - X0) / self.step, 0.0, self.nx as f32 - 1.001);
        let fz = clamp((z - Z0) / self.step, 0.0, self.nz as f32 - 1.001);
        let (i, j) = (fx as usize, fz as usize);
        let (u, v) = (fx - i as f32, fz - j as f32);
        let k = j * self.nx + i;
        let (a, b, c, d) = (
            self.h[k],
            self.h[k + 1],
            self.h[k + self.nx],
            self.h[k + self.nx + 1],
        );
        a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
    }

    /// The ground or the water, whichever is higher.
    pub fn top_at(&self, x: f32, z: f32) -> f32 {
        self.height_at(x, z).max(WATER)
    }

    /// The land's mesh (vertex-painted), and a copy just over it carrying the province picture's
    /// coordinates (`over`: lifted over the water, for the painted overlay).
    pub fn mesh(&self, over: bool) -> Mesh3D {
        let (nx, nz) = (self.nx, self.nz);
        let h = &self.h;
        let st = self.step;
        let painted = map_data();
        let mut vertices = Vec::with_capacity(nx * nz);
        for j in 0..nz {
            for i in 0..nx {
                let k = j * nx + i;
                let (x, z) = (X0 + i as f32 * st, Z0 + j as f32 * st);
                let hl = h[j * nx + i.saturating_sub(1)];
                let hr = h[j * nx + (i + 1).min(nx - 1)];
                let hd = h[j.saturating_sub(1) * nx + i];
                let hu = h[(j + 1).min(nz - 1) * nx + i];
                let n = Vec3::new(hl - hr, 2.0 * st, hd - hu).normalize_or_zero();
                let y = if over { h[k].max(WATER) + 0.05 } else { h[k] };
                let mut v =
                    MeshVertex3D::new([x, y, z], [n.x, n.y, n.z], [(x - X0) / W, (z - Z0) / D]);
                if !over {
                    let slope = (hr - hl).hypot(hu - hd) / (2.0 * st);
                    let mut c = match painted {
                        // the 2.0 land's own colour (its rock and snow already in it)
                        Some(m) => m.color[k],
                        None => land_color(x, z, h[k]),
                    };
                    if painted.is_none() {
                        c = mix(c, rgb(0x7a7088), smooth(0.6, 1.4, slope) * 0.8);
                    }
                    if h[k] < WATER + 0.2 {
                        c = mix(c, rgb(0x4e4636), 0.7);
                    }
                    let s = 0.95 + noise(x / 2.2, z / 2.2, 77) * 0.1;
                    v.color = [c[0] * s, c[1] * s, c[2] * s, 1.0];
                }
                vertices.push(v);
            }
        }
        let mut indices = Vec::with_capacity((nx - 1) * (nz - 1) * 6);
        for j in 0..nz - 1 {
            for i in 0..nx - 1 {
                let a = (j * nx + i) as u32;
                let (b, c) = (a + 1, a + nx as u32);
                let d = c + 1;
                indices.extend([a, c, b, b, c, d]);
            }
        }
        Mesh3D {
            vertices,
            indices,
            material: Material3D::default(),
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            texture_index: None,
            normal_texture_index: None,
            skinning: None,
        }
    }
}

impl Land {
    /// The land for its painted picture (`data/guerra/tierra.jpg`, the web map's colour): the same
    /// mesh, its vertex colours white (the sea bed darker) so the picture shows as painted.
    pub fn mesh_textured(&self) -> Mesh3D {
        let mut m = self.mesh(false);
        for v in &mut m.vertices {
            let wet = v.position[1] < WATER + 0.2;
            let k = if wet { 0.6 } else { 1.0 };
            v.color = [k, k, k, 1.0];
        }
        m
    }
}

/// The water as the web map draws it: one sheet at the water line whose colour follows the depth
/// (turquoise shallows, deep blue sea), a little broken foam at the very shore, clear where it is
/// shallowest; the open sea deepening towards the map's edge and out to the horizon. UVs every 4
/// units for the waves' normal map ([`wave_normals`]).
pub fn water_web(land: &Land) -> Mesh3D {
    let st = land.step;
    let (nx, nz) = ((W / st) as usize + 1, (D / st) as usize + 1);
    // the web's colours, a little darker: its water shows mostly a dark reflection, ours is lit
    let lin = |c: [f32; 3]| [c[0] * 0.72, c[1] * 0.72, c[2] * 0.72];
    let shallow = lin([0.16, 0.52, 0.52]);
    let mid = lin([0.05, 0.27, 0.36]);
    let deep = lin([0.02, 0.09, 0.17]);
    let color = |depth: f32, x: f32, z: f32| -> [f32; 4] {
        let mut c = mix(shallow, mid, smooth(0.0, 1.2, depth));
        c = mix(c, deep, smooth(1.2, 5.5, depth));
        let n = noise(x / 3.0, z / 3.0, 31);
        let foam = (1.0 - smooth(0.0, 0.22, depth)) * smooth(0.35, 0.7, n + 0.25);
        c = mix(c, [0.92, 0.95, 0.95], foam * 0.85);
        let a = smooth(0.0, 0.35, depth) * (0.7 + 0.3 * smooth(0.35, 3.0, depth));
        [c[0], c[1], c[2], a]
    };
    let ground = |x: f32, z: f32| -> f32 {
        // the open sea deepens towards the map's edge, so its rectangle never shows
        let e = (x - X0).min(-X0 - x).min(z - Z0).min(-Z0 - z);
        let g = land.height_at(x, z);
        let k = smooth(0.0, 14.0, e);
        -8.0 + (g + 8.0) * k
    };
    let mut id = vec![u32::MAX; nx * nz];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let uv = |x: f32, z: f32| [x / 4.0, z / 4.0];
    let mut vert = |i: usize, j: usize, vertices: &mut Vec<MeshVertex3D>| {
        let k = j * nx + i;
        if id[k] == u32::MAX {
            let (x, z) = (X0 + i as f32 * st, Z0 + j as f32 * st);
            let depth = (WATER - ground(x, z)).max(0.0);
            let mut v = MeshVertex3D::new([x, WATER, z], [0.0, 1.0, 0.0], uv(x, z));
            v.color = color(depth, x, z);
            id[k] = vertices.len() as u32;
            vertices.push(v);
        }
        id[k]
    };
    for j in 0..nz - 1 {
        for i in 0..nx - 1 {
            let (x, z) = (X0 + i as f32 * st, Z0 + j as f32 * st);
            let wet = [(0.0, 0.0), (st, 0.0), (0.0, st), (st, st)]
                .iter()
                .any(|(a, b)| ground(x + a, z + b) < WATER + 0.05);
            if !wet {
                continue;
            }
            let a = vert(i, j, &mut vertices);
            let b = vert(i + 1, j, &mut vertices);
            let c = vert(i, j + 1, &mut vertices);
            let d = vert(i + 1, j + 1, &mut vertices);
            indices.extend([a, c, b, b, c, d]);
        }
    }
    // out to the horizon: a ring of deep sea round the map
    let deep_c = color(9.0, 0.0, 0.0);
    let r = 1600.0f32;
    let ring = [
        (X0, Z0, -X0, Z0, -r, -r, r, -r),
        (-X0, Z0, -X0, -Z0, r, -r, r, r),
        (-X0, -Z0, X0, -Z0, r, r, -r, r),
        (X0, -Z0, X0, Z0, -r, r, -r, -r),
    ];
    for (ax, az, bx, bz, cx, cz, dx, dz) in ring {
        let base = vertices.len() as u32;
        for (x, z) in [(ax, az), (bx, bz), (cx, cz), (dx, dz)] {
            let mut v = MeshVertex3D::new([x, WATER, z], [0.0, 1.0, 0.0], uv(x, z));
            v.color = deep_c;
            vertices.push(v);
        }
        // a, b the inner edge; c under a and d under b on the outer edge
        indices.extend([base, base + 1, base + 3, base, base + 3, base + 2]);
    }
    // whichever winding, face up
    for t in indices.chunks_exact_mut(3) {
        let p = |k: u32| Vec3::from_array(vertices[k as usize].position);
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        if (b - a).cross(c - a).y < 0.0 {
            t.swap(1, 2);
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 0.08,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

/// The waves' normal map (the web map's: a sum of 22 sines, tiling), RGBA8 tangent space.
pub fn wave_normals(size: usize, seed: u32) -> Vec<u8> {
    let mut s = seed as u64;
    let mut rnd = || {
        s = (s * 16807) % 2147483647;
        s as f32 / 2147483647.0
    };
    let waves: Vec<(f32, f32, f32, f32)> = (0..22)
        .map(|_| {
            let kx = (rnd() * 12.0 - 6.0).round();
            let ky = (rnd() * 12.0 - 6.0).round();
            (kx, ky, rnd() * 6.28, 0.3 + rnd())
        })
        .collect();
    let mut h = vec![0.0f32; size * size];
    for y in 0..size {
        for x in 0..size {
            h[y * size + x] = waves
                .iter()
                .map(|(kx, ky, p, a)| a * (((kx * x as f32 + ky * y as f32) / size as f32) * 6.283 + p).sin())
                .sum();
        }
    }
    let mut out = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let dx = h[y * size + (x + 1) % size] - h[y * size + (x + size - 1) % size];
            let dy = h[((y + 1) % size) * size + x] - h[((y + size - 1) % size) * size + x];
            let n = Vec3::new(-dx * 0.25, -dy * 0.25, 1.0).normalize_or_zero();
            let o = (y * size + x) * 4;
            out[o] = ((n.x * 0.5 + 0.5) * 255.0) as u8;
            out[o + 1] = ((n.y * 0.5 + 0.5) * 255.0) as u8;
            out[o + 2] = ((n.z * 0.5 + 0.5) * 255.0) as u8;
            out[o + 3] = 255;
        }
    }
    out
}

/// The sea round the continent, out to the horizon (the web map's): one deep sheet a little under
/// the water line (the coast's own water sheet lies over it).
pub fn sea() -> Mesh3D {
    // the coast sheet's deep colour, so the map's edge does not show where the two meet
    let c = rgb(0x23406a);
    let r = 1800.0;
    let mut vertices = Vec::new();
    for (x, z) in [(-r, -r), (r, -r), (-r, r), (r, r)] {
        let mut v = MeshVertex3D::new([x, WATER - 0.04, z], [0.0, 1.0, 0.0], [0.0, 0.0]);
        v.color = [c[0], c[1], c[2], 1.0];
        vertices.push(v);
    }
    Mesh3D {
        vertices,
        indices: vec![0, 2, 1, 1, 2, 3],
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 0.25,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

impl Default for Land {
    fn default() -> Self {
        Self::new()
    }
}

/// The table's sides (layered earth going down) and its frame: dark carved wood, a gold inlay.
pub fn table(land: &Land) -> Mesh3D {
    let mut b = Builder::default();
    let corners = [
        (X0, Z0, -X0, Z0),
        (-X0, Z0, -X0, -Z0),
        (-X0, -Z0, X0, -Z0),
        (X0, -Z0, X0, Z0),
    ];
    for (ax, az, bx, bz) in corners {
        let l = (bx - ax).hypot(bz - az);
        let n = (l / 2.0).ceil() as usize;
        for k in 0..n {
            let t0 = k as f32 / n as f32;
            let t1 = (k + 1) as f32 / n as f32;
            let p0 = (ax + (bx - ax) * t0, az + (bz - az) * t0);
            let p1 = (ax + (bx - ax) * t1, az + (bz - az) * t1);
            let top0 = land.height_at(p0.0, p0.1) - 0.05;
            let top1 = land.height_at(p1.0, p1.1) - 0.05;
            for (y0, y1, hex) in [(-3.0f32, 30.0f32, 0x5a4636u32), (-9.0, -3.0, 0x3a2c30)] {
                let (u0, u1) = (top0.min(y1), top1.min(y1));
                let s = prims::Shape {
                    tris: vec![
                        [p0.0, y0, p0.1],
                        [p1.0, y0, p1.1],
                        [p1.0, u1, p1.1],
                        [p0.0, y0, p0.1],
                        [p1.0, u1, p1.1],
                        [p0.0, u0, p0.1],
                        [p1.0, y0, p1.1],
                        [p0.0, y0, p0.1],
                        [p0.0, u0, p0.1],
                        [p1.0, y0, p1.1],
                        [p0.0, u0, p0.1],
                        [p1.0, u1, p1.1],
                    ],
                };
                b.add(&s, rgb(hex));
            }
        }
    }
    let (t, y) = (5.0f32, 0.6f32);
    let wood = rgb(0x2e1c26);
    let gold = rgb(0xd9a94a);
    let mut add = |w: f32, h: f32, d: f32, x: f32, yy: f32, z: f32, c: [f32; 3]| {
        b.add(&prims::boxed(w, h, d).translate(x, yy, z), c)
    };
    add(W + 2.0 * t, 3.4, t, 0.0, y, Z0 - t / 2.0, wood);
    add(W + 2.0 * t, 3.4, t, 0.0, y, -Z0 + t / 2.0, wood);
    add(t, 3.4, D, X0 - t / 2.0, y, 0.0, wood);
    add(t, 3.4, D, -X0 + t / 2.0, y, 0.0, wood);
    add(
        W + 2.0 * t + 0.4,
        0.5,
        0.9,
        0.0,
        y + 1.9,
        Z0 - t + 0.6,
        gold,
    );
    add(
        W + 2.0 * t + 0.4,
        0.5,
        0.9,
        0.0,
        y + 1.9,
        -Z0 + t - 0.6,
        gold,
    );
    add(0.9, 0.5, D + 2.0 * t, X0 - t + 0.6, y + 1.9, 0.0, gold);
    add(0.9, 0.5, D + 2.0 * t, -X0 + t - 0.6, y + 1.9, 0.0, gold);
    for (cx, cz) in [
        (X0 - t / 2.0, Z0 - t / 2.0),
        (-X0 + t / 2.0, Z0 - t / 2.0),
        (X0 - t / 2.0, -Z0 + t / 2.0),
        (-X0 + t / 2.0, -Z0 + t / 2.0),
    ] {
        add(t + 1.6, 4.6, t + 1.6, cx, y + 0.4, cz, rgb(0x3a2430));
        add(2.2, 1.2, 2.2, cx, y + 3.2, cz, gold);
    }
    b.build().expect("the table")
}

/// The water sheet (the river, the marsh), shallow to deep, clear at its edge.
pub fn water(land: &Land) -> Mesh3D {
    let shallow = rgb(0x4f9a96);
    let deep = rgb(0x23406a);
    let foam = rgb(0xf6ecd8);
    let st = land.step;
    let (nx, nz) = ((W / st) as usize + 1, (D / st) as usize + 1);
    let mut id = vec![u32::MAX; nx * nz];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vert = |i: usize, j: usize, vertices: &mut Vec<MeshVertex3D>| {
        let k = j * nx + i;
        if id[k] == u32::MAX {
            let (x, z) = (X0 + i as f32 * st, Z0 + j as f32 * st);
            let depth = (WATER - land.height_at(x, z)) * 1.8;
            let d = (depth / 1.6).clamp(0.0, 1.0);
            let mut c = mix(
                shallow,
                deep,
                smooth(0.25, 0.45, d) * 0.7 + smooth(0.6, 0.85, d) * 0.3,
            );
            c = mix(c, rgb(0xe9a07e), 0.12 * (1.0 - d));
            let edge = 1.0 - smooth(0.0, 0.09, depth);
            c = mix(c, foam, edge * 0.8);
            let a = (0.62 + 0.31 * smooth(0.1, 0.7, d)) * smooth(-0.02, 0.02, depth);
            let mut v = MeshVertex3D::new([x, WATER, z], [0.0, 1.0, 0.0], [0.0, 0.0]);
            v.color = [c[0], c[1], c[2], a];
            id[k] = vertices.len() as u32;
            vertices.push(v);
        }
        id[k]
    };
    for j in 0..nz - 1 {
        for i in 0..nx - 1 {
            let (x, z) = (X0 + i as f32 * st, Z0 + j as f32 * st);
            let wet = [(0.0, 0.0), (st, 0.0), (0.0, st), (st, st)]
                .iter()
                .any(|(a, b)| land.height_at(x + a, z + b) < WATER + 0.4);
            if !wet {
                continue;
            }
            let a = vert(i, j, &mut vertices);
            let b = vert(i + 1, j, &mut vertices);
            let c = vert(i, j + 1, &mut vertices);
            let d = vert(i + 1, j + 1, &mut vertices);
            indices.extend([a, c, b, b, c, d]);
        }
    }
    Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 0.2,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}

/// The woods: the Dark Forest, the north's pines, groves in the west, a few in the east. Instances
/// of the battlefield's tree models (`kind` 0 pine, 1 oak, 2 violet, 3 amber, 4 dead).
pub fn woods(land: &Land) -> Vec<Instance> {
    if let Some(m) = map_data() {
        return m
            .trees
            .iter()
            .enumerate()
            // the game's trees are fuller than the prototype's: two in five of them make the woods
            .filter(|(i, _)| i % 5 < 2)
            .map(|(_, t)| t)
            .filter(|t| {
                !PROVINCES.iter().any(|p| {
                    let (sx, sz) = p.site();
                    (t[0] - sx).hypot(t[1] - sz) < 4.5
                })
            })
            .map(|t| {
                let (x, z) = (t[0], t[1]);
                let yaw = noise(x * 3.1, z * 2.7, 9) * std::f32::consts::TAU * 3.0;
                let s = (t[3] * 0.62).clamp(0.14, 0.42);
                Instance {
                    kind: (t[2] as usize).min(4),
                    transform: Mat4::from_translation(Vec3::new(x, land.height_at(x, z) - 0.1, z))
                        * Mat4::from_rotation_y(yaw)
                        * Mat4::from_scale(Vec3::new(s, s, s)),
                    tint: [1.0; 3],
                }
            })
            .collect();
    }
    let mut r = Rng::new(808);
    let mut out = Vec::new();
    for _ in 0..26000 {
        let x = X0 + 4.0 + r.next() * (W - 8.0);
        let z = Z0 + 4.0 + r.next() * (D - 8.0);
        let h = land.height_at(x, z);
        if h < WATER + 0.4 || river_dist(x, z).0 < 4.0 {
            continue;
        }
        let forest = smooth(25.0, 10.0, (x + 20.0).hypot(z - 2.0));
        let west = smooth(RIVER_X - 6.0, RIVER_X - 30.0, x)
            * 0.025
            * if noise(x / 14.0, z / 14.0, 61) > 0.55 {
                6.0
            } else {
                1.0
            };
        let north = smooth(-44.0, -52.0, z) * 0.28;
        let east = if x > RIVER_X + 6.0 && z > -50.0 {
            0.006
        } else {
            0.0
        };
        let dens = (forest * 0.95).max(west).max(north).max(east);
        if r.next() > dens {
            continue;
        }
        if PROVINCES.iter().any(|p| {
            let (sx, sz) = p.site();
            (x - sx).hypot(z - sz) < 12.0
        }) {
            continue;
        }
        let mut kind = if forest > 0.4 {
            if r.next() < 0.55 {
                0
            } else if r.next() < 0.6 {
                2
            } else {
                1
            }
        } else if north > 0.2 {
            0
        } else if r.next() < 0.5 {
            1
        } else if r.next() < 0.4 {
            3
        } else {
            0
        };
        if r.next() < 0.02 {
            kind = 4;
        }
        let s = 0.32 + r.next() * 0.18;
        let yaw = r.next() * std::f32::consts::TAU;
        out.push(Instance {
            kind,
            transform: Mat4::from_translation(Vec3::new(x, h - 0.1, z))
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(s, s, s)),
            tint: [1.0; 3],
        });
    }
    out
}

/// The roads between neighbouring provinces: a gentle bend each, a dirt ribbon on the land.
pub fn roads(land: &Land) -> Option<Mesh3D> {
    let mut b = Builder::default();
    let road = rgb(0xa98a5c);
    if let Some(m) = map_data() {
        for pts in &m.roads {
            let line = resample(pts, 1.0, 1);
            road_ribbon(&mut b, land, &line, road, |p| {
                PROVINCES.iter().any(|q| {
                    let (sx, sz) = q.site();
                    (p.x - sx).hypot(p.z - sz) < 3.0
                })
            });
        }
        return b.build();
    }
    let mut done = std::collections::BTreeSet::new();
    for pa in PROVINCES.iter() {
        for &bid in adjacent(pa.id) {
            let key = if pa.id < bid {
                (pa.id, bid)
            } else {
                (bid, pa.id)
            };
            if !done.insert(key) {
                continue;
            }
            let pb = crate::data::province(bid);
            let mx = (pa.x + pb.x) / 2.0 + (noise(pa.x, pb.z, 3) - 0.5) * 10.0;
            let mz = (pa.z + pb.z) / 2.0 + (noise(pb.x, pa.z, 4) - 0.5) * 10.0;
            let line = resample(&[[pa.x, pa.z], [mx, mz], [pb.x, pb.z]], 1.2, 2);
            for k in 0..line.len().saturating_sub(1) {
                let (p, q) = (line[k], line[k + 1]);
                if (p.x - pa.x).hypot(p.z - pa.z) < 7.0 || (p.x - pb.x).hypot(p.z - pb.z) < 7.0 {
                    continue;
                }
                let (ha, hb) = (land.height_at(p.x, p.z), land.height_at(q.x, q.z));
                if ha < WATER + 0.1 || hb < WATER + 0.1 {
                    continue;
                }
                let (nx, nz) = (-p.tz * 0.7, p.tx * 0.7);
                let s = prims::Shape {
                    tris: vec![
                        [p.x - nx, ha + 0.06, p.z - nz],
                        [p.x + nx, ha + 0.06, p.z + nz],
                        [q.x + nx, hb + 0.06, q.z + nz],
                        [p.x - nx, ha + 0.06, p.z - nz],
                        [q.x + nx, hb + 0.06, q.z + nz],
                        [q.x - nx, hb + 0.06, q.z - nz],
                    ],
                };
                // the ribbon faces up whichever way it was wound
                let mut s = s;
                for t in s.tris.chunks_exact_mut(3) {
                    let (a, bq, c) = (
                        Vec3::from_array(t[0]),
                        Vec3::from_array(t[1]),
                        Vec3::from_array(t[2]),
                    );
                    if (bq - a).cross(c - a).y < 0.0 {
                        t.swap(1, 2);
                    }
                }
                b.add(&s, road);
            }
        }
    }
    b.build()
}

/// A dirt ribbon laid along a line on the land (skipping the bits `skip` says, and the water).
fn road_ribbon(b: &mut Builder, land: &Land, line: &[LinePoint], road: [f32; 3], skip: impl Fn(&LinePoint) -> bool) {
    for k in 0..line.len().saturating_sub(1) {
        let (p, q) = (line[k], line[k + 1]);
        if skip(&p) || skip(&q) {
            continue;
        }
        let (ha, hb) = (land.height_at(p.x, p.z), land.height_at(q.x, q.z));
        if ha < WATER + 0.1 || hb < WATER + 0.1 {
            continue;
        }
        let (nx, nz) = (-p.tz * 0.55, p.tx * 0.55);
        let mut s = prims::Shape {
            tris: vec![
                [p.x - nx, ha + 0.08, p.z - nz],
                [p.x + nx, ha + 0.08, p.z + nz],
                [q.x + nx, hb + 0.08, q.z + nz],
                [p.x - nx, ha + 0.08, p.z - nz],
                [q.x + nx, hb + 0.08, q.z + nz],
                [q.x - nx, hb + 0.08, q.z - nz],
            ],
        };
        for t in s.tris.chunks_exact_mut(3) {
            let (a, bq, c) = (
                Vec3::from_array(t[0]),
                Vec3::from_array(t[1]),
                Vec3::from_array(t[2]),
            );
            if (bq - a).cross(c - a).y < 0.0 {
                t.swap(1, 2);
            }
        }
        b.add(&s, road);
    }
}

trait FromArray {
    fn from_array(a: [f32; 3]) -> Self;
}

impl FromArray for Vec3 {
    fn from_array(a: [f32; 3]) -> Self {
        Vec3::new(a[0], a[1], a[2])
    }
}

/// The provinces painted on the land: which province each texel is in, and how far it is from a
/// border (the shapes never change: worked out once).
pub struct Provinces {
    pub cw: usize,
    pub ch: usize,
    pub ids: Vec<u8>,
    pub edge: Vec<u8>,
    /// The war map 2.0's smooth border distances (units), when it is in place.
    pub smooth: Option<(Vec<f32>, Vec<f32>)>,
    /// The painted picture's size (twice the regions' raster, as the web map draws its borders
    /// thin and smooth), and per texel: its region, its distance to a region's border and to a
    /// province's (hundredths of a unit), and the region across the nearest border (255: none).
    pub pw: usize,
    pub ph: usize,
    pid: Vec<u8>,
    pd: Vec<u8>,
    pdp: Vec<u8>,
    pother: Vec<u8>,
}

/// How much finer the painted picture is than the regions' raster.
const PAINT_SCALE: usize = 2;

/// What the picture shows besides the owners.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Paint {
    /// Hidden by the fog (by province index).
    pub fog: Vec<bool>,
    /// Where the chosen army can go.
    pub reach: Vec<bool>,
    pub selected: Option<usize>,
    pub hover: Option<usize>,
}

impl Provinces {
    pub fn new() -> Provinces {
        let (cw, ch) = (W as usize * PX, D as usize * PX);
        let mut ids = vec![0u8; cw * ch];
        match map_data().filter(|m| m.rw == cw && m.rh == ch) {
            Some(m) => ids.copy_from_slice(&m.regions),
            None => {
                for py in 0..ch {
                    for px in 0..cw {
                        let x = X0 + (px as f32 + 0.5) / PX as f32;
                        let z = Z0 + (py as f32 + 0.5) / PX as f32;
                        ids[py * cw + px] = province_at(x, z) as u8;
                    }
                }
            }
        }
        let mut edge = vec![9u8; cw * ch];
        for y in 0..ch {
            for x in 0..cw {
                let k = y * cw + x;
                let id = ids[k];
                // a border is between two lands; the water's edge is the shore, not a border
                let other = |q: u8| q != id && q != 255;
                for r in 1..=7usize {
                    if (x >= r && other(ids[k - r]))
                        || (x + r < cw && other(ids[k + r]))
                        || (y >= r && other(ids[k - r * cw]))
                        || (y + r < ch && other(ids[k + r * cw]))
                    {
                        edge[k] = r as u8;
                        break;
                    }
                }
            }
        }
        let smooth = map_data()
            .filter(|m| m.rw == cw && m.rh == ch && m.edge_region.len() == cw * ch)
            .map(|m| {
                (
                    m.edge_region.iter().map(|&v| v as f32 / 255.0 * 2.0).collect(),
                    m.edge_group.iter().map(|&v| v as f32 / 255.0 * 3.0).collect(),
                )
            });
        // the painted picture, twice as fine: the region by the nearest raster texel, the
        // distances blended between the four, the region across the border found nearby
        let (pw, ph) = (cw * PAINT_SCALE, ch * PAINT_SCALE);
        let mut pid = vec![255u8; pw * ph];
        let mut pd = vec![255u8; pw * ph];
        let mut pdp = vec![255u8; pw * ph];
        let mut pother = vec![255u8; pw * ph];
        let sample = |v: &Vec<f32>, fx: f32, fy: f32| -> f32 {
            let x0 = (fx.floor().max(0.0) as usize).min(cw - 1);
            let y0 = (fy.floor().max(0.0) as usize).min(ch - 1);
            let (x1, y1) = ((x0 + 1).min(cw - 1), (y0 + 1).min(ch - 1));
            let (tx, ty) = ((fx - x0 as f32).clamp(0.0, 1.0), (fy - y0 as f32).clamp(0.0, 1.0));
            let a = v[y0 * cw + x0] * (1.0 - tx) + v[y0 * cw + x1] * tx;
            let b = v[y1 * cw + x0] * (1.0 - tx) + v[y1 * cw + x1] * tx;
            a * (1.0 - ty) + b * ty
        };
        for y in 0..ph {
            for x in 0..pw {
                let fx = (x as f32 + 0.5) / PAINT_SCALE as f32 - 0.5;
                let fy = (y as f32 + 0.5) / PAINT_SCALE as f32 - 0.5;
                let (bx, by) = (
                    ((fx + 0.5) as usize).min(cw - 1),
                    ((fy + 0.5) as usize).min(ch - 1),
                );
                let k = y * pw + x;
                let id = ids[by * cw + bx];
                pid[k] = id;
                if let Some((dr, dg)) = &smooth {
                    pd[k] = (sample(dr, fx, fy) * 100.0).clamp(0.0, 255.0) as u8;
                    pdp[k] = (sample(dg, fx, fy) * 100.0).clamp(0.0, 255.0) as u8;
                } else {
                    let e = edge[by * cw + bx] as f32 / PX as f32;
                    pd[k] = (e * 100.0).clamp(0.0, 255.0) as u8;
                    pdp[k] = 255;
                }
                // the region across the border (only near one)
                if id != 255 && pd[k] < 120 {
                    'look: for r in 1..=8i32 {
                        for (dx, dy) in [(r, 0), (-r, 0), (0, r), (0, -r), (r, r), (-r, r), (r, -r), (-r, -r)] {
                            let (qx, qy) = (bx as i32 + dx, by as i32 + dy);
                            if qx < 0 || qy < 0 || qx >= cw as i32 || qy >= ch as i32 {
                                continue;
                            }
                            let q = ids[qy as usize * cw + qx as usize];
                            if q != id && q != 255 {
                                pother[k] = q;
                                break 'look;
                            }
                        }
                    }
                }
            }
        }
        Provinces {
            cw,
            ch,
            ids,
            edge,
            smooth,
            pw,
            ph,
            pid,
            pd,
            pdp,
            pother,
        }
    }

    /// The province under a point of the map (its index), if it is on the table.
    pub fn at(&self, x: f32, z: f32) -> Option<usize> {
        if x.abs() > W / 2.0 || z.abs() > D / 2.0 {
            return None;
        }
        let px = (((x - X0) * PX as f32) as usize).min(self.cw - 1);
        let pz = (((z - Z0) * PX as f32) as usize).min(self.ch - 1);
        let id = self.ids[pz * self.cw + px];
        (id != 255).then_some(id as usize)
    }

    /// The picture as the web map draws it (`pw` × `ph`, RGBA, sRGB): a faint owner's tint, thin
    /// anti-aliased borders, strong only where two owners meet (their bright colour and a dark
    /// core), faint between regions of one owner, a heavier line between the ten provinces; the
    /// reach, the chosen and the hovered region; the fog.
    pub fn paint(&self, state: &Campaign, p: &Paint) -> Vec<u8> {
        let to8 = |hex: u32| {
            let c = rgb(hex);
            [c[0], c[1], c[2]].map(|v| (v.powf(1.0 / 2.2) * 255.0).round())
        };
        let owners: Vec<Faction> = PROVINCES.iter().map(|pr| state.prov(pr.id).owner).collect();
        let n = PROVINCES.len();
        let mut out = vec![0u8; self.pw * self.ph * 4];
        // rows in bands, one per core (the hover repaints it: it must be quick)
        let threads = std::thread::available_parallelism().map_or(4, |t| t.get()).clamp(1, 16);
        let rows = self.ph.div_ceil(threads);
        let pw = self.pw;
        std::thread::scope(|scope| {
            for (band, chunk) in out.chunks_mut(rows * pw * 4).enumerate() {
                let owners = &owners;
                scope.spawn(move || self.paint_rows(state, p, owners, band * rows, chunk));
            }
        });
        out
    }

    fn paint_rows(&self, state: &Campaign, p: &Paint, owners: &[Faction], y0: usize, out: &mut [u8]) {
        let to8 = |hex: u32| {
            let c = rgb(hex);
            [c[0], c[1], c[2]].map(|v| (v.powf(1.0 / 2.2) * 255.0).round())
        };
        let n = PROVINCES.len();
        let cols: Vec<([f32; 3], [f32; 3])> = owners
            .iter()
            .map(|o| (to8(o.info().color), to8(o.info().bright)))
            .collect();
        let rows = out.len() / (self.pw * 4);
        for yy in 0..rows {
            let y = y0 + yy;
            for x in 0..self.pw {
                let k = y * self.pw + x;
                let id = self.pid[k] as usize;
                if id >= n {
                    continue; // the water: left clear
                }
                let o = (yy * self.pw + x) * 4;
                let (c, bright) = cols[id];
                let fog = p.fog.get(id).copied().unwrap_or(false);
                let hi = p.reach.get(id).copied().unwrap_or(false);
                let sel = p.selected == Some(id);
                let hover = p.hover == Some(id);
                let lit = state.provinces[id].lit;
                // the inside of a region, plain: its faint tint
                if self.pd[k] > 110 && self.pdp[k] > 12 && !fog && !hi && !sel && !hover && lit {
                    out[o] = c[0] as u8;
                    out[o + 1] = c[1] as u8;
                    out[o + 2] = c[2] as u8;
                    out[o + 3] = 10;
                    continue;
                }
                let d = self.pd[k] as f32 / 100.0;
                let dp = self.pdp[k] as f32 / 100.0;
                let other = self.pother[k] as usize;
                let frontier = other < n && owners[other] != owners[id];
                let [mut r, mut g, mut b] = c;
                let mut a = 0.04f32;
                let mut lay = |col: [f32; 3], alpha: f32, t: f32, r: &mut f32, g: &mut f32, b: &mut f32, a: &mut f32| {
                    let t = t.clamp(0.0, 1.0);
                    *r += (col[0] - *r) * t;
                    *g += (col[1] - *g) * t;
                    *b += (col[2] - *b) * t;
                    *a += (alpha - *a) * t;
                };
                if hi {
                    [r, g, b] = [242.0, 181.0, 74.0];
                    a = 0.16;
                }
                if hover {
                    a = a.max(0.12);
                }
                if !lit {
                    r = r * 0.5 + 60.0;
                    g = g * 0.5 + 56.0;
                    b = b * 0.5 + 66.0;
                    a = a.max(0.2);
                }
                let _ = (bright, dp);
                // the owner's colour washed in along a frontier (the lines are the ribbons')
                if frontier {
                    lay(c, 0.26, 1.0 - smooth(0.0, 0.85, d), &mut r, &mut g, &mut b, &mut a);
                } else {
                    lay(c, 0.1, 1.0 - smooth(0.0, 0.85, d), &mut r, &mut g, &mut b, &mut a);
                }
                if hi || sel || hover {
                    let gold = if sel || hover { [255.0, 222.0, 150.0] } else { [242.0, 181.0, 74.0] };
                    lay(gold, 0.3, 1.0 - smooth(0.0, 1.0, d), &mut r, &mut g, &mut b, &mut a);
                }
                if fog {
                    let nz = noise(x as f32 / 80.0, y as f32 / 80.0, 5) * 0.25;
                    r = r * 0.15 + 34.0;
                    g = g * 0.15 + 26.0;
                    b = b * 0.15 + 52.0;
                    a = a.max(0.72 + nz);
                }
                out[o] = r.clamp(0.0, 255.0) as u8;
                out[o + 1] = g.clamp(0.0, 255.0) as u8;
                out[o + 2] = b.clamp(0.0, 255.0) as u8;
                out[o + 3] = (a * 255.0).clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// The old picture at the raster's size (the procedural map before 2.0).
    pub fn paint_old(&self, state: &Campaign, p: &Paint) -> Vec<u8> {
        let to8 = |hex: u32| {
            let c = rgb(hex);
            [c[0], c[1], c[2]].map(|v| (v.powf(1.0 / 2.2) * 255.0).round())
        };
        struct Info {
            c: [f32; 3],
            b: [f32; 3],
            fog: bool,
            hi: bool,
            sel: bool,
            hover: bool,
            lit: bool,
        }
        let info: Vec<Info> = PROVINCES
            .iter()
            .enumerate()
            .map(|(i, pr)| {
                let st = state.prov(pr.id);
                let f = st.owner.info();
                Info {
                    c: to8(f.color),
                    b: to8(f.bright),
                    fog: p.fog.get(i).copied().unwrap_or(false),
                    hi: p.reach.get(i).copied().unwrap_or(false),
                    sel: p.selected == Some(i),
                    hover: p.hover == Some(i),
                    lit: st.lit,
                }
            })
            .collect();
        let mut out = vec![0u8; self.cw * self.ch * 4];
        for y in 0..self.ch {
            for x in 0..self.cw {
                let k = y * self.cw + x;
                let Some(inf) = info.get(self.ids[k] as usize) else {
                    continue; // the water: left clear
                };
                let edge = self.edge[k];
                let [mut r, mut g, mut b] = inf.c;
                let mut a = 0.05f32;
                if let Some((dr, dg)) = &self.smooth {
                    // anti-aliased: a thin ink line, a bright band and a faint tint fading off the
                    // border, a heavier ink between the ten provinces
                    let (d, dp) = (dr[k], dg[k]);
                    if inf.hi {
                        [r, g, b] = [242.0, 181.0, 74.0];
                        a = 0.2;
                    }
                    if inf.hover {
                        a = a.max(0.14);
                    }
                    if !inf.lit {
                        r = r * 0.5 + 60.0;
                        g = g * 0.5 + 56.0;
                        b = b * 0.5 + 66.0;
                        a = a.max(0.22);
                    }
                    let mut lay = |c: [f32; 3], alpha: f32, t: f32| {
                        let t = t.clamp(0.0, 1.0);
                        r += (c[0] - r) * t;
                        g += (c[1] - g) * t;
                        b += (c[2] - b) * t;
                        a += (alpha - a) * t;
                    };
                    lay(inf.c, 0.3, 1.0 - smooth(0.7, 1.05, d));
                    if inf.hi || inf.sel {
                        let gold = if inf.sel { [255.0, 216.0, 138.0] } else { [242.0, 181.0, 74.0] };
                        lay(gold, 0.45, 1.0 - smooth(0.9, 1.5, d));
                    }
                    let band = if inf.sel || inf.hover { [255.0, 226.0, 150.0] } else { inf.b };
                    lay(band, 0.92, 1.0 - smooth(0.42, 0.58, d));
                    lay([20.0, 10.0, 28.0], 0.85, 1.0 - smooth(0.1, 0.22, d));
                    lay([14.0, 8.0, 20.0], 0.95, 1.0 - smooth(0.2, 0.34, dp));
                    if inf.fog {
                        let n = noise(x as f32 / 40.0, y as f32 / 40.0, 5) * 0.25;
                        r = r * 0.15 + 34.0;
                        g = g * 0.15 + 26.0;
                        b = b * 0.15 + 52.0;
                        a = a.max(0.72 + n);
                    }
                    let o = k * 4;
                    out[o] = r.clamp(0.0, 255.0) as u8;
                    out[o + 1] = g.clamp(0.0, 255.0) as u8;
                    out[o + 2] = b.clamp(0.0, 255.0) as u8;
                    out[o + 3] = (a * 255.0).clamp(0.0, 255.0) as u8;
                    continue;
                }
                if inf.hi {
                    [r, g, b] = [242.0, 181.0, 74.0];
                    a = 0.2;
                }
                if inf.hover {
                    a = a.max(0.14);
                }
                if !inf.lit {
                    r = r * 0.5 + 60.0;
                    g = g * 0.5 + 56.0;
                    b = b * 0.5 + 66.0;
                    a = a.max(0.22);
                }
                if edge <= 1 {
                    [r, g, b] = [20.0, 10.0, 28.0];
                    a = 0.85;
                } else if edge <= 3 {
                    [r, g, b] = inf.b;
                    a = 0.92;
                    if inf.sel || inf.hover {
                        [r, g, b] = [255.0, 226.0, 150.0];
                        a = 1.0;
                    }
                } else if edge <= 5 {
                    [r, g, b] = inf.c;
                    a = 0.3;
                } else if edge <= 7 && (inf.hi || inf.sel) {
                    [r, g, b] = if inf.sel {
                        [255.0, 216.0, 138.0]
                    } else {
                        [242.0, 181.0, 74.0]
                    };
                    a = 0.5 - edge as f32 * 0.05;
                }
                if inf.fog {
                    let n = noise(x as f32 / 40.0, y as f32 / 40.0, 5) * 0.25;
                    r = r * 0.15 + 34.0;
                    g = g * 0.15 + 26.0;
                    b = b * 0.15 + 52.0;
                    a = a.max(0.72 + n);
                }
                let o = k * 4;
                out[o] = r.clamp(0.0, 255.0) as u8;
                out[o + 1] = g.clamp(0.0, 255.0) as u8;
                out[o + 2] = b.clamp(0.0, 255.0) as u8;
                out[o + 3] = (a * 255.0).clamp(0.0, 255.0) as u8;
            }
        }
        out
    }
}

impl Default for Provinces {
    fn default() -> Self {
        Self::new()
    }
}

/// The puffs of cloud drifting over the fog: where, how big, over which province.
pub fn clouds() -> Vec<(f32, f32, f32, f32, usize, f32)> {
    let mut r = Rng::new(77);
    (0..220)
        .map(|_| {
            let x = X0 + r.next() * W;
            let z = Z0 + r.next() * D;
            let size = 16.0 + r.next() * 18.0;
            let y = 9.0 + r.next() * 6.0;
            let _drift = r.next();
            let phase = r.next() * std::f32::consts::TAU;
            (x, y, z, size, province_at(x, z), phase)
        })
        .collect()
}

/// The settlement model of a province (its file in `data/assets/war/campaign`).
pub fn settlement_model(kind: &str) -> &str {
    kind
}

/// Who owns what, painted on a settlement's banners.
pub fn flag_color(owner: Faction) -> [f32; 3] {
    match owner {
        Faction::Alianza => rgb(0xece6d4),
        f => rgb(f.info().cloth),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_war_map_2_reads_and_its_regions_are_the_campaigns() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/guerra");
        let (Ok(bin), Ok(json)) = (std::fs::read(dir.join("mapa.bin")), std::fs::read_to_string(dir.join("mapa.json"))) else {
            return; // not exported here
        };
        let m = MapData::parse(&bin, &json).expect("a war map");
        assert_eq!(m.rw, W as usize * PX);
        let mut seen = vec![0usize; PROVINCES.len()];
        for &r in &m.regions {
            if r != 255 {
                seen[r as usize] += 1;
            }
        }
        assert!(seen.iter().all(|&n| n > 200), "{seen:?}");
        for (i, p) in PROVINCES.iter().enumerate() {
            let (sx, sz) = p.site();
            assert_eq!(m.region_near(sx, sz), i, "{} not under its settlement", p.id);
            assert!(m.height(sx, sz) > WATER, "{} under water", p.id);
        }
        assert!(m.trees.len() > 1000);
    }

    #[test]
    fn the_map_builds_and_every_province_has_ground() {
        let land = Land::new();
        assert!(!land.mesh(false).indices.is_empty());
        assert!(!water(&land).indices.is_empty());
        assert!(roads(&land).is_some());
        assert!(woods(&land).len() > 500);
        let provs = Provinces::new();
        let mut seen = vec![0usize; PROVINCES.len()];
        for &i in &provs.ids {
            seen[i as usize] += 1;
        }
        assert!(seen.iter().all(|&n| n > 500), "{seen:?}");
        for (i, p) in PROVINCES.iter().enumerate() {
            assert_eq!(
                provs.at(p.x, p.z),
                Some(i),
                "{} not under its own centre",
                p.id
            );
        }
        let state = Campaign::new(5, false);
        let img = provs.paint(
            &state,
            &Paint {
                fog: vec![false; PROVINCES.len()],
                ..Default::default()
            },
        );
        assert_eq!(img.len(), provs.pw * provs.ph * 4);
        let _ = table(&land);
    }
}
