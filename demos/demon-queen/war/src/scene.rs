//! The battlefield drawn, as meshes the game uploads once: the ground (vertex-painted grass, dry
//! hilltops, rock on the steeps, mud on the banks, the dirt road, farmland), the river and the
//! marsh water, the trolls' stone bridge, the rock walls of the cliffs, the stone walls and the
//! gatehouse, and the woods, rocks, reeds, grass and flowers as instances of a few small models.
//! Low-poly and flat-shaded for the toon pass, as the prototype's `terrain-mesh.js` and
//! `setpieces.js` were.
use std::f32::consts::{PI, TAU};

use velvet_math::{Mat4, Vec3};
use velvet_render::{Material3D, Mesh3D, MeshVertex3D};

use crate::data::rgb;
use crate::field::{Field, WATER_Y};
use crate::noise::{fbm, noise, smooth, Rng};
use crate::prims::{self, Builder, Shape};

/// A model drawn many times (a kind of tree, a rock, a tuft of grass).
pub struct ScatterKind {
    pub name: &'static str,
    pub mesh: Mesh3D,
    /// Whether it casts a shadow.
    pub shadow: bool,
}

/// One placed copy of a scatter model.
#[derive(Clone, Copy, Debug)]
pub struct Instance {
    pub kind: usize,
    pub transform: Mat4,
    /// Its brightness (1 as modelled).
    pub tint: [f32; 3],
}

/// A set piece from Blender, placed.
#[derive(Clone, Debug)]
pub struct PropPlace {
    pub kind: &'static str,
    pub transform: Mat4,
}

/// Everything of the field that does not move.
pub struct Scenery {
    /// The ground, in tiles (so what is off screen is skipped).
    pub ground: Vec<Mesh3D>,
    /// The water sheet (vertex alpha: clear at the banks).
    pub water: Option<Mesh3D>,
    /// The bridge, the walls (one mesh each, vertex-coloured).
    pub solids: Vec<Mesh3D>,
    pub kinds: Vec<ScatterKind>,
    pub instances: Vec<Instance>,
    pub props: Vec<PropPlace>,
    /// Where the windmills' sails turn.
    pub spinners: Vec<Mat4>,
}

const fn hex(c: u32) -> u32 {
    c
}

struct Pal {
    grass_a: [f32; 3],
    grass_b: [f32; 3],
    lush: [f32; 3],
    dry: [f32; 3],
    forest: [f32; 3],
    rock: [f32; 3],
    mud: [f32; 3],
    bed: [f32; 3],
    road: [f32; 3],
    marsh: [f32; 3],
    wheat: [f32; 3],
    plowed: [f32; 3],
    field2: [f32; 3],
}

