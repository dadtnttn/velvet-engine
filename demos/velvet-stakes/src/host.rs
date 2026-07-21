//! Story host: `style.*` (`.vcss`) + `stakes.*` game commands.
//!
//! The author language (`.vstory`) drives flow; this host mutates shared run state
//! and keeps the active stylesheet in sync for paint + deal motion.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use indexmap::IndexMap;
use velvet_story::{
    CommandOutcome, StoryCommandError, StoryCommandHost, StoryValue, StoryVariables,
};
use velvet_story_lang::commands::{CommandParam, CommandRegistry, CommandSpec, ParamTy};
use velvet_style::{
    call_style_fn, emit_style_event, parse_stylesheet, JsValue, StyleStoryHost, Stylesheet,
};

use crate::catalog::CardStats;
use crate::game::{Outcome, Run, Screen, BLINDS};

pub const COLLECTION_DECK_MIN: usize = 8;
pub const COLLECTION_DECK_MAX: usize = 40;

/// One concrete card copy offered by the Night Market.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketOffer {
    pub card_id: String,
    pub price: i64,
    pub bought: bool,
}

/// Shared game world visible to the window loop and story host.
pub struct StakesWorld {
    pub screen: Screen,
    pub menu_sel: usize,
    pub pause_sel: usize,
    pub result_sel: usize,
    pub collection_sel: usize,
    pub collection_filter: usize,
    pub collection_status: String,
    pub result: Option<Outcome>,
    pub run: Option<Run>,
    pub stats: HashMap<String, CardStats>,
    pub starter_deck_ids: Vec<String>,
    pub deck_ids: Vec<String>,
    pub stylesheet: Stylesheet,
    pub money: i64,
    pub seed: u64,
    pub status: String,
    pub quit: bool,
    /// Story wait phase mirrored for input routing (`title`, `blind`, `play`, …).
    pub wait_phase: Option<String>,
    pub data_root: PathBuf,
    pub meta_chips: i64,
    pub meta_crystals: i64,
    pub meta_mult: f32,
    pub market_offers: Vec<MarketOffer>,
    pub market_sel: usize,
    pub market_status: String,
    pub market_rerolls: u32,
    pub market_visit: u32,
}

impl StakesWorld {
    pub fn new(
        stats: HashMap<String, CardStats>,
        deck_ids: Vec<String>,
        data_root: PathBuf,
    ) -> Self {
        let starter_deck_ids = deck_ids.clone();
        Self {
            screen: Screen::Title,
            menu_sel: 0,
            pause_sel: 0,
            result_sel: 0,
            collection_sel: 0,
            collection_filter: 0,
            collection_status: "Select a card to inspect and edit the starter deck.".into(),
            result: None,
            run: None,
            stats,
            starter_deck_ids,
            deck_ids,
            stylesheet: Stylesheet::default(),
            money: 0,
            seed: 0xBA_1A_70_01,
            status: "Velvet Arcana".into(),
            quit: false,
            wait_phase: None,
            data_root,
            meta_chips: 12_450,
            meta_crystals: 870,
            meta_mult: 3.2,
            market_offers: Vec::new(),
            market_sel: 0,
            market_status: "Select a card to inspect it.".into(),
            market_rerolls: 0,
            market_visit: 0,
        }
    }

    pub fn begin_run(&mut self) {
        self.deck_ids = self.starter_deck_ids.clone();
        self.money = 4;
        self.seed = self.seed.wrapping_add(13);
        self.start_blind(0);
    }

    pub fn start_blind(&mut self, ante: usize) {
        self.seed = self.seed.wrapping_add(1);
        let mut run = Run::start(ante, &self.deck_ids, self.seed, self.money);
        let sheet = self.stylesheet.clone();
        run.rebuild_visuals(true, Some(&sheet));
        self.status = format!("{} / {}", run.blind_name, run.target);
        self.run = Some(run);
        self.screen = Screen::BlindInfo;
        self.result = None;
    }

