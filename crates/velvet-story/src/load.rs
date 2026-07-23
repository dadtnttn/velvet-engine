//! Lower Velvet Script AST into a [`StoryProgram`].

use indexmap::IndexMap;
use thiserror::Error;
use velvet_script_ast::{BinOp, Expr, Item, Module, SourceLoc, Stmt, UnaryOp};
use velvet_script_parser::{parse_file, ParseError};

use crate::character::Character;
use crate::ir::{
    StoryChoice, StoryCmpOp, StoryCond, StoryExpr, StoryOp, StoryOperand, StoryProgram, StoryScene,
};
use crate::value::{from_ast_expr, StoryValue};
use crate::variables::AssignOp;

/// One structured classic-story diagnostic (file:line:col when known).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryDiagnostic {
    /// Human-readable message.
    pub message: String,
    /// Source location (line/column 1-based when known).
    pub loc: SourceLoc,
}

impl StoryDiagnostic {
    /// Format `file:line:col: message` (or `line:col: message`).
    pub fn display(&self) -> String {
        if self.loc.line > 0 {
            format!("{}: {}", self.loc.display(), self.message)
        } else {
            self.message.clone()
        }
    }

    /// Construct from message + loc.
    pub fn new(message: impl Into<String>, loc: SourceLoc) -> Self {
        Self {
            message: message.into(),
            loc,
        }
    }
}

/// Load errors with optional structured diagnostics.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum LoadError {
    /// Parse failed (lexer/syntax) — may carry a location string.
    #[error("parse: {0}")]
    Parse(#[from] ParseError),
    /// One or more structured diagnostics (preferred product path).
    #[error("{}", display_story_diags(.0))]
    Diagnostics(Vec<StoryDiagnostic>),
    /// Semantic issue without structured loc (fallback).
    #[error("{0}")]
    Semantic(String),
}

fn display_story_diags(diags: &[StoryDiagnostic]) -> String {
    if diags.is_empty() {
        return "story load failed".into();
    }
    diags
        .iter()
        .map(StoryDiagnostic::display)
        .collect::<Vec<_>>()
        .join("\n")
}

impl LoadError {
    /// Structured diagnostics when available.
    pub fn diagnostics(&self) -> &[StoryDiagnostic] {
        match self {
            Self::Diagnostics(d) => d,
            _ => &[],
        }
    }

    /// True if any diagnostic has a known line (`line > 0`), or parse syntax loc looks located.
    pub fn has_located_diagnostic(&self) -> bool {
        if self.diagnostics().iter().any(|d| d.loc.line > 0) {
            return true;
        }
        match self {
            Self::Parse(ParseError::Syntax { loc, .. }) => {
                // "file:12:3" or "12:3"
                loc.chars().any(|c| c == ':') && loc.chars().any(|c| c.is_ascii_digit())
            }
            Self::Parse(ParseError::Lexer(e)) => {
                let s = e.to_string();
                s.contains(':') && s.chars().any(|c| c.is_ascii_digit())
            }
            _ => false,
        }
    }

    /// Convenience: first diagnostic message (or Display).
    pub fn primary_message(&self) -> String {
        if let Some(d) = self.diagnostics().first() {
            return d.message.clone();
        }
        self.to_string()
    }
}

