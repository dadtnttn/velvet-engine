//! The title screen in Persona's manner, drawn in code like the pause menu: the place of the option
//! in focus fills the screen behind (the throne, the balcony, the alcove, the gate), crossing over
//! to the next when the focus moves; a band of that place's light runs down the left with the
//! title and the four options as slanted plates (the one in focus bone-white with a glint of the
//! accent and a red pointer at it), embers rising, the seal turning; the place's name sits in
//! the corner on a slash. Focus only recolours the plates; the band and the plates come in once,
//! when the screen opens.
use std::time::Instant;

use crate::menu_ui as ui;
use crate::prompt_icons::Icon;
use crate::{localization::Language, App, Screen};
use winit::keyboard::KeyCode;

/// The place behind each option (under `data/assets`, 1920 × 1080, drawn 1:1) and what the
/// corner says of it.
pub(crate) const PLACES: [(&str, &str, &str); 4] = [
    (
        "native_1080p/title/lugar_trono",
        "EL TRONO",
        "El gran salón, a medianoche",
    ),
    (
        "native_1080p/title/lugar_balcon",
        "EL BALCÓN",
        "Dos lunas sobre la villa",
    ),
    (
        "native_1080p/title/lugar_alcoba",
        "LA ALCOBA",
        "El tocador del espejo ovalado",
    ),
    (
        "native_1080p/title/lugar_puertas",
        "LAS PUERTAS",
        "El camino al Bosque Oscuro",
    ),
];

/// The places' names in the title's reel (`title_reel.rs`): `trono` loops, `trono_balcon` flies.
const REEL: [&str; 4] = ["trono", "balcon", "alcoba", "puertas"];

const ITEMS: [(&str, &str, Icon); 4] = [
    ("Nueva partida", "New game", Icon::Play),
    ("Continuar", "Continue", Icon::Disk),
    ("Opciones", "Options", Icon::Gear),
    ("Salir", "Quit", Icon::Power),
];

/// How long the place behind takes to cross over, and the screen to come in, in seconds.
const CROSS: f32 = 0.45;
const ENTER: f32 = 0.42;
const SKEW: f32 = 22.0;

/// An option's plate: a stair stepping right as it goes down.
pub(crate) fn item_rect(index: usize) -> (i32, i32, i32, i32) {
    (86 + index as i32 * 24, 468 + index as i32 * 98, 430, 76)
}

/// The option under (x, y).
pub(crate) fn item_at(x: i32, y: i32) -> Option<usize> {
    (0..ITEMS.len()).find(|&i| {
        let (rx, ry, rw, rh) = item_rect(i);
        crate::rect_contains((rx, ry, rw, rh), x, y)
    })
}

fn tr(language: Language, es: &'static str, en: &'static str) -> &'static str {
    if language == Language::Spanish {
        es
    } else {
        en
    }
}

impl App {
    fn title_enabled(&self, index: usize) -> bool {
        index != 1 || self.continue_available
    }

    /// The next option that can be taken, `step` (±1) at a time.
    fn title_step(&self, step: i32) -> usize {
        let mut i = self.menu_sel.min(ITEMS.len() - 1) as i32;
        for _ in 0..ITEMS.len() {
            i = (i + step).rem_euclid(ITEMS.len() as i32);
            if self.title_enabled(i as usize) {
                break;
            }
        }
        i as usize
    }

    /// Focus on option `index`: its place crosses over the one behind.
    pub(crate) fn title_focus(&mut self, index: usize) {
        let index = index.min(ITEMS.len() - 1);
        if index == self.menu_sel && self.title_background_current == index {
            return;
        }
        self.menu_sel = index;
        if self.title_background_current != index {
            let from = self.title_background_current.min(ITEMS.len() - 1);
            if let Some(reel) = self.title_reel.as_mut() {
                reel.fly(REEL[from], REEL[index]);
            }
            self.title_background_previous = from;
            self.title_background_current = index;
            self.title_background_transition_started = Instant::now();
            self.title_background_transition_active = !self.reduced_motion;
        }
        self.needs_paint = true;
    }

