//! Lower Velvet Story AST → product [`StoryProgram`] (velvet-story IR).
//!
//! Preferred product path for Velvet 2.5 writers (not a second VM).
//! Also builds parallel [`OpSrc`] trees so StoryProgram → OpVs2 can emit
//! source maps with real PCs and include-aware origins.

use indexmap::IndexMap;
use velvet_story::{
    AssignOp, Character, StoryArithOp, StoryChoice, StoryCmpOp, StoryCond, StoryExpr, StoryOp,
    StoryOperand, StoryProgram, StoryScene, StoryValue,
};

use crate::ast::{BinOp, Expr, Stmt, StoryFile, TopItem};
use crate::span::Span;

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

/// Origin of one [`StoryOp`] (mirrors nested If/Choice structure).
#[derive(Debug, Clone)]
pub enum OpSrc {
    /// Leaf op (dialogue, assign, jump, …).
    Leaf {
        /// Origin file (include-aware).
        file: String,
        /// Writer span.
        span: Span,
        /// Node kind for maps.
        kind: String,
        /// Generated note.
        gen: String,
    },
    /// Conditional block.
    If {
        /// Origin file.
        file: String,
        /// Span of `if`.
        span: Span,
        /// Then-arm origins (parallel to then_ops).
        then: Vec<OpSrc>,
        /// Else-arm origins.
        else_ops: Vec<OpSrc>,
    },
    /// Choice menu.
    Choice {
        /// Origin file.
        file: String,
        /// Span of `choice`.
        span: Span,
        /// Per-arm body origins.
        arms: Vec<Vec<OpSrc>>,
    },
}

/// Product IR plus source origins for PC-aware maps.
#[derive(Debug, Clone)]
pub struct ProgramWithOrigins {
    /// StoryProgram.
    pub program: StoryProgram,
    /// Per-scene op origins (same order/shape as scene.ops).
    pub origins: IndexMap<String, Vec<OpSrc>>,
}

/// Lower a resolved story file to product IR (origins discarded).
pub fn to_story_program(
    file: &StoryFile,
    title: impl Into<String>,
) -> Result<StoryProgram, ToProgramError> {
    Ok(to_story_program_with_origins(file, title)?.program)
}

