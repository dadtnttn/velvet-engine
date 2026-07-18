//! Lower Velvet Story AST → product [`StoryProgram`] (velvet-story IR).
//!
//! Preferred product path for Velvet 2.5 writers (not a second VM).

use indexmap::IndexMap;
use velvet_story::{
    AssignOp, Character, StoryChoice, StoryOp, StoryProgram, StoryScene, StoryValue,
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
        Stmt::Sound { .. } => vec![StoryOp::Nop],
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
        Stmt::Return { .. } => vec![StoryOp::Nop],
        Stmt::End { .. } => vec![StoryOp::End { ending: None }],
        Stmt::Label { name, .. } => vec![StoryOp::Label { name: name.clone() }],
        Stmt::Set { name, value, span } => {
            let v = expr_to_value(value).ok_or_else(|| ToProgramError {
                message: format!("set `{name}` needs a literal value"),
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
            // Product IR If is truthiness of a var. Map `if x > 0` / bare `if x` to that.
            let cond_var = cond_to_var(cond).ok_or_else(|| ToProgramError {
                message: "if supports a variable (or `var > 0`) in StoryProgram path".into(),
                line: span.line,
            })?;
            vec![StoryOp::If {
                cond_var,
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
        Stmt::Pause { .. } | Stmt::Transition { .. } | Stmt::Comment { .. } => {
            vec![StoryOp::Nop]
        }
    })
}

fn cond_to_var(e: &Expr) -> Option<String> {
    match e {
        Expr::Ident(name, _) => Some(name.clone()),
        // `affection > 0` / `>= 1` → truthiness of affection (works for welcome sample)
        Expr::Binary {
            op: BinOp::Gt | BinOp::Ge,
            left,
            right,
            ..
        } => match (left.as_ref(), right.as_ref()) {
            (Expr::Ident(name, _), Expr::Int(0 | 1, _)) => Some(name.clone()),
            _ => None,
        },
        Expr::Unary {
            op: UnaryOp::Not, ..
        } => None,
        _ => None,
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
