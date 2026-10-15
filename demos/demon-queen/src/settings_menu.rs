//! The settings, one panel shared by the title's Options and the pause menu's Settings page:
//! four tabs (game, sound, display, controls) of rows in the manner of Persona's menus — a
//! choice between arrows, a slanted ten-step bar, or a switch — with a line under each name
//! saying what it does. Volumes, text speed, mouse speed and full screen live in
//! `exploration3d::prefs`; the language and reduced motion are the app's. Focus and the pointer
//! only change a row's light and colour, never its place or size.
use std::time::Instant;

use winit::keyboard::KeyCode;
use winit::window::Fullscreen;

use crate::exploration3d::prefs::{self, Channel};
use crate::exploration3d::ui_sfx::{self, UiSfx};
use crate::localization::Language;
use crate::menu_ui::{self as ui, Rect, GOLD, INK, MUTED};
use crate::render::{Canvas, Fonts};
use crate::pause_menu::{Page, SETTINGS_AREA};
use crate::title_menu::OPTIONS_AREA;
use crate::{App, Screen};

pub(crate) const TABS: usize = 5;
const TAB_GAP: i32 = 14;
const TAB_H: i32 = 58;
/// Where the rows start under the tabs, and each row's height and pitch.
const ROWS_TOP: i32 = 88;
const ROW_H: i32 = 80;
const ROW_PITCH: i32 = 92;
/// The controls at a row's right end: their width, and each bar segment's.
const CONTROL_W: i32 = 470;
const SEGMENT_W: i32 = 34;
const SEGMENT_GAP: i32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Language,
    TextSpeed,
    Sensitivity,
    Volume(Channel),
    Fullscreen,
    NativeResolution,
    ReducedMotion,
    Gfx(Gfx),
}

/// The graphics tab's rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gfx {
    Preset,
    Shadows,
    BouncedLight,
    Reflections,
    Bloom,
    RenderScale,
    Upscaler,
    Dynamic,
    Antialias,
    Sharpness,
    Lod,
    Prepass,
}

const GRAPHICS: [Row; 12] = [
    Row::Gfx(Gfx::Preset),
    Row::Gfx(Gfx::Shadows),
    Row::Gfx(Gfx::BouncedLight),
    Row::Gfx(Gfx::Reflections),
    Row::Gfx(Gfx::Bloom),
    Row::Gfx(Gfx::RenderScale),
    Row::Gfx(Gfx::Upscaler),
    Row::Gfx(Gfx::Dynamic),
    Row::Gfx(Gfx::Antialias),
    Row::Gfx(Gfx::Sharpness),
    Row::Gfx(Gfx::Lod),
    Row::Gfx(Gfx::Prepass),
];

const GAME: [Row; 3] = [Row::Language, Row::TextSpeed, Row::Sensitivity];
const SOUND: [Row; 5] = [
    Row::Volume(Channel::Master),
    Row::Volume(Channel::Music),
    Row::Volume(Channel::Ambience),
    Row::Volume(Channel::Effects),
    Row::Volume(Channel::Scenes),
];
const DISPLAY: [Row; 3] = [Row::Fullscreen, Row::NativeResolution, Row::ReducedMotion];

/// The rows of tab `tab` (the controls' tab has none: it is a list of keys).
pub(crate) fn rows(tab: usize) -> &'static [Row] {
    match tab {
        0 => &GAME,
        1 => &SOUND,
        2 => &DISPLAY,
        3 => &GRAPHICS,
        _ => &[],
    }
}

/// Where the keyboard is in the panel, and since when the text-speed preview types.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SettingsUi {
    pub tab: usize,
    /// `None`: on the tabs.
    pub row: Option<usize>,
    pub preview_since: Instant,
    /// The first row shown when a tab has more than fit (the focus keeps itself in view).
    pub scroll: usize,
}

impl Default for SettingsUi {
    fn default() -> Self {
        Self {
            tab: 0,
            row: None,
            preview_since: Instant::now(),
            scroll: 0,
        }
    }
}

impl SettingsUi {
    /// Does the panel move by itself (the preview typing), so it must be painted every frame?
    pub(crate) fn animating(&self) -> bool {
        self.tab == 0
    }

    fn set_tab(&mut self, tab: usize) {
        self.tab = tab % TABS;
        self.row = None;
        self.scroll = 0;
        self.preview_since = Instant::now();
    }
}

/// What a change asks of the app (the rest is done here, in `prefs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    None,
    /// Step through the languages by this much.
    Language(i32),
    ReducedMotion,
    Fullscreen,
}

/// What a key did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Not the panel's key.
    Ignored,
    Moved,
    Changed(Effect),
    /// The keyboard leaves the panel (to the menu's list on its left).
    Leave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Previous,
    Next,
    /// A bar's step (0 = silence).
    Set(u8),
    Whole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hit {
    Tab(usize),
    Row(usize, Part),
}

fn t(language: Language, es: &'static str, en: &'static str) -> &'static str {
    if language != Language::English {
        es
    } else {
        en
    }
}

/// The panel's place: its left, top and width.
pub(crate) type Area = (i32, i32, i32);

fn tab_rect(area: Area, index: usize) -> Rect {
    let w = (area.2 - TAB_GAP * (TABS as i32 - 1)) / TABS as i32;
    (area.0 + index as i32 * (w + TAB_GAP), area.1, w, TAB_H)
}

/// How many rows fit under the tabs at `area`.
fn capacity(area: Area) -> usize {
    ((1040 - area.1 - ROWS_TOP) / ROW_PITCH).max(1) as usize
}

