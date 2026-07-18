//! Simple shops.

use thiserror::Error;

use crate::inventory::Inventory;
use crate::item::ItemDb;

/// Shop errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ShopError {
    /// Not enough gold.
    #[error("not enough gold")]
    NoGold,
    /// Unknown item.
    #[error("unknown item")]
    UnknownItem,
    /// Inventory error.
    #[error("inventory: {0}")]
    Inventory(String),
}

/// Shop stock entry.
#[derive(Debug, Clone)]
pub struct Shop {
    /// Item def ids sold.
    pub stock: Vec<String>,
    /// Price multiplier.
    pub price_mul: f32,
}

impl Default for Shop {
    fn default() -> Self {
        Self {
            stock: Vec::new(),
            price_mul: 1.0,
        }
    }
}

impl Shop {
    /// Buy item into inventory.
    pub fn buy(&self, def_id: &str, inv: &mut Inventory, db: &ItemDb) -> Result<(), ShopError> {
        if !self.stock.iter().any(|s| s == def_id) {
            return Err(ShopError::UnknownItem);
        }
        let def = db.get(def_id).ok_or(ShopError::UnknownItem)?;
        let price = ((def.price as f32) * self.price_mul).ceil() as u32;
        if inv.gold < price {
            return Err(ShopError::NoGold);
        }
        inv.gold -= price;
        inv.add(def_id, 1, def.max_stack)
            .map_err(|e| ShopError::Inventory(e.to_string()))?;
        Ok(())
    }

    /// Sell from inventory.
    pub fn sell(
        &self,
        def_id: &str,
        count: u32,
        inv: &mut Inventory,
        db: &ItemDb,
    ) -> Result<(), ShopError> {
        let def = db.get(def_id).ok_or(ShopError::UnknownItem)?;
        let price = ((def.price as f32) * 0.5 * self.price_mul).floor() as u32 * count;
        inv.remove(def_id, count)
            .map_err(|e| ShopError::Inventory(e.to_string()))?;
        inv.gold += price;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::ItemDef;

    #[test]
    fn buy_and_sell() {
        let mut db = ItemDb::default();
        db.insert(ItemDef::potion("potion", "Potion", 10.0, 20));
        let shop = Shop {
            stock: vec!["potion".into()],
            price_mul: 1.0,
        };
        let mut inv = Inventory::with_capacity(10);
        inv.gold = 50;
        shop.buy("potion", &mut inv, &db).unwrap();
        assert_eq!(inv.gold, 30);
        assert_eq!(inv.count("potion"), 1);
        shop.sell("potion", 1, &mut inv, &db).unwrap();
        assert_eq!(inv.count("potion"), 0);
        assert!(inv.gold >= 40);
    }
}
