//! Extensible command registry for Velvet Story (from VS2 / host).

use serde::{Deserialize, Serialize};

/// Parameter type visible to writers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamTy {
    /// Text.
    Text,
    /// Integer.
    Int,
    /// Float.
    Float,
    /// Bool.
    Bool,
    /// Asset / ident.
    Ident,
}

/// One parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParam {
    /// Name.
    pub name: String,
    /// Type.
    pub ty: ParamTy,
    /// Required.
    pub required: bool,
    /// Default as display string.
    pub default: Option<String>,
    /// Docs.
    pub doc: String,
}

/// Registered command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Public name e.g. `combat.start`.
    pub name: String,
    /// Category for Studio.
    pub category: String,
    /// Description.
    pub description: String,
    /// Parameters.
    pub params: Vec<CommandParam>,
    /// Required param names.
    pub required: Vec<String>,
    /// Autocomplete insert snippet.
    pub snippet: String,
    /// Error help.
    pub error_help: String,
}

/// Registry of writer-facing commands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommandRegistry {
    /// Specs by name.
    pub commands: Vec<CommandSpec>,
}

impl CommandRegistry {
    /// Empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Built-in commands shipped with the engine.
    pub fn builtin() -> Self {
        let mut r = Self::new();
        r.register(CommandSpec {
            name: "combat.start".into(),
            category: "gameplay".into(),
            description: "Inicia un combate (implementado en Velvet Script 2 / host).".into(),
            params: vec![
                CommandParam {
                    name: "enemy".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Identificador del enemigo".into(),
                },
                CommandParam {
                    name: "difficulty".into(),
                    ty: ParamTy::Int,
                    required: false,
                    default: Some("1".into()),
                    doc: "Dificultad".into(),
                },
                CommandParam {
                    name: "can_escape".into(),
                    ty: ParamTy::Bool,
                    required: false,
                    default: Some("true".into()),
                    doc: "Si el jugador puede huir".into(),
                },
            ],
            required: vec!["enemy".into()],
            snippet: "call combat.start:\n    enemy: forest_guardian\n    difficulty: 3\n    can_escape: true\n".into(),
            error_help: "Ejemplo:\ncall combat.start:\n    enemy: forest_guardian".into(),
        });
        r.register(CommandSpec {
            name: "notify".into(),
            category: "ui".into(),
            description: "Muestra un aviso breve al jugador.".into(),
            params: vec![CommandParam {
                name: "text".into(),
                ty: ParamTy::Text,
                required: true,
                default: None,
                doc: "Mensaje".into(),
            }],
            required: vec!["text".into()],
            snippet: "call notify:\n    text: \"Hola\"\n".into(),
            error_help: "call notify:\n    text: \"…\"".into(),
        });
        r.register(CommandSpec {
            name: "flag.set".into(),
            category: "state".into(),
            description: "Marca un flag narrativo persistente.".into(),
            params: vec![
                CommandParam {
                    name: "name".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Nombre del flag".into(),
                },
                CommandParam {
                    name: "value".into(),
                    ty: ParamTy::Bool,
                    required: false,
                    default: Some("true".into()),
                    doc: "Valor".into(),
                },
            ],
            required: vec!["name".into()],
            snippet: "call flag.set:\n    name: met_luna\n    value: true\n".into(),
            error_help: "call flag.set:\n    name: met_luna".into(),
        });
        // Animation / VFX (implemented by velvet-anim::AnimStoryHost)
        r.register(CommandSpec {
            name: "anim.fx".into(),
            category: "animation".into(),
            description: "Reproduce un efecto (deal, fade_in, shake, punch, …) en un target."
                .into(),
            params: vec![
                CommandParam {
                    name: "target".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Id del objeto (card0, hero, ui.banner…)".into(),
                },
                CommandParam {
                    name: "effect".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "deal | fade_in | fade_out | move | punch | shake | bounce | pop".into(),
                },
                CommandParam {
                    name: "x".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0".into()),
                    doc: "Destino X / origen según efecto".into(),
                },
                CommandParam {
                    name: "y".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0".into()),
                    doc: "Destino Y".into(),
                },
                CommandParam {
                    name: "duration".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0.35".into()),
                    doc: "Duración en segundos".into(),
                },
                CommandParam {
                    name: "delay".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0".into()),
                    doc: "Retraso antes de empezar".into(),
                },
                CommandParam {
                    name: "strength".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("8".into()),
                    doc: "Fuerza (shake px / punch scale)".into(),
                },
                CommandParam {
                    name: "ease".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: Some("cubic_out".into()),
                    doc: "Curva: linear, cubic_out, back_out, bounce, …".into(),
                },
            ],
            required: vec!["target".into(), "effect".into()],
            snippet: "call anim.fx:\n    target: card0\n    effect: deal\n    x: 200\n    y: 360\n    duration: 0.35\n".into(),
            error_help: "call anim.fx:\n    target: card0\n    effect: deal".into(),
        });
        r.register(CommandSpec {
            name: "anim.move".into(),
            category: "animation".into(),
            description: "Mueve un target a (x,y) con tween.".into(),
            params: vec![
                CommandParam {
                    name: "target".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Id del objeto".into(),
                },
                CommandParam {
                    name: "x".into(),
                    ty: ParamTy::Float,
                    required: true,
                    default: None,
                    doc: "X destino".into(),
                },
                CommandParam {
                    name: "y".into(),
                    ty: ParamTy::Float,
                    required: true,
                    default: None,
                    doc: "Y destino".into(),
                },
                CommandParam {
                    name: "duration".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0.3".into()),
                    doc: "Segundos".into(),
                },
                CommandParam {
                    name: "ease".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: Some("cubic_out".into()),
                    doc: "Easing".into(),
                },
            ],
            required: vec!["target".into(), "x".into(), "y".into()],
            snippet: "call anim.move:\n    target: card0\n    x: 100\n    y: 200\n    duration: 0.25\n".into(),
            error_help: "call anim.move:\n    target: card0\n    x: 0\n    y: 0".into(),
        });
        r.register(CommandSpec {
            name: "anim.stop".into(),
            category: "animation".into(),
            description: "Detiene tweens del target (congela la pose).".into(),
            params: vec![CommandParam {
                name: "target".into(),
                ty: ParamTy::Ident,
                required: true,
                default: None,
                doc: "Id del objeto".into(),
            }],
            required: vec!["target".into()],
            snippet: "call anim.stop:\n    target: card0\n".into(),
            error_help: "call anim.stop:\n    target: card0".into(),
        });
        r.register(CommandSpec {
            name: "anim.script".into(),
            category: "animation".into(),
            description: "Ejecuta un mini-script .vanim en el argumento body/code.".into(),
            params: vec![CommandParam {
                name: "body".into(),
                ty: ParamTy::Text,
                required: true,
                default: None,
                doc: "Texto del script (fx, move, wait, spawn)".into(),
            }],
            required: vec!["body".into()],
            snippet: "call anim.script:\n    body: \"fx card0 deal 200 360 0.3\"\n".into(),
            error_help: "call anim.script:\n    body: \"fx card0 fade_in\"".into(),
        });
        // Generic 3D image tools (compose your own motions — no premade pack cutscene)
        r.register(CommandSpec {
            name: "anim.billboard".into(),
            category: "animation".into(),
            description: "Crea/actualiza un billboard 3D (imagen) con tamaño y contenido."
                .into(),
            params: vec![
                CommandParam {
                    name: "target".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Id del billboard".into(),
                },
                CommandParam {
                    name: "x".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Posición X".into(),
                },
                CommandParam {
                    name: "y".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Posición Y".into(),
                },
                CommandParam {
                    name: "half_w".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("70".into()),
                    doc: "Semi-ancho local".into(),
                },
                CommandParam {
                    name: "half_h".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("100".into()),
                    doc: "Semi-alto local".into(),
                },
                CommandParam {
                    name: "front".into(),
                    ty: ParamTy::Text,
                    required: false,
                    default: None,
                    doc: "Clave de imagen frontal".into(),
                },
                CommandParam {
                    name: "back".into(),
                    ty: ParamTy::Text,
                    required: false,
                    default: None,
                    doc: "Clave de imagen trasera".into(),
                },
            ],
            required: vec!["target".into()],
            snippet:
                "call anim.billboard:\n    target: card0\n    x: 200\n    y: 300\n    front: \"art/card\"\n"
                    .into(),
            error_help: "call anim.billboard:\n    target: card0".into(),
        });
        r.register(CommandSpec {
            name: "anim.pose3d".into(),
            category: "animation".into(),
            description: "Ajusta canales 3D del billboard (yaw, pitch, roll, foil, …).".into(),
            params: vec![
                CommandParam {
                    name: "target".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Id".into(),
                },
                CommandParam {
                    name: "yaw".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Rotación Y (rad)".into(),
                },
                CommandParam {
                    name: "pitch".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Rotación X".into(),
                },
                CommandParam {
                    name: "roll".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Rotación Z".into(),
                },
                CommandParam {
                    name: "opacity".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Opacidad".into(),
                },
                CommandParam {
                    name: "foil".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: None,
                    doc: "Fase holofoil 0..1".into(),
                },
            ],
            required: vec!["target".into()],
            snippet: "call anim.pose3d:\n    target: card0\n    yaw: 1.2\n    foil: 0.5\n".into(),
            error_help: "call anim.pose3d:\n    target: card0\n    yaw: 0".into(),
        });
        r.register(CommandSpec {
            name: "anim.track".into(),
            category: "animation".into(),
            description: "Keyframe tool: anima un canal (yaw/x/opacity…) con from/to o keys."
                .into(),
            params: vec![
                CommandParam {
                    name: "target".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "Id del billboard".into(),
                },
                CommandParam {
                    name: "channel".into(),
                    ty: ParamTy::Ident,
                    required: true,
                    default: None,
                    doc: "yaw | pitch | roll | x | y | scale | opacity | foil | depth".into(),
                },
                CommandParam {
                    name: "from".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0".into()),
                    doc: "Valor inicial".into(),
                },
                CommandParam {
                    name: "to".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0".into()),
                    doc: "Valor final".into(),
                },
                CommandParam {
                    name: "duration".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("0.4".into()),
                    doc: "Duración".into(),
                },
                CommandParam {
                    name: "keys".into(),
                    ty: ParamTy::Text,
                    required: false,
                    default: None,
                    doc: "Pares t v: \"0 0 0.4 3.14\"".into(),
                },
                CommandParam {
                    name: "ease".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: Some("cubic_out".into()),
                    doc: "Easing".into(),
                },
            ],
            required: vec!["target".into(), "channel".into()],
            snippet:
                "call anim.track:\n    target: card0\n    channel: yaw\n    from: 0\n    to: 3.14\n    duration: 0.4\n"
                    .into(),
            error_help: "call anim.track:\n    target: card0\n    channel: yaw\n    from: 0\n    to: 3.14".into(),
        });
        // CSS-like style tools (.vcss)
        r.register(CommandSpec {
            name: "style.load".into(),
            category: "style".into(),
            description: "Carga una hoja .vcss (CSS-like) en el registro de estilos.".into(),
            params: vec![
                CommandParam {
                    name: "name".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: Some("default".into()),
                    doc: "Nombre de la hoja".into(),
                },
                CommandParam {
                    name: "path".into(),
                    ty: ParamTy::Text,
                    required: false,
                    default: None,
                    doc: "Ruta al archivo .vcss".into(),
                },
                CommandParam {
                    name: "body".into(),
                    ty: ParamTy::Text,
                    required: false,
                    default: None,
                    doc: "Fuente inline".into(),
                },
            ],
            required: vec![],
            snippet: "call style.load:\n    name: casino\n    path: styles/casino.vcss\n".into(),
            error_help: "call style.load:\n    name: casino\n    path: styles/casino.vcss".into(),
        });
        r.register(CommandSpec {
            name: "style.use".into(),
            category: "style".into(),
            description: "Activa una hoja de estilos ya cargada.".into(),
            params: vec![CommandParam {
                name: "name".into(),
                ty: ParamTy::Ident,
                required: true,
                default: None,
                doc: "Nombre".into(),
            }],
            required: vec!["name".into()],
            snippet: "call style.use:\n    name: casino\n".into(),
            error_help: "call style.use:\n    name: casino".into(),
        });
        r.register(CommandSpec {
            name: "style.resolve".into(),
            category: "style".into(),
            description: "Resuelve class/state y deja style.bg / style.color en variables.".into(),
            params: vec![
                CommandParam {
                    name: "class".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: Some("button".into()),
                    doc: "Clase CSS".into(),
                },
                CommandParam {
                    name: "state".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: None,
                    doc: "Pseudo estado (selected, …)".into(),
                },
                CommandParam {
                    name: "id".into(),
                    ty: ParamTy::Ident,
                    required: false,
                    default: None,
                    doc: "Id elemento".into(),
                },
            ],
            required: vec![],
            snippet: "call style.resolve:\n    class: button\n    state: selected\n".into(),
            error_help: "call style.resolve:\n    class: button".into(),
        });
        r
    }

