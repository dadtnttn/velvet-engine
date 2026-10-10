//! A walk made by hand for a Mixamo rig that came without one.
//!
//! Each bone is given a rotation in the model's own space, applied on top of its parent's
//! (`R_bone = D_bone · R_parent`), and turned back into the local rotations a clip stores
//! (`L' = (R_p G_p)⁻¹ · R_b G_b`, with G the rest pose's global rotations). Bones left alone ride
//! along with their parents. The arms come down from the T-pose and swing against the legs; the
//! knees bend through the swing; the spine twists a little and the hips the other way.

use super::*;

type Quat = [f32; 4];

fn q_mul(a: Quat, b: Quat) -> Quat {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn q_conj(q: Quat) -> Quat {
    [-q[0], -q[1], -q[2], q[3]]
}

fn q_axis(axis: [f32; 3], angle: f32) -> Quat {
    let (s, c) = (angle * 0.5).sin_cos();
    [axis[0] * s, axis[1] * s, axis[2] * s, c]
}

fn q_norm(q: Quat) -> Quat {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3])
        .sqrt()
        .max(1e-9);
    q.map(|v| v / l)
}

const X: [f32; 3] = [1.0, 0.0, 0.0];
const Y: [f32; 3] = [0.0, 1.0, 0.0];
const Z: [f32; 3] = [0.0, 0.0, 1.0];

/// How a walk moves.
#[derive(Debug, Clone, Copy)]
pub(crate) struct GaitStyle {
    /// Thigh swing either way (radians).
    pub stride: f32,
    /// How far the knee folds through the swing.
    pub knee: f32,
    /// The arms' swing, and how far they come down from the T-pose.
    pub arm_swing: f32,
    pub arm_down: f32,
    /// The elbows' constant bend.
    pub elbow: f32,
    /// The spine's twist.
    pub twist: f32,
}

impl Default for GaitStyle {
    fn default() -> Self {
        Self {
            stride: 0.42,
            knee: 0.75,
            arm_swing: 0.32,
            arm_down: 1.22,
            elbow: 0.3,
            twist: 0.08,
        }
    }
}

/// The model-space rotation (applied over the parent's) for a bone at phase `phi` (0..2π, the
/// left foot forward at π/2).
fn delta(name: &str, phi: f32, s: &GaitStyle) -> Option<Quat> {
    let bone = name.rsplit(':').next().unwrap_or(name);
    let swing = phi.sin();
    // The knee folds while its leg comes forward (the thigh's swing rising).
    let fold = |rising: f32| 0.12 + s.knee * rising.max(0.0).powf(1.4);
    Some(match bone {
        "Hips" => q_axis(Y, -s.twist * 0.6 * swing),
        "Spine" => q_axis(Y, s.twist * swing),
        "LeftUpLeg" => q_axis(X, -s.stride * swing),
        "RightUpLeg" => q_axis(X, s.stride * swing),
        "LeftLeg" => q_axis(X, fold(phi.cos())),
        "RightLeg" => q_axis(X, fold(-phi.cos())),
        // Feet stay level with the ground: undo most of the leg's tilt.
        "LeftFoot" => q_axis(X, s.stride * swing * 0.5 - fold(phi.cos()) * 0.6),
        "RightFoot" => q_axis(X, -s.stride * swing * 0.5 - fold(-phi.cos()) * 0.6),
        // Down from the T-pose, then swinging against the legs.
        "LeftArm" => q_mul(q_axis(X, s.arm_swing * swing), q_axis(Z, -s.arm_down)),
        "RightArm" => q_mul(q_axis(X, -s.arm_swing * swing), q_axis(Z, s.arm_down)),
        "LeftForeArm" => q_axis(X, -s.elbow - 0.15 * swing.max(0.0)),
        "RightForeArm" => q_axis(X, -s.elbow - 0.15 * (-swing).max(0.0)),
        _ => return None,
    })
}

