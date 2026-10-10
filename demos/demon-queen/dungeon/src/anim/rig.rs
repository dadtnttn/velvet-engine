//! Procedural animation for the KayKit Rig_Medium skeletons (web prototype
//! `anim/rig.js`). The packs have no melee or ranged combat clips, so
//! attacks are authored as poses and baked into ordinary clips.
//!
//! Poses say where each limb POINTS in the character's own frame —
//! `d(right, up, forward)` — plus a few torso angles. The solver turns
//! directions into bone-local quaternions (keeping each bone's twist from
//! its rest pose) and aligns a held weapon's axis to a target direction.
//! The math is Three.js's, in double precision, so clips match the web.
use std::collections::BTreeMap;

// ------------------------------------------------------------ vectors

pub type V = [f64; 3];

pub fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
pub fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub fn mul(a: V, k: f64) -> V {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn length(a: V) -> f64 {
    dot(a, a).sqrt()
}
/// Three.js `normalize`: a zero vector stays zero.
pub fn normalize(a: V) -> V {
    let l = length(a);
    mul(a, 1.0 / if l == 0.0 { 1.0 } else { l })
}
fn lerp_v(a: V, b: V, t: f64) -> V {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Unit direction in the character's frame: right, up, forward.
pub fn d(right: f64, up: f64, forward: f64) -> V {
    normalize([-right, up, forward])
}

// ------------------------------------------------------------ quaternions

/// A quaternion `[x, y, z, w]`.
pub type Q = [f64; 4];
pub const QI: Q = [0.0, 0.0, 0.0, 1.0];

/// Three.js `a.multiply(b)`: a × b.
pub fn qmul(a: Q, b: Q) -> Q {
    let (qax, qay, qaz, qaw) = (a[0], a[1], a[2], a[3]);
    let (qbx, qby, qbz, qbw) = (b[0], b[1], b[2], b[3]);
    [
        qax * qbw + qaw * qbx + qay * qbz - qaz * qby,
        qay * qbw + qaw * qby + qaz * qbx - qax * qbz,
        qaz * qbw + qaw * qbz + qax * qby - qay * qbx,
        qaw * qbw - qax * qbx - qay * qby - qaz * qbz,
    ]
}
/// Three.js `invert` (the conjugate).
pub fn qinv(q: Q) -> Q {
    [-q[0], -q[1], -q[2], q[3]]
}
fn qnormalize(q: Q) -> Q {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if l == 0.0 {
        QI
    } else {
        let l = 1.0 / l;
        [q[0] * l, q[1] * l, q[2] * l, q[3] * l]
    }
}
fn qdot(a: Q, b: Q) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}
/// Three.js `Vector3.applyQuaternion`.
pub fn rotate(v: V, q: Q) -> V {
    let (x, y, z) = (v[0], v[1], v[2]);
    let (qx, qy, qz, qw) = (q[0], q[1], q[2], q[3]);
    let tx = 2.0 * (qy * z - qz * y);
    let ty = 2.0 * (qz * x - qx * z);
    let tz = 2.0 * (qx * y - qy * x);
    [
        x + qw * tx + qy * tz - qz * ty,
        y + qw * ty + qz * tx - qx * tz,
        z + qw * tz + qx * ty - qy * tx,
    ]
}
/// Euler angles to a quaternion in Three.js order "YXZ".
pub fn euler_yxz(e: [f64; 3]) -> Q {
    let (c1, c2, c3) = ((e[0] / 2.0).cos(), (e[1] / 2.0).cos(), (e[2] / 2.0).cos());
    let (s1, s2, s3) = ((e[0] / 2.0).sin(), (e[1] / 2.0).sin(), (e[2] / 2.0).sin());
    [
        s1 * c2 * c3 + c1 * s2 * s3,
        c1 * s2 * c3 - s1 * c2 * s3,
        c1 * c2 * s3 - s1 * s2 * c3,
        c1 * c2 * c3 + s1 * s2 * s3,
    ]
}
/// Euler angles to a quaternion in Three.js's default order "XYZ".
pub fn euler_xyz(e: [f64; 3]) -> Q {
    let (c1, c2, c3) = ((e[0] / 2.0).cos(), (e[1] / 2.0).cos(), (e[2] / 2.0).cos());
    let (s1, s2, s3) = ((e[0] / 2.0).sin(), (e[1] / 2.0).sin(), (e[2] / 2.0).sin());
    [
        s1 * c2 * c3 + c1 * s2 * s3,
        c1 * s2 * c3 - s1 * c2 * s3,
        c1 * c2 * s3 + s1 * s2 * c3,
        c1 * c2 * c3 - s1 * s2 * s3,
    ]
}
/// Three.js `setFromUnitVectors`.
fn from_unit_vectors(f: V, t: V) -> Q {
    let mut r = dot(f, t) + 1.0;
    let q = if r < 1e-8 {
        r = 0.0;
        if f[0].abs() > f[2].abs() {
            [-f[1], f[0], 0.0, r]
        } else {
            [0.0, -f[2], f[1], r]
        }
    } else {
        [
            f[1] * t[2] - f[2] * t[1],
            f[2] * t[0] - f[0] * t[2],
            f[0] * t[1] - f[1] * t[0],
            r,
        ]
    };
    qnormalize(q)
}
/// Three.js `setFromAxisAngle` (the axis must be unit length).
fn axis_angle(a: V, angle: f64) -> Q {
    let s = (angle / 2.0).sin();
    [a[0] * s, a[1] * s, a[2] * s, (angle / 2.0).cos()]
}
/// Three.js (r186) `slerpQuaternions(a, b, t)`.
fn slerp(a: Q, b: Q, t: f64) -> Q {
    let mut o = b;
    let mut dot = qdot(a, b);
    if dot < 0.0 {
        o = [-b[0], -b[1], -b[2], -b[3]];
        dot = -dot;
    }
    let mut s = 1.0 - t;
    let mut t = t;
    if dot < 0.9995 {
        let theta = dot.acos();
        let sin = theta.sin();
        s = (s * theta).sin() / sin;
        t = (t * theta).sin() / sin;
        [
            a[0] * s + o[0] * t,
            a[1] * s + o[1] * t,
            a[2] * s + o[2] * t,
            a[3] * s + o[3] * t,
        ]
    } else {
        qnormalize([
            a[0] * s + o[0] * t,
            a[1] * s + o[1] * t,
            a[2] * s + o[2] * t,
            a[3] * s + o[3] * t,
        ])
    }
}

/// The rotation taking direction `from` onto `to` (web `align`).
fn align(from: V, to: V) -> Q {
    let f = normalize(from);
    let t = normalize(to);
    if dot(f, t) < -0.99999 {
        let helper = if f[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        return axis_angle(normalize(cross(helper, f)), std::f64::consts::PI);
    }
    from_unit_vectors(f, t)
}
fn spin(axis: V, angle: f64) -> Q {
    axis_angle(normalize(axis), angle)
}

const YV: V = [0.0, 1.0, 0.0];

// ------------------------------------------------------------ rig

pub const KEYS: [&str; 23] = [
    "root",
    "hips",
    "spine",
    "chest",
    "head",
    "upperarmR",
    "lowerarmR",
    "wristR",
    "handR",
    "handslotR",
    "upperarmL",
    "lowerarmL",
    "wristL",
    "handL",
    "handslotL",
    "upperlegR",
    "lowerlegR",
    "footR",
    "toesR",
    "upperlegL",
    "lowerlegL",
    "footL",
    "toesL",
];

fn canon(name: &str) -> Option<&'static str> {
    let n: String = name
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase())
        .collect();
    KEYS.iter().copied().find(|k| k.to_ascii_lowercase() == n)
}

