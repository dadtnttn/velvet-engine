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

/// File path used for diagnostics inside a scene (include origin when present).
fn diag_file(file: &StoryFile, scene: &Scene) -> String {
    scene
        .origin_file
        .clone()
        .unwrap_or_else(|| file.file.clone())
}

/// Validate a story file.
pub fn analyze(file: &StoryFile, cmds: &CommandRegistry) -> SemaResult {
    let mut r = SemaResult::default();
    let mut scene_spans: HashMap<String, Span> = HashMap::new();

    for item in &file.items {
        if let TopItem::Scene(sc) = item {
            let origin = diag_file(file, sc);
            if let Some(prev) = scene_spans.insert(sc.name.clone(), sc.span) {
                let line = prev.line.to_string();
                r.diags.push(
                    StoryDiag::error_key(
                        "VST020",
                        &[("name", sc.name.as_str()), ("line", line.as_str())],
                        origin,
                        sc.span,
                    )
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
            check_gotos(&mut r, file, sc, &sc.body);
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
    let origin = diag_file(file, scene);
    for st in stmts {
        match st {
            Stmt::Set { name, .. } => {
                r.variables.insert(name.clone());
            }
            Stmt::Add { name, span, .. } | Stmt::Sub { name, span, .. } => {
                if !r.variables.contains(name) {
                    r.diags.push(StoryDiag::warning_key(
                        "VST021",
                        &[("name", name.as_str())],
                        &origin,
                        *span,
                    ));
                }
                r.variables.insert(name.clone());
            }
            Stmt::CallCommand { name, args, span } => {
                if let Some(spec) = cmds.get(name) {
                    for req in &spec.required {
                        if !args.iter().any(|(k, _)| k == req) {
                            r.diags.push(
                                StoryDiag::error_key(
                                    "VST022",
                                    &[("name", name.as_str()), ("req", req.as_str())],
                                    &origin,
                                    *span,
                                )
                                .with_node("call"),
                            );
                        }
                    }
                    for (k, _) in args {
                        if !spec.params.iter().any(|p| p.name == *k) {
                            r.diags.push(
                                StoryDiag::warning_key(
                                    "VST023",
                                    &[("name", k.as_str()), ("cmd", name.as_str())],
                                    &origin,
                                    *span,
                                )
                                .with_node("call"),
                            );
                        }
                    }
                } else {
                    r.diags.push(
                        StoryDiag::error_key(
                            "VST024",
                            &[("name", name.as_str())],
                            &origin,
                            *span,
                        )
                        .with_node("call"),
                    );
                }
            }
            Stmt::Dialogue { text, span, .. } => {
                if text.trim().is_empty() {
                    r.diags.push(
                        StoryDiag::warning_key("VST025", &[], &origin, *span)
                            .with_node("dialogue"),
                    );
                }
            }
            Stmt::Choice { options, span } => {
                if options.is_empty() {
                    r.diags.push(
                        StoryDiag::error_key("VST026", &[], &origin, *span).with_node("choice"),
                    );
                }
                for o in options {
                    walk_stmts(r, file, scene, &o.body, cmds);
                }
            }
            Stmt::If {
                cond,
                then_body,
                else_body,
                span,
            } => {
                check_if_cond(r, file, scene, cond, *span);
                walk_stmts(r, file, scene, then_body, cmds);
                if let Some(e) = else_body {
                    walk_stmts(r, file, scene, e, cmds);
                }
            }
            _ => {}
        }
    }
}

/// Writer-facing: `if` needs a condition that can be true/false.
fn check_if_cond(r: &mut SemaResult, file: &StoryFile, scene: &Scene, cond: &Expr, span: Span) {
    if cond_is_booleanish(cond) {
        return;
    }
    let hint = match cond {
        Expr::Str(s, _) => crate::locale::if_cond_hint_str(s),
        Expr::Int(n, _) => crate::locale::if_cond_hint_int(*n),
        Expr::Float(s, _) => crate::locale::if_cond_hint_float(s),
        _ => crate::locale::if_cond_hint_other(),
    };
    r.diags.push(
        StoryDiag::error_key(
            "VST030",
            &[("hint", hint.as_str())],
            diag_file(file, scene),
            span,
        )
        .with_node("if"),
    );
}

fn cond_is_booleanish(e: &Expr) -> bool {
    match e {
        // bare identifiers are allowed as truthy flags/vars
        Expr::Ident(_, _) => true,
        Expr::Bool(_, _) => true,
        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => cond_is_booleanish(expr),
        Expr::Unary {
            op: UnaryOp::Neg,
            ..
        } => false,
        Expr::Binary { op, left, right, .. } => match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => true,
            BinOp::And | BinOp::Or => cond_is_booleanish(left) && cond_is_booleanish(right),
            // arithmetic alone is not a valid condition for writers
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => false,
        },
        // bare string / number / float alone → invalid
        Expr::Str(_, _) | Expr::Int(_, _) | Expr::Float(_, _) => false,
    }
}

fn check_gotos(r: &mut SemaResult, file: &StoryFile, scene: &Scene, stmts: &[Stmt]) {
    let origin = diag_file(file, scene);
    for st in stmts {
        match st {
            Stmt::Goto { target, span } | Stmt::CallScene { target, span } => {
                if !r.scenes.contains(target) && !r.labels.contains(target) {
                    r.diags.push(
                        StoryDiag::error_key(
                            "VST027",
                            &[("target", target.as_str())],
                            &origin,
                            *span,
                        )
                        .with_node("goto"),
                    );
                }
            }
            Stmt::Choice { options, .. } => {
                for o in options {
                    check_gotos(r, file, scene, &o.body);
                }
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                check_gotos(r, file, scene, then_body);
                if let Some(e) = else_body {
                    check_gotos(r, file, scene, e);
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