    pub fn enter_play(&mut self) {
        self.screen = Screen::Play;
        self.status = "1-8 select · P play · D discard".into();
        let sheet = self.stylesheet.clone();
        if let Some(r) = self.run.as_mut() {
            // re-deal motion when entering play from blind info
            r.rebuild_visuals(true, Some(&sheet));
        }
    }

    pub fn apply_outcome(&mut self, o: Outcome) {
        if let Some(r) = &self.run {
            self.money = r.money;
        }
        self.result = Some(o);
        self.screen = Screen::Result;
        self.result_sel = 0;
        self.status = match o {
            Outcome::WinBlind => "¡Ciega superada!".into(),
            Outcome::LoseBlind => "Ciega fallida".into(),
            Outcome::RunClear => "¡Run completa!".into(),
        };
    }

    /// Open the archive in starter-deck editing mode.
    pub fn open_collection(&mut self) {
        self.collection_sel = 0;
        self.collection_filter = 0;
        self.collection_status =
            "Select a card, then add or remove a copy from the starter deck.".into();
        self.screen = Screen::Collection;
    }

    /// Add one real catalog card to the persistent starter deck.
    pub fn add_collection_card(&mut self, id: &str) -> bool {
        if self.run.is_some() {
            self.collection_status = "Starter decks cannot be edited during a run.".into();
            return false;
        }
        let Some(card) = self.stats.get(id) else {
            self.collection_status = "That card is not in the archive.".into();
            return false;
        };
        if self.starter_deck_ids.len() >= COLLECTION_DECK_MAX {
            self.collection_status = format!("Deck limit reached: {COLLECTION_DECK_MAX} cards.");
            return false;
        }
        let name = card.name.clone();
        self.starter_deck_ids.push(id.to_string());
        self.deck_ids = self.starter_deck_ids.clone();
        self.collection_status = format!(
            "{name} added. {} / {COLLECTION_DECK_MAX} cards.",
            self.starter_deck_ids.len()
        );
        true
    }

    /// Remove one matching card copy while preserving a playable deck size.
    pub fn remove_collection_card(&mut self, id: &str) -> bool {
        if self.run.is_some() {
            self.collection_status = "Starter decks cannot be edited during a run.".into();
            return false;
        }
        if self.starter_deck_ids.len() <= COLLECTION_DECK_MIN {
            self.collection_status =
                format!("Keep at least {COLLECTION_DECK_MIN} cards in the starter deck.");
            return false;
        }
        let Some(index) = self.starter_deck_ids.iter().rposition(|card| card == id) else {
            self.collection_status = "No copy of that card remains in the deck.".into();
            return false;
        };
        let name = self
            .stats
            .get(id)
            .map(|card| card.name.clone())
            .unwrap_or_else(|| id.to_string());
        self.starter_deck_ids.remove(index);
        self.deck_ids = self.starter_deck_ids.clone();
        self.collection_status = format!(
            "{name} removed. {} / {COLLECTION_DECK_MAX} cards.",
            self.starter_deck_ids.len()
        );
        true
    }

    /// Build fresh deterministic market stock for the current visit.
    pub fn open_market(&mut self) {
        self.market_visit = self.market_visit.saturating_add(1);
        self.market_rerolls = 0;
        self.market_sel = 0;
        self.refresh_market_stock();
        self.screen = Screen::Shop;
        self.market_status = if self.run.is_some() {
            "Blind cleared. Improve the deck, then continue.".into()
        } else {
            "Browse the stock. Start a run to make purchases.".into()
        };
    }

    /// Current progressive reroll price.
    pub fn market_reroll_cost(&self) -> i64 {
        1 + self.market_rerolls as i64
    }

