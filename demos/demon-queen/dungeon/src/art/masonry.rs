//! Baked masonry architecture (web prototype `architecture.js`): pilasters,
//! vault ribs spanning the room, blind wall arches, plinth and cornice bands
//! and a ridge rib with carved bosses, in the room's own finish; the
//! treasury's chunky piers and the sanctuary's lintelled nave.
use super::super::architecture::{layout, Layout, Pilaster as Pil, PROJECTION};
use super::finishes::{Bands, BayKind, Finish, Pilaster, Rib};
use super::kit::*;
use super::surfaces::{is_arcane_hall, is_grand_treasury, is_loot_hall, is_sanctuary_hall, ornate};
use super::vaults::{PIER_DEPTH, PIER_HALF};
use std::f32::consts::PI;
use velvet_dungeon::Room;
use velvet_math::Vec3;
use velvet_render::Mesh3D;

fn edge(room: &Room, i: usize) -> (Vec3, f32, Vec3, Vec3) {
    let a = room.outline[i];
    let b = room.outline[(i + 1) % room.outline.len()];
    let start = v(room.center.x + a[0], room.center.y, room.center.z + a[1]);
    let delta = v(b[0] - a[0], 0.0, b[1] - a[1]);
    let tangent = norm(delta);
    (
        start,
        delta.length(),
        tangent,
        v(-tangent.z, 0.0, tangent.x),
    )
}

/// Upright block against the wall, 12 cm behind it to hide the seam.
#[allow(clippy::too_many_arguments)]
fn wall_block(m: &mut Mesh3D, p: &Pil, y0: f32, y1: f32, half_width: f32, depth: f32, color: C3) {
    let back = 0.12;
    let c = p.base + p.inward * ((depth - back) * 0.5) + Y((y0 + y1) * 0.5);
    block(
        m,
        c,
        p.tangent,
        VY,
        p.inward,
        v(half_width, (y1 - y0) * 0.5, (depth + back) * 0.5),
        color,
    );
}

/// Arc of voussoirs from `a` to `b`, rising `rise`, depth along `across`.
#[allow(clippy::too_many_arguments)]
fn arch(
    m: &mut Mesh3D,
    a: Vec3,
    b: Vec3,
    rise: f32,
    across: Vec3,
    thickness: f32,
    half_depth: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let span = hz(b - a);
    let half = span.length() * 0.5;
    if half < 0.3 {
        return;
    }
    let dir = norm(span);
    let mid = (a + b) * 0.5;
    let pieces = ((((half * 2.0 + rise) / 0.62).ceil() as usize).max(5)) | 1;
    let at = |t: f32| {
        let theta = PI * t;
        mid - dir * (theta.cos() * half) + Y(theta.sin() * rise)
    };
    for k in 0..pieces {
        let p0 = at(k as f32 / pieces as f32);
        let p1 = at((k + 1) as f32 / pieces as f32);
        let tangent = norm(p1 - p0);
        let mut radial = norm(tangent.cross(across));
        if radial.y < 0.0 {
            radial = -radial;
        }
        let keystone = k == pieces / 2;
        let length = (p1 - p0).length() * 0.5 - 0.012;
        let thick = thickness * if keystone { 1.22 } else { 1.0 };
        let shade = if keystone { 1.28 } else { rng.range(0.9, 1.08) };
        block(
            m,
            (p0 + p1) * 0.5 + radial * (thick * 0.5),
            tangent,
            radial,
            across,
            v(
                length,
                thick * 0.5,
                half_depth * if keystone { 1.15 } else { 1.0 },
            ),
            tint(stone, shade),
        );
    }
}

/// Chunky block against the wall: `depth` from the wall line into the room.
#[allow(clippy::too_many_arguments)]
pub fn wall_chamfer(
    m: &mut Mesh3D,
    p: &Pil,
    y0: f32,
    y1: f32,
    half_width: f32,
    depth: f32,
    bevel: f32,
    color: C3,
) {
    let back = 0.12;
    let c = p.base + p.inward * ((depth - back) * 0.5) + Y((y0 + y1) * 0.5);
    chamfer_box(
        m,
        c,
        p.tangent,
        VY,
        p.inward,
        v(half_width, (y1 - y0) * 0.5, (depth + back) * 0.5),
        bevel,
        color,
    );
}

