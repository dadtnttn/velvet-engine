//! Art-only rock mesher. Jittered anisotropic Voronoi cells make broad hewn
//! planes, chipped bevels and recessed seams, not noise-displaced triangle soup.
//! Each panel is watertight behind its relief, and ALL relief goes outward from
//! the validated walkable volume. Does not implement mining/destruction.
use super::visual::{quad, tint, tri};
use std::{collections::BTreeMap, sync::OnceLock};
use velvet_dungeon::{rng::Rng, Biome, Room, Shape};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

#[derive(Clone, Copy, serde::Deserialize)]
pub struct Style {
    pub stone: [f32; 3],
    pub floor: [f32; 3],
    pub moss: [f32; 3],
    pub moss_coverage: f32,
}

/// Render-only mood of a biome: light colours, atmosphere and its signature
/// room beacon. Never read by generation, collision, saves or rules hashing.
#[derive(Clone, Copy, serde::Deserialize)]
pub struct Mood {
    /// Wall torch and brazier flame colour (also their light colour).
    pub flame: [f32; 3],
    pub flame_intensity: f32,
    /// Colour of the biome's signature glow (crystals, embers, pit light).
    pub glow: [f32; 3],
    pub ambient: [f32; 3],
    pub fog: [f32; 3],
    pub beacon: Beacon,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Beacon {
    Brazier,
    Crystal,
    Embers,
}

#[derive(Clone, Copy, serde::Deserialize)]
struct Entry {
    #[serde(flatten)]
    style: Style,
    mood: Option<Mood>,
}

fn entries() -> &'static BTreeMap<String, Entry> {
    static STYLES: OnceLock<BTreeMap<String, Entry>> = OnceLock::new();
    STYLES.get_or_init(|| {
        serde_json::from_str(include_str!("surface_styles.json")).expect("surface art styles")
    })
}

pub fn style(biome: &Biome) -> Style {
    entries()
        .get(&biome.id)
        .map(|entry| entry.style)
        .unwrap_or(Style {
            stone: biome.wall,
            floor: biome.floor,
            moss: [0.18, 0.24, 0.12],
            moss_coverage: 0.15,
        })
}

pub fn mood(biome: &Biome) -> Mood {
    entries()
        .get(&biome.id)
        .and_then(|entry| entry.mood)
        .unwrap_or(Mood {
            flame: [1.0, 0.56, 0.18],
            flame_intensity: 1.0,
            glow: biome.accent,
            ambient: biome.ambient,
            fog: biome.fog,
            beacon: Beacon::Brazier,
        })
}
pub fn styled_biome(b: &Biome) -> Biome {
    let mut b = b.clone();
    let s = style(&b);
    b.wall = s.stone;
    b.floor = s.floor;
    b
}

