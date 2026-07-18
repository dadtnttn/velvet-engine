//! Semantic validation for Velvet Story (writer-facing).

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::commands::CommandRegistry;
use crate::diag::StoryDiag;
use crate::span::Span;

/// Semantic analysis result.
#[derive(Debug, Default)]
pub struct SemaResult {
    /// Diagnostics.
    pub diags: Vec<StoryDiag>,
    /// Scene names.
    pub scenes: HashSet<String>,
    /// Labels (scene::label).
    pub labels: HashSet<String>,
    /// Variables assigned.
    pub variables: HashSet<String>,
}

/// Validate a story file.
pub fn analyze(file: &StoryFile, cmds: &CommandRegistry) -> SemaResult {
    let mut r = SemaResult::default();
    let mut scene_spans: HashMap<String, Span> = HashMap::new();

    for item in &file.items {
        if let TopItem::Scene(sc) = item {
            if let Some(prev) = scene_spans.insert(sc.name.clone(), sc.span) {
                r.diags.push(
                    StoryDiag::error(
                        "VST020",
                        format!("La escena `{}` ya existe.", sc.name),
                        &file.file,
                        sc.span,
                    )
                    .with_suggestion(format!(
                        "Renombra una de las dos escenas. La primera estaba cerca de la línea {}.",
                        prev.line
                    ))
                    .with_node("scene"),
                );
            }
            r.scenes.insert(sc.name.clone());
        }
    }

    for item in &file.items {
        if let TopItem::Scene(sc) = item {
            collect_labels(&mut r, &sc.name, &sc.body);
            walk_stmts(&mut r, file, sc, &sc.body, cmds);
        }
    }

    // second pass: gotos
    for item in &file.items {
        if let TopItem::Scene(sc) = item {
            check_gotos(&mut r, file, &sc.body);
        }
    }

    r
}

fn collect_labels(r: &mut SemaResult, scene: &str, stmts: &[Stmt]) {
    for st in stmts {
        match st {
            Stmt::Label { name, .. } => {
                r.labels.insert(format!("{scene}::{name}"));
                r.labels.insert(name.clone());
            }
            Stmt::Choice { options, .. } => {
                for o in options {
                    collect_labels(r, scene, &o.body);
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_labels(r, scene, then_body);
                if let Some(e) = else_body {
                    collect_labels(r, scene, e);
                }
            }
            _ => {}
        }
    }
}

fn walk_stmts(
    r: &mut SemaResult,
    file: &StoryFile,
    scene: &Scene,
    stmts: &[Stmt],
    cmds: &CommandRegistry,
) {
    for st in stmts {
        match st {
            Stmt::Set { name, .. } => {
                r.variables.insert(name.clone());
            }
            Stmt::Add { name, span, .. } | Stmt::Sub { name, span, .. } => {
                if !r.variables.contains(name) {
                    r.diags.push(
                        StoryDiag::warning(
                            "VST021",
                            format!(
                                "La variable `{name}` se modifica sin un `set` previo; se asumirá 0."
                            ),
                            &file.file,
                            *span,
                        )
                        .with_suggestion(format!("set {name} = 0")),
                    );
                }
                r.variables.insert(name.clone());
            }
            Stmt::CallCommand { name, args, span } => {
                if let Some(spec) = cmds.get(name) {
                    for req in &spec.required {
                        if !args.iter().any(|(k, _)| k == req) {
                            r.diags.push(
                                StoryDiag::error(
                                    "VST022",
                                    format!(
                                        "Al comando `{name}` le falta el parámetro obligatorio `{req}`."
                                    ),
                                    &file.file,
                                    *span,
                                )
                                .with_suggestion(format!("{req}: …"))
                                .with_node("call"),
                            );
                        }
                    }
                    for (k, _) in args {
                        if !spec.params.iter().any(|p| p.name == *k) {
                            r.diags.push(
                                StoryDiag::warning(
                                    "VST023",
                                    format!(
                                        "El parámetro `{k}` no está documentado para `{name}`."
                                    ),
                                    &file.file,
                                    *span,
                                )
                                .with_node("call"),
                            );
                        }
                    }
                } else {
                    r.diags.push(
                        StoryDiag::error(
                            "VST024",
                            format!(
                                "No hay un comando registrado llamado `{name}`. Un programador debe exponerlo desde Velvet Script 2."
                            ),
                            &file.file,
                            *span,
                        )
                        .with_node("call"),
                    );
                }
            }
            Stmt::Dialogue { text, span, .. } => {
                if text.trim().is_empty() {
                    r.diags.push(
                        StoryDiag::warning(
                            "VST025",
                            "Este diálogo no tiene texto.",
                            &file.file,
                            *span,
                        )
                        .with_node("dialogue"),
                    );
                }
            }
            Stmt::Choice { options, span } => {
                if options.is_empty() {
                    r.diags.push(
                        StoryDiag::error(
                            "VST026",
                            "Un `choice` necesita al menos una opción.",
                            &file.file,
                            *span,
                        )
                        .with_node("choice"),
                    );
                }
                for o in options {
                    walk_stmts(r, file, scene, &o.body, cmds);
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                walk_stmts(r, file, scene, then_body, cmds);
                if let Some(e) = else_body {
                    walk_stmts(r, file, scene, e, cmds);
                }
            }
            _ => {}
        }
    }
}

fn check_gotos(r: &mut SemaResult, file: &StoryFile, stmts: &[Stmt]) {
    for st in stmts {
        match st {
            Stmt::Goto { target, span } | Stmt::CallScene { target, span } => {
                if !r.scenes.contains(target) && !r.labels.contains(target) {
                    r.diags.push(
                        StoryDiag::error(
                            "VST027",
                            format!(
                                "No existe la escena o etiqueta `{target}`."
                            ),
                            &file.file,
                            *span,
                        )
                        .with_suggestion("Revisa el nombre o crea la escena con `scene …`.")
                        .with_node("goto"),
                    );
                }
            }
            Stmt::Choice { options, .. } => {
                for o in options {
                    check_gotos(r, file, &o.body);
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                check_gotos(r, file, then_body);
                if let Some(e) = else_body {
                    check_gotos(r, file, e);
                }
            }
            _ => {}
        }
    }
}

/// True if no errors.
pub fn ok(r: &SemaResult) -> bool {
    !r.diags.iter().any(|d| d.is_error())
}