    pub(crate) fn title_activate(&mut self, index: usize) {
        if !self.title_enabled(index) {
            return;
        }
        let result = match index {
            0 => self.start_game(),
            1 => self.continue_game(),
            2 => {
                self.open_options();
                Ok(())
            }
            _ => {
                self.quit = true;
                Ok(())
            }
        };
        if let Err(error) = result {
            eprintln!("no se pudo abrir la partida: {error:#}");
            self.autosave_error = Some(format!("{error:#}"));
            self.continue_available = crate::game::has_continue();
            self.needs_paint = true;
        }
    }

    pub(crate) fn title_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                let i = self.title_step(-1);
                self.title_focus(i);
            }
            KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => {
                let i = self.title_step(1);
                self.title_focus(i);
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                self.title_activate(self.menu_sel)
            }
            KeyCode::Digit1 => self.title_activate(0),
            KeyCode::Digit2 => self.title_activate(1),
            KeyCode::Digit3 => self.title_activate(2),
            KeyCode::Digit4 => self.title_activate(3),
            _ => {}
        }
    }

    pub(crate) fn title_click(&mut self, x: i32, y: i32) {
        if let Some(i) = item_at(x, y) {
            if self.title_enabled(i) {
                self.title_focus(i);
                self.title_activate(i);
            }
        }
    }

    pub(crate) fn title_scroll(&mut self, up: bool) {
        let i = self.title_step(if up { -1 } else { 1 });
        self.title_focus(i);
    }

    /// The light of place `index` (its dominant colour, 0xRRGGBB), worked out once.
    fn title_light(&mut self, index: usize) -> u32 {
        if let Some(light) = self.title_lights[index] {
            return light;
        }
        let light = self
            .art
            .original(PLACES[index].0)
            .map(|b| {
                let c = crate::dominant_light_with(b.w, b.h, |x, y| {
                    let i = (y * b.w + x) * 4;
                    [b.rgba[i], b.rgba[i + 1], b.rgba[i + 2], 255]
                });
                let to = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
                (to(c[0]) << 16) | (to(c[1]) << 8) | to(c[2])
            })
            .unwrap_or(0x8a3a9a);
        self.title_lights[index] = Some(light);
        light
    }

    pub(crate) fn paint_title_persona(&mut self) {
        let now = Instant::now();
        let age = now.duration_since(self.title_since).as_secs_f32();
        let clock = age;
        let motion = !self.reduced_motion;
        // Under the pointer, an option takes the focus (and its place comes up behind).
        if let Some(i) = item_at(self.mouse.0, self.mouse.1) {
            if self.title_enabled(i) && i != self.menu_sel {
                self.title_focus(i);
            }
        }
        let current = self.title_background_current.min(ITEMS.len() - 1);
        let previous = self.title_background_previous.min(ITEMS.len() - 1);
        let cross = if self.title_background_transition_active {
            let k = self
                .title_background_transition_started
                .elapsed()
                .as_secs_f32()
                / CROSS;
            if k >= 1.0 {
                self.title_background_transition_active = false;
                self.title_background_previous = current;
                1.0
            } else {
                ui::ease_out(k)
            }
        } else {
            1.0
        };
        // ── The place: its clip (a flight to it, then its loop) when the reel has one; else the
        // old picture whole and the new one over it.
        if !self.title_reel_tried {
            self.title_reel_tried = true;
            self.title_reel =
                crate::title_reel::TitleReel::open(crate::assets_root().join("title/reel"));
            if let Some(reel) = self.title_reel.as_mut() {
                reel.idle(REEL[current]);
            }
        }
        self.canvas.clear(0xff0b0710);
        let moving = match self.title_reel.as_mut().and_then(|r| r.frame()) {
            Some(frame) => {
                self.canvas.blit_rgba(0, 0, frame);
                true
            }
            None => false,
        };
        if !moving {
            if cross < 1.0 {
                if let Some(b) = self.art.original(PLACES[previous].0) {
                    self.canvas.blit_rgba(0, 0, b);
                }
            }
            if let Some(b) = self.art.original(PLACES[current].0) {
                if cross >= 0.999 {
                    self.canvas.blit_rgba(0, 0, b);
                } else {
                    self.canvas.blit_rgba_adjusted(0, 0, b, 1.0, cross);
                }
            }
        }
        if self.ui_hidden {
            return;
        }
        let light = {
            let (a, b) = (self.title_light(previous), self.title_light(current));
            ui::mix(a, b, cross)
        };
        let accent = ui::accent(light);
        let enabled: [bool; 4] = [0, 1, 2, 3].map(|i| self.title_enabled(i));
        let (menu_sel, mouse_down) = (self.menu_sel, self.mouse_down);
        let language = self.localizer.language();
        let t = |es, en| tr(language, es, en);
        let enter = if motion {
            ui::entrance(age, 0.0, ENTER)
        } else {
            1.0
        };
        let slide = 620.0 * (1.0 - enter);
        let canvas = &mut self.canvas;

        // ── A shade from the left (the band and its words read on any place).
        for k in 0..10 {
            let x = 640.0 + k as f32 * 60.0;
            let a = (0x58 as f32 * (1.0 - k as f32 / 10.0)) as u32;
            canvas.convex(
                &[(0.0, 0.0), (x, 0.0), (x - 90.0, 1080.0), (0.0, 1080.0)],
                ui::argb(a / 3, 0x07030c),
            );
        }
        // ── The band down the left: its shadow, a deep shade of the accent, a lighter slash,
        // halftone at its foot and a white hairline along its edge.
        let band = [
            (-slide, 0.0),
            (600.0 - slide, 0.0),
            (512.0 - slide, 1080.0),
            (-slide, 1080.0),
        ];
        canvas.convex(&ui::moved(&band, 16.0, 0.0), 0x9a000000);
        canvas.convex(&band, ui::argb(0xe8, ui::deepen(accent, 0.66, 0.14)));
        let edge = |y: f32| 600.0 - 88.0 * y / 1080.0 - slide;
        canvas.convex(
            &[
                (-slide, 300.0),
                (edge(170.0), 170.0),
                (edge(250.0), 250.0),
                (-slide, 400.0),
            ],
            ui::argb(0x5c, accent),
        );
        ui::halftone(
            canvas,
            (-slide, 700.0, 520.0, 380.0),
            15.0,
            5.6,
            (-slide, 1080.0),
            470.0,
            0x55000000,
        );
        canvas.line(
            (606.0 - slide, 0.0),
            (518.0 - slide, 1080.0),
            2.0,
            0xd8ffffff,
        );
        canvas.line(
            (618.0 - slide, 0.0),
            (530.0 - slide, 1080.0),
            6.0,
            ui::argb(0xff, accent),
        );
        if motion {
            crate::pause_menu::paint_embers(canvas, clock, slide, accent);
        }
        crate::pause_menu::paint_sigil(
            canvas,
            262.0 - slide,
            1010.0,
            196.0,
            clock * 0.08,
            0x40000000,
        );
        crate::pause_menu::paint_sigil(
            canvas,
            1812.0,
            108.0,
            62.0,
            -clock * 0.2,
            ui::argb(0x90, accent),
        );

        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        // ── The title.
        let sx = -(slide.round() as i32);
        canvas.display_text(fonts, "Demon Queen", 72 + sx + 5, 74 + 5, 80.0, 0xff000000);
        canvas.display_text(fonts, "Demon Queen", 72 + sx, 74, 80.0, 0xffffffff);
        let subtitle = self.localizer.ui("title.subtitle");
        let sub = if subtitle.is_empty() {
            t(
                "Aquella vez que un infarto me reencarnó",
                "That time a heart attack reincarnated me",
            )
        } else {
            subtitle
        };
        ui::tag(canvas, fonts, 80 + sx, 182, sub, 15.0, ui::ABYSS, ui::BONE);
        canvas.line(
            (80.0 - slide, 246.0),
            (470.0 - slide, 236.0),
            2.0,
            0xccffffff,
        );
        ui::text_mid(
            canvas,
            fonts,
            t(
                "Una novela visual de la Reina Demonio",
                "A Demon Queen visual novel",
            ),
            82 + sx,
            282,
            18.0,
            0xddffffff,
            false,
        );

        // ── The options, coming in one after another; the one in focus with a pointer at it.
        for (index, (es, en, icon)) in ITEMS.iter().enumerate() {
            let rect = item_rect(index);
            let enabled = enabled[index];
            let selected = enabled && menu_sel == index;
            let come = if motion {
                ui::entrance(age, 0.10 + index as f32 * 0.06, 0.30)
            } else {
                1.0
            };
            let dx = -((rect.0 + rect.2 + 90) as f32) * (1.0 - come);
            plate(
                canvas,
                fonts,
                rect,
                dx,
                t(es, en),
                *icon,
                enabled,
                selected,
                index == 3,
                accent,
                clock,
                motion,
                mouse_down && selected,
            );
            if selected && come > 0.99 {
                let bob = if motion {
                    (clock * 5.0).sin() * 5.0
                } else {
                    0.0
                };
                pointer(
                    canvas,
                    (rect.0 as f32 - 14.0 + bob, (rect.1 + rect.3 / 2) as f32),
                    22.0,
                );
            }
        }

        // ── The place's name in the corner, on a slash of the accent (the old one fading out).
        // (The old name goes in the first half of the crossing, the new comes in the second.)
        for (index, alpha) in [
            (previous, 1.0 - (cross * 2.0).min(1.0)),
            (current, (cross * 2.0 - 1.0).max(0.0)),
        ] {
            if alpha <= 0.01 {
                continue;
            }
            let (_, name, line) = PLACES[index];
            let a = |c: u32| ui::argb(((c >> 24) as f32 * alpha * enter) as u32, c & 0x00ff_ffff);
            ui::plate(
                canvas,
                1360.0,
                914.0,
                520.0,
                74.0,
                26.0,
                a(ui::argb(0xf0, ui::deepen(accent, 0.7, 0.16))),
            );
            ui::plate(
                canvas,
                1360.0,
                914.0,
                14.0,
                74.0,
                6.0,
                a(ui::argb(0xff, accent)),
            );
            canvas.display_text(fonts, name, 1404 + 3, 922 + 3, 46.0, a(0xff000000));
            canvas.display_text(fonts, name, 1404, 922, 46.0, a(0xffffffff));
            ui::text_mid(canvas, fonts, line, 1406, 1012, 18.0, a(0xeeffffff), false);
        }

        // ── The keys, the makers.
        let y = 1010;
        ui::key_hints(
            canvas,
            fonts,
            80 + sx,
            y,
            &[
                ("↑↓", t("elegir", "choose")),
                ("ENTER", t("aceptar", "accept")),
                ("ESC", t("salir", "quit")),
            ],
        );
        if let Some(error) = self.autosave_error.as_deref() {
            ui::text_mid(canvas, fonts, error, 82 + sx, 980, 15.0, 0xffe8737f, false);
        }
        ui::text_mid(
            canvas,
            fonts,
            "A GAME BY VELVET WEAVER · VELVET ENGINE",
            1404,
            66,
            14.0,
            0xb0ffffff,
            true,
        );
    }
}

