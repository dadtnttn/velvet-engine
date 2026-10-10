//! Las misiones de la historia (`data/misiones/acto<n>.json`, ver su README): cuáles están abiertas
//! según las variables de los guiones, la que se sigue (J la cambia) y su paso actual, y los avisos
//! de «Nueva misión» y «Misión completada». La guía hasta el sitio del paso está en
//! `explore/guide.rs`.
use super::ui_sfx::{self, UiSfx};
use super::*;

/// The story variable that keeps the followed mission (its `orden`; saved with the story).
pub(crate) const FOLLOWED: &str = "mision_seguida";
/// How long a notice stays up (seconds).
pub(crate) const NOTICE_SECS: f32 = 4.2;

/// One step: what to do, where (a zone and a spot in it) and until when.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Step {
    #[serde(rename = "texto")]
    pub text: String,
    #[serde(rename = "detalle", default)]
    pub detail: String,
    /// "castillo", "bosque", "villa", "penascal", "termas" or "burdel".
    #[serde(rename = "zona")]
    pub zone: String,
    /// A hotspot of that zone (a person or a place); empty: nowhere in particular.
    #[serde(rename = "objetivo", default)]
    pub target: String,
    /// What closes the step (guion syntax).
    #[serde(rename = "hasta")]
    pub until: String,
}

fn main_kind() -> String {
    "principal".to_owned()
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Mission {
    pub id: String,
    #[serde(rename = "titulo")]
    pub title: String,
    /// "principal" or "secundaria".
    #[serde(rename = "tipo", default = "main_kind")]
    pub kind: String,
    #[serde(rename = "orden")]
    pub order: i32,
    #[serde(rename = "resumen", default)]
    #[allow(dead_code)]
    pub summary: String,
    /// What opens it (empty: open from the start).
    #[serde(rename = "disponible", default)]
    pub available: String,
    /// What closes it.
    #[serde(rename = "hecha")]
    pub done: String,
    #[serde(rename = "pasos")]
    pub steps: Vec<Step>,
    /// The act it belongs to (its file's).
    #[serde(skip)]
    pub act: i32,
}

impl Mission {
    /// Part of the story (not a side errand).
    pub(crate) fn main(&self) -> bool {
        self.kind != "secundaria"
    }
}

#[derive(Deserialize)]
struct File {
    #[serde(rename = "acto")]
    act: i32,
    #[serde(rename = "misiones")]
    missions: Vec<Mission>,
}

/// Every act's missions: the main ones first, each in its act and order.
pub(crate) fn catalog() -> &'static [Mission] {
    static CATALOG: OnceLock<Vec<Mission>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let dir = exploration3d_state::data_dir().join("misiones");
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|d| {
                d.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x == "json"))
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        let mut all = Vec::new();
        for path in files {
            let read = fs::read_to_string(&path)
                .map_err(anyhow::Error::from)
                .and_then(|text| serde_json::from_str::<File>(&text).map_err(anyhow::Error::from));
            match read {
                Ok(file) => all.extend(file.missions.into_iter().map(|mut m| {
                    m.act = file.act;
                    m
                })),
                Err(error) => eprintln!("[misiones] {}: {error:#}", path.display()),
            }
        }
        all.sort_by_key(|m| (!m.main(), m.act, m.order));
        all
    })
}

