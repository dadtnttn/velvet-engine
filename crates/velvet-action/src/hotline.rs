//! **Tools** for top-down aim, fragility, loadouts, hitscan/melee, pickups.
//!
//! Not a finished Hotline Miami game. Compose these in your loop, or see
//! [`crate::recipes::RoomRun`] for an optional sample room glue.
//!
//! - [`AimFacing`] — aim independent of move  
//! - [`Fragility`] / [`apply_fragile_damage`] — damage rules you configure  
//! - [`WeaponLoadout`] / [`GroundWeapon`] / pickup & throw  
//! - [`try_attack`] / [`hitscan_first`] — combat queries  
//! - [`WeaponKits`] — free functions that return [`Weapon`] data (not a mode)

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;
use velvet_play::Health;

use crate::combat::{apply_damage, melee_targets, DamageEvent, DeathEvent};
use crate::weapon::{Weapon, WeaponKind};

/// How the player dies / takes damage (**tool config**).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fragility {
    /// Any positive damage kills.
    pub one_hit: bool,
    /// Damage multiplier when not one-hit.
    pub damage_mul: f32,
}

impl Default for Fragility {
    fn default() -> Self {
        Self {
            one_hit: true,
            damage_mul: 1.0,
        }
    }
}

impl Fragility {
    /// One-hit config.
    pub fn one_hit() -> Self {
        Self::default()
    }

    /// Multiplier-only config.
    pub fn brittle(mul: f32) -> Self {
        Self {
            one_hit: false,
            damage_mul: mul.max(1.0),
        }
    }
}

/// Aim direction independent of locomotion (**tool**).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AimFacing {
    /// Facing vector.
    pub dir: Vec2,
}

impl Default for AimFacing {
    fn default() -> Self {
        Self {
            dir: Vec2::new(1.0, 0.0),
        }
    }
}

impl AimFacing {
    /// From vector.
    pub fn from_dir(dir: Vec2) -> Self {
        let n = dir.normalize_or_zero();
        Self {
            dir: if n.length_squared() < 1e-8 {
                Vec2::new(1.0, 0.0)
            } else {
                n
            },
        }
    }

    /// From angle radians.
    pub fn from_angle(radians: f32) -> Self {
        Self {
            dir: Vec2::new(radians.cos(), radians.sin()),
        }
    }

    /// Look at target.
    pub fn look_at(origin: Vec2, target: Vec2) -> Self {
        Self::from_dir(target - origin)
    }

    /// Unit facing.
    pub fn unit(&self) -> Vec2 {
        let n = self.dir.normalize_or_zero();
        if n.length_squared() < 1e-8 {
            Vec2::new(1.0, 0.0)
        } else {
            n
        }
    }

    /// Angle.
    pub fn angle(&self) -> f32 {
        let u = self.unit();
        u.y.atan2(u.x)
    }
}

/// Weapon on the ground (**tool data**).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundWeapon {
    /// Id.
    pub id: u64,
    /// Position.
    pub pos: Vec2,
    /// Weapon.
    pub weapon: Weapon,
}

/// Hands + optional held weapon (**tool**).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeaponLoadout {
    /// Bare hands fallback.
    pub fists: Weapon,
    /// Held weapon.
    pub held: Option<Weapon>,
}

/// Old name — same as [`WeaponLoadout`].
pub type HotlineLoadout = WeaponLoadout;

impl Default for WeaponLoadout {
    fn default() -> Self {
        Self {
            fists: Weapon::melee("fists", 15.0, 28.0, 0.28),
            held: None,
        }
    }
}

impl WeaponLoadout {
    /// With starting weapon.
    pub fn with_held(weapon: Weapon) -> Self {
        Self {
            held: Some(weapon),
            ..Default::default()
        }
    }

    /// Active weapon.
    pub fn active(&self) -> &Weapon {
        self.held.as_ref().unwrap_or(&self.fists)
    }

    /// Active mut.
    pub fn active_mut(&mut self) -> &mut Weapon {
        if self.held.is_some() {
            self.held.as_mut().unwrap()
        } else {
            &mut self.fists
        }
    }

    /// Tick cooldowns.
    pub fn tick(&mut self, dt: f32) {
        self.fists.tick(dt);
        if let Some(w) = self.held.as_mut() {
            w.tick(dt);
        }
    }

    /// Holding non-fist.
    pub fn has_weapon(&self) -> bool {
        self.held.is_some()
    }
}

/// Free functions that build [`Weapon`] values — **data kits**, not a game mode.
pub struct WeaponKits;