#[derive(Clone, Debug)]
pub struct Bone {
    /// The node's own name (clip tracks target it).
    pub name: String,
    pub parent: Option<&'static str>,
    pub rest_q: Q,
    pub rest_t: V,
}

/// Weapons each hand holds, and their axes carried into the wrist frame.
#[derive(Clone, Debug, Default)]
pub struct Armed {
    pub weapon: [Option<&'static str>; 2],
    pub axis: [Option<V>; 2],
    pub edge: [Option<V>; 2],
    pub face: [Option<V>; 2],
}

#[derive(Clone, Debug)]
pub struct Rig {
    pub bones: BTreeMap<&'static str, Bone>,
    /// Parents before children, for forward kinematics.
    pub order: Vec<&'static str>,
    pub ankle_y: Option<f64>,
    pub armed: Armed,
    /// Swing the elbow so a held blade, axe or staff leaves the fist across
    /// the forearm, as a real grip does, instead of bending the wrist until
    /// the weapon extends the arm like a club. Off reproduces the web clips.
    pub natural_grip: bool,
}

/// Rig description from a model's rest nodes (name, parent index,
/// translation, rotation).
pub fn rig_from_nodes(nodes: &[(String, Option<usize>, [f64; 3], [f64; 4])]) -> Rig {
    let mut bones = BTreeMap::new();
    let mut insertion: Vec<&'static str> = vec![];
    for (name, parent, t, r) in nodes {
        let Some(key) = canon(name) else { continue };
        let mut up = *parent;
        while let Some(p) = up {
            if canon(&nodes[p].0).is_some() {
                break;
            }
            up = nodes[p].1;
        }
        if !bones.contains_key(key) {
            insertion.push(key);
        }
        bones.insert(
            key,
            Bone {
                name: name.clone(),
                parent: up.and_then(|p| canon(&nodes[p].0)),
                rest_q: *r,
                rest_t: *t,
            },
        );
    }
    let mut order: Vec<&'static str> = vec![];
    fn visit(
        key: &'static str,
        bones: &BTreeMap<&'static str, Bone>,
        order: &mut Vec<&'static str>,
    ) {
        if order.contains(&key) {
            return;
        }
        if let Some(parent) = bones[key].parent {
            if bones.contains_key(parent) {
                visit(parent, bones, order);
            }
        }
        order.push(key);
    }
    for key in insertion {
        visit(key, &bones, &mut order);
    }
    let mut rig = Rig {
        bones,
        order,
        ankle_y: None,
        armed: Armed::default(),
        natural_grip: false,
    };
    if rig.bones.contains_key("footR") {
        let world = forward_kinematics(&rig, &Solved::default());
        rig.ankle_y = world.get("footR").map(|w| w.1[1]);
    }
    rig
}

/// How weapons sit in a hand slot: (Euler XYZ rotation, position).
pub fn attach_for(weapon: &str, side: usize) -> ([f64; 3], [f64; 3]) {
    match (weapon, side) {
        ("shield", _) => ([-0.266, 0.0, -1.2015], [0.0, -0.02, 0.04]),
        (_, 0) => (
            [-0.12, std::f64::consts::PI * 0.5, -0.08],
            [0.0, -0.04, 0.02],
        ),
        _ => ([0.12, std::f64::consts::PI * 0.5, 0.1], [0.0, -0.04, 0.02]),
    }
}
/// The long axis of each weapon model (toward its business end).
pub fn weapon_axis(weapon: &str) -> Option<V> {
    match weapon {
        "blade" | "axe" | "staff" | "shield" => Some([0.0, 1.0, 0.0]),
        "crossbow" => Some([0.0, 0.0, 1.0]),
        _ => None,
    }
}
/// Distance from the grip to the tip, in weapon units (before scaling).
pub fn weapon_reach(weapon: &str) -> f64 {
    match weapon {
        "blade" => 1.17,
        "axe" => 1.0,
        "staff" => 1.25,
        "crossbow" => 1.14,
        "shield" => 0.57,
        _ => 1.0,
    }
}
/// Weapon scale in the hand.
pub const WEAPON_SCALE: f64 = 0.72;

const SIDES: [&str; 2] = ["R", "L"];
fn key(prefix: &str, side: usize) -> &'static str {
    let name = format!("{prefix}{}", SIDES[side]);
    KEYS.iter()
        .copied()
        .find(|k| *k == name)
        .expect("known bone key")
}

