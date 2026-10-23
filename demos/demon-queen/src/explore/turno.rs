//! «Al ritmo de la Reina»: los turnos con que la Reina paga su deuda en la casa de Rosalía
//! (`venue.rs`, `deuda.rs`). Cada turno es un cliente: su escena en el guion (`burdel.md`,
//! `turno_<n>`) y luego su minijuego en este escenario, visto desde los ojos de él.
//!
//! Ella lleva la máscara y el mando. Marca el ritmo con un anillo que se cierra sobre cada compás:
//! Espacio a tiempo le gusta (sube el vial y la propina), fuera de tiempo no. Mientras tanto el
//! aguante de él sube solo; Shift lo contiene (sube más despacio) pero el compás se estrecha. Las
//! tomas suben con el vial (con un puente entre las dos imágenes base de cada cliente); llena el
//! vial y él elige dónde termina (1 · 2). Si el aguante se llena antes, termina antes: ella se ríe y
//! cobra igual, sin propina. V cambia entre la vista de él y la de lado, cuando la toma tiene las
//! dos (`tools/escena_clips.py turnos` solo empaqueta las vistas buenas).
//!
//! La propina baja la deuda al cerrar la tarjeta.
use super::*;

/// The story variable the shift's guion sets to open its minigame: the scene's number (`turno_<n>`).
pub(crate) const GAME_VAR: &str = "juego_turno";

/// One step of her rhythm: its loop (or the bridge to the second picture), how fast she goes
/// (beats a second), how fast his stamina climbs, the pleasure that moves her on, what it says on
/// the stage, and what she says while it loops.
pub(crate) struct Step {
    pub clip: &'static str,
    pub bridge: bool,
    pub bps: f32,
    pub heat: f32,
    pub next: f32,
    pub label: &'static str,
    pub says: &'static [&'static str],
}

/// An ending he may choose: its clip, its tile, what she says as it starts and her last word.
pub(crate) struct Final {
    pub clip: &'static str,
    /// Another take of the same ending ("": none), played instead half the time.
    pub alt: &'static str,
    pub label: &'static str,
    pub say: &'static str,
    pub end: &'static str,
}

/// A client of the house.
pub(crate) struct Client {
    /// His folder (`burdel/turnos/<id>`).
    pub id: &'static str,
    pub name: &'static str,
    /// What the shift is.
    pub act: &'static str,
    /// The guion's scene (`turno_<n>`): its number.
    pub scene: i32,
    pub accent: [u8; 3],
    pub tint: [u8; 3],
    /// What she calls him.
    pub you: &'static str,
    pub intro: &'static str,
    pub steps: &'static [Step],
    pub finals: [Final; 2],
    /// His thoughts.
    pub think: &'static [&'static str],
    /// Chiquita's asides (the first opens the scene).
    pub chq: &'static [(&'static str, mascot::Cue)],
    /// The bridge back from the second picture to the first ("": none): an ending that starts on the
    /// first picture, picked on the second, plays it first.
    pub back: &'static str,
    /// Ended too soon: her word and Chiquita's.
    pub early: &'static str,
    pub early_chq: &'static str,
    /// Every shot that may be on disk (the views left out by the packing are skipped).
    pub shots: &'static [(&'static str, bool)],
}

const PINK: [u8; 3] = [0xff, 0x6e, 0xa0];

