//! Rendering recipes consume semantic definitions, never affect navigation.
//! One static batch per room/corridor. All large pillars come from collision data.
use velvet_dungeon::*;
use velvet_math::Vec3;
use velvet_render::{Material3D, Mesh3D, MeshVertex3D};
pub fn empty() -> Mesh3D {
    Mesh3D {
        vertices: vec![],
        indices: vec![],
        material: Material3D::rgb(1.0, 1.0, 1.0),
        metallic_factor: 0.0,
        roughness_factor: 0.86,
        texture_index: None,
        normal_texture_index: None,
        skinning: None,
    }
}
pub fn tint(c: [f32; 3], s: f32) -> [f32; 3] {
    [c[0] * s, c[1] * s, c[2] * s]
}
pub fn tri(m: &mut Mesh3D, a: Vec3, b: Vec3, c: Vec3, normal: Vec3, color: [f32; 3]) {
    let area = (b - a).cross(c - a);
    if area.length_squared() < 1e-12 {
        return;
    }
    let index = m.vertices.len() as u32;
    let n = area.normalize_or_zero();
    let n = if n.dot(normal) < 0.0 { -n } else { n };
    for p in [a, b, c] {
        let uv = if normal.y.abs() > 0.65 {
            [p.x * 0.28, p.z * 0.28]
        } else if normal.x.abs() > normal.z.abs() {
            [p.z * 0.28, p.y * 0.28]
        } else {
            [p.x * 0.28, p.y * 0.28]
        };
        m.vertices.push(MeshVertex3D {
            position: p.to_array(),
            normal: n.to_array(),
            uv,
            color: [color[0], color[1], color[2], 1.0],
        });
    }
    if (b - a).cross(c - a).dot(normal) < 0.0 {
        m.indices.extend([index, index + 2, index + 1]);
    } else {
        m.indices.extend([index, index + 1, index + 2]);
    }
}
pub fn quad(m: &mut Mesh3D, p: [Vec3; 4], normal: Vec3, color: [f32; 3]) {
    tri(m, p[0], p[1], p[2], normal, color);
    tri(m, p[0], p[2], p[3], normal, tint(color, 0.98));
}
pub fn cuboid(m: &mut Mesh3D, base: Vec3, half: Vec3, color: [f32; 3]) {
    let p = |x: f32, y: f32, z: f32| base + Vec3::new(x * half.x, y * half.y, z * half.z);
    for (v, n) in [
        (
            [
                p(-1.0, -1.0, 1.0),
                p(1.0, -1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(-1.0, 1.0, 1.0),
            ],
            Vec3::Z,
        ),
        (
            [
                p(1.0, -1.0, -1.0),
                p(-1.0, -1.0, -1.0),
                p(-1.0, 1.0, -1.0),
                p(1.0, 1.0, -1.0),
            ],
            -Vec3::Z,
        ),
        (
            [
                p(-1.0, -1.0, -1.0),
                p(-1.0, -1.0, 1.0),
                p(-1.0, 1.0, 1.0),
                p(-1.0, 1.0, -1.0),
            ],
            -Vec3::X,
        ),
        (
            [
                p(1.0, -1.0, 1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(1.0, 1.0, 1.0),
            ],
            Vec3::X,
        ),
        (
            [
                p(-1.0, 1.0, -1.0),
                p(-1.0, 1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(1.0, 1.0, -1.0),
            ],
            Vec3::Y,
        ),
        (
            [
                p(-1.0, -1.0, 1.0),
                p(-1.0, -1.0, -1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, -1.0, 1.0),
            ],
            -Vec3::Y,
        ),
    ] {
        quad(m, v, n, color);
    }
}
fn wayfinding_stroke(
    mesh: &mut Mesh3D,
    mut start: Vec3,
    mut end: Vec3,
    width: f32,
    y: f32,
    color: [f32; 3],
) {
    start.y = y;
    end.y = y;
    let segment = end - start;
    let across = Vec3::new(-segment.z, 0.0, segment.x).normalize_or_zero() * (width * 0.5);
    quad(
        mesh,
        [start - across, start + across, end + across, end - across],
        Vec3::Y,
        color,
    );
}

/// Low-profile stone inlay pointing from a room toward its critical exit.
/// Two-tone chevrons replace the old raised, axis-aligned accent cubes.
fn wayfinding_glyph(
    mesh: &mut Mesh3D,
    center: Vec3,
    outward: Vec3,
    floor: [f32; 3],
    accent: [f32; 3],
) {
    let forward = Vec3::new(outward.x, 0.0, outward.z).normalize_or_zero();
    if forward.length_squared() < 0.5 {
        return;
    }
    let side = Vec3::new(-forward.z, 0.0, forward.x);
    let left = center - side * 0.15 - forward * 0.105;
    let tip = center + forward * 0.105;
    let right = center + side * 0.15 - forward * 0.105;
    let shadow = tint(floor, 0.62);
    let inlay = [
        floor[0] * 0.66 + accent[0] * 0.34,
        floor[1] * 0.66 + accent[1] * 0.34,
        floor[2] * 0.66 + accent[2] * 0.34,
    ];

    // The dark under-stroke reads like a cut groove; the narrower color strip
    // sits just above it, close enough to the floor relief to avoid floating.
    let groove_y = center.y - 0.022;
    for (start, end) in [(left, tip), (right, tip)] {
        wayfinding_stroke(mesh, start, end, 0.060, groove_y, shadow);
    }
    let inlay_y = center.y - 0.015;
    for (start, end) in [(left, tip), (right, tip)] {
        wayfinding_stroke(mesh, start, end, 0.026, inlay_y, inlay);
    }
}

pub fn crystal(
    m: &mut Mesh3D,
    base: Vec3,
    radius: f32,
    height: f32,
    color: [f32; 3],
    sides: usize,
) {
    if sides < 3 || radius <= 0.0 || height.abs() < 0.001 {
        return;
    }
    let phase = (radius * 17.0 + height * 3.0).sin() * 0.12;
    let ring = |index: usize, level: usize| {
        let side = index % sides;
        let angle = side as f32 * std::f32::consts::TAU / sides as f32 + phase;
        let wobble = 0.94 + 0.06 * (side as f32 * 5.7 + radius * 13.0).sin();
        let (y, width) = match level {
            0 => (0.0, 0.78),
            1 => (height * 0.20, 1.0),
            _ => (height * 0.70, 0.72),
        };
        base + Vec3::new(
            angle.cos() * radius * width * wobble,
            y,
            angle.sin() * radius * width * wobble,
        )
    };
    let tip = base + Vec3::new(radius * 0.10, height, radius * 0.07);
    let base_normal = if height > 0.0 { -Vec3::Y } else { Vec3::Y };
    for i in 0..sides {
        let p0 = ring(i, 0);
        let q0 = ring(i + 1, 0);
        let p1 = ring(i, 1);
        let q1 = ring(i + 1, 1);
        let p2 = ring(i, 2);
        let q2 = ring(i + 1, 2);
        let radial = Vec3::new(
            (p0.x + q0.x) * 0.5 - base.x,
            0.0,
            (p0.z + q0.z) * 0.5 - base.z,
        )
        .normalize_or_zero();
        let shade = tint(color, 0.82 + (i % 3) as f32 * 0.10);
        quad(m, [p0, q0, q1, p1], radial, tint(shade, 0.92));
        quad(m, [p1, q1, q2, p2], radial, shade);
        tri(
            m,
            p2,
            q2,
            tip,
            (radial + Vec3::Y * height.signum() * 0.62).normalize_or_zero(),
            tint(shade, 1.08),
        );
        tri(m, base, q0, p0, base_normal, tint(color, 0.72));
    }
}

fn coin(
    mesh: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    thickness: f32,
    phase: f32,
    tilt: Vec3,
    color: [f32; 3],
) {
    const SIDES: usize = 10;
    let low_y = center.y - thickness * 0.5;
    let high_y = center.y + thickness * 0.5;
    let bevel_y = thickness * 0.22;
    let bevel_radius = radius * 0.12;
    let rotate = |v: Vec3| {
        let (sin_x, cos_x) = tilt.x.sin_cos();
        let (sin_z, cos_z) = tilt.z.sin_cos();
        let x_rotated = Vec3::new(v.x, v.y * cos_x - v.z * sin_x, v.y * sin_x + v.z * cos_x);
        Vec3::new(
            x_rotated.x * cos_z - x_rotated.y * sin_z,
            x_rotated.x * sin_z + x_rotated.y * cos_z,
            x_rotated.z,
        )
    };
    let point = |index: usize, y: f32, r: f32| {
        let angle = index as f32 * std::f32::consts::TAU / SIDES as f32 + phase;
        center + rotate(Vec3::new(angle.cos() * r, y - center.y, angle.sin() * r))
    };
    let top = center + rotate(Vec3::Y * (high_y - center.y - bevel_y));
    let bottom = center + rotate(-Vec3::Y * (center.y - low_y - bevel_y));
    for index in 0..SIDES {
        let a = index as f32 * std::f32::consts::TAU / SIDES as f32 + phase;
        let b = (index + 1) as f32 * std::f32::consts::TAU / SIDES as f32 + phase;
        let normal = rotate(Vec3::new((a + b).cos(), 0.0, (a + b).sin()));
        let low_a = point(index, low_y, radius);
        let low_b = point(index + 1, low_y, radius);
        let high_a = point(index, high_y, radius);
        let high_b = point(index + 1, high_y, radius);
        let bottom_a = point(index, low_y + bevel_y, radius - bevel_radius);
        let bottom_b = point(index + 1, low_y + bevel_y, radius - bevel_radius);
        let top_a = point(index, high_y - bevel_y, radius - bevel_radius);
        let top_b = point(index + 1, high_y - bevel_y, radius - bevel_radius);
        quad(
            mesh,
            [low_a, low_b, high_b, high_a],
            normal,
            tint(color, 0.78 + (index % 3) as f32 * 0.09),
        );
        let bevel_normal = (normal + rotate(Vec3::Y)).normalize_or_zero();
        quad(
            mesh,
            [high_a, high_b, top_b, top_a],
            bevel_normal,
            tint(color, 1.12 + (index % 2) as f32 * 0.08),
        );
        quad(
            mesh,
            [bottom_a, bottom_b, low_b, low_a],
            (normal - rotate(Vec3::Y)).normalize_or_zero(),
            tint(color, 0.72),
        );
        tri(
            mesh,
            top,
            top_a,
            top_b,
            rotate(Vec3::Y),
            tint(color, if index % 2 == 0 { 1.24 } else { 1.02 }),
        );
        // A restrained center glint is part of the closed top face, so the
        // raised-looking coin detail never introduces loose, paper-thin geo.
        let center_vertex = mesh.vertices.len() - 3;
        mesh.vertices[center_vertex].color = [1.0, 0.88, 0.36, 1.0];
        tri(
            mesh,
            bottom,
            point(index + 1, low_y + bevel_y, radius - bevel_radius),
            bottom_a,
            -rotate(Vec3::Y),
            tint(color, 0.70),
        );
    }
}

/// Small low-poly crown used as a readable treasure-room focal prop.
pub fn crown() -> Mesh3D {
    const SIDES: usize = 8;
    let mut mesh = empty();
    let outer = 0.30;
    let inner = 0.22;
    let bottom = 0.025;
    let top = 0.17;
    let point = |index: usize, radius: f32, y: f32| {
        let angle = index as f32 * std::f32::consts::TAU / SIDES as f32;
        Vec3::new(angle.cos() * radius, y, angle.sin() * radius)
    };
    let gold = [0.96, 0.59, 0.12];
    let shade = [0.70, 0.31, 0.055];

    for side in 0..SIDES {
        let next = side + 1;
        let outward = Vec3::new(
            (point(side, outer, 0.0).x + point(next, outer, 0.0).x) * 0.5,
            0.0,
            (point(side, outer, 0.0).z + point(next, outer, 0.0).z) * 0.5,
        )
        .normalize_or_zero();
        quad(
            &mut mesh,
            [
                point(side, outer, bottom),
                point(next, outer, bottom),
                point(next, outer, top),
                point(side, outer, top),
            ],
            outward,
            if side % 2 == 0 {
                gold
            } else {
                tint(gold, 0.88)
            },
        );
        quad(
            &mut mesh,
            [
                point(side, inner, bottom),
                point(side, inner, top),
                point(next, inner, top),
                point(next, inner, bottom),
            ],
            -outward,
            shade,
        );
        quad(
            &mut mesh,
            [
                point(side, inner, top),
                point(side, outer, top),
                point(next, outer, top),
                point(next, inner, top),
            ],
            Vec3::Y,
            tint(gold, 1.08),
        );
        quad(
            &mut mesh,
            [
                point(side, outer, bottom),
                point(side, inner, bottom),
                point(next, inner, bottom),
                point(next, outer, bottom),
            ],
            -Vec3::Y,
            shade,
        );
    }

    for tooth in [0, 2, 3, 5, 7] {
        let left = point(tooth, outer, top);
        let right = point(tooth + 1, outer, top);
        let angle = (tooth as f32 + 0.5) * std::f32::consts::TAU / SIDES as f32;
        let tip = Vec3::new(angle.cos() * 0.27, 0.43, angle.sin() * 0.27);
        let normal = Vec3::new(angle.cos(), 0.12, angle.sin()).normalize_or_zero();
        tri(&mut mesh, left, right, tip, normal, tint(gold, 1.08));
        tri(&mut mesh, right, left, tip, -normal, tint(gold, 0.78));
    }

    // Jewel faces into the room so the little crown reads at player height.
    crystal(
        &mut mesh,
        Vec3::new(-0.055, 0.075, 0.279),
        0.061,
        0.105,
        [0.16, 0.84, 1.0],
        5,
    );
    mesh
}

/// Closed, low-poly coin stacks sized to sit below an opened chest's rim.
pub fn chest_gold() -> Mesh3D {
    let mut mesh = empty();
    let gold = [1.0, 0.58, 0.08];
    for (x, z, stack, phase) in [(-0.16, -0.025, 3, 0.10), (0.15, 0.035, 2, 0.42)] {
        for layer in 0..stack {
            coin(
                &mut mesh,
                Vec3::new(
                    x + (layer % 2) as f32 * 0.018,
                    0.035 + layer as f32 * 0.043,
                    z - (layer % 2) as f32 * 0.012,
                ),
                0.13,
                0.034,
                phase + layer as f32 * 0.17,
                Vec3::ZERO,
                gold,
            );
        }
    }
    for (x, z, phase, tilt) in [
        (-0.075, 0.19, 0.24, Vec3::new(0.28, 0.0, -0.17)),
        (0.09, -0.18, 0.58, Vec3::new(-0.22, 0.0, 0.30)),
    ] {
        coin(
            &mut mesh,
            Vec3::new(x, 0.032, z),
            0.125,
            0.03,
            phase,
            tilt,
            tint(gold, 0.96),
        );
    }
    mesh
}

/// Segmented ground marker for a committed melee attack. Draw at attack reach;
/// its flat silhouette reads as a danger boundary without floating particles.
/// Branching, floor-flush fissure used by the grave mage's avoidable rupture.
pub fn spell_fissure() -> Mesh3D {
    let mut mesh = empty();
    let paths: &[&[(f32, f32)]] = &[
        &[
            (0.0, -0.98),
            (0.08, -0.66),
            (-0.04, -0.38),
            (0.15, -0.13),
            (0.0, 0.12),
            (0.17, 0.40),
            (0.07, 0.76),
            (0.20, 1.04),
        ],
        &[(0.02, -0.56), (-0.28, -0.40), (-0.45, -0.14), (-0.77, 0.02)],
        &[(0.04, -0.34), (0.36, -0.20), (0.49, 0.05), (0.82, 0.24)],
        &[(0.09, 0.35), (-0.16, 0.52), (-0.28, 0.82)],
        &[(0.09, 0.72), (0.39, 0.88), (0.57, 1.10)],
    ];
    for path in paths {
        for segment in path.windows(2) {
            let a = Vec3::new(segment[0].0, 0.0, segment[0].1);
            let b = Vec3::new(segment[1].0, 0.0, segment[1].1);
            wayfinding_stroke(&mut mesh, a, b, 0.105, 0.014, [0.055, 0.025, 0.12]);
            wayfinding_stroke(&mut mesh, a, b, 0.030, 0.021, [0.34, 0.76, 1.0]);
        }
    }
    mesh
}

fn flask_profile(m: &mut Mesh3D, profile: &[(f32, f32)], base: [f32; 3], sides: usize) {
    for pair in profile.windows(2) {
        let (y0, r0) = pair[0];
        let (y1, r1) = pair[1];
        for side in 0..sides {
            let a = side as f32 * std::f32::consts::TAU / sides as f32;
            let b = (side + 1) as f32 * std::f32::consts::TAU / sides as f32;
            let mid = (a + b) * 0.5;
            let front_light = mid.sin().max(0.0);
            let shade = 0.72 + front_light * 0.32 + (side % 3) as f32 * 0.025;
            let p = |y: f32, radius: f32, angle: f32| {
                Vec3::new(radius * angle.cos(), y, radius * angle.sin())
            };
            quad(
                m,
                [p(y0, r0, a), p(y0, r0, b), p(y1, r1, b), p(y1, r1, a)],
                Vec3::new(mid.cos(), r0 - r1, mid.sin()),
                tint(base, shade),
            );
        }
    }
}

fn flask_cap(m: &mut Mesh3D, y: f32, radius: f32, color: [f32; 3], sides: usize, top: bool) {
    for side in 0..sides {
        let a = side as f32 * std::f32::consts::TAU / sides as f32;
        let b = (side + 1) as f32 * std::f32::consts::TAU / sides as f32;
        let point = |angle: f32| Vec3::new(radius * angle.cos(), y, radius * angle.sin());
        let normal = if top { Vec3::Y } else { -Vec3::Y };
        tri(
            m,
            Vec3::new(0.0, y, 0.0),
            point(a),
            point(b),
            normal,
            tint(color, 0.88 + (side % 3) as f32 * 0.06),
        );
    }
}

/// Small, fully opaque low-poly healing flask used as physical dungeon loot.
/// Keeping the liquid in the silhouette makes it readable under cave lighting.
pub fn healing_potion() -> Mesh3D {
    let mut m = empty();
    m.roughness_factor = 0.38;
    m.metallic_factor = 0.035;
    let sides = 8;

    let liquid = [0.72, 0.035, 0.065];
    flask_profile(
        &mut m,
        &[
            (0.03, 0.18),
            (0.10, 0.38),
            (0.22, 0.47),
            (0.50, 0.46),
            (0.62, 0.38),
            (0.72, 0.19),
        ],
        liquid,
        sides,
    );
    flask_cap(&mut m, 0.03, 0.18, tint(liquid, 0.78), sides, false);
    flask_cap(&mut m, 0.72, 0.19, [0.88, 0.075, 0.10], sides, true);

    // Neck and collar are deliberately opaque cool silver-blue, not a glass
    // shell: the red contents stay visible in a dark first-person scene.
    flask_profile(
        &mut m,
        &[(0.72, 0.17), (0.82, 0.16), (0.86, 0.16)],
        [0.48, 0.68, 0.72],
        sides,
    );
    flask_profile(
        &mut m,
        &[(0.86, 0.16), (0.88, 0.19), (0.91, 0.19)],
        [0.72, 0.78, 0.66],
        sides,
    );
    flask_profile(
        &mut m,
        &[(0.91, 0.15), (0.98, 0.15), (1.0, 0.12)],
        [0.56, 0.29, 0.13],
        sides,
    );
    flask_cap(&mut m, 1.0, 0.12, [0.74, 0.43, 0.20], sides, true);

    // A narrow painted-glass glint is part of the bottle mesh, not a floating
    // effect. It follows the curved front contour of the flask.
    quad(
        &mut m,
        [
            Vec3::new(-0.13, 0.24, 0.455),
            Vec3::new(-0.07, 0.24, 0.474),
            Vec3::new(-0.07, 0.53, 0.425),
            Vec3::new(-0.13, 0.53, 0.410),
        ],
        Vec3::Z,
        [1.0, 0.34, 0.30],
    );
    m
}

/// A room's static shell: the web prototype's `roomMesh`.
pub fn room(map: &Blueprint, catalog: &Catalog, id: usize) -> Mesh3D {
    let plan = super::art::theme::any(map, &map.rooms[id]);
    super::art::chambers::room_mesh(map, catalog, id, plan.as_ref())
}

fn oriented_box(
    mesh: &mut Mesh3D,
    center: Vec3,
    side: Vec3,
    forward: Vec3,
    half_side: f32,
    half_height: f32,
    half_depth: f32,
    color: [f32; 3],
) {
    let point = |s: f32, y: f32, d: f32| center + side * s + Vec3::Y * y + forward * d;
    let (sx, sy, sz) = (half_side, half_height, half_depth);
    quad(
        mesh,
        [
            point(-sx, -sy, sz),
            point(sx, -sy, sz),
            point(sx, sy, sz),
            point(-sx, sy, sz),
        ],
        forward,
        tint(color, 1.12),
    );
    quad(
        mesh,
        [
            point(sx, -sy, -sz),
            point(-sx, -sy, -sz),
            point(-sx, sy, -sz),
            point(sx, sy, -sz),
        ],
        -forward,
        tint(color, 0.64),
    );
    quad(
        mesh,
        [
            point(-sx, sy, sz),
            point(sx, sy, sz),
            point(sx, sy, -sz),
            point(-sx, sy, -sz),
        ],
        Vec3::Y,
        tint(color, 1.24),
    );
    quad(
        mesh,
        [
            point(-sx, -sy, -sz),
            point(sx, -sy, -sz),
            point(sx, -sy, sz),
            point(-sx, -sy, sz),
        ],
        -Vec3::Y,
        tint(color, 0.52),
    );
    quad(
        mesh,
        [
            point(-sx, -sy, -sz),
            point(-sx, -sy, sz),
            point(-sx, sy, sz),
            point(-sx, sy, -sz),
        ],
        -side,
        tint(color, 0.78),
    );
    quad(
        mesh,
        [
            point(sx, -sy, sz),
            point(sx, -sy, -sz),
            point(sx, sy, -sz),
            point(sx, sy, sz),
        ],
        side,
        tint(color, 0.88),
    );
}

/// Arched doorway for signature rooms: quoined jambs, moulded imposts, a
/// ring of voussoirs with a projecting keystone, an outer archivolt and
/// stone spandrels closing the square opening above the arch. The passage
/// below the intrados stays open; everything above jump headroom.
fn doorway_arch(mesh: &mut Mesh3D, socket: &Socket, stone: [f32; 3], accent: [f32; 3], seed: u64) {
    use super::architecture::block;
    let outward = horizontal(socket.outward).normalize_or_zero();
    let f = -outward;
    let t = Vec3::new(-f.z, 0.0, f.x);
    let base = socket.position;
    let hw = socket.width * 0.5;
    let front = 0.32;
    let back = 0.1;
    let dc = (front - back) * 0.5;
    let hd = (front + back) * 0.5;
    let spring = socket.height - 0.83;
    let mut rng = rng::Rng::stream(seed, "arched_door_v1", socket.connection as u64);
    // Quoined jambs: alternating long and short stones up to the impost.
    for sign in [-1.0f32, 1.0] {
        let mut y = 0.0;
        let mut k = 0;
        while y < spring - 0.02 {
            let h = 0.42f32.min(spring - y);
            let w = if k % 2 == 0 { 0.3 } else { 0.21 };
            let c = base + t * (sign * (hw + w)) + f * dc + Vec3::Y * (y + h * 0.5);
            block(
                mesh,
                c,
                t,
                Vec3::Y,
                f,
                Vec3::new(w - 0.012, h * 0.5 - 0.012, hd),
                tint(stone, rng.range(0.9, 1.08)),
            );
            y += h;
            k += 1;
        }
        block(
            mesh,
            base + t * (sign * (hw + 0.38)) + f * (dc + 0.02) + Vec3::Y * (spring + 0.06),
            t,
            Vec3::Y,
            f,
            Vec3::new(0.37, 0.06, hd + 0.03),
            tint(stone, 1.16),
        );
    }
    // Voussoirs on an elliptical intrados from impost to the opening top.
    let s0 = spring + 0.12;
    let rise = socket.height - s0;
    let thickness = 0.44;
    let pieces = 13;
    let at = |theta: f32| base + t * (-theta.cos() * hw) + Vec3::Y * (s0 + theta.sin() * rise);
    let normal_at =
        |theta: f32| (t * (-theta.cos() / hw) + Vec3::Y * (theta.sin() / rise)).normalize_or_zero();
    for k in 0..pieces {
        let a0 = std::f32::consts::PI * k as f32 / pieces as f32;
        let a1 = std::f32::consts::PI * (k + 1) as f32 / pieces as f32;
        let (p0, p1) = (at(a0), at(a1));
        let tangent = (p1 - p0).normalize_or_zero();
        let radial = normal_at((a0 + a1) * 0.5);
        let keystone = k == pieces / 2;
        let thick = thickness * if keystone { 1.28 } else { 1.0 };
        let c = (p0 + p1) * 0.5 + radial * (thick * 0.5) + f * dc;
        let proud = if keystone { 0.05 } else { 0.0 };
        block(
            mesh,
            c + f * proud,
            tangent,
            radial,
            f,
            Vec3::new((p1 - p0).length() * 0.5 - 0.012, thick * 0.5, hd),
            tint(stone, if keystone { 1.24 } else { rng.range(0.94, 1.1) }),
        );
        // Archivolt: a lighter moulding riding the extrados.
        let e0 = at(a0) + normal_at(a0) * thickness;
        let e1 = at(a1) + normal_at(a1) * thickness;
        block(
            mesh,
            (e0 + e1) * 0.5 + radial * 0.05 + f * (dc + 0.04),
            (e1 - e0).normalize_or_zero(),
            radial,
            f,
            Vec3::new((e1 - e0).length() * 0.5, 0.05, hd + 0.02),
            tint(stone, 1.14),
        );
        if keystone {
            // Carved emblem on the keystone face.
            let face = c + f * (hd + proud + 0.012);
            block(
                mesh,
                face,
                tangent,
                radial,
                f,
                Vec3::new(0.09, 0.12, 0.015),
                tint(accent, 1.05),
            );
            block(
                mesh,
                face + f * 0.012,
                tangent,
                radial,
                f,
                Vec3::new(0.045, 0.06, 0.012),
                tint(accent, 0.7),
            );
        }
    }
    // Spandrels: close the rectangular opening between extrados and lintel.
    let top = socket.height;
    let ex = hw + thickness;
    let ey = rise + thickness;
    let columns = 24;
    for i in 0..columns {
        let x0 = -hw + socket.width * i as f32 / columns as f32;
        let x1 = -hw + socket.width * (i + 1) as f32 / columns as f32;
        let y_at = |x: f32| s0 + ey * (1.0 - (x / ex).powi(2)).max(0.0).sqrt();
        let (y0, y1) = (y_at(x0).min(top), y_at(x1).min(top));
        if y0 >= top - 0.001 && y1 >= top - 0.001 {
            continue;
        }
        let p = |x: f32, y: f32| base + t * x + Vec3::Y * y + f * 0.012;
        let color = tint(stone, 0.9);
        quad(
            mesh,
            [p(x0, y0), p(x1, y1), p(x1, top), p(x0, top)],
            f,
            color,
        );
        quad(
            mesh,
            [p(x0, y0), p(x1, y1), p(x1, top), p(x0, top)],
            -f,
            tint(color, 0.7),
        );
    }
}

fn doorway_frame(mesh: &mut Mesh3D, socket: &Socket, stone: [f32; 3], seed: u64) {
    let outward = Vec3::new(socket.outward.x, 0.0, socket.outward.z).normalize_or_zero();
    let forward = -outward;
    let side = Vec3::new(-forward.z, 0.0, forward.x);
    let center = socket.position + forward * 0.12;
    let half_depth = 0.17;
    let post_half = 0.21;
    let post_offset = socket.width * 0.5 + post_half + 0.06;
    let courses = ((socket.height - 0.46) / 0.62).ceil().max(1.0) as usize;
    let mut rng = rng::Rng::stream(seed, "procedural_arch_v1", socket.connection as u64);

    for sign in [-1.0, 1.0] {
        for course in 0..courses {
            let y0 = course as f32 * (socket.height - 0.46) / courses as f32;
            let y1 = (course + 1) as f32 * (socket.height - 0.46) / courses as f32;
            let half_height = ((y1 - y0) - 0.035).max(0.12) * 0.5;
            let position =
                center + side * (sign * post_offset) + Vec3::Y * ((y0 + y1) * 0.5 + 0.017);
            oriented_box(
                mesh,
                position,
                side,
                forward,
                post_half * rng.range(0.93, 1.04),
                half_height,
                half_depth,
                tint(stone, rng.range(0.88, 1.12)),
            );
        }
        let foot_offset = socket.width * 0.5 + post_half + 0.13;
        let foot = center + side * (sign * foot_offset) + Vec3::Y * 0.13;
        oriented_box(
            mesh,
            foot,
            side,
            forward,
            post_half + 0.10,
            0.13,
            half_depth + 0.055,
            tint(stone, 1.18),
        );
    }

    // Keyed lintel stones sit above the player's jump envelope. The centre
    // voussoir is wider and brighter, so every opening reads at a glance.
    let lintel_bottom = socket.height - 0.46;
    let lintel_height = socket.height - lintel_bottom;
    let block_count = (socket.width / 0.58).ceil().max(3.0) as usize;
    for index in 0..block_count {
        let fraction = (index as f32 + 0.5) / block_count as f32;
        let offset = (fraction - 0.5) * socket.width;
        let half_side = socket.width / block_count as f32 * 0.52;
        let keystone = index == block_count / 2;
        let y = (lintel_bottom + socket.height) * 0.5;
        oriented_box(
            mesh,
            center + side * offset + Vec3::Y * y,
            side,
            forward,
            half_side,
            lintel_height * 0.5,
            half_depth + if keystone { 0.045 } else { 0.0 },
            tint(
                stone,
                if keystone {
                    1.32
                } else {
                    0.96 + rng.range(-0.08, 0.08)
                },
            ),
        );
    }
}

pub fn crystal_cluster(stone: [f32; 3], accent: [f32; 3]) -> Mesh3D {
    let mut mesh = empty();
    rock(
        &mut mesh,
        Vec3::new(0.0, 0.02, 0.0),
        Vec3::new(0.52, 0.19, 0.43),
        stone,
        0.27,
    );
    for (index, (x, z, radius, height)) in [
        (-0.34, -0.08, 0.15, 0.43),
        (-0.17, 0.19, 0.17, 0.61),
        (0.08, -0.12, 0.21, 0.88),
        (0.32, 0.13, 0.14, 0.54),
        (0.22, 0.30, 0.10, 0.33),
    ]
    .into_iter()
    .enumerate()
    {
        crystal(
            &mut mesh,
            Vec3::new(x, 0.13 + (index % 2) as f32 * 0.035, z),
            radius,
            height,
            tint(accent, [0.84, 1.04, 0.94, 1.12, 0.90][index]),
            5 + index % 2,
        );
    }
    mesh
}

/// Carved standing brazier: plinth, fluted column, cap and an iron bowl of
/// coals. About 1.05 m tall; the animated flame is drawn separately on top.
pub fn brazier(stone: [f32; 3]) -> Mesh3D {
    let mut mesh = empty();
    let iron = [0.10, 0.085, 0.075];
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.08, 0.0),
        Vec3::new(0.34, 0.08, 0.34),
        tint(stone, 0.72),
    );
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.19, 0.0),
        Vec3::new(0.26, 0.04, 0.26),
        tint(stone, 0.9),
    );
    // Solid tapered shaft with carved corner ribs.
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.52, 0.0),
        Vec3::new(0.15, 0.30, 0.15),
        tint(stone, 0.95),
    );
    for (x, z) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        cuboid(
            &mut mesh,
            Vec3::new(x * 0.15, 0.52, z * 0.15),
            Vec3::new(0.035, 0.28, 0.035),
            tint(stone, 1.15),
        );
    }
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.85, 0.0),
        Vec3::new(0.21, 0.04, 0.21),
        tint(stone, 1.1),
    );
    // Flared iron bowl: three widening rings, open coals on top.
    for (y, half) in [(0.92, 0.20), (0.99, 0.28), (1.05, 0.35)] {
        cuboid(
            &mut mesh,
            Vec3::new(0.0, y, 0.0),
            Vec3::new(half, 0.035, half),
            iron,
        );
    }
    for (x, z) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        cuboid(
            &mut mesh,
            Vec3::new(x * 0.34, 1.11, z * 0.34),
            Vec3::new(0.025 + z.abs() * 0.34, 0.03, 0.025 + x.abs() * 0.34),
            tint(iron, 1.6),
        );
    }
    rock(
        &mut mesh,
        Vec3::new(0.0, 1.06, 0.0),
        Vec3::new(0.28, 0.08, 0.26),
        [0.20, 0.07, 0.03],
        0.4,
    );
    mesh
}