/// Carved diamond: a raised lozenge frame around a darker recessed core.
pub fn diamond(m: &mut Mesh3D, c: Vec3, t: Vec3, n: Vec3, size: f32, stone: C3) {
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
    chamfer_box(
        m,
        c + n * 0.05,
        d1,
        d2,
        n,
        v(size * 0.22, size * 0.22, 0.03),
        0.01,
        tint(stone, 1.2),
    );
}

/// Massive pier: plinth, stacked chunky courses, carved diamond, capital.
fn treasury_pier(m: &mut Mesh3D, p: &Pil, spring: f32, stone: C3, rng: &mut ArtRng, carved: bool) {
    wall_chamfer(
        m,
        p,
        0.0,
        0.42,
        PIER_HALF + 0.12,
        PIER_DEPTH + 0.12,
        0.07,
        tint(stone, 0.8),
    );
    let top = spring - 0.62;
    let courses = (((top - 0.42) / 0.6).round() as usize).max(2);
    let course = (top - 0.42) / courses as f32;
    for c in 0..courses {
        let y0 = 0.42 + course * c as f32;
        let wide = if c % 2 == 0 { 0.03 } else { -0.01 };
        let shade = rng.range(0.84, 1.06);
        wall_chamfer(
            m,
            p,
            y0 + 0.012,
            y0 + course - 0.012,
            PIER_HALF + wide,
            PIER_DEPTH + wide * 0.5,
            0.06,
            tint(stone, shade),
        );
    }
    if carved {
        diamond(
            m,
            p.base + p.inward * (PIER_DEPTH + 0.02) + Y(top - course * 0.5),
            p.tangent,
            p.inward,
            0.15,
            stone,
        );
    }
    wall_chamfer(
        m,
        p,
        top,
        top + 0.16,
        PIER_HALF + 0.1,
        PIER_DEPTH + 0.08,
        0.05,
        tint(stone, 1.12),
    );
    wall_chamfer(
        m,
        p,
        top + 0.16,
        top + 0.4,
        PIER_HALF + 0.04,
        PIER_DEPTH + 0.03,
        0.05,
        tint(stone, 0.9),
    );
    wall_chamfer(
        m,
        p,
        top + 0.4,
        spring,
        PIER_HALF + 0.16,
        PIER_DEPTH + 0.14,
        0.06,
        tint(stone, 1.16),
    );
}

/// Round arch of chunky voussoirs from `a` to `b` rising `rise`.
#[allow(clippy::too_many_arguments)]
pub fn chunky_arch(
    m: &mut Mesh3D,
    a: Vec3,
    b: Vec3,
    rise: f32,
    across: Vec3,
    thickness: f32,
    half_depth: f32,
    stone: C3,
    rng: &mut ArtRng,
) {
    let span = hz(b - a);
    let half = span.length() * 0.5;
    if half < 0.3 {
        return;
    }
    let dir = norm(span);
    let mid = (a + b) * 0.5;
    let pieces = ((((half * 2.0 + rise) / 0.55).ceil() as usize).max(5)) | 1;
    let at = |t: f32| mid - dir * ((PI * t).cos() * half) + Y((PI * t).sin() * rise);
    for k in 0..pieces {
        let p0 = at(k as f32 / pieces as f32);
        let p1 = at((k + 1) as f32 / pieces as f32);
        let tangent = norm(p1 - p0);
        let mut radial = norm(tangent.cross(across));
        if radial.y < 0.0 {
            radial = -radial;
        }
        let keystone = k == pieces / 2;
        let thick = thickness
            * if keystone {
                1.22
            } else {
                rng.range(0.98, 1.02)
            };
        let shade = if keystone {
            1.16
        } else {
            rng.range(0.93, 1.03)
        };
        chamfer_box(
            m,
            (p0 + p1) * 0.5 + radial * (thick * 0.5),
            tangent,
            radial,
            across,
            v(
                (p1 - p0).length() * 0.5 - 0.012,
                thick * 0.5,
                half_depth * if keystone { 1.1 } else { 1.0 },
            ),
            0.03,
            tint(stone, shade),
        );
    }
}

