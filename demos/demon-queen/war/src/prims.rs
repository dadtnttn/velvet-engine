//! Low-poly primitives for the battlefield's dressing (the shapes three.js gave the prototype:
//! boxes, cylinders, cones, icosahedra, dodecahedra, spheres), flat-shaded and vertex-coloured,
//! merged into a few meshes for the toon pass.
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use velvet_math::{Mat4, Vec3};
use velvet_render::{Material3D, Mesh3D, MeshVertex3D};

use crate::noise::Rng;

/// Triangles (each three positions), one colour.
#[derive(Clone, Debug, Default)]
pub struct Shape {
    pub tris: Vec<[f32; 3]>,
}

impl Shape {
    pub fn transform(mut self, m: Mat4) -> Shape {
        for p in &mut self.tris {
            let q = m.transform_point3(Vec3::new(p[0], p[1], p[2]));
            *p = [q.x, q.y, q.z];
        }
        self
    }

    pub fn translate(self, x: f32, y: f32, z: f32) -> Shape {
        self.transform(Mat4::from_translation(Vec3::new(x, y, z)))
    }

    pub fn scale(self, x: f32, y: f32, z: f32) -> Shape {
        self.transform(Mat4::from_scale(Vec3::new(x, y, z)))
    }

    pub fn rotate_x(self, a: f32) -> Shape {
        self.transform(Mat4::from_rotation_x(a))
    }

    pub fn rotate_y(self, a: f32) -> Shape {
        self.transform(Mat4::from_rotation_y(a))
    }

    pub fn rotate_z(self, a: f32) -> Shape {
        self.transform(Mat4::from_rotation_z(a))
    }

    /// Each corner moved a little at random (the same for corners that meet).
    pub fn jagged(mut self, amount: f32, seed: u32) -> Shape {
        let mut r = Rng::new(seed);
        let mut seen: HashMap<[i32; 3], [f32; 3]> = HashMap::new();
        for p in &mut self.tris {
            let key = [
                (p[0] * 1000.0).round() as i32,
                (p[1] * 1000.0).round() as i32,
                (p[2] * 1000.0).round() as i32,
            ];
            let o = *seen.entry(key).or_insert_with(|| {
                [
                    (r.next() - 0.5) * amount,
                    (r.next() - 0.5) * amount * 0.6,
                    (r.next() - 0.5) * amount,
                ]
            });
            p[0] += o[0];
            p[1] += o[1];
            p[2] += o[2];
        }
        self
    }

    /// Corners pushed out or in by a random factor (a crag's lumps).
    pub fn lumpy(mut self, seed: u32, min: f32, spread: f32) -> Shape {
        let mut r = Rng::new(seed);
        let mut seen: HashMap<[i32; 3], f32> = HashMap::new();
        for p in &mut self.tris {
            let key = [
                (p[0] * 1000.0).round() as i32,
                (p[1] * 1000.0).round() as i32,
                (p[2] * 1000.0).round() as i32,
            ];
            let k = *seen.entry(key).or_insert_with(|| min + r.next() * spread);
            let top = if p[1] > 0.4 { 0.85 } else { 1.0 };
            *p = [p[0] * k, p[1] * k * top, p[2] * k];
        }
        self
    }
}

fn tri(out: &mut Vec<[f32; 3]>, a: [f32; 3], b: [f32; 3], c: [f32; 3]) {
    out.extend([a, b, c]);
}

/// A box w × h × d centred on the origin.
pub fn boxed(w: f32, h: f32, d: f32) -> Shape {
    let (x, y, z) = (w / 2.0, h / 2.0, d / 2.0);
    let c = |sx: f32, sy: f32, sz: f32| [sx * x, sy * y, sz * z];
    let mut t = Vec::new();
    let quads = [
        [
            c(1., -1., -1.),
            c(1., 1., -1.),
            c(1., 1., 1.),
            c(1., -1., 1.),
        ],
        [
            c(-1., -1., 1.),
            c(-1., 1., 1.),
            c(-1., 1., -1.),
            c(-1., -1., -1.),
        ],
        [
            c(-1., 1., -1.),
            c(-1., 1., 1.),
            c(1., 1., 1.),
            c(1., 1., -1.),
        ],
        [
            c(-1., -1., 1.),
            c(-1., -1., -1.),
            c(1., -1., -1.),
            c(1., -1., 1.),
        ],
        [
            c(-1., -1., 1.),
            c(1., -1., 1.),
            c(1., 1., 1.),
            c(-1., 1., 1.),
        ],
        [
            c(1., -1., -1.),
            c(-1., -1., -1.),
            c(-1., 1., -1.),
            c(1., 1., -1.),
        ],
    ];
    for q in quads {
        tri(&mut t, q[0], q[1], q[2]);
        tri(&mut t, q[0], q[2], q[3]);
    }
    Shape { tris: t }
}

