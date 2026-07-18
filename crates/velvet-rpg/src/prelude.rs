//! RPG prelude.

pub use crate::dialogue_bridge::{DialogueBridge, DialogueMapping, DialogueResolveContext};
pub use crate::equipment::{EffectiveStats, EquipmentLoadout, StatModifiers};
pub use crate::inventory::{Inventory, InventoryError};
pub use crate::item::{EquipSlot, ItemDb, ItemDef, ItemId, ItemInstance, ItemKind};
pub use crate::leveling::{LevelingSystem, StatGrowth, XpCurve};
pub use crate::party::{Party, PartyMember};
pub use crate::plugin::RpgPlugin;
pub use crate::quest::{Quest, QuestJournal, QuestObjective, QuestStatus};
pub use crate::shop::Shop;
pub use crate::stats::{Attributes, LevelProgress, StatBlock};
