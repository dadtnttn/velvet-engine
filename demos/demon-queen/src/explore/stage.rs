//! El escenario: las escenas de guion que se juegan a pantalla completa (la bienvenida de las termas
//! y la del burdel, las charlas con las chicas). El guion pone el decorado con sus órdenes (`@fondo`,
//! `@izq`, `@der`, `@pose`, `@sello`…) y este módulo lleva la cuenta de lo que hay en escena y de
//! cuándo cambió cada cosa, para que el overlay lo anime. Los pasos se encadenan como en las escenas
//! web: un título está solo un momento, lo que llega con un barrido cambia bajo sus franjas, un
//! revelado y un giro se ven enteros antes de lo que sigue, y un objeto se enseña solo hasta F.
use super::ui_sfx::{self, UiSfx};
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
    /// Girando sobre sí misma: los fotogramas, cuándo empezó y la imagen de antes del giro.
    pub spin: Option<(Vec<String>, f32, String)>,
    /// Su tamaño frente al resto (`@der Mugo termas_vn/mugo_normal 0.8`).
    pub scale: f32,
    /// Con el encuadre de las escenas web (`@der Mugo termas_vn/mugo_normal 0.889 0.719 0.696`): la
    /// parte del lienzo que se ve desde arriba, hasta media pierna, y dónde está su centro (fracción
    /// del ancho); el tamaño es entonces el alto del lienzo en pantallas.
    pub window: Option<f32>,
    pub x: Option<f32>,
    /// Iluminada (habla, o todas con la narración) y desde cuándo (la luz se funde).
    pub lit: bool,
    pub lit_at: f32,
}

/// Un panel del tríptico: su imagen, cómo va dentro (`[alto, x %, y]`, como el `background-size` y
/// el `background-position` de las escenas web), cuándo llegó y cuándo se fue (sale volando hacia
/// arriba).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Panel {
    pub image: String,
    pub crop: Option<[f32; 3]>,
    pub at: f32,
    pub out_at: Option<f32>,
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
    pub panels: [Option<Panel>; 3],
    pub reveal: Option<(String, f32)>,
    pub unlocks: Vec<String>,
    /// El rótulo del final (`@cierre FOTO GUARDADA`; si no, «ESCENA COMPLETA»).
    pub outro_title: Option<String>,
    /// La escena terminó y se enseña lo desbloqueado hasta que se pulse F.
    pub outro_at: Option<f32>,
    /// Los colores de la escena web de su sitio: 0 el burdel (rosa), 1 las termas (turquesa, con
    /// vapor), 2 el acto 1 (lila).
    pub theme: u8,
    /// Lo que queda del paso en escena: órdenes que esperan su momento (`Some`) o a que F guarde el
    /// objeto que se enseña (`None`).
    pub pending: Vec<(Option<f32>, String, String)>,
    /// Hasta entonces la caja espera y F no hace nada (un título, el barrido tapando, un revelado,
    /// un giro).
    pub hold_until: f32,
    /// Un objeto se enseña solo: F lo guarda.
    pub item_wait: bool,
    /// La frase de Chiquita espera a que acabe el paso para decirla ella.
    pub cue_wait: bool,
    /// La caja: si está y desde cuándo (entra una vez, no con cada frase).
    pub box_on: bool,
    pub box_at: f32,
    /// El lado de quien habla (true: la derecha), el de antes y desde cuándo (la franja de color
    /// detrás).
    pub focus: Option<bool>,
    pub focus_prev: Option<bool>,
    pub focus_at: f32,
    /// Every picture this scene and the ones it leads to show, to load before they come on (the
    /// web scenes preload theirs: a picture loaded as it comes on stutters its entrance).
    pub preload: Vec<String>,
}