/// Old name.
pub type HotlinePresets = WeaponKits;

impl WeaponKits {
    /// Bat stats.
    pub fn bat() -> Weapon {
        let mut w = Weapon::melee("bat", 45.0, 42.0, 0.38);
        w.knockback = 140.0;
        w
    }

    /// Knife stats.
    pub fn knife() -> Weapon {
        let mut w = Weapon::melee("knife", 55.0, 32.0, 0.22);
        w.knockback = 40.0;
        w
    }

    /// Pistol stats.
    pub fn pistol() -> Weapon {
        Weapon::pistol("hm_pistol")
    }

    /// Uzi-like stats.
    pub fn uzi() -> Weapon {
        Weapon {
            id: crate::weapon::WeaponId("uzi".into()),
            kind: WeaponKind::Projectile,
            damage: 12.0,
            range: 280.0,
            cooldown: 0.08,
            cooldown_left: 0.0,
            magazine: 24,
            ammo: 24,
            reserve: 48,
            reload_secs: 1.4,
            reload_left: 0.0,
            spread: 0.12,
            projectile_speed: 420.0,
            knockback: 20.0,
        }
    }

    /// Shotgun stats.
    pub fn shotgun() -> Weapon {
        Weapon {
            id: crate::weapon::WeaponId("shotgun".into()),
            kind: WeaponKind::Hitscan,
            damage: 18.0,
            range: 120.0,
            cooldown: 0.55,
            cooldown_left: 0.0,
            magazine: 6,
            ammo: 6,
            reserve: 18,
            reload_secs: 1.8,
            reload_left: 0.0,
            spread: 0.35,
            projectile_speed: 0.0,
            knockback: 160.0,
        }
    }
}

/// Result of fragility-aware damage.
#[derive(Debug, Clone, PartialEq)]
pub struct FragilityHit {
    /// Damage event.
    pub damage: DamageEvent,
    /// Death if any.
    pub death: Option<DeathEvent>,
    /// Forced one-hit.
    pub one_hit_kill: bool,
}

/// Apply damage with fragility tool config.
pub fn apply_fragile_damage(
    health: &mut Health,
    fragility: &Fragility,
    target: usize,
    source: usize,
    amount: f32,
    point: Vec2,
) -> FragilityHit {
    let mut one_hit_kill = false;
    let amount = if fragility.one_hit && amount > 0.0 {
        one_hit_kill = true;
        health.current.max(1.0)
    } else {
        amount * fragility.damage_mul
    };
    let (damage, death) = apply_damage(health, target, source, amount, point);
    FragilityHit {
        damage,
        death,
        one_hit_kill,
    }
}

/// Hitscan first hit along aim (**tool**).
pub fn hitscan_first(
    origin: Vec2,
    aim: Vec2,
    range: f32,
    candidates: &[(usize, Vec2)],
    ignore: usize,
) -> Option<(usize, Vec2, f32)> {
    let dir = aim.normalize_or_zero();
    if dir.length_squared() < 1e-8 {
        return None;
    }
    let mut best: Option<(usize, Vec2, f32)> = None;
    for (id, pos) in candidates {
        if *id == ignore {
            continue;
        }
        let offset = *pos - origin;
        let dist = offset.length();
        if dist > range || dist < 1e-4 {
            continue;
        }
        let along = offset.dot(dir);
        if along <= 0.0 || along > range {
            continue;
        }
        let closest = origin + dir * along;
        let lateral = (*pos - closest).length();
        if lateral > 14.0 {
            continue;
        }
        if best.map(|(_, _, d)| dist < d).unwrap_or(true) {
            best = Some((*id, *pos, dist));
        }
    }
    best
}

/// Attack query result (**tool**).
#[derive(Debug, Clone, PartialEq)]
pub enum AttackOutcome {
    /// No fire.
    Missed,
    /// Melee hits.
    MeleeHits {
        /// Targets.
        targets: Vec<usize>,
        /// Damage.
        damage: f32,
    },
    /// Hitscan hit.
    HitscanHit {
        /// Target.
        target: usize,
        /// Point.
        point: Vec2,
        /// Damage.
        damage: f32,
    },
    /// Shot without hit.
    ShotEmpty {
        /// Kind.
        kind: WeaponKind,
    },
    /// Caller spawns projectile.
    SpawnProjectile {
        /// Damage.
        damage: f32,
        /// Speed.
        speed: f32,
        /// Dir.
        dir: Vec2,
    },
}