/// Treasury architecture: piers, pier-to-pier arches and vault ribs.
fn treasury_architecture(m: &mut Mesh3D, room: &Room, plan: &Layout, stone: C3, rng: &mut ArtRng) {
    let spring = plan.spring;
    // Even piers carry a torch; odd piers a carved diamond.
    for (k, p) in plan.pilasters.iter().enumerate() {
        treasury_pier(m, p, spring, stone, rng, k % 2 == 1);
    }
    let rise = (room.height - spring - 0.1).max(0.4);
    for &(a, b) in &plan.ribs {
        let d = norm(hz(b - a));
        let from = a + d * (PIER_DEPTH - PROJECTION);
        let to = b - d * (PIER_DEPTH - PROJECTION);
        chunky_arch(
            m,
            from,
            to,
            rise,
            v(-d.z, 0.0, d.x),
            0.44,
            0.3,
            tint(stone, 1.02),
            rng,
        );
    }
    for bay in &plan.bays {
        let half = bay.width * 0.5 - PIER_HALF - 0.02;
        if half < 0.4 {
            continue;
        }
        let bay_rise = half.min(room.height - spring - 0.18).max(0.3);
        let a = bay.center - bay.tangent * half + bay.inward * 0.12 + Y(spring);
        let b = bay.center + bay.tangent * half + bay.inward * 0.12 + Y(spring);
        chunky_arch(
            m,
            a,
            b,
            bay_rise,
            bay.inward,
            0.3,
            0.18,
            tint(stone, 0.94),
            rng,
        );
    }
}

/// Sanctuary nave: plain massive piers joined by lintel beams on dentils.
fn sanctuary_architecture(m: &mut Mesh3D, room: &Room, plan: &Layout, stone: C3, rng: &mut ArtRng) {
    let spring = plan.spring;
    for p in &plan.pilasters {
        treasury_pier(m, p, spring, stone, rng, false);
    }
    for bay in &plan.bays {
        let half = bay.width * 0.5 - PIER_HALF + 0.08;
        if half < 0.3 {
            continue;
        }
        let c = bay.center + bay.inward * ((0.34 - 0.12) * 0.5);
        chamfer_box(
            m,
            c + Y(spring - 0.16),
            bay.tangent,
            VY,
            bay.inward,
            v(half, 0.18, (0.34 + 0.12) * 0.5),
            0.04,
            tint(stone, 1.06),
        );
        let dentils = ((half * 2.0 / 0.32).floor() as usize).max(3);
        for k in 0..dentils {
            let along = -half + (k as f32 + 0.5) * (half * 2.0 / dentils as f32);
            let shade = rng.range(0.82, 0.95);
            chamfer_box(
                m,
                bay.center + bay.tangent * along + bay.inward * 0.1 + Y(spring - 0.4),
                bay.tangent,
                VY,
                bay.inward,
                v(0.07, 0.06, 0.12),
                0.015,
                tint(stone, shade),
            );
        }
    }
    wall_bands(m, room, spring, stone, false);
}

/// Append pilasters, ribs and blind arches to a room mesh.
pub fn append_architecture(
    m: &mut Mesh3D,
    room: &Room,
    stone: C3,
    seed: u64,
    finish: Option<&Finish>,
) {
    let plan = layout(room);
    if plan.pilasters.is_empty() || is_grand_treasury(room) || is_arcane_hall(room) {
        return;
    }
    if let Some(f) = finish {
        if !is_sanctuary_hall(room) && !is_loot_hall(room) {
            finished_architecture(m, room, &plan, f, seed);
            return;
        }
    }
    if is_sanctuary_hall(room) {
        let mut rng = ArtRng::stream(seed, "sanctuary_architecture_v1", room.id as u64);
        sanctuary_architecture(m, room, &plan, stone, &mut rng);
        return;
    }
    if is_loot_hall(room) {
        let mut rng = ArtRng::stream(seed, "treasury_architecture_v1", room.id as u64);
        treasury_architecture(m, room, &plan, stone, &mut rng);
        wall_bands(m, room, plan.spring, stone, false);
        return;
    }
    let mut rng = ArtRng::stream(seed, "architecture_v1", room.id as u64);
    let spring = plan.spring;
    let accent = ornate(room);
    for p in &plan.pilasters {
        if accent.is_some() {
            ornate_pilaster(m, p, spring, stone, &mut rng);
            continue;
        }
        block_pilaster(m, p, spring, stone, &mut rng);
    }
    let rise = (room.height - spring - 0.12).max(0.4);
    for &(a, b) in &plan.ribs {
        let d = norm(hz(b - a));
        arch(
            m,
            a,
            b,
            rise,
            v(-d.z, 0.0, d.x),
            0.34,
            0.24,
            tint(stone, 1.04),
            &mut rng,
        );
    }
    for bay in &plan.bays {
        let half = bay.width * 0.5 - 0.3;
        let bay_rise = (half * 0.62).min(room.height - spring - 0.2).max(0.25);
        let a = bay.center - bay.tangent * half + bay.inward * 0.03 + Y(spring);
        let b = bay.center + bay.tangent * half + bay.inward * 0.03 + Y(spring);
        arch(
            m,
            a,
            b,
            bay_rise,
            bay.inward,
            0.24,
            0.1,
            tint(stone, 0.96),
            &mut rng,
        );
    }
    if let Some(accent) = accent {
        wall_bands(m, room, spring, stone, false);
        ridge(m, room, &plan, stone, accent);
    }
}