/// The first row shown: the stored scroll, moved so the focus is in view.
fn first_row(s: &SettingsUi, area: Area) -> usize {
    let n = rows(s.tab).len();
    let cap = capacity(area);
    if n <= cap {
        return 0;
    }
    let mut f = s.scroll.min(n - cap);
    if let Some(r) = s.row {
        if r < f {
            f = r;
        } else if r >= f + cap {
            f = r + 1 - cap;
        }
    }
    f
}

/// Row `index`'s place, where it shows now (`None` scrolled out of view).
fn shown_row_rect(s: &SettingsUi, area: Area, index: usize) -> Option<Rect> {
    let first = first_row(s, area);
    (index >= first && index < first + capacity(area)).then(|| row_rect(area, index - first))
}

fn row_rect(area: Area, index: usize) -> Rect {
    (
        area.0,
        area.1 + ROWS_TOP + index as i32 * ROW_PITCH,
        area.2,
        ROW_H,
    )
}

fn previous_rect(row: Rect) -> Rect {
    (row.0 + row.2 - CONTROL_W - 24, row.1 + 12, 76, row.3 - 24)
}

fn next_rect(row: Rect) -> Rect {
    (row.0 + row.2 - 24 - 76, row.1 + 12, 76, row.3 - 24)
}

/// A bar's segment `step` (0-based): the (step + 1)th of ten.
fn segment_rect(row: Rect, step: usize) -> Rect {
    let left = row.0 + row.2 - CONTROL_W - 24;
    (
        left + step as i32 * (SEGMENT_W + SEGMENT_GAP),
        row.1 + 22,
        SEGMENT_W,
        row.3 - 44,
    )
}

/// The bar's value now, its least, and its most.
fn bar(row: Row) -> Option<(u8, u8, u8)> {
    match row {
        Row::Volume(c) => Some((prefs::volume(c), 0, prefs::VOLUME_STEPS)),
        Row::Sensitivity => Some((prefs::sensitivity(), 1, prefs::SENSITIVITY_STEPS)),
        Row::Gfx(Gfx::Sharpness) => Some((prefs::graphics().sharpness, 0, 10)),
        _ => None,
    }
}

fn is_switch(row: Row) -> bool {
    matches!(
        row,
        Row::Fullscreen
            | Row::NativeResolution
            | Row::ReducedMotion
            | Row::Gfx(Gfx::BouncedLight)
            | Row::Gfx(Gfx::Reflections)
            | Row::Gfx(Gfx::Prepass)
    )
}

/// What is under (x, y).
pub(crate) fn hit(s: &SettingsUi, area: Area, x: i32, y: i32) -> Option<Hit> {
    let inside = |r: Rect| x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3;
    if let Some(i) = (0..TABS).find(|&i| inside(tab_rect(area, i))) {
        return Some(Hit::Tab(i));
    }
    for (i, row) in rows(s.tab).iter().enumerate() {
        let Some(r) = shown_row_rect(s, area, i) else {
            continue;
        };
        if !inside(r) {
            continue;
        }
        if bar(*row).is_some() {
            let last = segment_rect(r, 9);
            let first = segment_rect(r, 0);
            if x >= first.0 - 30 && x < last.0 + last.2 + 6 {
                // Left of the first segment is silence; each segment its own step.
                let step = if x < first.0 {
                    0
                } else {
                    ((x - first.0) / (SEGMENT_W + SEGMENT_GAP) + 1).min(10)
                };
                return Some(Hit::Row(i, Part::Set(step as u8)));
            }
            return Some(Hit::Row(i, Part::Whole));
        }
        if !is_switch(*row) {
            if inside(previous_rect(r)) {
                return Some(Hit::Row(i, Part::Previous));
            }
            if inside(next_rect(r)) {
                return Some(Hit::Row(i, Part::Next));
            }
        }
        return Some(Hit::Row(i, Part::Whole));
    }
    None
}

/// Moves a row's value by `delta` (bars a step, choices one along); the app's own come back
/// as an effect.
fn step(row: Row, delta: i32) -> Effect {
    match row {
        Row::Language => Effect::Language(delta),
        Row::TextSpeed => {
            let n = prefs::TEXT_SPEEDS as i32;
            prefs::set_text_speed((prefs::text_speed() as i32 + delta).rem_euclid(n) as u8);
            save();
            Effect::None
        }
        Row::Fullscreen => Effect::Fullscreen,
        Row::NativeResolution => {
            prefs::set_native_resolution(!prefs::native_resolution());
            save();
            Effect::None
        }
        Row::ReducedMotion => Effect::ReducedMotion,
        Row::Gfx(g) if g != Gfx::Sharpness => {
            set_graphics(step_graphics(g, delta));
            Effect::None
        }
        Row::Sensitivity | Row::Volume(_) | Row::Gfx(_) => {
            let (now, least, most) = bar(row).unwrap_or((0, 0, 10));
            set(row, (now as i32 + delta).clamp(least as i32, most as i32) as u8)
        }
    }
}

fn set(row: Row, value: u8) -> Effect {
    match row {
        Row::Volume(c) => prefs::set_volume(c, value),
        Row::Sensitivity => prefs::set_sensitivity(value),
        Row::Gfx(Gfx::Sharpness) => {
            let mut g = prefs::graphics();
            g.sharpness = value.min(10);
            g.preset = prefs::PRESET_CUSTOM;
            set_graphics(g);
            return Effect::None;
        }
        _ => return Effect::None,
    }
    save();
    Effect::None
}

/// The graphics settings changed and kept.
fn set_graphics(g: prefs::Graphics) {
    prefs::set_graphics(g);
    save();
}