pub(crate) const CLIENTS: [Client; 4] = [
    Client {
        id: "bram",
        back: "p_b_a",
        name: "Bram",
        act: "La mamada",
        scene: 1,
        accent: PINK,
        tint: [0xff, 0x9a, 0x80],
        you: "soldado",
        intro: "La máscara de encaje, la sonrisa debajo. Se arrodilla sin dejar de mirarte.",
        steps: &[
            Step { clip: "c1_cara", bridge: false, bps: 0.8, heat: 0.015, next: 0.18, label: "Te la apoya en la mejilla y te mira",
                says: &["—Mírame a mí, soldado. No a eso.", "—Tranquilo. Todavía no he empezado."] },
            Step { clip: "c2_golpes", bridge: false, bps: 1.4, heat: 0.02, next: 0.36, label: "Golpecitos en la cara",
                says: &["—Toc, toc. ¿Hay alguien despierto ahí abajo?", "—Firme. Así me gusta la tropa."] },
            Step { clip: "c3_besos", bridge: false, bps: 1.0, heat: 0.025, next: 0.52, label: "Besos de abajo arriba",
                says: &["—Uno por cada moneda que has pagado. …Has pagado muchas.", "—Despacio. Esto se saborea."] },
            Step { clip: "p_a_b", bridge: true, bps: 1.0, heat: 0.0, next: 0.52, label: "Se la mete en la boca", says: &[] },
            Step { clip: "c4_mamada", bridge: false, bps: 1.25, heat: 0.03, next: 0.8, label: "La mamada, a su ritmo",
                says: &["—Mmh…", "(Sigue mi ritmo. Ni más rápido ni más lento.)"] },
            Step { clip: "c5_profunda", bridge: false, bps: 0.6, heat: 0.045, next: 1.0, label: "Hasta el fondo. Quieto.",
                says: &["—Mmmph…", "(Ni se te ocurra moverte.)"] },
        ],
        finals: [
            Final { clip: "f1_boca", alt: "f1_boca_b", label: "En su boca", say: "—Mmh… …no te muevas.",
                end: "—…Mmh. Cobro aparte por tragar. Es broma. …Casi." },
            Final { clip: "f2_cara", alt: "", label: "En su cara", say: "—Sácala. Aquí. Mírame.",
                end: "—En la máscara no, que es prestada. …Demasiado tarde." },
        ],
        think: &[
            "Respira. Respira. Es solo… una señora. Con cuernos. Y máscara.",
            "No pienses en el sargento. No pienses en el sargento.",
            "Esos ojos los he visto antes. …¿Dónde?",
        ],
        chq: &[
            ("—Mi reina, de rodillas, cobrando por horas. Lo apunto en tu biografía.", mascot::Cue::Jealous),
            ("—El chico no respira. Alguien debería decirle que respire.", mascot::Cue::Talk),
            ("—Ese compás es el de la marcha del cuartel. Lo reconocería en cualquier parte.", mascot::Cue::Talk),
            ("—Tres siglos de dinastía y acabamos pagando deudas así. …No pares, que vas bien.", mascot::Cue::Scold),
        ],
        early: "—¿Ya? …Soldado, eso ha sido una retirada, no una carga. Se cobra igual.",
        early_chq: "—Cuarenta segundos. He visto cargas de caballería más largas.",
        shots: &[
            ("c1_cara_pov", true), ("c1_cara_lado", true), ("c2_golpes_pov", true), ("c2_golpes_lado", true),
            ("c3_besos_pov", true), ("c3_besos_lado", true), ("p_a_b_pov", false), ("p_a_b_lado", false),
            ("c4_mamada_pov", true), ("c4_mamada_lado", true), ("c5_profunda_pov", true), ("c5_profunda_lado", true),
            ("f1_boca_pov", false), ("f1_boca_lado", false), ("f1_boca_b_pov", false), ("f1_boca_b_lado", false),
            ("f2_cara_pov", false), ("f2_cara_lado", false), ("p_b_a_pov", false), ("p_b_a_lado", false),
        ],
    },
    Client {
        id: "rus",
        back: "",
        name: "Rus",
        act: "La cubana",
        scene: 4,
        accent: [0x8f, 0xd4, 0x6a],
        tint: [0xff, 0xb0, 0x70],
        you: "goblin",
        intro: "Se baja las copas despacio, sin dejar de sonreír. Se te olvidan todos los chistes.",
        steps: &[
            Step { clip: "c1_aprieta", bridge: false, bps: 0.9, heat: 0.02, next: 0.25, label: "Te aprieta entre sus pechos",
                says: &["—¿Qué pasa, Rus? ¿Se te acabaron los chistes?", "—Quieto. Las manos donde yo las vea."] },
            Step { clip: "c2_rapido", bridge: false, bps: 1.6, heat: 0.035, next: 0.55, label: "Más rápido, riéndose",
                says: &["—¿Así? ¿Ahora te ríes?", "—Ja. Mírate la cara."] },
            Step { clip: "p_a_b", bridge: true, bps: 1.0, heat: 0.0, next: 0.55, label: "Baja la cabeza", says: &[] },
            Step { clip: "c3_lengua", bridge: false, bps: 1.1, heat: 0.04, next: 1.0, label: "Te lame la punta mientras aprieta",
                says: &["—Mmh. …Esto no te lo descuento de la tarifa.", "(Sigue mi ritmo, goblin.)"] },
        ],
        finals: [
            Final { clip: "f1_pecho", alt: "", label: "En sus pechos", say: "—Ahí. Donde estás.",
                end: "—Mira lo que has hecho. Esto se lava aparte." },
            Final { clip: "f2_cara", alt: "", label: "En su cara", say: "—¿En la cara? Atrévete.",
                end: "—¡En la máscara otra vez! ¿Es que nadie apunta en esta ciudad?" },
        ],
        think: &[
            "Diez monedas por esto. Las mejores diez monedas de mi vida.",
            "No hagas chistes. No hagas chistes. No…",
            "¿Los cuernos son de verdad? …No preguntes.",
        ],
        chq: &[
            ("—Un goblin. En la cama de una reina. Mi abuela se levantaría de la tumba… para mirar.", mascot::Cue::Jealous),
            ("—Vigílale las manos. Los goblins roban hasta cuando no quieren.", mascot::Cue::Scold),
            ("—Ese ritmo es de feria. Él lo conoce: síguelo.", mascot::Cue::Talk),
            ("—Cuando se lo cuente a su madriguera no le va a creer nadie. Mejor.", mascot::Cue::Talk),
        ],
        early: "—¿Ya? No te ha dado tiempo ni a contar un chiste.",
        early_chq: "—El goblin más rápido de los seis reinos. Y no por las manos.",
        shots: &[
            ("c1_aprieta_pov", true), ("c1_aprieta_lado", true), ("c2_rapido_pov", true), ("c2_rapido_lado", true),
            ("p_a_b_pov", false), ("p_a_b_lado", false), ("c3_lengua_pov", true), ("c3_lengua_lado", true),
            ("f1_pecho_pov", false), ("f1_pecho_lado", false), ("f2_cara_pov", false), ("f2_cara_lado", false),
        ],
    },
    Client {
        id: "gorm",
        back: "",
        name: "Gorm",
        act: "El primer asalto",
        scene: 5,
        accent: [0xc9, 0xa3, 0x6a],
        tint: [0xff, 0x8a, 0x80],
        you: "grandullón",
        intro: "La cama cruje. Ella se sienta sobre ti como quien se sienta en un trono.",
        steps: &[
            Step { clip: "c1_cabalga", bridge: false, bps: 0.8, heat: 0.02, next: 0.25, label: "Te cabalga despacio",
                says: &["—Tranquilo, grandullón. Yo me encargo.", "—No te muevas. Si te mueves, la cama se rompe."] },
            Step { clip: "c2_rapido", bridge: false, bps: 1.5, heat: 0.04, next: 0.55, label: "Más fuerte; la cama cruje",
                says: &["—¡Ah…! …Eres… mucho.", "—Así. Así. No pares… no, tú no. Yo."] },
            Step { clip: "p_a_b", bridge: true, bps: 1.0, heat: 0.0, next: 0.55, label: "Se tumba sobre tu pecho", says: &[] },
            Step { clip: "c3_pegada", bridge: false, bps: 0.9, heat: 0.04, next: 1.0, label: "Pegada a ti, moviendo las caderas",
                says: &["—Mmh… …hueles a bosque. Me gusta.", "(Quieto. Respira conmigo.)"] },
        ],
        finals: [
            Final { clip: "f1_dentro", alt: "", label: "Dentro", say: "—Dentro. Todo. Ni se te ocurra salirte.",
                end: "—…Mmh. Todo. …Esto también me lo quedo." },
            Final { clip: "f2_fuera", alt: "", label: "Fuera", say: "—Fuera, fuera… ¡ahora!",
                end: "—Uf. Necesito otra bañera. O dos." },
        ],
        think: &[
            "Es tan pequeña. …No, no lo es. Es que yo soy enorme.",
            "No apretar. No apretar. Las manos quietas.",
            "Nadie me había mirado así nunca.",
        ],
        chq: &[
            ("—Un ogro. La cama va a durar menos que tu última dieta.", mascot::Cue::Scold),
            ("—Si le da por estornudar, adiós burdel.", mascot::Cue::Flustered),
            ("—Lento, como un tambor de guerra. Él es el tambor.", mascot::Cue::Talk),
            ("—Mírala. Tres siglos de dinastía montando un ogro. Qué orgullo.", mascot::Cue::Jealous),
        ],
        early: "—¿Ya? Tanto ogro para tan poco asalto.",
        early_chq: "—La cama ha aguantado más que él. Apúntalo.",
        shots: &[
            ("c1_cabalga_pov", true), ("c1_cabalga_lado", true), ("c2_rapido_pov", true), ("c2_rapido_lado", true),
            ("p_a_b_pov", false), ("p_a_b_lado", false), ("c3_pegada_pov", true), ("c3_pegada_lado", true),
            ("f1_dentro_pov", false), ("f1_dentro_lado", false), ("f2_fuera_pov", false), ("f2_fuera_lado", false),
        ],
    },
    Client {
        id: "varg",
        back: "",
        name: "Varg",
        act: "Por detrás",
        scene: 6,
        accent: [0x9f, 0xb4, 0xc9],
        tint: [0xc8, 0x9a, 0xff],
        you: "lobito",
        intro: "Se pone a cuatro patas y te mira por encima del hombro. La cola se te mueve sola.",
        steps: &[
            Step { clip: "c1_perrito", bridge: false, bps: 0.9, heat: 0.02, next: 0.25, label: "Por detrás, a su ritmo",
                says: &["—Despacio, lobito. La luna aún no ha salido.", "—Las manos en las caderas. Las garras, guardadas."] },
            Step { clip: "c2_rapido", bridge: false, bps: 1.6, heat: 0.04, next: 0.55, label: "Más fuerte; ella empuja contra ti",
                says: &["—¡Ah…! …Así. No aúlles.", "—Más. …He dicho más, no más fuerte. …Bueno, también."] },
            Step { clip: "p_a_b", bridge: true, bps: 1.0, heat: 0.0, next: 0.55, label: "Apoya el pecho en las sábanas", says: &[] },
            Step { clip: "c3_hondo", bridge: false, bps: 0.8, heat: 0.04, next: 1.0, label: "Hondo y lento",
                says: &["—Mmh… …quédate ahí.", "(Hondo. Lento. Como yo diga.)"] },
        ],
        finals: [
            Final { clip: "f1_dentro", alt: "", label: "Dentro", say: "—Dentro. Quieto. …Así.",
                end: "—…Mmh. Ahora sí puedes aullar. Bajito." },
            Final { clip: "f2_espalda", alt: "", label: "En su espalda", say: "—Fuera. Sobre mí.",
                end: "—Mi espalda, mis sábanas y tu sueldo de un mes. Todo pringado." },
        ],
        think: &[
            "No muerdas. No muerdas. Es una dama.",
            "Huele a rosas y a algo más. A algo antiguo.",
            "La cola. Que alguien pare la cola.",
        ],
        chq: &[
            ("—Un lobo con modales. Lo que hay que ver.", mascot::Cue::Talk),
            ("—Si aúlla, nos echan a los tres. A mí la primera.", mascot::Cue::Flustered),
            ("—Ese ritmo es de caza. Él lo sabe de memoria.", mascot::Cue::Talk),
            ("—La luna aún no ha salido y ya está perdido.", mascot::Cue::Jealous),
        ],
        early: "—¿Ya? Y eso que no había luna.",
        early_chq: "—Lobo que ladra… bueno, este ni ha ladrado.",
        shots: &[
            ("c1_perrito_pov", true), ("c1_perrito_lado", true), ("c2_rapido_pov", true), ("c2_rapido_lado", true),
            ("p_a_b_pov", false), ("p_a_b_lado", false), ("c3_hondo_pov", true), ("c3_hondo_lado", true),
            ("f1_dentro_pov", false), ("f1_dentro_lado", false), ("f2_espalda_pov", false), ("f2_espalda_lado", false),
        ],
    },
];

