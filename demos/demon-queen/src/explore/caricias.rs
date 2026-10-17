//! Caricias: la noche en que Lira o Nyxara vienen al cuarto de la Reina (desde la cama, «Invitar a
//! alguien», tras su escena en `corte.md`). Se juega con el ratón como la maqueta web
//! `tools/escena-demo/tocar.html`, de la que sale todo: seis manos (caricia, dedo, agarrar,
//! pellizco, dos manos, beso), zonas con el deseo que piden y su ritmo bueno, la ropa que se quita
//! arrastrando sus lazos en orden, el deseo y la confianza, su punto débil, los sustos, una vela de
//! tres minutos y los finales (ella llega al final, ella para, la Reina para, se apaga la vela).
//! El arte es el de verdad, en capas: el cuerpo se dobla con los mismos deformadores que el
//! sombreador de la maqueta (pechos con muelle, cabeza, cadera, respiración, el pelo con su
//! máscara, las abolladuras de la mano) aplicados a una malla, y las caras se mezclan encima.
//! Al final, el clip de H3 con su sonido.
use std::path::PathBuf;

use super::ui_sfx::{self, UiSfx};
use super::*;

// ------------------------------------------------------------------ the picture's place

pub(crate) const ART_W: f32 = 1824.0;
pub(crate) const ART_H: f32 = 1024.0;
/// The art laid over the 1080p screen (as the web page lays it).
fn art_scale() -> f32 {
    (1920.0 / ART_W).min(1080.0 / ART_H)
}
fn art_origin() -> [f32; 2] {
    let k = art_scale();
    [(1920.0 - ART_W * k) * 0.5, (1080.0 - ART_H * k) * 0.5]
}
pub(crate) fn from_art(p: [f32; 2]) -> [f32; 2] {
    let (k, o) = (art_scale(), art_origin());
    [p[0] * k + o[0], p[1] * k + o[1]]
}
fn to_art(p: [f32; 2]) -> [f32; 2] {
    let (k, o) = (art_scale(), art_origin());
    [(p[0] - o[0]) / k, (p[1] - o[1]) / k]
}
/// The art's rect on the screen.
pub(crate) fn art_rect() -> [f32; 4] {
    let (k, o) = (art_scale(), art_origin());
    [o[0], o[1], ART_W * k, ART_H * k]
}

// ------------------------------------------------------------------ the zones and the hands

/// A part of her body: its name, the desire it asks first, the good pace (px/s), how much it gives
/// a second, and whether it can be kissed.
pub(crate) struct ZoneDef {
    pub id: &'static str,
    pub name: &'static str,
    min: f32,
    spd: [f32; 2],
    gain: f32,
    kiss: bool,
}

/// In the order a touch looks for them (the small ones first).
pub(crate) static ZONES: [ZoneDef; 11] = [
    ZoneDef {
        id: "labios",
        name: "Labios",
        min: 25.0,
        spd: [40.0, 300.0],
        gain: 7.0,
        kiss: true,
    },
    ZoneDef {
        id: "oreja",
        name: "Oreja",
        min: 12.0,
        spd: [40.0, 320.0],
        gain: 6.0,
        kiss: true,
    },
    ZoneDef {
        id: "mejilla",
        name: "Mejilla",
        min: 0.0,
        spd: [60.0, 500.0],
        gain: 3.0,
        kiss: true,
    },
    ZoneDef {
        id: "pelo",
        name: "Pelo",
        min: 0.0,
        spd: [80.0, 700.0],
        gain: 2.5,
        kiss: true,
    },
    ZoneDef {
        id: "cuello",
        name: "Cuello",
        min: 20.0,
        spd: [50.0, 380.0],
        gain: 6.5,
        kiss: true,
    },
    ZoneDef {
        id: "mano",
        name: "Mano",
        min: 0.0,
        spd: [40.0, 450.0],
        gain: 2.5,
        kiss: true,
    },
    ZoneDef {
        id: "hombro",
        name: "Hombro",
        min: 8.0,
        spd: [60.0, 520.0],
        gain: 3.5,
        kiss: true,
    },
    ZoneDef {
        id: "pecho",
        name: "Pecho",
        min: 40.0,
        spd: [80.0, 560.0],
        gain: 8.0,
        kiss: false,
    },
    ZoneDef {
        id: "cintura",
        name: "Cintura",
        min: 25.0,
        spd: [80.0, 600.0],
        gain: 5.0,
        kiss: false,
    },
    ZoneDef {
        id: "cadera",
        name: "Cadera",
        min: 45.0,
        spd: [70.0, 520.0],
        gain: 7.5,
        kiss: false,
    },
    ZoneDef {
        id: "muslos",
        name: "Muslos",
        min: 50.0,
        spd: [70.0, 560.0],
        gain: 8.0,
        kiss: false,
    },
];

fn zone_index(id: &str) -> Option<usize> {
    ZONES.iter().position(|z| z.id == id)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Stroke,
    Hold,
    Click,
    Kiss,
}

/// A hand: how it touches, the desire it asks on top of the zone's, what it gives, its pace.
pub(crate) struct Tool {
    pub id: &'static str,
    pub name: &'static str,
    pub help: &'static str,
    pub kind: Kind,
    min: f32,
    gain: f32,
    spd: f32,
    /// Zones it is made for (the finger): more there, less elsewhere.
    fine: &'static [&'static str],
    /// The only zones it may touch (else she flinches).
    ok: &'static [&'static str],
    pub both: bool,
    /// How far from a zone it still finds it (the hand is a soft circle).
    margin: f32,
    force: f32,
    spot: f32,
}

pub(crate) static TOOLS: [Tool; 6] = [
    Tool {
        id: "mano",
        name: "MANO",
        help: "Mantén y desliza: caricia.",
        kind: Kind::Stroke,
        min: 0.0,
        gain: 1.0,
        spd: 1.0,
        fine: &[],
        ok: &[],
        both: false,
        margin: 46.0,
        force: 1.0,
        spot: 80.0,
    },
    Tool {
        id: "dedo",
        name: "DEDO",
        help: "Roza con la punta. Orejas, labios, cuello.",
        kind: Kind::Stroke,
        min: -5.0,
        gain: 1.0,
        spd: 0.7,
        fine: &["oreja", "labios", "cuello", "mejilla", "mano", "pecho"],
        ok: &[],
        both: false,
        margin: 36.0,
        force: 0.8,
        spot: 46.0,
    },
    Tool {
        id: "agarrar",
        name: "AGARRAR",
        help: "Mantén quieto: apretar. Suelta: rebota.",
        kind: Kind::Hold,
        min: 10.0,
        gain: 1.4,
        spd: 1.0,
        fine: &[],
        ok: &["pecho", "cadera", "muslos", "cintura", "mano", "hombro"],
        both: false,
        margin: 56.0,
        force: 1.4,
        spot: 95.0,
    },
    Tool {
        id: "pellizco",
        name: "PELLIZCO",
        help: "Clic. Solo cuando ya está muy encendida.",
        kind: Kind::Click,
        min: 20.0,
        gain: 2.2,
        spd: 1.0,
        fine: &[],
        ok: &["pecho", "cadera", "muslos", "mejilla"],
        both: false,
        margin: 34.0,
        force: 1.8,
        spot: 40.0,
    },
    Tool {
        id: "dos",
        name: "DOS MANOS",
        help: "Las dos a la vez, en espejo. Más fuerte en zonas pares.",
        kind: Kind::Stroke,
        min: 5.0,
        gain: 1.0,
        spd: 1.0,
        fine: &[],
        ok: &[],
        both: true,
        margin: 46.0,
        force: 1.15,
        spot: 80.0,
    },
    Tool {
        id: "beso",
        name: "BESO",
        help: "Clic: beso (también clic derecho).",
        kind: Kind::Kiss,
        min: 0.0,
        gain: 1.0,
        spd: 1.0,
        fine: &[],
        ok: &[],
        both: false,
        margin: 44.0,
        force: 0.9,
        spot: 60.0,
    },
];

// ------------------------------------------------------------------ the girls

/// A garment (or her braid): what desire lets it come off, its handle (art px) and the way it is
/// pulled, what must come off first, the zones it covers.
pub(crate) struct Piece {
    pub id: &'static str,
    pub name: &'static str,
    unlock: f32,
    handle: [f32; 2],
    dir: [f32; 2],
    requires: &'static [&'static str],
    covers: &'static [&'static str],
    hair: bool,
}

/// Where her body moves (art px ellipses: centre and radii), her hair's mask and pivot, her face.
pub(crate) struct Rig {
    hair_mask: &'static str,
    mid: f32,
    hair_pivot: f32,
    hair_amp: f32,
    breasts: [[f32; 4]; 2],
    head: [f32; 4],
    chest: [f32; 4],
    thighs: [[f32; 4]; 2],
    body: [f32; 4],
    hips: [f32; 4],
    shoulders: [[f32; 4]; 2],
    pub face_rect: [f32; 4],
    /// The faces (expression, picture): without glasses, and with them (the cap still on).
    faces: &'static [(&'static str, &'static str)],
    faces_glasses: &'static [(&'static str, &'static str)],
}

pub(crate) struct Lines {
    start: [&'static str; 2],
    flinch: &'static [&'static str],
    fast: &'static [&'static str],
    good: &'static [&'static str],
    fav: &'static [&'static str],
    sat: &'static [&'static str],
    locked: &'static [&'static str],
    kiss: &'static [&'static str],
    undress: &'static [(&'static str, &'static str)],
    more: &'static str,
    stop: &'static str,
    thanks: &'static str,
    climax: &'static str,
    rough: &'static str,
    pinch: &'static [&'static str],
    two: &'static str,
    timeout: &'static str,
}

pub(crate) struct Girl {
    pub id: &'static str,
    pub name: &'static str,
    pub title: &'static str,
    pub accent: u32,
    fav: &'static str,
    /// Her room, emptied (a picture key under `data/assets`, no `.png`).
    pub bg: &'static str,
    pieces: &'static [Piece],
    rig: Rig,
    /// Her zones in her art's pixels: (zone, polygon).
    zones: &'static [(&'static str, &'static [[f32; 2]])],
    lines: Lines,
    chq: &'static [&'static str],
    /// Her art by what she still wears (the pieces' ids in order, joined by commas).
    art: &'static [(&'static str, &'static str)],
    /// The final clip's shot folder by what she still wears.
    clips: &'static [(&'static str, &'static str)],
}

