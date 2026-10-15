//! Shared, non-destructive pause menu. The host retains the live 3D scene.
//! Slot files include story, rollback, map flags/transform and a native-size thumbnail.
use std::collections::BTreeMap;
use std::io::Cursor;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::backpack_menu as bpm;
use crate::exploration3d::ui_sfx::{self, UiSfx};
use crate::menu_ui as ui;
use crate::prompt_icons::Icon;
use crate::{
    centered_in_rect, ellipsize, game,
    localization::Language,
    rect_contains,
    render::{Canvas, Fonts},
    App, ArtBitmap, Screen,
};
use winit::keyboard::KeyCode;

pub const TRIGGER: (i32, i32, i32, i32) = crate::window_display::MENU_RECT;
const GOLD: u32 = 0xffd9b67b;
const INK: u32 = 0xfff5eaf3;
const MUTED: u32 = 0xffbaacbf;
const EDGE: u32 = 0xff58435d;
const DANGER: u32 = 0xffe8737f;
pub(crate) const NAV_COUNT: usize = 10;
const HISTORY_VISIBLE_LINES: usize = 18;
/// How long the menu takes to come in, and a page to wipe in, in seconds.
const ENTER: f32 = 0.38;
const WIPE: f32 = 0.36;
/// The page's pane.
const PANE: (i32, i32, i32, i32) = (440, 48, 1440, 984);
/// The Queen's illustration on the status page, at the size its art is prepared.
pub(crate) const PORTRAIT: (i32, i32, i32, i32) = (480, 222, 432, 738);
/// Where that art goes (under `data/assets`): one per lust rank, `reina_status_<rank>.png` (her
/// clothes come off as the rank rises), else this one; until any exists, its frame says so.
const PORTRAIT_FILE: &str = "ui/status/reina_status.png";

struct HistoryLine {
    text: String,
    speaker: bool,
}

fn wrap_history(canvas: &Canvas, fonts: &Fonts, history: &[game::SceneView]) -> Vec<HistoryLine> {
    let mut lines = Vec::new();
    for view in history {
        lines.push(HistoryLine {
            text: view.speaker.clone(),
            speaker: true,
        });
        lines.extend(
            canvas
                .wrap(fonts, &view.text, 1238, 24.0, false)
                .into_iter()
                .map(|text| HistoryLine {
                    text,
                    speaker: false,
                }),
        );
        lines.push(HistoryLine {
            text: String::new(),
            speaker: false,
        });
    }
    lines
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Status,
    Inventory,
    Equipment,
    Save,
    Load,
    History,
    Settings,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostAction {
    Load(usize),
    Title,
    Quit,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirm {
    Save(usize),
    Load(usize),
    Title,
    Quit,
}

pub struct PauseMenu {
    pub origin: Screen,
    pub page: Page,
    pub opened: Instant,
    pub background: Vec<u32>,
    pub focus: usize,
    /// State of the grid backpack and equipment screens.
    pub bp: bpm::Ui,
    pub slot_page: usize,
    pub history_offset: usize,
    pub slots: Vec<Option<game::SaveSlot>>,
    thumbs: Vec<Option<ArtBitmap>>,
    pub history: Vec<game::SceneView>,
    wrapped_history: Vec<HistoryLine>,
    pub notice: String,
    pub confirm: Option<Confirm>,
    pub confirm_yes: bool,
    /// 0xRRGGBB: the colour of the scene behind the menu, which lights it.
    pub light: u32,
    /// When the page on show came up (its entrance animation counts from here).
    pub page_since: Instant,
    /// The still layer under the menu (scene, band, pane), kept once the entrance is over.
    backdrop: Vec<u32>,
    /// The Queen's illustration for the status page, fitted to its frame.
    portrait: Option<ArtBitmap>,
    /// When the menu was last painted in full, and after which of the window's events.
    painted: Option<(Instant, u64)>,
    /// The settings page's panel (tab, row in focus).
    pub settings: crate::settings_menu::SettingsUi,
}

/// English keeps its own words; every other language draws the Spanish source, which the
/// draw-time translation (the game's text catalogs) turns into that language.
fn tr(language: Language, es: &'static str, en: &'static str) -> &'static str {
    if language != Language::English {
        es
    } else {
        en
    }
}
fn nav_rect(index: usize) -> (i32, i32, i32, i32) {
    (76, 226 + index as i32 * 66, 308, 56)
}
fn card_rect(index: usize) -> (i32, i32, i32, i32) {
    (
        474 + (index % 3) as i32 * 446,
        226 + (index / 3) as i32 * 334,
        416,
        302,
    )
}
const PREVIOUS: (i32, i32, i32, i32) = (1548, 898, 112, 56);
const NEXT: (i32, i32, i32, i32) = (1692, 898, 112, 56);
const AUTO: (i32, i32, i32, i32) = (474, 898, 480, 56);
const CANCEL: (i32, i32, i32, i32) = (650, 600, 280, 62);
/// The settings panel's place on its page.
pub(crate) const SETTINGS_AREA: crate::settings_menu::Area = (486, 236, 1306);
/// The settings page's place in the list down the left.
const SETTINGS_NAV: usize = 7;
const ACCEPT: (i32, i32, i32, i32) = (990, 600, 280, 62);

/// The shop's vector icon `file` (in `assets/ui/shop`) at 256 px.
pub(crate) fn shop_icon(file: &str) -> Option<crate::ArtBitmap> {
    let root = Some(crate::exploration3d_state::game_dir().join("assets/ui/shop"))
        .filter(|p| p.is_dir())?;
    let source = std::fs::read_to_string(root.join(file)).ok()?;
    // Rasterized large and filtered down, like the shop's own atlas.
    let big = velvet_image::rasterize_simple_svg(&source, 768, 768).ok()?;
    let big = image::RgbaImage::from_raw(768, 768, big.pixels)?;
    let small = image::imageops::resize(&big, 256, 256, image::imageops::FilterType::Lanczos3);
    Some(crate::ArtBitmap {
        w: 256,
        h: 256,
        rgba: small.into_raw(),
    })
}

/// A stand-in glyph for items that ship without an icon of their own.
fn fallback_glyph(id: &str) -> Option<Icon> {
    Some(match id {
        i if i.starts_with("pocion") || i.starts_with("elixir") || i == "esencia_carmesi" => {
            Icon::Heart
        }
        "hierbas_curativas" | "cataplasma_herbal" => Icon::Forest,
        "pan_del_dia" => Icon::Feast,
        "tela_resistente" => Icon::Wardrobe,
        "espada_de_madera" | "espada_reliquia" => Icon::Mine,
        "tinta_runica" | "fragmento_sello" | "sello_inscrito" => Icon::Rune,
        "cristal_memoria" | "amuleto_ocultacion" => Icon::Memory,
        "llave_boveda" => Icon::Lock,
        "mapa_mercado_bajo" => Icon::Door,
        "caja_cerrada" => Icon::Chest,
        "hueso_viejo" | "craneo_campeon" | "esquirla_corazon" => Icon::Crack,
        "ceniza_arcana" => Icon::Loot,
        "gelatina_azul" | "gelatina_verde" => Icon::Bath,
        "perfume_sucubo" => Icon::Rest,
        _ => return None,
    })
}

pub(crate) fn paint_item_icon(
    canvas: &mut Canvas,
    fonts: &Fonts,
    item: &crate::exploration3d::InventoryMenuItem,
    rect: (i32, i32, i32, i32),
    background: u32,
    label_size: f32,
) {
    let cx = rect.0 + rect.2 / 2;
    let cy = rect.1 + rect.3 / 2;
    if let Some(file) = item.icon_svg.as_deref() {
        static ICONS: std::sync::OnceLock<
            std::sync::Mutex<std::collections::HashMap<String, Option<crate::ArtBitmap>>>,
        > = std::sync::OnceLock::new();
        let mut icons = ICONS.get_or_init(Default::default).lock().unwrap();
        let icon = icons
            .entry(file.to_owned())
            .or_insert_with(|| shop_icon(file));
        if let Some(icon) = icon {
            canvas.blit_rgba_scaled(rect.0, rect.1, rect.2, rect.3, icon, 1.0);
            return;
        }
    }
    if item.id == "anillo_misterioso" {
        let radius = rect.2.min(rect.3) / 3;
        canvas.circle(cx, cy, radius + (rect.2 / 12).max(2), 0x30e66eb2);
        canvas.circle(cx, cy, radius, 0xffe6b54b);
        canvas.circle(cx, cy, (radius as f32 * 0.58) as i32, background);
        canvas.circle(
            cx,
            cy - (radius as f32 * 0.62) as i32,
            (radius as f32 * 0.34) as i32,
            0xffbf4caa,
        );
        canvas.circle(
            cx,
            cy - (radius as f32 * 0.62) as i32,
            (radius as f32 * 0.16) as i32,
            0xffffd8f2,
        );
    } else if let Some(glyph) = fallback_glyph(&item.id) {
        // No art for this item: a vector glyph in ivory, sized to the plate.
        let size = rect.2.min(rect.3) as f32 * 0.86;
        ui::icon(
            canvas,
            glyph,
            cx,
            cy,
            size as i32,
            0xfff2e4cf,
            background & 0x00ff_ffff,
        );
    } else {
        let icon = item
            .icon
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        centered_in_rect(canvas, fonts, &icon, rect, label_size, 0xfff5eaf3, true);
    }
}

fn frame(canvas: &mut Canvas, rect: (i32, i32, i32, i32), fill: u32, edge: u32) {
    let (x, y, w, h) = rect;
    canvas.rect(x, y, w, h, fill);
    canvas.rect(x, y, w, 1, edge);
    canvas.rect(x, y + h - 1, w, 1, edge);
    canvas.rect(x, y, 1, h, edge);
    canvas.rect(x + w - 1, y, 1, h, edge);
}
fn button(
    canvas: &mut Canvas,
    fonts: &Fonts,
    rect: (i32, i32, i32, i32),
    text: &str,
    focused: bool,
    enabled: bool,
) {
    frame(
        canvas,
        rect,
        if focused { 0xff38223e } else { 0xff201727 },
        if focused { GOLD } else { EDGE },
    );
    centered_in_rect(
        canvas,
        fonts,
        text,
        rect,
        23.0,
        if !enabled {
            MUTED
        } else if focused {
            GOLD
        } else {
            INK
        },
        true,
    );
}
fn age(saved_at: u64, language: Language) -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(saved_at);
    if seconds < 60 {
        tr(language, "Ahora", "Just now").into()
    } else if seconds < 3600 {
        format!("{} min", seconds / 60)
    } else if seconds < 86400 {
        format!("{} h", seconds / 3600)
    } else {
        format!("{} {}", seconds / 86400, tr(language, "días", "days"))
    }
}

