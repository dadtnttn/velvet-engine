//! The Queen's backpack, in the style of an attaché case: a small grid of
//! limited slots in which every item has its own shape and can be rotated,
//! stacked, combined with another item, or worn as equipment. This module is
//! only the rules and the saved layout; the pause menu draws and drives it,
//! and the world keeps the authoritative item counts (`reconcile` makes the
//! layout agree with them).
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const COLS: u8 = 8;
pub const ROWS: u8 = 5;
/// Places on the equipped weapon for future upgrades (edge, pommel...).
pub const WEAPON_SOCKETS: usize = 2;

/// Body slots that take equipment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Weapon,
    Charm,
    Ring,
}

impl Slot {
    pub const ALL: [Slot; 3] = [Slot::Weapon, Slot::Charm, Slot::Ring];

    pub fn label(self, spanish: bool) -> &'static str {
        match (self, spanish) {
            (Slot::Weapon, true) => "Arma",
            (Slot::Weapon, false) => "Weapon",
            (Slot::Charm, true) => "Amuleto",
            (Slot::Charm, false) => "Charm",
            (Slot::Ring, true) => "Anillo",
            (Slot::Ring, false) => "Ring",
        }
    }
}

/// An item's shape, stack limit and where it can be worn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Info {
    pub w: u8,
    pub h: u8,
    pub stack: u32,
    pub slot: Option<Slot>,
}

const fn info_of(w: u8, h: u8, stack: u32, slot: Option<Slot>) -> Info {
    Info { w, h, stack, slot }
}

/// Every item the pack knows, with its footprint. Unknown ids get a single
/// cell so future items never break a save.
pub fn info(id: &str) -> Info {
    match id {
        "pocion_pequena" => info_of(1, 2, 3, None),
        "hierbas_curativas" => info_of(1, 1, 6, None),
        "pan_del_dia" => info_of(2, 1, 3, None),
        "tela_resistente" => info_of(2, 1, 4, None),
        "espada_de_madera" => info_of(1, 4, 1, Some(Slot::Weapon)),
        "espada_reliquia" => info_of(1, 5, 1, Some(Slot::Weapon)),
        "pocion_misteriosa" => info_of(1, 2, 2, None),
        "esencia_carmesi" => info_of(1, 1, 4, None),
        "tinta_runica" => info_of(1, 2, 3, None),
        "anillo_misterioso" => info_of(1, 1, 1, Some(Slot::Ring)),
        "fragmento_sello" => info_of(2, 1, 3, None),
        "llave_boveda" => info_of(1, 2, 1, None),
        "perfume_sucubo" => info_of(1, 2, 2, None),
        "cristal_memoria" => info_of(1, 2, 1, None),
        "amuleto_ocultacion" => info_of(1, 1, 1, Some(Slot::Charm)),
        "mapa_mercado_bajo" => info_of(2, 2, 1, None),
        "caja_cerrada" => info_of(2, 2, 1, None),
        "hueso_viejo" => info_of(1, 2, 6, None),
        "ceniza_arcana" => info_of(1, 1, 8, None),
        "gelatina_azul" | "gelatina_verde" => info_of(1, 1, 6, None),
        "craneo_campeon" => info_of(2, 2, 1, None),
        "esquirla_corazon" => info_of(1, 2, 2, None),
        "cataplasma_herbal" => info_of(1, 1, 4, None),
        "pocion_reforzada" | "elixir_carmesi" => info_of(1, 2, 2, None),
        "sello_inscrito" => info_of(2, 1, 1, None),
        // The hypnotist's wares (`explore/mage.rs`).
        "sales_despertadoras" => info_of(1, 1, 5, None),
        "amuleto_ojo_cerrado" => info_of(1, 1, 1, None),
        "pendulo_practica" => info_of(1, 2, 1, None),
        _ => info_of(1, 1, 5, None),
    }
}

/// Items the villa's shop does not describe: what only a combination makes, and what the hypnotist
/// of the wood sells at his own stall.
pub struct Extra {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
    /// File in `assets/ui/shop`.
    pub icon_svg: &'static str,
    pub category: &'static str,
}

