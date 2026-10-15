//! Las escenas íntimas como minijuego, en un panel en el centro de la pantalla mientras el mundo
//! sigue detrás. Sus tomas (`data/assets/<carpeta>/{intro,bucle,bucle_rapido,climax,relax}`): la
//! entrada (una vez) → el bucle (mientras se mantiene Espacio o el clic la barra de placer sube y
//! el ritmo se acelera; si se suelta, baja; pasada la mitad, el bucle intenso si la escena lo
//! tiene) → clímax → calma. Cada cambio de toma funde la imagen saliente bajo la nueva. Esc lo deja
//! a medias. Lo que cambia de una a otra (frases, finales, efectos, si te frenan) está en su
//! [`Spec`]. Además, para jugarla: ella pregunta y la Reina responde (1–3, con un tiempo para
//! hacerlo), el bucle intenso puede tener posturas que se cambian con ←/→, el clímax puede oírse
//! sobre negro, y lo que dice Chiquita flota en la pantalla en vez de ir a las tarjetas.
use std::sync::mpsc;

use super::*;

/// Frames per second of the clips (LTX renders at 24).
const FPS: f32 = 24.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Intro,
    /// The loop the player holds: unhurried.
    Loop,
    /// The same loop, eager (past half the bar; optional).
    LoopFast,
    Climax,
    Relax,
}

impl Part {
    const ALL: [Part; 5] = [
        Part::Intro,
        Part::Loop,
        Part::LoopFast,
        Part::Climax,
        Part::Relax,
    ];

    fn dir(self) -> &'static str {
        match self {
            Part::Intro => "intro",
            Part::Loop => "bucle",
            Part::LoopFast => "bucle_rapido",
            Part::Climax => "climax",
            Part::Relax => "relax",
        }
    }

    fn index(self) -> usize {
        Part::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    fn looping(self) -> bool {
        matches!(self, Part::Loop | Part::LoopFast)
    }

    /// Its line under the picture (the two loops share one).
    fn phase_slot(self) -> usize {
        match self {
            Part::Intro => 0,
            Part::Loop | Part::LoopFast => 1,
            Part::Climax => 2,
            Part::Relax => 3,
        }
    }
}

/// The picture going out under the new one when the scene changes clips.
#[derive(Clone, Copy, Debug)]
struct Fade {
    part: Part,
    /// The posture it was in (the eager loop's).
    pose: usize,
    time: f32,
    rate: f32,
    start: f32,
    length: f32,
}

/// The decoder's job: a frame, and maybe the frame fading out over it with its weight.
pub(super) type Job = (u64, PathBuf, Option<(PathBuf, f32)>);

/// Who says a line of the scene (and with what face).
#[derive(Clone, Copy)]
pub(super) enum Voice {
    /// The Queen thinking (her portrait, the thought's parentheses).
    Thought,
    Chiquita,
    /// Chiquita jealous and flustered (her arms crossed).
    ChiquitaJealous,
    /// Chiquita turning her back on the scene ("me voy a mirar la pared"), peeking.
    ChiquitaAway,
    /// Chiquita with her hands on her cheeks.
    ChiquitaFlustered,
    /// Chiquita flying at the screen and splatting against the glass.
    ChiquitaSplat,
    /// Chiquita wagging a finger at the screen.
    ChiquitaScold,
    Nina,
    /// Nina sly, a finger on her lip.
    NinaSly,
    /// The brothel's other girls, in their minigames (`play_scene.rs`).
    Alba,
    Noa,
    /// The Queen out loud (the hypnotist's game): as she is, angry, or in a trance.
    Queen,
    QueenAngry,
    QueenTrance,
    /// The hypnotist of the wood.
    Mago,
    /// The village's shopkeeper (the shop's favours).
    Tendero,
    Narration,
    /// A sound, between dashes.
    Sound,
}

impl Voice {
    /// Its name, colour, face (who and mood in `retratos.json`) and whether it is a thought.
    pub(super) fn card(
        self,
    ) -> (
        &'static str,
        [u8; 3],
        Option<(&'static str, Option<&'static str>)>,
        bool,
    ) {
        match self {
            Voice::Thought => (
                "Reina",
                [0xff, 0x6e, 0xa0],
                Some(("gorou", Some("piensa"))),
                true,
            ),
            Voice::Chiquita => (
                "Chiquita",
                [0xd9, 0xa6, 0xff],
                Some(("chiquita", None)),
                false,
            ),
            Voice::ChiquitaJealous
            | Voice::ChiquitaAway
            | Voice::ChiquitaFlustered
            | Voice::ChiquitaSplat
            | Voice::ChiquitaScold => (
                "Chiquita",
                [0xd9, 0xa6, 0xff],
                Some(("chiquita", Some("celosa"))),
                false,
            ),
            Voice::Nina => ("Nina", [0xe8, 0x84, 0x5c], Some(("nina", None)), false),
            Voice::NinaSly => (
                "Nina",
                [0xe8, 0x84, 0x5c],
                Some(("nina", Some("picara"))),
                false,
            ),
            Voice::Alba => ("Alba", [0xf0, 0xc0, 0x60], Some(("alba", None)), false),
            Voice::Noa => ("Noa", [0xc4, 0x3a, 0x58], Some(("noa", None)), false),
            Voice::Queen => ("Reina", [0xff, 0x6e, 0xa0], Some(("gorou", None)), false),
            Voice::QueenAngry => (
                "Reina",
                [0xff, 0x6e, 0xa0],
                Some(("gorou", Some("enfadada"))),
                false,
            ),
            Voice::QueenTrance => (
                "Reina",
                [0xc8, 0x8a, 0xff],
                Some(("gorou", Some("trance"))),
                false,
            ),
            Voice::Mago => ("Mago", [0xd6, 0x5c, 0xff], Some(("hypnotist", None)), false),
            Voice::Tendero => (
                "Tendero",
                [0x4a, 0xd0, 0x9a],
                Some(("tendero", None)),
                false,
            ),
            Voice::Narration | Voice::Sound => ("", [0xd9, 0xbd, 0x8a], None, false),
        }
    }

    pub(super) fn spoken(self) -> bool {
        !matches!(self, Voice::Narration | Voice::Sound)
    }

    /// Chiquita: her lines float on the screen instead of joining the cards, and say what she does.
    pub(super) fn floats(self) -> Option<mascot::Cue> {
        match self {
            Voice::Chiquita => Some(mascot::Cue::Talk),
            Voice::ChiquitaJealous => Some(mascot::Cue::Jealous),
            Voice::ChiquitaAway => Some(mascot::Cue::Away),
            Voice::ChiquitaFlustered => Some(mascot::Cue::Flustered),
            Voice::ChiquitaSplat => Some(mascot::Cue::Splat),
            Voice::ChiquitaScold => Some(mascot::Cue::Scold),
            _ => None,
        }
    }
}