/// An option's plate (as the pause menu's, bigger): a slanted black plate, bone-white with dark
/// words in focus with a glint of the accent running over it; Salir's accent is its warning red.
#[allow(clippy::too_many_arguments)]
fn plate(
    canvas: &mut crate::render::Canvas,
    fonts: &crate::render::Fonts,
    rect: (i32, i32, i32, i32),
    dx: f32,
    label: &str,
    icon: Icon,
    enabled: bool,
    selected: bool,
    danger: bool,
    accent: u32,
    clock: f32,
    motion: bool,
    pressed: bool,
) {
    let (x, y, w, h) = (
        rect.0 as f32 + dx,
        rect.1 as f32,
        rect.2 as f32,
        rect.3 as f32,
    );
    let accent = if danger {
        ui::DANGER & 0x00ff_ffff
    } else {
        accent
    };
    ui::plate(canvas, x + 9.0, y + 9.0, w, h, SKEW, 0xa8000000);
    let face = if selected {
        if pressed {
            ui::argb(0xff, ui::mix(ui::BONE, accent, 0.35))
        } else {
            ui::argb(0xff, ui::BONE)
        }
    } else {
        0xf2120b17
    };
    ui::plate(canvas, x, y, w, h, SKEW, face);
    if selected && motion {
        let p = (clock % 1.8) / 1.8;
        let band = x - 120.0 + (w + 240.0) * p;
        let (lo, hi) = (x.max(band), (x + w).min(band + 80.0));
        if hi - lo > SKEW + 1.0 {
            ui::plate(canvas, lo, y, hi - lo, h, SKEW, ui::argb(0x34, accent));
        }
    }
    // A wedge of the accent at the plate's end.
    ui::plate(
        canvas,
        x + w - 40.0,
        y,
        40.0,
        h,
        SKEW,
        ui::argb(if selected { 0xff } else { 0x70 }, accent),
    );
    ui::outline(
        canvas,
        &ui::plate_points(x, y, w, h, SKEW),
        if selected { 2.5 } else { 1.0 },
        if selected {
            ui::argb(0xff, accent)
        } else {
            0x40ffffff
        },
    );
    let cy = rect.1 + rect.3 / 2;
    ui::icon_disc(
        canvas,
        icon,
        (x + 50.0).round() as i32,
        cy,
        25,
        accent,
        selected,
    );
    let color = if !enabled {
        ui::argb(0xff, ui::mix(ui::MUTED, 0x000000, 0.4))
    } else if selected {
        if danger {
            0xffa3182f
        } else {
            0xff0c0811
        }
    } else {
        ui::INK
    };
    let size = ui::fit_size(fonts, label, 300, 31.0, true);
    ui::text_mid(
        canvas,
        fonts,
        label,
        (x + 92.0).round() as i32,
        cy,
        size,
        color,
        true,
    );
}