/// Low ring of broken basalt around a bed of coals (drawn emissive separately).
pub fn ember_ring(stone: [f32; 3]) -> Mesh3D {
    let mut mesh = empty();
    for index in 0..9 {
        let a = index as f32 * std::f32::consts::TAU / 9.0;
        rock(
            &mut mesh,
            Vec3::new(a.cos() * 0.62, 0.0, a.sin() * 0.62),
            Vec3::new(0.23, 0.20 + (index % 3) as f32 * 0.07, 0.20),
            tint(stone, 0.8 + (index % 2) as f32 * 0.25),
            a,
        );
    }
    rock(
        &mut mesh,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.52, 0.07, 0.50),
        [0.07, 0.035, 0.03],
        0.9,
    );
    mesh
}

pub fn runestone(stone: [f32; 3]) -> Mesh3D {
    let mut mesh = empty();
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.08, 0.0),
        Vec3::new(0.47, 0.08, 0.28),
        tint(stone, 0.68),
    );
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.17, 0.0),
        Vec3::new(0.39, 0.09, 0.23),
        tint(stone, 1.05),
    );
    let ring = [
        [-0.40, 0.26],
        [-0.30, 0.18],
        [0.30, 0.18],
        [0.40, 0.26],
        [0.36, 0.84],
        [0.18, 0.97],
        [-0.18, 0.97],
        [-0.36, 0.84],
    ];
    for index in 0..ring.len() {
        let a = ring[index];
        let b = ring[(index + 1) % ring.len()];
        let p = |v: [f32; 2], z: f32| Vec3::new(v[0], v[1], z);
        let center = p([0.0, 0.59], 0.11);
        tri(
            &mut mesh,
            center,
            p(a, 0.11),
            p(b, 0.11),
            Vec3::Z,
            tint(stone, if index % 2 == 0 { 1.10 } else { 0.92 }),
        );
        tri(
            &mut mesh,
            p([0.0, 0.59], -0.11),
            p(b, -0.11),
            p(a, -0.11),
            -Vec3::Z,
            tint(stone, 0.62),
        );
        quad(
            &mut mesh,
            [p(a, 0.11), p(b, 0.11), p(b, -0.11), p(a, -0.11)],
            Vec3::new(a[0] + b[0], 0.0, 0.0).normalize_or_zero(),
            tint(stone, if index % 2 == 0 { 1.17 } else { 0.76 }),
        );
    }
    // A dark inset makes the later rotating rune read as carved into stone.
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.59, 0.105),
        Vec3::new(0.25, 0.25, 0.012),
        tint(stone, 0.45),
    );
    mesh
}