pub(crate) static LIRA: Girl = Girl {
    id: "lira",
    name: "LIRA",
    title: "Desatar la trenza",
    accent: 0x9fd8ff,
    fav: "oreja",
    bg: "caricias/lira/fondo/lira_fondo_9941",
    pieces: &[
        Piece {
            id: "cofia",
            name: "Cofia",
            unlock: 10.0,
            handle: [935.0, 62.0],
            dir: [1.0, -0.4],
            requires: &[],
            covers: &["pelo"],
            hair: false,
        },
        Piece {
            id: "vestido",
            name: "Vestido",
            unlock: 40.0,
            handle: [850.0, 365.0],
            dir: [0.0, 1.0],
            requires: &["cofia"],
            covers: &["pecho", "cintura", "cadera", "hombro", "muslos"],
            hair: false,
        },
        Piece {
            id: "medias",
            name: "Medias",
            unlock: 54.0,
            handle: [760.0, 962.0],
            dir: [0.0, 1.0],
            requires: &["vestido"],
            covers: &["muslos"],
            hair: false,
        },
        Piece {
            id: "lenceria",
            name: "Lencería",
            unlock: 66.0,
            handle: [843.0, 532.0],
            dir: [0.0, 1.0],
            requires: &["medias"],
            covers: &["pecho", "cadera"],
            hair: false,
        },
        Piece {
            id: "trenza",
            name: "Trenza",
            unlock: 72.0,
            handle: [722.0, 647.0],
            dir: [0.0, 1.0],
            requires: &["lenceria"],
            covers: &[],
            hair: true,
        },
    ],
    rig: Rig {
        hair_mask: "caricias/lira/lira_pelo_mascara",
        mid: 850.0,
        hair_pivot: 250.0,
        hair_amp: 0.35,
        breasts: [[775.0, 515.0, 80.0, 80.0], [935.0, 515.0, 80.0, 80.0]],
        head: [850.0, 200.0, 130.0, 165.0],
        chest: [850.0, 520.0, 240.0, 220.0],
        thighs: [[760.0, 955.0, 110.0, 110.0], [945.0, 955.0, 110.0, 110.0]],
        body: [850.0, 620.0, 320.0, 500.0],
        hips: [850.0, 815.0, 290.0, 160.0],
        shoulders: [[720.0, 365.0, 100.0, 90.0], [980.0, 365.0, 100.0, 90.0]],
        face_rect: [760.0, 128.0, 180.0, 180.0],
        faces: &[
            ("sonrojada", "caricias/lira/caras/sonrojada"),
            ("sorpresa", "caricias/lira/caras/sorpresa"),
            ("placer", "caricias/lira/caras/placer"),
            ("extasis", "caricias/lira/caras/extasis"),
            ("parpadeo", "caricias/lira/caras/parpadeo"),
        ],
        faces_glasses: &[],
    },
    zones: &[
        (
            "labios",
            &[
                [822.0, 252.0],
                [870.0, 252.0],
                [870.0, 282.0],
                [822.0, 282.0],
            ],
        ),
        (
            "oreja",
            &[
                [720.0, 158.0],
                [790.0, 188.0],
                [796.0, 230.0],
                [760.0, 216.0],
            ],
        ),
        (
            "oreja",
            &[
                [967.0, 156.0],
                [905.0, 184.0],
                [900.0, 226.0],
                [935.0, 212.0],
            ],
        ),
        (
            "mejilla",
            &[
                [785.0, 215.0],
                [825.0, 215.0],
                [825.0, 262.0],
                [790.0, 256.0],
            ],
        ),
        (
            "mejilla",
            &[
                [868.0, 215.0],
                [906.0, 215.0],
                [903.0, 256.0],
                [868.0, 262.0],
            ],
        ),
        (
            "pelo",
            &[[765.0, 48.0], [935.0, 48.0], [940.0, 150.0], [760.0, 150.0]],
        ),
        (
            "cuello",
            &[
                [808.0, 292.0],
                [882.0, 292.0],
                [887.0, 342.0],
                [803.0, 342.0],
            ],
        ),
        (
            "mano",
            &[
                [478.0, 860.0],
                [582.0, 860.0],
                [582.0, 1000.0],
                [478.0, 1000.0],
            ],
        ),
        (
            "mano",
            &[
                [1128.0, 860.0],
                [1236.0, 860.0],
                [1236.0, 1000.0],
                [1128.0, 1000.0],
            ],
        ),
        (
            "hombro",
            &[
                [680.0, 335.0],
                [772.0, 325.0],
                [766.0, 402.0],
                [672.0, 412.0],
            ],
        ),
        (
            "hombro",
            &[
                [925.0, 325.0],
                [1016.0, 335.0],
                [1024.0, 412.0],
                [930.0, 402.0],
            ],
        ),
        (
            "pecho",
            &[
                [700.0, 430.0],
                [1000.0, 430.0],
                [1005.0, 592.0],
                [695.0, 592.0],
            ],
        ),
        (
            "cintura",
            &[
                [722.0, 596.0],
                [992.0, 596.0],
                [997.0, 732.0],
                [718.0, 732.0],
            ],
        ),
        (
            "cadera",
            &[
                [680.0, 740.0],
                [1046.0, 740.0],
                [1050.0, 892.0],
                [675.0, 892.0],
            ],
        ),
        (
            "muslos",
            &[
                [652.0, 896.0],
                [1062.0, 896.0],
                [1066.0, 1024.0],
                [648.0, 1024.0],
            ],
        ),
    ],
    lines: Lines {
        start: [
            "—Majestad. Me invitó a estas horas… y quise venir.",
            "—Si algo no me gusta, se lo diré. …Y no me mire así.",
        ],
        flinch: &[
            "—M-Majestad. Aún no.",
            "—Eso… todavía no.",
            "—Despacio. Se lo ruego.",
        ],
        fast: &[
            "—Sin prisa. La casa no arde.",
            "—Despacio, Majestad. Por favor.",
        ],
        good: &[
            "—…Mm.",
            "—Eso… está bien.",
            "—No se detenga.",
            "—Tiene las manos frías. …No pare.",
        ],
        fav: &[
            "—¡Ah…! Las orejas… sí. Ahí sí, Majestad.",
            "—Ahí… eso es injusto. No pare.",
        ],
        sat: &["—Ahí ya… basta. Pruebe otra cosa.", "—Se repite, Majestad."],
        locked: &["—Aún no, Majestad.", "—Eso lo decido yo. …Todavía no."],
        kiss: &["—…", "—Majestad…", "—Eso no lo apunto en el libro."],
        undress: &[
            (
                "cofia",
                "—Mi cofia… diez años sirviendo en este castillo y nadie me la había quitado.",
            ),
            ("trenza", "—Nadie me había soltado el pelo. Nunca."),
            ("vestido", "—Los botones son difíciles. …Yo la ayudo."),
            ("medias", "—Con cuidado. Son las únicas que tengo."),
            ("lenceria", "—No mire tanto. …Majestad."),
        ],
        more: "—Majestad… aún llevo algo puesto.",
        stop: "—Majestad… ¿podemos dejarlo aquí por hoy? …No es usted. Soy yo.",
        thanks: "—Gracias por parar. …De verdad.",
        climax: "—…Majestad. No se lo diga a nadie. Ni a mi libro.",
        rough: "—¡Majestad! Eso no se aprieta.",
        pinch: &["—¡Ah…! …Majestad.", "—¡Ah! …Avíseme la próxima vez. …Otra."],
        two: "—Las dos manos… ¿Tanto me echaba de menos?",
        timeout: "—Se apagó la vela. …Mañana madrugo, Majestad.",
    },
    chq: &[
        "—Las elfas de nieve y las orejas… créeme.",
        "—Lira odia las prisas. Despacio.",
        "—Nadie le ha soltado nunca la trenza.",
        "—Prueba el cuello. …O no, que muerde. Mentira, no muerde.",
    ],
    art: &[
        (
            "cofia,vestido,medias,lenceria,trenza",
            "caricias/lira/lira_capa4_cofia_alfa",
        ),
        (
            "vestido,medias,lenceria,trenza",
            "caricias/lira/lira_capa3_vestido_alfa",
        ),
        (
            "medias,lenceria,trenza",
            "caricias/lira/lira_capa2_medias_alfa",
        ),
        ("lenceria,trenza", "caricias/lira/lira_capa1_lenceria_alfa"),
        ("trenza", "caricias/lira/lira_base_desnuda_alfa"),
        ("", "caricias/lira/lira_suelta_alfa"),
    ],
    clips: &[
        ("trenza", "caricias/lira/final/final_trenza"),
        ("", "caricias/lira/final/final_suelta"),
    ],
};

pub(crate) static NYXARA: Girl = Girl {
    id: "nyxara",
    name: "NYXARA",
    title: "Calibración del sello",
    accent: 0xff5d72,
    fav: "mano",
    bg: "caricias/nyxara/fondo/nyx_fondo_9951",
    pieces: &[
        Piece {
            id: "cofia",
            name: "Cofia y gafas",
            unlock: 14.0,
            handle: [1012.0, 46.0],
            dir: [1.0, -0.3],
            requires: &[],
            covers: &["pelo"],
            hair: false,
        },
        Piece {
            id: "vestido",
            name: "Vestido",
            unlock: 40.0,
            handle: [880.0, 452.0],
            dir: [0.0, 1.0],
            requires: &["cofia"],
            covers: &["pecho", "cintura", "cadera", "hombro"],
            hair: false,
        },
        Piece {
            id: "medias",
            name: "Medias",
            unlock: 55.0,
            handle: [800.0, 960.0],
            dir: [0.0, 1.0],
            requires: &["vestido"],
            covers: &["muslos"],
            hair: false,
        },
        Piece {
            id: "lenceria",
            name: "Lencería",
            unlock: 68.0,
            handle: [880.0, 548.0],
            dir: [0.0, 1.0],
            requires: &["medias"],
            covers: &["pecho", "cadera"],
            hair: false,
        },
    ],
    rig: Rig {
        hair_mask: "caricias/nyxara/nyx_pelo_mascara",
        mid: 888.0,
        hair_pivot: 300.0,
        hair_amp: 1.0,
        breasts: [[812.0, 505.0, 92.0, 100.0], [965.0, 505.0, 92.0, 100.0]],
        head: [888.0, 210.0, 135.0, 165.0],
        chest: [888.0, 520.0, 250.0, 230.0],
        thighs: [[792.0, 930.0, 115.0, 120.0], [985.0, 930.0, 115.0, 120.0]],
        body: [888.0, 610.0, 330.0, 520.0],
        hips: [888.0, 820.0, 300.0, 170.0],
        shoulders: [[760.0, 365.0, 115.0, 95.0], [1015.0, 365.0, 115.0, 95.0]],
        face_rect: [790.0, 130.0, 200.0, 200.0],
        faces: &[
            ("sonrojada", "caricias/nyxara/caras/normal_sonrojada"),
            ("sorpresa", "caricias/nyxara/caras/normal_sorpresa"),
            ("placer", "caricias/nyxara/caras/normal_placer"),
            ("extasis", "caricias/nyxara/caras/normal_extasis"),
            ("parpadeo", "caricias/nyxara/caras/normal_parpadeo"),
        ],
        faces_glasses: &[
            ("sonrojada", "caricias/nyxara/caras/gafas_sonrojada"),
            ("sorpresa", "caricias/nyxara/caras/gafas_sorpresa"),
            ("placer", "caricias/nyxara/caras/gafas_placer"),
            ("extasis", "caricias/nyxara/caras/gafas_extasis"),
            ("parpadeo", "caricias/nyxara/caras/gafas_parpadeo"),
        ],
    },
    zones: &[
        (
            "labios",
            &[
                [860.0, 252.0],
                [910.0, 252.0],
                [910.0, 282.0],
                [860.0, 282.0],
            ],
        ),
        (
            "oreja",
            &[
                [716.0, 162.0],
                [800.0, 196.0],
                [806.0, 236.0],
                [770.0, 220.0],
            ],
        ),
        (
            "oreja",
            &[
                [1022.0, 154.0],
                [960.0, 192.0],
                [956.0, 232.0],
                [990.0, 214.0],
            ],
        ),
        (
            "mejilla",
            &[
                [818.0, 220.0],
                [858.0, 220.0],
                [858.0, 268.0],
                [822.0, 262.0],
            ],
        ),
        (
            "mejilla",
            &[
                [912.0, 220.0],
                [952.0, 220.0],
                [950.0, 262.0],
                [912.0, 268.0],
            ],
        ),
        (
            "pelo",
            &[[800.0, 46.0], [985.0, 46.0], [990.0, 150.0], [796.0, 150.0]],
        ),
        (
            "cuello",
            &[
                [850.0, 296.0],
                [925.0, 296.0],
                [930.0, 350.0],
                [845.0, 350.0],
            ],
        ),
        (
            "mano",
            &[
                [495.0, 830.0],
                [590.0, 830.0],
                [590.0, 970.0],
                [495.0, 970.0],
            ],
        ),
        (
            "mano",
            &[
                [1160.0, 840.0],
                [1255.0, 840.0],
                [1255.0, 970.0],
                [1160.0, 970.0],
            ],
        ),
        (
            "hombro",
            &[
                [735.0, 330.0],
                [820.0, 320.0],
                [812.0, 400.0],
                [728.0, 410.0],
            ],
        ),
        (
            "hombro",
            &[
                [945.0, 320.0],
                [1035.0, 330.0],
                [1045.0, 410.0],
                [960.0, 400.0],
            ],
        ),
        (
            "pecho",
            &[
                [740.0, 400.0],
                [1040.0, 400.0],
                [1045.0, 595.0],
                [735.0, 595.0],
            ],
        ),
        (
            "cintura",
            &[
                [760.0, 600.0],
                [1020.0, 600.0],
                [1025.0, 745.0],
                [755.0, 745.0],
            ],
        ),
        (
            "cadera",
            &[
                [712.0, 750.0],
                [1062.0, 750.0],
                [1060.0, 900.0],
                [714.0, 900.0],
            ],
        ),
        (
            "muslos",
            &[
                [692.0, 905.0],
                [1080.0, 905.0],
                [1085.0, 1024.0],
                [690.0, 1024.0],
            ],
        ),
    ],
    lines: Lines {
        start: [
            "—Calibración número uno. Diga «basta» cuando quiera.",
            "—El sello reacciona a lo que usted siente. Toque. Yo apunto.",
        ],
        flinch: &[
            "—Variable fuera de protocolo. Aún no.",
            "—Eso altera los datos. Todavía no.",
        ],
        fast: &[
            "—Pulso disparado. El suyo, no el mío. Despacio.",
            "—Demasiado rápido para medir nada.",
        ],
        good: &[
            "—Interesante…",
            "—Anoto: respuesta positiva.",
            "—…Mm. Siga. Por la ciencia.",
            "—Deje de anotarlo usted en mi cuello.",
        ],
        fav: &[
            "—Me está tomando el pulso. …Está alto. No lo apunte.",
            "—La muñeca… eso no estaba en mis notas.",
        ],
        sat: &[
            "—Ese dato ya lo tengo. Otra zona.",
            "—Redundante, Majestad.",
        ],
        locked: &[
            "—Eso no estaba en el protocolo. Todavía.",
            "—Primero lo primero, Majestad.",
        ],
        kiss: &[
            "—…Eso no es un instrumento de medida.",
            "—Contaminación de la muestra. …Otra vez.",
        ],
        undress: &[
            ("cofia", "—Me quito las gafas. Para no estudiarla."),
            ("vestido", "—Esto ya no es ciencia, Majestad."),
            ("medias", "—…Sin prisa."),
            (
                "lenceria",
                "—Mire usted. Yo ya no apunto. …Fin del protocolo.",
            ),
        ],
        more: "—Protocolo incompleto, Majestad. Aún llevo algo puesto.",
        stop: "—Paramos aquí. Dato importante: usted sabe parar. Gracias.",
        thanks: "—Parar también es un resultado. Gracias, Majestad.",
        climax: "—…Resultado: inconcluso. Habrá que repetirlo.",
        rough: "—Eso no se aprieta, Majestad. No es una almohada.",
        pinch: &[
            "—¡Ah! …Anoto: dolor leve. Placer… no lo anoto.",
            "—Repítalo. Por la ciencia.",
        ],
        two: "—Ambas manos. Muestra bilateral. …Muy rigurosa.",
        timeout: "—Se acabó la sesión. Mañana repetimos la medición.",
    },
    chq: &[
        "—Le encanta que le tomen el pulso. La muñeca.",
        "—Las gafas primero: sin ellas no te analiza.",
        "—Dicen que le gustan los pies. …Mentira, lo dije yo.",
    ],
    art: &[
        (
            "cofia,vestido,medias,lenceria",
            "caricias/nyxara/nyx_capa4_cofia_alfa",
        ),
        (
            "vestido,medias,lenceria",
            "caricias/nyxara/nyx_capa3_vestido_alfa",
        ),
        ("medias,lenceria", "caricias/nyxara/nyx_capa2_medias_alfa"),
        ("lenceria", "caricias/nyxara/nyx_capa1_lenceria_alfa"),
        ("", "caricias/nyxara/nyx_base_desnuda_alfa"),
    ],
    clips: &[("", "caricias/nyxara/final/final")],
};

