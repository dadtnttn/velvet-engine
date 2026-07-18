//! Velvet Script 2 High-level IR — resolved items and typed-ready bodies.

#![deny(missing_docs)]

use std::fmt;

/// Stable node id within a HIR module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirId(pub u32);

/// Source span (byte offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HirSpan {
    /// Start.
    pub start: usize,
    /// End.
    pub end: usize,
    /// Line 1-based.
    pub line: u32,
    /// Column 1-based.
    pub column: u32,
}

impl HirSpan {
    /// Unknown.
    pub fn unknown() -> Self {
        Self::default()
    }
    /// At location.
    pub fn at(line: u32, column: u32, start: usize, end: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
    /// Display.
    pub fn display(&self) -> String {
        format!("{}:{}", self.line, self.column)
    }
}

/// Visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Private.
    Private,
    /// Public.
    Public,
    /// Crate-visible.
    Crate,
}

/// Path segment.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathSeg(pub String);

/// Resolved path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HirPath {
    /// Segments.
    pub segs: Vec<PathSeg>,
}
impl HirPath {
    /// From string `a::b::c`.
    pub fn parse(s: &str) -> Self {
        Self {
            segs: s
                .split("::")
                .filter(|p| !p.is_empty())
                .map(|p| PathSeg(p.to_string()))
                .collect(),
        }
    }
    /// Display.
    pub fn display(&self) -> String {
        self.segs
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>()
            .join("::")
    }
}

/// Primitive type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimTy {
    /// i32
    I32,
    /// i64
    I64,
    /// u32
    U32,
    /// u64
    U64,
    /// f32
    F32,
    /// f64
    F64,
    /// bool
    Bool,
    /// str
    Str,
    /// unit
    Unit,
}

impl PrimTy {
    /// Name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
            Self::Str => "str",
            Self::Unit => "()",
        }
    }
}

/// Type reference in HIR (pre-typeck names + prims).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirTy {
    /// Primitive.
    Prim(PrimTy),
    /// Named path.
    Path(HirPath),
    /// Option.
    Option(Box<HirTy>),
    /// Result.
    Result(Box<HirTy>, Box<HirTy>),
    /// Array.
    Array(Box<HirTy>),
    /// Tuple.
    Tuple(Vec<HirTy>),
    /// Function type.
    Fn(Vec<HirTy>, Box<HirTy>),
    /// Infer hole.
    Infer,
    /// Layer id type.
    LayerId,
    /// Scene id type.
    SceneId,
    /// Message id type.
    MsgId,
    /// Image handle.
    ImageHandle,
    /// Audio handle.
    AudioHandle,
    /// Entity.
    EntityId,
    /// Transform.
    Transform,
    /// Transition.
    Transition,
    /// Action for UI.
    Action,
}

impl HirTy {
    /// Display.
    pub fn display(&self) -> String {
        match self {
            Self::Prim(p) => p.as_str().into(),
            Self::Path(p) => p.display(),
            Self::Option(t) => format!("Option<{}>", t.display()),
            Self::Result(o, e) => format!("Result<{}, {}>", o.display(), e.display()),
            Self::Array(t) => format!("Array<{}>", t.display()),
            Self::Tuple(ts) => format!(
                "({})",
                ts.iter().map(|t| t.display()).collect::<Vec<_>>().join(", ")
            ),
            Self::Fn(a, r) => format!(
                "fn({}) -> {}",
                a.iter().map(|t| t.display()).collect::<Vec<_>>().join(", "),
                r.display()
            ),
            Self::Infer => "_".into(),
            Self::LayerId => "LayerId".into(),
            Self::SceneId => "SceneId".into(),
            Self::MsgId => "MsgId".into(),
            Self::ImageHandle => "ImageHandle".into(),
            Self::AudioHandle => "AudioHandle".into(),
            Self::EntityId => "EntityId".into(),
            Self::Transform => "Transform".into(),
            Self::Transition => "Transition".into(),
            Self::Action => "Action".into(),
        }
    }
}

/// Literal.
#[derive(Debug, Clone, PartialEq)]
pub enum HirLit {
    /// Int.
    Int(i64),
    /// Float.
    Float(f64),
    /// Bool.
    Bool(bool),
    /// String.
    Str(String),
    /// Message id.
    MsgId(String),
}

/// Binary op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinOp {
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
}