/// The corner button that opens the menu, in the game's Persona manner: a slanted dark card with
/// a wedge of colour, the key on its own slanted cap and the word beside it. `(x, y)`: its corner.
pub fn paint_trigger(
    canvas: &mut Canvas,
    fonts: &Fonts,
    (x, y): (i32, i32),
    key: &str,
    label: &str,
    hover: bool,
    mask: bool,
) {
    let (x, y) = (x as f32, y as f32);
    let accent = if hover { 0xffffd98a } else { 0xff9a5cff };
    let pick = |c: u32| if mask { 0xffffffff } else { c };
    ui::plate(canvas, x + 10.0, y + 8.0, 190.0, 46.0, 12.0, pick(0xb0000000));
    ui::plate(canvas, x + 4.0, y + 4.0, 190.0, 46.0, 12.0, pick(0xf00e0816));
    ui::plate(canvas, x + 4.0, y + 4.0, 22.0, 46.0, 12.0, pick(accent));
    if mask {
        return;
    }
    ui::outline(
        canvas,
        &ui::plate_points(x + 4.0, y + 4.0, 190.0, 46.0, 12.0),
        if hover { 1.8 } else { 1.0 },
        ui::argb(0xc0, accent & 0xffffff),
    );
    // The key on its slanted cap.
    let kw = fonts.text_width(key, 17.0, true) as f32 + 26.0;
    ui::plate(canvas, x + 32.0, y + 12.0, kw, 30.0, 7.0, accent);
    ui::plate(canvas, x + 35.0, y + 14.5, kw - 6.0, 25.0, 6.0, 0xff160c22);
    ui::text_center(
        canvas,
        fonts,
        key,
        (x + 32.0 + kw * 0.5) as i32,
        (y + 27.0) as i32,
        17.0,
        0xfffff6ee,
        true,
    );
    ui::text_mid(
        canvas,
        fonts,
        label,
        (x + 44.0 + kw) as i32,
        (y + 27.0) as i32,
        23.0,
        if hover { 0xffffe7ae } else { 0xfffff6ee },
        true,
    );
}

pub fn trigger_bitmap(fonts: &Fonts, language: Language, hover: bool) -> ArtBitmap {
    // Painted on black (so the colour comes out premultiplied) and again in white as its mask:
    // the card's own transparency, with soft edges, over the world.
    let (w, h) = (204usize, 58usize);
    let label = tr(language, "Menú", "Menu");
    let mut colour = Canvas::new(w, h, 0xff000000);
    paint_trigger(&mut colour, fonts, (0, 0), "ESC", label, hover, false);
    let mut mask = Canvas::new(w, h, 0xff000000);
    paint_trigger(&mut mask, fonts, (0, 0), "ESC", label, hover, true);
    ArtBitmap {
        w,
        h,
        rgba: colour
            .pixels
            .iter()
            .zip(&mask.pixels)
            .flat_map(|(p, m)| {
                let a = ((m >> 8) & 255) as u8;
                [
                    (((p >> 16) & 255) as u8).min(a),
                    (((p >> 8) & 255) as u8).min(a),
                    ((p & 255) as u8).min(a),
                    a,
                ]
            })
            .collect(),
    }
}

impl App {
    pub fn open_pause(&mut self, page: Page) {
        self.stop_dialogue_skip();
        self.quick_action_focus = None;
        if self.pause.is_none() {
            ui_sfx::play(UiSfx::Open);
            let origin = self.screen;
            let mut history = self
                .game
                .as_ref()
                .map(|g| g.history_views())
                .unwrap_or_default();
            for view in &mut history {
                self.localizer.localize_scene(view);
            }
            let wrapped_history = self
                .fonts
                .as_ref()
                .map(|fonts| wrap_history(&self.canvas, fonts, &history))
                .unwrap_or_default();
            self.pause = Some(PauseMenu {
                origin,
                page,
                opened: Instant::now(),
                background: self.canvas.pixels.clone(),
                focus: 0,
                bp: Default::default(),
                slot_page: 0,
                history_offset: 0,
                slots: Vec::new(),
                thumbs: Vec::new(),
                history,
                wrapped_history,
                notice: String::new(),
                confirm: None,
                confirm_yes: false,
                light: ui::light_from_pixels(&self.canvas.pixels, self.canvas.pw, self.canvas.ph),
                page_since: Instant::now(),
                backdrop: Vec::new(),
                portrait: load_status_portrait(
                    lust_rank(lust_points(&self.pause_story)),
                    self.pause_outfit,
                ),
                painted: None,
                settings: Default::default(),
            });
            self.refresh_slots();
        }
        if let Some(menu) = &mut self.pause {
            if menu.page != page {
                menu.page_since = Instant::now();
            }
            menu.page = page;
        }
        self.screen = Screen::GameMenu;
        self.ui_hidden = false;
        self.mouse_down = false;
        self.scroll_accumulator = 0.0;
        self.needs_paint = true;
    }

    pub fn resume_pause(&mut self) {
        if let Some(menu) = self.pause.take() {
            ui_sfx::play(UiSfx::Close);
            self.screen = menu.origin;
            self.reveal_started += menu.opened.elapsed();
            self.canvas.restore(&menu.background);
            self.mouse_down = false;
            self.needs_paint = true;
        }
    }

    fn refresh_slots(&mut self) {
        let slots = self
            .game
            .as_ref()
            .map(|g| g.save_slots())
            .or_else(|| game::Game::load().ok().map(|g| g.save_slots()))
            .unwrap_or_default();
        let thumbs = slots
            .iter()
            .map(|slot| {
                let bytes = slot.as_ref()?.thumbnail.as_ref()?;
                let img = image::load_from_memory(bytes).ok()?.to_rgba8();
                // Thumbnails are authored once, not resized by the live renderer.
                if img.dimensions() != (368, 207) {
                    return None;
                }
                Some(ArtBitmap {
                    w: 368,
                    h: 207,
                    rgba: img.into_raw(),
                })
            })
            .collect();
        if let Some(menu) = &mut self.pause {
            menu.slots = slots;
            menu.thumbs = thumbs;
        }
    }

    pub fn ask_menu_confirmation(&mut self, confirm: Confirm) {
        ui_sfx::play(UiSfx::Select);
        if let Some(menu) = &mut self.pause {
            menu.confirm = Some(confirm);
            menu.confirm_yes = false;
        }
        self.needs_paint = true;
    }

