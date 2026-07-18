//! Structured APIs for Velvet Studio (not string-only).

use serde::{Deserialize, Serialize};

use crate::ast::{Stmt, TopItem};
use crate::commands::CommandRegistry;
use crate::diag::StoryDiag;
use crate::parser::parse;
use crate::sema;

/// Outline scene for navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneOutline {
    /// Name.
    pub name: String,
    /// Line.
    pub line: u32,
    /// Goto targets.
    pub jumps: Vec<String>,
    /// Speakers used.
    pub speakers: Vec<String>,
    /// Choice labels.
    pub choices: Vec<String>,
}

/// Variable mention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarInfo {
    /// Name.
    pub name: String,
    /// First line.
    pub line: u32,
}

/// Studio document model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioModel {
    /// File path.
    pub file: String,
    /// Scenes.
    pub scenes: Vec<SceneOutline>,
    /// Characters / speakers.
    pub characters: Vec<String>,
    /// Variables.
    pub variables: Vec<VarInfo>,
    /// Registered commands (for autocomplete).
    pub commands: Vec<String>,
    /// Diagnostics.
    pub diagnostics: Vec<StoryDiag>,
    /// Completions labels.
    pub completions: Vec<CompletionItem>,
}

/// Completion item for Studio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// Label.
    pub label: String,
    /// Kind.
    pub kind: String,
    /// Detail.
    pub detail: String,
    /// Insert text.
    pub insert: String,
}

/// Build model from source.
pub fn build_model(source: &str, file: &str, cmds: &CommandRegistry) -> StudioModel {
    let parsed = parse(source, file);
    let sema = sema::analyze(&parsed.file, cmds);
    let mut diags = parsed.diags;
    diags.extend(sema.diags);

    let mut scenes = Vec::new();
    let mut characters = Vec::new();
    let mut variables = Vec::new();

    for item in &parsed.file.items {
        match item {
            TopItem::Scene(sc) => {
                let mut jumps = Vec::new();
                let mut speakers = Vec::new();
                let mut choices = Vec::new();
                collect_scene(&sc.body, &mut jumps, &mut speakers, &mut choices, &mut variables);
                for s in &speakers {
                    if !characters.contains(s) {
                        characters.push(s.clone());
                    }
                }
                scenes.push(SceneOutline {
                    name: sc.name.clone(),
                    line: sc.span.line,
                    jumps,
                    speakers,
                    choices,
                });
            }
            TopItem::CharacterDecl { name, .. } => {
                if !characters.contains(name) {
                    characters.push(name.clone());
                }
            }
            _ => {}
        }
    }

    let mut completions = vec![
        CompletionItem {
            label: "scene".into(),
            kind: "keyword".into(),
            detail: "Nueva escena".into(),
            insert: "scene ${1:name}\n\n".into(),
        },
        CompletionItem {
            label: "choice".into(),
            kind: "keyword".into(),
            detail: "Elección".into(),
            insert: "choice:\n    \"${1:opción}\":\n        goto ${2:target}\n".into(),
        },
        CompletionItem {
            label: "goto".into(),
            kind: "keyword".into(),
            detail: "Saltar a escena".into(),
            insert: "goto ${1:scene}\n".into(),
        },
    ];
    for (name, desc, snip) in cmds.completions() {
        completions.push(CompletionItem {
            label: name.clone(),
            kind: "command".into(),
            detail: desc,
            insert: snip,
        });
    }
    for sc in &scenes {
        completions.push(CompletionItem {
            label: sc.name.clone(),
            kind: "scene".into(),
            detail: "escena".into(),
            insert: sc.name.clone(),
        });
    }

    StudioModel {
        file: file.into(),
        scenes,
        characters,
        variables,
        commands: cmds.commands.iter().map(|c| c.name.clone()).collect(),
        diagnostics: diags,
        completions,
    }
}

fn collect_scene(
    body: &[Stmt],
    jumps: &mut Vec<String>,
    speakers: &mut Vec<String>,
    choices: &mut Vec<String>,
    variables: &mut Vec<VarInfo>,
) {
    for st in body {
        match st {
            Stmt::Goto { target, .. } | Stmt::CallScene { target, .. } => {
                if !jumps.contains(target) {
                    jumps.push(target.clone());
                }
            }
            Stmt::Dialogue { speaker, span, .. } => {
                if !speakers.contains(speaker) {
                    speakers.push(speaker.clone());
                }
                let _ = span;
            }
            Stmt::Choice { options, .. } => {
                for o in options {
                    choices.push(o.label.clone());
                    collect_scene(&o.body, jumps, speakers, choices, variables);
                }
            }
            Stmt::Set { name, span, .. } | Stmt::Add { name, span, .. } | Stmt::Sub { name, span, .. } => {
                if !variables.iter().any(|v| v.name == *name) {
                    variables.push(VarInfo {
                        name: name.clone(),
                        line: span.line,
                    });
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_scene(then_body, jumps, speakers, choices, variables);
                if let Some(e) = else_body {
                    collect_scene(e, jumps, speakers, choices, variables);
                }
            }
            _ => {}
        }
    }
}

/// JSON for Studio.
pub fn model_json(model: &StudioModel) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(model)
}
