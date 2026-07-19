//! # velvet-action
//!
//! Top-down action: weapons, projectiles, perception, enemies, score, combos,
//! dash, arena waves, hitstop, quick restart.

#![deny(missing_docs)]

mod arena;
mod combat;
mod combo;
mod dash;
mod enemy;
mod hitstop;
mod perception;
mod plugin;
mod projectile;
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
pub use perception::{hear, see_target, Perception, PerceptionConfig};
pub use plugin::ActionPlugin;
pub use projectile::{Projectile, ProjectileSystem};
pub use score::{ComboState, ScoreBoard};
pub use story_host::{finish_combat, CombatHostState, CombatStoryHost};
pub use weapon::{Weapon, WeaponId, WeaponKind};
