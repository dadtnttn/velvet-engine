//! El elixir de Rosalía, bebido en 3D. `Queen_Drink` es un clip hecho aquí sobre el esqueleto
//! Mixamo de la Reina (no hay animación descargada): saca el frasco de la cadera, lo mira, le salta
//! el corcho, se lo lleva a los labios (una IK de dos huesos, el codo abajo y hacia fuera), echa la
//! cabeza atrás, tres tragos, «¡ah!», y lo baja. El frasco es el de la casa (un corazón de cristal
//! con su lazo, su collar de cuernos y su tapón de oro; `tools/elixir_rosalia_blender.py`), en la
//! palma de su mano derecha, con la cara del corazón vuelta hacia la cámara; se vacía con cada
//! trago. La cámara se acerca en perspectiva para
//! el plano; lo que va encima (la cortinilla, los «glup», el sello que despierta) lo pinta el
//! overlay con los mismos tiempos.
use std::f32::consts::TAU;

use super::retarget::{mat_rotation, q_axis, q_between, q_conj, q_mul, q_norm, Quat};
use super::*;

/// The clip's name on each of her models.
pub(crate) const CLIP: &str = "Queen_Drink";
/// From the hip and back to it (s).
pub(crate) const SECS: f32 = 4.4;
/// The cork pops (s into the clip).
pub(crate) const POP: f32 = 1.0;
/// The three gulps.
pub(crate) const GULPS: [f32; 3] = [2.05, 2.45, 2.85];
/// «¡Ah!»: the bottle comes away from her lips.
pub(crate) const AH: f32 = 3.2;

/// The flask: the house's crystal heart (made by `tools/elixir_rosalia_blender.py`).
pub(crate) const BOTTLE: &str = "items/elixir_rosalia.glb";
/// Its size in her hand, her model's units per the flask's: 0.83 tall becomes a little over half
/// her head, the heart about as wide as her hand is long.
pub(crate) const BOTTLE_SCALE: f32 = 0.15;
/// Up the flask's axis, in its own units: where her palm holds it (the heart's middle), the lip of
/// its neck, and the floor of its liquid.
const GRIP_Y: f32 = 0.25;
const LIP_Y: f32 = 0.665;
const LIQUID_FLOOR_Y: f32 = 0.087;
/// Which way the heart's face turns in her model: toward the close shot, at her front-left.
const FACE_TOWARD: [f32; 3] = [0.6, 0.0, 0.8];
/// From her wrist to the middle of her palm, along the hand (her model's units).
const PALM: f32 = 0.034;
/// Her lips from her head bone in the rest pose (measured on the mesh: the chin at 0.665, the face's
/// front at 0.115, the head bone at 0.713).
const LIPS_FROM_HEAD: [f32; 3] = [0.0, -0.008, 0.112];

const HIPS: &str = "mixamorig:Hips";
const SPINE2: &str = "mixamorig:Spine2";
const NECK: &str = "mixamorig:Neck";
const HEAD: &str = "mixamorig:Head";
const SHOULDER: &str = "mixamorig:RightShoulder";
const ARM: &str = "mixamorig:RightArm";
const FOREARM: &str = "mixamorig:RightForeArm";
/// The bone the flask rides on.
pub(crate) const HAND: &str = "mixamorig:RightHand";

/// Where her right hand is going (her model's space: she faces +Z, her left is +X).
#[derive(Clone, Copy)]
enum Grip {
    /// Where it hangs as she stands.
    Hip,
    /// A point in front of her.
    At([f32; 3]),
    /// This far from her lips.
    Lips([f32; 3]),
    /// The flask's lip on her lips.
    Mouth,
}

/// One moment of the clip.
#[derive(Clone, Copy)]
struct Key {
    t: f32,
    grip: Grip,
    /// The flask's axis, base to neck.
    axis: [f32; 3],
    /// Head thrown back (0..1) and bowed toward the flask in her hand (0..1).
    back: f32,
    look: f32,
}