/// The shifts in the order the house hands them out (the guion's scene numbers): Goswin (2) and the
/// masked lady (3) wait for their art.
pub(crate) const ROTATION: [i32; 4] = [1, 4, 5, 6];

/// Whether an ending clip starts on the second picture (the one after the bridge): each was made
/// from the picture the minigame is on when it comes.
fn final_on_b(client: &str, clip: &str) -> bool {
    matches!(
        (client, clip),
        ("bram", "f1_boca") | ("rus", "f2_cara") | ("gorm", "f2_fuera") | ("varg", "f2_espalda")
    )
}

/// The client of a guion scene's number.
pub(crate) fn client_of_scene(scene: i32) -> Option<usize> {
    CLIENTS.iter().position(|c| c.scene == scene)
}

/// The shots of a client that are on disk under `root` (a view the packing left out is skipped).
pub(super) fn shots_on_disk(client: usize, root: &Path) -> Vec<(&'static str, bool)> {
    CLIENTS[client.min(CLIENTS.len() - 1)]
        .shots
        .iter()
        .copied()
        .filter(|(name, _)| root.join(name).is_dir())
        .collect()
}

/// How long the rules stay up.
const RULES_SECS: f32 = 6.0;
/// The beat's window (s either side), wide and while holding back.
const WINDOW: f32 = 0.18;
const WINDOW_HELD: f32 = 0.1;
/// How much a beat on time pleases her, and how much one off it costs.
const HIT: f32 = 0.035;
const MISS: f32 = 0.03;
/// Holding back: the stamina climbs at this share.
const HELD: f32 = 0.45;
/// The tip: one gold a beat on time, up to this.
pub(crate) const TIP_MAX: u32 = 40;
/// Beats on time in a row from which each one is worth two gold (and a stamp on each such run).
const STREAK_GOLD: u32 = 8;

