//! Card Duel — windowed demo built on Velvet Engine + `velvet-cards`.
//!
//! Screens: Title menu → How to Play → Battle → Result (win/lose).
//! Uses catalog/deck/zones tools for the real deck math; UI is a softbuffer host.
//!
//! Controls (menus): Up/Down or W/S · Enter/Space select · Esc back/quit  
//! Controls (battle): 1–6 play hand slot · E end turn · Esc pause  
//! `--headless`: auto smoke through menu → battle → exit 0 on clear path

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use softbuffer::{Context as SbContext, Surface};
use velvet_cards::{
    load_catalog_json, load_deck_json, validate_deck, CardCatalog, CardDef, CardZones, DeckList,
    DeckRules,
};
use velvet_story::{draw_text_line, pack_rgb};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const WW: u32 = 960;
const WH: u32 = 540;

const PLAYER_MAX_HP: i32 = 30;
const ENEMY_MAX_HP: i32 = 36;
const ENERGY_MAX: i32 = 3;
const HAND_SIZE: usize = 5;

// --- screens -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    HowTo,
    Battle,
    Pause,
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Win,
    Lose,
}

// --- battle state ------------------------------------------------------------

struct Battle {
    catalog: CardCatalog,
    zones: CardZones,
    player_hp: i32,
    player_block: i32,
    enemy_hp: i32,
    energy: i32,
    energy_max: i32,
    turn: u32,
    log: Vec<String>,
    seed: u64,
}

impl Battle {
    fn start(catalog: CardCatalog, deck: &DeckList, seed: u64) -> Result<Self> {
        let v = validate_deck(&catalog, deck, &DeckRules::open());
        if !v.ok {
            bail!("deck invalid: {:?}", v.violations);
        }
        let mut zones = CardZones::from_deck_list(deck);
        zones.shuffle_library(seed);
        zones.draw(HAND_SIZE).context("opening hand")?;
        Ok(Self {
            catalog,
            zones,
            player_hp: PLAYER_MAX_HP,
            player_block: 0,
            enemy_hp: ENEMY_MAX_HP,
            energy: ENERGY_MAX,
            energy_max: ENERGY_MAX,
            turn: 1,
            log: vec!["Battle start — play cards (1-6), E end turn.".into()],
            seed,
        })
    }

    fn push_log(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 8 {
            let n = self.log.len() - 8;
            self.log.drain(0..n);
        }
    }

    fn def(&self, id: &str) -> Option<&CardDef> {
        self.catalog.get(id)
    }

    fn play_hand(&mut self, index: usize) -> Result<(), String> {
        if index >= self.zones.hand.len() {
            return Err("no card in that slot".into());
        }
        let id = self.zones.hand[index].clone();
        let def = self
            .def(&id)
            .ok_or_else(|| format!("unknown card {id}"))?
            .clone();
        if def.cost > self.energy {
            return Err(format!("need {} energy", def.cost));
        }
        // Spend + move to discard
        self.energy -= def.cost;
        let played = self
            .zones
            .discard_from_hand(index)
            .map_err(|e| e.to_string())?;
        self.resolve_card(&def, &played);
        Ok(())
    }

    fn resolve_card(&mut self, def: &CardDef, id: &str) {
        let tags = &def.tags;
        let ctype = def.card_type.as_deref().unwrap_or("");
        if tags.iter().any(|t| t == "draw") || id == "draw_two" {
            let n = 2.min(self.zones.library.len());
            if n > 0 {
                let _ = self.zones.draw(n);
                self.push_log(format!("{}: draw {n}", def.name));
            } else {
                self.push_log(format!("{}: library empty", def.name));
            }
            return;
        }
        if tags.iter().any(|t| t == "defense")
            || ctype == "skill" && tags.iter().any(|t| t == "defense")
        {
            let block = match id {
                "fortify" => 7,
                _ => 4,
            };
            self.player_block += block;
            self.push_log(format!("{}: +{block} block", def.name));
            return;
        }
        // Default: damage
        let dmg = match id {
            "fireball" => 9,
            "bash" => 8,
            "strike" => 5,
            _ => 4 + def.cost,
        };
        self.enemy_hp = (self.enemy_hp - dmg).max(0);
        self.push_log(format!("{}: {dmg} dmg → enemy", def.name));
    }

