//! The court's days. The morning's routine: each of the court does the task she was given (it
//! stays given until changed), tells in the morning how it went, and the Queen hands out today's.
//! The balcony's telescope: north, east, south and west, each with what can be seen there now; some
//! sightings open side missions. The library's book of lore: entries that open as the story, the
//! restored library, Nyxara's study of the seal and Maren's reports go on. Texts in
//! `data/castillo/{tareas,catalejo,codice}.json`; the progress in the story's variables.
use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::court::{item_name, read, Page};
use super::ui_sfx::{self, UiSfx};
use super::*;
use demon_queen_dungeon::i18n::tr;

// ------------------------------------------------------------------ what the files say

/// The court's tasks (`data/castillo/tareas.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Tareas {
    /// When the routine begins (a guion condition).
    pub desde: String,
    pub personas: Vec<Persona>,
    /// What one says when her task could not be done.
    pub cerrada: String,
}

/// One of the court who takes a task.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Persona {
    pub id: String,
    pub nombre: String,
    pub oficio: String,
    /// Her portrait's key in `retratos.json`, and its mood.
    pub retrato: String,
    pub animo: String,
    /// When she is there to be given one.
    pub desde: String,
    pub tareas: Vec<Tarea>,
}

/// A day's task: what it is, what it needs, what it gives, what she says of it.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Tarea {
    pub id: String,
    pub titulo: String,
    pub texto: String,
    pub requiere: String,
    pub falta: String,
    pub oro: i32,
    pub reputacion: i32,
    pub objetos: BTreeMap<String, u32>,
    pub vars: BTreeMap<String, i32>,
    pub parte: Vec<String>,
}

/// The telescope (`data/castillo/catalejo.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Catalejo {
    pub direcciones: Vec<Direccion>,
    pub vistas: Vec<Vista>,
    pub nada: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Direccion {
    pub id: String,
    pub nombre: String,
    pub lugar: String,
}

/// Something to be seen one way.
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Vista {
    pub id: String,
    pub direccion: String,
    pub desde: String,
    pub titulo: String,
    pub imagen: String,
    pub recorte: [u32; 4],
    pub texto: String,
    pub chiquita: String,
    pub vars: BTreeMap<String, i32>,
}

/// The library's book of lore (`data/castillo/codice.json`).
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Codice {
    pub entradas: Vec<Entrada>,
    /// The last card: the old Queen's diary (its guion scene, `castillo.estudio`).
    pub diario: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub(crate) struct Entrada {
    pub id: String,
    pub grupo: String,
    pub titulo: String,
    pub desde: String,
    /// What is shown while it is closed (where to look for it).
    pub cerrada: String,
    pub texto: String,
    pub chiquita: String,
}

pub(crate) fn tareas() -> &'static Tareas {
    static T: OnceLock<Tareas> = OnceLock::new();
    T.get_or_init(|| read("tareas.json"))
}

pub(crate) fn catalejo() -> &'static Catalejo {
    static C: OnceLock<Catalejo> = OnceLock::new();
    C.get_or_init(|| read("catalejo.json"))
}

pub(crate) fn codice() -> &'static Codice {
    static C: OnceLock<Codice> = OnceLock::new();
    C.get_or_init(|| read("codice.json"))
}

/// What one of the court told in the morning.
#[derive(Clone, Debug)]
pub(crate) struct Report {
    /// Who (an index of `personas`).
    pub who: usize,
    pub line: String,
    /// What it gave: text and tone (1 good, 2 bad, 3 gold).
    pub chips: Vec<(String, u8)>,
}

/// A line from a list, turning with `k`.
fn turn(lines: &[String], k: usize) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        lines[k % lines.len()].clone()
    }
}

// ------------------------------------------------------------------ the routine

impl App {
    /// The routine has begun (the court is the Queen's to run).
    pub(crate) fn routine_open(&self) -> bool {
        !tareas().personas.is_empty() && missions::holds(&tareas().desde, &self.story)
    }

    /// Who of the court is there to be given a task (indexes of `personas`).
    pub(crate) fn routine_people(&self) -> Vec<usize> {
        tareas()
            .personas
            .iter()
            .enumerate()
            .filter(|(_, p)| missions::holds(&p.desde, &self.story))
            .map(|(i, _)| i)
            .collect()
    }

    fn task_open(&self, t: &Tarea) -> bool {
        missions::holds(&t.requiere, &self.story)
    }