/// Classical pilaster: base with torus, reeded shaft, flared capital.
pub fn ornate_pilaster(m: &mut Mesh3D, p: &Pil, spring: f32, stone: C3, rng: &mut ArtRng) {
    let dark = tint(stone, 0.72);
    let mid = tint(stone, 0.95);
    let light = tint(stone, 1.16);
    wall_block(m, p, 0.0, 0.26, 0.44, PROJECTION + 0.06, dark);
    wall_block(m, p, 0.26, 0.34, 0.41, PROJECTION + 0.05, light);
    wall_block(m, p, 0.34, 0.37, 0.36, PROJECTION + 0.02, mid);
    wall_block(m, p, 0.37, 0.43, 0.33, PROJECTION, light);
    let top = spring - 0.52;
    wall_block(m, p, 0.43, top, 0.3, PROJECTION - 0.06, tint(stone, 0.62));
    for offset in [-0.2, -0.068, 0.068, 0.2] {
        let c = p.base
            + p.tangent * offset
            + p.inward * ((PROJECTION - 0.02 - 0.12) * 0.5)
            + Y((0.43 + top) * 0.5);
        let shade = rng.range(0.96, 1.08);
        block(
            m,
            c,
            p.tangent,
            VY,
            p.inward,
            v(0.052, (top - 0.43) * 0.5, (PROJECTION - 0.02 + 0.12) * 0.5),
            tint(stone, shade),
        );
    }
    wall_block(m, p, top, top + 0.06, 0.31, PROJECTION, light);
    wall_block(m, p, top + 0.06, top + 0.1, 0.29, PROJECTION - 0.02, dark);
    wall_block(m, p, top + 0.1, top + 0.26, 0.33, PROJECTION + 0.03, mid);
    wall_block(m, p, top + 0.26, top + 0.37, 0.39, PROJECTION + 0.08, light);
    wall_block(m, p, top + 0.37, top + 0.41, 0.36, PROJECTION + 0.06, dark);
    wall_block(
        m,
        p,
        top + 0.41,
        spring,
        0.45,
        PROJECTION + 0.12,
        tint(stone, 1.22),
    );
}

/// Plinth course and moulded cornice along every wall, broken at doors.
pub fn wall_bands(m: &mut Mesh3D, room: &Room, spring: f32, stone: C3, plinth_only: bool) {
    for i in 0..room.outline.len() {
        let (start, length, tangent, inward) = edge(room, i);
        if length < 0.2 {
            continue;
        }
        let mut gaps: Vec<(f32, f32)> = room
            .sockets
            .iter()
            .filter(|s| {
                v(s.position.x - start.x, 0.0, s.position.z - start.z)
                    .dot(inward)
                    .abs()
                    < 0.05
            })
            .map(|s| {
                let at = v(s.position.x - start.x, 0.0, s.position.z - start.z).dot(tangent);
                (at - s.width * 0.5 - 0.5, at + s.width * 0.5 + 0.5)
            })
            .collect();
        gaps.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut spans = vec![];
        let mut cursor = 0.0f32;
        for (g0, g1) in gaps {
            if g0 > cursor + 0.05 {
                spans.push((cursor, g0.min(length)));
            }
            cursor = cursor.max(g1);
        }
        if cursor < length - 0.05 {
            spans.push((cursor, length));
        }
        let floor = room.center.y;
        for (a, b) in spans {
            let mid = start + tangent * ((a + b) * 0.5);
            let half = (b - a) * 0.5;
            let mut band = |y0: f32, y1: f32, depth: f32, color: C3| {
                let back = 0.1;
                block(
                    m,
                    v(mid.x, floor + (y0 + y1) * 0.5, mid.z) + inward * ((depth - back) * 0.5),
                    tangent,
                    VY,
                    inward,
                    v(half, (y1 - y0) * 0.5, (depth + back) * 0.5),
                    color,
                );
            };
            band(0.0, 0.3, 0.07, tint(stone, 0.66));
            band(0.3, 0.36, 0.1, tint(stone, 1.1));
            if plinth_only {
                continue;
            }
            band(spring - 0.12, spring - 0.04, 0.06, tint(stone, 0.8));
            band(spring - 0.04, spring + 0.08, 0.12, tint(stone, 1.12));
            band(spring + 0.08, spring + 0.16, 0.18, tint(stone, 0.9));
            band(spring + 0.16, spring + 0.2, 0.22, tint(stone, 1.2));
        }
    }
}

