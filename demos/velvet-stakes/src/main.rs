//! Velvet Stakes — Balatro-like poker demo (pre-alpha).
//!
//! Play poker hands for **chips × mult**, beat the **blind** target score.
//! Deck/hand/discard via `velvet-cards` zones. Softbuffer window + menus.
//!
//! Not Balatro / not affiliated — inspired mechanics for engine demo only.
//!
//! Controls:
//!   Menus: ↑↓ W/S · Enter · Esc  
//!   Play: 1–8 toggle select · P play hand · D discard · Esc pause  
//!   `--headless`: auto-play smoke

mod poker;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use softbuffer::{Context as SbContext, Surface};
use velvet_cards::{shuffle_in_place, CardZones, DeckList};
use velvet_story::{draw_text_line, pack_rgb};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use poker::{evaluate_hand, standard_deck_ids, PlayingCard};

const WW: u32 = 960;
const WH: u32 = 540;
const HAND_SIZE: usize = 8;
const MAX_SELECT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    HowTo,
    BlindInfo,
    Play,
    Pause,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    WinBlind,
    LoseBlind,
    RunClear,
}

struct BlindDef {
    name: &'static str,
    target: i64,
    hands: u32,
    discards: u32,
}

const BLINDS: &[BlindDef] = &[
    BlindDef {
        name: "Small Blind",
        target: 300,
        hands: 4,
        discards: 3,
    },
    BlindDef {
        name: "Big Blind",
        target: 800,
        hands: 4,
        discards: 3,
    },
    BlindDef {
        name: "Boss Blind",
        target: 1600,
        hands: 4,
        discards: 2,
    },
];

struct Run {
    ante_index: usize,
    zones: CardZones,
    /// Indices into hand that are selected.
    selected: Vec<bool>,
    score: i64,
    hands_left: u32,
    discards_left: u32,
    target: i64,
    blind_name: String,
    log: Vec<String>,
    last_hand: String,
    seed: u64,
    money: i64,
}

impl Run {
    fn start_blind(ante_index: usize, seed: u64, money: i64) -> Self {
        let blind = &BLINDS[ante_index.min(BLINDS.len() - 1)];
        let mut ids = standard_deck_ids();
        shuffle_in_place(&mut ids, seed);
        let deck = DeckList::from_ids(ids);
        let mut zones = CardZones::from_deck_list(&deck);
        // already shuffled ids order = library bottom→top; top is last
        let _ = zones.draw(HAND_SIZE);
        let mut selected = vec![false; zones.hand.len()];
        selected.resize(zones.hand.len(), false);
        Self {
            ante_index,
            zones,
            selected,
            score: 0,
            hands_left: blind.hands,
            discards_left: blind.discards,
            target: blind.target,
            blind_name: blind.name.into(),
            log: vec![format!(
                "{} — target {} chips",
                blind.name, blind.target
            )],
            last_hand: String::new(),
            seed,
            money,
        }
    }

    fn push_log(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 7 {
            let n = self.log.len() - 7;
            self.log.drain(0..n);
        }
    }

    fn hand_cards(&self) -> Vec<PlayingCard> {
        self.zones
            .hand
            .iter()
            .filter_map(|id| PlayingCard::parse(id))
            .collect()
    }