    pub fn save_menu_slot(&mut self, index: usize) {
        let result = (|| -> anyhow::Result<()> {
            let menu = self
                .pause
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("menú cerrado"))?;
            anyhow::ensure!(
                matches!(menu.origin, Screen::Play | Screen::Explore | Screen::End),
                "sin partida activa"
            );
            let bytes: Vec<u8> = menu
                .background
                .iter()
                .flat_map(|p| {
                    [
                        ((p >> 16) & 255) as u8,
                        ((p >> 8) & 255) as u8,
                        (p & 255) as u8,
                        255,
                    ]
                })
                .collect();
            // The frozen world is in the canvas's own pixels (the screen's, with native
            // resolution), whatever size that is.
            let (w, h) = if menu.background.len() == self.canvas.pw * self.canvas.ph {
                (self.canvas.pw as u32, self.canvas.ph as u32)
            } else {
                let w = ((menu.background.len() as f64 * 16.0 / 9.0).sqrt().round() as u32).max(1);
                (w, menu.background.len() as u32 / w)
            };
            let image = image::RgbaImage::from_raw(w, h, bytes)
                .ok_or_else(|| anyhow::anyhow!("captura inválida"))?;
            let thumb =
                image::imageops::resize(&image, 368, 207, image::imageops::FilterType::Triangle);
            let mut output = Cursor::new(Vec::new());
            thumb.write_to(&mut output, image::ImageFormat::Png)?;
            // The bag as she has laid it out in this menu goes in too.
            let pack = self.pause_backpack.clone();
            let game = self
                .game
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("sin partida"))?;
            if let Some(pack) = pack {
                game.set_world_backpack(pack);
            }
            game.save_slot(index, Some(output.into_inner()))
        })();
        let language = self.localizer.language();
        if let Some(menu) = &mut self.pause {
            ui_sfx::play(if result.is_ok() {
                UiSfx::Deal
            } else {
                UiSfx::Error
            });
            menu.notice = match result {
                Ok(()) => format!(
                    "{} {:02}",
                    tr(language, "Partida guardada · Ranura", "Saved · Slot"),
                    index + 1
                ),
                Err(error) => format!(
                    "{}: {error}",
                    tr(language, "No se pudo guardar", "Save failed")
                ),
            };
        }
        self.refresh_slots();
        self.needs_paint = true;
    }

    fn activate_menu_item(&mut self, index: usize) {
        let active = self
            .pause
            .as_ref()
            .is_some_and(|m| matches!(m.origin, Screen::Play | Screen::Explore | Screen::End));
        match index {
            0 => self.resume_pause(),
            1 if active => self.set_menu_page(Page::Save),
            2 => self.set_menu_page(Page::Load),
            3 => self.set_menu_page(Page::Inventory),
            4 => self.set_menu_page(Page::Equipment),
            5 => self.set_menu_page(Page::Status),
            6 => self.set_menu_page(Page::History),
            7 => self.set_menu_page(Page::Settings),
            8 => self.ask_menu_confirmation(Confirm::Title),
            9 => self.ask_menu_confirmation(Confirm::Quit),
            _ => {}
        }
    }

    pub fn set_menu_page(&mut self, page: Page) {
        if let Some(menu) = &mut self.pause {
            if menu.page != page {
                ui_sfx::play(UiSfx::Page);
                menu.page_since = Instant::now();
            }
            menu.page = page;
            menu.confirm = None;
            menu.bp.held = None;
            if matches!(page, Page::Inventory | Page::Equipment) {
                menu.focus = NAV_COUNT;
            }
        }
        self.needs_paint = true;
    }

    fn activate_slot(&mut self, index: usize) {
        let Some(menu) = &self.pause else {
            return;
        };
        let exists = menu.slots.get(index).and_then(Option::as_ref);
        match menu.page {
            Page::Save if index < game::MANUAL_SLOTS => {
                if exists.is_some() {
                    self.ask_menu_confirmation(Confirm::Save(index));
                } else {
                    self.save_menu_slot(index);
                }
            }
            Page::Load if exists.is_some_and(|slot| slot.error.is_none()) => {
                self.ask_menu_confirmation(Confirm::Load(index));
            }
            _ => {}
        }
    }

    pub fn confirm_menu_action(&mut self) {
        let Some(action) = self.pause.as_mut().and_then(|m| m.confirm.take()) else {
            return;
        };
        ui_sfx::play(UiSfx::Select);
        match action {
            Confirm::Save(index) => self.save_menu_slot(index),
            Confirm::Load(index) => self.menu_action = Some(HostAction::Load(index)),
            Confirm::Title => self.menu_action = Some(HostAction::Title),
            Confirm::Quit => self.menu_action = Some(HostAction::Quit),
        }
        self.needs_paint = true;
    }

    fn menu_control(&mut self, index: usize) {
        let Some(menu) = &self.pause else {
            return;
        };
        match menu.page {
            Page::Save | Page::Load => match index {
                0..=5 => self.activate_slot(menu.slot_page * 6 + index),
                6 if menu.page == Page::Load => self.activate_slot(game::MANUAL_SLOTS),
                7 | 8 => {
                    if let Some(m) = &mut self.pause {
                        m.slot_page = if index == 7 {
                            m.slot_page.saturating_sub(1)
                        } else {
                            (m.slot_page + 1).min(1)
                        };
                    }
                }
                _ => {}
            },
            Page::History => self.menu_scroll(index == 0),
            Page::Settings => {}
            Page::Status | Page::Inventory => {}
            Page::Equipment => self.equipment_control(index),
        }
        self.needs_paint = true;
    }

    /// The dialogue history again, in the language now chosen.
    pub(crate) fn refresh_pause_history(&mut self) {
        let history = self
            .game
            .as_ref()
            .map(|g| g.history_views())
            .unwrap_or_default();
        if let Some(menu) = &mut self.pause {
            menu.history = history;
            for view in &mut menu.history {
                self.localizer.localize_scene(view);
            }
            menu.wrapped_history = self
                .fonts
                .as_ref()
                .map(|fonts| wrap_history(&self.canvas, fonts, &menu.history))
                .unwrap_or_default();
            menu.history_offset = 0;
        }
    }

    pub fn menu_key(&mut self, key: KeyCode) {
        // A stack in hand goes back with Esc, wherever the focus is; on the
        // grid the keys belong to the backpack first.
        let backpack_keys = self.pause.as_ref().is_some_and(|m| {
            m.confirm.is_none()
                && m.page == Page::Inventory
                && (m.focus >= NAV_COUNT || (m.bp.held.is_some() && key == KeyCode::Escape))
        });
        if backpack_keys {
            if key == KeyCode::Escape {
                if let Some(m) = &mut self.pause {
                    if m.bp.held.take().is_some() {
                        self.needs_paint = true;
                        return;
                    }
                }
            }
            if self.pause.as_ref().is_some_and(|m| m.focus >= NAV_COUNT) && self.backpack_key(key) {
                return;
            }
        }
        let equipment_controls = self.equipment_control_count();
        let Some(menu) = &mut self.pause else {
            return;
        };
        if menu.confirm.is_some() {
            match key {
                KeyCode::Escape => {
                    menu.confirm = None;
                    ui_sfx::play(UiSfx::Back);
                }
                KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::Tab => {
                    menu.confirm_yes = !menu.confirm_yes;
                    ui_sfx::play(UiSfx::Move);
                }
                KeyCode::Enter | KeyCode::Space => {
                    if menu.confirm_yes {
                        self.confirm_menu_action();
                    } else if let Some(m) = &mut self.pause {
                        m.confirm = None;
                        ui_sfx::play(UiSfx::Back);
                    }
                }
                _ => {}
            }
        } else if menu.page == Page::Settings
            && menu.focus >= NAV_COUNT
            && key != KeyCode::Escape
        {
            let outcome = crate::settings_menu::key(&mut menu.settings, key);
            if outcome == crate::settings_menu::Outcome::Leave {
                menu.focus = SETTINGS_NAV;
            }
            self.settings_outcome(outcome);
            return;
        } else {
            let controls = match menu.page {
                Page::Save | Page::Load => 9,
                Page::History => 2,
                Page::Settings => 1,
                Page::Inventory => 1,
                Page::Equipment => equipment_controls,
                _ => 0,
            };
            let focus_before = menu.focus;
            match key {
                KeyCode::Escape => self.resume_pause(),
                KeyCode::Tab | KeyCode::ArrowDown => {
                    menu.focus = (menu.focus + 1) % (NAV_COUNT + controls)
                }
                KeyCode::ArrowUp => {
                    menu.focus = (menu.focus + NAV_COUNT + controls - 1) % (NAV_COUNT + controls)
                }
                KeyCode::ArrowRight if controls > 0 => {
                    menu.focus = if menu.focus < NAV_COUNT {
                        NAV_COUNT
                    } else {
                        (menu.focus + 1 - NAV_COUNT) % controls + NAV_COUNT
                    }
                }
                KeyCode::ArrowLeft => {
                    menu.focus = if menu.focus > NAV_COUNT {
                        menu.focus - 1
                    } else {
                        0
                    }
                }
                KeyCode::PageUp | KeyCode::PageDown => self.menu_scroll(key == KeyCode::PageUp),
                KeyCode::Enter | KeyCode::Space => {
                    let selected = menu.focus;
                    if selected < NAV_COUNT {
                        if selected != 0 {
                            ui_sfx::play(UiSfx::Select);
                        }
                        self.activate_menu_item(selected);
                    } else {
                        ui_sfx::play(UiSfx::Select);
                        self.menu_control(selected - NAV_COUNT);
                    }
                }
                _ => {}
            }
            if self
                .pause
                .as_ref()
                .is_some_and(|m| m.focus != focus_before && m.confirm.is_none())
            {
                ui_sfx::play(UiSfx::Move);
            }
        }
        // The slot list follows the focus so the candidates match it.
        if let Some(menu) = &mut self.pause {
            if menu.page == Page::Equipment && (NAV_COUNT..NAV_COUNT + 3).contains(&menu.focus) {
                menu.bp.slot = menu.focus - NAV_COUNT;
            }
        }
        self.needs_paint = true;
    }

    pub fn menu_scroll(&mut self, up: bool) {
        if let Some(menu) = &mut self.pause {
            if menu.confirm.is_some() {
                return;
            }
            match menu.page {
                Page::History => {
                    let max = menu
                        .wrapped_history
                        .len()
                        .saturating_sub(HISTORY_VISIBLE_LINES);
                    menu.history_offset = if up {
                        (menu.history_offset + 3).min(max)
                    } else {
                        menu.history_offset.saturating_sub(3)
                    };
                }
                Page::Save | Page::Load => {
                    menu.slot_page = if up { 0 } else { 1 };
                }
                _ => {}
            }
        }
        self.needs_paint = true;
    }

    pub fn menu_click(&mut self, x: i32, y: i32) {
        let Some(menu) = &mut self.pause else {
            return;
        };
        if menu.confirm.is_some() {
            if rect_contains(CANCEL, x, y) {
                menu.confirm = None;
                ui_sfx::play(UiSfx::Back);
            } else if rect_contains(ACCEPT, x, y) {
                self.confirm_menu_action();
            }
            self.needs_paint = true;
            return;
        }
        for index in 0..NAV_COUNT {
            if rect_contains(nav_rect(index), x, y) {
                menu.focus = index;
                if index != 0 {
                    ui_sfx::play(UiSfx::Select);
                }
                self.activate_menu_item(index);
                self.needs_paint = true;
                return;
            }
        }
        let page = menu.page;
        if page == Page::Inventory || page == Page::Equipment {
            let hit = if page == Page::Inventory {
                self.backpack_click(x, y)
            } else {
                self.equipment_click(x, y)
            };
            if hit {
                if let Some(m) = &mut self.pause {
                    if page == Page::Inventory {
                        m.focus = NAV_COUNT;
                    }
                }
            }
            self.needs_paint = true;
            return;
        }
        let Some(menu) = &mut self.pause else {
            return;
        };
        let rects: Vec<(i32, i32, i32, i32)> = match menu.page {
            Page::Save | Page::Load => (0..6)
                .map(card_rect)
                .chain([AUTO, PREVIOUS, NEXT])
                .collect(),
            Page::History => vec![PREVIOUS, NEXT],
            Page::Settings => {
                if let Some(outcome) =
                    crate::settings_menu::click(&mut menu.settings, SETTINGS_AREA, x, y)
                {
                    menu.focus = NAV_COUNT;
                    self.settings_outcome(outcome);
                }
                return;
            }
            _ => vec![],
        };
        if let Some(index) = rects.iter().position(|r| rect_contains(*r, x, y)) {
            menu.focus = NAV_COUNT + index;
            ui_sfx::play(UiSfx::Select);
            self.menu_control(index);
        }
    }

    pub fn paint_menu_trigger(&mut self) {
        if self.ui_hidden {
            return;
        }
        if let Some(fonts) = &self.fonts {
            let language = self.localizer.language();
            let (key, label) = if self.screen == Screen::Resume {
                ("F9", tr(language, "Cargar", "Load"))
            } else {
                ("ESC", tr(language, "Menú", "Menu"))
            };
            paint_trigger(
                &mut self.canvas,
                fonts,
                (TRIGGER.0, TRIGGER.1),
                key,
                label,
                rect_contains(TRIGGER, self.mouse.0, self.mouse.1),
                false,
            );
        }
    }

    pub fn paint_game_menu(&mut self) {
        let motion = !self.reduced_motion;
        let generation = self.input_generation;
        // Between the window's events, once everything has come in, only what moves is painted
        // again (the band down the left, the header's seal); the rest of the frame stays as it
        // was painted. A full paint at least every second keeps anything else honest.
        if motion {
            if let (Some(menu), Some(fonts)) = (self.pause.as_ref(), self.fonts.as_ref()) {
                let menu_age = menu.opened.elapsed().as_secs_f32();
                let settled = menu_age > ENTER + 0.8
                    && menu.page_since.elapsed().as_secs_f32() > 1.5
                    && !(menu.page == Page::Settings && menu.settings.animating());
                let fresh = menu.painted.is_some_and(|(at, painted_after)| {
                    painted_after == generation && at.elapsed().as_secs_f32() < 1.0
                });
                if settled && fresh && menu.backdrop.len() == self.canvas.pixels.len() {
                    if menu.confirm.is_none() {
                        let language = self.localizer.language();
                        paint_moving(
                            &mut self.canvas,
                            fonts,
                            language,
                            menu,
                            self.mouse,
                            menu_age,
                        );
                    }
                    return;
                }
            }
        }
        let stats = self.game.as_mut().and_then(|g| g.stats().ok());
        let inventory_gold = self.pause_inventory_gold;
        let inventory_items = &self.pause_inventory_items;
        // ---- the still layer: the scene darkened, the band in its colour and the pane. While
        // the menu comes in it fades up over the scene; after that it is kept and only copied.
        if let Some(menu) = self.pause.as_mut() {
            let t = if motion {
                (menu.opened.elapsed().as_secs_f32() / ENTER).min(1.0)
            } else {
                1.0
            };
            if t >= 1.0 && menu.backdrop.len() == self.canvas.pixels.len() {
                self.canvas.pixels.copy_from_slice(&menu.backdrop);
            } else {
                if menu.background.len() != self.canvas.pixels.len() {
                    // The window changed size under the menu: its frozen world follows.
                    self.canvas.restore(&menu.background);
                    menu.background.clone_from(&self.canvas.pixels);
                }
                self.canvas.restore(&menu.background);
                paint_backdrop(&mut self.canvas, menu.light, t);
                if t >= 1.0 {
                    menu.backdrop.clone_from(&self.canvas.pixels);
                } else {
                    let k = ui::ease_out(t);
                    for (pixel, under) in self.canvas.pixels.iter_mut().zip(&menu.background) {
                        *pixel = lerp_rgb(*under, *pixel, k);
                    }
                }
            }
        }
        let Some(menu) = &self.pause else {
            return;
        };
        let Some(fonts) = &self.fonts else {
            return;
        };
        let language = self.localizer.language();
        let t = |es, en| tr(language, es, en);
        // The scene's light tints the panes; made vivid, it is the menu's accent.
        let light = menu.light;
        let accent = ui::accent(light);
        let canvas = &mut self.canvas;
        // The menu's clock. Entrances count from `opened` and `page_since`; with reduced motion
        // everything is already in place and nothing moves.
        let menu_age = if motion {
            menu.opened.elapsed().as_secs_f32()
        } else {
            60.0
        };
        let page_age = if motion {
            menu.page_since.elapsed().as_secs_f32()
        } else {
            60.0
        };
        let clock = if motion { menu_age } else { 2.4 };

        // ---- the band down the left
        paint_sidebar(
            canvas, fonts, language, menu, self.mouse, menu_age, clock, motion,
        );

        // ---- the page's title on a plate of the accent, the act under it, a seal turning beside
        let (title, page_icon) = match menu.page {
            Page::Save => (t("Guardar partida", "Save game"), Icon::Disk),
            Page::Load => (t("Cargar partida", "Load game"), Icon::Folder),
            Page::Inventory => (t("Mochila", "Backpack"), Icon::Backpack),
            Page::Equipment => (t("Equipo", "Equipment"), Icon::Wardrobe),
            Page::History => (t("Historial de diálogo", "Dialogue history"), Icon::Council),
            Page::Settings => (t("Ajustes", "Settings"), Icon::Gear),
            Page::Status => (t("Ero status", "Ero status"), Icon::Heart),
        };
        let title_w = fonts.display_width(title, 44.0) as f32;
        ui::plate(canvas, 484.0, 76.0, title_w + 136.0, 68.0, 24.0, 0xb4000000);
        ui::plate(
            canvas,
            474.0,
            66.0,
            title_w + 136.0,
            68.0,
            24.0,
            ui::argb(0xff, accent),
        );
        ui::icon_disc(canvas, page_icon, 524, 100, 24, ui::BONE, false);
        // White words with a black drop on a deep accent, black words on a bright one.
        let title_ink = ui::ink_on(accent);
        if title_ink == 0xffff_ffff {
            canvas.display_text(fonts, title, 566 + 3, 70 + 3, 44.0, 0xff000000);
        }
        canvas.display_text(fonts, title, 566, 70, 44.0, title_ink);
        let chapter = if menu.origin == Screen::Explore {
            t("ACTO I · EXPLORACIÓN 3D", "ACT I · 3D EXPLORATION")
        } else {
            t("PRÓLOGO · NOVELA VISUAL", "PROLOGUE · VISUAL NOVEL")
        };
        ui::chip(canvas, fonts, 560, 150, chapter, GOLD & 0x00ff_ffff);
        canvas.rect(482, 202, 1354, 2, ui::argb(0x80, accent));
        paint_header_seal(canvas, clock, accent);
        let focused = |index, rect| {
            menu.focus == NAV_COUNT + index || rect_contains(rect, self.mouse.0, self.mouse.1)
        };
        // When the menu opens, its first page waits for the slash that brings it in.
        let opening = menu
            .page_since
            .saturating_duration_since(menu.opened)
            .as_secs_f32()
            < 0.05;
        let wipe_age = page_age - if opening { 0.12 } else { 0.0 };
        match menu.page {
            _ if wipe_age <= 0.0 => {}
            Page::Save | Page::Load => {
                for local in 0..6 {
                    let index = menu.slot_page * 6 + local;
                    let rect = card_rect(local);
                    let (x, y, w, _) = rect;
                    let slot = menu.slots.get(index).and_then(Option::as_ref);
                    let hot = focused(local, rect);
                    ui::glass(canvas, rect, 20, light, if hot { 1.0 } else { 0.0 });
                    canvas.smooth_disc(
                        (x + 38) as f32,
                        (y + 27) as f32,
                        14.0,
                        ui::argb(0x40, GOLD),
                    );
                    canvas.smooth_ring(
                        (x + 38) as f32,
                        (y + 27) as f32,
                        13.0,
                        1.4,
                        ui::argb(0xcc, GOLD),
                    );
                    ui::text_center(
                        canvas,
                        fonts,
                        &format!("{:02}", index + 1),
                        x + 38,
                        y + 27,
                        15.0,
                        GOLD,
                        true,
                    );
                    ui::text_mid(
                        canvas,
                        fonts,
                        &t("RANURA", "SLOT"),
                        x + 62,
                        y + 27,
                        16.0,
                        MUTED,
                        true,
                    );
                    if let Some(slot) = slot {
                        let kind = if slot.is_world { "3D" } else { "VN" };
                        let kw = fonts.text_width(kind, 15.0, true) + 22;
                        canvas.round_rect(
                            x + w - kw - 22,
                            y + 13,
                            kw,
                            28,
                            14,
                            ui::argb(0x33, light),
                            ui::argb(0x33, light),
                        );
                        ui::text_center(
                            canvas,
                            fonts,
                            kind,
                            x + w - kw / 2 - 22,
                            y + 27,
                            15.0,
                            ui::mix(light, 0xffffff, 0.4),
                            true,
                        );
                        if let Some(thumb) = menu.thumbs.get(index).and_then(Option::as_ref) {
                            canvas.blit_rgba(x + 24, y + 48, thumb);
                        } else {
                            canvas.rect(x + 24, y + 48, 368, 207, 0xff18131f);
                        }
                        canvas.round_outline(
                            x + 22,
                            y + 46,
                            372,
                            211,
                            6,
                            2.0,
                            if hot {
                                ui::argb(0xe6, ui::mix(light, 0xffffff, 0.2))
                            } else {
                                0x30ffffff
                            },
                        );
                        if slot.error.is_some() {
                            canvas.rect(x + 24, y + 48, 368, 207, 0xb0140a12);
                            centered_in_rect(
                                canvas,
                                fonts,
                                t("Guardado incompatible", "Unreadable save"),
                                (x + 24, y + 78, 368, 140),
                                23.0,
                                0xffffb9b4,
                                true,
                            );
                        }
                        let name = slot
                            .scene
                            .as_ref()
                            .map(|v| {
                                let mut localized = v.clone();
                                self.localizer.localize_scene(&mut localized);
                                localized.room_name
                            })
                            .unwrap_or_default();
                        ui::text_mid(
                            canvas,
                            fonts,
                            &ellipsize(fonts, &name, 250, 19.0, false),
                            x + 26,
                            y + 279,
                            19.0,
                            INK,
                            false,
                        );
                        ui::text_right(
                            canvas,
                            fonts,
                            &age(slot.saved_at, language),
                            x + w - 26,
                            y + 279,
                            17.0,
                            MUTED,
                            false,
                        );
                    } else {
                        canvas.round_rect(x + 24, y + 48, 368, 207, 12, 0x66120b19, 0x66100916);
                        canvas.round_outline(x + 24, y + 48, 368, 207, 12, 1.2, 0x26ffffff);
                        let plus = if menu.page == Page::Save {
                            Icon::Disk
                        } else {
                            Icon::Folder
                        };
                        ui::icon(canvas, plus, x + 208, y + 124, 46, 0x40ffffff, 0x00000000);
                        ui::text_center(
                            canvas,
                            fonts,
                            t("Ranura vacía", "Empty slot"),
                            x + 208,
                            y + 190,
                            22.0,
                            MUTED,
                            false,
                        );
                        ui::text_mid(
                            canvas,
                            fonts,
                            if menu.page == Page::Save {
                                t("Seleccionar para guardar", "Select to save")
                            } else {
                                t("Sin partida guardada", "No saved game")
                            },
                            x + 26,
                            y + 279,
                            17.0,
                            MUTED,
                            false,
                        );
                    }
                }
                if menu.page == Page::Load {
                    let valid = menu
                        .slots
                        .get(game::MANUAL_SLOTS)
                        .and_then(Option::as_ref)
                        .is_some_and(|s| s.error.is_none());
                    ui::pill(
                        canvas,
                        fonts,
                        AUTO,
                        t("Cargar autoguardado", "Load autosave"),
                        focused(6, AUTO),
                        valid,
                        light,
                        false,
                    );
                }
                // Page dots between the arrows.
                for dot in 0..2 {
                    let on = menu.slot_page == dot;
                    canvas.smooth_disc(
                        (1362 + dot as i32 * 20) as f32,
                        926.0,
                        if on { 6.0 } else { 4.0 },
                        if on { ui::argb(0xff, GOLD) } else { 0x55ffffff },
                    );
                }
                ui::text_right(
                    canvas,
                    fonts,
                    &format!("{} {} / 2", t("Página", "Page"), menu.slot_page + 1),
                    1520,
                    926,
                    21.0,
                    MUTED,
                    false,
                );
                ui::pill(
                    canvas,
                    fonts,
                    PREVIOUS,
                    "‹",
                    focused(7, PREVIOUS),
                    menu.slot_page > 0,
                    light,
                    false,
                );
                ui::pill(
                    canvas,
                    fonts,
                    NEXT,
                    "›",
                    focused(8, NEXT),
                    menu.slot_page < 1,
                    light,
                    false,
                );
            }
            Page::Inventory | Page::Equipment => {
                let view = bpm::View {
                    fonts,
                    language,
                    items: inventory_items,
                    gold: inventory_gold,
                    pack: self.pause_backpack.as_ref(),
                    ui: &menu.bp,
                    focused: menu.focus >= NAV_COUNT,
                    focus_index: menu.focus.saturating_sub(NAV_COUNT),
                    mouse: self.mouse,
                    light,
                    craft: self.pause_backpack_craft,
                    vitality: self.pause_vitality,
                };
                if menu.page == Page::Inventory {
                    bpm::paint_backpack(canvas, &view);
                } else {
                    bpm::paint_equipment(canvas, &view);
                }
            }
            Page::Status => {
                let room = self
                    .view
                    .as_ref()
                    .map(|v| v.room_name.as_str())
                    .unwrap_or(t("Sin partida activa", "No active game"));
                paint_status(
                    canvas,
                    fonts,
                    language,
                    &StatusView {
                        story: &self.pause_story,
                        stats,
                        room,
                        portrait: menu.portrait.as_ref(),
                        accent,
                    },
                    page_age,
                );
            }
            Page::History => {
                let end = menu
                    .wrapped_history
                    .len()
                    .saturating_sub(menu.history_offset);
                let start = end.saturating_sub(HISTORY_VISIBLE_LINES);
                ui::glass(canvas, (480, 218, 1356, 664), 22, light, 0.0);
                if menu.history.is_empty() {
                    ui::text_mid(
                        canvas,
                        fonts,
                        t("Todavía no hay diálogos.", "No dialogue yet."),
                        510,
                        276,
                        26.0,
                        MUTED,
                        false,
                    );
                }
                for (index, line) in menu.wrapped_history[start..end].iter().enumerate() {
                    let cy = 258 + index as i32 * 34;
                    if line.speaker {
                        // Speakers wear a pill of their own.
                        let w = fonts.text_width(&line.text, 19.0, true) + 30;
                        canvas.round_rect(
                            508,
                            cy - 14,
                            w,
                            28,
                            14,
                            ui::argb(0x3a, GOLD),
                            ui::argb(0x3a, GOLD),
                        );
                        ui::text_center(
                            canvas,
                            fonts,
                            &line.text,
                            508 + w / 2,
                            cy,
                            19.0,
                            GOLD,
                            true,
                        );
                    } else {
                        ui::text_mid(canvas, fonts, &line.text, 512, cy, 24.0, INK, false);
                    }
                }
                // Scrollbar on the right of the pane.
                let track = (1800, 240, 8, 612);
                canvas.round_rect(
                    track.0, track.1, track.2, track.3, 4, 0x33ffffff, 0x33ffffff,
                );
                let total = menu.wrapped_history.len().max(1);
                let visible = HISTORY_VISIBLE_LINES.min(total);
                let thumb_h =
                    ((track.3 as usize * visible) / total).clamp(36, track.3 as usize) as i32;
                let travel = track.3 - thumb_h;
                let top_fraction = if total > visible {
                    start as f32 / (total - visible) as f32
                } else {
                    0.0
                };
                canvas.round_rect(
                    track.0,
                    track.1 + (travel as f32 * top_fraction).round() as i32,
                    track.2,
                    thumb_h,
                    4,
                    ui::argb(0xcc, light),
                    ui::argb(0xcc, light),
                );
                ui::text_right(
                    canvas,
                    fonts,
                    &format!(
                        "{}–{} / {}",
                        start + usize::from(end > 0),
                        end,
                        menu.wrapped_history.len()
                    ),
                    1520,
                    926,
                    19.0,
                    MUTED,
                    false,
                );
                ui::text_mid(
                    canvas,
                    fonts,
                    t(
                        "Consulta sin retroceder ni cambiar decisiones.",
                        "Read without rolling back or changing decisions.",
                    ),
                    490,
                    926,
                    21.0,
                    MUTED,
                    false,
                );
                ui::pill(
                    canvas,
                    fonts,
                    PREVIOUS,
                    "‹",
                    focused(0, PREVIOUS),
                    start > 0,
                    light,
                    false,
                );
                ui::pill(
                    canvas,
                    fonts,
                    NEXT,
                    "›",
                    focused(1, NEXT),
                    menu.history_offset > 0,
                    light,
                    false,
                );
            }
            Page::Settings => {
                crate::settings_menu::paint(
                    canvas,
                    fonts,
                    &menu.settings,
                    SETTINGS_AREA,
                    &crate::settings_menu::Shown {
                        language,
                        reduced_motion: !motion,
                        light,
                        mouse: self.mouse,
                        keyboard: menu.focus >= NAV_COUNT,
                    },
                );
            }
        }
        // ---- footer: a notice, an error, or the keys
        let footer_y = 972;
        if !menu.notice.is_empty() {
            ui::text_mid(
                canvas,
                fonts,
                &ellipsize(fonts, &menu.notice, 1340, 20.0, false),
                486,
                footer_y + 15,
                20.0,
                MUTED,
                false,
            );
        } else if let Some(error) = self.autosave_error.as_deref() {
            ui::text_mid(
                canvas,
                fonts,
                &ellipsize(fonts, error, 1340, 20.0, false),
                486,
                footer_y + 15,
                20.0,
                0xffffb9b4,
                false,
            );
        } else {
            ui::key_hints(
                canvas,
                fonts,
                486,
                footer_y,
                &[
                    ("TAB", t("navegar", "navigate")),
                    ("←↑↓→", t("moverse", "move")),
                    ("ENTER", t("seleccionar", "select")),
                    ("ESC", t("volver", "back")),
                ],
            );
        }
        // ---- a page comes in behind a slash of the accent sweeping the pane
        if wipe_age < WIPE {
            let shade = if opening {
                (ui::entrance(menu_age, 0.0, ENTER) * 1.6).min(1.0)
            } else {
                1.0
            };
            paint_wipe(canvas, light, ui::ease_out(wipe_age.max(0.0) / WIPE), shade);
        }
        if let Some(confirm) = menu.confirm {
            canvas.rect(0, 0, 1920, 1080, 0xd0080610);
            let (question, modal_icon, danger) = match confirm {
                Confirm::Save(_) => (
                    t("¿Reemplazar esta partida?", "Overwrite this save?"),
                    Icon::Disk,
                    true,
                ),
                Confirm::Load(_) => (
                    t("¿Cargar esta partida?", "Load this save?"),
                    Icon::Folder,
                    false,
                ),
                Confirm::Title => (
                    t("¿Volver al menú principal?", "Return to the main menu?"),
                    Icon::House,
                    true,
                ),
                Confirm::Quit => (t("¿Salir del juego?", "Quit the game?"), Icon::Power, true),
            };
            let tint = if danger { DANGER & 0x00ff_ffff } else { light };
            ui::glass(canvas, (550, 358, 820, 362), 28, tint, 0.6);
            ui::icon_disc(canvas, modal_icon, 640, 440, 40, ui::accent(tint), true);
            canvas.display_text(fonts, question, 704, 410, 36.0, INK);
            let detail = match confirm {
                Confirm::Save(_) => t(
                    "Se conserva una copia del guardado anterior.",
                    "A backup of the previous slot will be kept.",
                ),
                Confirm::Load(_) => t(
                    "Sustituye tu sesión actual. Guarda antes si quieres conservarla.",
                    "Replaces your current session. Save first if you want to keep it.",
                ),
                _ => t(
                    "Se conservará el autoguardado antes de salir.",
                    "Your autosave will be kept before leaving.",
                ),
            };
            canvas.paragraph(fonts, detail, 704, 476, 620, 34, 23.0, MUTED, false);
            ui::pill(
                canvas,
                fonts,
                CANCEL,
                t("Cancelar", "Cancel"),
                !menu.confirm_yes || rect_contains(CANCEL, self.mouse.0, self.mouse.1),
                true,
                light,
                false,
            );
            ui::pill(
                canvas,
                fonts,
                ACCEPT,
                t("Confirmar", "Confirm"),
                menu.confirm_yes || rect_contains(ACCEPT, self.mouse.0, self.mouse.1),
                true,
                light,
                danger,
            );
        }
        if let Some(menu) = self.pause.as_mut() {
            menu.painted = Some((Instant::now(), generation));
        }
    }
}