    fn end_turn(&mut self) {
        // Enemy attack
        let raw = 6 + (self.turn as i32 % 3);
        let absorbed = raw.min(self.player_block);
        self.player_block -= absorbed;
        let dealt = raw - absorbed;
        self.player_hp = (self.player_hp - dealt).max(0);
        self.push_log(format!("Enemy hits {raw} (blocked {absorbed})"));

        if self.player_hp <= 0 || self.enemy_hp <= 0 {
            return;
        }

        // Discard remaining hand? Keep for simplicity (roguelike-lite: keep hand)
        // Refill energy, draw 1, next turn
        self.energy = self.energy_max;
        self.player_block = 0; // block expires
        if !self.zones.library.is_empty() {
            let _ = self.zones.draw(1);
        } else if !self.zones.discard.is_empty() {
            // recycle discard → library
            self.zones.library.append(&mut self.zones.discard);
            self.zones
                .shuffle_library(self.seed.wrapping_add(self.turn as u64));
            let _ = self.zones.draw(1);
            self.push_log("Recycled discard into library.");
        }
        self.turn += 1;
        self.push_log(format!("--- Turn {} ---", self.turn));
    }

    fn outcome(&self) -> Option<Outcome> {
        if self.enemy_hp <= 0 {
            Some(Outcome::Win)
        } else if self.player_hp <= 0 {
            Some(Outcome::Lose)
        } else {
            None
        }
    }
}

// --- app ---------------------------------------------------------------------

struct App {
    screen: Screen,
    menu_sel: usize,
    pause_sel: usize,
    result_sel: usize,
    result: Option<Outcome>,
    battle: Option<Battle>,
    catalog_path: PathBuf,
    deck_path: PathBuf,
    catalog: CardCatalog,
    deck: DeckList,
    status: String,
    window: Option<Arc<Window>>,
    context: Option<SbContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    pixels: Vec<u32>,
    /// Headless script step.
    auto_step: u32,
}

fn data_paths() -> (PathBuf, PathBuf) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let candidates = [
        base.clone(),
        PathBuf::from("demos/card-duel/data"),
        PathBuf::from("data"),
    ];
    for c in candidates {
        let cat = c.join("catalog.json");
        let deck = c.join("deck.json");
        if cat.exists() && deck.exists() {
            return (cat, deck);
        }
    }
    (
        base.join("catalog.json"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data")
            .join("deck.json"),
    )
}

impl App {
    fn new(headless: bool) -> Result<Self> {
        let (catalog_path, deck_path) = data_paths();
        let catalog = load_catalog_json(&catalog_path)
            .with_context(|| format!("catalog {}", catalog_path.display()))?;
        let deck =
            load_deck_json(&deck_path).with_context(|| format!("deck {}", deck_path.display()))?;
        let v = validate_deck(&catalog, &deck, &DeckRules::open());
        if !v.ok {
            bail!("starter deck invalid: {:?}", v.violations);
        }
        Ok(Self {
            screen: Screen::Title,
            menu_sel: 0,
            pause_sel: 0,
            result_sel: 0,
            result: None,
            battle: None,
            catalog_path,
            deck_path,
            catalog,
            deck,
            status: "Velvet Card Duel".into(),
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            pixels: vec![0; (WW * WH) as usize],
            auto_step: 0,
        })
    }

