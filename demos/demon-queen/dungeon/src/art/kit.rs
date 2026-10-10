//! Procedural mesh kit for the cave art, a line-for-line port of the web
//! prototype's `art/kit.js`: flat-shaded, vertex-coloured triangles wound to
//! face a hint normal, smooth lathes, tori and grids, and the same sfc32 art
//! RNG so every seed dresses a room exactly as the Three.js caves do.
#![allow(clippy::too_many_arguments)]
use velvet_math::Vec3;
use velvet_render::{Mesh3D, MeshVertex3D};

pub use super::super::visual::{empty, quad, tint, tri};

pub const TAU: f32 = std::f32::consts::TAU;
pub const VX: Vec3 = Vec3::X;
pub const VY: Vec3 = Vec3::Y;
pub const VZ: Vec3 = Vec3::Z;
pub type C3 = [f32; 3];

#[allow(non_snake_case)]
pub fn Y(h: f32) -> Vec3 {
    Vec3::new(0.0, h, 0.0)
}
pub fn v(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}
pub fn yaw_dir(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}
pub fn yaw_of(d: Vec3) -> f32 {
    d.x.atan2(d.z)
}
pub fn a3(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}
pub fn hz(p: Vec3) -> Vec3 {
    Vec3::new(p.x, 0.0, p.z)
}
pub fn with_y(p: Vec3, y: f32) -> Vec3 {
    Vec3::new(p.x, y, p.z)
}
/// `V.norm()`: zero stays zero.
pub fn norm(p: Vec3) -> Vec3 {
    let l = p.length();
    if l > 1e-12 {
        p * (1.0 / l)
    } else {
        Vec3::ZERO
    }
}
pub fn hdist(a: Vec3, b: Vec3) -> f32 {
    (a.x - b.x).hypot(a.z - b.z)
}
pub fn mix_color(a: C3, b: C3, t: f32) -> C3 {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}
pub fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

// ---------------------------------------------------------------------- RNG

fn fnv32(text: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for unit in text.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn mix32(value: u32) -> u32 {
    let mut x = value;
    x = (x ^ (x >> 16)).wrapping_mul(0x7feb352d);
    x = (x ^ (x >> 15)).wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}

/// Fold a 64-bit seed into 32 bits (kit.js `seed32`).
pub fn seed32(seed: u64) -> u32 {
    let low = (seed & 0xffff_ffff) as u32;
    let high = ((seed >> 32) & 0xffff_ffff) as u32;
    mix32(low ^ mix32(high.wrapping_add(0x9e3779b9)))
}

/// A prototype seed: a map seed (a BigInt in the web version) or a 32-bit
/// number (`wrapSeed`, `nextSeed`), which `seed32` folds differently.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seed {
    Big(u64),
    Num(u32),
}

impl Seed {
    pub fn s32(self) -> u32 {
        match self {
            Seed::Big(v) => seed32(v),
            Seed::Num(n) => mix32(n),
        }
    }
}

/// kit.js `wrapSeed`: a seed offset by `add`, as a 32-bit number.
pub fn wrap_seed(seed: Seed, add: u32) -> Seed {
    Seed::Num(seed.s32().wrapping_add(add))
}