pub fn rune_spiral() -> Mesh3D {
    let mut mesh = empty();
    const STEPS: usize = 72;
    let point = |step: usize| {
        let t = step as f32 / STEPS as f32;
        let angle = t * std::f32::consts::TAU * 2.15;
        let radius = 0.035 + 0.39 * t;
        Vec3::new(angle.cos() * radius, angle.sin() * radius, 0.016)
    };
    for step in 0..STEPS {
        let a = point(step);
        let b = point(step + 1);
        let tangent = (b - a).normalize_or_zero();
        let side = Vec3::new(-tangent.y, tangent.x, 0.0) * 0.027;
        quad(
            &mut mesh,
            [a - side, b - side, b + side, a + side],
            Vec3::Z,
            if step % 9 == 0 {
                [1.0; 3]
            } else {
                [0.88, 0.91, 0.82]
            },
        );
    }
    mesh
}

pub fn exit_altar(stone: [f32; 3], accent: [f32; 3]) -> Mesh3D {
    let mut mesh = empty();
    for (center, half, shade) in [
        (Vec3::new(0.0, 0.08, 0.0), Vec3::new(0.48, 0.08, 0.40), 0.68),
        (Vec3::new(0.0, 0.20, 0.0), Vec3::new(0.40, 0.08, 0.34), 0.92),
        (Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.32, 0.07, 0.28), 1.12),
        (Vec3::new(0.0, 0.49, 0.0), Vec3::new(0.24, 0.09, 0.20), 0.84),
    ] {
        cuboid(&mut mesh, center, half, tint(stone, shade));
    }
    for sign in [-1.0, 1.0] {
        cuboid(
            &mut mesh,
            Vec3::new(sign * 0.27, 0.62, 0.0),
            Vec3::new(0.085, 0.22, 0.085),
            tint(stone, 1.16),
        );
    }
    cuboid(
        &mut mesh,
        Vec3::new(0.0, 0.80, 0.0),
        Vec3::new(0.38, 0.08, 0.17),
        tint(stone, 1.28),
    );
    crystal(
        &mut mesh,
        Vec3::new(0.0, 0.86, 0.0),
        0.19,
        0.45,
        tint(accent, 1.15),
        5,
    );
    mesh
}

