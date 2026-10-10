//! Lo que lleva la Reina: el oro y los objetos para el menú, la mochila de cuadrícula, el equipo y el botín que trae de la mazmorra.
use super::*;

impl App {
    /// Whether the Queen owns anything to fight with.
    pub(crate) fn has_weapon(&self) -> bool {
        self.backpack.equipment.weapon.is_some()
            || [RELIC_SWORD_ID, "espada_de_madera"]
                .iter()
                .any(|id| self.inventory.get(*id).is_some_and(|n| *n > 0))
            || self.dungeon.unbanked_relic_sword
    }

    /// Move chest supplies into the shared, serialized economy exactly once.
    pub(super) fn bank_dungeon_loot(&mut self) -> bool {
        let (potions, herbs) = self.dungeon.take_acquired_items();
        let relic_sword = self.dungeon.take_relic_sword();
        if relic_sword {
            self.inventory.insert(RELIC_SWORD_ID.into(), 1);
            // A relic found with nothing wielded is taken in hand at once.
            let mut pack = self.backpack.clone();
            let counts: BTreeMap<String, u32> = self.inventory.clone();
            pack.reconcile(&counts);
            pack.equip_if_empty(RELIC_SWORD_ID);
            self.backpack = pack;
        }
        // Bones, ash, jelly and trophies go into the pack to sell at the shop.
        let materials = self.dungeon.take_materials();
        let gathered = !materials.is_empty();
        for (id, count) in materials {
            *self.inventory.entry(id).or_default() += count;
        }
        if potions == 0 && herbs == 0 {
            return relic_sword || gathered;
        }
        if potions > 0 {
            *self.inventory.entry("pocion_pequena".into()).or_default() += potions;
        }
        if herbs > 0 {
            *self
                .inventory
                .entry("hierbas_curativas".into())
                .or_default() += herbs;
        }
        true
    }

