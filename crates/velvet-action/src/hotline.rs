//! Top-down action shooter helpers (Hotline Miami–like).
//!
//! Genre names: **top-down action shooter**, **neo-noir top-down shooter**,
//! sometimes loosely "twin-stick" (this path prefers **move + free aim** like Hotline).
//!
//! Pre-alpha product spine:
//! - fragile player (default one-hit death)
//! - aim facing independent of movement
//! - melee cone + hitscan guns
//! - ground weapon drops / pickup / throw
//! - room clear + instant restart on death

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;
use velvet_play::Health;

use crate::combat::{apply_damage, melee_targets, DamageEvent, DeathEvent};
use crate::score::ScoreBoard;
use crate::weapon::{Weapon, WeaponKind};

/// Run / mission phase for a single room or floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotlinePhase {
    /// Player is fighting.
    Playing,
    /// Player died (waiting for quick restart).
    Dead,
    /// All hostiles down.
    Cleared,
}

/// How the player dies / takes damage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fragility {
    /// Any positive damage kills the player.
    pub one_hit: bool,
    /// Extra damage multiplier when not one-hit (default 1.0).
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
    /// Classic Hotline-style glass cannon.
    pub fn one_hit() -> Self {
        Self::default()
    }

    /// Slightly more forgiving (still brittle).
    pub fn brittle(mul: f32) -> Self {
        Self {
            one_hit: false,
            damage_mul: mul.max(1.0),
        }
    }
}

/// Aim direction independent of locomotion (mouse / right stick).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AimFacing {
    /// Facing vector (not required unit; helpers normalize).
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
    /// From a free vector.
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

    /// From angle in radians (0 = +X, CCW).
    pub fn from_angle(radians: f32) -> Self {
        Self {
            dir: Vec2::new(radians.cos(), radians.sin()),
        }
    }

    /// Point at world position from origin.
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

    /// Angle in radians.
    pub fn angle(&self) -> f32 {
        let u = self.unit();
        u.y.atan2(u.x)
    }
}

/// Weapon lying on the floor for pickup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundWeapon {
    /// Stable id within the run.
    pub id: u64,
    /// World position.
    pub pos: Vec2,
    /// Weapon payload.
    pub weapon: Weapon,
}

/// Player loadout: empty hands (fists) or a held weapon.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotlineLoadout {
    /// Bare-handed fallback (always present).
    pub fists: Weapon,
    /// Currently held gun/melee (None = fists).
    pub held: Option<Weapon>,
}

impl Default for HotlineLoadout {
    fn default() -> Self {
        Self {
            fists: Weapon::melee("fists", 15.0, 28.0, 0.28),
            held: None,
        }
    }
}

impl HotlineLoadout {
    /// New loadout with optional starting weapon.
    pub fn with_held(weapon: Weapon) -> Self {
        Self {
            held: Some(weapon),
            ..Default::default()
        }
    }

    /// Active weapon reference.
    pub fn active(&self) -> &Weapon {
        self.held.as_ref().unwrap_or(&self.fists)
    }

    /// Active weapon mut.
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

    /// Whether holding a non-fist weapon.
    pub fn has_weapon(&self) -> bool {
        self.held.is_some()
    }
}

/// Preset kits for Hotline-style rooms.
pub struct HotlinePresets;

impl HotlinePresets {
    /// Baseball bat.
    pub fn bat() -> Weapon {
        let mut w = Weapon::melee("bat", 45.0, 42.0, 0.38);
        w.knockback = 140.0;
        w
    }

    /// Combat knife.
    pub fn knife() -> Weapon {
        let mut w = Weapon::melee("knife", 55.0, 32.0, 0.22);
        w.knockback = 40.0;
        w
    }

    /// Compact pistol.
    pub fn pistol() -> Weapon {
        Weapon::pistol("hm_pistol")
    }

    /// Fast SMG-ish spray.
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