pub fn rock(m: &mut Mesh3D, base: Vec3, scale: Vec3, color: [f32; 3], phase: f32) {
    let rings = [(0.0, 0.7), (0.30, 1.0), (0.72, 0.85), (1.0, 0.20)];
    let vertex = |i: usize, j: usize| {
        let side = i % 7;
        let a = side as f32 * std::f32::consts::TAU / 7.0 + phase;
        let v = 0.88 + 0.12 * (side as f32 * 7.13 + j as f32 * 1.73).sin();
        base + Vec3::new(
            a.cos() * rings[j].1 * scale.x * v,
            rings[j].0 * scale.y,
            a.sin() * rings[j].1 * scale.z * v,
        )
    };
    for j in 0..rings.len() - 1 {
        for i in 0..7 {
            let p = [
                vertex(i, j),
                vertex(i + 1, j),
                vertex(i + 1, j + 1),
                vertex(i, j + 1),
            ];
            let normal = ((p[0] + p[1] + p[2] + p[3]) * 0.25 - base).normalize_or_zero();
            quad(m, p, normal, tint(color, 0.94 + (i % 3) as f32 * 0.035));
        }
    }
    let bottom = base;
    let top = base + Vec3::Y * scale.y;
    for i in 0..7 {
        tri(
            m,
            bottom,
            vertex(i + 1, 0),
            vertex(i, 0),
            -Vec3::Y,
            tint(color, 0.78),
        );
        tri(
            m,
            top,
            vertex(i, rings.len() - 1),
            vertex(i + 1, rings.len() - 1),
            Vec3::Y,
            tint(color, 0.96),
        );
    }
}