fn pal() -> Pal {
    Pal {
        grass_a: rgb(hex(0x5f8a3a)),
        grass_b: rgb(0x89a04a),
        lush: rgb(0x4b7a34),
        // (a dry, pale grass, not the sand that read orange on the high ground under the warm sun)
        dry: rgb(0x9a9d5c),
        forest: rgb(0x435a2c),
        rock: rgb(0x857a8e),
        mud: rgb(0x5d543c),
        bed: rgb(0x3f3c31),
        // Dull trodden dirt: under the warm sun a browner one read orange.
        road: rgb(0x8d7f68),
        marsh: rgb(0x56603a),
        wheat: rgb(0xcfad55),
        plowed: rgb(0x7d5c3c),
        field2: rgb(0x93a24c),
    }
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn scale3(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// Farmland: rotated strips of wheat, plough and pasture (0 wheat, 1 plough, 2 green) and the
/// furrows' stripe.
fn field_pattern(x: f32, z: f32) -> (u8, f32) {
    let a = 0.35f32;
    let u = x * a.cos() - z * a.sin();
    let v = x * a.sin() + z * a.cos();
    let (ci, cj) = ((u / 34.0).floor(), (v / 22.0).floor());
    let h = ((ci * 12.9898 + cj * 78.233).sin() * 43758.547)
        .abs()
        .fract();
    let stripe = (v * 2.1).sin() * 0.5 + 0.5;
    (
        if h < 0.42 {
            0
        } else if h < 0.7 {
            1
        } else {
            2
        },
        stripe,
    )
}

/// The ground's colour at a grid point.
fn ground_color(field: &Field, p: &Pal, i: usize, j: usize) -> [f32; 3] {
    let (nx, nz) = (field.nx, field.nz);
    let k = j * nx + i;
    if let Some(m) = &field.baked {
        // the prototype's own ground, mixed down per node by its exporter (sRGB)
        let c = m.land.colour[k];
        let lin = |v: u8| {
            let s = v as f32 / 255.0;
            if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        let x = field.x0 + i as f32 * field.res;
        let z = field.z0 + j as f32 * field.res;
        let v = 0.94 + noise(x / 3.1, z / 3.1, 77) * 0.1;
        return scale3([lin(c[0]), lin(c[1]), lin(c[2])], v / DETAIL_MEAN);
    }
    let x = field.x0 + i as f32 * field.res;
    let z = field.z0 + j as f32 * field.res;
    let h = &field.h;
    let y = h[k];
    let hl = h[j * nx + i.saturating_sub(1)];
    let hr = h[j * nx + (i + 1).min(nx - 1)];
    let hd = h[j.saturating_sub(1) * nx + i];
    let hu = h[(j + 1).min(nz - 1) * nx + i];
    let slope = (hr - hl).hypot(hu - hd) / (2.0 * field.res);
    let seed = field.spec.seed;
    let n1 = fbm(x / 40.0, z / 40.0, seed + 31, 3);
    let n2 = fbm(x / 13.0, z / 13.0, seed + 47, 2);
    let mut c = mix(p.grass_a, p.grass_b, smooth(0.3, 0.7, n1));
    // lush hollows, and worn earth showing through here and there
    c = mix(c, p.lush, smooth(0.55, 0.8, n2) * 0.6);
    c = mix(
        c,
        p.mud,
        smooth(0.74, 0.88, 1.0 - n2) * 0.3 * (1.0 - field.forest[k]),
    );
    c = mix(c, p.dry, smooth(6.0, 16.0, y) * 0.3);
    if field.spec.fields && field.forest[k] < 0.2 && field.river_d[k] > 20.0 {
        let pl = field.play;
        let in_play = x > pl.x0 - 30.0 && x < pl.x1 + 30.0 && z > pl.z0 - 30.0 && z < pl.z1 + 30.0;
        if in_play {
            let (kind, stripe) = field_pattern(x, z);
            let t = match kind {
                0 => p.wheat,
                1 => p.plowed,
                _ => p.field2,
            };
            c = mix(c, scale3(t, 0.92 + stripe * 0.12), 0.75);
        }
    }
    c = mix(c, p.forest, smooth(0.15, 0.7, field.forest[k]) * 0.85);
    c = mix(c, p.marsh, field.marsh[k] * 0.9);
    c = mix(c, p.road, field.road[k] * 0.85);
    c = mix(
        c,
        scale3(p.rock, 0.88 + n2 * 0.24),
        smooth(0.42, 0.85, slope),
    );
    let depth = WATER_Y - y;
    // a band of wet earth at the water (narrow: in a flat marsh a wide one ringed every pool in brown)
    if depth > -0.25 {
        c = mix(c, p.mud, smooth(-0.25, 0.05, depth) * 0.7);
    }
    if depth > 0.2 {
        c = mix(c, p.bed, smooth(0.2, 1.2, depth));
    }
    let v = 0.92 + noise(x / 3.1, z / 3.1, seed + 77) * 0.14;
    // (the detail picture over it darkens by about this much on average)
    scale3(c, v / DETAIL_MEAN)
}

/// The detail picture's mean brightness (the ground's colours are lifted by as much).
const DETAIL_MEAN: f32 = 0.88;

/// A tiling picture of the ground's grain, grey (it multiplies the painted colours): soft blotches,
/// blades of grass with lighter tips, grit. `size` pixels square, RGBA, repeating at its edges.
pub fn ground_detail(size: usize) -> Vec<u8> {
    let n = size;
    let mut r = Rng::new(9137);
    // soft blotches: value noise on a lattice that wraps
    let lattice = |cells: usize, salt: u32| {
        let mut rr = Rng::new(salt);
        (0..cells * cells).map(|_| rr.next()).collect::<Vec<f32>>()
    };
    let (l1, l2) = (lattice(4, 11), lattice(11, 23));
    let sample = |l: &[f32], cells: usize, x: f32, y: f32| {
        let (fx, fy) = (x * cells as f32, y * cells as f32);
        let (i0, j0) = (fx.floor() as usize % cells, fy.floor() as usize % cells);
        let (i1, j1) = ((i0 + 1) % cells, (j0 + 1) % cells);
        let (tx, ty) = (fx.fract(), fy.fract());
        let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let a = l[j0 * cells + i0] + (l[j0 * cells + i1] - l[j0 * cells + i0]) * tx;
        let b = l[j1 * cells + i0] + (l[j1 * cells + i1] - l[j1 * cells + i0]) * tx;
        a + (b - a) * ty
    };
    let mut v = vec![0.0f32; n * n];
    for y in 0..n {
        for x in 0..n {
            let (u, w) = (x as f32 / n as f32, y as f32 / n as f32);
            v[y * n + x] =
                0.95 + (sample(&l1, 4, u, w) - 0.5) * 0.1 + (sample(&l2, 11, u, w) - 0.5) * 0.08;
        }
    }
    let wrap = |p: f32| (p.rem_euclid(n as f32)) as usize % n;
    // blades: many short, soft strokes, shaded at the root and lit at the tip, some all light
    let blades = n * n / 26;
    for _ in 0..blades {
        let (x, y) = (r.next() * n as f32, r.next() * n as f32);
        let a = r.next() * std::f32::consts::TAU;
        let len = n as f32 * (0.008 + r.next() * 0.012);
        let (dx, dy) = (a.cos(), a.sin());
        let lit = r.next() < 0.45;
        let steps = (len * 1.5) as usize + 1;
        for k in 0..steps {
            let t = k as f32 / steps as f32;
            let shade = if lit { 1.0 + t * 0.06 } else { 0.84 + t * 0.2 };
            let p = wrap(x + dx * len * t) + wrap(y + dy * len * t) * n;
            v[p] = v[p] * 0.55 + shade * 0.45;
        }
    }
    // grit: specks of soil and a few light ones
    for _ in 0..n * n / 140 {
        let p = wrap(r.next() * n as f32) + wrap(r.next() * n as f32) * n;
        v[p] *= if r.next() < 0.65 { 0.82 } else { 1.08 };
    }
    // to the mean the ground's colours expect
    let mean = v.iter().sum::<f32>() / v.len() as f32;
    let mut out = Vec::with_capacity(n * n * 4);
    for x in v {
        let g = (x * DETAIL_MEAN / mean).clamp(0.0, 1.0);
        let b = (g * 255.0).round() as u8;
        out.extend([b, b, b, 255]);
    }
    out
}

/// The ground in tiles of `tile` cells, smooth-shaded from the height field.
pub fn ground(field: &Field, tile: usize) -> Vec<Mesh3D> {
    let p = pal();
    let (nx, nz) = (field.nx, field.nz);
    let h = &field.h;
    let normal = |i: usize, j: usize| {
        let hl = h[j * nx + i.saturating_sub(1)];
        let hr = h[j * nx + (i + 1).min(nx - 1)];
        let hd = h[j.saturating_sub(1) * nx + i];
        let hu = h[(j + 1).min(nz - 1) * nx + i];
        let n = Vec3::new(hl - hr, 2.0 * field.res, hd - hu).normalize_or_zero();
        [n.x, n.y, n.z]
    };
    let mut colors = vec![[0.0f32; 3]; nx * nz];
    for j in 0..nz {
        for i in 0..nx {
            colors[j * nx + i] = ground_color(field, &p, i, j);
        }
    }
    let mut out = Vec::new();
    let mut tj = 0;
    while tj + 1 < nz {
        let j1 = (tj + tile).min(nz - 1);
        let mut ti = 0;
        while ti + 1 < nx {
            let i1 = (ti + tile).min(nx - 1);
            let w = i1 - ti + 1;
            let mut vertices = Vec::with_capacity(w * (j1 - tj + 1));
            for j in tj..=j1 {
                for i in ti..=i1 {
                    let k = j * nx + i;
                    let x = field.x0 + i as f32 * field.res;
                    let z = field.z0 + j as f32 * field.res;
                    let mut v = MeshVertex3D::new([x, h[k], z], normal(i, j), [x / 6.0, z / 6.0]);
                    let c = colors[k];
                    v.color = [c[0], c[1], c[2], 1.0];
                    vertices.push(v);
                }
            }
            let mut indices = Vec::with_capacity((w - 1) * (j1 - tj) * 6);
            for j in 0..(j1 - tj) {
                for i in 0..(w - 1) {
                    let a = (j * w + i) as u32;
                    let b = a + 1;
                    let c = a + w as u32;
                    let d = c + 1;
                    indices.extend([a, c, b, b, c, d]);
                }
            }
            out.push(Mesh3D {
                vertices,
                indices,
                material: Material3D::default(),
                metallic_factor: 0.0,
                roughness_factor: 1.0,
                texture_index: None,
                normal_texture_index: None,
                skinning: None,
            });
            ti = i1;
        }
        tj = j1;
    }
    out
}

/// One sheet of water at the water line where the ground dips under it: shallow to deep, a line
/// of foam along the banks, clear at the very edge.
pub fn water(field: &Field) -> Option<Mesh3D> {
    if field.baked.is_some() {
        return baked_water(field);
    }
    let step = 3.0f32;
    let nx = (field.w / step).floor() as usize + 1;
    let nz = (field.d / step).floor() as usize + 1;
    let wet = |x: f32, z: f32| WATER_Y - field.height_at(x, z) > -0.6;
    let marsh = !field.spec.marsh.is_empty();
    // (a marsh's pools are a murky teal: near-grey ones read under the warm light as mud or holes)
    let shallow = if marsh { rgb(0x2e7468) } else { rgb(0x4f9a96) };
    // (the prototype's #2a2f6c, a little bluer: under the toon light and its violet sky it
    // read as purple)
    let deep = if marsh { rgb(0x1b4656) } else { rgb(0x23406a) };
    let foam = rgb(0xf6ecd8);
    let mut id = vec![u32::MAX; nx * nz];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut vert = |i: usize, j: usize, vertices: &mut Vec<MeshVertex3D>| {
        let k = j * nx + i;
        if id[k] == u32::MAX {
            let (x, z) = (field.x0 + i as f32 * step, field.z0 + j as f32 * step);
            let depth = WATER_Y - field.height_at(x, z);
            let d = (depth / 1.6).clamp(0.0, 1.0);
            let mut c = mix(
                shallow,
                deep,
                smooth(0.25, 0.45, d) * 0.7 + smooth(0.6, 0.85, d) * 0.3,
            );
            if marsh {
                // still water, as dark in the shallows as in the middle
                c = mix(c, deep, 0.35);
            }
            // drifting ripple bands, frozen: a lighter step here and there
            let r = noise(x * 0.22, z * 0.22, 5) * 0.6 + noise(x * 0.6, z * 0.6, 6) * 0.4;
            if r > 0.74 {
                c = [
                    c[0] + 0.05 * (1.0 - d * 0.5),
                    c[1] + 0.05 * (1.0 - d * 0.5),
                    c[2] + 0.05 * (1.0 - d * 0.5),
                ];
            }
            // the warm sky in it (the prototype's grazing reflection, as the eye mostly sees it)
            c = mix(
                c,
                rgb(0xf0a882),
                if marsh { 0.04 } else { 0.14 } * (1.0 - d),
            );
            // a thin line of foam along the bank: only where the next point is dry land (a flat,
            // shallow pool is shallow all over and was foam all over)
            let shore = [(-step, 0.0), (step, 0.0), (0.0, -step), (0.0, step)]
                .iter()
                .any(|&(dx, dz)| WATER_Y - field.height_at(x + dx, z + dz) < 0.0);
            let edge = if shore {
                1.0 - smooth(0.0, 0.12, depth + (r - 0.5) * 0.03)
            } else {
                0.0
            };
            c = mix(c, foam, edge * if marsh { 0.15 } else { 0.6 });
            let a = (if marsh { 0.88 } else { 0.74 } + (0.95 - 0.74) * smooth(0.1, 0.7, d))
                .min(0.96)
                * smooth(-0.02, 0.02, depth);
            let mut v = MeshVertex3D::new([x, WATER_Y, z], [0.0, 1.0, 0.0], [0.0, 0.0]);
            v.color = [c[0], c[1], c[2], a];
            id[k] = vertices.len() as u32;
            vertices.push(v);
        }
        id[k]
    };
    for j in 0..nz - 1 {
        for i in 0..nx - 1 {
            let (x, z) = (field.x0 + i as f32 * step, field.z0 + j as f32 * step);
            if !(wet(x, z) || wet(x + step, z) || wet(x, z + step) || wet(x + step, z + step)) {
                continue;
            }
            let a = vert(i, j, &mut vertices);
            let b = vert(i + 1, j, &mut vertices);
            let c = vert(i, j + 1, &mut vertices);
            let d = vert(i + 1, j + 1, &mut vertices);
            indices.extend([a, c, b, b, c, d]);
        }
    }
    (!indices.is_empty()).then(|| Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        // (a marsh is still and murky: the sky's glare off a glassy one turned its pools pale)
        roughness_factor: if marsh { 0.8 } else { 0.2 },
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    })
}

/// A baked map's water: a sheet at each pool's, river's or sea's own level on the land's grid,
/// shallow to deep, foam where it meets the bank, murky where it is marsh.
fn baked_water(field: &Field) -> Option<Mesh3D> {
    let m = field.baked.as_ref()?;
    let l = &m.land;
    let (nx, nz) = (l.nx, l.nz);
    let shallow = rgb(0x4f9a96);
    let deep = rgb(0x23406a);
    let marsh_c = (rgb(0x2e7468), rgb(0x1b4656));
    let foam = rgb(0xf6ecd8);
    let mut id = vec![u32::MAX; nx * nz];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let wet = |k: usize| l.water_banks[k] > crate::baked::NO_WATER;
    for j in 0..nz - 1 {
        for i in 0..nx - 1 {
            let ks = [
                j * nx + i,
                j * nx + i + 1,
                (j + 1) * nx + i,
                (j + 1) * nx + i + 1,
            ];
            if !ks.iter().any(|&k| l.water[k] > crate::baked::NO_WATER)
                || !ks.iter().all(|&k| wet(k))
            {
                continue;
            }
            let mut vid = [0u32; 4];
            for (c, &k) in ks.iter().enumerate() {
                if id[k] == u32::MAX {
                    let (ii, jj) = (k % nx, k / nx);
                    let (x, z) = (l.x0 + ii as f32 * l.res, l.z0 + jj as f32 * l.res);
                    let y = l.water_banks[k];
                    let depth = y - l.h[k];
                    let d = (depth / 1.6).clamp(0.0, 1.0);
                    let marshy = l.marsh[k] > 0.4;
                    let (sh, dp) = if marshy { marsh_c } else { (shallow, deep) };
                    let mut col = mix(
                        sh,
                        dp,
                        smooth(0.25, 0.45, d) * 0.7 + smooth(0.6, 0.85, d) * 0.3,
                    );
                    let r = noise(x * 0.22, z * 0.22, 5) * 0.6 + noise(x * 0.6, z * 0.6, 6) * 0.4;
                    if r > 0.74 {
                        col = scale3(col, 1.0 + 0.12 * (1.0 - d * 0.5));
                    }
                    col = mix(
                        col,
                        rgb(0xf0a882),
                        if marshy { 0.04 } else { 0.12 } * (1.0 - d),
                    );
                    let shore = l.water[k] <= crate::baked::NO_WATER || depth < 0.12;
                    if shore && !marshy {
                        col = mix(col, foam, 0.45);
                    }
                    let a = (if marshy { 0.88 } else { 0.74 }
                        + (0.95 - 0.74) * smooth(0.1, 0.7, d))
                    .min(0.96)
                        * smooth(-0.05, 0.05, depth);
                    let mut v = MeshVertex3D::new([x, y, z], [0.0, 1.0, 0.0], [0.0, 0.0]);
                    v.color = [col[0], col[1], col[2], a];
                    id[k] = vertices.len() as u32;
                    vertices.push(v);
                }
                vid[c] = id[k];
            }
            indices.extend([vid[0], vid[2], vid[1], vid[1], vid[2], vid[3]]);
        }
    }
    (!indices.is_empty()).then(|| Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 0.25,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    })
}

/// A box placed like three.js does: rotated (pitch about x, then yaw about y, order YXZ), moved.
fn placed(shape: Shape, pitch: f32, yaw: f32, at: [f32; 3]) -> Shape {
    shape
        .rotate_x(-pitch)
        .rotate_y(yaw)
        .translate(at[0], at[1], at[2])
}

/// The trolls' bridge: an arched deck in segments, parapets, piers down into the river, posts
/// with mossy stones at its ends.
fn bridge(field: &Field, out: &mut Builder) {
    let Some(b) = field.bridge else {
        return;
    };
    let stone = rgb(0x8f8496);
    let dark = rgb(0x6a5f72);
    let moss = rgb(0x5d7040);
    let yaw = b.ux.atan2(b.uz);
    let seg = 14;
    let len = 2.0 * b.half / seg as f32;
    for s in 0..seg {
        let u0 = -b.half + s as f32 * len;
        let u1 = u0 + len;
        let um = (u0 + u1) / 2.0;
        let y0 = b.base + b.rise * (1.0 - (u0 / b.half).powi(2));
        let y1 = b.base + b.rise * (1.0 - (u1 / b.half).powi(2));
        let ym = (y0 + y1) / 2.0;
        let pitch = (y1 - y0).atan2(len);
        let at = [b.x + b.ux * um, ym - 0.36, b.z + b.uz * um];
        out.add(
            &placed(prims::boxed(b.width, 0.7, len + 0.12), pitch, yaw, at),
            stone,
        );
        for side in [-1.0f32, 1.0] {
            let w = [
                at[0] + b.vx * side * (b.width / 2.0 - 0.25),
                ym + 0.4,
                at[2] + b.vz * side * (b.width / 2.0 - 0.25),
            ];
            out.add(
                &placed(prims::boxed(0.55, 0.85, len + 0.1), pitch, yaw, w),
                if s % 3 == 1 { dark } else { stone },
            );
        }
    }
    for u in [-b.half * 0.38, b.half * 0.38] {
        let top = b.base + b.rise * (1.0 - (u / b.half).powi(2)) - 0.6;
        let pier = prims::boxed(b.width + 0.6, top + 3.4, 2.4)
            .rotate_y(yaw)
            .translate(b.x + b.ux * u, top - (top + 3.4) / 2.0, b.z + b.uz * u);
        out.add(&pier, dark);
    }
    for end in [-1.0f32, 1.0] {
        for side in [-1.0f32, 1.0] {
            let px = b.x + b.ux * end * (b.half + 0.8) + b.vx * side * (b.width / 2.0 + 0.6);
            let pz = b.z + b.uz * end * (b.half + 0.8) + b.vz * side * (b.width / 2.0 + 0.6);
            let gy = field.height_at(px, pz);
            out.add(
                &prims::cylinder(0.6, 0.85, 2.6, 7).translate(px, gy + 1.2, pz),
                stone,
            );
            out.add(
                &prims::dodecahedron(0.8, 0).translate(px, gy + 2.8, pz),
                moss,
            );
        }
    }
}

/// Walls along their lines, open at their gaps; tall ones get merlons and a gatehouse.
fn walls(field: &Field, out: &mut Builder) {
    let stone = rgb(0xa39aa8);
    let dark = rgb(0x857c8c);
    let roof = rgb(0x5a3e8a);
    let wood = rgb(0x6a4a2e);
    let mut r = Rng::new((field.spec.seed * 7 + 3) as u32);
    let mut jit = Rng::new(11);
    let mut paint = |out: &mut Builder, shape: &Shape, c: [f32; 3]| {
        let ks: Vec<f32> = (0..shape.tris.len() / 3)
            .map(|_| 1.0 + (jit.next() - 0.5) * 0.12)
            .collect();
        out.add_shaded(shape, |k| scale3(c, ks[k]));
    };
    for w in &field.walls {
        let l = &w.line;
        let (hh, t) = (w.height, w.thick);
        let in_gap = |s: f32| w.gaps.iter().any(|&(at, gw)| (s - at).abs() < gw / 2.0);
        for k in 0..l.len().saturating_sub(1) {
            let (a, b) = (&l[k], &l[k + 1]);
            if in_gap((a.s + b.s) / 2.0) {
                continue;
            }
            let len = (b.x - a.x).hypot(b.z - a.z) + 0.05;
            let yaw = (b.x - a.x).atan2(b.z - a.z);
            let (mx, mz) = ((a.x + b.x) / 2.0, (a.z + b.z) / 2.0);
            let gy = field.height_at(a.x, a.z).min(field.height_at(b.x, b.z));
            let h = hh * if w.low { 0.85 + r.next() * 0.3 } else { 1.0 };
            let seg = prims::boxed(t, h + 1.2, len).rotate_y(yaw).translate(
                mx,
                gy - 0.6 + (h + 1.2) / 2.0,
                mz,
            );
            paint(out, &seg, if k % 5 == 2 { dark } else { stone });
            if !w.low && k % 2 == 0 {
                let m = prims::boxed(t + 0.3, 1.0, len.min(1.2))
                    .rotate_y(yaw)
                    .translate(mx, gy + h + 0.5, mz);
                paint(out, &m, stone);
            }
        }
        if let Some(gate) = w.gatehouse {
            let at = |s: f32| {
                l.iter()
                    .min_by(|p, q| (p.s - s).abs().total_cmp(&(q.s - s).abs()))
                    .copied()
                    .unwrap_or(l[0])
            };
            let gap = w.gaps.iter().find(|g| g.0 == gate).map_or(10.0, |g| g.1);
            for side in [-1.0f32, 1.0] {
                let p = at(gate + side * (gap / 2.0 + 3.0));
                let gy = field.height_at(p.x, p.z);
                paint(
                    out,
                    &prims::cylinder(4.0, 4.6, hh + 6.0, 10).translate(
                        p.x,
                        gy - 1.0 + (hh + 6.0) / 2.0,
                        p.z,
                    ),
                    stone,
                );
                paint(
                    out,
                    &prims::cone(5.0, 5.0, 10).translate(p.x, gy + hh + 7.4, p.z),
                    roof,
                );
                paint(
                    out,
                    &prims::cylinder(4.4, 4.4, 0.8, 10).translate(p.x, gy + hh + 4.6, p.z),
                    dark,
                );
            }
            let c = at(gate);
            let gy = field.height_at(c.x, c.z);
            let yaw = c.tx.atan2(c.tz);
            paint(
                out,
                &prims::boxed(t + 0.6, 2.2, gap + 4.0)
                    .rotate_y(yaw)
                    .translate(c.x, gy + hh + 1.6, c.z),
                stone,
            );
            for side in [-1.0f32, 1.0] {
                let door = prims::boxed(0.4, hh - 0.5, gap / 2.0 - 1.0)
                    .translate(0.0, 0.0, side * (gap / 4.0 + 2.5))
                    .rotate_y(yaw + side * 1.1);
                let (px, pz) = (
                    c.x + c.tx * side * (gap / 2.0 - 0.5),
                    c.z + c.tz * side * (gap / 2.0 - 0.5),
                );
                paint(out, &door.translate(px, gy + (hh - 0.5) / 2.0, pz), wood);
            }
        }
    }
}

fn shape_mesh(build: impl FnOnce(&mut Builder)) -> Mesh3D {
    let mut b = Builder::default();
    build(&mut b);
    b.build()
        .unwrap_or_else(|| Builder::default().build_empty())
}

trait Empty {
    fn build_empty(self) -> Mesh3D;
}

impl Empty for Builder {
    fn build_empty(self) -> Mesh3D {
        let mut b = Builder::default();
        b.add(&prims::boxed(0.01, 0.01, 0.01), [0.0; 3]);
        b.build().expect("a box")
    }
}

/// The scatter models: five trees, a rock, a crag, moss, a tuft, a flower, a reed.
pub fn kinds() -> Vec<ScatterKind> {
    let pine = shape_mesh(|b| {
        b.add(
            &prims::cylinder(0.2, 0.32, 2.4, 6).translate(0.0, 1.2, 0.0),
            rgb(0x5a3a2c),
        );
        b.add(
            &prims::cone(2.4, 3.3, 7)
                .translate(0.0, 3.6, 0.0)
                .jagged(0.35, 1),
            rgb(0x2f5a4c),
        );
        b.add(
            &prims::cone(1.9, 2.9, 7)
                .rotate_y(0.4)
                .translate(0.0, 5.3, 0.0)
                .jagged(0.3, 2),
            rgb(0x376a56),
        );
        b.add(
            &prims::cone(1.25, 2.4, 7)
                .rotate_y(0.9)
                .translate(0.0, 6.8, 0.0)
                .jagged(0.25, 3),
            rgb(0x407460),
        );
    });
    let leafy = |a: u32, c: u32, seed: u32| {
        shape_mesh(|b| {
            b.add(
                &prims::cylinder(0.24, 0.4, 3.0, 6).translate(0.0, 1.5, 0.0),
                rgb(0x5c3e2c),
            );
            b.add(
                &prims::icosahedron(2.2, 0)
                    .translate(0.0, 4.3, 0.0)
                    .jagged(0.5, seed),
                rgb(a),
            );
            b.add(
                &prims::icosahedron(1.6, 0)
                    .translate(1.0, 5.2, 0.4)
                    .jagged(0.4, seed + 1),
                rgb(c),
            );
            b.add(
                &prims::icosahedron(1.45, 0)
                    .translate(-0.9, 4.9, -0.6)
                    .jagged(0.4, seed + 2),
                rgb(a),
            );
        })
    };
    let dead = shape_mesh(|b| {
        let c = rgb(0x6e6470);
        b.add(
            &prims::cylinder(0.16, 0.34, 4.2, 5).translate(0.0, 2.1, 0.0),
            c,
        );
        b.add(
            &prims::cylinder(0.08, 0.14, 2.0, 4)
                .rotate_z(0.9)
                .translate(0.7, 3.4, 0.0),
            c,
        );
        b.add(
            &prims::cylinder(0.07, 0.12, 1.6, 4)
                .rotate_z(-1.0)
                .translate(-0.6, 2.8, 0.1),
            c,
        );
    });
    let rock = shape_mesh(|b| {
        b.add(
            &prims::dodecahedron(1.0, 0)
                .jagged(0.45, 5)
                .scale(1.0, 0.62, 1.0),
            rgb(0x8a7e94),
        )
    });
    let crag = shape_mesh(|b| {
        let s = prims::dodecahedron(1.0, 1).lumpy(3, 0.78, 0.4);
        let mut r = Rng::new(3);
        let ks: Vec<f32> = (0..s.tris.len() / 3)
            .map(|_| 1.0 + (r.next() - 0.5) * 0.16)
            .collect();
        let c = rgb(0x857a92);
        b.add_shaded(&s, |k| scale3(c, ks[k]));
    });
    let moss = shape_mesh(|b| {
        let s = prims::icosahedron(1.0, 0);
        let mut r = Rng::new(5);
        let ks: Vec<f32> = (0..s.tris.len() / 3)
            .map(|_| 1.0 + (r.next() - 0.5) * 0.2)
            .collect();
        let c = rgb(0x5f7a42);
        b.add_shaded(&s, |k| scale3(c, ks[k]));
    });
    let tuft = shape_mesh(|b| {
        // a clump of blades fanning out from its root
        let greens = [
            0x6f9440, 0x86a64a, 0x5f8638, 0x7c9e44, 0x93ad52, 0x688d3c, 0x7fa046,
        ];
        for (k, &c) in greens.iter().enumerate() {
            let a = k as f32 * 0.9;
            let lean = 0.32 + (k % 3) as f32 * 0.1;
            let h = 0.5 + ((k * 7) % 5) as f32 * 0.09;
            b.add(
                &prims::cone(0.055, h, 3)
                    .translate(0.0, h / 2.0, 0.0)
                    .rotate_z(lean)
                    .rotate_y(a)
                    .translate(a.cos() * 0.06, 0.0, a.sin() * 0.06),
                rgb(c),
            );
        }
    });
    let flower = |petal: u32| {
        shape_mesh(|b| {
            let c = rgb(petal);
            b.add(&prims::icosahedron(0.11, 0).translate(0.0, 0.3, 0.0), c);
            b.add(&prims::icosahedron(0.1, 0).translate(0.18, 0.24, 0.1), c);
            b.add(
                &prims::icosahedron(0.09, 0).translate(-0.12, 0.22, -0.14),
                c,
            );
            b.add(
                &prims::cylinder(0.015, 0.015, 0.3, 3).translate(0.0, 0.15, 0.0),
                rgb(0x5a7a30),
            );
        })
    };
    let reed = shape_mesh(|b| {
        b.add(
            &prims::cone(0.1, 1.4, 4).translate(0.0, 0.7, 0.0),
            rgb(0x8a9a48),
        );
        b.add(
            &prims::cone(0.09, 1.15, 4).translate(0.2, 0.57, 0.1),
            rgb(0xa4a456),
        );
        b.add(
            &prims::cone(0.09, 1.25, 4).translate(-0.14, 0.62, 0.16),
            rgb(0x7c9044),
        );
        b.add(
            &prims::cylinder(0.07, 0.07, 0.3, 5).translate(0.2, 1.18, 0.1),
            rgb(0x7a5634),
        );
    });
    let ford = shape_mesh(|b| {
        b.add(
            &prims::dodecahedron(1.0, 0)
                .jagged(0.45, 9)
                .scale(1.0, 0.62, 1.0),
            rgb(0x9d93a6),
        )
    });
    let k = |name, mesh, shadow| ScatterKind { name, mesh, shadow };
    vec![
        k("pino", pine, true),
        k("roble", leafy(0x55803a, 0x6a9442, 11), true),
        k("violeta", leafy(0x6d4f8f, 0x8a64a8, 21), true),
        k("ambar", leafy(0xc4863a, 0xd9a04c, 31), true),
        k("seco", dead, true),
        k("roca", rock, true),
        k("peña", crag, true),
        k("musgo", moss, true),
        k("hierba", tuft, false),
        k("flor_ambar", flower(0xf2b54a), false),
        k("flor_violeta", flower(0xb98aff), false),
        k("flor_blanca", flower(0xffe9d0), false),
        k("junco", reed, false),
        k("piedra_vado", ford, false),
    ]
}

const PINE: usize = 0;
const OAK: usize = 1;
const VIOLET: usize = 2;
const AMBER: usize = 3;
const DEAD: usize = 4;
const ROCK: usize = 5;
const CRAG: usize = 6;
const MOSS: usize = 7;
const TUFT: usize = 8;
const FLOWER: usize = 9;
const REED: usize = 12;
const FORD: usize = 13;

fn place(x: f32, y: f32, z: f32, yaw: f32, sx: f32, sy: f32, sz: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(x, y, z))
        * Mat4::from_rotation_y(yaw)
        * Mat4::from_scale(Vec3::new(sx, sy, sz))
}

