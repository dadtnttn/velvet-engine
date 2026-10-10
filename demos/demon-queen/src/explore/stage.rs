//! El escenario: las escenas de guion que se juegan a pantalla completa (la bienvenida de las termas
//! y la del burdel, las charlas con las chicas). El guion pone el decorado con sus órdenes (`@fondo`,
//! `@izq`, `@der`, `@pose`, `@sello`…) y este módulo lleva la cuenta de lo que hay en escena y de
//! cuándo cambió cada cosa, para que el overlay lo anime.
use super::*;

/// Alguien en escena.
#[derive(Clone, Debug)]
pub(super) struct Actor {
    /// Su id de personaje (el del guion: `gorou` para la Reina).
    pub who: String,
    pub image: String,
    pub prev: Option<String>,
    pub swapped_at: f32,
    pub right: bool,
    pub entered_at: f32,
    pub left_at: Option<f32>,
    pub shake_at: f32,
    pub bump_at: f32,
    pub spin: Option<(Vec<String>, f32)>,
    /// Su tamaño frente al resto (`@der Mugo termas_vn/mugo_normal 0.8`).
    pub scale: f32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Stage {
    pub started: f32,
    pub bg: Option<String>,
    pub bg_prev: Option<String>,
    pub wipe_at: f32,
    pub actors: Vec<Actor>,
    pub title: Option<(String, String, f32)>,
    pub stamp: Option<(String, bool, f32)>,
    pub flash_at: f32,
    pub item: Option<(String, String, String, f32)>,
    pub panels: [Option<(String, f32)>; 3],
    pub reveal: Option<(String, f32)>,
    pub unlocks: Vec<String>,
    /// El rótulo del final (`@cierre FOTO GUARDADA`; si no, «ESCENA COMPLETA»).
    pub outro_title: Option<String>,
    /// La escena terminó y se enseña lo desbloqueado hasta que se pulse F.
    pub outro_at: Option<f32>,
}

const LONG_AGO: f32 = -1.0e6;
/// Lo que tarda una vuelta de `@giro` por fotograma.
const SPIN_FRAME: f32 = 0.52;

impl Stage {
    fn new(now: f32) -> Self {
        Self {
            started: now,
            wipe_at: LONG_AGO,
            flash_at: LONG_AGO,
            ..Default::default()
        }
    }

    fn actor(&mut self, who: &str) -> Option<&mut Actor> {
        self.actors
            .iter_mut()
            .find(|a| a.who == who && a.left_at.is_none())
    }