/// A posture of the eager loop: its clip's folder (under the scene's), its name, her word when it
/// starts.
pub(super) struct Pose {
    dir: &'static str,
    label: &'static str,
    line: (Voice, &'static str),
}

/// A moment she asks and the Queen answers: when it comes (the part and how far into it), the
/// question, and three answers (the last is what silence says when time runs out).
pub(super) struct Prompt {
    at: (Part, f32),
    ask: (Voice, &'static str),
    answers: [Answer; 3],
}

/// An answer: its words, her reply, what it does to the pleasure and to the story.
pub(super) struct Answer {
    label: &'static str,
    reply: (Voice, &'static str),
    heat: f32,
    effects: &'static [(&'static str, i32)],
}

/// How long an answer waits (s).
const ASK_SECS: f32 = 7.0;

/// A line of the scene on its card.
pub(super) fn card_line(voice: Voice, text: &str, alpha: u8, age: u8) -> map_overlay::SceneLine {
    let (name, accent, face, thought) = voice.card();
    map_overlay::SceneLine {
        name: name.to_owned(),
        text: text.to_owned(),
        accent,
        thought,
        portrait: face
            .and_then(|(who, mood)| guion::portraits().find(who, mood))
            .map(|p| map_overlay::PortraitRef {
                image: p.image,
                crop: p.crop,
            }),
        alpha,
        age,
    }
}

/// What the story's variables are called on the closing card.
pub(super) fn label(var: &str) -> &str {
    match var {
        "alba_trust" => "Confianza de Alba",
        "noa_trust" => "Confianza de Noa",
        "info" => "Rumores",
        "deseo" => "Deseo",
        "hunger" => "Hambre del sello",
        "dignity" => "Dignidad",
        "devotion" => "Devoción",
        "fear" => "Miedo",
        "nina_trust" => "Confianza de Nina",
        "chiquita_afecto" => "Cariño de Chiquita",
        "mago_derrotas" => "Derrotas del mago",
        "mago_mordiscos" => "Mordiscos al mago",
        "mago_caidas" => "Veces que caíste",
        other => other,
    }
}

/// How a scene ends: its card and what it does to the story.
pub(super) struct Ending {
    title: &'static str,
    last: (Voice, &'static str),
    effects: &'static [(&'static str, i32)],
    /// What she thinks once the card closes.
    thought: &'static str,
}

