//! Lower Velvet Story AST → product [`StoryProgram`] (velvet-story IR).
//!
//! Preferred product path for Velvet 2.5 writers (not a second VM).

use indexmap::IndexMap;
use velvet_story::{
    AssignOp, Character, StoryChoice, StoryCmpOp, StoryCond, StoryOp, StoryOperand, StoryProgram,
    StoryScene, StoryValue,
};

use crate::ast::{BinOp, Expr, Stmt, StoryFile, TopItem, UnaryOp};

/// Errors lowering to StoryProgram.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToProgramError {
    /// Human message.
    pub message: String,
    /// Source line (1-based when known).
    pub line: u32,
}

impl std::fmt::Display for ToProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line.max(1), self.message)
    }
}

/// Lower a resolved story file to product IR.
pub fn to_story_program(file: &StoryFile, title: impl Into<String>) -> Result<StoryProgram, ToProgramError> {
    let mut program = StoryProgram::new(title);
    let mut first_scene = None;

    for item in &file.items {
        match item {
            TopItem::Scene(sc) => {
                if first_scene.is_none() {
                    first_scene = Some(sc.name.clone());
                }
                let mut scene = StoryScene {
                    name: sc.name.clone(),
                    ops: lower_stmts(&sc.body)?,
                    labels: IndexMap::new(),
                };
                scene.reindex_labels();
                program.scenes.insert(sc.name.clone(), scene);
            }
            TopItem::CharacterDecl { name, display, .. } => {
                let mut ch = Character::new(name.clone(), name.clone());
                if let Some(d) = display {
                    ch.name = d.clone();
                }
                program.characters.insert(name.clone(), ch);
            }
            TopItem::Include { .. } => {
                // includes must be expanded before this lower
            }
        }
    }

    if !program.scenes.contains_key(&program.entry) {
        if let Some(s) = first_scene {
            program.entry = s;
        } else if let Some(s) = program.scenes.keys().next() {
            program.entry = s.clone();
        }
    }
    if program.scenes.is_empty() {
        return Err(ToProgramError {
            message: "no scenes in story".into(),
            line: 1,
        });
    }
    // Prefer start scene as entry when present
    if program.scenes.contains_key("start") {
        program.entry = "start".into();
    }
    Ok(program)
}

fn lower_stmts(body: &[Stmt]) -> Result<Vec<StoryOp>, ToProgramError> {
    let mut ops = Vec::new();
    for st in body {
        ops.extend(lower_stmt(st)?);
    }
    Ok(ops)
}

