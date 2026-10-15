//! El mago del bosque: su puesto, en la manera de los menús de Persona. Al pulsar F junto a él en su
//! claro se abre: Hablar (una charla corta, como una escena de novela visual, que siempre acaba en
//! su pregunta), Comprar (sus sales despertadoras contra su propio anillo, un amuleto de latón y un
//! péndulo de práctica que no hace nada) e Irse. A «¿Quiere probar la hipnosis?» se contesta «Uhm…
//! no demasiado» (y la ataca a traición con el anillo: empieza su minijuego, `hipno.rs`) o «No, para
//! nada» (y se aguanta, por ahora). Recuerda cómo acabó la última vez (`mago_ultima`) y lo saca.
use super::ui_sfx::{self, UiSfx};
use super::*;

/// The page on show.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Page {
    Main,
    Talk,
    Shop,
    /// He springs the ring on her: a moment of it, then his game.
    Ambush,
}

/// Who says a line of the talk.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Who {
    Mage,
    Queen,
    /// The Queen, only thinking it.
    Thought,
}

type Line = (Who, &'static str);

/// What he sells: its id (the bag's), name, what it is, what it does, price, how many she can
/// carry, his sales pitch and what Chiquita thinks of it.
pub(super) struct Ware {
    pub(super) id: &'static str,
    pub(super) name: &'static str,
    pub(super) text: &'static str,
    pub(super) effect: &'static str,
    pub(super) price: u32,
    most: u32,
    pitch: &'static str,
    full: &'static str,
    chiquita: (&'static str, mascot::Cue),
}

pub(super) const WARES: [Ware; 3] = [
    Ware {
        id: play_scene::SALTS,
        name: "Sales despertadoras",
        text: "Un frasquito que pica en la nariz hasta las lágrimas.",
        effect: "En su hipnosis, Q: despiertas de golpe.",
        price: 15,
        most: 5,
        pitch: "—Para despertar a los clientes que no pagan. Jajaja.",
        full: "—¿Más sales? Se me va a quedar sin nariz, Majestad.",
        chiquita: (
            "—¿Le compras a ÉL las sales contra ÉL? …Bueno. Tiene su lógica.",
            mascot::Cue::Flustered,
        ),
    },
    Ware {
        id: play_scene::AMULET,
        name: "Amuleto del ojo cerrado",
        text: "Un ojo de latón, cerrado y un poco bizco.",
        effect: "Llevarlo encima basta: su anillo tira menos de tu mirada.",
        price: 45,
        most: 1,
        pitch: "—Funciona de maravilla. Contra otros hipnotistas.",
        full: "—Ya tiene uno. Dos no funcionan mejor. Bueno, sí, pero no se lo vendo.",
        chiquita: (
            "—Es de latón. Pero el latón también cuenta, supongo.",
            mascot::Cue::Talk,
        ),
    },
    Ware {
        id: "pendulo_practica",
        name: "Péndulo de práctica",
        text: "Una tuerca atada a un cordel. «Para practicar en casa».",
        effect: "Ninguno conocido. Chiquita dice que es una estafa.",
        price: 8,
        most: 1,
        pitch: "—Para practicar en casa. Con quien quiera. Jajaja.",
        full: "—Uno basta. Créame. Con ese ya puede hipnotizar a… a una piedra.",
        chiquita: (
            "—Te ha vendido una cuerda con una tuerca. Y tú se la has comprado.",
            mascot::Cue::Jealous,
        ),
    },
];

/// The talks, by how it went the last time (`mago_ultima`: 0 never, 1 she woke and punched him,
/// 2 she bit him, 3 she fell, 4 she ran, 5 she said no); each ends in his question.
fn script(last: i32, on_purpose: bool) -> &'static [Line] {
    match last {
        1 => &[
            (Who::Queen, "—¿Todavía te atreves a hablarme?"),
            (Who::Mage, "—Jajaja. Sin rencor, Majestad. Un artista acepta las críticas. Hasta las de puño."),
            (Who::Mage, "—¿Otra sesión? Le hago precio de amiga: gratis."),
        ],
        2 => &[
            (Who::Mage, "—Majestad. Antes de nada: ¿hoy viene con hambre?"),
            (Who::Queen, "—Depende de lo que me ofrezcas."),
            (Who::Mage, "—Solo el anillo. Solo mirar. Prometo no acercarle nada a la boca. …Casi nada."),
        ],
        3 if on_purpose => &[
            (Who::Mage, "—Usted mira el anillo con unas ganas… Jajaja."),
            (Who::Thought, "(Ni lo niego ni lo confirmo.)"),
            (Who::Mage, "—¿Repetimos? Hoy brilla más que nunca."),
        ],
        3 => &[
            (Who::Mage, "—¿Se acuerda de algo de la última vez?"),
            (Who::Queen, "—…No. ¿Debería?"),
            (Who::Mage, "—No, no. Jajaja. Nada importante. ¿Repetimos?"),
        ],
        4 => &[
            (Who::Mage, "—La otra vez salió corriendo. ¿Fue el anillo o fui yo?"),
            (Who::Queen, "—Fuiste tú."),
            (Who::Mage, "—Jajaja. Justo lo que pensaba. ¿Lo intentamos otra vez?"),
        ],
        5 => &[
            (Who::Mage, "—¿Sabe que el anillo tiene trescientos años? Lo heredé. De un tío. Que me lo vendió."),
            (Who::Thought, "(Trescientos años y sigue pareciendo de feria.)"),
            (Who::Mage, "—Bueno. ¿Ahora sí quiere probar?"),
        ],
        _ => &[
            (Who::Mage, "—Soy hipnotista. Ambulante. Sobre todo ambulante: en los pueblos me echan pronto."),
            (Who::Queen, "—¿Y qué hipnotizas, exactamente?"),
            (Who::Mage, "—Quito el hipo, el miedo y la vergüenza. Lo de la vergüenza se me da de maravilla."),
            (Who::Mage, "—¿Quiere probar? Gratis. Solo mirar el anillo… un ratito."),
        ],
    }
}

/// What he says when she comes to his stall.
fn greeting(last: i32) -> &'static str {
    match last {
        1 => "—Usted otra vez. …Aún me duele la nariz, Majestad.",
        2 => "—¡Ah! ¡Usted! …Esos dientes, lejos, ¿eh? Jajaja.",
        3 => "—¡Majestad! ¿Durmió bien la última vez? Jajaja.",
        4 => "—¡La fugitiva! Jajaja. Pase, pase, que no muerdo.",
        5 => "—¿Ha cambiado de opinión? El anillo la ha echado de menos.",
        _ => "—¡Oh! ¡Una visitante! Jajaja. Pase, pase. Bienvenida a mi consulta.",
    }
}

