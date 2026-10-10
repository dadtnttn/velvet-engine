//! «No mires el anillo»: el minijuego del mago hipnotista del bosque, en el escenario de las escenas
//! íntimas (el cuadro con sus clips, las tarjetas con caras, el vial y Chiquita en 3D).
//!
//! El viejo te asalta en su claro con un anillo de cobre colgado de un hilo. Sobre el cuadro oscila
//! el anillo y, debajo, tu mirada: con A/D (o el ratón sobre el cuadro) la apartas, y el anillo tira
//! de ella como un muelle, más cuanto más hondo es el trance y cada vez que susurra. Mirándolo, sube
//! el trance (el vial) y, por tramos, la Reina cae: de rodillas, sus manos, en la cara, la boca. Con
//! la mirada lejos sube el «Despertar»: lleno, la Reina despierta y le da su merecido (el puñetazo
//! que toca a ese tramo). Cada poco da una orden con tres respuestas: obedecer hunde, negarse
//! despierta y burlarse despierta más, pero solo si en ese momento no miras el anillo (si lo miras,
//! la burla te sale obediente). Espacio mira a propósito: caer también es un final, y Chiquita no
//! se lo calla («¿por qué lo estás mirando?», cada vez más harta). A veces ella vuela delante del
//! anillo y lo tapa; a veces se hipnotiza ella. Con la boca ocupada, E (machacando) despierta la
//! lucidez: llena, la Reina muerde; si antes se llena el placer (Espacio, más fuerte), cae del todo.
//! Las sales que vende el propio mago (Q) dan un golpe de lucidez; su amuleto hace que el anillo
//! tire menos.
//!
//! Los clips están encadenados (cada uno empieza en la foto donde acaba el anterior); los tramos
//! los ponen en cola y los bucles esperan su turno (`hipno_reel`).
use super::*;

/// How far the ring hangs below its pivot, across the picture (the picture's side is 1).
const RING_LENGTH: f32 = 0.40;
/// Her gaze is on the ring within this much of it.
const ON_RING: f32 = 0.055;
/// How long an order waits for its answer (s).
const ORDER_SECS: f32 = 6.0;
/// A crossfade between clips that do not meet (s).
const FADE: f32 = 0.45;

/// How it ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum End {
    /// Awake: she punches him.
    Escape,
    /// Lucid with her mouth full: she bites.
    Bite,
    /// All the way under.
    Fall,
}

/// A stage of the trance: where it begins on the vial, what it is called and what happens, the
/// clips that take her there and the loop she stays in, her way out of it (the punch), the light;
/// what is said on getting there, and what he whispers in it.
struct Stage {
    at: f32,
    name: &'static str,
    hint: &'static str,
    into: &'static [&'static str],
    stay: &'static [&'static str],
    escape: &'static [&'static str],
    tint: [u8; 3],
    stamp: &'static str,
    queen: &'static str,
    mage: &'static str,
    caption: &'static str,
    chiquita: (&'static str, mascot::Cue),
    whispers: &'static [&'static str],
}

const STAGES: [Stage; 5] = [
    Stage {
        at: 0.0,
        name: "De pie",
        hint: "Aparta la mirada del anillo",
        into: &[],
        stay: &["l0"],
        escape: &["p0"],
        tint: [0x8a, 0x6a, 0xff],
        stamp: "",
        queen: "",
        mage: "",
        caption: "",
        chiquita: ("", mascot::Cue::Talk),
        whispers: &[
            "Mire el anillo… solo un momentito.",
            "Respire conmigo. Dentro… y fuera.",
            "Cobre viejo. Gira… y gira… y gira…",
            "Los párpados pesan. Como plomo tibio.",
            "No luche. Luchar cansa. Mirar descansa.",
            "Cada vez que aparta la vista… vuelve. ¿Lo nota?",
        ],
    },
    Stage {
        at: 0.2,
        name: "De rodillas",
        hint: "Las piernas ya no la sostienen",
        into: &["ta"],
        stay: &["l1"],
        escape: &["p1"],
        tint: [0xa8, 0x5a, 0xff],
        stamp: "¡DE RODILLAS!",
        queen: "—…¿Por qué… estoy en el suelo?",
        mage: "—Desde ahí abajo se ve mejor el anillo, ¿verdad? Jajaja.",
        caption: "Las rodillas de la Reina tocan la hierba sin que ella lo decida.",
        chiquita: (
            "—¡¿DE RODILLAS?! ¡Levántate! ¡Que eres una REINA!",
            mascot::Cue::Splat,
        ),
        whispers: &[
            "Así. Desde ahí abajo se ve mejor.",
            "Qué obediente es su cuerpo, Majestad. Más que usted.",
            "Ni el trono es tan cómodo como el suelo.",
            "Su cabeza está vacía y tibia. Como un baño.",
            "Más abajo… más hondo… más mía.",
        ],
    },
    Stage {
        at: 0.4,
        name: "Sus manos",
        hint: "El vestido se le escurre",
        into: &["tb"],
        stay: &["l2"],
        escape: &["p2"],
        tint: [0xc8, 0x4a, 0xe8],
        stamp: "¡LAS MANOS!",
        queen: "—El vestido… pesa mucho. …Mejor sin él.",
        mage: "—Shh. Las manos son parte del truco. Truco antiguo.",
        caption: "El viejo ya no lleva la túnica. Nadie le ha visto quitársela.",
        chiquita: (
            "—¡¿Y SU ROPA?! ¡¿Cuándo se ha quitado la ropa ese viejo?!",
            mascot::Cue::Flustered,
        ),
        whispers: &[
            "Mire el anillo y no mire mis manos.",
            "Su piel está caliente. Es la hipnosis. Seguro.",
            "Este vestido le estorba. A usted no, a mí.",
            "Truco antiguo. Muy antiguo. Muy, muy antiguo.",
            "Shh… shh… gira… gira…",
        ],
    },
    Stage {
        at: 0.6,
        name: "En la cara",
        hint: "Ya casi no hay vuelta",
        into: &["c3", "c4a", "c4b"],
        stay: &["l3"],
        escape: &["p3", "p3b", "p3c"],
        tint: [0xe0, 0x40, 0xc0],
        stamp: "¡EN LA CARA!",
        queen: "—…Huele a… cobre. …Y a algo más.",
        mage: "—Buena chica. Sin prisa. El anillo no tiene prisa.",
        caption: "El anillo gira delante de sus ojos. Y algo más, delante de su cara.",
        chiquita: (
            "—¡Eso NO es un anillo! ¡Majestad, eso NO ES UN ANILLO!",
            mascot::Cue::Scold,
        ),
        whispers: &[
            "Ya no hace falta el anillo. Pero lo dejo, por si acaso.",
            "Abra un poquito la boca. Para respirar.",
            "Todo lo que tiene delante es suyo, Majestad.",
            "Qué carita. Qué carita de reina.",
            "Un poquito más cerca… nada más.",
        ],
    },
    Stage {
        at: 0.8,
        name: "Con la boca",
        hint: "Suya",
        into: &["c5"],
        stay: &["c6"],
        escape: &[],
        tint: [0xff, 0x4a, 0x9a],
        stamp: "SUYA",
        queen: "—…Ya no quiero mirar otra cosa.",
        mage: "—Ahora sí, Majestad. Ahora es mía. Jajaja.",
        caption: "La Reina abre la boca. El anillo sigue girando.",
        chiquita: (
            "—…Vale. Ya está. Yo no miro. …Bueno, miro un poco.",
            mascot::Cue::Away,
        ),
        whispers: &[],
    },
];