    fn selected_cards(&self) -> Vec<PlayingCard> {
        self.zones
            .hand
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected.get(*i).copied().unwrap_or(false))
            .filter_map(|(_, id)| PlayingCard::parse(id))
            .collect()
    }

    fn toggle_select(&mut self, index: usize) {
        if index >= self.zones.hand.len() {
            return;
        }
        if self.selected.len() != self.zones.hand.len() {
            self.selected.resize(self.zones.hand.len(), false);
        }
        if self.selected[index] {
            self.selected[index] = false;
            return;
        }
        let count = self.selected.iter().filter(|s| **s).count();
        if count >= MAX_SELECT {
            self.push_log(format!("Max {MAX_SELECT} cards"));
            return;
        }
        self.selected[index] = true;
    }

    fn clear_selection(&mut self) {
        for s in &mut self.selected {
            *s = false;
        }
    }

    fn refill_hand(&mut self) {
        while self.zones.hand.len() < HAND_SIZE {
            if self.zones.library.is_empty() {
                if self.zones.discard.is_empty() {
                    break;
                }
                self.zones.library.append(&mut self.zones.discard);
                shuffle_in_place(
                    &mut self.zones.library,
                    self.seed.wrapping_add(self.hands_left as u64 + 99),
                );
                self.push_log("Shuffled discard → deck");
            }
            if self.zones.draw(1).is_err() {
                break;
            }
        }
        self.selected.resize(self.zones.hand.len(), false);
        self.clear_selection();
    }

    /// Play selected cards. Returns true if blind resolved this action.
    fn play_selected(&mut self) -> Option<Outcome> {
        let sel: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();
        if sel.is_empty() {
            self.push_log("Select 1–5 cards first");
            return None;
        }
        if self.hands_left == 0 {
            self.push_log("No hands left");
            return None;
        }

        let cards = self.selected_cards();
        let score = evaluate_hand(&cards);
        self.score += score.total;
        self.hands_left -= 1;
        self.last_hand = format!(
            "{}  {}×{} = +{}",
            score.kind.name(),
            score.chips,
            score.mult,
            score.total
        );
        self.push_log(self.last_hand.clone());

        // Remove selected from hand → discard (high index first)
        let mut idxs = sel;
        idxs.sort_unstable();
        for i in idxs.into_iter().rev() {
            let _ = self.zones.discard_from_hand(i);
        }
        self.refill_hand();

        if self.score >= self.target {
            self.money += 5 + self.ante_index as i64 * 3;
            return Some(if self.ante_index + 1 >= BLINDS.len() {
                Outcome::RunClear
            } else {
                Outcome::WinBlind
            });
        }
        if self.hands_left == 0 {
            return Some(Outcome::LoseBlind);
        }
        None
    }

    fn discard_selected(&mut self) {
        if self.discards_left == 0 {
            self.push_log("No discards left");
            return;
        }
        let sel: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();
        if sel.is_empty() {
            self.push_log("Select cards to discard");
            return;
        }
        self.discards_left -= 1;
        let mut idxs = sel;
        idxs.sort_unstable();
        let n = idxs.len();
        for i in idxs.into_iter().rev() {
            let _ = self.zones.discard_from_hand(i);
        }
        self.push_log(format!("Discarded {n} · left {}", self.discards_left));
        self.refill_hand();
    }
}

struct App {
    screen: Screen,
    menu_sel: usize,
    pause_sel: usize,
    result_sel: usize,
    result: Option<Outcome>,
    run: Option<Run>,
    status: String,
    window: Option<Arc<Window>>,
    context: Option<SbContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    pixels: Vec<u32>,
    money: i64,
    next_seed: u64,
}

impl App {
    fn new(headless: bool) -> Self {
        Self {
            screen: Screen::Title,
            menu_sel: 0,
            pause_sel: 0,
            result_sel: 0,
            result: None,
            run: None,
            status: "Velvet Stakes".into(),
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            pixels: vec![0; (WW * WH) as usize],
            money: 0,
            next_seed: 0xBA_1A_70_42,
        }
    }

