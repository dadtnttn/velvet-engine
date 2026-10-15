//! Lo que lleva la Reina: el oro y los objetos para el menú, la mochila de cuadrícula, el equipo y el botín que trae de la mazmorra.
use super::*;

impl App {
    /// Whether the Queen owns anything to fight with.
    pub(crate) fn has_weapon(&self) -> bool {
        self.backpack.equipment.weapon.is_some()
            || [
                RELIC_SWORD_ID,
                "espada_de_madera",
                "espada_pegajosa",
                "espada_brasas",
                "espada_corazon",
            ]
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
                effect: backpack::use_line(&product.id).unwrap_or_else(|| product.effect.clone()),
                quantity: self
                    .inventory
                    .get(&product.id)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(if self.dungeon_mode {
                        match product.id.as_str() {
                            "pocion_pequena" => self.dungeon.unbanked_potions,
                            "hierbas_curativas" => self.dungeon.unbanked_herbs,
                            // The stone sword found in a treasury on this trip.
                            RELIC_SWORD_ID => u32::from(self.dungeon.unbanked_relic_sword),
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
                effect: backpack::use_line(extra.id).unwrap_or_default(),
                quantity: self.inventory.get(extra.id).copied().unwrap_or(0),
            });
        }
        // (The stone sword is one of the shop's products now: counted with them, once.)
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
                effect: backpack::use_line(&product.id).unwrap_or_else(|| product.effect.clone()),
                quantity: sample.get(&product.id).copied().unwrap_or(0),
            })
            .collect();
        for extra in backpack::EXTRAS.iter() {
            items.push(InventoryMenuItem {
                id: extra.id.into(),
                name: extra.name.into(),
                description: extra.description.into(),
                icon: extra.icon.into(),
                icon_svg: Some(extra.icon_svg.into()),
                category: extra.category.into(),
                effect: backpack::use_line(extra.id).unwrap_or_default(),
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

    /// What the worn weapon does in the caves (and against Varrak): a stone blade cuts deeper,
    /// the fused ones carry goo, embers or the Heart.
    pub(crate) fn apply_equipment(&mut self) {
        let owns_relic = self.inventory.get(RELIC_SWORD_ID).is_some_and(|n| *n > 0)
            || self.dungeon.unbanked_relic_sword;
        let worn = self.backpack.equipment.weapon.clone();
        // Taken off by hand: bare fists (no sword drawn, half the damage).
        let bare = worn.is_none() && self.backpack.equipment.bare_hands;
        self.dungeon.relic_sword = match worn.as_deref() {
            Some(id) => backpack::stone_sword(id),
            None => owns_relic && !bare,
        };
        self.dungeon.armed = !bare;
        self.dungeon.sword_goo = worn.as_deref().and_then(backpack::sword_goo);
        self.dungeon.weapon_fx = worn
            .as_deref()
            .map(backpack::weapon_fx)
            .unwrap_or_default();
        let damage = worn.as_deref().map_or(
            if bare {
                0.5
            } else if owns_relic {
                dungeon::RELIC_SWORD_BONUS
            } else {
                1.0
            },
            backpack::weapon_damage,
        );
        let fx = self.dungeon.weapon_fx;
        if let Some(fight) = self.boss.as_mut() {
            fight.blade_damage = damage;
            fight.blade = fx;
        }
    }

    /// Her vitality where it counts (the mine, a fight): now and at most.
    pub(crate) fn vitality(&self) -> Option<(f32, f32)> {
        if self.dungeon_mode {
            Some((self.dungeon.health, 100.0))
        } else {
            self.boss
                .as_ref()
                .map(|f| (f.player.health, f.player.max_health))
        }
    }

    /// How many of `id` she carries (in the mine, with what she has picked up there).
    fn owned(&self, id: &str) -> u32 {
        let found = if self.dungeon_mode {
            match id {
                "pocion_pequena" => self.dungeon.unbanked_potions,
                "hierbas_curativas" => self.dungeon.unbanked_herbs,
                _ => 0,
            }
        } else {
            0
        };
        self.inventory.get(id).copied().unwrap_or(0) + found
    }

    /// One `id` spent: what the mine picked up goes first, then the bag (and its grid, when
    /// `from_pack`; the pause menu has already taken it off its own).
    fn spend_one(&mut self, id: &str, from_pack: bool) {
        if self.dungeon_mode {
            if id == "pocion_pequena" && self.dungeon.unbanked_potions > 0 {
                self.dungeon.unbanked_potions -= 1;
                self.dungeon.potions = self.dungeon.potions.saturating_sub(1);
                return;
            }
            if id == "hierbas_curativas" && self.dungeon.unbanked_herbs > 0 {
                self.dungeon.unbanked_herbs -= 1;
                self.dungeon.herbs = self.dungeon.herbs.saturating_sub(1);
                return;
            }
        }
        if let Some(n) = self.inventory.get_mut(id) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.inventory.remove(id);
            }
        }
        if from_pack {
            self.backpack.remove(id, 1);
        }
    }

    /// The vitality goes in: the mine's or the fight's.
    fn take_cure(&mut self, id: &str, amount: f32) {
        let name = backpack::extra(id)
            .map(|e| e.name.to_owned())
            .or_else(|| {
                shop::catalog()
                    .products
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name.clone())
            })
            .unwrap_or_else(|| id.to_owned());
        if self.dungeon_mode {
            self.dungeon
                .cure(amount, format!("{name}: +{amount:.0} vitalidad."));
        } else if let Some(fight) = self.boss.as_mut() {
            fight.cure(amount);
        }
        eprintln!("[mochila] {name}: +{amount:.0}");
    }

    /// Q in the mine or a fight: the expedition's own tonics first, then the cure from the bag
    /// that best fills what she is missing.
    pub(crate) fn quick_cure(&mut self) {
        let Some((now, most)) = self.vitality() else {
            return;
        };
        if now >= most {
            return;
        }
        if self.dungeon_mode && self.dungeon.has_supplies() {
            self.dungeon.heal();
            return;
        }
        if self.boss.as_ref().is_some_and(|f| !f.can_cure()) {
            return;
        }
        let missing = most - now;
        let Some(id) = backpack::best_cure(missing, |id| self.owned(id) > 0) else {
            if self.dungeon_mode {
                self.dungeon
                    .message("No te queda nada para curarte en la mochila.");
            } else if let Some(fight) = self.boss.as_mut() {
                fight.say("La mochila está vacía de curas. Aguanta.");
            }
            return;
        };
        let amount = backpack::cure(id).unwrap_or(0.0);
        self.spend_one(id, true);
        self.take_cure(id, amount);
    }

    /// What was used from the pause menu's backpack (already taken off its grid).
    pub(crate) fn use_from_menu(&mut self, ids: &[String]) {
        for id in ids {
            let Some(amount) = backpack::cure(id) else {
                continue;
            };
            // Outside the mine the menu's counts become the bag's when it closes.
            if self.dungeon_mode {
                self.spend_one(id, false);
            }
            self.take_cure(id, amount);
        }
    }
}