    fn title_items() -> &'static [&'static str] {
        &["Iniciar duelo", "Cómo jugar", "Salir"]
    }

    fn pause_items() -> &'static [&'static str] {
        &["Continuar", "Reiniciar duelo", "Menú principal"]
    }

    fn result_items() -> &'static [&'static str] {
        &["Revancha", "Menú principal"]
    }

    fn start_battle(&mut self) {
        match Battle::start(self.catalog.clone(), &self.deck, 0xC42D_0E11) {
            Ok(b) => {
                self.battle = Some(b);
                self.screen = Screen::Battle;
                self.status = "Duelo en curso".into();
                self.result = None;
            }
            Err(e) => self.status = format!("No se pudo iniciar: {e:#}"),
        }
    }

    fn confirm_title(&mut self, el: &ActiveEventLoop) {
        match self.menu_sel {
            0 => self.start_battle(),
            1 => {
                self.screen = Screen::HowTo;
                self.status = "Cómo jugar".into();
            }
            2 => el.exit(),
            _ => {}
        }
    }

    fn confirm_pause(&mut self) {
        match self.pause_sel {
            0 => self.screen = Screen::Battle,
            1 => self.start_battle(),
            2 => {
                self.battle = None;
                self.screen = Screen::Title;
                self.menu_sel = 0;
                self.status = "Menú principal".into();
            }
            _ => {}
        }
    }

    fn confirm_result(&mut self) {
        match self.result_sel {
            0 => self.start_battle(),
            1 => {
                self.battle = None;
                self.result = None;
                self.screen = Screen::Title;
                self.menu_sel = 0;
            }
            _ => {}
        }
    }

    fn try_play(&mut self, slot: usize) {
        let Some(battle) = self.battle.as_mut() else {
            return;
        };
        match battle.play_hand(slot) {
            Ok(()) => {
                self.status = format!("Jugaste slot {}", slot + 1);
                if let Some(o) = battle.outcome() {
                    self.result = Some(o);
                    self.screen = Screen::Result;
                    self.result_sel = 0;
                }
            }
            Err(e) => self.status = e,
        }
    }

    fn try_end_turn(&mut self) {
        let Some(battle) = self.battle.as_mut() else {
            return;
        };
        battle.end_turn();
        if let Some(o) = battle.outcome() {
            self.result = Some(o);
            self.screen = Screen::Result;
            self.result_sel = 0;
            self.status = match o {
                Outcome::Win => "¡Victoria!".into(),
                Outcome::Lose => "Derrota…".into(),
            };
        } else {
            self.status = format!("Turno {}", battle.turn);
        }
    }

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

    fn paint_menu_list(&mut self, title: &str, items: &[&str], sel: usize, hint: &str, y0: i32) {
        self.text(48, 36, title, (255, 230, 160), 3);
        self.text(48, 72, "Velvet Engine · velvet-cards", (160, 155, 180), 1);
        for (i, item) in items.iter().enumerate() {
            let y = y0 + (i as i32) * 40;
            let selected = i == sel;
            if selected {
                self.rect(40, y - 6, 420, 32, (60, 40, 90));
            }
            let prefix = if selected { "> " } else { "  " };
            let color = if selected {
                (255, 240, 200)
            } else {
                (200, 195, 210)
            };
            self.text(56, y, &format!("{prefix}{item}"), color, 2);
        }
        self.text(48, (WH as i32) - 48, hint, (140, 135, 155), 1);
        let status = self.status.clone();
        self.text(48, (WH as i32) - 28, &status, (180, 175, 200), 1);
    }

    fn paint_title(&mut self) {
        self.fill_bg((18, 12, 28));
        self.rect(0, 0, WW as i32, 8, (140, 60, 200));
        self.paint_menu_list(
            "CARD DUEL",
            Self::title_items(),
            self.menu_sel,
            "↑↓ / W S  ·  Enter iniciar  ·  Esc salir",
            160,
        );
        self.text(
            48,
            120,
            "Demo de menús + duelo simple (no es un TCG completo)",
            (170, 165, 190),
            1,
        );
    }

    fn paint_howto(&mut self) {
        self.fill_bg((14, 16, 24));
        self.text(48, 36, "CÓMO JUGAR", (200, 220, 255), 3);
        let lines = [
            "Objetivo: bajar la vida del enemigo a 0.",
            "Energía: 3 por turno. Cada carta tiene un coste.",
            "1-6: jugar la carta en esa ranura de la mano.",
            "E / Espacio: terminar turno (el enemigo ataca).",
            "Esc: pausa. Las zonas usan velvet-cards (library/hand/discard).",
            "",
            "Strike/Bash/Fireball = daño · Guard/Fortify = bloqueo · Focus = robar.",
            "",
            "Enter o Esc: volver al menú",
        ];
        for (i, line) in lines.iter().enumerate() {
            self.text(48, 100 + (i as i32) * 28, line, (210, 210, 220), 1);
        }
    }

    fn paint_battle(&mut self) {
        self.fill_bg((22, 18, 30));
        self.rect(0, 0, WW as i32, 6, (90, 50, 140));

        let (php, pblk, ehp, energy, turn, hand, lib, disc, log) = {
            let b = self.battle.as_ref().unwrap();
            (
                b.player_hp,
                b.player_block,
                b.enemy_hp,
                b.energy,
                b.turn,
                b.zones.hand.clone(),
                b.zones.library.len(),
                b.zones.discard.len(),
                b.log.clone(),
            )
        };

        self.text(24, 16, "DUEL", (255, 220, 140), 2);
        self.text(
            120,
            20,
            &format!("Turn {turn}  ·  Energy {energy}/{ENERGY_MAX}"),
            (220, 210, 230),
            1,
        );
        self.text(
            24,
            52,
            &format!("YOU  HP {php}/{PLAYER_MAX_HP}  Block {pblk}"),
            (120, 220, 160),
            2,
        );
        self.text(
            24,
            88,
            &format!("FOE  HP {ehp}/{ENEMY_MAX_HP}"),
            (240, 120, 120),
            2,
        );
        // HP bars
        let bar_w = 280;
        self.rect(320, 56, bar_w, 14, (40, 40, 50));
        let pw = ((php as f32 / PLAYER_MAX_HP as f32) * bar_w as f32) as i32;
        self.rect(320, 56, pw.max(0), 14, (60, 180, 100));
        self.rect(320, 92, bar_w, 14, (40, 40, 50));
        let ew = ((ehp as f32 / ENEMY_MAX_HP as f32) * bar_w as f32) as i32;
        self.rect(320, 92, ew.max(0), 14, (200, 70, 70));

        self.text(
            24,
            130,
            &format!("Library {lib}  ·  Discard {disc}  ·  Hand {}", hand.len()),
            (160, 155, 180),
            1,
        );

        // Hand cards as panels
        for (i, id) in hand.iter().enumerate() {
            let x = 24 + (i as i32) * 150;
            let y = 170;
            self.rect(x, y, 140, 180, (45, 35, 70));
            self.rect(x + 2, y + 2, 136, 176, (55, 45, 85));
            let name = self
                .catalog
                .get(id)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| id.clone());
            let cost = self.catalog.get(id).map(|c| c.cost).unwrap_or(0);
            let blurb = self
                .catalog
                .get(id)
                .and_then(|c| c.text.clone())
                .unwrap_or_default();
            let short: String = blurb.chars().take(22).collect();
            self.text(
                x + 10,
                y + 12,
                &format!("[{}] {}", i + 1, name),
                (255, 240, 200),
                1,
            );
            self.text(x + 10, y + 40, &format!("Cost {cost}"), (180, 200, 255), 1);
            self.text(x + 10, y + 70, &short, (200, 195, 210), 1);
            if cost > energy {
                self.text(x + 10, y + 150, "(no energy)", (180, 100, 100), 1);
            }
        }

        // Log
        self.text(24, 370, "Log", (180, 170, 200), 1);
        for (i, line) in log.iter().enumerate() {
            self.text(24, 392 + (i as i32) * 16, line, (150, 150, 165), 1);
        }

        self.text(
            24,
            (WH as i32) - 28,
            "1-6 jugar · E fin de turno · Esc pausa",
            (140, 135, 155),
            1,
        );
        let status = self.status.clone();
        self.text(400, (WH as i32) - 28, &status, (200, 190, 170), 1);
    }

    fn paint_pause(&mut self) {
        // dim battle underneath lightly
        self.paint_battle();
        for p in &mut self.pixels {
            // darken
            let a = *p;
            let r = ((a >> 16) & 0xFF) as u8 / 3;
            let g = ((a >> 8) & 0xFF) as u8 / 3;
            let b = (a & 0xFF) as u8 / 3;
            *p = pack_rgb(r, g, b);
        }
        self.rect(220, 120, 520, 280, (30, 24, 48));
        self.text(260, 150, "PAUSA", (255, 230, 160), 3);
        for (i, item) in Self::pause_items().iter().enumerate() {
            let y = 220 + (i as i32) * 40;
            let selected = i == self.pause_sel;
            if selected {
                self.rect(250, y - 4, 400, 30, (70, 50, 100));
            }
            let prefix = if selected { "> " } else { "  " };
            self.text(270, y, &format!("{prefix}{item}"), (230, 225, 240), 2);
        }
    }

    fn paint_result(&mut self) {
        self.fill_bg((16, 14, 22));
        let (title, color) = match self.result {
            Some(Outcome::Win) => ("¡VICTORIA!", (120, 230, 140)),
            Some(Outcome::Lose) => ("DERROTA", (240, 120, 120)),
            None => ("FIN", (220, 220, 220)),
        };
        self.text(48, 80, title, color, 4);
        if let Some(b) = &self.battle {
            self.text(
                48,
                150,
                &format!(
                    "Turnos: {}  ·  Enemigo HP final: {}  ·  Tu HP: {}",
                    b.turn, b.enemy_hp, b.player_hp
                ),
                (200, 195, 210),
                1,
            );
        }
        for (i, item) in Self::result_items().iter().enumerate() {
            let y = 240 + (i as i32) * 44;
            let selected = i == self.result_sel;
            if selected {
                self.rect(40, y - 6, 360, 34, (55, 40, 80));
            }
            let prefix = if selected { "> " } else { "  " };
            self.text(56, y, &format!("{prefix}{item}"), (240, 235, 220), 2);
        }
        self.text(
            48,
            (WH as i32) - 40,
            "Enter seleccionar",
            (140, 135, 155),
            1,
        );
    }

    fn paint(&mut self) {
        match self.screen {
            Screen::Title => self.paint_title(),
            Screen::HowTo => self.paint_howto(),
            Screen::Battle => self.paint_battle(),
            Screen::Pause => self.paint_pause(),
            Screen::Result => self.paint_result(),
        }

        let Some(window) = self.window.clone() else {
            return;
        };
        let size = window.inner_size();
        let ww = size.width.max(1);
        let wh = size.height.max(1);
        // Present at logical fixed res stretched by softbuffer buffer size = window
        // We always paint into WW×WH then scale naively if needed.
        let mut present = self.pixels.clone();
        if ww != WW || wh != WH {
            present = scale_nearest(&self.pixels, WW, WH, ww, wh);
        }

        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        let _ = surface.resize(NonZeroU32::new(ww).unwrap(), NonZeroU32::new(wh).unwrap());
        if let Ok(mut buf) = surface.buffer_mut() {
            let n = present.len().min(buf.len());
            buf[..n].copy_from_slice(&present[..n]);
            let _ = buf.present();
        }
        window.set_title(&format!("Card Duel — {:?} — {}", self.screen, self.status));
    }

    /// Headless automation: open start → play cards until result or timeout.
    fn auto_tick(&mut self, el: &ActiveEventLoop) {
        self.auto_step += 1;
        match self.screen {
            Screen::Title if self.auto_step == 1 => {
                self.menu_sel = 0;
                self.start_battle();
            }
            Screen::Battle => {
                // Try play affordable cards left-to-right, else end turn
                let play_idx = self.battle.as_ref().and_then(|b| {
                    b.zones.hand.iter().enumerate().find_map(|(i, id)| {
                        let cost = b.def(id).map(|d| d.cost).unwrap_or(99);
                        if cost <= b.energy {
                            Some(i)
                        } else {
                            None
                        }
                    })
                });
                if let Some(idx) = play_idx {
                    self.try_play(idx);
                } else if self.screen == Screen::Battle {
                    self.try_end_turn();
                }
            }
            Screen::Result if self.auto_step > 2 => {
                println!("headless result={:?} frames={}", self.result, self.hframes);
                el.exit();
            }
            Screen::HowTo | Screen::Pause | Screen::Title => {}
            Screen::Result => {}
        }
        if self.hframes > 400 {
            println!("headless timeout screen={:?}", self.screen);
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
            .with_title("Card Duel — Velvet Engine")
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
        let dt = now.duration_since(self.last);
        if dt < Duration::from_millis(16) {
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

impl App {
    fn on_key(&mut self, c: KeyCode, el: &ActiveEventLoop) {
        match self.screen {
            Screen::Title => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    if self.menu_sel > 0 {
                        self.menu_sel -= 1;
                    }
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    if self.menu_sel + 1 < Self::title_items().len() {
                        self.menu_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space | KeyCode::NumpadEnter => {
                    self.confirm_title(el);
                }
                KeyCode::Escape => el.exit(),
                _ => {}
            },
            Screen::HowTo => match c {
                KeyCode::Enter | KeyCode::Space | KeyCode::Escape | KeyCode::NumpadEnter => {
                    self.screen = Screen::Title;
                    self.status = "Menú principal".into();
                }
                _ => {}
            },
            Screen::Battle => match c {
                KeyCode::Digit1 | KeyCode::Numpad1 => self.try_play(0),
                KeyCode::Digit2 | KeyCode::Numpad2 => self.try_play(1),
                KeyCode::Digit3 | KeyCode::Numpad3 => self.try_play(2),
                KeyCode::Digit4 | KeyCode::Numpad4 => self.try_play(3),
                KeyCode::Digit5 | KeyCode::Numpad5 => self.try_play(4),
                KeyCode::Digit6 | KeyCode::Numpad6 => self.try_play(5),
                KeyCode::KeyE | KeyCode::Space => self.try_end_turn(),
                KeyCode::Escape => {
                    self.screen = Screen::Pause;
                    self.pause_sel = 0;
                    self.status = "Pausa".into();
                }
                _ => {}
            },
            Screen::Pause => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    if self.pause_sel > 0 {
                        self.pause_sel -= 1;
                    }
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    if self.pause_sel + 1 < Self::pause_items().len() {
                        self.pause_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space | KeyCode::NumpadEnter => self.confirm_pause(),
                KeyCode::Escape => {
                    self.screen = Screen::Battle;
                }
                _ => {}
            },
            Screen::Result => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    if self.result_sel > 0 {
                        self.result_sel -= 1;
                    }
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    if self.result_sel + 1 < Self::result_items().len() {
                        self.result_sel += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Space | KeyCode::NumpadEnter => self.confirm_result(),
                KeyCode::Escape => {
                    self.battle = None;
                    self.result = None;
                    self.screen = Screen::Title;
                }
                _ => {}
            },
        }
    }
}

fn main() -> Result<()> {
    velvet_core::init_tracing_default("card_duel=info,info");
    let headless = std::env::args().any(|a| a == "--headless");
    let mut app = App::new(headless)?;
    println!(
        "Card Duel — catalog={} deck={} cards={} deck_size={}",
        app.catalog_path.display(),
        app.deck_path.display(),
        app.catalog.len(),
        app.deck.len()
    );
    if headless {
        println!("headless smoke: title → battle → result");
    }

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}