/// Fire/swing with loadout + aim tools.
pub fn try_attack(
    loadout: &mut WeaponLoadout,
    origin: Vec2,
    aim: &AimFacing,
    candidates: &[(usize, Vec2)],
    attacker: usize,
    melee_half_arc: f32,
) -> AttackOutcome {
    let unit = aim.unit();
    let kind = loadout.active().kind;
    let range = loadout.active().range;
    let damage = loadout.active().damage;
    let speed = loadout.active().projectile_speed;

    if !loadout.active_mut().fire() {
        return AttackOutcome::Missed;
    }

    match kind {
        WeaponKind::Melee => {
            let ids = melee_targets(origin, unit, range, melee_half_arc, candidates);
            if ids.is_empty() {
                AttackOutcome::Missed
            } else {
                AttackOutcome::MeleeHits {
                    targets: ids,
                    damage,
                }
            }
        }
        WeaponKind::Hitscan => {
            if let Some((tid, point, _)) = hitscan_first(origin, unit, range, candidates, attacker)
            {
                AttackOutcome::HitscanHit {
                    target: tid,
                    point,
                    damage,
                }
            } else {
                AttackOutcome::ShotEmpty { kind }
            }
        }
        WeaponKind::Projectile => AttackOutcome::SpawnProjectile {
            damage,
            speed,
            dir: unit,
        },
    }
}

/// Default pickup radius.
pub const PICKUP_RADIUS: f32 = 22.0;

/// Pickup nearest ground weapon (**tool**).
pub fn try_pickup(
    loadout: &mut WeaponLoadout,
    drops: &mut Vec<GroundWeapon>,
    player_pos: Vec2,
    radius: f32,
) -> Option<u64> {
    let mut best: Option<(usize, f32)> = None;
    for (i, d) in drops.iter().enumerate() {
        let dist = (d.pos - player_pos).length();
        if dist <= radius && best.map(|(_, bd)| dist < bd).unwrap_or(true) {
            best = Some((i, dist));
        }
    }
    let (idx, _) = best?;
    let drop = drops.remove(idx);
    if let Some(held) = loadout.held.take() {
        drops.push(GroundWeapon {
            id: drop.id.wrapping_add(10_000),
            pos: player_pos,
            weapon: held,
        });
    }
    loadout.held = Some(drop.weapon);
    Some(drop.id)
}

/// Throw held weapon (**tool**).
pub fn throw_held(
    loadout: &mut WeaponLoadout,
    drops: &mut Vec<GroundWeapon>,
    player_pos: Vec2,
    aim: &AimFacing,
    next_id: &mut u64,
) -> Option<GroundWeapon> {
    let weapon = loadout.held.take()?;
    let pos = player_pos + aim.unit() * 28.0;
    let id = *next_id;
    *next_id = next_id.wrapping_add(1);
    let drop = GroundWeapon { id, pos, weapon };
    drops.push(drop.clone());
    Some(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_hitscan_and_fragility() {
        let mut loadout = WeaponLoadout::with_held(WeaponKits::pistol());
        let aim = AimFacing::from_dir(Vec2::new(1.0, 0.0));
        let out = try_attack(
            &mut loadout,
            Vec2::ZERO,
            &aim,
            &[(2, Vec2::new(80.0, 1.0))],
            1,
            0.5,
        );
        assert!(matches!(out, AttackOutcome::HitscanHit { target: 2, .. }));

        let mut hp = Health::full(100.0);
        let hit = apply_fragile_damage(
            &mut hp,
            &Fragility::one_hit(),
            1,
            9,
            1.0,
            Vec2::ZERO,
        );
        assert!(hit.one_hit_kill);
        assert!(hit.death.is_some());
    }

    #[test]
    fn pickup_throw_tools() {
        let mut loadout = WeaponLoadout::default();
        let mut drops = vec![GroundWeapon {
            id: 1,
            pos: Vec2::new(5.0, 0.0),
            weapon: WeaponKits::knife(),
        }];
        let id = try_pickup(&mut loadout, &mut drops, Vec2::ZERO, PICKUP_RADIUS).unwrap();
        assert_eq!(id, 1);
        assert!(loadout.has_weapon());
        let mut next = 2u64;
        let aim = AimFacing::from_dir(Vec2::new(1.0, 0.0));
        throw_held(&mut loadout, &mut drops, Vec2::ZERO, &aim, &mut next).unwrap();
        assert!(!loadout.has_weapon());
    }

    #[test]
    fn aim_tool() {
        let aim = AimFacing::look_at(Vec2::ZERO, Vec2::new(0.0, 10.0));
        assert!(aim.unit().y > 0.9);
    }
}