    /// Una orden del guion, ahora.
    fn apply(&mut self, cmd: &str, arg: &str, now: f32, who_of: &dyn Fn(&str) -> String) {
        let parts = || arg.split('|').map(str::trim).collect::<Vec<_>>();
        match cmd {
            "fondo" => {
                if self.bg.as_deref() != Some(arg) {
                    self.bg_prev = self.bg.take();
                    self.bg = Some(arg.to_owned());
                    self.wipe_at = if self.bg_prev.is_some() {
                        now
                    } else {
                        LONG_AGO
                    };
                }
                self.panels = Default::default();
                self.reveal = None;
            }
            "barrido" => {
                self.bg_prev = self.bg.clone();
                self.wipe_at = now;
                self.panels = Default::default();
            }
            "izq" | "der" => {
                let Some((name, image)) = arg.split_once(char::is_whitespace) else {
                    return;
                };
                // Tras la imagen, un número opcional: su tamaño (1 = el de todas).
                let mut words: Vec<&str> = image.split_whitespace().collect();
                let scale = match words.last().map(|w| w.parse::<f32>()) {
                    Some(Ok(k)) if words.len() > 1 => {
                        words.pop();
                        k.clamp(0.3, 1.5)
                    }
                    _ => 1.0,
                };
                let image = words.join(" ");
                let (who, image, right) = (who_of(name), image, cmd == "der");
                // Quien estaba en ese lado y no es ella se va.
                for a in self.actors.iter_mut() {
                    if a.right == right && a.who != who && a.left_at.is_none() {
                        a.left_at = Some(now);
                    }
                }
                if let Some(a) = self.actor(&who) {
                    if a.image != image {
                        a.prev = Some(std::mem::replace(&mut a.image, image));
                        a.swapped_at = now;
                    }
                    a.right = right;
                    a.spin = None;
                    a.scale = scale;
                } else {
                    self.actors.retain(|a| a.who != who);
                    self.actors.push(Actor {
                        who,
                        image,
                        prev: None,
                        swapped_at: LONG_AGO,
                        right,
                        entered_at: now,
                        left_at: None,
                        shake_at: LONG_AGO,
                        bump_at: LONG_AGO,
                        spin: None,
                        scale,
                    });
                }
            }
            "pose" => {
                let Some((name, pose)) = arg.split_once(char::is_whitespace) else {
                    return;
                };
                let who = who_of(name);
                if let Some(a) = self.actor(&who) {
                    let image = posed(&a.image, pose.trim());
                    if a.image != image || a.spin.is_some() {
                        a.prev = Some(std::mem::replace(&mut a.image, image));
                        a.swapped_at = now;
                        a.spin = None;
                    }
                }
            }
            "quitar" => {
                let who = who_of(arg);
                if let Some(a) = self.actor(&who) {
                    a.left_at = Some(now);
                }
            }
            "sacude" => {
                if let Some(a) = self.actor(&who_of(arg)) {
                    a.shake_at = now;
                }
            }
            "bote" => {
                if let Some(a) = self.actor(&who_of(arg)) {
                    a.bump_at = now;
                }
            }
            "titulo" => {
                let p = parts();
                self.title = Some((
                    p.first().copied().unwrap_or("").to_owned(),
                    p.get(1).copied().unwrap_or("").to_owned(),
                    now,
                ));
            }
            "sello" => {
                let p = parts();
                let gold = p.get(1).is_some_and(|x| guion::slug(x) == "oro");
                self.stamp = Some((p.first().copied().unwrap_or("").to_owned(), gold, now));
            }
            "destello" => self.flash_at = now,
            "objeto" => {
                let p = parts();
                self.item = Some((
                    p.first().copied().unwrap_or("").to_owned(),
                    p.get(1).copied().unwrap_or("").to_owned(),
                    p.get(2).copied().unwrap_or("").to_owned(),
                    now,
                ));
            }
            "panel" => {
                if let Some((n, image)) = arg.split_once(char::is_whitespace) {
                    if let Ok(n) = n.trim().parse::<usize>() {
                        if (1..=3).contains(&n) {
                            self.panels[n - 1] = Some((image.trim().to_owned(), now));
                        }
                    }
                }
            }
            "revela" => {
                self.panels = Default::default();
                self.reveal = Some((arg.trim().to_owned(), now));
            }
            "giro" => {
                let frames: Vec<String> = arg.split_whitespace().map(str::to_owned).collect();
                if frames.is_empty() {
                    return;
                }
                let who = who_of("reina");
                let target = if self.actor(&who).is_some() {
                    Some(who)
                } else {
                    self.actors
                        .iter()
                        .find(|a| a.left_at.is_none())
                        .map(|a| a.who.clone())
                };
                if let Some(who) = target {
                    if let Some(a) = self.actor(&who) {
                        a.image = frames.last().cloned().unwrap_or_default();
                        a.prev = None;
                        a.spin = Some((frames, now));
                    }
                }
            }
            "cierre" => self.outro_title = Some(arg.trim().to_owned()).filter(|t| !t.is_empty()),
            "desbloquea" => {
                self.unlocks = parts()
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .map(str::to_owned)
                    .collect()
            }
            _ => {}
        }
    }

