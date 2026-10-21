//! Enemy combat from the web prototype (`actors.js`): what each foe carries,
//! the attacks its weapon cycles through, their timing and hit tests, the
//! crossbow bolt and the grave mage's rupture, and the short-lived effects
//! blows leave behind. The poses behind these attacks live in `anim/`.
use velvet_dungeon::gameplay::SpawnKind;
use velvet_math::Vec3;

/// What an enemy carries: its skeleton model and weapons (web `enemyLoadout`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loadout {
    pub model: &'static str,
    pub weapon: &'static str,
    pub offhand: Option<&'static str>,
}

pub fn loadout(definition: &str, kind: SpawnKind, variant: usize) -> Loadout {
    let simple = |model, weapon| Loadout {
        model,
        weapon,
        offhand: None,
    };
    if kind == SpawnKind::Boss || definition == "grave_mage" {
        return simple("mage", "staff");
    }
    if definition == "bone_scout" {
        return simple("rogue", "crossbow");
    }
    if matches!(definition, "bone_guard" | "sentinel")
        || matches!(kind, SpawnKind::Elite | SpawnKind::Miniboss)
    {
        return if kind == SpawnKind::Elite {
            simple("rogue", "blade")
        } else {
            Loadout {
                model: "warrior",
                weapon: "axe",
                offhand: Some("shield"),
            }
        };
    }
    simple(
        if definition == "lurker" && variant % 2 == 1 {
            "rogue"
        } else {
            "minion"
        },
        "blade",
    )
}

/// Foes that fight from range: scouts shoot, grave mages cast.
pub fn ranged(definition: &str) -> bool {
    matches!(definition, "bone_scout" | "grave_mage")
}

/// The boss swings its staff instead of casting.
pub fn melee_staff(definition: &str, kind: SpawnKind) -> bool {
    !ranged(definition) && loadout(definition, kind, 0).weapon == "staff"
}

/// Body height in metres (the web fits every skeleton to this).
pub fn body_height(kind: SpawnKind) -> f32 {
    match kind {
        SpawnKind::Boss => 3.9,
        SpawnKind::Miniboss => 2.36,
        _ => 1.95,
    }
}

/// Size relative to an ordinary skeleton; reaches scale with it.
pub fn body_size(kind: SpawnKind) -> f32 {
    body_height(kind) / 1.95
}

/// Big foes wind up slower, elites faster.
pub fn attack_speed(kind: SpawnKind) -> f32 {
    match kind {
        SpawnKind::Boss => 0.88,
        SpawnKind::Miniboss => 0.92,
        SpawnKind::Elite => 1.08,
        _ => 1.0,
    }
}

/// Breather after a blow before the next may start.
fn attack_pause(kind: SpawnKind) -> f32 {
    match kind {
        SpawnKind::Boss => 0.35,
        SpawnKind::Miniboss => 0.5,
        SpawnKind::Elite => 0.6,
        _ => 0.85,
    }
}

/// What a move does when it lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveKind {
    /// A weapon blow: reach, arc and height decide the hit.
    Melee,
    /// A crossbow bolt at the player's chest.
    Shoot,
    /// An orb to the player's feet, marked when the cast began.
    Cast,
    /// Three crimson orbs to three marked spots around the player.
    Barrage,
    /// A leap onto the player, then a slam (the miniboss).
    Leap,
    /// A spin with the weapon out, striking twice all around.
    Whirl,
    /// Force rolling out all around the boss.
    Nova,
    /// A champion's roar at half health: no blow, but it enrages.
    Roar,
}

/// One attack: its clip (same name), when it lands, and what it reaches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    pub kind: MoveKind,
    /// A second landing (the whirlwind's second sweep).
    pub repeat: Option<f32>,
    pub name: &'static str,
    pub strike: f32,
    pub reach: f32,
    /// Full arc in degrees.
    pub arc: f32,
    pub damage: f32,
    pub ranged: bool,
    pub shockwave: Option<f32>,
    /// Clip length in seconds.
    pub duration: f32,
    /// Where the right-hand weapon's tip is at the strike, for an ordinary
    /// skeleton: (right, up, forward) in metres (solved from the poses).
    pub tip: [f32; 3],
}

