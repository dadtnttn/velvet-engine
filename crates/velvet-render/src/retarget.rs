//! Clips from one rig onto another: a Mixamo walk onto a character's own skeleton, one Mixamo
//! rig's clips onto another's with different rests, and so on (Demon Queen borrows its spa girl's
//! sitting and lying clips for the Queen's rig this way).
//!
//! Each mapped bone points the way its counterpart points, frame by frame: the counterpart's turn
//! away from its rest pose, applied to the target bone after lining the two rests' bone directions
//! up (`R_t = D_s · A · R_t,rest`, `D_s = R_s · R_s,rest⁻¹`, `A` the shortest turn from the target's
//! rest direction to the source's). Bones without a counterpart ride along with their parents. Only
//! rotations are copied: where the body sits is left to whoever places it.
//!
//! A bone map lists `(target bone, source bone, Some((target child, source child)))`: the children
//! give each pair's rest direction; without them the rests are taken as already lined up.

use velvet_math::{Mat4, Vec3};

use crate::mesh3d::{GltfModel, Render3DError, RotationTracks3D};

/// A rotation as `[x, y, z, w]`.
pub type Quat = [f32; 4];

/// One line of a bone map: `(target bone, source bone, Some((target child, source child)))`.
pub type BonePair<'a> = (&'a str, &'a str, Option<(&'a str, &'a str)>);

/// A rest node as [`GltfModel::rest_nodes`] gives it: name, parent, translation, rotation.
type RestNode = (String, Option<usize>, Vec3, [f32; 4]);

