//! Three authored first-person sword arcs, translated from the web prototype.
//! Camera-space key poses and Catmull-Rom/spherical direction interpolation
//! are pure math; hit timing reads the same clips as rendering.
use super::visual;
use velvet_math::{Mat4, Vec3};
use velvet_render::{Camera3D, Material3D, Mesh3D, MeshVertex3D};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub hand: Vec3,
    pub blade: Vec3,
    pub roll: f32,
}

pub const REST: Pose = Pose {
    hand: Vec3::new(0.43, -0.31, -0.55),
    blade: Vec3::new(-0.15, 0.95, -0.27),
    roll: 0.5,
};
pub const VIEW_SCALE: f32 = 0.58;

#[derive(Clone, Copy)]
enum Ease {
    In,
    Out,
    InOut,
    Linear,
}
#[derive(Clone, Copy)]
struct Key {
    t: f32,
    hand: Vec3,
    blade: Vec3,
    roll: f32,
    ease: Ease,
}
const fn key(t: f32, hand: Vec3, blade: Vec3, roll: f32, ease: Ease) -> Key {
    Key {
        t,
        hand,
        blade,
        roll,
        ease,
    }
}
const FOREHAND: [Key; 5] = [
    key(
        0.07,
        Vec3::new(0.30, -0.06, -0.42),
        Vec3::new(0.55, 0.8, 0.15),
        0.0,
        Ease::Out,
    ),
    key(
        0.11,
        Vec3::new(0.20, -0.08, -0.52),
        Vec3::new(0.10, 0.95, -0.30),
        0.0,
        Ease::In,
    ),
    key(
        0.16,
        Vec3::new(-0.02, -0.26, -0.54),
        Vec3::new(-0.88, 0.05, -0.47),
        0.0,
        Ease::Linear,
    ),
    key(
        0.25,
        Vec3::new(-0.10, -0.36, -0.46),
        Vec3::new(-0.75, -0.55, 0.35),
        0.0,
        Ease::Out,
    ),
    key(0.46, REST.hand, REST.blade, REST.roll, Ease::InOut),
];
const BACKHAND: [Key; 5] = [
    key(
        0.07,
        Vec3::new(-0.06, -0.20, -0.44),
        Vec3::new(-0.8, 0.5, 0.2),
        0.0,
        Ease::Out,
    ),
    key(
        0.11,
        Vec3::new(0.04, -0.18, -0.52),
        Vec3::new(-0.3, 0.9, -0.3),
        0.0,
        Ease::In,
    ),
    key(
        0.16,
        Vec3::new(0.26, -0.16, -0.52),
        Vec3::new(0.9, 0.2, -0.38),
        0.0,
        Ease::Linear,
    ),
    key(
        0.25,
        Vec3::new(0.32, -0.12, -0.42),
        Vec3::new(0.75, 0.5, 0.42),
        0.0,
        Ease::Out,
    ),
    key(0.46, REST.hand, REST.blade, REST.roll, Ease::InOut),
];
const OVERHEAD: [Key; 5] = [
    key(
        0.10,
        Vec3::new(0.12, 0.12, -0.50),
        Vec3::new(0.1, 0.7, 0.7),
        0.0,
        Ease::Out,
    ),
    key(
        0.16,
        Vec3::new(0.08, 0.06, -0.56),
        Vec3::new(-0.05, 0.98, -0.2),
        0.0,
        Ease::In,
    ),
    key(
        0.22,
        Vec3::new(0.00, -0.26, -0.56),
        Vec3::new(-0.6, -0.3, -0.74),
        0.0,
        Ease::Linear,
    ),
    key(
        0.33,
        Vec3::new(-0.02, -0.36, -0.50),
        Vec3::new(-0.55, -0.6, -0.58),
        0.0,
        Ease::Out,
    ),
    key(0.62, REST.hand, REST.blade, REST.roll, Ease::InOut),
];

