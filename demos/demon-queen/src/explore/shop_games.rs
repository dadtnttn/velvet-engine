//! The village shop's two little games. Haggling: the shopkeeper's patience against the Queen's
//! nerve. A needle sweeps a bar and each tactic (flattery, pressing, a demon's threat) pays off
//! when it is stopped inside its zone, the narrower the zone the bigger the cut, and a miss costs
//! patience. When it stalls (or once he knows her) he hints at other ways to pay: «Otro trato», a
//! favour for a bigger cut (`tienda.rs`), played as a scene. Fusion: two things from her bag on his altar, a third out of them (the backpack's
//! recipes) for a fee. The rules come first; the shop's side (keys, gold, the bag, what the
//! overlay is shown) after them.
use super::ui_sfx::{self, UiSfx};
use super::*;

/// How far a price can move in her favour (percent).
pub(super) const MOST: u32 = 40;
/// Tries before his last word.
pub(super) const ROUNDS: u8 = 6;
/// His patience at the start.
pub(super) const PATIENCE: u8 = 3;
/// What he charges to fuse.
pub(super) const FUSE_FEE: u32 = 15;
/// How long the fusion's show lasts (s): the two spiral in, flash, and the third appears.
pub(super) const FUSE_SHOW: f32 = 1.6;
/// How long a closed haggle stays on show before the shelves come back (s).
const CLOSE_HOLD: f32 = 1.5;
/// His illustration, once it exists (under `data/assets`, without the extension). His other faces
/// sit beside it as `tendero_<face>.png`; a face that is not there yet shows this one.
const MERCHANT_ART: &str = "ui/shop/tendero";
/// How long his face answers an outcome before it settles back (s); a deal or his «out» keep theirs.
const FACE_HOLD: f32 = 1.8;
/// How long he keeps smiling after greeting her (s).
const GREET_HOLD: f32 = 2.6;

/// The shop's page: the shelves, a haggle over the lit product, or the fusion altar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Page {
    #[default]
    List,
    Haggle,
    Fuse,
}

/// The tiles of a haggle, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tactic {
    Flatter,
    Press,
    Threaten,
    Deal,
    Leave,
    /// «Otro trato»: the favour he asks for (only once he has hinted at it).
    Favor,
}

impl Tactic {
    pub(super) const ALL: [Tactic; 6] = [
        Tactic::Flatter,
        Tactic::Press,
        Tactic::Threaten,
        Tactic::Deal,
        Tactic::Leave,
        Tactic::Favor,
    ];
    /// The favour's tile among them.
    pub(super) const FAVOR: usize = 5;

    /// Half the width of its zone on the bar (0..1), the cut when stopped inside it and when
    /// dead on (percent), and the patience a miss costs.
    pub(super) fn rule(self) -> Option<(f32, u32, u32, u8)> {
        match self {
            Tactic::Flatter => Some((0.17, 4, 7, 0)),
            Tactic::Press => Some((0.11, 8, 12, 1)),
            Tactic::Threaten => Some((0.06, 15, 20, 2)),
            Tactic::Deal | Tactic::Leave | Tactic::Favor => None,
        }
    }
}

/// The tiles' names, for the haggle's ticket.
const TACTIC_NAMES: [&str; 6] = [
    "Halagar",
    "Regatear",
    "Intimidar",
    "Cerrar trato",
    "Dejarlo",
    "Otro trato",
];

/// What came of a try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    /// Dead on: the biggest cut.
    Perfect,
    Good,
    /// Outside the zone: no cut, and some patience lost.
    Miss,
    /// A threat missed: two patience at once.
    Offended,
    /// The price agreed and paid.
    Deal,
    /// His patience ran out: no more haggling over this today.
    Out,
    /// A favour paid: the cut it bought.
    Favor,
}

impl Outcome {
    /// Its number for the overlay.
    pub(super) fn code(self) -> u8 {
        match self {
            Outcome::Perfect => 0,
            Outcome::Good => 1,
            Outcome::Miss => 2,
            Outcome::Offended => 3,
            Outcome::Deal => 4,
            Outcome::Out => 5,
            Outcome::Favor => 6,
        }
    }

    /// The shopkeeper's face at it: ruined by a big cut, pondering a small one, refusing a miss,
    /// offended (and a little scared) by a threat, pleased with a deal, throwing her out.
    pub(super) fn face(self) -> &'static str {
        match self {
            Outcome::Perfect => "arruinado",
            Outcome::Good => "pensativo",
            Outcome::Miss => "negando",
            Outcome::Offended => "ofendido",
            Outcome::Deal | Outcome::Favor => "contento",
            Outcome::Out => "echando",
        }
    }
}

/// A haggle over the lit product.
#[derive(Clone, Debug)]
pub(super) struct Haggle {
    /// The product (its index in the catalog), whether she sells it, how many.
    pub product: usize,
    pub selling: bool,
    pub quantity: u32,
    /// The unit price before haggling.
    pub base: u32,
    /// How far it has moved in her favour (percent).
    pub percent: u32,
    pub patience: u8,
    pub rounds: u8,
    /// The lit tile.
    pub tactic: usize,
    /// The centre of the zone on the bar (0..1), and when the needle set off.
    pub zone: f32,
    pub needle_from: f32,
    /// The last try's outcome and when.
    pub result: Option<(Outcome, f32)>,
    /// When it ended (a deal, or his patience gone).
    pub closed: Option<f32>,
    /// «Otro trato»: the favour on its tile (`tienda::FAVORS`) and since when it is offered; one
    /// is paid per haggle.
    pub offer: Option<usize>,
    pub offer_at: f32,
    pub favor_done: bool,
    /// What has been tried, in order (its ticket on the page).
    pub log: Vec<Try>,
}