    /// The task `who` has (the one given, if she can do it; else the first she can).
    pub(crate) fn assigned(&self, who: usize) -> Option<usize> {
        let p = tareas().personas.get(who)?;
        let given = self.story.get(&format!("tarea_{}", p.id));
        if given > 0 {
            let i = (given - 1) as usize;
            if p.tareas.get(i).is_some_and(|t| self.task_open(t)) {
                return Some(i);
            }
        }
        p.tareas.iter().position(|t| self.task_open(t))
    }

    /// The morning: everyone does the task she has and tells how it went (from `court_new_day`).
    pub(super) fn routine_morning(&mut self) {
        self.court_report.clear();
        if !self.routine_open() {
            return;
        }
        let day = {
            let d = self
                .story
                .values
                .entry("dias_corte".to_owned())
                .or_insert(0);
            *d += 1;
            *d as usize
        };
        for who in self.routine_people() {
            let p = &tareas().personas[who];
            let Some(ti) = self.assigned(who) else {
                continue;
            };
            let t = &p.tareas[ti];
            let mut chips = Vec::new();
            if t.oro != 0 {
                self.gold = (self.gold as i64 + t.oro as i64).max(0) as u32;
                chips.push((format!("{:+} oro", t.oro), if t.oro > 0 { 3 } else { 2 }));
            }
            if t.reputacion != 0 {
                *self
                    .story
                    .values
                    .entry("reputacion".to_owned())
                    .or_insert(0) += t.reputacion;
                chips.push((format!("{:+} reputación", t.reputacion), 1));
            }
            for (id, n) in &t.objetos {
                self.court_give(id, *n);
                chips.push((format!("+{n} {}", tr(&item_name(id))), 1));
            }
            for (var, n) in &t.vars {
                *self.story.values.entry(guion::slug(var)).or_insert(0) += n;
            }
            self.court_report.push(Report {
                who,
                line: turn(&t.parte, day + who * 7),
                chips,
            });
        }
        // Maren, gone east free and a friend: a letter now and then, unsigned, in her hand.
        self.court_letter = false;
        let away = self.story.get("prisionera_libre") > 0 && self.story.get("prisionera_amiga") > 0;
        if away && day % 2 == 0 && self.story.get("maren_informes") < 4 {
            *self
                .story
                .values
                .entry("maren_informes".to_owned())
                .or_insert(0) += 1;
            self.court_letter = true;
        }
        self.court_morning_pending = !self.court_report.is_empty();
    }

    /// The morning's page: what they did, and today's tasks.
    pub(super) fn open_routine(&mut self) -> bool {
        if !self.routine_open() || self.routine_people().is_empty() {
            return false;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_morning_pending = false;
        self.court_show(Page::Morning);
        if std::mem::take(&mut self.court_letter) {
            self.court_notice(
                "Ha llegado una carta del este, sin firma. La letra es de Maren: un informe nuevo en la biblioteca.",
            );
        }
        true
    }

    /// The task cards in order: (who, task) for each.
    pub(crate) fn routine_cards(&self) -> Vec<(usize, usize)> {
        self.routine_people()
            .into_iter()
            .flat_map(|who| (0..tareas().personas[who].tareas.len()).map(move |t| (who, t)))
            .collect()
    }

    /// Card `pick` given: that task is hers from tomorrow on.
    pub(super) fn routine_assign(&mut self, pick: usize) {
        let Some(&(who, ti)) = self.routine_cards().get(pick) else {
            return;
        };
        let p = &tareas().personas[who];
        let t = &p.tareas[ti];
        if !self.task_open(t) {
            ui_sfx::play(UiSfx::Error);
            self.court_notice(if t.falta.is_empty() {
                "Todavía no se puede.".to_owned()
            } else {
                t.falta.clone()
            });
            return;
        }
        self.story
            .values
            .insert(format!("tarea_{}", p.id), ti as i32 + 1);
        ui_sfx::play(UiSfx::Select);
        self.court_notice(format!("{}: «{}», desde mañana.", p.nombre, t.titulo));
        self.persist_3d_state(true);
    }

    /// After sleeping: the morning's page, once she is free to see it (in the castle, nothing else
    /// up).
    pub(super) fn tick_routine(&mut self) {
        if !self.court_morning_pending || self.castle.is_none() {
            return;
        }
        let busy = self.court.is_some()
            || self.talk.is_some()
            || self.alcove.is_some()
            || self.stage_up()
            || self.caricias_open()
            || self.shop_open
            || self.wardrobe_open;
        if !busy {
            self.open_routine();
        }
    }
}

// ------------------------------------------------------------------ the telescope

impl App {
    /// What can be seen towards direction `d` now: the first sighting not seen yet, else the last
    /// one seen.
    pub(crate) fn sighting(&self, d: usize) -> Option<usize> {
        let dir = catalejo().direcciones.get(d)?;
        let open: Vec<usize> = catalejo()
            .vistas
            .iter()
            .enumerate()
            .filter(|(_, v)| v.direccion == dir.id && missions::holds(&v.desde, &self.story))
            .map(|(i, _)| i)
            .collect();
        let seen = |i: &usize| {
            self.story
                .get(&format!("vista_{}", catalejo().vistas[*i].id))
                > 0
        };
        open.iter()
            .copied()
            .find(|i| !seen(i))
            .or_else(|| open.iter().copied().rev().find(seen))
    }

