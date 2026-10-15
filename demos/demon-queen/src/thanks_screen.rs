//! The thanks at the end of act 1: once the last scene of the act has played (the chess move,
//! `ajedrez_hecho`), the world stops under a Persona card — the act's name on a tag, «¡Gracias
//! por jugar!», a few lines on what comes next (the world stays open, the game keeps changing,
//! this build's version) — with the Queen beside it, smiling, and her bikini self laughing
//! behind her (`tools/gracias_art.py` makes them at their size). Enter, Esc or the button go back
//! to the world. It shows once (`missions::THANKS_SHOWN` in the story).
use std::time::Instant;

use winit::keyboard::KeyCode;

use crate::exploration3d::ui_sfx::{self, UiSfx};
use crate::localization::Language;
use crate::menu_ui::{self as ui, Rect, GOLD, INK};
use crate::{App, Screen};

const BUTTON: Rect = (110, 900, 420, 76);
const PANEL: (f32, f32, f32, f32) = (40.0, 70.0, 1040.0, 940.0);

fn t(language: Language, es: &'static str, en: &'static str) -> &'static str {
    if language != Language::English {
        es
    } else {
        en
    }
}

fn inside(r: Rect, x: i32, y: i32) -> bool {
    x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
}

impl App {
    /// The card comes up over `backdrop` (the world as it was).
    pub(crate) fn open_thanks(&mut self) {
        self.thanks_backdrop = self.canvas.pixels.clone();
        self.thanks_since = Instant::now();
        self.screen = Screen::Thanks;
        self.ui_hidden = false;
        self.mouse_down = false;
        ui_sfx::play(UiSfx::Open);
        self.needs_paint = true;
    }

    /// Back to the world.
    pub(crate) fn close_thanks(&mut self) {
        if self.screen != Screen::Thanks {
            return;
        }
        ui_sfx::play(UiSfx::Close);
        let backdrop = std::mem::take(&mut self.thanks_backdrop);
        self.canvas.restore(&backdrop);
        self.screen = Screen::Explore;
        self.needs_paint = true;
    }

    pub(crate) fn thanks_key(&mut self, key: KeyCode) {
        // A beat before it can be closed, so a held key does not skip it unseen.
        if self.thanks_since.elapsed().as_secs_f32() < 0.8 {
            return;
        }
        if matches!(
            key,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::Escape | KeyCode::KeyF
        ) {
            self.close_thanks();
        }
    }

    pub(crate) fn thanks_click(&mut self, x: i32, y: i32) {
        if inside(BUTTON, x, y) && self.thanks_since.elapsed().as_secs_f32() >= 0.8 {
            self.close_thanks();
        }
    }

