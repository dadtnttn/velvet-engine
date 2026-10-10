//! The skeletons' combat animations, authored as poses (see `rig.rs`) and
//! baked per loadout (web `anim/enemyAttacks.js`). Each attack reads in three
//! beats — a readable wind-up (the telegraph), a fast strike on the `strike`
//! time (when damage lands or a bolt flies), and a follow-through and
//! recovery — and every weapon has a guard stance between blows.
use super::rig::{build_clip, d, with_weapons, Arm, Baked, Ease, Hips, Key, Leg, Pose, Rig};

const P: Pose = Pose {
    hips: None,
    spine: None,
    chest: None,
    head: None,
    leg: [None, None],
    arm: [None, None],
};
const A: Arm = Arm {
    reach: None,
    grip: None,
    pole: None,
    upper: None,
    fore: None,
    roll: None,
    fore_roll: None,
    blade: None,
    aim: None,
    edge: None,
    face: None,
    blade_roll: None,
    wrist: None,
};

fn hips(pos: [f64; 3], rot: [f64; 3]) -> Option<Hips> {
    Some(Hips {
        pos: Some(pos),
        rot: Some(rot),
    })
}
fn hips_rot(rot: [f64; 3]) -> Option<Hips> {
    Some(Hips {
        pos: None,
        rot: Some(rot),
    })
}
fn hips_pos(pos: [f64; 3]) -> Option<Hips> {
    Some(Hips {
        pos: Some(pos),
        rot: None,
    })
}
fn foot(right: f64, forward: f64) -> Option<Leg> {
    Some(Leg {
        foot: Some([right, forward]),
        ..Leg::default()
    })
}
fn key(t: f64, ease: Ease, pose: Pose) -> Key {
    Key { t, ease, pose }
}
use Ease::{In, In3, InOut, Linear, Out, Snap};

// ---------------------------------------------------------------- stances

pub fn blade_guard() -> Pose {
    Pose {
        hips: hips([0.0, -0.035, 0.0], [0.0, 0.14, 0.0]),
        spine: Some([0.05, 0.05, 0.0]),
        chest: Some([0.07, 0.08, 0.0]),
        head: Some([-0.06, -0.22, 0.0]),
        leg: [foot(0.2, 0.13), foot(-0.2, -0.12)],
        arm: [
            Some(Arm {
                reach: Some([0.25, 0.84, 0.3]),
                pole: Some(d(1.0, -0.5, -0.3)),
                blade: Some(d(-0.1, 0.62, 0.78)),
                edge: Some(d(0.0, -0.78, 0.62)),
                blade_roll: Some(0.0),
                ..A
            }),
            Some(Arm {
                reach: Some([-0.32, 0.86, 0.26]),
                pole: Some(d(-1.0, -0.5, -0.2)),
                wrist: Some([0.0, 0.0, 0.0]),
                ..A
            }),
        ],
    }
}

pub fn axe_guard() -> Pose {
    Pose {
        hips: hips([0.0, -0.04, 0.0], [0.0, -0.1, 0.0]),
        spine: Some([0.05, -0.05, 0.0]),
        chest: Some([0.08, -0.06, 0.0]),
        head: Some([-0.08, 0.14, 0.0]),
        leg: [foot(0.2, -0.11), foot(-0.2, 0.15)],
        arm: [
            Some(Arm {
                reach: Some([0.3, 0.98, 0.1]),
                pole: Some(d(1.0, -0.4, -0.2)),
                blade: Some(d(0.08, 0.9, 0.42)),
                edge: Some(d(0.0, -0.4, 1.0)),
                blade_roll: Some(0.0),
                ..A
            }),
            Some(Arm {
                reach: Some([-0.12, 0.92, 0.4]),
                pole: Some(d(-1.0, -0.3, 0.0)),
                aim: Some(d(0.05, 1.0, 0.08)),
                face: Some(d(0.1, 0.0, 1.0)),
                blade_roll: Some(0.0),
                ..A
            }),
        ],
    }
}

pub fn crossbow_guard() -> Pose {
    Pose {
        hips: hips([0.0, -0.03, 0.0], [0.0, -0.22, 0.0]),
        spine: Some([0.03, -0.05, 0.0]),
        chest: Some([0.05, -0.1, 0.0]),
        head: Some([0.0, 0.3, 0.0]),
        leg: [foot(0.2, -0.1), foot(-0.18, 0.14)],
        arm: [
            Some(Arm {
                reach: Some([0.2, 0.78, 0.2]),
                pole: Some(d(1.0, -0.5, -0.2)),
                aim: Some(d(0.0, -0.3, 0.95)),
                edge: Some(d(1.0, 0.0, 0.0)),
                blade_roll: Some(0.0),
                ..A
            }),
            Some(Arm {
                grip: Some(0.72),
                pole: Some(d(-1.0, -0.6, 0.0)),
                ..A
            }),
        ],
    }
}

pub fn staff_guard() -> Pose {
    Pose {
        hips: hips([0.0, -0.02, 0.0], [0.0, 0.08, 0.0]),
        spine: Some([0.02, 0.03, 0.0]),
        chest: Some([0.03, 0.05, 0.0]),
        head: Some([-0.04, -0.12, 0.0]),
        leg: [foot(0.2, -0.06), foot(-0.19, 0.08)],
        arm: [
            Some(Arm {
                reach: Some([0.26, 0.92, 0.22]),
                pole: Some(d(1.0, -0.4, -0.3)),
                blade: Some(d(-0.08, 0.95, 0.3)),
                blade_roll: Some(0.0),
                ..A
            }),
            Some(Arm {
                reach: Some([-0.3, 0.9, 0.3]),
                pole: Some(d(-1.0, -0.5, -0.2)),
                wrist: Some([0.0, 0.0, 0.0]),
                ..A
            }),
        ],
    }
}