fn lower_stmt(st: &Stmt) -> Result<Vec<StoryOp>, ToProgramError> {
    Ok(match st {
        Stmt::Background { id, .. } => vec![StoryOp::Background { path: id.clone() }],
        Stmt::Music { id, .. } => vec![StoryOp::Music {
            path: id.clone(),
            fade_in: None,
        }],
        Stmt::Show {
            character,
            expression,
            at,
            ..
        } => {
            let target = match expression {
                Some(e) => format!("{character}.{e}"),
                None => character.clone(),
            };
            vec![StoryOp::Show {
                target,
                at: at.clone(),
            }]
        }
        Stmt::Hide { character, .. } => vec![StoryOp::Hide {
            target: character.clone(),
        }],
        Stmt::Dialogue {
            speaker, text, ..
        } => {
            let speaker = if speaker == "narrator" {
                None
            } else {
                Some(speaker.clone())
            };
            vec![StoryOp::Dialogue {
                speaker,
                text: text.clone(),
            }]
        }
        Stmt::Choice { options, .. } => {
            let mut arms = Vec::new();
            for o in options {
                arms.push(StoryChoice {
                    text: o.label.clone(),
                    body: lower_stmts(&o.body)?,
                    require: None,
                    hidden_if_locked: false,
                });
            }
            vec![StoryOp::Choice { options: arms }]
        }
        Stmt::Goto { target, .. } => vec![StoryOp::Jump {
            target: target.clone(),
        }],
        Stmt::CallScene { target, .. } => vec![StoryOp::Call {
            target: target.clone(),
        }],
        Stmt::Return { .. } => vec![StoryOp::Return],
        Stmt::End { .. } => vec![StoryOp::End { ending: None }],
        Stmt::Label { name, .. } => vec![StoryOp::Label { name: name.clone() }],
        Stmt::Set { name, value, span } => {
            let v = expr_to_value(value).ok_or_else(|| ToProgramError {
                message: format!("set `{name}` needs a literal or simple value"),
                line: span.line,
            })?;
            vec![StoryOp::Assign {
                name: name.clone(),
                assign_op: AssignOp::Set,
                value: v,
            }]
        }
        Stmt::Add { name, value, span } => {
            let v = expr_to_value(value).ok_or_else(|| ToProgramError {
                message: format!("add `{name}` needs a literal value"),
                line: span.line,
            })?;
            vec![StoryOp::Assign {
                name: name.clone(),
                assign_op: AssignOp::Add,
                value: v,
            }]
        }
        Stmt::Sub { name, value, span } => {
            let v = expr_to_value(value).ok_or_else(|| ToProgramError {
                message: format!("sub `{name}` needs a literal value"),
                line: span.line,
            })?;
            vec![StoryOp::Assign {
                name: name.clone(),
                assign_op: AssignOp::Sub,
                value: v,
            }]
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
            span,
        } => {
            let story_cond = expr_to_cond(cond, span.line)?;
            vec![StoryOp::If {
                cond: story_cond,
                then_ops: lower_stmts(then_body)?,
                else_ops: match else_body {
                    Some(e) => lower_stmts(e)?,
                    None => vec![],
                },
            }]
        }
        Stmt::CallCommand { name, args, span } => {
            let mut map = IndexMap::new();
            for (k, v) in args {
                let sv = expr_to_value(v).ok_or_else(|| ToProgramError {
                    message: format!("command arg `{k}` must be a literal"),
                    line: span.line,
                })?;
                map.insert(k.clone(), sv);
            }
            vec![StoryOp::HostCall {
                name: name.clone(),
                args: map,
            }]
        }
        Stmt::Sound { id, .. } => vec![StoryOp::Sound { path: id.clone() }],
        Stmt::Pause { duration, span } => {
            let seconds = match duration {
                None => None,
                Some(e) => match expr_to_value(e) {
                    Some(StoryValue::Int(n)) => Some(n as f64),
                    Some(StoryValue::Float(f)) => Some(f),
                    _ => {
                        return Err(ToProgramError {
                            message: "pause duration must be a number".into(),
                            line: span.line,
                        });
                    }
                },
            };
            vec![StoryOp::Pause { seconds }]
        }
        Stmt::Transition { name, .. } => vec![StoryOp::Transition { name: name.clone() }],
        // Comments are authoring-only; not runtime ops (but stay in AST for format).
        Stmt::Comment { .. } => vec![],
    })
}

/// Lower a writer condition expression into product [`StoryCond`].
fn expr_to_cond(e: &Expr, line: u32) -> Result<StoryCond, ToProgramError> {
    match e {
        Expr::Ident(name, _) => Ok(StoryCond::Var { name: name.clone() }),
        Expr::Bool(b, _) => Ok(StoryCond::Const { value: *b }),
        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => Ok(StoryCond::Not {
            inner: Box::new(expr_to_cond(expr, line)?),
        }),
        Expr::Binary {
            op: BinOp::And,
            left,
            right,
            ..
        } => Ok(StoryCond::And {
            left: Box::new(expr_to_cond(left, line)?),
            right: Box::new(expr_to_cond(right, line)?),
        }),
        Expr::Binary {
            op: BinOp::Or,
            left,
            right,
            ..
        } => Ok(StoryCond::Or {
            left: Box::new(expr_to_cond(left, line)?),
            right: Box::new(expr_to_cond(right, line)?),
        }),
        Expr::Binary {
            op:
                op @ (BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge),
            left,
            right,
            ..
        } => Ok(StoryCond::Cmp {
            left: expr_to_operand(left, line)?,
            op: binop_to_cmp(*op)?,
            right: expr_to_operand(right, line)?,
        }),
        _ => Err(ToProgramError {
            message: "if condition must be a variable, not/and/or, or a comparison".into(),
            line,
        }),
    }
}

