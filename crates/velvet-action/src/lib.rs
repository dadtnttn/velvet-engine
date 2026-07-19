//! # velvet-action
//!
//! **Tools** for top-down action: weapons, projectiles, perception, enemies,
//! score, combos, dash, arena, hitstop, aim/loadout/hitscan.
//!
//! Optional room-loop glue lives in [`recipes`] (not required).
//! Demos (`examples/hotline-rush`) show composition — they are not the API.

#![deny(missing_docs)]

mod arena;
mod combat;
mod combo;
mod dash;
mod enemy;
mod hitstop;
mod hotline;
mod perception;
mod plugin;
mod projectile;
/// Optional sample compositions of tools.
pub mod recipes;
mod score;
mod story_host;
mod weapon;

pub mod prelude;

pub use arena::{ArenaController, ArenaPhase, SpawnRequest, WaveDef};
pub use combat::{apply_damage, DamageEvent, DeathEvent};
pub use combo::{AttackCombo, ComboEvent, ComboInput, ComboPhase, ComboStep};
pub use dash::{DashConfig, DashState};
pub use enemy::{EnemyAi, EnemyKind, PatrolPath};
pub use hitstop::{Hitstop, HitstopConfig};
pub use hotline::{
    apply_fragile_damage, hitscan_first, throw_held, try_attack, try_pickup, AimFacing,
    AttackOutcome, Fragility, FragilityHit, GroundWeapon, HotlineLoadout, HotlinePresets,
    WeaponKits, WeaponLoadout, PICKUP_RADIUS,
};
// Recipe re-exports for older demos (prefer tools + your own loop).
pub use recipes::{HotlinePhase, HotlineRun, KillStyle, RoomPhase, RoomRun};
pub use perception::{hear, see_target, Perception, PerceptionConfig};
pub use plugin::ActionPlugin;
pub use projectile::{Projectile, ProjectileSystem};
pub use score::{ComboState, ScoreBoard};
pub use story_host::{finish_combat, CombatHostState, CombatStoryHost};
pub use weapon::{Weapon, WeaponId, WeaponKind};