    /// Close-range shotgun (hitscan cone, multi-pellet handled by caller).
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

/// How a kill was scored (style hooks for later juice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KillStyle {
    /// Melee finish.
    Melee,
    /// Firearm.
    Gun,
    /// Environmental / other.
    Other,
}

/// Result of applying damage under fragility rules.
#[derive(Debug, Clone, PartialEq)]
pub struct FragilityHit {
    /// Damage event applied.
    pub damage: DamageEvent,
    /// Death if lethal.
    pub death: Option<DeathEvent>,
    /// True if fragility forced a one-hit kill.
    pub one_hit_kill: bool,
}

/// Apply damage with player fragility rules.
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

/// Hitscan along aim: first target within range, sorted by distance.
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
        // Project onto aim ray; require near-line (generous for pre-alpha).
        let along = offset.dot(dir);
        if along <= 0.0 || along > range {
            continue;
        }
        let closest = origin + dir * along;
        let lateral = (*pos - closest).length();
        // ~half-body width acceptance
        if lateral > 14.0 {
            continue;
        }
        if best.map(|(_, _, d)| dist < d).unwrap_or(true) {
            best = Some((*id, *pos, dist));
        }
    }
    best
}

/// Attempt a primary attack with the active weapon.
#[derive(Debug, Clone, PartialEq)]
pub enum AttackOutcome {
    /// Nothing (cooldown / empty).
    Missed,
    /// Melee swing produced hits.
    MeleeHits {
        /// Hit target ids.
        targets: Vec<usize>,
        /// Damage per target.
        damage: f32,
    },
    /// Hitscan connected.
    HitscanHit {
        /// Target id.
        target: usize,
        /// Hit point.
        point: Vec2,
        /// Damage.
        damage: f32,
    },
    /// Hitscan / projectile shot with no hit (still consumes ammo).
    ShotEmpty {
        /// Kind fired.
        kind: WeaponKind,
    },
    /// Projectile weapons: caller should spawn a projectile.
    SpawnProjectile {
        /// Damage.
        damage: f32,
        /// Speed.
        speed: f32,
        /// Aim unit.
        dir: Vec2,
    },
}

