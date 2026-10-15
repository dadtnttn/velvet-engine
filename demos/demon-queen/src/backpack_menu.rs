//! The pause menu's grid backpack and equipment screens. Items are picked up
//! and dropped on a limited grid (click, or Enter on the keyboard); dropping
//! one on another combines or stacks them; the worn gear has its own page.
//! The rules live in `exploration3d::backpack`; this file is input and paint.
use crate::exploration3d::backpack::{self, Backpack, Refuse, Slot};
use crate::exploration3d::ui_sfx::{self, UiSfx};
use crate::exploration3d::InventoryMenuItem;
use crate::localization::Language;
use crate::menu_ui as ui;
use crate::prompt_icons::Icon;
use crate::render::{Canvas, Fonts};
use crate::{ellipsize, rect_contains, App};
use std::time::{Duration, Instant};
use winit::keyboard::KeyCode;

pub type Rect = (i32, i32, i32, i32);

pub const CELL: i32 = 88;
pub const PANE_L: Rect = (480, 226, 800, 744);
pub const PANE_R: Rect = (1304, 226, 506, 744);
const GRID_Y: i32 = 292;
const BTN_SORT: Rect = (512, 832, 138, 54);
const BTN_ROTATE: Rect = (659, 832, 138, 54);
const BTN_EQUIP: Rect = (806, 832, 138, 54);
const BTN_SPLIT: Rect = (953, 832, 138, 54);
const BTN_USE: Rect = (1100, 832, 148, 54);
/// Where the equipment cards and candidate rows sit.
pub fn slot_card(i: usize) -> Rect {
    (512, 268 + i as i32 * 146, 736, 130)
}
pub fn slot_remove(i: usize) -> Rect {
    let c = slot_card(i);
    (c.0 + c.2 - 150, c.1 + 42, 126, 46)
}
pub fn candidate_row(k: usize) -> Rect {
    (1328, 290 + k as i32 * 84, 458, 72)
}
pub const MAX_CANDIDATES: usize = 7;

/// English keeps its own words; every other language draws the Spanish source, which the
/// draw-time translation (the game's text catalogs) turns into that language.
pub fn tr(language: Language, es: &'static str, en: &'static str) -> &'static str {
    if language != Language::English {
        es
    } else {
        en
    }
}

/// A stack lifted off the grid and following the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub index: usize,
    pub rot: bool,
}

/// Menu-side state of the two screens.
#[derive(Clone, Debug, Default)]
pub struct Ui {
    /// The cell the keyboard (or the last click) points at.
    pub cursor: (u8, u8),
    pub held: Option<Held>,
    note: String,
    note_until: Option<Instant>,
    /// Selected equipment slot (index into `Slot::ALL`).
    pub slot: usize,
}

impl Ui {
    pub fn set_note(&mut self, text: impl Into<String>) {
        self.note = text.into();
        self.note_until = Some(Instant::now() + Duration::from_secs(4));
    }
    pub fn note(&self) -> Option<&str> {
        self.note_until
            .filter(|t| Instant::now() < *t)
            .map(|_| self.note.as_str())
    }
}

pub fn grid_origin(pack: &Backpack) -> (i32, i32) {
    (PANE_L.0 + (PANE_L.2 - pack.cols as i32 * CELL) / 2, GRID_Y)
}

pub fn cell_at(pack: &Backpack, x: i32, y: i32) -> Option<(u8, u8)> {
    let (gx, gy) = grid_origin(pack);
    let (cx, cy) = ((x - gx).div_euclid(CELL), (y - gy).div_euclid(CELL));
    let inside = x >= gx && y >= gy && cx < pack.cols as i32 && cy < pack.rows as i32;
    inside.then_some((cx as u8, cy as u8))
}

fn stack_rect(pack: &Backpack, x: i32, y: i32, w: u8, h: u8) -> Rect {
    let (gx, gy) = grid_origin(pack);
    (
        gx + x * CELL + 3,
        gy + y * CELL + 3,
        w as i32 * CELL - 6,
        h as i32 * CELL - 6,
    )
}

/// What dropping the held stack on `cell` would do.
#[derive(Clone, Debug, PartialEq)]
pub enum Plan {
    Place {
        x: i32,
        y: i32,
    },
    Merge(usize),
    Combine(usize, String),
    /// A recipe exists but cannot be used right now.
    Locked(usize),
    Blocked,
}

pub fn plan_drop(pack: &Backpack, held: Held, cell: (i32, i32), craft: bool) -> Plan {
    let Some(me) = pack.placed.get(held.index) else {
        return Plan::Blocked;
    };
    let inside =
        cell.0 >= 0 && cell.1 >= 0 && cell.0 < pack.cols as i32 && cell.1 < pack.rows as i32;
    if inside {
        if let Some(t) = pack
            .at(cell.0 as u8, cell.1 as u8)
            .filter(|t| *t != held.index)
        {
            let other = &pack.placed[t];
            if other.id == me.id {
                if other.qty < backpack::info(&me.id).stack {
                    return Plan::Merge(t);
                }
            } else if let Some(recipe) = backpack::recipe_for(&me.id, &other.id) {
                return if craft {
                    Plan::Combine(t, recipe.result.to_string())
                } else {
                    Plan::Locked(t)
                };
            }
        }
    }
    let (w, h) = backpack::size_of(&me.id, held.rot);
    let anchors = [
        (cell.0 - (w as i32 - 1) / 2, cell.1 - (h as i32 - 1) / 2),
        cell,
    ];
    for (ax, ay) in anchors {
        let x = ax.clamp(0, (pack.cols as i32 - w as i32).max(0));
        let y = ay.clamp(0, (pack.rows as i32 - h as i32).max(0));
        if pack.can_place(w, h, x, y, Some(held.index)) {
            return Plan::Place { x, y };
        }
    }
    Plan::Blocked
}

pub enum Outcome {
    Moved,
    Merged,
    Combined(String),
    Refused(Refuse),
    CraftLocked,
}

pub fn apply_plan(pack: &mut Backpack, held: Held, plan: Plan) -> Outcome {
    match plan {
        Plan::Place { x, y } => match pack.move_to(held.index, x, y, held.rot) {
            Ok(()) => Outcome::Moved,
            Err(e) => Outcome::Refused(e),
        },
        Plan::Merge(t) => match pack.merge(held.index, t) {
            Ok(()) => Outcome::Merged,
            Err(e) => Outcome::Refused(e),
        },
        Plan::Combine(t, _) => match pack.combine(held.index, t) {
            Ok(id) => Outcome::Combined(id),
            Err(e) => Outcome::Refused(e),
        },
        Plan::Locked(_) => Outcome::CraftLocked,
        Plan::Blocked => Outcome::Refused(Refuse::Blocked),
    }
}

fn refusal(language: Language, why: Refuse) -> &'static str {
    match why {
        Refuse::Blocked => tr(language, "No cabe ahí.", "It does not fit there."),
        Refuse::NoRoom => tr(
            language,
            "No queda espacio en la mochila.",
            "The backpack has no room.",
        ),
        Refuse::NoRecipe => tr(
            language,
            "Esos objetos no se combinan.",
            "Those items do not combine.",
        ),
        Refuse::NotEquipment => tr(
            language,
            "Ese objeto no se puede equipar.",
            "That cannot be equipped.",
        ),
        Refuse::CannotMerge => tr(language, "No se pueden juntar.", "They cannot be stacked."),
        Refuse::NothingThere => tr(language, "No hay nada ahí.", "Nothing there."),
    }
}

fn item<'a>(items: &'a [InventoryMenuItem], id: &str) -> Option<&'a InventoryMenuItem> {
    items.iter().find(|i| i.id == id)
}

fn name_of(items: &[InventoryMenuItem], id: &str) -> String {
    item(items, id)
        .map(|i| i.name.clone())
        .unwrap_or_else(|| id.replace('_', " "))
}

fn tint_of(items: &[InventoryMenuItem], id: &str) -> u32 {
    if backpack::info(id).slot.is_some() {
        return 0x86b8f2;
    }
    match item(items, id).map(|i| i.category.as_str()) {
        Some("consumables") => 0x6fd3a8,
        Some("materials") => 0xd6a662,
        Some("equipment") => 0x86b8f2,
        Some("special") => 0xc094f2,
        _ => 0xbfb2c9,
    }
}

impl App {
    fn bp_pack(&mut self) -> Option<&mut backpack::Backpack> {
        self.pause_backpack.as_mut()
    }

    fn bp_sound(&self, sfx: UiSfx) {
        ui_sfx::play(sfx);
    }

    fn bp_changed(&mut self) {
        self.backpack_dirty = true;
        self.needs_paint = true;
    }

    fn bp_say(&mut self, text: impl Into<String>) {
        if let Some(menu) = &mut self.pause {
            menu.bp.set_note(text);
        }
        self.needs_paint = true;
    }

    /// The stack the actions of the page apply to: the one in hand, else the
    /// one under the cursor cell.
    fn bp_target(&self) -> Option<usize> {
        let menu = self.pause.as_ref()?;
        let pack = self.pause_backpack.as_ref()?;
        menu.bp
            .held
            .map(|h| h.index)
            .or_else(|| pack.at(menu.bp.cursor.0, menu.bp.cursor.1))
    }