/// Maren, the scout from the east who stayed of her own will: her night comes after the court's
/// routine and her talks (`corte.noche_maren`). Her art: a clean nude base (`maren_mezcla_9915`) and
/// its layers by Krea Edit, cut by rembg (`maren_cut.py`), in the Queen's alcove at night.
pub(crate) static MAREN: Girl = Girl {
    id: "maren",
    name: "MAREN",
    title: "Sin uniforme",
    accent: 0x8fd1b0,
    fav: "cuello",
    bg: "caricias/maren/fondo/maren_fondo_alcoba",
    pieces: &[
        Piece {
            id: "camisa",
            name: "Camisa",
            unlock: 12.0,
            handle: [968.0, 420.0],
            dir: [1.0, -0.2],
            requires: &[],
            covers: &["pecho", "cintura", "hombro"],
            hair: false,
        },
        Piece {
            id: "pantalones",
            name: "Pantalones",
            unlock: 38.0,
            handle: [895.0, 712.0],
            dir: [0.0, 1.0],
            requires: &["camisa"],
            covers: &["cadera", "muslos"],
            hair: false,
        },
        Piece {
            id: "faja",
            name: "Faja",
            unlock: 52.0,
            handle: [895.0, 470.0],
            dir: [0.0, -1.0],
            requires: &["pantalones"],
            covers: &["pecho"],
            hair: false,
        },
        Piece {
            id: "calzas",
            name: "Calzas",
            unlock: 66.0,
            handle: [895.0, 815.0],
            dir: [0.0, 1.0],
            requires: &["faja"],
            covers: &["cadera"],
            hair: false,
        },
    ],
    rig: Rig {
        hair_mask: "caricias/maren/maren_pelo_mascara",
        mid: 900.0,
        hair_pivot: 210.0,
        hair_amp: 0.7,
        breasts: [[805.0, 460.0, 72.0, 66.0], [982.0, 478.0, 70.0, 66.0]],
        head: [900.0, 165.0, 105.0, 135.0],
        chest: [895.0, 470.0, 230.0, 200.0],
        thighs: [[795.0, 950.0, 105.0, 110.0], [1000.0, 950.0, 105.0, 110.0]],
        body: [900.0, 620.0, 300.0, 500.0],
        hips: [900.0, 780.0, 280.0, 150.0],
        shoulders: [[760.0, 345.0, 95.0, 80.0], [1040.0, 355.0, 95.0, 80.0]],
        face_rect: [800.0, 85.0, 200.0, 200.0],
        faces: &[
            ("sonrojada", "caricias/maren/caras/normal_sonrojada"),
            ("sorpresa", "caricias/maren/caras/normal_sorpresa"),
            ("placer", "caricias/maren/caras/normal_placer"),
            ("extasis", "caricias/maren/caras/normal_extasis"),
            ("parpadeo", "caricias/maren/caras/normal_parpadeo"),
        ],
        faces_glasses: &[],
    },
    zones: &[
        (
            "labios",
            &[
                [876.0, 204.0],
                [924.0, 204.0],
                [924.0, 234.0],
                [876.0, 234.0],
            ],
        ),
        (
            "oreja",
            &[
                [788.0, 150.0],
                [828.0, 158.0],
                [830.0, 216.0],
                [796.0, 212.0],
            ],
        ),
        (
            "oreja",
            &[
                [1008.0, 150.0],
                [972.0, 158.0],
                [970.0, 216.0],
                [1004.0, 212.0],
            ],
        ),
        (
            "mejilla",
            &[
                [840.0, 170.0],
                [876.0, 170.0],
                [876.0, 214.0],
                [844.0, 214.0],
            ],
        ),
        (
            "mejilla",
            &[
                [924.0, 170.0],
                [958.0, 170.0],
                [956.0, 214.0],
                [924.0, 214.0],
            ],
        ),
        (
            "pelo",
            &[
                [808.0, 18.0],
                [995.0, 18.0],
                [1000.0, 104.0],
                [802.0, 104.0],
            ],
        ),
        (
            "cuello",
            &[
                [862.0, 240.0],
                [938.0, 240.0],
                [944.0, 304.0],
                [856.0, 304.0],
            ],
        ),
        (
            "mano",
            &[
                [585.0, 880.0],
                [665.0, 880.0],
                [665.0, 1012.0],
                [585.0, 1012.0],
            ],
        ),
        (
            "mano",
            &[
                [1135.0, 895.0],
                [1212.0, 895.0],
                [1212.0, 1012.0],
                [1135.0, 1012.0],
            ],
        ),
        (
            "hombro",
            &[
                [712.0, 312.0],
                [800.0, 304.0],
                [796.0, 385.0],
                [705.0, 395.0],
            ],
        ),
        (
            "hombro",
            &[
                [1000.0, 310.0],
                [1088.0, 322.0],
                [1095.0, 400.0],
                [1004.0, 390.0],
            ],
        ),
        (
            "pecho",
            &[
                [730.0, 390.0],
                [1055.0, 390.0],
                [1058.0, 560.0],
                [728.0, 560.0],
            ],
        ),
        (
            "cintura",
            &[
                [770.0, 562.0],
                [1022.0, 562.0],
                [1030.0, 700.0],
                [766.0, 700.0],
            ],
        ),
        (
            "cadera",
            &[
                [722.0, 702.0],
                [1078.0, 702.0],
                [1084.0, 845.0],
                [716.0, 845.0],
            ],
        ),
        (
            "muslos",
            &[
                [706.0, 848.0],
                [1096.0, 848.0],
                [1092.0, 1024.0],
                [710.0, 1024.0],
            ],
        ),
    ],
    lines: Lines {
        start: [
            "—Las del este no nos dejamos tocar el cuello. …Eso era en el este.",
            "—Despacio. Una exploradora necesita saber por dónde va el camino.",
        ],
        flinch: &[
            "—Ahí no. Todavía no. Llevo cinco años vigilando ese lado.",
            "—Eh. Sin prisas, Majestad.",
        ],
        fast: &[
            "—Despacio. A este paso nos perdemos las dos.",
            "—Así no se lee un mapa. Más despacio.",
        ],
        good: &[
            "—…Eso sí.",
            "—Bien. Sigue por ahí.",
            "—No lo apunte nadie. Esto no va en ningún informe.",
            "—Mm. Así.",
        ],
        fav: &[
            "—El cuello. Nadie había llegado ahí sin un cuchillo.",
            "—Ahí… ahí sí. No pares.",
        ],
        sat: &[
            "—Ese camino ya lo conozco. Busca otro.",
            "—Eso ya lo sé de memoria, Majestad.",
        ],
        locked: &[
            "—Esa puerta aún tiene llave. La abro yo.",
            "—Primero lo de encima. Soy de orden.",
        ],
        kiss: &[
            "—…En el este esto sería un escándalo.",
            "—Otra. Para estar segura.",
        ],
        undress: &[
            ("camisa", "—La camisa fuera. Ya no hace frío aquí."),
            ("pantalones", "—El cinturón lo suelto yo. Costumbre."),
            ("faja", "—…Mírame. Sin vendas."),
            ("calzas", "—Ya está. Nada del uniforme. Solo yo."),
        ],
        more:
            "—Todavía llevo algo, Majestad. Una exploradora nunca entrega el mapa entero de golpe.",
        stop: "—Paramos aquí. Gracias por parar. En el este nadie para.",
        thanks: "—Saber parar también es saber el camino. Gracias.",
        climax: "—…Esto tampoco cabe en un informe.",
        rough: "—Suave. Ya me apretaron bastante en mi vida.",
        pinch: &["—¡Ah! …Eso dolía. Hazlo otra vez.", "—Atrevida. Me gusta."],
        two: "—Las dos manos. Ya no tengo dónde esconderme.",
        timeout: "—Se ha consumido la vela. Mañana sigo el camino, si quieres.",
    },
    chq: &[
        "—Una exploradora protege el cuello. Si te deja tocarlo, has ganado.",
        "—La camisa primero: la lleva para no parecer de ningún bando.",
        "—Despacio con ella. Viene de un sitio donde nadie pregunta.",
    ],
    art: &[
        (
            "camisa,pantalones,faja,calzas",
            "caricias/maren/maren_capa3_camisa_alfa",
        ),
        (
            "pantalones,faja,calzas",
            "caricias/maren/maren_capa2_pantalones_alfa",
        ),
        ("faja,calzas", "caricias/maren/maren_capa1_faja_alfa"),
        ("calzas", "caricias/maren/maren_capa0_calzas_alfa"),
        ("", "caricias/maren/maren_base_desnuda_alfa"),
    ],
    clips: &[("", "caricias/maren/final/final")],
};

pub(crate) fn girl(id: &str) -> Option<&'static Girl> {
    match id {
        "lira" => Some(&LIRA),
        "nyxara" => Some(&NYXARA),
        "maren" => Some(&MAREN),
        _ => None,
    }
}

// ------------------------------------------------------------------ geometry

fn in_poly(x: f32, y: f32, poly: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let ([xi, yi], [xj, yj]) = (poly[i], poly[j]);
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_to_poly(x: f32, y: f32, poly: &[[f32; 2]]) -> f32 {
    if in_poly(x, y, poly) {
        return 0.0;
    }
    let mut best = f32::MAX;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let ([ax, ay], [bx, by]) = (poly[j], poly[i]);
        let (dx, dy) = (bx - ax, by - ay);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        let t = (((x - ax) * dx + (y - ay) * dy) / len2).clamp(0.0, 1.0);
        best = best.min((x - ax - t * dx).hypot(y - ay - t * dy));
        j = i;
    }
    best
}

fn in_ellipse(p: [f32; 2], e: [f32; 4]) -> bool {
    let (dx, dy) = ((p[0] - e[0]) / e[2], (p[1] - e[1]) / e[3]);
    dx * dx + dy * dy < 1.0
}

// ------------------------------------------------------------------ her body moving

#[derive(Clone, Copy, Default)]
struct Spring {
    o: [f32; 2],
    v: [f32; 2],
}

#[derive(Clone, Copy, Default)]
struct Soft {
    o: [f32; 2],
    v: [f32; 2],
    sq: f32,
    sq_t: f32,
    tgt: Option<[f32; 2]>,
}

#[derive(Clone, Copy, Default)]
struct Spot {
    c: [f32; 2],
    r: f32,
    o: [f32; 2],
    v: [f32; 2],
    sq: f32,
    sqv: f32,
    on: bool,
}

#[derive(Clone, Copy)]
struct Dent {
    c: [f32; 2],
    r: [f32; 2],
    o: [f32; 2],
    sq: f32,
    life: f32,
    max: f32,
    hand: bool,
}

/// The springs the web page's shader read: breasts, thighs, head, body, hips, shoulders, the hair,
/// a jump, the hand's spots and short dents, and the face crossing over.
#[derive(Clone, Default)]
struct Body {
    br: [Soft; 2],
    th: [Soft; 2],
    head: Spring,
    body: Spring,
    hips: Spring,
    sh: [Spring; 2],
    hair_a: [f32; 2],
    hair_v: [f32; 2],
    jump: [f32; 2],
    spots: [Spot; 2],
    dents: Vec<Dent>,
    face_a: &'static str,
    face_b: &'static str,
    face_t: f32,
    blink_at: f32,
    blink_t: f32,
}

fn spring(o: &mut Spring, k: f32, c: f32, lo: f32, hi: f32, h: f32) {
    for j in 0..2 {
        o.v[j] += (-k * o.o[j] - c * o.v[j]) * h;
        o.o[j] = (o.o[j] + o.v[j] * h).clamp(lo, hi);
    }
}

/// A floating heart, a ring where a hand landed, a drop of sweat, a startled «!».
#[derive(Clone, Copy)]
pub(crate) struct Fx {
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    pub life: f32,
    pub max: f32,
    pub s: f32,
}

#[derive(Clone, Copy)]
struct Stroke {
    blocked: bool,
    x0: f32,
    y0: f32,
    zone: Option<usize>,
    tense: f32,
    next_react: f32,
}

#[derive(Clone, Copy)]
struct Drag {
    piece: usize,
    x0: f32,
    y0: f32,
    proj: f32,
}

/// How it ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum End {
    Climax,
    Stop,
    Queen,
    Timeout,
}

/// The final clip playing: its frames, its folder, its clock.
pub(crate) struct Reel {
    frames: Vec<PathBuf>,
    pub dir: PathBuf,
    pub time: f32,
    shown: Option<usize>,
}

// ------------------------------------------------------------------ the night

