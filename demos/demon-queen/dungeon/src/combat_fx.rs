//! Drawing the enemy combat effects (web `anim/combatFx.js`, and the grave
//! mage's telegraph and rupture from `actors.js`): weapon trails, spark and
//! dust bursts, the boss's shockwave ring and the fletched crossbow bolt.
use velvet_math::{Mat4, Vec3};
use velvet_render::{
    Material3D, Mesh3D, MeshDraw3D, MeshHandle, MeshVertex3D, PointLight3D, Transform3D,
};

use super::combat::{self, Fx};
use super::visual;

pub const TRAIL_SEGMENTS: usize = 14;
pub const TRAIL_LIFE: f32 = 0.16;

/// A ribbon between the base and the tip of a swung weapon (web `WeaponTrail`).
#[derive(Clone, Debug, Default)]
pub struct Trail {
    /// Newest first: (base, tip, time pushed).
    points: Vec<(Vec3, Vec3, f32)>,
    pub color: [f32; 3],
}

impl Trail {
    pub fn new(color: [f32; 3]) -> Self {
        Self {
            points: Vec::new(),
            color,
        }
    }

    pub fn push(&mut self, base: Vec3, tip: Vec3, now: f32) {
        self.points.insert(0, (base, tip, now));
        self.points.truncate(TRAIL_SEGMENTS + 1);
    }

    /// Drop faded points; whether enough remain to draw.
    pub fn prune(&mut self, now: f32) -> bool {
        while self.points.last().is_some_and(|p| now - p.2 > TRAIL_LIFE) {
            self.points.pop();
        }
        self.points.len() >= 2
    }

    /// Ribbon vertices: the tip edge bright, the inner edge fading into the blade.
    pub fn vertices(&self, now: f32) -> Vec<MeshVertex3D> {
        let n = self.points.len();
        let mut out = Vec::with_capacity((TRAIL_SEGMENTS + 1) * 2);
        for i in 0..=TRAIL_SEGMENTS {
            let (base, tip, time) = self.points[i.min(n - 1)];
            let fade = if i < n {
                (1.0 - (now - time) / TRAIL_LIFE).max(0.0)
                    * (1.0 - i as f32 / (TRAIL_SEGMENTS + 1) as f32)
            } else {
                0.0
            };
            let c = self.color;
            for (p, k) in [(base, 0.15), (tip, 0.95)] {
                out.push(MeshVertex3D {
                    position: p.to_array(),
                    normal: [0.0, 1.0, 0.0],
                    uv: [0.0; 2],
                    color: [c[0], c[1], c[2], fade * k],
                });
            }
        }
        out
    }
}

/// A double-sided ribbon mesh for `Trail::vertices`.
pub fn trail_mesh() -> Mesh3D {
    let mut mesh = visual::empty();
    mesh.material = Material3D::rgb(1.0, 1.0, 1.0);
    mesh.vertices = vec![
        MeshVertex3D {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            color: [1.0, 1.0, 1.0, 0.0]
        };
        (TRAIL_SEGMENTS + 1) * 2
    ];
    for i in 0..TRAIL_SEGMENTS {
        let a = (i * 2) as u32;
        mesh.indices.extend([a, a + 1, a + 3, a, a + 3, a + 2]);
        mesh.indices.extend([a + 3, a + 1, a, a + 2, a + 3, a]);
    }
    mesh
}

/// A flat ring on the floor, outer radius 1, both faces.
pub fn ring_mesh(inner: f32, segments: usize) -> Mesh3D {
    let mut mesh = visual::empty();
    let point = |k: usize, r: f32| {
        let a = k as f32 * std::f32::consts::TAU / segments as f32;
        Vec3::new(a.cos() * r, 0.0, a.sin() * r)
    };
    for k in 0..segments {
        let quad = [
            point(k, inner),
            point(k, 1.0),
            point(k + 1, 1.0),
            point(k + 1, inner),
        ];
        visual::quad(&mut mesh, quad, Vec3::Y, [1.0, 1.0, 1.0]);
        visual::quad(
            &mut mesh,
            [quad[3], quad[2], quad[1], quad[0]],
            -Vec3::Y,
            [1.0, 1.0, 1.0],
        );
    }
    mesh
}

