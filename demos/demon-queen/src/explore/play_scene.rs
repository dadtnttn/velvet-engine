//! Los minijuegos de las chicas del burdel, en el escenario de las escenas íntimas (el marco, las
//! tarjetas con caras, el vial del sello, Chiquita en su esquina). Cada chica tiene el suyo:
//!
//! - **Alba, «Lo que pide Alba»** (sin el elixir, o con el sello despierto): no se calla ni en la
//!   cama. Cada poco grita lo que quiere y hay que elegir su palabra (1–3 o el clic) antes de que se
//!   consuma la mecha; cuanto más caliente está, peor se le entiende. Acertar le da justo eso (su
//!   toma) y llena el vial; tres seguidas llenan más. Chiquita sopla pistas, no siempre ciertas. Al
//!   final, una copla: la rima buena es el final perfecto. Con el elixir pide posturas, y el final
//!   es el de la postura en que se llena el vial.
//! - **Noa, «Silencio»** (con el sello despierto): las paredes del burdel son de papel. Mantener
//!   Espacio va más hondo y llena el vial, pero hace ruido; en la boca, además, le gasta el aire
//!   (sin aire tose, y eso se oye). Callada le gusta más: con poco ruido el vial sube más deprisa
//!   (el «sigilo»). A veces la de al lado se calla y escucha, y la pared aguanta menos ruido: si
//!   pasáis sin que os oiga, Noa lo celebra; si el ruido llega a la línea, llaman a la pared. Las
//!   posturas se abren según sube el vial (A/D); T tira de la correa. El final es el de la postura.
//!
//! Las líneas de ellas se quedan lo que se tarda en leerlas (las que llegan antes esperan), lo de
//! Chiquita flota en sus bocadillos y no más de una vez cada pocos segundos, salvo sus números (se
//! estampa contra la pantalla, vuela, rompe la pantalla delante de un corte a negro). La luz de la
//! sala toma el color de la postura y destella con lo que pasa.
//!
//! - **El mago, «No mires el anillo»** (`hipno.rs`): el hipnotista del bosque y su anillo; no es
//!   del burdel, pero se juega en el mismo escenario.
//! - **El tendero, «Otro trato»** (`tienda.rs`): los favores del regateo picante de la tienda.
//!
//! Las tomas son carpetas de fotogramas con su sonido (`tools/escena_clips.py`), como las de Nina.
use std::collections::VecDeque;

use super::dungeon::sound::Sfx;

use super::bed_scene::{card_line, label, Decoder, Voice};
use super::*;

#[path = "hipno.rs"]
mod hipno;
use hipno::HipnoGame;

#[path = "tienda.rs"]
pub(super) mod tienda;
use tienda::TiendaGame;

#[path = "turno.rs"]
pub(super) mod turno;
use turno::TurnoGame;

/// Frames per second of the clips.
const FPS: f32 = 24.0;

/// How long a line wants to stay to be read (s): about sixteen letters a second, never less than
/// a little over two seconds.
fn read_secs(text: &str) -> f32 {
    (1.1 + text.chars().count() as f32 * 0.058).clamp(2.2, 7.0)
}

/// A small random stream, so no two nights play the same.
struct Dice(u32);

impl Dice {
    fn roll(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 100_000) as f32 / 100_000.0
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.roll() * n as f32) as usize).min(n.saturating_sub(1))
    }

    fn pick<'a, T>(&mut self, list: &'a [T]) -> &'a T {
        &list[self.below(list.len())]
    }
}

// ───────────────────────────── the clips ─────────────────────────────

/// One shot: its frames, its folder (its sound is there), whether it loops.
struct Shot {
    name: &'static str,
    frames: Vec<PathBuf>,
    dir: PathBuf,
    looping: bool,
}

/// The picture going out under the new one.
#[derive(Clone, Copy)]
struct Fade {
    shot: usize,
    time: f32,
    rate: f32,
    start: f32,
    length: f32,
}

/// A scene's shots and the one on show, decoded on a thread of its own.
struct Reel {
    shots: Vec<Shot>,
    on: usize,
    time: f32,
    rate: f32,
    fade: Option<Fade>,
    decoder: Decoder,
    shown: Option<u64>,
    asked: Option<u64>,
    upload: Option<image::RgbaImage>,
    uploads: u32,
    /// Cut to black: nothing on show.
    black: bool,
    /// The loop came round to its start this step.
    wrapped: bool,
}

impl Reel {
    /// `None` when a shot is not on disk.
    fn open(root: &Path, list: &[(&'static str, bool)]) -> Option<Self> {
        let mut shots = Vec::with_capacity(list.len());
        for &(name, looping) in list {
            let dir = root.join(name);
            let frames = super::clip_video::frames(&dir);
            if frames.is_empty() {
                eprintln!("[juego] falta la toma {}", dir.display());
                return None;
            }
            shots.push(Shot {
                name,
                frames,
                dir,
                looping,
            });
        }
        Some(Self {
            shots,
            on: 0,
            time: 0.0,
            rate: 1.0,
            fade: None,
            decoder: Decoder::new(),
            shown: None,
            asked: None,
            upload: None,
            uploads: 0,
            black: false,
            wrapped: false,
        })
    }

    fn index(&self, name: &str) -> Option<usize> {
        self.shots.iter().position(|s| s.name == name)
    }

    fn name(&self) -> &'static str {
        self.shots[self.on].name
    }

    fn length(&self, shot: usize) -> f32 {
        self.shots[shot].frames.len().max(1) as f32 / FPS
    }

    /// On to `name` from its start, the old picture fading out under it for `fade` seconds.
    fn play(&mut self, name: &str, fade: f32, clock: f32) {
        let Some(next) = self.index(name) else {
            eprintln!("[juego] no hay toma «{name}»");
            return;
        };
        self.fade = (fade > 0.0 && !self.black).then_some(Fade {
            shot: self.on,
            time: self.time,
            rate: self.rate,
            start: clock,
            length: fade,
        });
        self.time = 0.0;
        self.on = next;
        self.black = false;
    }

    /// Nothing on show until the next `play`.
    fn cut_to_black(&mut self) {
        self.black = true;
        self.fade = None;
    }

    /// A shot that does not loop has played to its end.
    fn done(&self) -> bool {
        !self.shots[self.on].looping && self.time >= self.length(self.on) - 0.5 / FPS
    }

    /// The clocks run on; the frame they are on is asked for.
    fn step(&mut self, dt: f32, clock: f32) {
        self.wrapped = false;
        let len = self.length(self.on);
        if self.shots[self.on].looping {
            let t = self.time + dt * self.rate;
            self.wrapped = t >= len;
            self.time = t % len;
        } else {
            self.time = (self.time + dt).min(len);
        }
        let fade_len = self.fade.map(|f| self.length(f.shot));
        if let (Some(f), Some(len)) = (self.fade.as_mut(), fade_len) {
            f.time += dt * f.rate;
            f.time = if self.shots[f.shot].looping {
                f.time % len
            } else {
                f.time.min(len - 1.0 / FPS)
            };
            if clock - f.start >= f.length {
                self.fade = None;
            }
        }
        while let Ok((id, picture)) = self.decoder.done.try_recv() {
            self.decoder.busy = false;
            self.shown = Some(id);
            self.upload = Some(picture);
        }
        if self.black {
            return;
        }
        let frame = |shot: usize, time: f32| {
            let list = &self.shots[shot].frames;
            let i = ((time * FPS) as usize).min(list.len() - 1);
            (i, list[i].clone())
        };
        let (index, path) = frame(self.on, self.time);
        let blend = self.fade.map(|f| {
            let (i, other) = frame(f.shot, f.time);
            let weight = 1.0 - ((clock - f.start) / f.length).clamp(0.0, 1.0);
            (f.shot as u64, i as u64, other, (weight * 16.0).round())
        });
        let mut id = (self.on as u64) << 48 | (index as u64) << 32;
        if let Some((shot, i, _, w)) = &blend {
            id |= 1 << 31 | shot << 24 | i << 8 | *w as u64;
        }
        if !self.decoder.busy && self.shown != Some(id) && self.asked != Some(id) {
            let job = (id, path, blend.map(|(_, _, o, w)| (o, w / 16.0)));
            if self.decoder.ask.send(job).is_ok() {
                self.decoder.busy = true;
                self.asked = Some(id);
            }
        }
    }
}

// ───────────────────────────── the lines ─────────────────────────────

struct Said {
    voice: Voice,
    text: String,
    at: f32,
}

/// Her lines as cards (the newest lit, two older under it), each up long enough to be read; what
/// comes sooner waits its turn. Narration and sounds are a subtitle over the picture.
#[derive(Default)]
struct Talk {
    said: Vec<Said>,
    queue: VecDeque<(Voice, String)>,
    /// The newest card has been read by then.
    free_at: f32,
    caption: Option<(String, f32)>,
}

impl Talk {
    /// Said when the card on show has been read.
    fn say(&mut self, voice: Voice, text: &str) {
        if !voice.spoken() {
            let shown = if matches!(voice, Voice::Sound) {
                format!("— {text} —")
            } else {
                text.to_owned()
            };
            self.caption = Some((shown, f32::NAN));
            return;
        }
        if self.queue.iter().any(|(_, t)| t == text) {
            return;
        }
        if self.queue.len() >= 2 {
            self.queue.pop_front();
        }
        self.queue.push_back((voice, text.to_owned()));
    }

    /// An answer to what just happened: what was waiting goes, and it comes as soon as the card on
    /// show has been up a moment.
    fn reply(&mut self, voice: Voice, text: &str) {
        self.queue.clear();
        let shown_at = self.said.last().map_or(-10.0, |l| l.at);
        self.free_at = self.free_at.min(shown_at + 0.9);
        self.say(voice, text);
    }

    fn idle(&self, now: f32) -> bool {
        self.queue.is_empty() && now >= self.free_at
    }

    fn tick(&mut self, now: f32) {
        // A new subtitle stays as long as it takes to read.
        if let Some((text, until)) = self.caption.as_mut() {
            if until.is_nan() {
                *until = now + (1.6 + text.chars().count() as f32 * 0.055).clamp(2.6, 6.5);
            }
        }
        if self
            .caption
            .as_ref()
            .is_some_and(|(_, until)| now >= *until)
        {
            self.caption = None;
        }
        if now >= self.free_at {
            if let Some((voice, text)) = self.queue.pop_front() {
                self.said.retain(|l| l.text != text);
                self.free_at = now + read_secs(&text);
                self.said.push(Said {
                    voice,
                    text,
                    at: now,
                });
                if self.said.len() > 3 {
                    self.said.remove(0);
                }
            }
        }
    }

    fn cards(&self, now: f32) -> Vec<map_overlay::SceneLine> {
        let n = self.said.len();
        self.said
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let fresh = ((now - l.at) / 0.3).clamp(0.0, 1.0);
                card_line(
                    l.voice,
                    &l.text,
                    (fresh * 255.0) as u8,
                    (n - 1 - i).min(2) as u8,
                )
            })
            .collect()
    }
}

// ───────────────────────────── Alba ─────────────────────────────

/// Something she asks for: her shot when she gets it, her word, how she shouts it, and whether it
/// is one of her secrets (it fills more).
struct Want {
    shot: &'static str,
    word: &'static str,
    shout: (&'static str, &'static str, &'static str),
    bonus: bool,
}

const WANTS: [Want; 3] = [
    Want {
        shot: "lento",
        word: "DESPACIO",
        shout: ("¡", "DESPACIO", "!"),
        bonus: false,
    },
    Want {
        shot: "rapido",
        word: "RÁPIDO",
        shout: ("¡MÁS ", "RÁPIDO", "!"),
        bonus: false,
    },
    Want {
        shot: "oreja",
        word: "OREJAS",
        shout: ("¡LAS ", "OREJAS", "!"),
        bonus: true,
    },
];

const WANTS_FUTA: [Want; 4] = [
    Want {
        shot: "pies",
        word: "PIES",
        shout: ("¡CON LOS ", "PIES", "!"),
        bonus: false,
    },
    Want {
        shot: "perrito",
        word: "COLETAS",
        shout: ("¡TÍRAME DE LAS ", "COLETAS", "!"),
        bonus: false,
    },
    Want {
        shot: "vaginal",
        word: "DENTRO",
        shout: ("¡MÉTEMELA ", "DENTRO", "!"),
        bonus: false,
    },
    Want {
        shot: "anal",
        word: "ATRÁS",
        shout: ("¡POR ", "ATRÁS", "!"),
        bonus: true,
    },
];

/// Words she never said.
const DECOYS: [&str; 12] = [
    "PEPINILLOS",
    "ROSALÍA",
    "LA CUENTA",
    "SOPA",
    "CALCETINES",
    "TAMBORES",
    "EL MOLINERO",
    "PROPINA",
    "NINA",
    "CHISMES",
    "CEBOLLAS",
    "EL HERRERO",
];

fn good_lines(shot: &str) -> &'static [&'static str] {
    match shot {
        "lento" => &[
            "—Mmmh… así, así. ¿Ves? Escuchas mejor que el herrero.",
            "—Despacito… que quiero contarte lo del Consejo sin perder el hilo.",
        ],
        "rapido" => &[
            "—¡Eso! ¡Eso, eso, ESO! ¡Ay, que se me olvida el chisme!",
            "—¡Más! ¡No pares, que esto no lo sabe ni Nina!",
        ],
        "oreja" => &[
            "—¡Ahí NO! …Bueno, ahí sí. Ahí SÍ. ¡No se lo digas a nadie!",
            "—¡Mis orejas! ¡Son mi punto débil y lo SABES!",
        ],
        "pies" => &[
            "—¿Cosquillas? ¡Las tuyas! Mira cómo se mueve… ¡jajaja!",
            "—Con los pies también se cotillea, ¿sabes? Se nota todo.",
        ],
        "perrito" => &[
            "—¡Ay! ¡Como riendas! ¡Arre, Majestad, arre!",
            "—¡Más fuerte! Total, mañana me peino.",
        ],
        "vaginal" => &[
            "—¡Eso! Entera… ¡ENTERA! ¿Cómo va a caber esto? …Cabe.",
            "—Esto se lo cuento a Nina. Con dibujos.",
        ],
        _ => &[
            "—¡Por ahí NO…! …Bueno. Por ahí sí. Por ahí MUCHO.",
            "—No se lo digas a nadie. Es mi secreto. El único que guardo.",
        ],
    }
}

