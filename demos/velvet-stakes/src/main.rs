//! Velvet Arcana — **local** Balatro-style casino demo.
//!
//! Author flow: **`.vstory`** (`data/story/main.vstory`)  
//! Look + motion: **`.vcss`** (`data/styles/casino.vcss`, CSS + JS-lite `@script`)  
//! Rust host: window, paint, input → story resume + `stakes.*` / `style.*`
//!
//! Controls: menus ↑↓ Enter · Play: 1–8 select · P play · D discard · Market: Enter buy · R reroll · C continue · Esc
//! `--headless`: auto smoke  
//! `--dev`: live reload VS2 menu / styles / images / story (no full restart)

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
use velvet_cards::{validate_deck, DeckRules};
use velvet_script_layers::ScreenBlueprint;
use velvet_stakes::render::{fill, load_rgb, outline, panel, rect, text, ArtBank, RgbImage};
use velvet_stakes::title_font::{draw_font_text, measure_text, title_font, ui_font};
use velvet_stakes::ui::buttons::{hit_test_button, ButtonColumnLayout, MenuInteraction};
use velvet_stakes::ui::collection::{
    hit_test_collection, paint_collection_screen, CollectionAction, CollectionCardView,
    CollectionFilter, CollectionHit, CollectionInteraction, CollectionView,
};
use velvet_stakes::ui::gameplay::{
    hit_test_gameplay, paint_gameplay, GameplayAction, GameplayCardView, GameplayHit,
    GameplayInteraction, GameplayView,
};
use velvet_stakes::ui::market::{
    hit_test_market, paint_market, MarketAction, MarketHit, MarketInteraction, MarketOfferView,
    MarketView,
};
use velvet_stakes::ui::result::{
    hit_test_result, paint_result_screen, ResultInteraction, ResultKind, ResultView,
};
use velvet_stakes::ui::theme::{Theme, WH, WW};
use velvet_stakes::ui::{paint_options, paint_title_menu};
use velvet_stakes::{load_title_wordmark, reload_screen, ImageSlot, LiveDevSession, RgbaBuf};
use velvet_story::{StoryPlayer, StoryValue, StoryWait};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use catalog::{make_catalog_and_deck, CardKind};
use game::{Outcome, Screen, BLINDS};
use host::{StakesHost, StakesWorld, COLLECTION_DECK_MAX, COLLECTION_DECK_MIN};
use story_boot::boot_player;