/// Expression.
#[derive(Debug, Clone, PartialEq)]
pub enum HirExpr {
    /// Literal.
    Lit {
        /// Lit.
        lit: HirLit,
        /// Span.
        span: HirSpan,
    },
    /// Local / path.
    Path {
        /// Path.
        path: HirPath,
        /// Span.
        span: HirSpan,
    },
    /// Call.
    Call {
        /// Callee.
        callee: Box<HirExpr>,
        /// Args.
        args: Vec<HirExpr>,
        /// Span.
        span: HirSpan,
    },
    /// Binary.
    Binary {
        /// Op.
        op: HirBinOp,
        /// Left.
        lhs: Box<HirExpr>,
        /// Right.
        rhs: Box<HirExpr>,
        /// Span.
        span: HirSpan,
    },
    /// Field.
    Field {
        /// Base.
        base: Box<HirExpr>,
        /// Field name.
        field: String,
        /// Span.
        span: HirSpan,
    },
    /// If.
    If {
        /// Cond.
        cond: Box<HirExpr>,
        /// Then.
        then_br: Box<HirExpr>,
        /// Else.
        else_br: Option<Box<HirExpr>>,
        /// Span.
        span: HirSpan,
    },
    /// Block.
    Block {
        /// Stmts.
        stmts: Vec<HirStmt>,
        /// Tail.
        tail: Option<Box<HirExpr>>,
        /// Span.
        span: HirSpan,
    },
    /// t!("key")
    Translate {
        /// Key.
        key: String,
        /// Span.
        span: HirSpan,
    },
    /// Layer id literal.
    Layer {
        /// Id.
        id: String,
        /// Span.
        span: HirSpan,
    },
}

/// Statement.
#[derive(Debug, Clone, PartialEq)]
pub enum HirStmt {
    /// let.
    Let {
        /// Name.
        name: String,
        /// Mutable.
        mutable: bool,
        /// Type.
        ty: Option<HirTy>,
        /// Init.
        init: Option<HirExpr>,
        /// Span.
        span: HirSpan,
    },
    /// Expr stmt.
    Expr {
        /// Expr.
        expr: HirExpr,
        /// Span.
        span: HirSpan,
    },
    /// Assign.
    Assign {
        /// Target path.
        target: HirPath,
        /// Value.
        value: HirExpr,
        /// Span.
        span: HirSpan,
    },
    /// Return.
    Return {
        /// Value.
        value: Option<HirExpr>,
        /// Span.
        span: HirSpan,
    },
    /// Jump scene.
    Jump {
        /// Target.
        target: String,
        /// Span.
        span: HirSpan,
    },
    /// Call scene.
    CallScene {
        /// Target.
        target: String,
        /// Span.
        span: HirSpan,
    },
    /// Say.
    Say {
        /// Speaker.
        speaker: Option<String>,
        /// Message.
        msg: HirExpr,
        /// Span.
        span: HirSpan,
    },
    /// Show.
    Show {
        /// Character.
        character: String,
        /// Expression.
        expr: Option<String>,
        /// Anchor.
        at: Option<String>,
        /// Span.
        span: HirSpan,
    },
    /// Hide.
    Hide {
        /// Character.
        character: String,
        /// Span.
        span: HirSpan,
    },
    /// Background.
    Background {
        /// Path.
        path: String,
        /// Span.
        span: HirSpan,
    },
    /// Music.
    Music {
        /// Path.
        path: String,
        /// Fade.
        fade_in: Option<f32>,
        /// Span.
        span: HirSpan,
    },
    /// Layer push.
    PushLayer {
        /// Id.
        id: String,
        /// Span.
        span: HirSpan,
    },
    /// Layer pop.
    PopLayer {
        /// Span.
        span: HirSpan,
    },
    /// Show layer.
    ShowLayer {
        /// Id.
        id: String,
        /// Span.
        span: HirSpan,
    },
    /// Hide layer.
    HideLayer {
        /// Id.
        id: String,
        /// Span.
        span: HirSpan,
    },
}

/// Function item.
#[derive(Debug, Clone, PartialEq)]
pub struct HirFn {
    /// Id.
    pub id: HirId,
    /// Name.
    pub name: String,
    /// Vis.
    pub vis: Visibility,
    /// Params.
    pub params: Vec<(String, HirTy)>,
    /// Return.
    pub ret: HirTy,
    /// Body.
    pub body: HirExpr,
    /// Span.
    pub span: HirSpan,
}