/// One of his orders: what he says; the three answers (the tile and what the Queen says); a
/// mockery that fails comes out obedient; from which stage on it can come.
struct Order {
    say: &'static str,
    obey: (&'static str, &'static str),
    refuse: (&'static str, &'static str),
    mock: (&'static str, &'static str, &'static str),
    from: usize,
}

const ORDERS: [Order; 9] = [
    Order {
        say: "—Diga «sí». Solo eso. «Sí».",
        obey: ("Decir «sí»", "—…Sí."),
        refuse: ("Callarte", "—No pienso decirte nada."),
        mock: (
            "Decirlo al revés",
            "—«Is». …Ja. ¿Contento?",
            "—Sí… sí, sí… ¡sí! …¿Por qué sigo diciendo que sí?",
        ),
        from: 0,
    },
    Order {
        say: "—Suelte los brazos. Muertos. Así.",
        obey: ("Soltarlos", "—…Vale. Sueltos."),
        refuse: ("Cruzarlos", "—Los brazos son míos, viejo."),
        mock: (
            "Corte de mangas",
            "—Toma. Brazos sueltos. Este, sobre todo.",
            "—…Se me ha caído el brazo. Encima de usted. Sin querer.",
        ),
        from: 0,
    },
    Order {
        say: "—Dé un pasito hacia mí. Uno solo.",
        obey: ("Dar el paso", "—…Un pasito. Nada más."),
        refuse: ("Retroceder", "—Ni medio paso, abuelo."),
        mock: (
            "Pisarle el pie",
            "—Uy. Perdón. Mentira, no lo siento.",
            "—…Ya está. Pegadita. ¿Qué hago aquí tan pegadita?",
        ),
        from: 0,
    },
    Order {
        say: "—Cuente conmigo: uno… dos…",
        obey: ("«…tres»", "—…tres."),
        refuse: ("Al revés", "—Diez, nueve, ocho… ¡JA!"),
        mock: (
            "Contarle arrugas",
            "—Ochenta y una, ochenta y dos… no acabo nunca.",
            "—Tres… cuatro… cinco… ¿por dónde iba? Mmh…",
        ),
        from: 0,
    },
    Order {
        say: "—Llámeme «maestro».",
        obey: ("«Maestro…»", "—…Maestro."),
        refuse: ("Insultarle", "—Te voy a llamar cosas peores que «abuelo»."),
        mock: (
            "«Abuelito»",
            "—Sí, abuelito. ¿Le traigo la mantita?",
            "—Abuel… …maestro. ¡¿QUÉ?!",
        ),
        from: 0,
    },
    Order {
        say: "—Sonría. Una sonrisita para el anillo.",
        obey: ("Sonreír", "—…Je. …Je."),
        refuse: ("Ceño de reina", "—Esta es mi cara de reina. Acostúmbrate."),
        mock: (
            "Sacarle la lengua",
            "—Bleeeh.",
            "—Bleh… mmh… ¿por qué no vuelve a entrar mi lengua?",
        ),
        from: 1,
    },
    Order {
        say: "—Diga: «el anillo manda».",
        obey: ("Repetirlo", "—…El anillo manda."),
        refuse: ("Negarse", "—Aquí manda la Reina."),
        mock: (
            "«Es de hojalata»",
            "—El anillo es de hojalata. Y tú, de chatarra.",
            "—El anillo… es… precioso. …No. Espera.",
        ),
        from: 1,
    },
    Order {
        say: "—Las manos detrás de la espalda.",
        obey: ("Obedecer", "—…Detrás. Como usted diga."),
        refuse: ("Manotazo", "—Las manos son para abofetear viejos."),
        mock: (
            "Rascarse",
            "—Uy, qué picor. Justo aquí. En tu cara.",
            "—…Detrás. Juntitas. ¿Por qué me gusta?",
        ),
        from: 2,
    },
    Order {
        say: "—Abra la boca. Solo un poco.",
        obey: ("Abrirla", "—…Aaah."),
        refuse: ("Apretar los dientes", "—Mmmh-mmh. Ni lo sueñes."),
        mock: (
            "Bostezarle",
            "—Aaaah… uf. Qué aburrido eres.",
            "—Aaah… …mmh… …¿y ahora qué?",
        ),
        from: 3,
    },
];

const MAGE_OBEYED: [&str; 4] = [
    "—Jajaja. Muy bien. Muy, muy bien.",
    "—Buena chica. Digo… buena reina.",
    "—¿Ve qué fácil? Jajaja.",
    "—Así me gusta. Obediente como un reloj.",
];
const MAGE_REFUSED: [&str; 4] = [
    "—Jajaja… Tozuda. Mire, mire el anillo.",
    "—Ay, qué genio. Más fuerte, entonces.",
    "—No pasa nada. Tengo todo el día.",
    "—Tozuda. Me gustan tozudas. Duran más.",
];
const MAGE_MOCKED: [&str; 4] = [
    "—¡Oiga! …Eso no estaba en el guion.",
    "—Qué falta de respeto… a un profesional.",
    "—Jajaja… ja. No tiene gracia.",
    "—¡Majestad! Que estoy trabajando.",
];
const MAGE_BACKFIRED: [&str; 3] = [
    "—Jajaja. ¿Lo ve? La boca ya es mía.",
    "—Hasta las burlas le salen obedientes.",
    "—Jajaja. Ni para burlarse le queda voluntad.",
];

/// What Chiquita says while the Queen stares at the ring on purpose, more fed up each time.
const STARED: [(&str, mascot::Cue); 7] = [
    ("—¿Por qué lo estás mirando?", mascot::Cue::Scold),
    (
        "—¡Eh! ¡Que lo estás mirando A PROPÓSITO!",
        mascot::Cue::Scold,
    ),
    (
        "—¿Te gusta el anillito? ¿Es eso? ¿Te gusta el anillito de cobre?",
        mascot::Cue::Jealous,
    ),
    (
        "—Majestad. Por favor. Por el sello. Por mí. Deja. De. Mirarlo.",
        mascot::Cue::Scold,
    ),
    (
        "—Vale. Muy bien. Mírale el anillo. Mírale lo que quieras. A mí ni caso.",
        mascot::Cue::Jealous,
    ),
    (
        "—Que conste en acta: yo te avisé. Seis veces.",
        mascot::Cue::Away,
    ),
    ("—¿OTRA VEZ? ¡¿EN SERIO?!", mascot::Cue::Splat),
];
const STARED_MORE: [(&str, mascot::Cue); 4] = [
    (
        "—Me voy a quejar al sello. …Ah, no, que el sello soy yo.",
        mascot::Cue::Jealous,
    ),
    (
        "—Te estás dejando. Te estás DEJANDO. Lo estoy viendo.",
        mascot::Cue::Scold,
    ),
    (
        "—¿Sabes lo que cuesta encontrar una reina que no se deje hipnotizar? Pues tú no eres esa.",
        mascot::Cue::Jealous,
    ),
    (
        "—Esto no se lo cuento a Lira. Me muero de vergüenza yo por ti.",
        mascot::Cue::Flustered,
    ),
];
/// When her gaze slides onto the ring by itself.
const ON_RING_SHOUTS: [&str; 5] = [
    "—¡NO MIRES!",
    "—¡Eh! ¡Aquí! ¡Mírame a MÍ!",
    "—¡Que es un anillo de feria!",
    "—¡Despierta, que se te cae la baba!",
    "—¡Majestad! ¡Para el otro lado!",
];
const COVERS: [&str; 3] = [
    "—¡Tapadera! ¡No mires!",
    "—¡Quieto ahí, anillo! ¡No pasarás!",
    "—¡Yo me pongo delante! ¡Mírame a mí, que soy más bonita!",
];
const DAZED: [&str; 3] = [
    "—…qué anillo… más bonito…",
    "—…sí, maestro… lo que usted diga, maestro…",
    "—…gira… y gira… jiji…",
];
const UNDAZED: [&str; 2] = [
    "—¡AH! ¿Qué? ¡NO! ¡Yo no he dicho nada!",
    "—¡¿Qué hacía yo?! ¡No ha pasado! ¡Nada! ¡Silencio!",
];
const OBEYED: [(&str, mascot::Cue); 4] = [
    ("—¿«Sí»? ¡¿Cómo que «sí»?!", mascot::Cue::Jealous),
    (
        "—¿Otra vez le haces caso? ¡Es un viejo con un anillo de feria!",
        mascot::Cue::Scold,
    ),
    (
        "—Vale, ya lo pillo. Te gusta obedecer. A ÉL. Y a mí nunca.",
        mascot::Cue::Jealous,
    ),
    (
        "—Obediente, obediente… ¿Y a mí cuándo me obedeces?",
        mascot::Cue::Jealous,
    ),
];
const MOCKED: [&str; 3] = [
    "—¡JA! ¡Toma ya!",
    "—¡Eso! ¡Dale donde duele!",
    "—Me encanta cuando eres mala.",
];
const BITE_HINTS: [&str; 3] = [
    "—¡Muérdele! ¡E, E, E!",
    "—¡Que te despiertes y MUERDAS!",
    "—¡Tienes dientes de demonio! ¡ÚSALOS!",
];
const MOUTH_LINES: [(Voice, &str); 6] = [
    (Voice::QueenTrance, "—Mmh… …el anillo sigue girando…"),
    (Voice::QueenTrance, "—…Más. …Lo dijo el anillo."),
    (Voice::QueenTrance, "—No puedo… pensar… …ni quiero."),
    (Voice::Mago, "—Jajaja. Así, Majestad. Así."),
    (Voice::Mago, "—Qué boca tan obediente."),
    (Voice::Mago, "—Mañana no se acordará de nada. Yo sí."),
];
const CLOSE_UPS: [&str; 3] = [
    "—…No. …No voy a mirar.",
    "—Ngh… ese anillo… ¡no!",
    "—…Mi cabeza… pesa… …No.",
];
/// The two takes with her mouth: their names, their clips (one after another), her first word.
const POSES: [(&str, &[&str], &str); 2] = [
    ("Mejilla", &["c6"], "—…Despacio… la mejilla…"),
    ("Al fondo", &["c7", "c8"], "—…Toda. …Lo dijo el anillo."),
];
/// Where each escape leaves him.
const ESCAPED: [&str; 4] = [
    "La Reina parpadea, se levanta y le suelta un puñetazo que lo tira del tocón.",
    "Desde el suelo, la Reina le hunde el puño en la tripa.",
    "La Reina se sube el vestido de un tirón y lo tumba de un golpe.",
    "La Reina despierta con algo en la cara. El puñetazo va directo… ahí.",
];

/// Clip ends that meet the next one's start (no crossfade needed).
fn seam(from: &str, to: &str) -> f32 {
    let clean = matches!(
        (from, to),
        ("l0", "ta" | "p0" | "l0")
            | ("ta", "l1" | "tb" | "p1")
            | ("l1", "tb" | "p1" | "l1")
            | ("tb", "l2" | "c3" | "p2")
            | ("l2", "c3" | "p2" | "l2")
            | ("c3", "c4a")
            | ("c4a", "c4b")
            | ("c4b", "l3" | "c5" | "p3" | "p3b" | "p3c")
            | ("l3", "c5" | "p3" | "p3b" | "p3c" | "l3")
            | ("c5", "bb0")
            | ("c6", "c6" | "c7")
            | ("c7", "c8")
            | ("c8", "c9")
            | ("bb0", "bl0")
            | ("bl0", "bb1")
            | ("bb2", "bl2")
    );
    if clean {
        0.0
    } else {
        FADE
    }
}

/// The shots the reel opens, and whether each loops.
pub(super) const SHOTS: [(&str, bool); 26] = [
    ("l0", true),
    ("l0c", false),
    ("ta", false),
    ("l1", true),
    ("tb", false),
    ("l2", true),
    ("c3", false),
    ("c4a", false),
    ("c4b", false),
    ("l3", true),
    ("c5", false),
    ("c6", true),
    ("c7", false),
    ("c8", false),
    ("c9", false),
    ("p0", false),
    ("p1", false),
    ("p2", false),
    ("p3", false),
    ("p3b", false),
    ("p3c", false),
    ("bb0", false),
    ("bl0", false),
    ("bb1", false),
    ("bb2", false),
    ("bl2", false),
];

/// The game's state.
pub(super) struct HipnoGame {
    pub(super) stage: usize,
    pub(super) trance: f32,
    pub(super) awake: f32,
    /// Her gaze across the picture (0..1) and how fast it slides.
    pub(super) gaze: f32,
    gaze_vel: f32,
    /// A/D (or the arrows) held.
    pub(super) left: bool,
    pub(super) right: bool,
    /// The pointer over the picture (0..1 across), and when it last moved.
    pointer: Option<f32>,
    pointer_at: f32,
    pub(super) staring: bool,
    pub(super) on_ring: bool,
    off_ring: f32,
    /// On the ring without pushing away (s): dozing.
    dozing: f32,
    dozed: bool,
    /// This stare (s), all of them (s), the next of Chiquita's lines about it and when she last
    /// said one.
    stare_run: f32,
    pub(super) stare_total: f32,
    stare_line: usize,
    stare_said: bool,
    stare_line_at: f32,
    /// Until then the ring pulls harder (a whisper, an order), or has lost its rhythm (a mockery).
    burst: f32,
    stutter: f32,
    /// The pendulum's own clock.
    ring_clock: f32,
    next_whisper: f32,
    /// What he whispers: words, since when, a seed for where it drifts.
    pub(super) whispers: Vec<(String, f32, u16)>,
    next_order: f32,
    /// The order open (which, since when), and the last answer (which tile, how it went: 1 good,
    /// 2 backfired; when) for the tiles' colours.
    pub(super) order: Option<(usize, f32)>,
    pub(super) answered: Option<(usize, usize, u8, f32)>,
    used_orders: Vec<usize>,
    pub(super) obeyed: u32,
    pub(super) refused: u32,
    pub(super) mocked: u32,
    pub(super) backfired: u32,
    ignored: u32,
    /// Next close-up allowed (stage 0).
    close_at: f32,
    /// Chiquita: her next number, covering the ring until, hypnotised until, the next shout.
    next_gag: f32,
    pub(super) covered_until: f32,
    pub(super) dazed_until: f32,
    shout_at: f32,
    /// Her lines, the stamps and the narration waiting for their moment.
    later: Vec<(f32, &'static str, mascot::Cue)>,
    captions: Vec<(f32, &'static str)>,
    stamps: Vec<(f32, &'static str, [u8; 3], f32)>,
    /// Smelling salts carried and used, the amulet worn, when salts were last sniffed.
    pub(super) salts: u32,
    pub(super) salts_used: u32,
    pub(super) amulet: bool,
    pub(super) salt_at: f32,
    /// The mouth: the take, which takes she has tried, lucidity, pleasure, the last E.
    pub(super) pose: usize,
    tried: [bool; 2],
    pub(super) lucid: f32,
    pub(super) pleasure: f32,
    pub(super) mash_at: f32,
    next_line: f32,
    next_hint: f32,
    /// The clips: those in line, the loop to stay in, which of its clips comes next.
    queue: VecDeque<&'static str>,
    stay: &'static [&'static str],
    stay_i: usize,
    pub(super) end: Option<End>,
}

impl Default for HipnoGame {
    fn default() -> Self {
        Self {
            stage: 0,
            trance: 0.0,
            awake: 0.0,
            gaze: 0.2,
            gaze_vel: 0.0,
            left: false,
            right: false,
            pointer: None,
            pointer_at: -10.0,
            staring: false,
            on_ring: false,
            off_ring: 0.0,
            dozing: 0.0,
            dozed: false,
            stare_run: 0.0,
            stare_total: 0.0,
            stare_line: 0,
            stare_said: false,
            stare_line_at: -10.0,
            burst: 0.0,
            stutter: 0.0,
            ring_clock: 0.0,
            next_whisper: 9.0,
            whispers: Vec::new(),
            next_order: 14.0,
            order: None,
            answered: None,
            used_orders: Vec::new(),
            obeyed: 0,
            refused: 0,
            mocked: 0,
            backfired: 0,
            ignored: 0,
            close_at: 12.0,
            next_gag: 24.0,
            covered_until: 0.0,
            dazed_until: 0.0,
            shout_at: 9.0,
            later: Vec::new(),
            captions: Vec::new(),
            stamps: Vec::new(),
            salts: 0,
            salts_used: 0,
            amulet: false,
            salt_at: -10.0,
            pose: 0,
            tried: [true, false],
            lucid: 0.0,
            pleasure: 0.0,
            mash_at: -10.0,
            next_line: 0.0,
            next_hint: 0.0,
            queue: VecDeque::new(),
            stay: &["l0"],
            stay_i: 0,
            end: None,
        }
    }
}

impl HipnoGame {
    /// The ring: where its string hangs from and where it is now (the picture's side is 1).
    pub(super) fn ring(&self) -> ([f32; 2], [f32; 2]) {
        let st = self.stage.min(3) as f32;
        let w = 1.5 + st * 0.45;
        let amp = 0.55 + st * 0.06;
        let a = (self.ring_clock * w).sin() * amp;
        let pivot = [0.6, -0.02];
        (
            pivot,
            [
                pivot[0] + a.sin() * RING_LENGTH,
                pivot[1] + a.cos() * RING_LENGTH,
            ],
        )
    }

    /// How its face turns to us as it spins on the string (0 edge-on, 1 full face).
    pub(super) fn ring_face(&self) -> f32 {
        (self.ring_clock * 3.0).cos().abs() * 0.8 + 0.2
    }

    pub(super) fn covered(&self, now: f32) -> bool {
        now < self.covered_until
    }

    pub(super) fn mouth(&self) -> bool {
        self.stage >= 4
    }

    /// The pointer is over the picture at `x` (0..1 across); `moved` when it moved this frame.
    pub(super) fn point(&mut self, x: Option<f32>, moved: bool, now: f32) {
        self.pointer = x;
        if moved && x.is_some() {
            self.pointer_at = now;
        }
    }

    /// The stage the trance has reached.
    fn stage_of(trance: f32) -> usize {
        STAGES.iter().rposition(|s| trance >= s.at).unwrap_or(0)
    }

    /// The orders that can come at this stage, not yet given (all again once all have been).
    fn next_order_index(&mut self, roll: f32) -> usize {
        let open: Vec<usize> = (0..ORDERS.len())
            .filter(|i| ORDERS[*i].from <= self.stage && !self.used_orders.contains(i))
            .collect();
        let open = if open.is_empty() {
            self.used_orders.clear();
            (0..ORDERS.len())
                .filter(|i| ORDERS[*i].from <= self.stage)
                .collect()
        } else {
            open
        };
        let i = open[((roll * open.len() as f32) as usize).min(open.len() - 1)];
        self.used_orders.push(i);
        i
    }

    /// The order's three tiles.
    pub(super) fn order_words(&self) -> Option<[&'static str; 3]> {
        let (i, _) = self.order.or(self.answered.map(|(i, _, _, _)| (i, 0.0)))?;
        let o = &ORDERS[i];
        Some([o.obey.0, o.refuse.0, o.mock.0])
    }

    pub(super) fn stage_name(&self) -> (&'static str, &'static str) {
        let s = &STAGES[self.stage.min(4)];
        (s.name, s.hint)
    }

    pub(super) fn tint(&self) -> [u8; 3] {
        STAGES[self.stage.min(4)].tint
    }

    pub(super) fn pose_name(&self, i: usize) -> &'static str {
        POSES[i.min(1)].0
    }

    /// The stage marks on the vial.
    pub(super) fn marks() -> Vec<u16> {
        STAGES
            .iter()
            .skip(1)
            .map(|s| (s.at * 1000.0) as u16)
            .collect()
    }
}

impl PlayScene {
    /// The scene opens: the ring already swinging, his first word, Chiquita furious.
    pub(super) fn open_hipno(&mut self) {
        self.reel.play("l0", 0.0, 0.0);
        self.hipno.stay = &["l0"];
        self.talk.say(
            Voice::Mago,
            "—Mire el anillo, Majestad… solo un momentito. Jajaja.",
        );
        self.chiquita(
            "—¡Que lo de «no demasiado» era un NO! ¡Ni se te ocurra mirar ese anillo!",
            mascot::Cue::Scold,
            true,
        );
    }