type P = [f32; 2];
fn dot(a: P, b: P) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn mix(a: P, b: P, t: f32) -> P {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}
fn clip(poly: &[P], normal: P, limit: f32) -> Vec<P> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let da = dot(a, normal) - limit;
        let db = dot(b, normal) - limit;
        if da <= 0.0 {
            out.push(a);
        }
        if (da < 0.0) != (db < 0.0) {
            out.push(mix(a, b, da / (da - db)));
        }
    }
    out
}
fn bounds(poly: &[P]) -> (P, P) {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for p in poly {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}
/// Convex panel in (u,v). Basis vectors are unit-length; n faces the player.
/// A recessed shell with perimeter returns seals corners/floor/socket seams.
fn panel(
    m: &mut Mesh3D,
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    n: Vec3,
    poly: &[P],
    cell: P,
    relief: f32,
    style: Style,
    seed: u64,
    masonry: bool,
) {
    if poly.len() < 3 {
        return;
    }
    let (lo, hi) = bounds(poly);
    // Natural paving uses pairwise Voronoi clipping. Bound the work on large
    // arenas so a small nominal tile size cannot cause a generation hitch.
    let span = [(hi[0] - lo[0]).max(0.0), (hi[1] - lo[1]).max(0.0)];
    let estimated = span[0] * span[1] / (cell[0] * cell[1]).max(0.0001);
    let scale = (estimated / 480.0).sqrt().max(1.0);
    let cell = [cell[0] * scale, cell[1] * scale];
    let mut rng = Rng::stream(seed, "hewn_surface_v1", 0);
    let nx = ((hi[0] - lo[0]) / cell[0]).ceil().max(1.0) as usize;
    let ny = ((hi[1] - lo[1]) / cell[1]).ceil().max(1.0) as usize;
    let mut sites = Vec::new();
    for y in 0..ny {
        for x in 0..nx {
            let jitter = if masonry { 0.12 } else { 0.29 };
            if !masonry && nx > 2 && ny > 1 && rng.chance(0.12) {
                continue;
            }
            let stagger = if y % 2 == 1 { 0.25 } else { 0.0 };
            sites.push([
                (x as f32 + 0.5 + stagger + rng.range(-jitter, jitter)) * cell[0] + lo[0],
                (y as f32 + 0.5 + rng.range(-jitter, jitter)) * cell[1] + lo[1],
            ]);
        }
    }
    // Keep an entire front face coplanar. Tapering each vertex independently
    // toward the panel edge made the outer blocks look like folded envelopes.
    let point = |p: P, depth: f32| origin + u * p[0] + v * p[1] - n * depth;
    // Backing MUST stay at the full recess at every point. A fan tapering from
    // its deep centre to front boundary would intersect the sculpted faces and
    // expose triangular slices, especially near doors and on sloping tunnels.
    let center = mix(lo, hi, 0.5);
    let backing = |p: P| origin + u * p[0] + v * p[1] - n * relief;
    for i in 0..poly.len() {
        tri(
            m,
            backing(center),
            backing(poly[i]),
            backing(poly[(i + 1) % poly.len()]),
            n,
            tint(style.stone, 0.30),
        );
    }
    // Perimeter returns seal the recess without drawing a diagonal sheet in
    // front of it. Neighbouring panels share the exact original boundary.
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let outward = (u * (a[0] - center[0]) + v * (a[1] - center[1])).normalize_or_zero();
        quad(
            m,
            [point(a, 0.0), backing(a), backing(b), point(b, 0.0)],
            -outward,
            tint(style.stone, 0.72),
        );
    }
    let mut cells = Vec::new();
    // Coursed, staggered masonry has an architectural rhythm. Voronoi alone
    // produced enormous triangular shards even in built rooms.
    if masonry {
        let row_h = (hi[1] - lo[1]) / ny as f32;
        for row in 0..ny {
            let y0 = lo[1] + row as f32 * row_h;
            let y1 = y0 + row_h;
            let mut x0 = lo[0] - if row % 2 == 1 { cell[0] * 0.5 } else { 0.0 };
            while x0 < hi[0] {
                let x1 = x0 + cell[0] * rng.range(0.78, 1.28);
                let mut block = poly.to_vec();
                for (normal, limit) in [
                    ([-1.0, 0.0], -x0),
                    ([1.0, 0.0], x1),
                    ([0.0, -1.0], -y0),
                    ([0.0, 1.0], y1),
                ] {
                    block = clip(&block, normal, limit);
                }
                if block.len() >= 3 {
                    cells.push(block);
                }
                x0 = x1;
            }
        }
    } else {
        for (id, &site) in sites.iter().enumerate() {
            let mut cell_poly = poly.to_vec();
            for (j, &other) in sites.iter().enumerate() {
                if id == j {
                    continue;
                }
                // Anisotropic metric gives sedimentary plates instead of pebbles.
                let a = [site[0] / cell[0], site[1] / cell[1]];
                let b = [other[0] / cell[0], other[1] / cell[1]];
                let ab = sub(b, a);
                cell_poly = clip(
                    &cell_poly,
                    [ab[0] / cell[0], ab[1] / cell[1]],
                    (dot(b, b) - dot(a, a)) * 0.5,
                );
                if cell_poly.len() < 3 {
                    break;
                }
            }
            if cell_poly.len() >= 3 {
                cells.push(cell_poly);
            }
        }
    }
    for mut cell_poly in cells {
        let mid = cell_poly.iter().fold([0.0; 2], |a, p| {
            [
                a[0] + p[0] / cell_poly.len() as f32,
                a[1] + p[1] / cell_poly.len() as f32,
            ]
        });
        let (low, high) = bounds(&cell_poly);
        if (high[0] - low[0]) * (high[1] - low[1]) < 0.018 {
            continue;
        }
        // Clip small uneven chips from corners; front faces remain broad and
        // calm. Actual normals, not painted triangular noise, light the edges.
        let corners = cell_poly.clone();
        for p in corners.into_iter().take(2) {
            let radial = sub(p, mid);
            let length = dot(radial, radial).sqrt().max(0.001);
            let cut = [radial[0] / length, radial[1] / length];
            let chip = if n.y > 0.65 {
                rng.range(0.006, 0.022)
            } else {
                rng.range(0.014, 0.05)
            };
            cell_poly = clip(&cell_poly, cut, dot(p, cut) - chip);
        }
        if cell_poly.len() < 3 {
            continue;
        }
        let mut color = tint(style.stone, rng.range(0.79, 1.13));
        // Broad mineral pigments, not an identical flat brown on every block.
        let mineral = rng.range(-0.09, 0.08);
        color[0] *= 1.0 + mineral;
        color[2] *= 1.0 - mineral * 1.5;
        let face_depth = relief * rng.range(0.44, 0.60);
        let tilt = [rng.range(-0.022, 0.022), rng.range(-0.023, 0.023)];
        // A minimum recess receives the shallow authored veneer without slicing
        // through its front. No gameplay bounds need to change for this art.
        let depth =
            |p: P| (face_depth + dot(sub(p, mid), tilt)).clamp(relief * 0.30, relief * 0.75);
        let face_mid = point(mid, depth(mid));
        let bevel = if n.y > 0.65 {
            rng.range(0.969, 0.984)
        } else {
            rng.range(0.87, 0.94)
        };
        let moss = rng.chance(style.moss_coverage);
        for i in 0..cell_poly.len() {
            let a = cell_poly[i];
            let b = cell_poly[(i + 1) % cell_poly.len()];
            let fa = mix(mid, a, bevel);
            let fb = mix(mid, b, bevel);
            let pa = point(fa, depth(fa));
            let pb = point(fb, depth(fb));
            let upper = (a[1] + b[1]) * 0.5 > mid[1] + 0.07;
            let bevel_color = if moss && (upper || n.y > 0.65) && n.y > -0.5 {
                style.moss
            } else {
                tint(color, if upper { 1.19 } else { 0.62 })
            };
            // Tiny seam, broad lit bevel and a single uninterrupted face.
            quad(
                m,
                [
                    point(mix(mid, a, 0.993), relief),
                    point(mix(mid, b, 0.993), relief),
                    pb,
                    pa,
                ],
                n,
                bevel_color,
            );
            let start = m.vertices.len();
            tri(m, face_mid, pa, pb, n, color);
            if m.vertices.len() == start + 3 {
                // Vertex-painted cavity gradient adds shaped light on broad
                // faces without a full-screen AO pass or noisy normal maps.
                for (j, strength) in [
                    1.07,
                    if upper { 1.02 } else { 0.83 },
                    if upper { 1.02 } else { 0.83 },
                ]
                .iter()
                .enumerate()
                {
                    for k in 0..3 {
                        m.vertices[start + j].color[k] *= strength;
                    }
                }
            }
            // Hand-painted-looking moss lip, fading into stone on the broad
            // face. Geometry follows the face and sits only 3 mm above it.
            if moss && upper && n.y.abs() < 0.5 {
                let tip = mix(mix(fa, fb, 0.5), mid, rng.range(0.12, 0.32));
                let start = m.vertices.len();
                tri(
                    m,
                    point(fa, depth(fa) - 0.003),
                    point(fb, depth(fb) - 0.003),
                    point(tip, depth(tip) - 0.003),
                    n,
                    style.moss,
                );
                if m.vertices.len() > start {
                    m.vertices.last_mut().unwrap().color = [color[0], color[1], color[2], 1.0];
                }
            }
        }
        // Sparse fractured seams, not a repeating high-frequency noise map.
        if rng.chance(if masonry { 0.24 } else { 0.34 }) {
            let edge = cell_poly[rng.index(cell_poly.len())];
            let a = mix(mid, edge, 0.24);
            let b = mix(mid, edge, 0.64);
            let e = sub(b, a);
            let len = dot(e, e).sqrt().max(0.001);
            let side = [-e[1] / len * 0.011, e[0] / len * 0.011];
            let offset = |p: P, s: f32| [p[0] + side[0] * s, p[1] + side[1] * s];
            quad(
                m,
                [
                    point(offset(a, -1.0), depth(a) - 0.004),
                    point(offset(b, -0.2), depth(b) - 0.004),
                    point(offset(b, 0.2), depth(b) - 0.004),
                    point(offset(a, 1.0), depth(a) - 0.004),
                ],
                n,
                tint(color, 0.58),
            );
        }
    }
}