/// Whether a condition holds, written as the guiones write theirs: `a` (set, above 0), `!a` (not
/// set), `a >= 3`, several joined by commas (all of them); an empty one always holds.
pub(crate) fn holds(text: &str, vars: &guion::Vars) -> bool {
    text.split([',', '&'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .all(|c| one_holds(c, vars))
}

fn one_holds(text: &str, vars: &guion::Vars) -> bool {
    if let Some(name) = text.strip_prefix('!') {
        return vars.get(&guion::slug(name)) <= 0;
    }
    for (sym, op) in [
        (">=", guion::Op::Gte),
        ("<=", guion::Op::Lte),
        ("!=", guion::Op::Ne),
        ("==", guion::Op::Eq),
        (">", guion::Op::Gt),
        ("<", guion::Op::Lt),
        ("=", guion::Op::Eq),
    ] {
        if let Some((a, b)) = text.split_once(sym) {
            let Ok(value) = b.trim().parse() else {
                return false;
            };
            let c = guion::Cond {
                var: guion::slug(a),
                op,
                value,
            };
            return c.holds(vars.get(&c.var));
        }
    }
    vars.get(&guion::slug(text)) > 0
}

/// Where a mission stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Locked,
    /// Open, on this step.
    Open(usize),
    Done,
}

pub(crate) fn status(m: &Mission, vars: &guion::Vars) -> Status {
    if holds(&m.done, vars) {
        return Status::Done;
    }
    if !holds(&m.available, vars) {
        return Status::Locked;
    }
    let step = m
        .steps
        .iter()
        .position(|s| !holds(&s.until, vars))
        .unwrap_or(m.steps.len().saturating_sub(1));
    Status::Open(step)
}

/// The open missions with their step, the main ones first.
pub(crate) fn open(vars: &guion::Vars) -> Vec<(&'static Mission, usize)> {
    catalog()
        .iter()
        .filter(|m| !m.steps.is_empty())
        .filter_map(|m| match status(m, vars) {
            Status::Open(step) => Some((m, step)),
            _ => None,
        })
        .collect()
}

/// The variables a condition names (`a`, `!a`, `a >= 3`, several joined by commas).
fn condition_vars(text: &str) -> Vec<String> {
    text.split([',', '&'])
        .map(|c| c.trim().trim_start_matches('!'))
        .filter(|c| !c.is_empty())
        .map(|c| {
            let name = c.split(['>', '<', '=', '!']).next().unwrap_or(c);
            guion::slug(name)
        })
        .collect()
}

/// Unsticks the story: a once-only guion scene that an open mission's step is waiting on (one at
/// the step's spot, that can close it) but that is marked seen while the step is still open — its
/// conversation was cut short, say by closing the game halfway — may play again. Returns the scenes
/// freed.
pub(crate) fn unstick(vars: &mut guion::Vars) -> Vec<String> {
    let lib = guion::library();
    let mut freed = Vec::new();
    for (mission, step) in open(vars) {
        let Some(step) = mission.steps.get(step) else {
            continue;
        };
        if step.target.is_empty() || holds(&step.until, vars) {
            continue;
        }
        let target = guion::slug(&step.target);
        let wanted = condition_vars(&step.until);
        for (f, g) in lib.guiones.iter().enumerate() {
            for (s, scene) in g.scenes.iter().enumerate() {
                let a = &scene.attrs;
                let here = a.en.as_deref() == Some(target.as_str())
                    || a.quien.as_deref() == Some(target.as_str());
                if !here
                    || !a.una_vez
                    || !vars.seen(&scene.id)
                    || !a.si.iter().all(|c| c.holds(vars.get(&c.var)))
                {
                    continue;
                }
                let sets = lib.vars_set_from((f, s));
                if wanted.iter().any(|v| sets.contains(v)) {
                    vars.seen.remove(&scene.id);
                    freed.push(scene.id.clone());
                }
            }
        }
    }
    freed
}

/// What the world remembers of the missions between steps: which were open and done when last
/// looked at (for the notices), and the notices themselves.
#[derive(Default)]
pub(crate) struct Tracker {
    /// `None` until the first look (a loaded game's missions are not news).
    open: Option<BTreeSet<&'static str>>,
    done: BTreeSet<&'static str>,
    /// Notices waiting (heading, title), and the one on show with when it came.
    queue: VecDeque<(&'static str, String)>,
    pub shown: Option<(&'static str, String, f32)>,
}

impl App {
    /// The followed mission and its step: the one chosen with J while it is open, else the first
    /// open one (the story's next).
    pub(super) fn followed_mission(&self) -> Option<(&'static Mission, usize)> {
        let open = open(&self.story);
        let chosen = self.story.get(FOLLOWED);
        open.iter()
            .find(|(m, _)| m.order == chosen)
            .or_else(|| open.first())
            .copied()
    }

    /// J: follow the next open mission.
    pub(super) fn cycle_mission(&mut self) {
        let open = open(&self.story);
        if open.len() < 2 {
            return;
        }
        let now = self.followed_mission().map(|(m, _)| m.order);
        let next = open
            .iter()
            .position(|(m, _)| Some(m.order) == now)
            .map_or(0, |i| (i + 1) % open.len());
        self.story
            .values
            .insert(FOLLOWED.to_owned(), open[next].0.order);
        self.route = None;
        ui_sfx::play(UiSfx::Select);
        eprintln!("[misiones] siguiendo «{}»", open[next].0.title);
    }

    /// Each step: notices for the missions that opened or closed since the last look (held while
    /// a conversation or a scene is on show).
    pub(super) fn tick_missions(&mut self) {
        let mut open_now = BTreeSet::new();
        let mut done_now = BTreeSet::new();
        for m in catalog() {
            match status(m, &self.story) {
                Status::Open(_) => {
                    open_now.insert(m.id.as_str());
                }
                Status::Done => {
                    done_now.insert(m.id.as_str());
                }
                Status::Locked => {}
            }
        }
        let first_look = self.missions.open.is_none();
        if let Some(before) = self.missions.open.as_ref() {
            for m in catalog() {
                let id = m.id.as_str();
                if done_now.contains(id) && !self.missions.done.contains(id) && before.contains(id)
                {
                    self.missions
                        .queue
                        .push_back(("Misión completada", m.title.clone()));
                    eprintln!("[misiones] completada: {}", m.title);
                }
            }
            for m in catalog() {
                let id = m.id.as_str();
                if open_now.contains(id) && !before.contains(id) {
                    self.missions
                        .queue
                        .push_back(("Nueva misión", m.title.clone()));
                    eprintln!("[misiones] nueva: {}", m.title);
                }
            }
        }
        self.missions.open = Some(open_now);
        self.missions.done = done_now;
        // The game opens with a reminder of where the story stands.
        if first_look {
            if let Some((m, _)) = self.followed_mission() {
                self.missions.queue.push_back(("Misión", m.title.clone()));
            }
        }
        let busy = self.talk.is_some()
            || self.stage.is_some()
            || self.bed_scene_open()
            || self.drink.is_some()
            || self.alcove.is_some();
        if self
            .missions
            .shown
            .as_ref()
            .is_some_and(|(_, _, at)| self.elapsed - at > NOTICE_SECS)
        {
            self.missions.shown = None;
        }
        if self.missions.shown.is_none() && !busy {
            if let Some((heading, title)) = self.missions.queue.pop_front() {
                ui_sfx::play(if heading == "Misión completada" {
                    UiSfx::Equip
                } else {
                    UiSfx::Open
                });
                self.missions.shown = Some((heading, title, self.elapsed));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, i32)]) -> guion::Vars {
        let mut v = guion::Vars::default();
        for (k, n) in pairs {
            v.values.insert((*k).to_owned(), *n);
        }
        v
    }

    #[test]
    fn conditions_read_as_the_guiones_write_them() {
        let v = vars(&[("a", 1), ("entreno", 3)]);
        assert!(holds("", &v));
        assert!(holds("a", &v));
        assert!(!holds("b", &v));
        assert!(holds("!b", &v));
        assert!(holds("entreno >= 3, a", &v));
        assert!(!holds("entreno > 3", &v));
        assert!(holds("b == 0", &v));
    }

    #[test]
    fn a_council_cut_short_plays_again_but_a_finished_one_does_not() {
        if catalog().is_empty() {
            return;
        }
        // Closed halfway through the first Council: seen, not done.
        let mut v = vars(&[]);
        v.seen.insert("acto1.consejo_1".into());
        assert_eq!(unstick(&mut v), vec!["acto1.consejo_1".to_owned()]);
        assert!(!v.seen("acto1.consejo_1"));
        // Done, and on to the second Council at the same table: the first stays seen.
        let mut v = vars(&[
            ("consejo_1_hecho", 1),
            ("lamento_visto", 1),
            ("gorou_cabeza_alta", 1),
            ("varrak_visto", 1),
        ]);
        v.seen.insert("acto1.consejo_1".into());
        assert!(unstick(&mut v).is_empty());
        assert!(v.seen("acto1.consejo_1"));
    }

    #[test]
    fn the_first_act_runs_from_the_first_council_to_the_chess_game() {
        let all = catalog();
        if all.is_empty() {
            return;
        }
        let mut v = guion::Vars::default();
        let first = open(&v);
        assert_eq!(first.first().map(|(m, _)| m.id.as_str()), Some("consejo_1"));
        // Through the story's variables, one at a time: each opens the next main mission.
        for (var, next) in [
            ("consejo_1_hecho", "lamento"),
            ("lamento_visto", "corral"),
            ("corral_pedido", "corral"),
            ("gorou_cabeza_alta", "campanas"),
            ("varrak_visto", "consejo_2"),
            ("consejo_2_hecho", "colmillo"),
            ("penascal_visto", "colmillo"),
            ("varrak_despierto", "ajedrez"),
        ] {
            v.values.insert(var.to_owned(), 1);
            if var == "varrak_despierto" {
                v.values.insert("acto1_fin".to_owned(), 1);
            }
            let main = open(&v)
                .into_iter()
                .find(|(m, _)| m.main())
                .map(|(m, step)| (m.id.as_str(), step));
            assert_eq!(main.map(|(id, _)| id), Some(next), "after {var}");
        }
        // The corral's second step is the villa's corral, the Peñascal's second the chief.
        let mut v = vars(&[
            ("consejo_1_hecho", 1),
            ("lamento_visto", 1),
            ("corral_pedido", 1),
        ]);
        let corral = open(&v)
            .into_iter()
            .find(|(m, _)| m.id == "corral")
            .unwrap();
        assert_eq!(corral.0.steps[corral.1].target, "corral");
        v.values.insert("ajedrez_hecho".to_owned(), 1);
        // Every step names a zone the guide knows.
        for m in all {
            for s in &m.steps {
                assert!(
                    ["castillo", "bosque", "villa", "penascal", "termas", "burdel"]
                        .contains(&s.zone.as_str()),
                    "{}: {}",
                    m.id,
                    s.zone
                );
            }
        }
    }
}