/// A breathing guard loop: the stance, sinking a little and back.
pub fn guard_keys(base: Pose) -> Vec<Key> {
    let bob = 0.012;
    let pos = base.hips.and_then(|h| h.pos).unwrap_or([0.0; 3]);
    let chest = base.chest.unwrap_or([0.0; 3]);
    let low = Pose {
        hips: hips_pos([pos[0], pos[1] - bob, pos[2]]),
        chest: Some([chest[0] + 0.03, chest[1], chest[2]]),
        ..P
    };
    vec![
        key(0.0, InOut, base),
        key(0.8, InOut, low),
        key(1.6, InOut, base),
    ]
}

// ---------------------------------------------------------------- attacks

#[derive(Clone, Debug)]
pub struct AttackDef {
    pub name: &'static str,
    /// Seconds into the clip when the blow lands (or the bolt flies).
    pub strike: f64,
    pub reach: f64,
    /// Full arc in degrees.
    pub arc: f64,
    pub damage: f64,
    pub ranged: bool,
    /// Radius of the ground shock at the strike, if any.
    pub shockwave: Option<f64>,
    pub keys: Vec<Key>,
}

pub fn slash_a() -> AttackDef {
    AttackDef {
        name: "Slash_A",
        strike: 0.42,
        reach: 1.9,
        arc: 70.0,
        damage: 1.0,
        ranged: false,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, blade_guard()),
            key(
                0.34,
                Out,
                Pose {
                    hips: hips([-0.02, -0.05, -0.04], [0.0, -0.35, 0.0]),
                    spine: Some([-0.05, -0.15, 0.0]),
                    chest: Some([-0.12, -0.3, -0.05]),
                    head: Some([-0.02, 0.45, 0.0]),
                    leg: [foot(0.22, 0.1), foot(-0.2, -0.14)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.42, 1.3, -0.05]),
                            pole: Some(d(1.0, 0.0, -0.3)),
                            blade: Some(d(-0.35, 0.55, -0.75)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.2, 1.0, 0.45]),
                            pole: Some(d(-1.0, -0.4, 0.0)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.39,
                In,
                Pose {
                    hips: hips([0.0, -0.06, 0.03], [0.0, -0.1, 0.0]),
                    chest: Some([0.05, -0.05, 0.0]),
                    head: Some([-0.08, 0.15, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.32, 1.12, 0.35]),
                            pole: Some(d(1.0, -0.2, -0.2)),
                            blade: Some(d(0.25, 0.45, 0.85)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.45,
                Linear,
                Pose {
                    hips: hips([0.02, -0.08, 0.12], [0.0, 0.35, 0.0]),
                    spine: Some([0.15, 0.2, 0.0]),
                    chest: Some([0.3, 0.4, 0.08]),
                    head: Some([-0.2, -0.35, 0.0]),
                    leg: [foot(0.2, 0.36), foot(-0.2, -0.16)],
                    arm: [
                        Some(Arm {
                            reach: Some([-0.1, 0.72, 0.55]),
                            pole: Some(d(1.0, -0.3, 0.0)),
                            blade: Some(d(-0.55, -0.25, 0.8)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.42, 0.72, -0.12]),
                            pole: Some(d(-1.0, -0.2, 0.0)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.6,
                Out,
                Pose {
                    hips: hips_rot([0.0, 0.42, 0.0]),
                    chest: Some([0.25, 0.55, 0.06]),
                    head: Some([-0.1, -0.42, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([-0.25, 0.6, 0.3]),
                            pole: Some(d(1.0, -0.4, 0.2)),
                            blade: Some(d(-0.7, -0.6, 0.25)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(1.0, InOut, blade_guard()),
        ],
    }
}

pub fn slash_b() -> AttackDef {
    AttackDef {
        name: "Slash_B",
        strike: 0.44,
        reach: 1.9,
        arc: 70.0,
        damage: 1.0,
        ranged: false,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, blade_guard()),
            key(
                0.34,
                Out,
                Pose {
                    hips: hips([0.01, -0.05, -0.03], [0.0, 0.38, 0.0]),
                    spine: Some([-0.02, 0.18, 0.0]),
                    chest: Some([-0.06, 0.4, 0.04]),
                    head: Some([-0.05, -0.5, 0.0]),
                    leg: [foot(0.2, 0.12), foot(-0.2, -0.12)],
                    arm: [
                        Some(Arm {
                            reach: Some([-0.25, 1.08, 0.12]),
                            pole: Some(d(0.6, -0.6, -0.4)),
                            blade: Some(d(-0.55, 0.55, -0.55)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.35, 0.8, -0.05]),
                            pole: Some(d(-1.0, -0.3, 0.0)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.4,
                In,
                Pose {
                    hips: hips_rot([0.0, 0.1, 0.0]),
                    chest: Some([0.05, 0.1, 0.0]),
                    head: Some([-0.08, -0.15, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.05, 0.98, 0.42]),
                            pole: Some(d(1.0, -0.4, -0.2)),
                            blade: Some(d(-0.2, 0.3, 0.93)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.47,
                Linear,
                Pose {
                    hips: hips([0.0, -0.08, 0.1], [0.0, -0.3, 0.0]),
                    spine: Some([0.1, -0.2, 0.0]),
                    chest: Some([0.2, -0.45, -0.05]),
                    head: Some([-0.12, 0.42, 0.0]),
                    leg: [foot(0.2, 0.32), foot(-0.2, -0.15)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.45, 0.85, 0.45]),
                            pole: Some(d(1.0, -0.4, -0.2)),
                            blade: Some(d(0.75, 0.05, 0.65)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.35, 0.95, 0.35]),
                            pole: Some(d(-1.0, -0.4, 0.0)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.62,
                Out,
                Pose {
                    chest: Some([0.15, -0.55, -0.05]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.55, 0.85, 0.15]),
                            pole: Some(d(1.0, -0.2, -0.3)),
                            blade: Some(d(0.8, -0.1, -0.2)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(1.0, InOut, blade_guard()),
        ],
    }
}

pub fn chop() -> AttackDef {
    AttackDef {
        name: "Chop",
        strike: 0.64,
        reach: 2.1,
        arc: 55.0,
        damage: 1.2,
        ranged: false,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, axe_guard()),
            key(
                0.5,
                Out,
                Pose {
                    hips: hips([0.0, -0.02, -0.05], [0.0, -0.12, 0.0]),
                    spine: Some([-0.1, -0.05, 0.0]),
                    chest: Some([-0.25, -0.1, 0.0]),
                    head: Some([-0.2, 0.15, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.2, 1.52, -0.08]),
                            pole: Some(d(1.0, 0.2, -0.2)),
                            blade: Some(d(0.1, 0.35, -0.93)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.1, 1.0, 0.42]),
                            pole: Some(d(-1.0, -0.2, 0.0)),
                            aim: Some(d(0.05, 1.0, 0.1)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.57,
                In,
                Pose {
                    chest: Some([0.05, -0.02, 0.0]),
                    head: Some([-0.15, 0.05, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.18, 1.45, 0.3]),
                            pole: Some(d(1.0, 0.0, -0.3)),
                            blade: Some(d(0.05, 0.95, 0.3)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.64,
                In3,
                Pose {
                    hips: hips([0.0, -0.12, 0.14], [0.0, 0.12, 0.0]),
                    spine: Some([0.2, 0.05, 0.0]),
                    chest: Some([0.45, 0.1, 0.0]),
                    head: Some([-0.35, -0.08, 0.0]),
                    leg: [foot(0.2, -0.14), foot(-0.2, 0.34)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.1, 0.62, 0.7]),
                            pole: Some(d(1.0, -0.2, 0.0)),
                            blade: Some(d(0.02, -0.55, 0.83)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.4, 0.75, 0.05]),
                            pole: Some(d(-1.0, -0.2, 0.0)),
                            aim: Some(d(-0.2, 0.95, 0.1)),
                            face: Some(d(-0.5, 0.0, 0.85)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.84,
                InOut,
                Pose {
                    chest: Some([0.4, 0.08, 0.0]),
                    ..P
                },
            ),
            key(1.25, InOut, axe_guard()),
        ],
    }
}

pub fn shield_bash() -> AttackDef {
    AttackDef {
        name: "Shield_Bash",
        strike: 0.36,
        reach: 1.7,
        arc: 60.0,
        damage: 0.7,
        ranged: false,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, axe_guard()),
            key(
                0.26,
                Out,
                Pose {
                    hips: hips([0.0, -0.06, -0.06], [0.0, -0.25, 0.0]),
                    chest: Some([0.1, -0.25, 0.0]),
                    head: Some([-0.1, 0.3, 0.0]),
                    arm: [
                        None,
                        Some(Arm {
                            reach: Some([-0.05, 0.95, 0.22]),
                            pole: Some(d(-1.0, -0.3, -0.2)),
                            aim: Some(d(0.1, 1.0, 0.05)),
                            face: Some(d(0.3, 0.0, 1.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.37,
                In3,
                Pose {
                    hips: hips([0.0, -0.08, 0.2], [0.0, 0.22, 0.0]),
                    chest: Some([0.22, 0.28, 0.0]),
                    head: Some([-0.15, -0.2, 0.0]),
                    leg: [foot(0.2, -0.12), foot(-0.2, 0.38)],
                    arm: [
                        None,
                        Some(Arm {
                            reach: Some([0.02, 0.98, 0.62]),
                            pole: Some(d(-1.0, -0.4, 0.0)),
                            aim: Some(d(0.0, 1.0, 0.1)),
                            face: Some(d(0.0, 0.0, 1.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.5,
                InOut,
                Pose {
                    chest: Some([0.2, 0.25, 0.0]),
                    ..P
                },
            ),
            key(0.85, InOut, axe_guard()),
        ],
    }
}

pub fn shoot() -> AttackDef {
    AttackDef {
        name: "Shoot",
        strike: 0.8,
        reach: 13.5,
        arc: 360.0,
        damage: 1.0,
        ranged: true,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, crossbow_guard()),
            key(
                0.42,
                Out,
                Pose {
                    hips: hips_rot([0.0, -0.28, 0.0]),
                    chest: Some([0.0, -0.14, 0.05]),
                    head: Some([0.12, 0.36, 0.08]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.12, 1.08, 0.2]),
                            pole: Some(d(1.0, -0.4, -0.3)),
                            aim: Some(d(0.0, 0.02, 1.0)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.62),
                            pole: Some(d(-1.0, -0.7, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.76,
                InOut,
                Pose {
                    head: Some([0.14, 0.34, 0.1]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.12, 1.09, 0.21]),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.81,
                Snap,
                Pose {
                    hips: hips_pos([0.0, -0.03, -0.05]),
                    chest: Some([-0.18, -0.12, 0.05]),
                    head: Some([-0.05, 0.3, 0.05]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.12, 1.14, 0.12]),
                            aim: Some(d(0.0, 0.35, 0.94)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.97,
                InOut,
                Pose {
                    hips: hips_pos([0.0, -0.03, 0.0]),
                    chest: Some([-0.02, -0.12, 0.05]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.13, 1.06, 0.2]),
                            aim: Some(d(0.0, 0.08, 1.0)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                1.25,
                InOut,
                Pose {
                    chest: Some([0.18, -0.05, 0.0]),
                    head: Some([0.35, 0.1, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.15, 0.82, 0.3]),
                            aim: Some(d(0.15, -0.55, 0.8)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.32),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                1.5,
                InOut,
                Pose {
                    arm: [
                        None,
                        Some(Arm {
                            grip: Some(0.1),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(1.9, InOut, crossbow_guard()),
        ],
    }
}

pub fn cast() -> AttackDef {
    AttackDef {
        name: "Cast",
        strike: 0.9,
        reach: 13.5,
        arc: 360.0,
        damage: 1.0,
        ranged: true,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, staff_guard()),
            key(
                0.5,
                Out,
                Pose {
                    hips: hips_pos([0.0, 0.0, -0.03]),
                    chest: Some([-0.25, 0.1, 0.0]),
                    head: Some([-0.35, 0.0, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.15, 1.52, 0.1]),
                            pole: Some(d(1.0, 0.0, -0.2)),
                            blade: Some(d(0.05, 0.98, -0.1)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.36, 1.32, 0.3]),
                            pole: Some(d(-1.0, -0.2, -0.2)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.78,
                InOut,
                Pose {
                    chest: Some([-0.28, 0.12, 0.03]),
                    arm: [
                        None,
                        Some(Arm {
                            reach: Some([-0.38, 1.36, 0.32]),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.9,
                In3,
                Pose {
                    hips: hips_pos([0.0, -0.08, 0.1]),
                    chest: Some([0.3, 0.0, 0.0]),
                    head: Some([-0.15, 0.0, 0.0]),
                    leg: [None, foot(-0.19, 0.24)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.15, 0.8, 0.55]),
                            pole: Some(d(1.0, -0.2, 0.0)),
                            blade: Some(d(0.0, -0.35, 0.94)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.2, 1.0, 0.58]),
                            pole: Some(d(-1.0, -0.4, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                1.05,
                InOut,
                Pose {
                    chest: Some([0.25, 0.0, 0.0]),
                    ..P
                },
            ),
            key(1.5, InOut, staff_guard()),
        ],
    }
}

pub fn smash() -> AttackDef {
    AttackDef {
        name: "Smash",
        strike: 0.73,
        reach: 1.85,
        arc: 70.0,
        damage: 1.25,
        ranged: false,
        shockwave: Some(1.2),
        keys: vec![
            key(0.0, InOut, staff_guard()),
            key(
                0.55,
                Out,
                Pose {
                    hips: hips([0.0, -0.02, -0.06], [0.0, -0.2, 0.0]),
                    chest: Some([-0.3, -0.25, 0.0]),
                    head: Some([-0.1, 0.25, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.3, 1.5, -0.05]),
                            pole: Some(d(1.0, 0.1, -0.2)),
                            blade: Some(d(-0.5, 0.6, -0.62)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.35),
                            pole: Some(d(-1.0, -0.2, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.64,
                In,
                Pose {
                    chest: Some([0.0, -0.1, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.2, 1.4, 0.3]),
                            blade: Some(d(-0.1, 0.95, 0.3)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.73,
                In3,
                Pose {
                    hips: hips([0.0, -0.14, 0.12], [0.0, 0.1, 0.0]),
                    chest: Some([0.5, 0.1, 0.0]),
                    head: Some([-0.3, -0.05, 0.0]),
                    leg: [foot(0.22, 0.3), foot(-0.2, -0.16)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.05, 0.62, 0.6]),
                            pole: Some(d(1.0, -0.3, 0.0)),
                            blade: Some(d(0.0, -0.5, 0.86)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.35),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.95,
                InOut,
                Pose {
                    chest: Some([0.45, 0.08, 0.0]),
                    ..P
                },
            ),
            key(1.5, InOut, staff_guard()),
        ],
    }
}

pub fn sweep() -> AttackDef {
    AttackDef {
        name: "Sweep",
        strike: 0.6,
        reach: 1.78,
        arc: 150.0,
        damage: 1.0,
        ranged: false,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, staff_guard()),
            key(
                0.45,
                Out,
                Pose {
                    hips: hips([0.0, -0.05, -0.04], [0.0, -0.35, 0.0]),
                    chest: Some([0.0, -0.5, 0.0]),
                    head: Some([-0.05, 0.55, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.45, 1.0, -0.1]),
                            pole: Some(d(1.0, -0.3, -0.2)),
                            blade: Some(d(0.6, 0.15, -0.78)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.4),
                            pole: Some(d(-1.0, -0.4, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.55,
                In,
                Pose {
                    hips: hips_rot([0.0, -0.05, 0.0]),
                    chest: Some([0.05, -0.1, 0.0]),
                    head: Some([-0.05, 0.15, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.35, 1.0, 0.35]),
                            blade: Some(d(0.35, 0.1, 0.93)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.63,
                Linear,
                Pose {
                    hips: hips([0.0, -0.07, 0.08], [0.0, 0.35, 0.0]),
                    chest: Some([0.1, 0.5, 0.0]),
                    head: Some([-0.1, -0.45, 0.0]),
                    leg: [foot(0.2, 0.28), None],
                    arm: [
                        Some(Arm {
                            reach: Some([-0.05, 0.98, 0.5]),
                            blade: Some(d(-0.7, 0.05, 0.7)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                0.8,
                Out,
                Pose {
                    chest: Some([0.1, 0.6, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([-0.3, 0.98, 0.25]),
                            blade: Some(d(-0.95, 0.0, -0.1)),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(1.3, InOut, staff_guard()),
        ],
    }
}

// ---------------------------------------------------------------- champions
//
// Moves only the miniboss and the boss know, authored in the same pose
// language: bigger wind-ups, whole-body commitment and a roar at half health.

/// The miniboss leaps at the player and brings the axe down with both
/// shoulders behind it. Gameplay carries it through the air on its arc.
pub fn leap_slam() -> AttackDef {
    AttackDef {
        name: "Leap_Slam",
        strike: 0.95,
        reach: 2.3,
        arc: 100.0,
        damage: 1.6,
        ranged: false,
        shockwave: Some(2.2),
        keys: vec![
            key(0.0, InOut, axe_guard()),
            key(
                0.35,
                Out,
                Pose {
                    hips: hips([0.0, -0.24, -0.06], [0.0, -0.08, 0.0]),
                    spine: Some([0.3, 0.0, 0.0]),
                    chest: Some([0.25, -0.05, 0.0]),
                    head: Some([-0.35, 0.05, 0.0]),
                    leg: [foot(0.24, 0.12), foot(-0.24, -0.12)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.36, 0.55, -0.22]),
                            pole: Some(d(1.0, 0.0, -0.2)),
                            blade: Some(d(0.3, -0.3, -0.9)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.26, 0.72, 0.32]),
                            pole: Some(d(-1.0, -0.3, 0.0)),
                            aim: Some(d(0.0, 1.0, 0.1)),
                            face: Some(d(0.0, 0.0, 1.0)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.62,
                Out,
                Pose {
                    hips: hips([0.0, 0.06, 0.05], [0.0, 0.0, 0.0]),
                    spine: Some([-0.15, 0.0, 0.0]),
                    chest: Some([-0.32, 0.0, 0.0]),
                    head: Some([-0.12, 0.0, 0.0]),
                    leg: [
                        Some(Leg {
                            foot: Some([0.2, -0.08]),
                            lift: Some(0.3),
                            ..Leg::default()
                        }),
                        Some(Leg {
                            foot: Some([-0.2, 0.14]),
                            lift: Some(0.22),
                            ..Leg::default()
                        }),
                    ],
                    arm: [
                        Some(Arm {
                            reach: Some([0.2, 1.62, -0.12]),
                            pole: Some(d(1.0, 0.3, -0.2)),
                            blade: Some(d(0.05, 0.35, -0.93)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.36, 1.1, 0.24]),
                            pole: Some(d(-1.0, -0.2, 0.0)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                0.95,
                In3,
                Pose {
                    hips: hips([0.0, -0.26, 0.2], [0.0, 0.08, 0.0]),
                    spine: Some([0.35, 0.0, 0.0]),
                    chest: Some([0.5, 0.05, 0.0]),
                    head: Some([-0.42, 0.0, 0.0]),
                    leg: [foot(0.24, -0.16), foot(-0.24, 0.36)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.05, 0.36, 0.76]),
                            pole: Some(d(1.0, -0.2, 0.0)),
                            blade: Some(d(0.0, -0.82, 0.57)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.42, 0.7, 0.1]),
                            pole: Some(d(-1.0, -0.2, 0.0)),
                            aim: Some(d(-0.2, 0.95, 0.1)),
                            face: Some(d(-0.5, 0.0, 0.85)),
                            ..A
                        }),
                    ],
                },
            ),
            key(
                1.25,
                InOut,
                Pose {
                    chest: Some([0.46, 0.05, 0.0]),
                    head: Some([-0.3, 0.0, 0.0]),
                    ..P
                },
            ),
            key(1.7, InOut, axe_guard()),
        ],
    }
}

/// The miniboss spins with the axe out at arm's length. Gameplay turns the
/// whole body; the pose holds the weapon level through two sweeps.
pub fn whirlwind() -> AttackDef {
    let spin_arm = Arm {
        reach: Some([0.62, 1.0, 0.25]),
        pole: Some(d(1.0, -0.1, -0.2)),
        blade: Some(d(1.0, 0.05, 0.25)),
        edge: Some(d(0.0, 0.0, 1.0)),
        ..A
    };
    let shield = Arm {
        reach: Some([-0.5, 1.0, 0.1]),
        pole: Some(d(-1.0, -0.2, 0.0)),
        aim: Some(d(-0.3, 1.0, 0.0)),
        face: Some(d(-1.0, 0.0, 0.2)),
        ..A
    };
    let spinning = Pose {
        hips: hips([0.0, -0.1, 0.0], [0.0, 0.3, 0.0]),
        chest: Some([0.1, 0.4, 0.0]),
        head: Some([-0.05, -0.3, 0.0]),
        leg: [foot(0.26, 0.05), foot(-0.26, -0.05)],
        arm: [Some(spin_arm), Some(shield)],
        ..P
    };
    AttackDef {
        name: "Whirlwind",
        strike: 0.8,
        reach: 2.4,
        arc: 360.0,
        damage: 0.9,
        ranged: false,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, axe_guard()),
            key(
                0.4,
                Out,
                Pose {
                    hips: hips([0.0, -0.1, 0.0], [0.0, -0.6, 0.0]),
                    chest: Some([0.1, -0.5, 0.0]),
                    head: Some([-0.05, 0.4, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.55, 0.95, -0.15]),
                            pole: Some(d(1.0, -0.3, -0.2)),
                            blade: Some(d(0.7, 0.1, -0.7)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.35, 0.95, 0.2]),
                            pole: Some(d(-1.0, -0.3, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(0.55, In, spinning),
            key(1.45, Linear, spinning),
            key(1.8, InOut, axe_guard()),
        ],
    }
}

/// The boss raises its staff and hurls three crimson orbs at marked spots.
pub fn barrage() -> AttackDef {
    AttackDef {
        name: "Barrage",
        strike: 1.0,
        reach: 13.5,
        arc: 360.0,
        damage: 0.8,
        ranged: true,
        shockwave: None,
        keys: vec![
            key(0.0, InOut, staff_guard()),
            key(
                0.6,
                Out,
                Pose {
                    hips: hips_pos([0.0, 0.0, -0.05]),
                    chest: Some([-0.3, 0.0, 0.0]),
                    head: Some([-0.4, 0.0, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.22, 1.5, 0.32]),
                            pole: Some(d(1.0, -0.2, -0.2)),
                            blade: Some(d(0.0, 1.0, 0.05)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.42, 1.45, 0.3]),
                            pole: Some(d(-1.0, 0.0, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                0.95,
                InOut,
                Pose {
                    chest: Some([-0.33, 0.02, 0.04]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.22, 1.56, 0.34]),
                            ..A
                        }),
                        None,
                    ],
                    ..P
                },
            ),
            key(
                1.05,
                In3,
                Pose {
                    hips: hips_pos([0.0, -0.06, 0.08]),
                    chest: Some([0.25, 0.0, 0.0]),
                    head: Some([-0.1, 0.0, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.15, 1.25, 0.55]),
                            pole: Some(d(1.0, -0.3, 0.0)),
                            blade: Some(d(0.0, 0.8, 0.6)),
                            ..A
                        }),
                        Some(Arm {
                            reach: Some([-0.25, 1.2, 0.6]),
                            pole: Some(d(-1.0, -0.3, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                1.4,
                InOut,
                Pose {
                    chest: Some([0.2, 0.0, 0.0]),
                    ..P
                },
            ),
            key(1.9, InOut, staff_guard()),
        ],
    }
}

/// The boss lifts its staff in both hands and drives it into the floor: a
/// nova of force rolls out all around it and smashes what stands near.
pub fn nova() -> AttackDef {
    AttackDef {
        name: "Nova",
        strike: 1.3,
        reach: 4.3,
        arc: 360.0,
        damage: 1.5,
        ranged: false,
        shockwave: Some(4.3),
        keys: vec![
            key(0.0, InOut, staff_guard()),
            key(
                0.8,
                Out,
                Pose {
                    hips: hips([0.0, 0.03, -0.06], [0.0, 0.0, 0.0]),
                    spine: Some([-0.1, 0.0, 0.0]),
                    chest: Some([-0.35, 0.0, 0.0]),
                    head: Some([-0.45, 0.0, 0.0]),
                    arm: [
                        Some(Arm {
                            reach: Some([0.1, 1.88, 0.06]),
                            pole: Some(d(1.0, 0.1, 0.0)),
                            blade: Some(d(0.0, 1.0, 0.0)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.3),
                            pole: Some(d(-1.0, 0.1, 0.0)),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                1.15,
                InOut,
                Pose {
                    chest: Some([-0.38, 0.0, 0.04]),
                    head: Some([-0.5, 0.0, 0.0]),
                    ..P
                },
            ),
            key(
                1.3,
                In3,
                Pose {
                    hips: hips([0.0, -0.3, 0.12], [0.0, 0.0, 0.0]),
                    spine: Some([0.3, 0.0, 0.0]),
                    chest: Some([0.55, 0.0, 0.0]),
                    head: Some([-0.35, 0.0, 0.0]),
                    leg: [foot(0.26, 0.25), foot(-0.26, -0.1)],
                    arm: [
                        Some(Arm {
                            reach: Some([0.05, 0.35, 0.72]),
                            pole: Some(d(1.0, -0.3, 0.0)),
                            blade: Some(d(0.0, -0.95, 0.3)),
                            ..A
                        }),
                        Some(Arm {
                            grip: Some(0.3),
                            ..A
                        }),
                    ],
                    ..P
                },
            ),
            key(
                1.7,
                InOut,
                Pose {
                    chest: Some([0.5, 0.0, 0.0]),
                    ..P
                },
            ),
            key(2.2, InOut, staff_guard()),
        ],
    }
}

/// Half-dead, a champion throws its arms wide and roars: it fights faster
/// from here on (and the Heart answers the boss's cry with minions).
pub fn roar(guard: Pose) -> AttackDef {
    let open = |arm: Option<Arm>, reach: [f64; 3]| {
        arm.map(|a| Arm {
            reach: Some(reach),
            grip: None,
            pole: None,
            ..a
        })
    };
    AttackDef {
        name: "Roar",
        strike: 0.7,
        reach: 3.2,
        arc: 360.0,
        damage: 0.0,
        ranged: false,
        shockwave: Some(3.2),
        keys: vec![
            key(0.0, InOut, guard),
            key(
                0.35,
                Out,
                Pose {
                    hips: hips_pos([0.0, -0.14, -0.05]),
                    spine: Some([0.2, 0.0, 0.0]),
                    chest: Some([0.35, 0.0, 0.0]),
                    head: Some([0.25, 0.0, 0.0]),
                    arm: [
                        open(guard.arm[0], [0.35, 0.7, 0.2]),
                        open(guard.arm[1], [-0.35, 0.7, 0.2]),
                    ],
                    ..P
                },
            ),
            key(
                0.7,
                In3,
                Pose {
                    hips: hips_pos([0.0, 0.0, 0.02]),
                    spine: Some([-0.15, 0.0, 0.0]),
                    chest: Some([-0.42, 0.0, 0.0]),
                    head: Some([-0.62, 0.0, 0.0]),
                    arm: [
                        open(guard.arm[0], [0.62, 1.35, 0.05]),
                        open(guard.arm[1], [-0.62, 1.35, 0.05]),
                    ],
                    ..P
                },
            ),
            key(
                1.15,
                InOut,
                Pose {
                    head: Some([-0.55, 0.12, 0.0]),
                    ..P
                },
            ),
            key(1.6, InOut, guard),
        ],
    }
}

// ---------------------------------------------------------------- sets

/// What an enemy carries (web `enemyLoadout`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loadout {
    pub weapon: &'static str,
    pub offhand: Option<&'static str>,
}

/// Guard and attacks for a loadout; `melee` picks the boss's staff set and
/// `champion` adds the miniboss's and boss's own moves (and their roar).
pub fn attack_set(loadout: Loadout, melee: bool, champion: bool) -> (Pose, Vec<AttackDef>) {
    match loadout.weapon {
        "crossbow" => (crossbow_guard(), vec![shoot()]),
        "axe" if loadout.offhand == Some("shield") && champion => (
            axe_guard(),
            vec![
                chop(),
                shield_bash(),
                chop(),
                leap_slam(),
                whirlwind(),
                roar(axe_guard()),
            ],
        ),
        "axe" if loadout.offhand == Some("shield") => {
            (axe_guard(), vec![chop(), shield_bash(), chop()])
        }
        "axe" => (axe_guard(), vec![chop()]),
        "staff" if melee && champion => (
            staff_guard(),
            vec![smash(), sweep(), barrage(), nova(), roar(staff_guard())],
        ),
        "staff" if melee => (staff_guard(), vec![smash(), sweep()]),
        "staff" => (staff_guard(), vec![cast()]),
        _ => (blade_guard(), vec![slash_a(), slash_b()]),
    }
}

/// Baked combat clips for a rig and loadout.
pub struct CombatClips {
    pub guard: Baked,
    pub attacks: Vec<(AttackDef, Baked)>,
}

/// Bake the guard loop and attacks of a loadout (web `combatClips`).
pub fn combat_clips(rig: &Rig, loadout: Loadout, melee: bool, champion: bool) -> CombatClips {
    let armed = with_weapons(rig, Some(loadout.weapon), loadout.offhand);
    let (guard, attacks) = attack_set(loadout, melee, champion);
    CombatClips {
        guard: build_clip(
            &armed,
            &format!("Guard_{}", loadout.weapon),
            &guard_keys(guard),
        ),
        attacks: attacks
            .into_iter()
            .map(|def| {
                let clip = build_clip(&armed, def.name, &def.keys);
                (def, clip)
            })
            .collect(),
    }
}

#[cfg(test)]
mod parity {
    use super::super::rig::rig_from_nodes;
    use super::*;

    /// Rest nodes straight from a GLB's JSON chunk (doubles, like the web).
    fn glb_nodes(path: &str) -> Vec<(String, Option<usize>, [f64; 3], [f64; 4])> {
        let bytes = std::fs::read(path).unwrap();
        let len = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&bytes[20..20 + len]).unwrap();
        let nodes = json["nodes"].as_array().unwrap();
        let mut parent = vec![None; nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            for child in node["children"].as_array().into_iter().flatten() {
                parent[child.as_u64().unwrap() as usize] = Some(i);
            }
        }
        let f = |v: &serde_json::Value, i: usize, d: f64| {
            v.as_array().map_or(d, |a| a[i].as_f64().unwrap())
        };
        nodes
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let (t, r) = (&n["translation"], &n["rotation"]);
                (
                    n["name"].as_str().unwrap_or("").to_string(),
                    parent[i],
                    [f(t, 0, 0.0), f(t, 1, 0.0), f(t, 2, 0.0)],
                    [f(r, 0, 0.0), f(r, 1, 0.0), f(r, 2, 0.0), f(r, 3, 1.0)],
                )
            })
            .collect()
    }

    #[test]
    fn baked_combat_clips_match_the_web_prototype() {
        let dump = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../temp/parity/combat-clips.json"
        );
        let Ok(text) = std::fs::read_to_string(dump) else {
            eprintln!("no web dump at {dump}; skipped");
            return;
        };
        let web: serde_json::Value = serde_json::from_str(&text).unwrap();
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/assets/3d/enemies/kaykit_skeletons/characters/"
        );
        let loadouts: [(&str, Option<&str>, bool); 6] = [
            ("blade", None, false),
            ("axe", Some("shield"), false),
            ("axe", None, false),
            ("crossbow", None, false),
            ("staff", None, false),
            ("staff", None, true),
        ];
        let (mut clips, mut worst) = (0, 0.0f64);
        for model in [
            "Skeleton_Minion",
            "Skeleton_Warrior",
            "Skeleton_Rogue",
            "Skeleton_Mage",
        ] {
            let rig = rig_from_nodes(&glb_nodes(&format!("{dir}{model}.glb")));
            for (weapon, offhand, melee) in loadouts {
                let set = combat_clips(&rig, Loadout { weapon, offhand }, melee, false);
                for clip in std::iter::once(&set.guard).chain(set.attacks.iter().map(|(_, c)| c)) {
                    let id = format!(
                        "{model}|{weapon}|{}|{melee}|{}",
                        offhand.unwrap_or(""),
                        clip.name
                    );
                    let w = &web[&id];
                    assert!(w.is_object(), "{id} missing from the web dump");
                    let times: Vec<f64> = w["times"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap())
                        .collect();
                    assert_eq!(times.len(), clip.times.len(), "{id} sample count");
                    for (a, b) in times.iter().zip(&clip.times) {
                        assert!((a - b).abs() < 1e-6, "{id} time {a} vs {b}");
                    }
                    let rots = w["rotations"].as_object().unwrap();
                    assert_eq!(rots.len(), clip.rotations.len(), "{id} track count");
                    for (key, values) in &clip.rotations {
                        let js: Vec<f64> = rots[*key]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_f64().unwrap())
                            .collect();
                        for (i, q) in values.iter().enumerate() {
                            for c in 0..4 {
                                let e = (js[i * 4 + c] - q[c]).abs();
                                worst = worst.max(e);
                                assert!(
                                    e < 2e-5,
                                    "{id} {key} sample {i} ({}s): web {:?} rust {:?}",
                                    clip.times[i],
                                    &js[i * 4..i * 4 + 4],
                                    q
                                );
                            }
                        }
                    }
                    let hips: Vec<f64> = w["hips"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap())
                        .collect();
                    for (i, h) in clip.hips.iter().enumerate() {
                        for c in 0..3 {
                            assert!(
                                (hips[i * 3 + c] - h[c]).abs() < 2e-5,
                                "{id} hips sample {i}"
                            );
                        }
                    }
                    clips += 1;
                }
            }
        }
        eprintln!("{clips} combat clips match (worst quaternion error {worst:e})");
        assert_eq!(clips, 64, "axe and shield repeat Chop");
    }
}

/// Whether a clip name is one of the baked attacks (played from its start).
pub fn is_attack_clip(name: &str) -> bool {
    matches!(
        name,
        "Slash_A"
            | "Slash_B"
            | "Chop"
            | "Shield_Bash"
            | "Shoot"
            | "Cast"
            | "Smash"
            | "Sweep"
            | "Leap_Slam"
            | "Whirlwind"
            | "Barrage"
            | "Nova"
            | "Roar"
    )
}

#[cfg(test)]
mod grip {
    use super::super::rig::*;
    use super::*;

    /// (mean |cos| forearm·weapon, mean wrist bend in degrees) over a clip's poses.
    fn grip_stats(rig: &Rig, keys: &[Key]) -> (f64, f64) {
        let full = full_keys(keys);
        let end = full.last().unwrap().t;
        let (mut across, mut bend, mut n) = (0.0, 0.0, 0.0);
        let mut t = 0.0;
        while t <= end {
            let world = forward_kinematics(rig, &solve(rig, &pose_at(&full, t)));
            let fore = rotate([0.0, 1.0, 0.0], world["lowerarmR"].0);
            let hand = rotate([0.0, 1.0, 0.0], world["wristR"].0);
            let weapon = rotate(rig.armed.axis[0].unwrap(), world["wristR"].0);
            across += dot(fore, weapon).abs();
            bend += dot(fore, hand).clamp(-1.0, 1.0).acos().to_degrees();
            n += 1.0;
            t += 1.0 / 30.0;
        }
        (across / n, bend / n)
    }

    #[test]
    fn held_weapons_are_gripped_without_bending_the_wrist_back() {
        let dir = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/assets/3d/enemies/kaykit_skeletons/characters/"
        );
        for (model, weapon, offhand, melee) in [
            ("Skeleton_Minion", "blade", None, false),
            ("Skeleton_Warrior", "axe", Some("shield"), false),
            ("Skeleton_Mage", "staff", None, true),
        ] {
            let gltf = velvet_render::GltfModel::load(format!("{dir}{model}.glb")).unwrap();
            let nodes: Vec<_> = gltf
                .rest_nodes()
                .into_iter()
                .map(|(n, p, t, r)| (n, p, [t.x as f64, t.y as f64, t.z as f64], r.map(f64::from)))
                .collect();
            let web = with_weapons(&rig_from_nodes(&nodes), Some(weapon), offhand);
            let natural = Rig {
                natural_grip: true,
                ..web.clone()
            };
            let (guard, defs) = attack_set(Loadout { weapon, offhand }, melee, true);
            for (name, keys) in std::iter::once(("guard", guard_keys(guard)))
                .chain(defs.iter().map(|d| (d.name, d.keys.clone())))
            {
                let before = grip_stats(&web, &keys);
                let after = grip_stats(&natural, &keys);
                eprintln!(
                    "{model} {name}: |cos| {:.2} -> {:.2}, wrist bend {:.0}° -> {:.0}°",
                    before.0, after.0, before.1, after.1
                );
                assert!(
                    after.1 < before.1 + 1.0 && after.1 < 75.0,
                    "{model} {name}: the wrist stays near straight"
                );
            }
        }
    }
}