    /// What she carries from his own stall (the App fills it in when the scene opens).
    pub(super) fn hipno_items(&mut self, salts: u32, amulet: bool) {
        self.hipno.salts = salts;
        self.hipno.amulet = amulet;
        if amulet {
            self.hipno.later.push((
                9.5,
                "—Llevas el amuleto que le compraste A ÉL. Es de latón. …Algo hará, supongo.",
                mascot::Cue::Talk,
            ));
        }
    }

    /// The clips play on: a clip that does not loop has played out, so the next in line plays (or
    /// the stage's loop); a loop gives way at once to what is waiting.
    pub(super) fn hipno_reel(&mut self) {
        let now = self.clock;
        let looping = self.reel.shots[self.reel.on].looping;
        let next = if looping && !self.hipno.queue.is_empty() {
            self.hipno.queue.pop_front()
        } else if !looping && self.reel.done() {
            self.hipno.queue.pop_front().or_else(|| {
                let h = &mut self.hipno;
                if h.stay.is_empty() {
                    return None;
                }
                let name = h.stay[h.stay_i % h.stay.len()];
                h.stay_i += 1;
                Some(name)
            })
        } else {
            None
        };
        if let Some(name) = next {
            let fade = seam(self.reel.name(), name);
            self.reel.play(name, fade, now);
        }
    }