    /// Whether direction `d` has something not seen yet.
    pub(crate) fn sighting_new(&self, d: usize) -> bool {
        self.sighting(d).is_some_and(|i| {
            self.story
                .get(&format!("vista_{}", catalejo().vistas[i].id))
                == 0
        })
    }

    pub(super) fn open_telescope(&mut self) -> bool {
        if catalejo().direcciones.is_empty() {
            return false;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Telescope { look: None });
        true
    }

    /// Looking towards direction `d`.
    pub(super) fn telescope_look(&mut self, d: usize) {
        let first = self.sighting_new(d);
        if let Some(i) = self.sighting(d) {
            let v = &catalejo().vistas[i];
            if first {
                self.story.values.insert(format!("vista_{}", v.id), 1);
                for (var, n) in &v.vars {
                    *self.story.values.entry(guion::slug(var)).or_insert(0) += n;
                }
                self.persist_3d_state(true);
            }
            if !v.chiquita.trim().is_empty() {
                self.float_chiquita_after(&v.chiquita, mascot::Cue::Talk, 0.9);
            }
        }
        ui_sfx::play(UiSfx::Select);
        self.court_show(Page::Telescope { look: Some(d) });
    }
}

// ------------------------------------------------------------------ the book of lore

impl App {
    /// Whether entry `i` is open.
    pub(crate) fn entry_open(&self, i: usize) -> bool {
        codice()
            .entradas
            .get(i)
            .is_some_and(|e| missions::holds(&e.desde, &self.story))
    }

    fn entry_read(&self, i: usize) -> bool {
        codice()
            .entradas
            .get(i)
            .is_some_and(|e| self.story.get(&format!("codice_{}", e.id)) > 0)
    }

    pub(super) fn open_codex(&mut self) -> bool {
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Codex);
        // The first entry open and not read yet is lit.
        let n = codice().entradas.len();
        if let Some(first) = (0..n).find(|&i| self.entry_open(i) && !self.entry_read(i)) {
            if let Some(c) = self.court.as_mut() {
                c.choice = first;
            }
        }
        true
    }

    /// Entry `pick` read (the last card: the old Queen's diary).
    pub(super) fn codex_pick(&mut self, pick: usize) {
        let n = codice().entradas.len();
        if pick >= n {
            self.close_court();
            self.start_talk(None, Some("estudio"));
            return;
        }
        if !self.entry_open(pick) {
            ui_sfx::play(UiSfx::Error);
            return;
        }
        let e = &codice().entradas[pick];
        let first = !self.entry_read(pick);
        self.story.values.insert(format!("codice_{}", e.id), 1);
        ui_sfx::play(UiSfx::Move);
        if first && !e.chiquita.trim().is_empty() {
            self.float_chiquita_after(&e.chiquita, mascot::Cue::Talk, 0.5);
        }
        if first {
            self.persist_3d_state(true);
        }
    }
}

// ------------------------------------------------------------------ what the overlay shows

