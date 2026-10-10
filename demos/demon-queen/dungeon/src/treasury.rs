//! Treasury art kit. Procedural carpet and round rug, coin-covered hoards,
//! altars with relics, painted urns, candles, candelabra, a red-and-gold
//! chandelier, heraldic banners, swords stuck in the gold and an overflowing
//! open chest. Everything is built once per room into two static meshes:
//! a vertex-coloured `detail` mesh and an emissive `glow` mesh (flames, gem
//! cores, gold threads). Two draw calls per treasury, whatever the detail.
use super::vaults::{Piece, Relic, Vault};
use super::visual::{crown, crystal, cuboid, empty, quad, tint, tri};
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use velvet_dungeon::{horizontal, rng::Rng};
use velvet_math::{Mat4, Vec3};
use velvet_render::{Mesh3D, MeshVertex3D};

pub const RED: [f32; 3] = [0.48, 0.05, 0.042];
pub const DEEP_RED: [f32; 3] = [0.22, 0.028, 0.024];
pub const GOLD_TRIM: [f32; 3] = [0.9, 0.58, 0.16];
pub const DARK_GOLD: [f32; 3] = [0.58, 0.33, 0.06];
const WAX: [f32; 3] = [0.96, 0.88, 0.68];
const IRON: [f32; 3] = [0.13, 0.11, 0.10];
const STEEL: [f32; 3] = [0.74, 0.77, 0.84];
const NAVY: [f32; 3] = [0.07, 0.09, 0.17];
const FLAME_OUTER: [f32; 3] = [1.0, 0.42, 0.07];
const FLAME_CORE: [f32; 3] = [1.0, 0.86, 0.5];
const GEMS: [[f32; 3]; 4] = [
    [0.9, 0.06, 0.12],
    [0.08, 0.32, 1.0],
    [0.1, 0.8, 0.28],
    [0.7, 0.2, 1.0],
];

pub struct Meshes {
    pub detail: Mesh3D,
    pub glow: Mesh3D,
}

// ---------------------------------------------------------------- helpers

pub(super) fn basis(n: Vec3) -> (Vec3, Vec3) {
    let helper = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let u = helper.cross(n).normalize_or_zero();
    let v = n.cross(u).normalize_or_zero();
    (u, v)
}