const fn melee(
    name: &'static str,
    strike: f32,
    reach: f32,
    arc: f32,
    damage: f32,
    duration: f32,
    tip: [f32; 3],
) -> Move {
    Move {
        kind: MoveKind::Melee,
        repeat: None,
        name,
        strike,
        reach,
        arc,
        damage,
        ranged: false,
        shockwave: None,
        duration,
        tip,
    }
}

pub const SLASH_A: Move = melee("Slash_A", 0.42, 1.9, 70.0, 1.0, 1.0, [-0.11, 0.73, 1.11]);
pub const SLASH_B: Move = melee("Slash_B", 0.44, 1.9, 70.0, 1.0, 1.0, [0.87, 0.77, 0.81]);
pub const CHOP: Move = melee("Chop", 0.64, 2.1, 55.0, 1.2, 1.25, [-0.10, -0.02, 0.68]);
pub const SHIELD_BASH: Move = melee(
    "Shield_Bash",
    0.36,
    1.7,
    60.0,
    0.7,
    0.85,
    [0.33, 1.11, 0.45],
);
pub const SHOOT: Move = Move {
    kind: MoveKind::Shoot,
    ranged: true,
    ..melee("Shoot", 0.8, 13.5, 360.0, 1.0, 1.9, [0.14, 1.28, 0.88])
};
pub const CAST: Move = Move {
    kind: MoveKind::Cast,
    ranged: true,
    ..melee("Cast", 0.9, 13.5, 360.0, 1.0, 1.5, [0.19, 0.18, 0.92])
};
pub const SMASH: Move = Move {
    shockwave: Some(1.2),
    ..melee("Smash", 0.73, 1.85, 70.0, 1.25, 1.5, [0.09, -0.09, 0.80])
};
pub const SWEEP: Move = melee("Sweep", 0.6, 1.78, 150.0, 1.0, 1.3, [-0.33, 0.69, 0.84]);
pub const LEAP_SLAM: Move = Move {
    kind: MoveKind::Leap,
    shockwave: Some(2.2),
    ..melee("Leap_Slam", 0.95, 2.3, 100.0, 1.6, 1.7, [-0.1, -0.02, 0.75])
};
pub const WHIRLWIND: Move = Move {
    kind: MoveKind::Whirl,
    repeat: Some(1.15),
    ..melee("Whirlwind", 0.8, 2.4, 360.0, 0.9, 1.8, [0.9, 1.0, 0.3])
};
pub const BARRAGE: Move = Move {
    kind: MoveKind::Barrage,
    ranged: true,
    ..melee("Barrage", 1.0, 13.5, 360.0, 0.8, 1.9, [0.15, 1.9, 0.95])
};
pub const NOVA: Move = Move {
    kind: MoveKind::Nova,
    shockwave: Some(4.3),
    ..melee("Nova", 1.3, 4.3, 360.0, 1.5, 2.2, [0.05, -0.1, 0.8])
};
pub const ROAR: Move = Move {
    kind: MoveKind::Roar,
    shockwave: Some(3.2),
    ..melee("Roar", 0.7, 3.2, 360.0, 0.0, 1.6, [0.0, 1.4, 0.3])
};

impl Move {
    /// World position of the weapon tip at the strike.
    pub fn tip_at(&self, position: Vec3, yaw: f32, size: f32) -> Vec3 {
        let forward = Vec3::new(yaw.sin(), 0.0, yaw.cos());
        let right = Vec3::new(-yaw.cos(), 0.0, yaw.sin());
        position + (right * self.tip[0] + Vec3::Y * self.tip[1] + forward * self.tip[2]) * size
    }
}