/// Ridge rib under the vault crown with carved, gilded bosses.
fn ridge(m: &mut Mesh3D, room: &Room, plan: &Layout, stone: C3, accent: C3) {
    let along_z = room.half[0] <= room.half[1];
    let long = if along_z { VZ } else { VX };
    let half_length = if along_z { room.half[1] } else { room.half[0] };
    let y = room.center.y + room.height - 0.2;
    let c = v(room.center.x, y, room.center.z);
    let side = v(long.z, 0.0, -long.x);
    block(
        m,
        c,
        long,
        VY,
        side,
        v(half_length - 0.2, 0.12, 0.16),
        tint(stone, 1.02),
    );
    block(
        m,
        c - Y(0.1),
        long,
        VY,
        side,
        v(half_length - 0.25, 0.04, 0.1),
        tint(stone, 0.8),
    );
    let mut bosses = vec![c];
    for &(a, b) in &plan.ribs {
        let mid = (a + b) * 0.5;
        bosses.push(v(mid.x, y, mid.z));
    }
    for boss in bosses {
        block(
            m,
            boss - Y(0.08),
            long,
            VY,
            side,
            v(0.26, 0.08, 0.26),
            tint(stone, 1.1),
        );
        for k in 0..8 {
            let a = k as f32 * TAU / 8.0;
            let d = long * a.cos() + side * a.sin();
            block(
                m,
                boss - Y(0.2) + d * 0.13,
                d,
                VY,
                v(-d.z, 0.0, d.x),
                v(0.11, 0.035, 0.045),
                tint(accent, if k % 2 == 0 { 1.1 } else { 0.85 }),
            );
        }
        block(
            m,
            boss - Y(0.23),
            long,
            VY,
            side,
            v(0.07, 0.07, 0.07),
            tint(accent, 1.2),
        );
    }
}

/// Pointed (gothic) profile: a cusp at the crown, vertical at the springers.
fn pointed(u: f32) -> f32 {
    (1.0 - u).max(0.0).powf(0.62)
}

/// Arch of voussoirs following y = rise * shape(|x| / half).
#[allow(clippy::too_many_arguments)]
fn profile_arch(
    m: &mut Mesh3D,
    a: Vec3,
    b: Vec3,
    rise: f32,
    across: Vec3,
    thickness: f32,
    half_depth: f32,
    stone: C3,
    rng: &mut ArtRng,
    shape: fn(f32) -> f32,
) {
    let span = hz(b - a);
    let half = span.length() * 0.5;
    if half < 0.3 {
        return;
    }
    let dir = norm(span);
    let mid = (a + b) * 0.5;
    let pieces = ((((half * 2.0 + rise) / 0.55).ceil() as usize).max(7)) | 1;
    let at = |t: f32| {
        let x = -half + 2.0 * half * t;
        mid + dir * x + Y(rise * shape(x.abs() / half))
    };
    for k in 0..pieces {
        let p0 = at(k as f32 / pieces as f32);
        let p1 = at((k + 1) as f32 / pieces as f32);
        let tangent = norm(p1 - p0);
        let mut radial = norm(tangent.cross(across));
        if radial.y < 0.0 {
            radial = -radial;
        }
        let key = k == pieces / 2;
        let shade = if key { 1.26 } else { rng.range(0.9, 1.08) };
        block(
            m,
            (p0 + p1) * 0.5 + radial * (thickness * 0.5),
            tangent,
            radial,
            across,
            v(
                (p1 - p0).length() * 0.5 - 0.012,
                thickness * 0.5 * if key { 1.25 } else { 1.0 },
                half_depth * if key { 1.12 } else { 1.0 },
            ),
            tint(stone, shade),
        );
    }
}