    /// Register (replace if same name).
    ///
    /// Synchronizes [`CommandSpec::required`] with each
    /// [`CommandParam::required`] so writers cannot leave the two lists out of
    /// sync.
    pub fn register(&mut self, mut spec: CommandSpec) {
        for p in &spec.params {
            if p.required && !spec.required.iter().any(|n| n == &p.name) {
                spec.required.push(p.name.clone());
            }
        }
        if let Some(i) = self.commands.iter().position(|c| c.name == spec.name) {
            self.commands[i] = spec;
        } else {
            self.commands.push(spec);
        }
    }

    /// Lookup.
    pub fn get(&self, name: &str) -> Option<&CommandSpec> {
        self.commands.iter().find(|c| c.name == name)
    }

    /// Required parameter names (single source of truth after register).
    pub fn required_params(&self, name: &str) -> Vec<String> {
        self.get(name)
            .map(|s| {
                let mut names: Vec<String> = s
                    .params
                    .iter()
                    .filter(|p| p.required)
                    .map(|p| p.name.clone())
                    .collect();
                for r in &s.required {
                    if !names.iter().any(|n| n == r) {
                        names.push(r.clone());
                    }
                }
                names
            })
            .unwrap_or_default()
    }

    /// Fill optional defaults into a kwargs map (does not override present keys).
    pub fn apply_defaults(
        &self,
        name: &str,
        args: &mut indexmap::IndexMap<String, velvet_story::StoryValue>,
    ) {
        let Some(spec) = self.get(name) else {
            return;
        };
        for p in &spec.params {
            if args.contains_key(&p.name) {
                continue;
            }
            let Some(def) = p.default.as_ref() else {
                continue;
            };
            let val = match p.ty {
                ParamTy::Int => def.parse::<i64>().ok().map(velvet_story::StoryValue::Int),
                ParamTy::Float => def.parse::<f64>().ok().map(velvet_story::StoryValue::Float),
                ParamTy::Bool => match def.as_str() {
                    "true" | "True" | "1" => Some(velvet_story::StoryValue::Bool(true)),
                    "false" | "False" | "0" => Some(velvet_story::StoryValue::Bool(false)),
                    _ => None,
                },
                ParamTy::Text | ParamTy::Ident => {
                    Some(velvet_story::StoryValue::String(def.clone()))
                }
            };
            if let Some(v) = val {
                args.insert(p.name.clone(), v);
            }
        }
    }

    /// Completions for Studio.
    pub fn completions(&self) -> Vec<(String, String, String)> {
        self.commands
            .iter()
            .map(|c| (c.name.clone(), c.description.clone(), c.snippet.clone()))
            .collect()
    }

    /// JSON for Studio.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Conceptual attribute registration (programmers document commands here).
///
/// Real proc-macros can call [`CommandRegistry::register`] at host init.
pub fn register_story_command(reg: &mut CommandRegistry, spec: CommandSpec) {
    reg.register(spec);
}