/// Lower with parallel origin tree for source maps.
pub fn to_story_program_with_origins(
    file: &StoryFile,
    title: impl Into<String>,
) -> Result<ProgramWithOrigins, ToProgramError> {
    let mut program = StoryProgram::new(title);
    let mut origins: IndexMap<String, Vec<OpSrc>> = IndexMap::new();
    let mut first_scene = None;

    for item in &file.items {
        match item {
            TopItem::Scene(sc) => {
                if first_scene.is_none() {
                    first_scene = Some(sc.name.clone());
                }
                let origin = sc
                    .origin_file
                    .clone()
                    .unwrap_or_else(|| file.file.clone());
                let (ops, srcs) = lower_stmts(&sc.body, &origin)?;
                let mut scene = StoryScene {
                    name: sc.name.clone(),
                    ops,
                    labels: IndexMap::new(),
                };
                scene.reindex_labels();
                program.scenes.insert(sc.name.clone(), scene);
                origins.insert(sc.name.clone(), srcs);
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
    if program.scenes.contains_key("start") {
        program.entry = "start".into();
    }
    Ok(ProgramWithOrigins { program, origins })
}

fn lower_stmts(body: &[Stmt], origin: &str) -> Result<(Vec<StoryOp>, Vec<OpSrc>), ToProgramError> {
    let mut ops = Vec::new();
    let mut srcs = Vec::new();
    for st in body {
        let (o, s) = lower_stmt(st, origin)?;
        ops.extend(o);
        srcs.extend(s);
    }
    Ok((ops, srcs))
}

fn leaf(origin: &str, span: Span, kind: &str, gen: impl Into<String>) -> OpSrc {
    OpSrc::Leaf {
        file: origin.into(),
        span,
        kind: kind.into(),
        gen: gen.into(),
    }
}

fn lower_stmt(st: &Stmt, origin: &str) -> Result<(Vec<StoryOp>, Vec<OpSrc>), ToProgramError> {
    Ok(match st {
        Stmt::Background { id, span } => (
            vec![StoryOp::Background { path: id.clone() }],
            vec![leaf(origin, *span, "background", id.clone())],
        ),
        Stmt::Music { id, span } => (
            vec![StoryOp::Music {
                path: id.clone(),
                fade_in: None,
            }],
            vec![leaf(origin, *span, "music", id.clone())],
        ),
        Stmt::Show {
            character,
            expression,
            at,
            span,
        } => {
            let target = match expression {
                Some(e) => format!("{character}.{e}"),
                None => character.clone(),
            };
            (
                vec![StoryOp::Show {
                    target: target.clone(),
                    at: at.clone(),
                }],
                vec![leaf(origin, *span, "show", target)],
            )
        }
        Stmt::Hide { character, span } => (
            vec![StoryOp::Hide {
                target: character.clone(),
            }],
            vec![leaf(origin, *span, "hide", character.clone())],
        ),
        Stmt::Dialogue {
            speaker,
            text,
            span,
            ..
        } => {
            let sp = if speaker == "narrator" {
                None
            } else {
                Some(speaker.clone())
            };
            (
                vec![StoryOp::Dialogue {
                    speaker: sp,
                    text: text.clone(),
                }],
                vec![leaf(origin, *span, "dialogue", speaker.clone())],
            )
        }
        Stmt::Choice {
            options, span, ..
        } => {
            let mut arms = Vec::new();
            let mut arm_srcs = Vec::new();
            for o in options {
                let (body, srcs) = lower_stmts(&o.body, origin)?;
                arms.push(StoryChoice {
                    text: o.label.clone(),
                    body,
                    require: None,
                    hidden_if_locked: false,
                });
                arm_srcs.push(srcs);
            }
            (
                vec![StoryOp::Choice { options: arms }],
                vec![OpSrc::Choice {
                    file: origin.into(),
                    span: *span,
                    arms: arm_srcs,
                }],
            )
        }
        Stmt::Goto { target, span } => (
            vec![StoryOp::Jump {
                target: target.clone(),
            }],
            vec![leaf(origin, *span, "goto", target.clone())],
        ),
        Stmt::CallScene { target, span } => (
            vec![StoryOp::Call {
                target: target.clone(),
            }],
            vec![leaf(origin, *span, "call_scene", target.clone())],
        ),
        Stmt::Return { span } => (
            vec![StoryOp::Return],
            vec![leaf(origin, *span, "return", "return")],
        ),
        Stmt::End { span } => (
            vec![StoryOp::End { ending: None }],
            vec![leaf(origin, *span, "end", "end")],
        ),
        Stmt::Label { name, span } => (
            vec![StoryOp::Label { name: name.clone() }],
            vec![leaf(origin, *span, "label", name.clone())],
        ),
        Stmt::Set { name, value, span } => {
            let v = expr_to_story_expr(value).ok_or_else(|| ToProgramError {
                message: format!(
                    "set `{name}` needs a literal, variable, or arithmetic expression"
                ),
                line: span.line,
            })?;
            (
                vec![StoryOp::Assign {
                    name: name.clone(),
                    assign_op: AssignOp::Set,
                    value: v,
                }],
                vec![leaf(origin, *span, "set", name.clone())],
            )
        }
        Stmt::Add { name, value, span } => {
            let v = expr_to_story_expr(value).ok_or_else(|| ToProgramError {
                message: format!(
                    "add `{name}` needs a literal, variable, or arithmetic expression"
                ),
                line: span.line,
            })?;
            (
                vec![StoryOp::Assign {
                    name: name.clone(),
                    assign_op: AssignOp::Add,
                    value: v,
                }],
                vec![leaf(origin, *span, "add", name.clone())],
            )
        }
        Stmt::Sub { name, value, span } => {
            let v = expr_to_story_expr(value).ok_or_else(|| ToProgramError {
                message: format!(
                    "sub `{name}` needs a literal, variable, or arithmetic expression"
                ),
                line: span.line,
            })?;
            (
                vec![StoryOp::Assign {
                    name: name.clone(),
                    assign_op: AssignOp::Sub,
                    value: v,
                }],
                vec![leaf(origin, *span, "sub", name.clone())],
            )
        }
        Stmt::If {
            cond,
            then_body,
            else_body,
            span,
        } => {
            let story_cond = expr_to_cond(cond, span.line)?;
            let (then_ops, then_src) = lower_stmts(then_body, origin)?;
            let (else_ops, else_src) = match else_body {
                Some(e) => lower_stmts(e, origin)?,
                None => (vec![], vec![]),
            };
            (
                vec![StoryOp::If {
                    cond: story_cond,
                    then_ops,
                    else_ops,
                }],
                vec![OpSrc::If {
                    file: origin.into(),
                    span: *span,
                    then: then_src,
                    else_ops: else_src,
                }],
            )
        }
        Stmt::CallCommand { name, args, span } => {
            let mut map = IndexMap::new();
            for (k, v) in args {
                // Command kwargs: bare idents stay string ids (enemy: forest_guardian).
                let sv = expr_to_literal_value(v).ok_or_else(|| ToProgramError {
                    message: format!("command arg `{k}` must be a literal"),
                    line: span.line,
                })?;
                map.insert(k.clone(), sv);
            }
            (
                vec![StoryOp::HostCall {
                    name: name.clone(),
                    args: map,
                }],
                vec![leaf(origin, *span, "call", name.clone())],
            )
        }
        Stmt::Sound { id, span } => (
            vec![StoryOp::Sound { path: id.clone() }],
            vec![leaf(origin, *span, "sound", id.clone())],
        ),
        Stmt::Pause { duration, span } => {
            let seconds = match duration {
                None => None,
                Some(e) => match expr_to_literal_value(e) {
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
            (
                vec![StoryOp::Pause { seconds }],
                vec![leaf(origin, *span, "pause", "await")],
            )
        }
        Stmt::Transition { name, span } => (
            vec![StoryOp::Transition { name: name.clone() }],
            vec![leaf(origin, *span, "transition", name.clone())],
        ),
        Stmt::Comment { .. } => (vec![], vec![]),
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

use crate::ast::UnaryOp;

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
    match e {
        Expr::Ident(name, _) => Ok(StoryOperand::var(name.clone())),
        _ => match expr_to_literal_value(e) {
            Some(v) => Ok(StoryOperand::value(v)),
            None => Err(ToProgramError {
                message: "comparison operand must be a variable or literal".into(),
                line,
            }),
        },
    }
}

/// Assignment / arithmetic RHS: idents are **variables**.
fn expr_to_story_expr(e: &Expr) -> Option<StoryExpr> {
    match e {
        Expr::Int(n, _) => Some(StoryExpr::value(StoryValue::Int(*n))),
        Expr::Float(s, _) => s.parse().ok().map(|f| StoryExpr::value(StoryValue::Float(f))),
        Expr::Bool(b, _) => Some(StoryExpr::value(StoryValue::Bool(*b))),
        Expr::Str(s, _) => Some(StoryExpr::value(StoryValue::String(s.clone()))),
        Expr::Ident(name, _) => Some(StoryExpr::var(name.clone())),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => Some(StoryExpr::Neg {
            inner: Box::new(expr_to_story_expr(expr)?),
        }),
        Expr::Binary {
            op: op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div),
            left,
            right,
            ..
        } => Some(StoryExpr::Binary {
            op: match op {
                BinOp::Add => StoryArithOp::Add,
                BinOp::Sub => StoryArithOp::Sub,
                BinOp::Mul => StoryArithOp::Mul,
                BinOp::Div => StoryArithOp::Div,
                _ => unreachable!(),
            },
            left: Box::new(expr_to_story_expr(left)?),
            right: Box::new(expr_to_story_expr(right)?),
        }),
        _ => None,
    }
}

/// Command kwargs / pause: idents are **string ids**, not variables.
fn expr_to_literal_value(e: &Expr) -> Option<StoryValue> {
    match e {
        Expr::Int(n, _) => Some(StoryValue::Int(*n)),
        Expr::Float(s, _) => s.parse().ok().map(StoryValue::Float),
        Expr::Bool(b, _) => Some(StoryValue::Bool(*b)),
        Expr::Str(s, _) => Some(StoryValue::String(s.clone())),
        Expr::Ident(s, _) => Some(StoryValue::String(s.clone())),
        Expr::Unary {
            op: UnaryOp::Neg,
            expr,
            ..
        } => match expr.as_ref() {
            Expr::Int(n, _) => Some(StoryValue::Int(-n)),
            Expr::Float(s, _) => s.parse::<f64>().ok().map(|f| StoryValue::Float(-f)),
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

    #[test]
    fn arithmetic_set_and_var_add() {
        let src = r#"
scene start
set score = 5
set bonus = 2
set total = score + bonus
add score bonus
set half = total / 2
end
"#;
        let cmds = CommandRegistry::builtin();
        let c = check_source(src, "e.vstory", &cmds);
        assert!(c.ok, "{:?}", c.diags);
        let prog = to_story_program(&c.file, "e").unwrap();
        let sc = prog.scenes.get("start").unwrap();
        assert!(sc.ops.iter().any(|op| matches!(
            op,
            StoryOp::Assign {
                name,
                value: StoryExpr::Binary { .. },
                ..
            } if name == "total"
        )));
        assert!(sc.ops.iter().any(|op| matches!(
            op,
            StoryOp::Assign {
                name,
                assign_op: AssignOp::Add,
                value: StoryExpr::Var { name: v },
                ..
            } if name == "score" && v == "bonus"
        )));
    }
}
