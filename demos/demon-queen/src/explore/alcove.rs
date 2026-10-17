//! La alcoba de la Reina: su cama con dosel como un menú propio, a solas. En 3D ella se tumba en la
//! cama y la cámara se acerca de lado; en 2D, encima, el menú del castillo (luna, plata y violeta,
//! cartas como ventanas góticas): descansar, explorar el cuerpo (mirarse, tocarse, dejarse llevar:
//! la escena de la cama juega con ella tumbada detrás), dejar que el sello decida, y molestar a
//! Chiquita, que contesta en grande. Lo que se cuenta sale de los guiones de `castillo.md`.
use super::ui_sfx::{self, UiSfx};
use super::*;

/// The page on show.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(super) enum Page {
    Main,
    Explore,
    Chiquita,
    /// Who she asks to come to her room tonight.
    Invite,
}

/// Who may be asked to come at night: her name, the scene in `corte.md`, and the bond it needs.
pub(super) const GUESTS: [(&str, &str); 3] = [
    ("Lira", "noche_lira"),
    ("Nyxara", "noche_nyxara"),
    ("Maren", "noche_maren"),
];

/// The guests' faces on their cards (portrait and mood in retratos.json).
const GUEST_FACES: [(&str, &str); 3] = [
    ("lira", "sonrojada"),
    ("nyxara", "sonrojada"),
    ("maren", "sonrisa"),
];

impl App {
    /// Whether `guest` (an index of `GUESTS`) would come, and if not, why.
    pub(super) fn guest_ready(&self, guest: usize) -> Result<(), &'static str> {
        match guest {
            0 if self.story.get("maids_trust") >= 3 || self.story.get("afecto_lira") >= 1 => Ok(()),
            0 => Err("Aún no hay confianza: habla más con ella."),
            1 if self.story.get("afecto_nyxara") >= 2 => Ok(()),
            1 => Err("Aún no: aprende dos lecciones con ella."),
            // Maren only once she stays of her own will, with the door open, and is at home here.
            2 if self.story.get("prisionera_queda") == 0
                || self.story.get("prisionera_encerrada") > 0 =>
            {
                Err("No está en el castillo por su propio pie.")
            }
            2 if self.story.get("maren_afecto") >= 2 => Ok(()),
            2 => Err("Aún no: dale tiempo en la corte (la rutina de la mañana)."),
            _ => Err(""),
        }
    }
}

/// One way to tease Chiquita: its card, what the Queen does or says, what Chiquita answers, and how
/// fond of her it leaves Chiquita.
pub(super) struct Tease {
    id: &'static str,
    label: &'static str,
    hint: &'static str,
    icon: &'static str,
    queen: &'static str,
    /// The Queen only thinks it.
    thought: bool,
    reply: &'static str,
    /// Chiquita's flustered face.
    jealous: bool,
    fondness: i32,
    /// What she does about it (her 3D self).
    cue: mascot::Cue,
}

pub(super) const TEASES: [Tease; 6] = [
    Tease {
        id: "cosquillas",
        label: "Hacerle cosquillas al sello",
        hint: "Si el sello es ella… ¿tendrá cosquillas?",
        icon: "feather",
        queen: "Un dedo. Sobre las runas. Muy despacio.",
        thought: true,
        reply: "—¡¿Qué haces?! ¡Para! ¡PARA! …No sabía que tenía cosquillas. Te odio.",
        jealous: true,
        fondness: 1,
        cue: mascot::Cue::Flustered,
    },
    Tease {
        id: "celos",
        label: "Preguntarle si está celosa",
        hint: "De Nina. De Rosalía. De todas.",
        icon: "heartbreak",
        queen: "—¿Estás celosa de Nina, Chiquita?",
        thought: false,
        reply: "—¿Celosa? ¿Yo? ¿De una doncella con cofia y cero runas? …Un poco. Cállate.",
        jealous: true,
        fondness: 0,
        cue: mascot::Cue::Jealous,
    },
    Tease {
        id: "apodo",
        label: "Ponerle un apodo",
        hint: "Uno que le dé mucha vergüenza.",
        icon: "ribbon",
        queen: "—A partir de hoy te llamas «Chiqui-chan».",
        thought: false,
        reply: "—Ni se te ocurra. …¿«Chan» es bueno o malo? Suena a bueno. Sigo ofendida.",
        jealous: false,
        fondness: 1,
        cue: mascot::Cue::Flustered,
    },
    Tease {
        id: "chiste",
        label: "Contarle un chiste de tu otra vida",
        hint: "Humor de oficina. Va a sufrir.",
        icon: "laugh",
        queen: "—¿Por qué el oficinista llevó una escalera al trabajo? ¡Para subir de puesto!",
        thought: false,
        reply: "—…¿Qué es un oficinista? ¿Qué es un puesto? ¿Por qué me haces esto a estas horas?",
        jealous: false,
        fondness: 0,
        cue: mascot::Cue::Scold,
    },
    Tease {
        id: "roncar",
        label: "Fingir que duermes y roncar",
        hint: "Fuerte. Con efectos.",
        icon: "zzz",
        queen: "Zzz… rrronc… zzz… rrrrRONC…",
        thought: true,
        reply: "—Sé que no estás dormida. Te oigo pensar. Piensas FUERTE.",
        jealous: true,
        fondness: 0,
        cue: mascot::Cue::Scold,
    },
    Tease {
        id: "buenas_noches",
        label: "Darle las buenas noches",
        hint: "Esta vez sin trampa.",
        icon: "moonheart",
        queen: "—Buenas noches, Chiquita. Gracias por cuidarme.",
        thought: false,
        reply: "—…Buenas noches, tonta. Digo, Majestad. Digo… buenas noches.",
        jealous: false,
        fondness: 2,
        cue: mascot::Cue::Cheer,
    },
];