const QUESTION: &str = "—¿Quiere probar la hipnosis, Majestad?";
const ANSWERS: [&str; 2] = ["Uhm… no demasiado", "No, para nada"];
/// How long his ambush takes before his game opens.
const AMBUSH_SECS: f32 = 2.2;

/// The stall while it is open.
pub(super) struct MageMenu {
    pub(super) page: Page,
    pub(super) choice: usize,
    page_at: f32,
    opened: f32,
    /// The talk: its lines, the one on show and since when (it types itself out), and whether the
    /// question is up (after the last line).
    script: &'static [Line],
    line: usize,
    line_at: f32,
    asking: bool,
    /// The ambush's or the refusal's own lines, after the question.
    after: &'static [Line],
    /// What he said last on the main page or the shop.
    said: Option<(&'static str, f32)>,
    /// What she just bought, and when.
    bought: Option<(usize, f32)>,
}

impl MageMenu {
    fn show(&mut self, page: Page, now: f32) {
        self.page = page;
        self.choice = 0;
        self.page_at = now;
    }
}

impl App {
    /// F beside him: his stall opens.
    pub(super) fn open_mage(&mut self) {
        let now = self.elapsed;
        let last = self.story.get("mago_ultima");
        self.mage = Some(MageMenu {
            page: Page::Main,
            choice: 0,
            page_at: now,
            opened: now,
            script: &[],
            line: 0,
            line_at: now,
            asking: false,
            after: &[],
            said: Some((greeting(last), now + 0.35)),
            bought: None,
        });
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
        ui_sfx::play(UiSfx::Open);
        if self.story.get("mago_visto") == 0 {
            self.story.values.insert("mago_visto".to_owned(), 1);
            self.float_chiquita_after(
                "—Ese viejo no me gusta. Tiene cara de vender anillos.",
                mascot::Cue::Jealous,
                0.9,
            );
        }
    }

