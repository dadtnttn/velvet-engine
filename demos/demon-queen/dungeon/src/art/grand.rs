//! The grand treasury hall and the arcane library shell (web prototype
//! `grand.js`): tall grey ashlar walls opened by deep arched niches between
//! square piers, and the piers, arch rings, bands and timber beams.
use super::super::architecture::layout;
use super::arcane::Niche;
use super::kit::*;
use super::masonry::{chunky_arch, ornate_pilaster, wall_chamfer};
use super::surfaces::{panel, quad64, PStyle, D3};
use super::vaults::{grand_niches, PIER_DEPTH, PIER_HALF};
use velvet_dungeon::{Biome, Room};
use velvet_render::Mesh3D;

pub const GRAND_STONE: C3 = [0.4, 0.385, 0.375];
pub const GRAND_FLOOR: C3 = [0.47, 0.43, 0.38];
pub const ARCANE_STONE: C3 = [0.44, 0.405, 0.37];
pub const ARCANE_FLOOR: C3 = [0.45, 0.42, 0.39];
const WOOD: C3 = [0.3, 0.18, 0.1];

/// A niche in full precision (the web layout works in doubles): centre,
/// inward, tangent, half width, springing and top.
#[derive(Clone, Copy)]
struct N64 {
    center: D3,
    inward: D3,
    tangent: D3,
    half: f64,
    spring: f64,
    top: f64,
}

/// The grand hall's niches, laid out exactly as the web `layout` does.
fn niches64(room: &Room) -> Vec<N64> {
    let h = lit(room.height);
    let spring = (h - 1.35).min((h * 0.62).max(3.05)) - 0.25;
    let mut out = vec![];
    if room.shape == velvet_dungeon::Shape::Bridge || room.outline.len() != 8 {
        return out;
    }
    let blocked = |p: D3| {
        room.sockets.iter().any(|s| {
            (s.position.x as f64 - p.0).hypot(s.position.z as f64 - p.2) < lit(s.width) * 0.5 + 0.95
        })
    };
    for i in (0..8).step_by(2) {
        let a = room.outline[i];
        let b = room.outline[(i + 1) % 8];
        let start = D3(
            room.center.x as f64 + a[0] as f64,
            room.center.y as f64,
            room.center.z as f64 + a[1] as f64,
        );
        let delta = D3(b[0] as f64 - a[0] as f64, 0.0, b[1] as f64 - a[1] as f64);
        let length = delta.len();
        let tangent = delta.norm();
        let inward = D3(-tangent.2, 0.0, tangent.0);
        let usable = length - 1.0;
        if usable < 1.5 {
            continue;
        }
        let count = ((usable / 3.2).round() as usize).max(1);
        let step = usable / count as f64;
        let mut previous: Option<D3> = None;
        for k in 0..=count {
            let p = start.add(tangent.mul(0.5 + step * k as f64));
            if blocked(p) {
                previous = None;
                continue;
            }
            if let Some(q) = previous {
                let mid = p.add(q).mul(0.5);
                if !blocked(mid) {
                    let width = D3(p.0 - q.0, 0.0, p.2 - q.2).len();
                    let half = (width * 0.5 - 0.56 - 0.14).min(1.0);
                    if half >= 0.55 && spring + half < h - 0.5 {
                        out.push(N64 {
                            center: D3(mid.0, room.center.y as f64, mid.2),
                            inward,
                            tangent,
                            half,
                            spring,
                            top: spring + half,
                        });
                    }
                }
            }
            previous = Some(p);
        }
    }
    out
}

fn wall_style(biome: &Biome, stone: C3) -> PStyle {
    PStyle {
        stone,
        bevel: Some([0.88, 0.93]),
        face_depth: Some([0.3, 0.5]),
        tone: Some([0.82, 1.1]),
        crack: Some(0.18),
        moss_coverage: 0.02,
        ..PStyle::of(biome)
    }
}

