#![cfg_attr(all(windows, not(feature = "console")), windows_subsystem = "windows")]

//! That Time a Heart Attack Reincarnated Me as the Most Feared Demon Queen
//! of the Dark Continent (and I Discovered the Royal Throne Has Armrests)
//!
//! VN 18+ whose authoritative story is the versioned JSON document at
//! `story/prologue.json`. Rooms, characters, image briefs, choices, variables
//! and effects live in data; this host renders a flat scene view.
//!
//! Controls: mouse or ↑↓ + Enter · Esc pause · S skip dialogue until a choice
//! `--headless [route]`: plays a full prologue route without a window.

mod backpack_menu;
mod exploration3d;
mod exploration3d_state;
mod game;
mod loading;
mod localization;
mod menu_ui;
mod pause_menu;
use demon_queen_dungeon::prompt_icons;
mod portable;
mod quick_actions;
mod render;
mod scene_transition;
mod title_menu;
mod title_reel;
mod unified_smoke;
mod vn_chiquita;
mod vn_web;
mod window_display;

use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use localization::{Language, Localizer};
use render::{Canvas, ChoiceTreatment, Fonts, NativeLabelFont};
use unicode_segmentation::UnicodeSegmentation;
use velvet_render::SoftwareViewport;
use velvet_style::{parse_stylesheet, resolve, ComputedStyle, StyleQuery, StyleValue, Stylesheet};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{DeviceEvent, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Icon, Window, WindowId};

const WIN_W: u32 = 1920;
const WIN_H: u32 = 1080;
const MIN_WINDOW_W: u32 = 640;
const MIN_WINDOW_H: u32 = 360;
const APP_ICON_PNG: &[u8] = include_bytes!("../data/assets/title/app_icon.png");
const SCROLL_ACTION_THRESHOLD: f32 = 48.0;

#[derive(Clone, Copy)]
struct MenuItem {
    id: &'static str,
    label: &'static str,
    enabled: bool,
    status: Option<&'static str>,
}

const MENU_ITEMS: [MenuItem; 4] = [
    MenuItem {
        id: "new-game",
        label: "Nueva Partida",
        enabled: true,
        status: None,
    },
    MenuItem {
        id: "continue",
        label: "Continuar",
        enabled: false,
        status: None,
    },
    MenuItem {
        id: "options",
        label: "Opciones",
        enabled: true,
        status: None,
    },
    MenuItem {
        id: "quit",
        label: "Salir",
        enabled: true,
        status: None,
    },
];

// Overlay cristalino de la imagen proporcionada por el usuario (DEMON QUEEN).
const DEMON_OVERLAY_KEY: &str = "title/demon_queen_menu_overlay_native_705x1050";
const TITLE_ART_KEY: &str = "native_1080p/title/title_queen_v2";
const TITLE_MENU_BACKGROUNDS: [&str; 9] = [
    "native_1080p/title/menu_bg_garden",
    "native_1080p/title/menu_bg_candy",
    "native_1080p/title/menu_bg_glitch",
    "native_1080p/title/menu_bg_surreal_poppy",
    "native_1080p/title/menu_bg_surreal_eyes",
    "native_1080p/title/menu_bg_violet_blinds",
    "native_1080p/title/menu_bg_house_cloud",
    "native_1080p/title/menu_bg_cosmic_trail",
    "native_1080p/title/menu_bg_smoking",
];
const TITLE_BACKGROUND_FADE: Duration = Duration::from_millis(220);
const TITLE_SURREAL_DELAY: Duration = Duration::from_millis(3500);
const TITLE_SURREAL_INTERVAL: Duration = Duration::from_millis(4000);
const TITLE_FOOTER_HINTS: [(&str, &str, &str); 5] = [
    ("navigate", "↑↓", "title.navigate"),
    ("select", "ENTER", "title.select"),
    ("reload", "R", "title.restart"),
    ("show-art", "ALT", "title.show_art"),
    ("exit", "ESC", "title.exit"),
];
const DEVELOPER_CREDIT: &str = "A GAME BY VELVET WEAVER";
const ENGINE_CREDIT: &str = "WOVEN WITH VELVET ENGINE";
const RESUME_BACKGROUND_KEY: &str = "native_1080p/title/menu_bg_gold";
const OPTIONS_BACKGROUND_KEY: &str = "native_1080p/title/menu_bg_candy";
const END_BACKGROUND_KEY: &str = "native_1080p/title/menu_bg_glitch";

#[derive(Debug, Clone, Copy)]
struct MenuLayout {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    gap: i32,
}

impl MenuLayout {
    fn resolve(styles: &Stylesheet, class: &str, fallback: Self) -> Self {
        let style = resolve(styles, &StyleQuery::class(class).with_element("menu"));
        Self {
            x: style.number("x", fallback.x as f32).round() as i32,
            y: style.number("y", fallback.y as f32).round() as i32,
            width: style.number("width", fallback.width as f32).round() as i32,
            height: style.number("height", fallback.height as f32).round() as i32,
            gap: style.number("gap", fallback.gap as f32).round() as i32,
        }
    }

    fn item_y(self, index: usize) -> i32 {
        self.y + index as i32 * (self.height + self.gap)
    }

    fn contains(self, index: usize, x: i32, y: i32) -> bool {
        let item_y = self.item_y(index);
        x >= self.x && x <= self.x + self.width && y >= item_y && y <= item_y + self.height
    }
}

/// Loads and caches ComfyUI art from `data/assets/`; procedural paint stays
/// as the honest fallback when an asset is absent.
const ART_CACHE_BUDGET_BYTES: usize = 256 * 1024 * 1024;
const MAX_ART_BITMAP_BYTES: usize = 32 * 1024 * 1024;

fn decoded_art_bytes(width: u32, height: u32) -> Option<usize> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
}

#[derive(Default)]
struct ArtCache {
    entries: HashMap<String, Option<ArtBitmap>>,
    bytes: usize,
}

impl ArtCache {
    fn load_or_insert<F>(&mut self, cache_key: String, loader: F)
    where
        F: FnOnce() -> Option<ArtBitmap>,
    {
        if !self.entries.contains_key(&cache_key) {
            self.insert_loaded(cache_key, loader());
        }
    }

    fn insert_loaded(&mut self, cache_key: String, loaded: Option<ArtBitmap>) {
        let incoming = loaded.as_ref().map_or(0, ArtBitmap::byte_len);
        if incoming > 0 && self.bytes.saturating_add(incoming) > ART_CACHE_BUDGET_BYTES {
            // A scene owns its art. Drop older scenes before admitting the
            // next frame instead of retaining every image visited this run.
            self.clear();
        }
        self.bytes = self.bytes.saturating_add(incoming);
        self.entries.insert(cache_key, loaded);
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    fn fitted(&mut self, key: &str, fit: ArtFit) -> Option<&ArtBitmap> {
        let cache_key = format!("{key}#{fit:?}");
        self.load_or_insert(cache_key.clone(), || load_art(key, fit));
        self.entries
            .get(&cache_key)
            .expect("just inserted")
            .as_ref()
    }

    fn original_trimmed(&mut self, key: &str) -> Option<&ArtBitmap> {
        self.fitted(key, ArtFit::OriginalTrimmed)
    }

    fn original(&mut self, key: &str) -> Option<&ArtBitmap> {
        self.fitted(key, ArtFit::Original)
    }
}

struct ArtBitmap {
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

impl ArtBitmap {
    fn byte_len(&self) -> usize {
        self.rgba.len()
    }
}

#[derive(Debug, Clone, Copy)]
enum ArtFit {
    /// Preserve source resolution while trimming transparent outer padding.
    OriginalTrimmed,
    /// Preserve both source resolution and transparent canvas bounds.
    Original,
}

fn assets_root() -> PathBuf {
    [
        PathBuf::from("demos/demon-queen/data/assets"),
        PathBuf::from("data/assets"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/assets"),
    ]
    .into_iter()
    .find(|p| p.is_dir())
    .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/assets"))
}

/// Decode a pre-sized art asset; `None` when missing or unreadable.
fn load_art(key: &str, fit: ArtFit) -> Option<ArtBitmap> {
    let relative = PathBuf::from(key);
    let relative = if relative.extension().is_some() {
        relative
    } else {
        PathBuf::from(format!("{key}.png"))
    };
    let portable_path = relative.to_string_lossy().replace('\\', "/");
    let decoded = if portable::enabled() {
        let bytes = portable::asset_bytes(&portable_path)?;
        let dimensions = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .ok()?
            .into_dimensions()
            .ok()?;
        if decoded_art_bytes(dimensions.0, dimensions.1)? > MAX_ART_BITMAP_BYTES {
            return None;
        }
        image::load_from_memory(bytes).ok()?
    } else {
        let path = assets_root().join(&relative);
        let dimensions = image::ImageReader::open(&path)
            .ok()?
            .into_dimensions()
            .ok()?;
        if decoded_art_bytes(dimensions.0, dimensions.1)? > MAX_ART_BITMAP_BYTES {
            return None;
        }
        image::open(path).ok()?
    };
    let source_bytes = decoded_art_bytes(decoded.width(), decoded.height())?;
    if source_bytes > MAX_ART_BITMAP_BYTES {
        return None;
    }
    let image = decoded.to_rgba8();
    let prepared = match fit {
        ArtFit::OriginalTrimmed => trim_transparent(&image),
        ArtFit::Original => image,
    };
    let (w, h) = prepared.dimensions();
    let bytes = decoded_art_bytes(w, h)?;
    if bytes > MAX_ART_BITMAP_BYTES {
        return None;
    }
    Some(ArtBitmap {
        w: w as usize,
        h: h as usize,
        rgba: prepared.into_raw(),
    })
}

fn trim_transparent(image: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = image.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 4 {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x > max_x || min_y > max_y {
        return image.clone();
    }
    image::imageops::crop_imm(image, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Title,
    Resume,
    Options,
    Gallery,
    Play,
    Explore,
    End,
    GameMenu,
}

struct App {
    screen: Screen,
    menu_sel: usize,
    choice_sel: usize,
    options_sel: usize,
    gallery_sel: usize,
    quit: bool,
    game: Option<game::Game>,
    view: Option<game::SceneView>,
    localizer: Localizer,
    gallery: game::GalleryLibrary,
    gallery_return: Screen,
    fonts: Option<Fonts>,
    native_label_fonts: HashMap<&'static str, Option<NativeLabelFont>>,
    canvas: Canvas,
    art: ArtCache,
    styles: Stylesheet,
    window: Option<Arc<Window>>,
    display: Option<window_display::WindowDisplay>,
    mouse: (i32, i32),
    mouse_down: bool,
    demon_hover_progress: [f32; 3],
    demon_hover_last_tick: Instant,
    demon_hover_animating: bool,
    title_background_current: usize,
    title_background_previous: usize,
    title_background_transition_started: Instant,
    title_background_transition_active: bool,
    title_jugar_entered_at: Instant,
    title_surreal_next_tick: Instant,
    title_last_menu_sel: usize,
    title_has_left_jugar: bool,
    title_surreal_unlocked: bool,
    /// When the title last came up (its entrance), the screen painted last, and each title
    /// place's light (worked out once).
    title_since: Instant,
    last_painted_screen: Option<Screen>,
    title_lights: [Option<u32>; 4],
    /// The title's moving places (`None` until looked for, or when there are no clips).
    title_reel: Option<title_reel::TitleReel>,
    title_reel_tried: bool,
    reduced_motion: bool,
    choice_hover_progress: [f32; 8],
    choice_hover_last_tick: Instant,
    choice_hover_animating: bool,
    /// Scene whose light the interface is wearing, and the fade towards it.
    ambient_key: String,
    ambient_from: [f32; 3],
    ambient_to: [f32; 3],
    ambient_started: Instant,
    ambient_animating: bool,
    ambient_cache: HashMap<String, [f32; 3]>,
    ui_hidden: bool,
    reveal_started: Instant,
    reveal_complete: bool,
    /// A guion chapter's title card is crossing the screen (repaint until it has gone).
    vn_card_animating: bool,
    /// Chiquita alive over the visual novel (her 3D model), and whether the last frame showed her.
    chiquita: vn_chiquita::Live,
    chiquita_shown: bool,
    skipping_dialogue: bool,
    next_dialogue_skip: Instant,
    quick_action_focus: Option<usize>,
    scroll_accumulator: f32,
    continue_available: bool,
    autosave_error: Option<String>,
    restart_requested: bool,
    needs_paint: bool,
    pending_3d: Option<game::WorldEntry>,
    /// Return point for a VN block launched from the embedded 3D world.
    /// Backward/PageUp walks the side-VN history first, then closes that block
    /// and restores this world when the local history is exhausted.
    world_vn_return: Option<game::WorldEntry>,
    world_return_requested: bool,
    pause: Option<pause_menu::PauseMenu>,
    pause_inventory_gold: u32,
    pause_inventory_items: Vec<exploration3d::InventoryMenuItem>,
    /// The grid backpack while the pause menu edits it, whether items can be
    /// combined right now, and whether the world still has to take the edits.
    pause_backpack: Option<exploration3d::backpack::Backpack>,
    pause_backpack_craft: bool,
    /// The story's variables while the pause menu is open (the world's live ones over the
    /// visual novel's): the status page's counters read them.
    pause_story: std::collections::BTreeMap<String, i64>,
    /// What the Queen wears as the menu opens (the wardrobe's slot), for the status page's art.
    pause_outfit: u8,
    /// When the pause menu paints its next moving frame.
    menu_next_frame: Instant,
    /// Counts the window's events: the pause menu repaints in full only after one (or when
    /// something enters), and otherwise just what moves.
    input_generation: u64,
    backpack_dirty: bool,
    menu_action: Option<pause_menu::HostAction>,
    options_return: Screen,
}

impl Default for App {
    fn default() -> Self {
        #[cfg(test)]
        isolate_test_saves();
        let localizer = Localizer::load().unwrap_or_else(|error| {
            eprintln!("no se pudo cargar la localización: {error:#}");
            Localizer::for_language(Language::DEFAULT)
                .expect("el catálogo inglés debe estar incluido")
        });
        let fonts = Fonts::new_for_language(localizer.language().code());
        Self {
            screen: Screen::Title,
            menu_sel: 0,
            choice_sel: 0,
            options_sel: 0,
            gallery_sel: 0,
            quit: false,
            game: None,
            view: None,
            localizer,
            gallery: game::GalleryLibrary::default(),
            gallery_return: Screen::Options,
            fonts,
            native_label_fonts: HashMap::new(),
            canvas: Canvas::new(WIN_W as usize, WIN_H as usize, 0xff120a16),
            art: ArtCache::default(),
            styles: load_menu_styles(),
            window: None,
            display: None,
            mouse: (-1, -1),
            mouse_down: false,
            demon_hover_progress: [0.0; 3],
            demon_hover_last_tick: Instant::now(),
            demon_hover_animating: false,
            title_background_current: 0,
            title_background_previous: 0,
            title_background_transition_started: Instant::now(),
            title_background_transition_active: false,
            title_jugar_entered_at: Instant::now(),
            title_surreal_next_tick: Instant::now() + TITLE_SURREAL_DELAY,
            title_last_menu_sel: 0,
            title_has_left_jugar: false,
            title_surreal_unlocked: false,
            title_since: Instant::now(),
            last_painted_screen: None,
            title_lights: [None; 4],
            title_reel: None,
            title_reel_tried: false,
            reduced_motion: reduced_motion_requested(),
            choice_hover_progress: [0.0; 8],
            choice_hover_last_tick: Instant::now(),
            choice_hover_animating: false,
            ambient_key: String::new(),
            ambient_from: AMBIENT_DEFAULT,
            ambient_to: AMBIENT_DEFAULT,
            ambient_started: Instant::now(),
            ambient_animating: false,
            ambient_cache: HashMap::new(),
            ui_hidden: false,
            reveal_started: Instant::now(),
            reveal_complete: true,
            vn_card_animating: false,
            chiquita: vn_chiquita::Live::new(),
            chiquita_shown: false,
            skipping_dialogue: false,
            next_dialogue_skip: Instant::now(),
            quick_action_focus: None,
            scroll_accumulator: 0.0,
            continue_available: game::has_continue(),
            autosave_error: None,
            restart_requested: false,
            needs_paint: true,
            pending_3d: None,
            world_vn_return: None,
            world_return_requested: false,
            pause: None,
            pause_inventory_gold: 0,
            pause_inventory_items: Vec::new(),
            pause_backpack: None,
            pause_backpack_craft: false,
            pause_story: std::collections::BTreeMap::new(),
            pause_outfit: 0,
            menu_next_frame: Instant::now(),
            input_generation: 0,
            backpack_dirty: false,
            menu_action: None,
            options_return: Screen::Title,
        }
    }
}

fn title_lines() -> [&'static str; 2] {
    [
        "That Time a Heart Attack Reincarnated Me as the Most Feared",
        "Demon Queen of the Dark Continent",
    ]
}

fn app_icon() -> Option<Icon> {
    let image = image::load_from_memory(APP_ICON_PNG).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

fn ease_out_quart(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    1.0 - (1.0 - value).powi(4)
}

fn reduced_motion_requested() -> bool {
    if std::env::var("DEMON_QUEEN_REDUCED_MOTION")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    {
        return true;
    }

    if let Some(value) = localization::saved_reduced_motion() {
        return value;
    }

    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETCLIENTAREAANIMATION,
        };

        let mut animations_enabled = 1_i32;
        if SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            std::ptr::addr_of_mut!(animations_enabled).cast(),
            0,
        ) != 0
        {
            return animations_enabled == 0;
        }
    }

    false
}

fn relaunch_current_executable() -> Result<()> {
    let executable = std::env::current_exe()?;
    let mut command = std::process::Command::new(executable);
    if let Ok(directory) = std::env::current_dir() {
        command.current_dir(directory);
    }
    command.spawn()?;
    Ok(())
}

fn load_menu_styles() -> Stylesheet {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/styles/main_menu.vcss");
    let source = if portable::enabled() {
        portable::menu_styles()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("el portable no contiene data/styles/main_menu.vcss"))
    } else {
        std::fs::read_to_string(&path).map_err(anyhow::Error::from)
    };
    match source.and_then(|source| parse_stylesheet(&source).map_err(anyhow::Error::from)) {
        Ok(styles) => styles,
        Err(error) => {
            eprintln!("menu style fallback ({}): {error}", path.display());
            Stylesheet::default()
        }
    }
}

fn title_menu_layout(styles: &Stylesheet) -> MenuLayout {
    MenuLayout::resolve(
        styles,
        "title-menu",
        MenuLayout {
            x: 660,
            y: 705,
            width: 600,
            height: 69,
            gap: 14,
        },
    )
}

const OPTIONS_GALLERY_INDEX: usize = Language::ALL.len();
const OPTIONS_BACK_INDEX: usize = Language::ALL.len() + 1;
const OPTIONS_ITEM_COUNT: usize = Language::ALL.len() + 2;

fn options_language_rect(index: usize) -> (i32, i32, i32, i32) {
    let column = index / 6;
    let row = index % 6;
    (184 + column as i32 * 520, 350 + row as i32 * 74, 492, 60)
}

fn options_gallery_rect() -> (i32, i32, i32, i32) {
    (1308, 608, 420, 74)
}

fn options_back_rect() -> (i32, i32, i32, i32) {
    (1308, 732, 420, 68)
}

fn rect_contains(rect: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= rect.0 && x <= rect.0 + rect.2 && y >= rect.1 && y <= rect.1 + rect.3
}

fn options_hit_test(x: i32, y: i32) -> Option<usize> {
    (0..Language::ALL.len())
        .find(|&index| rect_contains(options_language_rect(index), x, y))
        .or_else(|| rect_contains(options_gallery_rect(), x, y).then_some(OPTIONS_GALLERY_INDEX))
        .or_else(|| rect_contains(options_back_rect(), x, y).then_some(OPTIONS_BACK_INDEX))
}

const GALLERY_PAGE_SIZE: usize = 8;

fn gallery_list_rect(slot: usize) -> (i32, i32, i32, i32) {
    (68, 258 + slot as i32 * 73, 432, 60)
}

fn gallery_previous_rect() -> (i32, i32, i32, i32) {
    (1370, 914, 205, 58)
}

fn gallery_next_rect() -> (i32, i32, i32, i32) {
    (1590, 914, 250, 58)
}

fn gallery_back_rect() -> (i32, i32, i32, i32) {
    (68, 878, 432, 66)
}

fn end_gallery_rect() -> (i32, i32, i32, i32) {
    (558, 782, 382, 72)
}

fn end_title_rect() -> (i32, i32, i32, i32) {
    (980, 782, 382, 72)
}

fn title_footer_layout(styles: &Stylesheet) -> MenuLayout {
    MenuLayout::resolve(
        styles,
        "title-footer",
        MenuLayout {
            x: 18,
            y: 998,
            width: 1884,
            height: 64,
            gap: 17,
        },
    )
}

fn dialogue_layout(styles: &Stylesheet) -> MenuLayout {
    MenuLayout::resolve(
        styles,
        "dialogue-panel",
        MenuLayout {
            x: 218,
            y: 750,
            width: 1485,
            height: 248,
            gap: 0,
        },
    )
}

/// The colour a speaker wears: their plate, the panel's rim and the mark
/// beside their words all follow it, so a scene reads by who is talking.
#[derive(Clone, Copy)]
struct SpeakerTone {
    /// 0xRRGGBB.
    accent: u32,
    /// Narration: softer text and a quieter plate.
    narrator: bool,
}

fn speaker_tone(name: &str) -> SpeakerTone {
    let lower = name.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));
    let (accent, narrator) = if has(&["narrat", "narrador", "erzähl"]) {
        (0xd9bd8a, true)
    } else if has(&["gorou", "tanaka"]) {
        (0xffb05c, false)
    } else if has(&["reina", "reyna", "queen"]) {
        (0xff6ea0, false)
    } else if has(&["lira"]) {
        (0x6eb0ff, false)
    } else if has(&["nemi"]) {
        (0xbe8cff, false)
    } else if has(&["nyx"]) {
        (0x64e0c8, false)
    } else if has(&["camarer", "waitress"]) {
        (0xeca46e, false)
    } else if has(&["hipnot", "hypnot"]) {
        (0xd65cff, false)
    } else if has(&["mago", "mage", "viejo"]) {
        (0xa08cff, false)
    } else if has(&["funcionario", "clerk"]) {
        (0x96c8aa, false)
    } else {
        // Anyone else gets a stable hue of their own.
        let hash = lower
            .bytes()
            .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
        let hue = (hash % 360) as f32;
        let (s, v) = (0.5f32, 1.0f32);
        let f = |n: f32| {
            let k = (n + hue / 60.0) % 6.0;
            v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
        };
        let to = |c: f32| (c * 255.0).round() as u32;
        ((to(f(5.0)) << 16) | (to(f(3.0)) << 8) | to(f(1.0)), false)
    };
    SpeakerTone { accent, narrator }
}

fn rgb_to_hsv(c: [f32; 3]) -> (f32, f32, f32) {
    let (max, min) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    let d = max - min;
    let h = if d < 1e-6 {
        0.0
    } else if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    };
    (h * 60.0, if max <= 0.0 { 0.0 } else { d / max }, max)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let f = |n: f32| {
        let k = (n + h / 60.0).rem_euclid(6.0);
        v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [f(5.0), f(3.0), f(1.0)]
}

fn rgb_u32(c: [f32; 3]) -> u32 {
    let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    (to(c[0]) << 16) | (to(c[1]) << 8) | to(c[2])
}

/// The light a scene casts on the interface: the colour that covers most of
/// the picture (a hue histogram weighted by saturation and brightness, so
/// black corners and blown-out whites do not win), lifted into a usable,
/// luminous tint.
fn dominant_light(bitmap: &ArtBitmap) -> [f32; 3] {
    dominant_light_with(bitmap.w, bitmap.h, |x, y| {
        let i = (y * bitmap.w + x) * 4;
        [
            bitmap.rgba[i],
            bitmap.rgba[i + 1],
            bitmap.rgba[i + 2],
            bitmap.rgba.get(i + 3).copied().unwrap_or(0),
        ]
    })
}

/// The same, for any picture given as a per-pixel RGBA lookup.
fn dominant_light_with(w: usize, h: usize, pixel: impl Fn(usize, usize) -> [u8; 4]) -> [f32; 3] {
    const BUCKETS: usize = 36;
    let mut weight = [0.0f64; BUCKETS];
    let mut sum = [[0.0f64; 3]; BUCKETS];
    let step = 6;
    for y in (0..h).step_by(step) {
        for x in (0..w).step_by(step) {
            let px = pixel(x, y);
            if px[3] < 128 {
                continue;
            }
            let c = [
                px[0] as f32 / 255.0,
                px[1] as f32 / 255.0,
                px[2] as f32 / 255.0,
            ];
            let (h, s, v) = rgb_to_hsv(c);
            let mut w = (0.25 + s) * (v / 0.35).min(1.0);
            if v > 0.95 && s < 0.12 {
                w *= 0.2;
            }
            let bucket = ((h / 360.0 * BUCKETS as f32) as usize).min(BUCKETS - 1);
            weight[bucket] += w as f64;
            for k in 0..3 {
                sum[bucket][k] += (c[k] * w) as f64;
            }
        }
    }
    let score = |b: usize| {
        weight[(b + BUCKETS - 1) % BUCKETS] * 0.5 + weight[b] + weight[(b + 1) % BUCKETS] * 0.5
    };
    let best = (0..BUCKETS)
        .max_by(|&a, &b| score(a).total_cmp(&score(b)))
        .unwrap_or(0);
    let mut total = 0.0;
    let mut mean = [0.0f64; 3];
    for b in [(best + BUCKETS - 1) % BUCKETS, best, (best + 1) % BUCKETS] {
        total += weight[b];
        for k in 0..3 {
            mean[k] += sum[b][k];
        }
    }
    if total <= 1e-6 {
        return AMBIENT_DEFAULT;
    }
    let mean = [
        (mean[0] / total) as f32,
        (mean[1] / total) as f32,
        (mean[2] / total) as f32,
    ];
    let (h, s, v) = rgb_to_hsv(mean);
    hsv_to_rgb(h, s.clamp(0.45, 0.85), v.max(0.86))
}

const AMBIENT_DEFAULT: [f32; 3] = [1.0, 0.56, 0.75];
/// Seconds a change of scene takes to change the light.
const AMBIENT_FADE: f32 = 0.9;

/// An opaque-colour ARGB with the given alpha byte.
fn mix_argb(alpha: u32, rgb: u32) -> u32 {
    (alpha << 24) | (rgb & 0x00ff_ffff)
}

fn mix_rgb(a: u32, b: u32, t: f32) -> u32 {
    let ch = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xff) as f32, ((b >> shift) & 0xff) as f32);
        ((x + (y - x) * t.clamp(0.0, 1.0)).round() as u32 & 0xff) << shift
    };
    ch(16) | ch(8) | ch(0)
}

fn speaker_layout(styles: &Stylesheet) -> MenuLayout {
    MenuLayout::resolve(
        styles,
        "speaker-plate",
        MenuLayout {
            x: 675,
            y: 686,
            width: 570,
            height: 81,
            gap: 0,
        },
    )
}

fn story_choices_layout(styles: &Stylesheet, count: usize, dialogue_y: i32) -> MenuLayout {
    let mut layout = MenuLayout::resolve(
        styles,
        "story-choices",
        MenuLayout {
            x: 300,
            y: 0,
            width: 1320,
            height: 108,
            gap: 12,
        },
    );
    let total_height = count as i32 * layout.height + count.saturating_sub(1) as i32 * layout.gap;
    let speaker = speaker_layout(styles);
    let choices_bottom = (dialogue_y - 66).min(speaker.y - 60);
    layout.y = choices_bottom - total_height;
    layout
}