/// How wide the band down the left is painted (its shadow and hairline in).
const SIDEBAR_W: usize = 458;
/// Where the header's seal turns (x0, y0, x1, y1).
const SEAL_BOX: (usize, usize, usize, usize) = (1726, 60, 1854, 188);

/// The band down the left: embers rising, a seal turning at its foot, the title, the buttons
/// (coming in one after another while the menu opens) and the way back.
#[allow(clippy::too_many_arguments)]
fn paint_sidebar(
    canvas: &mut Canvas,
    fonts: &Fonts,
    language: Language,
    menu: &PauseMenu,
    mouse: (i32, i32),
    menu_age: f32,
    clock: f32,
    motion: bool,
) {
    let t = |es, en| tr(language, es, en);
    let accent = ui::accent(menu.light);
    let slide = 470.0 * (1.0 - ui::entrance(menu_age, 0.0, ENTER));
    let sx = -(slide.round() as i32);
    paint_sigil(
        canvas,
        206.0 - slide,
        1004.0,
        172.0,
        clock * 0.10,
        0x46000000,
    );
    paint_embers(canvas, clock, slide, accent);
    canvas.brand_text(fonts, "Demon Queen", 64 + sx + 4, 62 + 4, 48.0, 0xff000000);
    canvas.brand_text(fonts, "Demon Queen", 64 + sx, 62, 48.0, 0xffffffff);
    ui::tag(
        canvas,
        fonts,
        70 + sx,
        140,
        t("PARTIDA EN PAUSA", "GAME PAUSED"),
        15.0,
        ui::ABYSS,
        ui::BONE,
    );
    canvas.line(
        (70.0 - slide, 202.0),
        (384.0 - slide, 194.0),
        2.0,
        0xccffffff,
    );
    let labels = [
        t("Continuar", "Resume"),
        t("Guardar", "Save"),
        t("Cargar", "Load"),
        t("Mochila", "Backpack"),
        t("Equipo", "Equipment"),
        t("Estado / Ero status", "Status / Ero status"),
        t("Historial", "History"),
        t("Ajustes", "Settings"),
        t("Menú principal", "Main menu"),
        t("Salir del juego", "Quit game"),
    ];
    let nav_icons = [
        Icon::Play,
        Icon::Disk,
        Icon::Folder,
        Icon::Backpack,
        Icon::Wardrobe,
        Icon::Heart,
        Icon::Council,
        Icon::Gear,
        Icon::House,
        Icon::Power,
    ];
    let active = matches!(menu.origin, Screen::Play | Screen::Explore | Screen::End);
    let current_index = match menu.page {
        Page::Save => 1,
        Page::Load => 2,
        Page::Inventory => 3,
        Page::Equipment => 4,
        Page::Status => 5,
        Page::History => 6,
        Page::Settings => 7,
    };
    for (index, label) in labels.iter().enumerate() {
        let rect = nav_rect(index);
        let enabled = index != 1 || active;
        let look = NavLook {
            enabled,
            selected: enabled && (menu.focus == index || rect_contains(rect, mouse.0, mouse.1)),
            current: current_index == index,
            danger: index == 9,
            accent,
        };
        // The buttons come in one after another from the left.
        let come = ui::entrance(menu_age, 0.06 + index as f32 * 0.028, 0.26);
        let dx = -((rect.0 + rect.2 + 80) as f32) * (1.0 - come);
        paint_nav_plate(
            canvas,
            fonts,
            rect,
            dx,
            label,
            nav_icons[index],
            look,
            clock,
            motion,
        );
    }
    ui::key_hints(
        canvas,
        fonts,
        76 + sx,
        920,
        &[("ESC", t("volver", "resume"))],
    );
    ui::text_mid(
        canvas,
        fonts,
        t("Sin cerrar la partida", "Your session stays open"),
        78 + sx,
        978,
        17.0,
        0xffffffff,
        false,
    );
}

