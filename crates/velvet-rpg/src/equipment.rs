//! Equipment slots and stat modifier application.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::item::{EquipSlot, ItemDb, ItemDef, ItemId, ItemKind};
use crate::stats::{Attributes, StatBlock};

/// Equipment errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EquipmentError {
    /// Item missing from database.
    #[error("item not found: {0}")]
    NotFound(String),
    /// Item cannot go in the requested slot.
    #[error("item does not fit slot")]
    WrongSlot,
    /// Item is not equipment.
    #[error("item is not equippable")]
    NotEquippable,
    /// Slot empty on unequip.
    #[error("slot empty")]
    SlotEmpty,
}

/// Additive modifiers from gear (and buffs).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct StatModifiers {
    /// Flat attack bonus.
    pub attack: f32,
    /// Flat defense bonus.
    pub defense: f32,
    /// Flat max HP bonus.
    pub max_hp: f32,
    /// Flat max MP bonus.
    pub max_mp: f32,
    /// Attribute deltas.
    pub strength: i32,
    /// Agility delta.
    pub agility: i32,
    /// Intellect delta.
    pub intellect: i32,
    /// Vitality delta.
    pub vitality: i32,
    /// Luck delta.
    pub luck: i32,
}

impl StatModifiers {
    /// Zero mods.
    pub const ZERO: Self = Self {
        attack: 0.0,
        defense: 0.0,
        max_hp: 0.0,
        max_mp: 0.0,
        strength: 0,
        agility: 0,
        intellect: 0,
        vitality: 0,
        luck: 0,
    };

    /// Combine two modifier sets.
    pub fn combine(self, other: Self) -> Self {
        Self {
            attack: self.attack + other.attack,
            defense: self.defense + other.defense,
            max_hp: self.max_hp + other.max_hp,
            max_mp: self.max_mp + other.max_mp,
            strength: self.strength + other.strength,
            agility: self.agility + other.agility,
            intellect: self.intellect + other.intellect,
            vitality: self.vitality + other.vitality,
            luck: self.luck + other.luck,
        }
    }

    /// Build from an item definition's attack/defense fields.
    pub fn from_item_def(def: &ItemDef) -> Self {
        Self {
            attack: def.attack,
            defense: def.defense,
            ..Self::ZERO
        }
    }

    /// Apply attribute portion onto a copy of base attributes.
    pub fn apply_attributes(self, base: Attributes) -> Attributes {
        Attributes {
            strength: base.strength + self.strength,
            agility: base.agility + self.agility,
            intellect: base.intellect + self.intellect,
            vitality: base.vitality + self.vitality,
            luck: base.luck + self.luck,
        }
    }
}

/// Final computed combat stats after equipment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EffectiveStats {
    /// Attributes after gear attribute mods.
    pub attributes: Attributes,
    /// Max HP including flat gear bonus.
    pub max_hp: f32,
    /// Max MP including flat gear bonus.
    pub max_mp: f32,
    /// Attack including gear.
    pub attack: f32,
    /// Defense including gear.
    pub defense: f32,
}

impl EffectiveStats {
    /// From base attributes + modifiers.
    pub fn compute(base: Attributes, mods: StatModifiers) -> Self {
        let attributes = mods.apply_attributes(base);
        Self {
            max_hp: attributes.max_hp() + mods.max_hp,
            max_mp: attributes.max_mp() + mods.max_mp,
            attack: attributes.attack() + mods.attack,
            defense: attributes.defense() + mods.defense,
            attributes,
        }
    }
}

/// Equipped loadout independent of bag inventory (can sync either way).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EquipmentLoadout {
    /// Slot → item def id.
    pub slots: IndexMap<EquipSlot, String>,
}

impl EquipmentLoadout {
    /// Empty loadout.
    pub fn new() -> Self {
        Self::default()
    }

