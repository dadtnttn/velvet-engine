//! The castle's dating sim. On the great hall's red sofas («Citas», a page of the court) the Queen picks whom to
//! spend the day with, Lira, Nyxara or Nemi: four day dates and a first night each (texts in
//! `data/castillo/citas.json`, scenes in `data/guiones/citas.md`). A date opens once the one before
//! is had and her affection (`afecto_<id>`) reaches its mark; one date a day (`cita_hoy`), the
//! nights only at night and with the alcove free (`noche_hoy`). Each date plays on the stage and
//! pays its memories: the shot full screen (`@recuerdo`), kept in the gallery's «Citas» album.
//! The page shows each girl's route and how her affection rises; her memories can be looked at
//! big from it (S, then Enter), and after a date it comes back on her with what the date brought.
use std::sync::OnceLock;

use super::court::{read, Page};
use super::ui_sfx::{self, UiSfx};
use super::*;

// ------------------------------------------------------------------ what the file says

/// The dating sim (`data/castillo/citas.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Citas {
    pub chicas: Vec<Chica>,
}

/// A girl of the court to date.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Chica {
    /// Her id: her portrait in `retratos.json`, and `cita_<id>` / `afecto_<id>`.
    pub id: String,
    pub nombre: String,
    pub oficio: String,
    /// Her portrait's mood on the card.
    pub humor: String,
    /// Her colour (0xRRGGBB).
    pub color: u32,
    /// What her card says once her route is done (empty: the night's invitation from the bed).
    pub fin: String,
    /// How her affection rises (her card says it while a date waits for it).
    pub como: String,
    /// Her tall portrait on the page: a 3:4 cut of her sprite (else her square portrait).
    pub busto: Option<Busto>,
    pub citas: Vec<Cita>,
}

/// A picture under `data/assets` and the cut of it to show `[x, y, w, h]`.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Busto {
    pub imagen: String,
    pub recorte: [u32; 4],
}

/// One date: where, what affection opens it, its scene and the memories it gives.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Cita {
    pub titulo: String,
    pub lugar: String,
    pub afecto: i32,
    /// Only at night (the first night).
    pub noche: bool,
    /// Its guion scene (`citas.<scene>`).
    pub escena: String,
    pub recuerdos: Vec<Recuerdo>,
}

/// A memory: the shot (under `data/assets`), its name, the line the gallery shows with it and the
/// square cut of its little picture on the page `[x, y, w, h]`.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Recuerdo {
    pub imagen: String,
    pub titulo: String,
    pub frase: String,
    pub recorte: [u32; 4],
}

pub(crate) fn citas() -> &'static Citas {
    static C: OnceLock<Citas> = OnceLock::new();
    C.get_or_init(|| read("citas.json"))
}

/// Every memory in order, girl by girl and date by date: (girl, memory).
pub(crate) fn memories() -> Vec<(&'static Chica, &'static Recuerdo)> {
    citas()
        .chicas
        .iter()
        .flat_map(|g| {
            g.citas
                .iter()
                .flat_map(move |c| c.recuerdos.iter().map(move |r| (g, r)))
        })
        .collect()
}

/// Where a girl stands on her route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DateState {
    /// Date `n` (0-based) can be had now.
    Ready(usize),
    /// Date `n` waits: why.
    Waiting(usize, String),
    /// Every date had.
    Done,
}

/// The dates' page as it stands: the keys on the girls or on the lit one's memories (`memory`, the
/// one lit), that memory shown big (`open`, since when), and the date just had (what it brought
/// comes in on her card).
#[derive(Clone, Debug, Default)]
pub(crate) struct DatesPage {
    pub memory: Option<usize>,
    pub open: Option<f32>,
    pub after: Option<DateAfter>,
}

/// A date about to be had: whose, and her affection and the dates had before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DateAfter {
    pub girl: usize,
    pub hearts: i32,
    pub had: usize,
}

/// Her memories in order, each with whether it is hers yet (its date had).
fn memories_of(g: &'static Chica, had: usize) -> Vec<(&'static Recuerdo, bool)> {
    g.citas
        .iter()
        .enumerate()
        .flat_map(|(n, c)| c.recuerdos.iter().map(move |r| (r, n < had)))
        .collect()
}

