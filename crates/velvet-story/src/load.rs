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
///
/// Two passes: screens/transforms/characters/state first (so scenes can
/// reference them regardless of author order), then scenes.
pub fn lower_module(module: &Module, title: String) -> Result<StoryProgram, LoadError> {
    let mut program = StoryProgram::new(title);
    let mut first_scene: Option<String> = None;
    let mut scene_bodies: Vec<(String, Vec<Stmt>)> = Vec::new();

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
                scene_bodies.push((name.clone(), body.clone()));
            }
            Item::Screen {
                name,
                properties,
                buttons,
                params,
                children,
                loc,
            } => {
                if program.screens.contains_key(name) {
                    return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                        format!("duplicate screen `{name}`"),
                        loc.clone(),
                    )]));
                }
                let def = crate::screens::lower_screen(name, properties, buttons, params, children)
                    .map_err(|m| {
                        LoadError::Diagnostics(vec![StoryDiagnostic::new(m, loc.clone())])
                    })?;
                program.screens.insert(name.clone(), def);
            }
            Item::Transform { name, steps, loc } => {
                if program.transforms.contains_key(name) {
                    return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                        format!("duplicate transform `{name}`"),
                        loc.clone(),
                    )]));
                }
                let def = crate::atl::lower_transform(name, steps).map_err(|m| {
                    LoadError::Diagnostics(vec![StoryDiagnostic::new(m, loc.clone())])
                })?;
                program.transforms.insert(name.clone(), def);
            }
            Item::Import { .. } | Item::Function { .. } | Item::Stmt(_) => {
                // Non-story items ignored for VN IR (can still run via script VM).
            }
        }
    }
    // Screen `use` targets must exist (checked after all screens collected).
    for def in program.screens.values() {
        for (target, args_len) in def.uses() {
            let Some(target_def) = program.screens.get(&target) else {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!("screen `{}` uses unknown screen `{target}`", def.name),
                    SourceLoc::unknown(),
                )]));
            };
            if let Err(m) = crate::screens::check_call_arity(&target, &target_def.params, args_len)
            {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!("screen `{}`: {m}", def.name),
                    SourceLoc::unknown(),
                )]));
            }
        }
    }

    let ctx = LowerCtx {
        screens: &program.screens,
        transforms: &program.transforms,
    };
    for (name, body) in &scene_bodies {
        let mut scene = StoryScene {
            name: name.clone(),
            ops: lower_stmts(body, &ctx)?,
            labels: IndexMap::new(),
        };
        scene.reindex_labels();
        program.scenes.insert(name.clone(), scene);
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
    validate_screen_actions(&program)?;
    Ok(program)
}

/// Validate known `textbutton … action …` calls inside game screens.
///
/// Known actions keep their contract (`Return(x)`, `Jump("scene")`,
/// `Show("screen")` / `Hide("screen")`, `Set("var", value)`); unknown
/// action names are custom host actions and pass through untouched.
fn validate_screen_actions(program: &StoryProgram) -> Result<(), LoadError> {
    for def in program.screens.values() {
        check_screen_children(program, &def.name, &def.children)?;
    }
    Ok(())
}

fn check_screen_children(
    program: &StoryProgram,
    screen: &str,
    children: &[crate::screens::ScreenChildRt],
) -> Result<(), LoadError> {
    for child in children {
        // Actions live in `textbutton` (`action:`) and generic `button`
        // widgets; both lower the label/action pair the same way.
        if child.kind == "textbutton" || child.kind == "button" {
            if let Some(action) = child.props.iter().find(|p| p.name == "action") {
                check_screen_action(program, screen, &action.value)?;
            }
        }
        check_screen_children(program, screen, &child.children)?;
    }
    Ok(())
}

