//! Clips from one rig onto another: the spa girl's Mixamo clips (sitting, lying) onto the Queen's
//! own 28-bone rig, and a Mixamo walk and the Queen's idle onto the brothel girls' `DEF-` rigs.
//!
//! Each mapped bone points the way its counterpart points, frame by frame: the counterpart's turn
//! away from its rest pose, applied to the target bone after lining the two rests' bone directions
//! up (`R_t = D_s · A · R_t,rest`, `D_s = R_s · R_s,rest⁻¹`, `A` the shortest turn from the target's
//! rest direction to the source's). Bones without a counterpart ride along with their parents. Only
//! rotations are copied: where the body sits is left to whoever places it.

use super::*;

pub(crate) type Quat = [f32; 4];

pub(crate) fn q_mul(a: Quat, b: Quat) -> Quat {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

pub(crate) fn q_conj(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

pub(crate) fn q_norm(q: Quat) -> Quat {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
        .sqrt()
        .max(1e-9);
    q.map(|v| v / l)
}

/// The rotation part of a global transform, as a quaternion (its columns normalised first: rigs
/// often carry a scale on their root).
pub(crate) fn mat_rotation(m: Mat4) -> Quat {
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
pub(crate) fn q_between(a: [f32; 3], b: [f32; 3]) -> Quat {
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

/// Which bone of a Mixamo rig drives which of the Queen's, and the child each points at.
pub(crate) const QUEEN_FROM_MIXAMO: [(&str, &str, Option<(&str, &str)>); 19] = [
    (
        "pelvis",
        "mixamorig:Hips",
        Some(("spine_lower", "mixamorig:Spine")),
    ),
    (
        "spine_lower",
        "mixamorig:Spine",
        Some(("spine_mid", "mixamorig:Spine1")),
    ),
    (
        "spine_mid",
        "mixamorig:Spine1",
        Some(("chest", "mixamorig:Spine2")),
    ),
    (
        "chest",
        "mixamorig:Spine2",
        Some(("neck", "mixamorig:Neck")),
    ),
    ("neck", "mixamorig:Neck", Some(("head", "mixamorig:Head"))),
    ("head", "mixamorig:Head", None),
    (
        "shoulder.L",
        "mixamorig:LeftShoulder",
        Some(("upper_arm.L", "mixamorig:LeftArm")),
    ),
    (
        "upper_arm.L",
        "mixamorig:LeftArm",
        Some(("forearm.L", "mixamorig:LeftForeArm")),
    ),
    (
        "forearm.L",
        "mixamorig:LeftForeArm",
        Some(("hand.L", "mixamorig:LeftHand")),
    ),
    ("hand.L", "mixamorig:LeftHand", None),
    (
        "shoulder.R",
        "mixamorig:RightShoulder",
        Some(("upper_arm.R", "mixamorig:RightArm")),
    ),
    (
        "upper_arm.R",
        "mixamorig:RightArm",
        Some(("forearm.R", "mixamorig:RightForeArm")),
    ),
    (
        "forearm.R",
        "mixamorig:RightForeArm",
        Some(("hand.R", "mixamorig:RightHand")),
    ),
    ("hand.R", "mixamorig:RightHand", None),
    (
        "thigh.L",
        "mixamorig:LeftUpLeg",
        Some(("shin.L", "mixamorig:LeftLeg")),
    ),
    (
        "shin.L",
        "mixamorig:LeftLeg",
        Some(("foot.L", "mixamorig:LeftFoot")),
    ),
    (
        "foot.L",
        "mixamorig:LeftFoot",
        Some(("toe.L", "mixamorig:LeftToeBase")),
    ),
    (
        "thigh.R",
        "mixamorig:RightUpLeg",
        Some(("shin.R", "mixamorig:RightLeg")),
    ),
    (
        "shin.R",
        "mixamorig:RightLeg",
        Some(("foot.R", "mixamorig:RightFoot")),
    ),
];

/// A Mixamo rig onto another Mixamo rig: each bone onto its namesake (their rests may differ).
pub(crate) const MIXAMO_TO_MIXAMO: [(&str, &str, Option<(&str, &str)>); 19] = [
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

pub(crate) fn q_axis(axis: [f32; 3], angle: f32) -> Quat {
    let l = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
        .sqrt()
        .max(1e-9);
    let (s, c) = (angle * 0.5).sin_cos();
    [axis[0] / l * s, axis[1] / l * s, axis[2] / l * s, c]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A calm standing loop made on a Mixamo rig from its T-pose rest (legs straight, back
/// straight): the arms let down along the body, the elbows a little bent forward, a slow breath
/// in the chest and the head following it. Added to `model` as `name` (4 s); `None` when the
/// rig is not Mixamo's.
pub(crate) fn add_mixamo_stand(model: &mut GltfModel, name: &str) -> Option<usize> {
    const FRAMES: usize = 32;
    const DURATION: f32 = 4.0;
    let rest = model.rest_pose();
    let nodes = model.rest_nodes();
    let index = |n: &str| nodes.iter().position(|x| x.0 == n);
    let world = |i: usize| {
        model
            .pose_node_transform(&rest, &nodes[i].0)
            .map(mat_rotation)
    };
    let at = |n: &str| {
        model
            .pose_node_transform(&rest, n)
            .map(|m| [m.w_axis[0], m.w_axis[1], m.w_axis[2]])
    };
    // Which way she faces (from the ankle to the toe), and her left.
    let (ankle, toe) = (at("mixamorig:LeftFoot")?, at("mixamorig:LeftToeBase")?);
    let fwd = {
        let f = [toe[0] - ankle[0], 0.0, toe[2] - ankle[2]];
        let l = f[0].hypot(f[2]).max(1e-6);
        [f[0] / l, 0.0, f[2] / l]
    };
    let up = [0.0, 1.0, 0.0];
    let left = cross(up, fwd);
    // A bone's local rotation once its parent is turned in the world by `parent_turn` and it by
    // `turn` (both world turns, the parent's included in `turn`).
    let local = |bone: &str, parent_turn: Quat, turn: Quat| -> Option<Quat> {
        let i = index(bone)?;
        let p = nodes[i].1?;
        let parent = q_mul(parent_turn, world(p)?);
        let own = q_mul(turn, world(i)?);
        Some(q_norm(q_mul(q_conj(parent), own)))
    };
    let none = [0.0, 0.0, 0.0, 1.0];
    // Arms down along the body (72 degrees from the T), elbows bent 20 degrees forward.
    let down_l = q_axis(fwd, -1.26);
    let down_r = q_axis(fwd, 1.26);
    let bend = q_axis([-left[0], -left[1], -left[2]], 0.35);
    let arm_l = local("mixamorig:LeftArm", none, down_l)?;
    let arm_r = local("mixamorig:RightArm", none, down_r)?;
    let fore_l = local("mixamorig:LeftForeArm", down_l, q_mul(bend, down_l))?;
    let fore_r = local("mixamorig:RightForeArm", down_r, q_mul(bend, down_r))?;
    let mut tracks: velvet_render::RotationTracks3D = vec![
        (
            "mixamorig:LeftArm".into(),
            vec![(0.0, arm_l), (DURATION, arm_l)],
        ),
        (
            "mixamorig:RightArm".into(),
            vec![(0.0, arm_r), (DURATION, arm_r)],
        ),
        (
            "mixamorig:LeftForeArm".into(),
            vec![(0.0, fore_l), (DURATION, fore_l)],
        ),
        (
            "mixamorig:RightForeArm".into(),
            vec![(0.0, fore_r), (DURATION, fore_r)],
        ),
    ];
    // The breath: the chest rises and the head nods with it, a little.
    let mut chest = Vec::new();
    let mut head = Vec::new();
    for f in 0..=FRAMES {
        let t = DURATION * f as f32 / FRAMES as f32;
        let breath = (t / DURATION * std::f32::consts::TAU).sin();
        let lean = q_axis(left, -0.022 * breath);
        chest.push((t, local("mixamorig:Spine2", none, lean)?));
        let nod = q_axis(left, 0.03 * breath);
        head.push((t, local("mixamorig:Head", lean, q_mul(nod, lean))?));
    }
    tracks.push(("mixamorig:Spine2".into(), chest));
    tracks.push(("mixamorig:Head".into(), head));
    model
        .add_animation_clip(name, DURATION, tracks, Vec::new())
        .ok()
}

/// The brothel girls' rigs (their Mixamo skeletons renamed `DEF-*` by the prototype's
/// `rig_mixamo.py`) driven by a Mixamo rig.
pub(crate) const DEF_FROM_MIXAMO: [(&str, &str, Option<(&str, &str)>); 20] = [
    (
        "DEF-pelvis",
        "mixamorig:Hips",
        Some(("DEF-spine", "mixamorig:Spine")),
    ),
    (
        "DEF-spine",
        "mixamorig:Spine",
        Some(("DEF-spine1", "mixamorig:Spine1")),
    ),
    (
        "DEF-spine1",
        "mixamorig:Spine1",
        Some(("DEF-chest", "mixamorig:Spine2")),
    ),
    (
        "DEF-chest",
        "mixamorig:Spine2",
        Some(("DEF-neck", "mixamorig:Neck")),
    ),
    (
        "DEF-neck",
        "mixamorig:Neck",
        Some(("DEF-head", "mixamorig:Head")),
    ),
    ("DEF-head", "mixamorig:Head", None),
    (
        "DEF-shoulderL",
        "mixamorig:LeftShoulder",
        Some(("DEF-upper_armL", "mixamorig:LeftArm")),
    ),
    (
        "DEF-upper_armL",
        "mixamorig:LeftArm",
        Some(("DEF-forearmL", "mixamorig:LeftForeArm")),
    ),
    (
        "DEF-forearmL",
        "mixamorig:LeftForeArm",
        Some(("DEF-handL", "mixamorig:LeftHand")),
    ),
    ("DEF-handL", "mixamorig:LeftHand", None),
    (
        "DEF-shoulderR",
        "mixamorig:RightShoulder",
        Some(("DEF-upper_armR", "mixamorig:RightArm")),
    ),
    (
        "DEF-upper_armR",
        "mixamorig:RightArm",
        Some(("DEF-forearmR", "mixamorig:RightForeArm")),
    ),
    (
        "DEF-forearmR",
        "mixamorig:RightForeArm",
        Some(("DEF-handR", "mixamorig:RightHand")),
    ),
    ("DEF-handR", "mixamorig:RightHand", None),
    (
        "DEF-thighL",
        "mixamorig:LeftUpLeg",
        Some(("DEF-shinL", "mixamorig:LeftLeg")),
    ),
    (
        "DEF-shinL",
        "mixamorig:LeftLeg",
        Some(("DEF-footL", "mixamorig:LeftFoot")),
    ),
    (
        "DEF-footL",
        "mixamorig:LeftFoot",
        Some(("DEF-toeL", "mixamorig:LeftToeBase")),
    ),
    (
        "DEF-thighR",
        "mixamorig:RightUpLeg",
        Some(("DEF-shinR", "mixamorig:RightLeg")),
    ),
    (
        "DEF-shinR",
        "mixamorig:RightLeg",
        Some(("DEF-footR", "mixamorig:RightFoot")),
    ),
    (
        "DEF-footR",
        "mixamorig:RightFoot",
        Some(("DEF-toeR", "mixamorig:RightToeBase")),
    ),
];

/// The brothel girls' rigs driven by the Queen's authored rig (`queen_chibi_v2_animated.glb`).
pub(crate) const DEF_FROM_AUTHORED: [(&str, &str, Option<(&str, &str)>); 20] = [
    ("DEF-pelvis", "pelvis", Some(("DEF-spine", "spine_lower"))),
    (
        "DEF-spine",
        "spine_lower",
        Some(("DEF-spine1", "spine_mid")),
    ),
    ("DEF-spine1", "spine_mid", Some(("DEF-chest", "chest"))),
    ("DEF-chest", "chest", Some(("DEF-neck", "neck"))),
    ("DEF-neck", "neck", Some(("DEF-head", "head"))),
    ("DEF-head", "head", None),
    (
        "DEF-shoulderL",
        "shoulder.L",
        Some(("DEF-upper_armL", "upper_arm.L")),
    ),
    (
        "DEF-upper_armL",
        "upper_arm.L",
        Some(("DEF-forearmL", "forearm.L")),
    ),
    ("DEF-forearmL", "forearm.L", Some(("DEF-handL", "hand.L"))),
    ("DEF-handL", "hand.L", None),
    (
        "DEF-shoulderR",
        "shoulder.R",
        Some(("DEF-upper_armR", "upper_arm.R")),
    ),
    (
        "DEF-upper_armR",
        "upper_arm.R",
        Some(("DEF-forearmR", "forearm.R")),
    ),
    ("DEF-forearmR", "forearm.R", Some(("DEF-handR", "hand.R"))),
    ("DEF-handR", "hand.R", None),
    ("DEF-thighL", "thigh.L", Some(("DEF-shinL", "shin.L"))),
    ("DEF-shinL", "shin.L", Some(("DEF-footL", "foot.L"))),
    ("DEF-footL", "foot.L", Some(("DEF-toeL", "toe.L"))),
    ("DEF-thighR", "thigh.R", Some(("DEF-shinR", "shin.R"))),
    ("DEF-shinR", "shin.R", Some(("DEF-footR", "foot.R"))),
    ("DEF-footR", "foot.R", Some(("DEF-toeR", "toe.R"))),
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
pub(crate) fn retarget_clip(
    target: &mut GltfModel,
    source: &GltfModel,
    src_clip: usize,
    name: &str,
    map: &[(&str, &str, Option<(&str, &str)>)],
    frames: usize,
) -> Result<usize> {
    let (duration, tracks) = retarget_tracks(target, source, src_clip, map, frames);
    Ok(target.add_animation_clip(name, duration, tracks, Vec::new())?)
}

/// The rotation tracks [`retarget_clip`] would add, and the clip's length (for a caller that
/// adds translations of its own, such as where the feet meet the floor).
pub(crate) fn retarget_tracks(
    target: &GltfModel,
    source: &GltfModel,
    src_clip: usize,
    map: &[(&str, &str, Option<(&str, &str)>)],
    frames: usize,
) -> (f32, velvet_render::RotationTracks3D) {
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
    let mut tracks: Vec<(String, Vec<(f32, [f32; 4])>)> = bones
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
            nodes: &[(String, Option<usize>, Vec3, [f32; 4])],
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

/// The Queen's relaxing clips, borrowed from the spa girl, and where her body is in them (scene
/// units, before her world scale): her chest seated, her back lying, which way her head lies.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RelaxClips {
    pub sit: Option<usize>,
    pub lie: Option<usize>,
    pub sit_chest: f32,
    pub lie_low: f32,
    /// Rosalía's elixir going down (`elixir::CLIP`, made on her rig, not borrowed).
    pub drink: Option<usize>,
}

/// Adds "Queen_Sit" and "Queen_Lie" to a Queen model from spa_girl.glb.
pub(crate) fn add_relax_clips(queen: &mut GltfModel) -> RelaxClips {
    let path = runtime_3d_root().join("baths").join("spa_girl.glb");
    let Ok(girl) = GltfModel::load(&path) else {
        return RelaxClips::default();
    };
    // The game's Queens are Mixamo rigs; the authored one has its own bone names.
    let mixamo = queen
        .rest_nodes()
        .iter()
        .any(|(n, ..)| n == "mixamorig:Hips");
    let map: &[(&str, &str, Option<(&str, &str)>)] = if mixamo {
        &MIXAMO_TO_MIXAMO
    } else {
        &QUEEN_FROM_MIXAMO
    };
    let (chest, low): (&str, [&str; 5]) = if mixamo {
        (
            "mixamorig:Spine2",
            [
                "mixamorig:Hips",
                "mixamorig:Spine",
                "mixamorig:Spine1",
                "mixamorig:Spine2",
                "mixamorig:Head",
            ],
        )
    } else {
        (
            "chest",
            ["pelvis", "spine_lower", "spine_mid", "chest", "head"],
        )
    };
    let clip = |queen: &mut GltfModel, src: &str, name: &str| {
        girl.animation_index(src)
            .and_then(|c| retarget_clip(queen, &girl, c, name, map, 32).ok())
    };
    let sit = clip(queen, "Spa_Sit", "Queen_Sit");
    let lie = clip(queen, "Spa_Lie", "Queen_Lie");
    let at = |queen: &GltfModel, clip: Option<usize>, bone: &str| -> Option<Vec3> {
        let pose = queen.pose(clip?, 0.0)?;
        let m = queen.pose_node_transform(&pose, bone)?;
        Some(Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2]))
    };
    let sit_chest = at(queen, sit, chest).map_or(0.6, |p| p.y);
    let lie_low = low
        .iter()
        .filter_map(|b| at(queen, lie, b))
        .map(|p| p.y)
        .fold(f32::INFINITY, f32::min);
    eprintln!(
        "[3d] queen relax clips: sit={sit:?} lie={lie:?} chest {sit_chest:.2} low {lie_low:.2}"
    );
    RelaxClips {
        sit,
        lie,
        sit_chest,
        lie_low: if lie_low.is_finite() { lie_low } else { 0.2 },
        drink: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_queen_sits_like_the_spa_girl() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/assets/3d");
        let (Ok(mut queen), Ok(girl)) = (
            GltfModel::load(root.join("characters/queen_chibi_v2_animated.glb")),
            GltfModel::load(root.join("baths/spa_girl.glb")),
        ) else {
            return;
        };
        let sit = girl.animation_index("Spa_Sit").unwrap();
        let clip =
            retarget_clip(&mut queen, &girl, sit, "Queen_Sit", &QUEEN_FROM_MIXAMO, 24).unwrap();
        let pose = queen.pose(clip, 0.5).unwrap();
        let rest = queen.rest_pose();
        let at = |pose: &velvet_render::Pose3D, n: &str| {
            let m = queen.pose_node_transform(pose, n).unwrap();
            Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2])
        };
        // Seated: the thighs go from hanging down to near level, the knees come forward of the
        // hips, the feet stay under the knees, and the head stays over the chest.
        let drop_rest = at(&rest, "thigh.L").y - at(&rest, "shin.L").y;
        let drop = at(&pose, "thigh.L").y - at(&pose, "shin.L").y;
        assert!(
            drop < drop_rest * 0.6,
            "thigh near level: {drop} vs {drop_rest}"
        );
        assert!(
            (at(&pose, "shin.L") - at(&pose, "thigh.L")).z.abs() > 0.05,
            "knee out from the hip"
        );
        assert!(
            at(&pose, "foot.L").y < at(&pose, "shin.L").y,
            "foot under the knee"
        );
        assert!(at(&pose, "head").y > at(&pose, "chest").y);
    }
}