/// A baked map's woods and rocks as the prototype placed them, its grass, crops and reeds by its
/// densities, drawn with the battle's own toon models.
fn baked_scatter(field: &Field, out: &mut Vec<Instance>) {
    let Some(m) = &field.baked else {
        return;
    };
    let l = &m.land;
    let leaf = |x: f32, z: f32| {
        let v = 0.88 + noise(x / 8.0, z / 8.0, 3) * 0.24;
        [v, v, v]
    };
    for t in &l.trees {
        let (kind, sy, sxz) = match t.kind {
            0 => (PINE, 1.0, 1.0),
            1 => (OAK, 1.0, 1.0),
            2 => (VIOLET, 1.05, 1.0),
            3 => (DEAD, 1.0, 1.0),
            _ => (MOSS, 0.7, 1.25),
        };
        let s = t.scale;
        out.push(Instance {
            kind,
            transform: place(
                t.x,
                field.height_at(t.x, t.z) - 0.12,
                t.z,
                t.rot,
                s * sxz,
                s * sy,
                s * sxz,
            ),
            tint: leaf(t.x, t.z),
        });
    }
    let mut r = Rng::new(7177);
    for rk in &l.rocks {
        let s = rk.scale;
        let big = s > 1.6;
        let sy = 0.55 + r.next() * 0.6;
        let lt = 0.72 + r.next() * 0.22;
        out.push(Instance {
            kind: if big { CRAG } else { ROCK },
            transform: place(
                rk.x,
                field.height_at(rk.x, rk.z) - 0.3 * s,
                rk.z,
                rk.rot,
                s * if big { 1.6 } else { 1.0 },
                s * sy * if big { 1.4 } else { 1.0 },
                s * if big { 1.6 } else { 1.0 },
            ),
            tint: [lt * 1.02, lt, lt * 1.04],
        });
    }
    let node = |x: f32, z: f32| {
        let i = ((x - l.x0) / l.res).round().clamp(0.0, l.nx as f32 - 1.0) as usize;
        let j = ((z - l.z0) / l.res).round().clamp(0.0, l.nz as f32 - 1.0) as usize;
        j * l.nx + i
    };
    let play = field.play;
    let (mut tufts, mut reeds, mut crops) = (0, 0, 0);
    for _ in 0..60000 {
        if tufts >= 3200 && reeds >= 600 && crops >= 900 {
            break;
        }
        let px = play.x0 - 30.0 + r.next() * (play.x1 - play.x0 + 60.0);
        let pz = play.z0 - 30.0 + r.next() * (play.z1 - play.z0 + 60.0);
        if field.blocked(px, pz) && field.depth_at(px, pz) < 0.0 {
            continue;
        }
        let k = node(px, pz);
        let roll = r.next() * 255.0;
        let y = field.height_at(px, pz) - 0.04;
        let yaw = r.next() * TAU;
        if (l.reeds[k] as f32) > roll && reeds < 600 {
            let s = 0.75 + r.next() * 0.7;
            out.push(Instance {
                kind: REED,
                transform: place(px, y, pz, yaw, s, s, s),
                tint: [1.0; 3],
            });
            reeds += 1;
        } else if field.depth_at(px, pz) > -0.1 {
            continue;
        } else if (l.crop[k] as f32) > roll && crops < 900 {
            let s = 0.8 + r.next() * 0.5;
            out.push(Instance {
                kind: TUFT,
                transform: place(px, y, pz, yaw, s, s * 1.3, s),
                tint: [1.55, 1.2, 0.55],
            });
            crops += 1;
        } else if (l.grass[k] as f32) * 0.8 > roll && tufts < 3200 {
            if r.next() < 0.06 {
                let hue = r.next();
                let kind = FLOWER
                    + if hue < 0.5 {
                        0
                    } else if hue < 0.8 {
                        1
                    } else {
                        2
                    };
                let s = 0.8 + r.next() * 0.6;
                out.push(Instance {
                    kind,
                    transform: place(px, y, pz, yaw, s, s, s),
                    tint: [1.0; 3],
                });
            } else {
                let s = 0.7 + r.next() * 0.8;
                let hue = (noise(px / 20.0, pz / 20.0, 4) - 0.5) * 0.08;
                let lt = 0.75 + r.next() * 0.3;
                out.push(Instance {
                    kind: TUFT,
                    transform: place(px, y, pz, yaw, s, s, s),
                    tint: [lt * (1.0 + hue), lt, lt * (1.0 - hue)],
                });
                tufts += 1;
            }
        }
    }
}