fn yaw_dir(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

/// Push one triangle of existing vertices, wound to face their normals.
fn push_tri(m: &mut Mesh3D, i0: u32, i1: u32, i2: u32) {
    let p = |i: u32| {
        let v = m.vertices[i as usize].position;
        Vec3::new(v[0], v[1], v[2])
    };
    let nrm = |i: u32| {
        let v = m.vertices[i as usize].normal;
        Vec3::new(v[0], v[1], v[2])
    };
    let face = (p(i1) - p(i0)).cross(p(i2) - p(i0));
    if face.length_squared() < 1.0e-14 {
        return;
    }
    if face.dot(nrm(i0) + nrm(i1) + nrm(i2)) < 0.0 {
        m.indices.extend([i0, i2, i1]);
    } else {
        m.indices.extend([i0, i1, i2]);
    }
}

fn vertex(m: &mut Mesh3D, p: Vec3, n: Vec3, color: [f32; 3]) -> u32 {
    m.vertices.push(MeshVertex3D {
        position: p.to_array(),
        normal: n.to_array(),
        uv: [0.0, 0.0],
        color: [color[0], color[1], color[2], 1.0],
    });
    (m.vertices.len() - 1) as u32
}

/// Smooth-shaded surface of revolution around `axis` through `base`;
/// `profile` lists (height along axis, radius). Normals are shared around
/// the ring and across gentle profile bends; hard creases stay crisp.
#[allow(clippy::too_many_arguments)]
pub(super) fn lathe(
    m: &mut Mesh3D,
    base: Vec3,
    axis: Vec3,
    profile: &[(f32, f32)],
    sides: usize,
    color: &dyn Fn(usize) -> [f32; 3],
    caps: bool,
) {
    if profile.len() < 2 {
        return;
    }
    let sides = (sides + sides / 2).max(8);
    let (u, v) = basis(axis);
    let seg: Vec<(f32, f32)> = profile
        .windows(2)
        .map(|w| {
            let (h0, r0) = w[0];
            let (h1, r1) = w[1];
            let (dh, dr) = (h1 - h0, r1 - r0);
            let len = (dh * dh + dr * dr).sqrt().max(1.0e-5);
            (dh / len, -dr / len)
        })
        .collect();
    let blend = |a: (f32, f32), b: (f32, f32)| {
        if a.0 * b.0 + a.1 * b.1 > 0.7 {
            let s = (a.0 + b.0, a.1 + b.1);
            let l = (s.0 * s.0 + s.1 * s.1).sqrt().max(1.0e-5);
            (s.0 / l, s.1 / l)
        } else {
            a
        }
    };
    for j in 0..seg.len() {
        let (h0, r0) = profile[j];
        let (h1, r1) = profile[j + 1];
        let n0 = if j > 0 {
            blend(seg[j], seg[j - 1])
        } else {
            seg[j]
        };
        let n1 = if j + 1 < seg.len() {
            blend(seg[j], seg[j + 1])
        } else {
            seg[j]
        };
        let c = color(j);
        let start = m.vertices.len() as u32;
        for i in 0..=sides {
            let a = i as f32 * TAU / sides as f32;
            let radial = u * a.cos() + v * a.sin();
            // Soft key from one side gives turned metal a readable sheen.
            let sheen = 0.9 + 0.1 * a.cos();
            for (h, r, nn) in [(h0, r0, n0), (h1, r1, n1)] {
                let normal = (radial * nn.0 + axis * nn.1).normalize_or_zero();
                vertex(m, base + axis * h + radial * r, normal, tint(c, sheen));
            }
        }
        for i in 0..sides as u32 {
            let a0 = start + i * 2;
            let (a1, b0, b1) = (a0 + 1, a0 + 2, a0 + 3);
            push_tri(m, a0, b0, b1);
            push_tri(m, a0, b1, a1);
        }
    }
    if caps {
        let (h0, r0) = profile[0];
        let (h1, r1) = profile[profile.len() - 1];
        if r0 > 1.0e-3 {
            disc(m, base + axis * h0, -axis, r0, sides, tint(color(0), 0.7));
        }
        if r1 > 1.0e-3 {
            disc(
                m,
                base + axis * h1,
                axis,
                r1,
                sides,
                color(profile.len().saturating_sub(2)),
            );
        }
    }
}

/// Smooth torus: `radius` around `n`, round tube of radius `tube`.
pub(super) fn tube_ring(
    m: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    radius: f32,
    tube: f32,
    segments: usize,
    color: [f32; 3],
) {
    let (u, v) = basis(n);
    let segments = (segments + segments / 2).max(12);
    let sides = 8;
    let start = m.vertices.len() as u32;
    for i in 0..=segments {
        let a = i as f32 * TAU / segments as f32;
        let dir = u * a.cos() + v * a.sin();
        for j in 0..=sides {
            let phi = j as f32 * TAU / sides as f32;
            let normal = (dir * phi.cos() + n * phi.sin()).normalize_or_zero();
            let shade = 0.82 + 0.26 * (0.5 + 0.5 * phi.sin());
            vertex(
                m,
                c + dir * (radius + tube * phi.cos()) + n * (tube * phi.sin()),
                normal,
                tint(color, shade),
            );
        }
    }
    let row = sides as u32 + 1;
    for i in 0..segments as u32 {
        for j in 0..sides as u32 {
            let a = start + i * row + j;
            let b = start + (i + 1) * row + j;
            push_tri(m, a, b, b + 1);
            push_tri(m, a, b + 1, a + 1);
        }
    }
}

/// Darken the feet of free-standing pieces where they meet the floor: cheap
/// baked contact occlusion. Upward faces (rugs, coin tops) keep their tone.
fn ground(m: &mut Mesh3D, floor: f32) {
    for v in &mut m.vertices {
        let h = v.position[1] - floor;
        if (0.0..0.4).contains(&h) && v.normal[1] < 0.6 {
            let k = 0.55 + 0.45 * (h / 0.4);
            for c in 0..3 {
                v.color[c] *= k;
            }
        }
    }
}

pub(super) fn disc(m: &mut Mesh3D, c: Vec3, n: Vec3, r: f32, sides: usize, color: [f32; 3]) {
    let (u, v) = basis(n);
    for i in 0..sides {
        let a = i as f32 * TAU / sides as f32;
        let b = (i + 1) as f32 * TAU / sides as f32;
        tri(
            m,
            c,
            c + (u * a.cos() + v * a.sin()) * r,
            c + (u * b.cos() + v * b.sin()) * r,
            n,
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn annulus(
    m: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    r0: f32,
    r1: f32,
    sides: usize,
    color: [f32; 3],
    phase: f32,
) {
    let (u, v) = basis(n);
    for i in 0..sides {
        let a = i as f32 * TAU / sides as f32 + phase;
        let b = (i + 1) as f32 * TAU / sides as f32 + phase;
        let p = |ang: f32, r: f32| c + (u * ang.cos() + v * ang.sin()) * r;
        quad(m, [p(a, r0), p(b, r0), p(b, r1), p(a, r1)], n, color);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn star(
    m: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    up: Vec3,
    r_out: f32,
    r_in: f32,
    points: usize,
    color: [f32; 3],
) {
    let u = up.cross(n).normalize_or_zero();
    let v = n.cross(u).normalize_or_zero();
    let count = points * 2;
    for i in 0..count {
        let a = i as f32 * TAU / count as f32;
        let b = (i + 1) as f32 * TAU / count as f32;
        let ra = if i % 2 == 0 { r_out } else { r_in };
        let rb = if i % 2 == 0 { r_in } else { r_out };
        tri(
            m,
            c,
            c + (v * a.cos() + u * a.sin()) * ra,
            c + (v * b.cos() + u * b.sin()) * rb,
            n,
            tint(color, if i % 2 == 0 { 1.08 } else { 0.9 }),
        );
    }
}

/// Box with orthonormal axes and half extents.
pub(super) fn block(m: &mut Mesh3D, c: Vec3, x: Vec3, y: Vec3, z: Vec3, h: Vec3, color: [f32; 3]) {
    let p = |sx: f32, sy: f32, sz: f32| c + x * (sx * h.x) + y * (sy * h.y) + z * (sz * h.z);
    for (normal, corners, shade) in [
        (
            z,
            [
                p(-1., -1., 1.),
                p(1., -1., 1.),
                p(1., 1., 1.),
                p(-1., 1., 1.),
            ],
            1.05,
        ),
        (
            -z,
            [
                p(1., -1., -1.),
                p(-1., -1., -1.),
                p(-1., 1., -1.),
                p(1., 1., -1.),
            ],
            0.9,
        ),
        (
            x,
            [
                p(1., -1., 1.),
                p(1., -1., -1.),
                p(1., 1., -1.),
                p(1., 1., 1.),
            ],
            0.97,
        ),
        (
            -x,
            [
                p(-1., -1., -1.),
                p(-1., -1., 1.),
                p(-1., 1., 1.),
                p(-1., 1., -1.),
            ],
            0.97,
        ),
        (
            y,
            [
                p(-1., 1., 1.),
                p(1., 1., 1.),
                p(1., 1., -1.),
                p(-1., 1., -1.),
            ],
            1.15,
        ),
        (
            -y,
            [
                p(-1., -1., -1.),
                p(1., -1., -1.),
                p(1., -1., 1.),
                p(-1., -1., 1.),
            ],
            0.7,
        ),
    ] {
        quad(m, corners, normal, tint(color, shade));
    }
}

/// Rectangular bar between two points; `flat` orients its wide side.
pub(super) fn bar(
    m: &mut Mesh3D,
    a: Vec3,
    b: Vec3,
    half_w: f32,
    half_t: f32,
    flat: Vec3,
    color: [f32; 3],
) {
    let axis = (b - a).normalize_or_zero();
    let mut side = flat - axis * flat.dot(axis);
    if side.length_squared() < 1.0e-6 {
        side = basis(axis).0;
    }
    let side = side.normalize_or_zero();
    let thick = axis.cross(side).normalize_or_zero();
    block(
        m,
        (a + b) * 0.5,
        side,
        axis,
        thick,
        Vec3::new(half_w, (b - a).length() * 0.5, half_t),
        color,
    );
}

/// Cheap octagonal coin (top + rim), oriented by its face normal.
fn coin(m: &mut Mesh3D, c: Vec3, n: Vec3, radius: f32, thick: f32, color: [f32; 3], phase: f32) {
    let (u, v) = basis(n);
    let top = c + n * (thick * 0.5);
    let bottom = c - n * (thick * 0.5);
    for i in 0..8 {
        let a = i as f32 * TAU / 8.0 + phase;
        let b = (i + 1) as f32 * TAU / 8.0 + phase;
        let ra = u * a.cos() + v * a.sin();
        let rb = u * b.cos() + v * b.sin();
        tri(
            m,
            top,
            top + ra * radius,
            top + rb * radius,
            n,
            tint(color, if i % 2 == 0 { 1.12 } else { 0.98 }),
        );
        quad(
            m,
            [
                bottom + ra * radius,
                bottom + rb * radius,
                top + rb * radius,
                top + ra * radius,
            ],
            (ra + rb).normalize_or_zero(),
            tint(color, 0.72),
        );
    }
}

/// Copy another mesh, transformed, multiplying its vertex colours.
fn append(m: &mut Mesh3D, src: &Mesh3D, transform: Mat4, color: [f32; 3]) {
    let base = m.vertices.len() as u32;
    for vertex in &src.vertices {
        let p = transform.transform_point3(Vec3::new(
            vertex.position[0],
            vertex.position[1],
            vertex.position[2],
        ));
        let n = transform
            .transform_vector3(Vec3::new(
                vertex.normal[0],
                vertex.normal[1],
                vertex.normal[2],
            ))
            .normalize_or_zero();
        m.vertices.push(MeshVertex3D {
            position: p.to_array(),
            normal: n.to_array(),
            uv: vertex.uv,
            color: [
                vertex.color[0] * color[0],
                vertex.color[1] * color[1],
                vertex.color[2] * color[2],
                vertex.color[3],
            ],
        });
    }
    m.indices.extend(src.indices.iter().map(|i| i + base));
}

fn flame(glow: &mut Mesh3D, base: Vec3, size: f32) {
    lathe(
        glow,
        base,
        Vec3::Y,
        &[
            (0.0, 0.0),
            (0.012 * size, 0.022 * size),
            (0.04 * size, 0.028 * size),
            (0.08 * size, 0.014 * size),
            (0.11 * size, 0.0),
        ],
        6,
        &|_| FLAME_OUTER,
        false,
    );
    lathe(
        glow,
        base + Vec3::Y * (0.006 * size),
        Vec3::Y,
        &[(0.0, 0.0), (0.02 * size, 0.013 * size), (0.05 * size, 0.0)],
        5,
        &|_| FLAME_CORE,
        false,
    );
}

/// Wax candle with drips and a flame; returns the flame centre.
pub(super) fn candle(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    radius: f32,
    height: f32,
    rng: &mut Rng,
) -> Vec3 {
    let wax = tint(WAX, rng.range(0.9, 1.04));
    lathe(
        m,
        base,
        Vec3::Y,
        &[
            (0.0, radius * 1.06),
            (height * 0.1, radius),
            (height, radius),
            (height, radius * 0.7),
        ],
        8,
        &|_| wax,
        true,
    );
    for k in 0..2 {
        let a = rng.range(0.0, TAU);
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let len = rng.range(0.03, height * 0.45);
        cuboid(
            m,
            base + d * (radius * 0.98) + Vec3::Y * (height - len * 0.5),
            Vec3::new(0.008 + k as f32 * 0.003, len * 0.5, 0.008),
            tint(wax, 1.06),
        );
    }
    cuboid(
        m,
        base + Vec3::Y * (height + 0.012),
        Vec3::new(0.003, 0.012, 0.003),
        [0.05, 0.04, 0.03],
    );
    let flame_base = base + Vec3::Y * (height + 0.014);
    flame(glow, flame_base, (radius / 0.045).clamp(0.8, 1.35));
    flame_base + Vec3::Y * 0.05
}

// ------------------------------------------------------------ the pieces

fn carpet(m: &mut Mesh3D, glow: &mut Mesh3D, from: Vec3, to: Vec3, hw: f32) {
    let axis = horizontal(to - from).normalize_or_zero();
    let side = Vec3::new(-axis.z, 0.0, axis.x);
    let len = horizontal(to - from).length();
    let rect = |m: &mut Mesh3D, y: f32, s0: f32, s1: f32, t0: f32, t1: f32, c: [f32; 3]| {
        let p = |s: f32, t: f32| from + side * s + axis * t + Vec3::Y * y;
        quad(m, [p(s0, t0), p(s1, t0), p(s1, t1), p(s0, t1)], Vec3::Y, c);
    };
    rect(m, 0.010, -hw, hw, 0.0, len, DEEP_RED);
    rect(
        m,
        0.011,
        -hw + 0.11,
        hw - 0.11,
        0.1,
        len - 0.05,
        tint(RED, 0.86),
    );
    for s in [-1.0f32, 1.0] {
        let outer = s * (hw - 0.065);
        rect(m, 0.012, outer - 0.022, outer + 0.022, 0.03, len, GOLD_TRIM);
        let inner = s * (hw - 0.2);
        rect(
            m,
            0.012,
            inner - 0.011,
            inner + 0.011,
            0.14,
            len - 0.06,
            tint(GOLD_TRIM, 0.9),
        );
        // Subtle glowing thread along the outer trim.
        rect(
            glow,
            0.0125,
            outer - 0.006,
            outer + 0.006,
            0.03,
            len,
            [0.28, 0.15, 0.03],
        );
    }
    let motifs = ((len - 0.4) / 0.9).floor().max(0.0) as usize;
    for k in 0..motifs {
        let t = 0.55 + k as f32 * 0.9;
        let c = from + axis * t + Vec3::Y * 0.0125;
        let diamond = |m: &mut Mesh3D, c: Vec3, a: f32, b: f32, color: [f32; 3]| {
            quad(
                m,
                [c - axis * a, c + side * b, c + axis * a, c - side * b],
                Vec3::Y,
                color,
            );
        };
        diamond(m, c, 0.2, 0.13, GOLD_TRIM);
        diamond(m, c + Vec3::Y * 0.001, 0.12, 0.075, DEEP_RED);
        diamond(m, c + Vec3::Y * 0.002, 0.05, 0.03, GOLD_TRIM);
    }
    // Tassel fringe at the door end.
    let tassels = (hw * 2.0 / 0.07) as usize;
    for k in 0..tassels {
        let s = -hw + 0.035 + k as f32 * 0.07;
        rect(
            m,
            0.0105,
            s - 0.018,
            s + 0.018,
            -0.1,
            0.0,
            [0.92, 0.72, 0.36],
        );
    }
}

fn rug(m: &mut Mesh3D, glow: &mut Mesh3D, c: Vec3, r: f32) {
    let up = Vec3::Y;
    // Tassel fringe.
    for k in 0..56 {
        let a = k as f32 * TAU / 56.0;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let t = Vec3::new(-d.z, 0.0, d.x);
        let p0 = c + d * r + Vec3::Y * 0.0105;
        let p1 = c + d * (r + 0.09) + Vec3::Y * 0.0105;
        quad(
            m,
            [p0 - t * 0.02, p1 - t * 0.02, p1 + t * 0.02, p0 + t * 0.02],
            up,
            [0.92, 0.72, 0.36],
        );
    }
    annulus(m, c + Vec3::Y * 0.010, up, 0.0, r, 48, DEEP_RED, 0.0);
    annulus(
        m,
        c + Vec3::Y * 0.0112,
        up,
        r * 0.86,
        r * 0.92,
        48,
        GOLD_TRIM,
        0.0,
    );
    annulus(
        m,
        c + Vec3::Y * 0.0112,
        up,
        0.0,
        r * 0.83,
        48,
        tint(RED, 0.92),
        0.0,
    );
    // Ring of gold studs in the dark border.
    for k in 0..28 {
        let a = k as f32 * TAU / 28.0;
        disc(
            m,
            c + Vec3::new(a.cos(), 0.0, a.sin()) * (r * 0.955) + Vec3::Y * 0.0118,
            up,
            0.03,
            6,
            GOLD_TRIM,
        );
    }
    star(
        m,
        c + Vec3::Y * 0.0124,
        up,
        Vec3::X,
        r * 0.78,
        r * 0.33,
        8,
        GOLD_TRIM,
    );
    star(
        m,
        c + Vec3::Y * 0.0130,
        up,
        Vec3::X,
        r * 0.62,
        r * 0.28,
        8,
        tint(RED, 0.8),
    );
    annulus(
        m,
        c + Vec3::Y * 0.0136,
        up,
        r * 0.24,
        r * 0.29,
        32,
        GOLD_TRIM,
        0.0,
    );
    annulus(
        m,
        c + Vec3::Y * 0.0136,
        up,
        0.0,
        r * 0.24,
        32,
        DEEP_RED,
        0.0,
    );
    // Gold threads catching the light (emissive).
    annulus(
        glow,
        c + Vec3::Y * 0.0145,
        up,
        r * 0.885,
        r * 0.895,
        64,
        [0.34, 0.19, 0.04],
        0.0,
    );
    annulus(
        glow,
        c + Vec3::Y * 0.0145,
        up,
        r * 0.285,
        r * 0.292,
        40,
        [0.3, 0.17, 0.035],
        0.0,
    );
}

fn coin_color(rng: &mut Rng) -> [f32; 3] {
    tint(GOLD_TRIM, rng.range(0.82, 1.12))
}

/// Heap silhouette: a slumped cone (steep middle, soft crown and skirt).
fn heap_radius(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    (1.0 - t).powf(0.85) * (1.0 - 0.15 * t)
}

/// Heap height at normalized radial distance `d` (0 centre, 1 rim).
fn heap_height(d: f32, height: f32) -> f32 {
    let d = d.clamp(0.0, 1.0);
    // Inverse of heap_radius sampled coarsely, smooth enough for placement.
    let mut t = 1.0 - d;
    for _ in 0..6 {
        let r = heap_radius(t);
        t = (t + (r - d) * 0.8).clamp(0.0, 1.0);
    }
    height * t
}

fn hoard(m: &mut Mesh3D, glow: &mut Mesh3D, center: Vec3, radius: f32, height: f32, rng: &mut Rng) {
    // Dark gold body under a skin of individual coins.
    let rings = 5;
    let sides = 16;
    let point = |ring: usize, i: usize| {
        let t = ring as f32 / rings as f32;
        let r = radius * heap_radius(t) * 0.97;
        let a = i as f32 * TAU / sides as f32 + ring as f32 * 0.2;
        center + Vec3::new(a.cos() * r, height * t, a.sin() * r)
    };
    let top = center + Vec3::Y * height;
    for ring in 0..rings {
        for i in 0..sides {
            let j = (i + 1) % sides;
            let shade = 0.78 + ring as f32 * 0.07;
            if ring + 1 == rings {
                tri(
                    m,
                    point(ring, i),
                    point(ring, j),
                    top,
                    Vec3::Y,
                    tint(DARK_GOLD, shade),
                );
            } else {
                let q = [
                    point(ring, i),
                    point(ring, j),
                    point(ring + 1, j),
                    point(ring + 1, i),
                ];
                let n = ((q[0] + q[1] + q[2] + q[3]) * 0.25 - center).normalize_or_zero();
                quad(m, q, n, tint(DARK_GOLD, shade));
            }
        }
    }
    let count = (radius * radius * 260.0) as usize + 30;
    for k in 0..count {
        let rs = radius * 0.98 * rng.unit().sqrt();
        let a = rng.range(0.0, TAU);
        let x = a.cos() * rs;
        let z = a.sin() * rs;
        let y = heap_height(rs / radius, height);
        let slope = (heap_height((rs / radius - 0.05).max(0.0), height)
            - heap_height((rs / radius + 0.05).min(1.0), height))
            / (0.1 * radius);
        let radial = Vec3::new(a.cos(), 0.0, a.sin());
        let normal = (Vec3::Y + radial * slope).normalize_or_zero();
        let n = (normal + Vec3::new(rng.range(-0.4, 0.4), 0.0, rng.range(-0.4, 0.4)))
            .normalize_or_zero();
        let color = coin_color(rng);
        coin(
            m,
            center + Vec3::new(x, y, z) + n * 0.008,
            n,
            rng.range(0.06, 0.092),
            0.018,
            color,
            k as f32,
        );
    }
    // Stacks around the foot.
    for _ in 0..rng.index(4) + 4 {
        let a = rng.range(0.0, TAU);
        let d = radius * rng.range(0.92, 1.22);
        let base = center + Vec3::new(a.cos() * d, 0.0, a.sin() * d);
        let layers = 3 + rng.index(9);
        for l in 0..layers {
            let jitter = Vec3::new(rng.range(-0.008, 0.008), 0.0, rng.range(-0.008, 0.008));
            coin(
                m,
                base + jitter + Vec3::Y * (0.009 + l as f32 * 0.017),
                Vec3::Y,
                0.062,
                0.016,
                coin_color(rng),
                l as f32 * 0.4,
            );
        }
    }
    // Gems pressed into the gold.
    for _ in 0..3 + rng.index(3) {
        let rs = radius * rng.range(0.1, 0.8);
        let a = rng.range(0.0, TAU);
        let y = heap_height(rs / radius, height);
        let gem = GEMS[rng.index(GEMS.len())];
        crystal(
            glow,
            center + Vec3::new(a.cos() * rs, y - 0.02, a.sin() * rs),
            rng.range(0.035, 0.05),
            rng.range(0.08, 0.13),
            tint(gem, 0.5),
            5,
        );
    }
    if rng.chance(0.55) {
        let rs = radius * rng.range(0.0, 0.3);
        let a = rng.range(0.0, TAU);
        let y = heap_height(rs / radius, height);
        sword(
            m,
            center + Vec3::new(a.cos() * rs, y, a.sin() * rs),
            rng.range(0.0, TAU),
            rng.range(0.1, 0.35),
        );
    }
}

fn sword(m: &mut Mesh3D, entry: Vec3, yaw: f32, lean: f32) {
    let tilt = yaw_dir(yaw);
    let up = (Vec3::Y * lean.cos() + tilt * lean.sin()).normalize_or_zero();
    let flat = Vec3::new(tilt.z, 0.0, -tilt.x);
    let at = |t: f32| entry + up * t;
    bar(m, at(-0.18), at(0.55), 0.034, 0.006, flat, STEEL);
    let n = flat.cross(up).normalize_or_zero();
    tri(
        m,
        at(0.55) - flat * 0.034,
        at(0.55) + flat * 0.034,
        at(0.62),
        n,
        tint(STEEL, 1.1),
    );
    tri(
        m,
        at(0.55) - flat * 0.034,
        at(0.55) + flat * 0.034,
        at(0.62),
        -n,
        tint(STEEL, 0.9),
    );
    let hilt = |t: f32| entry - up * t;
    bar(
        m,
        hilt(0.18) - flat * 0.14,
        hilt(0.18) + flat * 0.14,
        0.018,
        0.02,
        up,
        GOLD_TRIM,
    );
    bar(
        m,
        hilt(0.2),
        hilt(0.36),
        0.018,
        0.018,
        flat,
        [0.32, 0.07, 0.05],
    );
    lathe(
        m,
        hilt(0.4),
        up,
        &[(0.0, 0.0), (0.02, 0.03), (0.04, 0.028), (0.06, 0.0)],
        6,
        &|_| GOLD_TRIM,
        false,
    );
}

fn altar(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    base: Vec3,
    yaw: f32,
    relic: Relic,
    stone: [f32; 3],
    rng: &mut Rng,
) {
    let front = yaw_dir(yaw);
    let side = Vec3::new(front.z, 0.0, -front.x);
    let y = |v: f32| base + Vec3::Y * v;
    cuboid(m, y(0.08), Vec3::new(0.4, 0.08, 0.4), tint(stone, 0.72));
    cuboid(m, y(0.18), Vec3::new(0.34, 0.02, 0.34), GOLD_TRIM);
    cuboid(m, y(0.56), Vec3::new(0.28, 0.36, 0.28), tint(stone, 1.0));
    for face in [side, -side, -front] {
        let across = Vec3::new(-face.z, 0.0, face.x);
        let c = y(0.56) + face * 0.281;
        let frame = tint(stone, 1.22);
        for sy in [-1.0f32, 1.0] {
            bar(
                m,
                c + Vec3::Y * (sy * 0.25) - across * 0.19,
                c + Vec3::Y * (sy * 0.25) + across * 0.19,
                0.018,
                0.012,
                Vec3::Y,
                frame,
            );
        }
        for sx in [-1.0f32, 1.0] {
            bar(
                m,
                c + across * (sx * 0.19) - Vec3::Y * 0.25,
                c + across * (sx * 0.19) + Vec3::Y * 0.25,
                0.018,
                0.012,
                across,
                frame,
            );
        }
        disc(m, c + face * 0.006, face, 0.1, 16, tint(stone, 0.8));
        star(
            m,
            c + face * 0.012,
            face,
            Vec3::Y,
            0.085,
            0.036,
            8,
            GOLD_TRIM,
        );
        disc(m, c + face * 0.016, face, 0.026, 10, tint(RED, 1.1));
    }
    cuboid(m, y(0.94), Vec3::new(0.32, 0.02, 0.32), GOLD_TRIM);
    cuboid(m, y(1.01), Vec3::new(0.37, 0.05, 0.37), tint(stone, 1.18));
    // Red cloth draped over the top and down the front, gold-edged.
    let top = 1.063;
    let hw = 0.26;
    let f = front * 0.375;
    quad(
        m,
        [
            y(top) - side * hw,
            y(top) + side * hw,
            y(top) + side * hw + f,
            y(top) - side * hw + f,
        ],
        Vec3::Y,
        RED,
    );
    let drop_top = y(top) + front * 0.376;
    let lower = drop_top - Vec3::Y * 0.42;
    let tip = drop_top - Vec3::Y * 0.58;
    quad(
        m,
        [
            drop_top - side * hw,
            drop_top + side * hw,
            lower + side * hw,
            lower - side * hw,
        ],
        front,
        RED,
    );
    tri(m, lower - side * hw, lower + side * hw, tip, front, RED);
    let edge = front * 0.003;
    for s in [-1.0f32, 1.0] {
        let a = drop_top + side * (s * (hw - 0.03)) + edge;
        let b = lower + side * (s * (hw - 0.03)) + edge;
        bar(m, a, b, 0.012, 0.002, side, GOLD_TRIM);
        bar(m, b, tip + edge, 0.012, 0.002, side, GOLD_TRIM);
    }
    star(
        m,
        drop_top - Vec3::Y * 0.2 + edge * 2.0,
        front,
        Vec3::Y,
        0.085,
        0.035,
        8,
        GOLD_TRIM,
    );
    lathe(
        m,
        tip - Vec3::Y * 0.02,
        -Vec3::Y,
        &[(0.0, 0.015), (0.03, 0.02), (0.08, 0.0)],
        6,
        &|_| GOLD_TRIM,
        false,
    );

    let t = y(top + 0.005);
    match relic {
        Relic::Crown => {
            for k in 0..16 {
                let a = rng.range(0.0, TAU);
                let r = 0.2 * rng.unit().sqrt();
                let h = 0.012 + (0.2 - r) * 0.25;
                let n = (Vec3::Y + Vec3::new(rng.range(-0.3, 0.3), 0.0, rng.range(-0.3, 0.3)))
                    .normalize_or_zero();
                coin(
                    m,
                    t + Vec3::new(a.cos() * r, h, a.sin() * r),
                    n,
                    0.055,
                    0.015,
                    coin_color(rng),
                    k as f32,
                );
            }
            let transform = Mat4::from_translation(t + Vec3::Y * 0.06)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(0.52, 0.52, 0.52));
            append(m, &crown(), transform, [1.08, 1.02, 0.9]);
            for k in 0..4 {
                let a = yaw + k as f32 * FRAC_PI_2 + 0.4;
                crystal(
                    glow,
                    t + Vec3::new(a.cos() * 0.14, 0.12, a.sin() * 0.14),
                    0.022,
                    0.05,
                    tint(GEMS[k % 3], 0.55),
                    5,
                );
            }
        }
        Relic::Helm => {
            // Gilded knight bust with a red crest and a small crown.
            let bust = &|j: usize| {
                if j == 7 {
                    [0.12, 0.08, 0.05]
                } else {
                    tint(GOLD_TRIM, 0.92)
                }
            };
            lathe(
                m,
                t,
                Vec3::Y,
                &[
                    (0.0, 0.2),
                    (0.06, 0.24),
                    (0.14, 0.2),
                    (0.2, 0.1),
                    (0.26, 0.075),
                    (0.3, 0.12),
                    (0.38, 0.145),
                    (0.44, 0.15),
                    (0.47, 0.148),
                    (0.55, 0.13),
                    (0.61, 0.08),
                    (0.64, 0.0),
                ],
                12,
                bust,
                true,
            );
            for k in 0..6 {
                let a = -0.9 + k as f32 * 0.36;
                let p0 = t + Vec3::Y * (0.6 + a.cos() * 0.06) + front * (a.sin() * 0.14);
                let p1 = t + Vec3::Y * (0.62 + a.cos() * 0.09) + front * (a.sin() * 0.14 - 0.07);
                bar(m, p0, p1, 0.022, 0.03, side, tint(RED, 1.1));
            }
            let transform = Mat4::from_translation(t + Vec3::Y * 0.6)
                * Mat4::from_rotation_y(yaw)
                * Mat4::from_scale(Vec3::new(0.34, 0.34, 0.34));
            append(m, &crown(), transform, [1.05, 1.0, 0.9]);
        }
        Relic::Armillary => {
            lathe(
                m,
                t,
                Vec3::Y,
                &[
                    (0.0, 0.12),
                    (0.03, 0.1),
                    (0.04, 0.04),
                    (0.2, 0.025),
                    (0.22, 0.05),
                    (0.24, 0.02),
                ],
                10,
                &|_| GOLD_TRIM,
                true,
            );
            let c = t + Vec3::Y * 0.46;
            tube_ring(m, c, Vec3::Y, 0.21, 0.011, 28, GOLD_TRIM);
            tube_ring(
                m,
                c,
                (Vec3::X + Vec3::Y * 0.4).normalize_or_zero(),
                0.21,
                0.011,
                28,
                tint(GOLD_TRIM, 0.92),
            );
            tube_ring(
                m,
                c,
                (Vec3::Z + Vec3::X * 0.5).normalize_or_zero(),
                0.19,
                0.01,
                28,
                tint(GOLD_TRIM, 1.05),
            );
            tube_ring(m, c, side, 0.235, 0.014, 28, DARK_GOLD);
            bar(
                m,
                c - Vec3::Y * 0.235,
                c - Vec3::Y * 0.22,
                0.02,
                0.02,
                side,
                GOLD_TRIM,
            );
            lathe(
                m,
                c - Vec3::Y * 0.07,
                Vec3::Y,
                &[
                    (0.0, 0.0),
                    (0.02, 0.05),
                    (0.07, 0.07),
                    (0.12, 0.05),
                    (0.14, 0.0),
                ],
                10,
                &|_| tint(GOLD_TRIM, 1.05),
                false,
            );
            star(
                m,
                c + Vec3::Y * 0.3,
                front,
                Vec3::Y,
                0.08,
                0.03,
                8,
                GOLD_TRIM,
            );
            star(
                m,
                c + Vec3::Y * 0.3,
                -front,
                Vec3::Y,
                0.08,
                0.03,
                8,
                GOLD_TRIM,
            );
            crystal(glow, c - Vec3::Y * 0.01, 0.03, 0.07, [0.5, 0.3, 0.06], 6);
        }
        Relic::Gems => {
            lathe(
                m,
                t,
                Vec3::Y,
                &[(0.0, 0.07), (0.02, 0.14), (0.05, 0.19), (0.055, 0.17)],
                12,
                &|_| GOLD_TRIM,
                true,
            );
            for (k, (dx, dz, r, h)) in [
                (0.0, 0.0, 0.07, 0.3),
                (0.09, 0.05, 0.05, 0.2),
                (-0.08, 0.06, 0.045, 0.17),
                (0.02, -0.1, 0.04, 0.14),
            ]
            .into_iter()
            .enumerate()
            {
                let p = t + side * dx + front * dz + Vec3::Y * 0.04;
                crystal(glow, p, r, h, tint(GEMS[k % GEMS.len()], 0.6), 6);
                crystal(m, p, r * 1.08, h * 0.96, tint(GEMS[k % GEMS.len()], 0.7), 6);
            }
        }
        Relic::Chalice => {
            let cup = &|j: usize| if j == 4 { tint(RED, 1.2) } else { GOLD_TRIM };
            lathe(
                m,
                t,
                Vec3::Y,
                &[
                    (0.0, 0.1),
                    (0.02, 0.1),
                    (0.035, 0.03),
                    (0.16, 0.025),
                    (0.18, 0.05),
                    (0.26, 0.1),
                    (0.3, 0.11),
                    (0.33, 0.11),
                ],
                12,
                cup,
                false,
            );
            disc(m, t + Vec3::Y * 0.3, Vec3::Y, 0.1, 12, [0.3, 0.02, 0.03]);
            for k in 0..3 {
                let a = yaw + 1.2 + k as f32 * 1.9;
                let base = t + Vec3::new(a.cos() * 0.2, 0.0, a.sin() * 0.2);
                for l in 0..(3 + k * 2) {
                    coin(
                        m,
                        base + Vec3::Y * (0.009 + l as f32 * 0.017),
                        Vec3::Y,
                        0.05,
                        0.016,
                        coin_color(rng),
                        l as f32,
                    );
                }
            }
        }
    }
}

fn urn(m: &mut Mesh3D, base: Vec3, height: f32, style: u8, yaw: f32) {
    let body = match style % 3 {
        0 => RED,
        1 => NAVY,
        _ => tint(GOLD_TRIM, 0.85),
    };
    let band = if style % 3 == 2 { RED } else { GOLD_TRIM };
    let s = height;
    let profile = [
        (0.0, 0.1),
        (0.04, 0.13),
        (0.07, 0.15),
        (0.2, 0.26),
        (0.42, 0.3),
        (0.62, 0.26),
        (0.76, 0.16),
        (0.84, 0.1),
        (0.9, 0.1),
        (0.94, 0.14),
        (0.97, 0.14),
        (0.97, 0.09),
    ]
    .map(|(y, r)| (y * s, r * s * 0.95));
    let colors = move |j: usize| match j {
        0 | 1 | 5 | 8 | 9 => band,
        10 => [0.06, 0.03, 0.02],
        _ => body,
    };
    lathe(m, base, Vec3::Y, &profile, 12, &colors, true);
    for k in 0..2 {
        let a = yaw + k as f32 * PI;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let t = Vec3::new(-d.z, 0.0, d.x);
        let steps = 7;
        let point = |i: usize| {
            let u = i as f32 / steps as f32;
            let y = s * (0.86 - u * 0.24);
            let r = s * 0.95 * (0.1 + u * 0.16) + (u * PI).sin() * s * 0.09;
            base + Vec3::Y * y + d * r
        };
        for i in 0..steps {
            bar(
                m,
                point(i),
                point(i + 1),
                0.016 * s.max(0.6),
                0.012 * s.max(0.6),
                t,
                band,
            );
        }
    }
    // Gold lozenges around the belly.
    for k in 0..6 {
        let a = yaw + k as f32 * TAU / 6.0;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let t = Vec3::new(-d.z, 0.0, d.x);
        let c = base + Vec3::Y * (0.42 * s) + d * (0.3 * s * 0.95 + 0.004);
        let h = 0.07 * s;
        quad(
            m,
            [
                c - Vec3::Y * h,
                c + t * (h * 0.6),
                c + Vec3::Y * h,
                c - t * (h * 0.6),
            ],
            d,
            band,
        );
    }
}

fn candelabrum(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut Rng) {
    let gold = GOLD_TRIM;
    for k in 0..3 {
        let a = yaw + k as f32 * TAU / 3.0;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        bar(
            m,
            base + Vec3::Y * 0.09,
            base + d * 0.24 + Vec3::Y * 0.015,
            0.02,
            0.015,
            Vec3::Y,
            tint(gold, 0.85),
        );
    }
    lathe(
        m,
        base,
        Vec3::Y,
        &[(0.0, 0.0), (0.04, 0.12), (0.08, 0.1), (0.1, 0.04)],
        10,
        &|_| gold,
        false,
    );
    lathe(
        m,
        base + Vec3::Y * 0.1,
        Vec3::Y,
        &[
            (0.0, 0.03),
            (0.2, 0.024),
            (0.24, 0.05),
            (0.28, 0.024),
            (0.78, 0.02),
            (0.82, 0.05),
            (0.86, 0.026),
            (1.06, 0.026),
        ],
        8,
        &|j| if j % 2 == 1 { tint(gold, 1.08) } else { gold },
        false,
    );
    let cup = |m: &mut Mesh3D, c: Vec3| {
        lathe(
            m,
            c,
            Vec3::Y,
            &[(0.0, 0.02), (0.03, 0.055), (0.045, 0.06)],
            8,
            &|_| gold,
            true,
        );
    };
    let top = base + Vec3::Y * 1.16;
    cup(m, top);
    candle(
        m,
        glow,
        top + Vec3::Y * 0.045,
        0.036,
        rng.range(0.18, 0.24),
        rng,
    );
    for k in 0..4 {
        let a = yaw + FRAC_PI_2 * 0.5 + k as f32 * FRAC_PI_2;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        let root = base + Vec3::Y * 0.96;
        let elbow = root + d * 0.3;
        bar(m, root, elbow, 0.014, 0.014, Vec3::Y, gold);
        bar(m, elbow, elbow + Vec3::Y * 0.1, 0.014, 0.014, d, gold);
        cup(m, elbow + Vec3::Y * 0.1);
        candle(
            m,
            glow,
            elbow + Vec3::Y * 0.145,
            0.032,
            rng.range(0.13, 0.19),
            rng,
        );
    }
}

fn chandelier(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    ceiling: Vec3,
    ring_y: f32,
    radius: f32,
    rng: &mut Rng,
) {
    let hub_y = ring_y + 0.72;
    let x0 = Vec3::new(ceiling.x, 0.0, ceiling.z);
    let at = |y: f32| x0 + Vec3::Y * y;
    // Chain of alternating links from the vault down to the hub.
    let mut y = ceiling.y + 0.25;
    let mut alt = false;
    while y > hub_y + 0.12 {
        let flat = if alt { Vec3::X } else { Vec3::Z };
        bar(m, at(y), at(y - 0.1), 0.028, 0.01, flat, IRON);
        y -= 0.085;
        alt = !alt;
    }
    lathe(
        m,
        at(hub_y - 0.1),
        Vec3::Y,
        &[
            (0.0, 0.0),
            (0.05, 0.09),
            (0.12, 0.11),
            (0.18, 0.06),
            (0.24, 0.02),
        ],
        10,
        &|_| GOLD_TRIM,
        false,
    );
    lathe(
        m,
        at(hub_y - 0.1),
        -Vec3::Y,
        &[(0.0, 0.05), (0.18, 0.03), (0.36, 0.0)],
        8,
        &|_| GOLD_TRIM,
        false,
    );
    for k in 0..4 {
        let a = k as f32 * FRAC_PI_2 + 0.39;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        bar(
            m,
            at(hub_y + 0.06),
            at(ring_y + 0.05) + d * (radius - 0.02),
            0.01,
            0.01,
            Vec3::Y,
            IRON,
        );
    }
    let c = at(ring_y);
    tube_ring(m, c, Vec3::Y, radius, 0.06, 32, tint(DEEP_RED, 1.3));
    tube_ring(
        m,
        c + Vec3::Y * 0.072,
        Vec3::Y,
        radius,
        0.026,
        32,
        GOLD_TRIM,
    );
    tube_ring(
        m,
        c - Vec3::Y * 0.072,
        Vec3::Y,
        radius,
        0.026,
        32,
        GOLD_TRIM,
    );
    for k in 0..16 {
        let a = k as f32 * TAU / 16.0;
        let d = Vec3::new(a.cos(), 0.0, a.sin());
        lathe(
            m,
            c + d * (radius + 0.06) - Vec3::Y * 0.1,
            -Vec3::Y,
            &[(0.0, 0.018), (0.04, 0.0)],
            5,
            &|_| GOLD_TRIM,
            false,
        );
    }
    for k in 0..8 {
        let a = k as f32 * TAU / 8.0;
        let p = c + Vec3::new(a.cos(), 0.0, a.sin()) * radius + Vec3::Y * 0.1;
        lathe(
            m,
            p - Vec3::Y * 0.01,
            Vec3::Y,
            &[(0.0, 0.025), (0.025, 0.05), (0.035, 0.055)],
            8,
            &|_| GOLD_TRIM,
            true,
        );
        candle(m, glow, p + Vec3::Y * 0.03, 0.03, rng.range(0.14, 0.2), rng);
    }
}

#[allow(clippy::too_many_arguments)]
fn banner(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    top: Vec3,
    inward: Vec3,
    tangent: Vec3,
    width: f32,
    length: f32,
    sun: bool,
) {
    let off = inward * 0.09;
    let rod = top + off + inward * 0.02;
    bar(
        m,
        rod - tangent * (width * 0.5 + 0.1),
        rod + tangent * (width * 0.5 + 0.1),
        0.018,
        0.018,
        Vec3::Y,
        GOLD_TRIM,
    );
    for s in [-1.0f32, 1.0] {
        lathe(
            m,
            rod + tangent * (s * (width * 0.5 + 0.1)),
            tangent * s,
            &[(0.0, 0.03), (0.03, 0.04), (0.07, 0.0)],
            6,
            &|_| GOLD_TRIM,
            false,
        );
        // Mounting brackets into the wall.
        bar(
            m,
            rod + tangent * (s * width * 0.42),
            top + tangent * (s * width * 0.42) - inward * 0.05,
            0.012,
            0.012,
            Vec3::Y,
            IRON,
        );
    }
    let hw = width * 0.5;
    let p = |x: f32, y: f32| top + off + tangent * x - Vec3::Y * y;
    let bottom = length;
    let tip = length + 0.36;
    // Cloth hangs in soft vertical folds: alternate the strip depth and shade.
    let strips = 6;
    let fold = |k: usize| if k % 2 == 0 { 0.0 } else { 0.022 };
    for k in 0..strips {
        let x0 = -hw + width * k as f32 / strips as f32;
        let x1 = -hw + width * (k + 1) as f32 / strips as f32;
        let a = |y: f32| p(x0, y) + inward * fold(k);
        let b = |y: f32| p(x1, y) + inward * fold(k + 1);
        let n = (b(0.5) - a(0.5)).cross(-Vec3::Y).normalize_or_zero();
        let n = if n.dot(inward) < 0.0 { -n } else { n };
        let shade = if k % 2 == 0 { 0.86 } else { 1.04 };
        quad(
            m,
            [a(0.03), b(0.03), b(bottom), a(bottom)],
            n,
            tint(RED, shade),
        );
        quad(
            m,
            [a(0.03), b(0.03), b(bottom), a(bottom)],
            -n,
            tint(RED, 0.62),
        );
    }
    for (normal, color) in [(inward, RED), (-inward, tint(RED, 0.62))] {
        tri(m, p(-hw, bottom), p(hw, bottom), p(0.0, tip), normal, color);
    }
    let front = |x: f32, y: f32| p(x, y) + inward * 0.026;
    let front2 = |x: f32, y: f32| p(x, y) + inward * 0.029;
    // Gold border: sides, top and along the swallow-tail edges.
    for s in [-1.0f32, 1.0] {
        let x = s * (hw - 0.07);
        quad(
            m,
            [
                front(x - 0.022, 0.1),
                front(x + 0.022, 0.1),
                front(x + 0.022, bottom - 0.02),
                front(x - 0.022, bottom - 0.02),
            ],
            inward,
            GOLD_TRIM,
        );
        let a = front(s * (hw - 0.07), bottom - 0.02);
        let b = front(0.0, tip - 0.1);
        bar(
            m,
            a,
            b,
            0.02,
            0.002,
            inward.cross((b - a).normalize_or_zero()),
            GOLD_TRIM,
        );
    }
    quad(
        m,
        [
            front(-hw + 0.05, 0.1),
            front(hw - 0.05, 0.1),
            front(hw - 0.05, 0.14),
            front(-hw + 0.05, 0.14),
        ],
        inward,
        GOLD_TRIM,
    );
    // Emblem: ringed eight-point star, the sigil of the treasury.
    let e = 0.72_f32.min(length * 0.42);
    let ec = front(0.0, e);
    annulus(m, ec, inward, 0.2, 0.235, 28, GOLD_TRIM, 0.0);
    if sun {
        // Sanctuary sigil: rayed sun around an almond eye.
        star(
            m,
            front2(0.0, e),
            inward,
            Vec3::Y,
            0.19,
            0.11,
            16,
            GOLD_TRIM,
        );
        disc(
            m,
            front2(0.0, e) + inward * 0.003,
            inward,
            0.1,
            16,
            DEEP_RED,
        );
        let side = Vec3::Y.cross(inward).normalize_or_zero();
        let c = front2(0.0, e) + inward * 0.006;
        for k in 0..12 {
            let a0 = k as f32 * TAU / 12.0;
            let a1 = (k + 1) as f32 * TAU / 12.0;
            let p = |a: f32| c + side * (a.cos() * 0.075) + Vec3::Y * (a.sin() * 0.038);
            super::visual::tri(m, c, p(a0), p(a1), inward, GOLD_TRIM);
        }
        disc(m, c + inward * 0.003, inward, 0.02, 8, DEEP_RED);
    } else {
        star(
            m,
            front2(0.0, e),
            inward,
            Vec3::Y,
            0.19,
            0.075,
            8,
            GOLD_TRIM,
        );
        disc(
            m,
            front2(0.0, e) + inward * 0.003,
            inward,
            0.045,
            10,
            DEEP_RED,
        );
    }
    annulus(
        glow,
        ec + inward * 0.004,
        inward,
        0.215,
        0.222,
        28,
        [0.25, 0.14, 0.03],
        0.0,
    );
    // Pendant rays under the emblem.
    for x in [-0.09f32, 0.0, 0.09] {
        let y0 = e + 0.3;
        let y1 = (bottom - 0.1).max(y0 + 0.2) + if x == 0.0 { 0.22 } else { 0.0 };
        quad(
            m,
            [
                front(x - 0.012, y0),
                front(x + 0.012, y0),
                front(x + 0.012, y1),
                front(x - 0.012, y1),
            ],
            inward,
            tint(GOLD_TRIM, 0.95),
        );
    }
    lathe(
        m,
        p(0.0, tip) + inward * 0.002,
        -Vec3::Y,
        &[(0.0, 0.02), (0.04, 0.035), (0.12, 0.0)],
        6,
        &|_| GOLD_TRIM,
        false,
    );
    for s in [-1.0f32, 1.0] {
        let a = p(s * hw, bottom);
        let b = p(0.0, tip);
        let strands = 9;
        for k in 0..=strands {
            let q = a.lerp(b, k as f32 / strands as f32) + inward * 0.01;
            bar(
                m,
                q,
                q - Vec3::Y * 0.07,
                0.008,
                0.006,
                tangent,
                tint(GOLD_TRIM, 0.95),
            );
        }
        lathe(
            m,
            a + inward * 0.01,
            -Vec3::Y,
            &[(0.0, 0.015), (0.03, 0.028), (0.1, 0.0)],
            6,
            &|_| GOLD_TRIM,
            false,
        );
    }
}

fn coin_scatter(
    m: &mut Mesh3D,
    glow: &mut Mesh3D,
    center: Vec3,
    radius: f32,
    count: u16,
    rng: &mut Rng,
) {
    // A couple of loose gems glint among the coins.
    for _ in 0..1 + count / 12 {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.unit().sqrt();
        let gem = GEMS[rng.index(GEMS.len())];
        crystal(
            glow,
            center + Vec3::new(a.cos() * r, 0.01, a.sin() * r),
            0.03,
            0.06,
            tint(gem, 0.5),
            5,
        );
    }
    // Short coin towers among the loose coins.
    for s in 0..count / 8 {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.range(0.2, 1.0);
        let base = center + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
        for l in 0..2 + rng.index(6) {
            let jitter = Vec3::new(rng.range(-0.006, 0.006), 0.0, rng.range(-0.006, 0.006));
            coin(
                m,
                base + jitter + Vec3::Y * (0.009 + l as f32 * 0.017),
                Vec3::Y,
                0.058,
                0.016,
                coin_color(rng),
                (s as usize + l) as f32,
            );
        }
    }
    for k in 0..count {
        let a = rng.range(0.0, TAU);
        let r = radius * rng.unit().sqrt();
        let n = (Vec3::Y + Vec3::new(rng.range(-0.25, 0.25), 0.0, rng.range(-0.25, 0.25)))
            .normalize_or_zero();
        coin(
            m,
            center + Vec3::new(a.cos() * r, 0.012, a.sin() * r),
            n,
            rng.range(0.05, 0.07),
            0.015,
            coin_color(rng),
            k as f32,
        );
    }
}

fn chest_hoard(m: &mut Mesh3D, glow: &mut Mesh3D, base: Vec3, yaw: f32, rng: &mut Rng) {
    // KayKit chest drawn at 0.62 m: body rim at ~0.29 m, inner half-size
    // ~0.34 x 0.25 m. Heap the gold above the rim and spill it out the front.
    let x = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let z = yaw_dir(yaw);
    for k in 0..70 {
        let u = rng.range(-1.0, 1.0);
        let v = rng.range(-1.0, 1.0);
        let fall = 1.0 - (u * u * 0.7 + v * v * 0.7).min(1.0);
        let p = base + x * (u * 0.33) + z * (v * 0.24) + Vec3::Y * (0.27 + fall * 0.13);
        let n = (Vec3::Y + x * (u * 0.4) + z * (v * 0.4)).normalize_or_zero();
        coin(
            m,
            p,
            n,
            rng.range(0.045, 0.06),
            0.014,
            coin_color(rng),
            k as f32,
        );
    }
    for k in 0..5 {
        let p = base + x * rng.range(-0.25, 0.25) + z * rng.range(-0.15, 0.18) + Vec3::Y * 0.36;
        crystal(
            glow,
            p,
            rng.range(0.03, 0.045),
            rng.range(0.07, 0.11),
            tint(GEMS[k % GEMS.len()], 0.55),
            5,
        );
    }
    // Spill down the front and across the floor.
    for k in 0..26 {
        let t = rng.unit();
        let p = base
            + z * (0.33 + t * 0.55)
            + x * rng.range(-0.3, 0.3)
            + Vec3::Y * (0.012 + (1.0 - t).powi(3) * 0.2);
        let n = (Vec3::Y + z * (0.6 * (1.0 - t))).normalize_or_zero();
        coin(
            m,
            p,
            n,
            rng.range(0.045, 0.065),
            0.015,
            coin_color(rng),
            k as f32,
        );
    }
}

/// Build the static detail and glow meshes for a treasury plan.
pub fn build(
    vault: &Vault,
    stone: [f32; 3],
    glow_color: [f32; 3],
    seed: u64,
    room_id: usize,
) -> Meshes {
    let mut m = empty();
    let mut glow = empty();
    let mut rng = Rng::stream(seed, "treasury_art_v1", room_id as u64);
    for piece in &vault.pieces {
        match *piece {
            Piece::Carpet {
                from,
                to,
                half_width,
            } => carpet(&mut m, &mut glow, from, to, half_width),
            Piece::Rug { center, radius } => rug(&mut m, &mut glow, center, radius),
            Piece::Urn {
                base,
                height,
                style,
                yaw,
            } => urn(&mut m, base, height, style, yaw),
            Piece::Candles {
                base,
                count,
                spread,
            } => {
                let mut placed: Vec<Vec3> = Vec::new();
                for _ in 0..count as usize * 4 {
                    if placed.len() >= count as usize {
                        break;
                    }
                    let a = rng.range(0.0, TAU);
                    let r = spread * rng.unit().sqrt();
                    let p = base + Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                    if placed.iter().all(|q| horizontal(*q - p).length() > 0.11) {
                        placed.push(p);
                        let radius = rng.range(0.03, 0.058);
                        let height = rng.range(0.1, 0.42);
                        candle(&mut m, &mut glow, p, radius, height, &mut rng);
                    }
                }
            }
            Piece::Candelabrum { base, yaw } => candelabrum(&mut m, &mut glow, base, yaw, &mut rng),
            Piece::Chandelier {
                ceiling,
                ring_y,
                radius,
            } => chandelier(&mut m, &mut glow, ceiling, ring_y, radius, &mut rng),
            Piece::Banner {
                top,
                inward,
                tangent,
                width,
                length,
                sun,
            } => banner(&mut m, &mut glow, top, inward, tangent, width, length, sun),
            Piece::Shrine {
                wall,
                inward,
                width,
            } => {
                super::sanctuary::shrine(&mut m, &mut glow, wall, inward, width, stone, glow_color)
            }
            Piece::CandleRow {
                from,
                to,
                count,
                gap,
            } => super::sanctuary::candle_row(&mut m, &mut glow, from, to, count, gap, &mut rng),
            Piece::CrystalBrazier { base, scale } => super::sanctuary::crystal_brazier(
                &mut m, &mut glow, base, scale, stone, glow_color, &mut rng,
            ),
            Piece::Fern { base, scale } => super::sanctuary::fern(&mut m, base, scale, &mut rng),
            Piece::Pot { base, height } => super::sanctuary::clay_pot(&mut m, base, height),
            Piece::SunRelief {
                center,
                normal,
                radius,
            } => super::sanctuary::sun_relief(
                &mut m, &mut glow, center, normal, radius, stone, glow_color,
            ),
            Piece::PrayerRug {
                center,
                along,
                half,
            } => super::sanctuary::prayer_rug(&mut m, &mut glow, center, along, half),
            Piece::CoinScatter {
                center,
                radius,
                count,
            } => coin_scatter(&mut m, &mut glow, center, radius, count, &mut rng),
            Piece::ChestHoard { base, yaw } => chest_hoard(&mut m, &mut glow, base, yaw, &mut rng),
        }
    }
    for (center, radius, height) in &vault.hoards {
        hoard(&mut m, &mut glow, *center, *radius, *height, &mut rng);
    }
    for pedestal in &vault.pedestals {
        altar(
            &mut m,
            &mut glow,
            pedestal.base,
            pedestal.yaw,
            pedestal.relic,
            stone,
            &mut rng,
        );
    }
    let _ = PI;
    ground(&mut m, vault.focus.y);
    Meshes { detail: m, glow }
}