const LONG_AGO: f32 = -1.0e6;
/// Lo que tarda una vuelta de `@giro` por fotograma.
const SPIN_FRAME: f32 = 0.52;
/// El título está solo en escena este rato; el barrido tapa el escenario a esta altura (lo que
/// cambia con él cambia debajo); el revelado dura esto (los paneles salen, tres cortes y fuera).
const TITLE_HOLD: f32 = 2.1;
const WIPE_COVER: f32 = 0.42;
const REVEAL_SECS: f32 = 3.01;
/// Los destellos del revelado: al empezar sus cortes y con cada uno (segundos desde que empieza).
const REVEAL_FLASHES: [f32; 4] = [0.56, 0.68, 1.01, 1.34];
/// Lo que tarda un panel en salir volando.
const PANEL_OUT: f32 = 0.5;
/// Lo que tarda en fundirse la luz de quien deja de hablar (o empieza).
const LIGHT_FADE: f32 = 0.3;
/// Lo oscura que queda quien no habla.
const DIM: f32 = 0.55;

/// Los colores de la escena web de cada sitio.
fn theme_of(scene: &str) -> u8 {
    if scene.starts_with("termas.") {
        1
    } else if scene.starts_with("acto1.") {
        2
    } else {
        0
    }
}

impl Stage {
    fn new(now: f32, theme: u8) -> Self {
        Self {
            started: now,
            wipe_at: LONG_AGO,
            flash_at: LONG_AGO,
            focus_at: LONG_AGO,
            theme,
            ..Default::default()
        }
    }

    fn actor(&mut self, who: &str) -> Option<&mut Actor> {
        self.actors
            .iter_mut()
            .find(|a| a.who == who && a.left_at.is_none())
    }

    /// Lo que había del tríptico y del revelado se va.
    fn clear_panels(&mut self) {
        self.panels = Default::default();
        self.reveal = None;
    }

