//! La corte de la Reina: lo que hace en su castillo además de dormir. Audiencias en el trono (cada
//! día unas peticiones de la villa, con oro, reputación y cosas que pasan después), las obras del
//! castillo que Lira lleva en la despensa (restaurar salas con oro, cada una con su uso), la armería
//! ya restaurada con sus espadas, las lecciones de magia de Nyxara (repetir las runas que enseña:
//! cada lección deja un don para la mina o contra Varrak) y el entrenamiento con Nemi en la arena
//! (parar sus golpes a tiempo: la destreza sube el daño de la espada). Los textos están en
//! `data/castillo/*.json`; el progreso, en las variables de la historia (se guardan con la partida).
use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::ui_sfx::{self, UiSfx};
use super::*;

// ------------------------------------------------------------------ what the files say

/// The throne's petitions (`data/castillo/audiencias.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Audiencias {
    /// How many are heard a day.
    pub por_dia: i32,
    pub peticiones: Vec<Peticion>,
}

/// Someone come to ask the Queen for something.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Peticion {
    pub id: String,
    pub quien: String,
    pub oficio: String,
    /// Their bust (a key under `data/assets`).
    pub retrato: String,
    /// When they may come (a guion condition; empty: from the start).
    pub desde: String,
    /// They may come again once heard.
    pub repetible: bool,
    pub texto: String,
    pub chiquita: String,
    pub opciones: Vec<Opcion>,
}

/// What the Queen may answer, and what it does.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Opcion {
    pub texto: String,
    /// Gold she gains (negative: she pays).
    pub oro: i32,
    pub reputacion: i32,
    /// Story variables it adds to.
    pub vars: BTreeMap<String, i32>,
    /// Things she is given.
    pub objetos: BTreeMap<String, u32>,
    pub respuesta: String,
    pub chiquita: String,
}

/// The castle's works (`data/castillo/obras.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Obras {
    pub obras: Vec<Obra>,
}

/// A room to restore with gold.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Obra {
    pub id: String,
    pub sala: String,
    pub coste: u32,
    pub imagen: String,
    pub ahora: String,
    pub da: String,
    pub hecha: String,
    pub chiquita: String,
    pub desde: String,
}

/// Nyxara's lessons (`data/castillo/lecciones.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Lecciones {
    pub lecciones: Vec<Leccion>,
}

/// A lesson: what is said, the runes to repeat, the gift it leaves.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Leccion {
    pub id: String,
    pub titulo: String,
    pub desde: String,
    /// The story variable the gift sets (`don_chispa`…).
    pub don: String,
    pub don_nombre: String,
    pub don_texto: String,
    pub intro: Vec<Linea>,
    pub exito: String,
    pub fallo: String,
    pub repaso: String,
    /// How many runes the first round shows (3 if unset).
    pub runas: usize,
    /// The scene that plays the first time it is learned (Nyxara takes her down to the seal; its
    /// shot is a memory), if any.
    pub escena: String,
}

/// A line said in a lesson: who says it and what.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Linea {
    pub quien: String,
    pub texto: String,
}

/// What Nemi (and Chiquita) say while they train (`data/castillo/entrenamiento.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Entrenamiento {
    pub inicio: Vec<String>,
    pub bien: Vec<String>,
    pub mal: Vec<String>,
    pub final_bien: Vec<String>,
    pub final_mal: Vec<String>,
    pub sube: Vec<String>,
    pub maximo: Vec<String>,
    pub cansada: Vec<String>,
    pub chiquita: Vec<String>,
}

pub(crate) fn read<T: serde::de::DeserializeOwned + Default>(name: &str) -> T {
    let path = crate::exploration3d_state::data_dir()
        .join("castillo")
        .join(name);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
            eprintln!("[corte] {}: {error}", path.display());
            T::default()
        }),
        Err(_) => {
            eprintln!("[corte] falta {}", path.display());
            T::default()
        }
    }
}

pub(crate) fn audiencias() -> &'static Audiencias {
    static A: OnceLock<Audiencias> = OnceLock::new();
    A.get_or_init(|| read("audiencias.json"))
}

pub(crate) fn obras() -> &'static Obras {
    static O: OnceLock<Obras> = OnceLock::new();
    O.get_or_init(|| read("obras.json"))
}

pub(crate) fn lecciones() -> &'static Lecciones {
    static L: OnceLock<Lecciones> = OnceLock::new();
    L.get_or_init(|| read("lecciones.json"))
}

pub(crate) fn entrenamiento() -> &'static Entrenamiento {
    static E: OnceLock<Entrenamiento> = OnceLock::new();
    E.get_or_init(|| read("entrenamiento.json"))
}

// ------------------------------------------------------------------ rules

/// The restored armoury's swords and their prices.
pub(crate) const ARMORY: [(&str, u32); 4] = [
    ("espada_reliquia", 150),
    ("espada_pegajosa", 120),
    ("espada_brasas", 240),
    ("espada_corazon", 320),
];

/// The gifts Nyxara's lessons leave (story variables).
pub(crate) const DON_CHISPA: &str = "don_chispa";
pub(crate) const DON_SELLO: &str = "don_sello";
pub(crate) const DON_CURA: &str = "don_cura";
pub(crate) const DON_AGUANTE: &str = "don_aguante";

/// What the day allows once (cleared when she sleeps).
const TODAY: [&str; 10] = [
    "aud_hoy",
    "botiquin_hoy",
    "cocina_hoy",
    "leccion_hoy",
    "entreno_hoy",
    "noche_hoy",
    "bano_hoy",
    "prisionera_hoy",
    "cita_hoy",
    // one turn of the war campaign a day (its gold is the Queen's: no farming it by turns)
    "guerra_turno_hoy",
];

/// Her sword skill's top level (Nemi's training), and what each level adds to a blow.
pub(crate) const SKILL_TOP: i32 = 5;
const SKILL_STEP: f32 = 0.08;

/// The runes Nyxara teaches (their number on the keys, 1–6).
pub(crate) const RUNES: usize = 6;

/// Each lesson's rune on its cards (`rune:<n>`, one of the six): the spark's bolt, the seal's cut
/// ring, the light that mends, the true name's barred stem.
fn lesson_rune(index: usize) -> String {
    format!("rune:{}", [0, 1, 2, 4][index % 4])
}
const RUNE_SHOW: f32 = 0.72;
const RUNE_GAP: f32 = 0.22;
const RUNE_ROUNDS: u8 = 3;
const RUNE_LIVES: u8 = 3;

/// Nemi's blows: how long a blow is announced before it lands, and how early or late a parry
/// still counts (seconds).
const BLOW_WARNING: f32 = 1.15;
const PARRY_EARLY: f32 = 0.24;
const PARRY_LATE: f32 = 0.2;
/// The session: rounds of blows, the pause between rounds, and the parries a good session needs.
const DUEL_ROUNDS: usize = 3;
const DUEL_BLOWS: usize = 6;
const DUEL_GOOD: u32 = 13;

/// A line from a list, by a turning counter (so they don't all say the first one).
fn pick(lines: &[String], k: usize) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        lines[k % lines.len()].clone()
    }
}

/// A small deterministic dice (the runes' order, the blows' sides) from a seed.
fn roll(seed: &mut u32) -> u32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed
}

/// The name of something she can carry, as the shop or the backpack call it.
pub(crate) fn item_name(id: &str) -> String {
    backpack::extra(id)
        .map(|e| e.name.to_owned())
        .or_else(|| {
            shop::catalog()
                .products
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.clone())
        })
        .unwrap_or_else(|| id.to_owned())
}

// ------------------------------------------------------------------ the pages

/// Nyxara's runes: a row of them shown one by one, then repeated from memory.
#[derive(Clone, Debug)]
pub(crate) struct RuneGame {
    pub sequence: Vec<u8>,
    pub entered: usize,
    pub round: u8,
    pub mistakes: u8,
    /// When the row began to be shown (`None`: it is her turn).
    pub showing: Option<f32>,
    /// The last rune she pressed, whether it was right, and when.
    pub flash: Option<(u8, bool, f32)>,
    /// Won or lost, and when.
    pub over: Option<(bool, f32)>,
    /// A round just won: the next is shown from then.
    pub next_round_at: Option<f32>,
    seed: u32,
}

