//! The Tendero's stall at the mine's mouth: he sells picks and bags for the shaft and buys what comes
//! up out of it, a little cheaper than the village's shop. Its rows, what choosing one does, and
//! where everything stands on the screen (the HUD draws it, the mouse finds it with the same layout).
use super::pozo::{self, BAGS, PICKS};
use super::Dungeon;

/// The stall's state while it is open.
#[derive(Clone, Debug, Default)]
pub struct Stall {
    pub open: bool,
    /// 0 buying, 1 selling.
    pub tab: u8,
    pub selection: usize,
    pub hover: Option<usize>,
    /// What he said last, and when.
    pub said: String,
    pub said_at: f32,
    pub opened_at: f32,
}

/// What a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deal {
    Pick(usize),
    Bag(usize),
    Sell(&'static str),
    SellAll,
    Nothing,
}

/// One row of the stall: what it shows and what it does.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub title: String,
    pub detail: String,
    /// The right-hand column: a price, «Tuyo», «En uso», a total.
    pub price: String,
    /// Whether she can take it now (owned or too dear: greyed).
    pub can: bool,
    /// A colour of its own (the pick's metal, the ore's).
    pub color: [f32; 3],
    pub deal: Deal,
}

/// Where the stall stands on the 1920×1080 screen.
pub const PANEL: [f32; 4] = [440.0, 150.0, 1040.0, 790.0];
pub const TAB_Y: f32 = 340.0;
pub const ROW_Y: f32 = 404.0;
pub const ROW_H: f32 = 56.0;
pub const ROWS_SHOWN: usize = 7;

/// The tabs' boxes.
pub fn tab_rect(i: u8) -> [f32; 4] {
    [PANEL[0] + 48.0 + i as f32 * 200.0, TAB_Y, 184.0, 44.0]
}

/// The close button's box.
pub fn close_rect() -> [f32; 4] {
    [PANEL[0] + PANEL[2] - 76.0, PANEL[1] + 26.0, 44.0, 44.0]
}

/// The first row shown (the list scrolls with the choice).
pub fn first_shown(selection: usize, count: usize) -> usize {
    if count <= ROWS_SHOWN {
        0
    } else {
        selection
            .saturating_sub(ROWS_SHOWN - 1)
            .min(count - ROWS_SHOWN)
    }
}

/// The box of row `i` (of `count`, with `selection` chosen), if shown.
pub fn row_rect(i: usize, selection: usize, count: usize) -> Option<[f32; 4]> {
    let first = first_shown(selection, count);
    (first..first + ROWS_SHOWN).contains(&i).then(|| {
        [
            PANEL[0] + 40.0,
            ROW_Y + (i - first) as f32 * ROW_H,
            PANEL[2] - 80.0,
            ROW_H - 6.0,
        ]
    })
}

fn inside(r: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= r[0] && p[0] <= r[0] + r[2] && p[1] >= r[1] && p[1] <= r[1] + r[3]
}

/// What a ore or find is worth at the stall (a little less than in the village).
pub fn mouth_price(id: &str) -> u32 {
    pozo::value(id)
        .map(|v| ((v as f32 * pozo::MOUTH_RATE).round() as u32).max(1))
        .unwrap_or(0)
}

const LINES_OPEN: [&str; 4] = [
    "¡Majestad! Picos, sacos y buen precio por lo que suba del pozo.",
    "Lo que hay dentro no lo vendo: eso se saca a pico… y se vuelve para contarlo.",
    "Cuanto más hondo, más brilla. Y más duele la espalda, se lo digo yo.",
    "En la villa le pagarán algo más, no le miento. Pero aquí le pago ya.",
];