/// What Chiquita says on a girl's card: why her date waits and how her affection rises, that she
/// is free (by day or at night), or what is left once her route is done.
fn date_say(g: &Chica, state: &DateState, next: Option<&Cita>, affection: i32) -> String {
    match state {
        DateState::Ready(_) if next.is_some_and(|c| c.noche) => {
            "Es de noche y la alcoba está libre. Ve.".to_owned()
        }
        DateState::Ready(_) => "Está libre. Elige bien lo que le dices: cuenta.".to_owned(),
        DateState::Waiting(_, why) if next.is_some_and(|c| affection < c.afecto) => {
            format!("{why} {}", g.como).trim().to_owned()
        }
        DateState::Waiting(_, why) => why.clone(),
        DateState::Done if g.fin.is_empty() => "Las noches con ella pueden ir a más.".to_owned(),
        DateState::Done => "Su ruta está completa. Revive sus recuerdos con S.".to_owned(),
    }
}

/// The memories that the dates before `had` gave.
fn memories_before(g: &Chica, had: usize) -> usize {
    g.citas.iter().take(had).map(|c| c.recuerdos.len()).sum()
}

impl App {
    /// Dates had with girl `g`.
    pub(crate) fn dates_had(&self, g: &Chica) -> usize {
        self.story.get(&format!("cita_{}", g.id)).max(0) as usize
    }

    fn date_night(&self) -> bool {
        !(0.2..=0.8).contains(&self.environment_cycle.phase())
    }

    /// Whether girl `girl`'s next date can be had now, and if not, why.
    pub(crate) fn date_state(&self, girl: usize) -> DateState {
        let Some(g) = citas().chicas.get(girl) else {
            return DateState::Done;
        };
        let had = self.dates_had(g);
        let Some(next) = g.citas.get(had) else {
            return DateState::Done;
        };
        let affection = self.story.get(&format!("afecto_{}", g.id));
        let why = if self.story.get("cita_hoy") > 0 {
            "Hoy ya has tenido una cita. Duerme y vuelve mañana.".to_owned()
        } else if affection + 1 == next.afecto {
            "Le falta 1 punto de afecto para esta cita.".to_owned()
        } else if affection < next.afecto {
            format!(
                "Le faltan {} puntos de afecto para esta cita.",
                next.afecto - affection
            )
        } else if next.noche && !self.date_night() {
            "Esta cita es de noche: vuelve cuando caiga el sol.".to_owned()
        } else if next.noche && self.story.get("noche_hoy") > 0 {
            "Esta noche ya tienes compañía.".to_owned()
        } else {
            return DateState::Ready(had);
        };
        DateState::Waiting(had, why)
    }

    fn dates_page(&self) -> Option<&DatesPage> {
        match self.court.as_ref().map(|c| &c.page) {
            Some(Page::Dates(d)) => Some(d),
            _ => None,
        }
    }

    fn dates_page_mut(&mut self) -> Option<&mut DatesPage> {
        match self.court.as_mut().map(|c| &mut c.page) {
            Some(Page::Dates(d)) => Some(d),
            _ => None,
        }
    }

    /// F at the great hall's sofas: the dates' page.
    pub(super) fn open_dates(&mut self) -> bool {
        if citas().chicas.is_empty() {
            return false;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Dates(DatesPage::default()));
        // The first girl with a date ready is lit.
        let n = citas().chicas.len();
        if let Some(first) = (0..n).find(|&i| matches!(self.date_state(i), DateState::Ready(_))) {
            if let Some(c) = self.court.as_mut() {
                c.choice = first;
            }
        }
        if self.story.get("citas_vistas") == 0 {
            self.story.values.insert("citas_vistas".to_owned(), 1);
            self.float_chiquita_after(
                "—¿Una cita? Con mi cara, mi pelo y mi vestido. Va a salir de maravilla. Elige bien: lo que les digas cuenta.",
                mascot::Cue::Talk,
                0.6,
            );
        }
        true
    }