/// Bottom up and neck to her lips, about 20° past level, coming in from her right (the hand that
/// holds it) so the close shot sees the heart from the side, not end on.
const DRINK_AXIS: [f32; 3] = [0.45, -0.35, -0.82];
const UPRIGHT: [f32; 3] = [0.0, 1.0, 0.0];

const KEYS: [Key; 10] = [
    Key {
        t: 0.0,
        grip: Grip::Hip,
        axis: UPRIGHT,
        back: 0.0,
        look: 0.0,
    },
    Key {
        t: 0.3,
        grip: Grip::Hip,
        axis: UPRIGHT,
        back: 0.0,
        look: 0.0,
    },
    // Up before her chest, and a look at it.
    Key {
        t: 0.78,
        grip: Grip::At([-0.075, 0.555, 0.15]),
        axis: UPRIGHT,
        back: 0.0,
        look: 1.0,
    },
    Key {
        t: 1.12,
        grip: Grip::At([-0.065, 0.575, 0.16]),
        axis: [0.08, 1.0, 0.05],
        back: 0.0,
        look: 1.0,
    },
    // Under her chin, tipping.
    Key {
        t: 1.45,
        grip: Grip::Lips([-0.03, -0.06, 0.075]),
        axis: [0.3, 0.86, -0.42],
        back: 0.25,
        look: 0.0,
    },
    Key {
        t: 1.75,
        grip: Grip::Mouth,
        axis: DRINK_AXIS,
        back: 1.0,
        look: 0.0,
    },
    Key {
        t: 2.95,
        grip: Grip::Mouth,
        axis: DRINK_AXIS,
        back: 1.0,
        look: 0.0,
    },
    // Away: «¡ah!».
    Key {
        t: AH + 0.1,
        grip: Grip::At([-0.07, 0.565, 0.165]),
        axis: UPRIGHT,
        back: 0.0,
        look: 0.0,
    },
    Key {
        t: 3.95,
        grip: Grip::Hip,
        axis: UPRIGHT,
        back: 0.0,
        look: 0.0,
    },
    Key {
        t: SECS,
        grip: Grip::Hip,
        axis: UPRIGHT,
        back: 0.0,
        look: 0.0,
    },
];

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::new(a[0], a[1], a[2])
}