/// Fire / swing the active loadout weapon toward `aim`.
pub fn try_attack(
    loadout: &mut HotlineLoadout,
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

/// Pickup radius default (pixels / world units).
pub const PICKUP_RADIUS: f32 = 22.0;

/// Pick nearest ground weapon if in range; returns the weapon id removed.
pub fn try_pickup(
    loadout: &mut HotlineLoadout,
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
    // Drop currently held if any.
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

/// Throw held weapon forward; becomes a ground drop (and mild projectile flavor later).
pub fn throw_held(
    loadout: &mut HotlineLoadout,
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

/// Single-room / floor run controller (pre-alpha).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotlineRun {
    /// Phase.
    pub phase: HotlinePhase,
    /// Score / combo.
    pub score: ScoreBoard,
    /// How many times the player restarted this room.
    pub restarts: u32,
    /// Hostile count remaining (caller keeps in sync or uses recompute).
    pub hostiles_alive: u32,
    /// Player fragility.
    pub fragility: Fragility,
    /// Loadout.
    pub loadout: HotlineLoadout,
    /// Free aim.
    pub aim: AimFacing,
    /// Floor drops.
    pub drops: Vec<GroundWeapon>,
    /// Next drop id.
    pub next_drop_id: u64,
    /// Half-arc for melee (radians).
    pub melee_half_arc: f32,
}

impl Default for HotlineRun {
    fn default() -> Self {
        Self {
            phase: HotlinePhase::Playing,
            score: ScoreBoard::default(),
            restarts: 0,
            hostiles_alive: 0,
            fragility: Fragility::one_hit(),
            loadout: HotlineLoadout::default(),
            aim: AimFacing::default(),
            drops: Vec::new(),
            next_drop_id: 1,
            melee_half_arc: std::f32::consts::FRAC_PI_3,
        }
    }
}

impl HotlineRun {
    /// Start a room with N hostiles and optional starting weapon.
    pub fn start_room(hostiles: u32, starting: Option<Weapon>) -> Self {
        let mut run = Self {
            hostiles_alive: hostiles,
            phase: if hostiles == 0 {
                HotlinePhase::Cleared
            } else {
                HotlinePhase::Playing
            },
            ..Default::default()
        };
        if let Some(w) = starting {
            run.loadout.held = Some(w);
        }
        run
    }

    /// Tick cooldowns / combo windows.
    pub fn tick(&mut self, dt: f32) {
        if self.phase != HotlinePhase::Playing {
            return;
        }
        self.loadout.tick(dt);
        self.score.tick(dt);
    }

    /// Set aim toward a world point.
    pub fn aim_at(&mut self, origin: Vec2, target: Vec2) {
        self.aim = AimFacing::look_at(origin, target);
    }

    /// Register a kill with style scoring.
    pub fn register_kill(&mut self, style: KillStyle) {
        let base = match style {
            KillStyle::Melee => 150,
            KillStyle::Gun => 100,
            KillStyle::Other => 80,
        };
        self.score.add_kill(base);
        self.hostiles_alive = self.hostiles_alive.saturating_sub(1);
        if self.hostiles_alive == 0 && self.phase == HotlinePhase::Playing {
            self.phase = HotlinePhase::Cleared;
            // Clear bonus
            self.score.score += 500;
        }
    }

    /// Player took a hit; may enter Dead.
    pub fn player_hit(
        &mut self,
        health: &mut Health,
        source: usize,
        amount: f32,
        point: Vec2,
        player_id: usize,
    ) -> FragilityHit {
        let hit = apply_fragile_damage(
            health,
            &self.fragility,
            player_id,
            source,
            amount,
            point,
        );
        if hit.death.is_some() {
            self.phase = HotlinePhase::Dead;
            self.score.add_death();
        }
        hit
    }

    /// Quick restart: same room hostile count, wipe combo, keep best, new loadout optional.
    pub fn quick_restart(&mut self, hostiles: u32, starting: Option<Weapon>) {
        self.restarts = self.restarts.saturating_add(1);
        self.phase = HotlinePhase::Playing;
        self.hostiles_alive = hostiles;
        self.score.combo.count = 0;
        self.score.combo.timer = 0.0;
        self.drops.clear();
        self.loadout = if let Some(w) = starting {
            HotlineLoadout::with_held(w)
        } else {
            HotlineLoadout::default()
        };
        self.aim = AimFacing::default();
    }

    /// Attack with current loadout.
    pub fn attack(
        &mut self,
        origin: Vec2,
        candidates: &[(usize, Vec2)],
        attacker: usize,
    ) -> AttackOutcome {
        if self.phase != HotlinePhase::Playing {
            return AttackOutcome::Missed;
        }
        try_attack(
            &mut self.loadout,
            origin,
            &self.aim,
            candidates,
            attacker,
            self.melee_half_arc,
        )
    }

    /// Pickup nearest drop.
    pub fn pickup(&mut self, player_pos: Vec2) -> Option<u64> {
        if self.phase != HotlinePhase::Playing {
            return None;
        }
        try_pickup(
            &mut self.loadout,
            &mut self.drops,
            player_pos,
            PICKUP_RADIUS,
        )
    }

    /// Throw held weapon.
    pub fn throw_weapon(&mut self, player_pos: Vec2) -> Option<GroundWeapon> {
        if self.phase != HotlinePhase::Playing {
            return None;
        }
        throw_held(
            &mut self.loadout,
            &mut self.drops,
            player_pos,
            &self.aim,
            &mut self.next_drop_id,
        )
    }

    /// Spawn a ground weapon in the room.
    pub fn spawn_drop(&mut self, pos: Vec2, weapon: Weapon) -> u64 {
        let id = self.next_drop_id;
        self.next_drop_id = self.next_drop_id.wrapping_add(1);
        self.drops.push(GroundWeapon { id, pos, weapon });
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_hit_kills_player() {
        let mut run = HotlineRun::start_room(3, Some(HotlinePresets::bat()));
        let mut hp = Health::full(100.0);
        let hit = run.player_hit(&mut hp, 9, 1.0, Vec2::ZERO, 1);
        assert!(hit.one_hit_kill);
        assert!(hit.death.is_some());
        assert_eq!(run.phase, HotlinePhase::Dead);
        assert_eq!(run.score.deaths, 1);
    }

    #[test]
    fn melee_clears_and_scores() {
        let mut run = HotlineRun::start_room(2, Some(HotlinePresets::bat()));
        run.aim = AimFacing::from_dir(Vec2::new(1.0, 0.0));
        let origin = Vec2::ZERO;
        let candidates = vec![(2, Vec2::new(20.0, 0.0)), (3, Vec2::new(25.0, 5.0))];
        let out = run.attack(origin, &candidates, 1);
        match out {
            AttackOutcome::MeleeHits { targets, .. } => {
                assert!(!targets.is_empty());
                for _ in &targets {
                    run.register_kill(KillStyle::Melee);
                }
            }
            other => panic!("expected melee hits, got {other:?}"),
        }
        assert!(run.score.kills >= 1);
        assert!(run.score.score > 0);
    }

    #[test]
    fn hitscan_gun_kill_clears_room() {
        let mut run = HotlineRun::start_room(1, Some(HotlinePresets::pistol()));
        run.aim = AimFacing::from_dir(Vec2::new(1.0, 0.0));
        let candidates = vec![(2, Vec2::new(80.0, 2.0))];
        let out = run.attack(Vec2::ZERO, &candidates, 1);
        match out {
            AttackOutcome::HitscanHit { target, .. } => {
                assert_eq!(target, 2);
                run.register_kill(KillStyle::Gun);
            }
            other => panic!("expected hitscan, got {other:?}"),
        }
        assert_eq!(run.phase, HotlinePhase::Cleared);
        assert!(run.score.score >= 600); // kill + clear bonus
    }

    #[test]
    fn pickup_and_throw_cycle() {
        let mut run = HotlineRun::start_room(1, None);
        run.spawn_drop(Vec2::new(5.0, 0.0), HotlinePresets::knife());
        let id = run.pickup(Vec2::ZERO).expect("pickup");
        assert!(run.loadout.has_weapon());
        assert_eq!(id, 1);
        run.aim = AimFacing::from_dir(Vec2::new(1.0, 0.0));
        let thrown = run.throw_weapon(Vec2::ZERO).expect("throw");
        assert!(!run.loadout.has_weapon());
        assert!(thrown.pos.x > 0.0);
        assert_eq!(run.drops.len(), 1);
    }

    #[test]
    fn quick_restart_resets_phase() {
        let mut run = HotlineRun::start_room(4, Some(HotlinePresets::bat()));
        let mut hp = Health::full(50.0);
        let _ = run.player_hit(&mut hp, 2, 1.0, Vec2::ZERO, 1);
        assert_eq!(run.phase, HotlinePhase::Dead);
        run.quick_restart(4, Some(HotlinePresets::bat()));
        assert_eq!(run.phase, HotlinePhase::Playing);
        assert_eq!(run.restarts, 1);
        assert_eq!(run.hostiles_alive, 4);
        assert_eq!(run.score.combo.count, 0);
    }

    #[test]
    fn aim_look_at_faces_target() {
        let aim = AimFacing::look_at(Vec2::ZERO, Vec2::new(0.0, 10.0));
        let u = aim.unit();
        assert!(u.y > 0.9);
        assert!(u.x.abs() < 0.1);
    }
}