/// What the shift keeps.
#[derive(Default)]
pub(super) struct TurnoGame {
    pub client: usize,
    step: usize,
    /// His stamina, 0..=1: full, it is over.
    pub stamina: f32,
    /// On the side view (else his own).
    pub side: bool,
    /// Shift held: holding back.
    pub hold: bool,
    next_beat: f32,
    /// The beat just gone by, while its window is still open and it was not hit; and whether the next
    /// one was already hit (each beat is worth one press).
    last_beat: Option<f32>,
    next_hit: bool,
    /// Beats on time in a row, and the longest run.
    pub streak: u32,
    pub best: u32,
    /// When her current loop started (its cut-in over the picture), and when the take on show began.
    loop_at: f32,
    shot_at: f32,
    hit_at: f32,
    miss_at: f32,
    pub hits: u32,
    pub misses: u32,
    pub tip: u32,
    /// The bridge to the second picture is playing.
    bridging: bool,
    /// The vial is full: he picks where (since then).
    choosing: Option<f32>,
    /// The ending picked, and whether it came too soon.
    pub ending: Option<usize>,
    pub early: bool,
    /// The clip on show once the ending started (its take, or the bridge back), and the ending's shot
    /// waiting behind that bridge.
    shown: &'static str,
    then: Option<(&'static str, String)>,
    next_line: f32,
    scolded_at: f32,
}

impl TurnoGame {
    /// Since when he has been picking where.
    pub(super) fn choosing_since(&self) -> Option<f32> {
        self.choosing
    }
}

impl PlayScene {
    pub(super) fn client(&self) -> &'static Client {
        &CLIENTS[self.turno.client.min(CLIENTS.len() - 1)]
    }

    /// Her voice in this shift: masked, as he sees her.
    fn masked(&self) -> Voice {
        Voice::QueenMasked(self.turno.client as u8)
    }