fn check_screen_action(
    program: &StoryProgram,
    screen: &str,
    value: &crate::screens::ScreenValueRt,
) -> Result<(), LoadError> {
    // Only literal actions validate; dynamic expressions resolve at the host.
    let source = match value {
        crate::screens::ScreenValueRt::Lit {
            value: crate::value::StoryValue::String(s),
        } => s.clone(),
        crate::screens::ScreenValueRt::Expr { source } => source.clone(),
        _ => return Ok(()),
    };
    let Some(call) = crate::screens::parse_screen_action(&source) else {
        // Not a call (plain ident / string): host-resolved, skip.
        return Ok(());
    };
    let bad = |message: String| {
        LoadError::Diagnostics(vec![StoryDiagnostic::new(
            format!("screen `{screen}` action: {message}"),
            SourceLoc::unknown(),
        )])
    };
    match call.name.as_str() {
        "Return" => {
            if call.args.len() != 1 {
                return Err(bad(format!(
                    "Return needs 1 argument, got {}",
                    call.args.len()
                )));
            }
        }
        "Jump" => {
            let target = match call.args.as_slice() {
                [crate::value::StoryValue::String(t)] => t.clone(),
                _ => {
                    return Err(bad("Jump needs 1 scene name, e.g. `Jump(\"start\")`".into()));
                }
            };
            if !program.scenes.contains_key(&target) {
                return Err(bad(format!("Jump targets unknown scene `{target}`")));
            }
        }
        "Show" | "Hide" => {
            let target = match call.args.as_slice() {
                [crate::value::StoryValue::String(t)] => t.clone(),
                _ => {
                    return Err(bad(format!(
                        "{} needs 1 screen id, e.g. `{}( \"help\" )`",
                        call.name, call.name
                    )));
                }
            };
            if !program.screens.contains_key(&target) {
                return Err(bad(format!(
                    "{} targets unknown screen `{target}`",
                    call.name
                )));
            }
        }
        "Set" => match call.args.as_slice() {
            [crate::value::StoryValue::String(var), _] if is_var_name(var) => {}
            _ => {
                return Err(bad(
                    "Set needs a variable name and a value, e.g. `Set(\"gold\", 5)`".into(),
                ));
            }
        },
        // Custom host actions pass through.
        _ => {}
    }
    Ok(())
}

/// Tables scenes may reference (collected before scene lowering).
struct LowerCtx<'a> {
    screens: &'a IndexMap<String, crate::screens::ScreenDef>,
    transforms: &'a IndexMap<String, crate::atl::TransformDef>,
}

fn lower_stmts(stmts: &[Stmt], ctx: &LowerCtx) -> Result<Vec<StoryOp>, LoadError> {
    let mut ops = Vec::new();
    for s in stmts {
        ops.extend(lower_stmt(s, ctx)?);
    }
    Ok(ops)
}

/// Plain variable name check for `into <var>` bindings.
fn is_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Resolve a `with <mini-motion|transform>` suffix shared by show/hide.
fn lower_with(
    with: &Option<String>,
    ctx: &LowerCtx,
    loc: &SourceLoc,
) -> Result<(Option<crate::ir::ShowMotion>, Option<String>), LoadError> {
    // `with <mini-motion>` wins; otherwise it names an ATL transform.
    match with {
        Some(w) => match crate::atl::parse_motion_words(w) {
            Some(m) => Ok((
                Some(crate::ir::ShowMotion::new(
                    m.kind.as_str(),
                    m.duration,
                    m.amount,
                )),
                None,
            )),
            None => {
                if !ctx.transforms.contains_key(w) {
                    return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                        format!(
                            "unknown motion or transform `{w}` (expected move, zoom, shake, fade, or a `transform` name)"
                        ),
                        loc.clone(),
                    )]));
                }
                Ok((None, Some(w.clone())))
            }
        },
        None => Ok((None, None)),
    }
}