/// The plain coursed pilaster.
fn block_pilaster(m: &mut Mesh3D, p: &Pil, spring: f32, stone: C3, rng: &mut ArtRng) {
    wall_block(m, p, 0.0, 0.34, 0.4, PROJECTION + 0.06, tint(stone, 0.78));
    wall_block(m, p, 0.34, 0.44, 0.34, PROJECTION + 0.02, tint(stone, 1.12));
    let courses = (((spring - 0.8) / 0.62).ceil() as usize).max(1);
    let course = (spring - 0.8) / courses as f32;
    for c in 0..courses {
        let y0 = 0.44 + course * c as f32;
        let shade = rng.range(0.9, 1.06);
        wall_block(
            m,
            p,
            y0 + 0.01,
            y0 + course - 0.01,
            0.27,
            PROJECTION - 0.02,
            tint(stone, shade),
        );
    }
    wall_block(
        m,
        p,
        spring - 0.36,
        spring - 0.22,
        0.33,
        PROJECTION + 0.02,
        tint(stone, 1.12),
    );
    wall_block(
        m,
        p,
        spring - 0.22,
        spring,
        0.4,
        PROJECTION + 0.08,
        tint(stone, 1.2),
    );
}

/// A pilaster in the room's own manner.
fn finished_pilaster(m: &mut Mesh3D, p: &Pil, spring: f32, finish: &Finish, rng: &mut ArtRng) {
    let stone = finish.stone;
    match finish.pilaster {
        Pilaster::None => {}
        Pilaster::Ornate => ornate_pilaster(m, p, spring, stone, rng),
        Pilaster::Rough => {
            // Rough-hewn blocks stacked up the wall, fatter at the foot.
            let mut y = 0.0;
            while y < spring {
                let h = rng.range(0.4, 0.7);
                let w = rng.range(0.28, 0.4) * if y < 0.5 { 1.15 } else { 1.0 };
                let depth = rng.range(PROJECTION - 0.04, PROJECTION + 0.05);
                let shift = rng.range(-0.05, 0.05);
                let c =
                    p.base + p.inward * ((depth - 0.12) * 0.5) + p.tangent * shift + Y(y + h * 0.5);
                let bevel = rng.range(0.04, 0.09);
                let shade = rng.range(0.66, 1.0);
                chamfer_box(
                    m,
                    c,
                    p.tangent,
                    VY,
                    p.inward,
                    v(w, h * 0.5, (depth + 0.12) * 0.5),
                    bevel,
                    tint(stone, shade),
                );
                y += h - 0.03;
            }
        }
        Pilaster::Brick => {
            wall_block(
                m,
                p,
                0.0,
                spring - 0.3,
                0.24,
                PROJECTION - 0.06,
                tint(finish.mortar, 1.25),
            );
            let mut k = 0;
            let mut y = 0.0;
            while y < spring - 0.32 {
                let shade = rng.range(0.72, 1.12);
                wall_block(
                    m,
                    p,
                    y + 0.01,
                    y + 0.19,
                    if k % 2 == 1 { 0.3 } else { 0.26 },
                    PROJECTION - 0.02,
                    tint(stone, shade),
                );
                y += 0.2;
                k += 1;
            }
            wall_block(
                m,
                p,
                spring - 0.3,
                spring - 0.15,
                0.34,
                PROJECTION + 0.02,
                tint(stone, 1.1),
            );
            wall_block(
                m,
                p,
                spring - 0.15,
                spring,
                0.4,
                PROJECTION + 0.06,
                tint(stone, 0.92),
            );
        }
        Pilaster::Cluster => {
            // A bundle of shafts: one full, two slender, on a shared base.
            wall_block(m, p, 0.0, 0.3, 0.42, PROJECTION + 0.06, tint(stone, 0.75));
            let face = p.base + p.inward * 0.02;
            for (off, r) in [(0.0, 0.13), (-0.21, 0.075), (0.21, 0.075)] {
                let shade = rng.range(0.92, 1.08);
                lathe(
                    m,
                    face + p.tangent * off + Y(0.3),
                    VY,
                    &[
                        (0.0, r + 0.025),
                        (0.06, r),
                        (spring - 0.72, r),
                        (spring - 0.66, r + 0.03),
                    ],
                    8,
                    tint(stone, shade),
                    true,
                );
            }
            wall_block(
                m,
                p,
                spring - 0.36,
                spring - 0.22,
                0.36,
                PROJECTION + 0.04,
                tint(stone, 1.14),
            );
            wall_block(
                m,
                p,
                spring - 0.22,
                spring,
                0.42,
                PROJECTION + 0.1,
                tint(stone, 1.2),
            );
        }
        Pilaster::Timber => {
            wall_block(m, p, 0.0, 0.3, 0.3, PROJECTION + 0.02, tint(stone, 0.75));
            let shade = rng.range(0.85, 1.05);
            wall_block(
                m,
                p,
                0.3,
                spring,
                0.14,
                PROJECTION,
                tint(finish.wood, shade),
            );
            let top = p.base + p.inward * (PROJECTION * 0.5) + Y(spring - 0.6);
            bar(
                m,
                top,
                top + p.inward * 0.45 + Y(0.58),
                0.05,
                0.05,
                p.tangent,
                tint(finish.wood, 0.85),
            );
        }
        Pilaster::Block => block_pilaster(m, p, spring, stone, rng),
    }
}

