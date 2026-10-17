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

/// The guide's round «i»: in the middle of the seal turning in the top-right corner.
pub(crate) const INFO_BUTTON: (f32, f32, f32) = (1812.0, 108.0, 34.0);

/// Is (x, y) on the guide's button?
pub(crate) fn info_button_at(x: i32, y: i32) -> bool {
    let (cx, cy, r) = INFO_BUTTON;
    (x as f32 - cx).hypot(y as f32 - cy) <= r + 8.0
}

/// A page of the guide: a place, where it is, what is there and what it asks of her. Every text
/// is (Spanish, English); other languages translate the Spanish when it is drawn.
pub(crate) struct GuidePage {
    pub(crate) name: (&'static str, &'static str),
    pub(crate) place: (&'static str, &'static str),
    /// Its picture under `data/assets` (a view of the place in the game).
    pub(crate) picture: Option<&'static str>,
    pub(crate) animated: bool,
    pub(crate) what: &'static [(&'static str, &'static str)],
    pub(crate) needs: &'static [(&'static str, &'static str)],
}

/// The guide's pages after its cover.
pub(crate) const GUIDE: [GuidePage; 9] = [
    GuidePage {
        name: ("El Burdel · la villa, de noche", "The Brothel · the village, at night"),
        place: ("En la villa. El mapa (M) lo marca.", "In the village. The map (M) marks it."),
        picture: Some("guia/burdel.jpg"),
        animated: true,
        what: &[
            (
                "Rosalía, en el mostrador, lleva el libro de chicas: eliges una, pagas y te acompaña a su cuarto.",
                "Rosalía keeps the book of girls at the desk: you pick one, pay, and she takes you to her room.",
            ),
            (
                "Alba («Lo que pide») y Noa («Silencio») tienen minijuegos con escenas animadas; Nina tiene su escena en la cama.",
                "Alba («What she asks for») and Noa («Silence») have minigames with animated scenes; Nina has her scene in bed.",
            ),
            (
                "El elixir de Rosalía abre escenas nuevas con cada chica (el primero cuesta 10 de oro, luego 40).",
                "Rosalía's elixir opens new scenes with every girl (the first costs 10 gold, then 40).",
            ),
        ],
        needs: &[
            (
                "La primera vez solo abre de noche: duerme en tu cama del castillo para que pase el día.",
                "The first time it only opens at night: sleep in your castle bed to let the day go by.",
            ),
            (
                "Necesitas la capa con capucha (tienda de la villa, 100 de oro); te la pones en la puerta.",
                "You need the hooded cloak (village shop, 100 gold); you put it on at the door.",
            ),
            (
                "Tras la primera noche, Rosalía te da su ficha y ya puedes entrar cuando quieras.",
                "After the first night Rosalía gives you her token, and you may come in whenever you like.",
            ),
        ],
    },
    GuidePage {
        name: ("La Tienda de la villa", "The village shop"),
        place: ("En la plaza de la villa.", "On the village square."),
        picture: Some("guia/tienda.jpg"),
        animated: true,
        what: &[
            (
                "Compra, vende y fusiona objetos (pestaña Fusión): curas, espadas y materiales de la mina.",
                "Buy, sell and fuse items (the Fusion tab): cures, swords and materials from the mine.",
            ),
            (
                "Regatea el precio (H). Si se atasca, el tendero propone «Otro trato»: un favor a cambio de descuento, con escena animada.",
                "Haggle over the price (H). If it stalls, the shopkeeper offers «Another deal»: a favour for a discount, with an animated scene.",
            ),
        ],
        needs: &[
            (
                "Hay once favores y cada día se abre uno nuevo: duerme en el castillo para que pase el día.",
                "There are eleven favours and a new one opens each day: sleep in the castle to let the day go by.",
            ),
            (
                "Lo que sacas de la mina se vende aquí.",
                "What you bring out of the mine sells here.",
            ),
        ],
    },
    GuidePage {
        name: ("Tu alcoba · el castillo", "Your alcove · the castle"),
        place: ("En el castillo, donde empieza cada día.", "In the castle, where every day begins."),
        picture: Some("guia/alcoba.jpg"),
        animated: true,
        what: &[
            (
                "La cama tiene su propio menú: explorar el cuerpo de la Reina (escena animada), el sello, bromear con Chiquita y dormir.",
                "The bed has its own menu: explore the Queen's body (animated scene), the seal, tease Tiny and sleep.",
            ),
            (
                "El vestidor del castillo cambia tu atuendo: vestido real, vestido verde, bikini y capa.",
                "The castle wardrobe changes your outfit: royal dress, green dress, bikini and cloak.",
            ),
        ],
        needs: &[(
            "Dormir pasa el día (o la noche): así abre el burdel y llega el favor nuevo de la tienda.",
            "Sleeping passes the day (or the night): that is how the brothel opens and the shop's new favour arrives.",
        )],
    },
    GuidePage {
        name: ("El Bosque Oscuro · el mago", "The Dark Forest · the mage"),
        place: ("En un claro del bosque, junto al sendero.", "In a clearing of the forest, by the trail."),
        picture: Some("guia/mago.jpg"),
        animated: true,
        what: &[
            (
                "Un mago hipnotista con su puesto: habla con él y compra sus sales y su amuleto.",
                "A hypnotist mage with his stall: talk to him and buy his salts and his amulet.",
            ),
            (
                "Su espectáculo, «No mires el anillo», es una escena animada con minijuego.",
                "His show, «Don't look at the ring», is an animated scene with a minigame.",
            ),
        ],
        needs: &[(
            "No hablará con la Reina vestida de reina: ponte el vestido verde en el vestidor del castillo.",
            "He won't talk to the Queen in her royal dress: put on the green dress at the castle wardrobe.",
        )],
    },
    GuidePage {
        name: ("Las Termas del Valle", "The Valley Hot Springs"),
        place: ("Desde la villa, por los baños termales.", "From the village, through the bathhouse."),
        picture: Some("guia/termas.jpg"),
        animated: false,
        what: &[
            (
                "Una bienvenida en escena con la recepcionista y Mugo, el goblin.",
                "A staged welcome with the receptionist and Mugo the goblin.",
            ),
            (
                "Te regalan el bikini negro y la piedra de grabación para hacer fotos; Mugo te lleva a seis sitios.",
                "You are given the black bikini and the recording stone to take photos; Mugo leads you to six spots.",
            ),
        ],
        needs: &[(
            "El bikini solo se lleva en las termas hasta que tu deseo llegue a «Despierta» (estado Ero, en el menú).",
            "The bikini stays at the springs until your desire reaches «Awakened» (Ero status, in the menu).",
        )],
    },
    GuidePage {
        name: ("El Peñascal", "The Crag"),
        place: ("Subiendo entre las rocas del bosque.", "Up between the rocks of the forest."),
        picture: Some("guia/penascal.jpg"),
        animated: false,
        what: &[
            (
                "El pueblo de Varrak, el jefe del acto 1: un combate frenético en primera persona.",
                "The village of Varrak, the boss of Act 1: a frantic first-person fight.",
            ),
            (
                "Llévalo contra las rocas para aturdirlo y usa el sello (mantén el clic derecho).",
                "Lure him into the rocks to stun him and use the seal (hold the right click).",
            ),
        ],
        needs: &[
            (
                "Varrak no te hará caso hasta el segundo Consejo (misión principal del acto 1).",
                "Varrak will ignore you until the second Council (Act 1 main mission).",
            ),
            (
                "Antes puedes retarle de práctica desde su piedra.",
                "Before that you can challenge him for practice from his stone.",
            ),
        ],
    },
    GuidePage {
        name: ("La mina", "The mine"),
        place: ("En el bosque, a la entrada de la cueva.", "In the forest, at the cave's mouth."),
        picture: Some("guia/mina.jpg"),
        animated: false,
        what: &[
            (
                "Calabozos en primera persona con enemigos, cofres y la espada de piedra.",
                "First-person dungeons with enemies, chests and the stone sword.",
            ),
            (
                "Lo que encuentres se vende o se fusiona en la tienda (la gelatina vuelve pegajosa tu espada).",
                "What you find sells or fuses at the shop (jelly makes your sword sticky).",
            ),
        ],
        needs: &[
            (
                "Necesitas un arma para bajar: la espada de madera se compra en la tienda (40 de oro).",
                "You need a weapon to go down: the wooden sword is sold at the shop (40 gold).",
            ),
            (
                "Q te cura con lo que lleves en la mochila.",
                "Q heals you with what you carry in the backpack.",
            ),
        ],
    },
    GuidePage {
        name: ("La villa · aún en obras", "The village · still being built"),
        place: ("El centro del mundo: la plaza, la tienda y el burdel.", "The heart of the world: the square, the shop and the brothel."),
        picture: Some("guia/villa.jpg"),
        animated: false,
        what: &[
            (
                "La taberna «El Jabalí Dorado» y la casa del alcalde todavía están en desarrollo: verás un aviso al entrar.",
                "The «Golden Boar» tavern and the mayor's house are still in development: you'll see a notice at the door.",
            ),
            (
                "Los aldeanos siguen su rutina de día y de noche.",
                "The villagers follow their routine by day and by night.",
            ),
        ],
        needs: &[],
    },
    GuidePage {
        name: ("Consejos", "Tips"),
        place: ("Para no perderse.", "So you don't get lost."),
        picture: None,
        animated: false,
        what: &[
            (
                "La misión elegida te guía con una línea en el suelo: J cambia de misión y M abre el mapa.",
                "The chosen mission guides you with a line on the ground: J switches mission and M opens the map.",
            ),
            (
                "Esc abre el menú: mochila, equipo, estado, guardar y ajustes. F5 guarda y F9 carga.",
                "Esc opens the menu: backpack, equipment, status, save and settings. F5 saves and F9 loads.",
            ),
            (
                "Si algo se atasca, «Continuar» te devuelve a tu última partida guardada.",
                "If something gets stuck, «Continue» takes you back to your last save.",
            ),
        ],
        needs: &[],
    },
];

