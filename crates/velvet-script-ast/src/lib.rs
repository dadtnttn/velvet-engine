//! # velvet-script-ast
//!
//! Abstract syntax tree for Velvet Script with source locations.

#![deny(missing_docs)]

use velvet_script_lexer::Span;

/// Source location (file optional, line/column 1-based).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceLoc {
    /// Optional file path.
    pub file: Option<String>,
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
    /// Byte span.
    pub span: Span,
}

impl SourceLoc {
    /// Unknown location.
    pub fn unknown() -> Self {
        Self::default()
    }

    /// From line/column/span.
    pub fn at(line: u32, column: u32, span: Span) -> Self {
        Self {
            file: None,
            line,
            column,
            span,
        }
    }

    /// With file.
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Format `file:line:column` or `line:column`.
    pub fn display(&self) -> String {
        match &self.file {
            Some(f) => format!("{f}:{}:{}", self.line, self.column),
            None => format!("{}:{}", self.line, self.column),
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
    /// %
    Rem,
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
    /// &&
    And,
    /// ||
    Or,
    /// =
    Assign,
    /// +=
    AddAssign,
    /// -=
    SubAssign,
    /// *=
    MulAssign,
    /// /=
    DivAssign,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// -
    Neg,
    /// !
    Not,
}

/// Expression node.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Null / none literal.
    Null {
        /// Location.
        loc: SourceLoc,
    },
    /// Boolean.
    Bool {
        /// Value.
        value: bool,
        /// Location.
        loc: SourceLoc,
    },
    /// Integer.
    Int {
        /// Value.
        value: i64,
        /// Location.
        loc: SourceLoc,
    },
    /// Float.
    Float {
        /// Value.
        value: f64,
        /// Location.
        loc: SourceLoc,
    },
    /// String.
    String {
        /// Value.
        value: String,
        /// Location.
        loc: SourceLoc,
    },
    /// Identifier.
    Ident {
        /// Name.
        name: String,
        /// Location.
        loc: SourceLoc,
    },
    /// List literal `[a, b]`.
    List {
        /// Elements.
        elements: Vec<Expr>,
        /// Location.
        loc: SourceLoc,
    },
    /// String-keyed map literal `{ "key": value }`.
    Map {
        /// Entries in source order.
        entries: Vec<(String, Expr)>,
        /// Location.
        loc: SourceLoc,
    },
    /// Unary.
    Unary {
        /// Operator.
        op: UnaryOp,
        /// Operand.
        expr: Box<Expr>,
        /// Location.
        loc: SourceLoc,
    },
    /// Binary.
    Binary {
        /// Left.
        left: Box<Expr>,
        /// Operator.
        op: BinOp,
        /// Right.
        right: Box<Expr>,
        /// Location.
        loc: SourceLoc,
    },
    /// Call `f(a, b)`.
    Call {
        /// Callee.
        callee: Box<Expr>,
        /// Arguments.
        args: Vec<Expr>,
        /// Location.
        loc: SourceLoc,
    },
    /// Field access `a.b`.
    Field {
        /// Object.
        object: Box<Expr>,
        /// Field name.
        field: String,
        /// Location.
        loc: SourceLoc,
    },
    /// Index `a[b]`.
    Index {
        /// Object.
        object: Box<Expr>,
        /// Index.
        index: Box<Expr>,
        /// Location.
        loc: SourceLoc,
    },
}

impl Expr {
    /// Source location of this expression.
    pub fn loc(&self) -> &SourceLoc {
        match self {
            Self::Null { loc }
            | Self::Bool { loc, .. }
            | Self::Int { loc, .. }
            | Self::Float { loc, .. }
            | Self::String { loc, .. }
            | Self::Ident { loc, .. }
            | Self::List { loc, .. }
            | Self::Map { loc, .. }
            | Self::Unary { loc, .. }
            | Self::Binary { loc, .. }
            | Self::Call { loc, .. }
            | Self::Field { loc, .. }
            | Self::Index { loc, .. } => loc,
        }
    }
}

/// Statement node.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Expression statement.
    Expr {
        /// Expression.
        expr: Expr,
        /// Location.
        loc: SourceLoc,
    },
    /// `let name = expr` or `let name: type = expr`.
    Let {
        /// Name.
        name: String,
        /// Optional type annotation text.
        ty: Option<String>,
        /// Initializer.
        init: Expr,
        /// Location.
        loc: SourceLoc,
    },
    /// `const name = expr`.
    Const {
        /// Name.
        name: String,
        /// Value.
        init: Expr,
        /// Location.
        loc: SourceLoc,
    },
    /// Block `{ ... }`.
    Block {
        /// Statements.
        body: Vec<Stmt>,
        /// Location.
        loc: SourceLoc,
    },
    /// `if cond { } else { }`.
    If {
        /// Condition.
        cond: Expr,
        /// Then branch.
        then_body: Box<Stmt>,
        /// Optional else.
        else_body: Option<Box<Stmt>>,
        /// Location.
        loc: SourceLoc,
    },
    /// `while cond { }`.
    While {
        /// Condition.
        cond: Expr,
        /// Body.
        body: Box<Stmt>,
        /// Location.
        loc: SourceLoc,
    },
    /// `return expr?`.
    Return {
        /// Optional value.
        value: Option<Expr>,
        /// Location.
        loc: SourceLoc,
    },
    /// Dialogue line: `speaker "text"` or narrator `"text"`.
    Dialogue {
        /// Speaker ident (None = narrator).
        speaker: Option<String>,
        /// Text.
        text: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `jump label`.
    Jump {
        /// Label.
        label: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `label name:`.
    Label {
        /// Name.
        name: String,
        /// Location.
        loc: SourceLoc,
    },
    /// Choice block.
    Choice {
        /// Options.
        options: Vec<ChoiceArm>,
        /// Location.
        loc: SourceLoc,
    },
    /// `show target at place` (simplified).
    Show {
        /// Target expression text / path.
        target: String,
        /// Optional placement.
        at: Option<String>,
        /// Optional mini-ATL suffix: `with <move|zoom|shake|fade> [secs] [amount]`.
        with: Option<String>,
        /// Location.
        loc: SourceLoc,
    },
    /// `screen <preset>` — apply a 3-line VN screen preset
    /// (`say_classic` / `say_minimal` / `nvl_page` / `menu`).
    Screen {
        /// Preset id.
        id: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `unlock gallery <id>` / `unlock music <id>` — unlock a CG or track.
    Unlock {
        /// Catalog: `gallery` or `music`.
        kind: String,
        /// Entry id.
        id: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `movie "assets/video/opening" [loop]` — play a cinema clip.
    Movie {
        /// Clip directory path.
        path: String,
        /// Loop instead of finishing.
        loop_clip: bool,
        /// Location.
        loc: SourceLoc,
    },
    /// `screen show|hide|call <id> [(args…)] [with …] [into var]` — actions.
    ScreenAction {
        /// Action kind.
        action: ScreenActionKind,
        /// Screen id.
        id: String,
        /// Positional argument expressions.
        args: Vec<Expr>,
        /// Optional `with` transition (mini-ATL words or transform name).
        with: Option<String>,
        /// Optional `into` result variable (`screen call` only).
        result: Option<String>,
        /// Location.
        loc: SourceLoc,
    },
    /// `background "path"`.
    Background {
        /// Path.
        path: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `music "path" ...`.
    Music {
        /// Path.
        path: String,
        /// Fade in seconds.
        fade_in: Option<f64>,
        /// Location.
        loc: SourceLoc,
    },
    /// `hide target [with motion|transform]`.
    Hide {
        /// Target id / expression path.
        target: String,
        /// Optional `with` suffix (mini-ATL words or transform name).
        with: Option<String>,
        /// Location.
        loc: SourceLoc,
    },
    /// `end` or `end "ending_id"`.
    End {
        /// Optional ending identifier.
        ending: Option<String>,
        /// Location.
        loc: SourceLoc,
    },
    /// `call scene_name` — subroutine jump with return.
    Call {
        /// Target scene or label.
        target: String,
        /// Location.
        loc: SourceLoc,
    },
    /// Host command: `call combat.start enemy "goblin"` (dotted name → host).
    HostCall {
        /// Command name e.g. `combat.start` or `ui.flag`.
        name: String,
        /// Named literal args (`key` then value expression).
        args: Vec<(String, Expr)>,
        /// Location.
        loc: SourceLoc,
    },
    /// `transition fade` / `transition dissolve`.
    Transition {
        /// Transition id.
        name: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `sound "path"`.
    Sound {
        /// Asset path.
        path: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `pause` or `pause 0.5`.
    Pause {
        /// Optional seconds.
        seconds: Option<f64>,
        /// Location.
        loc: SourceLoc,
    },
    /// `for name in expr { body }` (desugared by compiler / lowered carefully).
    For {
        /// Loop variable.
        name: String,
        /// Iterable expression.
        iter: Expr,
        /// Loop body.
        body: Box<Stmt>,
        /// Location.
        loc: SourceLoc,
    },
    /// `break` from nearest loop.
    Break {
        /// Location.
        loc: SourceLoc,
    },
    /// `continue` nearest loop.
    Continue {
        /// Location.
        loc: SourceLoc,
    },
}

impl Stmt {
    /// Source location of this statement.
    pub fn loc(&self) -> &SourceLoc {
        match self {
            Self::Expr { loc, .. }
            | Self::Let { loc, .. }
            | Self::Const { loc, .. }
            | Self::Block { loc, .. }
            | Self::If { loc, .. }
            | Self::While { loc, .. }
            | Self::Return { loc, .. }
            | Self::Dialogue { loc, .. }
            | Self::Jump { loc, .. }
            | Self::Label { loc, .. }
            | Self::Choice { loc, .. }
            | Self::Show { loc, .. }
            | Self::Screen { loc, .. }
            | Self::Unlock { loc, .. }
            | Self::Movie { loc, .. }
            | Self::ScreenAction { loc, .. }
            | Self::Background { loc, .. }
            | Self::Music { loc, .. }
            | Self::Hide { loc, .. }
            | Self::End { loc, .. }
            | Self::Call { loc, .. }
            | Self::HostCall { loc, .. }
            | Self::Transition { loc, .. }
            | Self::Sound { loc, .. }
            | Self::Pause { loc, .. }
            | Self::For { loc, .. }
            | Self::Break { loc, .. }
            | Self::Continue { loc, .. } => loc,
        }
    }
}

/// One choice arm.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceArm {
    /// Option text.
    pub text: String,
    /// Body statements.
    pub body: Vec<Stmt>,
    /// Location.
    pub loc: SourceLoc,
}

/// Named value inside a declarative VS2 screen or widget.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenProperty {
    /// Property name, for example `title`, `class`, or `description`.
    pub name: String,
    /// Property expression. Screen compilation currently accepts literal values.
    pub value: Expr,
    /// Source location of the property name.
    pub loc: SourceLoc,
}

/// Declarative button inside a VS2 [`Item::Screen`].
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenButton {
    /// Stable widget id used by actions and VCSS `#id` selectors.
    pub id: String,
    /// Button properties such as `label`, `action`, and `icon`.
    pub properties: Vec<ScreenProperty>,
    /// Source location of the `button` declaration.
    pub loc: SourceLoc,
}

/// Parameter of a game screen (`screen inv(items)`), with optional default.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenParam {
    /// Parameter name.
    pub name: String,
    /// Default expression when the caller omits it.
    pub default: Option<Expr>,
    /// Source location of the parameter name.
    pub loc: SourceLoc,
}

/// One widget child inside a game screen.
#[derive(Debug, Clone, PartialEq)]
pub enum ScreenNode {
    /// `text <expr>`.
    Text {
        /// Text expression.
        text: Expr,
        /// Location.
        loc: SourceLoc,
    },
    /// `textbutton <label> action <action>`.
    TextButton {
        /// Label expression.
        label: Expr,
        /// Action expression (`Return(…)`, `Jump(…)`, …).
        action: Expr,
        /// Location.
        loc: SourceLoc,
    },
    /// `use <screen> [(args…)]`.
    Use {
        /// Referenced screen name.
        screen: String,
        /// Positional argument expressions.
        args: Vec<Expr>,
        /// Location.
        loc: SourceLoc,
    },
    /// Generic widget (`image`, `bar`, `vbox`, `hbox`, `fixed`, `timer`,
    /// `key`, `input`) with properties and nested children.
    Widget {
        /// Widget kind.
        kind: String,
        /// Property list (`label: …`, `action: …`, …).
        properties: Vec<ScreenProperty>,
        /// Nested children (containers).
        children: Vec<ScreenNode>,
        /// Location.
        loc: SourceLoc,
    },
}

impl ScreenNode {
    /// Source location of this node.
    pub fn loc(&self) -> &SourceLoc {
        match self {
            Self::Text { loc, .. }
            | Self::TextButton { loc, .. }
            | Self::Use { loc, .. }
            | Self::Widget { loc, .. } => loc,
        }
    }
}

/// Screen show/hide/call action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenActionKind {
    /// `screen show <id>` / `show screen <id>`.
    Show,
    /// `screen hide <id>` / `hide screen <id>`.
    Hide,
    /// `screen call <id>` (blocks for a return value).
    Call,
}

impl ScreenActionKind {
    /// Parse action word.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "show" => Some(Self::Show),
            "hide" => Some(Self::Hide),
            "call" => Some(Self::Call),
            _ => None,
        }
    }

    /// Canonical name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Hide => "hide",
            Self::Call => "call",
        }
    }
}