/// Woods, rocks, reeds, grass and flowers from the field's maps, and the cliffs' crags.
fn scatter(field: &Field, out: &mut Vec<Instance>) {
    if field.baked.is_some() {
        baked_scatter(field, out);
        return;
    }
    let seed = field.spec.seed;
    let mut r = Rng::new((seed * 31 + 5) as u32);
    let cell = 4.4f32;
    let (x0, z0, w, d, play) = (field.x0, field.z0, field.w, field.d, field.play);
    let leaf = |x: f32, z: f32| {
        let v = 0.88 + noise(x / 8.0, z / 8.0, 3) * 0.24;
        [v, v, v]
    };
    let mut z = z0 + 6.0;
    while z < -z0 - 6.0 {
        let mut x = x0 + 6.0;
        while x < -x0 - 6.0 {
            let px = x + (r.next() - 0.5) * cell * 0.9;
            let pz = z + (r.next() - 0.5) * cell * 0.9;
            x += cell;
            let f = field.forest_at(px, pz);
            if f < 0.25 || r.next() > f * 1.1 {
                continue;
            }
            if field.depth_at(px, pz) > -0.25 {
                continue;
            }
            let y = field.height_at(px, pz);
            let nn = noise(px / 37.0, pz / 37.0, seed + 41);
            let mut kind = if nn < 0.45 {
                PINE
            } else if nn < 0.8 {
                OAK
            } else if nn < 0.92 {
                VIOLET
            } else {
                AMBER
            };
            if r.next() < 0.025 {
                kind = DEAD;
            }
            let s = 0.8 + r.next() * 0.55;
            let yaw = r.next() * TAU;
            out.push(Instance {
                kind,
                transform: place(px, y - 0.15, pz, yaw, s, s, s),
                tint: leaf(px, pz),
            });
        }
        z += cell;
    }
    // a few lone trees in the open
    for _ in 0..26 {
        let px = play.x0 + r.next() * (play.x1 - play.x0);
        let pz = play.z0 + r.next() * (play.z1 - play.z0);
        if field.depth_at(px, pz) > -0.3
            || field.forest_at(px, pz) > 0.2
            || (pz.abs() > 70.0 && px.abs() < 130.0 && r.next() < 0.6)
        {
            continue;
        }
        let kind = if r.next() < 0.5 {
            OAK
        } else if r.next() < 0.5 {
            AMBER
        } else {
            PINE
        };
        let s = 0.9 + r.next() * 0.4;
        let yaw = r.next() * std::f32::consts::TAU;
        out.push(Instance {
            kind,
            transform: place(px, field.height_at(px, pz) - 0.1, pz, yaw, s, s, s),
            tint: leaf(px, pz),
        });
    }
    // rocks: on slopes, at the border, by the river
    let mut rocks = 0;
    for _ in 0..900 {
        if rocks >= 170 {
            break;
        }
        let px = x0 + 20.0 + r.next() * (w - 40.0);
        let pz = z0 + 20.0 + r.next() * (d - 40.0);
        let (gx, gz) = field.grad_at(px, pz);
        let slope = gx.hypot(gz);
        let in_play = px > play.x0 && px < play.x1 && pz > play.z0 && pz < play.z1;
        if field.depth_at(px, pz) > 0.1 {
            continue;
        }
        if slope < 0.18 && r.next() > if in_play { 0.04 } else { 0.25 } {
            continue;
        }
        let s = 0.6 + r.next() * if in_play { 1.3 } else { 2.6 };
        let sy = 0.5 + r.next() * 0.9;
        let yaw = r.next() * std::f32::consts::TAU;
        let l = 0.7 + r.next() * 0.25;
        out.push(Instance {
            kind: ROCK,
            transform: place(px, field.height_at(px, pz) - 0.3, pz, yaw, s, sy, s),
            tint: [l * 1.02, l, l * 1.04],
        });
        rocks += 1;
    }
    // grass tufts, flowers and reeds over the open ground of the field
    let (mut tufts, mut reeds) = (0, 0);
    for _ in 0..14000 {
        if tufts >= 2600 {
            break;
        }
        let px = play.x0 - 20.0 + r.next() * (play.x1 - play.x0 + 40.0);
        let pz = play.z0 - 20.0 + r.next() * (play.z1 - play.z0 + 40.0);
        let dep = field.depth_at(px, pz);
        if dep > -0.15 {
            if dep < 0.25 && reeds < 420 && r.next() < 0.5 {
                let s = 0.8 + r.next() * 0.7;
                let yaw = r.next() * std::f32::consts::TAU;
                out.push(Instance {
                    kind: REED,
                    transform: place(px, field.height_at(px, pz) - 0.05, pz, yaw, s, s, s),
                    tint: [1.0; 3],
                });
                reeds += 1;
            }
            continue;
        }
        if !field.spec.roads.is_empty() && field.road_at(px, pz) > 0.3 {
            continue;
        }
        if dep > -0.6 && reeds < 420 && r.next() < 0.35 {
            let s = 0.7 + r.next() * 0.6;
            let yaw = r.next() * std::f32::consts::TAU;
            out.push(Instance {
                kind: REED,
                transform: place(px, field.height_at(px, pz) - 0.05, pz, yaw, s, s, s),
                tint: [1.0; 3],
            });
            reeds += 1;
            continue;
        }
        let y = field.height_at(px, pz) - 0.04;
        if r.next() < 0.07 {
            let s = 0.8 + r.next() * 0.6;
            let yaw = r.next() * std::f32::consts::TAU;
            let hue = r.next();
            let kind = FLOWER
                + if hue < 0.5 {
                    0
                } else if hue < 0.8 {
                    1
                } else {
                    2
                };
            out.push(Instance {
                kind,
                transform: place(px, y, pz, yaw, s, s, s),
                tint: [1.0; 3],
            });
        } else {
            let s = 0.7 + r.next() * 0.8;
            let yaw = r.next() * std::f32::consts::TAU;
            let hue = (noise(px / 20.0, pz / 20.0, 4) - 0.5) * 0.08;
            let l = 0.75 + r.next() * 0.3;
            out.push(Instance {
                kind: TUFT,
                transform: place(px, y, pz, yaw, s, s, s),
                tint: [l * (1.0 + hue), l, l * (1.0 - hue)],
            });
            tufts += 1;
        }
    }
    // the ford's stepping stones, across the river where it is shallow
    if let (Some(rv), Some(c)) = (
        field.river.as_ref(),
        field.crossings.iter().find(|c| c.ford),
    ) {
        if let Some(p) = rv.line.iter().min_by(|a, b| {
            (a.x - c.x)
                .hypot(a.z - c.z)
                .total_cmp(&(b.x - c.x).hypot(b.z - c.z))
        }) {
            let (tx, tz) = (p.tx, p.tz);
            let (nx, nz) = (-tz, tx);
            for _ in 0..70 {
                let along = (r.next() - 0.5) * c.width * 0.8;
                let across = (r.next() - 0.5) * (rv.w2 * 2.0 + 6.0);
                let (sx, sz) = (
                    c.x + tx * along + nx * across,
                    c.z + tz * along + nz * across,
                );
                if field.depth_at(sx, sz) < 0.05 {
                    continue;
                }
                let s = 0.4 + r.next() * 0.5;
                let yaw = r.next() * std::f32::consts::TAU;
                out.push(Instance {
                    kind: FORD,
                    transform: place(sx, WATER_Y - 0.25, sz, yaw, s, s * 0.4, s),
                    tint: [1.0; 3],
                });
            }
        }
    }
    // craggy rock along every cliff band: tall chunks, smaller ones at the foot, moss on top
    let mut r = Rng::new((seed * 13 + 1) as u32);
    for c in &field.cliffs {
        let l = &c.line;
        let mut k = 0;
        while k < l.len() {
            let p = l[k];
            k += 2;
            let (nx, nz) = (-p.tz, p.tx);
            for _ in 0..2 {
                let off = (r.next() - 0.5) * c.thick * 0.9;
                let (x, z) = (p.x + nx * off, p.z + nz * off);
                let gy = field.height_at(x, z);
                let h = c.height * (0.55 + r.next() * 0.5);
                let sx = 3.0 + r.next() * 3.0;
                let sz = 3.0 + r.next() * 3.0;
                let yaw = r.next() * std::f32::consts::TAU;
                out.push(Instance {
                    kind: CRAG,
                    transform: place(x, gy - h * 0.55, z, yaw, sx, h, sz),
                    tint: [1.0; 3],
                });
                if r.next() < 0.35 {
                    let sx = 2.2 + r.next() * 1.5;
                    let sz = 2.2 + r.next() * 1.5;
                    let yaw = r.next() * std::f32::consts::TAU;
                    out.push(Instance {
                        kind: MOSS,
                        transform: place(x, gy + h * 0.3, z, yaw, sx, 0.6, sz),
                        tint: [1.0; 3],
                    });
                }
            }
            if r.next() < 0.6 {
                let side = if r.next() < 0.5 { 1.0 } else { -1.0 };
                let out_d = c.thick / 2.0 + 2.0 + r.next() * 4.0;
                let (x, z) = (p.x + nx * side * out_d, p.z + nz * side * out_d);
                let s = 1.2 + r.next() * 1.6;
                let sy = 1.0 + r.next() * 1.4;
                let s2 = 1.2 + r.next() * 1.6;
                let yaw = r.next() * std::f32::consts::TAU;
                out.push(Instance {
                    kind: CRAG,
                    transform: place(x, field.height_at(x, z) - 0.6, z, yaw, s, sy, s2),
                    tint: [1.0; 3],
                });
            }
        }
    }
}