    /// Each step while it plays.
    pub(super) fn step_hipno(&mut self, dt: f32) {
        let now = self.clock;
        // Chiquita's lines and the stamps that wait for their moment.
        let mut due = Vec::new();
        self.hipno.later.retain(|(at, line, cue)| {
            if now >= *at {
                due.push((*line, *cue));
                false
            } else {
                true
            }
        });
        for (line, cue) in due {
            self.chiquita(line, cue, true);
        }
        let mut stamps = Vec::new();
        self.hipno.stamps.retain(|(at, words, color, jolt)| {
            if now >= *at {
                stamps.push((*words, *color, *jolt));
                false
            } else {
                true
            }
        });
        for (words, color, jolt) in stamps {
            self.stamp(words, color);
            self.jolt(jolt);
            self.flash(color, 0.6);
        }
        let mut captions = Vec::new();
        self.hipno.captions.retain(|(at, text)| {
            if now >= *at {
                captions.push(*text);
                false
            } else {
                true
            }
        });
        for text in captions {
            self.talk.say(Voice::Narration, text);
        }
        self.hipno.whispers.retain(|(_, at, _)| now - at < 5.2);
        if self.phase != Phase::Play {
            return;
        }
        if self.hipno.mouth() {
            self.step_mouth(dt);
        } else {
            self.step_resist(dt);
        }
    }