/// The game while it is up.
pub(crate) struct Caricias {
    pub girl: &'static Girl,
    pub t: f32,
    pub deseo: f32,
    pub conf: f32,
    max_deseo: f32,
    pub time: f32,
    pub worn: Vec<bool>,
    sat: [f32; 11],
    pub fav_found: bool,
    pub flinches: u32,
    pub kisses: u32,
    pub tool: usize,
    pub mouse: [f32; 2],
    speed: f32,
    last_move: Option<std::time::Instant>,
    last_d: [f32; 2],
    zone: Option<usize>,
    stroke: Option<Stroke>,
    drag: Option<Drag>,
    flinch_t: f32,
    shake: f32,
    kiss_cd: f32,
    kiss_t: f32,
    fav_t: f32,
    undress_t: f32,
    /// The girl's line on show: text, until, priority.
    line: Option<(String, f32, u8)>,
    last_chq: f32,
    last_fast: f32,
    last_good: f32,
    last_sat: f32,
    last_cap: f32,
    last_gain: f32,
    milestones: [bool; 3],
    two_said: bool,
    body: Body,
    pub fx: Vec<Fx>,
    pub paused: bool,
    pub pause_choice: usize,
    /// Ended: how, and when.
    pub ended: Option<(End, f32)>,
    /// Its end card is up.
    pub card: bool,
    pub reel: Option<Reel>,
    /// What Chiquita says next (the App passes it on).
    chq_out: Vec<String>,
    sfx: Vec<UiSfx>,
    hair: Option<(u32, u32, Vec<u8>)>,
    seed: u32,
    /// The «¡SU PUNTO DÉBIL!» toast and when.
    pub toast: Option<(&'static str, f32)>,
    pub start_line_two: bool,
}

const CANDLE: f32 = 180.0;
const BALANCE: f32 = 0.6;

impl Caricias {
    pub(crate) fn new(girl: &'static Girl, seed: u32) -> Self {
        let hair = velvet_avif::open_image(
            crate::exploration3d_state::data_dir()
                .join("assets")
                .join(format!("{}.png", girl.rig.hair_mask))
                .as_ref(),
        )
        .ok()
        .map(|m| {
            let l = m.to_luma8();
            (l.width(), l.height(), l.into_raw())
        });
        let mut c = Self {
            girl,
            t: 0.0,
            deseo: 0.0,
            conf: 60.0,
            max_deseo: 0.0,
            time: CANDLE,
            worn: vec![true; girl.pieces.len()],
            sat: [0.0; 11],
            fav_found: false,
            flinches: 0,
            kisses: 0,
            tool: 0,
            mouse: [960.0, 540.0],
            speed: 0.0,
            last_move: None,
            last_d: [0.0; 2],
            zone: None,
            stroke: None,
            drag: None,
            flinch_t: 0.0,
            shake: 0.0,
            kiss_cd: 0.0,
            kiss_t: 0.0,
            fav_t: 0.0,
            undress_t: 0.0,
            line: None,
            last_chq: -99.0,
            last_fast: -99.0,
            last_good: -99.0,
            last_sat: -99.0,
            last_cap: -99.0,
            last_gain: 0.0,
            milestones: [false; 3],
            two_said: false,
            body: Body {
                face_a: "",
                face_b: "",
                face_t: 1.0,
                blink_at: 2.5,
                ..Default::default()
            },
            fx: Vec::new(),
            paused: false,
            pause_choice: 0,
            ended: None,
            card: false,
            reel: None,
            chq_out: Vec::new(),
            sfx: Vec::new(),
            hair,
            seed: seed.max(1),
            toast: None,
            start_line_two: false,
        };
        c.kick_enter();
        c.say(girl.lines.start[0], 3);
        c
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed % 10_000) as f32 / 10_000.0
    }

