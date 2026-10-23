//! Non-explicit salon menu: companion requests, client hospitality jobs, and room rentals.
//! Economy is stored in the exploration checkpoint; UI selection is transient.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    /// Gold owed to Rosalía: her loan and her cut (0, nothing owed).
    pub debt: u32,
    /// The world's day (`dia_mundo`) the debt falls due.
    pub debt_due: i32,
    /// How the Queen pays it once it is due: `""` (not chosen yet), [`WAY_MINE`], [`WAY_HOUSE`]
    /// or [`WAY_BANNED`].
    pub debt_way: String,
    /// Shifts worked in the house to pay it (they take turns of the clients' scenes).
    pub shifts_worked: u32,
    /// Loans taken so far.
    pub loans: u32,
    /// What the last loan lent (her ledger shows the loan, her cut and what is paid).
    pub debt_lent: u32,
    /// The house's jobs are open to her: she worked a debt off once (a shift or the shaft).
    pub jobs_open: bool,
    /// Jobs done for pay: shifts and shaft errands.
    pub jobs_shifts: u32,
    pub jobs_errands: u32,
    /// Times each client of the house was served (by his id).
    pub served: BTreeMap<String, u32>,
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
            debt: 0,
            debt_due: 0,
            debt_way: String::new(),
            shifts_worked: 0,
            loans: 0,
            debt_lent: 0,
            jobs_open: false,
            jobs_shifts: 0,
            jobs_errands: 0,
            served: BTreeMap::new(),
        }
    }
}

/// Rosalía's elixir: the price of the first (her welcome) and of the rest.
pub const ELIXIR_WELCOME: u32 = 10;
pub const ELIXIR_PRICE: u32 = 40;

/// Rosalía lends like a usurer (the user, 2026-10-22): when the Queen is short of gold at the
/// desk she offers one of these, to be paid back with her cut within a few days. Once due, the
/// Queen chooses how: the mine's earnings go to her till, shifts in the house, or none, and
/// the door stays shut until it is paid.
pub const LOANS: [u32; 2] = [100, 250];
/// Days to pay it back.
pub const LOAN_DAYS: i32 = 3;
/// What one shift in the house takes off the debt (or pays, as a job of the house).
pub const SHIFT_PAY: u32 = 80;
/// The shaft errand as a job of the house: gold of ore to dig for her wagon, and her pay for it.
pub const ERRAND_QUOTA: u32 = 120;
pub const ERRAND_PAY: u32 = 160;
pub const WAY_MINE: &str = "mina";
pub const WAY_HOUSE: &str = "casa";
pub const WAY_BANNED: &str = "vetada";

/// What is owed back for a loan: the loan and her 30 %.
pub fn owed_for(amount: u32) -> u32 {
    amount + amount * 3 / 10
}

/// What the loan page offers now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LoanAction {
    /// Take this much.
    Borrow(u32),
    /// Pay back all that is owed.
    Repay,
    /// Once due: the mine's earnings go to Rosalía.
    Mine,
    /// Once due: work shifts in the house.
    House,
    /// Once due: neither; the door stays shut until it is paid.
    Refuse,
    /// Paying in the house: one shift now.
    Shift,
    Back,
}

/// The book's pages: the girls side by side, what to ask of the one chosen, Rosalía's offer and
/// the elixir's leaflet (its side effects, read from the offer).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Page {
    #[default]
    Girls,
    Ask,
    Offer,
    Leaflet,
    /// Rosalía's loan: the offer, what is owed, and once due, how it will be paid.
    Loan,
    /// Rosalía's desk: the Queen's account with the house, paying it, the loan, the house's jobs.
    Desk,
    /// The house's clients, to serve one (once a debt was worked off).
    Clients,
}

/// The house's clients as Rosalía's book shows them (the shift minigame's are `turno::CLIENTS`):
/// who he is, a line on him, what he asks for, his guion scene (`turno_<n>`, its minigame), his
/// colour and whether he can be served yet (his clips are made).
pub struct ClientCard {
    pub id: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub about: &'static str,
    pub act: &'static str,
    pub scene: i32,
    pub accent: u32,
    pub ready: bool,
}