    pub(super) fn close_mage(&mut self) {
        if self.mage.is_some() {
            ui_sfx::play(UiSfx::Close);
        }
        self.mage = None;
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// The options of the page on show: label, hint, and for the wares their price and how many
    /// she has.
    fn mage_options(&self) -> Vec<map_overlay::MageOption> {
        let Some(m) = self.mage.as_ref() else {
            return Vec::new();
        };
        let opt = |label: &str, hint: &str, icon: u8| map_overlay::MageOption {
            label: label.to_owned(),
            hint: hint.to_owned(),
            icon,
            ..Default::default()
        };
        match m.page {
            Page::Main => vec![
                opt(
                    "Hablar",
                    "Una charla corta. Él siempre acaba en lo mismo.",
                    0,
                ),
                opt("Comprar", "Lo que vende: sales, amuletos… y baratijas.", 1),
                opt("Irse", "Volver al bosque. Sin mirarle el anillo.", 2),
            ],
            Page::Talk if m.asking => ANSWERS
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    opt(
                        a,
                        if i == 0 {
                            "Ni sí ni no. …Más bien sí."
                        } else {
                            "Un no como una casa."
                        },
                        3 + i as u8,
                    )
                })
                .collect(),
            Page::Shop => {
                let mut list: Vec<_> = WARES
                    .iter()
                    .enumerate()
                    .map(|(i, w)| {
                        let owned = self.inventory.get(w.id).copied().unwrap_or(0);
                        map_overlay::MageOption {
                            label: w.name.to_owned(),
                            hint: w.text.to_owned(),
                            icon: 10 + i as u8,
                            price: w.price,
                            owned,
                            locked: owned >= w.most,
                            effect: w.effect.to_owned(),
                        }
                    })
                    .collect();
                list.push(opt("Volver", "A su puesto.", 2));
                list
            }
            _ => Vec::new(),
        }
    }

    /// The line of the talk on show, if any: who, the words.
    fn mage_line(&self) -> Option<(Who, &'static str)> {
        let m = self.mage.as_ref()?;
        if m.page != Page::Talk {
            return None;
        }
        if m.asking {
            return Some((Who::Mage, QUESTION));
        }
        if m.line < m.script.len() {
            return Some(m.script[m.line]);
        }
        let k = m.line - m.script.len();
        m.after.get(k).copied()
    }

    /// How many of its letters are out by now.
    fn mage_shown(&self, text: &str) -> usize {
        let at = self.mage.as_ref().map_or(0.0, |m| m.line_at);
        // (Counted in the player's language: the overlay types the translated line.)
        (((self.elapsed - at).max(0.0) * crate::exploration3d::prefs::text_rate(52.0)) as usize)
            .min(demon_queen_dungeon::i18n::tr(text).chars().count())
    }