/// A graphics row moved by `delta`: the preset brings all its values; anything else changed by
/// hand makes the settings custom.
fn step_graphics(row: Gfx, delta: i32) -> prefs::Graphics {
    let mut g = prefs::graphics();
    let cycle = |v: u8, n: u8| (v as i32 + delta).rem_euclid(n as i32) as u8;
    match row {
        Gfx::Preset => {
            // Low, medium, high, maximum (custom is only reached by hand).
            let from = if g.preset >= prefs::PRESET_CUSTOM { 3 } else { g.preset };
            return prefs::Graphics::preset(cycle(from, 4));
        }
        Gfx::Shadows => g.shadows = cycle(g.shadows, 4),
        Gfx::BouncedLight => g.bounced_light = !g.bounced_light,
        Gfx::Reflections => g.reflections = !g.reflections,
        Gfx::Bloom => g.bloom = cycle(g.bloom, 3),
        Gfx::RenderScale => {
            let at = prefs::RENDER_SCALES.iter().position(|&v| v == g.render_scale).unwrap_or(5);
            g.render_scale = prefs::RENDER_SCALES[cycle(at as u8, prefs::RENDER_SCALES.len() as u8) as usize];
        }
        Gfx::Upscaler => g.upscaler = cycle(g.upscaler, 2),
        Gfx::Dynamic => g.dynamic = cycle(g.dynamic, 3),
        Gfx::Antialias => g.antialias = cycle(g.antialias, 2),
        Gfx::Sharpness => g.sharpness = (g.sharpness as i32 + delta).clamp(0, 10) as u8,
        Gfx::Lod => g.lod = cycle(g.lod, 3),
        Gfx::Prepass => g.prepass = !g.prepass,
    }
    g.preset = prefs::PRESET_CUSTOM;
    g
}

/// What a graphics row shows as its value.
fn graphics_value(language: Language, row: Gfx) -> String {
    let g = prefs::graphics();
    let t = |es, en| t(language, es, en).to_owned();
    match row {
        Gfx::Preset => match g.preset {
            0 => t("Baja", "Low"),
            1 => t("Media", "Medium"),
            2 => t("Alta", "High"),
            3 => t("Máxima", "Maximum"),
            _ => t("Personalizada", "Custom"),
        },
        Gfx::Shadows => match g.shadows {
            0 => t("Sin sombras", "Off"),
            1 => t("Bajas", "Low"),
            2 => t("Medias", "Medium"),
            _ => t("Altas", "High"),
        },
        Gfx::Bloom => match g.bloom {
            0 => t("Apagado", "Off"),
            1 => t("Ligero", "Light"),
            _ => t("Completo", "Full"),
        },
        Gfx::RenderScale => format!("{} %", g.render_scale),
        Gfx::Upscaler => {
            if g.upscaler == 0 {
                t("Bilineal", "Bilinear")
            } else {
                "AMD FSR 1".to_owned()
            }
        }
        Gfx::Dynamic => match g.dynamic {
            0 => t("Apagada", "Off"),
            1 => t("Mantener 30 fps", "Keep 30 fps"),
            _ => t("Mantener 60 fps", "Keep 60 fps"),
        },
        Gfx::Antialias => {
            if g.antialias == 0 {
                t("Apagado", "Off")
            } else {
                "FXAA".to_owned()
            }
        }
        Gfx::Lod => match g.lod {
            0 => t("Máximo detalle", "Full detail"),
            1 => t("Equilibrado", "Balanced"),
            _ => t("Rendimiento", "Performance"),
        },
        Gfx::BouncedLight | Gfx::Reflections | Gfx::Sharpness | Gfx::Prepass => String::new(),
    }
}

fn save() {
    if let Err(error) = prefs::save() {
        eprintln!("no se pudieron guardar los ajustes: {error}");
    }
}

/// A key on the panel.
pub(crate) fn key(s: &mut SettingsUi, key: KeyCode) -> Outcome {
    let count = rows(s.tab).len();
    match key {
        KeyCode::KeyQ | KeyCode::PageUp => {
            s.set_tab((s.tab + TABS - 1) % TABS);
            return Outcome::Moved;
        }
        KeyCode::KeyE | KeyCode::PageDown | KeyCode::Tab => {
            s.set_tab((s.tab + 1) % TABS);
            return Outcome::Moved;
        }
        _ => {}
    }
    match s.row {
        None => match key {
            KeyCode::ArrowLeft | KeyCode::KeyA if s.tab == 0 => Outcome::Leave,
            KeyCode::ArrowLeft | KeyCode::KeyA => {
                s.set_tab(s.tab - 1);
                Outcome::Moved
            }
            KeyCode::ArrowRight | KeyCode::KeyD => {
                s.set_tab((s.tab + 1) % TABS);
                Outcome::Moved
            }
            KeyCode::ArrowDown
            | KeyCode::KeyS
            | KeyCode::Enter
            | KeyCode::NumpadEnter
            | KeyCode::Space
                if count > 0 =>
            {
                s.row = Some(0);
                Outcome::Moved
            }
            KeyCode::ArrowUp | KeyCode::KeyW => Outcome::Leave,
            _ => Outcome::Ignored,
        },
        Some(i) => {
            let Some(row) = rows(s.tab).get(i).copied() else {
                s.row = None;
                return Outcome::Moved;
            };
            match key {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    s.row = i.checked_sub(1);
                    Outcome::Moved
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    s.row = Some((i + 1).min(count - 1));
                    Outcome::Moved
                }
                KeyCode::ArrowLeft | KeyCode::KeyA => {
                    if row == Row::TextSpeed {
                        s.preview_since = Instant::now();
                    }
                    Outcome::Changed(step(row, -1))
                }
                KeyCode::ArrowRight | KeyCode::KeyD => {
                    if row == Row::TextSpeed {
                        s.preview_since = Instant::now();
                    }
                    Outcome::Changed(step(row, 1))
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    if bar(row).is_some() {
                        Outcome::Ignored
                    } else {
                        if row == Row::TextSpeed {
                            s.preview_since = Instant::now();
                        }
                        Outcome::Changed(step(row, 1))
                    }
                }
                _ => Outcome::Ignored,
            }
        }
    }
}