/// Pilasters, bay arches, ribs, bands and ridge in the room's finish.
fn finished_architecture(m: &mut Mesh3D, room: &Room, plan: &Layout, finish: &Finish, seed: u64) {
    let mut rng = ArtRng::stream(seed, "architecture_finish_v1", room.id as u64);
    let stone = finish.stone;
    let spring = plan.spring;
    for p in &plan.pilasters {
        finished_pilaster(m, p, spring, finish, &mut rng);
    }
    let rise = (room.height - spring - 0.12).max(0.4);
    for &(a, b) in &plan.ribs {
        let d = norm(hz(b - a));
        let across = v(-d.z, 0.0, d.x);
        match finish.rib {
            Rib::None => continue,
            Rib::Beam => {
                // A timber tie beam with knee braces down onto the pilasters.
                let shade = rng.range(0.8, 1.0);
                bar(
                    m,
                    a + Y(0.2),
                    b + Y(0.2),
                    0.17,
                    0.12,
                    VY,
                    tint(finish.wood, shade),
                );
                for (end, dir) in [(a, d), (b, -d)] {
                    bar(
                        m,
                        end - Y(0.55),
                        end + dir * 0.6 + Y(0.12),
                        0.06,
                        0.06,
                        across,
                        tint(finish.wood, 0.85),
                    );
                }
            }
            Rib::Pointed => profile_arch(
                m,
                a,
                b,
                rise,
                across,
                0.34,
                0.24,
                tint(stone, 1.04),
                &mut rng,
                pointed,
            ),
            Rib::Round => arch(
                m,
                a,
                b,
                rise,
                across,
                0.34,
                0.24,
                tint(stone, 1.04),
                &mut rng,
            ),
        }
    }
    for bay in &plan.bays {
        if finish.bay == BayKind::None {
            continue;
        }
        let half = bay.width * 0.5 - 0.3;
        let bay_rise = (half * 0.62).min(room.height - spring - 0.2).max(0.25);
        let a = bay.center - bay.tangent * half + bay.inward * 0.03 + Y(spring);
        let b = bay.center + bay.tangent * half + bay.inward * 0.03 + Y(spring);
        match finish.bay {
            BayKind::Lintel => {
                let c = bay.center + bay.inward * 0.02 + Y(spring - 0.12);
                let shade = rng.range(0.9, 1.05);
                let color = if finish.ceiling == super::finishes::CeilingKind::Timber {
                    finish.wood
                } else {
                    stone
                };
                block(
                    m,
                    c,
                    bay.tangent,
                    VY,
                    bay.inward,
                    v(half + 0.32, 0.14, 0.12),
                    tint(color, shade),
                );
            }
            BayKind::Pointed => profile_arch(
                m,
                a,
                b,
                (bay_rise * 1.35).min(room.height - spring - 0.2),
                bay.inward,
                0.22,
                0.1,
                tint(stone, 0.96),
                &mut rng,
                pointed,
            ),
            _ => arch(
                m,
                a,
                b,
                bay_rise,
                bay.inward,
                0.24,
                0.1,
                tint(stone, 0.96),
                &mut rng,
            ),
        }
    }
    match finish.bands {
        Some(Bands::Full) => wall_bands(m, room, spring, stone, false),
        Some(Bands::Plinth) => wall_bands(m, room, spring, stone, true),
        None => {}
    }
    if finish.ridge {
        ridge(m, room, plan, stone, finish.accent);
    }
}