    /// F, Enter or a click in the talk: the whole line at once, else the next; or the question.
    fn mage_advance(&mut self) {
        let now = self.elapsed;
        let Some((_, text)) = self.mage_line() else {
            return;
        };
        if self.mage_shown(text) < demon_queen_dungeon::i18n::tr(text).chars().count() {
            if let Some(m) = self.mage.as_mut() {
                m.line_at = -1.0e6;
            }
            return;
        }
        let Some(m) = self.mage.as_mut() else {
            return;
        };
        if m.asking {
            return;
        }
        m.line += 1;
        m.line_at = now;
        let total = m.script.len() + m.after.len();
        if m.line == m.script.len() && m.after.is_empty() {
            // The question.
            m.asking = true;
            m.choice = 0;
            m.line_at = now;
        } else if !m.after.is_empty() && m.line >= total {
            // The ambush's lines or the refusal's are over.
            let ambush = m.choice == 0;
            if ambush {
                m.show(Page::Ambush, now);
                let first = self.story.get("mago_emboscadas") == 0;
                *self
                    .story
                    .values
                    .entry("mago_emboscadas".to_owned())
                    .or_insert(0) += 1;
                let line = if first {
                    "—¡COBARDE! ¡A traición no se hipnotiza a nadie!"
                } else {
                    "—¡¿OTRA VEZ A TRAICIÓN?! ¡Cobarde de feria!"
                };
                self.float_chiquita_after(line, mascot::Cue::Scold, 0.25);
                self.ui_sound(dungeon::sound::Sfx::SpellBurst, 0.7);
            } else {
                self.story.values.insert("mago_ultima".to_owned(), 5);
                *self
                    .story
                    .values
                    .entry("mago_rechazos".to_owned())
                    .or_insert(0) += 1;
                if let Some(m) = self.mage.as_mut() {
                    m.show(Page::Main, now);
                    m.said = Some((
                        "—Bueno. Si cambia de opinión, ya sabe dónde estoy. Jajaja.",
                        now,
                    ));
                }
                self.float_chiquita_after(
                    "—Bien dicho. Y ni se te ocurra darle la espalda.",
                    mascot::Cue::Scold,
                    0.3,
                );
            }
        }
    }

    /// Her answer to his question.
    fn mage_answer(&mut self, pick: usize) {
        let now = self.elapsed;
        let again = self.story.get("mago_emboscadas") > 0;
        let Some(m) = self.mage.as_mut() else {
            return;
        };
        if !m.asking {
            return;
        }
        m.asking = false;
        m.choice = pick;
        m.line = m.script.len();
        m.line_at = now;
        m.after = if pick == 0 {
            if again {
                &[
                    (
                        Who::Mage,
                        "—«No demasiado», otra vez. Jajaja. Me encanta ese «no demasiado».",
                    ),
                    (Who::Mage, "—¡MIRE EL ANILLO!"),
                ]
            } else {
                &[
                    (
                        Who::Mage,
                        "—¿«No demasiado»? …Eso es un «un poquito». ¡Y un poquito es un SÍ!",
                    ),
                    (Who::Mage, "—¡MIRE EL ANILLO!"),
                ]
            }
        } else {
            &[
                (Who::Mage, "—Jajaja. Claro, claro. Respeto mucho un «no»."),
                (Who::Mage, "—…Por ahora."),
            ]
        };
    }

    /// Enter on the option lit.
    fn mage_pick(&mut self, pick: usize) {
        let now = self.elapsed;
        let Some(page) = self.mage.as_ref().map(|m| m.page) else {
            return;
        };
        let n = self.mage_options().len();
        if pick >= n && !(page == Page::Talk && !self.mage.as_ref().is_some_and(|m| m.asking)) {
            return;
        }
        match page {
            Page::Main => match pick {
                0 => {
                    let last = self.story.get("mago_ultima");
                    let on_purpose = self.story.get("mago_a_proposito") > 0;
                    if let Some(m) = self.mage.as_mut() {
                        m.show(Page::Talk, now);
                        m.script = script(last, on_purpose);
                        m.after = &[];
                        m.line = 0;
                        m.line_at = now + 0.3;
                        m.asking = false;
                    }
                }
                1 => {
                    if let Some(m) = self.mage.as_mut() {
                        m.show(Page::Shop, now);
                        m.said = Some(("—Mire, mire. Todo auténtico. Casi todo.", now + 0.3));
                    }
                }
                _ => self.close_mage(),
            },
            Page::Talk => {
                if let Some(m) = self.mage.as_ref().filter(|m| m.asking) {
                    // A moment for the question to land: F mashed through the lines does not answer it.
                    if now - m.line_at >= 0.6 {
                        self.mage_answer(pick.min(1));
                    }
                } else {
                    self.mage_advance();
                }
            }
            Page::Shop => {
                if pick >= WARES.len() {
                    if let Some(m) = self.mage.as_mut() {
                        m.show(Page::Main, now);
                        m.choice = 1;
                    }
                    return;
                }
                self.mage_buy(pick);
            }
            Page::Ambush => {}
        }
    }

