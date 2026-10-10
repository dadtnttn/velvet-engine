//! Non-explicit salon menu: companion requests, client hospitality jobs, and room rentals.
//! Economy is stored in the exploration checkpoint; UI selection is transient.
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

fn default_libido() -> u32 {
    60
}
fn default_energy() -> u32 {
    80
}
fn default_mood() -> u32 {
    70
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash)]
#[serde(default)]
pub struct Progress {
    pub room: Option<String>,
    pub room_until: u32,
    pub hours: u32,
    pub shifts: Vec<String>,
    #[serde(default = "default_libido")]
    pub libido: u32,
    #[serde(default = "default_energy")]
    pub energy: u32,
    #[serde(default = "default_mood")]
    pub mood: u32,
    /// Girls paid for so far (Rosalía makes her offer after the first).
    pub visits: u32,
    /// Rosalía has offered her elixir (from then on it is in the book).
    pub elixir_offered: bool,
    /// Elixirs bought (the first at the welcome price).
    pub elixirs_bought: u32,
    /// One is drunk and not yet spent: the seal can take its shape with a girl.
    pub elixir_active: bool,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            room: None,
            room_until: 0,
            hours: 0,
            shifts: Vec::new(),
            libido: 60,
            energy: 80,
            mood: 70,
            visits: 0,
            elixir_offered: false,
            elixirs_bought: 0,
            elixir_active: false,
        }
    }
}

/// Rosalía's elixir: the price of the first (her welcome) and of the rest.
pub const ELIXIR_WELCOME: u32 = 10;
pub const ELIXIR_PRICE: u32 = 40;

/// The book's pages: the girls side by side, what to ask of the one chosen, Rosalía's offer and
/// the elixir's leaflet (its side effects, read from the offer).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Page {
    #[default]
    Girls,
    Ask,
    Offer,
    Leaflet,
}

/// The offer's options, in the order the keys go through them.
pub const OFFER_BUY: usize = 0;
pub const OFFER_DECLINE: usize = 1;
pub const OFFER_LEAFLET: usize = 2;

/// One thing to ask of a girl, as the book shows it now.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Asking {
    pub label: String,
    pub hint: String,
    pub icon: String,
    /// It wants the elixir and none is drunk: picking it opens Rosalía's offer.
    pub locked: bool,
    /// It wants the elixir and one is drunk (the seal glows on it).
    pub awake: bool,
    /// «Volver».
    pub back: bool,
}

#[derive(Default, Hash)]
pub struct Menu {
    pub open: bool,
    pub tab: usize,
    pub selected: usize,
    pub notice: String,
    pub progress: Progress,
    /// A girl was paid for: her id (she takes the Queen up; `burdel_<id>` opens in her room).
    pub scene: Option<String>,
    /// What was asked of her: the option's id (`sello` asks for the seal's shape).
    pub asked: Option<String>,
    /// The page on show, the option lit on it, and when it opened (ms of the game's clock, for
    /// its entrance).
    pub page: Page,
    pub choice: usize,
    pub page_at_ms: u32,
    /// The elixir was just bought: the Queen drinks it now.
    pub drink: bool,
    /// The page to go back to after the offer.
    pub offer_from: Page,
}

#[derive(Clone, Deserialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub tier: String,
    pub summary: String,
    pub description: String,
    pub quote: String,
    #[serde(default)]
    pub price: u32,
    #[serde(default)]
    pub reward: u32,
    #[serde(default)]
    pub company_price: u32,
    #[serde(default)]
    pub rest_price: u32,
    #[serde(default)]
    pub available: u32,
    #[serde(default)]
    pub duration: String,
    #[serde(default)]
    pub min_libido: u32,
    #[serde(default)]
    pub requires_room: bool,
    #[serde(default)]
    pub job_type: String,
    #[serde(default)]
    pub type_icon: String,
    #[serde(default)]
    pub privacy_stars: u32,
    #[serde(default)]
    pub privacy_label: String,
    #[serde(default)]
    pub comfort_stars: u32,
    #[serde(default)]
    pub comfort_label: String,
    #[serde(default)]
    pub benefits: Vec<String>,
    #[serde(default)]
    pub icon: String,
    /// Companions: the room she takes the Queen up to (0 or 1, the free ones).
    #[serde(default = "default_room")]
    pub room: usize,
    /// Companions: her word when she comes for the Queen at the desk.
    #[serde(default)]
    pub escort_line: String,
    /// Companions: her colour in the book (`#rrggbb`).
    #[serde(default)]
    pub accent: String,
    /// Companions: what can be asked of her.
    #[serde(default)]
    pub options: Vec<Ask>,
}

