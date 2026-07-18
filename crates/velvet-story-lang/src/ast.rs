//! Narrative AST for Velvet Story.

use crate::span::Span;
use serde::{Deserialize, Serialize};

/// Top-level document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryFile {
    /// Source path.
    pub file: String,
    /// Top-level items (usually scenes + includes).
    pub items: Vec<TopItem>,
}

/// Top-level item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopItem {
    /// `scene name`
    Scene(Scene),
    /// `include path`
    Include {
        /// Path string.
        path: String,
        /// Span.
        span: Span,
    },
    /// Character declaration (optional metadata).
    CharacterDecl {
        /// Id.
        name: String,
        /// Display name.
        display: Option<String>,
        /// Span.
        span: Span,
    },
}

/// One scene block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    /// Stable scene id (also used in saves).
    pub name: String,
    /// Statements.
    pub body: Vec<Stmt>,
    /// Span of `scene name`.
    pub span: Span,
}

/// Statement in a scene / block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    /// `background id`
    Background {
        /// Asset id.
        id: String,
        /// Span.
        span: Span,
    },
    /// `music id`
    Music {
        /// Asset id.
        id: String,
        /// Span.
        span: Span,
    },
    /// `sound id`
    Sound {
        /// Asset id.
        id: String,
        /// Span.
        span: Span,
    },
    /// `show char [expr] [at pos]`
    Show {
        /// Character id.
        character: String,
        /// Expression / emotion.
        expression: Option<String>,
        /// Position.
        at: Option<String>,
        /// Span.
        span: Span,
    },
    /// `hide char`
    Hide {
        /// Character.
        character: String,
        /// Span.
        span: Span,
    },
    /// Dialogue: `speaker [@msg_id]:` + indented lines
    Dialogue {
        /// Speaker id (`narrator` for narration).
        speaker: String,
        /// Stable loc id if present.
        msg_id: Option<String>,
        /// Lines joined.
        text: String,
        /// Span.
        span: Span,
    },
    /// `choice:` with options
    Choice {
        /// Options.
        options: Vec<ChoiceArm>,
        /// Span.
        span: Span,
    },
    /// `goto target` / `jump target`
    Goto {
        /// Scene or label name.
        target: String,
        /// Span.
        span: Span,
    },
    /// `call scene_name`
    CallScene {
        /// Target scene.
        target: String,
        /// Span.
        span: Span,
    },
    /// `return`
    Return {
        /// Span.
        span: Span,
    },
    /// `end`
    End {
        /// Span.
        span: Span,
    },
    /// `label name`
    Label {
        /// Label name.
        name: String,
        /// Span.
        span: Span,
    },
    /// `set name = expr`
    Set {
        /// Variable.
        name: String,
        /// Value.
        value: Expr,
        /// Span.
        span: Span,
    },
    /// `add name n` / `add name expr`
    Add {
        /// Variable.
        name: String,
        /// Delta.
        value: Expr,
        /// Span.
        span: Span,
    },
    /// `sub name n`
    Sub {
        /// Variable.
        name: String,
        /// Delta.
        value: Expr,
        /// Span.
        span: Span,
    },
    /// `if cond:` / `else:`
    If {
        /// Condition.
        cond: Expr,
        /// Then body.
        then_body: Vec<Stmt>,
        /// Else body.
        else_body: Option<Vec<Stmt>>,
        /// Span.
        span: Span,
    },
    /// `call cmd.name:` with kwargs
    CallCommand {
        /// Command path e.g. `combat.start`.
        name: String,
        /// Named args.
        args: Vec<(String, Expr)>,
        /// Span.
        span: Span,
    },
    /// `pause` / `wait`
    Pause {
        /// Optional seconds (as expr).
        duration: Option<Expr>,
        /// Span.
        span: Span,
    },
    /// `with transition`
    Transition {
        /// Name.
        name: String,
        /// Span.
        span: Span,
    },
    /// Comment preserved.
    Comment {
        /// Text.
        text: String,
        /// Span.
        span: Span,
    },
}

/// Choice arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChoiceArm {
    /// Visible label.
    pub label: String,
    /// Optional loc id.
    pub msg_id: Option<String>,
    /// Body.
    pub body: Vec<Stmt>,
    /// Span.
    pub span: Span,
}

/// Simple expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    /// Integer.
    Int(i64, Span),
    /// Float text.
    Float(String, Span),
    /// Bool.
    Bool(bool, Span),
    /// String.
    Str(String, Span),
    /// Variable / ident.
    Ident(String, Span),
    /// Binary.
    Binary {
        /// Op.
        op: BinOp,
        /// Left.
        left: Box<Expr>,
        /// Right.
        right: Box<Expr>,
        /// Span.
        span: Span,
    },
    /// Unary not / minus.
    Unary {
        /// Op.
        op: UnaryOp,
        /// Expr.
        expr: Box<Expr>,
        /// Span.
        span: Span,
    },
}

/// Binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
    /// ==
    Eq,
    /// !=
    Ne,
    /// <
    Lt,
    /// <=
    Le,
    /// >
    Gt,
    /// >=
    Ge,
    /// and
    And,
    /// or
    Or,
}

/// Unary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    /// not
    Not,
    /// -
    Neg,
}

impl Expr {
    /// Span of expression.
    pub fn span(&self) -> Span {
        match self {
            Expr::Int(_, s)
            | Expr::Float(_, s)
            | Expr::Bool(_, s)
            | Expr::Str(_, s)
            | Expr::Ident(_, s) => *s,
            Expr::Binary { span, .. } | Expr::Unary { span, .. } => *span,
        }
    }
}

impl Stmt {
    /// Span.
    pub fn span(&self) -> Span {
        match self {
            Stmt::Background { span, .. }
            | Stmt::Music { span, .. }
            | Stmt::Sound { span, .. }
            | Stmt::Show { span, .. }
            | Stmt::Hide { span, .. }
            | Stmt::Dialogue { span, .. }
            | Stmt::Choice { span, .. }
            | Stmt::Goto { span, .. }
            | Stmt::CallScene { span, .. }
            | Stmt::Return { span }
            | Stmt::End { span }
            | Stmt::Label { span, .. }
            | Stmt::Set { span, .. }
            | Stmt::Add { span, .. }
            | Stmt::Sub { span, .. }
            | Stmt::If { span, .. }
            | Stmt::CallCommand { span, .. }
            | Stmt::Pause { span, .. }
            | Stmt::Transition { span, .. }
            | Stmt::Comment { span, .. } => *span,
        }
    }
}