/// Tell the rig which weapon each hand holds (web `withWeapons`).
pub fn with_weapons(rig: &Rig, right: Option<&'static str>, left: Option<&'static str>) -> Rig {
    let mut out = rig.clone();
    out.armed = Armed {
        weapon: [right, left],
        ..Armed::default()
    };
    for (side, name) in [right, left].into_iter().enumerate() {
        let Some(name) = name else { continue };
        let Some(axis) = weapon_axis(name) else {
            continue;
        };
        let (Some(hand), Some(slot)) = (
            rig.bones.get(key("hand", side)),
            rig.bones.get(key("handslot", side)),
        ) else {
            continue;
        };
        let attach = euler_xyz(attach_for(name, side).0);
        let to_wrist =
            |v: V| normalize(rotate(rotate(rotate(v, attach), slot.rest_q), hand.rest_q));
        out.armed.axis[side] = Some(to_wrist(axis));
        out.armed.edge[side] = Some(to_wrist([1.0, 0.0, 0.0]));
        out.armed.face[side] = Some(to_wrist(if name == "crossbow" {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        }));
    }
    out
}

// ------------------------------------------------------------ poses

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hips {
    pub rot: Option<[f64; 3]>,
    pub pos: Option<[f64; 3]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Leg {
    pub foot: Option<[f64; 2]>,
    pub lift: Option<f64>,
    pub pole: Option<V>,
    pub upper: Option<V>,
    pub lower: Option<V>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Arm {
    pub reach: Option<[f64; 3]>,
    pub grip: Option<f64>,
    pub pole: Option<V>,
    pub upper: Option<V>,
    pub fore: Option<V>,
    pub roll: Option<f64>,
    pub fore_roll: Option<f64>,
    pub blade: Option<V>,
    pub aim: Option<V>,
    pub edge: Option<V>,
    pub face: Option<V>,
    pub blade_roll: Option<f64>,
    pub wrist: Option<[f64; 3]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub hips: Option<Hips>,
    pub spine: Option<[f64; 3]>,
    pub chest: Option<[f64; 3]>,
    pub head: Option<[f64; 3]>,
    pub leg: [Option<Leg>; 2],
    pub arm: [Option<Arm>; 2],
}

fn over<T: Copy>(base: Option<T>, over: Option<T>) -> Option<T> {
    over.or(base)
}
fn merge_hips(a: Option<Hips>, b: Option<Hips>) -> Option<Hips> {
    match (a, b) {
        (a, None) => a,
        (None, Some(b)) => Some(b),
        (Some(a), Some(b)) => Some(Hips {
            rot: over(a.rot, b.rot),
            pos: over(a.pos, b.pos),
        }),
    }
}
fn merge_leg(a: Option<Leg>, b: Option<Leg>) -> Option<Leg> {
    match (a, b) {
        (a, None) => a,
        (None, Some(b)) => Some(b),
        (Some(a), Some(b)) => Some(Leg {
            foot: over(a.foot, b.foot),
            lift: over(a.lift, b.lift),
            pole: over(a.pole, b.pole),
            upper: over(a.upper, b.upper),
            lower: over(a.lower, b.lower),
        }),
    }
}
fn merge_arm(a: Option<Arm>, b: Option<Arm>) -> Option<Arm> {
    match (a, b) {
        (a, None) => a,
        (None, Some(b)) => Some(b),
        (Some(a), Some(b)) => Some(Arm {
            reach: over(a.reach, b.reach),
            grip: over(a.grip, b.grip),
            pole: over(a.pole, b.pole),
            upper: over(a.upper, b.upper),
            fore: over(a.fore, b.fore),
            roll: over(a.roll, b.roll),
            fore_roll: over(a.fore_roll, b.fore_roll),
            blade: over(a.blade, b.blade),
            aim: over(a.aim, b.aim),
            edge: over(a.edge, b.edge),
            face: over(a.face, b.face),
            blade_roll: over(a.blade_roll, b.blade_roll),
            wrist: over(a.wrist, b.wrist),
        }),
    }
}
/// Web `mergePose`: the later pose overrides, field by field.
pub fn merge(base: &Pose, o: &Pose) -> Pose {
    Pose {
        hips: merge_hips(base.hips, o.hips),
        spine: over(base.spine, o.spine),
        chest: over(base.chest, o.chest),
        head: over(base.head, o.head),
        leg: [
            merge_leg(base.leg[0], o.leg[0]),
            merge_leg(base.leg[1], o.leg[1]),
        ],
        arm: [
            merge_arm(base.arm[0], o.arm[0]),
            merge_arm(base.arm[1], o.arm[1]),
        ],
    }
}

fn slerp_dir(a: V, b: V, t: f64) -> V {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let q = slerp(QI, align(a, b), t);
    rotate(normalize(a), q)
}
fn l_num(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + (b - a) * t),
        (None, b) => b,
        (a, None) => a,
    }
}
fn l_arr<const N: usize>(a: Option<[f64; N]>, b: Option<[f64; N]>, t: f64) -> Option<[f64; N]> {
    match (a, b) {
        (Some(a), Some(b)) => Some(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)),
        (None, b) => b,
        (a, None) => a,
    }
}
fn l_dir(a: Option<V>, b: Option<V>, t: f64) -> Option<V> {
    match (a, b) {
        (Some(a), Some(b)) => Some(slerp_dir(a, b, t)),
        (None, b) => b,
        (a, None) => a,
    }
}
/// Web `lerpPose`.
pub fn lerp_pose(a: &Pose, b: &Pose, t: f64) -> Pose {
    let hips = match (a.hips, b.hips) {
        (Some(x), Some(y)) => Some(Hips {
            rot: l_arr(x.rot, y.rot, t),
            pos: l_arr(x.pos, y.pos, t),
        }),
        (None, y) => y,
        (x, None) => x,
    };
    let leg = |x: Option<Leg>, y: Option<Leg>| match (x, y) {
        (Some(x), Some(y)) => Some(Leg {
            foot: l_arr(x.foot, y.foot, t),
            lift: l_num(x.lift, y.lift, t),
            pole: l_dir(x.pole, y.pole, t),
            upper: l_dir(x.upper, y.upper, t),
            lower: l_dir(x.lower, y.lower, t),
        }),
        (None, y) => y,
        (x, None) => x,
    };
    let arm = |x: Option<Arm>, y: Option<Arm>| match (x, y) {
        (Some(x), Some(y)) => Some(Arm {
            reach: l_arr(x.reach, y.reach, t),
            grip: l_num(x.grip, y.grip, t),
            pole: l_dir(x.pole, y.pole, t),
            upper: l_dir(x.upper, y.upper, t),
            fore: l_dir(x.fore, y.fore, t),
            roll: l_num(x.roll, y.roll, t),
            fore_roll: l_num(x.fore_roll, y.fore_roll, t),
            blade: l_dir(x.blade, y.blade, t),
            aim: l_dir(x.aim, y.aim, t),
            edge: l_dir(x.edge, y.edge, t),
            face: l_dir(x.face, y.face, t),
            blade_roll: l_num(x.blade_roll, y.blade_roll, t),
            wrist: l_arr(x.wrist, y.wrist, t),
        }),
        (None, y) => y,
        (x, None) => x,
    };
    Pose {
        hips,
        spine: l_arr(a.spine, b.spine, t),
        chest: l_arr(a.chest, b.chest, t),
        head: l_arr(a.head, b.head, t),
        leg: [leg(a.leg[0], b.leg[0]), leg(a.leg[1], b.leg[1])],
        arm: [arm(a.arm[0], b.arm[0]), arm(a.arm[1], b.arm[1])],
    }
}