fn resume_choices_layout(styles: &Stylesheet) -> MenuLayout {
    MenuLayout::resolve(
        styles,
        "resume-choices",
        MenuLayout {
            x: 300,
            y: 405,
            width: 1320,
            height: 108,
            gap: 14,
        },
    )
}

fn component_style(styles: &Stylesheet, class: &str, element: &str) -> ComputedStyle {
    resolve(
        styles,
        &StyleQuery::class(class).with_element(element.to_string()),
    )
}

fn title_footer_shortcut_style(styles: &Stylesheet, id: &str) -> ComputedStyle {
    resolve(
        styles,
        &StyleQuery::class("shortcut-key")
            .with_element("panel")
            .with_id(id)
            .with_ancestor_class("title-footer"),
    )
}

fn computed_color_argb(style: &ComputedStyle, property: &str, fallback: u32) -> u32 {
    style
        .props
        .get(property)
        .and_then(StyleValue::as_color)
        .map(|color| {
            let alpha = (color.a.clamp(0.0, 1.0) * 255.0).round() as u32;
            (alpha << 24) | ((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32
        })
        .unwrap_or(fallback)
}

fn footer_shortcut_width(fonts: &Fonts, key: &str, label: &str, style: &ComputedStyle) -> i32 {
    let key_size = style.number("font-size", 14.0);
    let label_size = style.number("secondary-font-size", 15.0);
    let padding = style.number("key-padding-x", 9.0).round() as i32;
    let label_gap = style.number("label-gap", 8.0).round() as i32;
    fonts.text_width(key, key_size, true)
        + padding * 2
        + label_gap
        + fonts.text_width(label, label_size, false)
}

fn paint_footer_shortcut(
    canvas: &mut Canvas,
    fonts: &Fonts,
    key: &str,
    label: &str,
    x: i32,
    center_y: i32,
    style: &ComputedStyle,
) -> i32 {
    let key_size = style.number("font-size", 14.0);
    let label_size = style.number("secondary-font-size", 15.0);
    let padding = style.number("key-padding-x", 9.0).round() as i32;
    let label_gap = style.number("label-gap", 8.0).round() as i32;
    let key_height = style.number("height", 27.0).round() as i32;
    let key_text_width = fonts.text_width(key, key_size, true);
    let key_width = key_text_width + padding * 2;
    let key_y = center_y - key_height / 2;

    canvas.styled_panel(x, key_y, key_width, key_height, style);
    canvas.text(
        fonts,
        key,
        x + (key_width - key_text_width) / 2,
        key_y + (key_height - key_size.round() as i32) / 2 - 1,
        key_size,
        computed_color_argb(style, "color", 0xffffeaf4),
        true,
    );
    canvas.text(
        fonts,
        label,
        x + key_width + label_gap,
        center_y - label_size.round() as i32 / 2 - 1,
        label_size,
        computed_color_argb(style, "secondary-color", 0xffd4bcca),
        false,
    );

    footer_shortcut_width(fonts, key, label, style)
}

fn paint_title_footer(
    canvas: &mut Canvas,
    fonts: &Fonts,
    styles: &Stylesheet,
    has_continue: bool,
    localizer: &Localizer,
) {
    let layout = title_footer_layout(styles);
    let footer_style = component_style(styles, "title-footer", "panel");
    canvas.styled_panel(
        layout.x,
        layout.y,
        layout.width,
        layout.height,
        &footer_style,
    );

    let center_y = layout.y + layout.height / 2;
    let primary = computed_color_argb(&footer_style, "color", 0xffffe7f1);
    let secondary = computed_color_argb(&footer_style, "secondary-color", 0xffc9adbc);
    let accent = computed_color_argb(&footer_style, "accent-color", 0xfff3bd73);
    let divider = computed_color_argb(&footer_style, "divider-color", 0xffb85c87);
    let ornament = computed_color_argb(&footer_style, "ornament-color", 0xffd65a8b);
    let gem = computed_color_argb(&footer_style, "gem-color", 0xffb741cf);
    let gem_core = computed_color_argb(&footer_style, "gem-core-color", 0xffffe2f4);
    let brand_size = footer_style.number("font-size", 16.0);
    let meta_size = footer_style.number("secondary-font-size", 14.5);
    let content_padding = footer_style.number("content-padding-x", 26.0).round() as i32;
    let brand_area_width = footer_style.number("brand-area-width", 420.0).round() as i32;

    let sigil_x = layout.x + content_padding + 22;
    canvas.command_bar_sigil(sigil_x, center_y, ornament, accent, gem, gem_core);
    let brand_x = sigil_x + 56;
    let brand = DEVELOPER_CREDIT;
    canvas.text(
        fonts,
        brand,
        brand_x,
        center_y - 20,
        brand_size,
        primary,
        true,
    );
    let meta_x = brand_x;
    let meta = ENGINE_CREDIT;
    canvas.text(
        fonts,
        meta,
        meta_x,
        center_y + 3,
        meta_size,
        secondary,
        false,
    );
    let meta_width = fonts.text_width(meta, meta_size, false);
    let age_style = title_footer_shortcut_style(styles, "age");
    let age_size = age_style.number("font-size", 12.0);
    let age_padding = age_style.number("key-padding-x", 8.0).round() as i32;
    let age_height = age_style.number("height", 24.0).round() as i32;
    let age_width = fonts.text_width("18+", age_size, true) + age_padding * 2;
    let age_x = meta_x + meta_width + 12;
    let age_y = center_y + 1;
    canvas.styled_panel(age_x, age_y, age_width, age_height, &age_style);
    canvas.text(
        fonts,
        "18+",
        age_x + age_padding,
        age_y + (age_height - age_size.round() as i32) / 2 - 1,
        age_size,
        computed_color_argb(&age_style, "color", accent),
        true,
    );

    let brand_divider_x = layout.x + brand_area_width;
    canvas.command_bar_divider(brand_divider_x, center_y, layout.height - 20, divider);

    let hint_gap = layout.gap;
    let group_gap = footer_style.number("control-group-gap", 34.0).round() as i32;
    let hint_widths: Vec<i32> = TITLE_FOOTER_HINTS
        .iter()
        .map(|(id, key, label_key)| {
            let style = title_footer_shortcut_style(styles, id);
            footer_shortcut_width(fonts, key, localizer.ui(label_key), &style)
        })
        .collect();
    let group_split = 2usize;
    let within_group_gaps = TITLE_FOOTER_HINTS.len().saturating_sub(2) as i32 * hint_gap;
    let hints_width = hint_widths.iter().sum::<i32>() + within_group_gaps + group_gap;
    let mut hint_x = layout.x + layout.width - content_padding - hints_width;
    let controls_divider_x = hint_x - 25;
    canvas.command_bar_divider(controls_divider_x, center_y, layout.height - 20, divider);

    let status = if has_continue {
        localizer.ui("title.save_ready")
    } else {
        localizer.ui("title.no_save")
    };
    let status_size = footer_style.number("status-font-size", 12.5);
    let status_width = fonts.text_width(status, status_size, true);
    let status_center_x = (brand_divider_x + controls_divider_x) / 2;
    let status_color = if has_continue {
        computed_color_argb(&footer_style, "status-ready-color", 0xfff0c17c)
    } else {
        computed_color_argb(&footer_style, "status-empty-color", 0xffb99aaa)
    };
    let status_rule = computed_color_argb(&footer_style, "status-rule-color", 0xff8f3f68);
    let status_rule_width = (controls_divider_x - brand_divider_x - 58).max(160);
    canvas.ornament_rule_with_gap(
        status_center_x,
        center_y,
        status_rule_width,
        status_width + 38,
        status_rule,
    );
    canvas.text(
        fonts,
        status,
        status_center_x - status_width / 2,
        center_y - status_size.round() as i32 / 2 - 1,
        status_size,
        status_color,
        true,
    );

    for (index, (id, key, label_key)) in TITLE_FOOTER_HINTS.into_iter().enumerate() {
        if index == group_split {
            let group_divider_x = hint_x + group_gap / 2;
            canvas.command_bar_divider(group_divider_x, center_y, layout.height - 28, divider);
            hint_x += group_gap;
        } else if index > 0 {
            hint_x += hint_gap;
        }
        let style = title_footer_shortcut_style(styles, id);
        hint_x += paint_footer_shortcut(
            canvas,
            fonts,
            key,
            localizer.ui(label_key),
            hint_x,
            center_y,
            &style,
        );
    }
}

fn choice_style(
    styles: &Stylesheet,
    container: &str,
    index: usize,
    enabled: bool,
    selected: bool,
    hovered: bool,
    pressed: bool,
) -> ComputedStyle {
    let mut query = StyleQuery::class("choice-item")
        .with_element("button")
        .with_id(format!("choice-{index}"))
        .with_ancestor_class(container.to_string());
    if selected {
        query = query.with_state("selected").with_state("focus");
    }
    if hovered {
        query = query.with_state("hover");
    }
    if pressed {
        query = query.with_state("active");
    }
    if !enabled {
        query = query.with_state("disabled");
    }
    resolve(styles, &query)
}

fn interactive_panel_style(
    styles: &Stylesheet,
    class: &str,
    id: impl Into<String>,
    selected: bool,
    hovered: bool,
    pressed: bool,
) -> ComputedStyle {
    let mut query = StyleQuery::class(class.to_string())
        .with_element("button")
        .with_id(id.into());
    if selected {
        query = query.with_state("selected").with_state("focus");
    }
    if hovered {
        query = query.with_state("hover");
    }
    if pressed {
        query = query.with_state("active");
    }
    resolve(styles, &query)
}

fn first_enabled_choice(choices: &[game::Choice]) -> usize {
    choices
        .iter()
        .position(|choice| choice.enabled)
        .unwrap_or(0)
}

fn is_inline_continue(view: &game::SceneView) -> bool {
    view.choices.len() == 1 && view.choices[0].id == "continue" && view.choices[0].enabled
}

fn present_world_gate(view: &mut game::SceneView) {
    if view.world_entry.is_some() {
        // Release gating still blocks Act I VN nodes in the story engine.
        // The host offers a final Continue into the playable map instead.
        view.ending = false;
        view.choices = vec![game::Choice {
            id: "continue".into(),
            label: "Continuar".into(),
            enabled: true,
            disabled_reason: None,
        }];
    }
}

fn dialogue_contains(layout: MenuLayout, x: i32, y: i32) -> bool {
    let horizontal_padding = 72;
    let vertical_padding = 34;
    x >= layout.x - horizontal_padding
        && x <= layout.x + layout.width + horizontal_padding
        && y >= layout.y - vertical_padding
        && y <= layout.y + layout.height + vertical_padding
}

fn resume_choice_enabled(index: usize, has_continue: bool) -> bool {
    index != 0 || has_continue
}

#[allow(dead_code)] // The Resume screen's way in (the old title's «Play»).
fn first_resume_choice(has_continue: bool) -> usize {
    if has_continue {
        0
    } else {
        1
    }
}

fn next_resume_choice(current: usize, direction: i32, has_continue: bool) -> usize {
    let mut next = current.min(2);
    for _ in 0..3 {
        next = (next as i32 + direction).rem_euclid(3) as usize;
        if resume_choice_enabled(next, has_continue) {
            return next;
        }
    }
    current.min(2)
}

fn next_enabled_choice(choices: &[game::Choice], current: usize, direction: i32) -> usize {
    if choices.is_empty() {
        return 0;
    }
    let mut next = current.min(choices.len() - 1);
    for _ in 0..choices.len() {
        next = (next as i32 + direction).rem_euclid(choices.len() as i32) as usize;
        if choices[next].enabled {
            return next;
        }
    }
    current.min(choices.len() - 1)
}

fn background_art_key(background: &str) -> String {
    if background.contains('/') || background.contains('\\') {
        background.to_string()
    } else {
        format!("native_1080p/bg/bg_{background}")
    }
}

fn scene_art_key(scene: &str) -> String {
    format!("native_1080p/cg/cg_{scene}")
}

fn legacy_scene_art_key(scene: &str) -> String {
    format!("native_1080p/cg/{scene}")
}

// ── Demon Queen overlay helpers ──────────────────────────────────────
struct DemonOverlayLayout {
    dst_x: i32,
    dst_y: i32,
    dst_w: i32,
    dst_h: i32,
}

fn demon_overlay_layout(art_w: i32, art_h: i32) -> DemonOverlayLayout {
    // Coordenadas nativas: ningún asset del menú se amplía o reduce en runtime.
    let dst_w = art_w;
    let dst_h = art_h;
    let dst_x = (WIN_W as i32 - dst_w) / 2;
    let dst_y = (WIN_H as i32 - dst_h) / 2;
    DemonOverlayLayout {
        dst_x,
        dst_y,
        dst_w,
        dst_h,
    }
}

fn demon_button_local_rects(art_w: i32, art_h: i32) -> [(i32, i32, i32, i32); 3] {
    // Coordenadas normalizadas desde el overlay fuente 614×913.
    let sx = art_w as f32 / 614.0;
    let sy = art_h as f32 / 913.0;
    let scaled = |x: f32, y: f32, w: f32, h: f32| {
        (
            (x * sx).round() as i32,
            (y * sy).round() as i32,
            (w * sx).round() as i32,
            (h * sy).round() as i32,
        )
    };
    [
        scaled(56.0, 493.0, 510.0, 128.0),
        scaled(56.0, 616.0, 510.0, 124.0),
        scaled(56.0, 735.0, 510.0, 125.0),
    ]
}

fn demon_button_rects(
    layout: &DemonOverlayLayout,
    art_w: i32,
    art_h: i32,
) -> [(i32, i32, i32, i32); 3] {
    let local = demon_button_local_rects(art_w, art_h);
    let mut out = [(0, 0, 0, 0); 3];
    for (i, (lx, ly, lw, lh)) in local.iter().enumerate() {
        out[i] = (layout.dst_x + *lx, layout.dst_y + *ly, *lw, *lh);
    }
    out
}

fn demon_overlay_asset_key(art: &mut ArtCache) -> Option<&'static str> {
    // Dedicated 1080p overlay; always blitted 1:1.
    if art.original(DEMON_OVERLAY_KEY).is_some() {
        Some(DEMON_OVERLAY_KEY)
    } else {
        None
    }
}

fn demon_contains(rect: (i32, i32, i32, i32), x: i32, y: i32) -> bool {
    x >= rect.0 && x <= rect.0 + rect.2 && y >= rect.1 && y <= rect.1 + rect.3
}

fn menu_button_style(
    styles: &Stylesheet,
    menu_class: &str,
    item: &MenuItem,
    selected: bool,
    hovered: bool,
    pressed: bool,
) -> ComputedStyle {
    let mut query = StyleQuery::class("menu-item")
        .with_element("button")
        .with_id(item.id)
        .with_ancestor_class(menu_class);
    if selected {
        query = query.with_state("selected").with_state("focus");
    }
    if hovered {
        query = query.with_state("hover");
    }
    if pressed {
        query = query.with_state("active");
    }
    if !item.enabled {
        query = query.with_state("disabled");
    }
    resolve(styles, &query)
}

fn background_image_path(style: &ComputedStyle) -> Option<&str> {
    match style.props.get("background-image") {
        Some(StyleValue::Url(path)) => Some(path),
        _ => None,
    }
}

fn title_menu_items(has_continue: bool) -> [MenuItem; 4] {
    let mut items = MENU_ITEMS;
    items[1].enabled = has_continue;
    items
}

#[allow(dead_code)] // The old title's keys (its tests still use it).
fn next_enabled_menu(current: usize, direction: i32, has_continue: bool) -> usize {
    let items = title_menu_items(has_continue);
    let mut next = current;
    loop {
        next = (next as i32 + direction).rem_euclid(items.len() as i32) as usize;
        if items[next].enabled {
            return next;
        }
    }
}

fn centered_display(
    canvas: &mut Canvas,
    fonts: &Fonts,
    value: &str,
    y: i32,
    size: f32,
    color: u32,
) {
    let x = (WIN_W as i32 - fonts.display_width(value, size)) / 2;
    canvas.display_text(fonts, value, x + 2, y + 3, size, 0xcc170818);
    canvas.display_text(fonts, value, x, y, size, color);
}

fn advance_native_button_hover<const N: usize>(
    progress: &mut [f32; N],
    last_tick: &mut Instant,
    hovered: Option<usize>,
    selected: usize,
    active_count: usize,
    reduced_motion: bool,
) -> bool {
    let now = Instant::now();
    let elapsed = now
        .saturating_duration_since(*last_tick)
        .as_secs_f32()
        .min(0.05);
    *last_tick = now;
    let step = (elapsed / 0.18).clamp(0.0, 1.0);
    let mut animating = false;
    for (index, value) in progress.iter_mut().enumerate() {
        let target = if index >= active_count {
            0.0
        } else if hovered == Some(index) {
            1.0
        } else if hovered.is_none() && selected == index {
            0.18
        } else {
            0.0
        };
        if reduced_motion {
            *value = target;
            continue;
        }
        let delta = target - *value;
        if delta.abs() < 0.004 {
            *value = target;
        } else {
            *value = if delta > 0.0 {
                (*value + step).min(target)
            } else {
                (*value - step).max(target)
            };
            animating = true;
        }
    }
    animating
}

#[allow(clippy::too_many_arguments)]
fn centered_text(
    canvas: &mut Canvas,
    fonts: &Fonts,
    value: &str,
    y: i32,
    size: f32,
    color: u32,
    bold: bool,
) {
    let width = fonts.text_width(value, size, bold);
    canvas.text(
        fonts,
        value,
        (WIN_W as i32 - width) / 2,
        y,
        size,
        color,
        bold,
    );
}

#[allow(clippy::too_many_arguments)]
fn centered_in_rect(
    canvas: &mut Canvas,
    fonts: &Fonts,
    value: &str,
    rect: (i32, i32, i32, i32),
    size: f32,
    color: u32,
    bold: bool,
) {
    let size = fitted_text_size(
        fonts,
        value,
        size,
        (size * 0.68).max(11.0),
        rect.2 - 34,
        bold,
        false,
    );
    let width = fonts.text_width(value, size, bold);
    canvas.text(
        fonts,
        value,
        rect.0 + (rect.2 - width) / 2,
        rect.1 + (rect.3 - size.round() as i32) / 2 - 1,
        size,
        color,
        bold,
    );
}

fn fitted_text_size(
    fonts: &Fonts,
    value: &str,
    preferred: f32,
    minimum: f32,
    max_width: i32,
    bold: bool,
    display: bool,
) -> f32 {
    let mut size = preferred;
    let measure = |size| {
        if display {
            fonts.display_width(value, size)
        } else {
            fonts.text_width(value, size, bold)
        }
    };
    while size > minimum && measure(size) > max_width {
        size = (size - 0.5).max(minimum);
    }
    size
}

fn ellipsize(fonts: &Fonts, value: &str, max_width: i32, size: f32, bold: bool) -> String {
    if fonts.text_width(value, size, bold) <= max_width {
        return value.to_string();
    }
    let suffix = "…";
    let suffix_width = fonts.text_width(suffix, size, bold);
    let mut output = String::new();
    for grapheme in value.graphemes(true) {
        let mut candidate = output.clone();
        candidate.push_str(grapheme);
        if fonts.text_width(&candidate, size, bold) + suffix_width > max_width {
            break;
        }
        output.push_str(grapheme);
    }
    output.push_str(suffix);
    output
}

impl App {
    fn set_title_background(&mut self, index: usize) {
        let index = index.min(TITLE_MENU_BACKGROUNDS.len() - 1);
        if index == self.title_background_current {
            return;
        }

        self.title_background_previous = self.title_background_current;
        self.title_background_current = index;
        self.title_background_transition_started = Instant::now();
        self.title_background_transition_active = !self.reduced_motion;
        if self.reduced_motion {
            self.title_background_previous = index;
        }
        self.needs_paint = true;
    }

    fn jump_to_title_background(&mut self, index: usize) {
        let index = index.min(TITLE_MENU_BACKGROUNDS.len() - 1);
        self.title_background_current = index;
        self.title_background_previous = index;
        self.title_background_transition_started = Instant::now();
        self.title_background_transition_active = false;
    }

    fn set_ui_hidden(&mut self, hidden: bool) {
        if self.ui_hidden == hidden {
            return;
        }
        self.ui_hidden = hidden;
        if hidden {
            self.stop_dialogue_skip();
            self.quick_action_focus = None;
        }
        self.needs_paint = true;
    }

    fn quick_restart(&mut self) {
        self.stop_dialogue_skip();
        self.quick_action_focus = None;
        self.screen = Screen::Title;
        self.menu_sel = 0;
        self.choice_sel = 0;
        self.options_sel = 0;
        self.gallery_sel = 0;
        self.quit = false;
        self.game = None;
        self.view = None;
        self.pause = None;
        self.menu_action = None;
        self.options_return = Screen::Title;
        self.localizer = Localizer::load().unwrap_or_else(|_| {
            Localizer::for_language(Language::DEFAULT)
                .expect("el catálogo inglés debe estar incluido")
        });
        self.fonts = Fonts::new_for_language(self.localizer.language().code());
        self.gallery = game::GalleryLibrary::default();
        self.gallery_return = Screen::Options;
        self.art = ArtCache::default();
        self.styles = load_menu_styles();
        self.mouse = (-1, -1);
        self.mouse_down = false;
        self.demon_hover_progress.fill(0.0);
        self.demon_hover_last_tick = Instant::now();
        self.demon_hover_animating = false;
        self.title_background_current = 0;
        self.title_background_previous = 0;
        self.title_background_transition_started = Instant::now();
        self.title_background_transition_active = false;
        self.title_jugar_entered_at = Instant::now();
        self.title_surreal_next_tick = self.title_jugar_entered_at + TITLE_SURREAL_DELAY;
        self.title_last_menu_sel = 0;
        self.title_has_left_jugar = false;
        self.title_surreal_unlocked = false;
        self.choice_hover_progress.fill(0.0);
        self.choice_hover_last_tick = Instant::now();
        self.choice_hover_animating = false;
        self.ui_hidden = false;
        self.reveal_started = Instant::now();
        self.reveal_complete = true;
        self.scroll_accumulator = 0.0;
        self.continue_available = game::has_continue();
        self.autosave_error = None;
        self.restart_requested = false;
        self.canvas.clear(0xff120a16);
        self.needs_paint = true;
    }

    fn request_restart(&mut self) {
        if self.restart_requested {
            return;
        }
        self.restart_requested = true;
        self.quit = true;
    }

    fn take_restart_request(&mut self) -> bool {
        std::mem::take(&mut self.restart_requested)
    }

    fn paint_title_background_layer(&mut self, index: usize, opacity: f32) -> bool {
        if opacity <= 0.0 {
            return false;
        }
        let requested = TITLE_MENU_BACKGROUNDS[index.min(TITLE_MENU_BACKGROUNDS.len() - 1)];
        let key = if self.art.original(requested).is_some() {
            requested
        } else {
            TITLE_ART_KEY
        };
        let Some(bitmap) = self.art.original(key) else {
            return false;
        };
        if opacity >= 0.999 {
            self.canvas.blit_rgba(0, 0, bitmap);
        } else {
            self.canvas.blit_rgba_adjusted(0, 0, bitmap, 1.0, opacity);
        }
        true
    }

    fn update_demon_hover(&mut self, hovered: Option<usize>) {
        self.demon_hover_animating = advance_native_button_hover(
            &mut self.demon_hover_progress,
            &mut self.demon_hover_last_tick,
            hovered,
            self.menu_sel,
            3,
            self.reduced_motion,
        );
    }

    fn begin_scene_reveal(&mut self) {
        // Keep only assets needed by the scene being revealed.  Without this
        // boundary, every visited VN image stayed decoded for the whole run.
        self.art.clear();
        self.quick_action_focus = None;
        self.reveal_started = Instant::now();
        self.reveal_complete = false;
        self.choice_hover_progress.fill(0.0);
        self.choice_hover_last_tick = Instant::now();
        self.choice_hover_animating = false;
        self.needs_paint = true;
    }

    fn world_vn_active(&self) -> bool {
        self.world_vn_return.is_some()
    }

    fn request_world_return(&mut self) {
        if self.world_vn_active() {
            self.stop_dialogue_skip();
            self.world_return_requested = true;
            self.needs_paint = false;
        }
    }

    fn take_world_return(&mut self) -> Option<game::WorldEntry> {
        if !self.world_return_requested {
            return None;
        }
        self.world_return_requested = false;
        self.world_vn_return.take()
    }

    /// Open a VN block launched from the embedded 3D world.  This keeps the
    /// same story runtime, rollback/autosave behaviour and procedural dialogue
    /// chrome as the prologue instead of maintaining a second HTML renderer.
    fn open_world_vn(&mut self, node: &str) -> Result<()> {
        self.stop_dialogue_skip();
        let return_entry = self.view.as_ref().and_then(|view| view.world_entry.clone());
        let return_entry = return_entry.or_else(|| {
            self.game
                .as_ref()
                .and_then(game::Game::world_snapshot)
                .map(|snapshot| game::WorldEntry {
                    world: snapshot.world.clone(),
                    spawn: snapshot.spawn.clone(),
                })
        });
        let return_entry =
            return_entry.ok_or_else(|| anyhow!("no hay un punto de retorno 3D para este VN"))?;
        let game = self
            .game
            .as_mut()
            .ok_or_else(|| anyhow!("no hay una partida activa para abrir el VN"))?;
        game.jump_to_node(node)?;
        let mut view = game.view()?;
        self.localizer.localize_scene(&mut view);
        present_world_gate(&mut view);
        self.choice_sel = first_enabled_choice(&view.choices);
        self.view = Some(view);
        self.screen = Screen::Play;
        self.ui_hidden = false;
        self.world_vn_return = Some(return_entry);
        self.world_return_requested = false;
        self.begin_scene_reveal();
        Ok(())
    }

    fn start_game(&mut self) -> Result<()> {
        let mut game = game::Game::load()?;
        game.new_game()?;
        self.enter_game(game)
    }

    fn continue_game(&mut self) -> Result<()> {
        let mut game = game::Game::load()?;
        game.continue_game()?;
        self.enter_game(game)
    }

    fn enter_game(&mut self, mut game: game::Game) -> Result<()> {
        self.stop_dialogue_skip();
        self.quick_action_focus = None;
        let mut view = game.view()?;
        self.localizer.localize_scene(&mut view);
        present_world_gate(&mut view);
        let ending = view.ending;
        let world_entry = view.world_entry.clone();
        let autosave_error = game.take_autosave_error();
        self.choice_sel = first_enabled_choice(&view.choices);
        self.game = Some(game);
        self.view = Some(view);
        if let Some(error) = &autosave_error {
            eprintln!("autosave no disponible: {error}");
        }
        self.autosave_error = autosave_error;
        self.continue_available = game::has_continue();
        self.ui_hidden = false;
        if let Some(entry) = world_entry.filter(|_| {
            self.game
                .as_ref()
                .is_some_and(|game| game.exploration_state().is_some())
        }) {
            self.enter_exploration(&entry)?;
        } else if ending {
            self.refresh_gallery();
            self.screen = Screen::End;
            self.reveal_complete = true;
            self.needs_paint = true;
        } else {
            self.screen = Screen::Play;
            self.begin_scene_reveal();
        }
        Ok(())
    }

    fn enter_exploration(&mut self, entry: &game::WorldEntry) -> Result<()> {
        // VN art is no longer needed once the map takes over the same window.
        self.art.clear();
        if let Some(game) = self.game.as_mut() {
            game.finish_world_vn();
        }
        self.world_vn_return = None;
        self.world_return_requested = false;
        self.prepare_3d_session(entry)?;
        self.pending_3d = Some(entry.clone());
        self.screen = Screen::Explore;
        self.needs_paint = false;
        Ok(())
    }

    fn prepare_3d_session(&mut self, entry: &game::WorldEntry) -> Result<()> {
        let saved = self.game.as_ref().and_then(game::Game::exploration_state);
        let previous = self
            .game
            .as_ref()
            .and_then(|g| g.world_snapshot().cloned())
            .or_else(|| exploration3d_state::load().ok().flatten());
        let mut session = exploration3d_state::State::new(
            entry.world.clone(),
            entry.spawn.clone(),
            exploration3d_state::default_spawn(entry.spawn.as_str()).0,
            exploration3d_state::default_spawn(entry.spawn.as_str()).1,
        );
        // The story's variables: what the world had, then what the visual novel says now.
        if let Some(previous) = previous.as_ref() {
            session.story = previous.story.clone();
        }
        if let Some(game) = self.game.as_ref() {
            for (key, value) in game.story_variables() {
                session.story.values.insert(
                    key.clone(),
                    (*value).clamp(i32::MIN as i64, i32::MAX as i64) as i32,
                );
            }
        }
        if let Some(saved) = saved.filter(|saved| saved.world == entry.world) {
            if let Some(previous) =
                previous.filter(|old| old.world == entry.world && old.spawn == entry.spawn)
            {
                session.nature_mode = previous.nature_mode;
                session.village_entered = previous.village_entered;
                // Keep the wardrobe selection when returning from the VN or
                // loading the 3D scene. Without this copy the session starts
                // at outfit 0 even though the checkpoint contains outfit 1.
                session.outfit = previous.outfit;
            }
            session.x = saved.x_milli as f32 / 1000.0;
            session.z = saved.y_milli as f32 / 1000.0;
            session.yaw =
                (saved.facing_x_milli as f32 / 1000.0).atan2(saved.facing_y_milli as f32 / 1000.0);
            session.seen_interactions = saved.seen_interactions.clone();
        }
        exploration3d_state::write(&mut session)
    }

    #[allow(dead_code)] // The old title's «Play»: the Resume screen.
    fn open_game(&mut self) -> Result<()> {
        self.continue_available = game::has_continue();
        self.screen = Screen::Resume;
        self.choice_sel = first_resume_choice(self.continue_available);
        self.choice_hover_progress.fill(0.0);
        self.choice_hover_last_tick = Instant::now();
        self.needs_paint = true;
        Ok(())
    }

    fn open_options(&mut self) {
        self.refresh_gallery();
        self.ensure_native_label_fonts();
        self.options_sel = Language::ALL
            .iter()
            .position(|language| *language == self.localizer.language())
            .unwrap_or(0);
        self.screen = Screen::Options;
        self.ui_hidden = false;
        self.needs_paint = true;
    }

    fn ensure_native_label_fonts(&mut self) {
        for language in Language::ALL {
            let profile = language.font_profile();
            self.native_label_fonts
                .entry(profile)
                .or_insert_with(|| NativeLabelFont::new_for_language(profile));
        }
    }

    fn refresh_localized_view(&mut self) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let Ok(mut view) = game.view() else {
            return;
        };
        self.localizer.localize_scene(&mut view);
        self.choice_sel = first_enabled_choice(&view.choices);
        self.view = Some(view);
    }

    fn select_language(&mut self, index: usize) {
        let Some(language) = Language::ALL.get(index).copied() else {
            return;
        };
        if language == self.localizer.language() {
            return;
        }
        if let Err(error) = self.localizer.set_language(language) {
            eprintln!(
                "no se pudo guardar el idioma {}: {error:#}",
                language.code()
            );
        }
        self.fonts = Fonts::new_for_language(language.code()).or_else(Fonts::new);
        self.refresh_localized_view();
        self.needs_paint = true;
    }

    fn preview_language(&mut self, code: &str) -> Result<()> {
        let language = Language::from_code(code)
            .ok_or_else(|| anyhow!("idioma de captura desconocido: {code}"))?;
        self.localizer = Localizer::for_language(language)?;
        self.fonts = Fonts::new_for_language(language.code()).or_else(Fonts::new);
        self.refresh_localized_view();
        Ok(())
    }

    fn refresh_gallery(&mut self) {
        let library = self
            .game
            .as_ref()
            .map(game::Game::gallery_library)
            .unwrap_or_else(game::gallery_library);
        match library {
            Ok(library) => {
                self.gallery = library;
                self.gallery_sel = self
                    .gallery_sel
                    .min(self.gallery.entries.len().saturating_sub(1));
            }
            Err(error) => {
                eprintln!("no se pudo abrir la galería: {error:#}");
                self.gallery = game::GalleryLibrary::default();
                self.gallery_sel = 0;
            }
        }
    }

    fn open_gallery(&mut self, return_to: Screen) {
        self.refresh_gallery();
        self.gallery_return = return_to;
        self.gallery_sel = self
            .gallery_sel
            .min(self.gallery.entries.len().saturating_sub(1));
        self.screen = Screen::Gallery;
        self.ui_hidden = false;
        self.needs_paint = true;
    }

    fn leave_gallery(&mut self) {
        self.screen = self.gallery_return;
        self.ui_hidden = false;
        self.needs_paint = true;
    }

    fn activate_resume_choice(&mut self) {
        if !resume_choice_enabled(self.choice_sel, self.continue_available) {
            return;
        }
        let result = match self.choice_sel {
            0 => self.continue_game(),
            1 => self.start_game(),
            2 => {
                self.screen = Screen::Title;
                self.needs_paint = true;
                Ok(())
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            eprintln!("no se pudo abrir la partida: {error:#}");
            self.autosave_error = Some(format!("{error:#}"));
            self.continue_available = game::has_continue();
            self.needs_paint = true;
        }
    }

    fn confirm_choice(&mut self) {
        // Present the prologue's last illustration/dialogue before the user
        // advances into exploration. Keep the save at this authored gate.
        if let Some(entry) = self.view.as_ref().and_then(|view| view.world_entry.clone()) {
            if let Err(error) = self.enter_exploration(&entry) {
                self.autosave_error = Some(format!("{error:#}"));
                self.needs_paint = true;
            }
            return;
        }
        let index = self.choice_sel;
        let Some(view) = self.view.as_ref() else {
            return;
        };
        if view.choices.is_empty() || index >= view.choices.len() {
            return;
        }
        let choice = &view.choices[index];
        if !choice.enabled {
            eprintln!(
                "decisión {} deshabilitada: {}",
                choice.id,
                choice
                    .disabled_reason
                    .as_deref()
                    .unwrap_or("condición no cumplida")
            );
            return;
        }
        let choice_id = choice.id.clone();
        let Some(game) = self.game.as_mut() else {
            return;
        };
        exploration3d::ui_sfx::play(exploration3d::ui_sfx::UiSfx::Select);
        if let Err(error) = game.decide(index) {
            eprintln!("falló la decisión {choice_id}: {error:#}");
            return;
        }
        let autosave_error = game.take_autosave_error();
        let Ok(mut next) = game.view() else {
            self.screen = Screen::Title;
            return;
        };
        self.localizer.localize_scene(&mut next);
        present_world_gate(&mut next);
        let reached_end = next.ending;
        self.choice_sel = first_enabled_choice(&next.choices);
        self.view = Some(next);
        if let Some(error) = &autosave_error {
            eprintln!("autosave no disponible: {error}");
        }
        self.autosave_error = autosave_error;
        self.continue_available = game::has_continue();
        if reached_end {
            self.refresh_gallery();
            self.screen = Screen::End;
            self.reveal_complete = true;
            self.needs_paint = true;
        } else {
            self.begin_scene_reveal();
        }
    }

    fn advance_play(&mut self) {
        if self.screen != Screen::Play {
            return;
        }
        if !self.reveal_complete {
            self.reveal_complete = true;
            self.needs_paint = true;
            return;
        }
        if self.view.as_ref().is_some_and(is_inline_continue) {
            self.choice_sel = 0;
            self.confirm_choice();
        }
    }

    fn rollback_scene(&mut self) {
        self.stop_dialogue_skip();
        if !matches!(self.screen, Screen::Play | Screen::End) {
            return;
        }
        let world_vn = self.world_vn_active();
        let Some(game) = self.game.as_mut() else {
            return;
        };
        let rolled_back = match if world_vn {
            game.rollback_world_vn()
        } else {
            game.rollback()
        } {
            Ok(value) => value,
            Err(error) => {
                eprintln!("no se pudo retroceder: {error:#}");
                return;
            }
        };
        if !rolled_back {
            if world_vn {
                // Only leave the 3D interaction once its own dialogue history
                // is exhausted.  Earlier wheel turns stay inside the VN.
                self.request_world_return();
            }
            return;
        }
        let autosave_error = game.take_autosave_error();
        let Ok(mut previous) = game.view() else {
            return;
        };
        self.localizer.localize_scene(&mut previous);
        present_world_gate(&mut previous);
        let ending = previous.ending;
        self.choice_sel = first_enabled_choice(&previous.choices);
        self.view = Some(previous);
        self.art.clear();
        self.reveal_complete = true;
        self.reveal_started = Instant::now();
        self.screen = if ending { Screen::End } else { Screen::Play };
        if let Some(error) = &autosave_error {
            eprintln!("autosave no disponible tras retroceder: {error}");
        }
        self.autosave_error = autosave_error;
        self.continue_available = game::has_continue();
        self.needs_paint = true;
    }

    fn handle_scroll(&mut self, delta_y: f32) {
        if delta_y == 0.0 {
            return;
        }
        self.stop_dialogue_skip();
        self.scroll_accumulator += delta_y;
        if self.scroll_accumulator.abs() < SCROLL_ACTION_THRESHOLD {
            return;
        }
        let scroll_up = self.scroll_accumulator > 0.0;
        self.scroll_accumulator = 0.0;
        match self.screen {
            Screen::GameMenu => self.menu_scroll(scroll_up),
            Screen::Play | Screen::End if scroll_up => self.rollback_scene(),
            Screen::Play => self.advance_play(),
            Screen::Resume => {
                let direction = if scroll_up { -1 } else { 1 };
                self.choice_sel =
                    next_resume_choice(self.choice_sel, direction, self.continue_available);
                self.needs_paint = true;
            }
            Screen::Title => self.title_scroll(scroll_up),
            Screen::Options => {
                let direction = if scroll_up { -1 } else { 1 };
                self.options_sel = (self.options_sel as i32 + direction)
                    .rem_euclid(Language::ALL.len() as i32 + 2)
                    as usize;
                self.needs_paint = true;
            }
            Screen::Gallery => {
                if !self.gallery.entries.is_empty() {
                    let direction = if scroll_up { -1 } else { 1 };
                    self.gallery_sel = (self.gallery_sel as i32 + direction)
                        .rem_euclid(self.gallery.entries.len() as i32)
                        as usize;
                    self.needs_paint = true;
                }
            }
            Screen::Explore => {}
            Screen::End => {}
        }
    }

    fn paint(&mut self) {
        self.demon_hover_animating = false;
        self.choice_hover_animating = false;
        self.ambient_animating = false;
        title_menu::note_screen(self);
        match self.screen {
            Screen::GameMenu => self.paint_game_menu(),
            Screen::Title => self.paint_title_persona(),
            Screen::Resume => self.paint_resume(),
            Screen::Options => self.paint_options(),
            Screen::Gallery => self.paint_gallery(),
            Screen::Play => self.paint_play(),
            Screen::Explore => self.canvas.clear(0xff09060e),
            Screen::End => self.paint_end(),
        }
        if matches!(self.screen, Screen::Explore | Screen::End | Screen::Resume) {
            self.paint_menu_trigger();
        }
        if self.screen == Screen::Title {
            if let (Some(fonts), Some(error)) = (&self.fonts, &self.autosave_error) {
                if error.starts_with("3D:") {
                    self.canvas.rect(180, 920, 1560, 100, 0xed170c15);
                    self.canvas
                        .text(fonts, error, 205, 939, 23.0, 0xffffcfaa, false);
                }
            }
        }
        let want = self.chiquita_want();
        let typing = !self.reveal_complete;
        if let Some(display) = &mut self.display {
            if self.screen == Screen::Play && !vn_web::classic() {
                self.chiquita
                    .prepare(&display.gpu.device, &display.gpu.queue);
            }
            self.chiquita.update(want, typing);
            let over = self
                .chiquita
                .render(&display.gpu.device, &display.gpu.queue);
            self.chiquita_shown = over.is_some();
            let presented = match over {
                Some(over) => {
                    display.present_canvas_over(Some(&self.canvas.pixels), 1.0, Some(over))
                }
                None => display.present_canvas(&self.canvas.pixels, 1.0),
            };
            match presented {
                Ok(true) => {}
                result => {
                    if let Err(error) = result {
                        eprintln!("[game] presentación: {error:#}");
                    }
                    self.needs_paint = true;
                    return;
                }
            }
        }
        self.needs_paint = (self.screen == Screen::Play
            && (!self.reveal_complete || self.vn_card_animating))
            || self.demon_hover_animating
            || self.choice_hover_animating
            || self.ambient_animating
            || self.title_background_transition_active;
    }

    /// What the visual novel's node wants of Chiquita alive: on stage where her pose was, or popping
    /// up in her corner for her own lines (not while the interface is hidden, nor in a room whose
    /// pictures show her already, nor before a chapter's title card has gone).
    fn chiquita_want(&self) -> Option<vn_chiquita::Want> {
        if self.screen != Screen::Play || vn_web::classic() || self.vn_card_animating {
            return None;
        }
        let want = vn_chiquita::want(self.view.as_ref()?)?;
        (want.on_stage() || !self.ui_hidden).then_some(want)
    }

    /// She is alive over the visual novel and moving (a frame every 16 ms).
    fn chiquita_moving(&self) -> bool {
        self.screen == Screen::Play && (self.chiquita.animating() || self.chiquita_shown)
    }

    /// Only Chiquita has moved: the canvas as it was, with her new picture over it.
    fn present_chiquita(&mut self) {
        let want = self.chiquita_want();
        self.chiquita.update(want, !self.reveal_complete);
        let Some(display) = &mut self.display else {
            return;
        };
        let over = self
            .chiquita
            .render(&display.gpu.device, &display.gpu.queue);
        self.chiquita_shown = over.is_some();
        if let Err(error) = display.present_canvas_over(None, 1.0, over) {
            eprintln!("[game] presentación: {error:#}");
        }
    }

    /// The light the interface wears for the scene drawn from `key`, fading
    /// from the previous scene's light when the scene changes.
    fn ambient_light(&mut self, key: &str) -> [f32; 3] {
        if !self.ambient_cache.contains_key(key) {
            let light = self
                .art
                .original(key)
                .map_or(AMBIENT_DEFAULT, dominant_light);
            self.ambient_cache.insert(key.to_string(), light);
        }
        let target = self.ambient_cache[key];
        if self.ambient_key != key {
            let first = self.ambient_key.is_empty();
            self.ambient_from = if first || self.reduced_motion {
                target
            } else {
                self.ambient_now()
            };
            self.ambient_to = target;
            self.ambient_started = Instant::now();
            self.ambient_key = key.to_string();
        }
        self.ambient_now_animating()
    }

    fn ambient_progress(&self) -> f32 {
        let t = (self.ambient_started.elapsed().as_secs_f32() / AMBIENT_FADE).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn ambient_now(&self) -> [f32; 3] {
        let t = self.ambient_progress();
        let mut out = [0.0; 3];
        for k in 0..3 {
            out[k] = self.ambient_from[k] + (self.ambient_to[k] - self.ambient_from[k]) * t;
        }
        out
    }

    fn ambient_now_animating(&mut self) -> [f32; 3] {
        self.ambient_animating =
            self.ambient_started.elapsed().as_secs_f32() < AMBIENT_FADE && !self.reduced_motion;
        self.ambient_now()
    }

    fn paint_title(&mut self) {
        let demon_key = demon_overlay_asset_key(&mut self.art);
        let demon_active = demon_key.is_some();
        let demon_hovered = demon_key.and_then(|key| {
            let (ow, oh) = self
                .art
                .original(key)
                .map(|bitmap| (bitmap.w as i32, bitmap.h as i32))?;
            let layout = demon_overlay_layout(ow, oh);
            demon_button_rects(&layout, ow, oh)
                .iter()
                .position(|rect| demon_contains(*rect, self.mouse.0, self.mouse.1))
        });
        // ── Dinámico para Jugar: alterna garden ↔ surreal_poppy cada INTERVAL tras DELAY inicial, + al volver desde otro botón reinicia ciclo ──
        let now = Instant::now();
        if self.menu_sel != self.title_last_menu_sel {
            if self.menu_sel == 0 {
                self.title_jugar_entered_at = now;
                self.title_surreal_next_tick = now + TITLE_SURREAL_DELAY;
            }
            self.title_last_menu_sel = self.menu_sel;
        }
        let base_selected = if demon_active {
            self.menu_sel.min(2)
        } else {
            match self.menu_sel {
                0 | 1 => 0,
                2 => 1,
                _ => 2,
            }
        };
        // Cálculo dinámico para Jugar: garden → poppy → eyes → violet → house cloud → cosmic trail → smoking → garden cada INTERVAL
        let jugar_dynamic = || {
            let elapsed = now.duration_since(self.title_jugar_entered_at);
            if elapsed < TITLE_SURREAL_DELAY {
                0
            } else {
                let cycle = ((elapsed - TITLE_SURREAL_DELAY).as_millis()
                    / TITLE_SURREAL_INTERVAL.as_millis())
                    % 7;
                match cycle {
                    0 => 3, // poppy amapolas
                    1 => 4, // eyes closeup
                    2 => 5, // violet blinds
                    3 => 6, // house cloud alive
                    4 => 7, // cosmic trail
                    5 => 8, // smoking
                    _ => 0, // garden
                }
            }
        };
        let selected_background = if self.menu_sel == 0 && base_selected == 0 {
            jugar_dynamic()
        } else {
            base_selected
        };
        let hover_background = demon_hovered.map(|h| {
            if h == 0 {
                // Hover sobre Jugar también es dinámico
                jugar_dynamic()
            } else {
                h.min(TITLE_MENU_BACKGROUNDS.len() - 1)
            }
        });
        if !self.ui_hidden {
            let background_index = hover_background.unwrap_or(selected_background);
            self.set_title_background(background_index);
        }
        if self.menu_sel == 0 && now >= self.title_surreal_next_tick {
            while self.title_surreal_next_tick <= now {
                self.title_surreal_next_tick += TITLE_SURREAL_INTERVAL;
            }
        }

        let elapsed = self.title_background_transition_started.elapsed();
        let fade = if self.title_background_transition_active {
            let linear = elapsed.as_secs_f32() / TITLE_BACKGROUND_FADE.as_secs_f32();
            if linear >= 1.0 {
                self.title_background_transition_active = false;
                self.title_background_previous = self.title_background_current;
                None
            } else {
                Some(ease_out_quart(linear))
            }
        } else {
            None
        };

        self.canvas.vgradient(0xff0d0611, 0xff2a1030);
        let painted = if let Some(opacity) = fade {
            let previous = self.paint_title_background_layer(self.title_background_previous, 1.0);
            let current = self.paint_title_background_layer(self.title_background_current, opacity);
            previous || current
        } else {
            self.paint_title_background_layer(self.title_background_current, 1.0)
        };
        if !painted {
            // Two moons of the Dark Continent.
            self.canvas.circle(1560, 195, 96, 0xffc9a4d4);
            self.canvas.circle(1680, 255, 60, 0xff8f6aa8);
            self.canvas.circle(1584, 177, 21, 0xff0d0611);
        }
        if self.ui_hidden {
            return;
        }
        if painted {
            if demon_active {
                self.canvas
                    .rect(0, 0, WIN_W as i32, WIN_H as i32, 0x52080612);
            } else {
                // Gentle scrims unify title and menu without flattening the key art.
                self.canvas
                    .rect(0, 0, WIN_W as i32, WIN_H as i32, 0x12120718);
                self.canvas.rect(0, 0, WIN_W as i32, 308, 0x3d090511);
            }
        }

        // ── Si existe el overlay DEMON QUEEN proporcionado por el usuario,
        //    lo usamos como menú principal cristalino.
        if demon_active {
            // Cargamos la misma versión que se va a pintar para que imagen,
            // hitboxes y hover compartan exactamente las mismas coordenadas.
            let key = demon_key.expect("overlay checked above");
            let (ow, oh) = {
                let bmp = self.art.original(key).expect("overlay key must resolve");
                (bmp.w as i32, bmp.h as i32)
            };
            let layout = demon_overlay_layout(ow, oh);
            if let Some(bmp) = self.art.original(key) {
                debug_assert_eq!((layout.dst_w, layout.dst_h), (ow, oh));
                self.canvas.blit_rgba(layout.dst_x, layout.dst_y, bmp);
            }

            let rects = demon_button_rects(&layout, ow, oh);
            let hovered = demon_hovered;
            self.update_demon_hover(hovered);
            if let Some(bmp) = self.art.original(key) {
                let sources = demon_button_local_rects(ow, oh);
                for index in 0..rects.len() {
                    self.canvas.native_button_feedback(
                        bmp,
                        sources[index],
                        rects[index],
                        self.demon_hover_progress[index],
                        hovered == Some(index) && self.mouse_down,
                    );
                }
            }

            // Localized subtitle under the English product logo.
            if let Some(fonts) = self.fonts.as_ref() {
                let sub = self.localizer.ui("title.subtitle");
                let sw = fonts.text_width(sub, 15.0, false);
                let sub_y = layout.dst_y + 353;
                self.canvas.text(
                    fonts,
                    sub,
                    (WIN_W as i32 - sw) / 2,
                    sub_y,
                    15.0,
                    0x55f0d8e8,
                    false,
                );
            }
        } else {
            let Some(fonts) = self.fonts.as_ref() else {
                // necesitamos fonts para los botones VCSS; si no hay, igual seguimos para footer
                return;
            };
            // Ceremonial title lockup, matching the throne's symmetry.
            centered_display(
                &mut self.canvas,
                fonts,
                "That Time a Heart Attack Reincarnated Me",
                36,
                46.5,
                0xffff91c2,
            );
            centered_display(&mut self.canvas, fonts, "as the", 92, 25.5, 0xffffc6dc);
            self.canvas
                .ornament_rule_with_gap(960, 111, 540, 177, 0xffb84b7b);
            centered_display(
                &mut self.canvas,
                fonts,
                "Most Feared Demon Queen",
                122,
                55.5,
                0xffff86b9,
            );
            centered_display(
                &mut self.canvas,
                fonts,
                "of the Dark Continent",
                182,
                51.0,
                0xffffa0c8,
            );
            self.canvas.ornament_rule(960, 240, 750, 0xffd65a8b);
            centered_display(
                &mut self.canvas,
                fonts,
                "(And I Discovered the Royal Throne Has Armrests)",
                252,
                22.5,
                0xffead4df,
            );

            // VCSS owns menu geometry, raster chrome and live pseudo states.
            let layout = title_menu_layout(&self.styles);
            let menu_items = title_menu_items(self.continue_available);
            for (index, item) in menu_items.iter().enumerate() {
                let hovered = layout.contains(index, self.mouse.0, self.mouse.1);
                let style = menu_button_style(
                    &self.styles,
                    "title-menu",
                    item,
                    self.menu_sel == index,
                    hovered,
                    hovered && self.mouse_down,
                );
                let plate =
                    background_image_path(&style).and_then(|path| self.art.original_trimmed(path));
                self.canvas.styled_button(
                    fonts,
                    plate,
                    layout.x,
                    layout.item_y(index),
                    layout.width,
                    layout.height,
                    item.label,
                    item.status,
                    &style,
                );
            }
        }
        if let Some(fonts) = self.fonts.as_ref() {
            paint_title_footer(
                &mut self.canvas,
                fonts,
                &self.styles,
                self.continue_available,
                &self.localizer,
            );
        }
    }

    fn paint_resume(&mut self) {
        let has_resume_background = self.art.original(RESUME_BACKGROUND_KEY).is_some();
        let background = if has_resume_background {
            self.art.original(RESUME_BACKGROUND_KEY)
        } else {
            self.art.original(TITLE_ART_KEY)
        };
        if let Some(art) = background {
            self.canvas.blit_rgba(0, 0, art);
        } else {
            self.canvas.vgradient(0xff0d0611, 0xff2a1030);
        }
        if self.ui_hidden {
            return;
        }
        self.canvas
            .rect(0, 0, WIN_W as i32, WIN_H as i32, 0xb808050e);
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let has_continue = self.continue_available;
        self.canvas.gothic_crest(960, 126, 0xffff9ac9);
        centered_display(
            &mut self.canvas,
            fonts,
            if has_continue {
                self.localizer.ui("resume.title_ready")
            } else {
                self.localizer.ui("resume.title_new")
            },
            166,
            52.0,
            0xffffd8e8,
        );
        self.canvas
            .ornament_rule_with_gap(960, 241, 720, 350, 0xffc75b8e);
        centered_text(
            &mut self.canvas,
            fonts,
            if has_continue {
                self.localizer.ui("resume.subtitle_ready")
            } else {
                self.localizer.ui("resume.subtitle_new")
            },
            275,
            22.0,
            0xffd9bdcc,
            false,
        );
        centered_text(
            &mut self.canvas,
            fonts,
            if has_continue {
                self.localizer.ui("resume.autosave")
            } else {
                self.localizer.ui("resume.no_save")
            },
            326,
            15.0,
            if has_continue { 0xffffc690 } else { 0xffa9909e },
            true,
        );

        let labels = [
            self.localizer.ui("resume.continue"),
            self.localizer.ui("resume.new"),
            self.localizer.ui("resume.back"),
        ];
        let layout = resume_choices_layout(&self.styles);
        let hovered_choice = (0..labels.len()).find(|&index| {
            resume_choice_enabled(index, has_continue)
                && layout.contains(index, self.mouse.0, self.mouse.1)
        });
        self.choice_hover_animating = advance_native_button_hover(
            &mut self.choice_hover_progress,
            &mut self.choice_hover_last_tick,
            hovered_choice,
            self.choice_sel,
            labels.len(),
            self.reduced_motion,
        );
        for (index, label) in labels.iter().enumerate() {
            let enabled = resume_choice_enabled(index, has_continue);
            let hovered = hovered_choice == Some(index);
            let style = choice_style(
                &self.styles,
                "resume-choices",
                index,
                enabled,
                self.choice_sel == index,
                hovered,
                hovered && self.mouse_down,
            );
            self.canvas.styled_choice_button(
                fonts,
                layout.x,
                layout.item_y(index),
                layout.width,
                layout.height,
                label,
                index + 1,
                self.choice_sel == index,
                self.choice_hover_progress[index],
                hovered && self.mouse_down,
                ChoiceTreatment::Narrative,
                &style,
            );
            let status = match index {
                0 if has_continue => self.localizer.ui("resume.last_save"),
                0 => self.localizer.ui("resume.unavailable"),
                1 => self.localizer.ui("resume.from_start"),
                _ => self.localizer.ui("resume.return"),
            };
            let status_size = 14.0;
            let status_width = fonts.text_width(status, status_size, true);
            self.canvas.text(
                fonts,
                status,
                layout.x + layout.width - status_width - 72,
                layout.item_y(index) + (layout.height - status_size.round() as i32) / 2 + 3,
                status_size,
                if enabled { 0xffe2b7ca } else { 0xff8f7d89 },
                true,
            );
        }
        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("resume.controls"),
            836,
            18.0,
            0xffc5aebe,
            false,
        );
        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("resume.story_controls"),
            872,
            16.0,
            0xff9f8797,
            false,
        );
    }

    fn paint_options(&mut self) {
        self.ensure_native_label_fonts();
        let has_options_background = self.art.original(OPTIONS_BACKGROUND_KEY).is_some();
        let background = if has_options_background {
            self.art.original(OPTIONS_BACKGROUND_KEY)
        } else {
            self.art.original(TITLE_ART_KEY)
        };
        if let Some(art) = background {
            self.canvas.blit_rgba(0, 0, art);
        } else {
            self.canvas.vgradient(0xff0d0611, 0xff2a1030);
        }
        if self.ui_hidden {
            return;
        }
        self.canvas
            .rect(0, 0, WIN_W as i32, WIN_H as i32, 0xc408040e);
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let shell = component_style(&self.styles, "options-shell", "panel");
        self.canvas.styled_panel(104, 116, 1712, 826, &shell);
        self.canvas.gothic_crest(960, 108, 0xffff8ec1);
        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("options.kicker"),
            148,
            14.0,
            0xffd998b7,
            true,
        );
        centered_display(
            &mut self.canvas,
            fonts,
            self.localizer.ui("options.title"),
            174,
            55.0,
            0xffffd9e8,
        );
        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("options.subtitle"),
            241,
            18.0,
            0xffc9a9b9,
            false,
        );
        self.canvas.ornament_rule(960, 282, 1390, 0xffb74f7d);

        let section = component_style(&self.styles, "options-section", "panel");
        self.canvas.styled_panel(148, 302, 1090, 554, &section);
        self.canvas.styled_panel(1274, 302, 498, 554, &section);
        self.canvas.text(
            fonts,
            self.localizer.ui("options.language"),
            184,
            316,
            17.0,
            0xffffc6dc,
            true,
        );
        self.canvas.text(
            fonts,
            self.localizer.ui("options.gallery"),
            1308,
            316,
            17.0,
            0xffffc6dc,
            true,
        );

        let hovered = options_hit_test(self.mouse.0, self.mouse.1);
        for (index, language) in Language::ALL.iter().copied().enumerate() {
            let rect = options_language_rect(index);
            let selected = self.options_sel == index;
            let active = self.localizer.language() == language;
            let is_hovered = hovered == Some(index);
            let style = interactive_panel_style(
                &self.styles,
                "language-option",
                format!("language-{}", language.code()),
                selected,
                is_hovered,
                is_hovered && self.mouse_down,
            );
            self.canvas
                .styled_panel(rect.0, rect.1, rect.2, rect.3, &style);
            let code_style = component_style(&self.styles, "language-code", "panel");
            self.canvas
                .styled_panel(rect.0 + 13, rect.1 + 11, 74, rect.3 - 22, &code_style);
            let code = language.short_code();
            let code_size = 13.0;
            let code_x = rect.0 + 13 + (74 - fonts.text_width(code, code_size, true)) / 2;
            self.canvas.text(
                fonts,
                code,
                code_x,
                rect.1 + 20,
                code_size,
                0xffffd4e6,
                true,
            );
            let native_font = self
                .native_label_fonts
                .get(language.font_profile())
                .and_then(Option::as_ref);
            let label = language.display_name();
            let label_color = if active { 0xfffff0cc } else { 0xfff2dce7 };
            let label_size = if native_font
                .map(|font| font.text_width(label, 21.0))
                .unwrap_or_else(|| fonts.text_width(label, 21.0, active))
                > rect.2 - 250
            {
                18.0
            } else {
                21.0
            };
            if let Some(native_font) = native_font {
                self.canvas.native_label_text(
                    native_font,
                    label,
                    rect.0 + 105,
                    rect.1 + 17,
                    label_size,
                    label_color,
                );
            } else {
                self.canvas.text(
                    fonts,
                    label,
                    rect.0 + 105,
                    rect.1 + 17,
                    label_size,
                    label_color,
                    active,
                );
            }
            if active {
                let label = self.localizer.ui("common.current");
                let size = 11.5;
                let width = fonts.text_width(label, size, true);
                self.canvas.text(
                    fonts,
                    label,
                    rect.0 + rect.2 - width - 22,
                    rect.1 + 22,
                    size,
                    0xfff4c77e,
                    true,
                );
            }
        }

        self.canvas.paragraph(
            fonts,
            self.localizer.ui("options.language_help"),
            184,
            806,
            1014,
            24,
            15.0,
            0xffbfa4b3,
            false,
        );

        let unlocked = self.gallery.entries.len();
        let count = self.localizer.formatted(
            "options.gallery_count",
            &[
                ("{unlocked}", unlocked.to_string()),
                ("{total}", self.gallery.total.to_string()),
            ],
        );
        centered_in_rect(
            &mut self.canvas,
            fonts,
            &count,
            (1308, 382, 430, 56),
            24.0,
            0xfff4c77e,
            true,
        );
        self.canvas.paragraph(
            fonts,
            self.localizer.ui("options.gallery_help"),
            1320,
            458,
            406,
            30,
            18.0,
            0xffd6bac8,
            false,
        );
        for (index, rect, key) in [
            (
                OPTIONS_GALLERY_INDEX,
                options_gallery_rect(),
                "options.open_gallery",
            ),
            (OPTIONS_BACK_INDEX, options_back_rect(), "options.back"),
        ] {
            let is_hovered = hovered == Some(index);
            let style = interactive_panel_style(
                &self.styles,
                "options-action",
                format!("options-action-{index}"),
                self.options_sel == index,
                is_hovered,
                is_hovered && self.mouse_down,
            );
            self.canvas
                .styled_panel(rect.0, rect.1, rect.2, rect.3, &style);
            centered_in_rect(
                &mut self.canvas,
                fonts,
                self.localizer.ui(key),
                rect,
                20.0,
                0xffffedf6,
                true,
            );
        }
        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("options.controls"),
            900,
            15.0,
            0xffb99aaa,
            false,
        );
    }

    fn paint_gallery(&mut self) {
        let selected = self.gallery.entries.get(self.gallery_sel).cloned();
        let painted = selected
            .as_ref()
            .and_then(|entry| self.art.original(&entry.asset))
            .map(|art| {
                self.canvas.blit_rgba(0, 0, art);
            })
            .is_some();
        if !painted {
            if let Some(art) = self.art.original(OPTIONS_BACKGROUND_KEY) {
                self.canvas.blit_rgba(0, 0, art);
            } else {
                self.canvas.vgradient(0xff090512, 0xff24102c);
            }
        }
        if self.ui_hidden {
            return;
        }
        self.canvas.rect(0, 0, 548, WIN_H as i32, 0xe508040e);
        self.canvas.rect(548, 0, 1372, 154, 0xa6090512);
        self.canvas.rect(548, 788, 1372, 292, 0xdb090512);
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };

        let sidebar = component_style(&self.styles, "gallery-sidebar", "panel");
        self.canvas.styled_panel(34, 34, 500, 1012, &sidebar);
        self.canvas.text(
            fonts,
            self.localizer.ui("gallery.kicker"),
            68,
            72,
            13.0,
            0xffd998b7,
            true,
        );
        self.canvas.display_text(
            fonts,
            self.localizer.ui("gallery.title"),
            68,
            101,
            48.0,
            0xffffe0ed,
        );
        let unlocked_text = self.localizer.formatted(
            "options.gallery_count",
            &[
                ("{unlocked}", self.gallery.entries.len().to_string()),
                ("{total}", self.gallery.total.to_string()),
            ],
        );
        self.canvas
            .text(fonts, &unlocked_text, 68, 168, 14.0, 0xfff4c77e, true);
        self.canvas.ornament_rule(284, 222, 400, 0xffa94772);

        if self.gallery.entries.is_empty() {
            self.canvas.display_text(
                fonts,
                self.localizer.ui("gallery.empty_title"),
                626,
                400,
                39.0,
                0xffffd8e8,
            );
            self.canvas.paragraph(
                fonts,
                self.localizer.ui("gallery.empty_body"),
                630,
                470,
                1000,
                38,
                23.0,
                0xffd3b6c5,
                false,
            );
        } else {
            let page_start = (self.gallery_sel / GALLERY_PAGE_SIZE) * GALLERY_PAGE_SIZE;
            let page_end = (page_start + GALLERY_PAGE_SIZE).min(self.gallery.entries.len());
            for (slot, index) in (page_start..page_end).enumerate() {
                let entry = &self.gallery.entries[index];
                let rect = gallery_list_rect(slot);
                let hovered = rect_contains(rect, self.mouse.0, self.mouse.1);
                let style = interactive_panel_style(
                    &self.styles,
                    "gallery-item",
                    format!("gallery-{index}"),
                    index == self.gallery_sel,
                    hovered,
                    hovered && self.mouse_down,
                );
                self.canvas
                    .styled_panel(rect.0, rect.1, rect.2, rect.3, &style);
                let number = format!("{:02}", index + 1);
                self.canvas.text(
                    fonts,
                    &number,
                    rect.0 + 18,
                    rect.1 + 18,
                    15.0,
                    0xfff4c77e,
                    true,
                );
                let speaker = self
                    .localizer
                    .character_name(&entry.speaker_id, &entry.speaker);
                let room = self.localizer.room_name(&entry.room_id, &entry.room_name);
                let label = ellipsize(fonts, &format!("{speaker} · {room}"), 330, 17.0, false);
                self.canvas.text(
                    fonts,
                    &label,
                    rect.0 + 65,
                    rect.1 + 17,
                    17.0,
                    if index == self.gallery_sel {
                        0xfffff3f8
                    } else {
                        0xffdec4d1
                    },
                    false,
                );
            }

            if let Some(entry) = selected {
                let speaker = self
                    .localizer
                    .character_name(&entry.speaker_id, &entry.speaker);
                let room = self.localizer.room_name(&entry.room_id, &entry.room_name);
                let text = self.localizer.node_text(&entry.node_id, &entry.text);
                let counter = self.localizer.formatted(
                    "gallery.count",
                    &[
                        ("{current}", (self.gallery_sel + 1).to_string()),
                        ("{total}", self.gallery.entries.len().to_string()),
                    ],
                );
                self.canvas
                    .text(fonts, &counter, 598, 70, 15.0, 0xfff4c77e, true);
                self.canvas
                    .display_text(fonts, &speaker, 598, 96, 36.0, 0xffffe4ef);
                self.canvas
                    .text(fonts, &room, 600, 139, 16.0, 0xffc9a8b9, false);

                let caption = component_style(&self.styles, "gallery-caption", "panel");
                self.canvas.styled_panel(580, 810, 1280, 184, &caption);
                self.canvas.text(
                    fonts,
                    self.localizer.ui("gallery.scene"),
                    620,
                    838,
                    13.0,
                    0xffe59bbd,
                    true,
                );
                self.canvas
                    .paragraph(fonts, &text, 620, 870, 700, 31, 19.0, 0xffffedf5, false);
                let controls = self.localizer.ui("gallery.controls");
                self.canvas
                    .text(fonts, controls, 620, 955, 13.5, 0xffb99aaa, false);
            }
        }

        let back_rect = gallery_back_rect();
        let back_hovered = rect_contains(back_rect, self.mouse.0, self.mouse.1);
        let back_style = interactive_panel_style(
            &self.styles,
            "gallery-action",
            "gallery-back",
            false,
            back_hovered,
            back_hovered && self.mouse_down,
        );
        self.canvas.styled_panel(
            back_rect.0,
            back_rect.1,
            back_rect.2,
            back_rect.3,
            &back_style,
        );
        centered_in_rect(
            &mut self.canvas,
            fonts,
            self.localizer.ui("gallery.back"),
            back_rect,
            18.0,
            0xffffedf5,
            true,
        );

        if !self.gallery.entries.is_empty() {
            for (rect, key) in [
                (gallery_previous_rect(), "gallery.previous"),
                (gallery_next_rect(), "gallery.next"),
            ] {
                let hovered = rect_contains(rect, self.mouse.0, self.mouse.1);
                let style = interactive_panel_style(
                    &self.styles,
                    "gallery-action",
                    key,
                    false,
                    hovered,
                    hovered && self.mouse_down,
                );
                self.canvas
                    .styled_panel(rect.0, rect.1, rect.2, rect.3, &style);
                centered_in_rect(
                    &mut self.canvas,
                    fonts,
                    self.localizer.ui(key),
                    rect,
                    16.0,
                    0xffffedf5,
                    true,
                );
            }
        }
    }

    fn paint_play(&mut self) {
        let Some(view) = self.view.clone() else {
            return;
        };
        let art_key = background_art_key(&view.bg);
        let mut light_key = art_key.clone();
        if let Some(art) = self.art.original(&art_key) {
            self.canvas.blit_rgba(0, 0, art);
        } else {
            paint_background(&mut self.canvas, &view.bg);
        }
        if view.layers.is_empty() {
            let mut cg_keys = Vec::with_capacity(3);
            if let Some(asset) = &view.image_asset {
                cg_keys.push(asset.clone());
            }
            cg_keys.push(scene_art_key(&view.id));
            cg_keys.push(legacy_scene_art_key(&view.id));
            cg_keys.dedup();
            for key in cg_keys {
                if let Some(cg) = self.art.original(&key) {
                    self.canvas.blit_rgba(0, 0, cg);
                    light_key = key;
                    break;
                }
            }
        } else {
            // Side VNs use the HTML reference composition: the room background
            // stays visible, then left/right transparent sprites are scaled
            // into the scene before the shared dialogue panel is drawn.
            // Chiquita's poses are left out while she is alive over the scene.
            let alive = !vn_web::classic() && self.chiquita.ready();
            for layer in &view.layers {
                if alive && layer.asset.starts_with("chiquita/") {
                    continue;
                }
                if let Some(sprite) = self.art.original(&layer.asset) {
                    self.canvas.blit_rgba_scaled(
                        layer.x,
                        layer.y,
                        layer.width,
                        layer.height,
                        sprite,
                        layer.opacity,
                    );
                }
            }
        }
        if self.ui_hidden {
            return;
        }
        // Cinematic veil preserves the illustration while keeping UI legible.
        self.canvas
            .rect(0, 0, WIN_W as i32, WIN_H as i32, 0x10100818);
        let light_rgb = rgb_u32(self.ambient_light(&light_key));
        // Every scene wears the web scenes' look (`VELVET_DIALOGO_CLASICO=1`: the old panel).
        if !vn_web::classic() {
            self.paint_vn_web(&view);
            // The quick actions sit on the box: a thought's card and Chiquita's corner have none
            // (their keys still work).
            if vn_web::has_box(&view) {
                self.paint_quick_actions();
            }
            return;
        }
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };

        let tone = speaker_tone(&view.speaker);
        let accent_argb = |alpha: f32| ((alpha.clamp(0.0, 1.0) * 255.0) as u32) << 24 | tone.accent;
        let dialogue = dialogue_layout(&self.styles);
        let dialogue_style = component_style(&self.styles, "dialogue-panel", "panel");
        let (px, py, pw, ph) = (dialogue.x, dialogue.y, dialogue.width, dialogue.height);
        // One clean pane of dark glass: a soft shadow, a fine rim in the
        // speaker's colour and a hairline of light along the top.
        self.canvas
            .round_rect(px - 3, py + 10, pw + 6, ph, 26, 0x18000000, 0x2a000000);
        self.canvas
            .round_rect(px, py + 5, pw, ph, 22, 0x26000000, 0x30000000);
        // Light from the scene: a glow spilling over the top edge, glass tinted
        // by the picture's dominant colour, a sheen just under the rim.
        let lit = |alpha: f32| (((alpha.clamp(0.0, 1.0)) * 255.0) as u32) << 24 | light_rgb;
        for (grow, alpha) in [(22, 0.035f32), (13, 0.05), (6, 0.07)] {
            self.canvas.round_rect(
                px - grow,
                py - grow,
                pw + grow * 2,
                ph + grow * 2,
                20 + grow,
                lit(alpha),
                lit(alpha),
            );
        }
        for (grow, alpha) in [(64, 0.06f32), (40, 0.09), (20, 0.13)] {
            self.canvas.round_rect(
                px - grow / 2,
                py - grow,
                pw + grow,
                grow + 30,
                24 + grow / 2,
                lit(0.0),
                lit(alpha),
            );
        }
        self.canvas.round_rect(
            px,
            py,
            pw,
            ph,
            20,
            mix_argb(0xe8, mix_rgb(0x140a1e, light_rgb, 0.27)),
            mix_argb(0xf2, mix_rgb(0x090410, light_rgb, 0.14)),
        );
        self.canvas
            .vertical_scrim(px + 22, py + 2, pw - 44, 70, light_rgb, 0.17, 0.0);
        self.canvas.rect(px + 24, py + 1, pw - 48, 1, lit(0.5));
        self.canvas.round_outline(
            px,
            py,
            pw,
            ph,
            20,
            1.2,
            mix_argb(
                if tone.narrator { 0xc0 } else { 0xcc },
                mix_rgb(
                    light_rgb,
                    tone.accent,
                    if tone.narrator { 0.1 } else { 0.3 },
                ),
            ),
        );

        if !view.speaker.is_empty() {
            let speaker_size = fitted_text_size(fonts, &view.speaker, 28.0, 19.0, 480, true, true);
            let name_width = fonts.display_width(&view.speaker, speaker_size);
            let tab_width = name_width + 76;
            let (tx, ty, th) = (px + 44, py - 24, 48);
            self.canvas.round_rect(
                tx - 2,
                ty + 7,
                tab_width + 4,
                th,
                17,
                0x22000000,
                0x30000000,
            );
            self.canvas.round_rect(
                tx,
                ty,
                tab_width,
                th,
                15,
                mix_argb(
                    0xf5,
                    mix_rgb(mix_rgb(0x2c1338, light_rgb, 0.14), tone.accent, 0.18),
                ),
                mix_argb(
                    0xf5,
                    mix_rgb(mix_rgb(0x180b21, light_rgb, 0.08), tone.accent, 0.10),
                ),
            );
            self.canvas.round_outline(
                tx,
                ty,
                tab_width,
                th,
                15,
                1.4,
                accent_argb(if tone.narrator { 0.55 } else { 0.95 }),
            );
            self.canvas.smooth_disc(
                (tx + 27) as f32,
                (ty + th / 2) as f32 + 0.5,
                5.0,
                accent_argb(1.0),
            );
            self.canvas.smooth_ring(
                (tx + 27) as f32,
                (ty + th / 2) as f32 + 0.5,
                9.0,
                1.2,
                accent_argb(0.4),
            );
            let name_y = ty + (th - speaker_size.round() as i32) / 2 - 3;
            self.canvas.display_text(
                fonts,
                &view.speaker,
                tx + 46,
                name_y + 2,
                speaker_size,
                0x99120818,
            );
            self.canvas.display_text(
                fonts,
                &view.speaker,
                tx + 45,
                name_y,
                speaker_size,
                0xfffff2df,
            );
        }
        let total_characters = view.text.graphemes(true).count();
        let visible_characters = if self.reveal_complete {
            total_characters
        } else {
            (self.reveal_started.elapsed().as_secs_f32() * 72.0)
                .floor()
                .max(1.0) as usize
        }
        .min(total_characters);
        if visible_characters >= total_characters {
            self.reveal_complete = true;
        }
        let visible_text: String = view.text.graphemes(true).take(visible_characters).collect();
        let dialogue_padding_x = dialogue_style.number("content-padding-x", 52.0).round() as i32;
        let dialogue_padding_top =
            dialogue_style.number("content-padding-top", 42.0).round() as i32;
        let preferred_text_size = dialogue_style.number("font-size", 28.0);
        let preferred_line_height = dialogue_style.number("line-height", 43.0);
        let text_left = px + dialogue_padding_x;
        let dialogue_text_width = (dialogue_style
            .number("text-max-width", (pw - dialogue_padding_x * 2) as f32)
            .round() as i32)
            .min(pw - dialogue_padding_x * 2);
        let mut dialogue_text_size = preferred_text_size;
        while dialogue_text_size > 20.0
            && self
                .canvas
                .wrap(
                    fonts,
                    &view.text,
                    dialogue_text_width,
                    dialogue_text_size,
                    false,
                )
                .len()
                > 3
        {
            dialogue_text_size -= 0.5;
        }
        if self
            .canvas
            .wrap(
                fonts,
                &view.text,
                dialogue_text_width,
                dialogue_text_size,
                false,
            )
            .len()
            >= 3
        {
            dialogue_text_size = dialogue_text_size.min(26.0);
        }
        let dialogue_line_height = (preferred_line_height
            * (dialogue_text_size / preferred_text_size))
            .round()
            .max(29.0) as i32;
        let lines = self.canvas.wrap(
            fonts,
            &view.text,
            dialogue_text_width,
            dialogue_text_size,
            false,
        );
        // Short lines sit centred between the name tab and the controls.
        let zone_height = ph - 30 - 56;
        let text_top =
            (py + 30 + (zone_height - lines.len().max(1) as i32 * dialogue_line_height) / 2)
                .max(py + dialogue_padding_top - 12);
        let text_color = if tone.narrator {
            0xffeee2d3
        } else {
            computed_color_argb(&dialogue_style, "color", 0xfffff7fb)
        };
        for (dx, dy, color) in [(1, 2, 0x88080308u32), (0, 0, text_color)] {
            self.canvas.paragraph(
                fonts,
                &visible_text,
                text_left + dx,
                text_top + dy,
                dialogue_text_width,
                dialogue_line_height,
                dialogue_text_size,
                color,
                false,
            );
        }
        // Once the line is all there and Enter moves on, a mark at its end.
        if self.reveal_complete && is_inline_continue(&view) {
            if let Some(last) = lines.last() {
                let end_x = (text_left + fonts.text_width(last, dialogue_text_size, false) + 16)
                    .min(px + pw - dialogue_padding_x - 16);
                let mid_y = text_top
                    + (lines.len() as i32 - 1) * dialogue_line_height
                    + dialogue_text_size as i32 / 2
                    + 3;
                self.canvas.polygon(
                    &[
                        (end_x, mid_y - 4),
                        (end_x + 13, mid_y - 4),
                        (end_x + 6, mid_y + 6),
                    ],
                    accent_argb(0.9),
                );
            }
        }

        let rollback_available = self.game.as_ref().is_some_and(game::Game::can_rollback);
        let save_hint = if self.autosave_error.is_some() {
            self.localizer.ui("play.save_error")
        } else if rollback_available {
            self.localizer.ui("play.save_rollback")
        } else {
            self.localizer.ui("play.save_active")
        };
        let hint_color = if self.autosave_error.is_some() {
            computed_color_argb(&dialogue_style, "error-color", 0xffffa0a8)
        } else {
            0xb3c9a9bb
        };
        let footer_mid = py + ph - 33;
        self.canvas.smooth_disc(
            (px + dialogue_padding_x + 3) as f32,
            footer_mid as f32 + 0.5,
            3.0,
            hint_color,
        );
        self.canvas.text(
            fonts,
            save_hint,
            px + dialogue_padding_x + 15,
            footer_mid - 8,
            dialogue_style.number("meta-font-size", 13.0),
            hint_color,
            false,
        );

        if self.reveal_complete && !is_inline_continue(&view) {
            let choices = story_choices_layout(&self.styles, view.choices.len(), dialogue.y);
            // Dim the scene behind the decision so it reads at a glance.
            let stack_height = view.choices.len() as i32 * choices.height
                + view.choices.len().saturating_sub(1) as i32 * choices.gap;
            let scrim_top = choices.y - 150;
            let flat_end = choices.y + stack_height + 24;
            self.canvas
                .vertical_scrim(0, scrim_top, WIN_W as i32, 120, 0x0a0410, 0.0, 0.5);
            self.canvas.rect(
                0,
                scrim_top + 120,
                WIN_W as i32,
                flat_end - (scrim_top + 120),
                0x800a0410,
            );
            self.canvas
                .vertical_scrim(0, flat_end, WIN_W as i32, 80, 0x0a0410, 0.5, 0.0);
            // Heading on one line: the title, a hairline, the keys.
            let title = self.localizer.ui("play.choose");
            let hint = self.localizer.ui("play.choose_hint");
            let title_size = 16.0;
            let title_width = fonts.text_width(title, title_size, true);
            let hint_size = 13.0;
            let hint_width = fonts.text_width(hint, hint_size, false);
            let heading_y = choices.y - 36;
            self.canvas
                .diamond(choices.x + 8, heading_y + 10, 5, 0xffffc78f);
            self.canvas.text(
                fonts,
                title,
                choices.x + 24,
                heading_y,
                title_size,
                0xffffe0bd,
                true,
            );
            let line_from = choices.x + 24 + title_width + 18;
            let line_to = choices.x + choices.width - hint_width - 18;
            self.canvas.rect(
                line_from,
                heading_y + 10,
                (line_to - line_from).max(0),
                1,
                0x3cffffff,
            );
            self.canvas.text(
                fonts,
                hint,
                choices.x + choices.width - hint_width,
                heading_y + 2,
                hint_size,
                0xc0d6b8c8,
                false,
            );
            let hovered_choice = (0..view.choices.len()).find(|&index| {
                view.choices[index].enabled && choices.contains(index, self.mouse.0, self.mouse.1)
            });
            self.choice_hover_animating = advance_native_button_hover(
                &mut self.choice_hover_progress,
                &mut self.choice_hover_last_tick,
                hovered_choice,
                self.choice_sel,
                view.choices.len(),
                self.reduced_motion,
            );
            for (index, choice) in view.choices.iter().enumerate() {
                let hovered = hovered_choice == Some(index);
                self.canvas.story_choice_card(
                    fonts,
                    choices.x,
                    choices.item_y(index),
                    choices.width,
                    choices.height,
                    &choice.label,
                    index + 1,
                    self.choice_sel == index,
                    self.choice_hover_progress
                        .get(index)
                        .copied()
                        .unwrap_or_default(),
                    hovered && self.mouse_down,
                    choice.enabled,
                    light_rgb,
                );
            }
        }

        if view.show_hud {
            paint_hud(
                &mut self.canvas,
                fonts,
                self.game.as_mut(),
                &self.styles,
                &self.localizer,
            );
        }
        self.paint_quick_actions();
    }

    fn paint_end(&mut self) {
        let has_end_background = if let Some(art) = self.art.original(END_BACKGROUND_KEY) {
            self.canvas.blit_rgba(0, 0, art);
            true
        } else {
            self.canvas.vgradient(0xff090512, 0xff1c0d26);
            false
        };
        if self.ui_hidden {
            return;
        }
        if has_end_background {
            self.canvas
                .rect(0, 0, WIN_W as i32, WIN_H as i32, 0xb8090512);
        }
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let panel = MenuLayout::resolve(
            &self.styles,
            "prologue-end-panel",
            MenuLayout {
                x: 300,
                y: 210,
                width: 1320,
                height: 690,
                gap: 0,
            },
        );
        let panel_style = component_style(&self.styles, "prologue-end-panel", "panel");
        self.canvas
            .styled_panel(panel.x, panel.y, panel.width, panel.height, &panel_style);

        let title_style = component_style(&self.styles, "prologue-end-title", "panel");
        let title_x = panel.x + panel_style.number("title-offset-x", 190.0).round() as i32;
        let title_y = panel.y + panel_style.number("title-offset-y", -38.0).round() as i32;
        let title_width = panel_style.number("title-width", 760.0).round() as i32;
        let title_height = panel_style.number("title-height", 88.0).round() as i32;
        self.canvas
            .styled_panel(title_x, title_y, title_width, title_height, &title_style);
        let title = self.localizer.ui("end.title");
        let title_size = fitted_text_size(
            fonts,
            title,
            title_style.number("font-size", 38.0),
            23.0,
            title_width - 80,
            true,
            true,
        );
        let title_text_x = title_x + (title_width - fonts.display_width(title, title_size)) / 2;
        let title_text_y = title_y + (title_height - title_size.round() as i32) / 2 - 2;
        self.canvas.display_text(
            fonts,
            title,
            title_text_x + 2,
            title_text_y + 3,
            title_size,
            computed_color_argb(&title_style, "text-shadow-color", 0xcc160c1f),
        );
        self.canvas.display_text(
            fonts,
            title,
            title_text_x,
            title_text_y,
            title_size,
            computed_color_argb(&title_style, "color", 0xfffff0dc),
        );

        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("end.kicker"),
            panel.y + 72,
            13.5,
            computed_color_argb(&panel_style, "secondary-color", 0xffd8b8c8),
            true,
        );
        let chapter = self.localizer.ui("end.chapter");
        let chapter_size =
            fitted_text_size(fonts, chapter, 25.0, 17.0, panel.width - 180, true, false);
        let chapter_x = panel.x + (panel.width - fonts.text_width(chapter, chapter_size, true)) / 2;
        self.canvas.text(
            fonts,
            chapter,
            chapter_x,
            panel.y + 106,
            chapter_size,
            computed_color_argb(&panel_style, "accent-color", 0xfff4c77e),
            true,
        );
        let summary = self.localizer.ui("end.summary");
        let summary_size =
            fitted_text_size(fonts, summary, 18.0, 14.0, panel.width - 180, false, false);
        let summary_x =
            panel.x + (panel.width - fonts.text_width(summary, summary_size, false)) / 2;
        self.canvas.text(
            fonts,
            summary,
            summary_x,
            panel.y + 153,
            summary_size,
            computed_color_argb(&panel_style, "secondary-color", 0xffd8b8c8),
            false,
        );

        let stats = self.game.as_mut().and_then(|g| g.stats().ok());
        if let Some(stats) = stats {
            let row_style = component_style(&self.styles, "stats-row", "panel");
            let cards = [
                (
                    self.localizer.ui("stat.fear"),
                    stats.fear,
                    computed_color_argb(&panel_style, "fear-color", 0xffff789f),
                ),
                (
                    self.localizer.ui("stat.devotion"),
                    stats.devotion,
                    computed_color_argb(&panel_style, "devotion-color", 0xffc592ff),
                ),
                (
                    self.localizer.ui("stat.dignity"),
                    stats.dignity,
                    computed_color_argb(&panel_style, "dignity-color", 0xfff2bf77),
                ),
                (
                    self.localizer.ui("stat.hunger"),
                    stats.hunger,
                    computed_color_argb(&panel_style, "hunger-color", 0xffff8ac6),
                ),
            ];
            let card_gap = 22;
            let card_width = (panel.width - 180 - card_gap) / 2;
            let card_height = 72;
            for (index, (label, value, accent)) in cards.iter().enumerate() {
                let column = index % 2;
                let row = index / 2;
                let x = panel.x + 90 + column as i32 * (card_width + card_gap);
                let y = panel.y + 225 + row as i32 * 96;
                self.canvas
                    .styled_panel(x, y, card_width, card_height, &row_style);
                self.canvas
                    .rect(x + 13, y + 14, 4, card_height - 28, *accent);
                self.canvas.text(
                    fonts,
                    label,
                    x + 31,
                    y + 25,
                    15.5,
                    computed_color_argb(&row_style, "color", 0xfff5e7ee),
                    true,
                );
                let value_text = value.to_string();
                let value_size = 27.0;
                let value_width = fonts.text_width(&value_text, value_size, true);
                self.canvas.text(
                    fonts,
                    &value_text,
                    x + card_width - value_width - 24,
                    y + 18,
                    value_size,
                    *accent,
                    true,
                );
            }
        }

        let memory_y = panel.y + 475;
        self.canvas.ornament_rule(
            panel.x + panel.width / 2,
            memory_y,
            panel.width - 460,
            computed_color_argb(&panel_style, "border-color", 0xffb74f7d),
        );
        centered_text(
            &mut self.canvas,
            fonts,
            self.localizer.ui("gallery.kicker"),
            memory_y + 22,
            12.5,
            computed_color_argb(&panel_style, "secondary-color", 0xffd8b8c8),
            true,
        );
        let memory_count = self.localizer.formatted(
            "options.gallery_count",
            &[
                ("{unlocked}", self.gallery.entries.len().to_string()),
                ("{total}", self.gallery.total.to_string()),
            ],
        );
        centered_text(
            &mut self.canvas,
            fonts,
            &memory_count,
            memory_y + 49,
            19.0,
            computed_color_argb(&panel_style, "accent-color", 0xfff4c77e),
            true,
        );

        for (rect, key) in [
            (end_gallery_rect(), "end.gallery"),
            (end_title_rect(), "end.title_menu"),
        ] {
            let hovered = rect_contains(rect, self.mouse.0, self.mouse.1);
            let style = interactive_panel_style(
                &self.styles,
                "end-action",
                key,
                false,
                hovered,
                hovered && self.mouse_down,
            );
            self.canvas
                .styled_panel(rect.0, rect.1, rect.2, rect.3, &style);
            centered_in_rect(
                &mut self.canvas,
                fonts,
                self.localizer.ui(key),
                rect,
                19.0,
                0xffffedf5,
                true,
            );
        }

        let hint = self.localizer.ui("end.hint");
        let hint_size = fitted_text_size(fonts, hint, 14.5, 11.0, panel.width - 160, true, false);
        let hint_x = panel.x + (panel.width - fonts.text_width(hint, hint_size, true)) / 2;
        self.canvas.text(
            fonts,
            hint,
            hint_x,
            panel.y + panel.height - 34,
            hint_size,
            computed_color_argb(&panel_style, "secondary-color", 0xffd8b8c8),
            true,
        );
    }

    /// A key in the host's own screens: what it does, then the menus' sound for it (a tick
    /// when a selection moves, a chime when a screen changes; the pause menu and the world
    /// sound on their own).
    fn handle_key(&mut self, key: KeyCode) {
        let before = self.menu_marks();
        self.handle_screen_key(key);
        let after = self.menu_marks();
        let arrows = matches!(
            key,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
        );
        self.menu_sound(before, after, arrows, key == KeyCode::Escape);
    }

    /// Where the host's menus stand: the screen and each list's selection.
    fn menu_marks(&self) -> (Screen, usize, usize, usize, usize) {
        (
            self.screen,
            self.menu_sel,
            self.choice_sel,
            self.options_sel,
            self.gallery_sel,
        )
    }

    fn menu_sound(
        &self,
        before: (Screen, usize, usize, usize, usize),
        after: (Screen, usize, usize, usize, usize),
        moved_by_arrows: bool,
        back: bool,
    ) {
        use exploration3d::ui_sfx::{self, UiSfx};
        let own = |screen: Screen| matches!(screen, Screen::GameMenu | Screen::Explore);
        if before == after || own(before.0) || own(after.0) {
            return;
        }
        if before.0 != after.0 {
            ui_sfx::play(if back { UiSfx::Back } else { UiSfx::Select });
        } else if moved_by_arrows {
            ui_sfx::play(UiSfx::Move);
        }
    }

    fn handle_screen_key(&mut self, key: KeyCode) {
        if self.quick_action_key(key) {
            return;
        }
        if self.skipping_dialogue
            && matches!(key, KeyCode::Enter | KeyCode::Space | KeyCode::PageDown)
        {
            self.stop_dialogue_skip();
            return;
        }
        if key == KeyCode::KeyR && self.screen == Screen::Title {
            self.request_restart();
            return;
        }
        match self.screen {
            Screen::GameMenu => self.menu_key(key),
            Screen::Title => {
                // The title in Persona's manner (`title_menu.rs`).
                self.title_key(key);
            }
            Screen::Resume => match key {
                KeyCode::ArrowUp => {
                    self.choice_sel =
                        next_resume_choice(self.choice_sel, -1, self.continue_available);
                    self.needs_paint = true;
                }
                KeyCode::ArrowDown => {
                    self.choice_sel =
                        next_resume_choice(self.choice_sel, 1, self.continue_available);
                    self.needs_paint = true;
                }
                KeyCode::Enter | KeyCode::Space => self.activate_resume_choice(),
                _ => {}
            },
            Screen::Options => match key {
                KeyCode::ArrowUp => {
                    self.options_sel = (self.options_sel as i32 - 1)
                        .rem_euclid(OPTIONS_ITEM_COUNT as i32)
                        as usize;
                    self.needs_paint = true;
                }
                KeyCode::ArrowDown => {
                    self.options_sel = (self.options_sel + 1) % OPTIONS_ITEM_COUNT;
                    self.needs_paint = true;
                }
                KeyCode::ArrowLeft if self.options_sel < Language::ALL.len() => {
                    self.options_sel = self.options_sel.saturating_sub(6);
                    self.needs_paint = true;
                }
                KeyCode::ArrowRight if self.options_sel < 6 => {
                    let candidate = self.options_sel + 6;
                    if candidate < Language::ALL.len() {
                        self.options_sel = candidate;
                        self.needs_paint = true;
                    }
                }
                KeyCode::Enter | KeyCode::Space => {
                    if self.options_sel < Language::ALL.len() {
                        self.select_language(self.options_sel);
                    } else if self.options_sel == OPTIONS_GALLERY_INDEX {
                        self.open_gallery(Screen::Options);
                    } else {
                        self.screen = self.options_return;
                        self.needs_paint = true;
                    }
                }
                KeyCode::Escape => {
                    self.screen = self.options_return;
                    self.needs_paint = true;
                }
                _ => {}
            },
            Screen::Gallery => match key {
                KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::PageUp => {
                    if !self.gallery.entries.is_empty() {
                        self.gallery_sel = (self.gallery_sel as i32 - 1)
                            .rem_euclid(self.gallery.entries.len() as i32)
                            as usize;
                        self.needs_paint = true;
                    }
                }
                KeyCode::ArrowRight | KeyCode::ArrowDown | KeyCode::PageDown => {
                    if !self.gallery.entries.is_empty() {
                        self.gallery_sel = (self.gallery_sel + 1) % self.gallery.entries.len();
                        self.needs_paint = true;
                    }
                }
                KeyCode::Escape => self.leave_gallery(),
                _ => {}
            },
            Screen::Play => {
                match key {
                    KeyCode::PageUp => {
                        self.rollback_scene();
                        return;
                    }
                    KeyCode::PageDown => {
                        self.advance_play();
                        return;
                    }
                    KeyCode::F5 => {
                        self.open_pause(pause_menu::Page::Save);
                        return;
                    }
                    KeyCode::F9 => {
                        self.open_pause(pause_menu::Page::Load);
                        return;
                    }
                    _ => {}
                }
                if !self.reveal_complete {
                    self.reveal_complete = true;
                    self.needs_paint = true;
                    return;
                }
                let count = self.view.as_ref().map(|v| v.choices.len()).unwrap_or(0);
                if count == 0 {
                    return;
                }
                match key {
                    KeyCode::ArrowUp => {
                        if let Some(view) = &self.view {
                            self.choice_sel =
                                next_enabled_choice(&view.choices, self.choice_sel, -1);
                        }
                        self.needs_paint = true;
                    }
                    KeyCode::ArrowDown => {
                        if let Some(view) = &self.view {
                            self.choice_sel =
                                next_enabled_choice(&view.choices, self.choice_sel, 1);
                        }
                        self.needs_paint = true;
                    }
                    KeyCode::Enter | KeyCode::Space => self.confirm_choice(),
                    _ => {}
                }
            }
            Screen::Explore => match key {
                KeyCode::F9 => {
                    if let Err(error) = self.continue_game() {
                        eprintln!("no se pudo cargar: {error:#}");
                    }
                }
                KeyCode::Escape => self.open_pause(pause_menu::Page::Status),
                _ => {}
            },
            Screen::End => match key {
                KeyCode::Enter | KeyCode::Space => self.open_gallery(Screen::End),
                KeyCode::Escape => {
                    self.screen = Screen::Title;
                    self.needs_paint = true;
                }
                _ => {}
            },
        }
    }

    fn click_at(&mut self, x: i32, y: i32) {
        let before = self.menu_marks();
        self.click_screen_at(x, y);
        let after = self.menu_marks();
        // A click in the story moves on (or takes a choice, which chimes itself): no tick.
        self.menu_sound(before, after, before.0 != Screen::Play, false);
    }

    fn click_screen_at(&mut self, x: i32, y: i32) {
        if self.quick_action_click(x, y) {
            return;
        }
        if self.skipping_dialogue {
            self.stop_dialogue_skip();
            return;
        }
        if matches!(self.screen, Screen::Explore | Screen::End | Screen::Resume)
            && rect_contains(pause_menu::TRIGGER, x, y)
        {
            let page = if self.screen == Screen::Resume {
                pause_menu::Page::Load
            } else {
                pause_menu::Page::Status
            };
            self.open_pause(page);
            return;
        }
        match self.screen {
            Screen::GameMenu => self.menu_click(x, y),
            Screen::Title => self.title_click(x, y),
            Screen::Resume => {
                let layout = resume_choices_layout(&self.styles);
                for hit in 0..3 {
                    if resume_choice_enabled(hit, self.continue_available)
                        && layout.contains(hit, x, y)
                    {
                        self.choice_sel = hit;
                        self.activate_resume_choice();
                        break;
                    }
                }
            }
            Screen::Options => {
                let Some(hit) = options_hit_test(x, y) else {
                    return;
                };
                self.options_sel = hit;
                if hit < Language::ALL.len() {
                    self.select_language(hit);
                } else if hit == OPTIONS_GALLERY_INDEX {
                    self.open_gallery(Screen::Options);
                } else {
                    self.screen = self.options_return;
                    self.needs_paint = true;
                }
            }
            Screen::Gallery => {
                if rect_contains(gallery_back_rect(), x, y) {
                    self.leave_gallery();
                    return;
                }
                if !self.gallery.entries.is_empty() {
                    if rect_contains(gallery_previous_rect(), x, y) {
                        self.gallery_sel = (self.gallery_sel as i32 - 1)
                            .rem_euclid(self.gallery.entries.len() as i32)
                            as usize;
                        self.needs_paint = true;
                        return;
                    }
                    if rect_contains(gallery_next_rect(), x, y) {
                        self.gallery_sel = (self.gallery_sel + 1) % self.gallery.entries.len();
                        self.needs_paint = true;
                        return;
                    }
                    let page_start = (self.gallery_sel / GALLERY_PAGE_SIZE) * GALLERY_PAGE_SIZE;
                    let page_end = (page_start + GALLERY_PAGE_SIZE).min(self.gallery.entries.len());
                    for (slot, index) in (page_start..page_end).enumerate() {
                        if rect_contains(gallery_list_rect(slot), x, y) {
                            self.gallery_sel = index;
                            self.needs_paint = true;
                            return;
                        }
                    }
                }
            }
            Screen::Play => {
                if !self.reveal_complete {
                    self.reveal_complete = true;
                    self.needs_paint = true;
                    return;
                }
                let count = self.view.as_ref().map(|v| v.choices.len()).unwrap_or(0);
                if count == 0 {
                    return;
                }
                let dialogue = dialogue_layout(&self.styles);
                let inline_continue = self.view.as_ref().is_some_and(is_inline_continue);
                if inline_continue {
                    if dialogue_contains(dialogue, x, y) {
                        self.choice_sel = 0;
                        self.confirm_choice();
                    }
                    return;
                }
                let choices = story_choices_layout(&self.styles, count, dialogue.y);
                for hit in 0..count {
                    let enabled = self
                        .view
                        .as_ref()
                        .and_then(|view| view.choices.get(hit))
                        .is_some_and(|choice| choice.enabled);
                    if enabled && choices.contains(hit, x, y) {
                        self.choice_sel = hit;
                        self.confirm_choice();
                        break;
                    }
                }
            }
            Screen::Explore => {}
            Screen::End => {
                if rect_contains(end_gallery_rect(), x, y) {
                    self.open_gallery(Screen::End);
                } else if rect_contains(end_title_rect(), x, y) {
                    self.screen = Screen::Title;
                    self.needs_paint = true;
                }
            }
        }
    }
}