/// Something to ask of a girl (`burdel.json`, `options`).
#[derive(Clone, Deserialize)]
pub struct Ask {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub icon: String,
    /// Only with Rosalía's elixir drunk (and only once she has offered it).
    #[serde(default)]
    pub elixir: bool,
    /// She will not, yet: her answer (no gold changes hands).
    #[serde(default)]
    pub refuse: String,
}

fn default_room() -> usize {
    1
}

#[derive(Deserialize)]
pub struct Catalog {
    pub company: Vec<Entry>,
    pub jobs: Vec<Entry>,
    pub rooms: Vec<Entry>,
}

pub fn catalog() -> &'static Catalog {
    static DATA: OnceLock<Catalog> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../data/venues/burdel.json"))
            .expect("valid salon catalog")
    })
}

impl Menu {
    pub fn entries(&self) -> &[Entry] {
        match self.tab {
            0 => &catalog().company,
            1 => &catalog().jobs,
            _ => &catalog().rooms,
        }
    }

    pub fn select_tab(&mut self, tab: usize) {
        self.tab = tab % 3;
        self.selected = 0;
        self.notice.clear();
    }

    pub fn navigate(&mut self, delta: isize) {
        if self.page != Page::Girls {
            let len = match self.page {
                Page::Ask => self.asking().len(),
                Page::Offer => 3,
                _ => 0,
            };
            if len > 0 {
                self.choice = (self.choice as isize + delta).rem_euclid(len as isize) as usize;
            }
            self.notice.clear();
            return;
        }
        let len = self.entries().len();
        if len > 0 {
            self.selected = (self.selected as isize + delta).rem_euclid(len as isize) as usize;
        }
        self.notice.clear();
    }

    /// The book opens at the desk: Rosalía's offer first, the first time after a girl.
    pub fn open_book(&mut self, now_ms: u32) {
        self.open = true;
        self.select_tab(0);
        self.drink = false;
        if self.progress.visits >= 1 && !self.progress.elixir_offered {
            self.show(Page::Offer, now_ms);
            self.offer_from = Page::Girls;
        } else {
            self.show(Page::Girls, now_ms);
        }
    }

    /// Turns to `page`.
    pub fn show(&mut self, page: Page, now_ms: u32) {
        self.page = page;
        self.choice = 0;
        self.page_at_ms = now_ms;
        self.notice.clear();
    }

    /// Esc: back a page, or `false` when the book should close.
    pub fn back(&mut self, now_ms: u32) -> bool {
        match self.page {
            Page::Girls => false,
            Page::Ask => {
                self.show(Page::Girls, now_ms);
                true
            }
            Page::Offer => {
                self.progress.elixir_offered = true;
                let from = self.offer_from;
                self.show(from, now_ms);
                true
            }
            Page::Leaflet => {
                // Back to the offer, at the leaflet's button.
                self.show(Page::Offer, now_ms);
                self.choice = OFFER_LEAFLET;
                true
            }
        }
    }

    /// The elixir's leaflet, read from the offer.
    pub fn open_leaflet(&mut self, now_ms: u32) {
        self.show(Page::Leaflet, now_ms);
    }