    /// Purchase one offered card and add a copy to this run's deck.
    pub fn buy_market_offer(&mut self, index: usize) -> bool {
        self.market_sel = index.min(self.market_offers.len().saturating_sub(1));
        let Some(offer) = self.market_offers.get(index).cloned() else {
            self.market_status = "That offer is no longer available.".into();
            return false;
        };
        if self.run.is_none() {
            self.market_status = "Start a run before buying market cards.".into();
            return false;
        }
        if offer.bought {
            self.market_status = "That copy is already sold.".into();
            return false;
        }
        if self.money < offer.price {
            self.market_status = format!("Need ${} more for this card.", offer.price - self.money);
            return false;
        }

        self.money -= offer.price;
        self.deck_ids.push(offer.card_id.clone());
        if let Some(item) = self.market_offers.get_mut(index) {
            item.bought = true;
        }
        if let Some(run) = self.run.as_mut() {
            run.money = self.money;
        }
        let name = self
            .stats
            .get(&offer.card_id)
            .map(|card| card.name.as_str())
            .unwrap_or(offer.card_id.as_str());
        self.market_status = format!("{name} added to the run deck.");
        true
    }

    /// Replace the market stock after paying its current cost.
    pub fn reroll_market(&mut self) -> bool {
        if self.run.is_none() {
            self.market_status = "Rerolls become available during a run.".into();
            return false;
        }
        let price = self.market_reroll_cost();
        if self.money < price {
            self.market_status = format!("Need ${} more to reroll stock.", price - self.money);
            return false;
        }
        self.money -= price;
        if let Some(run) = self.run.as_mut() {
            run.money = self.money;
        }
        self.market_rerolls = self.market_rerolls.saturating_add(1);
        self.market_sel = 0;
        self.refresh_market_stock();
        self.market_status = format!("Fresh stock arrived. Reroll cost ${price}.");
        true
    }

    fn refresh_market_stock(&mut self) {
        const IDS: [&str; 5] = ["strike", "guard", "fireball", "focus", "bash"];
        let mut state = self.seed
            ^ (self.market_visit as u64).wrapping_mul(0x9E37_79B9)
            ^ (self.market_rerolls as u64).wrapping_mul(0xD1B5_4A32);
        let mut offers = Vec::with_capacity(5);
        for slot in 0..5u64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407 + slot * 97);
            let id = IDS[((state >> 32) as usize) % IDS.len()];
            let base = self.stats.get(id).map(|card| card.cost as i64).unwrap_or(1);
            let premium = ((state >> 17) & 1) as i64;
            offers.push(MarketOffer {
                card_id: id.into(),
                price: (base + 1 + premium).max(1),
                bought: false,
            });
        }
        self.market_offers = offers;
    }

    pub fn to_title(&mut self) {
        self.run = None;
        self.result = None;
        self.screen = Screen::Title;
        self.menu_sel = 0;
    }

    pub fn sync_vars(&self, vars: &mut StoryVariables) {
        vars.set("ui.screen", StoryValue::String(self.screen.as_id().into()));
        vars.set("stakes.money", StoryValue::Int(self.money));
        vars.set(
            "stakes.outcome",
            StoryValue::String(
                self.result
                    .map(|o| o.as_str().into())
                    .unwrap_or_else(|| "none".into()),
            ),
        );
        if let Some(r) = &self.run {
            vars.set("stakes.score", StoryValue::Int(r.score));
            vars.set("stakes.target", StoryValue::Int(r.target));
            vars.set("stakes.ante", StoryValue::Int(r.ante as i64));
            vars.set("stakes.blind", StoryValue::String(r.blind_name.clone()));
            vars.set("stakes.hands", StoryValue::Int(r.hands_left as i64));
            vars.set("stakes.discards", StoryValue::Int(r.discards_left as i64));
        }
        vars.set(
            "style.rules",
            StoryValue::Int(self.stylesheet.rules.len() as i64),
        );
        vars.set(
            "style.keyframes",
            StoryValue::Int(self.stylesheet.keyframes.len() as i64),
        );
        vars.set(
            "style.fns",
            StoryValue::Int(self.stylesheet.script.functions.len() as i64),
        );
        vars.set(
            "market.offers",
            StoryValue::Int(self.market_offers.len() as i64),
        );
        vars.set(
            "market.rerolls",
            StoryValue::Int(self.market_rerolls as i64),
        );
        vars.set(
            "collection.cards",
            StoryValue::Int(self.starter_deck_ids.len() as i64),
        );
    }
}