fn arr(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

fn pos(m: Mat4) -> Vec3 {
    Vec3::new(m.w_axis[0], m.w_axis[1], m.w_axis[2])
}

fn rotate(q: Quat, v: Vec3) -> Vec3 {
    Mat4::from_quaternion(q).transform_vector3(v)
}

/// Along the short way from `a` to `b`.
fn nlerp(a: Quat, b: Quat, t: f32) -> Quat {
    let d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let s = if d < 0.0 { -1.0 } else { 1.0 };
    q_norm([
        a[0] + (b[0] * s - a[0]) * t,
        a[1] + (b[1] * s - a[1]) * t,
        a[2] + (b[2] * s - a[2]) * t,
        a[3] + (b[3] * s - a[3]) * t,
    ])
}

/// The turn whose axes are `x`, `y` and `z` (orthonormal, right-handed).
fn basis(x: Vec3, y: Vec3, z: Vec3) -> Quat {
    mat_rotation(Mat4::from_cols(
        [x.x, x.y, x.z, 0.0],
        [y.x, y.y, y.z, 0.0],
        [z.x, z.y, z.z, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ))
}

/// The two keys around `t` and how far between them (eased).
fn span(t: f32) -> (Key, Key, f32) {
    let t = t.clamp(0.0, SECS);
    for pair in KEYS.windows(2) {
        if t <= pair[1].t {
            let u = (t - pair[0].t) / (pair[1].t - pair[0].t).max(1e-4);
            return (pair[0], pair[1], smooth(u));
        }
    }
    let last = KEYS[KEYS.len() - 1];
    (last, last, 1.0)
}

/// The gulps, as bumps 0..1.
fn gulp(t: f32) -> f32 {
    GULPS
        .iter()
        .map(|g| (-((t - g) / 0.09).powi(2)).exp())
        .sum::<f32>()
        .min(1.0)
}

/// How much her arm follows the clip rather than the way she stands, 0..1.
fn reach(t: f32) -> f32 {
    smooth((t - 0.3) / 0.35) * (1.0 - smooth((t - 3.55) / 0.4))
}

/// The flask's axis (base to neck) in her model at `t`: upright at the hip, bottom up at her lips,
/// tipping a little further with each gulp.
pub(crate) fn axis(t: f32) -> Vec3 {
    let (a, b, u) = span(t);
    let v = v3(a.axis)
        .normalize_or_zero()
        .lerp(v3(b.axis).normalize_or_zero(), u);
    (v + Vec3::new(0.0, -0.12, 0.0) * gulp(t)).normalize_or_zero()
}

/// What is left in the flask, 1 full to 0: a third goes with each gulp.
pub(crate) fn liquid(t: f32) -> f32 {
    let drunk: f32 = GULPS.iter().map(|g| smooth((t - g + 0.1) / 0.32)).sum();
    (1.0 - drunk / 3.0).clamp(0.0, 1.0)
}

/// The cork is still in.
pub(crate) fn corked(t: f32) -> bool {
    t < POP
}

/// Her head and chest at `t`: pitch (+ bows, − throws it back), yaw (− to her right) and how far
/// her chest leans back (radians).
struct Lean {
    pitch: f32,
    yaw: f32,
    chest: f32,
}

fn lean(t: f32) -> Lean {
    let (a, b, u) = span(t);
    let back = a.back + (b.back - a.back) * u;
    let look = a.look + (b.look - a.look) * u;
    let after = t - AH;
    let shake = if after > 0.0 {
        0.09 * (after * 3.2 * TAU).sin() * (-after * 3.5).exp()
    } else {
        0.0
    };
    Lean {
        pitch: -back * 0.34 - gulp(t) * 0.06 + look * 0.22,
        yaw: -look * 0.3 + shake,
        chest: back * 0.09,
    }
}

/// Where her palm should be at `t`: `hip` where it hangs as she stands, `lips` her lips now.
fn grip_target(t: f32, hip: Vec3, lips: Vec3) -> Vec3 {
    let (a, b, u) = span(t);
    let at = |k: &Key| match k.grip {
        Grip::Hip => hip,
        Grip::At(p) => v3(p),
        Grip::Lips(o) => lips + v3(o),
        // The flask's lip on hers, a touch to her right of centre.
        Grip::Mouth => {
            lips - v3(k.axis).normalize_or_zero() * ((LIP_Y - GRIP_Y) * BOTTLE_SCALE)
                + Vec3::new(-0.006, 0.0, 0.0)
        }
    };
    at(&a).lerp(at(&b), u)
}

/// Her arm to `target`: the shoulder at `s`, the elbow at `e0` and the wrist at `w0` as they are,
/// the palm carried on past the wrist in line with the forearm (the hand straightened to its rest
/// turn, `hand_local`). The elbow bends the way it already bends (its hinge carried over), pointing
/// down and out. Returns the upper arm's, the forearm's and the hand's turns in her model.
fn solve(
    s: Vec3,
    e0: Vec3,
    w0: Vec3,
    q_ua: Quat,
    q_fa: Quat,
    hand_local: Quat,
    target: Vec3,
) -> (Quat, Quat, Quat) {
    let a = (e0 - s).length().max(1e-4);
    let b0 = (w0 - e0).length().max(1e-4);
    let b = b0 + PALM;
    let u0 = (e0 - s) * (1.0 / a);
    let f0 = (w0 - e0) * (1.0 / b0);
    let h0 = {
        let h = u0.cross(f0);
        if h.length() > 1e-3 {
            h.normalize_or_zero()
        } else {
            u0.cross(Vec3::new(0.0, 0.0, 1.0)).normalize_or_zero()
        }
    };
    let d = target - s;
    let dist = d.length().max(1e-4);
    let dir = d * (1.0 / dist);
    let l = dist.clamp((a - b).abs() + 1e-3, a + b - 1e-3);
    let pole = Vec3::new(-0.55, -1.0, 0.2).normalize_or_zero();
    let n = (pole - dir * pole.dot(dir)).normalize_or_zero();
    let cos_a = ((a * a + l * l - b * b) / (2.0 * a * l)).clamp(-1.0, 1.0);
    let sin_a = (1.0 - cos_a * cos_a).max(0.0).sqrt();
    let e1 = s + dir * (a * cos_a) + n * (a * sin_a);
    let u1 = (e1 - s) * (1.0 / a);
    let f1 = (s + dir * l - e1).normalize_or_zero();
    let h1 = {
        let h = u1.cross(f1);
        if h.length() > 1e-3 {
            h.normalize_or_zero()
        } else {
            n.cross(dir).normalize_or_zero()
        }
    };
    let turn = q_mul(
        basis(u1, h1, u1.cross(h1)),
        q_conj(basis(u0, h0, u0.cross(h0))),
    );
    let ua = q_norm(q_mul(turn, q_ua));
    let carried = rotate(turn, f0);
    let fa = q_norm(q_mul(q_between(arr(carried), arr(f1)), q_mul(turn, q_fa)));
    (ua, fa, q_norm(q_mul(fa, hand_local)))
}

/// Adds [`CLIP`] to one of her models (a Mixamo rig), beginning and ending on the pose she stands
/// in: `stand` is that clip and whether it moves (an idle) or holds its first frame (the walk's).
/// `None` when the rig lacks the bones.
pub(crate) fn add_clip(model: &mut GltfModel, stand: Option<(usize, bool)>) -> Option<usize> {
    const FRAMES: usize = 66;
    let (stand_clip, animated) = stand?;
    let nodes = model.rest_nodes();
    let index = |n: &str| nodes.iter().position(|x| x.0 == n);
    let (hips, spine, neck, head, arm, forearm, hand) = (
        index(HIPS)?,
        index(SPINE2)?,
        index(NECK)?,
        index(HEAD)?,
        index(ARM)?,
        index(FOREARM)?,
        index(HAND)?,
    );
    index(SHOULDER)?;
    let skeleton: Vec<usize> = (0..nodes.len())
        .filter(|&i| nodes[i].0.starts_with("mixamorig:"))
        .collect();
    let rest = model.rest_pose();
    let head_rest = model.pose_node_transform(&rest, HEAD)?;
    let lips_local = head_rest
        .inverse()?
        .transform_point3(pos(head_rest) + v3(LIPS_FROM_HEAD));
    let hand_local = nodes[hand].3;
    let left = [1.0, 0.0, 0.0];
    let up = [0.0, 1.0, 0.0];
    let times: Vec<f32> = (0..=FRAMES)
        .map(|f| SECS * f as f32 / FRAMES as f32)
        .collect();

    // First the way she stands, her chest and her head.
    let mut tracks: Vec<(String, Vec<(f32, Quat)>)> = skeleton
        .iter()
        .map(|&i| (nodes[i].0.clone(), Vec::with_capacity(times.len())))
        .collect();
    let mut hips_track = Vec::with_capacity(times.len());
    for &t in &times {
        let base = model.pose(stand_clip, if animated { t } else { 0.0 })?;
        let world: Vec<Option<Quat>> = nodes
            .iter()
            .map(|n| model.pose_node_transform(&base, &n.0).map(mat_rotation))
            .collect();
        let local = |i: usize| -> Option<Quat> {
            let w = world[i]?;
            Some(match nodes[i].1 {
                Some(p) => q_norm(q_mul(q_conj(world[p]?), w)),
                None => w,
            })
        };
        let l = lean(t);
        let chest = q_axis(left, -l.chest);
        let neck_turn = q_mul(q_axis(up, l.yaw * 0.45), q_axis(left, l.pitch * 0.5));
        let head_turn = q_mul(q_axis(up, l.yaw * 0.55), q_axis(left, l.pitch * 0.5));
        let spine_w = q_mul(chest, world[spine]?);
        let neck_w = q_mul(neck_turn, q_mul(chest, world[neck]?));
        let head_w = q_mul(head_turn, q_mul(neck_turn, q_mul(chest, world[head]?)));
        for (k, &i) in skeleton.iter().enumerate() {
            let q = if i == spine {
                match nodes[spine].1 {
                    Some(p) => q_norm(q_mul(q_conj(world[p]?), spine_w)),
                    None => spine_w,
                }
            } else if i == neck {
                q_norm(q_mul(q_conj(spine_w), neck_w))
            } else if i == head {
                q_norm(q_mul(q_conj(neck_w), head_w))
            } else {
                local(i)?
            };
            tracks[k].1.push((t, q));
        }
        let hips_m = model.pose_node_transform(&base, HIPS)?;
        let parent = match nodes[hips].1 {
            Some(p) => model.pose_node_transform(&base, &nodes[p].0)?,
            None => Mat4::IDENTITY,
        };
        hips_track.push((t, parent.inverse()?.transform_point3(pos(hips_m))));
    }
    let clip = model
        .add_animation_clip(
            CLIP,
            SECS,
            tracks.clone(),
            vec![(HIPS.to_owned(), hips_track.clone())],
        )
        .ok()?;

    // Then her right arm, to wherever the flask goes.
    let k_arm = skeleton.iter().position(|&i| i == arm)?;
    let k_fore = skeleton.iter().position(|&i| i == forearm)?;
    let k_hand = skeleton.iter().position(|&i| i == hand)?;
    for (f, &t) in times.iter().enumerate() {
        let pose = model.pose(clip, t)?;
        let m = |n: &str| model.pose_node_transform(&pose, n);
        let (m_sh, m_ua, m_fa, m_h, m_head) =
            (m(SHOULDER)?, m(ARM)?, m(FOREARM)?, m(HAND)?, m(HEAD)?);
        let (q_sh, q_ua, q_fa, q_h) = (
            mat_rotation(m_sh),
            mat_rotation(m_ua),
            mat_rotation(m_fa),
            mat_rotation(m_h),
        );
        let weight = reach(t);
        let (ua, fa, hd) = if weight > 1e-3 {
            let fingers = m_h.transform_vector3(Vec3::Y).normalize_or_zero();
            let hip = pos(m_h) + fingers * PALM;
            let lips = m_head.transform_point3(lips_local);
            let target = grip_target(t, hip, lips);
            let (ua, fa, hd) = solve(
                pos(m_ua),
                pos(m_fa),
                pos(m_h),
                q_ua,
                q_fa,
                hand_local,
                target,
            );
            (
                nlerp(q_ua, ua, weight),
                nlerp(q_fa, fa, weight),
                nlerp(q_h, hd, weight),
            )
        } else {
            (q_ua, q_fa, q_h)
        };
        tracks[k_arm].1[f].1 = q_norm(q_mul(q_conj(q_sh), ua));
        tracks[k_fore].1[f].1 = q_norm(q_mul(q_conj(ua), fa));
        tracks[k_hand].1[f].1 = q_norm(q_mul(q_conj(fa), hd));
    }
    model
        .add_animation_clip(CLIP, SECS, tracks, vec![(HIPS.to_owned(), hips_track)])
        .ok()
}

/// The flask in her hand at `t`, in her model's space: `hand` is her right hand's transform there.
/// Its heart in her palm, its axis the clip's, the heart's face turned toward the camera.
pub(crate) fn in_hand(hand: Mat4, t: f32) -> Mat4 {
    let fingers = hand.transform_vector3(Vec3::Y).normalize_or_zero();
    let grip = pos(hand) + fingers * PALM;
    let up = axis(t);
    let toward = v3(FACE_TOWARD);
    let face = {
        let f = toward - up * toward.dot(up);
        if f.length() > 1e-3 {
            f.normalize_or_zero()
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        }
    };
    let side = up.cross(face).normalize_or_zero();
    let turn = Mat4::from_cols(
        [side.x, side.y, side.z, 0.0],
        [up.x, up.y, up.z, 0.0],
        [face.x, face.y, face.z, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    );
    Mat4::from_translation(grip)
        * turn
        * Mat4::from_scale(Vec3::new(BOTTLE_SCALE, BOTTLE_SCALE, BOTTLE_SCALE))
        * Mat4::from_translation(Vec3::new(0.0, -GRIP_Y, 0.0))
}

/// What a piece of the flask is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Piece {
    Liquid,
    Glass,
    Cork,
}

/// One piece of the flask, uploaded.
pub(crate) struct BottlePart {
    mesh: MeshHandle,
    material: Material3D,
    texture: Option<TextureHandle>,
    /// Its place in the flask.
    local: Mat4,
    kind: Piece,
}

/// The shop's flask, for her hand (empty when it cannot be read).
pub(crate) fn load_bottle(
    renderer: &mut Wgpu3DRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    cache: &mut HashMap<TextureCacheKey, TextureHandle>,
    white: &TextureData3D,
) -> Vec<BottlePart> {
    let path = runtime_3d_root().join(BOTTLE);
    let model = match GltfModel::load(&path) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("[elixir] {}: {error}", path.display());
            return Vec::new();
        }
    };
    let mut parts = Vec::new();
    for (i, mesh) in model.meshes.iter().enumerate() {
        let name = model.mesh_names.get(i).map(String::as_str).unwrap_or("");
        let kind = if name.contains("Liquido") {
            Piece::Liquid
        } else if name.contains("Tapon") {
            Piece::Cork
        } else {
            Piece::Glass
        };
        let Ok(handle) = renderer.upload_mesh(device, mesh) else {
            continue;
        };
        let texture = upload_texture_pair(
            renderer,
            device,
            queue,
            mesh.texture_index.and_then(|t| model.textures.get(t)),
            mesh.normal_texture_index
                .and_then(|t| model.textures.get(t)),
            cache,
            white,
        )
        .ok()
        .flatten();
        let mut material = mesh.material;
        if kind == Piece::Liquid {
            // Rosalía's pink, glowing a little of its own.
            material = Material3D::rgb(0.9, 0.06, 0.45).with_emission(0.6);
        }
        parts.push(BottlePart {
            mesh: handle,
            material,
            texture,
            local: model.mesh_transform(i),
            kind,
        });
    }
    parts
}