fn ignore_repeated_key(key: PhysicalKey, repeat: bool) -> bool {
    repeat
        && !matches!(
            key,
            PhysicalKey::Code(KeyCode::ArrowUp | KeyCode::ArrowDown)
        )
}

fn paint_background(canvas: &mut Canvas, bg: &str) {
    match bg {
        "earth_buffet" => {
            canvas.vgradient(0xff3a2418, 0xff17100b);
            canvas.rect(0, 780, WIN_W as i32, 300, 0xff241a12);
            canvas.rect(0, 774, WIN_W as i32, 6, 0xff51382a);
            for step in 0..8 {
                canvas.rect(120 + step * 225, 300, 135, 450, 0xff4a3324);
            }
        }
        "void" => {
            canvas.vgradient(0xff070310, 0xff150a22);
            canvas.circle(360, 270, 5, 0x88b48ad0);
            canvas.circle(1080, 480, 3, 0x66b48ad0);
            canvas.circle(1500, 210, 6, 0x55b48ad0);
        }
        "castle_bedroom" => {
            canvas.vgradient(0xff1c1024, 0xff0d0714);
            canvas.rect(1320, 270, 420, 570, 0xff2c1a38); // bed
            canvas.rect(1320, 240, 420, 45, 0xff4a2c50);
            canvas.rect(210, 210, 330, 450, 0xff20122c); // mirror
            canvas.circle(375, 435, 135, 0xff31203f);
        }
        "throne_room" => {
            canvas.vgradient(0xff260a14, 0xff12050c);
            for pillar in 0..5 {
                canvas.rect(135 + pillar * 405, 120, 81, 750, 0xff3d1224);
                canvas.rect(135 + pillar * 405, 120, 81, 15, 0xff6b2140);
            }
            canvas.rect(645, 645, 630, 225, 0xff571c33); // throne base
            canvas.rect(765, 525, 390, 135, 0xff6b2140); // backrest
            canvas.rect(750, 660, 60, 90, 0xff7d2a4c); // reposabrazos L
            canvas.rect(1125, 660, 60, 90, 0xff7d2a4c); // reposabrazos R
        }
        "castle_balcony" => {
            canvas.vgradient(0xff0a0d26, 0xff1d1236);
            canvas.circle(1470, 180, 87, 0xffe8dff2);
            canvas.circle(1680, 300, 51, 0xffb9a6d8);
            for star_x in [210, 480, 750, 1050, 1290] {
                canvas.circle(star_x, 135 + (star_x % 90), 3, 0xffcfc0ea);
            }
            canvas.rect(0, 840, WIN_W as i32, 240, 0xff141021);
        }
        _ => canvas.clear(0xff120a16),
    }
}