/// Composite host: style.* + stakes.*.
pub struct StakesHost {
    pub world: Mutex<StakesWorld>,
    pub style: StyleStoryHost,
}

impl StakesHost {
    pub fn new(world: StakesWorld) -> Self {
        Self {
            world: Mutex::new(world),
            style: StyleStoryHost::new(),
        }
    }

    /// Apply a reparsed stylesheet from live-dev (or tools) without restarting.
    pub fn apply_stylesheet(&self, name: &str, sheet: Stylesheet) -> Result<(), String> {
        let mut w = self.world.lock().map_err(|e| e.to_string())?;
        w.stylesheet = sheet.clone();
        let mut reg = self.style.registry.lock().map_err(|e| e.to_string())?;
        reg.insert(name, sheet);
        Ok(())
    }

    fn load_vcss_into(&self, w: &mut StakesWorld, name: &str, src: &str) -> Result<(), String> {
        let sheet = parse_stylesheet(src).map_err(|e| e.to_string())?;
        w.stylesheet = sheet.clone();
        let mut reg = self.style.registry.lock().map_err(|e| e.to_string())?;
        reg.insert(name, sheet);
        Ok(())
    }

    fn resolve_style_path(&self, w: &StakesWorld, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        let candidates = [
            w.data_root.join(path),
            w.data_root.join("styles").join(path),
            PathBuf::from(path),
        ];
        candidates
            .into_iter()
            .find(|c| c.exists())
            .unwrap_or_else(|| w.data_root.join(path))
    }

    fn sync_sheet_from_style_reg(&self, w: &mut StakesWorld) {
        if let Ok(reg) = self.style.registry.lock() {
            if let Some(active) = &reg.active {
                if let Some(s) = reg.sheets.get(active) {
                    w.stylesheet = s.clone();
                }
            } else if let Some((_, s)) = reg.sheets.last() {
                w.stylesheet = s.clone();
            }
        }
    }
}