    fn view_name(&self) -> &'static str {
        if self.turno.side {
            "lado"
        } else {
            "pov"
        }
    }

    /// `clip` in the view on show, or in the other one when that is the only one on disk.
    fn turno_shot(&mut self, clip: &str) -> Option<String> {
        let here = format!("{clip}_{}", self.view_name());
        if self.reel.index(&here).is_some() {
            return Some(here);
        }
        // Only the other view is good for this clip: it plays that one, and the next clip goes back to
        // the view he picked.
        let other = format!("{clip}_{}", if self.turno.side { "pov" } else { "lado" });
        if self.reel.index(&other).is_some() {
            return Some(other);
        }
        eprintln!("[turno] no hay toma «{clip}»");
        None
    }

    /// The clip on show (the step's, or the ending's).
    fn turno_clip(&self) -> &'static str {
        let c = self.client();
        match self.turno.ending {
            Some(_) if !self.turno.shown.is_empty() => self.turno.shown,
            Some(i) => c.finals[i].clip,
            None => c.steps[self.turno.step.min(c.steps.len() - 1)].clip,
        }
    }

    /// Both views of the clip on show are on disk.
    pub(super) fn turno_two_views(&self) -> bool {
        let clip = self.turno_clip();
        ["pov", "lado"]
            .iter()
            .all(|v| self.reel.index(&format!("{clip}_{v}")).is_some())
    }

    pub(super) fn open_turno(&mut self, client: usize) {
        self.turno.client = client.min(CLIENTS.len() - 1);
        self.rules_until = RULES_SECS;
        self.talk.free_at = 0.8;
        let c = self.client();
        self.tint = c.tint.map(f32::from);
        self.talk.say(Voice::Narration, c.intro);
        self.chq_free = 0.0;
        let (aside, cue) = c.chq[0];
        self.chiquita(aside, cue, true);
        self.turno.next_line = RULES_SECS + 3.5;
        self.turno_play_step(0.0);
    }

    /// The step's clip from its start; a loop starts its beat.
    fn turno_play_step(&mut self, fade: f32) {
        let now = self.clock;
        let c = self.client();
        let s = &c.steps[self.turno.step.min(c.steps.len() - 1)];
        if let Some(shot) = self.turno_shot(s.clip) {
            self.reel.play(&shot, fade, now);
        }
        self.turno.shot_at = now;
        self.turno.bridging = s.bridge;
        if !s.bridge {
            self.turno.next_beat = now.max(self.rules_until) + 0.6;
            self.turno.last_beat = None;
            self.turno.next_hit = false;
            self.turno.loop_at = now;
            if let Some(line) = s.says.first() {
                let line = if self.turno.step > 0 && s.says.len() > 1 && self.dice.roll() < 0.5 {
                    s.says[1]
                } else {
                    line
                };
                self.talk.say(self.masked(), line);
            }
        }
    }

    /// Each step: her beat goes round, his stamina climbs, the vial moves her on; full, he picks.
    pub(super) fn step_turno(&mut self, dt: f32) {
        let now = self.clock;
        if self.phase != Phase::Play || self.turno.ending.is_some() {
            return;
        }
        let c = self.client();
        if self.turno.bridging {
            if self.reel.done() {
                self.turno.step += 1;
                self.turno_play_step(0.0);
            }
            return;
        }
        if self.turno.choosing.is_some() || now < self.rules_until {
            return;
        }
        let s = &c.steps[self.turno.step];
        let period = 1.0 / s.bps;
        let mut passed = false;
        while now >= self.turno.next_beat {
            // The beat goes by: hit already, or its window stays open a moment for a late press.
            self.turno.last_beat = (!self.turno.next_hit).then_some(self.turno.next_beat);
            self.turno.next_hit = false;
            self.turno.next_beat += period;
            passed = true;
        }
        if passed {
            // His heart keeps her time, louder as he nears the end.
            self.sound(Sfx::HeartBeat, 0.0, 0.1 + 0.3 * self.turno.stamina);
        }
        let window = if self.turno.hold { WINDOW_HELD } else { WINDOW };
        if self.turno.last_beat.is_some_and(|t| now > t + window) {
            // A beat let go by: the run is broken.
            self.turno.last_beat = None;
            self.turno.streak = 0;
        }
        let held = if self.turno.hold { HELD } else { 1.0 };
        self.turno.stamina =
            (self.turno.stamina + s.heat * dt * held * (0.7 + self.heat * 0.6)).min(1.0);
        if self.turno.stamina >= 1.0 {
            // Too soon: the ending that starts on the picture he is on.
            let on_b = c.steps[..=self.turno.step].iter().any(|st| st.bridge);
            let i = c
                .finals
                .iter()
                .position(|f| final_on_b(c.id, f.clip) == on_b)
                .unwrap_or(0);
            self.turno_finish(i, true);
            return;
        }
        if self.heat >= s.next {
            if self.turno.step + 1 >= c.steps.len() {
                self.turno.choosing = Some(now);
                self.talk.reply(
                    self.masked(),
                    &format!("—Bien aguantado, {}. Te has ganado elegir: ¿dónde?", c.you),
                );
                self.stamp("¡ELIGE!", [0xff, 0xd5, 0x6b]);
                self.flash([0xff, 0xd0, 0x80], 0.5);
                return;
            }
            self.turno.step += 1;
            self.turno_play_step(0.35);
            if self.turno.step == 2 && c.chq.len() > 1 {
                let (aside, cue) = c.chq[1 + self.dice.below(c.chq.len() - 1)];
                self.chiquita(aside, cue, false);
            }
            return;
        }
        if self.talk.idle(now) && now >= self.turno.next_line {
            if self.dice.roll() < 0.35 {
                let thought = *self.dice.pick(c.think);
                self.talk
                    .say(Voice::Client(self.turno.client as u8), thought);
            } else if !s.says.is_empty() {
                let line = *self.dice.pick(s.says);
                self.talk.say(self.masked(), line);
            }
            if c.chq.len() > 1 && self.dice.roll() < 0.3 {
                let (aside, cue) = c.chq[1 + self.dice.below(c.chq.len() - 1)];
                self.chiquita(aside, cue, false);
            }
            self.turno.next_line = now + 7.0;
        }
    }

    /// Space (or a click): on her beat it pleases her, off it does not.
    pub(super) fn turno_beat(&mut self) {
        let now = self.clock;
        if self.phase != Phase::Play
            || self.turno.bridging
            || self.turno.choosing.is_some()
            || self.turno.ending.is_some()
            || now < self.rules_until
        {
            return;
        }
        let c = self.client();
        let s = &c.steps[self.turno.step];
        let window = if self.turno.hold { WINDOW_HELD } else { WINDOW };
        // Each beat takes one press: late on the one gone by, or early on the next.
        let late = self.turno.last_beat.is_some_and(|t| now - t < window);
        let early = !self.turno.next_hit && self.turno.next_beat - now < window;
        if late || early {
            if late {
                self.turno.last_beat = None;
            } else {
                self.turno.next_hit = true;
            }
            self.turno.streak += 1;
            self.turno.best = self.turno.best.max(self.turno.streak);
            let streak = self.turno.streak;
            // A slow beat is worth more: each one pleases her as much as a second of following her;
            // a run a little more still.
            let run = 1.0 + 0.05 * streak.min(8) as f32;
            self.heat = (self.heat + HIT * (1.0 / s.bps).max(1.0) * run).min(1.0);
            let gold = if streak >= STREAK_GOLD { 2 } else { 1 };
            self.turno.tip = (self.turno.tip + gold).min(TIP_MAX);
            self.turno.hits += 1;
            self.turno.hit_at = now;
            self.sound(Sfx::Pickup, 0.0, 0.18);
            if streak % STREAK_GOLD == 0 {
                self.stamp(&format!("¡RACHA ×{streak}!"), [0xff, 0xd5, 0x6b]);
                self.sound(Sfx::Coins, 0.05, 0.45);
            }
        } else {
            self.heat = (self.heat - MISS).max(0.0);
            self.turno.streak = 0;
            self.turno.misses += 1;
            self.turno.miss_at = now;
            if now - self.turno.scolded_at > 4.0 {
                self.turno.scolded_at = now;
                let line = *self.dice.pick(&[
                    "—Te he dicho a mi ritmo.",
                    "—Eh. Conmigo, no contra mí.",
                    "—Escucha. Uno… dos…",
                ]);
                self.talk.reply(self.masked(), line);
            }
        }
    }

    /// V: his eyes or from the side, where the clip has both; the loop keeps its moment.
    pub(super) fn turno_flip_view(&mut self) {
        if !self.turno_two_views() || self.ended.is_some() {
            return;
        }
        let clip = self.turno_clip();
        let time = self.reel.time;
        self.turno.side = !self.reel.name().ends_with("_lado");
        let name = format!("{clip}_{}", self.view_name());
        self.reel.play(&name, 0.2, self.clock);
        self.reel.time = time.min(self.reel.length(self.reel.on) - 1.0 / 24.0);
    }

    /// 1 · 2 (or a tile): where he ends.
    pub(super) fn turno_pick(&mut self, i: usize) {
        if self.turno.choosing.is_some() && i < 2 {
            self.turno_finish(i, false);
        }
    }

    fn turno_finish(&mut self, i: usize, early: bool) {
        let now = self.clock;
        let c = self.client();
        let f = &c.finals[i.min(1)];
        self.turno.choosing = None;
        self.turno.ending = Some(i.min(1));
        self.turno.early = early;
        // Half the time the other take, where there is one on disk.
        let take = if !f.alt.is_empty()
            && self
                .reel
                .index(&format!("{}_pov", f.alt))
                .or(self.reel.index(&format!("{}_lado", f.alt)))
                .is_some()
            && self.dice.roll() < 0.5
        {
            f.alt
        } else {
            f.clip
        };
        // Picked on the second picture, an ending that starts on the first goes back to it first.
        let on_b = c.steps[..=self.turno.step.min(c.steps.len() - 1)]
            .iter()
            .any(|st| st.bridge);
        let back = (on_b && !final_on_b(c.id, f.clip) && !c.back.is_empty())
            .then(|| self.turno_shot(c.back))
            .flatten();
        if let Some(shot) = self.turno_shot(take) {
            match back {
                Some(bridge) => {
                    self.reel.play(&bridge, 0.3, now);
                    self.turno.shown = c.back;
                    self.turno.then = Some((take, shot));
                }
                None => {
                    self.reel.play(&shot, 0.3, now);
                    self.turno.shown = take;
                }
            }
        }
        self.turno.shot_at = now;
        self.phase = Phase::Climax;
        self.phase_since = now;
        self.flash(PINK, 0.8);
        if early {
            self.talk.reply(self.masked(), "—¿Eh? …Oh. Ya veo.");
            self.chiquita(c.early_chq, mascot::Cue::Scold, true);
        } else {
            self.talk.reply(self.masked(), f.say);
        }
    }

    /// The ending clip has played: her last word, then the card.
    pub(super) fn turno_after(&mut self) {
        let now = self.clock;
        // Back on the first picture: now the ending.
        if let Some((take, shot)) = self.turno.then.take() {
            self.reel.play(&shot, 0.0, now);
            self.turno.shown = take;
            self.turno.shot_at = now;
            return;
        }
        let c = self.client();
        self.phase = Phase::Relax(now + 3.5);
        self.phase_since = now;
        if !self.turno.early {
            let (grade, color) = self.turno_grade();
            self.stamp(grade, color);
        }
        let line = match (self.turno.early, self.turno.ending) {
            (true, _) => c.early,
            (false, Some(i)) => c.finals[i.min(1)].end,
            _ => "",
        };
        if !line.is_empty() {
            self.talk.reply(self.masked(), line);
        }
    }

    /// How he kept her time, stamped over the picture at the end.
    fn turno_grade(&self) -> (&'static str, [u8; 3]) {
        let t = &self.turno;
        let total = (t.hits + t.misses).max(1) as f32;
        let share = t.hits as f32 / total;
        if share >= 0.95 && t.best >= 12 {
            ("¡IMPECABLE!", [0xff, 0xd5, 0x6b])
        } else if share >= 0.85 {
            ("¡A SU RITMO!", [0xff, 0x6e, 0xa0])
        } else if share >= 0.7 {
            ("CUMPLIDOR", [0xe8, 0xd4, 0xb0])
        } else {
            ("A TROMPICONES", [0xb8, 0x9a, 0xff])
        }
    }

    /// Where the vial moves her on (its marks).
    pub(super) fn turno_marks(&self) -> Vec<u16> {
        let steps = self.client().steps;
        steps
            .iter()
            .filter(|s| !s.bridge && s.next < 1.0)
            .map(|s| (s.next * 1000.0) as u16)
            .collect()
    }

    /// The tiles while he picks.
    pub(super) fn turno_chips(&self) -> Vec<map_overlay::GameChip> {
        if self.turno.choosing.is_none() {
            return Vec::new();
        }
        self.client()
            .finals
            .iter()
            .map(|f| map_overlay::GameChip {
                label: f.label.to_owned(),
                state: 0,
                ..Default::default()
            })
            .collect()
    }

    pub(super) fn turno_rules(&self) -> (String, Vec<(String, String)>) {
        let c = self.client();
        (
            format!("Al ritmo de la Reina · {}", c.name),
            vec![
                (
                    "Espacio".to_owned(),
                    "Al compás del anillo: sigues su ritmo, le gusta y sube la propina.".to_owned(),
                ),
                (
                    "Shift".to_owned(),
                    "Contenerte: tu aguante sube más despacio, pero el compás se estrecha."
                        .to_owned(),
                ),
                (
                    "V".to_owned(),
                    "Su vista o la de lado, cuando hay las dos.".to_owned(),
                ),
                (
                    "1 · 2".to_owned(),
                    "Llena su gusto antes que tu aguante y eliges dónde.".to_owned(),
                ),
            ],
        )
    }

    /// The words for the stage's top line.
    pub(super) fn turno_phase(&self) -> String {
        let c = self.client();
        if self.turno.choosing.is_some() {
            return "Te has ganado elegir: ¿dónde?".to_owned();
        }
        match self.phase {
            Phase::Climax | Phase::Relax(_) if self.turno.early => {
                "Retirada antes de tiempo".to_owned()
            }
            Phase::Climax | Phase::Relax(_) => "Un cliente contento".to_owned(),
            _ if self.turno.hold => "Te contienes…".to_owned(),
            _ => c.steps[self.turno.step.min(c.steps.len() - 1)]
                .label
                .to_owned(),
        }
    }

    /// Seconds to her next beat (QA plays on it); infinite when there is none to hit.
    pub(super) fn turno_beat_in(&self) -> f32 {
        let beating = matches!(self.game, Game::Turno(_))
            && self.phase == Phase::Play
            && !self.turno.bridging
            && self.turno.choosing.is_none()
            && self.clock >= self.rules_until;
        if beating {
            self.turno.next_beat - self.clock
        } else {
            f32::INFINITY
        }
    }

    /// The ring, his stamina and the view, for the overlay.
    pub(super) fn turno_view(&self) -> map_overlay::TurnoView {
        let now = self.clock;
        let c = self.client();
        let s = &c.steps[self.turno.step.min(c.steps.len() - 1)];
        let period = 1.0 / s.bps;
        let beating = self.phase == Phase::Play
            && !self.turno.bridging
            && self.turno.choosing.is_none()
            && now >= self.rules_until;
        let ring = if beating {
            (1.0 - (self.turno.next_beat - now) / period).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ms = |at: f32| {
            if at > 0.0 {
                ((now - at).max(0.0) * 1000.0) as u32
            } else {
                u32::MAX
            }
        };
        // The pips under the picture: one a loop and the last for the ending.
        let loops = c.steps.iter().filter(|st| !st.bridge).count();
        let done = c.steps[..self.turno.step.min(c.steps.len())]
            .iter()
            .filter(|st| !st.bridge)
            .count();
        let at = if self.turno.choosing.is_some() || self.turno.ending.is_some() {
            loops
        } else {
            done
        };
        map_overlay::TurnoView {
            beating,
            streak: self.turno.streak.min(999) as u16,
            at: at as u8,
            steps: (loops + 1) as u8,
            name: c.name.to_owned(),
            act: c.act.to_owned(),
            // Her new loop thrown over the picture (not the first: the rules are up then).
            cut: if done > 0
                && !self.turno.bridging
                && self.turno.choosing.is_none()
                && self.turno.ending.is_none()
            {
                s.label.to_owned()
            } else {
                String::new()
            },
            cut_ms: ms(self.turno.loop_at),
            view_new: if self.turno_two_views() {
                ms(self.turno.shot_at)
            } else {
                u32::MAX
            },
            teach: self.turno.hits < 3 && self.turno.step == 0,
            face: guion::portraits().find("clientes", Some(c.id)).map(|p| {
                map_overlay::PortraitRef {
                    image: p.image,
                    crop: p.crop,
                }
            }),
            ring: (ring * 1000.0) as u16,
            hit_ms: ms(self.turno.hit_at),
            miss_ms: ms(self.turno.miss_at),
            stamina: (self.turno.stamina.clamp(0.0, 1.0) * 1000.0) as u16,
            held: self.turno.hold,
            side: self.reel.name().ends_with("_lado"),
            views: self.turno_two_views(),
            tip: self.turno.tip as u16,
        }
    }

    /// The card: how it went and what it does (the tip lands on the debt when it closes).
    pub(super) fn turno_ending(
        &self,
        done: bool,
    ) -> (
        &'static str,
        Vec<(&'static str, i32)>,
        (Voice, &'static str),
        String,
    ) {
        let c = self.client();
        let t = &self.turno;
        let tip = if done && !t.early { t.tip } else { 0 };
        let how = format!(
            "{} · +{tip} oro · {}/{} a compás",
            c.name,
            t.hits,
            t.hits + t.misses
        );
        if !done {
            return (
                "Te detienes antes de tiempo",
                vec![("hunger", -1)],
                (self.masked(), "—¿Ya? El turno está pagado igual."),
                how,
            );
        }
        if t.early {
            return (
                "Retirada antes de tiempo",
                vec![("hunger", -2), ("deseo", 1)],
                (self.masked(), c.early),
                how,
            );
        }
        let end = t.ending.map_or("", |i| c.finals[i.min(1)].end);
        (
            "Un cliente contento",
            vec![("hunger", -4), ("deseo", 2)],
            (self.masked(), end),
            how,
        )
    }

    /// The tip to take off the debt, once the card closes.
    pub(super) fn turno_tip(&self) -> u32 {
        if self.ended == Some(true) && !self.turno.early {
            self.turno.tip
        } else {
            0
        }
    }

    pub(super) fn turno_thought(&self) -> &'static str {
        match (
            self.ended.unwrap_or(false),
            self.turno.early,
            self.turno.client,
        ) {
            (false, _, _) => "Turno cobrado. El resto, otro día.",
            (true, true, _) => "Ni la mitad de un turno. Y aun así cuenta para la deuda.",
            (true, false, 0) => {
                "Mañana hará guardia a la puerta de mi castillo y no sabrá por qué sonríe."
            }
            (true, false, 1) => "Un goblin sin chistes. Eso sí que no lo había visto nunca.",
            (true, false, 2) => "La cama ha sobrevivido. Rosalía no se lo va a creer.",
            (true, false, _) => "Ni ha aullado. Un caballero, para ser un lobo.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_client_climbs_to_a_full_vial_and_has_its_shots() {
        for c in &CLIENTS {
            let loops: Vec<&Step> = c.steps.iter().filter(|s| !s.bridge).collect();
            assert!(!loops.is_empty(), "{}", c.id);
            assert_eq!(c.steps.last().map(|s| s.next), Some(1.0), "{}", c.id);
            for w in loops.windows(2) {
                assert!(
                    w[1].next > w[0].next,
                    "{}: {} → {}",
                    c.id,
                    w[0].clip,
                    w[1].clip
                );
            }
            for s in c.steps {
                assert!(
                    c.shots
                        .iter()
                        .any(|(n, l)| n.starts_with(s.clip) && *l != s.bridge),
                    "{}: {}",
                    c.id,
                    s.clip
                );
                assert!(s.bridge || !s.says.is_empty(), "{}: {}", c.id, s.clip);
            }
            for clip in [c.back]
                .into_iter()
                .chain(c.finals.iter().map(|f| f.alt))
                .filter(|x| !x.is_empty())
            {
                assert!(
                    c.shots.iter().any(|(n, l)| n.starts_with(clip) && !*l),
                    "{}: {}",
                    c.id,
                    clip
                );
            }
            for f in &c.finals {
                assert!(
                    c.shots.iter().any(|(n, l)| n.starts_with(f.clip) && !*l),
                    "{}: {}",
                    c.id,
                    f.clip
                );
            }
            assert!(client_of_scene(c.scene).is_some());
        }
        for scene in ROTATION {
            assert!(client_of_scene(scene).is_some(), "turno_{scene}");
        }
    }

    #[test]
    fn rosalias_book_offers_every_client_with_a_minigame() {
        // Her clients' page (`venue::ROSTER`): each one served there has his shift here, same scene.
        for card in crate::exploration3d::venue::ROSTER
            .iter()
            .filter(|c| c.ready)
        {
            let c = client_of_scene(card.scene).map(|i| &CLIENTS[i]);
            assert_eq!(c.map(|c| c.id), Some(card.id), "turno_{}", card.scene);
        }
        for c in &CLIENTS {
            assert!(
                crate::exploration3d::venue::ROSTER
                    .iter()
                    .any(|r| r.id == c.id && r.ready),
                "{} missing from the book",
                c.id
            );
        }
    }
}