fn paint_hud(
    canvas: &mut Canvas,
    fonts: &Fonts,
    game: Option<&mut game::Game>,
    styles: &Stylesheet,
    localizer: &Localizer,
) {
    let Some(game) = game else { return };
    let Ok(stats) = game.stats() else { return };
    let layout = MenuLayout::resolve(
        styles,
        "stats-hud",
        MenuLayout {
            x: 28,
            y: 38,
            width: 438,
            height: 220,
            gap: 0,
        },
    );
    let style = component_style(styles, "stats-hud", "panel");
    canvas.styled_panel(layout.x, layout.y, layout.width, layout.height, &style);

    let title_style = component_style(styles, "stats-hud-title", "panel");
    let title_x = layout.x + style.number("title-offset-x", 28.0).round() as i32;
    let title_y = layout.y + style.number("title-offset-y", -18.0).round() as i32;
    let title_width = style.number("title-width", 278.0).round() as i32;
    let title_height = style.number("title-height", 46.0).round() as i32;
    canvas.styled_panel(title_x, title_y, title_width, title_height, &title_style);
    let title = localizer.ui("hud.title");
    let title_size = title_style.number("font-size", 18.0);
    let title_text_x = title_x + (title_width - fonts.display_width(title, title_size)) / 2;
    let title_text_y = title_y + (title_height - title_size.round() as i32) / 2 - 1;
    canvas.display_text(
        fonts,
        title,
        title_text_x + 1,
        title_text_y + 2,
        title_size,
        computed_color_argb(&title_style, "text-shadow-color", 0xcc160c1f),
    );
    canvas.display_text(
        fonts,
        title,
        title_text_x,
        title_text_y,
        title_size,
        computed_color_argb(&title_style, "color", 0xfffff0dc),
    );

    let row_style = component_style(styles, "stats-row", "panel");
    let track_style = component_style(styles, "stats-track", "progress-bar");
    let rows = [
        (
            localizer.ui("stat.fear"),
            stats.fear,
            computed_color_argb(&style, "fear-color", 0xffff789f),
        ),
        (
            localizer.ui("stat.devotion"),
            stats.devotion,
            computed_color_argb(&style, "devotion-color", 0xffc592ff),
        ),
        (
            localizer.ui("stat.dignity"),
            stats.dignity,
            computed_color_argb(&style, "dignity-color", 0xfff2bf77),
        ),
        (
            localizer.ui("stat.hunger"),
            stats.hunger,
            computed_color_argb(&style, "hunger-color", 0xffff8ac6),
        ),
    ];
    let content_padding_x = style.number("content-padding-x", 18.0).round() as i32;
    let content_y = layout.y + style.number("content-padding-top", 48.0).round() as i32;
    let row_height = style.number("row-height", 34.0).round() as i32;
    let row_gap = style.number("row-gap", 8.0).round() as i32;
    let row_x = layout.x + content_padding_x;
    let row_width = layout.width - content_padding_x * 2;
    let label_width = style.number("label-width", 104.0).round() as i32;
    let value_width = style.number("value-width", 42.0).round() as i32;
    let track_height = style.number("track-height", 10.0).round() as i32;
    let label_size = row_style.number("font-size", 16.5);
    let value_size = row_style.number("value-font-size", 16.0);
    let track_border = computed_color_argb(&track_style, "border-color", 0xd8613049);
    let track_background = computed_color_argb(&track_style, "background", 0xe8120812);
    let track_highlight = computed_color_argb(&track_style, "highlight-color", 0x70ffffff);

    for (index, (label, value, accent)) in rows.iter().enumerate() {
        let row_y = content_y + index as i32 * (row_height + row_gap);
        canvas.styled_panel(row_x, row_y, row_width, row_height, &row_style);
        canvas.rect(row_x + 9, row_y + 9, 3, row_height - 18, *accent);
        canvas.text(
            fonts,
            label,
            row_x + 20,
            row_y + 7,
            label_size,
            computed_color_argb(&row_style, "color", 0xfff5e7ee),
            false,
        );

        let track_x = row_x + label_width + 18;
        let track_y = row_y + (row_height - track_height) / 2;
        let track_width = (row_width - label_width - value_width - 42).max(40);
        canvas.rect(track_x, track_y, track_width, track_height, track_border);
        canvas.rect(
            track_x + 1,
            track_y + 1,
            track_width - 2,
            track_height - 2,
            track_background,
        );
        let fill_max = (track_width - 4).max(0);
        let fill_width = ((*value).clamp(0, 100) as f32 / 100.0 * fill_max as f32).round() as i32;
        if fill_width > 0 {
            canvas.rect(
                track_x + 2,
                track_y + 2,
                fill_width,
                track_height - 4,
                *accent,
            );
            canvas.rect(track_x + 2, track_y + 2, fill_width, 1, track_highlight);
        }

        let value_text = value.to_string();
        let value_text_width = fonts.text_width(&value_text, value_size, true);
        canvas.text(
            fonts,
            &value_text,
            row_x + row_width - 16 - value_text_width,
            row_y + 6,
            value_size,
            computed_color_argb(&row_style, "secondary-color", 0xfffff4f8),
            true,
        );
    }
}