/// What she says to a wrong word (`{w}`: the word, as said; `{W}`: shouted).
const ALBA_BAD: [&str; 5] = [
    "—¡Que no, tonta! ¡Escucha lo que te digo!",
    "—¿{w}? ¿{W}? Me muero de la risa…",
    "—Pareces el herrero: oyes lo que quieres.",
    "—¿{w}? ¡Ja! Te has equivocado y lo pienso contar.",
    "—¿Quién ha dicho {w}? ¡Yo no he dicho {w}!",
];

/// A word as said in a sentence: its first letter up, the rest down.
fn said(word: &str) -> String {
    let lower = word.to_lowercase();
    let mut chars = lower.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

const ALBA_LATE: [&str; 3] = [
    "—¡Tardas más que Rosalía cobrando!",
    "—¿Hola? ¿Sigues ahí? Que me enfrío…",
    "—¡Que se me pasa! ¡Contesta, Majestad!",
];

/// What she rattles on about between her shouts, as the vial fills.
const CHATTER: [(f32, &str); 5] = [
    (0.0, "—¿Te cuento un secreto? Bueno, te lo cuento igual. Dicen que en el Bosque Oscuro hay luces que caminan…"),
    (0.2, "—…y el del Consejo paga las rondas con monedas NUEVAS. Nuevas, nuevas. ¿De dónde las saca, eh?"),
    (0.4, "—¡Y la mujer del molinero! Bueno, eso… eso luego. ¡Luego! ¡No pares!"),
    (0.6, "—Me… me he perdido. ¿Dónde iba? ¡Da igual! ¡No pares!"),
    (0.8, "—No… no me salen las palabras… ¡tú adivina!"),
];

const CHATTER_FUTA: [(f32, &str); 4] = [
    (
        0.0,
        "—¡Esto no cabe en ningún chisme! …Bueno. En uno sí. En el MÍO.",
    ),
    (
        0.25,
        "—Mañana lo sabe toda la villa. Con medidas. ¡Con MEDIDAS!",
    ),
    (
        0.5,
        "—Se me olvidan los rumores… ¡todos! ¡Me has borrado la cabeza!",
    ),
    (0.75, "—No… no sé ni lo que pido… ¡tú adivina!"),
];

/// Her copla after the climax: the verse, the word that rhymes, and the three on offer.
struct Copla {
    verse: &'static str,
    rhyme: &'static str,
    words: [&'static str; 3],
}

const COPLAS: [Copla; 3] = [
    Copla {
        verse: "Dicen que la Reina es seria, que no se ríe jamás… / ¡y en mi cama se ha pasado la noche pidiendo…",
        rhyme: "MÁS",
        words: ["MÁS", "PERDÓN", "AGUA"],
    },
    Copla {
        verse: "Me muerdes la oreja y pierdo el sentido y el hilo: / ¡que se entere todo el burdel, que aquí nadie duerme…",
        rhyme: "TRANQUILO",
        words: ["TRANQUILO", "NUNCA", "SOLA"],
    },
    Copla {
        verse: "Rosalía cobra la cama, el vino y hasta el candil, / pero esto que me has hecho tú no se paga ni con…",
        rhyme: "MIL",
        words: ["MIL", "DOS", "NADA"],
    },
];

const COPLAS_FUTA: [Copla; 3] = [
    Copla {
        verse: "Con el elixir de Rosalía no hay quien la aguante: / eso no es un sello, señora, ¡eso es un…",
        rhyme: "GIGANTE",
        words: ["GIGANTE", "SELLO", "DRAGÓN"],
    },
    Copla {
        verse: "Si me tiras de las coletas y me dices cosas guarras, / mañana lo cuento todo… ¡a voces y con…",
        rhyme: "GUITARRAS",
        words: ["GUITARRAS", "TAMBORES", "NINA"],
    },
    Copla {
        verse: "Por atrás dije que no, y lo dije muy convencida… / ¡y ahora te lo pido a gritos por el resto de mi…",
        rhyme: "VIDA",
        words: ["VIDA", "CENA", "SEMANA"],
    },
];

/// One of her shouts: what she wants, the three words, when it came and how long it waits, how
/// it reads, and what was picked (which, right, when).
struct Round {
    want: usize,
    words: [&'static str; 3],
    start: f32,
    secs: f32,
    shout: String,
    wild: bool,
    picked: Option<(Option<usize>, bool, f32)>,
}

/// The copla on offer: which, its words in the order shown, when it came, what was picked.
struct Verse {
    copla: usize,
    words: [&'static str; 3],
    start: f32,
    picked: Option<(Option<usize>, bool, f32)>,
}

#[derive(Default)]
struct AlbaGame {
    /// Her shot (without its second take's `_b`).
    shot: &'static str,
    /// The second take of the shot is on.
    take_b: bool,
    round: Option<Round>,
    next_round: f32,
    streak: u32,
    best: u32,
    hits: u32,
    misses: u32,
    misses_row: u32,
    /// The chatter already said, and when the next may come.
    chatted: usize,
    next_chat: f32,
    verse: Option<Verse>,
    perfect: bool,
    splatted: bool,
    swooped: bool,
}

/// How she says `want` this hot: plainly; stammering; or the letters all in a heap.
/// In the player's language: the whole shout and her word are translated as they are (the
/// catalogs hold both), and the word is stammered or jumbled inside the translated shout.
fn garble(want: &Want, heat: f32, dice: &mut Dice) -> (String, bool) {
    let tr = |s: &str| demon_queen_dungeon::i18n::tr(s).into_owned();
    let (pre, word, post) = want.shout;
    let shout = tr(&format!("{pre}{word}{post}"));
    if heat < 0.35 {
        return (shout, false);
    }
    let word = tr(word);
    // Her word put back in the shout as she says it.
    let say = |said: &str| {
        if shout.contains(word.as_str()) {
            shout.replacen(word.as_str(), said, 1)
        } else {
            format!("{said}!")
        }
    };
    let letters: Vec<char> = word.chars().collect();
    if heat < 0.7 {
        let head: String = letters.iter().take(2).collect();
        let rest: String = letters.iter().skip(2).collect::<String>().to_lowercase();
        return (say(&format!("{head}… {head}~{rest}")), false);
    }
    let mut mixed = letters.clone();
    for _ in 0..12 {
        for i in (1..mixed.len()).rev() {
            let j = dice.below(i + 1);
            mixed.swap(i, j);
        }
        if mixed != letters {
            break;
        }
    }
    (say(&mixed.iter().collect::<String>()), true)
}

// ───────────────────────────── Noa ─────────────────────────────

/// One of Noa's postures: its name, its shot (and a harder take while held), the vial it opens at,
/// how fast the vial fills, the noise climbs and her air goes while held (0: her mouth is free), its
/// light, what is happening and what Space does, her words in it (always, the first time, when it
/// opens), its climax, and whether that climax hides behind a cut to black (her clothes change in it).
struct NoaPose {
    label: &'static str,
    shot: &'static str,
    deep: Option<&'static str>,
    open_at: f32,
    heat: f32,
    noise: f32,
    air: f32,
    tint: [u8; 3],
    phase: &'static str,
    key: &'static str,
    lines: &'static [&'static str],
    first: &'static str,
    opens: &'static str,
    climax: &'static str,
    cut: bool,
}

/// With the elixir: her mouth, upside down, then the bed.
const NOA_FUTA: [NoaPose; 4] = [
    NoaPose {
        label: "De rodillas",
        shot: "rodillas",
        deep: Some("garganta"),
        open_at: 0.0,
        heat: 0.022,
        noise: 0.07,
        air: 0.16,
        tint: [0xff, 0xa2, 0x4a],
        phase: "De rodillas · suelta para que respire",
        key: "mantén: más hondo",
        lines: &[
            "—…Despacio. Que no nos oigan.",
            "—Mmh… …Sí. Así.",
            "—No… no mires. …Bueno, mira.",
        ],
        first: "—…Despacio. Que no nos oigan.",
        opens: "",
        climax: "climax_rodillas",
        cut: false,
    },
    NoaPose {
        label: "Del revés",
        shot: "reves",
        deep: None,
        open_at: 0.22,
        heat: 0.03,
        noise: 0.09,
        air: 0.24,
        tint: [0xd0, 0x30, 0x4a],
        phase: "Del revés · hasta el fondo… y que respire",
        key: "mantén: más hondo",
        lines: &["—…Mmh-mmh.", "—…mmh… ¡mmh…!"],
        first: "—…Así. Con la cabeza colgando. …No me sueltes.",
        opens: "—…Túmbame del revés. Así llega más hondo. …Lo leí.",
        climax: "climax_reves",
        cut: false,
    },
    NoaPose {
        label: "Misionero",
        shot: "misionero",
        deep: None,
        open_at: 0.45,
        heat: 0.026,
        noise: 0.06,
        air: 0.0,
        tint: [0xff, 0x6a, 0x8a],
        phase: "Misionero · se tapa la boca",
        key: "mantén: empujar",
        lines: &[
            "—…Me tapo la boca. Tú no pares.",
            "—Mmh… …Si se me escapa algo, haz como que no lo has oído.",
        ],
        first: "—…Boca arriba. Así me tapo mejor la boca.",
        opens: "—…Ahora tú. Encima. O detrás. …Despacio, que las paredes oyen.",
        climax: "climax_misionero",
        cut: true,
    },
    NoaPose {
        label: "Perrito",
        shot: "perrito",
        deep: None,
        open_at: 0.45,
        heat: 0.03,
        noise: 0.12,
        air: 0.0,
        tint: [0xc0, 0x20, 0x40],
        phase: "Perrito · la correa en tu mano",
        key: "mantén: empujar",
        lines: &[
            "—…Tira. Un poco más. …Pero que no nos oigan.",
            "—La correa… …es mía. La compré yo. No preguntes.",
        ],
        first: "—…A cuatro patas. Con la correa. …No digas nada.",
        opens: "—…Ahora tú. Encima. O detrás. …Despacio, que las paredes oyen.",
        climax: "climax_perrito",
        cut: false,
    },
];

/// Without the elixir: the Queen's fingers, her mouth, and then Noa's turn.
const NOA_PLAIN: [NoaPose; 3] = [
    NoaPose {
        label: "Tus dedos",
        shot: "dedos",
        deep: None,
        open_at: 0.0,
        heat: 0.026,
        noise: 0.06,
        air: 0.0,
        tint: [0xff, 0xb0, 0x70],
        phase: "Tus dedos · se muerde el dedo para callar",
        key: "mantén: más deprisa",
        lines: &[
            "—…Despacio. Así. …No pares.",
            "—Mmh… …Me muerdo el dedo. Si no, se oye.",
            "—Más adentro… …pero bajito.",
        ],
        first: "—…Tus dedos. …Fríos. Me gusta.",
        opens: "",
        climax: "climax_dedos",
        cut: false,
    },
    NoaPose {
        label: "Tu boca",
        shot: "oral",
        deep: Some("oral_b"),
        open_at: 0.25,
        heat: 0.03,
        noise: 0.08,
        air: 0.0,
        tint: [0xff, 0x6a, 0x9a],
        phase: "Tu boca · se tapa la boca con la mano",
        key: "mantén: más deprisa",
        lines: &[
            "—¡…! …Shh. Shh. Me tapo la boca.",
            "—Tu lengua… …no, no pares. Pero que no se oiga.",
            "—…Majestad. …Majestad.",
        ],
        first: "—…Ahí abajo. …No me mires a la cara.",
        opens: "—…Más. Con la boca. …Por favor.",
        climax: "climax_oral",
        cut: false,
    },
    NoaPose {
        label: "Su boca",
        shot: "reina",
        deep: None,
        open_at: 0.5,
        heat: 0.028,
        noise: 0.05,
        air: 0.14,
        tint: [0xb0, 0x6a, 0xff],
        phase: "Su boca · ahora la que calla eres tú",
        key: "mantén: que siga",
        lines: &[
            "—…Dime si lo hago bien.",
            "—Mmh… …Sabe a tormenta. …¿Te gusta?",
            "—Shh. Ahora la que no puede hacer ruido eres tú.",
        ],
        first: "—…Siéntate. Ahí. …Ahora me toca.",
        opens: "—…Déjame a mí. …Ahora te toca a ti callarte.",
        climax: "climax_reina",
        cut: false,
    },
];

/// Under this much noise she counts it double (her secret).
const QUIET: f32 = 0.4;
/// The line the wall takes, and the lower one while the next room listens.
const LIMIT: f32 = 1.0;
const LIMIT_LISTENING: f32 = 0.55;

const NOA_DEEP: [&str; 2] = ["—Mmmph…", "—…mmh… ¡mmh…!"];
const NOA_LEASH: [&str; 3] = [
    "—¡Ah…! …Shh. Shh, shh. Eso no ha sonado.",
    "—¡…! …Otra vez. Pero no tan fuerte. …O sí.",
    "—…Más corta. La correa. …Por favor.",
];
const NOA_COUGH: [&str; 2] = [
    "—¡Kof! ¡Kof, kof…! …Perdón. Perdón. …Otra vez.",
    "—¡Kh…! …No pasa nada. Sigue. Pero bajito.",
];
const NOA_QUIET: [&str; 3] = [
    "—…Así. Calladitas. …Me gusta más así.",
    "—Nadie lo sabe. …Eso es lo que más me gusta.",
    "—Shh… …Que piensen que estoy contando velas.",
];
/// What they say through the wall.
const KNOCKS: [&str; 5] = [
    "—¿Noa? ¿Todo bien ahí dentro?",
    "—Noa, cariño, ¿te has caído de la cama otra vez?",
    "—¡Las velas! ¿Se te ha caído algo?",
    "—…Sea lo que sea, que no se entere Rosalía.",
    "—¡NOA!",
];
const NOA_AFTER_KNOCK: [&str; 3] = [
    "—¡B-bien! ¡Todo bien! …Ay, por las velas. Qué vergüenza.",
    "—…¡Que sí! ¡Que estoy bien! …Lo sabe. Lo sabe todo el burdel.",
    "—…Mañana no salgo de la habitación. En una semana.",
];

#[derive(Default)]
struct NoaGame {
    pose: usize,
    /// How many postures are open.
    open: usize,
    noise: f32,
    air: f32,
    /// Held this long at a stretch; let go since.
    held: f32,
    let_go: f32,
    /// Coughing: holding counts again after.
    cough_until: f32,
    coughs: u32,
    knocks: u32,
    knock_at: f32,
    /// No knock again before (the wall needs a moment).
    grace: f32,
    /// The next room's listening: announced at `.0`, on until `.1`; knocked during it.
    listen: Option<(f32, f32)>,
    knocked_in: bool,
    next_listen: f32,
    passed: u32,
    leash_at: f32,
    /// A whisper about the quiet now and then; a line of the posture when the cards are free.
    quiet_for: f32,
    next_quiet: f32,
    next_line: f32,
    lines: usize,
    /// Her reply to a knock, due then.
    reply_at: Option<f32>,
    /// Postures she has been in (the first time each says more).
    tried: [bool; 4],
}

impl NoaGame {
    fn listening(&self, now: f32) -> bool {
        self.listen
            .is_some_and(|(from, until)| now >= from && now < until)
    }

    fn limit(&self, now: f32) -> f32 {
        if self.listening(now) {
            LIMIT_LISTENING
        } else {
            LIMIT
        }
    }

    /// 0 asleep, 1 stirring, 2 listening, 3 knocking.
    fn neighbour(&self, now: f32) -> u8 {
        if now - self.knock_at < 1.6 {
            3
        } else if self.listening(now) {
            2
        } else if self.listen.is_some_and(|(from, _)| now < from) || self.noise > 0.7 {
            1
        } else {
            0
        }
    }
}

// ───────────────────────────── the scene ─────────────────────────────

/// Which girl, and how.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Game {
    Alba,
    AlbaFuta,
    /// Noa without the elixir: the Queen's fingers and mouth, then hers.
    Noa,
    NoaFuta,
    /// The hypnotist of the wood and his ring.
    Hipno,
    /// The shopkeeper's favour `n` (`tienda::FAVORS`), paid for a cut in a haggle.
    Tienda(u8),
    /// A shift in Rosalía's house to pay the debt: client `n` (`turno::CLIENTS`).
    Turno(u8),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Phase {
    /// Her first shot plays once.
    Intro,
    Play,
    /// Cut to black before the climax (Chiquita breaks the screen) until then.
    Cut(f32),
    Climax,
    /// Alba's copla, over the climax's last frame.
    Verse,
    /// The afterglow: its shot, or the last frame held, at least until then.
    Relax(f32),
}

pub(super) struct PlayScene {
    pub(super) game: Game,
    reel: Reel,
    talk: Talk,
    dice: Dice,
    phase: Phase,
    phase_since: f32,
    pub(super) heat: f32,
    pub(super) holding: bool,
    held_glow: f32,
    clock: f32,
    started_at: f32,
    pub(super) ended: Option<bool>,
    ended_at: f32,
    /// How to play is up until then.
    rules_until: f32,
    /// What Chiquita says (and does) for the screen, and when she may chat again.
    floats: Vec<(String, mascot::Cue)>,
    chq_free: f32,
    /// Sounds due (what, when, how loud).
    sfx: Vec<(Sfx, f32, f32)>,
    /// The sound playing belongs to this shot.
    sound_for: Option<usize>,
    /// The room's light, eased toward the posture's; an event's flash; a shake; a stamp.
    tint: [f32; 3],
    pulse: Option<(f32, [u8; 3], f32)>,
    shake: Option<(f32, f32)>,
    stamp: Option<(String, [u8; 3], f32)>,
    climax_shot: &'static str,
    relax_shot: Option<&'static str>,
    alba: AlbaGame,
    noa: NoaGame,
    hipno: HipnoGame,
    tienda: TiendaGame,
    turno: TurnoGame,
    /// The pointer last frame (it pushes the gaze only while it moves).
    last_pointer: Option<(f32, f32)>,
}

impl PlayScene {
    fn open(game: Game, elapsed: f32) -> Option<Self> {
        // A shift's shots are those the packing kept (a view may be left out).
        let turno_dir: String;
        let turno_shots: Vec<(&'static str, bool)>;
        let (dir, shots): (&str, &[(&'static str, bool)]) = match game {
            Game::Alba => (
                "burdel/alba/escena",
                &[
                    ("intro", false),
                    ("lento", true),
                    ("lento_b", true),
                    ("rapido", true),
                    ("rapido_b", true),
                    ("oreja", true),
                    ("oreja_b", true),
                    ("climax", false),
                    ("relax", false),
                    ("final", false),
                ],
            ),
            Game::AlbaFuta => (
                "burdel/alba/escena_futa",
                &[
                    ("pies", true),
                    ("perrito", true),
                    ("vaginal", true),
                    ("anal", true),
                    ("climax_cv", false),
                    ("relax_cv", false),
                    ("climax_ca", false),
                    ("relax_ca", false),
                    ("climax_cf", false),
                    ("relax_cf", false),
                ],
            ),
            Game::Noa => (
                "burdel/noa/escena",
                &[
                    ("dedos", true),
                    ("oral", true),
                    ("oral_b", true),
                    ("reina", true),
                    ("climax_dedos", false),
                    ("climax_oral", false),
                    ("climax_reina", false),
                ],
            ),
            Game::NoaFuta => (
                "burdel/noa/escena_futa",
                &[
                    ("intro", false),
                    ("rodillas", true),
                    ("garganta", true),
                    ("reves", true),
                    ("misionero", true),
                    ("perrito", true),
                    ("climax_rodillas", false),
                    ("climax_reves", false),
                    ("climax_misionero", false),
                    ("climax_perrito", false),
                ],
            ),
            Game::Hipno => ("hipno/escena", &hipno::SHOTS),
            Game::Tienda(_) => ("tienda/escena", &tienda::SHOTS),
            Game::Turno(c) => {
                let client = &turno::CLIENTS[(c as usize).min(turno::CLIENTS.len() - 1)];
                turno_dir = format!("burdel/turnos/{}", client.id);
                let root = crate::exploration3d_state::data_dir()
                    .join("assets")
                    .join(&turno_dir);
                turno_shots = turno::shots_on_disk(c as usize, &root);
                (turno_dir.as_str(), turno_shots.as_slice())
            }
        };
        let root = crate::exploration3d_state::data_dir()
            .join("assets")
            .join(dir);
        let reel = Reel::open(&root, shots)?;
        let seed = (elapsed * 1000.0) as u32 ^ 0x9e37_79b9;
        let mut scene = Self {
            game,
            reel,
            talk: Talk::default(),
            dice: Dice(seed | 1),
            phase: if matches!(
                game,
                Game::AlbaFuta | Game::Noa | Game::Hipno | Game::Tienda(_) | Game::Turno(_)
            ) {
                Phase::Play
            } else {
                Phase::Intro
            },
            phase_since: 0.0,
            heat: 0.0,
            holding: false,
            held_glow: 0.0,
            clock: 0.0,
            started_at: elapsed,
            ended: None,
            ended_at: 0.0,
            rules_until: 6.2,
            floats: Vec::new(),
            chq_free: 0.0,
            sfx: Vec::new(),
            sound_for: None,
            tint: [255.0, 176.0, 96.0],
            pulse: None,
            shake: None,
            stamp: None,
            climax_shot: "",
            relax_shot: None,
            alba: AlbaGame {
                shot: if game == Game::AlbaFuta {
                    "pies"
                } else {
                    "lento"
                },
                next_round: 6.6,
                next_chat: 9.0,
                ..Default::default()
            },
            noa: NoaGame {
                open: 1,
                air: 1.0,
                tried: [true, false, false, false],
                next_listen: 24.0,
                next_line: 10.0,
                next_quiet: 14.0,
                ..Default::default()
            },
            hipno: HipnoGame::default(),
            tienda: TiendaGame::default(),
            turno: TurnoGame::default(),
            last_pointer: None,
        };
        // Her lines wait for the rules card to go.
        scene.talk.free_at = scene.rules_until;
        match game {
            Game::Alba => {
                scene.talk.say(Voice::Alba, "—¡Por fin alguien que me escucha! Túmbate, túmbate… y haz lo que yo te diga, ¿eh?");
                scene.chiquita("—Esta habla más que yo. Y eso que yo tengo historias para llenar una biblioteca.", mascot::Cue::Talk, true);
            }
            Game::AlbaFuta => {
                scene.talk.say(
                    Voice::Alba,
                    "—¡¿Con ESO?! …Vale. Vale, vale. Pero esto lo cuento. A TODO el mundo.",
                );
                scene.chiquita("—Le has enseñado ESO a la mayor chismosa de la villa. Mañana sale en los pregones.", mascot::Cue::Jealous, true);
            }
            Game::Noa => {
                scene.talk.say(
                    Voice::Noa,
                    "—…Sin el frasco. Solo nosotras. …Y muy bajito, ¿sí?",
                );
                scene.chiquita(
                    "—Sin elixir y todo. Esta noche la que escucha eres tú.",
                    mascot::Cue::Talk,
                    true,
                );
            }
            Game::NoaFuta => {
                scene.talk.say(
                    Voice::Noa,
                    "—…No va a caber. …Bueno. Lo intento. Pero bajito, ¿sí?",
                );
                scene.chiquita(
                    "—La tímida. Claro. Siempre son las tímidas.",
                    mascot::Cue::Jealous,
                    true,
                );
            }
            Game::Hipno => scene.open_hipno(),
            Game::Tienda(favor) => scene.open_tienda(favor as usize),
            Game::Turno(client) => scene.open_turno(client as usize),
        }
        Some(scene)
    }

    /// Chiquita says `text` doing `cue`; unless `now`, only if she has not spoken for a while.
    fn chiquita(&mut self, text: &str, cue: mascot::Cue, now: bool) {
        if !now && self.clock < self.chq_free {
            return;
        }
        self.chq_free = self.clock + 7.5 + read_secs(text);
        self.floats.push((text.to_owned(), cue));
    }

    fn flash(&mut self, color: [u8; 3], strength: f32) {
        self.pulse = Some((self.clock, color, strength));
    }

    fn jolt(&mut self, pixels: f32) {
        self.shake = Some((self.clock, pixels));
    }

    fn stamp(&mut self, words: &str, color: [u8; 3]) {
        self.stamp = Some((words.to_owned(), color, self.clock));
    }

    fn sound(&mut self, sfx: Sfx, after: f32, volume: f32) {
        self.sfx.push((sfx, self.clock + after, volume));
    }

    fn is_noa(&self) -> bool {
        matches!(self.game, Game::Noa | Game::NoaFuta)
    }

    /// Noa's postures in this game.
    fn noa_poses(&self) -> &'static [NoaPose] {
        if self.game == Game::NoaFuta {
            &NOA_FUTA
        } else {
            &NOA_PLAIN
        }
    }

    fn wants(&self) -> &'static [Want] {
        if self.game == Game::AlbaFuta {
            &WANTS_FUTA
        } else {
            &WANTS
        }
    }

    /// The light the room wants now.
    fn tint_target(&self) -> [u8; 3] {
        if matches!(
            self.phase,
            Phase::Climax | Phase::Cut(_) | Phase::Verse | Phase::Relax(_)
        ) {
            return [0xff, 0x6e, 0xa0];
        }
        match self.game {
            Game::Alba => match self.alba.shot {
                "rapido" => [0xff, 0x4a, 0x8a],
                "oreja" => [0xff, 0xd2, 0x5a],
                _ => [0xff, 0xb0, 0x60],
            },
            Game::AlbaFuta => match self.alba.shot {
                "pies" => [0xff, 0xa0, 0x70],
                "perrito" => [0xff, 0x5a, 0x9a],
                "vaginal" => [0xff, 0x3a, 0x5a],
                _ => [0xb0, 0x5a, 0xff],
            },
            Game::Noa | Game::NoaFuta => {
                if self.noa.listening(self.clock) {
                    [0x6a, 0x8a, 0xff]
                } else {
                    self.noa_poses()[self.noa.pose].tint
                }
            }
            Game::Hipno => {
                if self.hipno.covered(self.clock) {
                    [0xff, 0xc8, 0x6a]
                } else {
                    self.hipno.tint()
                }
            }
            Game::Tienda(_) => self.favor().tint,
            Game::Turno(_) => self.client().tint,
        }
    }

    // ── each step ──

    fn step(&mut self, dt: f32) {
        self.clock += dt;
        let now = self.clock;
        if self.ended.is_some() {
            if self.ended_at == 0.0 {
                self.ended_at = now;
            }
            return;
        }
        // The clocks wait for the first frame, so it never starts on a black square.
        let started = self.reel.shown.is_some();
        self.reel.step(if started { dt } else { 0.0 }, now);
        if !started {
            self.clock -= dt;
            return;
        }
        self.talk.tick(now);
        let target = self.tint_target();
        let k = 1.0 - (-dt * 2.5).exp();
        for (c, t) in self.tint.iter_mut().zip(target) {
            *c += (t as f32 - *c) * k;
        }
        let lit = self.holding
            && self.phase == Phase::Play
            && (self.is_noa() || self.game == Game::Hipno);
        let (aim, rate) = if lit { (1.0, 5.0) } else { (0.0, 2.2) };
        self.held_glow += (aim - self.held_glow) * (1.0 - (-dt * rate).exp());
        if self.game == Game::Hipno {
            self.hipno_reel();
            self.step_hipno(dt);
        }
        if matches!(self.game, Game::Tienda(_)) {
            self.step_tienda(dt);
        }
        if matches!(self.game, Game::Turno(_)) {
            self.step_turno(dt);
        }
        match self.phase {
            Phase::Intro => {
                if self.reel.done() {
                    self.begin_play();
                }
            }
            Phase::Play => match self.game {
                Game::Alba | Game::AlbaFuta => self.step_alba(dt),
                Game::Noa | Game::NoaFuta => self.step_noa(dt),
                Game::Hipno | Game::Tienda(_) | Game::Turno(_) => {}
            },
            Phase::Cut(until) => {
                if now >= until {
                    self.reel.play(self.climax_shot, 0.0, now);
                    self.phase = Phase::Climax;
                    self.phase_since = now;
                }
            }
            Phase::Climax => {
                if self.reel.done() {
                    self.after_climax();
                }
            }
            Phase::Verse => self.step_verse(),
            Phase::Relax(until) => {
                // A favour's loop goes on under its card: it ends on time.
                let shot_over = self.reel.done() || matches!(self.game, Game::Tienda(_));
                if now >= until && shot_over && (self.game != Game::Hipno || self.hipno_reel_done())
                {
                    self.ended = Some(true);
                }
            }
        }
    }

    fn begin_play(&mut self) {
        let now = self.clock;
        self.phase = Phase::Play;
        self.phase_since = now;
        match self.game {
            Game::Alba => self.reel.play("lento", 0.4, now),
            Game::AlbaFuta | Game::Noa | Game::Hipno | Game::Tienda(_) | Game::Turno(_) => {}
            Game::NoaFuta => {
                self.reel.play("rodillas", 0.5, now);
                self.noa.tried[0] = true;
            }
        }
    }

    // ── Alba ──

    fn step_alba(&mut self, dt: f32) {
        let now = self.clock;
        self.heat = (self.heat - 0.006 * dt).max(0.0);
        self.reel.rate = 0.95 + self.heat * 0.25;
        // Each time round, the other take of the same shot.
        if self.reel.wrapped && self.game == Game::Alba {
            let next = if self.alba.take_b {
                self.alba.shot.to_owned()
            } else {
                format!("{}_b", self.alba.shot)
            };
            if self.reel.index(&next).is_some() {
                self.reel.play(&next, 0.0, now);
                self.alba.take_b = !self.alba.take_b;
            }
        }
        // The shout: answered a moment ago it goes; unanswered, the fuse burns out; none, the next.
        let (over, burnt) = match &self.alba.round {
            Some(round) => (
                round.picked.is_some_and(|(_, _, at)| now - at > 0.95),
                round.picked.is_none() && now - round.start >= round.secs,
            ),
            None => (false, false),
        };
        if over {
            self.alba.round = None;
            self.alba.next_round = now + 1.5 + self.dice.roll() * 0.9;
        } else if burnt {
            self.answer(None);
        } else if self.alba.round.is_none()
            && now >= self.alba.next_round
            && now >= self.rules_until
        {
            self.new_round();
        }
        // She chatters between shouts, when the cards are free.
        if self.alba.round.is_none() && self.talk.idle(now) && now >= self.alba.next_chat {
            let chatter: &[(f32, &str)] = if self.game == Game::AlbaFuta {
                &CHATTER_FUTA
            } else {
                &CHATTER
            };
            if let Some(&(_, line)) = chatter
                .iter()
                .skip(self.alba.chatted)
                .find(|(at, _)| self.heat >= *at)
            {
                self.alba.chatted = chatter
                    .iter()
                    .position(|(_, l)| *l == line)
                    .map_or(chatter.len(), |i| i + 1);
                self.talk.say(Voice::Alba, line);
            }
            self.alba.next_chat = now + 8.0;
        }
    }

    fn new_round(&mut self) {
        let now = self.clock;
        let wants = self.wants();
        let current = wants
            .iter()
            .position(|w| w.shot == self.alba.shot)
            .unwrap_or(0);
        // Mostly something else than what she has; now and then more of the same.
        let want = if self.dice.roll() < 0.72 {
            let others: Vec<usize> = (0..wants.len()).filter(|&i| i != current).collect();
            *self.dice.pick(&others)
        } else {
            current
        };
        let others: Vec<usize> = (0..wants.len()).filter(|&i| i != want).collect();
        let other = wants[*self.dice.pick(&others)].word;
        let decoys: Vec<&'static str> = DECOYS.iter().copied().filter(|d| *d != other).collect();
        let decoy = *self.dice.pick(&decoys);
        let mut words = [wants[want].word, other, decoy];
        for i in (1..3).rev() {
            let j = self.dice.below(i + 1);
            words.swap(i, j);
        }
        let (shout, wild) = garble(&wants[want], self.heat, &mut self.dice);
        self.alba.round = Some(Round {
            want,
            words,
            start: now,
            secs: (5.4 - self.heat * 1.6).max(3.6),
            shout,
            wild,
            picked: None,
        });
        self.sound(Sfx::Pickup, 0.0, 0.35);
        // Chiquita whispers a hint now and then (not always the truth).
        if self.dice.roll() < 0.24 {
            let hint = if self.dice.roll() < 0.72 {
                format!(
                    "—Psst. Ha dicho «{}». De nada.",
                    demon_queen_dungeon::i18n::tr(wants[want].word)
                )
            } else {
                let wrong: Vec<&str> = words
                    .iter()
                    .copied()
                    .filter(|w| *w != wants[want].word)
                    .collect();
                format!(
                    "—Psst. Ha dicho «{}». …Seguro. Casi.",
                    self.dice.pick(&wrong)
                )
            };
            self.chiquita(&hint, mascot::Cue::Talk, false);
        }
    }

    /// Her word picked (`None`: the fuse burnt out).
    fn answer(&mut self, pick: Option<usize>) {
        let now = self.clock;
        let wants = self.wants();
        let Some(round) = self.alba.round.as_mut() else {
            return;
        };
        if round.picked.is_some() {
            return;
        }
        let want = &wants[round.want];
        let right = pick.is_some_and(|i| round.words.get(i) == Some(&want.word));
        round.picked = Some((pick, right, now));
        if right {
            self.alba.streak += 1;
            self.alba.hits += 1;
            self.alba.best = self.alba.best.max(self.alba.streak);
            self.alba.misses_row = 0;
            let gain = if want.bonus { 0.09 } else { 0.062 }
                * if self.alba.streak >= 3 { 1.5 } else { 1.0 };
            self.heat = (self.heat + gain).min(1.0);
            let line = *self.dice.pick(good_lines(want.shot));
            self.talk.reply(Voice::Alba, line);
            self.flash([0xff, 0xe0, 0x8a], 0.8);
            self.sound(Sfx::RuneChime, 0.0, 0.55);
            let streak = self.alba.streak;
            if streak == 3 || streak == 5 || (streak >= 8 && streak % 4 == 0) {
                self.stamp(&format!("¡RACHA ×{streak}!"), [0xff, 0x8a, 0x3d]);
                self.sound(Sfx::Coins, 0.15, 0.5);
            }
            if want.bonus {
                self.jolt(7.0);
                self.flash([0xff, 0x3a, 0x6a], 1.0);
                let text = if want.shot == "oreja" {
                    "—Las orejas de elfa. El cliché más viejo del mundo. Y funciona. Qué rabia."
                } else {
                    "—«Por ahí no», dice. Y luego pide más. Esta chica es un rumor con patas."
                };
                if !self.alba.splatted {
                    self.alba.splatted = true;
                    self.chiquita(text, mascot::Cue::Splat, true);
                } else {
                    self.chiquita(text, mascot::Cue::Flustered, false);
                }
            } else if want.shot == "perrito" && self.alba.shot != "perrito" {
                self.chiquita(
                    "—Riendas. Le has puesto riendas. No pienso comentar nada. …Arre.",
                    mascot::Cue::Scold,
                    false,
                );
            }
            if streak == 5 && !self.alba.swooped {
                self.alba.swooped = true;
                self.chiquita(
                    "—¡Cinco seguidas! ¿Desde cuándo escuchas tan bien? Conmigo nunca.",
                    mascot::Cue::Swoop,
                    true,
                );
            }
            if want.shot != self.alba.shot {
                let fade = if self.game == Game::AlbaFuta {
                    0.8
                } else {
                    0.5
                };
                self.alba.shot = want.shot;
                self.alba.take_b = false;
                self.reel.play(want.shot, fade, now);
            }
        } else {
            self.alba.streak = 0;
            self.alba.misses += 1;
            self.alba.misses_row += 1;
            self.heat = (self.heat - if pick.is_none() { 0.04 } else { 0.06 }).max(0.0);
            let picked = pick.and_then(|i| {
                self.alba
                    .round
                    .as_ref()
                    .and_then(|r| r.words.get(i).copied())
            });
            let line = match picked {
                None => (*self.dice.pick(&ALBA_LATE)).to_owned(),
                Some(word) => {
                    // In the player's language: the line, then the word put in it (said and
                    // shouted), each translated on its own.
                    let tr = |s: &str| demon_queen_dungeon::i18n::tr(s).into_owned();
                    let shouted = tr(word);
                    tr(self.dice.pick(&ALBA_BAD))
                        .replace("{w}", &said(&shouted))
                        .replace("{W}", &shouted)
                }
            };
            self.talk.reply(Voice::Alba, &line);
            self.flash([0x7a, 0x8a, 0xff], 0.6);
            if pick.is_some() {
                self.jolt(3.5);
                self.sound(Sfx::HitSlime, 0.0, 0.4);
            }
            if self.alba.misses_row == 3 {
                self.chiquita(
                    "—Majestad. Escucha. Con. Las. Orejas. Las tuyas, digo.",
                    mascot::Cue::Scold,
                    true,
                );
            }
        }
        if self.heat >= 1.0 {
            self.alba_climax();
        }
    }

    fn alba_climax(&mut self) {
        let now = self.clock;
        self.alba.round = None;
        self.stamp = None;
        let (shot, relax) = match self.game {
            Game::AlbaFuta => match self.alba.shot {
                "pies" => ("climax_cf", "relax_cf"),
                "anal" => ("climax_ca", "relax_ca"),
                _ => ("climax_cv", "relax_cv"),
            },
            _ => ("climax", "relax"),
        };
        self.climax_shot = shot;
        self.relax_shot = Some(relax);
        self.reel.play(shot, 0.6, now);
        self.phase = Phase::Climax;
        self.phase_since = now;
        self.talk.say(
            Voice::Narration,
            "Alba arquea la espalda y deja de hablar por primera vez en toda la noche.",
        );
        self.sound(Sfx::SealOpen, 2.2, 0.6);
        self.chiquita(
            "—Ya está. Media villa lo ha oído. Mañana lo sabe la otra media.",
            mascot::Cue::Flustered,
            false,
        );
    }

    fn begin_verse(&mut self) {
        let now = self.clock;
        let coplas: &[Copla] = if self.game == Game::AlbaFuta {
            &COPLAS_FUTA
        } else {
            &COPLAS
        };
        let copla = self.dice.below(coplas.len());
        let mut words = coplas[copla].words;
        for i in (1..3).rev() {
            let j = self.dice.below(i + 1);
            words.swap(i, j);
        }
        self.alba.verse = Some(Verse {
            copla,
            words,
            start: now,
            picked: None,
        });
        self.phase = Phase::Verse;
        self.phase_since = now;
        self.talk.reply(
            Voice::Alba,
            "—Ay… ay… espera, espera, que me ha salido una copla…",
        );
    }

    fn copla(&self) -> Option<&'static Copla> {
        let verse = self.alba.verse.as_ref()?;
        let coplas: &'static [Copla] = if self.game == Game::AlbaFuta {
            &COPLAS_FUTA
        } else {
            &COPLAS
        };
        coplas.get(verse.copla)
    }

    /// The rhyme picked (`None`: time ran out).
    fn rhyme(&mut self, pick: Option<usize>) {
        let now = self.clock;
        let Some(copla) = self.copla() else {
            return;
        };
        let Some(verse) = self.alba.verse.as_mut() else {
            return;
        };
        if verse.picked.is_some() {
            return;
        }
        let right = pick.is_some_and(|i| verse.words.get(i) == Some(&copla.rhyme));
        verse.picked = Some((pick, right, now));
        self.alba.perfect = right;
        if right {
            self.talk.reply(
                Voice::Alba,
                &format!(
                    "—¡{}! ¡JA! ¡Rima! Eres de las mías, Majestad.",
                    demon_queen_dungeon::i18n::tr(copla.rhyme)
                ),
            );
            self.flash([0xff, 0xe0, 0x8a], 1.0);
            self.stamp("¡RIMA!", [0xff, 0xc0, 0x40]);
            self.sound(Sfx::Relic, 0.0, 0.6);
            self.chiquita(
                "—¿Encima rimas? Me voy a tener que buscar otra reina.",
                mascot::Cue::Cheer,
                true,
            );
        } else if pick.is_some() {
            self.talk.reply(
                Voice::Alba,
                "—¡Eso no rima, guapa! …Me muero. Me MUERO de risa.",
            );
            self.flash([0x7a, 0x8a, 0xff], 0.6);
        } else {
            self.talk.reply(
                Voice::Alba,
                "—¿Te has quedado sin rima? ¡La Reina, sin palabras! Esto también lo cuento.",
            );
        }
    }

    fn step_verse(&mut self) {
        let now = self.clock;
        let Some((picked, start)) = self.alba.verse.as_ref().map(|v| (v.picked, v.start)) else {
            return;
        };
        match picked {
            None if now - start > 13.0 => self.rhyme(None),
            Some((_, right, at)) if now - at > 1.8 => {
                let shot = match (self.game, right) {
                    (Game::Alba, true) => "final",
                    _ => self.relax_shot.unwrap_or("relax"),
                };
                self.reel.play(shot, 0.6, now);
                self.phase = Phase::Relax(now + 4.0);
                self.phase_since = now;
                let thought = if right {
                    "Mañana lo sabrá todo el burdel. Y por una vez, no me importa."
                } else {
                    "Habla más que Chiquita. Y eso es mucho decir."
                };
                self.talk.say(Voice::Thought, thought);
            }
            _ => {}
        }
    }

    // ── Noa ──

    fn step_noa(&mut self, dt: f32) {
        let now = self.clock;
        let pose = self.noa.pose;
        let here = &self.noa_poses()[pose];
        let (heat_rate, noise_rate, air_rate) = (here.heat, here.noise, here.air);
        let oral = air_rate > 0.0;
        let holding = self.holding && now >= self.noa.cough_until;
        let sneaky = holding && self.noa.noise < QUIET;
        if holding {
            self.heat += heat_rate * if sneaky { 1.5 } else { 1.0 } * dt;
            self.noa.noise += noise_rate * dt;
            if oral {
                self.noa.air -= air_rate * dt;
            }
            self.noa.held += dt;
            self.noa.let_go = 0.0;
        } else {
            self.heat -= 0.007 * dt;
            self.noa.noise -= if self.noa.listening(now) { 0.28 } else { 0.2 } * dt;
            self.noa.air += 0.42 * dt;
            self.noa.held = 0.0;
            self.noa.let_go += dt;
        }
        self.heat = self.heat.clamp(0.0, 1.0);
        self.noa.noise = self.noa.noise.clamp(0.0, 1.2);
        self.noa.air = self.noa.air.clamp(0.0, 1.0);
        self.reel.rate = if holding {
            0.95 + 0.25 * self.heat
        } else {
            0.85
        };
        // A posture with a harder take: held a moment, that one; let go a moment, the slow one.
        if let Some(deep) = here.deep {
            let name = self.reel.name();
            if name == here.shot && holding && self.noa.held > 0.35 {
                self.reel.play(deep, 0.45, now);
                if self.talk.idle(now) {
                    let line = *self.dice.pick(&NOA_DEEP);
                    self.talk.say(Voice::Noa, line);
                }
            } else if name == deep && !holding && self.noa.let_go > 0.5 {
                self.reel.play(here.shot, 0.5, now);
            }
        }
        // Out of air: she coughs, and that is heard.
        if oral && self.noa.air <= 0.0 {
            self.cough();
        }
        // Quiet, she likes it more: now and then she whispers so.
        if sneaky {
            self.noa.quiet_for += dt;
            if self.noa.quiet_for > 6.0 && now >= self.noa.next_quiet && self.talk.idle(now) {
                let line = *self.dice.pick(&NOA_QUIET);
                self.talk.say(Voice::Noa, line);
                self.noa.next_quiet = now + 16.0;
                self.noa.quiet_for = 0.0;
            }
        } else if !holding {
            self.noa.quiet_for = 0.0;
        }
        self.step_wall();
        // Her reply to a knock.
        if self.noa.reply_at.is_some_and(|at| now >= at) {
            self.noa.reply_at = None;
            let line = NOA_AFTER_KNOCK[(self.noa.knocks as usize)
                .saturating_sub(1)
                .min(NOA_AFTER_KNOCK.len() - 1)];
            self.talk.reply(Voice::Noa, line);
        }
        // New postures as the vial fills (those that open together, announced together).
        let poses = self.noa_poses();
        let open = poses
            .iter()
            .take_while(|p| self.heat >= p.open_at)
            .count()
            .max(self.noa.open);
        if open > self.noa.open {
            let new = &poses[self.noa.open];
            let several = open - self.noa.open > 1;
            self.noa.open = open;
            self.stamp(
                if several {
                    "NUEVAS POSTURAS"
                } else {
                    "NUEVA POSTURA"
                },
                [0xff, 0xc8, 0x60],
            );
            self.sound(Sfx::Relic, 0.0, 0.55);
            self.talk.reply(Voice::Noa, new.opens);
            match new.shot {
                "reves" => self.chiquita(
                    "—¿«Lo leí»? ¿Dónde? ¿Qué lee esta chica?",
                    mascot::Cue::Flustered,
                    true,
                ),
                "reina" => self.chiquita(
                    "—¿Ahora ella a ti? …Esto quiero verlo. Por ciencia.",
                    mascot::Cue::Flustered,
                    true,
                ),
                _ => {}
            }
        }
        // A word of the posture now and then, when the cards are free.
        if self.talk.idle(now) && now >= self.noa.next_line {
            let lines = poses[pose].lines;
            let line = lines[self.noa.lines % lines.len()];
            self.noa.lines += 1;
            self.talk.say(Voice::Noa, line);
            self.noa.next_line = now + 9.0;
        }
        if self.heat >= 1.0 {
            self.noa_climax();
        }
    }

    /// The next room: now and then it goes quiet and listens; too loud, they knock.
    fn step_wall(&mut self) {
        let now = self.clock;
        match self.noa.listen {
            None if now >= self.noa.next_listen => {
                self.noa.listen = Some((now + 1.8, now + 1.8 + 6.5));
                self.noa.knocked_in = false;
                self.talk.say(
                    Voice::Sound,
                    "al otro lado de la pared, la cama deja de crujir",
                );
                self.chiquita(
                    "—Shh. Se ha callado la de al lado. Está escuchando.",
                    mascot::Cue::Scold,
                    true,
                );
            }
            Some((_, until)) if now >= until => {
                self.noa.listen = None;
                self.noa.next_listen = now + 18.0 + self.dice.roll() * 10.0;
                if !self.noa.knocked_in {
                    self.noa.passed += 1;
                    self.heat = (self.heat + 0.05).min(0.99);
                    self.stamp("¡NO OS HA OÍDO!", [0x5a, 0xff, 0xc8]);
                    self.flash([0x5a, 0xff, 0xc8], 0.7);
                    self.sound(Sfx::RuneChime, 0.0, 0.5);
                    self.talk
                        .reply(Voice::Noa, "—…No nos ha oído. …Otra vez. Igual de bajito.");
                }
            }
            _ => {}
        }
        // A moment to bring the noise down when they start to listen.
        let grace = self.noa.listen.is_some_and(|(from, _)| now < from + 1.2);
        if now >= self.noa.grace && !grace && self.noa.noise >= self.noa.limit(now) {
            self.knock();
        }
    }

    fn knock(&mut self) {
        let now = self.clock;
        self.noa.knocks += 1;
        self.noa.knock_at = now;
        self.noa.grace = now + 2.5;
        self.noa.noise = 0.2;
        self.heat = (self.heat - 0.12).max(0.0);
        if self.noa.listening(now) {
            self.noa.knocked_in = true;
        }
        self.stamp("¡TOC, TOC!", [0xff, 0x9a, 0x3a]);
        self.jolt(10.0);
        self.flash([0xff, 0x9a, 0x3a], 1.0);
        self.sound(Sfx::StepWood, 0.0, 1.0);
        self.sound(Sfx::StepWood, 0.24, 1.0);
        let line = KNOCKS[(self.noa.knocks as usize - 1).min(KNOCKS.len() - 1)];
        self.talk.caption = Some((line.to_owned(), f32::NAN));
        self.noa.reply_at = Some(now + 2.0);
        if self.noa.knocks == 1 {
            self.chiquita(
                "—¡¿QUIÉN ANDA AHÍ?! …Ah. La pared. Ya sabía yo.",
                mascot::Cue::Splat,
                true,
            );
        } else {
            self.chiquita(
                "—Te lo dije. Papel de fumar, estas paredes.",
                mascot::Cue::Scold,
                false,
            );
        }
    }

    fn cough(&mut self) {
        let now = self.clock;
        self.noa.coughs += 1;
        self.noa.air = 0.35;
        self.noa.noise += 0.3;
        self.heat = (self.heat - 0.04).max(0.0);
        self.noa.cough_until = now + 1.3;
        self.jolt(5.0);
        self.flash([0xff, 0xff, 0xff], 0.5);
        let line = *self.dice.pick(&NOA_COUGH);
        self.talk.reply(Voice::Noa, line);
        let here = &self.noa_poses()[self.noa.pose];
        if here.deep.is_some_and(|deep| self.reel.name() == deep) {
            self.reel.play(here.shot, 0.4, now);
        }
    }

    /// T: a tug on her leash.
    fn leash(&mut self) {
        let now = self.clock;
        if self.phase != Phase::Play || self.game != Game::NoaFuta || now - self.noa.leash_at < 0.9
        {
            return;
        }
        self.noa.leash_at = now;
        self.heat = (self.heat + 0.04).min(1.0);
        self.noa.noise += 0.2;
        if self.noa_poses()[self.noa.pose].air > 0.0 {
            self.noa.air = (self.noa.air - 0.12).max(0.0);
        }
        self.flash([0xff, 0x2a, 0x5a], 0.75);
        self.jolt(4.0);
        let line = *self.dice.pick(&NOA_LEASH);
        self.talk.reply(Voice::Noa, line);
        if self.dice.roll() < 0.35 {
            self.chiquita(
                "—La correa. Claro. La tímida tiene correa. Cómo no.",
                mascot::Cue::Flustered,
                false,
            );
        }
        if self.heat >= 1.0 {
            self.noa_climax();
        }
    }

    /// A/D (or a posture's cell): to another open posture.
    fn turn_pose(&mut self, to: usize) {
        let now = self.clock;
        if self.phase != Phase::Play || !self.is_noa() || to >= self.noa.open || to == self.noa.pose
        {
            return;
        }
        self.noa.pose = to;
        let here = &self.noa_poses()[to];
        self.reel.play(here.shot, 0.7, now);
        let first = !self.noa.tried[to];
        self.noa.tried[to] = true;
        let line = if first { here.first } else { here.lines[0] };
        self.talk.reply(Voice::Noa, line);
        self.noa.next_line = now + 9.0;
    }

    fn noa_climax(&mut self) {
        let now = self.clock;
        let here = &self.noa_poses()[self.noa.pose];
        self.stamp = None;
        let shot = here.climax;
        self.climax_shot = shot;
        self.relax_shot = None;
        self.heat = 1.0;
        let silent = self.noa.knocks == 0;
        let narration = match (shot, silent) {
            ("climax_reina", true) => {
                "La Reina echa la cabeza atrás sin un sonido; Noa no la suelta hasta el final."
            }
            ("climax_reina", false) => {
                "La Reina ahoga un gemido en la almohada; al otro lado de la pared, alguien tose."
            }
            (_, true) => "Noa se deshace en silencio, temblando, sin un solo ruido.",
            (_, false) => "Noa muerde la almohada; aun así, al otro lado de la pared alguien tose.",
        };
        self.talk.say(Voice::Narration, narration);
        self.sound(Sfx::SealOpen, 1.9, 0.6);
        if here.cut {
            // Her clothes go in this one: a cut to black, Chiquita in front of it breaking the
            // screen, and the picture comes back over the falling glass.
            self.reel.cut_to_black();
            self.phase = Phase::Cut(now + 1.9);
            self.chiquita(
                "—¡Quieta la cámara! …Vale. Ya. Ya puedes mirar.",
                mascot::Cue::Smash,
                true,
            );
        } else {
            self.reel.play(shot, 0.5, now);
            self.phase = Phase::Climax;
            if shot == "climax_reina" {
                self.chiquita(
                    "—…¿Eso has sido TÚ? Avisa, que estoy aquí dentro.",
                    mascot::Cue::Flustered,
                    true,
                );
            } else {
                self.chiquita(
                    "—…Ni un ruido. Qué miedo me da esta chica.",
                    mascot::Cue::Flustered,
                    false,
                );
            }
        }
        self.phase_since = now;
    }

    fn after_climax(&mut self) {
        let now = self.clock;
        match self.game {
            Game::Hipno => self.hipno_after(),
            Game::Tienda(_) => {}
            Game::Turno(_) => self.turno_after(),
            Game::Alba | Game::AlbaFuta => self.begin_verse(),
            Game::Noa | Game::NoaFuta => {
                self.phase = Phase::Relax(now + 5.0);
                self.phase_since = now;
                let line = match (self.game, self.climax_shot, self.noa.knocks) {
                    (Game::Noa, "climax_reina", _) => {
                        "—…¿Lo he hecho bien? …No digas nada. Ya lo sé."
                    }
                    (Game::Noa, _, 0) => "—…Quédate un poco. …Hasta que se apaguen las velas.",
                    (Game::NoaFuta, _, 0) => "—…Vuelve. Otra noche. …Con la correa.",
                    _ => "—…Mañana no salgo de la habitación. En una semana.",
                };
                self.talk.reply(Voice::Noa, line);
            }
        }
    }

    // ── what the keys and the pointer do ──

    fn set_holding(&mut self, holding: bool) {
        self.holding = holding;
    }

    /// 1–3 (or a tile clicked): her word, or the copla's rhyme.
    fn pick(&mut self, i: usize) {
        if matches!(self.game, Game::Turno(_)) {
            self.turno_pick(i);
            return;
        }
        if self.game == Game::Hipno {
            if self.phase == Phase::Play && self.hipno.order.is_some() {
                self.hipno_answer(i);
            } else if self.phase == Phase::Play && self.hipno.mouth() {
                self.hipno_pose(i);
            }
            return;
        }
        match self.phase {
            Phase::Play if self.alba.round.as_ref().is_some_and(|r| r.picked.is_none()) => {
                self.answer(Some(i))
            }
            Phase::Verse => self.rhyme(Some(i)),
            _ => {}
        }
    }

    /// Esc: half-way out while it builds; on to the card once it has burst.
    fn stop(&mut self) {
        self.ended = Some(!matches!(self.phase, Phase::Intro | Phase::Play));
    }

    fn card_ready(&self) -> bool {
        self.ended.is_some() && self.ended_at > 0.0 && self.clock - self.ended_at > 1.0
    }

    /// The tiles on show now (her words or the copla's).
    fn chips(&self) -> Vec<map_overlay::GameChip> {
        if matches!(self.game, Game::Turno(_)) {
            return self.turno_chips();
        }
        if self.game == Game::Hipno {
            return if self.phase == Phase::Play && self.hipno_chips_up() {
                self.hipno_chips()
            } else {
                Vec::new()
            };
        }
        let tiles =
            |words: &[&'static str; 3], right: &str, picked: Option<(Option<usize>, bool, f32)>| {
                words
                    .iter()
                    .enumerate()
                    .map(|(i, w)| map_overlay::GameChip {
                        label: (*w).to_owned(),
                        state: match picked {
                            None => 0,
                            Some(_) if *w == right => 1,
                            Some((Some(p), _, _)) if p == i => 2,
                            Some(_) => 3,
                        },
                        ..Default::default()
                    })
                    .collect()
            };
        match self.phase {
            Phase::Play => self
                .alba
                .round
                .as_ref()
                .map(|r| tiles(&r.words, self.wants()[r.want].word, r.picked))
                .unwrap_or_default(),
            Phase::Verse => match (self.alba.verse.as_ref(), self.copla()) {
                (Some(v), Some(c)) => tiles(&v.words, c.rhyme, v.picked),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    // ── the stage ──

    fn flash_now(&self) -> f32 {
        if self.phase != Phase::Climax {
            return 0.0;
        }
        let (start, peak) = match self.game {
            Game::Noa | Game::NoaFuta => (1.7, 2.1),
            Game::Hipno => (2.4, 2.9),
            _ => (2.3, 2.8),
        };
        let t = self.clock - self.phase_since;
        if t < start {
            0.0
        } else if t < peak {
            (t - start) / (peak - start)
        } else {
            (1.0 - (t - peak) / 1.6).max(0.0).powi(2)
        }
    }

    fn rules(&self) -> map_overlay::GameRules {
        if matches!(self.game, Game::Tienda(_) | Game::Turno(_)) {
            let (title, rows) = if matches!(self.game, Game::Turno(_)) {
                self.turno_rules()
            } else {
                self.tienda_rules()
            };
            return map_overlay::GameRules {
                title,
                rows,
                age_ms: (self.clock.max(0.0) * 1000.0) as u32,
                left_ms: ((self.rules_until - self.clock).max(0.0) * 1000.0) as u32,
            };
        }
        if self.game == Game::Hipno {
            return map_overlay::GameRules {
                title: "No mires el anillo".to_owned(),
                rows: self
                    .hipno_rules()
                    .into_iter()
                    .map(|(k, t)| (k.to_owned(), t))
                    .collect(),
                age_ms: (self.clock.max(0.0) * 1000.0) as u32,
                left_ms: ((self.rules_until - self.clock).max(0.0) * 1000.0) as u32,
            };
        }
        let (title, rows): (&str, &[(&str, &str)]) = match self.game {
            Game::Alba => (
                "Lo que pide Alba",
                &[
                    (
                        "1 2 3",
                        "Grita lo que quiere: elige su palabra antes de que se consuma la mecha.",
                    ),
                    (
                        "×1,5",
                        "Tres aciertos seguidos llenan más el vial. Fallar lo vacía.",
                    ),
                    ("¡¿?!", "Cuanto más caliente está, peor se le entiende."),
                    (
                        "Rima",
                        "Al final canta una copla: la rima buena es el final perfecto.",
                    ),
                ],
            ),
            Game::AlbaFuta => (
                "Lo que pide Alba · con el sello despierto",
                &[
                    (
                        "1 2 3",
                        "Pide postura a gritos: dale la que pide antes de que se consuma la mecha.",
                    ),
                    ("ATRÁS", "«¡Por atrás!» es su secreto: llena el doble."),
                    ("Final", "Se corre como estéis cuando se llene el vial."),
                    ("Rima", "Y luego, una copla: acierta la rima."),
                ],
            ),
            Game::Noa => (
                "Silencio · sin elixir",
                &[
                    (
                        "Espacio",
                        "Mantén: más deprisa. Suelta: que se calme. En su boca, también respira.",
                    ),
                    (
                        "Pared",
                        "Todo hace ruido. Si llega a la línea, llaman a la pared.",
                    ),
                    (
                        "Shh",
                        "Callada le gusta más: con poco ruido el vial sube más deprisa.",
                    ),
                    (
                        "A · D",
                        "Según se llena el vial: tus dedos, tu boca… y luego la suya.",
                    ),
                ],
            ),
            Game::NoaFuta => (
                "Silencio",
                &[
                    (
                        "Espacio",
                        "Mantén: más hondo. Suelta: que respire. Sin aire, tose.",
                    ),
                    (
                        "Pared",
                        "Todo hace ruido. Si llega a la línea, llaman a la pared.",
                    ),
                    (
                        "Shh",
                        "Callada le gusta más: con poco ruido el vial sube más deprisa.",
                    ),
                    (
                        "A · D",
                        "Posturas nuevas según se llena el vial. T: la correa.",
                    ),
                ],
            ),
            Game::Hipno | Game::Tienda(_) | Game::Turno(_) => ("", &[]),
        };
        map_overlay::GameRules {
            title: title.to_owned(),
            rows: rows
                .iter()
                .map(|(k, t)| ((*k).to_owned(), (*t).to_owned()))
                .collect(),
            age_ms: (self.clock.max(0.0) * 1000.0) as u32,
            left_ms: ((self.rules_until - self.clock).max(0.0) * 1000.0) as u32,
        }
    }

    fn view(&self, elapsed: f32) -> map_overlay::SceneView {
        let now = self.clock;
        let noa = self.is_noa();
        let hipno = self.game == Game::Hipno;
        let tienda = matches!(self.game, Game::Tienda(_));
        let turno = matches!(self.game, Game::Turno(_));
        let (who, accent) = if turno {
            ("reina_mascara", self.client().accent)
        } else if hipno {
            ("hypnotist", [0xd6, 0x5c, 0xff])
        } else if tienda {
            ("tendero", [0x4a, 0xd0, 0x9a])
        } else if noa {
            ("noa", [0xc4, 0x3a, 0x58])
        } else {
            ("alba", [0xf0, 0xc0, 0x60])
        };
        let portrait = guion::portraits()
            .find(who, turno.then(|| self.client().id))
            .map(|p| map_overlay::PortraitRef {
                image: p.image,
                crop: p.crop,
            });
        let mut game = map_overlay::GameView {
            kind: if turno {
                3
            } else if hipno {
                2
            } else {
                u8::from(noa)
            },
            who: if turno {
                self.client().name
            } else if hipno {
                "Mago"
            } else if tienda {
                "Tendero"
            } else if noa {
                "Noa"
            } else {
                "Alba"
            }
            .to_owned(),
            accent,
            face: portrait,
            tint: self.tint.map(|c| c.round().clamp(0.0, 255.0) as u8),
            ..Default::default()
        };
        if let Some((at, color, strength)) = self.pulse {
            let t = now - at;
            game.pulse_tint = color;
            game.pulse = ((strength * (-t / 0.35).exp()).clamp(0.0, 1.0) * 1000.0) as u16;
        }
        if let Some(done) = self.ended {
            return self.result_view(done, elapsed, game);
        }
        if let Some((at, px)) = self.shake {
            let t = now - at;
            game.shake = ((px * (-t / 0.22).exp()).max(0.0) * 10.0) as u16;
        }
        if now < self.rules_until {
            game.rules = Some(self.rules());
        }
        if let Some((words, color, at)) = &self.stamp {
            if now - at < 1.7 {
                game.stamp = Some((words.clone(), *color, ((now - at) * 1000.0) as u32));
            }
        }
        let phase: String;
        if turno {
            game.turno = Some(self.turno_view());
            game.chips = self.chips();
            if let Some(since) = self.turno.choosing_since() {
                game.chips_age = ((now - since) * 1000.0) as u32;
            }
            if self.phase == Phase::Play && game.chips.is_empty() {
                let tv = game
                    .turno
                    .as_ref()
                    .map(|t| (t.hit_ms < 250, t.held, t.views));
                let (hit, held, views) = tv.unwrap_or_default();
                game.keys
                    .push(("ESPACIO".to_owned(), "al compás".to_owned(), hit));
                game.keys
                    .push(("SHIFT".to_owned(), "contenerte".to_owned(), held));
                if views {
                    game.keys
                        .push(("V".to_owned(), "otra vista".to_owned(), false));
                }
            } else if !game.chips.is_empty() {
                game.keys
                    .push(("1 2".to_owned(), "elige dónde".to_owned(), false));
            }
            phase = self.turno_phase();
        } else if hipno {
            game.hypno = Some(self.hipno_view());
            game.chips = self.chips();
            if let Some((_, since)) = self.hipno.order {
                game.chips_age = ((now - since) * 1000.0) as u32;
                game.chips_left =
                    Some(((1.0 - (now - since) / 6.0).clamp(0.0, 1.0) * 1000.0) as u16);
            } else if let Some((_, _, _, at)) = self.hipno.answered {
                game.chips_age = ((now - at) * 1000.0) as u32 + 400;
            }
            game.keys = self.hipno_keys();
            if self.phase == Phase::Play && self.hipno.mouth() {
                game.poses = (0..2)
                    .map(|i| (self.hipno.pose_name(i).to_owned(), true))
                    .collect();
                game.pose = self.hipno.pose as u8;
                game.pose_active = self.reel.name() != "c5";
            }
            phase = self.hipno_phase();
        } else if tienda {
            if self.phase == Phase::Play {
                game.keys
                    .push(("ESPACIO".to_owned(), "darle prisa".to_owned(), self.holding));
            }
            phase = self.tienda_phase();
        } else if noa {
            let n = &self.noa;
            let listening = n.listening(now);
            game.noise = Some((n.noise.min(1.0) * 1000.0) as u16);
            game.limit = (n.limit(now) * 1000.0) as u16;
            let poses = self.noa_poses();
            let here = &poses[n.pose];
            game.air =
                (here.air > 0.0 && self.phase == Phase::Play).then(|| (n.air * 1000.0) as u16);
            game.sneaky = self.phase == Phase::Play && self.holding && n.noise < QUIET;
            game.neighbour = n.neighbour(now);
            game.neighbour_ms = match game.neighbour {
                3 => ((now - n.knock_at) * 1000.0) as u32,
                2 => n
                    .listen
                    .map_or(0, |(from, _)| ((now - from).max(0.0) * 1000.0) as u32),
                _ => 0,
            };
            game.poses = poses
                .iter()
                .enumerate()
                .map(|(i, p)| (p.label.to_owned(), i < n.open))
                .collect();
            game.pose = n.pose as u8;
            game.pose_active = self.phase == Phase::Play && n.open > 1;
            if self.phase == Phase::Play {
                game.keys
                    .push(("ESPACIO".to_owned(), here.key.to_owned(), self.holding));
                if self.game == Game::NoaFuta {
                    game.keys.push((
                        "T".to_owned(),
                        "la correa".to_owned(),
                        now - n.leash_at < 0.5,
                    ));
                }
            }
            phase = match self.phase {
                Phase::Intro => "Noa se arrodilla, sin mirarte…".to_owned(),
                Phase::Play if listening => "¡Shh! La de al lado está escuchando".to_owned(),
                Phase::Play if now < n.cough_until => "Tose… déjala respirar".to_owned(),
                Phase::Play => here.phase.to_owned(),
                Phase::Cut(_) | Phase::Climax if self.climax_shot == "climax_reina" => {
                    "La Reina se deshace…".to_owned()
                }
                Phase::Cut(_) | Phase::Climax => "Noa se deshace en silencio…".to_owned(),
                _ => "Calma".to_owned(),
            };
        } else {
            if self.phase == Phase::Play {
                game.streak = self.alba.streak as u16;
            }
            if let Some(round) = &self.alba.round {
                game.shout = Some(map_overlay::GameShout {
                    text: round.shout.clone(),
                    wild: round.wild,
                    age_ms: ((now - round.start) * 1000.0) as u32,
                });
                game.chips_age = ((now - round.start) * 1000.0) as u32;
                if round.picked.is_none() {
                    game.chips_left = Some(
                        ((1.0 - (now - round.start) / round.secs).clamp(0.0, 1.0) * 1000.0) as u16,
                    );
                }
            }
            if let (Some(verse), Some(copla)) = (&self.alba.verse, self.copla()) {
                if self.phase == Phase::Verse {
                    let said = demon_queen_dungeon::i18n::tr(copla.verse);
                    game.verse = Some(format!("«{}»", said.replace(" / ", "\n")));
                    game.chips_age = ((now - verse.start) * 1000.0) as u32;
                    if verse.picked.is_none() {
                        game.chips_left = Some(
                            ((1.0 - (now - verse.start) / 13.0).clamp(0.0, 1.0) * 1000.0) as u16,
                        );
                    }
                }
            }
            game.chips = self.chips();
            if game.chips.is_empty() && self.phase == Phase::Play {
                game.keys
                    .push(("1 2 3".to_owned(), "elige lo que grita".to_owned(), false));
            }
            phase = match self.phase {
                Phase::Intro => "Alba se tira en la cama riéndose…".to_owned(),
                Phase::Play if self.alba.round.as_ref().is_some_and(|r| r.picked.is_none()) => {
                    "¡Elige lo que pide!".to_owned()
                }
                Phase::Play => "Escucha lo que pide Alba".to_owned(),
                Phase::Cut(_) | Phase::Climax => "¡Alba se deshace!".to_owned(),
                Phase::Verse => "La copla: elige la rima".to_owned(),
                Phase::Relax(_) => if self.alba.perfect {
                    "Final perfecto"
                } else {
                    "Calma"
                }
                .to_owned(),
            };
        }
        let building = matches!(self.phase, Phase::Intro | Phase::Play);
        // The hypnotist's game keeps its vial where it was when she woke or bit.
        let kept = hipno
            && matches!(self.phase, Phase::Relax(_))
            && self.hipno.end != Some(hipno::End::Fall);
        map_overlay::SceneView {
            heat: Some(if building || kept {
                (self.heat * 1000.0) as u16
            } else {
                1000
            }),
            phase,
            holding: self.holding && (noa || hipno || tienda),
            hold: None,
            braked: false,
            marks: if hipno && building && !self.hipno.mouth() {
                HipnoGame::marks()
            } else if turno && building {
                self.turno_marks()
            } else if noa && building {
                self.noa_poses()
                    .iter()
                    .skip(1)
                    .map(|p| (p.open_at * 1000.0) as u16)
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            },
            lines: self.talk.cards(now),
            caption: self.talk.caption.as_ref().map(|(text, _)| text.clone()),
            keys: building,
            esc: if building { "parar" } else { "saltar" }.to_owned(),
            result: None,
            frame: self.reel.uploads,
            flash: (self.flash_now() * 1000.0) as u16,
            glow: (self.held_glow.clamp(0.0, 1.0) * 1000.0) as u16,
            prompt: None,
            poses: Vec::new(),
            pose: 0,
            pose_active: false,
            black: self.reel.black,
            game: Some(game),
            deal: None,
        }
    }

    /// The closing card: how it went, what changed, the last word.
    fn result_view(
        &self,
        done: bool,
        elapsed: f32,
        mut game: map_overlay::GameView,
    ) -> map_overlay::SceneView {
        let (title, effects, quote, who) = self.ending(done);
        game.keys.clear();
        let result = map_overlay::SceneResult {
            title: title.to_owned(),
            who,
            complete: done,
            effects: effects
                .iter()
                .map(|(var, by)| (label(var).to_owned(), *by))
                .collect(),
            seconds: (elapsed - self.started_at).max(0.0).round() as u32,
            quote: card_line(quote.0, quote.1, 255, 0),
            age_ms: ((self.clock - self.ended_at).max(0.0) * 1000.0) as u32,
        };
        // The hypnotist's game shows where the trance was when she woke or bit; only a fall fills it.
        let full = done && (self.game != Game::Hipno || self.hipno.end == Some(hipno::End::Fall));
        map_overlay::SceneView {
            heat: Some(if full {
                1000
            } else {
                (self.heat * 1000.0) as u16
            }),
            result: Some(result),
            frame: self.reel.uploads,
            black: self.reel.black,
            game: Some(game),
            ..Default::default()
        }
    }

    /// How it ended: the card's title, what it does to the story, the last word, and the line of
    /// how it went.
    fn ending(
        &self,
        done: bool,
    ) -> (
        &'static str,
        Vec<(&'static str, i32)>,
        (Voice, &'static str),
        String,
    ) {
        match self.game {
            Game::Hipno => self.hipno_ending(done),
            Game::Tienda(_) => self.tienda_ending(done),
            Game::Turno(_) => self.turno_ending(done),
            Game::Alba | Game::AlbaFuta => {
                let a = &self.alba;
                let futa = self.game == Game::AlbaFuta;
                let how = format!("Alba · {} aciertos · racha {}", a.hits, a.best);
                if !done {
                    return (
                        "Te levantas antes de tiempo",
                        vec![("hunger", -1), ("info", 1)],
                        (
                            Voice::Alba,
                            "—¿Ya? ¡Pero si no te he contado lo del molinero!",
                        ),
                        how,
                    );
                }
                let mut effects = vec![
                    ("hunger", if futa { -7 } else { -5 }),
                    ("deseo", if futa { 3 } else { 2 }),
                    ("info", if a.perfect { 2 } else { 1 }),
                    ("alba_trust", 1),
                ];
                if a.best >= 5 {
                    effects.push(("chiquita_afecto", -1));
                }
                let quote = if futa {
                    (
                        Voice::Chiquita,
                        "—Mañana lo sabe toda la villa. Con medidas.",
                    )
                } else {
                    (
                        Voice::Chiquita,
                        "—Te ha contado tres rumores y medio. El medio era sobre ti.",
                    )
                };
                let title = if a.perfect {
                    "¡Final perfecto!"
                } else {
                    "Alba, sin palabras (por fin)"
                };
                (title, effects, quote, how)
            }
            Game::Noa | Game::NoaFuta => {
                let n = &self.noa;
                let futa = self.game == Game::NoaFuta;
                let how = format!(
                    "Noa · llamadas a la pared {} · toses {}",
                    n.knocks, n.coughs
                );
                if !done {
                    return (
                        "Te detienes antes de tiempo",
                        vec![("hunger", -2)],
                        (Voice::Noa, "—…¿Ya? …Vale. Sí. Mejor así."),
                        how,
                    );
                }
                let mut effects = vec![
                    ("hunger", if futa { -7 } else { -4 }),
                    ("deseo", if futa { 3 } else { 2 }),
                    ("noa_trust", if n.knocks == 0 { 2 } else { 1 }),
                ];
                if n.knocks >= 2 {
                    effects.push(("dignity", -1));
                }
                let (title, quote) = match n.knocks {
                    0 if futa => ("¡Ni un ruido!", (Voice::Noa, "—…Vuelve. Otra noche. …Con la correa.")),
                    0 => ("¡Ni un ruido!", (Voice::Noa, "—…Vuelve. Sin el frasco también. …Sobre todo sin el frasco.")),
                    1 => ("Casi nadie se ha enterado", (Voice::Chiquita, "—Una llamada a la pared. Para ser tú, es discreción.")),
                    _ => (
                        "Todo el burdel se ha enterado",
                        (Voice::Chiquita, "—Rosalía ha dejado una nota bajo la puerta: «Las paredes son de papel, guapas».",),
                    ),
                };
                (title, effects, quote, how)
            }
        }
    }

    /// What she thinks once the card closes.
    fn thought(&self) -> &'static str {
        if self.game == Game::Hipno {
            return self.hipno_thought();
        }
        if matches!(self.game, Game::Tienda(_)) {
            return self.tienda_thought();
        }
        if matches!(self.game, Game::Turno(_)) {
            return self.turno_thought();
        }
        match (self.game, self.ended.unwrap_or(false)) {
            (Game::NoaFuta, true) if self.noa.knocks == 0 => {
                "Ni un ruido. Y aun así, el sello lo ha oído todo."
            }
            (Game::NoaFuta | Game::Noa, true) if self.noa.knocks > 0 => {
                "Mañana la miraré a los ojos en el pasillo. O no."
            }
            (Game::Noa, true) => "Sin elixir. Solo ella, las velas y una pared muy fina.",
            (Game::NoaFuta | Game::Noa, _) => "Despacio. Con Noa todo es despacio.",
            (_, true) => {
                "Tres rumores, una copla y ni un segundo de silencio. Vuelvo la semana que viene."
            }
            (_, false) => "Otro día. Con tapones.",
        }
    }
}

impl App {
    /// Opens a girl's minigame.
    pub(super) fn open_play_scene(&mut self, game: Game) {
        let Some(mut scene) = PlayScene::open(game, self.elapsed) else {
            return;
        };
        if game == Game::Hipno {
            // What she carries from his own stall.
            let salts = self.inventory.get(SALTS).copied().unwrap_or(0);
            let amulet = self.inventory.get(AMULET).is_some_and(|n| *n > 0);
            scene.hipno_items(salts, amulet);
        }
        eprintln!("[juego] {game:?}");
        self.play_scene = Some(scene);
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
    }

    /// Each frame while it plays: the game steps, its sound follows the shot, Chiquita and the
    /// sounds it asks for come out.
    pub(super) fn tick_play_scene(&mut self, dt: f32) {
        let pointer = self.shop_pointer_position();
        let Some(scene) = self.play_scene.as_mut() else {
            return;
        };
        let mut cover = None;
        if scene.game == Game::Hipno {
            // The pointer over the picture pushes her gaze; Chiquita keeps in front of the ring.
            let [px, py, pw, ph] =
                map_overlay::scene_picture_rect(WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
            let over = pointer
                .filter(|(x, y)| (px..=px + pw).contains(x) && (py..=py + ph).contains(y))
                .map(|(x, _)| (x - px) / pw);
            let moved = pointer != scene.last_pointer;
            scene.last_pointer = pointer;
            let now = scene.clock;
            scene.hipno.point(over, moved, now);
            if scene.hipno.covered(now) {
                let (_, ring) = scene.hipno.ring();
                cover = Some([px + ring[0] * pw, py + ring[1] * ph]);
            }
        }
        scene.step(dt);
        if matches!(scene.game, Game::Tienda(_)) {
            scene.tienda_face();
        }
        let want = scene.ended.is_none().then_some(scene.reel.on);
        let mut sound = None;
        if want != scene.sound_for {
            scene.sound_for = want;
            if let Some(i) = want {
                super::gallery_seen::shot(&scene.reel.shots[i].dir);
            }
            sound = Some(want.map(|i| {
                (
                    scene.reel.shots[i].dir.join("audio.ogg"),
                    scene.reel.shots[i].looping,
                )
            }));
        }
        let now = scene.clock;
        let mut due = Vec::new();
        scene.sfx.retain(|(sfx, at, volume)| {
            if now >= *at {
                due.push((*sfx, *volume));
                false
            } else {
                true
            }
        });
        let floats = std::mem::take(&mut scene.floats);
        if let (Some(sound), Some(audio)) = (sound, self.audio.as_mut()) {
            match sound {
                Some((path, looped)) => audio.scene_sound(Some(&path), looped),
                None => audio.scene_sound(None, false),
            }
        }
        for (sfx, volume) in due {
            self.ui_sound(sfx, volume);
        }
        for (text, cue) in floats {
            self.float_chiquita_cue(&text, cue);
        }
        if let Some(at) = cover {
            self.mascot_cover_track(at);
        }
        self.clear_input();
    }

    /// Keys while it plays: every key is the game's.
    pub(super) fn play_scene_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        let Some(scene) = self.play_scene.as_mut() else {
            return;
        };
        if scene.ended.is_some() {
            if pressed
                && !repeat
                && scene.card_ready()
                && matches!(
                    code,
                    KeyCode::KeyF
                        | KeyCode::Space
                        | KeyCode::Enter
                        | KeyCode::NumpadEnter
                        | KeyCode::Escape
                        | KeyCode::Backspace
                )
            {
                self.close_play_scene();
            }
            return;
        }
        let fresh = pressed && !repeat;
        // A shift: Space on her beat, Shift holds back, V the other view, 1 · 2 where.
        if matches!(scene.game, Game::Turno(_)) {
            match code {
                KeyCode::Space if fresh => scene.turno_beat(),
                KeyCode::ShiftLeft | KeyCode::ShiftRight => scene.turno.hold = pressed,
                KeyCode::KeyV if fresh => scene.turno_flip_view(),
                KeyCode::Digit1 | KeyCode::Numpad1 if fresh => scene.pick(0),
                KeyCode::Digit2 | KeyCode::Numpad2 if fresh => scene.pick(1),
                KeyCode::Escape | KeyCode::Backspace if fresh => scene.stop(),
                _ => {}
            }
            return;
        }
        // A favour: Space hurries it, Esc backs out.
        if matches!(scene.game, Game::Tienda(_)) {
            match code {
                KeyCode::Space => scene.set_holding(pressed),
                KeyCode::Escape | KeyCode::Backspace if fresh => scene.stop(),
                _ => {}
            }
            return;
        }
        if scene.game == Game::Hipno {
            match code {
                KeyCode::Space => scene.set_holding(pressed),
                KeyCode::Escape | KeyCode::Backspace if fresh => scene.stop(),
                KeyCode::ArrowLeft | KeyCode::KeyA => {
                    scene.hipno.left = pressed;
                    if fresh && scene.hipno.mouth() {
                        scene.hipno_pose(0);
                    }
                }
                KeyCode::ArrowRight | KeyCode::KeyD => {
                    scene.hipno.right = pressed;
                    if fresh && scene.hipno.mouth() {
                        scene.hipno_pose(1);
                    }
                }
                KeyCode::Digit1 | KeyCode::Numpad1 if fresh => scene.pick(0),
                KeyCode::Digit2 | KeyCode::Numpad2 if fresh => scene.pick(1),
                KeyCode::Digit3 | KeyCode::Numpad3 if fresh => scene.pick(2),
                KeyCode::KeyE if fresh => scene.hipno_mash(),
                KeyCode::KeyQ if fresh => scene.hipno_salts(),
                _ => {}
            }
            return;
        }
        match code {
            KeyCode::Space => scene.set_holding(pressed),
            KeyCode::Escape | KeyCode::Backspace if fresh => scene.stop(),
            KeyCode::Digit1 | KeyCode::Numpad1 if fresh => scene.pick(0),
            KeyCode::Digit2 | KeyCode::Numpad2 if fresh => scene.pick(1),
            KeyCode::Digit3 | KeyCode::Numpad3 if fresh => scene.pick(2),
            KeyCode::KeyT if fresh => scene.leash(),
            KeyCode::ArrowLeft | KeyCode::KeyA if fresh => {
                let n = scene.noa.open.max(1);
                scene.turn_pose((scene.noa.pose + n - 1) % n);
            }
            KeyCode::ArrowRight | KeyCode::KeyD if fresh => {
                let n = scene.noa.open.max(1);
                scene.turn_pose((scene.noa.pose + 1) % n);
            }
            _ => {}
        }
    }

    /// The left button: a tile, a posture or its arrows when it is on one, else held like Space; or
    /// closes the card.
    pub(super) fn play_scene_button(&mut self, pressed: bool) {
        let pointer = self.shop_pointer_position();
        let Some(scene) = self.play_scene.as_mut() else {
            return;
        };
        if scene.ended.is_some() {
            if pressed && scene.card_ready() {
                self.close_play_scene();
            }
            return;
        }
        // A favour's page: «Echarse atrás» (once sealed, «Saltar») is Esc.
        if pressed && matches!(scene.game, Game::Tienda(_)) {
            if let Some((x, y)) = pointer {
                if map_overlay::scene_favor_hit(x, y, false) == Some(1) {
                    scene.stop();
                    return;
                }
            }
        }
        if pressed {
            if let Some((x, y)) = pointer {
                let (w, h) = (WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
                let tiles = scene.chips().len();
                if tiles > 0 {
                    if let Some(i) = map_overlay::game_chip_hit(w, h, tiles, x, y) {
                        scene.pick(i);
                        return;
                    }
                }
                if matches!(scene.game, Game::Turno(_)) {
                    scene.turno_beat();
                    return;
                }
                if scene.game == Game::Hipno && scene.hipno.mouth() {
                    match map_overlay::game_pose_hit(w, h, 2, x, y) {
                        Some(Ok(by)) => {
                            scene.hipno_pose((scene.hipno.pose as i32 + by).rem_euclid(2) as usize);
                            return;
                        }
                        Some(Err(cell)) => {
                            scene.hipno_pose(cell);
                            return;
                        }
                        None => {}
                    }
                }
                if scene.is_noa() {
                    match map_overlay::game_pose_hit(w, h, scene.noa_poses().len(), x, y) {
                        Some(Ok(by)) => {
                            let n = scene.noa.open.max(1);
                            scene.turn_pose(
                                (scene.noa.pose as i32 + by).rem_euclid(n as i32) as usize
                            );
                            return;
                        }
                        Some(Err(cell)) => {
                            scene.turn_pose(cell);
                            return;
                        }
                        None => {}
                    }
                }
            }
        }
        if matches!(scene.game, Game::Turno(_)) {
            return;
        }
        scene.set_holding(pressed);
    }

    /// The pause menu or a lost focus: whatever is held lets go; a game still going ends.
    pub(super) fn play_scene_interrupt(&mut self) {
        if let Some(scene) = self.play_scene.as_mut() {
            scene.set_holding(false);
            scene.turno.hold = false;
            if scene.ended.is_none() {
                scene.stop();
            }
        }
    }

    /// The climax is behind black right now (Chiquita guards it).
    pub(super) fn play_scene_black(&self) -> bool {
        self.play_scene
            .as_ref()
            .is_some_and(|s| s.ended.is_none() && matches!(s.phase, Phase::Cut(_)))
    }

    /// Shuts the card: the effects land, she thinks one last thing, the game saves.
    fn close_play_scene(&mut self) {
        let Some(scene) = self.play_scene.take() else {
            return;
        };
        let done = scene.ended.unwrap_or(false);
        let (_, effects, _, _) = scene.ending(done);
        for (var, by) in effects {
            *self.story.values.entry(var.to_owned()).or_insert(0) += by;
        }
        // The ero record (the pause menu's status page).
        match scene.game {
            Game::Alba => super::ero::encounter(&mut self.story.values, "alba", false, done),
            Game::AlbaFuta => super::ero::encounter(&mut self.story.values, "alba", true, done),
            Game::Noa => super::ero::encounter(&mut self.story.values, "noa", false, done),
            Game::NoaFuta => super::ero::encounter(&mut self.story.values, "noa", true, done),
            Game::Hipno => {
                if done && scene.hipno.end == Some(hipno::End::Fall) {
                    super::ero::tally(&mut self.story.values, super::ero::HIPNOTIZADA);
                }
            }
            Game::Tienda(_) => {
                if done {
                    super::ero::encounter(&mut self.story.values, "tendero", false, false);
                }
            }
            Game::Turno(_) => {
                super::ero::encounter(&mut self.story.values, scene.client().id, false, done);
            }
        }
        // A shift: the tip comes off the debt; the first time it goes all the way, it is the first.
        if let Game::Turno(client) = scene.game {
            self.after_turno(
                client as usize,
                done && !scene.turno.early,
                scene.turno_tip(),
            );
        }
        // A favour: the ladder climbs, and back in the shop the haggle takes its cut.
        if let Game::Tienda(favor) = scene.game {
            self.after_favor(favor as usize, done);
        }
        if scene.game == Game::Hipno {
            // The salts sniffed are gone; the hypnotist remembers how it went.
            let used = scene.hipno.salts_used;
            if used > 0 {
                let left = self
                    .inventory
                    .get(SALTS)
                    .copied()
                    .unwrap_or(0)
                    .saturating_sub(used);
                if left == 0 {
                    self.inventory.remove(SALTS);
                } else {
                    self.inventory.insert(SALTS.to_owned(), left);
                }
                let mut pack = self.backpack.clone();
                pack.reconcile(&self.inventory);
                self.backpack = pack;
            }
            let last = match (done, scene.hipno.end) {
                (true, Some(hipno::End::Escape)) => 1,
                (true, Some(hipno::End::Bite)) => 2,
                (true, Some(hipno::End::Fall)) => 3,
                _ => 4,
            };
            self.story.values.insert("mago_ultima".to_owned(), last);
            *self
                .story
                .values
                .entry("mago_sesiones".to_owned())
                .or_insert(0) += 1;
            if last == 3 && scene.hipno_on_purpose() {
                *self
                    .story
                    .values
                    .entry("mago_a_proposito".to_owned())
                    .or_insert(0) += 1;
            }
        }
        if let Some(audio) = &mut self.audio {
            audio.scene_sound(None, false);
        }
        self.queen_thought = Some((scene.thought(), self.elapsed + 4.5));
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// A favour of the shop plays as a page of the shop: the haggle under it (what it buys, the
    /// price as it stands and once the favour is paid), the favour, her gold, and the
    /// shopkeeper's face of the moment (grinning while she pays, swamped while she hurries him,
    /// shaking his head if she backs out).
    fn favor_deal(&self) -> Option<map_overlay::FavorDeal> {
        let scene = self.play_scene.as_ref()?;
        let Game::Tienda(favor) = scene.game else {
            return None;
        };
        let now = scene.clock;
        let (face, before, changed) = scene.tienda.face;
        let said_ms = scene
            .talk
            .said
            .iter()
            .rev()
            .find(|l| matches!(l.voice, Voice::Tendero))
            .map(|l| ((now - l.at).max(0.0) * 1000.0) as u32);
        let index = (favor as usize).min(tienda::FAVORS.len() - 1);
        let f = &tienda::FAVORS[index];
        let (_, fresh) = tienda::available(&self.story);
        let art = super::shop_games::merchant_art();
        let portrait = |name: &'static str| {
            art.map(|crop| {
                super::shop_games::merchant_face(
                    if name.is_empty() { "contento" } else { name },
                    crop,
                )
            })
        };
        let mut deal = map_overlay::FavorDeal {
            quantity: 1,
            favor: f.name.to_owned(),
            cut: f.percent,
            step: index as u8 + 1,
            steps: tienda::FAVORS.len() as u8,
            new: fresh == Some(index),
            gold: self.gold,
            face: portrait(face),
            face_from: portrait(before),
            face_ms: ((now - changed).max(0.0) * 1000.0) as u32,
            said_ms,
            open_ms: (now.max(0.0) * 1000.0) as u32,
            ..Default::default()
        };
        if let Some(h) = self.haggle.as_ref() {
            deal.product = Some(h.product);
            deal.quantity = h.quantity;
            deal.base = h.base;
            deal.price = h.price();
            deal.after = h.price_at((h.percent + f.percent).min(100));
            deal.percent = h.percent;
        }
        Some(deal)
    }

    /// The stage for the overlay, and the frame waiting to go up to the GPU.
    pub(super) fn play_scene_view(
        &mut self,
    ) -> Option<(map_overlay::SceneView, Option<image::RgbaImage>)> {
        let elapsed = self.elapsed;
        let deal = self.favor_deal();
        let scene = self.play_scene.as_mut()?;
        let upload = scene.reel.upload.take();
        if upload.is_some() {
            scene.reel.uploads = scene.reel.uploads.wrapping_add(1);
        }
        let mut view = scene.view(elapsed);
        view.deal = deal;
        Some((view, upload))
    }

    /// Visual QA: waits for the frame being decoded, so a capture shows the clock's frame.
    pub(super) fn play_scene_settle(&mut self) {
        let Some(scene) = self.play_scene.as_mut() else {
            return;
        };
        if scene.reel.decoder.busy {
            if let Ok((id, picture)) = scene
                .reel
                .decoder
                .done
                .recv_timeout(std::time::Duration::from_secs(3))
            {
                scene.reel.decoder.busy = false;
                scene.reel.shown = Some(id);
                scene.reel.upload = Some(picture);
            }
        }
    }

    /// Visual QA: the game's state, for a capture script to play it.
    pub(super) fn play_scene_qa(&self) -> Option<PlayQa> {
        let s = self.play_scene.as_ref()?;
        Some(PlayQa {
            phase: match s.phase {
                Phase::Intro => "intro",
                Phase::Play => "play",
                Phase::Cut(_) => "cut",
                Phase::Climax => "climax",
                Phase::Verse => "verse",
                Phase::Relax(_) => "relax",
            },
            ended: s.ended.is_some(),
            heat: s.heat,
            streak: s.alba.streak,
            right: s
                .alba
                .round
                .as_ref()
                .filter(|r| r.picked.is_none())
                .and_then(|r| r.words.iter().position(|w| *w == s.wants()[r.want].word))
                .or_else(|| {
                    let verse = s.alba.verse.as_ref().filter(|v| v.picked.is_none())?;
                    let copla = s.copla()?;
                    verse.words.iter().position(|w| *w == copla.rhyme)
                }),
            noise: s.noa.noise,
            air: s.noa.air,
            open: s.noa.open,
            pose: s.noa.pose,
            listening: s.noa.listening(s.clock),
            knocks: s.noa.knocks,
            stage: s.hipno.stage,
            trance: s.hipno.trance,
            awake: s.hipno.awake,
            order: s.hipno.order.is_some(),
            clip: s.reel.name(),
            gaze: s.hipno.gaze,
            ring: s.hipno.ring().1[0],
            mouth: s.hipno.mouth(),
            beat_in: s.turno_beat_in(),
            stamina: s.turno.stamina,
            choosing: s.turno.choosing_since().is_some(),
        })
    }

    /// Visual QA: one of Chiquita's numbers in the hypnotist's game, now (`cover` or under the
    /// ring's spell herself).
    pub(super) fn play_scene_force_gag(&mut self, cover: bool) {
        if let Some(scene) = self.play_scene.as_mut() {
            scene.hipno_force_gag(cover);
        }
    }
}

/// What the hypnotist sells that his own game knows (`mage.rs`): the smelling salts and the amulet.
pub(super) const SALTS: &str = "sales_despertadoras";
pub(super) const AMULET: &str = "amuleto_ojo_cerrado";

/// What a capture script needs to know to play a game.
#[derive(Clone, Copy, Debug)]
pub(super) struct PlayQa {
    pub(super) phase: &'static str,
    pub(super) ended: bool,
    pub(super) heat: f32,
    pub(super) streak: u32,
    /// The right tile now, if a pick is open.
    pub(super) right: Option<usize>,
    pub(super) noise: f32,
    pub(super) air: f32,
    pub(super) open: usize,
    pub(super) pose: usize,
    pub(super) listening: bool,
    pub(super) knocks: u32,
    /// The hypnotist's game: the stage, trance, waking, an order open, the clip on show.
    pub(super) stage: usize,
    pub(super) trance: f32,
    pub(super) awake: f32,
    pub(super) order: bool,
    pub(super) clip: &'static str,
    /// Her gaze and the ring across the picture (0..1); her mouth is busy.
    pub(super) gaze: f32,
    pub(super) ring: f32,
    pub(super) mouth: bool,
    /// A shift: seconds to her next beat (`f32::INFINITY` when none), his stamina, picking where.
    pub(super) beat_in: f32,
    pub(super) stamina: f32,
    pub(super) choosing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn her_shout_gets_harder_to_read_the_hotter_she_is() {
        let mut dice = Dice(12345);
        let want = &WANTS[0];
        assert_eq!(
            garble(want, 0.1, &mut dice),
            ("¡DESPACIO!".to_owned(), false)
        );
        let (stammer, wild) = garble(want, 0.5, &mut dice);
        assert!(stammer.contains("DE…") && !wild, "{stammer}");
        let (heap, wild) = garble(want, 0.9, &mut dice);
        assert!(wild && heap != "¡DESPACIO!", "{heap}");
        let mut letters: Vec<char> = heap
            .trim_matches(|c| c == '¡' || c == '!')
            .chars()
            .collect();
        letters.sort_unstable();
        let mut word: Vec<char> = "DESPACIO".chars().collect();
        word.sort_unstable();
        assert_eq!(letters, word, "the same letters, mixed");
    }

    #[test]
    fn a_wrong_word_is_said_back_as_she_would() {
        assert_eq!(said("CEBOLLAS"), "Cebollas");
        assert_eq!(said("EL MOLINERO"), "El molinero");
        assert_eq!(said("ROSALÍA"), "Rosalía");
    }

    #[test]
    fn her_lines_wait_until_the_one_on_show_has_been_read() {
        let mut talk = Talk::default();
        talk.say(
            Voice::Alba,
            "—Una frase bastante larga para leerla con calma, de verdad.",
        );
        talk.say(Voice::Alba, "—La siguiente.");
        talk.tick(0.0);
        assert_eq!(talk.said.len(), 1);
        talk.tick(1.0);
        assert_eq!(talk.said.len(), 1, "too soon for the next");
        talk.tick(10.0);
        assert_eq!(talk.said.len(), 2);
        // A reply comes a moment after the card on show, dropping what waited.
        talk.say(Voice::Alba, "—Esperando.");
        talk.reply(Voice::Alba, "—¡Respuesta!");
        talk.tick(10.5);
        assert_eq!(talk.said.len(), 2);
        talk.tick(11.0);
        assert_eq!(
            talk.said.last().map(|l| l.text.as_str()),
            Some("—¡Respuesta!")
        );
        assert!(talk.queue.is_empty());
        // Narration is a subtitle, up as long as it takes to read.
        talk.say(Voice::Narration, "Algo pasa.");
        talk.tick(12.0);
        assert!(talk
            .caption
            .as_ref()
            .is_some_and(|(_, until)| *until > 14.0));
        talk.tick(30.0);
        assert!(talk.caption.is_none());
    }

    #[test]
    fn the_wall_takes_less_noise_while_the_next_room_listens() {
        let mut noa = NoaGame {
            listen: Some((10.0, 16.0)),
            ..Default::default()
        };
        assert_eq!(noa.limit(5.0), LIMIT);
        assert_eq!(noa.neighbour(5.0), 1, "stirring before it listens");
        assert_eq!(noa.limit(12.0), LIMIT_LISTENING);
        assert_eq!(noa.neighbour(12.0), 2);
        assert_eq!(noa.limit(17.0), LIMIT);
        noa.knock_at = 20.0;
        assert_eq!(noa.neighbour(20.5), 3);
    }
}