/// The attacks a loadout cycles through (web `attackSet`); champions (the
/// miniboss and boss) add their own, and end with the roar they keep for
/// half health (never part of the cycle).
pub fn moves_for(loadout: Loadout, melee_staff: bool, champion: bool) -> &'static [Move] {
    match loadout.weapon {
        "crossbow" => &[SHOOT],
        "axe" if loadout.offhand == Some("shield") && champion => {
            &[CHOP, SHIELD_BASH, CHOP, LEAP_SLAM, WHIRLWIND, ROAR]
        }
        "axe" if loadout.offhand == Some("shield") => &[CHOP, SHIELD_BASH, CHOP],
        "axe" => &[CHOP],
        "staff" if melee_staff && champion => &[SMASH, SWEEP, BARRAGE, NOVA, ROAR],
        "staff" if melee_staff => &[SMASH, SWEEP],
        "staff" => &[CAST],
        _ => &[SLASH_A, SLASH_B],
    }
}

/// The attacks this foe cycles through.
pub fn moves(definition: &str, kind: SpawnKind, id: usize) -> &'static [Move] {
    moves_for(
        loadout(definition, kind, id),
        melee_staff(definition, kind),
        champion(kind),
    )
}

/// The miniboss and the boss: tougher, with their own moves and a roar.
pub fn champion(kind: SpawnKind) -> bool {
    matches!(kind, SpawnKind::Miniboss | SpawnKind::Boss)
}

/// How many of `moves` take turns (a trailing roar is held back).
pub fn cycle_len(moves: &[Move]) -> usize {
    moves.len() - usize::from(moves.last().is_some_and(|m| m.kind == MoveKind::Roar))
}

/// Index of the roar in `moves`, for champions.
pub fn roar_index(moves: &[Move]) -> Option<usize> {
    moves.iter().position(|m| m.kind == MoveKind::Roar)
}

/// How many plain cuts a foe of this kind takes, relative to an ordinary
/// one: the miniboss about 2.5 times as many, the boss four.
pub fn toughness(kind: SpawnKind) -> f32 {
    match kind {
        SpawnKind::Boss => 4.0,
        SpawnKind::Miniboss => 2.5,
        _ => 1.0,
    }
}

/// How long a fallen foe stays: its death animation, then it sinks away.
pub const DEATH_SECONDS: f32 = 3.4;
/// When the body starts sinking into the floor.
pub const SINK_AFTER: f32 = 2.3;

/// Pause before a foe's first blow, staggered so a group doesn't strike as one.
pub fn opening_cooldown(id: usize) -> f32 {
    0.9 + (id % 7) as f32 * 0.11
}

/// Cooldown set when an attack starts: the clip, a breather and a stagger.
pub fn cooldown_after(mv: &Move, kind: SpawnKind, id: usize) -> f32 {
    mv.duration / attack_speed(kind) + attack_pause(kind) + (id % 5) as f32 * 0.08
}

/// Once enraged a champion strikes a third faster, with shorter breathers.
pub const ENRAGED_SPEED: f32 = 1.3;

/// Height of a leaping foe above the floor at clip time `t`.
pub fn leap_lift(mv: &Move, t: f32) -> f32 {
    if mv.kind != MoveKind::Leap {
        return 0.0;
    }
    let (start, end) = (LEAP_TAKEOFF, mv.strike);
    if t <= start || t >= end {
        return 0.0;
    }
    let u = (t - start) / (end - start);
    (u * std::f32::consts::PI).sin() * 1.5
}
/// When a leap leaves the ground (clip seconds).
pub const LEAP_TAKEOFF: f32 = 0.4;

/// The three spots a barrage marks: the player and either side of them.
pub fn barrage_points(caster: Vec3, target: Vec3) -> [Vec3; 3] {
    let toward = Vec3::new(target.x - caster.x, 0.0, target.z - caster.z).normalize_or_zero();
    let side = Vec3::new(-toward.z, 0.0, toward.x) * 2.1;
    [
        target,
        target + side - toward * 0.6,
        target - side - toward * 0.6,
    ]
}

/// Signed angle wrapped to (-π, π].
pub fn wrap(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

/// Yaw that faces local +Z toward `toward`.
pub fn yaw_to(toward: Vec3) -> f32 {
    toward.x.atan2(toward.z)
}

/// Turn toward a direction at a limited rate; returns the remaining error.
pub fn turn_toward(yaw: &mut f32, toward: Vec3, dt: f32, rate: f32) -> f32 {
    if toward.x * toward.x + toward.z * toward.z < 1e-10 {
        return 0.0;
    }
    let diff = wrap(yaw_to(toward) - *yaw);
    *yaw += diff.clamp(-rate * dt, rate * dt);
    wrap(yaw_to(toward) - *yaw).abs()
}

/// How a foe moves this frame, which picks its animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gait {
    /// Unaware: the idle loop.
    #[default]
    Idle,
    /// Aware and squared up: the weapon's guard stance.
    Guard,
    Walk,
    Run,
}