/// The seal turning at the header's right, in the accent.
fn paint_header_seal(canvas: &mut Canvas, clock: f32, accent: u32) {
    paint_sigil(
        canvas,
        1790.0,
        124.0,
        56.0,
        -clock * 0.22,
        ui::argb(0x90, accent),
    );
}

/// Between events: only what moves, over the still layer — the band down the left and the
/// header's seal. Everything else on the canvas is the last full paint.
fn paint_moving(
    canvas: &mut Canvas,
    fonts: &Fonts,
    language: Language,
    menu: &PauseMenu,
    mouse: (i32, i32),
    menu_age: f32,
) {
    // In the canvas's own pixels (`k` per layout pixel).
    let (w, k) = (canvas.pw, canvas.k);
    let px = |v: usize| ((v as f32 * k).round() as usize).min(w);
    let sidebar = px(SIDEBAR_W);
    for y in 0..canvas.ph {
        let row = y * w;
        canvas.pixels[row..row + sidebar].copy_from_slice(&menu.backdrop[row..row + sidebar]);
    }
    let (x0, y0, x1, y1) = SEAL_BOX;
    let (x0, x1) = (px(x0), px(x1));
    for y in px(y0)..px(y1).min(canvas.ph) {
        let row = y * w;
        canvas.pixels[row + x0..row + x1].copy_from_slice(&menu.backdrop[row + x0..row + x1]);
    }
    paint_sidebar(
        canvas, fonts, language, menu, mouse, menu_age, menu_age, true,
    );
    paint_header_seal(canvas, menu_age, ui::accent(menu.light));
}