/// A click: focus goes where it lands and what is under it changes.
pub(crate) fn click(s: &mut SettingsUi, area: Area, x: i32, y: i32) -> Option<Outcome> {
    let h = hit(s, area, x, y)?;
    Some(match h {
        Hit::Tab(i) => {
            if i != s.tab {
                s.set_tab(i);
            }
            Outcome::Moved
        }
        Hit::Row(i, part) => {
            s.row = Some(i);
            let row = rows(s.tab)[i];
            if row == Row::TextSpeed {
                s.preview_since = Instant::now();
            }
            match part {
                Part::Previous => Outcome::Changed(step(row, -1)),
                Part::Next => Outcome::Changed(step(row, 1)),
                Part::Set(v) => {
                    let (_, least, most) = bar(row).unwrap_or((0, 0, 10));
                    Outcome::Changed(set(row, v.clamp(least, most)))
                }
                Part::Whole if is_switch(row) || row == Row::TextSpeed => {
                    Outcome::Changed(step(row, 1))
                }
                Part::Whole => Outcome::Moved,
            }
        }
    })
}

/// Dragging along a bar with the button held sets it where the pointer is.
pub(crate) fn drag(s: &mut SettingsUi, area: Area, x: i32, y: i32) -> bool {
    let Some(i) = s.row else {
        return false;
    };
    let Some(row) = rows(s.tab).get(i).copied() else {
        return false;
    };
    let Some((now, least, most)) = bar(row) else {
        return false;
    };
    let Some(r) = shown_row_rect(s, area, i) else {
        return false;
    };
    if y < r.1 - 40 || y > r.1 + r.3 + 40 {
        return false;
    }
    let first = segment_rect(r, 0);
    let step = ((x - first.0 + SEGMENT_W / 2) / (SEGMENT_W + SEGMENT_GAP)).clamp(0, 10) as u8;
    let value = step.clamp(least, most);
    if value != now {
        set(row, value);
        return true;
    }
    false
}

/// What the panel shows for the app's own settings.
pub(crate) struct Shown {
    pub language: Language,
    pub reduced_motion: bool,
    pub light: u32,
    pub mouse: (i32, i32),
    /// The keyboard is in the panel (its focus is drawn).
    pub keyboard: bool,
}

fn label(language: Language, row: Row) -> (&'static str, &'static str) {
    let t = |es, en| t(language, es, en);
    match row {
        Row::Language => (
            t("Idioma", "Language"),
            t(
                "Todo el juego cambia al momento.",
                "The whole game switches at once.",
            ),
        ),
        Row::TextSpeed => (
            t("Velocidad del texto", "Text speed"),
            t(
                "Lo rápido que se escriben los diálogos.",
                "How fast dialogue lines type out.",
            ),
        ),
        Row::Sensitivity => (
            t("Sensibilidad del ratón", "Mouse sensitivity"),
            t(
                "Al mirar en la mina y en el combate contra Varrak.",
                "Looking around in the mine and the fight with Varrak.",
            ),
        ),
        Row::Volume(Channel::Master) => (
            t("Volumen general", "Master volume"),
            t("Todo el sonido del juego.", "All of the game's sound."),
        ),
        Row::Volume(Channel::Music) => (
            t("Música", "Music"),
            t("La banda sonora de cada lugar.", "Each place's soundtrack."),
        ),
        Row::Volume(Channel::Ambience) => (
            t("Ambiente", "Ambience"),
            t(
                "Viento, agua, gente y los sonidos del mundo.",
                "Wind, water, crowds and the world's sounds.",
            ),
        ),
        Row::Volume(Channel::Effects) => (
            t("Efectos", "Effects"),
            t(
                "Pasos, golpes y los clics de los menús.",
                "Steps, blows and the menus' clicks.",
            ),
        ),
        Row::Volume(Channel::Scenes) => (
            t("Escenas y voces", "Scenes and voices"),
            t(
                "El sonido de las escenas animadas.",
                "The sound of the animated scenes.",
            ),
        ),
        Row::Fullscreen => (
            t("Pantalla completa", "Full screen"),
            t("También con F11.", "Also with F11."),
        ),
        Row::NativeResolution => (
            t("Resolución nativa", "Native resolution"),
            t(
                "Dibuja a la resolución de tu pantalla: más nítido. Apágalo si va lento o falla (1920×1080).",
                "Draws at your screen's resolution: sharper. Turn it off if it runs slow or fails (1920×1080).",
            ),
        ),
        Row::Gfx(g) => match g {
            Gfx::Preset => (
                t("Calidad gráfica", "Graphics quality"),
                t(
                    "Máxima es el juego tal cual. Cambiar algo a mano la deja en Personalizada.",
                    "Maximum is the game as made. Changing anything by hand makes it Custom.",
                ),
            ),
            Gfx::Shadows => (
                t("Sombras del sol", "Sun shadows"),
                t("Menos muestras: bordes más duros, más rápido.", "Fewer samples: harder edges, faster."),
            ),
            Gfx::BouncedLight => (
                t("Luz rebotada", "Bounced light"),
                t("La luz que reflejan las superficies cercanas (GI).", "Light reflected by nearby surfaces (GI)."),
            ),
            Gfx::Reflections => (
                t("Reflejos en el agua", "Water reflections"),
                t("El mundo reflejado en el agua quieta.", "The world mirrored in still water."),
            ),
            Gfx::Bloom => (
                t("Resplandor (bloom)", "Glow (bloom)"),
                t("El halo de las luces fuertes.", "The halo round bright lights."),
            ),
            Gfx::RenderScale => (
                t("Escala de render", "Render scale"),
                t(
                    "Dibuja el mundo más pequeño y lo reescala; la interfaz sigue nítida.",
                    "Draws the world smaller and scales it up; the interface stays sharp.",
                ),
            ),
            Gfx::Upscaler => (
                t("Reescalado", "Upscaler"),
                t(
                    "Cómo se agranda la imagen cuando la escala es menor del 100 %.",
                    "How the picture is enlarged when the scale is under 100 %.",
                ),
            ),
            Gfx::Dynamic => (
                t("Resolución dinámica", "Dynamic resolution"),
                t(
                    "Baja la escala sola cuando el juego va lento y la sube al recuperarse.",
                    "Lowers the scale by itself when the game slows down, and raises it back.",
                ),
            ),
            Gfx::Antialias => (
                t("Antialiasing", "Antialiasing"),
                t("Suaviza los bordes dentados.", "Smooths jagged edges."),
            ),
            Gfx::Sharpness => (
                t("Nitidez", "Sharpening"),
                t("Realce de detalle (AMD FSR 1 RCAS).", "Detail enhancement (AMD FSR 1 RCAS)."),
            ),
            Gfx::Prepass => (
                t("Pase previo de profundidad", "Depth pre-pass"),
                t(
                    "No pinta lo que queda tapado: más rápido. Donde dos superficies se solapan puede variar algún píxel.",
                    "Skips shading what ends up hidden: faster. Where two surfaces overlap a pixel may differ.",
                ),
            ),
            Gfx::Lod => (
                t("Nivel de detalle", "Level of detail"),
                t(
                    "Versiones más sencillas de lo que se ve pequeño en pantalla.",
                    "Simpler versions of things that look small on screen.",
                ),
            ),
        },
        Row::ReducedMotion => (
            t("Movimiento reducido", "Reduced motion"),
            t(
                "Sin entradas animadas ni temblores en los menús.",
                "No animated entrances or shaking in the menus.",
            ),
        ),
    }
}