    /// Buying ware `i`.
    fn mage_buy(&mut self, i: usize) {
        let now = self.elapsed;
        let w = &WARES[i];
        let owned = self.inventory.get(w.id).copied().unwrap_or(0);
        let said = if owned >= w.most {
            w.full
        } else if self.gold < w.price {
            "—Sin oro no hay magia, Majestad. Es la primera ley."
        } else {
            self.gold -= w.price;
            *self.inventory.entry(w.id.to_owned()).or_insert(0) += 1;
            let mut pack = self.backpack.clone();
            pack.reconcile(&self.inventory);
            self.backpack = pack;
            self.ui_sound(dungeon::sound::Sfx::Coins, 0.7);
            if let Some(m) = self.mage.as_mut() {
                m.bought = Some((i, now));
            }
            let first = self.story.get(&format!("mago_compra_{i}")) == 0;
            if first {
                self.story.values.insert(format!("mago_compra_{i}"), 1);
                self.float_chiquita_after(w.chiquita.0, w.chiquita.1, 0.6);
            }
            self.persist_3d_state(true);
            w.pitch
        };
        if let Some(m) = self.mage.as_mut() {
            m.said = Some((said, now));
            m.choice = i;
        }
    }

    /// Esc: back a page, or away from his stall.
    fn mage_back(&mut self) {
        let now = self.elapsed;
        match self.mage.as_ref().map(|m| m.page) {
            Some(Page::Main) => self.close_mage(),
            Some(Page::Talk | Page::Shop) => {
                if let Some(m) = self.mage.as_mut() {
                    // Halfway through his ambush there is no going back.
                    if m.page == Page::Talk && !m.after.is_empty() && m.choice == 0 {
                        ui_sfx::play(UiSfx::Error);
                        return;
                    }
                    m.show(Page::Main, now);
                    ui_sfx::play(UiSfx::Back);
                }
            }
            _ => {}
        }
    }

    /// Each step: after his ambush, his game.
    pub(super) fn tick_mage(&mut self) {
        let now = self.elapsed;
        let Some(m) = self.mage.as_ref() else {
            return;
        };
        if m.page == Page::Ambush && now - m.page_at >= AMBUSH_SECS {
            self.mage = None;
            self.open_play_scene(play_scene::Game::Hipno);
            self.persist_3d_state(true);
        }
    }

