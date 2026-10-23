//! The red divan by the fire in the Queen's alcove (user 2026-10-22: «mueve caricias al sillón
//! tántrico rojo… como hicimos con la de citas»): F on it opens a court page of the girls whose
//! Caricias she can have, each one's card with her face and what she still lacks. Lira and Nyxara
//! (and Nemi, since 2026-10-23) after two dates with her (`alcove::CARICIAS_CITAS`), Maren once she
//! would come at night (`guest_ready`). Picking one opens her Caricias. The
//! nights in the alcove still lead to them too.
use super::court::Page;
use super::ui_sfx::{self, UiSfx};
use super::*;
use map_overlay::{CourtCard, CourtChip};

fn chip(text: String, tone: u8) -> CourtChip {
    CourtChip { text, tone }
}

/// The divan's girls: her id (Caricias and portraits), her name, her portrait's mood, a line on her.
const GIRLS: [(&str, &str, &str, &str); 4] = [
    (
        "lira",
        "Lira",
        "sonrojada",
        "La ama de llaves, sin libro y sin cofia.",
    ),
    (
        "nyxara",
        "Nyxara",
        "sonrojada",
        "La doctora, esta vez sin la tablilla.",
    ),
    (
        "maren",
        "Maren",
        "sonrisa",
        "La exploradora del este, sin uniforme.",
    ),
    (
        "nemi",
        "Nemi",
        "sonrojada",
        "La doncella, sin cofia y con la cola a la vista.",
    ),
];

impl App {
    /// F on the divan: its page.
    pub(super) fn open_divan(&mut self) -> bool {
        ui_sfx::play(UiSfx::Open);
        self.court_show(Page::Divan);
        true
    }

    pub(super) fn divan_count(&self) -> usize {
        GIRLS.len()
    }

    /// Whether girl `i` of the divan can be had now, and if not, why.
    fn divan_state(&self, i: usize) -> Result<(), String> {
        let need = alcove::CARICIAS_CITAS;
        match GIRLS.get(i).map(|g| g.0) {
            Some(id @ ("lira" | "nyxara" | "nemi")) => {
                let had = self.story.get(&format!("cita_{id}"));
                if had >= need {
                    Ok(())
                } else {
                    Err(format!(
                        "Tras {need} citas con ella en los sofás del salón ({had}/{need})."
                    ))
                }
            }
            Some("maren") => {
                if self.story.get("prisionera_n") == 0 {
                    return Err("Aún no la conoces.".to_owned());
                }
                self.guest_ready(2).map_err(str::to_owned)
            }
            Some(_) => Err(String::new()),
            None => Err(String::new()),
        }
    }

    /// Enter on girl `pick`: her Caricias, if she is ready.
    pub(super) fn divan_pick(&mut self, pick: usize) {
        let Some(&(id, ..)) = GIRLS.get(pick) else {
            return;
        };
        match self.divan_state(pick) {
            Ok(()) => {
                ui_sfx::play(UiSfx::Select);
                self.close_court();
                self.open_caricias(id);
            }
            Err(why) => {
                ui_sfx::play(UiSfx::Error);
                self.court_notice(&why);
            }
        }
    }

    pub(super) fn divan_view(&self, view: &mut map_overlay::CourtView) {
        view.kind = 1;
        view.title = "El diván rojo".to_owned();
        view.tag = "CARICIAS · CON QUIEN YA TE TIENE CONFIANZA".to_owned();
        view.accent = 0xd83c58;
        view.cards = GIRLS
            .iter()
            .enumerate()
            .map(|(i, (id, name, mood, line))| {
                let state = self.divan_state(i);
                let known = !(*id == "maren" && self.story.get("prisionera_n") == 0);
                CourtCard {
                    title: if known {
                        (*name).to_owned()
                    } else {
                        "· · ·".to_owned()
                    },
                    detail: match &state {
                        Ok(()) => (*line).to_owned(),
                        Err(why) => why.clone(),
                    },
                    chips: vec![match &state {
                        Ok(()) => chip("ABIERTAS".into(), 3),
                        Err(_) => chip("AÚN NO".into(), 2),
                    }],
                    picture: known.then(|| self.court_portrait(id, Some(mood))).flatten(),
                    icon: String::new(),
                    state: u8::from(state.is_err()),
                }
            })
            .collect();
        view.keys = vec![
            ("A D W S".into(), "elegir".into()),
            ("Enter".into(), "acariciar".into()),
            ("Esc".into(), "levantarse".into()),
        ];
    }
}