/// The flask's draws: `queen` places her in the world, `hand` is her right hand in her model,
/// `t` seconds into the clip (the cork comes off, the liquid goes down).
pub(crate) fn bottle_draws(
    parts: &[BottlePart],
    queen: Mat4,
    hand: Mat4,
    t: f32,
    out: &mut Vec<MeshDraw3D>,
) {
    let holder = queen * in_hand(hand, t);
    let left = liquid(t);
    for part in parts {
        let transform = match part.kind {
            Piece::Cork if !corked(t) => continue,
            Piece::Liquid if left <= 0.02 => continue,
            Piece::Liquid => {
                holder
                    * Mat4::from_translation(Vec3::new(0.0, LIQUID_FLOOR_Y, 0.0))
                    * Mat4::from_scale(Vec3::new(1.0, left, 1.0))
                    * Mat4::from_translation(Vec3::new(0.0, -LIQUID_FLOOR_Y, 0.0))
                    * part.local
            }
            _ => holder * part.local,
        };
        out.push(MeshDraw3D {
            mesh: part.mesh,
            transform,
            material: part.material,
            texture: part.texture,
        });
    }
}

/// The close shot, `c` seconds into the scene: three quarters from her left at her mouth's height,
/// so the flask reads at her lips against her face rather than over it, easing in as she drinks.
/// She stands at `spot` facing `yaw`.
pub(crate) fn camera(spot: Vec3, yaw: f32, c: f32) -> Camera3D {
    let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let left = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    let push = smooth(c / 5.0);
    Camera3D {
        position: spot
            + forward * (1.6 - 0.22 * push)
            + left * (1.12 - 0.14 * push)
            + Vec3::new(0.0, 1.5, 0.0),
        target: spot + forward * 0.08 + Vec3::new(0.0, 1.38 + 0.04 * push, 0.0),
        up: Vec3::Y,
        fov_y_radians: 36f32.to_radians(),
        orthographic_height: None,
        near: 0.05,
        far: 60.0,
    }
}