/// The world the prologue ends in, loading in the background while the prologue plays.
struct WorldPreload {
    world: String,
    spawn: String,
    /// The dungeon seed it is loading for (the world is made with it when it is entered).
    seed: u64,
    job: std::thread::JoinHandle<Result<exploration3d::RenderState>>,
}

struct WinitApp {
    app: App,
    world: Option<exploration3d::App>,
    loading: Option<std::thread::JoinHandle<Result<exploration3d::RenderState>>>,
    preload: Option<WorldPreload>,
    transition: Option<scene_transition::SceneTransition>,
    world_ready: bool,
    world_paused: bool,
    menu_button_signature: Option<(Language, bool)>,
    close_after_load: bool,
    next_world_checkpoint: Instant,
    smoke: Option<unified_smoke::SmokeTest>,
    /// `--check-chiquita <node>`: the visual novel at that node in the real window for a few
    /// seconds, counting the frames Chiquita alive was laid over (then it quits).
    chiquita_check: Option<ChiquitaCheck>,
    /// The pixel-art minigame shown while a world loads, and its pacing.
    loading_screen: Option<loading::Loader>,
    loading_last_tick: Instant,
    loading_last_paint: Instant,
    loading_leave_at: Option<Instant>,
    /// Which scene the next loading screen opens on (they take turns).
    loading_next_scene: usize,
}

/// A real-window check of Chiquita alive over the visual novel (`--check-chiquita <node>`).
struct ChiquitaCheck {
    node: String,
    /// When the node came up, and the frames presented by then.
    started: Option<(Instant, u64)>,
    shown: u32,
}

impl WinitApp {
    /// The check's step: open the node once the window shows, count, report and quit.
    fn tick_chiquita_check(&mut self, event_loop: &ActiveEventLoop) {
        let Some(check) = self.chiquita_check.as_mut() else {
            return;
        };
        let presented = self.app.display.as_ref().map_or(0, |d| d.presented_frames);
        match check.started {
            None if presented > 0 => {
                let entered = game::Game::load().and_then(|mut game| {
                    game.new_game()?;
                    game.jump_to_node(&check.node)?;
                    Ok(game)
                });
                match entered.and_then(|game| self.app.enter_game(game)) {
                    Ok(()) => {
                        self.app.needs_paint = true;
                        check.started = Some((Instant::now(), presented));
                    }
                    Err(error) => {
                        eprintln!("[check] {}: {error:#}", check.node);
                        event_loop.exit();
                    }
                }
            }
            Some((at, from)) => {
                if self.app.chiquita_shown {
                    check.shown += 1;
                }
                if at.elapsed() > Duration::from_secs(4) {
                    println!(
                        "[check] {}: {} frames in 4 s, Chiquita over {} of the loop's turns, shown at the end: {}",
                        check.node,
                        presented - from,
                        check.shown,
                        self.app.chiquita_shown
                    );
                    event_loop.exit();
                    self.chiquita_check = None;
                }
            }
            None => {}
        }
    }
}

/// Loading art is drawn at a steady 30 fps, like the pixel art it is.
const LOADING_FRAME: Duration = Duration::from_millis(33);
/// The loading screen fades out over this long before the world is revealed.
const LOADING_LEAVE: Duration = Duration::from_millis(260);

