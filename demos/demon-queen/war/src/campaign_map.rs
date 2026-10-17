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

/// The height of the land (map units, ~metres of the diorama).
pub fn land_height(x: f32, z: f32) -> f32 {
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
}

impl Land {
    pub fn new() -> Land {
        let nx = W as usize + 1;
        let nz = D as usize + 1;
        let mut h = vec![0.0; nx * nz];
        for j in 0..nz {
            for i in 0..nx {
                h[j * nx + i] = land_height(X0 + i as f32, Z0 + j as f32);
            }
        }
        Land { nx, nz, h }
    }

    pub fn height_at(&self, x: f32, z: f32) -> f32 {
        let fx = clamp(x - X0, 0.0, self.nx as f32 - 1.001);
        let fz = clamp(z - Z0, 0.0, self.nz as f32 - 1.001);
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
        let mut vertices = Vec::with_capacity(nx * nz);
        for j in 0..nz {
            for i in 0..nx {
                let k = j * nx + i;
                let (x, z) = (X0 + i as f32, Z0 + j as f32);
                let hl = h[j * nx + i.saturating_sub(1)];
                let hr = h[j * nx + (i + 1).min(nx - 1)];
                let hd = h[j.saturating_sub(1) * nx + i];
                let hu = h[(j + 1).min(nz - 1) * nx + i];
                let n = Vec3::new(hl - hr, 2.0, hd - hu).normalize_or_zero();
                let y = if over { h[k].max(WATER) + 0.05 } else { h[k] };
                let mut v =
                    MeshVertex3D::new([x, y, z], [n.x, n.y, n.z], [(x - X0) / W, (z - Z0) / D]);
                if !over {
                    let slope = (hr - hl).hypot(hu - hd) / 2.0;
                    let mut c = land_color(x, z, h[k]);
                    c = mix(c, rgb(0x7a7088), smooth(0.6, 1.4, slope) * 0.8);
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
    let (nx, nz) = (W as usize + 1, D as usize + 1);
    let mut id = vec![u32::MAX; nx * nz];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vert = |i: usize, j: usize, vertices: &mut Vec<MeshVertex3D>| {
        let k = j * nx + i;
        if id[k] == u32::MAX {
            let (x, z) = (X0 + i as f32, Z0 + j as f32);
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
            let (x, z) = (X0 + i as f32, Z0 + j as f32);
            if (x - X0).min(-X0 - x).min(z - Z0).min(-Z0 - z) < 7.0 {
                continue;
            }
            let wet = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
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
}

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
        for py in 0..ch {
            for px in 0..cw {
                let x = X0 + (px as f32 + 0.5) / PX as f32;
                let z = Z0 + (py as f32 + 0.5) / PX as f32;
                ids[py * cw + px] = province_at(x, z) as u8;
            }
        }
        let mut edge = vec![9u8; cw * ch];
        for y in 0..ch {
            for x in 0..cw {
                let k = y * cw + x;
                let id = ids[k];
                for r in 1..=7usize {
                    if (x >= r && ids[k - r] != id)
                        || (x + r < cw && ids[k + r] != id)
                        || (y >= r && ids[k - r * cw] != id)
                        || (y + r < ch && ids[k + r * cw] != id)
                    {
                        edge[k] = r as u8;
                        break;
                    }
                }
            }
        }
        Provinces { cw, ch, ids, edge }
    }

    /// The province under a point of the map (its index), if it is on the table.
    pub fn at(&self, x: f32, z: f32) -> Option<usize> {
        if x.abs() > W / 2.0 || z.abs() > D / 2.0 {
            return None;
        }
        let px = (((x - X0) * PX as f32) as usize).min(self.cw - 1);
        let pz = (((z - Z0) * PX as f32) as usize).min(self.ch - 1);
        Some(self.ids[pz * self.cw + px] as usize)
    }

    /// The picture: owner tints, inked borders, highlights, the fog (RGBA, sRGB).
    pub fn paint(&self, state: &Campaign, p: &Paint) -> Vec<u8> {
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
                let inf = &info[self.ids[k] as usize];
                let edge = self.edge[k];
                let [mut r, mut g, mut b] = inf.c;
                let mut a = 0.05f32;
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
        assert_eq!(img.len(), provs.cw * provs.ch * 4);
        let _ = table(&land);
    }
}
