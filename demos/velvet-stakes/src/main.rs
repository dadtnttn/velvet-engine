//! Velvet Arcana / Stakes — **local** Balatro-style casino demo.
//!
//! Title menu styled after Nightfall Casino art. Illustrated cards + chips×mult
//! blinds + deal animation. Commits stay local unless you push.
//!
//! Controls: menus ↑↓ Enter · Play: 1–8 select · P play · D discard · Esc  
//! `--headless`: auto smoke

mod catalog;
mod render;
mod ui;

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use softbuffer::{Context as SbContext, Surface};
use velvet_anim::{ChannelTrack, Pose3D, Pose3DChannel, Timeline};
use velvet_cards::{shuffle_in_place, validate_deck, CardZones, DeckRules};
use velvet_math::{Ease, Vec2};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use catalog::{make_catalog_and_deck, score_played, CardStats, HandScore};
use render::{blit_card, fill, load_rgb, rect, text, ArtBank, RgbImage};
use ui::theme::{Theme, TITLE_ITEMS, WW, WH};
use ui::{paint_collection, paint_options, paint_shop, paint_title_menu};
use velvet_style::{parse_stylesheet, Stylesheet};

const HAND_SIZE: usize = 8;
const MAX_SELECT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    Collection,
    Shop,
    Options,
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
        target: 250,
        hands: 4,
        discards: 3,
    },
    BlindDef {
        name: "Big Blind",
        target: 700,
        hands: 4,
        discards: 3,
    },
    BlindDef {
        name: "Boss Blind",
        target: 1400,
        hands: 4,
        discards: 2,
    },
];

/// Visual slot for a hand card (animated).
struct CardVisual {
    id: String,
    /// Rest pose in hand.
    rest: Vec2,
    pose: Pose3D,
    timeline: Timeline,
}

struct Run {
    zones: CardZones,
    selected: Vec<bool>,
    score: i64,
    hands_left: u32,
    discards_left: u32,
    target: i64,
    blind_name: String,
    ante: usize,
    log: Vec<String>,
    last: String,
    money: i64,
    visuals: Vec<CardVisual>,
    deal_t: f32,
}

impl Run {
    fn start(ante: usize, deck_cards: &[String], seed: u64, money: i64) -> Self {
        let blind = &BLINDS[ante.min(BLINDS.len() - 1)];
        let mut ids = deck_cards.to_vec();
        shuffle_in_place(&mut ids, seed);
        let mut zones = CardZones {
            library: ids,
            hand: Vec::new(),
            discard: Vec::new(),
        };
        let _ = zones.draw(HAND_SIZE.min(zones.library.len()));
        let mut run = Self {
            zones,
            selected: vec![false; HAND_SIZE],
            score: 0,
            hands_left: blind.hands,
            discards_left: blind.discards,
            target: blind.target,
            blind_name: blind.name.into(),
            ante,
            log: vec![format!("{} — target {}", blind.name, blind.target)],
            last: String::new(),
            money,
            visuals: Vec::new(),
            deal_t: 0.0,
        };
        run.rebuild_visuals(true);
        run
    }

    fn hand_slot_pos(i: usize, n: usize) -> Vec2 {
        let n = n.max(1) as f32;
        let total_w = (n - 1.0) * 108.0;
        let x0 = (WW as f32 - total_w) * 0.5 - 40.0;
        Vec2::new(x0 + i as f32 * 108.0, 200.0)
    }