/// Points of her body in her model at `t` (for the overlay's words): her lips and her belly.
pub(crate) fn lips_and_belly(model: &GltfModel, clip: usize, t: f32) -> Option<(Vec3, Vec3)> {
    let rest = model.rest_pose();
    let head_rest = model.pose_node_transform(&rest, HEAD)?;
    let lips_local = head_rest
        .inverse()?
        .transform_point3(pos(head_rest) + v3(LIPS_FROM_HEAD));
    let pose = model.pose(clip, t)?;
    let lips = model
        .pose_node_transform(&pose, HEAD)?
        .transform_point3(lips_local);
    let belly = pos(model.pose_node_transform(&pose, HIPS)?) + Vec3::new(0.0, 0.05, 0.06);
    Some((lips, belly))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queen() -> Option<GltfModel> {
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/assets/3d/characters");
        GltfModel::load(root.join("queen_chibi_v2_hair_micro_run.glb")).ok()
    }

    #[test]
    fn the_timeline_holds_together() {
        assert!(KEYS.windows(2).all(|k| k[0].t < k[1].t));
        assert_eq!(KEYS[KEYS.len() - 1].t, SECS);
        assert!(corked(0.5) && !corked(POP + 0.01));
        assert!((liquid(1.0) - 1.0).abs() < 1e-3);
        assert!(liquid(SECS) < 0.01);
        assert!(liquid(2.3) < liquid(2.0) && liquid(2.3) > liquid(3.2));
        assert!(axis(0.0).y > 0.99, "upright at the hip");
        assert!(axis(2.3).y < -0.25, "bottom up while she drinks");
        assert!(reach(0.0) == 0.0 && reach(SECS) == 0.0 && reach(2.3) == 1.0);
    }

    #[test]
    fn she_drinks_from_it() {
        let Some(mut model) = queen() else { return };
        let walk = model.animation_index("Queen_Walk").expect("the walk");
        let clip = add_clip(&mut model, Some((walk, false))).expect("the drink clip");
        let at = |pose: &velvet_render::Pose3D, n: &str| {
            pos(model.pose_node_transform(pose, n).unwrap())
        };
        // It starts and ends on the way she stands.
        let stand = model.pose(walk, 0.0).unwrap();
        for t in [0.0, SECS] {
            let pose = model.pose(clip, t).unwrap();
            for bone in [HAND, HEAD, "mixamorig:LeftHand"] {
                let d = (at(&pose, bone) - at(&stand, bone)).length();
                assert!(d < 0.004, "{bone} at {t}: {d}");
            }
        }
        // At her lips: the flask's lip within a couple of centimetres (model units ×2 in the
        // world) of hers, in front of her face, its bottom up.
        for t in [1.9, 2.3, 2.8] {
            let pose = model.pose(clip, t).unwrap();
            let hand = model.pose_node_transform(&pose, HAND).unwrap();
            let flask = in_hand(hand, t);
            let lip = flask.transform_point3(Vec3::new(0.0, LIP_Y, 0.0));
            let (lips, _) = lips_and_belly(&model, clip, t).unwrap();
            let miss = (lip - lips).length();
            assert!(miss < 0.02, "flask's lip {miss:.3} from hers at {t}");
            let belly = flask.transform_point3(Vec3::new(0.0, GRIP_Y, 0.0));
            assert!(
                belly.z > lips.z + 0.02,
                "the flask in front of her face at {t}"
            );
            let base = flask.transform_point3(Vec3::ZERO);
            assert!(base.y > lip.y, "bottom up at {t}");
        }
        // Her head goes back while she drinks, and the hand is the right one, raised.
        let drinking = model.pose(clip, 2.3).unwrap();
        let top = at(&drinking, "mixamorig:HeadTop_End") - at(&drinking, HEAD);
        let top0 = at(&stand, "mixamorig:HeadTop_End") - at(&stand, HEAD);
        assert!(
            top.z < top0.z - 0.03,
            "head thrown back: {} vs {}",
            top.z,
            top0.z
        );
        assert!(at(&drinking, HAND).y > at(&stand, HAND).y + 0.18);
        assert!(at(&drinking, "mixamorig:LeftHand").y < at(&stand, HAND).y + 0.05);
    }
}