struct App {
    host: Arc<StakesHost>,
    player: StoryPlayer,
    art: ArtBank,
    menu_bg: Option<RgbImage>,
    /// Clean gameplay plate; all HUD/card chrome is rendered live above it.
    gameplay_bg: Option<RgbImage>,
    /// Clean Night Market environment; all inventory and controls render live.
    market_bg: Option<RgbImage>,
    /// Elegant title wordmark (black keyed / alpha) — live-dev reloads this.
    logo_title: Option<RgbaBuf>,
    portrait: Option<RgbImage>,
    theme: Theme,
    /// Live author hot-reload (`--dev`).
    live_dev: Option<LiveDevSession>,
    /// Declarative title menu authored in Velvet Script 2.
    title_screen: ScreenBlueprint,
    /// Pointer-derived VCSS `:hover` state.
    menu_hovered: Option<usize>,
    /// Pointer-derived VCSS `:active` state.
    menu_pressed: Option<usize>,
    /// Pointer feedback for cards and the right-side gameplay actions.
    gameplay_interaction: GameplayInteraction,
    /// Pointer feedback for market stock and market actions.
    market_interaction: MarketInteraction,
    /// Pointer feedback for archive cards, filters, and deck-editing actions.
    collection_interaction: CollectionInteraction,
    /// Result action under the pointer.
    result_hovered: Option<usize>,
    /// Result action currently held down.
    result_pressed: Option<usize>,
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
    if p.join("menu_bg_city.png").exists() || p.join("menu_bg.jpg").exists() {
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
        let art =
            ArtBank::from_catalog_dir(&art_path, &["strike", "guard", "fireball", "focus", "bash"]);
        if art.images.len() < 5 {
            bail!(
                "missing card art in {} (found {})",
                art_path.display(),
                art.images.len()
            );
        }
        let ui = ui_dir(&root);
        let menu_bg =
            load_rgb(&ui.join("menu_bg_city.png")).or_else(|| load_rgb(&ui.join("menu_bg.jpg")));
        let gameplay_bg = load_rgb(&ui.join("gameplay_bg_night_broker.png"));
        let market_bg = load_rgb(&ui.join("night_market_bg.png"));
        // Authored copper wordmark — soft black-key + supersampled blit (smooth edges)
        let logo_title = load_title_wordmark(&ui.join("logo_title.png"));
        let portrait = load_rgb(&ui.join("portrait_collector.jpg"));
        let title_screen = reload_screen(&ui.join("main_menu.vel")).map_err(anyhow::Error::msg)?;
        if title_screen.buttons.is_empty() {
            bail!("VS2 title screen must declare at least one button");
        }

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
            gameplay_bg,
            market_bg,
            logo_title,
            portrait,
            theme: Theme::default(),
            live_dev,
            title_screen,
            menu_hovered: None,
            menu_pressed: None,
            gameplay_interaction: GameplayInteraction::default(),
            market_interaction: MarketInteraction::default(),
            collection_interaction: CollectionInteraction::default(),
            result_hovered: None,
            result_pressed: None,
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
            && apply.screen.is_none()
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
        if let Some(screen) = apply.screen {
            self.title_screen = screen;
            self.menu_hovered = None;
            self.menu_pressed = None;
            self.ensure_title_selection();
            self.status_line = "dev: VS2 menu live".into();
        }
        for (slot, buf) in apply.images {
            match slot {
                ImageSlot::MenuBg => self.menu_bg = Some(buf),
                ImageSlot::GameplayBg => self.gameplay_bg = Some(buf),
                ImageSlot::MarketBg => self.market_bg = Some(buf),
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
                    &self.title_screen,
                    sel,
                    MenuInteraction {
                        hovered: self.menu_hovered,
                        pressed: self.menu_pressed,
                    },
                    chips,
                    cry,
                    mult,
                );
            }
            Screen::Collection => self.paint_collection_screen(),
            Screen::Shop => {
                self.paint_market_screen();
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

    fn paint_collection_screen(&mut self) {
        const IDS: [&str; 5] = ["strike", "guard", "fireball", "focus", "bash"];
        let (
            vault,
            crystals,
            multiplier,
            deck_count,
            total_chips,
            extra_mult,
            kind_counts,
            filter,
            selected,
            status,
            all_cards,
        ) = self.with_world(|world| {
            let mut total_chips = 0;
            let mut extra_mult = 0;
            let mut kind_counts = [0usize; 4];
            for id in &world.starter_deck_ids {
                let Some(card) = world.stats.get(id) else {
                    continue;
                };
                total_chips += card.chips;
                extra_mult += card.mult.saturating_sub(1);
                kind_counts[kind_index(card.kind)] += 1;
            }
            let all_cards = IDS
                .iter()
                .filter_map(|id| {
                    let card = world.stats.get(*id)?;
                    let owned = world
                        .starter_deck_ids
                        .iter()
                        .filter(|deck_id| deck_id.as_str() == *id)
                        .count();
                    Some((
                        card.id.clone(),
                        card.name.clone(),
                        card.kind,
                        card.kind.label().to_string(),
                        card.rules_text(),
                        owned,
                    ))
                })
                .collect::<Vec<_>>();
            (
                world.meta_chips,
                world.meta_crystals,
                world.meta_mult,
                world.starter_deck_ids.len(),
                total_chips,
                extra_mult,
                kind_counts,
                CollectionFilter::from_index(world.collection_filter),
                world.collection_sel,
                world.collection_status.clone(),
                all_cards,
            )
        });

        let visible = all_cards
            .iter()
            .filter(|(_, _, kind, _, _, _)| collection_filter_matches(filter, *kind))
            .cloned()
            .collect::<Vec<_>>();
        let visible_views = visible
            .iter()
            .map(|(id, name, _, kind, rules, owned)| CollectionCardView {
                id,
                name,
                kind,
                rules,
                owned: *owned,
            })
            .collect::<Vec<_>>();
        let composition_views = all_cards
            .iter()
            .map(|(id, name, _, kind, rules, owned)| CollectionCardView {
                id,
                name,
                kind,
                rules,
                owned: *owned,
            })
            .collect::<Vec<_>>();
        let view = CollectionView {
            vault,
            crystals,
            multiplier,
            deck_count,
            deck_limit: COLLECTION_DECK_MAX,
            min_deck: COLLECTION_DECK_MIN,
            total_chips,
            extra_mult,
            kind_counts,
            filter,
            selected_card: selected.min(visible_views.len().saturating_sub(1)),
            status: &status,
            cards: &visible_views,
            composition: &composition_views,
        };
        paint_collection_screen(
            &mut self.pixels,
            &self.theme,
            self.menu_bg.as_ref(),
            &self.art,
            &view,
            self.collection_interaction,
        );
    }

    fn paint_market_screen(&mut self) {
        let (
            vault,
            money,
            score,
            next_target,
            next_round,
            rounds_total,
            deck_count,
            reroll_cost,
            in_run,
            selected_offer,
            status,
            offers,
        ) = self.with_world(|world| {
            let in_run = world.run.is_some() && matches!(world.result, Some(Outcome::WinBlind));
            let (score, next_target, next_round) = world
                .run
                .as_ref()
                .map(|run| {
                    let next_index = (run.ante + 1).min(BLINDS.len().saturating_sub(1));
                    (run.score, BLINDS[next_index].target, next_index as u32 + 1)
                })
                .unwrap_or((0, BLINDS[0].target, 1));
            let offers = world
                .market_offers
                .iter()
                .map(|offer| {
                    let stat = world.stats.get(&offer.card_id);
                    (
                        offer.card_id.clone(),
                        stat.map(|card| card.name.clone())
                            .unwrap_or_else(|| offer.card_id.clone()),
                        stat.map(|card| card.kind.label().to_string())
                            .unwrap_or_else(|| "CARD".into()),
                        stat.map(|card| card.rules_text()).unwrap_or_default(),
                        offer.price,
                        offer.bought,
                        in_run && !offer.bought && world.money >= offer.price,
                    )
                })
                .collect::<Vec<_>>();
            (
                world.meta_chips,
                world.money,
                score,
                next_target,
                next_round,
                BLINDS.len() as u32,
                world.deck_ids.len(),
                world.market_reroll_cost(),
                in_run,
                world.market_sel,
                world.market_status.clone(),
                offers,
            )
        });
        let offer_views = offers
            .iter()
            .map(
                |(id, name, kind, rules, price, bought, affordable)| MarketOfferView {
                    id,
                    name,
                    kind,
                    rules,
                    price: *price,
                    bought: *bought,
                    affordable: *affordable,
                },
            )
            .collect::<Vec<_>>();
        let view = MarketView {
            vault,
            money,
            score,
            next_target,
            next_round,
            rounds_total,
            deck_count,
            reroll_cost,
            in_run,
            selected_offer,
            status: &status,
            offers: &offer_views,
        };
        paint_market(
            &mut self.pixels,
            &self.theme,
            self.market_bg.as_ref().or(self.menu_bg.as_ref()),
            &self.art,
            &view,
            self.market_interaction,
        );
    }

    fn paint_blind(&mut self) {
        let has_run = self.with_world(|world| world.run.is_some());
        if has_run {
            self.paint_play();
        } else {
            fill(&mut self.pixels, WW, WH, self.theme.void);
        }

        panel(
            &mut self.pixels,
            WW,
            WH,
            0,
            0,
            WW as i32,
            WH as i32,
            (3, 2, 10),
            0.72,
        );
        let (name, target, hands, discards, reward, round, rounds_total) =
            self.with_world(|world| {
                world
                    .run
                    .as_ref()
                    .map(|run| {
                        (
                            run.blind_name.to_uppercase(),
                            run.target,
                            run.hands_left,
                            run.discards_left,
                            run.blind_reward(),
                            run.round_number(),
                            run.round_count(),
                        )
                    })
                    .unwrap_or_else(|| ("UNKNOWN BLIND".into(), 0, 0, 0, 0, 1, 1))
            });

        const X: i32 = 330;
        const Y: i32 = 142;
        const W: i32 = 620;
        const H: i32 = 436;
        panel(
            &mut self.pixels,
            WW,
            WH,
            X + 7,
            Y + 9,
            W,
            H,
            (0, 0, 4),
            0.78,
        );
        panel(&mut self.pixels, WW, WH, X, Y, W, H, (8, 5, 18), 0.96);
        outline(&mut self.pixels, WW, WH, X, Y, W, H, (166, 101, 58), 2);
        outline(
            &mut self.pixels,
            WW,
            WH,
            X + 8,
            Y + 8,
            W - 16,
            H - 16,
            (79, 48, 59),
            1,
        );

        let centered_title = |pixels: &mut [u32], value: &str, baseline: f32, px: f32, color| {
            if let Some(font) = title_font() {
                let width = measure_text(font, value, px);
                draw_font_text(
                    pixels,
                    font,
                    WW as f32 * 0.5 - width * 0.5,
                    baseline,
                    value,
                    px,
                    color,
                    1.0,
                );
            } else {
                let scale = (px / 8.0).round().max(1.0) as i32;
                let width = value.chars().count() as i32 * 6 * scale;
                text(
                    pixels,
                    WW,
                    WH,
                    WW as i32 / 2 - width / 2,
                    baseline as i32 - 7 * scale,
                    value,
                    color,
                    scale,
                );
            }
        };
        let centered_ui = |pixels: &mut [u32], value: &str, baseline: f32, px: f32, color| {
            if let Some(font) = ui_font() {
                let width = measure_text(font, value, px);
                draw_font_text(
                    pixels,
                    font,
                    WW as f32 * 0.5 - width * 0.5,
                    baseline,
                    value,
                    px,
                    color,
                    1.0,
                );
            } else {
                let scale = (px / 8.0).round().max(1.0) as i32;
                let width = value.chars().count() as i32 * 6 * scale;
                text(
                    pixels,
                    WW,
                    WH,
                    WW as i32 / 2 - width / 2,
                    baseline as i32 - 7 * scale,
                    value,
                    color,
                    scale,
                );
            }
        };

        centered_ui(
            &mut self.pixels,
            &format!("ROUND {round} / {rounds_total}  ·  NEW CHALLENGE"),
            184.0,
            12.0,
            (226, 151, 91),
        );
        centered_title(&mut self.pixels, &name, 239.0, 38.0, self.theme.gold_soft);
        rect(
            &mut self.pixels,
            WW,
            WH,
            X + 112,
            Y + 117,
            W - 224,
            1,
            (166, 101, 58),
        );
        centered_ui(
            &mut self.pixels,
            "SCORE REQUIRED",
            292.0,
            11.0,
            (190, 132, 124),
        );
        centered_title(
            &mut self.pixels,
            &target.to_string(),
            348.0,
            42.0,
            (242, 221, 232),
        );

        for (index, (label, value)) in [
            ("HANDS", hands.to_string()),
            ("DISCARDS", discards.to_string()),
            ("REWARD", format!("${reward}")),
        ]
        .into_iter()
        .enumerate()
        {
            let stat_x = X + 92 + index as i32 * 148;
            panel(
                &mut self.pixels,
                WW,
                WH,
                stat_x,
                371,
                128,
                65,
                (19, 10, 31),
                0.94,
            );
            outline(
                &mut self.pixels,
                WW,
                WH,
                stat_x,
                371,
                128,
                65,
                (83, 48, 66),
                1,
            );
            if let Some(font) = ui_font() {
                let label_width = measure_text(font, label, 9.0);
                draw_font_text(
                    &mut self.pixels,
                    font,
                    stat_x as f32 + 64.0 - label_width * 0.5,
                    393.0,
                    label,
                    9.0,
                    (190, 132, 124),
                    1.0,
                );
                let value_width = measure_text(font, &value, 18.0);
                draw_font_text(
                    &mut self.pixels,
                    font,
                    stat_x as f32 + 64.0 - value_width * 0.5,
                    421.0,
                    &value,
                    18.0,
                    self.theme.text,
                    1.0,
                );
            }
        }

        panel(
            &mut self.pixels,
            WW,
            WH,
            455,
            466,
            370,
            68,
            (63, 16, 86),
            0.98,
        );
        outline(
            &mut self.pixels,
            WW,
            WH,
            455,
            466,
            370,
            68,
            (226, 151, 91),
            2,
        );
        outline(
            &mut self.pixels,
            WW,
            WH,
            463,
            474,
            354,
            52,
            (121, 48, 126),
            1,
        );
        centered_title(
            &mut self.pixels,
            "ENTER TABLE",
            502.0,
            23.0,
            self.theme.gold_soft,
        );
        centered_ui(
            &mut self.pixels,
            "ENTER / SPACE",
            521.0,
            8.0,
            (222, 210, 235),
        );
        centered_ui(
            &mut self.pixels,
            "ESC  ·  BACK TO LOBBY",
            558.0,
            9.0,
            self.theme.muted,
        );
    }
    fn paint_play(&mut self) {
        let hovered_card = self.gameplay_interaction.hovered_card;
        let (
            score,
            target,
            progress,
            hands,
            discards,
            money,
            blind,
            draw_count,
            discard_count,
            round,
            rounds_total,
            ante,
            last_message,
            preview,
            cards,
        ) = self.with_world(|world| {
            let run = world
                .run
                .as_ref()
                .expect("play screen requires an active run");
            let preview = run.preview_score(&world.stats);
            let cards = run
                .visuals
                .iter()
                .enumerate()
                .map(|(index, visual)| {
                    let stat = world.stats.get(&visual.id);
                    (
                        visual.id.clone(),
                        stat.map(|card| card.name.clone())
                            .unwrap_or_else(|| visual.id.clone()),
                        stat.map(|card| card.kind.label()).unwrap_or("CARD"),
                        stat.map(|card| card.chips).unwrap_or(0),
                        stat.map(|card| card.mult).unwrap_or(1),
                        run.selected.get(index).copied().unwrap_or(false),
                        visual.pose.opacity,
                        visual.pose.scale,
                    )
                })
                .collect::<Vec<_>>();
            (
                run.score,
                run.target,
                run.progress_ratio(),
                run.hands_left,
                run.discards_left,
                run.money,
                run.blind_name.clone(),
                run.zones.library.len(),
                run.zones.discard.len(),
                run.round_number() as u32,
                run.round_count() as u32,
                run.ante as u32 + 1,
                if let Some(card) = hovered_card
                    .and_then(|index| run.zones.hand.get(index))
                    .and_then(|id| world.stats.get(id))
                {
                    format!("{}  ·  {}", card.name, card.rules_text())
                } else if run.last.is_empty() {
                    format!("{} POINTS TO CLEAR", run.score_remaining())
                } else {
                    run.last.clone()
                },
                preview,
                cards,
            )
        });

        let hand = cards
            .iter()
            .map(
                |(id, name, kind, chips, mult, selected, opacity, scale)| GameplayCardView {
                    id,
                    name,
                    kind,
                    chips: *chips,
                    mult: *mult,
                    selected: *selected,
                    opacity: *opacity,
                    scale: *scale,
                },
            )
            .collect::<Vec<_>>();
        let opponent_name = match round {
            1 => "THE VIOLET CROUPIER",
            2 => "NIGHT SYNDICATE BROKER",
            _ => "THE OBSIDIAN HOUSE",
        };
        let view = GameplayView {
            chips: preview.chips,
            multiplier: preview.mult,
            score,
            target,
            progress,
            round,
            rounds_total,
            ante,
            risk: round.min(u8::MAX as u32) as u8,
            risk_total: rounds_total.min(u8::MAX as u32) as u8,
            player_name: "THE COLLECTOR",
            player_rank: "CHAOS DEALER",
            hands_left: hands,
            discards_left: discards,
            money,
            draw_count,
            discard_count,
            opponent_name,
            opponent_rank: &blind,
            preview_label: &preview.label,
            preview_chips: preview.chips,
            preview_mult: preview.mult,
            preview_total: preview.total,
            last_message: &last_message,
            hand: &hand,
        };
        paint_gameplay(
            &mut self.pixels,
            &self.theme,
            self.gameplay_bg.as_ref().or(self.menu_bg.as_ref()),
            self.portrait.as_ref(),
            &self.art,
            &view,
            self.gameplay_interaction,
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
        rect(&mut self.pixels, WW, WH, 260, 150, 440, 200, (30, 26, 48));
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
        let (kind, score, target, cash, deck_count, round, rounds_total, last, selected) = self
            .with_world(|world| {
                let kind = match world.result {
                    Some(Outcome::WinBlind) => ResultKind::BlindClear,
                    Some(Outcome::LoseBlind) => ResultKind::Defeat,
                    Some(Outcome::RunClear) => ResultKind::RunClear,
                    None => ResultKind::Aborted,
                };
                let (score, target, cash, round, last) = world
                    .run
                    .as_ref()
                    .map(|run| {
                        (
                            run.score,
                            run.target,
                            run.money,
                            run.round_number() as u32,
                            run.last.clone(),
                        )
                    })
                    .unwrap_or((0, 0, world.money, 0, String::new()));
                (
                    kind,
                    score,
                    target,
                    cash,
                    world.deck_ids.len(),
                    round,
                    BLINDS.len() as u32,
                    last,
                    world.result_sel,
                )
            });
        let view = ResultView {
            kind,
            score,
            target,
            cash,
            deck_count,
            round,
            rounds_total,
            last_combo: &last,
        };
        paint_result_screen(
            &mut self.pixels,
            &self.theme,
            self.gameplay_bg.as_ref().or(self.menu_bg.as_ref()),
            &view,
            ResultInteraction {
                selected,
                hovered: self.result_hovered,
                pressed: self.result_pressed,
            },
        );
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
        let dev = if self.live_dev.is_some() {
            " · DEV"
        } else {
            ""
        };
        let extra = if self.status_line.is_empty() {
            String::new()
        } else {
            format!(
                " · {}",
                self.status_line.chars().take(48).collect::<String>()
            )
        };
        window.set_title(&format!(
            "Velvet Arcana — {:?} · story:{phase}{dev}{extra}",
            screen
        ));
    }

    fn title_is_interactive(&self) -> bool {
        self.with_world(|world| world.screen == Screen::Title)
            && self.wait_phase().as_deref() == Some("title")
    }

    fn ensure_title_selection(&self) {
        let current = self.with_world(|world| world.menu_sel);
        if self
            .title_screen
            .buttons
            .get(current)
            .is_some_and(|button| button.enabled)
        {
            return;
        }
        if let Some(index) = self
            .title_screen
            .buttons
            .iter()
            .position(|button| button.enabled)
        {
            self.with_world_mut(|world| world.menu_sel = index);
        }
    }

    fn move_title_selection(&self, direction: i32) {
        let count = self.title_screen.buttons.len();
        if count == 0 {
            return;
        }
        let current = self.with_world(|world| world.menu_sel).min(count - 1);
        for distance in 1..=count {
            let index =
                (current as i32 + direction * distance as i32).rem_euclid(count as i32) as usize;
            if self.title_screen.buttons[index].enabled {
                self.with_world_mut(|world| world.menu_sel = index);
                break;
            }
        }
    }

    fn activate_title_index(&mut self, index: usize, el: &ActiveEventLoop) {
        let Some(button) = self.title_screen.buttons.get(index) else {
            return;
        };
        if !button.enabled {
            return;
        }
        let action = button.action.clone();
        self.with_world_mut(|world| world.menu_sel = index);
        self.player
            .variables_mut()
            .set("menu_action", StoryValue::String(action));
        self.resume("title");
        self.menu_hovered = None;
        self.menu_pressed = None;
        if self.with_world(|world| world.quit) {
            el.exit();
        }
    }

    fn activate_title_selected(&mut self, el: &ActiveEventLoop) {
        let selected = self.with_world(|world| world.menu_sel);
        self.activate_title_index(selected, el);
    }

    fn title_hotkey_index(&self, code: KeyCode) -> Option<usize> {
        self.title_screen
            .buttons
            .iter()
            .position(|button| button.enabled && hotkey_matches(code, button.hotkey.as_str()))
    }

    fn pointer_to_logical(&self, x: f64, y: f64) -> Option<(i32, i32)> {
        let size = self.window.as_ref()?.inner_size();
        let dw = size.width.max(1);
        let dh = size.height.max(1);
        let scale = (dw as f64 / WW as f64).min(dh as f64 / WH as f64);
        let tw = ((WW as f64 * scale).round() as u32).max(1).min(dw);
        let th = ((WH as f64 * scale).round() as u32).max(1).min(dh);
        let ox = (dw - tw) as f64 * 0.5;
        let oy = (dh - th) as f64 * 0.5;
        if x < ox || y < oy || x >= ox + tw as f64 || y >= oy + th as f64 {
            return None;
        }
        Some((
            ((x - ox) * WW as f64 / tw as f64).floor() as i32,
            ((y - oy) * WH as f64 / th as f64).floor() as i32,
        ))
    }

    fn title_button_at(&self, x: f64, y: f64) -> Option<usize> {
        if !self.title_is_interactive() {
            return None;
        }
        let (x, y) = self.pointer_to_logical(x, y)?;
        let sheet = self.with_world(|world| world.stylesheet.clone());
        let layout = ButtonColumnLayout::from_style(&sheet, &self.title_screen);
        hit_test_button(&sheet, &self.title_screen, &layout, x, y).filter(|index| {
            self.title_screen
                .buttons
                .get(*index)
                .is_some_and(|button| button.enabled)
        })
    }

    fn collection_visible_ids(&self) -> Vec<String> {
        const IDS: [&str; 5] = ["strike", "guard", "fireball", "focus", "bash"];
        self.with_world(|world| {
            let filter = CollectionFilter::from_index(world.collection_filter);
            IDS.iter()
                .filter_map(|id| {
                    let card = world.stats.get(*id)?;
                    collection_filter_matches(filter, card.kind).then(|| (*id).to_string())
                })
                .collect()
        })
    }

    fn collection_hit_at(&self, x: f64, y: f64) -> Option<CollectionHit> {
        if self.with_world(|world| world.screen) != Screen::Collection {
            return None;
        }
        let (x, y) = self.pointer_to_logical(x, y)?;
        hit_test_collection(self.collection_visible_ids().len(), x, y)
    }

    fn hovered_collection_hit(&self) -> Option<CollectionHit> {
        self.collection_interaction
            .hovered_card
            .map(CollectionHit::Card)
            .or_else(|| {
                self.collection_interaction
                    .hovered_filter
                    .map(CollectionHit::Filter)
            })
            .or_else(|| {
                self.collection_interaction
                    .hovered_action
                    .map(|action| match action {
                        CollectionAction::Add => CollectionHit::Add,
                        CollectionAction::Remove => CollectionHit::Remove,
                        CollectionAction::Back => CollectionHit::Back,
                    })
            })
    }

    fn pressed_collection_hit(&self) -> Option<CollectionHit> {
        self.collection_interaction
            .pressed_card
            .map(CollectionHit::Card)
            .or_else(|| {
                self.collection_interaction
                    .pressed_filter
                    .map(CollectionHit::Filter)
            })
            .or_else(|| {
                self.collection_interaction
                    .pressed_action
                    .map(|action| match action {
                        CollectionAction::Add => CollectionHit::Add,
                        CollectionAction::Remove => CollectionHit::Remove,
                        CollectionAction::Back => CollectionHit::Back,
                    })
            })
    }

    fn set_collection_hover(&mut self, hit: Option<CollectionHit>) {
        self.collection_interaction.hovered_card = match hit {
            Some(CollectionHit::Card(index)) => Some(index),
            _ => None,
        };
        self.collection_interaction.hovered_filter = match hit {
            Some(CollectionHit::Filter(filter)) => Some(filter),
            _ => None,
        };
        self.collection_interaction.hovered_action = match hit {
            Some(CollectionHit::Add) => Some(CollectionAction::Add),
            Some(CollectionHit::Remove) => Some(CollectionAction::Remove),
            Some(CollectionHit::Back) => Some(CollectionAction::Back),
            _ => None,
        };
        if let Some(CollectionHit::Card(index)) = hit {
            self.with_world_mut(|world| world.collection_sel = index);
        }
    }

    fn set_collection_pressed(&mut self, hit: Option<CollectionHit>) {
        self.collection_interaction.pressed_card = match hit {
            Some(CollectionHit::Card(index)) => Some(index),
            _ => None,
        };
        self.collection_interaction.pressed_filter = match hit {
            Some(CollectionHit::Filter(filter)) => Some(filter),
            _ => None,
        };
        self.collection_interaction.pressed_action = match hit {
            Some(CollectionHit::Add) => Some(CollectionAction::Add),
            Some(CollectionHit::Remove) => Some(CollectionAction::Remove),
            Some(CollectionHit::Back) => Some(CollectionAction::Back),
            _ => None,
        };
    }

    fn move_collection_selection(&mut self, direction: i32) {
        let count = self.collection_visible_ids().len();
        if count == 0 {
            return;
        }
        self.collection_interaction.hovered_card = None;
        self.with_world_mut(|world| {
            world.collection_sel =
                (world.collection_sel as i32 + direction).rem_euclid(count as i32) as usize;
        });
    }

    fn cycle_collection_filter(&mut self, direction: i32) {
        self.collection_interaction = CollectionInteraction::default();
        self.with_world_mut(|world| {
            world.collection_filter = (world.collection_filter as i32 + direction)
                .rem_euclid(CollectionFilter::ALL.len() as i32)
                as usize;
            world.collection_sel = 0;
            let label = CollectionFilter::from_index(world.collection_filter).label();
            world.collection_status = format!("{label} filter active.");
        });
    }

    fn selected_collection_id(&self) -> Option<String> {
        let ids = self.collection_visible_ids();
        let selected = self.with_world(|world| world.collection_sel);
        ids.get(selected.min(ids.len().saturating_sub(1))).cloned()
    }

    fn activate_collection_hit(&mut self, hit: CollectionHit) {
        match hit {
            CollectionHit::Card(index) => {
                self.with_world_mut(|world| {
                    world.collection_sel = index;
                    world.collection_status =
                        "Card focused. Use ADD COPY or REMOVE COPY to edit the deck.".into();
                });
            }
            CollectionHit::Filter(filter) => {
                self.collection_interaction.hovered_card = None;
                self.with_world_mut(|world| {
                    world.collection_filter = filter.index();
                    world.collection_sel = 0;
                    world.collection_status = format!("{} filter active.", filter.label());
                });
            }
            CollectionHit::Add => {
                if let Some(id) = self.selected_collection_id() {
                    self.with_world_mut(|world| {
                        world.add_collection_card(&id);
                    });
                }
            }
            CollectionHit::Remove => {
                if let Some(id) = self.selected_collection_id() {
                    self.with_world_mut(|world| {
                        world.remove_collection_card(&id);
                    });
                }
            }
            CollectionHit::Back => {
                if self.wait_phase().as_deref() == Some("collection") {
                    self.collection_interaction = CollectionInteraction::default();
                    self.resume("collection");
                }
            }
        }
    }

    fn market_hit_at(&self, x: f64, y: f64) -> Option<MarketHit> {
        if self.with_world(|world| world.screen) != Screen::Shop {
            return None;
        }
        let (x, y) = self.pointer_to_logical(x, y)?;
        let (offer_count, in_run) = self.with_world(|world| {
            (
                world.market_offers.len(),
                world.run.is_some() && matches!(world.result, Some(Outcome::WinBlind)),
            )
        });
        hit_test_market(offer_count, in_run, x, y)
    }

    fn result_hit_at(&self, x: f64, y: f64) -> Option<usize> {
        if self.with_world(|world| world.screen) != Screen::Result {
            return None;
        }
        let (x, y) = self.pointer_to_logical(x, y)?;
        hit_test_result(x, y)
    }

    fn activate_result_index(&mut self, index: usize) {
        if index >= 2 || self.wait_phase().as_deref() != Some("result") {
            return;
        }
        self.with_world_mut(|world| world.result_sel = index);
        self.player
            .variables_mut()
            .set("result_action", StoryValue::Int(index as i64));
        if index == 1 {
            self.with_world_mut(|world| world.return_to_title());
        }
        self.result_hovered = None;
        self.result_pressed = None;
        self.resume("result");
    }

    fn hovered_market_hit(&self) -> Option<MarketHit> {
        self.market_interaction
            .hovered_offer
            .map(MarketHit::Offer)
            .or_else(|| {
                self.market_interaction
                    .hovered_action
                    .map(|action| match action {
                        MarketAction::Reroll => MarketHit::Reroll,
                        MarketAction::Continue => MarketHit::Continue,
                        MarketAction::Back => MarketHit::Back,
                    })
            })
    }

    fn pressed_market_hit(&self) -> Option<MarketHit> {
        self.market_interaction
            .pressed_offer
            .map(MarketHit::Offer)
            .or_else(|| {
                self.market_interaction
                    .pressed_action
                    .map(|action| match action {
                        MarketAction::Reroll => MarketHit::Reroll,
                        MarketAction::Continue => MarketHit::Continue,
                        MarketAction::Back => MarketHit::Back,
                    })
            })
    }

    fn set_market_hover(&mut self, hit: Option<MarketHit>) {
        self.market_interaction.hovered_offer = match hit {
            Some(MarketHit::Offer(index)) => Some(index),
            _ => None,
        };
        self.market_interaction.hovered_action = match hit {
            Some(MarketHit::Reroll) => Some(MarketAction::Reroll),
            Some(MarketHit::Continue) => Some(MarketAction::Continue),
            Some(MarketHit::Back) => Some(MarketAction::Back),
            _ => None,
        };
        if let Some(MarketHit::Offer(index)) = hit {
            self.with_world_mut(|world| world.market_sel = index);
        }
    }

    fn set_market_pressed(&mut self, hit: Option<MarketHit>) {
        self.market_interaction.pressed_offer = match hit {
            Some(MarketHit::Offer(index)) => Some(index),
            _ => None,
        };
        self.market_interaction.pressed_action = match hit {
            Some(MarketHit::Reroll) => Some(MarketAction::Reroll),
            Some(MarketHit::Continue) => Some(MarketAction::Continue),
            Some(MarketHit::Back) => Some(MarketAction::Back),
            _ => None,
        };
    }

    fn move_market_selection(&self, direction: i32) {
        self.with_world_mut(|world| {
            let count = world.market_offers.len();
            if count == 0 {
                return;
            }
            world.market_sel =
                (world.market_sel as i32 + direction).rem_euclid(count as i32) as usize;
        });
    }

    fn activate_market_hit(&mut self, hit: MarketHit) {
        match hit {
            MarketHit::Offer(index) => {
                self.with_world_mut(|world| {
                    world.buy_market_offer(index);
                });
            }
            MarketHit::Reroll => {
                self.with_world_mut(|world| {
                    world.reroll_market();
                });
            }
            MarketHit::Continue => {
                if self.wait_phase().as_deref() == Some("market") {
                    self.market_interaction = MarketInteraction::default();
                    self.resume("market");
                }
            }
            MarketHit::Back => {
                if self.wait_phase().as_deref() == Some("submenu") {
                    self.market_interaction = MarketInteraction::default();
                    self.resume("submenu");
                }
            }
        }
    }

    fn gameplay_hit_at(&self, x: f64, y: f64) -> Option<GameplayHit> {
        if self.with_world(|world| world.screen) != Screen::Play {
            return None;
        }
        let (x, y) = self.pointer_to_logical(x, y)?;
        let (card_count, selected) = self.with_world(|world| {
            world
                .run
                .as_ref()
                .map(|run| (run.zones.hand.len(), run.selected.clone()))
                .unwrap_or_default()
        });
        hit_test_gameplay(card_count, &selected, x, y)
    }

    fn hovered_gameplay_hit(&self) -> Option<GameplayHit> {
        self.gameplay_interaction
            .hovered_card
            .map(GameplayHit::Card)
            .or_else(|| {
                self.gameplay_interaction
                    .hovered_action
                    .map(|action| match action {
                        GameplayAction::Play => GameplayHit::Play,
                        GameplayAction::Discard => GameplayHit::Discard,
                        GameplayAction::Pause => GameplayHit::Pause,
                    })
            })
    }

    fn pressed_gameplay_hit(&self) -> Option<GameplayHit> {
        self.gameplay_interaction
            .pressed_card
            .map(GameplayHit::Card)
            .or_else(|| {
                self.gameplay_interaction
                    .pressed_action
                    .map(|action| match action {
                        GameplayAction::Play => GameplayHit::Play,
                        GameplayAction::Discard => GameplayHit::Discard,
                        GameplayAction::Pause => GameplayHit::Pause,
                    })
            })
    }

    fn set_gameplay_hover(&mut self, hit: Option<GameplayHit>) {
        self.gameplay_interaction.hovered_card = match hit {
            Some(GameplayHit::Card(index)) => Some(index),
            _ => None,
        };
        self.gameplay_interaction.hovered_action = match hit {
            Some(GameplayHit::Play) => Some(GameplayAction::Play),
            Some(GameplayHit::Discard) => Some(GameplayAction::Discard),
            Some(GameplayHit::Pause) => Some(GameplayAction::Pause),
            _ => None,
        };
    }

    fn set_gameplay_pressed(&mut self, hit: Option<GameplayHit>) {
        self.gameplay_interaction.pressed_card = match hit {
            Some(GameplayHit::Card(index)) => Some(index),
            _ => None,
        };
        self.gameplay_interaction.pressed_action = match hit {
            Some(GameplayHit::Play) => Some(GameplayAction::Play),
            Some(GameplayHit::Discard) => Some(GameplayAction::Discard),
            Some(GameplayHit::Pause) => Some(GameplayAction::Pause),
            _ => None,
        };
    }

    fn play_current_hand(&mut self) {
        let outcome = self.with_world_mut(|world| {
            let sheet = world.stylesheet.clone();
            let stats = world.stats.clone();
            world
                .run
                .as_mut()
                .and_then(|run| run.play_selected(&stats, Some(&sheet)))
        });
        if let Some(outcome) = outcome {
            self.with_world_mut(|world| world.apply_outcome(outcome));
            self.player.variables_mut().set(
                "stakes_outcome",
                StoryValue::String(outcome.as_str().into()),
            );
            self.resume("play");
        }
    }

    fn discard_current_hand(&mut self) {
        self.with_world_mut(|world| {
            let sheet = world.stylesheet.clone();
            if let Some(run) = world.run.as_mut() {
                run.discard_selected(Some(&sheet));
            }
        });
    }

    fn activate_gameplay_hit(&mut self, hit: GameplayHit) {
        match hit {
            GameplayHit::Card(index) => self.with_world_mut(|world| {
                if let Some(run) = world.run.as_mut() {
                    run.toggle(index);
                }
            }),
            GameplayHit::Play => {
                if self.with_world(|world| world.run.as_ref().is_some_and(|run| run.can_play())) {
                    self.play_current_hand();
                }
            }
            GameplayHit::Discard => {
                if self.with_world(|world| world.run.as_ref().is_some_and(|run| run.can_discard()))
                {
                    self.discard_current_hand();
                }
            }
            GameplayHit::Pause => {
                self.with_world_mut(|world| {
                    world.screen = Screen::Pause;
                    world.pause_sel = 0;
                });
            }
        }
    }

    fn on_cursor_moved(&mut self, x: f64, y: f64) {
        self.menu_hovered = self.title_button_at(x, y);
        if let Some(index) = self.menu_hovered {
            self.with_world_mut(|world| world.menu_sel = index);
        }
        let gameplay_hit = self.gameplay_hit_at(x, y);
        self.set_gameplay_hover(gameplay_hit);
        let market_hit = self.market_hit_at(x, y);
        self.set_market_hover(market_hit);
        let collection_hit = self.collection_hit_at(x, y);
        self.set_collection_hover(collection_hit);
        self.result_hovered = self.result_hit_at(x, y);
        if let Some(index) = self.result_hovered {
            self.with_world_mut(|world| world.result_sel = index);
        }
    }

    fn on_primary_pointer(&mut self, state: ElementState, el: &ActiveEventLoop) {
        if self.with_world(|world| world.screen) == Screen::Collection {
            match state {
                ElementState::Pressed => {
                    self.set_collection_pressed(self.hovered_collection_hit());
                }
                ElementState::Released => {
                    let activate = self
                        .pressed_collection_hit()
                        .filter(|pressed| Some(*pressed) == self.hovered_collection_hit());
                    self.set_collection_pressed(None);
                    if let Some(hit) = activate {
                        self.activate_collection_hit(hit);
                    }
                }
            }
            return;
        }
        if self.with_world(|world| world.screen) == Screen::Result {
            match state {
                ElementState::Pressed => {
                    self.result_pressed = self.result_hovered;
                }
                ElementState::Released => {
                    let activate = self
                        .result_pressed
                        .filter(|pressed| Some(*pressed) == self.result_hovered);
                    self.result_pressed = None;
                    if let Some(index) = activate {
                        self.activate_result_index(index);
                    }
                }
            }
            return;
        }
        if self.with_world(|world| world.screen) == Screen::Shop {
            match state {
                ElementState::Pressed => {
                    self.set_market_pressed(self.hovered_market_hit());
                }
                ElementState::Released => {
                    let activate = self
                        .pressed_market_hit()
                        .filter(|pressed| Some(*pressed) == self.hovered_market_hit());
                    self.set_market_pressed(None);
                    if let Some(hit) = activate {
                        self.activate_market_hit(hit);
                    }
                }
            }
            return;
        }
        if self.with_world(|world| world.screen) == Screen::Play {
            match state {
                ElementState::Pressed => {
                    self.set_gameplay_pressed(self.hovered_gameplay_hit());
                }
                ElementState::Released => {
                    let activate = self
                        .pressed_gameplay_hit()
                        .filter(|pressed| Some(*pressed) == self.hovered_gameplay_hit());
                    self.set_gameplay_pressed(None);
                    if let Some(hit) = activate {
                        self.activate_gameplay_hit(hit);
                    }
                }
            }
            return;
        }
        match state {
            ElementState::Pressed => {
                self.menu_pressed = self.menu_hovered.filter(|_| self.title_is_interactive());
            }
            ElementState::Released => {
                let activate = self
                    .menu_pressed
                    .filter(|pressed| Some(*pressed) == self.menu_hovered);
                self.menu_pressed = None;
                if let Some(index) = activate {
                    self.activate_title_index(index, el);
                }
            }
        }
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
                        self.with_world_mut(|w| w.return_to_title());
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
                    self.move_title_selection(-1);
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.move_title_selection(1);
                }
                KeyCode::Enter | KeyCode::Space => {
                    self.activate_title_selected(el);
                }
                _ => {
                    if let Some(index) = self.title_hotkey_index(c) {
                        self.activate_title_index(index, el);
                    } else if c == KeyCode::Escape {
                        el.exit();
                    }
                }
            },
            Some("collection") => match c {
                KeyCode::ArrowLeft | KeyCode::KeyA => self.move_collection_selection(-1),
                KeyCode::ArrowRight | KeyCode::KeyD => self.move_collection_selection(1),
                KeyCode::ArrowUp | KeyCode::KeyW | KeyCode::KeyQ => {
                    self.cycle_collection_filter(-1);
                }
                KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::KeyE => {
                    self.cycle_collection_filter(1);
                }
                KeyCode::Digit1 => {
                    self.activate_collection_hit(CollectionHit::Filter(CollectionFilter::All))
                }
                KeyCode::Digit2 => {
                    self.activate_collection_hit(CollectionHit::Filter(CollectionFilter::Attack))
                }
                KeyCode::Digit3 => {
                    self.activate_collection_hit(CollectionHit::Filter(CollectionFilter::Defense))
                }
                KeyCode::Digit4 => {
                    self.activate_collection_hit(CollectionHit::Filter(CollectionFilter::Spell))
                }
                KeyCode::Digit5 => {
                    self.activate_collection_hit(CollectionHit::Filter(CollectionFilter::Skill))
                }
                KeyCode::Enter | KeyCode::Space => self.activate_collection_hit(CollectionHit::Add),
                KeyCode::KeyX | KeyCode::Backspace | KeyCode::Delete => {
                    self.activate_collection_hit(CollectionHit::Remove)
                }
                KeyCode::Escape | KeyCode::KeyB => {
                    self.activate_collection_hit(CollectionHit::Back)
                }
                _ => {}
            },
            Some("submenu") if screen == Screen::Shop => match c {
                KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::KeyA | KeyCode::KeyW => {
                    self.move_market_selection(-1);
                }
                KeyCode::ArrowRight | KeyCode::ArrowDown | KeyCode::KeyD | KeyCode::KeyS => {
                    self.move_market_selection(1);
                }
                KeyCode::Enter => {
                    let selected = self.with_world(|world| world.market_sel);
                    self.activate_market_hit(MarketHit::Offer(selected));
                }
                KeyCode::KeyR => self.activate_market_hit(MarketHit::Reroll),
                KeyCode::KeyC | KeyCode::Space | KeyCode::Escape => {
                    self.activate_market_hit(MarketHit::Back);
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
                    self.with_world_mut(|w| w.return_to_title());
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
                    self.play_current_hand();
                }
                KeyCode::KeyD => {
                    self.discard_current_hand();
                }
                KeyCode::Escape => {
                    self.with_world_mut(|w| {
                        w.screen = Screen::Pause;
                        w.pause_sel = 0;
                    });
                }
                _ => {}
            },
            Some("market") => match c {
                KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::KeyA | KeyCode::KeyW => {
                    self.move_market_selection(-1);
                }
                KeyCode::ArrowRight | KeyCode::ArrowDown | KeyCode::KeyD | KeyCode::KeyS => {
                    self.move_market_selection(1);
                }
                KeyCode::Enter => {
                    let selected = self.with_world(|world| world.market_sel);
                    self.activate_market_hit(MarketHit::Offer(selected));
                }
                KeyCode::KeyR => self.activate_market_hit(MarketHit::Reroll),
                KeyCode::KeyC | KeyCode::Space => {
                    self.activate_market_hit(MarketHit::Continue);
                }
                KeyCode::Escape => {
                    self.with_world_mut(|world| {
                        world.market_status = "Use CONTINUE RUN to enter the next blind.".into();
                    });
                }
                _ => {}
            },
            Some("result") => match c {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.result_hovered = None;
                    self.with_world_mut(|w| w.result_sel = w.result_sel.saturating_sub(1));
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.result_hovered = None;
                    self.with_world_mut(|w| {
                        if w.result_sel < 1 {
                            w.result_sel += 1;
                        }
                    });
                }
                KeyCode::Enter | KeyCode::Space => {
                    let result_sel = self.with_world(|world| world.result_sel);
                    self.activate_result_index(result_sel);
                }
                KeyCode::Escape => {
                    self.activate_result_index(1);
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
                    .set("menu_action", StoryValue::String("start".into()));
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
                    self.player
                        .variables_mut()
                        .set("stakes_outcome", StoryValue::String(o.as_str().into()));
                    self.resume("play");
                }
            }
            Some("market") if self.hframes > 20 => {
                self.resume("market");
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
            Some("collection") => self.resume("collection"),
            Some("submenu") => self.resume("submenu"),
            _ => {}
        }
        if self.hframes > 600 {
            println!("headless timeout phase={:?}", self.wait_phase());
            el.exit();
        }
    }
}