/// The band down the left (behind the menu's buttons), `slide` px off to the left.
fn band_points(slide: f32) -> [(f32, f32); 4] {
    [
        (-slide, 0.0),
        (436.0 - slide, 0.0),
        (398.0 - slide, 1080.0),
        (-slide, 1080.0),
    ]
}

/// The still layer under the menu, `t` (0..1) into its entrance: the scene darkened, the band
/// down the left (sliding in) in a deep shade of the accent with its slash and halftone, and
/// the page's pane.
fn paint_backdrop(canvas: &mut Canvas, light: u32, t: f32) {
    let accent = ui::accent(light);
    canvas.rect(
        0,
        0,
        1920,
        1080,
        ui::argb(0xd0, ui::mix(0x0b0710, light, 0.06)),
    );
    canvas.vertical_scrim(0, 0, 1920, 360, light, 0.08, 0.0);
    let slide = 470.0 * (1.0 - ui::ease_out(t));
    let band = band_points(slide);
    canvas.convex(&ui::moved(&band, 12.0, 0.0), 0xa0000000);
    // As deep as the crimson band's wine was, and never so bright white words fade on it.
    canvas.convex(&band, ui::argb(0xf0, ui::deepen(accent, 0.68, 0.14)));
    // A lighter slash crossing it under the buttons, and halftone darkening its foot.
    let edge = |y: f32| 436.0 - 38.0 * y / 1080.0 - slide;
    canvas.convex(
        &[
            (-slide, 880.0),
            (edge(560.0), 560.0),
            (edge(650.0), 650.0),
            (-slide, 980.0),
        ],
        ui::argb(0x66, accent),
    );
    ui::halftone(
        canvas,
        (-slide, 600.0, 400.0, 480.0),
        15.0,
        5.4,
        (-slide, 1080.0),
        430.0,
        0x55000000,
    );
    canvas.line(
        (442.0 - slide, 0.0),
        (404.0 - slide, 1080.0),
        2.0,
        0xccffffff,
    );
    ui::glass(canvas, PANE, 26, light, 0.0);
}

/// `a` towards `b` by `k` (0..1), per channel; the result is opaque.
fn lerp_rgb(a: u32, b: u32, k: f32) -> u32 {
    let w = (k.clamp(0.0, 1.0) * 256.0) as u32;
    let ch = |s: u32| ((((a >> s) & 0xff) * (256 - w) + ((b >> s) & 0xff) * w) >> 8) << s;
    0xff00_0000 | ch(16) | ch(8) | ch(0)
}

/// A number in 0..1 that `n` always gives the same.
fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9e37_79b1) ^ 0x5bd1_e995;
    x ^= x >> 15;
    x = x.wrapping_mul(0x85eb_ca77);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae3d);
    x ^= x >> 16;
    (x >> 8) as f32 / 16_777_216.0
}

/// Embers of the accent rising up the band, `clock` seconds in (`slide` while the band comes
/// in).
pub(crate) fn paint_embers(canvas: &mut Canvas, clock: f32, slide: f32, accent: u32) {
    let (glow, core) = (accent, ui::mix(accent, 0xffffff, 0.68));
    for i in 0..30u32 {
        let (r1, r2, r3, r4) = (
            hash01(i * 4),
            hash01(i * 4 + 1),
            hash01(i * 4 + 2),
            hash01(i * 4 + 3),
        );
        let speed = 36.0 + 64.0 * r1;
        let span = 1180.0;
        let y = 1100.0 - (clock * speed + r2 * span) % span;
        let sway = (clock * (0.6 + r4) + r1 * 6.3).sin() * 12.0;
        let x = 16.0 + 372.0 * r3 + sway - (1100.0 - y) * 0.03 - slide;
        let life = (y / 1080.0).clamp(0.0, 1.0);
        let size = 1.2 + 2.0 * r4;
        canvas.smooth_disc(x, y, size * 3.0, ui::argb((40.0 * life) as u32, glow));
        canvas.smooth_disc(x, y, size, ui::argb((225.0 * life) as u32, core));
    }
}

/// A demon's seal turning `angle` radians: a double ring, a five-pointed star and ticks.
pub(crate) fn paint_sigil(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, angle: f32, color: u32) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    // Its rings as fine polygons: far cheaper than a disc's every pixel, and as smooth.
    let ring = |canvas: &mut Canvas, radius: f32, width: f32| {
        let n = ((radius * 0.45) as usize).clamp(24, 96);
        let at = |k: usize| {
            let a = k as f32 * TAU / n as f32;
            (cx + radius * a.cos(), cy + radius * a.sin())
        };
        for k in 0..n {
            canvas.line(at(k), at(k + 1), width, color);
        }
    };
    ring(canvas, r, 2.4);
    ring(canvas, r * 0.82, 1.4);
    let star: Vec<(f32, f32)> = (0..5)
        .map(|k| {
            let a = angle + k as f32 * TAU / 5.0 - FRAC_PI_2;
            (cx + r * 0.82 * a.cos(), cy + r * 0.82 * a.sin())
        })
        .collect();
    for k in 0..5 {
        canvas.line(star[k], star[(k + 2) % 5], 1.8, color);
    }
    for k in 0..30 {
        let a = -angle * 0.5 + k as f32 * TAU / 30.0;
        let (c, s) = (a.cos(), a.sin());
        let (r0, r1) = if k % 5 == 0 {
            (r * 0.85, r * 0.97)
        } else {
            (r * 0.88, r * 0.93)
        };
        canvas.line(
            (cx + c * r0, cy + s * r0),
            (cx + c * r1, cy + s * r1),
            1.6,
            color,
        );
    }
}

/// The part of a convex polygon between x = `lo` and x = `hi`.
fn clip_x(points: &[(f32, f32)], lo: f32, hi: f32) -> Vec<(f32, f32)> {
    let keep = |points: &[(f32, f32)], inside: &dyn Fn(f32) -> bool, edge: f32| {
        let mut out = Vec::with_capacity(points.len() + 2);
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            if inside(a.0) {
                out.push(a);
            }
            if inside(a.0) != inside(b.0) {
                let k = (edge - a.0) / (b.0 - a.0);
                out.push((edge, a.1 + (b.1 - a.1) * k));
            }
        }
        out
    };
    let left = keep(points, &|x| x >= lo, lo);
    keep(&left, &|x| x <= hi, hi)
}

/// The four corners of a stroke from `a` to `b`, `width` wide.
fn line_quad(a: (f32, f32), b: (f32, f32), width: f32) -> [(f32, f32); 4] {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (nx, ny) = (-dy / length * width * 0.5, dx / length * width * 0.5);
    [
        (a.0 + nx, a.1 + ny),
        (b.0 + nx, b.1 + ny),
        (b.0 - nx, b.1 - ny),
        (a.0 - nx, a.1 - ny),
    ]
}

/// The sweep that brings a page in, `k` (0..1) across the pane: the pane's dark ahead of a
/// slash of the accent with a white hairline. `shade` dims the dark while the menu itself
/// comes in.
fn paint_wipe(canvas: &mut Canvas, light: u32, k: f32, shade: f32) {
    let (x, y, w, h) = (PANE.0 as f32, PANE.1 as f32, PANE.2 as f32, PANE.3 as f32);
    let lean = 150.0;
    let front = x - lean + (w + lean * 2.0) * k;
    let (top, bottom) = (front + lean * 0.5, front - lean * 0.5);
    let ahead = clip_x(
        &[(top, y), (x + w, y), (x + w, y + h), (bottom, y + h)],
        x,
        x + w,
    );
    canvas.convex(
        &ahead,
        ui::argb((255.0 * shade) as u32, ui::mix(ui::ABYSS, light, 0.05)),
    );
    let slash = clip_x(
        &[
            (top - 34.0, y),
            (top, y),
            (bottom, y + h),
            (bottom - 34.0, y + h),
        ],
        x,
        x + w,
    );
    canvas.convex(&slash, ui::argb(0xff, ui::accent(light)));
    let hair = clip_x(
        &[
            (top + 6.0, y),
            (top + 10.0, y),
            (bottom + 10.0, y + h),
            (bottom + 6.0, y + h),
        ],
        x,
        x + w,
    );
    canvas.convex(&hair, 0xffffffff);
}

#[derive(Clone, Copy)]
struct NavLook {
    enabled: bool,
    selected: bool,
    current: bool,
    danger: bool,
    /// The menu's accent (0xRRGGBB).
    accent: u32,
}

/// One of the menu's buttons: a slanted black plate that turns bone-white with dark words in
/// focus, a glint of the accent running over it; the button of the page on show wears a wedge
/// of the accent. Focus only recolours it.
#[allow(clippy::too_many_arguments)]
fn paint_nav_plate(
    canvas: &mut Canvas,
    fonts: &Fonts,
    rect: (i32, i32, i32, i32),
    dx: f32,
    label: &str,
    icon: Icon,
    look: NavLook,
    clock: f32,
    motion: bool,
) {
    let (x, y, w, h) = (
        rect.0 as f32 + dx,
        rect.1 as f32,
        rect.2 as f32,
        rect.3 as f32,
    );
    let skew = 16.0;
    let accent = if look.danger {
        DANGER & 0x00ff_ffff
    } else {
        look.accent
    };
    ui::plate(canvas, x + 7.0, y + 7.0, w, h, skew, 0xa6000000);
    ui::plate(
        canvas,
        x,
        y,
        w,
        h,
        skew,
        if look.selected {
            ui::argb(0xff, ui::BONE)
        } else {
            0xf4120b17
        },
    );
    if look.selected && motion {
        let p = (clock % 1.8) / 1.8;
        let band = x - 100.0 + (w + 200.0) * p;
        let (lo, hi) = (x.max(band), (x + w).min(band + 64.0));
        if hi - lo > skew + 1.0 {
            ui::plate(canvas, lo, y, hi - lo, h, skew, ui::argb(0x30, accent));
        }
    }
    if look.current {
        ui::plate(
            canvas,
            x + w - 34.0,
            y,
            34.0,
            h,
            skew,
            ui::argb(0xff, accent),
        );
    }
    ui::outline(
        canvas,
        &ui::plate_points(x, y, w, h, skew),
        if look.selected { 2.0 } else { 1.0 },
        if look.selected {
            ui::argb(0xff, accent)
        } else {
            0x40ffffff
        },
    );
    let cy = rect.1 + rect.3 / 2;
    ui::icon_disc(
        canvas,
        icon,
        (x + 38.0).round() as i32,
        cy,
        19,
        accent,
        look.selected || look.current,
    );
    let color = if !look.enabled {
        ui::argb(0xff, ui::mix(MUTED, 0x000000, 0.35))
    } else if look.selected {
        if look.danger {
            0xffa3182f
        } else {
            0xff0c0811
        }
    } else {
        INK
    };
    let size = ui::fit_size(fonts, label, 200, 22.0, true);
    ui::text_mid(
        canvas,
        fonts,
        label,
        (x + 68.0).round() as i32,
        cy,
        size,
        color,
        true,
    );
}