/// The field's set pieces (the Blender models), standing on the ground.
fn props(field: &Field) -> (Vec<PropPlace>, Vec<Mat4>) {
    let mut out = Vec::new();
    let mut spinners = Vec::new();
    for p in &field.spec.props {
        let t = Mat4::from_translation(Vec3::new(p.x, field.height_at(p.x, p.z) - 0.15, p.z))
            * Mat4::from_rotation_y(p.rot);
        if p.kind == "windmill" {
            spinners.push(t);
        }
        out.push(PropPlace {
            kind: p.kind,
            transform: t,
        });
    }
    (out, spinners)
}

/// The whole field's scenery.
pub fn build(field: &Field) -> Scenery {
    let mut solid = Builder::default();
    bridge(field, &mut solid);
    let mut wall = Builder::default();
    walls(field, &mut wall);
    let mut instances = Vec::new();
    scatter(field, &mut instances);
    let (props, spinners) = props(field);
    Scenery {
        ground: ground(field, 48),
        water: water(field),
        solids: [solid.build(), wall.build()]
            .into_iter()
            .flatten()
            .collect(),
        kinds: kinds(),
        instances,
        props,
        spinners,
    }
}

/// The minimap's picture of the field: hillshade over the ground colours, water, woods, the
/// field's margins dimmed. `w` × `h` RGBA texels, and the rectangle of the world it shows.
pub fn minimap(field: &Field, w: usize) -> (usize, usize, Vec<u8>, [f32; 4]) {
    let play = field.play;
    let (mx0, mx1, mz0, mz1) = (
        play.x0 - 14.0,
        play.x1 + 14.0,
        play.z0 - 14.0,
        play.z1 + 14.0,
    );
    let h = ((w as f32 * (mz1 - mz0)) / (mx1 - mx0)).round() as usize;
    let mut img = vec![0u8; w * h * 4];
    for py in 0..h {
        for px in 0..w {
            let x = mx0 + ((px as f32 + 0.5) / w as f32) * (mx1 - mx0);
            let z = mz0 + ((py as f32 + 0.5) / h as f32) * (mz1 - mz0);
            let (gx, gz) = field.grad_at(x, z);
            let shade = (0.82 - gx * 1.6 - gz * 1.1).clamp(0.45, 1.25);
            let y = field.height_at(x, z);
            let dry = smooth(3.5, 12.0, y);
            let (mut r, mut g, mut b) = (104.0 + dry * 60.0, 128.0 + dry * 26.0, 66.0 + dry * 14.0);
            let f = field.forest_at(x, z);
            r *= 1.0 - f * 0.55;
            g *= 1.0 - f * 0.4;
            b *= 1.0 - f * 0.45;
            let (mut rr, mut gg, mut bb) = (r * shade, g * shade, b * shade);
            let dep = WATER_Y - y;
            let deck = field.deck_at(x, z).is_some();
            if dep > 0.02 && !deck {
                let k = (dep / 2.0).clamp(0.0, 1.0);
                rr = 110.0 - k * 60.0;
                gg = 170.0 - k * 100.0;
                bb = 190.0 - k * 50.0;
            }
            if deck {
                rr = 160.0;
                gg = 150.0;
                bb = 170.0;
            }
            if field.wall_at(x, z) || field.cliff_at(x, z) {
                rr = 150.0;
                gg = 140.0;
                bb = 156.0;
            }
            if x < play.x0 || x > play.x1 || z < play.z0 || z > play.z1 {
                rr *= 0.55;
                gg *= 0.5;
                bb *= 0.62;
            }
            let i = (py * w + px) * 4;
            img[i] = rr.clamp(0.0, 255.0) as u8;
            img[i + 1] = gg.clamp(0.0, 255.0) as u8;
            img[i + 2] = bb.clamp(0.0, 255.0) as u8;
            img[i + 3] = 255;
        }
    }
    (w, h, img, [mx0, mx1, mz0, mz1])
}