    /// Lo que se ha ido del todo deja de estar en escena.
    fn tidy(&mut self, now: f32) {
        self.actors
            .retain(|a| a.left_at.is_none_or(|t| now - t < 0.6));
        if self.reveal.as_ref().is_some_and(|(_, t)| now - t > 2.6) {
            self.reveal = None;
        }
    }
}

/// `image` con otra pose: lo que va tras el primer `_` de su nombre cambia por `pose`
/// (`burdel_vn/reina_capucha_normal` + `capucha_duda` → `burdel_vn/reina_capucha_duda`).
fn posed(image: &str, pose: &str) -> String {
    let (dir, file) = image.rsplit_once('/').unwrap_or(("", image));
    let who = file.split('_').next().unwrap_or(file);
    if dir.is_empty() {
        format!("{who}_{pose}")
    } else {
        format!("{dir}/{who}_{pose}")
    }
}

impl App {
    /// Recoge las órdenes de escenario que el guion acaba de pasar y las pone en escena (el
    /// escenario se levanta con la primera).
    pub(super) fn stage_take(&mut self) {
        let Some(talk) = self.talk.as_mut() else {
            return;
        };
        let staged = talk.take_staging();
        // Solo las órdenes del escenario lo levantan (las del VN, como `@sala`, no).
        let opens = |cmd: &str| !matches!(cmd, "sala" | "cg" | "sprite" | "musica");
        if staged.is_empty() || (self.stage.is_none() && !staged.iter().any(|(c, _)| opens(c))) {
            return;
        }
        let now = self.elapsed;
        let lib = guion::library();
        let who_of = |name: &str| lib.cast.speaker_id(name);
        let stage = self.stage.get_or_insert_with(|| Stage::new(now));
        for (cmd, arg) in &staged {
            stage.apply(cmd, arg, now, &who_of);
        }
        stage.tidy(now);
    }

    /// Se pasa a la línea siguiente: el objeto enseñado se guarda.
    pub(super) fn stage_line_passed(&mut self) {
        if let Some(stage) = self.stage.as_mut() {
            stage.item = None;
            stage.tidy(self.elapsed);
        }
    }

    pub(super) fn stage_up(&self) -> bool {
        self.stage.is_some()
    }

    /// Quién está en escena (para no repetir su cara en la caja).
    pub(super) fn on_stage(&self, who: &str) -> bool {
        self.stage
            .as_ref()
            .is_some_and(|s| s.actors.iter().any(|a| a.who == who && a.left_at.is_none()))
    }