    /// Back from a date: the page again, on her, with what the date brought coming in.
    pub(super) fn dates_back(&mut self, after: DateAfter) {
        if citas().chicas.get(after.girl).is_none() {
            return;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Dates(DatesPage {
            after: Some(after),
            ..Default::default()
        }));
        if let Some(c) = self.court.as_mut() {
            c.choice = after.girl;
        }
    }

    /// Enter on girl `pick`: her next date plays, if it can.
    pub(super) fn dates_pick(&mut self, pick: usize) {
        let Some(g) = citas().chicas.get(pick) else {
            return;
        };
        match self.date_state(pick) {
            DateState::Ready(n) => {
                let scene = g.citas[n].escena.clone();
                let hearts = self.story.get(&format!("afecto_{}", g.id));
                ui_sfx::play(UiSfx::Select);
                self.close_court();
                if self.start_talk_at(&scene) {
                    self.date_after = Some(DateAfter {
                        girl: pick,
                        hearts,
                        had: n,
                    });
                } else {
                    eprintln!("[citas] no hay escena {scene}");
                }
            }
            DateState::Waiting(_, why) => {
                ui_sfx::play(UiSfx::Error);
                self.court_notice(why);
            }
            DateState::Done => {
                ui_sfx::play(UiSfx::Error);
                self.court_notice(if g.fin.is_empty() {
                    format!(
                        "Ruta de {} completa. De noche, invítala desde la cama.",
                        g.nombre
                    )
                } else {
                    g.fin.clone()
                });
            }
        }
    }

    /// Girl `girl`'s memories, each with whether it is hers yet.
    fn girl_memories(&self, girl: usize) -> Vec<(&'static Recuerdo, bool)> {
        citas()
            .chicas
            .get(girl)
            .map(|g| memories_of(g, self.dates_had(g)))
            .unwrap_or_default()
    }

    /// The keys on the dates' page that are its own (her memories, and one shown big); the rest
    /// go the court's way (A D between the girls, Enter, Esc). Whether the key was taken.
    pub(super) fn dates_key(&mut self, code: KeyCode) -> bool {
        let Some(page) = self.dates_page().cloned() else {
            return false;
        };
        let girl = self.court.as_ref().map_or(0, |c| c.choice);
        let left = matches!(code, KeyCode::ArrowLeft | KeyCode::KeyA);
        let right = matches!(code, KeyCode::ArrowRight | KeyCode::KeyD);
        let up = matches!(code, KeyCode::ArrowUp | KeyCode::KeyW);
        let down = matches!(code, KeyCode::ArrowDown | KeyCode::KeyS);
        let enter = matches!(
            code,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::KeyF
        );
        let back = matches!(code, KeyCode::Escape | KeyCode::Backspace);
        match (page.memory, page.open) {
            // a memory shown big: the next of hers, or back to the row
            (Some(m), Some(_)) => {
                if left || right {
                    self.dates_step_memory(girl, m, right);
                } else if back || enter || up || down {
                    ui_sfx::play(UiSfx::Back);
                    if let Some(d) = self.dates_page_mut() {
                        d.open = None;
                    }
                }
                true
            }
            // her memories' row
            (Some(m), None) => {
                let count = self.girl_memories(girl).len().max(1);
                if left || right {
                    let to = if right {
                        (m + 1) % count
                    } else {
                        (m + count - 1) % count
                    };
                    ui_sfx::play(UiSfx::Move);
                    if let Some(d) = self.dates_page_mut() {
                        d.memory = Some(to);
                    }
                } else if enter {
                    self.dates_open_memory(girl, m);
                } else if up || back {
                    ui_sfx::play(UiSfx::Back);
                    if let Some(d) = self.dates_page_mut() {
                        d.memory = None;
                    }
                }
                true
            }
            // the girls: down to her memories; above them there is nothing
            _ if down => {
                self.dates_to_memories(girl);
                true
            }
            _ => up,
        }
    }

