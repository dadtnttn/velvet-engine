//! Static room and passage meshes (web prototype `chambers.js`): one
//! vertex-coloured batch per room or passage, in the room's finish.
use super::super::architecture::layout;
use super::super::surfaces as styles;
use super::finishes::{finish_of, weather, Door, Finish, Surface};
use super::formations::{formations, wall_torch_for, Formation, Kind};
use super::grand::{
    grand_architecture, grand_walls, hall_beams, ARCANE_FLOOR, ARCANE_STONE, GRAND_FLOOR,
    GRAND_STONE,
};
use super::kit::*;
use super::masonry::{append_architecture, diamond};
use super::plan::Plan;
use super::surfaces::*;
use std::f32::consts::PI;
use velvet_dungeon::{Biome, Blueprint, Catalog, Role, Room, Shape, Socket};
use velvet_math::Vec3;
use velvet_render::Mesh3D;

type P = [f64; 2];

fn clip_axis(poly: &[P], axis: usize, value: f64, greater: bool) -> Vec<P> {
    let mut out = vec![];
    let inside = |p: &P| {
        if greater {
            p[axis] >= value
        } else {
            p[axis] <= value
        }
    };
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if inside(&a) {
            out.push(a);
        }
        if inside(&a) != inside(&b) {
            let t = (value - a[axis]) / (b[axis] - a[axis]);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    out
}

fn wayfinding_stroke(m: &mut Mesh3D, start: Vec3, end: Vec3, width: f32, y: f32, color: C3) {
    let s = with_y(start, y);
    let e = with_y(end, y);
    let segment = e - s;
    let across = norm(v(-segment.z, 0.0, segment.x)) * (width * 0.5);
    quad(
        m,
        [s - across, s + across, e + across, e - across],
        VY,
        color,
    );
}

/// Low chevron inlay pointing toward a critical exit.
fn wayfinding_glyph(m: &mut Mesh3D, centre: Vec3, outward: Vec3, floor: C3, accent: C3) {
    let forward = norm(v(outward.x, 0.0, outward.z));
    if forward.length_squared() < 0.5 {
        return;
    }
    let side = v(-forward.z, 0.0, forward.x);
    let left = centre - side * 0.15 - forward * 0.105;
    let tip = centre + forward * 0.105;
    let right = centre + side * 0.15 - forward * 0.105;
    let shadow = tint(floor, 0.62);
    let inlay = [
        floor[0] * 0.66 + accent[0] * 0.34,
        floor[1] * 0.66 + accent[1] * 0.34,
        floor[2] * 0.66 + accent[2] * 0.34,
    ];
    for (s, e) in [(left, tip), (right, tip)] {
        wayfinding_stroke(m, s, e, 0.06, centre.y - 0.022, shadow);
    }
    for (s, e) in [(left, tip), (right, tip)] {
        wayfinding_stroke(m, s, e, 0.026, centre.y - 0.015, inlay);
    }
}

/// Arched doorway: quoined jambs, moulded imposts, voussoirs with a proud
/// keystone emblem, an archivolt and spandrels up to the opening top.
fn doorway_arch(m: &mut Mesh3D, socket: &Socket, stone: C3, accent: C3, seed: u64) {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    let f = -outward;
    let t = v(-f.z, 0.0, f.x);
    let base = socket.position;
    let hw = socket.width * 0.5;
    let (front, back) = (0.32, 0.1);
    let dc = (front - back) * 0.5;
    let hd = (front + back) * 0.5;
    let spring = socket.height - 0.83;
    let mut rng = ArtRng::stream(seed, "arched_door_v1", socket.connection as u64);
    for sign in [-1.0f32, 1.0] {
        let mut y = 0.0;
        let mut k = 0;
        while y < spring - 0.02 {
            let h = (spring - y).min(0.42);
            let w = if k % 2 == 0 { 0.3 } else { 0.21 };
            let c = base + t * (sign * (hw + w)) + f * dc + Y(y + h * 0.5);
            let shade = rng.range(0.9, 1.08);
            block(
                m,
                c,
                t,
                VY,
                f,
                v(w - 0.012, h * 0.5 - 0.012, hd),
                tint(stone, shade),
            );
            y += h;
            k += 1;
        }
        block(
            m,
            base + t * (sign * (hw + 0.38)) + f * (dc + 0.02) + Y(spring + 0.06),
            t,
            VY,
            f,
            v(0.37, 0.06, hd + 0.03),
            tint(stone, 1.16),
        );
    }
    let s0 = spring + 0.12;
    let rise = socket.height - s0;
    let thickness = 0.44;
    let pieces = 13;
    let at = |theta: f32| base + t * (-theta.cos() * hw) + Y(s0 + theta.sin() * rise);
    let normal_at = |theta: f32| norm(t * (-theta.cos() / hw) + Y(theta.sin() / rise));
    for k in 0..pieces {
        let a0 = PI * k as f32 / pieces as f32;
        let a1 = PI * (k + 1) as f32 / pieces as f32;
        let (p0, p1) = (at(a0), at(a1));
        let tangent = norm(p1 - p0);
        let radial = normal_at((a0 + a1) * 0.5);
        let keystone = k == pieces / 2;
        let thick = thickness * if keystone { 1.28 } else { 1.0 };
        let c = (p0 + p1) * 0.5 + radial * (thick * 0.5) + f * dc;
        let proud = if keystone { 0.05 } else { 0.0 };
        let shade = if keystone { 1.24 } else { rng.range(0.94, 1.1) };
        block(
            m,
            c + f * proud,
            tangent,
            radial,
            f,
            v((p1 - p0).length() * 0.5 - 0.012, thick * 0.5, hd),
            tint(stone, shade),
        );
        let e0 = at(a0) + normal_at(a0) * thickness;
        let e1 = at(a1) + normal_at(a1) * thickness;
        block(
            m,
            (e0 + e1) * 0.5 + radial * 0.05 + f * (dc + 0.04),
            norm(e1 - e0),
            radial,
            f,
            v((e1 - e0).length() * 0.5, 0.05, hd + 0.02),
            tint(stone, 1.14),
        );
        if keystone {
            let face = c + f * (hd + proud + 0.012);
            block(
                m,
                face,
                tangent,
                radial,
                f,
                v(0.09, 0.12, 0.015),
                tint(accent, 1.05),
            );
            block(
                m,
                face + f * 0.012,
                tangent,
                radial,
                f,
                v(0.045, 0.06, 0.012),
                tint(accent, 0.7),
            );
        }
    }
    // Spandrels close the rectangular opening between extrados and lintel.
    let top = socket.height;
    let ex = hw + thickness;
    let ey = rise + thickness;
    let columns = 24;
    let y_at = |x: f32| s0 + ey * (1.0 - (x / ex).powi(2)).max(0.0).sqrt();
    for i in 0..columns {
        let x0 = -hw + socket.width * i as f32 / columns as f32;
        let x1 = -hw + socket.width * (i + 1) as f32 / columns as f32;
        let (y0, y1) = (y_at(x0).min(top), y_at(x1).min(top));
        if y0 >= top - 0.001 && y1 >= top - 0.001 {
            continue;
        }
        let p = |x: f32, y: f32| base + t * x + Y(y) + f * 0.012;
        let color = tint(stone, 0.9);
        quad(m, [p(x0, y0), p(x1, y1), p(x1, top), p(x0, top)], f, color);
        quad(
            m,
            [p(x0, y0), p(x1, y1), p(x1, top), p(x0, top)],
            -f,
            tint(color, 0.7),
        );
    }
}

/// Treasury portal: quoined jambs, a three-stone lintel and a pediment.
fn treasury_doorway(
    m: &mut Mesh3D,
    socket: &Socket,
    stone: C3,
    room_height: f32,
    seed: u64,
    pediment: bool,
) {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    let f = -outward;
    let t = v(-f.z, 0.0, f.x);
    let base = socket.position;
    let hw = socket.width * 0.5;
    let mut rng = ArtRng::stream(seed, "treasury_door_v1", socket.connection as u64);
    let lintel = socket.height - 0.5;
    let piece = |m: &mut Mesh3D,
                 along: f32,
                 y0: f32,
                 y1: f32,
                 half_width: f32,
                 depth: f32,
                 color: C3,
                 bevel: f32| {
        let back = 0.12;
        chamfer_box(
            m,
            base + t * along + f * ((depth - back) * 0.5) + Y((y0 + y1) * 0.5),
            t,
            VY,
            f,
            v(half_width, (y1 - y0) * 0.5, (depth + back) * 0.5),
            bevel,
            color,
        );
    };
    for sign in [-1.0f32, 1.0] {
        let mut y = 0.0;
        let mut k = 0;
        while y < lintel - 0.02 {
            let h = (if k == 0 { 0.5 } else { rng.range(0.46, 0.62) }).min(lintel - y);
            let w = if k % 2 == 0 { 0.34 } else { 0.25 };
            let shade = rng.range(0.86, 1.06);
            piece(
                m,
                sign * (hw + w),
                y + 0.01,
                y + h - 0.01,
                w - 0.012,
                if k % 2 == 0 { 0.34 } else { 0.28 },
                tint(stone, shade),
                0.05,
            );
            y += h;
            k += 1;
        }
    }
    // Three-stone lintel with a taller central key.
    let span = hw + 0.62;
    piece(
        m,
        -(span + hw * 0.34) * 0.5,
        lintel,
        socket.height,
        (span - hw * 0.34) * 0.5 - 0.01,
        0.38,
        tint(stone, 0.96),
        0.05,
    );
    piece(
        m,
        (span + hw * 0.34) * 0.5,
        lintel,
        socket.height,
        (span - hw * 0.34) * 0.5 - 0.01,
        0.38,
        tint(stone, 0.92),
        0.05,
    );
    piece(
        m,
        0.0,
        lintel - 0.06,
        socket.height + 0.1,
        hw * 0.34,
        0.42,
        tint(stone, 1.1),
        0.05,
    );
    // Gabled pediment with a carved diamond, only where the wall is tall enough.
    let rise = if pediment {
        (room_height - socket.height - 0.2).min(0.85)
    } else {
        0.0
    };
    if rise > 0.35 {
        let apex = base + Y(socket.height + rise) + f * 0.1;
        for sign in [-1.0f32, 1.0] {
            let foot = base + t * (sign * span) + Y(socket.height + 0.1) + f * 0.1;
            let dir = norm(apex - foot);
            let up = norm(f.cross(dir));
            let normal = if up.y < 0.0 { -up } else { up };
            chamfer_box(
                m,
                (foot + apex) * 0.5 + normal * 0.1,
                dir,
                normal,
                f,
                v((apex - foot).length() * 0.5 + 0.08, 0.12, 0.22),
                0.05,
                tint(stone, 1.08),
            );
        }
        // Tympanum: a recessed triangle behind the raking stones.
        let left = base + t * (-span + 0.2) + Y(socket.height + 0.1) + f * 0.02;
        let right = base + t * (span - 0.2) + Y(socket.height + 0.1) + f * 0.02;
        let top = base + Y(socket.height + rise - 0.12) + f * 0.02;
        tri(m, left, right, top, f, tint(stone, 0.7));
        diamond(
            m,
            base + Y(socket.height + 0.1 + (rise - 0.12) * 0.42) + f * 0.06,
            t,
            f,
            (rise * 0.26).min(0.2),
            stone,
        );
    }
}

/// A mine-style doorway: timber posts, a heavy cap and knee braces.
fn timber_doorway(m: &mut Mesh3D, socket: &Socket, finish: &Finish, seed: u64) {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    let f = -outward;
    let t = v(-f.z, 0.0, f.x);
    let base = socket.position;
    let hw = socket.width * 0.5;
    let mut rng = ArtRng::stream(seed, "timber_door_v1", socket.connection as u64);
    let wood = finish.wood;
    for sign in [-1.0f32, 1.0] {
        let post = base + t * (sign * (hw + 0.14)) + f * 0.1;
        let shade = rng.range(0.85, 1.05);
        chamfer_box(
            m,
            post + Y(socket.height * 0.5),
            t,
            VY,
            f,
            v(0.12, socket.height * 0.5, 0.12),
            0.02,
            tint(wood, shade),
        );
        chamfer_box(
            m,
            post + Y(0.08),
            t,
            VY,
            f,
            v(0.17, 0.08, 0.17),
            0.02,
            tint(finish.stone, 0.8),
        );
        bar(
            m,
            post + f * 0.02 + Y(socket.height - 0.62),
            base + t * (sign * (hw - 0.4)) + f * 0.12 + Y(socket.height - 0.02),
            0.05,
            0.05,
            f,
            tint(wood, 0.88),
        );
    }
    chamfer_box(
        m,
        base + f * 0.1 + Y(socket.height + 0.1),
        t,
        VY,
        f,
        v(hw + 0.36, 0.12, 0.14),
        0.02,
        tint(wood, 1.1),
    );
    for k in -2..=2 {
        let shade = rng.range(0.6, 0.9);
        chamfer_box(
            m,
            base + t * (k as f32 * (hw + 0.2) / 2.5) + f * 0.06 + Y(socket.height + 0.3),
            t,
            VY,
            f,
            v(0.17, 0.08, 0.1),
            0.02,
            tint(wood, shade),
        );
    }
}

/// A doorway broken through the rock: jagged stones around the opening.
fn rough_doorway(m: &mut Mesh3D, socket: &Socket, finish: &Finish, seed: u64) {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    let f = -outward;
    let t = v(-f.z, 0.0, f.x);
    let base = socket.position;
    let hw = socket.width * 0.5;
    let mut rng = ArtRng::stream(seed, "rough_door_v1", socket.connection as u64);
    for sign in [-1.0f32, 1.0] {
        let mut y = -0.05;
        while y < socket.height {
            let size = rng.range(0.14, 0.26);
            let d = rng.range(0.02, 0.1);
            let shade = rng.range(0.55, 0.95);
            let phase = rng.range(0.0, TAU);
            rock(
                m,
                base + t * (sign * (hw + size * 1.3)) + f * d + Y(y),
                v(size * 1.25, size, size * 1.1),
                tint(finish.stone, shade),
                phase,
            );
            y += rng.range(0.32, 0.5);
        }
    }
    let mut x = -hw - 0.3;
    while x <= hw + 0.3 {
        let size = rng.range(0.14, 0.24);
        let d = rng.range(0.02, 0.1);
        let lift = rng.range(0.0, 0.15);
        let shade = rng.range(0.55, 0.95);
        let phase = rng.range(0.0, TAU);
        rock(
            m,
            base + t * x + f * d + Y(socket.height + lift),
            v(size * 1.3, size, size * 1.1),
            tint(finish.stone, shade),
            phase,
        );
        x += rng.range(0.25, 0.4);
    }
}

/// Coursed posts and a keyed lintel for plain (bridge) rooms.
fn doorway_frame(m: &mut Mesh3D, socket: &Socket, stone: C3, seed: u64) {
    let outward = norm(v(socket.outward.x, 0.0, socket.outward.z));
    let forward = -outward;
    let side = v(-forward.z, 0.0, forward.x);
    let centre = socket.position + forward * 0.12;
    let half_depth = 0.17;
    let post_half = 0.21;
    let post_offset = socket.width * 0.5 + post_half + 0.06;
    let courses = (((socket.height - 0.46) / 0.62).ceil() as usize).max(1);
    let mut rng = ArtRng::stream(seed, "procedural_arch_v1", socket.connection as u64);
    for sign in [-1.0f32, 1.0] {
        for course in 0..courses {
            let y0 = course as f32 * (socket.height - 0.46) / courses as f32;
            let y1 = (course + 1) as f32 * (socket.height - 0.46) / courses as f32;
            let half_height = ((y1 - y0) - 0.035).max(0.12) * 0.5;
            let sx = post_half * rng.range(0.93, 1.04);
            let shade = rng.range(0.88, 1.12);
            oriented_box(
                m,
                centre + side * (sign * post_offset) + Y((y0 + y1) * 0.5 + 0.017),
                side,
                forward,
                sx,
                half_height,
                half_depth,
                tint(stone, shade),
            );
        }
        oriented_box(
            m,
            centre + side * (sign * (socket.width * 0.5 + post_half + 0.13)) + Y(0.13),
            side,
            forward,
            post_half + 0.1,
            0.13,
            half_depth + 0.055,
            tint(stone, 1.18),
        );
    }
    let lintel_bottom = socket.height - 0.46;
    let lintel_height = socket.height - lintel_bottom;
    let count = ((socket.width / 0.58).ceil() as usize).max(3);
    for index in 0..count {
        let offset = ((index as f32 + 0.5) / count as f32 - 0.5) * socket.width;
        let keystone = index == count / 2;
        let shade = if keystone {
            1.32
        } else {
            0.96 + rng.range(-0.08, 0.08)
        };
        oriented_box(
            m,
            centre + side * offset + Y((lintel_bottom + socket.height) * 0.5),
            side,
            forward,
            socket.width / count as f32 * 0.52,
            lintel_height * 0.5,
            half_depth + if keystone { 0.045 } else { 0.0 },
            tint(stone, shade),
        );
    }
}

fn append_feature(m: &mut Mesh3D, feature: &Formation, stone: C3, accent: C3) {
    let mut rng = ArtRng::stream_s(feature.seed, "procedural_feature_v1", 0);
    let face = v(feature.yaw.sin(), 0.0, feature.yaw.cos());
    let side = v(-face.z, 0.0, face.x);
    let (w, h, d) = (feature.size.x, feature.size.y, feature.size.z);
    match feature.feature {
        Kind::Outcrop => {
            for band in 0..3 {
                let level = (band as f32 + 0.5) / 3.0;
                let width = w * rng.range(0.34, 0.47);
                let height = h * rng.range(0.11, 0.17);
                let offset = rng.range(-w * 0.16, w * 0.16);
                let fz = rng.range(-0.035, 0.035);
                let point = feature.position + side * offset + Y(level * h - h * 0.5) + face * fz;
                let depth = d * rng.range(0.3, 0.47);
                let shade = rng.range(0.82, 1.16);
                oriented_box(
                    m,
                    point,
                    side,
                    face,
                    width,
                    height,
                    depth,
                    tint(stone, shade),
                );
            }
        }
        Kind::Stalagmites | Kind::Crystals => {
            let crystals = feature.feature == Kind::Crystals;
            let count = 2 + rng.index(2);
            for index in 0..count {
                let x = (index as f32 - (count - 1) as f32 * 0.5)
                    * w
                    * if crystals { 0.23 } else { 0.24 };
                let height = h * if crystals {
                    rng.range(0.52, 0.82)
                } else {
                    rng.range(0.68, 0.96)
                };
                let r = d * if crystals {
                    rng.range(0.37, 0.59)
                } else {
                    rng.range(0.36, 0.58)
                };
                let radius = r.max(if crystals { 0.08 } else { 0.075 }).min(if crystals {
                    0.21
                } else {
                    0.22
                });
                let base = feature.position + side * x + Y(if crystals { 0.018 } else { 0.015 });
                let shade = if crystals {
                    rng.range(0.82, 1.14)
                } else {
                    rng.range(0.78, 1.12)
                };
                crystal(
                    m,
                    base,
                    radius,
                    height,
                    if crystals {
                        tint(accent, shade)
                    } else {
                        tint(stone, shade)
                    },
                    5,
                );
            }
        }
        Kind::Stalactites => {
            let count = 2 + rng.index(2);
            for index in 0..count {
                let x = (index as f32 - (count - 1) as f32 * 0.5) * w * 0.22;
                let radius = (d * rng.range(0.28, 0.44)).max(0.07).min(0.18);
                let len = h * rng.range(0.72, 1.0);
                crystal(
                    m,
                    feature.position + v(x, h, 0.0),
                    radius,
                    -len,
                    tint(stone, 0.76),
                    5,
                );
            }
        }
    }
}

/// The art biome of a room: the treasury keeps warm sandstone.
pub fn art_biome_index(room: &Room) -> usize {
    if room.role == Role::Treasure && room.shape != Shape::Bridge {
        0
    } else {
        room.biome
    }
}

/// Static mesh of one room: floor, walls, vault, architecture, formations.
pub fn room_mesh(map: &Blueprint, catalog: &Catalog, id: usize, vault: Option<&Plan>) -> Mesh3D {
    let r = &map.rooms[id];
    let treasury = is_treasury(r);
    let grand = is_grand_treasury(r);
    let sanctuary = is_sanctuary_hall(r);
    let arcane = is_arcane_hall(r);
    let raw_biome: &Biome = &catalog.biomes[art_biome_index(r)];
    let biome = styles::styled_biome(raw_biome);
    let nave = if sanctuary {
        Some(sanctuary_stone(raw_biome))
    } else {
        None
    };
    let mut m = empty();
    let mut rng = ArtRng::stream(map.seed, "decoration_v2", id as u64);
    let base = r.center;
    let seed_l = Seed::Big(r.landmark_seed);
    let finish = finish_of(map, r, &catalog.biomes[r.biome]);
    let finish = finish.as_ref();
    if r.shape == Shape::Bridge {
        let xs = [-r.half[0] as f64, -1.8, 1.8, r.half[0] as f64];
        let zs = [-r.half[1] as f64, -1.8, 1.8, r.half[1] as f64];
        for x in 0..3 {
            for z in 0..3 {
                let mut poly: Vec<P> = poly64(&r.outline);
                poly = clip_axis(&poly, 0, xs[x], true);
                poly = clip_axis(&poly, 0, xs[x + 1], false);
                poly = clip_axis(&poly, 1, zs[z], true);
                poly = clip_axis(&poly, 1, zs[z + 1], false);
                let y = if x != 1 && z != 1 {
                    base.y as f64 - 4.0
                } else {
                    base.y as f64
                };
                if poly.len() >= 3 {
                    let seed = rng.next_seed();
                    flagstones(
                        &mut m,
                        D3(base.x as f64, y, base.z as f64),
                        D3(1.0, 0.0, 0.0),
                        D3(0.0, 0.0, 1.0),
                        D3(0.0, 1.0, 0.0),
                        &poly,
                        biome.floor,
                        seed,
                    );
                }
            }
        }
        for sign in [-1.0f32, 1.0] {
            for side in [-1.0f32, 1.0] {
                let a = base + v(sign * 1.8, 0.0, side * 1.8);
                let b = base + v(sign * 1.8, 0.0, side * r.half[1]);
                quad(
                    &mut m,
                    [a, b, b - Y(4.0), a - Y(4.0)],
                    v(sign, 0.0, 0.0),
                    tint(biome.wall, 0.72),
                );
                let a = base + v(side * 1.8, 0.0, sign * 1.8);
                let b = base + v(side * r.half[0], 0.0, sign * 1.8);
                quad(
                    &mut m,
                    [a, b, b - Y(4.0), a - Y(4.0)],
                    v(0.0, 0.0, sign),
                    tint(biome.wall, 0.72),
                );
            }
        }
    } else {
        let seed = rng.next_seed();
        let floor_start = m.vertices.len();
        let entry_axis = || {
            r.sockets.first().map_or(D3(0.0, 0.0, 1.0), |s| {
                D3(-s.outward.x as f64, 0.0, -s.outward.z as f64).norm()
            })
        };
        if arcane {
            ashlar_floor(&mut m, r, entry_axis(), ARCANE_FLOOR, seed, [1.1, 1.05]);
        } else if grand {
            ashlar_floor(&mut m, r, entry_axis(), GRAND_FLOOR, seed, [1.45, 0.95]);
        } else if treasury {
            treasury_floor(&mut m, r, biome.floor, seed);
        } else if let Some((_, floor)) = nave {
            treasury_floor(&mut m, r, floor, seed);
        } else if let Some(f) = finish {
            finished_floor(&mut m, r, f, seed);
        } else if ornate(r).is_some() {
            ashlar_floor(&mut m, r, entry_axis(), biome.floor, seed, [1.0, 0.62]);
        } else {
            flagstones(
                &mut m,
                D3::of(base),
                D3(1.0, 0.0, 0.0),
                D3(0.0, 0.0, 1.0),
                D3(0.0, 1.0, 0.0),
                &poly64(&r.outline),
                biome.floor,
                seed,
            );
        }
        let end = m.vertices.len();
        weather(&mut m, floor_start, end, finish, r, Surface::Floor, &[]);
    }
    let torches: Vec<Vec3> = r
        .sockets
        .iter()
        .filter_map(|s| wall_torch_for(r, s))
        .map(|t| t.flame)
        .collect();
    let ceiling_start = m.vertices.len();
    ceiling(&mut m, r, raw_biome, seed_l, finish);
    let end = m.vertices.len();
    weather(&mut m, ceiling_start, end, finish, r, Surface::Ceiling, &[]);
    let wall_start = m.vertices.len();
    walls(&mut m, r, raw_biome, seed_l, finish);
    if grand {
        grand_walls(&mut m, r, raw_biome, seed_l, GRAND_STONE);
        grand_architecture(&mut m, r, map.seed, GRAND_STONE, false);
    }
    if arcane {
        grand_walls(&mut m, r, raw_biome, seed_l, ARCANE_STONE);
        grand_architecture(&mut m, r, map.seed, ARCANE_STONE, true);
        hall_beams(&mut m, r, map.seed);
    }
    age_dressing(&mut m, r, raw_biome, seed_l, finish);
    append_architecture(
        &mut m,
        r,
        nave.map_or(biome.wall, |n| n.0),
        map.seed,
        finish,
    );
    let accent = ornate(r);
    for s in &r.sockets {
        if grand {
            doorway_arch(
                &mut m,
                s,
                [0.44, 0.42, 0.4],
                accent.unwrap_or([0.9, 0.58, 0.16]),
                map.seed,
            );
        } else if treasury {
            treasury_doorway(&mut m, s, biome.wall, r.height, map.seed, true);
        } else if let Some((stone, _)) = nave {
            treasury_doorway(&mut m, s, stone, r.height, map.seed, false);
        } else if let Some(f) = finish {
            match f.door {
                Door::Timber => timber_doorway(&mut m, s, f, map.seed),
                Door::Rough => rough_doorway(&mut m, s, f, map.seed),
                Door::Frame => doorway_frame(&mut m, s, f.stone, map.seed),
                Door::Arch => doorway_arch(&mut m, s, f.stone, f.accent, map.seed),
            }
        } else if let Some(accent) = accent {
            doorway_arch(
                &mut m,
                s,
                if arcane { ARCANE_STONE } else { biome.wall },
                accent,
                map.seed,
            );
        } else {
            doorway_frame(&mut m, s, biome.wall, map.seed);
        }
        if map.connections[s.connection].critical {
            let outward = v(s.outward.x, 0.0, s.outward.z);
            for i in 0..3 {
                let mut p = s.position - outward * (0.4 + i as f32 * 0.5);
                p = with_y(p, r.floor_y(p.x, p.z));
                wayfinding_glyph(&mut m, p, outward, biome.floor, biome.accent);
            }
        }
    }
    let host = vault.and_then(|p| p.host_pillar);
    let replace = vault.is_some_and(|p| p.replace_pillars);
    for (index, o) in map.obstacles.iter().filter(|o| o.room == id).enumerate() {
        // A crossroads sanctuary replaces one cover pillar with its shrine.
        if host.is_some_and(|h| (h.x - o.center.x).hypot(h.z - o.center.z) < 0.01) || replace {
            continue;
        }
        pillar(
            &mut m,
            o.center,
            o.half,
            o.height,
            raw_biome,
            wrap_seed(seed_l, index as u32),
            accent,
            treasury,
            sanctuary,
            finish,
        );
    }
    let end = m.vertices.len();
    weather(&mut m, wall_start, end, finish, r, Surface::Wall, &torches);
    // Built halls keep their floors free of cave rubble.
    let gallery = super::super::themes::of(map, r).is_some_and(super::theme::is_gallery);
    let built = finish.is_some_and(|f| {
        !matches!(
            f.wall,
            super::finishes::Wall::Rock | super::finishes::Wall::Rubble
        )
    });
    let count = if treasury || arcane || sanctuary || gallery || built {
        0
    } else {
        (18.0 * map.config.decoration_density).floor() as usize
    };
    for _ in 0..count {
        let a = rng.range(0.0, TAU);
        let mut p = base + v(a.cos() * r.half[0] * 0.91, 0.0, a.sin() * r.half[1] * 0.91);
        if !r.contains(p.x, p.z, 0.1)
            || r.sockets
                .iter()
                .any(|s| (s.position.x - p.x).hypot(s.position.z - p.z) < 2.0)
        {
            continue;
        }
        p = with_y(p, r.floor_y(p.x, p.z));
        let radius = rng.range(0.12, 0.4);
        let h = rng.range(0.25, 0.7);
        if r.biome == 1 || rng.chance(0.22) {
            rock(
                &mut m,
                p + Y(0.015),
                v(radius * 1.45, 0.13, radius * 1.28),
                tint(biome.wall, 0.68),
                a,
            );
            for (j, &(x, z, rs, hs)) in [
                (-0.52, -0.18, 0.54, 0.86),
                (0.02, 0.12, 0.88, 1.34),
                (0.58, -0.1, 0.5, 0.72),
            ]
            .iter()
            .enumerate()
            {
                crystal(
                    &mut m,
                    p + v(x * radius, 0.075, z * radius),
                    radius * rs,
                    h * hs,
                    tint(biome.accent, [0.94, 1.08, 0.88][j]),
                    5 + (j % 2),
                );
            }
        } else {
            let shade = rng.range(0.75, 1.12);
            rock(
                &mut m,
                p,
                v(radius, h, radius * 0.9),
                tint(biome.wall, shade),
                a,
            );
        }
        if rng.chance(0.4) {
            crystal(
                &mut m,
                v(p.x, base.y + r.height, p.z),
                radius,
                -h * 2.0,
                tint(biome.wall, 0.67),
                5,
            );
        }
    }
    if let Some(plan) = vault {
        for &p in &plan.obelisks {
            super::vaults::obelisk(&mut m, p, biome.wall);
        }
    }
    let pilasters = layout(r).pilasters;
    let stone = styles::style(raw_biome).stone;
    for feature in formations(map, r) {
        let crowds = feature.feature != Kind::Stalactites
            && pilasters
                .iter()
                .any(|p| hz(p.base - feature.position).length() < feature.size.x * 0.5 + 0.55);
        if !crowds
            && !treasury
            && !arcane
            && !((sanctuary || gallery || built) && feature.feature != Kind::Stalactites)
        {
            append_feature(
                &mut m,
                &feature,
                finish.map_or(stone, |f| f.stone),
                raw_biome.accent,
            );
        }
    }
    m
}

fn stair_riser(
    m: &mut Mesh3D,
    at: Vec3,
    side: Vec3,
    axis: Vec3,
    half_width: f32,
    from_y: f32,
    to_y: f32,
    color: C3,
) {
    if (to_y - from_y).abs() < 0.001 {
        return;
    }
    let flat = with_y(at, 0.0);
    let left = flat - side * half_width;
    let right = flat + side * half_width;
    let (low, high) = (from_y.min(to_y), from_y.max(to_y));
    if to_y > from_y {
        quad(
            m,
            [
                left + Y(low),
                right + Y(low),
                right + Y(high),
                left + Y(high),
            ],
            -axis,
            tint(color, 0.82),
        );
    } else {
        quad(
            m,
            [
                left + Y(high),
                right + Y(high),
                right + Y(low),
                left + Y(low),
            ],
            axis,
            tint(color, 0.82),
        );
    }
}

/// Stone rib following the tunnel cross-section: pilasters, voussoirs, keystone.
fn tunnel_rib(
    m: &mut Mesh3D,
    at: Vec3,
    along: Vec3,
    side: Vec3,
    cross: &[[f32; 2]],
    stone: C3,
    seed: Seed,
) {
    let mut rng = ArtRng::stream_s(seed, "tunnel_rib_v1", 0);
    let point = |c: [f32; 2]| at + side * c[0] + Y(c[1]);
    let centre = at + Y(1.5);
    let (depth, back) = (0.13, 0.12);
    for j in 0..cross.len() - 1 {
        let mut a = point(cross[j]);
        let mut b = point(cross[j + 1]);
        if j == 0 || j == cross.len() - 2 {
            let lift = |p: Vec3| if p.y < at.y { with_y(p, at.y) } else { p };
            a = lift(a);
            b = lift(b);
        }
        let dir = norm(b - a);
        let mut inward = norm(along.cross(dir));
        if inward.dot(centre - (a + b) * 0.5) < 0.0 {
            inward = -inward;
        }
        let pieces = (((b - a).length() / 0.55).ceil() as usize).max(1);
        for k in 0..pieces {
            let p0 = a + (b - a) * (k as f32 / pieces as f32);
            let p1 = a + (b - a) * ((k + 1) as f32 / pieces as f32);
            let keystone = j == cross.len() / 2 - 1 && k == pieces / 2;
            let d = if keystone { depth + 0.03 } else { depth };
            let shade = if keystone { 1.2 } else { rng.range(0.9, 1.1) };
            block(
                m,
                (p0 + p1) * 0.5 + inward * ((d - back) * 0.5),
                dir,
                inward,
                along,
                v(
                    (p1 - p0).length() * 0.5 - 0.01,
                    (d + back) * 0.5,
                    if keystone { 0.24 } else { 0.2 },
                ),
                tint(stone, shade),
            );
        }
    }
    for s in [0, cross.len() - 2] {
        let c = cross[if s == 0 { 1 } else { s }];
        let p = point(c);
        let inward = hz(side * -c[0].signum());
        block(
            m,
            p - Y(0.06) + inward * 0.01,
            along,
            VY,
            inward,
            v(0.26, 0.06, 0.11),
            tint(stone, 1.18),
        );
    }
}

/// Static mesh of one passage: paving or stairs, ashlar walls, vault, ribs.
pub fn corridor_mesh(map: &Blueprint, catalog: &Catalog, id: usize) -> Mesh3D {
    let c = &map.connections[id];
    let a = styles::styled_biome(&catalog.biomes[map.rooms[c.a].biome]);
    let b = styles::styled_biome(&catalog.biomes[map.rooms[c.b].biome]);
    let mut m = empty();
    let (start, end) = (D3::of(c.start), D3::of(c.end));
    let axis = D3(end.0 - start.0, 0.0, end.2 - start.2).norm();
    let side = D3(-axis.2, 0.0, axis.0);
    let width = lit(c.width) * 0.5;
    let height = lit(c.height);
    let length = (end.0 - start.0).hypot(end.2 - start.2);
    let seed = Seed::Big(map.seed);
    let up = D3(0.0, 1.0, 0.0);
    let yv = |h: f64| D3(0.0, h, 0.0);
    for (at, n) in [(start, axis.mul(-1.0)), (end, axis)] {
        for sign in [-1.0f64, 1.0] {
            tri(
                &mut m,
                at.add(side.mul(sign * width)).add(yv(2.65)).f(),
                at.add(side.mul(sign * width)).add(yv(height)).f(),
                at.add(side.mul(sign * width * 0.52)).add(yv(height)).f(),
                n.f(),
                a.wall,
            );
        }
    }
    let floor_color = |t: f64| -> C3 {
        std::array::from_fn(|k| (a.floor[k] as f64 * (1.0 - t) + b.floor[k] as f64 * t) as f32)
    };
    let lerp3 = |t: f64| start.add(end.sub(start).mul(t));
    let with_yd = |p: D3, y: f64| D3(p.0, y, p.2);
    let (sy, ey) = (c.start.y as f64, c.end.y as f64);
    if c.kind == velvet_dungeon::ConnectionKind::Stairs {
        let steps = (((ey - sy).abs() / 0.15).ceil() as usize).max(1);
        let rise = (ey - sy) / steps as f64;
        for i in 0..steps {
            let t0 = i as f64 / steps as f64;
            let t1 = (i + 1) as f64 / steps as f64;
            let level = sy + rise * i as f64;
            let p = with_yd(lerp3(t0), level);
            let color = floor_color((t0 + t1) * 0.5);
            if i > 0 {
                stair_riser(
                    &mut m,
                    p.f(),
                    side.f(),
                    axis.f(),
                    width as f32,
                    (level - rise) as f32,
                    level as f32,
                    color,
                );
            }
            let tread = length * (t1 - t0);
            flagstones(
                &mut m,
                p,
                side,
                axis,
                up,
                &[[-width, 0.0], [width, 0.0], [width, tread], [-width, tread]],
                color,
                wrap_seed(seed, (id * 997 + i) as u32),
            );
        }
        stair_riser(
            &mut m,
            end.f(),
            side.f(),
            axis.f(),
            width as f32,
            (ey - rise) as f32,
            ey as f32,
            floor_color(1.0),
        );
    } else {
        let steps = (length / 1.4).ceil() as usize;
        let mut ts: Vec<f64> = vec![0.0, 1.0];
        for i in 0..steps {
            ts.push(i as f64 / steps as f64);
        }
        ts.sort_by(|x, y| x.total_cmp(y));
        ts.dedup();
        for k in 0..ts.len() - 1 {
            let (t0, t1) = (ts[k], ts[k + 1]);
            let p = with_yd(lerp3(t0), conn_floor_y(c, t0));
            let q = with_yd(lerp3(t1), conn_floor_y(c, t1));
            let forward = q.sub(p).norm();
            let segment = q.sub(p).len();
            let mut normal = side.cross(forward).norm();
            if normal.1 < 0.0 {
                normal = normal.mul(-1.0);
            }
            paving(
                &mut m,
                p,
                side,
                forward,
                normal,
                &[
                    [-width, 0.0],
                    [width, 0.0],
                    [width, segment],
                    [-width, segment],
                ],
                floor_color((t0 + t1) * 0.5),
                wrap_seed(seed, (id * 997) as u32 + (t0 * 100000.0).floor() as u32),
            );
        }
    }
    let strips = ((length / 3.2).ceil() as usize).max(1);
    let cross: [P; 6] = [
        [-width, -0.18],
        [-width, 2.65],
        [-width * 0.52, height],
        [width * 0.52, height],
        [width, 2.65],
        [width, -0.18],
    ];
    for i in 0..strips {
        let p = lerp3(i as f64 / strips as f64);
        let q = lerp3((i + 1) as f64 / strips as f64);
        let t = (i as f64 + 0.5) / strips as f64;
        let mut biome = a.clone();
        biome.wall =
            std::array::from_fn(|k| (a.wall[k] as f64 * (1.0 - t) + b.wall[k] as f64 * t) as f32);
        for j in 0..cross.len() - 1 {
            let (u, w) = (cross[j], cross[j + 1]);
            let pp = p.add(side.mul(u[0])).add(yv(u[1]));
            let pq = p.add(side.mul(w[0])).add(yv(w[1]));
            let qp = q.add(side.mul(u[0])).add(yv(u[1]));
            let qq = q.add(side.mul(w[0])).add(yv(w[1]));
            let centre = pp.add(pq).add(qp).add(qq).mul(0.25);
            let inside = p.add(q).mul(0.5).add(yv(1.5)).sub(centre);
            let mut n = pq.sub(pp).cross(qp.sub(pp)).norm();
            if n.dot(inside) < 0.0 {
                n = n.mul(-1.0);
            }
            let s = wrap_seed(seed, (id * 1009 + i * 7 + j) as u32);
            if j == 0 || j == cross.len() - 2 {
                let (bottom, top) = if pq.sub(pp).1 > 0.0 {
                    (pp, qp)
                } else {
                    (pq, qq)
                };
                tunnel_ashlar(
                    &mut m,
                    bottom,
                    top.sub(bottom),
                    up,
                    n,
                    top.sub(bottom).len(),
                    pq.sub(pp).len(),
                    biome.wall,
                    s,
                );
            } else {
                tunnel_panel(&mut m, [pp, pq, qq, qp], n, &biome, s);
            }
        }
        for sign in [-1.0f64, 1.0] {
            let inward = side.mul(-sign);
            let pa = p.add(side.mul(sign * width)).add(inward.mul(0.03));
            let pb = q.add(side.mul(sign * width)).add(inward.mul(0.03));
            block(
                &mut m,
                pa.add(pb).mul(0.5).add(yv(0.14)).f(),
                pb.sub(pa).norm().f(),
                VY,
                inward.f(),
                v((pb.sub(pa).len() * 0.5) as f32, 0.16, 0.08),
                tint(biome.wall, 0.7),
            );
        }
        if i > 0 {
            let cross32: Vec<[f32; 2]> = cross.iter().map(|c| [c[0] as f32, c[1] as f32]).collect();
            tunnel_rib(
                &mut m,
                p.f(),
                D3(q.0 - p.0, 0.0, q.2 - p.2).norm().f(),
                side.f(),
                &cross32,
                biome.wall,
                wrap_seed(seed, (id * 131 + i) as u32),
            );
        }
    }
    m
}

/// Floor height along a connection (web `connFloorY`).
fn conn_floor_y(c: &velvet_dungeon::Connection, t: f64) -> f64 {
    let k = t.clamp(0.0, 1.0);
    let (sy, ey) = (c.start.y as f64, c.end.y as f64);
    if c.kind != velvet_dungeon::ConnectionKind::Stairs {
        return sy + (ey - sy) * k;
    }
    let n = (((ey - sy).abs() / 0.15).ceil()).max(1.0);
    let step = n.min((k * n + 1e-5).floor());
    sy + (ey - sy) * step / n
}