    fn rebuild_visuals(&mut self, animate_deal: bool) {
        let n = self.zones.hand.len();
        self.selected.resize(n, false);
        self.visuals.clear();
        for (i, id) in self.zones.hand.iter().enumerate() {
            let rest = Self::hand_slot_pos(i, n);
            let mut pose = Pose3D::flat(rest);
            let mut timeline = Timeline::new();
            if animate_deal {
                // deal from pack center
                let from = Vec2::new(WW as f32 * 0.5, WH as f32 * 0.35);
                pose.pos = from;
                pose.opacity = 0.0;
                pose.yaw = 0.8;
                pose.scale = 0.7;
                let delay = i as f32 * 0.06;
                timeline = Timeline::new()
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::X)
                            .key(delay, from.x, Ease::Linear)
                            .key(delay + 0.28, rest.x, Ease::CubicOut),
                    )
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::Y)
                            .key(delay, from.y, Ease::Linear)
                            .key(delay + 0.28, rest.y, Ease::BackOut),
                    )
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::Opacity)
                            .key(delay, 0.0, Ease::Linear)
                            .key(delay + 0.15, 1.0, Ease::QuadOut),
                    )
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::Yaw)
                            .key(delay, 0.9, Ease::Linear)
                            .key(delay + 0.28, 0.0, Ease::CubicOut),
                    )
                    .with_channel(
                        ChannelTrack::new(Pose3DChannel::Scale)
                            .key(delay, 0.65, Ease::Linear)
                            .key(delay + 0.28, 1.0, Ease::BackOut),
                    );
            } else {
                pose.opacity = 1.0;
            }
            self.visuals.push(CardVisual {
                id: id.clone(),
                rest,
                pose,
                timeline,
            });
        }
        self.deal_t = 0.0;
    }

    fn tick_anims(&mut self, dt: f32) {
        self.deal_t += dt;
        for v in &mut self.visuals {
            if v.timeline.playing || !v.timeline.finished() {
                v.timeline.tick(dt);
                v.timeline.apply(&mut v.pose);
            }
        }
        // selection offset by index
        for (i, v) in self.visuals.iter_mut().enumerate() {
            if self.selected.get(i).copied().unwrap_or(false) {
                if v.timeline.finished() || !v.timeline.playing {
                    v.pose.pos.y = v.rest.y - 18.0;
                    v.pose.scale = 1.06;
                }
            } else if v.timeline.finished() || !v.timeline.playing {
                v.pose.pos = v.rest;
                v.pose.scale = 1.0;
                v.pose.opacity = 1.0;
            }
        }
    }

    fn push_log(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 6 {
            let n = self.log.len() - 6;
            self.log.drain(0..n);
        }
    }

    fn toggle(&mut self, i: usize) {
        if i >= self.zones.hand.len() {
            return;
        }
        if self.selected[i] {
            self.selected[i] = false;
            return;
        }
        if self.selected.iter().filter(|s| **s).count() >= MAX_SELECT {
            self.push_log(format!("Max {MAX_SELECT} cards"));
            return;
        }
        self.selected[i] = true;
    }

    fn selected_ids(&self) -> Vec<String> {
        self.zones
            .hand
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected.get(*i).copied().unwrap_or(false))
            .map(|(_, id)| id.clone())
            .collect()
    }

    fn preview_score(&self, stats: &HashMap<String, CardStats>) -> HandScore {
        score_played(&self.selected_ids(), stats)
    }

    fn play_selected(&mut self, stats: &HashMap<String, CardStats>) -> Option<Outcome> {
        let ids = self.selected_ids();
        if ids.is_empty() {
            self.push_log("Select 1–5 cards");
            return None;
        }
        if self.hands_left == 0 {
            return None;
        }
        let sc = score_played(&ids, stats);
        self.score += sc.total;
        self.hands_left -= 1;
        self.last = format!("{}  {}×{} = +{}", sc.label, sc.chips, sc.mult, sc.total);
        self.push_log(self.last.clone());

        // focus skill: draw 1 after play
        let focus_n = ids.iter().filter(|id| id.as_str() == "focus").count();

        let mut idxs: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();
        idxs.sort_unstable();
        for i in idxs.into_iter().rev() {
            let _ = self.zones.discard_from_hand(i);
        }
        self.refill(focus_n);
        self.rebuild_visuals(true);

        if self.score >= self.target {
            self.money += 4 + self.ante as i64 * 2;
            return Some(if self.ante + 1 >= BLINDS.len() {
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
            self.push_log("No discards");
            return;
        }
        let mut idxs: Vec<usize> = self
            .selected
            .iter()
            .enumerate()
            .filter(|(_, s)| **s)
            .map(|(i, _)| i)
            .collect();
        if idxs.is_empty() {
            self.push_log("Select to discard");
            return;
        }
        self.discards_left -= 1;
        idxs.sort_unstable();
        let n = idxs.len();
        for i in idxs.into_iter().rev() {
            let _ = self.zones.discard_from_hand(i);
        }
        self.push_log(format!("Discarded {n}"));
        self.refill(0);
        self.rebuild_visuals(true);
    }

    fn refill(&mut self, bonus_draw: usize) {
        let need = HAND_SIZE.saturating_sub(self.zones.hand.len()) + bonus_draw;
        for _ in 0..need {
            if self.zones.library.is_empty() {
                if self.zones.discard.is_empty() {
                    break;
                }
                self.zones.library.append(&mut self.zones.discard);
                shuffle_in_place(&mut self.zones.library, 0xDEC_A_DE + self.hands_left as u64);
                self.push_log("Shuffled discard");
            }
            let _ = self.zones.draw(1);
        }
        self.selected = vec![false; self.zones.hand.len()];
    }
}

struct App {
    screen: Screen,
    menu_sel: usize,
    pause_sel: usize,
    result_sel: usize,
    result: Option<Outcome>,
    run: Option<Run>,
    stats: HashMap<String, CardStats>,
    deck_ids: Vec<String>,
    art: ArtBank,
    /// Original generated lobby background (not the user reference file).
    menu_bg: Option<RgbImage>,
    logo_emblem: Option<RgbImage>,
    /// CSS-like styles for lobby UI.
    stylesheet: Stylesheet,
    theme: Theme,
    /// Meta stats shown on title HUD (flavor / progress).
    meta_chips: i64,
    meta_crystals: i64,
    meta_mult: f32,
    money: i64,
    seed: u64,
    status: String,
    window: Option<Arc<Window>>,
    context: Option<SbContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    pixels: Vec<u32>,
}

fn load_casino_stylesheet() -> Stylesheet {
    const EMBEDDED: &str = include_str!("../data/styles/casino.vcss");
    let paths = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/styles/casino.vcss"),
        PathBuf::from("demos/velvet-stakes/data/styles/casino.vcss"),
        PathBuf::from("data/styles/casino.vcss"),
    ];
    for p in &paths {
        if let Ok(src) = std::fs::read_to_string(p) {
            if let Ok(sheet) = parse_stylesheet(&src) {
                return sheet;
            }
        }
    }
    parse_stylesheet(EMBEDDED).unwrap_or_default()
}