// ------------------------------------------------------------ solving

/// Local rotations of the solved bones (in solving order) and the hips offset.
#[derive(Clone, Debug, Default)]
pub struct Solved {
    pub rotations: Vec<(&'static str, Q)>,
    pub hips_t: Option<V>,
}
impl Solved {
    fn get(&self, key: &str) -> Option<Q> {
        self.rotations
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, q)| *q)
    }
    fn set(&mut self, key: &'static str, q: Q) {
        if let Some(slot) = self.rotations.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = q;
        } else {
            self.rotations.push((key, q));
        }
    }
}

/// Two-bone IK: segment directions for a limb from `root` reaching
/// `target`, bending toward `pole`.
fn two_bone(root: V, target: V, a: f64, b: f64, pole: V) -> (V, V) {
    let toward = sub(target, root);
    let dd = ((a - b).abs() + 1e-3).max((a + b - 1e-3).min(length(toward)));
    let u = normalize(toward);
    let mut w = sub(pole, mul(u, dot(pole, u)));
    if dot(w, w) < 1e-8 {
        w = if u[1].abs() < 0.9 {
            sub([0.0, 1.0, 0.0], mul(u, u[1]))
        } else {
            [1.0, 0.0, 0.0]
        };
    }
    let w = normalize(w);
    let cos_a = ((a * a + dd * dd - b * b) / (2.0 * a * dd)).clamp(-1.0, 1.0);
    let sin_a = (1.0 - cos_a * cos_a).sqrt();
    let elbow = add(add(root, mul(u, a * cos_a)), mul(w, a * sin_a));
    let end = add(root, mul(u, dd));
    (normalize(sub(elbow, root)), normalize(sub(end, elbow)))
}