/// A blow in progress (web `actor.attack`).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Swing {
    /// Index into the foe's `moves`.
    pub index: usize,
    /// Clip time, already scaled by the foe's attack speed.
    pub t: f32,
    pub struck: bool,
    /// Where a cast will erupt (the player's feet when it began), or where
    /// a leap will land.
    pub cast_point: Vec3,
    /// A second landing already struck (the whirlwind).
    #[serde(default)]
    pub struck_again: bool,
    /// Where the foe stood when the move began (a leap's take-off).
    #[serde(default)]
    pub origin: Vec3,
}

/// Bolt flight speed, metres per second.
pub const BOLT_SPEED: f32 = 26.0;
/// A bolt only hurts if the player is still this close to where it aimed.
pub const BOLT_DODGE: f32 = 0.85;
/// A cast erupts under this radius around its point.
pub const CAST_RADIUS: f32 = 1.52;
/// Flight speed of a mage's orb, metres per second: slow enough to see.
pub const SPELL_SPEED: f32 = 10.5;
/// Violet of the grave mages' magic; the boss's burns crimson.
pub const MAGE_VIOLET: [f32; 3] = [0.62, 0.36, 1.0];
pub const BOSS_CRIMSON: [f32; 3] = [1.0, 0.22, 0.14];

/// Short-lived combat effects in world space. Gameplay owns them (bolts deal
/// their damage on arrival); the scene only draws them.
#[derive(Clone, Debug, PartialEq)]
pub enum Fx {
    /// A fletched bolt flying from the crossbow to where the player was.
    Bolt {
        from: Vec3,
        to: Vec3,
        age: f32,
        arrived: bool,
        damage: f32,
    },
    /// A burst of sparks or dust (seeded, so it is the same every frame).
    Burst {
        at: Vec3,
        color: [f32; 3],
        count: usize,
        speed: f32,
        life: f32,
        gravity: f32,
        up: f32,
        length: f32,
        seed: u32,
        age: f32,
    },
    /// A flat ring racing out over the floor.
    Shockwave { at: Vec3, radius: f32, age: f32 },
    /// The ring and cracks warning where a cast will erupt; `life` covers
    /// the wind-up and the orb's flight.
    Telegraph {
        at: Vec3,
        age: f32,
        life: f32,
        radius: f32,
    },
    /// Stone shards thrown up where a cast erupted.
    Rupture { at: Vec3, age: f32 },
    /// A mage's orb arcing from the staff to its mark; it erupts there and
    /// hurts whoever is still inside `radius`.
    Spell {
        from: Vec3,
        to: Vec3,
        age: f32,
        damage: f32,
        radius: f32,
        color: [f32; 3],
    },
    /// Rubble flying from a pillar or prop a big foe smashed.
    Debris {
        at: Vec3,
        size: f32,
        seed: u32,
        age: f32,
    },
    /// The dark circle minions claw their way out of.
    Summon { at: Vec3, age: f32 },
}

pub const RUPTURE_SECONDS: f32 = 1.1;
pub const SHOCKWAVE_SECONDS: f32 = 0.45;
pub const DEBRIS_SECONDS: f32 = 1.4;
pub const SUMMON_SECONDS: f32 = 1.6;

impl Fx {
    pub fn burst(at: Vec3, color: [f32; 3], count: usize, speed: f32, seed: u32) -> Self {
        Fx::Burst {
            at,
            color,
            count,
            speed,
            life: 0.32,
            gravity: 5.0,
            up: 0.6,
            length: 0.09,
            seed,
            age: 0.0,
        }
    }
    /// Dust kicked up where a heavy blow meets the floor.
    pub fn dust(at: Vec3, seed: u32) -> Self {
        Fx::Burst {
            at,
            color: [0.55, 0.48, 0.4],
            count: 16,
            speed: 2.2,
            life: 0.5,
            gravity: 3.5,
            up: 0.95,
            length: 0.05,
            seed,
            age: 0.0,
        }
    }