/// One intimate scene.
pub(super) struct Spec {
    pub(super) name: &'static str,
    /// The clips' folder under `data/assets`.
    dir: &'static str,
    /// What is happening in each part (over the key).
    phases: [&'static str; 4],
    /// What is said as the pleasure climbs (threshold, voice, line).
    lines: &'static [(f32, Voice, &'static str)],
    climax: (Voice, &'static str),
    relax: (Voice, &'static str),
    done: Ending,
    half: Ending,
    /// Held longer than this many seconds at a stretch, she stops you: the pleasure drops by the
    /// second number and the button must be let go before it counts again.
    brake: Option<(f32, f32, Voice, &'static str)>,
    /// When in the climax clip the burst comes and peaks (s): the room's light flares with it.
    burst: Option<(f32, f32)>,
    /// How long the two loops cross-fade (s): longer when they are different shots.
    cross: f32,
    /// Postures of the eager loop (←/→ change them); empty: its one `bucle_rapido`.
    poses: &'static [Pose],
    /// What she asks along the way.
    prompts: &'static [Prompt],
    /// The climax heard over black for this long (s), its clip not shown.
    black_climax: Option<f32>,
    /// Chiquita steps in front of the black and punches the screen, saying this.
    smash: Option<&'static str>,
}

/// «Explorar el cuerpo» in the castle bed.
pub(super) const BED: Spec = Spec {
    name: "la cama",
    dir: "castillo/escena_cama",
    phases: [
        "Se tumba y abre las piernas…",
        "Tocarse",
        "¡El sello estalla!",
        "Calma",
    ],
    lines: &[
        (0.0, Voice::Thought, "Es mi cuerpo. Técnicamente."),
        (0.15, Voice::ChiquitaAway, "—Yo me voy a mirar la pared."),
        (0.35, Voice::Thought, "No... no pares ahora."),
        (
            0.55,
            Voice::ChiquitaAway,
            "—…La pared está muy interesante hoy.",
        ),
        (
            0.75,
            Voice::Narration,
            "El sello late cada vez más deprisa.",
        ),
        (
            0.90,
            Voice::ChiquitaFlustered,
            "—Vale. Eso lo he sentido yo también.",
        ),
    ],
    climax: (Voice::Sound, "las runas se encienden de golpe"),
    relax: (Voice::Thought, "…Vale. Eso ha sido mucho."),
    done: Ending {
        title: "El sello se calma",
        last: (Voice::Chiquita, "—No lo sientas tanto."),
        effects: &[
            ("deseo", 3),
            ("dignity", -2),
            ("devotion", 1),
            ("hunger", -2),
        ],
        thought: "...Vale. Eso ha sido mucho.",
    },
    half: Ending {
        title: "Lo dejas a medias",
        last: (Voice::ChiquitaJealous, "—¿Ya? Qué decepción. Para las dos."),
        effects: &[("deseo", 1), ("hunger", -1)],
        thought: "Mejor... lo dejo para otro día.",
    },
    brake: None,
    burst: None,
    cross: 0.5,
    poses: &[],
    // Chiquita, inside the seal, cannot help asking; the Queen can tease her for it.
    prompts: &[
        Prompt {
            at: (Part::Intro, 0.8),
            ask: (
                Voice::ChiquitaJealous,
                "—¿De verdad vas a…? ¿Ahora? ¿Conmigo AQUÍ dentro?",
            ),
            answers: [
                Answer {
                    label: "—Puedes mirar, si quieres.",
                    reply: (
                        Voice::ChiquitaSplat,
                        "—¡¿MIRAR?! …No pienso mirar. …¿Desde dónde se ve mejor?",
                    ),
                    heat: 0.08,
                    effects: &[("chiquita_afecto", 1), ("deseo", 1)],
                },
                Answer {
                    label: "—Mira la pared, como prometiste.",
                    reply: (
                        Voice::Chiquita,
                        "—La pared. Claro. Preciosa pared. Muy… pared.",
                    ),
                    heat: 0.0,
                    effects: &[],
                },
                Answer {
                    label: "(Hacer como que no está)",
                    reply: (
                        Voice::Thought,
                        "Si no la miro, no existe. Como los correos del lunes.",
                    ),
                    heat: 0.04,
                    effects: &[("dignity", 1)],
                },
            ],
        },
        Prompt {
            at: (Part::Loop, 4.0),
            ask: (
                Voice::ChiquitaJealous,
                "—¿…En quién estás pensando? Dímelo. No. No me lo digas.",
            ),
            answers: [
                Answer {
                    label: "—En Nina.",
                    reply: (
                        Voice::ChiquitaScold,
                        "—¿EN NINA? ¿La de la cofia? Estoy literalmente AQUÍ.",
                    ),
                    heat: 0.1,
                    effects: &[("deseo", 1)],
                },
                Answer {
                    label: "—En ti, Chiquita.",
                    reply: (
                        Voice::ChiquitaFlustered,
                        "—…¿En mí? …Vale. Bien. Sigue. No me mires así.",
                    ),
                    heat: 0.12,
                    effects: &[("chiquita_afecto", 2)],
                },
                Answer {
                    label: "—En nada. Solo en esto.",
                    reply: (
                        Voice::Thought,
                        "Por una vez, la cabeza en blanco. Qué descanso.",
                    ),
                    heat: 0.05,
                    effects: &[("fear", -1)],
                },
            ],
        },
    ],
    black_climax: None,
    smash: None,
};

/// Nina, in the brothel's back room (Rosalía's menu, «Compañía»).
pub(super) const NINA: Spec = Spec {
    name: "Nina",
    dir: "burdel/nina/escena",
    phases: [
        "Nina se arrodilla entre tus muslos…",
        "Déjate hacer · suelta antes de que Nina te frene",
        "¡El sello estalla!",
        "Calma",
    ],
    lines: &[
        (
            0.0,
            Voice::Nina,
            "—Relájese, señora. Aquí nadie sabe quién es usted.",
        ),
        (
            0.15,
            Voice::Thought,
            "Eso es exactamente lo que me preocupa.",
        ),
        (
            0.30,
            Voice::ChiquitaJealous,
            "—Trescientos años sin pisar este sitio. Y tú ya tienes tarjeta de clienta.",
        ),
        (
            0.50,
            Voice::NinaSly,
            "—Mmh... sabe a tormenta. ¿Seguro que es usted de por aquí?",
        ),
        (
            0.70,
            Voice::Narration,
            "El sello bebe de cada caricia, ansioso.",
        ),
        (
            0.88,
            Voice::ChiquitaScold,
            "—No pienso dejarle propina. Que conste.",
        ),
    ],
    climax: (
        Voice::Sound,
        "las runas estallan en rojo bajo la lengua de Nina",
    ),
    relax: (
        Voice::NinaSly,
        "—Vuelva cuando quiera, señora. Yo no hago preguntas.",
    ),
    done: Ending {
        title: "El sello, saciado",
        last: (
            Voice::ChiquitaScold,
            "—Rosalía te va a cobrar el doble la próxima vez.",
        ),
        effects: &[("hunger", -6), ("deseo", 2), ("nina_trust", 1)],
        thought: "...El sello ha comido más que en una semana de té.",
    },
    half: Ending {
        title: "Te levantas antes de tiempo",
        last: (Voice::Nina, "—¿Ya se va? Qué lástima. La noche era larga."),
        effects: &[("hunger", -2), ("deseo", 1)],
        thought: "Otro día. Quizá.",
    },
    brake: Some((
        3.0,
        0.15,
        Voice::NinaSly,
        "—Shh... despacio, señora. La noche es larga.",
    )),
    // v5 climax (H3 clip climax_52): the gush starts at 3.3 s and is strongest at 3.6 s.
    burst: Some((3.25, 3.6)),
    cross: 0.5,
    poses: &[],
    prompts: &[Prompt {
        at: (Part::Intro, 1.0),
        ask: (
            Voice::NinaSly,
            "—¿Cómo la quiere, señora? ¿Despacito… o como a mí me gusta?",
        ),
        answers: [
            Answer {
                label: "—Despacito.",
                reply: (Voice::Nina, "—Como mande. Tenemos toda la noche."),
                heat: -0.02,
                effects: &[("nina_trust", 1)],
            },
            Answer {
                label: "—Como a ti te guste.",
                reply: (
                    Voice::NinaSly,
                    "—Uy. Eso ha sido un error, señora. Uno precioso.",
                ),
                heat: 0.1,
                effects: &[("deseo", 1)],
            },
            Answer {
                label: "(Cerrar los ojos)",
                reply: (
                    Voice::ChiquitaAway,
                    "—Yo también cierro los ojos. Por solidaridad.",
                ),
                heat: 0.04,
                effects: &[],
            },
        ],
    }],
    black_climax: None,
    smash: None,
};

/// Nina again, the Queen with the shape Rosalía's elixir gives the seal, seen through the Queen's
/// eyes: Nina's mouth (the calm loop), then her body (the eager loop, missionary or from behind,
/// ←/→), and the burst heard over black.
pub(super) const NINA_FUTA: Spec = Spec {
    name: "Nina (futa)",
    dir: "burdel/nina/escena_futa",
    phases: [
        "Nina se arrodilla ante ti…",
        "Su boca, luego su cuerpo · suelta antes de que te frene",
        "¡El sello se derrama!",
        "Calma",
    ],
    lines: &[
        (
            0.0,
            Voice::NinaSly,
            "—Mmh. Deje que la vea bien, señora… Madre mía.",
        ),
        (
            0.15,
            Voice::Thought,
            "Técnicamente es del sello. Pero me lo quedo.",
        ),
        (
            0.30,
            Voice::ChiquitaJealous,
            "—Trescientos años sin crecerte nada, y te crece ESO. Felicidades, supongo.",
        ),
        (
            0.44,
            Voice::NinaSly,
            "—No me cabe en la boca. Literalmente. …Pienso seguir intentándolo.",
        ),
        (
            0.58,
            Voice::Nina,
            "—Ahora sí… venga aquí. Quiero sentirla entera. Si es que se puede.",
        ),
        (
            0.72,
            Voice::NinaSly,
            "—Me va a dejar marcada la forma, señora. Pienso cobrárselo.",
        ),
        (
            0.88,
            Voice::ChiquitaAway,
            "—No mires. No mires. …Vale, estoy mirando.",
        ),
    ],
    climax: (
        Voice::Sound,
        "las runas estallan y el sello se derrama dentro de Nina",
    ),
    relax: (
        Voice::NinaSly,
        "—Volverá, ¿verdad? …Y tráigase el frasco de Rosalía. Dos.",
    ),
    done: Ending {
        title: "El sello, desbordado",
        last: (
            Voice::ChiquitaScold,
            "—Rosalía te va a cobrar las sábanas. Y el colchón.",
        ),
        effects: &[("hunger", -7), ("deseo", 3), ("nina_trust", 1)],
        thought: "...Así que el sello también sabe tomar. Bueno es saberlo.",
    },
    half: Ending {
        title: "Te detienes a tiempo",
        last: (
            Voice::Nina,
            "—¿Ya? Qué lástima… guárdemelo para otra noche.",
        ),
        effects: &[("hunger", -2), ("deseo", 1)],
        thought: "Otro día. Cuando sepa qué hacer con esto.",
    },
    brake: Some((
        3.0,
        0.15,
        Voice::NinaSly,
        "—Shh... despacio, señora. Que me la va a gastar.",
    )),
    // The climax's sound (clip f_climax_162): the burst comes at 2.5 s and peaks at 2.8 s.
    burst: Some((2.5, 2.8)),
    // Mouth and body are different shots: a slower dissolve between them.
    cross: 0.9,
    poses: &[
        Pose {
            dir: "bucle_rapido",
            label: "Misionero",
            line: (
                Voice::NinaSly,
                "—Míreme a los ojos mientras… ¡ay! No se acaba nunca, ¿verdad?",
            ),
        },
        Pose {
            dir: "bucle_perrito",
            label: "Perrito",
            line: (
                Voice::Nina,
                "—¿Así? ¡Ay! Así se nota todavía más grande… ¡Más!",
            ),
        },
    ],
    prompts: &[
        Prompt {
            at: (Part::Intro, 0.8),
            ask: (Voice::Nina, "—¡¿S-señora?! ¿Eso es… todo suyo?"),
            answers: [
                Answer {
                    label: "—Todo tuyo esta noche.",
                    reply: (
                        Voice::NinaSly,
                        "—Ay, madre. Voy a necesitar las dos manos. Y fe.",
                    ),
                    heat: 0.08,
                    effects: &[("nina_trust", 1)],
                },
                Answer {
                    label: "—¿Demasiado para ti?",
                    reply: (
                        Voice::NinaSly,
                        "—¿Para mí? Señora, no sabe con quién habla. …Bueno. Un poquito.",
                    ),
                    heat: 0.12,
                    effects: &[("deseo", 1)],
                },
                Answer {
                    label: "(Sonreír sin decir nada)",
                    reply: (
                        Voice::ChiquitaFlustered,
                        "—Mírala. Se ha quedado sin palabras. Como yo.",
                    ),
                    heat: 0.04,
                    effects: &[],
                },
            ],
        },
        Prompt {
            at: (Part::LoopFast, 1.2),
            ask: (Voice::Nina, "—No va a caber, señora. Se lo digo yo."),
            answers: [
                Answer {
                    label: "—Despacio, entonces.",
                    reply: (
                        Voice::Nina,
                        "—Despacio… sí… ¡ay! Despacio, he dicho. …Vale. Cabe.",
                    ),
                    heat: -0.04,
                    effects: &[("nina_trust", 1)],
                },
                Answer {
                    label: "—Ya verás que sí.",
                    reply: (
                        Voice::NinaSly,
                        "—…Cabe. ¿Cómo que cabe? ¿Dónde lo estaba guardando?",
                    ),
                    heat: 0.1,
                    effects: &[("deseo", 1)],
                },
                Answer {
                    label: "—¿Quieres que pare?",
                    reply: (Voice::NinaSly, "—Ni se le ocurra."),
                    heat: 0.06,
                    effects: &[],
                },
            ],
        },
    ],
    // The burst over black: its sound (5.2 s) and the room's light, no picture.
    black_climax: Some(5.18),
    smash: Some("—¡EH! ¡Esta parte NO se ve! …Bueno, se oye. Pero NO se ve."),
};

/// Frames decoded on a thread of their own, newest request first (JPEG at 1024² takes longer
/// than a frame of the world).
pub(super) struct Decoder {
    pub(super) ask: mpsc::Sender<Job>,
    pub(super) done: mpsc::Receiver<(u64, image::RgbaImage)>,
    pub(super) busy: bool,
}

/// A frame from disk (black when it cannot be read).
fn load_frame(path: &Path) -> image::RgbaImage {
    super::clip_video::load(path).unwrap_or_else(|| {
        eprintln!("[escena] {} no se pudo leer", path.display());
        image::RgbaImage::new(1, 1)
    })
}

impl Decoder {
    pub(super) fn new() -> Self {
        let (ask, requests) = mpsc::channel::<Job>();
        let (send, done) = mpsc::channel();
        let _ = std::thread::Builder::new()
            .name("bed-scene-frames".into())
            .spawn(move || {
                while let Ok(mut job) = requests.recv() {
                    // Only the newest request matters.
                    while let Ok(newer) = requests.try_recv() {
                        job = newer;
                    }
                    let mut picture = load_frame(&job.1);
                    // Fading: the outgoing frame mixed over it by its weight.
                    if let Some((other, weight)) = &job.2 {
                        let old = load_frame(other);
                        if old.dimensions() == picture.dimensions() {
                            let w = (weight.clamp(0.0, 1.0) * 256.0) as u32;
                            for (a, b) in picture.as_mut().iter_mut().zip(old.as_raw()) {
                                *a = ((*a as u32 * (256 - w) + *b as u32 * w) >> 8) as u8;
                            }
                        }
                    }
                    if send.send((job.0, picture)).is_err() {
                        break;
                    }
                }
                super::clip_video::close_all();
            });
        Self {
            ask,
            done,
            busy: false,
        }
    }
}

pub(super) struct BedScene {
    spec: &'static Spec,
    root: PathBuf,
    frames: [Vec<PathBuf>; 5],
    part: Part,
    /// The picture going out while the clips change.
    fade: Option<Fade>,
    /// Seconds into the part's clip (the loop's run faster with the pleasure).
    time: f32,
    /// The pleasure bar, 0..1.
    pub(super) heat: f32,
    /// Space or the button held, and for how long.
    holding: bool,
    held: f32,
    /// The closing card: finished (`true`) or left half-way.
    ended: Option<bool>,
    started_at: f32,
    decoder: Decoder,
    /// The frame on show and the one asked for (its part, index and any fade, as one key).
    shown: Option<u64>,
    asked: Option<u64>,
    /// A decoded frame waiting for the GPU.
    upload: Option<image::RgbaImage>,
    uploads: u32,
    /// The sound playing belongs to this part (and posture).
    sound_for: Option<(Part, usize)>,
    /// Seconds since it opened, and until when the brake's line shows.
    clock: f32,
    braked_until: f32,
    /// She stopped you: holding counts again once the button is let go.
    pub(super) braked: bool,
    /// The lines said so far (voice, text, when), newest last; at most three are kept.
    said: Vec<(Voice, &'static str, f32)>,
    /// When the closing card came up (it ignores the keys for a moment, so a press still
    /// meant for the scene does not shut it unseen).
    ended_at: f32,
    /// The eager loop's postures (their frames) and the one playing.
    poses: Vec<Vec<PathBuf>>,
    pose: usize,
    /// When the part on show began (scene clock).
    part_since: f32,
    /// The question open: which, when it opened, the answer lit.
    asking: Option<(usize, f32, usize)>,
    /// The questions already asked.
    questioned: Vec<bool>,
    /// A line said over the heat's for a moment (a reply, a posture's word), until then.
    aside: Option<(Voice, &'static str, f32)>,
    /// What the answers changed (it lands with the ending's effects).
    gained: Vec<(&'static str, i32)>,
    /// Chiquita's comments for the screen (with what she does), and those already sent.
    floats: Vec<(&'static str, mascot::Cue)>,
    floated: Vec<&'static str>,
    /// She has already broken the screen in front of this climax.
    smashed: bool,
    /// The light that answers the held key, 0..1: it swells while held and ebbs after.
    held_glow: f32,
}

impl BedScene {
    /// `None` when the clips are not on disk.
    pub(super) fn open(spec: &'static Spec, elapsed: f32) -> Option<Self> {
        let root = crate::exploration3d_state::data_dir()
            .join("assets")
            .join(spec.dir);
        let frames = Part::ALL.map(|part| super::clip_video::frames(&root.join(part.dir())));
        let poses: Vec<Vec<PathBuf>> = spec
            .poses
            .iter()
            .map(|pose| super::clip_video::frames(&root.join(pose.dir)))
            .collect();
        // The eager loop is optional, and a climax heard over black has no frames; every other
        // part is needed.
        if Part::ALL.iter().any(|&p| {
            p != Part::LoopFast
                && !(p == Part::Climax && spec.black_climax.is_some())
                && frames[p.index()].is_empty()
        }) || poses.iter().any(Vec::is_empty)
        {
            eprintln!("[escena] faltan las tomas en {}", root.display());
            return None;
        }
        Some(Self {
            spec,
            root,
            frames,
            part: Part::Intro,
            fade: None,
            time: 0.0,
            heat: 0.0,
            holding: false,
            held: 0.0,
            ended: None,
            started_at: elapsed,
            decoder: Decoder::new(),
            shown: None,
            asked: None,
            upload: None,
            uploads: 0,
            sound_for: None,
            clock: 0.0,
            braked_until: -1.0,
            braked: false,
            ended_at: 0.0,
            said: Vec::new(),
            poses,
            pose: 0,
            part_since: 0.0,
            asking: None,
            questioned: vec![false; spec.prompts.len()],
            aside: None,
            gained: Vec::new(),
            floats: Vec::new(),
            floated: Vec::new(),
            smashed: false,
            held_glow: 0.0,
        })
    }

    /// The frames of `part` (the eager loop's in posture `pose`).
    fn clip(&self, part: Part, pose: usize) -> &[PathBuf] {
        if part == Part::LoopFast && !self.poses.is_empty() {
            return &self.poses[pose.min(self.poses.len() - 1)];
        }
        &self.frames[part.index()]
    }

    /// The folder of `part`'s clip (its sound is there).
    fn clip_dir(&self, part: Part, pose: usize) -> PathBuf {
        match (part, self.spec.poses.get(pose)) {
            (Part::LoopFast, Some(p)) => self.root.join(p.dir),
            _ => self.root.join(part.dir()),
        }
    }

    /// The climax is heard over black.
    fn black(&self) -> bool {
        self.part == Part::Climax && self.spec.black_climax.is_some()
    }

    /// ←/→ in the eager loop: the next posture, the old one dissolving under it.
    pub(super) fn turn_pose(&mut self, by: i32) {
        let n = self.poses.len();
        if n < 2 || self.ended.is_some() || self.part != Part::LoopFast {
            return;
        }
        let next = (self.pose as i32 + by).rem_euclid(n as i32) as usize;
        self.fade = Some(Fade {
            part: Part::LoopFast,
            pose: self.pose,
            time: self.time,
            rate: self.rate(),
            start: self.clock,
            length: self.spec.cross,
        });
        self.pose = next;
        self.time %= self.length(Part::LoopFast);
        let (voice, text) = self.spec.poses[next].line;
        self.say_aside(voice, text, 2.8);
    }

    /// A line over the heat's for `secs` (Chiquita's float instead).
    fn say_aside(&mut self, voice: Voice, text: &'static str, secs: f32) {
        match voice.floats() {
            Some(cue) => self.floats.push((text, cue)),
            None => self.aside = Some((voice, text, self.clock + secs)),
        }
    }

    /// The climax is going on over black (Chiquita guards it).
    pub(super) fn black_now(&self) -> bool {
        self.ended.is_none() && self.black()
    }

    /// The answer `pick` to the question open.
    pub(super) fn answer(&mut self, pick: usize) {
        let Some((i, _, _)) = self.asking.take() else {
            return;
        };
        let Some(a) = self.spec.prompts[i].answers.get(pick) else {
            return;
        };
        self.heat = (self.heat + a.heat).clamp(0.0, 0.97);
        self.gained.extend_from_slice(a.effects);
        self.say_aside(a.reply.0, a.reply.1, 3.4);
    }

    /// ↑/↓ while a question is open: the answer lit.
    pub(super) fn light_answer(&mut self, by: i32) {
        if let Some((_, _, lit)) = self.asking.as_mut() {
            *lit = (*lit as i32 + by).rem_euclid(3) as usize;
        }
    }

    /// The answer lit, for Enter.
    pub(super) fn lit_answer(&self) -> Option<usize> {
        self.asking.map(|(_, _, lit)| lit)
    }

    /// Chiquita's comments since the last step.
    pub(super) fn take_floats(&mut self) -> Vec<(&'static str, mascot::Cue)> {
        std::mem::take(&mut self.floats)
    }

    fn part_index(&self) -> usize {
        self.part.index()
    }

    /// How long a part's clip lasts (s).
    fn length(&self, part: Part) -> f32 {
        if part == Part::Climax {
            if let Some(secs) = self.spec.black_climax {
                return secs;
            }
        }
        self.clip(part, self.pose).len().max(1) as f32 / FPS
    }

    /// The scene has its eager loop.
    fn has_fast(&self) -> bool {
        !self.clip(Part::LoopFast, self.pose).is_empty()
    }

    /// How fast the clip plays now: the loops quicken with the pleasure.
    fn rate(&self) -> f32 {
        match self.part {
            Part::Loop => 0.75 + self.heat * 0.6,
            Part::LoopFast => 0.8 + self.heat * 0.4,
            _ => 1.0,
        }
    }

    /// On to `part`: from its start, or (between the two loops) at the same moment of the cycle,
    /// with the old picture fading out under the new one.
    fn go(&mut self, part: Part) {
        let between_loops = part.looping() && self.part.looping();
        self.part_since = self.clock;
        if part == Part::Climax {
            self.asking = None;
        }
        self.fade = Some(Fade {
            part: self.part,
            pose: self.pose,
            time: self.time,
            rate: self.rate(),
            start: self.clock,
            // The climax and its afterglow are different shots: a longer fade between them.
            length: if between_loops {
                self.spec.cross
            } else if part == Part::Relax {
                0.9
            } else {
                0.3
            },
        });
        self.time = if between_loops {
            self.time % self.length(part)
        } else {
            0.0
        };
        self.part = part;
    }

    pub(super) fn set_holding(&mut self, holding: bool) {
        if !holding {
            self.braked = false;
            self.held = 0.0;
        }
        self.holding = holding && !self.braked;
    }

    /// The card has been up long enough to be closed.
    fn card_ready(&self) -> bool {
        self.ended.is_some() && self.ended_at > 0.0 && self.clock - self.ended_at > 1.0
    }

    /// Esc: half-way out while it builds; skips to the card once it has burst.
    fn stop(&mut self) {
        match self.part {
            Part::Intro | Part::Loop | Part::LoopFast => self.ended = Some(false),
            Part::Climax | Part::Relax => self.ended = Some(true),
        }
    }

    /// One step: the pleasure, the clip's clock, which frame to decode. Returns when it ended.
    fn step(&mut self, dt: f32) {
        self.clock += dt;
        let lit = self.holding && !self.braked && self.ended.is_none() && self.part.looping();
        let (target, rate) = if lit { (1.0, 5.0) } else { (0.0, 2.2) };
        self.held_glow += (target - self.held_glow) * (1.0 - (-dt * rate).exp());
        if self.ended.is_some() && self.ended_at == 0.0 {
            self.ended_at = self.clock;
        }
        // The clock waits for the first frame, so it never starts on a black square.
        if self.ended.is_none() && self.shown.is_some() {
            let len = self.length(self.part);
            match self.part {
                Part::Loop | Part::LoopFast => {
                    if self.holding {
                        self.held += dt;
                        self.heat += (0.108 + self.held.min(4.0) * 0.036) * dt;
                        // Too long at a stretch: she stops you, and the pleasure drops.
                        if let Some((after, drop, _, _)) = self.spec.brake {
                            if self.held > after {
                                self.heat -= drop;
                                self.holding = false;
                                self.braked = true;
                                self.held = 0.0;
                                self.braked_until = self.clock + 2.5;
                            }
                        }
                    } else {
                        self.heat -= 0.072 * dt;
                    }
                    self.heat = self.heat.clamp(0.0, 1.0);
                    // Past half the bar the eager loop takes over; back under, the calm one.
                    if self.has_fast() {
                        let want = if self.heat >= 0.55 {
                            Part::LoopFast
                        } else if self.heat < 0.42 {
                            Part::Loop
                        } else {
                            self.part
                        };
                        if want != self.part {
                            self.go(want);
                        }
                    }
                    let len = self.length(self.part);
                    self.time = (self.time + dt * self.rate()) % len;
                    if self.heat >= 1.0 {
                        self.go(Part::Climax);
                    }
                }
                _ => {
                    self.time += dt;
                    if self.time >= len {
                        match self.part {
                            Part::Intro => self.go(Part::Loop),
                            Part::Climax => self.go(Part::Relax),
                            _ => self.ended = Some(true),
                        }
                    }
                }
            }
        }
        // A question comes when its moment does; unanswered, silence answers for her.
        if self.ended.is_none() && self.shown.is_some() {
            if let Some((_, opened, _)) = self.asking {
                if self.clock - opened > ASK_SECS {
                    self.answer(2);
                }
            } else if let Some(i) = (0..self.spec.prompts.len()).find(|&i| {
                let (part, after) = self.spec.prompts[i].at;
                !self.questioned[i] && part == self.part && self.clock - self.part_since >= after
            }) {
                self.questioned[i] = true;
                self.asking = Some((i, self.clock, 0));
            }
        }
        // The picture going out runs on under the new one until it has faded.
        let fade_len = self
            .fade
            .map(|f| self.clip(f.part, f.pose).len().max(1) as f32 / FPS);
        if let (Some(fade), Some(len)) = (self.fade.as_mut(), fade_len) {
            fade.time += dt * fade.rate;
            fade.time = if fade.part.looping() {
                fade.time % len
            } else {
                fade.time.min(len - 1.0 / FPS)
            };
            if self.clock - fade.start >= fade.length {
                self.fade = None;
            }
        }
        // The line of the moment joins the cards when it changes (narration goes to the subtitle,
        // Chiquita floats on the screen).
        if self.ended.is_none() {
            let (voice, text) = self.current_line();
            // Over black, Chiquita steps in front of it and breaks the screen.
            if self.black() && !self.smashed {
                self.smashed = true;
                if let Some(line) = self.spec.smash {
                    self.floats.push((line, mascot::Cue::Smash));
                }
            }
            if let Some(cue) = voice.floats() {
                if !self.floated.contains(&text) {
                    self.floated.push(text);
                    self.floats.push((text, cue));
                }
            } else if voice.spoken() && self.said.last().map(|l| l.1) != Some(text) {
                // Said again (the brake, say): it moves to the front instead of showing twice.
                self.said.retain(|l| l.1 != text);
                self.said.push((voice, text, self.clock));
                if self.said.len() > 3 {
                    self.said.remove(0);
                }
            }
        }
        // The newest decoded frame goes up next draw; ask for the one the clock is on.
        while let Ok((id, picture)) = self.decoder.done.try_recv() {
            self.decoder.busy = false;
            self.shown = Some(id);
            self.upload = Some(picture);
        }
        let frame = |part: Part, pose: usize, time: f32| {
            let list = self.clip(part, pose);
            let index = ((time * FPS) as usize).min(list.len().saturating_sub(1));
            (index, list.get(index).cloned())
        };
        let (index, path) = frame(self.part, self.pose, self.time);
        // Over black, nothing to decode.
        let Some(path) = path.filter(|_| !self.black()) else {
            return;
        };
        // The frame going out, and how much of it is left (in 16ths, part of the key).
        let blend = self.fade.and_then(|f| {
            let (i, other) = frame(f.part, f.pose, f.time);
            let weight = 1.0 - ((self.clock - f.start) / f.length).clamp(0.0, 1.0);
            other.map(|o| {
                (
                    f.part.index() as u64 + 8 * f.pose as u64,
                    i as u64,
                    o,
                    (weight * 16.0).round(),
                )
            })
        });
        let mut id =
            (self.part_index() as u64) << 48 | (self.pose as u64) << 44 | (index as u64) << 32;
        if let Some((part, i, _, w)) = &blend {
            id |= 1 << 31 | part << 24 | i << 8 | *w as u64;
        }
        if !self.decoder.busy && self.shown != Some(id) && self.asked != Some(id) {
            let job = (id, path, blend.map(|(_, _, o, w)| (o, w / 16.0)));
            if self.decoder.ask.send(job).is_ok() {
                self.decoder.busy = true;
                self.asked = Some(id);
            }
        }
    }

    /// The burst's light, 0..1: rises to its peak in the climax clip, then fades over 1.5 s.
    fn flash(&self) -> f32 {
        let (Some((start, peak)), Part::Climax) = (self.spec.burst, self.part) else {
            return 0.0;
        };
        let t = self.time;
        if t < start {
            0.0
        } else if t < peak {
            (t - start) / (peak - start).max(0.01)
        } else {
            (1.0 - (t - peak) / 1.5).max(0.0).powi(2)
        }
    }

    /// What is being said now: the brake's line, a reply or a posture's word, the question open,
    /// the heat's line, or the part's.
    fn current_line(&self) -> (Voice, &'static str) {
        if let Some((voice, text, until)) = self.aside {
            if self.clock < until && !(self.part.looping() && self.clock < self.braked_until) {
                return (voice, text);
            }
        }
        if let Some((i, _, _)) = self.asking {
            if !(self.part.looping() && self.clock < self.braked_until) {
                return self.spec.prompts[i].ask;
            }
        }
        match self.part {
            Part::Loop | Part::LoopFast if self.clock < self.braked_until => {
                let (_, _, voice, text) = self.spec.brake.expect("braked without a brake");
                (voice, text)
            }
            Part::Intro | Part::Loop | Part::LoopFast => {
                let lines = self.spec.lines;
                let (_, voice, text) = lines
                    .iter()
                    .rev()
                    .find(|(at, _, _)| self.heat >= *at)
                    .copied()
                    .unwrap_or(lines[0]);
                (voice, text)
            }
            Part::Climax => self.spec.climax,
            Part::Relax => self.spec.relax,
        }
    }

    /// The stage: the cards, the vial, the key, or the closing card.
    fn view(&self, elapsed: f32) -> map_overlay::SceneView {
        if let Some(done) = self.ended {
            let ending = if done {
                &self.spec.done
            } else {
                &self.spec.half
            };
            let name = self.spec.name.split(" (").next().unwrap_or(self.spec.name);
            let mut who = name.chars();
            let result = map_overlay::SceneResult {
                title: ending.title.to_owned(),
                who: who
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + who.as_str())
                    .unwrap_or_default(),
                complete: done,
                effects: {
                    let mut all: Vec<(&str, i32)> = ending.effects.to_vec();
                    if done {
                        for &(var, by) in &self.gained {
                            match all.iter_mut().find(|entry| entry.0 == var) {
                                Some(entry) => entry.1 += by,
                                None => all.push((var, by)),
                            }
                        }
                    }
                    all.iter()
                        .map(|(var, by)| (label(var).to_owned(), *by))
                        .collect()
                },
                seconds: (elapsed - self.started_at).max(0.0).round() as u32,
                quote: card_line(ending.last.0, ending.last.1, 255, 0),
                age_ms: ((self.clock - self.ended_at).max(0.0) * 1000.0) as u32,
            };
            return map_overlay::SceneView {
                heat: Some(if done {
                    1000
                } else {
                    (self.heat * 1000.0) as u16
                }),
                phase: String::new(),
                lines: Vec::new(),
                result: Some(result),
                frame: self.uploads,
                ..Default::default()
            };
        }
        let n = self.said.len();
        let lines = self
            .said
            .iter()
            .enumerate()
            .map(|(i, (voice, text, at))| {
                let fresh = ((self.clock - at) / 0.3).clamp(0.0, 1.0);
                card_line(
                    *voice,
                    text,
                    (fresh * 255.0) as u8,
                    (n - 1 - i).min(2) as u8,
                )
            })
            .collect();
        let (voice, text) = self.current_line();
        let caption = match voice {
            Voice::Narration => Some(text.to_owned()),
            Voice::Sound => Some(format!("— {text} —")),
            _ => None,
        };
        let building = matches!(self.part, Part::Intro | Part::Loop | Part::LoopFast);
        map_overlay::SceneView {
            heat: Some(match self.part {
                Part::Intro | Part::Loop | Part::LoopFast => (self.heat * 1000.0) as u16,
                _ => 1000,
            }),
            phase: self.spec.phases[self.part.phase_slot()].to_owned(),
            holding: self.holding,
            hold: match (self.spec.brake, self.part) {
                (Some((after, _, _, _)), Part::Loop | Part::LoopFast) => {
                    Some(((self.held / after).clamp(0.0, 1.0) * 1000.0) as u16)
                }
                _ => None,
            },
            braked: self.braked,
            marks: if building {
                self.spec
                    .lines
                    .iter()
                    .skip(1)
                    .map(|(at, _, _)| (at * 1000.0) as u16)
                    .collect()
            } else {
                Vec::new()
            },
            lines,
            caption,
            keys: building,
            esc: if building { "parar" } else { "saltar" }.to_owned(),
            result: None,
            frame: self.uploads,
            flash: (self.flash() * 1000.0) as u16,
            glow: (self.held_glow.clamp(0.0, 1.0) * 1000.0) as u16,
            prompt: self.asking.map(|(i, opened, lit)| {
                let prompt = &self.spec.prompts[i];
                map_overlay::ScenePrompt {
                    answers: prompt.answers.iter().map(|a| a.label.to_owned()).collect(),
                    lit: lit as u8,
                    left: ((1.0 - (self.clock - opened) / ASK_SECS).clamp(0.0, 1.0) * 1000.0)
                        as u16,
                    accent: prompt.ask.0.card().1,
                }
            }),
            poses: self.spec.poses.iter().map(|p| p.label.to_owned()).collect(),
            pose: self.pose as u8,
            pose_active: self.part == Part::LoopFast
                && self.poses.len() > 1
                && self.ended.is_none(),
            black: self.black(),
            game: None,
            deal: None,
        }
    }
}

impl App {
    /// Opens the bed scene (the guion's `@var escena_cama`); `eager` starts the pleasure half-way.
    pub(super) fn open_bed_scene(&mut self, eager: bool) {
        self.open_scene(&BED, eager);
    }

    /// Opens an intimate scene; `eager` starts the pleasure half-way.
    pub(super) fn open_scene(&mut self, spec: &'static Spec, eager: bool) {
        let Some(mut scene) = BedScene::open(spec, self.elapsed) else {
            return;
        };
        if eager {
            scene.heat = 0.5;
        }
        eprintln!("[escena] {}", spec.name);
        self.bed_scene = Some(scene);
        self.map_open = false;
        self.queen_thought = None;
        self.clear_input();
    }

    /// A scene (or a girl's minigame) is on the stage.
    pub(super) fn bed_scene_open(&self) -> bool {
        self.bed_scene.is_some() || self.play_scene.is_some()
    }

    /// The climax is behind black right now (Chiquita guards it).
    pub(super) fn scene_black(&self) -> bool {
        self.bed_scene.as_ref().is_some_and(|s| s.black_now()) || self.play_scene_black()
    }

    /// Each frame while it plays: the minigame steps, and its sound follows the part.
    pub(super) fn tick_bed_scene(&mut self, dt: f32) {
        self.tick_play_scene(dt);
        let Some(scene) = self.bed_scene.as_mut() else {
            return;
        };
        scene.step(dt);
        let want = if scene.ended.is_some() {
            None
        } else {
            Some((scene.part, scene.pose))
        };
        if want != scene.sound_for {
            scene.sound_for = want;
            if let Some((part, pose)) = want {
                super::gallery_seen::shot(&scene.clip_dir(part, pose));
            }
            let path = want.map(|(part, pose)| scene.clip_dir(part, pose).join("audio.ogg"));
            if let Some(audio) = &mut self.audio {
                audio.scene_sound(
                    path.as_deref(),
                    want.is_some_and(|(part, _)| part.looping()),
                );
            }
        }
        for (text, cue) in scene.take_floats() {
            self.float_chiquita_cue(text, cue);
        }
        self.clear_input();
    }

    /// Keys while it plays: hold Space; Esc stops (or skips to the card); F/Space/Enter on the
    /// card closes it. Every key is the scene's.
    pub(super) fn bed_scene_key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if self.play_scene.is_some() {
            self.play_scene_key(code, pressed, repeat);
            return;
        }
        let Some(scene) = self.bed_scene.as_mut() else {
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
                self.close_bed_scene();
            }
            return;
        }
        let fresh = pressed && !repeat;
        match code {
            KeyCode::Space => scene.set_holding(pressed),
            KeyCode::Escape | KeyCode::Backspace if fresh => scene.stop(),
            // Her question: 1–3, or ↑/↓ and Enter/F.
            KeyCode::Digit1 | KeyCode::Numpad1 if fresh => scene.answer(0),
            KeyCode::Digit2 | KeyCode::Numpad2 if fresh => scene.answer(1),
            KeyCode::Digit3 | KeyCode::Numpad3 if fresh => scene.answer(2),
            KeyCode::ArrowUp | KeyCode::KeyW if fresh => scene.light_answer(-1),
            KeyCode::ArrowDown | KeyCode::KeyS if fresh => scene.light_answer(1),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::KeyF if fresh => {
                if let Some(lit) = scene.lit_answer() {
                    scene.answer(lit);
                }
            }
            // The posture, in the eager loop.
            KeyCode::ArrowLeft | KeyCode::KeyA if fresh => scene.turn_pose(-1),
            KeyCode::ArrowRight | KeyCode::KeyD if fresh => scene.turn_pose(1),
            _ => {}
        }
    }

    /// The left button: an answer or a posture's arrow when it is on one, else held like Space;
    /// or closes the card.
    pub(super) fn bed_scene_button(&mut self, pressed: bool) {
        if self.play_scene.is_some() {
            self.play_scene_button(pressed);
            return;
        }
        let pointer = self.shop_pointer_position();
        let Some(scene) = self.bed_scene.as_mut() else {
            return;
        };
        if scene.ended.is_some() {
            if pressed && scene.card_ready() {
                self.close_bed_scene();
            }
            return;
        }
        if pressed {
            if let Some((x, y)) = pointer {
                let (w, h) = (WINDOW_WIDTH as f32, WINDOW_HEIGHT as f32);
                if scene.asking.is_some() {
                    if let Some(pick) = map_overlay::scene_prompt_hit(w, h, x, y) {
                        scene.answer(pick);
                        return;
                    }
                }
                if scene.part == Part::LoopFast {
                    if let Some(by) = map_overlay::scene_pose_hit(w, h, x, y) {
                        scene.turn_pose(by);
                        return;
                    }
                }
            }
        }
        scene.set_holding(pressed);
    }

    /// The pause menu or a lost focus: whatever is held lets go; a scene still building ends.
    pub(super) fn bed_scene_interrupt(&mut self) {
        self.play_scene_interrupt();
        if let Some(scene) = self.bed_scene.as_mut() {
            scene.set_holding(false);
            if scene.ended.is_none() {
                scene.stop();
            }
        }
    }

    /// Shuts the card: the effects land, she thinks one last thing, the game saves.
    fn close_bed_scene(&mut self) {
        let Some(scene) = self.bed_scene.take() else {
            return;
        };
        let ending = if scene.ended.unwrap_or(false) {
            &scene.spec.done
        } else {
            &scene.spec.half
        };
        let answered = if scene.ended.unwrap_or(false) {
            &scene.gained[..]
        } else {
            &[]
        };
        for (var, by) in ending.effects.iter().chain(answered) {
            *self.story.values.entry((*var).to_owned()).or_insert(0) += by;
        }
        // The ero record (the pause menu's status page).
        let climax = scene.ended.unwrap_or(false);
        match scene.spec.dir {
            "castillo/escena_cama" => super::ero::alone(&mut self.story.values, climax),
            "burdel/nina/escena" => {
                super::ero::encounter(&mut self.story.values, "nina", false, climax)
            }
            "burdel/nina/escena_futa" => {
                super::ero::encounter(&mut self.story.values, "nina", true, climax)
            }
            _ => {}
        }
        if let Some(audio) = &mut self.audio {
            audio.scene_sound(None, false);
        }
        self.queen_thought = Some((ending.thought, self.elapsed + 4.5));
        self.clear_input();
        self.persist_3d_state(true);
    }

    /// The panel for the overlay, and the frame waiting to go up to the GPU.
    pub(super) fn bed_scene_view(
        &mut self,
    ) -> Option<(map_overlay::SceneView, Option<image::RgbaImage>)> {
        if self.play_scene.is_some() {
            return self.play_scene_view();
        }
        let elapsed = self.elapsed;
        let scene = self.bed_scene.as_mut()?;
        let upload = scene.upload.take();
        if upload.is_some() {
            scene.uploads = scene.uploads.wrapping_add(1);
        }
        Some((scene.view(elapsed), upload))
    }
}

impl App {
    /// Visual QA: waits for the frame being decoded, so a capture shows the clock's frame.
    pub(super) fn bed_scene_settle(&mut self) {
        self.play_scene_settle();
        let Some(scene) = self.bed_scene.as_mut() else {
            return;
        };
        if scene.decoder.busy {
            if let Ok((id, picture)) = scene
                .decoder
                .done
                .recv_timeout(std::time::Duration::from_secs(3))
            {
                scene.decoder.busy = false;
                scene.shown = Some(id);
                scene.upload = Some(picture);
            }
        }
    }
}