/// ATL scalar: number or keyword channel value.
#[derive(Debug, Clone, PartialEq)]
pub enum AtlValue {
    /// Number.
    Number(f64),
    /// Keyword (warper names, `repeat` counts are separate).
    Keyword(String),
}

/// One ATL step inside `transform`.
#[derive(Debug, Clone, PartialEq)]
pub enum AtlStep {
    /// `prop value [duration] [warper], …` — one or more channels.
    Set {
        /// Channel assignments.
        props: Vec<(String, AtlValue)>,
        /// Duration seconds (0 = instant).
        duration: f64,
        /// Warper name (`linear` default).
        warper: String,
        /// Location.
        loc: SourceLoc,
    },
    /// `parallel { … }` — steps run together.
    Parallel {
        /// Concurrent steps.
        steps: Vec<AtlStep>,
        /// Location.
        loc: SourceLoc,
    },
    /// `choice { … }` — host picks one variant.
    Choice {
        /// Variants.
        steps: Vec<AtlStep>,
        /// Location.
        loc: SourceLoc,
    },
    /// `repeat [n]` — loop enclosing block (None = forever).
    Repeat {
        /// Iterations (None = infinite).
        count: Option<u32>,
        /// Repeated steps.
        steps: Vec<AtlStep>,
        /// Location.
        loc: SourceLoc,
    },
    /// `on show, hide { … }` — event handler block.
    On {
        /// Events (`show` / `hide` / `replace`).
        events: Vec<String>,
        /// Handler steps.
        steps: Vec<AtlStep>,
        /// Location.
        loc: SourceLoc,
    },
}