/// Walls with the socket and every niche cut out, plus the niche interiors.
pub fn grand_walls(m: &mut Mesh3D, room: &Room, biome: &Biome, seed: Seed, stone: C3) {
    let s = wall_style(biome, stone);
    let niches = niches64(room);
    let up = D3(0.0, 1.0, 0.0);
    for i in 0..room.outline.len() {
        let p = room.outline[i];
        let q = room.outline[(i + 1) % room.outline.len()];
        let origin = D3(
            room.center.x as f64 + p[0] as f64,
            room.center.y as f64,
            room.center.z as f64 + p[1] as f64,
        );
        let edge = D3(q[0] as f64 - p[0] as f64, 0.0, q[1] as f64 - p[1] as f64);
        let length = edge.len();
        let u = edge.norm();
        let n = D3(-u.2, 0.0, u.0);
        let mut cuts: Vec<(f64, f64, f64)> = vec![];
        for socket in &room.sockets {
            let d = D3(
                socket.position.x as f64 - origin.0,
                0.0,
                socket.position.z as f64 - origin.2,
            );
            if d.dot(n).abs() < 0.01 && d.dot(u) >= 0.0 && d.dot(u) <= length {
                cuts.push((
                    d.dot(u) - lit(socket.width) * 0.5,
                    d.dot(u) + lit(socket.width) * 0.5,
                    lit(socket.height),
                ));
            }
        }
        for niche in &niches {
            let d = D3(niche.center.0 - origin.0, 0.0, niche.center.2 - origin.2);
            if d.dot(n).abs() > 0.05 || d.dot(u) < 0.0 || d.dot(u) > length {
                continue;
            }
            cuts.push((d.dot(u) - niche.half, d.dot(u) + niche.half, niche.top));
        }
        cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let h = lit(room.height);
        let mut regions = vec![];
        let mut cursor = 0.0f64;
        for (c0, c1, top) in cuts {
            if c0 > cursor + 0.01 {
                regions.push([cursor, c0, 0.0, h]);
            }
            regions.push([c0, c1, top, h]);
            cursor = c1;
        }
        if cursor < length - 0.01 {
            regions.push([cursor, length, 0.0, h]);
        }
        for (j, &[x0, x1, y0, y1]) in regions.iter().enumerate() {
            if x1 - x0 < 0.01 || y1 - y0 < 0.01 {
                continue;
            }
            panel(
                m,
                origin,
                u,
                up,
                n,
                &[[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
                [1.1, 0.55],
                0.26,
                &s,
                wrap_seed(seed, (i * 13 + j) as u32),
                true,
            );
        }
    }
    for (k, niche) in niches.iter().enumerate() {
        niche_interior(m, niche, &s, wrap_seed(seed, 700 + k as u32));
    }
}

/// Back wall, side walls, barrel vault, floor and spandrels of one niche.
fn niche_interior(m: &mut Mesh3D, niche: &N64, style: &PStyle, seed: Seed) {
    let (r, sp, top, d) = (
        niche.half,
        niche.spring,
        niche.top,
        lit(super::vaults::NICHE_DEPTH),
    );
    let (c, t, nn) = (niche.center, niche.tangent, niche.inward);
    let back = c.sub(nn.mul(d));
    let inner = PStyle {
        stone: tint(style.stone, 0.78),
        ..*style
    };
    let up = D3(0.0, 1.0, 0.0);
    panel(
        m,
        back.sub(t.mul(r)),
        t,
        up,
        nn,
        &[[0.0, 0.0], [2.0 * r, 0.0], [2.0 * r, top], [0.0, top]],
        [0.62, 0.36],
        0.1,
        &inner,
        seed,
        true,
    );
    for sign in [-1.0f64, 1.0] {
        panel(
            m,
            c.add(t.mul(sign * r)),
            nn.mul(-1.0),
            up,
            t.mul(-sign),
            &[[0.0, 0.0], [d, 0.0], [d, sp], [0.0, sp]],
            [0.55, 0.4],
            0.08,
            &inner,
            wrap_seed(seed, if sign > 0.0 { 1 } else { 2 }),
            true,
        );
    }
    let segments = 14;
    let at = |theta: f64, dd: f64| {
        c.sub(nn.mul(dd))
            .add(t.mul(-theta.cos() * r))
            .add(up.mul(sp + theta.sin() * r))
    };
    for k in 0..segments {
        let a0 = std::f64::consts::PI * k as f64 / segments as f64;
        let a1 = std::f64::consts::PI * (k + 1) as f64 / segments as f64;
        let mid = (a0 + a1) * 0.5;
        let normal = t.mul(mid.cos()).add(D3(0.0, -mid.sin(), 0.0)).norm().f();
        quad64(
            m,
            [at(a0, 0.0), at(a1, 0.0), at(a1, d), at(a0, d)],
            normal,
            tint(style.stone, 0.55 + (k % 2) as f32 * 0.06),
        );
    }
    let y = c.1 - 0.004;
    let flat = |p: D3| D3(p.0, y, p.2);
    quad64(
        m,
        [
            flat(c.sub(t.mul(r))),
            flat(c.add(t.mul(r))),
            flat(back.add(t.mul(r))),
            flat(back.sub(t.mul(r))),
        ],
        VY,
        tint(style.stone, 0.72),
    );
    // Spandrels close the rectangular cut between the arch and its top.
    let columns = 16;
    for i in 0..columns {
        let x0 = -r + 2.0 * r * i as f64 / columns as f64;
        let x1 = -r + 2.0 * r * (i + 1) as f64 / columns as f64;
        let y0 = sp + (r * r - x0 * x0).max(0.0).sqrt();
        let y1 = sp + (r * r - x1 * x1).max(0.0).sqrt();
        let p = |x: f64, yy: f64| c.add(t.mul(x)).add(up.mul(yy));
        quad64(
            m,
            [p(x0, y0), p(x1, y1), p(x1, top), p(x0, top)],
            nn.f(),
            tint(style.stone, 0.92),
        );
    }
}

/// Square piers, the niche arch rings and moulded bands of the hall.
pub fn grand_architecture(m: &mut Mesh3D, room: &Room, seed: u64, stone: C3, fluted: bool) {
    let plan = layout(room);
    let mut rng = ArtRng::stream(seed, "grand_treasury_architecture_v1", room.id as u64);
    let niches = grand_niches(room);
    let s = niches.first().map_or(plan.spring - 0.25, |n| n.spring);
    for p in &plan.pilasters {
        if fluted {
            // Fluted classical pilaster to the niche springing, plain above.
            ornate_pilaster(m, p, s + 0.22, stone, &mut rng);
            let upper = room.height - 0.5;
            wall_chamfer(m, p, s + 0.22, upper, 0.3, 0.2, 0.025, tint(stone, 0.95));
            wall_chamfer(
                m,
                p,
                upper,
                room.height + 0.25,
                0.42,
                0.3,
                0.03,
                tint(stone, 1.12),
            );
            continue;
        }
        wall_chamfer(
            m,
            p,
            0.0,
            0.46,
            PIER_HALF + 0.1,
            PIER_DEPTH + 0.1,
            0.04,
            tint(stone, 0.8),
        );
        let courses = (((s - 0.1 - 0.46) / 0.56).round() as usize).max(3);
        let course = (s - 0.1 - 0.46) / courses as f32;
        for k in 0..courses {
            let y0 = 0.46 + k as f32 * course;
            let wide = if k % 2 == 0 { 0.02 } else { -0.01 };
            let shade = rng.range(0.9, 1.05);
            wall_chamfer(
                m,
                p,
                y0 + 0.01,
                y0 + course - 0.01,
                PIER_HALF - 0.04 + wide,
                PIER_DEPTH - 0.02 + wide,
                0.035,
                tint(stone, shade),
            );
        }
        wall_chamfer(
            m,
            p,
            s - 0.1,
            s + 0.08,
            PIER_HALF + 0.06,
            PIER_DEPTH + 0.06,
            0.03,
            tint(stone, 1.14),
        );
        wall_chamfer(
            m,
            p,
            s + 0.08,
            s + 0.26,
            PIER_HALF + 0.02,
            PIER_DEPTH + 0.02,
            0.03,
            tint(stone, 0.92),
        );
        let upper = room.height - 0.55;
        let high = (((upper - s - 0.26) / 0.6).round() as usize).max(1);
        let step = (upper - s - 0.26) / high as f32;
        for k in 0..high {
            let y0 = s + 0.26 + k as f32 * step;
            let shade = rng.range(0.88, 1.02);
            wall_chamfer(
                m,
                p,
                y0 + 0.01,
                y0 + step - 0.01,
                PIER_HALF - 0.06,
                PIER_DEPTH - 0.06,
                0.035,
                tint(stone, shade),
            );
        }
        wall_chamfer(
            m,
            p,
            upper,
            room.height + 0.25,
            PIER_HALF + 0.08,
            PIER_DEPTH + 0.04,
            0.04,
            tint(stone, 1.1),
        );
    }
    for niche in &niches {
        let r = niche.half;
        let a = niche.center - niche.tangent * r + niche.inward * 0.06 + Y(niche.spring);
        let b = niche.center + niche.tangent * r + niche.inward * 0.06 + Y(niche.spring);
        chunky_arch(
            m,
            a,
            b,
            r,
            niche.inward,
            0.3,
            0.14,
            tint(stone, 1.02),
            &mut rng,
        );
        for sign in [-1.0f32, 1.0] {
            chamfer_box(
                m,
                niche.center
                    + niche.tangent * (sign * (r + 0.12))
                    + niche.inward * 0.06
                    + Y(niche.spring - 0.04),
                niche.tangent,
                VY,
                niche.inward,
                v(0.2, 0.06, 0.14),
                0.02,
                tint(stone, 1.15),
            );
        }
    }
    bands(m, room, &niches, stone);
}

/// Dark timber beams across the vault on iron brackets.
pub fn hall_beams(m: &mut Mesh3D, room: &Room, seed: u64) {
    let mut rng = ArtRng::stream(seed, "hall_beams_v1", room.id as u64);
    let along_x = room.half[0] <= room.half[1];
    let span = if along_x { room.half[0] } else { room.half[1] };
    let length = if along_x { room.half[1] } else { room.half[0] };
    let across = if along_x { VX } else { VZ };
    let along = if along_x { VZ } else { VX };
    let y = room.center.y + room.height - 0.2;
    let count = ((length * 2.0 / 2.6).round() as usize).max(2);
    for k in 0..count {
        let t = -length + (k as f32 + 0.5) * (length * 2.0 / count as f32);
        let c = v(room.center.x, y, room.center.z) + along * t;
        let shade = rng.range(0.85, 1.1);
        chamfer_box(
            m,
            c,
            across,
            VY,
            along,
            v(span + 0.3, 0.16, 0.13),
            0.025,
            tint(WOOD, shade),
        );
        for s in [-1.0f32, 1.0] {
            let end = c + across * (s * (span - 0.35));
            chamfer_box(
                m,
                end - Y(0.2),
                across,
                VY,
                along,
                v(0.22, 0.04, 0.15),
                0.01,
                [0.12, 0.1, 0.09],
            );
            chamfer_box(
                m,
                end - Y(0.36) + across * (s * 0.2),
                VY,
                across * -s,
                along,
                v(0.18, 0.05, 0.09),
                0.015,
                tint(WOOD, 0.8),
            );
        }
    }
}

/// Plinth course (broken at doors and niches) and a high cornice.
fn bands(m: &mut Mesh3D, room: &Room, niches: &[Niche], stone: C3) {
    let top = niches.first().map_or(room.height - 1.5, |n| n.top) + 0.35;
    for i in 0..room.outline.len() {
        let a = room.outline[i];
        let b = room.outline[(i + 1) % room.outline.len()];
        let start = v(room.center.x + a[0], room.center.y, room.center.z + a[1]);
        let delta = v(b[0] - a[0], 0.0, b[1] - a[1]);
        let length = delta.length();
        if length < 0.2 {
            continue;
        }
        let tangent = norm(delta);
        let inward = v(-tangent.z, 0.0, tangent.x);
        let mut gaps: Vec<(f32, f32)> = vec![];
        for s in &room.sockets {
            let d = v(s.position.x - start.x, 0.0, s.position.z - start.z);
            if d.dot(inward).abs() < 0.05 {
                gaps.push((
                    d.dot(tangent) - s.width * 0.5 - 0.5,
                    d.dot(tangent) + s.width * 0.5 + 0.5,
                ));
            }
        }
        for niche in niches {
            let d = hz(niche.center - start);
            if d.dot(inward).abs() < 0.05 {
                gaps.push((d.dot(tangent) - niche.half, d.dot(tangent) + niche.half));
            }
        }
        gaps.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut spans = vec![];
        let mut cursor = 0.0f32;
        for &(g0, g1) in &gaps {
            if g0 > cursor + 0.05 {
                spans.push((cursor, g0.min(length)));
            }
            cursor = cursor.max(g1);
        }
        if cursor < length - 0.05 {
            spans.push((cursor, length));
        }
        let band = |m: &mut Mesh3D, s0: f32, s1: f32, y0: f32, y1: f32, depth: f32, color: C3| {
            let mid = start + tangent * ((s0 + s1) * 0.5);
            chamfer_box(
                m,
                v(mid.x, room.center.y + (y0 + y1) * 0.5, mid.z) + inward * ((depth - 0.1) * 0.5),
                tangent,
                VY,
                inward,
                v((s1 - s0) * 0.5, (y1 - y0) * 0.5, (depth + 0.1) * 0.5),
                0.02,
                color,
            );
        };
        for &(s0, s1) in &spans {
            band(m, s0, s1, 0.0, 0.28, 0.08, tint(stone, 0.7));
            band(m, s0, s1, 0.28, 0.34, 0.11, tint(stone, 1.08));
        }
        // The cornice breaks over doorways, leaving room for the royal crest.
        let door_gaps: Vec<(f32, f32)> = gaps
            .iter()
            .filter(|&&(g0, g1)| {
                room.sockets.iter().any(|s| {
                    let d = v(s.position.x - start.x, 0.0, s.position.z - start.z);
                    d.dot(inward).abs() < 0.05 && (d.dot(tangent) - (g0 + g1) * 0.5).abs() < 0.01
                })
            })
            .map(|&(g0, g1)| (g0 - 0.35, g1 + 0.35))
            .collect();
        let mut from = 0.0f32;
        for (g0, g1) in door_gaps {
            if g0 > from + 0.05 {
                band(m, from, g0, top, top + 0.12, 0.12, tint(stone, 1.1));
                band(m, from, g0, top + 0.12, top + 0.2, 0.18, tint(stone, 0.9));
            }
            from = from.max(g1);
        }
        if from < length - 0.05 {
            band(m, from, length, top, top + 0.12, 0.12, tint(stone, 1.1));
            band(
                m,
                from,
                length,
                top + 0.12,
                top + 0.2,
                0.18,
                tint(stone, 0.9),
            );
        }
    }
}