/// The status art's name for what she wears: the royal dress has the plain files, the green dress
/// `_verde`, the bikini `_bikini` (the wardrobe's slots 0, 1 and 2).
fn outfit_suffix(outfit: u8) -> &'static str {
    match outfit {
        1 => "_verde",
        2 => "_bikini",
        _ => "",
    }
}

/// The Queen's illustration for the status page at lust rank `rank` in `outfit`
/// (`data/assets/ui/status/reina_status<_verde|_bikini>_<rank>.png`; else the royal dress's of the
/// rank — the last rank is nude in every outfit —, else `reina_status.png`), scaled to cover its
/// frame (kept to the top, where her face is) unless it already fits.
fn load_status_portrait(rank: usize, outfit: u8) -> Option<ArtBitmap> {
    let suffix = outfit_suffix(outfit);
    let art = Some(suffix)
        .filter(|s| !s.is_empty())
        .and_then(|s| {
            crate::load_art(
                &format!("ui/status/reina_status{s}_{rank}.png"),
                crate::ArtFit::Original,
            )
        })
        .or_else(|| {
            crate::load_art(
                &format!("ui/status/reina_status_{rank}.png"),
                crate::ArtFit::Original,
            )
        })
        .or_else(|| crate::load_art(PORTRAIT_FILE, crate::ArtFit::Original))?;
    let (w, h) = (PORTRAIT.2 as u32, PORTRAIT.3 as u32);
    if art.w as u32 == w && art.h as u32 == h {
        return Some(art);
    }
    let image = image::RgbaImage::from_raw(art.w as u32, art.h as u32, art.rgba)?;
    let scale = (w as f32 / image.width() as f32).max(h as f32 / image.height() as f32);
    let sw = ((image.width() as f32 * scale).ceil() as u32).max(w);
    let sh = ((image.height() as f32 * scale).ceil() as u32).max(h);
    let scaled = image::imageops::resize(&image, sw, sh, image::imageops::FilterType::Lanczos3);
    let fitted = image::imageops::crop_imm(&scaled, (sw - w) / 2, 0, w, h).to_image();
    Some(ArtBitmap {
        w: w as usize,
        h: h as usize,
        rgba: fitted.into_raw(),
    })
}

/// What the status page shows.
struct StatusView<'a> {
    story: &'a BTreeMap<String, i64>,
    stats: Option<game::Stats>,
    room: &'a str,
    portrait: Option<&'a ArtBitmap>,
    /// The menu's accent (0xRRGGBB).
    accent: u32,
}

/// The lust ranks: from how many points each starts, and its name.
const LUST_RANKS: [(i64, &str, &str); 6] = [
    (0, "Inocente", "Innocent"),
    (1, "Curiosa", "Curious"),
    (4, "Despierta", "Awakened"),
    (10, "Lasciva", "Lascivious"),
    (20, "Insaciable", "Insatiable"),
    (35, "Reina del deseo", "Queen of desire"),
];

/// Her lust points: each time alone and each climax one, each encounter two, the elixir and the
/// ring one more.
fn lust_points(story: &BTreeMap<String, i64>) -> i64 {
    use crate::exploration3d::ero;
    let count = |key: &str| story.get(key).copied().unwrap_or(0).max(0);
    count(ero::MASTURBACIONES)
        + count(ero::ORGASMOS)
        + 2 * count(ero::ENCUENTROS)
        + count(ero::FUTA)
        + count(ero::HIPNOTIZADA)
}

/// The rank `points` reach (0..=5).
fn lust_rank(points: i64) -> usize {
    LUST_RANKS
        .iter()
        .rposition(|(from, _, _)| points >= *from)
        .unwrap_or(0)
}

/// The status page: the Queen's illustration (or the frame waiting for it), her lust rank,
/// whether she is still a virgin, how many times she has done what, and the story's
/// attributes. Numbers count up as the page comes in.
fn paint_status(
    canvas: &mut Canvas,
    fonts: &Fonts,
    language: Language,
    view: &StatusView,
    page_age: f32,
) {
    use crate::exploration3d::ero;
    let t = |es, en| tr(language, es, en);
    let count = |key: &str| view.story.get(key).copied().unwrap_or(0).max(0);
    let shown =
        |value: i64, delay: f32| (value as f32 * ui::entrance(page_age, delay, 0.8)).round() as i64;
    paint_portrait(canvas, fonts, language, view, page_age);

    // ---- lust rank, and the way to the next one
    let points = lust_points(view.story);
    let rank = lust_rank(points);
    let (from, es, en) = LUST_RANKS[rank];
    let rx = 948;
    let accent = view.accent;
    ui::tag(
        canvas,
        fonts,
        rx,
        222,
        t("RANGO DE LUJURIA", "LUST RANK"),
        14.0,
        accent,
        ui::ink_on(accent),
    );
    canvas.display_text(fonts, t(es, en), rx + 3, 258 + 3, 50.0, 0xff000000);
    canvas.display_text(fonts, t(es, en), rx, 258, 50.0, 0xffffffff);
    for k in 0..5 {
        let (cx, cy) = (rx as f32 + 12.0 + k as f32 * 28.0, 346.0);
        canvas.convex(
            &[
                (cx, cy - 10.0),
                (cx + 10.0, cy),
                (cx, cy + 10.0),
                (cx - 10.0, cy),
            ],
            if k < rank { GOLD } else { 0x40ffffff },
        );
    }
    if let Some(&(next, next_es, next_en)) = LUST_RANKS.get(rank + 1) {
        let k = (points - from) as f32 / (next - from) as f32;
        ui::meter(
            canvas,
            rx + 160,
            338,
            360,
            16,
            k * ui::entrance(page_age, 0.2, 0.7),
            accent,
        );
        ui::text_mid(
            canvas,
            fonts,
            &format!("{points} / {next} · {}", t(next_es, next_en)),
            rx + 540,
            346,
            17.0,
            MUTED,
            false,
        );
    } else {
        ui::text_mid(
            canvas,
            fonts,
            &format!("{points} · {}", t("rango máximo", "top rank")),
            rx + 160,
            346,
            17.0,
            GOLD,
            true,
        );
    }

    // ---- virginity: a stamp that drops into place
    let virgin = count(ero::VIRGINIDAD_PERDIDA) == 0;
    let fall = 1.0 - ui::entrance(page_age, 0.45, 0.3);
    let (vx, vy) = (1566.0, 226.0 - 46.0 * fall);
    let (ground, ink, word) = if virgin {
        // Bone, inked in a deep shade of the accent.
        (
            ui::BONE,
            ui::argb(0xff, ui::deepen(accent, 0.72, 0.085)),
            t("INTACTA", "INTACT"),
        )
    } else {
        (accent, ui::ink_on(accent), t("PERDIDA", "LOST"))
    };
    ui::plate(canvas, vx + 8.0, vy + 8.0, 270.0, 104.0, 26.0, 0xb4000000);
    ui::plate(canvas, vx, vy, 270.0, 104.0, 26.0, ui::argb(0xff, ground));
    ui::outline(
        canvas,
        &ui::plate_points(vx + 8.0, vy + 7.0, 254.0, 90.0, 23.0),
        1.6,
        ink,
    );
    let (vxi, vyi) = (vx.round() as i32, vy.round() as i32);
    ui::text_center(
        canvas,
        fonts,
        t("VIRGINIDAD", "VIRGINITY"),
        vxi + 135,
        vyi + 30,
        15.0,
        ink,
        true,
    );
    // The stamp's word fits inside its frame in every language.
    let word_size = (40.0 * 230.0 / fonts.display_width(word, 40.0).max(1) as f32).min(40.0);
    let word_w = fonts.display_width(word, word_size);
    let word_y = vyi + 44 + ((40.0 - word_size) * 0.5).round() as i32;
    canvas.display_text(fonts, word, vxi + 135 - word_w / 2, word_y, word_size, ink);

    // ---- how many times
    let tiles = [
        (
            t("Masturbaciones", "Masturbation"),
            count(ero::MASTURBACIONES),
            Icon::Rest,
        ),
        (t("Orgasmos", "Orgasms"), count(ero::ORGASMOS), Icon::Heart),
        (
            t("Encuentros", "Encounters"),
            count(ero::ENCUENTROS),
            Icon::Tavern,
        ),
        (
            t("Con Nina", "With Nina"),
            count(&ero::with("nina")),
            Icon::Talk,
        ),
        (
            t("Con Alba", "With Alba"),
            count(&ero::with("alba")),
            Icon::Talk,
        ),
        (
            t("Con Noa", "With Noa"),
            count(&ero::with("noa")),
            Icon::Talk,
        ),
        (
            t("Con el elixir", "Under the elixir"),
            count(ero::FUTA),
            Icon::Rune,
        ),
        (
            t("Elixires bebidos", "Elixirs drunk"),
            count(ero::ELIXIR),
            Icon::Loot,
        ),
        (
            t("Hipnotizada", "Hypnotized"),
            count(ero::HIPNOTIZADA),
            Icon::Memory,
        ),
    ];
    for (i, (label, value, icon)) in tiles.into_iter().enumerate() {
        let (col, row) = ((i % 3) as i32, (i / 3) as i32);
        let delay = 0.10 + i as f32 * 0.035;
        let come = ui::entrance(page_age, delay, 0.3);
        ero_tile(
            canvas,
            fonts,
            (rx + col * 304, 380 + row * 134, 284, 118),
            44.0 * (1.0 - come),
            label,
            shown(value, delay + 0.05),
            value > 0,
            icon,
            accent,
        );
    }

    // ---- the story's attributes
    let (bx, by, bw, bh) = (rx as f32, 790.0, 892.0, 168.0);
    ui::plate(canvas, bx + 8.0, by + 8.0, bw, bh, 22.0, 0xa0000000);
    ui::plate(canvas, bx, by, bw, bh, 22.0, 0xf00c0811);
    ui::outline(
        canvas,
        &ui::plate_points(bx, by, bw, bh, 22.0),
        1.0,
        0x38ffffff,
    );
    ui::tag(
        canvas,
        fonts,
        rx + 30,
        774,
        t("ATRIBUTOS", "ATTRIBUTES"),
        14.0,
        ui::BONE,
        ui::ABYSS,
    );
    if let Some(stats) = view.stats {
        // The canon's («El Relevo»): what the court suspects, the seal's hunger, her dignity, the
        // maids' trust and her desire, each on its own scale (the most it reaches).
        for (i, (label, value, most, color)) in [
            (
                t("Sospecha", "Suspicion"),
                stats.suspicion,
                10,
                0xe8786a_u32,
            ),
            (
                t("Hambre del sello", "Seal's hunger"),
                stats.hunger,
                100,
                0xdd7ca8,
            ),
            (t("Dignidad", "Dignity"), stats.dignity, 20, 0xd9b67b),
            (t("Confianza", "Trust"), stats.maids_trust, 10, 0x8fc7e8),
            (t("Deseo", "Desire"), stats.desire, 50, 0xe0566b),
        ]
        .into_iter()
        .enumerate()
        {
            let x = rx + 44 + i as i32 * 170;
            let value = value.clamp(0, most);
            let delay = 0.3 + i as f32 * 0.05;
            ui::text_mid(
                canvas,
                fonts,
                &label.to_uppercase(),
                x,
                836,
                15.0,
                MUTED,
                true,
            );
            canvas.text(
                fonts,
                &shown(value, delay).to_string(),
                x,
                852,
                34.0,
                ui::argb(0xff, ui::mix(color, 0xffffff, 0.3)),
                true,
            );
            ui::meter(
                canvas,
                x,
                914,
                140,
                14,
                value as f32 / most as f32 * ui::entrance(page_age, delay, 0.6),
                color,
            );
        }
    } else {
        ui::text_mid(
            canvas,
            fonts,
            t(
                "La historia aún no ha empezado.",
                "The story has not started yet.",
            ),
            rx + 44,
            874,
            21.0,
            MUTED,
            false,
        );
    }
}