    fn pick(&mut self, list: &[&'static str]) -> &'static str {
        if list.is_empty() {
            return "";
        }
        let i = ((self.rand() * list.len() as f32) as usize).min(list.len() - 1);
        list[i]
    }

    fn say(&mut self, text: &str, pri: u8) {
        if text.is_empty() {
            return;
        }
        if let Some((_, until, p)) = &self.line {
            if self.t < *until && pri < *p {
                return;
            }
        }
        let secs = (text.chars().count() as f32 / 16.0).max(2.2);
        self.line = Some((text.to_owned(), self.t + secs, pri));
    }

    fn chq(&mut self, text: &str, force: bool) {
        if !force && self.t - self.last_chq < 7.5 {
            return;
        }
        self.last_chq = self.t;
        self.chq_out.push(text.to_owned());
    }

    /// The girl's line on show, with how long it has left.
    pub(crate) fn line(&self) -> Option<(&str, f32)> {
        self.line
            .as_ref()
            .filter(|(_, until, _)| self.t < until + 0.6)
            .map(|(text, until, _)| (text.as_str(), until - self.t))
    }

    pub(crate) fn tool(&self) -> &'static Tool {
        &TOOLS[self.tool % TOOLS.len()]
    }

    fn piece_index(&self, id: &str) -> Option<usize> {
        self.girl.pieces.iter().position(|p| p.id == id)
    }

    /// A garment whose handle can be pulled now (still on, and what it needs already off).
    pub(crate) fn piece_open(&self, i: usize) -> bool {
        let p = &self.girl.pieces[i];
        self.worn[i]
            && p.requires
                .iter()
                .all(|r| self.piece_index(r).is_none_or(|j| !self.worn[j]))
    }

    fn covered(&self, zone: usize) -> bool {
        let id = ZONES[zone].id;
        self.girl
            .pieces
            .iter()
            .enumerate()
            .any(|(i, p)| self.worn[i] && p.covers.contains(&id))
    }

    /// What she still wears (the art's key).
    pub(crate) fn art_key(&self) -> String {
        self.girl
            .pieces
            .iter()
            .enumerate()
            .filter(|(i, _)| self.worn[*i])
            .map(|(_, p)| p.id)
            .collect::<Vec<_>>()
            .join(",")
    }

    pub(crate) fn art(&self) -> Option<&'static str> {
        let key = self.art_key();
        self.girl
            .art
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, a)| *a)
    }

    /// A garment's handle on the screen (pulled along its way while dragged).
    pub(crate) fn handle(&self, i: usize) -> [f32; 2] {
        let p = &self.girl.pieces[i];
        let mut h = from_art(p.handle);
        if let Some(d) = self.drag.filter(|d| d.piece == i) {
            let len = p.dir[0].hypot(p.dir[1]).max(1e-3);
            h[0] += p.dir[0] / len * d.proj;
            h[1] += p.dir[1] / len * d.proj;
        }
        h
    }

    pub(crate) fn handle_dir(&self, i: usize) -> [f32; 2] {
        let p = &self.girl.pieces[i];
        let len = p.dir[0].hypot(p.dir[1]).max(1e-3);
        [p.dir[0] / len, p.dir[1] / len]
    }

    pub(crate) fn piece_ready(&self, i: usize) -> bool {
        self.deseo >= self.girl.pieces[i].unlock
    }

    fn handle_at(&self, x: f32, y: f32) -> Option<usize> {
        (0..self.girl.pieces.len()).find(|&i| {
            if !self.piece_open(i) {
                return false;
            }
            let h = from_art(self.girl.pieces[i].handle);
            (x - h[0]).hypot(y - h[1]) < 34.0
        })
    }

    /// The zone under (x, y) for the hand in use (a soft circle: a near miss finds the closest).
    fn zone_at(&self, x: f32, y: f32) -> Option<usize> {
        let tool = self.tool();
        let m = tool.margin;
        let a = to_art([x, y]);
        let k = art_scale();
        let zones = self.girl.zones;
        if tool.id == "dedo" || tool.id == "beso" {
            for small in ["labios", "oreja"] {
                if zones
                    .iter()
                    .filter(|(z, _)| *z == small)
                    .any(|(_, poly)| dist_to_poly(a[0], a[1], poly) * k < m * 0.6)
                {
                    return zone_index(small);
                }
            }
        }
        for z in ZONES.iter() {
            if zones
                .iter()
                .filter(|(id, _)| *id == z.id)
                .any(|(_, poly)| in_poly(a[0], a[1], poly))
            {
                return zone_index(z.id);
            }
        }
        let mut best = None;
        let mut bd = m;
        for (id, poly) in zones {
            let d = dist_to_poly(a[0], a[1], poly) * k;
            if d < bd {
                bd = d;
                best = zone_index(id);
            }
        }
        best
    }

    // ---- what moves her

    fn kick_enter(&mut self) {
        let b = &mut self.body;
        b.jump = [120.0, 0.0];
        for (i, br) in b.br.iter_mut().enumerate() {
            br.v[0] += if i == 1 { 30.0 } else { -30.0 };
            br.v[1] += 260.0;
        }
        b.hair_v[0] += 0.03;
        b.hair_v[1] -= 0.03;
        for (i, th) in b.th.iter_mut().enumerate() {
            th.v[1] += 120.0;
            th.v[0] += if i == 1 { 25.0 } else { -25.0 };
        }
    }

    fn kick_breasts(&mut self, vx: f32, vy: f32) {
        for (i, b) in self.body.br.iter_mut().enumerate() {
            b.v[0] += if i == 1 { vx } else { -vx };
            b.v[1] += vy;
        }
    }

    fn kick_flinch(&mut self) {
        for (i, th) in self.body.th.iter_mut().enumerate() {
            th.v[1] += 120.0;
            th.v[0] += if i == 1 { 25.0 } else { -25.0 };
        }
        for s in &mut self.body.sh {
            s.v[1] -= 160.0;
        }
        self.body.body.v[1] -= 60.0;
        self.body.jump[1] -= 260.0;
        self.body.head.v[1] -= 160.0;
        self.kick_breasts(20.0, 300.0);
        self.body.hair_v[0] -= 0.05;
        self.body.hair_v[1] += 0.05;
    }

    fn kick_climax(&mut self) {
        self.body.body.v[1] -= 160.0;
        self.body.head.v[1] -= 220.0;
        for s in &mut self.body.sh {
            s.v[1] -= 200.0;
        }
        self.kick_breasts(60.0, -420.0);
        for (i, th) in self.body.th.iter_mut().enumerate() {
            th.v[0] += if i == 1 { -160.0 } else { 160.0 };
        }
        self.body.hips.v[1] -= 140.0;
        self.body.hair_v[0] -= 0.06;
        self.body.hair_v[1] += 0.06;
    }

    fn kick_undress(&mut self, id: &str) {
        match id {
            "lenceria" => self.kick_breasts(70.0, 420.0),
            "vestido" => self.kick_breasts(35.0, 240.0),
            "cofia" => {
                self.body.head.v[1] -= 90.0;
                self.body.hair_v[0] -= 0.04;
                self.body.hair_v[1] += 0.04;
            }
            _ => self.kick_breasts(0.0, 90.0),
        }
    }

    fn kick_pinch(&mut self, x: f32, y: f32) {
        let a = to_art([x, y]);
        self.body.dents.push(Dent {
            c: a,
            r: [26.0, 26.0],
            o: [0.0; 2],
            sq: 0.35,
            life: 0.35,
            max: 0.35,
            hand: false,
        });
        let girl: &'static Girl = self.girl;
        let rig = &girl.rig;
        for i in 0..2 {
            let r = (self.rand() - 0.5) * 120.0;
            if in_ellipse(a, rig.breasts[i]) {
                self.body.br[i].v[1] -= 220.0;
                self.body.br[i].v[0] += r;
            }
            let r = (self.rand() - 0.5) * 80.0;
            if in_ellipse(a, rig.thighs[i]) {
                self.body.th[i].v[1] -= 160.0;
                self.body.th[i].v[0] += r;
            }
        }
    }

    fn side_of(&self, x: f32) -> f32 {
        if x < from_art([self.girl.rig.mid, 0.0])[0] {
            -1.0
        } else {
            1.0
        }
    }

    fn mirror_x(&self, x: f32) -> f32 {
        2.0 * from_art([self.girl.rig.mid, 0.0])[0] - x
    }

    /// A touch dents her where it lands and wobbles there.
    fn poke(&mut self, i: usize, x: f32, y: f32, tool: &Tool, k: f32) {
        let a = to_art([x, y]);
        let r1 = self.rand();
        let sp = &mut self.body.spots[i];
        sp.c = a;
        sp.r = tool.spot;
        sp.on = true;
        sp.v[0] += (r1 - 0.5) * 40.0 * k;
        sp.v[1] += 70.0 * k;
        sp.sqv += match tool.id {
            "pellizco" => 3.2,
            "agarrar" => 1.6,
            "dedo" => 1.4,
            _ => 1.0,
        } * k;
    }

    /// Her body answers a touch of `zone`: away from the hand while cold, into it once warm.
    fn react(&mut self, zone: usize, tool: &'static Tool, x: f32, k: f32) {
        let (mx, my) = (self.mouse[0], self.mouse[1]);
        self.poke(0, mx, my, tool, k);
        if tool.both {
            let m = self.mirror_x(mx);
            self.poke(1, m, my, tool, k);
        }
        if k >= 1.0 {
            self.fx.push(Fx {
                kind: 1,
                x: mx,
                y: my,
                vx: 0.0,
                vy: 0.0,
                life: 0.55,
                max: 0.55,
                s: if tool.id == "pellizco" {
                    0.6
                } else if tool.id == "agarrar" {
                    1.4
                } else {
                    1.0
                },
            });
            if tool.both {
                let m = self.mirror_x(mx);
                self.fx.push(Fx {
                    kind: 1,
                    x: m,
                    y: my,
                    vx: 0.0,
                    vy: 0.0,
                    life: 0.55,
                    max: 0.55,
                    s: 1.0,
                });
            }
        }
        let s = self.side_of(x);
        let lean = if self.deseo > 60.0 {
            -0.6
        } else if self.deseo > 30.0 {
            0.45
        } else {
            1.0
        };
        let f = tool.force * k;
        if tool.both {
            self.react_zone(zone, -1.0, lean, f * 0.75);
            self.react_zone(zone, 1.0, -lean, f * 0.75);
        } else {
            self.react_zone(zone, s, -s * lean, f);
        }
    }

    fn react_zone(&mut self, zone: usize, s: f32, aw: f32, f: f32) {
        let b = &mut self.body;
        let head = |b: &mut Body, x: f32, y: f32| {
            b.head.v[0] += x * 10.0;
            b.head.v[1] += y * 10.0;
        };
        let bod = |b: &mut Body, x: f32, y: f32| {
            b.body.v[0] += x * 4.5;
            b.body.v[1] += y * 4.5;
        };
        let hips = |b: &mut Body, x: f32, y: f32| {
            b.hips.v[0] += x * 5.5;
            b.hips.v[1] += y * 5.5;
        };
        let sh = |b: &mut Body, side: f32, y: f32| {
            b.sh[if side < 0.0 { 0 } else { 1 }].v[1] += y * 8.0;
        };
        let br = |b: &mut Body, vx: f32, vy: f32| {
            for (i, x) in b.br.iter_mut().enumerate() {
                x.v[0] += if i == 1 { vx } else { vx * 0.9 } * 0.55;
                x.v[1] += vy * 0.55;
            }
        };
        let th = |b: &mut Body, inward: f32, up: f32| {
            for (i, x) in b.th.iter_mut().enumerate() {
                x.v[0] += if i == 1 { -inward } else { inward } * 12.0;
                x.v[1] += up * 12.0;
            }
        };
        match ZONES[zone].id {
            "oreja" => {
                head(b, aw * 13.0 * f, 5.0 * f);
                sh(b, s, -16.0 * f);
                bod(b, aw * 3.0 * f, 0.0);
            }
            "mejilla" => head(b, aw * 6.0 * f, 2.0 * f),
            "labios" => {
                head(b, 0.0, -5.0 * f);
                bod(b, 0.0, -3.0 * f);
                br(b, 0.0, 50.0 * f);
            }
            "pelo" => {
                head(b, -s * 3.0 * f, -4.0 * f);
                b.hair_v[0] += 0.012 * f * s;
                b.hair_v[1] += 0.012 * f * s;
            }
            "cuello" => {
                sh(b, -1.0, -13.0 * f);
                sh(b, 1.0, -13.0 * f);
                head(b, aw * 8.0 * f, 4.0 * f);
                br(b, 0.0, -70.0 * f);
            }
            "hombro" => {
                sh(b, s, -15.0 * f);
                bod(b, aw * 5.0 * f, 0.0);
                head(b, -aw * 3.0 * f, 0.0);
            }
            "mano" => {
                bod(b, -aw * 4.0 * f, 0.0);
                sh(b, s, -6.0 * f);
            }
            "pecho" => {
                br(b, -aw * 40.0 * f, 140.0 * f);
                bod(b, 0.0, -6.0 * f);
                sh(b, -1.0, 4.0 * f);
                sh(b, 1.0, 4.0 * f);
                head(b, 0.0, -3.0 * f);
            }
            "cintura" => {
                hips(b, aw * 15.0 * f, 0.0);
                bod(b, aw * 6.0 * f, -2.0 * f);
                br(b, aw * 50.0 * f, 60.0 * f);
            }
            "cadera" => {
                hips(b, aw * 12.0 * f, -4.0 * f);
                th(b, 4.0 * f, -3.0 * f);
                br(b, 0.0, 60.0 * f);
            }
            "muslos" => {
                th(b, 9.0 * f, -2.0 * f);
                hips(b, 0.0, -6.0 * f);
                bod(b, 0.0, -3.0 * f);
                br(b, 0.0, 70.0 * f);
            }
            _ => {}
        }
    }

    // ---- the rules

    fn heart(&mut self, x: f32, y: f32, big: bool) {
        let (a, b) = (self.rand(), self.rand());
        self.fx.push(Fx {
            kind: 0,
            x,
            y,
            vx: (a - 0.5) * 60.0,
            vy: -90.0 - b * 80.0,
            life: 1.4,
            max: 1.4,
            s: if big { 1.5 } else { 1.0 },
        });
    }

    fn gain(&mut self, v: f32, zone: Option<usize>) {
        let v = if v > 0.0 { v * BALANCE } else { v };
        // The end only comes with her bare: while she still wears something, desire stops at 95.
        let dressed = self
            .girl
            .pieces
            .iter()
            .enumerate()
            .any(|(i, p)| self.worn[i] && !p.hair);
        self.deseo = (self.deseo + v).clamp(0.0, if dressed { 95.0 } else { 100.0 });
        if dressed && self.deseo >= 95.0 && self.t - self.last_cap > 9.0 {
            self.last_cap = self.t;
            self.chq("—¿Y la ropa? Así no se termina nada. Quítasela.", true);
            let more = self.girl.lines.more;
            self.say(more, 3);
        }
        self.max_deseo = self.max_deseo.max(self.deseo);
        self.last_gain = self.t;
        if self.rand() < v * 1.8 {
            let fav = zone.is_some_and(|z| ZONES[z].id == self.girl.fav);
            let (x, y) = (self.mouse[0], self.mouse[1]);
            self.heart(x, y, fav);
        }
        for (k, m) in [25.0, 50.0, 75.0].into_iter().enumerate() {
            if self.deseo >= m && !self.milestones[k] {
                self.milestones[k] = true;
                if k == 1 {
                    self.chq("—Vas bien. …Yo miro la pared.", true);
                }
                if k == 2 {
                    self.chq("—…No pienso comentar nada.", true);
                }
            }
        }
        if self.deseo >= 100.0 {
            self.end(End::Climax);
        }
    }

    fn flinch(&mut self, text: Option<&'static str>) {
        self.flinch_t = 0.9;
        self.shake = 0.35;
        self.flinches += 1;
        self.kick_flinch();
        self.conf = (self.conf - 9.0).clamp(0.0, 100.0);
        let line = text.unwrap_or_else(|| {
            let l = self.girl.lines.flinch;
            self.pick(l)
        });
        self.say(line, 2);
        if self.flinches == 1 {
            self.chq(
                "—¿Ves? Despacio. Empieza por lo fácil: la mano, el pelo.",
                true,
            );
        }
        let head = self.girl.rig.head;
        let drop = from_art([head[0] + head[2] * 0.62, head[1] - head[3] * 0.4]);
        self.fx.push(Fx {
            kind: 2,
            x: drop[0],
            y: drop[1],
            vx: 10.0,
            vy: 30.0,
            life: 1.0,
            max: 1.0,
            s: 1.0,
        });
        self.fx.push(Fx {
            kind: 3,
            x: drop[0] + 20.0,
            y: drop[1] - 44.0,
            vx: 0.0,
            vy: 0.0,
            life: 0.9,
            max: 0.9,
            s: 1.0,
        });
        self.sfx.push(UiSfx::Error);
        if self.conf <= 0.0 {
            self.end(End::Stop);
        }
    }

    fn touch(&mut self, dt: f32) {
        let tool = self.tool();
        let Some(mut stroke) = self.stroke else {
            return;
        };
        if !matches!(tool.kind, Kind::Stroke | Kind::Hold) {
            return;
        }
        let (mx, my) = (self.mouse[0], self.mouse[1]);
        let zone = self.zone_at(mx, my);
        self.zone = zone;
        let Some(z) = zone else {
            return;
        };
        if stroke.blocked {
            return;
        }
        if stroke.zone != Some(z) {
            stroke.zone = Some(z);
            stroke.tense = 0.0;
            stroke.next_react = self.t + 0.45;
            self.stroke = Some(stroke);
            self.react(z, tool, mx, 1.0);
        } else if self.t > stroke.next_react {
            stroke.next_react = self.t + 0.35 + self.rand() * 0.25;
            self.stroke = Some(stroke);
            self.react(z, tool, mx, 0.45);
        }
        let zd = &ZONES[z];
        let rough = !tool.ok.is_empty() && !tool.ok.contains(&zd.id);
        if rough || self.deseo < zd.min + tool.min {
            // A brush is forgiven: she tenses first, and flinches only if the hand stays.
            let mut s = self.stroke.unwrap_or(stroke);
            s.tense += dt;
            if s.tense < dt * 1.5 {
                self.body.head.v[1] -= 40.0;
                let l = self.pick(&["—…", "—Mm…", "—Majestad…"]);
                self.say(l, 1);
            }
            if s.tense > 0.55 {
                s.blocked = true;
                self.stroke = Some(s);
                self.flinch(rough.then_some(self.girl.lines.rough));
                return;
            }
            self.stroke = Some(s);
            return;
        }
        let mut tm = tool.gain;
        if !tool.fine.is_empty() {
            tm *= if tool.fine.contains(&zd.id) {
                1.35
            } else {
                0.6
            };
        }
        if tool.both {
            let mid = from_art([self.girl.rig.mid, 0.0])[0];
            let z2 = self.zone_at(2.0 * mid - mx, my);
            if z2 == Some(z) && (mx - mid).abs() > 30.0 {
                tm *= 1.6;
            }
            if !self.two_said {
                self.two_said = true;
                let two = self.girl.lines.two;
                self.say(two, 2);
                self.chq("—¿Dos manos? Qué ambición.", true);
            }
        }
        let fav = if zd.id == self.girl.fav { 1.5 } else { 1.0 };
        let cloth = if self.covered(z) { 0.55 } else { 1.25 };
        if tool.kind == Kind::Hold {
            let sat = self.sat[z];
            self.gain(
                zd.gain * tm * cloth * fav * (1.0 - sat / 6.0).max(0.2) * dt,
                Some(z),
            );
            self.sat[z] = sat + dt * 1.3;
            self.conf = (self.conf + 0.6 * dt).clamp(0.0, 100.0);
            return;
        }
        let (lo, hi, v) = (zd.spd[0] * tool.spd, zd.spd[1] * tool.spd, self.speed);
        if v > hi * 1.8 {
            self.conf = (self.conf - 12.0 * dt).clamp(0.0, 100.0);
            if self.t - self.last_fast > 3.0 {
                self.last_fast = self.t;
                let l = self.girl.lines.fast;
                let l = self.pick(l);
                self.say(l, 2);
            }
            if self.conf <= 0.0 {
                self.end(End::Stop);
            }
            return;
        }
        let sat = self.sat[z];
        let sat_f = (1.0 - sat / 7.0).max(0.2);
        let rhythm = if v >= lo {
            if v <= hi {
                1.0
            } else {
                0.7
            }
        } else if v >= lo * 0.4 {
            0.6
        } else {
            0.4
        };
        self.gain(zd.gain * tm * cloth * fav * sat_f * rhythm * dt, Some(z));
        self.sat[z] = sat + dt * rhythm;
        self.conf = (self.conf + 1.6 * dt).clamp(0.0, 100.0);
        if fav > 1.0 && !self.fav_found && rhythm == 1.0 {
            self.found_fav();
        } else if sat_f < 0.45 && self.t - self.last_sat > 6.0 {
            self.last_sat = self.t;
            let l = self.girl.lines.sat;
            let l = self.pick(l);
            self.say(l, 1);
        } else if rhythm == 1.0 && self.t - self.last_good > 5.5 && self.rand() < dt * 0.9 {
            self.last_good = self.t;
            let list = if fav > 1.0 {
                self.girl.lines.fav
            } else {
                self.girl.lines.good
            };
            let l = self.pick(list);
            self.say(l, 1);
        }
    }

    fn found_fav(&mut self) {
        self.fav_found = true;
        self.fav_t = 2.5;
        self.toast = Some(("¡SU PUNTO DÉBIL!", self.t));
        let l = self.girl.lines.fav;
        let l = self.pick(l);
        self.say(l, 3);
        self.sfx.push(UiSfx::Deal);
    }

    fn kiss(&mut self, x: f32, y: f32) {
        if self.kiss_cd > 0.0 {
            return;
        }
        self.kiss_cd = 0.5;
        let Some(z) = self.zone_at(x, y) else {
            return;
        };
        let beso = &TOOLS[5];
        let zd = &ZONES[z];
        if !zd.kiss {
            self.say("—Ahí… con la mano, Majestad.", 1);
            self.react(z, beso, x, 0.6);
            return;
        }
        if self.deseo < zd.min {
            self.conf = (self.conf - 4.0).clamp(0.0, 100.0);
            let s = self.side_of(x);
            self.body.head.v[0] += s * 120.0;
            self.body.head.v[1] += 30.0;
            self.react(z, beso, x, 0.5);
            let l = self.girl.lines.flinch;
            let l = self.pick(l);
            self.say(l, 2);
            if self.conf <= 0.0 {
                self.end(End::Stop);
            }
            return;
        }
        self.kisses += 1;
        self.kiss_t = 1.5;
        self.body.head.v[1] += 70.0;
        self.kick_breasts(0.0, 80.0);
        self.react(z, beso, x, 1.0);
        let sat = self.sat[z];
        let fav = if zd.id == self.girl.fav { 1.5 } else { 1.0 };
        let cloth = if self.covered(z) { 0.7 } else { 1.1 };
        self.gain(
            zd.gain * 1.6 * cloth * fav * (1.0 - sat / 7.0).max(0.2),
            Some(z),
        );
        self.sat[z] = sat + 0.6;
        for _ in 0..4 {
            let r = self.rand();
            self.heart(x + (r - 0.5) * 40.0, y, true);
        }
        if self.rand() < 0.45 {
            let l = self.girl.lines.kiss;
            let l = self.pick(l);
            self.say(l, 1);
        }
        if zd.id == self.girl.fav && !self.fav_found {
            self.found_fav();
        }
        self.sfx.push(UiSfx::Pick);
    }

    fn pinch(&mut self, x: f32, y: f32) {
        if self.kiss_cd > 0.0 {
            return;
        }
        self.kiss_cd = 0.35;
        let Some(z) = self.zone_at(x, y) else {
            return;
        };
        let t = &TOOLS[3];
        let zd = &ZONES[z];
        if !t.ok.contains(&zd.id) || self.deseo < zd.min + t.min {
            self.react(z, t, x, 1.0);
            self.flinch(Some(self.girl.lines.rough));
            return;
        }
        let sat = self.sat[z];
        let fav = if zd.id == self.girl.fav { 1.5 } else { 1.0 };
        let cloth = if self.covered(z) { 0.6 } else { 1.2 };
        self.gain(
            zd.gain * t.gain * cloth * fav * (1.0 - sat / 6.0).max(0.2),
            Some(z),
        );
        self.sat[z] = sat + 0.8;
        self.shake = 0.12;
        self.kick_pinch(x, y);
        self.react(z, t, x, 1.0);
        for _ in 0..3 {
            let r = self.rand();
            self.heart(x + (r - 0.5) * 30.0, y, false);
        }
        if self.rand() < 0.6 {
            let l = self.girl.lines.pinch;
            let l = self.pick(l);
            self.say(l, 2);
        }
    }

    fn try_remove(&mut self, i: usize) {
        let p = &self.girl.pieces[i];
        if self.deseo < p.unlock {
            self.conf = (self.conf - 6.0).clamp(0.0, 100.0);
            let l = self.girl.lines.locked;
            let l = self.pick(l);
            self.say(l, 2);
            self.shake = 0.2;
            self.sfx.push(UiSfx::Error);
            if self.conf <= 0.0 {
                self.end(End::Stop);
            }
            return;
        }
        self.worn[i] = false;
        self.undress_t = 2.0;
        self.kick_undress(p.id);
        self.gain(4.0, None);
        let line = self
            .girl
            .lines
            .undress
            .iter()
            .find(|(id, _)| *id == p.id)
            .map_or("—…", |(_, l)| *l);
        self.say(line, 4);
        self.sfx.push(UiSfx::Drop);
        if p.hair {
            self.toast = Some(("PELO SUELTO", self.t));
        }
        if self.worn.iter().all(|w| !w) {
            self.chq("—Ya no le queda nada que quitarle. …Yo no miro.", true);
        }
    }

    pub(crate) fn end(&mut self, kind: End) {
        if self.ended.is_some() {
            return;
        }
        self.ended = Some((kind, self.t));
        self.stroke = None;
        self.drag = None;
        let l = &self.girl.lines;
        let quote = match kind {
            End::Climax => l.climax,
            End::Stop => l.stop,
            End::Queen => l.thanks,
            End::Timeout => l.timeout,
        };
        self.say(quote, 9);
        if kind == End::Climax {
            self.kick_climax();
            for _ in 0..24 {
                let (a, b) = (self.rand(), self.rand());
                self.heart(700.0 + a * 520.0, 300.0 + b * 500.0, true);
            }
            self.sfx.push(UiSfx::Deal);
        }
    }

    /// The final clip's shot folder, for what she still wears.
    fn clip_dir(&self) -> Option<PathBuf> {
        let key = self.art_key();
        self.girl
            .clips
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, d)| {
                crate::exploration3d_state::data_dir()
                    .join("assets")
                    .join(d)
            })
            .filter(|d| super::clip_video::is_shot(d))
    }

    pub(crate) fn has_clip(&self) -> bool {
        self.clip_dir().is_some()
    }

    fn start_reel(&mut self) {
        if let Some(dir) = self.clip_dir() {
            let frames = super::clip_video::frames(&dir);
            if !frames.is_empty() {
                self.reel = Some(Reel {
                    frames,
                    dir,
                    time: 0.0,
                    shown: None,
                });
                self.card = false;
            }
        }
    }

    // ---- input

    pub(crate) fn pointer(&mut self, x: f32, y: f32) {
        let now = std::time::Instant::now();
        if let Some(last) = self.last_move {
            let d = (x - self.mouse[0]).hypot(y - self.mouse[1]);
            let dt = last.elapsed().as_secs_f32().max(0.012);
            self.speed = self.speed * 0.7 + (d / dt).min(3000.0) * 0.3;
            self.last_d = [x - self.mouse[0], y - self.mouse[1]];
        }
        self.last_move = Some(now);
        self.mouse = [x, y];
        if let Some(mut d) = self.drag {
            let p = &self.girl.pieces[d.piece];
            let len = p.dir[0].hypot(p.dir[1]).max(1e-3);
            d.proj = (((x - d.x0) * p.dir[0] + (y - d.y0) * p.dir[1]) / len).clamp(0.0, 180.0);
            self.drag = Some(d);
            if d.proj >= 30.0 && y > 1072.0 {
                self.drag = None;
                self.try_remove(d.piece);
            }
        }
    }

    pub(crate) fn press(&mut self, right: bool) {
        if self.ended.is_some() || self.paused {
            return;
        }
        let (x, y) = (self.mouse[0], self.mouse[1]);
        if right || self.tool().kind == Kind::Kiss {
            self.kiss(x, y);
            return;
        }
        if let Some(i) = self.handle_at(x, y) {
            self.drag = Some(Drag {
                piece: i,
                x0: x,
                y0: y,
                proj: 0.0,
            });
            return;
        }
        if self.tool().kind == Kind::Click {
            self.pinch(x, y);
            return;
        }
        self.stroke = Some(Stroke {
            blocked: false,
            x0: x,
            y0: y,
            zone: None,
            tense: 0.0,
            next_react: 0.0,
        });
        self.speed = 0.0;
    }

    pub(crate) fn release(&mut self) {
        if let Some(d) = self.drag.take() {
            if d.proj >= 140.0 {
                self.try_remove(d.piece);
            }
        }
        self.stroke = None;
    }

    pub(crate) fn set_tool(&mut self, i: usize) {
        self.tool = i % TOOLS.len();
        self.stroke = None;
    }

    // ---- each step

    pub(crate) fn step(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        self.t += dt;
        let playing = self.ended.is_none();
        if playing {
            self.time -= dt;
            if self.t > 2.6 && !self.start_line_two && self.t < 6.0 {
                self.start_line_two = true;
                let l = self.girl.lines.start[1];
                self.say(l, 3);
            }
            if self.t > 5.2 && self.last_chq < 0.0 {
                let l = self.girl.chq[0];
                self.chq(l, true);
            }
        }
        self.flinch_t = (self.flinch_t - dt).max(0.0);
        self.shake = (self.shake - dt).max(0.0);
        self.kiss_cd = (self.kiss_cd - dt).max(0.0);
        self.kiss_t = (self.kiss_t - dt).max(0.0);
        self.fav_t = (self.fav_t - dt).max(0.0);
        self.undress_t = (self.undress_t - dt).max(0.0);
        if playing {
            // The stroke's speed falls by itself when the hand stays still.
            if self
                .last_move
                .is_some_and(|m| m.elapsed().as_secs_f32() > 0.09)
            {
                self.speed *= 0.02f32.powf(dt);
            }
            self.touch(dt);
            for (k, s) in self.sat.iter_mut().enumerate() {
                if self.stroke.is_none() || self.zone != Some(k) {
                    *s = (*s - dt * 0.35).max(0.0);
                }
            }
            if self.stroke.is_none() {
                self.conf = (self.conf + 0.5 * dt).clamp(0.0, 100.0);
            }
            // Left alone, she cools down a little.
            if self.t - self.last_gain > 2.5 {
                self.deseo = (self.deseo - 0.8 * dt).max(0.0);
            }
            if self.t > 8.0 && self.t - self.last_chq > 18.0 {
                let l = self.girl.chq;
                let l = self.pick(l);
                self.chq(l, false);
            }
            if self.time <= 0.0 {
                self.end(End::Timeout);
            }
        }
        // After the end: the clip (when there is one), then the card.
        if let Some((kind, at)) = self.ended {
            let since = self.t - at;
            if self.reel.is_none() && !self.card {
                if kind == End::Climax && self.has_clip() && since > 0.35 {
                    self.start_reel();
                } else if since > if kind == End::Climax { 0.9 } else { 1.6 } {
                    self.card = true;
                }
            }
            if let Some(reel) = self.reel.as_mut() {
                reel.time += dt;
                if reel.time * 30.0 >= reel.frames.len() as f32 + 3.0 {
                    self.reel = None;
                    self.card = true;
                }
            }
        }
        self.physics(dt);
        self.faces(dt);
        for f in &mut self.fx {
            f.life -= dt;
            f.x += f.vx * dt;
            f.y += f.vy * dt;
        }
        self.fx.retain(|f| f.life > 0.0);
    }

    fn physics(&mut self, dt: f32) {
        let steps = (dt / 0.012).ceil().max(1.0) as usize;
        let h = dt / steps as f32;
        let heat = self.deseo;
        for n in 0..steps {
            let b = &mut self.body;
            for x in &mut b.br {
                for j in 0..2 {
                    let tgt = x.tgt.map_or(0.0, |t| t[j]);
                    x.v[j] += (-170.0 * (x.o[j] - tgt) - 7.0 * x.v[j]) * h;
                    x.o[j] = if j == 1 {
                        (x.o[j] + x.v[j] * h).clamp(-20.0, 40.0)
                    } else {
                        (x.o[j] + x.v[j] * h).clamp(-24.0, 24.0)
                    };
                }
                x.sq += (x.sq_t - x.sq) * (h * 12.0).min(1.0);
            }
            for x in &mut b.th {
                for j in 0..2 {
                    let tgt = x.tgt.map_or(0.0, |t| t[j]);
                    x.v[j] += (-210.0 * (x.o[j] - tgt) - 9.0 * x.v[j]) * h;
                    x.o[j] = (x.o[j] + x.v[j] * h).clamp(-18.0, 18.0);
                }
                x.sq += (x.sq_t - x.sq) * (h * 12.0).min(1.0);
            }
            spring(&mut b.body, 45.0, 5.0, -24.0, 24.0, h);
            spring(&mut b.hips, 55.0, 5.0, -30.0, 30.0, h);
            for sp in &mut b.spots {
                let mut s = Spring { o: sp.o, v: sp.v };
                spring(&mut s, 260.0, 9.0, -9.0, 9.0, h);
                sp.o = s.o;
                sp.v = s.v;
                sp.sqv += (-300.0 * sp.sq - 11.0 * sp.sqv) * h;
                sp.sq = (sp.sq + sp.sqv * h).clamp(-0.12, 0.16);
            }
            for s in &mut b.sh {
                spring(s, 90.0, 7.0, -20.0, 20.0, h);
            }
            for j in 0..2 {
                b.head.v[j] += (-70.0 * b.head.o[j] - 6.0 * b.head.v[j]) * h;
                b.head.o[j] = (b.head.o[j] + b.head.v[j] * h).clamp(-26.0, 26.0);
                b.hair_v[j] += (-14.0 * b.hair_a[j] - 1.7 * b.hair_v[j]) * h;
                b.hair_a[j] = (b.hair_a[j] + b.hair_v[j] * h).clamp(-0.04, 0.04);
            }
            b.jump[1] += (-120.0 * b.jump[0] - 11.0 * b.jump[1]) * h;
            b.jump[0] += b.jump[1] * h;
            let _ = n;
        }
        // The hair breathes and sways on its own, more when she is worked up.
        let t = self.t;
        let k = 1.0 + heat / 100.0;
        self.body.hair_v[0] += (t * 1.3).sin() * 0.0016 * k * dt * 10.0;
        self.body.hair_v[1] += (t * 1.1 + 1.7).sin() * 0.0016 * k * dt * 10.0;
        for d in &mut self.body.dents {
            d.life -= dt;
        }
        self.body.dents.retain(|d| d.life > 0.0 && !d.hand);
        // Hands on her: breasts and thighs follow a pressed hand, the hair swings when brushed.
        for x in self.body.br.iter_mut().chain(self.body.th.iter_mut()) {
            x.tgt = None;
            x.sq_t = 0.0;
        }
        let Some(stroke) = self.stroke.filter(|s| !s.blocked) else {
            return;
        };
        let tool = self.tool();
        let girl: &'static Girl = self.girl;
        let rig = &girl.rig;
        let a0 = to_art(self.mouse);
        let follow = (dt * 12.0).min(1.0);
        let sp0 = &mut self.body.spots[0];
        sp0.c[0] += (a0[0] - sp0.c[0]) * follow;
        sp0.c[1] += (a0[1] - sp0.c[1]) * follow;
        if tool.both {
            let m1 = [2.0 * rig.mid - a0[0], a0[1]];
            let sp1 = &mut self.body.spots[1];
            sp1.c[0] += (m1[0] - sp1.c[0]) * follow;
            sp1.c[1] += (m1[1] - sp1.c[1]) * follow;
        }
        let mut pts = vec![self.mouse];
        if tool.both {
            pts.push([self.mirror_x(self.mouse[0]), self.mouse[1]]);
        }
        let (dx, dy) = (self.mouse[0] - stroke.x0, self.mouse[1] - stroke.y0);
        let speed = self.speed;
        let last_d = self.last_d;
        for (k, p) in pts.into_iter().enumerate() {
            let a = to_art(p);
            let sx = if k == 1 { -dx } else { dx };
            let hold = tool.kind == Kind::Hold;
            for i in 0..2 {
                if in_ellipse(a, rig.thighs[i]) {
                    let (f, lim) = if hold { (0.4, 16.0) } else { (0.15, 8.0) };
                    self.body.th[i].tgt =
                        Some([(sx * f).clamp(-lim, lim), (dy * f).clamp(-lim, lim)]);
                    self.body.th[i].sq_t = if hold { 0.06 } else { 0.0 };
                }
                if in_ellipse(a, rig.breasts[i]) {
                    let (f, lim) = if hold { (0.45, 28.0) } else { (0.16, 10.0) };
                    self.body.br[i].tgt =
                        Some([(sx * f).clamp(-lim, lim), (dy * f).clamp(-lim, lim)]);
                    self.body.br[i].sq_t = if hold { 0.07 } else { 0.0 };
                }
            }
            if (a[0] - rig.mid).abs() > 95.0 && a[1] > rig.hair_pivot && a[1] < 780.0 {
                let side = if a[0] < rig.mid { 0 } else { 1 };
                let dir = if k == 1 { -last_d[0] } else { last_d[0] };
                self.body.hair_v[side] += speed.clamp(0.0, 1500.0) * 0.000_004_5 * dir.signum();
            }
            if tool.id == "dedo" {
                self.body.dents.push(Dent {
                    c: a,
                    r: [24.0, 24.0],
                    o: [0.0; 2],
                    sq: 0.07,
                    life: 0.05,
                    max: 0.05,
                    hand: true,
                });
            }
            if tool.id == "mano" || tool.both {
                let ddx = (last_d[0] * 0.4).clamp(-4.0, 4.0) * if k == 1 { -1.0 } else { 1.0 };
                let ddy = (last_d[1] * 0.4).clamp(-4.0, 4.0);
                self.body.dents.push(Dent {
                    c: a,
                    r: [50.0, 50.0],
                    o: [ddx, ddy],
                    sq: 0.02,
                    life: 0.06,
                    max: 0.06,
                    hand: true,
                });
            }
            if hold {
                self.body.dents.push(Dent {
                    c: a,
                    r: [62.0, 62.0],
                    o: [0.0; 2],
                    sq: 0.08,
                    life: 0.05,
                    max: 0.05,
                    hand: true,
                });
            }
        }
    }

    /// The face she should wear now (a blink now and then), crossing over from the last.
    fn faces(&mut self, dt: f32) {
        let mut f = "neutral";
        if self.flinch_t > 0.0 {
            f = "sorpresa";
        } else if matches!(self.ended, Some((End::Climax, _))) || self.deseo > 80.0 {
            f = "extasis";
        } else if self.deseo > 50.0 || self.fav_t > 0.0 {
            f = "placer";
        } else if self.deseo > 18.0 || self.kiss_t > 0.0 || self.undress_t > 0.0 {
            f = "sonrojada";
        }
        self.body.blink_at -= dt;
        if self.body.blink_at <= 0.0 {
            self.body.blink_t = 0.13;
            self.body.blink_at = 2.5 + self.rand() * 3.5;
        }
        self.body.blink_t = (self.body.blink_t - dt).max(0.0);
        if self.body.blink_t > 0.0 && (f == "neutral" || f == "sonrojada") {
            f = "parpadeo";
        }
        let set = self.face_set();
        let has = |name: &str| set.iter().any(|(n, _)| *n == name);
        if !has(f) {
            f = match f {
                "extasis" => ["placer", "sonrojada"]
                    .into_iter()
                    .find(|n| has(n))
                    .unwrap_or("neutral"),
                "placer" | "sorpresa" => {
                    if has("sonrojada") {
                        "sonrojada"
                    } else {
                        "neutral"
                    }
                }
                _ => "neutral",
            };
        }
        let b = &mut self.body;
        if f != b.face_b {
            b.face_a = if b.face_t >= 0.5 { b.face_b } else { b.face_a };
            b.face_b = f;
            b.face_t = 0.0;
        }
        let quick = f == "parpadeo" || b.face_a == "parpadeo";
        b.face_t = (b.face_t + dt * if quick { 30.0 } else { 9.0 }).min(1.0);
    }

    fn face_set(&self) -> &'static [(&'static str, &'static str)] {
        let rig = &self.girl.rig;
        let cap_on = self.piece_index("cofia").is_some_and(|i| self.worn[i]);
        if cap_on && !rig.faces_glasses.is_empty() {
            rig.faces_glasses
        } else {
            rig.faces
        }
    }

    fn face_picture(&self, name: &str) -> Option<&'static str> {
        self.face_set()
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, p)| *p)
    }

    /// The faces to cross: from, to, how far (0..1).
    pub(crate) fn face_pictures(&self) -> (Option<&'static str>, Option<&'static str>, f32) {
        (
            self.face_picture(self.body.face_a),
            self.face_picture(self.body.face_b),
            self.body.face_t,
        )
    }

    /// Every picture the night may show (loaded ahead).
    pub(crate) fn preload(&self) -> Vec<String> {
        let mut out = vec![self.girl.bg.to_owned()];
        out.extend(self.girl.art.iter().map(|(_, a)| (*a).to_owned()));
        out.extend(self.girl.rig.faces.iter().map(|(_, a)| (*a).to_owned()));
        out.extend(
            self.girl
                .rig
                .faces_glasses
                .iter()
                .map(|(_, a)| (*a).to_owned()),
        );
        out
    }

    // ---- her body bent: where each point of her art goes on the screen

    /// The deformers (art px ellipse, offset, squeeze), as the web page's shader read them.
    fn deformers(&self) -> Vec<([f32; 4], [f32; 2], f32)> {
        let rig = &self.girl.rig;
        let b = &self.body;
        let t = self.t;
        let rate = 1.1 + self.deseo / 40.0;
        let breath = -(t * rate * std::f32::consts::PI).sin() * (1.6 + self.deseo / 45.0);
        let mut d = vec![
            (rig.chest, [0.0, breath], 0.0),
            (rig.head, [b.head.o[0], b.head.o[1] + breath * 0.6], 0.0),
        ];
        for i in 0..2 {
            d.push((
                rig.breasts[i],
                [b.br[i].o[0], b.br[i].o[1] + breath * 0.8],
                b.br[i].sq,
            ));
        }
        for i in 0..2 {
            d.push((rig.thighs[i], b.th[i].o, b.th[i].sq));
        }
        d.push((rig.body, b.body.o, 0.0));
        d.push((rig.hips, b.hips.o, 0.0));
        for i in 0..2 {
            d.push((rig.shoulders[i], b.sh[i].o, 0.0));
        }
        for sp in &b.spots {
            if sp.on && sp.o[0].abs() + sp.o[1].abs() + sp.sq.abs() * 40.0 > 0.05 {
                d.push(([sp.c[0], sp.c[1], sp.r, sp.r * 0.9], sp.o, sp.sq));
            }
        }
        for l in &b.dents {
            let k = l.life / l.max;
            d.push((
                [l.c[0], l.c[1], l.r[0], l.r[1]],
                [l.o[0] * k, l.o[1] * k],
                l.sq * k,
            ));
        }
        d.truncate(16);
        d
    }

    fn hair_weight(&self, p: [f32; 2]) -> f32 {
        let Some((w, h, px)) = &self.hair else {
            return 0.0;
        };
        let x = ((p[0] / ART_W) * *w as f32) as i32;
        let y = ((p[1] / ART_H) * *h as f32) as i32;
        if x < 0 || y < 0 || x >= *w as i32 || y >= *h as i32 {
            return 0.0;
        }
        px[(y as u32 * w + x as u32) as usize] as f32 / 255.0
    }

    /// Where art point `q` is drawn now (screen px): the deformers pushed forward, the hair swung,
    /// the jump and the shake.
    fn warp(&self, q: [f32; 2], defs: &[([f32; 4], [f32; 2], f32)]) -> [f32; 2] {
        let mut p = q;
        for (c, o, sq) in defs {
            let d = [(q[0] - c[0]) / c[2], (q[1] - c[1]) / c[3]];
            let r2 = d[0] * d[0] + d[1] * d[1];
            if r2 >= 1.0 {
                continue;
            }
            let w = (1.0 - r2) * (1.0 - r2);
            p[0] += w * o[0];
            p[1] += w * o[1];
            let k = 1.0 / (1.0 + sq * w);
            p[0] = c[0] + (p[0] - c[0]) * k;
            p[1] = c[1] + (p[1] - c[1]) * k;
        }
        let hm = self.hair_weight(q);
        if hm > 0.002 {
            let rig = &self.girl.rig;
            let ang = if q[0] < rig.mid {
                self.body.hair_a[0]
            } else {
                self.body.hair_a[1]
            } * rig.hair_amp;
            let arm = (q[1] - rig.hair_pivot).max(0.0);
            p[0] += hm * ang * arm;
            p[1] -= hm * ang.abs() * arm * 0.12;
        }
        let s = from_art(p);
        let shake = if self.shake > 0.0 {
            (self.t * 70.0).sin() * 7.0 * self.shake / 0.35
        } else {
            0.0
        };
        [s[0] + shake, s[1] + self.body.jump[0] * art_scale()]
    }

    /// Her art as a bent grid: (columns, rows, the screen point of each corner, row by row).
    pub(crate) fn mesh(&self, cols: usize, rows: usize) -> Vec<[f32; 2]> {
        let defs = self.deformers();
        let mut out = Vec::with_capacity((cols + 1) * (rows + 1));
        for j in 0..=rows {
            for i in 0..=cols {
                let q = [
                    ART_W * i as f32 / cols as f32,
                    ART_H * j as f32 / rows as f32,
                ];
                out.push(self.warp(q, &defs));
            }
        }
        out
    }

    /// Her face patch as a bent grid over her face (the same bending).
    pub(crate) fn face_mesh(&self, n: usize) -> Vec<[f32; 2]> {
        let defs = self.deformers();
        let r = self.girl.rig.face_rect;
        let mut out = Vec::with_capacity((n + 1) * (n + 1));
        for j in 0..=n {
            for i in 0..=n {
                let q = [
                    r[0] + r[2] * i as f32 / n as f32,
                    r[1] + r[3] * j as f32 / n as f32,
                ];
                out.push(self.warp(q, &defs));
            }
        }
        out
    }

    /// The pace under the hand, as words.
    pub(crate) fn rhythm(&self) -> &'static str {
        let (Some(_), Some(z)) = (self.stroke, self.zone) else {
            return "";
        };
        let tool = self.tool();
        if tool.kind == Kind::Hold {
            return "APRETANDO";
        }
        let zd = &ZONES[z];
        let v = self.speed;
        if v > zd.spd[1] * tool.spd * 1.8 {
            "RITMO: ¡DEMASIADO RÁPIDO!"
        } else if v > zd.spd[1] * tool.spd {
            "RITMO: algo rápido"
        } else if v < zd.spd[0] * tool.spd * 0.4 {
            "RITMO: muy quieto"
        } else {
            "RITMO: bien"
        }
    }

    /// What the pointer is over, as words beside it.
    pub(crate) fn hint(&self) -> String {
        if self.drag.is_some() || self.ended.is_some() {
            return String::new();
        }
        let (x, y) = (self.mouse[0], self.mouse[1]);
        if let Some(i) = self.handle_at(x, y) {
            let p = &self.girl.pieces[i];
            return format!(
                "{}{}",
                if self.deseo >= p.unlock {
                    "Quitar: "
                } else {
                    "Aún no: "
                },
                p.name
            );
        }
        match self.zone_at(x, y) {
            Some(z) => {
                let zd = &ZONES[z];
                format!(
                    "{}{}",
                    zd.name,
                    if self.deseo < zd.min {
                        "  · aún no"
                    } else if self.covered(z) {
                        "  · sobre la ropa"
                    } else {
                        ""
                    }
                )
            }
            None => String::new(),
        }
    }

    pub(crate) fn removed(&self) -> usize {
        self.worn.iter().filter(|w| !**w).count()
    }

    pub(crate) fn max_deseo(&self) -> f32 {
        self.max_deseo
    }

    pub(crate) fn fav_name(&self) -> &'static str {
        if !self.fav_found {
            return "sin descubrir";
        }
        if self.girl.fav == "mano" {
            "la muñeca"
        } else {
            zone_index(self.girl.fav).map_or("", |z| ZONES[z].name)
        }
    }

    /// The final clip's frame to show now (a new one, or `None` while the same).
    pub(crate) fn reel_frame(&mut self) -> Option<PathBuf> {
        let reel = self.reel.as_mut()?;
        let i = ((reel.time * 30.0) as usize).min(reel.frames.len().saturating_sub(1));
        if reel.shown == Some(i) {
            return None;
        }
        reel.shown = Some(i);
        reel.frames.get(i).cloned()
    }

    pub(crate) fn replay(&mut self) {
        if self.has_clip() {
            self.start_reel();
        }
    }

    /// What Chiquita and the sounds have to say since last asked.
    fn take_out(&mut self) -> (Vec<String>, Vec<UiSfx>) {
        (
            std::mem::take(&mut self.chq_out),
            std::mem::take(&mut self.sfx),
        )
    }
}