impl Dungeon {
    /// The rows of the tab she is on.
    pub fn stall_rows(&self) -> Vec<Row> {
        let gold = self.gold + self.purse;
        if self.stall.tab == 0 {
            let mut rows = Vec::new();
            for (i, p) in PICKS.iter().enumerate().skip(1) {
                let owned = self.gear.pick >= i;
                let breaks = pozo::Rock::ALL
                    .iter()
                    .rev()
                    .find(|r| r.tier() <= p.tier)
                    .map_or("", |r| r.name());
                rows.push(Row {
                    title: p.name.to_string(),
                    detail: format!(
                        "Golpe {} · {:.1} golpes por segundo · rompe hasta {}",
                        p.damage, p.rate, breaks
                    ),
                    price: if self.gear.pick == i {
                        "En uso".into()
                    } else if owned {
                        "Tuyo".into()
                    } else {
                        format!("{} oro", p.price)
                    },
                    can: !owned && gold >= p.price,
                    color: p.color,
                    deal: Deal::Pick(i),
                });
            }
            for (i, b) in BAGS.iter().enumerate().skip(1) {
                let owned = self.gear.bag >= i;
                rows.push(Row {
                    title: b.name.to_string(),
                    detail: format!("Lleva {} piezas del pozo", b.capacity),
                    price: if self.gear.bag == i {
                        "En uso".into()
                    } else if owned {
                        "Tuyo".into()
                    } else {
                        format!("{} oro", b.price)
                    },
                    can: !owned && gold >= b.price,
                    color: [0.62, 0.42, 0.24],
                    deal: Deal::Bag(i),
                });
            }
            rows
        } else {
            let mut rows: Vec<Row> = Vec::new();
            let mut total = 0;
            let goods = pozo::ORES
                .iter()
                .map(|o| (o.id, o.name, o.color))
                .chain(std::iter::once((
                    pozo::FOSSIL.0,
                    pozo::FOSSIL.1,
                    [0.86, 0.8, 0.66],
                )));
            for (id, name, color) in goods {
                let n = self.materials.get(id).copied().unwrap_or(0);
                if n == 0 {
                    continue;
                }
                let each = mouth_price(id);
                total += each * n;
                rows.push(Row {
                    title: format!("{name} ×{n}"),
                    detail: format!(
                        "{each} oro cada una (en la villa, {})",
                        pozo::value(id).unwrap_or(0)
                    ),
                    price: format!("{} oro", each * n),
                    can: true,
                    color,
                    deal: Deal::Sell(id),
                });
            }
            if rows.is_empty() {
                rows.push(Row {
                    title: "No llevas nada del pozo".into(),
                    detail: "Baja al Pozo del Minero y súbeme lo que saques.".into(),
                    price: String::new(),
                    can: false,
                    color: [0.6, 0.6, 0.6],
                    deal: Deal::Nothing,
                });
            } else {
                rows.push(Row {
                    title: "Venderlo todo".into(),
                    detail: "Todo lo que llevas del pozo, de una vez".into(),
                    price: format!("{total} oro"),
                    can: true,
                    color: [1.0, 0.8, 0.3],
                    deal: Deal::SellAll,
                });
            }
            rows
        }
    }

    /// He opens his stall to her.
    pub fn open_stall(&mut self) {
        let line = LINES_OPEN[self.npc_talks % LINES_OPEN.len()];
        self.npc_talks += 1;
        self.stall = Stall {
            open: true,
            tab: 0,
            selection: 0,
            hover: None,
            said: line.into(),
            said_at: self.elapsed,
            opened_at: self.elapsed,
        };
        self.auto_walk = false;
    }

    pub fn close_stall(&mut self) {
        self.stall.open = false;
    }

    fn say(&mut self, line: impl Into<String>) {
        self.stall.said = line.into();
        self.stall.said_at = self.elapsed;
    }

    /// Gold she can spend: what she found on this trip first, then what she has at home.
    fn spend(&mut self, cost: u32) -> bool {
        if self.gold + self.purse < cost {
            return false;
        }
        let from_trip = cost.min(self.gold);
        self.gold -= from_trip;
        let rest = cost - from_trip;
        self.purse -= rest;
        self.purse_spent += rest;
        true
    }

    /// The chosen row's deal.
    pub fn stall_act(&mut self, index: usize) {
        let rows = self.stall_rows();
        let Some(row) = rows.get(index).cloned() else {
            return;
        };
        self.stall.selection = index;
        match row.deal {
            Deal::Pick(i) | Deal::Bag(i) => {
                let (owned, price, id, name) = match row.deal {
                    Deal::Pick(_) => (
                        self.gear.pick >= i,
                        PICKS[i].price,
                        PICKS[i].id,
                        PICKS[i].name,
                    ),
                    _ => (self.gear.bag >= i, BAGS[i].price, BAGS[i].id, BAGS[i].name),
                };
                if owned {
                    self.say("Ese ya es suyo, Majestad. Y del bueno.");
                    return;
                }
                if !self.spend(price) {
                    self.say(format!(
                        "Le faltan {} de oro. Pique un poco más y vuelva.",
                        price - (self.gold + self.purse)
                    ));
                    self.sound(super::sound::Sfx::BoneRattle, None, 0.3);
                    return;
                }
                match row.deal {
                    Deal::Pick(_) => self.gear.pick = i,
                    _ => self.gear.bag = i,
                }
                self.bought.push(id.to_string());
                self.say(format!(
                    "¡{name}! Que le dure… y que no me lo traiga mellado."
                ));
                self.sound(super::sound::Sfx::Coins, None, 0.8);
            }
            Deal::Sell(id) => {
                let n = self.materials.remove(id).unwrap_or(0);
                let paid = mouth_price(id) * n;
                self.gold += paid;
                self.say(format!("Hecho: {paid} de oro. Buen material, sí señora."));
                self.sound(super::sound::Sfx::Coins, None, 0.8);
                let count = self.stall_rows().len();
                self.stall.selection = self.stall.selection.min(count.saturating_sub(1));
            }
            Deal::SellAll => {
                let ids: Vec<String> = self
                    .materials
                    .keys()
                    .filter(|id| pozo::value(id).is_some())
                    .cloned()
                    .collect();
                let mut paid = 0;
                for id in ids {
                    let n = self.materials.remove(&id).unwrap_or(0);
                    paid += mouth_price(&id) * n;
                }
                self.gold += paid;
                self.say(format!(
                    "¡{paid} de oro! Con esto me compro otro barril de aceite."
                ));
                self.sound(super::sound::Sfx::Coins, None, 1.0);
                self.stall.selection = 0;
            }
            Deal::Nothing => {}
        }
    }