fn art_dir() -> PathBuf {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/art"),
        PathBuf::from("demos/velvet-stakes/data/art"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../card-duel/data/art"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("strike.jpg").exists())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/art"))
}

fn ui_dir() -> PathBuf {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui"),
        PathBuf::from("demos/velvet-stakes/data/ui"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("menu_bg.jpg").exists())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui"))
}

impl App {
    fn new(headless: bool) -> Result<Self> {
        let art_dir = art_dir();
        let (cat, deck, stats) = make_catalog_and_deck(&art_dir);
        let v = validate_deck(&cat, &deck, &DeckRules::open());
        if !v.ok {
            bail!("deck invalid: {:?}", v.violations);
        }
        let art = ArtBank::from_catalog_dir(
            &art_dir,
            &["strike", "guard", "fireball", "focus", "bash"],
        );
        if art.images.len() < 5 {
            bail!(
                "missing card art in {} (found {})",
                art_dir.display(),
                art.images.len()
            );
        }
        let ui = ui_dir();
        let menu_bg = load_rgb(&ui.join("menu_bg.jpg"));
        let logo_emblem = load_rgb(&ui.join("logo_emblem.jpg"));
        let stylesheet = load_casino_stylesheet();
        Ok(Self {
            screen: Screen::Title,
            menu_sel: 0,
            pause_sel: 0,
            result_sel: 0,
            result: None,
            run: None,
            stats,
            deck_ids: deck.cards,
            art,
            menu_bg,
            logo_emblem,
            stylesheet,
            theme: Theme::default(),
            meta_chips: 12_450,
            meta_crystals: 870,
            meta_mult: 3.2,
            money: 0,
            seed: 0xBA_1A_70_01,
            status: "Velvet Arcana".into(),
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            pixels: vec![0; (WW * WH) as usize],
        })
    }

    fn begin_run(&mut self) {
        self.money = 4;
        self.seed = self.seed.wrapping_add(13);
        self.start_blind(0);
    }

    fn start_blind(&mut self, ante: usize) {
        self.seed = self.seed.wrapping_add(1);
        let run = Run::start(ante, &self.deck_ids, self.seed, self.money);
        self.status = format!("{} / {}", run.blind_name, run.target);
        self.run = Some(run);
        self.screen = Screen::BlindInfo;
        self.result = None;
    }

    fn enter_play(&mut self) {
        self.screen = Screen::Play;
        self.status = "1-8 select · P play · D discard".into();
    }