    /// Keys while his stall is open.
    pub(super) fn mage_key(&mut self, code: KeyCode) {
        let Some(page) = self.mage.as_ref().map(|m| m.page) else {
            return;
        };
        if page == Page::Ambush {
            return;
        }
        let talking = page == Page::Talk && !self.mage.as_ref().is_some_and(|m| m.asking);
        let n = self.mage_options().len().max(1);
        let choice = self.mage.as_ref().map_or(0, |m| m.choice);
        match code {
            KeyCode::Escape | KeyCode::Backspace => self.mage_back(),
            _ if talking => {
                if matches!(
                    code,
                    KeyCode::KeyF | KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter
                ) {
                    self.mage_advance();
                }
            }
            KeyCode::ArrowUp | KeyCode::KeyW | KeyCode::ArrowLeft | KeyCode::KeyA => {
                if let Some(m) = self.mage.as_mut() {
                    m.choice = (choice + n - 1) % n;
                    ui_sfx::play(UiSfx::Move);
                }
            }
            KeyCode::ArrowDown | KeyCode::KeyS | KeyCode::ArrowRight | KeyCode::KeyD => {
                if let Some(m) = self.mage.as_mut() {
                    m.choice = (choice + 1) % n;
                    ui_sfx::play(UiSfx::Move);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyF => {
                ui_sfx::play(UiSfx::Select);
                self.mage_pick(choice)
            }
            KeyCode::Digit1 | KeyCode::Numpad1 => self.mage_pick(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => self.mage_pick(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => self.mage_pick(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => self.mage_pick(3),
            _ => {}
        }
    }

    /// A click: the option under the pointer (the lit one is taken; another is lit), or on in the talk.
    pub(super) fn mage_click(&mut self) {
        let Some((x, y)) = self.shop_pointer_position() else {
            return;
        };
        let Some(page) = self.mage.as_ref().map(|m| m.page) else {
            return;
        };
        if page == Page::Ambush {
            return;
        }
        let asking = self.mage.as_ref().is_some_and(|m| m.asking);
        if page == Page::Talk && !asking {
            self.mage_advance();
            return;
        }
        let kind = match page {
            Page::Shop => 2,
            Page::Talk => 1,
            _ => 0,
        };
        let n = self.mage_options().len();
        if let Some(i) =
            map_overlay::mage_hit(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32, kind, n, x, y)
        {
            if self.mage.as_ref().is_some_and(|m| m.choice == i) {
                ui_sfx::play(UiSfx::Select);
                self.mage_pick(i);
            } else if let Some(m) = self.mage.as_mut() {
                m.choice = i;
                ui_sfx::play(UiSfx::Move);
            }
        }
    }

    /// The stall for the overlay.
    pub(super) fn mage_view(&self) -> Option<map_overlay::MageView> {
        let m = self.mage.as_ref()?;
        let now = self.elapsed;
        let ms = |t: f32| ((now - t).max(0.0) * 1000.0) as u32;
        let line = self.mage_line().map(|(who, text)| map_overlay::MageLine {
            who: match who {
                Who::Mage => 0,
                Who::Queen => 1,
                Who::Thought => 2,
            },
            text: text.to_owned(),
            shown: self.mage_shown(text) as u16,
            age_ms: ms(m.line_at),
        });
        let queen_mood = match m.script.get(m.line).map(|l| l.0) {
            Some(Who::Thought) => Some("duda"),
            _ => None,
        };
        let face = |who: &str, mood: Option<&str>| {
            guion::portraits()
                .find(who, mood)
                .map(|p| map_overlay::PortraitRef {
                    image: p.image,
                    crop: p.crop,
                })
        };
        Some(map_overlay::MageView {
            page: match m.page {
                Page::Main => 0,
                Page::Talk => 1,
                Page::Shop => 2,
                Page::Ambush => 3,
            },
            choice: m.choice as u8,
            page_ms: ms(m.page_at),
            open_ms: ms(m.opened),
            options: self.mage_options(),
            said: m
                .said
                .filter(|(_, at)| now >= *at)
                .map(|(text, at)| (text.to_owned(), ms(at))),
            line,
            asking: m.asking,
            question: QUESTION.to_owned(),
            gold: self.gold,
            bought: m.bought.map(|(i, at)| (i as u8, ms(at))),
            mage: Some(map_overlay::PortraitRef {
                image: "reference/approved/oldman_mage3_nobg".to_owned(),
                crop: [62, 23, 900, 900],
            }),
            queen: face("gorou", queen_mood),
        })
    }

    /// His question is up (for the QA scripts).
    pub(super) fn mage_asking(&self) -> bool {
        self.mage.as_ref().is_some_and(|m| m.asking)
    }

    /// The stall is open (it takes the keys and the pointer).
    pub(super) fn mage_open(&self) -> bool {
        self.mage.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_talk_ends_in_his_question_and_his_wares_are_known_to_his_game() {
        for last in 0..=6 {
            for purpose in [false, true] {
                let s = script(last, purpose);
                assert!(!s.is_empty() && s.len() <= 4, "short talks: {last}");
                assert!(!greeting(last).is_empty());
            }
        }
        assert_eq!(WARES[0].id, play_scene::SALTS);
        assert_eq!(WARES[1].id, play_scene::AMULET);
        assert!(WARES.iter().all(|w| w.price > 0 && w.most >= 1));
    }
}