fn binop_to_cmp(op: BinOp) -> Result<StoryCmpOp, ToProgramError> {
    Ok(match op {
        BinOp::Eq => StoryCmpOp::Eq,
        BinOp::Ne => StoryCmpOp::Ne,
        BinOp::Lt => StoryCmpOp::Lt,
        BinOp::Le => StoryCmpOp::Le,
        BinOp::Gt => StoryCmpOp::Gt,
        BinOp::Ge => StoryCmpOp::Ge,
        _ => {
            return Err(ToProgramError {
                message: "internal: not a comparison op".into(),
                line: 0,
            })
        }
    })
}

fn expr_to_operand(e: &Expr, line: u32) -> Result<StoryOperand, ToProgramError> {
    // Identifiers are variables in conditions (not bare string literals).
    match e {
        Expr::Ident(name, _) => Ok(StoryOperand::var(name.clone())),
        _ => match expr_to_value(e) {
            Some(v) => Ok(StoryOperand::value(v)),
            None => Err(ToProgramError {
                message: "comparison operand must be a variable or literal".into(),
                line,
            }),
        },
    }
}

fn expr_to_value(e: &Expr) -> Option<StoryValue> {
    match e {
        Expr::Int(n, _) => Some(StoryValue::Int(*n)),
        Expr::Float(s, _) => s.parse().ok().map(StoryValue::Float),
        Expr::Bool(b, _) => Some(StoryValue::Bool(*b)),
        Expr::Str(s, _) => Some(StoryValue::String(s.clone())),
        // bare ident in command kwargs is asset/id string
        Expr::Ident(s, _) => Some(StoryValue::String(s.clone())),
        // unary negation of literal
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => match expr.as_ref() {
            Expr::Int(n, _) => Some(StoryValue::Int(-n)),
            Expr::Float(s, _) => s
                .parse::<f64>()
                .ok()
                .map(|f| StoryValue::Float(-f)),
            _ => None,
        },
        _ => None,
    }
}

/// Collect dialogue lines by walking program structure (for tests without full session).
pub fn collect_dialogue_ops(program: &StoryProgram) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    for scene in program.scenes.values() {
        collect_from_ops(&scene.ops, &mut out);
    }
    out
}

fn collect_from_ops(ops: &[StoryOp], out: &mut Vec<(Option<String>, String)>) {
    for op in ops {
        match op {
            StoryOp::Dialogue { speaker, text } => out.push((speaker.clone(), text.clone())),
            StoryOp::Choice { options } => {
                for a in options {
                    collect_from_ops(&a.body, out);
                }
            }
            StoryOp::If {
                then_ops, else_ops, ..
            } => {
                collect_from_ops(then_ops, out);
                collect_from_ops(else_ops, out);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandRegistry;
    use crate::pipeline::check_source;
    use crate::WELCOME_SAMPLE;

    #[test]
    fn welcome_to_story_program() {
        let cmds = CommandRegistry::builtin();
        let c = check_source(WELCOME_SAMPLE, "welcome.vstory", &cmds);
        assert!(c.ok, "{:?}", c.diags);
        let prog = to_story_program(&c.file, "Welcome").unwrap();
        assert!(prog.scenes.contains_key("start"));
        assert!(prog.scenes.contains_key("ending"));
        assert_eq!(prog.entry, "start");
        let lines = collect_dialogue_ops(&prog);
        assert!(
            lines.iter().any(|(_, t)| t.contains("Bienvenido")),
            "{lines:?}"
        );
    }

    #[test]
    fn command_becomes_host_call() {
        let src = r#"
scene start
call combat.start:
    enemy: forest_guardian
    difficulty: 3
end
"#;
        let cmds = CommandRegistry::builtin();
        let c = check_source(src, "c.vstory", &cmds);
        assert!(c.ok);
        let prog = to_story_program(&c.file, "c").unwrap();
        let sc = prog.scenes.get("start").unwrap();
        assert!(sc.ops.iter().any(|op| matches!(
            op,
            StoryOp::HostCall { name, args }
                if name == "combat.start"
                    && args.get("enemy") == Some(&StoryValue::String("forest_guardian".into()))
        )));
    }
}
