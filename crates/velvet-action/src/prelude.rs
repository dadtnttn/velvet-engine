//! Action prelude.

pub use crate::arena::{ArenaController, ArenaPhase, WaveDef};
pub use crate::combat::{apply_damage, melee_targets, DamageEvent, DeathEvent};
pub use crate::combo::{AttackCombo, ComboInput, ComboStep};
pub use crate::dash::{DashConfig, DashState};
pub use crate::enemy::{EnemyAi, EnemyKind, PatrolPath};
pub use crate::hitstop::{Hitstop, HitstopConfig};
pub use crate::perception::{hear, see_target, Perception, PerceptionConfig};
pub use crate::plugin::ActionPlugin;
pub use crate::projectile::{Projectile, ProjectileSystem};
pub use crate::score::{ComboState, ScoreBoard};
pub use crate::weapon::{Weapon, WeaponId, WeaponKind};