/// A regiment's standard: a pole and a waving cloth in a faction's colours, as a crowd model.
pub fn standard(look: crate::data::Faction) -> velvet_render::CrowdMesh3D {
    use crate::data::Faction;
    use velvet_render::crowd3d::part;
    let (cloth, band, mark) = match look {
        Faction::Reina => (0x6a32c0, 0xf0b44a, 0xf6e2a8),
        Faction::Alianza => (0xece6d4, 0x3d6fd0, 0xe8b040),
        Faction::Rebeldes => (0xa8442a, 0x2e1e12, 0xefe2c8),
        Faction::Libres => (0x6f7f68, 0x3a3226, 0xe8dcc0),
    };
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut add = |shape: Shape, c: [f32; 3], p: f32, pivot: [f32; 3]| {
        let mut b = Builder::default();
        b.add(&shape, c);
        let base = vertices.len() as u32;
        for v in &b.vertices {
            vertices.push(velvet_render::CrowdVertex3D {
                position: v.position,
                part: p,
                normal: v.normal,
                _pad: 0.0,
                color: [c[0], c[1], c[2], 1.0],
                pivot: [pivot[0], pivot[1], pivot[2], 0.0],
            });
        }
        indices.extend(b.indices.iter().map(|i| i + base));
    };
    let pole = rgb(0x4a3020);
    add(
        prims::cylinder(0.035, 0.04, 4.6, 5).translate(0.0, 2.3, 0.0),
        pole,
        part::BODY,
        [0.0; 3],
    );
    add(
        prims::cone(0.08, 0.22, 4).translate(0.0, 4.7, 0.0),
        rgb(0xe8c060),
        part::BODY,
        [0.0; 3],
    );
    add(
        prims::boxed(0.95, 0.06, 0.06).translate(0.44, 4.4, 0.0),
        pole,
        part::BODY,
        [0.0; 3],
    );
    let top = [0.0, 4.38, 0.0];
    add(
        prims::plate(
            &[
                [0.0, 0.0],
                [0.9, 0.0],
                [0.9, -1.25],
                [0.45, -1.0],
                [0.0, -1.25],
            ],
            0.03,
        )
        .translate(0.0, 4.38, 0.0),
        rgb(cloth),
        part::CAPE,
        top,
    );
    add(
        prims::plate(
            &[[0.0, -0.28], [0.9, -0.28], [0.9, -0.44], [0.0, -0.44]],
            0.04,
        )
        .translate(0.0, 4.38, 0.0),
        rgb(band),
        part::CAPE,
        top,
    );
    add(
        prims::plate(
            &[[0.45, -0.52], [0.6, -0.68], [0.45, -0.84], [0.3, -0.68]],
            0.045,
        )
        .translate(0.0, 4.38, 0.0),
        rgb(mark),
        part::CAPE,
        top,
    );
    velvet_render::CrowdMesh3D {
        vertices,
        indices,
        quadruped: false,
    }
}