/// Meshes the effects are drawn with.
pub struct FxMeshes {
    pub cube: MeshHandle,
    pub crystal: MeshHandle,
    pub ring: MeshHandle,
    pub fissure: MeshHandle,
    /// A unit sphere (orbs and glows).
    pub orb: MeshHandle,
}

fn glow(color: [f32; 3], emission: f32, alpha: f32) -> Material3D {
    let mut material = Material3D::rgb(color[0], color[1], color[2]).with_emission(emission);
    material.base_color[3] = alpha;
    material
}

/// A unit cube stretched from `from` to `to`.
fn along(from: Vec3, to: Vec3, width: f32) -> Mat4 {
    let d = to - from;
    let length = d.length().max(1e-4);
    let dir = d * (1.0 / length);
    let yaw = dir.x.atan2(dir.z);
    let pitch = -dir
        .y
        .atan2((dir.x * dir.x + dir.z * dir.z).sqrt().max(1e-4));
    Transform3D {
        position: (from + to) * 0.5,
        rotation: Vec3::new(pitch, yaw, 0.0),
        scale: Vec3::new(width, width, length),
    }
    .matrix()
}

fn flat(at: Vec3, spin: f32, scale: f32) -> Mat4 {
    Transform3D {
        position: at,
        rotation: Vec3::new(0.0, spin, 0.0),
        scale: Vec3::new(scale, scale, scale),
    }
    .matrix()
}

fn ball(at: Vec3, radius: f32) -> Mat4 {
    Mat4::from_translation(at) * Mat4::from_scale(Vec3::new(radius, radius, radius))
}

/// A glowing orb of magic: a hot core in a soft halo.
pub fn draw_orb(
    meshes: &FxMeshes,
    draws: &mut Vec<MeshDraw3D>,
    at: Vec3,
    radius: f32,
    color: [f32; 3],
) {
    let core = [
        0.6 + color[0] * 0.4,
        0.6 + color[1] * 0.4,
        0.6 + color[2] * 0.4,
    ];
    draws.push(MeshDraw3D {
        mesh: meshes.orb,
        transform: ball(at, radius * 0.55),
        material: glow(core, 2.4, 1.0),
        texture: None,
    });
    draws.push(MeshDraw3D {
        mesh: meshes.orb,
        transform: ball(at, radius),
        material: glow(color, 1.6, 0.45),
        texture: None,
    });
}