fn speed_name(language: Language, speed: u8) -> &'static str {
    match speed {
        0 => t(language, "Lenta", "Slow"),
        1 => t(language, "Normal", "Normal"),
        2 => t(language, "Rápida", "Fast"),
        _ => t(language, "Instantánea", "Instant"),
    }
}

/// Paints the panel at `area`.
pub(crate) fn paint(
    canvas: &mut Canvas,
    fonts: &Fonts,
    s: &SettingsUi,
    area: Area,
    shown: &Shown,
) {
    let language = shown.language;
    let light = shown.light;
    let accent = ui::accent(light);
    let over = |r: Rect| {
        let (x, y) = shown.mouse;
        x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
    };

    // ---- the tabs: slanted plates, the open one in the accent
    let names = [
        t(language, "JUEGO", "GAME"),
        t(language, "SONIDO", "SOUND"),
        t(language, "PANTALLA", "DISPLAY"),
        t(language, "GRÁFICOS", "GRAPHICS"),
        t(language, "CONTROLES", "CONTROLS"),
    ];
    for (i, name) in names.iter().enumerate() {
        let r = tab_rect(area, i);
        let open = s.tab == i;
        let keyed = shown.keyboard && s.row.is_none() && open;
        let hot = over(r) || keyed;
        let (x, y, w, h) = (r.0 as f32, r.1 as f32, r.2 as f32, r.3 as f32);
        ui::plate(canvas, x + 6.0, y + 6.0, w, h, 16.0, 0x99000000);
        let face = if open {
            ui::argb(0xff, accent)
        } else if hot {
            ui::argb(0xf4, ui::mix(0x221830, light, 0.22))
        } else {
            ui::argb(0xee, ui::mix(0x110c17, light, 0.06))
        };
        ui::plate(canvas, x, y, w, h, 16.0, face);
        ui::outline(
            canvas,
            &ui::plate_points(x, y, w, h, 16.0),
            if keyed { 2.4 } else { 1.1 },
            if keyed {
                0xffffffff
            } else if open {
                0xccffffff
            } else {
                0x44ffffff
            },
        );
        let ink = if open { ui::ink_on(accent) } else { INK };
        let size = ui::fit_size(fonts, name, r.2 - 40, 21.0, true);
        ui::text_center(canvas, fonts, name, r.0 + r.2 / 2, r.1 + r.3 / 2, size, ink, true);
    }

    // ---- the rows
    for (i, row) in rows(s.tab).iter().copied().enumerate() {
        let Some(r) = shown_row_rect(s, area, i) else {
            continue;
        };
        let keyed = shown.keyboard && s.row == Some(i);
        let hot = keyed || over(r);
        ui::glass(canvas, r, 18, light, if hot { 1.0 } else { 0.0 });
        if keyed {
            // A bar of the accent down the row's left: where the keys are.
            ui::plate(
                canvas,
                r.0 as f32 - 4.0,
                r.1 as f32 + 8.0,
                12.0,
                r.3 as f32 - 16.0,
                4.0,
                ui::argb(0xff, accent),
            );
        }
        let (name, hint) = label(language, row);
        let words_w = r.2 - CONTROL_W - 90;
        let name_size = ui::fit_size(fonts, name, words_w, 24.0, true);
        ui::text_mid(canvas, fonts, name, r.0 + 34, r.1 + 29, name_size, INK, true);
        let hint_size = ui::fit_size(fonts, hint, words_w, 16.0, false);
        ui::text_mid(canvas, fonts, hint, r.0 + 34, r.1 + 57, hint_size, MUTED, false);

        if let Some((value, least, most)) = bar(row) {
            for step in 0..10usize {
                let seg = segment_rect(r, step);
                let (x, y, w, h) = (seg.0 as f32, seg.1 as f32, seg.2 as f32, seg.3 as f32);
                let lit = (step as u8) < value;
                ui::plate(canvas, x + 3.0, y + 3.0, w, h, 8.0, 0x88000000);
                let fill = if lit {
                    // The bar warms from the accent toward white as it fills.
                    ui::argb(
                        0xff,
                        ui::mix(accent, 0xffffff, 0.08 + 0.3 * step as f32 / 9.0),
                    )
                } else {
                    0xff1a1322
                };
                ui::plate(canvas, x, y, w, h, 8.0, fill);
                ui::outline(
                    canvas,
                    &ui::plate_points(x, y, w, h, 8.0),
                    1.0,
                    if lit { 0x88ffffff } else { 0x30ffffff },
                );
            }
            let _ = (least, most);
            let last = segment_rect(r, 9);
            let number = if matches!(row, Row::Volume(_)) {
                format!("{}", value as u32 * 10)
            } else {
                format!("{value}")
            };
            ui::text_center(
                canvas,
                fonts,
                &number,
                last.0 + last.2 + 50,
                r.1 + r.3 / 2,
                24.0,
                if hot { 0xffffffff } else { INK },
                true,
            );
        } else if is_switch(row) {
            let on = match row {
                Row::Fullscreen => prefs::fullscreen(),
                Row::NativeResolution => prefs::native_resolution(),
                Row::Gfx(Gfx::BouncedLight) => prefs::graphics().bounced_light,
                Row::Gfx(Gfx::Reflections) => prefs::graphics().reflections,
                Row::Gfx(Gfx::Prepass) => prefs::graphics().prepass,
                _ => shown.reduced_motion,
            };
            let x = r.0 + r.2 - 24 - 64;
            ui::toggle(canvas, x, r.1 + (r.3 - 34) / 2, on, light);
            let word = if on {
                t(language, "SÍ", "ON")
            } else {
                t(language, "NO", "OFF")
            };
            ui::text_right(
                canvas,
                fonts,
                word,
                x - 18,
                r.1 + r.3 / 2,
                18.0,
                if on { GOLD } else { MUTED },
                true,
            );
        } else {
            let previous = previous_rect(r);
            let next = next_rect(r);
            ui::pill(canvas, fonts, previous, "‹", over(previous), true, light, false);
            ui::pill(canvas, fonts, next, "›", over(next), true, light, false);
            let gfx_value;
            let value = match row {
                Row::Language => language.display_name(),
                Row::Gfx(g) => {
                    gfx_value = graphics_value(language, g);
                    gfx_value.as_str()
                }
                _ => speed_name(language, prefs::text_speed()),
            };
            let room = next.0 - (previous.0 + previous.2) - 30;
            let size = ui::fit_size(fonts, value, room, 26.0, true);
            ui::text_center(
                canvas,
                fonts,
                value,
                (previous.0 + previous.2 + next.0) / 2,
                r.1 + r.3 / 2,
                size,
                if hot { 0xffffffff } else { INK },
                true,
            );
        }
    }

    // ---- under the rows: what each tab has to add
    let shown_rows = rows(s.tab).len().min(capacity(area));
    let below = area.1 + ROWS_TOP + shown_rows as i32 * ROW_PITCH + 6;
    // More rows than fit: where the list stands (and the tab's note goes beside it, on the
    // right: the list reaches the bottom, where the pause menu's keys are on the left).
    let mut note_right = None;
    if rows(s.tab).len() > shown_rows {
        let first = first_row(s, area);
        let more = format!(
            "{}{} {}–{} / {}",
            if first > 0 { "▲ " } else { "" },
            if first + shown_rows < rows(s.tab).len() { "▼" } else { "" },
            first + 1,
            first + shown_rows,
            rows(s.tab).len()
        );
        ui::text_right(canvas, fonts, &more, area.0 + area.2 - 8, below + 18, 17.0, MUTED, true);
        note_right = Some(area.0 + area.2 - 8 - fonts.text_width(&more, 17.0, true) - 28);
    }
    match s.tab {
        0 => paint_preview(canvas, fonts, s, (area.0, below, area.2, 150), language, light),
        1 => {
            ui::text_mid(
                canvas,
                fonts,
                t(
                    language,
                    "Los cambios se oyen al momento.",
                    "Changes are heard at once.",
                ),
                area.0 + 8,
                below + 18,
                17.0,
                MUTED,
                false,
            );
        }
        2 => {
            ui::text_mid(
                canvas,
                fonts,
                t(
                    language,
                    "La imagen conserva siempre su proporción 16:9.",
                    "The picture always keeps its 16:9 shape.",
                ),
                area.0 + 8,
                below + 18,
                17.0,
                MUTED,
                false,
            );
        }
        3 => {
            let note = t(
                language,
                "Los cambios se ven al momento en el mundo 3D.",
                "Changes show at once in the 3D world.",
            );
            match note_right {
                Some(right) => {
                    ui::text_right(canvas, fonts, note, right, below + 18, 17.0, MUTED, false)
                }
                None => {
                    ui::text_mid(canvas, fonts, note, area.0 + 8, below + 18, 17.0, MUTED, false);
                }
            }
        }
        _ => paint_controls(canvas, fonts, (area.0, area.1 + ROWS_TOP, area.2), language, light),
    }
}