/// An arrow, a bolt or a javelin, pointing along +z with its head at the origin.
pub fn missile(len: f32, shaft: f32, head: f32, fletch: f32) -> Mesh3D {
    let mut b = Builder::default();
    b.add(
        &prims::boxed(shaft, shaft, len).translate(0.0, 0.0, -len / 2.0),
        rgb(0x6a4a2c),
    );
    b.add(
        &prims::cone(head, head * 3.0, 4)
            .rotate_x(PI / 2.0)
            .translate(0.0, 0.0, head * 1.2),
        rgb(0xd8d8e4),
    );
    if fletch > 0.0 {
        b.add(
            &prims::boxed(0.005, fletch, fletch * 2.2).translate(0.0, 0.0, -len + fletch),
            rgb(0xf0e6d0),
        );
    }
    b.build().expect("a missile")
}

/// Rings of points round the origin in a plane: `rings` as (radius, alpha), `seg` round. In the
/// ground plane (`flat`, facing up) or facing +z; all of `part`, turning round the origin.
fn rings_model(
    rings: &[(f32, f32)],
    seg: usize,
    flat: bool,
    part: f32,
) -> velvet_render::CrowdMesh3D {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let normal = if flat {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let at = |r: f32, a: f32| {
        if flat {
            [r * a.cos(), 0.0, -r * a.sin()]
        } else {
            [r * a.cos(), r * a.sin(), 0.0]
        }
    };
    let first_center = rings[0].0 == 0.0;
    for &(r, alpha) in rings {
        let n = if r == 0.0 { 1 } else { seg };
        for k in 0..n {
            let a = k as f32 / seg as f32 * TAU;
            vertices.push(velvet_render::CrowdVertex3D {
                position: at(r, a),
                part,
                normal,
                _pad: 0.0,
                color: [1.0, 1.0, 1.0, alpha],
                pivot: [0.0; 4],
            });
        }
    }
    let base_of = |ri: usize| -> u32 {
        if first_center {
            if ri == 0 {
                0
            } else {
                1 + ((ri - 1) * seg) as u32
            }
        } else {
            (ri * seg) as u32
        }
    };
    for ri in 0..rings.len() - 1 {
        for k in 0..seg {
            let k1 = (k + 1) % seg;
            if first_center && ri == 0 {
                let o = base_of(1);
                indices.extend([0, o + k as u32, o + k1 as u32]);
            } else {
                let (a, b) = (base_of(ri), base_of(ri + 1));
                let (p0, p1, q0, q1) = (a + k as u32, a + k1 as u32, b + k as u32, b + k1 as u32);
                indices.extend([p0, q0, q1, p0, q1, p1]);
            }
        }
    }
    velvet_render::CrowdMesh3D {
        vertices,
        indices,
        quadruped: false,
    }
}

/// A soft dark blob on the ground (a soldier's shadow), 1 m across.
pub fn blob_model() -> velvet_render::CrowdMesh3D {
    rings_model(
        &[(0.0, 1.0), (0.18, 0.95), (0.35, 0.55), (0.5, 0.0)],
        14,
        true,
        velvet_render::crowd3d::part::BODY,
    )
}

/// A ring on the ground (a chosen soldier, an order's ghost), 1 m across, a faint blob inside.
pub fn ring_model() -> velvet_render::CrowdMesh3D {
    rings_model(
        &[
            (0.0, 0.25),
            (0.22, 0.12),
            (0.30, 0.0),
            (0.33, 0.95),
            (0.42, 0.95),
            (0.495, 0.0),
        ],
        20,
        true,
        velvet_render::crowd3d::part::BODY,
    )
}

/// A soft round speck facing +z (dust, sparks, embers), 1 m across; it pitches as a spear does, so
/// it can be turned to face the camera.
pub fn speck_model() -> velvet_render::CrowdMesh3D {
    rings_model(
        &[(0.0, 1.0), (0.22, 0.55), (0.5, 0.0)],
        10,
        false,
        velvet_render::crowd3d::part::THRUST,
    )
}

/// An arrow, a bolt or a javelin along +z, head at the origin; it pitches as a spear does.
pub fn missile_model(len: f32, shaft: f32, head: f32, fletch: f32) -> velvet_render::CrowdMesh3D {
    let m = missile(len, shaft, head, fletch);
    velvet_render::CrowdMesh3D {
        vertices: m
            .vertices
            .iter()
            .map(|v| velvet_render::CrowdVertex3D {
                position: v.position,
                part: velvet_render::crowd3d::part::THRUST,
                normal: v.normal,
                _pad: 0.0,
                color: v.color,
                pivot: [0.0; 4],
            })
            .collect(),
        indices: m.indices,
        quadruped: false,
    }
}

/// The dusk sky over the field: a dome with a violet zenith, a warm band on the horizon, a soft
/// glow round the sun and faint bands of cloud (its colours in its vertices, linear).
pub fn sky_dome(sun: [f32; 3], top: u32, mid: u32, horizon: u32, ground: u32) -> Mesh3D {
    let (top, mid, hz, gr) = (rgb(top), rgb(mid), rgb(horizon), rgb(ground));
    let s = Vec3::new(sun[0], sun[1], sun[2]).normalize_or_zero();
    let (seg, rings) = (64usize, 32usize);
    let mut vertices = Vec::new();
    for j in 0..=rings {
        let th = j as f32 / rings as f32 * PI;
        for i in 0..=seg {
            let ph = i as f32 / seg as f32 * TAU;
            let d = Vec3::new(th.sin() * ph.sin(), th.cos(), th.sin() * ph.cos());
            let y = d.y;
            let mut c = mix(hz, mid, smooth(0.02, 0.28, y));
            c = mix(c, top, smooth(0.25, 0.85, y));
            c = mix(c, gr, smooth(0.0, -0.12, y));
            let k = d.dot(s).max(0.0);
            let glow = k.powf(24.0) * 0.55 + k.powf(4.0) * 0.12;
            c = [c[0] + glow, c[1] + 0.72 * glow, c[2] + 0.42 * glow];
            let band = smooth(0.03, 0.09, y) * (1.0 - smooth(0.1, 0.22, y));
            let wave = (d.x * 9.0 + d.z * 5.0).sin() * 0.5 + 0.5;
            c = mix(c, scale3(hz, 1.08), band * wave * 0.22);
            let mut v = MeshVertex3D::new([d.x, d.y, d.z], [-d.x, -d.y, -d.z], [0.0, 0.0]);
            v.color = [c[0], c[1], c[2], 1.0];
            vertices.push(v);
        }
    }
    let mut indices = Vec::new();
    let w = (seg + 1) as u32;
    for j in 0..rings as u32 {
        for i in 0..seg as u32 {
            let a = j * w + i;
            let (b, c) = (a + 1, a + w);
            let d = c + 1;
            indices.extend([a, b, c, b, d, c]);
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

/// The Queen's spark: a flat ring of violet and amber fire (2 m across, to be scaled) and a column
/// of light (1 m high, to be scaled), their glow in their vertices.
pub fn spark_meshes() -> (Mesh3D, Mesh3D) {
    let violet = [0.62, 0.34, 1.0];
    let amber = [1.0, 0.68, 0.24];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let seg = 48usize;
    let rings: [(f32, f32, [f32; 3]); 7] = [
        (0.0, 0.25, violet),
        (0.55, 0.12, violet),
        (0.62, 0.8, violet),
        (0.68, 0.1, violet),
        (0.86, 0.9, mix(violet, amber, 0.5)),
        (0.94, 1.0, amber),
        (1.0, 0.0, amber),
    ];
    for &(r, a, c) in &rings {
        for k in 0..=seg {
            let t = k as f32 / seg as f32 * TAU;
            let mut v = MeshVertex3D::new(
                [r * t.cos(), 0.0, -r * t.sin()],
                [0.0, 1.0, 0.0],
                [0.0, 0.0],
            );
            v.color = [c[0], c[1], c[2], a];
            vertices.push(v);
        }
    }
    let w = (seg + 1) as u32;
    for j in 0..(rings.len() - 1) as u32 {
        for k in 0..seg as u32 {
            let (a, b) = (j * w + k, (j + 1) * w + k);
            indices.extend([a, b, b + 1, a, b + 1, a + 1]);
        }
    }
    let ring = Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 1.0,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    };
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let (seg, rows) = (24usize, 6usize);
    for j in 0..=rows {
        let y = j as f32 / rows as f32;
        let r = 1.4 + (1.0 - 1.4) * y;
        let c = mix([1.0, 0.8, 0.45], [0.75, 0.5, 1.0], y);
        for k in 0..=seg {
            let t = k as f32 / seg as f32 * TAU;
            let mut v = MeshVertex3D::new(
                [r * t.cos(), y, r * t.sin()],
                [t.cos(), 0.0, t.sin()],
                [0.0, 0.0],
            );
            v.color = [c[0], c[1], c[2], (1.0 - y).powf(1.4)];
            vertices.push(v);
        }
    }
    let w = (seg + 1) as u32;
    for j in 0..rows as u32 {
        for k in 0..seg as u32 {
            let (a, b) = (j * w + k, (j + 1) * w + k);
            indices.extend([a, b, b + 1, a, b + 1, a + 1, a, b + 1, b, a, a + 1, b + 1]);
        }
    }
    let column = Mesh3D {
        vertices,
        indices,
        material: Material3D::default(),
        metallic_factor: 0.0,
        roughness_factor: 1.0,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    };
    (ring, column)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maps::map_spec;

    #[test]
    fn every_map_builds_its_scenery() {
        for id in [
            "vado", "atalaya", "bosque", "penascal", "puerta", "cienaga", "trigales",
        ] {
            let field = Field::build(map_spec(id, None));
            let s = build(&field);
            let tris: usize = s.ground.iter().map(|m| m.indices.len() / 3).sum();
            assert!(tris > 10_000, "{id}: {tris} ground triangles");
            assert!(!s.instances.is_empty(), "{id}: no scatter");
            for m in &s.ground {
                assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
            }
            let (w, h, img, _) = minimap(&field, 120);
            assert_eq!(img.len(), w * h * 4);
            eprintln!(
                "{id}: {} ground tiles, {tris} tris, water {}, {} scatter, {} props",
                s.ground.len(),
                s.water.as_ref().map_or(0, |m| m.indices.len() / 3),
                s.instances.len(),
                s.props.len()
            );
        }
    }

    #[test]
    fn standards_and_missiles_build() {
        let s = standard(crate::data::Faction::Reina);
        assert!(s
            .vertices
            .iter()
            .any(|v| v.part == velvet_render::crowd3d::part::CAPE));
        assert!(!missile(0.85, 0.025, 0.035, 0.07).indices.is_empty());
    }
}