impl App {
    pub(super) fn routine_view(&self, view: &mut map_overlay::CourtView) {
        use map_overlay::{CourtCard, CourtChip, CourtRow};
        let chip = |text: String, tone: u8| CourtChip { text, tone };
        view.kind = 8;
        view.title = "La mañana en la corte".to_owned();
        view.tag = format!(
            "DÍA {} · LO QUE HICIERON AYER · LO QUE HARÁN HOY",
            self.story.get("dias_corte").max(1)
        );
        view.accent = 0xffb347;
        let mut first = 0u8;
        for who in self.routine_people() {
            let p = &tareas().personas[who];
            let given = self.assigned(who);
            let report = self.court_report.iter().find(|r| r.who == who);
            view.rows.push(CourtRow {
                name: p.nombre.clone(),
                role: p.oficio.clone(),
                report: report.map(|r| r.line.clone()).unwrap_or_default(),
                chips: report
                    .map(|r| r.chips.iter().map(|(t, k)| chip(t.clone(), *k)).collect())
                    .unwrap_or_default(),
                portrait: guion::portraits()
                    .find(&p.retrato, Some(p.animo.as_str()).filter(|m| !m.is_empty()))
                    .map(|q| map_overlay::PortraitRef {
                        image: q.image,
                        crop: q.crop,
                    }),
                first,
                count: p.tareas.len() as u8,
            });
            first += p.tareas.len() as u8;
            for (ti, t) in p.tareas.iter().enumerate() {
                let open = self.task_open(t);
                let mut chips = Vec::new();
                if given == Some(ti) {
                    chips.push(chip("HOY".into(), 3));
                }
                if t.oro != 0 {
                    chips.push(chip(format!("{:+} oro", t.oro), 3));
                }
                if t.reputacion != 0 {
                    chips.push(chip(format!("{:+} rep.", t.reputacion), 1));
                }
                for (id, n) in &t.objetos {
                    chips.push(chip(format!("+{n} {}", tr(&item_name(id))), 1));
                }
                view.cards.push(CourtCard {
                    title: t.titulo.clone(),
                    detail: if open {
                        t.texto.clone()
                    } else {
                        t.falta.clone()
                    },
                    chips,
                    picture: None,
                    icon: String::new(),
                    state: if !open {
                        1
                    } else if given == Some(ti) {
                        2
                    } else {
                        0
                    },
                });
            }
        }
        view.keys = vec![
            ("A D W S".into(), "elegir".into()),
            ("Enter".into(), "dar la tarea".into()),
            ("Esc".into(), "a trabajar".into()),
        ];
    }

    pub(super) fn telescope_view(&self, view: &mut map_overlay::CourtView, look: Option<usize>) {
        use map_overlay::{CourtCard, CourtChip};
        view.kind = 9;
        view.title = "El catalejo".to_owned();
        view.tag = "EL BALCÓN · LO QUE SE VE DESDE EL CASTILLO".to_owned();
        view.accent = 0xe2b04a;
        view.cards = catalejo()
            .direcciones
            .iter()
            .enumerate()
            .map(|(d, dir)| CourtCard {
                title: dir.nombre.clone(),
                detail: dir.lugar.clone(),
                chips: if self.sighting_new(d) {
                    vec![CourtChip {
                        text: "ALGO NUEVO".into(),
                        tone: 3,
                    }]
                } else {
                    Vec::new()
                },
                picture: None,
                icon: String::new(),
                state: 0,
            })
            .collect();
        match look {
            None => {
                view.keys = vec![
                    ("A D W S".into(), "dirección".into()),
                    ("Enter".into(), "mirar".into()),
                    ("Esc".into(), "dejarlo".into()),
                ];
            }
            Some(d) => {
                view.scope = true;
                view.choice = d as u8;
                let dir = &catalejo().direcciones[d];
                view.role = format!("{} · {}", dir.nombre, dir.lugar);
                match self.sighting(d).map(|i| &catalejo().vistas[i]) {
                    Some(v) => {
                        view.speaker = v.titulo.clone();
                        view.text = v.texto.clone();
                        view.portrait = (!v.imagen.is_empty()).then(|| map_overlay::PortraitRef {
                            image: v.imagen.clone(),
                            crop: v.recorte,
                        });
                        let opens = v.vars.keys().any(|k| k.starts_with("cat_"));
                        if opens {
                            view.chips.push(CourtChip {
                                text: "MISIÓN EN EL DIARIO (J)".into(),
                                tone: 3,
                            });
                        }
                    }
                    None => {
                        view.speaker = dir.nombre.clone();
                        view.text = catalejo().nada.clone();
                    }
                }
                view.keys = vec![
                    ("Enter".into(), "otra dirección".into()),
                    ("Esc".into(), "dejarlo".into()),
                ];
            }
        }
    }