pub fn walls(m: &mut Mesh3D, r: &Room, biome: &Biome, seed: u64) {
    walls_with_finish(m, r, biome, seed, None);
}

pub fn walls_with_finish(
    m: &mut Mesh3D,
    r: &Room,
    biome: &Biome,
    seed: u64,
    finish: Option<&super::themes::Finish>,
) {
    let mut s = style(biome);
    if let Some(f) = finish {
        s.stone = f.stone;
        s.moss_coverage = f.wear.moss;
    }
    let fine = super::architecture::ornate(r).is_some();
    let masonry = finish
        .map(|f| f.masonry)
        .unwrap_or(fine || r.shape == Shape::Vault || r.biome == 0);
    let perimeter: f32 = r
        .outline
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let b = r.outline[(i + 1) % r.outline.len()];
            ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
        })
        .sum();
    // Signature rooms use smaller, finely jointed ashlar courses.
    let cell = if let Some(f) = finish {
        f.wall_cell
    } else if fine {
        [0.86, 0.43]
    } else if masonry {
        [1.34, 0.81]
    } else {
        [1.75, 1.08]
    };
    let area = perimeter * (r.height + if r.shape == Shape::Bridge { 4.0 } else { 0.0 });
    // Keep huge arenas on the same bounded geometry budget as ordinary rooms.
    let budget = if fine { 620.0 } else { 230.0 };
    let budget_scale = (area / (cell[0] * cell[1] * budget)).sqrt().max(1.0);
    for i in 0..r.outline.len() {
        let p = r.outline[i];
        let q = r.outline[(i + 1) % r.outline.len()];
        let origin = r.center + Vec3::new(p[0], 0.0, p[1]);
        let edge = Vec3::new(q[0] - p[0], 0.0, q[1] - p[1]);
        let length = edge.length();
        let u = edge.normalize_or_zero();
        let n = Vec3::new(-u.z, 0.0, u.x);
        let bottom = if r.shape == Shape::Bridge { -4.0 } else { 0.0 };
        let socket = r.sockets.iter().find(|s| {
            (s.position - origin).dot(n).abs() < 0.01
                && (s.position - origin).dot(u) >= 0.0
                && (s.position - origin).dot(u) <= length
        });
        let regions = if let Some(socket) = socket {
            let at = (socket.position - origin).dot(u);
            vec![
                (0.0, at - socket.width * 0.5, bottom, r.height),
                (at + socket.width * 0.5, length, bottom, r.height),
                (
                    at - socket.width * 0.5,
                    at + socket.width * 0.5,
                    socket.height,
                    r.height,
                ),
            ]
        } else {
            vec![(0.0, length, bottom, r.height)]
        };
        for (j, (x0, x1, y0, y1)) in regions.into_iter().enumerate() {
            if x1 - x0 < 0.01 || y1 - y0 < 0.01 {
                continue;
            }
            panel(
                m,
                origin,
                u,
                Vec3::Y,
                n,
                &[[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                [cell[0] * budget_scale, cell[1] * budget_scale],
                if let Some(f) = finish {
                    f.wall_relief
                } else if fine {
                    0.22
                } else if masonry {
                    0.30
                } else {
                    0.43
                },
                s,
                seed.wrapping_add((i * 13 + j) as u64),
                masonry,
            );
        }
    }
}

pub fn ceiling(m: &mut Mesh3D, r: &Room, biome: &Biome, seed: u64) {
    ceiling_with_finish(m, r, biome, seed, None);
}

pub fn ceiling_with_finish(
    m: &mut Mesh3D,
    r: &Room,
    biome: &Biome,
    seed: u64,
    finish: Option<&super::themes::Finish>,
) {
    let mut s = style(biome);
    if let Some(f) = finish {
        s.stone = if f.timber_ceiling { f.wood } else { f.stone };
    }
    s.stone = tint(s.stone, 0.86);
    s.moss_coverage = 0.0;
    panel(
        m,
        r.center + Vec3::Y * r.height,
        Vec3::X,
        Vec3::Z,
        -Vec3::Y,
        &r.outline,
        finish.map(|f| f.ceiling_cell).unwrap_or([2.25, 1.8]),
        finish.map(|f| f.ceiling_relief).unwrap_or(0.68),
        s,
        seed,
        finish.is_some_and(|f| f.timber_ceiling),
    );
}

/// Tunnel panels use the same stone language; boundary returns seal the exact
/// cross-section at their ends. Stairs remain the exact collision treads.
pub fn tunnel_panel(m: &mut Mesh3D, p: [Vec3; 4], normal: Vec3, biome: &Biome, seed: u64) {
    let width = (p[1] - p[0]).length();
    let length = (p[3] - p[0]).length();
    if width < 0.01 || length < 0.01 {
        return;
    }
    let mut s = style(biome);
    s.stone = biome.wall;
    panel(
        m,
        p[0],
        (p[1] - p[0]).normalize_or_zero(),
        (p[3] - p[0]).normalize_or_zero(),
        normal,
        &[[0.0, 0.0], [width, 0.0], [width, length], [0.0, length]],
        [0.92, 1.25],
        0.28,
        s,
        seed,
        false,
    );
}

/// Shallow recessed flagstones: the navigation surface stays unchanged and
/// every slab/joint remains at or below it. Used on rooms, bridges and ramps.
pub fn flagstones(
    m: &mut Mesh3D,
    origin: Vec3,
    u: Vec3,
    v: Vec3,
    normal: Vec3,
    outline: &[[f32; 2]],
    color: [f32; 3],
    seed: u64,
) {
    panel(
        m,
        origin,
        u,
        v,
        normal,
        outline,
        [1.35, 1.12],
        0.055,
        Style {
            stone: color,
            floor: color,
            moss: [0.07, 0.12, 0.025],
            moss_coverage: 0.22,
        },
        seed,
        false,
    );
}

/// Inset a convex polygon by `d` (same vertex count), in its own plane.
fn inset(poly: &[P], d: f32) -> Vec<P> {
    let n = poly.len();
    let centroid = poly.iter().fold([0.0; 2], |a, p| {
        [a[0] + p[0] / n as f32, a[1] + p[1] / n as f32]
    });
    let lines: Vec<(P, P)> = (0..n)
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % n];
            let e = sub(b, a);
            let len = dot(e, e).sqrt().max(1.0e-4);
            let dir = [e[0] / len, e[1] / len];
            let mut normal = [-dir[1], dir[0]];
            if dot(sub(centroid, a), normal) < 0.0 {
                normal = [dir[1], -dir[0]];
            }
            ([a[0] + normal[0] * d, a[1] + normal[1] * d], dir)
        })
        .collect();
    (0..n)
        .map(|i| {
            let (p0, d0) = lines[(i + n - 1) % n];
            let (p1, d1) = lines[i];
            let denom = d0[0] * d1[1] - d0[1] * d1[0];
            if denom.abs() < 1.0e-5 {
                return p1;
            }
            let w = sub(p1, p0);
            let t = (w[0] * d1[1] - w[1] * d1[0]) / denom;
            [p0[0] + d0[0] * t, p0[1] + d0[1] * t]
        })
        .collect()
}