/// Draw every live effect and the light it throws; returns the triangle estimate.
pub fn draw(
    fx: &[Fx],
    meshes: &FxMeshes,
    draws: &mut Vec<MeshDraw3D>,
    lights: &mut Vec<PointLight3D>,
) -> usize {
    let mut triangles = 0;
    let mut push = |draws: &mut Vec<MeshDraw3D>, mesh, transform, material, tris| {
        draws.push(MeshDraw3D {
            mesh,
            transform,
            material,
            texture: None,
        });
        triangles += tris;
    };
    for effect in fx {
        match *effect {
            Fx::Bolt { from, to, age, .. } => {
                let dir = (to - from).normalize_or_zero();
                let at = combat::bolt_position(from, to, age);
                // Shaft, iron head and three red vanes, spinning in flight.
                push(
                    draws,
                    meshes.cube,
                    along(at - dir * 0.31, at + dir * 0.31, 0.028),
                    Material3D::rgb(0.48, 0.31, 0.17),
                    12,
                );
                push(
                    draws,
                    meshes.crystal,
                    along(at + dir * 0.31, at + dir * 0.44, 0.07),
                    Material3D::rgb(0.81, 0.78, 0.7),
                    12,
                );
                let side = dir.cross(Vec3::Y).normalize_or_zero();
                let up = side.cross(dir);
                for k in 0..3 {
                    let a = age * 18.0 + k as f32 * std::f32::consts::TAU / 3.0;
                    let offset = (side * a.cos() + up * a.sin()) * 0.035;
                    let centre = at - dir * 0.24 + offset;
                    push(
                        draws,
                        meshes.cube,
                        along(centre - dir * 0.08, centre + dir * 0.08, 0.02),
                        Material3D::rgb(0.72, 0.2, 0.16),
                        12,
                    );
                }
            }
            Fx::Burst {
                at,
                color,
                count,
                speed,
                life,
                gravity,
                up,
                length,
                seed,
                age,
            } => {
                let t = age / life;
                let material = glow(color, 1.1, 1.0 - t * t);
                for (head, tail) in
                    combat::burst_segments(at, count, speed, gravity, up, length, seed, age)
                {
                    push(draws, meshes.cube, along(tail, head, 0.016), material, 12);
                }
            }
            Fx::Shockwave { at, radius, age } => {
                let t = age / combat::SHOCKWAVE_SECONDS;
                let eased = 1.0 - (1.0 - t).powi(3);
                let material = glow([1.0, 0.45, 0.15], 1.6, (1.0 - t).powf(1.5));
                push(
                    draws,
                    meshes.ring,
                    flat(at + Vec3::Y * 0.05, 0.0, 0.25 + eased * radius),
                    material,
                    96,
                );
            }
            Fx::Telegraph {
                at,
                age,
                life,
                radius,
            } => {
                // Pulses faster as the orb nears.
                let t = (age / life).min(1.0);
                let size = radius / combat::CAST_RADIUS;
                let scale =
                    (0.86 + (age * std::f32::consts::PI * (3.0 + t * 6.0)).sin() * 0.08) * size;
                let spin = age * 0.9;
                let at = at + Vec3::Y * 0.07;
                push(
                    draws,
                    meshes.ring,
                    flat(at, spin, 1.235 * scale),
                    glow([1.0, 0.44, 0.26], 0.72 + t, 0.8),
                    64,
                );
                push(
                    draws,
                    meshes.fissure,
                    flat(at - Vec3::Y * 0.04, spin, scale),
                    glow([1.0, 0.7, 0.34], 0.9 + t, 0.88),
                    64,
                );
            }
            Fx::Spell {
                from,
                to,
                age,
                color,
                ..
            } => {
                let at = combat::spell_position(from, to, age);
                let pulse = 1.0 + (age * 23.0).sin() * 0.12;
                draw_orb(meshes, draws, at, 0.24 * pulse, color);
                // A fading tail along the path it flew.
                for k in 1..7 {
                    let back = combat::spell_position(from, to, (age - k as f32 * 0.035).max(0.0));
                    let r = 0.16 * (1.0 - k as f32 / 7.0);
                    push(
                        draws,
                        meshes.orb,
                        ball(back, r),
                        glow(color, 1.4, 0.5 * (1.0 - k as f32 / 7.0)),
                        96,
                    );
                }
                lights.push(PointLight3D::new(at, color, 2.6, 4.5));
            }
            Fx::Debris {
                at,
                size,
                seed,
                age,
            } => {
                // Chunks of the smashed piece tumble out and fall.
                let t = age / combat::DEBRIS_SECONDS;
                for k in 0..9u32 {
                    let r = |j: u32| combat::unit(seed, k * 8 + j);
                    let angle = r(0) * std::f32::consts::TAU;
                    let speed = (1.6 + r(1) * 2.4) * size;
                    let v = Vec3::new(angle.cos() * speed, 2.5 + r(2) * 3.0, angle.sin() * speed);
                    let mut p = at + Vec3::Y * (0.6 * size) + v * age - Vec3::Y * (4.9 * age * age);
                    p.y = p.y.max(at.y + 0.08);
                    let chunk = (0.12 + r(3) * 0.16) * size * (1.0 - t * 0.3);
                    let spin = age * (4.0 + r(4) * 6.0);
                    let transform = Transform3D {
                        position: p,
                        rotation: Vec3::new(spin, spin * 0.7 + angle, 0.0),
                        scale: Vec3::new(chunk, chunk * 0.8, chunk * 1.1),
                    }
                    .matrix();
                    push(
                        draws,
                        meshes.cube,
                        transform,
                        Material3D::rgb(0.46, 0.4, 0.34),
                        12,
                    );
                }
            }
            Fx::Summon { at, age } => {
                // A dark ring of blood-light that opens, then closes.
                let t = age / combat::SUMMON_SECONDS;
                let open = (t * std::f32::consts::PI).sin();
                let at = at + Vec3::Y * 0.06;
                push(
                    draws,
                    meshes.ring,
                    flat(at, age * 2.0, 1.1 * open),
                    glow([0.9, 0.08, 0.1], 1.8, 0.9),
                    96,
                );
                push(
                    draws,
                    meshes.fissure,
                    flat(at, -age * 1.3, 1.0 * open),
                    glow([0.35, 0.02, 0.05], 0.8, 0.95),
                    64,
                );
                for k in 0..6 {
                    let a = k as f32 / 6.0 * std::f32::consts::TAU + age * 1.5;
                    let rise = (age * 1.6 + k as f32 * 0.2) % 1.2;
                    let p = at + Vec3::new(a.cos() * 0.8 * open, rise, a.sin() * 0.8 * open);
                    push(
                        draws,
                        meshes.orb,
                        ball(p, 0.05),
                        glow([1.0, 0.18, 0.12], 2.0, 1.0 - rise / 1.2),
                        0,
                    );
                }
                lights.push(PointLight3D::new(
                    at + Vec3::Y * 0.5,
                    [1.0, 0.12, 0.08],
                    2.4 * open,
                    4.0,
                ));
            }
            Fx::Rupture { at, age } => {
                let t = (age / combat::RUPTURE_SECONDS).min(1.0);
                let scale = 1.0 + (t * std::f32::consts::PI).sin() * 0.16;
                let spin = age * 0.7;
                let base = at + Vec3::Y * 0.08;
                let turn = Mat4::from_translation(base)
                    * Mat4::from_rotation_y(spin)
                    * Mat4::from_scale(Vec3::new(scale, scale, scale));
                for index in 0..7 {
                    let angle = index as f32 * std::f32::consts::TAU / 7.0;
                    let height = 0.35 + (index % 3) as f32 * 0.13;
                    let reach = 0.5 + (index % 2) as f32 * 0.2;
                    let radius = if index % 2 == 0 { 0.2 } else { 0.27 };
                    let local = Mat4::from_translation(Vec3::new(
                        angle.cos() * reach,
                        height * 0.43,
                        angle.sin() * reach,
                    )) * Mat4::from_rotation_y(angle)
                        * Mat4::from_rotation_x(angle * 0.3)
                        * Mat4::from_scale(Vec3::new(
                            0.65 * radius * 2.0,
                            height * radius * 2.0,
                            0.75 * radius * 2.0,
                        ));
                    let material = if index % 3 == 0 {
                        glow([1.0, 0.54, 0.31], 1.05, 1.0)
                    } else {
                        Material3D::rgb(0.55, 0.35, 0.25).with_emission(0.28)
                    };
                    push(draws, meshes.crystal, turn * local, material, 12);
                }
                push(
                    draws,
                    meshes.ring,
                    turn * Mat4::from_translation(Vec3::Y * 0.035)
                        * Mat4::from_scale(Vec3::new(1.015, 1.015, 1.015)),
                    glow([1.0, 0.54, 0.31], 1.05, 1.0),
                    64,
                );
            }
        }
    }
    triangles
}