pub const EXTRAS: [Extra; 7] = [
    Extra {
        id: "cataplasma_herbal",
        name: "Cataplasma de hierbas",
        description: "Dos manojos machacados y prensados en un paño. Huele a monte y a cura reciente.",
        icon: "C",
        icon_svg: "item_poultice.svg",
        category: "consumables",
    },
    Extra {
        id: "pocion_reforzada",
        name: "Poción reforzada",
        description: "Una poción pequeña con hierbas maceradas dentro. Más espesa, más amarga y bastante más eficaz.",
        icon: "R",
        icon_svg: "item_potion_strong.svg",
        category: "consumables",
    },
    Extra {
        id: "elixir_carmesi",
        name: "Elixir carmesí",
        description: "La esencia carmesí disuelta en una poción. Late al ritmo de quien la sostiene.",
        icon: "E",
        icon_svg: "item_elixir.svg",
        category: "consumables",
    },
    Extra {
        id: "sello_inscrito",
        name: "Sello inscrito",
        description: "Un fragmento de sello con su runa reescrita en tinta. Aún tibio al tacto.",
        icon: "S",
        icon_svg: "item_seal_inscribed.svg",
        category: "special",
    },
    Extra {
        id: "sales_despertadoras",
        name: "Sales despertadoras",
        description: "Un frasquito que pica en la nariz hasta las lágrimas. Se lo vendió el mago del bosque: en su hipnosis, Q despierta de golpe.",
        icon: "S",
        icon_svg: "item_salts.svg",
        category: "consumables",
    },
    Extra {
        id: "amuleto_ojo_cerrado",
        name: "Amuleto del ojo cerrado",
        description: "Un ojo de latón, cerrado y un poco bizco. Llevarlo encima basta: el anillo del mago tira menos de la mirada.",
        icon: "O",
        icon_svg: "item_eye_amulet.svg",
        category: "special",
    },
    Extra {
        id: "pendulo_practica",
        name: "Péndulo de práctica",
        description: "Una tuerca atada a un cordel. «Para practicar en casa», dijo el mago. No hace nada. Chiquita dice que es una estafa.",
        icon: "P",
        icon_svg: "item_pendulum.svg",
        category: "special",
    },
];

pub fn extra(id: &str) -> Option<&'static Extra> {
    EXTRAS.iter().find(|e| e.id == id)
}

/// Two items that make a third when one is dropped on the other.
pub struct Recipe {
    pub a: &'static str,
    pub b: &'static str,
    pub result: &'static str,
}

pub const RECIPES: [Recipe; 4] = [
    Recipe {
        a: "hierbas_curativas",
        b: "hierbas_curativas",
        result: "cataplasma_herbal",
    },
    Recipe {
        a: "hierbas_curativas",
        b: "pocion_pequena",
        result: "pocion_reforzada",
    },
    Recipe {
        a: "pocion_pequena",
        b: "esencia_carmesi",
        result: "elixir_carmesi",
    },
    Recipe {
        a: "fragmento_sello",
        b: "tinta_runica",
        result: "sello_inscrito",
    },
];

pub fn recipe_for(a: &str, b: &str) -> Option<&'static Recipe> {
    RECIPES
        .iter()
        .find(|r| (r.a == a && r.b == b) || (r.a == b && r.b == a))
}

/// Recipes that use `id`, for the item's detail card.
pub fn recipes_with(id: &str) -> Vec<&'static Recipe> {
    RECIPES.iter().filter(|r| r.a == id || r.b == id).collect()
}

/// Damage multiplier of a weapon, and a line for the equipment screen.
pub fn weapon_damage(id: &str) -> f32 {
    match id {
        "espada_reliquia" => super::dungeon::RELIC_SWORD_BONUS,
        _ => 1.0,
    }
}

/// One item (or a stack) lying in the grid: its top-left cell, whether it is
/// turned on its side, and how many units it holds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placed {
    pub id: String,
    pub x: u8,
    pub y: u8,
    #[serde(default)]
    pub rot: bool,
    pub qty: u32,
}

impl Placed {
    pub fn size(&self) -> (u8, u8) {
        size_of(&self.id, self.rot)
    }
}

pub fn size_of(id: &str, rot: bool) -> (u8, u8) {
    let i = info(id);
    if rot {
        (i.h, i.w)
    } else {
        (i.w, i.h)
    }
}

/// What is worn, and what is fitted to the weapon.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    #[serde(default)]
    pub weapon: Option<String>,
    #[serde(default)]
    pub charm: Option<String>,
    #[serde(default)]
    pub ring: Option<String>,
    /// Upgrades fitted to the weapon, one per socket.
    #[serde(default)]
    pub weapon_mods: Vec<Option<String>>,
}

impl Equipment {
    pub fn get(&self, slot: Slot) -> Option<&str> {
        match slot {
            Slot::Weapon => self.weapon.as_deref(),
            Slot::Charm => self.charm.as_deref(),
            Slot::Ring => self.ring.as_deref(),
        }
    }
    fn slot_mut(&mut self, slot: Slot) -> &mut Option<String> {
        match slot {
            Slot::Weapon => &mut self.weapon,
            Slot::Charm => &mut self.charm,
            Slot::Ring => &mut self.ring,
        }
    }
    fn worn(&self) -> impl Iterator<Item = &str> {
        Slot::ALL.into_iter().filter_map(|s| self.get(s))
    }
}

/// Why an action on the pack was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refuse {
    /// The shape would leave the grid or overlap another item.
    Blocked,
    /// There is nowhere in the grid for what the action would produce.
    NoRoom,
    NoRecipe,
    NotEquipment,
    /// The two items are not the same kind, or one stack is already full.
    CannotMerge,
    NothingThere,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Backpack {
    pub cols: u8,
    pub rows: u8,
    pub placed: Vec<Placed>,
    #[serde(default)]
    pub equipment: Equipment,
    /// Owned but with no room in the grid: they wait until space frees up.
    #[serde(default)]
    pub overflow: BTreeMap<String, u32>,
}