#[derive(Clone, Copy)]
pub struct Swing {
    pub duration: f32,
    pub strike: f32,
    pub chain: f32,
    pub trail: (f32, f32),
    keys: &'static [Key; 5],
}
pub const SWINGS: [Swing; 3] = [
    Swing {
        duration: 0.46,
        strike: 0.135,
        chain: 0.44,
        trail: (0.08, 0.22),
        keys: &FOREHAND,
    },
    Swing {
        duration: 0.46,
        strike: 0.135,
        chain: 0.44,
        trail: (0.08, 0.22),
        keys: &BACKHAND,
    },
    Swing {
        duration: 0.62,
        strike: 0.19,
        chain: 0.0,
        trail: (0.13, 0.27),
        keys: &OVERHEAD,
    },
];
pub const CHAIN_COMMIT: f32 = 0.25;

fn ease(mode: Ease, x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    match mode {
        Ease::In => x * x,
        Ease::Out => 1.0 - (1.0 - x) * (1.0 - x),
        Ease::InOut => x * x * (3.0 - 2.0 * x),
        Ease::Linear => x,
    }
}
fn catmull(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let t2 = t * t;
    let t3 = t2 * t;
    (p1 * 2.0
        + (p2 - p0) * t
        + (p0 * 2.0 - p1 * 5.0 + p2 * 4.0 - p3) * t2
        + (p3 - p0 + p1 * 3.0 - p2 * 3.0) * t3)
        * 0.5
}
fn slerp_dir(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    let a = a.normalize_or_zero();
    let b = b.normalize_or_zero();
    let dot = a.dot(b).clamp(-1.0, 1.0);
    if dot > 0.9995 {
        return a.lerp(b, t).normalize_or_zero();
    }
    let theta = dot.acos();
    let inv = 1.0 / theta.sin().max(0.0001);
    (a * ((1.0 - t) * theta).sin() + b * (t * theta).sin()) * inv
}

/// Pose at clip time, starting from the current pose when chaining.
pub fn pose(swing: usize, t: f32, from: Pose) -> Pose {
    let clip = SWINGS[swing.min(2)];
    if t >= clip.duration {
        return REST;
    }
    let mut keys = [key(0.0, from.hand, from.blade, from.roll, Ease::Linear); 6];
    keys[1..].copy_from_slice(clip.keys);
    let mut i = 0;
    while i + 1 < keys.len() - 1 && keys[i + 1].t < t {
        i += 1;
    }
    let a = keys[i];
    let b = keys[i + 1];
    let u = ease(b.ease, (t - a.t) / (b.t - a.t).max(1.0e-6));
    let p0 = keys[i.saturating_sub(1)].hand;
    let p3 = keys[(i + 2).min(keys.len() - 1)].hand;
    Pose {
        hand: catmull(p0, a.hand, b.hand, p3, u),
        blade: slerp_dir(a.blade, b.blade, u),
        roll: a.roll + (b.roll - a.roll) * u,
    }
}

pub struct WorldPose {
    pub hand: Vec3,
    pub blade: Vec3,
    pub shoulder: Vec3,
}
pub fn camera_vector(camera: Camera3D, v: Vec3) -> Vec3 {
    let front = (camera.target - camera.position).normalize_or_zero();
    let right = front.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(front).normalize_or_zero();
    right * v.x + up * v.y - front * v.z
}
/// Convert Three.js camera-space pose (+X right, +Y up, -Z forward) to WGPU.
pub fn world_pose(
    camera: Camera3D,
    pose: Pose,
    walk_sway: f32,
    walk_bob: f32,
    breath: f32,
) -> WorldPose {
    let p = pose.hand + Vec3::new(walk_sway * 0.012, walk_bob * -0.014 + breath * 0.004, 0.0);
    WorldPose {
        hand: camera.position + camera_vector(camera, p),
        blade: camera_vector(camera, pose.blade).normalize_or_zero(),
        shoulder: camera.position + camera_vector(camera, Vec3::new(0.20, -0.30, 0.06)),
    }
}