    fn step_resist(&mut self, dt: f32) {
        let now = self.clock;
        let staring = self.holding;
        let covered = self.hipno.covered(now);
        let st = self.hipno.stage;
        {
            let h = &mut self.hipno;
            h.ring_clock += dt * if now < h.stutter { 0.45 } else { 1.0 };
            let (_, ring) = h.ring();
            // Her gaze: a spring pulling it to the ring, against what the keys or the pointer push.
            let mut push = (h.right as i32 - h.left as i32) as f32;
            if push == 0.0 {
                if let Some(px) = h.pointer.filter(|_| now - h.pointer_at < 1.2) {
                    push = ((px - h.gaze) / 0.148).clamp(-1.0, 1.0);
                }
            }
            let mut pull = if covered {
                0.0
            } else {
                (2.4 + st as f32 * 1.25 + if now < h.burst { 4.0 } else { 0.0 })
                    * if h.amulet { 0.75 } else { 1.0 }
            };
            h.staring = staring;
            if staring && !covered {
                pull = 14.0;
                push = 0.0;
            }
            let acc = (ring[0] - h.gaze) * pull + push;
            h.gaze_vel = (h.gaze_vel + acc * dt) * (-3.2 * dt).exp();
            h.gaze = (h.gaze + h.gaze_vel * dt).clamp(0.03, 0.97);
            h.on_ring = !covered && (ring[0] - h.gaze).abs() < ON_RING;
            h.off_ring = if h.on_ring { 0.0 } else { h.off_ring + dt };
            h.dozing = if h.on_ring && push == 0.0 && !staring {
                h.dozing + dt
            } else {
                0.0
            };
            // The trance and the waking.
            let floor = STAGES[st].at;
            if staring && !covered {
                h.trance += 0.15 * dt;
                h.awake = (h.awake - 0.05 * dt).max(0.0);
            } else if h.on_ring {
                h.trance += (0.075 + st as f32 * 0.012) * dt;
                h.awake = (h.awake - 0.02 * dt).max(0.0);
            } else {
                h.trance = (h.trance - 0.012 * dt).max(floor);
                h.awake +=
                    (0.022 * (1.0 - h.trance * 0.75) + if covered { 0.03 } else { 0.0 }) * dt;
            }
            if now < h.dazed_until {
                h.trance += 0.01 * dt;
            }
            h.trance = h.trance.min(1.0);
            h.awake = h.awake.min(1.0);
        }
        self.heat = self.hipno.trance;

        // Staring on purpose: Chiquita has something to say about it, more fed up each time.
        if staring && !covered {
            let h = &mut self.hipno;
            h.stare_run += dt;
            h.stare_total += dt;
            if h.stare_run >= 1.0 && !h.stare_said && now - h.stare_line_at > 4.0 {
                h.stare_said = true;
                h.stare_line_at = now;
                let k = h.stare_line;
                h.stare_line += 1;
                let (line, cue) = if k < STARED.len() {
                    STARED[k]
                } else {
                    *self.dice.pick(&STARED_MORE)
                };
                self.chiquita(line, cue, true);
            }
        } else {
            self.hipno.stare_run = 0.0;
            self.hipno.stare_said = false;
        }
        // On the ring by itself: she shouts; left there without a fight, she asks if you sleep.
        if self.hipno.on_ring && !staring && now >= self.hipno.shout_at {
            let line = *self.dice.pick(&ON_RING_SHOUTS);
            self.chiquita(line, mascot::Cue::Scold, false);
            self.hipno.shout_at = now + 6.0;
        }
        if self.hipno.dozing > 7.0 && !self.hipno.dozed {
            self.hipno.dozed = true;
            self.chiquita(
                "—¿Hola? ¿Te has dormido de pie? ¡Empuja, que se te va la mirada!",
                mascot::Cue::Scold,
                true,
            );
        }
        if !self.hipno.on_ring {
            self.hipno.dozed = false;
        }

        // A close look at her fighting it, when she has kept her eyes off it a while (standing).
        if st == 0
            && self.hipno.off_ring > 2.5
            && now > self.hipno.close_at
            && self.reel.name() == "l0"
            && self.hipno.queue.is_empty()
        {
            self.hipno.close_at = now + 14.0;
            self.hipno.queue.push_back("l0c");
            let line = *self.dice.pick(&CLOSE_UPS);
            self.talk.say(Voice::Queen, line);
        }

        // He whispers: the ring pulls harder for a moment.
        if now >= self.hipno.next_whisper {
            let line = *self.dice.pick(STAGES[st].whispers);
            let seed = (self.dice.roll() * 1000.0) as u16;
            let h = &mut self.hipno;
            h.whispers.push((line.to_owned(), now, seed));
            if h.whispers.len() > 2 {
                h.whispers.remove(0);
            }
            h.burst = now + 1.3;
            h.next_whisper = now + 5.5 + 3.0 * self.dice.roll();
            self.sound(Sfx::RuneChime, 0.0, 0.25);
        }

        // His orders.
        match self.hipno.order {
            None if now >= self.hipno.next_order && self.hipno.queue.is_empty() => {
                let roll = self.dice.roll();
                let i = self.hipno.next_order_index(roll);
                self.hipno.order = Some((i, now));
                self.hipno.answered = None;
                self.hipno.burst = now + 1.0;
                self.talk.reply(Voice::Mago, ORDERS[i].say);
                self.sound(Sfx::SpellCharge, 0.0, 0.35);
            }
            Some((_, since)) if now - since >= ORDER_SECS => {
                let h = &mut self.hipno;
                h.order = None;
                h.answered = None;
                h.ignored += 1;
                h.next_order = now + 10.0;
                h.awake = (h.awake + 0.03).min(1.0);
                self.talk.say(
                    Voice::Narration,
                    "La Reina deja pasar la orden sin contestar.",
                );
                self.talk
                    .say(Voice::Mago, "—…¿Majestad? ¿Hola? Jajaja… ¿Me oye?");
            }
            _ => {}
        }

        // Chiquita's numbers: in front of the ring, or under its spell herself.
        if now >= self.hipno.next_gag && self.hipno.order.is_none() {
            self.hipno.next_gag = now + 16.0 + 8.0 * self.dice.roll();
            if self.dice.roll() < 0.6 {
                self.hipno.covered_until = now + mascot::COVER_SECS;
                let line = *self.dice.pick(&COVERS);
                self.chiquita(line, mascot::Cue::Cover, true);
                if staring {
                    self.hipno.later.push((
                        now + 1.2,
                        "—¡JA! Ahora mira el anillo, si puedes.",
                        mascot::Cue::Cover,
                    ));
                }
            } else {
                self.hipno.dazed_until = now + mascot::DAZED_SECS;
                let line = *self.dice.pick(&DAZED);
                self.chiquita(line, mascot::Cue::Dazed, true);
                let after = *self.dice.pick(&UNDAZED);
                self.hipno.later.push((
                    now + mascot::DAZED_SECS + 0.1,
                    after,
                    mascot::Cue::Flustered,
                ));
            }
        }

        // Deeper: the next stage, one at a time, and only once the clips have taken her into the
        // last one (until then the trance waits just under the next mark).
        let caught_up = self.hipno.queue.is_empty() && self.reel.shots[self.reel.on].looping;
        if !caught_up && st < 4 {
            let cap = STAGES[st + 1].at - 0.002;
            self.hipno.trance = self.hipno.trance.min(cap);
            self.heat = self.hipno.trance;
        } else if HipnoGame::stage_of(self.hipno.trance) > st {
            self.hipno_stage(st + 1);
            return;
        }
        // Awake: she gives him what he has earned.
        if self.hipno.awake >= 1.0 {
            self.hipno_escape();
        }
    }

    /// Down to stage `n`: its clips in line, its light, what each says.
    fn hipno_stage(&mut self, n: usize) {
        let now = self.clock;
        self.hipno.queue.extend(STAGES[n].into.iter().copied());
        let s = &STAGES[n];
        self.hipno.stage = n;
        self.hipno.stay = s.stay;
        self.hipno.stay_i = 0;
        self.flash(s.tint, 0.9);
        self.jolt(9.0);
        self.stamp(s.stamp, s.tint);
        self.sound(Sfx::SealOpen, 0.0, 0.45);
        self.talk.reply(Voice::QueenTrance, s.queen);
        self.talk.say(Voice::Mago, s.mage);
        self.hipno.captions.push((now + 1.8, s.caption));
        let staring = self.holding;
        if staring && n < 4 {
            self.chiquita(
                "—¡Lo has hecho TÚ! ¡Yo lo he visto! ¡Lo estabas mirando!",
                mascot::Cue::Scold,
                true,
            );
        } else {
            self.chiquita(s.chiquita.0, s.chiquita.1, true);
        }
        if n >= 4 {
            let h = &mut self.hipno;
            h.order = None;
            h.answered = None;
            h.pose = 0;
            h.tried = [true, false];
            h.pleasure = 0.0;
            h.lucid = 0.0;
            h.next_line = now + 7.0;
            h.next_hint = now + 10.0;
            h.whispers.clear();
            self.heat = 0.0;
        }
    }