/// Ashlar paving for signature rooms: running-bond slabs aligned with the
/// room's processional axis inside a kerb band of long stones. Like
/// `flagstones`, every slab and joint stays at or below the walking plane.
pub fn ashlar_floor(m: &mut Mesh3D, r: &Room, axis: Vec3, color: [f32; 3], seed: u64) {
    let u = Vec3::new(axis.x, 0.0, axis.z).normalize_or_zero();
    let u = if u.length_squared() < 0.5 { Vec3::X } else { u };
    let v = Vec3::new(-u.z, 0.0, u.x);
    let outline: Vec<P> = r
        .outline
        .iter()
        .map(|p| {
            let w = Vec3::new(p[0], 0.0, p[1]);
            [w.dot(u), w.dot(v)]
        })
        .collect();
    let band = 0.58;
    let inner = inset(&outline, band);
    let origin = r.center;
    let paving = Style {
        stone: color,
        floor: color,
        moss: [0.07, 0.12, 0.025],
        moss_coverage: 0.03,
    };
    panel(
        m,
        origin,
        u,
        v,
        Vec3::Y,
        &inner,
        [1.0, 0.62],
        0.05,
        paving,
        seed,
        true,
    );
    let kerb = Style {
        stone: tint(color, 0.78),
        ..paving
    };
    let n = outline.len();
    for i in 0..n {
        let (a, b) = (outline[i], outline[(i + 1) % n]);
        let (ai, bi) = (inner[i], inner[(i + 1) % n]);
        let e = sub(b, a);
        let len = dot(e, e).sqrt();
        if len < 0.05 {
            continue;
        }
        let su = [e[0] / len, e[1] / len];
        let mut sv = [-su[1], su[0]];
        if dot(sub(ai, a), sv) < 0.0 {
            sv = [su[1], -su[0]];
        }
        let local = |p: P| {
            let q = sub(p, a);
            [dot(q, su), dot(q, sv)]
        };
        let strip = [local(a), local(b), local(bi), local(ai)];
        let world = |p: P| u * p[0] + v * p[1];
        let o = origin + world(a);
        panel(
            m,
            o,
            world(su),
            world(sv),
            Vec3::Y,
            &strip,
            [0.9, band + 0.02],
            0.05,
            kerb,
            seed.wrapping_add(97 + i as u64),
            true,
        );
    }
}