/// The text speed shown working: a line typing itself at the chosen speed, again and again.
fn paint_preview(
    canvas: &mut Canvas,
    fonts: &Fonts,
    s: &SettingsUi,
    rect: Rect,
    language: Language,
    light: u32,
) {
    ui::glass(canvas, rect, 18, light, 0.0);
    ui::text_mid(
        canvas,
        fonts,
        t(language, "VISTA PREVIA", "PREVIEW"),
        rect.0 + 30,
        rect.1 + 30,
        16.0,
        GOLD,
        true,
    );
    ui::tag(
        canvas,
        fonts,
        rect.0 + 30,
        rect.1 + 56,
        t(language, "CHIQUITA", "TINY"),
        15.0,
        ui::ABYSS,
        ui::BONE,
    );
    let line = t(
        language,
        "—Así aparecerán los diálogos, Majestad. Ni más rápido ni más lento de lo que vos digáis.",
        "—This is how lines will appear, Majesty. No faster and no slower than you say.",
    );
    // Typed in the player's language: the count is of what is drawn.
    let line = demon_queen_dungeon::i18n::tr(line);
    let total = line.chars().count();
    let rate = prefs::text_rate(72.0);
    let cycle = total as f32 / rate + 1.8;
    let age = s.preview_since.elapsed().as_secs_f32() % cycle.max(0.1);
    let shown = ((age * rate) as usize).min(total);
    let text: String = line.chars().take(shown).collect();
    let size = ui::fit_size(fonts, &line, rect.2 - 60, 21.0, false);
    canvas.text(fonts, &text, rect.0 + 30, rect.1 + 92, size, INK, false);
}