impl StoryCommandHost for StakesHost {
    fn call(
        &self,
        name: &str,
        args: &IndexMap<String, StoryValue>,
        vars: &mut StoryVariables,
    ) -> Result<CommandOutcome, StoryCommandError> {
        // ── style.* → StyleStoryHost, then mirror sheet into world ──
        if name.starts_with("style.") {
            // Resolve relative paths against data_root
            let mut args = args.clone();
            if let Some(path) = args.get("path").map(|v| v.display_str()) {
                if let Ok(w) = self.world.lock() {
                    let resolved = self.resolve_style_path(&w, &path);
                    args.insert(
                        "path".into(),
                        StoryValue::String(resolved.to_string_lossy().into()),
                    );
                }
            }
            let out = self.style.call(name, &args, vars)?;
            if let Ok(mut w) = self.world.lock() {
                self.sync_sheet_from_style_reg(&mut w);
                w.sync_vars(vars);
            }
            return Ok(out);
        }

        let mut w = self
            .world
            .lock()
            .map_err(|e| StoryCommandError::new(e.to_string()))?;

        match name {
            "stakes.boot" => {
                // Prefer on-disk casino.vcss; fallback embedded.
                const EMBEDDED: &str = include_str!("../data/styles/casino.vcss");
                let path = w.data_root.join("styles/casino.vcss");
                let src = std::fs::read_to_string(&path).unwrap_or_else(|_| EMBEDDED.into());
                self.load_vcss_into(&mut w, "casino", &src)
                    .map_err(StoryCommandError::new)?;
                // Also notify style registry active
                vars.set("style.active", StoryValue::String("casino".into()));
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.set_screen" => {
                let id = arg_str(args, "id").unwrap_or_else(|| "title".into());
                if let Some(s) = Screen::from_id(&id) {
                    w.screen = s;
                }
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.wait" => {
                let phase = arg_str(args, "phase").unwrap_or_else(|| "input".into());
                w.wait_phase = Some(phase.clone());
                w.sync_vars(vars);
                vars.set("stakes.phase", StoryValue::String(phase.clone()));
                Ok(CommandOutcome::Wait { token: phase })
            }
            "stakes.begin_run" => {
                w.begin_run();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.start_blind" => {
                let ante = arg_i64(args, "ante").unwrap_or(0) as usize;
                w.start_blind(ante);
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.advance_blind" => {
                let ante = w.run.as_ref().map(|run| run.ante + 1).unwrap_or(0);
                w.start_blind(ante);
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.enter_play" => {
                w.enter_play();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.open_market" => {
                w.open_market();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.open_collection" => {
                w.open_collection();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.deal" => {
                // Explicit deal from .vcss @script
                let sheet = w.stylesheet.clone();
                let n = w.run.as_ref().map(|r| r.zones.hand.len()).unwrap_or(0) as f32;
                if let Some(r) = w.run.as_mut() {
                    r.rebuild_visuals(true, Some(&sheet));
                }
                if let Ok(run) = call_style_fn(&sheet, "dealHand", &[JsValue::num(n)]) {
                    vars.set("style.actions", StoryValue::Int(run.actions.len() as i64));
                    vars.set(
                        "style.timelines",
                        StoryValue::Int(run.timelines.len() as i64),
                    );
                }
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.emit_style" => {
                let event = arg_str(args, "event").unwrap_or_else(|| "menu.open".into());
                if let Ok(run) = emit_style_event(&w.stylesheet, &event, &[]) {
                    vars.set("style.actions", StoryValue::Int(run.actions.len() as i64));
                }
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.to_title" => {
                w.to_title();
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            "stakes.quit" => {
                w.quit = true;
                w.sync_vars(vars);
                Ok(CommandOutcome::End {
                    ending: Some("quit".into()),
                })
            }
            "stakes.sync" => {
                w.sync_vars(vars);
                Ok(CommandOutcome::Continue)
            }
            _ => {
                // unknown: ignore so story can keep flowing in demos
                Ok(CommandOutcome::Continue)
            }
        }
    }
}

fn arg_str(args: &IndexMap<String, StoryValue>, key: &str) -> Option<String> {
    args.get(key).map(|v| v.display_str())
}

fn arg_i64(args: &IndexMap<String, StoryValue>, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| v.as_i64())
}