    /// Keys while the stall is open.
    pub fn stall_key(&mut self, code: winit::keyboard::KeyCode) {
        use winit::keyboard::KeyCode::*;
        let count = self.stall_rows().len();
        match code {
            Escape | KeyE | KeyG => self.close_stall(),
            KeyW | ArrowUp => self.stall.selection = self.stall.selection.saturating_sub(1),
            KeyS | ArrowDown => {
                self.stall.selection = (self.stall.selection + 1).min(count.saturating_sub(1))
            }
            KeyA | KeyD | ArrowLeft | ArrowRight | Tab => {
                self.stall.tab = 1 - self.stall.tab.min(1);
                self.stall.selection = 0;
            }
            KeyF | Enter | NumpadEnter | Space => self.stall_act(self.stall.selection),
            _ => {}
        }
    }

    /// The pointer over the stall (screen 1920×1080): which row it is on.
    pub fn stall_pointer(&mut self, at: Option<[f32; 2]>) {
        let count = self.stall_rows().len();
        self.stall.hover = at.and_then(|p| {
            (0..count)
                .find(|&i| row_rect(i, self.stall.selection, count).is_some_and(|r| inside(r, p)))
        });
    }

    /// A click on the stall.
    pub fn stall_click(&mut self, at: [f32; 2]) {
        if inside(close_rect(), at) {
            self.close_stall();
            return;
        }
        for tab in 0..2u8 {
            if inside(tab_rect(tab), at) {
                if self.stall.tab != tab {
                    self.stall.tab = tab;
                    self.stall.selection = 0;
                }
                return;
            }
        }
        let count = self.stall_rows().len();
        if let Some(i) = (0..count)
            .find(|&i| row_rect(i, self.stall.selection, count).is_some_and(|r| inside(r, at)))
        {
            if self.stall.selection == i {
                self.stall_act(i);
            } else {
                self.stall.selection = i;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn he_sells_picks_for_the_gold_she_has_and_buys_ore_a_little_cheaper() {
        let mut run = Dungeon::generate(5);
        run.gold = 20;
        run.purse = 200;
        run.open_stall();
        // the iron pick: 120, the trip's 20 first, then 100 from home
        let iron = run
            .stall_rows()
            .iter()
            .position(|r| r.deal == Deal::Pick(1))
            .unwrap();
        run.stall_act(iron);
        assert_eq!(run.gear.pick, 1);
        assert_eq!((run.gold, run.purse, run.purse_spent), (0, 100, 100));
        assert_eq!(run.bought, vec!["pico_hierro".to_string()]);
        // too dear: nothing changes
        let steel = run
            .stall_rows()
            .iter()
            .position(|r| r.deal == Deal::Pick(2))
            .unwrap();
        run.stall_act(steel);
        assert_eq!(run.gear.pick, 1);
        // selling: 80% of the village's price
        run.materials.insert("mineral_oro".into(), 5);
        run.materials.insert("hueso_viejo".into(), 3);
        run.stall.tab = 1;
        let rows = run.stall_rows();
        assert_eq!(
            rows.len(),
            2,
            "the gold and «sell it all»: bones are the shop's business"
        );
        run.stall_act(rows.len() - 1);
        assert_eq!(run.gold, 5 * mouth_price("mineral_oro"));
        assert_eq!(mouth_price("mineral_oro"), 22);
        assert_eq!(run.materials.get("hueso_viejo"), Some(&3));
    }
}