/// Low-poly vegetation and small rubble are baked into each room's one static
/// batch. Hanging foliage is confined to the ceiling/wall perimeter, above
/// jump headroom; no new colliders, transparent sorting or per-leaf draws.
pub fn age_dressing(m: &mut Mesh3D, r: &Room, biome: &Biome, seed: u64) {
    let s = style(biome);
    let mut rng = Rng::stream(seed, "masonry_age_v1", 0);
    for i in 0..r.outline.len() {
        let a = r.outline[i];
        let b = r.outline[(i + 1) % r.outline.len()];
        let edge = Vec3::new(b[0] - a[0], 0.0, b[1] - a[1]);
        let u = edge.normalize_or_zero();
        let inward = Vec3::new(-u.z, 0.0, u.x);
        for j in 0..(edge.length() / 1.4).ceil() as usize {
            let t = (j as f32 + rng.range(0.1, 0.8)) * 1.4 / edge.length();
            if t > 0.97 {
                continue;
            }
            let p = r.center + Vec3::new(a[0], 0.0, a[1]) + edge * t + inward * 0.14;
            if r.sockets
                .iter()
                .any(|s| (s.position - p).length() < s.width * 0.5 + 0.65)
            {
                continue;
            }
            if rng.chance(0.65) {
                let floor = Vec3::new(p.x, r.floor_y(p.x, p.z), p.z);
                for _ in 0..3 {
                    let q = floor + u * rng.range(-0.3, 0.3) + inward * rng.range(0.0, 0.22);
                    let size = rng.range(0.045, 0.16);
                    super::visual::rock(
                        m,
                        q,
                        Vec3::new(size * 1.4, size, size),
                        tint(s.stone, rng.range(0.52, 0.92)),
                        rng.range(0.0, 6.28),
                    );
                }
            }
            if !rng.chance(s.moss_coverage * 1.55) {
                continue;
            }
            // The roof itself is recessed above the collision plane. Embed the
            // roots in its relief instead of leaving moss floating below it.
            let root = p + Vec3::Y * (r.height + 0.50);
            super::visual::rock(
                m,
                root - Vec3::Y * 0.075,
                Vec3::new(0.29, 0.13, 0.22),
                tint(s.moss, 1.25),
                rng.range(0.0, 6.28),
            );
            for strand in 0..rng.index(3) + 3 {
                let length = rng
                    .range(0.72, 1.24)
                    .min(((r.height + 0.50 - 2.95) / 1.07).max(0.05));
                let root = root + u * (strand as f32 * 0.09 - 0.18);
                for k in 0..5 {
                    let f = k as f32 / 5.0;
                    let at = root - Vec3::Y * (f * length) + inward * ((f * 6.0).sin() * 0.06);
                    let width = (1.0 - f) * rng.range(0.045, 0.1);
                    let tip = at - Vec3::Y * (length * 0.27) + u * rng.range(-0.04, 0.04);
                    let ridge = at + inward * 0.035;
                    tri(m, at - u * width, ridge, tip, inward, tint(s.moss, 0.72));
                    tri(m, ridge, at + u * width, tip, inward, tint(s.moss, 1.3));
                    tri(m, tip, at + u * width, at - u * width, -inward, s.moss);
                }
            }
        }
    }
}

