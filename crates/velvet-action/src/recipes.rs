//! Optional **recipes** built from action tools — not the product API.
//!
//! Prefer composing the `hotline` tools (aim, loadout, hitscan, fragility)
//! yourself. These helpers only show one way to glue a room loop.

use serde::{Deserialize, Serialize};
use velvet_math::Vec2;
use velvet_play::Health;

use crate::hotline::{
    apply_fragile_damage, throw_held, try_attack, try_pickup, AimFacing, AttackOutcome, Fragility,
    FragilityHit, GroundWeapon, WeaponLoadout, PICKUP_RADIUS,
};
use crate::score::ScoreBoard;
use crate::weapon::Weapon;

/// Room / mission phase used by the sample room loop recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomPhase {
    /// Fighting.
    Playing,
    /// Dead.
    Dead,
    /// Cleared.
    Cleared,
}

/// Kill style scoring labels (recipe scoring only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KillStyle {
    /// Melee.
    Melee,
    /// Gun.
    Gun,
    /// Other.
    Other,
}

/// Sample single-room controller built from tools.
///
/// **Recipe**, not required: demos may use this; games can reimplement with the same tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoomRun {
    /// Phase.
    pub phase: RoomPhase,
    /// Score board tool.
    pub score: ScoreBoard,
    /// Restarts.
    pub restarts: u32,
    /// Hostiles left (caller may also track entities).
    pub hostiles_alive: u32,
    /// Fragility tool config.
    pub fragility: Fragility,
    /// Loadout tool.
    pub loadout: WeaponLoadout,
    /// Aim tool.
    pub aim: AimFacing,
    /// Ground drops.
    pub drops: Vec<GroundWeapon>,
    /// Next drop id.
    pub next_drop_id: u64,
    /// Melee half-arc radians.
    pub melee_half_arc: f32,
}

impl Default for RoomRun {
    fn default() -> Self {
        Self {
            phase: RoomPhase::Playing,
            score: ScoreBoard::default(),
            restarts: 0,
            hostiles_alive: 0,
            fragility: Fragility::one_hit(),
            loadout: WeaponLoadout::default(),
            aim: AimFacing::default(),
            drops: Vec::new(),
            next_drop_id: 1,
            melee_half_arc: std::f32::consts::FRAC_PI_3,
        }
    }
}

impl RoomRun {
    /// Start with N hostiles and optional weapon.
    pub fn start_room(hostiles: u32, starting: Option<Weapon>) -> Self {
        let mut run = Self {
            hostiles_alive: hostiles,
            phase: if hostiles == 0 {
                RoomPhase::Cleared
            } else {
                RoomPhase::Playing
            },
            ..Default::default()
        };
        if let Some(w) = starting {
            run.loadout.held = Some(w);
        }
        run
    }

    /// Tick loadout + score timers.
    pub fn tick(&mut self, dt: f32) {
        if self.phase != RoomPhase::Playing {
            return;
        }
        self.loadout.tick(dt);
        self.score.tick(dt);
    }

    /// Aim tool.
    pub fn aim_at(&mut self, origin: Vec2, target: Vec2) {
        self.aim = AimFacing::look_at(origin, target);
    }

    /// Recipe scoring on kill.
    pub fn register_kill(&mut self, style: KillStyle) {
        let base = match style {
            KillStyle::Melee => 150,
            KillStyle::Gun => 100,
            KillStyle::Other => 80,
        };
        self.score.add_kill(base);
        self.hostiles_alive = self.hostiles_alive.saturating_sub(1);
        if self.hostiles_alive == 0 && self.phase == RoomPhase::Playing {
            self.phase = RoomPhase::Cleared;
            self.score.score += 500;
        }
    }

    /// Apply fragility tool on player hit.
    pub fn player_hit(
        &mut self,
        health: &mut Health,
        source: usize,
        amount: f32,
        point: Vec2,
        player_id: usize,
    ) -> FragilityHit {
        let hit = apply_fragile_damage(health, &self.fragility, player_id, source, amount, point);
        if hit.death.is_some() {
            self.phase = RoomPhase::Dead;
            self.score.add_death();
        }
        hit
    }

    /// Restart recipe state.
    pub fn quick_restart(&mut self, hostiles: u32, starting: Option<Weapon>) {
        self.restarts = self.restarts.saturating_add(1);
        self.phase = RoomPhase::Playing;
        self.hostiles_alive = hostiles;
        self.score.combo.count = 0;
        self.score.combo.timer = 0.0;
        self.drops.clear();
        self.loadout = if let Some(w) = starting {
            WeaponLoadout::with_held(w)
        } else {
            WeaponLoadout::default()
        };
        self.aim = AimFacing::default();
    }

    /// Attack via tools.
    pub fn attack(
        &mut self,
        origin: Vec2,
        candidates: &[(usize, Vec2)],
        attacker: usize,
    ) -> AttackOutcome {
        if self.phase != RoomPhase::Playing {
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

    /// Pickup tool.
    pub fn pickup(&mut self, player_pos: Vec2) -> Option<u64> {
        if self.phase != RoomPhase::Playing {
            return None;
        }
        try_pickup(
            &mut self.loadout,
            &mut self.drops,
            player_pos,
            PICKUP_RADIUS,
        )
    }

    /// Throw tool.
    pub fn throw_weapon(&mut self, player_pos: Vec2) -> Option<GroundWeapon> {
        if self.phase != RoomPhase::Playing {
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

    /// Spawn drop.
    pub fn spawn_drop(&mut self, pos: Vec2, weapon: Weapon) -> u64 {
        let id = self.next_drop_id;
        self.next_drop_id = self.next_drop_id.wrapping_add(1);
        self.drops.push(GroundWeapon { id, pos, weapon });
        id
    }
}

/// Backward-compatible aliases (recipe names used by older demos).
pub type HotlineRun = RoomRun;
/// Alias.
pub type HotlinePhase = RoomPhase;