/// Faceted hand torch with a carved haft, leather grip, riveted iron cage and
/// a broad ember cup. The local origin is the mouth of the cup so the animated
/// flame shares the torch's hand-space transform exactly.
pub fn hand_torch() -> Mesh3D {
    let mut m = empty();
    let rings = [
        (-0.46, 0.030, [0.22, 0.105, 0.040]),
        (-0.43, 0.046, [0.38, 0.19, 0.065]),
        (-0.38, 0.041, [0.31, 0.14, 0.045]),
        (-0.33, 0.034, [0.28, 0.12, 0.040]),
        (-0.16, 0.034, [0.34, 0.16, 0.050]),
        (-0.12, 0.042, [0.43, 0.22, 0.075]),
        (-0.095, 0.050, [0.14, 0.075, 0.035]),
        (-0.055, 0.060, [0.12, 0.075, 0.035]),
        (-0.025, 0.073, [0.48, 0.30, 0.095]),
        (0.005, 0.075, [0.52, 0.33, 0.11]),
    ];
    let point = |i: usize, y: f32, radius: f32| {
        let a = i as f32 * std::f32::consts::TAU / 10.0;
        Vec3::new(a.cos() * radius, y, a.sin() * radius)
    };
    for j in 0..rings.len() - 1 {
        for i in 0..10 {
            let a = i as f32 * std::f32::consts::TAU / 10.0;
            let n = Vec3::new(a.cos(), 0.15, a.sin()).normalize_or_zero();
            quad(
                &mut m,
                [
                    point(i, rings[j].0, rings[j].1),
                    point(i + 1, rings[j].0, rings[j].1),
                    point(i + 1, rings[j + 1].0, rings[j + 1].1),
                    point(i, rings[j + 1].0, rings[j + 1].1),
                ],
                n,
                tint(rings[j].2, 0.88 + (i % 3) as f32 * 0.075),
            );
        }
    }

    // Raised leather bands make the grip legible against the dark wood.
    torch_band(&mut m, -0.34, -0.31, 0.039, 0.030, 10, [0.18, 0.075, 0.027]);
    torch_band(
        &mut m,
        -0.25,
        -0.225,
        0.036,
        0.029,
        10,
        [0.22, 0.095, 0.030],
    );
    torch_band(
        &mut m,
        -0.175,
        -0.15,
        0.037,
        0.029,
        10,
        [0.17, 0.067, 0.024],
    );
    torch_band(
        &mut m,
        -0.455,
        -0.425,
        0.048,
        0.026,
        10,
        [0.38, 0.22, 0.075],
    );

    // The open iron basket cages the flame instead of hiding it behind a cap.
    torch_band(
        &mut m,
        -0.035,
        -0.012,
        0.078,
        0.063,
        10,
        [0.13, 0.105, 0.071],
    );
    torch_band(&mut m, 0.103, 0.124, 0.052, 0.039, 10, [0.36, 0.25, 0.12]);
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0;
        let bottom = Vec3::new(a.cos() * 0.073, -0.014, a.sin() * 0.073);
        let top = Vec3::new(a.cos() * 0.048, 0.111, a.sin() * 0.048);
        let side = Vec3::new(-a.sin(), 0.0, a.cos()) * 0.008;
        let n = Vec3::new(a.cos(), 0.12, a.sin()).normalize_or_zero();
        quad(
            &mut m,
            [bottom - side, bottom + side, top + side, top - side],
            n,
            if i % 2 == 0 {
                [0.16, 0.11, 0.055]
            } else {
                [0.31, 0.20, 0.085]
            },
        );
        quad(
            &mut m,
            [bottom + side, bottom - side, top - side, top + side],
            -n,
            [0.075, 0.062, 0.046],
        );
    }
    m
}