/// Where the hand points (its bone's +Y) when the weapon it grips points
/// along `weapon` with its edge toward `edge`: the grip fixes the hand.
fn held_hand(weapon: V, edge: V, axis_local: V, edge_local: V) -> Option<V> {
    let e = sub(edge, mul(weapon, dot(edge, weapon)));
    if dot(e, e) < 1e-6 {
        return None;
    }
    let e = normalize(e);
    let c_local = cross(axis_local, edge_local);
    let c_world = cross(weapon, e);
    // The wrist's +Y in the weapon's local frame, carried to the world one.
    let y = [0.0, 1.0, 0.0];
    Some(normalize(add(
        add(mul(weapon, dot(y, axis_local)), mul(e, dot(y, edge_local))),
        mul(c_world, dot(y, c_local)),
    )))
}

/// The elbow pole, swung about the shoulder-to-hand axis, that lines the
/// forearm up with the gripping hand (no bent-back wrist) — or, when the
/// hand is free to turn, sets the forearm across the weapon — while staying
/// near the authored elbow.
fn grip_pole(shoulder: V, target: V, a: f64, b: f64, pole: V, weapon: V, hand: Option<V>) -> V {
    let u = normalize(sub(target, shoulder));
    let base = sub(pole, mul(u, dot(pole, u)));
    if dot(base, base) < 1e-8 {
        return pole;
    }
    let base = normalize(base);
    let mut best = (f64::INFINITY, pole);
    for step in 0..48 {
        let angle = (step as f64 / 48.0 - 0.5) * std::f64::consts::TAU;
        let candidate = rotate(base, axis_angle(u, angle));
        let (_, fore) = two_bone(shoulder, target, a, b, candidate);
        let grip = match hand {
            Some(hand) => dot(fore, hand).clamp(-1.0, 1.0).acos() / std::f64::consts::PI,
            None => dot(fore, weapon).abs(),
        };
        let cost = grip + 0.25 * (angle.abs() / std::f64::consts::PI).powi(2);
        if cost < best.0 {
            best = (cost, candidate);
        }
    }
    best.1
}

/// World transforms of bones from what is solved so far.
fn world_of(
    rig: &Rig,
    solved: &Solved,
    key: &'static str,
    memo: &mut BTreeMap<&'static str, (Q, V)>,
) -> (Q, V) {
    if let Some(w) = memo.get(key) {
        return *w;
    }
    let bone = &rig.bones[key];
    let q = solved.get(key).unwrap_or(bone.rest_q);
    let t = if key == "hips" {
        solved.hips_t.unwrap_or(bone.rest_t)
    } else {
        bone.rest_t
    };
    let w = match bone.parent {
        Some(p) => {
            let pw = world_of(rig, solved, p, memo);
            (qmul(pw.0, q), add(pw.1, rotate(t, pw.0)))
        }
        None => (q, t),
    };
    memo.insert(key, w);
    w
}