    fn apply_outcome(&mut self, o: Outcome) {
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

    fn to_title(&mut self) {
        self.run = None;
        self.result = None;
        self.screen = Screen::Title;
        self.menu_sel = 0;
    }

    fn paint(&mut self) {
        match self.screen {
            Screen::Title => {
                paint_title_menu(
                    &mut self.pixels,
                    &self.theme,
                    self.menu_bg.as_ref(),
                    self.logo_emblem.as_ref(),
                    &self.stylesheet,
                    self.menu_sel,
                    self.meta_chips,
                    self.meta_crystals,
                    self.meta_mult,
                );
            }
            Screen::Collection => {
                paint_collection(
                    &mut self.pixels,
                    &self.theme,
                    self.menu_bg.as_ref(),
                    &self.art,
                );
            }
            Screen::Shop => {
                paint_shop(&mut self.pixels, &self.theme, self.menu_bg.as_ref());
            }
            Screen::Options => {
                paint_options(&mut self.pixels, &self.theme, self.menu_bg.as_ref());
            }
            Screen::BlindInfo => self.paint_blind(),
            Screen::Play | Screen::Pause => {
                self.paint_play();
                if self.screen == Screen::Pause {
                    self.paint_pause_overlay();
                }
            }
            Screen::Result => self.paint_result(),
        }
        self.present();
    }

    fn paint_blind(&mut self) {
        fill(&mut self.pixels, WW, WH, (16, 12, 28));
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
        text(
            &mut self.pixels,
            WW,
            WH,
            40,
            80,
            "SIGUIENTE CIEGA",
            (200, 180, 255),
            2,
        );
        text(&mut self.pixels, WW, WH, 40, 140, &name, (255, 220, 120), 3);
        text(
            &mut self.pixels,
            WW,
            WH,
            40,
            200,
            &format!("Target: {target} chips"),
            (220, 220, 230),
            2,
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            40,
            250,
            &format!("Hands {hands} · Discards {disc} · $ {money}"),
            (180, 190, 210),
            1,
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            40,
            340,
            "Enter = jugar",
            (255, 240, 200),
            2,
        );
    }

    fn paint_play(&mut self) {
        fill(&mut self.pixels, WW, WH, (18, 16, 28));
        rect(&mut self.pixels, WW, WH, 0, 0, WW as i32, 5, (220, 170, 50));

        let (score, target, hands, disc, money, blind, lib, disc_n, log, last, preview) = {
            let r = self.run.as_ref().unwrap();
            let prev = r.preview_score(&self.stats);
            (
                r.score,
                r.target,
                r.hands_left,
                r.discards_left,
                r.money,
                r.blind_name.clone(),
                r.zones.library.len(),
                r.zones.discard.len(),
                r.log.clone(),
                r.last.clone(),
                prev,
            )
        };

        text(
            &mut self.pixels,
            WW,
            WH,
            16,
            10,
            "VELVET STAKES",
            (255, 210, 90),
            1,
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            200,
            10,
            &blind,
            (200, 190, 255),
            1,
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            16,
            36,
            &format!("SCORE {score} / {target}"),
            (120, 255, 160),
            2,
        );
        let bar_w = 400;
        rect(&mut self.pixels, WW, WH, 16, 68, bar_w, 12, (40, 40, 55));
        let fill_w =
            ((score as f32 / target.max(1) as f32).min(1.0) * bar_w as f32) as i32;
        rect(
            &mut self.pixels,
            WW,
            WH,
            16,
            68,
            fill_w.max(0),
            12,
            (90, 200, 110),
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            16,
            90,
            &format!(
                "Hands {hands}  Disc {disc}  $ {money}  Deck {lib}  Discard {disc_n}"
            ),
            (170, 175, 190),
            1,
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            16,
            112,
            &format!(
                "Preview: {}  {}x{} = {}",
                preview.label, preview.chips, preview.mult, preview.total
            ),
            (255, 230, 140),
            1,
        );
        if !last.is_empty() {
            text(
                &mut self.pixels,
                WW,
                WH,
                16,
                132,
                &format!("Last: {last}"),
                (150, 200, 255),
                1,
            );
        }

        // cards
        let visuals: Vec<(String, Pose3D, bool)> = self
            .run
            .as_ref()
            .map(|r| {
                r.visuals
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        (
                            v.id.clone(),
                            v.pose,
                            r.selected.get(i).copied().unwrap_or(false),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        for (i, (id, pose, sel)) in visuals.iter().enumerate() {
            let base_w = 100.0;
            let base_h = 140.0;
            let w = (base_w * pose.scale) as i32;
            let h = (base_h * pose.scale) as i32;
            // yaw compress width (pseudo-3D)
            let yaw_scale = pose.yaw.cos().abs().max(0.25);
            let w = (w as f32 * yaw_scale) as i32;
            let x = pose.pos.x as i32 - w / 2;
            let y = pose.pos.y as i32;
            if *sel {
                rect(
                    &mut self.pixels,
                    WW,
                    WH,
                    x - 4,
                    y - 4,
                    w + 8,
                    h + 8,
                    (90, 70, 30),
                );
            }
            if let Some(art) = self.art.images.get(id) {
                blit_card(
                    &mut self.pixels,
                    WW,
                    WH,
                    art,
                    x,
                    y,
                    w.max(8),
                    h.max(8),
                    pose.opacity,
                );
            } else {
                rect(
                    &mut self.pixels,
                    WW,
                    WH,
                    x,
                    y,
                    w,
                    h,
                    (60, 50, 80),
                );
            }
            text(
                &mut self.pixels,
                WW,
                WH,
                x + 6,
                y + h + 4,
                &format!("[{}] {id}", i + 1),
                (200, 195, 180),
                1,
            );
        }

        for (i, line) in log.iter().enumerate() {
            text(
                &mut self.pixels,
                WW,
                WH,
                16,
                380 + i as i32 * 16,
                line,
                (140, 145, 160),
                1,
            );
        }
        text(
            &mut self.pixels,
            WW,
            WH,
            16,
            (WH as i32) - 24,
            "1-8 select · P play · D discard · Esc pause",
            (130, 130, 145),
            1,
        );
    }

    fn paint_pause_overlay(&mut self) {
        for p in &mut self.pixels {
            let a = *p;
            let r = ((a >> 16) & 0xFF) as u8 / 3;
            let g = ((a >> 8) & 0xFF) as u8 / 3;
            let b = (a & 0xFF) as u8 / 3;
            *p = velvet_story::pack_rgb(r, g, b);
        }
        rect(
            &mut self.pixels,
            WW,
            WH,
            260,
            150,
            440,
            200,
            (30, 26, 48),
        );
        text(
            &mut self.pixels,
            WW,
            WH,
            320,
            180,
            "PAUSA",
            (255, 210, 90),
            3,
        );
        let items = ["Continuar", "Menu principal"];
        for (i, item) in items.iter().enumerate() {
            let y = 250 + i as i32 * 40;
            let sel = i == self.pause_sel;
            let p = if sel { "> " } else { "  " };
            text(
                &mut self.pixels,
                WW,
                WH,
                320,
                y,
                &format!("{p}{item}"),
                (230, 230, 240),
                2,
            );
        }
    }

    fn paint_result(&mut self) {
        fill(&mut self.pixels, WW, WH, (14, 12, 20));
        let (title, col) = match self.result {
            Some(Outcome::WinBlind) => ("¡CIEGA SUPERADA!", (120, 255, 150)),
            Some(Outcome::LoseBlind) => ("CIEGA FALLIDA", (255, 120, 120)),
            Some(Outcome::RunClear) => ("¡RUN COMPLETA!", (255, 220, 80)),
            None => ("FIN", (200, 200, 200)),
        };
        text(&mut self.pixels, WW, WH, 40, 80, title, col, 3);
        if let Some(r) = &self.run {
            text(
                &mut self.pixels,
                WW,
                WH,
                40,
                150,
                &format!("Score {} / {}  ·  $ {}", r.score, r.target, r.money),
                (200, 200, 210),
                1,
            );
            if !r.last.is_empty() {
                text(
                    &mut self.pixels,
                    WW,
                    WH,
                    40,
                    180,
                    &format!("Ultima: {}", r.last),
                    (160, 180, 200),
                    1,
                );
            }
        }
        let items: &[&str] = match self.result {
            Some(Outcome::WinBlind) => &["Siguiente ciega", "Menu principal"],
            _ => &["Nueva run", "Menu principal"],
        };
        for (i, item) in items.iter().enumerate() {
            let y = 260 + i as i32 * 44;
            let sel = i == self.result_sel;
            let p = if sel { "> " } else { "  " };
            if sel {
                rect(
                    &mut self.pixels,
                    WW,
                    WH,
                    36,
                    y - 4,
                    400,
                    32,
                    (50, 45, 70),
                );
            }
            text(
                &mut self.pixels,
                WW,
                WH,
                48,
                y,
                &format!("{p}{item}"),
                (240, 235, 220),
                2,
            );
        }
    }

    fn present(&mut self) {
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
        window.set_title(&format!("Velvet Arcana — {:?}", self.screen));
    }

    fn on_key(&mut self, c: KeyCode, el: &ActiveEventLoop) {
        match self.screen {
            Screen::Title => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.menu_sel = self.menu_sel.saturating_sub(1);
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    if self.menu_sel + 1 < TITLE_ITEMS.len() {
                        self.menu_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space => match self.menu_sel {
                    0 => self.begin_run(),
                    1 => self.screen = Screen::Collection,
                    2 => self.screen = Screen::Shop,
                    3 => self.screen = Screen::Options,
                    4 => el.exit(),
                    _ => {}
                },
                KeyCode::Escape => el.exit(),
                _ => {}
            },
            Screen::Collection | Screen::Shop | Screen::Options => {
                if matches!(c, KeyCode::Enter | KeyCode::Space | KeyCode::Escape) {
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
                        r.toggle(0);
                    }
                }
                KeyCode::Digit2 | KeyCode::Numpad2 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(1);
                    }
                }
                KeyCode::Digit3 | KeyCode::Numpad3 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(2);
                    }
                }
                KeyCode::Digit4 | KeyCode::Numpad4 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(3);
                    }
                }
                KeyCode::Digit5 | KeyCode::Numpad5 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(4);
                    }
                }
                KeyCode::Digit6 | KeyCode::Numpad6 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(5);
                    }
                }
                KeyCode::Digit7 | KeyCode::Numpad7 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(6);
                    }
                }
                KeyCode::Digit8 | KeyCode::Numpad8 => {
                    if let Some(r) = self.run.as_mut() {
                        r.toggle(7);
                    }
                }
                KeyCode::KeyP | KeyCode::Enter => {
                    if let Some(r) = self.run.as_mut() {
                        if let Some(o) = r.play_selected(&self.stats) {
                            self.apply_outcome(o);
                        }
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
                    if self.pause_sel < 1 {
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
                    if self.result_sel < 1 {
                        self.result_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space => match self.result {
                    Some(Outcome::WinBlind) if self.result_sel == 0 => {
                        let next = self.run.as_ref().map(|r| r.ante + 1).unwrap_or(1);
                        self.start_blind(next);
                    }
                    Some(Outcome::WinBlind) => self.to_title(),
                    _ if self.result_sel == 0 => self.begin_run(),
                    _ => self.to_title(),
                },
                KeyCode::Escape => self.to_title(),
                _ => {}
            },
        }
    }

    fn auto_tick(&mut self, el: &ActiveEventLoop) {
        match self.screen {
            Screen::Title if self.hframes == 2 => self.begin_run(),
            Screen::BlindInfo if self.hframes >= 5 => self.enter_play(),
            Screen::Play => {
                if let Some(r) = self.run.as_mut() {
                    // select first 2 if none
                    if !r.selected.iter().any(|s| *s) && !r.zones.hand.is_empty() {
                        r.selected[0] = true;
                        if r.zones.hand.len() > 1 {
                            r.selected[1] = true;
                        }
                    }
                    if let Some(o) = r.play_selected(&self.stats) {
                        self.apply_outcome(o);
                    }
                }
            }
            Screen::Result if self.hframes > 40 => {
                println!(
                    "headless result={:?} money={} frames={}",
                    self.result, self.money, self.hframes
                );
                el.exit();
            }
            _ => {}
        }
        if self.hframes > 600 {
            println!("headless timeout");
            el.exit();
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
            .with_title("Velvet Arcana — Nightfall Casino")
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
        let dt = now.duration_since(self.last).as_secs_f32().min(0.05);
        self.last = now;
        self.hframes += 1;
        if let Some(r) = self.run.as_mut() {
            r.tick_anims(dt);
        }
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
    let mut app = App::new(headless)?;
    println!(
        "Velvet Arcana LOCAL — cards={} deck={} bg={} logo={} vcss_rules={}",
        app.art.images.len(),
        app.deck_ids.len(),
        app.menu_bg.is_some(),
        app.logo_emblem.is_some(),
        app.stylesheet.rules.len()
    );
    if headless {
        println!("headless smoke…");
    } else {
        println!("Lobby menu (reference art) · 1-8 select · P play · D discard");
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}
