//! The castle's dating sim. At the vanity («Citas», a page of the court) the Queen picks whom to
//! spend the day with, Lira or Nyxara: four day dates and a first night each (texts in
//! `data/castillo/citas.json`, scenes in `data/guiones/citas.md`). A date opens once the one before
//! is had and her affection (`afecto_<id>`) reaches its mark; one date a day (`cita_hoy`), the
//! nights only at night and with the alcove free (`noche_hoy`). Each date plays on the stage and
//! pays its memories: the shot full screen (`@recuerdo`), kept in the gallery's «Citas» album.
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
    pub citas: Vec<Cita>,
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
        } else if affection < next.afecto {
            format!(
                "Afecto {affection} de {}: pasa más tiempo con ella.",
                next.afecto
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

    /// F at the vanity: the dates' page.
    pub(super) fn open_dates(&mut self) -> bool {
        if citas().chicas.is_empty() {
            return false;
        }
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Dates);
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

    /// Enter on girl `pick`: her next date plays, if it can.
    pub(super) fn dates_pick(&mut self, pick: usize) {
        let Some(g) = citas().chicas.get(pick) else {
            return;
        };
        match self.date_state(pick) {
            DateState::Ready(n) => {
                let scene = g.citas[n].escena.clone();
                ui_sfx::play(UiSfx::Select);
                self.close_court();
                if !self.start_talk_at(&scene) {
                    eprintln!("[citas] no hay escena {scene}");
                }
            }
            DateState::Waiting(_, why) => {
                ui_sfx::play(UiSfx::Error);
                self.court_notice(why);
            }
            DateState::Done => {
                ui_sfx::play(UiSfx::Error);
                self.court_notice(format!(
                    "Ruta de {} completa. De noche, invítala desde la cama.",
                    g.nombre
                ));
            }
        }
    }

    pub(super) fn dates_view(&self, view: &mut map_overlay::CourtView) {
        use map_overlay::{CourtCard, DateView, PortraitRef};
        let night = self.date_night();
        view.kind = 11;
        view.title = "Citas".to_owned();
        view.tag = if night {
            "EL TOCADOR · ¿CON QUIÉN PASAS LA NOCHE?".to_owned()
        } else {
            "EL TOCADOR · ¿CON QUIÉN PASAS EL DÍA?".to_owned()
        };
        view.accent = 0xff6fa8;
        view.stats = vec![
            (
                "Hoy".to_owned(),
                if self.story.get("cita_hoy") > 0 {
                    "cita hecha".to_owned()
                } else {
                    "libre".to_owned()
                },
            ),
            (
                "Hora".to_owned(),
                if night { "noche" } else { "día" }.to_owned(),
            ),
        ];
        for (i, g) in citas().chicas.iter().enumerate() {
            let had = self.dates_had(g);
            let state = self.date_state(i);
            let next = g.citas.get(had);
            let affection = self.story.get(&format!("afecto_{}", g.id));
            let memories = g
                .citas
                .iter()
                .enumerate()
                .flat_map(|(n, c)| {
                    c.recuerdos.iter().map(move |r| {
                        (
                            PortraitRef {
                                image: r.imagen.clone(),
                                crop: r.recorte,
                            },
                            n < had,
                        )
                    })
                })
                .collect();
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
                    || "De noche, invítala desde la cama.".to_owned(),
                    |c| c.lugar.clone(),
                ),
                why: match state {
                    DateState::Waiting(_, why) => why,
                    _ => String::new(),
                },
                night: next.is_some_and(|c| c.noche),
                memories,
            });
        }
        view.keys = vec![
            ("A D".into(), "elegir".into()),
            ("Enter".into(), "pedirle una cita".into()),
            ("Esc".into(), "cerrar".into()),
        ];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dates_file_reads_and_every_date_has_its_scene_and_pictures() {
        let c = citas();
        assert_eq!(c.chicas.len(), 2);
        let lib = guion::library();
        let root = crate::exploration3d_state::data_dir().join("assets");
        for g in &c.chicas {
            assert_eq!(g.citas.len(), 5, "{}", g.id);
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
        assert_eq!(memories().len(), 12);
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