fn kind_index(kind: CardKind) -> usize {
    match kind {
        CardKind::Attack => 0,
        CardKind::Defense => 1,
        CardKind::Spell => 2,
        CardKind::Skill => 3,
    }
}

fn collection_filter_matches(filter: CollectionFilter, kind: CardKind) -> bool {
    match filter {
        CollectionFilter::All => true,
        CollectionFilter::Attack => kind == CardKind::Attack,
        CollectionFilter::Defense => kind == CardKind::Defense,
        CollectionFilter::Spell => kind == CardKind::Spell,
        CollectionFilter::Skill => kind == CardKind::Skill,
    }
}

fn hotkey_matches(code: KeyCode, authored: &str) -> bool {
    let authored = authored
        .trim()
        .to_ascii_uppercase()
        .replace([' ', '_', '-'], "");
    if authored.is_empty() {
        return false;
    }
    match authored.as_str() {
        "ESC" | "ESCAPE" => code == KeyCode::Escape,
        "ENTER" | "RETURN" => code == KeyCode::Enter,
        "SPACE" | "SPACEBAR" => code == KeyCode::Space,
        _ => {
            let physical = format!("{code:?}").to_ascii_uppercase();
            physical == authored
                || physical == format!("KEY{authored}")
                || physical == format!("DIGIT{authored}")
                || physical == format!("NUMPAD{authored}")
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
            WindowEvent::CursorMoved { position, .. } => {
                self.on_cursor_moved(position.x, position.y);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.menu_hovered = None;
                self.menu_pressed = None;
                self.gameplay_interaction = GameplayInteraction::default();
                self.market_interaction = MarketInteraction::default();
                self.collection_interaction = CollectionInteraction::default();
                self.result_hovered = None;
                self.result_pressed = None;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.on_primary_pointer(state, el);
                if let Some(window) = &self.window {
                    window.request_redraw();
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
        println!(
            "Lobby UI=data/ui/main_menu.vel · flow=data/story/main.vstory · look=data/styles/casino.vcss"
        );
        println!("Tip: cargo run -p velvet-stakes -- --dev   for live style/image reload");
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod flow_tests {
    use super::*;

    #[test]
    fn start_run_enters_gameplay_without_legacy_blind_stop() {
        let mut app = App::new(true, false).expect("boot demo");
        assert_eq!(app.wait_phase().as_deref(), Some("title"));

        app.player
            .variables_mut()
            .set("menu_action", StoryValue::String("start".into()));
        app.resume("title");

        assert_eq!(app.wait_phase().as_deref(), Some("play"));
        assert_eq!(app.with_world(|world| world.screen), Screen::Play);
        assert!(app.with_world(|world| world.run.is_some()));
    }

    #[test]
    fn blind_win_routes_through_playable_market_before_next_round() {
        let mut app = App::new(true, false).expect("boot demo");
        app.player
            .variables_mut()
            .set("menu_action", StoryValue::String("start".into()));
        app.resume("title");
        app.with_world_mut(|world| {
            if let Some(run) = world.run.as_mut() {
                run.money = 10;
                run.score = run.target;
            }
            world.apply_outcome(Outcome::WinBlind);
        });
        app.player.variables_mut().set(
            "stakes_outcome",
            StoryValue::String(Outcome::WinBlind.as_str().into()),
        );
        app.resume("play");

        assert_eq!(app.wait_phase().as_deref(), Some("market"));
        assert_eq!(app.with_world(|world| world.screen), Screen::Shop);
        assert_eq!(app.with_world(|world| world.market_offers.len()), 5);
        assert!(app.with_world_mut(|world| world.buy_market_offer(0)));
        assert_eq!(app.with_world(|world| world.deck_ids.len()), 21);

        app.resume("market");
        assert_eq!(app.wait_phase().as_deref(), Some("play"));
        assert_eq!(app.with_world(|world| world.screen), Screen::Play);
        assert_eq!(
            app.with_world(|world| world.run.as_ref().unwrap().round_number()),
            2
        );
        let run_card_count = app.with_world(|world| {
            let zones = &world.run.as_ref().unwrap().zones;
            zones.library.len() + zones.hand.len() + zones.discard.len()
        });
        assert_eq!(run_card_count, 21);
    }

    #[test]
    fn collection_route_edits_the_starter_deck_and_returns_to_lobby() {
        let mut app = App::new(true, false).expect("boot demo");
        app.player
            .variables_mut()
            .set("menu_action", StoryValue::String("collection".into()));
        app.resume("title");

        assert_eq!(app.wait_phase().as_deref(), Some("collection"));
        assert_eq!(app.with_world(|world| world.screen), Screen::Collection);
        assert!(app.with_world_mut(|world| world.add_collection_card("fireball")));
        assert_eq!(app.with_world(|world| world.starter_deck_ids.len()), 21);

        app.resume("collection");
        assert_eq!(app.wait_phase().as_deref(), Some("title"));
        assert_eq!(app.with_world(|world| world.screen), Screen::Title);
    }

    #[test]
    #[ignore = "manual visual evidence; run explicitly with --ignored"]
    fn dump_styled_blind_fallback_for_evidence() {
        let mut app = App::new(true, false).expect("boot demo");
        app.with_world_mut(|world| world.begin_run());
        app.paint_blind();

        let output =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/blind_intro_paint.png");
        let mut rgba = Vec::with_capacity((WW * WH * 4) as usize);
        for pixel in &app.pixels {
            rgba.extend_from_slice(&[
                ((pixel >> 16) & 0xff) as u8,
                ((pixel >> 8) & 0xff) as u8,
                (pixel & 0xff) as u8,
                255,
            ]);
        }
        image::save_buffer(&output, &rgba, WW, WH, image::ColorType::Rgba8)
            .expect("save blind intro evidence");
        assert!(std::fs::metadata(output).unwrap().len() > 20_000);
    }
}