/// One try of a haggle, for its ticket: the tile played, what came of it, how far the price moved
/// (percent) and, for «Otro trato», which favour.
#[derive(Clone, Copy, Debug)]
pub(super) struct Try {
    pub tile: usize,
    pub outcome: Outcome,
    pub moved: u32,
    pub favor: Option<usize>,
}

impl Haggle {
    pub(super) fn new(product: usize, selling: bool, quantity: u32, base: u32, now: f32) -> Self {
        Self {
            product,
            selling,
            quantity,
            base,
            percent: 0,
            patience: PATIENCE,
            rounds: 0,
            tactic: 0,
            zone: zone_for(product, 0),
            needle_from: now,
            result: None,
            closed: None,
            offer: None,
            offer_at: now,
            favor_done: false,
            log: Vec::new(),
        }
    }

    /// The unit price as it stands: lower when she buys, higher when she sells.
    pub(super) fn price(&self) -> u32 {
        self.price_at(self.percent)
    }

    /// The unit price once it has moved `percent` (a favour's cut on top of the haggle's).
    pub(super) fn price_at(&self, percent: u32) -> u32 {
        // A favour can bring it all the way down.
        if !self.selling && percent >= 100 {
            return 0;
        }
        let moved = if self.selling {
            100 + percent
        } else {
            100 - percent
        };
        ((self.base * moved + 50) / 100).max(1)
    }

    /// What the whole lot comes to as it stands: the haggle's percent on the lot, not on each
    /// piece (a cheap thing sold by the dozen moves too: 28 bones at 1 gold, +40 %, fetch 39).
    pub(super) fn total(&self) -> u32 {
        self.total_at(self.percent)
    }

    /// The whole lot once the price has moved `percent`.
    pub(super) fn total_at(&self, percent: u32) -> u32 {
        if !self.selling && percent >= 100 {
            return 0;
        }
        let moved = if self.selling {
            100 + percent
        } else {
            100 - percent
        };
        let lot = self.base.saturating_mul(self.quantity.max(1));
        ((lot * moved + 50) / 100).max(1)
    }

    /// Where the needle is (0..1): back and forth along the bar, quicker each round.
    pub(super) fn needle(&self, now: f32) -> f32 {
        let period = (1.7 - 0.14 * self.rounds as f32).max(0.75);
        let u = ((now - self.needle_from).max(0.0) / period).rem_euclid(1.0);
        1.0 - (2.0 * u - 1.0).abs()
    }

    /// Plays `tactic` with the needle where it is at `now`: what came of it (`None` when it
    /// cannot be played: the deal and leaving are the shop's, and the tries are over).
    pub(super) fn play(&mut self, tactic: Tactic, now: f32) -> Option<Outcome> {
        let (half, good, perfect, loss) = tactic.rule()?;
        if self.closed.is_some() || self.rounds >= ROUNDS {
            return None;
        }
        let off = (self.needle(now) - self.zone).abs();
        let mut outcome = if off <= half * 0.35 {
            Outcome::Perfect
        } else if off <= half {
            Outcome::Good
        } else if loss >= 2 {
            Outcome::Offended
        } else {
            Outcome::Miss
        };
        // Words alone go no further than `MOST` (a favour may have gone past it already).
        let most = MOST.max(self.percent);
        let before = self.percent;
        match outcome {
            Outcome::Perfect => self.percent = (self.percent + perfect).min(most),
            Outcome::Good => self.percent = (self.percent + good).min(most),
            _ => self.patience = self.patience.saturating_sub(loss),
        }
        self.rounds += 1;
        self.zone = zone_for(self.product, self.rounds);
        self.needle_from = now;
        if self.patience == 0 {
            outcome = Outcome::Out;
            self.closed = Some(now);
        }
        self.result = Some((outcome, now));
        self.log.push(Try {
            tile: Tactic::ALL.iter().position(|t| *t == tactic).unwrap_or(0),
            outcome,
            moved: self.percent - before,
            favor: None,
        });
        Some(outcome)
    }

    /// Closed long enough ago for the shelves to come back.
    pub(super) fn over(&self, now: f32) -> bool {
        self.closed.is_some_and(|at| now - at >= CLOSE_HOLD)
    }
}

/// Where the zone sits on a product's `round`-th try (0.22..0.78): the same every time.
fn zone_for(product: usize, round: u8) -> f32 {
    let mut x =
        (product as u32).wrapping_mul(0x9e37_79b1) ^ u32::from(round).wrapping_mul(0x85eb_ca77);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2c1b_3c6d);
    x ^= x >> 12;
    0.22 + 0.56 * ((x & 0xffff) as f32 / 65535.0)
}

/// His first word when she starts to haggle.
pub(super) const OPENING: &str =
    "—¿Regatear con un comerciante honrado? Adelante, Majestad… si se atreve.";
const LAST: &str = "—Última oferta, Majestad. Lo toma o lo deja.";
const LEAVE: &str = "—Vuelva cuando quiera… a precio completo.";