/// Normalized two-sided secret slab. Generated once per biome, shared by all
/// closed secret passages; opening a secret simply stops submitting the slab.
pub fn secret_wall(biome: &Biome) -> Mesh3D {
    let mut m = super::visual::empty();
    let s = style(biome);
    let poly = [[-0.5, 0.0], [0.5, 0.0], [0.5, 1.0], [-0.5, 1.0]];
    for sign in [-1.0, 1.0] {
        panel(
            &mut m,
            Vec3::Z * (sign * 0.08),
            Vec3::X,
            Vec3::Y,
            Vec3::Z * sign,
            &poly,
            [0.42, 0.3],
            0.06,
            s,
            91,
            false,
        );
    }
    m
}

#[cfg(test)]
mod tests {
    use super::super::visual;
    use super::*;
    use velvet_dungeon::{gameplay::Difficulty, generate, Catalog, Config};

    fn valid_mesh(m: &Mesh3D) {
        assert!(!m.indices.is_empty());
        assert_eq!(m.indices.len() % 3, 0);
        assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
        for v in &m.vertices {
            assert!(v
                .position
                .iter()
                .chain(v.normal.iter())
                .chain(v.color.iter())
                .all(|f| f.is_finite()));
            let n = Vec3::new(v.normal[0], v.normal[1], v.normal[2]);
            assert!((n.length() - 1.0).abs() < 0.001, "invalid normal {n:?}");
        }
    }
    fn fingerprint(m: &Mesh3D) -> u64 {
        let mut bytes = Vec::new();
        for v in &m.vertices {
            for f in v
                .position
                .iter()
                .chain(v.normal.iter())
                .chain(v.color.iter())
            {
                bytes.extend(f.to_bits().to_le_bytes());
            }
        }
        for i in &m.indices {
            bytes.extend(i.to_le_bytes());
        }
        velvet_dungeon::rng::hash(&bytes)
    }
    #[test]
    fn surfaces_repeat_without_changing_navigation_or_blocking_sockets() {
        let catalog = Catalog::bundled();
        for seed in [0, 1, 7, 42, 99, 4815, 8675309, u64::MAX] {
            let map = generate(seed, &Config::default(), &catalog, &Difficulty::default()).unwrap();
            let before = serde_json::to_vec(&map).unwrap();
            for r in &map.rooms {
                let biome = &catalog.biomes[r.biome];
                let mut wall = visual::empty();
                walls(&mut wall, r, biome, r.landmark_seed);
                valid_mesh(&wall);
                // No procedural rock can intrude into the validated capsule area.
                for v in &wall.vertices {
                    assert!(
                        !r.contains(v.position[0], v.position[2], 0.002),
                        "wall intrudes: seed {seed} room {}",
                        r.id
                    );
                }
                for s in &r.sockets {
                    let tangent = Vec3::new(-s.outward.z, 0.0, s.outward.x);
                    for t in wall.indices.chunks_exact(3) {
                        let p = t.iter().fold(Vec3::ZERO, |p, &i| {
                            let v = wall.vertices[i as usize].position;
                            p + Vec3::new(v[0], v[1], v[2])
                        }) * (1.0 / 3.0)
                            - s.position;
                        let depth = p.dot(s.outward);
                        let along = p.dot(tangent);
                        assert!(
                            !(depth > -0.002
                                && depth < 0.8
                                && along.abs() < s.width * 0.5 - 0.002
                                && p.y > 0.0
                                && p.y < s.height - 0.002),
                            "socket occluded: seed {seed} room {}",
                            r.id
                        );
                    }
                }
                let mut roof = visual::empty();
                ceiling(&mut roof, r, biome, r.landmark_seed);
                valid_mesh(&roof);
                for v in &roof.vertices {
                    assert!(v.position[1] >= r.center.y + r.height - 0.001);
                }
                let mut again = visual::empty();
                walls(&mut again, r, biome, r.landmark_seed);
                assert_eq!(fingerprint(&wall), fingerprint(&again));
                // Signature rooms trade a larger static budget for fine ashlar.
                let budget = if super::super::architecture::ornate(r).is_some() {
                    28000
                } else {
                    12000
                };
                assert!(
                    wall.indices.len() / 3 < budget,
                    "room geometry budget: {}",
                    wall.indices.len() / 3
                );
                assert!(roof.indices.len() / 3 < 6000, "roof geometry budget");
            }
            for c in &map.connections {
                let m = visual::corridor(&map, &catalog, c.id);
                valid_mesh(&m);
                assert!(m.indices.len() / 3 < 20000);
            }
            assert_eq!(
                before,
                serde_json::to_vec(&map).unwrap(),
                "art changed logical map"
            );
        }
    }
    #[test]
    fn tunnel_relief_stays_outside_flat_and_sloping_panels() {
        let catalog = Catalog::bundled();
        for slope in [0.0, 1.7, -1.7] {
            let p = [
                Vec3::ZERO,
                Vec3::Y * 3.4,
                Vec3::new(0.0, 3.4 + slope, 5.0),
                Vec3::new(0.0, slope, 5.0),
            ];
            let mut m = visual::empty();
            tunnel_panel(&mut m, p, Vec3::X, &catalog.biomes[0], 42);
            valid_mesh(&m);
            for v in &m.vertices {
                assert!(v.position[0] <= 0.0001);
            }
            // Exact boundary corners seal adjoining panels: no crack at ends.
            for corner in p {
                assert!(m.vertices.iter().any(|v| (Vec3::new(
                    v.position[0],
                    v.position[1],
                    v.position[2]
                ) - corner)
                    .length()
                    < 0.0001));
            }
        }
    }
    #[test]
    fn backing_is_behind_every_stone_face_not_a_tapered_occluder() {
        let mut m = visual::empty();
        let catalog = Catalog::bundled();
        panel(
            &mut m,
            Vec3::ZERO,
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            &[[0.0, 0.0], [8.0, 0.0], [8.0, 6.0], [0.0, 6.0]],
            [2.25, 1.2],
            0.46,
            style(&catalog.biomes[0]),
            42,
            false,
        );
        for v in &m.vertices[..12] {
            assert!((v.position[2] + 0.46).abs() < 0.0001);
        }
        valid_mesh(&m);
        // Returns must face the interior. Back-facing returns disappeared
        // under GPU culling and exposed blue strips between wall and ceiling.
        for v in &m.vertices[12..36] {
            let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
            let n = Vec3::new(v.normal[0], v.normal[1], v.normal[2]);
            assert!(n.dot(Vec3::new(4.0, 3.0, 0.0) - p) > 0.0);
        }
    }