/// Local bone rotations (and the hips offset) for a pose (web `solve`).
pub fn solve(rig: &Rig, pose: &Pose) -> Solved {
    let b = &rig.bones;
    let mut out = Solved::default();
    let hips_rot = pose.hips.and_then(|h| h.rot).unwrap_or([0.0; 3]);
    if let Some(h) = b.get("hips") {
        out.set("hips", qmul(h.rest_q, euler_yxz(hips_rot)));
    }
    for (k, angles) in [
        ("spine", pose.spine),
        ("chest", pose.chest),
        ("head", pose.head),
    ] {
        if let Some(bone) = b.get(k) {
            out.set(k, qmul(bone.rest_q, euler_yxz(angles.unwrap_or([0.0; 3]))));
        }
    }
    let pos = pose.hips.and_then(|h| h.pos).unwrap_or([0.0; 3]);
    out.hips_t = b
        .get("hips")
        .map(|h| add(h.rest_t, [-pos[0], pos[1], pos[2]]));
    let mut memo: BTreeMap<&'static str, (Q, V)> = BTreeMap::new();

    // Legs first: they only depend on the hips.
    for side in 0..2 {
        let Some(mut spec) = pose.leg[side] else {
            continue;
        };
        let (Some(upper), Some(lower)) =
            (b.get(key("upperleg", side)), b.get(key("lowerleg", side)))
        else {
            continue;
        };
        let foot = b.get(key("foot", side));
        if let Some(foot_at) = spec.foot {
            let hip = world_of(rig, &out, key("upperleg", side), &mut memo).1;
            let mut target = [
                -foot_at[0],
                rig.ankle_y.unwrap_or(0.14) + spec.lift.unwrap_or(0.0),
                foot_at[1],
            ];
            let a = length(lower.rest_t);
            let bb = foot.map_or(0.15, |f| length(f.rest_t));
            // Out of reach, the foot slides in toward the hip rather than float.
            let reach = (a + bb) * 0.998;
            let dy = target[1] - hip[1];
            let flat = [target[0] - hip[0], 0.0, target[2] - hip[2]];
            if length(flat).powi(2) + dy * dy > reach * reach && dy.abs() < reach {
                let f = mul(normalize(flat), (reach * reach - dy * dy).sqrt());
                target = [hip[0] + f[0], target[1], hip[2] + f[2]];
            }
            let hips_q = world_of(rig, &out, "hips", &mut memo).0;
            let pole = rotate(spec.pole.unwrap_or(d(0.0, 0.0, 1.0)), hips_q);
            let (thigh, shin) = two_bone(hip, target, a, bb, pole);
            let inv = qinv(hips_q);
            spec = Leg {
                upper: Some(normalize(rotate(thigh, inv))),
                lower: Some(normalize(rotate(shin, inv))),
                ..Leg::default()
            };
        }
        let (Some(su), Some(sl)) = (spec.upper, spec.lower) else {
            continue;
        };
        let upper_h = qmul(align(rotate(YV, upper.rest_q), su), upper.rest_q);
        out.set(key("upperleg", side), upper_h);
        let inherit = qmul(upper_h, lower.rest_q);
        let lower_h = qmul(align(rotate(YV, inherit), sl), inherit);
        out.set(key("lowerleg", side), qmul(qinv(upper_h), lower_h));
        if let Some(foot) = foot {
            // Feet stay flat on the ground, whatever the knees do.
            let rest_foot = qmul(
                qmul(qmul(b["hips"].rest_q, upper.rest_q), lower.rest_q),
                foot.rest_q,
            );
            let hips_q = world_of(rig, &out, "hips", &mut memo).0;
            let lower_world = qmul(
                qmul(hips_q, upper_h),
                out.get(key("lowerleg", side)).unwrap(),
            );
            out.set(key("foot", side), qmul(qinv(lower_world), rest_foot));
        }
        memo.clear();
    }

    // Arms: the weapon hand first, so the other hand can grip its weapon.
    let order: [usize; 2] = if pose.arm[1].is_some_and(|a| a.grip.is_some()) {
        [0, 1]
    } else if pose.arm[0].is_some_and(|a| a.grip.is_some()) {
        [1, 0]
    } else {
        [0, 1]
    };
    for side in order {
        let Some(mut spec) = pose.arm[side] else {
            continue;
        };
        let (Some(upper), Some(lower), Some(wrist)) = (
            b.get(key("upperarm", side)),
            b.get(key("lowerarm", side)),
            b.get(key("wrist", side)),
        ) else {
            continue;
        };
        if spec.reach.is_some() || spec.grip.is_some() {
            let target = if let Some(grip) = spec.grip {
                let other = 1 - side;
                let slot = world_of(rig, &out, key("handslot", other), &mut memo).1;
                let wq = world_of(rig, &out, key("wrist", other), &mut memo).0;
                let axis = rotate(rig.armed.axis[other].unwrap_or([0.0, 1.0, 0.0]), wq);
                let reach = rig.armed.weapon[other].map_or(1.0, weapon_reach);
                let mut target = add(slot, mul(axis, grip * reach * WEAPON_SCALE));
                // The wrist sits a hand's length short of the grip.
                let shoulder = world_of(rig, &out, key("upperarm", side), &mut memo).1;
                let from_shoulder = normalize(sub(target, shoulder));
                target = sub(target, mul(from_shoulder, 0.12));
                target
            } else {
                let r = spec.reach.unwrap();
                [-r[0], r[1], r[2]]
            };
            let chest_q = world_of(rig, &out, "chest", &mut memo).0;
            let mut pole = rotate(
                spec.pole
                    .unwrap_or(d(if side == 0 { 1.0 } else { -1.0 }, -0.6, -0.2)),
                chest_q,
            );
            let shoulder = world_of(rig, &out, key("upperarm", side), &mut memo).1;
            let (a, b) = (length(lower.rest_t), length(wrist.rest_t));
            let held = rig.armed.weapon[side].filter(|w| matches!(*w, "blade" | "axe" | "staff"));
            // Weapon and edge directions in the model frame.
            let to_model = |v: V| {
                if spec.aim.is_some() {
                    v
                } else {
                    rotate(v, chest_q)
                }
            };
            let weapon_dir = spec.aim.or(spec.blade).map(to_model);
            if let (true, Some(_), Some(weapon_dir), None) =
                (rig.natural_grip, held, weapon_dir, spec.grip)
            {
                let hand = match (
                    spec.edge.map(to_model),
                    rig.armed.axis[side],
                    rig.armed.edge[side],
                ) {
                    (Some(edge), Some(axis), Some(edge_local)) => {
                        held_hand(normalize(weapon_dir), edge, axis, edge_local)
                    }
                    _ => None,
                };
                pole = grip_pole(shoulder, target, a, b, pole, normalize(weapon_dir), hand);
            }
            let (up, fore) = two_bone(shoulder, target, a, b, pole);
            let inv = qinv(chest_q);
            spec.upper = Some(normalize(rotate(up, inv)));
            spec.fore = Some(normalize(rotate(fore, inv)));
        }
        let (Some(su), Some(sf)) = (spec.upper, spec.fore) else {
            continue;
        };
        let mut upper_c = qmul(align(rotate(YV, upper.rest_q), su), upper.rest_q);
        if let Some(roll) = spec.roll.filter(|r| *r != 0.0) {
            upper_c = qmul(spin(su, roll), upper_c);
        }
        out.set(key("upperarm", side), upper_c);
        let inherit = qmul(upper_c, lower.rest_q);
        let mut lower_c = qmul(align(rotate(YV, inherit), sf), inherit);
        if let Some(roll) = spec.fore_roll.filter(|r| *r != 0.0) {
            lower_c = qmul(spin(sf, roll), lower_c);
        }
        out.set(key("lowerarm", side), qmul(qinv(upper_c), lower_c));
        let wrist_c0 = qmul(lower_c, wrist.rest_q);
        let mut wrist_c = wrist_c0;
        let axis = rig.armed.axis[side];
        // `aim` points the weapon in the character's frame (whatever the
        // torso does); `blade` points it in the chest frame.
        let chest_inv = qinv(world_of(rig, &out, "chest", &mut memo).0);
        let to_chest = |v: V| normalize(rotate(v, chest_inv));
        let target = match spec.aim {
            Some(aim) => Some(to_chest(aim)),
            None => spec.blade,
        };
        if let (Some(target), Some(axis)) = (target, axis) {
            wrist_c = qmul(align(rotate(axis, wrist_c0), target), wrist_c0);
            // Turn the weapon about its own axis: its edge leads the swing (or
            // lies level), or its face turns to a direction (shields).
            let want = if let Some(face) = spec.face {
                rig.armed.face[side].map(|l| (l, face))
            } else if let Some(edge) = spec.edge {
                rig.armed.edge[side].map(|l| (l, edge))
            } else {
                None
            };
            if let Some((local, dir)) = want {
                let t = normalize(target);
                let dir = if spec.aim.is_some() {
                    to_chest(dir)
                } else {
                    dir
                };
                let have = rotate(local, wrist_c);
                let a = sub(have, mul(t, dot(have, t)));
                let bb = sub(dir, mul(t, dot(dir, t)));
                if dot(a, a) > 1e-6 && dot(bb, bb) > 1e-6 {
                    let a = normalize(a);
                    let bb = normalize(bb);
                    let angle = dot(cross(a, bb), t).atan2(dot(a, bb));
                    wrist_c = qmul(spin(t, angle), wrist_c);
                }
            }
            if let Some(roll) = spec.blade_roll.filter(|r| *r != 0.0) {
                wrist_c = qmul(spin(target, roll), wrist_c);
            }
        } else if let Some(w) = spec.wrist {
            wrist_c = qmul(wrist_c0, euler_yxz(w));
        }
        out.set(key("wrist", side), qmul(qinv(lower_c), wrist_c));
        memo.clear();
    }
    out
}