/// What he says to an outcome (`n` picks among his lines).
pub(super) fn line(outcome: Outcome, tactic: Tactic, selling: bool, n: usize) -> &'static str {
    let pick = |lines: &[&'static str]| lines[n % lines.len()];
    match outcome {
        Outcome::Perfect if selling => pick(&[
            "—¡Ay! Está bien, le pago más. ¡Me arruina!",
            "—Esa mirada… De acuerdo, subo la oferta.",
        ]),
        Outcome::Perfect => pick(&[
            "—¡Ay! ¡Me va a arruinar!",
            "—Esa mirada… Está bien, está bien. Rebajo.",
            "—Con usted no se puede. Trato más dulce.",
        ]),
        Outcome::Good if selling => pick(&[
            "—Un poco más. Solo un poco.",
            "—Puedo subir algo, Majestad.",
        ]),
        Outcome::Good => pick(&[
            "—Hmm. Un poco. Solo un poco.",
            "—Puedo hacer un esfuerzo, Majestad.",
            "—Por ser usted… algo menos.",
        ]),
        Outcome::Miss if tactic == Tactic::Flatter => pick(&[
            "—Los halagos no pagan el alquiler.",
            "—Qué amable. El precio sigue igual.",
        ]),
        Outcome::Miss => pick(&[
            "—Ni hablar. Ese es mi precio.",
            "—Me ofende, Majestad. Un poco.",
        ]),
        Outcome::Offended => pick(&[
            "—¿Me está amenazando? ¡En mi propia tienda!",
            "—¡Esos cuernos no me asustan! …Mucho.",
        ]),
        Outcome::Deal => pick(&[
            "—Trato hecho. Un placer hacer negocios.",
            "—Cerrado. No se lo cuente a nadie.",
        ]),
        Outcome::Out => "—¡Basta! Hoy no regateo más con usted.",
        Outcome::Favor => "—Trato es trato, Majestad. Ese precio es suyo.",
    }
}

/// His hint that there are other ways to pay: the first time ever, and once he knows her.
const HINT_FIRST: &str =
    "—Mire, Majestad… si el oro no llega, hay otras formas de pagar. Usted ya me entiende.";
const HINT_AGAIN: &str = "—¿Lo de siempre, Majestad? …O algo más.";
/// The same when he has dared his new one today already: tomorrow, more (sleeping passes the day).
const HINT_TOMORROW: &str = "—¿Lo de siempre, Majestad? Para algo más… vuelva mañana.";
/// When she has done them all.
const HINT_ALL: &str = "—¿Lo de siempre, Majestad? Elija usted. Ya no me queda nada por pedirle.";
/// After a new one, while there are more.
const AFTER_NEW: &str = "…Y mañana, si quiere, algo más.";

/// The fusion altar.
#[derive(Clone, Debug, Default)]
pub(super) struct Fuse {
    /// The lit tile of her things.
    pub cursor: usize,
    /// What lies in the altar's two sockets.
    pub slots: [Option<String>; 2],
    /// The fusion going off: when, and what it makes.
    pub fusing: Option<(f32, String)>,
    /// What went into it (for its show).
    pub fused_from: [String; 2],
    /// The last thing made, shown in the result socket until the altar changes.
    pub made: Option<String>,
    pub notice: String,
}

/// What `a` and `b` make on the altar, when they make anything.
pub(super) fn fusion(a: &str, b: &str) -> Option<&'static str> {
    backpack::recipe_for(a, b).map(|r| r.result)
}

/// An item's name, from the shop's catalog or the things only a combination makes.
pub(super) fn item_name(id: &str) -> String {
    shop::catalog()
        .products
        .iter()
        .find(|p| p.id == id)
        .map(|p| p.name.clone())
        .or_else(|| backpack::extra(id).map(|e| e.name.to_owned()))
        .unwrap_or_else(|| id.to_owned())
}

// ───────────────────────────── the shop's side ─────────────────────────────

impl App {
    /// The lit product's haggle starts (if he will still haggle over it today).
    pub(super) fn start_haggle(&mut self) {
        let Some(index) = self.selected_shop_product_index() else {
            ui_sfx::play(UiSfx::Error);
            return;
        };
        if self.shop_refused.contains(&index) {
            self.shop_notice = Some("Hoy no quiere regatear eso con usted.".to_owned());
            ui_sfx::play(UiSfx::Error);
            return;
        }
        let p = &shop::catalog().products[index];
        let selling = self.shop_tab == shop::Tab::Sell;
        let base = if selling { p.sell_price } else { p.buy_price };
        self.haggle = Some(Haggle::new(
            index,
            selling,
            self.shop_quantity.max(1),
            base,
            self.elapsed,
        ));
        self.set_shop_page(Page::Haggle);
        self.shop_say(OPENING);
        self.refresh_offer();
        ui_sfx::play(UiSfx::Open);
    }

