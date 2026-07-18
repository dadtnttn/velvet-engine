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
        r
    }

    /// Register (replace if same name).
    pub fn register(&mut self, spec: CommandSpec) {
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
