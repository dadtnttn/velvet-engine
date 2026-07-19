//! Velvet Arcana — **local** Balatro-style casino demo.
//!
//! Author flow: **`.vstory`** (`data/story/main.vstory`)  
//! Look + motion: **`.vcss`** (`data/styles/casino.vcss`, CSS + JS-lite `@script`)  
//! Rust host: window, paint, input → story resume + `stakes.*` / `style.*`
//!
//! Controls: menus ↑↓ Enter · Play: 1–8 select · P play · D discard · Esc  
//! `--headless`: auto smoke  
//! `--dev`: live reload styles / images / story from disk (no full restart)

mod catalog;
mod game;
mod host;
mod story_boot;

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use softbuffer::{Context as SbContext, Surface};
use velvet_anim::Pose3D;
use velvet_cards::{validate_deck, DeckRules};
use velvet_stakes::render::{blit_card, fill, load_rgb, rect, text, ArtBank, RgbImage};
use velvet_stakes::ui::theme::{Theme, TITLE_ITEMS, WW, WH};
use velvet_stakes::ui::{paint_collection, paint_options, paint_shop, paint_title_menu};
use velvet_stakes::{ImageSlot, LiveDevSession, RgbaBuf};
use velvet_story::{StoryPlayer, StoryValue, StoryWait};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use catalog::make_catalog_and_deck;
use game::{Outcome, Screen};
use host::{StakesHost, StakesWorld};
use story_boot::boot_player;

struct App {
    host: Arc<StakesHost>,
    player: StoryPlayer,
    art: ArtBank,
    menu_bg: Option<RgbImage>,
    /// Elegant title wordmark (black keyed / alpha) — live-dev reloads this.
    logo_title: Option<RgbaBuf>,
    portrait: Option<RgbImage>,
    theme: Theme,
    /// Live author hot-reload (`--dev`).
    live_dev: Option<LiveDevSession>,
    data_root: PathBuf,
    window: Option<Arc<Window>>,
    context: Option<SbContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    last: Instant,
    headless: bool,
    hframes: u32,
    pixels: Vec<u32>,
    status_line: String,
}

fn data_root() -> PathBuf {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"),
        PathBuf::from("demos/velvet-stakes/data"),
        PathBuf::from("data"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("styles/casino.vcss").exists() || p.join("art/strike.jpg").exists())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"))
}

fn art_dir(root: &std::path::Path) -> PathBuf {
    let p = root.join("art");
    if p.join("strike.jpg").exists() {
        p
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/art")
    }
}

fn ui_dir(root: &std::path::Path) -> PathBuf {
    let p = root.join("ui");
    if p.join("menu_bg.jpg").exists() {
        p
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/ui")
    }
}

impl App {
    fn new(headless: bool, dev: bool) -> Result<Self> {
        let root = data_root();
        let art_path = art_dir(&root);
        let (cat, deck, stats) = make_catalog_and_deck(&art_path);
        let v = validate_deck(&cat, &deck, &DeckRules::open());
        if !v.ok {
            bail!("deck invalid: {:?}", v.violations);
        }
        let art = ArtBank::from_catalog_dir(
            &art_path,
            &["strike", "guard", "fireball", "focus", "bash"],
        );
        if art.images.len() < 5 {
            bail!(
                "missing card art in {} (found {})",
                art_path.display(),
                art.images.len()
            );
        }
        let ui = ui_dir(&root);
        let menu_bg = load_rgb(&ui.join("menu_bg.jpg"));
        // Title is painted with a serif font (fontdue); no logo plate required
        let logo_title: Option<RgbaBuf> = None;
        let portrait = load_rgb(&ui.join("portrait_collector.jpg"));

        let world = StakesWorld::new(stats, deck.cards, root.clone());
        let host = Arc::new(StakesHost::new(world));
        let player = boot_player(host.clone(), &root)?;

        let live_dev = if dev {
            Some(LiveDevSession::watch_stakes_tree(&root))
        } else {
            None
        };

        Ok(Self {
            host,
            player,
            art,
            menu_bg,
            logo_title,
            portrait,
            theme: Theme::default(),
            live_dev,
            data_root: root,
            window: None,
            context: None,
            surface: None,
            last: Instant::now(),
            headless,
            hframes: 0,
            pixels: vec![0; (WW * WH) as usize],
            status_line: if dev {
                "DEV live-reload on".into()
            } else {
                String::new()
            },
        })
    }