/// World (model-space) transforms of every bone for a solved pose.
pub fn forward_kinematics(rig: &Rig, solved: &Solved) -> BTreeMap<&'static str, (Q, V)> {
    let mut world: BTreeMap<&'static str, (Q, V)> = BTreeMap::new();
    for &k in &rig.order {
        let bone = &rig.bones[k];
        let q = solved.get(k).unwrap_or(bone.rest_q);
        let t = if k == "hips" {
            solved.hips_t.unwrap_or(bone.rest_t)
        } else {
            bone.rest_t
        };
        let w = match bone.parent.and_then(|p| world.get(p)) {
            Some(&(pq, pp)) => (qmul(pq, q), add(pp, rotate(t, pq))),
            None => (q, t),
        };
        world.insert(k, w);
    }
    world
}

// ------------------------------------------------------------ clips

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ease {
    Linear,
    In,
    Out,
    InOut,
    In3,
    Out3,
    Snap,
}
impl Ease {
    fn apply(self, t: f64) -> f64 {
        match self {
            Ease::Linear => t,
            Ease::In => t * t,
            Ease::Out => 1.0 - (1.0 - t) * (1.0 - t),
            Ease::InOut => t * t * (3.0 - 2.0 * t),
            Ease::In3 => t * t * t,
            Ease::Out3 => 1.0 - (1.0 - t).powi(3),
            Ease::Snap => 1.0 - (1.0 - t).powi(4),
        }
    }
}

