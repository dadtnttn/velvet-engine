//! «Otro trato»: el regateo picante de la tienda de la villa (`DemonQueen-Lore/TIENDA_REGATEO.md`).
//! Cuando el regateo se atasca (o el tendero ya la conoce) él insinúa que hay otras formas de pagar,
//! y en la página del regateo aparece una sexta casilla con el favor que pide y la rebaja que da.
//! Los favores son una escalera, de una nalgada a encima en su taburete: cada día él se atreve con
//! uno nuevo, y los ya hechos se pueden repetir. Uno por regateo.
//!
//! El favor se juega en el escenario de las escenas íntimas: su toma en bucle (el clip y su puente
//! de vuelta, `tools/escena_clips.py tienda`), lo que él dice y lo que ella contesta, Chiquita en
//! su esquina. El vial es el trato: se llena solo, y con Espacio más deprisa. Lleno, trato hecho:
//! la rebaja se suma al regateo y el sello come. Esc: se echa atrás y el precio sigue como estaba.
use super::*;

/// The story variables of the ladder: how many favours she has done (the next one is the new one)
/// and whether a new one was done today (sleeping clears it).
pub(crate) const LADDER: &str = "tienda_favor";
pub(crate) const TODAY: &str = "tienda_favor_hoy";

/// One favour: its loop, its name on the tile, the cut it buys, what is said, what it feeds.
pub(crate) struct Favor {
    /// Its loop (`tienda/escena/<shot>`).
    pub shot: &'static str,
    pub name: &'static str,
    /// How much more the price comes down (percent; the sum stops at free).
    pub percent: u32,
    /// What he asks (in the shop, and as the scene opens), her answer, Chiquita's aside.
    pub ask: &'static str,
    pub(super) answer: (Voice, &'static str),
    pub(super) aside: (&'static str, mascot::Cue),
    /// What he says as the deal fills, in turn.
    pub during: [&'static str; 2],
    /// His last word.
    pub after: &'static str,
    /// The seal: its hunger down, desire up.
    pub hunger: i32,
    pub deseo: i32,
    /// The light of the room.
    pub tint: [u8; 3],
}

const WARM: [u8; 3] = [0xff, 0xc0, 0x70];
const HOT: [u8; 3] = [0xff, 0x8a, 0x80];
const RED: [u8; 3] = [0xff, 0x5a, 0x8a];

pub(crate) const FAVORS: [Favor; 11] = [
    Favor {
        shot: "nalgada",
        name: "Una nalgada",
        percent: 10,
        ask: "—Un diez por ciento más… si me deja darle una palmadita. Por el comercio, entiéndame.",
        answer: (Voice::Queen, "—Una. Y rápido."),
        aside: ("—Una nalgada a la Reina Demonio por un diez por ciento. Me preocupa la economía de este reino.", mascot::Cue::Scold),
        during: ["—¡Firme! Como el pan de ayer.", "—Ay, Majestad, qué mercancía."],
        after: "—Diez por ciento. Un placer hacer negocios, Majestad.",
        hunger: -1,
        deseo: 1,
        tint: WARM,
    },
    Favor {
        shot: "pecho",
        name: "Tocarle el pecho",
        percent: 15,
        ask: "—Quince, si me deja… comprobar la mercancía. Como con la fruta.",
        answer: (Voice::Queen, "—Compruébala y calla."),
        aside: ("—Como con la fruta, dice. Ese hombre no ha tocado una fruta en su vida.", mascot::Cue::Jealous),
        during: ["—Madura. Muy madura.", "—Esto en los mercados del este no lo tienen."],
        after: "—Quince por ciento. Género de primera, sí señora.",
        hunger: -2,
        deseo: 1,
        tint: WARM,
    },
    Favor {
        shot: "cara_poner",
        name: "Ponérsela en la cara",
        percent: 20,
        ask: "—Veinte por ciento… si me deja ponérsela en la cara. Quietecita, nada más.",
        answer: (Voice::QueenAngry, "—…Si alguien entra, te convierto en sapo."),
        aside: ("—Quietecita, dice. Como si fueras parte del mostrador.", mascot::Cue::Flustered),
        during: ["—Así. Quietecita. Qué bien le queda.", "—Si la viera su Consejo…"],
        after: "—Veinte por ciento. Y no se lo cuento a nadie. Palabra de comerciante.",
        hunger: -2,
        deseo: 2,
        tint: WARM,
    },
    Favor {
        shot: "cara_frotar",
        name: "Pasársela por la cara",
        percent: 25,
        ask: "—¿Y si se la paso un poco? Por la mejilla. Veinticinco.",
        answer: (Voice::Queen, "—Por la mejilla. Solo por la mejilla."),
        aside: ("—Por veinticinco te dejas pintar la cara. Por treinta, ¿qué?", mascot::Cue::Scold),
        during: ["—Suave como la seda de oriente…", "—Qué cara más de reina, Majestad."],
        after: "—Veinticinco. Y la cara brillante se la dejo gratis.",
        hunger: -3,
        deseo: 2,
        tint: HOT,
    },
    Favor {
        shot: "cara_pegar",
        name: "Darle con ella en la cara",
        percent: 30,
        ask: "—Treinta… si me deja darle unos golpecitos con ella. Con cariño.",
        answer: (Voice::QueenAngry, "—Como cuentes una palabra de esto en la taberna…"),
        aside: ("—Trescientos años de reinado para esto. Que conste que la culpa es tuya.", mascot::Cue::Jealous),
        during: ["—Pim. Pam. Con cariño, ¿eh?", "—Esto, por los impuestos del año pasado."],
        after: "—Treinta por ciento. Y estamos en paz con los impuestos.",
        hunger: -3,
        deseo: 2,
        tint: HOT,
    },
    Favor {
        shot: "chupar",
        name: "Que se la chupe",
        percent: 40,
        ask: "—Si me la chupa, Majestad… cuarenta por ciento. Casi a mitad de precio.",
        answer: (Voice::Queen, "—Cuarenta. Y cierra los ojos, que me miras."),
        aside: ("—La harina sube cada semana. Técnicamente, es una inversión.", mascot::Cue::Away),
        during: ["—Ay… ay, Majestad… qué manera de regatear.", "—Esto no lo hace la competencia."],
        after: "—Cuarenta por ciento. Vuelva cuando quiera. Cuando quiera, ¿eh?",
        hunger: -4,
        deseo: 3,
        tint: HOT,
    },
    Favor {
        shot: "garganta",
        name: "Hasta el fondo",
        percent: 50,
        ask: "—Hasta el fondo, Majestad, y le dejo el género a mitad de precio.",
        answer: (Voice::QueenAngry, "—Hasta el fondo. Y si te ríes, te muerdo."),
        aside: ("—Si te ahogas por un cincuenta por ciento, no pienso decir unas palabras en tu funeral.", mascot::Cue::Scold),
        during: ["—¡Mitad de precio! ¡Mitad de precio!", "—Respire, Majestad… o no. Como prefiera."],
        after: "—Cincuenta. Y que conste que me ha ganado usted.",
        hunger: -5,
        deseo: 3,
        tint: HOT,
    },
    Favor {
        shot: "mejilla",
        name: "Por el lado, que se note",
        percent: 60,
        ask: "—Déjeme… por el lado. Que se le note en la mejilla. Sesenta.",
        answer: (Voice::Queen, "—Sesenta. Y las velas."),
        aside: ("—Se te ve desde la calle. Hay una señora con una cesta mirando.", mascot::Cue::Flustered),
        during: ["—Mire cómo se le marca… Una obra de arte.", "—Sesenta y las velas. Usted sí que sabe."],
        after: "—Sesenta por ciento. Las velas, de regalo. Ya sabe dónde estoy.",
        hunger: -5,
        deseo: 3,
        tint: RED,
    },
    Favor {
        shot: "perrito",
        name: "Por detrás, en el mostrador",
        percent: 75,
        ask: "—Inclínese sobre el mostrador, Majestad. Y le dejo el precio en una cuarta parte.",
        answer: (Voice::Queen, "—Cierra la puerta."),
        aside: ("—Al menos cierra la puerta. …No la cierra.", mascot::Cue::Away),
        during: ["—¡Ay, Majestad! ¡Que se caen los tarros!", "—Esto… esto es comercio justo."],
        after: "—Setenta y cinco por ciento. Los tarros rotos, de mi cuenta.",
        hunger: -6,
        deseo: 4,
        tint: RED,
    },
    Favor {
        shot: "mostrador",
        name: "Tumbada entre las monedas",
        percent: 90,
        ask: "—Túmbese encima de las monedas. Las que se le queden pegadas, suyas.",
        answer: (Voice::Queen, "—Las de oro también."),
        aside: ("—Te vas a llevar el cambio pegado a la espalda. Muy de reina.", mascot::Cue::Jealous),
        during: ["—Cuente, cuente las que se le pegan…", "—La mejor venta de mi vida, Majestad."],
        after: "—Noventa por ciento. Y lo pegado, suyo, como le dije.",
        hunger: -7,
        deseo: 4,
        tint: RED,
    },
    Favor {
        shot: "vaquera",
        name: "Encima, en su taburete",
        percent: 100,
        ask: "—Súbase usted, Majestad. Y le regalo… la estantería entera.",
        answer: (Voice::Queen, "—La tienda."),
        aside: ("—La Reina Demonio, dueña de una tienda de ultramarinos. Mi madre estaría orgullosa.", mascot::Cue::Cheer),
        during: ["—…La tienda. ¡Está bien, la tienda!", "—¡Mi taburete no estaba hecho para esto!"],
        after: "—Gratis, Majestad. Hoy, gratis. …Lo de la tienda era broma, ¿verdad?",
        hunger: -8,
        deseo: 5,
        tint: RED,
    },
];

/// The loops of the favours, for the reel (each loops on itself).
pub(super) const SHOTS: [(&str, bool); 11] = [
    ("nalgada", true),
    ("pecho", true),
    ("cara_poner", true),
    ("cara_frotar", true),
    ("cara_pegar", true),
    ("chupar", true),
    ("garganta", true),
    ("mejilla", true),
    ("perrito", true),
    ("mostrador", true),
    ("vaquera", true),
];

/// How long the deal takes to fill on its own (s), and how much faster with Space held.
const FILL_SECS: f32 = 13.0;
const HURRY: f32 = 1.9;
/// How long the rules stay up (they are short).
const RULES_SECS: f32 = 4.2;

/// The favours she can offer now: those done (again, any time) and the next one, if today has not
/// had its new one yet. The second value is that new one.
pub(crate) fn available(story: &guion::Vars) -> (Vec<usize>, Option<usize>) {
    let done = (story.get(LADDER).max(0) as usize).min(FAVORS.len());
    let fresh = (done < FAVORS.len() && story.get(TODAY) <= 0).then_some(done);
    let mut all: Vec<usize> = (0..done).collect();
    all.extend(fresh);
    (all, fresh)
}

/// What the favour scene keeps.
#[derive(Default)]
pub(super) struct TiendaGame {
    pub favor: usize,
    /// His lines said while it fills.
    said: usize,
    /// When the deal was sealed.
    sealed: Option<f32>,
}

impl PlayScene {
    pub(super) fn favor(&self) -> &'static Favor {
        &FAVORS[self.tienda.favor.min(FAVORS.len() - 1)]
    }

    /// The scene opens on his ask, her answer and Chiquita's aside.
    pub(super) fn open_tienda(&mut self, favor: usize) {
        self.tienda.favor = favor.min(FAVORS.len() - 1);
        self.rules_until = RULES_SECS;
        self.talk.free_at = 0.6;
        let f = self.favor();
        self.reel.play(f.shot, 0.0, 0.0);
        self.talk.say(Voice::Tendero, f.ask);
        self.talk.say(f.answer.0, f.answer.1);
        self.chq_free = 0.0;
        self.chiquita(f.aside.0, f.aside.1, true);
        self.tint = f.tint.map(f32::from);
    }

    /// Each step: the deal fills (faster with Space held, the loop too), he talks as it goes, and
    /// full, it is sealed.
    pub(super) fn step_tienda(&mut self, dt: f32) {
        let now = self.clock;
        if self.phase != Phase::Play {
            return;
        }
        let hurry = if self.holding { HURRY } else { 1.0 };
        self.reel.rate = if self.holding { 1.18 } else { 1.0 };
        self.heat = (self.heat + dt * hurry / FILL_SECS).min(1.0);
        let f = self.favor();
        if self.tienda.said < 2
            && self.heat >= [0.34, 0.68][self.tienda.said]
            && self.talk.idle(now)
        {
            self.talk.say(Voice::Tendero, f.during[self.tienda.said]);
            self.tienda.said += 1;
        }
        if self.heat >= 1.0 {
            self.tienda.sealed = Some(now);
            self.stamp("¡TRATO!", [0xff, 0xd8, 0x6a]);
            self.flash([0xff, 0xd0, 0x80], 0.8);
            self.reel.rate = 1.0;
            self.talk.reply(Voice::Tendero, f.after);
            self.phase = Phase::Relax(now + 3.2);
            self.phase_since = now;
        }
    }

    pub(super) fn tienda_rules(&self) -> (String, Vec<(String, String)>) {
        let f = self.favor();
        let cut = if f.percent >= 100 {
            "Gratis si llegas al final.".to_owned()
        } else {
            format!("El precio baja un {} % más si llegas al final.", f.percent)
        };
        (
            format!("Otro trato · {}", f.name),
            vec![
                (
                    "Espacio".to_owned(),
                    "Mantén para darle prisa: el trato se cierra antes.".to_owned(),
                ),
                ("Trato".to_owned(), cut),
                (
                    "Esc".to_owned(),
                    "Echarse atrás: el precio sigue como estaba.".to_owned(),
                ),
            ],
        )
    }

    /// The scene's words for the stage's top line.
    pub(super) fn tienda_phase(&self) -> String {
        match self.phase {
            Phase::Play if self.holding => "Le das prisa…".to_owned(),
            Phase::Play => "El tendero cobra su parte del trato".to_owned(),
            _ => "Trato hecho".to_owned(),
        }
    }

    /// The card: the cut and the seal fed when it was sealed; nothing when she backed out.
    pub(super) fn tienda_ending(
        &self,
        done: bool,
    ) -> (
        &'static str,
        Vec<(&'static str, i32)>,
        (Voice, &'static str),
        String,
    ) {
        let f = self.favor();
        let cut = if f.percent >= 100 {
            "gratis".to_owned()
        } else {
            format!("−{} %", f.percent)
        };
        let how = format!("Tendero · {} · {cut}", f.name);
        if !done {
            return (
                "Te echas atrás",
                Vec::new(),
                (
                    Voice::Tendero,
                    "—¿Ya? Pues el precio sigue como estaba, Majestad.",
                ),
                how,
            );
        }
        (
            "¡Trato hecho!",
            vec![("hunger", f.hunger), ("deseo", f.deseo)],
            (Voice::Tendero, f.after),
            how,
        )
    }

    pub(super) fn tienda_thought(&self) -> &'static str {
        match (self.ended.unwrap_or(false), self.tienda.favor) {
            (false, _) => "Mejor pago con oro. Por hoy.",
            (true, 0..=1) => "Una rebaja es una rebaja. Y el sello ha comido algo.",
            (true, 2..=4) => "Si el Consejo se entera de cómo regateo, me quitan la corona.",
            (true, 5..=7) => "El sello ronronea. Mi dignidad, no tanto.",
            (true, _) => "La mejor compra del reino. No pienso contarla nunca.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn story(pairs: &[(&str, i32)]) -> guion::Vars {
        let mut v = guion::Vars::default();
        for (k, n) in pairs {
            v.values.insert((*k).to_owned(), *n);
        }
        v
    }

    #[test]
    fn the_ladder_opens_one_new_favour_a_day_and_keeps_the_old_ones() {
        assert_eq!(available(&story(&[])), (vec![0], Some(0)));
        assert_eq!(
            available(&story(&[(LADDER, 3)])),
            (vec![0, 1, 2, 3], Some(3))
        );
        // A new one done today: only the old ones until she sleeps.
        assert_eq!(
            available(&story(&[(LADDER, 3), (TODAY, 1)])),
            (vec![0, 1, 2], None)
        );
        // All done: they are all repeatable, none is new.
        assert_eq!(
            available(&story(&[(LADDER, 11)])),
            ((0..11).collect(), None)
        );
    }

    #[test]
    fn every_favour_has_its_loop_and_climbs() {
        for (i, f) in FAVORS.iter().enumerate() {
            assert_eq!(SHOTS[i].0, f.shot);
            if i > 0 {
                assert!(f.percent > FAVORS[i - 1].percent, "{}", f.name);
            }
        }
        assert_eq!(FAVORS.last().map(|f| f.percent), Some(100));
    }
}