impl WinitApp {
    fn new(app: App) -> Self {
        Self {
            app,
            world: None,
            loading: None,
            preload: None,
            transition: None,
            world_ready: false,
            world_paused: false,
            menu_button_signature: None,
            close_after_load: false,
            next_world_checkpoint: Instant::now(),
            smoke: None,
            chiquita_check: None,
            loading_screen: None,
            loading_last_tick: Instant::now(),
            loading_last_paint: Instant::now(),
            loading_leave_at: None,
            loading_next_scene: 0,
        }
    }

    /// While the prologue's visual novel plays, the castle it ends in loads in the background (the
    /// same `prepare` the loading screen waits on, for a dungeon seed chosen now), so the world
    /// opens without a wait when the prologue ends. Going back to the title lets it go.
    fn preload_world(&mut self) {
        if self.app.screen == Screen::Title {
            self.preload = None;
            return;
        }
        let in_prologue = self.app.screen == Screen::Play
            && self
                .app
                .view
                .as_ref()
                .is_some_and(|view| view.id.starts_with(game::PROLOGUE_NODE_PREFIX));
        if !in_prologue || self.preload.is_some() || self.world.is_some() || self.loading.is_some()
        {
            return;
        }
        let Some(display) = &self.app.display else {
            return;
        };
        let gpu = display.gpu.clone();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64);
        let (world, spawn) = game::GUION_WORLD_ENTRY;
        self.preload = Some(WorldPreload {
            world: world.to_owned(),
            spawn: spawn.to_owned(),
            seed,
            job: std::thread::spawn(move || exploration3d::App::prepare(gpu, seed)),
        });
        eprintln!("[game] el castillo se carga mientras se juega el prólogo");
    }

    fn begin_world(&mut self, entry: game::WorldEntry) {
        let Some(display) = &self.app.display else {
            return;
        };
        let gpu = display.gpu.clone();
        // The castle the prologue has been loading, when this is its way in.
        let preload = self
            .preload
            .take()
            .filter(|p| p.world == entry.world && p.spawn == entry.spawn);
        self.world = Some(exploration3d::App::for_entry(
            entry.world,
            entry.spawn,
            exploration3d_state::path(),
            self.app.window.as_ref().expect("host window").clone(),
            preload.as_ref().map(|p| p.seed),
        ));
        self.world_ready = false;
        self.loading_screen = None;
        self.loading_leave_at = None;
        self.transition = Some(scene_transition::SceneTransition::new(
            Instant::now(),
            self.app.reduced_motion,
        ));
        let dungeon_seed = self
            .world
            .as_ref()
            .expect("world entry was initialized before loading")
            .dungeon_seed();
        self.loading = Some(match preload {
            // (A saved dungeon of this world came back instead of the fresh one: that loads as usual.)
            Some(preload) if preload.seed == dungeon_seed => {
                eprintln!("[game] el castillo ya venía cargándose desde el prólogo");
                preload.job
            }
            _ => std::thread::spawn(move || exploration3d::App::prepare(gpu, dungeon_seed)),
        });
        self.app.mouse_down = false;
        self.app.needs_paint = false;
        eprintln!(
            "[game] VN → 3D in-process, same window {:?}",
            self.app.window.as_ref().map(|w| w.id())
        );
    }

    /// Leaves the 3D world for the visual novel at `node` (carrying the story's variables over).
    fn begin_world_vn(
        &mut self,
        node: &str,
        values: Option<std::collections::BTreeMap<String, i32>>,
    ) {
        if self.loading.is_some() || self.transition.is_some() || !self.world_ready {
            return;
        }
        self.checkpoint_world(true);
        if let Some(world) = &mut self.world {
            match world.capture_pixels() {
                Ok(pixels) => {
                    self.app.canvas.pixels = pixels
                        .chunks_exact(4)
                        .map(|p| {
                            0xff000000 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32
                        })
                        .collect();
                }
                Err(error) => {
                    eprintln!("[vn] captura 3D antes de la conversación: {error}");
                    self.app.canvas.clear(0xff120a16);
                }
            }
        }
        // The snapshot was checkpointed above. Drop the GPU scene before the
        // VN starts so repeated interactions do not retain old world buffers.
        self.release_world();
        self.world_ready = false;
        self.world_paused = false;
        self.transition = None;
        self.app.pause = None;
        if let (Some(values), Some(game)) = (values.as_ref(), self.app.game.as_mut()) {
            game.set_story_variables(values);
        }
        if let Err(error) = self.app.open_world_vn(node) {
            eprintln!("[vn] no se pudo abrir la escena {node}: {error:#}");
            self.app.autosave_error = Some(format!("VN: {error:#}"));
            self.app.screen = Screen::Title;
        }
        self.app.needs_paint = true;
        eprintln!("[game] 3D → VN: {node}");
    }

    fn release_world(&mut self) {
        // WGPU destruction is deferred until the device is polled. Explicitly
        // drop the scene before replacing it and drain completed submissions;
        // repeated VN↔3D loads must not retain every old world allocation.
        self.world.take();
        if let Some(display) = &self.app.display {
            let _ = display.gpu.device.poll(wgpu::Maintain::Wait);
        }
    }

    fn poll_world(&mut self) {
        if self.loading.as_ref().is_some_and(|job| job.is_finished()) {
            let result = self
                .loading
                .take()
                .expect("finished job")
                .join()
                .unwrap_or_else(|_| Err(anyhow!("3D loader failed")));
            match result {
                Ok(state) => {
                    if let (Some(world), Some(window)) = (&mut self.world, &self.app.window) {
                        world.attach(window.clone(), state);
                        self.world_ready = true;
                        self.checkpoint_world(true);
                    }
                }
                Err(error) => {
                    eprintln!("[game] no se pudo cargar el mapa: {error:#}");
                    self.app.autosave_error = Some(format!("3D: {error:#}. Retry with Continue."));
                    self.release_world();
                    self.app.art.clear();
                    self.transition = None;
                    self.app.screen = Screen::Title;
                    self.app.needs_paint = true;
                }
            }
        }
        let loading_done = self.loading_finished(Instant::now());
        if let Some(fade) = &mut self.transition {
            fade.reveal_if_ready(Instant::now(), self.world_ready && loading_done);
        }
    }

    /// True when no loading screen is holding the world back: none was shown,
    /// or it has run its bar to the end and faded out.
    fn loading_finished(&self, now: Instant) -> bool {
        match (&self.loading_screen, self.loading_leave_at) {
            (None, _) => true,
            (Some(_), Some(at)) => now.saturating_duration_since(at) >= LOADING_LEAVE,
            (Some(_), None) => false,
        }
    }

    /// Draw one loading-screen frame if one is due; returns whether the
    /// canvas now holds a frame to present.
    fn paint_loading_screen(&mut self, now: Instant) -> bool {
        let Some(fonts) = self.app.fonts.as_ref() else {
            return false;
        };
        if self.loading_screen.is_none() {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64);
            let scene = self.loading_next_scene;
            self.loading_next_scene = self.loading_next_scene.wrapping_add(1);
            let mut loader = loading::Loader::new(scene, seed);
            self.loading_last_tick = now;
            self.loading_last_paint = now - LOADING_FRAME;
            // A load that beat the black stage never needs the screen.
            if self.world_ready {
                loader.set_ready();
                if loader.can_leave() {
                    self.loading_screen = Some(loader);
                    self.loading_leave_at = Some(now - LOADING_LEAVE);
                    return false;
                }
            }
            self.loading_screen = Some(loader);
        }
        if now.saturating_duration_since(self.loading_last_paint) < LOADING_FRAME {
            return false;
        }
        let loader = self.loading_screen.as_mut().expect("loading screen exists");
        let dt = now
            .saturating_duration_since(self.loading_last_tick)
            .as_secs_f64();
        self.loading_last_tick = now;
        self.loading_last_paint = now;
        loader.tick(dt);
        if self.world_ready {
            loader.set_ready();
        }
        if loader.can_leave() && self.loading_leave_at.is_none() {
            self.loading_leave_at = Some(now);
        }
        let mut fade = 1.0f32;
        // Fade in from the black stage, and out before the reveal.
        fade = fade.min((loader.clock() as f32 / 0.3).clamp(0.0, 1.0));
        if let Some(at) = self.loading_leave_at {
            let gone =
                now.saturating_duration_since(at).as_secs_f32() / LOADING_LEAVE.as_secs_f32();
            fade = fade.min((1.0 - gone).clamp(0.0, 1.0));
        }
        if self.app.reduced_motion {
            fade = 1.0;
        }
        loading::hud::paint(&mut self.app.canvas, fonts, loader, fade);
        true
    }

    /// Hands the backpack edits made in the pause menu to the world and
    /// checkpoints, so a save made from the menu already has them.
    fn sync_backpack(&mut self) {
        if !self.app.backpack_dirty {
            return;
        }
        self.app.backpack_dirty = false;
        if let (Some(pack), Some(world)) = (&self.app.pause_backpack, &mut self.world) {
            world.commit_backpack(pack.clone());
            self.checkpoint_world(true);
        }
    }

    fn checkpoint_world(&mut self, force: bool) {
        if !self.world_ready || (!force && Instant::now() < self.next_world_checkpoint) {
            return;
        }
        let Some(world) = &mut self.world else {
            return;
        };
        world.persist_3d_state(force);
        let saved = world.snapshot();
        if let Some(game) = &mut self.app.game {
            game.checkpoint_3d(saved);
            self.app.autosave_error = game.take_autosave_error();
            self.app.continue_available = game::has_continue();
        }
        self.next_world_checkpoint = Instant::now() + Duration::from_secs(1);
    }

    fn leave_world(&mut self) {
        self.checkpoint_world(true);
        self.release_world();
        self.app.art.clear();
        self.world_ready = false;
        self.transition = None;
        self.app.screen = Screen::Title;
        self.app.needs_paint = true;
        self.app.pause = None;
    }

    fn open_game_menu(&mut self, page: pause_menu::Page) {
        if self.loading.is_some() || self.transition.is_some() {
            return;
        }
        if self.app.screen == Screen::Explore && self.world.is_some() {
            self.checkpoint_world(true);
            if let Some(world) = &mut self.world {
                world.close_overlays_for_pause();
                match world.capture_pixels() {
                    Ok(pixels) => {
                        self.app.canvas.pixels = pixels
                            .chunks_exact(4)
                            .map(|p| {
                                0xff000000 | (p[0] as u32) << 16 | (p[1] as u32) << 8 | p[2] as u32
                            })
                            .collect()
                    }
                    Err(error) => {
                        eprintln!("[menu] capture: {error}");
                        self.app.canvas.clear(0xff181220);
                    }
                }
            }
        }
        self.app.pause_story = self
            .app
            .game
            .as_ref()
            .map(|g| g.story_variables().clone())
            .unwrap_or_default();
        // What she wears: the live world's, else the last one the world saved.
        self.app.pause_outfit = match &self.world {
            Some(world) => world.outfit(),
            None => exploration3d_state::load()
                .ok()
                .flatten()
                .map_or(0, |s| s.outfit),
        };
        if let Some(world) = &self.world {
            // The status page reads the story's variables: bring the world's live ones in first.
            if let Some(game) = self.app.game.as_mut() {
                game.set_story_variables(&world.story_values());
            }
            for (key, value) in world.story_values() {
                self.app.pause_story.insert(key, value as i64);
            }
            let (gold, items) = world.inventory_for_menu();
            self.app.pause_inventory_gold = gold;
            self.app.pause_inventory_items = items;
            let (pack, craft) = world.backpack_for_menu();
            self.app.pause_backpack = Some(pack);
            self.app.pause_backpack_craft = craft;
        } else {
            self.app.pause_inventory_gold = 0;
            self.app.pause_inventory_items.clear();
            self.app.pause_backpack = None;
        }
        self.app.backpack_dirty = false;
        self.app.open_pause(page);
        self.world_paused = self.world.is_some();
    }

    fn menu_back(&mut self) {
        if self.app.world_vn_active() {
            // A world-launched VN owns the back gesture.  Do not open the
            // global pause/menu stack here: that path can end at the title
            // screen and loses the 3D interaction context.
            self.app.request_world_return();
            return;
        }
        match self.app.screen {
            Screen::GameMenu => {
                self.app.menu_key(KeyCode::Escape);
                if self.app.screen == Screen::Explore {
                    if let Some(world) = &mut self.world {
                        world.clear_input();
                    }
                }
            }
            Screen::Gallery => self.app.leave_gallery(),
            Screen::Options => {
                self.app.screen = self.app.options_return;
                self.app.needs_paint = true;
            }
            Screen::Resume => {
                self.app.screen = Screen::Title;
                self.app.needs_paint = true;
            }
            Screen::Title => {
                self.open_game_menu(pause_menu::Page::Load);
                self.app.ask_menu_confirmation(pause_menu::Confirm::Quit);
            }
            _ => self.open_game_menu(pause_menu::Page::Status),
        }
    }

    fn process_menu_action(&mut self) {
        let Some(action) = self.app.menu_action.take() else {
            return;
        };
        let result = (|| -> Result<()> {
            match action {
                pause_menu::HostAction::Load(index) => {
                    // Validate into a fresh runtime before touching the live session or autosave.
                    let mut restored = game::Game::load()?;
                    restored.load_slot(index)?;
                    restored.view()?;
                    if let Some(mut snapshot) = restored.world_snapshot().cloned() {
                        exploration3d_state::write(&mut snapshot)?;
                    } else {
                        let path = exploration3d_state::path();
                        if path.is_file() {
                            std::fs::remove_file(path)?;
                        }
                    }
                    restored.save_continue()?;
                    self.release_world();
                    self.world_ready = false;
                    self.transition = None;
                    self.app.pause = None;
                    self.app.enter_game(restored)?;
                }
                pause_menu::HostAction::Title | pause_menu::HostAction::Quit => {
                    self.checkpoint_world(true);
                    if let Some(game) = &self.app.game {
                        game.save_continue()?;
                    }
                    self.leave_world();
                    self.app.game = None;
                    self.app.view = None;
                    self.app.continue_available = game::has_continue();
                    if action == pause_menu::HostAction::Quit {
                        self.app.quit = true;
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            if let Some(menu) = &mut self.app.pause {
                menu.notice = format!("{error:#}");
            }
            self.app.autosave_error = Some(format!("{error:#}"));
            self.app.needs_paint = true;
        }
    }

    fn draw_world(&mut self, event_loop: &ActiveEventLoop, id: WindowId) {
        let signature = (
            self.app.localizer.language(),
            rect_contains(pause_menu::TRIGGER, self.app.mouse.0, self.app.mouse.1),
        );
        if self.menu_button_signature != Some(signature) {
            if let (Some(fonts), Some(display)) = (&self.app.fonts, &mut self.app.display) {
                let button = pause_menu::trigger_bitmap(fonts, signature.0, signature.1);
                display.set_menu_button(&button.rgba, button.w as u32, button.h as u32);
                self.menu_button_signature = Some(signature);
            }
        }
        let now = Instant::now();
        // Once the last VN frame has faded out, the pixel-art loading screen
        // takes the black stage. It repaints at 30 fps; between frames the
        // last one stays up and there is nothing to present.
        let black_stage = self
            .transition
            .as_ref()
            .is_some_and(|fade| fade.revealed.is_none() && fade.outgoing(now) == 0.0);
        let loading_frame = black_stage && self.paint_loading_screen(now);
        if black_stage
            && !loading_frame
            && self.loading_screen.is_some()
            && self.loading_leave_at.is_none()
        {
            return;
        }
        if let Some(fade) = &mut self.transition {
            if fade.revealed.is_none() {
                let opacity = fade.outgoing(now);
                if let Some(display) = &mut self.app.display {
                    // Retain the actual last VN frame until fully faded out.
                    // A subtle animated loading marker replaces white/blank
                    // flashes when there is no loading screen to show.
                    if opacity == 0.0 && !loading_frame {
                        self.app.canvas.clear(0xff000000);
                        let active = (fade.started.elapsed().as_millis() / 240) % 3;
                        for i in 0..3 {
                            let color = if i == active { 0xffcfa45c } else { 0xff493a28 };
                            self.app.canvas.rect(939 + i as i32 * 18, 1000, 6, 6, color);
                        }
                    }
                    let shown = display.present_canvas(
                        &self.app.canvas.pixels,
                        if opacity == 0.0 { 1.0 } else { opacity },
                    );
                    if matches!(shown, Ok(true)) && opacity == 0.0 {
                        fade.black_presented = true;
                    }
                }
                return;
            }
        }
        // The world is revealed: the loading screen has done its job.
        if self.loading_screen.take().is_some() {
            self.loading_leave_at = None;
        }
        if let Some(world) = &mut self.world {
            world.window_event(event_loop, id, WindowEvent::RedrawRequested);
            let opacity = self
                .transition
                .as_ref()
                .map_or(1.0, |fade| fade.incoming(now));
            if let (Some(display), Some(view)) = (&mut self.app.display, world.texture_view()) {
                match display.present_world(
                    view,
                    opacity,
                    !world.covers_menu_button() && !world.dungeon_active(),
                ) {
                    Ok(true) if opacity >= 1.0 => {
                        self.transition = None;
                    }
                    Err(error) => {
                        eprintln!("[game] presentación 3D falló: {error:#}");
                    }
                    _ => {}
                }
            }
        }
        self.checkpoint_world(false);
    }
}

impl ApplicationHandler for WinitApp {
    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if self.app.screen != Screen::Explore || self.transition.is_some() || self.world_paused {
            return;
        }
        if let Some(world) = &mut self.world {
            world.device_event(event_loop, device_id, event);
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.app.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(title_lines().join(" "))
            .with_inner_size(PhysicalSize::new(WIN_W, WIN_H))
            .with_min_inner_size(PhysicalSize::new(MIN_WINDOW_W, MIN_WINDOW_H))
            .with_visible(false)
            .with_resizable(true)
            .with_window_icon(app_icon());
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let display = match pollster::block_on(window_display::WindowDisplay::new(window.clone())) {
            Ok(display) => display,
            Err(error) => {
                eprintln!("[game] graphics initialization failed: {error:#}");
                #[cfg(windows)]
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::*;
                    let title: Vec<u16> = "Demon Queen\0".encode_utf16().collect();
                    let message: Vec<u16> = format!("Unable to initialize graphics.\n{error:#}\0")
                        .encode_utf16()
                        .collect();
                    MessageBoxW(
                        std::ptr::null_mut(),
                        message.as_ptr(),
                        title.as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
                self.app.quit = true;
                event_loop.exit();
                return;
            }
        };
        self.app.window = Some(window);
        self.app.display = Some(display);
        self.app.needs_paint = true;
        // Do not expose a white OS window while the first frame initializes.
        self.app.paint();
        if let Some(window) = &self.app.window {
            window.set_visible(true);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if !matches!(event, WindowEvent::RedrawRequested) {
            self.app.input_generation = self.app.input_generation.wrapping_add(1);
        }
        if matches!(event, WindowEvent::CloseRequested) {
            if self.loading.is_some() || self.transition.is_some() {
                self.close_after_load = true;
                return;
            }
            if self.app.pause.is_none() {
                self.open_game_menu(pause_menu::Page::Status);
            }
            self.app.screen = Screen::GameMenu;
            self.app.ask_menu_confirmation(pause_menu::Confirm::Quit);
            return;
        }
        if let WindowEvent::CursorMoved { position, .. } = &event {
            let size = self.app.window.as_ref().expect("game window").inner_size();
            self.app.mouse = SoftwareViewport::letterbox(size.width, size.height, WIN_W, WIN_H)
                .and_then(|v| v.physical_to_virtual(position.x, position.y))
                .map(|(x, y)| (x.floor() as i32, y.floor() as i32))
                .unwrap_or((-1, -1));
        }
        if let WindowEvent::KeyboardInput { event: ref key, .. } = event {
            if key.state == ElementState::Pressed && !key.repeat {
                if key.physical_key == PhysicalKey::Code(KeyCode::Backspace)
                    && self.app.world_vn_active()
                {
                    self.app.request_world_return();
                    return;
                }
                // An open map closes first; the menu is for the next press.
                if matches!(
                    key.physical_key,
                    PhysicalKey::Code(KeyCode::Escape | KeyCode::Backspace)
                ) && self.app.screen == Screen::Explore
                    && self
                        .world
                        .as_mut()
                        .is_some_and(|world| world.close_map_if_open())
                {
                    return;
                }
                let modal_captures_escape = self.app.screen == Screen::Explore
                    && self
                        .world
                        .as_ref()
                        .is_some_and(|world| world.modal_is_open());
                if matches!(
                    key.physical_key,
                    PhysicalKey::Code(KeyCode::Escape | KeyCode::Backspace)
                ) && !modal_captures_escape
                {
                    self.menu_back();
                    return;
                }
                if matches!(
                    key.physical_key,
                    PhysicalKey::Code(KeyCode::F5 | KeyCode::F9)
                ) && !matches!(self.app.screen, Screen::Gallery | Screen::Options)
                {
                    if self.app.pause.as_ref().is_some_and(|m| m.confirm.is_some()) {
                        return;
                    }
                    let page = if key.physical_key == PhysicalKey::Code(KeyCode::F5) {
                        pause_menu::Page::Save
                    } else {
                        pause_menu::Page::Load
                    };
                    if page != pause_menu::Page::Save || self.app.game.is_some() {
                        self.open_game_menu(page);
                    }
                    return;
                }
            }
            if key.repeat
                && matches!(
                    key.physical_key,
                    PhysicalKey::Code(
                        KeyCode::Escape | KeyCode::Backspace | KeyCode::F5 | KeyCode::F9
                    )
                )
            {
                return;
            }
        }
        if let WindowEvent::MouseInput {
            state: ElementState::Released,
            button,
            ..
        } = event
        {
            if button == MouseButton::Right
                && self.app.screen == Screen::GameMenu
                && self.app.backpack_right_click()
            {
                return;
            }
            if matches!(button, MouseButton::Right | MouseButton::Back) {
                if self.app.screen == Screen::Explore
                    && self
                        .world
                        .as_mut()
                        .is_some_and(|world| world.close_map_if_open())
                {
                    return;
                }
                if self.app.screen == Screen::Explore
                    && self
                        .world
                        .as_ref()
                        .is_some_and(|world| world.modal_is_open())
                {
                    if let Some(world) = &mut self.world {
                        world.close_overlays_for_pause();
                    }
                    return;
                }
                if self.app.screen != Screen::Title {
                    self.menu_back();
                }
                return;
            }
            if button == MouseButton::Left
                && self.app.screen == Screen::Explore
                && !self
                    .world
                    .as_ref()
                    .is_some_and(|world| world.covers_menu_button())
                && rect_contains(pause_menu::TRIGGER, self.app.mouse.0, self.app.mouse.1)
            {
                self.open_game_menu(pause_menu::Page::Status);
                return;
            }
        }
        if self.app.screen == Screen::Explore && self.world.is_some() {
            if matches!(
                event,
                WindowEvent::MouseInput {
                    button: MouseButton::Left,
                    ..
                }
            ) && rect_contains(pause_menu::TRIGGER, self.app.mouse.0, self.app.mouse.1)
                && !self
                    .world
                    .as_ref()
                    .is_some_and(|world| world.covers_menu_button())
            {
                return;
            }
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => self.draw_world(event_loop, window_id),
                WindowEvent::KeyboardInput { ref event, .. }
                    if event.physical_key == PhysicalKey::Code(KeyCode::F5)
                        && event.state == ElementState::Pressed =>
                {
                    self.checkpoint_world(true)
                }
                WindowEvent::Focused(false) => {
                    if let Some(world) = &mut self.world {
                        world.window_event(event_loop, window_id, WindowEvent::Focused(false));
                    }
                    self.checkpoint_world(true);
                }
                // While the pixel-art loading screen is up, keys and clicks are
                // its minigame: mash to help, arrows to change scene.
                WindowEvent::KeyboardInput { ref event, .. }
                    if self.transition.is_some() && self.loading_screen.is_some() =>
                {
                    if event.state == ElementState::Pressed && !event.repeat {
                        if let Some(loader) = &mut self.loading_screen {
                            match event.physical_key {
                                PhysicalKey::Code(
                                    KeyCode::Space
                                    | KeyCode::Enter
                                    | KeyCode::NumpadEnter
                                    | KeyCode::KeyZ
                                    | KeyCode::KeyX,
                                ) => loader.mash(),
                                PhysicalKey::Code(KeyCode::ArrowRight) => loader.switch(1),
                                PhysicalKey::Code(KeyCode::ArrowLeft) => loader.switch(-1),
                                _ => {}
                            }
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } if self.transition.is_some() && self.loading_screen.is_some() => {
                    if let Some(loader) = &mut self.loading_screen {
                        loader.mash();
                    }
                }
                event if self.transition.is_none() => {
                    let (
                        story_requested,
                        villa_interaction,
                        dungeon_changed,
                        dungeon_active,
                        guion_vn,
                    ) = {
                        let Some(world) = &mut self.world else {
                            return;
                        };
                        let was_in_dungeon = world.dungeon_active();
                        world.window_event(event_loop, window_id, event);
                        (
                            world.take_story_request(),
                            world.take_villa_interaction(),
                            was_in_dungeon != world.dungeon_active(),
                            world.dungeon_active(),
                            world
                                .take_guion_vn()
                                .map(|node| (node, world.story_values())),
                        )
                    };
                    if dungeon_changed {
                        if let Some(window) = &self.app.window {
                            let title = if dungeon_active {
                                "Demon Queen — Mazmorra procedural | Ratón: mirar · WASD: mover · Shift: correr · G: volver · Esc: pausa"
                                    .to_owned()
                            } else {
                                title_lines().join(" ")
                            };
                            window.set_title(&title);
                        }
                    }
                    if story_requested {
                        self.begin_world_vn("mage_vn_01_intro", None);
                    }
                    if let Some((node, values)) = guion_vn {
                        self.begin_world_vn(&node, Some(values));
                    }
                    if let Some(building_id) = villa_interaction {
                        // The marker and ID are live now; each building's VN
                        // can be added later without changing movement,
                        // collision or the interaction prompt.
                        eprintln!("[game] villa VN hook selected: {building_id} (script pending)");
                    }
                }
                _ => {}
            }
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.app.needs_paint = true;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (pw, ph) = self
                    .app
                    .window
                    .as_ref()
                    .map(|w| w.inner_size())
                    .map(|s| (s.width.max(1), s.height.max(1)))
                    .unwrap_or((WIN_W, WIN_H));
                self.app.mouse = SoftwareViewport::letterbox(pw, ph, WIN_W, WIN_H)
                    .and_then(|viewport| viewport.physical_to_virtual(position.x, position.y))
                    .map(|(x, y)| (x.floor() as i32, y.floor() as i32))
                    .unwrap_or((-1, -1));
                self.app.needs_paint = true;
            }
            WindowEvent::CursorLeft { .. } => {
                self.app.mouse = (-1, -1);
                self.app.needs_paint = true;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * SCROLL_ACTION_THRESHOLD,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32,
                };
                self.app.handle_scroll(delta_y);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == MouseButton::Left {
                    self.app.mouse_down = state == ElementState::Pressed;
                    self.app.needs_paint = true;
                    if state == ElementState::Released {
                        let (x, y) = self.app.mouse;
                        self.app.click_at(x, y);
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = event.physical_key;
                let is_alt = matches!(key, PhysicalKey::Code(KeyCode::AltLeft | KeyCode::AltRight));
                if is_alt {
                    let hidden = event.state == ElementState::Pressed;
                    self.app.set_ui_hidden(hidden);
                } else if event.state == ElementState::Pressed {
                    if ignore_repeated_key(key, event.repeat) {
                        return;
                    }
                    if let winit::keyboard::PhysicalKey::Code(code) = key {
                        self.app.handle_key(code);
                    }
                }
            }
            WindowEvent::Focused(false) => {
                self.app.stop_dialogue_skip();
                self.app.mouse_down = false;
                self.app.set_ui_hidden(false);
            }
            WindowEvent::RedrawRequested if self.app.needs_paint => {
                self.app.paint();
            }
            WindowEvent::RedrawRequested if self.app.chiquita_moving() => {
                self.app.present_chiquita();
            }
            _ => {}
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.tick_chiquita_check(event_loop);
        self.sync_backpack();
        self.process_menu_action();
        if let Some(mut smoke) = self.smoke.take() {
            if let Err(error) = smoke.tick(self, event_loop) {
                smoke.fail(&error);
                event_loop.exit();
            }
            self.smoke = Some(smoke);
        }
        if let Some(entry) = self.app.take_world_return() {
            if let Err(error) = self.app.enter_exploration(&entry) {
                self.app.autosave_error = Some(format!("{error:#}"));
                self.app.screen = Screen::Title;
                self.app.needs_paint = true;
            }
        }
        if let Some(entry) = self.app.pending_3d.take() {
            self.begin_world(entry);
        }
        self.preload_world();
        self.poll_world();
        if self.close_after_load && self.loading.is_none() && self.transition.is_none() {
            self.close_after_load = false;
            self.open_game_menu(pause_menu::Page::Status);
            self.app.ask_menu_confirmation(pause_menu::Confirm::Quit);
        }
        if self.app.screen == Screen::Explore && self.world.is_some() {
            if self.world_paused {
                if let Some(world) = &mut self.world {
                    world.resume_after_pause();
                }
                self.world_paused = false;
            }
            if let Some(world) = &mut self.world {
                world.about_to_wait(event_loop);
            }
            return;
        }
        if self.app.take_restart_request() {
            match relaunch_current_executable() {
                Ok(()) => {
                    event_loop.exit();
                    return;
                }
                Err(error) => {
                    eprintln!("no se pudo relanzar el juego: {error:#}");
                    self.app.quick_restart();
                }
            }
        }
        if self.app.quit {
            event_loop.exit();
            return;
        }
        let now = Instant::now();
        self.app.tick_dialogue_skip(now);
        let awaiting_surreal =
            self.app.screen == Screen::Title && self.app.menu_sel == 0 && !self.app.ui_hidden;
        if awaiting_surreal && now >= self.app.title_surreal_next_tick {
            self.app.needs_paint = true;
        }
        // The pause menu moves (embers, the turning sigil, entrances): ~30 frames a second while
        // it is open, unless motion is reduced.
        let menu_moving = ((self.app.screen == Screen::GameMenu && self.app.pause.is_some())
            || self.app.screen == Screen::Title)
            && !self.app.reduced_motion;
        if menu_moving && now >= self.app.menu_next_frame {
            self.app.menu_next_frame = now + Duration::from_millis(33);
            self.app.needs_paint = true;
        }
        // Minimised: nothing can be presented, so nothing is painted; look again in a moment
        // (the repaint stays pending for when the window comes back).
        if self
            .app
            .display
            .as_ref()
            .is_some_and(|display| display.is_hidden())
        {
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(250)));
            return;
        }
        if self.app.needs_paint {
            if let Some(window) = &self.app.window {
                window.request_redraw();
            }
            if self.app.demon_hover_animating
                || self.app.choice_hover_animating
                || (self.app.screen == Screen::Play && self.app.vn_card_animating)
                || self.app.ambient_animating
                || self.app.title_background_transition_active
            {
                event_loop
                    .set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(16)));
            } else if awaiting_surreal {
                event_loop
                    .set_control_flow(ControlFlow::WaitUntil(self.app.title_surreal_next_tick));
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        } else if awaiting_surreal {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.app.title_surreal_next_tick));
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
        // Chiquita alive over the visual novel: a frame every 16 ms while she is on screen.
        if self.app.chiquita_moving() {
            if let Some(window) = &self.app.window {
                window.request_redraw();
            }
            event_loop.set_control_flow(ControlFlow::WaitUntil(now + Duration::from_millis(16)));
        }
        if self.app.skipping_dialogue {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.app.next_dialogue_skip));
        }
        if menu_moving {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.app.menu_next_frame));
        }
    }
}