    /// Down from a girl to her memories' row (the first she has lit).
    fn dates_to_memories(&mut self, girl: usize) {
        let mems = self.girl_memories(girl);
        match mems.iter().position(|(_, had)| *had) {
            Some(first) => {
                ui_sfx::play(UiSfx::Move);
                if let Some(d) = self.dates_page_mut() {
                    d.memory = Some(first);
                    d.after = None;
                }
            }
            None => {
                ui_sfx::play(UiSfx::Error);
                self.court_notice("Aún no tienes recuerdos con ella: ten una cita.");
            }
        }
    }

    /// Memory `m` of girl `girl` shown big, if it is hers.
    fn dates_open_memory(&mut self, girl: usize, m: usize) {
        let had = self.girl_memories(girl).get(m).is_some_and(|(_, had)| *had);
        if !had {
            ui_sfx::play(UiSfx::Error);
            self.court_notice("Este recuerdo aún no es tuyo: llegará con su cita.");
            return;
        }
        let now = self.elapsed;
        ui_sfx::play(UiSfx::Select);
        if let Some(d) = self.dates_page_mut() {
            d.memory = Some(m);
            d.open = Some(now);
            d.after = None;
        }
    }

    /// The next (or the one before) of her memories she has, shown big.
    fn dates_step_memory(&mut self, girl: usize, m: usize, forward: bool) {
        let mems = self.girl_memories(girl);
        let n = mems.len();
        if n == 0 {
            return;
        }
        let to = (1..=n)
            .map(|k| {
                if forward {
                    (m + k) % n
                } else {
                    (m + n * 2 - k) % n
                }
            })
            .find(|&i| mems[i].1);
        if let Some(to) = to.filter(|&to| to != m) {
            let now = self.elapsed;
            ui_sfx::play(UiSfx::Move);
            if let Some(d) = self.dates_page_mut() {
                d.memory = Some(to);
                d.open = Some(now);
            }
        }
    }

    /// The pointer over the dates' page: a card (`hit` < 100) or a memory on one (100 + 16 × the
    /// girl + the memory) lights.
    pub(super) fn dates_hover(&mut self, hit: usize) {
        let Some(page) = self.dates_page().cloned() else {
            return;
        };
        if page.open.is_some()
            || matches!(
                hit,
                map_overlay::MEMORY_SHOWN
                    | map_overlay::DATE_GO_HIT
                    | map_overlay::DATE_MEMORIES_HIT
            )
        {
            return;
        }
        let (girl, memory) = if hit >= 100 {
            ((hit - 100) / 16, Some((hit - 100) % 16))
        } else {
            (hit, None)
        };
        let now = self.elapsed;
        let changed = self.court.as_ref().is_some_and(|c| c.choice != girl) || page.memory != memory;
        if changed {
            ui_sfx::play(UiSfx::Move);
            if let Some(c) = self.court.as_mut() {
                c.choice = girl;
                c.choice_at = now;
            }
            if let Some(d) = self.dates_page_mut() {
                d.memory = memory;
            }
        }
    }