/// The guide's pages: its cover and one per place.
pub(crate) const GUIDE_PAGES: usize = GUIDE.len() + 1;

/// How long a page takes to turn.
const TURN: f32 = 0.42;

/// What a player should know of this build.
const NOTES: [&str; 6] = [
    "Es una versión en desarrollo: faltan muchas cosas y se trabaja en ellas cada día.",
    "Se pueden jugar el prólogo y el acto 1. Una parte del acto 1 (los preparativos antes del Peñascal) y los actos siguientes llegarán más adelante.",
    "Unos sitios están más pulidos que otros: puede haber rincones vacíos, textos provisionales o sonidos de prueba.",
    "El arte y las escenas animadas se siguen rehaciendo y pueden cambiar.",
    "La partida se guarda sola: si algo falla o se queda atascado, vuelve con «Continuar».",
    "En el juego: M el mapa, J la misión que te guía, Esc el menú.",
];

/// The option under (x, y).
pub(crate) fn item_at(x: i32, y: i32) -> Option<usize> {
    (0..ITEMS.len()).find(|&i| {
        let (rx, ry, rw, rh) = item_rect(i);
        crate::rect_contains((rx, ry, rw, rh), x, y)
    })
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

    /// Opens or closes the guide (it opens on its cover).
    pub(crate) fn title_info_toggle(&mut self) {
        self.title_info = !self.title_info;
        self.title_info_since = Instant::now();
        if self.title_info {
            self.title_info_page = 0;
            self.title_info_turn = None;
        }
        self.needs_paint = true;
    }

    /// The guide on page `to`, turning to it.
    pub(crate) fn guide_turn_to(&mut self, to: usize) {
        let to = to.min(GUIDE_PAGES - 1);
        if to == self.title_info_page {
            return;
        }
        let dir = if to > self.title_info_page { 1 } else { -1 };
        self.title_info_turn = Some((self.title_info_page, Instant::now(), dir));
        self.title_info_page = to;
        crate::exploration3d::ui_sfx::play(crate::exploration3d::ui_sfx::UiSfx::Page);
        self.needs_paint = true;
    }

    /// One page on (`step` 1) or back (-1).
    fn guide_step(&mut self, step: i32) {
        let to = self.title_info_page as i32 + step;
        if (0..GUIDE_PAGES as i32).contains(&to) {
            self.guide_turn_to(to as usize);
        }
    }

    /// A click on the open guide: its arrows, a place of the cover's index, or out of it (closes).
    fn guide_click(&mut self, x: i32, y: i32) {
        let (px, py, pw, ph) = GUIDE_PLATE;
        if !crate::rect_contains((px, py, pw, ph), x, y) {
            self.title_info_toggle();
            return;
        }
        if crate::rect_contains(GUIDE_PREV, x, y) {
            self.guide_step(-1);
        } else if crate::rect_contains(GUIDE_NEXT, x, y) {
            self.guide_step(1);
        } else if self.title_info_page == 0 {
            if let Some(i) =
                (0..GUIDE.len()).find(|&i| crate::rect_contains(guide_index_rect(i), x, y))
            {
                self.guide_turn_to(i + 1);
            }
        }
    }

    pub(crate) fn title_key(&mut self, key: KeyCode) {
        if self.title_info {
            match key {
                KeyCode::Escape | KeyCode::KeyI | KeyCode::Backspace => self.title_info_toggle(),
                KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::PageUp => self.guide_step(-1),
                KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::PageDown => self.guide_step(1),
                KeyCode::Home => self.guide_turn_to(0),
                // On to the next page; past the last one, closed.
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    if self.title_info_page + 1 < GUIDE_PAGES {
                        self.guide_step(1);
                    } else {
                        self.title_info_toggle();
                    }
                }
                _ => {}
            }
            return;
        }
        match key {
            KeyCode::KeyI => self.title_info_toggle(),
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
        // The guide: its own clicks while open; the «i» opens it.
        if self.title_info {
            self.guide_click(x, y);
            return;
        }
        if info_button_at(x, y) {
            self.title_info_toggle();
            return;
        }
        if let Some(i) = item_at(x, y) {
            if self.title_enabled(i) {
                self.title_focus(i);
                self.title_activate(i);
            }
        }
    }

    pub(crate) fn title_scroll(&mut self, up: bool) {
        if self.title_info {
            self.guide_step(if up { -1 } else { 1 });
            return;
        }
        let i = self.title_step(if up { -1 } else { 1 });
        self.title_focus(i);
    }

    /// The light of place `index` (its dominant colour, 0xRRGGBB), worked out once.
    pub(crate) fn title_light(&mut self, index: usize) -> u32 {
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
        if let Some(i) = item_at(self.mouse.0, self.mouse.1).filter(|_| !self.title_info) {
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
        // The clip's frame, already scaled on its thread to the size it takes here when it came
        // so (the same pixels as scaling it now).
        let moving = match self.title_reel.as_mut() {
            Some(reel) => match reel.frame().map(|f| (f.w, f.h)) {
                Some((w, h)) => {
                    let size = self.canvas.physical_size(0, 0, w, h);
                    reel.set_target(size);
                    let copied = reel
                        .shown_scaled(size)
                        .is_some_and(|scaled| self.canvas.blit_prescaled(0, 0, w, h, scaled));
                    if !copied {
                        if let Some(frame) = reel.frame() {
                            self.canvas.blit_rgba(0, 0, frame);
                        }
                    }
                    true
                }
                None => false,
            },
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

        paint_band(canvas, accent, slide, clock, motion);

        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        // ── The guide's «i», in the middle of the corner's seal (under the pointer it lights up;
        // its size never changes).
        {
            let (cx, cy, r) = INFO_BUTTON;
            let over = info_button_at(self.mouse.0, self.mouse.1) || self.title_info;
            let pulse = if motion {
                0.5 + 0.5 * (clock * 2.4).sin()
            } else {
                0.5
            };
            canvas.smooth_disc(cx + 4.0, cy + 5.0, r, 0x99000000);
            canvas.smooth_ring(
                cx,
                cy,
                r + 7.0,
                3.0,
                ui::argb((0x50 as f32 + 0x60 as f32 * pulse) as u32, accent),
            );
            canvas.smooth_disc(
                cx,
                cy,
                r,
                if over {
                    ui::argb(0xff, accent)
                } else {
                    ui::argb(0xff, ui::BONE)
                },
            );
            canvas.smooth_ring(
                cx,
                cy,
                r,
                3.0,
                if over {
                    0xffffffff
                } else {
                    ui::argb(0xff, accent)
                },
            );
            let ink = if over { 0xffffffff } else { 0xff120b17 };
            // The «i»: its dot and its stem.
            canvas.smooth_disc(cx, cy - 13.0, 4.6, ink);
            canvas.convex(
                &[
                    (cx - 4.2, cy - 4.0),
                    (cx + 4.2, cy - 4.0),
                    (cx + 4.2, cy + 17.0),
                    (cx - 4.2, cy + 17.0),
                ],
                ink,
            );
            // Its word ends just before the seal, however long a language makes it.
            let word = t("GUÍA", "GUIDE");
            ui::text_mid(
                canvas,
                fonts,
                word,
                (cx - r - 30.0) as i32 - fonts.text_width(word, 15.0, true),
                (cy + 2.0) as i32,
                15.0,
                if over { 0xffffffff } else { 0xb0ffffff },
                true,
            );
        }
        // ── The title.
        let sx = -(slide.round() as i32);
        canvas.brand_text(fonts, "Demon Queen", 72 + sx + 5, 74 + 5, 80.0, 0xff000000);
        canvas.brand_text(fonts, "Demon Queen", 72 + sx, 74, 80.0, 0xffffffff);
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
        // Two rows, so they stay on the band whatever the language makes of them.
        ui::key_hints(
            canvas,
            fonts,
            80 + sx,
            y - 44,
            &[
                ("↑↓", t("elegir", "choose")),
                ("ENTER", t("aceptar", "accept")),
                ("I", t("guía", "guide")),
            ],
        );
        ui::key_hints(
            canvas,
            fonts,
            80 + sx,
            y,
            &[
                ("ALT", t("mantener: ver el fondo", "hold: see the picture")),
                ("ESC", t("salir", "quit")),
            ],
        );
        if let Some(error) = self.autosave_error.as_deref() {
            ui::text_mid(canvas, fonts, error, 82 + sx, 930, 15.0, 0xffe8737f, false);
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
        // The build's version (the game's own, from its Cargo.toml).
        ui::text_mid(
            canvas,
            fonts,
            &format!("VERSIÓN {} · EN DESARROLLO", env!("CARGO_PKG_VERSION")),
            1404,
            90,
            13.0,
            0x90ffffff,
            true,
        );
        if self.title_info {
            let k = if motion {
                ui::entrance(self.title_info_since.elapsed().as_secs_f32(), 0.0, 0.3)
            } else {
                1.0
            };
            // The pages on show: the one turned to, and while it turns the one going out.
            let turn = self.title_info_turn.and_then(|(from, since, dir)| {
                let t = since.elapsed().as_secs_f32() / TURN;
                (motion && t < 1.0).then_some((from, ui::ease_out(t), dir))
            });
            if turn.is_none() {
                self.title_info_turn = None;
            }
            let page = self.title_info_page;
            let picture = |p: usize| GUIDE.get(p.wrapping_sub(1)).and_then(|g| g.picture);
            let (now_key, old_key) = (picture(page), turn.and_then(|(from, _, _)| picture(from)));
            // Both in the cache (loading the second may clear it: the first comes back then).
            for key in [now_key, old_key, now_key].into_iter().flatten() {
                let _ = self.art.original(key);
            }
            let now_art = now_key.and_then(|key| self.art.peek_original(key));
            let old_art = old_key.and_then(|key| self.art.peek_original(key));
            paint_info(
                &mut self.canvas,
                fonts,
                accent,
                language,
                k,
                GuideView {
                    page,
                    turn,
                    now_art,
                    old_art,
                    mouse: self.mouse,
                },
            );
        }
    }
}

/// The guide's plate on screen (x, y, w, h).
const GUIDE_PLATE: (i32, i32, i32, i32) = (170, 92, 1580, 900);
/// Its arrows, bottom left and right.
const GUIDE_PREV: (i32, i32, i32, i32) = (240, 860, 120, 56);
const GUIDE_NEXT: (i32, i32, i32, i32) = (1560, 860, 120, 56);

/// A place's line in the cover's index (clicked, its page).
fn guide_index_rect(i: usize) -> (i32, i32, i32, i32) {
    (240, 352 + i as i32 * 56, 700, 46)
}

/// What the guide shows this frame.
struct GuideView<'a> {
    page: usize,
    /// Turning: the page going out, how far (0..1, eased) and which way (1 on, -1 back).
    turn: Option<(usize, f32, i32)>,
    now_art: Option<&'a crate::ArtBitmap>,
    old_art: Option<&'a crate::ArtBitmap>,
    mouse: (i32, i32),
}

/// The guide: a book on a slanted dark plate over the dimmed title. Its cover holds the index of
/// the places and the notes on this build; each place has a page (its picture, what is there and
/// what it asks of her). The pages turn sideways: the old one slides out and fades as the new one
/// slides in, with a slash of the accent sweeping across like a page's edge.
fn paint_info(
    canvas: &mut crate::render::Canvas,
    fonts: &crate::render::Fonts,
    accent: u32,
    language: Language,
    k: f32,
    view: GuideView,
) {
    let t = |es, en| tr(language, es, en);
    let a = |c: u32| ui::argb(((c >> 24) as f32 * k) as u32, c & 0x00ff_ffff);
    canvas.rect(0, 0, 1920, 1080, a(0xc8050309));
    let dy = (1.0 - k) * 40.0;
    let (px, py, pw, ph) = GUIDE_PLATE;
    let (x, y, w, h) = (px as f32, py as f32 + dy, pw as f32, ph as f32);
    ui::plate(canvas, x + 14.0, y + 14.0, w, h, 26.0, a(0xb0000000));
    ui::plate(canvas, x, y, w, h, 26.0, a(0xf6100a16));
    ui::plate(canvas, x, y, 16.0, h, 8.0, a(ui::argb(0xff, accent)));
    ui::outline(
        canvas,
        &ui::plate_points(x, y, w, h, 26.0),
        2.0,
        a(ui::argb(0xff, accent)),
    );
    let (xi, yi) = (x as i32, y as i32);
    // The pages. Turning, a slanted edge sweeps across the plate the way it turns: past it the
    // new page (drawn whole, then kept only there), before it the old one, a shadow on the old
    // side and the edge itself a slash of the accent with a white hairline.
    match view.turn {
        Some((from, e, dir)) => {
            let base = canvas.pixels.clone();
            paint_guide_page(
                canvas,
                fonts,
                accent,
                language,
                view.page,
                view.now_art,
                view.mouse,
                (xi, yi),
                dir as f32 * 50.0 * (1.0 - e),
                k,
            );
            let fresh = std::mem::replace(&mut canvas.pixels, base);
            paint_guide_page(
                canvas,
                fonts,
                accent,
                language,
                from,
                view.old_art,
                view.mouse,
                (xi, yi),
                -(dir as f32) * 30.0 * e,
                k,
            );
            let slant = 64.0;
            let ex = if dir > 0 {
                x + w + 10.0 - e * (w + slant + 30.0)
            } else {
                x - slant - 20.0 + e * (w + slant + 30.0)
            };
            // Where the edge crosses logical row `ly`.
            let edge_at = |ly: f32| ex + (y + h - ly) / h * slant;
            let (pw, ph, kk) = (canvas.pw, canvas.ph, canvas.k);
            let top = ((y * kk).floor().max(0.0) as usize).min(ph);
            let bottom = (((y + h) * kk).ceil().max(0.0) as usize).min(ph);
            for py in top..bottom {
                let edge = (edge_at(py as f32 / kk) * kk).round().clamp(0.0, pw as f32) as usize;
                let row = py * pw;
                let span = if dir > 0 {
                    row + edge..row + pw
                } else {
                    row..row + edge
                };
                canvas.pixels[span.clone()].copy_from_slice(&fresh[span]);
            }
            // The shadow the turning page casts on the old one, then the edge.
            let side = -(dir as f32);
            for i in 0..8 {
                let d0 = side * (i as f32 * 9.0);
                let d1 = side * ((i + 1) as f32 * 9.0);
                let band = [
                    (ex + d0, y + h),
                    (ex + d1, y + h),
                    (ex + d1 + slant, y),
                    (ex + d0 + slant, y),
                ];
                canvas.convex(&band, a(ui::argb((0x50 - i * 9) as u32, 0x000000)));
            }
            let edge = [
                (ex - 10.0, y + h),
                (ex + 10.0, y + h),
                (ex + 10.0 + slant, y),
                (ex - 10.0 + slant, y),
            ];
            canvas.convex(&edge, a(ui::argb(0xe0, accent)));
            let line = [
                (ex - 1.5, y + h),
                (ex + 1.5, y + h),
                (ex + 1.5 + slant, y),
                (ex - 1.5 + slant, y),
            ];
            canvas.convex(&line, a(0xffffffff));
        }
        None => paint_guide_page(
            canvas,
            fonts,
            accent,
            language,
            view.page,
            view.now_art,
            view.mouse,
            (xi, yi),
            0.0,
            k,
        ),
    }
    // The foot, which stays: the arrows, a dot per page and the keys.
    let dots_w = GUIDE_PAGES as i32 * 22;
    let mut dx = xi + pw / 2 - dots_w / 2;
    for i in 0..GUIDE_PAGES {
        let on = i == view.page;
        let r = if on { 6.5 } else { 4.5 };
        canvas.smooth_disc(
            (dx + 11) as f32,
            (yi + 836) as f32,
            r,
            a(if on {
                ui::argb(0xff, accent)
            } else {
                0x66ffffff
            }),
        );
        dx += 22;
    }
    ui::text_center(
        canvas,
        fonts,
        &format!(
            "{} {} / {}",
            t("Página", "Page"),
            view.page + 1,
            GUIDE_PAGES
        ),
        xi + pw / 2,
        yi + 866,
        15.0,
        a(0xa0ffffff),
        true,
    );
    for (rect, label, enabled) in [
        (GUIDE_PREV, "‹", view.page > 0),
        (GUIDE_NEXT, "›", view.page + 1 < GUIDE_PAGES),
    ] {
        let (rx, ry, rw, rh) = (rect.0, rect.1 + dy as i32, rect.2, rect.3);
        let over = enabled && crate::rect_contains((rx, ry, rw, rh), view.mouse.0, view.mouse.1);
        let fill = if !enabled {
            0x30ffffff
        } else if over {
            ui::argb(0xff, accent)
        } else {
            0xe0141019
        };
        ui::plate(
            canvas,
            rx as f32,
            ry as f32,
            rw as f32,
            rh as f32,
            12.0,
            a(fill),
        );
        ui::outline(
            canvas,
            &ui::plate_points(rx as f32, ry as f32, rw as f32, rh as f32, 12.0),
            1.5,
            a(if enabled {
                ui::argb(0xff, accent)
            } else {
                0x40ffffff
            }),
        );
        ui::text_center(
            canvas,
            fonts,
            label,
            rx + rw / 2,
            ry + rh / 2 - 2,
            34.0,
            a(if enabled { 0xffffffff } else { 0x60ffffff }),
            true,
        );
    }
    ui::text_right(
        canvas,
        fonts,
        t("←→ página · ESC cerrar", "←→ page · ESC close"),
        xi + pw - 260,
        yi + 866,
        15.0,
        a(0xa0ffffff),
        true,
    );
}

/// One page of the guide, `shift` pixels aside and at `alpha` (0..1).
#[allow(clippy::too_many_arguments)]
fn paint_guide_page(
    canvas: &mut crate::render::Canvas,
    fonts: &crate::render::Fonts,
    accent: u32,
    language: Language,
    page: usize,
    art: Option<&crate::ArtBitmap>,
    mouse: (i32, i32),
    (x, y): (i32, i32),
    shift: f32,
    alpha: f32,
) {
    if alpha <= 0.01 {
        return;
    }
    let t = |p: (&'static str, &'static str)| tr(language, p.0, p.1);
    let a = |c: u32| {
        ui::argb(
            ((c >> 24) as f32 * alpha.clamp(0.0, 1.0)) as u32,
            c & 0x00ff_ffff,
        )
    };
    let x = x + shift.round() as i32;
    let soft = a(ui::argb(0xff, ui::mix(accent, 0xffffff, 0.55)));
    let heading = |canvas: &mut crate::render::Canvas, text: &str, hx: i32, hy: i32| {
        ui::text_mid(
            canvas,
            fonts,
            text,
            hx,
            hy,
            22.0,
            a(ui::argb(0xff, accent)),
            true,
        );
    };
    // The header: the tag, the title, a line under it.
    let (tag, title, sub) = match page.checked_sub(1).and_then(|i| GUIDE.get(i)) {
        None => (
            t(("GUÍA", "GUIDE")),
            t(("Antes de empezar", "Before you start")),
            t((
                "Demon Queen está en desarrollo.",
                "Demon Queen is in development.",
            )),
        ),
        Some(g) => (t(("GUÍA", "GUIDE")), t(g.name), t(g.place)),
    };
    ui::tag(
        canvas,
        fonts,
        x + 70,
        y + 34,
        tag,
        16.0,
        ui::ABYSS,
        ui::BONE,
    );
    let size = ui::fit_size(fonts, title, 1300, 56.0, false).min(56.0);
    canvas.display_text(fonts, title, x + 70 + 3, y + 84 + 3, size, a(0xff000000));
    canvas.display_text(fonts, title, x + 70, y + 84, size, a(0xffffffff));
    ui::text_mid(canvas, fonts, sub, x + 74, y + 172, 20.0, soft, true);
    canvas.line(
        ((x + 70) as f32, (y + 198) as f32),
        ((x + 1500) as f32, (y + 194) as f32),
        2.0,
        a(0x55ffffff),
    );

    let Some(g) = page.checked_sub(1).and_then(|i| GUIDE.get(i)) else {
        // The cover: the index of the places (each opens its page) and the notes on this build.
        let (lx, ly) = (x + 70, y + 230);
        heading(
            canvas,
            t(("SITIOS DE INTERÉS", "PLACES OF INTEREST")),
            lx,
            ly,
        );
        for (i, place) in GUIDE.iter().enumerate() {
            let (rx, ry, rw, rh) = guide_index_rect(i);
            let (rx, ry) = (rx + (x - GUIDE_PLATE.0), ry + (y - GUIDE_PLATE.1));
            let over = crate::rect_contains((rx, ry, rw, rh), mouse.0, mouse.1);
            if over {
                ui::plate(
                    canvas,
                    rx as f32,
                    ry as f32,
                    rw as f32,
                    rh as f32,
                    10.0,
                    a(ui::argb(0x40, accent)),
                );
            }
            let cy = ry + rh / 2;
            canvas.smooth_disc((rx + 14) as f32, cy as f32, 5.0, a(ui::argb(0xff, accent)));
            let nw = ui::text_mid(
                canvas,
                fonts,
                t(place.name),
                rx + 32,
                cy,
                21.0,
                a(0xffffffff),
                true,
            );
            if place.animated {
                let cx = rx + 32 + nw + 14;
                let label = t(("ANIMADO", "ANIMATED"));
                let lw = fonts.text_width(label, 13.0, true) + 20;
                ui::plate(
                    canvas,
                    cx as f32,
                    (cy - 12) as f32,
                    lw as f32,
                    24.0,
                    6.0,
                    a(ui::argb(0xff, accent)),
                );
                ui::text_mid(canvas, fonts, label, cx + 10, cy, 13.0, a(0xffffffff), true);
            }
            ui::text_right(
                canvas,
                fonts,
                &format!("{}", i + 2),
                rx + rw - 12,
                cy,
                17.0,
                a(0x90ffffff),
                true,
            );
        }
        ui::text_mid(
            canvas,
            fonts,
            t((
                "Clic en un sitio para abrir su página.",
                "Click a place to open its page.",
            )),
            lx,
            y + 352 + GUIDE.len() as i32 * 56 + 14,
            16.0,
            a(0xa0ffffff),
            false,
        );
        let (rx, mut ry) = (x + 860, y + 230);
        heading(
            canvas,
            t(("NOTAS DE DESARROLLO", "DEVELOPMENT NOTES")),
            rx,
            ry,
        );
        ry += 34;
        for note in NOTES {
            canvas.convex(
                &[
                    ((rx + 2) as f32, (ry - 5) as f32),
                    ((rx + 12) as f32, (ry - 5) as f32),
                    ((rx + 8) as f32, (ry + 5) as f32),
                    ((rx - 2) as f32, (ry + 5) as f32),
                ],
                a(ui::argb(0xff, accent)),
            );
            let bottom = canvas.paragraph(
                fonts,
                note,
                rx + 24,
                ry - 12,
                640,
                27,
                18.0,
                a(0xeef1e9f5),
                false,
            );
            ry = bottom + 18;
        }
        return;
    };

    // A place: its picture on the right, what is there and what it asks on the left.
    let (lx, mut ly) = (x + 70, y + 232);
    let left_w = if g.picture.is_some() { 700 } else { 1400 };
    heading(canvas, t(("QUÉ HAY", "WHAT'S THERE")), lx, ly);
    if g.animated {
        let label = t(("ANIMADO", "ANIMATED"));
        let hw = fonts.text_width(t(("QUÉ HAY", "WHAT'S THERE")), 22.0, true);
        let lw = fonts.text_width(label, 13.0, true) + 20;
        ui::plate(
            canvas,
            (lx + hw + 16) as f32,
            (ly - 12) as f32,
            lw as f32,
            24.0,
            6.0,
            a(ui::argb(0xff, accent)),
        );
        ui::text_mid(
            canvas,
            fonts,
            label,
            lx + hw + 26,
            ly,
            13.0,
            a(0xffffffff),
            true,
        );
    }
    ly += 30;
    for line in g.what {
        canvas.smooth_disc(
            (lx + 6) as f32,
            (ly + 1) as f32,
            4.0,
            a(ui::argb(0xff, accent)),
        );
        let bottom = canvas.paragraph(
            fonts,
            t(*line),
            lx + 22,
            ly - 12,
            left_w,
            27,
            18.0,
            a(0xeef1e9f5),
            false,
        );
        ly = bottom + 14;
    }
    if !g.needs.is_empty() {
        ly += 14;
        // What it asks of her, in the warm colour of a condition, each with its key.
        let warm = 0xffffc36b;
        ui::text_mid(
            canvas,
            fonts,
            t(("CONDICIONES", "CONDITIONS")),
            lx,
            ly,
            22.0,
            a(warm),
            true,
        );
        ly += 30;
        for need in g.needs {
            let (kx, ky) = (lx as f32, ly as f32);
            canvas.convex(
                &[
                    (kx + 6.0, ky - 7.0),
                    (kx + 13.0, ky),
                    (kx + 6.0, ky + 7.0),
                    (kx - 1.0, ky),
                ],
                a(warm),
            );
            let bottom = canvas.paragraph(
                fonts,
                t(*need),
                lx + 22,
                ly - 12,
                left_w,
                27,
                18.0,
                a(0xfff6e2c4),
                false,
            );
            ly = bottom + 14;
        }
    }
    if let (Some(_), Some(art)) = (g.picture, art) {
        // The picture: a shadow under it, a thin frame of the accent and its bar on the left.
        let (fx, fy, fw, fh) = (x + 800, y + 232, 700, 394);
        canvas.rect(fx + 10, fy + 10, fw, fh, a(0x90000000));
        canvas.blit_rgba_scaled(fx, fy, fw, fh, art, alpha.clamp(0.0, 1.0));
        let frame = a(ui::argb(0xff, accent));
        canvas.rect(fx - 2, fy - 2, fw + 4, 2, frame);
        canvas.rect(fx - 2, fy + fh, fw + 4, 2, frame);
        canvas.rect(fx + fw, fy - 2, 2, fh + 4, frame);
        canvas.rect(fx - 8, fy - 2, 8, fh + 4, frame);
    }
}

/// An option's plate (as the pause menu's, bigger): a slanted black plate, bone-white with dark
/// words in focus with a glint of the accent running over it; Salir's accent is its warning red.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plate(
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
pub(crate) fn pointer(canvas: &mut crate::render::Canvas, tip: (f32, f32), size: f32) {
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
    fn the_guide_turns_its_pages_by_key_wheel_and_click_and_closes_outside() {
        let mut app = App::default();
        app.screen = Screen::Title;
        app.title_info_toggle();
        assert!(app.title_info);
        assert_eq!(app.title_info_page, 0);
        // Back from the cover goes nowhere; on, page by page.
        app.title_key(KeyCode::ArrowLeft);
        assert_eq!(app.title_info_page, 0);
        app.title_key(KeyCode::ArrowRight);
        assert_eq!(app.title_info_page, 1);
        assert!(app
            .title_info_turn
            .is_some_and(|(from, _, dir)| from == 0 && dir == 1));
        app.title_scroll(false);
        assert_eq!(app.title_info_page, 2);
        app.title_scroll(true);
        assert_eq!(app.title_info_page, 1);
        // The arrows.
        let (x, y, w, h) = GUIDE_NEXT;
        app.title_click(x + w / 2, y + h / 2);
        assert_eq!(app.title_info_page, 2);
        let (x, y, w, h) = GUIDE_PREV;
        app.title_click(x + w / 2, y + h / 2);
        assert_eq!(app.title_info_page, 1);
        // The cover's index opens a place's page.
        app.title_key(KeyCode::Home);
        assert_eq!(app.title_info_page, 0);
        let (x, y, w, h) = guide_index_rect(3);
        app.title_click(x + w / 2, y + h / 2);
        assert_eq!(app.title_info_page, 4);
        // Enter walks on and closes it past the last page.
        for _ in 0..GUIDE_PAGES {
            app.title_key(KeyCode::Enter);
        }
        assert!(!app.title_info);
        // Opened again it is on its cover; a click outside the plate closes it.
        app.title_info_toggle();
        assert_eq!(app.title_info_page, 0);
        app.title_click(60, 60);
        assert!(!app.title_info);
    }

    #[test]
    fn every_guide_picture_is_there() {
        for page in &GUIDE {
            if let Some(key) = page.picture {
                assert!(crate::assets_root().join(key).is_file(), "{key}");
            }
        }
    }

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

/// The title's backdrop over its place: a shade from the left, the band down the left in a deep
/// shade of `accent` (shadow, a lighter slash, halftone at its foot, a white hairline along its
/// edge), the embers and the seals. `slide` moves the band out to the left as it comes in.
pub(crate) fn paint_band(
    canvas: &mut crate::render::Canvas,
    accent: u32,
    slide: f32,
    clock: f32,
    motion: bool,
) {
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
}

/// The options' list down the band: the settings, the gallery, back to the title.
const OPTION_ITEMS: [(&str, &str, Icon); 3] = [
    ("Ajustes", "Settings", Icon::Gear),
    ("Galería", "Gallery", Icon::Memory),
    ("Volver", "Back", Icon::Leave),
];

/// The settings panel's place on the options screen.
pub(crate) const OPTIONS_AREA: crate::settings_menu::Area = (690, 176, 1150);

impl App {
    /// The options screen comes up with the keyboard on the settings' tabs.
    pub(crate) fn options_reset(&mut self) {
        self.options_sel = 0;
        self.options_panel = true;
        self.options_settings = Default::default();
        self.options_since = Instant::now();
    }

    fn options_activate(&mut self, index: usize) {
        match index {
            0 => {
                self.options_panel = true;
                self.options_settings.row = None;
                crate::exploration3d::ui_sfx::play(crate::exploration3d::ui_sfx::UiSfx::Select);
            }
            1 => self.open_gallery(Screen::Options),
            _ => self.screen = self.options_return,
        }
        self.needs_paint = true;
    }

    pub(crate) fn options_key(&mut self, key: KeyCode) {
        if key == KeyCode::Escape {
            self.screen = self.options_return;
            self.needs_paint = true;
            return;
        }
        let tab_key = matches!(
            key,
            KeyCode::KeyQ | KeyCode::KeyE | KeyCode::PageUp | KeyCode::PageDown
        );
        if self.options_panel || tab_key {
            self.options_panel = true;
            self.options_sel = 0;
            let outcome = crate::settings_menu::key(&mut self.options_settings, key);
            if outcome == crate::settings_menu::Outcome::Leave {
                self.options_panel = false;
            }
            self.settings_outcome(outcome);
            return;
        }
        let count = OPTION_ITEMS.len();
        match key {
            KeyCode::ArrowUp | KeyCode::KeyW => {
                self.options_sel = (self.options_sel + count - 1) % count
            }
            KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::Tab => {
                self.options_sel = (self.options_sel + 1) % count
            }
            KeyCode::ArrowRight | KeyCode::KeyD if self.options_sel == 0 => {
                self.options_activate(0)
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                self.options_activate(self.options_sel)
            }
            _ => {}
        }
        self.needs_paint = true;
    }

    pub(crate) fn options_click(&mut self, x: i32, y: i32) {
        if let Some(i) = (0..OPTION_ITEMS.len()).find(|&i| {
            let r = item_rect(i);
            x >= r.0 && x < r.0 + r.2 && y >= r.1 && y < r.1 + r.3
        }) {
            self.options_sel = i;
            self.options_activate(i);
            return;
        }
        if let Some(outcome) =
            crate::settings_menu::click(&mut self.options_settings, OPTIONS_AREA, x, y)
        {
            self.options_panel = true;
            self.options_sel = 0;
            self.settings_outcome(outcome);
        }
    }

    /// The wheel: through the rows in the panel, through the list on the band.
    pub(crate) fn options_scroll(&mut self, up: bool) {
        if self.options_panel {
            let key = if up {
                KeyCode::ArrowUp
            } else {
                KeyCode::ArrowDown
            };
            let outcome = crate::settings_menu::key(&mut self.options_settings, key);
            if outcome == crate::settings_menu::Outcome::Leave {
                return;
            }
            self.settings_outcome(outcome);
        } else {
            let count = OPTION_ITEMS.len();
            self.options_sel = (self.options_sel + if up { count - 1 } else { 1 }) % count;
            self.needs_paint = true;
        }
    }

    /// The options in the title's manner: its place still behind (its loop, when the reel has
    /// one), the band with the list down the left, and the settings panel on the right.
    pub(crate) fn paint_options_persona(&mut self) {
        let motion = !self.reduced_motion;
        let age = self.options_since.elapsed().as_secs_f32();
        let clock = self.title_since.elapsed().as_secs_f32();
        let current = self.title_background_current.min(ITEMS.len() - 1);
        self.canvas.clear(0xff0b0710);
        // The clip's frame, already scaled on its thread to the size it takes here when it came
        // so (the same pixels as scaling it now).
        let moving = match self.title_reel.as_mut() {
            Some(reel) => match reel.frame().map(|f| (f.w, f.h)) {
                Some((w, h)) => {
                    let size = self.canvas.physical_size(0, 0, w, h);
                    reel.set_target(size);
                    let copied = reel
                        .shown_scaled(size)
                        .is_some_and(|scaled| self.canvas.blit_prescaled(0, 0, w, h, scaled));
                    if !copied {
                        if let Some(frame) = reel.frame() {
                            self.canvas.blit_rgba(0, 0, frame);
                        }
                    }
                    true
                }
                None => false,
            },
            None => false,
        };
        if !moving {
            if let Some(b) = self.art.original(PLACES[current].0) {
                self.canvas.blit_rgba(0, 0, b);
            }
        }
        if self.ui_hidden {
            return;
        }
        let light = self.title_light(current);
        let accent = ui::accent(light);
        let language = self.localizer.language();
        let t = |es, en| tr(language, es, en);
        let enter = if motion {
            ui::entrance(age, 0.0, ENTER)
        } else {
            1.0
        };
        let panel_in = if motion {
            ui::entrance(age, 0.12, 0.34)
        } else {
            1.0
        };
        let slide = 620.0 * (1.0 - enter);
        let (options_sel, options_panel, mouse) =
            (self.options_sel, self.options_panel, self.mouse);
        let gallery_count = format!("{}/{}", self.gallery.entries.len(), self.gallery.total);
        let canvas = &mut self.canvas;

        // ── The place goes dim on the right, where the panel reads.
        canvas.rect(0, 0, 1920, 1080, 0x7008040e);
        paint_band(canvas, accent, slide, clock, motion);
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };

        // ── The screen's name, as the title's is.
        let sx = -(slide.round() as i32);
        let title = t("Opciones", "Options");
        let size = ui::fit_size(fonts, title, 460, 80.0, false);
        canvas.display_text(fonts, title, 72 + sx + 5, 74 + 5, size, 0xff000000);
        canvas.display_text(fonts, title, 72 + sx, 74, size, 0xffffffff);
        ui::tag(
            canvas,
            fonts,
            80 + sx,
            182,
            t("A vuestro gusto, Majestad", "As you please, Majesty"),
            15.0,
            ui::ABYSS,
            ui::BONE,
        );
        canvas.line(
            (80.0 - slide, 246.0),
            (470.0 - slide, 236.0),
            2.0,
            0xccffffff,
        );
        ui::text_mid(
            canvas,
            fonts,
            &format!("VERSIÓN {} · EN DESARROLLO", env!("CARGO_PKG_VERSION")),
            82 + sx,
            282,
            15.0,
            0xddffffff,
            true,
        );

        // ── The list, as the title's options; the one in focus with the pointer at it.
        for (index, (es, en, icon)) in OPTION_ITEMS.iter().enumerate() {
            let rect = item_rect(index);
            let over = mouse.0 >= rect.0
                && mouse.0 < rect.0 + rect.2
                && mouse.1 >= rect.1
                && mouse.1 < rect.1 + rect.3;
            let keyed = !options_panel && options_sel == index;
            let selected = keyed || over || (index == 0 && options_panel);
            let come = if motion {
                ui::entrance(age, 0.10 + index as f32 * 0.06, 0.30)
            } else {
                1.0
            };
            let dx = -((rect.0 + rect.2 + 90) as f32) * (1.0 - come);
            let label = if index == 1 {
                format!("{} · {gallery_count}", t(es, en))
            } else {
                t(es, en).to_string()
            };
            plate(
                canvas,
                fonts,
                rect,
                dx,
                &label,
                *icon,
                true,
                selected,
                false,
                accent,
                clock,
                motion,
                over && self.mouse_down,
            );
            if (keyed || (index == 0 && options_panel)) && come > 0.99 {
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

        // ── The settings, sliding in from the right.
        let area = (
            OPTIONS_AREA.0 + (260.0 * (1.0 - panel_in)) as i32,
            OPTIONS_AREA.1,
            OPTIONS_AREA.2,
        );
        crate::settings_menu::paint(
            canvas,
            fonts,
            &self.options_settings,
            area,
            &crate::settings_menu::Shown {
                language,
                reduced_motion: self.reduced_motion,
                light,
                mouse,
                keyboard: options_panel,
            },
        );

        // ── The keys.
        ui::key_hints(
            canvas,
            fonts,
            80 + sx,
            1010,
            &[
                ("↑↓", t("elegir", "choose")),
                ("←→", t("cambiar", "change")),
                ("Q E", t("pestaña", "tab")),
                ("ESC", t("volver", "back")),
            ],
        );
    }
}