/// sfc32, bit-equal to the web prototype's `ArtRng`.
#[derive(Clone, Debug)]
pub struct ArtRng {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

impl ArtRng {
    pub fn new(a: u32, b: u32, c: u32, d: u32) -> Self {
        let mut rng = Self { a, b, c, d };
        for _ in 0..12 {
            rng.next_u32();
        }
        rng
    }
    pub fn stream(seed: u64, domain: &str, index: u64) -> Self {
        let s = seed32(seed);
        let h = fnv32(domain);
        let i = mix32((index as u32).wrapping_mul(0x9e3779b9) ^ 0x85ebca6b);
        Self::new(s ^ h, mix32(s.wrapping_add(i)), h ^ i, mix32(s ^ h ^ i) | 1)
    }
    /// A stream from a prototype seed (map seed or numeric seed).
    pub fn stream_s(seed: Seed, domain: &str, index: u64) -> Self {
        let s = seed.s32();
        let h = fnv32(domain);
        let i = mix32((index as u32).wrapping_mul(0x9e3779b9) ^ 0x85ebca6b);
        Self::new(s ^ h, mix32(s.wrapping_add(i)), h ^ i, mix32(s ^ h ^ i) | 1)
    }
    /// kit.js `nextSeed`.
    pub fn next_seed(&mut self) -> Seed {
        Seed::Num(self.next_u32())
    }
    pub fn next_u32(&mut self) -> u32 {
        let t = self.a.wrapping_add(self.b).wrapping_add(self.d);
        self.d = self.d.wrapping_add(1);
        self.a = self.b ^ (self.b >> 9);
        self.b = self.c.wrapping_add(self.c << 3);
        self.c = (self.c << 21) | (self.c >> 11);
        self.c = self.c.wrapping_add(t);
        t
    }
    pub fn unit64(&mut self) -> f64 {
        self.next_u32() as f64 / 4294967296.0
    }
    pub fn unit(&mut self) -> f32 {
        self.unit64() as f32
    }
    /// `range` in full precision, for polygon math that must tie-break
    /// exactly like the web version.
    pub fn range64(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit64()
    }
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        (low as f64 + (high as f64 - low as f64) * self.unit64()) as f32
    }
    pub fn index(&mut self, count: usize) -> usize {
        (self.unit64() * count.max(1) as f64).floor() as usize
    }
    pub fn chance(&mut self, probability: f64) -> bool {
        self.unit64() < probability
    }
    pub fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = self.index(i + 1);
            values.swap(i, j);
        }
    }
    pub fn shuffled<T: Clone>(&mut self, values: &[T]) -> Vec<T> {
        let mut out = values.to_vec();
        self.shuffle(&mut out);
        out
    }
}

// --------------------------------------------------------------------- mesh

pub fn vertex(m: &mut Mesh3D, p: Vec3, n: Vec3, c: C3) -> u32 {
    m.vertices.push(MeshVertex3D {
        position: p.to_array(),
        normal: n.to_array(),
        uv: [0.0, 0.0],
        color: [c[0], c[1], c[2], 1.0],
    });
    (m.vertices.len() - 1) as u32
}

/// Triangle of existing vertices, wound to face their summed normals.
pub fn push_tri(m: &mut Mesh3D, i0: u32, i1: u32, i2: u32) {
    let p = |i: u32| a3(m.vertices[i as usize].position);
    let n = |i: u32| a3(m.vertices[i as usize].normal);
    let face = (p(i1) - p(i0)).cross(p(i2) - p(i0));
    if face.length_squared() < 1e-14 {
        return;
    }
    if face.dot(n(i0) + n(i1) + n(i2)) < 0.0 {
        m.indices.extend([i0, i2, i1]);
    } else {
        m.indices.extend([i0, i1, i2]);
    }
}

pub fn cuboid(m: &mut Mesh3D, base: Vec3, half: Vec3, color: C3) {
    super::super::visual::cuboid(m, base, half, color);
}

const ARCH_SHADES: [f32; 6] = [1.10, 0.92, 1.0, 1.0, 1.18, 0.7];
const KIT_SHADES: [f32; 6] = [1.05, 0.9, 0.97, 0.97, 1.15, 0.7];