    /// «Otro trato» comes up when she is buying and the haggle stalls (two tries, his patience
    /// short, her gold short), or at once if he already knows her; his hint is said once.
    pub(super) fn refresh_offer(&mut self) {
        let (choices, fresh) = play_scene::tienda::available(&self.story);
        let known = self.story.get(play_scene::tienda::LADDER) > 0;
        let gold = self.gold;
        let now = self.elapsed;
        let Some(h) = self.haggle.as_mut() else {
            return;
        };
        if h.offer.is_some()
            || h.favor_done
            || h.selling
            || h.closed.is_some()
            || choices.is_empty()
        {
            return;
        }
        let short = h.total() > gold;
        if !(known || h.rounds >= 2 || h.patience < PATIENCE || short) {
            return;
        }
        h.offer = fresh.or(choices.last().copied());
        h.offer_at = now;
        let all = choices.len() >= play_scene::tienda::FAVORS.len() && fresh.is_none();
        self.shop_say(match (known, fresh.is_some(), all) {
            (false, _, _) => HINT_FIRST,
            (true, true, _) => HINT_AGAIN,
            (true, false, true) => HINT_ALL,
            (true, false, false) => HINT_TOMORROW,
        });
    }

    /// A/D on the favour's tile: the other favours she can offer (the new one, the old ones).
    pub(super) fn cycle_favor(&mut self, by: isize) {
        let (choices, _) = play_scene::tienda::available(&self.story);
        let Some(h) = self.haggle.as_mut() else {
            return;
        };
        let Some(on) = h.offer else {
            return;
        };
        if choices.len() < 2 {
            return;
        }
        let at = choices.iter().position(|&c| c == on).unwrap_or(0) as isize;
        let next = choices[(at + by).rem_euclid(choices.len() as isize) as usize];
        h.offer = Some(next);
        h.tactic = Tactic::FAVOR;
        self.shop_say(play_scene::tienda::FAVORS[next].ask);
        ui_sfx::play(UiSfx::Move);
    }

    /// Back from a favour: a new step climbs the ladder (one new a day); sealed, its cut joins the
    /// haggle's and the deal is lit; backed out, the price stays.
    pub(super) fn after_favor(&mut self, favor: usize, done: bool) {
        use play_scene::tienda::{FAVORS, LADDER, TODAY};
        let now = self.elapsed;
        let new = done && favor as i32 >= self.story.get(LADDER);
        if new {
            self.story
                .values
                .insert(LADDER.to_owned(), favor as i32 + 1);
            self.story.values.insert(TODAY.to_owned(), 1);
        }
        let Some(f) = FAVORS.get(favor) else {
            return;
        };
        if let Some(h) = self.haggle.as_mut() {
            if done {
                let before = h.percent;
                h.percent = (h.percent + f.percent).min(100);
                h.log.push(Try {
                    tile: Tactic::FAVOR,
                    outcome: Outcome::Favor,
                    moved: h.percent - before,
                    favor: Some(favor),
                });
                h.favor_done = true;
                h.offer = None;
                h.tactic = Tactic::ALL
                    .iter()
                    .position(|t| *t == Tactic::Deal)
                    .unwrap_or(3);
                h.result = Some((Outcome::Favor, now));
            }
        }
        // A new one done: he says what he will dare tomorrow (one new a day; sleeping passes it).
        let words = match (done, new && favor + 1 < FAVORS.len()) {
            // Two lines in one: each translated on its own (the pair has no entry).
            (true, true) => {
                let tr = |s: &str| demon_queen_dungeon::i18n::tr(s).into_owned();
                format!("{} {}", tr(f.after), tr(AFTER_NEW))
            }
            (true, false) => f.after.to_owned(),
            (false, _) => "—¿Ya? Pues el precio sigue como estaba, Majestad.".to_owned(),
        };
        self.shop_say(&words);
        ui_sfx::play(if done { UiSfx::Deal } else { UiSfx::Back });
    }

    /// The shop changes page (its entrance counts from now).
    pub(super) fn set_shop_page(&mut self, page: Page) {
        if self.shop_page != page {
            self.shop_page = page;
            self.shop_page_at = self.elapsed;
        }
    }

    /// The shopkeeper says `words` in his box.
    pub(super) fn shop_say(&mut self, words: &str) {
        self.shop_said = Some((words.to_owned(), self.elapsed));
    }