    fn title_items() -> &'static [&'static str] {
        &["Nueva run", "Cómo jugar", "Salir"]
    }

    fn pause_items() -> &'static [&'static str] {
        &["Continuar", "Menú principal"]
    }

    fn result_items_win() -> &'static [&'static str] {
        &["Siguiente ciega", "Menú principal"]
    }

    fn result_items_lose() -> &'static [&'static str] {
        &["Reintentar run", "Menú principal"]
    }

    fn result_items_clear() -> &'static [&'static str] {
        &["Nueva run", "Menú principal"]
    }

    fn begin_run(&mut self) {
        self.money = 4;
        self.next_seed = self.next_seed.wrapping_add(17);
        self.start_blind(0);
    }

    fn start_blind(&mut self, ante: usize) {
        self.next_seed = self.next_seed.wrapping_add(1);
        let run = Run::start_blind(ante, self.next_seed, self.money);
        self.status = format!("{} / target {}", run.blind_name, run.target);
        self.run = Some(run);
        self.screen = Screen::BlindInfo;
        self.result = None;
    }

    fn enter_play(&mut self) {
        self.screen = Screen::Play;
        self.status = "Select cards · P play · D discard".into();
    }

    fn apply_outcome(&mut self, o: Outcome) {
        self.result = Some(o);
        self.screen = Screen::Result;
        self.result_sel = 0;
        if let Some(r) = &self.run {
            self.money = r.money;
        }
        self.status = match o {
            Outcome::WinBlind => "¡Ciega superada!".into(),
            Outcome::LoseBlind => "Ciega fallida…".into(),
            Outcome::RunClear => "¡Run completa!".into(),
        };
    }

    fn confirm_title(&mut self, el: &ActiveEventLoop) {
        match self.menu_sel {
            0 => self.begin_run(),
            1 => {
                self.screen = Screen::HowTo;
            }
            2 => el.exit(),
            _ => {}
        }
    }

    fn confirm_result(&mut self) {
        match self.result {
            Some(Outcome::WinBlind) => {
                if self.result_sel == 0 {
                    let next = self.run.as_ref().map(|r| r.ante_index + 1).unwrap_or(1);
                    self.start_blind(next);
                } else {
                    self.to_title();
                }
            }
            Some(Outcome::LoseBlind) | Some(Outcome::RunClear) => {
                if self.result_sel == 0 {
                    self.begin_run();
                } else {
                    self.to_title();
                }
            }
            None => self.to_title(),
        }
    }

    fn to_title(&mut self) {
        self.run = None;
        self.result = None;
        self.screen = Screen::Title;
        self.menu_sel = 0;
        self.status = "Menú".into();
    }

    // --- paint helpers ---

    fn fill_bg(&mut self, rgb: (u8, u8, u8)) {
        let c = pack_rgb(rgb.0, rgb.1, rgb.2);
        for p in &mut self.pixels {
            *p = c;
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, rgb: (u8, u8, u8)) {
        let c = pack_rgb(rgb.0, rgb.1, rgb.2);
        let ww = WW as i32;
        let wh = WH as i32;
        for row in y.max(0)..(y + h).min(wh) {
            for col in x.max(0)..(x + w).min(ww) {
                self.pixels[(row as u32 * WW + col as u32) as usize] = c;
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, rgb: (u8, u8, u8), scale: i32) {
        draw_text_line(
            &mut self.pixels,
            WW,
            WH,
            x,
            y,
            s,
            pack_rgb(rgb.0, rgb.1, rgb.2),
            scale,
        );
    }

    fn paint_title(&mut self) {
        self.fill_bg((12, 18, 14));
        self.rect(0, 0, WW as i32, 6, (220, 170, 40));
        self.text(40, 40, "VELVET STAKES", (255, 210, 80), 3);
        self.text(
            40,
            90,
            "Balatro-like poker · chips x mult · blinds",
            (180, 200, 170),
            1,
        );
        let items = Self::title_items();
        for (i, item) in items.iter().enumerate() {
            let y = 180 + (i as i32) * 44;
            let sel = i == self.menu_sel;
            if sel {
                self.rect(36, y - 6, 400, 34, (40, 70, 45));
            }
            let p = if sel { "> " } else { "  " };
            self.text(
                48,
                y,
                &format!("{p}{item}"),
                if sel {
                    (255, 240, 180)
                } else {
                    (200, 210, 200)
                },
                2,
            );
        }
        self.text(
            40,
            (WH as i32) - 36,
            "Inspired by Balatro (fan demo, not affiliated)",
            (120, 130, 120),
            1,
        );
    }

    fn paint_howto(&mut self) {
        self.fill_bg((14, 16, 20));
        self.text(40, 30, "COMO JUGAR", (255, 210, 100), 3);
        let lines = [
            "Elige hasta 5 cartas (teclas 1-8) y pulsa P para jugar la mano.",
            "La mano de poker da CHIPS x MULT (ej. Pair 10x2 + valores).",
            "Suma puntos hasta el TARGET de la ciega (blind).",
            "D = descartar seleccion (usos limitados).",
            "Se acaban las manos sin llegar al target = pierdes la ciega.",
            "Gana Small, Big y Boss Blind para completar la run.",
            "",
            "Enter / Esc = volver",
        ];
        for (i, l) in lines.iter().enumerate() {
            self.text(40, 100 + (i as i32) * 28, l, (210, 215, 220), 1);
        }
    }

    fn paint_blind_info(&mut self) {
        self.fill_bg((16, 14, 28));
        let (name, target, hands, disc, money) = self
            .run
            .as_ref()
            .map(|r| {
                (
                    r.blind_name.clone(),
                    r.target,
                    r.hands_left,
                    r.discards_left,
                    r.money,
                )
            })
            .unwrap_or_else(|| ("?".into(), 0, 0, 0, 0));
        self.text(40, 60, "SIGUIENTE CIEGA", (200, 180, 255), 2);
        self.text(40, 120, &name, (255, 220, 120), 3);
        self.text(40, 180, &format!("Target: {target} chips"), (220, 220, 230), 2);
        self.text(
            40,
            220,
            &format!("Hands: {hands}  ·  Discards: {disc}  ·  $ {money}"),
            (180, 190, 210),
            1,
        );
        self.text(40, 320, "Enter = jugar ciega", (255, 240, 200), 2);
        self.text(40, 360, "Esc = menu", (150, 150, 160), 1);
    }

    fn paint_play(&mut self) {
        self.fill_bg((18, 22, 20));
        self.rect(0, 0, WW as i32, 5, (220, 170, 40));

        let (
            score,
            target,
            hands,
            disc,
            money,
            blind,
            hand,
            selected,
            lib,
            disc_n,
            log,
            last,
            preview,
        ) = {
            let r = self.run.as_ref().unwrap();
            let cards = r.selected_cards();
            let preview = if cards.is_empty() {
                "—".into()
            } else {
                let s = evaluate_hand(&cards);
                format!(
                    "{}  {}x{} = {}",
                    s.kind.name(),
                    s.chips,
                    s.mult,
                    s.total
                )
            };
            (
                r.score,
                r.target,
                r.hands_left,
                r.discards_left,
                r.money,
                r.blind_name.clone(),
                r.zones.hand.clone(),
                r.selected.clone(),
                r.zones.library.len(),
                r.zones.discard.len(),
                r.log.clone(),
                r.last_hand.clone(),
                preview,
            )
        };

        self.text(20, 12, "VELVET STAKES", (255, 210, 80), 1);
        self.text(200, 12, &blind, (200, 190, 255), 1);
        self.text(
            20,
            40,
            &format!("SCORE {score} / {target}"),
            (120, 255, 160),
            2,
        );
        // progress bar
        let bar_w = 400;
        self.rect(20, 72, bar_w, 12, (40, 50, 45));
        let fill = ((score as f32 / target.max(1) as f32).min(1.0) * bar_w as f32) as i32;
        self.rect(20, 72, fill.max(0), 12, (80, 200, 100));

        self.text(
            20,
            96,
            &format!("Hands {hands}  Disc {disc}  $ {money}  Deck {lib}  Discard {disc_n}"),
            (180, 190, 180),
            1,
        );
        self.text(20, 120, &format!("Preview: {preview}"), (255, 230, 140), 1);
        if !last.is_empty() {
            self.text(20, 142, &format!("Last: {last}"), (160, 200, 255), 1);
        }

        // Hand
        for (i, id) in hand.iter().enumerate() {
            let x = 16 + (i as i32) * 116;
            let y = 180;
            let sel = selected.get(i).copied().unwrap_or(false);
            let bg = if sel {
                (90, 70, 30)
            } else {
                (40, 50, 48)
            };
            self.rect(x, y, 108, 150, bg);
            self.rect(x + 2, y + 2, 104, 146, (50, 62, 58));
            let label = PlayingCard::parse(id)
                .map(|c| c.short())
                .unwrap_or_else(|| id.clone());
            let suit_col = match label.chars().last() {
                Some('H') | Some('D') => (255, 120, 120),
                _ => (200, 210, 255),
            };
            self.text(x + 12, y + 20, &format!("[{}]", i + 1), (160, 160, 160), 1);
            self.text(x + 20, y + 60, &label, suit_col, 3);
            if sel {
                self.text(x + 20, y + 120, "SEL", (255, 220, 80), 1);
            }
        }

        self.text(20, 350, "Log", (150, 160, 150), 1);
        for (i, line) in log.iter().enumerate() {
            self.text(20, 370 + (i as i32) * 16, line, (140, 150, 145), 1);
        }
        self.text(
            20,
            (WH as i32) - 28,
            "1-8 select · P play · D discard · Esc pause",
            (130, 140, 130),
            1,
        );
    }

    fn paint_pause(&mut self) {
        self.paint_play();
        for p in &mut self.pixels {
            let a = *p;
            let r = ((a >> 16) & 0xFF) as u8 / 3;
            let g = ((a >> 8) & 0xFF) as u8 / 3;
            let b = (a & 0xFF) as u8 / 3;
            *p = pack_rgb(r, g, b);
        }
        self.rect(250, 140, 460, 220, (28, 36, 32));
        self.text(300, 170, "PAUSA", (255, 210, 80), 3);
        for (i, item) in Self::pause_items().iter().enumerate() {
            let y = 240 + (i as i32) * 40;
            let sel = i == self.pause_sel;
            if sel {
                self.rect(290, y - 4, 360, 30, (50, 80, 55));
            }
            let p = if sel { "> " } else { "  " };
            self.text(310, y, &format!("{p}{item}"), (230, 240, 230), 2);
        }
    }

    fn paint_result(&mut self) {
        self.fill_bg((14, 16, 18));
        let (title, col, items) = match self.result {
            Some(Outcome::WinBlind) => (
                "¡CIEGA SUPERADA!",
                (120, 255, 150),
                Self::result_items_win(),
            ),
            Some(Outcome::LoseBlind) => (
                "CIEGA FALLIDA",
                (255, 120, 120),
                Self::result_items_lose(),
            ),
            Some(Outcome::RunClear) => (
                "¡RUN COMPLETA!",
                (255, 220, 80),
                Self::result_items_clear(),
            ),
            None => ("FIN", (200, 200, 200), Self::result_items_lose()),
        };
        self.text(40, 70, title, col, 3);
        let (score_line, last_line) = self
            .run
            .as_ref()
            .map(|r| {
                (
                    format!(
                        "Score final {} / {}  ·  $ {}",
                        r.score, r.target, r.money
                    ),
                    r.last_hand.clone(),
                )
            })
            .unwrap_or_default();
        if !score_line.is_empty() {
            self.text(40, 140, &score_line, (200, 210, 200), 1);
        }
        if !last_line.is_empty() {
            self.text(
                40,
                170,
                &format!("Ultima: {last_line}"),
                (160, 180, 200),
                1,
            );
        }
        for (i, item) in items.iter().enumerate() {
            let y = 240 + (i as i32) * 44;
            let sel = i == self.result_sel;
            if sel {
                self.rect(36, y - 6, 420, 34, (45, 60, 50));
            }
            let p = if sel { "> " } else { "  " };
            self.text(48, y, &format!("{p}{item}"), (240, 240, 230), 2);
        }
    }

    fn paint(&mut self) {
        match self.screen {
            Screen::Title => self.paint_title(),
            Screen::HowTo => self.paint_howto(),
            Screen::BlindInfo => self.paint_blind_info(),
            Screen::Play => self.paint_play(),
            Screen::Pause => self.paint_pause(),
            Screen::Result => self.paint_result(),
        }

        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let ww = size.width.max(1);
        let wh = size.height.max(1);
        let present = if ww != WW || wh != WH {
            scale_nearest(&self.pixels, WW, WH, ww, wh)
        } else {
            self.pixels.clone()
        };
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(NonZeroU32::new(ww).unwrap(), NonZeroU32::new(wh).unwrap());
        if let Ok(mut buf) = surface.buffer_mut() {
            let n = present.len().min(buf.len());
            buf[..n].copy_from_slice(&present[..n]);
            let _ = buf.present();
        }
        window.set_title(&format!("Velvet Stakes — {:?}", self.screen));
    }

    fn auto_tick(&mut self, el: &ActiveEventLoop) {
        match self.screen {
            Screen::Title if self.hframes == 2 => {
                self.begin_run();
            }
            Screen::BlindInfo if self.hframes >= 4 => {
                self.enter_play();
            }
            Screen::Play => {
                // Auto: select first 2 cards if possible and play; else discard 1; else play high card
                if let Some(r) = self.run.as_mut() {
                    if r.selected.iter().all(|s| !s) && !r.zones.hand.is_empty() {
                        // pick pair if any
                        let hand = r.hand_cards();
                        let mut picked = false;
                        for i in 0..hand.len() {
                            for j in (i + 1)..hand.len() {
                                if hand[i].rank == hand[j].rank {
                                    r.selected[i] = true;
                                    r.selected[j] = true;
                                    picked = true;
                                    break;
                                }
                            }
                            if picked {
                                break;
                            }
                        }
                        if !picked {
                            r.selected[0] = true;
                        }
                    }
                }
                if let Some(r) = self.run.as_ref() {
                    let any = r.selected.iter().any(|s| *s);
                    if any {
                        if let Some(o) = self.run.as_mut().and_then(|r| r.play_selected()) {
                            self.apply_outcome(o);
                        }
                    }
                }
            }
            Screen::Result if self.hframes > 30 => {
                println!(
                    "headless result={:?} money={} frames={}",
                    self.result, self.money, self.hframes
                );
                el.exit();
            }
            _ => {}
        }
        if self.hframes > 500 {
            println!("headless timeout {:?}", self.screen);
            el.exit();
        }
    }

    fn on_key(&mut self, c: KeyCode, el: &ActiveEventLoop) {
        match self.screen {
            Screen::Title => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.menu_sel = self.menu_sel.saturating_sub(1);
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    if self.menu_sel + 1 < Self::title_items().len() {
                        self.menu_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space => self.confirm_title(el),
                KeyCode::Escape => el.exit(),
                _ => {}
            },
            Screen::HowTo => {
                if matches!(
                    c,
                    KeyCode::Enter | KeyCode::Space | KeyCode::Escape
                ) {
                    self.screen = Screen::Title;
                }
            }
            Screen::BlindInfo => match c {
                KeyCode::Enter | KeyCode::Space => self.enter_play(),
                KeyCode::Escape => self.to_title(),
                _ => {}
            },
            Screen::Play => match c {
                KeyCode::Digit1 | KeyCode::Numpad1 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(0);
                    }
                }
                KeyCode::Digit2 | KeyCode::Numpad2 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(1);
                    }
                }
                KeyCode::Digit3 | KeyCode::Numpad3 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(2);
                    }
                }
                KeyCode::Digit4 | KeyCode::Numpad4 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(3);
                    }
                }
                KeyCode::Digit5 | KeyCode::Numpad5 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(4);
                    }
                }
                KeyCode::Digit6 | KeyCode::Numpad6 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(5);
                    }
                }
                KeyCode::Digit7 | KeyCode::Numpad7 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(6);
                    }
                }
                KeyCode::Digit8 | KeyCode::Numpad8 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle_select(7);
                    }
                }
                KeyCode::KeyP | KeyCode::Enter => {
                    if let Some(o) = self.run.as_mut().and_then(|r| r.play_selected()) {
                        self.apply_outcome(o);
                    }
                }
                KeyCode::KeyD => {
                    if let Some(r) = self.run.as_mut() {
                        r.discard_selected();
                    }
                }
                KeyCode::Escape => {
                    self.screen = Screen::Pause;
                    self.pause_sel = 0;
                }
                _ => {}
            },
            Screen::Pause => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.pause_sel = self.pause_sel.saturating_sub(1);
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    if self.pause_sel + 1 < Self::pause_items().len() {
                        self.pause_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space => match self.pause_sel {
                    0 => self.screen = Screen::Play,
                    1 => self.to_title(),
                    _ => {}
                },
                KeyCode::Escape => self.screen = Screen::Play,
                _ => {}
            },
            Screen::Result => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.result_sel = self.result_sel.saturating_sub(1);
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    let n = match self.result {
                        Some(Outcome::WinBlind) => Self::result_items_win().len(),
                        Some(Outcome::RunClear) => Self::result_items_clear().len(),
                        _ => Self::result_items_lose().len(),
                    };
                    if self.result_sel + 1 < n {
                        self.result_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space => self.confirm_result(),
                KeyCode::Escape => self.to_title(),
                _ => {}
            },
        }
    }
}