/// Adds a walk loop of `duration` seconds named `name` to a Mixamo-rigged model; returns its
/// index. `frames` keys a cycle (more is smoother).
pub(crate) fn add_walk_clip(
    model: &mut GltfModel,
    name: &str,
    duration: f32,
    style: &GaitStyle,
    frames: usize,
) -> Result<usize> {
    let nodes = model.rest_nodes();
    let n = nodes.len();
    // Rest global rotations.
    fn global(
        i: usize,
        nodes: &[(String, Option<usize>, Vec3, [f32; 4])],
        memo: &mut Vec<Option<Quat>>,
    ) -> Quat {
        if let Some(q) = memo[i] {
            return q;
        }
        let local = nodes[i].3;
        let q = match nodes[i].1 {
            Some(p) => q_norm(q_mul(global(p, nodes, memo), local)),
            None => local,
        };
        memo[i] = Some(q);
        q
    }
    let mut memo = vec![None; n];
    let rest: Vec<Quat> = (0..n).map(|i| global(i, &nodes, &mut memo)).collect();
    // Which bones carry keys: every bone with a delta, and their children (whose locals change
    // when a parent's delta differs from theirs... they do not: L' = conj(R_p G_p) R_p G_b = L).
    let keyed: Vec<usize> = (0..n)
        .filter(|&i| delta(&nodes[i].0, 0.0, style).is_some())
        .collect();
    let mut tracks: Vec<(String, Vec<(f32, [f32; 4])>)> = keyed
        .iter()
        .map(|&i| (nodes[i].0.clone(), Vec::new()))
        .collect();
    for f in 0..=frames {
        let t = f as f32 / frames as f32;
        let phi = t * std::f32::consts::TAU;
        // R per bone, root first (memoised over the parent chain).
        let mut r: Vec<Option<Quat>> = vec![None; n];
        fn world(
            i: usize,
            nodes: &[(String, Option<usize>, Vec3, [f32; 4])],
            phi: f32,
            style: &GaitStyle,
            r: &mut Vec<Option<Quat>>,
        ) -> Quat {
            if let Some(q) = r[i] {
                return q;
            }
            let parent = nodes[i]
                .1
                .map(|p| world(p, nodes, phi, style, r))
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
            let q = match delta(&nodes[i].0, phi, style) {
                Some(d) => q_norm(q_mul(d, parent)),
                None => parent,
            };
            r[i] = Some(q);
            q
        }
        for (k, &i) in keyed.iter().enumerate() {
            let rb = world(i, &nodes, phi, style, &mut r);
            let (rp, gp) = match nodes[i].1 {
                Some(p) => (world(p, &nodes, phi, style, &mut r), rest[p]),
                None => ([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]),
            };
            let local = q_norm(q_mul(q_conj(q_mul(rp, gp)), q_mul(rb, rest[i])));
            tracks[k].1.push((t * duration, local));
        }
    }
    Ok(model.add_animation_clip(name, duration, tracks, Vec::new())?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone_pos(model: &GltfModel, pose: &velvet_render::Pose3D, name: &str) -> Vec3 {
        let m = model
            .pose_node_transform(pose, name)
            .unwrap_or_else(|| panic!("{name}"));
        Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2])
    }

    #[test]
    fn the_spa_girl_walks_arms_down_legs_swinging() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/assets/3d/baths/spa_girl.glb");
        if !path.exists() {
            return;
        }
        let mut model = GltfModel::load(&path).unwrap();
        let clip = add_walk_clip(&mut model, "Spa_Walk", 1.0, &GaitStyle::default(), 16).unwrap();
        let rest = model.rest_pose();
        // Which way she faces: her toes ahead of her ankles.
        let toe = bone_pos(&model, &rest, "mixamorig:LeftToeBase");
        let foot = bone_pos(&model, &rest, "mixamorig:LeftFoot");
        let forward = Vec3::new(toe.x - foot.x, 0.0, toe.z - foot.z).normalize_or_zero();
        let height = bone_pos(&model, &rest, "mixamorig:Head").y - foot.y;
        // A quarter cycle in: the left foot ahead, the right behind.
        let pose = model.pose(clip, 0.25).unwrap();
        let lf = bone_pos(&model, &pose, "mixamorig:LeftFoot");
        let rf = bone_pos(&model, &pose, "mixamorig:RightFoot");
        assert!(
            (lf - rf).dot(forward) > 0.05 * height,
            "left foot forward: {:?} {:?}",
            lf,
            rf
        );
        // The arms hang: hands well under the shoulders.
        for side in ["Left", "Right"] {
            let hand = bone_pos(&model, &pose, &format!("mixamorig:{side}Hand"));
            let shoulder = bone_pos(&model, &pose, &format!("mixamorig:{side}Arm"));
            assert!(
                shoulder.y - hand.y > 0.12 * height,
                "{side} hand down: {} vs {}",
                hand.y,
                shoulder.y
            );
        }
        // Half a cycle later the other foot leads.
        let pose = model.pose(clip, 0.75).unwrap();
        let lf = bone_pos(&model, &pose, "mixamorig:LeftFoot");
        let rf = bone_pos(&model, &pose, "mixamorig:RightFoot");
        assert!((rf - lf).dot(forward) > 0.05 * height);
        // The loop closes.
        let a = model
            .joint_matrices(0, &model.pose(clip, 0.0).unwrap())
            .unwrap();
        let b = model
            .joint_matrices(0, &model.pose(clip, 0.999).unwrap())
            .unwrap();
        let worst = a
            .iter()
            .zip(&b)
            .map(|(p, q)| {
                (p.transform_point3(Vec3::ZERO) - q.transform_point3(Vec3::ZERO)).length()
            })
            .fold(0.0, f32::max);
        assert!(worst < 0.02 * height / 0.01, "seamless loop: {worst}");
    }
}