    fn bp_pick_or_drop(&mut self, cell: (u8, u8)) {
        let language = self.localizer.language();
        let craft = self.pause_backpack_craft;
        let Some(held) = self.pause.as_ref().and_then(|m| m.bp.held) else {
            // Pick up whatever lies there.
            let picked = self
                .pause_backpack
                .as_ref()
                .and_then(|p| p.at(cell.0, cell.1).map(|i| (i, p.placed[i].rot)));
            if picked.is_some() {
                self.bp_sound(UiSfx::Pick);
            }
            if let Some(menu) = &mut self.pause {
                menu.bp.cursor = cell;
                menu.bp.held = picked.map(|(index, rot)| Held { index, rot });
            }
            self.needs_paint = true;
            return;
        };
        let Some(pack) = self.pause_backpack.as_mut() else {
            return;
        };
        let plan = plan_drop(pack, held, (cell.0 as i32, cell.1 as i32), craft);
        let outcome = apply_plan(pack, held, plan);
        let note = match &outcome {
            Outcome::Moved => None,
            Outcome::Merged => Some(tr(language, "Apilados.", "Stacked.").to_string()),
            Outcome::Combined(id) => {
                let name = self
                    .pause_inventory_items
                    .iter()
                    .find(|i| i.id == *id)
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|| id.clone());
                Some(format!("{}: {name}", tr(language, "Combinado", "Combined")))
            }
            Outcome::Refused(why) => Some(refusal(language, *why).to_string()),
            Outcome::CraftLocked => Some(
                tr(
                    language,
                    "Combina objetos fuera de la cueva.",
                    "Combine items outside the cave.",
                )
                .to_string(),
            ),
        };
        let success = matches!(
            outcome,
            Outcome::Moved | Outcome::Merged | Outcome::Combined(_)
        );
        self.bp_sound(match outcome {
            Outcome::Moved | Outcome::Merged => UiSfx::Drop,
            Outcome::Combined(_) => UiSfx::Fuse,
            Outcome::Refused(_) | Outcome::CraftLocked => UiSfx::Error,
        });
        if let Some(menu) = &mut self.pause {
            menu.bp.cursor = cell;
            if success {
                menu.bp.held = None;
            }
        }
        if let Some(note) = note {
            self.bp_say(note);
        }
        if success {
            self.bp_changed();
        }
        self.needs_paint = true;
    }

    fn bp_rotate(&mut self) {
        let language = self.localizer.language();
        let held = self.pause.as_ref().and_then(|m| m.bp.held);
        if let Some(held) = held {
            if let Some(menu) = &mut self.pause {
                menu.bp.held = Some(Held {
                    rot: !held.rot,
                    ..held
                });
            }
            self.bp_sound(UiSfx::Rotate);
            self.needs_paint = true;
            return;
        }
        let Some(index) = self.bp_target() else {
            return;
        };
        let result = self.bp_pack().map(|p| p.rotate(index));
        match result {
            Some(Ok(())) => {
                self.bp_sound(UiSfx::Rotate);
                self.bp_changed()
            }
            Some(Err(why)) => {
                self.bp_sound(UiSfx::Error);
                self.bp_say(refusal(language, why))
            }
            None => {}
        }
    }

    fn bp_equip(&mut self) {
        let language = self.localizer.language();
        let Some(index) = self.bp_target() else {
            return;
        };
        let result = self.bp_pack().map(|p| {
            p.equip(index)
                .map(|slot| (slot, p.equipment.get(slot).map(str::to_string)))
        });
        match result {
            Some(Ok((slot, id))) => {
                if let Some(menu) = &mut self.pause {
                    menu.bp.held = None;
                    menu.bp.slot = Slot::ALL.iter().position(|s| *s == slot).unwrap_or(0);
                }
                let name = id
                    .map(|id| name_of(&self.pause_inventory_items, &id))
                    .unwrap_or_default();
                self.bp_sound(UiSfx::Equip);
                self.bp_say(format!("{}: {name}", tr(language, "Equipado", "Equipped")));
                self.bp_changed();
            }
            Some(Err(why)) => {
                self.bp_sound(UiSfx::Error);
                self.bp_say(refusal(language, why))
            }
            None => {}
        }
    }

    fn bp_split(&mut self) {
        let language = self.localizer.language();
        let Some(index) = self.bp_target() else {
            return;
        };
        let result = self.bp_pack().map(|p| p.split_one(index));
        match result {
            Some(Ok(new)) => {
                // The new unit is the one to carry to another stack.
                if let Some(menu) = &mut self.pause {
                    menu.bp.held = None;
                }
                let _ = new;
                self.bp_sound(UiSfx::Pick);
                self.bp_changed();
            }
            Some(Err(Refuse::CannotMerge)) => {
                self.bp_sound(UiSfx::Error);
                self.bp_say(tr(language, "Solo hay una unidad.", "There is only one."))
            }
            Some(Err(why)) => {
                self.bp_sound(UiSfx::Error);
                self.bp_say(refusal(language, why))
            }
            None => {}
        }
    }

    /// Eats or drinks what is in hand (or under the cursor): only where vitality counts.
    fn bp_use(&mut self) {
        let language = self.localizer.language();
        let Some(index) = self.bp_target() else {
            return;
        };
        let Some(id) = self.pause_backpack.as_ref().map(|p| p.placed[index].id.clone()) else {
            return;
        };
        let Some(amount) = backpack::cure(&id) else {
            self.bp_sound(UiSfx::Error);
            self.bp_say(tr(language, "Eso no se usa así.", "That is not used that way."));
            return;
        };
        let Some((now, most)) = self.pause_vitality else {
            self.bp_sound(UiSfx::Error);
            self.bp_say(tr(
                language,
                "Guárdalo para la mina o un combate: ahí cura.",
                "Keep it for the mine or a fight: it heals there.",
            ));
            return;
        };
        if now >= most {
            self.bp_sound(UiSfx::Error);
            self.bp_say(tr(language, "Ya tienes toda la vitalidad.", "Your vitality is full."));
            return;
        }
        if let Some(pack) = self.pause_backpack.as_mut() {
            pack.remove(&id, 1);
        }
        if let Some(menu) = &mut self.pause {
            menu.bp.held = None;
        }
        self.pause_vitality = Some(((now + amount).min(most), most));
        self.pause_uses.push(id.clone());
        let name = name_of(&self.pause_inventory_items, &id);
        if let Some(item) = self.pause_inventory_items.iter_mut().find(|i| i.id == id) {
            item.quantity = item.quantity.saturating_sub(1);
        }
        self.bp_sound(UiSfx::Fuse);
        // The word in the player's language before the numbers join it (the whole line is no entry).
        let word = demon_queen_dungeon::i18n::tr(tr(language, "vitalidad", "vitality")).into_owned();
        self.bp_say(format!("{name}: +{amount:.0} {word}"));
        self.bp_changed();
    }

    fn bp_sort(&mut self) {
        let language = self.localizer.language();
        if let Some(menu) = &mut self.pause {
            menu.bp.held = None;
        }
        if let Some(pack) = self.bp_pack() {
            pack.arrange();
        }
        self.bp_sound(UiSfx::Page);
        self.bp_say(tr(language, "Mochila ordenada.", "Backpack sorted."));
        self.bp_changed();
    }

    /// Keys on the grid. Returns whether the key was used.
    pub fn backpack_key(&mut self, key: KeyCode) -> bool {
        let Some(pack) = self.pause_backpack.as_ref() else {
            return false;
        };
        let (cols, rows) = (pack.cols, pack.rows);
        let Some(menu) = &mut self.pause else {
            return false;
        };
        let (mut cx, mut cy) = menu.bp.cursor;
        match key {
            KeyCode::ArrowLeft if cx > 0 => cx -= 1,
            KeyCode::ArrowRight if cx + 1 < cols => cx += 1,
            KeyCode::ArrowUp if cy > 0 => cy -= 1,
            KeyCode::ArrowDown if cy + 1 < rows => cy += 1,
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                // At an edge the arrows stay on the grid; Tab leaves it.
                return matches!(
                    key,
                    KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
                );
            }
            KeyCode::Enter | KeyCode::Space | KeyCode::NumpadEnter => {
                let cell = menu.bp.cursor;
                self.bp_pick_or_drop(cell);
                return true;
            }
            KeyCode::KeyR => {
                self.bp_rotate();
                return true;
            }
            KeyCode::KeyE => {
                self.bp_equip();
                return true;
            }
            KeyCode::KeyV => {
                self.bp_split();
                return true;
            }
            KeyCode::KeyO => {
                self.bp_sort();
                return true;
            }
            KeyCode::KeyU => {
                self.bp_use();
                return true;
            }
            KeyCode::Escape if menu.bp.held.is_some() => {
                menu.bp.held = None;
                ui_sfx::play(UiSfx::Back);
                self.needs_paint = true;
                return true;
            }
            _ => return false,
        }
        if menu.bp.cursor != (cx, cy) {
            ui_sfx::play(UiSfx::Move);
        }
        menu.bp.cursor = (cx, cy);
        self.needs_paint = true;
        true
    }

    /// A click on the backpack page. Returns whether it hit something.
    pub fn backpack_click(&mut self, x: i32, y: i32) -> bool {
        let Some(pack) = self.pause_backpack.as_ref() else {
            return false;
        };
        if let Some(cell) = cell_at(pack, x, y) {
            self.bp_pick_or_drop(cell);
            return true;
        }
        if rect_contains(BTN_SORT, x, y) {
            self.bp_sort();
        } else if rect_contains(BTN_ROTATE, x, y) {
            self.bp_rotate();
        } else if rect_contains(BTN_EQUIP, x, y) {
            self.bp_equip();
        } else if rect_contains(BTN_SPLIT, x, y) {
            self.bp_split();
        } else if rect_contains(BTN_USE, x, y) {
            self.bp_use();
        } else {
            return false;
        }
        true
    }

    /// Right click turns what is in hand (or under the cursor).
    pub fn backpack_right_click(&mut self) -> bool {
        let on_page = self
            .pause
            .as_ref()
            .is_some_and(|m| m.page == crate::pause_menu::Page::Inventory);
        if !on_page
            || self.pause_backpack.is_none()
            || self.pause.as_ref().is_some_and(|m| m.confirm.is_some())
        {
            return false;
        }
        self.bp_rotate();
        true
    }

    /// Items in the pack that can be worn in `slot`, one row per kind.
    pub fn equipment_candidates(&self, slot: Slot) -> Vec<String> {
        let Some(pack) = &self.pause_backpack else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for p in &pack.placed {
            if backpack::info(&p.id).slot == Some(slot) && !out.contains(&p.id) {
                out.push(p.id.clone());
            }
        }
        out.truncate(MAX_CANDIDATES);
        out
    }

    pub fn equipment_control_count(&self) -> usize {
        Slot::ALL.len() + self.equipment_candidates(self.selected_slot()).len()
    }

    pub fn selected_slot(&self) -> Slot {
        let i = self.pause.as_ref().map(|m| m.bp.slot).unwrap_or(0);
        Slot::ALL[i.min(Slot::ALL.len() - 1)]
    }

    pub fn select_slot(&mut self, i: usize) {
        if let Some(menu) = &mut self.pause {
            menu.bp.slot = i.min(Slot::ALL.len() - 1);
        }
    }

    /// Activates control `index` of the equipment page: a slot (take its
    /// item off) or a candidate (put it on).
    pub fn equipment_control(&mut self, index: usize) {
        let language = self.localizer.language();
        if index < Slot::ALL.len() {
            self.select_slot(index);
            let slot = Slot::ALL[index];
            let result = self.bp_pack().map(|p| p.unequip(slot));
            match result {
                Some(Ok(())) => {
                    self.bp_sound(UiSfx::Drop);
                    self.bp_changed()
                }
                Some(Err(Refuse::NothingThere)) => {
                    self.bp_sound(UiSfx::Error);
                    self.bp_say(tr(
                        language,
                        "Esa ranura está vacía.",
                        "That slot is empty.",
                    ))
                }
                Some(Err(why)) => {
                    self.bp_sound(UiSfx::Error);
                    self.bp_say(refusal(language, why))
                }
                None => {}
            }
            return;
        }
        let slot = self.selected_slot();
        let candidates = self.equipment_candidates(slot);
        let Some(id) = candidates.get(index - Slot::ALL.len()).cloned() else {
            return;
        };
        let Some(pos) = self
            .pause_backpack
            .as_ref()
            .and_then(|p| p.placed.iter().position(|s| s.id == id))
        else {
            return;
        };
        let result = self.bp_pack().map(|p| p.equip(pos));
        match result {
            Some(Ok(_)) => {
                let name = name_of(&self.pause_inventory_items, &id);
                self.bp_sound(UiSfx::Equip);
                self.bp_say(format!("{}: {name}", tr(language, "Equipado", "Equipped")));
                self.bp_changed();
            }
            Some(Err(why)) => {
                self.bp_sound(UiSfx::Error);
                self.bp_say(refusal(language, why))
            }
            None => {}
        }
    }

    /// A click on the equipment page. Returns whether it hit something.
    pub fn equipment_click(&mut self, x: i32, y: i32) -> bool {
        if self.pause_backpack.is_none() {
            return false;
        }
        for (i, slot) in Slot::ALL.iter().enumerate() {
            if rect_contains(slot_remove(i), x, y)
                && self
                    .pause_backpack
                    .as_ref()
                    .is_some_and(|p| p.equipment.get(*slot).is_some())
            {
                self.equipment_control(i);
                return true;
            }
            if rect_contains(slot_card(i), x, y) {
                self.select_slot(i);
                if let Some(menu) = &mut self.pause {
                    menu.focus = crate::pause_menu::NAV_COUNT + i;
                }
                self.needs_paint = true;
                return true;
            }
        }
        let slot = self.selected_slot();
        for k in 0..self.equipment_candidates(slot).len() {
            if rect_contains(candidate_row(k), x, y) {
                self.equipment_control(Slot::ALL.len() + k);
                return true;
            }
        }
        false
    }
}