/// Where the canopy bed is (the castle plan's `cama`): the mattress's middle, its quilt's top,
/// and where her head rests (the pillows, toward −Z).
const BED: [f32; 2] = [-15.5, -20.05];
const BED_TOP: f32 = 0.9;
const PILLOW_Z: f32 = -20.95;

/// The night passing under black: when it is morning (s), and when she is up again.
const SLEEP_SWAP: f32 = 1.3;
const SLEEP_DONE: f32 = 2.8;

/// The bedroom while she is in bed.
pub(super) struct Alcove {
    pub(super) page: Page,
    pub(super) choice: usize,
    page_at: f32,
    started: f32,
    from: Vec3,
    from_yaw: f32,
    /// Where she lies and which way she faces lying.
    spot: Vec3,
    yaw: f32,
    /// A guion or the bed scene plays over her: the menu waits.
    busy: bool,
    /// The last tease and when.
    teased: Option<(usize, f32)>,
    /// Asleep since, whether the day has turned yet, and whether it was night when she slept.
    sleeping: Option<f32>,
    slept: bool,
    slept_at_night: bool,
}

/// A card as the overlay shows it.
pub(super) fn card(label: &str, hint: &str, icon: &str, locked: bool) -> map_overlay::AlcoveCard {
    map_overlay::AlcoveCard {
        label: label.to_owned(),
        hint: hint.to_owned(),
        icon: icon.to_owned(),
        face: None,
        locked,
    }
}

impl App {
    /// It is night now (the sky's clock).
    fn alcove_night(&self) -> bool {
        !(0.2..=0.8).contains(&self.environment_cycle.phase())
    }

    /// She lies down in her bed: the menu opens over the close shot.
    pub(super) fn open_alcove(&mut self) {
        let outfit = self.outfit;
        let lying = self.state.as_ref().and_then(|s| s.queen_lying(outfit));
        let Some((axis, head, low)) = lying else {
            // No lying clip: the bed's guion, as before.
            self.start_talk(None, Some("cama"));
            return;
        };
        // Her body along the bed, her head on the pillows (−Z), down its middle.
        let yaw = axis.x.atan2(-axis.z);
        let (s, c) = yaw.sin_cos();
        let head =
            Vec3::new(head.x * c + head.z * s, 0.0, -head.x * s + head.z * c) * QUEEN_WORLD_SCALE;
        let spot = Vec3::new(
            BED[0] - head.x,
            BED_TOP - low * QUEEN_WORLD_SCALE + 0.03,
            PILLOW_Z - head.z,
        );
        self.alcove = Some(Alcove {
            page: Page::Main,
            choice: 0,
            page_at: self.elapsed,
            started: self.elapsed,
            from: self.player,
            from_yaw: self.player_yaw,
            spot,
            yaw,
            busy: false,
            teased: None,
            sleeping: None,
            slept: false,
            slept_at_night: false,
        });
        self.queen_thought = None;
        self.map_open = false;
        self.clear_input();
    }