    /// 1–3 (or a tile): his order's answer.
    pub(super) fn hipno_answer(&mut self, pick: usize) {
        let now = self.clock;
        let Some((i, _)) = self.hipno.order else {
            return;
        };
        let o = &ORDERS[i];
        let roll = self.dice.roll();
        let st = self.hipno.stage;
        let on_ring = self.hipno.on_ring || self.holding;
        self.hipno.order = None;
        self.hipno.next_order = now + 11.0 + 5.0 * roll;
        let trance_voice = if st >= 1 {
            Voice::QueenTrance
        } else {
            Voice::Queen
        };
        match pick {
            0 => {
                let h = &mut self.hipno;
                h.obeyed += 1;
                h.trance = (h.trance + 0.08).min(0.8).max(h.trance);
                h.answered = Some((i, 0, 1, now));
                let n = h.obeyed as usize;
                self.flash([0xd6, 0x5c, 0xff], 0.5);
                self.talk.reply(trance_voice, o.obey.1);
                let line = *self.dice.pick(&MAGE_OBEYED);
                self.talk.say(Voice::Mago, line);
                let (line, cue) = OBEYED[(n - 1).min(OBEYED.len() - 1)];
                self.chiquita(line, cue, n <= 2);
            }
            1 => {
                let h = &mut self.hipno;
                h.refused += 1;
                h.awake = (h.awake + 0.07).min(1.0);
                h.burst = now + 1.6;
                h.answered = Some((i, 1, 1, now));
                self.flash([0x5a, 0xff, 0xc8], 0.4);
                self.talk.reply(Voice::QueenAngry, o.refuse.1);
                let line = *self.dice.pick(&MAGE_REFUSED);
                self.talk.say(Voice::Mago, line);
                if st == 0
                    && now > self.hipno.close_at - 8.0
                    && self.reel.name() == "l0"
                    && self.hipno.queue.is_empty()
                {
                    self.hipno.close_at = now + 14.0;
                    self.hipno.queue.push_back("l0c");
                }
            }
            _ => {
                if !on_ring {
                    let h = &mut self.hipno;
                    h.mocked += 1;
                    h.awake = (h.awake + 0.12).min(1.0);
                    h.stutter = now + 2.5;
                    h.burst = 0.0;
                    h.answered = Some((i, 2, 1, now));
                    let first = h.mocked == 1;
                    self.flash([0xff, 0xd2, 0x5a], 0.55);
                    self.stamp("¡ZAS!", [0xff, 0xd2, 0x5a]);
                    self.jolt(5.0);
                    self.talk.reply(Voice::QueenAngry, o.mock.1);
                    let line = *self.dice.pick(&MAGE_MOCKED);
                    self.talk.say(Voice::Mago, line);
                    if first {
                        self.chiquita("—¡JA! ¡JAJAJA! ¡Esa es mi Reina!", mascot::Cue::Swoop, true);
                    } else {
                        let line = *self.dice.pick(&MOCKED);
                        self.chiquita(line, mascot::Cue::Cheer, false);
                    }
                } else {
                    let h = &mut self.hipno;
                    h.backfired += 1;
                    h.trance = (h.trance + 0.1).min(0.8).max(h.trance);
                    h.answered = Some((i, 2, 2, now));
                    self.flash([0xd6, 0x5c, 0xff], 0.7);
                    self.stamp("¡PILLADA!", [0xd6, 0x5c, 0xff]);
                    self.talk.reply(trance_voice, o.mock.2);
                    let line = *self.dice.pick(&MAGE_BACKFIRED);
                    self.talk.say(Voice::Mago, line);
                    self.chiquita(
                        "—¿Burlarte MIRÁNDOLE el anillo? ¡Así no! ¡Primero aparta la mirada!",
                        mascot::Cue::Scold,
                        true,
                    );
                }
            }
        }
    }

    /// Q: a sniff of his own smelling salts.
    pub(super) fn hipno_salts(&mut self) {
        let now = self.clock;
        if self.phase != Phase::Play
            || self.hipno.salts <= self.hipno.salts_used
            || now - self.hipno.salt_at < 2.5
        {
            return;
        }
        let h = &mut self.hipno;
        h.salts_used += 1;
        h.salt_at = now;
        if h.mouth() {
            h.lucid = (h.lucid + 0.4).min(1.0);
        } else {
            h.awake = (h.awake + 0.25).min(1.0);
            h.trance = (h.trance - 0.04).max(STAGES[h.stage].at);
        }
        self.flash([0x5a, 0xff, 0xc8], 0.9);
        self.stamp("¡SALES!", [0x5a, 0xff, 0xc8]);
        self.jolt(7.0);
        self.talk
            .say(Voice::Queen, "—¡Puaj! …¡Ngh! ¡Eso pica hasta en el alma!");
        self.chiquita(
            "—¡Uf! ¡Hasta yo me he despertado! Y eso que soy un tatuaje.",
            mascot::Cue::Flustered,
            true,
        );
    }

    /// E: with her mouth busy, a jolt of lucidity (it also cools him down a little).
    pub(super) fn hipno_mash(&mut self) {
        if self.phase != Phase::Play || !self.hipno.mouth() {
            return;
        }
        let h = &mut self.hipno;
        h.lucid = (h.lucid + 0.12).min(1.0);
        h.pleasure = (h.pleasure - 0.025).max(0.0);
        h.mash_at = self.clock;
        self.flash([0x5a, 0xff, 0xc8], 0.15);
    }

    /// A/D or 1–2 with her mouth busy: the other take.
    pub(super) fn hipno_pose(&mut self, to: usize) {
        let now = self.clock;
        if self.phase != Phase::Play || !self.hipno.mouth() || to == self.hipno.pose || to > 1 {
            return;
        }
        // Still on its way in (c5): it waits for the first take.
        if self.reel.name() == "c5" {
            return;
        }
        let h = &mut self.hipno;
        h.pose = to;
        h.queue.clear();
        h.stay = POSES[to].1;
        h.stay_i = 1;
        let first = !h.tried[to];
        h.tried[to] = true;
        let clip = POSES[to].1[0];
        let fade = seam(self.reel.name(), clip).min(0.3);
        self.reel.play(clip, fade, now);
        self.flash([0xff, 0x9a, 0xd0], 0.35);
        if first {
            self.talk.reply(Voice::QueenTrance, POSES[to].2);
        }
        self.hipno.next_line = now + 7.0;
    }

    fn step_mouth(&mut self, dt: f32) {
        let now = self.clock;
        let hard = self.holding;
        {
            let h = &mut self.hipno;
            h.pleasure = (h.pleasure + if hard { 0.07 } else { 0.035 } * dt).min(1.0);
            // A full bar stays full until the bite comes.
            if h.lucid < 1.0 {
                h.lucid = (h.lucid - 0.03 * dt).max(0.0);
            }
        }
        self.heat = self.hipno.pleasure;
        self.reel.rate = if hard { 1.3 } else { 1.0 };
        if now >= self.hipno.next_hint {
            self.hipno.next_hint = now + 14.0 + 6.0 * self.dice.roll();
            let line = *self.dice.pick(&BITE_HINTS);
            self.chiquita(line, mascot::Cue::Scold, true);
        }
        if now >= self.hipno.next_line {
            self.hipno.next_line = now + 6.0 + 3.0 * self.dice.roll();
            let (voice, line) = *self.dice.pick(&MOUTH_LINES);
            self.talk.say(voice, line);
        }
        if self.hipno.lucid >= 1.0 {
            self.hipno_bite();
        } else if self.hipno.pleasure >= 1.0 {
            self.hipno_climax();
        }
    }

    /// Awake: the punch that belongs to the stage she woke in.
    fn hipno_escape(&mut self) {
        let now = self.clock;
        let st = self.hipno.stage.min(3);
        let clip = *self.dice.pick(STAGES[st].escape);
        let h = &mut self.hipno;
        h.queue.clear();
        h.stay = &[];
        h.order = None;
        h.answered = None;
        h.whispers.clear();
        h.end = Some(End::Escape);
        let fade = seam(self.reel.name(), clip).min(0.3);
        self.reel.play(clip, fade, now);
        self.phase = Phase::Relax(now + 6.5);
        self.phase_since = now;
        self.flash([0x5a, 0xff, 0xc8], 1.0);
        self.jolt(16.0);
        self.stamp("¡DESPIERTA!", [0x5a, 0xff, 0xc8]);
        self.talk
            .reply(Voice::QueenAngry, "—…¿Qué? ¡¿QUÉ?! ¡Viejo verde!");
        self.talk.say(Voice::Narration, ESCAPED[st]);
        self.talk.say(
            Voice::Mago,
            "—¡Ay! ¡Mi nariz! ¡Era una broma! ¡Una broma artística!",
        );
        self.sound(Sfx::Shockwave, 0.0, 0.6);
        // Chiquita jumps in for one of her own.
        self.chiquita("—¡Y ESTE VA DE MI PARTE!", mascot::Cue::Jab, true);
        self.hipno
            .stamps
            .push((now + mascot::JAB_HIT, "¡PAM!", [0xff, 0xd2, 0x5a], 14.0));
        self.sound(Sfx::BoneCrack, mascot::JAB_HIT, 0.55);
        self.hipno.later.push((
            now + 3.2,
            "—¡LO SABÍA! ¡Le he pisado el anillo! ¡Esa es mi Reina!",
            mascot::Cue::Cheer,
        ));
    }

