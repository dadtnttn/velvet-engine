//! Item definitions and instances.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Stable item definition id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub String);

impl ItemId {
    /// Create.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Item category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    /// Misc.
    Misc,
    /// Consumable.
    Consumable,
    /// Weapon.
    Weapon,
    /// Armor.
    Armor,
    /// Key item.
    Key,
}

/// Equipment slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipSlot {
    /// Main hand.
    MainHand,
    /// Off hand.
    OffHand,
    /// Head.
    Head,
    /// Body.
    Body,
    /// Accessory.
    Accessory,
}

/// Static item definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDef {
    /// Id.
    pub id: ItemId,
    /// Display name.
    pub name: String,
    /// Kind.
    pub kind: ItemKind,
    /// Equip slot if equipment.
    pub equip_slot: Option<EquipSlot>,
    /// Max stack.
    pub max_stack: u32,
    /// Buy price.
    pub price: u32,
    /// Attack bonus.
    pub attack: f32,
    /// Defense bonus.
    pub defense: f32,
    /// Heal amount for consumables.
    pub heal: f32,
    /// Description.
    pub description: String,
}

impl ItemDef {
    /// Simple consumable.
    pub fn potion(id: &str, name: &str, heal: f32, price: u32) -> Self {
        Self {
            id: ItemId::new(id),
            name: name.into(),
            kind: ItemKind::Consumable,
            equip_slot: None,
            max_stack: 99,
            price,
            attack: 0.0,
            defense: 0.0,
            heal,
            description: format!("Restores {heal} HP"),
        }
    }

    /// Weapon.
    pub fn weapon(id: &str, name: &str, attack: f32, price: u32) -> Self {
        Self {
            id: ItemId::new(id),
            name: name.into(),
            kind: ItemKind::Weapon,
            equip_slot: Some(EquipSlot::MainHand),
            max_stack: 1,
            price,
            attack,
            defense: 0.0,
            heal: 0.0,
            description: name.into(),
        }
    }
}

/// Instance in inventory (stack or unique).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemInstance {
    /// Definition id.
    pub def_id: ItemId,
    /// Stack count.
    pub count: u32,
}

impl ItemInstance {
    /// Create stack.
    pub fn stack(def_id: impl Into<String>, count: u32) -> Self {
        Self {
            def_id: ItemId::new(def_id),
            count,
        }
    }
}

/// Item database.
#[derive(Debug, Clone, Default)]
pub struct ItemDb {
    /// Definitions.
    pub defs: IndexMap<String, ItemDef>,
}

impl ItemDb {
    /// Insert.
    pub fn insert(&mut self, def: ItemDef) {
        self.defs.insert(def.id.0.clone(), def);
    }

    /// Get.
    pub fn get(&self, id: &str) -> Option<&ItemDef> {
        self.defs.get(id)
    }
}