    /// A key while haggling.
    pub(super) fn haggle_key(&mut self, code: KeyCode) {
        let Some(h) = self.haggle.as_mut() else {
            return;
        };
        if h.closed.is_some() {
            return;
        }
        // The favour's tile is among them only while it is offered.
        let tiles = if h.offer.is_some() {
            Tactic::ALL.len()
        } else {
            Tactic::FAVOR
        };
        match code {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                h.tactic = (h.tactic.min(tiles - 1) + tiles - 1) % tiles;
                let ask = (h.tactic == Tactic::FAVOR).then_some(h.offer).flatten();
                if let Some(f) = ask {
                    self.shop_say(play_scene::tienda::FAVORS[f].ask);
                }
                ui_sfx::play(UiSfx::Move);
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                h.tactic = (h.tactic.min(tiles - 1) + 1) % tiles;
                let ask = (h.tactic == Tactic::FAVOR).then_some(h.offer).flatten();
                if let Some(f) = ask {
                    self.shop_say(play_scene::tienda::FAVORS[f].ask);
                }
                ui_sfx::play(UiSfx::Move);
            }
            KeyCode::ArrowLeft | KeyCode::KeyA if h.tactic == Tactic::FAVOR => self.cycle_favor(-1),
            KeyCode::ArrowRight | KeyCode::KeyD if h.tactic == Tactic::FAVOR => self.cycle_favor(1),
            KeyCode::Digit6 | KeyCode::Numpad6 if h.offer.is_some() => {
                h.tactic = Tactic::FAVOR;
                self.haggle_play(Tactic::Favor);
            }
            KeyCode::Digit1
            | KeyCode::Digit2
            | KeyCode::Digit3
            | KeyCode::Digit4
            | KeyCode::Digit5 => {
                let i = match code {
                    KeyCode::Digit1 => 0,
                    KeyCode::Digit2 => 1,
                    KeyCode::Digit3 => 2,
                    KeyCode::Digit4 => 3,
                    _ => 4,
                };
                h.tactic = i;
                self.haggle_play(Tactic::ALL[i]);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                let tactic = Tactic::ALL[h.tactic];
                self.haggle_play(tactic);
            }
            KeyCode::Escape | KeyCode::Backspace | KeyCode::KeyF => self.haggle_play(Tactic::Leave),
            _ => {}
        }
    }

    /// The tile `i` of the haggle, clicked: lit, and played if it was already.
    pub(super) fn haggle_click(&mut self, i: usize) {
        let Some(h) = self.haggle.as_mut() else {
            return;
        };
        if h.closed.is_some() || i >= Tactic::ALL.len() || (i == Tactic::FAVOR && h.offer.is_none())
        {
            return;
        }
        h.tactic = i;
        self.haggle_play(Tactic::ALL[i]);
    }

    /// Plays a tactic: a try at the needle, the deal, or leaving.
    pub(super) fn haggle_play(&mut self, tactic: Tactic) {
        let now = self.elapsed;
        let Some(h) = self.haggle.as_mut() else {
            return;
        };
        match tactic {
            Tactic::Favor => {
                // The favour plays on the stage; the haggle waits under it.
                if let Some(favor) = h.offer {
                    ui_sfx::play(UiSfx::Open);
                    self.open_play_scene(play_scene::Game::Tienda(favor as u8));
                }
            }
            Tactic::Leave => {
                self.haggle = None;
                self.set_shop_page(Page::List);
                self.shop_say(LEAVE);
                ui_sfx::play(UiSfx::Back);
            }
            Tactic::Deal => {
                let (total, quantity, selling) = (h.total(), h.quantity, h.selling);
                self.shop_quantity = quantity;
                if self.commit_shop_at(Some(total)) {
                    if let Some(h) = self.haggle.as_mut() {
                        h.closed = Some(now);
                        h.result = Some((Outcome::Deal, now));
                    }
                    let n = (now * 7.0) as usize;
                    self.shop_say(line(Outcome::Deal, tactic, selling, n));
                    ui_sfx::play(UiSfx::Deal);
                } else {
                    ui_sfx::play(UiSfx::Error);
                }
            }
            _ => {
                let selling = h.selling;
                let product = h.product;
                let Some(outcome) = h.play(tactic, now) else {
                    self.shop_say(LAST);
                    ui_sfx::play(UiSfx::Error);
                    return;
                };
                let last = h.rounds >= ROUNDS && h.closed.is_none();
                let n = (now * 13.0) as usize;
                self.shop_say(if last && !matches!(outcome, Outcome::Out) {
                    LAST
                } else {
                    line(outcome, tactic, selling, n)
                });
                if outcome == Outcome::Out {
                    self.shop_refused.push(product);
                } else {
                    // A haggle going badly reminds him of other ways to pay.
                    self.refresh_offer();
                }
                ui_sfx::play(match outcome {
                    Outcome::Perfect => UiSfx::Deal,
                    Outcome::Good => UiSfx::Select,
                    Outcome::Miss => UiSfx::Fail,
                    Outcome::Offended | Outcome::Out => UiSfx::Error,
                    Outcome::Deal | Outcome::Favor => UiSfx::Coins,
                });
            }
        }
    }

    /// Each step: a closed haggle gives the shelves back; a fusion's show ends in its result.
    pub(super) fn tick_shop(&mut self) {
        if !self.shop_open {
            return;
        }
        let now = self.elapsed;
        if self.haggle.as_ref().is_some_and(|h| h.over(now)) {
            self.haggle = None;
            self.set_shop_page(Page::List);
        }
        if let Some((at, made)) = self.fuse.fusing.clone() {
            if now - at >= FUSE_SHOW {
                self.fuse.fusing = None;
                *self.inventory.entry(made.clone()).or_insert(0) += 1;
                let mut pack = self.backpack.clone();
                pack.reconcile(&self.inventory);
                self.backpack = pack;
                self.fuse.notice = format!("Obtuviste: {}.", item_name(&made));
                self.fuse.made = Some(made);
                self.achievements.record("shop:fuse");
                self.persist_3d_state(true);
                ui_sfx::play(UiSfx::Deal);
            }
        }
    }

    /// Her things for the altar: what she has, by name, with how many.
    pub(super) fn fuse_items(&self) -> Vec<(String, String, u32)> {
        let mut items: Vec<(String, String, u32)> = self
            .inventory
            .iter()
            .filter(|(_, n)| **n > 0)
            .map(|(id, n)| (id.clone(), item_name(id), *n))
            .collect();
        items.sort_by(|a, b| a.1.cmp(&b.1));
        items
    }

    /// The altar opens (the third tab).
    pub(super) fn open_fusion(&mut self) {
        self.haggle = None;
        self.fuse.cursor = self
            .fuse
            .cursor
            .min(self.fuse_items().len().saturating_sub(1));
        self.set_shop_page(Page::Fuse);
        self.shop_say("—Dos cosas suyas en el altar y una tarifa justa. Lo que salga… ya veremos.");
        ui_sfx::play(UiSfx::Page);
    }

    /// A key at the altar.
    pub(super) fn fuse_key(&mut self, code: KeyCode) {
        if self.fuse.fusing.is_some() {
            return;
        }
        let count = self.fuse_items().len();
        let step = |cursor: usize, by: isize| -> usize {
            if count == 0 {
                0
            } else {
                (cursor as isize + by).clamp(0, count as isize - 1) as usize
            }
        };
        let before = self.fuse.cursor;
        match code {
            KeyCode::ArrowLeft | KeyCode::KeyA => self.fuse.cursor = step(before, -1),
            KeyCode::ArrowRight | KeyCode::KeyD => self.fuse.cursor = step(before, 1),
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.fuse.cursor = step(before, -(FUSE_COLUMNS as isize))
            }
            KeyCode::ArrowDown | KeyCode::KeyS => {
                self.fuse.cursor = step(before, FUSE_COLUMNS as isize)
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => self.fuse_pick(before),
            KeyCode::KeyE => self.fuse_go(),
            KeyCode::KeyQ | KeyCode::Delete => self.fuse_clear(),
            _ => {}
        }
        if self.fuse.cursor != before {
            ui_sfx::play(UiSfx::Move);
        }
    }

    /// Her thing `i` onto the altar (the first empty socket, else the second).
    pub(super) fn fuse_pick(&mut self, i: usize) {
        let items = self.fuse_items();
        let Some((id, _, owned)) = items.get(i) else {
            return;
        };
        let socket = usize::from(self.fuse.slots[0].is_some());
        // The same thing twice needs two of it.
        let twice = self.fuse.slots[1 - socket].as_deref() == Some(id.as_str());
        self.fuse.cursor = i;
        self.fuse.made = None;
        if twice && *owned < 2 {
            self.fuse.notice = "Solo tienes uno de eso.".to_owned();
            ui_sfx::play(UiSfx::Error);
            return;
        }
        self.fuse.slots[socket] = Some(id.clone());
        self.fuse.notice.clear();
        ui_sfx::play(UiSfx::Pick);
    }

    /// The altar's socket `slot` emptied.
    pub(super) fn fuse_take(&mut self, slot: usize) {
        if self.fuse.fusing.is_none() && self.fuse.slots.get(slot).is_some_and(Option::is_some) {
            self.fuse.slots[slot] = None;
            self.fuse.notice.clear();
            ui_sfx::play(UiSfx::Drop);
        }
    }

    /// Both sockets emptied.
    pub(super) fn fuse_clear(&mut self) {
        if self.fuse.fusing.is_none() && self.fuse.slots.iter().any(Option::is_some) {
            self.fuse.slots = [None, None];
            self.fuse.notice.clear();
            ui_sfx::play(UiSfx::Drop);
        }
    }

    /// The fusion goes off: the fee paid, the two taken, the show begins (its result comes at
    /// its end, in `tick_shop`).
    pub(super) fn fuse_go(&mut self) {
        if self.fuse.fusing.is_some() {
            return;
        }
        let [Some(a), Some(b)] = self.fuse.slots.clone() else {
            self.fuse.notice = "Pon dos cosas en el altar.".to_owned();
            ui_sfx::play(UiSfx::Error);
            return;
        };
        let Some(made) = fusion(&a, &b) else {
            self.fuse.notice = "Estos dos no reaccionan.".to_owned();
            ui_sfx::play(UiSfx::Fail);
            return;
        };
        if self.gold < FUSE_FEE {
            self.fuse.notice = format!("Te faltan {} de oro.", FUSE_FEE - self.gold);
            ui_sfx::play(UiSfx::Error);
            return;
        }
        let have = |id: &str| self.inventory.get(id).copied().unwrap_or(0);
        let enough = if a == b {
            have(&a) >= 2
        } else {
            have(&a) >= 1 && have(&b) >= 1
        };
        if !enough {
            self.fuse.notice = "Ya no tienes lo que pusiste.".to_owned();
            self.fuse.slots = [None, None];
            ui_sfx::play(UiSfx::Error);
            return;
        }
        self.gold -= FUSE_FEE;
        for id in [&a, &b] {
            if let Some(n) = self.inventory.get_mut(id.as_str()) {
                *n -= 1;
                if *n == 0 {
                    self.inventory.remove(id.as_str());
                }
            }
        }
        self.fuse.slots = [None, None];
        self.fuse.fused_from = [a, b];
        self.fuse.fusing = Some((self.elapsed, made.to_owned()));
        self.fuse.notice.clear();
        let count = self.fuse_items().len();
        self.fuse.cursor = self.fuse.cursor.min(count.saturating_sub(1));
        ui_sfx::play(UiSfx::Fuse);
    }

    /// What the overlay shows of the shop beyond its shelves.
    pub(super) fn shop_view(&self) -> Option<map_overlay::ShopView> {
        // A favour on the stage hides the shop until it closes.
        if !self.shop_open || self.venue.open || self.play_scene.is_some() {
            return None;
        }
        let now = self.elapsed;
        let ms = |t: f32| ((now - t).max(0.0) * 1000.0) as u32;
        let named = |id: &Option<String>| id.as_ref().map(|id| (id.clone(), item_name(id)));
        let haggle = self.haggle.as_ref().map(|h| {
            let tactic = Tactic::ALL[h.tactic.min(Tactic::ALL.len() - 1)];
            map_overlay::HaggleView {
                product: h.product,
                selling: h.selling,
                quantity: h.quantity,
                base: h.base,
                price: h.price(),
                total: h.total(),
                percent: h.percent,
                patience: h.patience,
                rounds: h.rounds,
                max_rounds: ROUNDS,
                tactic: h.tactic as u8,
                needle: h.needle(now),
                zone: h.zone,
                half: tactic.rule().map_or(0.0, |r| r.0),
                result: h.result.map(|(o, at)| (o.code(), ms(at))),
                closed: h.closed.is_some(),
                gold_short: (!h.selling)
                    .then(|| h.total().saturating_sub(self.gold))
                    .unwrap_or(0),
                log: h
                    .log
                    .iter()
                    .map(|t| {
                        let label = match t.favor {
                            Some(i) => play_scene::tienda::FAVORS[i].name.to_owned(),
                            None => TACTIC_NAMES[t.tile.min(TACTIC_NAMES.len() - 1)].to_owned(),
                        };
                        (label, t.outcome.code(), t.moved)
                    })
                    .collect(),
                favor: h.offer.map(|i| {
                    let f = &play_scene::tienda::FAVORS[i];
                    let (choices, fresh) = play_scene::tienda::available(&self.story);
                    map_overlay::FavorTile {
                        name: f.name.to_owned(),
                        percent: f.percent,
                        new: fresh == Some(i),
                        tomorrow: fresh.is_none()
                            && choices.len() < play_scene::tienda::FAVORS.len(),
                        choices: choices.len() as u8,
                        age_ms: ms(h.offer_at),
                    }
                }),
            }
        });
        let fuse = (self.shop_page == Page::Fuse).then(|| {
            let [a, b] = &self.fuse.slots;
            map_overlay::FuseView {
                items: self.fuse_items(),
                cursor: self.fuse.cursor,
                slots: [named(a), named(b)],
                result: match (a, b) {
                    (Some(a), Some(b)) => fusion(a, b).map(|r| (r.to_owned(), item_name(r))),
                    _ => None,
                },
                fee: FUSE_FEE,
                show: self
                    .fuse
                    .fusing
                    .as_ref()
                    .map(|(at, made)| map_overlay::FuseShow {
                        ms: ms(*at),
                        made: (made.clone(), item_name(made)),
                        from: self.fuse.fused_from.clone(),
                    }),
                made: named(&self.fuse.made),
                notice: self.fuse.notice.clone(),
            }
        });
        let (face, before, changed) = self.merchant_faces();
        Some(map_overlay::ShopView {
            page: match self.shop_page {
                Page::List => 0,
                Page::Haggle => 1,
                Page::Fuse => 2,
            },
            open_ms: ms(self.shop_opened_at),
            page_ms: ms(self.shop_page_at),
            said: self
                .shop_said
                .as_ref()
                .map(|(text, at)| (text.clone(), ms(*at))),
            merchant: merchant_art().map(|crop| merchant_face(face, crop)),
            merchant_from: merchant_art().map(|crop| merchant_face(before, crop)),
            merchant_ms: ms(changed),
            haggle,
            fuse,
            refused: self.shop_refused.clone(),
        })
    }

    /// The shopkeeper's face now, the one before it, and when it changed (the game's clock): a
    /// smile while he greets her, then his usual face; in a haggle, each outcome's face for a
    /// moment (a deal's or his «out» to the end).
    fn merchant_faces(&self) -> (&'static str, &'static str, f32) {
        let now = self.elapsed;
        if let Some(h) = &self.haggle {
            return match h.result {
                Some((outcome, at)) => {
                    let face = outcome.face();
                    let sticky =
                        matches!(outcome, Outcome::Deal | Outcome::Out) || h.closed.is_some();
                    if sticky || now - at < FACE_HOLD {
                        (face, "normal", at)
                    } else {
                        ("normal", face, at + FACE_HOLD)
                    }
                }
                None => ("normal", "contento", self.shop_page_at),
            };
        }
        let greeted = self.shop_opened_at;
        if now - greeted < GREET_HOLD {
            ("contento", "normal", greeted)
        } else {
            ("normal", "contento", greeted + GREET_HOLD)
        }
    }
}