pub const ROSTER: [ClientCard; 5] = [
    ClientCard {
        id: "bram",
        name: "Bram",
        title: "Soldado novato",
        about: "Tímido, pecoso y obediente. Tiembla al pagar.",
        act: "La mamada",
        scene: 1,
        accent: 0xff6ea0,
        ready: true,
    },
    ClientCard {
        id: "rus",
        name: "Rus",
        title: "Goblin juguetón",
        about: "Chistes malos y manos rápidas. Paga en monedas sueltas.",
        act: "La cubana",
        scene: 4,
        accent: 0x8fd46a,
        ready: true,
    },
    ClientCard {
        id: "gorm",
        name: "Gorm",
        title: "Ogro de las flores",
        about: "Enorme, torpe y muy tierno. Trae flores del camino.",
        act: "El primer asalto",
        scene: 5,
        accent: 0xc9a36a,
        ready: true,
    },
    ClientCard {
        id: "varg",
        name: "Varg",
        title: "Lobo de la luna",
        about: "Un caballero con colmillos. Aúlla bajito.",
        act: "Por detrás",
        scene: 6,
        accent: 0x9fb4c9,
        ready: true,
    },
    ClientCard {
        id: "kael",
        name: "Kael",
        title: "Elfo oscuro",
        about: "Embajador arrogante. Pide «lo mejor de la casa».",
        act: "Pronto",
        scene: 7,
        accent: 0xb48cff,
        ready: false,
    },
];

/// What Rosalía's desk offers now.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeskAction {
    /// No debt: borrow (her notes); with one: her ledger (the loan page).
    Loan,
    /// Pay what is owed: all of it if the purse has it, else what it has.
    Pay,
    /// Work it off in the shaft: dig its worth for her wagon (or back down to finish it).
    Mine,
    /// Work it off in the house: a shift now.
    House,
    /// The house's clients: serve one, for pay (open once a debt was worked off).
    Clients,
    /// The shaft for the house: [`ERRAND_QUOTA`] of ore for her wagon, [`ERRAND_PAY`] for it.
    Errand,
    /// Something the house will offer later.
    Soon,
    Back,
}

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
    /// The world's day now (`dia_mundo`), set by the game before the book is used.
    pub today: i32,
    /// A shift in the house was chosen: the game plays its scene.
    pub shift: bool,
    /// She would not pay: the game sees her out of the house.
    pub expelled: bool,
    /// Down the shaft for Rosalía: the quota, the pay (0: a debt) and whether it works a debt off.
    pub to_mine: Option<(u32, u32, bool)>,
    /// A client chosen on his card: his guion scene (`turno_<n>`), served now.
    pub client_shift: Option<i32>,
}