    /// Seconds an orb takes from the staff to its mark.
    pub fn spell_travel(from: Vec3, to: Vec3) -> f32 {
        (to - from).length() / SPELL_SPEED
    }

    /// Advance; returns the damage landing on the player this frame (a bolt
    /// or orb that finds them still there), whether the effect lives on,
    /// and any effect it leaves behind.
    pub fn tick(&mut self, dt: f32, player: Vec3) -> (f32, bool, Option<Fx>) {
        let player_chest = player + Vec3::Y * 1.15;
        if let Fx::Spell {
            from,
            to,
            age,
            damage,
            radius,
            ..
        } = self
        {
            *age += dt;
            if *age < Fx::spell_travel(*from, *to) {
                return (0.0, true, None);
            }
            let flat = Vec3::new(player.x - to.x, 0.0, player.z - to.z).length();
            let hurt = if flat < *radius && (player.y - to.y).abs() < 1.4 {
                *damage
            } else {
                0.0
            };
            return (hurt, false, Some(Fx::Rupture { at: *to, age: 0.0 }));
        }
        let (hurt, alive) = match self {
            Fx::Bolt {
                from,
                to,
                age,
                arrived,
                damage,
            } => {
                *age += dt;
                let travel = (*to - *from).length() / BOLT_SPEED;
                let mut hurt = 0.0;
                if !*arrived && *age >= travel {
                    *arrived = true;
                    if (player_chest - *to).length() < BOLT_DODGE {
                        hurt = *damage;
                    }
                }
                (hurt, *age < travel + 0.3)
            }
            Fx::Burst { life, age, .. } => {
                *age += dt;
                (0.0, *age < *life)
            }
            Fx::Shockwave { age, .. } => {
                *age += dt;
                (0.0, *age < SHOCKWAVE_SECONDS)
            }
            Fx::Telegraph { age, life, .. } => {
                *age += dt;
                (0.0, *age < *life)
            }
            Fx::Rupture { age, .. } => {
                *age += dt;
                (0.0, *age < RUPTURE_SECONDS)
            }
            Fx::Debris { age, .. } => {
                *age += dt;
                (0.0, *age < DEBRIS_SECONDS)
            }
            Fx::Summon { age, .. } => {
                *age += dt;
                (0.0, *age < SUMMON_SECONDS)
            }
            Fx::Spell { .. } => unreachable!("handled above"),
        };
        (hurt, alive, None)
    }
}

/// Position of an orb `age` seconds after it left: a low arc to its mark.
pub fn spell_position(from: Vec3, to: Vec3, age: f32) -> Vec3 {
    let u = (age / Fx::spell_travel(from, to).max(1e-3)).clamp(0.0, 1.0);
    from + (to - from) * u + Vec3::Y * ((u * std::f32::consts::PI).sin() * 1.1)
}

/// Position of a bolt `age` seconds after it left.
pub fn bolt_position(from: Vec3, to: Vec3, age: f32) -> Vec3 {
    from + (to - from).normalize_or_zero() * (age * BOLT_SPEED)
}