/// Everything the two pages need to paint.
pub struct View<'a> {
    pub fonts: &'a Fonts,
    pub language: Language,
    pub items: &'a [InventoryMenuItem],
    pub gold: u32,
    pub pack: Option<&'a Backpack>,
    pub ui: &'a Ui,
    /// The keyboard is on the grid (or the equipment list).
    pub focused: bool,
    pub focus_index: usize,
    pub mouse: (i32, i32),
    pub light: u32,
    pub craft: bool,
    /// Her vitality where cures count (the mine, a fight), now and at most.
    pub vitality: Option<(f32, f32)>,
}

fn paint_tile(c: &mut Canvas, v: &View, id: &str, rect: Rect, qty: u32, hot: bool, selected: bool) {
    let (x, y, w, h) = rect;
    let tint = tint_of(v.items, id);
    c.round_rect(x, y + 4, w, h, 14, 0x22000000, 0x34000000);
    let (top, bottom) = if hot || selected {
        (
            ui::argb(0xf6, ui::mix(0x30223f, tint, 0.36)),
            ui::argb(0xf6, ui::mix(0x1a1126, tint, 0.20)),
        )
    } else {
        (
            ui::argb(0xf0, ui::mix(0x281d36, tint, 0.22)),
            ui::argb(0xf0, ui::mix(0x150e1f, tint, 0.10)),
        )
    };
    c.round_rect(x, y, w, h, 14, top, bottom);
    let border = if selected {
        ui::argb(0xff, ui::mix(v.light, 0xffffff, 0.35))
    } else if hot {
        ui::argb(0xe8, ui::mix(tint, 0xffffff, 0.3))
    } else {
        ui::argb(0x9a, tint)
    };
    c.round_outline(
        x,
        y,
        w,
        h,
        14,
        if selected {
            2.4
        } else if hot {
            1.8
        } else {
            1.2
        },
        border,
    );
    let side = (w.min(h) - 12).clamp(28, 78);
    let icon = (x + (w - side) / 2, y + (h - side) / 2, side, side);
    if let Some(it) = item(v.items, id) {
        crate::pause_menu::paint_item_icon(c, v.fonts, it, icon, 0xff2a1f36, side as f32 * 0.55);
    }
    let stack = backpack::info(id).stack;
    if stack > 1 && qty >= 1 {
        let label = format!("×{qty}");
        let bw = v.fonts.text_width(&label, 16.0, true) + 14;
        c.round_rect(
            x + w - bw - 6,
            y + h - 28,
            bw,
            22,
            11,
            0xd0100a17,
            0xd0100a17,
        );
        ui::text_center(
            c,
            v.fonts,
            &label,
            x + w - bw / 2 - 6,
            y + h - 17,
            16.0,
            if qty >= stack { ui::GOLD } else { ui::INK },
            true,
        );
    }
    if backpack::info(id).slot.is_some() {
        c.smooth_disc(
            (x + 14) as f32,
            (y + 14) as f32,
            5.0,
            ui::argb(0xff, ui::mix(tint, 0xffffff, 0.4)),
        );
    }
}