    /// Lucid with her mouth full: she bites.
    fn hipno_bite(&mut self) {
        let now = self.clock;
        let chain: &'static [&'static str] = if self.reel.name() == "c5" {
            &["bb0", "bl0", "bb1"]
        } else {
            &["bb2", "bl2", "bb1"]
        };
        let h = &mut self.hipno;
        h.queue = chain[1..].iter().copied().collect();
        h.stay = &[];
        h.end = Some(End::Bite);
        let fade = seam(self.reel.name(), chain[0]).min(0.3);
        self.reel.play(chain[0], fade, now);
        self.reel.rate = 1.0;
        self.phase = Phase::Relax(now + 12.0);
        self.phase_since = now;
        self.flash([0x5a, 0xff, 0xc8], 1.0);
        self.jolt(14.0);
        self.stamp("¡ÑAM!", [0xff, 0x5a, 0x6a]);
        self.sound(Sfx::BoneCrack, 0.25, 0.7);
        self.talk.reply(
            Voice::QueenAngry,
            "—…¿Qué tengo en la boca? …¡¿QUÉ TENGO EN LA BOCA?!",
        );
        self.talk.say(Voice::Narration, "Y muerde.");
        self.talk.say(
            Voice::Mago,
            "—¡¡AAAAY!! ¡Suelte! ¡SUELTE, POR TODOS LOS ANILLOS!",
        );
        self.chiquita(
            "—¡ÑAM! ¡Esa es mi Reina! ¡Muerde, muerde!",
            mascot::Cue::Swoop,
            true,
        );
    }

    /// All the way under.
    fn hipno_climax(&mut self) {
        let now = self.clock;
        let h = &mut self.hipno;
        h.queue.clear();
        h.stay = &[];
        h.end = Some(End::Fall);
        let fade = seam(self.reel.name(), "c9").min(0.4);
        self.reel.play("c9", fade, now);
        self.reel.rate = 1.0;
        self.climax_shot = "c9";
        self.relax_shot = None;
        self.heat = 1.0;
        self.phase = Phase::Climax;
        self.phase_since = now;
        self.talk
            .say(Voice::Narration, "El anillo deja de girar. La Reina, no.");
        self.sound(Sfx::SealOpen, 1.9, 0.6);
        self.chiquita(
            "—…Yo esto no lo he visto. Y si lo he visto, era un sueño.",
            mascot::Cue::Away,
            true,
        );
    }

    /// After the fall: his last word, hers.
    pub(super) fn hipno_after(&mut self) {
        let now = self.clock;
        self.phase = Phase::Relax(now + 4.5);
        self.phase_since = now;
        self.talk.reply(
            Voice::Mago,
            "—Jajaja… Vuelva cuando quiera, Majestad. Gratis. Siempre gratis.",
        );
        self.talk.say(Voice::QueenTrance, "—…Sí, maestro.");
    }

    /// Visual QA: Chiquita's next number now.
    pub(super) fn hipno_force_gag(&mut self, cover: bool) {
        let now = self.clock;
        self.hipno.next_gag = now + 30.0;
        if cover {
            self.hipno.covered_until = now + mascot::COVER_SECS;
            self.chiquita(COVERS[0], mascot::Cue::Cover, true);
        } else {
            self.hipno.dazed_until = now + mascot::DAZED_SECS;
            self.chiquita(DAZED[0], mascot::Cue::Dazed, true);
            self.hipno.later.push((
                now + mascot::DAZED_SECS + 0.1,
                UNDAZED[0],
                mascot::Cue::Flustered,
            ));
        }
    }

    /// Whether the clips in line have all played (the closing card waits for them).
    pub(super) fn hipno_reel_done(&self) -> bool {
        self.hipno.queue.is_empty() && self.reel.done()
    }

    /// How to play.
    pub(super) fn hipno_rules(&self) -> Vec<(&'static str, String)> {
        let mut rows = vec![
            ("A · D", "Aparta la mirada (o con el ratón sobre el cuadro). El anillo tira de ella; cuanto más hondo, más tira.".to_owned()),
            ("Despertar", "Con la mirada lejos del anillo sube. Llénalo y le das su merecido.".to_owned()),
            ("1 2 3", "Sus órdenes: obedecer hunde, negarse despierta y burlarte despierta más… si no estás mirando.".to_owned()),
            ("Espacio", "Mirar a propósito. Caer también es un final.".to_owned()),
            ("E", "Con la boca ocupada: machácala para despertar… y morder.".to_owned()),
        ];
        if self.hipno.salts > 0 {
            rows.push((
                "Q",
                format!(
                    "Sales despertadoras: un golpe de lucidez (llevas {}).",
                    self.hipno.salts
                ),
            ));
        }
        rows
    }

    /// The overlay's part: the ring, her gaze, the whispers, the order, the second column.
    pub(super) fn hipno_view(&self) -> map_overlay::HypnoView {
        let now = self.clock;
        let h = &self.hipno;
        let (pivot, ring) = h.ring();
        let pm = |v: f32| (v.clamp(-0.2, 1.2) * 1000.0 + 200.0) as u16;
        let ms = |t: f32| ((now - t).max(0.0) * 1000.0) as u32;
        let mode = if h.end.is_some() || self.phase != Phase::Play {
            2
        } else if h.mouth() {
            1
        } else {
            0
        };
        let order = h
            .order
            .map(|(i, since)| (ORDERS[i].say.to_owned(), ms(since)));
        map_overlay::HypnoView {
            mode,
            stage: h.stage as u8,
            pivot: [pm(pivot[0]), pm(pivot[1])],
            ring: [pm(ring[0]), pm(ring[1])],
            turn: (h.ring_face() * 1000.0) as u16,
            burn: if now < h.burst {
                1000
            } else {
                (((h.burst - now) / 0.6 + 1.0).clamp(0.0, 1.0) * 1000.0) as u16
            },
            trance: (h.trance.clamp(0.0, 1.0) * 1000.0) as u16,
            gaze: pm(h.gaze),
            on_ring: h.on_ring,
            staring: h.staring && !h.covered(now),
            covered: h.covered(now),
            dazed: now < h.dazed_until,
            whispers: h
                .whispers
                .iter()
                .map(|(text, at, seed)| (text.clone(), ms(*at), *seed))
                .collect(),
            order,
            awake: ((if h.mouth() { h.lucid } else { h.awake }).clamp(0.0, 1.0) * 1000.0) as u16,
            awake_label: if h.mouth() { "LUCIDEZ" } else { "DESPERTAR" }.to_owned(),
            mash_ms: ms(h.mash_at),
            salts: h.salts.saturating_sub(h.salts_used).min(255) as u8,
            amulet: h.amulet,
            salt_ms: ms(h.salt_at),
        }
    }

    /// The tiles of the order open (or of the one just answered, showing how it went).
    pub(super) fn hipno_chips(&self) -> Vec<map_overlay::GameChip> {
        let h = &self.hipno;
        let Some(words) = h.order_words() else {
            return Vec::new();
        };
        let tags = ["OBEDECER", "NEGARSE", "BURLARTE · SIN MIRAR"];
        let tones = [1, 2, 3];
        words
            .iter()
            .enumerate()
            .map(|(k, w)| map_overlay::GameChip {
                label: (*w).to_owned(),
                state: match h.answered {
                    None => 0,
                    Some((_, pick, 1, _)) if pick == k => 1,
                    Some((_, pick, _, _)) if pick == k => 2,
                    Some(_) => 3,
                },
                tag: tags[k].to_owned(),
                tone: tones[k],
            })
            .collect()
    }

    /// The tiles stay up a moment after the answer, to show how it went.
    pub(super) fn hipno_chips_up(&self) -> bool {
        self.hipno.order.is_some()
            || self
                .hipno
                .answered
                .is_some_and(|(_, _, _, at)| self.clock - at < 1.2)
    }