/// Every control, by where it is used, in four columns.
fn paint_controls(canvas: &mut Canvas, fonts: &Fonts, area: Area, language: Language, light: u32) {
    let t = |es, en| t(language, es, en);
    ui::glass(canvas, (area.0, area.1, area.2, 360), 22, light, 0.0);
    let columns: [(&str, &[(&str, &str)]); 4] = [
        (
            t("EXPLORAR", "EXPLORING"),
            &[
                ("WASD", t("moverse", "move")),
                ("Mayús", t("correr", "run")),
                ("F", t("hablar · usar", "talk · use")),
                ("M", t("mapa", "map")),
                ("J", t("cambiar misión", "next mission")),
                ("L", t("logros", "achievements")),
                ("Q  E", t("acercar · alejar", "zoom in · out")),
                ("Z  X", t("girar cámara", "turn camera")),
                ("V", t("vista desde arriba", "top view")),
            ],
        ),
        (
            t("LA MINA", "THE MINE"),
            &[
                (t("Ratón", "Mouse"), t("mirar", "look")),
                (t("Clic", "Click"), t("atacar", "attack")),
                (t("Espacio", "Space"), t("saltar", "jump")),
                ("F", t("usar · abrir", "use · open")),
                ("Q", t("curarse (y mochila)", "heal (and backpack)")),
                ("T", t("antorcha", "torch")),
                ("R", t("andar solo", "auto-walk")),
                ("M", t("mapa", "map")),
                ("G", t("salir", "leave")),
            ],
        ),
        (
            t("COMBATE · VARRAK", "FIGHT · VARRAK"),
            &[
                (t("Clic", "Click"), t("espada", "sword")),
                (t("Clic dcho.", "Right click"), t("sello (mantener)", "seal (hold)")),
                ("Mayús", t("correr", "run")),
                (t("Espacio", "Space"), t("saltar la onda", "jump the wave")),
                ("Q", t("curarse (mochila)", "heal (backpack)")),
                ("WASD", t("moverse", "move")),
            ],
        ),
        (
            t("GENERAL", "GENERAL"),
            &[
                ("Esc", t("menú · volver", "menu · back")),
                ("F5", t("guardar", "save")),
                ("F9", t("cargar", "load")),
                ("F11", t("pantalla completa", "full screen")),
                ("Enter", t("avanzar · aceptar", "next · accept")),
                (t("Rueda ↑", "Wheel ↑"), t("diálogo anterior", "previous line")),
                ("1-4", t("elegir respuesta", "pick an answer")),
                ("U", t("usar (en la mochila)", "use (backpack)")),
            ],
        ),
    ];
    let column_w = (area.2 - 72) / 4;
    for (k, (title, rows)) in columns.iter().enumerate() {
        let x = area.0 + 36 + k as i32 * column_w;
        ui::text_mid(
            canvas,
            fonts,
            title,
            x,
            area.1 + 36,
            16.0,
            ui::mix(light, 0xffffff, 0.5) | 0xff000000,
            true,
        );
        for (i, (key, what)) in rows.iter().enumerate() {
            let y = area.1 + 60 + i as i32 * 32;
            let w = ui::keycap(canvas, fonts, x, y - 1, key);
            let room = column_w - w - 22;
            let size = ui::fit_size(fonts, what, room, 16.0, false);
            ui::text_mid(canvas, fonts, what, x + w + 10, y + 14, size, INK, false);
        }
    }
}

impl App {
    /// What a change of the panel asks of the app.
    pub(crate) fn settings_effect(&mut self, effect: Effect) {
        match effect {
            Effect::None => {}
            Effect::Language(delta) => {
                let current = Language::ALL
                    .iter()
                    .position(|l| *l == self.localizer.language())
                    .unwrap_or(0);
                let next = (current as i32 + delta).rem_euclid(Language::ALL.len() as i32) as usize;
                self.select_language(next);
                self.refresh_pause_history();
            }
            Effect::ReducedMotion => {
                let value = !self.reduced_motion;
                match crate::localization::save_reduced_motion(value, self.localizer.language()) {
                    Ok(()) => self.reduced_motion = value,
                    Err(error) => eprintln!("no se pudo guardar el movimiento reducido: {error:#}"),
                }
            }
            Effect::Fullscreen => self.toggle_fullscreen(),
        }
        self.needs_paint = true;
    }