fn headless(route: usize) -> Result<()> {
    let mut game = game::Game::load()?;
    game.new_game()?;
    println!("DEMON QUEEN HEADLESS — historia JSON, ruta: {route}");
    let mut steps = 0;
    loop {
        let view = game.view()?;
        if view.ending {
            break;
        }
        let enabled: Vec<usize> = view
            .choices
            .iter()
            .enumerate()
            .filter_map(|(index, choice)| choice.enabled.then_some(index))
            .collect();
        if enabled.is_empty() {
            return Err(anyhow!("dead end at {}", view.id));
        }
        let decision = enabled[route % enabled.len()];
        println!(
            "[{} · {}] {} → {}",
            view.room_name, view.id, view.speaker, view.choices[decision].id
        );
        game.decide(decision)?;
        steps += 1;
        if steps >= 256 {
            return Err(anyhow!("la ruta no converge tras {steps} pasos"));
        }
    }
    let stats = game.stats()?;
    println!(
        "stats fear={} devotion={} dignity={} hunger={} decree={}",
        stats.fear, stats.devotion, stats.dignity, stats.hunger, stats.decree
    );
    println!("DEMON QUEEN HEADLESS OK — PRÓLOGO COMPLETADO");
    Ok(())
}

fn save_note_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".headless-ok")
}

fn optional_path_after(args: &[String], flag: &str) -> Option<PathBuf> {
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|position| args.get(position + 1))
        .filter(|value| !value.starts_with("--"))
        .map(PathBuf::from)
}

fn validate_story_cli(path: &Path) -> Result<()> {
    let summary = game::validate_story_path(path)?;
    let (nodes, choices) = game::localization_keys(path)?;
    print_story_validation(summary, nodes, choices)
}

fn validate_default_story_cli() -> Result<()> {
    let summary = game::validate_default_story()?;
    let (nodes, choices) = game::default_localization_keys()?;
    print_story_validation(summary, nodes, choices)
}

fn print_story_validation(
    summary: game::StorySummary,
    nodes: Vec<String>,
    choices: Vec<String>,
) -> Result<()> {
    localization::validate_catalogs(&nodes, &choices)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    println!(
        "localización: {} idiomas · {} escenas · {} decisiones",
        Language::ALL.len(),
        nodes.len(),
        choices.len()
    );
    Ok(())
}

fn image_plan_cli(path: &Path) -> Result<()> {
    let plan = game::image_plan(path)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn default_image_plan_cli() -> Result<()> {
    let plan = game::default_image_plan()?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}

fn storyboard_cli(path: &Path) -> Result<()> {
    print!("{}", game::storyboard_markdown(path)?);
    Ok(())
}

fn default_storyboard_cli() -> Result<()> {
    print!("{}", game::default_storyboard_markdown()?);
    Ok(())
}

fn export_storyboard_cli(story: Option<&Path>, output: &Path) -> Result<()> {
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let markdown = if let Some(story) = story {
        game::storyboard_markdown(story)?
    } else {
        game::default_storyboard_markdown()?
    };
    std::fs::write(output, markdown)?;
    println!("storyboard Markdown: {}", output.display());
    Ok(())
}

/// `--capture-loading <png> [scene] [seconds] [mash] [ready]`: a frame of the
/// pixel-art loading screen after playing it for a while.
fn capture_loading(path: &Path, scene: usize, seconds: f64, mash: bool, ready: bool) -> Result<()> {
    let app = native_resolution_app();
    let fonts = app
        .fonts
        .as_ref()
        .ok_or_else(|| anyhow!("no hay fuentes"))?;
    let mut loader = loading::Loader::new(scene, 7);
    let frames = (seconds * 30.0) as usize;
    for frame in 0..frames {
        if mash && frame % 5 == 0 {
            loader.mash();
        }
        if ready && frame + 45 >= frames {
            loader.set_ready();
        }
        loader.tick(1.0 / 30.0);
        let _ = loader.frame();
    }
    let mut canvas = Canvas::new(WIN_W as usize, WIN_H as usize, 0xff000000);
    loading::hud::paint(&mut canvas, fonts, &mut loader, 1.0);
    save_canvas(path, &canvas, "de la pantalla de carga")
}

/// `--capture-menu <png> [page 0-6] [focus] [confirm 0-3]`: the pause menu over
/// the current scene (pages: status, backpack, save, load, history, settings,
/// equipment). The backpack and equipment pages show a sample pack.
/// `VELVET_MENU_LIGHT=rrggbb` paints it as the light of another scene would;
/// `VELVET_LUST_POINTS=n` sets the Ero status to that many lust points (its rank's portrait).
fn capture_menu(path: &Path, page: usize, focus: usize, confirm: Option<usize>) -> Result<()> {
    let mut app = native_resolution_app();
    app.start_game()?;
    for _ in 0..6 {
        app.reveal_complete = true;
        app.confirm_choice();
    }
    app.reveal_complete = true;
    app.paint_play();
    let page = [
        pause_menu::Page::Status,
        pause_menu::Page::Inventory,
        pause_menu::Page::Save,
        pause_menu::Page::Load,
        pause_menu::Page::History,
        pause_menu::Page::Settings,
        pause_menu::Page::Equipment,
    ][page.min(6)];
    if matches!(
        page,
        pause_menu::Page::Inventory | pause_menu::Page::Equipment
    ) {
        let (gold, items, pack) = exploration3d::App::sample_menu_inventory();
        app.pause_inventory_gold = gold;
        app.pause_inventory_items = items;
        app.pause_backpack = Some(pack);
        app.pause_backpack_craft = true;
    }
    for (key, value) in [
        ("ero_masturbaciones", 7),
        ("ero_orgasmos", 9),
        ("ero_encuentros", 4),
        ("ero_con_nina", 2),
        ("ero_con_alba", 1),
        ("ero_con_noa", 1),
        ("ero_futa", 1),
        ("ero_elixir", 2),
        ("ero_hipnotizada", 1),
    ] {
        app.pause_story.insert(key.to_owned(), value);
    }
    // VELVET_LUST_POINTS=n: that many lust points instead (all as times alone), to see a rank.
    if let Some(points) = std::env::var("VELVET_LUST_POINTS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
    {
        app.pause_story.retain(|key, _| !key.starts_with("ero_"));
        app.pause_story
            .insert("ero_masturbaciones".to_owned(), points);
    }
    // VELVET_OUTFIT=n: dressed in the wardrobe's slot n (1 the green dress, 2 the bikini).
    if let Some(outfit) = std::env::var("VELVET_OUTFIT")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
    {
        app.pause_outfit = outfit;
    }
    app.open_pause(page);
    if let Some(menu) = &mut app.pause {
        menu.focus = focus;
        // VELVET_MENU_LIGHT=rrggbb: as if the scene behind had cast that light.
        if let Some(light) = std::env::var("VELVET_MENU_LIGHT")
            .ok()
            .and_then(|v| u32::from_str_radix(v.trim_start_matches('#'), 16).ok())
        {
            menu.light = light & 0x00ff_ffff;
        }
        // Captured once its entrance is over (or at VELVET_MENU_AGE seconds into it).
        let age = std::env::var("VELVET_MENU_AGE")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(5.0);
        let back = std::time::Duration::from_secs_f32(age.max(0.0));
        if let Some(then) = Instant::now().checked_sub(back) {
            menu.opened = then;
            menu.page_since = then;
        }
    }
    if let Some(confirm) = confirm {
        app.ask_menu_confirmation(
            [
                pause_menu::Confirm::Save(0),
                pause_menu::Confirm::Load(0),
                pause_menu::Confirm::Title,
                pause_menu::Confirm::Quit,
            ][confirm.min(3)],
        );
    }
    app.mouse = (-1, -1);
    app.paint_game_menu();
    // VELVET_MENU_BENCH=n: n more paints, timed (the menu repaints ~30 times a second while open).
    if let Some(n) = std::env::var("VELVET_MENU_BENCH")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n > 0)
    {
        let started = Instant::now();
        for _ in 0..n {
            app.paint_game_menu();
        }
        println!(
            "pintado del menú: {:.2} ms de media en {n} pasadas",
            started.elapsed().as_secs_f64() * 1000.0 / n as f64
        );
    }
    save_canvas(path, &app.canvas, "del menú de pausa")
}

fn native_resolution_app() -> App {
    App::default()
}

fn save_canvas(path: &Path, canvas: &Canvas, label: &str) -> Result<()> {
    let mut rgba = Vec::with_capacity(canvas.pixels.len() * 4);
    for pixel in &canvas.pixels {
        rgba.extend_from_slice(&[
            ((pixel >> 16) & 0xff) as u8,
            ((pixel >> 8) & 0xff) as u8,
            (pixel & 0xff) as u8,
            ((pixel >> 24) & 0xff) as u8,
        ]);
    }
    image::save_buffer(
        path,
        &rgba,
        canvas.w as u32,
        canvas.h as u32,
        image::ColorType::Rgba8,
    )?;
    println!("captura {label}: {}", path.display());
    Ok(())
}

fn capture_title(path: &Path, focus: usize, cross: f32) -> Result<()> {
    let mut app = native_resolution_app();
    app.screen = Screen::Title;
    // Well past its entrance; `cross` of the way from the throne to the option's place.
    app.last_painted_screen = Some(Screen::Title);
    app.title_since = Instant::now() - Duration::from_secs(4);
    app.menu_sel = focus.min(3);
    app.title_background_previous = 0;
    app.title_background_current = focus.min(3);
    app.title_background_transition_active = cross < 1.0;
    app.title_background_transition_started =
        Instant::now() - Duration::from_secs_f32(0.45 * cross.clamp(0.0, 1.0));
    app.paint_title_persona();
    save_canvas(path, &app.canvas, "del menú")
}

fn capture_title_flight(path: &Path, from: usize, to: usize, secs: f32) -> Result<()> {
    let mut app = native_resolution_app();
    app.screen = Screen::Title;
    app.last_painted_screen = Some(Screen::Title);
    app.title_since = Instant::now() - Duration::from_secs(4);
    app.menu_sel = from.min(3);
    app.title_background_current = from.min(3);
    app.title_background_previous = from.min(3);
    // The place's loop on show (its first frames decode on the reel's thread).
    for _ in 0..40 {
        app.paint_title_persona();
        std::thread::sleep(Duration::from_millis(50));
    }
    if app.title_reel.is_none() {
        return Err(anyhow!("no title reel in data/assets/title/reel"));
    }
    app.title_focus(to.min(3));
    let started = Instant::now();
    while started.elapsed().as_secs_f32() < secs {
        app.paint_title_persona();
        std::thread::sleep(Duration::from_millis(30));
    }
    app.paint_title_persona();
    save_canvas(path, &app.canvas, "del vuelo del menú")
}

fn capture_title_hover(path: &Path, index: usize) -> Result<()> {
    let mut app = native_resolution_app();
    let (width, height) = app
        .art
        .original(DEMON_OVERLAY_KEY)
        .map(|bitmap| (bitmap.w as i32, bitmap.h as i32))
        .ok_or_else(|| anyhow!("missing native menu overlay"))?;
    let layout = demon_overlay_layout(width, height);
    let rects = demon_button_rects(&layout, width, height);
    let index = index.min(rects.len() - 1);
    let rect = rects[index];
    app.mouse = (rect.0 + rect.2 / 2, rect.1 + rect.3 / 2);
    app.demon_hover_progress[index] = 1.0;
    app.jump_to_title_background(index);
    app.paint_title();
    save_canvas(path, &app.canvas, "del hover del menú")
}

fn capture_resume(path: &Path) -> Result<()> {
    let mut app = native_resolution_app();
    app.screen = Screen::Resume;
    app.paint_resume();
    save_canvas(path, &app.canvas, "de continuar partida")
}

fn capture_options(path: &Path, language: &str) -> Result<()> {
    let mut app = native_resolution_app();
    app.preview_language(language)?;
    app.open_options();
    app.paint_options();
    save_canvas(path, &app.canvas, "de opciones")
}

fn capture_gallery(path: &Path, steps: usize, language: &str) -> Result<()> {
    let mut app = native_resolution_app();
    app.preview_language(language)?;
    app.start_game()?;
    for _ in 0..steps {
        if app.screen == Screen::End {
            break;
        }
        app.reveal_complete = true;
        app.confirm_choice();
    }
    app.open_gallery(Screen::Options);
    app.gallery_sel = app.gallery.entries.len().saturating_sub(1);
    app.paint_gallery();
    save_canvas(path, &app.canvas, "de la galería")
}

fn capture_end(path: &Path, language: &str) -> Result<()> {
    let mut app = native_resolution_app();
    app.preview_language(language)?;
    app.start_game()?;
    for _ in 0..256 {
        if app.screen == Screen::Explore {
            app.reveal_complete = true;
            app.paint_play();
            return save_canvas(path, &app.canvas, "del cierre del prólogo");
        }
        app.reveal_complete = true;
        app.confirm_choice();
    }
    Err(anyhow!(
        "la ruta de captura no alcanzó el cierre del prólogo"
    ))
}

/// `VELVET_PLAY_NODE=<node id>` starts at that node instead of the beginning (then `steps` on);
/// `VELVET_PLAY_AGE=<s>` paints the last node that long after it began (a title card mid-way).
fn capture_play(path: &Path, steps: usize, language: &str) -> Result<()> {
    let mut app = native_resolution_app();
    app.preview_language(language)?;
    app.start_game()?;
    if let Ok(node) = std::env::var("VELVET_PLAY_NODE") {
        let game = app.game.as_mut().ok_or_else(|| anyhow!("sin partida"))?;
        game.jump_to_node(&node)?;
        let mut view = game.view()?;
        app.localizer.localize_scene(&mut view);
        present_world_gate(&mut view);
        app.choice_sel = first_enabled_choice(&view.choices);
        app.view = Some(view);
    }
    for _ in 0..steps {
        app.reveal_complete = true;
        app.confirm_choice();
    }
    app.reveal_complete = true;
    if let Some(age) = std::env::var("VELVET_PLAY_AGE")
        .ok()
        .and_then(|a| a.parse::<f32>().ok())
    {
        app.reveal_started = Instant::now() - Duration::from_secs_f32(age);
    }
    // `VELVET_PLAY_CHIQUITA=<s>`: Chiquita alive over the scene, as she is that long into the line.
    let chiquita = std::env::var("VELVET_PLAY_CHIQUITA")
        .ok()
        .and_then(|a| a.parse::<f32>().ok());
    let gpu = chiquita.and_then(|_| capture_gpu());
    if gpu.is_some() {
        app.screen = Screen::Play;
    }
    if let Some((device, queue)) = &gpu {
        app.chiquita.prepare(device, queue);
    }
    app.paint_play();
    if let (Some(seconds), Some((device, queue))) = (chiquita, &gpu) {
        let want = app.chiquita_want();
        app.chiquita.run_for(want, 1.5, seconds);
        if app.chiquita.render(device, queue).is_some() {
            if let Some((w, h, rgba)) = app.chiquita.snapshot(device, queue) {
                composite_premultiplied(&mut app.canvas, w, h, &rgba);
            }
        }
    }
    save_canvas(path, &app.canvas, "de escena")
}

/// A graphics device without a window, for captures that draw Chiquita alive.
fn capture_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))?;
    pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("demon-queen-capture"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
        },
        None,
    ))
    .ok()
}

/// Lays a picture premultiplied by its own coverage (sRGB bytes) over the canvas, as the window's
/// compositor does (it blends in linear light).
fn composite_premultiplied(canvas: &mut Canvas, w: u32, h: u32, rgba: &[u8]) {
    let to_linear = |c: u8| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let to_srgb = |c: f32| {
        let c = c.clamp(0.0, 1.0);
        let s = if c <= 0.003_130_8 {
            c * 12.92
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        };
        (s * 255.0).round() as u32
    };
    let (cw, ch) = (canvas.w as u32, canvas.h as u32);
    for y in 0..h.min(ch) {
        for x in 0..w.min(cw) {
            let i = ((y * w + x) * 4) as usize;
            let a = rgba[i + 3] as f32 / 255.0;
            if a <= 0.0 {
                continue;
            }
            let p = &mut canvas.pixels[(y * cw + x) as usize];
            let back = [(*p >> 16) & 0xff, (*p >> 8) & 0xff, *p & 0xff].map(|c| to_linear(c as u8));
            // The picture's bytes are sRGB-encoded linear premultiplied values.
            let front = [rgba[i], rgba[i + 1], rgba[i + 2]].map(to_linear);
            let mix = |k: usize| to_srgb(front[k] + back[k] * (1.0 - a));
            *p = 0xff00_0000 | (mix(0) << 16) | (mix(1) << 8) | mix(2);
        }
    }
}

fn capture_play_hover(path: &Path, steps: usize, index: usize) -> Result<()> {
    let mut app = native_resolution_app();
    app.start_game()?;
    for _ in 0..steps {
        app.reveal_complete = true;
        app.confirm_choice();
    }
    app.reveal_complete = true;
    let count = app
        .view
        .as_ref()
        .map(|view| view.choices.len())
        .unwrap_or_default();
    if count == 0 {
        return Err(anyhow!("scene has no choices to capture"));
    }
    let index = index.min(count - 1);
    let dialogue = dialogue_layout(&app.styles);
    let choices = story_choices_layout(&app.styles, count, dialogue.y);
    app.mouse = (
        choices.x + choices.width / 2,
        choices.item_y(index) + choices.height / 2,
    );
    if let Some(progress) = app.choice_hover_progress.get_mut(index) {
        *progress = 1.0;
    }
    app.paint_play();
    save_canvas(path, &app.canvas, "del hover de una decisión")
}