// ------------------------------------------------------------------ the world's side

impl App {
    /// The touch game with `girl` (after her night's scene in `corte.md`).
    pub(super) fn open_caricias(&mut self, girl: &str) -> bool {
        let Some(g) = self::girl(girl) else {
            return false;
        };
        let seed = (self.elapsed * 1000.0) as u32 ^ 0x51ed_270b;
        self.caricias = Some(Caricias::new(g, seed));
        self.caricias_frame = None;
        self.clear_input();
        ui_sfx::play(UiSfx::Open);
        true
    }

    pub(super) fn caricias_open(&self) -> bool {
        self.caricias.is_some()
    }

    /// Each step: the night's clock, her body, what Chiquita says, the clip's frames.
    pub(super) fn tick_caricias(&mut self, dt: f32) {
        let Some(c) = self.caricias.as_mut() else {
            return;
        };
        c.step(dt);
        let (lines, sounds) = c.take_out();
        let frame = c.reel_frame();
        let reel_dir = c.reel.as_ref().map(|r| r.dir.clone());
        let sound_on = self.caricias_sound.clone();
        for s in sounds {
            ui_sfx::play(s);
        }
        for l in lines {
            self.float_chiquita_cue(&l, mascot::Cue::Talk);
        }
        if let Some(path) = frame {
            self.caricias_frame = super::clip_video::load(&path);
        }
        // The clip's sound with it.
        if reel_dir != sound_on {
            self.caricias_sound = reel_dir.clone();
            if let Some(audio) = self.audio.as_mut() {
                match reel_dir {
                    Some(dir) => audio.scene_sound(Some(&dir.join("audio.ogg")), false),
                    None => audio.scene_sound(None, false),
                }
            }
        }
    }