/// Grip frame shared by the actual asset and the procedural fist. The two
/// transforms must never drift apart or the sword will appear to float.
pub fn grip_frame(hand: Vec3, blade: Vec3, shoulder: Vec3, roll: f32) -> Mat4 {
    let y = blade.normalize_or_zero();
    let mut x = shoulder - hand - y * (shoulder - hand).dot(y);
    if x.length_squared() < 1.0e-6 {
        x = Vec3::X - y * y.x;
    }
    x = x.normalize_or_zero();
    let z = x.cross(y).normalize_or_zero();
    let (s, c) = roll.sin_cos();
    let xr = x * c - z * s;
    let zr = x * s + z * c;
    Mat4::from_cols(
        [xr.x, xr.y, xr.z, 0.0],
        [y.x, y.y, y.z, 0.0],
        [zr.x, zr.y, zr.z, 0.0],
        [hand.x, hand.y, hand.z, 1.0],
    )
}

/// Exact GLB grip alignment measured in the web cave. Original mesh units,
/// not a bounding-box approximation: hilt meets the camera-space fist.
pub fn sword_transform(hand: Vec3, blade: Vec3, shoulder: Vec3, roll: f32) -> Mat4 {
    const GRIP: Vec3 = Vec3::new(-0.2682, 0.2017, 0.0);
    let axis = Vec3::new(0.631, 0.776, 0.002).normalize_or_zero();
    let v = axis.cross(Vec3::Y);
    let k = 1.0 / (1.0 + axis.dot(Vec3::Y));
    let rotate = |p: Vec3| p + v.cross(p) + v.cross(v.cross(p)) * k;
    let local = Mat4::from_cols(
        [rotate(Vec3::X).x, rotate(Vec3::X).y, rotate(Vec3::X).z, 0.0],
        [rotate(Vec3::Y).x, rotate(Vec3::Y).y, rotate(Vec3::Y).z, 0.0],
        [rotate(Vec3::Z).x, rotate(Vec3::Z).y, rotate(Vec3::Z).z, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    );
    grip_frame(hand, blade, shoulder, roll)
        * Mat4::from_scale(Vec3::new(
            0.8 / 1.287 * VIEW_SCALE,
            0.8 / 1.287 * VIEW_SCALE,
            0.8 / 1.287 * VIEW_SCALE,
        ))
        * local
        * Mat4::from_translation(-GRIP)
}

/// Two-bone arm solution from the web viewmodel. Returns adjusted shoulder
/// and elbow; the shoulder leans in only when the authored hand goes out of
/// reach, avoiding a detached forearm during the overhead swing.
pub fn arm_ik(shoulder: Vec3, wrist: Vec3, pole: Vec3) -> (Vec3, Vec3) {
    const UPPER: f32 = 0.34;
    const FORE: f32 = 0.33;
    let reach = (UPPER + FORE) * 0.985;
    let mut s = shoulder;
    let toward = wrist - s;
    let distance = toward.length();
    if distance > reach {
        s += toward.normalize_or_zero() * (distance - reach);
    }
    let d = (wrist - s)
        .length()
        .clamp((UPPER - FORE).abs() + 0.001, reach);
    let u = (wrist - s).normalize_or_zero();
    let mut p = pole - u * pole.dot(u);
    if p.length_squared() < 1.0e-6 {
        p = Vec3::Y - u * u.y;
    }
    p = p.normalize_or_zero();
    let cos_a = ((UPPER * UPPER + d * d - FORE * FORE) / (2.0 * UPPER * d)).clamp(-1.0, 1.0);
    let sin_a = (1.0 - cos_a * cos_a).max(0.0).sqrt();
    (s, s + u * (UPPER * cos_a) + p * (UPPER * sin_a))
}

/// Place the +Y, 0..1 sleeve mesh between two joints; its cross-section uses
/// the same 0.58 viewmodel scale as the web version, but arm reach is metric.
pub fn limb_transform(from: Vec3, to: Vec3) -> Mat4 {
    let span = to - from;
    let y = span.normalize_or_zero();
    let reference = if y.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let x = reference.cross(y).normalize_or_zero();
    let z = x.cross(y).normalize_or_zero();
    Mat4::from_cols(
        [x.x * VIEW_SCALE, x.y * VIEW_SCALE, x.z * VIEW_SCALE, 0.0],
        [span.x, span.y, span.z, 0.0],
        [z.x * VIEW_SCALE, z.y * VIEW_SCALE, z.z * VIEW_SCALE, 0.0],
        [from.x, from.y, from.z, 1.0],
    )
}

pub fn wrist(frame: Mat4, right: bool) -> Vec3 {
    frame.transform_point3(Vec3::new(0.1, 0.0, if right { -0.034 } else { 0.034 }) * VIEW_SCALE)
}

fn tube(mesh: &mut Mesh3D, path: &[Vec3], radius: f32, color: [f32; 3]) {
    for pair in path.windows(2) {
        let axis = (pair[1] - pair[0]).normalize_or_zero();
        let helper = if axis.y.abs() > 0.8 { Vec3::X } else { Vec3::Y };
        let x = helper.cross(axis).normalize_or_zero();
        let z = axis.cross(x).normalize_or_zero();
        let point = |center: Vec3, j: usize| {
            let a = j as f32 * std::f32::consts::TAU / 7.0;
            center + (x * a.cos() + z * a.sin()) * radius
        };
        for j in 0..7 {
            let n = x * ((j as f32 + 0.5) * std::f32::consts::TAU / 7.0).cos()
                + z * ((j as f32 + 0.5) * std::f32::consts::TAU / 7.0).sin();
            visual::quad(
                mesh,
                [
                    point(pair[0], j),
                    point(pair[0], j + 1),
                    point(pair[1], j + 1),
                    point(pair[1], j),
                ],
                n,
                color,
            );
        }
    }
}

/// Glove closed around the sword or torch. This is actual scene geometry,
/// not a screen-space decal; wrist, fingers and cuff share the grip frame.
pub fn fist_mesh(right: bool) -> Mesh3D {
    let mut m = visual::empty();
    let side = if right { -1.0 } else { 1.0 };
    let glove = [0.105, 0.085, 0.095];
    let shade = [0.19, 0.15, 0.18];
    let gold = [0.86, 0.60, 0.20];
    let point = |x: f32, y: f32, z: f32| Vec3::new(x, y, z * side);
    visual::cuboid(
        &mut m,
        point(0.067, 0.0, 0.037),
        Vec3::new(0.046, 0.045, 0.020),
        glove,
    );
    visual::cuboid(
        &mut m,
        point(0.027, -0.004, -0.020),
        Vec3::new(0.029, 0.042, 0.015),
        visual::tint(glove, 0.83),
    );
    let around = |degrees: f32, radius: f32, y: f32| {
        let angle = degrees.to_radians();
        point(angle.cos() * radius, y, angle.sin() * radius)
    };
    for (y, r, tip) in [
        (0.034, 0.0118, 318.0),
        (0.012, 0.0122, 322.0),
        (-0.011, 0.0115, 316.0),
        (-0.032, 0.0098, 300.0),
    ] {
        let path = [72.0, 120.0, 168.0, 212.0, 256.0, tip].map(|a| around(a, 0.036 + r + 0.003, y));
        tube(&mut m, &path, r, glove);
        let knuckle = around(72.0, 0.036 + r + 0.005, y);
        visual::cuboid(&mut m, knuckle, Vec3::new(r * 1.1, r * 1.1, r * 1.1), shade);
    }
    tube(
        &mut m,
        &[
            point(0.074, 0.036, -0.004),
            point(0.058, 0.052, -0.030),
            point(0.032, 0.058, -0.048),
            point(0.002, 0.052, -0.052),
            point(-0.020, 0.043, -0.046),
        ],
        0.014,
        glove,
    );
    // Turned-back leather cuff and a narrow brass seam at the wrist.
    tube(
        &mut m,
        &[point(0.087, 0.0, 0.034), point(0.128, 0.0, 0.041)],
        0.037,
        glove,
    );
    tube(
        &mut m,
        &[point(0.126, 0.0, 0.041), point(0.132, 0.0, 0.042)],
        0.039,
        gold,
    );
    m
}

/// Folded cloth sleeve; local +Y is wrist-to-elbow or elbow-to-shoulder.
pub fn sleeve_mesh(cuff: bool) -> Mesh3D {
    let mut m = visual::empty();
    let rows = 14;
    let sides = 12;
    let color = [0.42, 0.05, 0.07];
    let ring = |y: f32, j: usize, wide: f32| {
        let a = j as f32 * std::f32::consts::TAU / sides as f32;
        let fold = 1.0 + 0.07 * (a * 5.0 + y * 9.0).sin() * (y * std::f32::consts::PI).sin();
        Vec3::new(
            a.cos() * 0.044 * (1.0 + 0.16 * y) * fold * wide,
            y,
            a.sin() * 0.044 * (1.0 + 0.16 * y) * fold * wide,
        )
    };
    for row in 0..rows {
        let y0 = row as f32 / rows as f32;
        let y1 = (row + 1) as f32 / rows as f32;
        for j in 0..sides {
            let angle = j as f32 * std::f32::consts::TAU / sides as f32;
            let shade = 0.8 + 0.2 * (angle - 1.2).cos() + 0.06 * (angle * 5.0 + y0 * 9.0).sin();
            let n = Vec3::new(angle.cos(), 0.0, angle.sin());
            visual::quad(
                &mut m,
                [
                    ring(y0, j, 1.0),
                    ring(y0, j + 1, 1.0),
                    ring(y1, j + 1, 1.0),
                    ring(y1, j, 1.0),
                ],
                n,
                visual::tint(color, shade),
            );
        }
    }
    if cuff {
        for (y0, y1, radius, c) in [
            (-0.012, 0.05, 1.30, color),
            (-0.014, -0.008, 1.36, [0.86, 0.60, 0.20]),
            (0.060, 0.066, 1.26, [0.72, 0.49, 0.18]),
        ] {
            for j in 0..sides {
                let n = Vec3::new(
                    (j as f32 * std::f32::consts::TAU / sides as f32).cos(),
                    0.0,
                    (j as f32 * std::f32::consts::TAU / sides as f32).sin(),
                );
                visual::quad(
                    &mut m,
                    [
                        ring(y0, j, radius),
                        ring(y0, j + 1, radius),
                        ring(y1, j + 1, radius),
                        ring(y1, j, radius),
                    ],
                    n,
                    c,
                );
            }
        }
    }
    m
}

const TRAIL_SAMPLES: usize = 7;
/// Reusable index topology. Only the twelve-ish vertex positions/colors are
/// replaced as a swing progresses; no per-frame GPU allocation or mesh leak.
pub fn trail_mesh() -> Mesh3D {
    let mut mesh = visual::empty();
    mesh.material = Material3D::rgb(1.0, 0.80, 0.42);
    mesh.vertices = vec![
        MeshVertex3D {
            position: [0.0; 3],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0; 2],
            color: [1.0, 0.88, 0.56, 0.0],
        };
        TRAIL_SAMPLES * 2
    ];
    for i in 0..TRAIL_SAMPLES - 1 {
        let a = (i * 2) as u32;
        mesh.indices.extend([a, a + 1, a + 3, a, a + 3, a + 2]);
        mesh.indices.extend([a + 3, a + 1, a, a + 2, a + 3, a]);
    }
    mesh
}