impl RuneGame {
    fn new(first: usize, seed: u32, now: f32) -> Self {
        let mut game = Self {
            sequence: Vec::new(),
            entered: 0,
            round: 0,
            mistakes: 0,
            showing: Some(now + 0.6),
            flash: None,
            over: None,
            next_round_at: None,
            seed: seed.max(1),
        };
        for _ in 0..first.clamp(2, 6) {
            game.grow();
        }
        game
    }

    fn grow(&mut self) {
        let last = self.sequence.last().copied();
        loop {
            let r = (roll(&mut self.seed) % RUNES as u32) as u8;
            if Some(r) != last {
                self.sequence.push(r);
                break;
            }
        }
    }

    /// The rune lit while the row is shown (and whether the showing is over).
    pub(crate) fn lit(&self, now: f32) -> (Option<u8>, bool) {
        let Some(from) = self.showing else {
            return (None, true);
        };
        let t = now - from;
        if t < 0.0 {
            return (None, false);
        }
        let step = RUNE_SHOW + RUNE_GAP;
        let i = (t / step) as usize;
        if i >= self.sequence.len() {
            return (None, true);
        }
        let within = t - i as f32 * step;
        ((within < RUNE_SHOW).then(|| self.sequence[i]), false)
    }

    /// The row shown: her turn. A round won: the next row after a breath.
    fn tick(&mut self, now: f32) {
        if self.over.is_some() {
            return;
        }
        if let Some(at) = self.next_round_at {
            if now >= at {
                self.next_round_at = None;
                self.grow();
                self.entered = 0;
                self.showing = Some(now);
            }
            return;
        }
        if self.showing.is_some() && self.lit(now).1 {
            self.showing = None;
        }
    }

    /// She presses rune `r`. Returns whether it was right.
    fn press(&mut self, r: u8, now: f32) -> Option<bool> {
        if self.over.is_some() || self.showing.is_some() || self.next_round_at.is_some() {
            return None;
        }
        let right = self.sequence.get(self.entered) == Some(&r);
        self.flash = Some((r, right, now));
        if right {
            self.entered += 1;
            if self.entered == self.sequence.len() {
                self.round += 1;
                if self.round >= RUNE_ROUNDS {
                    self.over = Some((true, now));
                } else {
                    self.next_round_at = Some(now + 0.9);
                }
            }
        } else {
            self.mistakes += 1;
            if self.mistakes >= RUNE_LIVES {
                self.over = Some((false, now));
            } else {
                // Shown again from the start.
                self.entered = 0;
                self.showing = Some(now + 0.7);
            }
        }
        Some(right)
    }
}

/// One of Nemi's blows: from where (0 left, 1 right, 2 above) and when it lands (s).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Blow {
    pub from: u8,
    pub at: f32,
    /// Parried or not, once it is decided.
    pub result: Option<bool>,
}

/// Nemi's training: her blows coming, what was parried, her words.
#[derive(Clone, Debug)]
pub(crate) struct Duel {
    pub started: f32,
    pub blows: Vec<Blow>,
    pub parried: u32,
    pub hit: u32,
    /// The last blow decided: parried?, from where, when.
    pub last: Option<(bool, u8, f32)>,
    pub line: Option<(String, f32)>,
    lines_said: usize,
    next_aside: f32,
    /// Done, and when; then what came of it.
    pub over: Option<f32>,
    pub outcome: String,
    pub level_up: bool,
}

impl Duel {
    fn new(now: f32, seed: u32) -> Self {
        let mut seed = seed.max(1);
        let mut blows = Vec::new();
        let mut t = 2.2;
        for round in 0..DUEL_ROUNDS {
            // Faster each round.
            let gap = 1.45 - round as f32 * 0.22;
            for _ in 0..DUEL_BLOWS {
                let from = (roll(&mut seed) % 3) as u8;
                blows.push(Blow {
                    from,
                    at: t,
                    result: None,
                });
                t += gap + (roll(&mut seed) % 100) as f32 / 100.0 * 0.25;
            }
            t += 1.6;
        }
        Self {
            started: now,
            blows,
            parried: 0,
            hit: 0,
            last: None,
            line: None,
            lines_said: 0,
            next_aside: now + 7.0,
            over: None,
            outcome: String::new(),
            level_up: false,
        }
    }

    fn clock(&self, now: f32) -> f32 {
        now - self.started
    }

    /// The blow she may parry now (the first not decided, if it is close enough).
    fn parryable(&self, now: f32) -> Option<usize> {
        let t = self.clock(now);
        self.blows
            .iter()
            .position(|b| b.result.is_none())
            .filter(|&i| t >= self.blows[i].at - PARRY_EARLY)
    }

    fn decide(&mut self, i: usize, parried: bool, now: f32) {
        let from = self.blows[i].from;
        self.blows[i].result = Some(parried);
        if parried {
            self.parried += 1;
        } else {
            self.hit += 1;
        }
        self.last = Some((parried, from, now));
        let words = entrenamiento();
        let list = if parried { &words.bien } else { &words.mal };
        // Not every blow gets a word: one in two.
        if self.lines_said % 2 == 0 || !parried {
            self.line = Some((pick(list, self.lines_said), now));
        }
        self.lines_said += 1;
    }
}

/// The page on show.
#[derive(Clone, Debug)]
pub(crate) enum Page {
    /// A petition at the throne, and the answer once given.
    Audience {
        index: usize,
        answered: Option<usize>,
    },
    /// The castle's works.
    Works,
    /// The restored armoury.
    Armory,
    /// Nyxara's lessons.
    Lessons,
    /// A lesson's words, a line at a time.
    LessonTalk { index: usize, line: usize },
    /// The runes to repeat.
    Runes { index: usize, game: RuneGame },
    /// How the lesson ended.
    LessonEnd {
        index: usize,
        learned: bool,
        first: bool,
    },
    /// Nemi's training at the arena.
    Training(Duel),
    /// The morning: what each of the court did, and today's tasks (`court_days.rs`).
    Morning,
    /// The balcony's telescope: a direction looked at (`None`: choosing one).
    Telescope { look: Option<usize> },
    /// The library's book of lore.
    Codex,
    /// The great hall's sofas: whom to date, and her memories (`dates.rs`).
    Dates(super::dates::DatesPage),
}

/// The court's screen while it is up.
pub(crate) struct Court {
    pub page: Page,
    pub choice: usize,
    pub opened: f32,
    pub page_at: f32,
    /// When the lit card last changed (the book's pages turn).
    pub choice_at: f32,
    /// A line at the foot (what was just bought or restored), and when.
    pub notice: Option<(String, f32)>,
    /// A big word thrown on screen, and when.
    pub stamp: Option<(String, f32)>,
}

impl Court {
    fn new(page: Page, now: f32) -> Self {
        Self {
            page,
            choice: 0,
            opened: now,
            page_at: now,
            choice_at: now,
            notice: None,
            stamp: None,
        }
    }
}

// ------------------------------------------------------------------ the world's side

impl App {
    /// The day's uses come back (she slept); a morning with a guard also brings the village's
    /// thanks.
    pub(super) fn court_new_day(&mut self, morning: bool) {
        for var in TODAY {
            self.story.values.insert(var.to_owned(), 0);
        }
        if morning && self.story.get("obra_barracones") > 0 {
            *self
                .story
                .values
                .entry("reputacion".to_owned())
                .or_insert(0) += 1;
        }
        // The court's routine: each did her task; the morning's page tells it.
        if morning {
            self.routine_morning();
        }
    }

    /// Her sword skill (0–5, Nemi's training).
    pub(crate) fn sword_skill(&self) -> i32 {
        self.story.get("destreza").clamp(0, SKILL_TOP)
    }