    /// The girl chosen on the first page.
    pub fn girl(&self) -> Option<&'static Entry> {
        catalog().company.get(self.selected)
    }

    /// What can be asked of the chosen girl now, «Volver» last. Asking for the seal's shape only
    /// shows once Rosalía has offered her elixir.
    pub fn asking(&self) -> Vec<Asking> {
        let mut out: Vec<Asking> = self
            .asks()
            .into_iter()
            .map(|o| Asking {
                label: o.label.clone(),
                hint: if o.elixir && !self.progress.elixir_active {
                    "Necesitas el elixir de Rosalía".to_owned()
                } else {
                    o.hint.clone()
                },
                icon: o.icon.clone(),
                locked: o.elixir && !self.progress.elixir_active,
                awake: o.elixir && self.progress.elixir_active,
                back: false,
            })
            .collect();
        out.push(Asking {
            label: "Volver".to_owned(),
            hint: "A las demás chicas".to_owned(),
            icon: "back".to_owned(),
            locked: false,
            awake: false,
            back: true,
        });
        out
    }

    /// The options behind [`Menu::asking`], in the same order (without «Volver»).
    fn asks(&self) -> Vec<&'static Ask> {
        self.girl()
            .map(|g| {
                g.options
                    .iter()
                    .filter(|o| !o.elixir || self.progress.elixir_offered)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// What the elixir costs now.
    pub fn elixir_price(&self) -> u32 {
        if self.progress.elixirs_bought == 0 {
            ELIXIR_WELCOME
        } else {
            ELIXIR_PRICE
        }
    }

    /// The chosen girl's price.
    pub fn girl_price(&self) -> u32 {
        self.girl()
            .map(|e| {
                if e.company_price > 0 {
                    e.company_price
                } else {
                    e.price
                }
            })
            .unwrap_or(0)
    }

    pub fn has_room(&self) -> bool {
        self.progress.room.is_some() && self.progress.room_until > self.progress.hours
    }

    pub fn room_name(&self) -> &str {
        if !self.has_room() {
            return "Ninguna";
        }
        catalog()
            .rooms
            .iter()
            .find(|e| Some(&e.id) == self.progress.room.as_ref())
            .map(|e| e.name.as_str())
            .unwrap_or("Ninguna")
    }

    pub fn available_rooms_count(&self) -> u32 {
        catalog().rooms.iter().map(|r| r.available).sum()
    }

    pub fn blocked(&self, gold: u32) -> Option<&'static str> {
        let e = self.entries().get(self.selected)?;
        match self.tab {
            0 => {
                let cost = if e.company_price > 0 {
                    e.company_price
                } else {
                    e.price
                };
                if gold < cost {
                    return Some("No tienes suficiente oro.");
                }
            }
            1 => {
                if e.requires_room && !self.has_room() {
                    return Some("No puedes atender clientes sin una habitación libre.");
                }
                if self.progress.libido < e.min_libido {
                    return Some("Líbido insuficiente para este trabajo.");
                }
                if self.progress.shifts.contains(&e.id) {
                    return Some("Este encargo ya se completó.");
                }
                if self.progress.energy < 15 {
                    return Some("No tienes suficiente energía.");
                }
            }
            2 => {
                if e.available == 0 {
                    return Some("Esta estancia está ocupada en este momento.");
                }
                if self.has_room() {
                    return Some("Ya tienes una habitación alquilada. Puedes descansar en ella.");
                }
                if gold < e.price {
                    return Some("No tienes suficiente oro.");
                }
            }
            _ => {}
        }
        None
    }

    pub fn talk(&mut self) {
        if self.tab == 2 && self.has_room() {
            self.progress.hours = self.progress.hours.saturating_add(8);
            self.progress.energy = 100;
            self.notice = "Has descansado ocho horas. Energía restaurada al máximo.".into();
        } else if let Some(e) = self.entries().get(self.selected) {
            self.notice = e.quote.clone();
        }
    }

    /// Enter on the page on show. `now_ms`: the game's clock, for the next page's entrance.
    pub fn act_at(&mut self, gold: &mut u32, now_ms: u32) {
        if self.tab != 0 {
            self.act(gold);
            return;
        }
        match self.page {
            Page::Girls => {
                if self.girl().is_some() {
                    self.show(Page::Ask, now_ms);
                }
            }
            Page::Ask => {
                let asking = self.asking();
                let Some(pick) = asking.get(self.choice).cloned() else {
                    return;
                };
                if pick.back {
                    self.show(Page::Girls, now_ms);
                    return;
                }
                let Some(ask) = self.asks().get(self.choice).copied() else {
                    return;
                };
                if pick.locked {
                    // No elixir drunk: Rosalía's offer, and back here after.
                    self.show(Page::Offer, now_ms);
                    self.offer_from = Page::Ask;
                    return;
                }
                if !ask.refuse.is_empty() {
                    self.notice = ask.refuse.clone();
                    return;
                }
                if *gold < self.girl_price() {
                    self.notice = "No tienes suficiente oro.".into();
                    return;
                }
                self.asked = Some(ask.id.clone());
                self.act(gold);
            }
            Page::Leaflet => {
                self.back(now_ms);
            }
            Page::Offer => {
                if self.choice == OFFER_DECLINE {
                    self.back(now_ms);
                    return;
                }
                if self.choice == OFFER_LEAFLET {
                    self.open_leaflet(now_ms);
                    return;
                }
                let price = self.elixir_price();
                if *gold < price {
                    self.notice = "No tienes suficiente oro.".into();
                    return;
                }
                *gold -= price;
                self.progress.elixirs_bought += 1;
                self.progress.elixir_offered = true;
                self.drink = true;
                self.notice.clear();
            }
        }
    }

    /// After the elixir has gone down: back to the page the offer came from, the seal awake.
    pub fn drunk(&mut self, now_ms: u32) {
        self.drink = false;
        self.progress.elixir_active = true;
        self.open = true;
        let from = self.offer_from;
        self.show(from, now_ms);
        if from == Page::Ask {
            // The seal's option, lit.
            if let Some(i) = self.asking().iter().position(|a| a.awake) {
                self.choice = i;
            }
        }
    }

    pub fn act(&mut self, gold: &mut u32) {
        if let Some(reason) = self.blocked(*gold) {
            self.notice = reason.into();
            return;
        }
        let Some(e) = self.entries().get(self.selected) else {
            return;
        };
        let id = e.id.clone();
        let name = e.name.clone();
        let price = e.price;
        let company_cost = if e.company_price > 0 {
            e.company_price
        } else {
            e.price
        };
        let job_reward = if e.reward > 0 { e.reward } else { e.price };
        match self.tab {
            0 => {
                *gold = gold.saturating_sub(company_cost);
                self.progress.hours = self.progress.hours.saturating_add(1);
                self.progress.energy = (self.progress.energy + 20).min(100);
                self.progress.mood = (self.progress.mood + 25).min(100);
                self.notice = format!(
                    "Compartiste una hora de conversación con {name}. -{company_cost} oro."
                );
                self.progress.visits += 1;
                // She comes to take the Queen up to her room.
                self.scene = Some(id);
            }
            1 => {
                *gold = gold.saturating_add(job_reward);
                self.progress.shifts.push(id);
                self.progress.hours = self.progress.hours.saturating_add(2);
                self.progress.energy = self.progress.energy.saturating_sub(15);
                self.progress.mood = (self.progress.mood + 10).min(100);
                self.notice = format!("Turno de atención completado: {name}. +{job_reward} oro.");
            }
            _ => {
                *gold = gold.saturating_sub(price);
                self.progress.room = Some(id);
                self.progress.room_until = self.progress.hours.saturating_add(24);
                self.notice = format!(
                    "Habitación alquilada: {name}. Ahora puedes aceptar clientes. -{price} oro."
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn economy_and_reservation_guards() {
        let mut m = Menu::default();
        let mut gold = 100;

        // In Trabajar tab (1), cannot work without a room
        m.select_tab(1);
        m.act(&mut gold);
        assert_eq!(gold, 100, "cannot work without room");
        assert!(m.blocked(gold).is_some());

        // Rent a room in tab 2
        m.select_tab(2);
        m.act(&mut gold);
        assert_eq!(gold, 70, "modest room costs 30");
        assert!(m.has_room());
        assert_ne!(m.room_name(), "Ninguna");

        // Now we can accept a client job in tab 1
        m.select_tab(1);
        assert!(m.blocked(gold).is_none());
        m.act(&mut gold);
        assert_eq!(gold, 130, "merchant gives 60 gold");
        // Cannot do same job twice
        assert!(m.blocked(gold).is_some());

        // Resting in room
        m.select_tab(2);
        m.talk();
        assert_eq!(m.progress.energy, 100);
    }

    #[test]
    fn the_elixir_comes_after_the_first_girl_and_opens_her_seal_option() {
        let mut m = Menu::default();
        let mut gold = 200;
        m.open_book(0);
        assert_eq!(m.page, Page::Girls, "no offer before any girl");
        // Nina: what to ask of her; the seal's shape is not there yet.
        m.selected = 0;
        m.act_at(&mut gold, 1);
        assert_eq!(m.page, Page::Ask);
        assert!(
            m.asking().iter().all(|a| !a.locked && !a.awake),
            "no elixir option yet"
        );
        m.choice = 0;
        m.act_at(&mut gold, 2);
        assert_eq!(m.scene.as_deref(), Some("nina"));
        assert_eq!(m.asked.as_deref(), Some("mimos"));
        assert_eq!(gold, 155);
        assert_eq!(m.progress.visits, 1);
        // Back at the desk: Rosalía's offer first.
        m.scene = None;
        m.open_book(10);
        assert_eq!(m.page, Page::Offer);
        assert_eq!(m.elixir_price(), ELIXIR_WELCOME);
        // Declined: the girls; the seal's option shows, locked; picking it brings the offer back.
        assert!(m.back(11));
        assert_eq!(m.page, Page::Girls);
        m.act_at(&mut gold, 12);
        let seal = m
            .asking()
            .iter()
            .position(|a| a.locked)
            .expect("the seal's option, locked");
        m.choice = seal;
        m.act_at(&mut gold, 13);
        assert_eq!(m.page, Page::Offer);
        m.choice = 0;
        m.act_at(&mut gold, 14);
        assert!(m.drink);
        assert_eq!(gold, 155 - ELIXIR_WELCOME);
        assert_eq!(
            m.elixir_price(),
            ELIXIR_PRICE,
            "the welcome price only once"
        );
        m.drunk(15);
        assert_eq!(m.page, Page::Ask);
        assert!(m.asking()[m.choice].awake);
        m.act_at(&mut gold, 16);
        assert_eq!(m.asked.as_deref(), Some("sello"));
        assert_eq!(m.scene.as_deref(), Some("nina"));
        // Alba takes it too now (her minigame with the seal awake): she is paid and comes.
        m.scene = None;
        m.show(Page::Girls, 17);
        m.selected = 1;
        m.act_at(&mut gold, 18);
        let before = gold;
        m.choice = m
            .asking()
            .iter()
            .position(|a| a.awake)
            .expect("Alba's seal option");
        m.act_at(&mut gold, 19);
        assert_eq!(m.scene.as_deref(), Some("alba"));
        assert_eq!(m.asked.as_deref(), Some("sello"));
        assert!(gold < before);
    }

    #[test]
    fn the_leaflet_opens_from_the_offer_and_goes_back_to_it_without_buying() {
        let mut m = Menu::default();
        let mut gold = 200;
        m.progress.visits = 1;
        m.open_book(0);
        assert_eq!(m.page, Page::Offer);
        // Left from «Comprar» wraps round to the leaflet's button.
        m.navigate(-1);
        assert_eq!(m.choice, OFFER_LEAFLET);
        m.act_at(&mut gold, 1);
        assert_eq!(m.page, Page::Leaflet);
        assert_eq!(gold, 200, "reading costs nothing");
        // Esc (or Enter) on the leaflet: the offer again, at the same button, nothing bought.
        assert!(m.back(2));
        assert_eq!(m.page, Page::Offer);
        assert_eq!(m.choice, OFFER_LEAFLET);
        assert!(!m.drink);
        m.open_leaflet(3);
        m.act_at(&mut gold, 4);
        assert_eq!(m.page, Page::Offer);
        assert_eq!(m.progress.elixirs_bought, 0);
    }
}