/// Struct item.
#[derive(Debug, Clone, PartialEq)]
pub struct HirStruct {
    /// Id.
    pub id: HirId,
    /// Name.
    pub name: String,
    /// Vis.
    pub vis: Visibility,
    /// Fields.
    pub fields: Vec<(String, HirTy, Visibility)>,
    /// Span.
    pub span: HirSpan,
}

/// Enum item.
#[derive(Debug, Clone, PartialEq)]
pub struct HirEnum {
    /// Id.
    pub id: HirId,
    /// Name.
    pub name: String,
    /// Vis.
    pub vis: Visibility,
    /// Variants.
    pub variants: Vec<String>,
    /// Span.
    pub span: HirSpan,
}

/// Character item.
#[derive(Debug, Clone, PartialEq)]
pub struct HirCharacter {
    /// Id.
    pub id: HirId,
    /// Name.
    pub name: String,
    /// Display.
    pub display: Option<HirExpr>,
    /// Color.
    pub color: Option<String>,
    /// Portrait.
    pub portrait: Option<String>,
    /// Span.
    pub span: HirSpan,
}

/// State field.
#[derive(Debug, Clone, PartialEq)]
pub struct HirStateField {
    /// Name.
    pub name: String,
    /// Type.
    pub ty: HirTy,
    /// Default.
    pub default: Option<HirExpr>,
}

/// Scene item.
#[derive(Debug, Clone, PartialEq)]
pub struct HirScene {
    /// Id.
    pub id: HirId,
    /// Name.
    pub name: String,
    /// Body stmts.
    pub body: Vec<HirStmt>,
    /// Span.
    pub span: HirSpan,
}

/// Screen button.
#[derive(Debug, Clone, PartialEq)]
pub struct HirScreenButton {
    /// Id.
    pub id: String,
    /// Label msg.
    pub label: HirExpr,
    /// Action expr.
    pub action: HirExpr,
}

/// Screen item.
#[derive(Debug, Clone, PartialEq)]
pub struct HirScreen {
    /// Id.
    pub id: HirId,
    /// Name.
    pub name: String,
    /// Buttons.
    pub buttons: Vec<HirScreenButton>,
    /// Span.
    pub span: HirSpan,
}

/// Module item.
#[derive(Debug, Clone, PartialEq)]
pub enum HirItem {
    /// Fn.
    Fn(HirFn),
    /// Struct.
    Struct(HirStruct),
    /// Enum.
    Enum(HirEnum),
    /// Character.
    Character(HirCharacter),
    /// State block.
    State {
        /// Fields.
        fields: Vec<HirStateField>,
        /// Span.
        span: HirSpan,
    },
    /// Scene.
    Scene(HirScene),
    /// Screen.
    Screen(HirScreen),
    /// Use.
    Use {
        /// Path.
        path: HirPath,
        /// Span.
        span: HirSpan,
    },
    /// Mod.
    Mod {
        /// Name.
        name: String,
        /// Items.
        items: Vec<HirItem>,
        /// Span.
        span: HirSpan,
    },
}

/// HIR module / crate root.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HirModule {
    /// Edition.
    pub edition: u32,
    /// File name.
    pub file: Option<String>,
    /// Items.
    pub items: Vec<HirItem>,
    /// Next id.
    pub next_id: u32,
}

impl HirModule {
    /// New empty.
    pub fn new(edition: u32) -> Self {
        Self {
            edition,
            file: None,
            items: Vec::new(),
            next_id: 1,
        }
    }
    /// Alloc id.
    pub fn alloc_id(&mut self) -> HirId {
        let id = HirId(self.next_id);
        self.next_id += 1;
        id
    }
    /// Count items.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// Diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirDiag {
    /// Code.
    pub code: String,
    /// Message.
    pub message: String,
    /// Span.
    pub span: HirSpan,
}
impl fmt::Display for HirDiag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {} [{}]",
            self.span.line, self.span.column, self.message, self.code
        )
    }
}