    /// What her skill adds to a blow (×).
    pub(crate) fn skill_multiplier(&self) -> f32 {
        1.0 + SKILL_STEP * self.sword_skill() as f32
    }

    /// A gift of Nyxara's she has.
    pub(crate) fn has_gift(&self, don: &str) -> bool {
        self.story.get(don) > 0
    }

    /// The court is up.
    pub(super) fn court_open(&self) -> bool {
        self.court.is_some()
    }

    pub(super) fn court_show(&mut self, page: Page) {
        let now = self.elapsed;
        match self.court.as_mut() {
            Some(c) => {
                c.page = page;
                c.choice = 0;
                c.page_at = now;
                c.stamp = None;
            }
            None => self.court = Some(Court::new(page, now)),
        }
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
    }

    /// Up from the court: back to walking.
    pub(super) fn close_court(&mut self) {
        if self.court.take().is_some() {
            ui_sfx::play(UiSfx::Close);
        }
        self.clear_input();
        self.persist_3d_state(true);
    }

    pub(super) fn court_notice(&mut self, text: impl Into<String>) {
        let now = self.elapsed;
        if let Some(c) = self.court.as_mut() {
            c.notice = Some((text.into(), now));
        }
    }

    fn court_stamp(&mut self, text: &str) {
        let now = self.elapsed;
        if let Some(c) = self.court.as_mut() {
            c.stamp = Some((text.to_owned(), now));
        }
    }

    pub(super) fn court_give(&mut self, id: &str, n: u32) {
        if n == 0 {
            return;
        }
        *self.inventory.entry(id.to_owned()).or_default() += n;
        let mut pack = self.backpack.clone();
        pack.reconcile(&self.inventory);
        self.backpack = pack;
    }

    // ---- the throne