/// The grid backpack page.
pub fn paint_backpack(c: &mut Canvas, v: &View) {
    let t = |es, en| tr(v.language, es, en);
    ui::glass(c, PANE_L, 22, v.light, 0.0);
    ui::glass(c, PANE_R, 22, v.light, 0.0);
    let Some(pack) = v.pack else {
        ui::icon_disc(
            c,
            Icon::Backpack,
            880,
            520,
            56,
            ui::mix(v.light, 0xffffff, 0.1),
            false,
        );
        ui::text_center(
            c,
            v.fonts,
            t("Mochila 3D", "3D backpack"),
            880,
            620,
            30.0,
            ui::INK,
            true,
        );
        c.paragraph(
            v.fonts,
            t(
                "Los objetos aparecen al entrar en la exploración 3D.",
                "Items appear when you enter 3D exploration.",
            ),
            560,
            668,
            640,
            34,
            23.0,
            ui::MUTED,
            false,
        );
        return;
    };
    ui::text_mid(
        c,
        v.fonts,
        t("OBJETOS Y MATERIALES", "ITEMS AND MATERIALS"),
        512,
        254,
        18.0,
        ui::GOLD,
        true,
    );
    // Capacity chip and gold.
    let used = pack.used_cells();
    let cap = pack.capacity();
    let label = format!(
        "{used} / {cap} {}",
        demon_queen_dungeon::i18n::tr(t("espacios", "slots"))
    );
    let full = used * 10 >= cap * 9;
    let cw = v.fonts.text_width(&label, 17.0, true) + 34;
    let chip_color = if full {
        ui::DANGER
    } else {
        ui::mix(v.light, 0xffffff, 0.3)
    } & 0x00ff_ffff;
    c.round_rect(
        1248 - cw,
        238,
        cw,
        32,
        16,
        ui::argb(0x33, chip_color),
        ui::argb(0x33, chip_color),
    );
    c.round_outline(1248 - cw, 238, cw, 32, 16, 1.0, ui::argb(0x88, chip_color));
    ui::text_center(
        c,
        v.fonts,
        &label,
        1248 - cw / 2,
        254,
        17.0,
        ui::mix(chip_color, 0xffffff, 0.35),
        true,
    );
    let gold = format!("{}", v.gold);
    let gw = v.fonts.text_width(&gold, 19.0, true) + 60;
    c.round_rect(
        1248 - cw - gw - 12,
        238,
        gw,
        32,
        16,
        ui::argb(0x33, ui::GOLD),
        ui::argb(0x33, ui::GOLD),
    );
    c.smooth_disc((1248 - cw - gw - 12 + 20) as f32, 254.0, 8.0, ui::GOLD);
    c.smooth_ring(
        (1248 - cw - gw - 12 + 20) as f32,
        254.0,
        5.0,
        1.4,
        0xff8a6a2c,
    );
    ui::text_mid(
        c,
        v.fonts,
        &gold,
        1248 - cw - gw - 12 + 36,
        254,
        19.0,
        ui::GOLD,
        true,
    );

    let (gx, gy) = grid_origin(pack);
    let held = v.ui.held;
    let mouse_cell = cell_at(pack, v.mouse.0, v.mouse.1);
    // Cells.
    for cy in 0..pack.rows as i32 {
        for cx in 0..pack.cols as i32 {
            let (x, y) = (gx + cx * CELL, gy + cy * CELL);
            c.round_rect(x + 3, y + 3, CELL - 6, CELL - 6, 10, 0x74140d1c, 0x74100a17);
            c.round_outline(x + 3, y + 3, CELL - 6, CELL - 6, 10, 1.0, 0x1effffff);
        }
    }
    let hover_index = mouse_cell
        .and_then(|(x, y)| pack.at(x, y))
        .filter(|i| Some(*i) != held.map(|h| h.index));
    let cursor_index = pack.at(v.ui.cursor.0, v.ui.cursor.1);
    // Stacks.
    for (i, p) in pack.placed.iter().enumerate() {
        if held.is_some_and(|h| h.index == i) {
            continue;
        }
        let (w, h) = p.size();
        let rect = stack_rect(pack, p.x as i32, p.y as i32, w, h);
        let selected =
            held.is_none() && cursor_index == Some(i) && (v.focused || mouse_cell.is_none());
        paint_tile(c, v, &p.id, rect, p.qty, hover_index == Some(i), selected);
    }
    // The keyboard cursor on an empty cell.
    if v.focused && cursor_index.is_none() && held.is_none() {
        let (x, y) = (
            gx + v.ui.cursor.0 as i32 * CELL,
            gy + v.ui.cursor.1 as i32 * CELL,
        );
        c.round_outline(
            x + 3,
            y + 3,
            CELL - 6,
            CELL - 6,
            10,
            2.4,
            ui::argb(0xff, ui::mix(v.light, 0xffffff, 0.35)),
        );
    }
    // What the held stack would do where the pointer is.
    let mut ghost: Option<(usize, u8, u8, bool)> = None;
    if let Some(h) = held {
        let target_cell = mouse_cell.or(v.focused.then_some(v.ui.cursor));
        if let (Some(me), Some(cell)) = (pack.placed.get(h.index), target_cell) {
            let plan = plan_drop(pack, h, (cell.0 as i32, cell.1 as i32), v.craft);
            let (w, hh) = backpack::size_of(&me.id, h.rot);
            let (color, rect) = match &plan {
                Plan::Place { x, y } => (0x33d17a_u32, stack_rect(pack, *x, *y, w, hh)),
                Plan::Merge(t) | Plan::Combine(t, _) | Plan::Locked(t) => {
                    let other = &pack.placed[*t];
                    let (ow, oh) = other.size();
                    let color = match &plan {
                        Plan::Merge(_) => 0x6ab0f2,
                        Plan::Combine(..) => 0xf2c14e,
                        _ => ui::DANGER & 0x00ff_ffff,
                    };
                    (
                        color,
                        stack_rect(pack, other.x as i32, other.y as i32, ow, oh),
                    )
                }
                Plan::Blocked => {
                    let x = (cell.0 as i32 - (w as i32 - 1) / 2)
                        .clamp(0, (pack.cols as i32 - w as i32).max(0));
                    let y = (cell.1 as i32 - (hh as i32 - 1) / 2)
                        .clamp(0, (pack.rows as i32 - hh as i32).max(0));
                    (ui::DANGER & 0x00ff_ffff, stack_rect(pack, x, y, w, hh))
                }
            };
            c.round_rect(
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                14,
                ui::argb(0x4a, color),
                ui::argb(0x33, color),
            );
            c.round_outline(
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                14,
                2.4,
                ui::argb(0xf0, color),
            );
            if let Plan::Combine(_, result) = &plan {
                let text = format!("→ {}", name_of(v.items, result));
                let tw = v.fonts.text_width(&text, 17.0, true) + 24;
                let (tx, ty) = (rect.0 + rect.2 / 2 - tw / 2, rect.1 - 30);
                c.round_rect(tx, ty, tw, 26, 13, 0xf0100a17, 0xf0100a17);
                c.round_outline(tx, ty, tw, 26, 13, 1.2, ui::argb(0xff, color));
                ui::text_center(
                    c,
                    v.fonts,
                    &text,
                    tx + tw / 2,
                    ty + 13,
                    17.0,
                    ui::mix(color, 0xffffff, 0.4),
                    true,
                );
            }
        }
        if let Some(me) = pack.placed.get(h.index) {
            let (w, hh) = backpack::size_of(&me.id, h.rot);
            ghost = Some((h.index, w, hh, h.rot));
        }
    }

    // Waiting items / hints below the grid.
    if pack.overflow.is_empty() {
        ui::text_mid(
            c,
            v.fonts,
            t(
                "Cada objeto ocupa su forma; ordena y combina para hacer sitio.",
                "Each item takes its own shape; sort and combine to make room.",
            ),
            512,
            774,
            17.0,
            ui::MUTED,
            false,
        );
    } else {
        ui::text_mid(
            c,
            v.fonts,
            t(
                "SIN ESPACIO · fuera de la mochila",
                "NO ROOM · outside the backpack",
            ),
            512,
            762,
            15.0,
            ui::DANGER,
            true,
        );
        let mut x = 512;
        for (id, n) in &pack.overflow {
            let label = format!(
                "{} ×{n}",
                ellipsize(v.fonts, &name_of(v.items, id), 150, 16.0, true)
            );
            let w = v.fonts.text_width(&label, 16.0, true) + 24;
            if x + w > 1248 {
                break;
            }
            c.round_rect(
                x,
                780,
                w,
                30,
                15,
                ui::argb(0x2e, ui::DANGER),
                ui::argb(0x2e, ui::DANGER),
            );
            c.round_outline(x, 780, w, 30, 15, 1.0, ui::argb(0x88, ui::DANGER));
            ui::text_center(c, v.fonts, &label, x + w / 2, 795, 16.0, ui::INK, true);
            x += w + 8;
        }
    }
    let target = held.map(|h| h.index).or(cursor_index);
    let can_equip = target
        .and_then(|i| pack.placed.get(i))
        .is_some_and(|p| backpack::info(&p.id).slot.is_some());
    let can_split = target
        .and_then(|i| pack.placed.get(i))
        .is_some_and(|p| p.qty > 1);
    let turnable = target
        .and_then(|i| pack.placed.get(i))
        .is_some_and(|p| backpack::info(&p.id).w != backpack::info(&p.id).h);
    let can_use = target
        .and_then(|i| pack.placed.get(i))
        .is_some_and(|p| backpack::cure(&p.id).is_some())
        && v.vitality.is_some_and(|(now, most)| now < most);
    ui::pill(
        c,
        v.fonts,
        BTN_SORT,
        t("Ordenar", "Sort"),
        rect_contains(BTN_SORT, v.mouse.0, v.mouse.1),
        true,
        v.light,
        false,
    );
    ui::pill(
        c,
        v.fonts,
        BTN_ROTATE,
        t("Girar", "Rotate"),
        rect_contains(BTN_ROTATE, v.mouse.0, v.mouse.1),
        turnable,
        v.light,
        false,
    );
    ui::pill(
        c,
        v.fonts,
        BTN_EQUIP,
        t("Equipar", "Equip"),
        rect_contains(BTN_EQUIP, v.mouse.0, v.mouse.1),
        can_equip,
        v.light,
        false,
    );
    ui::pill(
        c,
        v.fonts,
        BTN_SPLIT,
        t("Dividir", "Split"),
        rect_contains(BTN_SPLIT, v.mouse.0, v.mouse.1),
        can_split,
        v.light,
        false,
    );
    ui::pill(
        c,
        v.fonts,
        BTN_USE,
        t("Usar", "Use"),
        rect_contains(BTN_USE, v.mouse.0, v.mouse.1),
        can_use,
        v.light,
        false,
    );
    let hints_w = ui::key_hints(
        c,
        v.fonts,
        512,
        904,
        &[
            ("O", t("ordenar", "sort")),
            ("R", t("girar", "rotate")),
            ("E", t("equipar", "equip")),
            ("V", t("dividir", "split")),
            ("U", t("usar", "use")),
        ],
    );
    // Her vitality where cures count: a bar at the right of the hints' row, or above the
    // buttons when a language's hints reach that far.
    if let Some((now, most)) = v.vitality {
        let (bx, by, bw) = if 512 + hints_w + 16 <= 1090 {
            (1090, 918, 158)
        } else {
            (1090, 812, 158)
        };
        let k = (now / most.max(1.0)).clamp(0.0, 1.0);
        let label = format!(
            "{} {:.0}/{:.0}",
            demon_queen_dungeon::i18n::tr(t("Vitalidad", "Vitality")),
            now,
            most
        );
        ui::text_mid(c, v.fonts, &label, bx, by - 12, 15.0, ui::INK, true);
        c.round_rect(bx, by, bw, 12, 6, 0xc0140d1c, 0xc0140d1c);
        let fill = if k < 0.35 { 0xffe0455a } else { 0xff5fd08a };
        if k > 0.0 {
            c.round_rect(bx, by, ((bw as f32) * k).max(12.0) as i32, 12, 6, fill, fill);
        }
    }
    if let Some(note) = v.ui.note() {
        ui::text_mid(
            c,
            v.fonts,
            &ellipsize(v.fonts, note, 720, 19.0, true),
            512,
            950,
            19.0,
            ui::mix(v.light, 0xffffff, 0.5),
            true,
        );
    } else if held.is_some() {
        ui::text_mid(
            c,
            v.fonts,
            t(
                "Clic para soltar · sobre otro objeto lo combina · Esc devuelve",
                "Click to drop · on another item combines · Esc puts it back",
            ),
            512,
            950,
            18.0,
            ui::MUTED,
            false,
        );
    } else {
        ui::text_mid(
            c,
            v.fonts,
            t(
                "Clic para tomar un objeto y otro clic para soltarlo",
                "Click to pick an item up and click again to drop it",
            ),
            512,
            950,
            18.0,
            ui::MUTED,
            false,
        );
    }

    paint_details(
        c,
        v,
        pack,
        held.map(|h| h.index).or(hover_index).or(cursor_index),
    );

    // The stack in hand follows the pointer.
    if let (Some((index, w, h, _)), true) = (ghost, v.mouse.0 >= 0) {
        if let Some(p) = pack.placed.get(index) {
            let (tw, th) = (w as i32 * CELL - 6, h as i32 * CELL - 6);
            let rect = (v.mouse.0 - tw / 2, v.mouse.1 - th / 2, tw, th);
            c.round_rect(
                rect.0 + 6,
                rect.1 + 12,
                rect.2,
                rect.3,
                14,
                0x40000000,
                0x40000000,
            );
            paint_tile(c, v, &p.id, rect, p.qty, true, true);
        }
    }
}