    /// Snapshot the live economy for the pause-menu backpack. Returning owned
    /// strings keeps the menu independent from the catalog's internal lock.
    pub(crate) fn inventory_for_menu(&self) -> (u32, Vec<InventoryMenuItem>) {
        let items: Vec<InventoryMenuItem> = shop::catalog()
            .products
            .iter()
            .map(|product| InventoryMenuItem {
                id: product.id.clone(),
                name: product.name.clone(),
                description: product.description.clone(),
                icon: product.icon.clone(),
                icon_svg: product.icon_svg.clone(),
                category: format!("{:?}", product.category).to_lowercase(),
                effect: product.effect.clone(),
                quantity: self
                    .inventory
                    .get(&product.id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(if self.dungeon_mode {
                        match product.id.as_str() {
                            "pocion_pequena" => self.dungeon.unbanked_potions,
                            "hierbas_curativas" => self.dungeon.unbanked_herbs,
                            id => self.dungeon.materials.get(id).copied().unwrap_or(0),
                        }
                    } else {
                        0
                    }),
            })
            .collect();
        let mut items = items;
        // Things only a combination makes.
        for extra in backpack::EXTRAS.iter() {
            items.push(InventoryMenuItem {
                id: extra.id.into(),
                name: extra.name.into(),
                description: extra.description.into(),
                icon: extra.icon.into(),
                icon_svg: Some(extra.icon_svg.into()),
                category: extra.category.into(),
                effect: String::new(),
                quantity: self.inventory.get(extra.id).copied().unwrap_or(0),
            });
        }
        if self.inventory.get(RELIC_SWORD_ID).is_some_and(|n| *n > 0)
            || (self.dungeon_mode && self.dungeon.unbanked_relic_sword)
        {
            items.push(InventoryMenuItem {
                id: RELIC_SWORD_ID.into(),
                name: "Espada de piedra".into(),
                description: "Hallada entre el oro de una tesorería. Corta más hondo que la espada de madera, aunque ningún enemigo cae de pocos golpes.".into(),
                icon: "E".into(),
                icon_svg: Some("item_stone_sword.svg".into()),
                category: "equipment".into(),
                effect: String::new(),
                quantity: 1,
            });
        }
        let expedition_gold = if self.dungeon_mode {
            self.dungeon.gold
        } else {
            0
        };
        (self.gold.saturating_add(expedition_gold), items)
    }

    /// A believable backpack for menu screenshots: the real catalogue with
    /// a sample of what an expedition brings back.
    #[allow(dead_code)]
    pub(crate) fn sample_menu_inventory() -> (u32, Vec<InventoryMenuItem>, backpack::Backpack) {
        let sample: BTreeMap<String, u32> = [
            ("espada_de_madera", 1),
            ("espada_reliquia", 1),
            ("pocion_pequena", 3),
            ("hierbas_curativas", 8),
            ("pan_del_dia", 2),
            ("tela_resistente", 3),
            ("esencia_carmesi", 2),
            ("tinta_runica", 1),
            ("fragmento_sello", 1),
            ("anillo_misterioso", 1),
            ("amuleto_ocultacion", 1),
            ("hueso_viejo", 9),
            ("ceniza_arcana", 4),
            ("gelatina_azul", 5),
            ("craneo_campeon", 1),
            ("llave_boveda", 1),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        let mut items: Vec<InventoryMenuItem> = shop::catalog()
            .products
            .iter()
            .map(|product| InventoryMenuItem {
                id: product.id.clone(),
                name: product.name.clone(),
                description: product.description.clone(),
                icon: product.icon.clone(),
                icon_svg: product.icon_svg.clone(),
                category: format!("{:?}", product.category).to_lowercase(),
                effect: product.effect.clone(),
                quantity: sample.get(&product.id).copied().unwrap_or(0),
            })
            .collect();
        items.push(InventoryMenuItem {
            id: RELIC_SWORD_ID.into(),
            name: "Espada de piedra".into(),
            description:
                "Hallada entre el oro de una tesorería. Corta más hondo que la espada de madera."
                    .into(),
            icon: "E".into(),
            icon_svg: Some("item_stone_sword.svg".into()),
            category: "equipment".into(),
            effect: String::new(),
            quantity: 1,
        });
        for extra in backpack::EXTRAS.iter() {
            items.push(InventoryMenuItem {
                id: extra.id.into(),
                name: extra.name.into(),
                description: extra.description.into(),
                icon: extra.icon.into(),
                icon_svg: Some(extra.icon_svg.into()),
                category: extra.category.into(),
                effect: String::new(),
                quantity: 0,
            });
        }
        let mut counts = sample;
        counts.insert(RELIC_SWORD_ID.into(), 1);
        let mut pack = backpack::Backpack::new();
        pack.reconcile(&counts);
        if let Some(i) = pack.placed.iter().position(|p| p.id == "espada_de_madera") {
            let _ = pack.equip(i);
        }
        (245, items, pack)
    }

    /// The grid backpack as the pause menu edits it, made to agree with the
    /// live item counts, and whether items can be combined right now (not
    /// mid-expedition, where potions and herbs are still being carried).
    pub(crate) fn backpack_for_menu(&self) -> (backpack::Backpack, bool) {
        let counts: BTreeMap<String, u32> = self
            .inventory_for_menu()
            .1
            .into_iter()
            .filter(|item| item.quantity > 0)
            .map(|item| (item.id, item.quantity))
            .collect();
        let mut pack = self.backpack.clone();
        pack.reconcile(&counts);
        (pack, !self.dungeon_mode)
    }

    /// Takes the layout the menu produced. Outside an expedition the counts
    /// follow it too (a combination spends items and makes another).
    pub(crate) fn commit_backpack(&mut self, pack: backpack::Backpack) {
        if !self.dungeon_mode {
            self.inventory = pack.totals();
        }
        self.backpack = pack;
        self.apply_equipment();
    }

    /// What the worn weapon does in the caves.
    pub(super) fn apply_equipment(&mut self) {
        let owns_relic = self.inventory.get(RELIC_SWORD_ID).is_some_and(|n| *n > 0)
            || self.dungeon.unbanked_relic_sword;
        self.dungeon.relic_sword = match self.backpack.equipment.weapon.as_deref() {
            Some(id) => id == RELIC_SWORD_ID,
            None => owns_relic,
        };
    }
}