/// The pointer at the option in focus: a red arrowhead with a white edge.
fn pointer(canvas: &mut crate::render::Canvas, tip: (f32, f32), size: f32) {
    let back = tip.0 - size * 1.25;
    let up = (back - size * 0.15, tip.1 - size * 0.62);
    let down = (back - size * 0.15, tip.1 + size * 0.62);
    let e = size * 0.18;
    canvas.convex(
        &[
            (tip.0 + e, tip.1),
            (down.0 - e, down.1 + e),
            (up.0 - e, up.1 - e),
        ],
        0xffffffff,
    );
    canvas.convex(&[tip, down, up], 0xffd02a52);
    canvas.convex(&[(back + size * 0.2, tip.1), down, up], 0xff8a1030);
}

/// Has the title just come up (for its entrance)? Called before each paint.
pub(crate) fn note_screen(app: &mut App) {
    if app.screen == Screen::Title && app.last_painted_screen != Some(Screen::Title) {
        app.title_since = Instant::now();
    }
    app.last_painted_screen = Some(app.screen);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_option_answers_the_pointer_at_its_middle_and_none_outside() {
        for i in 0..ITEMS.len() {
            let (x, y, w, h) = item_rect(i);
            assert_eq!(item_at(x + w / 2, y + h / 2), Some(i));
        }
        assert_eq!(item_at(1500, 500), None);
        assert_eq!(item_at(100, 100), None);
        // The plates do not overlap.
        for i in 1..ITEMS.len() {
            let (_, y0, _, h0) = item_rect(i - 1);
            let (_, y1, _, _) = item_rect(i);
            assert!(y0 + h0 < y1);
        }
    }
}
