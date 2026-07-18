//! Lower Velvet Script AST into a [`StoryProgram`].

use thiserror::Error;
use velvet_script_ast::{BinOp, Expr, Item, Module, Stmt};
use velvet_script_parser::{parse_file, ParseError};

use crate::character::Character;
use crate::ir::{StoryChoice, StoryOp, StoryProgram, StoryScene};
use crate::value::{from_ast_expr, StoryValue};
use crate::variables::AssignOp;

/// Load errors.
#[derive(Debug, Error)]
pub enum LoadError {
    /// Parse failed.
    #[error("parse: {0}")]
    Parse(#[from] ParseError),
    /// Semantic issue.
    #[error("{0}")]
    Semantic(String),
}

/// Load a story program from source text.
pub fn load_program_from_source(
    source: &str,
    file: Option<&str>,
    title: impl Into<String>,
) -> Result<StoryProgram, LoadError> {
    let parsed = parse_file(source, file)?;
    if parsed.module.has_errors() {
        let msgs: Vec<_> = parsed
            .module
            .diagnostics
            .iter()
            .map(|d| d.display())
            .collect();
        return Err(LoadError::Semantic(msgs.join("; ")));
    }
    lower_module(&parsed.module, title.into())
}

/// Lower an already-parsed module.
pub fn lower_module(module: &Module, title: String) -> Result<StoryProgram, LoadError> {
    let mut program = StoryProgram::new(title);
    let mut first_scene = None;

    for item in &module.items {
        match item {
            Item::Character { name, fields, .. } => {
                let mut ch = Character::new(name.clone(), name.clone());
                for (fname, fexpr) in fields {
                    match fname.as_str() {
                        "name" => {
                            if let Some(StoryValue::String(s)) = from_ast_expr(fexpr) {
                                ch.name = s;
                            }
                        }
                        "color" => {
                            if let Some(StoryValue::String(s)) = from_ast_expr(fexpr) {
                                ch.color = s;
                            }
                        }
                        "portrait" => {
                            if let Some(StoryValue::String(s)) = from_ast_expr(fexpr) {
                                ch.portrait = Some(s);
                            }
                        }
                        other => {
                            if let Some(StoryValue::String(s)) = from_ast_expr(fexpr) {
                                ch.expressions.insert(other.to_string(), s);
                            }
                        }
                    }
                }
                program.characters.insert(name.clone(), ch);
            }
            Item::State { bindings, .. } => {
                for b in bindings {
                    let val = from_ast_expr(&b.init).unwrap_or(StoryValue::Null);
                    program.initial_vars.insert(b.name.clone(), val);
                }
            }
            Item::Scene { name, body, .. } => {
                if first_scene.is_none() {
                    first_scene = Some(name.clone());
                }
                let mut scene = StoryScene {
                    name: name.clone(),
                    ops: lower_stmts(body)?,
                    labels: indexmap::IndexMap::new(),
                };
                scene.reindex_labels();
                program.scenes.insert(name.clone(), scene);
            }
            Item::Function { .. } | Item::Stmt(_) => {
                // Non-story items ignored for VN IR (can still run via script VM).
            }
        }
    }

    if program.entry == "main" && !program.scenes.contains_key("main") {
        if let Some(s) = first_scene {
            program.entry = s;
        }
    }
    if program.scenes.is_empty() {
        return Err(LoadError::Semantic("no scenes in story script".into()));
    }
    Ok(program)
}

fn lower_stmts(stmts: &[Stmt]) -> Result<Vec<StoryOp>, LoadError> {
    let mut ops = Vec::new();
    for s in stmts {
        ops.extend(lower_stmt(s)?);
    }
    Ok(ops)
}

fn lower_stmt(stmt: &Stmt) -> Result<Vec<StoryOp>, LoadError> {
    Ok(match stmt {
        Stmt::Background { path, .. } => vec![StoryOp::Background { path: path.clone() }],
        Stmt::Music { path, fade_in, .. } => vec![StoryOp::Music {
            path: path.clone(),
            fade_in: *fade_in,
        }],
        Stmt::Show { target, at, .. } => vec![StoryOp::Show {
            target: target.clone(),
            at: at.clone(),
        }],
        Stmt::Hide { target, .. } => vec![StoryOp::Hide {
            target: target.clone(),
        }],
        Stmt::End { ending, .. } => vec![StoryOp::End {
            ending: ending.clone(),
        }],
        Stmt::Call { target, .. } => vec![StoryOp::Call {
            target: target.clone(),
        }],
        Stmt::Dialogue { speaker, text, .. } => vec![StoryOp::Dialogue {
            speaker: speaker.clone(),
            text: text.clone(),
        }],
        Stmt::Jump { label, .. } => vec![StoryOp::Jump {
            target: label.clone(),
        }],
        Stmt::Label { name, .. } => vec![StoryOp::Label { name: name.clone() }],
        Stmt::Choice { options, .. } => {
            let mut arms = Vec::new();
            for arm in options {
                arms.push(StoryChoice {
                    text: arm.text.clone(),
                    body: lower_stmts(&arm.body)?,
                    require: None,
                    hidden_if_locked: false,
                });
            }
            vec![StoryOp::Choice { options: arms }]
        }
        Stmt::Expr { expr, .. } => lower_expr_stmt(expr)?,
        Stmt::Let { name, init, .. } | Stmt::Const { name, init, .. } => {
            let value = from_ast_expr(init).unwrap_or(StoryValue::Null);
            vec![StoryOp::Assign {
                name: name.clone(),
                assign_op: AssignOp::Set,
                value,
            }]
        }
        Stmt::Block { body, .. } => lower_stmts(body)?,
        Stmt::If {
            cond,
            then_body,
            else_body,
            ..
        } => {
            let cond_var = match cond {
                Expr::Ident { name, .. } => name.clone(),
                _ => {
                    return Err(LoadError::Semantic(
                        "story if supports identifier conditions only in v1".into(),
                    ))
                }
            };
            let then_ops = lower_stmt(then_body)?;
            let else_ops = match else_body {
                Some(e) => lower_stmt(e)?,
                None => vec![],
            };
            vec![StoryOp::If {
                cond_var,
                then_ops,
                else_ops,
            }]
        }
        // Gameplay-only constructs are ignored when lowering narrative IR.
        Stmt::Return { .. }
        | Stmt::While { .. }
        | Stmt::For { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. } => vec![StoryOp::Nop],
    })
}

fn lower_expr_stmt(expr: &Expr) -> Result<Vec<StoryOp>, LoadError> {
    match expr {
        Expr::Binary {
            left, op, right, ..
        } => {
            let name = match left.as_ref() {
                Expr::Ident { name, .. } => name.clone(),
                _ => {
                    return Err(LoadError::Semantic(
                        "assignment target must be identifier".into(),
                    ))
                }
            };
            let assign_op = match op {
                BinOp::Assign => AssignOp::Set,
                BinOp::AddAssign => AssignOp::Add,
                BinOp::SubAssign => AssignOp::Sub,
                _ => return Ok(vec![StoryOp::Nop]),
            };
            let value = from_ast_expr(right).unwrap_or(StoryValue::Null);
            Ok(vec![StoryOp::Assign {
                name,
                assign_op,
                value,
            }])
        }
        _ => Ok(vec![StoryOp::Nop]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_basic_story() {
        let src = r##"
character aria {
    name: "Aria"
    color: "#ff4f8b"
}

state {
    trust: int = 0
}

scene start {
    background "bg.png"
    aria "Hello"
    choice {
        "Hi" {
            trust += 1
            jump end
        }
        "..." {
            jump end
        }
    }
}

scene end {
    "The end"
}
"##;
        let p = load_program_from_source(src, Some("t.vel"), "Test").unwrap();
        assert!(p.characters.contains_key("aria"));
        assert_eq!(p.initial_vars.get("trust"), Some(&StoryValue::Int(0)));
        assert_eq!(p.scenes.len(), 2);
        assert_eq!(p.entry, "start");
    }
}