fn torch_band(
    m: &mut Mesh3D,
    bottom: f32,
    top: f32,
    outer: f32,
    inner: f32,
    sides: usize,
    color: [f32; 3],
) {
    for i in 0..sides {
        let a0 = i as f32 * std::f32::consts::TAU / sides as f32;
        let a1 = (i + 1) as f32 * std::f32::consts::TAU / sides as f32;
        let p = |a: f32, y: f32, r: f32| Vec3::new(a.cos() * r, y, a.sin() * r);
        quad(
            m,
            [
                p(a0, bottom, outer),
                p(a1, bottom, outer),
                p(a1, top, outer),
                p(a0, top, outer),
            ],
            Vec3::new(a0.cos() + a1.cos(), 0.0, a0.sin() + a1.sin()),
            tint(color, 0.91 + (i % 3) as f32 * 0.07),
        );
        quad(
            m,
            [
                p(a0, top, outer),
                p(a1, top, outer),
                p(a1, top, inner),
                p(a0, top, inner),
            ],
            Vec3::Y,
            tint(color, 1.16),
        );
    }
}

/// A small, staged fracture decal placed flush against the enemy's chest.
/// Its branching count increases with successive sword hits.
pub fn fracture_marks(stage: u8) -> Mesh3D {
    let mut m = empty();
    let paths: [&[(f32, f32)]; 3] = [
        &[
            (-0.11, 0.16),
            (-0.035, 0.085),
            (-0.065, 0.035),
            (0.015, -0.018),
            (-0.005, -0.095),
            (0.065, -0.17),
        ],
        &[
            (-0.035, 0.085),
            (0.055, 0.105),
            (0.095, 0.06),
            (0.14, 0.035),
        ],
        &[(0.015, -0.018), (-0.095, -0.065), (-0.14, -0.13)],
    ];
    for path in paths.iter().take(stage.clamp(1, 3) as usize) {
        let points = path
            .iter()
            .map(|&(x, y)| Vec3::new(x, y, 0.0))
            .collect::<Vec<_>>();
        fracture_stroke(&mut m, &points, 0.019, 0.0, [0.92, 0.48, 0.12]);
        fracture_stroke(&mut m, &points, 0.008, 0.003, [0.14, 0.055, 0.025]);
    }
    m
}

fn fracture_stroke(m: &mut Mesh3D, points: &[Vec3], width: f32, z: f32, color: [f32; 3]) {
    for segment in points.windows(2) {
        let a = Vec3::new(segment[0].x, segment[0].y, z);
        let b = Vec3::new(segment[1].x, segment[1].y, z);
        let direction = (b - a).normalize_or_zero();
        let side = Vec3::new(-direction.y, direction.x, 0.0) * (width * 0.5);
        quad(m, [a - side, a + side, b + side, b - side], Vec3::Z, color);
    }
}