fn paint_details(c: &mut Canvas, v: &View, pack: &Backpack, index: Option<usize>) {
    let t = |es, en| tr(v.language, es, en);
    let (x, w) = (1340, 434);
    let Some(p) = index.and_then(|i| pack.placed.get(i)) else {
        ui::icon_disc(
            c,
            Icon::Backpack,
            1557,
            380,
            52,
            ui::mix(v.light, 0xffffff, 0.1),
            false,
        );
        ui::text_center(
            c,
            v.fonts,
            t("Mochila de la Reina", "The Queen's backpack"),
            1557,
            480,
            28.0,
            ui::INK,
            true,
        );
        c.paragraph(
            v.fonts,
            t(
                "Los objetos tienen forma y ocupan espacio. Gira, junta y combina para llevar más.",
                "Items have a shape and take room. Rotate, stack and combine to carry more.",
            ),
            x,
            530,
            w,
            32,
            21.0,
            ui::MUTED,
            false,
        );
        ui::key_hints(
            c,
            v.fonts,
            x,
            700,
            &[
                ("Clic", t("tomar / soltar", "pick / drop")),
                ("Clic der.", t("girar", "rotate")),
            ],
        );
        return;
    };
    let tint = tint_of(v.items, &p.id);
    let it = item(v.items, &p.id);
    // Icon plate.
    c.round_rect(
        x,
        260,
        150,
        150,
        20,
        ui::argb(0x55, ui::mix(0x3a2c46, tint, 0.4)),
        ui::argb(0x33, 0x3a2c46),
    );
    c.round_outline(x, 260, 150, 150, 20, 1.2, ui::argb(0x99, tint));
    if let Some(it) = it {
        crate::pause_menu::paint_item_icon(
            c,
            v.fonts,
            it,
            (x + 10, 270, 130, 130),
            0xff3a2c46,
            70.0,
        );
    }
    let name = name_of(v.items, &p.id);
    c.display_text(
        v.fonts,
        &ellipsize(v.fonts, &name, 262, 26.0, false),
        x + 172,
        274,
        26.0,
        ui::INK,
    );
    let info = backpack::info(&p.id);
    let chips = [
        format!("{}×{}", info.w, info.h),
        format!(
            "{} {}{}",
            t("Cant.", "Qty"),
            p.qty,
            if info.stack > 1 {
                format!(" / {}", info.stack)
            } else {
                String::new()
            }
        ),
    ];
    let mut cx = x + 172;
    for (k, label) in chips.iter().enumerate() {
        let cw = v.fonts.text_width(label, 16.0, true) + 24;
        let col = if k == 0 {
            ui::mix(tint, 0xffffff, 0.2)
        } else {
            ui::GOLD
        } & 0x00ff_ffff;
        c.round_rect(
            cx,
            330,
            cw,
            30,
            15,
            ui::argb(0x33, col),
            ui::argb(0x33, col),
        );
        c.round_outline(cx, 330, cw, 30, 15, 1.0, ui::argb(0x88, col));
        ui::text_center(
            c,
            v.fonts,
            label,
            cx + cw / 2,
            345,
            16.0,
            ui::mix(col, 0xffffff, 0.35),
            true,
        );
        cx += cw + 8;
    }
    if let Some(slot) = info.slot {
        let label = format!(
            "{} · {}",
            t("Equipable", "Equippable"),
            slot.label(v.language != Language::English)
        );
        ui::text_mid(
            c,
            v.fonts,
            &label,
            x + 172,
            384,
            17.0,
            ui::mix(0x86b8f2, 0xffffff, 0.2),
            true,
        );
    }
    let description = it.map(|i| i.description.as_str()).unwrap_or("");
    let bottom = c.paragraph(v.fonts, description, x, 434, w, 30, 21.0, ui::MUTED, false);
    if let Some(effect) = it
        .map(|i| i.effect.as_str())
        .filter(|e| !e.is_empty() && *e != description)
    {
        c.paragraph(
            v.fonts,
            effect,
            x,
            bottom + 6,
            w,
            28,
            19.0,
            ui::mix(ui::INK, ui::MUTED, 0.3),
            false,
        );
    }
    // Combinations this item takes part in.
    let recipes = backpack::recipes_with(&p.id);
    if !recipes.is_empty() {
        let top = 700;
        c.rect(x, top - 14, w, 1, 0x30ffffff);
        ui::text_mid(
            c,
            v.fonts,
            t("SE COMBINA CON", "COMBINES WITH"),
            x,
            top,
            14.0,
            ui::GOLD,
            true,
        );
        for (k, r) in recipes.iter().take(3).enumerate() {
            let other = if r.a == p.id { r.b } else { r.a };
            let owned = pack.count(other) >= if other == p.id { 2 } else { 1 };
            let line = format!(
                "{} → {}",
                name_of(v.items, other),
                name_of(v.items, r.result)
            );
            let color = if owned {
                ui::INK
            } else {
                ui::argb(0xa0, ui::MUTED)
            };
            ui::text_mid(
                c,
                v.fonts,
                &ellipsize(v.fonts, &line, w - 30, 17.0, false),
                x + 22,
                top + 30 + k as i32 * 30,
                17.0,
                color,
                false,
            );
            c.smooth_disc(
                (x + 7) as f32,
                (top + 30 + k as i32 * 30) as f32,
                4.0,
                if owned {
                    ui::argb(0xff, ui::GOLD)
                } else {
                    0x55ffffff
                },
            );
        }
        if !v.craft {
            ui::text_mid(
                c,
                v.fonts,
                t("Solo fuera de la cueva", "Only outside the cave"),
                x,
                top + 30 + 3 * 30,
                15.0,
                ui::argb(0xcc, ui::DANGER),
                false,
            );
        }
    }
}