/// His picture with `face` («normal» is his illustration itself), or his illustration when that
/// face has not been drawn yet.
pub(super) fn merchant_face(face: &'static str, crop: [u32; 4]) -> map_overlay::PortraitRef {
    static FOUND: OnceLock<std::collections::HashSet<&'static str>> = OnceLock::new();
    let found = FOUND.get_or_init(|| {
        let dir = crate::exploration3d_state::data_dir().join("assets");
        [
            "contento",
            "arruinado",
            "pensativo",
            "negando",
            "ofendido",
            "echando",
        ]
        .into_iter()
        .filter(|f| dir.join(format!("{MERCHANT_ART}_{f}.png")).exists())
        .collect()
    });
    let image = if face != "normal" && found.contains(face) {
        format!("{MERCHANT_ART}_{face}")
    } else {
        MERCHANT_ART.to_owned()
    };
    map_overlay::PortraitRef { image, crop }
}

/// The altar's grid is this many things wide.
pub(super) const FUSE_COLUMNS: usize = 4;

/// His illustration's square (the top of it, full width), if it exists (looked for once).
pub(super) fn merchant_art() -> Option<[u32; 4]> {
    static FOUND: OnceLock<Option<[u32; 4]>> = OnceLock::new();
    *FOUND.get_or_init(|| {
        let path = crate::exploration3d_state::data_dir()
            .join("assets")
            .join(format!("{MERCHANT_ART}.png"));
        let (w, h) = image::image_dimensions(path).ok()?;
        let side = w.min(h);
        Some([(w - side) / 2, 0, side, side])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_needle_sweeps_and_a_dead_on_threat_takes_the_biggest_cut() {
        let mut h = Haggle::new(0, false, 1, 100, 10.0);
        assert_eq!(h.needle(10.0), 0.0);
        assert!(
            (h.needle(10.0 + 1.7 * 0.5) - 1.0).abs() < 1e-4,
            "the far end halfway"
        );
        // Stop it on the zone: the time the needle takes to get there on its way out.
        let at = 10.0 + 1.7 * 0.5 * h.zone;
        assert_eq!(h.play(Tactic::Threaten, at), Some(Outcome::Perfect));
        assert_eq!(h.percent, 20);
        assert_eq!(h.price(), 80);
        assert_eq!(h.rounds, 1);
    }

    #[test]
    fn each_try_goes_on_the_ticket_with_how_far_it_moved_the_price() {
        let mut h = Haggle::new(0, false, 1, 100, 10.0);
        let at = 10.0 + 1.7 * 0.5 * h.zone;
        h.play(Tactic::Threaten, at);
        let from = h.needle_from;
        h.play(Tactic::Flatter, from);
        let tries: Vec<(usize, Outcome, u32)> =
            h.log.iter().map(|t| (t.tile, t.outcome, t.moved)).collect();
        assert_eq!(
            tries,
            vec![(2, Outcome::Perfect, 20), (0, Outcome::Miss, 0)]
        );
        assert!(h.log.iter().all(|t| t.favor.is_none()));
        // A favour's cut on top of the haggle's: the price it would come to.
        assert_eq!(h.price_at(h.percent + 10), 70);
        assert_eq!(h.price_at(100), 0);
    }

    #[test]
    fn misses_cost_patience_and_his_patience_ends_it() {
        let mut h = Haggle::new(3, false, 1, 50, 0.0);
        // Far from any zone: the needle at an end (0), the zone never below 0.22.
        assert_eq!(h.play(Tactic::Flatter, 0.0), Some(Outcome::Miss));
        assert_eq!(h.patience, PATIENCE, "flattery costs nothing");
        let from = h.needle_from;
        assert_eq!(h.play(Tactic::Press, from), Some(Outcome::Miss));
        assert_eq!(h.patience, PATIENCE - 1);
        let from = h.needle_from;
        assert_eq!(h.play(Tactic::Threaten, from), Some(Outcome::Out));
        assert!(h.closed.is_some());
        assert_eq!(h.play(Tactic::Flatter, from + 0.1), None, "over is over");
        assert!(h.over(from + 2.0));
    }

    #[test]
    fn cuts_stop_at_the_most_and_selling_raises_the_price() {
        let mut h = Haggle::new(1, true, 2, 30, 0.0);
        for _ in 0..ROUNDS {
            let at = h.needle_from + (1.7 - 0.14 * h.rounds as f32).max(0.75) * 0.5 * h.zone;
            h.play(Tactic::Threaten, at);
        }
        assert_eq!(h.percent, MOST);
        assert_eq!(h.price(), 42);
        assert_eq!(h.play(Tactic::Flatter, 99.0), None, "no tries left");
    }

    #[test]
    fn a_cheap_lot_sold_by_the_dozen_fetches_more_too() {
        // 28 old bones at 1 gold each: +40 % on the lot, not 1 gold each rounded back to 1.
        let mut h = Haggle::new(0, true, 28, 1, 0.0);
        assert_eq!(h.total(), 28);
        h.percent = 40;
        assert_eq!(h.total(), 39);
        // Buying, the cut is on the lot as well.
        let mut b = Haggle::new(0, false, 3, 5, 0.0);
        b.percent = 20;
        assert_eq!(b.total(), 12);
    }

    #[test]
    fn a_favour_goes_past_what_words_can_and_free_is_free() {
        let mut h = Haggle::new(0, false, 1, 100, 0.0);
        h.percent = 75;
        // A good try after a favour does not take the favour's cut back to the words' most.
        let at = h.needle_from + 1.7 * 0.5 * h.zone;
        assert_eq!(h.play(Tactic::Threaten, at), Some(Outcome::Perfect));
        assert_eq!(h.percent, 75);
        h.percent = 100;
        assert_eq!(h.price(), 0);
        assert_eq!(Tactic::ALL[Tactic::FAVOR], Tactic::Favor);
        assert!(Tactic::Favor.rule().is_none());
    }

    #[test]
    fn the_altar_knows_the_backpack_recipes_both_ways() {
        assert_eq!(
            fusion("hierbas_curativas", "pocion_pequena"),
            Some("pocion_reforzada")
        );
        assert_eq!(
            fusion("pocion_pequena", "hierbas_curativas"),
            Some("pocion_reforzada")
        );
        assert_eq!(fusion("pan_del_dia", "hueso_viejo"), None);
        assert!(!item_name("cataplasma_herbal").is_empty());
    }
}