    /// The cards of the page on show.
    pub(super) fn alcove_cards(&self) -> Vec<map_overlay::AlcoveCard> {
        let Some(a) = self.alcove.as_ref() else {
            return Vec::new();
        };
        let deseo = self.story.get("deseo");
        let explored = self.story.get("explorado_hoy") >= 1;
        match a.page {
            Page::Main => vec![
                card(
                    "Descansar",
                    if self.alcove_night() {
                        "Dormir hasta la mañana."
                    } else {
                        "Una siesta hasta la noche."
                    },
                    "moon",
                    false,
                ),
                card(
                    "Explorar el cuerpo",
                    if explored {
                        "Ya te has explorado hoy. Duerme primero."
                    } else {
                        "Mirarte, tocarte… o dejarte llevar."
                    },
                    "hand",
                    explored,
                ),
                card(
                    "Dejar que el sello decida",
                    if explored {
                        "Por hoy ya ha comido. Duerme primero."
                    } else if deseo < 5 {
                        "Aún no tiene hambre bastante (Deseo 5)."
                    } else {
                        "Que mande él. Esta noche."
                    },
                    "seal",
                    explored || deseo < 5,
                ),
                card(
                    "Molestar a Chiquita",
                    "Solo un poquito. Se lo merece.",
                    "chiquita",
                    false,
                ),
                {
                    let night = self.alcove_night();
                    let done = self.story.get("noche_hoy") >= 1;
                    card(
                        "Invitar a alguien",
                        if !night {
                            "Solo de noche."
                        } else if done {
                            "Por esta noche ya está."
                        } else {
                            "Que alguien venga a tu cuarto esta noche."
                        },
                        "moonheart",
                        !night || done,
                    )
                },
                card("Levantarse", "Volver a la alcoba.", "back", false),
            ],
            Page::Explore => vec![
                card("Solo mirarse", "Con calma. Reconocerte.", "eye", false),
                card(
                    "Tocarse despacio",
                    "Que el sello coma un poco.",
                    "heart",
                    false,
                ),
                card(
                    "Dejarse llevar del todo",
                    if deseo < 3 {
                        "Aún no te atreves (Deseo 3)."
                    } else {
                        "Sin contenerte."
                    },
                    "flame",
                    deseo < 3,
                ),
                card("Volver", "A la cama.", "back", false),
            ],
            Page::Invite => {
                let mut cards: Vec<_> = GUESTS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| {
                        // Maren has no card before the Queen has met her.
                        let name = if i == 2 && self.story.get("prisionera_n") == 0 {
                            "· · ·"
                        } else {
                            name
                        };
                        let mut c = match self.guest_ready(i) {
                            Ok(()) => card(name, "Que venga esta noche.", "heart", false),
                            Err(why) => card(name, why, "heart", true),
                        };
                        // Her face in the card's window, once the Queen knows it.
                        if name != "· · ·" {
                            let (who, mood) = GUEST_FACES[i];
                            c.face = guion::portraits().find(who, Some(mood)).map(|p| {
                                map_overlay::PortraitRef {
                                    image: p.image,
                                    crop: p.crop,
                                }
                            });
                        }
                        c
                    })
                    .collect();
                cards.push(card("Volver", "A la cama.", "back", false));
                cards
            }
            Page::Chiquita => {
                let mut cards: Vec<_> = TEASES
                    .iter()
                    .map(|t| {
                        let done = self.story.get(&format!("chq_{}", t.id)) >= 1;
                        card(
                            t.label,
                            if done { "Por hoy ya basta." } else { t.hint },
                            t.icon,
                            done,
                        )
                    })
                    .collect();
                cards.push(card("Volver", "A la cama.", "back", false));
                cards
            }
        }
    }

    fn alcove_show(&mut self, page: Page) {
        let now = self.elapsed;
        if let Some(a) = self.alcove.as_mut() {
            a.page = page;
            a.choice = 0;
            a.page_at = now;
            a.teased = None;
        }
        if page == Page::Explore {
            self.float_chiquita_cue(
                "—Técnicamente es el mío. Yo me voy a mirar la pared.",
                mascot::Cue::Away,
            );
        }
    }

    /// Esc: back a page, or up from the bed.
    fn alcove_back(&mut self) {
        match self.alcove.as_ref().map(|a| a.page) {
            Some(Page::Main) => {
                ui_sfx::play(UiSfx::Close);
                self.close_alcove()
            }
            Some(_) => {
                ui_sfx::play(UiSfx::Back);
                self.alcove_show(Page::Main)
            }
            None => {}
        }
    }

    /// A guion of the bed's (`castillo.<scene>`) plays over her; the menu waits for it.
    fn alcove_tell(&mut self, scene: &str) {
        self.alcove_tell_at(&format!("castillo.{scene}"));
    }

    /// A guion scene (`file.scene`) plays over her; the menu waits for it.
    fn alcove_tell_at(&mut self, at: &str) {
        if let Some(a) = self.alcove.as_mut() {
            a.busy = true;
        }
        if !self.start_talk_at(at) {
            if let Some(a) = self.alcove.as_mut() {
                a.busy = false;
            }
        }
    }

    /// Enter on the card lit.
    pub(super) fn alcove_pick(&mut self, pick: usize) {
        let Some(page) = self.alcove.as_ref().map(|a| a.page) else {
            return;
        };
        let cards = self.alcove_cards();
        let Some(chosen) = cards.get(pick) else {
            return;
        };
        if let Some(a) = self.alcove.as_mut() {
            a.choice = pick;
        }
        if chosen.locked {
            ui_sfx::play(UiSfx::Error);
            return;
        }
        ui_sfx::play(UiSfx::Select);
        match (page, pick) {
            (Page::Main, 0) => {
                let night = self.alcove_night();
                if let Some(a) = self.alcove.as_mut() {
                    a.sleeping = Some(self.elapsed);
                    a.slept_at_night = night;
                }
            }
            (Page::Main, 1) => self.alcove_show(Page::Explore),
            (Page::Main, 2) => self.alcove_tell("cama_sello"),
            (Page::Main, 3) => self.alcove_show(Page::Chiquita),
            (Page::Main, 4) => self.alcove_show(Page::Invite),
            (Page::Main, _) => self.close_alcove(),
            (Page::Invite, i) if i < GUESTS.len() => {
                self.story.values.insert("noche_hoy".to_owned(), 1);
                self.alcove_tell_at(&format!("corte.{}", GUESTS[i].1));
            }
            (Page::Invite, _) => self.alcove_show(Page::Main),
            (Page::Explore, 0) => {
                self.story.values.insert("explorado_hoy".to_owned(), 1);
                *self.story.values.entry("deseo".to_owned()).or_insert(0) += 1;
                *self.story.values.entry("fear".to_owned()).or_insert(0) -= 1;
                self.alcove_tell("cama_mirar");
            }
            (Page::Explore, 1) => {
                self.story.values.insert("explorado_hoy".to_owned(), 1);
                self.alcove_tell("cama_tocar");
            }
            (Page::Explore, 2) => {
                self.story.values.insert("explorado_hoy".to_owned(), 1);
                self.alcove_tell("cama_todo");
            }
            (Page::Explore, _) => self.alcove_show(Page::Main),
            (Page::Chiquita, i) if i < TEASES.len() => {
                let t = &TEASES[i];
                self.story.values.insert(format!("chq_{}", t.id), 1);
                *self
                    .story
                    .values
                    .entry("chiquita_afecto".to_owned())
                    .or_insert(0) += t.fondness;
                let now = self.elapsed;
                if let Some(a) = self.alcove.as_mut() {
                    a.teased = Some((i, now));
                }
                self.float_chiquita_after(t.reply, t.cue, 0.45);
            }
            (Page::Chiquita, _) => self.alcove_show(Page::Main),
        }
        self.persist_3d_state(true);
    }

    /// Up from the bed: back where she stood.
    pub(super) fn close_alcove(&mut self) {
        let Some(a) = self.alcove.take() else {
            return;
        };
        self.player = a.from;
        self.player_yaw = a.from_yaw;
        self.camera_focus = a.from;
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// Each step: she stays in bed; the menu comes back once what played over her is done; the
    /// night passes when she sleeps.
    pub(super) fn tick_alcove(&mut self) {
        let now = self.elapsed;
        let free = self.talk.is_none() && !self.bed_scene_open() && self.caricias.is_none();
        let Some(a) = self.alcove.as_mut() else {
            return;
        };
        self.player = a.spot;
        self.player_yaw = a.yaw;
        if a.busy && free {
            a.busy = false;
            a.page = Page::Main;
            a.choice = 0;
            a.page_at = now;
        }
        if let Some(since) = a.sleeping {
            if !a.slept && now - since >= SLEEP_SWAP {
                a.slept = true;
                self.sleep_in_bed();
                for t in &TEASES {
                    self.story.values.insert(format!("chq_{}", t.id), 0);
                }
            }
            if now - since >= SLEEP_DONE {
                self.close_alcove();
            }
        }
    }

    /// Her lying pose drives the Queen's model.
    pub(super) fn alcove_lying(&self) -> bool {
        self.alcove.is_some()
    }

    /// The menu is up (not waiting under a guion or the bed scene).
    pub(super) fn alcove_menu_open(&self) -> bool {
        self.alcove.as_ref().is_some_and(|a| !a.busy)
    }

    /// The close shot: from the bed's open side, above, looking down along her.
    pub(super) fn alcove_camera(&self) -> Option<Camera3D> {
        let a = self.alcove.as_ref()?;
        let c = (self.elapsed - a.started).max(0.0);
        let ease = 1.0 - (1.0 - (c / 1.2).min(1.0)).powi(3);
        let target = Vec3::new(BED[0], BED_TOP + 0.15, BED[1] - 0.25);
        Some(Camera3D {
            position: Vec3::new(
                BED[0] + 2.6 - 0.25 * ease,
                2.35 - 0.15 * ease,
                BED[1] + 1.25 - 0.2 * ease,
            ),
            target,
            up: Vec3::Y,
            fov_y_radians: 40f32.to_radians(),
            orthographic_height: None,
            near: 0.05,
            far: 80.0,
        })
    }

    /// Keys while she is in bed: ←/→ choose, Enter/F/Space take, 1–7 take that card, Esc back.
    pub(super) fn alcove_key(&mut self, code: KeyCode) {
        if self.alcove.as_ref().is_some_and(|a| a.sleeping.is_some()) {
            return;
        }
        let n = self.alcove_cards().len().max(1);
        let choice = self.alcove.as_ref().map_or(0, |a| a.choice);
        match code {
            KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::ArrowUp | KeyCode::KeyW => {
                if let Some(a) = self.alcove.as_mut() {
                    a.choice = (choice + n - 1) % n;
                    ui_sfx::play(UiSfx::Move);
                }
            }
            KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::ArrowDown | KeyCode::KeyS => {
                if let Some(a) = self.alcove.as_mut() {
                    a.choice = (choice + 1) % n;
                    ui_sfx::play(UiSfx::Move);
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyF => {
                self.alcove_pick(choice)
            }
            KeyCode::Escape | KeyCode::Backspace => self.alcove_back(),
            KeyCode::Digit1
            | KeyCode::Digit2
            | KeyCode::Digit3
            | KeyCode::Digit4
            | KeyCode::Digit5
            | KeyCode::Digit6
            | KeyCode::Digit7 => {
                let pick = match code {
                    KeyCode::Digit1 => 0,
                    KeyCode::Digit2 => 1,
                    KeyCode::Digit3 => 2,
                    KeyCode::Digit4 => 3,
                    KeyCode::Digit5 => 4,
                    KeyCode::Digit6 => 5,
                    _ => 6,
                };
                if pick < n {
                    self.alcove_pick(pick);
                }
            }
            _ => {}
        }
    }

    /// A click: the card under the pointer.
    pub(super) fn alcove_click(&mut self) {
        let Some((x, y)) = self.shop_pointer_position() else {
            return;
        };
        let n = self.alcove_cards().len();
        if let Some(i) = map_overlay::alcove_hit(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32, n, x, y)
        {
            if self.alcove.as_ref().is_some_and(|a| a.choice == i) {
                self.alcove_pick(i);
            } else if let Some(a) = self.alcove.as_mut() {
                a.choice = i;
                ui_sfx::play(UiSfx::Move);
            }
        }
    }

    /// The 2D half for the overlay (`None` while a guion or the bed scene plays over her).
    pub(super) fn alcove_view(&self) -> Option<map_overlay::AlcoveView> {
        let a = self.alcove.as_ref()?;
        if a.busy {
            return None;
        }
        let ms = |t: f32| ((self.elapsed - t).max(0.0) * 1000.0) as u32;
        let tease = a.teased.map(|(i, at)| {
            let t = &TEASES[i];
            map_overlay::AlcoveTease {
                queen: t.queen.to_owned(),
                thought: t.thought,
                reply: t.reply.to_owned(),
                jealous: t.jealous,
                age_ms: ms(at),
            }
        });
        let face = |who: &str, mood: Option<&str>| {
            guion::portraits()
                .find(who, mood)
                .map(|p| map_overlay::PortraitRef {
                    image: p.image,
                    crop: p.crop,
                })
        };
        let jealous = tease.as_ref().is_some_and(|t| t.jealous);
        Some(map_overlay::AlcoveView {
            page: match a.page {
                Page::Main => 0,
                Page::Explore => 1,
                Page::Chiquita => 2,
                Page::Invite => 3,
            },
            cards: self.alcove_cards(),
            choice: a.choice as u8,
            page_ms: ms(a.page_at),
            open_ms: ms(a.started),
            night: if a.sleeping.is_some() {
                a.slept_at_night
            } else {
                self.alcove_night()
            },
            fondness: self.story.get("chiquita_afecto"),
            tease,
            sleep_ms: a.sleeping.map(ms),
            chiquita: face("chiquita", jealous.then_some("celosa")),
            mascot: true,
            queen: face("gorou", Some("sonrojada")),
        })
    }
}