/// A cylinder (or a cone when `rt` is 0) of height h centred on the origin, `seg` sides.
pub fn cylinder(rt: f32, rb: f32, h: f32, seg: usize) -> Shape {
    let mut t = Vec::new();
    let y0 = -h / 2.0;
    let y1 = h / 2.0;
    for k in 0..seg {
        let a0 = k as f32 / seg as f32 * TAU;
        let a1 = (k + 1) as f32 / seg as f32 * TAU;
        let (s0, c0, s1, c1) = (a0.sin(), a0.cos(), a1.sin(), a1.cos());
        let b0 = [rb * s0, y0, rb * c0];
        let b1 = [rb * s1, y0, rb * c1];
        let t0 = [rt * s0, y1, rt * c0];
        let t1 = [rt * s1, y1, rt * c1];
        if rt > 0.0 {
            tri(&mut t, b0, b1, t1);
            tri(&mut t, b0, t1, t0);
            tri(&mut t, [0.0, y1, 0.0], t0, t1);
        } else {
            tri(&mut t, b0, b1, [0.0, y1, 0.0]);
        }
        if rb > 0.0 {
            tri(&mut t, [0.0, y0, 0.0], b1, b0);
        }
    }
    Shape { tris: t }
}

pub fn cone(r: f32, h: f32, seg: usize) -> Shape {
    cylinder(0.0, r, h, seg)
}