    /// The key's sound and what it changed.
    pub(crate) fn settings_outcome(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Ignored => {}
            Outcome::Moved => ui_sfx::play(UiSfx::Move),
            Outcome::Leave => ui_sfx::play(UiSfx::Back),
            Outcome::Changed(effect) => {
                ui_sfx::play(UiSfx::Select);
                self.settings_effect(effect);
            }
        }
        self.needs_paint = true;
    }

    /// The pointer moved with the button held: a bar in focus follows it.
    pub(crate) fn settings_drag(&mut self) {
        let (x, y) = self.mouse;
        let moved = match self.screen {
            Screen::Options => drag(&mut self.options_settings, OPTIONS_AREA, x, y),
            Screen::GameMenu => self.pause.as_mut().is_some_and(|m| {
                m.page == Page::Settings && drag(&mut m.settings, SETTINGS_AREA, x, y)
            }),
            _ => false,
        };
        if moved {
            self.needs_paint = true;
        }
    }

    /// Is a settings panel on show typing its preview (so it wants frames)?
    pub(crate) fn settings_typing(&self) -> bool {
        match self.screen {
            Screen::Options => self.options_settings.animating(),
            Screen::GameMenu => self
                .pause
                .as_ref()
                .is_some_and(|m| m.page == Page::Settings && m.settings.animating()),
            _ => false,
        }
    }

    /// Full screen on or off, kept for next time.
    pub(crate) fn toggle_fullscreen(&mut self) {
        prefs::set_fullscreen(!prefs::fullscreen());
        save();
        self.apply_fullscreen();
    }

    /// The window as the preference says: borderless full screen, or a window.
    pub(crate) fn apply_fullscreen(&self) {
        if let Some(window) = &self.window {
            window.set_fullscreen(prefs::fullscreen().then_some(Fullscreen::Borderless(None)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Area = (486, 236, 1306);

    #[test]
    fn the_keys_walk_the_tabs_and_rows_and_leave_on_the_left() {
        let mut s = SettingsUi::default();
        assert_eq!(key(&mut s, KeyCode::ArrowLeft), Outcome::Leave);
        assert_eq!(key(&mut s, KeyCode::ArrowRight), Outcome::Moved);
        assert_eq!(s.tab, 1);
        assert_eq!(key(&mut s, KeyCode::ArrowDown), Outcome::Moved);
        assert_eq!(s.row, Some(0));
        for _ in 0..9 {
            key(&mut s, KeyCode::ArrowDown);
        }
        assert_eq!(s.row, Some(rows(1).len() - 1));
        assert_eq!(key(&mut s, KeyCode::KeyE), Outcome::Moved);
        assert_eq!((s.tab, s.row), (2, None));
        key(&mut s, KeyCode::KeyE);
        key(&mut s, KeyCode::KeyE);
        key(&mut s, KeyCode::KeyE);
        assert_eq!(s.tab, 0);
        // The controls' tab has no rows: down stays on the tabs.
        s.set_tab(4);
        assert_eq!(key(&mut s, KeyCode::ArrowDown), Outcome::Ignored);
    }

    #[test]
    fn the_graphics_tab_scrolls_to_its_last_row_and_a_change_by_hand_is_custom() {
        let mut s = SettingsUi::default();
        s.set_tab(3);
        key(&mut s, KeyCode::ArrowDown);
        for _ in 0..20 {
            key(&mut s, KeyCode::ArrowDown);
        }
        let last = rows(3).len() - 1;
        assert_eq!(s.row, Some(last));
        // The focused row is in view, the first ones scrolled out.
        assert!(shown_row_rect(&s, AREA, last).is_some());
        assert!(shown_row_rect(&s, AREA, 0).is_none());
        // The preset brings its values; one by hand makes it custom.
        prefs::set_graphics(prefs::Graphics::MAXIMUM);
        let low = step_graphics(Gfx::Preset, 1);
        assert_eq!(low, prefs::Graphics::preset(0));
        prefs::set_graphics(low);
        let changed = step_graphics(Gfx::Shadows, 1);
        assert_eq!(changed.preset, prefs::PRESET_CUSTOM);
        assert_eq!(changed.shadows, 1);
        // Maximum is the renderer's full look.
        assert_eq!(prefs::Graphics::MAXIMUM.quality(), velvet_render::Quality3D::default());
        prefs::set_graphics(prefs::Graphics::MAXIMUM);
    }

    #[test]
    fn each_part_of_a_row_answers_the_pointer_at_its_middle() {
        let mut s = SettingsUi::default();
        for i in 0..TABS {
            let r = tab_rect(AREA, i);
            assert_eq!(hit(&s, AREA, r.0 + r.2 / 2, r.1 + r.3 / 2), Some(Hit::Tab(i)));
        }
        let row = row_rect(AREA, 0);
        let p = previous_rect(row);
        let n = next_rect(row);
        assert_eq!(hit(&s, AREA, p.0 + p.2 / 2, p.1 + p.3 / 2), Some(Hit::Row(0, Part::Previous)));
        assert_eq!(hit(&s, AREA, n.0 + n.2 / 2, n.1 + n.3 / 2), Some(Hit::Row(0, Part::Next)));
        s.set_tab(1);
        let row = row_rect(AREA, 2);
        for step in 0..10 {
            let seg = segment_rect(row, step);
            assert_eq!(
                hit(&s, AREA, seg.0 + seg.2 / 2, seg.1 + seg.3 / 2),
                Some(Hit::Row(2, Part::Set(step as u8 + 1)))
            );
        }
        let first = segment_rect(row, 0);
        assert_eq!(hit(&s, AREA, first.0 - 10, first.1 + 4), Some(Hit::Row(2, Part::Set(0))));
        assert_eq!(hit(&s, AREA, AREA.0 - 20, row.1 + 10), None);
    }
}