fn scale_nearest(src: &[u32], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u32> {
    let mut out = vec![0u32; (dw * dh) as usize];
    for y in 0..dh {
        let sy = y * sh / dh;
        for x in 0..dw {
            let sx = x * sw / dw;
            out[(y * dw + x) as usize] = src[(sy * sw + sx) as usize];
        }
    }
    out
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("Velvet Stakes — Balatro-like")
            .with_inner_size(LogicalSize::new(WW, WH));
        let window = Arc::new(el.create_window(attrs).expect("window"));
        let context = SbContext::new(window.clone()).expect("ctx");
        let surface = Surface::new(&context, window.clone()).expect("surface");
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
        self.last = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::RedrawRequested => self.paint(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let PhysicalKey::Code(c) = event.physical_key else {
                    return;
                };
                self.on_key(c, el);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let now = Instant::now();
        if now.duration_since(self.last) < Duration::from_millis(16) {
            el.set_control_flow(ControlFlow::WaitUntil(
                self.last + Duration::from_millis(16),
            ));
            return;
        }
        self.last = now;
        self.hframes += 1;
        if self.headless {
            self.auto_tick(el);
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        el.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("velvet_stakes=info,info");
    let headless = std::env::args().any(|a| a == "--headless");
    let mut app = App::new(headless);
    println!("Velvet Stakes — Balatro-like poker demo (fan, not affiliated)");
    if headless {
        println!("headless smoke…");
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}
