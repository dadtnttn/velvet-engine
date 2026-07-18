//! # velvet-rpg
//!
//! RPG systems layered on Velvet Play: stats, inventory, equipment, leveling,
//! dialogue bridge, quests, party, shops.

#![deny(missing_docs)]

mod dialogue_bridge;
mod equipment;
mod inventory;
mod item;
mod leveling;
mod party;
mod plugin;
mod quest;
mod shop;
mod stats;

pub mod prelude;

pub use dialogue_bridge::{
    DialogueBridge, DialogueBridgeError, DialogueGate, DialogueMapping, DialogueResolveContext,
};
pub use equipment::{armor_def, EffectiveStats, EquipmentError, EquipmentLoadout, StatModifiers};
pub use inventory::{Inventory, InventoryError};
pub use item::{EquipSlot, ItemDef, ItemId, ItemInstance, ItemKind};
pub use leveling::{LevelUpResult, LevelingSystem, StatGrowth, XpCurve};
pub use party::{Party, PartyMember};
pub use plugin::RpgPlugin;
pub use quest::{Quest, QuestId, QuestJournal, QuestObjective, QuestStatus};
pub use shop::{Shop, ShopError};
pub use stats::{Attributes, LevelProgress, StatBlock};