    /// A click on the dates' page: on a memory, it shown big; on a card, lit, then its date; on a
    /// memory shown big, back.
    pub(super) fn dates_click(&mut self, hit: Option<usize>) {
        let Some(page) = self.dates_page().cloned() else {
            return;
        };
        if page.open.is_some() {
            ui_sfx::play(UiSfx::Back);
            if let Some(d) = self.dates_page_mut() {
                d.open = None;
            }
            return;
        }
        let girl = self.court.as_ref().map_or(0, |c| c.choice);
        match hit {
            Some(map_overlay::DATE_GO_HIT) => self.dates_pick(girl),
            Some(map_overlay::DATE_MEMORIES_HIT) => self.dates_to_memories(girl),
            Some(h) if h >= 100 && h != map_overlay::MEMORY_SHOWN => {
                let (girl, m) = ((h - 100) / 16, (h - 100) % 16);
                if let Some(c) = self.court.as_mut() {
                    c.choice = girl;
                }
                self.dates_open_memory(girl, m);
            }
            Some(girl) if girl < 100 => {
                let lit = self.court.as_ref().is_some_and(|c| c.choice == girl);
                if lit && page.memory.is_none() {
                    self.dates_pick(girl);
                } else {
                    ui_sfx::play(UiSfx::Move);
                    let now = self.elapsed;
                    if let Some(c) = self.court.as_mut() {
                        c.choice = girl;
                        c.choice_at = now;
                    }
                    if let Some(d) = self.dates_page_mut() {
                        d.memory = None;
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn dates_view(&self, page: &DatesPage, view: &mut map_overlay::CourtView) {
        use map_overlay::{CourtCard, DateView, MemoryShow, PortraitRef};
        let night = self.date_night();
        let lit = self.court.as_ref().map_or(0, |c| c.choice);
        let now = self.elapsed;
        view.kind = 11;
        view.title = "Citas".to_owned();
        view.tag = if night {
            "EL GRAN SALÓN · ¿CON QUIÉN PASAS LA NOCHE?".to_owned()
        } else {
            "EL GRAN SALÓN · ¿CON QUIÉN PASAS EL DÍA?".to_owned()
        };
        view.accent = 0xff6fa8;
        // (no stats: the title says day or night, each girl's row and card whether she can go)
        view.stats = Vec::new();
        for (i, g) in citas().chicas.iter().enumerate() {
            let had = self.dates_had(g);
            let state = self.date_state(i);
            let next = g.citas.get(had);
            let affection = self.story.get(&format!("afecto_{}", g.id));
            let mems = memories_of(g, had);
            let focus = page.memory.filter(|_| i == lit);
            let fresh = page.after.filter(|a| a.girl == i).map(|a| {
                (
                    a.hearts,
                    a.had.min(had) as u8,
                    memories_before(g, a.had.min(had)) as u8,
                )
            });
            view.cards.push(CourtCard {
                title: g.nombre.clone(),
                detail: g.oficio.clone(),
                chips: Vec::new(),
                picture: self.court_portrait(&g.id, Some(&g.humor)),
                icon: String::new(),
                state: match state {
                    DateState::Ready(_) => 0,
                    DateState::Waiting(..) => 1,
                    DateState::Done => 2,
                },
            });
            view.dates.push(DateView {
                name: g.nombre.clone(),
                role: g.oficio.clone(),
                color: g.color,
                hearts: affection,
                need: next.map_or(0, |c| c.afecto),
                done: had as u8,
                total: g.citas.len() as u8,
                next: next.map_or_else(|| "Ruta completa".to_owned(), |c| c.titulo.clone()),
                place: next.map_or_else(
                    || {
                        if g.fin.is_empty() {
                            "De noche, invítala desde la cama.".to_owned()
                        } else {
                            g.fin.clone()
                        }
                    },
                    |c| c.lugar.clone(),
                ),
                nights: g.fin.is_empty(),
                say: date_say(g, &state, next, affection),
                why: match state {
                    DateState::Waiting(_, why) => why,
                    _ => String::new(),
                },
                sprite: g.busto.as_ref().map(|b| b.imagen.clone()).unwrap_or_default(),
                reward: next.map_or(0, |c| c.recuerdos.len() as u8),
                night: next.is_some_and(|c| c.noche),
                route: g.citas.iter().map(|c| c.noche).collect(),
                memories: mems
                    .iter()
                    .map(|(r, open)| {
                        (
                            PortraitRef {
                                image: r.imagen.clone(),
                                crop: r.recorte,
                            },
                            *open,
                        )
                    })
                    .collect(),
                focus: focus.map(|m| m as u8),
                focus_title: focus
                    .and_then(|m| mems.get(m))
                    .map(|(r, open)| {
                        if *open {
                            r.titulo.clone()
                        } else {
                            "Aún no es tuyo".to_owned()
                        }
                    })
                    .unwrap_or_default(),
                fresh,
            });
        }
        // the girls' sprites and the lit one's memories load ahead, to come up at once
        view.preload = citas()
            .chicas
            .iter()
            .filter_map(|g| g.busto.as_ref().map(|b| b.imagen.clone()))
            .chain(
                self.girl_memories(lit)
                    .iter()
                    .filter(|(_, had)| *had)
                    .map(|(r, _)| r.imagen.clone()),
            )
            .collect();
        if let (Some(m), Some(at), Some(g)) = (page.memory, page.open, citas().chicas.get(lit)) {
            let mems = self.girl_memories(lit);
            if let Some((r, true)) = mems.get(m) {
                let had: Vec<usize> = (0..mems.len()).filter(|&k| mems[k].1).collect();
                view.show = Some(MemoryShow {
                    image: r.imagen.clone(),
                    title: r.titulo.clone(),
                    line: r.frase.clone(),
                    girl: g.nombre.clone(),
                    color: g.color,
                    index: had.iter().position(|&k| k == m).unwrap_or(0) as u8 + 1,
                    count: had.len() as u8,
                    age_ms: ((now - at).max(0.0) * 1000.0) as u32,
                });
            }
        }
        view.keys = if view.show.is_some() {
            vec![
                ("A D".into(), "otro recuerdo".into()),
                ("Esc".into(), "volver".into()),
            ]
        } else if page.memory.is_some() {
            vec![
                ("A D".into(), "elegir recuerdo".into()),
                ("Enter".into(), "verlo".into()),
                ("W".into(), "volver".into()),
            ]
        } else {
            vec![
                ("A D".into(), "elegir".into()),
                ("Enter".into(), "pedirle una cita".into()),
                ("S".into(), "sus recuerdos".into()),
                ("Esc".into(), "cerrar".into()),
            ]
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dates_file_reads_and_every_date_has_its_scene_and_pictures() {
        let c = citas();
        assert_eq!(c.chicas.len(), 3);
        let lib = guion::library();
        let root = crate::exploration3d_state::data_dir().join("assets");
        for g in &c.chicas {
            assert_eq!(g.citas.len(), 5, "{}", g.id);
            assert!(!g.como.is_empty(), "{}: cómo sube su afecto", g.id);
            let mut last = -1;
            for cita in &g.citas {
                assert!(
                    cita.afecto > last,
                    "{} {}: el afecto sube",
                    g.id,
                    cita.titulo
                );
                last = cita.afecto;
                assert!(
                    lib.find(&cita.escena, None).is_some(),
                    "falta la escena {}",
                    cita.escena
                );
                assert!(!cita.recuerdos.is_empty());
                for r in &cita.recuerdos {
                    assert!(
                        root.join(format!("{}.png", r.imagen)).is_file(),
                        "falta {}",
                        r.imagen
                    );
                    let [x, y, w, h] = r.recorte;
                    assert!(w == h && x + w <= 1920 && y + h <= 1080, "{}", r.imagen);
                }
            }
            assert!(
                g.citas.last().is_some_and(|c| c.noche),
                "{}: la última es de noche",
                g.id
            );
        }
        assert_eq!(memories().len(), 18);
    }

    #[test]
    fn a_girls_memories_are_hers_date_by_date() {
        let g = &citas().chicas[0];
        assert!(memories_of(g, 0).iter().all(|(_, had)| !had));
        let one = memories_of(g, 1);
        assert_eq!(one.iter().filter(|(_, had)| *had).count(), g.citas[0].recuerdos.len());
        assert!(one[0].1 && !one[one.len() - 1].1);
        assert_eq!(memories_before(g, 0), 0);
        assert_eq!(memories_before(g, g.citas.len()), memories_of(g, 0).len());
    }

    #[test]
    fn every_date_scene_pays_its_memories_and_moves_the_route_on() {
        let path = crate::exploration3d_state::data_dir().join("guiones/citas.md");
        let text = std::fs::read_to_string(path).unwrap();
        for g in &citas().chicas {
            for (n, cita) in g.citas.iter().enumerate() {
                for r in &cita.recuerdos {
                    assert!(
                        text.contains(&format!("@recuerdo {} | {}", r.imagen, r.titulo)),
                        "{} no enseña {}",
                        cita.escena,
                        r.imagen
                    );
                }
                assert!(
                    text.contains(&format!("@var cita_{} = {}", g.id, n + 1)),
                    "{} no avanza la ruta",
                    cita.escena
                );
            }
        }
    }
}