/// One entry of a venue's data file. Some of its fields are there for the writers and the web
/// pages, not read by the game.
#[derive(Clone, Deserialize)]
#[allow(dead_code)]
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
                Page::Loan => self.loan_actions().len(),
                Page::Desk => self.desk_actions().len(),
                Page::Clients => ROSTER.len(),
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

    /// The book opens at the desk: a debt that fell due first (Rosalía collects), then her offer,
    /// the first time after a girl.
    pub fn open_book(&mut self, now_ms: u32) {
        self.open = true;
        self.select_tab(0);
        self.drink = false;
        self.shift = false;
        self.expelled = false;
        self.to_mine = None;
        self.client_shift = None;
        if self.collecting() {
            self.show(Page::Loan, now_ms);
            self.offer_from = Page::Girls;
        } else if self.progress.visits >= 1 && !self.progress.elixir_offered {
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
            Page::Loan => {
                let from = self.offer_from;
                self.show(from, now_ms);
                true
            }
            Page::Desk => {
                self.show(Page::Girls, now_ms);
                true
            }
            Page::Clients => {
                self.show(Page::Desk, now_ms);
                self.choice = self
                    .desk_actions()
                    .iter()
                    .position(|a| matches!(a, DeskAction::Clients | DeskAction::House))
                    .unwrap_or(0);
                true
            }
        }
    }

    /// Rosalía's desk, from the first page.
    pub fn open_desk(&mut self, now_ms: u32) {
        self.show(Page::Desk, now_ms);
    }

    /// Her desk's rows, in order.
    pub fn desk_actions(&self) -> Vec<DeskAction> {
        use DeskAction::*;
        let p = &self.progress;
        if p.debt == 0 {
            return vec![Clients, Errand, Loan, Soon, Back];
        }
        if p.debt_way == WAY_MINE {
            return vec![Mine, Pay, Loan, Back];
        }
        if p.debt_way == WAY_HOUSE {
            return vec![House, Pay, Loan, Back];
        }
        vec![Pay, Mine, House, Loan, Back]
    }

    /// Her desk's rows as the book draws them, and whether each is for adults.
    pub fn desk_rows(&self, gold: u32) -> Vec<(Asking, bool)> {
        let p = &self.progress;
        let debt = p.debt;
        let open = p.jobs_open;
        let row = |label: String, hint: String, icon: &str, locked: bool| Asking {
            label,
            hint,
            icon: icon.to_owned(),
            locked,
            awake: false,
            back: false,
        };
        let shut = || "Al saldar una deuda trabajando".to_owned();
        let days = self.days_left();
        self.desk_actions()
            .into_iter()
            .map(|a| match a {
                DeskAction::Loan if debt == 0 => (
                    row(
                        "Pedir un préstamo".into(),
                        format!(
                            "{} o {} oro · devuelves +30 % en {LOAN_DAYS} días",
                            LOANS[0], LOANS[1]
                        ),
                        "coin",
                        false,
                    ),
                    false,
                ),
                DeskAction::Loan => (
                    row(
                        "Su libro de cuentas".into(),
                        "Lo prestado, su 30 % y lo pagado".into(),
                        "candle",
                        false,
                    ),
                    false,
                ),
                DeskAction::Pay => (
                    if gold >= debt {
                        row(
                            format!("Pagar {debt} oro"),
                            "Lo tacha de su libro y quedas en paz".into(),
                            "coin",
                            false,
                        )
                    } else if gold > 0 {
                        row(
                            format!("Abonar {gold} oro"),
                            format!("Todo lo que llevas · quedarían {}", debt - gold),
                            "coin",
                            false,
                        )
                    } else {
                        row("Pagar".into(), "No llevas oro encima".into(), "coin", true)
                    },
                    false,
                ),
                DeskAction::Mine if p.debt_way == WAY_MINE => (
                    row(
                        "Volver al pozo".into(),
                        format!("Tu encargo sigue: {debt} oro en mineral"),
                        "pick",
                        false,
                    ),
                    false,
                ),
                DeskAction::Mine => (
                    row(
                        "Saldarla en el pozo".into(),
                        format!("Picar {debt} oro en mineral para ella"),
                        "pick",
                        false,
                    ),
                    false,
                ),
                DeskAction::House => (
                    row(
                        if p.debt_way == WAY_HOUSE {
                            "Atender a un cliente".into()
                        } else {
                            "Saldarla en la casa".into()
                        },
                        if open {
                            format!("Eliges cliente · −{SHIFT_PAY} por turno")
                        } else {
                            format!("Un cliente con máscara · −{SHIFT_PAY} por turno")
                        },
                        "mask",
                        false,
                    ),
                    true,
                ),
                DeskAction::Clients => (
                    row(
                        "Clientes de la casa".into(),
                        if open {
                            format!("Elige a quién atender · +{SHIFT_PAY} y propina")
                        } else {
                            shut()
                        },
                        "mask",
                        !open,
                    ),
                    true,
                ),
                DeskAction::Errand => (
                    row(
                        "Encargo en el pozo".into(),
                        if open {
                            format!("Picar {ERRAND_QUOTA} oro en mineral · cobras {ERRAND_PAY}")
                        } else {
                            shut()
                        },
                        "pick",
                        !open,
                    ),
                    false,
                ),
                DeskAction::Soon => (
                    row(
                        "Favores de Rosalía".into(),
                        "Pronto · la casa crece".into(),
                        "candle",
                        true,
                    ),
                    false,
                ),
                DeskAction::Back => (
                    Asking {
                        back: true,
                        ..row("Volver".into(), "A las chicas".into(), "back", false)
                    },
                    false,
                ),
            })
            .map(|(mut a, adult)| {
                if days < 0 && matches!(a.icon.as_str(), "coin") && !a.locked && debt > 0 {
                    a.hint = format!("{} · plazo vencido", a.hint);
                }
                (a, adult)
            })
            .collect()
    }

    /// Rosalía's line at her desk (or over her clients).
    pub fn desk_line(&self) -> &'static str {
        let p = &self.progress;
        if self.page == Page::Clients {
            return if p.debt > 0 {
                "—Cada uno, un pellizco menos de deuda. Elija con cariño… o con hambre."
            } else {
                "—Todos pagan bien, mi reina. Y todos se van contentos. Bueno… casi todos."
            };
        }
        if p.debt == 0 {
            if p.jobs_open {
                "—Mi mejor trabajadora. No ponga esa cara, es un cumplido. ¿Qué le apetece hoy?"
            } else {
                "—Su cuenta está limpia, mi reina. Si alguna noche le falta oro, ya sabe dónde está mi libro."
            }
        } else if p.debt_way == WAY_MINE {
            "—Mi vagoneta la espera en el pozo. Hasta que no esté llena, el enano no sube la jaula."
        } else if p.debt_way == WAY_HOUSE {
            "—Las chicas le guardan la barra. Un turno, y un pellizco menos."
        } else if self.overdue() {
            "—Vencida, cariño. Oro, pico o la casa. Usted elige… mientras pueda elegir."
        } else {
            "—Puede pagarme cuando quiera. Todo, o a poquitos. Yo lo apunto todo, ya lo sabe."
        }
    }

    /// Enter on her desk.
    fn act_desk(&mut self, gold: &mut u32, now_ms: u32) {
        let actions = self.desk_actions();
        let Some(&action) = actions.get(self.choice) else {
            return;
        };
        let shut = "—Primero salde una deuda trabajando, cariño. Luego hablamos de sueldo.";
        match action {
            DeskAction::Loan => {
                self.show(Page::Loan, now_ms);
                self.offer_from = Page::Desk;
            }
            DeskAction::Pay => {
                let p = &mut self.progress;
                if *gold == 0 {
                    self.notice = "No llevas oro encima.".into();
                    return;
                }
                let paid = (*gold).min(p.debt);
                *gold -= paid;
                p.debt -= paid;
                if p.debt == 0 {
                    p.debt_way.clear();
                    self.notice =
                        format!("Deuda saldada: {paid} oro. Rosalía lo tacha de su libro.");
                } else {
                    self.notice = format!("Abonas {paid} oro. Quedan {}.", p.debt);
                }
                self.choice = 0;
            }
            DeskAction::Mine => {
                let p = &mut self.progress;
                p.debt_way = WAY_MINE.to_owned();
                self.to_mine = Some((p.debt, 0, true));
            }
            DeskAction::House => {
                self.progress.debt_way = WAY_HOUSE.to_owned();
                // Once she knows the house, she picks the client; before, the next in turn.
                if self.progress.jobs_open {
                    self.show(Page::Clients, now_ms);
                } else {
                    self.shift = true;
                }
            }
            DeskAction::Clients => {
                if self.progress.jobs_open {
                    self.show(Page::Clients, now_ms);
                } else {
                    self.notice = shut.into();
                }
            }
            DeskAction::Errand => {
                if self.progress.jobs_open {
                    self.to_mine = Some((ERRAND_QUOTA, ERRAND_PAY, false));
                } else {
                    self.notice = shut.into();
                }
            }
            DeskAction::Soon => {
                self.notice = "—Pronto, mi reina. La casa crece… y usted con ella.".into();
            }
            DeskAction::Back => {
                self.back(now_ms);
            }
        }
    }

    /// Enter on a client's card: his shift (the game plays his scene, then his minigame).
    fn act_clients(&mut self) {
        let Some(c) = ROSTER.get(self.choice) else {
            return;
        };
        if !c.ready {
            self.notice = format!("—{} aún no ha pedido cita, mi reina. Pronto.", c.name);
            return;
        }
        self.client_shift = Some(c.scene);
    }

    /// Times this client has been served.
    pub fn served(&self, id: &str) -> u32 {
        self.progress.served.get(id).copied().unwrap_or(0)
    }

    /// A shaft errand came to an end: met (a debt written off, or its pay), or given up (a debt's:
    /// what was sent up comes off it, and the house shuts until it is paid). What the game says.
    pub fn errand_done(
        &mut self,
        met: bool,
        pay: u32,
        debt: bool,
        sent: u32,
        gold: &mut u32,
    ) -> String {
        let p = &mut self.progress;
        match (met, debt) {
            (true, true) => {
                let owed = p.debt;
                p.debt = 0;
                p.debt_way.clear();
                p.jobs_open = true;
                format!(
                    "Deuda saldada en el pozo
La vagoneta de Rosalía sube llena: {owed} oro menos. Su casa te ofrece trabajo."
                )
            }
            (true, false) => {
                *gold = gold.saturating_add(pay);
                p.jobs_errands += 1;
                format!(
                    "Encargo del pozo cumplido
+{pay} oro de Rosalía."
                )
            }
            (false, true) => {
                let off = sent.min(p.debt);
                p.debt -= off;
                if p.debt == 0 {
                    p.debt_way.clear();
                    return "Encargo abandonado
Lo que subiste ya cubría la deuda: Rosalía la da por saldada."
                        .into();
                }
                p.debt_way = WAY_BANNED.to_owned();
                format!(
                    "Encargo abandonado
Rosalía se queda lo que subiste (−{off}) y te veta la casa hasta que pagues {}.",
                    p.debt
                )
            }
            (false, false) => "Encargo abandonado
La casa no paga lo que no se termina."
                .into(),
        }
    }

    /// The debt fell due and how to pay it is not chosen yet.
    pub fn overdue(&self) -> bool {
        self.progress.debt > 0 && self.today > self.progress.debt_due
    }

    /// Rosalía wants her gold before anything else: due and not being paid off in the mine.
    pub fn collecting(&self) -> bool {
        self.overdue() && self.progress.debt_way != WAY_MINE
    }

    /// What has been paid off the last loan so far.
    pub fn debt_paid(&self) -> u32 {
        let p = &self.progress;
        if p.debt_lent == 0 {
            return 0;
        }
        owed_for(p.debt_lent).saturating_sub(p.debt)
    }

    /// Days left to pay (0 on the last one; negative once due).
    pub fn days_left(&self) -> i32 {
        self.progress.debt_due - self.today
    }

    /// The loan page's buttons, in order.
    pub fn loan_actions(&self) -> Vec<LoanAction> {
        let p = &self.progress;
        if p.debt == 0 {
            let mut out: Vec<LoanAction> = LOANS.iter().map(|&a| LoanAction::Borrow(a)).collect();
            out.push(LoanAction::Back);
            return out;
        }
        if p.debt_way == WAY_HOUSE {
            return vec![LoanAction::Shift, LoanAction::Repay, LoanAction::Back];
        }
        if self.overdue() && p.debt_way.is_empty() {
            return vec![LoanAction::Mine, LoanAction::House, LoanAction::Refuse];
        }
        vec![LoanAction::Repay, LoanAction::Back]
    }

    /// Whether a button can be taken with this gold.
    pub fn loan_enabled(&self, action: LoanAction, gold: u32) -> bool {
        match action {
            LoanAction::Repay => gold >= self.progress.debt,
            _ => true,
        }
    }

    /// Rosalía's line on the loan page.
    pub fn loan_line(&self) -> &'static str {
        let p = &self.progress;
        if p.debt == 0 {
            if p.loans == 0 {
                "—¿Sin oro, mi reina? Qué contratiempo… Yo le presto. Tres días. Y si no puede devolverlo, ya encontraremos la manera. Aquí todo se paga, de un modo u otro."
            } else {
                "—¿Otra vez corta de oro? La casa confía en usted, mi reina. Casi tanto como en su… capacidad de pago."
            }
        } else if p.debt_way == WAY_HOUSE {
            "—Las chicas le han dejado sitio en la barra. Un turno, y un pellizco menos de deuda. Usted elige al cliente… más o menos."
        } else if self.overdue() && p.debt_way.is_empty() {
            "—El plazo venció, cariño. No me ponga esa cara. Hay tres maneras de arreglarlo: el pico, la casa… o la calle."
        } else if p.debt_way == WAY_MINE {
            "—Mi amigo el enano me guarda lo que usted le vende. Pico, pico, y estamos en paz."
        } else {
            "—Sin prisa, mi reina. Bueno… con un poquito de prisa. El reloj corre, y mis intereses también."
        }
    }

    /// The loan page's top line: the deal or the debt.
    pub fn loan_title(&self) -> &'static str {
        let p = &self.progress;
        if p.debt == 0 {
            "PRÉSTAMO DE LA CASA"
        } else if p.debt_way == WAY_HOUSE {
            "TURNOS EN LA CASA"
        } else if self.overdue() && p.debt_way != WAY_MINE {
            "PLAZO VENCIDO"
        } else {
            "SU DEUDA"
        }
    }

    /// A shift worked: its pay comes off the debt. What it took off.
    pub fn shift_done(&mut self) -> u32 {
        let p = &mut self.progress;
        let paid = SHIFT_PAY.min(p.debt);
        p.debt -= paid;
        p.shifts_worked += 1;
        // A debt worked off once: the house offers her work from then on.
        p.jobs_open = true;
        if p.debt == 0 {
            p.debt_way.clear();
        }
        paid
    }

    /// A client's tip after a shift: it goes to the debt first. What it took off.
    pub fn take_tip(&mut self, tip: u32) -> u32 {
        let p = &mut self.progress;
        let off = tip.min(p.debt);
        p.debt -= off;
        if p.debt == 0 {
            p.debt_way.clear();
        }
        off
    }

    /// Paying it off in the mine: what Rosalía takes of `earned` (all of it, up to the debt).
    pub fn takes_from_sale(&mut self, earned: u32) -> u32 {
        let p = &mut self.progress;
        if p.debt_way != WAY_MINE || p.debt == 0 {
            return 0;
        }
        let cut = earned.min(p.debt);
        p.debt -= cut;
        if p.debt == 0 {
            p.debt_way.clear();
        }
        cut
    }

    /// Paid at the door with her own gold: whether it was enough (the door opens again).
    pub fn pay_at_door(&mut self, gold: &mut u32) -> bool {
        let p = &mut self.progress;
        if *gold < p.debt {
            return false;
        }
        *gold -= p.debt;
        p.debt = 0;
        p.debt_way.clear();
        true
    }

    /// Kept out of the house: she would not pay and still owes.
    pub fn banned(&self) -> bool {
        self.progress.debt > 0 && self.progress.debt_way == WAY_BANNED
    }

    /// Enter on the loan page.
    fn act_loan(&mut self, gold: &mut u32, now_ms: u32) {
        let actions = self.loan_actions();
        let Some(&action) = actions.get(self.choice) else {
            return;
        };
        let p = &mut self.progress;
        match action {
            LoanAction::Borrow(amount) => {
                *gold = gold.saturating_add(amount);
                p.debt = owed_for(amount);
                p.debt_due = self.today + LOAN_DAYS;
                p.debt_way.clear();
                p.loans += 1;
                p.debt_lent = amount;
                let owed = p.debt;
                let from = self.offer_from;
                self.show(from, now_ms);
                self.notice = format!(
                    "Rosalía te presta {amount} oro. Le devuelves {owed} en {LOAN_DAYS} días."
                );
            }
            LoanAction::Repay => {
                if *gold < p.debt {
                    self.notice = format!("Te faltan {} de oro.", p.debt - *gold);
                    return;
                }
                *gold -= p.debt;
                let paid = p.debt;
                p.debt = 0;
                p.debt_way.clear();
                self.show(Page::Girls, now_ms);
                self.notice = format!("Deuda saldada: {paid} oro. Rosalía sonríe y lo apunta.");
            }
            LoanAction::Mine => {
                // Down the shaft at once, to dig its worth for her wagon.
                p.debt_way = WAY_MINE.to_owned();
                self.to_mine = Some((p.debt, 0, true));
            }
            LoanAction::House => {
                p.debt_way = WAY_HOUSE.to_owned();
                self.show(Page::Loan, now_ms);
            }
            LoanAction::Refuse => {
                p.debt_way = WAY_BANNED.to_owned();
                self.expelled = true;
            }
            LoanAction::Shift => {
                self.shift = true;
            }
            LoanAction::Back => {
                self.back(now_ms);
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

    #[cfg(test)]
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

    /// Enter on the page on show. `now_ms`: the game's clock, for the next page's entrance.
    pub fn act_at(&mut self, gold: &mut u32, now_ms: u32) {
        if self.tab != 0 {
            self.act(gold);
            return;
        }
        // A debt due comes first: Rosalía collects before anything else is served.
        if matches!(self.page, Page::Girls | Page::Ask) && self.overdue() {
            if self.collecting() {
                self.show(Page::Loan, now_ms);
                self.offer_from = Page::Girls;
            } else {
                self.notice = format!(
                    "Primero la deuda: aún le debes {} oro a Rosalía.",
                    self.progress.debt
                );
            }
            return;
        }
        match self.page {
            Page::Girls => {
                if self.girl().is_some() {
                    self.show(Page::Ask, now_ms);
                }
            }
            Page::Loan => self.act_loan(gold, now_ms),
            Page::Desk => self.act_desk(gold, now_ms),
            Page::Clients => self.act_clients(),
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
                    if self.progress.debt == 0 {
                        // Short of gold: Rosalía notices, and lends.
                        self.show(Page::Loan, now_ms);
                        self.offer_from = Page::Ask;
                    } else {
                        self.notice = format!(
                            "No tienes suficiente oro (y ya le debes {} a Rosalía).",
                            self.progress.debt
                        );
                    }
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

    /// Nina picked with the gold short: the loan page opens; `pick` is taken on it.
    fn borrow(m: &mut Menu, gold: &mut u32, pick: usize) {
        m.open_book(0);
        m.selected = 0;
        m.act_at(gold, 1);
        assert_eq!(m.page, Page::Ask);
        m.choice = 0;
        m.act_at(gold, 2);
        assert_eq!(m.page, Page::Loan, "short of gold: Rosalía lends");
        m.choice = pick;
        m.act_at(gold, 3);
    }

    #[test]
    fn short_of_gold_rosalia_lends_and_it_is_paid_back_with_her_cut() {
        let mut m = Menu::default();
        let mut gold = 10;
        borrow(&mut m, &mut gold, 0);
        assert_eq!(gold, 10 + LOANS[0]);
        assert_eq!(m.progress.debt, owed_for(LOANS[0]));
        assert_eq!(m.progress.debt, 130);
        assert_eq!(m.progress.debt_due, LOAN_DAYS);
        assert_eq!(m.page, Page::Ask, "back where the girl was being asked");
        // Within the days she pays from the loan page.
        m.show(Page::Loan, 4);
        assert_eq!(m.loan_actions(), vec![LoanAction::Repay, LoanAction::Back]);
        gold = 200;
        m.choice = 0;
        m.act_at(&mut gold, 5);
        assert_eq!(gold, 70);
        assert_eq!(m.progress.debt, 0);
    }

    #[test]
    fn once_due_she_chooses_the_mine_the_house_or_the_street() {
        let mut gold = 0;
        // The mine: what she sells goes to Rosalía until it is paid; the book serves again.
        let mut m = Menu::default();
        borrow(&mut m, &mut gold, 1);
        assert_eq!(m.progress.debt, 325);
        m.today = LOAN_DAYS + 1;
        m.open_book(10);
        assert_eq!(m.page, Page::Loan, "due: she collects first");
        assert_eq!(
            m.loan_actions(),
            vec![LoanAction::Mine, LoanAction::House, LoanAction::Refuse]
        );
        m.choice = 0;
        m.act_at(&mut gold, 11);
        assert_eq!(m.progress.debt_way, WAY_MINE);
        assert!(!m.collecting());
        assert_eq!(m.takes_from_sale(200), 200);
        assert_eq!(m.takes_from_sale(200), 125);
        assert_eq!(m.progress.debt, 0);
        assert_eq!(m.takes_from_sale(50), 0, "paid: the sales are hers again");
        assert!(m.progress.debt_way.is_empty());

        // The house: shifts, each one off the debt.
        let mut m = Menu::default();
        let mut gold = 0;
        borrow(&mut m, &mut gold, 0);
        m.today = LOAN_DAYS + 1;
        m.open_book(10);
        m.choice = 1;
        m.act_at(&mut gold, 11);
        assert_eq!(m.progress.debt_way, WAY_HOUSE);
        assert_eq!(m.page, Page::Loan);
        assert_eq!(m.loan_actions()[0], LoanAction::Shift);
        m.choice = 0;
        m.act_at(&mut gold, 12);
        assert!(m.shift, "the game plays the shift's scene");
        assert_eq!(m.shift_done(), SHIFT_PAY);
        assert_eq!(m.shift_done(), 130 - SHIFT_PAY);
        assert_eq!(m.progress.debt, 0);
        assert_eq!(m.progress.shifts_worked, 2);

        // Neither: out, and the door stays shut until she pays at it.
        let mut m = Menu::default();
        let mut gold = 0;
        borrow(&mut m, &mut gold, 0);
        m.today = LOAN_DAYS + 1;
        m.open_book(10);
        m.choice = 2;
        m.act_at(&mut gold, 11);
        assert!(m.expelled);
        assert!(m.banned());
        gold = 50;
        assert!(!m.pay_at_door(&mut gold));
        gold = 150;
        assert!(m.pay_at_door(&mut gold));
        assert_eq!(gold, 20);
        assert!(!m.banned());
    }

    #[test]
    fn a_debt_due_blocks_the_girls_until_it_is_settled() {
        let mut m = Menu::default();
        let mut gold = 0;
        borrow(&mut m, &mut gold, 0);
        m.progress.debt_way = WAY_MINE.to_owned();
        m.today = LOAN_DAYS + 1;
        gold = 500;
        m.open_book(10);
        assert_eq!(
            m.page,
            Page::Girls,
            "paying in the mine: the book opens as usual"
        );
        m.act_at(&mut gold, 11);
        assert_eq!(m.page, Page::Girls, "but nothing is served");
        assert!(m.notice.contains("deuda"));
        assert_eq!(gold, 500);
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

    #[test]
    fn rosalias_desk_takes_gold_sends_her_down_the_shaft_and_opens_the_jobs() {
        let mut m = Menu::default();
        m.open_desk(1);
        assert_eq!(
            m.desk_actions(),
            vec![
                DeskAction::Clients,
                DeskAction::Errand,
                DeskAction::Loan,
                DeskAction::Soon,
                DeskAction::Back
            ]
        );
        // the clients stay shut until a debt is worked off
        m.choice = 0;
        let mut gold = 0;
        m.act_at(&mut gold, 2);
        assert_eq!(m.page, Page::Desk);
        // with a debt: pay what she has, a little at a time
        m.progress.debt = 130;
        m.progress.debt_lent = 100;
        assert_eq!(m.desk_actions()[0], DeskAction::Pay);
        let mut gold = 50;
        m.choice = 0;
        m.act_at(&mut gold, 3);
        assert_eq!((gold, m.progress.debt), (0, 80));
        // down the shaft for the rest
        m.choice = 1;
        m.act_at(&mut gold, 4);
        assert_eq!(m.to_mine, Some((80, 0, true)));
        assert_eq!(m.progress.debt_way, WAY_MINE);
        // given up: what was sent up comes off, and the house shuts
        let note = m.errand_done(false, 0, true, 30, &mut gold);
        assert!(note.contains("veta"), "{note}");
        assert_eq!(m.progress.debt, 50);
        assert!(m.banned());
        // met: written off, and the house offers work
        m.progress.debt_way = WAY_MINE.to_owned();
        m.errand_done(true, 0, true, 50, &mut gold);
        assert_eq!(m.progress.debt, 0);
        assert!(m.progress.jobs_open && !m.banned());
        // a paid errand pays
        m.errand_done(true, ERRAND_PAY, false, ERRAND_QUOTA, &mut gold);
        assert_eq!((gold, m.progress.jobs_errands), (ERRAND_PAY, 1));
    }
}