/// Reconstruct a 120 ms sword trail directly from the authored animation.
/// This stays deterministic even when the render rate or hit-stop varies.
pub fn trail_vertices(
    camera: Camera3D,
    combo: usize,
    elapsed: f32,
    from: Pose,
    walk_sway: f32,
    walk_bob: f32,
    breath: f32,
) -> Vec<MeshVertex3D> {
    let mut vertices = trail_mesh().vertices;
    let start = (elapsed - 0.065).max(SWINGS[combo.min(2)].trail.0);
    for i in 0..TRAIL_SAMPLES {
        let age = i as f32 / (TRAIL_SAMPLES - 1) as f32;
        let time = elapsed + (start - elapsed) * age;
        let authored = pose(combo, time, from);
        let world = world_pose(camera, authored, walk_sway, walk_bob, breath);
        let alpha = (1.0 - age).powf(1.5) * 0.46;
        // Keep the streak near the tip. Joining hilt-to-tip across past poses
        // creates a huge translucent sail over the entire first-person view.
        for (edge, along) in [0.35, 0.45].into_iter().enumerate() {
            let p = world.hand + world.blade * along;
            let v = &mut vertices[i * 2 + edge];
            v.position = p.to_array();
            v.color = [1.0, 0.88 - age * 0.20, 0.56 - age * 0.25, alpha];
            v.uv = [edge as f32, age];
        }
    }
    vertices
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_arcs_are_finite_and_cross_the_screen_in_opposite_directions() {
        let mut impact_x = [0.0; 3];
        for (index, clip) in SWINGS.iter().enumerate() {
            assert_eq!(pose(index, clip.duration, REST), REST);
            for step in 0..=100 {
                let p = pose(index, clip.duration * step as f32 / 100.0, REST);
                assert!(p
                    .hand
                    .to_array()
                    .into_iter()
                    .chain(p.blade.to_array())
                    .chain([p.roll])
                    .all(|x| x.is_finite()));
                assert!((p.blade.length() - 1.0).abs() < 0.01 || step == 100);
            }
            impact_x[index] = pose(index, clip.strike, REST).blade.x;
        }
        assert!(impact_x[0] < 0.0 && impact_x[1] > 0.0);
        assert!(SWINGS[2].duration > SWINGS[0].duration);
    }
    #[test]
    fn sword_glb_grip_reaches_the_fist_and_axis_follows_the_blade() {
        let hand = Vec3::new(1.0, 2.0, 3.0);
        let blade = Vec3::new(-0.25, 0.9, -0.2).normalize_or_zero();
        let transform = sword_transform(hand, blade, hand + Vec3::new(0.3, -0.2, 0.1), 0.5);
        let grip = Vec3::new(-0.2682, 0.2017, 0.0);
        assert!((transform.transform_point3(grip) - hand).length() < 1.0e-5);
        let tip = grip + Vec3::new(0.631, 0.776, 0.002).normalize_or_zero() * 1.287;
        assert!(
            (transform.transform_point3(tip) - (hand + blade * 0.8 * VIEW_SCALE)).length() < 1.0e-4
        );
    }

    #[test]
    fn arm_reaches_swing_and_trail_topology_is_stable() {
        let camera = Camera3D {
            position: Vec3::new(0.0, 1.6, 0.0),
            target: Vec3::new(0.0, 1.6, -1.0),
            ..Camera3D::default()
        };
        let mesh = trail_mesh();
        assert_eq!(mesh.vertices.len(), 14);
        assert_eq!(mesh.indices.len(), 72);
        for combo in 0..3 {
            for i in 0..=20 {
                let t = SWINGS[combo].duration * i as f32 / 20.0;
                let p = pose(combo, t, REST);
                let w = world_pose(camera, p, 0.0, 0.0, 0.0);
                let frame = grip_frame(w.hand, w.blade, w.shoulder, p.roll);
                let wrist = wrist(frame, true);
                let (shoulder, elbow) = arm_ik(
                    w.shoulder,
                    wrist,
                    Vec3::new(0.7, -1.0, 0.25).normalize_or_zero(),
                );
                assert!((elbow - shoulder).length() < 0.341);
                assert!((elbow - wrist).length() < 0.331);
                let trail = trail_vertices(camera, combo, t, REST, 0.0, 0.0, 0.0);
                assert_eq!(trail.len(), mesh.vertices.len());
                assert!(trail
                    .iter()
                    .all(|v| v.position.iter().all(|x| x.is_finite())));
            }
        }
    }
}