/// Load a story program from source text.
pub fn load_program_from_source(
    source: &str,
    file: Option<&str>,
    title: impl Into<String>,
) -> Result<StoryProgram, LoadError> {
    let parsed = parse_file(source, file).map_err(|e| map_parse_err(e, file))?;
    if parsed.module.has_errors() {
        let diags: Vec<_> = parsed
            .module
            .diagnostics
            .iter()
            .filter(|d| d.severity == velvet_script_ast::Severity::Error)
            .map(|d| StoryDiagnostic::new(d.message.clone(), d.loc.clone()))
            .collect();
        if !diags.is_empty() {
            return Err(LoadError::Diagnostics(diags));
        }
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

fn map_parse_err(e: ParseError, file: Option<&str>) -> LoadError {
    match &e {
        ParseError::Syntax { message, loc } => {
            let (line, column) = parse_line_col(loc);
            let mut sl = SourceLoc::at(line, column, Default::default());
            if let Some(f) = file {
                sl = sl.with_file(f);
            } else if let Some((f, rest)) = loc.split_once(':') {
                // loc may be "file.vel:3:5"
                if rest
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
                {
                    sl = SourceLoc::at(line, column, Default::default()).with_file(f);
                }
            }
            LoadError::Diagnostics(vec![StoryDiagnostic::new(message.clone(), sl)])
        }
        ParseError::Lexer(le) => {
            let s = le.to_string();
            let (line, column) = parse_line_col(&s);
            let mut sl = SourceLoc::at(line.max(1), column.max(1), Default::default());
            if let Some(f) = file {
                sl = sl.with_file(f);
            }
            LoadError::Diagnostics(vec![StoryDiagnostic::new(s, sl)])
        }
    }
}

fn parse_line_col(s: &str) -> (u32, u32) {
    // Find first `digits:digits` pattern
    let parts: Vec<&str> = s.split([':', ' ']).collect();
    let mut nums = Vec::new();
    for p in parts {
        if let Ok(n) = p.parse::<u32>() {
            nums.push(n);
            if nums.len() == 2 {
                break;
            }
        }
    }
    match nums.as_slice() {
        [l, c, ..] => (*l, *c),
        [l] => (*l, 1),
        _ => (1, 1),
    }
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
                    labels: IndexMap::new(),
                };
                scene.reindex_labels();
                program.scenes.insert(name.clone(), scene);
            }
            Item::Function { .. } | Item::Screen { .. } | Item::Stmt(_) => {
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
        return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
            "no scenes in story script",
            SourceLoc::at(1, 1, Default::default()),
        )]));
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
        Stmt::HostCall { name, args, .. } => {
            let mut map = IndexMap::new();
            for (k, e) in args {
                if let Some(v) = from_ast_expr(e) {
                    map.insert(k.clone(), v);
                }
            }
            vec![StoryOp::HostCall {
                name: name.clone(),
                args: map,
            }]
        }
        Stmt::Transition { name, .. } => vec![StoryOp::Transition { name: name.clone() }],
        Stmt::Sound { path, .. } => vec![StoryOp::Sound { path: path.clone() }],
        Stmt::Pause { seconds, .. } => vec![StoryOp::Pause { seconds: *seconds }],
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
            let value = StoryExpr::value(from_ast_expr(init).unwrap_or(StoryValue::Null));
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
            loc,
            ..
        } => {
            let cond = lower_cond(cond)
                .map_err(|m| LoadError::Diagnostics(vec![StoryDiagnostic::new(m, loc.clone())]))?;
            let then_ops = lower_stmt(then_body)?;
            let else_ops = match else_body {
                Some(e) => lower_stmt(e)?,
                None => vec![],
            };
            vec![StoryOp::If {
                cond,
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

/// Lower a story condition expression into [`StoryCond`].
fn lower_cond(expr: &Expr) -> Result<StoryCond, String> {
    match expr {
        Expr::Ident { name, .. } => Ok(StoryCond::var(name.clone())),
        Expr::Bool { value, .. } => Ok(StoryCond::Const { value: *value }),
        Expr::Unary {
            op: UnaryOp::Not,
            expr,
            ..
        } => Ok(StoryCond::Not {
            inner: Box::new(lower_cond(expr)?),
        }),
        Expr::Binary {
            left, op, right, ..
        } => match op {
            BinOp::And => Ok(StoryCond::And {
                left: Box::new(lower_cond(left)?),
                right: Box::new(lower_cond(right)?),
            }),
            BinOp::Or => Ok(StoryCond::Or {
                left: Box::new(lower_cond(left)?),
                right: Box::new(lower_cond(right)?),
            }),
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let cmp = match op {
                    BinOp::Eq => StoryCmpOp::Eq,
                    BinOp::Ne => StoryCmpOp::Ne,
                    BinOp::Lt => StoryCmpOp::Lt,
                    BinOp::Le => StoryCmpOp::Le,
                    BinOp::Gt => StoryCmpOp::Gt,
                    BinOp::Ge => StoryCmpOp::Ge,
                    _ => unreachable!(),
                };
                Ok(StoryCond::Cmp {
                    left: lower_operand(left)?,
                    op: cmp,
                    right: lower_operand(right)?,
                })
            }
            _ => Err(format!(
                "story if does not support operator `{op:?}` in condition"
            )),
        },
        _ => Err(
            "story if supports identifiers, bools, ! / && / ||, and comparisons in conditions"
                .into(),
        ),
    }
}

fn lower_operand(expr: &Expr) -> Result<StoryOperand, String> {
    match expr {
        Expr::Ident { name, .. } => Ok(StoryOperand::var(name.clone())),
        Expr::Int { value, .. } => Ok(StoryOperand::value(StoryValue::Int(*value))),
        Expr::Float { value, .. } => Ok(StoryOperand::value(StoryValue::Float(*value))),
        Expr::Bool { value, .. } => Ok(StoryOperand::value(StoryValue::Bool(*value))),
        Expr::String { value, .. } => Ok(StoryOperand::value(StoryValue::String(value.clone()))),
        _ => Err("comparison operand must be identifier or literal".into()),
    }
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
            let value = StoryExpr::value(from_ast_expr(right).unwrap_or(StoryValue::Null));
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
    use crate::runtime::{StoryPlayer, StoryWait};

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

    #[test]
    fn broken_source_has_located_diagnostic() {
        let src = r#"
scene broken {
    if {
        "x"
    }
}
"#;
        let err = load_program_from_source(src, Some("broken.vel"), "B").unwrap_err();
        assert!(
            err.has_located_diagnostic(),
            "expected located diag, got {err}"
        );
        let s = err.to_string();
        assert!(
            s.contains("broken.vel") || s.contains(':'),
            "display should include loc: {s}"
        );
        assert!(!err.primary_message().is_empty());
    }

    #[test]
    fn no_scenes_is_structured_diagnostic() {
        let src = r#"
character hero { name: "H" }
"#;
        let err = load_program_from_source(src, Some("empty.vel"), "E").unwrap_err();
        assert_eq!(err.diagnostics().len(), 1);
        assert_eq!(err.diagnostics()[0].message, "no scenes in story script");
        assert_eq!(err.diagnostics()[0].loc.line, 1);
    }

    #[test]
    fn labels_jump_if_choice_on_real_player() {
        let src = r#"
state {
    trust: int = 0
    path: int = 0
}

scene main {
    background "bg/station.png"
    show nora.happy at left
    "Intro line"
    label fork:
    if trust > 0 {
        jump good
    } else {
        jump choice_room
    }
}

scene choice_room {
    "Pick one"
    choice {
        "Help her" {
            trust += 1
            path = 1
            jump good
        }
        "Walk away" {
            path = 2
            jump bad
        }
        "Stay silent" {
            path = 3
            jump bad
        }
    }
}

scene good {
    label win:
    "Good end"
    end good_end
}

scene bad {
    "Bad end"
    end bad_end
}
"#;
        let program = load_program_from_source(src, Some("surface.vel"), "S").unwrap();
        // Labels indexed
        let main = program.scenes.get("main").unwrap();
        assert!(main.labels.contains_key("fork"), "labels={:?}", main.labels);
        let good = program.scenes.get("good").unwrap();
        assert!(good.labels.contains_key("win"));

        // Path: trust=0 → choice → help → good
        let mut player = StoryPlayer::start(program.clone());
        let mut steps = 0;
        loop {
            steps += 1;
            assert!(steps < 40, "stuck wait={:?}", player.wait());
            match player.wait().clone() {
                StoryWait::Line | StoryWait::Ready => player.advance(),
                StoryWait::Choice => {
                    assert!(
                        player.choices().len() >= 3,
                        "multi-arm {:?}",
                        player.choices()
                    );
                    player.choose(0).unwrap(); // Help her
                }
                StoryWait::Ended => break,
                other => panic!("unexpected {other:?}"),
            }
        }
        assert_eq!(player.ending(), Some("good_end"));
        assert_eq!(player.variables().get_int("trust", 0), 1);
        assert_eq!(player.variables().get_int("path", 0), 1);

        // if branch: pre-set trust via state — use second program with trust 1
        let src2 = src.replacen("trust: int = 0", "trust: int = 1", 1);
        let program2 = load_program_from_source(&src2, Some("surface2.vel"), "S").unwrap();
        let mut p2 = StoryPlayer::start(program2);
        let mut steps = 0;
        loop {
            steps += 1;
            assert!(steps < 40);
            match p2.wait().clone() {
                StoryWait::Line | StoryWait::Ready => p2.advance(),
                StoryWait::Choice => panic!("should skip choice when trust>0"),
                StoryWait::Ended => break,
                _ => {}
            }
        }
        assert_eq!(p2.ending(), Some("good_end"));
    }

    #[test]
    fn presentation_ops_lower_from_classic_vel() {
        let src = r#"
scene main {
    background "bg/rain.png"
    transition fade
    show hero.angry at right
    sound "sfx/hit.ogg"
    pause 0.1
    hide hero
    "done"
    end
}
"#;
        let program = load_program_from_source(src, Some("pres.vel"), "P").unwrap();
        let ops = &program.scenes["main"].ops;
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Background { path } if path.contains("rain"))));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Transition { name } if name == "fade")));
        assert!(ops.iter().any(|o| matches!(o, StoryOp::Show { target, at } if target == "hero.angry" && at.as_deref() == Some("right"))));
        assert!(ops.iter().any(|o| matches!(o, StoryOp::Sound { .. })));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Pause { seconds: Some(0.1) })));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Hide { target } if target == "hero")));
    }

    #[test]
    fn host_call_lowers_from_dotted_call() {
        let src = r#"
scene main {
    call combat.start enemy "goblin"
    "after"
    end
}
"#;
        let program = load_program_from_source(src, Some("host.vel"), "H").unwrap();
        let ops = &program.scenes["main"].ops;
        let host = ops.iter().find_map(|o| match o {
            StoryOp::HostCall { name, args } => Some((name.as_str(), args)),
            _ => None,
        });
        let (name, args) = host.expect("HostCall");
        assert_eq!(name, "combat.start");
        assert_eq!(
            args.get("enemy"),
            Some(&StoryValue::String("goblin".into()))
        );
    }
}