    /// Poll watched author files and apply (no process restart).
    fn tick_live_dev(&mut self) {
        let Some(dev) = self.live_dev.as_mut() else {
            return;
        };
        let apply = dev.tick();
        if apply.reloaded.is_empty()
            && apply.stylesheet.is_none()
            && apply.images.is_empty()
            && apply.logo_title.is_none()
            && !apply.story_reload
        {
            return;
        }
        if let Some((name, sheet)) = apply.stylesheet {
            if let Err(e) = self.host.apply_stylesheet(&name, sheet) {
                eprintln!("dev: apply stylesheet failed: {e}");
            } else {
                self.status_line = format!("dev: style `{name}` live");
            }
        }
        if let Some(logo) = apply.logo_title {
            self.logo_title = Some(logo);
            self.status_line = "dev: logo_title live".into();
        }
        for (slot, buf) in apply.images {
            match slot {
                ImageSlot::MenuBg => self.menu_bg = Some(buf),
                ImageSlot::Logo => {
                    // Handled via apply.logo_title (RGBA soft-key path)
                    let _ = buf;
                }
                ImageSlot::Portrait => self.portrait = Some(buf),
                ImageSlot::Card(id) => {
                    self.art.images.insert(id, buf);
                }
            }
            self.status_line = "dev: image live".into();
        }
        if apply.story_reload {
            // Soft re-boot only when sitting on title wait (safe)
            let phase = match self.player.wait() {
                StoryWait::Host { token } => Some(token.as_str()),
                _ => None,
            };
            if matches!(phase, Some("title") | None) {
                match boot_player(self.host.clone(), &self.data_root) {
                    Ok(p) => {
                        self.player = p;
                        self.status_line = "dev: story reloaded".into();
                        eprintln!("dev: .vstory soft-reloaded");
                    }
                    Err(e) => eprintln!("dev: story reload failed (kept previous): {e}"),
                }
            } else {
                self.status_line = "dev: story changed (reload on title)".into();
                eprintln!("dev: story file changed — will soft-reload when back on title");
            }
        }
        for line in apply.log {
            // already eprinted inside session; keep last for window title
            if line.contains("reloaded") {
                self.status_line = line;
            }
        }
    }

    fn with_world_mut<R>(&self, f: impl FnOnce(&mut StakesWorld) -> R) -> R {
        let mut w = self.host.world.lock().expect("world");
        f(&mut w)
    }

    fn with_world<R>(&self, f: impl FnOnce(&StakesWorld) -> R) -> R {
        let w = self.host.world.lock().expect("world");
        f(&w)
    }

    fn wait_phase(&self) -> Option<String> {
        match self.player.wait() {
            StoryWait::Host { token } => Some(token.clone()),
            _ => self.with_world(|w| w.wait_phase.clone()),
        }
    }

    fn resume(&mut self, phase: &str) {
        if let Err(e) = self.player.resume_host(phase) {
            eprintln!("resume_host({phase}): {e}");
        }
        // clear wait phase mirror if story moved on
        if let Ok(mut w) = self.host.world.lock() {
            if !matches!(self.player.wait(), StoryWait::Host { token } if token == phase) {
                w.wait_phase = None;
            }
            w.sync_vars(self.player.variables_mut());
        }
        if self.with_world(|w| w.quit) || self.player.is_ended() {
            // handled by event loop exit
        }
    }