pub fn flame() -> Mesh3D {
    let mut m = empty();
    let rings = [(0.0, 0.24), (0.25, 0.56), (0.64, 0.38), (1.0, 0.0)];
    for j in 0..3 {
        for i in 0..6 {
            let at = |i: usize, j: usize| {
                let a = i as f32 * std::f32::consts::TAU / 6.0;
                Vec3::new(
                    a.cos() * rings[j].1 + rings[j].0 * 0.11,
                    rings[j].0,
                    a.sin() * rings[j].1,
                )
            };
            let a = i as f32 * std::f32::consts::TAU / 6.0;
            quad(
                &mut m,
                [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                Vec3::new(a.cos(), 0.3, a.sin()),
                // Near-white ramp, hottest at the tip: the draw's material
                // colour gives each biome its own flame (amber, spirit-cyan, ember).
                [1.0, 0.86 + (i % 3) as f32 * 0.07, 0.72 + j as f32 * 0.09],
            );
        }
    }
    m
}

/// Subtle broad pigment variation, not photographic pores/grain. Sculpted
/// planes supply the detail. Shared and mipmapped, with a nearly flat normal.
pub fn stone_texture() -> (velvet_render::TextureData3D, velvet_render::TextureData3D) {
    const N: usize = 256;
    let mut height = vec![0.0f32; N * N];
    fn value(x: f32, y: f32, period: i32) -> f32 {
        let hash = |x: i32, y: i32| {
            let mut r = rng::Rng::stream(
                617,
                "stone",
                (x.rem_euclid(period) + y.rem_euclid(period) * period) as u64,
            );
            r.unit()
        };
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let s = |v: f32| v * v * (3.0 - 2.0 * v);
        let u = s(x - x.floor());
        let v = s(y - y.floor());
        (hash(ix, iy) * (1.0 - u) + hash(ix + 1, iy) * u) * (1.0 - v)
            + (hash(ix, iy + 1) * (1.0 - u) + hash(ix + 1, iy + 1) * u) * v
    }
    for y in 0..N {
        for x in 0..N {
            let u = x as f32 / N as f32;
            let v = y as f32 / N as f32;
            let h = value(u * 4.0, v * 4.0, 4) * 0.55
                + value(u * 12.0, v * 12.0, 12) * 0.34
                + value(u * 24.0, v * 24.0, 24) * 0.11;
            height[y * N + x] = h;
        }
    }
    let mut rgba = vec![0u8; N * N * 4];
    let mut normals = rgba.clone();
    for y in 0..N {
        for x in 0..N {
            let h = height[y * N + x];
            let c = 0.74 + h * 0.26;
            let i = (y * N + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[
                (c * 255.0) as u8,
                (c * 255.0) as u8,
                (c * 255.0) as u8,
                255,
            ]);
            let dx = height[y * N + (x + 1) % N] - height[y * N + (x + N - 1) % N];
            let dy = height[((y + 1) % N) * N + x] - height[((y + N - 1) % N) * N + x];
            let n = Vec3::new(-dx * 0.12, -dy * 0.12, 1.0).normalize_or_zero();
            normals[i..i + 4].copy_from_slice(&[
                ((n.x * 0.5 + 0.5) * 255.0) as u8,
                ((n.y * 0.5 + 0.5) * 255.0) as u8,
                ((n.z * 0.5 + 0.5) * 255.0) as u8,
                255,
            ]);
        }
    }
    (
        velvet_render::TextureData3D {
            width: N as u32,
            height: N as u32,
            rgba,
        },
        velvet_render::TextureData3D {
            width: N as u32,
            height: N as u32,
            rgba: normals,
        },
    )
}
/// A passage's static shell: the web prototype's `corridorMesh`.
pub fn corridor(map: &Blueprint, catalog: &Catalog, id: usize) -> Mesh3D {
    super::art::chambers::corridor_mesh(map, catalog, id)
}

fn stair_riser(
    mesh: &mut Mesh3D,
    at: Vec3,
    side: Vec3,
    axis: Vec3,
    half_width: f32,
    from_y: f32,
    to_y: f32,
    color: [f32; 3],
) {
    if (to_y - from_y).abs() < 0.001 {
        return;
    }
    let at = Vec3::new(at.x, 0.0, at.z);
    let left = at - side * half_width;
    let right = at + side * half_width;
    let low = from_y.min(to_y);
    let high = from_y.max(to_y);
    let (corners, normal) = if to_y > from_y {
        (
            [
                left + Vec3::Y * low,
                right + Vec3::Y * low,
                right + Vec3::Y * high,
                left + Vec3::Y * high,
            ],
            -axis,
        )
    } else {
        (
            [
                left + Vec3::Y * high,
                right + Vec3::Y * high,
                right + Vec3::Y * low,
                left + Vec3::Y * low,
            ],
            axis,
        )
    };
    quad(mesh, corners, normal, tint(color, 0.82));
}

#[cfg(test)]
mod stair_tests {
    use super::*;

    fn closed_surface(mesh: &Mesh3D) -> bool {
        let point = |index: u32| {
            let [x, y, z] = mesh.vertices[index as usize].position;
            [
                (x * 100_000.0).round() as i32,
                (y * 100_000.0).round() as i32,
                (z * 100_000.0).round() as i32,
            ]
        };
        let mut edges = std::collections::BTreeMap::new();
        for triangle in mesh.indices.chunks_exact(3) {
            for (a, b) in [
                (triangle[0], triangle[1]),
                (triangle[1], triangle[2]),
                (triangle[2], triangle[0]),
            ] {
                let a = point(a);
                let b = point(b);
                let key = if a <= b { (a, b) } else { (b, a) };
                *edges.entry(key).or_insert(0usize) += 1;
            }
        }
        !edges.is_empty() && edges.values().all(|&uses| uses == 2)
    }

    #[test]
    fn risers_use_absolute_heights_and_face_downhill() {
        for (from_y, to_y, expected_normal) in [(1.35, 1.5, -Vec3::X), (1.5, 1.35, Vec3::X)] {
            let mut mesh = empty();
            stair_riser(
                &mut mesh,
                Vec3::new(3.0, 9.0, 4.0),
                Vec3::Z,
                Vec3::X,
                1.0,
                from_y,
                to_y,
                [0.6, 0.5, 0.4],
            );
            assert_eq!(mesh.vertices.len(), 6);
            let min_y = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::INFINITY, f32::min);
            let max_y = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position[1])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!((min_y - 1.35).abs() < 1.0e-5);
            assert!((max_y - 1.5).abs() < 1.0e-5);
            assert!(mesh.vertices.iter().all(|vertex| {
                Vec3::new(vertex.normal[0], vertex.normal[1], vertex.normal[2]).dot(expected_normal)
                    > 0.999
            }));
        }
    }

    #[test]
    fn wayfinding_inlays_follow_the_exit_and_stay_close_to_the_floor() {
        let mut mesh = empty();
        let center = Vec3::new(4.0, 2.0, -3.0);
        let forward = Vec3::X;
        wayfinding_glyph(
            &mut mesh,
            center,
            forward,
            [0.24, 0.28, 0.27],
            [0.20, 0.85, 0.90],
        );

        assert_eq!(mesh.vertices.len(), 24);
        assert!(mesh.vertices.iter().all(|vertex| {
            vertex.position.iter().all(|value| value.is_finite())
                && vertex.position[1] < center.y
                && vertex.position[1] > center.y - 0.03
                && Vec3::new(vertex.normal[0], vertex.normal[1], vertex.normal[2]).dot(Vec3::Y)
                    > 0.999
        }));
        let furthest_forward = mesh
            .vertices
            .iter()
            .map(|vertex| {
                (Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2]) - center)
                    .dot(forward)
            })
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(furthest_forward > 0.10, "glyph must point through the exit");
    }

    #[test]
    fn hand_torch_is_a_complete_caged_mesh_with_grip_below_the_flame_origin() {
        let mesh = hand_torch();
        let low = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let high = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(low < -0.45, "haft and pommel should extend below the cup");
        assert!(high > 0.12, "the flame cage should rise above the cup");
        assert!(mesh.vertices.len() > 300);
        assert!(mesh
            .indices
            .iter()
            .all(|&index| (index as usize) < mesh.vertices.len()));
    }

    #[test]
    fn skeleton_fractures_gain_branches_with_each_damage_stage() {
        let stages = [fracture_marks(1), fracture_marks(2), fracture_marks(3)];
        assert!(stages[0].indices.len() < stages[1].indices.len());
        assert!(stages[1].indices.len() < stages[2].indices.len());
        assert!(stages.iter().all(|mesh| {
            mesh.vertices.iter().all(|vertex| {
                vertex
                    .position
                    .into_iter()
                    .chain(vertex.normal)
                    .chain(vertex.color)
                    .all(f32::is_finite)
            }) && mesh
                .indices
                .iter()
                .all(|&index| (index as usize) < mesh.vertices.len())
        }));
    }

    #[test]
    fn tunnel_ribs_and_plinths_stay_out_of_the_walking_volume() {
        let catalog = Catalog::bundled();
        for seed in [0, 7, 42, 4815] {
            let map = generate(
                seed,
                &Config::default(),
                &catalog,
                &gameplay::Difficulty::default(),
            )
            .unwrap();
            for c in &map.connections {
                let mesh = corridor(&map, &catalog, c.id);
                let axis = horizontal(c.end - c.start);
                let length = axis.length();
                let dir = axis.normalize_or_zero();
                let side = Vec3::new(-dir.z, 0.0, dir.x);
                for v in &mesh.vertices {
                    let p = Vec3::new(v.position[0], v.position[1], v.position[2]);
                    let rel = p - c.start;
                    let t = (horizontal(rel).dot(dir) / length).clamp(0.0, 1.0);
                    let lateral = rel.dot(side).abs();
                    let floor = c.start.y + (c.end.y - c.start.y) * t;
                    if lateral < c.width * 0.5 - 0.2 {
                        let y = p.y - floor;
                        assert!(
                            y <= 0.17 || y >= 2.6,
                            "seed {seed} corridor {}: geometry at {y:.2} m inside the passage",
                            c.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn arched_doorways_and_ashlar_paving_keep_the_passage_clear() {
        let catalog = Catalog::bundled();
        let mut arches = 0;
        for seed in 0..24 {
            let map = generate(
                seed,
                &Config::default(),
                &catalog,
                &gameplay::Difficulty::default(),
            )
            .unwrap();
            for room in map
                .rooms
                .iter()
                .filter(|r| super::super::architecture::ornate(r).is_some())
            {
                for socket in &room.sockets {
                    let mut mesh = empty();
                    doorway_arch(
                        &mut mesh,
                        socket,
                        catalog.biomes[0].wall,
                        [0.9, 0.6, 0.2],
                        map.seed,
                    );
                    arches += 1;
                    let forward =
                        Vec3::new(-socket.outward.x, 0.0, -socket.outward.z).normalize_or_zero();
                    let side = Vec3::new(-forward.z, 0.0, forward.x);
                    for vertex in &mesh.vertices {
                        let p =
                            Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2])
                                - socket.position;
                        assert!(
                            p.dot(side).abs() > socket.width * 0.5 - 0.01 || p.y >= 2.65,
                            "seed {seed}: arch intrudes into the doorway at {p:?}"
                        );
                    }
                }
                let mut floor = empty();
                super::super::surfaces::ashlar_floor(
                    &mut floor,
                    room,
                    Vec3::Z,
                    [0.3, 0.28, 0.24],
                    seed,
                );
                assert!(!floor.indices.is_empty());
                for vertex in &floor.vertices {
                    assert!(
                        vertex.position[1] <= room.center.y + 1.0e-4,
                        "paving rises above the walking plane"
                    );
                    assert!(vertex.position[1] >= room.center.y - 0.06);
                }
            }
        }
        assert!(arches > 30, "{arches}");
    }

    #[test]
    fn generated_door_frames_leave_jump_and_walk_clearance() {
        let catalog = Catalog::bundled();
        let map = generate(
            42,
            &Config::default(),
            &catalog,
            &gameplay::Difficulty::default(),
        )
        .unwrap();
        for socket in map.rooms.iter().flat_map(|room| &room.sockets) {
            let mut mesh = empty();
            doorway_frame(&mut mesh, socket, catalog.biomes[0].wall, map.seed);
            let forward = Vec3::new(-socket.outward.x, 0.0, -socket.outward.z).normalize_or_zero();
            let side = Vec3::new(-forward.z, 0.0, forward.x);
            assert!(mesh.vertices.iter().all(|vertex| {
                let point = Vec3::new(vertex.position[0], vertex.position[1], vertex.position[2])
                    - socket.position;
                point.dot(side).abs() > socket.width * 0.5 - 0.01 || point.y >= 2.65
            }));
        }
    }

    #[test]
    fn procedural_landmarks_have_finite_geometry_and_valid_indices() {
        for mesh in [
            crown(),
            crystal_cluster([0.35, 0.34, 0.30], [0.20, 0.85, 0.90]),
            runestone([0.35, 0.34, 0.30]),
            rune_spiral(),
            exit_altar([0.35, 0.34, 0.30], [1.0, 0.35, 0.16]),
        ] {
            assert!(!mesh.vertices.is_empty());
            assert!(!mesh.indices.is_empty());
            assert!(mesh
                .indices
                .iter()
                .all(|&index| (index as usize) < mesh.vertices.len()));
            assert!(mesh.vertices.iter().all(|vertex| {
                vertex
                    .position
                    .into_iter()
                    .chain(vertex.normal)
                    .chain(vertex.color)
                    .all(f32::is_finite)
            }));
        }
    }

    #[test]
    fn cave_formations_and_chest_coins_are_closed_solid_meshes() {
        let mut boulder = empty();
        rock(
            &mut boulder,
            Vec3::ZERO,
            Vec3::new(0.42, 0.24, 0.36),
            [0.42, 0.38, 0.29],
            0.31,
        );
        let mut stalactite = empty();
        crystal(
            &mut stalactite,
            Vec3::ZERO,
            0.16,
            -0.82,
            [0.18, 0.78, 0.82],
            6,
        );
        for (name, mesh) in [
            ("rock", boulder),
            ("crystal", stalactite),
            (
                "crystal cluster",
                crystal_cluster([0.35, 0.34, 0.30], [0.20, 0.85, 0.90]),
            ),
            ("chest gold", chest_gold()),
        ] {
            assert!(closed_surface(&mesh), "{name} mesh has exposed edges");
        }
    }
}

pub fn enemy() -> Mesh3D {
    let mut m = empty();
    crystal(
        &mut m,
        Vec3::new(0.0, 0.12, 0.0),
        0.48,
        1.30,
        [0.20, 0.25, 0.29],
        7,
    );
    crystal(
        &mut m,
        Vec3::new(0.0, 1.13, 0.0),
        0.31,
        0.45,
        [0.34, 0.40, 0.43],
        6,
    );
    for side in [-1.0, 1.0] {
        crystal(
            &mut m,
            Vec3::new(side * 0.48, 0.45, 0.0),
            0.13,
            0.76,
            [0.20, 0.30, 0.32],
            5,
        );
        cuboid(
            &mut m,
            Vec3::new(side * 0.12, 1.35, -0.29),
            Vec3::new(0.06, 0.035, 0.04),
            [1.0, 0.35, 0.17],
        );
    }
    crystal(
        &mut m,
        Vec3::new(0.0, 0.62, -0.35),
        0.18,
        0.40,
        [0.86, 0.26, 0.18],
        5,
    );
    m
}

#[cfg(test)]
mod potion_tests {
    use super::*;

    #[test]
    fn healing_potion_is_bounded_opaque_and_has_readable_color() {
        let mesh = healing_potion();
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
        let mut red_vertices = 0;
        for vertex in &mesh.vertices {
            assert!(vertex.position.iter().all(|value| value.is_finite()));
            assert!((0.0..=1.0).contains(&vertex.color[3]));
            assert_eq!(vertex.color[3], 1.0);
            red_vertices += usize::from(vertex.color[0] > vertex.color[1] * 2.0);
        }
        assert!(
            red_vertices > 40,
            "the red healing liquid must remain legible"
        );
        let min_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((min_y - 0.03).abs() < 0.001);
        assert!((max_y - 1.0).abs() < 0.001);
    }
}
