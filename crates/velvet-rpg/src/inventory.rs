//! Inventory bags and equipment.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::item::{EquipSlot, ItemDb, ItemId, ItemInstance};
use crate::stats::StatBlock;

/// Inventory errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InventoryError {
    /// Full.
    #[error("inventory full")]
    Full,
    /// Missing item.
    #[error("item not found: {0}")]
    NotFound(String),
    /// Not enough quantity.
    #[error("not enough items")]
    NotEnough,
    /// Cannot equip.
    #[error("cannot equip")]
    CannotEquip,
}

/// Character inventory + equipment + gold.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inventory {
    /// Stacks (def_id -> instance).
    pub items: IndexMap<String, ItemInstance>,
    /// Max distinct stacks.
    pub capacity: usize,
    /// Gold / currency.
    pub gold: u32,
    /// Equipped item def ids.
    pub equipped: IndexMap<EquipSlot, String>,
}

impl Inventory {
    /// Create with capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    /// Add items; merges stacks.
    pub fn add(&mut self, def_id: &str, count: u32, max_stack: u32) -> Result<(), InventoryError> {
        if count == 0 {
            return Ok(());
        }
        if let Some(stack) = self.items.get_mut(def_id) {
            let room = max_stack.saturating_sub(stack.count);
            let add = count.min(room);
            stack.count += add;
            let left = count - add;
            if left > 0 {
                return Err(InventoryError::Full);
            }
            return Ok(());
        }
        if self.items.len() >= self.capacity {
            return Err(InventoryError::Full);
        }
        self.items.insert(
            def_id.into(),
            ItemInstance {
                def_id: ItemId::new(def_id),
                count: count.min(max_stack),
            },
        );
        Ok(())
    }

    /// Remove count.
    pub fn remove(&mut self, def_id: &str, count: u32) -> Result<(), InventoryError> {
        let stack = self
            .items
            .get_mut(def_id)
            .ok_or_else(|| InventoryError::NotFound(def_id.into()))?;
        if stack.count < count {
            return Err(InventoryError::NotEnough);
        }
        stack.count -= count;
        if stack.count == 0 {
            self.items.shift_remove(def_id);
        }
        Ok(())
    }

    /// Count of item.
    pub fn count(&self, def_id: &str) -> u32 {
        self.items.get(def_id).map(|i| i.count).unwrap_or(0)
    }

    /// Equip from inventory.
    pub fn equip(
        &mut self,
        def_id: &str,
        slot: EquipSlot,
        db: &ItemDb,
    ) -> Result<(), InventoryError> {
        let def = db
            .get(def_id)
            .ok_or_else(|| InventoryError::NotFound(def_id.into()))?;
        if def.equip_slot != Some(slot) {
            return Err(InventoryError::CannotEquip);
        }
        if self.count(def_id) == 0 {
            return Err(InventoryError::NotFound(def_id.into()));
        }
        // Unequip previous back to bag
        if let Some(prev) = self.equipped.insert(slot, def_id.into()) {
            let max = db.get(&prev).map(|d| d.max_stack).unwrap_or(1);
            let _ = self.add(&prev, 1, max);
        }
        self.remove(def_id, 1)?;
        Ok(())
    }

    /// Sum equipment bonuses into a temporary attack/defense pair.
    pub fn equipment_bonuses(&self, db: &ItemDb) -> (f32, f32) {
        let mut atk = 0.0;
        let mut def = 0.0;
        for id in self.equipped.values() {
            if let Some(d) = db.get(id) {
                atk += d.attack;
                def += d.defense;
            }
        }
        (atk, def)
    }

    /// Use consumable on stats.
    pub fn use_consumable(
        &mut self,
        def_id: &str,
        db: &ItemDb,
        stats: &mut StatBlock,
    ) -> Result<(), InventoryError> {
        let def = db
            .get(def_id)
            .ok_or_else(|| InventoryError::NotFound(def_id.into()))?;
        if !matches!(def.kind, crate::item::ItemKind::Consumable) {
            return Err(InventoryError::CannotEquip);
        }
        self.remove(def_id, 1)?;
        stats.heal(def.heal);
        Ok(())
    }

    /// Unequip a slot back into the bag.
    pub fn unequip(&mut self, slot: EquipSlot, db: &ItemDb) -> Result<String, InventoryError> {
        let id = self
            .equipped
            .shift_remove(&slot)
            .ok_or_else(|| InventoryError::NotFound(format!("{slot:?}")))?;
        let max = db.get(&id).map(|d| d.max_stack).unwrap_or(1);
        self.add(&id, 1, max)?;
        Ok(id)
    }

    /// Whether the bag contains at least `count` of an item.
    pub fn has(&self, def_id: &str, count: u32) -> bool {
        self.count(def_id) >= count
    }

    /// Free stack slots remaining.
    pub fn free_slots(&self) -> usize {
        self.capacity.saturating_sub(self.items.len())
    }

    /// Distinct item kinds in bag.
    pub fn distinct_count(&self) -> usize {
        self.items.len()
    }

    /// Total item units across all stacks.
    pub fn total_items(&self) -> u32 {
        self.items.values().map(|i| i.count).sum()
    }

    /// Transfer gold (checked subtract).
    pub fn spend_gold(&mut self, amount: u32) -> Result<(), InventoryError> {
        if self.gold < amount {
            return Err(InventoryError::NotEnough);
        }
        self.gold -= amount;
        Ok(())
    }

    /// Add gold with saturating add.
    pub fn add_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemDef;

    #[test]
    fn add_equip_use() {
        let mut db = ItemDb::default();
        db.insert(ItemDef::potion("potion", "Potion", 25.0, 10));
        db.insert(ItemDef::weapon("sword", "Sword", 5.0, 50));
        let mut inv = Inventory::with_capacity(20);
        inv.gold = 100;
        inv.add("potion", 3, 99).unwrap();
        inv.add("sword", 1, 1).unwrap();
        inv.equip("sword", EquipSlot::MainHand, &db).unwrap();
        assert_eq!(inv.count("sword"), 0);
        assert_eq!(
            inv.equipped.get(&EquipSlot::MainHand).map(|s| s.as_str()),
            Some("sword")
        );
        let mut stats = StatBlock::default();
        let hp = stats.hp;
        stats.take_damage(30.0);
        inv.use_consumable("potion", &db, &mut stats).unwrap();
        assert!(stats.hp > hp - 30.0);
    }
}
