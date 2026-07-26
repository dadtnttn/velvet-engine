mod assets;
mod model;
mod render;
mod save;

use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use assets::AssetStore;
use model::{
    apply_choice, first_scene_for_location, scene_view, EndingId, GameState, LocationId, SceneId,
    SceneOutcome,
};
use render::{rgba, Canvas, FontSystem, Rect, HEIGHT, WIDTH};
use softbuffer::{Context as SoftContext, Surface};
use velvet_script_vs3::compile as compile_vs3;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Map,
    Scene(SceneId),
    Gallery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GalleryMode {
    DressUp,
    Cg,
    Animation,
}

#[derive(Debug, Clone)]
struct GalleryState {
    mode: GalleryMode,
    pose: u8,
    top: usize,
    bottom: usize,
    shoes: usize,
    accessory: usize,
    expression: usize,
    cg: usize,
    alternate: bool,
    frame: usize,
    playing: bool,
    last_frame: Instant,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            mode: GalleryMode::DressUp,
            pose: 1,
            top: 3,
            bottom: 3,
            shoes: 2,
            accessory: 0,
            expression: 0,
            cg: 0,
            alternate: false,
            frame: 1,
            playing: false,
            last_frame: Instant::now(),
        }
    }
}

impl GalleryState {
    const TOPS: [&'static str; 6] = ["none", "shirt", "hoodie", "casual", "work", "bikini"];
    const TOP_LABELS: [&'static str; 6] = [
        "NINGUNA",
        "CAMISETA",
        "HOODIE",
        "TOP CASUAL",
        "TOP TALLER",
        "BIKINI TOP",
    ];
    const BOTTOMS: [&'static str; 6] = ["none", "panties", "pants", "casual", "work", "bikini"];
    const BOTTOM_LABELS: [&'static str; 6] = [
        "NINGUNA",
        "ROPA INTERIOR",
        "PANTALÓN",
        "SHORT CASUAL",
        "PANTALÓN TALLER",
        "BIKINI BOTTOM",
    ];
    const SHOES: [&'static str; 4] = ["none", "socks", "casual", "work"];
    const SHOE_LABELS: [&'static str; 4] = ["DESCALZA", "CALCETINES", "ZAPATILLAS", "BOTAS TALLER"];
    const ACCESSORIES: [&'static str; 2] = ["none", "collar"];
    const ACCESSORY_LABELS: [&'static str; 2] = ["NINGUNO", "COLLAR"];
    const EXPRESSIONS: [&'static str; 8] = [
        "happy",
        "superhappy",
        "blushed1",
        "blushed2",
        "surprised",
        "sad",
        "angry",
        "horny_pose8",
    ];
    const EXPRESSION_LABELS: [&'static str; 8] = [
        "FELIZ",
        "MUY FELIZ",
        "SONROJADA",
        "AVERGONZADA",
        "SORPRENDIDA",
        "TRISTE",
        "ENOJADA",
        "ADULTA",
    ];
}

#[derive(Debug, Clone)]
enum UiAction {
    NewGame,
    Continue,
    OpenGallery,
    Quit,
    SceneChoice(usize),
    Visit(LocationId),
    ReturnMenu,
    GalleryMode(GalleryMode),
    GalleryPose(i8),
    GalleryTop(i8),
    GalleryBottom(i8),
    GalleryShoes(i8),
    GalleryAccessory(i8),
    GalleryExpression(i8),
    GalleryCg(i8),
    GalleryAlternate,
    GalleryPlay,
}

#[derive(Debug, Clone)]
struct Hotspot {
    rect: Rect,
    action: UiAction,
    enabled: bool,
}

struct App {
    window: Option<Arc<Window>>,
    context: Option<SoftContext<Arc<Window>>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    canvas: Canvas,
    fonts: FontSystem,
    assets: AssetStore,
    state: GameState,
    screen: Screen,
    gallery: GalleryState,
    hotspots: Vec<Hotspot>,
    pointer: (f32, f32),
    selected_choice: usize,
    status: Option<(String, Instant)>,
    fullscreen: bool,
}

impl App {
    fn new() -> Result<Self> {
        let root = locate_asset_root();
        validate_vs3_script()?;
        Ok(Self {
            window: None,
            context: None,
            surface: None,
            canvas: Canvas::new(),
            fonts: FontSystem::load_system()?,
            assets: AssetStore::new(root),
            state: GameState::default(),
            screen: Screen::Menu,
            gallery: GalleryState::default(),
            hotspots: Vec::new(),
            pointer: (-100.0, -100.0),
            selected_choice: 0,
            status: None,
            fullscreen: false,
        })
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn hovered(&self, rect: Rect) -> bool {
        rect.contains(self.pointer.0, self.pointer.1)
    }

    fn push_hotspot(&mut self, rect: Rect, action: UiAction, enabled: bool) {
        self.hotspots.push(Hotspot {
            rect,
            action,
            enabled,
        });
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.status = Some((message.into(), Instant::now()));
    }

    fn autosave(&mut self) {
        if let Err(error) = save::save(&self.state) {
            self.set_status(format!("No se pudo guardar: {error:#}"));
        }
    }

    fn new_game(&mut self) {
        self.state = GameState::default();
        self.screen = Screen::Scene(SceneId::TrainAwakening);
        self.selected_choice = 0;
        self.autosave();
    }

    fn continue_game(&mut self) {
        match save::load() {
            Ok(state) => {
                self.state = state;
                self.screen = if self.state.resume_on_map {
                    Screen::Map
                } else {
                    Screen::Scene(self.state.current_scene)
                };
                self.selected_choice = 0;
            }
            Err(error) => self.set_status(format!("No se pudo continuar: {error:#}")),
        }
    }

    fn go_to_map_or_night(&mut self) {
        if self.state.needs_night_resolution() {
            self.state.current_scene = SceneId::NightDecision;
            self.screen = Screen::Scene(SceneId::NightDecision);
            self.state.resume_on_map = false;
        } else {
            self.screen = Screen::Map;
            self.state.resume_on_map = true;
        }
        self.selected_choice = 0;
        self.autosave();
    }

    fn handle_action(&mut self, action: UiAction, event_loop: &ActiveEventLoop) {
        match action {
            UiAction::NewGame => self.new_game(),
            UiAction::Continue => self.continue_game(),
            UiAction::OpenGallery => {
                self.gallery = GalleryState::default();
                self.screen = Screen::Gallery;
            }
            UiAction::Quit => event_loop.exit(),
            UiAction::ReturnMenu => {
                self.screen = Screen::Menu;
                self.gallery.playing = false;
            }
            UiAction::Visit(location) => {
                if self.state.needs_night_resolution() {
                    self.state.current_scene = SceneId::NightDecision;
                    self.state.resume_on_map = false;
                    self.screen = Screen::Scene(SceneId::NightDecision);
                } else {
                    let scene = first_scene_for_location(location, &self.state);
                    self.state.current_scene = scene;
                    self.state.resume_on_map = false;
                    self.screen = Screen::Scene(scene);
                }
                self.selected_choice = 0;
            }
            UiAction::SceneChoice(index) => {
                let Screen::Scene(scene) = self.screen else {
                    return;
                };
                let view = scene_view(scene, &self.state);
                if !view.choices.get(index).is_some_and(|choice| choice.enabled) {
                    if let Some(hint) = view
                        .choices
                        .get(index)
                        .and_then(|choice| choice.hint.clone())
                    {
                        self.set_status(hint);
                    }
                    return;
                }
                match apply_choice(&mut self.state, scene, index) {
                    SceneOutcome::Scene(next) => {
                        self.screen = Screen::Scene(next);
                        self.state.resume_on_map = false;
                        self.selected_choice = 0;
                        self.autosave();
                    }
                    SceneOutcome::Map => self.go_to_map_or_night(),
                    SceneOutcome::Menu => {
                        self.screen = Screen::Menu;
                        self.autosave();
                    }
                }
            }
            UiAction::GalleryMode(mode) => {
                self.gallery.mode = mode;
                self.gallery.playing = false;
            }
            UiAction::GalleryPose(delta) => {
                let value = self.gallery.pose as i16 + delta as i16;
                self.gallery.pose = ((value - 1).rem_euclid(8) + 1) as u8;
            }
            UiAction::GalleryTop(delta) => {
                self.gallery.top = wrap_index(self.gallery.top, delta, GalleryState::TOPS.len());
            }
            UiAction::GalleryBottom(delta) => {
                self.gallery.bottom =
                    wrap_index(self.gallery.bottom, delta, GalleryState::BOTTOMS.len());
            }
            UiAction::GalleryShoes(delta) => {
                self.gallery.shoes =
                    wrap_index(self.gallery.shoes, delta, GalleryState::SHOES.len());
            }
            UiAction::GalleryAccessory(delta) => {
                self.gallery.accessory = wrap_index(
                    self.gallery.accessory,
                    delta,
                    GalleryState::ACCESSORIES.len(),
                );
            }
            UiAction::GalleryExpression(delta) => {
                self.gallery.expression = wrap_index(
                    self.gallery.expression,
                    delta,
                    GalleryState::EXPRESSIONS.len(),
                );
            }
            UiAction::GalleryCg(delta) => {
                self.gallery.cg = wrap_index(self.gallery.cg, delta, 3);
            }
            UiAction::GalleryAlternate => self.gallery.alternate = !self.gallery.alternate,
            UiAction::GalleryPlay => {
                self.gallery.mode = GalleryMode::Animation;
                self.gallery.playing = !self.gallery.playing;
                self.gallery.last_frame = Instant::now();
            }
        }
        self.request_redraw();
    }

    fn handle_click(&mut self, event_loop: &ActiveEventLoop) {
        let action = self
            .hotspots
            .iter()
            .rev()
            .find(|hotspot| {
                hotspot.enabled && hotspot.rect.contains(self.pointer.0, self.pointer.1)
            })
            .map(|hotspot| hotspot.action.clone());
        if let Some(action) = action {
            self.handle_action(action, event_loop);
        }
    }

    fn handle_key(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        match key {
            KeyCode::F11 => {
                self.fullscreen = !self.fullscreen;
                if let Some(window) = &self.window {
                    window.set_fullscreen(if self.fullscreen {
                        Some(Fullscreen::Borderless(None))
                    } else {
                        None
                    });
                }
            }
            KeyCode::Escape => match self.screen {
                Screen::Menu => event_loop.exit(),
                Screen::Map | Screen::Gallery => self.screen = Screen::Menu,
                Screen::Scene(_) => self.screen = Screen::Map,
            },
            KeyCode::ArrowUp => {
                if let Screen::Scene(scene) = self.screen {
                    let count = scene_view(scene, &self.state).choices.len().max(1);
                    self.selected_choice = (self.selected_choice + count - 1) % count;
                }
            }
            KeyCode::ArrowDown => {
                if let Screen::Scene(scene) = self.screen {
                    let count = scene_view(scene, &self.state).choices.len().max(1);
                    self.selected_choice = (self.selected_choice + 1) % count;
                }
            }
            KeyCode::Enter | KeyCode::Space => match self.screen {
                Screen::Menu => self.new_game(),
                Screen::Scene(_) => {
                    self.handle_action(UiAction::SceneChoice(self.selected_choice), event_loop)
                }
                Screen::Gallery if self.gallery.mode == GalleryMode::Animation => {
                    self.handle_action(UiAction::GalleryPlay, event_loop)
                }
                _ => {}
            },
            KeyCode::ArrowLeft if self.screen == Screen::Gallery => match self.gallery.mode {
                GalleryMode::DressUp => self.handle_action(UiAction::GalleryPose(-1), event_loop),
                GalleryMode::Cg => self.handle_action(UiAction::GalleryCg(-1), event_loop),
                GalleryMode::Animation => {}
            },
            KeyCode::ArrowRight if self.screen == Screen::Gallery => match self.gallery.mode {
                GalleryMode::DressUp => self.handle_action(UiAction::GalleryPose(1), event_loop),
                GalleryMode::Cg => self.handle_action(UiAction::GalleryCg(1), event_loop),
                GalleryMode::Animation => {}
            },
            _ => {}
        }
        self.request_redraw();
    }

    fn update(&mut self) {
        if self.screen == Screen::Gallery
            && self.gallery.mode == GalleryMode::Animation
            && self.gallery.playing
        {
            let frame_duration = Duration::from_millis(33);
            while self.gallery.last_frame.elapsed() >= frame_duration {
                self.gallery.frame = self.gallery.frame % 30 + 1;
                self.gallery.last_frame += frame_duration;
            }
        }
        if self
            .status
            .as_ref()
            .is_some_and(|(_, time)| time.elapsed() > Duration::from_secs(4))
        {
            self.status = None;
        }
    }

    fn render(&mut self) {
        self.hotspots.clear();
        match self.screen {
            Screen::Menu => self.render_menu(),
            Screen::Map => self.render_map(),
            Screen::Scene(scene) => self.render_scene(scene),
            Screen::Gallery => self.render_gallery(),
        }
        self.render_status();
    }

    fn draw_background(&mut self, name: &str) {
        self.canvas.clear(0xff171724);
        if let Some(image) = self.assets.background(name) {
            self.canvas
                .image_cover(&image, Rect::new(0, 0, WIDTH as i32, HEIGHT as i32));
        }
    }

    fn render_menu(&mut self) {
        self.draw_background("menu");
        self.canvas.alpha_rect(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            rgba(9, 10, 22, 100),
        );
        self.canvas
            .text_line(self.fonts.font(true), "SAM", 72, 128, 72.0, 0xffff6f91);
        self.canvas.text_line(
            self.fonts.font(true),
            "HONEST STRANGER",
            76,
            177,
            28.0,
            0xfff4eff7,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            "Una chica de un mundo sin mentiras llega a una ciudad donde sobrevivir también significa aprender lo que la gente calla.",
            78,
            208,
            430,
            19.0,
            27,
            0xffd7d3df,
        );
        self.canvas
            .alpha_rect(Rect::new(610, 65, 275, 505), rgba(19, 20, 35, 218));
        self.canvas
            .border(Rect::new(610, 65, 275, 505), 2, 0xff8a668f);
        self.canvas.text_line(
            self.fonts.font(true),
            "PRIMER DÍA",
            650,
            130,
            25.0,
            0xffff9ab4,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            "VN + mapa + supervivencia social\n\nEncuentra comida.\nConsigue trabajo.\nBusca dónde dormir.",
            650,
            148,
            195,
            15.0,
            22,
            0xffece7ef,
        );

        let buttons = [
            (
                Rect::new(650, 320, 195, 48),
                "NUEVA PARTIDA",
                UiAction::NewGame,
                true,
            ),
            (
                Rect::new(650, 380, 195, 48),
                "CONTINUAR",
                UiAction::Continue,
                save::exists(),
            ),
            (
                Rect::new(650, 440, 195, 48),
                "EXTRAS 18+",
                UiAction::OpenGallery,
                true,
            ),
            (Rect::new(650, 500, 195, 48), "SALIR", UiAction::Quit, true),
        ];
        for (rect, label, action, enabled) in buttons {
            self.canvas
                .button(&self.fonts, rect, label, self.hovered(rect), enabled, false);
            self.push_hotspot(rect, action, enabled);
        }
        self.canvas.text_line(
            self.fonts.font(false),
            "Ratón · Enter · Flechas · F11",
            74,
            595,
            14.0,
            0xffaaa6b3,
        );
    }

    fn render_hud(&mut self) {
        self.canvas
            .alpha_rect(Rect::new(0, 0, WIDTH as i32, 72), rgba(15, 17, 29, 232));
        self.canvas.text_line(
            self.fonts.font(true),
            &format!("DÍA {} · {}", self.state.day, self.state.time.label()),
            22,
            28,
            18.0,
            0xffff8dac,
        );
        self.canvas.text_line(
            self.fonts.font(true),
            &format!("DINERO  {} €", self.state.money),
            22,
            55,
            17.0,
            0xfff8e6ba,
        );
        let room = if self.state.has_room {
            "REFUGIO: SÍ"
        } else {
            "REFUGIO: NO"
        };
        self.canvas.text_line(
            self.fonts.font(true),
            room,
            190,
            55,
            15.0,
            if self.state.has_room {
                0xff8ee0a7
            } else {
                0xffffa0a0
            },
        );
        self.canvas.stat_bar(
            &self.fonts,
            Rect::new(500, 16, 125, 32),
            "HAMBRE",
            self.state.hunger,
            0xffe4a04f,
        );
        self.canvas.stat_bar(
            &self.fonts,
            Rect::new(650, 16, 125, 32),
            "ENERGÍA",
            self.state.energy,
            0xff62b8dc,
        );
        self.canvas.stat_bar(
            &self.fonts,
            Rect::new(800, 16, 125, 32),
            "CONFIANZA",
            self.state.trust,
            0xffdf7fa7,
        );
    }

    fn render_map(&mut self) {
        self.draw_background("map");
        self.render_hud();
        self.canvas
            .alpha_rect(Rect::new(20, 90, 290, 100), rgba(24, 24, 35, 220));
        self.canvas
            .border(Rect::new(20, 90, 290, 100), 2, 0xff776b75);
        self.canvas.text_line(
            self.fonts.font(true),
            "MAPA DE LA CIUDAD",
            42,
            125,
            22.0,
            0xfff1e9ef,
        );
        self.canvas.text_wrapped(
            self.fonts.font(false),
            "Elige una locación. Cada actividad importante hace avanzar el día.",
            42,
            140,
            245,
            15.0,
            21,
            0xffd7d0db,
        );

        let nodes = [
            (LocationId::Plaza, Rect::new(115, 205, 155, 52)),
            (LocationId::Cafe, Rect::new(400, 185, 165, 52)),
            (LocationId::Garage, Rect::new(680, 210, 170, 52)),
            (LocationId::Train, Rect::new(205, 475, 165, 52)),
            (LocationId::BoardingHouse, Rect::new(570, 480, 190, 52)),
        ];
        for (location, rect) in nodes {
            let visited = self.state.visited.contains(&location);
            let label = if visited {
                format!("{}  [OK]", location.label())
            } else {
                location.label().to_owned()
            };
            self.canvas
                .button(&self.fonts, rect, &label, self.hovered(rect), true, false);
            self.push_hotspot(rect, UiAction::Visit(location), true);
        }

        let menu = Rect::new(780, 575, 155, 42);
        self.canvas
            .button(&self.fonts, menu, "MENÚ", self.hovered(menu), true, false);
        self.push_hotspot(menu, UiAction::ReturnMenu, true);
        self.canvas
            .alpha_rect(Rect::new(20, 550, 470, 66), rgba(30, 27, 35, 210));
        let objective = if self.state.has_room {
            "Ya tienes dónde dormir. Busca comida o una oportunidad para mañana."
        } else if self.state.money >= 18 {
            "Puedes pagar una habitación. También podrías buscar trabajo antes de la noche."
        } else {
            "Objetivo: consigue 18 € o una referencia que convenza a Inés."
        };
        self.canvas.text_wrapped(
            self.fonts.font(true),
            objective,
            38,
            566,
            430,
            15.0,
            21,
            0xfff1e9ef,
        );
    }

    fn render_scene(&mut self, scene: SceneId) {
        let view = scene_view(scene, &self.state);
        let background = if matches!(
            scene,
            SceneId::NightStreet | SceneId::Ending(EndingId::StreetPromise)
        ) {
            "night"
        } else {
            view.location.background()
        };
        self.draw_background(background);
        self.render_hud();
        self.canvas
            .alpha_rect(Rect::new(0, 72, WIDTH as i32, 568), rgba(12, 13, 22, 42));

        let character_rect = Rect::new(520, 76, 415, 505);
        for layer in self.assets.character_layers(view.look) {
            self.canvas.image_fit(&layer, character_rect, true);
        }

        self.canvas
            .alpha_rect(Rect::new(24, 92, 495, 470), rgba(17, 18, 30, 225));
        self.canvas
            .border(Rect::new(24, 92, 495, 470), 2, 0xff82566f);
        self.canvas.text_line(
            self.fonts.font(true),
            view.location.label(),
            48,
            127,
            15.0,
            0xffaaa4b5,
        );
        self.canvas.text_line(
            self.fonts.font(true),
            view.speaker,
            48,
            166,
            24.0,
            0xffff789d,
        );
        let text_end = self.canvas.text_wrapped(
            self.fonts.font(false),
            &view.text,
            48,
            184,
            440,
            17.0,
            24,
            0xfff2edf4,
        );
        let mut y = (text_end + 15).min(382);
        for (index, choice) in view.choices.iter().enumerate() {
            let rect = Rect::new(48, y, 440, 48);
            self.canvas.button(
                &self.fonts,
                rect,
                &choice.text,
                self.hovered(rect),
                choice.enabled,
                index == self.selected_choice,
            );
            self.push_hotspot(rect, UiAction::SceneChoice(index), choice.enabled);
            y += 57;
        }
        self.canvas.text_line(
            self.fonts.font(false),
            "↑ ↓ para elegir · Enter para confirmar",
            48,
            545,
            13.0,
            0xffa9a4b1,
        );
    }

    fn render_gallery(&mut self) {
        self.draw_background("menu");
        self.canvas.alpha_rect(
            Rect::new(0, 0, WIDTH as i32, HEIGHT as i32),
            rgba(8, 9, 18, 155),
        );
        self.canvas.text_line(
            self.fonts.font(true),
            "EXTRAS DE SAM · SOLO ADULTOS",
            28,
            42,
            22.0,
            0xffff789d,
        );
        self.canvas.text_line(self.fonts.font(false), "Visor de assets incluido en la demo. La historia principal no depende de esta sección.", 28, 68, 14.0, 0xffbbb6c4);

        let tabs = [
            (GalleryMode::DressUp, "VESTIDOR"),
            (GalleryMode::Cg, "GALERÍA"),
            (GalleryMode::Animation, "ANIMACIÓN"),
        ];
        for (index, (mode, label)) in tabs.into_iter().enumerate() {
            let rect = Rect::new(28 + index as i32 * 165, 88, 150, 42);
            self.canvas.button(
                &self.fonts,
                rect,
                label,
                self.hovered(rect),
                true,
                self.gallery.mode == mode,
            );
            self.push_hotspot(rect, UiAction::GalleryMode(mode), true);
        }

        self.canvas
            .alpha_rect(Rect::new(25, 145, 610, 460), rgba(15, 16, 28, 225));
        self.canvas
            .border(Rect::new(25, 145, 610, 460), 2, 0xff7b657c);
        match self.gallery.mode {
            GalleryMode::DressUp => {
                for layer in self.assets.dressup_layers(
                    self.gallery.pose,
                    GalleryState::TOPS[self.gallery.top],
                    GalleryState::BOTTOMS[self.gallery.bottom],
                    GalleryState::SHOES[self.gallery.shoes],
                    GalleryState::ACCESSORIES[self.gallery.accessory],
                    GalleryState::EXPRESSIONS[self.gallery.expression],
                ) {
                    self.canvas
                        .image_fit(&layer, Rect::new(45, 155, 575, 440), true);
                }
            }
            GalleryMode::Cg => {
                if let Some(image) = self.assets.cg(self.gallery.cg, self.gallery.alternate) {
                    self.canvas
                        .image_fit(&image, Rect::new(40, 160, 580, 430), true);
                }
            }
            GalleryMode::Animation => {
                if let Some(image) = self.assets.animation_frame(self.gallery.frame) {
                    self.canvas
                        .image_fit(&image, Rect::new(40, 160, 580, 430), true);
                }
            }
        }

        self.canvas
            .alpha_rect(Rect::new(650, 145, 285, 460), rgba(18, 19, 32, 235));
        self.canvas
            .border(Rect::new(650, 145, 285, 460), 2, 0xff7b657c);
        match self.gallery.mode {
            GalleryMode::DressUp => {
                self.gallery_compact_control(
                    158,
                    "POSE",
                    &format!("{} / 8", self.gallery.pose),
                    UiAction::GalleryPose(-1),
                    UiAction::GalleryPose(1),
                );
                self.gallery_compact_control(
                    220,
                    "ARRIBA",
                    GalleryState::TOP_LABELS[self.gallery.top],
                    UiAction::GalleryTop(-1),
                    UiAction::GalleryTop(1),
                );
                self.gallery_compact_control(
                    282,
                    "ABAJO",
                    GalleryState::BOTTOM_LABELS[self.gallery.bottom],
                    UiAction::GalleryBottom(-1),
                    UiAction::GalleryBottom(1),
                );
                self.gallery_compact_control(
                    344,
                    "CALZADO",
                    GalleryState::SHOE_LABELS[self.gallery.shoes],
                    UiAction::GalleryShoes(-1),
                    UiAction::GalleryShoes(1),
                );
                self.gallery_compact_control(
                    406,
                    "ACCESORIO",
                    GalleryState::ACCESSORY_LABELS[self.gallery.accessory],
                    UiAction::GalleryAccessory(-1),
                    UiAction::GalleryAccessory(1),
                );
                self.gallery_compact_control(
                    468,
                    "EXPRESIÓN",
                    GalleryState::EXPRESSION_LABELS[self.gallery.expression],
                    UiAction::GalleryExpression(-1),
                    UiAction::GalleryExpression(1),
                );
            }
            GalleryMode::Cg => {
                self.gallery_control_pair(
                    190,
                    "ILUSTRACIÓN",
                    &format!("SAM {} / 3", self.gallery.cg + 1),
                    UiAction::GalleryCg(-1),
                    UiAction::GalleryCg(1),
                );
                let rect = Rect::new(735, 310, 165, 48);
                let label = if self.gallery.alternate {
                    "VARIANTE B"
                } else {
                    "VARIANTE A"
                };
                self.canvas
                    .button(&self.fonts, rect, label, self.hovered(rect), true, false);
                self.push_hotspot(rect, UiAction::GalleryAlternate, true);
            }
            GalleryMode::Animation => {
                self.canvas.text_line(
                    self.fonts.font(true),
                    &format!("FRAME {:02} / 30", self.gallery.frame),
                    748,
                    220,
                    18.0,
                    0xfff1edf4,
                );
                let rect = Rect::new(735, 270, 165, 50);
                let label = if self.gallery.playing {
                    "PAUSAR"
                } else {
                    "REPRODUCIR"
                };
                self.canvas
                    .button(&self.fonts, rect, label, self.hovered(rect), true, false);
                self.push_hotspot(rect, UiAction::GalleryPlay, true);
                self.canvas.text_wrapped(
                    self.fonts.font(false),
                    "Reproducción real a 30 FPS.",
                    735,
                    345,
                    170,
                    14.0,
                    20,
                    0xffb8b2c1,
                );
            }
        }
        let back = Rect::new(690, 560, 205, 34);
        self.canvas
            .button(&self.fonts, back, "VOLVER", self.hovered(back), true, false);
        self.push_hotspot(back, UiAction::ReturnMenu, true);
    }

    fn gallery_compact_control(
        &mut self,
        y: i32,
        title: &str,
        value: &str,
        previous: UiAction,
        next: UiAction,
    ) {
        let row = Rect::new(665, y, 255, 56);
        self.canvas.alpha_rect(row, rgba(26, 27, 43, 185));
        self.canvas.border(row, 1, 0xff4d4c63);
        self.canvas
            .text_line(self.fonts.font(true), title, 678, y + 18, 12.0, 0xffff8dac);
        self.canvas
            .text_line(self.fonts.font(false), value, 678, y + 39, 13.0, 0xffeee8f0);
        let left = Rect::new(842, y + 9, 32, 38);
        let right = Rect::new(882, y + 9, 32, 38);
        self.canvas
            .button(&self.fonts, left, "<", self.hovered(left), true, false);
        self.canvas
            .button(&self.fonts, right, ">", self.hovered(right), true, false);
        self.push_hotspot(left, previous, true);
        self.push_hotspot(right, next, true);
    }

    fn gallery_control_pair(
        &mut self,
        y: i32,
        title: &str,
        value: &str,
        previous: UiAction,
        next: UiAction,
    ) {
        self.canvas
            .text_line(self.fonts.font(true), title, 735, y, 14.0, 0xffff8dac);
        self.canvas.text_wrapped(
            self.fonts.font(false),
            value,
            735,
            y + 15,
            170,
            15.0,
            20,
            0xffeee8f0,
        );
        let left = Rect::new(735, y + 53, 75, 38);
        let right = Rect::new(825, y + 53, 75, 38);
        self.canvas
            .button(&self.fonts, left, "<", self.hovered(left), true, false);
        self.canvas
            .button(&self.fonts, right, ">", self.hovered(right), true, false);
        self.push_hotspot(left, previous, true);
        self.push_hotspot(right, next, true);
    }

    fn render_status(&mut self) {
        let Some((message, _)) = self.status.clone() else {
            return;
        };
        let rect = Rect::new(235, 585, 490, 42);
        self.canvas.alpha_rect(rect, rgba(40, 27, 39, 245));
        self.canvas.border(rect, 2, 0xffff8dac);
        self.canvas.text_wrapped(
            self.fonts.font(true),
            &message,
            250,
            594,
            460,
            14.0,
            18,
            0xffffffff,
        );
    }

    fn present(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(surface) = &mut self.surface else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let Some(width) = NonZeroU32::new(size.width) else {
            return;
        };
        let Some(height) = NonZeroU32::new(size.height) else {
            return;
        };
        if let Err(error) = surface.resize(width, height) {
            self.set_status(format!("Error de superficie: {error}"));
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        scale_letterboxed(
            &self.canvas.pixels,
            WIDTH,
            HEIGHT,
            &mut buffer,
            size.width as usize,
            size.height as usize,
        );
        let _ = buffer.present();
    }

    fn map_pointer(&mut self, physical_x: f64, physical_y: f64) {
        let Some(window) = &self.window else { return };
        let size = window.inner_size();
        let (scale, offset_x, offset_y) =
            viewport_transform(size.width as usize, size.height as usize);
        self.pointer = (
            ((physical_x as f32 - offset_x) / scale).clamp(-1000.0, 2000.0),
            ((physical_y as f32 - offset_y) / scale).clamp(-1000.0, 2000.0),
        );
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("Sam: Honest Stranger — Velvet Engine")
            .with_inner_size(PhysicalSize::new(1100, 700))
            .with_position(PhysicalPosition::new(90, 55))
            .with_min_inner_size(PhysicalSize::new(900, 600));
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                match SoftContext::new(window.clone()).and_then(|context| {
                    Surface::new(&context, window.clone()).map(|surface| (context, surface))
                }) {
                    Ok((context, surface)) => {
                        self.window = Some(window);
                        self.context = Some(context);
                        self.surface = Some(surface);
                        self.request_redraw();
                    }
                    Err(error) => {
                        eprintln!("No se pudo crear softbuffer: {error}");
                        event_loop.exit();
                    }
                }
            }
            Err(error) => {
                eprintln!("No se pudo crear la ventana: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.map_pointer(position.x, position.y);
                self.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.handle_click(event_loop),
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    self.handle_key(key, event_loop);
                }
            }
            WindowEvent::Resized(_) => self.request_redraw(),
            WindowEvent::RedrawRequested => {
                self.update();
                self.render();
                self.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.screen == Screen::Gallery
            && self.gallery.mode == GalleryMode::Animation
            && self.gallery.playing
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(16),
            ));
            self.request_redraw();
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn wrap_index(current: usize, delta: i8, count: usize) -> usize {
    (current as isize + delta as isize).rem_euclid(count as isize) as usize
}

fn locate_asset_root() -> PathBuf {
    let candidates = [
        PathBuf::from("demos/sam-tomboy/data/assets"),
        PathBuf::from("data/assets"),
        PathBuf::from("../sam-tomboy/data/assets"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_dir())
        .unwrap_or_else(|| PathBuf::from("demos/sam-tomboy/data/assets"))
}

fn locate_story_script() -> PathBuf {
    [
        PathBuf::from("demos/sam-tomboy/story/sam_logic.vel"),
        PathBuf::from("story/sam_logic.vel"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .unwrap_or_else(|| PathBuf::from("demos/sam-tomboy/story/sam_logic.vel"))
}

fn validate_vs3_script() -> Result<()> {
    let path = locate_story_script();
    let source = std::fs::read_to_string(&path)
        .with_context(|| format!("no se pudo leer {}", path.display()))?;
    compile_vs3(&source, Some(path.to_string_lossy().as_ref()))
        .map(|_| ())
        .with_context(|| format!("el módulo VS3 no compila: {}", path.display()))
}

fn viewport_transform(destination_width: usize, destination_height: usize) -> (f32, f32, f32) {
    let scale = (destination_width as f32 / WIDTH as f32)
        .min(destination_height as f32 / HEIGHT as f32)
        .max(0.001);
    let viewport_width = WIDTH as f32 * scale;
    let viewport_height = HEIGHT as f32 * scale;
    (
        scale,
        (destination_width as f32 - viewport_width) * 0.5,
        (destination_height as f32 - viewport_height) * 0.5,
    )
}

fn scale_letterboxed(
    source: &[u32],
    source_width: usize,
    source_height: usize,
    destination: &mut [u32],
    destination_width: usize,
    destination_height: usize,
) {
    destination.fill(0xff080910);
    let (scale, offset_x, offset_y) = viewport_transform(destination_width, destination_height);
    let viewport_width = (source_width as f32 * scale).round() as usize;
    let viewport_height = (source_height as f32 * scale).round() as usize;
    let start_x = offset_x.max(0.0).round() as usize;
    let start_y = offset_y.max(0.0).round() as usize;
    for dy in 0..viewport_height.min(destination_height.saturating_sub(start_y)) {
        let sy = (dy as f32 / scale) as usize;
        for dx in 0..viewport_width.min(destination_width.saturating_sub(start_x)) {
            let sx = (dx as f32 / scale) as usize;
            destination[(start_y + dy) * destination_width + start_x + dx] =
                source[sy.min(source_height - 1) * source_width + sx.min(source_width - 1)];
        }
    }
}

fn run_headless() -> Result<()> {
    validate_vs3_script()?;
    let mut state = GameState::default();
    let path = [
        (SceneId::TrainAwakening, 0),
        (SceneId::TrainTicket, 0),
        (SceneId::TrainArrival, 0),
        (SceneId::PlazaFirst, 0),
        (SceneId::PlazaTruth, 0),
        (SceneId::GarageFirst, 0),
        (SceneId::GarageTruth, 0),
        (SceneId::GarageWork, 0),
        (SceneId::BoardingFirst, 0),
        (SceneId::BoardingPay, 0),
        (SceneId::CafeFirst, 1),
        (SceneId::CafeWork, 0),
        (SceneId::NightDecision, 0),
    ];
    for (scene, choice) in path {
        state.current_scene = scene;
        let view = scene_view(scene, &state);
        anyhow::ensure!(
            view.choices.get(choice).is_some_and(|item| item.enabled),
            "elección headless bloqueada en {scene:?}"
        );
        let _ = apply_choice(&mut state, scene, choice);
    }
    anyhow::ensure!(state.completed, "la ruta headless no completó la demo");
    anyhow::ensure!(state.has_room, "la ruta headless no obtuvo refugio");
    anyhow::ensure!(state.job_offer, "la ruta headless no obtuvo trabajo");
    anyhow::ensure!(
        state.ending == Some(EndingId::HonestWork),
        "final inesperado"
    );
    let encoded = serde_json::to_vec(&state)?;
    let decoded: GameState = serde_json::from_slice(&encoded)?;
    anyhow::ensure!(
        decoded.ending == state.ending,
        "falló el round-trip del guardado"
    );
    println!("SAM HONEST STRANGER HEADLESS OK");
    println!("ending=HonestWork");
    println!(
        "money={} hunger={} energy={} trust={}",
        state.money, state.hunger, state.energy, state.trust
    );
    println!("save_roundtrip=ok");
    println!("vs3_check=ok");
    Ok(())
}

fn run_capture(screen_name: &str, output: &Path) -> Result<()> {
    let mut app = App::new()?;
    match screen_name {
        "menu" => app.screen = Screen::Menu,
        "intro" => app.screen = Screen::Scene(SceneId::TrainAwakening),
        "ticket" => app.screen = Screen::Scene(SceneId::TrainTicket),
        "map" => {
            app.state.plaza_met_lina = true;
            app.state.money = 10;
            app.state.trust = 2;
            app.state.visited.push(LocationId::Plaza);
            app.screen = Screen::Map;
        }
        "plaza" => app.screen = Screen::Scene(SceneId::PlazaFirst),
        "cafe" => app.screen = Screen::Scene(SceneId::CafeFirst),
        "garage" => app.screen = Screen::Scene(SceneId::GarageFirst),
        "boarding" => app.screen = Screen::Scene(SceneId::BoardingFirst),
        "gallery" => app.screen = Screen::Gallery,
        "ending" => {
            app.state.has_room = true;
            app.state.job_offer = true;
            app.state.completed = true;
            app.state.ending = Some(EndingId::HonestWork);
            app.screen = Screen::Scene(SceneId::Ending(EndingId::HonestWork));
        }
        other => anyhow::bail!("pantalla de captura desconocida: {other}"),
    }
    app.render();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("no se pudo crear {}", parent.display()))?;
    }
    let mut rgba = Vec::with_capacity(WIDTH * HEIGHT * 4);
    for pixel in &app.canvas.pixels {
        rgba.push(((pixel >> 16) & 0xff) as u8);
        rgba.push(((pixel >> 8) & 0xff) as u8);
        rgba.push((pixel & 0xff) as u8);
        rgba.push(255);
    }
    image::save_buffer(
        output,
        &rgba,
        WIDTH as u32,
        HEIGHT as u32,
        image::ColorType::Rgba8,
    )
    .with_context(|| format!("no se pudo guardar {}", output.display()))?;
    println!("capture={} -> {}", screen_name, output.display());
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.iter().any(|argument| argument == "--headless") {
        return run_headless();
    }
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument == "--capture-screen")
    {
        let screen = arguments
            .get(index + 1)
            .context("falta el nombre de pantalla")?;
        let output = arguments
            .get(index + 2)
            .context("falta la ruta de salida")?;
        return run_capture(screen, Path::new(output));
    }
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