fn main() -> Result<()> {
    // Keep Windows from virtualizing the fixed 1920×1080 physical framebuffer.
    // Assets and UI pixels are presented 1:1; runtime display scaling is banned.
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::HiDpi::SetProcessDpiAwarenessContext;
        let _ = SetProcessDpiAwarenessContext(
            windows_sys::Win32::UI::HiDpi::DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        );
    }
    let args: Vec<String> = std::env::args().collect();
    if portable::enabled() {
        eprintln!(
            "Demon Queen portable · {} imágenes incrustadas · Velvet Weaver",
            portable::asset_count()
        );
    }
    if args.iter().any(|argument| argument == "--validate-story") {
        return match optional_path_after(&args, "--validate-story") {
            Some(path) => validate_story_cli(&path),
            None => validate_default_story_cli(),
        };
    }
    if args.iter().any(|argument| argument == "--image-plan") {
        return match optional_path_after(&args, "--image-plan") {
            Some(path) => image_plan_cli(&path),
            None => default_image_plan_cli(),
        };
    }
    if args.iter().any(|argument| argument == "--storyboard-md") {
        return match optional_path_after(&args, "--storyboard-md") {
            Some(path) => storyboard_cli(&path),
            None => default_storyboard_cli(),
        };
    }
    if let Some(position) = args
        .iter()
        .position(|argument| argument == "--export-storyboard")
    {
        let output = args
            .get(position + 1)
            .filter(|value| !value.starts_with("--"))
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("--export-storyboard requiere una ruta de salida"))?;
        return export_storyboard_cli(None, &output);
    }
    if let Some(pos) = args
        .iter()
        .position(|argument| argument == "--capture-resume")
    {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-resume.png"));
        return capture_resume(&path);
    }
    if let Some(pos) = args
        .iter()
        .position(|argument| argument == "--capture-options")
    {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-options.png"));
        let language = args
            .get(pos + 2)
            .map(String::as_str)
            .unwrap_or(Language::DEFAULT.code());
        return capture_options(&path, language);
    }
    if let Some(pos) = args
        .iter()
        .position(|argument| argument == "--capture-gallery")
    {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-gallery.png"));
        let steps = args
            .get(pos + 2)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(12);
        let language = args
            .get(pos + 3)
            .map(String::as_str)
            .unwrap_or(Language::DEFAULT.code());
        return capture_gallery(&path, steps, language);
    }
    if let Some(pos) = args.iter().position(|argument| argument == "--capture-end") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-prologue-end.png"));
        let language = args
            .get(pos + 2)
            .map(String::as_str)
            .unwrap_or(Language::DEFAULT.code());
        return capture_end(&path, language);
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-title-hover") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-title-hover.png"));
        let index = args
            .get(pos + 2)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        return capture_title_hover(&path, index);
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-title-flight") {
        // The title's reel: the place `from` on show, then the flight to `to`, `secs` into it.
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-title-flight.png"));
        let number = |offset: usize| args.get(pos + offset).and_then(|v| v.parse::<usize>().ok());
        let secs = args
            .get(pos + 4)
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0);
        return capture_title_flight(&path, number(2).unwrap_or(0), number(3).unwrap_or(1), secs);
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-title") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-title.png"));
        // The option in focus, and how far into the crossing to it (0..1; 1: done).
        let focus = args
            .get(pos + 2)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let cross = args
            .get(pos + 3)
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.0);
        return capture_title(&path, focus, cross);
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-loading") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-loading.png"));
        let number = |offset: usize, default: f64| {
            args.get(pos + offset)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(default)
        };
        return capture_loading(
            &path,
            number(2, 0.0) as usize,
            number(3, 4.0),
            number(4, 0.0) > 0.0,
            number(5, 0.0) > 0.0,
        );
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-menu") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-menu.png"));
        let number = |offset: usize| args.get(pos + offset).and_then(|v| v.parse::<usize>().ok());
        return capture_menu(
            &path,
            number(2).unwrap_or(0),
            number(3).unwrap_or(0),
            number(4),
        );
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-play-hover") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-play-hover.png"));
        let steps = args
            .get(pos + 2)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let index = args
            .get(pos + 3)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        return capture_play_hover(&path, steps, index);
    }
    if let Some(pos) = args.iter().position(|a| a == "--capture-play") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("demon-queen-play.png"));
        let steps = args
            .get(pos + 2)
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let language = args
            .get(pos + 3)
            .map(String::as_str)
            .unwrap_or(Language::DEFAULT.code());
        return capture_play(&path, steps, language);
    }
    if let Some(pos) = args.iter().position(|a| a == "--headless") {
        let route = args
            .get(pos + 1)
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);
        let result = headless(route % 3);
        if result.is_ok() {
            let _ = std::fs::write(save_note_path(), b"ok");
        }
        return result;
    }
    let event_loop = EventLoop::new()?;
    let mut winit_app = WinitApp::new(App::default());
    // The desktop game embeds exploration in this event loop/window/device.
    if let Some(report) = optional_path_after(&args, "--smoke-unified") {
        winit_app.smoke = Some(unified_smoke::SmokeTest::new(report)?);
    }
    if let Some(pos) = args.iter().position(|a| a == "--check-chiquita") {
        let node = args.get(pos + 1).cloned().unwrap_or_default();
        winit_app.chiquita_check = Some(ChiquitaCheck {
            node,
            started: None,
            shown: 0,
        });
    }
    event_loop.run_app(&mut winit_app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_world_gate_preserves_the_final_vn_dialogue() {
        let mut app = App::default();
        app.start_game().expect("new VN");
        for _ in 0..256 {
            if app
                .view
                .as_ref()
                .is_some_and(|view| view.world_entry.is_some())
            {
                break;
            }
            app.choice_sel = first_enabled_choice(&app.view.as_ref().expect("view").choices);
            app.confirm_choice();
        }
        let view = app.view.as_ref().expect("final view");
        // The prologue's last line is kept up, with the way into the world on it.
        assert!(
            view.id.starts_with(game::PROLOGUE_NODE_PREFIX),
            "{}",
            view.id
        );
        assert!(view.text.contains("El Consejo espera"), "{}", view.text);
        assert!(view.world_entry.is_some());
        assert_eq!(app.screen, Screen::Play);
        assert!(is_inline_continue(view));
        assert!(!view.ending);
        app.handle_key(KeyCode::KeyS);
        assert!(!app.skipping_dialogue);
        assert!(app.pending_3d.is_none());
        assert!(!app.quit);
    }

    #[test]
    fn mage_vn_back_gesture_rolls_dialogue_before_world_return() {
        let mut app = App::default();
        app.start_game().expect("new VN");
        for _ in 0..256 {
            if app
                .view
                .as_ref()
                .is_some_and(|view| view.world_entry.is_some())
            {
                break;
            }
            app.choice_sel = first_enabled_choice(&app.view.as_ref().expect("view").choices);
            app.confirm_choice();
        }
        app.open_world_vn("mage_vn_01_intro")
            .expect("open mage side VN");
        assert_eq!(app.screen, Screen::Play);
        assert!(app.world_vn_active());

        app.reveal_complete = true;
        app.advance_play();
        assert_eq!(
            app.view.as_ref().expect("greeting view").id,
            "mage_vn_02_greeting"
        );

        app.handle_scroll(SCROLL_ACTION_THRESHOLD);
        assert_eq!(
            app.view.as_ref().expect("rolled-back view").id,
            "mage_vn_01_intro"
        );
        assert!(!app.world_return_requested);

        app.handle_scroll(SCROLL_ACTION_THRESHOLD);

        assert!(app.world_return_requested);
        assert_eq!(
            app.take_world_return().map(|entry| entry.world),
            Some("act1_castle".to_string())
        );
        assert!(!app.world_return_requested);
    }

    fn png_path(key: &str) -> PathBuf {
        let relative = PathBuf::from(key);
        if relative.extension().is_some() {
            assets_root().join(relative)
        } else {
            assets_root().join(format!("{key}.png"))
        }
    }

    #[cfg(feature = "portable")]
    fn portable_png_key(key: &str) -> String {
        let normalized = key.replace('\\', "/");
        if Path::new(&normalized).extension().is_some() {
            normalized
        } else {
            format!("{normalized}.png")
        }
    }

    fn assert_native_1080p(path: &Path) {
        assert!(path.is_file(), "missing runtime art: {}", path.display());
        assert_eq!(
            image::image_dimensions(path).expect("runtime PNG decodes"),
            (WIN_W, WIN_H),
            "runtime art must be authored at 1920x1080: {}",
            path.display()
        );
    }

    fn assert_krea2_provenance(path: &Path) {
        assert_native_1080p(path);
        let bytes = std::fs::read(path).expect("runtime PNG bytes");
        let metadata = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
        for banned in [
            "ponydiffusion",
            "autismmix",
            "saffy-character-azshara",
            "kajinmanxl",
            "incase_style",
        ] {
            assert!(
                !metadata.contains(banned),
                "retired Pony-era model marker {banned:?} in {}",
                path.display()
            );
        }
        assert!(
            metadata.contains("redcraftredmixhybrida2a_30krea2.safetensors"),
            "missing RedCraft Krea2 provenance in {}",
            path.display()
        );
    }

    #[test]
    fn menu_geometry_comes_from_vcss() {
        let styles = load_menu_styles();
        let layout = title_menu_layout(&styles);
        assert_eq!((layout.x, layout.y), (660, 705));
        assert_eq!((layout.width, layout.height, layout.gap), (600, 69, 14));
    }

    #[test]
    fn title_footer_is_vcss_driven_and_advertises_alt() {
        let styles = load_menu_styles();
        let layout = title_footer_layout(&styles);
        assert_eq!((layout.x, layout.y), (18, 998));
        assert_eq!((layout.width, layout.height, layout.gap), (1884, 64, 17));
        assert!(TITLE_FOOTER_HINTS
            .iter()
            .any(|(id, key, label)| *id == "show-art"
                && *key == "ALT"
                && *label == "title.show_art"));
        let localizer = Localizer::for_language(Language::Spanish).unwrap();
        assert!(localizer.ui("title.show_art").contains("fondo"));

        let footer = component_style(&styles, "title-footer", "panel");
        assert_eq!(footer.keyword("chrome", "plain"), "arcane");
        assert_eq!(footer.number("brand-area-width", 0.0), 430.0);
        assert!(footer.number("status-font-size", 0.0) > 0.0);

        let alt = title_footer_shortcut_style(&styles, "show-art");
        assert_eq!(computed_color_argb(&alt, "border-color", 0), 0xe6df_aa64);
        assert_eq!(DEVELOPER_CREDIT, "A GAME BY VELVET WEAVER");
        assert_eq!(ENGINE_CREDIT, "WOVEN WITH VELVET ENGINE");
    }

    #[test]
    fn product_window_title_is_english() {
        assert_eq!(
            title_lines().join(" "),
            "That Time a Heart Attack Reincarnated Me as the Most Feared Demon Queen of the Dark Continent"
        );
    }

    #[test]
    fn r_restarts_only_from_title_not_during_gameplay() {
        let mut app = native_resolution_app();
        let continue_before = game::has_continue();
        app.screen = Screen::End;

        app.handle_key(KeyCode::KeyR);
        assert!(!app.restart_requested && !app.quit);
        app.screen = Screen::Title;
        app.handle_key(KeyCode::KeyR);

        assert!(app.restart_requested);
        assert!(app.quit);
        assert_eq!(app.screen, Screen::Title);
        assert_eq!(app.continue_available, continue_before);
        assert!(app.take_restart_request());
        assert!(!app.take_restart_request());
    }

    #[test]
    fn repeated_confirmation_keys_cannot_skip_story_nodes() {
        assert!(ignore_repeated_key(PhysicalKey::Code(KeyCode::Enter), true));
        assert!(ignore_repeated_key(PhysicalKey::Code(KeyCode::Space), true));
        assert!(ignore_repeated_key(
            PhysicalKey::Code(KeyCode::PageDown),
            true
        ));
        assert!(!ignore_repeated_key(
            PhysicalKey::Code(KeyCode::ArrowDown),
            true
        ));
        assert!(!ignore_repeated_key(
            PhysicalKey::Code(KeyCode::Enter),
            false
        ));
    }

    #[test]
    fn failed_process_restart_falls_back_to_content_reload() {
        let mut app = native_resolution_app();
        let continue_before = game::has_continue();
        app.screen = Screen::End;
        app.menu_sel = 2;
        app.ui_hidden = true;
        app.restart_requested = true;
        app.quit = true;
        app.art.entries.insert("stale-preview".into(), None);
        app.styles = Stylesheet::default();

        app.quick_restart();

        assert_eq!(app.screen, Screen::Title);
        assert_eq!(app.menu_sel, 0);
        assert!(!app.ui_hidden);
        assert!(!app.restart_requested);
        assert!(!app.quit);
        assert!(app.game.is_none());
        assert!(app.view.is_none());
        assert!(app.art.entries.is_empty());
        assert_eq!(app.continue_available, continue_before);
        assert_eq!(title_footer_layout(&app.styles).width, 1884);
        assert!(app.needs_paint);
    }

    #[test]
    fn art_cache_tracks_decoded_bytes_and_clears() {
        let mut cache = ArtCache::default();
        cache.insert_loaded(
            "scene-a".into(),
            Some(ArtBitmap {
                w: 1,
                h: 1,
                rgba: vec![0; 4],
            }),
        );

        assert_eq!(cache.bytes, 4);
        assert_eq!(cache.entries.len(), 1);
        cache.clear();
        assert_eq!(cache.bytes, 0);
        assert!(cache.entries.is_empty());
    }

    #[test]
    fn art_cache_drops_old_frames_before_budget_overflow() {
        let mut cache = ArtCache::default();
        cache.entries.insert("old-scene".into(), None);
        cache.bytes = ART_CACHE_BUDGET_BYTES;
        cache.insert_loaded(
            "new-scene".into(),
            Some(ArtBitmap {
                w: 1,
                h: 1,
                rgba: vec![0; 4],
            }),
        );

        assert!(!cache.entries.contains_key("old-scene"));
        assert!(cache.entries.contains_key("new-scene"));
        assert_eq!(cache.bytes, 4);
    }

    #[test]
    fn oversized_art_is_rejected_before_cache_admission() {
        assert_eq!(decoded_art_bytes(1920, 1080), Some(8_294_400));
        assert!(decoded_art_bytes(8192, 8192).is_some_and(|bytes| { bytes > MAX_ART_BITMAP_BYTES }));
    }

    #[test]
    fn continue_and_resume_navigation_follow_real_save_state() {
        assert!(!title_menu_items(false)[1].enabled);
        assert!(title_menu_items(true)[1].enabled);
        assert_eq!(first_resume_choice(false), 1);
        assert_eq!(first_resume_choice(true), 0);
        assert_eq!(next_resume_choice(2, 1, false), 1);
        assert_eq!(next_resume_choice(1, -1, false), 2);
    }

    #[test]
    fn full_dialogue_chrome_is_a_click_target() {
        let styles = load_menu_styles();
        let dialogue = dialogue_layout(&styles);
        assert!(dialogue_contains(dialogue, dialogue.x - 60, dialogue.y));
        assert!(dialogue_contains(
            dialogue,
            dialogue.x + dialogue.width + 60,
            dialogue.y + dialogue.height
        ));
        assert!(!dialogue_contains(dialogue, 0, 0));
    }

    #[test]
    fn clicking_new_game_from_resume_enters_the_story() {
        let mut app = native_resolution_app();
        app.screen = Screen::Resume;
        app.continue_available = false;
        app.choice_sel = first_resume_choice(false);
        let layout = resume_choices_layout(&app.styles);

        app.click_at(
            layout.x + layout.width / 2,
            layout.item_y(1) + layout.height / 2,
        );

        assert_eq!(app.screen, Screen::Play);
        assert!(app.game.is_some());
        assert!(app.view.is_some());
    }

    #[test]
    fn clicking_dialogue_advances_and_mouse_wheel_rolls_back() {
        let mut app = native_resolution_app();
        app.start_game().unwrap();
        app.reveal_complete = true;
        let opening = app.view.as_ref().unwrap().id.clone();
        let dialogue = dialogue_layout(&app.styles);

        app.click_at(
            dialogue.x + dialogue.width / 2,
            dialogue.y + dialogue.height / 2,
        );
        let advanced = app.view.as_ref().unwrap().id.clone();
        assert_ne!(advanced, opening);

        app.handle_scroll(SCROLL_ACTION_THRESHOLD);
        assert_eq!(app.view.as_ref().unwrap().id, opening);
    }

    #[test]
    fn button_states_come_from_vcss_without_runtime_asset_scaling() {
        let styles = load_menu_styles();
        let selected = menu_button_style(&styles, "title-menu", &MENU_ITEMS[0], true, false, false);
        assert_eq!(background_image_path(&selected), None);
        assert_eq!(selected.number("glow-strength", 0.0), 0.78);

        let disabled =
            menu_button_style(&styles, "title-menu", &MENU_ITEMS[1], false, false, false);
        assert_eq!(disabled.number("opacity", 1.0), 0.42);
    }

    #[test]
    fn demon_overlay_uses_the_rendered_asset_coordinates() {
        let mut art = ArtCache::default();
        let bitmap = art.original(DEMON_OVERLAY_KEY).expect("menu overlay asset");
        assert_eq!((bitmap.w, bitmap.h), (705, 1050));

        let layout = demon_overlay_layout(705, 1050);
        assert_eq!((layout.dst_x, layout.dst_y), (607, 15));
        assert_eq!((layout.dst_w, layout.dst_h), (705, 1050));
        let rects = demon_button_rects(&layout, 705, 1050);
        assert!(demon_contains(rects[0], 960, 660));
        assert!(!demon_contains(rects[0], 960, 800));
    }

    #[test]
    fn canvas_keeps_native_dimensions_and_coordinates() {
        let mut canvas = Canvas::new(4, 4, 0xff00_0000);
        canvas.rect(1, 1, 1, 1, 0xffff_ffff);
        assert_eq!((canvas.w, canvas.h), (4, 4));
        assert_eq!(
            canvas
                .pixels
                .iter()
                .filter(|pixel| **pixel == 0xffff_ffff)
                .count(),
            1
        );
    }

    #[test]
    fn app_framebuffer_is_native_1080p() {
        let app = native_resolution_app();
        assert_eq!((app.canvas.w, app.canvas.h), (1920, 1080));
    }

    #[test]
    fn resizable_window_preserves_canvas_coordinates() {
        let viewport = SoftwareViewport::letterbox(1600, 1000, WIN_W, WIN_H).expect("viewport");
        assert_eq!(
            (viewport.x, viewport.y, viewport.width, viewport.height),
            (0, 50, 1600, 900)
        );
        assert_eq!(
            viewport.physical_to_virtual(800.0, 500.0),
            Some((960.0, 540.0))
        );
        assert_eq!(viewport.physical_to_virtual(800.0, 20.0), None);
    }

    #[test]
    fn window_icon_is_embedded_and_square() {
        let icon = image::load_from_memory(APP_ICON_PNG).expect("embedded app icon");
        assert_eq!((icon.width(), icon.height()), (256, 256));
        assert!(app_icon().is_some());
    }

    #[test]
    fn alt_hides_chrome_on_every_menu_surface() {
        let mut app = native_resolution_app();
        app.reduced_motion = true;
        let sample_indices = [
            0usize,
            (540 * WIN_W + 960) as usize,
            ((WIN_H - 1) * WIN_W + (WIN_W - 1)) as usize,
        ];

        for screen in [
            Screen::Title,
            Screen::Resume,
            Screen::Options,
            Screen::Gallery,
            Screen::End,
        ] {
            app.screen = screen;
            app.set_ui_hidden(true);
            match screen {
                Screen::Title => app.paint_title(),
                Screen::Resume => app.paint_resume(),
                Screen::Options => app.paint_options(),
                Screen::Gallery => app.paint_gallery(),
                Screen::End => app.paint_end(),
                Screen::Play => unreachable!(),
                Screen::Explore => unreachable!(),
                Screen::GameMenu => unreachable!(),
            }
            let background_only = sample_indices.map(|index| app.canvas.pixels[index]);

            app.set_ui_hidden(false);
            match screen {
                Screen::Title => app.paint_title(),
                Screen::Resume => app.paint_resume(),
                Screen::Options => app.paint_options(),
                Screen::Gallery => app.paint_gallery(),
                Screen::End => app.paint_end(),
                Screen::Play => unreachable!(),
                Screen::Explore => unreachable!(),
                Screen::GameMenu => unreachable!(),
            }
            let with_chrome = sample_indices.map(|index| app.canvas.pixels[index]);

            assert_ne!(
                background_only, with_chrome,
                "UI did not hide on {screen:?}"
            );
        }
    }

    #[test]
    fn title_background_uses_a_short_eased_crossfade() {
        assert_eq!(TITLE_BACKGROUND_FADE, Duration::from_millis(220));
        assert_eq!(ease_out_quart(0.0), 0.0);
        assert_eq!(ease_out_quart(1.0), 1.0);
        assert!(ease_out_quart(0.5) > 0.9);

        let mut app = native_resolution_app();
        app.reduced_motion = false;
        app.set_title_background(2);
        assert_eq!(app.title_background_previous, 0);
        assert_eq!(app.title_background_current, 2);
        assert!(app.title_background_transition_active);
    }

    #[test]
    fn title_background_switches_immediately_with_reduced_motion() {
        let mut app = native_resolution_app();
        app.reduced_motion = true;
        app.set_title_background(1);
        assert_eq!(app.title_background_current, 1);
        assert_eq!(app.title_background_previous, 1);
        assert!(!app.title_background_transition_active);
    }

    #[test]
    fn native_art_regions_are_copied_one_to_one() {
        let mut canvas = Canvas::new(4, 2, 0xff00_0000);
        let art = ArtBitmap {
            w: 2,
            h: 1,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 255],
        };

        canvas.blit_rgba(1, 0, &art);
        assert_eq!(canvas.pixels[1], 0xffff_0000);
        assert_eq!(canvas.pixels[2], 0xff00_ff00);

        canvas.blit_region_adjusted(&art, 1, 0, 1, 1, 0, 1, 1.0, 1.0);
        assert_eq!(canvas.pixels[4], 0xff00_ff00);
    }

    #[test]
    fn native_button_feedback_changes_brightness_without_changing_geometry() {
        let mut canvas = Canvas::new(1, 1, 0xff00_0000);
        let art = ArtBitmap {
            w: 1,
            h: 1,
            rgba: vec![100, 80, 60, 255],
        };
        canvas.blit_region_adjusted(&art, 0, 0, 1, 1, 0, 0, 1.25, 1.0);
        assert_eq!(canvas.pixels, vec![0xff7d_644b]);
    }

    #[test]
    fn reduced_motion_snaps_interaction_feedback_without_animating() {
        let mut progress = [0.0; 8];
        let mut tick = Instant::now();
        let animating = advance_native_button_hover(&mut progress, &mut tick, Some(1), 0, 3, true);
        assert!(!animating);
        assert_eq!(progress[0], 0.0);
        assert_eq!(progress[1], 1.0);
        assert_eq!(progress[2], 0.0);
    }

    #[test]
    fn paragraph_wrap_measures_letters_without_phantom_spaces() {
        let fonts = Fonts::new().expect("system fonts");
        let canvas = Canvas::new(1, 1, 0xff00_0000);
        let text = "El aceite crujía detrás del mostrador";
        let width = fonts.text_width(text, 28.0, false);
        assert_eq!(canvas.wrap(&fonts, text, width, 28.0, false), [text]);
    }

    #[test]
    fn complex_script_font_profiles_shape_without_missing_runtime_fonts() {
        for (language, sample) in [
            ("zh-Hans", "序章完成"),
            ("zh-Hant", "序章完成"),
            ("hi", "प्रस्तावना पूरी हुई"),
            ("ja", "プロローグ完了"),
            ("ko", "프롤로그 완료"),
        ] {
            let fonts = Fonts::new_for_language(language).expect("script font profile");
            assert!(fonts.text_width(sample, 28.0, false) > 20, "{language}");
        }
    }

    #[test]
    fn automatic_story_links_use_the_inline_continue_affordance() {
        let mut game = game::Game::load().expect("story");
        game.new_game().expect("new game");
        let opening = game.view().expect("opening view");
        assert!(is_inline_continue(&opening));

        // On through the lines (each one an inline continue) up to the first real decision.
        let mut decision = opening;
        for _ in 0..80 {
            if decision.choices.len() > 1 {
                break;
            }
            assert!(is_inline_continue(&decision), "{}", decision.id);
            game.decide(0).expect("advance story");
            decision = game.view().expect("next view");
        }
        assert!(decision.choices.len() > 1);
        assert!(!is_inline_continue(&decision));
    }

    #[test]
    fn story_assets_use_the_prefixed_file_names() {
        let mut art = ArtCache::default();
        let title = art.original(TITLE_ART_KEY).expect("native title art");
        assert_eq!((title.w, title.h), (WIN_W as usize, WIN_H as usize));
        for key in TITLE_MENU_BACKGROUNDS
            .into_iter()
            .chain(std::iter::once(RESUME_BACKGROUND_KEY))
        {
            let background = art.original(key).expect("native menu background");
            assert_eq!(
                (background.w, background.h),
                (WIN_W as usize, WIN_H as usize)
            );
        }
        for background in [
            "earth_buffet",
            "void",
            "castle_bedroom",
            "throne_room",
            "castle_balcony",
        ] {
            let key = background_art_key(background);
            assert!(assets_root().join(format!("{key}.png")).is_file());
            let bitmap = art.original(&key).expect("native background art");
            assert_eq!((bitmap.w, bitmap.h), (WIN_W as usize, WIN_H as usize));
        }
        let first_cg_key = legacy_scene_art_key("p01_last_tray");
        assert!(assets_root().join(format!("{first_cg_key}.png")).is_file());
        let first_cg = art
            .original(&first_cg_key)
            .expect("native prologue scene art");
        assert_eq!((first_cg.w, first_cg.h), (WIN_W as usize, WIN_H as usize));

        let clerk_queen_key = legacy_scene_art_key("p02_clerk_queen");
        assert!(assets_root()
            .join(format!("{clerk_queen_key}.png"))
            .is_file());
        let clerk_queen = art
            .original(&clerk_queen_key)
            .expect("native soul-department scene art");
        assert_eq!(
            (clerk_queen.w, clerk_queen.h),
            (WIN_W as usize, WIN_H as usize)
        );
    }

    #[test]
    fn every_ready_story_asset_and_room_background_exists_at_native_resolution() {
        let bytes = std::fs::read(game::story_path()).expect("authoritative story JSON");
        let document: serde_json::Value =
            serde_json::from_slice(&bytes).expect("valid authoritative story JSON");

        for room in document["rooms"].as_array().expect("story rooms") {
            let background = room["background"].as_str().expect("room background");
            let key = background_art_key(background);
            assert_native_1080p(&png_path(&key));
            #[cfg(feature = "portable")]
            assert!(
                portable::asset_bytes(&portable_png_key(&key)).is_some(),
                "room background absent from portable bundle: {key}"
            );
        }

        for node in document["nodes"].as_array().expect("story nodes") {
            let visual = &node["visual"];
            if visual["status"].as_str() != Some("ready") {
                continue;
            }
            let node_id = node["id"].as_str().expect("node id");
            let asset = visual["asset"]
                .as_str()
                .unwrap_or_else(|| panic!("ready node {node_id} has no asset"));
            assert!(
                !asset.is_empty() && asset != "pendiente",
                "ready node {node_id} has placeholder asset"
            );
            assert_native_1080p(&png_path(asset));
            #[cfg(feature = "portable")]
            assert!(
                portable::asset_bytes(&portable_png_key(asset)).is_some(),
                "ready node {node_id} absent from portable bundle: {asset}"
            );
        }
    }

    #[test]
    fn active_runtime_illustrations_use_krea2_not_pony() {
        let mut paths = vec![png_path(TITLE_ART_KEY)];
        for directory in ["native_1080p/bg", "native_1080p/cg"] {
            let entries = std::fs::read_dir(assets_root().join(directory))
                .unwrap_or_else(|error| panic!("cannot inspect {directory}: {error}"));
            for entry in entries {
                let path = entry.expect("runtime art entry").path();
                if path.extension().and_then(|extension| extension.to_str()) == Some("png") {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        paths.dedup();
        for path in paths {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("runtime illustration filename");
            if file_name == "p06_drops.png" {
                // CG43 fue aprobado por revisión visual, pero su PNG fuente no
                // conserva metadatos ComfyUI/Krea2. Se mantiene como excepción
                // explícita y se verifica resolución nativa.
                assert_native_1080p(&path);
            } else {
                assert_krea2_provenance(&path);
            }
        }

        // Four menu plates came from user-approved Krea/Niji references whose
        // clipboard PNGs had no embedded Comfy workflow. They are explicit,
        // narrow exceptions; every generated title plate must retain metadata.
        let approved_reference_only = [
            "menu_bg_garden.png",
            "menu_bg_candy.png",
            "menu_bg_glitch.png",
            "menu_bg_gold.png",
        ];
        for key in TITLE_MENU_BACKGROUNDS.into_iter().chain([
            RESUME_BACKGROUND_KEY,
            OPTIONS_BACKGROUND_KEY,
            END_BACKGROUND_KEY,
        ]) {
            let path = png_path(key);
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("title filename");
            if approved_reference_only.contains(&file_name) {
                assert_native_1080p(&path);
            } else {
                assert_krea2_provenance(&path);
            }
        }
    }

    #[test]
    fn narrative_surfaces_use_arcane_vcss_chrome() {
        let styles = load_menu_styles();
        let dialogue = component_style(&styles, "dialogue-panel", "panel");
        assert_eq!(dialogue.keyword("chrome", "plain"), "arcane");
        assert!(dialogue.number("inner-border-width", 0.0) > 0.0);
        assert!(dialogue.number("ornament-center", 0.0) >= 0.5);

        let choice = choice_style(&styles, "story-choices", 0, true, true, false, false);
        assert_eq!(choice.keyword("chrome", "plain"), "arcane");
        assert!(choice.number("marker-size", 0.0) >= 24.0);
        assert!(choice.number("glow-strength", 0.0) > 0.0);

        let stats = component_style(&styles, "stats-hud", "panel");
        assert_eq!(stats.keyword("chrome", "plain"), "arcane");
        assert_eq!(stats.number("track-height", 0.0), 10.0);
        assert!(stats.number("inner-border-width", 0.0) > 0.0);

        let ending = component_style(&styles, "prologue-end-panel", "panel");
        assert_eq!(ending.keyword("chrome", "plain"), "arcane");
        assert!(ending.number("ornament-sides", 0.0) >= 0.5);
    }

    fn solid_bitmap(w: usize, h: usize, fill: impl Fn(usize, usize) -> [u8; 3]) -> ArtBitmap {
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let c = fill(x, y);
                rgba.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
        ArtBitmap { w, h, rgba }
    }

    #[test]
    fn scene_light_follows_the_colour_that_covers_most_of_the_picture() {
        // A mostly green forest with a small red lantern and black corners.
        let forest = solid_bitmap(320, 180, |x, y| {
            if x < 40 && y < 40 {
                [0, 0, 0]
            } else if x > 280 && y > 140 {
                [220, 30, 30]
            } else {
                [60, 140, 70]
            }
        });
        let (hue, saturation, value) = rgb_to_hsv(dominant_light(&forest));
        assert!((hue - 128.0).abs() < 20.0, "hue {hue}");
        assert!(saturation >= 0.45 && value >= 0.86);
        // A warm room reads warm.
        let room = solid_bitmap(320, 180, |_, _| [200, 130, 60]);
        let (hue, ..) = rgb_to_hsv(dominant_light(&room));
        assert!((20.0..45.0).contains(&hue), "hue {hue}");
        // A grey picture still gives a usable light (never black or NaN).
        let grey = solid_bitmap(64, 64, |_, _| [90, 90, 90]);
        let light = dominant_light(&grey);
        assert!(light
            .iter()
            .all(|c| c.is_finite() && *c >= 0.0 && *c <= 1.0));
        assert!(light.iter().cloned().fold(0.0, f32::max) >= 0.86);
    }

    #[test]
    fn story_choices_clear_the_speaker_plate() {
        let styles = load_menu_styles();
        let dialogue = dialogue_layout(&styles);
        let speaker = speaker_layout(&styles);
        let choices = story_choices_layout(&styles, 3, dialogue.y);
        assert_eq!((choices.x, choices.width, choices.height), (410, 1100, 80));
        let choices_bottom = choices.item_y(2) + choices.height;
        assert!(choices_bottom <= speaker.y - 60);
    }
}

/// Tests save to a scratch folder of their own: reaching the world's gate writes the 3D session, and
/// a test must never touch the player's games.
#[cfg(test)]
fn isolate_test_saves() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir =
            std::env::temp_dir().join(format!("demon-queen-test-saves-{}", std::process::id()));
        // Set once, before any test thread reads it.
        std::env::set_var("DEMON_QUEEN_SAVES", dir);
    });
}