impl AtlStep {
    /// Source location of this step.
    pub fn loc(&self) -> &SourceLoc {
        match self {
            Self::Set { loc, .. }
            | Self::Parallel { loc, .. }
            | Self::Choice { loc, .. }
            | Self::Repeat { loc, .. }
            | Self::On { loc, .. } => loc,
        }
    }
}

/// Top-level item.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// Source import. `alias` is present for nominal module imports.
    Import {
        /// Relative source path.
        path: String,
        /// Optional local module alias from `import "path" as alias`.
        alias: Option<String>,
        /// Location.
        loc: SourceLoc,
    },
    /// Function definition.
    Function {
        /// Whether the function is explicitly exported from its source module.
        exported: bool,
        /// Name.
        name: String,
        /// Parameters.
        params: Vec<Param>,
        /// Body.
        body: Vec<Stmt>,
        /// Location.
        loc: SourceLoc,
    },
    /// Character definition.
    Character {
        /// Id.
        name: String,
        /// Fields as statements/bindings.
        fields: Vec<(String, Expr)>,
        /// Location.
        loc: SourceLoc,
    },
    /// State block with variables.
    State {
        /// Bindings name + optional type + init.
        bindings: Vec<StateBinding>,
        /// Location.
        loc: SourceLoc,
    },
    /// Scene block.
    Scene {
        /// Name.
        name: String,
        /// Body.
        body: Vec<Stmt>,
        /// Location.
        loc: SourceLoc,
    },
    /// Declarative UI screen.
    Screen {
        /// Stable screen name.
        name: String,
        /// Screen-level properties such as `class`, `title`, and `subtitle`.
        properties: Vec<ScreenProperty>,
        /// Interactive buttons in author order.
        buttons: Vec<ScreenButton>,
        /// Screen parameters (`screen inv(items)`), possibly empty.
        params: Vec<ScreenParam>,
        /// Widget children (Ren'Py-style screens), possibly empty.
        children: Vec<ScreenNode>,
        /// Location.
        loc: SourceLoc,
    },
    /// ATL transform block (`transform bounce { … }`).
    Transform {
        /// Transform name (used by `show … with <name>`).
        name: String,
        /// Steps in author order.
        steps: Vec<AtlStep>,
        /// Location.
        loc: SourceLoc,
    },
    /// Free statement at module level.
    Stmt(Stmt),
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// Name.
    pub name: String,
    /// Optional type name.
    pub ty: Option<String>,
}

/// State variable binding.
#[derive(Debug, Clone, PartialEq)]
pub struct StateBinding {
    /// Name.
    pub name: String,
    /// Type annotation.
    pub ty: Option<String>,
    /// Initializer.
    pub init: Expr,
    /// Location.
    pub loc: SourceLoc,
}

/// Parsed module / file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// Optional file name.
    pub file: Option<String>,
    /// Items.
    pub items: Vec<Item>,
    /// Diagnostics collected during recovery (non-fatal).
    pub diagnostics: Vec<Diagnostic>,
}

/// Parser/compiler diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Message.
    pub message: String,
    /// Location.
    pub loc: SourceLoc,
}

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Error.
    Error,
    /// Warning.
    Warning,
}

impl Diagnostic {
    /// Error diagnostic.
    pub fn error(message: impl Into<String>, loc: SourceLoc) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            loc,
        }
    }

    /// Format with location.
    pub fn display(&self) -> String {
        format!("{}: {}", self.loc.display(), self.message)
    }
}

impl Module {
    /// Whether any error diagnostics exist.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}
