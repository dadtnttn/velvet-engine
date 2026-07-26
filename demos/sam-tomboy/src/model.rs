use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeSlot {
    Morning,
    Noon,
    Afternoon,
    Evening,
    Night,
}

impl TimeSlot {
    pub fn label(self) -> &'static str {
        match self {
            Self::Morning => "MAÑANA",
            Self::Noon => "MEDIODÍA",
            Self::Afternoon => "TARDE",
            Self::Evening => "ATARDECER",
            Self::Night => "NOCHE",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Morning => Self::Noon,
            Self::Noon => Self::Afternoon,
            Self::Afternoon => Self::Evening,
            Self::Evening | Self::Night => Self::Night,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocationId {
    Train,
    Plaza,
    Cafe,
    Garage,
    BoardingHouse,
}

impl LocationId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Train => "ESTACIÓN",
            Self::Plaza => "PLAZA",
            Self::Cafe => "CAFÉ LUCERO",
            Self::Garage => "TALLER NORTE",
            Self::BoardingHouse => "PENSIÓN AZUL",
        }
    }

    pub fn background(self) -> &'static str {
        match self {
            Self::Train => "train",
            Self::Plaza => "plaza",
            Self::Cafe => "cafe",
            Self::Garage => "garage",
            Self::BoardingHouse => "boarding",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndingId {
    SafeRoom,
    HonestWork,
    StreetPromise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneId {
    TrainAwakening,
    TrainTicket,
    TrainArrival,
    PlazaFirst,
    PlazaTruth,
    PlazaObserve,
    CafeFirst,
    CafeTruth,
    CafeWork,
    CafeMeal,
    CafeWater,
    GarageFirst,
    GarageTruth,
    GarageSweep,
    GarageWork,
    BoardingFirst,
    BoardingPay,
    BoardingClean,
    BoardingLeave,
    TrainReturn,
    NightDecision,
    NightStreet,
    Ending(EndingId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub day: u32,
    pub time: TimeSlot,
    pub money: i32,
    pub hunger: i32,
    pub energy: i32,
    pub trust: i32,
    pub has_room: bool,
    pub job_offer: bool,
    pub cafe_worked: bool,
    pub garage_worked: bool,
    pub plaza_met_lina: bool,
    pub boarding_met_ines: bool,
    pub truth_moments: u32,
    pub visited: Vec<LocationId>,
    pub current_scene: SceneId,
    #[serde(default)]
    pub resume_on_map: bool,
    pub completed: bool,
    pub ending: Option<EndingId>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            day: 1,
            time: TimeSlot::Morning,
            money: 6,
            hunger: 28,
            energy: 86,
            trust: 0,
            has_room: false,
            job_offer: false,
            cafe_worked: false,
            garage_worked: false,
            plaza_met_lina: false,
            boarding_met_ines: false,
            truth_moments: 0,
            visited: Vec::new(),
            current_scene: SceneId::TrainAwakening,
            resume_on_map: false,
            completed: false,
            ending: None,
        }
    }
}

impl GameState {
    pub fn clamp_stats(&mut self) {
        self.hunger = self.hunger.clamp(0, 100);
        self.energy = self.energy.clamp(0, 100);
        self.trust = self.trust.clamp(0, 100);
        self.money = self.money.max(0);
    }

    pub fn visit(&mut self, location: LocationId) {
        if !self.visited.contains(&location) {
            self.visited.push(location);
        }
    }

    pub fn advance_time(&mut self) {
        self.time = self.time.next();
        self.hunger += 12;
        self.energy -= 7;
        self.clamp_stats();
    }

    pub fn needs_night_resolution(&self) -> bool {
        self.time == TimeSlot::Night
    }

    pub fn choose_ending(&self) -> EndingId {
        if self.has_room && self.job_offer {
            EndingId::HonestWork
        } else if self.has_room {
            EndingId::SafeRoom
        } else {
            EndingId::StreetPromise
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SamLook {
    pub pose: u8,
    pub outfit: &'static str,
    pub expression: &'static str,
}

#[derive(Debug, Clone)]
pub struct ChoiceView {
    pub text: String,
    pub enabled: bool,
    pub hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SceneView {
    pub location: LocationId,
    pub speaker: &'static str,
    pub text: String,
    pub choices: Vec<ChoiceView>,
    pub look: SamLook,
}

fn choice(text: impl Into<String>) -> ChoiceView {
    ChoiceView {
        text: text.into(),
        enabled: true,
        hint: None,
    }
}

fn locked(text: impl Into<String>, hint: impl Into<String>) -> ChoiceView {
    ChoiceView {
        text: text.into(),
        enabled: false,
        hint: Some(hint.into()),
    }
}

pub fn scene_view(scene: SceneId, state: &GameState) -> SceneView {
    use LocationId::*;
    use SceneId::*;

    match scene {
        TrainAwakening => SceneView {
            location: Train,
            speaker: "SAM",
            text: "El tren avanza bajo una lluvia que Sam nunca había visto. En su mundo, nadie oculta lo que piensa. Aquí, todos miran sus teléfonos y evitan encontrarse los ojos.".into(),
            choices: vec![choice("Mirar por la ventana")],
            look: SamLook { pose: 1, outfit: "hoodie", expression: "surprised" },
        },
        TrainTicket => SceneView {
            location: Train,
            speaker: "REVISOR",
            text: "—¿Billete?\n\nSam revisa sus bolsillos. Solo encuentra seis euros y una pequeña pieza metálica de su mundo.".into(),
            choices: vec![
                choice("Decir toda la verdad: «Vengo de otro mundo y no sé qué es un billete»"),
                choice("Preguntar cuánto cuesta sin explicar nada"),
            ],
            look: SamLook { pose: 2, outfit: "hoodie", expression: "blushed1" },
        },
        TrainArrival => SceneView {
            location: Train,
            speaker: "NARRADOR",
            text: "El revisor no le cree, pero la deja bajar en la siguiente estación. Sam llega a una ciudad desconocida con 6 €, hambre creciente y una certeza: antes de que llegue la noche necesitará comida, trabajo y un lugar donde dormir.".into(),
            choices: vec![choice("Abrir el mapa de la ciudad")],
            look: SamLook { pose: 3, outfit: "hoodie", expression: "happy" },
        },
        PlazaFirst => SceneView {
            location: Plaza,
            speaker: "LINA",
            text: "Una joven que reparte folletos se acerca.\n\n—Pareces perdida. ¿Estás esperando a alguien?\n\nSam entiende la pregunta literalmente: no espera a nadie. Ni siquiera sabe si alguien la está buscando.".into(),
            choices: vec![
                choice("«Estoy sola. No tengo casa ni sé cómo funciona este mundo»"),
                choice("«Estoy explorando...» y corregirse antes de convertirlo en mentira"),
            ],
            look: SamLook { pose: 1, outfit: "shirt", expression: "surprised" },
        },
        PlazaTruth => SceneView {
            location: Plaza,
            speaker: "LINA",
            text: "Lina tarda unos segundos en decidir si Sam bromea. Su forma de hablar es demasiado directa para parecer una estafa. Le entrega un mapa, 4 € y la dirección de una pensión barata.\n\n—No cuentes eso a cualquiera. Aquí algunas personas ayudan... y otras aprovechan.".into(),
            choices: vec![choice("Agradecer y volver al mapa")],
            look: SamLook { pose: 6, outfit: "shirt", expression: "superhappy" },
        },
        PlazaObserve => SceneView {
            location: Plaza,
            speaker: "SAM",
            text: "Sam intenta responder sin mentir, pero descubre que omitir una verdad también puede cambiar lo que otra persona entiende. Confiesa su situación. Lina sonríe, le entrega un mapa y le señala un café, un taller y una pensión.".into(),
            choices: vec![choice("Volver al mapa")],
            look: SamLook { pose: 2, outfit: "shirt", expression: "blushed2" },
        },
        CafeFirst => SceneView {
            location: Cafe,
            speaker: "NORA",
            text: "El Café Lucero huele a pan caliente. La encargada observa la ropa de Sam y pregunta:\n\n—¿Tienes experiencia trabajando cara al público?\n\nSam no conoce la expresión. Tampoco ha usado nunca una caja registradora.".into(),
            choices: vec![
                choice("Responder con completa honestidad"),
                choice("Decir que aprende rápido y pedir una oportunidad"),
                if state.money >= 7 { choice("Comprar el menú del día — 7 €") } else { locked("Comprar el menú del día — 7 €", "No tienes suficiente dinero") },
                choice("Pedir solo un vaso de agua")
            ],
            look: SamLook { pose: 4, outfit: "casual", expression: "happy" },
        },
        CafeTruth => SceneView {
            location: Cafe,
            speaker: "NORA",
            text: "—No tengo experiencia. Tampoco documentos que usted considere reales. Pero no robaré, no fingiré saber algo y preguntaré antes de cometer un error.\n\nNora se queda inmóvil y después se ríe. Es la peor y la mejor entrevista que ha escuchado.".into(),
            choices: vec![choice("Aceptar un turno de prueba")],
            look: SamLook { pose: 3, outfit: "casual", expression: "blushed1" },
        },
        CafeWork => SceneView {
            location: Cafe,
            speaker: "NARRADOR",
            text: "Sam limpia mesas y aprende a servir pedidos. Cuando un cliente pregunta si el pastel es fresco, ella responde que lleva dos días en la vitrina. Nora pierde una venta... pero decide no despedirla.\n\nGanas 14 € y una posible jornada para mañana.".into(),
            choices: vec![choice("Terminar el turno y volver al mapa")],
            look: SamLook { pose: 5, outfit: "casual", expression: "superhappy" },
        },
        CafeMeal => SceneView {
            location: Cafe,
            speaker: "SAM",
            text: "Es la primera comida de Sam en este mundo. Le sorprende que el menú muestre una fotografía más grande y perfecta que el plato real. Nora explica que eso se llama publicidad. Sam decide que la publicidad es una mentira con permiso.".into(),
            choices: vec![choice("Comer y volver al mapa")],
            look: SamLook { pose: 6, outfit: "casual", expression: "happy" },
        },
        CafeWater => SceneView {
            location: Cafe,
            speaker: "NORA",
            text: "Nora le sirve agua y una rebanada de pan que iba a desechar.\n\n—No te estoy regalando nada —dice—. Me ayudas a llevar esas cajas y quedamos en paz.\n\nSam percibe que no es verdad, pero también que Nora intenta proteger su dignidad.".into(),
            choices: vec![choice("Ayudar con las cajas")],
            look: SamLook { pose: 2, outfit: "casual", expression: "blushed1" },
        },
        GarageFirst => SceneView {
            location: Garage,
            speaker: "IVO",
            text: "En el Taller Norte, un hombre pelea con el motor de una motocicleta.\n\n—¿Sabes reparar algo?\n\nSam reconoce principios parecidos a las máquinas de su mundo, aunque nunca ha visto un motor de combustión.".into(),
            choices: vec![
                choice("Contar la verdad y señalar lo que cree que está fallando"),
                choice("Ofrecerse a barrer y ordenar herramientas")
            ],
            look: SamLook { pose: 7, outfit: "work", expression: "happy" },
        },
        GarageTruth => SceneView {
            location: Garage,
            speaker: "IVO",
            text: "Sam explica que no conoce ese motor, pero que el sonido irregular coincide con una entrada de aire mal sellada. Ivo revisa la manguera y encuentra una grieta.\n\n—No sé de dónde saliste, chica, pero sabes escuchar una máquina.".into(),
            choices: vec![choice("Ayudar durante unas horas")],
            look: SamLook { pose: 8, outfit: "work", expression: "superhappy" },
        },
        GarageSweep => SceneView {
            location: Garage,
            speaker: "NARRADOR",
            text: "Sam ordena llaves, limpia aceite y pregunta para qué sirve cada herramienta. Ivo le paga 10 € y le dice que puede volver. No es un contrato, pero es la primera puerta que se abre sin que Sam tenga que fingir.".into(),
            choices: vec![choice("Volver al mapa")],
            look: SamLook { pose: 5, outfit: "work", expression: "happy" },
        },
        GarageWork => SceneView {
            location: Garage,
            speaker: "NARRADOR",
            text: "El motor vuelve a encender. Ivo paga a Sam 20 € y promete enseñarle el trabajo si regresa al día siguiente. La sinceridad de Sam le resulta extraña, pero mucho menos peligrosa que la seguridad fingida de otros aprendices.".into(),
            choices: vec![choice("Guardar el dinero y volver al mapa")],
            look: SamLook { pose: 8, outfit: "work", expression: "superhappy" },
        },
        BoardingFirst => SceneView {
            location: BoardingHouse,
            speaker: "INÉS",
            text: "La Pensión Azul tiene una habitación libre. Inés pide 18 € por una noche y mira a Sam con sospecha.\n\n—Sin documentos, se paga por adelantado. No quiero problemas.".into(),
            choices: vec![
                if state.money >= 18 { choice("Pagar 18 € por la habitación") } else { locked("Pagar 18 € por la habitación", format!("Te faltan {} €", 18 - state.money)) },
                choice("Ofrecer limpiar la pensión a cambio de una cama"),
                choice("Agradecer e irse")
            ],
            look: SamLook { pose: 1, outfit: "hoodie", expression: "sad" },
        },
        BoardingPay => SceneView {
            location: BoardingHouse,
            speaker: "INÉS",
            text: "Inés entrega una llave pequeña con una etiqueta azul. La habitación es estrecha, pero tiene una cama, una ducha y una puerta que puede cerrarse. Para Sam, eso basta para llamarlo refugio.".into(),
            choices: vec![choice("Guardar la llave y volver al mapa")],
            look: SamLook { pose: 6, outfit: "hoodie", expression: "superhappy" },
        },
        BoardingClean => SceneView {
            location: BoardingHouse,
            speaker: "INÉS",
            text: if state.trust >= 3 || state.job_offer {
                "Sam explica cada detalle de su situación sin adornarlo. Inés no cree la parte del otro mundo, pero Lina o Ivo ya hablaron bien de ella. Acepta que limpie el pasillo y le ofrece una habitación para esa noche.".into()
            } else {
                "Inés escucha la propuesta, pero no está dispuesta a entregar una llave a una desconocida sin referencias. Le recomienda volver cuando alguien del barrio pueda responder por ella.".into()
            },
            choices: vec![choice("Aceptar la respuesta y volver al mapa")],
            look: SamLook { pose: 2, outfit: "hoodie", expression: if state.trust >= 3 || state.job_offer { "happy" } else { "sad" } },
        },
        BoardingLeave => SceneView {
            location: BoardingHouse,
            speaker: "SAM",
            text: "Sam guarda la dirección. Una habitación cuesta más que casi todo el dinero que posee, pero ahora sabe qué debe conseguir antes de la noche.".into(),
            choices: vec![choice("Volver al mapa")],
            look: SamLook { pose: 4, outfit: "hoodie", expression: "blushed1" },
        },
        TrainReturn => SceneView {
            location: Train,
            speaker: "SAM",
            text: "La estación sigue abierta. Los trenes conectan lugares, pero cada viaje exige un billete, un destino y una razón. Sam todavía no tiene ninguno de los tres. Decide permanecer en la ciudad al menos hasta mañana.".into(),
            choices: vec![choice("Volver al mapa")],
            look: SamLook { pose: 3, outfit: "hoodie", expression: "happy" },
        },
        NightDecision => SceneView {
            location: BoardingHouse,
            speaker: "NARRADOR",
            text: if state.has_room {
                "La noche cae. Sam tiene una llave en el bolsillo y, por primera vez desde que llegó, un lugar donde cerrar los ojos sin vigilar sus pertenencias.".into()
            } else {
                "La noche cae. La temperatura desciende y las tiendas comienzan a cerrar. Sam todavía no tiene una habitación. Debe decidir qué hacer con lo que consiguió durante el día.".into()
            },
            choices: if state.has_room {
                vec![choice("Terminar el día en la pensión")]
            } else if state.money >= 18 {
                vec![choice("Pagar 18 € en la Pensión Azul"), choice("Conservar el dinero y dormir en la plaza")]
            } else {
                vec![choice("Buscar un lugar protegido en la plaza")]
            },
            look: SamLook { pose: 1, outfit: "hoodie", expression: if state.has_room { "happy" } else { "sad" } },
        },
        NightStreet => SceneView {
            location: Plaza,
            speaker: "SAM",
            text: "Sam se acomoda bajo el techo de una parada. No entiende por qué existen edificios vacíos mientras algunas personas duermen afuera. Guarda el mapa cerca del pecho y repasa los nombres de quienes no la trataron como una amenaza.".into(),
            choices: vec![choice("Cerrar los ojos y terminar la demo")],
            look: SamLook { pose: 4, outfit: "hoodie", expression: "sad" },
        },
        Ending(EndingId::SafeRoom) => SceneView {
            location: BoardingHouse,
            speaker: "SAM",
            text: "FINAL: UNA PUERTA CERRADA\n\nSam consiguió una habitación para pasar la noche. Aún no comprende las reglas de este mundo, pero ya sabe que la verdad puede asustar, incomodar y también abrir una puerta.".into(),
            choices: vec![choice("Volver al menú principal")],
            look: SamLook { pose: 6, outfit: "hoodie", expression: "superhappy" },
        },
        Ending(EndingId::HonestWork) => SceneView {
            location: BoardingHouse,
            speaker: "SAM",
            text: "FINAL: MAÑANA HAY TRABAJO\n\nSam duerme bajo techo y tiene una oportunidad para mañana. No tuvo que inventar experiencia ni ocultar quién era. Quizá este mundo no necesite que aprenda a mentir, sino que aprenda cuándo una verdad necesita cuidado.".into(),
            choices: vec![choice("Volver al menú principal")],
            look: SamLook { pose: 8, outfit: "casual", expression: "superhappy" },
        },
        Ending(EndingId::StreetPromise) => SceneView {
            location: Plaza,
            speaker: "SAM",
            text: "FINAL: LA PRIMERA NOCHE\n\nSam no consiguió una habitación, pero sobrevivió a su primer día. Tiene un mapa, algunos nombres y una promesa para sí misma: mañana volverá a intentarlo sin convertirse en alguien que no reconoce.".into(),
            choices: vec![choice("Volver al menú principal")],
            look: SamLook { pose: 3, outfit: "hoodie", expression: "blushed2" },
        },
    }
}

pub fn first_scene_for_location(location: LocationId, state: &GameState) -> SceneId {
    match location {
        LocationId::Train => SceneId::TrainReturn,
        LocationId::Plaza => {
            if state.plaza_met_lina {
                SceneId::PlazaObserve
            } else {
                SceneId::PlazaFirst
            }
        }
        LocationId::Cafe => SceneId::CafeFirst,
        LocationId::Garage => SceneId::GarageFirst,
        LocationId::BoardingHouse => SceneId::BoardingFirst,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneOutcome {
    Scene(SceneId),
    Map,
    Menu,
}

pub fn apply_choice(state: &mut GameState, scene: SceneId, choice_index: usize) -> SceneOutcome {
    use SceneId::*;

    let outcome = match scene {
        TrainAwakening => SceneOutcome::Scene(TrainTicket),
        TrainTicket => {
            if choice_index == 0 {
                state.trust += 1;
                state.truth_moments += 1;
            }
            SceneOutcome::Scene(TrainArrival)
        }
        TrainArrival => SceneOutcome::Map,
        PlazaFirst => {
            state.plaza_met_lina = true;
            state.visit(LocationId::Plaza);
            if choice_index == 0 {
                state.money += 4;
                state.trust += 2;
                state.truth_moments += 1;
                SceneOutcome::Scene(PlazaTruth)
            } else {
                state.trust += 1;
                state.truth_moments += 1;
                SceneOutcome::Scene(PlazaObserve)
            }
        }
        PlazaTruth | PlazaObserve => {
            state.advance_time();
            SceneOutcome::Map
        }
        CafeFirst => {
            state.visit(LocationId::Cafe);
            match choice_index {
                0 => SceneOutcome::Scene(CafeTruth),
                1 => SceneOutcome::Scene(CafeWork),
                2 if state.money >= 7 => {
                    state.money -= 7;
                    state.hunger -= 42;
                    state.trust += 1;
                    SceneOutcome::Scene(CafeMeal)
                }
                _ => {
                    state.hunger -= 18;
                    state.energy += 5;
                    state.trust += 1;
                    SceneOutcome::Scene(CafeWater)
                }
            }
        }
        CafeTruth => {
            state.truth_moments += 1;
            state.trust += 2;
            SceneOutcome::Scene(CafeWork)
        }
        CafeWork => {
            state.money += 14;
            state.energy -= 22;
            state.hunger += 10;
            state.job_offer = true;
            state.cafe_worked = true;
            state.advance_time();
            SceneOutcome::Map
        }
        CafeMeal => {
            state.advance_time();
            SceneOutcome::Map
        }
        CafeWater => {
            state.energy -= 8;
            state.advance_time();
            SceneOutcome::Map
        }
        GarageFirst => {
            state.visit(LocationId::Garage);
            if choice_index == 0 {
                state.truth_moments += 1;
                state.trust += 2;
                SceneOutcome::Scene(GarageTruth)
            } else {
                SceneOutcome::Scene(GarageSweep)
            }
        }
        GarageTruth => SceneOutcome::Scene(GarageWork),
        GarageSweep => {
            state.money += 10;
            state.energy -= 15;
            state.trust += 1;
            state.advance_time();
            SceneOutcome::Map
        }
        GarageWork => {
            state.money += 20;
            state.energy -= 28;
            state.hunger += 12;
            state.trust += 2;
            state.job_offer = true;
            state.garage_worked = true;
            state.advance_time();
            SceneOutcome::Map
        }
        BoardingFirst => {
            state.visit(LocationId::BoardingHouse);
            state.boarding_met_ines = true;
            match choice_index {
                0 if state.money >= 18 => {
                    state.money -= 18;
                    state.has_room = true;
                    SceneOutcome::Scene(BoardingPay)
                }
                1 => {
                    if state.trust >= 3 || state.job_offer {
                        state.has_room = true;
                        state.energy -= 12;
                        state.trust += 1;
                    }
                    SceneOutcome::Scene(BoardingClean)
                }
                _ => SceneOutcome::Scene(BoardingLeave),
            }
        }
        BoardingPay | BoardingClean | BoardingLeave => {
            state.advance_time();
            SceneOutcome::Map
        }
        TrainReturn => SceneOutcome::Map,
        NightDecision => {
            if state.has_room {
                let ending = state.choose_ending();
                state.ending = Some(ending);
                state.completed = true;
                SceneOutcome::Scene(Ending(ending))
            } else if state.money >= 18 && choice_index == 0 {
                state.money -= 18;
                state.has_room = true;
                let ending = state.choose_ending();
                state.ending = Some(ending);
                state.completed = true;
                SceneOutcome::Scene(Ending(ending))
            } else {
                SceneOutcome::Scene(NightStreet)
            }
        }
        NightStreet => {
            state.completed = true;
            state.ending = Some(EndingId::StreetPromise);
            SceneOutcome::Scene(Ending(EndingId::StreetPromise))
        }
        Ending(_) => SceneOutcome::Menu,
    };

    state.current_scene = match outcome {
        SceneOutcome::Scene(next) => next,
        _ => state.current_scene,
    };
    state.clamp_stats();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honest_work_route_reaches_best_demo_ending() {
        let mut state = GameState::default();
        let route = [
            (SceneId::TrainAwakening, 0),
            (SceneId::TrainTicket, 0),
            (SceneId::TrainArrival, 0),
            (SceneId::PlazaFirst, 0),
            (SceneId::PlazaTruth, 0),
            (SceneId::GarageFirst, 0),
            (SceneId::GarageTruth, 0),
            (SceneId::GarageWork, 0),
            (SceneId::BoardingFirst, 0),
            (SceneId::BoardingPay, 0),
            (SceneId::CafeFirst, 1),
            (SceneId::CafeWork, 0),
            (SceneId::NightDecision, 0),
        ];
        for (scene, choice) in route {
            let outcome = apply_choice(&mut state, scene, choice);
            if let SceneOutcome::Scene(next) = outcome {
                state.current_scene = next;
            }
        }
        assert!(state.completed);
        assert!(state.has_room);
        assert!(state.job_offer);
        assert_eq!(state.ending, Some(EndingId::HonestWork));
    }

    #[test]
    fn sleeping_outside_produces_street_ending() {
        let mut state = GameState {
            time: TimeSlot::Night,
            money: 2,
            ..GameState::default()
        };
        let outcome = apply_choice(&mut state, SceneId::NightDecision, 0);
        assert_eq!(outcome, SceneOutcome::Scene(SceneId::NightStreet));
        let outcome = apply_choice(&mut state, SceneId::NightStreet, 0);
        assert_eq!(
            outcome,
            SceneOutcome::Scene(SceneId::Ending(EndingId::StreetPromise))
        );
        assert!(state.completed);
    }

    #[test]
    fn paid_room_choice_is_locked_without_money() {
        let state = GameState {
            money: 6,
            ..GameState::default()
        };
        let view = scene_view(SceneId::BoardingFirst, &state);
        assert!(!view.choices[0].enabled);
        assert!(view.choices[0].hint.is_some());
    }
}