    /// The keys on the plate.
    pub(super) fn hipno_keys(&self) -> Vec<(String, String, bool)> {
        let now = self.clock;
        let h = &self.hipno;
        let mut keys = Vec::new();
        if self.phase != Phase::Play {
            return keys;
        }
        if h.mouth() {
            keys.push((
                "E".to_owned(),
                "despertar".to_owned(),
                now - h.mash_at < 0.18,
            ));
            keys.push(("ESPACIO".to_owned(), "más fuerte".to_owned(), self.holding));
        } else {
            keys.push(("A D".to_owned(), "apartar".to_owned(), h.left || h.right));
            keys.push(("ESPACIO".to_owned(), "mirarlo".to_owned(), h.staring));
        }
        let left = h.salts.saturating_sub(h.salts_used);
        if left > 0 {
            keys.push((
                "Q".to_owned(),
                format!("sales ×{left}"),
                now - h.salt_at < 0.5,
            ));
        }
        keys
    }

    /// What is happening, over the keys.
    pub(super) fn hipno_phase(&self) -> String {
        let now = self.clock;
        let h = &self.hipno;
        match self.phase {
            Phase::Play if h.mouth() && self.reel.name() == "c5" => {
                "La Reina abre la boca…".to_owned()
            }
            Phase::Play if h.mouth() && h.lucid > 0.75 => "¡Ya casi! ¡Muerde!".to_owned(),
            Phase::Play if h.mouth() => format!("Con la boca · {}", POSES[h.pose].0),
            Phase::Play if h.order.is_some() => "¡Una orden! Elige qué contestas".to_owned(),
            Phase::Play if h.covered(now) => "¡Chiquita te tapa el anillo!".to_owned(),
            Phase::Play if now < h.dazed_until => "Chiquita se ha hipnotizado…".to_owned(),
            Phase::Play if h.staring => "Lo estás mirando a propósito…".to_owned(),
            Phase::Play => {
                let (name, hint) = h.stage_name();
                format!("{name} · {hint}")
            }
            Phase::Climax | Phase::Cut(_) => "La Reina cae del todo…".to_owned(),
            _ => match h.end {
                Some(End::Escape) => "¡Despierta!".to_owned(),
                Some(End::Bite) => "¡Le muerde!".to_owned(),
                _ => "Calma".to_owned(),
            },
        }
    }

    /// How it ended: the card's title, what it does to the story, the last word, the line of how
    /// it went.
    pub(super) fn hipno_ending(
        &self,
        done: bool,
    ) -> (
        &'static str,
        Vec<(&'static str, i32)>,
        (Voice, &'static str),
        String,
    ) {
        let h = &self.hipno;
        let mut how = format!(
            "Órdenes: {} sí · {} no · {} burlas",
            h.obeyed, h.refused, h.mocked
        );
        if h.stare_total >= 1.0 {
            how.push_str(&format!(" · mirándolo {} s", h.stare_total.round() as i32));
        }
        if !done {
            return (
                "Huyes del claro",
                vec![("fear", 1)],
                (
                    Voice::Mago,
                    "—¡Majestad! ¡Que no he cobrado! …Ah, no, que era gratis.",
                ),
                how,
            );
        }
        match h.end {
            Some(End::Escape) => (
                "¡Despiertas!",
                vec![("dignity", 1), ("chiquita_afecto", 1), ("mago_derrotas", 1)],
                (
                    Voice::Chiquita,
                    "—Puñetazo y a casa. El viejo, sin anillo y sin dignidad.",
                ),
                how,
            ),
            Some(End::Bite) => (
                "¡Le muerdes!",
                vec![
                    ("dignity", 2),
                    ("chiquita_afecto", 2),
                    ("mago_mordiscos", 1),
                ],
                (
                    Voice::Chiquita,
                    "—Se ha ido cojeando. Y sin anillo. Y sin… bueno, casi.",
                ),
                how,
            ),
            _ => {
                let quote = if self.hipno_on_purpose() {
                    (
                        Voice::ChiquitaJealous,
                        "—Lo has hecho a propósito. Lo sé. Lo SÉ.",
                    )
                } else {
                    (
                        Voice::Mago,
                        "—Vuelva cuando quiera, Majestad. Gratis. Siempre gratis.",
                    )
                };
                (
                    "Caíste del todo",
                    vec![
                        ("hunger", -6),
                        ("deseo", 3),
                        ("dignity", -1),
                        ("mago_caidas", 1),
                    ],
                    quote,
                    how,
                )
            }
        }
    }

    /// She let herself fall: she stared at it on purpose a good while, or obeyed again and again.
    pub(super) fn hipno_on_purpose(&self) -> bool {
        self.hipno.stare_total > 6.0 || self.hipno.obeyed >= 3
    }

    pub(super) fn hipno_thought(&self) -> &'static str {
        match (self.ended.unwrap_or(false), self.hipno.end) {
            (false, _) => "Ni un minuto más en ese claro.",
            (true, Some(End::Escape)) => {
                "Un viejo con un anillo. Casi me la juega un viejo con un anillo."
            }
            (true, Some(End::Bite)) => {
                "Aún tengo su sabor en la boca. Y su grito en los oídos. Lo segundo me gusta más."
            }
            (true, _) => "No me acuerdo de nada. Pero me duelen las rodillas. Y la mandíbula.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stages_follow_the_trance_and_never_go_back() {
        assert_eq!(HipnoGame::stage_of(0.0), 0);
        assert_eq!(HipnoGame::stage_of(0.19), 0);
        assert_eq!(HipnoGame::stage_of(0.2), 1);
        assert_eq!(HipnoGame::stage_of(0.65), 3);
        assert_eq!(HipnoGame::stage_of(1.0), 4);
        // Each stage's floor is where it begins: the trance never sinks under it.
        for (k, s) in STAGES.iter().enumerate() {
            assert_eq!(HipnoGame::stage_of(s.at), k);
        }
    }

    #[test]
    fn the_chained_clips_meet_without_a_fade_and_the_rest_cross() {
        // l0 → ta → l1 → tb → l2 → c3 → c4a → c4b → l3 → c5: the whole way down is seamless.
        let down = ["l0", "ta", "l1", "tb", "l2", "c3", "c4a", "c4b", "l3", "c5"];
        for w in down.windows(2) {
            assert_eq!(seam(w[0], w[1]), 0.0, "{} → {}", w[0], w[1]);
        }
        assert_eq!(seam("c7", "c8"), 0.0);
        assert_eq!(seam("c8", "c9"), 0.0);
        assert_eq!(seam("c5", "c6"), FADE, "c6 starts closer in: a crossfade");
        assert_eq!(seam("c8", "c7"), FADE);
        // Every clip the game names is one the reel opens.
        let names: Vec<&str> = SHOTS.iter().map(|(n, _)| *n).collect();
        for s in &STAGES {
            for n in s.into.iter().chain(s.stay).chain(s.escape) {
                assert!(names.contains(n), "{n}");
            }
        }
        for (_, clips, _) in &POSES {
            for n in clips.iter() {
                assert!(names.contains(n), "{n}");
            }
        }
    }

    #[test]
    fn orders_come_by_stage_and_do_not_repeat_until_all_have() {
        let mut h = HipnoGame::default();
        let mut seen = Vec::new();
        for k in 0..5 {
            let i = h.next_order_index((k as f32 * 0.37).fract());
            assert!(
                ORDERS[i].from == 0,
                "only the stage-0 orders while standing"
            );
            assert!(!seen.contains(&i), "no repeats");
            seen.push(i);
        }
        // All five stage-0 orders given: the bag refills.
        let i = h.next_order_index(0.5);
        assert!(ORDERS[i].from == 0);
        h.stage = 3;
        let deep: Vec<usize> = (0..12)
            .map(|k| h.next_order_index((k as f32 * 0.61).fract()))
            .collect();
        assert!(
            deep.iter().any(|i| ORDERS[*i].from == 3),
            "the deepest order shows up at stage 3"
        );
    }

    #[test]
    fn the_ring_swings_inside_the_picture() {
        let mut h = HipnoGame::default();
        for k in 0..400 {
            h.ring_clock = k as f32 * 0.05;
            for st in 0..4 {
                h.stage = st;
                let (_, r) = h.ring();
                assert!((0.15..=0.95).contains(&r[0]), "x {}", r[0]);
                assert!((0.2..=0.4).contains(&r[1]), "y {}", r[1]);
            }
        }
    }
}