/// One key of an authored clip: its time, easing into it, and pose changes.
#[derive(Clone, Copy, Debug)]
pub struct Key {
    pub t: f64,
    pub ease: Ease,
    pub pose: Pose,
}

/// Full (filled-forward) poses of a key list.
pub fn full_keys(keys: &[Key]) -> Vec<Key> {
    let mut pose = Pose::default();
    keys.iter()
        .map(|k| {
            pose = merge(&pose, &k.pose);
            Key {
                t: k.t,
                ease: k.ease,
                pose,
            }
        })
        .collect()
}

/// The pose at time `t` of a filled key list.
pub fn pose_at(full: &[Key], t: f64) -> Pose {
    if t <= full[0].t {
        return full[0].pose;
    }
    for i in 0..full.len() - 1 {
        let (a, b) = (&full[i], &full[i + 1]);
        if t <= b.t {
            let u = (t - a.t) / (b.t - a.t).max(1e-6);
            return lerp_pose(&a.pose, &b.pose, b.ease.apply(u));
        }
    }
    full[full.len() - 1].pose
}

/// A baked clip: sample times, rotation tracks per bone key, hips positions.
#[derive(Clone, Debug)]
pub struct Baked {
    pub name: String,
    pub duration: f64,
    pub times: Vec<f64>,
    pub rotations: Vec<(&'static str, Vec<Q>)>,
    pub hips: Vec<V>,
}

/// Bake a key list into a clip for this rig (web `buildClip`, 30 fps).
pub fn build_clip(rig: &Rig, name: &str, keys: &[Key]) -> Baked {
    let mut full = full_keys(keys);
    let duration = full[full.len() - 1].t;
    let mut times: Vec<f64> = vec![];
    let mut t = 0.0f64;
    while t < duration {
        times.push((t * 1e4).round() / 1e4);
        t += 1.0 / 30.0;
    }
    for k in &full {
        times.push(k.t);
    }
    times.sort_by(|a, b| a.total_cmp(b));
    times.dedup();
    let blade_at = |full: &[Key], side: usize, t: f64| {
        pose_at(full, t.clamp(0.0, duration)).arm[side]
            .and_then(|a| a.blade)
            .map(normalize)
    };
    let mut rotations: Vec<(&'static str, Vec<Q>)> = vec![];
    let mut hips = vec![];
    let mut previous: BTreeMap<&'static str, Q> = BTreeMap::new();
    let mut last_edge: [Option<V>; 2] = [None, None];
    for &t in &times {
        // The web version edits the first key's pose in place when it
        // samples at (or before) its time; later lerps see that edge.
        let at_first = t <= full[0].t;
        let mut pose = pose_at(&full, t);
        for side in 0..2 {
            let Some(spec) = pose.arm[side] else { continue };
            let weapon = rig.armed.weapon[side];
            if spec.blade.is_none()
                || spec.face.is_some()
                || !matches!(weapon, Some("blade") | Some("axe"))
            {
                continue;
            }
            // Held still, the edge faces where the pose says; swung, it cuts
            // along the tip's direction of travel.
            let mut edge = spec.edge.map(normalize).or(last_edge[side]);
            if let (Some(before), Some(after)) = (
                blade_at(&full, side, t - 0.03),
                blade_at(&full, side, t + 0.03),
            ) {
                let travel = sub(after, before);
                let speed = length(travel) / 0.06;
                if speed > 0.5 {
                    let mut dir = normalize(travel);
                    let reference = last_edge[side].or(edge);
                    if reference.is_some_and(|r| dot(dir, r) < 0.0) {
                        dir = mul(dir, -1.0);
                    }
                    let w = ((speed - 0.5) / 3.5).clamp(0.0, 1.0);
                    edge = Some(match edge {
                        Some(e) => normalize(lerp_v(e, dir, w * w * (3.0 - 2.0 * w))),
                        None => dir,
                    });
                }
            }
            if let Some(e) = edge {
                last_edge[side] = Some(e);
                pose.arm[side] = Some(Arm {
                    edge: Some(e),
                    ..spec
                });
                if at_first {
                    full[0].pose.arm[side] = pose.arm[side];
                }
            }
        }
        let solved = solve(rig, &pose);
        for &(k, q) in &solved.rotations {
            // Keep neighbouring samples in the same hemisphere.
            let q = match previous.get(k) {
                Some(p) if qdot(*p, q) < 0.0 => [-q[0], -q[1], -q[2], -q[3]],
                _ => q,
            };
            previous.insert(k, q);
            match rotations.iter_mut().find(|(key, _)| *key == k) {
                Some((_, values)) => values.push(q),
                None => rotations.push((k, vec![q])),
            }
        }
        if let Some(h) = solved.hips_t {
            hips.push(h);
        }
    }
    Baked {
        name: name.to_string(),
        duration,
        times,
        rotations,
        hips,
    }
}