    pub(super) fn codex_view(&self, view: &mut map_overlay::CourtView, choice: usize) {
        use map_overlay::{CourtCard, CourtChip};
        view.kind = 10;
        view.title = "La biblioteca".to_owned();
        view.tag = "LO QUE CUENTAN LOS LIBROS DEL CASTILLO".to_owned();
        view.accent = 0x7fb8ff;
        let c = codice();
        for (i, e) in c.entradas.iter().enumerate() {
            let open = self.entry_open(i);
            let fresh = open && !self.entry_read(i);
            view.cards.push(CourtCard {
                title: if open {
                    e.titulo.clone()
                } else {
                    "· · ·".to_owned()
                },
                detail: e.grupo.clone(),
                chips: if fresh {
                    vec![CourtChip {
                        text: "NUEVO".into(),
                        tone: 3,
                    }]
                } else {
                    Vec::new()
                },
                picture: None,
                icon: String::new(),
                state: if open { 0 } else { 1 },
            });
        }
        view.cards.push(CourtCard {
            title: c.diario.clone(),
            detail: "El estudio".to_owned(),
            chips: Vec::new(),
            picture: None,
            icon: String::new(),
            state: 0,
        });
        match c.entradas.get(choice) {
            Some(e) if self.entry_open(choice) => {
                view.speaker = e.titulo.clone();
                view.role = e.grupo.clone();
                view.text = e.texto.clone();
            }
            Some(e) => {
                view.speaker = "· · ·".to_owned();
                view.role = e.grupo.clone();
                view.text = e.cerrada.clone();
            }
            None => {
                view.speaker = c.diario.clone();
                view.role = "El estudio".to_owned();
                view.text =
                    "Un diario abierto, con la letra de la Reina de antes. La última página está en blanco."
                        .to_owned();
            }
        }
        let open_n = (0..c.entradas.len())
            .filter(|&i| self.entry_open(i))
            .count();
        view.stats = vec![(
            "Entradas".to_owned(),
            format!("{open_n} / {}", c.entradas.len()),
        )];
        view.keys = vec![
            ("W S".into(), "pasar página".into()),
            ("Enter".into(), "leer".into()),
            ("Esc".into(), "cerrar".into()),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_court_files_read_and_point_at_real_things() {
        let t = tareas();
        assert!(t.personas.len() >= 3);
        for p in &t.personas {
            assert_eq!(p.tareas.len(), 4, "{}", p.id);
            for task in &p.tareas {
                assert!(!task.parte.is_empty(), "{}.{}", p.id, task.id);
                for id in task.objetos.keys() {
                    assert_ne!(item_name(id), *id, "{id} is a real thing");
                }
            }
        }
        let c = catalejo();
        assert_eq!(c.direcciones.len(), 4);
        for v in &c.vistas {
            assert!(
                c.direcciones.iter().any(|d| d.id == v.direccion),
                "{}",
                v.id
            );
            let [x, y, w, h] = v.recorte;
            assert!(w > 0 && h > 0 && x + w <= 1920 && y + h <= 1080, "{}", v.id);
        }
        assert!(codice().entradas.len() >= 8);
    }

    #[test]
    fn every_sighting_mission_plays_its_scene_where_it_points() {
        let lib = guion::library();
        for (var, place, scene) in [
            ("cat_penascal", "penascal_atalaya", "catalejo.atalaya"),
            ("cat_tablon", "tablon", "catalejo.tablon"),
            ("cat_santuario", "bosque_santuario", "catalejo.santuario"),
        ] {
            let mut v = guion::Vars::default();
            assert!(
                lib.entry(None, Some(place), &v).is_none(),
                "{place} before the sighting"
            );
            v.values.insert(var.into(), 1);
            let at = lib.entry(None, Some(place), &v).expect(place);
            let talk = guion::Conversation::start(lib, at, &mut v);
            assert_eq!(
                format!(
                    "catalejo.{}",
                    talk.scene_id(lib).rsplit('.').next().unwrap_or("")
                ),
                scene
            );
            v.values.insert(var.into(), 2);
            assert!(
                lib.entry(None, Some(place), &v).is_none(),
                "{place} once done"
            );
        }
        // the boat: Maren tells about it in her cell before the day's talk
        let mut v = guion::Vars::default();
        for (k, n) in [
            ("varrak_despierto", 1),
            ("prisionera_queda", 1),
            ("prisionera_n", 5),
            ("cat_barca", 1),
        ] {
            v.values.insert(k.into(), n);
        }
        let at = lib.entry(None, Some("celda"), &v).expect("the cell");
        let talk = guion::Conversation::start(lib, at, &mut v);
        assert!(
            talk.scene_id(lib).ends_with("celda_barca"),
            "{}",
            talk.scene_id(lib)
        );
        // every sighting's mission is in the act's missions, open by its variable
        for var in ["cat_penascal", "cat_tablon", "cat_santuario", "cat_barca"] {
            let m = missions::catalog().iter().find(|m| m.id == var).expect(var);
            assert_eq!(m.available, format!("{var} >= 1"));
        }
    }
}