    #[test]
    fn flagstones_remain_below_walkable_planes_and_repeat() {
        for rise in [-0.4f32, 0.0, 0.4] {
            let origin = Vec3::new(7.0, 6.0, -11.0);
            let u = Vec3::X;
            let v = Vec3::new(0.0, rise, 1.0).normalize_or_zero();
            let n = v.cross(u).normalize_or_zero();
            let outline = [[-4.0, 0.0], [4.0, 0.0], [4.0, 6.0], [-4.0, 6.0]];
            let make = || {
                let mut m = visual::empty();
                flagstones(&mut m, origin, u, v, n, &outline, [0.3; 3], 42);
                m
            };
            let m = make();
            valid_mesh(&m);
            assert_eq!(fingerprint(&m), fingerprint(&make()));
            assert!(m.indices.len() / 3 < 2800);
            for vertex in &m.vertices {
                let p = Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2]);
                let distance = (p - origin).dot(n);
                assert!(
                    (-0.056..=0.001).contains(&distance),
                    "slab intrudes into ramp: {distance}"
                );
            }
        }
    }

    #[test]
    fn foliage_and_rubble_do_not_occupy_the_player_height_band() {
        let catalog = Catalog::bundled();
        let map = generate(42, &Config::default(), &catalog, &Difficulty::default()).unwrap();
        for r in &map.rooms {
            let mut m = visual::empty();
            age_dressing(&mut m, r, &catalog.biomes[r.biome], r.landmark_seed);
            if m.indices.is_empty() {
                continue;
            }
            valid_mesh(&m);
            for vertex in &m.vertices {
                let p = vertex.position;
                let above_floor = p[1] - r.floor_y(p[0], p[2]);
                assert!(
                    above_floor <= 0.17 || above_floor >= 2.80,
                    "decoration at capsule height: {above_floor}"
                );
            }
        }
        valid_mesh(&visual::hand_torch());
    }
}