    /// Una orden del guion, ahora.
    fn apply(&mut self, cmd: &str, arg: &str, now: f32, who_of: &dyn Fn(&str) -> String) {
        let parts = || arg.split('|').map(str::trim).collect::<Vec<_>>();
        match cmd {
            "fondo" => {
                if self.bg.as_deref() != Some(arg) {
                    self.bg_prev = self.bg.take();
                    self.bg = Some(arg.to_owned());
                    if self.bg_prev.is_some() {
                        // Barre: lo que había se va cuando las franjas lo tapan.
                        self.wipe_at = now;
                        self.pending
                            .push((Some(now + WIPE_COVER), "_despeja".to_owned(), String::new()));
                        return;
                    }
                    self.wipe_at = LONG_AGO;
                }
                self.clear_panels();
            }
            "barrido" => {
                self.bg_prev = self.bg.clone();
                self.wipe_at = now;
                self.pending
                    .push((Some(now + WIPE_COVER), "_despeja".to_owned(), String::new()));
            }
            "_despeja" => self.clear_panels(),
            "izq" | "der" => {
                let Some((name, image)) = arg.split_once(char::is_whitespace) else {
                    return;
                };
                // Tras la imagen, hasta tres números: su tamaño (1 = el de todas), la parte del recorte
                // que se ve desde arriba y dónde está su centro (fracciones de la pantalla).
                let mut words: Vec<&str> = image.split_whitespace().collect();
                let mut numbers = Vec::new();
                while words.len() > 1 && numbers.len() < 3 {
                    match words.last().map(|w| w.parse::<f32>()) {
                        Some(Ok(k)) => {
                            numbers.insert(0, k);
                            words.pop();
                        }
                        _ => break,
                    }
                }
                let scale = numbers.first().map_or(1.0, |k| k.clamp(0.3, 1.5));
                let window = numbers.get(1).map(|k| k.clamp(0.2, 1.0));
                let x = numbers.get(2).map(|k| k.clamp(0.05, 0.95));
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
                    a.window = window;
                    a.x = x;
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
                        window,
                        x,
                        lit: true,
                        lit_at: LONG_AGO,
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
                    let spinning = a
                        .spin
                        .as_ref()
                        .is_some_and(|(f, t, _)| now - t < f.len() as f32 * SPIN_FRAME);
                    if spinning {
                        // A media vuelta: acaba en esta.
                        a.image = image;
                    } else if a.image != image || a.spin.is_some() {
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
                // `@panel 2 termas_vn/vest_2 [alto x y]`: el encuadre de la web, si lo trae.
                let mut words = arg.split_whitespace();
                let (Some(n), Some(image)) =
                    (words.next().and_then(|n| n.parse::<usize>().ok()), words.next())
                else {
                    return;
                };
                let numbers: Vec<f32> = words.filter_map(|w| w.parse().ok()).collect();
                let crop = (numbers.len() == 3).then(|| [numbers[0], numbers[1], numbers[2]]);
                if (1..=3).contains(&n) {
                    self.panels[n - 1] = Some(Panel {
                        image: image.to_owned(),
                        crop,
                        at: now,
                        out_at: None,
                    });
                }
            }
            "revela" => {
                // Los paneles salen volando y llegan los tres cortes.
                for p in self.panels.iter_mut().flatten() {
                    p.out_at.get_or_insert(now);
                }
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
                        let before = std::mem::replace(
                            &mut a.image,
                            frames.last().cloned().unwrap_or_default(),
                        );
                        a.prev = None;
                        a.spin = Some((frames, now, before));
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
        for p in self.panels.iter_mut() {
            if p
                .as_ref()
                .is_some_and(|p| p.out_at.is_some_and(|t| now - t > PANEL_OUT))
            {
                *p = None;
            }
        }
        if self
            .reveal
            .as_ref()
            .is_some_and(|(_, t)| now - t > REVEAL_SECS + 0.1)
        {
            self.reveal = None;
        }
    }

    /// Un paso se está jugando: la caja espera.
    fn waiting(&self, now: f32) -> bool {
        now < self.hold_until || self.item_wait
    }
}

/// Las imágenes que enseñan las escenas de escenario de un guion desde `scene` (su etiqueta) hasta la
/// siguiente que se abre sola (`en=`, `quien=`): fondos, recortes (cada pose resuelta como la
/// pondría la escena), paneles, revelados, giros y objetos.
fn scene_pictures(text: &str, scene: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cast: std::collections::HashMap<String, String> = Default::default();
    let mut on = false;
    let push = |out: &mut Vec<String>, s: &str| {
        let s = s.trim();
        if !s.is_empty() && !out.iter().any(|o| o == s) {
            out.push(s.to_owned());
        }
    };
    for line in text.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix("## ") {
            let label = header.split(['{', ' ']).next().unwrap_or("").trim();
            if label == scene {
                on = true;
            } else if on && (header.contains("en=") || header.contains("quien=")) {
                break;
            }
            continue;
        }
        if !on {
            continue;
        }
        let Some(rest) = line.strip_prefix('@') else {
            continue;
        };
        let (cmd, arg) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let arg = arg.trim();
        match cmd {
            "fondo" | "revela" => push(&mut out, arg),
            "izq" | "der" => {
                if let Some((name, image)) = arg.split_once(char::is_whitespace) {
                    let mut words: Vec<&str> = image.split_whitespace().collect();
                    while words.len() > 1 && words.last().is_some_and(|w| w.parse::<f32>().is_ok()) {
                        words.pop();
                    }
                    let image = words.join(" ");
                    cast.insert(guion::slug(name), image.clone());
                    push(&mut out, &image);
                }
            }
            "pose" => {
                if let Some((name, pose)) = arg.split_once(char::is_whitespace) {
                    if let Some(image) = cast.get(&guion::slug(name)).cloned() {
                        let image = posed(&image, pose.trim());
                        cast.insert(guion::slug(name), image.clone());
                        push(&mut out, &image);
                    }
                }
            }
            "panel" => {
                if let Some(image) = arg.split_whitespace().nth(1) {
                    push(&mut out, image);
                }
            }
            "giro" => arg.split_whitespace().for_each(|f| push(&mut out, f)),
            "objeto" => push(&mut out, arg.split('|').next().unwrap_or("")),
            _ => {}
        }
    }
    out
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
    /// escenario se levanta con la primera), encadenadas como en las escenas web: un título está
    /// solo un rato, lo que llega con un barrido cambia bajo sus franjas, un revelado se ve entero
    /// antes que el resto de su paso, lo que sigue a un giro espera a que acabe, y un objeto se
    /// enseña solo hasta que F lo guarda.
    pub(super) fn stage_take(&mut self) {
        let Some(talk) = self.talk.as_mut() else {
            return;
        };
        let mut staged = talk.take_staging();
        // `@llevar <sitio>`: la Reina aparece allí (detrás del diálogo), vuelta hacia ello.
        let mut brings = Vec::new();
        staged.retain(|(cmd, arg)| {
            if cmd == "llevar" {
                brings.push(arg.trim().to_owned());
                false
            } else {
                true
            }
        });
        for place in brings {
            self.bring_queen_to(&place);
        }
        // Solo las órdenes del escenario lo levantan (las del VN, como `@sala`, no).
        let opens = |cmd: &str| !matches!(cmd, "sala" | "cg" | "sprite" | "musica");
        if staged.is_empty() || (self.stage.is_none() && !staged.iter().any(|(c, _)| opens(c))) {
            return;
        }
        let now = self.elapsed;
        let lib = guion::library();
        let who_of = |name: &str| lib.cast.speaker_id(name);
        let scene = self.talk.as_ref().map_or("", |t| t.scene_id(lib)).to_owned();
        let theme = theme_of(&scene);
        let stage = self.stage.get_or_insert_with(|| {
            let mut stage = Stage::new(now, theme);
            // What it will show, to load ahead.
            if let Some((file, label)) = scene.split_once('.') {
                let path = crate::exploration3d_state::data_dir()
                    .join("guiones")
                    .join(format!("{file}.md"));
                if let Ok(text) = std::fs::read_to_string(path) {
                    stage.preload = scene_pictures(&text, label);
                }
            }
            stage
        });
        let reveal = staged.iter().any(|(c, _)| c == "revela");
        let mut at = Some(now);
        for (cmd, arg) in &staged {
            if reveal && cmd != "revela" {
                stage
                    .pending
                    .push((Some(now + REVEAL_SECS), cmd.clone(), arg.clone()));
                continue;
            }
            if at != Some(now) {
                stage.pending.push((at, cmd.clone(), arg.clone()));
                continue;
            }
            stage.apply(cmd, arg, now, &who_of);
            // Lo que este hace esperar al resto del paso.
            let hold = |stage: &mut Stage, until: f32| stage.hold_until = stage.hold_until.max(until);
            match cmd.as_str() {
                "titulo" => hold(stage, now + TITLE_HOLD),
                "barrido" => {
                    at = Some(now + WIPE_COVER);
                    hold(stage, now + WIPE_COVER);
                }
                "fondo" if stage.wipe_at == now => {
                    at = Some(now + WIPE_COVER);
                    hold(stage, now + WIPE_COVER);
                }
                "revela" => hold(stage, now + REVEAL_SECS),
                "giro" => {
                    let end = now + arg.split_whitespace().count() as f32 * SPIN_FRAME + 0.06;
                    at = Some(end);
                    hold(stage, end);
                }
                "objeto" => {
                    at = None;
                    stage.item_wait = true;
                }
                _ => {}
            }
        }
        stage.tidy(now);
        // Las palabras de la caja empiezan cuando acaba el paso.
        let hold_until = stage.hold_until;
        if hold_until > now {
            self.talk_line_at = self.talk_line_at.max(hold_until);
        }
    }

    /// La Reina llevada a un sitio del castillo (un punto de interés, o donde está Lira o Nemi):
    /// en el hueco libre más cercano, vuelta hacia él, y la cámara con ella.
    pub(super) fn bring_queen_to(&mut self, place: &str) {
        let Some(inside) = self.castle.as_mut() else {
            eprintln!("[guion] @llevar {place}: fuera del castillo, no se mueve");
            return;
        };
        let Some((x, z, level)) = inside.place_of(place) else {
            eprintln!("[guion] @llevar {place}: no hay tal sitio");
            return;
        };
        if let Some(yaw) = inside.bring_to(x, z, level) {
            let at = inside.position();
            let focus = inside.focus();
            self.player = at;
            self.player_yaw = yaw;
            self.camera_focus = focus;
        }
    }

    /// Se pasa a la línea siguiente: el objeto enseñado se guarda.
    pub(super) fn stage_line_passed(&mut self) {
        if let Some(stage) = self.stage.as_mut() {
            if !stage.item_wait {
                stage.item = None;
            }
            stage.tidy(self.elapsed);
        }
    }

    /// F mientras se juega un paso del escenario: nada hasta que acabe; con un objeto enseñado, lo
    /// guarda (y pasa lo que esperaba por él). Verdad si el escenario se quedó la tecla.
    pub(super) fn stage_confirm(&mut self) -> bool {
        let now = self.elapsed;
        let Some(stage) = self.stage.as_mut() else {
            return false;
        };
        if stage.outro_at.is_some() {
            return false;
        }
        if now < stage.hold_until {
            return true;
        }
        if !stage.item_wait {
            return false;
        }
        stage.item_wait = false;
        stage.item = None;
        let lib = guion::library();
        let who_of = |name: &str| lib.cast.speaker_id(name);
        let mut waited = Vec::new();
        stage.pending.retain(|(t, cmd, arg)| {
            if t.is_none() {
                waited.push((cmd.clone(), arg.clone()));
            }
            t.is_some()
        });
        for (cmd, arg) in waited {
            stage.apply(&cmd, &arg, now, &who_of);
        }
        stage.tidy(now);
        self.talk_line_at = now;
        ui_sfx::play(UiSfx::Select);
        true
    }

    /// Un paso del escenario se está jugando (la caja espera; elegir, también).
    pub(super) fn stage_waiting(&self) -> bool {
        self.stage
            .as_ref()
            .is_some_and(|s| s.outro_at.is_none() && s.waiting(self.elapsed))
    }

    /// Chiquita dice su frase cuando el paso deja de jugarse.
    pub(super) fn stage_cue_later(&mut self) {
        if let Some(stage) = self.stage.as_mut() {
            stage.cue_wait = true;
        }
    }

    /// Cada paso: lo que esperaba su momento pasa; la luz de cada una, la franja detrás de quien
    /// habla y la caja siguen a la línea en pantalla; Chiquita dice su frase cuando acaba el paso.
    pub(super) fn tick_stage(&mut self) {
        if self.stage.is_none() {
            return;
        }
        let now = self.elapsed;
        let lib = guion::library();
        let speaking = self.stage_speaker();
        let in_box = self.stage_box_line();
        let mut cue = false;
        if let Some(stage) = self.stage.as_mut() {
            let who_of = |name: &str| lib.cast.speaker_id(name);
            let mut due = Vec::new();
            stage.pending.retain(|(t, cmd, arg)| match t {
                Some(t) if *t <= now => {
                    due.push((cmd.clone(), arg.clone()));
                    false
                }
                _ => true,
            });
            for (cmd, arg) in due {
                stage.apply(&cmd, &arg, now, &who_of);
            }
            stage.tidy(now);
            // Mientras habla Chiquita (o nadie de la escena), la luz se queda como estaba.
            if speaking.as_deref() != Some("chiquita") {
                for a in stage.actors.iter_mut() {
                    let lit = speaking.as_deref().is_none_or(|w| w == a.who);
                    if lit != a.lit {
                        a.lit = lit;
                        a.lit_at = now;
                    }
                }
                let side = speaking
                    .as_deref()
                    .and_then(|w| {
                        stage
                            .actors
                            .iter()
                            .find(|a| a.who == w && a.left_at.is_none())
                    })
                    .map(|a| a.right);
                if side != stage.focus {
                    stage.focus_prev = stage.focus;
                    stage.focus = side;
                    stage.focus_at = now;
                }
            }
            let waiting = stage.waiting(now);
            let on = in_box && !waiting;
            if on && !stage.box_on {
                stage.box_at = now;
            }
            stage.box_on = on;
            if stage.cue_wait && !waiting {
                stage.cue_wait = false;
                cue = true;
            }
        }
        if cue {
            if let Some(guion::Shown::Line {
                who,
                text,
                kind: guion::LineKind::Dialogue,
                mood,
                ..
            }) = self.talk.as_ref().map(|t| t.shown(lib, &self.story))
            {
                if who == "chiquita" {
                    self.mascot_say(mascot::line_cue(mood.as_deref(), &text), &text, 0.0);
                }
            }
        }
    }

    /// Quién habla en la línea en pantalla (con un pensamiento, la Reina; con una elección,
    /// también; con narración, nadie).
    fn stage_speaker(&self) -> Option<String> {
        let lib = guion::library();
        match self.talk.as_ref().map(|t| t.shown(lib, &self.story)) {
            Some(guion::Shown::Line { who, kind, .. }) => match kind {
                guion::LineKind::Dialogue => Some(who),
                guion::LineKind::Thought => {
                    Some(who.trim_end_matches(guion::THOUGHT_SUFFIX).to_owned())
                }
                _ => None,
            },
            Some(guion::Shown::Choice { .. }) => Some("gorou".to_owned()),
            _ => None,
        }
    }

    /// La línea en pantalla va en la caja (lo que dice alguien que no es Chiquita, una narración,
    /// un sonido), no en una tarjeta, un bocadillo o como elección.
    fn stage_box_line(&self) -> bool {
        let lib = guion::library();
        match self.talk.as_ref().map(|t| t.shown(lib, &self.story)) {
            Some(guion::Shown::Line { who, kind, .. }) => match kind {
                guion::LineKind::Thought => false,
                guion::LineKind::Dialogue => who != "chiquita",
                _ => true,
            },
            _ => false,
        }
    }

    pub(super) fn stage_up(&self) -> bool {
        self.stage.is_some()
    }

    /// Chiquita is saying a line on the stage: she says it herself, up in her corner (once the beat
    /// on show is over).
    pub(super) fn stage_chiquita_speaking(&self) -> bool {
        self.stage
            .as_ref()
            .is_some_and(|s| s.outro_at.is_none() && !s.waiting(self.elapsed))
            && matches!(
                self.talk.as_ref().map(|t| t.shown(guion::library(), &self.story)),
                Some(guion::Shown::Line { ref who, kind: guion::LineKind::Dialogue, .. }) if who == "chiquita"
            )
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
        let accent = self.dialogue_view().map_or(0, |d| {
            ((d.accent[0] as u32) << 16) | ((d.accent[1] as u32) << 8) | d.accent[2] as u32
        });
        let age = |t: f32| (now - t).max(0.0);
        // The last flash: a `@destello`, or one of the reveal's.
        let mut flash = age(stage.flash_at);
        if let Some((_, t0)) = &stage.reveal {
            for d in REVEAL_FLASHES {
                if now >= t0 + d {
                    flash = flash.min(now - (t0 + d));
                }
            }
        }
        Some(map_overlay::StageView {
            bg: stage.bg.clone(),
            bg_prev: stage.bg_prev.clone(),
            wipe: (age(stage.wipe_at) / 0.9).min(1.0),
            actors: stage
                .actors
                .iter()
                .map(|a| {
                    let k = (age(a.lit_at) / LIGHT_FADE).min(1.0);
                    let (from, to) = if a.lit { (DIM, 1.0) } else { (1.0, DIM) };
                    map_overlay::StageActor {
                        image: a.image.clone(),
                        prev: a.prev.clone(),
                        swap: age(a.swapped_at),
                        right: a.right,
                        enter: match a.left_at {
                            Some(t) => age(t),
                            None => age(a.entered_at),
                        },
                        leaving: a.left_at.is_some(),
                        shake: age(a.shake_at),
                        bump: age(a.bump_at),
                        lit: a.lit,
                        light: from + (to - from) * k,
                        spin: a
                            .spin
                            .as_ref()
                            .filter(|(frames, t, _)| age(*t) < frames.len() as f32 * SPIN_FRAME)
                            .map(|(frames, t, before)| (frames.clone(), age(*t), before.clone())),
                        scale: a.scale,
                        window: a.window,
                        x: a.x,
                    }
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
                .filter(|(_, _, t)| age(*t) < 1.6)
                .map(|(w, g, t)| (w.clone(), *g, age(*t))),
            flash,
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
                .map(|p| {
                    p.as_ref().map(|p| map_overlay::StagePanel {
                        image: p.image.clone(),
                        crop: p.crop,
                        age: age(p.at),
                        out: p.out_at.map(age),
                    })
                })
                .collect(),
            reveal: stage.reveal.as_ref().map(|(i, t)| (i.clone(), age(*t))),
            outro: stage.outro_at.map(|t| (stage.unlocks.clone(), age(t))),
            outro_title: stage
                .outro_title
                .clone()
                .unwrap_or_else(|| "ESCENA COMPLETA".to_owned()),
            age: age(stage.started),
            accent,
            speaker: stage.focus,
            speaker_prev: stage.focus_prev,
            speaker_age: age(stage.focus_at),
            line_age: (now - self.talk_line_at).max(0.0),
            box_age: if stage.box_on { age(stage.box_at) } else { 0.0 },
            waiting: stage.waiting(now),
            theme: stage.theme,
            preload: stage.preload.clone(),
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
        let mut s = Stage::new(0.0, 0);
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
        let mut s = Stage::new(0.0, 0);
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

    #[test]
    fn a_turn_plays_whole_and_ends_on_the_pose_that_follows_it() {
        let mut s = Stage::new(0.0, 1);
        s.apply("izq", "Reina termas_vn/reina_bikini_risa", 0.0, &who);
        s.apply("giro", "termas_vn/reina_giro1 termas_vn/reina_giro2", 1.0, &who);
        // A pose given mid-turn does not cut it short: it is where she ends.
        s.apply("pose", "Reina giro2", 1.2, &who);
        let a = &s.actors[0];
        assert!(a.spin.is_some(), "the turn goes on");
        assert_eq!(a.image, "termas_vn/reina_giro2");
        assert_eq!(
            a.spin.as_ref().map(|(_, _, before)| before.as_str()),
            Some("termas_vn/reina_bikini_risa")
        );
    }

    #[test]
    fn panels_keep_the_webs_framing_and_fly_off_at_a_reveal() {
        let mut s = Stage::new(0.0, 1);
        s.apply("panel", "1 termas_vn/vest_1 1080 50 -20", 0.0, &who);
        s.apply("panel", "2 termas_vn/vest_2", 0.5, &who);
        assert_eq!(
            s.panels[0].as_ref().and_then(|p| p.crop),
            Some([1080.0, 50.0, -20.0])
        );
        assert_eq!(s.panels[1].as_ref().and_then(|p| p.crop), None);
        s.apply("revela", "termas_vn/reina_bikini_timida", 2.0, &who);
        assert!(s.panels.iter().flatten().all(|p| p.out_at == Some(2.0)));
        s.tidy(2.0 + PANEL_OUT + 0.01);
        assert!(s.panels.iter().all(Option::is_none), "gone once they have flown");
        assert!(s.reveal.is_some());
        s.tidy(2.0 + REVEAL_SECS + 0.2);
        assert!(s.reveal.is_none());
    }

    #[test]
    fn a_wipe_clears_the_panels_once_its_bands_cover_the_stage() {
        let mut s = Stage::new(0.0, 1);
        s.apply("panel", "1 termas_vn/vest_1", 0.0, &who);
        s.apply("barrido", "", 1.0, &who);
        assert!(s.panels[0].is_some(), "still there until covered");
        assert_eq!(
            s.pending.first().map(|(t, c, _)| (*t, c.as_str())),
            Some((Some(1.0 + WIPE_COVER), "_despeja"))
        );
    }

    #[test]
    fn a_scene_knows_the_pictures_it_will_show() {
        let text = "## antes {en=otra}
@fondo x/no
## hola {en=hola}
@fondo x/bg
@izq Reina x/reina_normal 1.09 0.8 0.22
@pose Reina duda
## hola_b
@panel 1 x/vest_1 1080 50 -20
@giro x/g1 x/g2
@objeto x/item | Cosa | Texto
## luego {en=luego}
@fondo x/despues
";
        assert_eq!(
            scene_pictures(text, "hola"),
            ["x/bg", "x/reina_normal", "x/reina_duda", "x/vest_1", "x/g1", "x/g2", "x/item"]
        );
    }

    #[test]
    fn each_place_wears_its_web_scenes_colours() {
        assert_eq!(theme_of("termas.bienvenida"), 1);
        assert_eq!(theme_of("acto1.consejo_1"), 2);
        assert_eq!(theme_of("burdel.bienvenida"), 0);
    }
}