/// Register `stakes.*` (and ensure style.*) for the story-lang checker / lowerer.
pub fn register_stakes_commands(reg: &mut CommandRegistry) {
    let stakes = [
        (
            "stakes.boot",
            "Carga .vcss casino y prepara el mundo del demo.",
            vec![],
        ),
        (
            "stakes.set_screen",
            "Cambia pantalla UI (title, play, …).",
            vec![("id", ParamTy::Ident, true)],
        ),
        (
            "stakes.wait",
            "Pausa el .vstory hasta input del jugador (token=phase).",
            vec![("phase", ParamTy::Ident, true)],
        ),
        ("stakes.begin_run", "Empieza una run (ante 0).", vec![]),
        (
            "stakes.start_blind",
            "Prepara ciega por ante.",
            vec![("ante", ParamTy::Int, false)],
        ),
        (
            "stakes.advance_blind",
            "Prepara la siguiente ciega de la run activa.",
            vec![],
        ),
        (
            "stakes.enter_play",
            "Entra a la mesa de juego y aplica deal .vcss.",
            vec![],
        ),
        (
            "stakes.deal",
            "Ejecuta dealHand del @script .vcss sobre la mano.",
            vec![],
        ),
        (
            "stakes.open_market",
            "Abre stock jugable del Night Market.",
            vec![],
        ),
        (
            "stakes.open_collection",
            "Abre la coleccion y el editor del mazo inicial.",
            vec![],
        ),
        (
            "stakes.emit_style",
            "Dispara on(event) del .vcss.",
            vec![("event", ParamTy::Text, true)],
        ),
        ("stakes.to_title", "Vuelve al menú título.", vec![]),
        ("stakes.quit", "Cierra el juego.", vec![]),
        (
            "stakes.sync",
            "Sincroniza variables de historia con el mundo.",
            vec![],
        ),
    ];
    for (name, desc, params) in stakes {
        let params: Vec<CommandParam> = params
            .into_iter()
            .map(|(n, ty, req)| CommandParam {
                name: n.into(),
                ty,
                required: req,
                default: None,
                doc: String::new(),
            })
            .collect();
        let required: Vec<String> = params
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.clone())
            .collect();
        reg.register(CommandSpec {
            name: name.into(),
            category: "stakes".into(),
            description: desc.into(),
            params,
            required,
            snippet: format!("call {name}:\n"),
            error_help: format!("call {name}:"),
        });
    }
    // Note: style.* already in builtin()
    let _ = BLINDS.len(); // keep import used if needed later
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::illustrated_stats;

    fn world() -> StakesWorld {
        let stats = illustrated_stats(Path::new("."))
            .into_iter()
            .map(|card| (card.id.clone(), card))
            .collect();
        let deck_ids = std::iter::repeat("strike".to_string()).take(20).collect();
        StakesWorld::new(stats, deck_ids, PathBuf::from("."))
    }

    #[test]
    fn market_purchase_spends_cash_and_persists_into_next_blind() {
        let mut world = world();
        world.begin_run();
        world.money = 20;
        world.run.as_mut().unwrap().money = 20;
        world.open_market();
        let bought_id = world.market_offers[0].card_id.clone();
        let price = world.market_offers[0].price;

        assert!(world.buy_market_offer(0));
        assert_eq!(world.money, 20 - price);
        assert!(world.market_offers[0].bought);
        assert_eq!(world.deck_ids.last(), Some(&bought_id));

        world.start_blind(1);
        let zones = &world.run.as_ref().unwrap().zones;
        let run_card_count = zones.library.len() + zones.hand.len() + zones.discard.len();
        assert_eq!(run_card_count, 21);
    }

    #[test]
    fn market_reroll_costs_cash_and_changes_stock() {
        let mut world = world();
        world.begin_run();
        world.money = 20;
        world.run.as_mut().unwrap().money = 20;
        world.open_market();
        let before = world.market_offers.clone();

        assert_eq!(world.market_reroll_cost(), 1);
        assert!(world.reroll_market());
        assert_eq!(world.money, 19);
        assert_eq!(world.market_reroll_cost(), 2);
        assert_ne!(world.market_offers, before);
    }

    #[test]
    fn new_run_restores_starter_deck_after_market_cards() {
        let mut world = world();
        world.begin_run();
        world.money = 20;
        world.run.as_mut().unwrap().money = 20;
        world.open_market();
        assert!(world.buy_market_offer(0));
        assert_eq!(world.deck_ids.len(), 21);

        world.to_title();
        world.begin_run();
        assert_eq!(world.deck_ids.len(), 20);
    }

    #[test]
    fn collection_edits_persist_into_the_next_run() {
        let mut world = world();
        world.open_collection();
        assert_eq!(world.screen, Screen::Collection);
        assert!(world.add_collection_card("fireball"));
        assert_eq!(world.starter_deck_ids.len(), 21);
        assert!(world.remove_collection_card("strike"));
        assert_eq!(world.starter_deck_ids.len(), 20);

        world.begin_run();
        let zones = &world.run.as_ref().unwrap().zones;
        let run_card_count = zones.library.len() + zones.hand.len() + zones.discard.len();
        assert_eq!(run_card_count, 20);
        assert!(world.deck_ids.iter().any(|id| id == "fireball"));
    }

    #[test]
    fn collection_enforces_safe_deck_limits() {
        let mut world = world();
        world.starter_deck_ids = std::iter::repeat("strike".to_string())
            .take(COLLECTION_DECK_MIN)
            .collect();
        world.deck_ids = world.starter_deck_ids.clone();
        assert!(!world.remove_collection_card("strike"));

        world.starter_deck_ids = std::iter::repeat("strike".to_string())
            .take(COLLECTION_DECK_MAX)
            .collect();
        world.deck_ids = world.starter_deck_ids.clone();
        assert!(!world.add_collection_card("guard"));
    }
}