    /// Equip an item id into its natural slot (from db). Returns previous id if any.
    pub fn equip(&mut self, def_id: &str, db: &ItemDb) -> Result<Option<String>, EquipmentError> {
        let def = db
            .get(def_id)
            .ok_or_else(|| EquipmentError::NotFound(def_id.into()))?;
        let slot = def.equip_slot.ok_or(EquipmentError::NotEquippable)?;
        if !matches!(
            def.kind,
            ItemKind::Weapon | ItemKind::Armor | ItemKind::Misc
        ) {
            // Allow weapon/armor; Misc with slot is ok for accessories.
            if def.equip_slot.is_none() {
                return Err(EquipmentError::NotEquippable);
            }
        }
        Ok(self.slots.insert(slot, def_id.into()))
    }

    /// Equip into an explicit slot (validates def.equip_slot matches).
    pub fn equip_in_slot(
        &mut self,
        def_id: &str,
        slot: EquipSlot,
        db: &ItemDb,
    ) -> Result<Option<String>, EquipmentError> {
        let def = db
            .get(def_id)
            .ok_or_else(|| EquipmentError::NotFound(def_id.into()))?;
        if def.equip_slot != Some(slot) {
            return Err(EquipmentError::WrongSlot);
        }
        Ok(self.slots.insert(slot, def_id.into()))
    }

    /// Unequip a slot; returns item id.
    pub fn unequip(&mut self, slot: EquipSlot) -> Result<String, EquipmentError> {
        self.slots
            .shift_remove(&slot)
            .ok_or(EquipmentError::SlotEmpty)
    }

    /// Get equipped id.
    pub fn get(&self, slot: EquipSlot) -> Option<&str> {
        self.slots.get(&slot).map(|s| s.as_str())
    }

    /// Sum modifiers from all equipped items.
    pub fn modifiers(&self, db: &ItemDb) -> StatModifiers {
        let mut mods = StatModifiers::ZERO;
        for id in self.slots.values() {
            if let Some(def) = db.get(id) {
                mods = mods.combine(StatModifiers::from_item_def(def));
            }
        }
        mods
    }

    /// Effective stats from base block + this loadout.
    pub fn effective(&self, base: &StatBlock, db: &ItemDb) -> EffectiveStats {
        EffectiveStats::compute(base.attributes, self.modifiers(db))
    }

    /// All equipped item ids.
    pub fn equipped_ids(&self) -> impl Iterator<Item = &str> {
        self.slots.values().map(|s| s.as_str())
    }

    /// Whether a slot is filled.
    pub fn is_equipped(&self, slot: EquipSlot) -> bool {
        self.slots.contains_key(&slot)
    }

    /// Clear all slots.
    pub fn clear(&mut self) {
        self.slots.clear();
    }
}

