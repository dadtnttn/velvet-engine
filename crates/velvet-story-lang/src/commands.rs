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
                doc: "Texto del script (fx, move, wait, spawn, pack_open)".into(),
            }],
            required: vec!["body".into()],
            snippet: "call anim.script:\n    body: \"fx card0 deal 200 360 0.3\"\n".into(),
            error_help: "call anim.script:\n    body: \"fx card0 fade_in\"".into(),
        });
        r.register(CommandSpec {
            name: "anim.pack_open".into(),
            category: "animation".into(),
            description:
                "Genera un efecto 3D de abrir sobre (pack) y abanico de cartas (billboards)."
                    .into(),
            params: vec![
                CommandParam {
                    name: "x".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("480".into()),
                    doc: "Centro X del sobre".into(),
                },
                CommandParam {
                    name: "y".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("270".into()),
                    doc: "Centro Y del sobre".into(),
                },
                CommandParam {
                    name: "cards".into(),
                    ty: ParamTy::Int,
                    required: false,
                    default: Some("5".into()),
                    doc: "Cartas a revelar".into(),
                },
                CommandParam {
                    name: "duration".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("2.2".into()),
                    doc: "Duración total de la secuencia".into(),
                },
                CommandParam {
                    name: "seed".into(),
                    ty: ParamTy::Int,
                    required: false,
                    default: Some("1".into()),
                    doc: "Semilla de variación de tilt".into(),
                },
                CommandParam {
                    name: "fan_spacing".into(),
                    ty: ParamTy::Float,
                    required: false,
                    default: Some("95".into()),
                    doc: "Separación horizontal del abanico".into(),
                },
            ],
            required: vec![],
            snippet:
                "call anim.pack_open:\n    x: 480\n    y: 270\n    cards: 5\n    duration: 2.0\n"
                    .into(),
            error_help: "call anim.pack_open:\n    cards: 5".into(),
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