    fn paint(&mut self) {
        let screen = self.with_world(|w| w.screen);
        match screen {
            Screen::Title => {
                let (sel, chips, cry, mult, sheet) = self.with_world(|w| {
                    (
                        w.menu_sel,
                        w.meta_chips,
                        w.meta_crystals,
                        w.meta_mult,
                        w.stylesheet.clone(),
                    )
                });
                paint_title_menu(
                    &mut self.pixels,
                    &self.theme,
                    self.menu_bg.as_ref(),
                    self.logo_title.as_ref(),
                    self.portrait.as_ref(),
                    &sheet,
                    sel,
                    chips,
                    cry,
                    mult,
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
                if screen == Screen::Pause {
                    self.paint_pause_overlay();
                }
            }
            Screen::Result => self.paint_result(),
        }
        self.present();
    }

    fn paint_blind(&mut self) {
        fill(&mut self.pixels, WW, WH, (16, 12, 28));
        let (name, target, hands, disc, money) = self.with_world(|w| {
            w.run
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
                .unwrap_or_else(|| ("?".into(), 0, 0, 0, 0))
        });
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
            "Enter = jugar  (.vstory → play + .vcss deal)",
            (255, 240, 200),
            2,
        );
    }

    fn paint_play(&mut self) {
        fill(&mut self.pixels, WW, WH, (18, 16, 28));
        rect(&mut self.pixels, WW, WH, 0, 0, WW as i32, 5, (220, 170, 50));

        let (score, target, hands, disc, money, blind, lib, disc_n, log, last, preview, visuals) =
            self.with_world(|w| {
                let r = w.run.as_ref().unwrap();
                let prev = r.preview_score(&w.stats);
                let visuals: Vec<(String, Pose3D, bool)> = r
                    .visuals
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        (
                            v.id.clone(),
                            v.pose,
                            r.selected.get(i).copied().unwrap_or(false),
                        )
                    })
                    .collect();
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
                    visuals,
                )
            });

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
        let fill_w = ((score as f32 / target.max(1) as f32).min(1.0) * bar_w as f32) as i32;
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
            &format!("Hands {hands}  Disc {disc}  $ {money}  Deck {lib}  Discard {disc_n}"),
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

        for (i, (id, pose, sel)) in visuals.iter().enumerate() {
            let base_w = 100.0;
            let base_h = 140.0;
            let w = (base_w * pose.scale) as i32;
            let h = (base_h * pose.scale) as i32;
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
                rect(&mut self.pixels, WW, WH, x, y, w, h, (60, 50, 80));
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
            "1-8 select · P play · D discard · Esc pause  |  flow: .vstory  style: .vcss",
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
        let pause_sel = self.with_world(|w| w.pause_sel);
        let items = ["Continuar", "Menu principal"];
        for (i, item) in items.iter().enumerate() {
            let y = 250 + i as i32 * 40;
            let sel = i == pause_sel;
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
        let (result, score_line, last, result_sel) = self.with_world(|w| {
            let result = w.result;
            let (score_line, last) = w
                .run
                .as_ref()
                .map(|r| {
                    (
                        format!("Score {} / {}  ·  $ {}", r.score, r.target, r.money),
                        r.last.clone(),
                    )
                })
                .unwrap_or_else(|| (String::new(), String::new()));
            (result, score_line, last, w.result_sel)
        });
        let (title, col) = match result {
            Some(Outcome::WinBlind) => ("¡CIEGA SUPERADA!", (120, 255, 150)),
            Some(Outcome::LoseBlind) => ("CIEGA FALLIDA", (255, 120, 120)),
            Some(Outcome::RunClear) => ("¡RUN COMPLETA!", (255, 220, 80)),
            None => ("FIN", (200, 200, 200)),
        };
        text(&mut self.pixels, WW, WH, 40, 80, title, col, 3);
        if !score_line.is_empty() {
            text(
                &mut self.pixels,
                WW,
                WH,
                40,
                150,
                &score_line,
                (200, 200, 210),
                1,
            );
        }
        if !last.is_empty() {
            text(
                &mut self.pixels,
                WW,
                WH,
                40,
                180,
                &format!("Ultima: {last}"),
                (160, 180, 200),
                1,
            );
        }
        let items: &[&str] = match result {
            Some(Outcome::WinBlind) => &["Siguiente ciega", "Menu principal"],
            _ => &["Nueva run", "Menu principal"],
        };
        for (i, item) in items.iter().enumerate() {
            let y = 260 + i as i32 * 44;
            let sel = i == result_sel;
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
        // Letterbox scale — never stretch 1280×720 into a wrong aspect (looks broken)
        let present = if ww == WW && wh == WH {
            self.pixels.clone()
        } else {
            scale_letterbox(&self.pixels, WW, WH, ww, wh, 0x0a0614)
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
        let screen = self.with_world(|w| w.screen);
        let phase = self.wait_phase().unwrap_or_default();
        let dev = if self.live_dev.is_some() { " · DEV" } else { "" };
        let extra = if self.status_line.is_empty() {
            String::new()
        } else {
            format!(" · {}", self.status_line.chars().take(48).collect::<String>())
        };
        window.set_title(&format!(
            "Velvet Arcana — {:?} · story:{phase}{dev}{extra}",
            screen
        ));
    }

    fn on_key(&mut self, c: KeyCode, el: &ActiveEventLoop) {
        if self.with_world(|w| w.quit) {
            el.exit();
            return;
        }

        let screen = self.with_world(|w| w.screen);
        let phase = self.wait_phase();

        // Pause overlay is UI-only while story still waits on "play"
        if screen == Screen::Pause {
            match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.with_world_mut(|w| w.pause_sel = w.pause_sel.saturating_sub(1));
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.with_world_mut(|w| {
                        if w.pause_sel < 1 {
                            w.pause_sel += 1;
                        }
                    });
                }
                KeyCode::Enter | KeyCode::Space => {
                    let sel = self.with_world(|w| w.pause_sel);
                    if sel == 0 {
                        self.with_world_mut(|w| w.screen = Screen::Play);
                    } else {
                        self.with_world_mut(|w| w.to_title());
                        self.player
                            .variables_mut()
                            .set("menu_action", StoryValue::Int(-1));
                        // Jump story back to title loop
                        if phase.as_deref() == Some("play") {
                            self.player
                                .variables_mut()
                                .set("stakes_outcome", StoryValue::String("abort".into()));
                            self.player
                                .variables_mut()
                                .set("result_action", StoryValue::Int(1));
                            self.resume("play");
                            // if now on result, resume to title
                            if self.wait_phase().as_deref() == Some("result") {
                                self.resume("result");
                            }
                        }
                    }
                }
                KeyCode::Escape => {
                    self.with_world_mut(|w| w.screen = Screen::Play);
                }
                _ => {}
            }
            return;
        }

        match phase.as_deref() {
            Some("title") => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.with_world_mut(|w| w.menu_sel = w.menu_sel.saturating_sub(1));
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.with_world_mut(|w| {
                        if w.menu_sel + 1 < TITLE_ITEMS.len() {
                            w.menu_sel += 1;
                        }
                    });
                }
                KeyCode::Enter | KeyCode::Space => {
                    let sel = self.with_world(|w| w.menu_sel);
                    self.player
                        .variables_mut()
                        .set("menu_action", StoryValue::Int(sel as i64));
                    if sel == 4 {
                        self.with_world_mut(|w| w.quit = true);
                        el.exit();
                        return;
                    }
                    self.resume("title");
                }
                KeyCode::Escape => {
                    el.exit();
                }
                _ => {}
            },
            Some("submenu") => {
                if matches!(c, KeyCode::Enter | KeyCode::Space | KeyCode::Escape) {
                    self.resume("submenu");
                }
            }
            Some("blind") => match c {
                KeyCode::Enter | KeyCode::Space => self.resume("blind"),
                KeyCode::Escape => {
                    self.with_world_mut(|w| w.to_title());
                    self.player
                        .variables_mut()
                        .set("menu_action", StoryValue::Int(-1));
                    // force back: set outcome path carefully — jump via quit wait
                    // simpler: restart player from title scene not available; resume and
                    // let next play wait be skipped — use to_title and rebuild wait
                    self.resume("blind");
                    // if we entered play, immediately bail to result→title
                    if self.wait_phase().as_deref() == Some("play") {
                        self.player
                            .variables_mut()
                            .set("stakes_outcome", StoryValue::String("abort".into()));
                        self.player
                            .variables_mut()
                            .set("result_action", StoryValue::Int(1));
                        self.resume("play");
                        if self.wait_phase().as_deref() == Some("result") {
                            self.resume("result");
                        }
                    }
                }
                _ => {}
            },
            Some("play") => match c {
                KeyCode::Digit1 | KeyCode::Numpad1 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(0);
                        }
                    });
                }
                KeyCode::Digit2 | KeyCode::Numpad2 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(1);
                        }
                    });
                }
                KeyCode::Digit3 | KeyCode::Numpad3 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(2);
                        }
                    });
                }
                KeyCode::Digit4 | KeyCode::Numpad4 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(3);
                        }
                    });
                }
                KeyCode::Digit5 | KeyCode::Numpad5 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(4);
                        }
                    });
                }
                KeyCode::Digit6 | KeyCode::Numpad6 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(5);
                        }
                    });
                }
                KeyCode::Digit7 | KeyCode::Numpad7 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(6);
                        }
                    });
                }
                KeyCode::Digit8 | KeyCode::Numpad8 => {
                    self.with_world_mut(|w| {
                        if let Some(r) = w.run.as_mut() {
                            r.toggle(7);
                        }
                    });
                }
                KeyCode::KeyP | KeyCode::Enter => {
                    let outcome = self.with_world_mut(|w| {
                        let sheet = w.stylesheet.clone();
                        let stats = w.stats.clone();
                        w.run
                            .as_mut()
                            .and_then(|r| r.play_selected(&stats, Some(&sheet)))
                    });
                    if let Some(o) = outcome {
                        self.with_world_mut(|w| w.apply_outcome(o));
                        self.player.variables_mut().set(
                            "stakes_outcome",
                            StoryValue::String(o.as_str().into()),
                        );
                        self.resume("play");
                    }
                }
                KeyCode::KeyD => {
                    self.with_world_mut(|w| {
                        let sheet = w.stylesheet.clone();
                        if let Some(r) = w.run.as_mut() {
                            r.discard_selected(Some(&sheet));
                        }
                    });
                }
                KeyCode::Escape => {
                    self.with_world_mut(|w| {
                        w.screen = Screen::Pause;
                        w.pause_sel = 0;
                    });
                }
                _ => {}
            },
            Some("result") => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.with_world_mut(|w| w.result_sel = w.result_sel.saturating_sub(1));
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.with_world_mut(|w| {
                        if w.result_sel < 1 {
                            w.result_sel += 1;
                        }
                    });
                }
                KeyCode::Enter | KeyCode::Space => {
                    let (result, result_sel, next_ante) = self.with_world(|w| {
                        (
                            w.result,
                            w.result_sel,
                            w.run.as_ref().map(|r| r.ante + 1).unwrap_or(1),
                        )
                    });
                    self.player
                        .variables_mut()
                        .set("result_action", StoryValue::Int(result_sel as i64));
                    if matches!(result, Some(Outcome::WinBlind)) && result_sel == 0 {
                        self.with_world_mut(|w| w.start_blind(next_ante));
                    }
                    if result_sel == 1 {
                        self.with_world_mut(|w| w.to_title());
                    }
                    self.resume("result");
                }
                KeyCode::Escape => {
                    self.with_world_mut(|w| w.to_title());
                    self.player
                        .variables_mut()
                        .set("result_action", StoryValue::Int(1));
                    self.resume("result");
                }
                _ => {}
            },
            _ => {
                // fallback: old screen-based if story not waiting
                if matches!(c, KeyCode::Escape) {
                    el.exit();
                }
            }
        }

        if self.with_world(|w| w.quit) {
            el.exit();
        }
    }

    fn auto_tick(&mut self, el: &ActiveEventLoop) {
        let phase = self.wait_phase();
        match phase.as_deref() {
            Some("title") if self.hframes == 2 => {
                self.player
                    .variables_mut()
                    .set("menu_action", StoryValue::Int(0));
                self.resume("title");
            }
            Some("blind") if self.hframes >= 5 => {
                self.resume("blind");
            }
            Some("play") => {
                let outcome = self.with_world_mut(|w| {
                    if let Some(r) = w.run.as_mut() {
                        if !r.selected.iter().any(|s| *s) && !r.zones.hand.is_empty() {
                            r.selected[0] = true;
                            if r.zones.hand.len() > 1 {
                                r.selected[1] = true;
                            }
                        }
                        let sheet = w.stylesheet.clone();
                        let stats = w.stats.clone();
                        r.play_selected(&stats, Some(&sheet))
                    } else {
                        None
                    }
                });
                if let Some(o) = outcome {
                    self.with_world_mut(|w| w.apply_outcome(o));
                    self.player.variables_mut().set(
                        "stakes_outcome",
                        StoryValue::String(o.as_str().into()),
                    );
                    self.resume("play");
                }
            }
            Some("result") if self.hframes > 40 => {
                let money = self.with_world(|w| w.money);
                let result = self.with_world(|w| w.result);
                println!(
                    "headless result={:?} money={} frames={} vcss_rules={}",
                    result,
                    money,
                    self.hframes,
                    self.with_world(|w| w.stylesheet.rules.len())
                );
                el.exit();
            }
            Some("submenu") => self.resume("submenu"),
            _ => {}
        }
        if self.hframes > 600 {
            println!("headless timeout phase={:?}", self.wait_phase());
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

/// Fit source into dest preserving aspect ratio; fill bars with `void`.
fn scale_letterbox(src: &[u32], sw: u32, sh: u32, dw: u32, dh: u32, void: u32) -> Vec<u32> {
    let mut out = vec![void; (dw * dh) as usize];
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return out;
    }
    let scale = (dw as f32 / sw as f32).min(dh as f32 / sh as f32);
    let tw = ((sw as f32 * scale).round() as u32).max(1).min(dw);
    let th = ((sh as f32 * scale).round() as u32).max(1).min(dh);
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    let scaled = scale_nearest(src, sw, sh, tw, th);
    for y in 0..th {
        for x in 0..tw {
            out[((oy + y) * dw + (ox + x)) as usize] = scaled[(y * tw + x) as usize];
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
        // Live author reload (styles / images / story) — like HTML live refresh
        self.tick_live_dev();
        self.with_world_mut(|w| {
            if let Some(r) = w.run.as_mut() {
                r.tick_anims(dt);
            }
        });
        if self.headless {
            self.auto_tick(el);
        }
        if self.with_world(|w| w.quit) {
            el.exit();
            return;
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
    let args: Vec<String> = std::env::args().collect();
    let headless = args.iter().any(|a| a == "--headless");
    let dev = args.iter().any(|a| a == "--dev");
    let mut app = App::new(headless, dev)?;
    let (cards, deck, rules, fns, phase, watches) = {
        let w = app.host.world.lock().unwrap();
        (
            app.art.images.len(),
            w.deck_ids.len(),
            w.stylesheet.rules.len(),
            w.stylesheet.script.functions.len(),
            app.wait_phase(),
            app.live_dev.as_ref().map(|d| d.watch_count()).unwrap_or(0),
        )
    };
    println!(
        "Velvet Arcana LOCAL — story=.vstory style=.vcss cards={cards} deck={deck} vcss_rules={rules} vcss_fns={fns} phase={phase:?} dev={} watches={watches}",
        if dev { "on" } else { "off" }
    );
    if headless {
        println!("headless smoke…");
    } else if dev {
        println!(
            "DEV live-reload: edit data/styles/*.vcss, data/ui/*, data/art/*, data/story/*.vstory — no restart"
        );
    } else {
        println!("Lobby driven by data/story/main.vstory · motion by data/styles/casino.vcss");
        println!("Tip: cargo run -p velvet-stakes -- --dev   for live style/image reload");
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}