    pub(crate) fn paint_thanks(&mut self) {
        let motion = !self.reduced_motion;
        let age = if motion {
            self.thanks_since.elapsed().as_secs_f32()
        } else {
            10.0
        };
        if self.thanks_backdrop.len() == self.canvas.pixels.len() {
            self.canvas.pixels.copy_from_slice(&self.thanks_backdrop);
        } else {
            self.canvas.clear(0xff120a16);
        }
        let light = ui::light_from_pixels(&self.thanks_backdrop, self.canvas.pw, self.canvas.ph);
        let accent = ui::accent(light);
        // The world goes dim, more on the left where the words are.
        let dim = (ui::ease_out((age / 0.4).min(1.0)) * 0xb0 as f32) as u32;
        self.canvas.rect(0, 0, 1920, 1080, dim << 24 | 0x07030c);
        for k in 0..8 {
            let x = 1250.0 - k as f32 * 70.0;
            self.canvas.convex(
                &[(0.0, 0.0), (x, 0.0), (x - 160.0, 1080.0), (0.0, 1080.0)],
                ui::argb(0x12, 0x07030c),
            );
        }
        // ── The Queen: her bikini self laughing behind, the smiling one in front.
        let rise = |delay: f32| {
            if motion {
                ui::entrance(age, delay, 0.5)
            } else {
                1.0
            }
        };
        let (behind, front) = (rise(0.25), rise(0.12));
        if let Some(b) = self.art.original("gracias/reina_bikini_risa.png") {
            let y = 150 + (60.0 * (1.0 - behind)) as i32;
            self.canvas.blit_rgba_adjusted(1370, y, b, 0.92, behind);
        }
        if let Some(b) = self.art.original("gracias/reina_sonrisa.png") {
            let y = -20 + (80.0 * (1.0 - front)) as i32;
            self.canvas.blit_rgba_adjusted(1060, y, b, 1.0, front);
        }
        if self.ui_hidden {
            return;
        }
        let language = self.localizer.language();
        let t = |es, en| t(language, es, en);
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let canvas = &mut self.canvas;

        // ── The card, slamming in from the left.
        let come = if motion { ui::entrance(age, 0.0, 0.42) } else { 1.0 };
        let dx = -(PANEL.2 + 200.0) * (1.0 - come);
        let (px, py, pw, ph) = (PANEL.0 + dx, PANEL.1, PANEL.2, PANEL.3);
        ui::plate(canvas, px + 14.0, py + 14.0, pw, ph, 60.0, 0xb4000000);
        ui::plate(canvas, px, py, pw, ph, 60.0, 0xf20d0814);
        ui::plate(canvas, px, py, 22.0, ph, 60.0, ui::argb(0xff, accent));
        ui::halftone(
            canvas,
            (px + pw * 0.75, py + ph * 0.8, pw * 0.25, ph * 0.2),
            14.0,
            3.4,
            (px + pw, py + ph),
            420.0,
            ui::argb(0x30, accent),
        );
        ui::outline(
            canvas,
            &ui::plate_points(px, py, pw, ph, 60.0),
            1.5,
            ui::argb(0x80, ui::mix(accent, 0xffffff, 0.4)),
        );
        let x = (px + 110.0) as i32;
        ui::tag(
            canvas,
            fonts,
            x,
            118,
            t(
                "FIN DEL ACTO 1 · LA CHISPA QUE SE APAGA",
                "END OF ACT 1 · THE SPARK THAT FADES",
            ),
            17.0,
            ui::ABYSS,
            ui::BONE,
        );
        let title = t("¡Gracias por jugar!", "Thank you for playing!");
        let size = ui::fit_size(fonts, title, 860, 92.0, false);
        canvas.display_text(fonts, title, x + 5, 176 + 5, size, 0xff000000);
        canvas.display_text(fonts, title, x, 176, size, 0xffffffff);
        canvas.line(
            (x as f32, 312.0),
            (x as f32 + 760.0, 302.0),
            3.0,
            ui::argb(0xff, accent),
        );
        let version = env!("CARGO_PKG_VERSION");
        let paragraphs = [
            {
                // Its two sentences each in the player's language, then joined (with no space
                // where the script uses none).
                let tr = |s: &str| demon_queen_dungeon::i18n::tr(s).into_owned();
                let first = tr(t(
                    "Has llegado al final del primer acto de Demon Queen.",
                    "You have reached the end of Demon Queen's first act.",
                ));
                let second = tr(&if language != Language::English {
                    format!("Esta es la versión {version}.")
                } else {
                    format!("This is version {version}.")
                });
                let gap = if matches!(
                    language,
                    Language::Japanese | Language::ChineseSimplified | Language::ChineseTraditional
                ) {
                    ""
                } else {
                    " "
                };
                format!("{first}{gap}{second}")
            },
            t(
                "El mundo sigue abierto: puedes seguir explorando el castillo, la villa, el bosque, las termas y el burdel, y volver a ver las escenas en la galería.",
                "The world stays open: you can keep exploring the castle, the village, the forest, the hot springs and the brothel, and watch the scenes again in the gallery.",
            )
            .to_owned(),
            t(
                "Demon Queen sigue en desarrollo. Cada versión traerá más historia y, además, irán cambiando y mejorando cosas que ya están: lugares, escenas y menús.",
                "Demon Queen is still in development. Every version will bring more story, and things already here will keep changing and improving too: places, scenes and menus.",
            )
            .to_owned(),
            t(
                "Gracias por acompañar a la Reina hasta aquí.",
                "Thank you for coming this far with the Queen.",
            )
            .to_owned(),
        ];
        let mut y = 346;
        for (i, text) in paragraphs.iter().enumerate() {
            let a = if motion {
                ui::entrance(age, 0.35 + 0.12 * i as f32, 0.4)
            } else {
                1.0
            };
            let color = if i == 3 { GOLD } else { INK };
            let alpha = ((color >> 24) as f32 * a) as u32;
            let color = alpha << 24 | (color & 0x00ff_ffff);
            y = canvas.paragraph(fonts, text, x, y, 820, 40, 26.0, color, i == 3) + 22;
        }
        ui::text_mid(
            canvas,
            fonts,
            "— Velvet Weaver",
            x + 520,
            y + 16,
            22.0,
            ui::argb(0xdd, ui::mix(accent, 0xffffff, 0.5)),
            true,
        );
        // ── The way back to the world.
        let ready = age >= 0.8;
        let button = (BUTTON.0 + dx as i32, BUTTON.1, BUTTON.2, BUTTON.3);
        ui::pill(
            canvas,
            fonts,
            button,
            t("Seguir explorando", "Keep exploring"),
            ready,
            ready,
            light,
            false,
        );
        ui::key_hints(
            canvas,
            fonts,
            button.0 + button.2 + 40,
            button.1 + 22,
            &[("ENTER", t("seguir", "continue"))],
        );
    }
}