/// The Queen's illustration in a slanted frame of the accent (or, until the art exists, the
/// frame waiting for it), her name on a plate across its foot and the place she is in.
fn paint_portrait(
    canvas: &mut Canvas,
    fonts: &Fonts,
    language: Language,
    view: &StatusView,
    page_age: f32,
) {
    let t = |es, en| tr(language, es, en);
    let accent = view.accent;
    let (px, py, pw, ph) = PORTRAIT;
    let dx = -60.0 * (1.0 - ui::entrance(page_age, 0.04, 0.34));
    let (x, y, w, h) = (px as f32 + dx, py as f32, pw as f32, ph as f32);
    let cut = 34.0;
    let frame = ui::cut_points(x, y, w, h, cut);
    canvas.convex(&ui::moved(&frame, 12.0, 12.0), 0xb4000000);
    canvas.convex(&frame, 0xff150d1b);
    let xi = x.round() as i32;
    // Behind her (her art is cut out), or waiting for her: a glow of the accent, its halftone and
    // fine diagonal hatching.
    canvas.smooth_disc(x + w * 0.5, y + h * 0.4, w * 0.62, ui::argb(0x1c, accent));
    canvas.smooth_disc(x + w * 0.5, y + h * 0.36, w * 0.38, ui::argb(0x1c, accent));
    ui::halftone(
        canvas,
        (x + 10.0, y + 10.0, w - 20.0, h - 20.0),
        16.0,
        4.6,
        (x + w * 0.5, y + h * 0.36),
        h * 0.62,
        ui::argb(0x30, accent),
    );
    let mut k = -h * 0.6;
    while k < w {
        let stroke = line_quad((x + k, y + h), (x + k + h * 0.6, y), 1.2);
        canvas.convex(&clip_x(&stroke, x + 2.0, x + w - 2.0), 0x10ffffff);
        k += 28.0;
    }
    if let Some(art) = view.portrait {
        canvas.blit_rgba(xi, py, art);
        // Its corners cut like the frame's.
        canvas.convex(
            &[(x + w - cut, y), (x + w, y), (x + w, y + cut)],
            ui::argb(0xff, accent),
        );
        canvas.convex(
            &[(x, y + h - cut), (x + cut, y + h), (x, y + h)],
            ui::argb(0xff, accent),
        );
    } else {
        ui::icon(
            canvas,
            Icon::Crown,
            xi + pw / 2,
            py + 286,
            150,
            0x38ffffff,
            0x00000000,
        );
        ui::text_center(
            canvas,
            fonts,
            t("ILUSTRACIÓN DE LA REINA", "THE QUEEN'S ILLUSTRATION"),
            xi + pw / 2,
            py + 410,
            20.0,
            ui::argb(0xff, ui::BONE),
            true,
        );
        ui::text_center(
            canvas,
            fonts,
            t("Próximamente", "Coming soon"),
            xi + pw / 2,
            py + 444,
            18.0,
            MUTED,
            false,
        );
        ui::text_center(
            canvas,
            fonts,
            &format!("{PORTRAIT_FILE} · {pw}×{ph}"),
            xi + pw / 2,
            py + 478,
            14.0,
            0xff7d6d84,
            false,
        );
    }
    ui::outline(canvas, &frame, 3.0, ui::argb(0xff, accent));
    // Her name on a black plate across the frame's foot, the place under it.
    let ny = y + h - 118.0;
    ui::plate(canvas, x - 14.0, ny + 8.0, 340.0, 60.0, 20.0, 0xb4000000);
    ui::plate(canvas, x - 22.0, ny, 340.0, 60.0, 20.0, 0xff0c0811);
    ui::outline(
        canvas,
        &ui::plate_points(x - 22.0, ny, 340.0, 60.0, 20.0),
        1.2,
        0x66ffffff,
    );
    canvas.display_text(
        fonts,
        t("La Reina Demonio", "The Demon Queen"),
        xi + 4,
        ny as i32 + 8,
        30.0,
        0xffffffff,
    );
    let room = ellipsize(fonts, view.room, 250, 16.0, true);
    let room_w = fonts.text_width(&room, 16.0, true) as f32 + 44.0;
    ui::plate(
        canvas,
        x - 6.0,
        ny + 66.0,
        room_w,
        32.0,
        12.0,
        ui::argb(0xff, accent),
    );
    ui::text_mid(
        canvas,
        fonts,
        &room,
        xi + 18,
        ny as i32 + 82,
        16.0,
        ui::ink_on(accent),
        true,
    );
}

/// One counter of the status page: a slanted black tile with an edge of the accent, its name,
/// an icon and the count large (dim while it is still zero).
#[allow(clippy::too_many_arguments)]
fn ero_tile(
    canvas: &mut Canvas,
    fonts: &Fonts,
    rect: (i32, i32, i32, i32),
    dx: f32,
    label: &str,
    shown: i64,
    done: bool,
    icon: Icon,
    accent: u32,
) {
    let (x, y, w, h) = (
        rect.0 as f32 + dx,
        rect.1 as f32,
        rect.2 as f32,
        rect.3 as f32,
    );
    let skew = 18.0;
    ui::plate(canvas, x + 7.0, y + 7.0, w, h, skew, 0xa0000000);
    ui::plate(canvas, x, y, w, h, skew, 0xf2120b17);
    ui::plate(
        canvas,
        x,
        y,
        32.0,
        h,
        skew,
        ui::argb(if done { 0xff } else { 0x70 }, accent),
    );
    ui::halftone(
        canvas,
        (x + w * 0.55, y + 4.0, w * 0.45 - skew - 4.0, h - 8.0),
        11.0,
        2.2,
        (x + w - skew, y + h),
        w * 0.5,
        0x1cffffff,
    );
    ui::outline(canvas, &ui::plate_points(x, y, w, h, skew), 1.0, 0x38ffffff);
    let (xi, yi) = (x.round() as i32, rect.1);
    ui::text_mid(
        canvas,
        fonts,
        &label.to_uppercase(),
        xi + 42,
        yi + 28,
        15.0,
        if done { INK } else { MUTED },
        true,
    );
    ui::icon_disc(canvas, icon, xi + 60, yi + 78, 21, accent, done);
    let number = shown.to_string();
    let number_w = fonts.text_width(&number, 50.0, true);
    let right = xi + rect.2 - 36;
    canvas.text(
        fonts,
        &number,
        right - number_w + 3,
        yi + 46 + 3,
        50.0,
        0xff000000,
        true,
    );
    canvas.text(
        fonts,
        &number,
        right - number_w,
        yi + 46,
        50.0,
        if done { 0xffffffff } else { 0xff5c4d63 },
        true,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_corner_menu_button_is_a_card_with_see_through_corners() {
        let Some(fonts) = Fonts::new() else {
            return;
        };
        for hover in [false, true] {
            let b = trigger_bitmap(&fonts, Language::Spanish, hover);
            let alpha = |x: usize, y: usize| b.rgba[(y * b.w + x) * 4 + 3];
            assert_eq!(alpha(0, 0), 0, "the corner shows the world");
            assert_eq!(alpha(100, 28), 255, "the card is solid");
            if let Ok(dir) = std::env::var("VELVET_DUMP") {
                image::save_buffer(
                    std::path::Path::new(&dir).join(format!("trigger-{hover}.png")),
                    &b.rgba,
                    b.w as u32,
                    b.h as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
    }
    fn host() -> crate::WinitApp {
        let mut app = App::default();
        app.start_game().unwrap();
        crate::WinitApp::new(app)
    }
    #[test]
    fn escape_pauses_and_resumes_same_dialogue_without_quit() {
        let mut host = host();
        let before = host.app.view.as_ref().unwrap().id.clone();
        host.menu_back();
        assert_eq!(host.app.screen, Screen::GameMenu);
        assert!(!host.app.quit);
        host.menu_back();
        assert_eq!(host.app.screen, Screen::Play);
        assert_eq!(host.app.view.as_ref().unwrap().id, before);
    }
    #[test]
    fn back_from_gallery_returns_to_pause_not_title() {
        let mut host = host();
        host.menu_back();
        host.app.open_gallery(Screen::GameMenu);
        host.menu_back();
        assert_eq!(host.app.screen, Screen::GameMenu);
        assert!(host.app.pause.is_some());
        host.menu_back();
        assert_eq!(host.app.screen, Screen::Play);
    }
    #[test]
    fn dangerous_actions_default_to_cancel_and_escape_dismisses_only_modal() {
        let mut host = host();
        host.menu_back();
        host.app.ask_menu_confirmation(Confirm::Quit);
        host.app.menu_key(KeyCode::Enter);
        assert!(host.app.menu_action.is_none());
        assert!(!host.app.quit);
        host.app.ask_menu_confirmation(Confirm::Load(0));
        host.menu_back();
        assert_eq!(host.app.screen, Screen::GameMenu);
        assert!(host.app.pause.as_ref().unwrap().confirm.is_none());
    }
    #[test]
    fn menu_keyboard_navigation_and_letterbox_hitboxes_are_bounded() {
        let mut host = host();
        host.open_game_menu(Page::Load);
        for _ in 0..50 {
            host.app.menu_key(KeyCode::Tab);
        }
        assert!(host.app.pause.as_ref().unwrap().focus < NAV_COUNT + 9);
        for size in [(1280, 800), (1000, 1000), (1920, 1080), (2560, 1440)] {
            let viewport =
                velvet_render::SoftwareViewport::letterbox(size.0, size.1, crate::WIN_W, crate::WIN_H).unwrap();
            let x = viewport.x as f64
                + (TRIGGER.0 + TRIGGER.2 / 2) as f64 * viewport.width as f64 / crate::WIN_W as f64;
            let y = viewport.y as f64
                + (TRIGGER.1 + TRIGGER.3 / 2) as f64 * viewport.height as f64 / crate::WIN_H as f64;
            let (vx, vy) = viewport.physical_to_virtual(x, y).unwrap();
            assert!(rect_contains(TRIGGER, vx as i32, vy as i32));
        }
        let focus = host.app.pause.as_ref().unwrap().focus;
        host.app.menu_click(-1, -1);
        assert_eq!(host.app.pause.as_ref().unwrap().focus, focus);
    }
}