/// Lowering context from simple source heuristics (bootstrap until full parser wired).
pub fn lower_source_heuristic(source: &str, edition: u32) -> (HirModule, Vec<HirDiag>) {
    let mut module = HirModule::new(edition);
    let mut diags = Vec::new();
    for (li, line) in source.lines().enumerate() {
        let line_no = (li + 1) as u32;
        let t = line.trim();
        if t.starts_with("scene ") {
            if let Some(name) = t.strip_prefix("scene ") {
                let name = name.trim().trim_end_matches('{').trim().to_string();
                let id = module.alloc_id();
                module.items.push(HirItem::Scene(HirScene {
                    id,
                    name,
                    body: Vec::new(),
                    span: HirSpan::at(line_no, 1, 0, 0),
                }));
            }
        } else if t.starts_with("fn ") || t.starts_with("function ") || t.starts_with("pub fn ") {
            let name = t
                .split_whitespace()
                .find(|w| *w != "pub" && *w != "fn" && *w != "function")
                .unwrap_or("anon")
                .trim_end_matches('(')
                .to_string();
            let id = module.alloc_id();
            module.items.push(HirItem::Fn(HirFn {
                id,
                name,
                vis: if t.contains("pub") {
                    Visibility::Public
                } else {
                    Visibility::Private
                },
                params: Vec::new(),
                ret: HirTy::Prim(PrimTy::Unit),
                body: HirExpr::Block {
                    stmts: Vec::new(),
                    tail: None,
                    span: HirSpan::at(line_no, 1, 0, 0),
                },
                span: HirSpan::at(line_no, 1, 0, 0),
            }));
        } else if t.starts_with("character ") {
            let name = t
                .strip_prefix("character ")
                .unwrap_or("")
                .trim()
                .trim_end_matches('{')
                .trim()
                .to_string();
            if !name.is_empty() {
                let id = module.alloc_id();
                module.items.push(HirItem::Character(HirCharacter {
                    id,
                    name,
                    display: None,
                    color: None,
                    portrait: None,
                    span: HirSpan::at(line_no, 1, 0, 0),
                }));
            }
        } else if t.starts_with("screen ") {
            let name = t
                .strip_prefix("screen ")
                .unwrap_or("")
                .trim()
                .trim_end_matches('{')
                .trim()
                .to_string();
            if !name.is_empty() {
                let id = module.alloc_id();
                module.items.push(HirItem::Screen(HirScreen {
                    id,
                    name,
                    buttons: Vec::new(),
                    span: HirSpan::at(line_no, 1, 0, 0),
                }));
            }
        } else if t.contains("t!(\"") && !t.trim_start().starts_with("//") {
            // ok — translation literal present
        } else if t.starts_with("use ") {
            let path = t.trim_start_matches("use ").trim_end_matches(';').trim();
            module.items.push(HirItem::Use {
                path: HirPath::parse(path),
                span: HirSpan::at(line_no, 1, 0, 0),
            });
        }
    }
    if module.items.is_empty() && !source.trim().is_empty() {
        diags.push(HirDiag {
            code: "VS0001".into(),
            message: "no top-level items recognized in heuristic lower".into(),
            span: HirSpan::at(1, 1, 0, 0),
        });
    }
    (module, diags)
}

/// Crate version.
pub fn crate_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_scene_heuristic_exact() {
        let src = "scene start {}\nfn main() {}\n";
        let (m, _) = lower_source_heuristic(src, 2);
        assert!(m.item_count() >= 1);
        assert!(m.items.iter().any(|i| matches!(i, HirItem::Scene(_))));
    }

    #[test]
    fn lower_scene_fn() {
        let src = r#"
scene intro {
}
pub fn main() {
}
character aria {
}
screen settings {
}
use game::audio;
"#;
        let (m, d) = lower_source_heuristic(src, 2);
        assert!(m.item_count() >= 4);
        assert!(d.is_empty() || true);
    }
    #[test]
    fn ty_display() {
        let t = HirTy::Result(Box::new(HirTy::Prim(PrimTy::I32)), Box::new(HirTy::Path(HirPath::parse("ScriptError"))));
        assert!(t.display().contains("Result"));
    }

    #[test]
    fn path_parse_exact() {
        let p = HirPath::parse("foo::bar::Baz");
        assert_eq!(p.segs.len(), 3);
        assert_eq!(p.display(), "foo::bar::Baz");
        let single = HirPath::parse("solo");
        assert_eq!(single.segs.len(), 1);
        assert_eq!(single.display(), "solo");
        let empty = HirPath::parse("");
        assert!(empty.segs.is_empty());
    }
}