/// Helper: armor item definition.
pub fn armor_def(id: &str, name: &str, slot: EquipSlot, defense: f32, price: u32) -> ItemDef {
    ItemDef {
        id: ItemId::new(id),
        name: name.into(),
        kind: ItemKind::Armor,
        equip_slot: Some(slot),
        max_stack: 1,
        price,
        attack: 0.0,
        defense,
        heal: 0.0,
        description: name.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemDef;

    fn db() -> ItemDb {
        let mut db = ItemDb::default();
        db.insert(ItemDef::weapon("sword", "Sword", 5.0, 50));
        db.insert(armor_def("mail", "Mail", EquipSlot::Body, 3.0, 40));
        db.insert(ItemDef::potion("potion", "Potion", 10.0, 5));
        db
    }

    #[test]
    fn equip_and_effective_stats() {
        let db = db();
        let mut loadout = EquipmentLoadout::new();
        loadout.equip("sword", &db).unwrap();
        loadout.equip("mail", &db).unwrap();
        let base = StatBlock::default();
        let eff = loadout.effective(&base, &db);
        assert!(eff.attack > base.attributes.attack());
        assert!(eff.defense > base.attributes.defense());
        assert!(loadout.is_equipped(EquipSlot::MainHand));
        assert_eq!(loadout.get(EquipSlot::Body), Some("mail"));
    }

    #[test]
    fn wrong_slot_and_not_equippable() {
        let db = db();
        let mut loadout = EquipmentLoadout::new();
        assert!(matches!(
            loadout.equip_in_slot("sword", EquipSlot::Head, &db),
            Err(EquipmentError::WrongSlot)
        ));
        assert!(matches!(
            loadout.equip("potion", &db),
            Err(EquipmentError::NotEquippable)
        ));
    }

    #[test]
    fn unequip_and_swap() {
        let db = db();
        let mut loadout = EquipmentLoadout::new();
        loadout.equip("sword", &db).unwrap();
        // Another weapon
        let mut db2 = db;
        db2.insert(ItemDef::weapon("axe", "Axe", 7.0, 60));
        let prev = loadout.equip("axe", &db2).unwrap();
        assert_eq!(prev.as_deref(), Some("sword"));
        let id = loadout.unequip(EquipSlot::MainHand).unwrap();
        assert_eq!(id, "axe");
        assert!(matches!(
            loadout.unequip(EquipSlot::MainHand),
            Err(EquipmentError::SlotEmpty)
        ));
    }

    #[test]
    fn modifiers_combine() {
        let a = StatModifiers {
            attack: 1.0,
            strength: 2,
            ..StatModifiers::ZERO
        };
        let b = StatModifiers {
            attack: 3.0,
            defense: 1.0,
            strength: 1,
            ..StatModifiers::ZERO
        };
        let c = a.combine(b);
        assert!((c.attack - 4.0).abs() < 1e-5);
        assert_eq!(c.strength, 3);
    }

    #[test]
    fn full_loadout_multi_slots() {
        let mut db = ItemDb::default();
        db.insert(ItemDef::weapon("sword", "Sword", 5.0, 50));
        db.insert(armor_def("helm", "Helm", EquipSlot::Head, 1.0, 20));
        db.insert(armor_def("mail", "Mail", EquipSlot::Body, 3.0, 40));
        db.insert(armor_def("ring", "Ring", EquipSlot::Accessory, 0.5, 15));
        let mut loadout = EquipmentLoadout::new();
        assert!(loadout.equip("sword", &db).unwrap().is_none());
        assert!(loadout.equip("helm", &db).unwrap().is_none());
        assert!(loadout.equip("mail", &db).unwrap().is_none());
        assert!(loadout.equip("ring", &db).unwrap().is_none());
        assert!(loadout.is_equipped(EquipSlot::MainHand));
        assert!(loadout.is_equipped(EquipSlot::Head));
        assert!(loadout.is_equipped(EquipSlot::Body));
        assert!(loadout.is_equipped(EquipSlot::Accessory));
        let base = StatBlock::default();
        let eff = loadout.effective(&base, &db);
        assert!(eff.attack > base.attributes.attack());
        assert!(eff.defense > base.attributes.defense());
    }

    #[test]
    fn equip_unknown_item_errors() {
        let db = db();
        let mut loadout = EquipmentLoadout::new();
        assert!(matches!(
            loadout.equip("nope", &db),
            Err(EquipmentError::NotFound(_))
        ));
    }

    #[test]
    fn unequip_all_slots() {
        let db = db();
        let mut loadout = EquipmentLoadout::new();
        loadout.equip("sword", &db).unwrap();
        loadout.equip("mail", &db).unwrap();
        assert_eq!(loadout.unequip(EquipSlot::MainHand).unwrap(), "sword");
        assert_eq!(loadout.unequip(EquipSlot::Body).unwrap(), "mail");
        assert!(!loadout.is_equipped(EquipSlot::MainHand));
        assert!(!loadout.is_equipped(EquipSlot::Body));
    }

    #[test]
    fn weapon_swap_returns_previous() {
        let mut db = db();
        db.insert(ItemDef::weapon("dagger", "Dagger", 2.0, 10));
        let mut loadout = EquipmentLoadout::new();
        loadout.equip("sword", &db).unwrap();
        let prev = loadout.equip("dagger", &db).unwrap();
        assert_eq!(prev.as_deref(), Some("sword"));
        assert_eq!(loadout.get(EquipSlot::MainHand), Some("dagger"));
    }
}