    /// El escenario para el overlay.
    pub(super) fn stage_view(&self) -> Option<map_overlay::StageView> {
        let stage = self.stage.as_ref()?;
        let now = self.elapsed;
        let lib = guion::library();
        // Quién habla: su figura se ilumina; con narración, todas.
        let speaking: Option<String> = match self.talk.as_ref().map(|t| t.shown(lib, &self.story)) {
            Some(guion::Shown::Line { who, kind, .. }) => match kind {
                guion::LineKind::Dialogue => Some(who),
                guion::LineKind::Thought => {
                    Some(who.trim_end_matches(guion::THOUGHT_SUFFIX).to_owned())
                }
                _ => None,
            },
            Some(guion::Shown::Choice { .. }) => Some("gorou".to_owned()),
            _ => None,
        };
        let accent = self.dialogue_view().map_or(0, |d| {
            ((d.accent[0] as u32) << 16) | ((d.accent[1] as u32) << 8) | d.accent[2] as u32
        });
        let age = |t: f32| (now - t).max(0.0);
        Some(map_overlay::StageView {
            bg: stage.bg.clone(),
            bg_prev: stage.bg_prev.clone(),
            wipe: (age(stage.wipe_at) / 0.9).min(1.0),
            actors: stage
                .actors
                .iter()
                .map(|a| map_overlay::StageActor {
                    image: a.image.clone(),
                    prev: a.prev.clone(),
                    swap: (age(a.swapped_at) / 0.2).min(1.0),
                    right: a.right,
                    enter: match a.left_at {
                        Some(t) => (age(t) / 0.3).min(1.0),
                        None => (age(a.entered_at) / 0.35).min(1.0),
                    },
                    leaving: a.left_at.is_some(),
                    shake: age(a.shake_at),
                    bump: age(a.bump_at),
                    lit: speaking.as_deref().is_none_or(|w| w == a.who),
                    spin: a
                        .spin
                        .as_ref()
                        .filter(|(frames, t)| age(*t) < frames.len() as f32 * SPIN_FRAME)
                        .map(|(frames, t)| (frames.clone(), age(*t))),
                    scale: a.scale,
                })
                .collect(),
            title: stage
                .title
                .as_ref()
                .filter(|(_, _, t)| age(*t) < 2.6)
                .map(|(a, b, t)| (a.clone(), b.clone(), age(*t))),
            stamp: stage
                .stamp
                .as_ref()
                .filter(|(_, _, t)| age(*t) < 1.5)
                .map(|(w, g, t)| (w.clone(), *g, age(*t))),
            flash: age(stage.flash_at),
            item: stage
                .item
                .as_ref()
                .map(|(icon, name, text, t)| map_overlay::StageItem {
                    icon: icon.clone(),
                    name: name.clone(),
                    text: text.clone(),
                    age: age(*t),
                }),
            panels: stage
                .panels
                .iter()
                .map(|p| p.as_ref().map(|(i, t)| (i.clone(), age(*t))))
                .collect(),
            reveal: stage.reveal.as_ref().map(|(i, t)| (i.clone(), age(*t))),
            outro: stage.outro_at.map(|t| (stage.unlocks.clone(), age(t))),
            outro_title: stage
                .outro_title
                .clone()
                .unwrap_or_else(|| "ESCENA COMPLETA".to_owned()),
            age: age(stage.started),
            accent,
            speaker: speaking
                .as_deref()
                .and_then(|w| {
                    stage
                        .actors
                        .iter()
                        .find(|a| a.who == w && a.left_at.is_none())
                })
                .map(|a| a.right),
            line_age: (now - self.talk_line_at).max(0.0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(name: &str) -> String {
        guion::slug(name)
    }

    #[test]
    fn a_pose_swaps_what_follows_the_name() {
        assert_eq!(
            posed("burdel_vn/reina_capucha_normal", "capucha_duda"),
            "burdel_vn/reina_capucha_duda"
        );
        assert_eq!(
            posed("termas_vn/esq_aburrida", "senala"),
            "termas_vn/esq_senala"
        );
    }

    #[test]
    fn the_cast_comes_on_changes_and_leaves() {
        let mut s = Stage::new(0.0);
        s.apply("fondo", "burdel_vn/fondos/calle", 0.0, &who);
        s.apply("izq", "Reina burdel_vn/reina_capucha_normal", 0.0, &who);
        s.apply("der", "Rosalía burdel_vn/rosalia_normal", 1.0, &who);
        assert_eq!(s.actors.len(), 2);
        assert_eq!(s.actors[1].scale, 1.0);
        s.apply("der", "Rosalía burdel_vn/rosalia_normal 0.8", 1.0, &who);
        assert_eq!(
            (s.actors[1].image.as_str(), s.actors[1].scale),
            ("burdel_vn/rosalia_normal", 0.8)
        );
        s.apply("der", "Rosalía burdel_vn/rosalia_normal", 1.0, &who);
        s.apply("pose", "Rosalía picara", 2.0, &who);
        let r = s.actors.iter().find(|a| a.who == "rosalia").unwrap();
        assert_eq!(r.image, "burdel_vn/rosalia_picara");
        assert_eq!(r.prev.as_deref(), Some("burdel_vn/rosalia_normal"));
        // Otra en el mismo lado: Rosalía se va.
        s.apply("der", "Nina burdel_vn/nina_normal", 3.0, &who);
        assert!(s
            .actors
            .iter()
            .find(|a| a.who == "rosalia")
            .unwrap()
            .left_at
            .is_some());
        s.tidy(4.0);
        assert_eq!(
            s.actors.iter().map(|a| a.who.as_str()).collect::<Vec<_>>(),
            ["reina", "nina"]
        );
        // Un fondo nuevo barre sobre el viejo.
        s.apply("fondo", "burdel_vn/fondos/barra", 5.0, &who);
        assert_eq!(s.bg_prev.as_deref(), Some("burdel_vn/fondos/calle"));
        assert_eq!(s.wipe_at, 5.0);
    }

    #[test]
    fn stamps_items_and_unlocks_read_their_parts() {
        let mut s = Stage::new(0.0);
        s.apply("sello", "BIENVENIDA | oro", 1.0, &who);
        assert_eq!(s.stamp, Some(("BIENVENIDA".to_owned(), true, 1.0)));
        s.apply(
            "objeto",
            "burdel_vn/item_ficha | Ficha | Vale por una copa.",
            1.0,
            &who,
        );
        assert_eq!(s.item.as_ref().map(|i| i.1.as_str()), Some("Ficha"));
        s.apply("desbloquea", "Uno | Dos", 1.0, &who);
        assert_eq!(s.unlocks, ["Uno", "Dos"]);
    }
}