fn lower_stmt(stmt: &Stmt, ctx: &LowerCtx) -> Result<Vec<StoryOp>, LoadError> {
    Ok(match stmt {
        Stmt::Background { path, .. } => vec![StoryOp::Background { path: path.clone() }],
        Stmt::Music { path, fade_in, .. } => vec![StoryOp::Music {
            path: path.clone(),
            fade_in: *fade_in,
        }],
        Stmt::Show {
            target,
            at,
            with,
            loc,
        } => {
            let (motion, transform) = lower_with(with, ctx, loc)?;
            vec![StoryOp::Show {
                target: target.clone(),
                at: at.clone(),
                motion,
                transform,
            }]
        }
        Stmt::ScreenAction {
            action,
            id,
            args,
            with,
            result,
            loc,
        } => {
            let Some(def) = ctx.screens.get(id) else {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!("unknown screen `{id}` (declare it with `screen {id} {{ … }}`)"),
                    loc.clone(),
                )]));
            };
            let mut values = Vec::new();
            for e in args {
                match from_ast_expr(e) {
                    Some(v) => values.push(v),
                    None => {
                        return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                            format!(
                                "screen `{id}` arguments must be literals (got a dynamic expression)"
                            ),
                            loc.clone(),
                        )]));
                    }
                }
            }
            if !matches!(action, velvet_script_ast::ScreenActionKind::Hide) {
                if let Err(m) = crate::screens::check_call_arity(id, &def.params, values.len()) {
                    return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                        m,
                        loc.clone(),
                    )]));
                }
            }
            if matches!(action, velvet_script_ast::ScreenActionKind::Hide) && !values.is_empty() {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!("screen hide `{id}` takes no arguments"),
                    loc.clone(),
                )]));
            }
            // `into` only makes sense for modal calls (show/hide return nothing).
            if result.is_some() && !matches!(action, velvet_script_ast::ScreenActionKind::Call) {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!(
                        "screen {} `{id}` takes no `into` (only `screen call` returns a value)",
                        action.as_str()
                    ),
                    loc.clone(),
                )]));
            }
            if let Some(var) = result {
                if !is_var_name(var) {
                    return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                        format!("`into` needs a variable name, got `{var}`"),
                        loc.clone(),
                    )]));
                }
            }
            let (motion, transform) = lower_with(with, ctx, loc)?;
            match action {
                velvet_script_ast::ScreenActionKind::Show => vec![StoryOp::ShowGameScreen {
                    id: id.clone(),
                    args: values,
                    motion,
                    transform,
                }],
                velvet_script_ast::ScreenActionKind::Hide => vec![StoryOp::HideGameScreen {
                    id: id.clone(),
                    motion,
                    transform,
                }],
                velvet_script_ast::ScreenActionKind::Call => vec![StoryOp::CallGameScreen {
                    id: id.clone(),
                    args: values,
                    motion,
                    transform,
                    result: result.clone(),
                }],
            }
        }
        Stmt::Screen { id, loc } => {
            if crate::screens::ScreenPreset::from_id(id).is_none() {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!(
                        "unknown screen preset `{id}` (expected say_classic, say_minimal, nvl_page or menu)"
                    ),
                    loc.clone(),
                )]));
            }
            vec![StoryOp::Screen { id: id.clone() }]
        }
        Stmt::Unlock { kind, id, loc } => {
            let kind_lower = kind.to_ascii_lowercase();
            if kind_lower != "gallery" && kind_lower != "music" {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    format!("unknown unlock catalog `{kind}` (expected gallery or music)"),
                    loc.clone(),
                )]));
            }
            if id.trim().is_empty() {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    "unlock needs an entry id, e.g. `unlock gallery cg_rooftop`".to_string(),
                    loc.clone(),
                )]));
            }
            vec![StoryOp::Unlock {
                catalog: kind_lower,
                id: id.clone(),
            }]
        }
        Stmt::Movie {
            path,
            loop_clip,
            loc,
        } => {
            if path.trim().is_empty() {
                return Err(LoadError::Diagnostics(vec![StoryDiagnostic::new(
                    "movie needs a clip directory, e.g. `movie \"assets/video/opening\"`"
                        .to_string(),
                    loc.clone(),
                )]));
            }
            vec![StoryOp::Movie {
                path: path.clone(),
                loop_clip: *loop_clip,
            }]
        }
        Stmt::Hide { target, with, loc } => {
            let (motion, transform) = lower_with(with, ctx, loc)?;
            vec![StoryOp::Hide {
                target: target.clone(),
                motion,
                transform,
            }]
        }
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
                    body: lower_stmts(&arm.body, ctx)?,
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
        Stmt::Block { body, .. } => lower_stmts(body, ctx)?,
        Stmt::If {
            cond,
            then_body,
            else_body,
            loc,
            ..
        } => {
            let cond = lower_cond(cond)
                .map_err(|m| LoadError::Diagnostics(vec![StoryDiagnostic::new(m, loc.clone())]))?;
            let then_ops = lower_stmt(then_body, ctx)?;
            let else_ops = match else_body {
                Some(e) => lower_stmt(e, ctx)?,
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
        assert!(ops.iter().any(|o| matches!(o, StoryOp::Show { target, at, .. } if target == "hero.angry" && at.as_deref() == Some("right"))));
        assert!(ops.iter().any(|o| matches!(o, StoryOp::Sound { .. })));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Pause { seconds: Some(0.1) })));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Hide { target, .. } if target == "hero")));
    }

    #[test]
    fn show_with_and_screen_lower() {
        let src = r#"
scene main {
    screen nvl_page
    show luna.happy at left with shake 0.4 10
    show luna with move
    "done"
    end
}
"#;
        let program = load_program_from_source(src, Some("m.vel"), "M").unwrap();
        let ops = &program.scenes["main"].ops;
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Screen { id } if id == "nvl_page")));
        let motions: Vec<_> = ops
            .iter()
            .filter_map(|o| match o {
                StoryOp::Show { motion, .. } => motion.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(motions.len(), 2);
        assert_eq!(motions[0].kind, "shake");
        assert!((motions[0].duration_secs - 0.4).abs() < 1e-5);
        assert_eq!(motions[1].kind, "move");
    }

    #[test]
    fn hide_with_lowers_motion_or_transform() {
        let src = "transform bob {\n y 0 1.0 linear\n}\nscene main {\n hide luna with fade\n hide kai with bob\n \"x\"\n}\n";
        let program = load_program_from_source(src, Some("h.vel"), "H").unwrap();
        let ops = &program.scenes["main"].ops;
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::Hide { target, motion: Some(m), .. } if target == "luna" && m.kind == "fade")
        ));
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::Hide { target, transform: Some(t), .. } if target == "kai" && t == "bob")
        ));
        let bad = "scene main {\n hide luna with wobble\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err
            .primary_message()
            .contains("unknown motion or transform"));
    }

    #[test]
    fn screen_action_with_into_and_validator() {
        let src = r#"
screen help {
  title: "Help"
  textbutton go action Jump("next")
}
screen bad {
  textbutton x action Jump("nowhere")
}
scene main {
  screen show help with fade
  screen call help into pick
  "x"
}
scene next {
  "y"
}
"#;
        // The bad screen fails first: Jump to an unknown scene.
        let err = load_program_from_source(src, Some("v.vel"), "V").unwrap_err();
        assert!(err.primary_message().contains("unknown scene `nowhere`"));
        let src = src.replace(
            "screen bad {\n  textbutton x action Jump(\"nowhere\")\n}\n",
            "",
        );
        let program = load_program_from_source(&src, Some("v.vel"), "V").unwrap();
        let ops = &program.scenes["main"].ops;
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::ShowGameScreen { id, motion: Some(m), .. } if id == "help" && m.kind == "fade")
        ));
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::CallGameScreen { id, result: Some(r), .. } if id == "help" && r == "pick")
        ));
        // `into` on show is rejected.
        let bad = "screen a {\n text \"x\"\n}\nscene main {\n screen show a into pick\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err.primary_message().contains("takes no `into`"));
        // Bad action arity is rejected.
        let bad = "screen a {\n textbutton x action Return(\"a\", \"b\")\n}\nscene main {\n screen show a\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err.primary_message().contains("Return needs 1 argument"));
        // Unknown action names pass through for custom hosts.
        let src = "screen a {\n textbutton x action Dance(\"wild\")\n}\nscene main {\n screen show a\n \"x\"\n}\n";
        load_program_from_source(src, Some("ok.vel"), "OK").unwrap();
    }

    #[test]
    fn into_binding_lands_in_variable() {
        let src = r#"
screen pick {
  textbutton a action Return("a")
}
scene main {
  screen call pick into choice
  "after"
  end "done"
}
"#;
        let program = load_program_from_source(src, Some("c.vel"), "C").unwrap();
        let mut player = StoryPlayer::start(program);
        let mut guard = 0;
        while !matches!(player.wait(), StoryWait::ScreenCall { .. }) && guard < 16 {
            player.advance();
            guard += 1;
        }
        player
            .return_from_screen(StoryValue::String("a".into()))
            .unwrap();
        assert_eq!(
            player.variables().get("choice").display_str(),
            "a".to_string()
        );
        assert_eq!(
            player.variables().get("_return").display_str(),
            "a".to_string()
        );
    }

    #[test]
    fn movie_lowers_with_loop_flag() {
        let src = "scene main {\n movie \"assets/video/opening\" loop\n \"x\"\n}\n";
        let program = load_program_from_source(src, Some("m.vel"), "M").unwrap();
        assert!(program.scenes["main"]
            .ops
            .iter()
            .any(|o| matches!(o, StoryOp::Movie { path, loop_clip }
                if path == "assets/video/opening" && *loop_clip)));
        let bad = "scene main {\n movie \"\"\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err
            .primary_message()
            .contains("movie needs a clip directory"));
    }

    #[test]
    fn unlock_lowers_and_validates_catalog() {
        let src = "scene main {\n unlock gallery cg_a\n unlock music bgm_b\n \"x\"\n}\n";
        let program = load_program_from_source(src, Some("u.vel"), "U").unwrap();
        let ops = &program.scenes["main"].ops;
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::Unlock { catalog, id } if catalog == "gallery" && id == "cg_a")
        ));
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::Unlock { catalog, id } if catalog == "music" && id == "bgm_b")
        ));
        let bad = "scene main {\n unlock video v1\n}\n";
        let err = load_program_from_source(bad, Some("v.vel"), "V").unwrap_err();
        assert!(err.primary_message().contains("unknown unlock catalog"));
    }

    #[test]
    fn game_screens_and_transforms_lower() {
        let src = r#"
screen inv(items, gold = 0) {
  title: "Bag"
  text "Loot:"
  textbutton done action Return("ok")
}
transform bob {
  y -8 1.2 ease_in_out, y 0 1.2 ease_in_out, repeat
}
scene main {
  show luna.happy at left with bob
  screen show inv ("sword", 5)
  screen call inv ("sword")
  screen hide inv
  "done"
  end
}
"#;
        let program = load_program_from_source(src, Some("g.vel"), "G").unwrap();
        let inv = &program.screens["inv"];
        assert_eq!(inv.params.len(), 2);
        assert_eq!(inv.children.len(), 2);
        let bob = &program.transforms["bob"];
        assert_eq!(bob.duration(), None);
        let ops = &program.scenes["main"].ops;
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::Show { transform: Some(t), .. } if t == "bob")));
        assert!(ops.iter().any(
            |o| matches!(o, StoryOp::ShowGameScreen { id, args, .. } if id == "inv" && args.len() == 2)
        ));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::CallGameScreen { id, .. } if id == "inv")));
        assert!(ops
            .iter()
            .any(|o| matches!(o, StoryOp::HideGameScreen { id, .. } if id == "inv")));
    }

    #[test]
    fn game_screen_errors_diagnose() {
        // Unknown screen.
        let bad = "scene main {\n screen show nope\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err.primary_message().contains("unknown screen `nope`"));
        // Unknown transform in `with`.
        let bad = "scene main {\n show luna with wobble\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err
            .primary_message()
            .contains("unknown motion or transform"));
        // Arity: `inv` needs 1 arg.
        let bad = "screen inv(items) {\n text items\n}\nscene main {\n screen show inv\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err.primary_message().contains("needs 1 argument"));
        // Duplicate screen.
        let bad =
            "screen a {\n text \"x\"\n}\nscreen a {\n text \"y\"\n}\nscene main {\n \"z\"\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err.primary_message().contains("duplicate screen `a`"));
        // `use` of an unknown screen.
        let bad = "screen a {\n use nope\n}\nscene main {\n screen show a\n}\n";
        let err = load_program_from_source(bad, Some("b.vel"), "B").unwrap_err();
        assert!(err.primary_message().contains("uses unknown screen"));
    }

    #[test]
    fn screen_call_blocks_until_return() {
        let src = r#"
screen pick {
  textbutton a action Return("a")
}
scene main {
  screen call pick
  "after"
  end "done"
}
"#;
        let program = load_program_from_source(src, Some("c.vel"), "C").unwrap();
        let mut player = StoryPlayer::start(program);
        let mut guard = 0;
        while !matches!(player.wait(), StoryWait::ScreenCall { .. }) && guard < 16 {
            player.advance();
            guard += 1;
        }
        assert!(matches!(player.wait(), StoryWait::ScreenCall { .. }));
        assert_eq!(player.game_screens().len(), 1);
        assert!(player.game_screens()[0].modal);
        assert!(player
            .return_from_screen(StoryValue::String("a".into()))
            .is_ok());
        assert!(player.variables().get("_return").is_truthy());
        assert!(player.game_screens().is_empty());
        let mut steps = 0;
        while !player.is_ended() && steps < 16 {
            match player.wait().clone() {
                StoryWait::Line | StoryWait::Ready | StoryWait::Pause { .. } => player.advance(),
                StoryWait::ScreenCall { .. } => {
                    player.return_from_screen(StoryValue::Null).unwrap();
                }
                StoryWait::Host { token } => {
                    let _ = player.resume_host(&token);
                }
                StoryWait::Choice | StoryWait::Ended => break,
            }
            steps += 1;
        }
        assert_eq!(player.ending(), Some("done"));
    }

    #[test]
    fn unknown_motion_and_screen_diagnose() {
        let bad_motion = "scene main {\n show luna with spin\n}\n";
        let err = load_program_from_source(bad_motion, Some("b.vel"), "B").unwrap_err();
        assert!(err.has_located_diagnostic());
        assert!(err.primary_message().contains("unknown motion"));
        let bad_screen = "scene main {\n screen cinema\n}\n";
        let err = load_program_from_source(bad_screen, Some("c.vel"), "C").unwrap_err();
        assert!(err.primary_message().contains("unknown screen preset"));
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