    /// Done: what it leaves (her fondness, the seal fed, the Queen's record), and back to bed.
    pub(super) fn close_caricias(&mut self) {
        let Some(c) = self.caricias.take() else {
            return;
        };
        if let Some(audio) = self.audio.as_mut() {
            audio.scene_sound(None, false);
        }
        self.caricias_sound = None;
        self.caricias_frame = None;
        let climax = matches!(c.ended, Some((End::Climax, _)));
        let girl = c.girl.id;
        let fond = format!("afecto_{girl}");
        let v = &mut self.story.values;
        if climax || c.max_deseo() >= 50.0 {
            *v.entry(fond).or_insert(0) += 1;
            *v.entry("deseo".to_owned()).or_insert(0) += if climax { 2 } else { 1 };
        }
        ero::encounter(v, girl, false, climax);
        self.clear_input();
        self.persist_3d_state(true);
    }

    pub(super) fn caricias_pointer(&mut self) {
        if let (Some((x, y)), Some(c)) = (self.shop_pointer_position(), self.caricias.as_mut()) {
            c.pointer(x, y);
        }
    }

    pub(super) fn caricias_button(&mut self, right: bool, pressed: bool) {
        self.caricias_pointer();
        let Some(c) = self.caricias.as_mut() else {
            return;
        };
        if c.card {
            if pressed && !right {
                self.close_caricias();
            }
            return;
        }
        if c.paused {
            return;
        }
        if pressed {
            c.press(right);
        } else if !right {
            c.release();
        }
    }

