//! Art-only rock mesher (web prototype `surfaces.js`): jittered anisotropic
//! Voronoi cells or coursed masonry make broad hewn planes, chipped bevels
//! and recessed seams. Every panel is watertight behind its relief, and ALL
//! relief recedes away from the walkable volume. Plus the walls, floors,
//! ceilings and columns built from it, in every finish.
use super::super::surfaces as styles;
use super::finishes::{
    ceiling_bond, floor_bond, wall_bond, wall_budget, BondStyle, Finish, PillarKind,
};
use super::kit::*;
use super::stage::corner_web;
use std::f32::consts::PI;
use velvet_dungeon::{Biome, Role, Room, Shape, Treasury};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

/// Polygon points in full precision, like the web version's numbers.
pub type P = [f64; 2];
fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn mix(a: P, b: P, t: f64) -> P {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn clip(poly: &[P], normal: P, limit: f64) -> Vec<P> {
    let mut out = vec![];
    for index in 0..poly.len() {
        let a = poly[index];
        let b = poly[(index + 1) % poly.len()];
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
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in poly {
        for i in 0..2 {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    (lo, hi)
}

/// A polygon of f32 points in full precision.
pub fn poly64(points: &[[f32; 2]]) -> Vec<P> {
    points.iter().map(|p| [p[0] as f64, p[1] as f64]).collect()
}

/// A 3D vector in full precision (the web version's `V`).
#[derive(Clone, Copy, Debug)]
pub struct D3(pub f64, pub f64, pub f64);
impl D3 {
    pub fn of(p: Vec3) -> Self {
        D3(p.x as f64, p.y as f64, p.z as f64)
    }
    pub fn f(self) -> Vec3 {
        v(self.0 as f32, self.1 as f32, self.2 as f32)
    }
    pub fn add(self, o: D3) -> D3 {
        D3(self.0 + o.0, self.1 + o.1, self.2 + o.2)
    }
    pub fn sub(self, o: D3) -> D3 {
        D3(self.0 - o.0, self.1 - o.1, self.2 - o.2)
    }
    pub fn mul(self, k: f64) -> D3 {
        D3(self.0 * k, self.1 * k, self.2 * k)
    }
    pub fn cross(self, o: D3) -> D3 {
        D3(
            self.1 * o.2 - self.2 * o.1,
            self.2 * o.0 - self.0 * o.2,
            self.0 * o.1 - self.1 * o.0,
        )
    }
    pub fn dot(self, o: D3) -> f64 {
        self.0 * o.0 + self.1 * o.1 + self.2 * o.2
    }
    pub fn len(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn norm(self) -> D3 {
        let l = self.len();
        if l > 0.0 {
            D3(self.0 / l, self.1 / l, self.2 / l)
        } else {
            self
        }
    }
}

/// `tri` with the web version's full-precision degeneracy test and winding.
pub fn tri64(m: &mut Mesh3D, a: D3, b: D3, c: D3, normal: Vec3, color: C3) -> bool {
    let cross = b.sub(a).cross(c.sub(a));
    let length_sq = cross.dot(cross);
    if length_sq < 1e-12 {
        return false;
    }
    let mut n = cross.mul(1.0 / length_sq.sqrt());
    let agree = n.dot(D3::of(normal)) >= 0.0;
    if !agree {
        n = n.mul(-1.0);
    }
    let base = m.vertices.len() as u32;
    let nf = [n.0 as f32, n.1 as f32, n.2 as f32];
    for p in [a, b, c] {
        let q = p.f();
        let uv = if normal.y.abs() > 0.65 {
            [q.x * 0.28, q.z * 0.28]
        } else if normal.x.abs() > normal.z.abs() {
            [q.z * 0.28, q.y * 0.28]
        } else {
            [q.x * 0.28, q.y * 0.28]
        };
        m.vertices.push(velvet_render::MeshVertex3D {
            position: q.to_array(),
            normal: nf,
            uv,
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    if agree {
        m.indices.extend([base, base + 1, base + 2]);
    } else {
        m.indices.extend([base, base + 2, base + 1]);
    }
    true
}

pub fn quad64(m: &mut Mesh3D, p: [D3; 4], normal: Vec3, color: C3) {
    tri64(m, p[0], p[1], p[2], normal, color);
    tri64(m, p[0], p[2], p[3], normal, tint(color, 0.98));
}

fn scale_colors(m: &mut Mesh3D, start: usize, end: usize, k: f32) {
    for vx in &mut m.vertices[start..end] {
        vx.color[0] *= k;
        vx.color[1] *= k;
        vx.color[2] *= k;
    }
}

/// `panel()` style: the biome's surface plus a bond's options.
#[derive(Clone, Copy, Debug)]
pub struct PStyle {
    pub stone: C3,
    pub moss: C3,
    pub moss_coverage: f32,
    pub mortar: Option<C3>,
    pub checker: Option<C3>,
    pub courses: Option<[f32; 2]>,
    pub bevel: Option<[f32; 2]>,
    pub face_depth: Option<[f32; 2]>,
    pub tone: Option<[f32; 2]>,
    pub crack: Option<f32>,
    pub chip: Option<[f32; 2]>,
}

impl PStyle {
    pub fn new(stone: C3, moss: C3, moss_coverage: f32) -> Self {
        Self {
            stone,
            moss,
            moss_coverage,
            mortar: None,
            checker: None,
            courses: None,
            bevel: None,
            face_depth: None,
            tone: None,
            crack: None,
            chip: None,
        }
    }
    pub fn of(biome: &Biome) -> Self {
        let s = styles::style(biome);
        Self::new(s.stone, s.moss, s.moss_coverage)
    }
    /// Lay a bond's options over this style (`{ ...style, ...bond.style }`).
    pub fn with(mut self, b: BondStyle) -> Self {
        if b.courses.is_some() {
            self.courses = b.courses;
        }
        if b.bevel.is_some() {
            self.bevel = b.bevel;
        }
        if b.face_depth.is_some() {
            self.face_depth = b.face_depth;
        }
        if b.tone.is_some() {
            self.tone = b.tone;
        }
        if b.crack.is_some() {
            self.crack = b.crack;
        }
        if b.chip.is_some() {
            self.chip = b.chip;
        }
        if let Some(c) = b.moss_coverage {
            self.moss_coverage = c;
        }
        self
    }
}

/// Convex panel in (u, v); unit basis vectors, `n` faces the player.
#[allow(clippy::too_many_arguments)]
pub fn panel(
    m: &mut Mesh3D,
    origin: D3,
    u: D3,
    v3: D3,
    n: D3,
    poly: &[P],
    cell: [f64; 2],
    relief: f64,
    style: &PStyle,
    seed: Seed,
    masonry: bool,
) {
    if poly.len() < 3 {
        return;
    }
    panel_inner(
        m, origin, u, v3, n, poly, cell, relief, style, seed, masonry,
    );
}

#[allow(clippy::too_many_arguments)]
fn panel_inner(
    m: &mut Mesh3D,
    origin: D3,
    u: D3,
    v3: D3,
    n: D3,
    poly: &[P],
    cell: [f64; 2],
    relief: f64,
    style: &PStyle,
    seed: Seed,
    masonry: bool,
) {
    let (lo, hi) = bounds(poly);
    let mut rng = ArtRng::stream_s(seed, "hewn_surface_v1", 0);
    let nx = (((hi[0] - lo[0]) / cell[0]).ceil() as usize).max(1);
    let ny = (((hi[1] - lo[1]) / cell[1]).ceil() as usize).max(1);
    let mut sites: Vec<P> = vec![];
    if !masonry {
        for y in 0..ny {
            for x in 0..nx {
                if nx > 2 && ny > 1 && rng.chance(0.12) {
                    continue;
                }
                let stagger = if y % 2 == 1 { 0.25 } else { 0.0 };
                let sx = rng.range64(-0.29, 0.29);
                let sy = rng.range64(-0.29, 0.29);
                sites.push([
                    (x as f64 + 0.5 + stagger + sx) * cell[0] + lo[0],
                    (y as f64 + 0.5 + sy) * cell[1] + lo[1],
                ]);
            }
        }
    }
    let nf = n.f();
    let point = |p: P, depth: f64| {
        D3(
            origin.0 + u.0 * p[0] + v3.0 * p[1] - n.0 * depth,
            origin.1 + u.1 * p[0] + v3.1 * p[1] - n.1 * depth,
            origin.2 + u.2 * p[0] + v3.2 * p[1] - n.2 * depth,
        )
    };
    // Backing stays at the full recess everywhere: never a tapered occluder.
    let centre = mix(lo, hi, 0.5);
    let backing = |p: P| point(p, relief);
    let back_color = style.mortar.unwrap_or(tint(style.stone, 0.3));
    for i in 0..poly.len() {
        tri64(
            m,
            backing(centre),
            backing(poly[i]),
            backing(poly[(i + 1) % poly.len()]),
            nf,
            back_color,
        );
    }
    // Perimeter returns seal the recess; neighbours share the exact boundary.
    let return_color = style
        .mortar
        .map_or(tint(style.stone, 0.72), |c| tint(c, 1.1));
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let ow = u
            .mul(a[0] - centre[0])
            .add(v3.mul(a[1] - centre[1]))
            .norm()
            .f();
        quad64(
            m,
            [point(a, 0.0), backing(a), backing(b), point(b, 0.0)],
            -ow,
            return_color,
        );
    }
    let pair = |r: Option<[f32; 2]>| r.map(|r| [lit(r[0]), lit(r[1])]);
    let (courses, chip, tone, face_depth, bevel) = (
        pair(style.courses),
        pair(style.chip),
        pair(style.tone),
        pair(style.face_depth),
        pair(style.bevel),
    );
    let mut cells: Vec<Vec<P>> = vec![];
    let mut parity: Vec<usize> = vec![];
    if masonry {
        let mut rows: Vec<(f64, f64)> = vec![];
        if let Some(courses) = courses {
            let mut y = lo[1];
            while y < hi[1] - 1e-4 {
                let mut h = cell[1] * rng.range64(courses[0], courses[1]);
                if hi[1] - (y + h) < cell[1] * 0.45 {
                    h = hi[1] - y;
                }
                rows.push((y, hi[1].min(y + h)));
                y += h;
            }
        } else {
            let row_h = (hi[1] - lo[1]) / ny as f64;
            for row in 0..ny {
                rows.push((lo[1] + row as f64 * row_h, lo[1] + (row + 1) as f64 * row_h));
            }
        }
        for (row, &(y0, y1)) in rows.iter().enumerate() {
            let shift = if courses.is_some() {
                rng.range64(0.0, cell[0])
            } else if row % 2 == 1 && style.checker.is_none() {
                cell[0] * 0.5
            } else {
                0.0
            };
            let mut x0 = lo[0] - shift;
            let mut col = 0;
            while x0 < hi[0] {
                let x1 = x0
                    + if style.checker.is_some() {
                        cell[0]
                    } else {
                        cell[0] * rng.range64(0.78, 1.28)
                    };
                let mut piece = poly.to_vec();
                piece = clip(&piece, [-1.0, 0.0], -x0);
                piece = clip(&piece, [1.0, 0.0], x1);
                piece = clip(&piece, [0.0, -1.0], -y0);
                piece = clip(&piece, [0.0, 1.0], y1);
                if piece.len() >= 3 {
                    cells.push(piece);
                    parity.push((row + col) % 2);
                }
                x0 = x1;
                col += 1;
            }
        }
    } else {
        for id in 0..sites.len() {
            let site = sites[id];
            let a = [site[0] / cell[0], site[1] / cell[1]];
            let mut piece = poly.to_vec();
            for (j, other) in sites.iter().enumerate() {
                if id == j {
                    continue;
                }
                let b = [other[0] / cell[0], other[1] / cell[1]];
                let ab = sub(b, a);
                // Anisotropic metric gives sedimentary plates instead of pebbles.
                piece = clip(
                    &piece,
                    [ab[0] / cell[0], ab[1] / cell[1]],
                    (dot(b, b) - dot(a, a)) * 0.5,
                );
                if piece.len() < 3 {
                    break;
                }
            }
            if piece.len() >= 3 {
                cells.push(piece);
            }
        }
    }
    let floor_like = n.1 > 0.65;
    for index in 0..cells.len() {
        let mut piece = cells[index].clone();
        let mut mid = [0.0f64, 0.0];
        for p in &piece {
            mid[0] += p[0] / piece.len() as f64;
            mid[1] += p[1] / piece.len() as f64;
        }
        let (low, high) = bounds(&piece);
        if (high[0] - low[0]) * (high[1] - low[1]) < 0.018 {
            continue;
        }
        // Clip small uneven chips from two corners; the face stays broad.
        let corners = [piece[0], piece[1]];
        for p in corners {
            let radial = sub(p, mid);
            let length = dot(radial, radial).sqrt().max(0.001);
            let cut = [radial[0] / length, radial[1] / length];
            let chip = match chip {
                Some(c) => rng.range64(c[0], c[1]),
                None if floor_like => rng.range64(0.006, 0.022),
                None => rng.range64(0.014, 0.05),
            };
            piece = clip(&piece, cut, dot(p, cut) - chip);
        }
        if piece.len() < 3 {
            continue;
        }
        let base_color = match (style.checker, parity.get(index)) {
            (Some(c), Some(1)) => c,
            _ => style.stone,
        };
        let tone = match tone {
            Some(t) => rng.range64(t[0], t[1]),
            None => rng.range64(0.79, 1.13),
        };
        let mut c64 = [
            base_color[0] as f64 * tone,
            base_color[1] as f64 * tone,
            base_color[2] as f64 * tone,
        ];
        let mineral = rng.range64(-0.09, 0.08);
        c64[0] *= 1.0 + mineral;
        c64[2] *= 1.0 - mineral * 1.5;
        let color: C3 = [c64[0] as f32, c64[1] as f32, c64[2] as f32];
        let face_depth = relief
            * match face_depth {
                Some(f) => rng.range64(f[0], f[1]),
                None => rng.range64(0.44, 0.6),
            };
        let tilt = [rng.range64(-0.022, 0.022), rng.range64(-0.023, 0.023)];
        let depth = |p: P| {
            (face_depth + dot(sub(p, mid), tilt))
                .max(relief * 0.3)
                .min(relief * 0.75)
        };
        let face_mid = point(mid, depth(mid));
        let bevel = match bevel {
            Some(b) => rng.range64(b[0], b[1]),
            None if floor_like => rng.range64(0.969, 0.984),
            None => rng.range64(0.87, 0.94),
        };
        let moss = rng.chance(lit(style.moss_coverage));
        for i in 0..piece.len() {
            let a = piece[i];
            let b = piece[(i + 1) % piece.len()];
            let fa = mix(mid, a, bevel);
            let fb = mix(mid, b, bevel);
            let pa = point(fa, depth(fa));
            let pb = point(fb, depth(fb));
            let upper = (a[1] + b[1]) * 0.5 > mid[1] + 0.07;
            let bevel_color = if moss && (upper || floor_like) && n.1 > -0.5 {
                style.moss
            } else {
                tint(color, if upper { 1.19 } else { 0.62 })
            };
            quad64(
                m,
                [
                    point(mix(mid, a, 0.993), relief),
                    point(mix(mid, b, 0.993), relief),
                    pb,
                    pa,
                ],
                nf,
                bevel_color,
            );
            let start = m.vertices.len();
            if tri64(m, face_mid, pa, pb, nf, color) {
                // Vertex-painted cavity gradient shapes light on broad faces.
                let edge = if upper { 1.02 } else { 0.83 };
                scale_colors(m, start, start + 1, 1.07);
                scale_colors(m, start + 1, start + 3, edge);
            }
            if moss && upper && n.1.abs() < 0.5 {
                let tip = mix(mix(fa, fb, 0.5), mid, rng.range64(0.12, 0.32));
                let (qa, qb, qt) = (
                    point(fa, depth(fa) - 0.003),
                    point(fb, depth(fb) - 0.003),
                    point(tip, depth(tip) - 0.003),
                );
                if tri64(m, qa, qb, qt, nf, style.moss) {
                    let last = m.vertices.len() - 1;
                    m.vertices[last].color[0] = color[0];
                    m.vertices[last].color[1] = color[1];
                    m.vertices[last].color[2] = color[2];
                }
            }
        }
        // Sparse fractured seams, not repeating noise.
        let crack = style.crack.map_or(if masonry { 0.24 } else { 0.34 }, lit);
        if rng.chance(crack) {
            let edge_point = piece[rng.index(piece.len())];
            let a = mix(mid, edge_point, 0.24);
            let b = mix(mid, edge_point, 0.64);
            let e = sub(b, a);
            let len = dot(e, e).sqrt().max(0.001);
            let side = [-e[1] / len * 0.011, e[0] / len * 0.011];
            let offset = |p: P, s: f64| [p[0] + side[0] * s, p[1] + side[1] * s];
            quad64(
                m,
                [
                    point(offset(a, -1.0), depth(a) - 0.004),
                    point(offset(b, -0.2), depth(b) - 0.004),
                    point(offset(b, 0.2), depth(b) - 0.004),
                    point(offset(a, 1.0), depth(a) - 0.004),
                ],
                nf,
                tint(color, 0.58),
            );
        }
    }
}

// ------------------------------------------------------------ room kinds

pub fn is_treasury(room: &Room) -> bool {
    room.role == Role::Treasure && room.shape != Shape::Bridge
}
pub fn is_grand_treasury(room: &Room) -> bool {
    is_treasury(room) && room.treasury == Some(Treasury::Grand)
}
pub fn is_loot_hall(room: &Room) -> bool {
    is_treasury(room) && room.treasury != Some(Treasury::Grand)
}
pub fn is_sanctuary_hall(room: &Room) -> bool {
    room.role == Role::Rest && room.shape != Shape::Bridge
}
pub fn is_arcane_hall(room: &Room) -> bool {
    room.role == Role::Secret && room.shape != Shape::Bridge
}

/// Warm hewn stone of the sanctuary nave, tinted by its biome.
pub fn sanctuary_stone(biome: &Biome) -> (C3, C3) {
    let s = styles::style(biome);
    (
        mix_color(s.stone, [0.46, 0.38, 0.31], 0.62),
        mix_color(s.floor, [0.42, 0.34, 0.27], 0.55),
    )
}

/// Accent of bosses and gilded bands (web `ornate`).
pub fn ornate(room: &Room) -> Option<C3> {
    if room.shape == Shape::Bridge {
        return None;
    }
    Some(match room.role {
        Role::Treasure => [0.9, 0.58, 0.16],
        Role::Secret => [0.42, 0.3, 0.62],
        _ => match room.biome {
            0 => [0.72, 0.5, 0.24],
            1 => [0.2, 0.58, 0.6],
            _ => [0.78, 0.3, 0.14],
        },
    })
}

/// The treasury's chunky hand-cut stone.
fn treasury_stone(s: PStyle) -> PStyle {
    PStyle {
        bevel: Some([0.8, 0.87]),
        face_depth: Some([0.2, 0.44]),
        tone: Some([0.7, 1.1]),
        courses: Some([0.75, 1.35]),
        crack: Some(0.3),
        moss_coverage: 0.04,
        ..s
    }
}

/// The socket on the wall edge (origin, u, n, length), if any.
fn wall_socket(
    room: &Room,
    origin: D3,
    u: D3,
    n: D3,
    length: f64,
) -> Option<&velvet_dungeon::Socket> {
    room.sockets.iter().find(|entry| {
        let d = D3(
            entry.position.x as f64 - origin.0,
            0.0,
            entry.position.z as f64 - origin.2,
        );
        d.dot(n).abs() < 0.01 && d.dot(u) >= 0.0 && d.dot(u) <= length
    })
}

fn wall_regions(room: &Room, origin: D3, u: D3, n: D3, length: f64, bottom: f64) -> Vec<[f64; 4]> {
    // Heights and widths are the catalog's decimals in the web version.
    let h = lit(room.height);
    match wall_socket(room, origin, u, n, length) {
        Some(socket) => {
            let at = D3(
                socket.position.x as f64 - origin.0,
                0.0,
                socket.position.z as f64 - origin.2,
            )
            .dot(u);
            let (w, sh) = (lit(socket.width), lit(socket.height));
            vec![
                [0.0, at - w * 0.5, bottom, h],
                [at + w * 0.5, length, bottom, h],
                [at - w * 0.5, at + w * 0.5, sh, h],
            ]
        }
        None => vec![[0.0, length, bottom, h]],
    }
}

/// An outline edge in full precision: (origin, u, n, length).
fn edge_frame(room: &Room, i: usize) -> (D3, D3, D3, f64) {
    let p = room.outline[i];
    let q = room.outline[(i + 1) % room.outline.len()];
    let origin = D3(
        room.center.x as f64 + p[0] as f64,
        room.center.y as f64,
        room.center.z as f64 + p[1] as f64,
    );
    let edge = D3(q[0] as f64 - p[0] as f64, 0.0, q[1] as f64 - p[1] as f64);
    let u = edge.norm();
    (origin, u, D3(-u.2, 0.0, u.0), edge.len())
}

fn rect(x0: f64, x1: f64, y0: f64, y1: f64) -> Vec<P> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn perimeter(room: &Room) -> f64 {
    let mut out = 0.0;
    for i in 0..room.outline.len() {
        let a = room.outline[i];
        let b = room.outline[(i + 1) % room.outline.len()];
        out += (b[0] as f64 - a[0] as f64).hypot(b[1] as f64 - a[1] as f64);
    }
    out
}

const DY: D3 = D3(0.0, 1.0, 0.0);
const DX: D3 = D3(1.0, 0.0, 0.0);
const DZ: D3 = D3(0.0, 0.0, 1.0);
const DDOWN: D3 = D3(0.0, -1.0, 0.0);

pub fn walls(m: &mut Mesh3D, room: &Room, biome: &Biome, seed: Seed, finish: Option<&Finish>) {
    if is_grand_treasury(room) || is_arcane_hall(room) {
        return; // grand.rs cuts the niches itself
    }
    if is_loot_hall(room) {
        treasury_walls(m, room, biome, seed, None);
        return;
    }
    if is_sanctuary_hall(room) {
        treasury_walls(m, room, biome, seed, Some(sanctuary_stone(biome).0));
        return;
    }
    if let Some(f) = finish {
        finished_walls(m, room, biome, seed, f);
        return;
    }
    let s = PStyle::of(biome);
    let fine = ornate(room).is_some();
    let masonry = fine || room.shape == Shape::Vault || room.biome == 0;
    let cell: [f64; 2] = if fine {
        [0.86, 0.43]
    } else if masonry {
        [1.34, 0.81]
    } else {
        [1.75, 1.08]
    };
    let area = perimeter(room)
        * (lit(room.height)
            + if room.shape == Shape::Bridge {
                4.0
            } else {
                0.0
            });
    let budget = if fine { 620.0 } else { 230.0 };
    let scale = (area / (cell[0] * cell[1] * budget)).sqrt().max(1.0);
    let relief = if fine {
        0.22
    } else if masonry {
        0.3
    } else {
        0.43
    };
    for i in 0..room.outline.len() {
        let (origin, u, n, length) = edge_frame(room, i);
        let bottom = if room.shape == Shape::Bridge {
            -4.0
        } else {
            0.0
        };
        for (j, r) in wall_regions(room, origin, u, n, length, bottom)
            .iter()
            .enumerate()
        {
            let [x0, x1, y0, y1] = *r;
            if x1 - x0 < 0.01 || y1 - y0 < 0.01 {
                continue;
            }
            panel(
                m,
                origin,
                u,
                DY,
                n,
                &rect(x0, x1, y0, y1),
                [cell[0] * scale, cell[1] * scale],
                relief,
                &s,
                wrap_seed(seed, (i * 13 + j) as u32),
                masonry,
            );
        }
    }
}

fn treasury_walls(m: &mut Mesh3D, room: &Room, biome: &Biome, seed: Seed, stone: Option<C3>) {
    let mut s = treasury_stone(PStyle::of(biome));
    if let Some(c) = stone {
        s.stone = c;
    }
    for i in 0..room.outline.len() {
        let (origin, u, n, length) = edge_frame(room, i);
        for (j, r) in wall_regions(room, origin, u, n, length, 0.0)
            .iter()
            .enumerate()
        {
            let [x0, x1, y0, y1] = *r;
            if x1 - x0 < 0.01 || y1 - y0 < 0.01 {
                continue;
            }
            panel(
                m,
                origin,
                u,
                DY,
                n,
                &rect(x0, x1, y0, y1),
                [1.2, 0.58],
                0.36,
                &s,
                wrap_seed(seed, (i * 13 + j) as u32),
                true,
            );
        }
    }
}

/// Big irregular flagstones with open joints and cracks (treasury floor).
pub fn treasury_floor(m: &mut Mesh3D, room: &Room, color: C3, seed: Seed) {
    let style = PStyle {
        bevel: Some([0.9, 0.945]),
        face_depth: Some([0.18, 0.4]),
        tone: Some([0.68, 1.1]),
        crack: Some(0.5),
        chip: Some([0.03, 0.08]),
        ..PStyle::new(color, [0.07, 0.12, 0.025], 0.03)
    };
    panel(
        m,
        D3::of(room.center),
        DX,
        DZ,
        DY,
        &poly64(&room.outline),
        [1.15, 0.95],
        0.075,
        &style,
        seed,
        false,
    );
}

pub fn ceiling(m: &mut Mesh3D, room: &Room, biome: &Biome, seed: Seed, finish: Option<&Finish>) {
    let base = PStyle::of(biome);
    let top = D3(
        room.center.x as f64,
        room.center.y as f64 + lit(room.height),
        room.center.z as f64,
    );
    let outline = poly64(&room.outline);
    if let Some(f) = finish {
        if !is_grand_treasury(room)
            && !is_sanctuary_hall(room)
            && !is_arcane_hall(room)
            && !is_loot_hall(room)
        {
            finished_ceiling(m, room, biome, seed, f);
            return;
        }
    }
    if is_grand_treasury(room) {
        let s = PStyle {
            stone: [0.3, 0.285, 0.28],
            moss_coverage: 0.0,
            ..base
        };
        panel(
            m,
            top,
            DX,
            DZ,
            DDOWN,
            &outline,
            [2.1, 1.7],
            0.8,
            &s,
            seed,
            false,
        );
        return;
    }
    if is_sanctuary_hall(room) {
        let s = PStyle {
            stone: tint(sanctuary_stone(biome).0, 0.8),
            moss_coverage: 0.0,
            bevel: Some([0.84, 0.9]),
            face_depth: Some([0.3, 0.5]),
            tone: Some([0.78, 1.08]),
            crack: Some(0.2),
            ..base
        };
        panel(
            m,
            top,
            DX,
            DZ,
            DDOWN,
            &outline,
            [2.3, 1.9],
            0.7,
            &s,
            seed,
            false,
        );
        return;
    }
    if is_arcane_hall(room) {
        let s = PStyle {
            stone: [0.31, 0.285, 0.265],
            moss_coverage: 0.0,
            bevel: Some([0.88, 0.93]),
            face_depth: Some([0.3, 0.5]),
            tone: Some([0.8, 1.08]),
            crack: Some(0.12),
            ..base
        };
        panel(
            m,
            top,
            DX,
            DZ,
            DDOWN,
            &outline,
            [1.4, 0.7],
            0.3,
            &s,
            seed,
            true,
        );
        return;
    }
    if is_loot_hall(room) {
        let s = PStyle {
            stone: tint(base.stone, 0.78),
            moss_coverage: 0.0,
            bevel: Some([0.86, 0.92]),
            face_depth: Some([0.35, 0.55]),
            crack: Some(0.15),
            ..treasury_stone(base)
        };
        panel(
            m,
            top,
            DX,
            DZ,
            DDOWN,
            &outline,
            [1.6, 0.85],
            0.3,
            &s,
            seed,
            true,
        );
        return;
    }
    let s = PStyle {
        stone: tint(base.stone, 0.86),
        moss_coverage: 0.0,
        ..base
    };
    panel(
        m,
        top,
        DX,
        DZ,
        DDOWN,
        &outline,
        [2.25, 1.8],
        0.68,
        &s,
        seed,
        false,
    );
}

/// Vault panels of a tunnel; boundary returns seal the cross-section.
pub fn tunnel_panel(m: &mut Mesh3D, p: [D3; 4], normal: D3, biome: &Biome, seed: Seed) {
    let width = p[1].sub(p[0]).len();
    let length = p[3].sub(p[0]).len();
    if width < 0.01 || length < 0.01 {
        return;
    }
    let s = PStyle {
        stone: biome.wall,
        ..PStyle::of(biome)
    };
    panel(
        m,
        p[0],
        p[1].sub(p[0]).norm(),
        p[3].sub(p[0]).norm(),
        normal,
        &rect(0.0, width, 0.0, length),
        [0.92, 1.25],
        0.28,
        &s,
        seed,
        false,
    );
}

/// Coursed ashlar for tunnel walls; relief only recedes.
#[allow(clippy::too_many_arguments)]
pub fn tunnel_ashlar(
    m: &mut Mesh3D,
    origin: D3,
    along: D3,
    up: D3,
    normal: D3,
    length: f64,
    height: f64,
    stone: C3,
    seed: Seed,
) {
    if length < 0.01 || height < 0.01 {
        return;
    }
    let style = PStyle::new(stone, [0.07, 0.13, 0.03], 0.08);
    panel(
        m,
        origin,
        along.norm(),
        up.norm(),
        normal,
        &rect(0.0, length, 0.0, height),
        [0.95, 0.47],
        0.2,
        &style,
        seed,
        true,
    );
}

/// Running-bond paving strip for passages.
#[allow(clippy::too_many_arguments)]
pub fn paving(
    m: &mut Mesh3D,
    origin: D3,
    u: D3,
    v: D3,
    normal: D3,
    outline: &[P],
    color: C3,
    seed: Seed,
) {
    panel(
        m,
        origin,
        u,
        v,
        normal,
        outline,
        [0.62, 0.95],
        0.05,
        &PStyle::new(color, [0.07, 0.12, 0.025], 0.05),
        seed,
        true,
    );
}

/// Shallow recessed flagstones; every slab stays at or below the floor.
#[allow(clippy::too_many_arguments)]
pub fn flagstones(
    m: &mut Mesh3D,
    origin: D3,
    u: D3,
    v: D3,
    normal: D3,
    outline: &[P],
    color: C3,
    seed: Seed,
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
        &PStyle::new(color, [0.07, 0.12, 0.025], 0.22),
        seed,
        false,
    );
}

/// Inset a convex polygon by `d`, keeping its vertex count.
fn inset(poly: &[P], d: f64) -> Vec<P> {
    let count = poly.len();
    let mut centroid = [0.0f64, 0.0];
    for p in poly {
        centroid[0] += p[0] / count as f64;
        centroid[1] += p[1] / count as f64;
    }
    let lines: Vec<(P, P)> = (0..count)
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % count];
            let e = sub(b, a);
            let len = dot(e, e).sqrt().max(1e-4);
            let dir = [e[0] / len, e[1] / len];
            let mut normal = [-dir[1], dir[0]];
            if dot(sub(centroid, a), normal) < 0.0 {
                normal = [dir[1], -dir[0]];
            }
            ([a[0] + normal[0] * d, a[1] + normal[1] * d], dir)
        })
        .collect();
    (0..count)
        .map(|i| {
            let (p0, d0) = lines[(i + count - 1) % count];
            let (p1, d1) = lines[i];
            let denom = d0[0] * d1[1] - d0[1] * d1[0];
            if denom.abs() < 1e-5 {
                return p1;
            }
            let w = sub(p1, p0);
            let t = (w[0] * d1[1] - w[1] * d1[0]) / denom;
            [p0[0] + d0[0] * t, p0[1] + d0[1] * t]
        })
        .collect()
}

/// Ashlar paving for signature rooms inside a kerb band of long stones.
pub fn ashlar_floor(m: &mut Mesh3D, room: &Room, axis: D3, color: C3, seed: Seed, cell: [f64; 2]) {
    let mut u = D3(axis.0, 0.0, axis.2).norm();
    if u.dot(u) < 0.5 {
        u = DX;
    }
    let w = D3(-u.2, 0.0, u.0);
    let outline: Vec<P> = room
        .outline
        .iter()
        .map(|p| {
            let q = D3(p[0] as f64, 0.0, p[1] as f64);
            [q.dot(u), q.dot(w)]
        })
        .collect();
    let band = 0.58;
    let inner = inset(&outline, band);
    let origin = D3::of(room.center);
    let pave = PStyle::new(color, [0.07, 0.12, 0.025], 0.03);
    panel(m, origin, u, w, DY, &inner, cell, 0.05, &pave, seed, true);
    let kerb = PStyle {
        stone: tint(color, 0.78),
        ..pave
    };
    let count = outline.len();
    for i in 0..count {
        let (a, b) = (outline[i], outline[(i + 1) % count]);
        let (ai, bi) = (inner[i], inner[(i + 1) % count]);
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
        let world = |p: P| u.mul(p[0]).add(w.mul(p[1]));
        panel(
            m,
            origin.add(world(a)),
            world(su),
            world(sv),
            DY,
            &strip,
            [0.9, band + 0.02],
            0.05,
            &kerb,
            wrap_seed(seed, 97 + i as u32),
            true,
        );
    }
}

/// Rubble at the wall foot and hanging moss above jump headroom.
pub fn age_dressing(
    m: &mut Mesh3D,
    room: &Room,
    biome: &Biome,
    seed: Seed,
    finish: Option<&Finish>,
) {
    let mut s = PStyle::of(biome);
    if let Some(f) = finish {
        s.stone = f.stone;
    }
    let mut rng = ArtRng::stream_s(seed, "masonry_age_v1", 0);
    let rubble = finish.map_or(0.65, |f| 0.12 + 0.75 * f.wear.rubble);
    let hanging = finish.map_or(s.moss_coverage * 1.55, |f| {
        s.moss_coverage * 1.55 * (0.15 + 1.6 * f.wear.moss)
    });
    let n = room.outline.len();
    if let Some(f) = finish.filter(|f| f.wear.webs > 0.05) {
        // Cobwebs in the upper corners, where walls meet the chamfers.
        for i in 0..n {
            if !rng.chance((f.wear.webs * 0.8) as f64) {
                continue;
            }
            let a = room.outline[i];
            let b = room.outline[(i + 1) % n];
            let c = room.outline[(i + n - 1) % n];
            let lift = rng.range(0.0, 1.0);
            let apex = v(
                room.center.x + a[0],
                room.center.y + (room.height - 0.3).min(3.2 + lift),
                room.center.z + a[1],
            );
            let inward = norm(v(room.center.x - apex.x, 0.0, room.center.z - apex.z));
            let da = norm(norm(v(b[0] - a[0], 0.0, b[1] - a[1])) + Y(-0.35));
            let dc = norm(norm(v(c[0] - a[0], 0.0, c[1] - a[1])) + Y(-0.35));
            let size = rng.range(0.55, 0.95);
            corner_web(
                m,
                apex + inward * 0.06,
                da,
                dc,
                size,
                &mut rng,
                [0.78, 0.78, 0.8],
            );
        }
    }
    for i in 0..n {
        let a = room.outline[i];
        let b = room.outline[(i + 1) % n];
        let edge = v(b[0] - a[0], 0.0, b[1] - a[1]);
        let edge_length = edge.length();
        let u = norm(edge);
        let inward = v(-u.z, 0.0, u.x);
        let steps = (edge_length / 1.4).ceil() as usize;
        for j in 0..steps {
            let t = (j as f32 + rng.range(0.1, 0.8)) * 1.4 / edge_length;
            if t > 0.97 {
                continue;
            }
            let p = v(room.center.x + a[0], room.center.y, room.center.z + a[1])
                + edge * t
                + inward * 0.14;
            if room
                .sockets
                .iter()
                .any(|socket| (socket.position - p).length() < socket.width * 0.5 + 0.65)
            {
                continue;
            }
            if rng.chance(rubble as f64) {
                let floor = v(p.x, room.floor_y(p.x, p.z), p.z);
                for _ in 0..3 {
                    let du = rng.range(-0.3, 0.3);
                    let di = rng.range(0.0, 0.22);
                    let q = floor + u * du + inward * di;
                    let size = rng.range(0.045, 0.16);
                    let shade = rng.range(0.52, 0.92);
                    let phase = rng.range(0.0, 6.28);
                    rock(m, q, v(size * 1.4, size, size), tint(s.stone, shade), phase);
                }
            }
            if !rng.chance(hanging as f64) {
                continue;
            }
            // Roots embed in the recessed roof relief instead of floating.
            let root = p + Y(room.height + 0.5);
            let phase = rng.range(0.0, 6.28);
            rock(
                m,
                root - Y(0.075),
                v(0.29, 0.13, 0.22),
                tint(s.moss, 1.25),
                phase,
            );
            let strands = rng.index(3) + 3;
            for strand in 0..strands {
                let length = rng
                    .range(0.72, 1.24)
                    .min(((room.height + 0.5 - 2.95) / 1.07).max(0.05));
                let start = root + u * (strand as f32 * 0.09 - 0.18);
                for k in 0..5 {
                    let f = k as f32 / 5.0;
                    let at = start - Y(f * length) + inward * ((f * 6.0).sin() * 0.06);
                    let width = (1.0 - f) * rng.range(0.045, 0.1);
                    let tip = at - Y(length * 0.27) + u * rng.range(-0.04, 0.04);
                    let ridge = at + inward * 0.035;
                    tri(m, at - u * width, ridge, tip, inward, tint(s.moss, 0.72));
                    tri(m, ridge, at + u * width, tip, inward, tint(s.moss, 1.3));
                    tri(m, tip, at + u * width, at - u * width, -inward, s.moss);
                }
            }
        }
    }
}

/// Free-standing treasury column: stacked chunky blocks with a capital.
fn treasury_column(
    m: &mut Mesh3D,
    at: Vec3,
    half: [f32; 2],
    height: f32,
    stone: C3,
    seed: Seed,
    carved: bool,
) {
    let mut rng = ArtRng::stream_s(seed, "treasury_column_v1", 0);
    let w = half[0].min(half[1]);
    let slab = |m: &mut Mesh3D, y0: f32, y1: f32, h: f32, color: C3, bevel: f32| {
        chamfer_box(
            m,
            at + Y((y0 + y1) * 0.5),
            VX,
            VY,
            VZ,
            v(h, (y1 - y0) * 0.5, h),
            bevel,
            color,
        )
    };
    slab(m, 0.0, 0.34, w + 0.02, tint(stone, 0.78), 0.06);
    let top = height - 1.05;
    let courses = (((top - 0.34) / 0.58).round() as usize).max(3);
    let course = (top - 0.34) / courses as f32;
    for c in 0..courses {
        let y0 = 0.34 + c as f32 * course;
        let shade = rng.range(0.84, 1.06);
        slab(
            m,
            y0 + 0.012,
            y0 + course - 0.012,
            w - if c % 2 == 0 { 0.04 } else { 0.07 },
            tint(stone, shade),
            0.05,
        );
    }
    if carved {
        for face in 0..4 {
            let a = face as f32 * PI / 2.0;
            let n = v(a.cos(), 0.0, a.sin());
            diamond_relief(
                m,
                at + n * (w - 0.02) + Y(top - course * 0.5),
                v(-n.z, 0.0, n.x),
                n,
                0.13,
                stone,
            );
        }
    }
    slab(m, top, top + 0.16, w + 0.08, tint(stone, 1.12), 0.05);
    slab(m, top + 0.16, top + 0.42, w + 0.02, tint(stone, 0.9), 0.05);
    slab(m, top + 0.42, top + 0.62, w + 0.16, tint(stone, 1.16), 0.06);
    slab(
        m,
        top + 0.62,
        height + 0.35,
        w + 0.04,
        tint(stone, 0.86),
        0.05,
    );
}

fn diamond_relief(m: &mut Mesh3D, c: Vec3, t: Vec3, n: Vec3, size: f32, stone: C3) {
    let d1 = norm(t + VY);
    let d2 = norm(-t + VY);
    chamfer_box(
        m,
        c,
        d1,
        d2,
        n,
        v(size, size, 0.045),
        0.025,
        tint(stone, 1.14),
    );
    chamfer_box(
        m,
        c + n * 0.03,
        d1,
        d2,
        n,
        v(size * 0.55, size * 0.55, 0.03),
        0.015,
        tint(stone, 0.72),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn pillar(
    m: &mut Mesh3D,
    position: Vec3,
    half: [f32; 2],
    height: f32,
    biome: &Biome,
    seed: Seed,
    accent: Option<C3>,
    treasury: bool,
    sanctuary: bool,
    finish: Option<&Finish>,
) {
    let s = styles::style(biome);
    if treasury {
        treasury_column(m, position, half, height, s.stone, seed, true);
        return;
    }
    if sanctuary {
        treasury_column(
            m,
            position,
            half,
            height,
            sanctuary_stone(biome).0,
            seed,
            false,
        );
        return;
    }
    if let Some(f) = finish {
        finished_pillar(m, position, half, height, seed, f);
        return;
    }
    if let Some(accent) = accent {
        ornate_pier(m, position, half, height, s.stone, accent, seed);
        return;
    }
    plain_pier(m, position, half, height, s.stone, seed);
}

/// Octagonal coursed pier inside the rectangular collision bound.
fn plain_pier(m: &mut Mesh3D, position: Vec3, half: [f32; 2], height: f32, stone: C3, seed: Seed) {
    let ring: [[f32; 2]; 8] = [
        [-1.0, -0.75],
        [-0.75, -1.0],
        [0.75, -1.0],
        [1.0, -0.75],
        [1.0, 0.75],
        [0.75, 1.0],
        [-0.75, 1.0],
        [-1.0, 0.75],
    ];
    let mut rng = ArtRng::stream_s(seed, "pillar_courses", 0);
    let count = (height / 0.65).ceil() as usize;
    for j in 0..count {
        let y0 = j as f32 / count as f32 * height;
        let y1 = (j + 1) as f32 / count as f32 * height;
        let f = if j == 0 || j == count - 1 {
            1.0
        } else {
            rng.range(0.87, 0.97)
        };
        let c = tint(stone, rng.range(0.85, 1.05));
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            let at = |p: [f32; 2], y: f32, k: f32| {
                position + v(p[0] * half[0] * k, y, p[1] * half[1] * k)
            };
            let n = norm(v(a[0] + b[0], 0.0, a[1] + b[1]));
            quad(
                m,
                [
                    at(a, y0, 0.82),
                    at(b, y0, 0.82),
                    at(b, y0 + 0.09, f),
                    at(a, y0 + 0.09, f),
                ],
                n,
                tint(c, 0.68),
            );
            quad(
                m,
                [
                    at(a, y0 + 0.09, f),
                    at(b, y0 + 0.09, f),
                    at(b, y1 - 0.07, f),
                    at(a, y1 - 0.07, f),
                ],
                n,
                c,
            );
            quad(
                m,
                [
                    at(a, y1 - 0.07, f),
                    at(b, y1 - 0.07, f),
                    at(b, y1, 0.82),
                    at(a, y1, 0.82),
                ],
                n,
                tint(c, 1.13),
            );
        }
    }
}

/// Classical pier: moulded base, reeded faces, gilded band, flared capital.
fn ornate_pier(
    m: &mut Mesh3D,
    at: Vec3,
    half: [f32; 2],
    height: f32,
    stone: C3,
    accent: C3,
    seed: Seed,
) {
    let mut rng = ArtRng::stream_s(seed, "ornate_pier_v1", 0);
    let w = half[0].min(half[1]);
    let slab = |m: &mut Mesh3D, y0: f32, y1: f32, h: f32, color: C3| {
        block(
            m,
            at + Y((y0 + y1) * 0.5),
            VX,
            VY,
            VZ,
            v(h, (y1 - y0) * 0.5, h),
            color,
        )
    };
    slab(m, 0.0, 0.28, w + 0.03, tint(stone, 0.68));
    slab(m, 0.28, 0.37, w, tint(stone, 1.14));
    slab(m, 0.37, 0.41, w - 0.05, tint(stone, 0.9));
    let top = height - 0.95;
    slab(m, 0.41, top, w - 0.1, tint(stone, 0.6));
    for face in 0..4 {
        let a = face as f32 * PI / 2.0;
        let n = v(a.cos(), 0.0, a.sin());
        let t = v(-n.z, 0.0, n.x);
        for offset in [-0.19, 0.0, 0.19] {
            let shade = rng.range(0.95, 1.08);
            block(
                m,
                at + n * (w - 0.1) + t * offset + Y((0.41 + top) * 0.5),
                t,
                VY,
                n,
                v(0.06, (top - 0.41) * 0.5, 0.055),
                tint(stone, shade),
            );
        }
    }
    slab(m, 1.95, 2.05, w - 0.02, tint(accent, 1.0));
    slab(m, 2.05, 2.08, w - 0.06, tint(accent, 0.7));
    slab(m, top, top + 0.06, w - 0.05, tint(stone, 1.1));
    slab(m, top + 0.06, top + 0.3, w - 0.02, tint(stone, 0.95));
    slab(m, top + 0.3, top + 0.44, w + 0.06, tint(stone, 1.12));
    slab(m, top + 0.44, top + 0.5, w + 0.03, tint(stone, 0.78));
    slab(m, top + 0.5, top + 0.62, w + 0.1, tint(stone, 1.22));
    slab(m, top + 0.62, height + 0.3, w - 0.02, tint(stone, 0.88));
}

// ------------------------------------------------------------ room finishes

/// Isotropic Voronoi pieces of a convex polygon (plaster sheets).
fn voronoi_pieces(poly: &[P], cell: [f64; 2], rng: &mut ArtRng) -> Vec<Vec<P>> {
    let (lo, hi) = bounds(poly);
    let nx = (((hi[0] - lo[0]) / cell[0]).ceil() as usize).max(1);
    let ny = (((hi[1] - lo[1]) / cell[1]).ceil() as usize).max(1);
    let mut sites = vec![];
    for y in 0..ny {
        for x in 0..nx {
            let sx = rng.range64(-0.32, 0.32);
            let sy = rng.range64(-0.32, 0.32);
            sites.push([
                (x as f64 + 0.5 + sx) * cell[0] + lo[0],
                (y as f64 + 0.5 + sy) * cell[1] + lo[1],
            ]);
        }
    }
    let mut pieces = vec![];
    for id in 0..sites.len() {
        let a = sites[id];
        let mut piece = poly.to_vec();
        for (j, &b) in sites.iter().enumerate() {
            if piece.len() < 3 {
                break;
            }
            if j == id {
                continue;
            }
            if (b[0] - a[0]).abs() > cell[0] * 2.5 || (b[1] - a[1]).abs() > cell[1] * 2.5 {
                continue;
            }
            piece = clip(&piece, sub(b, a), (dot(b, b) - dot(a, a)) * 0.5);
        }
        if piece.len() >= 3 {
            pieces.push(piece);
        }
    }
    pieces
}

/// Plaster over the bond, fallen away in patches (most near the floor).
#[allow(clippy::too_many_arguments)]
fn plaster_sheet(
    m: &mut Mesh3D,
    origin: D3,
    u: D3,
    w: D3,
    n: D3,
    poly: &[P],
    color: C3,
    holes: f64,
    seed: Seed,
) {
    let mut rng = ArtRng::stream_s(seed, "plaster_sheet_v1", 0);
    let (lo, _) = bounds(poly);
    let at = |p: P, d: f64| origin.add(u.mul(p[0])).add(w.mul(p[1])).sub(n.mul(d));
    let nf = n.f();
    for piece in voronoi_pieces(poly, [0.85, 0.7], &mut rng) {
        let mut mid = [0.0f64, 0.0];
        for p in &piece {
            mid[0] += p[0] / piece.len() as f64;
            mid[1] += p[1] / piece.len() as f64;
        }
        let low = (1.0 - (mid[1] - lo[1]) / 1.4).max(0.0);
        if rng.chance(holes * (0.55 + low * 1.1)) {
            continue;
        }
        let c = tint(color, rng.range(0.88, 1.05));
        for i in 0..piece.len() {
            tri64(
                m,
                at(mid, 0.004),
                at(piece[i], 0.004),
                at(piece[(i + 1) % piece.len()], 0.004),
                nf,
                if i % 2 == 1 { c } else { tint(c, 0.97) },
            );
        }
    }
}

/// Panelled wainscot along the foot of a wall region: rails, stiles, fields.
#[allow(clippy::too_many_arguments)]
fn wainscot(
    m: &mut Mesh3D,
    origin: D3,
    u: D3,
    n: D3,
    x0: f64,
    x1: f64,
    h: f32,
    wood: C3,
    seed: Seed,
) {
    if x1 - x0 < 0.35 {
        return;
    }
    let (origin, u, n) = (origin.f(), u.f(), n.f());
    let (x0, x1) = (x0 as f32, x1 as f32);
    let mut rng = ArtRng::stream_s(seed, "wainscot_v1", 0);
    let p = |x: f32, y: f32, d: f32| origin + u * x + Y(y) - n * d;
    let face = |m: &mut Mesh3D, xa: f32, xb: f32, ya: f32, yb: f32, d: f32, color: C3| {
        quad(
            m,
            [p(xa, ya, d), p(xb, ya, d), p(xb, yb, d), p(xa, yb, d)],
            n,
            color,
        )
    };
    face(m, x0, x1, 0.0, 0.17, 0.003, tint(wood, 0.6));
    face(m, x0, x1, 0.17, 0.25, 0.006, tint(wood, 0.95));
    face(m, x0, x1, h - 0.1, h, 0.005, tint(wood, 1.02));
    face(m, x0, x1, h, h + 0.035, 0.002, tint(wood, 1.32));
    let count = (((x1 - x0) / 0.72).round() as usize).max(1);
    let step = (x1 - x0) / count as f32;
    for k in 0..=count {
        let x = x0 + k as f32 * step;
        let shade = rng.range(0.85, 0.98);
        face(
            m,
            x0.max(x - 0.05),
            x1.min(x + 0.05),
            0.25,
            h - 0.1,
            0.005,
            tint(wood, shade),
        );
    }
    for k in 0..count {
        let xa = x0 + k as f32 * step + 0.05;
        let xb = x0 + (k + 1) as f32 * step - 0.05;
        let (ya, yb) = (0.25, h - 0.1);
        let b = 0.055;
        let shade = rng.range(0.66, 0.82);
        face(m, xa + b, xb - b, ya + b, yb - b, 0.028, tint(wood, shade));
        let down = norm(n + Y(0.6));
        let up = norm(n - Y(0.6));
        quad(
            m,
            [
                p(xa, ya, 0.006),
                p(xb, ya, 0.006),
                p(xb - b, ya + b, 0.028),
                p(xa + b, ya + b, 0.028),
            ],
            down,
            tint(wood, 1.08),
        );
        quad(
            m,
            [
                p(xa, yb, 0.006),
                p(xb, yb, 0.006),
                p(xb - b, yb - b, 0.028),
                p(xa + b, yb - b, 0.028),
            ],
            up,
            tint(wood, 0.58),
        );
        quad(
            m,
            [
                p(xa, ya, 0.006),
                p(xa, yb, 0.006),
                p(xa + b, yb - b, 0.028),
                p(xa + b, ya + b, 0.028),
            ],
            norm(n + u * 0.6),
            tint(wood, 0.8),
        );
        quad(
            m,
            [
                p(xb, ya, 0.006),
                p(xb, yb, 0.006),
                p(xb - b, yb - b, 0.028),
                p(xb - b, ya + b, 0.028),
            ],
            norm(n - u * 0.6),
            tint(wood, 0.92),
        );
    }
}

/// Walls in a room's own bond, plaster or wainscot on top.
pub fn finished_walls(m: &mut Mesh3D, room: &Room, biome: &Biome, seed: Seed, finish: &Finish) {
    let base = PStyle::of(biome);
    let bond = wall_bond(finish.wall);
    let style = PStyle {
        stone: finish.stone,
        moss_coverage: base.moss_coverage * (0.15 + finish.wear.moss),
        mortar: if bond.mortar {
            Some(tint(finish.mortar, 1.25))
        } else {
            None
        },
        ..base.with(bond.style)
    };
    let area = perimeter(room) * lit(room.height);
    let (bc, relief) = ([lit(bond.cell[0]), lit(bond.cell[1])], lit(bond.relief));
    let scale = (area / (bc[0] * bc[1] * wall_budget(finish.wall) as f64))
        .sqrt()
        .max(1.0);
    let cell = [bc[0] * scale, bc[1] * scale];
    for i in 0..room.outline.len() {
        let (origin, u, n, length) = edge_frame(room, i);
        for (j, r) in wall_regions(room, origin, u, n, length, 0.0)
            .iter()
            .enumerate()
        {
            let [x0, x1, y0, y1] = *r;
            if x1 - x0 < 0.01 || y1 - y0 < 0.01 {
                continue;
            }
            let poly = rect(x0, x1, y0, y1);
            let region_seed = wrap_seed(seed, (i * 13 + j) as u32);
            panel(
                m,
                origin,
                u,
                DY,
                n,
                &poly,
                cell,
                relief,
                &style,
                region_seed,
                bond.masonry,
            );
            if bond.plaster > 0.0 {
                plaster_sheet(
                    m,
                    origin,
                    u,
                    DY,
                    n,
                    &poly,
                    finish.plaster,
                    lit(bond.plaster) + finish.wear.damp as f64 * 0.15,
                    region_seed,
                );
            }
            if bond.wainscot > 0.0 && y0 == 0.0 {
                wainscot(
                    m,
                    origin,
                    u,
                    n,
                    x0,
                    x1,
                    bond.wainscot,
                    finish.wood,
                    region_seed,
                );
            }
        }
    }
}

/// The floor of a finished room: slabs, cobbles, dirt, planks, checker or brick.
pub fn finished_floor(m: &mut Mesh3D, room: &Room, finish: &Finish, seed: Seed) {
    let kind = floor_bond(finish.floor_kind);
    let mut axis = room.sockets.first().map_or(DZ, |s| {
        D3(-s.outward.x as f64, 0.0, -s.outward.z as f64).norm()
    });
    if axis.dot(axis) < 0.5 {
        axis = DZ;
    }
    if kind.ashlar {
        ashlar_floor(m, room, axis, finish.floor, seed, [1.0, 0.62]);
        return;
    }
    let wood = finish.floor_kind == super::finishes::FloorKind::Planks;
    let mut style = PStyle::new(
        if wood { finish.wood } else { finish.floor },
        [0.07, 0.12, 0.025],
        0.25 * finish.wear.moss,
    )
    .with(kind.style);
    if kind.checker {
        style.checker = Some(finish.floor2);
    }
    let origin = room.center;
    let (cell, relief) = ([lit(kind.cell[0]), lit(kind.cell[1])], lit(kind.relief));
    if kind.axis {
        let u = axis;
        let w = D3(-u.2, 0.0, u.0);
        let outline: Vec<P> = room
            .outline
            .iter()
            .map(|p| {
                let q = D3(p[0] as f64, 0.0, p[1] as f64);
                [q.dot(u), q.dot(w)]
            })
            .collect();
        panel(
            m,
            D3::of(origin),
            u,
            w,
            DY,
            &outline,
            cell,
            relief,
            &style,
            seed,
            kind.masonry,
        );
    } else {
        panel(
            m,
            D3::of(origin),
            DX,
            DZ,
            DY,
            &poly64(&room.outline),
            cell,
            relief,
            &style,
            seed,
            kind.masonry,
        );
    }
    if kind.pebbles {
        let mut rng = ArtRng::stream_s(seed, "floor_pebbles_v1", 0);
        for _ in 0..60 {
            let x = rng.range(-room.half[0], room.half[0]);
            let z = rng.range(-room.half[1], room.half[1]);
            let p = origin + v(x, 0.0, z);
            if !room.contains(p.x, p.z, 0.25)
                || room.sockets.iter().any(|sk| {
                    (sk.position.x - p.x).hypot(sk.position.z - p.z) < sk.width * 0.5 + 0.4
                })
            {
                continue;
            }
            let size = rng.range(0.025, 0.07);
            let shade = rng.range(0.7, 1.3);
            let phase = rng.range(0.0, 6.28);
            rock(
                m,
                p,
                v(size * 1.3, size * 0.7, size),
                tint(finish.floor, shade),
                phase,
            );
        }
    }
}

/// Ceiling of a finished room: rock, a coursed vault, timber or coffers.
pub fn finished_ceiling(m: &mut Mesh3D, room: &Room, biome: &Biome, seed: Seed, finish: &Finish) {
    let base = PStyle::of(biome);
    let kind = ceiling_bond(finish.ceiling);
    let mut s = PStyle {
        stone: if kind.wood {
            finish.wood
        } else {
            tint(finish.stone, 0.86)
        },
        moss_coverage: if finish.wear.moss > 0.6 { 0.15 } else { 0.0 },
        ..base
    };
    if let Some(bs) = kind.style {
        s = s.with(bs);
    }
    let top = room.center + Y(room.height);
    let top64 = D3(
        room.center.x as f64,
        room.center.y as f64 + lit(room.height),
        room.center.z as f64,
    );
    panel(
        m,
        top64,
        DX,
        DZ,
        DDOWN,
        &poly64(&room.outline),
        [lit(kind.cell[0]), lit(kind.cell[1])],
        lit(kind.relief),
        &s,
        seed,
        kind.masonry,
    );
    if kind.beams {
        // Heavy joists across the short span, just under the boards.
        let along_x = room.half[0] <= room.half[1];
        let span = if along_x { room.half[0] } else { room.half[1] };
        let long = if along_x { room.half[1] } else { room.half[0] };
        let mut rng = ArtRng::stream_s(seed, "ceiling_beams_v1", 0);
        let mut t = -long + 0.9;
        while t <= long - 0.9 {
            let (a, b) = if along_x {
                (top + v(-span, -0.16, t), top + v(span, -0.16, t))
            } else {
                (top + v(t, -0.16, -span), top + v(t, -0.16, span))
            };
            let shade = rng.range(0.7, 0.9);
            bar(m, a, b, 0.15, 0.1, VY, tint(finish.wood, shade));
            t += 1.35;
        }
    }
}

/// Free-standing column in a room's finish.
fn finished_pillar(
    m: &mut Mesh3D,
    position: Vec3,
    half: [f32; 2],
    height: f32,
    seed: Seed,
    finish: &Finish,
) {
    let mut rng = ArtRng::stream_s(seed, "finished_pillar_v1", 0);
    let w = half[0].min(half[1]);
    let stone = finish.stone;
    match finish.pillar {
        PillarKind::Rough => {
            let mut y = 0.0;
            while y < height + 0.3 {
                let h = rng.range(0.34, 0.6);
                let yaw = rng.range(-0.3, 0.3);
                let x = v(yaw.cos(), 0.0, yaw.sin());
                let z = v(-x.z, 0.0, x.x);
                let k = if y < 0.4 { 1.0 } else { rng.range(0.8, 0.96) };
                let kz = rng.range(0.85, 1.0);
                let bevel = rng.range(0.04, 0.09);
                let shade = rng.range(0.7, 1.05);
                chamfer_box(
                    m,
                    position + Y(y + h * 0.5),
                    x,
                    VY,
                    z,
                    v(w * k, h * 0.5, w * k * kz),
                    bevel,
                    tint(stone, shade),
                );
                y += h - 0.02;
            }
            for _ in 0..5 {
                let a = rng.range(0.0, 6.28);
                let shade = rng.range(0.6, 0.9);
                rock(
                    m,
                    position + v(a.cos() * w * 1.05, 0.0, a.sin() * w * 1.05),
                    v(0.12, 0.09, 0.1),
                    tint(stone, shade),
                    a,
                );
            }
        }
        PillarKind::Timber => {
            chamfer_box(
                m,
                position + Y(0.18),
                VX,
                VY,
                VZ,
                v(w * 0.85, 0.18, w * 0.85),
                0.04,
                tint(stone, 0.8),
            );
            let post = w * 0.5;
            let shade = rng.range(0.85, 1.05);
            chamfer_box(
                m,
                position + Y((height + 0.36) * 0.5 + 0.18),
                VX,
                VY,
                VZ,
                v(post, (height - 0.36) * 0.5 + 0.18, post),
                0.02,
                tint(finish.wood, shade),
            );
            for y in [0.9, height - 0.8] {
                chamfer_box(
                    m,
                    position + Y(y),
                    VX,
                    VY,
                    VZ,
                    v(post + 0.012, 0.03, post + 0.012),
                    0.005,
                    [0.12, 0.11, 0.1],
                );
            }
            for (dx, dz) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                let d = v(dx, 0.0, dz);
                bar(
                    m,
                    position + d * post + Y(height - 0.75),
                    position + d * (w * 1.6) + Y(height + 0.05),
                    0.05,
                    0.05,
                    VY,
                    tint(finish.wood, 0.8),
                );
            }
        }
        PillarKind::Brick => {
            let courses = ((height + 0.3) / 0.2).ceil() as usize;
            for c in 0..courses {
                let y0 = c as f32 * 0.2;
                let k = if c % 2 == 1 { 0.97 } else { 1.0 };
                let shade = rng.range(0.72, 1.12);
                chamfer_box(
                    m,
                    position + Y(y0 + 0.1),
                    VX,
                    VY,
                    VZ,
                    v(w * k, 0.088, w * (2.0 - k) * 0.99),
                    0.012,
                    tint(stone, shade),
                );
            }
            chamfer_box(
                m,
                position + Y(height * 0.5),
                VX,
                VY,
                VZ,
                v(w * 0.93, height * 0.5 + 0.2, w * 0.93),
                0.01,
                tint(finish.mortar, 1.2),
            );
        }
        PillarKind::Cluster => {
            chamfer_box(
                m,
                position + Y(0.16),
                VX,
                VY,
                VZ,
                v(w * 1.02, 0.16, w * 1.02),
                0.03,
                tint(stone, 0.78),
            );
            chamfer_box(
                m,
                position + Y(height * 0.5),
                VX,
                VY,
                VZ,
                v(w * 0.45, height * 0.5, w * 0.45),
                0.02,
                tint(stone, 0.72),
            );
            for (dx, dz) in [(1.0f32, 1.0f32), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                let c = position + v(dx * w * 0.52, 0.0, dz * w * 0.52);
                let shade = rng.range(0.95, 1.1);
                lathe(
                    m,
                    c + Y(0.32),
                    VY,
                    &[
                        (0.0, w * 0.34),
                        (0.06, w * 0.3),
                        (height - 0.9, w * 0.26),
                        (height - 0.82, w * 0.34),
                    ],
                    8,
                    tint(stone, shade),
                    true,
                );
            }
            for dx in [-1.0f32, 1.0] {
                lathe(
                    m,
                    position + v(dx * w * 0.72, 0.32, 0.0),
                    VY,
                    &[(0.0, w * 0.2), (height - 0.85, w * 0.18)],
                    7,
                    tint(stone, 1.05),
                    true,
                );
            }
            chamfer_box(
                m,
                position + Y(height - 0.62),
                VX,
                VY,
                VZ,
                v(w * 1.05, 0.12, w * 1.05),
                0.04,
                tint(stone, 1.14),
            );
            chamfer_box(
                m,
                position + Y(height - 0.2),
                VX,
                VY,
                VZ,
                v(w * 0.95, 0.35, w * 0.95),
                0.03,
                tint(stone, 0.9),
            );
        }
        PillarKind::Ornate => ornate_pier(m, position, half, height, stone, finish.accent, seed),
        PillarKind::Octagon => plain_pier(m, position, half, height, stone, seed),
    }
}