impl Default for Backpack {
    fn default() -> Self {
        Self::new()
    }
}

impl Backpack {
    pub fn new() -> Self {
        Self {
            cols: COLS,
            rows: ROWS,
            placed: Vec::new(),
            equipment: Equipment {
                weapon_mods: vec![None; WEAPON_SOCKETS],
                ..Default::default()
            },
            overflow: BTreeMap::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.cols as usize * self.rows as usize
    }

    pub fn used_cells(&self) -> usize {
        self.placed
            .iter()
            .map(|p| {
                let (w, h) = p.size();
                w as usize * h as usize
            })
            .sum()
    }

    /// Index of the stack covering cell (x, y).
    pub fn at(&self, x: u8, y: u8) -> Option<usize> {
        self.placed.iter().position(|p| {
            let (w, h) = p.size();
            x >= p.x && x < p.x + w && y >= p.y && y < p.y + h
        })
    }

    /// Whether a `w` x `h` shape fits with its top-left at (x, y), ignoring
    /// the stack `ignore` (the one being moved).
    pub fn can_place(&self, w: u8, h: u8, x: i32, y: i32, ignore: Option<usize>) -> bool {
        if x < 0 || y < 0 || x + w as i32 > self.cols as i32 || y + h as i32 > self.rows as i32 {
            return false;
        }
        let (x, y) = (x as u8, y as u8);
        !self.placed.iter().enumerate().any(|(i, p)| {
            if Some(i) == ignore {
                return false;
            }
            let (pw, ph) = p.size();
            x < p.x + pw && p.x < x + w && y < p.y + ph && p.y < y + h
        })
    }

    /// First free spot for `id`, scanning rows left to right, upright first
    /// and turned only when no upright spot exists.
    pub fn find_space(&self, id: &str) -> Option<(u8, u8, bool)> {
        let i = info(id);
        let orientations: &[bool] = if i.w == i.h { &[false] } else { &[false, true] };
        for &rot in orientations {
            let (w, h) = if rot { (i.h, i.w) } else { (i.w, i.h) };
            for y in 0..self.rows {
                for x in 0..self.cols {
                    if self.can_place(w, h, x as i32, y as i32, None) {
                        return Some((x, y, rot));
                    }
                }
            }
        }
        None
    }

    /// Units of `id` anywhere: in the grid, worn, or waiting.
    pub fn count(&self, id: &str) -> u32 {
        let placed: u32 = self
            .placed
            .iter()
            .filter(|p| p.id == id)
            .map(|p| p.qty)
            .sum();
        let worn = self.equipment.worn().filter(|w| *w == id).count() as u32
            + self
                .equipment
                .weapon_mods
                .iter()
                .flatten()
                .filter(|m| *m == id)
                .count() as u32;
        placed + worn + self.overflow.get(id).copied().unwrap_or(0)
    }

    pub fn totals(&self) -> BTreeMap<String, u32> {
        let mut out: BTreeMap<String, u32> = BTreeMap::new();
        for p in &self.placed {
            *out.entry(p.id.clone()).or_default() += p.qty;
        }
        for id in self.equipment.worn().chain(
            self.equipment
                .weapon_mods
                .iter()
                .flatten()
                .map(String::as_str),
        ) {
            *out.entry(id.to_string()).or_default() += 1;
        }
        for (id, n) in &self.overflow {
            *out.entry(id.clone()).or_default() += n;
        }
        out.retain(|_, n| *n > 0);
        out
    }

    /// Adds `qty` units: tops up stacks, then opens new ones wherever they
    /// fit. What does not fit waits in `overflow`; returns how many waited.
    pub fn add(&mut self, id: &str, qty: u32) -> u32 {
        let stack = info(id).stack.max(1);
        let mut left = qty;
        for p in self
            .placed
            .iter_mut()
            .filter(|p| p.id == id && p.qty < stack)
        {
            let take = (stack - p.qty).min(left);
            p.qty += take;
            left -= take;
        }
        while left > 0 {
            let chunk = left.min(stack);
            let Some((x, y, rot)) = self.find_space(id) else {
                *self.overflow.entry(id.to_string()).or_default() += left;
                return left;
            };
            self.placed.push(Placed {
                id: id.to_string(),
                x,
                y,
                rot,
                qty: chunk,
            });
            left -= chunk;
        }
        0
    }

    /// Removes up to `qty` units, waiting ones first, then the emptiest
    /// stacks, and only then what is worn. Returns how many were removed.
    pub fn remove(&mut self, id: &str, qty: u32) -> u32 {
        let mut left = qty;
        if let Some(n) = self.overflow.get_mut(id) {
            let take = (*n).min(left);
            *n -= take;
            left -= take;
            if *n == 0 {
                self.overflow.remove(id);
            }
        }
        while left > 0 {
            let Some(i) = self
                .placed
                .iter()
                .enumerate()
                .filter(|(_, p)| p.id == id)
                .min_by_key(|(_, p)| p.qty)
                .map(|(i, _)| i)
            else {
                break;
            };
            let take = self.placed[i].qty.min(left);
            self.placed[i].qty -= take;
            left -= take;
            if self.placed[i].qty == 0 {
                self.placed.remove(i);
            }
        }
        for slot in Slot::ALL {
            if left > 0 && self.equipment.get(slot) == Some(id) {
                *self.equipment.slot_mut(slot) = None;
                left -= 1;
            }
        }
        for m in self.equipment.weapon_mods.iter_mut() {
            if left > 0 && m.as_deref() == Some(id) {
                *m = None;
                left -= 1;
            }
        }
        qty - left
    }

    /// Makes the pack agree with the world's counts: adds what was gained,
    /// removes what was spent or sold, drops entries that no longer fit the
    /// grid, then lets waiting items in.
    pub fn reconcile(&mut self, counts: &BTreeMap<String, u32>) {
        if self.equipment.weapon_mods.len() != WEAPON_SOCKETS {
            self.equipment.weapon_mods.resize(WEAPON_SOCKETS, None);
        }
        // A saved layout from another grid size, or a broken one, is rebuilt.
        let old = std::mem::take(&mut self.placed);
        let mut stray: Vec<Placed> = Vec::new();
        for p in old {
            let (w, h) = p.size();
            if p.qty > 0 && self.can_place(w, h, p.x as i32, p.y as i32, None) {
                self.placed.push(p);
            } else if p.qty > 0 {
                stray.push(p);
            }
        }
        for p in stray {
            self.add(&p.id, p.qty);
        }
        let mut ids: BTreeSet<String> = counts.keys().cloned().collect();
        ids.extend(self.totals().into_keys());
        for id in ids {
            let want = counts.get(&id).copied().unwrap_or(0);
            let have = self.count(&id);
            if have > want {
                self.remove(&id, have - want);
            } else if have < want {
                self.add(&id, want - have);
            }
        }
        self.settle();
    }

    /// Lets items waiting in `overflow` into any space that has opened.
    pub fn settle(&mut self) {
        for (id, n) in std::mem::take(&mut self.overflow) {
            self.add(&id, n);
        }
    }

    /// Moves a stack (turned or not) so its top-left is at (x, y).
    pub fn move_to(&mut self, index: usize, x: i32, y: i32, rot: bool) -> Result<(), Refuse> {
        let id = self
            .placed
            .get(index)
            .ok_or(Refuse::NothingThere)?
            .id
            .clone();
        let (w, h) = size_of(&id, rot);
        if !self.can_place(w, h, x, y, Some(index)) {
            return Err(Refuse::Blocked);
        }
        let p = &mut self.placed[index];
        p.x = x as u8;
        p.y = y as u8;
        p.rot = rot;
        Ok(())
    }

    /// Turns a stack a quarter turn where it lies, if it still fits.
    pub fn rotate(&mut self, index: usize) -> Result<(), Refuse> {
        let p = self.placed.get(index).ok_or(Refuse::NothingThere)?;
        let (x, y, rot) = (p.x as i32, p.y as i32, !p.rot);
        if info(&p.id).w == info(&p.id).h {
            return Ok(());
        }
        self.move_to(index, x, y, rot)
    }

    /// Pours stack `from` into stack `into` (same item, up to its limit).
    pub fn merge(&mut self, from: usize, into: usize) -> Result<(), Refuse> {
        if from == into || from >= self.placed.len() || into >= self.placed.len() {
            return Err(Refuse::NothingThere);
        }
        if self.placed[from].id != self.placed[into].id {
            return Err(Refuse::CannotMerge);
        }
        let stack = info(&self.placed[into].id).stack;
        let room = stack.saturating_sub(self.placed[into].qty);
        if room == 0 {
            return Err(Refuse::CannotMerge);
        }
        let take = room.min(self.placed[from].qty);
        self.placed[into].qty += take;
        self.placed[from].qty -= take;
        if self.placed[from].qty == 0 {
            self.placed.remove(from);
            self.settle();
        }
        Ok(())
    }

    /// Takes one unit off a stack into a new stack of its own.
    pub fn split_one(&mut self, index: usize) -> Result<usize, Refuse> {
        let p = self.placed.get(index).ok_or(Refuse::NothingThere)?;
        if p.qty < 2 {
            return Err(Refuse::CannotMerge);
        }
        let id = p.id.clone();
        self.placed[index].qty -= 1;
        match self.find_space(&id) {
            Some((x, y, rot)) => {
                self.placed.push(Placed {
                    id,
                    x,
                    y,
                    rot,
                    qty: 1,
                });
                Ok(self.placed.len() - 1)
            }
            None => {
                self.placed[index].qty += 1;
                Err(Refuse::NoRoom)
            }
        }
    }

    /// Drops stack `a` on stack `b`: one unit of each becomes the recipe's
    /// result. Nothing changes if the result has nowhere to go.
    pub fn combine(&mut self, a: usize, b: usize) -> Result<String, Refuse> {
        if a == b || a >= self.placed.len() || b >= self.placed.len() {
            return Err(Refuse::NothingThere);
        }
        let (ia, ib) = (self.placed[a].id.clone(), self.placed[b].id.clone());
        let recipe = recipe_for(&ia, &ib).ok_or(Refuse::NoRecipe)?;
        // Two of the same kind (herb + herb) need two units to spend.
        if ia == ib && self.count(&ia) < 2 {
            return Err(Refuse::NoRecipe);
        }
        let mut trial = self.clone();
        let (spot_x, spot_y) = (self.placed[b].x, self.placed[b].y);
        trial.placed[a].qty -= 1;
        trial.placed[b].qty -= 1;
        trial.placed.retain(|p| p.qty > 0);
        let result = recipe.result;
        let (w, h) = size_of(result, false);
        // Prefer the target's own cell, so the result appears where you dropped.
        let mut placed_here = false;
        if trial.can_place(w, h, spot_x as i32, spot_y as i32, None) {
            let stack = info(result).stack.max(1);
            if let Some(same) = trial
                .placed
                .iter_mut()
                .find(|p| p.id == result && p.qty < stack)
            {
                same.qty += 1;
            } else {
                trial.placed.push(Placed {
                    id: result.to_string(),
                    x: spot_x,
                    y: spot_y,
                    rot: false,
                    qty: 1,
                });
            }
            placed_here = true;
        }
        if !placed_here && trial.add(result, 1) > 0 {
            return Err(Refuse::NoRoom);
        }
        *self = trial;
        self.settle();
        Ok(result.to_string())
    }

    /// Puts one unit of a stack on its body slot; whatever was there goes
    /// back to the grid. Refused if that has no room.
    pub fn equip(&mut self, index: usize) -> Result<Slot, Refuse> {
        let id = self
            .placed
            .get(index)
            .ok_or(Refuse::NothingThere)?
            .id
            .clone();
        let slot = info(&id).slot.ok_or(Refuse::NotEquipment)?;
        let mut trial = self.clone();
        trial.placed[index].qty -= 1;
        trial.placed.retain(|p| p.qty > 0);
        if let Some(old) = trial.equipment.slot_mut(slot).replace(id) {
            if trial.add(&old, 1) > 0 {
                return Err(Refuse::NoRoom);
            }
        }
        *self = trial;
        Ok(slot)
    }

    /// Takes what is worn in `slot` back into the grid.
    pub fn unequip(&mut self, slot: Slot) -> Result<(), Refuse> {
        let mut trial = self.clone();
        let Some(id) = trial.equipment.slot_mut(slot).take() else {
            return Err(Refuse::NothingThere);
        };
        if trial.add(&id, 1) > 0 {
            return Err(Refuse::NoRoom);
        }
        *self = trial;
        Ok(())
    }

    /// Equips `id` from the grid if the slot is empty (a new relic goes on
    /// by itself). Returns whether it was worn.
    pub fn equip_if_empty(&mut self, id: &str) -> bool {
        let Some(slot) = info(id).slot else {
            return false;
        };
        if self.equipment.get(slot).is_some() {
            return false;
        }
        match self.placed.iter().position(|p| p.id == id) {
            Some(i) => self.equip(i).is_ok(),
            None => false,
        }
    }

    /// Repacks the grid: big shapes first, upright where possible.
    pub fn arrange(&mut self) {
        let mut stacks = std::mem::take(&mut self.placed);
        stacks.sort_by(|a, b| {
            let (ia, ib) = (info(&a.id), info(&b.id));
            let area = |i: Info| i.w as u32 * i.h as u32;
            area(ib)
                .cmp(&area(ia))
                .then(ib.h.cmp(&ia.h))
                .then(a.id.cmp(&b.id))
                .then(b.qty.cmp(&a.qty))
        });
        for s in stacks {
            match self.find_space(&s.id) {
                Some((x, y, rot)) => self.placed.push(Placed { x, y, rot, ..s }),
                None => *self.overflow.entry(s.id).or_default() += s.qty,
            }
        }
        self.settle();
    }

    /// Whether every stack lies inside the grid without overlap.
    pub fn is_valid(&self) -> bool {
        self.placed.iter().enumerate().all(|(i, p)| {
            let (w, h) = p.size();
            p.qty > 0 && self.can_place(w, h, p.x as i32, p.y as i32, Some(i)) && {
                // No overlap with any other stack.
                self.placed.iter().enumerate().all(|(j, q)| {
                    if i == j {
                        return true;
                    }
                    let (qw, qh) = q.size();
                    !(p.x < q.x + qw && q.x < p.x + w && p.y < q.y + qh && q.y < p.y + h)
                })
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(list: &[(&str, u32)]) -> BTreeMap<String, u32> {
        list.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    const ALL_IDS: [&str; 25] = [
        "pocion_pequena",
        "hierbas_curativas",
        "pan_del_dia",
        "tela_resistente",
        "espada_de_madera",
        "espada_reliquia",
        "pocion_misteriosa",
        "esencia_carmesi",
        "tinta_runica",
        "anillo_misterioso",
        "fragmento_sello",
        "llave_boveda",
        "perfume_sucubo",
        "cristal_memoria",
        "amuleto_ocultacion",
        "mapa_mercado_bajo",
        "caja_cerrada",
        "hueso_viejo",
        "ceniza_arcana",
        "gelatina_azul",
        "gelatina_verde",
        "craneo_campeon",
        "esquirla_corazon",
        "cataplasma_herbal",
        "sello_inscrito",
    ];

    #[test]
    fn every_item_fits_the_empty_grid_upright_or_turned() {
        let pack = Backpack::new();
        for id in ALL_IDS
            .iter()
            .chain(["pocion_reforzada", "elixir_carmesi"].iter())
        {
            let i = info(id);
            assert!(i.w >= 1 && i.h >= 1 && i.stack >= 1, "{id}");
            assert!(pack.find_space(id).is_some(), "{id} must fit an empty pack");
        }
    }

    #[test]
    fn stacks_fill_then_open_new_stacks_and_the_rest_waits() {
        let mut pack = Backpack::new();
        assert_eq!(pack.add("hierbas_curativas", 14), 0);
        let stacks: Vec<u32> = pack.placed.iter().map(|p| p.qty).collect();
        assert_eq!(stacks, vec![6, 6, 2]);
        // Fill the rest of the grid with big things until something waits.
        let mut waited = 0;
        for _ in 0..12 {
            waited += pack.add("craneo_campeon", 1);
        }
        assert!(waited > 0, "a 40-cell pack cannot hold twelve 2x2 skulls");
        assert!(pack.is_valid());
        assert_eq!(pack.count("craneo_campeon"), 12);
        assert_eq!(pack.overflow.get("craneo_campeon").copied(), Some(waited));
    }

    #[test]
    fn reconcile_matches_counts_and_keeps_the_layout_valid() {
        let mut pack = Backpack::new();
        let mut want = counts(&[]);
        for (k, id) in ALL_IDS.iter().enumerate() {
            want.insert(id.to_string(), 1 + (k as u32 * 3) % 7);
        }
        pack.reconcile(&want);
        assert!(pack.is_valid());
        assert_eq!(pack.totals(), want);
        // Spend some, gain some, lose an item entirely.
        want.insert("hueso_viejo".into(), 2);
        want.insert("ceniza_arcana".into(), 30);
        want.remove("gelatina_azul");
        pack.reconcile(&want);
        assert!(pack.is_valid());
        assert_eq!(pack.totals(), want);
    }

    #[test]
    fn reconcile_repairs_a_layout_saved_for_another_grid() {
        let mut pack = Backpack::new();
        pack.placed.push(Placed {
            id: "espada_reliquia".into(),
            x: 7,
            y: 3,
            rot: false,
            qty: 1,
        });
        pack.placed.push(Placed {
            id: "hueso_viejo".into(),
            x: 0,
            y: 0,
            rot: false,
            qty: 3,
        });
        pack.placed.push(Placed {
            id: "hueso_viejo".into(),
            x: 0,
            y: 0,
            rot: false,
            qty: 2,
        });
        pack.reconcile(&counts(&[("espada_reliquia", 1), ("hueso_viejo", 5)]));
        assert!(pack.is_valid());
        assert_eq!(pack.count("espada_reliquia"), 1);
        assert_eq!(pack.count("hueso_viejo"), 5);
    }

    #[test]
    fn moving_and_turning_respect_bounds_and_neighbours() {
        let mut pack = Backpack::new();
        pack.add("espada_de_madera", 1); // 1x4 at (0,0)
        pack.add("pan_del_dia", 1); // 2x1 at (1,0)
        let sword = 0;
        let bread = 1;
        assert_eq!(
            pack.move_to(sword, 8, 0, false),
            Err(Refuse::Blocked),
            "outside the grid"
        );
        assert_eq!(
            pack.move_to(sword, 0, 3, false),
            Err(Refuse::Blocked),
            "1x4 cannot start on row 3 of 5"
        );
        assert_eq!(
            pack.move_to(bread, 0, 0, false),
            Err(Refuse::Blocked),
            "the sword is there"
        );
        assert_eq!(pack.move_to(sword, 3, 0, false), Ok(()));
        // Turned, the sword is 4x1; from x=3 that reaches x=6: fine.
        assert_eq!(pack.rotate(sword), Ok(()));
        assert_eq!(pack.placed[sword].size(), (4, 1));
        // Turned at x=5 it would pass the right edge, and it cannot move back.
        assert_eq!(pack.move_to(sword, 5, 0, true), Err(Refuse::Blocked));
        assert!(pack.is_valid());
    }

    #[test]
    fn merging_stops_at_the_stack_limit_and_needs_the_same_item() {
        let mut pack = Backpack::new();
        pack.add("hierbas_curativas", 4);
        pack.placed.push(Placed {
            id: "hierbas_curativas".into(),
            x: 5,
            y: 4,
            rot: false,
            qty: 5,
        });
        pack.add("pan_del_dia", 1);
        assert_eq!(
            pack.merge(0, 2),
            Err(Refuse::CannotMerge),
            "bread is not a herb"
        );
        assert_eq!(pack.merge(0, 1), Ok(()));
        assert_eq!(
            pack.placed
                .iter()
                .filter(|p| p.id == "hierbas_curativas")
                .map(|p| p.qty)
                .sum::<u32>(),
            9
        );
        assert!(
            pack.placed.iter().any(|p| p.qty == 6),
            "the target fills to its limit"
        );
        assert_eq!(pack.count("hierbas_curativas"), 9);
    }

    #[test]
    fn a_stack_can_be_split_one_at_a_time() {
        let mut pack = Backpack::new();
        pack.add("pocion_pequena", 3);
        let new = pack.split_one(0).unwrap();
        assert_eq!(pack.placed[0].qty, 2);
        assert_eq!(pack.placed[new].qty, 1);
        assert_eq!(pack.split_one(new), Err(Refuse::CannotMerge));
        assert!(pack.is_valid());
    }

    #[test]
    fn combining_spends_one_of_each_and_makes_the_result() {
        let mut pack = Backpack::new();
        pack.add("hierbas_curativas", 2);
        pack.add("pocion_pequena", 1);
        let herbs = pack
            .placed
            .iter()
            .position(|p| p.id == "hierbas_curativas")
            .unwrap();
        let potion = pack
            .placed
            .iter()
            .position(|p| p.id == "pocion_pequena")
            .unwrap();
        assert_eq!(
            pack.combine(herbs, potion).as_deref(),
            Ok("pocion_reforzada")
        );
        assert_eq!(pack.count("hierbas_curativas"), 1);
        assert_eq!(pack.count("pocion_pequena"), 0);
        assert_eq!(pack.count("pocion_reforzada"), 1);
        assert!(pack.is_valid());
        // Herb + herb: two separate stacks, one unit spent from each.
        let mut two = Backpack::new();
        two.placed.push(Placed {
            id: "hierbas_curativas".into(),
            x: 0,
            y: 0,
            rot: false,
            qty: 1,
        });
        two.placed.push(Placed {
            id: "hierbas_curativas".into(),
            x: 1,
            y: 0,
            rot: false,
            qty: 1,
        });
        assert_eq!(two.combine(0, 1).as_deref(), Ok("cataplasma_herbal"));
        assert_eq!(two.count("hierbas_curativas"), 0);
        assert_eq!(two.count("cataplasma_herbal"), 1);
        assert_eq!(two.combine(0, 0), Err(Refuse::NothingThere));
    }

    #[test]
    fn combining_with_no_room_for_the_result_changes_nothing() {
        let mut pack = Backpack::new();
        pack.add("hierbas_curativas", 3);
        pack.add("pocion_pequena", 2);
        while let Some((x, y, rot)) = pack.find_space("ceniza_arcana") {
            pack.placed.push(Placed {
                id: "gelatina_azul".into(),
                x,
                y,
                rot,
                qty: 1,
            });
        }
        let before = pack.clone();
        let herb = pack
            .placed
            .iter()
            .position(|p| p.id == "hierbas_curativas")
            .unwrap();
        let potion = pack
            .placed
            .iter()
            .position(|p| p.id == "pocion_pequena")
            .unwrap();
        let jelly = pack
            .placed
            .iter()
            .position(|p| p.id == "gelatina_azul")
            .unwrap();
        assert_eq!(pack.combine(herb, potion), Err(Refuse::NoRoom));
        assert_eq!(pack.combine(herb, jelly), Err(Refuse::NoRecipe));
        assert_eq!(
            pack, before,
            "a refused combination must not spend anything"
        );
    }

    #[test]
    fn equipping_swaps_the_old_item_back_and_refuses_what_is_not_gear() {
        let mut pack = Backpack::new();
        pack.add("espada_de_madera", 1);
        pack.add("espada_reliquia", 1);
        pack.add("hierbas_curativas", 1);
        let wooden = pack
            .placed
            .iter()
            .position(|p| p.id == "espada_de_madera")
            .unwrap();
        assert_eq!(pack.equip(wooden), Ok(Slot::Weapon));
        assert_eq!(pack.equipment.weapon.as_deref(), Some("espada_de_madera"));
        let relic = pack
            .placed
            .iter()
            .position(|p| p.id == "espada_reliquia")
            .unwrap();
        assert_eq!(pack.equip(relic), Ok(Slot::Weapon));
        assert_eq!(pack.equipment.weapon.as_deref(), Some("espada_reliquia"));
        assert!(
            pack.placed.iter().any(|p| p.id == "espada_de_madera"),
            "the old sword returns to the pack"
        );
        let herb = pack
            .placed
            .iter()
            .position(|p| p.id == "hierbas_curativas")
            .unwrap();
        assert_eq!(pack.equip(herb), Err(Refuse::NotEquipment));
        assert_eq!(
            pack.count("espada_reliquia"),
            1,
            "worn items still count as owned"
        );
        assert!(pack.is_valid());
        assert_eq!(pack.unequip(Slot::Weapon), Ok(()));
        assert_eq!(pack.unequip(Slot::Weapon), Err(Refuse::NothingThere));
        assert_eq!(pack.equipment.weapon, None);
    }

    #[test]
    fn unequipping_needs_room_and_leaves_the_gear_on_when_there_is_none() {
        let mut pack = Backpack::new();
        pack.equipment.weapon = Some("espada_reliquia".into());
        while pack.find_space("craneo_campeon").is_some() {
            pack.add("craneo_campeon", 1);
        }
        while let Some((x, y, rot)) = pack.find_space("ceniza_arcana") {
            pack.placed.push(Placed {
                id: "gelatina_azul".into(),
                x,
                y,
                rot,
                qty: 1,
            });
        }
        assert_eq!(pack.unequip(Slot::Weapon), Err(Refuse::NoRoom));
        assert_eq!(pack.equipment.weapon.as_deref(), Some("espada_reliquia"));
    }

    #[test]
    fn waiting_items_come_in_when_space_opens() {
        let mut pack = Backpack::new();
        for _ in 0..12 {
            pack.add("craneo_campeon", 1);
        }
        assert!(!pack.overflow.is_empty());
        let waiting = pack.overflow["craneo_campeon"];
        pack.remove("craneo_campeon", 2);
        pack.settle();
        assert!(pack.overflow.get("craneo_campeon").copied().unwrap_or(0) < waiting);
        assert!(pack.is_valid());
    }

    #[test]
    fn arranging_packs_tightly_keeps_everything_and_never_overlaps() {
        let mut pack = Backpack::new();
        let mut want = BTreeMap::new();
        for (k, id) in ALL_IDS.iter().enumerate() {
            want.insert(id.to_string(), 1 + (k as u32 % 3));
        }
        pack.reconcile(&want);
        // Scatter it first.
        for i in 0..pack.placed.len() {
            let (x, y, rot) = ((i * 3 % 8) as i32, (i * 2 % 5) as i32, i % 2 == 0);
            let _ = pack.move_to(i, x, y, rot);
        }
        let before = pack.totals();
        let used = pack.used_cells();
        pack.arrange();
        assert!(pack.is_valid());
        assert_eq!(pack.totals(), before);
        assert!(pack.used_cells() >= used.min(pack.capacity()) - pack.overflow.len() * 4);
    }

    #[test]
    fn the_layout_survives_a_save_round_trip() {
        let mut pack = Backpack::new();
        pack.reconcile(&counts(&[
            ("espada_de_madera", 1),
            ("hueso_viejo", 9),
            ("anillo_misterioso", 1),
        ]));
        pack.equip_if_empty("espada_de_madera");
        let json = serde_json::to_string(&pack).unwrap();
        let back: Backpack = serde_json::from_str(&json).unwrap();
        assert_eq!(back, pack);
        // An old save without equipment or overflow still loads.
        let old: Backpack = serde_json::from_str(r#"{"cols":8,"rows":5,"placed":[]}"#).unwrap();
        assert!(old.equipment.weapon.is_none() && old.overflow.is_empty());
    }

    #[test]
    fn reconcile_takes_from_what_waits_before_touching_worn_gear() {
        let mut pack = Backpack::new();
        pack.equipment.ring = Some("anillo_misterioso".into());
        pack.reconcile(&counts(&[("anillo_misterioso", 1)]));
        assert_eq!(pack.equipment.ring.as_deref(), Some("anillo_misterioso"));
        assert_eq!(pack.placed.len(), 0);
        pack.reconcile(&counts(&[]));
        assert_eq!(pack.equipment.ring, None, "sold away, so no longer worn");
    }

    #[test]
    fn recipes_only_use_known_items_and_are_symmetric() {
        for r in RECIPES.iter() {
            assert!(recipe_for(r.a, r.b).is_some() && recipe_for(r.b, r.a).is_some());
            assert!(
                extra(r.result).is_some(),
                "{} needs a description",
                r.result
            );
            assert!(recipes_with(r.a).iter().any(|x| x.result == r.result));
        }
        assert!(recipe_for("hueso_viejo", "pan_del_dia").is_none());
    }

    #[test]
    fn a_relic_that_arrives_wears_itself_only_when_the_slot_is_empty() {
        let mut pack = Backpack::new();
        pack.reconcile(&counts(&[("espada_reliquia", 1)]));
        assert!(pack.equip_if_empty("espada_reliquia"));
        assert!(!pack.equip_if_empty("espada_reliquia"));
        assert!(weapon_damage("espada_reliquia") > weapon_damage("espada_de_madera"));
    }
}