/// The equipment page: worn slots on the left, what could go on them on the right.
pub fn paint_equipment(c: &mut Canvas, v: &View) {
    let t = |es, en| tr(v.language, es, en);
    let spanish = v.language != Language::English;
    ui::glass(c, PANE_L, 22, v.light, 0.0);
    ui::glass(c, PANE_R, 22, v.light, 0.0);
    let Some(pack) = v.pack else {
        ui::icon_disc(
            c,
            Icon::Wardrobe,
            880,
            520,
            56,
            ui::mix(v.light, 0xffffff, 0.1),
            false,
        );
        ui::text_center(
            c,
            v.fonts,
            t("Equipo", "Equipment"),
            880,
            620,
            30.0,
            ui::INK,
            true,
        );
        c.paragraph(
            v.fonts,
            t(
                "Podrás equipar objetos al entrar en la exploración 3D.",
                "You can equip items once you enter 3D exploration.",
            ),
            560,
            668,
            640,
            34,
            23.0,
            ui::MUTED,
            false,
        );
        return;
    };
    ui::text_mid(
        c,
        v.fonts,
        t("RANURAS DE EQUIPO", "EQUIPMENT SLOTS"),
        512,
        254,
        18.0,
        ui::GOLD,
        true,
    );
    let selected = v.ui.slot.min(Slot::ALL.len() - 1);
    let icons = [Icon::Mine, Icon::Memory, Icon::Rune];
    for (i, slot) in Slot::ALL.iter().enumerate() {
        let rect = slot_card(i);
        let (x, y, w, h) = rect;
        let hot = selected == i || rect_contains(rect, v.mouse.0, v.mouse.1);
        let worn = pack.equipment.get(*slot);
        let tint = worn.map(|id| tint_of(v.items, id)).unwrap_or(0x8f83a0);
        c.round_rect(x, y + 4, w, h, 20, 0x22000000, 0x34000000);
        let (top, bottom) = if hot {
            (
                ui::argb(0xf2, ui::mix(0x2c1f3a, v.light, 0.26)),
                ui::argb(0xf2, ui::mix(0x181023, v.light, 0.12)),
            )
        } else {
            (
                ui::argb(0xe6, ui::mix(0x22182e, v.light, 0.08)),
                ui::argb(0xe6, ui::mix(0x150d1e, v.light, 0.04)),
            )
        };
        c.round_rect(x, y, w, h, 20, top, bottom);
        c.round_outline(
            x,
            y,
            w,
            h,
            20,
            if hot { 2.0 } else { 1.0 },
            if hot {
                ui::argb(0xf0, ui::mix(v.light, 0xffffff, 0.3))
            } else {
                0x30ffffff
            },
        );
        // The item's plate.
        let plate = (x + 20, y + 17, 96, 96);
        c.round_rect(
            plate.0,
            plate.1,
            plate.2,
            plate.3,
            16,
            ui::argb(
                if worn.is_some() { 0x66 } else { 0x33 },
                ui::mix(0x3a2c46, tint, 0.3),
            ),
            ui::argb(0x22, 0x3a2c46),
        );
        c.round_outline(
            plate.0,
            plate.1,
            plate.2,
            plate.3,
            16,
            1.2,
            ui::argb(if worn.is_some() { 0xaa } else { 0x44 }, tint),
        );
        match worn {
            Some(id) => {
                if let Some(it) = item(v.items, id) {
                    crate::pause_menu::paint_item_icon(
                        c,
                        v.fonts,
                        it,
                        (plate.0 + 8, plate.1 + 8, 80, 80),
                        0xff3a2c46,
                        46.0,
                    );
                }
            }
            None => ui::icon(
                c,
                icons[i],
                plate.0 + 48,
                plate.1 + 48,
                46,
                0x44ffffff,
                0x00000000,
            ),
        }
        ui::text_mid(
            c,
            v.fonts,
            &slot.label(spanish).to_uppercase(),
            x + 140,
            y + 30,
            15.0,
            ui::GOLD,
            true,
        );
        match worn {
            Some(id) => {
                ui::text_mid(
                    c,
                    v.fonts,
                    &ellipsize(v.fonts, &name_of(v.items, id), 400, 27.0, true),
                    x + 140,
                    y + 62,
                    27.0,
                    ui::INK,
                    true,
                );
                let line = match slot {
                    Slot::Weapon => format!(
                        "{} ×{:.2}",
                        t("Daño de espada", "Sword damage"),
                        backpack::weapon_damage(id)
                    ),
                    _ => t("Efecto aún desconocido", "Effect still unknown").to_string(),
                };
                ui::text_mid(c, v.fonts, &line, x + 140, y + 98, 18.0, ui::MUTED, false);
                let r = slot_remove(i);
                ui::pill(
                    c,
                    v.fonts,
                    r,
                    t("Quitar", "Remove"),
                    rect_contains(r, v.mouse.0, v.mouse.1),
                    true,
                    v.light,
                    false,
                );
            }
            None => {
                ui::text_mid(
                    c,
                    v.fonts,
                    t("Vacía", "Empty"),
                    x + 140,
                    y + 62,
                    27.0,
                    ui::argb(0xa0, ui::MUTED),
                    false,
                );
                ui::text_mid(
                    c,
                    v.fonts,
                    t("Elige un objeto de la lista", "Pick an item from the list"),
                    x + 140,
                    y + 98,
                    18.0,
                    ui::MUTED,
                    false,
                );
            }
        }
    }
    // Weapon upgrade sockets.
    let top = 716;
    c.rect(512, top - 12, 736, 1, 0x30ffffff);
    ui::text_mid(
        c,
        v.fonts,
        t("MEJORAS DEL ARMA", "WEAPON UPGRADES"),
        512,
        top + 6,
        15.0,
        ui::GOLD,
        true,
    );
    for k in 0..backpack::WEAPON_SOCKETS {
        let (x, y) = (512 + k as i32 * 92, top + 28);
        let filled = pack.equipment.weapon_mods.get(k).and_then(|m| m.as_deref());
        c.round_rect(x, y, 80, 80, 14, 0x66140d1c, 0x66100a17);
        c.round_outline(
            x,
            y,
            80,
            80,
            14,
            1.2,
            if filled.is_some() {
                ui::argb(0xcc, ui::GOLD)
            } else {
                0x38ffffff
            },
        );
        match filled {
            Some(id) => {
                if let Some(it) = item(v.items, id) {
                    crate::pause_menu::paint_item_icon(
                        c,
                        v.fonts,
                        it,
                        (x + 8, y + 8, 64, 64),
                        0xff2a1f36,
                        36.0,
                    );
                }
            }
            None => c.smooth_disc((x + 40) as f32, (y + 40) as f32, 4.0, 0x33ffffff),
        }
    }
    c.paragraph(
        v.fonts,
        t(
            "Filo y empuñadura admitirán mejoras: aquí se colocarán cuando existan.",
            "Edge and pommel will take upgrades: they will go here once they exist.",
        ),
        724,
        top + 32,
        520,
        28,
        18.0,
        ui::MUTED,
        false,
    );

    // Right: what can be worn in the selected slot.
    let slot = Slot::ALL[selected];
    ui::text_mid(
        c,
        v.fonts,
        &format!(
            "{} · {}",
            t("EN LA MOCHILA", "IN THE BACKPACK"),
            slot.label(spanish).to_uppercase()
        ),
        1336,
        254,
        16.0,
        ui::GOLD,
        true,
    );
    let mut candidates: Vec<&str> = Vec::new();
    for p in &pack.placed {
        if backpack::info(&p.id).slot == Some(slot) && !candidates.contains(&p.id.as_str()) {
            candidates.push(&p.id);
        }
    }
    if candidates.is_empty() {
        c.paragraph(
            v.fonts,
            t(
                "No llevas ningún objeto que vaya en esta ranura.",
                "You carry nothing that fits this slot.",
            ),
            1336,
            300,
            440,
            32,
            22.0,
            ui::MUTED,
            false,
        );
    }
    let worn_now = pack.equipment.get(slot);
    for (k, id) in candidates.iter().take(MAX_CANDIDATES).enumerate() {
        let rect = candidate_row(k);
        let (x, y, w, h) = rect;
        let focus = v.focused && v.focus_index == Slot::ALL.len() + k;
        let hot = focus || rect_contains(rect, v.mouse.0, v.mouse.1);
        let tint = tint_of(v.items, id);
        c.round_rect(x, y + 3, w, h, 16, 0x22000000, 0x30000000);
        c.round_rect(
            x,
            y,
            w,
            h,
            16,
            if hot {
                ui::argb(0xf2, ui::mix(0x2c1f3a, v.light, 0.28))
            } else {
                0x9a1a1224
            },
            if hot {
                ui::argb(0xf2, ui::mix(0x181023, v.light, 0.14))
            } else {
                0x9a140d1c
            },
        );
        c.round_outline(
            x,
            y,
            w,
            h,
            16,
            if hot { 1.8 } else { 1.0 },
            if hot {
                ui::argb(0xe6, ui::mix(v.light, 0xffffff, 0.2))
            } else {
                0x30ffffff
            },
        );
        c.round_rect(
            x + 10,
            y + 8,
            56,
            56,
            12,
            ui::argb(0x55, ui::mix(0x3a2c46, tint, 0.35)),
            ui::argb(0x22, 0x3a2c46),
        );
        if let Some(it) = item(v.items, id) {
            crate::pause_menu::paint_item_icon(
                c,
                v.fonts,
                it,
                (x + 14, y + 12, 48, 48),
                0xff3a2c46,
                28.0,
            );
        }
        ui::text_mid(
            c,
            v.fonts,
            &ellipsize(v.fonts, &name_of(v.items, id), 250, 20.0, true),
            x + 80,
            y + 24,
            20.0,
            ui::INK,
            true,
        );
        let detail = match slot {
            Slot::Weapon => {
                let (now, then) = (
                    worn_now.map(backpack::weapon_damage).unwrap_or(1.0),
                    backpack::weapon_damage(id),
                );
                format!(
                    "{} ×{:.2}  ({:+.0}%)",
                    t("Daño", "Damage"),
                    then,
                    (then / now - 1.0) * 100.0
                )
            }
            _ => t("Efecto aún desconocido", "Effect still unknown").to_string(),
        };
        ui::text_mid(c, v.fonts, &detail, x + 80, y + 52, 16.0, ui::MUTED, false);
        let pill = (x + w - 122, y + 14, 106, 44);
        ui::pill(
            c,
            v.fonts,
            pill,
            t("Equipar", "Equip"),
            hot,
            true,
            v.light,
            false,
        );
    }
    // Summary of what the equipment does.
    let damage = pack
        .equipment
        .weapon
        .as_deref()
        .map(backpack::weapon_damage)
        .unwrap_or(1.0);
    c.round_rect(
        1328,
        896,
        458,
        56,
        16,
        ui::argb(0x33, v.light),
        ui::argb(0x18, v.light),
    );
    c.round_outline(1328, 896, 458, 56, 16, 1.0, ui::argb(0x66, v.light));
    ui::text_mid(
        c,
        v.fonts,
        t("Daño de espada", "Sword damage"),
        1348,
        924,
        19.0,
        ui::MUTED,
        false,
    );
    ui::text_right(
        c,
        v.fonts,
        &format!("×{damage:.2}"),
        1766,
        924,
        26.0,
        ui::mix(ui::GOLD, 0xffffff, 0.2),
        true,
    );
    if let Some(note) = v.ui.note() {
        ui::text_mid(
            c,
            v.fonts,
            &ellipsize(v.fonts, note, 720, 19.0, true),
            512,
            950,
            19.0,
            ui::mix(v.light, 0xffffff, 0.5),
            true,
        );
    } else {
        ui::text_mid(
            c,
            v.fonts,
            t(
                "Enter sobre una ranura la vacía; sobre un objeto lo equipa.",
                "Enter on a slot empties it; on an item equips it.",
            ),
            512,
            950,
            18.0,
            ui::MUTED,
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::KeyCode;

    fn pack(list: &[(&str, u32)]) -> Backpack {
        let mut p = Backpack::new();
        p.reconcile(&list.iter().map(|(k, v)| (k.to_string(), *v)).collect());
        p
    }

    #[test]
    fn grid_cells_map_to_pixels_and_back() {
        let p = Backpack::new();
        let (gx, gy) = grid_origin(&p);
        for (cx, cy) in [(0u8, 0u8), (3, 2), (7, 4)] {
            let rect = stack_rect(&p, cx as i32, cy as i32, 1, 1);
            assert_eq!(
                cell_at(&p, rect.0 + rect.2 / 2, rect.1 + rect.3 / 2),
                Some((cx, cy))
            );
        }
        assert_eq!(cell_at(&p, gx - 1, gy), None);
        assert_eq!(cell_at(&p, gx + p.cols as i32 * CELL, gy), None);
        assert_eq!(cell_at(&p, gx, gy + p.rows as i32 * CELL), None);
    }

    #[test]
    fn the_grid_and_buttons_fit_inside_the_left_pane() {
        let p = Backpack::new();
        let (gx, gy) = grid_origin(&p);
        assert!(gx >= PANE_L.0 && gx + p.cols as i32 * CELL <= PANE_L.0 + PANE_L.2);
        assert!(gy + p.rows as i32 * CELL < BTN_SORT.1);
        for b in [BTN_SORT, BTN_ROTATE, BTN_EQUIP, BTN_SPLIT, BTN_USE] {
            assert!(
                b.0 >= PANE_L.0
                    && b.0 + b.2 <= PANE_L.0 + PANE_L.2
                    && b.1 + b.3 < PANE_L.1 + PANE_L.3
            );
        }
        for i in 0..Slot::ALL.len() {
            let c = slot_card(i);
            assert!(c.0 + c.2 <= PANE_L.0 + PANE_L.2 && c.1 + c.3 < 716);
        }
        let last = candidate_row(MAX_CANDIDATES - 1);
        assert!(last.1 + last.3 < 896 && last.0 + last.2 <= PANE_R.0 + PANE_R.2);
    }

    #[test]
    fn dropping_on_a_free_spot_moves_and_snaps_inside_the_grid() {
        let mut p = pack(&[("espada_de_madera", 1)]);
        let held = Held {
            index: 0,
            rot: false,
        };
        // Pointing at the far bottom-right: a 1x4 sword clamps to fit.
        let plan = plan_drop(&p, held, (7, 4), true);
        assert_eq!(plan, Plan::Place { x: 7, y: 1 });
        assert!(matches!(apply_plan(&mut p, held, plan), Outcome::Moved));
        assert_eq!(p.placed[0].x, 7);
        assert!(p.is_valid());
    }

    #[test]
    fn dropping_on_the_same_item_stacks_and_on_a_recipe_combines() {
        let mut p = pack(&[("hierbas_curativas", 8), ("pocion_pequena", 1)]);
        // Two herb stacks (6 + 2) and a potion.
        let herbs: Vec<usize> = (0..p.placed.len())
            .filter(|i| p.placed[*i].id == "hierbas_curativas")
            .collect();
        let potion = p
            .placed
            .iter()
            .position(|s| s.id == "pocion_pequena")
            .unwrap();
        let big = *herbs.iter().find(|i| p.placed[**i].qty == 6).unwrap();
        let small = *herbs.iter().find(|i| p.placed[**i].qty == 2).unwrap();
        let target = (p.placed[big].x as i32, p.placed[big].y as i32);
        // Small onto the full stack: cannot merge, so it is blocked (cells taken).
        assert!(!matches!(
            plan_drop(
                &p,
                Held {
                    index: small,
                    rot: false
                },
                target,
                true
            ),
            Plan::Merge(_)
        ));
        // Herbs onto the potion combine when crafting is allowed and lock otherwise.
        let cell = (p.placed[potion].x as i32, p.placed[potion].y as i32);
        assert_eq!(
            plan_drop(
                &p,
                Held {
                    index: small,
                    rot: false
                },
                cell,
                true
            ),
            Plan::Combine(potion, "pocion_reforzada".into())
        );
        assert_eq!(
            plan_drop(
                &p,
                Held {
                    index: small,
                    rot: false
                },
                cell,
                false
            ),
            Plan::Locked(potion)
        );
        let held = Held {
            index: small,
            rot: false,
        };
        let plan = plan_drop(&p, held, cell, true);
        assert!(
            matches!(apply_plan(&mut p, held, plan), Outcome::Combined(id) if id == "pocion_reforzada")
        );
        assert!(p.is_valid());
        assert_eq!(p.count("hierbas_curativas"), 7);
    }

    #[test]
    fn a_turned_item_is_planned_with_its_turned_shape() {
        let mut p = pack(&[("espada_reliquia", 1)]);
        let plan = plan_drop(
            &p,
            Held {
                index: 0,
                rot: true,
            },
            (0, 0),
            true,
        );
        assert_eq!(plan, Plan::Place { x: 0, y: 0 });
        assert!(matches!(
            apply_plan(
                &mut p,
                Held {
                    index: 0,
                    rot: true
                },
                plan
            ),
            Outcome::Moved
        ));
        assert_eq!(p.placed[0].size(), (5, 1));
        // A 5x1 sword turned into the last column cannot fit any more.
        assert_eq!(
            plan_drop(
                &p,
                Held {
                    index: 0,
                    rot: false
                },
                (0, 0),
                true
            ),
            Plan::Place { x: 0, y: 0 }
        );
    }

    #[test]
    fn a_blocked_drop_reports_blocked_and_changes_nothing() {
        let mut p = Backpack::new();
        p.placed.push(backpack::Placed {
            id: "espada_reliquia".into(),
            x: 0,
            y: 0,
            rot: false,
            qty: 1,
        });
        // (Bones: slime would combine with the stone sword.)
        for x in 1..8u8 {
            for y in 0..5u8 {
                p.placed.push(backpack::Placed {
                    id: "hueso_viejo".into(),
                    x,
                    y,
                    rot: false,
                    qty: 1,
                });
            }
        }
        let before = p.clone();
        let held = Held {
            index: 0,
            rot: true,
        };
        let plan = plan_drop(&p, held, (3, 3), true);
        assert!(matches!(plan, Plan::Merge(_) | Plan::Blocked), "{plan:?}");
        if plan == Plan::Blocked {
            assert!(matches!(
                apply_plan(&mut p, held, plan),
                Outcome::Refused(Refuse::Blocked)
            ));
            assert_eq!(p, before);
        }
    }

    // ---- the pause menu driven end to end

    fn menu_with_pack(page: crate::pause_menu::Page, craft: bool) -> App {
        let mut app = App::default();
        app.start_game().unwrap();
        let (gold, items, mut pack) = crate::exploration3d::App::sample_menu_inventory();
        // Start from a tidy, known layout with nothing worn.
        pack.unequip(Slot::Weapon).unwrap();
        pack.arrange();
        app.pause_inventory_gold = gold;
        app.pause_inventory_items = items;
        app.pause_backpack = Some(pack);
        app.pause_backpack_craft = craft;
        app.open_pause(page);
        app
    }

    fn center(app: &App, cell: (u8, u8)) -> (i32, i32) {
        let pack = app.pause_backpack.as_ref().unwrap();
        let (gx, gy) = grid_origin(pack);
        (
            gx + cell.0 as i32 * CELL + CELL / 2,
            gy + cell.1 as i32 * CELL + CELL / 2,
        )
    }

    fn first_empty(app: &App) -> (u8, u8) {
        let pack = app.pause_backpack.as_ref().unwrap();
        for y in 0..pack.rows {
            for x in 0..pack.cols {
                if pack.at(x, y).is_none() {
                    return (x, y);
                }
            }
        }
        panic!("the sample pack has no free cell");
    }

    #[test]
    fn clicking_picks_an_item_up_and_a_second_click_drops_it_and_marks_the_pack_dirty() {
        let mut app = menu_with_pack(crate::pause_menu::Page::Inventory, true);
        let ring = app
            .pause_backpack
            .as_ref()
            .unwrap()
            .placed
            .iter()
            .position(|p| p.id == "anillo_misterioso")
            .unwrap();
        let at = {
            let p = &app.pause_backpack.as_ref().unwrap().placed[ring];
            (p.x, p.y)
        };
        let (x, y) = center(&app, at);
        app.menu_click(x, y);
        assert_eq!(
            app.pause.as_ref().unwrap().bp.held.map(|h| h.index),
            Some(ring)
        );
        assert!(!app.backpack_dirty, "picking up changes nothing yet");
        let empty = first_empty(&app);
        let (x, y) = center(&app, empty);
        app.menu_click(x, y);
        assert!(app.pause.as_ref().unwrap().bp.held.is_none());
        assert!(app.backpack_dirty);
        let pack = app.pause_backpack.as_ref().unwrap();
        assert!(pack.is_valid());
        assert_eq!(pack.placed[ring].id, "anillo_misterioso");
        assert_eq!((pack.placed[ring].x, pack.placed[ring].y), empty);
    }

    #[test]
    fn escape_returns_the_held_item_first_and_only_then_closes_the_menu() {
        let mut app = menu_with_pack(crate::pause_menu::Page::Inventory, true);
        let (x, y) = center(&app, (0, 0));
        app.menu_click(x, y);
        assert!(app.pause.as_ref().unwrap().bp.held.is_some());
        app.menu_key(KeyCode::Escape);
        assert!(app.pause.as_ref().unwrap().bp.held.is_none());
        assert_eq!(
            app.screen,
            crate::Screen::GameMenu,
            "the first Esc only puts the item back"
        );
        assert!(!app.backpack_dirty);
        app.menu_key(KeyCode::Escape);
        assert_ne!(app.screen, crate::Screen::GameMenu);
    }

    #[test]
    fn the_keyboard_can_pick_turn_and_drop() {
        let mut app = menu_with_pack(crate::pause_menu::Page::Inventory, true);
        let empty = first_empty(&app);
        {
            let menu = app.pause.as_mut().unwrap();
            menu.bp.cursor = (0, 0);
            menu.focus = crate::pause_menu::NAV_COUNT;
        }
        assert!(app.pause_backpack.as_ref().unwrap().at(0, 0).is_some());
        app.menu_key(KeyCode::Enter);
        assert!(app.pause.as_ref().unwrap().bp.held.is_some());
        let before = app.pause.as_ref().unwrap().bp.held.unwrap().rot;
        app.menu_key(KeyCode::KeyR);
        assert_ne!(app.pause.as_ref().unwrap().bp.held.unwrap().rot, before);
        app.pause.as_mut().unwrap().bp.cursor = empty;
        app.menu_key(KeyCode::Enter);
        // It either fit (dropped) or a note says why; the pack is valid either way.
        assert!(app.pause_backpack.as_ref().unwrap().is_valid());
    }

    #[test]
    fn dropping_an_herb_on_a_potion_combines_them_only_outside_the_cave() {
        for (craft, combined) in [(true, true), (false, false)] {
            let mut app = menu_with_pack(crate::pause_menu::Page::Inventory, craft);
            let (herb, potion) = {
                let p = app.pause_backpack.as_ref().unwrap();
                (
                    p.placed
                        .iter()
                        .position(|s| s.id == "hierbas_curativas")
                        .unwrap(),
                    p.placed
                        .iter()
                        .position(|s| s.id == "pocion_pequena")
                        .unwrap(),
                )
            };
            let (hx, hy) = {
                let s = &app.pause_backpack.as_ref().unwrap().placed[herb];
                center(&app, (s.x, s.y))
            };
            let (px, py) = {
                let s = &app.pause_backpack.as_ref().unwrap().placed[potion];
                center(&app, (s.x, s.y))
            };
            app.menu_click(hx, hy);
            app.menu_click(px, py);
            let pack = app.pause_backpack.as_ref().unwrap();
            assert_eq!(pack.count("pocion_reforzada"), combined as u32);
            assert_eq!(pack.count("pocion_pequena"), if combined { 2 } else { 3 });
            assert_eq!(app.backpack_dirty, combined);
            assert!(pack.is_valid());
        }
    }

    #[test]
    fn the_equip_key_wears_a_sword_and_the_equipment_page_takes_it_off() {
        let mut app = menu_with_pack(crate::pause_menu::Page::Inventory, true);
        let sword = app
            .pause_backpack
            .as_ref()
            .unwrap()
            .placed
            .iter()
            .position(|p| p.id == "espada_de_madera")
            .unwrap();
        let cell = {
            let p = &app.pause_backpack.as_ref().unwrap().placed[sword];
            (p.x, p.y)
        };
        {
            let menu = app.pause.as_mut().unwrap();
            menu.bp.cursor = cell;
            menu.focus = crate::pause_menu::NAV_COUNT;
        }
        app.menu_key(KeyCode::KeyE);
        assert_eq!(
            app.pause_backpack
                .as_ref()
                .unwrap()
                .equipment
                .weapon
                .as_deref(),
            Some("espada_de_madera")
        );
        assert!(app.backpack_dirty);
        app.set_menu_page(crate::pause_menu::Page::Equipment);
        // Slot 0 is the weapon: Enter on it takes the sword off again.
        app.menu_key(KeyCode::Enter);
        assert_eq!(app.pause_backpack.as_ref().unwrap().equipment.weapon, None);
        assert!(
            app.pause_backpack
                .as_ref()
                .unwrap()
                .count("espada_de_madera")
                == 1
        );
        // A candidate row equips one of the two swords.
        assert_eq!(app.equipment_candidates(Slot::Weapon).len(), 2);
        app.equipment_control(Slot::ALL.len() + 1);
        assert!(app
            .pause_backpack
            .as_ref()
            .unwrap()
            .equipment
            .weapon
            .is_some());
        assert!(app.pause_backpack.as_ref().unwrap().is_valid());
    }

    #[test]
    fn every_backpack_item_with_an_icon_file_draws_it_from_vector_art() {
        let (_, items, _) = crate::exploration3d::App::sample_menu_inventory();
        let mut checked = 0;
        for item in items.iter().filter(|i| i.icon_svg.is_some()) {
            let file = item.icon_svg.as_deref().unwrap();
            let bitmap = crate::pause_menu::shop_icon(file)
                .unwrap_or_else(|| panic!("{} ({file}) did not load", item.id));
            assert_eq!((bitmap.w, bitmap.h), (256, 256));
            assert!(
                bitmap.rgba.chunks_exact(4).filter(|p| p[3] > 200).count() > 256 * 256 / 10,
                "{file} is mostly empty"
            );
            checked += 1;
        }
        assert!(
            checked >= 22,
            "the catalogue, the relic sword and the crafted items all have icons ({checked})"
        );
    }

    #[test]
    fn both_pages_paint_with_an_item_in_hand_at_any_pointer_position() {
        for page in [
            crate::pause_menu::Page::Inventory,
            crate::pause_menu::Page::Equipment,
        ] {
            let mut app = menu_with_pack(page, true);
            let (x, y) = center(&app, (0, 0));
            app.menu_click(x, y);
            for mouse in [(-1, -1), (x, y), (1279, 700), (1900, 1070), (600, 300)] {
                app.mouse = mouse;
                app.paint_game_menu();
                assert_eq!(
                    app.canvas.pixels.len(),
                    (crate::WIN_W * crate::WIN_H) as usize
                );
            }
        }
    }
}