/// Deterministic unit random in [0, 1) for effect particles.
pub fn unit(seed: u32, k: u32) -> f32 {
    let mut x = seed ^ k.wrapping_mul(0x9E37_79B9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Particle segments (head, tail) of a burst at its age (web `Burst`).
#[allow(clippy::too_many_arguments)]
pub fn burst_segments(
    at: Vec3,
    count: usize,
    speed: f32,
    gravity: f32,
    up: f32,
    length: f32,
    seed: u32,
    age: f32,
) -> Vec<(Vec3, Vec3)> {
    (0..count as u32)
        .map(|i| {
            let r = |k: u32| unit(seed, i * 8 + k);
            let dir = Vec3::new(
                r(0) * 2.0 - 1.0,
                r(1) * up + (1.0 - up) * (r(2) * 2.0 - 1.0),
                r(3) * 2.0 - 1.0,
            )
            .normalize_or_zero();
            let v0 = dir * (speed * (0.45 + r(4) * 0.8));
            // Closed form of the web's per-frame integration.
            let p = at + v0 * age - Vec3::Y * (0.5 * gravity * age * age);
            let v = v0 - Vec3::Y * (gravity * age);
            let tail = p - v * (length / v.length().max(0.5));
            (p, tail)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_table_matches_the_authored_attacks() {
        use super::super::anim::attacks::{attack_set, Loadout as AnimLoadout};
        for champion in [false, true] {
            for (weapon, offhand, melee) in [
                ("blade", None, false),
                ("axe", Some("shield"), false),
                ("axe", None, false),
                ("crossbow", None, false),
                ("staff", None, false),
                ("staff", None, true),
            ] {
                let table = moves_for(
                    Loadout {
                        model: "",
                        weapon,
                        offhand,
                    },
                    melee,
                    champion,
                );
                let (_, defs) = attack_set(AnimLoadout { weapon, offhand }, melee, champion);
                assert_eq!(table.len(), defs.len());
                for (m, d) in table.iter().zip(&defs) {
                    assert_eq!(m.name, d.name);
                    assert_eq!(m.strike, d.strike as f32);
                    assert_eq!(m.reach, d.reach as f32);
                    assert_eq!(m.arc, d.arc as f32);
                    assert_eq!(m.damage, d.damage as f32);
                    assert_eq!(m.ranged, d.ranged);
                    assert_eq!(m.shockwave, d.shockwave.map(|s| s as f32));
                    assert_eq!(m.duration, d.keys.last().unwrap().t as f32);
                }
            }
        }
    }

    #[test]
    fn loadouts_follow_the_web_roster() {
        assert_eq!(loadout("lurker", SpawnKind::Boss, 0).weapon, "staff");
        assert!(melee_staff("lurker", SpawnKind::Boss));
        assert!(!melee_staff("grave_mage", SpawnKind::Enemy));
        assert_eq!(
            loadout("bone_scout", SpawnKind::Enemy, 0).weapon,
            "crossbow"
        );
        assert_eq!(
            loadout("sentinel", SpawnKind::Enemy, 0).offhand,
            Some("shield")
        );
        assert_eq!(
            loadout("lurker", SpawnKind::Elite, 0),
            Loadout {
                model: "rogue",
                weapon: "blade",
                offhand: None
            }
        );
        assert_eq!(loadout("lurker", SpawnKind::Enemy, 3).model, "rogue");
        assert_eq!(loadout("lurker", SpawnKind::Enemy, 4).model, "minion");
    }

    #[test]
    fn turning_is_rate_limited_and_reports_the_error() {
        let mut yaw = 0.0;
        let error = turn_toward(&mut yaw, Vec3::new(-1.0, 0.0, 0.0), 0.1, 4.5);
        assert!((yaw + 0.45).abs() < 1e-5);
        assert!((error - (std::f32::consts::FRAC_PI_2 - 0.45)).abs() < 1e-4);
    }

    #[test]
    fn a_bolt_hurts_only_a_player_who_stayed_put() {
        let aim = Vec3::new(0.0, 1.15, 13.0);
        let mut bolt = Fx::Bolt {
            from: Vec3::new(0.0, 1.3, 0.0),
            to: aim,
            age: 0.0,
            arrived: false,
            damage: 8.0,
        };
        let (mut hurt, mut alive) = (0.0, true);
        while alive {
            let (h, a, _) = bolt.tick(1.0 / 60.0, aim + Vec3::X * 0.3 - Vec3::Y * 1.15);
            hurt += h;
            alive = a;
        }
        assert_eq!(hurt, 8.0);
        let mut dodged = Fx::Bolt {
            from: Vec3::new(0.0, 1.3, 0.0),
            to: aim,
            age: 0.0,
            arrived: false,
            damage: 8.0,
        };
        let mut total = 0.0;
        for _ in 0..120 {
            total += dodged
                .tick(1.0 / 60.0, aim + Vec3::X * 1.2 - Vec3::Y * 1.15)
                .0;
        }
        assert_eq!(total, 0.0);
    }
}