/// `a` after `b` (the Hamilton product).
pub fn q_mul(a: Quat, b: Quat) -> Quat {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

/// The inverse of a unit quaternion.
pub fn q_conj(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

/// Scaled back to unit length.
pub fn q_norm(q: Quat) -> Quat {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
        .sqrt()
        .max(1e-9);
    q.map(|v| v / l)
}

/// The rotation part of a global transform, as a quaternion (its columns normalised first: rigs
/// often carry a scale on their root).
pub fn mat_rotation(m: Mat4) -> Quat {
    let c = m.to_cols_array();
    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let x = norm([c[0], c[1], c[2]]);
    let y = norm([c[4], c[5], c[6]]);
    let z = norm([c[8], c[9], c[10]]);
    let (m00, m11, m22) = (x[0], y[1], z[2]);
    let trace = m00 + m11 + m22;
    let q = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [
            (y[2] - z[1]) / s,
            (z[0] - x[2]) / s,
            (x[1] - y[0]) / s,
            0.25 * s,
        ]
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        [
            0.25 * s,
            (y[0] + x[1]) / s,
            (z[0] + x[2]) / s,
            (y[2] - z[1]) / s,
        ]
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        [
            (y[0] + x[1]) / s,
            0.25 * s,
            (z[1] + y[2]) / s,
            (z[0] - x[2]) / s,
        ]
    } else {
        let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        [
            (z[0] + x[2]) / s,
            (z[1] + y[2]) / s,
            0.25 * s,
            (x[1] - y[0]) / s,
        ]
    };
    q_norm(q)
}

/// The shortest rotation taking direction `a` onto `b`.
pub fn q_between(a: [f32; 3], b: [f32; 3]) -> Quat {
    let n = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let (a, b) = (n(a), n(b));
    let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    if d < -0.9999 {
        let axis = n(if a[0].abs() < 0.9 {
            [0.0, -a[2], a[1]]
        } else {
            [a[2], 0.0, -a[0]]
        });
        return [axis[0], axis[1], axis[2], 0.0];
    }
    let c = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    q_norm([c[0], c[1], c[2], 1.0 + d])
}

/// A turn of `angle` radians about `axis` (any length).
pub fn q_axis(axis: [f32; 3], angle: f32) -> Quat {
    let l = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
        .sqrt()
        .max(1e-9);
    let (s, c) = (angle * 0.5).sin_cos();
    [axis[0] / l * s, axis[1] / l * s, axis[2] / l * s, c]
}

/// A Mixamo rig onto another Mixamo rig: each bone onto its namesake (their rests may differ).
pub const MIXAMO_TO_MIXAMO: [BonePair<'static>; 19] = [
    (
        "mixamorig:Hips",
        "mixamorig:Hips",
        Some(("mixamorig:Spine", "mixamorig:Spine")),
    ),
    (
        "mixamorig:Spine",
        "mixamorig:Spine",
        Some(("mixamorig:Spine1", "mixamorig:Spine1")),
    ),
    (
        "mixamorig:Spine1",
        "mixamorig:Spine1",
        Some(("mixamorig:Spine2", "mixamorig:Spine2")),
    ),
    (
        "mixamorig:Spine2",
        "mixamorig:Spine2",
        Some(("mixamorig:Neck", "mixamorig:Neck")),
    ),
    (
        "mixamorig:Neck",
        "mixamorig:Neck",
        Some(("mixamorig:Head", "mixamorig:Head")),
    ),
    ("mixamorig:Head", "mixamorig:Head", None),
    (
        "mixamorig:LeftShoulder",
        "mixamorig:LeftShoulder",
        Some(("mixamorig:LeftArm", "mixamorig:LeftArm")),
    ),
    (
        "mixamorig:LeftArm",
        "mixamorig:LeftArm",
        Some(("mixamorig:LeftForeArm", "mixamorig:LeftForeArm")),
    ),
    (
        "mixamorig:LeftForeArm",
        "mixamorig:LeftForeArm",
        Some(("mixamorig:LeftHand", "mixamorig:LeftHand")),
    ),
    ("mixamorig:LeftHand", "mixamorig:LeftHand", None),
    (
        "mixamorig:RightShoulder",
        "mixamorig:RightShoulder",
        Some(("mixamorig:RightArm", "mixamorig:RightArm")),
    ),
    (
        "mixamorig:RightArm",
        "mixamorig:RightArm",
        Some(("mixamorig:RightForeArm", "mixamorig:RightForeArm")),
    ),
    (
        "mixamorig:RightForeArm",
        "mixamorig:RightForeArm",
        Some(("mixamorig:RightHand", "mixamorig:RightHand")),
    ),
    ("mixamorig:RightHand", "mixamorig:RightHand", None),
    (
        "mixamorig:LeftUpLeg",
        "mixamorig:LeftUpLeg",
        Some(("mixamorig:LeftLeg", "mixamorig:LeftLeg")),
    ),
    (
        "mixamorig:LeftLeg",
        "mixamorig:LeftLeg",
        Some(("mixamorig:LeftFoot", "mixamorig:LeftFoot")),
    ),
    (
        "mixamorig:LeftFoot",
        "mixamorig:LeftFoot",
        Some(("mixamorig:LeftToeBase", "mixamorig:LeftToeBase")),
    ),
    (
        "mixamorig:RightUpLeg",
        "mixamorig:RightUpLeg",
        Some(("mixamorig:RightLeg", "mixamorig:RightLeg")),
    ),
    (
        "mixamorig:RightLeg",
        "mixamorig:RightLeg",
        Some(("mixamorig:RightFoot", "mixamorig:RightFoot")),
    ),
];

struct Bone {
    node: usize,
    src: String,
    rest_t: Quat,
    rest_s_inv: Quat,
    align: Quat,
}

/// Copies clip `src_clip` of `source` onto `target` as `name` (see the module notes); returns its
/// index. `frames` keys the clip.
pub fn retarget_clip(
    target: &mut GltfModel,
    source: &GltfModel,
    src_clip: usize,
    name: &str,
    map: &[BonePair],
    frames: usize,
) -> Result<usize, Render3DError> {
    let (duration, tracks) = retarget_tracks(target, source, src_clip, map, frames);
    target.add_animation_clip(name, duration, tracks, Vec::new())
}

/// The rotation tracks [`retarget_clip`] would add, and the clip's length (for a caller that
/// adds translations of its own, such as where the feet meet the floor).
pub fn retarget_tracks(
    target: &GltfModel,
    source: &GltfModel,
    src_clip: usize,
    map: &[BonePair],
    frames: usize,
) -> (f32, RotationTracks3D) {
    let duration = source
        .animation_clips
        .get(src_clip)
        .map(|c| c.duration)
        .unwrap_or(1.0)
        .max(0.05);
    let nodes = target.rest_nodes();
    let index_of = |n: &str| nodes.iter().position(|(name, ..)| name == n);
    let t_rest = target.rest_pose();
    let s_rest = source.rest_pose();
    let pos = |m: Mat4| [m.w_axis[0], m.w_axis[1], m.w_axis[2]];
    let mut bones = Vec::new();
    for (t, s, dirs) in map {
        let (Some(node), Some(mt), Some(ms)) = (
            index_of(t),
            target.pose_node_transform(&t_rest, t),
            source.pose_node_transform(&s_rest, s),
        ) else {
            continue;
        };
        let align = match dirs.and_then(|(tc, sc)| {
            Some((
                target.pose_node_transform(&t_rest, tc)?,
                source.pose_node_transform(&s_rest, sc)?,
            ))
        }) {
            Some((mtc, msc)) => {
                let dt = [
                    pos(mtc)[0] - pos(mt)[0],
                    pos(mtc)[1] - pos(mt)[1],
                    pos(mtc)[2] - pos(mt)[2],
                ];
                let ds = [
                    pos(msc)[0] - pos(ms)[0],
                    pos(msc)[1] - pos(ms)[1],
                    pos(msc)[2] - pos(ms)[2],
                ];
                q_between(dt, ds)
            }
            None => [0.0, 0.0, 0.0, 1.0],
        };
        bones.push(Bone {
            node,
            src: s.to_string(),
            rest_t: mat_rotation(mt),
            rest_s_inv: q_conj(mat_rotation(ms)),
            align,
        });
    }
    let n = nodes.len();
    let mut tracks: RotationTracks3D = bones
        .iter()
        .map(|b| (nodes[b.node].0.clone(), Vec::new()))
        .collect();
    for f in 0..=frames {
        let t = duration * f as f32 / frames as f32;
        let Some(src_pose) = source.pose(src_clip, t) else {
            continue;
        };
        // What each mapped bone's world rotation should be this frame.
        let mut wanted: Vec<Option<Quat>> = vec![None; n];
        for b in &bones {
            if let Some(ms) = source.pose_node_transform(&src_pose, &b.src) {
                let delta = q_mul(mat_rotation(ms), b.rest_s_inv);
                wanted[b.node] = Some(q_norm(q_mul(q_mul(delta, b.align), b.rest_t)));
            }
        }
        // The world rotation of every target node (unmapped ones ride along with their parent).
        let mut world: Vec<Option<Quat>> = vec![None; n];
        fn resolve(
            i: usize,
            nodes: &[RestNode],
            wanted: &[Option<Quat>],
            world: &mut Vec<Option<Quat>>,
        ) -> Quat {
            if let Some(q) = world[i] {
                return q;
            }
            let q = match wanted[i] {
                Some(w) => w,
                None => {
                    let parent = nodes[i]
                        .1
                        .map(|p| resolve(p, nodes, wanted, world))
                        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                    q_norm(q_mul(parent, nodes[i].3))
                }
            };
            world[i] = Some(q);
            q
        }
        for (k, b) in bones.iter().enumerate() {
            let w = resolve(b.node, &nodes, &wanted, &mut world);
            let parent = nodes[b.node]
                .1
                .map(|p| resolve(p, &nodes, &wanted, &mut world))
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            tracks[k].1.push((t, q_norm(q_mul(q_conj(parent), w))));
        }
    }
    (duration, tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Quat, b: Quat) -> bool {
        // q and -q are the same turn
        let d = a
            .iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0, f32::max);
        let e = a
            .iter()
            .zip(b)
            .map(|(x, y)| (x + y).abs())
            .fold(0.0, f32::max);
        d.min(e) < 1e-5
    }

    #[test]
    fn the_shortest_turn_takes_one_direction_onto_the_other() {
        let q = q_between([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert!(close(
            q,
            q_axis([0.0, 0.0, 1.0], std::f32::consts::FRAC_PI_2)
        ));
        // opposite directions still give a half turn
        let half = q_between([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
        assert!(half[3].abs() < 1e-6);
    }

    #[test]
    fn a_scaled_rotation_matrix_reads_back_as_its_turn() {
        let turn = q_axis([0.3, 1.0, -0.2], 1.1);
        let m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0))
            .mul_mat4(Mat4::from_quaternion(turn))
            .mul_mat4(Mat4::from_scale(Vec3::new(2.0, 2.0, 2.0)));
        assert!(close(mat_rotation(m), turn));
        assert!(close(q_mul(turn, q_conj(turn)), [0.0, 0.0, 0.0, 1.0]));
    }
}