    pub(super) fn caricias_wheel(&mut self, down: bool) {
        if let Some(c) = self.caricias.as_mut() {
            let n = TOOLS.len();
            let i = (c.tool + if down { 1 } else { n - 1 }) % n;
            c.set_tool(i);
            ui_sfx::play(UiSfx::Move);
        }
    }

    pub(super) fn caricias_key(&mut self, code: KeyCode) {
        let Some(c) = self.caricias.as_mut() else {
            return;
        };
        if c.card {
            match code {
                KeyCode::KeyV if c.has_clip() => c.replay(),
                KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::KeyF
                | KeyCode::Space
                | KeyCode::Escape => self.close_caricias(),
                _ => {}
            }
            return;
        }
        if c.reel.is_some() {
            // The clip can be skipped to the card.
            if matches!(code, KeyCode::Escape | KeyCode::Enter | KeyCode::Space) {
                c.reel = None;
                c.card = true;
            }
            return;
        }
        if c.paused {
            match code {
                KeyCode::ArrowUp | KeyCode::KeyW | KeyCode::ArrowDown | KeyCode::KeyS => {
                    c.pause_choice = 1 - c.pause_choice;
                    ui_sfx::play(UiSfx::Move);
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::KeyF | KeyCode::Space => {
                    if c.pause_choice == 0 {
                        c.paused = false;
                    } else {
                        c.paused = false;
                        c.end(End::Queen);
                    }
                    ui_sfx::play(UiSfx::Select);
                }
                KeyCode::Escape => {
                    c.paused = false;
                    ui_sfx::play(UiSfx::Back);
                }
                _ => {}
            }
            return;
        }
        if c.ended.is_some() {
            return;
        }
        let tool = match code {
            KeyCode::Digit1 | KeyCode::Numpad1 => Some(0),
            KeyCode::Digit2 | KeyCode::Numpad2 => Some(1),
            KeyCode::Digit3 | KeyCode::Numpad3 => Some(2),
            KeyCode::Digit4 | KeyCode::Numpad4 => Some(3),
            KeyCode::Digit5 | KeyCode::Numpad5 => Some(4),
            KeyCode::Digit6 | KeyCode::Numpad6 => Some(5),
            _ => None,
        };
        if let Some(t) = tool {
            c.set_tool(t);
            ui_sfx::play(UiSfx::Move);
            return;
        }
        if code == KeyCode::Escape {
            c.paused = true;
            c.pause_choice = 0;
            c.release();
            ui_sfx::play(UiSfx::Open);
        }
    }

    /// The overlay's half: her pictures, the bent mesh, the HUD.
    pub(super) fn caricias_view(&self) -> Option<map_overlay::CariciasView> {
        let c = self.caricias.as_ref()?;
        let q = |v: f32| (v * 4.0).round() as i32;
        let ms = |t: f32| (t.max(0.0) * 1000.0) as u32;
        let (fa, fb, ft) = c.face_pictures();
        const COLS: usize = 56;
        const ROWS: usize = 32;
        const FACE_N: usize = 6;
        let ended = c.ended.map(|(k, at)| (k, c.t - at));
        let card = c.card.then(|| {
            let (kind, _) = c.ended.unwrap_or((End::Queen, 0.0));
            let secs = c.t.round() as i32;
            map_overlay::CariciasCard {
                title: match kind {
                    End::Climax => "FINAL",
                    End::Stop => "LO DEJASTEIS AQUÍ",
                    End::Queen => "PARASTE TÚ",
                    End::Timeout => "SE APAGÓ LA VELA",
                }
                .to_owned(),
                quote: c.line().map(|(t, _)| t.to_owned()).unwrap_or_else(|| {
                    let l = &c.girl.lines;
                    match kind {
                        End::Climax => l.climax,
                        End::Stop => l.stop,
                        End::Queen => l.thanks,
                        End::Timeout => l.timeout,
                    }
                    .to_owned()
                }),
                stats: vec![
                    (
                        "Tiempo".to_owned(),
                        format!("{}:{:02}", secs / 60, secs % 60),
                    ),
                    (
                        "Deseo máximo".to_owned(),
                        format!("{}", c.max_deseo().round() as i32),
                    ),
                    (
                        "Ropa quitada".to_owned(),
                        format!("{} de {}", c.removed(), c.girl.pieces.len()),
                    ),
                    ("Punto débil".to_owned(), c.fav_name().to_owned()),
                    ("Sustos".to_owned(), c.flinches.to_string()),
                    ("Besos".to_owned(), c.kisses.to_string()),
                ],
                clip: c.has_clip() && matches!(kind, End::Climax),
            }
        });
        let reel = c.reel.is_some();
        Some(map_overlay::CariciasView {
            name: c.girl.name.to_owned(),
            title: c.girl.title.to_owned(),
            accent: c.girl.accent,
            bg: c.girl.bg.to_owned(),
            art: c.art().unwrap_or_default().to_owned(),
            preload: c.preload(),
            mesh: if reel {
                Vec::new()
            } else {
                c.mesh(COLS, ROWS)
                    .into_iter()
                    .map(|p| [q(p[0]), q(p[1])])
                    .collect()
            },
            cols: COLS as u16,
            rows: ROWS as u16,
            face_a: fa.unwrap_or_default().to_owned(),
            face_b: fb.unwrap_or_default().to_owned(),
            face_t: (ft * 1000.0) as u32,
            face_mesh: if reel {
                Vec::new()
            } else {
                c.face_mesh(FACE_N)
                    .into_iter()
                    .map(|p| [q(p[0]), q(p[1])])
                    .collect()
            },
            face_n: FACE_N as u16,
            deseo: (c.deseo * 10.0) as u32,
            conf: (c.conf * 10.0) as u32,
            time_ms: ms(c.time),
            tool: c.tool as u8,
            rhythm: c.rhythm().to_owned(),
            hint: c.hint(),
            mouse: [q(c.mouse[0]), q(c.mouse[1])],
            mirror: c.tool().both.then(|| q(c.mirror_x(c.mouse[0]))),
            line: c
                .line()
                .map(|(t, left)| (t.to_owned(), (left.clamp(0.0, 0.6) / 0.6 * 1000.0) as u32)),
            handles: (0..c.girl.pieces.len())
                .filter(|&i| c.piece_open(i) && ended.is_none())
                .map(|i| {
                    let h = c.handle(i);
                    let d = c.handle_dir(i);
                    (
                        q(h[0]),
                        q(h[1]),
                        q(d[0] * 100.0),
                        q(d[1] * 100.0),
                        c.piece_ready(i),
                    )
                })
                .collect(),
            fx: c
                .fx
                .iter()
                .map(|f| {
                    (
                        f.kind,
                        q(f.x),
                        q(f.y),
                        (f.s * 100.0) as u16,
                        ((f.life / f.max).clamp(0.0, 1.0) * 1000.0) as u16,
                    )
                })
                .collect(),
            tint: (c.deseo / 100.0 * 320.0) as u32,
            cold: if c.conf < 25.0 {
                ((25.0 - c.conf) / 25.0 * 600.0) as u32
            } else {
                0
            },
            toast: c
                .toast
                .filter(|(_, at)| c.t - at < 1.9)
                .map(|(t, at)| (t.to_owned(), ms(c.t - at))),
            paused: c.paused.then_some(c.pause_choice as u8),
            card,
            reel,
            playing: ended.is_none() && !c.paused,
            t_ms: ms(c.t),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zone_and_handle_is_on_her_art_and_the_hands_find_them() {
        for g in [&LIRA, &NYXARA, &MAREN] {
            for (id, poly) in g.zones {
                assert!(zone_index(id).is_some(), "{} {id}", g.id);
                for p in poly.iter() {
                    assert!((0.0..=ART_W).contains(&p[0]) && (0.0..=ART_H).contains(&p[1]));
                }
            }
            for p in g.pieces {
                assert!(
                    (0.0..=ART_W).contains(&p.handle[0]) && (0.0..=ART_H).contains(&p.handle[1])
                );
            }
            // The art keys follow the order the pieces come off.
            let mut c = Caricias::new(g, 3);
            assert!(c.art().is_some(), "{}: all on", g.id);
            for i in 0..g.pieces.len() {
                c.worn[i] = false;
                assert!(c.art().is_some(), "{}: {} off", g.id, g.pieces[i].id);
            }
            // The middle of her chest is her chest.
            let chest = from_art([g.rig.chest[0], g.rig.chest[1] - 10.0]);
            assert_eq!(
                c.zone_at(chest[0], chest[1]).map(|z| ZONES[z].id),
                Some("pecho")
            );
        }
    }

    #[test]
    fn a_slow_stroke_where_she_likes_it_warms_her_and_a_rough_grab_makes_her_flinch() {
        let mut c = Caricias::new(&LIRA, 9);
        // Her hair, slowly: desire goes up, nobody flinches.
        let hair = from_art([850.0, 100.0]);
        c.pointer(hair[0], hair[1]);
        c.press(false);
        for k in 0..240 {
            c.speed = 200.0;
            c.mouse = [hair[0] + (k as f32 * 0.3).sin() * 30.0, hair[1]];
            c.step(1.0 / 60.0);
        }
        // Over her cap, and she gets used to it: slowly (as on the web page).
        assert!(c.deseo > 1.0, "{}", c.deseo);
        assert_eq!(c.flinches, 0);
        c.release();
        // A grab on her chest this early: she flinches.
        c.set_tool(2);
        let chest = from_art([850.0, 500.0]);
        c.pointer(chest[0], chest[1]);
        c.press(false);
        for _ in 0..60 {
            c.step(1.0 / 60.0);
        }
        assert!(c.flinches >= 1);
    }

    #[test]
    fn nothing_comes_off_too_soon_and_the_end_needs_her_bare() {
        let mut c = Caricias::new(&NYXARA, 4);
        c.try_remove(0);
        assert!(c.worn[0], "the cap stays before the desire it asks");
        c.deseo = 94.0;
        c.gain(50.0, None);
        assert!(c.deseo <= 95.0 && c.ended.is_none(), "dressed: no end");
        for i in 0..c.worn.len() {
            c.deseo = 90.0;
            c.try_remove(i);
        }
        assert!(c.worn.iter().all(|w| !w));
        c.gain(100.0, None);
        assert_eq!(c.ended.map(|(k, _)| k), Some(End::Climax));
    }

    #[test]
    fn her_body_bends_and_comes_back_to_rest() {
        let mut c = Caricias::new(&LIRA, 2);
        for _ in 0..600 {
            c.step(1.0 / 60.0);
        }
        let rest = c.mesh(8, 4);
        // Nothing far from where it was drawn (the breath only).
        for (j, p) in rest.iter().enumerate() {
            let (i, r) = (j % 9, j / 9);
            let q = from_art([ART_W * i as f32 / 8.0, ART_H * r as f32 / 4.0]);
            assert!(
                (p[0] - q[0]).abs() < 4.0 && (p[1] - q[1]).abs() < 4.0,
                "{p:?} {q:?}"
            );
        }
    }
}