/// An icosahedron of radius r, subdivided `detail` times (0: twenty faces).
pub fn icosahedron(r: f32, detail: u32) -> Shape {
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;
    let v = [
        [-1.0, t, 0.0],
        [1.0, t, 0.0],
        [-1.0, -t, 0.0],
        [1.0, -t, 0.0],
        [0.0, -1.0, t],
        [0.0, 1.0, t],
        [0.0, -1.0, -t],
        [0.0, 1.0, -t],
        [t, 0.0, -1.0],
        [t, 0.0, 1.0],
        [-t, 0.0, -1.0],
        [-t, 0.0, 1.0],
    ];
    let f = [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    let faces: Vec<[[f32; 3]; 3]> = f.iter().map(|q| [v[q[0]], v[q[1]], v[q[2]]]).collect();
    polyhedron(faces, r, detail)
}

/// A dodecahedron of radius r (each pentagon three triangles), subdivided `detail` times.
pub fn dodecahedron(r: f32, detail: u32) -> Shape {
    let t = (1.0 + 5.0f32.sqrt()) / 2.0;
    let q = 1.0 / t;
    let v: [[f32; 3]; 20] = [
        [-1., -1., -1.],
        [-1., -1., 1.],
        [-1., 1., -1.],
        [-1., 1., 1.],
        [1., -1., -1.],
        [1., -1., 1.],
        [1., 1., -1.],
        [1., 1., 1.],
        [0., -q, -t],
        [0., -q, t],
        [0., q, -t],
        [0., q, t],
        [-q, -t, 0.],
        [-q, t, 0.],
        [q, -t, 0.],
        [q, t, 0.],
        [-t, 0., -q],
        [t, 0., -q],
        [-t, 0., q],
        [t, 0., q],
    ];
    let idx: [usize; 108] = [
        3, 11, 7, 3, 7, 15, 3, 15, 13, 7, 19, 17, 7, 17, 6, 7, 6, 15, 17, 4, 8, 17, 8, 10, 17, 10,
        6, 8, 0, 16, 8, 16, 2, 8, 2, 10, 0, 12, 1, 0, 1, 18, 0, 18, 16, 6, 10, 2, 6, 2, 13, 6, 13,
        15, 2, 16, 18, 2, 18, 3, 2, 3, 13, 18, 1, 9, 18, 9, 11, 18, 11, 3, 4, 14, 12, 4, 12, 0, 4,
        0, 8, 11, 9, 5, 11, 5, 19, 11, 19, 7, 19, 5, 14, 19, 14, 4, 19, 4, 17, 1, 12, 14, 1, 14, 5,
        1, 5, 9,
    ];
    let faces: Vec<[[f32; 3]; 3]> = idx.chunks(3).map(|c| [v[c[0]], v[c[1]], v[c[2]]]).collect();
    polyhedron(faces, r, detail)
}

fn polyhedron(faces: Vec<[[f32; 3]; 3]>, r: f32, detail: u32) -> Shape {
    let norm = |p: [f32; 3]| {
        let l = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        [p[0] / l * r, p[1] / l * r, p[2] / l * r]
    };
    let mut out = Vec::new();
    let n = detail + 1;
    for [a, b, c] in faces {
        // subdivide each face into n² triangles on the sphere
        let at = |i: u32, j: u32| {
            let (fi, fj) = (i as f32 / n as f32, j as f32 / n as f32);
            let p = [
                a[0] + (b[0] - a[0]) * fi + (c[0] - a[0]) * fj,
                a[1] + (b[1] - a[1]) * fi + (c[1] - a[1]) * fj,
                a[2] + (b[2] - a[2]) * fi + (c[2] - a[2]) * fj,
            ];
            norm(p)
        };
        for i in 0..n {
            for j in 0..n - i {
                tri(&mut out, at(i, j), at(i + 1, j), at(i, j + 1));
                if i + j + 1 < n {
                    tri(&mut out, at(i + 1, j), at(i + 1, j + 1), at(i, j + 1));
                }
            }
        }
    }
    Shape { tris: out }
}

/// A UV sphere of radius r.
pub fn sphere(r: f32, seg: usize, rings: usize) -> Shape {
    let mut out = Vec::new();
    let p = |i: usize, j: usize| {
        let th = j as f32 / rings as f32 * PI;
        let ph = i as f32 / seg as f32 * TAU;
        [
            r * th.sin() * ph.sin(),
            r * th.cos(),
            r * th.sin() * ph.cos(),
        ]
    };
    for j in 0..rings {
        for i in 0..seg {
            let (a, b, c, d) = (p(i, j), p(i + 1, j), p(i, j + 1), p(i + 1, j + 1));
            if j > 0 {
                tri(&mut out, a, c, b);
            }
            if j + 1 < rings {
                tri(&mut out, b, c, d);
            }
        }
    }
    Shape { tris: out }
}

/// A flat plate in the x-y plane from a polygon outline, `depth` thick.
pub fn plate(points: &[[f32; 2]], depth: f32) -> Shape {
    let mut out = Vec::new();
    let z = depth / 2.0;
    // fan from the first point (the outlines are convex enough)
    for k in 1..points.len() - 1 {
        let (a, b, c) = (points[0], points[k], points[k + 1]);
        tri(&mut out, [a[0], a[1], z], [b[0], b[1], z], [c[0], c[1], z]);
        tri(
            &mut out,
            [a[0], a[1], -z],
            [c[0], c[1], -z],
            [b[0], b[1], -z],
        );
    }
    for k in 0..points.len() {
        let (a, b) = (points[k], points[(k + 1) % points.len()]);
        tri(
            &mut out,
            [a[0], a[1], -z],
            [b[0], b[1], -z],
            [b[0], b[1], z],
        );
        tri(&mut out, [a[0], a[1], -z], [b[0], b[1], z], [a[0], a[1], z]);
    }
    Shape { tris: out }
}

/// Many coloured shapes merged into one flat-shaded mesh.
#[derive(Default)]
pub struct Builder {
    pub vertices: Vec<MeshVertex3D>,
    pub indices: Vec<u32>,
}

impl Builder {
    pub fn add(&mut self, shape: &Shape, color: [f32; 3]) {
        self.add_shaded(shape, |_| color);
    }

    /// Each triangle its own colour (a little jitter per face, like the prototype's `painted`).
    pub fn add_shaded(&mut self, shape: &Shape, mut color: impl FnMut(usize) -> [f32; 3]) {
        for (k, t) in shape.tris.chunks_exact(3).enumerate() {
            let (a, b, c) = (
                Vec3::new(t[0][0], t[0][1], t[0][2]),
                Vec3::new(t[1][0], t[1][1], t[1][2]),
                Vec3::new(t[2][0], t[2][1], t[2][2]),
            );
            let n = (b - a).cross(c - a).normalize_or_zero();
            let col = color(k);
            let base = self.vertices.len() as u32;
            for p in t {
                let mut v = MeshVertex3D::new(*p, [n.x, n.y, n.z], [0.0, 0.0]);
                v.color = [col[0], col[1], col[2], 1.0];
                self.vertices.push(v);
            }
            self.indices.extend([base, base + 1, base + 2]);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn build(self) -> Option<Mesh3D> {
        (!self.vertices.is_empty()).then(|| Mesh3D {
            vertices: self.vertices,
            indices: self.indices,
            material: Material3D::default(),
            metallic_factor: 0.0,
            roughness_factor: 0.95,
            texture_index: None,
            normal_texture_index: None,
            skinning: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_have_whole_triangles_and_the_right_size() {
        for s in [
            boxed(1.0, 2.0, 3.0),
            cylinder(0.5, 1.0, 2.0, 7),
            cone(1.0, 2.0, 6),
            icosahedron(1.0, 0),
            dodecahedron(1.0, 1),
            sphere(1.0, 8, 6),
        ] {
            assert!(!s.tris.is_empty() && s.tris.len() % 3 == 0);
            let r = s
                .tris
                .iter()
                .map(|p| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt())
                .fold(0.0f32, f32::max);
            assert!(r > 0.4 && r < 2.0, "{r}");
        }
        assert_eq!(icosahedron(1.0, 0).tris.len(), 60);
        assert_eq!(dodecahedron(1.0, 0).tris.len(), 108);
    }
}