    /// Who waits at the throne now: the petitions that may come, in their order.
    pub(crate) fn petitions_waiting(&self) -> Vec<usize> {
        let a = audiencias();
        if self.story.get("aud_hoy") >= a.por_dia.max(1) {
            return Vec::new();
        }
        a.peticiones
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.opciones.is_empty())
            .filter(|(_, p)| missions::holds(&p.desde, &self.story))
            .filter(|(_, p)| p.repetible || self.story.get(&format!("aud_{}", p.id)) == 0)
            .map(|(i, _)| i)
            .collect()
    }

    /// The throne: whoever waits comes forward. Whether anyone did.
    pub(super) fn open_audience(&mut self) -> bool {
        // The ones not heard yet first; the ones that may come again after them.
        let waiting = self.petitions_waiting();
        let a = audiencias();
        let first = waiting
            .iter()
            .copied()
            .find(|&i| self.story.get(&format!("aud_{}", a.peticiones[i].id)) == 0)
            .or_else(|| waiting.first().copied());
        let Some(index) = first else {
            return false;
        };
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Audience {
            index,
            answered: None,
        });
        if let Some(p) = a.peticiones.get(index) {
            if !p.chiquita.trim().is_empty() {
                self.float_chiquita_after(&p.chiquita, mascot::Cue::Talk, 1.6);
            }
        }
        true
    }

    /// The Queen answers petition `index` with option `pick`.
    fn answer_petition(&mut self, index: usize, pick: usize) {
        let Some(p) = audiencias().peticiones.get(index) else {
            return;
        };
        let Some(o) = p.opciones.get(pick) else {
            return;
        };
        if o.oro < 0 && self.gold < o.oro.unsigned_abs() {
            ui_sfx::play(UiSfx::Error);
            self.court_notice(format!(
                "No tienes oro bastante (te faltan {}).",
                o.oro.unsigned_abs() - self.gold
            ));
            return;
        }
        self.gold = (self.gold as i64 + o.oro as i64).max(0) as u32;
        *self
            .story
            .values
            .entry("reputacion".to_owned())
            .or_insert(0) += o.reputacion;
        for (var, n) in &o.vars {
            *self.story.values.entry(guion::slug(var)).or_insert(0) += n;
        }
        for (id, n) in &o.objetos {
            self.court_give(id, *n);
        }
        self.story
            .values
            .insert(format!("aud_{}", p.id), pick as i32 + 1);
        *self.story.values.entry("aud_hoy".to_owned()).or_insert(0) += 1;
        *self
            .story
            .values
            .entry("audiencias_total".to_owned())
            .or_insert(0) += 1;
        ui_sfx::play(if o.oro != 0 {
            UiSfx::Coins
        } else {
            UiSfx::Select
        });
        if let Some(c) = self.court.as_mut() {
            c.page = Page::Audience {
                index,
                answered: Some(pick),
            };
            c.page_at = self.elapsed;
        }
        // The sentence lands.
        self.court_stamp("¡QUE ASÍ SEA!");
        if !o.chiquita.trim().is_empty() {
            self.float_chiquita_after(&o.chiquita, mascot::Cue::Talk, 1.2);
        }
        self.persist_3d_state(true);
    }

    /// After an answer: the next petition, or the court rises.
    fn next_petition(&mut self) {
        if !self.open_audience() {
            self.close_court();
            self.float_chiquita_cue(
                "—Se acabaron las audiencias por hoy. Mañana, más quejas.",
                mascot::Cue::Talk,
            );
        }
    }

    // ---- the works

    /// Lira's accounts in the pantry: the castle's works.
    pub(super) fn open_works(&mut self) -> bool {
        if obras().obras.is_empty() {
            return false;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Works);
        true
    }

    fn restore(&mut self, index: usize) {
        let Some(o) = obras().obras.get(index) else {
            return;
        };
        let var = format!("obra_{}", o.id);
        if self.story.get(&var) > 0 {
            ui_sfx::play(UiSfx::Error);
            self.court_notice(format!("{}: ya está restaurada.", o.sala));
            return;
        }
        if !missions::holds(&o.desde, &self.story) {
            ui_sfx::play(UiSfx::Error);
            self.court_notice("Todavía no se puede.");
            return;
        }
        if self.gold < o.coste {
            ui_sfx::play(UiSfx::Error);
            self.court_notice(format!(
                "No tienes oro bastante (te faltan {}).",
                o.coste - self.gold
            ));
            return;
        }
        self.gold -= o.coste;
        self.story.values.insert(var, 1);
        ui_sfx::play(UiSfx::Deal);
        self.court_stamp("¡RESTAURADA!");
        let line = if o.hecha.trim().is_empty() {
            format!("{}: restaurada.", o.sala)
        } else {
            format!("Lira: {}", o.hecha.trim())
        };
        self.court_notice(line);
        if !o.chiquita.trim().is_empty() {
            self.float_chiquita_after(&o.chiquita, mascot::Cue::Cheer, 0.8);
        }
        self.persist_3d_state(true);
    }

    // ---- the armoury

    /// The restored armoury's racks.
    pub(super) fn open_armory(&mut self) {
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Armory);
    }

    fn buy_sword(&mut self, index: usize) {
        let Some(&(id, price)) = ARMORY.get(index) else {
            return;
        };
        if self.gold < price {
            ui_sfx::play(UiSfx::Error);
            self.court_notice(format!(
                "No tienes oro bastante (te faltan {}).",
                price - self.gold
            ));
            return;
        }
        self.gold -= price;
        self.court_give(id, 1);
        ui_sfx::play(UiSfx::Coins);
        self.court_stamp("¡COMPRADA!");
        self.court_notice(format!("{}: tuya. Está en la mochila.", item_name(id)));
        self.persist_3d_state(true);
    }

    // ---- the rooms' daily uses

    /// The infirmary's cabinet: a cure a day once it is restored. Whether it did anything.
    pub(super) fn use_cabinet(&mut self) -> bool {
        if self.story.get("obra_enfermeria") == 0 {
            self.float_chiquita_cue(
                "—Frascos rotos y vendas de hace un siglo. Si Lira arregla la enfermería (las obras, en la despensa), esto servirá de algo.",
                mascot::Cue::Talk,
            );
            return true;
        }
        if self.story.get("botiquin_hoy") > 0 {
            self.float_chiquita_cue(
                "—Ya cogiste la de hoy. Nyxara cuenta los frascos.",
                mascot::Cue::Talk,
            );
            return true;
        }
        self.story.values.insert("botiquin_hoy".to_owned(), 1);
        self.court_give("pocion_pequena", 1);
        ui_sfx::play(UiSfx::Pick);
        self.notice_note(format!(
            "Botiquín\n+1 {} (una al día).",
            item_name("pocion_pequena")
        ));
        self.persist_3d_state(true);
        true
    }

    /// The kitchen's meal of the day, once it is restored. Whether it did anything.
    pub(super) fn use_kitchen(&mut self) -> bool {
        if self.story.get("obra_cocina") == 0 || self.story.get("cocina_hoy") > 0 {
            return false;
        }
        self.story.values.insert("cocina_hoy".to_owned(), 1);
        self.court_give("pan_del_dia", 2);
        self.court_give("hierbas_curativas", 1);
        ui_sfx::play(UiSfx::Pick);
        self.notice_note(format!(
            "Comida del día\n+2 {} · +1 {}",
            item_name("pan_del_dia"),
            item_name("hierbas_curativas")
        ));
        self.float_chiquita_cue(
            "—Lira cocina como si fuéramos a una guerra. Bueno. Vamos a una.",
            mascot::Cue::Cheer,
        );
        self.persist_3d_state(true);
        true
    }

    /// The barracks: the guard on duty, once they are restored. Whether it did anything.
    pub(super) fn use_barracks(&mut self) -> bool {
        if self.story.get("obra_barracones") == 0 {
            return false;
        }
        // the first visit once restored: Nemi presents the new guard
        if self.story.get("barracones_vista") == 0 && self.start_talk_at("corte.barracones") {
            return true;
        }
        self.float_chiquita_cue(
            "—Una guardia de verdad. La villa duerme más tranquila, y te quiere un poco más cada mañana.",
            mascot::Cue::Talk,
        );
        true
    }

    // ---- Nyxara's lessons

    /// Nyxara's lessons (F on her).
    pub(super) fn open_lessons(&mut self) -> bool {
        if lecciones().lecciones.is_empty() {
            return false;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Lessons);
        true
    }

    /// A lesson she may take now (and why not, if not).
    pub(crate) fn lesson_state(&self, index: usize) -> (bool, &'static str) {
        let Some(l) = lecciones().lecciones.get(index) else {
            return (false, "");
        };
        if !missions::holds(&l.desde, &self.story) {
            return (false, "Necesita la biblioteca restaurada.");
        }
        if !self.has_gift(&l.don) && self.story.get("leccion_hoy") > 0 {
            return (false, "Una lección al día. Mañana, otra.");
        }
        (true, "")
    }

    fn start_lesson(&mut self, index: usize) {
        let (open, why) = self.lesson_state(index);
        if !open {
            ui_sfx::play(UiSfx::Error);
            self.court_notice(why);
            return;
        }
        ui_sfx::play(UiSfx::Select);
        self.court_show(Page::LessonTalk { index, line: 0 });
    }

    fn lesson_runes(&mut self, index: usize) {
        let first =
            lecciones()
                .lecciones
                .get(index)
                .map_or(3, |l| if l.runas == 0 { 3 } else { l.runas });
        let seed = (self.elapsed * 1000.0) as u32 ^ (index as u32 * 7919 + 17);
        let game = RuneGame::new(first, seed, self.elapsed);
        self.court_show(Page::Runes { index, game });
    }

    fn lesson_over(&mut self, index: usize, won: bool) {
        let Some(l) = lecciones().lecciones.get(index) else {
            return;
        };
        let first = won && !self.has_gift(&l.don);
        if first {
            self.story.values.insert(l.don.clone(), 1);
            self.story.values.insert("leccion_hoy".to_owned(), 1);
            *self
                .story
                .values
                .entry("afecto_nyxara".to_owned())
                .or_insert(0) += 1;
            ui_sfx::play(UiSfx::Deal);
        } else if won {
            ui_sfx::play(UiSfx::Select);
        } else {
            ui_sfx::play(UiSfx::Fail);
        }
        self.court_show(Page::LessonEnd {
            index,
            learned: won,
            first,
        });
        self.court_stamp(if first {
            "¡APRENDIDO!"
        } else if won {
            "¡BIEN!"
        } else {
            "¡OTRA VEZ!"
        });
        self.persist_3d_state(true);
    }

    // ---- Nemi's training

    /// The arena: Nemi trains her once the first three lessons are done. Whether it opened.
    pub(super) fn open_training(&mut self) -> bool {
        if self.story.get("espada_regalo") == 0 || self.story.get("entreno") < 3 {
            return false;
        }
        let words = entrenamiento();
        if self.story.get("entreno_hoy") > 0 {
            let line = pick(&words.cansada, self.story.get("destreza") as usize);
            self.float_chiquita_cue(
                if line.is_empty() {
                    "—Nemi dice que mañana. Hoy ya te ha zurrado bastante."
                } else {
                    &line
                },
                mascot::Cue::Talk,
            );
            return true;
        }
        let seed = (self.elapsed * 977.0) as u32 ^ 0x9e37_79b9;
        let mut duel = Duel::new(self.elapsed, seed);
        duel.line = Some((
            pick(&words.inicio, self.story.get("destreza") as usize),
            self.elapsed,
        ));
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Training(duel));
        true
    }

    /// She parries toward `from` (0 left, 1 right, 2 above).
    fn parry(&mut self, from: u8) {
        let now = self.elapsed;
        let Some(Court {
            page: Page::Training(duel),
            ..
        }) = self.court.as_mut()
        else {
            return;
        };
        if duel.over.is_some() {
            return;
        }
        let Some(i) = duel.parryable(now) else {
            return;
        };
        let right = duel.blows[i].from == from;
        duel.decide(i, right, now);
        ui_sfx::play(if right { UiSfx::Equip } else { UiSfx::Fail });
    }

    fn training_over(&mut self) {
        let now = self.elapsed;
        let words = entrenamiento();
        let (parried, already_today) = match self.court.as_ref() {
            Some(Court {
                page: Page::Training(d),
                ..
            }) => (d.parried, self.story.get("entreno_hoy") > 0),
            _ => return,
        };
        let good = parried >= DUEL_GOOD;
        let level = self.sword_skill();
        let mut level_up = false;
        let outcome = if !good {
            pick(&words.final_mal, parried as usize)
        } else if already_today {
            pick(&words.final_bien, parried as usize)
        } else if level >= SKILL_TOP {
            pick(&words.maximo, parried as usize)
        } else {
            level_up = true;
            pick(&words.sube, level as usize).replace("{0}", &(level + 1).to_string())
        };
        if level_up {
            self.story.values.insert("destreza".to_owned(), level + 1);
            self.apply_equipment();
        }
        // a good morning with Nemi (the first of the day) brings her closer (the dates' affection)
        if good && !already_today {
            *self
                .story
                .values
                .entry("afecto_nemi".to_owned())
                .or_insert(0) += 1;
        }
        self.story.values.insert("entreno_hoy".to_owned(), 1);
        if let Some(Court {
            page: Page::Training(d),
            ..
        }) = self.court.as_mut()
        {
            d.over = Some(now);
            d.outcome = outcome;
            d.level_up = level_up;
        }
        ui_sfx::play(if level_up {
            UiSfx::Deal
        } else if good {
            UiSfx::Select
        } else {
            UiSfx::Fail
        });
        self.court_stamp(if level_up {
            "¡DESTREZA +1!"
        } else if good {
            "¡BIEN!"
        } else {
            "¡AY!"
        });
        self.persist_3d_state(true);
    }

    // ---- each step

    pub(super) fn tick_court(&mut self) {
        let now = self.elapsed;
        let mut runes_over = None;
        let mut duel_done = false;
        let mut aside = None;
        match self.court.as_mut().map(|c| &mut c.page) {
            Some(Page::Runes { index, game }) => {
                game.tick(now);
                if let Some((won, at)) = game.over {
                    if now - at > 1.1 {
                        runes_over = Some((*index, won));
                    }
                }
            }
            Some(Page::Training(duel)) => {
                if duel.over.is_none() {
                    let t = duel.clock(now);
                    // Blows not parried in time land.
                    for i in 0..duel.blows.len() {
                        if duel.blows[i].result.is_none() && t > duel.blows[i].at + PARRY_LATE {
                            duel.decide(i, false, now);
                            ui_sfx::play(UiSfx::Fail);
                        }
                    }
                    if now >= duel.next_aside {
                        duel.next_aside = now + 9.0;
                        aside = Some(duel.lines_said);
                    }
                    if duel.blows.iter().all(|b| b.result.is_some())
                        && duel.last.is_some_and(|(_, _, at)| now - at > 0.9)
                    {
                        duel_done = true;
                    }
                }
            }
            _ => {}
        }
        if let Some((index, won)) = runes_over {
            self.lesson_over(index, won);
        }
        if duel_done {
            self.training_over();
        }
        if let Some(k) = aside {
            let line = pick(&entrenamiento().chiquita, k);
            if !line.is_empty() {
                self.float_chiquita_cue(&line, mascot::Cue::Talk);
            }
        }
    }

    // ---- keys and clicks

    /// How many cards the page shows (its choices).
    pub(crate) fn court_cards(&self) -> usize {
        match self.court.as_ref().map(|c| &c.page) {
            Some(Page::Audience {
                index,
                answered: None,
            }) => audiencias()
                .peticiones
                .get(*index)
                .map_or(0, |p| p.opciones.len()),
            Some(Page::Works) => obras().obras.len(),
            Some(Page::Armory) => ARMORY.len(),
            Some(Page::Lessons) => lecciones().lecciones.len(),
            Some(Page::Runes { .. }) => RUNES,
            Some(Page::Morning) => self.routine_cards().len(),
            Some(Page::Telescope { look: None }) => court_days::catalejo().direcciones.len(),
            Some(Page::Codex) => court_days::codice().entradas.len() + 1,
            Some(Page::Dates(_)) => dates::citas().chicas.len(),
            _ => 0,
        }
    }

    /// Enter on the card lit (or on what the page waits for).
    pub(super) fn court_pick(&mut self, pick: usize) {
        let Some(page) = self.court.as_ref().map(|c| c.page.clone()) else {
            return;
        };
        if let Some(c) = self.court.as_mut() {
            c.choice = pick;
        }
        match page {
            Page::Audience {
                index,
                answered: None,
            } => self.answer_petition(index, pick),
            Page::Audience { .. } => self.next_petition(),
            Page::Works => self.restore(pick),
            Page::Armory => self.buy_sword(pick),
            Page::Lessons => self.start_lesson(pick),
            Page::LessonTalk { index, line } => {
                let lines = lecciones()
                    .lecciones
                    .get(index)
                    .map_or(0, |l| l.intro.len());
                ui_sfx::play(UiSfx::Move);
                if line + 1 < lines {
                    if let Some(c) = self.court.as_mut() {
                        c.page = Page::LessonTalk {
                            index,
                            line: line + 1,
                        };
                        c.page_at = self.elapsed;
                    }
                } else {
                    self.lesson_runes(index);
                }
            }
            Page::Runes { .. } => self.press_rune(pick as u8),
            Page::LessonEnd {
                learned,
                index,
                first,
            } => {
                let scene = lecciones()
                    .lecciones
                    .get(index)
                    .map(|l| l.escena.clone())
                    .unwrap_or_default();
                if learned && first && !scene.is_empty() {
                    // learned for the first time: down to the seal to try it for real
                    self.close_court();
                    if !self.start_talk_at(&scene) {
                        eprintln!("[corte] no hay escena {scene}");
                    }
                } else if learned {
                    self.court_show(Page::Lessons);
                } else {
                    // Again, straight away.
                    self.lesson_runes(index);
                }
            }
            Page::Training(d) => {
                if d.over.is_some_and(|at| self.elapsed - at > 0.8) {
                    self.close_court();
                }
            }
            Page::Morning => self.routine_assign(pick),
            Page::Telescope { look: None } => self.telescope_look(pick),
            Page::Telescope { look: Some(d) } => {
                ui_sfx::play(UiSfx::Back);
                self.court_show(Page::Telescope { look: None });
                if let Some(c) = self.court.as_mut() {
                    c.choice = d;
                }
            }
            Page::Codex => self.codex_pick(pick),
            Page::Dates(_) => self.dates_pick(pick),
        }
    }

    fn press_rune(&mut self, r: u8) {
        let now = self.elapsed;
        let result = match self.court.as_mut() {
            Some(Court {
                page: Page::Runes { game, .. },
                ..
            }) => game.press(r, now),
            _ => None,
        };
        match result {
            Some(true) => ui_sfx::play(UiSfx::Move),
            Some(false) => ui_sfx::play(UiSfx::Error),
            None => {}
        }
    }

    /// Esc: back a page, or up from the court.
    fn court_back(&mut self) {
        let page = self.court.as_ref().map(|c| c.page.clone());
        match page {
            Some(Page::LessonTalk { .. })
            | Some(Page::Runes { .. })
            | Some(Page::LessonEnd { .. }) => {
                ui_sfx::play(UiSfx::Back);
                self.court_show(Page::Lessons);
            }
            Some(Page::Telescope { look: Some(d) }) => {
                ui_sfx::play(UiSfx::Back);
                self.court_show(Page::Telescope { look: None });
                if let Some(c) = self.court.as_mut() {
                    c.choice = d;
                }
            }
            Some(Page::Dates(d)) if d.memory.is_some() => {
                self.dates_key(KeyCode::Escape);
            }
            Some(Page::Training(d)) if d.over.is_none() => {
                // Leaving mid-training counts as a session lost (Nemi is not offended).
                ui_sfx::play(UiSfx::Back);
                self.close_court();
            }
            Some(_) => self.close_court(),
            None => {}
        }
    }

    pub(super) fn court_key(&mut self, code: KeyCode) {
        let n = self.court_cards();
        let choice = self.court.as_ref().map_or(0, |c| c.choice);
        let training = matches!(
            self.court.as_ref().map(|c| &c.page),
            Some(Page::Training(_))
        );
        let runes = matches!(
            self.court.as_ref().map(|c| &c.page),
            Some(Page::Runes { .. })
        );
        if training {
            match code {
                KeyCode::ArrowLeft | KeyCode::KeyA => self.parry(0),
                KeyCode::ArrowRight | KeyCode::KeyD => self.parry(1),
                KeyCode::ArrowUp | KeyCode::KeyW => self.parry(2),
                KeyCode::Escape | KeyCode::Backspace => self.court_back(),
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyF => {
                    self.court_pick(0)
                }
                _ => {}
            }
            return;
        }
        // The dates' page: her memories' row and a memory shown big take their own keys.
        if self.dates_key(code) {
            return;
        }
        let digit = match code {
            KeyCode::Digit1 | KeyCode::Numpad1 => Some(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => Some(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => Some(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => Some(3),
            KeyCode::Digit5 | KeyCode::Numpad5 => Some(4),
            KeyCode::Digit6 | KeyCode::Numpad6 => Some(5),
            _ => None,
        };
        if let Some(d) = digit {
            if d < n {
                self.court_pick(d);
            }
            return;
        }
        let page = self.court.as_ref().map(|c| c.page.clone());
        let now = self.elapsed;
        let lit = |app: &mut Self, to: usize| {
            if let Some(c) = app.court.as_mut() {
                if c.choice != to {
                    c.choice = to;
                    c.choice_at = now;
                    ui_sfx::play(UiSfx::Move);
                }
            }
        };
        match (&page, code) {
            // The morning's grid: a row per person, a column per task.
            (Some(Page::Morning), KeyCode::ArrowUp | KeyCode::KeyW) if choice >= 4 => {
                return lit(self, choice - 4);
            }
            (Some(Page::Morning), KeyCode::ArrowDown | KeyCode::KeyS) if choice + 4 < n => {
                return lit(self, choice + 4);
            }
            (
                Some(Page::Morning),
                KeyCode::ArrowUp | KeyCode::KeyW | KeyCode::ArrowDown | KeyCode::KeyS,
            ) => {
                return;
            }
            // The telescope's compass: north, east, south, west.
            (Some(Page::Telescope { look: None }), KeyCode::ArrowUp | KeyCode::KeyW) => {
                return lit(self, 0)
            }
            (Some(Page::Telescope { look: None }), KeyCode::ArrowRight | KeyCode::KeyD) => {
                return lit(self, 1)
            }
            (Some(Page::Telescope { look: None }), KeyCode::ArrowDown | KeyCode::KeyS) => {
                return lit(self, 2)
            }
            (Some(Page::Telescope { look: None }), KeyCode::ArrowLeft | KeyCode::KeyA) => {
                return lit(self, 3)
            }
            _ => {}
        }
        match code {
            KeyCode::ArrowLeft | KeyCode::KeyA | KeyCode::ArrowUp | KeyCode::KeyW if n > 0 => {
                if let Some(c) = self.court.as_mut() {
                    c.choice = (choice + n - 1) % n;
                    c.choice_at = now;
                }
                ui_sfx::play(UiSfx::Move);
            }
            KeyCode::ArrowRight | KeyCode::KeyD | KeyCode::ArrowDown | KeyCode::KeyS if n > 0 => {
                if let Some(c) = self.court.as_mut() {
                    c.choice = (choice + 1) % n;
                    c.choice_at = now;
                }
                ui_sfx::play(UiSfx::Move);
            }
            // Lira's accounts: the day's tasks are on the other page.
            KeyCode::KeyT | KeyCode::Tab if matches!(page, Some(Page::Works)) => {
                self.open_routine();
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyF => {
                // The runes are pressed by their number (or a click), not with Enter.
                if !runes {
                    self.court_pick(choice);
                }
            }
            KeyCode::Escape | KeyCode::Backspace => self.court_back(),
            _ => {}
        }
    }

    /// A click: the card (or rune) under the pointer; elsewhere, on with the page.
    pub(super) fn court_click(&mut self) {
        let Some((x, y)) = self.shop_pointer_position() else {
            return;
        };
        let Some(view) = self.court_view() else {
            return;
        };
        let hit = map_overlay::court_hit(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32, &view, x, y);
        if matches!(self.court.as_ref().map(|c| &c.page), Some(Page::Dates(_))) {
            self.dates_click(hit);
            return;
        }
        match hit {
            Some(i) => {
                let runes = matches!(
                    self.court.as_ref().map(|c| &c.page),
                    Some(Page::Runes { .. })
                );
                if runes || self.court.as_ref().is_some_and(|c| c.choice == i) {
                    self.court_pick(i);
                } else if let Some(c) = self.court.as_mut() {
                    c.choice = i;
                    c.choice_at = self.elapsed;
                    ui_sfx::play(UiSfx::Move);
                }
            }
            None => {
                // Pages without cards go on with a click anywhere.
                if self.court_cards() == 0 {
                    let choice = self.court.as_ref().map_or(0, |c| c.choice);
                    self.court_pick(choice);
                }
            }
        }
    }

    /// The pointer moved: the card under it lights.
    pub(super) fn court_hover(&mut self) {
        let Some((x, y)) = self.shop_pointer_position() else {
            return;
        };
        let Some(view) = self.court_view() else {
            return;
        };
        if matches!(
            self.court.as_ref().map(|c| &c.page),
            Some(Page::Runes { .. }) | Some(Page::Training(_))
        ) {
            return;
        }
        let hit = map_overlay::court_hit(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32, &view, x, y);
        if matches!(self.court.as_ref().map(|c| &c.page), Some(Page::Dates(_))) {
            if let Some(i) = hit {
                self.dates_hover(i);
            }
            return;
        }
        if let Some(i) = hit {
            let now = self.elapsed;
            if let Some(c) = self.court.as_mut() {
                if c.choice != i {
                    c.choice = i;
                    c.choice_at = now;
                    ui_sfx::play(UiSfx::Move);
                }
            }
        }
    }

    // ---- what the overlay shows

    pub(super) fn court_portrait(
        &self,
        who: &str,
        mood: Option<&str>,
    ) -> Option<map_overlay::PortraitRef> {
        guion::portraits()
            .find(who, mood)
            .map(|p| map_overlay::PortraitRef {
                image: p.image,
                crop: p.crop,
            })
    }

    fn court_stats(&self) -> Vec<(String, String)> {
        vec![
            ("Oro".to_owned(), self.gold.to_string()),
            (
                "Reputación".to_owned(),
                self.story.get("reputacion").to_string(),
            ),
        ]
    }

    pub(crate) fn court_view(&self) -> Option<map_overlay::CourtView> {
        use map_overlay::{CourtCard, CourtChip, CourtView, DuelView, RuneView};
        let c = self.court.as_ref()?;
        let now = self.elapsed;
        let ms = |t: f32| ((now - t).max(0.0) * 1000.0) as u32;
        let chip = |text: String, tone: u8| CourtChip { text, tone };
        let mut view = CourtView {
            choice: c.choice as u8,
            page_ms: ms(c.page_at),
            open_ms: ms(c.opened),
            notice: c
                .notice
                .as_ref()
                .filter(|(_, at)| now - at < 6.0)
                .map(|(t, _)| t.clone())
                .unwrap_or_default(),
            stamp: c
                .stamp
                .as_ref()
                .filter(|(_, at)| now - at < 2.2)
                .map(|(t, _)| t.clone())
                .unwrap_or_default(),
            stamp_ms: c.stamp.as_ref().map_or(0, |(_, at)| ms(*at)),
            flip_ms: ms(c.choice_at),
            stats: self.court_stats(),
            ..Default::default()
        };
        match &c.page {
            Page::Audience { index, answered } => {
                let p = audiencias().peticiones.get(*index)?;
                let waiting = self.petitions_waiting().len();
                view.kind = 0;
                view.title = "Audiencia".to_owned();
                view.tag = format!(
                    "LA SALA DEL TRONO · HOY {} / {}",
                    self.story.get("aud_hoy") + i32::from(answered.is_none()),
                    audiencias().por_dia.max(1)
                );
                view.speaker = p.quien.clone();
                view.role = p.oficio.clone();
                view.portrait = (!p.retrato.is_empty()).then(|| map_overlay::PortraitRef {
                    image: p.retrato.clone(),
                    crop: [0, 0, 800, 800],
                });
                match answered {
                    None => {
                        view.text = p.texto.clone();
                        view.cards = p
                            .opciones
                            .iter()
                            .map(|o| {
                                let mut chips = Vec::new();
                                if o.oro != 0 {
                                    chips.push(chip(
                                        format!("{:+} oro", o.oro),
                                        if o.oro > 0 { 3 } else { 2 },
                                    ));
                                }
                                if o.reputacion != 0 {
                                    chips.push(chip(
                                        format!("{:+} reputación", o.reputacion),
                                        if o.reputacion > 0 { 1 } else { 2 },
                                    ));
                                }
                                for (id, n) in &o.objetos {
                                    chips.push(chip(
                                        format!(
                                            "+{n} {}",
                                            demon_queen_dungeon::i18n::tr(&item_name(id))
                                        ),
                                        1,
                                    ));
                                }
                                let locked = o.oro < 0 && self.gold < o.oro.unsigned_abs();
                                CourtCard {
                                    title: o.texto.clone(),
                                    detail: if locked {
                                        format!(
                                            "Te faltan {} de oro.",
                                            o.oro.unsigned_abs() - self.gold
                                        )
                                    } else {
                                        String::new()
                                    },
                                    chips,
                                    picture: None,
                                    icon: String::new(),
                                    state: u8::from(locked),
                                }
                            })
                            .collect();
                        view.keys = vec![
                            ("W S".into(), "elegir".into()),
                            ("1-3".into(), "directo".into()),
                            ("Enter".into(), "responder".into()),
                            ("Esc".into(), "levantarse".into()),
                        ];
                    }
                    Some(pick) => {
                        let o = p.opciones.get(*pick)?;
                        view.text = o.respuesta.clone();
                        view.aside = o.texto.clone();
                        if o.oro != 0 {
                            view.chips.push(chip(
                                format!("{:+} oro", o.oro),
                                if o.oro > 0 { 3 } else { 2 },
                            ));
                        }
                        if o.reputacion != 0 {
                            view.chips.push(chip(
                                format!("{:+} reputación", o.reputacion),
                                if o.reputacion > 0 { 1 } else { 2 },
                            ));
                        }
                        for (id, n) in &o.objetos {
                            view.chips.push(chip(
                                format!("+{n} {}", demon_queen_dungeon::i18n::tr(&item_name(id))),
                                1,
                            ));
                        }
                        let more = waiting > 0;
                        view.keys = vec![(
                            "Enter".into(),
                            if more {
                                "que pase el siguiente".into()
                            } else {
                                "levantarse del trono".into()
                            },
                        )];
                    }
                }
            }
            Page::Works => {
                view.kind = 1;
                view.title = "Obras del castillo".to_owned();
                view.tag = "LAS CUENTAS DE LIRA · RESTAURAR SALAS CON ORO".to_owned();
                view.accent = 0x7fd6a8;
                view.cards = obras()
                    .obras
                    .iter()
                    .map(|o| {
                        let done = self.story.get(&format!("obra_{}", o.id)) > 0;
                        let open = missions::holds(&o.desde, &self.story);
                        let short = !done && self.gold < o.coste;
                        CourtCard {
                            title: o.sala.clone(),
                            detail: if done {
                                o.da.clone()
                            } else {
                                format!(
                                    "{}
{}",
                                    o.ahora.trim(),
                                    o.da.trim()
                                )
                            },
                            chips: vec![if done {
                                chip("RESTAURADA".into(), 1)
                            } else {
                                chip(format!("{} oro", o.coste), if short { 2 } else { 3 })
                            }],
                            picture: (!o.imagen.is_empty()).then(|| map_overlay::PortraitRef {
                                image: o.imagen.clone(),
                                crop: [180, 0, 600, 600],
                            }),
                            icon: String::new(),
                            state: if done { 2 } else { u8::from(!open || short) },
                        }
                    })
                    .collect();
                view.keys = vec![
                    ("A D W S".into(), "elegir".into()),
                    ("Enter".into(), "restaurar".into()),
                    ("T".into(), "tareas del día".into()),
                    ("Esc".into(), "cerrar".into()),
                ];
            }
            Page::Armory => {
                view.kind = 2;
                view.title = "Armería".to_owned();
                view.tag = "ESPADAS DEL CASTILLO · LISTAS PARA LA MINA".to_owned();
                view.accent = 0xe2334a;
                view.cards = ARMORY
                    .iter()
                    .map(|&(id, price)| {
                        let owned = self.inventory.get(id).copied().unwrap_or(0);
                        CourtCard {
                            title: item_name(id),
                            detail: backpack::extra(id)
                                .map(|e| e.description.to_owned())
                                .or_else(|| backpack::use_line(id))
                                .unwrap_or_default(),
                            chips: {
                                let mut c = vec![chip(
                                    format!("{price} oro"),
                                    if self.gold < price { 2 } else { 3 },
                                )];
                                if owned > 0 {
                                    c.push(chip(format!("TIENES {owned}"), 1));
                                }
                                c
                            },
                            picture: None,
                            icon: id.to_owned(),
                            state: u8::from(self.gold < price),
                        }
                    })
                    .collect();
                view.keys = vec![
                    ("A D".into(), "elegir".into()),
                    ("Enter".into(), "comprar".into()),
                    ("Esc".into(), "cerrar".into()),
                ];
            }
            Page::Lessons => {
                view.kind = 3;
                view.title = "Lecciones de Nyxara".to_owned();
                view.tag = "EL SELLO, LA CHISPA Y LO QUE ELLA SABE".to_owned();
                view.accent = 0x9a5cff;
                view.portrait = self.court_portrait("nyxara", Some("sonrisa"));
                view.speaker = "Nyxara".to_owned();
                view.cards = lecciones()
                    .lecciones
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let learned = self.has_gift(&l.don);
                        let (open, why) = self.lesson_state(i);
                        CourtCard {
                            title: l.titulo.clone(),
                            detail: if !open && !why.is_empty() {
                                why.to_owned()
                            } else {
                                l.don_texto.clone()
                            },
                            chips: vec![if learned {
                                chip("APRENDIDA · REPASAR".into(), 1)
                            } else {
                                chip(format!("DON · {}", l.don_nombre), 3)
                            }],
                            picture: None,
                            icon: lesson_rune(i),
                            state: if learned { 2 } else { u8::from(!open) },
                        }
                    })
                    .collect();
                view.keys = vec![
                    ("A D".into(), "elegir".into()),
                    ("Enter".into(), "empezar".into()),
                    ("Esc".into(), "cerrar".into()),
                ];
            }
            Page::LessonTalk { index, line } => {
                let l = lecciones().lecciones.get(*index)?;
                let said = l.intro.get(*line)?;
                view.kind = 4;
                view.title = l.titulo.clone();
                view.tag = format!("LECCIÓN · {} / {}", line + 1, l.intro.len());
                view.accent = 0x9a5cff;
                let who = said.quien.to_lowercase();
                let (key, mood) = if who.contains("chiquita") {
                    ("chiquita", None)
                } else if who.contains("reina") || who.contains("gorou") {
                    ("gorou", None)
                } else {
                    ("nyxara", Some("seria"))
                };
                view.portrait = self.court_portrait(key, mood);
                view.speaker = said.quien.clone();
                view.text = said.texto.clone();
                view.keys = vec![
                    ("Enter".into(), "seguir".into()),
                    ("Esc".into(), "dejarlo".into()),
                ];
            }
            Page::Runes { index, game } => {
                let l = lecciones().lecciones.get(*index)?;
                view.kind = 5;
                view.title = l.titulo.clone();
                view.tag = "REPITE LAS RUNAS QUE NYXARA TRAZA".to_owned();
                view.accent = 0x9a5cff;
                let (lit, done_showing) = game.lit(now);
                view.runes = Some(RuneView {
                    lit,
                    showing: game.showing.is_some() && !done_showing,
                    count: game.sequence.len() as u8,
                    entered: game.entered as u8,
                    round: game.round,
                    rounds: RUNE_ROUNDS,
                    lives: RUNE_LIVES.saturating_sub(game.mistakes),
                    flash: game
                        .flash
                        .filter(|(_, _, at)| now - at < 0.6)
                        .map(|(r, right, at)| (r, right, ms(at))),
                    over: game.over.map(|(won, _)| won),
                });
                view.text = if game.over.is_some() {
                    String::new()
                } else if game.showing.is_some() {
                    "Mira bien…".to_owned()
                } else {
                    "Ahora tú: pulsa las runas en el mismo orden (1-6 o clic).".to_owned()
                };
                // Nyxara beside the circle, her face following the game: tracing, watching,
                // a wince at a wrong rune, pleased at the end.
                let missed = game
                    .flash
                    .is_some_and(|(_, right, at)| !right && now - at < 0.9);
                let mood = match game.over {
                    Some((true, _)) => "sonrisa",
                    Some((false, _)) => "preocupada",
                    None if missed => "preocupada",
                    None if game.showing.is_some() => "pensativa",
                    None => "seria",
                };
                view.portrait = self.court_portrait("nyxara", Some(mood));
                view.speaker = "Nyxara".to_owned();
                view.keys = vec![
                    ("1-6".into(), "runa".into()),
                    ("Esc".into(), "dejarlo".into()),
                ];
            }
            Page::LessonEnd {
                index,
                learned,
                first,
            } => {
                let l = lecciones().lecciones.get(*index)?;
                view.kind = 6;
                view.title = l.titulo.clone();
                view.tag = if *first {
                    "UN DON NUEVO".to_owned()
                } else if *learned {
                    "REPASO".to_owned()
                } else {
                    "OTRA VEZ".to_owned()
                };
                view.accent = 0x9a5cff;
                view.portrait = self.court_portrait(
                    "nyxara",
                    Some(if *learned { "sonrisa" } else { "pensativa" }),
                );
                view.speaker = "Nyxara".to_owned();
                view.text = if *first {
                    l.exito.clone()
                } else if *learned {
                    l.repaso.clone()
                } else {
                    l.fallo.clone()
                };
                if *learned {
                    view.cards = vec![CourtCard {
                        title: l.don_nombre.clone(),
                        detail: l.don_texto.clone(),
                        chips: vec![chip(
                            if *first {
                                "NUEVO".into()
                            } else {
                                "YA ES TUYO".into()
                            },
                            1,
                        )],
                        picture: None,
                        icon: lesson_rune(*index),
                        state: 2,
                    }];
                }
                view.keys = vec![(
                    "Enter".into(),
                    if *learned {
                        "volver".into()
                    } else {
                        "intentarlo otra vez".into()
                    },
                )];
                // The card is shown, not chosen.
                view.choice = 0;
            }
            Page::Training(d) => {
                view.kind = 7;
                view.title = "Entrenamiento".to_owned();
                view.tag = format!(
                    "LA ARENA · CON NEMI · DESTREZA {} / {}",
                    self.sword_skill(),
                    SKILL_TOP
                );
                view.accent = 0xff5c9a;
                view.speaker = "Nemi".to_owned();
                let mood = match d.last {
                    Some((false, _, at)) if now - at < 0.8 => "risa",
                    Some((true, _, at)) if now - at < 0.8 => "sorpresa",
                    _ => "normal",
                };
                view.portrait = Some(map_overlay::PortraitRef {
                    image: format!("castillo_vn/nemi_{mood}"),
                    crop: [256, 60, 460, 460],
                });
                view.text = match d.over {
                    Some(_) => d.outcome.clone(),
                    None => d
                        .line
                        .as_ref()
                        .filter(|(_, at)| now - at < 3.0)
                        .map(|(t, _)| t.clone())
                        .unwrap_or_default(),
                };
                let t = d.clock(now);
                view.duel = Some(DuelView {
                    incoming: d
                        .blows
                        .iter()
                        .filter(|b| b.result.is_none())
                        .filter(|b| t >= b.at - BLOW_WARNING)
                        .take(3)
                        .map(|b| {
                            let k = 1.0 - (b.at - t) / BLOW_WARNING;
                            (b.from, (k.clamp(0.0, 1.2) * 1000.0) as u32)
                        })
                        .collect(),
                    last: d
                        .last
                        .filter(|(_, _, at)| now - at < 0.7)
                        .map(|(p, from, at)| (p, from, ms(at))),
                    parried: d.parried,
                    hit: d.hit,
                    total: d.blows.len() as u32,
                    good: DUEL_GOOD,
                    over: d.over.is_some(),
                    start_ms: ms(d.started),
                });
                view.keys = if d.over.is_some() {
                    vec![("Enter".into(), "terminar".into())]
                } else {
                    vec![
                        ("A".into(), "izquierda".into()),
                        ("W".into(), "arriba".into()),
                        ("D".into(), "derecha".into()),
                        ("Esc".into(), "dejarlo".into()),
                    ]
                };
                view.stats = vec![
                    (
                        "Paradas".to_owned(),
                        format!("{} / {}", d.parried, d.blows.len()),
                    ),
                    ("Para subir".to_owned(), format!("{DUEL_GOOD}")),
                ];
            }
            Page::Morning => self.routine_view(&mut view),
            Page::Telescope { look } => self.telescope_view(&mut view, *look),
            Page::Codex => self.codex_view(&mut view, c.choice),
            Page::Dates(d) => self.dates_view(d, &mut view),
        }
        Some(view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_runes_are_won_by_repeating_them_and_lost_by_three_mistakes() {
        let mut g = RuneGame::new(3, 7, 0.0);
        let mut now = 0.0;
        for round in 0..RUNE_ROUNDS {
            // Wait out the showing.
            for _ in 0..200 {
                now += 0.05;
                g.tick(now);
                if g.showing.is_none() && g.next_round_at.is_none() {
                    break;
                }
            }
            assert_eq!(g.sequence.len(), 3 + round as usize);
            for i in 0..g.sequence.len() {
                let r = g.sequence[i];
                assert_eq!(g.press(r, now), Some(true));
            }
        }
        assert_eq!(g.over.map(|(won, _)| won), Some(true));

        let mut g = RuneGame::new(3, 11, 0.0);
        let mut now = 0.0;
        for _ in 0..RUNE_LIVES {
            for _ in 0..200 {
                now += 0.05;
                g.tick(now);
                if g.showing.is_none() {
                    break;
                }
            }
            let wrong = (g.sequence[0] + 1) % RUNES as u8;
            assert_eq!(g.press(wrong, now), Some(false));
        }
        assert_eq!(g.over.map(|(won, _)| won), Some(false));
    }

    #[test]
    fn nemi_blows_are_parried_only_from_their_side_and_on_time() {
        let mut d = Duel::new(0.0, 5);
        assert_eq!(d.blows.len(), DUEL_ROUNDS * DUEL_BLOWS);
        let first = d.blows[0];
        // Too early: nothing to parry yet.
        assert!(d.parryable(first.at - 0.5).is_none());
        // On time: the first blow.
        assert_eq!(d.parryable(first.at), Some(0));
        d.decide(0, true, first.at);
        assert_eq!(d.parried, 1);
        // Every blow comes after the one before, and from the three sides.
        assert!(d.blows.windows(2).all(|w| w[1].at > w[0].at));
        assert!(d.blows.iter().all(|b| b.from < 3));
    }

    #[test]
    fn the_castle_files_read_cleanly_when_they_are_there() {
        let dir = crate::exploration3d_state::data_dir().join("castillo");
        if !dir.is_dir() {
            return;
        }
        for (name, ok) in [
            (
                "audiencias.json",
                serde_json::from_str::<Audiencias>(
                    &std::fs::read_to_string(dir.join("audiencias.json")).unwrap_or_default(),
                )
                .is_ok(),
            ),
            (
                "obras.json",
                serde_json::from_str::<Obras>(
                    &std::fs::read_to_string(dir.join("obras.json")).unwrap_or_default(),
                )
                .is_ok(),
            ),
            (
                "lecciones.json",
                serde_json::from_str::<Lecciones>(
                    &std::fs::read_to_string(dir.join("lecciones.json")).unwrap_or_default(),
                )
                .is_ok(),
            ),
            (
                "entrenamiento.json",
                serde_json::from_str::<Entrenamiento>(
                    &std::fs::read_to_string(dir.join("entrenamiento.json")).unwrap_or_default(),
                )
                .is_ok(),
            ),
        ] {
            if dir.join(name).is_file() {
                assert!(ok, "{name}");
            }
        }
        // Every petition's items are things the game knows.
        for p in &audiencias().peticiones {
            for o in &p.opciones {
                for id in o.objetos.keys() {
                    assert!(
                        backpack::extra(id).is_some()
                            || shop::catalog().products.iter().any(|x| &x.id == id),
                        "{}: {id}",
                        p.id
                    );
                }
            }
        }
    }
}