fn shaded_box(
    m: &mut Mesh3D,
    c: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
    h: Vec3,
    color: C3,
    shades: [f32; 6],
) {
    let p = |sx: f32, sy: f32, sz: f32| c + x * (sx * h.x) + y * (sy * h.y) + z * (sz * h.z);
    let faces = [
        (
            z,
            [
                p(-1.0, -1.0, 1.0),
                p(1.0, -1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(-1.0, 1.0, 1.0),
            ],
        ),
        (
            -z,
            [
                p(1.0, -1.0, -1.0),
                p(-1.0, -1.0, -1.0),
                p(-1.0, 1.0, -1.0),
                p(1.0, 1.0, -1.0),
            ],
        ),
        (
            x,
            [
                p(1.0, -1.0, 1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, 1.0, -1.0),
                p(1.0, 1.0, 1.0),
            ],
        ),
        (
            -x,
            [
                p(-1.0, -1.0, -1.0),
                p(-1.0, -1.0, 1.0),
                p(-1.0, 1.0, 1.0),
                p(-1.0, 1.0, -1.0),
            ],
        ),
        (
            y,
            [
                p(-1.0, 1.0, 1.0),
                p(1.0, 1.0, 1.0),
                p(1.0, 1.0, -1.0),
                p(-1.0, 1.0, -1.0),
            ],
        ),
        (
            -y,
            [
                p(-1.0, -1.0, -1.0),
                p(1.0, -1.0, -1.0),
                p(1.0, -1.0, 1.0),
                p(-1.0, -1.0, 1.0),
            ],
        ),
    ];
    for (index, (normal, corners)) in faces.into_iter().enumerate() {
        quad(m, corners, normal, tint(color, shades[index]));
    }
}

/// Masonry box with orthonormal axes and half extents (architecture.rs).
pub fn block(m: &mut Mesh3D, c: Vec3, x: Vec3, y: Vec3, z: Vec3, h: Vec3, color: C3) {
    shaded_box(m, c, x, y, z, h, color, ARCH_SHADES);
}
/// Art-kit box with the treasury's softer face shading.
pub fn kit_block(m: &mut Mesh3D, c: Vec3, x: Vec3, y: Vec3, z: Vec3, h: Vec3, color: C3) {
    shaded_box(m, c, x, y, z, h, color, KIT_SHADES);
}

pub fn oriented_box(
    m: &mut Mesh3D,
    center: Vec3,
    side: Vec3,
    forward: Vec3,
    sx: f32,
    sy: f32,
    sz: f32,
    color: C3,
) {
    let point = |s: f32, y: f32, d: f32| center + side * s + Y(y) + forward * d;
    quad(
        m,
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
        m,
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
        m,
        [
            point(-sx, sy, sz),
            point(sx, sy, sz),
            point(sx, sy, -sz),
            point(-sx, sy, -sz),
        ],
        VY,
        tint(color, 1.24),
    );
    quad(
        m,
        [
            point(-sx, -sy, -sz),
            point(sx, -sy, -sz),
            point(sx, -sy, sz),
            point(-sx, -sy, sz),
        ],
        -VY,
        tint(color, 0.52),
    );
    quad(
        m,
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
        m,
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

pub fn basis(n: Vec3) -> (Vec3, Vec3) {
    let helper = if n.y.abs() < 0.9 { VY } else { VX };
    let u = norm(helper.cross(n));
    let v = norm(n.cross(u));
    (u, v)
}

/// Rectangular bar between two points; `flat` orients its wide side.
pub fn bar(m: &mut Mesh3D, a: Vec3, b: Vec3, half_w: f32, half_t: f32, flat: Vec3, color: C3) {
    let axis = norm(b - a);
    let mut side = flat - axis * flat.dot(axis);
    if side.length_squared() < 1e-6 {
        side = basis(axis).0;
    }
    side = norm(side);
    let thick = norm(axis.cross(side));
    kit_block(
        m,
        (a + b) * 0.5,
        side,
        axis,
        thick,
        v(half_w, (b - a).length() * 0.5, half_t),
        color,
    );
}

pub fn disc(m: &mut Mesh3D, c: Vec3, n: Vec3, r: f32, sides: usize, color: C3) {
    let (u, w) = basis(n);
    for index in 0..sides {
        let a = index as f32 * TAU / sides as f32;
        let b = (index + 1) as f32 * TAU / sides as f32;
        tri(
            m,
            c,
            c + (u * a.cos() + w * a.sin()) * r,
            c + (u * b.cos() + w * b.sin()) * r,
            n,
            color,
        );
    }
}

pub fn annulus(
    m: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    r0: f32,
    r1: f32,
    sides: usize,
    color: C3,
    phase: f32,
) {
    let (u, w) = basis(n);
    let p = |angle: f32, r: f32| c + (u * angle.cos() + w * angle.sin()) * r;
    for index in 0..sides {
        let a = index as f32 * TAU / sides as f32 + phase;
        let b = (index + 1) as f32 * TAU / sides as f32 + phase;
        quad(m, [p(a, r0), p(b, r0), p(b, r1), p(a, r1)], n, color);
    }
}

pub fn star(
    m: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    up: Vec3,
    r_out: f32,
    r_in: f32,
    points: usize,
    color: C3,
) {
    let u = norm(up.cross(n));
    let w = norm(n.cross(u));
    let count = points * 2;
    for index in 0..count {
        let a = index as f32 * TAU / count as f32;
        let b = (index + 1) as f32 * TAU / count as f32;
        let (ra, rb) = if index % 2 == 0 {
            (r_out, r_in)
        } else {
            (r_in, r_out)
        };
        tri(
            m,
            c,
            c + (w * a.cos() + u * a.sin()) * ra,
            c + (w * b.cos() + u * b.sin()) * rb,
            n,
            tint(color, if index % 2 == 0 { 1.08 } else { 0.9 }),
        );
    }
}

/// Smooth surface of revolution; `profile` lists (height along axis, radius).
pub fn lathe_fn(
    m: &mut Mesh3D,
    base: Vec3,
    axis: Vec3,
    profile: &[(f32, f32)],
    sides: usize,
    color: &dyn Fn(usize) -> C3,
    caps: bool,
) {
    if profile.len() < 2 {
        return;
    }
    let ring_sides = (sides + sides / 2).max(8);
    let (u, w) = basis(axis);
    let mut seg = Vec::with_capacity(profile.len() - 1);
    for index in 0..profile.len() - 1 {
        let (h0, r0) = profile[index];
        let (h1, r1) = profile[index + 1];
        let (dh, dr) = (h1 - h0, r1 - r0);
        let length = dh.hypot(dr).max(1e-5);
        seg.push((dh / length, -dr / length));
    }
    let blend = |a: (f32, f32), b: (f32, f32)| {
        if a.0 * b.0 + a.1 * b.1 > 0.7 {
            let (s0, s1) = (a.0 + b.0, a.1 + b.1);
            let l = s0.hypot(s1).max(1e-5);
            (s0 / l, s1 / l)
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
        for i in 0..=ring_sides {
            let a = i as f32 * TAU / ring_sides as f32;
            let radial = u * a.cos() + w * a.sin();
            let sheen = 0.9 + 0.1 * a.cos();
            for (h, r, nn) in [(h0, r0, n0), (h1, r1, n1)] {
                let normal = norm(radial * nn.0 + axis * nn.1);
                vertex(m, base + axis * h + radial * r, normal, tint(c, sheen));
            }
        }
        for i in 0..ring_sides as u32 {
            let a0 = start + i * 2;
            push_tri(m, a0, a0 + 2, a0 + 3);
            push_tri(m, a0, a0 + 3, a0 + 1);
        }
    }
    if caps {
        let (h0, r0) = profile[0];
        let (h1, r1) = profile[profile.len() - 1];
        if r0 > 1e-3 {
            disc(
                m,
                base + axis * h0,
                -axis,
                r0,
                ring_sides,
                tint(color(0), 0.7),
            );
        }
        if r1 > 1e-3 {
            disc(
                m,
                base + axis * h1,
                axis,
                r1,
                ring_sides,
                color(profile.len().saturating_sub(2)),
            );
        }
    }
}
pub fn lathe(
    m: &mut Mesh3D,
    base: Vec3,
    axis: Vec3,
    profile: &[(f32, f32)],
    sides: usize,
    color: C3,
    caps: bool,
) {
    lathe_fn(m, base, axis, profile, sides, &|_| color, caps);
}

/// Smooth torus of `radius` around `n` with a round tube.
pub fn tube_ring(
    m: &mut Mesh3D,
    c: Vec3,
    n: Vec3,
    radius: f32,
    tube: f32,
    segments: usize,
    color: C3,
) {
    let (u, w) = basis(n);
    let count = (segments + segments / 2).max(12);
    let sides = 8usize;
    let start = m.vertices.len() as u32;
    for i in 0..=count {
        let a = i as f32 * TAU / count as f32;
        let dir = u * a.cos() + w * a.sin();
        for j in 0..=sides {
            let phi = j as f32 * TAU / sides as f32;
            let normal = norm(dir * phi.cos() + n * phi.sin());
            let shade = 0.82 + 0.26 * (0.5 + 0.5 * phi.sin());
            vertex(
                m,
                c + dir * (radius + tube * phi.cos()) + n * (tube * phi.sin()),
                normal,
                tint(color, shade),
            );
        }
    }
    let row = (sides + 1) as u32;
    for i in 0..count as u32 {
        for j in 0..sides as u32 {
            let a = start + i * row + j;
            let b = start + (i + 1) * row + j;
            push_tri(m, a, b, b + 1);
            push_tri(m, a, b + 1, a + 1);
        }
    }
}

/// Smooth surface through a grid of points `at(i, j)` (rows 0..=rows,
/// columns 0..=cols); normals from central differences, turned toward
/// `outward`; `closed` wraps the columns; `inner` also emits the back side.
pub fn grid_surface(
    m: &mut Mesh3D,
    rows: usize,
    cols: usize,
    at: &dyn Fn(usize, usize) -> Vec3,
    color: &dyn Fn(usize, usize) -> C3,
    closed: bool,
    outward: Option<&dyn Fn(Vec3, usize, usize) -> Vec3>,
    inner: Option<&dyn Fn(usize, usize) -> C3>,
) {
    let grid: Vec<Vec<Vec3>> = (0..=rows)
        .map(|i| (0..=cols).map(|j| at(i, j)).collect())
        .collect();
    let get = |i: isize, j: isize| {
        let r = i.clamp(0, rows as isize) as usize;
        let c = if closed {
            (j.rem_euclid(cols as isize)) as usize
        } else {
            j.clamp(0, cols as isize) as usize
        };
        grid[r][c]
    };
    let mut normals = vec![vec![Vec3::ZERO; cols + 1]; rows + 1];
    for i in 0..=rows {
        for j in 0..=cols {
            let (ii, jj) = (i as isize, j as isize);
            let p = grid[i][j];
            let mut n =
                (get(ii, jj + 1) - get(ii, jj - 1)).cross(get(ii + 1, jj) - get(ii - 1, jj));
            if n.length_squared() < 1e-14 {
                n = (get(ii, jj + 1) - get(ii, jj - 1)).cross(get(ii + 2, jj) - get(ii - 2, jj));
            }
            if n.length_squared() < 1e-14 {
                n = outward.map(|f| f(p, i, j)).unwrap_or(VY);
            }
            n = norm(n);
            if let Some(f) = outward {
                if n.dot(f(p, i, j)) < 0.0 {
                    n = -n;
                }
            }
            normals[i][j] = n;
        }
    }
    let sides: &[f32] = if inner.is_some() {
        &[1.0, -1.0]
    } else {
        &[1.0]
    };
    for &side in sides {
        let start = m.vertices.len() as u32;
        for i in 0..=rows {
            for j in 0..=cols {
                let n = if side > 0.0 {
                    normals[i][j]
                } else {
                    -normals[i][j]
                };
                let c = if side > 0.0 {
                    color(i, j)
                } else {
                    inner.unwrap()(i, j)
                };
                vertex(m, grid[i][j], n, c);
            }
        }
        for i in 0..rows as u32 {
            for j in 0..cols as u32 {
                let a = start + i * (cols as u32 + 1) + j;
                let b = a + cols as u32 + 1;
                push_tri(m, a, b, b + 1);
                push_tri(m, a, b + 1, a + 1);
            }
        }
    }
}

/// Faceted crystal / stalactite (negative height hangs downward).
pub fn crystal(m: &mut Mesh3D, base: Vec3, radius: f32, height: f32, color: C3, sides: usize) {
    if sides < 3 || radius <= 0.0 || height.abs() < 0.001 {
        return;
    }
    let phase = (radius * 17.0 + height * 3.0).sin() * 0.12;
    let ring = |index: usize, level: u8| {
        let side = index % sides;
        let angle = side as f32 * TAU / sides as f32 + phase;
        let wobble = 0.94 + 0.06 * (side as f32 * 5.7 + radius * 13.0).sin();
        let (y, width) = match level {
            0 => (0.0, 0.78),
            1 => (height * 0.2, 1.0),
            _ => (height * 0.7, 0.72),
        };
        base + v(
            angle.cos() * radius * width * wobble,
            y,
            angle.sin() * radius * width * wobble,
        )
    };
    let tip = base + v(radius * 0.1, height, radius * 0.07);
    let base_normal = if height > 0.0 { -VY } else { VY };
    for i in 0..sides {
        let (p0, q0) = (ring(i, 0), ring(i + 1, 0));
        let (p1, q1) = (ring(i, 1), ring(i + 1, 1));
        let (p2, q2) = (ring(i, 2), ring(i + 1, 2));
        let radial = norm(v(
            (p0.x + q0.x) * 0.5 - base.x,
            0.0,
            (p0.z + q0.z) * 0.5 - base.z,
        ));
        let shade = tint(color, 0.82 + (i % 3) as f32 * 0.1);
        quad(m, [p0, q0, q1, p1], radial, tint(shade, 0.92));
        quad(m, [p1, q1, q2, p2], radial, shade);
        tri(
            m,
            p2,
            q2,
            tip,
            norm(radial + Y(height.signum() * 0.62)),
            tint(shade, 1.08),
        );
        tri(m, base, q0, p0, base_normal, tint(color, 0.72));
    }
}

/// Seven-sided boulder whose `base` is its bottom centre.
pub fn rock(m: &mut Mesh3D, base: Vec3, scale: Vec3, color: C3, phase: f32) {
    const RINGS: [(f32, f32); 4] = [(0.0, 0.7), (0.3, 1.0), (0.72, 0.85), (1.0, 0.2)];
    let vtx = |i: usize, j: usize| {
        let side = i % 7;
        let a = side as f32 * TAU / 7.0 + phase;
        let wobble = 0.88 + 0.12 * (side as f32 * 7.13 + j as f32 * 1.73).sin();
        base + v(
            a.cos() * RINGS[j].1 * scale.x * wobble,
            RINGS[j].0 * scale.y,
            a.sin() * RINGS[j].1 * scale.z * wobble,
        )
    };
    for j in 0..RINGS.len() - 1 {
        for i in 0..7 {
            let p = [vtx(i, j), vtx(i + 1, j), vtx(i + 1, j + 1), vtx(i, j + 1)];
            let normal = norm((p[0] + p[1] + p[2] + p[3]) * 0.25 - base);
            quad(m, p, normal, tint(color, 0.94 + (i % 3) as f32 * 0.035));
        }
    }
    let top = base + Y(scale.y);
    for i in 0..7 {
        tri(m, base, vtx(i + 1, 0), vtx(i, 0), -VY, tint(color, 0.78));
        tri(
            m,
            top,
            vtx(i, RINGS.len() - 1),
            vtx(i + 1, RINGS.len() - 1),
            VY,
            tint(color, 0.96),
        );
    }
}

/// Cheap octagonal coin (top + rim), oriented by its face normal.
pub fn coin(m: &mut Mesh3D, c: Vec3, n: Vec3, radius: f32, thick: f32, color: C3, phase: f32) {
    let (u, w) = basis(n);
    let top = c + n * (thick * 0.5);
    let bottom = c - n * (thick * 0.5);
    for index in 0..8 {
        let a = index as f32 * TAU / 8.0 + phase;
        let b = (index + 1) as f32 * TAU / 8.0 + phase;
        let ra = u * a.cos() + w * a.sin();
        let rb = u * b.cos() + w * b.sin();
        tri(
            m,
            top,
            top + ra * radius,
            top + rb * radius,
            n,
            tint(color, if index % 2 == 0 { 1.12 } else { 0.98 }),
        );
        quad(
            m,
            [
                bottom + ra * radius,
                bottom + rb * radius,
                top + rb * radius,
                top + ra * radius,
            ],
            norm(ra + rb),
            tint(color, 0.72),
        );
    }
}

/// A point/normal mapper, as the prototype's `{ point, normal }` objects.
pub struct Mapper {
    pub point: Box<dyn Fn(Vec3) -> Vec3>,
    pub normal: Box<dyn Fn(Vec3) -> Vec3>,
}

pub fn identity() -> Mapper {
    Mapper {
        point: Box::new(|p| p),
        normal: Box::new(|n| n),
    }
}

/// Translation · rotationY(yaw) · uniform scale.
pub fn trs_yaw(t: Vec3, yaw: f32, s: f32) -> Mapper {
    let (c, sn) = (yaw.cos(), yaw.sin());
    Mapper {
        point: Box::new(move |p| {
            v(
                t.x + (p.x * c + p.z * sn) * s,
                t.y + p.y * s,
                t.z + (-p.x * sn + p.z * c) * s,
            )
        }),
        normal: Box::new(move |n| v(n.x * c + n.z * sn, n.y, -n.x * sn + n.z * c)),
    }
}

/// Mapper: scale, tilt about local X, roll about local Z, yaw about Y, then translate.
pub fn pose_mapper(t: Vec3, yaw: f32, tilt: f32, roll: f32, s: f32) -> Mapper {
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let (ct, st) = (tilt.cos(), tilt.sin());
    let (cr, sr) = (roll.cos(), roll.sin());
    let rotate = move |p: Vec3| {
        let x = p.x * cr - p.y * sr;
        let y = p.x * sr + p.y * cr;
        let z = p.z;
        let y2 = y * ct - z * st;
        let z2 = y * st + z * ct;
        v(x * cy + z2 * sy, y2, -x * sy + z2 * cy)
    };
    Mapper {
        point: Box::new(move |p| {
            let r = rotate(p);
            v(t.x + r.x * s, t.y + r.y * s, t.z + r.z * s)
        }),
        normal: Box::new(rotate),
    }
}

/// Copy another mesh through a mapper, multiplying its vertex colours.
pub fn append(m: &mut Mesh3D, src: &Mesh3D, mapper: &Mapper, color: C3) {
    let base = m.vertices.len() as u32;
    for vtx in &src.vertices {
        let p = (mapper.point)(a3(vtx.position));
        let n = norm((mapper.normal)(a3(vtx.normal)));
        let c = vtx.color;
        vertex(m, p, n, [c[0] * color[0], c[1] * color[1], c[2] * color[2]]);
    }
    m.indices.extend(src.indices.iter().map(|i| i + base));
}

/// Merge `from` into `into`, scaled by `k` about `origin`.
pub fn merge_scaled(into: &mut Mesh3D, from: &Mesh3D, origin: Vec3, k: f32) {
    append(
        into,
        from,
        &Mapper {
            point: Box::new(move |p| origin + (p - origin) * k),
            normal: Box::new(|n| n),
        },
        [1.0; 3],
    );
}

/// Baked contact occlusion at the feet of free-standing pieces.
pub fn ground_occlusion(m: &mut Mesh3D, floor: f32) {
    for vtx in &mut m.vertices {
        let h = vtx.position[1] - floor;
        if (0.0..0.4).contains(&h) && vtx.normal[1] < 0.6 {
            let k = 0.55 + 0.45 * (h / 0.4);
            vtx.color[0] *= k;
            vtx.color[1] *= k;
            vtx.color[2] *= k;
        }
    }
}

/// Box with chamfered edges and corners, shading baked from the normal.
pub fn chamfer_box(m: &mut Mesh3D, c: Vec3, x: Vec3, y: Vec3, z: Vec3, h: Vec3, b: f32, color: C3) {
    let axes = [x, y, z];
    let hh = [h.x, h.y, h.z];
    let bev = b.min(hh[0] * 0.45).min(hh[1] * 0.45).min(hh[2] * 0.45);
    let local = |l: [f32; 3]| c + x * l[0] + y * l[1] + z * l[2];
    let vtx = |a: usize, sa: f32, b_axis: usize, sb: f32, k: usize, t: f32| {
        let mut l = [0.0; 3];
        l[a] = sa * hh[a];
        l[b_axis] = sb * (hh[b_axis] - bev);
        l[k] = t * (hh[k] - bev);
        local(l)
    };
    let shade = |n: Vec3| tint(color, 0.8 + 0.3 * n.y + 0.06 * n.dot(z).max(0.0));
    for a in 0..3 {
        let (b1, b2) = ((a + 1) % 3, (a + 2) % 3);
        for s in [-1.0, 1.0] {
            let n = axes[a] * s;
            quad(
                m,
                [
                    vtx(a, s, b1, -1.0, b2, -1.0),
                    vtx(a, s, b1, 1.0, b2, -1.0),
                    vtx(a, s, b1, 1.0, b2, 1.0),
                    vtx(a, s, b1, -1.0, b2, 1.0),
                ],
                n,
                shade(n),
            );
        }
    }
    for a in 0..3 {
        let (b2, k) = ((a + 1) % 3, (a + 2) % 3);
        for sa in [-1.0, 1.0] {
            for sb in [-1.0, 1.0] {
                let n = norm(axes[a] * sa + axes[b2] * sb);
                quad(
                    m,
                    [
                        vtx(a, sa, b2, sb, k, -1.0),
                        vtx(a, sa, b2, sb, k, 1.0),
                        vtx(b2, sb, a, sa, k, 1.0),
                        vtx(b2, sb, a, sa, k, -1.0),
                    ],
                    n,
                    shade(n),
                );
            }
        }
    }
    for sx in [-1.0, 1.0] {
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                let n = norm(x * sx + y * sy + z * sz);
                tri(
                    m,
                    vtx(0, sx, 1, sy, 2, sz),
                    vtx(1, sy, 0, sx, 2, sz),
                    vtx(2, sz, 0, sx, 1, sy),
                    n,
                    shade(n),
                );
            }
        }
    }
}

// --------------------------------------------------------------- theme kit

/// A smooth tube through `pts`, radius `radius(t)`, colour `color(t, around)`.
pub fn tube(
    m: &mut Mesh3D,
    pts: &[Vec3],
    radius: &dyn Fn(f32) -> f32,
    sides: usize,
    color: &dyn Fn(f32, f32) -> C3,
) {
    if pts.len() < 2 {
        return;
    }
    let n = pts.len();
    let tangents: Vec<Vec3> = (0..n)
        .map(|i| norm(pts[(i + 1).min(n - 1)] - pts[i.saturating_sub(1)]))
        .collect();
    let mut normal = if tangents[0].y.abs() < 0.9 {
        norm(VY.cross(tangents[0]))
    } else {
        norm(VX.cross(tangents[0]))
    };
    let start = m.vertices.len() as u32;
    for i in 0..n {
        if i > 0 {
            normal = norm(normal - tangents[i] * normal.dot(tangents[i]));
        }
        let binormal = norm(tangents[i].cross(normal));
        let t = i as f32 / (n - 1) as f32;
        let r = radius(t);
        for k in 0..=sides {
            let a = (k as f32 / sides as f32) * TAU;
            let d = normal * a.cos() + binormal * a.sin();
            vertex(m, pts[i] + d * r, d, color(t, k as f32 / sides as f32));
        }
    }
    let row = (sides + 1) as u32;
    for i in 0..(n - 1) as u32 {
        for k in 0..sides as u32 {
            let a = start + i * row + k;
            let b = start + (i + 1) * row + k;
            push_tri(m, a, b, b + 1);
            push_tri(m, a, b + 1, a + 1);
        }
    }
}
pub fn tube_c(m: &mut Mesh3D, pts: &[Vec3], radius: f32, sides: usize, color: C3) {
    tube(m, pts, &|_| radius, sides, &|_, _| color);
}

/// Quadratic Bézier from `a` via `c` to `b`, `steps` + 1 points.
pub fn bezier(a: Vec3, c: Vec3, b: Vec3, steps: usize) -> Vec<Vec3> {
    (0..=steps)
        .map(|k| {
            let t = k as f32 / steps as f32;
            let u = 1.0 - t;
            a * (u * u) + c * (2.0 * u * t) + b * (t * t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_rng_matches_the_web_prototype() {
        // Values printed by the prototype's ArtRng for the same streams.
        let mut rng = ArtRng::stream(731942, "room_theme_v1", 3);
        let first: Vec<u32> = (0..3).map(|_| rng.next_u32()).collect();
        assert_eq!(first, [1210274909, 3437367828, 2683598219]);
        let mut rng = ArtRng::stream(18446744073709551557, "treasury_art_v1", 17);
        assert_eq!([rng.next_u32(), rng.next_u32()], [4124180846, 2723002113]);
    }
}

/// The relic crown, modelled about its base (web `crownMesh`).
pub fn crown_mesh() -> Mesh3D {
    let mut m = empty();
    let sides = 8;
    let (outer, inner, bottom, top) = (0.3, 0.22, 0.025, 0.17);
    let point = |index: usize, radius: f32, y: f32| {
        let angle = index as f32 * TAU / sides as f32;
        v(angle.cos() * radius, y, angle.sin() * radius)
    };
    let gold = [0.96, 0.59, 0.12];
    let shade = [0.7, 0.31, 0.055];
    for side in 0..sides {
        let next = side + 1;
        let outward = norm(v(
            (point(side, outer, 0.0).x + point(next, outer, 0.0).x) * 0.5,
            0.0,
            (point(side, outer, 0.0).z + point(next, outer, 0.0).z) * 0.5,
        ));
        quad(
            &mut m,
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
            &mut m,
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
            &mut m,
            [
                point(side, inner, top),
                point(side, outer, top),
                point(next, outer, top),
                point(next, inner, top),
            ],
            VY,
            tint(gold, 1.08),
        );
        quad(
            &mut m,
            [
                point(side, outer, bottom),
                point(side, inner, bottom),
                point(next, inner, bottom),
                point(next, outer, bottom),
            ],
            -VY,
            shade,
        );
    }
    for tooth in [0usize, 2, 3, 5, 7] {
        let left = point(tooth, outer, top);
        let right = point(tooth + 1, outer, top);
        let angle = (tooth as f32 + 0.5) * TAU / sides as f32;
        let tip = v(angle.cos() * 0.27, 0.43, angle.sin() * 0.27);
        let normal = norm(v(angle.cos(), 0.12, angle.sin()));
        tri(&mut m, left, right, tip, normal, tint(gold, 1.08));
        tri(&mut m, right, left, tip, -normal, tint(gold, 0.78));
    }
    crystal(
        &mut m,
        v(-0.055, 0.075, 0.279),
        0.061,
        0.105,
        [0.16, 0.84, 1.0],
        5,
    );
    m
}

/// The f64 a JS literal written like this f32 would be (0.95f32 → 0.95).
pub fn lit(x: f32) -> f64 {
    format!("{x}").parse().unwrap_or(x as f64)
}
